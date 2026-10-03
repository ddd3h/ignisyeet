// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025-2026 西濱大将 (NISHIHAMA Daisuke)
//! Minimal 3D vector / quaternion math shared by all crates.

use std::ops::{Add, AddAssign, Div, Index, Mul, Neg, Sub};

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Vec3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Vec3 {
    pub const ZERO: Vec3 = Vec3 { x: 0.0, y: 0.0, z: 0.0 };

    pub const fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }
    pub fn dot(self, o: Vec3) -> f64 {
        self.x * o.x + self.y * o.y + self.z * o.z
    }
    pub fn cross(self, o: Vec3) -> Vec3 {
        Vec3::new(
            self.y * o.z - self.z * o.y,
            self.z * o.x - self.x * o.z,
            self.x * o.y - self.y * o.x,
        )
    }
    pub fn norm(self) -> f64 {
        self.dot(self).sqrt()
    }
    pub fn normalized(self) -> Vec3 {
        let n = self.norm();
        if n > 0.0 {
            self / n
        } else {
            self
        }
    }
}

impl Add for Vec3 {
    type Output = Vec3;
    fn add(self, o: Vec3) -> Vec3 {
        Vec3::new(self.x + o.x, self.y + o.y, self.z + o.z)
    }
}
impl AddAssign for Vec3 {
    fn add_assign(&mut self, o: Vec3) {
        *self = *self + o;
    }
}
impl Sub for Vec3 {
    type Output = Vec3;
    fn sub(self, o: Vec3) -> Vec3 {
        Vec3::new(self.x - o.x, self.y - o.y, self.z - o.z)
    }
}
impl Neg for Vec3 {
    type Output = Vec3;
    fn neg(self) -> Vec3 {
        Vec3::new(-self.x, -self.y, -self.z)
    }
}
impl Mul<f64> for Vec3 {
    type Output = Vec3;
    fn mul(self, s: f64) -> Vec3 {
        Vec3::new(self.x * s, self.y * s, self.z * s)
    }
}
impl Mul<Vec3> for f64 {
    type Output = Vec3;
    fn mul(self, v: Vec3) -> Vec3 {
        v * self
    }
}
impl Div<f64> for Vec3 {
    type Output = Vec3;
    fn div(self, s: f64) -> Vec3 {
        Vec3::new(self.x / s, self.y / s, self.z / s)
    }
}
impl Index<usize> for Vec3 {
    type Output = f64;
    fn index(&self, i: usize) -> &f64 {
        match i {
            0 => &self.x,
            1 => &self.y,
            2 => &self.z,
            _ => panic!("Vec3 index out of range"),
        }
    }
}

/// Row-major 3x3 matrix.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Mat3(pub [[f64; 3]; 3]);

impl Mat3 {
    pub fn from_cols(a: Vec3, b: Vec3, c: Vec3) -> Mat3 {
        Mat3([[a.x, b.x, c.x], [a.y, b.y, c.y], [a.z, b.z, c.z]])
    }
    pub fn col(&self, j: usize) -> Vec3 {
        Vec3::new(self.0[0][j], self.0[1][j], self.0[2][j])
    }
    pub fn mul_vec(&self, v: Vec3) -> Vec3 {
        let m = &self.0;
        Vec3::new(
            m[0][0] * v.x + m[0][1] * v.y + m[0][2] * v.z,
            m[1][0] * v.x + m[1][1] * v.y + m[1][2] * v.z,
            m[2][0] * v.x + m[2][1] * v.y + m[2][2] * v.z,
        )
    }
    pub fn transpose_mul_vec(&self, v: Vec3) -> Vec3 {
        let m = &self.0;
        Vec3::new(
            m[0][0] * v.x + m[1][0] * v.y + m[2][0] * v.z,
            m[0][1] * v.x + m[1][1] * v.y + m[2][1] * v.z,
            m[0][2] * v.x + m[1][2] * v.y + m[2][2] * v.z,
        )
    }
}

/// Eigen decomposition of a symmetric 3x3 matrix (cyclic Jacobi).
/// Returns eigenvalues in descending order with matching unit eigenvectors.
pub fn sym_eigen(a: [[f64; 3]; 3]) -> ([f64; 3], [Vec3; 3]) {
    let mut a = a;
    let mut v = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
    for _ in 0..100 {
        let off = a[0][1].powi(2) + a[0][2].powi(2) + a[1][2].powi(2);
        if off < 1e-30 {
            break;
        }
        for (p, q) in [(0, 1), (0, 2), (1, 2)] {
            if a[p][q].abs() < 1e-300 {
                continue;
            }
            let theta = (a[q][q] - a[p][p]) / (2.0 * a[p][q]);
            let t = theta.signum() / (theta.abs() + (theta * theta + 1.0).sqrt());
            let t = if theta == 0.0 { 1.0 } else { t };
            let c = 1.0 / (t * t + 1.0).sqrt();
            let s = t * c;
            for k in 0..3 {
                let akp = a[k][p];
                let akq = a[k][q];
                a[k][p] = c * akp - s * akq;
                a[k][q] = s * akp + c * akq;
            }
            for k in 0..3 {
                let apk = a[p][k];
                let aqk = a[q][k];
                a[p][k] = c * apk - s * aqk;
                a[q][k] = s * apk + c * aqk;
            }
            for row in v.iter_mut() {
                let vkp = row[p];
                let vkq = row[q];
                row[p] = c * vkp - s * vkq;
                row[q] = s * vkp + c * vkq;
            }
        }
    }
    let mut idx = [0usize, 1, 2];
    idx.sort_by(|&i, &j| a[j][j].partial_cmp(&a[i][i]).unwrap());
    let vals = [a[idx[0]][idx[0]], a[idx[1]][idx[1]], a[idx[2]][idx[2]]];
    let vecs = idx.map(|j| Vec3::new(v[0][j], v[1][j], v[2][j]).normalized());
    (vals, vecs)
}

/// Unit quaternion (w, x, y, z) rotating body-frame vectors into the world frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Quat {
    pub w: f64,
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Quat {
    pub const IDENTITY: Quat = Quat { w: 1.0, x: 0.0, y: 0.0, z: 0.0 };

    pub fn normalized(self) -> Quat {
        let n = (self.w * self.w + self.x * self.x + self.y * self.y + self.z * self.z).sqrt();
        Quat { w: self.w / n, x: self.x / n, y: self.y / n, z: self.z / n }
    }

    /// Rotation matrix whose columns are the body axes expressed in the world frame.
    pub fn to_mat(self) -> Mat3 {
        let Quat { w, x, y, z } = self;
        Mat3([
            [1.0 - 2.0 * (y * y + z * z), 2.0 * (x * y - w * z), 2.0 * (x * z + w * y)],
            [2.0 * (x * y + w * z), 1.0 - 2.0 * (x * x + z * z), 2.0 * (y * z - w * x)],
            [2.0 * (x * z - w * y), 2.0 * (y * z + w * x), 1.0 - 2.0 * (x * x + y * y)],
        ])
    }

    pub fn from_mat(m: &Mat3) -> Quat {
        let m = &m.0;
        let tr = m[0][0] + m[1][1] + m[2][2];
        let q = if tr > 0.0 {
            let s = (tr + 1.0).sqrt() * 2.0;
            Quat { w: 0.25 * s, x: (m[2][1] - m[1][2]) / s, y: (m[0][2] - m[2][0]) / s, z: (m[1][0] - m[0][1]) / s }
        } else if m[0][0] > m[1][1] && m[0][0] > m[2][2] {
            let s = (1.0 + m[0][0] - m[1][1] - m[2][2]).sqrt() * 2.0;
            Quat { w: (m[2][1] - m[1][2]) / s, x: 0.25 * s, y: (m[0][1] + m[1][0]) / s, z: (m[0][2] + m[2][0]) / s }
        } else if m[1][1] > m[2][2] {
            let s = (1.0 + m[1][1] - m[0][0] - m[2][2]).sqrt() * 2.0;
            Quat { w: (m[0][2] - m[2][0]) / s, x: (m[0][1] + m[1][0]) / s, y: 0.25 * s, z: (m[1][2] + m[2][1]) / s }
        } else {
            let s = (1.0 + m[2][2] - m[0][0] - m[1][1]).sqrt() * 2.0;
            Quat { w: (m[1][0] - m[0][1]) / s, x: (m[0][2] + m[2][0]) / s, y: (m[1][2] + m[2][1]) / s, z: 0.25 * s }
        };
        q.normalized()
    }

    /// Time derivative for body angular velocity `w` (rad/s): q_dot = 0.5 q ⊗ (0, w).
    pub fn derivative(self, w: Vec3) -> Quat {
        let Quat { w: q0, x: q1, y: q2, z: q3 } = self;
        Quat {
            w: 0.5 * (-q1 * w.x - q2 * w.y - q3 * w.z),
            x: 0.5 * (q0 * w.x + q2 * w.z - q3 * w.y),
            y: 0.5 * (q0 * w.y - q1 * w.z + q3 * w.x),
            z: 0.5 * (q0 * w.z + q1 * w.y - q2 * w.x),
        }
    }
}

/// Cubic Hermite interpolation of f on [x0, x1] given values and slopes.
pub fn hermite(x: f64, x0: f64, x1: f64, f0: f64, f1: f64, d0: f64, d1: f64) -> f64 {
    let h = x1 - x0;
    let t = (x - x0) / h;
    let t2 = t * t;
    let t3 = t2 * t;
    (2.0 * t3 - 3.0 * t2 + 1.0) * f0
        + (t3 - 2.0 * t2 + t) * h * d0
        + (-2.0 * t3 + 3.0 * t2) * f1
        + (t3 - t2) * h * d1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eigen_diag() {
        let (vals, vecs) = sym_eigen([[1.0, 0.0, 0.0], [0.0, 5.0, 0.0], [0.0, 0.0, 3.0]]);
        assert!((vals[0] - 5.0).abs() < 1e-12);
        assert!((vecs[0].y.abs() - 1.0).abs() < 1e-12);
    }

    #[test]
    fn eigen_rotated() {
        // Matrix with a dominant direction along (1,1,0)/sqrt2.
        let (vals, vecs) = sym_eigen([[2.0, 1.0, 0.0], [1.0, 2.0, 0.0], [0.0, 0.0, 1.0]]);
        assert!((vals[0] - 3.0).abs() < 1e-10);
        assert!((vecs[0].x.abs() - std::f64::consts::FRAC_1_SQRT_2).abs() < 1e-10);
    }

    #[test]
    fn quat_mat_roundtrip() {
        let q = Quat { w: 0.9, x: 0.1, y: -0.3, z: 0.2 }.normalized();
        let q2 = Quat::from_mat(&q.to_mat());
        let d = (q.w - q2.w).abs() + (q.x - q2.x).abs() + (q.y - q2.y).abs() + (q.z - q2.z).abs();
        assert!(d < 1e-12);
    }
}
