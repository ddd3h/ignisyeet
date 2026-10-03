// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025-2026 西濱大将 (NISHIHAMA Daisuke)
//! Procedural rocket mesh used for examples and tests.
//! The generated mesh has the nose tip at the origin and the body axis along +x (metres).

use crate::math::Vec3;
use crate::stl::Triangle;
use std::f64::consts::PI;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NoseShape {
    Cone,
    TangentOgive,
}

#[derive(Debug, Clone)]
pub struct SampleRocket {
    pub nose_shape: NoseShape,
    pub nose_length: f64,
    pub radius: f64,
    /// Total length from nose tip to base.
    pub length: f64,
    pub fin_count: usize,
    pub root_chord: f64,
    pub tip_chord: f64,
    pub span: f64,
    /// Axial distance from root leading edge to tip leading edge.
    pub sweep: f64,
    pub thickness: f64,
    /// Axial gap between root trailing edge and the base.
    pub fin_offset_from_base: f64,
    pub segments: usize,
}

impl Default for SampleRocket {
    fn default() -> Self {
        Self {
            nose_shape: NoseShape::TangentOgive,
            nose_length: 0.30,
            radius: 0.05,
            length: 1.50,
            fin_count: 4,
            root_chord: 0.20,
            tip_chord: 0.10,
            span: 0.10,
            sweep: 0.08,
            thickness: 0.004,
            fin_offset_from_base: 0.0,
            segments: 72,
        }
    }
}

impl SampleRocket {
    pub fn nose_radius(&self, x: f64) -> f64 {
        let (l, r) = (self.nose_length, self.radius);
        match self.nose_shape {
            NoseShape::Cone => r * x / l,
            NoseShape::TangentOgive => {
                let rho = (r * r + l * l) / (2.0 * r);
                ((rho * rho - (l - x).powi(2)).sqrt() + r - rho).max(0.0)
            }
        }
    }

    pub fn mesh(&self) -> Vec<Triangle> {
        let mut tris = Vec::new();
        // Axial profile (x, r).
        let n_nose = 60;
        let mut prof: Vec<(f64, f64)> = (1..=n_nose)
            .map(|i| {
                let x = self.nose_length * i as f64 / n_nose as f64;
                (x, self.nose_radius(x))
            })
            .collect();
        let n_body = 40;
        for i in 1..=n_body {
            let x = self.nose_length + (self.length - self.nose_length) * i as f64 / n_body as f64;
            prof.push((x, self.radius));
        }
        let m = self.segments;
        let ring = |x: f64, r: f64, j: usize| {
            let a = 2.0 * PI * j as f64 / m as f64;
            Vec3::new(x, r * a.cos(), r * a.sin())
        };
        let tip = Vec3::ZERO;
        let (x0, r0) = prof[0];
        for j in 0..m {
            tris.push([tip, ring(x0, r0, j + 1), ring(x0, r0, j)]);
        }
        for w in prof.windows(2) {
            let ((xa, ra), (xb, rb)) = (w[0], w[1]);
            for j in 0..m {
                let (a0, a1, b0, b1) = (ring(xa, ra, j), ring(xa, ra, j + 1), ring(xb, rb, j), ring(xb, rb, j + 1));
                tris.push([a0, a1, b1]);
                tris.push([a0, b1, b0]);
            }
        }
        let base = Vec3::new(self.length, 0.0, 0.0);
        for j in 0..m {
            tris.push([base, ring(self.length, self.radius, j), ring(self.length, self.radius, j + 1)]);
        }
        for k in 0..self.fin_count {
            let phi = 2.0 * PI * k as f64 / self.fin_count as f64 + PI / self.fin_count as f64;
            tris.extend(self.fin_box(phi));
        }
        tris
    }

    fn fin_box(&self, phi: f64) -> Vec<Triangle> {
        let er = Vec3::new(0.0, phi.cos(), phi.sin());
        let et = Vec3::new(0.0, -phi.sin(), phi.cos());
        let x_te = self.length - self.fin_offset_from_base;
        let x_le = x_te - self.root_chord;
        // Embed the root slightly into the body, continuing the edge lines inwards so the
        // fin meets the body surface exactly at the nominal chord.
        let depth = 0.1 * self.radius;
        let r_in = self.radius - depth;
        let r_out = self.radius + self.span;
        let te_slope = (x_le + self.sweep + self.tip_chord - x_te) / self.span;
        let corners = [
            (x_le - self.sweep / self.span * depth, r_in),
            ((x_te - te_slope * depth).min(self.length), r_in),
            (x_le + self.sweep + self.tip_chord, r_out),
            (x_le + self.sweep, r_out),
        ];
        let h = self.thickness / 2.0;
        let p = |c: (f64, f64), d: f64| Vec3::new(c.0, 0.0, 0.0) + er * c.1 + et * d;
        let a: Vec<Vec3> = corners.iter().map(|&c| p(c, h)).collect();
        let b: Vec<Vec3> = corners.iter().map(|&c| p(c, -h)).collect();
        let mut t = vec![[a[0], a[1], a[2]], [a[0], a[2], a[3]], [b[0], b[2], b[1]], [b[0], b[3], b[2]]];
        for i in 0..4 {
            let j = (i + 1) % 4;
            t.push([a[i], b[i], b[j]]);
            t.push([a[i], b[j], a[j]]);
        }
        t
    }
}
