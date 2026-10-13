// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025-2026 西濱大将 (NISHIHAMA Daisuke)
//! Wall surface in the CFD frame, as binary STL.
//!
//! CFD frame: `x` aft from the nose tip (nose at the origin, base at `x = L`), `y` and `z` the body
//! axes `y_b`, `-z_b` of `geom::Frame::body_basis` (`R_cb = diag(-1, 1, -1)`, a rotation by 180 deg
//! about y). A raw STL keeps its own roll about the axis ([`to_cfd_frame`]); the geometry rebuilt by the
//! panel mesh generator is rolled so that **one fin points to +z**: with evenly spaced fins this makes
//! it mirror-symmetric about the pitch plane `y = 0`, which the half model (`symmetry = true`) relies on.
//!
//! [`repair`] prepares a raw STL for the union with manifold3d: weld, drop degenerate facets, split the
//! connected components (the sample STL models its fins as closed boxes embedded in the body),
//! orient every component outward by its signed volume and check it is watertight.
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
            "[IY-F080] the wall surface is not watertight: {open} edges belong to only one triangle (holes) and {over} edges to more than two \
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

/// Maps a point of the STL (metres, mesh frame) to the CFD frame: body coordinates of the frame
/// (nose at the origin, x forward) with `x` and `z` reversed, `(-x_b, y_b, -z_b)`.
pub fn to_cfd_frame(raw: &[Triangle], geom: &Geometry) -> Vec<Triangle> {
    raw.iter()
        .map(|t| {
            t.map(|v| {
                let b = geom.frame.to_body(v);
                Vec3::new(-b.x, b.y, -b.z)
            })
        })
        .collect()
}

/// Splits aft-facing base facets from the wall: a facet is `base` when its centroid lies aft of the base
/// plane `x = length` by more than `tol` (a tail fairing), or within `tol` of it and its normal points
/// along `+x` (flat base disc, fin trailing edges flush with the base). Returns `(wall, base)`.
///
/// `mesh_stl.py` applies the same rule to the meshed surface.
pub fn split_base(tris: &[Triangle], length: f64, tol: f64) -> (Vec<Triangle>, Vec<Triangle>) {
    let mut wall = Vec::new();
    let mut base = Vec::new();
    for t in tris {
        let c = (t[0] + t[1] + t[2]) / 3.0;
        let n = tri_normal(t);
        let flat = c.x >= length - tol && n.x > 0.9 * n.norm();
        if c.x > length + tol || flat {
            base.push(*t);
        } else {
            wall.push(*t);
        }
    }
    (wall, base)
}

/// One connected closed shell of the input STL after [`repair`].
#[derive(Debug, Clone, serde::Serialize)]
pub struct ComponentInfo {
    pub triangles: usize,
    /// Enclosed volume after orientation [m^3].
    pub volume: f64,
    /// The input had it inside-out (negative signed volume): every facet was flipped.
    pub flipped: bool,
    /// Facets flipped to make the orientation of the shell consistent.
    pub reoriented_facets: usize,
    pub bbox_min: [f64; 3],
    pub bbox_max: [f64; 3],
}

/// What [`repair`] changed. Repairs are topological only; no vertex is moved except by welding.
#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct RepairReport {
    pub input_triangles: usize,
    pub weld_tolerance: f64,
    pub vertices: usize,
    pub degenerate_dropped: usize,
    pub duplicate_dropped: usize,
    pub components: Vec<ComponentInfo>,
}

/// Closed, outward-oriented components of a raw STL.
#[derive(Debug, Clone)]
pub struct Repaired {
    pub components: Vec<Vec<Triangle>>,
    pub report: RepairReport,
}

fn find(parent: &mut [usize], mut x: usize) -> usize {
    while parent[x] != x {
        parent[x] = parent[parent[x]];
        x = parent[x];
    }
    x
}

fn signed_volume(tris: &[Triangle]) -> f64 {
    let o = tris[0][0];
    tris.iter().map(|t| (t[0] - o).dot((t[1] - o).cross(t[2] - o)) / 6.0).sum()
}

fn bbox(tris: &[Triangle]) -> ([f64; 3], [f64; 3]) {
    let (mut lo, mut hi) = ([f64::INFINITY; 3], [f64::NEG_INFINITY; 3]);
    for v in tris.iter().flatten() {
        for (i, c) in [v.x, v.y, v.z].into_iter().enumerate() {
            lo[i] = lo[i].min(c);
            hi[i] = hi[i].max(c);
        }
    }
    (lo, hi)
}

/// Repairs a raw STL for the union: welds vertices within `tol`, drops degenerate and duplicate facets,
/// splits the connected components, makes each shell consistently oriented, outward by its signed volume
/// (so the all-inside-out shells of the sample STL are flipped) and checks that it is watertight.
pub fn repair(tris: &[Triangle], tol: f64) -> Result<Repaired> {
    let mut report = RepairReport { input_triangles: tris.len(), weld_tolerance: tol, ..Default::default() };
    let mut index: HashMap<Key, usize> = HashMap::new();
    let mut verts: Vec<Vec3> = Vec::new();
    let mut idx: Vec<[usize; 3]> = Vec::with_capacity(tris.len());
    let mut seen: std::collections::HashSet<[usize; 3]> = std::collections::HashSet::new();
    for t in tris {
        let k = t.map(|v| {
            *index.entry(key(v, tol)).or_insert_with(|| {
                verts.push(v);
                verts.len() - 1
            })
        });
        let area = 0.5 * tri_normal(&[verts[k[0]], verts[k[1]], verts[k[2]]]).norm();
        if k[0] == k[1] || k[1] == k[2] || k[0] == k[2] || area < 1e-3 * tol * tol {
            report.degenerate_dropped += 1;
            continue;
        }
        let mut sorted = k;
        sorted.sort_unstable();
        if !seen.insert(sorted) {
            report.duplicate_dropped += 1;
            continue;
        }
        idx.push(k);
    }
    report.vertices = verts.len();
    if idx.is_empty() {
        bail!("[IY-F081] the STL has no usable triangles");
    }
    // Connected components over shared vertices.
    let mut parent: Vec<usize> = (0..verts.len()).collect();
    for k in &idx {
        for (a, b) in [(k[0], k[1]), (k[1], k[2])] {
            let (ra, rb) = (find(&mut parent, a), find(&mut parent, b));
            if ra != rb {
                parent[ra] = rb;
            }
        }
    }
    let mut groups: Vec<(usize, Vec<usize>)> = Vec::new();
    let mut slot: HashMap<usize, usize> = HashMap::new();
    for (t, k) in idx.iter().enumerate() {
        let r = find(&mut parent, k[0]);
        let g = *slot.entry(r).or_insert_with(|| {
            groups.push((r, Vec::new()));
            groups.len() - 1
        });
        groups[g].1.push(t);
    }
    let mut components = Vec::new();
    for (ci, (_, members)) in groups.iter().enumerate() {
        // Consistent orientation by propagation over the shared edges.
        let mut edges: HashMap<(usize, usize), Vec<(usize, bool)>> = HashMap::new();
        for (m, &t) in members.iter().enumerate() {
            let k = idx[t];
            for i in 0..3 {
                let (a, b) = (k[i], k[(i + 1) % 3]);
                edges.entry((a.min(b), a.max(b))).or_default().push((m, a < b));
            }
        }
        if let Some(((a, b), v)) = edges.iter().find(|(_, v)| v.len() > 2) {
            bail!("[IY-F082] STL component {ci} is not a manifold surface: the edge ({:?}, {:?}) belongs to {} facets", verts[*a], verts[*b], v.len());
        }
        let mut flip: Vec<Option<bool>> = vec![None; members.len()];
        let mut reoriented = 0;
        for start in 0..members.len() {
            if flip[start].is_some() {
                continue;
            }
            flip[start] = Some(false);
            let mut stack = vec![start];
            while let Some(m) = stack.pop() {
                let t = idx[members[m]];
                for i in 0..3 {
                    let (a, b) = (t[i], t[(i + 1) % 3]);
                    let fwd_m = (a < b) != flip[m].unwrap();
                    for &(n, fwd_n) in &edges[&(a.min(b), a.max(b))] {
                        if n == m {
                            continue;
                        }
                        // Neighbours must traverse the shared edge in the opposite direction.
                        let want = fwd_n == fwd_m;
                        if flip[n].is_none() {
                            flip[n] = Some(want);
                            if want {
                                reoriented += 1;
                            }
                            stack.push(n);
                        }
                    }
                }
            }
        }
        let mut comp: Vec<Triangle> = members
            .iter()
            .enumerate()
            .map(|(m, &t)| {
                let k = idx[t];
                if flip[m].unwrap() {
                    [verts[k[0]], verts[k[2]], verts[k[1]]]
                } else {
                    [verts[k[0]], verts[k[1]], verts[k[2]]]
                }
            })
            .collect();
        let (lo, hi) = bbox(&comp);
        check_watertight(&comp, tol).map_err(|e| {
            anyhow::anyhow!(
                "STL component {ci} ({} triangles, x {:.4}..{:.4} m, y {:.4}..{:.4} m, z {:.4}..{:.4} m): {e}",
                comp.len(),
                lo[0],
                hi[0],
                lo[1],
                hi[1],
                lo[2],
                hi[2]
            )
        })?;
        let mut vol = signed_volume(&comp);
        let flipped = vol < 0.0;
        if flipped {
            for t in &mut comp {
                t.swap(1, 2);
            }
            vol = -vol;
        }
        if vol < 1e-12 {
            bail!("[IY-F084] STL component {ci} ({} triangles) encloses no volume ({vol:.3e} m^3)", comp.len());
        }
        report.components.push(ComponentInfo { triangles: comp.len(), volume: vol, flipped, reoriented_facets: reoriented, bbox_min: lo, bbox_max: hi });
        components.push(comp);
    }
    Ok(Repaired { components, report })
}

/// The outer boundary loop of the flat aft-facing base cap of a component (facets with centroid within `tol` of
/// the base plane `x = length` and normal along `+x`), ordered along the facets' winding. `None` when the
/// component has no such cap or its boundary is not a single closed loop.
pub fn base_ring(tris: &[Triangle], length: f64, tol: f64) -> Option<Vec<Vec3>> {
    let cap: Vec<&Triangle> = tris
        .iter()
        .filter(|t| {
            let c = (t[0] + t[1] + t[2]) / 3.0;
            let n = tri_normal(t);
            c.x >= length - tol && c.x <= length + tol && n.x > 0.9 * n.norm()
        })
        .collect();
    if cap.is_empty() {
        return None;
    }
    let k = |v: Vec3| key(v, 1e-9 * length.max(1.0));
    let mut next: HashMap<Key, (Vec3, Key)> = HashMap::new();
    let mut count: HashMap<(Key, Key), i32> = HashMap::new();
    for t in &cap {
        for i in 0..3 {
            let (a, b) = (t[i], t[(i + 1) % 3]);
            *count.entry((k(a), k(b))).or_default() += 1;
            *count.entry((k(b), k(a))).or_default() -= 1;
        }
    }
    // Boundary edges: directed edges without an opposite partner.
    for t in &cap {
        for i in 0..3 {
            let (a, b) = (t[i], t[(i + 1) % 3]);
            if count[&(k(a), k(b))] > 0 && next.insert(k(a), (a, k(b))).is_some() {
                return None; // a vertex with two outgoing boundary edges: not a simple loop
            }
        }
    }
    let &start = next.keys().next()?;
    let mut ring = Vec::new();
    let mut cur = start;
    loop {
        let (p, nxt) = *next.get(&cur)?;
        ring.push(p);
        cur = nxt;
        if cur == start {
            break;
        }
        if ring.len() > next.len() {
            return None;
        }
    }
    (ring.len() == next.len() && ring.len() >= 8).then_some(ring)
}

/// Triangles between two rings of the same winding, ordered by angle `t` (increasing from 0 to 2 pi): both
/// pointers advance in angle order, so rings with a different number of vertices are connected without slivers.
fn band(a: &[Vec3], ta: &[f64], b: &[Vec3], tb: &[f64], out: &mut Vec<Triangle>) {
    let tau = 2.0 * std::f64::consts::PI;
    let (na, nb) = (a.len(), b.len());
    let (mut i, mut k) = (0usize, 0usize);
    while i < na || k < nb {
        let nta = if i < na {
            if i + 1 < na {
                ta[i + 1]
            } else {
                tau
            }
        } else {
            f64::INFINITY
        };
        let ntb = if k < nb {
            if k + 1 < nb {
                tb[k + 1]
            } else {
                tau
            }
        } else {
            f64::INFINITY
        };
        if i < na && (nta <= ntb || k >= nb) {
            out.push([a[i], a[(i + 1) % na], b[k % nb]]);
            i += 1;
        } else {
            out.push([a[i % na], b[(k + 1) % nb], b[k]]);
            k += 1;
        }
    }
}

/// A ring of vertices with the azimuth `atan2(z, y)` of each and the index of its mirror image about `y = 0`
/// (`my`, azimuth `pi - a`) and `z = 0` (`mz`, azimuth `-a`).
struct SymRing {
    pts: Vec<Vec3>,
    q: Vec<f64>,
    my: Vec<usize>,
    mz: Vec<usize>,
}

impl SymRing {
    fn new(pts: &[Vec3], tol: f64) -> Option<Self> {
        let find = |p: Vec3| pts.iter().position(|v| (*v - p).norm() <= tol);
        let my = pts.iter().map(|v| find(Vec3::new(v.x, -v.y, v.z))).collect::<Option<Vec<_>>>()?;
        let mz = pts.iter().map(|v| find(Vec3::new(v.x, v.y, -v.z))).collect::<Option<Vec<_>>>()?;
        Some(Self { pts: pts.to_vec(), q: pts.iter().map(|v| v.z.atan2(v.y)).collect(), my, mz })
    }

    /// Vertices of the first quadrant (azimuth 0 to pi/2, both ends present) in ascending azimuth.
    fn quadrant(&self) -> Option<Vec<usize>> {
        let half_pi = std::f64::consts::FRAC_PI_2;
        let mut v: Vec<usize> = (0..self.pts.len()).filter(|&i| self.q[i] >= -1e-9 && self.q[i] <= half_pi + 1e-9).collect();
        v.sort_by(|&a, &b| self.q[a].partial_cmp(&self.q[b]).unwrap());
        (v.len() >= 2 && self.q[v[0]].abs() < 1e-9 && (self.q[v[v.len() - 1]] - half_pi).abs() < 1e-9).then_some(v)
    }
}

/// Band between two symmetric rings built in the first quadrant (merge of the ascending azimuths) and reflected
/// into the other three, so that the triangulation is exactly mirror symmetric. `flip`: the winding of the rings is
/// against increasing azimuth.
fn symmetric_band(a: &SymRing, b: &SymRing, flip: bool, out: &mut Vec<Triangle>) -> Option<()> {
    let (la, lb) = (a.quadrant()?, b.quadrant()?);
    let mut tris: Vec<[(bool, usize); 3]> = Vec::new();
    let (mut i, mut k) = (0, 0);
    while i + 1 < la.len() || k + 1 < lb.len() {
        let adv_a = if k + 1 >= lb.len() {
            true
        } else if i + 1 >= la.len() {
            false
        } else {
            a.q[la[i + 1]] <= b.q[lb[k + 1]]
        };
        if adv_a {
            tris.push([(false, la[i]), (false, la[i + 1]), (true, lb[k])]);
            i += 1;
        } else {
            tris.push([(false, la[i]), (true, lb[k + 1]), (true, lb[k])]);
            k += 1;
        }
    }
    for map in 0..4u8 {
        for tr in &tris {
            let pt = |&(is_b, idx): &(bool, usize)| {
                let r = if is_b { b } else { a };
                let mut j = idx;
                if map & 1 != 0 {
                    j = r.my[j];
                }
                if map & 2 != 0 {
                    j = r.mz[j];
                }
                r.pts[j]
            };
            let mut t = [pt(&tr[0]), pt(&tr[1]), pt(&tr[2])];
            // A single reflection reverses the orientation.
            if (map.count_ones() == 1) != flip {
                t.swap(1, 2);
            }
            out.push(t);
        }
    }
    Some(())
}

/// The cone surface of [`fairing_cone`] behind a base ring that is symmetric about both planes: rings of a multiple
/// of four vertices at the azimuths `2 pi k / n`, bands built per quadrant ([`symmetric_band`]) and the apex fan.
/// `None` when the base ring is not symmetric (the caller falls back to the plain construction).
fn symmetric_tail(ring0: &[Vec3], flip: bool, x0: f64, r0: f64, tail_length: f64, f_end: f64, m: usize) -> Option<Vec<Triangle>> {
    let tau = 2.0 * std::f64::consts::PI;
    let mut prev = SymRing::new(ring0, 1e-4 * r0)?;
    let n0 = ring0.len();
    let mut out = Vec::new();
    for j in 1..=m {
        let f = f_end * j as f64 / m as f64;
        let nj = (((n0 as f64 * (1.0 - f) / 4.0).round() as usize) * 4).max(8);
        let pts: Vec<Vec3> = (0..nj)
            .map(|k| {
                let q = tau * k as f64 / nj as f64;
                Vec3::new(x0 + f * tail_length, r0 * (1.0 - f) * q.cos(), r0 * (1.0 - f) * q.sin())
            })
            .collect();
        let cur = SymRing::new(&pts, 1e-9 * r0)?;
        symmetric_band(&prev, &cur, flip, &mut out)?;
        prev = cur;
    }
    let apex = Vec3::new(x0 + f_end * tail_length, 0.0, 0.0);
    let n = prev.pts.len();
    for k in 0..n {
        let mut tri = [prev.pts[k], prev.pts[(k + 1) % n], apex];
        if flip {
            tri.swap(1, 2);
        }
        out.push(tri);
    }
    Some(out)
}

/// Closed conical tail fairing behind the base plane `x = length`, `tail_length` long, to be united with the body.
///
/// With the base `ring` of the body ([`base_ring`]) the cone starts at exactly that polygon (its front disc is
/// coplanar with the body's base disc and has the same boundary, so the union has no step and no sliver at the
/// junction). Without a ring it is a cone of 98 % of the base radius with a short cylinder continuing into the
/// body. The radius tapers linearly towards the apex and the cone is cut at 10 % of the base radius, closed by a
/// flat cap (a sharp apex makes the surface parametrisation of gmsh fail); the number of vertices per ring tapers
/// with the radius, so the circumferential edges stay at the size of the base ring and the axial step is twice
/// that (no needle facets towards the cap). `wall_size` sets the facet size of the fallback. Outward oriented.
pub fn fairing_cone(length: f64, base_radius: f64, tail_length: f64, wall_size: f64, ring: Option<&[Vec3]>) -> Vec<Triangle> {
    let tau = 2.0 * std::f64::consts::PI;
    let mut t: Vec<Triangle> = Vec::new();
    let (ring0, ang0): (Vec<Vec3>, Vec<f64>) = match ring {
        Some(r) => {
            // Unwrapped angles about the axis, in the direction of the winding, starting at 0.
            let mut a: Vec<f64> = r.iter().map(|v| v.z.atan2(v.y)).collect();
            for i in 1..a.len() {
                while a[i] - a[i - 1] > tau / 2.0 {
                    a[i] -= tau;
                }
                while a[i] - a[i - 1] < -tau / 2.0 {
                    a[i] += tau;
                }
            }
            let dir = if a[a.len() - 1] >= a[0] { 1.0 } else { -1.0 };
            let a0 = a[0];
            (r.to_vec(), a.iter().map(|v| dir * (v - a0)).collect())
        }
        None => {
            let rc = 0.98 * base_radius;
            let n = ((tau * rc / (0.8 * wall_size)).ceil() as usize).clamp(24, 256).div_ceil(4) * 4;
            let inside = (0.02 * length).max(2.0 * wall_size).min(0.1 * length);
            let poly = |x: f64| -> Vec<Vec3> {
                (0..n).map(|k| Vec3::new(x, rc * (tau * k as f64 / n as f64).cos(), rc * (tau * k as f64 / n as f64).sin())).collect()
            };
            let tt: Vec<f64> = (0..n).map(|k| tau * k as f64 / n as f64).collect();
            // Short cylinder into the body (front ring -> base-plane ring).
            let front = poly(length - inside);
            let base = poly(length);
            band(&front, &tt, &base, &tt, &mut t);
            let c = Vec3::new(length - inside, 0.0, 0.0);
            for k in 0..n {
                t.push([c, front[(k + 1) % n], front[k]]);
            }
            (base, tt)
        }
    };
    let n0 = ring0.len();
    let x0 = ring0.iter().map(|v| v.x).sum::<f64>() / n0 as f64;
    let r0 = ring0.iter().map(|v| v.y.hypot(v.z)).sum::<f64>() / n0 as f64;
    let (dir, az0) = match ring {
        Some(r) => {
            let first = r[0].z.atan2(r[0].y);
            let second = r[1 % r.len()].z.atan2(r[1 % r.len()].y);
            let mut d = second - first;
            while d > tau / 2.0 {
                d -= tau;
            }
            while d < -tau / 2.0 {
                d += tau;
            }
            (if d >= 0.0 { 1.0 } else { -1.0 }, first)
        }
        None => (1.0, 0.0),
    };
    let edge = ring0.iter().enumerate().map(|(i, v)| (ring0[(i + 1) % n0] - *v).norm()).sum::<f64>() / n0 as f64;
    if ring.is_none() {
        // Front disc of the short cylinder is closed above; the base-plane ring continues below.
    } else {
        // Front disc coplanar with the body's base disc.
        let c = Vec3::new(x0, 0.0, 0.0);
        for k in 0..n0 {
            t.push([c, ring0[(k + 1) % n0], ring0[k]]);
        }
    }
    // The cone is cut at 10 % of the base radius (flat cap of 1 % of the base area): a sharp apex makes the
    // surface parametrisation of gmsh (`stl_remesh = "classify"`) fail.
    let f_end = 0.9;
    let m = ((f_end * tail_length / (2.0 * edge.max(1e-9))).ceil() as usize).clamp(4, 400);
    // A base ring that is mirror symmetric about y = 0 and z = 0 (with vertices on the planes) gets a fairing that
    // is exactly symmetric too: the union of a symmetric body with it stays symmetric (see `symmetry`).
    if let Some(tail) = symmetric_tail(&ring0, dir < 0.0, x0, r0, tail_length, f_end, m) {
        t.extend(tail);
        if signed_volume(&t) < 0.0 {
            for tri in &mut t {
                tri.swap(1, 2);
            }
        }
        return t;
    }
    let mut prev = (ring0.clone(), ang0.clone());
    for j in 1..=m {
        let f = f_end * j as f64 / m as f64;
        let nj = (((n0 as f64 * (1.0 - f) / 4.0).round() as usize) * 4).max(8);
        // Vertices in the order of the angle measured from the first vertex of the base ring, in its winding direction.
        let mut ring: Vec<(f64, Vec3)> = (0..nj)
            .map(|k| {
                let q = tau * k as f64 / nj as f64;
                let rel = (dir * (q - az0)).rem_euclid(tau);
                (rel, Vec3::new(x0 + f * tail_length, r0 * (1.0 - f) * q.cos(), r0 * (1.0 - f) * q.sin()))
            })
            .collect();
        ring.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        let (tj, pts): (Vec<f64>, Vec<Vec3>) = ring.into_iter().unzip();
        band(&prev.0, &prev.1, &pts, &tj, &mut t);
        prev = (pts, tj);
    }
    let apex = Vec3::new(x0 + f_end * tail_length, 0.0, 0.0);
    let last = &prev.0;
    for k in 0..last.len() {
        t.push([last[k], last[(k + 1) % last.len()], apex]);
    }
    if signed_volume(&t) < 0.0 {
        for tri in &mut t {
            tri.swap(1, 2);
        }
    }
    t
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

/// Surface from a raw STL (metres) transformed to the CFD frame ([`to_cfd_frame`]), closed with the
/// base cap; the roll of the STL is kept.
pub fn stl_surface(raw: &[Triangle], geom: &Geometry) -> Result<WallSurface> {
    finish(to_cfd_frame(raw, geom), geom.length)
}

fn finish(tris: Vec<Triangle>, length: f64) -> Result<WallSurface> {
    let (wall, base) = split_and_close_base(tris, length);
    let s = WallSurface { wall, base };
    check_watertight(&s.all(), 1e-8 * length)?;
    Ok(s)
}

fn comp(v: Vec3, axis: usize) -> f64 {
    [v.x, v.y, v.z][axis]
}

fn with_comp(mut v: Vec3, axis: usize, c: f64) -> Vec3 {
    match axis {
        0 => v.x = c,
        1 => v.y = c,
        _ => v.z = c,
    }
    v
}

/// Clips triangles at the plane `y = 0`, keeping `y >= 0` (see [`clip_plane`]).
pub fn clip_half(tris: &[Triangle], tol: f64) -> Vec<Triangle> {
    clip_plane(tris, 1, tol)
}

/// Clips triangles at the plane `x[axis] = 0` (`axis` 1: y, 2: z), keeping `x[axis] >= 0`. Vertices within `tol`
/// of the plane are snapped onto it, triangles lying in the plane are dropped, and edge intersections are computed
/// from a canonical vertex order so that neighbouring triangles obtain bit-identical points.
pub fn clip_plane(tris: &[Triangle], axis: usize, tol: f64) -> Vec<Triangle> {
    let snap = |v: Vec3| if comp(v, axis).abs() < tol { with_comp(v, axis, 0.0) } else { v };
    let cut = |a: Vec3, b: Vec3| {
        let (p, q) = if (a.x, a.y, a.z) <= (b.x, b.y, b.z) { (a, b) } else { (b, a) };
        let t = (0.0 - comp(p, axis)) / (comp(q, axis) - comp(p, axis));
        with_comp(p + (q - p) * t, axis, 0.0)
    };
    let mut out = Vec::with_capacity(tris.len());
    for t in tris {
        let v = t.map(snap);
        if v.iter().all(|p| comp(*p, axis) >= 0.0) {
            if v.iter().any(|p| comp(*p, axis) > 0.0) {
                out.push(v);
            }
            continue;
        }
        if v.iter().all(|p| comp(*p, axis) <= 0.0) {
            continue;
        }
        // Sutherland-Hodgman against x[axis] >= 0.
        let mut poly: Vec<Vec3> = Vec::with_capacity(4);
        for i in 0..3 {
            let (a, b) = (v[i], v[(i + 1) % 3]);
            let (ca, cb) = (comp(a, axis), comp(b, axis));
            if ca >= 0.0 {
                poly.push(a);
            }
            if (ca > 0.0 && cb < 0.0) || (ca < 0.0 && cb > 0.0) {
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

/// Edge census of a surface clipped at the planes `y = 0` (`clip_y`) and/or `z = 0` (`clip_z`): the open edges
/// must lie in those planes only, and no edge may belong to more than two triangles.
pub fn check_clipped(tris: &[Triangle], clip_y: bool, clip_z: bool, length: f64) -> Result<Watertight> {
    let (cnt, nv) = edge_counts(tris, 1e-8 * length);
    let on_plane = |k: &(Key, Key), a: usize| {
        let g = |p: &Key| [p.0, p.1, p.2][a];
        g(&k.0) == 0 && g(&k.1) == 0
    };
    let stray = cnt.iter().filter(|(k, &c)| c == 1 && !((clip_y && on_plane(k, 1)) || (clip_z && on_plane(k, 2)))).count();
    let over = cnt.values().filter(|&&c| c > 2).count();
    if stray > 0 || over > 0 {
        bail!("[IY-F083] clipping at the symmetry plane left {stray} open edges off the plane and {over} non-manifold edges");
    }
    Ok(Watertight { triangles: tris.len(), vertices: nv, edges: cnt.len() })
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
        bail!("[IY-F086] half model: clipping at y = 0 left {stray} open edges off the symmetry plane and {over} non-manifold edges; use symmetry = false");
    }
    Ok(h)
}

/// Surface according to `opt.surface`; `raw` is the STL (metres) for `SurfaceKind::Stl`. The CAD B-rep of
/// `SurfaceKind::Step` has no triangle surface of its own here (the mesher tessellates it, see [`crate::step_mesh`]).
pub fn build_surface(geom: &Geometry, opt: &CfdOptions, raw: Option<&[Triangle]>) -> Result<WallSurface> {
    match opt.surface {
        SurfaceKind::PanelMesh => panel_mesh_surface(geom, opt),
        SurfaceKind::Stl => stl_surface(raw.context("[IY-F005] surface = \"stl\" needs the STL triangles")?, geom),
        SurfaceKind::Step => bail!("[IY-F087] surface = \"step\": the wall is the CAD B-rep, which only the mesher tessellates (see step_mesh)"),
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
    use crate::test_tmp::TempDir;
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
        // The fins keep their own roll: the extent in y, z is the fin tip circle (a fin lies in a plane through the axis).
        let rmax = s.all().iter().flatten().map(|v| v.y.hypot(v.z)).fold(0.0, f64::max);
        assert!((rmax - (rk.radius + rk.span)).abs() < 5e-3, "{rmax}");
        assert!(!s.base.is_empty());
        check_watertight(&s.all(), 1e-8 * g.length).unwrap();
        // The frame is a pure rotation of the body frame: R_cb = diag(-1, 1, -1).
        let t = &raw[5];
        let c = to_cfd_frame(&[*t], &g)[0];
        for i in 0..3 {
            let b = g.frame.to_body(t[i]);
            assert!((c[i] - Vec3::new(-b.x, b.y, -b.z)).norm() < 1e-12);
        }
        // Distances are preserved (rigid transformation).
        assert!((((c[0] - c[1]).norm()) - (t[0] - t[1]).norm()).abs() < 1e-12);
    }

    #[test]
    fn open_surface_is_rejected_with_clear_error() {
        let g = geom_of(&SampleRocket::default());
        let mut raw = SampleRocket::default().mesh();
        raw.truncate(raw.len() / 2);
        let e = stl_surface(&raw, &g).unwrap_err().to_string();
        assert!(e.contains("not watertight") && e.contains("holes") && e.matches("IY-F080").count() == 1, "{e}");
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
    fn quarter_clip_has_open_edges_only_in_the_two_planes() {
        let g = geom_of(&SampleRocket::default());
        let full = panel_mesh_surface(&g, &CfdOptions::default()).unwrap().all();
        let tol = 1e-7 * g.length;
        let half = clip_plane(&full, 1, tol);
        let quarter = clip_plane(&half, 2, tol);
        check_clipped(&half, true, false, g.length).unwrap();
        check_clipped(&quarter, true, true, g.length).unwrap();
        // Closed surface: no open edge at all; the half has some, which a check that allows no plane rejects.
        check_watertight(&full, 1e-8 * g.length).unwrap();
        assert!(check_clipped(&half, false, false, g.length).is_err() && check_clipped(&quarter, true, false, g.length).is_err());
        assert!(quarter.iter().flatten().all(|v| v.y >= 0.0 && v.z >= 0.0));
        let (af, ah, aq) = (area(&full), area(&half), area(&quarter));
        assert!((ah / af - 0.5).abs() < 5e-3 && (aq / af - 0.25).abs() < 5e-3, "{} {}", ah / af, aq / af);
        // Every open edge of the quarter lies in y = 0 or z = 0, and both planes have some.
        let (cnt, _) = edge_counts(&quarter, 1e-8 * g.length);
        let open: Vec<_> = cnt.iter().filter(|(_, &c)| c == 1).collect();
        assert!(open.iter().any(|(k, _)| k.0 .1 == 0 && k.1 .1 == 0) && open.iter().any(|(k, _)| k.0 .2 == 0 && k.1 .2 == 0));
        assert!(open.iter().all(|(k, _)| (k.0 .1 == 0 && k.1 .1 == 0) || (k.0 .2 == 0 && k.1 .2 == 0)));
        assert!(cnt.values().all(|&c| c <= 2));
        // The z clip alone is the mirror image of the y clip about y = z.
        let zonly = clip_plane(&full, 2, tol);
        check_clipped(&zonly, false, true, g.length).unwrap();
        assert_eq!(clip_plane(&quarter, 2, tol).len(), quarter.len());
    }

    #[test]
    fn fairing_cone_behind_a_symmetric_ring_is_exactly_mirror_symmetric() {
        let (raw, g) = sample_stl();
        let r = repair(&to_cfd_frame(&raw, &g), 1e-6 * g.length).unwrap();
        let ring = r.components.iter().filter_map(|c| base_ring(c, g.length, 1e-4 * g.length)).max_by_key(|r| r.len()).unwrap();
        let cone = fairing_cone(g.length, g.base_radius, 0.3, 0.005, Some(&ring));
        check_watertight(&cone, 1e-9).unwrap();
        assert!(signed_volume(&cone) > 0.0);
        for axis in [1, 2] {
            let c = crate::symmetry::check_plane(&cone, axis, 1e-6 * g.length);
            assert!(c.symmetric && c.triangulation_identical && c.max_mismatch < 1e-9, "{c:?}");
        }
        // Without a ring (cylinder fallback) it is symmetric too.
        let plain = fairing_cone(1.5, 0.05, 0.3, 0.005, None);
        assert!(crate::symmetry::check_plane(&plain, 1, 1e-6).symmetric && crate::symmetry::check_plane(&plain, 2, 1e-6).symmetric);
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
        let dir = TempDir::new("cfd_surf");
        let (w, b) = write_surface(&dir, &s).unwrap();
        assert_eq!(geom::stl::read_stl(&w).unwrap().len(), s.wall.len());
        assert_eq!(geom::stl::read_stl(&b).unwrap().len(), s.base.len());
        let back: Vec<Triangle> = geom::stl::read_stl(&w).unwrap().into_iter().chain(geom::stl::read_stl(&b).unwrap()).collect();
        check_watertight(&back, 1e-8 * g.length).unwrap();
    }

    fn sample_stl() -> (Vec<Triangle>, Geometry) {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/sample_rocket.stl");
        let raw: Vec<Triangle> = geom::stl::read_stl(&path).unwrap().into_iter().map(|t| t.map(|v| v * 0.001)).collect();
        let g = extract(&raw, &ExtractOptions::default()).unwrap();
        (raw, g)
    }

    #[test]
    fn repair_splits_and_orients_the_sample_stl() {
        let (raw, g) = sample_stl();
        let r = repair(&to_cfd_frame(&raw, &g), 1e-6 * g.length).unwrap();
        assert_eq!(r.components.len(), 5, "{:?}", r.report);
        assert_eq!(r.report.degenerate_dropped, 0);
        let mut sizes: Vec<usize> = r.components.iter().map(Vec::len).collect();
        sizes.sort_unstable();
        assert_eq!(sizes[4], 14400);
        for (c, info) in r.components.iter().zip(&r.report.components) {
            // Outward: positive volume, every shell watertight (and consistently oriented).
            assert!(signed_volume(c) > 0.0 && (signed_volume(c) - info.volume).abs() < 1e-12);
            check_watertight(c, 1e-6 * g.length).unwrap();
            let mut dir = std::collections::HashSet::new();
            let k = |v: Vec3| key(v, 1e-6 * g.length);
            for t in c {
                for i in 0..3 {
                    assert!(dir.insert((k(t[i]), k(t[(i + 1) % 3]))), "inconsistent orientation");
                }
            }
        }
        // The sample writer's axis map is a reflection: all five components are inside-out.
        assert!(r.report.components.iter().all(|c| c.flipped), "{:?}", r.report.components);
        // The body is the big component, 0.6 m^3... check its volume against the geometry (about 10.9 L).
        let vol: f64 = r.report.components.iter().map(|c| c.volume).sum();
        assert!((vol / g.volume - 1.0).abs() < 0.1, "{vol} vs {}", g.volume);
    }

    #[test]
    fn repair_drops_degenerates_flips_facets_and_reports_holes() {
        let (raw, g) = sample_stl();
        let mut cfd = to_cfd_frame(&raw, &g);
        let tol = 1e-6 * g.length;
        let n0 = cfd.len();
        // A zero-area facet, an exact duplicate and one facet with the wrong winding.
        cfd.push([cfd[0][0], cfd[0][0], cfd[1][1]]);
        cfd.push(cfd[3]);
        cfd[7].swap(1, 2);
        let r = repair(&cfd, tol).unwrap();
        assert_eq!((r.report.degenerate_dropped, r.report.duplicate_dropped), (1, 1));
        assert_eq!(r.components.iter().map(Vec::len).sum::<usize>(), n0);
        assert!(r.report.components.iter().map(|c| c.reoriented_facets).sum::<usize>() >= 1);
        // A hole is an error that names the component.
        let mut holed = to_cfd_frame(&raw, &g);
        holed.remove(10);
        let e = repair(&holed, tol).unwrap_err().to_string();
        assert!(e.contains("STL component") && e.contains("not watertight"), "{e}");
    }

    #[test]
    fn base_facets_are_split_by_centroid_and_normal() {
        let (raw, g) = sample_stl();
        let r = repair(&to_cfd_frame(&raw, &g), 1e-6 * g.length).unwrap();
        let all: Vec<Triangle> = r.components.iter().flatten().copied().collect();
        let (wall, base) = split_base(&all, g.length, 1e-4 * g.length);
        assert!(!base.is_empty() && wall.len() + base.len() == all.len());
        assert!(base.iter().all(|t| t.iter().all(|v| (v.x - g.length).abs() < 1e-3)), "flat base only");
        // The base disc of the body plus the trailing edges of the fins that end at the base.
        let area: f64 = base.iter().map(|t| 0.5 * tri_normal(t).norm()).sum();
        assert!(area > 0.9 * std::f64::consts::PI * g.base_radius.powi(2), "{area}");
        // With a fairing the cone is base too.
        let cone = fairing_cone(g.length, g.base_radius, 6.0 * g.base_radius, 0.005, None);
        let (w2, b2) = split_base(&cone, g.length, 1e-4 * g.length);
        assert!(w2.len() < cone.len() / 10 + 100 && b2.len() > cone.len() / 2, "{} {}", w2.len(), b2.len());
    }

    #[test]
    fn fairing_cone_is_closed_and_outward() {
        let c = fairing_cone(1.5, 0.05, 0.3, 0.005, None);
        check_watertight(&c, 1e-9).unwrap();
        assert!(signed_volume(&c) > 0.0);
        let (lo, hi) = extent(&c);
        assert!((hi[0] - 1.77).abs() < 1e-12 && lo[0] < 1.5);
        assert!(hi[1] < 0.05 && hi[1] > 0.048);
        let r = repair(&c, 1e-7).unwrap();
        assert_eq!((r.components.len(), r.report.components[0].flipped), (1, false));
    }

    #[test]
    fn fairing_cone_follows_the_base_ring_of_the_body() {
        let (raw, g) = sample_stl();
        let r = repair(&to_cfd_frame(&raw, &g), 1e-6 * g.length).unwrap();
        let rings: Vec<Vec<Vec3>> = r.components.iter().filter_map(|c| base_ring(c, g.length, 1e-4 * g.length)).collect();
        // The body has a base disc; the fin boxes end flush with it but their caps are rectangles of other loops.
        let body = rings.iter().max_by_key(|r| r.len()).expect("the body has a base ring");
        assert!(body.len() >= 60, "{}", body.len());
        for v in body {
            assert!((v.y.hypot(v.z) - g.base_radius).abs() < 2e-3 && (v.x - g.length).abs() < 1e-3, "{v:?}");
        }
        let cone = fairing_cone(g.length, g.base_radius, 0.3, 0.005, Some(body));
        check_watertight(&cone, 1e-9).unwrap();
        assert!(signed_volume(&cone) > 0.0);
        // Same radius as the body at the junction (no step) and shared ring vertices.
        let (lo, hi) = extent(&cone);
        assert!((hi[1] - g.base_radius).abs() < 2e-3 && (hi[0] - (g.length + 0.27)).abs() < 2e-3 && lo[0] >= g.length - 1e-3, "{lo:?} {hi:?}");
        assert!(cone.iter().flatten().any(|v| *v == body[0]));
    }
}
