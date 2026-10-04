// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025-2026 西濱大将 (NISHIHAMA Daisuke)
//! SU2 (v8) configuration file for one (Mach, alpha) case.
//!
//! # Frame and sign conventions (CFD frame)
//!
//! * The body is **fixed** in the mesh and the freestream direction is rotated by the angle of
//!   attack, so SU2's force and moment coefficients are directly body-axis coefficients.
//! * `x` points **downstream** and equals the body axis measured **aft from the nose tip**
//!   (nose tip at the origin, base at `x = L`). `z` is up, `y` is span.
//! * SU2 (3-D) uses freestream velocity direction `(cos a cos b, sin b, sin a cos b)`. With
//!   `SIDESLIP_ANGLE = 0` the flow has a `+z` component for `a > 0`, so the normal force on the
//!   body is `+z`: `CN = CFz` and `CA = CFx` (positive aft, drag-like).
//! * Pitch plane = the x-z plane. The symmetry plane is `y = 0`; the half model keeps `y >= 0`.
//! * Moments are taken about the nose tip (`REF_ORIGIN_MOMENT = 0 0 0`). SU2 computes
//!   `M = r x F`, so `CMy = (z Fx - x Fz) / (q S L)` and a positive normal force acting aft of
//!   the nose gives a negative `CMy` (see [`crate::forces`]).
//! * `REF_LENGTH` is the body diameter `d`, `REF_AREA` the body cross-section `S`; with a half
//!   model `REF_AREA = S/2` so the half-model forces yield full-model coefficients.
//! * Markers: `wall` (monitored body surface), `base` (flat base cap; for Euler an Euler wall
//!   that is **not** monitored, so the physically meaningless inviscid base pressure never
//!   enters the axial force and the Barrowman base term replaces it; for RANS a monitored
//!   adiabatic wall), `farfield`, `symmetry`.
//! * Thermodynamic state: ideal air at the pressure and temperature of the reference altitude
//!   (`TD_CONDITIONS`, `TEMPERATURE_FS`); for RANS SU2 then derives the Reynolds number from the
//!   mesh length unit (metres) and the Sutherland viscosity, which equals the Sutherland law of
//!   `aero::atmosphere`.
//! * RANS (no boundary-layer prisms, see `mesh`): SA, standard wall function, first-order Roe, CFL limit `4 cfl`.
//! * Time stepping: implicit Euler with adaptive CFL starting at `cfl`
//!   (limits `0.2 cfl` .. `50 cfl`, factors 0.5 down / 1.2 up).

use crate::config::{CfdOptions, FlowModel, Scheme};
use aero::atmosphere::{Atmosphere, GAMMA, R_AIR};

/// Iteration at which the slope limiter is frozen (ROE + MUSCL).
pub const LIMITER_FREEZE_ITER: usize = 250;

/// Iteration cap of RANS cases (see `su2_config`).
pub const RANS_MAX_ITER: usize = 300;

/// Reference dimensions of the full (not halved) rocket.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RefDims {
    /// Reference length: body diameter [m].
    pub length: f64,
    /// Reference area: body cross-section [m^2].
    pub area: f64,
}

/// File names (relative to the case directory) used by a case.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaseFiles {
    pub mesh: String,
    /// Restart file of a neighbouring case used as initial solution (warm start).
    pub restart_from: Option<String>,
}

impl Default for CaseFiles {
    fn default() -> Self {
        Self { mesh: "mesh.su2".into(), restart_from: None }
    }
}

#[derive(Default)]
struct Cfg(Vec<String>);

impl Cfg {
    fn kv(&mut self, k: &str, v: impl std::fmt::Display) {
        self.0.push(format!("{k}= {v}"));
    }
    fn blank(&mut self) {
        self.0.push(String::new());
    }
}

/// Text of the SU2 configuration for Mach `mach` and angle of attack `alpha_deg`.
pub fn su2_config(opt: &CfdOptions, mach: f64, alpha_deg: f64, dims: &RefDims, atm: &Atmosphere, files: &CaseFiles) -> String {
    let rans = opt.model == FlowModel::Rans;
    let area = if opt.symmetry { 0.5 * dims.area } else { dims.area };
    let mut c = Cfg::default();
    c.kv("SOLVER", if rans { "RANS" } else { "EULER" });
    if rans {
        c.kv("KIND_TURB_MODEL", "SA");
    }
    c.kv("MATH_PROBLEM", "DIRECT");
    c.kv("SYSTEM_MEASUREMENTS", "SI");
    c.kv("RESTART_SOL", if files.restart_from.is_some() { "YES" } else { "NO" });
    if let Some(f) = &files.restart_from {
        c.kv("SOLUTION_FILENAME", f);
    }
    c.blank();
    c.kv("MACH_NUMBER", format!("{mach:.6}"));
    c.kv("AOA", format!("{alpha_deg:.6}"));
    c.kv("SIDESLIP_ANGLE", "0.0");
    c.kv("FLUID_MODEL", "IDEAL_GAS");
    c.kv("GAMMA_VALUE", GAMMA);
    c.kv("GAS_CONSTANT", format!("{R_AIR:.5}"));
    c.kv("INIT_OPTION", "TD_CONDITIONS");
    c.kv("FREESTREAM_OPTION", "TEMPERATURE_FS");
    c.kv("FREESTREAM_PRESSURE", format!("{:.4}", atm.pressure));
    c.kv("FREESTREAM_TEMPERATURE", format!("{:.4}", atm.temperature));
    if rans {
        c.kv("VISCOSITY_MODEL", "SUTHERLAND");
        c.kv("MU_REF", "1.716E-5");
        c.kv("MU_T_REF", "273.15");
        c.kv("SUTHERLAND_CONSTANT", "110.4");
        c.kv("FREESTREAM_NU_FACTOR", "3.0");
    }
    c.kv("REF_DIMENSIONALIZATION", "DIMENSIONAL");
    c.blank();
    c.kv("REF_ORIGIN_MOMENT_X", "0.0");
    c.kv("REF_ORIGIN_MOMENT_Y", "0.0");
    c.kv("REF_ORIGIN_MOMENT_Z", "0.0");
    c.kv("REF_LENGTH", format!("{:.8}", dims.length));
    c.kv("REF_AREA", format!("{area:.10}"));
    c.blank();
    if rans {
        c.kv("MARKER_HEATFLUX", "( wall, 0.0, base, 0.0 )");
        // No prismatic boundary layer (see `mesh`): the first cell is at y+ ~ 50-300, so the wall shear
        // comes from the standard (log-law) wall function.
        c.kv("MARKER_WALL_FUNCTIONS", "( wall, STANDARD_WALL_FUNCTION, base, STANDARD_WALL_FUNCTION )");
        c.kv("MARKER_MONITORING", "( wall, base )");
    } else {
        c.kv("MARKER_EULER", "( wall, base )");
        c.kv("MARKER_MONITORING", "( wall )");
    }
    c.kv("MARKER_FAR", "( farfield )");
    if opt.symmetry {
        c.kv("MARKER_SYM", "( symmetry )");
    }
    c.kv("MARKER_PLOTTING", "( wall, base )");
    c.blank();
    c.kv("NUM_METHOD_GRAD", "WEIGHTED_LEAST_SQUARES");
    if opt.scheme == Scheme::Jst {
        c.kv("CONV_NUM_METHOD_FLOW", "JST");
        c.kv("JST_SENSOR_COEFF", "( 0.5, 0.02 )");
        c.kv("MUSCL_FLOW", "NO");
    } else if rans {
        // Second-order reconstruction diverged on the tetrahedral wall mesh: first-order Roe.
        c.kv("CONV_NUM_METHOD_FLOW", "ROE");
        c.kv("MUSCL_FLOW", "NO");
    } else {
        c.kv("CONV_NUM_METHOD_FLOW", "ROE");
        c.kv("MUSCL_FLOW", "YES");
        c.kv("SLOPE_LIMITER_FLOW", "VENKATAKRISHNAN");
        c.kv("VENKAT_LIMITER_COEFF", "0.05");
        // Freeze the limiter once the flow has developed: without it the limiter chatter keeps the
        // residual at about 1e-4 and the base wake of the Euler model prevents convergence.
        c.kv("LIMITER_ITER", LIMITER_FREEZE_ITER);
    }
    c.kv("TIME_DISCRE_FLOW", "EULER_IMPLICIT");
    if rans {
        c.kv("CONV_NUM_METHOD_TURB", "SCALAR_UPWIND");
        c.kv("MUSCL_TURB", "NO");
        c.kv("TIME_DISCRE_TURB", "EULER_IMPLICIT");
    }
    c.blank();
    c.kv("CFL_NUMBER", opt.cfl);
    c.kv("CFL_ADAPT", "YES");
    // ( factor down, factor up, CFL min, CFL max )
    let cfl_max = if rans { 4.0 } else { 50.0 };
    c.kv("CFL_ADAPT_PARAM", format!("( 0.5, 1.2, {:.3}, {:.1} )", 0.2 * opt.cfl, cfl_max * opt.cfl));
    c.kv("LINEAR_SOLVER", "FGMRES");
    c.kv("LINEAR_SOLVER_PREC", "ILU");
    c.kv("LINEAR_SOLVER_ERROR", "1E-6");
    c.kv("LINEAR_SOLVER_ITER", "10");
    c.blank();
    // RANS (first-order Roe, no prisms): the residual stalls near 1e-5 and drifts towards divergence
        // after a few hundred iterations while the coefficients are flat from about iteration 100.
        c.kv("ITER", if rans { opt.iterations.min(RANS_MAX_ITER) } else { opt.iterations });
    c.kv("CONV_FIELD", "RMS_DENSITY");
    c.kv("CONV_RESIDUAL_MINVAL", opt.residual_minval());
    c.kv("CONV_STARTITER", "10");
    c.blank();
    c.kv("MESH_FILENAME", &files.mesh);
    c.kv("MESH_FORMAT", "SU2");
    c.kv("RESTART_FILENAME", "restart.dat");
    c.kv("CONV_FILENAME", "history");
    c.kv("SURFACE_FILENAME", "surface_flow");
    c.kv("TABULAR_FORMAT", "CSV");
    c.kv("OUTPUT_FILES", "( RESTART, SURFACE_CSV, SURFACE_PARAVIEW_ASCII )");
    c.kv("HISTORY_OUTPUT", "( ITER, RMS_RES, AERO_COEFF )");
    c.kv("SCREEN_OUTPUT", "( INNER_ITER, RMS_DENSITY, LIFT, DRAG, MOMENT_Y )");
    c.kv("VOLUME_OUTPUT", "( COORDINATES, SOLUTION, PRIMITIVE )");
    c.kv("SCREEN_WRT_FREQ_INNER", "10");
    c.kv("HISTORY_WRT_FREQ_INNER", "1");
    c.kv("OUTPUT_WRT_FREQ", "500");
    c.0.join("\n") + "\n"
}

#[cfg(test)]
mod tests {
    use super::*;
    use aero::atmosphere::AtmosphereModel;

    fn dims() -> RefDims {
        RefDims { length: 0.1, area: std::f64::consts::PI * 0.05 * 0.05 }
    }

    fn atm() -> Atmosphere {
        AtmosphereModel::default().at(0.0)
    }

    fn has(s: &str, line: &str) -> bool {
        s.lines().any(|l| l == line)
    }

    #[test]
    fn euler_half_model_golden_lines() {
        let o = CfdOptions::default();
        let s = su2_config(&o, 2.0, 4.0, &dims(), &atm(), &CaseFiles::default());
        for l in [
            "SOLVER= EULER",
            "MATH_PROBLEM= DIRECT",
            "MACH_NUMBER= 2.000000",
            "AOA= 4.000000",
            "SIDESLIP_ANGLE= 0.0",
            "FREESTREAM_PRESSURE= 101325.0000",
            "FREESTREAM_TEMPERATURE= 288.1500",
            "REF_ORIGIN_MOMENT_X= 0.0",
            "REF_LENGTH= 0.10000000",
            "MARKER_EULER= ( wall, base )",
            "MARKER_MONITORING= ( wall )",
            "MARKER_FAR= ( farfield )",
            "MARKER_SYM= ( symmetry )",
            "CONV_NUM_METHOD_FLOW= ROE",
            "MUSCL_FLOW= YES",
            "SLOPE_LIMITER_FLOW= VENKATAKRISHNAN",
            "LIMITER_ITER= 250",
            "TIME_DISCRE_FLOW= EULER_IMPLICIT",
            "CFL_ADAPT= YES",
            "CFL_NUMBER= 5",
            "ITER= 3000",
            "CONV_FIELD= RMS_DENSITY",
            "CONV_RESIDUAL_MINVAL= -6",
            "MESH_FILENAME= mesh.su2",
            "RESTART_SOL= NO",
            "OUTPUT_FILES= ( RESTART, SURFACE_CSV, SURFACE_PARAVIEW_ASCII )",
            "HISTORY_OUTPUT= ( ITER, RMS_RES, AERO_COEFF )",
            "VOLUME_OUTPUT= ( COORDINATES, SOLUTION, PRIMITIVE )",
        ] {
            assert!(has(&s, l), "missing line {l:?} in\n{s}");
        }
        let area = 0.5 * std::f64::consts::PI * 0.05 * 0.05;
        assert!(has(&s, &format!("REF_AREA= {area:.10}")), "{s}");
        assert!(!s.contains("KIND_TURB_MODEL") && !s.contains("MARKER_HEATFLUX"));
    }

    #[test]
    fn rans_full_model_with_warm_start() {
        let o = CfdOptions { model: FlowModel::Rans, symmetry: false, scheme: Scheme::Jst, ..Default::default() };
        let f = CaseFiles { mesh: "../mesh.su2".into(), restart_from: Some("../m0.600_a02.00/restart.dat".into()) };
        let s = su2_config(&o, 0.6, 8.0, &dims(), &atm(), &f);
        for l in [
            "SOLVER= RANS",
            "KIND_TURB_MODEL= SA",
            "INIT_OPTION= TD_CONDITIONS",
            "FREESTREAM_OPTION= TEMPERATURE_FS",
            "MARKER_HEATFLUX= ( wall, 0.0, base, 0.0 )",
            "MARKER_WALL_FUNCTIONS= ( wall, STANDARD_WALL_FUNCTION, base, STANDARD_WALL_FUNCTION )",
            "MARKER_MONITORING= ( wall, base )",
            "CONV_NUM_METHOD_FLOW= JST",
            "MUSCL_FLOW= NO",
            "RESTART_SOL= YES",
            "SOLUTION_FILENAME= ../m0.600_a02.00/restart.dat",
            "MESH_FILENAME= ../mesh.su2",
        ] {
            assert!(has(&s, l), "missing line {l:?} in\n{s}");
        }
        assert!(!s.contains("MARKER_SYM"));
        assert!(has(&s, &format!("REF_AREA= {:.10}", dims().area)));
    }
}
