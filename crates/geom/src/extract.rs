// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025-2026 西濱大将 (NISHIHAMA Daisuke)
//! Extraction of an axisymmetric body profile and a trapezoidal fin set from a rocket mesh.
//!
//! The mesh is sliced perpendicular to its principal (longest) axis. In each slice the body
//! radius is the median over angular bins of the outermost point, which ignores the thin
//! fins. Points well outside the body radius are clustered by angle to find the fins.

use crate::math::{sym_eigen, Vec3};
use crate::stl::Triangle;
use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use std::f64::consts::PI;

#[derive(Debug, Clone)]
pub struct ExtractOptions {
    pub n_slices: usize,
    /// Direction (in mesh coordinates) the nose points to. `None` = detect automatically.
    pub nose_direction: Option<Vec3>,
    /// A point counts as fin when it lies this fraction of the max body radius outside the body.
    pub fin_threshold: f64,
}

impl Default for ExtractOptions {
    fn default() -> Self {
        Self { n_slices: 600, nose_direction: None, fin_threshold: 0.05 }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfilePoint {
    /// Distance aft of the nose tip [m].
    pub x: f64,
    /// Body radius [m].
    pub r: f64,
    /// Fin tip radius minus body radius at this station (0 = no fin) [m].
    pub fin_span: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Nose {
    pub length: f64,
    pub base_radius: f64,
    /// Half angle of the profile at the nose/body joint [deg] (cone: atan(R/L), tangent ogive: ~0).
    pub joint_half_angle_deg: f64,
    /// Nose volume divided by the enclosing cylinder volume (cone 1/3, tangent ogive ~0.5+).
    pub volume_ratio: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Boattail {
    pub length: f64,
    pub r_fore: f64,
    pub r_aft: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FinSet {
    pub count: usize,
    pub root_chord: f64,
    pub tip_chord: f64,
    pub span: f64,
    /// Axial distance from root leading edge to tip leading edge.
    pub sweep: f64,
    pub thickness: f64,
    /// Root leading edge, distance aft of the nose tip.
    pub x_le_root: f64,
    /// Body radius at the fin root.
    pub body_radius: f64,
    /// Fin cant angle [deg]; positive cant produces a positive roll moment about the forward body x axis.
    #[serde(default)]
    pub cant_deg: f64,
}

impl FinSet {
    /// Planform area of a single fin.
    pub fn area(&self) -> f64 {
        0.5 * (self.root_chord + self.tip_chord) * self.span
    }
    /// Mean aerodynamic chord length.
    pub fn mac(&self) -> f64 {
        let (a, b) = (self.root_chord, self.tip_chord);
        if a + b <= 0.0 {
            return 0.0;
        }
        2.0 / 3.0 * (a + b - a * b / (a + b))
    }
    /// Axial offset of the MAC leading edge from the root leading edge.
    pub fn mac_le_offset(&self) -> f64 {
        let (a, b) = (self.root_chord, self.tip_chord);
        if a + b <= 0.0 {
            return 0.0;
        }
        self.sweep * (a + 2.0 * b) / (3.0 * (a + b))
    }
    /// Sweep angle of the mid-chord line [rad].
    pub fn midchord_sweep(&self) -> f64 {
        ((self.sweep + 0.5 * self.tip_chord - 0.5 * self.root_chord) / self.span).atan()
    }
    /// Sweep angle of the leading edge [rad].
    pub fn le_sweep(&self) -> f64 {
        (self.sweep / self.span).atan()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Frame {
    /// Mesh-coordinate point on the body axis used as origin of the radial coordinates.
    pub axis_point: [f64; 3],
    /// Unit vector (mesh coordinates) pointing from the nose towards the tail.
    pub aft_direction: [f64; 3],
    /// Nose tip position along the aft direction relative to `axis_point`.
    pub nose_offset: f64,
}

impl Frame {
    /// Body axes expressed in mesh (STL) coordinates: x forward (towards the nose), y and z across.
    ///
    /// y is `aft × helper` with helper the mesh x axis (or y when the body lies along x), and
    /// z completes the right-handed set. Every component that needs a body-fixed lateral direction
    /// (6-component coefficients, CG offsets, products of inertia, thrust misalignment) uses these axes.
    pub fn body_basis(&self) -> [Vec3; 3] {
        let aft = Vec3::new(self.aft_direction[0], self.aft_direction[1], self.aft_direction[2]).normalized();
        let helper = if aft.x.abs() < 0.9 { Vec3::new(1.0, 0.0, 0.0) } else { Vec3::new(0.0, 1.0, 0.0) };
        let ey = aft.cross(helper).normalized();
        let ez_aft = aft.cross(ey);
        [aft * -1.0, ey, ez_aft * -1.0]
    }
    /// Mesh point -> body coordinates with the origin at the nose tip (x forward, so x <= 0 on the body).
    pub fn to_body(&self, v: Vec3) -> Vec3 {
        let [ex, ey, ez] = self.body_basis();
        let d = v - Vec3::new(self.axis_point[0], self.axis_point[1], self.axis_point[2]);
        Vec3::new(d.dot(ex) + self.nose_offset, d.dot(ey), d.dot(ez))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Geometry {
    pub length: f64,
    pub ref_radius: f64,
    pub ref_area: f64,
    pub base_radius: f64,
    pub nose: Nose,
    pub boattail: Option<Boattail>,
    pub wetted_area_body: f64,
    pub volume: f64,
    pub planform_area: f64,
    pub planform_centroid: f64,
    pub fins: Option<FinSet>,
    pub frame: Frame,
    pub profile: Vec<ProfilePoint>,
    /// Plausibility findings of the extraction (orientation, proportions, protrusions); empty when
    /// nothing is suspicious.
    #[serde(default)]
    pub warnings: Vec<String>,
}

struct Slice {
    x: f64,
    r_body: f64,
    /// Segment endpoints in polar form (r, theta).
    segs: Vec<[(f64, f64); 2]>,
}

const N_BINS: usize = 72;

pub fn extract(tris: &[Triangle], opt: &ExtractOptions) -> Result<Geometry> {
    if opt.n_slices < 20 {
        bail!("n_slices must be at least 20");
    }
    // Area-weighted centroid and covariance of triangle centroids.
    let mut area = 0.0;
    let mut c = Vec3::ZERO;
    for t in tris {
        let a = 0.5 * (t[1] - t[0]).cross(t[2] - t[0]).norm();
        area += a;
        c += (t[0] + t[1] + t[2]) / 3.0 * a;
    }
    if area <= 0.0 {
        bail!("mesh has zero surface area");
    }
    c = c / area;
    let mut cov = [[0.0; 3]; 3];
    for t in tris {
        let a = 0.5 * (t[1] - t[0]).cross(t[2] - t[0]).norm();
        for v in t {
            let d = *v - c;
            for i in 0..3 {
                for j in 0..3 {
                    cov[i][j] += a * d[i] * d[j];
                }
            }
        }
    }
    let ex = match opt.nose_direction {
        Some(d) => -d.normalized(),
        None => sym_eigen(cov).1[0],
    };
    let helper = if ex.x.abs() < 0.9 { Vec3::new(1.0, 0.0, 0.0) } else { Vec3::new(0.0, 1.0, 0.0) };
    let ey = ex.cross(helper).normalized();
    let ez = ex.cross(ey);
    let local: Vec<[Vec3; 3]> = tris
        .iter()
        .map(|t| {
            t.map(|v| {
                let d = v - c;
                Vec3::new(d.dot(ex), d.dot(ey), d.dot(ez))
            })
        })
        .collect();
    let (mut xmin, mut xmax) = (f64::INFINITY, f64::NEG_INFINITY);
    for t in &local {
        for v in t {
            xmin = xmin.min(v.x);
            xmax = xmax.max(v.x);
        }
    }
    let length = xmax - xmin;
    if length <= 0.0 {
        bail!("degenerate mesh");
    }

    let mut slices = slice_mesh(&local, xmin, length, opt.n_slices);
    let n = slices.len();
    let dx = length / n as f64;
    let (r_max, r_note) = reference_radius(&slices);
    if r_max <= 0.0 {
        bail!("could not determine body radius");
    }
    let mut warnings: Vec<String> = Vec::new();
    warnings.extend(r_note);

    // Orientation. The fin set is the most reliable cue (fins sit in the aft part of the body);
    // the radius at the extreme ends (a pointed or rounded nose tip is smaller than the base end,
    // also with a boattail or a protruding nozzle) is the fallback.
    let fin_thr = opt.fin_threshold * r_max;
    let fins_fwd = {
        let info: Vec<FinSlice> = slices.iter().map(|s| fin_slice(s, fin_thr)).collect();
        detect_fins(&slices, &info, dx)
    };
    let fin_pos = fins_fwd.as_ref().map(|f| (f.x_le_root + 0.5 * f.root_chord) / length);
    let front_tip: f64 = slices.iter().take(2).map(|s| s.r_body).sum();
    let back_tip: f64 = slices.iter().rev().take(2).map(|s| s.r_body).sum();
    let shape_flip = front_tip > back_tip;
    let flip = match (opt.nose_direction, fin_pos) {
        (Some(_), _) => false,
        (None, Some(p)) if (p - 0.5).abs() > 0.08 => {
            let by_fins = p < 0.5;
            if by_fins != shape_flip {
                warnings.push(
                    "the nose/tail shape and the fin position suggest opposite orientations; the fin position was used (set nose_direction to override)".into(),
                );
            }
            by_fins
        }
        (None, _) => {
            warnings.push("orientation decided from the end radii only (no clear fin set); check the extracted nose/tail or set nose_direction".into());
            shape_flip
        }
    };
    let (aft, nose_offset) = if flip {
        slices.reverse();
        for s in &mut slices {
            s.x = length - s.x;
        }
        (-ex, -xmax)
    } else {
        (ex, xmin)
    };

    // Nose.
    let i_nose = slices.iter().position(|s| s.r_body >= 0.999 * r_max).unwrap_or(0);
    let nose_len = slices[i_nose].x;
    let nose_r = slices[i_nose].r_body;
    let k0 = ((i_nose as f64) * 0.9) as usize;
    let joint = if i_nose > k0 + 1 { ((slices[i_nose].r_body - slices[k0].r_body) / (slices[i_nose].x - slices[k0].x)).atan() } else { 0.0 };
    let nose_vol: f64 = slices[..=i_nose].iter().map(|s| PI * s.r_body * s.r_body * dx).sum();
    let volume_ratio = if nose_len > 0.0 { nose_vol / (PI * nose_r * nose_r * nose_len) } else { 0.0 };

    // Boattail.
    let base_radius = slices[n - 1].r_body;
    let i_aft = slices.iter().rposition(|s| s.r_body >= 0.99 * r_max).unwrap_or(n - 1);
    let boattail = if n - 1 - i_aft > 2 && base_radius < 0.97 * r_max {
        Some(Boattail { length: length - slices[i_aft].x, r_fore: slices[i_aft].r_body, r_aft: base_radius })
    } else {
        None
    };

    // Integral properties of the body of revolution.
    let mut wet = 0.0;
    let mut vol = 0.0;
    let mut plan = 0.0;
    let mut plan_x = 0.0;
    for i in 0..n {
        let r = slices[i].r_body;
        let drdx = if i + 1 < n { (slices[i + 1].r_body - r) / dx } else { 0.0 };
        wet += 2.0 * PI * r * dx * (1.0 + drdx * drdx).sqrt();
        vol += PI * r * r * dx;
        plan += 2.0 * r * dx;
        plan_x += 2.0 * r * dx * slices[i].x;
    }

    let fin_info: Vec<FinSlice> = slices.iter().map(|s| fin_slice(s, fin_thr)).collect();
    let fins = detect_fins(&slices, &fin_info, dx);

    // Plausibility checks.
    let d = 2.0 * r_max;
    if length / d < 3.0 || length / d > 40.0 {
        warnings.push(format!(
            "length/diameter ratio {:.1} is unusual for a rocket (length {length:.3} m, diameter {d:.3} m); check the STL scale and units",
            length / d
        ));
    }
    if nose_len < 0.4 * d || nose_len > 8.0 * d {
        warnings.push(format!("detected nose length {nose_len:.3} m ({:.1} calibers) is implausible", nose_len / d));
    }
    if base_radius < 0.6 * r_max {
        warnings.push(format!(
            "the base radius is only {:.0} % of the body radius (protruding nozzle or strong boattail); base drag uses this radius",
            100.0 * base_radius / r_max
        ));
    }
    if let Some(f) = &fins {
        let xc = (f.x_le_root + 0.5 * f.root_chord) / length;
        if f.x_le_root < 0.5 * length {
            warnings.push(format!(
                "the fin root leading edge is at {:.0} % of the length, in the forward half; the body may be reversed (nose_direction)",
                100.0 * f.x_le_root / length
            ));
        } else if xc < 0.6 {
            warnings.push(format!("the fins are centred at {:.0} % of the length, unusually far forward", 100.0 * xc));
        }
        if f.span > 2.0 * d {
            warnings.push(format!("the fin span {:.3} m exceeds two body diameters ({d:.3} m)", f.span));
        }
    }

    let profile = slices.iter().zip(&fin_info).map(|(s, f)| ProfilePoint { x: s.x, r: s.r_body, fin_span: f.span }).collect();

    Ok(Geometry {
        length,
        ref_radius: r_max,
        ref_area: PI * r_max * r_max,
        base_radius,
        nose: Nose { length: nose_len, base_radius: nose_r, joint_half_angle_deg: joint.to_degrees(), volume_ratio },
        boattail,
        wetted_area_body: wet,
        volume: vol,
        planform_area: plan,
        planform_centroid: if plan > 0.0 { plan_x / plan } else { 0.0 },
        fins,
        frame: Frame { axis_point: [c.x, c.y, c.z], aft_direction: [aft.x, aft.y, aft.z], nose_offset },
        profile,
        warnings,
    })
}

/// Reference (maximum body) radius that ignores short protrusions: the largest per-slice radius,
/// unless it exceeds the 90th percentile by more than 15 %, which only a short ring or collar
/// does (a cone-only or boattailed body stays below that ratio); then the percentile is used.
fn reference_radius(slices: &[Slice]) -> (f64, Option<String>) {
    let max = slices.iter().map(|s| s.r_body).fold(0.0, f64::max);
    let mut v: Vec<f64> = slices.iter().map(|s| s.r_body).filter(|&r| r > 0.0).collect();
    if v.is_empty() {
        return (0.0, None);
    }
    v.sort_by(|a, b| a.total_cmp(b));
    let p90 = v[((v.len() - 1) as f64 * 0.9) as usize];
    if max <= 1.15 * p90 {
        (max, None)
    } else {
        (p90, Some(format!("a short feature with radius {max:.4} m exceeds the typical body radius {p90:.4} m and was ignored")))
    }
}

fn slice_mesh(local: &[[Vec3; 3]], xmin: f64, length: f64, n: usize) -> Vec<Slice> {
    let dx = length / n as f64;
    let mut slices: Vec<Slice> = (0..n).map(|i| Slice { x: (i as f64 + 0.5) * dx, r_body: 0.0, segs: Vec::new() }).collect();
    for t in local {
        let xs = t.map(|v| v.x - xmin);
        let lo = xs.iter().copied().fold(f64::INFINITY, f64::min);
        let hi = xs.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        let i0 = ((lo / dx - 0.5).ceil().max(0.0)) as usize;
        let i1 = (((hi / dx - 0.5).floor()) as isize).min(n as isize - 1);
        if i1 < i0 as isize {
            continue;
        }
        for (i, slice) in slices.iter_mut().enumerate().take(i1 as usize + 1).skip(i0) {
            let xs_i = (i as f64 + 0.5) * dx;
            let mut pts = [(0.0, 0.0); 2];
            let mut k = 0;
            for e in 0..3 {
                let (a, b) = (e, (e + 1) % 3);
                let (above_a, above_b) = (xs[a] >= xs_i, xs[b] >= xs_i);
                if above_a != above_b && k < 2 {
                    let s = (xs_i - xs[a]) / (xs[b] - xs[a]);
                    let p = t[a] + (t[b] - t[a]) * s;
                    pts[k] = (p.y.hypot(p.z), p.z.atan2(p.y));
                    k += 1;
                }
            }
            if k == 2 {
                slice.segs.push(pts);
            }
        }
    }
    for s in &mut slices {
        let mut bins = [0.0f64; N_BINS];
        for seg in &s.segs {
            for &(r, th) in seg {
                let b = (((th + PI) / (2.0 * PI) * N_BINS as f64) as usize).min(N_BINS - 1);
                bins[b] = bins[b].max(r);
            }
        }
        let mut filled: Vec<f64> = bins.iter().copied().filter(|&r| r > 0.0).collect();
        s.r_body = if filled.len() >= 3 {
            filled.sort_by(|a, b| a.total_cmp(b));
            filled[filled.len() / 2]
        } else {
            filled.iter().copied().fold(0.0, f64::max)
        };
    }
    slices
}

#[derive(Debug, Clone, Default)]
struct FinSlice {
    clusters: Vec<Vec<(f64, f64)>>,
    span: f64,
    attached: bool,
}

fn fin_slice(s: &Slice, thr: f64) -> FinSlice {
    let limit = s.r_body + thr;
    let mut out: Vec<(f64, f64)> = Vec::new();
    let mut attached = false;
    for seg in &s.segs {
        let (o0, o1) = (seg[0].0 > limit, seg[1].0 > limit);
        if o0 != o1 {
            attached = true;
        }
        for (&p, o) in seg.iter().zip([o0, o1]) {
            if o {
                out.push(p);
            }
        }
    }
    if out.is_empty() {
        return FinSlice::default();
    }
    out.sort_by(|a, b| a.1.total_cmp(&b.1));
    let gap = 10f64.to_radians();
    let mut clusters: Vec<Vec<(f64, f64)>> = vec![vec![out[0]]];
    for w in out.windows(2) {
        if w[1].1 - w[0].1 > gap {
            clusters.push(Vec::new());
        }
        clusters.last_mut().unwrap().push(w[1]);
    }
    if clusters.len() > 1 {
        let first = clusters[0][0].1;
        let last = clusters.last().unwrap().last().unwrap().1;
        if first + 2.0 * PI - last <= gap {
            let tail = clusters.pop().unwrap();
            clusters[0].extend(tail);
        }
    }
    let span = out.iter().map(|p| p.0).fold(0.0, f64::max) - s.r_body;
    FinSlice { clusters, span, attached }
}

fn detect_fins(slices: &[Slice], info: &[FinSlice], dx: f64) -> Option<FinSet> {
    // Largest contiguous run of slices containing fin points (weighted by span).
    let mut best: Option<(usize, usize, f64)> = None;
    let mut i = 0;
    while i < info.len() {
        if info[i].clusters.is_empty() {
            i += 1;
            continue;
        }
        let start = i;
        let mut w = 0.0;
        while i < info.len() && !info[i].clusters.is_empty() {
            w += info[i].span;
            i += 1;
        }
        if best.is_none_or(|b| w > b.2) {
            best = Some((start, i - 1, w));
        }
    }
    let (g0, g1, _) = best?;
    if g1 - g0 < 2 {
        return None;
    }
    let group = g0..=g1;

    let mut counts = std::collections::HashMap::new();
    for k in group.clone() {
        *counts.entry(info[k].clusters.len()).or_insert(0usize) += 1;
    }
    let count = counts.into_iter().max_by_key(|&(c, n)| (n, c)).map(|(c, _)| c)?;

    let root: Vec<usize> = group.clone().filter(|&k| info[k].attached).collect();
    let (r0, r1) = (*root.first()?, *root.last()?);
    // Fin points are only visible beyond the threshold radius, so extrapolate the span
    // profile linearly to zero at both ends of the root to locate the edges.
    let edge = |k: usize, k2: usize, dir: f64| {
        let (s1, s2) = (info[k].span, info[k2].span);
        let half = slices[k].x + dir * 0.5 * dx;
        if k2 != k && s2 > s1 {
            let x0 = slices[k].x + dir * s1 * dx / (s2 - s1);
            if dir < 0.0 {
                x0.max(slices[k].x - 3.0 * dx).min(half)
            } else {
                x0.min(slices[k].x + 3.0 * dx).max(half)
            }
        } else {
            half
        }
    };
    let x_le_root = edge(r0, (r0 + 1).min(r1), -1.0);
    let x_te_root = edge(r1, r1.saturating_sub(1).max(r0), 1.0);
    let root_chord = x_te_root - x_le_root;
    let span = group.clone().map(|k| info[k].span).fold(0.0, f64::max);
    let fin_area: f64 = group.clone().map(|k| info[k].span * dx).sum();
    let tip_chord = (2.0 * fin_area / span - root_chord).clamp(0.0, 3.0 * root_chord);
    let k_tip = group.clone().find(|&k| info[k].span >= 0.97 * span)?;
    let sweep = (slices[k_tip].x - 0.5 * dx - x_le_root) / 0.97;

    let mut body_r: Vec<f64> = root.iter().map(|&k| slices[k].r_body).collect();
    body_r.sort_by(|a, b| a.total_cmp(b));
    let body_radius = body_r[body_r.len() / 2];

    let mut th: Vec<f64> = Vec::new();
    for k in group {
        for cl in &info[k].clusters {
            let (sx, sy) = cl.iter().fold((0.0, 0.0), |acc, p| (acc.0 + p.1.cos(), acc.1 + p.1.sin()));
            let phi = sy.atan2(sx);
            let lim = slices[k].r_body + 0.9 * span;
            let d: Vec<f64> = cl.iter().filter(|p| p.0 < lim).map(|p| p.0 * (p.1 - phi).sin()).collect();
            if d.len() >= 2 {
                let lo = d.iter().copied().fold(f64::INFINITY, f64::min);
                let hi = d.iter().copied().fold(f64::NEG_INFINITY, f64::max);
                th.push(hi - lo);
            }
        }
    }
    th.sort_by(|a, b| a.total_cmp(b));
    let thickness = if th.is_empty() { 0.0 } else { th[th.len() / 2] };

    Some(FinSet { count, root_chord, tip_chord, span, sweep, thickness, x_le_root, body_radius, cant_deg: 0.0 })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::math::Quat;
    use crate::sample::{NoseShape, SampleRocket};

    fn close(a: f64, b: f64, tol: f64) -> bool {
        (a - b).abs() <= tol
    }

    #[test]
    fn body_basis_is_right_handed_and_points_to_the_nose() {
        let fr = Frame { axis_point: [1.0, 2.0, 3.0], aft_direction: [0.0, 0.0, -1.0], nose_offset: 0.4 };
        let [ex, ey, ez] = fr.body_basis();
        assert!((ex.cross(ey) - ez).norm() < 1e-12);
        assert!((ex - Vec3::new(0.0, 0.0, 1.0)).norm() < 1e-12);
        // The nose tip sits at the body origin; a point 1 m aft has x = -1.
        let nose = Vec3::new(1.0, 2.0, 3.0 - 0.4);
        assert!(fr.to_body(nose).norm() < 1e-12);
        assert!((fr.to_body(nose + Vec3::new(0.0, 0.0, -1.0)).x + 1.0).abs() < 1e-12);
    }

    #[test]
    fn recovers_sample_rocket_in_arbitrary_orientation() {
        let rk = SampleRocket::default();
        let q = Quat { w: 0.8, x: 0.3, y: -0.4, z: 0.2 }.normalized().to_mat();
        let off = Vec3::new(1.0, -2.0, 0.5);
        // Scale to millimetres, rotate and translate, then scale back.
        let tris: Vec<Triangle> = rk.mesh().iter().map(|t| t.map(|v| q.mul_vec(v * 1000.0) + off)).collect();
        let tris: Vec<Triangle> = tris.iter().map(|t| t.map(|v| v * 0.001)).collect();
        let g = extract(&tris, &ExtractOptions::default()).unwrap();
        // Nose must be detected at the right end.
        let aft = Vec3::new(g.frame.aft_direction[0], g.frame.aft_direction[1], g.frame.aft_direction[2]);
        assert!(aft.dot(q.mul_vec(Vec3::new(1.0, 0.0, 0.0))) > 0.999);
        assert!(close(g.length, rk.length, 0.005), "length {}", g.length);
        assert!(close(g.ref_radius, rk.radius, 0.0005), "radius {}", g.ref_radius);
        assert!(close(g.nose.length, rk.nose_length, 0.03), "nose {}", g.nose.length);
        let f = g.fins.expect("fins detected");
        assert_eq!(f.count, 4);
        assert!(close(f.root_chord, rk.root_chord, 0.006), "root {}", f.root_chord);
        assert!(close(f.tip_chord, rk.tip_chord, 0.008), "tip {}", f.tip_chord);
        assert!(close(f.span, rk.span, 0.002), "span {}", f.span);
        assert!(close(f.sweep, rk.sweep, 0.008), "sweep {}", f.sweep);
        assert!(close(f.thickness, rk.thickness, 0.0008), "thickness {}", f.thickness);
        assert!(close(f.x_le_root, rk.length - rk.root_chord, 0.006), "x_le {}", f.x_le_root);
    }

    #[test]
    fn forward_swept_fin_keeps_negative_sweep() {
        let rk = SampleRocket { sweep: -0.03, ..Default::default() };
        let g = extract(&rk.mesh(), &ExtractOptions::default()).unwrap();
        let f = g.fins.expect("fins detected");
        assert!(close(f.sweep, rk.sweep, 0.008), "sweep {}", f.sweep);
        assert!(f.sweep < -0.01, "sweep {}", f.sweep);
    }

    #[test]
    fn cone_nose_joint_angle_and_three_fins() {
        let rk = SampleRocket { nose_shape: NoseShape::Cone, fin_count: 3, fin_offset_from_base: 0.05, ..Default::default() };
        let g = extract(&rk.mesh(), &ExtractOptions::default()).unwrap();
        let expect = (rk.radius / rk.nose_length).atan().to_degrees();
        assert!(close(g.nose.joint_half_angle_deg, expect, 0.5), "joint {}", g.nose.joint_half_angle_deg);
        assert!(close(g.nose.volume_ratio, 1.0 / 3.0, 0.03), "vr {}", g.nose.volume_ratio);
        let f = g.fins.unwrap();
        assert_eq!(f.count, 3);
        assert!(close(f.x_le_root, rk.length - 0.05 - rk.root_chord, 0.006));
    }

    use crate::sample::SampleRocket as Rk;

    /// Rotate the mesh so the nose points to -x (reversed relative to the sample mesh).
    fn reversed(tris: &[Triangle], length: f64) -> Vec<Triangle> {
        tris.iter().map(|t| t.map(|v| Vec3::new(length - v.x, v.y, -v.z))).collect()
    }

    fn check(rk: &Rk, mesh: &[Triangle], body_len: f64) -> Geometry {
        let g = extract(mesh, &ExtractOptions::default()).unwrap();
        let mesh_len = body_len;
        assert!(close(g.length, mesh_len, 0.01), "length {}", g.length);
        assert!(close(g.nose.length, rk.nose_length, 0.02), "nose {} vs {}", g.nose.length, rk.nose_length);
        let f = g.fins.as_ref().expect("fins");
        assert_eq!(f.count, rk.fin_count);
        let x_le = rk.length - rk.fin_offset_from_base - rk.root_chord;
        assert!(close(f.x_le_root, x_le, 0.01), "fin x_le {} vs {x_le}", f.x_le_root);
        g
    }

    #[test]
    fn hemispherical_nose_with_long_boattail_is_oriented_correctly() {
        // Regression: this body was detected backwards (fins at x = 0.15 m).
        let rk = Rk {
            nose_shape: NoseShape::Hemisphere,
            nose_length: 0.05,
            boattail_length: 0.15,
            boattail_end_radius: 0.035,
            fin_offset_from_base: 0.15,
            ..Default::default()
        };
        let g = check(&rk, &rk.mesh(), rk.length);
        assert!(g.boattail.is_some());
        assert!(close(g.base_radius, 0.035, 0.002));
        assert!(g.warnings.is_empty(), "{:?}", g.warnings);
        // Reversed in the mesh: oriented correctly as well (nose length and fin position checked).
        let g2 = check(&rk, &reversed(&rk.mesh(), rk.length), rk.length);
        assert!(g2.warnings.is_empty(), "{:?}", g2.warnings);
    }

    #[test]
    fn reversed_sample_rocket_is_flipped_automatically() {
        let rk = Rk::default();
        let g = check(&rk, &reversed(&rk.mesh(), rk.length), rk.length);
        assert!(g.warnings.is_empty(), "{:?}", g.warnings);
        assert!(g.profile.first().unwrap().r < 0.02 && g.profile.last().unwrap().r > 0.045);
    }

    #[test]
    fn launch_lug_does_not_disturb_nose_or_fins() {
        let rk = Rk { lug_x: Some(0.8), ..Default::default() };
        let g = check(&rk, &rk.mesh(), rk.length);
        assert!(close(g.ref_radius, rk.radius, 0.0005), "radius {}", g.ref_radius);
        // Lug close to the nose.
        let rk = Rk { lug_x: Some(0.45), ..Default::default() };
        let g = check(&rk, &rk.mesh(), rk.length);
        assert!(close(g.ref_radius, rk.radius, 0.0005));
    }

    #[test]
    fn protruding_nozzle_keeps_nose_detection_and_warns() {
        let rk = Rk { nozzle_length: 0.06, nozzle_radius: 0.02, ..Default::default() };
        let g = check(&rk, &rk.mesh(), rk.length + rk.nozzle_length);
        assert!(close(g.ref_radius, rk.radius, 0.0005));
        assert!(g.warnings.iter().any(|w| w.contains("base radius")), "{:?}", g.warnings);
        // Reversed nozzle body is also recognised.
        let g = check(&rk, &reversed(&rk.mesh(), rk.length + rk.nozzle_length), rk.length + rk.nozzle_length);
        assert!(g.profile.first().unwrap().r < 0.02);
    }

    #[test]
    fn plausible_rocket_has_no_warnings_and_odd_proportions_warn() {
        let g = extract(&Rk::default().mesh(), &ExtractOptions::default()).unwrap();
        assert!(g.warnings.is_empty(), "{:?}", g.warnings);
        // A stubby body: length/diameter < 3.
        let rk = Rk { length: 0.28, nose_length: 0.1, root_chord: 0.08, tip_chord: 0.04, sweep: 0.03, span: 0.03, ..Default::default() };
        let g = extract(&rk.mesh(), &ExtractOptions::default()).unwrap();
        assert!(g.warnings.iter().any(|w| w.contains("length/diameter")), "{:?}", g.warnings);
    }

    #[test]
    fn explicit_nose_direction_is_respected_and_forward_fins_warn() {
        let rk = Rk::default();
        // Declare the nose at the tail end (+x; the true nose points to -x): no automatic flip, warning about the forward fins.
        let opt = ExtractOptions { nose_direction: Some(Vec3::new(1.0, 0.0, 0.0)), ..Default::default() };
        let g = extract(&rk.mesh(), &opt).unwrap();
        assert!(g.warnings.iter().any(|w| w.contains("forward half")), "{:?}", g.warnings);
    }
}
