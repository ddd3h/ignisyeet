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
//! * Volume (Euler): tetrahedra (HXT) with a size field `min(farfield L / 10, wall_size + 0.2 d)` of the
//!   distance `d` to the wall, i.e. growth about 1.2 per element row. The SU2 mesh is written by gmsh with the
//!   physical groups `wall`, `base`, `farfield`, `symmetry`.
//! * Fin faces (`fin_size`, [`CfdOptions::fin_size_for`]; Euler default a quarter of the fin thickness): a second
//!   threshold field of the distance to the fin faces gives `fin_size` there, graded by the same growth (the minimum of
//!   both fields; the smallest size is then at most `fin_size`). A fin face is a wall face whose centre of mass lies
//!   outside the body profile by more than a tenth of the span. With `fin_size = wall_size` (RANS default) the size field
//!   is the wall field alone.
//! * Volume (RANS, [`BlParams`]): prismatic boundary layer on wall and base, built by the script from the surface mesh
//!   of the half solid `y >= 0` (same size field as Euler). Columns along node normals: the most visible direction of
//!   the incident wall triangles, in the plane on `y = 0`, smoothed where that keeps them visible and fanned out of
//!   concave corners (the fin-body junction) like a fillet. Layer law ([`bl_layers`]): first height from
//!   [`first_height`] for `yplus` at 5 % of the body length and the largest solved Mach number, growth ratio
//!   [`BL_GROWTH`] until the last layer is [`BL_TOP`] `wall_size` (about the height of the tetrahedra outside, so the
//!   size changes smoothly at the layer top), with fewer layers where that would be thicker than [`BL_DELTA_FACTOR`]
//!   turbulent boundary layer thicknesses at the body end or [`BL_RADIUS_FRACTION`] of the body radius (the tetrahedra
//!   then start from the last layer height and grow by 1.2 per row up to `wall_size`); 22 mm at wall_size 2.5 mm, 25 mm
//!   (half the body radius) from 3 mm on for the sample rocket. Base columns are at most [`BL_BASE_FRACTION`] of the
//!   base radius: behind the base they only resolve the reverse flow of the wake. Near the junction the columns are
//!   thinner (at most [`BL_JUNCTION`] of the thickness plus the distance to the junction), so are columns that diverge
//!   from their neighbours round fin edges and tips (the layer top widens by at most [`BL_FAN`] `wall_size`), and
//!   columns whose prisms would invert are thinned locally (the layer count stays; a thinner column grows more slowly).
//!   The fin edges are blunted (the section thickness of `xi` in `[edge_blunt, 1 - edge_blunt]` stretched over the
//!   chord, about 8 % of the thickness at the edges). Outside, gmsh meshes the `y = 0` plane around the layer section
//!   and a box farfield (half width `farfield L`), and fills the volume with tetrahedra of size `wall_size` at the layer
//!   top growing by 1.2 per element row. The mesh (tetrahedra, prisms; marker triangles and the quadrilateral layer
//!   faces on `y = 0`) is written by `su2_writer.py`, which orients the cells by their signed volume and rejects
//!   inverted prisms, cells of non-positive volume and boundary faces without a marker; an invalid layer is meshed again
//!   once with more smoothing and a thinner junction, then it is a mesh error. The full domain is the half mirrored
//!   about `y = 0`; `z_mirror_mesh` has no effect, and a fin layout that is not symmetric about `y = 0` is rejected. The
//!   SICN quality covers the tetrahedra only.
//!
//! The mesh is cached below `<workdir>/mesh/` keyed by a hash of the geometry and the settings.

use crate::case::hash_parts;
use crate::config::{y_symmetric_layout, CfdOptions, FlowModel};
use crate::tools;
use aero::atmosphere::Atmosphere;
use anyhow::{bail, Context, Result};
use geom::Geometry;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub const SCRIPT: &str = include_str!("mesh_gmsh.py");
/// SU2 writer of the boundary-layer mesh (imported by [`SCRIPT`]).
pub const WRITER: &str = include_str!("su2_writer.py");
/// Prismatic boundary layer for the half or full domain (imported by [`SCRIPT`] and `mesh_step.py`).
pub const BL_LAYER: &str = include_str!("bl_layer.py");

/// Growth ratio of the prism layer heights.
pub const BL_GROWTH: f64 = 1.2;
/// Height of the last prism layer in wall sizes (about the height of the tetrahedra at the layer top).
pub const BL_TOP: f64 = 1.3;
/// Column thickness at the fin-body junction in layer thicknesses (it grows with the distance to the junction).
pub const BL_JUNCTION: f64 = 0.5;
/// Cap of the layer thickness: this multiple of the turbulent flat-plate boundary layer thickness `0.37 L Re_L^-0.2`
/// at the body end and the smallest solved Mach number (the thickest boundary layer of the run).
pub const BL_DELTA_FACTOR: f64 = 1.5;
/// Cap of the layer thickness in largest body radii.
pub const BL_RADIUS_FRACTION: f64 = 0.5;
/// Cap of the columns on the flat base in base radii: behind the base the layers only resolve the reverse flow of the
/// wake next to the wall.
pub const BL_BASE_FRACTION: f64 = 0.25;
/// Largest widening of the layer top where neighbouring columns diverge (round fin edges and tips), in wall sizes.
pub const BL_FAN: f64 = 2.0;
/// Attempts of the prism layer, the second after the first failed the validity check: normal smoothing passes and
/// junction thickness ([`BL_JUNCTION`]).
pub const BL_ATTEMPTS: [(usize, f64); 2] = [(3, BL_JUNCTION), (10, 0.25)];
/// Fin edge bluntness: the biconvex section is cut at this fraction of the chord from each edge.
pub const BL_EDGE_BLUNT: f64 = 0.02;
/// The first layer height gives `yplus` at this fraction of the body length (behind the nose tip).
pub const BL_YPLUS_STATION: f64 = 0.05;

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

/// Estimated y+ of the first prism layer (RANS): the first layer height of [`bl_params`] (chosen for `yplus` at
/// [`BL_YPLUS_STATION`] of the body length) over the flat-plate height of y+ = 1 at the body length, at the largest
/// solved Mach number (see [`first_height`]).
pub fn wall_yplus(opt: &CfdOptions, length: f64, atm: &Atmosphere) -> f64 {
    let m = opt.machs.iter().copied().fold(0.0, f64::max);
    first_height(opt.yplus, BL_YPLUS_STATION * length, atm, m) / first_height(1.0, length, atm, m)
}

/// Prism boundary layer (RANS), `bl` in `params.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BlParams {
    /// First layer height [m].
    pub first_height: f64,
    pub layers: usize,
    /// Geometric growth ratio of the layer heights.
    pub ratio: f64,
    /// Total thickness [m].
    pub thickness: f64,
    /// Fin edge bluntness (fraction of the chord, see [`BL_EDGE_BLUNT`]).
    pub edge_blunt: f64,
    /// Normal smoothing passes.
    pub smoothing: usize,
    /// Smoothing passes of the normals near the junction (they fan out of the corner).
    pub corner_smoothing: usize,
    /// Column thickness at the junction in layer thicknesses (see [`BL_JUNCTION`]).
    pub junction: f64,
    /// Largest widening of the layer top over diverging columns, in wall sizes ([`BL_FAN`]).
    pub fan: f64,
    /// Thickness cap of the columns on the base [m] ([`BL_BASE_FRACTION`]).
    pub base_thickness: f64,
    /// Size of the tetrahedra at the layer top [m]: the smaller of `wall_size` and the last layer height; they grow
    /// by 1.2 per element row from there.
    pub top_size: f64,
    /// Parameters of the next attempt after an invalid layer (`None` = mesh error).
    pub retry: Option<Box<BlParams>>,
}

/// Layer count and total thickness of a prism layer of first height `h1` growing by `ratio` until the last layer is at
/// least `top` high (one layer if `h1` already is).
pub fn bl_layers(h1: f64, top: f64, ratio: f64) -> (usize, f64) {
    let n = if h1 >= top || ratio <= 1.0 { 1 } else { ((top / h1).ln() / ratio.ln() - 1e-9).ceil() as usize + 1 };
    let t = if ratio == 1.0 { h1 * n as f64 } else { h1 * (ratio.powi(n as i32) - 1.0) / (ratio - 1.0) };
    (n, t)
}

/// Turbulent flat-plate boundary layer thickness `0.37 x Re_x^-0.2` at `x` for the freestream of `atm` at `mach`.
pub fn turbulent_delta(x: f64, atm: &Atmosphere, mach: f64) -> f64 {
    let re = mach * atm.sound_speed * x / atm.kinematic_viscosity();
    0.37 * x * re.powf(-0.2)
}

/// Prism layer of a RANS mesh: first height for `opt.yplus` at [`BL_YPLUS_STATION`] of the body length and the
/// largest solved Mach number of `atm`; growth [`BL_GROWTH`] to a last layer of [`BL_TOP`] `wall_size`
/// ([`bl_layers`]), with fewer layers where that is thicker than [`BL_DELTA_FACTOR`] boundary layer thicknesses at the
/// body end (smallest solved Mach number) or [`BL_RADIUS_FRACTION`] of the largest body radius; base columns at most
/// [`BL_BASE_FRACTION`] of the base radius; and the retry of [`BL_ATTEMPTS`].
pub fn bl_params(opt: &CfdOptions, geom: &Geometry, atm: &Atmosphere) -> BlParams {
    let m_max = opt.machs.iter().copied().fold(0.0, f64::max);
    let m_min = opt.machs.iter().copied().filter(|&m| m > 0.0).fold(f64::INFINITY, f64::min).min(m_max);
    let h1 = first_height(opt.yplus, BL_YPLUS_STATION * geom.length, atm, m_max);
    let (mut layers, mut thickness) = bl_layers(h1, BL_TOP * opt.wall_size, BL_GROWTH);
    let radius = geom.profile.iter().map(|p| p.r).fold(geom.base_radius, f64::max);
    let cap = (BL_DELTA_FACTOR * turbulent_delta(geom.length, atm, m_min)).min(BL_RADIUS_FRACTION * radius);
    while layers > 1 && thickness > cap {
        layers -= 1;
        thickness = h1 * (BL_GROWTH.powi(layers as i32) - 1.0) / (BL_GROWTH - 1.0);
    }
    let last = h1 * BL_GROWTH.powi(layers as i32 - 1);
    let mut next: Option<Box<BlParams>> = None;
    for &(smoothing, junction) in BL_ATTEMPTS.iter().rev() {
        next = Some(Box::new(BlParams {
            first_height: h1,
            layers,
            ratio: BL_GROWTH,
            thickness,
            edge_blunt: BL_EDGE_BLUNT,
            smoothing,
            corner_smoothing: (2.0 * thickness / opt.wall_size).round() as usize,
            junction,
            fan: BL_FAN,
            base_thickness: thickness.min(BL_BASE_FRACTION * geom.base_radius),
            top_size: opt.wall_size.min(last),
            retry: next,
        }));
    }
    *next.expect("BL_ATTEMPTS is not empty")
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
    /// Mesh only the half y >= 0 and keep the `symmetry` marker (SU2 half domain).
    pub symmetry: bool,
    /// Full domain of a y-symmetric layout: mesh the half and mirror it about y = 0 (no `symmetry` marker).
    pub y_mirror: bool,
    /// `half`, `full_mirror` (half mesh mirrored about y = 0), `full_box` (box farfield), `stl_box`, `stl_half`,
    /// `stl_half_full` (the STL's half mirrored to the full domain), `stl_quarter_half` or `stl_quarter_full`:
    /// part of the mesh hash.
    pub domain: String,
    /// `cad` (rebuilt geometry, `mesh_gmsh.py`) or `stl` (direct STL surface, `mesh_stl.py`).
    pub kind: String,
    /// STL surface: hash of the united surface the mesh was made from (empty for `cad`).
    pub union_hash: String,
    /// `stl`: file name of the united surface (binary STL, next to the script) and how it is prepared (`classify`, `none`).
    pub stl_surface: String,
    pub stl_remesh: String,
    /// `stl`: 3D algorithm of gmsh, 10 = HXT, 1 = Delaunay (the retry after an HXT failure).
    pub algorithm3d: u32,
    pub wall_size: f64,
    /// Surface element size on the fin faces ([`CfdOptions::fin_size_for`]); equal to `wall_size`: no fin refinement.
    pub fin_size: f64,
    pub max_size: f64,
    pub growth: f64,
    pub curvature_elements: usize,
    pub min_size_divisor: f64,
    /// Length of the conical tail fairing behind the base [m]; 0 = flat base.
    pub tail_length: f64,
    pub threads: usize,
    /// Roll of the fin set about the body axis [deg].
    pub fin_roll_deg: f64,
    /// Mesh the quarter domain and mirror it about z = 0 (exactly z-symmetric half-model mesh).
    pub z_mirror: bool,
    /// Body profile `(x, r)` from the nose tip to the base.
    pub profile: Vec<[f64; 2]>,
    pub fins: Option<FinParams>,
    /// Prismatic boundary layer (RANS on the rebuilt geometry); `None` = tetrahedra only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bl: Option<BlParams>,
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
        let layout_y = y_symmetric_layout(fins.as_ref().map_or(0, |f| f.count), opt.fin_roll_deg);
        let (y_mirror, domain) = match (opt.symmetry, layout_y) {
            (true, _) => (false, "half"),
            (false, true) => (true, "full_mirror"),
            (false, false) => (false, "full_box"),
        };
        MeshParams {
            out_su2: p("mesh.su2"),
            y_mirror,
            domain: domain.into(),
            kind: "cad".into(),
            union_hash: String::new(),
            stl_surface: String::new(),
            stl_remesh: String::new(),
            algorithm3d: 10,
            stats_json: p("mesh_stats.json"),
            length,
            center_x: 0.5 * length,
            farfield_radius: r,
            symmetry: opt.symmetry,
            wall_size: opt.wall_size,
            fin_size: opt.fin_size_for(fins.as_ref().map(|f| f.thickness)),
            max_size: r / 10.0,
            growth: 1.2,
            curvature_elements: 24,
            min_size_divisor: 8.0,
            tail_length,
            threads: threads.max(1),
            fin_roll_deg: opt.fin_roll_deg,
            z_mirror: opt.z_mirror_mesh,
            profile,
            fins,
            bl: None,
        }
    }

    /// Parameters of the rebuilt geometry (`surface = "cad"`): [`MeshParams::new`] plus the prism layer for RANS
    /// ([`bl_params`] with the freestream `atm`), which is meshed as the half model (no `z_mirror`).
    pub fn cad(opt: &CfdOptions, geom: &Geometry, atm: &Atmosphere, dir: &Path, threads: usize) -> Self {
        let mut p = Self::new(opt, geom, dir, threads);
        if opt.model == FlowModel::Rans {
            p.bl = Some(bl_params(opt, geom, atm));
            p.z_mirror = false;
        }
        p
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
    /// Prism layer (RANS): number of layers, first height and total thickness [m] (0 without a layer).
    #[serde(default)]
    pub bl_layers: usize,
    #[serde(default)]
    pub bl_first_height: f64,
    #[serde(default)]
    pub bl_thickness: f64,
    /// Inverted prisms of the written mesh (always 0: an invalid layer is a mesh error).
    #[serde(default)]
    pub bl_invalid: usize,
    /// Attempt of [`BL_ATTEMPTS`] that gave the mesh (1 = first; 0 without a layer).
    #[serde(default)]
    pub bl_attempt: usize,
    /// Size change at the layer top: height of the tetrahedron on a top face over the last layer height there (or
    /// its inverse), 99th percentile and largest.
    #[serde(default)]
    pub bl_top_ratio_p99: f64,
    #[serde(default)]
    pub bl_top_ratio_max: f64,
}

/// A mesh on disk.
#[derive(Debug, Clone)]
pub struct MeshResult {
    pub su2: PathBuf,
    pub hash: String,
    pub stats: MeshStats,
    pub cached: bool,
    /// `surface = "stl"`: repair, union and geometry fidelity of the wall.
    pub stl: Option<crate::stl_mesh::StlMeshInfo>,
    /// `surface = "step"`: import, registration against the STL, symmetry and domain.
    pub step: Option<crate::step_mesh::StepMeshInfo>,
}

/// Hash of the geometry description, every setting that changes the mesh and the script itself.
pub fn mesh_hash(params: &MeshParams) -> String {
    let mut q = params.clone();
    // Paths and thread count do not change the mesh.
    q.out_su2.clear();
    q.stats_json.clear();
    q.threads = 0;
    let json = serde_json::to_string(&q).unwrap_or_default();
    if q.kind == "stl" {
        hash_parts(&[json.as_bytes(), crate::stl_mesh::SCRIPT.as_bytes(), crate::stl_mesh::UNION_SCRIPT.as_bytes()])
    } else {
        hash_parts(&[json.as_bytes(), SCRIPT.as_bytes(), WRITER.as_bytes(), BL_LAYER.as_bytes()])
    }
}

/// Runs a python script with the gmsh interpreter in `dir` (`script params`), logging to `log`.
/// Returns the log text; fails unless the process succeeds and the log contains `ok_token`.
#[allow(clippy::too_many_arguments)]
pub(crate) fn run_python(py: &Path, prefix: Option<&Path>, dir: &Path, script: &str, params: &str, log: &str, ok_token: &str, what: &str) -> Result<String> {
    let out = tools::command(py, prefix)
        .arg(dir.join(script))
        .arg(dir.join(params))
        .current_dir(dir)
        // Unbuffered: the progress lines stay in the log when the process is killed.
        .env("PYTHONUNBUFFERED", "1")
        .output()
        .with_context(|| format!("cannot run {}", py.display()))?;
    let text = format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
    std::fs::write(dir.join(log), &text)?;
    if !out.status.success() || !text.contains(ok_token) {
        // The progress lines of gmsh use carriage returns; keep the last meaningful lines.
        let clean = text.replace('\r', "\n");
        let tail: Vec<&str> =
            clean.lines().filter(|l| !l.contains("%]")).collect::<Vec<_>>().into_iter().rev().take(12).collect::<Vec<_>>().into_iter().rev().collect();
        let killed = if out.status.success() || failure_kind(&out.status) == "gmsh meshing failed" {
            String::new()
        } else {
            format!(" ({})", failure_kind(&out.status).replacen("gmsh meshing was ", "", 1))
        };
        bail!("{what} failed{killed} (log: {}):\n{}", dir.join(log).display(), tail.join("\n"));
    }
    Ok(text)
}

#[derive(Serialize, Deserialize)]
struct Stamp {
    hash: String,
    stats: MeshStats,
}

/// What went wrong with the mesher process: killed by a signal (`SIGKILL`, 9, is the out-of-memory killer) or failed.
fn failure_kind(status: &std::process::ExitStatus) -> String {
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        if let Some(sig) = status.signal() {
            return format!(
                "gmsh meshing was killed by signal {sig}{}",
                if sig == 9 { " (most likely out of memory: close other programs or use a larger wall_size)" } else { "" }
            );
        }
    }
    let _ = status;
    "gmsh meshing failed".into()
}

/// Meshes (or reuses the cached mesh) into `<workdir>/mesh/`. `atm` is the freestream of the cases (prism layer
/// height of RANS).
pub fn ensure_mesh(opt: &CfdOptions, geom: &Geometry, atm: &Atmosphere, workdir: &Path, threads: usize) -> Result<MeshResult> {
    let dir = workdir.join("mesh");
    std::fs::create_dir_all(&dir).with_context(|| format!("cannot create {}", dir.display()))?;
    let params = MeshParams::cad(opt, geom, atm, &dir, threads);
    if params.bl.is_some() && params.domain == "full_box" {
        bail!(
            "aero.cfd: the RANS boundary-layer mesh needs a fin layout symmetric about the y = 0 plane (fin_roll_deg = {} with {} fins is not): \
             use a symmetric fin_roll_deg (for example 0 or 45 with 4 fins) or model = \"euler\"",
            opt.fin_roll_deg,
            cad_description(geom).1.map_or(0, |f| f.count)
        );
    }
    let hash = mesh_hash(&params);
    let su2 = dir.join("mesh.su2");
    let stamp_path = dir.join("mesh.json");
    if let Some(st) = std::fs::read_to_string(&stamp_path).ok().and_then(|t| serde_json::from_str::<Stamp>(&t).ok()) {
        if st.hash == hash && su2.exists() {
            return Ok(MeshResult { su2, hash, stats: st.stats, cached: true, stl: None, step: None });
        }
    }
    let _ = std::fs::remove_file(&stamp_path);
    let tools = tools::check_tools(opt, &[]);
    if !tools.gmsh.ok() {
        bail!("{}", crate::not_installed_message(&tools, opt));
    }
    std::fs::write(dir.join("mesh_gmsh.py"), SCRIPT)?;
    std::fs::write(dir.join("su2_writer.py"), WRITER)?;
    std::fs::write(dir.join("bl_layer.py"), BL_LAYER)?;
    let _ = std::fs::remove_file(dir.join("params_retry.json"));
    std::fs::write(dir.join("params.json"), serde_json::to_string_pretty(&params)?)?;
    let py = tools.gmsh.path.clone().context("no python with gmsh")?;
    let out = tools::command(&py, tools.prefix.as_deref())
        .arg(dir.join("mesh_gmsh.py"))
        .arg(dir.join("params.json"))
        .current_dir(&dir)
        // Unbuffered: the progress lines stay in the log when the process is killed.
        .env("PYTHONUNBUFFERED", "1")
        .output()
        .with_context(|| format!("cannot run {}", py.display()))?;
    let log = format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
    std::fs::write(dir.join("mesh.log"), &log)?;
    if !out.status.success() || !log.contains("MESH_OK") {
        let tail: Vec<&str> = log.lines().rev().take(15).collect::<Vec<_>>().into_iter().rev().collect();
        bail!("{} (log: {}):\n{}", failure_kind(&out.status), dir.join("mesh.log").display(), tail.join("\n"));
    }
    let stats: MeshStats = serde_json::from_str(&std::fs::read_to_string(&params.stats_json)?).context("bad mesh_stats.json")?;
    std::fs::write(&stamp_path, serde_json::to_string(&Stamp { hash: hash.clone(), stats: stats.clone() })?)?;
    Ok(MeshResult { su2, hash, stats, cached: false, stl: None, step: None })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_tmp::TempDir;
    use aero::atmosphere::AtmosphereModel;

    #[test]
    fn first_height_is_flat_plate_estimate() {
        let atm = AtmosphereModel::default().at(0.0);
        let y = first_height(1.0, 1.6, &atm, 0.8);
        assert!(y > 1e-6 && y < 1e-4, "{y}");
        assert!((first_height(2.0, 1.6, &atm, 0.8) / y - 2.0).abs() < 1e-12);
        assert!(first_height(1.0, 1.6, &atm, 2.0) < y);
        let o = CfdOptions { machs: vec![0.3, 2.0], wall_size: 0.005, ..Default::default() };
        let h1 = first_height(1.0, BL_YPLUS_STATION * 1.6, &atm, 2.0);
        assert!((wall_yplus(&o, 1.6, &atm) - h1 / first_height(1.0, 1.6, &atm, 2.0)).abs() < 1e-9);
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
        // Every key a script reads must be provided.
        let mut n = 0;
        for (name, script) in [("mesh_gmsh.py", SCRIPT), ("mesh_stl.py", crate::stl_mesh::SCRIPT), ("stl_union.py", crate::stl_mesh::UNION_SCRIPT)] {
            let mut keys = 0;
            for line in script.lines() {
                let mut rest = line;
                while let Some(i) = rest.find("p[\"") {
                    rest = &rest[i + 3..];
                    let key = &rest[..rest.find('"').unwrap()];
                    // The union script has its own small parameter file.
                    if name != "stl_union.py" {
                        assert!(j.get(key).is_some(), "{name} reads missing key {key}");
                    } else {
                        assert!(["components", "out_stl", "report_json"].contains(&key), "{key}");
                    }
                    keys += 1;
                    n += 1;
                }
            }
            assert!(keys >= 3, "{name}");
        }
        assert!(n > 20);
        for name in ["wall", "base", "farfield", "fluid"] {
            assert!(crate::stl_mesh::SCRIPT.contains(&format!("\"{name}\"")), "{name}");
        }
        assert!(
            crate::stl_mesh::SCRIPT.contains("Mesh.Algorithm3D")
                && crate::stl_mesh::SCRIPT.contains("classifySurfaces")
                && crate::stl_mesh::UNION_SCRIPT.contains("batch_boolean")
        );
        for name in ["wall", "base", "farfield", "symmetry", "fluid"] {
            assert!(SCRIPT.contains(&format!("\"{name}\"")), "{name}");
        }
        assert!(SCRIPT.contains("occ.fuse") && SCRIPT.contains("occ.cut") && SCRIPT.contains("Mesh.Algorithm3D"));
        assert!(SCRIPT.contains("bl_layer.build(") && BL_LAYER.contains("def column_heights(") && SCRIPT.contains("Geometry.Tolerance"));
        for f in ["def write(", "def mirror(", "def orient(", "def check(", "def unmarked_faces("] {
            assert!(WRITER.contains(f), "{f}");
        }
    }

    #[cfg(unix)]
    #[test]
    fn run_python_names_a_killed_process() {
        let dir = TempDir::new("run_python_kill");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("die.sh"), "echo started\nkill -9 $$\n").unwrap();
        std::fs::write(dir.join("p.json"), "{}").unwrap();
        let e = run_python(Path::new("/bin/sh"), None, &dir, "die.sh", "p.json", "log.txt", "OK", "the test mesher").unwrap_err().to_string();
        assert!(e.contains("the test mesher failed (killed by signal 9 (most likely out of memory") && e.contains("started"), "{e}");
        std::fs::write(dir.join("fail.sh"), "echo nope\nexit 1\n").unwrap();
        let e = run_python(Path::new("/bin/sh"), None, &dir, "fail.sh", "p.json", "log.txt", "OK", "the test mesher").unwrap_err().to_string();
        assert!(e.starts_with("the test mesher failed (log:"), "{e}");
    }

    #[cfg(unix)]
    #[test]
    fn a_killed_mesher_is_reported_as_such() {
        use std::os::unix::process::ExitStatusExt;
        let killed = failure_kind(&std::process::ExitStatus::from_raw(9));
        assert!(killed.contains("killed by signal 9") && killed.contains("out of memory"), "{killed}");
        assert_eq!(failure_kind(&std::process::ExitStatus::from_raw(1 << 8)), "gmsh meshing failed");
    }

    #[test]
    fn bl_layers_grow_to_the_top_height() {
        for (h1, top) in [(5.7e-7, 6.5e-3), (5.7e-7, 3.25e-3), (1e-5, 2e-3), (1e-4, 1.2e-3)] {
            let (n, t) = bl_layers(h1, top, 1.2);
            let last = h1 * 1.2f64.powi(n as i32 - 1);
            assert!(last >= top * (1.0 - 1e-9) && last < 1.2 * top, "{h1} {top}: {n} {last}");
            let total: f64 = (0..n).map(|i| h1 * 1.2f64.powi(i as i32)).sum();
            assert!((total / t - 1.0).abs() < 1e-9, "{total} {t}");
        }
        // 5 mm wall size at y+ 1 on the sample: 52 layers, 37 mm (last layer 6.5 mm).
        let (n, t) = bl_layers(5.715e-7, 1.3 * 0.005, 1.2);
        assert_eq!(n, 53);
        assert!(t > 0.03 && t < 0.045, "{t}");
        assert_eq!(bl_layers(1e-3, 8e-4, 1.2), (1, 1e-3));
    }

    #[test]
    fn bl_thickness_is_capped_by_the_boundary_layer_and_the_body_radius() {
        let g = sample_geom();
        let atm = AtmosphereModel::default().at(0.0);
        let radius = g.profile.iter().map(|p| p.r).fold(g.base_radius, f64::max);
        let o = |w: f64, machs: Vec<f64>| CfdOptions { model: FlowModel::Rans, wall_size: w, machs, ..Default::default() };
        // 8 mm: the growth law alone gives about 65 mm (more than the body radius); the cap is the smaller of 1.5 delta
        // at the body end (M 0.8) and half the body radius.
        let b = bl_params(&o(0.008, vec![0.8, 2.0]), &g, &atm);
        let cap = (BL_DELTA_FACTOR * turbulent_delta(g.length, &atm, 0.8)).min(BL_RADIUS_FRACTION * radius);
        let (n_law, t_law) = bl_layers(b.first_height, BL_TOP * 0.008, BL_GROWTH);
        assert!(t_law > 0.06 && b.thickness <= cap && b.layers < n_law, "{b:?} {t_law}");
        // As many layers as fit: one more would pass the cap.
        assert!(b.first_height * (BL_GROWTH.powi(b.layers as i32 + 1) - 1.0) / (BL_GROWTH - 1.0) > cap);
        // The tetrahedra at the top start from the last layer height, smaller than the wall size.
        let last = b.first_height * BL_GROWTH.powi(b.layers as i32 - 1);
        assert!((b.top_size - last).abs() < 1e-15 && b.top_size < 0.008, "{b:?}");
        assert!(b.base_thickness <= BL_BASE_FRACTION * g.base_radius + 1e-15);
        // 2.5 mm stays uncapped (about 22 mm) and keeps the wall size at the top.
        let c = bl_params(&o(0.0025, vec![0.8, 2.0]), &g, &atm);
        assert_eq!((c.layers, c.thickness), bl_layers(c.first_height, BL_TOP * 0.0025, BL_GROWTH));
        assert_eq!(c.top_size, 0.0025);
        // A lower Mach number has the thicker boundary layer.
        assert!(turbulent_delta(g.length, &atm, 0.3) > turbulent_delta(g.length, &atm, 2.0));
        let delta = turbulent_delta(1.0, &atm, 0.8);
        assert!(delta > 0.01 && delta < 0.02, "{delta}");
    }

    #[test]
    fn rans_rejects_a_fin_layout_asymmetric_about_y() {
        let g = sample_geom();
        let atm = AtmosphereModel::default().at(0.0);
        let dir = TempDir::new("rans_box");
        let opt = CfdOptions { model: FlowModel::Rans, symmetry: false, fin_roll_deg: 10.0, ..Default::default() };
        // Checked before gmsh runs, so the test needs no tools.
        let e = ensure_mesh(&opt, &g, &atm, &dir, 1).unwrap_err().to_string();
        assert!(e.contains("symmetric about the y = 0 plane") && e.contains("fin_roll_deg = 10"), "{e}");
    }

    #[test]
    fn rans_gets_a_prism_layer_and_euler_is_unchanged() {
        let g = sample_geom();
        let atm = AtmosphereModel::default().at(0.0);
        let euler = CfdOptions::default();
        let rans = CfdOptions { model: FlowModel::Rans, ..euler.clone() };
        let dir = Path::new("/a");
        // Euler: the CAD parameters are those of `new` (no `bl` key in params.json, so the same hash inputs).
        let e = MeshParams::cad(&euler, &g, &atm, dir, 4);
        assert!(e.bl.is_none() && e.z_mirror);
        assert_eq!(serde_json::to_string(&e).unwrap(), serde_json::to_string(&MeshParams::new(&euler, &g, dir, 4)).unwrap());
        assert!(!serde_json::to_string(&e).unwrap().contains("\"bl\""));
        // y+ changes the hash of RANS only.
        let e2 = MeshParams::cad(&CfdOptions { yplus: 5.0, ..euler.clone() }, &g, &atm, dir, 4);
        assert_eq!(mesh_hash(&e), mesh_hash(&e2));
        let r = MeshParams::cad(&rans, &g, &atm, dir, 4);
        let r2 = MeshParams::cad(&CfdOptions { yplus: 5.0, ..rans.clone() }, &g, &atm, dir, 4);
        assert_ne!(mesh_hash(&r), mesh_hash(&r2));
        assert_ne!(mesh_hash(&r), mesh_hash(&e));
        // The layer: half model without the quarter mirror, flat base, first height from the nose station.
        let bl = r.bl.as_ref().unwrap();
        assert!(!r.z_mirror && r.tail_length == 0.0 && r.domain == "half");
        let m = rans.machs.iter().copied().fold(0.0, f64::max);
        assert!((bl.first_height - first_height(1.0, BL_YPLUS_STATION * g.length, &atm, m)).abs() < 1e-15);
        let (n, t) = bl_layers(bl.first_height, BL_TOP * rans.wall_size, BL_GROWTH);
        assert!(bl.layers == n && bl.thickness == t && bl.ratio == BL_GROWTH && bl.corner_smoothing > 0, "{bl:?}");
        assert!(bl.top_size == rans.wall_size && bl.base_thickness == t.min(BL_BASE_FRACTION * g.base_radius), "{bl:?}");
        // The attempts in order, each the retry of the previous one.
        let mut a = Some(bl);
        for &(smoothing, junction) in &BL_ATTEMPTS {
            let b = a.expect("attempt");
            assert!(b.layers == n && b.smoothing == smoothing && b.junction == junction && b.edge_blunt == BL_EDGE_BLUNT, "{b:?}");
            a = b.retry.as_deref();
        }
        assert!(a.is_none());
        // y+ of the first layer at the body length is below the target (thinner boundary layer at the nose station).
        let y = wall_yplus(&rans, g.length, &atm);
        assert!(y > 0.3 && y < 1.0, "{y}");
        // Full domain of the symmetric layout: the half mesh mirrored.
        let f = MeshParams::cad(&CfdOptions { symmetry: false, ..rans.clone() }, &g, &atm, dir, 4);
        assert!(f.y_mirror && !f.z_mirror && f.bl.is_some());
        // Every key the script reads from `bl` is provided.
        let j = serde_json::to_value(bl).unwrap();
        let mut n = 0;
        for line in SCRIPT.lines().chain(BL_LAYER.lines()) {
            let mut rest = line;
            while let Some(i) = rest.find("bl[\"") {
                rest = &rest[i + 4..];
                let key = &rest[..rest.find('"').unwrap()];
                assert!(j.get(key).is_some(), "mesh_gmsh.py / bl_layer.py reads missing bl key {key}");
                n += 1;
            }
        }
        assert!(n >= 5, "{n}");
    }

    #[test]
    fn fin_size_reaches_the_params_and_the_hash() {
        let g = sample_geom();
        let t = g.fins.as_ref().unwrap().thickness;
        let atm = AtmosphereModel::default().at(0.0);
        // Euler: a quarter of the fin thickness below wall_size; the script refines the fin faces with it.
        let e = MeshParams::cad(&CfdOptions { wall_size: 0.006, ..Default::default() }, &g, &atm, Path::new("/a"), 4);
        assert!((e.fin_size - t / 4.0).abs() < 1e-12 && e.fin_size < e.wall_size, "{} {t}", e.fin_size);
        assert!(serde_json::to_value(&e).unwrap().get("fin_size").is_some());
        let ew = MeshParams::cad(&CfdOptions { wall_size: 0.006, fin_size: crate::FinSize::Size(0.006), ..Default::default() }, &g, &atm, Path::new("/a"), 4);
        assert_eq!(ew.fin_size, ew.wall_size);
        assert_ne!(mesh_hash(&e), mesh_hash(&ew));
        // RANS: fin_size = wall_size (no refinement), the same parameters as an explicit fin_size = wall_size.
        let r = MeshParams::cad(&CfdOptions::rans(), &g, &atm, Path::new("/a"), 4);
        assert_eq!(r.fin_size, r.wall_size);
        let rw = MeshParams::cad(
            &CfdOptions { fin_size: crate::FinSize::Size(crate::config::RANS_DEFAULT_WALL_SIZE), ..CfdOptions::rans() },
            &g,
            &atm,
            Path::new("/a"),
            4,
        );
        assert_eq!(mesh_hash(&r), mesh_hash(&rw));
        // The fin size field: a Distance + Threshold on the fin faces combined with the wall field by Min, only below
        // wall_size (fin_size = wall_size keeps the single wall field of the background mesh).
        for (name, script) in [("mesh_gmsh.py", SCRIPT), ("mesh_stl.py", crate::stl_mesh::SCRIPT), ("mesh_step.py", crate::step_mesh::SCRIPT)] {
            for key in ["\"Min\", 5", "SizeMin\", hf", "setAsBackgroundMesh(5)", "setAsBackgroundMesh(2)", "h0 * (1.0 - 1e-9)"] {
                if name == "mesh_step.py" && key == "h0 * (1.0 - 1e-9)" {
                    continue; // the caller decides (fin_refine is None at fin_size = wall_size)
                }
                assert!(script.contains(key), "{name}: {key}");
            }
            assert!(script.contains("+ 0.1 * "), "{name}: fin face test");
        }
        // The cad script: the fin faces are found among the wall faces of both the Euler and the RANS surface mesh.
        assert!(SCRIPT.contains("size_field(wall + cap, wall)") && SCRIPT.contains("size_field(wallsurf + cap, wallsurf)"));
        assert!(SCRIPT.contains("fin_refine = bool(fins) and hf < h0 * (1.0 - 1e-9)"));
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
        assert_eq!((a.domain.as_str(), d.domain.as_str(), d.y_mirror), ("half", "full_mirror", true));
        // A layout that is not symmetric about y = 0 gets the box farfield.
        let e = MeshParams::new(&CfdOptions { symmetry: false, fin_roll_deg: 10.0, ..o.clone() }, &g, Path::new("/a"), 4);
        assert_eq!((e.domain.as_str(), e.y_mirror), ("full_box", false));
        assert_ne!(mesh_hash(&d), mesh_hash(&e));
        // The STL kind, its union and the remesh switch are part of the hash.
        let mut s1 = a.clone();
        s1.kind = "stl".into();
        let mut s2 = s1.clone();
        s2.union_hash = "u".into();
        let mut s3 = s1.clone();
        s3.stl_remesh = "classify".into();
        assert!(mesh_hash(&s1) != mesh_hash(&a) && mesh_hash(&s1) != mesh_hash(&s2) && mesh_hash(&s1) != mesh_hash(&s3));
        // The chosen STL symmetry (domain, mirror steps) is part of the hash.
        let mut q1 = s1.clone();
        q1.domain = "stl_quarter_half".into();
        q1.symmetry = true;
        q1.z_mirror = true;
        let mut q2 = q1.clone();
        q2.symmetry = false;
        q2.y_mirror = true;
        q2.domain = "stl_quarter_full".into();
        let mut h1 = q1.clone();
        h1.z_mirror = false;
        h1.domain = "stl_half".into();
        let hashes = [&s1, &q1, &q2, &h1].map(mesh_hash);
        for i in 0..4 {
            for j in 0..i {
                assert_ne!(hashes[i], hashes[j], "{i} {j}");
            }
        }
        let mut g2 = g.clone();
        g2.fins.as_mut().unwrap().span *= 1.1;
        assert_ne!(mesh_hash(&a), mesh_hash(&MeshParams::new(&o, &g2, Path::new("/a"), 4)));
    }
}
