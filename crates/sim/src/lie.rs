// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025-2026 西濱大将 (NISHIHAMA Daisuke)
//! Lie-group attitude integration on SO(3) / unit quaternions.
//!
//! Provides the exponential map and its (inverse) derivative on SO(3) and a generic
//! Runge-Kutta-Munthe-Kaas (RKMK) step for the coupled system
//! `(x, q) in R^m x S^3`, where `x` holds the Euclidean unknowns (angular velocity,
//! position, velocity, ...) and `q` is the body-to-world attitude quaternion.
//!
//! Conventions: `q_dot = 1/2 q (x) (0, w)` with `w` the angular velocity in the BODY frame
//! (right-trivialised in body coordinates). An update is `q_{n+1} = q_n (x) exp(Theta)`,
//! where `Theta` is the body rotation vector (rad). `exp_so3(theta)` is the unit quaternion of
//! the rotation by angle `|theta|` about `theta`, i.e. `(cos(|theta|/2), sin(|theta|/2) theta/|theta|)`.
//! The Lie algebra so(3) is identified with R^3 so that the bracket is the cross product.
//!
//! Because every update is a product of unit quaternions, `|q|` stays 1 to machine precision
//! without renormalisation.
//!
//! References: H. Munthe-Kaas, "Runge-Kutta methods on Lie groups", BIT 38 (1998) 92-111;
//! H. Munthe-Kaas, "High order Runge-Kutta methods on manifolds", Appl. Numer. Math. 29 (1999)
//! 115-127; A. Iserles, H. Munthe-Kaas, S. Norsett, A. Zanna, "Lie-group methods",
//! Acta Numerica 9 (2000) 215-365.

use crate::tableau::RkTableau;
use geom::{Quat, Vec3};

/// Hamilton product `a (x) b`.
pub fn quat_mul(a: Quat, b: Quat) -> Quat {
    Quat {
        w: a.w * b.w - a.x * b.x - a.y * b.y - a.z * b.z,
        x: a.w * b.x + a.x * b.w + a.y * b.z - a.z * b.y,
        y: a.w * b.y - a.x * b.z + a.y * b.w + a.z * b.x,
        z: a.w * b.z + a.x * b.y - a.y * b.x + a.z * b.w,
    }
}

/// Conjugate (= inverse for unit quaternions).
pub fn quat_conj(q: Quat) -> Quat {
    Quat { w: q.w, x: -q.x, y: -q.y, z: -q.z }
}

/// Quaternion norm.
pub fn quat_norm(q: Quat) -> f64 {
    (q.w * q.w + q.x * q.x + q.y * q.y + q.z * q.z).sqrt()
}

/// Exponential map so(3) -> SO(3) as a unit quaternion: rotation by angle `|theta|` about `theta`.
/// Uses a Taylor series of `sin(a)/|theta|` (a = |theta|/2) for small angles.
pub fn exp_so3(theta: Vec3) -> Quat {
    let n = theta.norm();
    let a = 0.5 * n;
    let (c, s) = if n < 0.2 {
        let a2 = a * a;
        // cos a and sin(a)/n = 0.5 * sin(a)/a, truncated after a^8 (error < 1e-17 for a < 0.1).
        let c = 1.0 - a2 / 2.0 * (1.0 - a2 / 12.0 * (1.0 - a2 / 30.0 * (1.0 - a2 / 56.0)));
        let sinc = 1.0 - a2 / 6.0 * (1.0 - a2 / 20.0 * (1.0 - a2 / 42.0 * (1.0 - a2 / 72.0)));
        (c, 0.5 * sinc)
    } else {
        (a.cos(), a.sin() / n)
    };
    Quat { w: c, x: s * theta.x, y: s * theta.y, z: s * theta.z }
}

/// Logarithm SO(3) -> so(3): rotation vector of the shortest rotation represented by `q`
/// (angle in [0, pi]); handles `q` and `-q` identically. `q` need not be exactly normalised.
pub fn log_so3(q: Quat) -> Vec3 {
    let q = if q.w < 0.0 { Quat { w: -q.w, x: -q.x, y: -q.y, z: -q.z } } else { q };
    let v = Vec3::new(q.x, q.y, q.z);
    let n = v.norm();
    let k = if n < 1e-8 {
        // 2 atan(n/w)/n = (2/w)(1 - r^2/3), r = n/w
        let r = n / q.w;
        2.0 / q.w * (1.0 - r * r / 3.0)
    } else {
        2.0 * n.atan2(q.w) / n
    };
    v * k
}

/// `dexp_u(v) = sum_k ad_u^k v / (k+1)!` on so(3):
/// `v + (1-cos|u|)/|u|^2 u x v + (|u|-sin|u|)/|u|^3 u x (u x v)`.
/// Derivative of the exponential map: `exp(u)^-1 d/de exp(u + e v) = dexp_{-u}(v)`.
pub fn dexp_so3(u: Vec3, v: Vec3) -> Vec3 {
    let s = u.norm();
    let s2 = s * s;
    let (c1, c2) = if s < 0.1 {
        // (1-cos s)/s^2 and (s-sin s)/s^3
        (
            0.5 * (1.0 - s2 / 12.0 * (1.0 - s2 / 30.0 * (1.0 - s2 / 56.0 * (1.0 - s2 / 90.0)))),
            (1.0 / 6.0) * (1.0 - s2 / 20.0 * (1.0 - s2 / 42.0 * (1.0 - s2 / 72.0 * (1.0 - s2 / 110.0)))),
        )
    } else {
        ((1.0 - s.cos()) / s2, (s - s.sin()) / (s2 * s))
    };
    let uv = u.cross(v);
    v + c1 * uv + c2 * u.cross(uv)
}

/// Inverse of [`dexp_so3`]:
/// `v - 1/2 u x v + (1/|u|^2)(1 - (|u|/2) cot(|u|/2)) u x (u x v)`.
/// Singular at `|u| = 2 pi` (never reached for small steps).
pub fn dexpinv_so3(u: Vec3, v: Vec3) -> Vec3 {
    let s = u.norm();
    let s2 = s * s;
    let g = if s < 0.1 {
        // 1/12 + s^2/720 + s^4/30240 + s^6/1209600 + s^8/47900160
        1.0 / 12.0 + s2 * (1.0 / 720.0 + s2 * (1.0 / 30240.0 + s2 * (1.0 / 1209600.0 + s2 / 47900160.0)))
    } else {
        let h = 0.5 * s;
        (1.0 - h / h.tan()) / s2
    };
    let uv = u.cross(v);
    v - 0.5 * uv + g * u.cross(uv)
}

/// Result of an RKMK step including the stage data, so that adaptive integrators
/// (e.g. DOP853) can form their own embedded error estimates.
#[derive(Debug, Clone)]
pub struct LieStages {
    /// Euclidean stage derivatives `kx_i` (each of length m).
    pub kx: Vec<Vec<f64>>,
    /// Algebra stage derivatives `K~_i = dexpinv(-Theta_i, omega_i)` (rad/s).
    pub ktilde: Vec<Vec3>,
    /// Body angular velocities `omega_i` returned by `f` at each stage.
    pub omegas: Vec<Vec3>,
    /// Step result.
    pub step: LieStep,
}

/// New state after one RKMK step.
#[derive(Debug, Clone)]
pub struct LieStep {
    pub x: Vec<f64>,
    pub q: Quat,
    pub stage_omegas: Vec<Vec3>,
}

impl LieStages {
    /// Linear combination of stages with weights `w` (same length as the tableau's `b`):
    /// returns `(h sum w_i kx_i, h sum w_i K~_i)`. With `w = b - b_hat` this is the embedded
    /// error estimate: the Euclidean part is measured in the units of `x`, the attitude part
    /// on the algebra increment `Theta - Theta_hat` (a rotation vector, rad). The attitude error
    /// norm is therefore `|Theta - Theta_hat|` in rad, directly comparable to angle tolerances.
    /// (To first order this equals the angle between the two propagated attitudes.)
    pub fn combine(&self, w: &[f64], h: f64) -> (Vec<f64>, Vec3) {
        let m = self.kx.first().map_or(0, |k| k.len());
        let mut dx = vec![0.0; m];
        let mut th = Vec3::ZERO;
        for (i, &wi) in w.iter().enumerate() {
            if wi == 0.0 {
                continue;
            }
            for (d, k) in dx.iter_mut().zip(&self.kx[i]) {
                *d += h * wi * k;
            }
            th += (h * wi) * self.ktilde[i];
        }
        (dx, th)
    }
}

/// One RKMK step keeping all stage data. `f(t, x, q) -> (dx/dt, omega_body)`.
///
/// Stage `i`: `Theta_i = h sum_j a_ij K~_j`, `Q_i = q (x) exp(Theta_i)`,
/// `X_i = x + h sum_j a_ij kx_j`, `(kx_i, w_i) = f(t + c_i h, X_i, Q_i)`,
/// `K~_i = dexpinv(-Theta_i, w_i)`. Final `Theta = h sum b_i K~_i`, `q_new = q (x) exp(Theta)`.
/// (The sign comes from the body-frame convention `R_dot = R w^`.) The full closed-form dexpinv is
/// used, so the classical order of the tableau is preserved.
pub fn rkmk_step_with_stages(
    tab: &RkTableau,
    f: &mut impl FnMut(f64, &[f64], Quat) -> (Vec<f64>, Vec3),
    t: f64,
    x: &[f64],
    q: Quat,
    h: f64,
) -> LieStages {
    let s = tab.stages();
    let m = x.len();
    let mut kx: Vec<Vec<f64>> = Vec::with_capacity(s);
    let mut ktilde: Vec<Vec3> = Vec::with_capacity(s);
    let mut omegas: Vec<Vec3> = Vec::with_capacity(s);
    let mut xi = vec![0.0; m];
    for i in 0..s {
        let mut theta = Vec3::ZERO;
        xi.copy_from_slice(x);
        for (j, &aij) in tab.a[i].iter().enumerate() {
            if aij == 0.0 {
                continue;
            }
            theta += (h * aij) * ktilde[j];
            for (xv, k) in xi.iter_mut().zip(&kx[j]) {
                *xv += h * aij * k;
            }
        }
        let qi = quat_mul(q, exp_so3(theta));
        let (k, w) = f(t + tab.c[i] * h, &xi, qi);
        debug_assert_eq!(k.len(), m);
        ktilde.push(dexpinv_so3(-theta, w));
        omegas.push(w);
        kx.push(k);
    }
    let mut xn = x.to_vec();
    let mut theta = Vec3::ZERO;
    for i in 0..s {
        let bi = tab.b[i];
        if bi == 0.0 {
            continue;
        }
        for (xv, k) in xn.iter_mut().zip(&kx[i]) {
            *xv += h * bi * k;
        }
        theta += (h * bi) * ktilde[i];
    }
    let qn = quat_mul(q, exp_so3(theta));
    debug_assert!((quat_norm(qn) - 1.0).abs() < 1e-9, "RKMK produced a non-unit quaternion (input q not unit?)");
    LieStages { kx, ktilde, omegas: omegas.clone(), step: LieStep { x: xn, q: qn, stage_omegas: omegas } }
}

/// One RKMK step for any explicit tableau. See [`rkmk_step_with_stages`].
pub fn rkmk_step(
    tab: &RkTableau,
    f: &mut impl FnMut(f64, &[f64], Quat) -> (Vec<f64>, Vec3),
    t: f64,
    x: &[f64],
    q: Quat,
    h: f64,
) -> LieStep {
    rkmk_step_with_stages(tab, f, t, x, q, h).step
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rel_angle(a: Quat, b: Quat) -> f64 {
        log_so3(quat_mul(quat_conj(a), b)).norm()
    }

    /// Dormand-Prince 5(4) tableau, fifth-order propagating weights.
    fn dp5() -> RkTableau {
        let a7 = vec![35.0 / 384.0, 0.0, 500.0 / 1113.0, 125.0 / 192.0, -2187.0 / 6784.0, 11.0 / 84.0];
        RkTableau {
            c: vec![0.0, 0.2, 0.3, 0.8, 8.0 / 9.0, 1.0, 1.0],
            a: vec![
                vec![],
                vec![0.2],
                vec![3.0 / 40.0, 9.0 / 40.0],
                vec![44.0 / 45.0, -56.0 / 15.0, 32.0 / 9.0],
                vec![19372.0 / 6561.0, -25360.0 / 2187.0, 64448.0 / 6561.0, -212.0 / 729.0],
                vec![9017.0 / 3168.0, -355.0 / 33.0, 46732.0 / 5247.0, 49.0 / 176.0, -5103.0 / 18656.0],
                a7.clone(),
            ],
            b: a7.into_iter().chain([0.0]).collect(),
            order: 5,
        }
    }

    const INERTIA: [f64; 3] = [1.0, 2.0, 3.0];

    /// Torque-free rigid body: x = omega_body; returns (domega/dt, omega).
    fn euler(_t: f64, x: &[f64], _q: Quat) -> (Vec<f64>, Vec3) {
        let [i1, i2, i3] = INERTIA;
        let (w1, w2, w3) = (x[0], x[1], x[2]);
        (
            vec![(i2 - i3) / i1 * w2 * w3, (i3 - i1) / i2 * w3 * w1, (i1 - i2) / i3 * w1 * w2],
            Vec3::new(w1, w2, w3),
        )
    }

    fn energy(x: &[f64]) -> f64 {
        0.5 * (INERTIA[0] * x[0] * x[0] + INERTIA[1] * x[1] * x[1] + INERTIA[2] * x[2] * x[2])
    }

    fn ang_mom(x: &[f64]) -> f64 {
        ((INERTIA[0] * x[0]).powi(2) + (INERTIA[1] * x[1]).powi(2) + (INERTIA[2] * x[2]).powi(2)).sqrt()
    }

    const X0: [f64; 3] = [0.3, 1.0, 0.4];

    fn q0() -> Quat {
        exp_so3(Vec3::new(0.4, -0.7, 0.2))
    }

    /// Integrate over [0, tend]; returns (final x, final q, max norm err, max energy err, max |L| err).
    fn run_rkmk(tab: &RkTableau, h: f64, tend: f64) -> (Vec<f64>, Quat, f64, f64, f64) {
        let mut x = X0.to_vec();
        let mut q = q0();
        let (e0, l0) = (energy(&x), ang_mom(&x));
        let (mut dn, mut de, mut dl) = (0.0f64, 0.0f64, 0.0f64);
        let n = (tend / h).round() as usize;
        let mut f = euler;
        for k in 0..n {
            let st = rkmk_step(tab, &mut f, k as f64 * h, &x, q, h);
            x = st.x;
            q = st.q;
            dn = dn.max((quat_norm(q) - 1.0).abs());
            de = de.max((energy(&x) - e0).abs() / e0);
            dl = dl.max((ang_mom(&x) - l0).abs() / l0);
        }
        (x, q, dn, de, dl)
    }

    fn run_rk4_normalise(h: f64, tend: f64) -> (Quat, f64, f64) {
        let tab = RkTableau::rk4();
        let mut x = X0.to_vec();
        let mut q = q0();
        let e0 = energy(&x);
        let mut de = 0.0f64;
        let n = (tend / h).round() as usize;
        for _ in 0..n {
            // Euclidean-embedded RK on (x, q in R^4), then renormalise.
            let mut kx: Vec<Vec<f64>> = vec![];
            let mut kq: Vec<[f64; 4]> = vec![];
            for i in 0..tab.stages() {
                let mut xi = x.clone();
                let mut qi = [q.w, q.x, q.y, q.z];
                for (j, &a) in tab.a[i].iter().enumerate() {
                    for (xv, k) in xi.iter_mut().zip(&kx[j]) {
                        *xv += h * a * k;
                    }
                    for c in 0..4 {
                        qi[c] += h * a * kq[j][c];
                    }
                }
                let qq = Quat { w: qi[0], x: qi[1], y: qi[2], z: qi[3] };
                let (k, w) = euler(0.0, &xi, qq);
                let d = qq.derivative(w);
                kq.push([d.w, d.x, d.y, d.z]);
                kx.push(k);
            }
            let mut qi = [q.w, q.x, q.y, q.z];
            for i in 0..tab.stages() {
                for (xv, k) in x.iter_mut().zip(&kx[i]) {
                    *xv += h * tab.b[i] * k;
                }
                for c in 0..4 {
                    qi[c] += h * tab.b[i] * kq[i][c];
                }
            }
            q = Quat { w: qi[0], x: qi[1], y: qi[2], z: qi[3] }.normalized();
            de = de.max((energy(&x) - e0).abs() / e0);
        }
        (q, de, ang_mom(&x))
    }

    #[test]
    fn exp_log_round_trip() {
        let dirs = [Vec3::new(1.0, 0.0, 0.0), Vec3::new(0.3, -0.5, 0.8).normalized(), Vec3::new(-1.0, 2.0, 0.5).normalized()];
        let angles = [0.0, 1e-12, 1e-9, 1e-5, 0.01, 0.19999, 0.2001, 1.0, 3.0, std::f64::consts::PI - 1e-9];
        for d in dirs {
            for &a in &angles {
                let th = d * a;
                let q = exp_so3(th);
                assert!((quat_norm(q) - 1.0).abs() < 1e-15);
                let back = log_so3(q);
                assert!((back - th).norm() <= 1e-13 * a.max(1e-3), "a={a} err={}", (back - th).norm());
            }
        }
        // beyond pi: shortest rotation
        let th = Vec3::new(0.0, 0.0, 4.0);
        let b = log_so3(exp_so3(th));
        assert!((b.z - (4.0 - 2.0 * std::f64::consts::PI)).abs() < 1e-13);
        // -q is the same rotation
        let q = exp_so3(Vec3::new(0.3, 0.2, -0.1));
        let mq = Quat { w: -q.w, x: -q.x, y: -q.y, z: -q.z };
        assert!((log_so3(mq) - log_so3(q)).norm() < 1e-15);
    }

    #[test]
    fn dexp_matches_numerical_derivative_and_inverse() {
        let v = Vec3::new(0.7, -0.2, 0.4);
        for u in [Vec3::new(0.0, 0.0, 0.0), Vec3::new(1e-4, 2e-4, -1e-4), Vec3::new(0.05, -0.04, 0.03), Vec3::new(0.3, -0.5, 0.8), Vec3::new(1.5, 1.0, -2.0)] {
            // exp(u)^-1 exp(u + e v) ~ exp(e dexp_{-u}(v)); central difference in e.
            let e = 1e-6;
            let num = |s: f64| {
                let d = quat_mul(quat_conj(exp_so3(u)), exp_so3(u + (s * e) * v));
                log_so3(d)
            };
            let deriv = (num(1.0) - num(-1.0)) / (2.0 * e);
            let ana = dexp_so3(-u, v);
            assert!((deriv - ana).norm() < 1e-8, "u={u:?}: {deriv:?} vs {ana:?}");
            let rt = dexpinv_so3(u, dexp_so3(u, v));
            assert!((rt - v).norm() < 1e-13, "round trip {rt:?}");
        }
        // continuity across the series/closed-form switch
        let dir = Vec3::new(0.2, 0.5, -0.3).normalized();
        let lo = dexpinv_so3(dir * (0.1 - 1e-12), v);
        let hi = dexpinv_so3(dir * (0.1 + 1e-12), v);
        assert!((lo - hi).norm() < 1e-12);
    }

    #[test]
    fn tableaux_ok() {
        dp5().check().unwrap();
    }

    #[test]
    fn free_rigid_body_rkmk_rk4() {
        let tab = RkTableau::rk4();
        let (_, q, dn, de, dl) = run_rkmk(&tab, 0.01, 20.0);
        println!("RKMK-RK4 h=0.01: max |norm-1| = {dn:.2e}, energy rel err = {de:.2e}, |L| rel err = {dl:.2e}");
        assert!(dn < 1e-14, "norm drift {dn}");
        assert!(de < 1e-6 && dl < 1e-6);
        // body angular momentum direction vs rotating frame: world L = R * I w must be constant.
        let (x, q1, ..) = run_rkmk(&tab, 0.01, 20.0);
        let l_w = q1.to_mat().mul_vec(Vec3::new(INERTIA[0] * x[0], INERTIA[1] * x[1], INERTIA[2] * x[2]));
        let l0 = q0().to_mat().mul_vec(Vec3::new(INERTIA[0] * X0[0], INERTIA[1] * X0[1], INERTIA[2] * X0[2]));
        println!("world L drift = {:.2e}", (l_w - l0).norm());
        assert!((l_w - l0).norm() < 1e-6);
        let _ = q;
    }

    fn observed_order(tab: &RkTableau, h: f64, href: f64, tend: f64) -> (f64, f64, f64) {
        let (_, qr, ..) = run_rkmk(&dp5(), href, tend);
        let e1 = rel_angle(qr, run_rkmk(tab, h, tend).1);
        let e2 = rel_angle(qr, run_rkmk(tab, h / 2.0, tend).1);
        ((e1 / e2).log2(), e1, e2)
    }

    #[test]
    fn convergence_order_rk4() {
        let (p, e1, e2) = observed_order(&RkTableau::rk4(), 0.05, 0.001, 20.0);
        println!("RKMK-RK4 order: {p:.3} (err {e1:.3e} -> {e2:.3e})");
        assert!((p - 4.0).abs() < 0.3, "order {p}");
    }

    #[test]
    fn convergence_order_dp5() {
        let (p, e1, e2) = observed_order(&dp5(), 0.05, 0.002, 20.0);
        println!("RKMK-DP5 order: {p:.3} (err {e1:.3e} -> {e2:.3e})");
        assert!((p - 5.0).abs() < 0.4, "order {p}");
    }

    #[test]
    fn compare_with_rk4_normalise() {
        let h = 0.05;
        let (_, qr, ..) = run_rkmk(&dp5(), 0.001, 20.0);
        let (_, ql, dn, de_l, dl) = run_rkmk(&RkTableau::rk4(), h, 20.0);
        let (qe, de_e, _) = run_rk4_normalise(h, 20.0);
        println!(
            "h={h}: RKMK-RK4 attitude err {:.3e} rad (energy {:.2e}, |L| {:.2e}, norm {:.1e}); RK4+normalise err {:.3e} rad (energy {:.2e})",
            rel_angle(qr, ql), de_l, dl, dn, rel_angle(qr, qe), de_e
        );
        // Both are 4th order; the Lie-group variant must not be worse by an order of magnitude.
        assert!(rel_angle(qr, ql) < 10.0 * rel_angle(qr, qe) + 1e-9);
    }

    #[test]
    fn embedded_combine_matches_step() {
        let tab = dp5();
        let mut f = euler;
        let st = rkmk_step_with_stages(&tab, &mut f, 0.0, &X0, q0(), 0.1);
        let (dx, th) = st.combine(&tab.b, 0.1);
        let q = quat_mul(q0(), exp_so3(th));
        assert!(rel_angle(q, st.step.q) < 1e-15);
        assert!((X0[0] + dx[0] - st.step.x[0]).abs() < 1e-15);
    }
}
