// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025-2026 西濱大将 (NISHIHAMA Daisuke)
//! 6-DoF flight simulation. The integration frame is selected by the Earth model (see
//! `frame.rs`): a flat local ENU frame (x east, y north, z up) or the rotating ECEF frame.
//! Body frame: x forward along the axis towards the nose, y and z the body axes of the aerodynamic
//! table. Axisymmetric tables use the total angle of attack; general tables the six body-axis
//! coefficients of (alpha, beta). The mass distribution may carry a full inertia tensor, a lateral
//! CG offset and a misaligned thrust line.

use crate::env::{is_pos, AtmosphereModel, AttitudeKind, Earth, IntegratorKind, Wind};
use crate::frame::{Frame, Local};
use crate::integrator::{self, State};
use crate::motor::Motor;
use crate::stepper::Stepper;
use aero::table::{aero_angles, induced_roll, Coeffs6, StaticMargins, TableKind};
use aero::{AeroCoeffs, AeroTable};
use anyhow::{bail, Result};
use geom::{Mat3, Quat, Vec3};
use serde::{Deserialize, Serialize};

/// Dry mass properties in body axes (x forward). The inertia entries are the elements of the
/// tensor `I = [[Ixx, Ixy, Ixz], [Ixy, Iyy, Iyz], [Ixz, Iyz, Izz]]` about the dry CG, so
/// `Ixy = -∫ x y dm` (the classical product of inertia with a minus sign).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
pub struct MassProperties {
    /// Mass without propellant (including the empty motor casing) [kg].
    pub dry_mass: f64,
    /// Dry centre of gravity aft of the nose tip [m].
    pub cg_dry: f64,
    /// Dry roll inertia about the axis [kg m^2].
    pub ixx_dry: f64,
    /// Dry pitch inertia about the dry CG [kg m^2].
    pub iyy_dry: f64,
    /// Dry yaw inertia [kg m^2]; equal to `iyy_dry` when absent.
    pub izz_dry: Option<f64>,
    pub ixy_dry: f64,
    pub ixz_dry: f64,
    pub iyz_dry: f64,
    /// Lateral dry CG offset (y, z) in body axes [m].
    pub cg_lateral_dry: [f64; 2],
}

impl MassProperties {
    /// Axisymmetric mass properties.
    pub fn axisymmetric(dry_mass: f64, cg_dry: f64, ixx_dry: f64, iyy_dry: f64) -> Self {
        Self { dry_mass, cg_dry, ixx_dry, iyy_dry, ..Default::default() }
    }

    /// Dry inertia tensor about the dry CG.
    pub fn dry_tensor(&self) -> Mat3 {
        let izz = self.izz_dry.unwrap_or(self.iyy_dry);
        Mat3([[self.ixx_dry, self.ixy_dry, self.ixz_dry], [self.ixy_dry, self.iyy_dry, self.iyz_dry], [self.ixz_dry, self.iyz_dry, izz]])
    }

    /// True when the rocket is a plain body of revolution (no products, Iyy = Izz, CG on the axis).
    pub fn is_axisymmetric(&self) -> bool {
        self.ixy_dry == 0.0 && self.ixz_dry == 0.0 && self.iyz_dry == 0.0 && self.izz_dry.is_none_or(|z| z == self.iyy_dry) && self.cg_lateral_dry == [0.0, 0.0]
    }
}

/// Mass, CG and inertia at one instant (body axes, tensor about the instantaneous CG).
#[derive(Debug, Clone, Copy)]
pub struct MassState {
    pub m: f64,
    /// CG aft of the nose tip [m].
    pub x_cg: f64,
    /// Lateral CG offset (y, z) in body axes [m].
    pub cg_lat: [f64; 2],
    pub inertia: Mat3,
    /// Inverse of `inertia` (general case only).
    pub inv: Option<Mat3>,
    /// True when the tensor is diag(Ixx, Iyy, Iyy) and the CG is on the axis.
    pub diagonal: bool,
}

/// Thrust line of the motor: a direction error and an offset of the nozzle-exit point.
///
/// The thrust direction in body axes is `(cos e, sin e cos f, sin e sin f)` with `e` the
/// misalignment and `f` its azimuth from +y_b towards +z_b.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct ThrustLine {
    pub misalignment_deg: f64,
    pub misalignment_azimuth_deg: f64,
    /// Nozzle-exit point (y, z) in body axes [m].
    pub offset: [f64; 2],
}

impl ThrustLine {
    pub fn is_aligned(&self) -> bool {
        self.misalignment_deg == 0.0 && self.offset == [0.0, 0.0]
    }
    /// Unit thrust direction in body axes.
    pub fn direction(&self) -> Vec3 {
        let (se, ce) = self.misalignment_deg.to_radians().sin_cos();
        let (sf, cf) = self.misalignment_azimuth_deg.to_radians().sin_cos();
        Vec3::new(ce, se * cf, se * sf)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Launch {
    pub latitude: f64,
    pub longitude: f64,
    /// Launch site altitude above mean sea level [m].
    pub altitude: f64,
    pub rail_length: f64,
    /// Rail elevation above the horizon [deg].
    pub elevation_deg: f64,
    /// Rail azimuth, clockwise from north [deg].
    pub azimuth_deg: f64,
    /// Roll of the body about the rail axis [deg]; 0 puts +y_b horizontal.
    #[serde(default)]
    pub roll_deg: f64,
}

impl Launch {
    /// Rail direction in launch-site ENU (x east, y north, z up).
    pub fn rail_dir_enu(&self) -> Vec3 {
        let (se, ce) = self.elevation_deg.to_radians().sin_cos();
        let (sa, ca) = self.azimuth_deg.to_radians().sin_cos();
        Vec3::new(ce * sa, ce * ca, se)
    }

    /// Body axes (x_b, y_b, z_b) on the rail in launch-site ENU.
    pub fn body_axes_enu(&self) -> [Vec3; 3] {
        self.body_axes(|v| v)
    }

    /// Body axes on the rail, built in the frame that `to_frame` maps launch-site ENU vectors
    /// into: x_b along the rail, y_b = (cos az, -sin az, 0) (horizontal, to the right of the
    /// azimuth) rolled by `roll_deg` about x_b, z_b = x_b cross y_b.
    fn body_axes(&self, to_frame: impl Fn(Vec3) -> Vec3) -> [Vec3; 3] {
        let xb = to_frame(self.rail_dir_enu());
        let az = self.azimuth_deg.to_radians();
        let yb = to_frame(Vec3::new(az.cos(), -az.sin(), 0.0));
        let zb = xb.cross(yb);
        if self.roll_deg == 0.0 {
            return [xb, yb, zb];
        }
        // Roll about the rail axis: y' = cos y + sin z, z' = x cross y'.
        let (sr, cr) = self.roll_deg.to_radians().sin_cos();
        let yr = yb * cr + zb * sr;
        [xb, yr, xb.cross(yr)]
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Recovery {
    pub enabled: bool,
    /// Parachute drag area Cd*S [m^2].
    pub cd_s: f64,
    /// Deployment delay after apogee [s].
    pub delay: f64,
    /// Dual deploy: drag area Cd*S of the main parachute [m^2]. The first parachute (`cd_s`, the
    /// drogue) deploys at apogee + `delay`; when the rocket descends through `main_altitude` the
    /// main replaces it, so the total drag area from then on is `main_cd_s` (the drogue is
    /// released or collapses behind the main; it is not added). Both fields or neither.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub main_cd_s: Option<f64>,
    /// Dual deploy: main deployment altitude above the launch site [m].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub main_altitude: Option<f64>,
}

impl Recovery {
    /// Checks an enabled recovery: drag area positive and finite, delay non-negative and finite,
    /// the dual-deploy fields both or neither, positive and finite. A disabled recovery is not
    /// flown and passes.
    pub fn validate(&self) -> Result<()> {
        if !self.enabled {
            return Ok(());
        }
        if !(self.cd_s.is_finite() && self.cd_s > 0.0) {
            bail!("recovery.cd_s must be positive and finite when recovery.enabled = true, got {}", self.cd_s);
        }
        if !(self.delay.is_finite() && self.delay >= 0.0) {
            bail!("recovery.delay must be non-negative and finite, got {}", self.delay);
        }
        match (self.main_cd_s, self.main_altitude) {
            (None, None) => Ok(()),
            (Some(c), Some(a)) => {
                if !(c.is_finite() && c > 0.0) {
                    bail!("recovery.main_cd_s must be positive and finite, got {c}");
                }
                if !(a.is_finite() && a > 0.0) {
                    bail!("recovery.main_altitude must be positive and finite, got {a}");
                }
                Ok(())
            }
            _ => bail!("recovery.main_cd_s and recovery.main_altitude must be given together (dual deploy) or both omitted"),
        }
    }

    /// The main parachute (Cd*S, altitude), when configured.
    pub fn main(&self) -> Option<(f64, f64)> {
        self.main_cd_s.zip(self.main_altitude)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Settings {
    pub dt: f64,
    pub max_time: f64,
    pub output_interval: f64,
    pub integrator: IntegratorKind,
    /// Attitude update: quaternion renormalisation or Lie-group (RKMK) steps.
    #[serde(default)]
    pub attitude: AttitudeKind,
    /// Relative and absolute tolerances of the adaptive integrator.
    pub rtol: f64,
    pub atol: f64,
}

impl Default for Settings {
    fn default() -> Self {
        Self { dt: 0.002, max_time: 3600.0, output_interval: 0.05, integrator: IntegratorKind::Rk4, attitude: AttitudeKind::Normalize, rtol: 1e-7, atol: 1e-6 }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Descent {
    Ballistic,
    Parachute,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Phase {
    Rail,
    Free,
    Parachute,
}

/// Multipliers applied to the aerodynamic table after lookup (dispersion studies); 1 = nominal.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AeroScale {
    /// Scales `cn`, `cna` and the pitch-damping sums (all proportional to CN_alpha).
    pub cn: f64,
    /// Scales `ca_on` and `ca_off`.
    pub ca: f64,
    /// Scales the roll-moment coefficient, its cant slope `cl_delta` and the induced roll `cl4`.
    pub cl: f64,
    /// Scales the roll-damping derivative `clp`.
    pub clp: f64,
    /// Scales the lag (apparent-mass) sums `lag_l0`, `lag_l1`; 0 switches the lag moment off.
    pub lag: f64,
    /// Scales the Magnus slope `cnpa`.
    pub magnus: f64,
    /// Scales the damping sums on top of `cn` (pitch-damping moment and the rotation-induced force).
    pub damp: f64,
    /// Scales the rotation-induced normal force of the damping sums (the CN_q force); 0 leaves
    /// the damping moment alone (as a moment-only damping model).
    pub cnq: f64,
}

impl Default for AeroScale {
    fn default() -> Self {
        Self { cn: 1.0, ca: 1.0, cl: 1.0, clp: 1.0, lag: 1.0, magnus: 1.0, damp: 1.0, cnq: 1.0 }
    }
}

impl AeroScale {
    /// The quasi-steady model only: no lag moment, no Magnus force or moment, no rotation-induced
    /// force (the induced roll `cl4` stays, it is a static roll coefficient).
    pub fn quasi_steady() -> Self {
        Self { lag: 0.0, magnus: 0.0, cnq: 0.0, ..Self::default() }
    }
}

/// Inertia tensor of a point mass `m` at offset `d` from the reference point: m (|d|^2 E - d d^T).
pub fn parallel_axis(m: f64, d: Vec3) -> Mat3 {
    let d2 = d.dot(d);
    let v = [d.x, d.y, d.z];
    let mut r = [[0.0; 3]; 3];
    for (i, row) in r.iter_mut().enumerate() {
        for (j, e) in row.iter_mut().enumerate() {
            *e = m * (if i == j { d2 } else { 0.0 } - v[i] * v[j]);
        }
    }
    Mat3(r)
}

/// Largest accepted ratio of the largest to the smallest principal moment of inertia.
pub const MAX_INERTIA_CONDITION: f64 = 1e8;

#[derive(Clone)]
pub struct Simulation<'a> {
    pub table: &'a AeroTable,
    pub mass: MassProperties,
    pub motor: &'a Motor,
    /// Aft end of the motor, aft of the nose tip [m].
    pub motor_aft_x: f64,
    pub launch: Launch,
    pub recovery: Recovery,
    pub wind: Wind,
    pub atmosphere: AtmosphereModel,
    pub earth: Earth,
    pub settings: Settings,
    /// Multipliers on the table coefficients (default 1).
    pub aero_scale: AeroScale,
    /// Thrust misalignment and nozzle offset.
    pub thrust_line: ThrustLine,
    /// Fin cant [deg]; the roll forcing is `cl_delta * (cant - table.fin_cant_ref_deg)` on top of the table.
    pub fin_cant_deg: f64,
}

#[derive(Debug, Clone, Copy, Serialize)]
pub struct Sample {
    pub t: f64,
    pub phase: Phase,
    pub east: f64,
    pub north: f64,
    pub up: f64,
    pub lat: f64,
    pub lon: f64,
    pub alt: f64,
    pub vel_e: f64,
    pub vel_n: f64,
    pub vel_u: f64,
    pub airspeed: f64,
    pub mach: f64,
    pub alpha_deg: f64,
    pub thrust: f64,
    pub mass: f64,
    pub x_cg: f64,
    /// Centre of pressure [m aft of the nose] at the state's current angle of attack (general table:
    /// of the transverse force at the current angles).
    pub x_cp: f64,
    /// (x_cp - x_cg) / d [cal] at the state's current angle of attack: a display of where the
    /// normal force acts at that angle, not the static margin. The stability verdict uses the
    /// small-angle margin of [`Summary::min_stability_cal`].
    pub stability_cal: f64,
    pub dyn_pressure: f64,
    pub cn: f64,
    pub ca: f64,
    pub pitch_deg: f64,
    pub heading_deg: f64,
    pub wx: f64,
    pub wy: f64,
    pub wz: f64,
    /// Sideslip asin(v / V) in body axes [deg].
    pub beta_deg: f64,
    /// Body roll rate [rad/s] (relative to the integration frame).
    pub roll_rate: f64,
    /// Total roll-moment coefficient applied (forcing and damping).
    pub cl: f64,
    /// Side-force coefficient (body y).
    pub cy: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Summary {
    pub descent: Descent,
    pub wind_speed: f64,
    pub wind_direction_deg: f64,
    pub rail_exit_time: f64,
    pub rail_exit_speed: f64,
    /// End of the thrust curve [s]; NaN (`null` in JSON) when the flight ended before it.
    pub burnout_time: f64,
    pub max_speed: f64,
    pub max_mach: f64,
    pub max_dyn_pressure: f64,
    pub apogee_time: f64,
    /// Apogee above the launch site [m].
    pub apogee: f64,
    pub apogee_east: f64,
    pub apogee_north: f64,
    /// Smallest static margin [cal], sampled once per integration step only while the rocket is in
    /// free flight (from the rail-exit state on) before apogee and its airspeed exceeds 10 m/s. The
    /// rail-exit state counts when its airspeed exceeds 10 m/s; the slow part near apogee and the
    /// descent (tumbling, parachute) are not included. NaN (`null` in JSON) when no state qualified,
    /// e.g. a flight whose airspeed never exceeds 10 m/s before apogee; `stability_at_rail_exit` is
    /// evaluated regardless of the airspeed, so [`static_margin_min`](Self::static_margin_min) still
    /// has a value then.
    ///
    /// Every margin of the summary is the small-angle (alpha -> 0) static margin of
    /// [`AeroTable::static_margins`] at the state's Mach number and CG, for both table kinds:
    /// axisymmetric table (x_cp - x_cg) / d with x_cp the centre of pressure the interpolated table
    /// applies at small angles (that of the first nonzero angle row), general table the smaller of
    /// the pitch and yaw margins from the slopes at zero angle. The angle of attack the state flies at does not
    /// enter, so the slow high-angle arc over apogee and the rail exit in a strong crosswind do not
    /// count as instability; the margin at the current angle is only the `stability_cal` column of
    /// the trajectory ([`Sample::stability_cal`]).
    pub min_stability_cal: f64,
    /// Small-angle static margin at the rail-exit state [cal] (general table: the smaller of pitch
    /// and yaw), evaluated at any airspeed.
    pub stability_at_rail_exit: f64,
    /// Smallest pitch-plane static margin [cal] over the steps of `min_stability_cal` and the rail
    /// exit. Axisymmetric table: the smaller of `min_stability_cal` and `stability_at_rail_exit`.
    /// NaN (`null` in JSON) when it could not be evaluated, see `stability_unknown`.
    #[serde(default = "nan")]
    pub static_margin_pitch_min_cal: f64,
    /// Smallest yaw-plane static margin [cal], as `static_margin_pitch_min_cal`.
    #[serde(default = "nan")]
    pub static_margin_yaw_min_cal: f64,
    /// Time [s] of the state with the smallest static margin
    /// ([`static_margin_min`](Self::static_margin_min)); NaN (`null` in JSON) when no margin was
    /// evaluated.
    #[serde(default = "nan")]
    pub static_margin_min_t: f64,
    /// Mach number of the state with the smallest static margin; NaN as `static_margin_min_t`.
    #[serde(default = "nan")]
    pub static_margin_min_mach: f64,
    /// Why the static stability could not be evaluated (the first step where a margin was
    /// missing); None when every margin was evaluated.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stability_unknown: Option<String>,
    /// Ballistic descent: time [s] after apogee the rocket flew with an angle of attack between 60
    /// and 120 deg (broadside to the flow). A long broadside fall is a trim at large angles, whose
    /// aerodynamics (axisymmetric table: the nose-first coefficients mirrored at 180 deg - alpha)
    /// are not validated; see [`fell_broadside`](Self::fell_broadside). 0 for the parachute descent.
    #[serde(default)]
    pub broadside_time: f64,
    pub landing_time: f64,
    pub landing_east: f64,
    pub landing_north: f64,
    pub landing_lat: f64,
    pub landing_lon: f64,
    pub landing_distance: f64,
    pub landing_speed: f64,
    /// Dual deploy: time of the main parachute deployment [s] (None without a main parachute or when the flight ended before).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub main_deploy_time: Option<f64>,
    /// Dual deploy: altitude of the main deployment above the launch site [m].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub main_deploy_altitude: Option<f64>,
    /// Time the (first) parachute deployed [s]; None for ballistic runs and when the flight
    /// landed (or ended) before apogee + `recovery.delay`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub drogue_deploy_time: Option<f64>,
    /// Parachute descent whose deployment instant (apogee + `recovery.delay`) came while the motor
    /// still burned: the deployment waited for burnout, the end of the thrust (`drogue_deploy_time`).
    /// None when the deployment was not delayed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deploy_delayed: Option<DeployDelay>,
    /// Airspeed [m/s] at the (first) parachute deployment.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub drogue_deploy_speed: Option<f64>,
    /// Dynamic pressure [Pa] at the (first) parachute deployment.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub drogue_deploy_q: Option<f64>,
    /// Dual deploy: airspeed [m/s] at the main deployment.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub main_deploy_speed: Option<f64>,
    /// Dual deploy: dynamic pressure [Pa] at the main deployment.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub main_deploy_q: Option<f64>,
    /// Wind profile model: factor the tabulated profile was scaled by in this run (1 = as in the
    /// CSV; dispersion cases and Monte Carlo samples scale it to their mean speed). None for the
    /// other wind models.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wind_scale: Option<f64>,
    /// Roll-pitch resonance: the first instant of the ascent at which the roll rate crossed the pitch
    /// natural frequency while a trim forcing (fin cant, lateral CG offset or thrust misalignment)
    /// was present; None when it did not happen (or there is no trim forcing). See [`RollResonance`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub roll_resonance: Option<RollResonance>,
}

/// Crossing of the roll rate `|p|` and the pitch natural frequency
/// `omega_n = sqrt(max(0, -Cm_alpha) q S d / I)` (`-Cm_alpha` the restoring derivative about the
/// CG, I the mean transverse inertia) in free flight before apogee above 10 m/s. With a trim
/// forcing fixed in the body, the forcing rolls with the body at `p`; at `p = omega_n` it drives
/// the pitch/yaw oscillation at its natural frequency (roll resonance; lock-in and catastrophic yaw
/// when the roll rate stays there). Sampled once per integration step; the crossing time and
/// Mach number are interpolated linearly between the two steps.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct RollResonance {
    /// Time of the first crossing [s].
    pub t: f64,
    /// Mach number at the first crossing.
    pub mach: f64,
    /// Roll rate |p| = pitch natural frequency at the first crossing [rad/s].
    pub frequency: f64,
    /// Number of crossings during the ascent.
    pub crossings: u32,
}

impl Summary {
    /// Summary of a run that produced no result (an errored case): every number is NaN.
    pub fn failed(descent: Descent, wind_speed: f64, wind_direction_deg: f64) -> Self {
        let n = f64::NAN;
        Self {
            descent,
            wind_speed,
            wind_direction_deg,
            rail_exit_time: n,
            rail_exit_speed: n,
            burnout_time: n,
            max_speed: n,
            max_mach: n,
            max_dyn_pressure: n,
            apogee_time: n,
            apogee: n,
            apogee_east: n,
            apogee_north: n,
            min_stability_cal: n,
            stability_at_rail_exit: n,
            static_margin_pitch_min_cal: n,
            static_margin_yaw_min_cal: n,
            static_margin_min_t: n,
            static_margin_min_mach: n,
            stability_unknown: None,
            broadside_time: n,
            landing_time: n,
            landing_east: n,
            landing_north: n,
            landing_lat: n,
            landing_lon: n,
            landing_distance: n,
            landing_speed: n,
            main_deploy_time: None,
            main_deploy_altitude: None,
            drogue_deploy_time: None,
            deploy_delayed: None,
            drogue_deploy_speed: None,
            drogue_deploy_q: None,
            main_deploy_speed: None,
            main_deploy_q: None,
            wind_scale: None,
            roll_resonance: None,
        }
    }

    /// Whether the flight reached the ground before `max_time`; when false every `landing_*` field is NaN.
    pub fn landed(&self) -> bool {
        self.landing_time.is_finite() && self.landing_east.is_finite() && self.landing_north.is_finite()
    }

    /// Smallest static margin of the flight including the rail exit [cal]: the smaller of
    /// `min_stability_cal` and `stability_at_rail_exit`, the rail-exit value alone when
    /// `min_stability_cal` is NaN (airspeed never above 10 m/s); NaN when neither is finite.
    pub fn static_margin_min(&self) -> f64 {
        [self.min_stability_cal, self.stability_at_rail_exit].into_iter().filter(|m| m.is_finite()).min_by(f64::total_cmp).unwrap_or(f64::NAN)
    }

    /// Whether the ballistic descent fell broadside (angle of attack 60 to 120 deg) for more than
    /// [`BROADSIDE_LIMIT`] seconds, see `broadside_time`.
    pub fn fell_broadside(&self) -> bool {
        self.broadside_time > BROADSIDE_LIMIT
    }

    /// Static-stability verdict: unstable when an evaluated margin is negative, unknown when a margin
    /// could not be evaluated (or none was), stable otherwise.
    pub fn stability(&self) -> Stability {
        let m = self.static_margin_min();
        if m < 0.0 {
            Stability::Unstable(m)
        } else if let Some(why) = &self.stability_unknown {
            Stability::Unknown(why.clone())
        } else if m.is_nan() {
            Stability::Unknown("no static margin was evaluated (no rail exit and no free flight above 10 m/s before apogee)".into())
        } else {
            Stability::Stable
        }
    }
}

/// Parachute deployment moved from apogee + `recovery.delay` to burnout, see [`Summary::deploy_delayed`].
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct DeployDelay {
    /// Apogee + `recovery.delay` [s], when the parachute would have deployed.
    pub nominal_time: f64,
    /// Thrust [N] at `nominal_time`.
    pub thrust: f64,
}

/// Static-stability verdict of a flight, see [`Summary::stability`].
#[derive(Debug, Clone, PartialEq)]
pub enum Stability {
    Stable,
    /// The smallest static margin [cal], negative.
    Unstable(f64),
    /// Why the static stability could not be evaluated.
    Unknown(String),
}

impl Stability {
    /// `Some(true)` stable, `Some(false)` unstable, None unknown.
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Stability::Stable => Some(true),
            Stability::Unstable(_) => Some(false),
            Stability::Unknown(_) => None,
        }
    }
}

/// Broadside time [s] above which a ballistic descent counts as fallen broadside, see
/// [`Summary::fell_broadside`].
pub const BROADSIDE_LIMIT: f64 = 10.0;

/// Angle-of-attack band [rad] of a broadside fall (60 to 120 deg).
const BROADSIDE_ALPHA: std::ops::RangeInclusive<f64> = (std::f64::consts::PI / 3.0)..=(2.0 * std::f64::consts::PI / 3.0);

fn nan() -> f64 {
    f64::NAN
}

pub struct SimResult {
    pub samples: Vec<Sample>,
    pub summary: Summary,
    /// Number of right-hand-side evaluations (cost of the integration).
    pub evals: u64,
    /// Largest deviation of the attitude quaternion norm from 1 over all accepted steps
    /// (measured on the state as stored, i.e. after any renormalisation the method applies).
    pub max_quat_norm_error: f64,
}

struct Aux {
    thrust: f64,
    mass: f64,
    x_cg: f64,
    x_cp: f64,
    airspeed: f64,
    mach: f64,
    alpha: f64,
    qdyn: f64,
    cn: f64,
    ca: f64,
    beta: f64,
    cl: f64,
    cy: f64,
}

/// Per-run constants of the dynamics: integration frame and rail direction in that frame.
struct Ctx {
    frame: Frame,
    rail: Vec3,
    /// Drag area Cd*S of the canopy currently deployed (drogue, then main) [m^2].
    cd_s: std::cell::Cell<f64>,
    /// Thrust-curve segment of the current step ([`Motor::segment_after`] at the step start;
    /// steps end at knots). The thrust is its linear piece throughout the step, and the
    /// power-on/off axial coefficient does not switch inside a step, so no stage of a step that
    /// ends at a knot (burnout) sees the next segment.
    segment: std::cell::Cell<Option<usize>>,
    /// No gravity (the free-oscillation test harness: constant speed and density in level flight).
    gravity_off: bool,
}

impl<'a> Simulation<'a> {
    /// Body-to-frame attitude on the rail.
    fn initial_attitude(&self, fr: &Frame) -> Quat {
        let [xb, yb, zb] = self.launch.body_axes(|v| fr.from_launch_enu(v));
        Quat::from_mat(&Mat3::from_cols(xb, yb, zb))
    }

    /// Table lookup with the dispersion multipliers applied.
    fn lookup(&self, mach: f64, alpha: f64) -> AeroCoeffs {
        let mut c = self.table.lookup(mach, alpha);
        let AeroScale { cn, ca, cl, clp, lag, magnus, damp, cnq: _ } = self.aero_scale;
        c.cn *= cn;
        c.cna *= cn;
        c.damp = c.damp.map(|d| d * cn * damp);
        c.ca_on *= ca;
        c.ca_off *= ca;
        c.cl_delta *= cl;
        c.cl4 *= cl;
        c.clp *= clp;
        c.lag_l0 *= lag;
        c.lag_l1 *= lag;
        c.cnpa *= magnus;
        c
    }

    /// General-table lookup with the dispersion multipliers applied (`cn` scales every normal-force
    /// and moment coefficient and the damping sums, `ca` the axial force).
    fn lookup6(&self, mach: f64, alpha: f64, beta: f64) -> Coeffs6 {
        let mut c = self.table.lookup6(mach, alpha, beta);
        let AeroScale { cn, ca, cl, clp, lag, magnus, damp, cnq: _ } = self.aero_scale;
        c.cl *= cl;
        c.cl_delta *= cl;
        c.clp *= clp;
        c.cy *= cn;
        c.cz *= cn;
        c.cm *= cn;
        c.cn *= cn;
        c.sq = c.sq.map(|d| d * cn * damp);
        c.sr = c.sr.map(|d| d * cn * damp);
        c.cx_on *= ca;
        c.cx_off *= ca;
        c.lag_l0 *= lag;
        c.lag_l1 *= lag;
        c.cnpa *= magnus;
        c
    }

    /// Effective centre of pressure [m aft of the nose] of the transverse force of a general table:
    /// from Cm = x_cp Cz / d and Cn = -x_cp Cy / d. Near zero transverse force the value at 0.5 deg
    /// angle of attack is used.
    fn eff_xcp(&self, c: &Coeffs6, mach: f64) -> Option<f64> {
        let d = self.table.meta.ref_diameter;
        let of = |c: &Coeffs6| {
            let t2 = c.cy * c.cy + c.cz * c.cz;
            (t2 > 1e-12).then(|| d * (c.cm * c.cz - c.cn * c.cy) / t2)
        };
        of(c).or_else(|| of(&self.lookup6(mach, 0.5f64.to_radians(), 0.0)))
    }

    /// Pitch and yaw static margins [cal] at a free-flight state (`aux` from [`deriv`](Self::deriv)):
    /// the small-angle margins of [`AeroTable::static_margins`] at the state's Mach number and CG,
    /// for both table kinds. The state's own angle of attack does not enter (see
    /// [`Summary::min_stability_cal`]).
    fn margins(&self, aux: &Aux) -> StaticMargins {
        self.table.static_margins(aux.mach, aux.x_cg)
    }

    /// Pitch natural frequency [rad/s] at a free-flight state (`aux` from [`deriv`](Self::deriv) at
    /// time `t`): `sqrt(max(0, R) q S d / I)` with `R = -dCm_cg/dalpha` the restoring derivative of the
    /// table at the state's Mach number and CG (the smaller of pitch and yaw, scaled as the normal
    /// force) and I the mean of the transverse moments of inertia. NaN when R is unknown.
    fn pitch_frequency(&self, t: f64, aux: &Aux) -> f64 {
        let ms = self.mass_at(t);
        let it = 0.5 * (ms.inertia.0[1][1] + ms.inertia.0[2][2]);
        let r = self.table.restoring_derivative(aux.mach, aux.x_cg) * self.aero_scale.cn;
        (r.max(0.0) * aux.qdyn * self.table.meta.ref_area * self.table.meta.ref_diameter / it).sqrt()
    }

    /// Mass, CG and inertia at time t.
    fn mass_at(&self, t: f64) -> MassState {
        let m = &self.mass;
        let mp = self.motor.propellant_left(t);
        let total = m.dry_mass + mp;
        let xp = self.motor_aft_x - 0.5 * self.motor.length;
        let xcg = (m.dry_mass * m.cg_dry + mp * xp) / total;
        let r = 0.5 * self.motor.diameter;
        if m.is_axisymmetric() {
            let iyy = m.iyy_dry + m.dry_mass * (m.cg_dry - xcg).powi(2) + mp * (3.0 * r * r + self.motor.length.powi(2)) / 12.0 + mp * (xp - xcg).powi(2);
            let ixx = m.ixx_dry + 0.5 * mp * r * r;
            return MassState { m: total, x_cg: xcg, cg_lat: [0.0, 0.0], inertia: Mat3::diag(ixx, iyy, iyy), inv: None, diagonal: true };
        }
        // Body coordinates of the parts (x forward): dry CG (-cg_dry, ly, lz), propellant (-xp, 0, 0).
        let lat = [m.dry_mass * m.cg_lateral_dry[0] / total, m.dry_mass * m.cg_lateral_dry[1] / total];
        let d_dry = Vec3::new(xcg - m.cg_dry, m.cg_lateral_dry[0] - lat[0], m.cg_lateral_dry[1] - lat[1]);
        let d_p = Vec3::new(xcg - xp, -lat[0], -lat[1]);
        let iyy_p = mp * (3.0 * r * r + self.motor.length.powi(2)) / 12.0;
        let prop = Mat3::diag(0.5 * mp * r * r, iyy_p, iyy_p);
        let inertia = m.dry_tensor().add(&parallel_axis(m.dry_mass, d_dry)).add(&prop).add(&parallel_axis(mp, d_p));
        MassState { m: total, x_cg: xcg, cg_lat: lat, inertia, inv: inertia.inverse(), diagonal: false }
    }

    /// Checks that the inertia tensor about the CG is positive definite with a ratio of the
    /// largest to the smallest principal moment of at most [`MAX_INERTIA_CONDITION`], dry and
    /// with propellant at the thrust-curve knots and on a uniform grid over the burn.
    fn check_inertia(&self) -> Result<()> {
        let burn = self.motor.burn_time();
        let mut times: Vec<f64> = (0..=32).map(|i| burn * f64::from(i) / 32.0).collect();
        let mut t = 0.0;
        while let Some(k) = self.motor.next_knot(t) {
            if k <= t {
                break;
            }
            times.push(k);
            t = k;
        }
        for t in times {
            let ms = self.mass_at(t);
            let (ev, _) = geom::math::sym_eigen(ms.inertia.0);
            let (hi, lo) = (ev[0], ev[2]);
            if !(ev.iter().all(|e| e.is_finite()) && lo > 0.0 && hi <= lo * MAX_INERTIA_CONDITION) {
                bail!(
                    "inertia tensor is singular or ill-conditioned at t = {t:.3} s (principal moments {:.6e}, {:.6e}, {:.6e} kg m^2; the largest may be at most {MAX_INERTIA_CONDITION:.0e} times the smallest): check rocket.ixx_dry, iyy_dry, izz_dry and the products of inertia",
                    ev[0],
                    ev[1],
                    ev[2]
                );
            }
        }
        Ok(())
    }

    /// Airspeed [m/s] (relative to the wind) and dynamic pressure [Pa] at state `s` (`loc` its local position).
    fn airspeed_q(&self, s: &State, loc: &Local) -> (f64, f64) {
        let v = (s.vel - loc.to_frame(self.wind.at(loc.height))).norm();
        (v, 0.5 * self.atmosphere.at(self.launch.altitude + loc.height).density * v * v)
    }

    /// State derivative. The frame supplies gravity, the apparent (Coriolis/centrifugal)
    /// accelerations, the local vertical for height and the ENU basis for the wind.
    fn deriv(&self, cx: &Ctx, t: f64, s: &State, phase: Phase) -> (State, Aux) {
        let seg = cx.segment.get();
        let thrust = self.motor.thrust_on_segment(t, seg);
        let powered = self.motor.segment_powered(seg);
        let ms = self.mass_at(t);
        let (mass, x_cg) = (ms.m, ms.x_cg);
        let fr = &cx.frame;
        let loc = fr.local(s.pos);
        let atm = self.atmosphere.at(self.launch.altitude + loc.height);
        let g = if cx.gravity_off { Vec3::ZERO } else { fr.gravity(s.pos, &loc, &self.earth) + fr.apparent_accel(s.pos, s.vel) };
        let zero = State::ZERO_DERIV;
        let s_ref = self.table.meta.ref_area;
        let mut aux = Aux { thrust, mass, x_cg, x_cp: 0.0, airspeed: 0.0, mach: 0.0, alpha: 0.0, qdyn: 0.0, cn: 0.0, ca: 0.0, beta: 0.0, cl: 0.0, cy: 0.0 };

        match phase {
            Phase::Rail => {
                let d = cx.rail;
                let v = s.vel.dot(d);
                let mach = v.abs() / atm.sound_speed;
                let (ca, x_cp) = if self.table.kind() == TableKind::General {
                    let c = self.lookup6(mach, 0.0, 0.0);
                    (-(if powered { c.cx_on } else { c.cx_off }), self.eff_xcp(&c, mach).unwrap_or(f64::NAN))
                } else {
                    let c = self.lookup(mach, 0.0);
                    (if powered { c.ca_on } else { c.ca_off }, c.xcp)
                };
                let qd = 0.5 * atm.density * v * v;
                let mut a = (thrust * self.thrust_line.direction().x - ca * qd * s_ref * v.signum()) / mass + g.dot(d);
                if v <= 0.0 && a < 0.0 {
                    a = 0.0;
                }
                aux.airspeed = v;
                aux.mach = mach;
                aux.qdyn = qd;
                aux.ca = ca;
                aux.x_cp = x_cp;
                (State { pos: s.vel, vel: d * a, ..zero }, aux)
            }
            Phase::Free => {
                let rot = s.q.to_mat();
                // ECEF: the wind (east, north) is taken in the local ENU basis at the rocket, not the launch-site
                // basis, so a wind "from the west" stays westerly along the track; the two differ by a rotation of
                // about downrange distance / R_earth (0.5 deg at 55 km), negligible against wind uncertainty.
                let va = s.vel - loc.to_frame(self.wind.at(loc.height));
                let ub = rot.transpose_mul_vec(va);
                let v = ub.norm();
                let mut f_body = Vec3::new(thrust, 0.0, 0.0);
                let mut m_body = Vec3::ZERO;
                let tl = &self.thrust_line;
                if !(tl.is_aligned() && ms.cg_lat == [0.0, 0.0]) {
                    // Thrust along the misaligned line through the nozzle-exit point; its moment about the CG.
                    let ft = tl.direction() * thrust;
                    let r_n = Vec3::new(-self.motor_aft_x, tl.offset[0], tl.offset[1]);
                    let r_cg = Vec3::new(-x_cg, ms.cg_lat[0], ms.cg_lat[1]);
                    f_body = ft;
                    m_body += (r_n - r_cg).cross(ft);
                }
                // Lag (apparent-mass) sums and fade factor, applied once the acceleration is known.
                let mut lag: Option<(f64, f64, f64)> = None;
                if v > 1e-6 {
                    let lat = ub.y.hypot(ub.z);
                    let alpha = lat.atan2(ub.x);
                    let beta = (ub.y / v).clamp(-1.0, 1.0).asin();
                    let d_ref = self.table.meta.ref_diameter;
                    let d_cant = (self.fin_cant_deg - self.table.meta.fin_cant_ref_deg).to_radians();
                    // Cant forcing follows the flow direction: in tail-first flow (ub.x < 0) the fins
                    // see the flow from behind and the whole cant roll moment (the reference cant
                    // contained in the table and the perturbation `d_cant`) reverses sign.
                    let sgn = if ub.x >= 0.0 { 1.0 } else { -1.0 };
                    let p_hat = s.w.x * d_ref / (2.0 * v);
                    // Effective (nose-first) angle of attack; tail-first flight mirrors it.
                    let alpha_eff = alpha.min(std::f64::consts::PI - alpha);
                    // e_x x w = (0, -r, q): a body station `l` aft of the CG moves with the lateral
                    // velocity `l (e_x x w)` relative to the air through the rotation alone.
                    let ex_w = Vec3::new(0.0, -s.w.z, s.w.y);
                    let cnq = self.aero_scale.cnq;
                    // Magnus force direction u_b x e_x = (0, w, -v): 90 deg from the crossflow (the static
                    // normal force acts along -(0, v, w)), length V sin(alpha).
                    let magnus_dir = Vec3::new(0.0, ub.z, -ub.y);
                    if self.table.kind() == TableKind::General {
                        // Six body-axis coefficients of (alpha, beta); no tail-first mirroring.
                        let (alpha_g, beta_g) = aero_angles(ub.x, ub.y, ub.z);
                        let mach = v / atm.sound_speed;
                        let c = self.lookup6(mach, alpha_g, beta_g);
                        let qd = 0.5 * atm.density * v * v;
                        let qs = qd * s_ref;
                        let cx = if powered { c.cx_on } else { c.cx_off };
                        let f_aero = Vec3::new(cx, c.cy, c.cz) * qs;
                        f_body += f_aero;
                        // The tabulated `cl` already holds the forcing at the reference cant for the
                        // given body-axis flow direction; the cant perturbation follows the same
                        // flow-direction convention as in the axisymmetric branch.
                        let cl = c.cl + c.cl_delta * d_cant * sgn + c.clp * p_hat;
                        // Table moments are about the nose tip; transfer to the CG at (-x_cg, y, z).
                        m_body += Vec3::new(cl, c.cm, c.cn) * (qs * d_ref) + Vec3::new(x_cg, -ms.cg_lat[0], -ms.cg_lat[1]).cross(f_aero);
                        // Rate damping about the CG from the sums about the nose; the lateral CG
                        // offset is neglected here.
                        let kq = (c.sq[2] - 2.0 * x_cg * c.sq[1] + x_cg * x_cg * c.sq[0]).max(0.0);
                        let kr = (c.sr[2] - 2.0 * x_cg * c.sr[1] + x_cg * x_cg * c.sr[0]).max(0.0);
                        m_body += Vec3::new(0.0, -s.w.y * kq, -s.w.z * kr) * (qs / v);
                        if cnq != 0.0 {
                            // Rotation-induced normal force (CN_q) of the same strips as the damping
                            // moment, pitch sums for the pitch rate (z force), yaw sums for the yaw rate
                            // (y force); see the axisymmetric branch for the derivation. Its moment about
                            // the CG is the damping moment above, so it is not transferred again.
                            let f_q = Vec3::new(0.0, (c.sr[1] - x_cg * c.sr[0]) * ex_w.y, (c.sq[1] - x_cg * c.sq[0]) * ex_w.z) * (-cnq * qs / v);
                            f_body += f_q;
                        }
                        // Effective centre of pressure of the transverse force at the current angles (the
                        // trajectory's `stability_cal` display; the summary margins use `margins`).
                        let x_cp = self.eff_xcp(&c, mach).unwrap_or(f64::NAN);
                        if c.cnpa != 0.0 && lat > 1e-12 {
                            // Magnus force C_N = cnpa p_hat sin(alpha) along u_b x e_x, at the centre of
                            // pressure of the static transverse force.
                            let f_m = magnus_dir * (qs * c.cnpa * p_hat / v);
                            f_body += f_m;
                            if x_cp.is_finite() {
                                m_body += Vec3::new(x_cg - x_cp, 0.0, 0.0).cross(f_m);
                            }
                        }
                        if c.lag_l0 != 0.0 || c.lag_l1 != 0.0 {
                            lag = Some((c.lag_l0, c.lag_l1, lag_fade(alpha_eff)));
                        }
                        aux = Aux { airspeed: v, mach, alpha, qdyn: qd, cn: c.cy.hypot(c.cz), ca: -cx, x_cp, beta, cl, cy: c.cy, ..aux };
                    } else {
                        let mach = v / atm.sound_speed;
                        let c = self.lookup(mach, alpha_eff);
                        let qd = 0.5 * atm.density * v * v;
                        let ca = if powered { c.ca_on } else { c.ca_off };
                        let axial = Vec3::new(-ca * qd * s_ref * ub.x.signum(), 0.0, 0.0);
                        f_body += axial;
                        let mut f_aero = axial;
                        if lat > 1e-12 {
                            let fn_b = Vec3::new(0.0, -ub.y / lat, -ub.z / lat) * (c.cn * qd * s_ref);
                            f_body += fn_b;
                            f_aero += fn_b;
                            m_body += Vec3::new(x_cg - c.xcp, 0.0, 0.0).cross(fn_b);
                        }
                        let [s0, s1, s2] = c.damp;
                        let k = (s2 - 2.0 * x_cg * s1 + x_cg * x_cg * s0).max(0.0);
                        m_body += Vec3::new(0.0, s.w.y, s.w.z) * (-qd * s_ref * k / v);
                        if cnq != 0.0 {
                            // Rotation-induced normal force (CN_q, Etkin's Z_q). A strip with slope
                            // CNa_i at x_i aft of the nose lies at r_i = -l_i e_x from the CG,
                            // l_i = x_i - x_cg; the rotation moves it with w x r_i = l_i (e_x x w), which adds
                            // to its lateral velocity relative to the air, and its normal force opposes that
                            // crossflow as the static force does: dF_i = -CNa_i (q S / V) l_i (e_x x w). The
                            // sum over the strips,
                            // dF = -(q S / V) (S1 - x_cg S0) (e_x x w), is the force whose moment about the
                            // CG, sum(l_i e_x x dF_i) = -(q S / V) sum(CNa_i l_i^2) (0, q, r), is the damping
                            // moment above. Pitch rate q > 0 with the fins aft (S1 > x_cg S0) gives a force
                            // along -z_b: the tail moves along +z_b and its fins push back.
                            let f_q = ex_w * (-cnq * qd * s_ref * (s1 - x_cg * s0) / v);
                            f_body += f_q;
                            f_aero += f_q;
                        }
                        if c.cnpa != 0.0 && lat > 1e-12 {
                            // Magnus force C_N = cnpa p_hat sin(alpha) along u_b x e_x (90 deg from the
                            // static plane, Murphy's convention), applied at the centre of pressure.
                            let f_m = magnus_dir * (qd * s_ref * c.cnpa * p_hat / v);
                            f_body += f_m;
                            f_aero += f_m;
                            m_body += Vec3::new(x_cg - c.xcp, 0.0, 0.0).cross(f_m);
                        }
                        // Roll: the tabulated forcing at the reference cant (zero for a rocket without
                        // cant), the change of cant and the roll damping.
                        let mut cl = c.cl_delta * (self.table.meta.fin_cant_ref_deg.to_radians() + d_cant) * sgn + c.clp * p_hat;
                        if c.cl4 != 0.0 {
                            // Induced roll of the fins (reversed in tail-first flow as the cant forcing).
                            cl += induced_roll(c.cl4, alpha_eff, ub.y, ub.z, self.table.meta.fin_roll_deg.to_radians()) * sgn;
                        }
                        m_body += Vec3::new(qd * s_ref * d_ref * cl, 0.0, 0.0);
                        if ms.cg_lat != [0.0, 0.0] {
                            // The aerodynamic force acts on the axis; the CG is off it.
                            m_body += Vec3::new(0.0, -ms.cg_lat[0], -ms.cg_lat[1]).cross(f_aero);
                        }
                        if c.lag_l0 != 0.0 || c.lag_l1 != 0.0 {
                            lag = Some((c.lag_l0, c.lag_l1, lag_fade(alpha_eff)));
                        }
                        aux = Aux { airspeed: v, mach, alpha, qdyn: qd, cn: c.cn, ca, x_cp: c.xcp, beta, cl, cy: 0.0, ..aux };
                    }
                }
                let acc = rot.mul_vec(f_body) / mass + g;
                if let Some((l0, l1, fade)) = lag.filter(|l| l.2 > 0.0) {
                    // Lag (alpha-dot) and convective rotation moments of the apparent mass, known here
                    // without an algebraic loop: no lag force is added, so `acc` does not depend on it.
                    // du_b/dt = R^T (dv/dt - dW/dt) - w_t x u_b, with dW/dt = (dW/dh) dh/dt the change of
                    // the wind along the trajectory (shear) and w_t = (0, q, r): the crossflow is
                    // differentiated in axes that do not roll with the body, since the apparent mass of a
                    // body of revolution does not change when it rolls (with p = 0 this is the body-axis
                    // derivative). In slender-body theory (see `AeroModel::lag_sums`) the section force
                    // -rho D/Dt[A w_c] has, besides the static and local-incidence parts of the table, the
                    // acceleration term A du_perp/dt and the convective rotation term V A q, whose
                    // lateral vector is -w_t x u_b (lateral part (0, -r V, q V)). Both act as a force
                    // -(rho S / 2) L0 (...) at L1 / L0 aft of the nose, so the moment is
                    // dM = -(rho S / 2)(x_cg L0 - L1)(e_x x a_perp), a = du_b/dt - w_t x u_b
                    // = R^T (dv/dt - dW/dt) - 2 w_t x u_b. Their forces (-rho Vol (dw/dt + V q)) are not
                    // applied. The reaction of the air the body carries does not depend on whether the
                    // nose or the tail leads (as the static normal force and the damping), so it is not
                    // reversed in tail-first flow.
                    //
                    // dW/dh is a central difference over +-1 cm; at a knot of a wind profile it is the
                    // mean of the two slopes (the profile is only piecewise smooth).
                    const DH: f64 = 0.01;
                    let h = loc.height;
                    let dwdh = (self.wind.at(h + DH) - self.wind.at(h - DH)) / (2.0 * DH);
                    let w_dot = loc.to_frame(dwdh) * s.vel.dot(loc.up);
                    let u_dot = rot.transpose_mul_vec(acc - w_dot) - Vec3::new(0.0, s.w.y, s.w.z).cross(ub) * 2.0;
                    let k = -0.5 * atm.density * s_ref * (x_cg * l0 - l1) * fade;
                    m_body += Vec3::new(0.0, -u_dot.z, u_dot.y) * k;
                }
                // `s.w` is the angular velocity relative to the frame; the Euler equations need the
                // inertial one, w + R^T Omega (Omega = 0 in the flat frame). In body components
                // dw_rel/dt = dw_inertial/dt + w_rel x (R^T Omega).
                let om_b = rot.transpose_mul_vec(fr.omega());
                let wi = s.w + om_b;
                let dwi = if ms.diagonal {
                    let (ixx, iyy) = (ms.inertia.0[0][0], ms.inertia.0[1][1]);
                    let iw = Vec3::new(ixx * wi.x, iyy * wi.y, iyy * wi.z);
                    let gyro = wi.cross(iw);
                    Vec3::new((m_body.x - gyro.x) / ixx, (m_body.y - gyro.y) / iyy, (m_body.z - gyro.z) / iyy)
                } else {
                    // Full tensor: dw = I^-1 (M - w x I w).
                    let gyro = wi.cross(ms.inertia.mul_vec(wi));
                    // `check_inertia` rejects tensors that cannot be inverted; should one still
                    // fail, the NaN stops the run as a divergence instead of a wrong answer.
                    ms.inv.map_or(Vec3::new(f64::NAN, f64::NAN, f64::NAN), |inv| inv.mul_vec(m_body - gyro))
                };
                let dw = dwi + s.w.cross(om_b);
                (State { pos: s.vel, vel: acc, q: s.q.derivative(s.w), w: dw }, aux)
            }
            Phase::Parachute => {
                // ECEF: the wind (east, north) is taken in the local ENU basis at the rocket, not the launch-site
                // basis, so a wind "from the west" stays westerly along the track; the two differ by a rotation of
                // about downrange distance / R_earth (0.5 deg at 55 km), negligible against wind uncertainty.
                let va = s.vel - loc.to_frame(self.wind.at(loc.height));
                let v = va.norm();
                let drag = va * (-0.5 * atm.density * v * cx.cd_s.get() / mass);
                aux.airspeed = v;
                aux.mach = v / atm.sound_speed;
                aux.qdyn = 0.5 * atm.density * v * v;
                (State { pos: s.vel, vel: drag + g, ..zero }, aux)
            }
        }
    }

    fn sample(&self, cx: &Ctx, t: f64, s: &State, phase: Phase, aux: &Aux) -> Sample {
        let fr = &cx.frame;
        let (lat, lon, alt) = fr.lla(s.pos);
        let loc = fr.local(s.pos);
        let enu = fr.position_to_launch_enu(s.pos);
        let vel = fr.to_launch_enu(s.vel);
        let xb = s.q.to_mat().col(0);
        let ref_d = self.table.meta.ref_diameter;
        Sample {
            t,
            phase,
            east: enu.x,
            north: enu.y,
            up: enu.z,
            lat,
            lon,
            alt,
            vel_e: vel.x,
            vel_n: vel.y,
            vel_u: vel.z,
            airspeed: aux.airspeed,
            mach: aux.mach,
            alpha_deg: aux.alpha.to_degrees(),
            thrust: aux.thrust,
            mass: aux.mass,
            x_cg: aux.x_cg,
            x_cp: aux.x_cp,
            stability_cal: (aux.x_cp - aux.x_cg) / ref_d,
            dyn_pressure: aux.qdyn,
            cn: aux.cn,
            ca: aux.ca,
            pitch_deg: xb.dot(loc.up).clamp(-1.0, 1.0).asin().to_degrees(),
            heading_deg: xb.dot(loc.east).atan2(xb.dot(loc.north)).to_degrees().rem_euclid(360.0),
            wx: s.w.x,
            wy: s.w.y,
            wz: s.w.z,
            beta_deg: aux.beta.to_degrees(),
            roll_rate: s.w.x,
            cl: aux.cl,
            cy: aux.cy,
        }
    }

    pub fn run(&self, descent: Descent, record: bool) -> Result<SimResult> {
        self.run_impl(descent, record, None)
    }

    /// Like [`run`](Self::run), additionally calling `observer` with every state sample taken at
    /// the output interval (and at touchdown), e.g. to drive a progress display. The samples are
    /// only stored in the result when `record` is set.
    pub fn run_with_observer(&self, descent: Descent, record: bool, observer: &mut dyn FnMut(&Sample)) -> Result<SimResult> {
        self.run_impl(descent, record, Some(observer))
    }

    fn run_impl(&self, descent: Descent, record: bool, mut observer: Option<&mut dyn FnMut(&Sample)>) -> Result<SimResult> {
        let want_samples = record || observer.is_some();
        let dt = self.settings.dt;
        if !is_pos(dt) {
            bail!("sim.dt must be positive");
        }
        let elev = self.launch.elevation_deg;
        if !(elev > 0.0 && elev <= 90.0) {
            bail!("launch.elevation_deg must be in (0, 90], found {elev}");
        }
        if descent == Descent::Parachute && !self.recovery.enabled {
            bail!("the parachute descent needs recovery.enabled = true (with recovery disabled there is no parachute to deploy); fly the ballistic descent or enable [recovery]");
        }
        self.recovery.validate()?;
        self.check_inertia()?;
        let stepper = Stepper::new(self.settings.integrator, self.settings.attitude);
        let adaptive = stepper.adaptive();
        if adaptive && !(is_pos(self.settings.rtol) && is_pos(self.settings.atol) && is_pos(self.settings.output_interval)) {
            bail!("sim.rtol, sim.atol and sim.output_interval must be positive for the adaptive integrators (rk45, dop853)");
        }
        let frame = Frame::new(&self.earth, &self.launch);
        let cx = Ctx { frame, rail: frame.from_launch_enu(self.launch.rail_dir_enu()), cd_s: Default::default(), segment: Default::default(), gravity_off: false };
        let mut s = State { pos: frame.origin(), vel: Vec3::ZERO, q: self.initial_attitude(&frame), w: Vec3::ZERO };
        cx.cd_s.set(self.recovery.cd_s);
        let main = if self.recovery.enabled { self.recovery.main() } else { None };
        let mut main_pending = false;
        let mut phase = Phase::Rail;
        let mut t = 0.0;
        let mut samples = Vec::new();
        let mut next_out = 0.0;
        let burn = self.motor.burn_time();
        let mut sum = Summary {
            descent,
            wind_speed: self.wind.speed,
            wind_direction_deg: self.wind.direction_deg,
            rail_exit_time: f64::NAN,
            rail_exit_speed: f64::NAN,
            burnout_time: burn,
            max_speed: 0.0,
            max_mach: 0.0,
            max_dyn_pressure: 0.0,
            apogee_time: 0.0,
            apogee: 0.0,
            apogee_east: 0.0,
            apogee_north: 0.0,
            min_stability_cal: f64::INFINITY,
            stability_at_rail_exit: f64::NAN,
            static_margin_pitch_min_cal: f64::NAN,
            static_margin_yaw_min_cal: f64::NAN,
            static_margin_min_t: f64::NAN,
            static_margin_min_mach: f64::NAN,
            stability_unknown: None,
            broadside_time: 0.0,
            landing_time: f64::NAN,
            landing_east: f64::NAN,
            landing_north: f64::NAN,
            landing_lat: f64::NAN,
            landing_lon: f64::NAN,
            landing_distance: f64::NAN,
            landing_speed: f64::NAN,
            main_deploy_time: None,
            main_deploy_altitude: None,
            drogue_deploy_time: None,
            deploy_delayed: None,
            drogue_deploy_speed: None,
            drogue_deploy_q: None,
            main_deploy_speed: None,
            main_deploy_q: None,
            wind_scale: (self.wind.model == crate::env::WindModel::Profile).then_some(self.wind.profile_scale),
            roll_resonance: None,
        };
        // Roll-resonance check: trim forcing fixed in the body, and (t, Mach, |p| - omega_n, |p|) of the previous ascent step.
        // A general table with a pitch or yaw moment at zero angles is a built-in (aerodynamic) trim forcing.
        let trim = self.fin_cant_deg != 0.0
            || self.mass.cg_lateral_dry != [0.0, 0.0]
            || !self.thrust_line.is_aligned()
            || self.table.max_zero_angle_moment() > ZERO_ANGLE_MOMENT_TRIM;
        let mut res_prev: Option<(f64, f64, f64, f64)> = None;
        let mut deploy_at = f64::INFINITY;
        let mut past_apogee = false;
        let mut margins = MarginTrack::new();
        // Time of the rail exit, whose margins are added to `margins` once, when the rail is left.
        let mut rail_exit_tracked_at: Option<f64> = None;
        let evals = std::cell::Cell::new(0u64);
        let mut h_ctrl = dt;
        let mut max_qerr = 0.0f64;
        // Derivative at (t, s) carried over from the previous step (first-same-as-last).
        let mut fsal: Option<(State, Aux)> = None;
        let mut loc = frame.local(s.pos);
        let max_time = self.settings.max_time;

        loop {
            // The thrust-curve segment is fixed per step; a new one invalidates the carried derivative.
            let seg = self.motor.segment_after(t);
            if seg != cx.segment.get() {
                cx.segment.set(seg);
                fsal = None;
            }
            let mut f = |tt: f64, st: &State| {
                evals.set(evals.get() + 1);
                self.deriv(&cx, tt, st, phase)
            };
            let (k1, aux) = fsal.take().unwrap_or_else(|| f(t, &s));
            // Ballistic descent broadside to the flow: the step counts with the angle at its start.
            let broadside = descent == Descent::Ballistic && phase == Phase::Free && past_apogee && BROADSIDE_ALPHA.contains(&aux.alpha);
            if want_samples && t + 1e-9 >= next_out {
                let smp = self.sample(&cx, t, &s, phase, &aux);
                if let Some(o) = observer.as_mut() {
                    o(&smp);
                }
                if record {
                    samples.push(smp);
                }
                next_out += self.settings.output_interval;
            }
            let speed = s.vel.norm();
            sum.max_speed = sum.max_speed.max(speed);
            sum.max_mach = sum.max_mach.max(aux.mach);
            sum.max_dyn_pressure = sum.max_dyn_pressure.max(aux.qdyn);
            // The rail-exit state was tracked when the rail was left (the first free-flight step starts there).
            if phase == Phase::Free && !past_apogee && aux.airspeed > 10.0 && rail_exit_tracked_at != Some(t) {
                let m = margins.add(&self.margins(&aux), t, aux.mach);
                sum.min_stability_cal = sum.min_stability_cal.min(m);
                if trim {
                    let wn = self.pitch_frequency(t, &aux);
                    if wn.is_finite() {
                        let (p, diff) = (s.w.x.abs(), s.w.x.abs() - wn);
                        if let Some((t0, m0, d0, p0)) = res_prev.filter(|r| (r.2 < 0.0) != (diff < 0.0)) {
                            let f = d0 / (d0 - diff);
                            let cross = RollResonance { t: t0 + (t - t0) * f, mach: m0 + (aux.mach - m0) * f, frequency: p0 + (p - p0) * f, crossings: 1 };
                            match &mut sum.roll_resonance {
                                Some(r) => r.crossings += 1,
                                None => sum.roll_resonance = Some(cross),
                            }
                        }
                        res_prev = Some((t, aux.mach, diff, p));
                    }
                }
            }
            if t >= max_time - 1e-9 {
                break;
            }
            if phase == Phase::Rail && t > burn && s.vel.dot(cx.rail) <= 0.0 {
                bail!("rocket never left the launch rail (thrust too low?)");
            }

            // Latest end of this step: sim.max_time, the next thrust-curve knot (burnout included)
            // and the parachute deployment instant. A step trimmed to one of them ends on it exactly.
            let mut limit = max_time;
            if let Some(tk) = self.motor.next_knot(t) {
                limit = limit.min(tk);
            }
            if descent == Descent::Parachute && phase == Phase::Free && deploy_at > t + 1e-12 {
                limit = limit.min(deploy_at);
            }
            let prev = s;
            let t0 = t;
            let hh;
            let mut next_fsal = None;
            if !adaptive {
                hh = dt.min(limit - t0);
                s = stepper.step(&mut f, t0, &s, &k1, hh, 0.0, 0.0, phase == Phase::Free).y;
            } else {
                // Largest allowed step: output interval, 0.05 s on the rail and during the burn,
                // trimmed to the limit above.
                let mut hmax = self.settings.output_interval;
                if phase == Phase::Rail || t0 < burn {
                    hmax = hmax.min(0.05);
                }
                if phase == Phase::Rail {
                    // Aim the step at the rail end (constant-acceleration estimate) so the exit
                    // time and speed are resolved to ~1e-6 s instead of the step size.
                    let remaining = self.launch.rail_length - (s.pos - frame.origin()).dot(cx.rail);
                    let (v, a) = (s.vel.dot(cx.rail), k1.vel.dot(cx.rail));
                    if a > 0.0 && remaining > 0.0 {
                        hmax = hmax.min((-v + (v * v + 2.0 * a * remaining).sqrt()) / a + 1e-6);
                    }
                }
                hmax = hmax.min(limit - t0);
                let mut h_try = h_ctrl.min(hmax);
                let mut rejected = false;
                loop {
                    let st = stepper.step(&mut f, t0, &s, &k1, h_try, self.settings.rtol, self.settings.atol, phase == Phase::Free);
                    let fac = stepper.factor(st.err);
                    if st.err <= 1.0 {
                        s = st.y;
                        next_fsal = st.next;
                        // A step trimmed by a limit must not shrink the controller's step.
                        h_ctrl = if h_try < h_ctrl && !rejected { h_ctrl.max(h_try * fac) } else { h_try * fac };
                        break;
                    }
                    rejected = true;
                    h_try *= fac;
                    integrator::check_min_step(h_try, t0)?;
                }
                hh = h_try;
            }
            t = if hh >= limit - t0 { limit } else { t0 + hh };
            let h = t - t0;
            if broadside {
                sum.broadside_time += h;
            }
            if !finite_state(&s) {
                bail!("integration diverged at t = {t:.4} s; reduce sim.dt or tighten rtol/atol, or check the aero table and mass properties");
            }
            // A state no rocket reaches is a numerical blow-up as well, even when it is still finite.
            let (speed_now, alt_now) = (s.vel.norm(), frame.local(s.pos).height);
            if speed_now > MAX_SPEED {
                bail!("integration diverged at t = {t:.4} s (speed {speed_now:.3e} m/s is not physical); reduce sim.dt or tighten rtol/atol, or check the aero table and mass properties");
            }
            if alt_now > MAX_ALTITUDE {
                bail!("integration diverged at t = {t:.4} s (altitude {alt_now:.3e} m is not physical); reduce sim.dt or tighten rtol/atol, or check the aero table and mass properties");
            }
            max_qerr = max_qerr.max((crate::lie::quat_norm(s.q) - 1.0).abs());
            let loc_prev = loc;
            loc = frame.local(s.pos);

            if phase == Phase::Rail {
                let along = |p: Vec3| (p - frame.origin()).dot(cx.rail) - self.launch.rail_length;
                let (v0, v1) = (prev.vel.dot(cx.rail), s.vel.dot(cx.rail));
                if v1 < 0.0 {
                    // The rocket is held where it stops (eq. rail: a = 0 for v <= 0 and a < 0), but a
                    // step that brakes it can end with a small negative speed. It stops instead at
                    // the zero of the step's linear velocity (the Hermite position there), at rest,
                    // never behind its position at the start of the step nor the rail base.
                    let fz = if v0 > 0.0 { v0 / (v0 - v1) } else { 0.0 };
                    let travelled = |p: Vec3| along(p) + self.launch.rail_length;
                    let stop = travelled(hermite_pos(&prev, &s, h, fz)).max(travelled(prev.pos)).max(0.0);
                    s = State { pos: frame.origin() + cx.rail * stop, vel: Vec3::ZERO, ..s };
                    loc = frame.local(s.pos);
                    fsal = None;
                    continue;
                }
                if along(s.pos) >= 0.0 {
                    // The step overshoots the rail end: find the crossing on the cubic Hermite
                    // interpolant of the step (exact for a cubic position, e.g. linearly varying
                    // acceleration; the exit velocity is its derivative) and restart the
                    // integration there in free flight.
                    let fe = bisect(|f| along(hermite_pos(&prev, &s, h, f)), along(prev.pos));
                    s = event_state(&prev, &s, h, fe);
                    t = t0 + fe * h;
                    loc = frame.local(s.pos);
                    phase = Phase::Free;
                    sum.rail_exit_time = t;
                    sum.rail_exit_speed = s.vel.norm();
                    let (_, aux) = self.deriv(&cx, t, &s, phase);
                    sum.stability_at_rail_exit = margins.add(&self.margins(&aux), t, aux.mach);
                    if aux.airspeed > 10.0 {
                        sum.min_stability_cal = sum.min_stability_cal.min(sum.stability_at_rail_exit);
                    }
                    rail_exit_tracked_at = Some(t);
                } else {
                    fsal = next_fsal;
                }
                continue;
            }

            if loc.height > sum.apogee {
                sum.apogee = loc.height;
                sum.apogee_time = t;
                let enu = frame.position_to_launch_enu(s.pos);
                sum.apogee_east = enu.x;
                sum.apogee_north = enu.y;
            }
            let (vz_prev, vz) = (prev.vel.dot(loc_prev.up), s.vel.dot(loc.up));
            if !past_apogee && vz_prev > 0.0 && vz <= 0.0 {
                past_apogee = true;
                // Zero of the vertical velocity of the step's cubic Hermite interpolant, and the
                // interpolant's position at that instant.
                let fa = vertical_speed_zero(&frame, &prev, &s, h);
                let t_apo = t0 + h * fa;
                let p = hermite_pos(&prev, &s, h, fa);
                let h_apo = frame.local(p).height;
                if h_apo >= sum.apogee {
                    let enu = frame.position_to_launch_enu(p);
                    sum.apogee = h_apo;
                    sum.apogee_east = enu.x;
                    sum.apogee_north = enu.y;
                }
                sum.apogee_time = t_apo;
                deploy_at = t_apo + self.recovery.delay;
                let thrust_end = self.motor.thrust_end();
                if descent == Descent::Parachute && deploy_at < thrust_end {
                    // The motor still burns at apogee + delay: a parachute opened under thrust is
                    // not modelled (the parachute phase has no thrust and the propellant would keep
                    // burning off), so the deployment waits for the end of the thrust, a
                    // thrust-curve knot and hence a step limit.
                    sum.deploy_delayed = Some(DeployDelay { nominal_time: deploy_at, thrust: self.motor.thrust(deploy_at) });
                    deploy_at = thrust_end;
                }
            }

            // Events inside the step, handled earliest first; the touchdown wins a tie, and an
            // event after the touchdown does not happen.
            // Touchdown: zero of the height on the cubic Hermite interpolant of the step.
            let f_ground = crosses_ground(loc_prev.height, loc.height).then(|| bisect(|f| -frame.local(hermite_pos(&prev, &s, h, f)).height, -loc_prev.height));
            let f_deploy = (descent == Descent::Parachute && phase == Phase::Free && deploy_at <= t + 1e-12).then(|| ((deploy_at - t0) / h).clamp(0.0, 1.0));
            let f_main = match main {
                Some((_, main_alt)) if main_pending && phase == Phase::Parachute && loc_prev.height > main_alt && loc.height <= main_alt => {
                    // Crossing of the main altitude on the cubic Hermite interpolant of the step.
                    Some(bisect(|f| main_alt - frame.local(hermite_pos(&prev, &s, h, f)).height, main_alt - loc_prev.height))
                }
                _ => None,
            };
            let f_event = f_deploy.or(f_main);
            if let Some(f) = f_ground.filter(|&g| f_event.is_none_or(|e| g <= e)) {
                let p = hermite_pos(&prev, &s, h, f);
                let enu = frame.position_to_launch_enu(p);
                let (lat, lon, _) = frame.lla(p);
                sum.landing_time = t0 + f * h;
                if broadside {
                    // Only the part of the step before the touchdown.
                    sum.broadside_time -= (1.0 - f) * h;
                }
                sum.landing_east = enu.x;
                sum.landing_north = enu.y;
                sum.landing_lat = lat;
                sum.landing_lon = lon;
                sum.landing_distance = enu.x.hypot(enu.y);
                // Full touchdown state: Hermite position and velocity, the rest linear between the last two states.
                let st = event_state(&prev, &s, h, f);
                sum.landing_speed = st.vel.norm();
                // The touchdown state counts for the maxima as every step start does.
                let (_, aux) = self.deriv(&cx, sum.landing_time, &st, phase);
                sum.max_speed = sum.max_speed.max(sum.landing_speed);
                sum.max_mach = sum.max_mach.max(aux.mach);
                sum.max_dyn_pressure = sum.max_dyn_pressure.max(aux.qdyn);
                if want_samples {
                    let mut last = self.sample(&cx, sum.landing_time, &st, phase, &aux);
                    if frame.is_flat() {
                        last.up = 0.0;
                    }
                    if let Some(o) = observer.as_mut() {
                        o(&last);
                    }
                    if record {
                        samples.push(last);
                    }
                }
                break;
            }
            if let Some(fe) = f_event {
                // Go back to the event instant (cubic Hermite position and velocity) unless
                // the step ends on it, and continue from there with the new drag area.
                if fe < 1.0 {
                    s = event_state(&prev, &s, h, fe);
                    t = if f_deploy.is_some() { deploy_at } else { t0 + fe * h };
                    loc = frame.local(s.pos);
                }
                // Airspeed and dynamic pressure the opening canopy meets.
                let (v_air, q_air) = self.airspeed_q(&s, &loc);
                if f_deploy.is_some() {
                    phase = Phase::Parachute;
                    sum.drogue_deploy_time = Some(t);
                    sum.drogue_deploy_speed = Some(v_air);
                    sum.drogue_deploy_q = Some(q_air);
                    cx.cd_s.set(self.recovery.cd_s);
                    if let Some((main_cd_s, main_alt)) = main {
                        if loc.height <= main_alt {
                            // Already below the main altitude: the main deploys at once.
                            cx.cd_s.set(main_cd_s);
                            sum.main_deploy_time = Some(t);
                            sum.main_deploy_altitude = Some(loc.height);
                            sum.main_deploy_speed = Some(v_air);
                            sum.main_deploy_q = Some(q_air);
                        } else {
                            main_pending = true;
                        }
                    }
                } else if let Some((main_cd_s, _)) = main {
                    cx.cd_s.set(main_cd_s);
                    main_pending = false;
                    sum.main_deploy_time = Some(t);
                    sum.main_deploy_altitude = Some(loc.height);
                    sum.main_deploy_speed = Some(v_air);
                    sum.main_deploy_q = Some(q_air);
                }
                continue;
            }
            fsal = next_fsal;
        }
        if !sum.min_stability_cal.is_finite() {
            sum.min_stability_cal = f64::NAN;
        }
        margins.finish(&mut sum);
        // The end of the thrust curve counts as burnout only when the flight lasted that long.
        let t_end = if sum.landed() { sum.landing_time } else { t };
        if burn > t_end + 1e-9 {
            sum.burnout_time = f64::NAN;
        }
        Ok(SimResult { samples, summary: sum, evals: evals.get(), max_quat_norm_error: max_qerr })
    }
}

/// Smallest pitch and yaw static margins of a flight; a plane with a missing margin at any evaluated
/// state ends as NaN, and the first reason is kept.
struct MarginTrack {
    pitch: f64,
    yaw: f64,
    /// Smallest evaluated margin of either plane, and the time and Mach number where it occurred.
    min: f64,
    min_t: f64,
    min_mach: f64,
    pitch_missing: bool,
    yaw_missing: bool,
    unknown: Option<String>,
}

impl MarginTrack {
    fn new() -> Self {
        Self {
            pitch: f64::INFINITY,
            yaw: f64::INFINITY,
            min: f64::INFINITY,
            min_t: f64::NAN,
            min_mach: f64::NAN,
            pitch_missing: false,
            yaw_missing: false,
            unknown: None,
        }
    }

    /// Adds the margins `m` at time `t` and Mach number `mach`; returns the smaller evaluated margin
    /// (NaN when neither is).
    fn add(&mut self, m: &StaticMargins, t: f64, mach: f64) -> f64 {
        let smaller = m.pitch.min(m.yaw);
        if smaller < self.min {
            (self.min, self.min_t, self.min_mach) = (smaller, t, mach);
        }
        self.pitch = self.pitch.min(m.pitch);
        self.yaw = self.yaw.min(m.yaw);
        self.pitch_missing |= m.pitch.is_nan();
        self.yaw_missing |= m.yaw.is_nan();
        if let (None, Some(why)) = (&self.unknown, &m.unknown) {
            self.unknown = Some(format!("at t = {t:.2} s: {why}"));
        }
        smaller
    }

    /// Writes the pitch and yaw minima and the reason into `sum`.
    fn finish(self, sum: &mut Summary) {
        let done = |v: f64, missing: bool| if missing || !v.is_finite() { f64::NAN } else { v };
        sum.static_margin_pitch_min_cal = done(self.pitch, self.pitch_missing);
        sum.static_margin_yaw_min_cal = done(self.yaw, self.yaw_missing);
        if self.min.is_finite() {
            (sum.static_margin_min_t, sum.static_margin_min_mach) = (self.min_t, self.min_mach);
        }
        sum.stability_unknown = self.unknown;
    }
}

/// Pitch or yaw moment coefficient at zero angles of a general table above which it counts as a trim
/// forcing of the roll-resonance check.
const ZERO_ANGLE_MOMENT_TRIM: f64 = 1e-4;

#[cfg(test)]
#[path = "flight_dyn_tests.rs"]
mod dyn_tests;

/// Fade of the lag moment with the effective angle of attack `a` [rad]: `cos(a)` times a smoothstep
/// from 1 at 20 deg to 0 at 30 deg. The apparent-mass and downwash-lag model is a small-angle model;
/// at larger angles the crossflow separates and the static table coefficients carry the physics.
fn lag_fade(a: f64) -> f64 {
    const A0: f64 = 20.0 * std::f64::consts::PI / 180.0;
    const A1: f64 = 30.0 * std::f64::consts::PI / 180.0;
    let t = ((a - A0) / (A1 - A0)).clamp(0.0, 1.0);
    a.cos().abs() * (1.0 - t * t * (3.0 - 2.0 * t))
}

/// Largest speed [m/s] and altitude [m] the integration may reach before it counts as diverged.
const MAX_SPEED: f64 = 8000.0;
const MAX_ALTITUDE: f64 = 1.0e6;

/// Cubic Hermite position at fraction `f` of the step `a -> b` of length `h` (positions and velocities at the ends).
fn hermite_pos(a: &State, b: &State, h: f64, f: f64) -> Vec3 {
    let (f2, f3) = (f * f, f * f * f);
    a.pos * (2.0 * f3 - 3.0 * f2 + 1.0) + a.vel * (h * (f3 - 2.0 * f2 + f)) + b.pos * (-2.0 * f3 + 3.0 * f2) + b.vel * (h * (f3 - f2))
}

/// State at fraction `f` of the step `a` -> `b` (length `h`) for an event inside the step (rail exit,
/// deployment, touchdown): position and velocity of the step's cubic Hermite interpolant (the
/// velocity is the time derivative of the position), attitude and angular rate linear.
fn event_state(a: &State, b: &State, h: f64, f: f64) -> State {
    State { pos: hermite_pos(a, b, h, f), vel: hermite_vel(a, b, h, f), ..lerp_state(a, b, f) }
}

/// Velocity of the cubic Hermite interpolant of a step (derivative of [`hermite_pos`] in time).
fn hermite_vel(a: &State, b: &State, h: f64, f: f64) -> Vec3 {
    let f2 = f * f;
    (a.pos - b.pos) * ((6.0 * f2 - 6.0 * f) / h) + a.vel * (3.0 * f2 - 4.0 * f + 1.0) + b.vel * (3.0 * f2 - 2.0 * f)
}

/// Fraction of the step `a` -> `b` (length `h`) where the vertical velocity of the cubic Hermite
/// interpolant falls through zero (the apogee); needs a rising start (`a`) and a non-rising end.
fn vertical_speed_zero(frame: &Frame, a: &State, b: &State, h: f64) -> f64 {
    let vz = |f: f64| {
        let p = hermite_pos(a, b, h, f);
        hermite_vel(a, b, h, f).dot(frame.local(p).up)
    };
    bisect(|f| -vz(f), -vz(0.0))
}

/// Whether a step from height `h0` to `h1` reaches the ground: needs the bracket `h0 > 0 >= h1`.
fn crosses_ground(h0: f64, h1: f64) -> bool {
    h0 > 0.0 && h1 <= 0.0
}

/// Root in [0, 1] of `g`, increasing through zero over the step: `g(0) = g0 < 0 <= g(1)`.
/// Bisection to 2^-52 of the step; returns the end of the final bracket where `g >= 0`.
fn bisect(g: impl Fn(f64) -> f64, g0: f64) -> f64 {
    debug_assert!(g0 < 0.0);
    let (mut lo, mut hi) = (0.0f64, 1.0f64);
    for _ in 0..52 {
        let mid = 0.5 * (lo + hi);
        if g(mid) < 0.0 {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    hi
}

fn finite_state(s: &State) -> bool {
    [s.pos.x, s.pos.y, s.pos.z, s.vel.x, s.vel.y, s.vel.z, s.q.w, s.q.x, s.q.y, s.q.z, s.w.x, s.w.y, s.w.z].iter().all(|v| v.is_finite())
}

/// Linear interpolation between two states (`f` = 0 gives `a`); the quaternion is renormalised.
fn lerp_state(a: &State, b: &State, f: f64) -> State {
    let l = |x: f64, y: f64| x + (y - x) * f;
    State {
        pos: a.pos + (b.pos - a.pos) * f,
        vel: a.vel + (b.vel - a.vel) * f,
        q: Quat { w: l(a.q.w, b.q.w), x: l(a.q.x, b.q.x), y: l(a.q.y, b.q.y), z: l(a.q.z, b.q.z) }.normalized(),
        w: a.w + (b.w - a.w) * f,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::env::{EarthModel, GravityModel, WindConfig};
    use aero::atmosphere::G0;
    use aero::table::TableMeta;
    use aero::{AeroCoeffs, AeroModel, AeroOptions, Extrapolation};
    use geom::sample::SampleRocket;
    use geom::{extract, ExtractOptions};

    fn launch(elev: f64) -> Launch {
        Launch { latitude: 35.0, longitude: 139.0, altitude: 0.0, rail_length: 3.0, elevation_deg: elev, azimuth_deg: 0.0, roll_deg: 0.0 }
    }

    fn wind(speed: f64) -> Wind {
        Wind::from_config(&WindConfig { speed, direction_deg: 0.0, ..Default::default() }).unwrap()
    }

    fn settings() -> Settings {
        Settings { max_time: 600.0, output_interval: 0.1, ..Default::default() }
    }

    const VAC_F: f64 = 300.0;
    const VAC_BURN: f64 = 2.0;
    const VAC_M: f64 = 10.0;

    fn vacuum_parts() -> (AeroTable, Motor) {
        let meta = TableMeta {
            source_hash: String::new(),
            ref_area: 0.01,
            ref_diameter: 0.1,
            length: 1.0,
            machs: vec![0.0, 5.0],
            alphas_deg: vec![0.0, 90.0],
            extrapolation: Extrapolation::Clamp,
            ..Default::default()
        };
        let table = AeroTable::from_fn(meta, |_, _| AeroCoeffs { xcp: 0.5, ..Default::default() });
        let motor = Motor::new("c".into(), 0.05, 0.3, 0.0, vec![(0.0, VAC_F), (VAC_BURN, VAC_F)]);
        (table, motor)
    }

    fn vacuum_sim<'a>(table: &'a AeroTable, motor: &'a Motor) -> Simulation<'a> {
        Simulation {
            table,
            mass: MassProperties::axisymmetric(VAC_M, 0.5, 0.01, 1.0),
            motor,
            motor_aft_x: 1.0,
            launch: launch(90.0),
            recovery: Recovery { enabled: false, cd_s: 0.0, delay: 0.0, main_cd_s: None, main_altitude: None },
            wind: wind(0.0),
            atmosphere: AtmosphereModel::default(),
            earth: Earth::default(),
            settings: settings(),
            aero_scale: AeroScale::default(),
            thrust_line: ThrustLine::default(),
            fin_cant_deg: 0.0,
        }
    }

    /// Cant forcing `aux.cl` at 50 m/s along (`sign` = +1) or against (-1) the body axis.
    fn cant_forcing(ref_cant: f64, fin_cant: f64, sign: f64) -> f64 {
        let meta = TableMeta {
            source_hash: String::new(),
            ref_area: 0.01,
            ref_diameter: 0.1,
            length: 1.0,
            machs: vec![0.0, 5.0],
            alphas_deg: vec![0.0, 90.0],
            extrapolation: Extrapolation::Clamp,
            fin_cant_ref_deg: ref_cant,
            ..Default::default()
        };
        let table = AeroTable::from_fn(meta, |_, _| AeroCoeffs { xcp: 0.5, cl_delta: 2.0, ..Default::default() });
        let (_, motor) = vacuum_parts();
        let mut sim = vacuum_sim(&table, &motor);
        sim.fin_cant_deg = fin_cant;
        let frame = Frame::new(&sim.earth, &sim.launch);
        let cx = Ctx {
            frame,
            rail: frame.from_launch_enu(sim.launch.rail_dir_enu()),
            cd_s: Default::default(),
            segment: std::cell::Cell::new(sim.motor.segment_after(10.0)),
            gravity_off: false,
        };
        let q = sim.initial_attitude(&frame);
        let x_b = q.to_mat().col(0);
        let s = State { pos: frame.origin(), vel: x_b * (50.0 * sign), q, w: Vec3::ZERO };
        sim.deriv(&cx, 10.0, &s, Phase::Free).1.cl
    }

    #[test]
    fn cant_forcing_reverses_in_tail_first_flow() {
        let k = 2.0 * 3f64.to_radians();
        assert!((cant_forcing(0.0, 3.0, 1.0) - k).abs() < 1e-12);
        // Tail-first: the whole cant forcing (also the perturbation) changes sign.
        assert!((cant_forcing(0.0, 3.0, -1.0) + k).abs() < 1e-12);
        assert!((cant_forcing(2.0, 3.0, -1.0) + k).abs() < 1e-12);
        assert!((cant_forcing(2.0, 3.0, 1.0) - k).abs() < 1e-12);
    }

    #[test]
    fn diverging_integration_is_an_error() {
        let g = extract(&SampleRocket::default().mesh(), &ExtractOptions::default()).unwrap();
        let model = AeroModel::new(g, AeroOptions::default());
        let table = AeroTable::build(&model, String::new(), Extrapolation::Linear);
        let (_, motor) = sample_parts();
        let mut sim = sample_sim(&table, &motor, 0.0);
        // A far too coarse RK4 step blows the attitude dynamics up (the sample rocket at dt = 0.2 s does).
        let e = [0.2, 0.5, 1.0, 2.0]
            .iter()
            .find_map(|&dt| {
                sim.settings.dt = dt;
                sim.run(Descent::Ballistic, false).err()
            })
            .expect("no step size diverged")
            .to_string();
        assert!(e.contains("integration diverged at t =") && e.contains("reduce sim.dt"), "{e}");
    }

    #[test]
    fn unphysical_state_is_divergence() {
        let g = extract(&SampleRocket::default().mesh(), &ExtractOptions::default()).unwrap();
        let model = AeroModel::new(g, AeroOptions::default());
        let table = AeroTable::build(&model, String::new(), Extrapolation::Linear);
        let (_, motor) = sample_parts();
        let mut sim = sample_sim(&table, &motor, 0.0);
        sim.settings.dt = 5.0;
        let e = format!("{:#}", sim.run(Descent::Ballistic, false).err().expect("diverges"));
        assert!(e.contains("integration diverged at t =") && e.contains("is not physical") && e.contains("reduce sim.dt"), "{e}");
    }

    /// Terminal speed sqrt(2 m g / (rho Cd S)) with the density at `h` metres.
    fn terminal(sim: &Simulation, mass: f64, cd_s: f64, h: f64) -> f64 {
        (2.0 * mass * G0 / (sim.atmosphere.at(sim.launch.altitude + h).density * cd_s)).sqrt()
    }

    #[test]
    fn dual_deploy_switches_to_main_terminal_speed() {
        let g = extract(&SampleRocket::default().mesh(), &ExtractOptions::default()).unwrap();
        let model = AeroModel::new(g, AeroOptions::default());
        let table = AeroTable::build(&model, String::new(), Extrapolation::Linear);
        let (_, motor) = sample_parts();
        for integrator in [IntegratorKind::Rk4, IntegratorKind::Rk45] {
            let mut sim = sample_sim(&table, &motor, 0.0);
            sim.settings.integrator = integrator;
            sim.recovery = Recovery { enabled: true, cd_s: 0.4, delay: 1.0, main_cd_s: Some(4.0), main_altitude: Some(300.0) };
            let r = sim.run(Descent::Parachute, true).unwrap();
            let sm = &r.summary;
            assert!((sm.main_deploy_altitude.unwrap() - 300.0).abs() < 1e-6);
            let tm = sm.main_deploy_time.unwrap();
            assert!(tm > sm.apogee_time + 1.0 && tm < sm.landing_time);
            // Just above the main altitude the drogue is at its terminal speed ...
            let above = r.samples.iter().rfind(|s| s.t < tm && s.alt > 340.0 && s.alt < 400.0 && s.phase == Phase::Parachute).unwrap();
            let vd = terminal(&sim, 8.0, 0.4, above.alt);
            assert!((above.airspeed / vd - 1.0).abs() < 0.05, "{integrator:?} drogue {} vs {vd}", above.airspeed);
            // ... and below it the descent settles to the main's terminal speed.
            let below = r.samples.iter().rfind(|s| s.t > tm && s.alt > 5.0).unwrap();
            let vm = terminal(&sim, 8.0, 4.0, below.alt);
            assert!((below.airspeed / vm - 1.0).abs() < 0.05, "{integrator:?} main {} vs {vm}", below.airspeed);
            assert!((sm.landing_speed / terminal(&sim, 8.0, 4.0, 0.0) - 1.0).abs() < 0.05, "landing {}", sm.landing_speed);
            let before = r.samples.iter().rfind(|s| s.t <= tm).unwrap();
            assert!(before.alt >= 300.0 - 1e-6 && below.alt < 300.0);
            // Ballistic descent ignores the main parachute.
            let b = sim.run(Descent::Ballistic, false).unwrap().summary;
            assert_eq!(b.main_deploy_time, None);
        }
    }

    #[test]
    fn dual_deploy_main_at_once_when_drogue_deploys_below_main_altitude() {
        let g = extract(&SampleRocket::default().mesh(), &ExtractOptions::default()).unwrap();
        let model = AeroModel::new(g, AeroOptions::default());
        let table = AeroTable::build(&model, String::new(), Extrapolation::Linear);
        let (_, motor) = sample_parts();
        let mut sim = sample_sim(&table, &motor, 0.0);
        sim.recovery = Recovery { enabled: true, cd_s: 0.4, delay: 1.0, main_cd_s: Some(4.0), main_altitude: Some(1.0e5) };
        let sm = sim.run(Descent::Parachute, false).unwrap().summary;
        assert!((sm.main_deploy_time.unwrap() - (sm.apogee_time + 1.0)).abs() < 0.01);
    }

    #[test]
    fn parachute_deploys_at_apogee_plus_delay_even_when_the_step_overshoots_it() {
        let g = extract(&SampleRocket::default().mesh(), &ExtractOptions::default()).unwrap();
        let model = AeroModel::new(g, AeroOptions::default());
        let table = AeroTable::build(&model, String::new(), Extrapolation::Linear);
        let (_, motor) = sample_parts();
        for integrator in [IntegratorKind::Rk4, IntegratorKind::Rk45] {
            for delay in [0.0, 0.003, 1.0] {
                let mut sim = sample_sim(&table, &motor, 0.0);
                sim.settings.integrator = integrator;
                sim.recovery = Recovery { enabled: true, cd_s: 3.5, delay, main_cd_s: None, main_altitude: None };
                let sm = sim.run(Descent::Parachute, false).unwrap().summary;
                let td = sm.drogue_deploy_time.unwrap();
                assert!((td - (sm.apogee_time + delay)).abs() < 1e-9, "{integrator:?} delay {delay}: {td} vs {}", sm.apogee_time + delay);
            }
        }
        // never deployed: landed before apogee + delay
        let mut sim = sample_sim(&table, &motor, 0.0);
        sim.recovery = Recovery { enabled: true, cd_s: 3.5, delay: 5000.0, main_cd_s: None, main_altitude: None };
        let sm = sim.run(Descent::Parachute, false).unwrap().summary;
        assert_eq!(sm.drogue_deploy_time, None);
        assert_eq!(sim.run(Descent::Ballistic, false).unwrap().summary.drogue_deploy_time, None);
    }

    #[test]
    fn dual_deploy_validation() {
        let mk = |c: Option<f64>, a: Option<f64>| Recovery { enabled: true, cd_s: 1.0, delay: 1.0, main_cd_s: c, main_altitude: a };
        assert!(mk(None, None).validate().is_ok() && mk(Some(3.0), Some(200.0)).validate().is_ok());
        for (c, a, needle) in [
            (Some(3.0), None, "together"),
            (None, Some(200.0), "together"),
            (Some(0.0), Some(200.0), "main_cd_s must be positive"),
            (Some(f64::NAN), Some(200.0), "main_cd_s must be positive"),
            (Some(3.0), Some(-1.0), "main_altitude must be positive"),
            (Some(3.0), Some(f64::INFINITY), "main_altitude must be positive"),
        ] {
            let e = mk(c, a).validate().unwrap_err().to_string();
            assert!(e.contains(needle), "{e}");
        }
        let r: Recovery = serde_json::from_str(r#"{"enabled":true,"cd_s":1.0,"delay":1.0}"#).unwrap();
        assert!(r.main().is_none());
        assert!(serde_json::from_str::<Recovery>(r#"{"enabled":true,"cd_s":1.0,"delay":1.0,"main":2}"#).is_err());
    }

    #[test]
    fn touchdown_and_rail_exit_are_interpolated() {
        let (table, motor) = vacuum_parts();
        // Vacuum, vertical: touchdown speed equals the launch-apogee drop exactly; coarse dt must not matter.
        let mut sim = vacuum_sim(&table, &motor);
        let a = VAC_F / VAC_M - G0;
        let v_burn = a * VAC_BURN;
        let h_burn = 0.5 * a * VAC_BURN * VAC_BURN;
        let apogee = h_burn + v_burn * v_burn / (2.0 * G0);
        let v_land = (2.0 * G0 * apogee).sqrt();
        // Rail exit: constant acceleration a over 3 m.
        let v_rail = (2.0 * a * 3.0).sqrt();
        sim.settings.dt = 0.01;
        let r = sim.run(Descent::Ballistic, true).unwrap();
        assert!((r.summary.rail_exit_speed - v_rail).abs() < 0.01, "rail {} vs {v_rail}", r.summary.rail_exit_speed);
        assert!((r.summary.landing_speed - v_land).abs() / v_land < 2e-3, "landing {} vs {v_land}", r.summary.landing_speed);
        let last = r.samples.last().unwrap();
        assert!((last.t - r.summary.landing_time).abs() < 1e-12 && last.up.abs() < 1e-9);
        let vl = (last.vel_e.powi(2) + last.vel_n.powi(2) + last.vel_u.powi(2)).sqrt();
        assert!((vl - r.summary.landing_speed).abs() < 1e-9);
    }

    #[test]
    fn rail_exit_velocity_is_the_derivative_of_the_hermite_position() {
        // Vacuum, vertical: thrust ramping linearly (300 N + 2000 N/s, 10 kg) gives a linearly growing
        // acceleration, a cubic position and a quadratic velocity. The step's cubic Hermite
        // interpolant is exact for it (RK4 integrates it exactly), so with a coarse step the exit
        // speed is exact when it is the Hermite derivative; a linear velocity interpolation is not.
        let (table, _) = vacuum_parts();
        let motor = Motor::new("ramp".into(), 0.05, 0.3, 0.0, vec![(0.0, 300.0), (1.0, 2300.0), (2.0, 2300.0)]);
        let mut sim = vacuum_sim(&table, &motor);
        sim.settings.dt = 0.05;
        let r = sim.run(Descent::Ballistic, false).unwrap();
        let (a0, j) = (300.0 / VAC_M - G0, 200.0);
        let x = |t: f64| 0.5 * a0 * t * t + j * t * t * t / 6.0;
        let (mut lo, mut hi) = (0.0, 1.0);
        for _ in 0..100 {
            let mid = 0.5 * (lo + hi);
            if x(mid) < 3.0 {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        let v_exit = a0 * lo + 0.5 * j * lo * lo;
        assert!((r.summary.rail_exit_time - lo).abs() < 1e-7, "{} vs {lo}", r.summary.rail_exit_time);
        assert!((r.summary.rail_exit_speed - v_exit).abs() < 1e-5, "{} vs {v_exit}", r.summary.rail_exit_speed);
    }

    #[test]
    fn a_thrust_dip_on_the_rail_never_moves_the_rocket_backward() {
        // Vacuum, vertical rail of 3 m, 10 kg: 300 N for 0.2 s (to 4.0 m/s, 0.40 m), a dip to 20 N
        // (decelerates at 7.8 m/s^2: comes to rest at about 1.45 m after 0.72 s and is held there),
        // then 300 N again until it leaves the rail.
        let (table, _) = vacuum_parts();
        let motor = Motor::new("dip".into(), 0.05, 0.3, 0.0, vec![(0.0, 300.0), (0.2, 300.0), (0.201, 20.0), (1.5, 20.0), (1.501, 300.0), (3.0, 300.0)]);
        for integrator in [IntegratorKind::Rk4, IntegratorKind::Rk45, IntegratorKind::Dop853] {
            let mut sim = vacuum_sim(&table, &motor);
            sim.settings.integrator = integrator;
            sim.settings.output_interval = 0.002;
            let r = sim.run(Descent::Ballistic, true).unwrap();
            let rail: Vec<&Sample> = r.samples.iter().filter(|s| s.phase == Phase::Rail).collect();
            assert!(rail.len() > 100, "{integrator:?}: {} rail samples", rail.len());
            for w in rail.windows(2) {
                let (a, b) = (w[0], w[1]);
                assert!(b.up >= a.up - 1e-12 && b.up >= -1e-12, "{integrator:?}: moved backward at t = {}: {} -> {}", b.t, a.up, b.up);
                assert!(b.vel_u >= 0.0, "{integrator:?}: negative rail velocity {} at t = {}", b.vel_u, b.t);
            }
            // Held at the stop point until the thrust returns.
            let held: Vec<&&Sample> = rail.iter().filter(|s| s.t > 0.95 && s.t < 1.5).collect();
            assert!(held.iter().all(|s| s.vel_u == 0.0 && (s.up - held[0].up).abs() < 1e-12), "{integrator:?}");
            assert!((held[0].up - 1.45).abs() < 0.02, "{integrator:?}: stopped at {} m", held[0].up);
            assert!(r.summary.rail_exit_time > 1.5, "{integrator:?}");
        }
    }

    #[test]
    fn vacuum_vertical_flight_matches_analytic() {
        let (table, motor) = vacuum_parts();
        let sim = vacuum_sim(&table, &motor);
        let r = sim.run(Descent::Ballistic, false).unwrap();
        let a = VAC_F / VAC_M - G0;
        let v = a * VAC_BURN;
        let apogee = 0.5 * a * VAC_BURN * VAC_BURN + v * v / (2.0 * G0);
        assert!((r.summary.apogee - apogee).abs() / apogee < 2e-3, "apogee {} vs {}", r.summary.apogee, apogee);
        assert!(r.summary.landing_distance < 1e-6);
    }

    #[test]
    fn hermite_position_is_exact_for_free_fall_and_gives_the_apogee() {
        // z(t) = 100 + 40 t - 4.9 t^2 over a step of 7 s starting at t = 1: apogee at t = 40/9.8.
        let st = |t: f64| State {
            pos: Vec3::new(0.0, 0.0, 100.0 + 40.0 * t - 4.9 * t * t),
            vel: Vec3::new(0.0, 0.0, 40.0 - 9.8 * t),
            q: Quat::IDENTITY,
            w: Vec3::ZERO,
        };
        let (a, b, h) = (st(1.0), st(8.0), 7.0);
        let f = a.vel.z / (a.vel.z - b.vel.z);
        let p = hermite_pos(&a, &b, h, f);
        let t_apo = 40.0 / 9.8;
        assert!((1.0 + h * f - t_apo).abs() < 1e-12);
        assert!((p.z - (100.0 + 40.0 * t_apo - 4.9 * t_apo * t_apo)).abs() < 1e-9, "{}", p.z);
        // End of the step would be 4.9 * (8 - 40/9.8)^2 below the apogee.
        assert!(b.pos.z < p.z - 50.0);
    }

    #[test]
    fn apogee_time_is_the_zero_of_the_hermite_vertical_velocity() {
        // Cubic height z(t) = 100 + 40 t - 3 t^2 - 0.2 t^3 (the step's Hermite interpolant is exact)
        // over a step from t = 1 to 8: vz = 40 - 6 t - 0.6 t^2 is zero at t = (-6 + sqrt(132)) / 1.2;
        // the linear interpolation of vz would give 3.93 s.
        let st = |t: f64| State {
            pos: Vec3::new(0.0, 0.0, 100.0 + 40.0 * t - 3.0 * t * t - 0.2 * t * t * t),
            vel: Vec3::new(0.0, 0.0, 40.0 - 6.0 * t - 0.6 * t * t),
            q: Quat::IDENTITY,
            w: Vec3::ZERO,
        };
        let (a, b, h) = (st(1.0), st(8.0), 7.0);
        let frame = Frame::new(&Earth::default(), &launch(90.0));
        let f = vertical_speed_zero(&frame, &a, &b, h);
        let t_apo = (-6.0 + 132f64.sqrt()) / 1.2;
        assert!((1.0 + h * f - t_apo).abs() < 1e-12, "{} vs {t_apo}", 1.0 + h * f);
        let z = |t: f64| 100.0 + 40.0 * t - 3.0 * t * t - 0.2 * t * t * t;
        assert!((hermite_pos(&a, &b, h, f).z - z(t_apo)).abs() < 1e-9);
    }

    fn sample_sim<'a>(table: &'a AeroTable, motor: &'a Motor, wind: f64) -> Simulation<'a> {
        Simulation {
            table,
            mass: MassProperties::axisymmetric(8.0, 0.85, 0.02, 1.5),
            motor,
            motor_aft_x: 1.5,
            launch: launch(85.0),
            recovery: Recovery { enabled: true, cd_s: 3.5, delay: 1.0, main_cd_s: None, main_altitude: None },
            wind: Wind::from_config(&WindConfig { speed: wind, direction_deg: 270.0, ..Default::default() }).unwrap(),
            atmosphere: AtmosphereModel::default(),
            earth: Earth::default(),
            settings: settings(),
            aero_scale: AeroScale::default(),
            thrust_line: ThrustLine::default(),
            fin_cant_deg: 0.0,
        }
    }

    #[test]
    fn weathercocking_and_drift() {
        let g = extract(&SampleRocket::default().mesh(), &ExtractOptions::default()).unwrap();
        let model = AeroModel::new(g, AeroOptions::default());
        let table = AeroTable::build(&model, String::new(), Extrapolation::Linear);
        let motor = Motor::new("s".into(), 0.054, 0.5, 1.0, vec![(0.0, 800.0), (2.0, 800.0)]);
        let calm = sample_sim(&table, &motor, 0.0).run(Descent::Ballistic, false).unwrap().summary;
        // Wind from the west: a stable rocket turns into the wind (west), a parachute drifts east.
        let windy = sample_sim(&table, &motor, 5.0);
        let bal = windy.run(Descent::Ballistic, false).unwrap().summary;
        let para = windy.run(Descent::Parachute, true).unwrap();
        assert!(calm.landing_east.abs() < 1.0, "calm drift {}", calm.landing_east);
        assert!(bal.landing_east < -20.0, "ballistic east {}", bal.landing_east);
        assert!(para.summary.landing_east > 50.0, "parachute east {}", para.summary.landing_east);
        assert!(calm.min_stability_cal > 1.0);
        assert!(para.summary.landing_speed < 10.0);
        assert!(para.samples.iter().all(|s| s.alpha_deg.is_finite()));
    }

    fn sample_parts() -> (AeroTable, Motor) {
        let g = extract(&SampleRocket::default().mesh(), &ExtractOptions::default()).unwrap();
        let model = AeroModel::new(g, AeroOptions::default());
        let table = AeroTable::build(&model, String::new(), Extrapolation::Linear);
        (table, Motor::new("s".into(), 0.054, 0.5, 1.0, vec![(0.0, 800.0), (2.0, 800.0)]))
    }

    fn ecef(gravity: GravityModel, omega: f64) -> Earth {
        Earth { model: EarthModel::Ecef, gravity, omega }
    }

    #[test]
    fn ecef_matches_flat_without_rotation_effects() {
        let (table, motor) = sample_parts();
        let flat = sample_sim(&table, &motor, 0.0);
        for descent in [Descent::Ballistic, Descent::Parachute] {
            let a = flat.run(descent, false).unwrap().summary;
            // Same gravity law along the geodetic down, no rotation: only the curvature of the
            // Earth (the vertical turns with downrange distance) remains.
            let mut e0 = flat.clone();
            e0.earth = ecef(GravityModel::InverseSquare, 0.0);
            let b = e0.run(descent, false).unwrap().summary;
            assert!((a.apogee - b.apogee).abs() / a.apogee < 5e-3, "apogee {} vs {}", a.apogee, b.apogee);
            println!("ecef-flat landing diff {}", (a.landing_east - b.landing_east).hypot(a.landing_north - b.landing_north));
            assert!((a.landing_east - b.landing_east).hypot(a.landing_north - b.landing_north) < 2.0);
            assert!((a.landing_time - b.landing_time).abs() / a.landing_time < 5e-3);
            // With Earth rotation on top the landing point moves by tens of metres only.
            let mut e1 = flat.clone();
            e1.earth = ecef(GravityModel::InverseSquare, crate::env::OMEGA_EARTH);
            let c = e1.run(descent, false).unwrap().summary;
            // Earth rotation adds a Coriolis drift of a couple of metres to the east.
            assert!((c.apogee - a.apogee).abs() / a.apogee < 5e-3);
            assert!(c.landing_east < -0.3 && c.landing_east > -10.0, "east {}", c.landing_east);
            assert!((c.landing_north - b.landing_north).abs() < 20.0);
        }
    }

    #[test]
    fn ecef_coriolis_drifts_west() {
        let meta = TableMeta {
            source_hash: String::new(),
            ref_area: 0.01,
            ref_diameter: 0.1,
            length: 1.0,
            machs: vec![0.0, 5.0],
            alphas_deg: vec![0.0, 90.0],
            extrapolation: Extrapolation::Clamp,
            ..Default::default()
        };
        let table = AeroTable::from_fn(meta, |_, _| AeroCoeffs { xcp: 0.5, ..Default::default() });
        let (f, burn, m) = (3000.0, 5.0, 10.0);
        let motor = Motor::new("c".into(), 0.05, 0.3, 0.0, vec![(0.0, f), (burn, f)]);
        let mut sim = vacuum_sim(&table, &motor);
        sim.earth = ecef(GravityModel::Constant, crate::env::OMEGA_EARTH);
        sim.settings.max_time = 1000.0;
        let r = sim.run(Descent::Ballistic, false).unwrap().summary;
        // First-order estimate: east acceleration = -2 Omega cos(lat) * v_up, hence
        // x_east = -2 Omega cos(lat) * integral of z dt for the ideal vertical flight.
        let a = f / m - G0;
        let (zb, vb) = (0.5 * a * burn * burn, a * burn);
        let t_coast = (vb + (vb * vb + 2.0 * G0 * zb).sqrt()) / G0;
        let mut int_z = a * burn.powi(3) / 6.0;
        let n = 100_000;
        let dtau = t_coast / n as f64;
        for i in 0..n {
            let tau = (i as f64 + 0.5) * dtau;
            int_z += (zb + vb * tau - 0.5 * G0 * tau * tau) * dtau;
        }
        let expect = -2.0 * crate::env::OMEGA_EARTH * 35f64.to_radians().cos() * int_z;
        assert!(r.landing_east < 0.0);
        assert!((r.landing_east / expect - 1.0).abs() < 0.05, "east {} vs {}", r.landing_east, expect);
    }

    #[test]
    fn rk45_matches_rk4() {
        let (vtable, vmotor) = vacuum_parts();
        let (stable, smotor) = sample_parts();
        let cases: Vec<(&str, Simulation, Descent)> = vec![
            ("vacuum", vacuum_sim(&vtable, &vmotor), Descent::Ballistic),
            ("sample ballistic", sample_sim(&stable, &smotor, 0.0), Descent::Ballistic),
            ("sample wind parachute", sample_sim(&stable, &smotor, 5.0), Descent::Parachute),
        ];
        for (name, base, descent) in cases {
            let mut fine = base.clone();
            fine.settings.dt = 0.001;
            let a = fine.run(descent, false).unwrap();
            let mut ad = base.clone();
            ad.settings.integrator = IntegratorKind::Rk45;
            let b = ad.run(descent, true).unwrap();
            let (sa, sb) = (&a.summary, &b.summary);
            println!(
                "{name}: rk4 apogee {} land ({}, {}) t {} evals {} | rk45 apogee {} land ({}, {}) t {} evals {} samples {}",
                sa.apogee,
                sa.landing_east,
                sa.landing_north,
                sa.landing_time,
                a.evals,
                sb.apogee,
                sb.landing_east,
                sb.landing_north,
                sb.landing_time,
                b.evals,
                b.samples.len()
            );
            assert!((sa.apogee - sb.apogee).abs() / sa.apogee < 1e-3, "{name} apogee");
            assert!((sa.landing_east - sb.landing_east).hypot(sa.landing_north - sb.landing_north) < 1.0, "{name} landing");
            assert!((sa.landing_time - sb.landing_time).abs() < 0.05, "{name} landing time");
            assert!((sa.rail_exit_time - sb.rail_exit_time).abs() < 2e-3 && (sa.rail_exit_speed - sb.rail_exit_speed).abs() < 0.2, "{name} rail exit");
            assert!(b.evals < a.evals, "{name}: rk45 {} evals vs rk4 {}", b.evals, a.evals);
            assert!(b.samples.windows(2).all(|w| w[1].t > w[0].t));
        }
    }

    const COMBOS: [(IntegratorKind, AttitudeKind); 6] = [
        (IntegratorKind::Rk4, AttitudeKind::Normalize),
        (IntegratorKind::Rk4, AttitudeKind::LieGroup),
        (IntegratorKind::Rk45, AttitudeKind::Normalize),
        (IntegratorKind::Rk45, AttitudeKind::LieGroup),
        (IntegratorKind::Dop853, AttitudeKind::Normalize),
        (IntegratorKind::Dop853, AttitudeKind::LieGroup),
    ];

    fn with<'a>(base: &Simulation<'a>, i: IntegratorKind, a: AttitudeKind) -> Simulation<'a> {
        let mut s = base.clone();
        s.settings.integrator = i;
        s.settings.attitude = a;
        s
    }

    #[test]
    fn default_settings_are_rk4_normalize() {
        let d = Settings::default();
        assert_eq!((d.integrator, d.attitude), (IntegratorKind::Rk4, AttitudeKind::Normalize));
    }

    #[test]
    fn vacuum_analytic_all_combinations() {
        let (table, motor) = vacuum_parts();
        let base = vacuum_sim(&table, &motor);
        let a = VAC_F / VAC_M - G0;
        let v = a * VAC_BURN;
        let apogee = 0.5 * a * VAC_BURN * VAC_BURN + v * v / (2.0 * G0);
        for (i, at) in COMBOS {
            let r = with(&base, i, at).run(Descent::Ballistic, false).unwrap();
            assert!((r.summary.apogee - apogee).abs() / apogee < 2e-3, "{i:?} {at:?} apogee {} vs {apogee}", r.summary.apogee);
            assert!(r.summary.landing_distance < 1e-6, "{i:?} {at:?}");
        }
    }

    /// RK4 dt = 0.5 ms reference for a sample-like rocket.
    fn reference(base: &Simulation<'_>, descent: Descent) -> Summary {
        let mut r = base.clone();
        r.settings.dt = 0.0005;
        r.run(descent, false).unwrap().summary
    }

    fn agree(base: &Simulation<'_>, name: &str) {
        // The fixed-step rails end at a step boundary, so rk4 needs dt = 1 ms to meet 0.5 m.
        let mut base = base.clone();
        base.settings.dt = 0.001;
        let base = &base;
        for descent in [Descent::Ballistic, Descent::Parachute] {
            let refs = reference(base, descent);
            for (i, at) in COMBOS {
                let r = with(base, i, at).run(descent, false).unwrap();
                let sm = &r.summary;
                let d_land = (sm.landing_east - refs.landing_east).hypot(sm.landing_north - refs.landing_north);
                println!(
                    "{name} {descent:?} {i:?}/{at:?}: apogee {:.3} ({:+.3}) landing ({:.2}, {:.2}) d {:.3} evals {}",
                    sm.apogee,
                    sm.apogee - refs.apogee,
                    sm.landing_east,
                    sm.landing_north,
                    d_land,
                    r.evals
                );
                assert!((sm.apogee - refs.apogee).abs() < 0.5, "{name} {descent:?} {i:?}/{at:?} apogee {} vs {}", sm.apogee, refs.apogee);
                assert!(d_land < 2.0, "{name} {descent:?} {i:?}/{at:?} landing off by {d_land}");
            }
        }
    }

    #[test]
    fn all_combinations_agree_with_fine_rk4() {
        let (table, motor) = sample_parts();
        agree(&sample_sim(&table, &motor, 5.0), "flat");
    }

    #[test]
    fn ecef_dop853_lie_group_runs_and_agrees() {
        let (table, motor) = sample_parts();
        let mut base = sample_sim(&table, &motor, 5.0);
        base.earth = ecef(GravityModel::InverseSquare, crate::env::OMEGA_EARTH);
        agree(&base, "ecef");
        let r = with(&base, IntegratorKind::Dop853, AttitudeKind::LieGroup).run(Descent::Ballistic, true).unwrap();
        assert!(r.samples.len() > 100 && r.summary.landing_time.is_finite());
    }

    /// Touchdown at rtol = atol = 1e-10 with a power-law wind must not stall the adaptive
    /// integrators. Before the cubic ground blend the infinite derivative of the power law at
    /// h = 0 shrank the step below 1e-6 s near touchdown and the run failed with a min-step error.
    /// The motor starts and ends at zero thrust so the launch-rail start is not the limiting
    /// discontinuity; the wind is the only non-smooth input near the ground.
    #[test]
    fn tight_tolerance_touchdown_with_wind_all_integrators() {
        let (table, _) = sample_parts();
        let motor = Motor::new("s".into(), 0.054, 0.5, 1.0, vec![(0.0, 0.0), (0.1, 800.0), (2.0, 800.0), (2.1, 0.0)]);
        let base = sample_sim(&table, &motor, 4.0);
        for descent in [Descent::Ballistic, Descent::Parachute] {
            for i in [IntegratorKind::Rk45, IntegratorKind::Dop853] {
                for at in [AttitudeKind::Normalize, AttitudeKind::LieGroup] {
                    let mut b = base.clone();
                    b.settings.rtol = 1e-10;
                    b.settings.atol = 1e-10;
                    let r = with(&b, i, at).run(descent, false).unwrap();
                    assert!(r.summary.landing_time.is_finite(), "{descent:?} {i:?}/{at:?}");
                    assert!(r.summary.landing_speed.is_finite(), "{descent:?} {i:?}/{at:?}");
                }
            }
        }
    }

    #[test]
    fn lie_group_keeps_unit_quaternion_without_normalisation() {
        let (table, motor) = sample_parts();
        let base = sample_sim(&table, &motor, 5.0);
        let mut earths = vec![Earth::default()];
        earths.push(ecef(GravityModel::InverseSquare, crate::env::OMEGA_EARTH));
        for earth in earths {
            let mut b = base.clone();
            b.earth = earth;
            for i in [IntegratorKind::Rk4, IntegratorKind::Rk45, IntegratorKind::Dop853] {
                let r = with(&b, i, AttitudeKind::LieGroup).run(Descent::Parachute, false).unwrap();
                assert!(r.max_quat_norm_error < 1e-12, "{i:?}: |q|-1 = {:e}", r.max_quat_norm_error);
            }
        }
    }

    #[test]
    fn dop853_needs_far_fewer_evaluations_than_rk45() {
        // A smooth model: coefficients that are constant in Mach and linear in alpha are
        // reproduced exactly by the table's linear interpolation, there is no wind (the power-law
        // profile is singular at the ground) and the thrust curve starts and ends at zero. The
        // tabulated aerodynamics of a real rocket are only piecewise linear, which limits the
        // step size of any high-order method independently of its order.
        let meta = TableMeta {
            source_hash: String::new(),
            ref_area: 0.01,
            ref_diameter: 0.1,
            length: 1.0,
            machs: vec![0.0, 5.0],
            alphas_deg: vec![0.0, 90.0],
            extrapolation: Extrapolation::Clamp,
            ..Default::default()
        };
        let table = AeroTable::from_fn(meta, |_, a| AeroCoeffs { cn: 0.2 * a, cna: 11.0, ca_on: 0.5, ca_off: 0.5, xcp: 1.0, ..Default::default() });
        let motor = Motor::new("s".into(), 0.054, 0.5, 1.0, vec![(0.0, 0.0), (0.1, 800.0), (2.0, 800.0), (2.1, 0.0)]);
        let mut base = sample_sim(&table, &motor, 0.0);
        base.settings.output_interval = 0.5;
        // (tolerance, required ratio dop853/rk45 evaluations)
        for (tol, ratio) in [(1e-9, 0.6), (1e-11, 0.4)] {
            base.settings.rtol = tol;
            base.settings.atol = tol;
            for at in [AttitudeKind::Normalize, AttitudeKind::LieGroup] {
                let a = with(&base, IntegratorKind::Rk45, at).run(Descent::Ballistic, false).unwrap();
                let d = with(&base, IntegratorKind::Dop853, at).run(Descent::Ballistic, false).unwrap();
                println!("tol {tol:e}, {at:?}: rk45 {} evals, dop853 {} evals", a.evals, d.evals);
                assert!((d.evals as f64) < ratio * a.evals as f64, "{tol:e} {at:?}: dop853 {} vs rk45 {}", d.evals, a.evals);
                assert!((a.summary.apogee - d.summary.apogee).abs() < 0.05);
            }
        }
    }

    // ---- roll, full inertia tensor, thrust misalignment, general tables ----

    fn roll_meta(machs: Vec<f64>) -> TableMeta {
        TableMeta {
            ref_area: 0.01,
            ref_diameter: 0.1,
            length: 1.0,
            machs,
            alphas_deg: vec![0.0, 90.0],
            extrapolation: Extrapolation::Clamp,
            ..Default::default()
        }
    }

    /// Vertical powered flight in a table with only roll terms; the roll inertia is small so the
    /// roll rate follows the quasi-steady value.
    fn roll_sim<'a>(table: &'a AeroTable, motor: &'a Motor, cant: f64) -> Simulation<'a> {
        let mut sim = vacuum_sim(table, motor);
        sim.mass = MassProperties::axisymmetric(VAC_M, 0.5, 0.001, 1.0);
        sim.fin_cant_deg = cant;
        sim
    }

    /// Steady roll rate p = -2 V Cl / (Clp d) for the total roll coefficient Cl; checked on the samples
    /// of the second half of the burn.
    fn check_steady_roll(sim: &Simulation, cl_total: f64, clp: f64) {
        let r = sim.run(Descent::Ballistic, true).unwrap();
        let d = sim.table.meta.ref_diameter;
        let mut n = 0;
        for smp in r.samples.iter().filter(|s| s.t > 1.2 && s.t < 1.9) {
            let expect = -2.0 * smp.airspeed * cl_total / (clp * d);
            assert!((smp.roll_rate / expect - 1.0).abs() < 0.08, "t {}: p {} vs {}", smp.t, smp.roll_rate, expect);
            n += 1;
        }
        assert!(n > 5);
    }

    #[test]
    fn axisymmetric_cant_gives_steady_roll_rate() {
        let (cl_delta, clp, cant) = (2.0, -20.0, 1.5f64);
        let meta = roll_meta(vec![0.0, 5.0]);
        let table = AeroTable::from_fn(meta, |_, _| AeroCoeffs { xcp: 0.5, cl_delta, clp, ..Default::default() });
        let (_, motor) = vacuum_parts();
        check_steady_roll(&roll_sim(&table, &motor, cant), cl_delta * cant.to_radians(), clp);
    }

    #[test]
    fn general_table_roll_adds_only_the_cant_difference() {
        let (cl_delta, clp, cl0) = (2.0, -20.0, 0.3);
        let mut meta = roll_meta(vec![0.0, 5.0]);
        meta.alphas_deg = vec![-180.0, -90.0, 0.0, 90.0, 180.0];
        meta.betas_deg = vec![-90.0, 0.0, 90.0];
        meta.fin_cant_ref_deg = 2.0;
        let table = AeroTable::general_from_fn(meta, |_, _, _| aero::table::Coeffs6 { cl: cl0, cl_delta, clp, ..Default::default() });
        let (_, motor) = vacuum_parts();
        // delta = delta_ref: the table value alone; otherwise the cant difference is added.
        check_steady_roll(&roll_sim(&table, &motor, 2.0), cl0, clp);
        check_steady_roll(&roll_sim(&table, &motor, 3.0), cl0 + cl_delta * 1.0f64.to_radians(), clp);
    }

    #[test]
    fn axisymmetric_reference_cant_with_equal_cant_keeps_table_roll() {
        let (cl_delta, clp) = (2.0, -20.0);
        let mut meta = roll_meta(vec![0.0, 5.0]);
        meta.fin_cant_ref_deg = 2.0;
        let table = AeroTable::from_fn(meta, |_, _| AeroCoeffs { xcp: 0.5, cl_delta, clp, ..Default::default() });
        let (_, motor) = vacuum_parts();
        check_steady_roll(&roll_sim(&table, &motor, 2.0), cl_delta * 2.0f64.to_radians(), clp);
    }

    #[test]
    fn parachute_descent_needs_recovery_enabled() {
        let (table, motor) = sample_parts();
        let mut sim = sample_sim(&table, &motor, 0.0);
        sim.recovery.enabled = false;
        let e = format!("{:#}", sim.run(Descent::Parachute, false).err().expect("parachute descent without recovery accepted"));
        assert!(e.contains("the parachute descent needs recovery.enabled = true"), "{e}");
        assert!(sim.run(Descent::Ballistic, false).is_ok());
        // With recovery enabled the drag area must be positive and finite.
        sim.recovery.enabled = true;
        for cd_s in [0.0, -1.0, f64::NAN] {
            sim.recovery.cd_s = cd_s;
            let e = format!("{:#}", sim.run(Descent::Parachute, false).err().expect("bad cd_s accepted"));
            assert!(e.contains("recovery.cd_s must be positive and finite"), "{e}");
        }
        sim.recovery.cd_s = 3.5;
        sim.recovery.delay = -1.0;
        let e = format!("{:#}", sim.run(Descent::Parachute, false).err().expect("negative delay accepted"));
        assert!(e.contains("recovery.delay must be non-negative and finite"), "{e}");
        // A disabled recovery is not validated (its fields are unused).
        sim.recovery = Recovery { enabled: false, cd_s: 0.0, delay: 0.0, main_cd_s: None, main_altitude: None };
        assert!(sim.recovery.validate().is_ok());
    }

    #[test]
    fn ill_conditioned_inertia_is_an_error() {
        let (table, motor) = sample_parts();
        // Positive definite but nearly singular dry tensor: no silent identity inverse.
        for (ixx, izz) in [(1e-16, 1.000001), (0.0, 1.0), (-0.01, 1.0)] {
            let mut sim = sample_sim(&table, &motor, 0.0);
            sim.mass.ixx_dry = ixx;
            sim.mass.iyy_dry = 1.0;
            sim.mass.izz_dry = Some(izz);
            let e = format!("{:#}", sim.run(Descent::Ballistic, false).err().expect("ill-conditioned inertia accepted"));
            assert!(e.contains("inertia tensor is singular or ill-conditioned"), "{e}");
        }
        // The axisymmetric path is checked too.
        let mut sim = sample_sim(&table, &motor, 0.0);
        sim.mass.ixx_dry = 0.0;
        let e = format!("{:#}", sim.run(Descent::Ballistic, false).err().expect("zero roll inertia accepted"));
        assert!(e.contains("inertia tensor is singular or ill-conditioned"), "{e}");
        // A well-conditioned asymmetric tensor flies.
        let mut sim = sample_sim(&table, &motor, 0.0);
        sim.mass.izz_dry = Some(sim.mass.iyy_dry * 1.1);
        sim.mass.ixy_dry = 1e-3;
        assert!(sim.run(Descent::Ballistic, false).is_ok());
    }

    #[test]
    fn point_mass_inertia_sign() {
        // A point mass at (x, y, 0): tensor element Ixy = -m x y (and Ixx = m y^2, Iyy = m x^2).
        let (m, x, y) = (2.0, 0.3, -0.2);
        let i = parallel_axis(m, Vec3::new(x, y, 0.0));
        assert!((i.0[0][1] + m * x * y).abs() < 1e-15 && (i.0[1][0] - i.0[0][1]).abs() < 1e-15);
        assert!((i.0[0][0] - m * y * y).abs() < 1e-15 && (i.0[1][1] - m * x * x).abs() < 1e-15);
        assert!((i.0[2][2] - m * (x * x + y * y)).abs() < 1e-15);
    }

    #[test]
    fn off_axis_dry_cg_gives_product_of_inertia_with_propellant() {
        let (table, _) = vacuum_parts();
        let motor = Motor::new("c".into(), 0.05, 0.3, 1.0, vec![(0.0, VAC_F), (VAC_BURN, VAC_F)]);
        let mut sim = vacuum_sim(&table, &motor);
        sim.mass = MassProperties { dry_mass: 10.0, cg_dry: 0.5, ixx_dry: 0.01, iyy_dry: 1.0, cg_lateral_dry: [0.02, 0.0], ..Default::default() };
        let ms = sim.mass_at(0.0);
        assert!(!ms.diagonal);
        let mp = motor.propellant_left(0.0);
        let total = 10.0 + mp;
        let xp = sim.motor_aft_x - 0.5 * motor.length;
        let lat = 10.0 * 0.02 / total;
        assert!((ms.cg_lat[0] - lat).abs() < 1e-15);
        // Ixy = -sum m dx dy about the common CG, d = r - r_cg in body axes (x = -aft).
        let (dx_d, dy_d) = (ms.x_cg - 0.5, 0.02 - lat);
        let (dx_p, dy_p) = (ms.x_cg - xp, -lat);
        let expect = -(10.0 * dx_d * dy_d + mp * dx_p * dy_p);
        assert!((ms.inertia.0[0][1] - expect).abs() < 1e-12, "{} vs {expect}", ms.inertia.0[0][1]);
        assert!(ms.inertia.0[0][1] != 0.0);
        assert!(ms.inertia.det() > 0.0);
    }

    /// Integrates the torque-free rotation of a state with the given integrator.
    fn spin_free(sim: &Simulation, i: IntegratorKind, at: AttitudeKind, w0: Vec3, tend: f64) -> (State, Vec3, f64) {
        let cx = Ctx {
            frame: Frame::new(&sim.earth, &sim.launch),
            rail: Vec3::new(0.0, 0.0, 1.0),
            cd_s: Default::default(),
            segment: std::cell::Cell::new(sim.motor.segment_after(100.0)),
            gravity_off: false,
        };
        let stepper = Stepper::new(i, at);
        let t0 = 100.0; // after burnout: no thrust, no propellant
        let mut s = State { pos: cx.frame.origin(), vel: Vec3::ZERO, q: Quat::IDENTITY, w: w0 };
        let mut f = |tt: f64, st: &State| sim.deriv(&cx, tt, st, Phase::Free);
        let mut t = 0.0;
        let mut h: f64 = 0.002;
        let (rtol, atol) = (1e-10, 1e-10);
        while t < tend - 1e-12 {
            let (k1, _) = f(t0 + t, &s);
            let hh = h.min(tend - t);
            let st = stepper.step(&mut f, t0 + t, &s, &k1, hh, rtol, atol, true);
            if !stepper.adaptive() {
                s = st.y;
                t += hh;
            } else if st.err <= 1.0 {
                s = st.y;
                t += hh;
                h = (hh * stepper.factor(st.err)).min(0.05);
            } else {
                h = hh * stepper.factor(st.err);
            }
        }
        let inertia = sim.mass_at(t0).inertia;
        // Inertial angular momentum, expressed in the frame.
        let l = s.q.to_mat().mul_vec(inertia.mul_vec(s.w));
        let e = 0.5 * s.w.dot(inertia.mul_vec(s.w));
        (s, l, e)
    }

    #[test]
    fn torque_free_asymmetric_body_conserves_energy_and_momentum() {
        let (table, motor) = vacuum_parts();
        let mut sim = vacuum_sim(&table, &motor);
        sim.mass = MassProperties {
            dry_mass: 5.0,
            cg_dry: 0.5,
            ixx_dry: 0.02,
            iyy_dry: 1.5,
            izz_dry: Some(1.8),
            ixy_dry: 0.05,
            ixz_dry: -0.03,
            iyz_dry: 0.1,
            cg_lateral_dry: [0.01, -0.02],
        };
        assert!(sim.mass_at(100.0).inertia.det() > 0.0);
        let w0 = Vec3::new(0.5, 3.0, 0.4);
        let l0 = sim.mass_at(100.0).inertia.mul_vec(w0).norm();
        let e0 = 0.5 * w0.dot(sim.mass_at(100.0).inertia.mul_vec(w0));
        for (i, at) in COMBOS {
            let (s, l, e) = spin_free(&sim, i, at, w0, 60.0);
            println!("{i:?}/{at:?}: |L| {:e} E {:e} |w| {}", l.norm() / l0 - 1.0, e / e0 - 1.0, s.w.norm());
            assert!((l.norm() / l0 - 1.0).abs() < 1e-6, "{i:?}/{at:?} |L|");
            assert!((e / e0 - 1.0).abs() < 1e-6, "{i:?}/{at:?} energy");
        }
    }

    #[test]
    fn misaligned_thrust_points_along_azimuth_and_pitches() {
        let (table, motor) = vacuum_parts();
        let aligned = vacuum_sim(&table, &motor);
        let mut mis = aligned.clone();
        mis.thrust_line = ThrustLine { misalignment_deg: 2.0, misalignment_azimuth_deg: 90.0, offset: [0.0, 0.0] };
        let cx = Ctx {
            frame: Frame::new(&aligned.earth, &aligned.launch),
            rail: Vec3::new(0.0, 0.0, 1.0),
            cd_s: Default::default(),
            segment: std::cell::Cell::new(aligned.motor.segment_after(1.0)),
            gravity_off: false,
        };
        let s = State { pos: cx.frame.origin(), vel: Vec3::ZERO, q: Quat::IDENTITY, w: Vec3::ZERO };
        let (a, _) = aligned.deriv(&cx, 1.0, &s, Phase::Free);
        let (b, _) = mis.deriv(&cx, 1.0, &s, Phase::Free);
        let ms = aligned.mass_at(1.0);
        let t = motor.thrust(1.0);
        let eps = 2.0f64.to_radians();
        // phi = 90 deg: lateral thrust component towards +z_b.
        assert!((b.vel.z - a.vel.z - t * eps.sin() / ms.m).abs() < 1e-9);
        assert!((b.vel.y - a.vel.y).abs() < 1e-9);
        assert!((b.vel.x - a.vel.x + t * (1.0 - eps.cos()) / ms.m).abs() < 1e-9);
        // The nozzle is aft of the CG: a +z_b force gives a +y_b moment.
        let lever = aligned.motor_aft_x - ms.x_cg;
        assert!((b.w.y - lever * t * eps.sin() / ms.inertia.0[1][1]).abs() < 1e-9);
        assert!(b.w.x.abs() < 1e-12 && b.w.z.abs() < 1e-9);
        // Zero misalignment adds no moment.
        assert_eq!(a.w.y, 0.0);
    }

    #[test]
    fn nozzle_offset_gives_moment_about_the_cg() {
        let (table, motor) = vacuum_parts();
        let mut sim = vacuum_sim(&table, &motor);
        sim.thrust_line = ThrustLine { misalignment_deg: 0.0, misalignment_azimuth_deg: 0.0, offset: [0.01, 0.0] };
        let cx = Ctx {
            frame: Frame::new(&sim.earth, &sim.launch),
            rail: Vec3::new(0.0, 0.0, 1.0),
            cd_s: Default::default(),
            segment: std::cell::Cell::new(sim.motor.segment_after(1.0)),
            gravity_off: false,
        };
        let s = State { pos: cx.frame.origin(), vel: Vec3::ZERO, q: Quat::IDENTITY, w: Vec3::ZERO };
        let (b, _) = sim.deriv(&cx, 1.0, &s, Phase::Free);
        // (r_n - r_cg) x (T, 0, 0) with r_n - r_cg = (-lever, 0.01, 0): z component = -0.01 T.
        let t = motor.thrust(1.0);
        assert!((b.w.z + 0.01 * t / sim.mass_at(1.0).inertia.0[2][2]).abs() < 1e-9);
    }

    #[test]
    fn general_table_flight_matches_axisymmetric_table() {
        let (table, motor) = sample_parts();
        let general = table.to_general(aero::table::angle_axis(180.0, 30.0, 2.0, 10.0), aero::table::angle_axis(90.0, 30.0, 2.0, 10.0));
        assert_eq!(general.kind(), TableKind::General);
        for wind in [0.0, 5.0] {
            let a = sample_sim(&table, &motor, wind).run(Descent::Ballistic, true).unwrap();
            let b = sample_sim(&general, &motor, wind).run(Descent::Ballistic, true).unwrap();
            println!(
                "wind {wind}: apogee {} vs {}, landing ({}, {}) vs ({}, {})",
                a.summary.apogee, b.summary.apogee, a.summary.landing_east, a.summary.landing_north, b.summary.landing_east, b.summary.landing_north
            );
            assert!((a.summary.apogee - b.summary.apogee).abs() < 0.5, "wind {wind} apogee");
            // Both margins are small-angle slopes over the first angle cell, 2 deg for the general
            // table and 1 deg for the axisymmetric one, so they differ by the grid resolution:
            // compare the general margin with its definition along the axisymmetric flight.
            let reference = a
                .samples
                .iter()
                .filter(|x| x.phase == Phase::Free && x.t <= a.summary.apogee_time && x.airspeed > 10.0)
                .map(|x| general.static_margins(x.mach, x.x_cg).min())
                .fold(f64::INFINITY, f64::min);
            println!("wind {wind}: margin {} (axisymmetric {}), reference {reference}", b.summary.min_stability_cal, a.summary.min_stability_cal);
            assert!((reference - b.summary.min_stability_cal).abs() < 0.02, "stability");
            assert!(b.samples.iter().all(|s| s.alpha_deg.is_finite() && s.beta_deg.is_finite()));
        }
    }

    /// The axisymmetric verdict uses the small-angle margin at the state's Mach number and CG, not the
    /// centre of pressure at the angle of attack the state flies at: a crosswind at the rail exit and
    /// the slow high-angle arc over apogee do not lower it, and the location of the minimum is kept.
    #[test]
    fn axisymmetric_margin_is_the_small_angle_margin() {
        let (table, motor) = sample_parts();
        let calm = sample_sim(&table, &motor, 0.0).run(Descent::Ballistic, true).unwrap();
        let windy = sample_sim(&table, &motor, 9.0).run(Descent::Ballistic, true).unwrap();
        for r in [&calm, &windy] {
            let s = &r.summary;
            let exit = r.samples.iter().find(|x| x.phase == Phase::Free).unwrap();
            let reference = r
                .samples
                .iter()
                .filter(|x| x.phase == Phase::Free && x.t <= s.apogee_time && x.airspeed > 10.0)
                .map(|x| table.static_margins(x.mach, x.x_cg).min())
                .fold(f64::INFINITY, f64::min);
            assert!((reference - s.static_margin_min()).abs() < 0.02, "{reference} {s:?}");
            assert_eq!(s.static_margin_min(), s.static_margin_pitch_min_cal);
            assert_eq!(s.static_margin_min(), s.static_margin_yaw_min_cal);
            // The minimum of the sample rocket is at the rail exit (the CG moves forward during the burn).
            assert_eq!(s.static_margin_min(), s.stability_at_rail_exit);
            assert_eq!(s.static_margin_min_t, s.rail_exit_time);
            assert!((s.static_margin_min_mach - exit.mach).abs() < 0.01, "{s:?} {exit:?}");
            assert_eq!(s.stability(), Stability::Stable);
        }
        // The wind changes the rail-exit Mach number only through the airspeed, not the margin.
        assert!((calm.summary.stability_at_rail_exit - windy.summary.stability_at_rail_exit).abs() < 0.01, "{:?}", (calm.summary, windy.summary));
        // The trajectory's display at the current angle of attack does drop with the wind angle
        // at the rail exit and over apogee.
        let display = |r: &SimResult| {
            r.samples.iter().filter(|x| x.phase == Phase::Free && x.t <= r.summary.apogee_time).map(|x| x.stability_cal).fold(f64::INFINITY, f64::min)
        };
        assert!(display(&windy) < windy.summary.static_margin_min() - 0.3, "{} {}", display(&windy), windy.summary.static_margin_min());
    }

    /// Ballistic descents broadside to the flow are timed; the sample rocket falls nose first.
    #[test]
    fn broadside_time_of_the_ballistic_descent() {
        let (table, motor) = sample_parts();
        let sim = sample_sim(&table, &motor, 3.0);
        let bal = sim.run(Descent::Ballistic, true).unwrap();
        let s = &bal.summary;
        // Reference: the recorded samples after apogee with alpha in 60..120 deg, each standing for an output interval.
        let dt = sim.settings.output_interval;
        let n = bal.samples.iter().filter(|x| x.t > s.apogee_time && x.phase == Phase::Free && (60.0..=120.0).contains(&x.alpha_deg)).count();
        assert!((s.broadside_time - n as f64 * dt).abs() <= 2.0 * dt + 0.05, "{} vs {}", s.broadside_time, n as f64 * dt);
        assert!(s.broadside_time < BROADSIDE_LIMIT && !s.fell_broadside(), "{s:?}");
        assert_eq!(sim.run(Descent::Parachute, false).unwrap().summary.broadside_time, 0.0);
        // A rocket with its CG far aft (CP ahead of the CG) falls partly broadside; its angle of attack
        // swings through the 60 deg bound, so the reference samples at 0.01 s.
        let mut tail_heavy = Simulation { mass: MassProperties::axisymmetric(8.0, 1.45, 0.02, 1.5), ..sample_sim(&table, &motor, 3.0) };
        tail_heavy.settings.output_interval = 0.01;
        let dt = tail_heavy.settings.output_interval;
        let t = tail_heavy.run(Descent::Ballistic, true).unwrap();
        let n = t.samples.iter().filter(|x| x.t > t.summary.apogee_time && (60.0..=120.0).contains(&x.alpha_deg)).count();
        assert!((t.summary.broadside_time - n as f64 * dt).abs() <= 2.0 * dt + 0.05, "{} vs {}", t.summary.broadside_time, n as f64 * dt);
        assert!(t.summary.broadside_time > 1.0, "{:?}", t.summary);
    }

    /// General table of the sample rocket with the yaw centre of pressure moved by `yaw_shift` [m] (aft
    /// positive) and the side force scaled by `cy_scale`.
    fn split_general(table: &AeroTable, yaw_shift: f64, cy_scale: f64) -> AeroTable {
        let g = table.to_general(aero::table::angle_axis(180.0, 30.0, 2.0, 10.0), aero::table::angle_axis(90.0, 30.0, 2.0, 10.0));
        let d = table.meta.ref_diameter;
        AeroTable::general_from_fn(g.meta.clone(), |m, a, b| {
            let mut c = g.lookup6(m, a.to_radians(), b.to_radians());
            c.cn -= yaw_shift * c.cy / d;
            c.cy *= cy_scale;
            c.cn *= cy_scale;
            c
        })
    }

    #[test]
    fn general_table_margins_are_evaluated_at_every_step_independent_of_recording() {
        let (table, motor) = sample_parts();
        // Yaw centre of pressure 0.05 m (1 cal) ahead of the pitch one.
        let general = split_general(&table, -table.meta.ref_diameter, 1.0);
        let mut sim = sample_sim(&general, &motor, 5.0);
        let quiet = sim.run(Descent::Ballistic, false).unwrap().summary;
        sim.settings.output_interval = 1.0;
        let coarse = sim.run(Descent::Ballistic, true).unwrap();
        sim.settings.output_interval = 0.002;
        let fine = sim.run(Descent::Ballistic, true).unwrap();
        for s in [&coarse.summary, &fine.summary] {
            assert_eq!(s.static_margin_pitch_min_cal, quiet.static_margin_pitch_min_cal);
            assert_eq!(s.static_margin_yaw_min_cal, quiet.static_margin_yaw_min_cal);
            assert_eq!(s.min_stability_cal, quiet.min_stability_cal);
            assert_eq!(s.stability_at_rail_exit, quiet.stability_at_rail_exit);
        }
        // The margins of the summary cover every step: no recorded sample is below them.
        let free = fine.samples.iter().filter(|x| x.phase == Phase::Free && x.t <= quiet.apogee_time && x.airspeed > 10.0);
        let (mut p_min, mut y_min) = (f64::INFINITY, f64::INFINITY);
        for x in free {
            let m = general.static_margins(x.mach, x.x_cg);
            (p_min, y_min) = (p_min.min(m.pitch), y_min.min(m.yaw));
        }
        assert!(quiet.static_margin_pitch_min_cal <= p_min + 1e-12 && quiet.static_margin_pitch_min_cal > p_min - 0.05, "{quiet:?} {p_min}");
        assert!(quiet.static_margin_yaw_min_cal <= y_min + 1e-12 && quiet.static_margin_yaw_min_cal > y_min - 0.05, "{quiet:?} {y_min}");
        // Yaw 1 cal below pitch; the overall margins are the smaller (yaw) one.
        assert!((quiet.static_margin_pitch_min_cal - quiet.static_margin_yaw_min_cal - 1.0).abs() < 0.05, "{quiet:?}");
        assert!(quiet.min_stability_cal.min(quiet.stability_at_rail_exit) == quiet.static_margin_yaw_min_cal);
        assert_eq!(quiet.stability(), Stability::Stable);
        // An axisymmetric table: pitch = yaw = the existing margin.
        let a = sample_sim(&table, &motor, 5.0).run(Descent::Ballistic, false).unwrap().summary;
        // The rail exit (above 10 m/s) is part of the free-flight minimum.
        assert!(a.rail_exit_speed > 10.0 && a.min_stability_cal <= a.stability_at_rail_exit, "{a:?}");
        let m = a.min_stability_cal.min(a.stability_at_rail_exit);
        assert_eq!((a.static_margin_pitch_min_cal, a.static_margin_yaw_min_cal), (m, m));
    }

    #[test]
    fn general_table_yaw_instability_and_unknown_stability_are_reported() {
        let (table, motor) = sample_parts();
        // Yaw centre of pressure 6 cal ahead: pitch stable, yaw unstable.
        let general = split_general(&table, -6.0 * table.meta.ref_diameter, 1.0);
        let s = sample_sim(&general, &motor, 0.0).run(Descent::Ballistic, false).unwrap().summary;
        assert!(s.static_margin_pitch_min_cal > 1.0 && s.static_margin_yaw_min_cal < 0.0, "{s:?}");
        assert!(matches!(s.stability(), Stability::Unstable(m) if m == s.static_margin_yaw_min_cal), "{:?}", s.stability());
        // No side force at all: the yaw margin cannot be evaluated.
        let general = split_general(&table, 0.0, 0.0);
        let s = sample_sim(&general, &motor, 0.0).run(Descent::Ballistic, false).unwrap().summary;
        assert!(s.static_margin_yaw_min_cal.is_nan() && s.static_margin_pitch_min_cal > 1.0, "{s:?}");
        let why = s.stability_unknown.clone().unwrap();
        assert!(why.contains("yaw normal-force slope dCy/dbeta is zero"), "{why}");
        assert!(matches!(s.stability(), Stability::Unknown(ref r) if *r == why));
        // The summary JSON has `null` for the unknown margin.
        let j = serde_json::to_value(&s).unwrap();
        assert!(j["static_margin_yaw_min_cal"].is_null() && j["stability_unknown"].is_string());
    }

    #[test]
    fn launch_roll_rotates_the_body_about_the_rail() {
        let (table, motor) = vacuum_parts();
        let mut sim = vacuum_sim(&table, &motor);
        sim.launch.elevation_deg = 80.0;
        let fr = Frame::new(&sim.earth, &sim.launch);
        let q0 = sim.initial_attitude(&fr).to_mat();
        sim.launch.roll_deg = 90.0;
        let q1 = sim.initial_attitude(&fr).to_mat();
        assert!((q1.col(0) - q0.col(0)).norm() < 1e-12);
        assert!((q1.col(1) - q0.col(2)).norm() < 1e-12);
        assert!((q1.col(2) - q0.col(1) * -1.0).norm() < 1e-12);
    }

    // ---- event handling: rail exit, competing events, max_time, thrust knots ----

    /// Vacuum sim with constant gravity (exact constant-acceleration phases for RK4).
    fn vacuum_const_g<'a>(table: &'a AeroTable, motor: &'a Motor, elev: f64, dt: f64) -> Simulation<'a> {
        let mut sim = vacuum_sim(table, motor);
        sim.launch = launch(elev);
        sim.earth.gravity = GravityModel::Constant;
        sim.settings.dt = dt;
        sim
    }

    #[test]
    fn free_flight_restarts_at_the_interpolated_rail_exit() {
        // Inclined rail in vacuum: on the rail a = F/m - g sin(e) along d; after the exit the
        // full gravity acts. Both phases are constant-acceleration, so RK4 is exact when the free
        // flight starts at the rail-exit instant (and not at the end of the overshooting step).
        let (table, motor) = vacuum_parts();
        let elev = 60f64;
        let sim = vacuum_const_g(&table, &motor, elev, 0.05);
        let r = sim.run(Descent::Ballistic, true).unwrap();
        let (se, ce) = elev.to_radians().sin_cos();
        let a_r = VAC_F / VAC_M - G0 * se;
        let te = (2.0 * 3.0 / a_r).sqrt();
        let ve = a_r * te;
        assert!((r.summary.rail_exit_time - te).abs() < 1e-9, "exit {} vs {te}", r.summary.rail_exit_time);
        assert!((r.summary.rail_exit_speed - ve).abs() < 1e-9, "exit speed {} vs {ve}", r.summary.rail_exit_speed);
        let mut checked = 0;
        for smp in r.samples.iter().filter(|s| s.phase == Phase::Free && s.t > te + 1e-6 && s.t < VAC_BURN - 1e-6) {
            let tau = smp.t - te;
            let along = 3.0 + ve * tau + 0.5 * VAC_F / VAC_M * tau * tau;
            let (north, up) = (along * ce, along * se - 0.5 * G0 * tau * tau);
            assert!((smp.north - north).abs() < 1e-7 && (smp.up - up).abs() < 1e-7, "t {}: ({}, {}) vs ({north}, {up})", smp.t, smp.north, smp.up);
            checked += 1;
        }
        assert!(checked > 5);
    }

    /// Vertical vacuum flight: apogee and touchdown times of the analytic solution.
    fn vacuum_times() -> (f64, f64) {
        let a = VAC_F / VAC_M - G0;
        let (v, h) = (a * VAC_BURN, 0.5 * a * VAC_BURN * VAC_BURN);
        let apo = h + v * v / (2.0 * G0);
        let t_apo = VAC_BURN + v / G0;
        (t_apo, t_apo + (2.0 * apo / G0).sqrt())
    }

    #[test]
    fn touchdown_before_deployment_in_the_same_step_wins() {
        // dt = 10 s puts the apogee and the touchdown into one step; the deployment instant
        // (apogee + delay) lies after the touchdown inside that step: no deployment.
        let (table, motor) = vacuum_parts();
        let mut sim = vacuum_const_g(&table, &motor, 90.0, 10.0);
        let (t_apo, t_land) = vacuum_times();
        sim.recovery = Recovery { enabled: true, cd_s: 1e-4, delay: t_land - t_apo + 0.1, main_cd_s: None, main_altitude: None };
        let s = sim.run(Descent::Parachute, false).unwrap().summary;
        assert_eq!(s.drogue_deploy_time, None);
        assert!((s.apogee_time - t_apo).abs() < 1e-9, "apogee {} vs {t_apo}", s.apogee_time);
        assert!((s.landing_time - t_land).abs() < 1e-9, "landing {} vs {t_land}", s.landing_time);
        // Deployment before the touchdown in the same step still deploys (a tiny canopy keeps the
        // 10 s step stable).
        sim.recovery.delay = t_land - t_apo - 1.0;
        let s = sim.run(Descent::Parachute, false).unwrap().summary;
        assert!((s.drogue_deploy_time.unwrap() - (t_apo + sim.recovery.delay)).abs() < 1e-9);
        assert!(s.landing_time > s.drogue_deploy_time.unwrap());
    }

    #[test]
    fn maxima_include_the_touchdown_state() {
        // Vacuum: the fall ends faster than the burnout speed, the fastest state is the touchdown.
        let (table, motor) = vacuum_parts();
        for integrator in [IntegratorKind::Rk4, IntegratorKind::Rk45] {
            let mut sim = vacuum_const_g(&table, &motor, 90.0, 0.5);
            sim.settings.integrator = integrator;
            sim.settings.output_interval = 0.5;
            let s = sim.run(Descent::Ballistic, false).unwrap().summary;
            let a = sim.atmosphere.at(sim.launch.altitude).sound_speed;
            let q = 0.5 * sim.atmosphere.at(sim.launch.altitude).density * s.landing_speed * s.landing_speed;
            assert!(s.max_speed >= s.landing_speed && s.max_speed - s.landing_speed < 1e-9, "{integrator:?}: {} vs {}", s.max_speed, s.landing_speed);
            assert!((s.max_mach - s.landing_speed / a).abs() < 1e-9 && (s.max_dyn_pressure - q).abs() < 1e-6 * q, "{s:?}");
        }
    }

    #[test]
    fn burnout_time_is_reported_only_when_the_flight_reaches_it() {
        let (table, motor) = vacuum_parts();
        let mut sim = vacuum_sim(&table, &motor);
        let s = sim.run(Descent::Ballistic, false).unwrap().summary;
        assert_eq!(s.burnout_time, VAC_BURN);
        // The run ends (sim.max_time) before the motor burns out: no burnout.
        sim.settings.max_time = 0.5 * VAC_BURN;
        let s = sim.run(Descent::Ballistic, false).unwrap().summary;
        assert!(s.burnout_time.is_nan(), "burnout {} after a run of {} s", s.burnout_time, sim.settings.max_time);
        assert!(serde_json::to_value(&s).unwrap()["burnout_time"].is_null());
        // Ending exactly at burnout reaches it.
        sim.settings.max_time = VAC_BURN;
        assert_eq!(sim.run(Descent::Ballistic, false).unwrap().summary.burnout_time, VAC_BURN);
    }

    #[test]
    fn no_landing_is_accepted_after_max_time() {
        let (table, motor) = vacuum_parts();
        let (_, t_land) = vacuum_times();
        for (integrator, dt) in [(IntegratorKind::Rk4, 0.5), (IntegratorKind::Rk45, 0.002)] {
            let mut sim = vacuum_const_g(&table, &motor, 90.0, dt);
            sim.settings.integrator = integrator;
            sim.settings.output_interval = 2.0;
            sim.settings.max_time = t_land - 0.01;
            let r = sim.run(Descent::Ballistic, true).unwrap();
            assert!(!r.summary.landed(), "{integrator:?}: landed at {} after max_time {}", r.summary.landing_time, sim.settings.max_time);
            assert!(r.samples.iter().all(|s| s.t <= sim.settings.max_time + 1e-9));
            sim.settings.max_time = t_land + 0.01;
            let s = sim.run(Descent::Ballistic, false).unwrap().summary;
            assert!(s.landed() && s.landing_time <= sim.settings.max_time, "{integrator:?}");
        }
    }

    #[test]
    fn fixed_steps_resolve_a_short_thrust_spike() {
        // 10 N s in 0.02 s; with dt = 0.05 every RK4 stage of the first step missed it.
        let (table, _) = vacuum_parts();
        let motor = Motor::new("spike".into(), 0.05, 0.3, 0.0, vec![(0.0, 0.0), (0.01, 1000.0), (0.02, 0.0), (1.0, 0.0)]);
        let mut sim = vacuum_const_g(&table, &motor, 90.0, 0.0005);
        sim.mass = MassProperties::axisymmetric(1.0, 0.5, 0.01, 1.0);
        let fine = sim.run(Descent::Ballistic, false).unwrap().summary;
        // Impulse 10 N s on 1 kg, minus gravity during the spike: apogee about v^2 / 2g.
        assert!((fine.apogee / (9.9f64.powi(2) / (2.0 * G0)) - 1.0).abs() < 0.02, "fine apogee {}", fine.apogee);
        sim.settings.dt = 0.05;
        let coarse = sim.run(Descent::Ballistic, false).unwrap().summary;
        // Not exact: at t = 0 the rail holds the rocket (thrust below weight), which the first
        // RK4 stage of a 0.01 s step sees for the whole step (~0.3 % of the apogee).
        assert!((coarse.apogee - fine.apogee).abs() < 1e-2 * fine.apogee, "coarse {} vs fine {}", coarse.apogee, fine.apogee);
    }

    #[test]
    fn dop853_tight_tolerance_through_burnout() {
        // The sample motor ends at full thrust: the power-on/off axial coefficient switches at
        // burnout. A step ending exactly at burnout must see the power-on side throughout.
        let (table, motor) = sample_parts();
        let mut b = sample_sim(&table, &motor, 0.0);
        b.settings.rtol = 1e-11;
        b.settings.atol = 1e-11;
        for i in [IntegratorKind::Rk45, IntegratorKind::Dop853] {
            let r = with(&b, i, AttitudeKind::Normalize).run(Descent::Ballistic, false);
            assert!(r.is_ok(), "{i:?}: {:#}", r.err().unwrap());
        }
    }

    #[test]
    fn touchdown_needs_a_bracket() {
        assert!(crosses_ground(10.0, -10.0) && crosses_ground(10.0, 0.0));
        assert!(!crosses_ground(-1.0, -2.0) && !crosses_ground(0.0, -2.0) && !crosses_ground(1.0, 2.0));
        // A rail at or below the horizon (or past the zenith) never gives a touchdown bracket: rejected.
        let (table, motor) = vacuum_parts();
        for elev in [0.0, -5.0, 90.5, f64::NAN] {
            let mut sim = vacuum_sim(&table, &motor);
            sim.launch.elevation_deg = elev;
            let e = sim.run(Descent::Ballistic, false).err().expect("rejected").to_string();
            assert!(e.contains("launch.elevation_deg must be in (0, 90]"), "{e}");
        }
    }

    #[test]
    fn deployment_waits_for_burnout_when_the_motor_still_burns() {
        // 300 N for 2 s, then a weak 70 N tail (below the weight of ~100 N) to 20 s: the apogee
        // (~16 s) comes while the motor still burns.
        let (table, _) = vacuum_parts();
        let motor = Motor::new("tail".into(), 0.05, 0.3, 1.0, vec![(0.0, 300.0), (2.0, 300.0), (2.01, 70.0), (19.99, 70.0), (20.0, 0.0)]);
        let mut sim = vacuum_sim(&table, &motor);
        sim.recovery = Recovery { enabled: true, cd_s: 1e-3, delay: 1.0, main_cd_s: None, main_altitude: None };
        let r = sim.run(Descent::Parachute, true).unwrap();
        let s = &r.summary;
        assert!(s.apogee_time + 1.0 < motor.burn_time(), "apogee {} not during the burn", s.apogee_time);
        // Deployment at burnout, not at apogee + delay; the delay is recorded with the thrust then.
        assert!((s.drogue_deploy_time.unwrap() - motor.burn_time()).abs() < 1e-9, "{:?}", s.drogue_deploy_time);
        let d = s.deploy_delayed.expect("delay recorded");
        assert!((d.nominal_time - (s.apogee_time + 1.0)).abs() < 1e-9 && (d.thrust - 70.0).abs() < 1e-9, "{d:?}");
        // Free flight with thrust up to burnout, the parachute phase without thrust or propellant left.
        for smp in &r.samples {
            if smp.phase == Phase::Parachute {
                assert!(smp.t >= motor.burn_time() - 1e-9 && smp.thrust == 0.0 && (smp.mass - VAC_M).abs() < 1e-9, "{smp:?}");
            } else if smp.t > 2.1 && smp.t < 19.9 {
                assert!((smp.thrust - 70.0).abs() < 1e-9, "{smp:?}");
            }
        }
        // A delay that ends after burnout is unchanged and not recorded; ballistic runs neither.
        sim.recovery.delay = 7.0;
        let s = sim.run(Descent::Parachute, false).unwrap().summary;
        assert!(s.deploy_delayed.is_none() && (s.drogue_deploy_time.unwrap() - (s.apogee_time + 7.0)).abs() < 1e-9, "{s:?}");
        assert!(sim.run(Descent::Ballistic, false).unwrap().summary.deploy_delayed.is_none());
    }

    #[test]
    fn main_deployment_state_is_at_the_main_altitude() {
        let (table, motor) = sample_parts();
        let mut sim = sample_sim(&table, &motor, 0.0);
        sim.settings.dt = 0.05;
        // A large drogue opening 8 s after apogee decelerates hard; the main altitude is crossed
        // shortly after, while the speed still changes quickly within a step.
        sim.recovery = Recovery { enabled: true, cd_s: 2.0, delay: 8.0, main_cd_s: None, main_altitude: None };
        let r = sim.run(Descent::Parachute, true).unwrap();
        let td = r.summary.drogue_deploy_time.unwrap();
        let h_d = r.samples.iter().rfind(|s| s.t <= td).unwrap().up;
        let main_alt = h_d - 40.0;
        sim.recovery.main_cd_s = Some(8.0);
        sim.recovery.main_altitude = Some(main_alt);
        sim.settings.output_interval = 1e-9; // a sample at every step start, also at the deployment
        let r = sim.run(Descent::Parachute, true).unwrap();
        let tm = r.summary.main_deploy_time.unwrap();
        let at = r.samples.iter().find(|s| (s.t - tm).abs() < 1e-12).expect("sample at the main deployment");
        let alt = r.summary.main_deploy_altitude.unwrap();
        assert!((at.up - main_alt).abs() < 1e-6, "state height {} vs main altitude {main_alt}", at.up);
        assert!((alt - at.up).abs() < 1e-9, "reported {alt} vs state {}", at.up);
        // Airspeed and dynamic pressure at the main opening: those of the state at the deployment.
        let (v, q) = (r.summary.main_deploy_speed.unwrap(), r.summary.main_deploy_q.unwrap());
        assert!((v - at.airspeed).abs() < 1e-9, "{v} vs {}", at.airspeed);
        let rho = sim.atmosphere.at(sim.launch.altitude + at.up).density;
        assert!((q - 0.5 * rho * v * v).abs() < 1e-9 * q.max(1.0));
        // The drogue opens 8 s after apogee, falling fast: its opening speed is recorded as well.
        let at_d = r.samples.iter().find(|s| (s.t - r.summary.drogue_deploy_time.unwrap()).abs() < 1e-12).expect("sample at the drogue deployment");
        assert!((r.summary.drogue_deploy_speed.unwrap() - at_d.airspeed).abs() < 1e-9);
        assert!(r.summary.drogue_deploy_speed.unwrap() > 50.0 && r.summary.drogue_deploy_q.unwrap() > 0.0, "{:?}", r.summary);
        // Ballistic runs open nothing.
        let b = sim.run(Descent::Ballistic, false).unwrap().summary;
        assert_eq!((b.drogue_deploy_speed, b.drogue_deploy_q, b.main_deploy_speed, b.main_deploy_q), (None, None, None, None));
    }
}
