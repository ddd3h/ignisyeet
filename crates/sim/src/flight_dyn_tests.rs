// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025-2026 西濱大将 (NISHIHAMA Daisuke)
//! Tests of the rate- and acceleration-dependent aerodynamics of the flight model: lag (alpha-dot)
//! moment, rotation-induced normal force, Magnus force and moment, and the roll-resonance check,
//! against the linear theory of the free oscillation (Etkin, Dynamics of Flight) and Murphy's
//! epicyclic theory of a spinning body (Murphy, BRL Report 1216; McCoy, Modern Exterior
//! Ballistics, ch. 10).

use super::*;
use crate::env::{WindConfig, WindModel};
use aero::table::{TableMeta, Extrapolation};
use aero::{AeroModel, AeroOptions};
use geom::sample::SampleRocket;
use geom::{extract, ExtractOptions};

// ---------------------------------------------------------------------------------------------
// Complex arithmetic for the mode fits.

#[derive(Debug, Clone, Copy, PartialEq)]
struct C(f64, f64);

impl C {
    fn add(self, o: C) -> C {
        C(self.0 + o.0, self.1 + o.1)
    }
    fn sub(self, o: C) -> C {
        C(self.0 - o.0, self.1 - o.1)
    }
    fn mul(self, o: C) -> C {
        C(self.0 * o.0 - self.1 * o.1, self.0 * o.1 + self.1 * o.0)
    }
    fn scale(self, k: f64) -> C {
        C(self.0 * k, self.1 * k)
    }
    fn conj(self) -> C {
        C(self.0, -self.1)
    }
    fn abs(self) -> f64 {
        self.0.hypot(self.1)
    }
    fn div(self, o: C) -> C {
        let d = o.0 * o.0 + o.1 * o.1;
        self.mul(o.conj()).scale(1.0 / d)
    }
    fn sqrt(self) -> C {
        let r = self.abs().sqrt();
        let a = 0.5 * self.1.atan2(self.0);
        C(r * a.cos(), r * a.sin())
    }
    fn ln(self) -> C {
        C(self.abs().ln(), self.1.atan2(self.0))
    }
    fn expi(phi: f64) -> C {
        C(phi.cos(), phi.sin())
    }
}

/// Roots of `z^2 + b z + c = 0`.
fn quad_roots(b: C, c: C) -> [C; 2] {
    let disc = b.mul(b).sub(c.scale(4.0)).sqrt();
    [b.scale(-1.0).add(disc).scale(0.5), b.scale(-1.0).sub(disc).scale(0.5)]
}

/// Two-mode Prony fit of uniformly sampled `x` (spacing `dt`): the exponents `lambda` of
/// `x_n = A e^(lambda_1 n dt) + B e^(lambda_2 n dt)` from the least-squares linear prediction
/// `x_{n+2} = c1 x_{n+1} + c2 x_n`.
fn prony2(x: &[C], dt: f64) -> [C; 2] {
    let (mut a11, mut a12, mut a21, mut a22, mut b1, mut b2) = (C(0.0, 0.0), C(0.0, 0.0), C(0.0, 0.0), C(0.0, 0.0), C(0.0, 0.0), C(0.0, 0.0));
    for w in x.windows(3) {
        let (x0, x1, x2) = (w[0], w[1], w[2]);
        a11 = a11.add(x1.conj().mul(x1));
        a12 = a12.add(x1.conj().mul(x0));
        a21 = a21.add(x0.conj().mul(x1));
        a22 = a22.add(x0.conj().mul(x0));
        b1 = b1.add(x1.conj().mul(x2));
        b2 = b2.add(x0.conj().mul(x2));
    }
    let det = a11.mul(a22).sub(a12.mul(a21));
    let c1 = b1.mul(a22).sub(a12.mul(b2)).div(det);
    let c2 = a11.mul(b2).sub(b1.mul(a21)).div(det);
    // z^2 - c1 z - c2 = 0
    quad_roots(c1.scale(-1.0), c2.scale(-1.0)).map(|z| z.ln().scale(1.0 / dt))
}

// ---------------------------------------------------------------------------------------------
// Free-flight harness: no gravity, no thrust, no drag, constant density and speed.

const RHO: f64 = 1.2;
const V: f64 = 100.0;
const S: f64 = 0.01;
const D: f64 = 0.1;

#[derive(Debug, Clone, Copy)]
struct Body {
    cna: f64,
    xcp: f64,
    damp: [f64; 3],
    lag: (f64, f64),
    cnpa: f64,
    mass: f64,
    x_cg: f64,
    ixx: f64,
    iyy: f64,
}

/// Body of the non-spinning oscillation: CNa 10 /rad at 1.0 m, CG 0.5 m, 0.5 kg, I = 1 kg m^2;
/// apparent mass L0 = 10 m at 0.8 m (strong, to resolve the lag term).
fn osc_body() -> Body {
    Body { cna: 10.0, xcp: 1.0, damp: [10.0, 10.0, 11.0], lag: (10.0, 8.0), cnpa: 0.0, mass: 0.5, x_cg: 0.5, ixx: 0.1, iyy: 1.0 }
}

fn table_of(b: &Body) -> AeroTable {
    let meta = TableMeta {
        ref_area: S,
        ref_diameter: D,
        length: 2.0,
        machs: vec![0.0, 5.0],
        alphas_deg: vec![0.0, 10.0, 90.0],
        extrapolation: Extrapolation::Clamp,
        ..Default::default()
    };
    let b = *b;
    AeroTable::from_fn(meta, move |_, a| AeroCoeffs {
        cn: b.cna * a.to_radians(),
        cna: b.cna,
        xcp: b.xcp,
        damp: b.damp,
        lag_l0: b.lag.0,
        lag_l1: b.lag.1,
        cnpa: b.cnpa,
        ..Default::default()
    })
}

fn no_thrust() -> Motor {
    Motor::new("none".into(), 0.05, 0.3, 0.0, vec![(0.0, 0.0), (0.1, 0.0)])
}

fn base_sim<'a>(table: &'a AeroTable, motor: &'a Motor, b: &Body, wind: Wind, atmosphere: AtmosphereModel) -> Simulation<'a> {
    Simulation {
        table,
        mass: MassProperties::axisymmetric(b.mass, b.x_cg, b.ixx, b.iyy),
        motor,
        motor_aft_x: 2.0,
        launch: Launch { latitude: 35.0, longitude: 139.0, altitude: 0.0, rail_length: 3.0, elevation_deg: 45.0, azimuth_deg: 0.0, roll_deg: 0.0 },
        recovery: Recovery { enabled: false, cd_s: 0.0, delay: 0.0, main_cd_s: None, main_altitude: None },
        wind,
        atmosphere,
        earth: Earth::default(),
        settings: Settings::default(),
        aero_scale: AeroScale::default(),
        thrust_line: ThrustLine::default(),
        fin_cant_deg: 0.0,
    }
}

fn calm() -> Wind {
    Wind::from_config(&WindConfig { speed: 0.0, ..Default::default() }).unwrap()
}

fn ctx(sim: &Simulation, gravity_off: bool) -> Ctx {
    let frame = Frame::new(&sim.earth, &sim.launch);
    Ctx { frame, rail: Vec3::new(0.0, 0.0, 1.0), cd_s: Default::default(), segment: std::cell::Cell::new(sim.motor.segment_after(100.0)), gravity_off }
}

/// Integrates the free flight (RK4, step `h`) from an angle of attack `alpha0` [rad] in the x-z
/// plane of the body and roll rate `p0`; returns the crossflow `(v + i w) / V` in non-rolling axes,
/// sampled every `every` steps (spacing `every h`) for `n` samples.
fn fly(b: &Body, scale: AeroScale, alpha0: f64, p0: f64, h: f64, every: usize, n: usize) -> Vec<C> {
    let table = table_of(b);
    let motor = no_thrust();
    let atm = AtmosphereModel::constant(RHO, 340.0, 1.8e-5).unwrap();
    let mut sim = base_sim(&table, &motor, b, calm(), atm);
    sim.aero_scale = scale;
    let cx = ctx(&sim, true);
    let mut s = State { pos: cx.frame.origin(), vel: Vec3::new(V * alpha0.cos(), 0.0, V * alpha0.sin()), q: Quat::IDENTITY, w: Vec3::new(p0, 0.0, 0.0) };
    let mut f = |t: f64, st: &State| sim.deriv(&cx, 100.0 + t, st, Phase::Free);
    let mut out = Vec::with_capacity(n);
    let mut t = 0.0;
    let mut phi = 0.0;
    for k in 0..n * every {
        if k % every == 0 {
            let ub = s.q.to_mat().transpose_mul_vec(s.vel);
            let xb = C(ub.y, ub.z).scale(1.0 / ub.norm());
            // Body axes are rolled by phi from the non-rolling axes: xi_nr = xi_b e^(i phi).
            out.push(xb.mul(C::expi(phi)));
        }
        let (k1, _) = f(t, &s);
        let p = s.w.x;
        s = integrator::rk4_step(&mut f, t, &s, &k1, h);
        phi += 0.5 * (p + s.w.x) * h;
        t += h;
    }
    assert!((s.w.x - p0).abs() < 1e-9 * p0.abs().max(1.0), "roll rate drifted: {} vs {p0}", s.w.x);
    assert!((s.vel.norm() / V - 1.0).abs() < 2e-3, "speed drifted: {}", s.vel.norm());
    out
}

/// Linear theory of the crossflow `xi = (v + i w) / V` in non-rolling axes for the harness:
/// `xi'' + B xi' + K xi = 0` (Murphy's `H - i P` and `-(M + i P T)`), from
///
/// * `xi' = -(a + i f_m) xi + i (1 - c_q) mu` (normal force, Magnus force, rotation-induced force),
/// * `mu' = (i M0 - m_m) xi - h mu + i P mu - i g xi'` (static, Magnus, damping and lag moments,
///   gyroscopic coupling), `mu = q + i r`,
///
/// with `a = CNa q S / (m V)`, `h = q S k / (V I)`, `k` the damping sum about the CG,
/// `c_q = q S (S1 - x_cg S0) / (m V^2)`, `M0 = CNa q S (x_cp - x_cg) / I`,
/// `g = (rho S V / 2)(x_cg L0 - L1) / I`, `f_m = q S cnpa p_hat / (m V)`,
/// `m_m = (x_cp - x_cg) q S cnpa p_hat / I`, `P = Ixx p / I`. Eliminating mu:
/// `B = a + h - (1 - c_q) g - i (P - f_m)` and
/// `K = (1 - c_q) M0 + h a + P f_m + i ((1 - c_q) m_m - P a + h f_m)`.
/// Without spin, `B = -[rho V S d^2 (Cmq + Cmalphadot) / (4 I) - rho V S CNa / (2 m)]` (with
/// `c_q = 0`) and `K = omega^2`, the classical short-period equation.
fn theory(b: &Body, scale: AeroScale, p: f64) -> (C, C) {
    let qs = 0.5 * RHO * V * V * S;
    let [s0, s1, s2] = b.damp;
    let x = b.x_cg;
    let k = s2 - 2.0 * x * s1 + x * x * s0;
    let a = b.cna * qs / (b.mass * V);
    let h = qs * k / (V * b.iyy);
    let cq = scale.cnq * qs * (s1 - x * s0) / (b.mass * V * V);
    let m0 = b.cna * qs * (b.xcp - x) / b.iyy;
    let g = scale.lag * 0.5 * RHO * S * V * (x * b.lag.0 - b.lag.1) / b.iyy;
    let p_hat = p * D / (2.0 * V);
    let cnpa = scale.magnus * b.cnpa;
    let fm = qs * cnpa * p_hat / (b.mass * V);
    let mm = (b.xcp - x) * qs * cnpa * p_hat / b.iyy;
    let pp = b.ixx * p / b.iyy;
    let kap = 1.0 - cq;
    let bb = C(a + h - kap * g, -(pp - fm));
    let kk = C(kap * m0 + h * a + pp * fm, kap * mm - pp * a + h * fm);
    (bb, kk)
}

#[test]
fn free_oscillation_matches_the_linear_theory() {
    let b = osc_body();
    let quasi = AeroScale::quasi_steady();
    let cases = [
        ("quasi-steady", quasi),
        ("lag", AeroScale { lag: 1.0, ..quasi }),
        ("CNq force", AeroScale { cnq: 1.0, ..quasi }),
        ("lag and CNq force", AeroScale::default()),
    ];
    let mut prev: Vec<(f64, f64)> = Vec::new();
    for (name, scale) in cases {
        let (dt, every, n) = (2e-4, 25, 120);
        let xi = fly(&b, scale, 0.5f64.to_radians(), 0.0, dt, every, n);
        let lam = prony2(&xi, dt * every as f64);
        let (bb, kk) = theory(&b, scale, 0.0);
        assert!(bb.1.abs() < 1e-15 && kk.1.abs() < 1e-15);
        // Classical form: omega_n^2 = K, log decrement 2 pi zeta / sqrt(1 - zeta^2), zeta = B / (2 omega_n).
        let wn_th = kk.0.sqrt();
        let zeta = bb.0 / (2.0 * wn_th);
        let dec_th = 2.0 * std::f64::consts::PI * zeta / (1.0 - zeta * zeta).sqrt();
        for l in lam {
            let wn = l.abs();
            let dec = 2.0 * std::f64::consts::PI * (-l.0) / l.1.abs();
            println!("{name}: omega_n {wn:.5} vs {wn_th:.5}, log decrement {dec:.5} vs {dec_th:.5}");
            assert!((wn / wn_th - 1.0).abs() < 0.01, "{name}: omega_n {wn} vs {wn_th}");
            assert!((dec / dec_th - 1.0).abs() < 0.03, "{name}: log decrement {dec} vs {dec_th}");
        }
        prev.push((wn_th, dec_th));
    }
    // The cases differ by more than the tolerances: the lag term (CG ahead of the volume centroid)
    // adds damping, the rotation-induced force lowers the frequency (1 - c_q).
    assert!(prev[1].1 > 1.1 * prev[0].1, "{prev:?}");
    assert!(prev[2].0 < 0.98 * prev[0].0, "{prev:?}");
}

#[test]
fn spinning_body_follows_murphys_epicyclic_theory() {
    // 5 kg, I = 1 kg m^2, Ixx = 0.1 kg m^2, p = 300 rad/s: P = 30 rad/s against omega = 17 rad/s.
    let base = Body { mass: 5.0, damp: [10.0, 10.0, 10.25], lag: (2.0, 1.6), ..osc_body() };
    let p = 300.0;
    for cnpa in [3.0, -3.0] {
        let b = Body { cnpa, ..base };
        let scale = AeroScale::default();
        let (dt, every, n) = (2e-4, 50, 200);
        let xi = fly(&b, scale, 0.5f64.to_radians(), p, dt, every, n);
        let mut lam = prony2(&xi, dt * every as f64);
        lam.sort_by(|x, y| y.1.total_cmp(&x.1));
        let (bb, kk) = theory(&b, scale, p);
        // Murphy: xi'' + (H - i P) xi' - (M + i P T) xi = 0.
        let (h, pp) = (bb.0, -bb.1);
        let (m, t) = (-kk.0, -kk.1 / pp);
        let disc = (pp * pp - 4.0 * m).sqrt();
        let w = [0.5 * (pp + disc), 0.5 * (pp - disc)];
        let damping = [-0.5 * (h - pp * (2.0 * t - h) / disc), -0.5 * (h + pp * (2.0 * t - h) / disc)];
        // Exact roots of the characteristic equation, fast first.
        let mut exact = quad_roots(bb, kk);
        exact.sort_by(|x, y| y.1.total_cmp(&x.1));
        for i in 0..2 {
            let mode = ["fast", "slow"][i];
            println!(
                "cnpa {cnpa} {mode}: omega {:.4} (Murphy {:.4}, exact {:.4}), damping {:.4} (Murphy {:.4}, exact {:.4})",
                lam[i].1, w[i], exact[i].1, lam[i].0, damping[i], exact[i].0
            );
            assert!((lam[i].1 / w[i] - 1.0).abs() < 0.01, "cnpa {cnpa} {mode} frequency {} vs {}", lam[i].1, w[i]);
            assert!((lam[i].0 / damping[i] - 1.0).abs() < 0.05, "cnpa {cnpa} {mode} damping {} vs {}", lam[i].0, damping[i]);
            assert!((lam[i].1 / exact[i].1 - 1.0).abs() < 1e-3 && (lam[i].0 / exact[i].0 - 1.0).abs() < 1e-2);
        }
        // The Magnus moment (CG ahead of the centre of pressure) damps the slow mode and undamps
        // the fast one for cnpa < 0, and the reverse for cnpa > 0 (Murphy's 2T - H).
        let (fast, slow) = (damping[0], damping[1]);
        if cnpa < 0.0 {
            assert!(slow < fast, "{fast} {slow}");
        } else {
            assert!(fast < slow, "{fast} {slow}");
        }
    }
}

/// Derivative at a free-flight state of the harness body with lag sums (10, 8) and `scale`.
fn deriv_at(b: &Body, scale: AeroScale, wind: Wind, s: &State) -> (State, Aux) {
    let table = table_of(b);
    let motor = no_thrust();
    let mut sim = base_sim(&table, &motor, b, wind, AtmosphereModel::constant(RHO, 340.0, 1.8e-5).unwrap());
    sim.aero_scale = scale;
    let cx = ctx(&sim, false);
    sim.deriv(&cx, 100.0, s, Phase::Free)
}

#[test]
fn rotation_induced_force_has_the_sign_of_the_damping_moment() {
    // Pitching at q = 2 rad/s in axial flow: the fins aft of the CG move along +z_b, the force points along -z_b.
    // No lag sums: the lag moment would respond to the force through the acceleration.
    let b = Body { lag: (0.0, 0.0), ..osc_body() };
    let s = State { pos: Vec3::new(0.0, 0.0, 100.0), vel: Vec3::new(V, 0.0, 0.0), q: Quat::IDENTITY, w: Vec3::new(0.0, 2.0, -1.0) };
    let (on, _) = deriv_at(&b, AeroScale::default(), calm(), &s);
    let (off, _) = deriv_at(&b, AeroScale { cnq: 0.0, ..AeroScale::default() }, calm(), &s);
    let qs = 0.5 * RHO * V * V * S;
    let d1 = b.damp[1] - b.x_cg * b.damp[0];
    // dF = -(q S / V)(S1 - x_cg S0)(e_x x w), e_x x w = (0, -r, q).
    let expect = Vec3::new(0.0, -qs / V * d1 * 1.0, -qs / V * d1 * 2.0) / b.mass;
    let got = on.vel - off.vel;
    assert!((got - expect).norm() < 1e-12 * expect.norm(), "{got:?} vs {expect:?}");
    assert!(got.z < 0.0 && got.y < 0.0);
    // The moment is the damping moment alone: unchanged.
    assert_eq!(on.w, off.w);
    // A general table expanded from it gives the same force.
    let t = table_of(&b).to_general(aero::table::angle_axis(180.0, 30.0, 2.0, 10.0), aero::table::angle_axis(90.0, 30.0, 2.0, 10.0));
    let motor = no_thrust();
    let atm = AtmosphereModel::constant(RHO, 340.0, 1.8e-5).unwrap();
    let mut sim = base_sim(&t, &motor, &b, calm(), atm);
    let cx = ctx(&sim, false);
    let g_on = sim.deriv(&cx, 100.0, &s, Phase::Free).0;
    sim.aero_scale.cnq = 0.0;
    let g_off = sim.deriv(&cx, 100.0, &s, Phase::Free).0;
    assert!(((g_on.vel - g_off.vel) - expect).norm() < 1e-9 * expect.norm());
}

#[test]
fn lag_moment_follows_the_crossflow_acceleration_with_wind_shear() {
    // Rising vertically at 100 m/s through a power-law wind at 50 m: the wind grows along the
    // climb, so the crossflow changes (alpha-dot) although the body does not rotate (q = 0). A 50 kg
    // body, whose lateral acceleration from the normal force is of the order of the shear term.
    let b = Body { mass: 50.0, ..osc_body() };
    let wind = Wind::from_config(&WindConfig { model: WindModel::Power, speed: 10.0, ref_height: 10.0, direction_deg: 270.0, ..Default::default() }).unwrap();
    // Body x up, y north, z = x cross y = west.
    let rot = Mat3::from_cols(Vec3::new(0.0, 0.0, 1.0), Vec3::new(0.0, 1.0, 0.0), Vec3::new(-1.0, 0.0, 0.0));
    let s = State { pos: Vec3::new(0.0, 0.0, 50.0), vel: Vec3::new(0.0, 0.0, V), q: Quat::from_mat(&rot), w: Vec3::new(0.0, 0.0, 0.0) };
    let on_scale = AeroScale::default();
    let (on, aux) = deriv_at(&b, on_scale, wind.clone(), &s);
    let (off, _) = deriv_at(&b, AeroScale { lag: 0.0, ..on_scale }, wind.clone(), &s);
    // No lag force: the acceleration is the same.
    assert_eq!(on.vel, off.vel);
    let r = s.q.to_mat();
    let ub = r.transpose_mul_vec(s.vel - wind.at(50.0));
    let dwdh = (wind.at(50.01) - wind.at(49.99)) / 0.02;
    let shear = dwdh * V;
    let u_dot = r.transpose_mul_vec(on.vel - shear);
    let fade = super::lag_fade(aux.alpha.min(std::f64::consts::PI - aux.alpha));
    let k = -0.5 * RHO * S * (b.x_cg * b.lag.0 - b.lag.1) * fade;
    let expect = Vec3::new(0.0, -u_dot.z, u_dot.y) * (k / b.iyy);
    let got = on.w - off.w;
    assert!((got - expect).norm() < 1e-9 * expect.norm(), "{got:?} vs {expect:?} (ub {ub:?})");
    // The shear term is a large part of it: without dW/dt the moment would be far off.
    let u_dot_still = r.transpose_mul_vec(on.vel);
    let without = Vec3::new(0.0, -u_dot_still.z, u_dot_still.y) * (k / b.iyy);
    assert!((without - got).norm() > 0.2 * got.norm(), "{without:?} vs {got:?}");
    assert!(ub.y.hypot(ub.z) > 1.0, "crosswind {ub:?}");
}

#[test]
fn lag_fade_is_smooth_and_vanishes_beyond_30_deg() {
    assert_eq!(super::lag_fade(0.0), 1.0);
    assert!((super::lag_fade(15f64.to_radians()) - 15f64.to_radians().cos()).abs() < 1e-15);
    assert_eq!(super::lag_fade(30f64.to_radians()), 0.0);
    assert_eq!(super::lag_fade(60f64.to_radians()), 0.0);
    let mut prev = 1.0;
    for i in 0..=300 {
        let f = super::lag_fade(f64::from(i) * 0.1f64.to_radians());
        assert!(f <= prev + 1e-15 && (prev - f) < 0.02, "step at {i}");
        prev = f;
    }
}

fn sample_parts() -> (AeroTable, Motor) {
    let g = extract(&SampleRocket::default().mesh(), &ExtractOptions::default()).unwrap();
    let model = AeroModel::new(g, AeroOptions::default());
    let table = AeroTable::build(&model, String::new(), Extrapolation::Linear);
    (table, Motor::new("s".into(), 0.054, 0.5, 1.0, vec![(0.0, 800.0), (2.0, 800.0)]))
}

fn sample_sim<'a>(table: &'a AeroTable, motor: &'a Motor, wind: f64) -> Simulation<'a> {
    Simulation {
        table,
        mass: MassProperties::axisymmetric(8.0, 0.85, 0.02, 1.5),
        motor,
        motor_aft_x: 1.5,
        launch: Launch { latitude: 35.0, longitude: 139.0, altitude: 0.0, rail_length: 3.0, elevation_deg: 85.0, azimuth_deg: 0.0, roll_deg: 0.0 },
        recovery: Recovery { enabled: true, cd_s: 3.5, delay: 1.0, main_cd_s: None, main_altitude: None },
        wind: Wind::from_config(&WindConfig { speed: wind, direction_deg: 270.0, ..Default::default() }).unwrap(),
        atmosphere: AtmosphereModel::default(),
        earth: Earth::default(),
        settings: Settings { max_time: 600.0, output_interval: 0.1, ..Default::default() },
        aero_scale: AeroScale::default(),
        thrust_line: ThrustLine::default(),
        fin_cant_deg: 0.0,
    }
}

#[test]
fn quasi_steady_switch_reproduces_the_quasi_steady_trajectory_bit_for_bit() {
    // Reference: the flight model without the lag, Magnus and rotation-induced terms (the summary of
    // this flight before they existed, bit patterns of apogee, landing east/north and max Mach), in a
    // crosswind with fin cant and a lateral CG offset, for the axisymmetric table and its general expansion.
    let (table, motor) = sample_parts();
    let general = table.to_general(aero::table::angle_axis(180.0, 30.0, 2.0, 10.0), aero::table::angle_axis(90.0, 30.0, 2.0, 10.0));
    let reference = [
        ("axisymmetric", &table, [0x4091c7aff71ab0ca_u64, 0xc07952c1ed3b6103, 0x40792466af48f2c6, 0x3fdf1a2db674188c]),
        ("general", &general, [0x4091c73be16b2bbd, 0xc07947aea62fb0da, 0x4079207d7a2f72b9, 0x3fdf1992aafce404]),
    ];
    for (name, t, bits) in reference {
        let mut sim = sample_sim(t, &motor, 5.0);
        sim.fin_cant_deg = 1.0;
        sim.mass.cg_lateral_dry = [0.001, 0.0];
        sim.aero_scale = AeroScale::quasi_steady();
        let s = sim.run(Descent::Ballistic, false).unwrap().summary;
        let got = [s.apogee.to_bits(), s.landing_east.to_bits(), s.landing_north.to_bits(), s.max_mach.to_bits()];
        assert_eq!(got, bits, "{name}: apogee {} landing ({}, {})", s.apogee, s.landing_east, s.landing_north);
        // The full model differs (the Barrowman table has lag sums and the damping sums give a force).
        sim.aero_scale = AeroScale::default();
        let full = sim.run(Descent::Ballistic, false).unwrap().summary;
        assert_ne!(full.apogee.to_bits(), bits[0], "{name}");
        assert!((full.apogee - s.apogee).abs() < 2.0, "{name}: {} vs {}", full.apogee, s.apogee);
    }
}

#[test]
fn roll_resonance_is_reported_for_a_canted_rocket_with_a_lateral_cg_offset() {
    let (table, motor) = sample_parts();
    let mut sim = sample_sim(&table, &motor, 0.0);
    sim.settings.output_interval = 0.002;
    let plain = sim.run(Descent::Ballistic, false).unwrap().summary;
    assert_eq!(plain.roll_resonance, None);
    sim.fin_cant_deg = 2.0;
    sim.mass.cg_lateral_dry = [0.002, 0.0];
    let r = sim.run(Descent::Ballistic, true).unwrap();
    let res = r.summary.roll_resonance.expect("roll resonance");
    println!("{res:?}");
    assert!(res.t > r.summary.rail_exit_time && res.t < r.summary.apogee_time, "{res:?}");
    assert!(res.mach > 0.0 && res.frequency > 0.0 && res.crossings >= 1);
    // At the crossing the roll rate equals the pitch natural frequency: check against the samples.
    let near = r.samples.iter().min_by(|a, b| (a.t - res.t).abs().total_cmp(&(b.t - res.t).abs())).unwrap();
    assert!((near.roll_rate.abs() / res.frequency - 1.0).abs() < 0.05, "{near:?} {res:?}");
    assert!((near.mach - res.mach).abs() < 0.005);
    // Summary JSON round trip.
    let j = serde_json::to_value(&r.summary).unwrap();
    assert_eq!(serde_json::from_value::<RollResonance>(j["roll_resonance"].clone()).unwrap(), res);
}
