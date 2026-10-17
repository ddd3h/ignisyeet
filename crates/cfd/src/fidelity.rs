// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2024-2026 西濱大将 (NISHIHAMA Daisuke)
//! Geometry fidelity of an STL-surface CFD mesh (`stl_fidelity.json`).
//!
//! Three surfaces are compared, all in the CFD frame:
//!
//! * the **input** STL (transformed, with its overlapping shells),
//! * the **union** of the repaired shells (manifold3d, plus the tail fairing when requested),
//! * the **CFD wall**: the triangles of the markers `wall` and `base` read back from the SU2 mesh.
//!
//! Each is described by bounding box, triangle count, surface area (and volume where meaningful). The distance
//! between surfaces is approximated by sampling: every vertex and `N` area-weighted random points
//! (deterministic generator) of one surface are projected on the other with an exact nearest-triangle
//! search (uniform grid, Ericson's closest point on a triangle). Reported are the maximum, the 99th percentile
//! and the root mean square:
//!
//! * `cfd_vs_union`: two-sided (CFD wall -> union and union -> CFD wall), the approximate Hausdorff distance
//!   of the mesh to the surface it was made from;
//! * `cfd_wall_vs_input`: one-sided, `wall` marker -> input STL. The reverse direction is meaningless
//!   because the input contains buried surfaces (fin boxes inside the body); the `base` marker is excluded
//!   because a tail fairing is not part of the input.
//!
//! A maximum above half the wall size is a warning, above `max_geometry_error` an error.

use anyhow::{bail, Context, Result};
use geom::stl::Triangle;
use geom::Vec3;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::BufRead;
use std::path::Path;

/// Bounding box, size and (where defined) volume of one surface.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SurfaceStats {
    pub triangles: usize,
    pub area: f64,
    /// Enclosed volume [m^3] (`None` for the input, whose shells overlap).
    pub volume: Option<f64>,
    pub bbox_min: [f64; 3],
    pub bbox_max: [f64; 3],
}

/// Distance statistics \[m\].
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct Distances {
    pub max: f64,
    /// Where the maximum occurs (a point of the surface it was measured from) \[m\].
    pub max_at: [f64; 3],
    pub p99: f64,
    pub rms: f64,
    pub samples: usize,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Fidelity {
    pub input: SurfaceStats,
    pub union: SurfaceStats,
    pub cfd: SurfaceStats,
    pub cfd_wall_triangles: usize,
    pub cfd_base_triangles: usize,
    pub cfd_vs_union: Distances,
    pub cfd_wall_vs_input: Distances,
    pub wall_size: f64,
    /// Warning level (0.5 wall size) and error level (`max_geometry_error`) of the maximum distances \[m\].
    pub warn_above: f64,
    pub fail_above: f64,
    pub genus: Option<i64>,
    pub warnings: Vec<String>,
    /// STL mirror-symmetry check and the domain chosen from it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub symmetry: Option<crate::symmetry::SymmetryDecision>,
}

impl Fidelity {
    /// The larger maximum of the two comparisons.
    pub fn max_error(&self) -> f64 {
        self.cfd_vs_union.max.max(self.cfd_wall_vs_input.max)
    }

    /// Fails above `fail_above`.
    pub fn check(&self) -> Result<()> {
        if self.max_error() > self.fail_above {
            bail!(
                "[IY-F088] STL geometry error {:.3e} m (CFD wall vs union {:.3e} m, wall vs input STL {:.3e} m) exceeds max_geometry_error {:.3e} m; \
                 use stl_remesh = \"none\", a smaller wall_size, or raise max_geometry_error",
                self.max_error(),
                self.cfd_vs_union.max,
                self.cfd_wall_vs_input.max,
                self.fail_above
            );
        }
        Ok(())
    }

    /// Short text for `cfd-check --mesh`.
    pub fn summary(&self) -> String {
        let f = |s: &SurfaceStats| {
            format!(
                "{} triangles, area {:.5} m^2{}, bbox x {:.4}..{:.4} y {:.4}..{:.4} z {:.4}..{:.4} m",
                s.triangles,
                s.area,
                s.volume.map_or(String::new(), |v| format!(", volume {v:.6} m^3")),
                s.bbox_min[0],
                s.bbox_max[0],
                s.bbox_min[1],
                s.bbox_max[1],
                s.bbox_min[2],
                s.bbox_max[2]
            )
        };
        let d = |x: &Distances| {
            format!(
                "max {:.3e} m at ({:.3}, {:.3}, {:.3}), p99 {:.3e} m, rms {:.3e} m ({} samples)",
                x.max, x.max_at[0], x.max_at[1], x.max_at[2], x.p99, x.rms, x.samples
            )
        };
        let mut s = format!(
            "  input STL   {}\n  union       {}{}\n  CFD wall    {} ({} wall + {} base)\n  CFD vs union (two-sided)  {}\n  CFD wall vs input STL     {}\n  wall_size {:.3e} m: warn above {:.3e} m, fail above {:.3e} m",
            f(&self.input),
            f(&self.union),
            self.genus.map_or(String::new(), |g| format!(", genus {g}")),
            f(&self.cfd),
            self.cfd_wall_triangles,
            self.cfd_base_triangles,
            d(&self.cfd_vs_union),
            d(&self.cfd_wall_vs_input),
            self.wall_size,
            self.warn_above,
            self.fail_above
        );
        if let Some(d) = &self.symmetry {
            s += &format!("\n  {}", d.summary());
        }
        for w in &self.warnings {
            s += &format!("\n  warning: {w}");
        }
        s
    }
}

/// Triangles of the named markers of an ASCII SU2 mesh.
pub fn read_su2_markers(path: &Path, tags: &[&str]) -> Result<HashMap<String, Vec<Triangle>>> {
    let f = std::fs::File::open(path).with_context(|| format!("cannot open {}", path.display()))?;
    let mut lines = std::io::BufReader::new(f).lines();
    let mut nodes: Vec<Vec3> = Vec::new();
    let mut out: HashMap<String, Vec<Triangle>> = HashMap::new();
    let count = |l: &str| l.split('=').nth(1).and_then(|v| v.split_whitespace().next()).and_then(|v| v.parse::<usize>().ok());
    while let Some(line) = lines.next() {
        let line = line?;
        if line.starts_with("NELEM=") {
            for _ in 0..count(&line).context("bad NELEM")? {
                lines.next().transpose()?;
            }
        } else if line.starts_with("NPOIN=") {
            let n = count(&line).context("bad NPOIN")?;
            nodes.reserve(n);
            for _ in 0..n {
                let l = lines.next().context("truncated node list")??;
                let mut it = l.split_whitespace().map(|v| v.parse::<f64>());
                let (x, y, z) = (it.next(), it.next(), it.next());
                match (x, y, z) {
                    (Some(Ok(x)), Some(Ok(y)), Some(Ok(z))) => nodes.push(Vec3::new(x, y, z)),
                    _ => bail!("bad node line: {l}"),
                }
            }
        } else if let Some(tag) = line.strip_prefix("MARKER_TAG=") {
            let tag = tag.trim().to_string();
            let n = count(&lines.next().context("truncated marker")??).context("bad MARKER_ELEMS")?;
            let wanted = tags.contains(&tag.as_str());
            let mut tris = Vec::new();
            for _ in 0..n {
                let l = lines.next().context("truncated marker")??;
                if wanted {
                    // Triangle (5) or quadrilateral (9, split into two triangles; the prism layer faces on y = 0).
                    let v: Vec<usize> = l.split_whitespace().filter_map(|s| s.parse().ok()).collect();
                    let n = match v.first() {
                        Some(9) => 4,
                        _ => 3,
                    };
                    let v = &v[1.min(v.len())..];
                    if v.len() >= n && v.iter().take(n).all(|&i| i < nodes.len()) {
                        tris.push([nodes[v[0]], nodes[v[1]], nodes[v[2]]]);
                        if n == 4 {
                            tris.push([nodes[v[0]], nodes[v[2]], nodes[v[3]]]);
                        }
                    }
                }
            }
            if wanted {
                out.insert(tag, tris);
            }
        }
    }
    Ok(out)
}

fn cross_area(t: &Triangle) -> (Vec3, f64) {
    let n = (t[1] - t[0]).cross(t[2] - t[0]);
    (n, 0.5 * n.norm())
}

fn stats(tris: &[Triangle], volume: Option<f64>) -> SurfaceStats {
    let (mut lo, mut hi) = ([f64::INFINITY; 3], [f64::NEG_INFINITY; 3]);
    for v in tris.iter().flatten() {
        for (i, c) in [v.x, v.y, v.z].into_iter().enumerate() {
            lo[i] = lo[i].min(c);
            hi[i] = hi[i].max(c);
        }
    }
    SurfaceStats { triangles: tris.len(), area: tris.iter().map(|t| cross_area(t).1).sum(), volume, bbox_min: lo, bbox_max: hi }
}

/// Closest point of triangle `t` to `p` (Ericson, Real-Time Collision Detection, 5.1.5); returns the squared distance.
fn dist2_point_triangle(p: Vec3, t: &Triangle) -> f64 {
    let (a, b, c) = (t[0], t[1], t[2]);
    let (ab, ac, ap) = (b - a, c - a, p - a);
    let (d1, d2) = (ab.dot(ap), ac.dot(ap));
    let q = |x: Vec3| (p - x).dot(p - x);
    if d1 <= 0.0 && d2 <= 0.0 {
        return q(a);
    }
    let bp = p - b;
    let (d3, d4) = (ab.dot(bp), ac.dot(bp));
    if d3 >= 0.0 && d4 <= d3 {
        return q(b);
    }
    let vc = d1 * d4 - d3 * d2;
    if vc <= 0.0 && d1 >= 0.0 && d3 <= 0.0 {
        return q(a + ab * (d1 / (d1 - d3)));
    }
    let cp = p - c;
    let (d5, d6) = (ab.dot(cp), ac.dot(cp));
    if d6 >= 0.0 && d5 <= d6 {
        return q(c);
    }
    let vb = d5 * d2 - d1 * d6;
    if vb <= 0.0 && d2 >= 0.0 && d6 <= 0.0 {
        return q(a + ac * (d2 / (d2 - d6)));
    }
    let va = d3 * d6 - d5 * d4;
    if va <= 0.0 && (d4 - d3) >= 0.0 && (d5 - d6) >= 0.0 {
        return q(b + (c - b) * ((d4 - d3) / ((d4 - d3) + (d5 - d6))));
    }
    let denom = 1.0 / (va + vb + vc);
    q(a + ab * (vb * denom) + ac * (vc * denom))
}

/// Uniform grid over the bounding box of a triangle set for nearest-triangle queries.
struct TriGrid<'a> {
    tris: &'a [Triangle],
    lo: Vec3,
    cell: f64,
    dims: [usize; 3],
    cells: Vec<Vec<u32>>,
}

impl<'a> TriGrid<'a> {
    fn new(tris: &'a [Triangle]) -> Self {
        let s = stats(tris, None);
        let lo = Vec3::new(s.bbox_min[0], s.bbox_min[1], s.bbox_min[2]);
        let ext = [s.bbox_max[0] - lo.x, s.bbox_max[1] - lo.y, s.bbox_max[2] - lo.z];
        let mean_edge = tris.iter().map(|t| ((t[1] - t[0]).norm() + (t[2] - t[1]).norm() + (t[0] - t[2]).norm()) / 3.0).sum::<f64>() / tris.len().max(1) as f64;
        let diag = (ext[0] * ext[0] + ext[1] * ext[1] + ext[2] * ext[2]).sqrt();
        let cell = (2.0 * mean_edge).clamp(diag / 128.0, diag / 8.0).max(1e-12);
        let dims = [0, 1, 2].map(|i| ((ext[i] / cell).floor() as usize + 1).min(512));
        let cell = (0..3).map(|i| ext[i] / dims[i] as f64 * 1.0000001).fold(cell, f64::max).max(cell);
        let mut g = Self { tris, lo, cell, dims, cells: vec![Vec::new(); dims[0] * dims[1] * dims[2]] };
        for (i, t) in tris.iter().enumerate() {
            let (mut a, mut b) = ([usize::MAX; 3], [0usize; 3]);
            for v in t {
                let c = g.index(*v);
                for k in 0..3 {
                    a[k] = a[k].min(c[k]);
                    b[k] = b[k].max(c[k]);
                }
            }
            for x in a[0]..=b[0] {
                for y in a[1]..=b[1] {
                    for z in a[2]..=b[2] {
                        let k = g.flat([x, y, z]);
                        g.cells[k].push(i as u32);
                    }
                }
            }
        }
        g
    }

    fn index(&self, p: Vec3) -> [usize; 3] {
        let f = |v: f64, lo: f64, n: usize| (((v - lo) / self.cell).floor().max(0.0) as usize).min(n - 1);
        [f(p.x, self.lo.x, self.dims[0]), f(p.y, self.lo.y, self.dims[1]), f(p.z, self.lo.z, self.dims[2])]
    }

    fn flat(&self, c: [usize; 3]) -> usize {
        (c[0] * self.dims[1] + c[1]) * self.dims[2] + c[2]
    }

    /// Index of the nearest triangle and its distance.
    fn nearest(&self, p: Vec3) -> (usize, f64) {
        let c = self.index(p);
        // Distance from p to the grid box (0 inside): the ring search is exact once the best distance is below
        // the distance to the unvisited rings, which grows by one cell per ring.
        let rmax = self.dims.iter().copied().max().unwrap();
        let (mut best, mut arg) = (f64::INFINITY, 0usize);
        for r in 0..=rmax {
            let ri = r as isize;
            for dx in -ri..=ri {
                for dy in -ri..=ri {
                    for dz in -ri..=ri {
                        if dx.abs().max(dy.abs()).max(dz.abs()) != ri {
                            continue;
                        }
                        let cc = [c[0] as isize + dx, c[1] as isize + dy, c[2] as isize + dz];
                        if (0..3).any(|k| cc[k] < 0 || cc[k] >= self.dims[k] as isize) {
                            continue;
                        }
                        for &ti in &self.cells[self.flat([cc[0] as usize, cc[1] as usize, cc[2] as usize])] {
                            let d = dist2_point_triangle(p, &self.tris[ti as usize]);
                            if d < best {
                                best = d;
                                arg = ti as usize;
                            }
                        }
                    }
                }
            }
            // Points outside the grid box are further from every ring: use the clamped cell, so the bound is
            // conservative (it only stops later when p is far outside).
            let reach = r as f64 * self.cell;
            let outside = {
                let q = |v: f64, lo: f64, n: usize| (lo - v).max(v - (lo + n as f64 * self.cell)).max(0.0);
                q(p.x, self.lo.x, self.dims[0]).max(q(p.y, self.lo.y, self.dims[1])).max(q(p.z, self.lo.z, self.dims[2]))
            };
            if best.is_finite() && best.sqrt() <= reach - outside.min(reach) + 0.0 && best.sqrt() <= reach {
                break;
            }
        }
        (arg, best.sqrt())
    }
}

/// Largest distance of `points` from the surface `tris` (exact nearest-triangle search) and the point where it occurs.
pub fn max_distance_to_surface(points: &[Vec3], tris: &[Triangle]) -> (f64, Vec3) {
    if tris.is_empty() {
        return (f64::INFINITY, Vec3::ZERO);
    }
    let g = TriGrid::new(tris);
    points.iter().map(|&p| (g.nearest(p).1, p)).fold((0.0, Vec3::ZERO), |a, b| if b.0 > a.0 { b } else { a })
}

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> f64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        (self.0.wrapping_mul(0x2545_F491_4F6C_DD1D) >> 11) as f64 / (1u64 << 53) as f64
    }
}

/// Vertices and area-weighted random points of a surface.
fn samples(tris: &[Triangle], n: usize) -> Vec<Vec3> {
    let mut out: Vec<Vec3> = tris.iter().flatten().copied().collect();
    let mut cum = Vec::with_capacity(tris.len());
    let mut acc = 0.0;
    for t in tris {
        acc += cross_area(t).1;
        cum.push(acc);
    }
    let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
    for _ in 0..n {
        let u = rng.next() * acc;
        let i = cum.partition_point(|&c| c < u).min(tris.len() - 1);
        let (r1, r2) = (rng.next().sqrt(), rng.next());
        let t = &tris[i];
        out.push(t[0] * (1.0 - r1) + t[1] * (r1 * (1.0 - r2)) + t[2] * (r1 * r2));
    }
    out
}

fn distances(from: &[Triangle], to: &TriGrid, n: usize) -> Vec<(f64, Vec3)> {
    samples(from, n).into_iter().map(|p| (to.nearest(p).1, p)).collect()
}

fn summarize(mut d: Vec<(f64, Vec3)>) -> Distances {
    if d.is_empty() {
        return Distances::default();
    }
    d.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
    let rms = (d.iter().map(|x| x.0 * x.0).sum::<f64>() / d.len() as f64).sqrt();
    let (max, at) = *d.last().unwrap();
    Distances { max, max_at: [at.x, at.y, at.z], p99: d[((d.len() - 1) as f64 * 0.99) as usize].0, rms, samples: d.len() }
}

/// Every vertex and `n` area-weighted random points (deterministic generator) of a surface, as sampled by [`compute`].
pub fn sample_points(tris: &[Triangle], n: usize) -> Vec<Vec3> {
    if tris.is_empty() {
        return Vec::new();
    }
    samples(tris, n)
}

/// Distance of every point to the surface `tris` (exact nearest-triangle search), paired with the point.
pub fn point_distances(points: &[Vec3], tris: &[Triangle]) -> Vec<(f64, Vec3)> {
    if tris.is_empty() {
        return points.iter().map(|&p| (f64::INFINITY, p)).collect();
    }
    let g = TriGrid::new(tris);
    points.iter().map(|&p| (g.nearest(p).1, p)).collect()
}

/// Maximum (and where), 99th percentile and root mean square of distances.
pub fn distance_stats(d: Vec<(f64, Vec3)>) -> Distances {
    summarize(d)
}

/// Inputs of [`compute`].
pub struct FidelityInput<'a> {
    pub input: &'a [Triangle],
    /// Sum of the volumes of the input shells (they overlap, so it is only informative), or `None`.
    pub union: &'a [Triangle],
    pub union_volume: f64,
    pub genus: Option<i64>,
    pub wall: &'a [Triangle],
    pub base: &'a [Triangle],
    pub wall_size: f64,
    pub max_error: f64,
    /// The wall is clipped at a symmetry plane (half domain): the enclosed volume is not defined.
    pub partial: bool,
}

/// Compares the three surfaces (see the module documentation).
pub fn compute(inp: &FidelityInput) -> Fidelity {
    let n = (4 * inp.wall.len()).clamp(20_000, 200_000);
    let all: Vec<Triangle> = inp.wall.iter().chain(inp.base).copied().collect();
    let union_grid = TriGrid::new(inp.union);
    let all_grid = TriGrid::new(&all);
    let input_grid = TriGrid::new(inp.input);
    let mut two = distances(&all, &union_grid, n);
    two.extend(distances(inp.union, &all_grid, n));
    let wall_in = distances(inp.wall, &input_grid, n);
    // Volume of the CFD wall: orient every triangle like the nearest union triangle, then the divergence theorem.
    let c0 = {
        let s = stats(inp.union, None);
        Vec3::new(0.5 * (s.bbox_min[0] + s.bbox_max[0]), 0.5 * (s.bbox_min[1] + s.bbox_max[1]), 0.5 * (s.bbox_min[2] + s.bbox_max[2]))
    };
    let mut vol = 0.0;
    for t in all.iter().filter(|_| !inp.partial) {
        let (cn, _) = cross_area(t);
        let cen = (t[0] + t[1] + t[2]) / 3.0;
        let (i, _) = union_grid.nearest(cen);
        let sign = if cn.dot(cross_area(&inp.union[i]).0) < 0.0 { -1.0 } else { 1.0 };
        vol += sign * (t[0] - c0).dot((t[1] - c0).cross(t[2] - c0)) / 6.0;
    }
    let mut f = Fidelity {
        input: stats(inp.input, None),
        union: stats(inp.union, (!inp.partial).then_some(inp.union_volume)),
        cfd: stats(&all, (!inp.partial).then_some(vol)),
        cfd_wall_triangles: inp.wall.len(),
        cfd_base_triangles: inp.base.len(),
        cfd_vs_union: summarize(two),
        cfd_wall_vs_input: summarize(wall_in),
        wall_size: inp.wall_size,
        warn_above: 0.5 * inp.wall_size,
        fail_above: inp.max_error,
        genus: inp.genus,
        warnings: Vec::new(),
        symmetry: None,
    };
    if f.max_error() > f.warn_above {
        f.warnings.push(format!("[IY-F513] geometry error {:.3e} m exceeds half the wall size ({:.3e} m)", f.max_error(), f.warn_above));
    }
    f
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_tmp::TempDir;

    /// Closed icosphere-like triangulation of a sphere (radius `r`, centre `c`) by recursive subdivision of an octahedron.
    fn sphere(r: f64, c: Vec3, level: usize) -> Vec<Triangle> {
        let p = |x: f64, y: f64, z: f64| Vec3::new(x, y, z);
        let v = [p(1.0, 0.0, 0.0), p(-1.0, 0.0, 0.0), p(0.0, 1.0, 0.0), p(0.0, -1.0, 0.0), p(0.0, 0.0, 1.0), p(0.0, 0.0, -1.0)];
        let mut t: Vec<Triangle> = vec![
            [v[0], v[2], v[4]],
            [v[2], v[1], v[4]],
            [v[1], v[3], v[4]],
            [v[3], v[0], v[4]],
            [v[2], v[0], v[5]],
            [v[1], v[2], v[5]],
            [v[3], v[1], v[5]],
            [v[0], v[3], v[5]],
        ];
        for _ in 0..level {
            t = t
                .iter()
                .flat_map(|t| {
                    let m = |a: Vec3, b: Vec3| ((a + b) * 0.5).normalized();
                    let (ab, bc, ca) = (m(t[0], t[1]), m(t[1], t[2]), m(t[2], t[0]));
                    [[t[0], ab, ca], [ab, t[1], bc], [ca, bc, t[2]], [ab, bc, ca]]
                })
                .collect();
        }
        t.iter().map(|t| t.map(|v| v * r + c)).collect()
    }

    fn input(wall: &[Triangle], union: &[Triangle], size: f64) -> Fidelity {
        compute(&FidelityInput { input: union, union, union_volume: 0.0, genus: Some(0), wall, base: &[], wall_size: size, max_error: size, partial: false })
    }

    #[test]
    fn identical_surfaces_have_zero_distance() {
        let s = sphere(0.05, Vec3::new(0.1, 0.0, 0.0), 3);
        let f = input(&s, &s, 0.005);
        assert!(f.cfd_vs_union.max < 1e-12 && f.cfd_wall_vs_input.max < 1e-12, "{f:?}");
        assert!(f.warnings.is_empty() && f.check().is_ok());
        let area = 4.0 * std::f64::consts::PI * 0.0025;
        assert!((f.cfd.area / area - 1.0).abs() < 0.03, "{}", f.cfd.area);
        // Divergence-theorem volume of the inscribed polyhedron.
        assert!((f.cfd.volume.unwrap() / (4.0 / 3.0 * std::f64::consts::PI * 0.05f64.powi(3)) - 1.0).abs() < 0.04);
        assert!((f.cfd.bbox_max[0] - 0.15).abs() < 1e-9 && (f.cfd.bbox_min[0] - 0.05).abs() < 1e-9);
        assert_eq!(f.cfd.triangles, s.len());
    }

    #[test]
    fn offset_sphere_has_the_known_distance() {
        // Concentric spheres of radius 0.05 and 0.052 (vertices on both): the distance is the radius difference.
        let (a, b) = (sphere(0.05, Vec3::ZERO, 3), sphere(0.052, Vec3::ZERO, 3));
        let f = input(&b, &a, 0.01);
        assert!((f.cfd_vs_union.max - 0.002).abs() < 1.5e-4, "{}", f.cfd_vs_union.max);
        assert!(f.cfd_vs_union.rms > 0.0015 && f.cfd_vs_union.p99 <= f.cfd_vs_union.max + 1e-15);
        assert!(f.warnings.is_empty() && f.check().is_ok());
        // Translated sphere: the maximum is the shift.
        let c = sphere(0.05, Vec3::new(0.003, 0.0, 0.0), 3);
        assert!((input(&c, &a, 0.01).cfd_vs_union.max - 0.003).abs() < 4e-4);
        // Above half the wall size: warning; above the wall size: error.
        let f = input(&b, &a, 0.003);
        assert_eq!(f.warnings.len(), 1);
        let f = input(&b, &a, 0.0015);
        assert!(f.check().unwrap_err().to_string().contains("max_geometry_error"));
    }

    #[test]
    fn nearest_triangle_matches_brute_force() {
        let s = sphere(0.05, Vec3::ZERO, 2);
        let g = TriGrid::new(&s);
        let mut rng = Rng(12345);
        for _ in 0..300 {
            let p = Vec3::new((rng.next() - 0.5) * 0.4, (rng.next() - 0.5) * 0.4, (rng.next() - 0.5) * 0.4);
            let brute = s.iter().map(|t| dist2_point_triangle(p, t)).fold(f64::INFINITY, f64::min).sqrt();
            assert!((g.nearest(p).1 - brute).abs() < 1e-12, "{p:?}");
        }
    }

    #[test]
    fn nearest_triangle_matches_brute_force_on_the_sample_stl() {
        // Few huge facets next to many tiny ones: the grid search must stay exact.
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/sample_rocket.stl");
        let tris: Vec<Triangle> = geom::stl::read_stl(&path).unwrap().into_iter().map(|t| t.map(|v| v * 0.001)).collect();
        let g = TriGrid::new(&tris);
        let mut rng = Rng(777);
        let mut worst = 0.0f64;
        for _ in 0..400 {
            let t = &tris[(rng.next() * tris.len() as f64) as usize % tris.len()];
            let mirrored = Vec3::new(-(t[0].x + t[1].x + t[2].x) / 3.0, (t[0].y + t[1].y + t[2].y) / 3.0, (t[0].z + t[1].z + t[2].z) / 3.0);
            let p = if rng.next() < 0.5 { mirrored } else { Vec3::new((rng.next() - 0.5) * 0.4, (rng.next() - 0.5) * 0.4, -rng.next() * 1.6) };
            let brute = tris.iter().map(|t| dist2_point_triangle(p, t)).fold(f64::INFINITY, f64::min).sqrt();
            worst = worst.max((g.nearest(p).1 - brute).abs());
        }
        assert!(worst < 1e-12, "{worst}");
    }

    #[test]
    fn reads_markers_of_an_su2_mesh() {
        let text = "NDIME= 3\nNELEM= 1\n10 0 1 2 3 0\nNPOIN= 4\n0 0 0 0\n1 0 0 1\n0 1 0 2\n0 0 1 3\nNMARK= 2\nMARKER_TAG= wall\nMARKER_ELEMS= 1\n5 0 1 2\nMARKER_TAG= farfield\nMARKER_ELEMS= 1\n5 1 2 3\n";
        let p_td = TempDir::new("fid");
        let p = p_td.join("fid.su2");
        std::fs::write(&p, text).unwrap();
        let m = read_su2_markers(&p, &["wall"]).unwrap();
        let _ = std::fs::remove_file(&p);
        assert_eq!(m.len(), 1);
        assert_eq!(m["wall"], vec![[Vec3::new(0.0, 0.0, 0.0), Vec3::new(1.0, 0.0, 0.0), Vec3::new(0.0, 1.0, 0.0)]]);
    }

    #[test]
    fn splits_quadrilateral_marker_faces() {
        // One prism (SU2 type 13) with a triangle and a quadrilateral (type 9) in the marker.
        let text = "NDIME= 3\nNELEM= 1\n13 0 2 1 3 5 4 0\nNPOIN= 6\n0 0 0 0\n1 0 0 1\n0 1 0 2\n0 0 1 3\n1 0 1 4\n0 1 1 5\nNMARK= 1\n\
                    MARKER_TAG= symmetry\nMARKER_ELEMS= 2\n5 0 1 2\n9 0 1 4 3\n";
        let p_td = TempDir::new("fid_quad");
        let p = p_td.join("fid_quad.su2");
        std::fs::write(&p, text).unwrap();
        let m = read_su2_markers(&p, &["symmetry"]).unwrap();
        let _ = std::fs::remove_file(&p);
        let t = &m["symmetry"];
        assert_eq!(t.len(), 3);
        let v = |x: f64, y: f64, z: f64| Vec3::new(x, y, z);
        assert_eq!(t[1], [v(0.0, 0.0, 0.0), v(1.0, 0.0, 0.0), v(1.0, 0.0, 1.0)]);
        assert_eq!(t[2], [v(0.0, 0.0, 0.0), v(1.0, 0.0, 1.0), v(0.0, 0.0, 1.0)]);
        // The two triangles cover the unit square.
        let area: f64 = t[1..].iter().map(|t| 0.5 * (t[1] - t[0]).cross(t[2] - t[0]).norm()).sum();
        assert!((area - 1.0).abs() < 1e-12);
    }
}
