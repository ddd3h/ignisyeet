// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025-2026 西濱大将 (NISHIHAMA Daisuke)
//! Skin friction by surface integration and the empirical axial-force increments.
//!
//! Local friction coefficient of a flat plate at running length `x_s` (nose tip for the body,
//! fin leading edge for the fins), Reynolds number `Re_x = V x_s / nu` in the configured atmosphere
//! (`AeroOptions::atmosphere`) at the reference altitude:
//! * compressibility by the Eckert reference temperature with an adiabatic wall,
//!   `T*/T = 1 + 0.032 M^2 + 0.58 (Tw/T - 1)`, `Tw/T = 1 + r (g-1)/2 M^2`, `r = 0.89`;
//!   `Re* = Re_x (T*/T)^-1.76` (power-law viscosity) and `cf = cf_inc(Re*) / (T*/T)`;
//! * laminar Blasius `0.664 / sqrt(Re*)` below `Re* = 5e5`, turbulent `0.0592 Re*^-0.2` above;
//! * fully rough floor (turbulent region only) `0.0256 (Rs/x_s)^0.2 (T*/T)^-0.65`, the local
//!   equivalent of the Barrowman-model average `0.032 (Rs/L)^0.2`.
//!
//! The axial force is `sum cf A sqrt(1 - n_x^2)` over the panels; base, fin edge, boattail and
//! additional drag come from `aero::AeroModel` (same flags as the Barrowman path).

use crate::mesh::Mesh;
use aero::atmosphere::GAMMA;
use aero::AeroOptions;

const RECOVERY: f64 = 0.89;
const RE_TRANSITION: f64 = 5e5;

/// Local skin-friction coefficient (referenced to the freestream dynamic pressure).
pub fn cf_local(re_x: f64, m: f64, x_s: f64, roughness: f64) -> f64 {
    let tw = 1.0 + RECOVERY * 0.5 * (GAMMA - 1.0) * m * m;
    let ts = 1.0 + 0.032 * m * m + 0.58 * (tw - 1.0);
    let re = (re_x * ts.powf(-1.76)).max(100.0);
    if re < RE_TRANSITION {
        return 0.664 / re.sqrt() / ts;
    }
    let turb = 0.0592 * re.powf(-0.2) / ts;
    if roughness > 0.0 {
        turb.max(0.0256 * (roughness / x_s).powf(0.2) * ts.powf(-0.65))
    } else {
        turb
    }
}

/// Per-panel data for the friction integral.
pub struct Friction {
    run: Vec<f64>,
    /// Panel area times the axial component of the unit tangent, `A sqrt(1 - n_x^2)`.
    axial_area: Vec<f64>,
    s_ref: f64,
    /// Wetted area (diagnostic).
    pub wetted_area: f64,
}

impl Friction {
    pub fn new(mesh: &Mesh, s_ref: f64) -> Self {
        let p: Vec<&crate::mesh::Panel> = mesh.panels.iter().filter(|q| q.part != crate::mesh::Part::Tail).collect();
        Self {
            run: p.iter().map(|p| p.run.max(1e-6)).collect(),
            axial_area: p.iter().map(|p| p.area * (1.0 - p.normal.x * p.normal.x).max(0.0).sqrt()).collect(),
            s_ref,
            wetted_area: p.iter().map(|p| p.area).sum(),
        }
    }

    /// Friction axial-force coefficient at Mach `m` (Reynolds number floored at M = 0.05).
    pub fn ca(&self, m: f64, aero: &AeroOptions) -> f64 {
        let atm = aero.atmosphere.at(aero.reference_altitude);
        let v = m.max(0.05) * atm.sound_speed;
        let nu = atm.kinematic_viscosity();
        let sum: f64 = self.run.iter().zip(&self.axial_area).map(|(&x, &a)| cf_local(v * x / nu, m, x, aero.roughness) * a).sum();
        sum / self.s_ref
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flat_plate_values() {
        // Incompressible turbulent plate at Re_x = 1e7: 0.0592 * Re^-0.2 = 2.3e-3.
        let cf = cf_local(1e7, 0.0, 1.0, 0.0);
        assert!((cf - 0.0592 * 1e7f64.powf(-0.2)).abs() < 1e-9);
        // Laminar below transition.
        assert!((cf_local(1e5, 0.0, 1.0, 0.0) - 0.664 / 1e5f64.sqrt()).abs() < 1e-9);
        // Compressibility lowers cf.
        assert!(cf_local(1e7, 2.0, 1.0, 0.0) < cf);
    }

    /// The friction Reynolds number uses the configured atmosphere (temperature offset, constant
    /// model), as the Barrowman skin friction does.
    #[test]
    fn friction_uses_the_configured_atmosphere() {
        use aero::atmosphere::AtmosphereModel;
        let f = Friction { run: vec![0.1, 0.5, 1.0], axial_area: vec![0.01, 0.02, 0.03], s_ref: 0.01, wetted_area: 0.06 };
        let base = AeroOptions { roughness: 0.0, ..Default::default() };
        let hot = AeroOptions { atmosphere: AtmosphereModel::us1976(30.0).unwrap(), ..base.clone() };
        let (c0, c1) = (f.ca(0.3, &base), f.ca(0.3, &hot));
        // Hotter air: higher kinematic viscosity at the same Mach number despite the faster flow -> lower Re -> higher cf.
        assert!(c1 > c0 * 1.001, "{c0} vs {c1}");
        let atm = hot.atmosphere.at(hot.reference_altitude);
        let v = 0.3 * atm.sound_speed;
        let nu = atm.kinematic_viscosity();
        let expect: f64 = f.run.iter().zip(&f.axial_area).map(|(&x, &a)| cf_local(v * x / nu, 0.3, x, 0.0) * a).sum::<f64>() / f.s_ref;
        assert!((c1 - expect).abs() < 1e-15, "{c1} vs {expect}");
    }
}
