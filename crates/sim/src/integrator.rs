// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025-2026 西濱大将 (NISHIHAMA Daisuke)
//! Time integrators for the 13-component rigid-body state (position, velocity, attitude
//! quaternion, body angular velocity): classical RK4 and Dormand-Prince 5(4) with step-size control.
//!
//! The right-hand side is a closure `f(t, &State) -> (State, A)` returning the state derivative and
//! an arbitrary auxiliary value `A` (the flight code uses it for telemetry). Every step function
//! returns the auxiliary value of its last evaluation so callers can reuse it.

use anyhow::{bail, Result};
use crate::tableau::RkTableau;
use geom::{Quat, Vec3};

#[derive(Debug, Clone, Copy)]
pub struct State {
    pub pos: Vec3,
    pub vel: Vec3,
    pub q: Quat,
    pub w: Vec3,
}

/// Number of scalar components of [`State`].
pub const N: usize = 13;

impl State {
    pub const ZERO_DERIV: State = State { pos: Vec3::ZERO, vel: Vec3::ZERO, q: Quat { w: 0.0, x: 0.0, y: 0.0, z: 0.0 }, w: Vec3::ZERO };

    pub fn to_arr(&self) -> [f64; N] {
        [
            self.pos.x, self.pos.y, self.pos.z, self.vel.x, self.vel.y, self.vel.z, self.q.w, self.q.x, self.q.y, self.q.z, self.w.x, self.w.y, self.w.z,
        ]
    }

    pub fn from_arr(a: &[f64; N]) -> State {
        State {
            pos: Vec3::new(a[0], a[1], a[2]),
            vel: Vec3::new(a[3], a[4], a[5]),
            q: Quat { w: a[6], x: a[7], y: a[8], z: a[9] },
            w: Vec3::new(a[10], a[11], a[12]),
        }
    }

    /// `self + d * h`.
    pub fn add(&self, d: &State, h: f64) -> State {
        let (a, b) = (self.to_arr(), d.to_arr());
        let mut r = [0.0; N];
        for i in 0..N {
            r[i] = a[i] + b[i] * h;
        }
        State::from_arr(&r)
    }
}

/// Classical fourth-order Runge-Kutta step with derivative `k1` at (t, s) already known.
/// Returns the new state (quaternion renormalised) and the auxiliary value of the last evaluation.
pub fn rk4_step<A>(f: &mut impl FnMut(f64, &State) -> (State, A), t: f64, s: &State, k1: &State, dt: f64) -> State {
    let (k2, _) = f(t + 0.5 * dt, &s.add(k1, 0.5 * dt));
    let (k3, _) = f(t + 0.5 * dt, &s.add(&k2, 0.5 * dt));
    let (k4, _) = f(t + dt, &s.add(&k3, dt));
    let mut n = *s;
    for (k, w) in [(k1, 1.0), (&k2, 2.0), (&k3, 2.0), (&k4, 1.0)] {
        n = n.add(k, dt * w / 6.0);
    }
    n.q = n.q.normalized();
    n
}

/// Dormand-Prince tableau (rows of the stage weights, abscissae, 5th order weights and the
/// difference b5 - b4 used for the error estimate).
const C: [f64; 6] = [1.0 / 5.0, 3.0 / 10.0, 4.0 / 5.0, 8.0 / 9.0, 1.0, 1.0];
const A: [[f64; 6]; 6] = [
    [1.0 / 5.0, 0.0, 0.0, 0.0, 0.0, 0.0],
    [3.0 / 40.0, 9.0 / 40.0, 0.0, 0.0, 0.0, 0.0],
    [44.0 / 45.0, -56.0 / 15.0, 32.0 / 9.0, 0.0, 0.0, 0.0],
    [19372.0 / 6561.0, -25360.0 / 2187.0, 64448.0 / 6561.0, -212.0 / 729.0, 0.0, 0.0],
    [9017.0 / 3168.0, -355.0 / 33.0, 46732.0 / 5247.0, 49.0 / 176.0, -5103.0 / 18656.0, 0.0],
    [35.0 / 384.0, 0.0, 500.0 / 1113.0, 125.0 / 192.0, -2187.0 / 6784.0, 11.0 / 84.0],
];
/// Error weights `b5 - b4` of the 7 stages.
pub const E: [f64; 7] = [71.0 / 57600.0, 0.0, -71.0 / 16695.0, 71.0 / 1920.0, -17253.0 / 339200.0, 22.0 / 525.0, -1.0 / 40.0];

/// The Dormand-Prince 5(4) tableau as an [`RkTableau`] with 7 stages (the 7th, FSAL, stage has
/// weight 0 in `b`), for the Lie-group integrator.
pub fn dopri5_tableau() -> RkTableau {
    let mut c = vec![0.0];
    c.extend_from_slice(&C);
    let mut a = vec![vec![]];
    for (i, row) in A.iter().enumerate() {
        a.push(row[..=i].to_vec());
    }
    let mut b = A[5].to_vec();
    b.push(0.0);
    RkTableau { c, a, b, order: 5 }
}

pub const SAFETY: f64 = 0.9;
pub const MIN_FACTOR: f64 = 0.2;
pub const MAX_FACTOR: f64 = 5.0;
/// Smallest step the adaptive integrator will take before giving up [s].
pub const MIN_STEP: f64 = 1e-6;

pub struct Dopri5Step<A> {
    /// 5th-order solution with renormalised quaternion.
    pub y: State,
    /// Scaled RMS error estimate (<= 1 means the step meets the tolerances).
    pub err: f64,
    /// Derivative at (t + h, y): the first stage of the next step (FSAL).
    pub k_end: State,
    pub aux_end: A,
}

/// One Dormand-Prince 5(4) step of size `h`; `k1` is the derivative at (t, s). The error is the
/// RMS over all 13 components of |y5 - y4| / (atol + rtol * max(|y0|, |y5|)).
pub fn dopri5_step<A>(f: &mut impl FnMut(f64, &State) -> (State, A), t: f64, s: &State, k1: &State, h: f64, rtol: f64, atol: f64) -> Dopri5Step<A> {
    let y0 = s.to_arr();
    let mut k: Vec<[f64; N]> = Vec::with_capacity(7);
    k.push(k1.to_arr());
    for i in 0..5 {
        let mut y = y0;
        for (j, kj) in k.iter().enumerate() {
            let a = A[i][j];
            if a != 0.0 {
                for c in 0..N {
                    y[c] += h * a * kj[c];
                }
            }
        }
        k.push(f(t + C[i] * h, &State::from_arr(&y)).0.to_arr());
    }
    // 5th-order solution (row 6 of the tableau; its stage-7 derivative is the FSAL one).
    let mut y5 = y0;
    for (j, kj) in k.iter().enumerate() {
        let a = A[5][j];
        if a != 0.0 {
            for c in 0..N {
                y5[c] += h * a * kj[c];
            }
        }
    }
    let mut ys = State::from_arr(&y5);
    ys.q = ys.q.normalized();
    let (k7, aux_end) = f(t + h, &ys);
    k.push(k7.to_arr());
    let mut sum = 0.0;
    let ya = ys.to_arr();
    for c in 0..N {
        let e: f64 = h * (0..7).map(|j| E[j] * k[j][c]).sum::<f64>();
        let sc = atol + rtol * y0[c].abs().max(ya[c].abs());
        sum += (e / sc) * (e / sc);
    }
    Dopri5Step { y: ys, err: (sum / N as f64).sqrt(), k_end: k7, aux_end }
}

/// Step-size factor after a step with scaled error `err` (order 5 controller).
pub fn step_factor(err: f64) -> f64 {
    if !err.is_finite() {
        MIN_FACTOR
    } else if err == 0.0 {
        MAX_FACTOR
    } else {
        (SAFETY * err.powf(-0.2)).clamp(MIN_FACTOR, MAX_FACTOR)
    }
}

/// Checks that a rejected step may still be shrunk.
pub fn check_min_step(h_new: f64, t: f64) -> Result<()> {
    if h_new < MIN_STEP {
        bail!("adaptive integrator step size fell below {MIN_STEP:e} s at t = {t:.4} s; loosen sim.rtol/sim.atol or use rk4");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // Harmonic oscillator x'' = -x embedded in the state (position x, velocity y).
    fn osc(_t: f64, s: &State) -> (State, ()) {
        (State { pos: s.vel, vel: Vec3::new(-s.pos.x, -s.pos.y, 0.0), ..State::ZERO_DERIV }, ())
    }

    fn init() -> State {
        State { pos: Vec3::new(1.0, 0.0, 0.0), vel: Vec3::ZERO, q: Quat::IDENTITY, w: Vec3::ZERO }
    }

    #[test]
    fn dopri5_tableau_is_consistent() {
        let t = dopri5_tableau();
        t.check().unwrap();
        assert_eq!(t.stages(), 7);
    }

    #[test]
    fn rk4_and_dopri_converge() {
        let mut f = osc;
        let (mut s4, mut t) = (init(), 0.0);
        for _ in 0..1000 {
            let (k1, _) = f(t, &s4);
            s4 = rk4_step(&mut f, t, &s4, &k1, 0.01);
            t += 0.01;
        }
        assert!((s4.pos.x - 10f64.cos()).abs() < 1e-8);
        let (mut s5, mut t, mut h) = (init(), 0.0_f64, 0.1_f64);
        while t < 10.0 - 1e-12 {
            h = h.min(10.0 - t);
            let (k1, _) = f(t, &s5);
            let st = dopri5_step(&mut f, t, &s5, &k1, h, 1e-9, 1e-9);
            if st.err <= 1.0 {
                s5 = st.y;
                t += h;
            }
            h *= step_factor(st.err);
        }
        assert!((s5.pos.x - 10f64.cos()).abs() < 1e-7 && (s5.vel.x + 10f64.sin()).abs() < 1e-7, "{}", s5.pos.x - 10f64.cos());
    }
}
