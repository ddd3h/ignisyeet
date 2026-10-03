//! 6-DoF flight simulation. The integration frame is selected by the Earth model (see
//! `frame.rs`): a flat local ENU frame (x east, y north, z up) or the rotating ECEF frame.
//! Body frame: x forward along the axis towards the nose; the rocket is axisymmetric.

use crate::env::{is_pos, AtmosphereModel, Earth, IntegratorKind, Wind};
use crate::frame::Frame;
use crate::integrator::{self, State};
use crate::motor::Motor;
use aero::{AeroCoeffs, AeroTable};
use anyhow::{bail, Result};
use geom::{Mat3, Quat, Vec3};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MassProperties {
    /// Mass without propellant (including the empty motor casing) [kg].
    pub dry_mass: f64,
    /// Dry centre of gravity aft of the nose tip [m].
    pub cg_dry: f64,
    /// Dry roll inertia about the axis [kg m^2].
    pub ixx_dry: f64,
    /// Dry pitch/yaw inertia about the dry CG [kg m^2].
    pub iyy_dry: f64,
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
    /// Relative and absolute tolerances of the adaptive integrator.
    pub rtol: f64,
    pub atol: f64,
}

impl Default for Settings {
    fn default() -> Self {
        Self { dt: 0.002, max_time: 1200.0, output_interval: 0.05, integrator: IntegratorKind::Rk4, rtol: 1e-7, atol: 1e-6 }
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
        Quat::from_mat(&Mat3::from_cols(xb, yb, zb))
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

    /// Mass, CG and inertias (Ixx, Iyy) at time t.
    fn mass_at(&self, t: f64) -> (f64, f64, f64, f64) {
        let m = &self.mass;
        let mp = self.motor.propellant_left(t);
        let total = m.dry_mass + mp;
        let xp = self.motor_aft_x - 0.5 * self.motor.length;
        let xcg = (m.dry_mass * m.cg_dry + mp * xp) / total;
        let r = 0.5 * self.motor.diameter;
        let iyy = m.iyy_dry
            + m.dry_mass * (m.cg_dry - xcg).powi(2)
            + mp * (3.0 * r * r + self.motor.length.powi(2)) / 12.0
            + mp * (xp - xcg).powi(2);
        let ixx = m.ixx_dry + 0.5 * mp * r * r;
        (total, xcg, ixx, iyy)
    }

    /// State derivative. The frame supplies gravity, the apparent (Coriolis/centrifugal)
    /// accelerations, the local vertical for height and the ENU basis for the wind.
    fn deriv(&self, cx: &Ctx, t: f64, s: &State, phase: Phase) -> (State, Aux) {
        let thrust = self.motor.thrust(t);
        let (mass, x_cg, ixx, iyy) = self.mass_at(t);
        let fr = &cx.frame;
        let loc = fr.local(s.pos);
        let atm = self.atmosphere.at(self.launch.altitude + loc.height);
        let g = fr.gravity(s.pos, &loc, &self.earth) + fr.apparent_accel(s.pos, s.vel);
        let zero = State::ZERO_DERIV;
        let s_ref = self.table.meta.ref_area;
        let mut aux = Aux { thrust, mass, x_cg, x_cp: 0.0, airspeed: 0.0, mach: 0.0, alpha: 0.0, qdyn: 0.0, cn: 0.0, ca: 0.0 };

        match phase {
            Phase::Rail => {
                let d = cx.rail;
                let v = s.vel.dot(d);
                let mach = v.abs() / atm.sound_speed;
                let c = self.lookup(mach, 0.0);
                let ca = if thrust > 0.0 { c.ca_on } else { c.ca_off };
                let qd = 0.5 * atm.density * v * v;
                let mut a = (thrust - ca * qd * s_ref * v.signum()) / mass + g.dot(d);
                if v <= 0.0 && a < 0.0 {
                    a = 0.0;
                }
                aux.airspeed = v;
                aux.mach = mach;
                aux.qdyn = qd;
                aux.ca = ca;
                aux.x_cp = c.xcp;
                (State { pos: s.vel, vel: d * a, ..zero }, aux)
            }
            Phase::Free => {
                let rot = s.q.to_mat();
                let va = s.vel - loc.to_frame(self.wind.at(loc.height));
                let ub = rot.transpose_mul_vec(va);
                let v = ub.norm();
                let mut f_body = Vec3::new(thrust, 0.0, 0.0);
                let mut m_body = Vec3::ZERO;
                if v > 1e-6 {
                    let lat = ub.y.hypot(ub.z);
                    let alpha = lat.atan2(ub.x);
                    // Tail-first flight (alpha > 90 deg) mirrors the nose-first coefficients.
                    let alpha_eff = alpha.min(std::f64::consts::PI - alpha);
                    let mach = v / atm.sound_speed;
                    let c = self.lookup(mach, alpha_eff);
                    let qd = 0.5 * atm.density * v * v;
                    let ca = if thrust > 0.0 { c.ca_on } else { c.ca_off };
                    f_body += Vec3::new(-ca * qd * s_ref * ub.x.signum(), 0.0, 0.0);
                    if lat > 1e-12 {
                        let fn_b = Vec3::new(0.0, -ub.y / lat, -ub.z / lat) * (c.cn * qd * s_ref);
                        f_body += fn_b;
                        m_body += Vec3::new(x_cg - c.xcp, 0.0, 0.0).cross(fn_b);
                    }
                    let [s0, s1, s2] = c.damp;
                    let k = (s2 - 2.0 * x_cg * s1 + x_cg * x_cg * s0).max(0.0);
                    m_body += Vec3::new(0.0, s.w.y, s.w.z) * (-qd * s_ref * k / v);
                    aux = Aux { airspeed: v, mach, alpha, qdyn: qd, cn: c.cn, ca, x_cp: c.xcp, ..aux };
                }
                let acc = rot.mul_vec(f_body) / mass + g;
                // `s.w` is the angular velocity relative to the frame; the Euler equations need the
                // inertial one, w + R^T Omega (Omega = 0 in the flat frame). In body components
                // dw_rel/dt = dw_inertial/dt + w_rel x (R^T Omega).
                let om_b = rot.transpose_mul_vec(fr.omega());
                let wi = s.w + om_b;
                let iw = Vec3::new(ixx * wi.x, iyy * wi.y, iyy * wi.z);
                let gyro = wi.cross(iw);
                let dwi = Vec3::new((m_body.x - gyro.x) / ixx, (m_body.y - gyro.y) / iyy, (m_body.z - gyro.z) / iyy);
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
        let adaptive = self.settings.integrator == IntegratorKind::Rk45;
        if adaptive && !(is_pos(self.settings.rtol) && is_pos(self.settings.atol) && is_pos(self.settings.output_interval)) {
            bail!("sim.rtol, sim.atol and sim.output_interval must be positive for the rk45 integrator");
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
                s = integrator::rk4_step(&mut f, t, &s, &k1, dt);
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
                    let st = integrator::dopri5_step(&mut f, t, &s, &k1, hh, self.settings.rtol, self.settings.atol);
                    let fac = integrator::step_factor(st.err);
                    if st.err <= 1.0 {
                        s = st.y;
                        next_fsal = Some((st.k_end, st.aux_end));
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
        Ok(SimResult { samples, summary: sum, evals: evals.get() })
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
        Launch { latitude: 35.0, longitude: 139.0, altitude: 0.0, rail_length: 3.0, elevation_deg: elev, azimuth_deg: 0.0 }
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
        };
        let table = AeroTable::from_fn(meta, |_, _| AeroCoeffs { xcp: 0.5, ..Default::default() });
        let motor = Motor::new("c".into(), 0.05, 0.3, 0.0, vec![(0.0, VAC_F), (VAC_BURN, VAC_F)]);
        (table, motor)
    }

    fn vacuum_sim<'a>(table: &'a AeroTable, motor: &'a Motor) -> Simulation<'a> {
        Simulation {
            table,
            mass: MassProperties { dry_mass: VAC_M, cg_dry: 0.5, ixx_dry: 0.01, iyy_dry: 1.0 },
            motor,
            motor_aft_x: 1.0,
            launch: launch(90.0),
            recovery: Recovery { enabled: false, cd_s: 0.0, delay: 0.0 },
            wind: wind(0.0),
            atmosphere: AtmosphereModel::default(),
            earth: Earth::default(),
            settings: settings(),
            aero_scale: AeroScale::default(),
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
            mass: MassProperties { dry_mass: 8.0, cg_dry: 0.85, ixx_dry: 0.02, iyy_dry: 1.5 },
            motor,
            motor_aft_x: 1.5,
            launch: launch(85.0),
            recovery: Recovery { enabled: true, cd_s: 3.5, delay: 1.0 },
            wind: Wind::from_config(&WindConfig { speed: wind, direction_deg: 270.0, ..Default::default() }).unwrap(),
            atmosphere: AtmosphereModel::default(),
            earth: Earth::default(),
            settings: settings(),
            aero_scale: AeroScale::default(),
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
}
