// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025-2026 西濱大将 (NISHIHAMA Daisuke)
//! Integration strategy: one step of the 13-component rigid-body state with a selectable
//! Runge-Kutta method ([`IntegratorKind`]) and attitude treatment ([`AttitudeKind`]).
//!
//! The flight dynamics are implemented once (`Simulation::deriv`); every combination calls it
//! through the closure `f(t, &State) -> (State, A)`.
//!
//! # Lie-group attitude
//! The state is split into the Euclidean part `x = (pos, vel, w)` (9 numbers) and the attitude
//! `q`. The quaternion kinematics of the flight code are `q_dot = 1/2 q (x) (0, w)` with
//! `w = State::w` the body angular velocity RELATIVE TO THE INTEGRATION FRAME. This holds in
//! both frames: in the flat frame the frame does not rotate, in ECEF `q` is the body-to-ECEF
//! attitude, `w` is relative to ECEF, and the `R^T Omega` terms only appear in the Euler
//! equations of `deriv` (the dynamics of `w`), not in `q_dot`. Hence the Lie algebra element
//! fed to RKMK is always `State::w` of the stage state (`omega_i = X_i.w`) and the update is
//! `q <- q (x) exp(Theta)`, `K~_i = dexpinv(-Theta_i, omega_i)`.
//! In the rail and parachute phases there is no attitude dynamics (`q` is frozen); the Lie step
//! then uses `omega = 0`, so `q` is unchanged exactly while the Euclidean part still follows the
//! same Runge-Kutta tableau.
//!
//! # Error estimates with Lie-group attitude
//! The error vector has 12 components (pos, vel, w and the 3 attitude algebra components).
//! * rk45: `h sum (b - b_hat)_i (kx_i, K~_i)` with the Dormand-Prince weights `b5 - b4`.
//! * dop853: DOP853's 5th and 3rd order estimates (`er` and `b - bhh` weights from
//!   `dop853.f`) applied to the Euclidean stage derivatives and to the algebra stage derivatives
//!   `K~_i`, combined with the DOP853 norm `|h| S5 / sqrt(n (S5 + 0.01 S3))`.
//!
//! Euclidean components are scaled by `atol + rtol max(|x0|, |x1|)`; the attitude increment
//! difference (a rotation vector in rad) by `atol + rtol pi`, i.e. an attitude error of `atol`
//! radians is tolerated for small `rtol`.
//!
//! Without Lie groups the 4 quaternion components are ordinary components (scale as above) and
//! the quaternion is renormalised after every accepted step.
//!
//! # FSAL
//! rk45 (vector and Lie) reuses the 7th stage derivative as the first stage of the next step;
//! dop853 reuses its 13th evaluation at the new point. For dop853 without Lie groups that
//! derivative is evaluated at the not yet renormalised quaternion (difference of the order of
//! the tolerance).

use crate::dop853;
use crate::env::{AttitudeKind, IntegratorKind};
use crate::integrator::{self, State, N};
use crate::lie::{self, LieStages};
use crate::tableau::RkTableau;
use geom::{Quat, Vec3};

/// Euclidean part length of the Lie formulation: pos, vel, w.
const M: usize = 9;

/// Result of one attempted step.
pub struct Attempt<A> {
    /// New state (unit quaternion).
    pub y: State,
    /// Scaled error estimate (<= 1 accepted); 0 for fixed-step methods.
    pub err: f64,
    /// Derivative and auxiliary value at the new point (FSAL), if the method provides it.
    pub next: Option<(State, A)>,
}

pub struct Stepper {
    pub kind: IntegratorKind,
    pub attitude: AttitudeKind,
    tab: RkTableau,
    e5: Vec<f64>,
    e3: Vec<f64>,
}

fn to_x(s: &State) -> [f64; M] {
    [s.pos.x, s.pos.y, s.pos.z, s.vel.x, s.vel.y, s.vel.z, s.w.x, s.w.y, s.w.z]
}

fn from_x(x: &[f64], q: Quat) -> State {
    State { pos: Vec3::new(x[0], x[1], x[2]), vel: Vec3::new(x[3], x[4], x[5]), q, w: Vec3::new(x[6], x[7], x[8]) }
}

impl Stepper {
    pub fn new(kind: IntegratorKind, attitude: AttitudeKind) -> Self {
        let tab = match kind {
            IntegratorKind::Rk4 => RkTableau::rk4(),
            IntegratorKind::Rk45 => integrator::dopri5_tableau(),
            IntegratorKind::Dop853 => dop853::tableau(),
        };
        let (e5, e3) = if kind == IntegratorKind::Dop853 { dop853::error_weights() } else { (integrator::E.to_vec(), vec![]) };
        Stepper { kind, attitude, tab, e5, e3 }
    }

    pub fn adaptive(&self) -> bool {
        self.kind.is_adaptive()
    }

    /// Step-size multiplier for a scaled error (controller of the underlying method).
    pub fn factor(&self, err: f64) -> f64 {
        match self.kind {
            IntegratorKind::Dop853 => dop853::step_factor(err),
            _ => integrator::step_factor(err),
        }
    }

    /// One step of size `h` from `(t, s)`; `k1` is the derivative at `(t, s)`. `rotating` tells
    /// whether the attitude evolves (free flight); otherwise `q` is frozen.
    #[allow(clippy::too_many_arguments)]
    pub fn step<A>(
        &self,
        f: &mut impl FnMut(f64, &State) -> (State, A),
        t: f64,
        s: &State,
        k1: &State,
        h: f64,
        rtol: f64,
        atol: f64,
        rotating: bool,
    ) -> Attempt<A> {
        if self.attitude == AttitudeKind::LieGroup {
            return self.lie_step(f, t, s, k1, h, rtol, atol, rotating);
        }
        match self.kind {
            IntegratorKind::Rk4 => Attempt { y: integrator::rk4_step(f, t, s, k1, h), err: 0.0, next: None },
            IntegratorKind::Rk45 => {
                let st = integrator::dopri5_step(f, t, s, k1, h, rtol, atol);
                Attempt { y: st.y, err: st.err, next: Some((st.k_end, st.aux_end)) }
            }
            IntegratorKind::Dop853 => {
                let mut last: Option<A> = None;
                let mut g = |tt: f64, y: &[f64], dy: &mut [f64]| {
                    let a: &[f64; N] = y.try_into().expect("state length");
                    let (d, aux) = f(tt, &State::from_arr(a));
                    dy.copy_from_slice(&d.to_arr());
                    last = Some(aux);
                };
                let st = dop853::step(&mut g, t, &s.to_arr(), &k1.to_arr(), h, rtol, atol);
                let ya: [f64; N] = st.y.as_slice().try_into().expect("state length");
                let ka: [f64; N] = st.k_new.as_slice().try_into().expect("state length");
                let mut y = State::from_arr(&ya);
                y.q = y.q.normalized();
                Attempt { y, err: st.err, next: Some((State::from_arr(&ka), last.expect("13th stage"))) }
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn lie_step<A>(
        &self,
        f: &mut impl FnMut(f64, &State) -> (State, A),
        t: f64,
        s: &State,
        k1: &State,
        h: f64,
        rtol: f64,
        atol: f64,
        rotating: bool,
    ) -> Attempt<A> {
        let mut last: Option<A> = None;
        let mut first = Some(*k1);
        let x0 = to_x(s);
        let mut g = |tt: f64, x: &[f64], q: Quat| {
            let st = from_x(x, q);
            // The first stage is the already known derivative at (t, s).
            let d = match first.take() {
                Some(d) => d,
                None => {
                    let (d, aux) = f(tt, &st);
                    last = Some(aux);
                    d
                }
            };
            (to_x(&State { pos: d.pos, vel: d.vel, q, w: d.w }).to_vec(), if rotating { st.w } else { Vec3::ZERO })
        };
        let stages: LieStages = lie::rkmk_step_with_stages(&self.tab, &mut g, t, &x0, s.q, h);
        let y = from_x(&stages.step.x, stages.step.q);
        let deriv_state = |kx: &[f64], y: &State| {
            let d = from_x(kx, Quat { w: 0.0, x: 0.0, y: 0.0, z: 0.0 });
            State { q: if rotating { y.q.derivative(y.w) } else { d.q }, ..d }
        };
        match self.kind {
            IntegratorKind::Rk4 => Attempt { y, err: 0.0, next: None },
            IntegratorKind::Rk45 => {
                // Stage 7 is evaluated at the new point: FSAL.
                let (dx, th) = stages.combine(&self.e5, h);
                let err = rms_err(&x0, &stages.step.x, &dx, th, rtol, atol);
                let kx7 = stages.kx.last().expect("stages");
                let next = last.map(|a| (deriv_state(kx7, &y), a));
                Attempt { y, err, next }
            }
            IntegratorKind::Dop853 => {
                let (dx5, th5) = stages.combine(&self.e5, h);
                let (dx3, th3) = stages.combine(&self.e3, h);
                let sc = |i: usize| atol + rtol * x0[i].abs().max(stages.step.x[i].abs());
                let sa = atol + rtol * std::f64::consts::PI;
                let (mut s5, mut s3) = (0.0, 0.0);
                for i in 0..M {
                    s5 += (dx5[i] / sc(i)).powi(2);
                    s3 += (dx3[i] / sc(i)).powi(2);
                }
                for (a, b) in [(th5.x, th3.x), (th5.y, th3.y), (th5.z, th3.z)] {
                    s5 += (a / sa).powi(2);
                    s3 += (b / sa).powi(2);
                }
                // dop853::error_norm expects the sums without the factor h.
                let (s5, s3) = (s5 / (h * h), s3 / (h * h));
                let err = dop853::error_norm(h, s5, s3, M + 3);
                // 13th evaluation at the accepted point (first stage of the next step).
                let (d, aux) = f(t + h, &y);
                Attempt { y, err, next: Some((d, aux)) }
            }
        }
    }
}

/// RMS scaled error over the 9 Euclidean components and the 3 attitude algebra components.
fn rms_err(x0: &[f64; M], x1: &[f64], dx: &[f64], th: Vec3, rtol: f64, atol: f64) -> f64 {
    let mut sum = 0.0;
    for i in 0..M {
        let sc = atol + rtol * x0[i].abs().max(x1[i].abs());
        sum += (dx[i] / sc).powi(2);
    }
    let sa = atol + rtol * std::f64::consts::PI;
    sum += (th.x / sa).powi(2) + (th.y / sa).powi(2) + (th.z / sa).powi(2);
    (sum / (M + 3) as f64).sqrt()
}
