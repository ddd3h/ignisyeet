// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025-2026 西濱大将 (NISHIHAMA Daisuke)
//! CAD B-rep surface (`surface = "step"`): the user's STEP, IGES or BREP model with every detail (launch lugs, rail
//! buttons, camera pods, nozzle, asymmetric parts) as the exact, smooth CFD wall.
//!
//! # Pipeline
//!
//! 1. **Unit.** OpenCASCADE converts the lengths of a STEP or IGES file to metres from the unit the file declares
//!    (`Geometry.OCCTargetUnit = "M"`); a BREP file has no unit and is read as metres. `step_unit` overrides the
//!    declared unit: the shape is scaled by `step_unit / declared unit` ([`file_unit_metres`] reads the declaration).
//! 2. **Prepare** (`mesh_step.py`, mode `prepare`, cached by [`prepare_hash`]): import, sewing of a file without a solid,
//!    fusion of several solids into one (internal faces disappear; disjoint solids are kept and reported), the
//!    transform to the CFD frame that the STL gets ([`cfd_affine`], the map of [`crate::surface::to_cfd_frame`]), a
//!    tessellation of the shape, the faces with their type, area, centroid and normal, the smallest feature and the
//!    mirror checks about `y = 0` and `z = 0`. Shape healing (degenerated edges, small edges and faces) is applied only
//!    when the surface mesher fails on the imported faces: on a valid solid of revolution the healing removes the
//!    degenerated apex edge, after which gmsh cannot mesh the surface ("the 1D mesh seems not to be forming a closed
//!    loop").
//! 3. **Registration** ([`register`]): the STEP must describe the same rocket as `rocket.stl`, from which the geometry of
//!    the flight model comes. The STL (in the CFD frame) is sampled and every sample must lie on the STEP surface:
//!    the largest distance must not exceed `max(2 wall_size, 1 % of the length)` ([`registration_threshold`]). Samples
//!    inside the STEP solid and deeper than the detail threshold are buried STL shells (fins modelled as closed boxes
//!    reaching into the body) and are left out. The bounding box of the STL must lie in that of the STEP within 1 %
//!    of the length, the nose tips must coincide, and the STEP may be at most 25 % larger per axis (its own details
//!    only). Anything else fails with the measured offsets and the likely cause: the unit, a different orientation or
//!    a different revision of the CAD model. Regions of the STEP farther than [`detail_threshold`] from the STL are
//!    its own details (allowed, counted in the report).
//! 4. **Symmetry**: the mirror checks of the shape (interior surface nodes of the tessellation lie exactly on the
//!    B-rep; their mirror images must lie on a face of the shape within [`crate::symmetry::SYMMETRY_TOL`] of the length,
//!    measured with the exact closest point on the face) go through [`crate::symmetry::decide`] with
//!    `shape_symmetry`: both planes give the quarter (Euler with `z_mirror_mesh`), `y = 0` the half, otherwise the full
//!    domain. RANS never uses the quarter. A lug or a rail button off the pitch plane makes the shape asymmetric: the
//!    full domain costs about twice the cells of the half domain and solves `+-alpha` and `+-beta`.
//! 5. **Base**: Euler needs `base_treatment` as for the STL: `fairing` fuses an exact OpenCASCADE cone (to a point
//!    `tail_fairing` base radii behind the base, cut at 10 % of the base radius like the STL fairing) behind the largest
//!    flat base face, `flat` keeps the shape. The marker `base` holds the faces of the base rule ([`is_base_face`], the
//!    rule of the STL surface), `wall` every other face of the shape.
//! 6. **Mesh** (`mesh_step.py`, mode `mesh`): Euler cuts the solid from the farfield (half or quarter ball, or a box for
//!    the full domain) and fills it with tetrahedra graded from the wall: size `wall_size` on the wall (Distance +
//!    Threshold), curvature refinement ([`CURVATURE_ELEMENTS`] per circle) and the smallest size
//!    `clamp(smallest feature / 4, wall_size / 8, wall_size)` ([`min_size`]), so a 6 mm lug gets several cells across.
//!    RANS builds the prism layer with `bl_layer.py` (the layer law of [`crate::mesh::bl_params`]: thickness cap, base
//!    columns, tetrahedra from `top_size`) on the half or full wall; it is accepted only if it passes the validity check of `su2_writer.py`. CAD edges are not
//!    blunted (unlike the fin edges of the rebuilt geometry): a layer that stays invalid after the retry is an error
//!    that suggests Euler or slightly rounded edges in the CAD model.
//!
//! The mesh hash contains the bytes of the CAD file, every parameter of both modes and the scripts (`mesh_step.py`,
//! `bl_layer.py`, `su2_writer.py`).

use crate::case::hash_parts;
use crate::config::{BaseTreatment, CfdOptions, FlowModel, StepUnit};
use crate::fidelity::{self, Distances};
use crate::mesh::{self, run_python, BlParams, MeshResult, MeshStats};
use crate::symmetry::{self, PlaneCheck, SymmetryDecision};
use crate::tools;
use aero::atmosphere::Atmosphere;
use anyhow::{bail, Context, Result};
use geom::stl::Triangle;
use geom::{Geometry, Vec3};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

pub const SCRIPT: &str = include_str!("mesh_step.py");

/// Curvature refinement of the wall: elements per full circle (`Mesh.MeshSizeFromCurvature`).
pub const CURVATURE_ELEMENTS: usize = 24;
/// The smallest feature of the shape gets this many cells across (see [`min_size`]).
pub const FEATURE_CELLS: f64 = 4.0;
/// Lower bound of the smallest mesh size in wall sizes.
pub const MIN_SIZE_DIVISOR: f64 = 8.0;
/// Interior surface points mirrored per symmetry plane.
pub const SYMMETRY_POINTS: usize = 6000;
/// Sample points of the STL in the registration.
pub const REGISTRATION_SAMPLES: usize = 40_000;
/// STEP bounding box: tolerance (fraction of the length) and largest growth per axis over the STL's (its own details).
pub const BBOX_TOLERANCE: f64 = 0.01;
pub const BBOX_GROWTH: f64 = 0.25;

/// The CFD-frame transform of [`crate::surface::to_cfd_frame`] as an affine map `p = M v + t` of a point `v` in metres
/// (the STL coordinates after `stl_scale`): the 12 entries of the first three rows of the 4x4 matrix, row by row (the
/// form of `gmsh.model.occ.affineTransform`).
pub fn cfd_affine(geom: &Geometry) -> [f64; 12] {
    let [ex, ey, ez] = geom.frame.body_basis();
    let a = Vec3::new(geom.frame.axis_point[0], geom.frame.axis_point[1], geom.frame.axis_point[2]);
    let off = geom.frame.nose_offset;
    // x = -(ex . (v - a) + off), y = ey . (v - a), z = -(ez . (v - a)).
    let rows = [(ex * -1.0, ex.dot(a) - off), (ey, -ey.dot(a)), (ez * -1.0, ez.dot(a))];
    let mut m = [0.0; 12];
    for (i, (r, t)) in rows.iter().enumerate() {
        m[4 * i..4 * i + 4].copy_from_slice(&[r.x, r.y, r.z, *t]);
    }
    m
}

/// The base rule of the STL surface ([`crate::surface::split_base`]) for a face of the B-rep: base when its centroid
/// lies aft of the base plane `x = length` by more than `tol` (tail fairing, protruding nozzle), or within `tol` of it
/// and the face is planar with `|n_x| > 0.9` (flat base, fin trailing edges flush with the base). `mesh_step.py`
/// applies the same rule (`is_base`).
pub fn is_base_face(centroid_x: f64, planar: bool, normal_x: f64, length: f64, tol: f64) -> bool {
    centroid_x > length + tol || ((centroid_x - length).abs() <= tol && planar && normal_x.abs() > 0.9)
}

/// The smallest mesh size: a quarter of the smallest feature (so it gets [`FEATURE_CELLS`] cells across), at most
/// `wall_size` and at least `wall_size / MIN_SIZE_DIVISOR`.
pub fn min_size(smallest_feature: f64, wall_size: f64) -> f64 {
    let f = if smallest_feature.is_finite() && smallest_feature > 0.0 { smallest_feature / FEATURE_CELLS } else { wall_size };
    f.clamp(wall_size / MIN_SIZE_DIVISOR, wall_size)
}

/// Largest allowed distance of the STL from the STEP surface: `max(2 wall_size, 1 % of the length)`.
pub fn registration_threshold(wall_size: f64, length: f64) -> f64 {
    (2.0 * wall_size).max(0.01 * length)
}

/// Distance from the STL beyond which a part of the STEP surface counts as its own detail: `max(wall_size, 0.2 % of the
/// length)`; also the depth below which STL samples inside the STEP solid count as buried shells.
pub fn detail_threshold(wall_size: f64, length: f64) -> f64 {
    wall_size.max(0.002 * length)
}

/// Format of a CAD file from its extension (`step`, `iges` or `brep`), `None` for anything else.
pub fn cad_format(path: &Path) -> Option<&'static str> {
    match path.extension()?.to_str()?.to_ascii_lowercase().as_str() {
        "step" | "stp" => Some("step"),
        "iges" | "igs" => Some("iges"),
        "brep" | "brp" => Some("brep"),
        _ => None,
    }
}

/// Metres per length unit declared by a STEP (`LENGTH_UNIT`: an `SI_UNIT` with its prefix or a
/// `CONVERSION_BASED_UNIT` such as `'INCH'`) or IGES file (units flag, parameter 14 of the global section). `None` when
/// the file declares none (always for BREP).
pub fn file_unit_metres(format: &str, bytes: &[u8]) -> Option<f64> {
    let text = String::from_utf8_lossy(bytes);
    match format {
        "step" => step_unit_metres(&text),
        "iges" => iges_unit_metres(&text),
        _ => None,
    }
}

fn step_unit_metres(text: &str) -> Option<f64> {
    let mut si = None;
    for ent in text.split(';') {
        let e: String = ent.chars().filter(|c| !c.is_whitespace()).collect::<String>().to_ascii_uppercase();
        if !e.contains("LENGTH_UNIT") {
            continue;
        }
        if let Some(i) = e.find("CONVERSION_BASED_UNIT('") {
            let name = e[i + 23..].split('\'').next().unwrap_or("");
            let m = match name {
                "INCH" | "IN" => Some(0.0254),
                "FOOT" | "FT" => Some(0.3048),
                "MILLIMETRE" | "MILLIMETER" | "MM" => Some(0.001),
                "CENTIMETRE" | "CENTIMETER" | "CM" => Some(0.01),
                "METRE" | "METER" | "M" => Some(1.0),
                _ => None,
            };
            if m.is_some() {
                return m;
            }
        } else if let Some(i) = e.find("SI_UNIT(") {
            let args = e[i + 8..].split(')').next().unwrap_or("");
            let mut it = args.split(',');
            let (prefix, unit) = (it.next().unwrap_or(""), it.next().unwrap_or(""));
            if unit != ".METRE." {
                continue;
            }
            si = si.or(match prefix {
                "$" => Some(1.0),
                ".KILO." => Some(1e3),
                ".DECI." => Some(0.1),
                ".CENTI." => Some(0.01),
                ".MILLI." => Some(1e-3),
                ".MICRO." => Some(1e-6),
                _ => None,
            });
        }
    }
    si
}

fn iges_unit_metres(text: &str) -> Option<f64> {
    // Global section: the lines with `G` in column 73, columns 1..72 concatenated.
    let g: String = text.lines().filter(|l| l.len() >= 73 && l.as_bytes()[72] == b'G').map(|l| &l[..72]).collect();
    let b = g.as_bytes();
    // Parameter delimiter: parameter 1 (`1Hx`), default `,`.
    let delim = if b.len() >= 3 && b[0] == b'1' && b[1] == b'H' { b[2] } else { b',' };
    let mut params: Vec<String> = Vec::new();
    let mut i = 0;
    let mut cur = String::new();
    while i < b.len() {
        let c = b[i];
        // Hollerith string nH...: copy n characters verbatim.
        if c.is_ascii_digit() && cur.chars().all(|x| x.is_ascii_digit()) {
            let mut j = i;
            while j < b.len() && b[j].is_ascii_digit() {
                j += 1;
            }
            if j < b.len() && b[j] == b'H' && cur.is_empty() {
                let n: usize = std::str::from_utf8(&b[i..j]).ok()?.parse().ok()?;
                let end = (j + 1 + n).min(b.len());
                cur.push_str(std::str::from_utf8(&b[j + 1..end]).unwrap_or(""));
                i = end;
                continue;
            }
        }
        if c == delim || c == b';' {
            params.push(std::mem::take(&mut cur));
            if c == b';' {
                break;
            }
        } else {
            cur.push(c as char);
        }
        i += 1;
    }
    let flag: i32 = params.get(13)?.trim().parse().ok()?;
    match flag {
        1 => Some(0.0254),
        2 => Some(0.001),
        4 => Some(0.3048),
        5 => Some(1609.344),
        6 => Some(1.0),
        7 => Some(1000.0),
        8 => Some(2.54e-5),
        9 => Some(1e-6),
        10 => Some(0.01),
        11 => Some(2.54e-8),
        _ => None,
    }
}

/// Scale applied after the import (see the module documentation): 1 for `auto`, else `step_unit / declared unit` (the
/// declared unit is 1 m for a BREP file).
pub fn unit_scale(unit: StepUnit, format: &str, declared: Option<f64>, file: &Path) -> Result<f64> {
    let Some(want) = unit.metres() else { return Ok(1.0) };
    match (format, declared) {
        (_, Some(d)) => Ok(want / d),
        ("brep", None) => Ok(want),
        _ => bail!(
            "aero.cfd.step_unit = \"{}\": the length unit of {} cannot be read from the file, so it cannot be overridden; use step_unit = \"auto\"",
            format!("{unit:?}").to_lowercase(),
            file.display()
        ),
    }
}

// ---- registration -------------------------------------------------------------------------------------------------

fn bbox(tris: &[Triangle]) -> [f64; 6] {
    let mut b = [f64::INFINITY, f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY];
    for v in tris.iter().flatten() {
        for (k, c) in [v.x, v.y, v.z].into_iter().enumerate() {
            b[k] = b[k].min(c);
            b[k + 3] = b[k + 3].max(c);
        }
    }
    b
}

/// Ray-parity inside test of points against a closed surface (rays along +x, triangles binned in the y-z plane).
struct InsideTest<'a> {
    tris: &'a [Triangle],
    lo: [f64; 2],
    cell: f64,
    n: [usize; 2],
    bins: Vec<Vec<u32>>,
    jitter: [f64; 2],
}

impl<'a> InsideTest<'a> {
    fn new(tris: &'a [Triangle]) -> Self {
        let b = bbox(tris);
        let ext = (b[4] - b[1]).max(b[5] - b[2]).max(1e-12);
        let nc = ((tris.len() as f64 / 4.0).sqrt().ceil() as usize).clamp(1, 512);
        let cell = ext / nc as f64 * 1.0001;
        let n = [(((b[4] - b[1]) / cell).ceil() as usize).max(1), (((b[5] - b[2]) / cell).ceil() as usize).max(1)];
        let mut bins = vec![Vec::new(); n[0] * n[1]];
        let idx = |v: f64, lo: f64, n: usize| (((v - lo) / cell).floor().max(0.0) as usize).min(n - 1);
        for (k, t) in tris.iter().enumerate() {
            let (ylo, yhi) = (t[0].y.min(t[1].y).min(t[2].y), t[0].y.max(t[1].y).max(t[2].y));
            let (zlo, zhi) = (t[0].z.min(t[1].z).min(t[2].z), t[0].z.max(t[1].z).max(t[2].z));
            for i in idx(ylo, b[1], n[0])..=idx(yhi, b[1], n[0]) {
                for j in idx(zlo, b[2], n[1])..=idx(zhi, b[2], n[1]) {
                    bins[i * n[1] + j].push(k as u32);
                }
            }
        }
        // An irrational offset keeps the rays off the edges and vertices of a regular tessellation.
        let jitter = [1.234_567e-7 * ext, 2.718_281e-7 * ext];
        Self { tris, lo: [b[1], b[2]], cell, n, bins, jitter }
    }

    fn inside(&self, p: Vec3) -> bool {
        let (y, z) = (p.y + self.jitter[0], p.z + self.jitter[1]);
        let (fi, fj) = ((y - self.lo[0]) / self.cell, (z - self.lo[1]) / self.cell);
        if fi < 0.0 || fj < 0.0 || fi >= self.n[0] as f64 || fj >= self.n[1] as f64 {
            return false;
        }
        let mut crossings = 0;
        for &k in &self.bins[fi as usize * self.n[1] + fj as usize] {
            let t = &self.tris[k as usize];
            // Barycentric coordinates of (y, z) in the projected triangle.
            let (y0, z0, y1, z1, y2, z2) = (t[0].y, t[0].z, t[1].y, t[1].z, t[2].y, t[2].z);
            let d = (y1 - y0) * (z2 - z0) - (y2 - y0) * (z1 - z0);
            if d.abs() < 1e-300 {
                continue;
            }
            let a = ((y1 - y) * (z2 - z) - (y2 - y) * (z1 - z)) / d;
            let b = ((y2 - y) * (z0 - z) - (y0 - y) * (z2 - z)) / d;
            let c = 1.0 - a - b;
            if a < 0.0 || b < 0.0 || c < 0.0 {
                continue;
            }
            if a * t[0].x + b * t[1].x + c * t[2].x > p.x {
                crossings += 1;
            }
        }
        crossings % 2 == 1
    }
}

/// Comparison of the STEP tessellation with the STL, both in the CFD frame (see the module documentation).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Registration {
    /// Allowed largest distance of the STL from the STEP surface [m] ([`registration_threshold`]).
    pub threshold: f64,
    /// [`detail_threshold`] [m].
    pub detail_threshold: f64,
    /// STL samples -> STEP surface, without the buried ones.
    pub stl_to_step: Distances,
    /// Fraction of the STL samples inside the STEP solid and deeper than the detail threshold (buried shells).
    pub stl_buried_fraction: f64,
    /// STEP triangle centroids -> STL surface.
    pub step_to_stl: Distances,
    /// Connected regions of the STEP surface farther than the detail threshold from the STL (its own details) and
    /// their area [m^2].
    pub step_only_regions: usize,
    pub step_only_area: f64,
    /// Bounding boxes `[xmin, ymin, zmin, xmax, ymax, zmax]` [m] and `STEP - STL`.
    pub stl_bbox: [f64; 6],
    pub step_bbox: [f64; 6],
    pub bbox_offsets: [f64; 6],
    /// Why the STEP does not match (empty when it does).
    pub problems: Vec<String>,
    /// Likely cause of a mismatch.
    pub hint: Option<String>,
}

impl Registration {
    pub fn ok(&self) -> bool {
        self.problems.is_empty()
    }

    /// Text for `cfd-check --mesh` and the error.
    pub fn summary(&self) -> String {
        let b = |x: &[f64; 6]| format!("x {:.4}..{:.4} y {:.4}..{:.4} z {:.4}..{:.4}", x[0], x[3], x[1], x[4], x[2], x[5]);
        if self.stl_to_step.samples == 0 {
            let o = &self.bbox_offsets;
            return format!(
                "registration  distances not measured (the bounding boxes differ)\n  bounding box [m]: STL  {}\n                    STEP {}\n  \
                 offsets STEP - STL [m]: min ({:+.4}, {:+.4}, {:+.4}), max ({:+.4}, {:+.4}, {:+.4})",
                b(&self.stl_bbox),
                b(&self.step_bbox),
                o[0],
                o[1],
                o[2],
                o[3],
                o[4],
                o[5]
            );
        }
        format!(
            "registration  STL -> STEP surface: max {:.3e} m at ({:.3}, {:.3}, {:.3}), p99 {:.3e} m, rms {:.3e} m ({} samples, allowed {:.3e} m; {:.1} % of the samples buried inside the STEP solid)\n  \
             STEP -> STL: max {:.3e} m, p99 {:.3e} m; {} STEP-only region(s) farther than {:.3e} m from the STL, {:.3e} m^2\n  \
             bounding box [m]: STL  {}\n                    STEP {}\n  \
             offsets STEP - STL [m]: min ({:+.4}, {:+.4}, {:+.4}), max ({:+.4}, {:+.4}, {:+.4})",
            self.stl_to_step.max,
            self.stl_to_step.max_at[0],
            self.stl_to_step.max_at[1],
            self.stl_to_step.max_at[2],
            self.stl_to_step.p99,
            self.stl_to_step.rms,
            self.stl_to_step.samples,
            self.threshold,
            100.0 * self.stl_buried_fraction,
            self.step_to_stl.max,
            self.step_to_stl.p99,
            self.step_only_regions,
            self.detail_threshold,
            self.step_only_area,
            b(&self.stl_bbox),
            b(&self.step_bbox),
            self.bbox_offsets[0],
            self.bbox_offsets[1],
            self.bbox_offsets[2],
            self.bbox_offsets[3],
            self.bbox_offsets[4],
            self.bbox_offsets[5],
        )
    }

    /// Fails unless the STEP matches the STL.
    pub fn check(&self, file: &Path) -> Result<()> {
        if self.ok() {
            return Ok(());
        }
        bail!(
            "aero.cfd.surface = \"step\": {} does not describe the same rocket as rocket.stl (compared in the CFD frame, after the transform of the STL):\n  - {}\n  {}\n  likely cause: {}",
            file.display(),
            self.problems.join("\n  - "),
            self.summary(),
            self.hint.as_deref().unwrap_or("a different revision of the CAD model")
        )
    }
}

fn tri_area(t: &Triangle) -> f64 {
    0.5 * (t[1] - t[0]).cross(t[2] - t[0]).norm()
}

/// Compares the STEP tessellation `step` with the STL `stl` (both in the CFD frame).
pub fn register(stl: &[Triangle], step: &[Triangle], wall_size: f64, length: f64) -> Registration {
    let threshold = registration_threshold(wall_size, length);
    let detail = detail_threshold(wall_size, length);
    let mut r = Registration { threshold, detail_threshold: detail, ..Default::default() };
    r.stl_bbox = bbox(stl);
    r.step_bbox = bbox(step);
    for k in 0..6 {
        r.bbox_offsets[k] = r.step_bbox[k] - r.stl_bbox[k];
    }
    if stl.is_empty() || step.is_empty() {
        r.problems.push("an empty surface".into());
        return r;
    }
    // Bounding boxes first: a gross mismatch (unit, orientation) fails without the distance search, which would crawl
    // over a surface a thousand times too large.
    let m = BBOX_TOLERANCE * length;
    let axes = ["x", "y", "z"];
    if r.bbox_offsets[0].abs() > m {
        r.problems.push(format!("the nose tips differ: STEP x min {:.4} m, STL x min {:.4} m (tolerance {:.3e} m)", r.step_bbox[0], r.stl_bbox[0], m));
    }
    for (k, axis) in axes.iter().enumerate() {
        if r.step_bbox[k] > r.stl_bbox[k] + m || r.step_bbox[k + 3] < r.stl_bbox[k + 3] - m {
            r.problems.push(format!(
                "the STL extends beyond the STEP along {}: STL {:.4}..{:.4} m, STEP {:.4}..{:.4} m (tolerance {:.3e} m)",
                axis,
                r.stl_bbox[k],
                r.stl_bbox[k + 3],
                r.step_bbox[k],
                r.step_bbox[k + 3],
                m
            ));
        }
        let (es, et) = (r.stl_bbox[k + 3] - r.stl_bbox[k], r.step_bbox[k + 3] - r.step_bbox[k]);
        if et > (1.0 + BBOX_GROWTH) * es + m {
            r.problems.push(format!("the STEP is {:.3} times as large as the STL along {} ({:.4} m vs {:.4} m)", et / es.max(1e-300), axis, et, es));
        }
    }
    if !r.ok() {
        r.stl_to_step.max = f64::INFINITY;
        r.step_to_stl.max = f64::INFINITY;
        r.hint = Some(mismatch_hint(&r.stl_bbox, &r.step_bbox));
        return r;
    }
    // STL -> STEP, without the buried samples.
    let inside = InsideTest::new(step);
    let pts = fidelity::sample_points(stl, REGISTRATION_SAMPLES);
    let d = fidelity::point_distances(&pts, step);
    let total = d.len();
    let (kept, buried): (Vec<_>, Vec<_>) = d.into_iter().partition(|&(dist, p)| dist <= detail || !inside.inside(p));
    r.stl_buried_fraction = buried.len() as f64 / total.max(1) as f64;
    r.stl_to_step = fidelity::distance_stats(kept);
    // STEP -> STL at the triangle centroids; the STEP-only regions.
    let cen: Vec<Vec3> = step.iter().map(|t| (t[0] + t[1] + t[2]) / 3.0).collect();
    let back = fidelity::point_distances(&cen, stl);
    let only: Vec<usize> = (0..step.len()).filter(|&i| back[i].0 > detail).collect();
    r.step_only_area = only.iter().map(|&i| tri_area(&step[i])).sum::<f64>() + 0.0;
    r.step_only_regions = regions(step, &only, 1e-7 * length);
    r.step_to_stl = fidelity::distance_stats(back);
    if r.stl_to_step.max > threshold {
        r.problems.push(format!(
            "the STL lies up to {:.3e} m from the STEP surface (allowed {:.3e} m = max(2 wall_size, 1 % of the length)), at ({:.4}, {:.4}, {:.4})",
            r.stl_to_step.max, threshold, r.stl_to_step.max_at[0], r.stl_to_step.max_at[1], r.stl_to_step.max_at[2]
        ));
    }
    if r.stl_buried_fraction > 0.25 {
        r.problems.push(format!("{:.0} % of the STL lies inside the STEP solid (the STEP is larger than the STL)", 100.0 * r.stl_buried_fraction));
    }
    if !r.ok() {
        r.hint = Some(mismatch_hint(&r.stl_bbox, &r.step_bbox));
    }
    r
}

/// Likely cause of a mismatch from the bounding boxes: a unit (the length ratio is a unit ratio), a different
/// orientation (the extents match in another axis order, or the shape lies on the other side of the nose) or a
/// different revision.
fn mismatch_hint(stl: &[f64; 6], step: &[f64; 6]) -> String {
    let ext = |b: &[f64; 6]| [b[3] - b[0], b[4] - b[1], b[5] - b[2]];
    let (es, et) = (ext(stl), ext(step));
    let longest = |e: [f64; 3]| e.iter().copied().fold(0.0, f64::max);
    let ratio = longest(et) / longest(es).max(1e-300);
    for (r, what) in [
        (1000.0, "the STEP is in millimetres but read as metres"),
        (0.001, "the STEP is in metres but read as millimetres"),
        (25.4, "inch vs millimetre"),
        (1.0 / 25.4, "millimetre vs inch"),
        (0.0254, "inch vs metre"),
        (1.0 / 0.0254, "metre vs inch"),
        (10.0, "centimetre vs millimetre"),
        (0.1, "millimetre vs centimetre"),
        (100.0, "metre vs centimetre"),
        (0.01, "centimetre vs metre"),
        (304.8, "foot vs millimetre"),
    ] {
        if (ratio / r - 1.0).abs() < 0.05 {
            return format!(
                "units: the STEP is {ratio:.4} times the size of the STL ({what}); check step_unit (\"auto\" reads the unit the file declares) and rocket.stl_scale"
            );
        }
    }
    if (ratio - 1.0).abs() < 0.05 {
        return "a different orientation or position: the STEP must use the coordinate system of the STL (the same nose direction and origin; \
                rocket.nose_direction applies to both), or a different revision of the CAD model"
            .into();
    }
    format!("a different revision of the CAD model (the STEP is {ratio:.3} times the size of the STL)")
}

/// Number of connected regions of the triangles `sel` of `tris` (vertices welded within `tol`).
fn regions(tris: &[Triangle], sel: &[usize], tol: f64) -> usize {
    if sel.is_empty() {
        return 0;
    }
    let key = |v: Vec3| ((v.x / tol).round() as i64, (v.y / tol).round() as i64, (v.z / tol).round() as i64);
    let mut parent: Vec<usize> = (0..sel.len()).collect();
    fn find(p: &mut [usize], mut x: usize) -> usize {
        while p[x] != x {
            p[x] = p[p[x]];
            x = p[x];
        }
        x
    }
    let mut owner: HashMap<(i64, i64, i64), usize> = HashMap::new();
    for (k, &i) in sel.iter().enumerate() {
        for v in tris[i] {
            match owner.get(&key(v)) {
                Some(&o) => {
                    let (a, b) = (find(&mut parent, o), find(&mut parent, k));
                    if a != b {
                        parent[a.max(b)] = a.min(b);
                    }
                }
                None => {
                    owner.insert(key(v), k);
                }
            }
        }
    }
    (0..sel.len()).filter(|&k| find(&mut parent, k) == k).count()
}

// ---- prepare and mesh ---------------------------------------------------------------------------------------------

/// `fairing` in `params.json`: Euler tail fairing behind the base.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FairingParams {
    /// Cone length in base radii.
    pub tail_fairing: f64,
}

/// Everything `mesh_step.py` reads (`params.json`); the prepare mode uses the first group only.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepParams {
    /// `prepare` or `mesh`.
    pub mode: String,
    pub step_file: String,
    pub unit_scale: f64,
    pub affine: [f64; 12],
    /// Base plane `x = length` of the geometry extracted from the STL.
    pub length: f64,
    pub base_tol: f64,
    pub heal_tol: f64,
    pub tess_size: f64,
    pub sym_tol: f64,
    pub sym_points: usize,
    pub fairing: Option<FairingParams>,
    pub prepared_brep: String,
    pub tess_stl: String,
    pub report_json: String,
    // mesh
    pub farfield_radius: f64,
    pub center_x: f64,
    pub wall_size: f64,
    pub max_size: f64,
    pub growth: f64,
    pub min_size: f64,
    pub curvature_elements: usize,
    /// `full`, `half` (y >= 0) or `quarter` (y >= 0, z >= 0, mirrored about z = 0).
    pub meshed: String,
    /// The half / quarter is mirrored about y = 0 to the full domain (no `symmetry` marker).
    pub y_mirror: bool,
    /// gmsh 3D algorithm: 10 = HXT, 1 = Delaunay (retry).
    pub algorithm3d: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bl: Option<BlParams>,
    pub out_su2: String,
    pub stats_json: String,
    pub threads: usize,
}

/// The parameters of the prepare mode that change its result.
#[derive(Serialize)]
struct PrepareKey<'a> {
    unit_scale: f64,
    affine: &'a [f64; 12],
    length: f64,
    base_tol: f64,
    heal_tol: f64,
    tess_size: f64,
    sym_tol: f64,
    sym_points: usize,
    fairing: &'a Option<FairingParams>,
}

/// Cache key of the prepare mode: the file bytes, the prepare parameters and the script.
pub fn prepare_hash(p: &StepParams, file_bytes: &[u8]) -> String {
    let k = PrepareKey {
        unit_scale: p.unit_scale,
        affine: &p.affine,
        length: p.length,
        base_tol: p.base_tol,
        heal_tol: p.heal_tol,
        tess_size: p.tess_size,
        sym_tol: p.sym_tol,
        sym_points: p.sym_points,
        fairing: &p.fairing,
    };
    hash_parts(&[file_bytes, serde_json::to_string(&k).unwrap_or_default().as_bytes(), SCRIPT.as_bytes()])
}

/// Mesh hash: the file bytes, every parameter (paths, mode and thread count excluded) and the scripts.
pub fn step_mesh_hash(p: &StepParams, file_bytes: &[u8]) -> String {
    let mut q = p.clone();
    for s in [&mut q.mode, &mut q.step_file, &mut q.prepared_brep, &mut q.tess_stl, &mut q.report_json, &mut q.out_su2, &mut q.stats_json] {
        s.clear();
    }
    q.threads = 0;
    hash_parts(&[file_bytes, serde_json::to_string(&q).unwrap_or_default().as_bytes(), SCRIPT.as_bytes(), mesh::BL_LAYER.as_bytes(), mesh::WRITER.as_bytes()])
}

/// One face of the B-rep (`step_report.json`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FaceInfo {
    pub tag: i64,
    #[serde(rename = "type")]
    pub kind: String,
    pub area: f64,
    pub centroid: [f64; 3],
    pub normal: [f64; 3],
    pub planar: bool,
    /// `2 area / perimeter` [m] (the width of a strip, the radius of a disc).
    pub width: f64,
    pub bbox: [f64; 6],
    pub base: bool,
}

/// `step_report.json` of the prepare mode.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepReport {
    pub solids_in_file: usize,
    pub faces_in_file: usize,
    pub notes: Vec<String>,
    pub solids: usize,
    pub volume: f64,
    pub area: f64,
    pub bbox: [f64; 6],
    pub faces: Vec<FaceInfo>,
    pub face_triangles: Vec<usize>,
    pub smallest_face_width: f64,
    pub smallest_face: Option<i64>,
    pub smallest_edge: f64,
    pub symmetry_y: PlaneCheck,
    pub symmetry_z: PlaneCheck,
    pub fairing: Option<serde_json::Value>,
    #[serde(default)]
    pub seconds: f64,
}

/// What the STEP pipeline derived (stored in `cfd_report.json` and shown by `cfd-check --mesh`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepMeshInfo {
    pub file: String,
    /// `step`, `iges` or `brep`.
    pub format: String,
    /// Metres per unit declared by the file (`None`: none declared).
    pub file_unit_m: Option<f64>,
    /// `step_unit` as configured and the scale it implies.
    pub step_unit: String,
    pub unit_scale: f64,
    pub solids_in_file: usize,
    pub solids: usize,
    pub faces: usize,
    pub base_faces: usize,
    pub volume: f64,
    pub area: f64,
    /// Import notes (sewing, healing, fusion).
    pub import_notes: Vec<String>,
    pub registration: Registration,
    pub symmetry: SymmetryDecision,
    /// `full`, `half` (SU2 half domain), `half_mirrored`, `quarter_half` (quarter mirrored about z = 0) or
    /// `quarter_full`.
    pub domain: String,
    /// `fairing`, `flat` or `none` (RANS).
    pub base_treatment: String,
    pub fairing: Option<serde_json::Value>,
    /// Smallest face width [m] and the smallest mesh size it gives ([`min_size`]).
    pub smallest_feature: f64,
    pub min_size: f64,
    pub warnings: Vec<String>,
}

impl StepMeshInfo {
    /// Text for `cfd-check --mesh`.
    pub fn summary(&self) -> String {
        let unit = match self.file_unit_m {
            Some(u) => format!("declared {u} m"),
            None => "none declared".into(),
        };
        let mut s = format!(
            "  file        {} ({}, unit {unit}, step_unit {}, scale {})\n  shape       {} solid(s) in the file, {} after fusion; {} faces ({} base), area {:.5} m^2, volume {:.6} m^3\n  details     {} STEP-only region(s), {:.3e} m^2; smallest face width {:.3e} m -> smallest mesh size {:.3e} m\n  domain      {}: {}\n  base        {}{}\n  {}",
            self.file,
            self.format,
            self.step_unit,
            self.unit_scale,
            self.solids_in_file,
            self.solids,
            self.faces,
            self.base_faces,
            self.area,
            self.volume,
            self.registration.step_only_regions,
            self.registration.step_only_area,
            self.smallest_feature,
            self.min_size,
            self.domain,
            self.symmetry.summary(),
            self.base_treatment,
            self.fairing.as_ref().map_or(String::new(), |f| format!(" {f}")),
            self.registration.summary()
        );
        for n in &self.import_notes {
            s += &format!("\n  note: {n}");
        }
        for w in &self.warnings {
            s += &format!("\n  warning: {w}");
        }
        s
    }
}

#[derive(Serialize, Deserialize)]
struct Stamp {
    hash: String,
    stats: MeshStats,
}

/// The base treatment in effect and the warnings it implies.
pub fn base_treatment_notes(opt: &CfdOptions) -> (&'static str, Vec<String>) {
    if opt.model != FlowModel::Euler {
        return ("none", Vec::new());
    }
    match opt.base_treatment {
        Some(BaseTreatment::Fairing) => (
            "fairing",
            vec![format!(
                "base_treatment = fairing: the CAD shape is MODIFIED, an exact cone of {} base radii (cut at 10 % of the base radius) is fused with the solid behind its flat base; \
                 the base is not integrated, the base drag comes from the empirical base term",
                opt.tail_fairing
            )],
        ),
        _ => (
            "flat",
            vec!["base_treatment = flat: the flat base is kept; the inviscid flow behind it is ill-posed, so the Euler solution is likely not to converge (cases that do not converge fail); the base is not integrated, the base drag comes from the empirical base term".to_string()],
        ),
    }
}

/// The parameters of both modes for `opt` (the mesh fields of the domain are filled by [`ensure_mesh_step`]).
fn base_params(opt: &CfdOptions, geom: &Geometry, dir: &Path, threads: usize, unit_scale: f64, treatment: &str) -> StepParams {
    let l = geom.length;
    let r = opt.farfield * l;
    let p = |n: &str| dir.join(n).display().to_string();
    StepParams {
        mode: "prepare".into(),
        step_file: std::path::absolute(&opt.step_file).unwrap_or_else(|_| opt.step_file.clone()).display().to_string(),
        unit_scale,
        affine: cfd_affine(geom),
        length: l,
        base_tol: 1e-4 * l,
        heal_tol: 1e-6 * l,
        tess_size: (2.0 * opt.wall_size).clamp(l / 1000.0, l / 300.0),
        sym_tol: symmetry::SYMMETRY_TOL * l,
        sym_points: SYMMETRY_POINTS,
        fairing: (treatment == "fairing").then_some(FairingParams { tail_fairing: opt.tail_fairing }),
        prepared_brep: p("step_prepared.brep"),
        tess_stl: p("step_surface.stl"),
        report_json: p("step_report.json"),
        farfield_radius: r,
        center_x: 0.5 * l,
        wall_size: opt.wall_size,
        max_size: r / 10.0,
        growth: 1.2,
        min_size: opt.wall_size / MIN_SIZE_DIVISOR,
        curvature_elements: CURVATURE_ELEMENTS,
        meshed: "full".into(),
        y_mirror: false,
        algorithm3d: 10,
        bl: None,
        out_su2: p("mesh.su2"),
        stats_json: p("mesh_stats.json"),
        threads: threads.max(1),
    }
}

/// Reads the CAD file and works out its format, declared unit and unit scale.
fn read_file(opt: &CfdOptions) -> Result<(Vec<u8>, &'static str, Option<f64>, f64)> {
    let path = &opt.step_file;
    let format = cad_format(path).with_context(|| format!("aero.cfd.step_file {}: not a STEP, IGES or BREP file", path.display()))?;
    let bytes = std::fs::read(path).with_context(|| format!("cannot read the CAD file aero.cfd.step_file = {}", path.display()))?;
    let declared = file_unit_metres(format, &bytes);
    let scale = unit_scale(opt.step_unit, format, declared, path)?;
    Ok((bytes, format, declared, scale))
}

fn python(opt: &CfdOptions) -> Result<(std::path::PathBuf, Option<std::path::PathBuf>)> {
    let tools = tools::check_tools(opt, &[]);
    if !tools.gmsh.ok() {
        bail!("{}", crate::not_installed_message(&tools, opt));
    }
    Ok((tools.gmsh.path.clone().context("no python with gmsh")?, tools.prefix.clone()))
}

/// Runs the prepare mode unless its outputs for this hash exist; returns the report and the tessellation.
fn ensure_prepared(opt: &CfdOptions, dir: &Path, params: &StepParams, bytes: &[u8]) -> Result<(StepReport, Vec<Triangle>)> {
    let hash = prepare_hash(params, bytes);
    let stamp = dir.join("step_prepare.json");
    let have = [&params.prepared_brep, &params.tess_stl, &params.report_json].iter().all(|f| Path::new(f.as_str()).exists());
    if !(have && std::fs::read_to_string(&stamp).is_ok_and(|t| t.trim() == hash)) {
        let _ = std::fs::remove_file(&stamp);
        let (py, prefix) = python(opt)?;
        std::fs::write(dir.join("mesh_step.py"), SCRIPT)?;
        std::fs::write(dir.join("su2_writer.py"), mesh::WRITER)?;
        std::fs::write(dir.join("bl_layer.py"), mesh::BL_LAYER)?;
        std::fs::write(dir.join("prepare_params.json"), serde_json::to_string_pretty(&StepParams { mode: "prepare".into(), ..params.clone() })?)?;
        run_python(&py, prefix.as_deref(), dir, "mesh_step.py", "prepare_params.json", "prepare.log", "PREPARE_OK", "the import of the CAD file")?;
        std::fs::write(&stamp, &hash)?;
    }
    let report: StepReport = serde_json::from_str(&std::fs::read_to_string(&params.report_json)?).with_context(|| format!("bad {}", params.report_json))?;
    let tess = geom::stl::read_stl(Path::new(&params.tess_stl))?;
    Ok((report, tess))
}

/// Imports, registers and checks the symmetry of the CAD file without meshing (cached below `<workdir>/mesh/`).
/// `raw` is the STL in metres. Returns the registration (not checked) and the symmetry decision.
pub fn check_step(opt: &CfdOptions, geom: &Geometry, raw: &[Triangle], workdir: &Path) -> Result<(Registration, SymmetryDecision)> {
    opt.validate()?;
    let dir = workdir.join("mesh");
    std::fs::create_dir_all(&dir).with_context(|| format!("cannot create {}", dir.display()))?;
    let (bytes, _, _, scale) = read_file(opt)?;
    let params = base_params(opt, geom, &dir, 1, scale, base_treatment_notes(opt).0);
    let (report, tess) = ensure_prepared(opt, &dir, &params, &bytes)?;
    let reg = register(&crate::surface::to_cfd_frame(raw, geom), &tess, opt.wall_size, geom.length);
    let decision = decide(opt, &report)?;
    Ok((reg, decision))
}

/// The options the symmetry decision is made with: no quarter mesh for RANS.
fn symmetry_options(opt: &CfdOptions) -> CfdOptions {
    CfdOptions { z_mirror_mesh: opt.z_mirror_mesh && opt.model == FlowModel::Euler, ..opt.clone() }
}

fn decide(opt: &CfdOptions, report: &StepReport) -> Result<SymmetryDecision> {
    symmetry::decide(&symmetry_options(opt), &report.symmetry_y, &report.symmetry_z)
        .map_err(|e| anyhow::anyhow!("{}", e.to_string().replace("the united STL is not", "the CAD shape is not")))
}

/// Meshes the CAD file (see the module documentation). `raw` is the STL in metres (the registration reference) and
/// `atm` the freestream of the cases (RANS prism layer). Returns the mesh and the options with the symmetry of the shape
/// resolved: the case matrix and the SU2 configuration must be made from them.
pub fn ensure_mesh_step(
    opt: &CfdOptions,
    geom: &Geometry,
    raw: &[Triangle],
    atm: &Atmosphere,
    workdir: &Path,
    threads: usize,
) -> Result<(MeshResult, CfdOptions)> {
    opt.validate()?;
    let dir = workdir.join("mesh");
    std::fs::create_dir_all(&dir).with_context(|| format!("cannot create {}", dir.display()))?;
    let (treatment, mut warnings) = base_treatment_notes(opt);
    for w in &warnings {
        eprintln!("warning: aero.cfd: {w}");
    }
    let (bytes, format, declared, scale) = read_file(opt)?;
    let mut params = base_params(opt, geom, &dir, threads, scale, treatment);
    let (report, tess) = ensure_prepared(opt, &dir, &params, &bytes)?;
    // Registration against the STL the flight model was extracted from.
    let reg = register(&crate::surface::to_cfd_frame(raw, geom), &tess, opt.wall_size, geom.length);
    reg.check(&opt.step_file)?;
    if let Some(e) = report.fairing.as_ref().and_then(|f| f.get("error")).and_then(|e| e.as_str()) {
        bail!("aero.cfd.surface = \"step\": {e}");
    }
    let base_faces = report.faces.iter().filter(|f| is_base_face(f.centroid[0], f.planar, f.normal[0], geom.length, params.base_tol)).count();
    if base_faces == 0 && treatment != "fairing" {
        bail!(
            "aero.cfd.surface = \"step\": the shape has no base faces (aft-facing faces at the base x = {:.4} m of the STL geometry); the CAD model must end in a flat base",
            geom.length
        );
    }
    let mut decision = decide(opt, &report)?;
    eprintln!("note: aero.cfd: {}", decision.summary());
    // The quarter (y >= 0, z >= 0) needs a boolean with the edge y = z = 0 through the nose tip, which OpenCASCADE cannot
    // always do on an imported shape; then the half is meshed (the shape is still verified symmetric about z = 0, but the
    // half mesh is not exactly z-symmetric, so -alpha is solved). The failure is remembered per prepared shape.
    let quarter_stamp = dir.join("step_quarter_failed.json");
    let prep_hash = prepare_hash(&params, &bytes);
    if decision.meshed == "quarter" && std::fs::read_to_string(&quarter_stamp).is_ok_and(|t| t.trim() == prep_hash) {
        half_instead_of_quarter(&mut decision);
    }
    if report.solids > 1 {
        warnings.push(format!("the CAD file holds {} disjoint solids; all of them are walls of the CFD mesh", report.solids));
    }
    let feature = report.smallest_face_width;
    params.mode = "mesh".into();
    params.min_size = min_size(feature, opt.wall_size);
    loop {
        // `apply` takes the quarter mesh (z_mirror_mesh) from the decision.
        let ropt = symmetry::apply(&symmetry_options(opt), &decision);
        let domain = match (decision.meshed.as_str(), ropt.symmetry) {
            ("quarter", true) => "quarter_half",
            ("quarter", false) => "quarter_full",
            ("half", true) => "half",
            ("half", false) => "half_mirrored",
            _ => "full",
        };
        params.meshed = decision.meshed.clone();
        params.y_mirror = decision.meshed != "full" && !ropt.symmetry;
        params.bl = (opt.model == FlowModel::Rans).then(|| {
            let mut bl = mesh::bl_params(&ropt, geom, atm);
            // The CAD edges are meshed as they are (no blunting).
            let mut b = Some(&mut bl);
            while let Some(x) = b {
                x.edge_blunt = 0.0;
                b = x.retry.as_deref_mut();
            }
            bl
        });
        let (stats, hash, was_cached) = match run_mesh(opt, &dir, &params, &bytes) {
            Ok(r) => r,
            Err(e) if decision.meshed == "quarter" && e.to_string().contains(QUARTER_FAILED) => {
                eprintln!("note: aero.cfd: the quarter domain cannot be cut from the CAD shape; meshing the half domain");
                std::fs::write(&quarter_stamp, &prep_hash)?;
                half_instead_of_quarter(&mut decision);
                continue;
            }
            Err(e) => return Err(e),
        };
        let info = StepMeshInfo {
            file: opt.step_file.display().to_string(),
            format: format.into(),
            file_unit_m: declared,
            step_unit: format!("{:?}", opt.step_unit).to_lowercase(),
            unit_scale: scale,
            solids_in_file: report.solids_in_file,
            solids: report.solids,
            faces: report.faces.len(),
            base_faces,
            volume: report.volume,
            area: report.area,
            import_notes: report.notes.clone(),
            registration: reg,
            symmetry: decision,
            domain: domain.into(),
            base_treatment: treatment.into(),
            fairing: report.fairing.clone(),
            smallest_feature: feature,
            min_size: params.min_size,
            warnings,
        };
        return Ok((MeshResult { su2: dir.join("mesh.su2"), hash, stats, cached: was_cached, stl: None, step: Some(info) }, ropt));
    }
}

/// Marker of `mesh_step.py` when the quarter domain cannot be cut from the shape.
const QUARTER_FAILED: &str = "QUARTER_FAILED";

fn half_instead_of_quarter(d: &mut SymmetryDecision) {
    d.meshed = "half".into();
    d.reason += "; the quarter domain cannot be cut from the CAD shape (OpenCASCADE boolean), so the half is meshed and -alpha is solved";
}

/// Runs the mesh mode unless the mesh of this hash exists (HXT, then a Delaunay retry for Euler). Returns the statistics,
/// the mesh hash and whether the mesh was cached.
fn run_mesh(opt: &CfdOptions, dir: &Path, params: &StepParams, bytes: &[u8]) -> Result<(MeshStats, String, bool)> {
    let hash = step_mesh_hash(params, bytes);
    let su2 = dir.join("mesh.su2");
    let stamp_path = dir.join("mesh.json");
    if let Some(st) =
        std::fs::read_to_string(&stamp_path).ok().and_then(|t| serde_json::from_str::<Stamp>(&t).ok()).filter(|st| st.hash == hash && su2.exists())
    {
        return Ok((st.stats, hash, true));
    }
    let _ = std::fs::remove_file(&stamp_path);
    let (py, prefix) = python(opt)?;
    std::fs::write(dir.join("mesh_step.py"), SCRIPT)?;
    std::fs::write(dir.join("su2_writer.py"), mesh::WRITER)?;
    std::fs::write(dir.join("bl_layer.py"), mesh::BL_LAYER)?;
    let _ = std::fs::remove_file(dir.join("params_retry.json"));
    std::fs::write(dir.join("params.json"), serde_json::to_string_pretty(params)?)?;
    let what = "gmsh meshing of the CAD surface";
    if let Err(e) = run_python(&py, prefix.as_deref(), dir, "mesh_step.py", "params.json", "mesh.log", "MESH_OK", what) {
        if params.bl.is_some() || e.to_string().contains(QUARTER_FAILED) {
            return Err(e);
        }
        // HXT can fail on a surface mesh it cannot recover; Delaunay (a fresh process) is slower but more tolerant.
        eprintln!("note: aero.cfd: HXT could not mesh the volume, retrying with Delaunay");
        let retry = StepParams { algorithm3d: 1, ..params.clone() };
        std::fs::write(dir.join("params.json"), serde_json::to_string_pretty(&retry)?)?;
        run_python(&py, prefix.as_deref(), dir, "mesh_step.py", "params.json", "mesh.log", "MESH_OK", "gmsh meshing of the CAD surface (Delaunay retry)")
            .map_err(|e2| anyhow::anyhow!("{e}\n{e2}"))?;
    }
    let stats: MeshStats = serde_json::from_str(&std::fs::read_to_string(&params.stats_json)?).context("bad mesh_stats.json")?;
    std::fs::write(&stamp_path, serde_json::to_string(&Stamp { hash: hash.clone(), stats: stats.clone() })?)?;
    Ok((stats, hash, false))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::SurfaceKind;
    use crate::test_tmp::TempDir;

    /// `symmetry_tolerance` of `mesh_step.py` (the script imports gmsh, so the two rule lines are run on their own).
    #[test]
    fn symmetry_tolerance_follows_the_self_projection_error() {
        let Ok(py) = std::process::Command::new("python3").arg("--version").output() else { return };
        if !py.status.success() {
            return;
        }
        let code = r#"
import ast, sys
src = open(sys.argv[1]).read()
keep = [n for n in ast.parse(src).body if (isinstance(n, ast.Assign) and getattr(n.targets[0], "id", "") == "SYM_SELF_FACTOR") or (isinstance(n, ast.FunctionDef) and n.name == "symmetry_tolerance")]
ns = {}
exec(compile(ast.Module(keep, []), "rule", "exec"), ns)
f = ns["symmetry_tolerance"]
print(f(float(sys.argv[2]), float(sys.argv[3])))
"#;
        let dir = TempDir::new("symtol");
        std::fs::create_dir_all(&dir).unwrap();
        let script = dir.join("mesh_step.py");
        std::fs::write(&script, SCRIPT).unwrap();
        let tol = |base: f64, self_err: f64| -> f64 {
            let o = std::process::Command::new("python3").args(["-c", code]).arg(&script).arg(base.to_string()).arg(self_err.to_string()).output().unwrap();
            String::from_utf8_lossy(&o.stdout).trim().parse().unwrap_or_else(|_| panic!("{}", String::from_utf8_lossy(&o.stderr)))
        };
        let l = 0.3;
        let base = symmetry::SYMMETRY_TOL * l;
        // Exact projection: the requested tolerance. Basic Finner (self error 6e-7 m): raised above it. Never below it.
        assert_eq!(tol(base, 0.0), base);
        assert_eq!(tol(base, 1e-8), base);
        assert!(tol(base, 6e-7) >= 4.0 * 6e-7 && tol(base, 6e-7) > base);
        // Still far below a rail button / lug offset (millimetres), so asymmetric shapes stay rejected.
        assert!(tol(base, 6e-7) < 1e-4 * l);
    }

    fn sample_geom() -> Geometry {
        use geom::sample::SampleRocket;
        geom::extract(&SampleRocket::default().mesh(), &geom::ExtractOptions::default()).unwrap()
    }

    #[test]
    fn affine_matches_the_stl_transform() {
        let raw = geom::sample::SampleRocket::default().mesh();
        let g = sample_geom();
        let a = cfd_affine(&g);
        let cfd = crate::surface::to_cfd_frame(&raw[..50], &g);
        for (t, c) in raw[..50].iter().zip(&cfd) {
            for (v, w) in t.iter().zip(c) {
                let p = Vec3::new(
                    a[0] * v.x + a[1] * v.y + a[2] * v.z + a[3],
                    a[4] * v.x + a[5] * v.y + a[6] * v.z + a[7],
                    a[8] * v.x + a[9] * v.y + a[10] * v.z + a[11],
                );
                assert!((p - *w).norm() < 1e-12, "{p:?} {w:?}");
            }
        }
        // A rotation (determinant +1), so the shape is not mirrored.
        let det = a[0] * (a[5] * a[10] - a[6] * a[9]) - a[1] * (a[4] * a[10] - a[6] * a[8]) + a[2] * (a[4] * a[9] - a[5] * a[8]);
        assert!((det - 1.0).abs() < 1e-12);
    }

    #[test]
    fn base_rule_and_min_size() {
        let (l, tol) = (1.5, 1.5e-4);
        assert!(is_base_face(1.5, true, 1.0, l, tol));
        assert!(is_base_face(1.5 + 0.5 * tol, true, -0.95, l, tol));
        // Not planar, or not facing along x, or ahead of the base: wall.
        assert!(!is_base_face(1.5, false, 1.0, l, tol));
        assert!(!is_base_face(1.5, true, 0.5, l, tol));
        assert!(!is_base_face(1.2, true, 1.0, l, tol));
        // Aft of the base plane (fairing, nozzle): base whatever the shape.
        assert!(is_base_face(1.6, false, 0.0, l, tol));
        assert!((min_size(0.002, 0.005) - 0.005 / 8.0).abs() < 1e-15);
        assert!((min_size(0.01, 0.005) - 0.0025).abs() < 1e-15);
        assert!((min_size(1.0, 0.005) - 0.005).abs() < 1e-15);
        assert!((min_size(f64::INFINITY, 0.005) - 0.005).abs() < 1e-15);
        assert!((registration_threshold(0.002, 1.8) - 0.018).abs() < 1e-15 && (registration_threshold(0.02, 1.0) - 0.04).abs() < 1e-15);
    }

    #[test]
    fn units_are_read_from_step_and_iges_headers() {
        let si = b"DATA;\n#346 = ( LENGTH_UNIT() NAMED_UNIT(*) SI_UNIT(.MILLI.,.METRE.) );\n#347 = ( NAMED_UNIT(*) PLANE_ANGLE_UNIT() SI_UNIT($,.RADIAN.) );";
        assert_eq!(file_unit_metres("step", si), Some(0.001));
        let m = b"#5=(LENGTH_UNIT()NAMED_UNIT(*)SI_UNIT($,.METRE.));";
        assert_eq!(file_unit_metres("step", m), Some(1.0));
        let inch = b"#10=(CONVERSION_BASED_UNIT('INCH',#11)LENGTH_UNIT()NAMED_UNIT(#12));\n#13=(LENGTH_UNIT()NAMED_UNIT(*)SI_UNIT(.MILLI.,.METRE.));";
        assert_eq!(file_unit_metres("step", inch), Some(0.0254));
        assert_eq!(file_unit_metres("step", b"no units here"), None);
        assert_eq!(file_unit_metres("brep", si), None);
        // IGES global section: parameter 14 is the units flag (2 = mm, 6 = m, 1 = inch).
        let iges = |flag: u32| {
            let g = format!(
                "1H,,1H;,4Hpart,8Hpart.igs,6Hsystem,3H1.0,32,38,6,308,15,4Hpart,1.,{flag},2HMM,1,0.01,15H20250101.000000,1E-06,1000.,6Hauthor,3Horg,11,0;"
            );
            let mut text = format!("{:<72}S      1\n", "start");
            for (k, chunk) in g.as_bytes().chunks(72).enumerate() {
                text += &format!("{:<72}G{:>7}\n", std::str::from_utf8(chunk).unwrap(), k + 1);
            }
            text
        };
        assert_eq!(file_unit_metres("iges", iges(2).as_bytes()), Some(0.001));
        assert_eq!(file_unit_metres("iges", iges(6).as_bytes()), Some(1.0));
        assert_eq!(file_unit_metres("iges", iges(1).as_bytes()), Some(0.0254));
        // Override: step_unit / declared unit; BREP has none (metres); STEP without a declaration cannot be overridden.
        let p = Path::new("x.step");
        assert_eq!(unit_scale(StepUnit::Auto, "step", Some(0.001), p).unwrap(), 1.0);
        assert!((unit_scale(StepUnit::Mm, "step", Some(1.0), p).unwrap() - 0.001).abs() < 1e-15);
        assert!((unit_scale(StepUnit::In, "brep", None, p).unwrap() - 0.0254).abs() < 1e-15);
        assert!(unit_scale(StepUnit::Mm, "step", None, p).unwrap_err().to_string().contains("cannot be overridden"));
        assert_eq!(cad_format(Path::new("a/b.STP")), Some("step"));
        assert_eq!(cad_format(Path::new("b.igs")), Some("iges"));
        assert_eq!(cad_format(Path::new("b.brep")), Some("brep"));
        assert_eq!(cad_format(Path::new("b.stl")), None);
    }

    /// Closed outward box.
    fn cuboid(lo: [f64; 3], hi: [f64; 3]) -> Vec<Triangle> {
        let p = |i: usize, j: usize, k: usize| Vec3::new([lo[0], hi[0]][i], [lo[1], hi[1]][j], [lo[2], hi[2]][k]);
        let quad = |a: Vec3, b: Vec3, c: Vec3, d: Vec3| vec![[a, b, c], [a, c, d]];
        let mut t = Vec::new();
        t.extend(quad(p(0, 0, 0), p(0, 1, 0), p(0, 1, 1), p(0, 0, 1)));
        t.extend(quad(p(1, 0, 0), p(1, 0, 1), p(1, 1, 1), p(1, 1, 0)));
        t.extend(quad(p(0, 0, 0), p(0, 0, 1), p(1, 0, 1), p(1, 0, 0)));
        t.extend(quad(p(0, 1, 0), p(1, 1, 0), p(1, 1, 1), p(0, 1, 1)));
        t.extend(quad(p(0, 0, 0), p(1, 0, 0), p(1, 1, 0), p(0, 1, 0)));
        t.extend(quad(p(0, 0, 1), p(0, 1, 1), p(1, 1, 1), p(1, 0, 1)));
        t
    }

    /// Refines every triangle into four, `n` times (dense samples on the faces).
    fn refine(t: &[Triangle], n: usize) -> Vec<Triangle> {
        let mut out = t.to_vec();
        for _ in 0..n {
            out = out
                .iter()
                .flat_map(|t| {
                    let (a, b, c) = (t[0], t[1], t[2]);
                    let (ab, bc, ca) = ((a + b) * 0.5, (b + c) * 0.5, (c + a) * 0.5);
                    [[a, ab, ca], [ab, b, bc], [ca, bc, c], [ab, bc, ca]]
                })
                .collect();
        }
        out
    }

    #[test]
    fn registration_accepts_step_only_details_and_buried_stl_shells() {
        // "Rocket" body 1 m x 0.1 x 0.1; the STL has a fin box reaching into the body (buried part), the STEP a lug.
        let body = refine(&cuboid([0.0, -0.05, -0.05], [1.0, 0.05, 0.05]), 3);
        let mut stl = body.clone();
        stl.extend(refine(&cuboid([0.7, -0.01, -0.04], [0.9, 0.01, 0.1]), 2));
        let mut step = body.clone();
        step.extend(refine(&cuboid([0.4, 0.05, -0.005], [0.45, 0.06, 0.005]), 2));
        // The part of the fin outside the body is in the STEP too.
        step.extend(refine(&cuboid([0.7, -0.01, 0.05], [0.9, 0.01, 0.1]), 2));
        let r = register(&stl, &step, 0.002, 1.0);
        assert!(r.ok(), "{:?}\n{}", r.problems, r.summary());
        assert!(r.stl_buried_fraction > 0.0 && r.stl_buried_fraction < 0.25, "{}", r.stl_buried_fraction);
        assert!(r.stl_to_step.max < r.detail_threshold, "{}", r.summary());
        assert!(r.step_only_regions >= 1 && r.step_only_area > 0.0, "{}", r.summary());
    }

    #[test]
    fn registration_names_units_orientation_and_revision() {
        let body = refine(&cuboid([0.0, -0.05, -0.05], [1.0, 0.05, 0.05]), 2);
        // Millimetres read as metres: 1000 times too large.
        let big: Vec<Triangle> = body.iter().map(|t| t.map(|v| v * 1000.0)).collect();
        let r = register(&body, &big, 0.002, 1.0);
        assert!(!r.ok());
        assert!(r.hint.as_deref().unwrap().contains("units") && r.hint.as_deref().unwrap().contains("millimetres"), "{:?}", r.hint);
        let e = r.check(Path::new("rocket.step")).unwrap_err().to_string();
        assert!(e.contains("does not describe the same rocket") && e.contains("offsets STEP - STL") && e.contains("step_unit"), "{e}");
        // Metres read as millimetres.
        let small: Vec<Triangle> = body.iter().map(|t| t.map(|v| v * 0.001)).collect();
        assert!(register(&body, &small, 0.002, 1.0).hint.unwrap().contains("units"));
        // Nose pointing the other way: the shape lies at x in -1..0.
        let flipped: Vec<Triangle> = body.iter().map(|t| t.map(|v| Vec3::new(-v.x, -v.y, v.z))).collect();
        let r = register(&body, &flipped, 0.002, 1.0);
        assert!(!r.ok() && r.problems.iter().any(|p| p.contains("nose tips")), "{:?}", r.problems);
        assert!(r.hint.as_deref().unwrap().contains("orientation"), "{:?}", r.hint);
        // A different revision: a body 3 cm fatter.
        let fat = refine(&cuboid([0.0, -0.08, -0.08], [1.0, 0.08, 0.08]), 2);
        assert!(!register(&body, &fat, 0.002, 1.0).ok());
        // A shorter STEP: the STL sticks out.
        let short = refine(&cuboid([0.0, -0.05, -0.05], [0.9, 0.05, 0.05]), 2);
        let r = register(&body, &short, 0.002, 1.0);
        assert!(!r.ok() && r.problems.iter().any(|p| p.contains("extends beyond")), "{:?}", r.problems);
    }

    #[test]
    fn inside_test_of_a_box() {
        let b = refine(&cuboid([0.0, 0.0, 0.0], [1.0, 1.0, 1.0]), 2);
        let t = InsideTest::new(&b);
        assert!(t.inside(Vec3::new(0.5, 0.5, 0.5)) && t.inside(Vec3::new(0.01, 0.25, 0.75)));
        assert!(!t.inside(Vec3::new(1.5, 0.5, 0.5)) && !t.inside(Vec3::new(-0.5, 0.5, 0.5)) && !t.inside(Vec3::new(0.5, 1.5, 0.5)));
    }

    fn params() -> StepParams {
        let o = CfdOptions { surface: SurfaceKind::Step, step_file: "r.step".into(), base_treatment: Some(BaseTreatment::Flat), ..Default::default() };
        base_params(&o, &sample_geom(), Path::new("/w"), 4, 1.0, "flat")
    }

    #[test]
    fn hashes_include_the_file_bytes_and_the_parameters_but_not_the_paths() {
        let p = params();
        let h = step_mesh_hash(&p, b"ISO-10303-21; one");
        assert_ne!(h, step_mesh_hash(&p, b"ISO-10303-21; two"));
        assert_ne!(prepare_hash(&p, b"a"), prepare_hash(&p, b"b"));
        // Paths, mode and threads do not change the mesh.
        let q = StepParams { step_file: "/elsewhere/r.step".into(), out_su2: "/x/mesh.su2".into(), threads: 16, mode: "mesh".into(), ..p.clone() };
        assert_eq!(h, step_mesh_hash(&q, b"ISO-10303-21; one"));
        assert_eq!(prepare_hash(&p, b"a"), prepare_hash(&q, b"a"));
        // Parameters do.
        assert_ne!(h, step_mesh_hash(&StepParams { unit_scale: 0.001, ..p.clone() }, b"ISO-10303-21; one"));
        assert_ne!(h, step_mesh_hash(&StepParams { meshed: "half".into(), ..p.clone() }, b"ISO-10303-21; one"));
        assert_ne!(prepare_hash(&p, b"a"), prepare_hash(&StepParams { fairing: Some(FairingParams { tail_fairing: 6.0 }), ..p.clone() }, b"a"));
        // The mesh parameters do not invalidate the prepared shape.
        assert_eq!(prepare_hash(&p, b"a"), prepare_hash(&StepParams { min_size: 1e-4, meshed: "half".into(), ..p.clone() }, b"a"));
    }

    #[test]
    fn params_and_script_agree() {
        let atm = aero::atmosphere::AtmosphereModel::default().at(0.0);
        let j = serde_json::to_value(StepParams { bl: Some(mesh::bl_params(&CfdOptions::rans(), &sample_geom(), &atm)), ..params() }).unwrap();
        // Every key the scripts read from params.json (`p["..."]`) and from the layer parameters (`bl["..."]`).
        let blk = j["bl"].as_object().unwrap();
        let mut keys = 0;
        for (name, script) in [("mesh_step.py", SCRIPT), ("bl_layer.py", mesh::BL_LAYER)] {
            for line in script.lines() {
                for (pat, obj) in [("p[\"", j.as_object().unwrap()), ("bl[\"", blk)] {
                    let mut rest = line;
                    while let Some(i) = rest.find(pat) {
                        rest = &rest[i + pat.len()..];
                        let key = &rest[..rest.find('"').unwrap()];
                        assert!(obj.contains_key(key), "{name} reads missing key {pat}{key}");
                        keys += 1;
                    }
                }
            }
        }
        assert!(keys > 40, "{keys}");
        // The layer is the shared module, with the thickness cap of the base and the tetrahedra from top_size.
        assert!(SCRIPT.contains("import bl_layer") && !SCRIPT.contains("def wall_normals"));
        assert!(mesh::BL_LAYER.contains("bl[\"base_thickness\"]") && mesh::BL_LAYER.contains("bl[\"top_size\"]"));
        // The base rule of the script is the one of is_base_face.
        assert!(SCRIPT.contains("cx > L + base_tol or (abs(cx - L) <= base_tol and planar and abs(nx) > 0.9)"));
        assert!(SCRIPT.contains("Geometry.OCCTargetUnit\", \"M\""));
        for k in ["layers", "first_height", "thickness", "smoothing", "corner_smoothing", "junction", "fan", "retry"] {
            assert!(blk.contains_key(k), "{k}");
        }
    }

    #[test]
    fn base_treatment_notes_of_step() {
        let s = CfdOptions { surface: SurfaceKind::Step, step_file: "r.step".into(), ..Default::default() };
        let (t, w) = base_treatment_notes(&CfdOptions { base_treatment: Some(BaseTreatment::Fairing), ..s.clone() });
        assert_eq!(t, "fairing");
        assert!(w[0].contains("CAD shape is MODIFIED"));
        assert_eq!(base_treatment_notes(&CfdOptions { model: FlowModel::Rans, ..s }).0, "none");
    }
}
