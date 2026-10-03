// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025-2026 西濱大将 (NISHIHAMA Daisuke)
//! Potential influence coefficients of constant-strength planar source and doublet panels.
//!
//! Conventions (outward normal `n`, perturbation potential `phi`):
//! * source of density `sigma`: `phi = -sigma * S`, with `S = (1/4 pi) * integral(dS / r)`;
//! * doublet of density `mu` along `n`: `phi = mu * D`, with `D = Omega / 4 pi`, where `Omega` is
//!   the solid angle subtended by the panel (positive on the `+n` side). On the panel itself
//!   `D -> +1/2` (outside) and `-1/2` (inside).
//!
//! The polygon formulas are the exact Hess–Smith / Newman expressions (Katz & Plotkin, §10.4)
//! evaluated in the panel's local plane. Beyond five panel diameters the point-source and
//! point-doublet approximations are used.

use crate::mesh::Panel;
use geom::Vec3;
use std::f64::consts::PI;

const FOUR_PI: f64 = 4.0 * PI;
const FAR_FIELD: f64 = 5.0;

/// Solid angle of the triangle `(a, b, c)` seen from the origin (positive if the triangle is
/// clockwise seen from the origin, i.e. counter-clockwise about a normal pointing at the origin).
fn solid_angle(a: Vec3, b: Vec3, c: Vec3) -> f64 {
    let (la, lb, lc) = (a.norm(), b.norm(), c.norm());
    let num = a.dot(b.cross(c));
    let den = la * lb * lc + a.dot(b) * lc + a.dot(c) * lb + b.dot(c) * la;
    -2.0 * num.atan2(den)
}

/// Returns `(S, D)`: source potential per unit `sigma` (without the minus sign) and doublet
/// potential per unit `mu`, at point `x`.
pub fn influence(p: &Panel, x: Vec3) -> (f64, f64) {
    let d = x - p.centroid;
    let r2 = d.dot(d);
    let lim = FAR_FIELD * p.diameter;
    if r2 > lim * lim {
        let r = r2.sqrt();
        return (p.area / (FOUR_PI * r), p.area * d.dot(p.normal) / (FOUR_PI * r2 * r));
    }
    let (xl, yl, zl) = (d.dot(p.e1), d.dot(p.e2), d.dot(p.normal));
    let n = p.n_v;
    let mut q = [(0.0, 0.0, 0.0); 4]; // corner offsets from the field point in local axes
    let mut xy = [(0.0, 0.0); 4];
    for k in 0..n {
        let c = p.corners[k] - p.centroid;
        let (u, v) = (c.dot(p.e1), c.dot(p.e2));
        xy[k] = (u, v);
        q[k] = (u - xl, v - yl, -zl);
    }
    let vec = |t: (f64, f64, f64)| Vec3::new(t.0, t.1, t.2);
    let mut omega = 0.0;
    for t in 1..n - 1 {
        omega += solid_angle(vec(q[0]), vec(q[t]), vec(q[t + 1]));
    }
    let mut edge_sum = 0.0;
    for k in 0..n {
        let k1 = (k + 1) % n;
        let (dx, dy) = (xy[k1].0 - xy[k].0, xy[k1].1 - xy[k].1);
        let dk = dx.hypot(dy);
        if dk < 1e-14 {
            continue;
        }
        let rk = vec(q[k]).norm();
        let rk1 = vec(q[k1]).norm();
        let den = rk + rk1 - dk;
        if den <= 1e-12 * dk {
            continue; // field point on the edge line: the geometric factor vanishes
        }
        let a = ((yl - xy[k].1) * dx - (xl - xy[k].0) * dy) / dk;
        edge_sum += a * ((rk + rk1 + dk) / den).ln();
    }
    let int_inv_r = edge_sum - zl * omega;
    (int_inv_r / FOUR_PI, omega / FOUR_PI)
}

/// Doublet-only influence (wake panels).
pub fn doublet(p: &Panel, x: Vec3) -> f64 {
    influence(p, x).1
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mesh::Part;

    fn quad(pts: [[f64; 3]; 4]) -> Panel {
        Panel::new(pts.map(|p| Vec3::new(p[0], p[1], p[2])), 4, Part::Body, 0.0)
    }

    /// Brute-force Gauss integration of the bilinear quad.
    fn numeric(p: &Panel, x: Vec3) -> (f64, f64) {
        let n = 200;
        let (mut s, mut dd) = (0.0, 0.0);
        let c = &p.corners;
        for i in 0..n {
            for j in 0..n {
                let (u, v) = ((i as f64 + 0.5) / n as f64, (j as f64 + 0.5) / n as f64);
                let q = c[0] * ((1.0 - u) * (1.0 - v)) + c[1] * (u * (1.0 - v)) + c[2] * (u * v) + c[3] * ((1.0 - u) * v);
                let a = ((c[1] - c[0]) * (1.0 - v) + (c[2] - c[3]) * v).cross((c[3] - c[0]) * (1.0 - u) + (c[2] - c[1]) * u).norm() / (n * n) as f64;
                let r = x - q;
                s += a / r.norm() / FOUR_PI;
                dd += a * r.dot(p.normal) / r.norm().powi(3) / FOUR_PI;
            }
        }
        (s, dd)
    }

    #[test]
    fn matches_numerical_integration() {
        let p = quad([[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [1.2, 0.8, 0.0], [0.0, 1.0, 0.0]]);
        assert!(p.normal.z > 0.99);
        for x in [Vec3::new(0.5, 0.4, 0.3), Vec3::new(2.0, -1.0, -0.7), Vec3::new(0.4, 0.3, -0.05), Vec3::new(-0.5, 2.0, 1.0)] {
            let (s, d) = influence(&p, x);
            let (sn, dn) = numeric(&p, x);
            assert!((s - sn).abs() < 2e-4 * sn.abs().max(0.01), "source {s} vs {sn} at {x:?}");
            assert!((d - dn).abs() < 2e-4, "doublet {d} vs {dn} at {x:?}");
        }
    }

    #[test]
    fn doublet_sides_and_self_value() {
        let p = quad([[-50.0, -50.0, 0.0], [50.0, -50.0, 0.0], [50.0, 50.0, 0.0], [-50.0, 50.0, 0.0]]);
        let above = influence(&p, Vec3::new(0.0, 0.0, 1e-3)).1;
        let below = influence(&p, Vec3::new(0.0, 0.0, -1e-3)).1;
        assert!((above - 0.5).abs() < 1e-3 && (below + 0.5).abs() < 1e-3, "{above} {below}");
    }

    #[test]
    fn far_field_matches_exact() {
        let p = quad([[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [1.0, 1.0, 0.0], [0.0, 1.0, 0.0]]);
        let x = Vec3::new(9.0, 3.0, 4.0);
        let far = influence(&p, x);
        let mut q = p.clone();
        q.diameter = 100.0; // force the exact path
        let ex = influence(&q, x);
        assert!((far.0 - ex.0).abs() < 1e-3 * ex.0 && (far.1 - ex.1).abs() < 5e-3 * ex.1.abs());
    }
}
