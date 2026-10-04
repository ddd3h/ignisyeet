// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025-2026 西濱大将 (NISHIHAMA Daisuke)
//! Wall surface in the CFD frame, as binary STL.
//!
//! CFD frame: `x` aft from the nose tip (nose at the origin, base at `x = L`), `z` up (pitch
//! plane = x-z), `y` span. The roll is chosen so that **one fin points to +z**; with evenly
//! spaced fins this makes the body mirror-symmetric about the pitch plane `y = 0`, which the half
//! model (`symmetry = true`) relies on. (The aerodynamic table is assumed roll-independent; the CFD
//! sees the fins in the "+" attitude with the flow in the plane of one fin.)
//!
//! The surface is closed with a flat base cap and split into two STL files: `wall.stl` (body, fins)
//! and `base.stl` (the cap), which become the markers `wall` and `base` (see `su2cfg`). The full
//! closed surface is always exported; the mesher intersects it with the half space `y >= 0` when
//! `symmetry` is on.

use crate::config::{CfdOptions, SurfaceKind};
use anyhow::{bail, Context, Result};
use geom::stl::Triangle;
use geom::{Geometry, Vec3};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct WallSurface {
    /// Body and fins.
    pub wall: Vec<Triangle>,
    /// Flat base cap(s), outward normal `+x`.
    pub base: Vec<Triangle>,
}

impl WallSurface {
    pub fn all(&self) -> Vec<Triangle> {
        self.wall.iter().chain(&self.base).copied().collect()
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Watertight {
    pub triangles: usize,
    pub vertices: usize,
    pub edges: usize,
}

type Key = (i64, i64, i64);

fn key(v: Vec3, tol: f64) -> Key {
    ((v.x / tol).round() as i64, (v.y / tol).round() as i64, (v.z / tol).round() as i64)
}

/// Directed-edge census over welded vertices: key pair -> triangle count per undirected edge.
fn edge_counts(tris: &[Triangle], tol: f64) -> (HashMap<(Key, Key), usize>, usize) {
    let mut cnt: HashMap<(Key, Key), usize> = HashMap::new();
    let mut verts = std::collections::HashSet::new();
    for t in tris {
        let k = [key(t[0], tol), key(t[1], tol), key(t[2], tol)];
        verts.extend(k);
        for i in 0..3 {
            let (a, b) = (k[i], k[(i + 1) % 3]);
            if a != b {
                *cnt.entry(if a < b { (a, b) } else { (b, a) }).or_default() += 1;
            }
        }
    }
    (cnt, verts.len())
}

/// Every edge must be shared by exactly two triangles (vertices welded within `tol`).
pub fn check_watertight(tris: &[Triangle], tol: f64) -> Result<Watertight> {
    let (cnt, nv) = edge_counts(tris, tol);
    let open = cnt.values().filter(|&&c| c == 1).count();
    let over = cnt.values().filter(|&&c| c > 2).count();
    if open > 0 || over > 0 {
        bail!(
            "the wall surface is not watertight: {open} edges belong to only one triangle (holes) and {over} edges to more than two \
             (non-manifold); CFD needs a closed surface"
        );
    }
    Ok(Watertight { triangles: tris.len(), vertices: nv, edges: cnt.len() })
}

fn tri_normal(t: &Triangle) -> Vec3 {
    (t[1] - t[0]).cross(t[2] - t[0])
}

/// Splits off flat base-cap triangles (in the plane `x = length`, normal `+x`) and, if the rest
/// has a single open boundary loop in that plane, closes it with a fan cap.
fn split_and_close_base(tris: Vec<Triangle>, length: f64) -> (Vec<Triangle>, Vec<Triangle>) {
    let tol = 1e-6 * length;
    let is_cap = |t: &Triangle| {
        let n = tri_normal(t);
        t.iter().all(|v| (v.x - length).abs() < tol) && n.x > 0.0 && n.x > 0.99 * n.norm()
    };
    let (mut base, wall): (Vec<Triangle>, Vec<Triangle>) = tris.into_iter().partition(|t| is_cap(t));
    // Open directed edges of everything so far.
    let wtol = 1e-8 * length;
    let all: Vec<Triangle> = wall.iter().chain(&base).copied().collect();
    let (cnt, _) = edge_counts(&all, wtol);
    let mut open: Vec<(Vec3, Vec3)> = Vec::new();
    for t in &all {
        for i in 0..3 {
            let (a, b) = (t[i], t[(i + 1) % 3]);
            let (ka, kb) = (key(a, wtol), key(b, wtol));
            if ka == kb {
                continue;
            }
            let ek = if ka < kb { (ka, kb) } else { (kb, ka) };
            if cnt.get(&ek) == Some(&1) {
                open.push((a, b));
            }
        }
    }
    if !open.is_empty() && open.iter().all(|(a, b)| (a.x - length).abs() < 1e-4 * length && (b.x - length).abs() < 1e-4 * length) {
        let mut c = Vec3::ZERO;
        for (a, b) in &open {
            c += (*a + *b) * 0.5;
        }
        let mut c = c / open.len() as f64;
        c.x = length;
        // Centre on the axis: a body of revolution's cap centre is the axis point.
        c.y = 0.0;
        c.z = 0.0;
        for (a, b) in open {
            base.push([b, a, c]);
        }
    }
    (wall, base)
}

/// Rotation of the (y, z) plane by `theta` about +x.
fn roll(v: Vec3, theta: f64) -> Vec3 {
    let (s, c) = theta.sin_cos();
    Vec3::new(v.x, c * v.y - s * v.z, s * v.y + c * v.z)
}

/// Surface from the `panel` crate's watertight mesh generator: no tail fairing, no wake, resolution
/// derived from `wall_size`, rolled so that a fin points to +z, base closed with a flat cap.
pub fn panel_mesh_surface(geom: &Geometry, opt: &CfdOptions) -> Result<WallSurface> {
    finish(panel_mesh_triangles(geom, opt)?, geom.length)
}

/// Open (base not capped) triangles of the rolled panel mesh.
fn panel_mesh_triangles(geom: &Geometry, opt: &CfdOptions) -> Result<Vec<Triangle>> {
    let l = geom.length;
    let n = |x: f64, lo: usize, hi: usize| ((x / opt.wall_size).ceil() as usize).clamp(lo, hi);
    let po = panel::PanelOptions {
        body_axial: n(l, 80, 600),
        body_circ: n(2.0 * std::f64::consts::PI * geom.ref_radius, 48, 160),
        fin_chord: geom.fins.as_ref().map_or(16, |f| n(f.root_chord, 16, 100)),
        fin_span: geom.fins.as_ref().map_or(10, |f| n(f.span, 10, 60)),
        wake_length: 0.0,
        tail_radii: 0.0,
        ..Default::default()
    };
    let mesh = panel::mesh::Mesh::from_geometry(geom, &po).context("panel mesh generation failed")?;
    // Fin k of the mesh sits at azimuth 2 pi k / N + pi / N: rotate fin 0 to +z (90 deg).
    let theta = match &geom.fins {
        Some(f) if f.count > 0 => std::f64::consts::FRAC_PI_2 - std::f64::consts::PI / f.count as f64,
        _ => 0.0,
    };
    let mut tris = Vec::new();
    for p in &mesh.panels {
        let c: Vec<Vec3> = p.corners[..p.n_v].iter().map(|&v| roll(v, theta)).collect();
        for k in 1..p.n_v - 1 {
            tris.push([c[0], c[k], c[k + 1]]);
        }
    }
    Ok(tris)
}

/// Surface from a raw STL (metres) transformed with the extracted `geom.frame`; rolled so that the
/// fin with the farthest vertex points to +z.
pub fn stl_surface(raw: &[Triangle], geom: &Geometry) -> Result<WallSurface> {
    let fr = &geom.frame;
    let ex = Vec3::new(fr.aft_direction[0], fr.aft_direction[1], fr.aft_direction[2]).normalized();
    let origin = Vec3::new(fr.axis_point[0], fr.axis_point[1], fr.axis_point[2]);
    let helper = if ex.x.abs() < 0.9 { Vec3::new(1.0, 0.0, 0.0) } else { Vec3::new(0.0, 1.0, 0.0) };
    let ey = ex.cross(helper).normalized();
    let ez = ex.cross(ey);
    let to_body = |v: Vec3| {
        let d = v - origin;
        Vec3::new(d.dot(ex) - fr.nose_offset, d.dot(ey), d.dot(ez))
    };
    let mut tris: Vec<Triangle> = raw.iter().map(|t| t.map(to_body)).collect();
    if geom.fins.is_some() {
        let far = tris.iter().flatten().max_by(|a, b| (a.y * a.y + a.z * a.z).partial_cmp(&(b.y * b.y + b.z * b.z)).unwrap()).copied();
        if let Some(v) = far {
            if v.y.hypot(v.z) > 1.2 * geom.ref_radius {
                let theta = std::f64::consts::FRAC_PI_2 - v.z.atan2(v.y);
                for t in &mut tris {
                    *t = t.map(|p| roll(p, theta));
                }
            }
        }
    }
    finish(tris, geom.length)
}

fn finish(tris: Vec<Triangle>, length: f64) -> Result<WallSurface> {
    let (wall, base) = split_and_close_base(tris, length);
    let s = WallSurface { wall, base };
    check_watertight(&s.all(), 1e-8 * length)?;
    Ok(s)
}

/// Clips triangles at the plane `y = 0`, keeping `y >= 0`. Vertices within `tol` of the plane are
/// snapped onto it, triangles lying in the plane are dropped, and edge intersections are computed
/// from a canonical vertex order so that neighbouring triangles obtain bit-identical points.
pub fn clip_half(tris: &[Triangle], tol: f64) -> Vec<Triangle> {
    let snap = |mut v: Vec3| {
        if v.y.abs() < tol {
            v.y = 0.0;
        }
        v
    };
    let cut = |a: Vec3, b: Vec3| {
        let (p, q) = if (a.x, a.y, a.z) <= (b.x, b.y, b.z) { (a, b) } else { (b, a) };
        let t = (0.0 - p.y) / (q.y - p.y);
        Vec3::new(p.x + t * (q.x - p.x), 0.0, p.z + t * (q.z - p.z))
    };
    let mut out = Vec::with_capacity(tris.len());
    for t in tris {
        let v = t.map(snap);
        if v.iter().all(|p| p.y >= 0.0) {
            if v.iter().any(|p| p.y > 0.0) {
                out.push(v);
            }
            continue;
        }
        if v.iter().all(|p| p.y <= 0.0) {
            continue;
        }
        // Sutherland-Hodgman against y >= 0.
        let mut poly: Vec<Vec3> = Vec::with_capacity(4);
        for i in 0..3 {
            let (a, b) = (v[i], v[(i + 1) % 3]);
            if a.y >= 0.0 {
                poly.push(a);
            }
            if (a.y > 0.0 && b.y < 0.0) || (a.y < 0.0 && b.y > 0.0) {
                poly.push(cut(a, b));
            }
        }
        for k in 1..poly.len().saturating_sub(1) {
            let tri = [poly[0], poly[k], poly[k + 1]];
            if tri_normal(&tri).norm() > 1e-20 {
                out.push(tri);
            }
        }
    }
    out
}

/// The `y >= 0` half of a closed surface; its only open boundary lies in the plane `y = 0`.
pub fn half_surface(s: &WallSurface, length: f64) -> Result<WallSurface> {
    let tol = 1e-7 * length;
    let h = WallSurface { wall: clip_half(&s.wall, tol), base: clip_half(&s.base, tol) };
    let (cnt, _) = edge_counts(&h.all(), 1e-8 * length);
    let key_on_plane = |k: &(Key, Key)| k.0 .1 == 0 && k.1 .1 == 0;
    let stray = cnt.iter().filter(|(k, &c)| c == 1 && !key_on_plane(k)).count();
    let over = cnt.values().filter(|&&c| c > 2).count();
    if stray > 0 || over > 0 {
        bail!("half model: clipping at y = 0 left {stray} open edges off the symmetry plane and {over} non-manifold edges; use symmetry = false");
    }
    Ok(h)
}

/// Surface according to `opt.surface`; `raw` is the STL (metres) for `SurfaceKind::Stl`.
pub fn build_surface(geom: &Geometry, opt: &CfdOptions, raw: Option<&[Triangle]>) -> Result<WallSurface> {
    match opt.surface {
        SurfaceKind::PanelMesh => panel_mesh_surface(geom, opt),
        SurfaceKind::Stl => stl_surface(raw.context("surface = \"stl\" needs the STL triangles")?, geom),
    }
}

/// Writes `wall.stl` and `base.stl` into `dir`.
pub fn write_surface(dir: &Path, s: &WallSurface) -> Result<(PathBuf, PathBuf)> {
    std::fs::create_dir_all(dir).with_context(|| format!("cannot create {}", dir.display()))?;
    let (w, b) = (dir.join("wall.stl"), dir.join("base.stl"));
    geom::stl::write_stl_binary(&w, &s.wall)?;
    geom::stl::write_stl_binary(&b, &s.base)?;
    Ok((w, b))
}

#[cfg(test)]
mod tests {
    use super::*;
    use geom::sample::SampleRocket;
    use geom::{extract, ExtractOptions, Quat};

    fn geom_of(rk: &SampleRocket) -> Geometry {
        extract(&rk.mesh(), &ExtractOptions::default()).unwrap()
    }

    fn extent(t: &[Triangle]) -> ([f64; 3], [f64; 3]) {
        let (mut lo, mut hi) = ([f64::INFINITY; 3], [f64::NEG_INFINITY; 3]);
        for v in t.iter().flatten() {
            for (i, c) in [v.x, v.y, v.z].into_iter().enumerate() {
                lo[i] = lo[i].min(c);
                hi[i] = hi[i].max(c);
            }
        }
        (lo, hi)
    }

    #[test]
    fn panel_mesh_export_is_watertight_with_cap() {
        let rk = SampleRocket::default();
        let g = geom_of(&rk);
        let s = panel_mesh_surface(&g, &CfdOptions::default()).unwrap();
        let w = check_watertight(&s.all(), 1e-8 * g.length).unwrap();
        assert!(w.triangles > 1000);
        assert!(!s.base.is_empty());
        assert!(s.base.iter().all(|t| t.iter().all(|v| (v.x - g.length).abs() < 1e-9) && tri_normal(t).x > 0.0));
        // Without the cap the open base would be a hole.
        assert!(check_watertight(&s.wall, 1e-8 * g.length).is_err());
        let (lo, hi) = extent(&s.all());
        assert!(lo[0].abs() < 1e-9 && (hi[0] - g.length).abs() < 1e-9);
        // A fin points to +z and the body is mirror symmetric in y.
        assert!((hi[2] - (rk.radius + rk.span)).abs() < 2e-3, "{hi:?}");
        assert!((hi[1] + lo[1]).abs() < 1e-6);
    }

    #[test]
    fn three_fins_get_pitch_plane_symmetry() {
        let rk = SampleRocket { fin_count: 3, ..Default::default() };
        let g = geom_of(&rk);
        let s = panel_mesh_surface(&g, &CfdOptions::default()).unwrap();
        let (lo, hi) = extent(&s.all());
        assert!((hi[2] - (rk.radius + rk.span)).abs() < 2e-3, "{hi:?}");
        assert!((hi[1] + lo[1]).abs() < 1e-3, "y extent {lo:?} {hi:?}");
    }

    #[test]
    fn stl_surface_is_transformed_to_cfd_frame() {
        let rk = SampleRocket::default();
        let q = Quat { w: 0.8, x: 0.3, y: -0.4, z: 0.2 }.normalized().to_mat();
        let off = Vec3::new(1.0, -2.0, 0.5);
        let raw: Vec<Triangle> = rk.mesh().iter().map(|t| t.map(|v| q.mul_vec(v) + off)).collect();
        let g = extract(&raw, &ExtractOptions::default()).unwrap();
        let s = stl_surface(&raw, &g).unwrap();
        let (lo, hi) = extent(&s.all());
        assert!(lo[0].abs() < 0.01 && (hi[0] - rk.length).abs() < 0.01, "x range {} {}", lo[0], hi[0]);
        assert!((hi[2] - (rk.radius + rk.span)).abs() < 5e-3, "{hi:?}");
        assert!(!s.base.is_empty());
        check_watertight(&s.all(), 1e-8 * g.length).unwrap();
    }

    #[test]
    fn open_surface_is_rejected_with_clear_error() {
        let g = geom_of(&SampleRocket::default());
        let mut raw = SampleRocket::default().mesh();
        raw.truncate(raw.len() / 2);
        let e = stl_surface(&raw, &g).unwrap_err().to_string();
        assert!(e.contains("not watertight") && e.contains("holes"), "{e}");
    }

    fn area(t: &[Triangle]) -> f64 {
        t.iter().map(|t| 0.5 * tri_normal(t).norm()).sum()
    }

    #[test]
    fn half_surface_is_clipped_at_the_symmetry_plane() {
        let g = geom_of(&SampleRocket::default());
        let full = panel_mesh_surface(&g, &CfdOptions::default()).unwrap();
        let half = half_surface(&full, g.length).unwrap();
        let (lo, hi) = extent(&half.all());
        assert!(lo[1] >= 0.0 && hi[1] > 0.05);
        // The sample rocket is mirror symmetric: half the surface area (plus nothing from the cut).
        let (af, ah) = (area(&full.all()), area(&half.all()));
        assert!((ah / af - 0.5).abs() < 5e-3, "{}", ah / af);
        // The only open edges lie in y = 0.
        let (cnt, _) = edge_counts(&half.all(), 1e-8 * g.length);
        let open: Vec<_> = cnt.iter().filter(|(_, &c)| c == 1).collect();
        assert!(!open.is_empty() && open.iter().all(|(k, _)| k.0 .1 == 0 && k.1 .1 == 0));
        // Clipping twice changes nothing.
        assert_eq!(clip_half(&half.wall, 1e-7 * g.length).len(), half.wall.len());
    }

    #[test]
    fn clip_splits_crossing_triangles_exactly() {
        let t: Triangle = [Vec3::new(0.0, -1.0, 0.0), Vec3::new(1.0, 1.0, 0.0), Vec3::new(0.0, 1.0, 1.0)];
        let c = clip_half(&[t], 1e-9);
        assert_eq!(c.len(), 2);
        assert!(c.iter().flatten().all(|v| v.y >= 0.0));
        assert!(clip_half(&[[Vec3::new(0.0, -1.0, 0.0), Vec3::new(1.0, -1.0, 0.0), Vec3::new(0.0, -0.5, 1.0)]], 1e-9).is_empty());
    }

    #[test]
    fn writes_two_stl_files() {
        let g = geom_of(&SampleRocket::default());
        let s = panel_mesh_surface(&g, &CfdOptions::default()).unwrap();
        let dir = std::env::temp_dir().join(format!("ignisyeet_cfd_surf_{}", std::process::id()));
        let (w, b) = write_surface(&dir, &s).unwrap();
        assert_eq!(geom::stl::read_stl(&w).unwrap().len(), s.wall.len());
        assert_eq!(geom::stl::read_stl(&b).unwrap().len(), s.base.len());
        let back: Vec<Triangle> = geom::stl::read_stl(&w).unwrap().into_iter().chain(geom::stl::read_stl(&b).unwrap()).collect();
        check_watertight(&back, 1e-8 * g.length).unwrap();
        let _ = std::fs::remove_dir_all(&dir);
    }
}
