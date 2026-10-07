// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025-2026 西濱大将 (NISHIHAMA Daisuke)
//! Fills the regular [`AeroTable`] grid from the solved (Mach, alpha) points.
//!
//! Per solved Mach column (alpha direction): `CN` and the nose moment `CN xcp` are odd in alpha
//! and anchored at `(0, 0)`; they and the CFD axial coefficient are interpolated with monotone
//! cubics (PCHIP) in alpha. Beyond the highest solved angle the coefficients blend into the Barrowman
//! model (see [`beyond_solved`]); the end slope is never continued. `CNa` and `xcp(0)` come from an
//! odd cubic fit `c1 a + c3 a^3` through the two lowest positive solved angles, and the PCHIPs of
//! `CN` and the moment start at `(0, 0)` with these slopes, so the slope, the damping sums and the
//! tabulated rows describe one curve. Failed cases
//! (`coeffs = None`) are simply left out of their column. Across Mach number every quantity is then
//! interpolated with PCHIP on the table's Mach grid (edge values held; `xcp(0)` as the ratio of the
//! interpolated moment slope `CNa xcp(0)` to `CNa`), so a failed
//! interior Mach column is bridged by its neighbours; a failed first or last Mach column cannot be
//! bridged and is an error. Finally hybrid viscous terms ([`crate::forces::hybrid_axial`]) are
//! added and the pitch-damping sums are scaled to the CFD slope and centroid.

use crate::config::{AlphaMode, FlowModel};
use crate::forces::{hybrid_axial, scale_damping, BodyCoeffs};
use aero::table::{AeroCoeffs, TableMeta};
use aero::{AeroModel, AeroTable, Extrapolation};
use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};

/// One solved (or failed) point of the case matrix.
#[derive(Debug, Clone, Copy)]
pub struct SolvedPoint {
    pub mach: f64,
    pub alpha_deg: f64,
    pub coeffs: Option<BodyCoeffs>,
}

/// Removes the numerical zero-angle offset of the normal force and nose moment.
///
/// The body is axisymmetric / mirror symmetric, so `CN(0) = 0` exactly; the steady solution on an
/// unstructured mesh can nevertheless carry a (sometimes large, mesh-dependent) asymmetric offset: a spurious
/// lift of under-resolved fins at subsonic Mach numbers (see [`crate::asymmetry`]). Per Mach number the `alpha = 0` values of `CN` and the
/// nose moment are subtracted from all angles of that Mach number (and the `alpha = 0` point itself
/// becomes 0). The axial coefficient is left alone. Returns the removed `(mach, CN0, moment0)`.
pub fn remove_zero_offset(points: &mut [SolvedPoint]) -> Vec<(f64, f64, f64)> {
    let mut removed = Vec::new();
    let zero: Vec<(f64, BodyCoeffs)> = points.iter().filter(|p| p.alpha_deg == 0.0).filter_map(|p| p.coeffs.map(|c| (p.mach, c))).collect();
    for (m, c0) in zero {
        for p in points.iter_mut().filter(|p| p.mach == m) {
            if let Some(c) = &mut p.coeffs {
                c.cn -= c0.cn;
                c.mom -= c0.mom;
            }
        }
        removed.push((m, c0.cn, c0.mom));
    }
    removed
}

/// Measured zero-angle asymmetry of one Mach number: `CN` and nose moment [m] at `alpha = 0`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Asymmetry {
    pub mach: f64,
    pub cn0: f64,
    pub mom0: f64,
}

/// Turns the solved points into the odd/even parts used by the table, according to `mode`.
///
/// * `Mirror`: for every `alpha > 0` with a solved `-alpha` partner the result is
///   `CN = (CN(+a) - CN(-a)) / 2`, nose moment likewise, `CA = (CA(+a) + CA(-a)) / 2`. A numerical
///   offset `c0` that is the same for both signs (`f(+a) = odd + c0`, `f(-a) = -odd + c0`) cancels
///   exactly. When one partner failed, the other is used with the `alpha = 0` value as the offset
///   estimate (raw if that failed too). The `alpha = 0` point keeps only its `CA`; its `CN` and moment
///   (the asymmetry) are returned, not used. Negative angles are consumed.
/// * `Offset`: the `alpha = 0` `CN` / moment are subtracted from every angle of that Mach number.
/// * `Single`: unchanged.
///
/// Returns the points for [`build_from_points`] and the measured asymmetry per Mach.
pub fn apply_alpha_mode(points: &[SolvedPoint], mode: AlphaMode) -> (Vec<SolvedPoint>, Vec<Asymmetry>) {
    let mut asym = Vec::new();
    for p in points.iter().filter(|p| p.alpha_deg == 0.0) {
        if let Some(c) = p.coeffs {
            asym.push(Asymmetry { mach: p.mach, cn0: c.cn, mom0: c.mom });
        }
    }
    let out = match mode {
        AlphaMode::Single => points.to_vec(),
        AlphaMode::Offset => {
            let mut v = points.to_vec();
            remove_zero_offset(&mut v);
            v
        }
        AlphaMode::Mirror => {
            let find = |m: f64, a: f64| points.iter().find(|q| q.mach == m && q.alpha_deg == a).and_then(|q| q.coeffs);
            let mut v = Vec::new();
            for p in points.iter().filter(|p| p.alpha_deg >= 0.0) {
                let coeffs = if p.alpha_deg == 0.0 {
                    p.coeffs.map(|c| BodyCoeffs { cn: 0.0, mom: 0.0, ..c })
                } else {
                    let zero = find(p.mach, 0.0);
                    let (c0, m0) = zero.map_or((0.0, 0.0), |z| (z.cn, z.mom));
                    match (p.coeffs, find(p.mach, -p.alpha_deg)) {
                        (Some(a), Some(b)) => Some(BodyCoeffs { cn: 0.5 * (a.cn - b.cn), ca: 0.5 * (a.ca + b.ca), mom: 0.5 * (a.mom - b.mom) }),
                        (Some(a), None) => Some(BodyCoeffs { cn: a.cn - c0, mom: a.mom - m0, ..a }),
                        (None, Some(b)) => Some(BodyCoeffs { cn: c0 - b.cn, mom: m0 - b.mom, ..b }),
                        (None, None) => None,
                    }
                };
                v.push(SolvedPoint { coeffs, ..*p });
            }
            v
        }
    };
    (out, asym)
}

/// What the table builder derived.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TableFill {
    /// Cases that failed and were bridged by interpolation.
    pub filled: Vec<(f64, f64)>,
    /// Per usable solved Mach: `(mach, CNa [1/rad], xcp at alpha -> 0 [m])`.
    pub slopes: Vec<(f64, f64, f64)>,
}

/// Monotone cubic (Fritsch-Carlson) interpolant with selectable extrapolation.
pub(crate) struct Pchip {
    xs: Vec<f64>,
    ys: Vec<f64>,
    m: Vec<f64>,
}

impl Pchip {
    pub(crate) fn new(xs: Vec<f64>, ys: Vec<f64>) -> Self {
        let n = xs.len();
        let mut m = vec![0.0; n];
        if n >= 2 {
            let h: Vec<f64> = xs.windows(2).map(|w| w[1] - w[0]).collect();
            let d: Vec<f64> = (0..n - 1).map(|i| (ys[i + 1] - ys[i]) / h[i]).collect();
            if n == 2 {
                m = vec![d[0]; 2];
            } else {
                for i in 1..n - 1 {
                    if d[i - 1] * d[i] > 0.0 {
                        let (w1, w2) = (2.0 * h[i] + h[i - 1], h[i] + 2.0 * h[i - 1]);
                        m[i] = (w1 + w2) / (w1 / d[i - 1] + w2 / d[i]);
                    }
                }
                let end = |h0: f64, h1: f64, d0: f64, d1: f64| {
                    let s = ((2.0 * h0 + h1) * d0 - h0 * d1) / (h0 + h1);
                    if s * d0 <= 0.0 {
                        0.0
                    } else if d0 * d1 <= 0.0 && s.abs() > 3.0 * d0.abs() {
                        3.0 * d0
                    } else {
                        s
                    }
                };
                m[0] = end(h[0], h[1], d[0], d[1]);
                m[n - 1] = end(h[n - 2], h[n - 3], d[n - 2], d[n - 3]);
            }
        }
        Self { xs, ys, m }
    }

    /// Sets the slope at the first node to `slope`; it must keep the sign of the first secant and
    /// stay within three times it (monotonicity), as [`odd_slope`] does.
    fn with_start_slope(mut self, slope: f64) -> Self {
        if let Some(m0) = self.m.first_mut() {
            *m0 = slope;
        }
        self
    }

    pub(crate) fn eval(&self, x: f64, linear: bool) -> f64 {
        let n = self.xs.len();
        if x <= self.xs[0] {
            return self.ys[0] + if linear { self.m[0] * (x - self.xs[0]) } else { 0.0 };
        }
        if x >= self.xs[n - 1] {
            return self.ys[n - 1] + if linear { self.m[n - 1] * (x - self.xs[n - 1]) } else { 0.0 };
        }
        let i = self.xs.partition_point(|&v| v <= x) - 1;
        geom::math::hermite(x, self.xs[i], self.xs[i + 1], self.ys[i], self.ys[i + 1], self.m[i], self.m[i + 1])
    }
}

pub(crate) fn grid(lo: f64, hi: f64, step: f64) -> Vec<f64> {
    let n = ((hi - lo) / step).round().max(1.0) as usize;
    (0..=n).map(|i| lo + (hi - lo) * i as f64 / n as f64).collect()
}

/// Slope at alpha -> 0 of an odd function from samples `(alpha_rad, value)`: fit `c1 a + c3 a^3`
/// through the two lowest points (falls back to the secant when the fit looks unreasonable).
pub(crate) fn odd_slope(pts: &[(f64, f64)]) -> f64 {
    let (a1, v1) = pts[0];
    let r1 = v1 / a1;
    if pts.len() < 2 {
        return r1;
    }
    let (a2, v2) = pts[1];
    let r2 = v2 / a2;
    let c3 = (r2 - r1) / (a2 * a2 - a1 * a1);
    let c1 = r1 - c3 * a1 * a1;
    if c1.is_finite() && (c1 - r1).abs() <= 0.5 * r1.abs().max(1e-9) {
        c1
    } else {
        r1
    }
}

/// Table-grid values of one solved Mach column.
struct Column {
    mach: f64,
    cn: Vec<f64>,
    mom: Vec<f64>,
    ca: Vec<f64>,
    cna: f64,
    xcp0: f64,
}

fn column(mach: f64, pts: &[(f64, BodyCoeffs)], alphas: &[f64], model: &AeroModel) -> Option<Column> {
    let pos: Vec<&(f64, BodyCoeffs)> = pts.iter().filter(|p| p.0 > 0.0).collect();
    if pos.is_empty() {
        return None;
    }
    let rad = |p: &&(f64, BodyCoeffs)| p.0.to_radians();
    let cn_pts: Vec<(f64, f64)> = pos.iter().map(|p| (rad(p), p.1.cn)).collect();
    let mom_pts: Vec<(f64, f64)> = pos.iter().map(|p| (rad(p), p.1.mom)).collect();
    let cna = odd_slope(&cn_pts);
    let m1 = odd_slope(&mom_pts);
    // The normal force and nose moment between the nodes start from zero with these slopes, so the
    // table's CNa, centre of pressure at zero angle and damping sums belong to the same curve as its
    // CN and moment rows.
    let mut xs = vec![0.0];
    xs.extend(pos.iter().map(|p| p.0));
    let per_deg = std::f64::consts::PI / 180.0;
    let cn_p = Pchip::new(xs.clone(), std::iter::once(0.0).chain(pos.iter().map(|p| p.1.cn)).collect()).with_start_slope(cna * per_deg);
    let mom_p = Pchip::new(xs, std::iter::once(0.0).chain(pos.iter().map(|p| p.1.mom)).collect()).with_start_slope(m1 * per_deg);
    let ca_p = Pchip::new(pts.iter().map(|p| p.0).collect(), pts.iter().map(|p| p.1.ca).collect());
    let xcp0 = if cna.abs() > 1e-9 { m1 / cna } else { 0.0 };
    let edge = pos.last().unwrap();
    let (a_e, e) = (edge.0, edge.1);
    let xcp_e = if e.cn.abs() > 1e-9 { e.mom / e.cn } else { xcp0 };
    let (mut cn, mut mom, mut ca) = (Vec::with_capacity(alphas.len()), Vec::with_capacity(alphas.len()), Vec::with_capacity(alphas.len()));
    for &a in alphas {
        if a <= a_e {
            cn.push(cn_p.eval(a, false));
            mom.push(mom_p.eval(a, false));
            ca.push(ca_p.eval(a, false));
        } else {
            let b = beyond_solved(model, mach, a, a_e, e.cn, xcp_e, ca_p.eval(a_e, false));
            cn.push(b.0);
            mom.push(b.0 * b.1);
            ca.push(b.2);
        }
    }
    Some(Column { mach, cn, mom, ca, cna, xcp0 })
}

/// Angle, as a multiple of the highest solved angle, at which the coefficients beyond the solved range
/// have relaxed to the Barrowman model (capped at 90°).
pub const RELAX_FACTOR: f64 = 2.0;

/// `(CN, xcp [m], CA)` at `a_deg` above the highest solved angle `a_e` [deg] of a Mach column, from the
/// CFD values at `a_e` (`cn_e`, `xcp_e`, `ca_e`) and the Barrowman model.
///
/// CFD gives no data beyond the solved angles, and continuing the end slope is not usable for a
/// descending rocket: a straight line through the last solved cell moves the centre of pressure without
/// bound and can put it ahead of the centre of gravity at large angles, which creates an artificial stable
/// trim at a high angle of attack (a slow, far-drifting ballistic descent). Instead the coefficients follow
/// the Barrowman model, whose fin normal force is bounded at large angles (crossflow plate value, see
/// [`AeroModel::fin_cn`]) and whose fin centre of pressure moves to the planform centroid:
///
/// * `CN = CN_B(a) (1 + (s - 1) w(a))` with `s = CN_e / CN_B(a_e)` (an offset `CN_e - CN_B(a_e)` scaled by
///   `w` when `s` is not positive),
/// * `xcp = xcp_B(a) + (xcp_e - xcp_B(a_e)) w(a)`,
/// * `CA = CA_e k(a) / k(a_e)` with the Barrowman angle-of-attack factor `k` ([`aero::model::axial_alpha_factor`]),
///
/// where `w` falls from 1 at `a_e` to 0 at `a_r = min(RELAX_FACTOR a_e, 90°)` with a C1 smoothstep. The values
/// match the CFD values at `a_e` and equal the Barrowman shape beyond `a_r` (the CA keeps the CFD level
/// times the Barrowman shape, which is 0 at 90°). The table is interpolated linearly between its nodes, so
/// the value match is the continuity that matters.
pub fn beyond_solved(model: &AeroModel, mach: f64, a_deg: f64, a_e: f64, cn_e: f64, xcp_e: f64, ca_e: f64) -> (f64, f64, f64) {
    let a_r = (RELAX_FACTOR * a_e).min(90.0).max(a_e);
    let w = if a_deg >= a_r || a_r <= a_e { 0.0 } else { geom::math::hermite(a_deg, a_e, a_r, 1.0, 0.0, 0.0, 0.0) };
    let (cn_be, x_be) = model.normal(mach, a_e.to_radians());
    let (cn_b, x_b) = model.normal(mach, a_deg.to_radians());
    let s = if cn_be.abs() > 1e-9 { cn_e / cn_be } else { 0.0 };
    let cn = if s > 0.0 { cn_b * (1.0 + (s - 1.0) * w) } else { cn_b + (cn_e - cn_be) * w };
    let mut xcp = x_b + (xcp_e - x_be) * w;
    if model.geom.length > 0.0 {
        xcp = xcp.clamp(0.0, model.geom.length);
    }
    let (k, k_e) = (aero::model::axial_alpha_factor(a_deg.to_radians()), aero::model::axial_alpha_factor(a_e.to_radians()));
    let ca = if k_e > 1e-9 { ca_e * k / k_e } else { ca_e };
    (cn, xcp, ca)
}

/// Builds the table on the grid of `model.opt` (Mach `mach_min..mach_max`, alpha `0..alpha_max`).
///
/// Above the highest solved angle of each Mach column the coefficients come from [`beyond_solved`]
/// (blend into the Barrowman model), independent of `extrapolation`; `extrapolation` is stored in the
/// table and applies to lookups beyond the table grid.
/// The CFD axial coefficient of the points is taken to include the base (`base_from_cfd = true`).
pub fn build_from_points(
    points: &[SolvedPoint],
    model: &AeroModel,
    flow: FlowModel,
    source_hash: String,
    extrapolation: Extrapolation,
) -> Result<(AeroTable, TableFill)> {
    build_from_points_roll(points, model, flow, source_hash, extrapolation, &[], true)
}

/// [`build_from_points`] with the roll-damping entries of the CFD roll pairs (empty: Barrowman `Clp`).
/// `cl_delta` is always the Barrowman cant forcing; the table's reference cant is the geometry cant.
pub fn build_from_points_roll(
    points: &[SolvedPoint],
    model: &AeroModel,
    flow: FlowModel,
    source_hash: String,
    extrapolation: Extrapolation,
    roll: &[crate::general::ClpEntry],
    base_from_cfd: bool,
) -> Result<(AeroTable, TableFill)> {
    let o = &model.opt;
    let machs = grid(o.mach_min.max(0.0), o.mach_max, o.mach_step);
    let alphas = grid(0.0, o.alpha_max_deg, o.alpha_step_deg);

    let mut solved: Vec<f64> = points.iter().map(|p| p.mach).collect();
    solved.sort_by(|a, b| a.partial_cmp(b).unwrap());
    solved.dedup();
    if solved.is_empty() {
        bail!("no CFD cases to build a table from");
    }
    let mut fill = TableFill::default();
    let mut cols: Vec<Column> = Vec::new();
    for &m in &solved {
        let mut pts: Vec<(f64, BodyCoeffs)> = Vec::new();
        for p in points.iter().filter(|p| p.mach == m) {
            match p.coeffs {
                Some(c) => pts.push((p.alpha_deg, c)),
                None => fill.filled.push((m, p.alpha_deg)),
            }
        }
        pts.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        if let Some(c) = column(m, &pts, &alphas, model) {
            fill.slopes.push((m, c.cna, c.xcp0));
            cols.push(c);
        }
    }
    for (edge, m) in [("lowest", solved[0]), ("highest", *solved.last().unwrap())] {
        if !cols.iter().any(|c| c.mach == m) {
            bail!("all CFD cases at the {edge} Mach number ({m}) failed (or have no angle of attack above 0); the table cannot be bridged there");
        }
    }

    let mx: Vec<f64> = cols.iter().map(|c| c.mach).collect();
    let across = |f: &dyn Fn(&Column) -> f64| Pchip::new(mx.clone(), cols.iter().map(f).collect());
    // The zero-angle centre of pressure is the ratio of the interpolated slopes (moment slope
    // CNa xcp0 over CNa), as the nonzero-angle rows divide the interpolated moment by CN.
    let cna_p = across(&|c| c.cna);
    let ma_p = across(&|c| c.cna * c.xcp0);
    let x0_p = across(&|c| c.xcp0);
    let per_alpha =
        |sel: fn(&Column) -> &Vec<f64>| -> Vec<Pchip> { (0..alphas.len()).map(|j| Pchip::new(mx.clone(), cols.iter().map(|c| sel(c)[j]).collect())).collect() };
    let (cn_p, mom_p, ca_p) = (per_alpha(|c| &c.cn), per_alpha(|c| &c.mom), per_alpha(|c| &c.ca));

    let mut rows: Vec<Vec<AeroCoeffs>> = Vec::with_capacity(machs.len());
    for &m in &machs {
        let cna = cna_p.eval(m, false);
        let xcp0 = if cna.abs() > 1e-9 { ma_p.eval(m, false) / cna } else { x0_p.eval(m, false) };
        let damp = scale_damping(model.damping_sums(m), cna, xcp0);
        let (cl_delta, _) = model.roll_terms(m);
        let clp = crate::general::clp_at(roll, model, m);
        let row = alphas
            .iter()
            .enumerate()
            .map(|(j, &a)| {
                let (cn, mom) = if a == 0.0 { (0.0, 0.0) } else { (cn_p[j].eval(m, false), mom_p[j].eval(m, false)) };
                let xcp = if cn.abs() > 1e-9 { mom / cn } else { xcp0 };
                let (ca_on, ca_off) = hybrid_axial(flow, false, base_from_cfd, model, m, a.to_radians(), ca_p[j].eval(m, false));
                AeroCoeffs { cn, ca_on, ca_off, xcp, cna, damp, cl_delta, clp, ..Default::default() }
            })
            .collect();
        rows.push(row);
    }
    let g = &model.geom;
    let meta = TableMeta {
        source_hash,
        ref_area: g.ref_area,
        ref_diameter: 2.0 * g.ref_radius,
        length: g.length,
        machs: machs.clone(),
        alphas_deg: alphas.clone(),
        fin_cant_ref_deg: g.fins.as_ref().map_or(0.0, |f| f.cant_deg),
        extrapolation,
        ..Default::default()
    };
    let table = AeroTable::from_fn(meta, |m, a| {
        let i = machs.iter().position(|&v| v == m).unwrap();
        let j = alphas.iter().position(|&v| v == a).unwrap();
        rows[i][j]
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
        AeroModel::new(g, AeroOptions { mach_max: 3.0, mach_step: 0.25, alpha_max_deg: 16.0, alpha_step_deg: 2.0, ..Default::default() })
    }

    // Analytic field: CN = (2 + 0.3 M) sin 2a, nose moment = CN (1.0 + 0.05 M), CA = 0.2 + 0.1 M + 0.002 a^2 (a in deg).
    fn cn(m: f64, a: f64) -> f64 {
        (2.0 + 0.3 * m) * (2.0 * a.to_radians()).sin()
    }
    fn xcp(m: f64) -> f64 {
        1.0 + 0.05 * m
    }
    fn ca(m: f64, a: f64) -> f64 {
        0.2 + 0.1 * m + 0.002 * a * a
    }

    fn points(machs: &[f64], alphas: &[f64]) -> Vec<SolvedPoint> {
        let mut v = Vec::new();
        for &m in machs {
            for &a in alphas {
                v.push(SolvedPoint { mach: m, alpha_deg: a, coeffs: Some(BodyCoeffs { cn: cn(m, a), ca: ca(m, a), mom: cn(m, a) * xcp(m) }) });
            }
        }
        v
    }

    const MACHS: [f64; 6] = [0.5, 1.0, 1.5, 2.0, 2.5, 3.0];
    const ALPHAS: [f64; 6] = [0.0, 2.0, 4.0, 8.0, 12.0, 16.0];

    #[test]
    fn zero_offset_is_removed_per_mach() {
        let mut p = vec![
            SolvedPoint { mach: 0.5, alpha_deg: 0.0, coeffs: Some(BodyCoeffs { cn: 0.3, ca: 0.1, mom: 0.4 }) },
            SolvedPoint { mach: 0.5, alpha_deg: 4.0, coeffs: Some(BodyCoeffs { cn: 1.3, ca: 0.1, mom: 1.9 }) },
            SolvedPoint { mach: 2.0, alpha_deg: 0.0, coeffs: None },
            SolvedPoint { mach: 2.0, alpha_deg: 4.0, coeffs: Some(BodyCoeffs { cn: 1.0, ca: 0.2, mom: 1.0 }) },
        ];
        let r = remove_zero_offset(&mut p);
        assert_eq!(r, vec![(0.5, 0.3, 0.4)]);
        assert_eq!(p[0].coeffs.unwrap().cn, 0.0);
        assert!((p[1].coeffs.unwrap().cn - 1.0).abs() < 1e-12 && (p[1].coeffs.unwrap().mom - 1.5).abs() < 1e-12);
        assert_eq!(p[1].coeffs.unwrap().ca, 0.1);
        assert_eq!(p[3].coeffs.unwrap().cn, 1.0, "no alpha = 0 result: unchanged");
    }

    #[test]
    fn mirror_combination_cancels_a_common_offset() {
        let pt = |m: f64, a: f64, cn: f64, ca: f64, mom: f64| SolvedPoint { mach: m, alpha_deg: a, coeffs: Some(BodyCoeffs { cn, ca, mom }) };
        // f(+a) = odd + c0, f(-a) = -odd + c0 with c0 = (0.3 CN, 0.4 moment); CA even part 0.2 +- 0.01.
        let p = vec![
            pt(0.5, 0.0, 0.3, 0.19, 0.4),
            pt(0.5, 4.0, 1.3, 0.21, 1.9),
            pt(0.5, 8.0, 2.3, 0.25, 3.4),
            pt(0.5, -4.0, -0.7, 0.20, -1.1),
            pt(0.5, -8.0, -1.7, 0.23, -2.6),
        ];
        let (v, asym) = apply_alpha_mode(&p, AlphaMode::Mirror);
        assert_eq!(asym, vec![Asymmetry { mach: 0.5, cn0: 0.3, mom0: 0.4 }]);
        assert_eq!(v.len(), 3, "negative angles are consumed");
        let c = |a: f64| v.iter().find(|q| q.alpha_deg == a).unwrap().coeffs.unwrap();
        assert_eq!((c(0.0).cn, c(0.0).mom, c(0.0).ca), (0.0, 0.0, 0.19));
        assert!((c(4.0).cn - 1.0).abs() < 1e-12 && (c(4.0).mom - 1.5).abs() < 1e-12 && (c(4.0).ca - 0.205).abs() < 1e-12);
        assert!((c(8.0).cn - 2.0).abs() < 1e-12 && (c(8.0).mom - 3.0).abs() < 1e-12);
        // One partner failed: the other one with the alpha = 0 offset.
        let mut q = p.clone();
        q[3].coeffs = None;
        let (v, _) = apply_alpha_mode(&q, AlphaMode::Mirror);
        assert!((v.iter().find(|x| x.alpha_deg == 4.0).unwrap().coeffs.unwrap().cn - 1.0).abs() < 1e-12);
        let mut q = p.clone();
        q[1].coeffs = None;
        let (v, _) = apply_alpha_mode(&q, AlphaMode::Mirror);
        let c4 = v.iter().find(|x| x.alpha_deg == 4.0).unwrap().coeffs.unwrap();
        assert!((c4.cn - 1.0).abs() < 1e-12 && (c4.mom - 1.5).abs() < 1e-12 && (c4.ca - 0.20).abs() < 1e-12);
        // Offset and single modes keep the angles they are given.
        let pos: Vec<SolvedPoint> = p.iter().filter(|x| x.alpha_deg >= 0.0).copied().collect();
        let (o, _) = apply_alpha_mode(&pos, AlphaMode::Offset);
        assert!((o[1].coeffs.unwrap().cn - 1.0).abs() < 1e-12 && o[0].coeffs.unwrap().cn == 0.0);
        let (s, a) = apply_alpha_mode(&pos, AlphaMode::Single);
        assert_eq!(s[1].coeffs.unwrap().cn, 1.3);
        assert_eq!(a.len(), 1);
    }

    #[test]
    fn zero_angle_slope_is_the_slope_of_the_normal_force_interpolant() {
        // Normal force (0, 0), (2, 2), (4, 3) [deg]: the slope at zero must be the one of the curve
        // the table rows are taken from, also for the nose moment (damping sums).
        let m = model();
        let pts: Vec<(f64, BodyCoeffs)> = [(0.0, 0.0), (2.0, 2.0), (4.0, 3.0)].iter().map(|&(a, c)| (a, BodyCoeffs { cn: c, ca: 0.3, mom: 1.1 * c })).collect();
        let h = 1e-4;
        let c = column(1.5, &pts, &[0.0, h, 2.0, 4.0], &m).unwrap();
        let interp = (c.cn[1] / h).to_degrees();
        assert!((c.cna - interp).abs() < 1e-3 * c.cna, "cna {} vs interpolant slope {interp}", c.cna);
        assert!((c.xcp0 - 1.1).abs() < 1e-9, "xcp0 {}", c.xcp0);
        assert!(((c.mom[1] / h).to_degrees() / interp - 1.1).abs() < 1e-6);
        // The nodes stay on the data.
        assert!((c.cn[2] - 2.0).abs() < 1e-12 && (c.cn[3] - 3.0).abs() < 1e-12);
    }

    #[test]
    fn zero_angle_centre_of_pressure_follows_the_interpolated_moment_across_mach() {
        // CN = k a with k = 2 (Mach 1) and 6 (Mach 2), x_cp = 1 and 2 m: at Mach 1.5 the interpolated
        // slope is 4 and the moment slope (2 + 12) / 2 = 7, so x_cp(0) = 1.75 as at the nonzero angles.
        let m = model();
        let pts: Vec<SolvedPoint> = [(1.0, 2.0, 1.0), (2.0, 6.0, 2.0)]
            .iter()
            .flat_map(|&(mach, k, x)| {
                ALPHAS.iter().map(move |&a| {
                    let cn = k * f64::to_radians(a);
                    SolvedPoint { mach, alpha_deg: a, coeffs: Some(BodyCoeffs { cn, ca: 0.3, mom: cn * x }) }
                })
            })
            .collect();
        let (t, _) = build_from_points(&pts, &m, FlowModel::Rans, "h".into(), Extrapolation::Linear).unwrap();
        let (c0, c2) = (t.lookup(1.5, 0.0), t.lookup(1.5, 2f64.to_radians()));
        assert!((c0.cna - 4.0).abs() < 1e-9, "cna {}", c0.cna);
        assert!((c0.xcp - 1.75).abs() < 1e-9 && (c2.xcp - 1.75).abs() < 1e-9, "x_cp(0) {} x_cp(2 deg) {}", c0.xcp, c2.xcp);
        assert!((c0.damp[1] / c0.damp[0] - 1.75).abs() < 1e-9);
    }

    #[test]
    fn matches_analytic_field() {
        let m = model();
        let (t, fill) = build_from_points(&points(&MACHS, &ALPHAS), &m, FlowModel::Rans, "h".into(), Extrapolation::Linear).unwrap();
        assert!(fill.filled.is_empty());
        for &mach in &[0.75, 1.0, 1.75, 2.5] {
            for &a in &[2.0f64, 5.0, 10.0, 16.0] {
                let c = t.lookup(mach, a.to_radians());
                assert!((c.cn - cn(mach, a)).abs() < 4e-3, "cn at M={mach} a={a}: {} vs {}", c.cn, cn(mach, a));
                assert!((c.xcp - xcp(mach)).abs() < 3e-3, "xcp {}", c.xcp);
                let k = aero::model::axial_alpha_factor(a.to_radians());
                let extra = m.cd0(mach, false).extra * k;
                assert!((c.ca_off - (ca(mach, a) + extra)).abs() < 3e-3, "ca {} vs {}", c.ca_off, ca(mach, a));
            }
        }
        let c0 = t.lookup(1.5, 0.0);
        assert!(c0.cn.abs() < 1e-12);
        assert!((c0.cna - 2.0 * (2.0 + 0.45)).abs() < 2e-3, "cna {}", c0.cna);
        assert!((c0.xcp - xcp(1.5)).abs() < 1e-6);
        assert!((c0.damp[0] - c0.cna).abs() < 1e-12 && (c0.damp[1] / c0.damp[0] - c0.xcp).abs() < 1e-9);
    }

    #[test]
    fn blends_into_barrowman_beyond_the_solved_angles_and_holds_mach() {
        let g = extract(&SampleRocket::default().mesh(), &ExtractOptions::default()).unwrap();
        let o = AeroOptions { mach_min: 0.0, mach_max: 4.0, mach_step: 0.5, alpha_max_deg: 90.0, alpha_step_deg: 2.0, ..Default::default() };
        let m = AeroModel::new(g, o);
        let (lin, _) = build_from_points(&points(&MACHS, &ALPHAS), &m, FlowModel::Rans, "h".into(), Extrapolation::Linear).unwrap();
        let (cl, _) = build_from_points(&points(&MACHS, &ALPHAS), &m, FlowModel::Rans, "h".into(), Extrapolation::Clamp).unwrap();
        let (a16, a24) = (16f64.to_radians(), 24f64.to_radians());
        for t in [&lin, &cl] {
            // At the highest solved angle the CFD values are kept.
            let c = t.lookup(2.0, a16);
            assert!((c.cn - cn(2.0, 16.0)).abs() < 1e-9 && (c.xcp - xcp(2.0)).abs() < 1e-9, "{c:?}");
            // In the relaxation band: the blend of the CFD edge values and the Barrowman model.
            let (bcn, bx, _) = beyond_solved(&m, 2.0, 24.0, 16.0, cn(2.0, 16.0), xcp(2.0), ca(2.0, 16.0));
            let c = t.lookup(2.0, a24);
            assert!((c.cn - bcn).abs() < 1e-9 && (c.xcp - bx).abs() < 1e-9);
            // From twice the highest solved angle on: the Barrowman normal force and centre of pressure.
            for a in [32.0f64, 60.0, 90.0] {
                let (c, (bcn, bx)) = (t.lookup(2.0, a.to_radians()), m.normal(2.0, a.to_radians()));
                assert!((c.cn - bcn).abs() < 1e-9 && (c.xcp - bx).abs() < 1e-9, "a = {a}: {} {} vs {bcn} {bx}", c.cn, c.xcp);
            }
            // CA follows the Barrowman angle factor from the edge value (0 at 90 deg plus the hybrid terms, also x 0).
            assert!(t.lookup(2.0, 90f64.to_radians()).ca_off.abs() < 1e-9);
        }
        // Beyond the solved Mach range the edge values are held for both modes (no end-slope continuation).
        for t in [&lin, &cl] {
            for &mach in &[3.5, 4.0] {
                let (e, h) = (t.lookup(mach, a16), t.lookup(3.0, a16));
                assert!((e.cn - h.cn).abs() < 1e-12 && (e.cna - h.cna).abs() < 1e-12 && (e.xcp - h.xcp).abs() < 1e-12);
            }
        }
    }

    #[test]
    fn a_forward_moving_end_slope_does_not_move_the_cp_ahead_of_the_body() {
        // Synthetic CFD column whose centre of pressure moves forward fast near the last solved angle: a
        // linear continuation of CN and the nose moment would put xcp at 0.62 m at 90 deg, far ahead of the
        // Barrowman value (0.91 m).
        let g = extract(&SampleRocket::default().mesh(), &ExtractOptions::default()).unwrap();
        let m = AeroModel::new(g, AeroOptions { mach_max: 1.0, mach_step: 0.5, alpha_max_deg: 90.0, alpha_step_deg: 1.0, ..Default::default() });
        let xc = |a: f64| 1.2 - 0.02 * a;
        let mut p = Vec::new();
        for mach in [0.3, 0.8] {
            for a in [0.0, 4.0, 8.0, 12.0, 16.0] {
                let c = 0.3 * a;
                p.push(SolvedPoint { mach, alpha_deg: a, coeffs: Some(BodyCoeffs { cn: c, ca: 0.1, mom: c * xc(a) }) });
            }
        }
        let (t, _) = build_from_points(&p, &m, FlowModel::Euler, "h".into(), Extrapolation::Linear).unwrap();
        let mut prev = f64::NAN;
        for a in (16..=90).map(f64::from) {
            let c = t.lookup(0.3, a.to_radians());
            let (_, bx) = m.normal(0.3, a.to_radians());
            // Close to or aft of the lower of the edge value and the Barrowman value, and the Barrowman value from 32 deg on.
            assert!(c.xcp >= bx.min(xc(16.0)) - 0.03, "a = {a}: xcp {}", c.xcp);
            if a >= 32.0 {
                assert!((c.xcp - bx).abs() < 0.01, "a = {a}: {} vs {bx}", c.xcp);
            }
            assert!(c.cn > 0.0 && c.cn.is_finite());
            if prev.is_finite() {
                assert!((c.xcp - prev).abs() < 0.03, "smooth hand-over at {a}: {prev} -> {}", c.xcp);
            }
            prev = c.xcp;
        }
        // The value at the edge is the CFD value.
        assert!((t.lookup(0.3, 16f64.to_radians()).xcp - xc(16.0)).abs() < 1e-9);
    }

    #[test]
    fn failed_cases_are_bridged() {
        let m = model();
        let mut p = points(&MACHS, &ALPHAS);
        for q in &mut p {
            if (q.mach == 1.5 && q.alpha_deg == 8.0) || (q.mach == 2.0) {
                q.coeffs = None;
            }
        }
        let (t, fill) = build_from_points(&p, &m, FlowModel::Euler, "h".into(), Extrapolation::Linear).unwrap();
        assert_eq!(fill.filled.len(), 1 + ALPHAS.len());
        assert_eq!(fill.slopes.len(), MACHS.len() - 1);
        let c = t.lookup(2.0, 8f64.to_radians());
        assert!((c.cn - cn(2.0, 8.0)).abs() < 8e-3, "{} vs {}", c.cn, cn(2.0, 8.0));
        let c = t.lookup(1.5, 8f64.to_radians());
        assert!((c.cn - cn(1.5, 8.0)).abs() < 8e-3);
    }

    /// The stored Euler cases of the sample rocket (doc/data/cfd, solved at 0, 2, 4, 8 deg).
    fn sample_euler_points() -> Vec<SolvedPoint> {
        let text = include_str!("../../../doc/data/cfd/sample_euler_cases.csv");
        let mut v = Vec::new();
        for line in text.lines().filter(|l| !l.starts_with('#')).skip(1) {
            let f: Vec<&str> = line.split(',').collect();
            if f[2] != "converged" && f[2] != "accepted" {
                continue;
            }
            let num = |i: usize| f[i].parse::<f64>().unwrap();
            v.push(SolvedPoint { mach: num(0), alpha_deg: num(1), coeffs: Some(BodyCoeffs { cn: num(6), ca: num(7), mom: num(8) }) });
        }
        remove_zero_offset(&mut v);
        v
    }

    #[test]
    fn sample_euler_table_at_large_angles() {
        let g = extract(&SampleRocket::default().mesh(), &ExtractOptions::default()).unwrap();
        let m = AeroModel::new(g, AeroOptions { mach_max: 2.0, mach_step: 0.1, alpha_max_deg: 90.0, alpha_step_deg: 1.0, ..Default::default() });
        let (t, _) = build_from_points(&sample_euler_points(), &m, FlowModel::Euler, "h".into(), Extrapolation::Linear).unwrap();
        // Solved up to 8 deg: CFD values there, Barrowman from 16 deg on (printed with --nocapture).
        for mach in [0.3, 0.8, 1.2, 2.0] {
            for a in [8.0f64, 12.0, 16.0, 30.0, 60.0, 90.0] {
                let c = t.lookup(mach, a.to_radians());
                let (bcn, bx) = m.normal(mach, a.to_radians());
                eprintln!("M {mach} a {a:>4}: CFD table CN {:.3} xcp {:.3} m | Barrowman CN {bcn:.3} xcp {bx:.3} m", c.cn, c.xcp);
                if a >= 16.0 {
                    assert!((c.cn - bcn).abs() < 1e-9 && (c.xcp - bx).abs() < 1e-9);
                }
                if a == 8.0 {
                    let solved = sample_euler_points().into_iter().find(|p| p.mach == mach && p.alpha_deg == 8.0).unwrap().coeffs.unwrap();
                    assert!((c.cn - solved.cn).abs() < 1e-9, "{} vs {}", c.cn, solved.cn);
                }
            }
        }
    }

    #[test]
    fn missing_edge_column_is_an_error() {
        let m = model();
        let mut p = points(&MACHS, &ALPHAS);
        for q in &mut p {
            if q.mach == 3.0 {
                q.coeffs = None;
            }
        }
        let e = build_from_points(&p, &m, FlowModel::Euler, "h".into(), Extrapolation::Linear).unwrap_err().to_string();
        assert!(e.contains("highest") && e.contains('3'), "{e}");
        assert!(build_from_points(&[], &m, FlowModel::Euler, "h".into(), Extrapolation::Linear).is_err());
    }

    /// RANS tables never carry the Euler build-up: the same CFD points give a lower axial force than the Euler path,
    /// and the base source only changes the base term.
    #[test]
    fn rans_table_axial_has_no_euler_terms() {
        let m = model();
        let pts = |ca: f64| -> Vec<SolvedPoint> {
            [0.5, 2.0]
                .iter()
                .flat_map(|&mach| [0.0, 2.0, 4.0].iter().map(move |&a| (mach, a)))
                .map(|(mach, a)| SolvedPoint { mach, alpha_deg: a, coeffs: Some(BodyCoeffs { cn: 0.3 * a, ca, mom: 0.5 * 0.3 * a }) })
                .collect()
        };
        let build = |flow: FlowModel, base: bool| build_from_points_roll(&pts(0.3), &m, flow, "h".into(), Extrapolation::Linear, &[], base).unwrap().0;
        let (eu, rm, rc) = (build(FlowModel::Euler, false), build(FlowModel::Rans, false), build(FlowModel::Rans, true));
        let b = m.cd0(0.5, false);
        let at = |t: &AeroTable| t.lookup(0.5, 0.0).ca_off;
        assert!((at(&rm) - (0.3 + b.base + b.extra)).abs() < 1e-9);
        assert!((at(&rc) - (0.3 + b.extra)).abs() < 1e-9);
        assert!(at(&eu) > at(&rm) + b.friction * 0.99);
    }
}
