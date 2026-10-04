// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025-2026 西濱大将 (NISHIHAMA Daisuke)
//! 6-DoF flight simulation. The integration frame is selected by the Earth model (see
//! `frame.rs`): a flat local ENU frame (x east, y north, z up) or the rotating ECEF frame.
//! Body frame: x forward along the axis towards the nose, y and z the body axes of the aerodynamic
//! table. Axisymmetric tables use the total angle of attack; general tables the six body-axis
//! coefficients of (alpha, beta). The mass distribution may carry a full inertia tensor, a lateral
//! CG offset and a misaligned thrust line.

use crate::env::{is_pos, AtmosphereModel, AttitudeKind, Earth, IntegratorKind, Wind};
use crate::frame::Frame;
use crate::integrator::{self, State};
use crate::stepper::Stepper;
use crate::motor::Motor;
use aero::table::{aero_angles, Coeffs6, TableKind};
use aero::{AeroCoeffs, AeroTable};
use anyhow::{bail, Result};
use geom::{Mat3, Quat, Vec3};
use serde::{Deserialize, Serialize};

/// Dry mass properties in body axes (x forward). The inertia entries are the elements of the
/// tensor `I = [[Ixx, Ixy, Ixz], [Ixy, Iyy, Iyz], [Ixz, Iyz, Izz]]` about the dry CG, so
/// `Ixy = -∫ x y dm` (the classical product of inertia with a minus sign).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
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
        self.ixy_dry == 0.0
            && self.ixz_dry == 0.0
            && self.iyz_dry == 0.0
            && self.izz_dry.map_or(true, |z| z == self.iyy_dry)
            && self.cg_lateral_dry == [0.0, 0.0]
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
#[serde(default)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recovery {
    pub enabled: bool,
    /// Parachute drag area Cd*S [m^2].
    pub cd_s: f64,
    /// Deployment delay after apogee [s].
    pub delay: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
        Self { dt: 0.002, max_time: 1200.0, output_interval: 0.05, integrator: IntegratorKind::Rk4, attitude: AttitudeKind::Normalize, rtol: 1e-7, atol: 1e-6 }
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
}

impl Default for AeroScale {
    fn default() -> Self {
        Self { cn: 1.0, ca: 1.0 }
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
    pub x_cp: f64,
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

#[derive(Debug, Clone, Serialize)]
pub struct Summary {
    pub descent: Descent,
    pub wind_speed: f64,
    pub wind_direction_deg: f64,
    pub rail_exit_time: f64,
    pub rail_exit_speed: f64,
    pub burnout_time: f64,
    pub max_speed: f64,
    pub max_mach: f64,
    pub max_dyn_pressure: f64,
    pub apogee_time: f64,
    /// Apogee above the launch site [m].
    pub apogee: f64,
    pub apogee_east: f64,
    pub apogee_north: f64,
    pub min_stability_cal: f64,
    pub stability_at_rail_exit: f64,
    pub landing_time: f64,
    pub landing_east: f64,
    pub landing_north: f64,
    pub landing_lat: f64,
    pub landing_lon: f64,
    pub landing_distance: f64,
    pub landing_speed: f64,
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
}

impl<'a> Simulation<'a> {
    /// Rail direction in launch-site ENU (x east, y north, z up).
    fn rail_dir_enu(&self) -> Vec3 {
        let (se, ce) = self.launch.elevation_deg.to_radians().sin_cos();
        let (sa, ca) = self.launch.azimuth_deg.to_radians().sin_cos();
        Vec3::new(ce * sa, ce * ca, se)
    }

    /// Body-to-frame attitude on the rail.
    fn initial_attitude(&self, fr: &Frame) -> Quat {
        let xb = fr.from_launch_enu(self.rail_dir_enu());
        let az = self.launch.azimuth_deg.to_radians();
        let yb = fr.from_launch_enu(Vec3::new(az.cos(), -az.sin(), 0.0));
        let zb = xb.cross(yb);
        if self.launch.roll_deg == 0.0 {
            return Quat::from_mat(&Mat3::from_cols(xb, yb, zb));
        }
        // Roll about the rail axis: y' = cos y + sin z, z' = x cross y'.
        let (sr, cr) = self.launch.roll_deg.to_radians().sin_cos();
        let yr = yb * cr + zb * sr;
        Quat::from_mat(&Mat3::from_cols(xb, yr, xb.cross(yr)))
    }

    /// Table lookup with the dispersion multipliers applied.
    fn lookup(&self, mach: f64, alpha: f64) -> AeroCoeffs {
        let mut c = self.table.lookup(mach, alpha);
        let AeroScale { cn, ca } = self.aero_scale;
        c.cn *= cn;
        c.cna *= cn;
        c.damp = c.damp.map(|d| d * cn);
        c.ca_on *= ca;
        c.ca_off *= ca;
        c
    }

    /// General-table lookup with the dispersion multipliers applied (`cn` scales every normal-force
    /// and moment coefficient and the damping sums, `ca` the axial force).
    fn lookup6(&self, mach: f64, alpha: f64, beta: f64) -> Coeffs6 {
        let mut c = self.table.lookup6(mach, alpha, beta);
        let AeroScale { cn, ca } = self.aero_scale;
        c.cy *= cn;
        c.cz *= cn;
        c.cm *= cn;
        c.cn *= cn;
        c.sq = c.sq.map(|d| d * cn);
        c.sr = c.sr.map(|d| d * cn);
        c.cx_on *= ca;
        c.cx_off *= ca;
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

    /// Mass, CG and inertia at time t.
    fn mass_at(&self, t: f64) -> MassState {
        let m = &self.mass;
        let mp = self.motor.propellant_left(t);
        let total = m.dry_mass + mp;
        let xp = self.motor_aft_x - 0.5 * self.motor.length;
        let xcg = (m.dry_mass * m.cg_dry + mp * xp) / total;
        let r = 0.5 * self.motor.diameter;
        if m.is_axisymmetric() {
            let iyy = m.iyy_dry
                + m.dry_mass * (m.cg_dry - xcg).powi(2)
                + mp * (3.0 * r * r + self.motor.length.powi(2)) / 12.0
                + mp * (xp - xcg).powi(2);
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

    /// State derivative. The frame supplies gravity, the apparent (Coriolis/centrifugal)
    /// accelerations, the local vertical for height and the ENU basis for the wind.
    fn deriv(&self, cx: &Ctx, t: f64, s: &State, phase: Phase) -> (State, Aux) {
        let thrust = self.motor.thrust(t);
        let ms = self.mass_at(t);
        let (mass, x_cg) = (ms.m, ms.x_cg);
        let fr = &cx.frame;
        let loc = fr.local(s.pos);
        let atm = self.atmosphere.at(self.launch.altitude + loc.height);
        let g = fr.gravity(s.pos, &loc, &self.earth) + fr.apparent_accel(s.pos, s.vel);
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
                    (-(if thrust > 0.0 { c.cx_on } else { c.cx_off }), self.eff_xcp(&c, mach).unwrap_or(f64::NAN))
                } else {
                    let c = self.lookup(mach, 0.0);
                    (if thrust > 0.0 { c.ca_on } else { c.ca_off }, c.xcp)
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
                if v > 1e-6 {
                    let lat = ub.y.hypot(ub.z);
                    let alpha = lat.atan2(ub.x);
                    let beta = (ub.y / v).clamp(-1.0, 1.0).asin();
                    let d_ref = self.table.meta.ref_diameter;
                    let d_cant = (self.fin_cant_deg - self.table.meta.fin_cant_ref_deg).to_radians();
                    let p_hat = s.w.x * d_ref / (2.0 * v);
                    if self.table.kind() == TableKind::General {
                        // Six body-axis coefficients of (alpha, beta); no tail-first mirroring.
                        let (alpha_g, beta_g) = aero_angles(ub.x, ub.y, ub.z);
                        let mach = v / atm.sound_speed;
                        let c = self.lookup6(mach, alpha_g, beta_g);
                        let qd = 0.5 * atm.density * v * v;
                        let qs = qd * s_ref;
                        let cx = if thrust > 0.0 { c.cx_on } else { c.cx_off };
                        let f_aero = Vec3::new(cx, c.cy, c.cz) * qs;
                        f_body += f_aero;
                        let cl = c.cl + c.cl_delta * d_cant + c.clp * p_hat;
                        // Table moments are about the nose tip; transfer to the CG at (-x_cg, y, z).
                        m_body += Vec3::new(cl, c.cm, c.cn) * (qs * d_ref) + Vec3::new(x_cg, -ms.cg_lat[0], -ms.cg_lat[1]).cross(f_aero);
                        // Rate damping about the CG from the sums about the nose; the lateral CG
                        // offset is neglected here.
                        let kq = (c.sq[2] - 2.0 * x_cg * c.sq[1] + x_cg * x_cg * c.sq[0]).max(0.0);
                        let kr = (c.sr[2] - 2.0 * x_cg * c.sr[1] + x_cg * x_cg * c.sr[0]).max(0.0);
                        m_body += Vec3::new(0.0, -s.w.y * kq, -s.w.z * kr) * (qs / v);
                        // Effective centre of pressure of the transverse force (for the stability margin).
                        let x_cp = self.eff_xcp(&c, mach).unwrap_or(f64::NAN);
                        aux = Aux { airspeed: v, mach, alpha, qdyn: qd, cn: c.cy.hypot(c.cz), ca: -cx, x_cp, beta, cl, cy: c.cy, ..aux };
                    } else {
                    // Tail-first flight (alpha > 90 deg) mirrors the nose-first coefficients.
                    let alpha_eff = alpha.min(std::f64::consts::PI - alpha);
                    let mach = v / atm.sound_speed;
                    let c = self.lookup(mach, alpha_eff);
                    let qd = 0.5 * atm.density * v * v;
                    let ca = if thrust > 0.0 { c.ca_on } else { c.ca_off };
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
                    // Roll: the tabulated forcing at the reference cant (zero for a rocket without
                    // cant), the change of cant and the roll damping.
                    let sgn = if ub.x >= 0.0 { 1.0 } else { -1.0 };
                    let cl = c.cl_delta * (self.table.meta.fin_cant_ref_deg.to_radians() * sgn + d_cant) + c.clp * p_hat;
                    m_body += Vec3::new(qd * s_ref * d_ref * cl, 0.0, 0.0);
                    if ms.cg_lat != [0.0, 0.0] {
                        // The aerodynamic force acts on the axis; the CG is off it.
                        m_body += Vec3::new(0.0, -ms.cg_lat[0], -ms.cg_lat[1]).cross(f_aero);
                    }
                    aux = Aux { airspeed: v, mach, alpha, qdyn: qd, cn: c.cn, ca, x_cp: c.xcp, beta, cl, cy: 0.0, ..aux };
                    }
                }
                let acc = rot.mul_vec(f_body) / mass + g;
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
                    ms.inv.unwrap_or(Mat3::identity()).mul_vec(m_body - gyro)
                };
                let dw = dwi + s.w.cross(om_b);
                (State { pos: s.vel, vel: acc, q: s.q.derivative(s.w), w: dw }, aux)
            }
            Phase::Parachute => {
                let va = s.vel - loc.to_frame(self.wind.at(loc.height));
                let v = va.norm();
                let drag = va * (-0.5 * atm.density * v * self.recovery.cd_s / mass);
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
        let stepper = Stepper::new(self.settings.integrator, self.settings.attitude);
        let adaptive = stepper.adaptive();
        if adaptive && !(is_pos(self.settings.rtol) && is_pos(self.settings.atol) && is_pos(self.settings.output_interval)) {
            bail!("sim.rtol, sim.atol and sim.output_interval must be positive for the adaptive integrators (rk45, dop853)");
        }
        let frame = Frame::new(&self.earth, &self.launch);
        let cx = Ctx { frame, rail: frame.from_launch_enu(self.rail_dir_enu()) };
        let mut s = State { pos: frame.origin(), vel: Vec3::ZERO, q: self.initial_attitude(&frame), w: Vec3::ZERO };
        let mut phase = Phase::Rail;
        let mut t = 0.0;
        let mut samples = Vec::new();
        let mut next_out = 0.0;
        let ref_d = self.table.meta.ref_diameter;
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
            landing_time: f64::NAN,
            landing_east: f64::NAN,
            landing_north: f64::NAN,
            landing_lat: f64::NAN,
            landing_lon: f64::NAN,
            landing_distance: f64::NAN,
            landing_speed: f64::NAN,
        };
        let mut deploy_at = f64::INFINITY;
        let mut past_apogee = false;
        let evals = std::cell::Cell::new(0u64);
        let mut h_ctrl = dt;
        let mut max_qerr = 0.0f64;
        // Derivative at (t, s) carried over from the previous step (first-same-as-last).
        let mut fsal: Option<(State, Aux)> = None;
        let mut loc = frame.local(s.pos);

        loop {
            let mut f = |tt: f64, st: &State| {
                evals.set(evals.get() + 1);
                self.deriv(&cx, tt, st, phase)
            };
            let (k1, aux) = fsal.take().unwrap_or_else(|| f(t, &s));
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
            if phase == Phase::Free && !past_apogee && aux.airspeed > 10.0 {
                sum.min_stability_cal = sum.min_stability_cal.min((aux.x_cp - aux.x_cg) / ref_d);
            }
            if t > self.settings.max_time {
                break;
            }
            if phase == Phase::Rail && t > burn {
                bail!("rocket never left the launch rail (thrust too low?)");
            }

            let prev = s;
            let h;
            let mut next_fsal = None;
            if !adaptive {
                h = dt;
                s = stepper.step(&mut f, t, &s, &k1, dt, 0.0, 0.0, phase == Phase::Free).y;
            } else {
                // Largest allowed step: output interval, 0.05 s on the rail and during the burn,
                // trimmed to thrust-curve knots and to the parachute deployment instant.
                let mut hmax = self.settings.output_interval;
                if phase == Phase::Rail || t < burn {
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
                if t < burn {
                    if let Some(&(tk, _)) = self.motor.curve.iter().find(|p| p.0 > t + 1e-9) {
                        hmax = hmax.min(tk - t);
                    }
                }
                if descent == Descent::Parachute && phase == Phase::Free && deploy_at > t + 1e-12 {
                    hmax = hmax.min(deploy_at - t);
                }
                let mut hh = h_ctrl.min(hmax);
                let mut rejected = false;
                loop {
                    let st = stepper.step(&mut f, t, &s, &k1, hh, self.settings.rtol, self.settings.atol, phase == Phase::Free);
                    let fac = stepper.factor(st.err);
                    if st.err <= 1.0 {
                        s = st.y;
                        next_fsal = st.next;
                        // A step trimmed by a limit must not shrink the controller's step.
                        h_ctrl = if hh < h_ctrl && !rejected { h_ctrl.max(hh * fac) } else { hh * fac };
                        break;
                    }
                    rejected = true;
                    hh *= fac;
                    integrator::check_min_step(hh, t)?;
                }
                h = hh;
            }
            max_qerr = max_qerr.max((crate::lie::quat_norm(s.q) - 1.0).abs());
            t += h;
            let loc_prev = loc;
            loc = frame.local(s.pos);

            let before = phase;
            if phase == Phase::Rail && (s.pos - frame.origin()).dot(cx.rail) >= self.launch.rail_length {
                phase = Phase::Free;
                sum.rail_exit_time = t;
                sum.rail_exit_speed = s.vel.norm();
                let (_, aux) = self.deriv(&cx, t, &s, phase);
                sum.stability_at_rail_exit = (aux.x_cp - aux.x_cg) / ref_d;
            }
            if loc.height > sum.apogee {
                sum.apogee = loc.height;
                sum.apogee_time = t;
                let enu = frame.position_to_launch_enu(s.pos);
                sum.apogee_east = enu.x;
                sum.apogee_north = enu.y;
            }
            let (vz_prev, vz) = (prev.vel.dot(loc_prev.up), s.vel.dot(loc.up));
            if phase != Phase::Rail && !past_apogee && vz_prev > 0.0 && vz <= 0.0 {
                past_apogee = true;
                // The adaptive integrator takes long steps, so it uses the zero crossing of the
                // vertical velocity (linear within the step); rk4 keeps the end of the step.
                let t_apo = if adaptive { t - h + h * vz_prev / (vz_prev - vz) } else { t };
                deploy_at = t_apo + self.recovery.delay;
            }
            if descent == Descent::Parachute && phase == Phase::Free && t >= deploy_at {
                phase = Phase::Parachute;
            }
            if phase == before {
                fsal = next_fsal;
            }
            if phase != Phase::Rail && loc.height <= 0.0 && t > sum.rail_exit_time {
                let f = loc_prev.height / (loc_prev.height - loc.height);
                let p = prev.pos + (s.pos - prev.pos) * f;
                let enu = frame.position_to_launch_enu(p);
                let (lat, lon, _) = frame.lla(p);
                sum.landing_time = t - h + f * h;
                sum.landing_east = enu.x;
                sum.landing_north = enu.y;
                sum.landing_lat = lat;
                sum.landing_lon = lon;
                sum.landing_distance = enu.x.hypot(enu.y);
                sum.landing_speed = s.vel.norm();
                if want_samples {
                    let (_, aux) = self.deriv(&cx, t, &s, phase);
                    let mut last = self.sample(&cx, sum.landing_time, &State { pos: p, ..s }, phase, &aux);
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
        }
        if !sum.min_stability_cal.is_finite() {
            sum.min_stability_cal = f64::NAN;
        }
        Ok(SimResult { samples, summary: sum, evals: evals.get(), max_quat_norm_error: max_qerr })
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
            recovery: Recovery { enabled: false, cd_s: 0.0, delay: 0.0 },
            wind: wind(0.0),
            atmosphere: AtmosphereModel::default(),
            earth: Earth::default(),
            settings: settings(),
            aero_scale: AeroScale::default(),
            thrust_line: ThrustLine::default(),
            fin_cant_deg: 0.0,
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

    fn sample_sim<'a>(table: &'a AeroTable, motor: &'a Motor, wind: f64) -> Simulation<'a> {
        Simulation {
            table,
            mass: MassProperties::axisymmetric(8.0, 0.85, 0.02, 1.5),
            motor,
            motor_aft_x: 1.5,
            launch: launch(85.0),
            recovery: Recovery { enabled: true, cd_s: 3.5, delay: 1.0 },
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
            println!("{name}: rk4 apogee {} land ({}, {}) t {} evals {} | rk45 apogee {} land ({}, {}) t {} evals {} samples {}", sa.apogee, sa.landing_east, sa.landing_north, sa.landing_time, a.evals, sb.apogee, sb.landing_east, sb.landing_north, sb.landing_time, b.evals, b.samples.len());
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
                println!("{name} {descent:?} {i:?}/{at:?}: apogee {:.3} ({:+.3}) landing ({:.2}, {:.2}) d {:.3} evals {}", sm.apogee, sm.apogee - refs.apogee, sm.landing_east, sm.landing_north, d_land, r.evals);
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
        TableMeta { ref_area: 0.01, ref_diameter: 0.1, length: 1.0, machs, alphas_deg: vec![0.0, 90.0], extrapolation: Extrapolation::Clamp, ..Default::default() }
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
        let cx = Ctx { frame: Frame::new(&sim.earth, &sim.launch), rail: Vec3::new(0.0, 0.0, 1.0) };
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
        sim.mass = MassProperties { dry_mass: 5.0, cg_dry: 0.5, ixx_dry: 0.02, iyy_dry: 1.5, izz_dry: Some(1.8), ixy_dry: 0.05, ixz_dry: -0.03, iyz_dry: 0.1, cg_lateral_dry: [0.01, -0.02] };
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
        let cx = Ctx { frame: Frame::new(&aligned.earth, &aligned.launch), rail: Vec3::new(0.0, 0.0, 1.0) };
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
        let cx = Ctx { frame: Frame::new(&sim.earth, &sim.launch), rail: Vec3::new(0.0, 0.0, 1.0) };
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
            println!("wind {wind}: apogee {} vs {}, landing ({}, {}) vs ({}, {})", a.summary.apogee, b.summary.apogee, a.summary.landing_east, a.summary.landing_north, b.summary.landing_east, b.summary.landing_north);
            assert!((a.summary.apogee - b.summary.apogee).abs() < 0.5, "wind {wind} apogee");
            assert!((a.summary.min_stability_cal - b.summary.min_stability_cal).abs() < 0.1, "stability");
            assert!(b.samples.iter().all(|s| s.alpha_deg.is_finite() && s.beta_deg.is_finite()));
        }
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
}
