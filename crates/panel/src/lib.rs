// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025-2026 西濱大将 (NISHIHAMA Daisuke)
//! Panel-method aerodynamics for IgnisYeet.
//!
//! Builds a watertight panel mesh of the extracted rocket geometry and produces an
//! [`aero::AeroTable`] with the same layout as the Barrowman path:
//! * subsonic Mach numbers: Morino/Dirichlet constant-source/doublet panel solution with the
//!   Goethert compressibility transformation (`solver`), with an empirical viscous cross-flow term;
//!   above the attached-flow angle (15°) the normal force and its moment relax to the component
//!   build-up model, whose fins carry the flat-plate crossflow force (see [`relax_to_large_angle`]);
//! * supersonic Mach numbers: local-inclination methods (`supersonic`);
//! * transonic band: cubic Hermite blend in Mach number between the two regimes, of the
//!   normal force and its nose moment (the centre of pressure is their ratio; monotone slopes) and
//!   of the axial force;
//! * axial force: surface-integrated skin friction (`viscous`), supersonic wave drag, plus the
//!   base / fin-edge / boattail / extra terms of `aero::AeroModel`.
//!
//! Sign and moment conventions: x is measured aft from the nose tip, `CN` is the force along the
//! transverse flow component (positive for positive alpha), moments are taken about the nose tip
//! and the centre of pressure is `xcp = M_nose / CN`.

pub mod influence;
pub mod memory;
pub mod mesh;
pub mod solver;
pub mod supersonic;
#[cfg(test)]
#[path = "../../../testutil/tmp.rs"]
mod test_tmp;
pub mod viscous;

use aero::model::{axial_alpha_factor, FIN_ALPHA_LINEAR};
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
    /// Mach range `[lo, hi]` of the transonic blend: `lo` is the last purely subsonic Mach number (the
    /// highest of `subsonic_machs` when that is lower), `hi > 1` the first fully supersonic one.
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

impl PanelOptions {
    /// Checks every option; errors name the configuration key (`aero.panel.*`).
    pub fn validate(&self) -> Result<()> {
        if self.body_axial < 6 {
            bail!("[IY-A010] aero.panel.body_axial must be at least 6, found {}", self.body_axial);
        }
        if self.body_circ < 8 {
            bail!("[IY-A011] aero.panel.body_circ must be at least 8, found {}", self.body_circ);
        }
        if self.fin_chord < 2 || self.fin_chord > self.body_axial - 4 {
            bail!(
                "[IY-A012] aero.panel.fin_chord must be between 2 and aero.panel.body_axial - 4 = {} (the chord panels are part of the body stations), found {}",
                self.body_axial - 4,
                self.fin_chord
            );
        }
        if self.fin_span < 2 {
            bail!("[IY-A013] aero.panel.fin_span must be at least 2, found {}", self.fin_span);
        }
        if !(self.wake_length.is_finite() && self.wake_length > 0.0) {
            bail!("[IY-A014] aero.panel.wake_length must be a finite number > 0 (body lengths), found {}", self.wake_length);
        }
        if !(self.tail_radii.is_finite() && self.tail_radii >= 0.0) {
            bail!("[IY-A015] aero.panel.tail_radii must be a finite number >= 0 (base radii; 0 = open base), found {}", self.tail_radii);
        }
        if self.subsonic_machs.is_empty() || self.subsonic_machs.iter().any(|m| !(m.is_finite() && (0.0..1.0).contains(m))) {
            bail!("[IY-A016] aero.panel.subsonic_machs must be a non-empty list of finite Mach numbers in [0, 1), found {:?}", self.subsonic_machs);
        }
        let [lo, hi] = self.transonic;
        if !(lo.is_finite() && hi.is_finite() && lo > 0.0 && lo < hi && hi > 1.0) {
            bail!(
                "[IY-A017] aero.panel.transonic = [{lo}, {hi}] must be finite and increasing, [lo, hi] with 0 < lo and hi > 1 (hi is the first fully supersonic Mach number)"
            );
        }
        Ok(())
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

/// The quantities the transonic band blends in Mach number, all linear in the flow solution: per
/// angle the normal force, its nose moment `cn * xcp` and the axial forces; per column the
/// zero-angle normal-force slope, its nose moment `cna * x_cp(0)` and the damping sums. `x0` is the
/// column's zero-angle centre of pressure.
fn to_linear(c: &AeroCoeffs, x0: f64) -> [f64; 9] {
    [c.cn, c.cn * c.xcp, c.ca_on, c.ca_off, c.cna, c.cna * x0, c.damp[0], c.damp[1], c.damp[2]]
}

/// Indices of the axial forces in [`to_linear`].
const AXIAL: [usize; 2] = [2, 3];

/// End slopes of the transonic Hermite blend from `f0` to `f1` over a band `width` wide, limited
/// (Fritsch-Carlson) so the blend stays between the edge values: a slope against the change across
/// the band is dropped and the pair is scaled down to at most three times the mean slope. The raw
/// slopes are finite differences of each regime at its edge, which near the low supersonic end vary
/// steeply with Mach number and would otherwise make the band overshoot.
fn monotone_slopes(f0: f64, f1: f64, width: f64, d0: f64, d1: f64) -> (f64, f64) {
    let s = (f1 - f0) / width;
    if s.abs() <= 1e-12 * (f0.abs() + f1.abs()).max(1e-300) {
        return (0.0, 0.0);
    }
    let (a, b) = ((d0 / s).max(0.0), (d1 / s).max(0.0));
    let r = a.hypot(b);
    let k = if r > 3.0 { 3.0 / r } else { 1.0 };
    (k * a * s, k * b * s)
}

/// Coefficients from blended [`to_linear`] values: the centre of pressure is the blended moment over
/// the blended normal force (at zero angle, or with no normal force, the zero-angle one from the
/// blended slope and its moment), so force, moment and slope stay consistent.
fn from_linear(a: [f64; 9]) -> AeroCoeffs {
    let x0 = if a[4].abs() > 1e-12 { a[5] / a[4] } else { 0.0 };
    let xcp = if a[0].abs() > 1e-12 { a[1] / a[0] } else { x0 };
    AeroCoeffs { cn: a[0], ca_on: a[2], ca_off: a[3], xcp, cna: a[4], damp: [a[6], a[7], a[8]], ..Default::default() }
}

/// Angle [rad] from which the subsonic columns equal the component build-up model: twice the
/// attached-flow angle [`FIN_ALPHA_LINEAR`] (30°).
pub const LARGE_ANGLE_RELAXED: f64 = 2.0 * FIN_ALPHA_LINEAR;

/// Large-angle value of a subsonic panel quantity (normal force or its nose moment) at `a` [rad] above
/// [`FIN_ALPHA_LINEAR`].
///
/// The subsonic panel solution is linear potential flow, `CN_alpha sin a cos a`, plus the body crossflow
/// term. It holds at small angles only: at 90° its potential part vanishes, so the fins carry no normal
/// force and the centre of pressure falls to the body planform centroid. Beyond the attached-flow angle
/// `a1 = FIN_ALPHA_LINEAR` the quantity therefore relaxes to the component build-up value `B(a)`
/// ([`AeroModel::normal`]: body, body crossflow and the fin flat-plate crossflow at the fin planform
/// centroid, see [`AeroModel::fin_cn`] and [`AeroModel::fin_xcp_at`]):
///
/// `P(a) = B(a) + (D + D' (a - a1)) w(a)`,  `D = P_panel(a1) - B(a1)`,  `D' = P_panel'(a1) - B'(a1)`,
///
/// with `w` a C1 smoothstep from 1 at `a1` to 0 at [`LARGE_ANGLE_RELAXED`]. Value and slope match the
/// panel result at `a1`; from `LARGE_ANGLE_RELAXED` on the value is `B(a)`.
fn relax_to_large_angle(a: f64, panel: impl Fn(f64) -> f64, barrowman: impl Fn(f64) -> f64) -> f64 {
    let (a1, ar) = (FIN_ALPHA_LINEAR, LARGE_ANGLE_RELAXED);
    let b = barrowman(a);
    if a >= ar {
        return b;
    }
    if a <= a1 {
        return panel(a);
    }
    let h = 1e-6;
    let p1 = panel(a1);
    let b1 = barrowman(a1);
    let dp = (p1 - panel(a1 - h)) / h;
    let db = (barrowman(a1 + h) - barrowman(a1 - h)) / (2.0 * h);
    let w = hermite(a, a1, ar, 1.0, 0.0, 0.0, 0.0);
    b + ((p1 - b1) + (dp - db) * (a - a1)) * w
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
                AeroCoeffs { cn, ca_on: ca_on * k, ca_off: ca_off * k, xcp, cna, damp, ..Default::default() }
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
        let potential = |a: f64| {
            let (cl, ml) = self.cross_flow(a);
            (d[0] * a.sin() * a.cos() + cl, d[1] * a.sin() * a.cos() + ml)
        };
        self.column_from(
            |a| {
                if a <= FIN_ALPHA_LINEAR {
                    return potential(a);
                }
                let barrowman = |a: f64| {
                    let (cn, x) = self.model.normal(m, a);
                    (cn, cn * x)
                };
                let cn = relax_to_large_angle(a, |a| potential(a).0, |a| barrowman(a).0);
                let mom = relax_to_large_angle(a, |a| potential(a).1, |a| barrowman(a).1);
                // Keep the centre of pressure on the body during the transition.
                let x = if cn.abs() > 1e-12 { (mom / cn).clamp(0.0, self.geom.length.max(0.0)) } else { 0.0 };
                (cn, cn * x)
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
    Mesh {
        subsonic_solves: usize,
    },
    MeshReady {
        panels: usize,
        wake_panels: usize,
    },
    /// Assembly, LU factorisation and solves of subsonic Mach number `mach` (`index` of `total`, 0-based) begin.
    Subsonic {
        index: usize,
        total: usize,
        mach: f64,
    },
    SubsonicDone {
        index: usize,
        total: usize,
        mach: f64,
    },
    /// Supersonic local-inclination tables are being set up.
    Supersonic,
    SupersonicDone,
    /// Transonic blend edges are being evaluated.
    Transonic,
    /// Table columns (Mach rows) filled; called from worker threads.
    Columns {
        done: usize,
        total: usize,
    },
    Done,
}

static MEMORY_LIMIT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// Sets the parallelism of the dense LU solver (faer): 1 runs sequentially, `n > 1` uses `n` rayon
/// threads and 0 uses the whole current rayon pool. Matrix assembly follows the caller's rayon pool.
pub fn set_parallelism(threads: usize) {
    use faer::Par;
    faer::set_global_parallelism(match threads {
        0 => Par::rayon(0),
        1 => Par::Seq,
        n => Par::rayon(n),
    });
}

/// Sets the memory budget [bytes] checked before the dense solve; `None` = no limit.
pub fn set_memory_limit(bytes: Option<u64>) {
    MEMORY_LIMIT.store(bytes.unwrap_or(0), std::sync::atomic::Ordering::Relaxed);
}

/// Peak memory [bytes] of the dense solve for `n` panels: one `n x n` f64 matrix (assembled, then
/// LU-factored in place, see [`solver::solve_potential`]), plus the right-hand sides. Computed in
/// saturating 128-bit arithmetic and saturated at `u64::MAX` (from about 1.5e9 panels on), so any size gives
/// a value to compare with the memory limits.
pub fn dense_solve_bytes(n: usize) -> u64 {
    let n = n as u128;
    u64::try_from(n.saturating_mul(n).saturating_mul(8).saturating_add(64 * n)).unwrap_or(u64::MAX)
}

/// Fails when the dense solve of `n` panels exceeds the budget `limit` (`resources.memory_gb`) or the
/// memory available to this process ([`memory::available`]: available memory, cgroup limit,
/// address-space limit; physical memory on macOS; no check where none is known).
pub fn check_memory(n: usize, limit: Option<u64>) -> Result<()> {
    check_memory_with(n, limit, memory::available())
}

/// [`check_memory`] with the available memory given.
pub fn check_memory_with(n: usize, limit: Option<u64>, available: Option<memory::Available>) -> Result<()> {
    if let Some(a) = available {
        let need = dense_solve_bytes(n);
        if need > a.bytes {
            bail!(
                "[IY-A018] the panel method needs about {:.2} GB for {n} panels (dense matrix), more than the {:.2} GB of memory available to this process ({}); \
                 use coarser panel settings (body_axial, body_circ, fin_chord, fin_span)",
                need as f64 / 1e9,
                a.bytes as f64 / 1e9,
                a.source
            );
        }
    }
    if let Some(limit) = limit {
        let need = dense_solve_bytes(n);
        if need > limit {
            bail!(
                "[IY-A019] the panel method needs about {:.2} GB for {n} panels (dense matrix), above resources.memory_gb = {:.3}; \
                 use coarser panel settings (body_axial, body_circ, fin_chord, fin_span) or raise the budget",
                need as f64 / 1e9,
                limit as f64 / 1e9
            );
        }
    }
    Ok(())
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
    panel.validate()?;
    let mut machs_sub: Vec<f64> = panel.subsonic_machs.clone();
    machs_sub.sort_by(f64::total_cmp);
    machs_sub.dedup_by(|a, b| (*a - *b).abs() < 1e-9);
    progress(PanelStage::Mesh { subsonic_solves: machs_sub.len() });
    let mesh = Mesh::from_geometry(geom, panel)?;
    progress(PanelStage::MeshReady { panels: mesh.panels.len(), wake_panels: mesh.wake_panels.len() });
    check_memory(
        mesh.panels.len(),
        match MEMORY_LIMIT.load(std::sync::atomic::Ordering::Relaxed) {
            0 => None,
            b => Some(b),
        },
    )?;
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
        let pot = if (beta - 1.0).abs() < 1e-12 { solver::solve_potential(&mesh)? } else { solver::solve_potential(&mesh.scaled_lateral(beta))? };
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
        sup: supersonic::SupersonicPanels::new(
            &mesh,
            s_ref,
            geom.fins.as_ref().map(|f| (f.x_le_root + f.sweep, f.body_radius + f.span)),
            geom.fins.as_ref().map(|f| (f.body_radius, f.x_le_root + f.root_chord)),
        ),
    };

    progress(PanelStage::SupersonicDone);
    progress(PanelStage::Transonic);
    // Implementation notes for the band (the user documentation only states the behaviour):
    // * the end-point Mach slopes are one-sided differences with step `h = 0.02` taken outside the band;
    // * for every quantity but the axial force the slopes are limited by `monotone_slopes`
    //   (Fritsch-Carlson: negative ratios to 0, ratio vector scaled to length 3), because the
    //   low-supersonic local-inclination values change quickly with Mach number and an unlimited
    //   Hermite blend would overshoot; the axial force keeps the raw slopes to preserve the drag rise;
    // * a band without Mach grid points inside it is not evaluated.
    // Transonic blend: values and Mach slopes of both regimes at the band edges.
    // The band starts at `transonic[0]`, or at the highest solved subsonic Mach number when that is
    // lower, so the subsonic columns are interpolated between solutions, never extrapolated.
    let m_lo = panel.transonic[0].min(*machs_sub.last().unwrap());
    let m_hi = panel.transonic[1];
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
                    // Blend the force and its nose moment (not the centre of pressure, a ratio) and
                    // the zero-angle slope with its moment, then derive the centres of pressure.
                    let lin = |c: &[AeroCoeffs]| to_linear(&c[j], c[0].xcp);
                    let (a0, am, a1, ap) = (lin(f0), lin(f0m), lin(f1), lin(f1p));
                    let mut out = [0.0; 9];
                    for q in 0..9 {
                        let (mut d0, mut d1) = ((a0[q] - am[q]) / h, (ap[q] - a1[q]) / h);
                        // The axial force keeps its slopes (they shape the transonic drag rise);
                        // the normal force, moment and slopes stay between the edge values.
                        if !AXIAL.contains(&q) {
                            (d0, d1) = monotone_slopes(a0[q], a1[q], m_hi - m_lo, d0, d1);
                        }
                        out[q] = hermite(m, m_lo, m_hi, a0[q], a1[q], d0, d1);
                    }
                    from_linear(out)
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
        fin_cant_ref_deg: geom.fins.as_ref().map_or(0.0, |f| f.cant_deg),
        extrapolation,
        ..Default::default()
    };
    // Roll terms (cant forcing, roll damping) come from the component build-up model.
    let roll = AeroModel::new(geom.clone(), aero.clone());
    let table = AeroTable::from_fn(meta, |m, a| {
        let i = machs.iter().position(|&v| v == m).unwrap();
        let j = alphas_deg.iter().position(|&v| v == a).unwrap();
        let (cl_delta, clp) = roll.roll_terms(m);
        AeroCoeffs { cl_delta, clp, ..cols[i][j] }
    });
    let report = PanelReport { panels: mesh.panels.len(), wake_panels: mesh.wake_panels.len(), solve_seconds, subsonic: summaries, surface_cp };
    progress(PanelStage::Done);
    Ok((table, report))
}

#[cfg(test)]
mod tests;
