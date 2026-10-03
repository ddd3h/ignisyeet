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
        .map(|t| t.map(|v| {
            let d = v - c;
            Vec3::new(d.dot(ex), d.dot(ey), d.dot(ez))
        }))
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
    let flip = match opt.nose_direction {
        Some(_) => false,
        None => {
            let k = (n / 30).max(2);
            let front: f64 = slices[..k].iter().map(|s| s.r_body).sum();
            let back: f64 = slices[n - k..].iter().map(|s| s.r_body).sum();
            front > back
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

    let dx = length / n as f64;
    let r_max = slices.iter().map(|s| s.r_body).fold(0.0, f64::max);
    if r_max <= 0.0 {
        bail!("could not determine body radius");
    }

    // Nose.
    let i_nose = slices.iter().position(|s| s.r_body >= 0.999 * r_max).unwrap_or(0);
    let nose_len = slices[i_nose].x;
    let nose_r = slices[i_nose].r_body;
    let k0 = ((i_nose as f64) * 0.9) as usize;
    let joint = if i_nose > k0 + 1 {
        ((slices[i_nose].r_body - slices[k0].r_body) / (slices[i_nose].x - slices[k0].x)).atan()
    } else {
        0.0
    };
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

    let fin_thr = opt.fin_threshold * r_max;
    let fin_info: Vec<FinSlice> = slices.iter().map(|s| fin_slice(s, fin_thr)).collect();
    let fins = detect_fins(&slices, &fin_info, dx);

    let profile = slices
        .iter()
        .zip(&fin_info)
        .map(|(s, f)| ProfilePoint { x: s.x, r: s.r_body, fin_span: f.span })
        .collect();

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
    })
}

fn slice_mesh(local: &[[Vec3; 3]], xmin: f64, length: f64, n: usize) -> Vec<Slice> {
    let dx = length / n as f64;
    let mut slices: Vec<Slice> = (0..n)
        .map(|i| Slice { x: (i as f64 + 0.5) * dx, r_body: 0.0, segs: Vec::new() })
        .collect();
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
            filled.sort_by(|a, b| a.partial_cmp(b).unwrap());
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
    out.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
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
            if dir < 0.0 { x0.max(slices[k].x - 3.0 * dx).min(half) } else { x0.min(slices[k].x + 3.0 * dx).max(half) }
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
    let sweep = ((slices[k_tip].x - 0.5 * dx - x_le_root) / 0.97).max(0.0);

    let mut body_r: Vec<f64> = root.iter().map(|&k| slices[k].r_body).collect();
    body_r.sort_by(|a, b| a.partial_cmp(b).unwrap());
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
    th.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let thickness = if th.is_empty() { 0.0 } else { th[th.len() / 2] };

    Some(FinSet { count, root_chord, tip_chord, span, sweep, thickness, x_le_root, body_radius })
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
}
