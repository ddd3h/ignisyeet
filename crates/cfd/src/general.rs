// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2024-2026 西濱大将 (NISHIHAMA Daisuke)
//! General (six-component) table builder and the roll-damping derivative from the CFD roll pair.
//!
//! # General table
//!
//! The table is a `TableKind::General` table on the grid `alpha = angle_axis(180, 30, 2, 10)`,
//! `beta = angle_axis(90, 30, 2, 10)` and the Mach grid of the aero options (or
//! `general_mach_step`). Per solved Mach number `M_k`:
//!
//! 1. **Symmetry completion.** The solved points `(alpha, beta, C)` are reflected with the mirror
//!    symmetries of the shape ([`ShapeSymmetry`]): the image of a state under `z -> -z` is
//!    `(-alpha, beta, C.mirror_z())`, under `y -> -y` it is `(alpha, -beta, C.mirror_y())`
//!    ([`BodyCoeffs6::mirror_z`], [`BodyCoeffs6::mirror_y`]). In `mirror` mode all images that fall
//!    on the same `(alpha, beta)` are averaged (this removes numerical asymmetry); otherwise a solved
//!    point wins over an image. In `mirror` and `offset` mode the side and normal forces and the pitch and yaw
//!    moments (`CY`, `CZ`, `Cm`, `Cn`, and `Cl` of a mirror-symmetric shape) of the `(0, 0)` point are subtracted
//!    from all points first (see [`crate::asymmetry`]); the roll moment of a shape without mirror symmetry is the
//!    roll forcing of canted fins and stays.
//! 2. **CFD interpolant `F(alpha, beta)`** on the solved rectangle `R = [a_min, a_max] x [b_min, b_max]`:
//!    PCHIP along `alpha` for every solved `beta`, then PCHIP along `beta` of those values (separable).
//! 3. **Model fill.** `T(alpha, beta)` is the Barrowman model expanded to six components
//!    ([`AeroTable::to_general`] of the model table up to 90 deg). Outside `R` the value is
//!    `F(clamp(alpha, beta)) + T(alpha, beta) - T(clamp(alpha, beta))`, i.e. the model's variation
//!    offset-matched at the boundary of the solved region (continuous across the boundary).
//! 4. **Axial force.** `CA_cfd = -F_cx` gets the empirical friction, base, fin-edge, boattail and extra
//!    terms of [`hybrid_axial`] at the total angle `acos(cos a cos b)`.
//! 5. Across Mach the node values are interpolated with PCHIP (`linear` / `clamp` extrapolation).
//!
//! Damping sums: with `CZ = -CN_pitch` and `Cm = x_cp CZ / d`, the pitch slope is
//! `CNa_p = -dCZ/dalpha` and `x_p = d (dCm/dalpha) / (dCZ/dalpha)` at `(0, 0)` (odd cubic fit through the
//! two lowest angles, `odd_slope`); the yaw slope `CNa_y = -dCY/dbeta`, `x_y = -d (dCn/dbeta) / (dCY/dbeta)`
//! at `alpha = 0`. [`scale_damping`] then rescales the Barrowman sums separately for pitch (`sq`) and yaw
//! (`sr`). Without solved sideslip the yaw sums equal the pitch sums.
//!
//! # Roll damping
//!
//! `Clp = (Cl(+p_hat) - Cl(-p_hat)) / (2 p_hat)` from the rotating-frame pair at `alpha = beta = 0`.
//! Unusable pairs (missing, positive, or `|Clp| < 1e-6` for a rocket with fins) fall back to the
//! Barrowman value of that Mach number. The table value is the Barrowman `Clp(M)` scaled with the
//! (clamped PCHIP) ratio CFD / Barrowman of the usable Mach numbers.

use crate::config::FlowModel;
use crate::config::ShapeSymmetry;
use crate::forces::{hybrid_axial, scale_damping, BodyCoeffs6};
use crate::table::{grid, odd_slope, Pchip};
use aero::table::{angle_axis, Coeffs6, TableMeta};
use aero::{AeroModel, AeroTable, Extrapolation};
use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// One solved point of the general matrix.
#[derive(Debug, Clone, Copy)]
pub struct SolvedPoint6 {
    pub mach: f64,
    pub alpha_deg: f64,
    pub beta_deg: f64,
    pub coeffs: Option<BodyCoeffs6>,
}

/// Roll pair of one Mach number: the Cl of the `+roll_hat` and `-roll_hat` runs.
#[derive(Debug, Clone, Copy)]
pub struct RollPair {
    pub mach: f64,
    pub roll_hat: f64,
    pub plus: Option<BodyCoeffs6>,
    pub minus: Option<BodyCoeffs6>,
}

/// Roll-damping derivative of one Mach number and where it came from.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClpEntry {
    pub mach: f64,
    /// `dCl/dp_hat` from CFD (`None`: the pair did not produce a number).
    pub cfd: Option<f64>,
    pub barrowman: f64,
    /// `cfd` or `barrowman`.
    pub source: String,
    pub reason: Option<String>,
}

/// Clp of every roll pair (see the module documentation for the acceptance rules).
pub fn roll_entries(pairs: &[RollPair], model: &AeroModel) -> Vec<ClpEntry> {
    let has_fins = model.geom.fins.as_ref().is_some_and(|f| f.count > 0 && f.span > 0.0);
    pairs
        .iter()
        .map(|p| {
            let barrowman = model.roll_terms(p.mach).1;
            let (cfd, reason) = match (p.plus, p.minus) {
                (Some(a), Some(b)) => {
                    let v = (a.cl - b.cl) / (2.0 * p.roll_hat);
                    if !v.is_finite() {
                        (None, Some("non-finite Clp".to_string()))
                    } else if v > 0.0 && has_fins {
                        (Some(v), Some(format!("positive Clp {v:.3e} (the roll damping must be negative)")))
                    } else if v.abs() < 1e-6 && has_fins {
                        (Some(v), Some(format!("|Clp| = {:.1e} is zero for a rocket with fins", v.abs())))
                    } else {
                        (Some(v), None)
                    }
                }
                _ => (None, Some("a roll case failed".to_string())),
            };
            let ok = cfd.is_some() && reason.is_none();
            ClpEntry { mach: p.mach, cfd, barrowman, source: if ok { "cfd" } else { "barrowman" }.into(), reason }
        })
        .collect()
}

/// Table value of `Clp` at Mach `m`: Barrowman scaled with the ratio CFD / Barrowman of the usable entries.
pub fn clp_at(entries: &[ClpEntry], model: &AeroModel, m: f64) -> f64 {
    let b = model.roll_terms(m).1;
    let used: Vec<&ClpEntry> = entries.iter().filter(|e| e.source == "cfd").collect();
    if used.is_empty() {
        return b;
    }
    let xs: Vec<f64> = used.iter().map(|e| e.mach).collect();
    if used.iter().all(|e| e.barrowman.abs() > 1e-12) {
        let ratio = Pchip::new(xs, used.iter().map(|e| e.cfd.unwrap() / e.barrowman).collect());
        b * ratio.eval(m, false)
    } else {
        Pchip::new(xs, used.iter().map(|e| e.cfd.unwrap()).collect()).eval(m, false)
    }
}

/// Solved region of the general table.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct Region {
    pub alpha_min_deg: f64,
    pub alpha_max_deg: f64,
    pub beta_min_deg: f64,
    pub beta_max_deg: f64,
}

/// Slopes at zero angle of one Mach number (diagnostics and damping scaling).
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Slopes {
    pub mach: f64,
    /// `-dCZ/dalpha` [1/rad].
    pub cna_pitch: f64,
    /// Pitch centre of pressure from the nose \[m\].
    pub xcp_pitch: f64,
    pub cna_yaw: Option<f64>,
    pub xcp_yaw: Option<f64>,
}

/// What the general builder derived.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GeneralFill {
    /// Cases without a result (bridged).
    pub filled: Vec<(f64, f64, f64)>,
    /// CFD region of the table; everywhere else the Barrowman model, offset-matched at the boundary.
    pub region: Region,
    pub slopes: Vec<Slopes>,
    pub yaw_from_pitch: bool,
    pub clp: Vec<ClpEntry>,
}

/// Inputs of [`build_general`].
pub struct GeneralInput<'a> {
    pub points: &'a [SolvedPoint6],
    pub roll: &'a [RollPair],
    pub model: &'a AeroModel,
    pub flow: FlowModel,
    pub source_hash: String,
    pub extrapolation: Extrapolation,
    pub shape: ShapeSymmetry,
    /// `mirror` mode: average all reflected images; otherwise solved points win.
    pub average: bool,
    /// `offset` mode: remove the zero-angle asymmetry.
    pub offset: bool,
    /// Mach step of the table; 0 = `model.opt.mach_step`.
    pub mach_step: f64,
    /// Fin cant \[deg\] contained in the CFD geometry (the reference of the roll forcing).
    pub fin_cant_ref_deg: f64,
    /// The CFD wall contains the fin leading / trailing faces (STL surface): the Euler hybrid axial
    /// force then omits the Barrowman fin-edge drag.
    pub fin_edges_in_cfd: bool,
    /// RANS: the base-cap force is in the CFD axial coefficient; otherwise the Barrowman base drag is added.
    pub base_from_cfd: bool,
}

fn key(a: f64, b: f64) -> (i64, i64) {
    ((a * 1e6).round() as i64, (b * 1e6).round() as i64)
}

/// Solved points of one Mach number completed by the mirror symmetries; keyed by `(alpha, beta)` in micro-degrees.
fn complete(pts: &[(f64, f64, BodyCoeffs6)], shape: ShapeSymmetry, average: bool, offset: bool) -> BTreeMap<(i64, i64), BodyCoeffs6> {
    let mut pts = pts.to_vec();
    if offset || average {
        // The lateral force and moments at zero angle are the numerical asymmetry of the mesh (see crate::asymmetry) and
        // are removed from every point. For a mirror-symmetric shape they are 0 exactly, and in `mirror` mode the
        // averaging with the images cancels them anyway; without an image (no mirror symmetry, CZ / Cm of a pitch-plane
        // shape) the subtraction is what removes them, together with the zero-angle force of an asymmetric detail,
        // which is small next to the spurious fin lift. The roll moment of a shape without mirror symmetry stays: it
        // is the roll forcing of canted fins.
        if let Some(&(_, _, z)) = pts.iter().find(|p| p.0 == 0.0 && p.1 == 0.0) {
            for p in &mut pts {
                p.2.cy -= z.cy;
                p.2.cz -= z.cz;
                p.2.cm -= z.cm;
                p.2.cn -= z.cn;
                if shape != ShapeSymmetry::None {
                    p.2.cl -= z.cl;
                }
            }
        }
    }
    // A shape that is mirror-symmetric about y = 0 has CY = Cl = Cn = 0 exactly for flow in the symmetry plane
    // (beta = 0); with a z = 0 symmetry too, CZ = Cl = Cm = 0 at alpha = 0. Whatever the solver leaves there
    // (symmetry-plane integration noise of a half domain) is removed, whatever the alpha mode (after the offset
    // correction, which needs the measured zero-angle values).
    if matches!(shape, ShapeSymmetry::Axisymmetric | ShapeSymmetry::PitchPlane) {
        for p in pts.iter_mut().filter(|p| p.1 == 0.0) {
            p.2.cy = 0.0;
            p.2.cl = 0.0;
            p.2.cn = 0.0;
        }
    }
    if shape == ShapeSymmetry::Axisymmetric {
        for p in pts.iter_mut().filter(|p| p.0 == 0.0) {
            p.2.cz = 0.0;
            p.2.cl = 0.0;
            p.2.cm = 0.0;
        }
    }
    let (mz, my) = match shape {
        ShapeSymmetry::Axisymmetric => (true, true),
        ShapeSymmetry::PitchPlane => (false, true),
        _ => (false, false),
    };
    let mut out = BTreeMap::new();
    let mean = |s: [f64; 6], n: usize| BodyCoeffs6::from_array(s.map(|v| v / n as f64));
    if average {
        let mut all = solved_only(&pts);
        for (k, (s, n)) in images_all(&pts, mz, my) {
            let e = all.entry(k).or_insert(([0.0; 6], 0));
            for (x, y) in e.0.iter_mut().zip(s) {
                *x += y;
            }
            e.1 += n;
        }
        for (k, (s, n)) in all {
            out.insert(k, mean(s, n));
        }
    } else {
        for (k, (s, n)) in solved_only(&pts) {
            out.insert(k, mean(s, n));
        }
        for (k, (s, n)) in images_all(&pts, mz, my) {
            out.entry(k).or_insert_with(|| mean(s, n));
        }
    }
    out
}

type Acc = BTreeMap<(i64, i64), ([f64; 6], usize)>;

fn accumulate(m: &mut Acc, a: f64, b: f64, c: BodyCoeffs6) {
    let e = m.entry(key(a, b)).or_insert(([0.0; 6], 0));
    for (s, v) in e.0.iter_mut().zip(c.as_array()) {
        *s += v;
    }
    e.1 += 1;
}

fn solved_only(pts: &[(f64, f64, BodyCoeffs6)]) -> Acc {
    let mut m = Acc::new();
    for &(a, b, c) in pts {
        accumulate(&mut m, a, b, c);
    }
    m
}

/// Images of the solved points under the reflections (without the identity).
fn images_all(pts: &[(f64, f64, BodyCoeffs6)], mz: bool, my: bool) -> Acc {
    let mut m = Acc::new();
    for &(a, b, c) in pts {
        if mz {
            accumulate(&mut m, -a, b, c.mirror_z());
        }
        if my {
            accumulate(&mut m, a, -b, c.mirror_y());
        }
        if mz && my {
            accumulate(&mut m, -a, -b, c.mirror_z().mirror_y());
        }
    }
    m
}

/// Per-Mach CFD interpolant on the solved rectangle.
struct Field {
    /// Sorted solved sideslip angles [deg] and the per-column Pchips of (cx, cy, cz, cl, cm, cn) over alpha.
    betas: Vec<f64>,
    cols: Vec<[Pchip; 6]>,
    region: Region,
}

impl Field {
    fn new(map: &BTreeMap<(i64, i64), BodyCoeffs6>) -> Option<Self> {
        let mut by_beta: BTreeMap<i64, Vec<(f64, BodyCoeffs6)>> = BTreeMap::new();
        for (&(a, b), &c) in map {
            by_beta.entry(b).or_default().push((a as f64 * 1e-6, c));
        }
        if by_beta.is_empty() {
            return None;
        }
        let mut betas = Vec::new();
        let mut cols = Vec::new();
        let mut region =
            Region { alpha_min_deg: f64::INFINITY, alpha_max_deg: f64::NEG_INFINITY, beta_min_deg: f64::INFINITY, beta_max_deg: f64::NEG_INFINITY };
        for (b, mut v) in by_beta {
            v.sort_by(|p, q| p.0.partial_cmp(&q.0).unwrap());
            let xs: Vec<f64> = v.iter().map(|p| p.0).collect();
            region.alpha_min_deg = region.alpha_min_deg.min(xs[0]);
            region.alpha_max_deg = region.alpha_max_deg.max(*xs.last().unwrap());
            let bd = b as f64 * 1e-6;
            region.beta_min_deg = region.beta_min_deg.min(bd);
            region.beta_max_deg = region.beta_max_deg.max(bd);
            betas.push(bd);
            cols.push(std::array::from_fn(|q| Pchip::new(xs.clone(), v.iter().map(|p| p.1.as_array()[q]).collect())));
        }
        Some(Self { betas, cols, region })
    }

    /// Value of the six coefficients at (alpha, beta) [deg] inside (or clamped to) the region.
    fn eval(&self, a: f64, b: f64) -> [f64; 6] {
        let a = a.clamp(self.region.alpha_min_deg, self.region.alpha_max_deg);
        let b = b.clamp(self.region.beta_min_deg, self.region.beta_max_deg);
        std::array::from_fn(|q| {
            if self.betas.len() == 1 {
                self.cols[0][q].eval(a, false)
            } else {
                Pchip::new(self.betas.clone(), self.cols.iter().map(|c| c[q].eval(a, false)).collect()).eval(b, false)
            }
        })
    }
}

/// Slope at zero angle of the odd part of `f` sampled at `(angle_deg, value)` pairs that include both signs.
fn zero_slope(samples: &[(f64, f64)]) -> Option<f64> {
    let mut odd: Vec<(f64, f64)> = Vec::new();
    for &(a, v) in samples.iter().filter(|s| s.0 > 0.0) {
        if let Some(&(_, w)) = samples.iter().find(|s| (s.0 + a).abs() < 1e-9) {
            odd.push((a.to_radians(), 0.5 * (v - w)));
        }
    }
    odd.sort_by(|p, q| p.0.partial_cmp(&q.0).unwrap());
    (!odd.is_empty()).then(|| odd_slope(&odd))
}

fn slopes_of(mach: f64, map: &BTreeMap<(i64, i64), BodyCoeffs6>, d: f64) -> Option<Slopes> {
    let col = |beta_fixed: Option<i64>, alpha_fixed: Option<i64>, sel: fn(&BodyCoeffs6) -> f64, vary_alpha: bool| -> Vec<(f64, f64)> {
        map.iter()
            .filter(|(k, _)| if vary_alpha { Some(k.1) == beta_fixed } else { Some(k.0) == alpha_fixed })
            .map(|(k, c)| (if vary_alpha { k.0 } else { k.1 } as f64 * 1e-6, sel(c)))
            .collect()
    };
    let dcz = zero_slope(&col(Some(0), None, |c| c.cz, true))?;
    let dcm = zero_slope(&col(Some(0), None, |c| c.cm, true))?;
    let cna_pitch = -dcz;
    let xcp_pitch = if dcz.abs() > 1e-9 { d * dcm / dcz } else { 0.0 };
    let dcy = zero_slope(&col(None, Some(0), |c| c.cy, false));
    let dcn = zero_slope(&col(None, Some(0), |c| c.cn, false));
    let (cna_yaw, xcp_yaw) = match (dcy, dcn) {
        (Some(y), Some(n)) => (Some(-y), Some(if y.abs() > 1e-9 { -d * n / y } else { 0.0 })),
        _ => (None, None),
    };
    Some(Slopes { mach, cna_pitch, xcp_pitch, cna_yaw, xcp_yaw })
}

/// Builds the general table (see the module documentation).
pub fn build_general(inp: &GeneralInput) -> Result<(AeroTable, GeneralFill)> {
    let model = inp.model;
    let o = &model.opt;
    let step = if inp.mach_step > 0.0 { inp.mach_step } else { o.mach_step };
    let machs = grid(o.mach_min.max(0.0), o.mach_max, step);
    let alphas = angle_axis(180.0, 30.0, 2.0, 10.0);
    let betas = angle_axis(90.0, 30.0, 2.0, 10.0);
    let d = 2.0 * model.geom.ref_radius;

    // Barrowman model expanded to the grid (total angle up to 90 deg).
    let mut mo = o.clone();
    mo.mach_step = step;
    mo.alpha_max_deg = 90.0;
    mo.alpha_step_deg = 1.0;
    let mm = AeroModel::new(model.geom.clone(), mo);
    let mt = AeroTable::build(&mm, String::new(), inp.extrapolation).to_general(alphas.clone(), betas.clone());

    let mut solved: Vec<f64> = inp.points.iter().map(|p| p.mach).collect();
    solved.sort_by(|a, b| a.partial_cmp(b).unwrap());
    solved.dedup();
    if solved.is_empty() {
        bail!("[IY-F003] no CFD cases to build a table from");
    }
    let mut fill = GeneralFill::default();
    struct Col {
        mach: f64,
        /// Node values [alpha][beta][cx_on, cx_off, cy, cz, cl, cm, cn].
        nodes: Vec<[f64; 7]>,
        slopes: Slopes,
    }
    let mut cols: Vec<Col> = Vec::new();
    let mut region = Region { alpha_min_deg: f64::INFINITY, alpha_max_deg: f64::NEG_INFINITY, beta_min_deg: f64::INFINITY, beta_max_deg: f64::NEG_INFINITY };
    for &m in &solved {
        let mut pts: Vec<(f64, f64, BodyCoeffs6)> = Vec::new();
        for p in inp.points.iter().filter(|p| p.mach == m) {
            match p.coeffs {
                Some(c) => pts.push((p.alpha_deg, p.beta_deg, c)),
                None => fill.filled.push((m, p.alpha_deg, p.beta_deg)),
            }
        }
        let map = complete(&pts, inp.shape, inp.average, inp.offset);
        let Some(field) = Field::new(&map) else { continue };
        let Some(slopes) = slopes_of(m, &map, d) else { continue };
        let r = field.region;
        region.alpha_min_deg = region.alpha_min_deg.min(r.alpha_min_deg);
        region.alpha_max_deg = region.alpha_max_deg.max(r.alpha_max_deg);
        region.beta_min_deg = region.beta_min_deg.min(r.beta_min_deg);
        region.beta_max_deg = region.beta_max_deg.max(r.beta_max_deg);
        let at = |a: f64, b: f64| mt.lookup6(m, a.to_radians(), b.to_radians());
        let mut nodes = Vec::with_capacity(alphas.len() * betas.len());
        for &a in &alphas {
            for &b in &betas {
                let (ah, bh) = (a.clamp(r.alpha_min_deg, r.alpha_max_deg), b.clamp(r.beta_min_deg, r.beta_max_deg));
                let f = field.eval(a, b);
                let (t, th) = (at(a, b), at(ah, bh));
                let total = (ah.to_radians().cos() * bh.to_radians().cos()).clamp(-1.0, 1.0).acos();
                let (ca_on, ca_off) = hybrid_axial(inp.flow, inp.fin_edges_in_cfd, inp.base_from_cfd, model, m, total.min(std::f64::consts::PI - total), -f[0]);
                nodes.push([
                    -ca_on + t.cx_on - th.cx_on,
                    -ca_off + t.cx_off - th.cx_off,
                    f[1] + t.cy - th.cy,
                    f[2] + t.cz - th.cz,
                    f[3] + t.cl - th.cl,
                    f[4] + t.cm - th.cm,
                    f[5] + t.cn - th.cn,
                ]);
            }
        }
        fill.slopes.push(slopes);
        cols.push(Col { mach: m, nodes, slopes });
    }
    for (edge, m) in [("lowest", solved[0]), ("highest", *solved.last().unwrap())] {
        if !cols.iter().any(|c| c.mach == m) {
            bail!("[IY-F004] all CFD cases at the {edge} Mach number ({m}) failed (or have no usable angles); the table cannot be bridged there");
        }
    }
    fill.region = region;
    let mx: Vec<f64> = cols.iter().map(|c| c.mach).collect();
    let across = |f: &dyn Fn(&Slopes) -> Option<f64>| -> Option<Pchip> {
        let v: Vec<(f64, f64)> = cols.iter().filter_map(|c| f(&c.slopes).map(|x| (c.mach, x))).collect();
        (!v.is_empty()).then(|| Pchip::new(v.iter().map(|p| p.0).collect(), v.iter().map(|p| p.1).collect()))
    };
    // The damping centroid is the ratio of the interpolated moment slope CNa x_cp to CNa (not the
    // interpolated x_cp), consistent with the interpolated forces and moments of the rows.
    let cna_p = across(&|s| Some(s.cna_pitch)).unwrap();
    let ma_p = across(&|s| Some(s.cna_pitch * s.xcp_pitch)).unwrap();
    let x_p = across(&|s| Some(s.xcp_pitch)).unwrap();
    let cna_y = across(&|s| s.cna_yaw);
    let ma_y = across(&|s| s.cna_yaw.zip(s.xcp_yaw).map(|(c, x)| c * x));
    let x_y = across(&|s| s.xcp_yaw);
    fill.yaw_from_pitch = cna_y.is_none() || x_y.is_none();
    let centroid = |cna: f64, ma: &Pchip, x: &Pchip, m: f64| if cna.abs() > 1e-9 { ma.eval(m, false) / cna } else { x.eval(m, false) };

    let (na, nb) = (alphas.len(), betas.len());
    let nm = machs.len();
    // Across Mach, node by node.
    let mut grid_vals: Vec<[f64; 7]> = vec![[0.0; 7]; nm * na * nb];
    for n in 0..na * nb {
        for q in 0..7 {
            let p = Pchip::new(mx.clone(), cols.iter().map(|c| c.nodes[n][q]).collect());
            for (i, &m) in machs.iter().enumerate() {
                let v = &mut grid_vals[i * na * nb + n];
                v[q] = p.eval(m, false);
            }
        }
    }
    fill.clp = roll_entries(inp.roll, model);
    let meta = TableMeta {
        source_hash: inp.source_hash.clone(),
        ref_area: model.geom.ref_area,
        ref_diameter: d,
        length: model.geom.length,
        machs: machs.clone(),
        alphas_deg: alphas,
        betas_deg: betas,
        fin_cant_ref_deg: inp.fin_cant_ref_deg,
        extrapolation: inp.extrapolation,
        ..Default::default()
    };
    // Per-Mach (not per-angle) terms.
    let per_mach: Vec<(f64, [f64; 3], [f64; 3], f64)> = machs
        .iter()
        .map(|&m| {
            let sums = model.damping_sums(m);
            let cna = cna_p.eval(m, false);
            let sq = scale_damping(sums, cna, centroid(cna, &ma_p, &x_p, m));
            let sr = match (&cna_y, &ma_y, &x_y) {
                (Some(c), Some(ma), Some(x)) => {
                    let cna = c.eval(m, false);
                    scale_damping(sums, cna, centroid(cna, ma, x, m))
                }
                _ => sq,
            };
            (m, sq, sr, clp_at(&fill.clp, model, m))
        })
        .collect();
    let counter = std::cell::Cell::new(0usize);
    let table = AeroTable::general_from_fn(meta, |m, _, _| {
        let k = counter.get();
        counter.set(k + 1);
        let v = grid_vals[k];
        let (i, ..) = (k / (na * nb), 0);
        let pm = &per_mach[i];
        debug_assert_eq!(pm.0, m);
        Coeffs6 {
            cx_on: v[0],
            cx_off: v[1],
            cy: v[2],
            cz: v[3],
            cl: v[4],
            cm: v[5],
            cn: v[6],
            cl_delta: model.roll_terms(m).0,
            clp: pm.3,
            sq: pm.1,
            sr: pm.2,
            ..Default::default()
        }
    });
    Ok((table, fill))
}

#[cfg(test)]
mod tests {
    use super::*;
    use aero::AeroOptions;
    use geom::sample::SampleRocket;
    use geom::{extract, ExtractOptions};

    fn model() -> AeroModel {
        let g = extract(&SampleRocket::default().mesh(), &ExtractOptions::default()).unwrap();
        AeroModel::new(g, AeroOptions { mach_max: 2.0, mach_step: 0.25, ..Default::default() })
    }

    // Analytic body-axis field of a doubly mirror-symmetric body (alpha, beta in deg), x_cp = 1.0 + 0.05 M.
    fn cn_total(m: f64, a: f64, b: f64) -> f64 {
        let ar = a.to_radians();
        let br = b.to_radians();
        (2.0 + 0.3 * m) * (ar.sin().powi(2) + br.sin().powi(2)).sqrt()
    }
    fn analytic(m: f64, a: f64, b: f64) -> BodyCoeffs6 {
        let (ar, br) = (a.to_radians(), b.to_radians());
        let lat = (ar.sin().powi(2) + br.sin().powi(2)).sqrt();
        let cn = cn_total(m, a, b);
        let (ey, ez) = if lat > 1e-12 { (br.sin() / lat, ar.sin() / lat) } else { (0.0, 0.0) };
        let (cy, cz) = (-cn * ey, -cn * ez);
        let xcp = 1.0 + 0.05 * m;
        let d = 0.1;
        BodyCoeffs6 { cx: -(0.2 + 0.1 * m + 0.002 * (a * a + b * b)), cy, cz, cl: 0.0, cm: xcp * cz / d, cn: -xcp * cy / d }
    }

    #[test]
    fn mirror_symmetry_completes_the_quadrants() {
        let mut pts = Vec::new();
        for &a in &[0.0, 4.0, 8.0] {
            for &b in &[0.0, 4.0] {
                pts.push((a, b, analytic(1.0, a, b)));
            }
        }
        let map = complete(&pts, ShapeSymmetry::Axisymmetric, true, false);
        assert_eq!(map.len(), 5 * 3);
        for (&(ka, kb), c) in &map {
            let (a, b) = (ka as f64 * 1e-6, kb as f64 * 1e-6);
            let e = analytic(1.0, a, b);
            for (x, y) in c.as_array().iter().zip(e.as_array()) {
                assert!((x - y).abs() < 1e-9, "({a},{b}) {c:?} vs {e:?}");
            }
        }
        // Pitch plane only: negative beta from the y reflection, no negative alpha.
        let map = complete(&pts, ShapeSymmetry::PitchPlane, false, false);
        assert!(map.keys().all(|k| k.0 >= 0) && map.contains_key(&key(8.0, -4.0)));
        // No symmetry: the solved points as they are.
        assert_eq!(complete(&pts, ShapeSymmetry::None, true, false).len(), 6);
    }

    #[test]
    fn symmetry_plane_noise_is_removed_for_symmetric_shapes() {
        let noisy = |a: f64| {
            let mut c = analytic(2.0, a, 0.0);
            c.cy = -0.043;
            c.cl = 0.01;
            c.cn = 0.54;
            (a, 0.0, c)
        };
        let pts = vec![noisy(0.0), noisy(2.0), noisy(4.0)];
        for (average, offset) in [(false, false), (false, true), (true, false)] {
            for shape in [ShapeSymmetry::Axisymmetric, ShapeSymmetry::PitchPlane] {
                let map = complete(&pts, shape, average, offset);
                for &a in &[0.0, 2.0, 4.0] {
                    let c = map[&key(a, 0.0)];
                    assert_eq!((c.cy, c.cl, c.cn), (0.0, 0.0, 0.0), "{shape:?} avg={average} off={offset} a={a}");
                }
            }
        }
        // No symmetry: the values are data.
        assert_eq!(complete(&pts, ShapeSymmetry::None, false, false)[&key(2.0, 0.0)].cn, 0.54);
    }

    #[test]
    fn averaging_removes_an_asymmetric_offset() {
        let p = |a: f64, off: f64| {
            let mut c = analytic(1.0, a, 0.0);
            c.cz += off;
            (a, 0.0, c)
        };
        let pts = vec![p(0.0, 0.01), p(4.0, 0.01), p(-4.0, 0.01)];
        let avg = complete(&pts, ShapeSymmetry::None, true, false);
        // Without symmetry there are no images: the zero-angle value is subtracted.
        assert!((avg[&key(4.0, 0.0)].cz - analytic(1.0, 4.0, 0.0).cz).abs() < 1e-12);
        assert!((avg[&key(-4.0, 0.0)].cz - analytic(1.0, -4.0, 0.0).cz).abs() < 1e-12);
        // Single mode keeps it.
        let raw = complete(&pts, ShapeSymmetry::None, false, false);
        assert!((raw[&key(4.0, 0.0)].cz - analytic(1.0, 4.0, 0.0).cz - 0.01).abs() < 1e-12);
        // Pitch-plane shape: CZ has no z image, the subtraction removes it in mirror mode too.
        let pp = complete(&pts, ShapeSymmetry::PitchPlane, true, false);
        assert!(pp[&key(0.0, 0.0)].cz.abs() < 1e-12 && (pp[&key(4.0, 0.0)].cz - analytic(1.0, 4.0, 0.0).cz).abs() < 1e-12);
        let pts_ax: Vec<_> = vec![p(0.0, 0.01), p(4.0, 0.01)];
        // Offset mode without symmetry: every lateral component of (0, 0) is removed, CX is kept.
        // The roll moment (fin cant) stays.
        let cant = |mut q: (f64, f64, BodyCoeffs6)| {
            q.2.cl = 0.03;
            q
        };
        let mut p0 = cant(p(0.0, 0.01));
        p0.2.cy += 0.1;
        p0.2.cn += 0.2;
        let none = complete(&[p0, cant(p(4.0, 0.01)), cant(p(-4.0, 0.01))], ShapeSymmetry::None, false, true);
        let (c0, c4) = (none[&key(0.0, 0.0)], none[&key(4.0, 0.0)]);
        let a4 = analytic(1.0, 4.0, 0.0);
        assert!([c0.cy, c0.cz, c0.cm, c0.cn].iter().all(|v| v.abs() < 1e-12) && c0.cl == 0.03 && c4.cl == 0.03, "{c0:?} {c4:?}");
        assert!((c4.cz - a4.cz).abs() < 1e-12 && (c4.cy - (a4.cy - 0.1)).abs() < 1e-12 && c4.cx == a4.cx, "{c4:?}");
        let off = complete(&pts_ax, ShapeSymmetry::Axisymmetric, false, true);
        assert!(off[&key(0.0, 0.0)].cz.abs() < 1e-12);
        assert!((off[&key(4.0, 0.0)].cz - analytic(1.0, 4.0, 0.0).cz).abs() < 1e-12);
    }

    #[test]
    fn general_builder_reproduces_an_analytic_field() {
        let m = model();
        let mut points = Vec::new();
        for &mach in &[0.5, 1.0, 1.5, 2.0] {
            for &a in &[0.0, 2.0, 4.0, 8.0, 12.0] {
                for &b in &[0.0, 2.0, 4.0, 8.0] {
                    points.push(SolvedPoint6 { mach, alpha_deg: a, beta_deg: b, coeffs: Some(analytic(mach, a, b)) });
                }
            }
        }
        let inp = GeneralInput {
            points: &points,
            roll: &[],
            model: &m,
            flow: FlowModel::Rans,
            source_hash: "h".into(),
            extrapolation: Extrapolation::Linear,
            shape: ShapeSymmetry::Axisymmetric,
            average: true,
            offset: false,
            mach_step: 0.25,
            fin_cant_ref_deg: 0.0,
            fin_edges_in_cfd: false,
            base_from_cfd: true,
        };
        let (t, fill) = build_general(&inp).unwrap();
        assert_eq!(t.kind(), aero::table::TableKind::General);
        assert_eq!(t.meta.alphas_deg.len(), 61);
        assert_eq!(t.meta.betas_deg.len(), 43);
        assert_eq!((fill.region.alpha_min_deg, fill.region.alpha_max_deg, fill.region.beta_min_deg, fill.region.beta_max_deg), (-12.0, 12.0, -8.0, 8.0));
        // Inside the solved region the table follows the analytic field (RANS: no hybrid terms but extra_cd = 0 here).
        for &(mach, a, b) in &[(1.0, 4.0, 4.0), (1.5, -8.0, 2.0), (0.5, 12.0, -8.0), (1.25, 6.0, 4.0)] {
            let c = t.lookup6(mach, f64::to_radians(a), f64::to_radians(b));
            let e = analytic(mach, a, b);
            let tol = if a == 6.0 || mach == 1.25 { 3e-2 } else { 1e-6 };
            assert!((c.cz - e.cz).abs() < tol * (1.0 + e.cz.abs()), "cz {} vs {} at {mach} {a} {b}", c.cz, e.cz);
            assert!((c.cy - e.cy).abs() < tol * (1.0 + e.cy.abs()), "cy {} vs {}", c.cy, e.cy);
            assert!((c.cm - e.cm).abs() < tol * (1.0 + e.cm.abs()), "cm {} vs {}", c.cm, e.cm);
            assert!((c.cn - e.cn).abs() < tol * (1.0 + e.cn.abs()), "cn {} vs {}", c.cn, e.cn);
        }
        // Symmetries of the table: CY odd in beta, CZ odd in alpha.
        let (a, b) = (10f64.to_radians(), 6f64.to_radians());
        let (p, q) = (t.lookup6(1.0, a, b), t.lookup6(1.0, a, -b));
        assert!((p.cy + q.cy).abs() < 1e-9 && (p.cz - q.cz).abs() < 1e-9);
        // Outside the solved region the model continues: continuous at the boundary, finite far away.
        let edge = t.lookup6(1.0, 12f64.to_radians(), 0.0);
        let beyond = t.lookup6(1.0, 12.5f64.to_radians(), 0.0);
        assert!((beyond.cz - edge.cz).abs() < 0.3 && (beyond.cz - edge.cz).abs() > 0.0);
        assert!(t.lookup6(1.0, 170f64.to_radians(), 30f64.to_radians()).cz.is_finite());
        // Slopes: CNa = 2 + 0.3 M per rad of total angle, x_cp = 1 + 0.05 M, pitch = yaw.
        let s = fill.slopes.iter().find(|s| s.mach == 1.0).unwrap();
        assert!((s.cna_pitch - 2.3).abs() < 0.02 && (s.xcp_pitch - 1.05).abs() < 0.01, "{s:?}");
        assert!((s.cna_yaw.unwrap() - 2.3).abs() < 0.02 && (s.xcp_yaw.unwrap() - 1.05).abs() < 0.01, "{s:?}");
        assert!(!fill.yaw_from_pitch);
        let c = t.lookup6(1.0, 0.0, 0.0);
        assert!((c.sq[0] - 2.3).abs() < 0.02 && (c.sq[1] / c.sq[0] - 1.05).abs() < 0.01 && (c.sr[0] - c.sq[0]).abs() < 0.02);
    }

    #[test]
    fn damping_centroid_follows_the_interpolated_moment_slope_across_mach() {
        // CN = k * total angle with k = 2 (Mach 1) and 6 (Mach 2), x_cp = 1 and 2 m: at Mach 1.5 the
        // interpolated slope is 4 and the moment slope (2 + 12) / 2 = 7, so the damping centroid
        // S1 / S0 is 1.75 m in pitch and yaw.
        let m = model();
        let field = |mach: f64, a: f64, b: f64| {
            let (k, xcp) = if mach == 1.0 { (2.0, 1.0) } else { (6.0, 2.0) };
            let (ar, br) = (f64::to_radians(a), f64::to_radians(b));
            let (cy, cz) = (-k * br.sin(), -k * ar.sin());
            BodyCoeffs6 { cx: -0.3, cy, cz, cl: 0.0, cm: xcp * cz / 0.1, cn: -xcp * cy / 0.1 }
        };
        let mut points = Vec::new();
        for &mach in &[1.0, 2.0] {
            for &a in &[0.0, 2.0, 4.0, 8.0] {
                for &b in &[0.0, 2.0, 4.0, 8.0] {
                    points.push(SolvedPoint6 { mach, alpha_deg: a, beta_deg: b, coeffs: Some(field(mach, a, b)) });
                }
            }
        }
        let inp = GeneralInput {
            points: &points,
            roll: &[],
            model: &m,
            flow: FlowModel::Rans,
            source_hash: "h".into(),
            extrapolation: Extrapolation::Linear,
            shape: ShapeSymmetry::Axisymmetric,
            average: true,
            offset: false,
            mach_step: 0.25,
            fin_cant_ref_deg: 0.0,
            fin_edges_in_cfd: false,
            base_from_cfd: true,
        };
        let (t, _) = build_general(&inp).unwrap();
        let c = t.lookup6(1.5, 0.0, 0.0);
        assert!((c.sq[1] / c.sq[0] - 1.75).abs() < 1e-3 && (c.sr[1] / c.sr[0] - 1.75).abs() < 1e-3, "{c:?}");
    }

    #[test]
    fn single_beta_column_uses_model_in_beta_and_pitch_damping_for_yaw() {
        let m = model();
        let mut points = Vec::new();
        for &mach in &[0.5, 2.0] {
            for &a in &[-8.0, -4.0, 0.0, 4.0, 8.0] {
                points.push(SolvedPoint6 { mach, alpha_deg: a, beta_deg: 0.0, coeffs: Some(analytic(mach, a, 0.0)) });
            }
        }
        let inp = GeneralInput {
            points: &points,
            roll: &[],
            model: &m,
            flow: FlowModel::Rans,
            source_hash: String::new(),
            extrapolation: Extrapolation::Clamp,
            shape: ShapeSymmetry::None,
            average: false,
            offset: false,
            mach_step: 0.5,
            fin_cant_ref_deg: 0.0,
            fin_edges_in_cfd: false,
            base_from_cfd: true,
        };
        let (t, fill) = build_general(&inp).unwrap();
        assert!(fill.yaw_from_pitch && fill.region.beta_max_deg == 0.0);
        let c = t.lookup6(2.0, 4f64.to_radians(), 0.0);
        assert!((c.cz - analytic(2.0, 4.0, 0.0).cz).abs() < 1e-9);
        // Sideslip comes from the model, which has a finite side force.
        assert!(t.lookup6(2.0, 0.0, 6f64.to_radians()).cy.abs() > 1e-3);
        assert_eq!(c.sq, c.sr);
    }

    #[test]
    fn clp_entries_accept_and_reject() {
        let m = model();
        let c = |cl: f64| Some(BodyCoeffs6 { cl, ..Default::default() });
        let b = m.roll_terms(0.8).1;
        assert!(b < 0.0);
        let e = roll_entries(
            &[
                RollPair { mach: 0.8, roll_hat: 0.01, plus: c(-0.2), minus: c(0.2) },
                RollPair { mach: 1.5, roll_hat: 0.01, plus: c(0.2), minus: c(-0.2) },
                RollPair { mach: 2.0, roll_hat: 0.01, plus: None, minus: c(0.1) },
                RollPair { mach: 0.3, roll_hat: 0.01, plus: c(-1e-9), minus: c(1e-9) },
            ],
            &m,
        );
        assert_eq!((e[0].source.as_str(), e[0].cfd), ("cfd", Some(-20.0)));
        assert_eq!(e[1].source, "barrowman");
        assert!(e[1].reason.as_ref().unwrap().contains("positive"));
        assert!(e[2].reason.as_ref().unwrap().contains("failed") && e[2].cfd.is_none());
        assert!(e[3].reason.as_ref().unwrap().contains("zero"));
        // One usable Mach: Barrowman scaled by the ratio there; with none usable the Barrowman value.
        let ok = vec![e[0].clone()];
        assert!((clp_at(&ok, &m, 0.8) + 20.0).abs() < 1e-9);
        assert!((clp_at(&ok, &m, 2.0) - m.roll_terms(2.0).1 * (-20.0 / b)).abs() < 1e-9);
        assert_eq!(clp_at(&[], &m, 0.8), b);
    }

    /// General table: the CFD axial coefficient is taken as is; the Barrowman base is added only when the base is not
    /// part of it.
    #[test]
    fn general_table_adds_the_barrowman_base_only_without_the_cfd_base() {
        let m = model();
        let mut points = Vec::new();
        for &mach in &[0.5, 2.0] {
            for &a in &[0.0, 2.0, 4.0] {
                for &b in &[0.0, 2.0, 4.0] {
                    let mut c = analytic(mach, a, b);
                    c.cx = -0.3;
                    points.push(SolvedPoint6 { mach, alpha_deg: a, beta_deg: b, coeffs: Some(c) });
                }
            }
        }
        let build = |base_from_cfd: bool| {
            let inp = GeneralInput {
                points: &points,
                roll: &[],
                model: &m,
                flow: FlowModel::Rans,
                source_hash: "h".into(),
                extrapolation: Extrapolation::Linear,
                shape: ShapeSymmetry::Axisymmetric,
                average: true,
                offset: false,
                mach_step: 0.25,
                fin_cant_ref_deg: 0.0,
                fin_edges_in_cfd: false,
                base_from_cfd,
            };
            build_general(&inp).unwrap().0
        };
        let b = m.cd0(0.5, false);
        let r = crate::forces::roughness_increment(&m, 0.5) + crate::forces::fin_edge_increment(&m, 0.5, false);
        let (model_base, cfd_base) = (build(false).lookup6(0.5, 0.0, 0.0), build(true).lookup6(0.5, 0.0, 0.0));
        assert!((-cfd_base.cx_off - (0.3 + b.extra + r)).abs() < 1e-6, "{}", cfd_base.cx_off);
        assert!((-model_base.cx_off - (0.3 + b.base + b.extra + r)).abs() < 1e-6, "{}", model_base.cx_off);
    }
}
