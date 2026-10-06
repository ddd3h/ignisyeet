// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025-2026 西濱大将 (NISHIHAMA Daisuke)
//! Mirror symmetry of an STL surface about the body-axis planes `y = 0` and `z = 0` (CFD frame), and the
//! choice of the mesh domain that follows from it.
//!
//! # The check
//!
//! The united surface is tested per plane. The vertices are welded (tolerance `1e-6 L`), every welded vertex is
//! mirrored and must coincide with a vertex of the surface within the tolerance. Then the triangulation is
//! compared: when the mirrored triangles, as sorted vertex triples, are exactly the original set, the surface is
//! symmetric to the tolerance. A mesh generator rarely triangulates two mirror-image quadrilaterals alike
//! (the diagonals are mirrored), so when the sets differ the check falls back to the surface itself: every
//! vertex, edge midpoint and centroid of the mirrored triangles must lie on the original surface within the
//! tolerance (an exact nearest-triangle search). A flat quadrilateral split along either diagonal passes; a
//! missing or displaced feature (a fin that is not mirrored, a shifted fin) fails. Boolean unions
//! additionally move the intersection vertices of fin and body, so the vertex coincidence of the first stage
//! is reported but not required.
//!
//! The result per plane is a [`PlaneCheck`]: symmetric or not, the largest mismatch distance and the reason.
//!
//! # The decision
//!
//! [`decide`] maps the user's `shape_symmetry` and the two checks to the symmetry in effect (see
//! [`SymmetryDecision`]): `auto` uses exactly the planes that pass (both: `axisymmetric`, only `y = 0`:
//! `pitch_plane`, otherwise `none`), an explicit `axisymmetric` / `pitch_plane` that fails the check is an
//! error (an asymmetric body is never mirrored silently) and `none` forces the full domain.

use crate::config::{CfdOptions, ShapeSymmetry};
use crate::fidelity;
use anyhow::{bail, Result};
use geom::stl::Triangle;
use geom::Vec3;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Welding and matching tolerance in units of the body length.
pub const SYMMETRY_TOL: f64 = 1e-6;

/// Result of the mirror check of one plane.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlaneCheck {
    /// `y` (the pitch plane `y = 0`) or `z` (`z = 0`).
    pub plane: String,
    pub symmetric: bool,
    /// Largest distance of a mirrored point from the surface (vertices, edge midpoints and centroids) [m].
    pub max_mismatch: f64,
    /// Welded vertices, and how many of them have a vertex at their mirror image.
    pub vertices: usize,
    pub vertices_matched: usize,
    /// The mirrored triangle set equals the original one as sorted vertex triples.
    pub triangulation_identical: bool,
    pub tolerance: f64,
    /// Why the plane failed.
    pub reason: Option<String>,
}

fn mirror(v: Vec3, axis: usize) -> Vec3 {
    match axis {
        1 => Vec3::new(v.x, -v.y, v.z),
        _ => Vec3::new(v.x, v.y, -v.z),
    }
}

/// Union-find over welded vertices; returns the representative index of every vertex and the list of
/// representatives. Vertices are swept in `x` order, so only the ones within `tol` in `x` are compared.
fn weld(v: &[Vec3], tol: f64) -> (Vec<usize>, Vec<usize>) {
    let mut order: Vec<usize> = (0..v.len()).collect();
    order.sort_by(|&a, &b| v[a].x.partial_cmp(&v[b].x).unwrap());
    let mut parent: Vec<usize> = (0..v.len()).collect();
    fn find(p: &mut [usize], mut x: usize) -> usize {
        while p[x] != x {
            p[x] = p[p[x]];
            x = p[x];
        }
        x
    }
    for (k, &i) in order.iter().enumerate() {
        for &j in order[..k].iter().rev() {
            if v[i].x - v[j].x > tol {
                break;
            }
            if (v[i] - v[j]).norm() <= tol {
                let (a, b) = (find(&mut parent, i), find(&mut parent, j));
                if a != b {
                    parent[a.max(b)] = a.min(b);
                }
            }
        }
    }
    let rep: Vec<usize> = (0..v.len()).map(|i| find(&mut parent, i)).collect();
    let mut reps: Vec<usize> = (0..v.len()).filter(|&i| rep[i] == i).collect();
    reps.sort_by(|&a, &b| v[a].x.partial_cmp(&v[b].x).unwrap());
    (rep, reps)
}

/// Checks the mirror symmetry of the closed surface `tris` about the plane `y = 0` (`axis` 1) or `z = 0`
/// (`axis` 2) with the welding and matching tolerance `tol` (see the module documentation).
pub fn check_plane(tris: &[Triangle], axis: usize, tol: f64) -> PlaneCheck {
    let name = if axis == 1 { "y" } else { "z" };
    let fail = |reason: String, max_mismatch: f64, vertices: usize, matched: usize| PlaneCheck {
        plane: name.into(),
        symmetric: false,
        max_mismatch,
        vertices,
        vertices_matched: matched,
        triangulation_identical: false,
        tolerance: tol,
        reason: Some(reason),
    };
    if tris.is_empty() {
        return fail("the surface has no triangles".into(), f64::INFINITY, 0, 0);
    }
    let all: Vec<Vec3> = tris.iter().flatten().copied().collect();
    let (rep, reps) = weld(&all, tol);
    let xs: Vec<f64> = reps.iter().map(|&i| all[i].x).collect();
    // Nearest welded vertex to `p`: distance and representative.
    let nearest = |p: Vec3| -> (f64, usize) {
        let (mut best, mut arg) = (f64::INFINITY, usize::MAX);
        let mut scan = |range: &mut dyn Iterator<Item = usize>| {
            for k in range {
                let dx = (xs[k] - p.x).abs();
                if dx >= best {
                    break;
                }
                let d = (all[reps[k]] - p).norm();
                if d < best {
                    (best, arg) = (d, reps[k]);
                }
            }
        };
        // Outward from the insertion point of `p.x`.
        let mid = xs.partition_point(|&x| x < p.x);
        scan(&mut (mid..xs.len()));
        scan(&mut (0..mid).rev());
        (best, arg)
    };
    let mut matched = 0usize;
    let mut vertex_max = 0.0f64;
    let mut image = std::collections::HashMap::new();
    for &i in &reps {
        let (d, j) = nearest(mirror(all[i], axis));
        vertex_max = vertex_max.max(d);
        if d <= tol {
            matched += 1;
            image.insert(i, j);
        }
    }
    let n_rep = reps.len();
    // Stage 1: identical triangulation.
    let key = |t: [usize; 3]| {
        let mut k = t;
        k.sort_unstable();
        k
    };
    let orig: HashSet<[usize; 3]> = (0..tris.len()).map(|t| key([rep[3 * t], rep[3 * t + 1], rep[3 * t + 2]])).collect();
    let identical = matched == n_rep && {
        let mir: HashSet<[usize; 3]> = (0..tris.len()).map(|t| key([image[&rep[3 * t]], image[&rep[3 * t + 1]], image[&rep[3 * t + 2]]])).collect();
        mir == orig
    };
    if identical {
        return PlaneCheck {
            plane: name.into(),
            symmetric: true,
            max_mismatch: vertex_max,
            vertices: n_rep,
            vertices_matched: matched,
            triangulation_identical: true,
            tolerance: tol,
            reason: None,
        };
    }
    // Stage 2: the mirrored triangles must lie on the surface.
    let mut pts = Vec::with_capacity(tris.len() * 7);
    for t in tris {
        let m = t.map(|v| mirror(v, axis));
        pts.extend_from_slice(&m);
        pts.extend([(m[0] + m[1]) * 0.5, (m[1] + m[2]) * 0.5, (m[2] + m[0]) * 0.5, (m[0] + m[1] + m[2]) / 3.0]);
    }
    let (dmax, at) = fidelity::max_distance_to_surface(&pts, tris);
    if dmax <= tol {
        return PlaneCheck {
            plane: name.into(),
            symmetric: true,
            max_mismatch: dmax,
            vertices: n_rep,
            vertices_matched: matched,
            triangulation_identical: false,
            tolerance: tol,
            reason: None,
        };
    }
    let reason = format!(
        "the mirror image of the surface about {name} = 0 is not the surface: a mirrored point lies {dmax:.3e} m (tolerance {tol:.1e} m) from it, at ({:.4}, {:.4}, {:.4}); \
         {} of {n_rep} vertices have no vertex at their mirror image (largest gap {vertex_max:.3e} m)",
        at.x,
        at.y,
        at.z,
        n_rep - matched
    );
    fail(reason, dmax, n_rep, matched)
}

/// Both planes of a surface.
pub fn check_planes(tris: &[Triangle], length: f64) -> (PlaneCheck, PlaneCheck) {
    let tol = SYMMETRY_TOL * length;
    (check_plane(tris, 1, tol), check_plane(tris, 2, tol))
}

/// The symmetry in effect for an STL surface and why (stored in `cfd_report.json` and `stl_fidelity.json`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SymmetryDecision {
    /// `shape_symmetry` as configured (`auto`, `axisymmetric`, `pitch_plane`, `none`).
    pub requested: String,
    /// `axisymmetric` (y = 0 and z = 0), `pitch_plane` (y = 0) or `none`.
    pub chosen: String,
    pub y: PlaneCheck,
    pub z: PlaneCheck,
    /// How the volume mesh is made: `quarter` (y >= 0, z >= 0, mirrored about z = 0 afterwards), `half`
    /// (y >= 0) or `full`.
    pub meshed: String,
    pub reason: String,
}

impl SymmetryDecision {
    /// Text for the log and `cfd-check --mesh`.
    pub fn summary(&self) -> String {
        let p = |c: &PlaneCheck| {
            format!(
                "{} = 0: {} (max mismatch {:.2e} m, {})",
                c.plane,
                if c.symmetric { "symmetric" } else { "not symmetric" },
                c.max_mismatch,
                if c.symmetric {
                    if c.triangulation_identical {
                        "identical triangulation"
                    } else {
                        "mirror-equivalent surface"
                    }
                } else {
                    c.reason.as_deref().unwrap_or("")
                }
            )
        };
        format!("shape symmetry {} (requested {}), {} mesh: {}\n    {}\n    {}", self.chosen, self.requested, self.meshed, self.reason, p(&self.y), p(&self.z))
    }
}

/// Maps the requested `shape_symmetry` and the plane checks to the symmetry in effect; an explicit symmetry that
/// fails its check is an error. `opt.z_mirror_mesh` decides whether an `axisymmetric` surface is meshed as a
/// quarter (exactly z-symmetric half mesh); the planes themselves are always used for the domain.
pub fn decide(opt: &CfdOptions, y: &PlaneCheck, z: &PlaneCheck) -> Result<SymmetryDecision> {
    let requested = match opt.shape_symmetry {
        None | Some(ShapeSymmetry::Auto) => "auto",
        Some(ShapeSymmetry::Axisymmetric) => "axisymmetric",
        Some(ShapeSymmetry::PitchPlane) => "pitch_plane",
        Some(ShapeSymmetry::None) => "none",
    };
    let numbers = |c: &PlaneCheck| {
        format!("{} = 0: max mismatch {:.3e} m (tolerance {:.1e} m): {}", c.plane, c.max_mismatch, c.tolerance, c.reason.as_deref().unwrap_or("symmetric"))
    };
    let (chosen, reason) = match requested {
        "none" => ("none", "shape_symmetry = none: full domain".to_string()),
        "axisymmetric" => {
            if !(y.symmetric && z.symmetric) {
                bail!(
                    "shape_symmetry = \"axisymmetric\" needs a surface that is mirror symmetric about y = 0 and z = 0, but the united STL is not:\n  {}\n  {}\n  \
                     use shape_symmetry = \"auto\" (the planes that pass the check are used) or \"none\" (full domain)",
                    numbers(y),
                    numbers(z)
                );
            }
            ("axisymmetric", "shape_symmetry = axisymmetric, both planes verified".to_string())
        }
        "pitch_plane" => {
            if !y.symmetric {
                bail!(
                    "shape_symmetry = \"pitch_plane\" needs a surface that is mirror symmetric about y = 0, but the united STL is not:\n  {}\n  \
                     use shape_symmetry = \"auto\" or \"none\" (full domain)",
                    numbers(y)
                );
            }
            ("pitch_plane", "shape_symmetry = pitch_plane, y = 0 verified".to_string())
        }
        _ => match (y.symmetric, z.symmetric) {
            (true, true) => ("axisymmetric", "auto: symmetric about y = 0 and z = 0".to_string()),
            (true, false) => ("pitch_plane", format!("auto: symmetric about y = 0 only (z = 0: {})", z.reason.as_deref().unwrap_or(""))),
            (false, true) => ("none", format!("auto: symmetric about z = 0 only, which no shape symmetry uses; y = 0: {}", y.reason.as_deref().unwrap_or(""))),
            (false, false) => {
                ("none", format!("auto: not symmetric (y = 0: {}; z = 0: {})", y.reason.as_deref().unwrap_or(""), z.reason.as_deref().unwrap_or("")))
            }
        },
    };
    let meshed = match chosen {
        "axisymmetric" if opt.z_mirror_mesh => "quarter",
        "axisymmetric" | "pitch_plane" => "half",
        _ => "full",
    };
    Ok(SymmetryDecision { requested: requested.into(), chosen: chosen.into(), y: y.clone(), z: z.clone(), meshed: meshed.into(), reason })
}

/// The options with the decision applied: the shape symmetry, the exactly z-symmetric (quarter) mesh and the SU2
/// half domain (kept while the case matrix permits: `betas_deg = [0]`, no roll cases, a symmetry that includes
/// y = 0, `symmetry = true`).
pub fn apply(opt: &CfdOptions, d: &SymmetryDecision) -> CfdOptions {
    let mut o = opt.clone();
    let shape = match d.chosen.as_str() {
        "axisymmetric" => ShapeSymmetry::Axisymmetric,
        "pitch_plane" => ShapeSymmetry::PitchPlane,
        _ => ShapeSymmetry::None,
    };
    let sideslip = o.betas_deg.iter().any(|&b| b != 0.0);
    o.shape_symmetry = Some(shape);
    o.z_mirror_mesh = d.meshed == "quarter";
    o.symmetry = o.symmetry && shape != ShapeSymmetry::None && !sideslip && !o.roll_damping;
    o
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Closed box [x0,x1] x [y0,y1] x [z0,z1], outward, each face split along `diag` (0 or 1).
    fn cuboid(lo: [f64; 3], hi: [f64; 3], diag: usize) -> Vec<Triangle> {
        let p = |i: usize, j: usize, k: usize| Vec3::new([lo[0], hi[0]][i], [lo[1], hi[1]][j], [lo[2], hi[2]][k]);
        let quad = |a: Vec3, b: Vec3, c: Vec3, d: Vec3| if diag == 0 { vec![[a, b, c], [a, c, d]] } else { vec![[a, b, d], [b, c, d]] };
        let mut t = Vec::new();
        t.extend(quad(p(0, 0, 0), p(0, 1, 0), p(0, 1, 1), p(0, 0, 1)));
        t.extend(quad(p(1, 0, 0), p(1, 0, 1), p(1, 1, 1), p(1, 1, 0)));
        t.extend(quad(p(0, 0, 0), p(0, 0, 1), p(1, 0, 1), p(1, 0, 0)));
        t.extend(quad(p(0, 1, 0), p(1, 1, 0), p(1, 1, 1), p(0, 1, 1)));
        t.extend(quad(p(0, 0, 0), p(1, 0, 0), p(1, 1, 0), p(0, 1, 0)));
        t.extend(quad(p(0, 0, 1), p(0, 1, 1), p(1, 1, 1), p(1, 0, 1)));
        t
    }

    #[test]
    fn symmetric_box_passes_both_planes_even_with_mirrored_diagonals() {
        let b = cuboid([0.0, -0.1, -0.2], [1.0, 0.1, 0.2], 0);
        let (y, z) = check_planes(&b, 1.0);
        assert!(y.symmetric && z.symmetric, "{y:?} {z:?}");
        assert_eq!(y.vertices, 8);
        assert!(y.max_mismatch < 1e-12 && y.reason.is_none());
        // Flat faces split along the other diagonal: the triangle set changes, the surface does not.
        let mut mixed = cuboid([0.0, -0.1, -0.2], [1.0, 0.1, 0.2], 0);
        let other = cuboid([0.0, -0.1, -0.2], [1.0, 0.1, 0.2], 1);
        mixed.splice(0..6, other[0..6].iter().copied());
        let (y2, z2) = check_planes(&mixed, 1.0);
        assert!(y2.symmetric && z2.symmetric && !y2.triangulation_identical, "{y2:?}");
    }

    #[test]
    fn offset_or_extra_features_fail_with_numbers() {
        // Extended to y = 0.12: not symmetric about y = 0, still symmetric about z = 0.
        let b = cuboid([0.0, -0.1, -0.2], [1.0, 0.12, 0.2], 0);
        let (y, z) = check_planes(&b, 1.0);
        assert!(!y.symmetric && z.symmetric, "{y:?} {z:?}");
        assert!((y.max_mismatch - 0.02).abs() < 1e-9 || y.max_mismatch > 0.01, "{}", y.max_mismatch);
        assert!(y.reason.as_ref().unwrap().contains("mirror image"));
        // Symmetric body plus one fin at +y only: asymmetric about y, symmetric about z.
        let mut b = cuboid([0.0, -0.1, -0.1], [1.0, 0.1, 0.1], 0);
        b.extend(cuboid([0.4, 0.1, -0.01], [0.6, 0.3, 0.01], 0));
        let (y, z) = check_planes(&b, 1.0);
        assert!(!y.symmetric && z.symmetric, "{y:?} {z:?}");
        // Sub-tolerance noise passes, beyond it fails.
        let jitter = |e: f64| {
            let mut b = cuboid([0.0, -0.1, -0.2], [1.0, 0.1, 0.2], 0);
            b[3][1].y += e;
            b
        };
        assert!(!check_planes(&jitter(1e-4), 1.0).0.symmetric);
    }

    fn sample() -> (Vec<Triangle>, geom::Geometry) {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/sample_rocket.stl");
        let raw: Vec<Triangle> = geom::stl::read_stl(&path).unwrap().into_iter().map(|t| t.map(|v| v * 0.001)).collect();
        let g = geom::extract(&raw, &geom::ExtractOptions::default()).unwrap();
        (crate::surface::to_cfd_frame(&raw, &g), g)
    }

    #[test]
    fn sample_stl_is_symmetric_about_both_planes_and_a_missing_or_shifted_fin_is_not() {
        let (cfd, g) = sample();
        let (y, z) = check_planes(&cfd, g.length);
        assert!(y.symmetric && z.symmetric, "{y:?} {z:?}");
        // The vertex sets are mirror images; the triangulation of the facets is not (diagonals).
        assert!(y.vertices_matched == y.vertices && !y.triangulation_identical, "{y:?}");
        assert!(y.max_mismatch < 1e-7 * g.length * 100.0 && z.max_mismatch < 1e-6, "{y:?}");
        // Remove the outer facets of the two fins on the +y side: the mirror image about y = 0 is gone (y fails, the
        // surface of the fin shell 2 mm (half the thickness) behind the removed tip is the nearest), the two
        // remaining sides are still mirror images about z = 0.
        let fin_plus_y = |t: &Triangle| t.iter().all(|v| v.y > 0.06 && v.x > 0.5);
        let without: Vec<Triangle> = cfd.iter().filter(|t| !fin_plus_y(t)).copied().collect();
        assert!(without.len() < cfd.len());
        let (y2, z2) = check_planes(&without, g.length);
        assert!(!y2.symmetric && z2.symmetric, "{y2:?} {z2:?}");
        assert!(y2.max_mismatch > 1e-3 && y2.reason.as_deref().unwrap().contains("mirror image"), "{y2:?}");
        // The same facets shifted by 1 mm along x: the mirror image is about 1 mm off the surface.
        let shifted: Vec<Triangle> = cfd.iter().map(|t| if fin_plus_y(t) { t.map(|v| geom::Vec3::new(v.x + 0.001, v.y, v.z)) } else { *t }).collect();
        let (y3, _) = check_planes(&shifted, g.length);
        assert!(!y3.symmetric && y3.max_mismatch > 1e-4 && y3.max_mismatch < 5e-3, "{y3:?}");
    }

    #[test]
    fn explicit_symmetry_on_an_asymmetric_sample_is_an_error() {
        let (cfd, g) = sample();
        let without: Vec<Triangle> = cfd.iter().filter(|t| !t.iter().all(|v| v.y > 0.06 && v.x > 0.5)).copied().collect();
        let (y, z) = check_planes(&without, g.length);
        let e = decide(&opts(Some(ShapeSymmetry::Axisymmetric)), &y, &z).unwrap_err().to_string();
        assert!(e.contains("y = 0") && e.contains("max mismatch") && e.contains("shape_symmetry"), "{e}");
        // auto uses the plane that passes: z only maps to none (no shape symmetry uses z alone) and says why.
        let d = decide(&opts(None), &y, &z).unwrap();
        assert_eq!((d.chosen.as_str(), d.meshed.as_str()), ("none", "full"));
        assert!(d.reason.contains("z = 0 only"), "{}", d.reason);
    }

    fn opts(s: Option<ShapeSymmetry>) -> CfdOptions {
        CfdOptions { shape_symmetry: s, surface: crate::config::SurfaceKind::Stl, ..Default::default() }
    }

    fn plane(name: &str, ok: bool) -> PlaneCheck {
        PlaneCheck {
            plane: name.into(),
            symmetric: ok,
            max_mismatch: if ok { 1e-9 } else { 0.004 },
            vertices: 10,
            vertices_matched: if ok { 10 } else { 6 },
            triangulation_identical: ok,
            tolerance: 1e-6,
            reason: (!ok).then(|| "fin missing".to_string()),
        }
    }

    #[test]
    fn auto_uses_exactly_the_planes_that_pass() {
        let d = |y, z| decide(&opts(None), &plane("y", y), &plane("z", z)).unwrap();
        assert_eq!((d(true, true).chosen.as_str(), d(true, true).meshed.as_str()), ("axisymmetric", "quarter"));
        assert_eq!((d(true, false).chosen.as_str(), d(true, false).meshed.as_str()), ("pitch_plane", "half"));
        assert_eq!(d(false, false).chosen, "none");
        assert_eq!(d(false, true).chosen, "none");
        assert_eq!(d(false, false).meshed, "full");
        assert!(d(true, false).reason.contains("fin missing"));
    }

    #[test]
    fn explicit_symmetry_that_fails_is_an_error_with_the_numbers() {
        let e = decide(&opts(Some(ShapeSymmetry::Axisymmetric)), &plane("y", true), &plane("z", false)).unwrap_err().to_string();
        assert!(e.contains("axisymmetric") && e.contains("z = 0") && e.contains("4.000e-3") && e.contains("fin missing"), "{e}");
        let e = decide(&opts(Some(ShapeSymmetry::PitchPlane)), &plane("y", false), &plane("z", true)).unwrap_err().to_string();
        assert!(e.contains("pitch_plane") && e.contains("y = 0"), "{e}");
        // Passing explicit choices are kept; none always gives the full domain.
        let d = decide(&opts(Some(ShapeSymmetry::PitchPlane)), &plane("y", true), &plane("z", true)).unwrap();
        assert_eq!((d.chosen.as_str(), d.meshed.as_str()), ("pitch_plane", "half"));
        let d = decide(&opts(Some(ShapeSymmetry::None)), &plane("y", true), &plane("z", true)).unwrap();
        assert_eq!((d.chosen.as_str(), d.meshed.as_str()), ("none", "full"));
    }

    #[test]
    fn applying_a_decision_sets_shape_domain_and_mirror_mesh() {
        let d = decide(&opts(None), &plane("y", true), &plane("z", true)).unwrap();
        let o = apply(&opts(None), &d);
        assert_eq!((o.shape(), o.symmetry, o.z_mirror_mesh), (ShapeSymmetry::Axisymmetric, true, true));
        // Sideslip or roll cases need the full domain, the quarter mesh stays.
        let o = apply(&CfdOptions { roll_damping: true, ..opts(None) }, &d);
        assert_eq!((o.symmetry, o.z_mirror_mesh), (false, true));
        let o = apply(
            &CfdOptions { z_mirror_mesh: false, ..opts(None) },
            &decide(&CfdOptions { z_mirror_mesh: false, ..opts(None) }, &plane("y", true), &plane("z", true)).unwrap(),
        );
        assert_eq!((o.shape(), o.symmetry, o.z_mirror_mesh), (ShapeSymmetry::Axisymmetric, true, false));
        let d = decide(&opts(None), &plane("y", false), &plane("z", false)).unwrap();
        let o = apply(&opts(None), &d);
        assert_eq!((o.shape(), o.symmetry, o.z_mirror_mesh), (ShapeSymmetry::None, false, false));
        // The `symmetry = false` switch forces the full domain.
        let d = decide(&opts(None), &plane("y", true), &plane("z", true)).unwrap();
        assert!(!apply(&CfdOptions { symmetry: false, ..opts(None) }, &d).symmetry);
    }
}
