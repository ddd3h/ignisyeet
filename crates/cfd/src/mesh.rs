// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025-2026 西濱大将 (NISHIHAMA Daisuke)
//! Volume meshing with gmsh (python API), driven by an embedded script.
//!
//! # Method
//!
//! * **Geometry** (`surface = "panel_mesh"`): the extracted geometry (body profile and fin set) is
//!   rebuilt as an OpenCASCADE solid inside gmsh: the profile (simplified to 0.05 mm) is revolved,
//!   the fins (biconvex section, constant thickness, taper and sweep exactly as in the `panel`
//!   mesh) are lofted and **fused** with the body by a boolean union. The result is a proper union
//!   with exact curves, which gmsh remeshes with its own surface mesher (size `wall_size`, finer on
//!   curved edges). A fin points to `+z` (see [`crate::surface`]).
//! * Why not the panel triangulation: its triangles are extremely small at the nose tip and where
//!   the pointed fin meets the body, and gmsh's discrete-surface tetrahedralisation fails on them
//!   ("vertex lies in a segment"). The CAD route is robust and also provides curvature-based refinement.
//! * Farfield: a sphere of radius `farfield L` centred at mid-body (`farfield` marker).
//! * Half model (`symmetry`): the ball is cut at `y = 0` with an OCC boolean (`symmetry` marker is the
//!   resulting plane face, with the body section as a hole). The body-of-revolution seam is placed away
//!   from `y = 0`.
//! * Volume: tetrahedra (HXT) with a size field `min(farfield L / 10, wall_size + 0.2 d)` of the
//!   distance `d` to the wall, i.e. growth about 1.2 per element row.
//! * RANS: no prismatic boundary layer (gmsh 4.15 has no 3D boundary-layer extrusion for OpenCASCADE
//!   surfaces); the tetrahedral wall mesh is used with a no-slip wall (SU2's wall functions diverged on it),
//!   so RANS skin friction is under-resolved, see [`wall_yplus`].
//! * The SU2 mesh is written with the physical groups `wall`, `base`, `farfield`, `symmetry`.
//!
//! The mesh is cached below `<workdir>/mesh/` keyed by a hash of the geometry and the settings.

use crate::case::hash_parts;
use crate::config::{CfdOptions, FlowModel};
use crate::tools;
use aero::atmosphere::Atmosphere;
use geom::Geometry;
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub const SCRIPT: &str = include_str!("mesh_gmsh.py");

/// Flat-plate estimate of the wall distance of the first cell centre that gives `yplus`.
///
/// Turbulent flat plate `Cf = 0.0576 Re_L^(-1/5)` at the body length and the freestream of `atm` at
/// `mach`: `u_tau = U sqrt(Cf / 2)`, `y = yplus nu / u_tau`.
pub fn first_height(yplus: f64, length: f64, atm: &Atmosphere, mach: f64) -> f64 {
    let u = mach * atm.sound_speed;
    let re = u * length / atm.kinematic_viscosity();
    let cf = 0.0576 * re.powf(-0.2);
    yplus * atm.kinematic_viscosity() / (u * (0.5 * cf).sqrt())
}

/// Estimated y+ of the first cell centre of the tetrahedral wall mesh: the centroid of a wall
/// tetrahedron of edge `wall_size` is about `0.2 wall_size` above the wall. Evaluated at the largest
/// solved Mach number (flat-plate friction, see [`first_height`]).
pub fn wall_yplus(opt: &CfdOptions, length: f64, atm: &Atmosphere) -> f64 {
    let m = opt.machs.iter().copied().fold(0.0, f64::max);
    0.2 * opt.wall_size / first_height(1.0, length, atm, m)
}

/// Fin set in the form the script needs (all lengths in metres, body frame).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FinParams {
    pub count: usize,
    pub x_le: f64,
    pub x_te: f64,
    pub tip_chord: f64,
    pub sweep: f64,
    pub span: f64,
    pub thickness: f64,
    pub body_radius: f64,
    /// Smallest body radius below the fin root (the fin is extended inside the body from half of it).
    pub inner_radius: f64,
}

/// Everything the script needs (written as `params.json`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeshParams {
    pub out_su2: String,
    pub stats_json: String,
    pub length: f64,
    pub center_x: f64,
    pub farfield_radius: f64,
    pub symmetry: bool,
    pub wall_size: f64,
    pub max_size: f64,
    pub growth: f64,
    pub curvature_elements: usize,
    pub min_size_divisor: f64,
    /// Length of the conical tail fairing behind the base [m]; 0 = flat base.
    pub tail_length: f64,
    pub threads: usize,
    /// Body profile `(x, r)` from the nose tip to the base.
    pub profile: Vec<[f64; 2]>,
    pub fins: Option<FinParams>,
}

/// Douglas-Peucker simplification of the profile with tolerance `tol` (end points kept).
pub fn simplify(pts: &[[f64; 2]], tol: f64) -> Vec<[f64; 2]> {
    if pts.len() < 3 {
        return pts.to_vec();
    }
    let (a, b) = (pts[0], pts[pts.len() - 1]);
    let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
    let len = dx.hypot(dy).max(1e-300);
    let (mut worst, mut idx) = (0.0, 0);
    for (i, p) in pts.iter().enumerate().take(pts.len() - 1).skip(1) {
        let d = ((p[0] - a[0]) * dy - (p[1] - a[1]) * dx).abs() / len;
        if d > worst {
            (worst, idx) = (d, i);
        }
    }
    if worst <= tol {
        return vec![a, b];
    }
    let mut left = simplify(&pts[..=idx], tol);
    left.pop();
    left.extend(simplify(&pts[idx..], tol));
    left
}

/// Body profile and fin parameters of `geom`, the same construction as the `panel` mesh
/// (nose tip at the origin, fin root trailing edge flush with the base when within 0.5 % of it).
pub fn cad_description(geom: &Geometry) -> (Vec<[f64; 2]>, Option<FinParams>) {
    let l = geom.length;
    let mut pts = vec![[0.0, 0.0]];
    for p in &geom.profile {
        if p.x > 0.0 && p.x < l && p.x > pts.last().unwrap()[0] {
            pts.push([p.x, p.r]);
        }
    }
    pts.push([l, geom.base_radius]);
    let radius = |x: f64| {
        let k = pts.partition_point(|p| p[0] <= x).clamp(1, pts.len() - 1);
        let (a, b) = (pts[k - 1], pts[k]);
        a[1] + (b[1] - a[1]) * ((x - a[0]) / (b[0] - a[0])).clamp(0.0, 1.0)
    };
    let fins = geom.fins.as_ref().filter(|f| f.count > 0 && f.span > 0.0 && f.root_chord > 0.0 && f.thickness > 0.0).map(|f| {
        let x_le = f.x_le_root.max(0.0);
        let mut x_te = f.x_le_root + f.root_chord;
        if x_te >= l * 0.995 {
            x_te = l;
        }
        let inner = (0..=20).map(|i| radius(x_le + (x_te - x_le) * i as f64 / 20.0)).fold(f64::INFINITY, f64::min);
        FinParams {
            count: f.count,
            x_le,
            x_te,
            tip_chord: f.tip_chord,
            sweep: f.sweep,
            span: f.span,
            thickness: f.thickness,
            body_radius: f.body_radius,
            inner_radius: inner,
        }
    });
    (simplify(&pts, 5e-5 * l), fins)
}

/// Profile of the CFD body: [`cad_description`] plus the conical tail fairing of `tail_length`.
pub fn cfd_profile(geom: &Geometry, tail_length: f64) -> Vec<[f64; 2]> {
    let (mut prof, _) = cad_description(geom);
    if tail_length > 0.0 && geom.base_radius > 0.0 {
        prof.push([geom.length + tail_length, 0.0]);
    }
    prof
}

impl MeshParams {
    pub fn new(opt: &CfdOptions, geom: &Geometry, dir: &Path, threads: usize) -> Self {
        let length = geom.length;
        let r = opt.farfield * length;
        let p = |n: &str| dir.join(n).display().to_string();
        let (_, fins) = cad_description(geom);
        let tail_length = if opt.model == FlowModel::Euler { opt.tail_fairing * geom.base_radius } else { 0.0 };
        let profile = cfd_profile(geom, tail_length);
        MeshParams {
            out_su2: p("mesh.su2"),
            stats_json: p("mesh_stats.json"),
            length,
            center_x: 0.5 * length,
            farfield_radius: r,
            symmetry: opt.symmetry,
            wall_size: opt.wall_size,
            max_size: r / 10.0,
            growth: 1.2,
            curvature_elements: 24,
            min_size_divisor: 8.0,
            tail_length,
            threads: threads.max(1),
            profile,
            fins,
        }
    }
}

/// Mesh statistics (`mesh_stats.json` written by the script).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MeshStats {
    pub nodes: usize,
    pub cells: usize,
    #[serde(default)]
    pub by_type: std::collections::BTreeMap<String, usize>,
    pub quality_sicn_min: f64,
    pub quality_sicn_mean: f64,
    #[serde(rename = "cells_sicn_below_0.05")]
    pub cells_bad: usize,
    #[serde(default)]
    pub wall_triangles: usize,
    #[serde(default)]
    pub seconds_setup: f64,
    #[serde(default)]
    pub seconds_mesh: f64,
}

/// A mesh on disk.
#[derive(Debug, Clone)]
pub struct MeshResult {
    pub su2: PathBuf,
    pub hash: String,
    pub stats: MeshStats,
    pub cached: bool,
}

/// Hash of the geometry description, every setting that changes the mesh and the script itself.
pub fn mesh_hash(params: &MeshParams) -> String {
    let mut q = params.clone();
    // Paths and thread count do not change the mesh.
    q.out_su2.clear();
    q.stats_json.clear();
    q.threads = 0;
    hash_parts(&[serde_json::to_string(&q).unwrap_or_default().as_bytes(), SCRIPT.as_bytes()])
}

#[derive(Serialize, Deserialize)]
struct Stamp {
    hash: String,
    stats: MeshStats,
}

/// Meshes (or reuses the cached mesh) into `<workdir>/mesh/`.
pub fn ensure_mesh(opt: &CfdOptions, geom: &Geometry, workdir: &Path, threads: usize) -> Result<MeshResult> {
    let dir = workdir.join("mesh");
    std::fs::create_dir_all(&dir).with_context(|| format!("cannot create {}", dir.display()))?;
    let params = MeshParams::new(opt, geom, &dir, threads);
    let hash = mesh_hash(&params);
    let su2 = dir.join("mesh.su2");
    let stamp_path = dir.join("mesh.json");
    if let Some(st) = std::fs::read_to_string(&stamp_path).ok().and_then(|t| serde_json::from_str::<Stamp>(&t).ok()) {
        if st.hash == hash && su2.exists() {
            return Ok(MeshResult { su2, hash, stats: st.stats, cached: true });
        }
    }
    let _ = std::fs::remove_file(&stamp_path);
    let tools = tools::check_tools(opt, &[]);
    if !tools.gmsh.ok() {
        bail!("{}", crate::not_installed_message(&tools, opt));
    }
    std::fs::write(dir.join("mesh_gmsh.py"), SCRIPT)?;
    std::fs::write(dir.join("params.json"), serde_json::to_string_pretty(&params)?)?;
    let py = tools.gmsh.path.clone().context("no python with gmsh")?;
    let out = tools::command(&py, tools.prefix.as_deref())
        .arg(dir.join("mesh_gmsh.py"))
        .arg(dir.join("params.json"))
        .current_dir(&dir)
        .output()
        .with_context(|| format!("cannot run {}", py.display()))?;
    let log = format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
    std::fs::write(dir.join("mesh.log"), &log)?;
    if !out.status.success() || !log.contains("MESH_OK") {
        let tail: Vec<&str> = log.lines().rev().take(15).collect::<Vec<_>>().into_iter().rev().collect();
        bail!("gmsh meshing failed (log: {}):\n{}", dir.join("mesh.log").display(), tail.join("\n"));
    }
    let stats: MeshStats = serde_json::from_str(&std::fs::read_to_string(&params.stats_json)?).context("bad mesh_stats.json")?;
    std::fs::write(&stamp_path, serde_json::to_string(&Stamp { hash: hash.clone(), stats: stats.clone() })?)?;
    Ok(MeshResult { su2, hash, stats, cached: false })
}

#[cfg(test)]
mod tests {
    use super::*;
    use aero::atmosphere::AtmosphereModel;

    #[test]
    fn first_height_is_flat_plate_estimate() {
        let atm = AtmosphereModel::default().at(0.0);
        let y = first_height(1.0, 1.6, &atm, 0.8);
        assert!(y > 1e-6 && y < 1e-4, "{y}");
        assert!((first_height(2.0, 1.6, &atm, 0.8) / y - 2.0).abs() < 1e-12);
        assert!(first_height(1.0, 1.6, &atm, 2.0) < y);
        let o = CfdOptions { machs: vec![0.3, 2.0], wall_size: 0.005, ..Default::default() };
        assert!((wall_yplus(&o, 1.6, &atm) - 0.2 * 0.005 / first_height(1.0, 1.6, &atm, 2.0)).abs() < 1e-9);
    }

    fn sample_geom() -> Geometry {
        use geom::sample::SampleRocket;
        geom::extract(&SampleRocket::default().mesh(), &geom::ExtractOptions::default()).unwrap()
    }

    #[test]
    fn params_and_script_agree() {
        let o = CfdOptions::default();
        let g = sample_geom();
        let p = MeshParams::new(&o, &g, Path::new("/w"), 4);
        assert!((p.farfield_radius - 20.0 * g.length).abs() < 1e-12);
        assert!((p.max_size - 2.0 * g.length).abs() < 1e-12);
        assert!((p.center_x - 0.5 * g.length).abs() < 1e-12);
        let j = serde_json::to_value(&p).unwrap();
        // Every key the script reads must be provided.
        let mut n = 0;
        for line in SCRIPT.lines() {
            let mut rest = line;
            while let Some(i) = rest.find("p[\"") {
                rest = &rest[i + 3..];
                let key = &rest[..rest.find('"').unwrap()];
                assert!(j.get(key).is_some(), "script reads missing key {key}");
                n += 1;
            }
        }
        assert!(n > 10);
        for name in ["wall", "base", "farfield", "symmetry", "fluid"] {
            assert!(SCRIPT.contains(&format!("\"{name}\"")), "{name}");
        }
        assert!(SCRIPT.contains("occ.fuse") && SCRIPT.contains("occ.cut") && SCRIPT.contains("Mesh.Algorithm3D"));
    }

    #[test]
    fn cad_description_matches_the_geometry() {
        let g = sample_geom();
        let (prof, fins) = cad_description(&g);
        assert_eq!(prof[0], [0.0, 0.0]);
        assert!((prof.last().unwrap()[0] - g.length).abs() < 1e-12);
        assert!(prof.len() < 120, "{}", prof.len());
        let max_r = prof.iter().map(|p| p[1]).fold(0.0, f64::max);
        assert!((max_r - g.ref_radius).abs() < 1e-3 * g.ref_radius);
        let f = fins.unwrap();
        assert_eq!(f.count, 4);
        assert!(f.x_te <= g.length + 1e-12 && f.x_te > f.x_le);
        assert!(f.inner_radius > 0.0 && f.inner_radius <= f.body_radius + 1e-9);
        let (_, none) = cad_description(&Geometry { fins: None, ..g });
        assert!(none.is_none());
    }

    #[test]
    fn simplify_keeps_ends_and_tolerance() {
        let pts: Vec<[f64; 2]> = (0..=100).map(|i| [i as f64 * 0.01, (i as f64 * 0.01).sqrt() * 0.1]).collect();
        let s = simplify(&pts, 1e-4);
        assert_eq!((s[0], *s.last().unwrap()), (pts[0], pts[100]));
        assert!(s.len() < pts.len() && s.len() > 3);
        let line: Vec<[f64; 2]> = (0..10).map(|i| [i as f64, 2.0 * i as f64]).collect();
        assert_eq!(simplify(&line, 1e-9).len(), 2);
    }

    #[test]
    fn hash_changes_with_settings_not_paths() {
        let o = CfdOptions::default();
        let g = sample_geom();
        let a = MeshParams::new(&o, &g, Path::new("/a"), 4);
        let b = MeshParams::new(&o, &g, Path::new("/b"), 4);
        assert_eq!(mesh_hash(&a), mesh_hash(&b));
        let c = MeshParams::new(&CfdOptions { wall_size: 0.01, ..o.clone() }, &g, Path::new("/a"), 4);
        assert_ne!(mesh_hash(&a), mesh_hash(&c));
        let d = MeshParams::new(&CfdOptions { symmetry: false, ..o.clone() }, &g, Path::new("/a"), 4);
        assert_ne!(mesh_hash(&a), mesh_hash(&d));
        let mut g2 = g.clone();
        g2.fins.as_mut().unwrap().span *= 1.1;
        assert_ne!(mesh_hash(&a), mesh_hash(&MeshParams::new(&o, &g2, Path::new("/a"), 4)));
    }
}
