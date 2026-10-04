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
//! * Sideslip and roll rate: see [`su2_config_case`].
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

use crate::case::CaseSpec;
use crate::config::{CfdOptions, FlowModel, MultigridCycle, Scheme};
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

/// Text of the SU2 configuration for Mach `mach` and angle of attack `alpha_deg` (no sideslip, no rotation).
pub fn su2_config(opt: &CfdOptions, mach: f64, alpha_deg: f64, dims: &RefDims, atm: &Atmosphere, files: &CaseFiles) -> String {
    su2_config_case(opt, &CaseSpec::new(mach, alpha_deg), dims, atm, files)
}

/// Roll rate [rad/s] of a roll-damping run: `p = 2 V p_hat / d` with `V = M a` at the reference altitude.
pub fn roll_rate(spec: &CaseSpec, dims: &RefDims, atm: &Atmosphere) -> f64 {
    2.0 * spec.mach * atm.sound_speed * spec.roll_hat / dims.length
}

/// Text of the SU2 configuration of one case.
///
/// * `SIDESLIP_ANGLE = -beta`: SU2's freestream direction is `(cos a cos b', sin b', sin a cos b')` with
///   `b'` its sideslip angle, while the body-axis sideslip is `beta = asin(v / V)` with `v` along
///   `y_body = y_cfd`; the two have opposite sign in the CFD frame (verified against the expansion of
///   an axisymmetric table).
/// * Roll-damping runs (`roll_hat != 0`, `alpha = beta = 0`): steady rotating reference frame,
///   `GRID_MOVEMENT = ROTATING_FRAME` about the nose-tip origin with `ROTATION_RATE = (-p, 0, 0)`
///   (the CFD x axis points aft, so a positive body roll rate is a rotation about `-x`). The rate is
///   fixed by `p = 2 V p_hat / d` ([`roll_rate`]), `V` from the Mach number and the reference
///   atmosphere, so that `Clp = dCl / dp_hat` has the normalisation of the flight model.
pub fn su2_config_case(opt: &CfdOptions, spec: &CaseSpec, dims: &RefDims, atm: &Atmosphere, files: &CaseFiles) -> String {
    let (mach, alpha_deg) = (spec.mach, spec.alpha_deg);
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
    if spec.beta_deg == 0.0 {
        c.kv("SIDESLIP_ANGLE", "0.0");
    } else {
        c.kv("SIDESLIP_ANGLE", format!("{:.6}", -spec.beta_deg));
    }
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
    if spec.is_roll() {
        c.blank();
        c.kv("GRID_MOVEMENT", "ROTATING_FRAME");
        c.kv("MOTION_ORIGIN", "0.0 0.0 0.0");
        c.kv("ROTATION_RATE", format!("{:.6} 0.0 0.0", -roll_rate(spec, dims, atm)));
    }
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
    let mg = opt.multigrid_levels();
    if mg > 0 {
        c.kv("MGLEVEL", mg);
        c.kv("MGCYCLE", match opt.multigrid_cycle {
            MultigridCycle::W => "W_CYCLE",
            MultigridCycle::V => "V_CYCLE",
        });
    }
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
    fn multigrid_lines() {
        // Single grid by default: no multigrid lines.
        let d = su2_config(&CfdOptions::default(), 0.8, 2.0, &dims(), &atm(), &CaseFiles::default());
        assert!(!d.contains("MGLEVEL") && !d.contains("MGCYCLE"), "{d}");
        let o = CfdOptions { multigrid: Some(3), multigrid_cycle: crate::config::MultigridCycle::V, ..Default::default() };
        let s = su2_config(&o, 0.8, 2.0, &dims(), &atm(), &CaseFiles::default());
        for l in ["MGLEVEL= 3", "MGCYCLE= V_CYCLE"] {
            assert!(has(&s, l), "missing {l:?} in\n{s}");
        }
        let w = su2_config(&CfdOptions { multigrid: Some(2), ..Default::default() }, 0.8, 2.0, &dims(), &atm(), &CaseFiles::default());
        assert!(has(&w, "MGLEVEL= 2") && has(&w, "MGCYCLE= W_CYCLE"));
        // The rotating frame of a roll case keeps the multigrid lines.
        let r = su2_config_case(&CfdOptions { multigrid: Some(2), symmetry: false, ..Default::default() }, &CaseSpec::roll(0.8, 0.01), &dims(), &atm(), &CaseFiles::default());
        assert!(has(&r, "MGLEVEL= 2") && has(&r, "GRID_MOVEMENT= ROTATING_FRAME"));
        // RANS: single grid unless multigrid is set explicitly.
        let rans = CfdOptions { model: FlowModel::Rans, ..Default::default() };
        assert!(!su2_config(&rans, 0.8, 2.0, &dims(), &atm(), &CaseFiles::default()).contains("MGLEVEL"));
        assert!(su2_config(&CfdOptions { multigrid: Some(1), ..rans }, 0.8, 2.0, &dims(), &atm(), &CaseFiles::default()).contains("MGLEVEL= 1"));
    }

    #[test]
    fn sideslip_and_roll_rate_lines() {
        let o = CfdOptions { symmetry: false, ..Default::default() };
        let spec = CaseSpec { mach: 0.8, alpha_deg: 2.0, beta_deg: 4.0, roll_hat: 0.0 };
        let s = su2_config_case(&o, &spec, &dims(), &atm(), &CaseFiles::default());
        assert!(has(&s, "SIDESLIP_ANGLE= -4.000000") && has(&s, "AOA= 2.000000"), "{s}");
        assert!(!s.contains("ROTATING_FRAME") && !s.contains("MARKER_SYM"));
        assert!(has(&s, &format!("REF_AREA= {:.10}", dims().area)));
        let r = CaseSpec::roll(0.8, 0.01);
        let p = 2.0 * 0.8 * atm().sound_speed * 0.01 / 0.1;
        let s = su2_config_case(&o, &r, &dims(), &atm(), &CaseFiles::default());
        for l in ["GRID_MOVEMENT= ROTATING_FRAME", "MOTION_ORIGIN= 0.0 0.0 0.0", &format!("ROTATION_RATE= {:.6} 0.0 0.0", -p), "SIDESLIP_ANGLE= 0.0", "AOA= 0.000000"] {
            assert!(has(&s, l), "missing {l:?} in\n{s}");
        }
        let n = su2_config_case(&o, &CaseSpec::roll(0.8, -0.01), &dims(), &atm(), &CaseFiles::default());
        assert!(has(&n, &format!("ROTATION_RATE= {:.6} 0.0 0.0", p)));
        // Default cases keep their configuration text (case hashes of existing caches stay valid).
        let d = CfdOptions::default();
        assert_eq!(su2_config(&d, 2.0, 4.0, &dims(), &atm(), &CaseFiles::default()), su2_config_case(&d, &CaseSpec::new(2.0, 4.0), &dims(), &atm(), &CaseFiles::default()));
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
