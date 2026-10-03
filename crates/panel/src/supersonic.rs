// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025-2026 西濱大将 (NISHIHAMA Daisuke)
//! Supersonic local-inclination methods.
//!
//! Each panel sees the freestream at the local inclination `delta` between its plane and the
//! flow (`delta > 0`: compression).
//! * Fin panels: tangent wedge (exact oblique shock, weak branch) for compression, modified
//!   Newtonian beyond the detachment limit, Prandtl–Meyer expansion from the freestream for
//!   expansion (leeward shadow), clipped at the vacuum limit.
//! * Body panels: tangent cone (Taylor–Maccoll, cached table; modified Newtonian beyond
//!   detachment) on the axisymmetric slope `delta0` (Prandtl–Meyer for boattail slopes). The
//!   change of inclination caused by incidence or pitch rate is applied as a linearised
//!   increment with the slender-body slope `4 sin d0 cos d0`. A 2-D expansion on the leeward side
//!   of a cylinder would produce a spurious first-order normal force; the increment reproduces
//!   `dCN/dalpha = 2 dS/dx` and leaves the second-order cross-flow load to the empirical term.

use crate::mesh::{Mesh, Part};
use aero::atmosphere::GAMMA;
use geom::Vec3;
use rayon::prelude::*;
use std::sync::OnceLock;

const G: f64 = GAMMA;

// ------------------------------------------------------------------------------------------
// Oblique shock, Prandtl–Meyer, Newtonian
// ------------------------------------------------------------------------------------------

/// Flow deflection angle for shock angle `beta` at Mach `m` (theta-beta-M relation).
pub fn deflection(m: f64, beta: f64) -> f64 {
    let ms2 = m * m * beta.sin().powi(2);
    (2.0 / beta.tan() * (ms2 - 1.0) / (m * m * (G + (2.0 * beta).cos()) + 2.0)).atan()
}

/// Shock angle at maximum deflection.
fn beta_at_max_deflection(m: f64) -> f64 {
    let m2 = m * m;
    let s = ((G + 1.0) * m2 / 4.0 - 1.0 + ((G + 1.0) * (1.0 + (G - 1.0) / 2.0 * m2 + (G + 1.0) / 16.0 * m2 * m2)).sqrt()) / (G * m2);
    s.sqrt().clamp(0.0, 1.0).asin()
}

/// Weak-branch shock angle for wedge half-angle `delta`; `None` if the shock detaches.
pub fn wedge_beta(m: f64, delta: f64) -> Option<f64> {
    if m <= 1.0 || delta <= 0.0 {
        return None;
    }
    let bm = beta_at_max_deflection(m);
    if delta > deflection(m, bm) {
        return None;
    }
    let (mut lo, mut hi) = ((1.0 / m).asin(), bm);
    for _ in 0..60 {
        let mid = 0.5 * (lo + hi);
        if deflection(m, mid) < delta {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    Some(0.5 * (lo + hi))
}

/// Tangent-wedge pressure coefficient (attached weak shock only).
pub fn wedge_cp(m: f64, delta: f64) -> Option<f64> {
    let b = wedge_beta(m, delta)?;
    Some(4.0 / (G + 1.0) * (b.sin().powi(2) - 1.0 / (m * m)))
}

/// Prandtl–Meyer function [rad].
pub fn prandtl_meyer(m: f64) -> f64 {
    let k = ((G + 1.0) / (G - 1.0)).sqrt();
    let s = (m * m - 1.0).max(0.0).sqrt();
    k * (s / k).atan() - s.atan()
}

fn pm_max() -> f64 {
    0.5 * std::f64::consts::PI * (((G + 1.0) / (G - 1.0)).sqrt() - 1.0)
}

/// Pressure coefficient behind an expansion through `delta_abs` (> 0) from Mach `m`.
pub fn expansion_cp(m: f64, delta_abs: f64) -> f64 {
    let nu = prandtl_meyer(m) + delta_abs;
    let vac = -2.0 / (G * m * m);
    if nu >= pm_max() - 1e-9 {
        return vac;
    }
    // Invert nu(M2) by bisection.
    let (mut lo, mut hi) = (m, 200.0);
    for _ in 0..70 {
        let mid = 0.5 * (lo + hi);
        if prandtl_meyer(mid) < nu {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    let m2 = 0.5 * (lo + hi);
    let t = (1.0 + 0.5 * (G - 1.0) * m * m) / (1.0 + 0.5 * (G - 1.0) * m2 * m2);
    let cp = (t.powf(G / (G - 1.0)) - 1.0) / (0.5 * G * m * m);
    cp.max(vac)
}

/// Maximum (stagnation) pressure coefficient behind a normal shock.
pub fn cp_max(m: f64) -> f64 {
    let m2 = m * m;
    let a = ((G + 1.0).powi(2) * m2 / (4.0 * G * m2 - 2.0 * (G - 1.0))).powf(G / (G - 1.0));
    let b = (1.0 - G + 2.0 * G * m2) / (G + 1.0);
    (a * b - 1.0) / (0.5 * G * m2)
}

/// Modified Newtonian pressure coefficient.
pub fn newton_cp(m: f64, delta: f64) -> f64 {
    cp_max(m) * delta.sin().powi(2)
}

// ------------------------------------------------------------------------------------------
// Taylor–Maccoll cone table
// ------------------------------------------------------------------------------------------

/// Surface state of a cone from a given shock angle: `(cone half angle, Cp)`.
fn taylor_maccoll(m1: f64, beta: f64) -> Option<(f64, f64)> {
    let mn1 = m1 * beta.sin();
    if mn1 <= 1.0 {
        return None;
    }
    let ts = deflection(m1, beta);
    let mn2sq = (1.0 + 0.5 * (G - 1.0) * mn1 * mn1) / (G * mn1 * mn1 - 0.5 * (G - 1.0));
    let m2sq = mn2sq / (beta - ts).sin().powi(2);
    let vmax = |msq: f64| ((G - 1.0) * msq / (2.0 + (G - 1.0) * msq)).sqrt();
    let v2 = vmax(m2sq);
    let mut th = beta;
    let (mut vr, mut vt) = (v2 * (beta - ts).cos(), -v2 * (beta - ts).sin());
    let f = |th: f64, vr: f64, vt: f64| -> (f64, f64) {
        let a2 = 0.5 * (G - 1.0) * (1.0 - vr * vr - vt * vt);
        let den = a2 - vt * vt;
        (vt, (vt * vt * vr - a2 * (2.0 * vr + vt / th.tan())) / den)
    };
    while th > 0.002f64.to_radians() {
        let h = -(th / 60.0).min(0.1f64.to_radians());
        let (k1r, k1t) = f(th, vr, vt);
        let (k2r, k2t) = f(th + 0.5 * h, vr + 0.5 * h * k1r, vt + 0.5 * h * k1t);
        let (k3r, k3t) = f(th + 0.5 * h, vr + 0.5 * h * k2r, vt + 0.5 * h * k2t);
        let (k4r, k4t) = f(th + h, vr + h * k3r, vt + h * k3t);
        let nr = vr + h / 6.0 * (k1r + 2.0 * k2r + 2.0 * k3r + k4r);
        let nt = vt + h / 6.0 * (k1t + 2.0 * k2t + 2.0 * k3t + k4t);
        if !nr.is_finite() || !nt.is_finite() {
            return None;
        }
        if nt >= 0.0 {
            let w = vt / (vt - nt);
            let thc = th + w * h;
            let vrc = vr + w * (nr - vr);
            let p2 = 1.0 + 2.0 * G / (G + 1.0) * (mn1 * mn1 - 1.0);
            let ratio = ((1.0 - vrc * vrc) / (1.0 - v2 * v2)).powf(G / (G - 1.0));
            return Some((thc, (p2 * ratio - 1.0) / (0.5 * G * m1 * m1)));
        }
        th += h;
        vr = nr;
        vt = nt;
    }
    None
}

const CONE_M0: f64 = 1.02;
const CONE_DM: f64 = 0.04;
const CONE_NM: usize = 150;
const CONE_DD: f64 = 0.25; // degrees

struct ConeRow {
    /// Cp at cone angles `k * CONE_DD` degrees, `k = 0..cp.len()`.
    cp: Vec<f64>,
}

struct ConeTable {
    rows: Vec<ConeRow>,
}

impl ConeTable {
    fn build() -> ConeTable {
        let rows = (0..CONE_NM)
            .into_par_iter()
            .map(|i| {
                let m = CONE_M0 + CONE_DM * i as f64;
                let mu = (1.0 / m).asin();
                let mut curve = vec![(0.0, 0.0)];
                let n = 600;
                for k in 1..=n {
                    // Shock angles clustered geometrically towards the Mach angle, where small
                    // cones live (the cone angle grows like the square root of beta - mu).
                    let u = (k as f64 / n as f64 - 1.0) * 9.0;
                    let beta = mu + (std::f64::consts::FRAC_PI_2 - mu) * 10f64.powf(u);
                    match taylor_maccoll(m, beta) {
                        Some((tc, cp)) => {
                            let top = curve.last().unwrap().0;
                            if tc > top {
                                curve.push((tc, cp));
                            } else if tc < top - 0.05f64.to_radians() {
                                break; // past the detachment limit
                            }
                        }
                        None => {
                            if curve.len() > 1 {
                                break;
                            }
                        }
                    }
                }
                let tmax = curve.last().unwrap().0.to_degrees();
                let nk = (tmax / CONE_DD).floor() as usize;
                let mut cp = Vec::with_capacity(nk + 1);
                let mut j = 0;
                for k in 0..=nk {
                    let t = (k as f64 * CONE_DD).to_radians();
                    while j + 2 < curve.len() && curve[j + 1].0 < t {
                        j += 1;
                    }
                    let (a, b) = (curve[j], curve[(j + 1).min(curve.len() - 1)]);
                    let w = if b.0 > a.0 { ((t - a.0) / (b.0 - a.0)).clamp(0.0, 1.0) } else { 0.0 };
                    cp.push(a.1 + w * (b.1 - a.1));
                }
                ConeRow { cp }
            })
            .collect();
        ConeTable { rows }
    }

    fn lookup(&self, m: f64, delta: f64) -> Option<f64> {
        let x = ((m - CONE_M0) / CONE_DM).clamp(0.0, (CONE_NM - 1) as f64 - 1e-9);
        let i = x as usize;
        let w = x - i as f64;
        let d = delta.to_degrees() / CONE_DD;
        let k = d as usize;
        let at = |r: &ConeRow| -> Option<f64> {
            if k + 1 >= r.cp.len() {
                return None;
            }
            if k == 0 {
                // Cone pressure grows quadratically from zero.
                return Some(r.cp[1] * d * d);
            }
            let f = d - k as f64;
            Some(r.cp[k] + f * (r.cp[k + 1] - r.cp[k]))
        };
        let (a, b) = (at(&self.rows[i])?, at(&self.rows[i + 1])?);
        Some(a + w * (b - a))
    }
}

fn cone_table() -> &'static ConeTable {
    static T: OnceLock<ConeTable> = OnceLock::new();
    T.get_or_init(ConeTable::build)
}

/// Tangent-cone surface pressure coefficient; `None` beyond the detachment limit.
pub fn cone_cp(m: f64, delta: f64) -> Option<f64> {
    if m <= CONE_M0 - 0.02 {
        return None;
    }
    cone_table().lookup(m, delta)
}

fn cone_or_newton(m: f64, delta: f64) -> f64 {
    cone_cp(m, delta).unwrap_or_else(|| newton_cp(m, delta))
}

/// Body-panel pressure coefficient: axisymmetric part at slope `delta0` plus a linearised
/// increment for the actual inclination `delta`, with the Newtonian slope
/// `4 sin d0 cos d0` (slender-body law `Cp = 2 d^2`; the cone log-law slope `2 Cp_cone/d0` gives
/// CNa of about 3.4 for a 10 deg cone at M=2 against 2 from slender-body theory and Stone). The increment vanishes on a cylinder, so incidence produces
/// the slender-body normal force `2 dS/dx` and no spurious first-order load on cylindrical
/// parts; the second-order cross-flow load is added by the empirical cross-flow term.
pub fn body_cp(m: f64, delta: f64, delta0: f64) -> f64 {
    let (cp0, slope) = if delta0 > 1e-12 {
        let c = cone_or_newton(m, delta0);
        (c, 4.0 * delta0.sin() * delta0.cos())
    } else if delta0 < -1e-12 {
        (expansion_cp(m, -delta0), 4.0 * delta0.sin() * delta0.cos())
    } else {
        (0.0, 0.0)
    };
    (cp0 + slope * (delta - delta0)).clamp(-2.0 / (G * m * m), cp_max(m))
}

/// Pressure coefficient of a fin panel at local inclination `delta` [rad].
pub fn panel_cp(m: f64, delta: f64, part: Part) -> f64 {
    if delta.abs() < 1e-12 {
        0.0
    } else if delta > 0.0 {
        let attached = match part {
            Part::Body => cone_cp(m, delta),
            Part::Fin | Part::Tail => wedge_cp(m, delta),
        };
        attached.unwrap_or_else(|| newton_cp(m, delta))
    } else {
        expansion_cp(m, -delta)
    }
}

// ------------------------------------------------------------------------------------------
// Panel integration
// ------------------------------------------------------------------------------------------

/// Pressure force coefficients (per `q S_ref`) in body axes.
#[derive(Debug, Clone, Copy, Default)]
pub struct Forces {
    /// Axial (drag-like, along +x) pressure force.
    pub fx: f64,
    /// Normal force along +z.
    pub fz: f64,
    /// Sum of normal force times x (moment about the nose tip).
    pub mz: f64,
}

/// Panel data needed for the local-inclination evaluation.
pub struct SupersonicPanels {
    c: Vec<Vec3>,
    n: Vec<Vec3>,
    area: Vec<f64>,
    part: Vec<Part>,
    /// Axisymmetric slope of each panel (inclination at zero incidence).
    delta0: Vec<f64>,
    /// Fin tip leading-edge corner `(x, radius)` for the 3-D tip relief.
    tip_le: Option<(f64, f64)>,
    s_ref: f64,
}

impl SupersonicPanels {
    pub fn new(mesh: &Mesh, s_ref: f64, tip_le: Option<(f64, f64)>) -> Self {
        let p: Vec<&crate::mesh::Panel> = mesh.panels.iter().filter(|q| q.part != Part::Tail).collect();
        Self {
            c: p.iter().map(|q| q.centroid).collect(),
            n: p.iter().map(|q| q.normal).collect(),
            area: p.iter().map(|q| q.area).collect(),
            part: p.iter().map(|q| q.part).collect(),
            delta0: p.iter().map(|q| (-q.normal.x).clamp(-1.0, 1.0).asin()).collect(),
            tip_le,
            s_ref,
        }
    }

    /// Linear-theory 3-D relief: inside the Mach cone of the fin-tip leading-edge corner the
    /// incidence loading is half of the two-dimensional value, elsewhere unchanged.
    fn tip_factor(&self, m: f64, c: Vec3) -> f64 {
        let Some((x_tip, r_tip)) = self.tip_le else { return 1.0 };
        let beta = (m * m - 1.0).max(0.0).sqrt();
        let rho = c.y.hypot(c.z);
        if c.x - x_tip >= beta * (r_tip - rho) {
            0.5
        } else {
            1.0
        }
    }

    /// Forces at Mach `m`, angle of attack `alpha` and nondimensional pitch rate `omega`
    /// (rotation about the nose tip: extra local incidence `omega * x`).
    pub fn forces(&self, m: f64, alpha: f64, omega: f64) -> Forces {
        let (sa, ca) = alpha.sin_cos();
        let mut f = Forces::default();
        for i in 0..self.c.len() {
            let u = Vec3::new(ca, 0.0, sa + omega * self.c[i].x).normalized();
            let delta = (-self.n[i].dot(u)).clamp(-1.0, 1.0).asin();
            let cp = if self.part[i] == Part::Body {
                body_cp(m, delta, self.delta0[i])
            } else {
                let cp0 = panel_cp(m, self.delta0[i], self.part[i]);
                cp0 + self.tip_factor(m, self.c[i]) * (panel_cp(m, delta, self.part[i]) - cp0)
            };
            let w = -cp * self.area[i] / self.s_ref;
            f.fx += w * self.n[i].x;
            f.fz += w * self.n[i].z;
            f.mz += w * self.n[i].z * self.c[i].x;
        }
        f
    }

    /// Stability derivatives by central differences: `(CNa, M_alpha, N_rot, M_rot)`.
    pub fn derivatives(&self, m: f64, length: f64) -> [f64; 4] {
        let ha = 0.25f64.to_radians();
        let ho = 0.25f64.to_radians() / length;
        let (ap, am) = (self.forces(m, ha, 0.0), self.forces(m, -ha, 0.0));
        let (rp, rm) = (self.forces(m, 0.0, ho), self.forces(m, 0.0, -ho));
        [(ap.fz - am.fz) / (2.0 * ha), (ap.mz - am.mz) / (2.0 * ha), (rp.fz - rm.fz) / (2.0 * ho), (rp.mz - rm.mz) / (2.0 * ho)]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cone_10deg_mach2() {
        let cp = cone_cp(2.0, 10f64.to_radians()).unwrap();
        println!("cone 10 deg M=2: table Cp {cp:.4}");
        assert!((cp - 0.098).abs() < 0.01, "cone Cp {cp}");
        // Direct shooting at the known shock angle (~31.3 deg).
        let (tc, cp2) = taylor_maccoll(2.0, 31.3f64.to_radians()).unwrap();
        assert!((tc.to_degrees() - 10.0).abs() < 0.3, "cone angle {}", tc.to_degrees());
        assert!((cp2 - 0.098).abs() < 0.02, "cone Cp from shooting {cp2}");
    }

    #[test]
    fn oblique_shock_and_pm() {
        let b = wedge_beta(2.0, 10f64.to_radians()).unwrap().to_degrees();
        assert!((b - 39.3).abs() < 0.1, "beta {b}");
        let cp = wedge_cp(2.0, 10f64.to_radians()).unwrap();
        assert!((cp - 0.2525).abs() < 0.005, "wedge Cp {cp}");
        assert!((prandtl_meyer(2.0).to_degrees() - 26.38).abs() < 0.01);
        assert!(wedge_cp(2.0, 30f64.to_radians()).is_none());
        // Expansion by a small angle follows linear theory -2 delta / sqrt(M^2-1).
        let d = 0.2f64.to_radians();
        let lin = -2.0 * d / 3f64.sqrt();
        assert!((expansion_cp(2.0, d) - lin).abs() < 0.02 * lin.abs(), "{} vs {lin}", expansion_cp(2.0, d));
        assert!((expansion_cp(2.0, 3.0) + 2.0 / (G * 4.0)).abs() < 1e-9);
    }

    #[test]
    fn cone_cp_is_smooth_and_bounded() {
        let mut prev = 0.0;
        for k in 1..60 {
            let d = (k as f64 * 0.5).to_radians();
            let cp = panel_cp(1.5, d, Part::Body);
            assert!(cp > prev - 0.02 && cp.is_finite() && cp < cp_max(1.5) * 1.05, "d={k}: {cp}");
            prev = cp;
        }
    }
}
