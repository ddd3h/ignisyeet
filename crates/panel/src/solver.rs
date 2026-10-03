// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025-2026 西濱大将 (NISHIHAMA Daisuke)
//! Morino low-order Dirichlet panel solver (constant source and doublet panels).
//!
//! With the interior perturbation potential set to zero, the exterior potential on the surface
//! equals the doublet strength `mu`, and the source strength is `sigma = -n . U`, where `U` is
//! the kinematic velocity of the oncoming flow at the panel. The Kutta condition folds the wake
//! doublet strength into the columns of the panels it is attached to:
//! `mu_wake = mu_upper - mu_lower` (fin trailing edge) or `mu_edge` (base ring).
//!
//! Three right-hand sides are solved per Mach number, `U = (1,0,0)`, `(0,0,1)` and
//! `(0,0,x)` (rotation about the nose tip), so that every angle of attack follows by
//! linear combination.

use crate::influence::{doublet, influence};
use crate::mesh::Mesh;
use faer::linalg::solvers::Solve;
use faer::Mat;
use geom::Vec3;
use rayon::prelude::*;

/// Solution of the three unit problems on one (possibly Goethert-scaled) mesh.
#[derive(Debug, Clone)]
pub struct Potential {
    /// Doublet strengths for the right-hand sides x-flow, z-flow, nose rotation.
    pub mu: [Vec<f64>; 3],
    /// Total tangential velocity at each panel for the same three problems.
    pub vel: [Vec<Vec3>; 3],
}

/// Kinematic velocity of the three unit problems at position `c`.
fn unit_velocity(k: usize, c: Vec3) -> Vec3 {
    match k {
        0 => Vec3::new(1.0, 0.0, 0.0),
        1 => Vec3::new(0.0, 0.0, 1.0),
        _ => Vec3::new(0.0, 0.0, c.x),
    }
}

/// Assembles and solves the unit problems on `mesh`.
pub fn solve_potential(mesh: &Mesh) -> Potential {
    let n = mesh.panels.len();
    let mut a = vec![0.0f64; n * n];
    let mut rhs = vec![[0.0f64; 3]; n];
    a.par_chunks_mut(n).zip(rhs.par_iter_mut()).enumerate().for_each(|(i, (row, rh))| {
        let xi = mesh.panels[i].centroid;
        let mut b = [0.0; 3];
        for (j, pj) in mesh.panels.iter().enumerate() {
            let (s, d) = influence(pj, xi);
            row[j] = if i == j { -0.5 } else { d };
            for (k, bk) in b.iter_mut().enumerate() {
                *bk -= s * pj.normal.dot(unit_velocity(k, pj.centroid));
            }
        }
        for w in &mesh.wake_cols {
            let wi: f64 = mesh.wake_panels[w.panels.clone()].iter().map(|wp| doublet(wp, xi)).sum();
            row[w.upper] += wi;
            if let Some(l) = w.lower {
                row[l] -= wi;
            }
        }
        *rh = b;
    });
    let m = Mat::<f64>::from_fn(n, n, |i, j| a[i * n + j]);
    drop(a);
    let lu = m.partial_piv_lu();
    let mut sol = Mat::<f64>::from_fn(n, 3, |i, k| rhs[i][k]);
    lu.solve_in_place(sol.as_mut());
    let mu: [Vec<f64>; 3] = std::array::from_fn(|k| (0..n).map(|i| sol[(i, k)]).collect());
    let vel = std::array::from_fn(|k| tangential_velocity(mesh, &mu[k], k));
    Potential { mu, vel }
}

/// Total tangential velocity: tangential part of `U` plus the surface gradient of `mu`.
fn tangential_velocity(mesh: &Mesh, mu: &[f64], k: usize) -> Vec<Vec3> {
    let grad = surface_gradient(mesh, mu);
    mesh.panels
        .iter()
        .zip(grad)
        .map(|(p, g)| {
            let u = unit_velocity(k, p.centroid);
            u - p.normal * u.dot(p.normal) + g
        })
        .collect()
}

/// Least-squares surface gradient of `mu` from edge-adjacent panels. Neighbours are unfolded
/// into the panel plane across the shared edge, so creases do not distort distances.
pub fn surface_gradient(mesh: &Mesh, mu: &[f64]) -> Vec<Vec3> {
    (0..mesh.panels.len())
        .into_par_iter()
        .map(|i| {
            let p = &mesh.panels[i];
            let (mut g11, mut g12, mut g22, mut r1, mut r2) = (0.0, 0.0, 0.0, 0.0, 0.0);
            for nb in &mesh.neighbors[i] {
                let q = &mesh.panels[nb.panel];
                let (pa, pb) = (mesh.verts[nb.edge[0]], mesh.verts[nb.edge[1]]);
                let mid = (pa + pb) * 0.5;
                let e = (pb - pa).normalized();
                let w = mid - p.centroid;
                let wp = w - e * w.dot(e);
                let wp = wp - p.normal * wp.dot(p.normal);
                let dist_i = wp.norm();
                if dist_i < 1e-14 {
                    continue;
                }
                let u = wp / dist_i;
                let wj = q.centroid - mid;
                let dist_j = (wj - e * wj.dot(e)).norm();
                let s = wj.dot(e) + w.dot(e);
                let ei = (e - p.normal * e.dot(p.normal)).normalized();
                let d = u * (dist_i + dist_j) + ei * s;
                let (dx, dy) = (d.dot(p.e1), d.dot(p.e2));
                let wgt = 1.0 / (dx * dx + dy * dy);
                let dm = mu[nb.panel] - mu[i];
                g11 += wgt * dx * dx;
                g12 += wgt * dx * dy;
                g22 += wgt * dy * dy;
                r1 += wgt * dx * dm;
                r2 += wgt * dy * dm;
            }
            let det = g11 * g22 - g12 * g12;
            if det.abs() < 1e-12 * (g11 * g22).abs().max(1e-300) {
                return Vec3::ZERO;
            }
            let (a, b) = ((g22 * r1 - g12 * r2) / det, (g11 * r2 - g12 * r1) / det);
            p.e1 * a + p.e2 * b
        })
        .collect()
}

/// Force/moment derivatives of a body in subsonic flow (potential part, compressibility
/// corrected). All lengths in metres, coefficients normalised with the reference area.
#[derive(Debug, Clone, Copy, Default)]
pub struct PotentialCoeffs {
    /// dCN/dalpha [1/rad].
    pub cna: f64,
    /// Moment about the nose per radian of uniform incidence, `sum CNa_i x_i` [m/rad].
    pub m_alpha: f64,
    /// Normal force per unit nondimensional pitch rate about the nose [m/rad].
    pub n_rot: f64,
    /// Moment about the nose per unit nondimensional pitch rate [m^2/rad].
    pub m_rot: f64,
}

/// Integrates the linearised pressure response on the real geometry `real`.
/// `trans` and `pot` belong to the Goethert-scaled mesh with factor `beta`.
pub fn coefficients(real: &Mesh, pot: &Potential, beta: f64, s_ref: f64) -> PotentialCoeffs {
    let mut c = PotentialCoeffs::default();
    for (i, p) in real.panels.iter().enumerate().filter(|(_, p)| p.part != crate::mesh::Part::Tail) {
        let vx = pot.vel[0][i];
        let cp_a = -2.0 * vx.dot(pot.vel[1][i]) / beta;
        let cp_r = -2.0 * vx.dot(pot.vel[2][i]) / beta;
        let fz = -p.normal.z * p.area / s_ref;
        c.cna += cp_a * fz;
        c.m_alpha += cp_a * fz * p.centroid.x;
        c.n_rot += cp_r * fz;
        c.m_rot += cp_r * fz * p.centroid.x;
    }
    c
}

/// Pressure coefficient at every panel for angle of attack `alpha` [rad] in incompressible flow.
pub fn cp_at_alpha(pot: &Potential, alpha: f64) -> Vec<f64> {
    let (sa, ca) = alpha.sin_cos();
    pot.vel[0].iter().zip(&pot.vel[1]).map(|(vx, vz)| 1.0 - (*vx * ca + *vz * sa).dot(*vx * ca + *vz * sa)).collect()
}
