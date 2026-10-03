//! Panel-method aerodynamics for IgnisYeet.
//!
//! Builds a watertight panel mesh of the extracted rocket geometry and produces an
//! [`aero::AeroTable`] with the same layout as the Barrowman path:
//! * subsonic Mach numbers: Morino/Dirichlet constant-source/doublet panel solution with the
//!   Goethert compressibility transformation (`solver`), with an empirical viscous cross-flow term;
//! * supersonic Mach numbers: local-inclination methods (`supersonic`);
//! * transonic band: cubic Hermite blend in Mach number between the two regimes;
//! * axial force: surface-integrated skin friction (`viscous`), supersonic wave drag, plus the
//!   base / fin-edge / boattail / extra terms of `aero::AeroModel`.
//!
//! Sign and moment conventions: x is measured aft from the nose tip, `CN` is the force along the
//! transverse flow component (positive for positive alpha), moments are taken about the nose tip
//! and the centre of pressure is `xcp = M_nose / CN`.

pub mod influence;
pub mod mesh;
pub mod solver;
pub mod supersonic;
pub mod viscous;

use aero::model::axial_alpha_factor;
use aero::table::{AeroCoeffs, TableMeta};
use aero::{AeroModel, AeroOptions, AeroTable, Extrapolation};
use anyhow::{bail, Result};
use geom::math::hermite;
use geom::Geometry;
use mesh::{Mesh, Part};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use solver::PotentialCoeffs;
use std::path::Path;
use std::time::Instant;

/// Fin cross-section.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum FinSection {
    /// Parabolic-arc section, maximum thickness at mid-chord, sharp leading and trailing edge.
    #[default]
    Biconvex,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct PanelOptions {
    /// Body panels along the axis (including the fin chord panels).
    pub body_axial: usize,
    /// Body nodes per revolution.
    pub body_circ: usize,
    /// Chordwise panels on each fin side.
    pub fin_chord: usize,
    /// Spanwise panels on each fin side.
    pub fin_span: usize,
    /// Fin wake length in body lengths.
    pub wake_length: f64,
    /// Length of the solid tail fairing behind the base, in base radii (0 = open base).
    pub tail_radii: f64,
    /// Mach numbers (< 1) of the subsonic panel solutions.
    pub subsonic_machs: Vec<f64>,
    /// Mach range `[lo, hi]` of the transonic blend; `hi` is the first fully supersonic Mach.
    pub transonic: [f64; 2],
    pub fin_section: FinSection,
}

impl Default for PanelOptions {
    fn default() -> Self {
        Self {
            body_axial: 80,
            body_circ: 48,
            fin_chord: 16,
            fin_span: 10,
            wake_length: 20.0,
            tail_radii: 6.0,
            subsonic_machs: vec![0.0, 0.3, 0.5, 0.6, 0.7, 0.8],
            transonic: [0.8, 1.2],
            fin_section: FinSection::Biconvex,
        }
    }
}

/// Subsonic panel results at one solved Mach number.
#[derive(Debug, Clone, Copy, Serialize)]
pub struct SubsonicSummary {
    pub mach: f64,
    /// Potential-flow CN slope [1/rad].
    pub cna: f64,
    /// Centre of pressure aft of the nose at alpha -> 0 [m].
    pub xcp: f64,
    /// Damping sums S0, S1, S2.
    pub damp: [f64; 3],
}

/// One surface panel with its pressure coefficient (for plotting).
#[derive(Debug, Clone, Copy)]
pub struct SurfaceCp {
    pub pos: [f64; 3],
    pub normal: [f64; 3],
    pub area: f64,
    pub cp: f64,
    pub part: Part,
}

#[derive(Debug, Clone)]
pub struct PanelReport {
    pub panels: usize,
    pub wake_panels: usize,
    /// Wall time of assembly and factorisation plus solves of all subsonic Mach numbers [s].
    pub solve_seconds: f64,
    pub subsonic: Vec<SubsonicSummary>,
    /// Surface Cp at alpha = 4 deg for the lowest subsonic Mach number (M = 0 by default).
    pub surface_cp: Vec<SurfaceCp>,
}

/// Writes `x,y,z,nx,ny,nz,area,cp,part` CSV.
pub fn write_surface_cp(path: &Path, report: &PanelReport) -> Result<()> {
    use std::io::Write;
    let mut f = std::io::BufWriter::new(std::fs::File::create(path)?);
    writeln!(f, "x,y,z,nx,ny,nz,area,cp,part")?;
    for s in &report.surface_cp {
        let part = if s.part == Part::Fin { "fin" } else { "body" };
        writeln!(
            f,
            "{:.6e},{:.6e},{:.6e},{:.6e},{:.6e},{:.6e},{:.6e},{:.6e},{part}",
            s.pos[0], s.pos[1], s.pos[2], s.normal[0], s.normal[1], s.normal[2], s.area, s.cp
        )?;
    }
    Ok(())
}

fn grid(lo: f64, hi: f64, step: f64) -> Vec<f64> {
    let n = ((hi - lo) / step).round().max(1.0) as usize;
    (0..=n).map(|i| lo + (hi - lo) * i as f64 / n as f64).collect()
}

/// Monotone cubic (Fritsch–Carlson) interpolation; clamps outside the data range.
fn pchip(xs: &[f64], ys: &[f64], x: f64) -> f64 {
    let n = xs.len();
    if n == 1 || x <= xs[0] {
        return ys[0];
    }
    if x >= xs[n - 1] {
        return ys[n - 1];
    }
    let h: Vec<f64> = xs.windows(2).map(|w| w[1] - w[0]).collect();
    let d: Vec<f64> = (0..n - 1).map(|i| (ys[i + 1] - ys[i]) / h[i]).collect();
    let mut m = vec![0.0; n];
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
    let i = xs.partition_point(|&v| v <= x) - 1;
    hermite(x, xs[i], xs[i + 1], ys[i], ys[i + 1], m[i], m[i + 1])
}

struct Ctx<'a> {
    geom: &'a Geometry,
    model: AeroModel,
    aero: &'a AeroOptions,
    alphas: Vec<f64>,
    sub: Vec<(f64, PotentialCoeffs)>,
    fric: viscous::Friction,
    sup: supersonic::SupersonicPanels,
}

fn to_arr(c: &AeroCoeffs) -> [f64; 8] {
    [c.cn, c.ca_on, c.ca_off, c.xcp, c.cna, c.damp[0], c.damp[1], c.damp[2]]
}

fn from_arr(a: [f64; 8]) -> AeroCoeffs {
    AeroCoeffs { cn: a[0], ca_on: a[1], ca_off: a[2], xcp: a[3], cna: a[4], damp: [a[5], a[6], a[7]] }
}

impl Ctx<'_> {
    /// Viscous cross-flow normal force and its nose moment at angle `a`.
    fn cross_flow(&self, a: f64) -> (f64, f64) {
        let cl = 1.1 * self.geom.planform_area / self.geom.ref_area * a.sin().powi(2);
        (cl, cl * self.geom.planform_centroid)
    }

    fn column_from(&self, cn_mom: impl Fn(f64) -> (f64, f64), derivs: [f64; 4], ca_on: f64, ca_off: f64) -> Vec<AeroCoeffs> {
        let [cna, m_alpha, n_rot, m_rot] = derivs;
        let damp = [cna, 0.5 * (m_alpha + n_rot), m_rot];
        let x0 = if cna.abs() > 1e-12 { m_alpha / cna } else { 0.0 };
        self.alphas
            .iter()
            .map(|&a| {
                let (cn, mom) = if a == 0.0 { (0.0, 0.0) } else { cn_mom(a) };
                let xcp = if a == 0.0 || cn.abs() < 1e-12 { x0 } else { mom / cn };
                let k = axial_alpha_factor(a);
                AeroCoeffs { cn, ca_on: ca_on * k, ca_off: ca_off * k, xcp, cna, damp }
            })
            .collect()
    }

    /// Subsonic column: interpolated potential-flow derivatives plus viscous terms.
    fn sub_column(&self, m: f64) -> Vec<AeroCoeffs> {
        let xs: Vec<f64> = self.sub.iter().map(|s| s.0).collect();
        let get = |f: fn(&PotentialCoeffs) -> f64| {
            let ys: Vec<f64> = self.sub.iter().map(|s| f(&s.1)).collect();
            pchip(&xs, &ys, m)
        };
        let d = [get(|c| c.cna), get(|c| c.m_alpha), get(|c| c.n_rot), get(|c| c.m_rot)];
        let (on, off) = (self.model.cd0(m, true), self.model.cd0(m, false));
        let fr = self.fric.ca(m, self.aero);
        let ca = |b: &aero::model::DragBreakdown| fr + b.nose + b.base + b.fins + b.boattail + b.extra;
        self.column_from(
            |a| {
                let (cl, ml) = self.cross_flow(a);
                (d[0] * a.sin() * a.cos() + cl, d[1] * a.sin() * a.cos() + ml)
            },
            d,
            ca(&on),
            ca(&off),
        )
    }

    /// Supersonic column from the local-inclination methods.
    fn sup_column(&self, m: f64) -> Vec<AeroCoeffs> {
        let d = self.sup.derivatives(m, self.geom.length);
        let wave = self.sup.forces(m, 0.0, 0.0).fx;
        let (on, off) = (self.model.cd0(m, true), self.model.cd0(m, false));
        let fr = self.fric.ca(m, self.aero);
        let ca = |b: &aero::model::DragBreakdown| wave + fr + b.base + b.fins + b.boattail + b.extra;
        self.column_from(
            |a| {
                let f = self.sup.forces(m, a, 0.0);
                let (cl, ml) = self.cross_flow(a);
                (f.fz + cl, f.mz + ml)
            },
            d,
            ca(&on),
            ca(&off),
        )
    }
}

/// Progress events of [`build_table_with_progress`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PanelStage {
    /// Meshing starts; `subsonic_solves` potential-flow solutions follow.
    Mesh { subsonic_solves: usize },
    MeshReady { panels: usize, wake_panels: usize },
    /// Assembly, LU factorisation and solves of subsonic Mach number `mach` (`index` of `total`, 0-based) begin.
    Subsonic { index: usize, total: usize, mach: f64 },
    SubsonicDone { index: usize, total: usize, mach: f64 },
    /// Supersonic local-inclination tables are being set up.
    Supersonic,
    SupersonicDone,
    /// Transonic blend edges are being evaluated.
    Transonic,
    /// Table columns (Mach rows) filled; called from worker threads.
    Columns { done: usize, total: usize },
    Done,
}

/// Builds the aerodynamic table with the panel method.
pub fn build_table(
    geom: &Geometry,
    panel: &PanelOptions,
    aero: &AeroOptions,
    source_hash: String,
    extrapolation: Extrapolation,
) -> Result<(AeroTable, PanelReport)> {
    build_table_with_progress(geom, panel, aero, source_hash, extrapolation, &|_| {})
}

/// [`build_table`] reporting its stages through `progress`.
pub fn build_table_with_progress(
    geom: &Geometry,
    panel: &PanelOptions,
    aero: &AeroOptions,
    source_hash: String,
    extrapolation: Extrapolation,
    progress: &(dyn Fn(PanelStage) + Sync),
) -> Result<(AeroTable, PanelReport)> {
    let mut machs_sub: Vec<f64> = panel.subsonic_machs.clone();
    machs_sub.sort_by(|a, b| a.partial_cmp(b).unwrap());
    machs_sub.dedup_by(|a, b| (*a - *b).abs() < 1e-9);
    if machs_sub.is_empty() || machs_sub.iter().any(|&m| !(0.0..1.0).contains(&m)) {
        bail!("subsonic_machs must be a non-empty list of Mach numbers in [0, 1)");
    }
    if panel.transonic[0] >= panel.transonic[1] {
        bail!("transonic range must be increasing");
    }
    progress(PanelStage::Mesh { subsonic_solves: machs_sub.len() });
    let mesh = Mesh::from_geometry(geom, panel)?;
    progress(PanelStage::MeshReady { panels: mesh.panels.len(), wake_panels: mesh.wake_panels.len() });
    let s_ref = geom.ref_area;

    // Subsonic solutions (Goethert-scaled incompressible problems).
    let t0 = Instant::now();
    let mut sub = Vec::new();
    let mut summaries = Vec::new();
    let mut surface_cp = Vec::new();
    let n_sub = machs_sub.len();
    for (k, &m) in machs_sub.iter().enumerate() {
        progress(PanelStage::Subsonic { index: k, total: n_sub, mach: m });
        let beta = (1.0 - m * m).sqrt();
        let pot = if (beta - 1.0).abs() < 1e-12 { solver::solve_potential(&mesh) } else { solver::solve_potential(&mesh.scaled_lateral(beta)) };
        let c = solver::coefficients(&mesh, &pot, beta, s_ref);
        if k == 0 {
            let cp = solver::cp_at_alpha(&pot, beta * 4f64.to_radians());
            surface_cp = mesh
                .panels
                .iter()
                .zip(cp)
                .filter(|(p, _)| p.part != Part::Tail)
                .map(|(p, cp)| SurfaceCp {
                    pos: [p.centroid.x, p.centroid.y, p.centroid.z],
                    normal: [p.normal.x, p.normal.y, p.normal.z],
                    area: p.area,
                    cp: cp / (beta * beta),
                    part: p.part,
                })
                .collect();
        }
        summaries.push(SubsonicSummary { mach: m, cna: c.cna, xcp: c.m_alpha / c.cna, damp: [c.cna, 0.5 * (c.m_alpha + c.n_rot), c.m_rot] });
        sub.push((m, c));
        progress(PanelStage::SubsonicDone { index: k, total: n_sub, mach: m });
    }
    let solve_seconds = t0.elapsed().as_secs_f64();

    let machs = grid(aero.mach_min.max(0.0), aero.mach_max, aero.mach_step);
    let alphas_deg = grid(0.0, aero.alpha_max_deg, aero.alpha_step_deg);
    progress(PanelStage::Supersonic);
    let ctx = Ctx {
        geom,
        model: AeroModel::new(geom.clone(), aero.clone()),
        aero,
        alphas: alphas_deg.iter().map(|a| a.to_radians()).collect(),
        sub,
        fric: viscous::Friction::new(&mesh, s_ref),
        sup: supersonic::SupersonicPanels::new(&mesh, s_ref, geom.fins.as_ref().map(|f| (f.x_le_root + f.sweep, f.body_radius + f.span))),
    };

    progress(PanelStage::SupersonicDone);
    progress(PanelStage::Transonic);
    // Transonic blend: values and Mach slopes of both regimes at the band edges.
    let m_lo = *machs_sub.last().unwrap();
    let m_hi = panel.transonic[1].max(m_lo + 1e-6);
    let needs_band = machs.iter().any(|&m| m > m_lo && m < m_hi);
    let band = needs_band.then(|| {
        let h = 0.02;
        let (f0, f0m) = (ctx.sub_column(m_lo), ctx.sub_column(m_lo - h));
        let (f1, f1p) = (ctx.sup_column(m_hi), ctx.sup_column(m_hi + h));
        (f0, f0m, f1, f1p, h)
    });
    let column = |m: f64| -> Vec<AeroCoeffs> {
        if m <= m_lo {
            ctx.sub_column(m)
        } else if m >= m_hi {
            ctx.sup_column(m)
        } else {
            let (f0, f0m, f1, f1p, h) = band.as_ref().unwrap();
            (0..f0.len())
                .map(|j| {
                    let (a0, am, a1, ap) = (to_arr(&f0[j]), to_arr(&f0m[j]), to_arr(&f1[j]), to_arr(&f1p[j]));
                    let mut out = [0.0; 8];
                    for q in 0..8 {
                        let (d0, d1) = ((a0[q] - am[q]) / h, (ap[q] - a1[q]) / h);
                        out[q] = hermite(m, m_lo, m_hi, a0[q], a1[q], d0, d1);
                    }
                    from_arr(out)
                })
                .collect()
        }
    };
    let done = std::sync::atomic::AtomicUsize::new(0);
    let n_cols = machs.len();
    let cols: Vec<Vec<AeroCoeffs>> = machs
        .par_iter()
        .map(|&m| {
            let c = column(m);
            progress(PanelStage::Columns { done: done.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1, total: n_cols });
            c
        })
        .collect();

    let meta = TableMeta {
        source_hash,
        ref_area: s_ref,
        ref_diameter: 2.0 * geom.ref_radius,
        length: geom.length,
        machs: machs.clone(),
        alphas_deg: alphas_deg.clone(),
        extrapolation,
    };
    let table = AeroTable::from_fn(meta, |m, a| {
        let i = machs.iter().position(|&v| v == m).unwrap();
        let j = alphas_deg.iter().position(|&v| v == a).unwrap();
        cols[i][j]
    });
    let report = PanelReport { panels: mesh.panels.len(), wake_panels: mesh.wake_panels.len(), solve_seconds, subsonic: summaries, surface_cp };
    progress(PanelStage::Done);
    Ok((table, report))
}

#[cfg(test)]
mod tests;
