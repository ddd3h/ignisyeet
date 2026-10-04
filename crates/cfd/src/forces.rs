// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025-2026 西濱大将 (NISHIHAMA Daisuke)
//! Conversion of SU2 coefficients to IgnisYeet body-axis coefficients, hybrid viscous terms and
//! pitch-damping scaling.
//!
//! # Conventions
//!
//! The mesh is in the body frame (x aft from the nose tip, z up, see [`crate::su2cfg`]) and the
//! freestream is rotated, so SU2's `CFx`, `CFz`, `CMy` are body-axis quantities:
//!
//! * `CA = CFx` (positive aft), `CN = CFz` (positive for positive alpha).
//! * SU2 evaluates `M = r x F`; the y component is `z Fx - x Fz`. A normal force `Fz > 0`
//!   at `x = xcp` aft of the nose therefore gives `CMy = -xcp CN / L_ref`, i.e.
//!   `xcp = -CMy L_ref / CN`. The nose moment of the normal force is `mom = CN xcp = -CMy L_ref`.
//! * If only `CL`, `CD` are available they are rotated back: with the freestream direction
//!   `d = (cos a, 0, sin a)` and lift direction `(-sin a, 0, cos a)`:
//!   `Fx = CD cos a - CL sin a`, `Fz = CD sin a + CL cos a`.
//! * Half models are normalised with `REF_AREA / 2`, so no further factor is needed.

use crate::config::FlowModel;
use crate::history::HistorySummary;
use aero::model::axial_alpha_factor;
use aero::AeroModel;
use anyhow::{bail, Result};

/// Force and moment coefficients as reported by SU2 (body axes, nose-tip moment origin).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Su2Coeffs {
    pub cfx: f64,
    pub cfz: f64,
    pub cmy: f64,
}

impl Su2Coeffs {
    /// From a history summary; `CFx` / `CFz` are rebuilt from `CD` / `CL` when absent.
    pub fn from_summary(s: &HistorySummary, alpha_deg: f64) -> Result<Self> {
        let a = alpha_deg.to_radians();
        let cfx = match (s.cfx, s.cd, s.cl) {
            (Some(x), _, _) => x,
            (None, Some(d), Some(l)) => d * a.cos() - l * a.sin(),
            _ => bail!("history has neither CFx nor CD and CL"),
        };
        let cfz = match (s.cfz, s.cd, s.cl) {
            (Some(z), _, _) => z,
            (None, Some(d), Some(l)) => d * a.sin() + l * a.cos(),
            _ => bail!("history has neither CFz nor CD and CL"),
        };
        let Some(cmy) = s.cmy else { bail!("history has no CMy") };
        if ![cfx, cfz, cmy].iter().all(|v| v.is_finite()) {
            bail!("non-finite coefficients in the history (diverged case)");
        }
        Ok(Self { cfx, cfz, cmy })
    }
}

/// Body-axis coefficients of one solved case (before hybrid terms).
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct BodyCoeffs {
    pub cn: f64,
    /// Axial force coefficient from CFD (positive aft).
    pub ca: f64,
    /// Nose-tip moment of the normal force, `CN xcp` [m].
    pub mom: f64,
}

impl BodyCoeffs {
    pub fn from_su2(c: &Su2Coeffs, ref_length: f64) -> Self {
        Self { cn: c.cfz, ca: c.cfx, mom: -c.cmy * ref_length }
    }

    /// Centre of pressure aft of the nose [m]; `None` when `|CN|` is negligible.
    pub fn xcp(&self) -> Option<f64> {
        (self.cn.abs() > 1e-9).then(|| self.mom / self.cn)
    }
}

/// Axial force coefficients (power-on, power-off) at `mach` / `alpha_rad` from the CFD axial
/// coefficient `ca_cfd` plus the terms CFD does not supply.
///
/// * Euler: friction + base + fin-edge (rounded leading edge, square trailing edge) + boattail +
///   extra drag of the Barrowman build-up. The nose pressure term is **excluded** (CFD gives the
///   pressure / wave drag). The CFD base cap is not monitored (see `su2cfg`), so the base term is
///   fully taken from the model; power-on uses the base area reduced by the nozzle exit area.
/// * RANS: friction, base pressure, fin and boattail drag come from CFD; only `extra_cd` is added,
///   and for power-on the base-pressure contribution over the nozzle exit area is removed,
///   `base_cd(M) (A_base - A_base,on) / S`.
///
/// The additions are multiplied by the model's `axial_alpha_factor`, like the Barrowman path.
pub fn hybrid_axial(flow: FlowModel, model: &AeroModel, mach: f64, alpha_rad: f64, ca_cfd: f64) -> (f64, f64) {
    let k = axial_alpha_factor(alpha_rad);
    let (on, off) = (model.cd0(mach, true), model.cd0(mach, false));
    let (add_on, add_off) = match flow {
        FlowModel::Euler => {
            let t = |b: &aero::model::DragBreakdown| b.friction + b.base + b.fins + b.boattail + b.extra;
            (t(&on), t(&off))
        }
        FlowModel::Rans => {
            let removed = (off.base - on.base).max(0.0);
            // Base drag over the nozzle exit area, `base_cd(M) A_nozzle / S`, is the model's off-on base difference.
            (off.extra - removed, off.extra)
        }
    };
    ((ca_cfd + add_on * k).max(0.0), (ca_cfd + add_off * k).max(0.0))
}

/// Pitch-damping sums `[S0, S1, S2]` (sum CNa_i, sum CNa_i x_i, sum CNa_i x_i^2) rescaled to CFD.
///
/// CFD steady runs give no damping. The model's sums `model_s` (Barrowman, or the panel method
/// when available) keep their *spread* about the centroid, `var = S2/S0 - (S1/S0)^2`, while the
/// total slope and the centroid are replaced by the CFD values: `S0 = cna`,
/// `S1 = cna xcp0`, `S2 = cna (xcp0^2 + var)`.
pub fn scale_damping(model_s: [f64; 3], cna: f64, xcp0: f64) -> [f64; 3] {
    let var = if model_s[0].abs() > 1e-12 { (model_s[2] / model_s[0] - (model_s[1] / model_s[0]).powi(2)).max(0.0) } else { 0.0 };
    [cna, cna * xcp0, cna * (xcp0 * xcp0 + var)]
}

#[cfg(test)]
mod tests {
    use super::*;
    use aero::AeroOptions;
    use geom::sample::SampleRocket;
    use geom::{extract, ExtractOptions};

    fn model(nozzle: f64) -> AeroModel {
        let g = extract(&SampleRocket::default().mesh(), &ExtractOptions::default()).unwrap();
        AeroModel::new(g, AeroOptions { nozzle_exit_area: nozzle, extra_cd: 0.02, ..Default::default() })
    }

    /// SU2's moment about the origin: M = r x F, y component (Fx z - Fz x), normalised by q S L.
    fn su2_cmy(r: [f64; 3], f: [f64; 3], qsl: f64) -> f64 {
        (f[0] * r[2] - f[2] * r[0]) / qsl
    }

    #[test]
    fn xcp_from_synthetic_force() {
        // Normal force 12 N (+z) and axial force 3 N (+x) acting at x = 0.8 m on the axis.
        let (q, s, l) = (500.0, 0.0078, 0.1);
        let qsl = q * s * l;
        let cmy = su2_cmy([0.8, 0.0, 0.0], [3.0, 0.0, 12.0], qsl);
        assert!(cmy < 0.0);
        let c = BodyCoeffs::from_su2(&Su2Coeffs { cfx: 3.0 / (q * s), cfz: 12.0 / (q * s), cmy }, l);
        assert!((c.cn - 12.0 / (q * s)).abs() < 1e-12 && (c.ca - 3.0 / (q * s)).abs() < 1e-12);
        assert!((c.xcp().unwrap() - 0.8).abs() < 1e-12);
        // Force applied off-axis at z = 0.05: the axial force adds a moment, xcp changes accordingly.
        let cmy2 = su2_cmy([0.8, 0.0, 0.05], [3.0, 0.0, 12.0], qsl);
        let c2 = BodyCoeffs::from_su2(&Su2Coeffs { cfx: 3.0 / (q * s), cfz: 12.0 / (q * s), cmy: cmy2 }, l);
        assert!((c2.xcp().unwrap() - (0.8 - 3.0 * 0.05 / 12.0)).abs() < 1e-12);
    }

    #[test]
    fn cl_cd_rotation_roundtrip() {
        let a = 7f64.to_radians();
        let (fx, fz) = (0.31, 0.52);
        let (cd, cl) = (fx * a.cos() + fz * a.sin(), -fx * a.sin() + fz * a.cos());
        let s = HistorySummary { cd: Some(cd), cl: Some(cl), cmy: Some(-0.1), ..Default::default() };
        let c = Su2Coeffs::from_summary(&s, 7.0).unwrap();
        assert!((c.cfx - fx).abs() < 1e-12 && (c.cfz - fz).abs() < 1e-12);
        assert!(Su2Coeffs::from_summary(&HistorySummary::default(), 0.0).is_err());
    }

    #[test]
    fn hybrid_terms() {
        let m = model(0.0);
        let (on, off) = hybrid_axial(FlowModel::Euler, &m, 0.5, 0.0, 0.1);
        let b = m.cd0(0.5, false);
        assert!((off - (0.1 + b.friction + b.base + b.fins + b.boattail + b.extra)).abs() < 1e-12);
        assert!((on - off).abs() < 1e-12, "no nozzle: power-on equals power-off");
        let m = model(0.5 * std::f64::consts::PI * 0.05 * 0.05);
        let (on, off) = hybrid_axial(FlowModel::Euler, &m, 0.5, 0.0, 0.1);
        assert!(on < off);
        // RANS: only extra_cd (and the nozzle correction) are added.
        let (ron, roff) = hybrid_axial(FlowModel::Rans, &m, 0.5, 0.0, 0.4);
        assert!((roff - 0.42).abs() < 1e-12);
        assert!(ron < roff && ron > 0.0);
    }

    #[test]
    fn damping_scaling_matches_cfd_slope_and_centroid() {
        let s = scale_damping([3.0, 3.6, 4.9], 4.0, 1.1);
        assert!((s[0] - 4.0).abs() < 1e-12 && (s[1] / s[0] - 1.1).abs() < 1e-12);
        let var = 4.9 / 3.0 - 1.2f64 * 1.2;
        assert!((s[2] / s[0] - (1.21 + var)).abs() < 1e-12);
    }
}
