// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025-2026 西濱大将 (NISHIHAMA Daisuke)
//! A CAD file (STEP, IGES or BREP) as the rocket geometry: the file is imported with OpenCASCADE (the gmsh python of the
//! CFD tools) and tessellated finely in its own frame and in metres, so that the geometry extraction, the Barrowman and
//! panel models and the flight work on it exactly as on an STL.
//!
//! The import is that of the CFD surface `surface = "step"` (`mesh_step.py`, mode `extract`): lengths converted to metres
//! from the unit the file declares (a BREP file has none and is read as metres, `unit` overrides the declaration), loose
//! faces sewn into a solid, several solids fused into one. The tessellation is cached below `<workdir>/cad/`, keyed by
//! the bytes of the file, the parameters and the script.
//!
//! # Tessellation
//!
//! Longest element: the [`DIVISIONS`]-th part of the longest extent of the shape; elements per full circle on curved
//! faces: [`CURVATURE_ELEMENTS`]. The slices of the extraction cut the triangles, so the radius profile is read from the
//! inscribed polygons of the circles: the radius is low by at most `1 - cos(pi / n)` for `n` elements per circle, and the
//! size limit keeps `n` far above [`CURVATURE_ELEMENTS`] on the body. Fins are planar faces, tessellated exactly.

use crate::case::hash_parts;
use crate::config::{CfdOptions, StepUnit};
use crate::mesh::run_python;
use crate::step_mesh::{cad_format, file_unit_metres, unit_scale_for, SCRIPT};
use crate::tools;
use anyhow::{bail, Context, Result};
use geom::stl::Triangle;
use serde::Serialize;
use std::path::{Path, PathBuf};

/// The longest triangle edge is at most this fraction of the longest extent of the shape.
pub const DIVISIONS: f64 = 400.0;
/// Elements per full circle on curved faces (`Mesh.MeshSizeFromCurvature`).
pub const CURVATURE_ELEMENTS: usize = 48;
/// Tolerance of the sewing and the shape healing \[m\].
pub const HEAL_TOL: f64 = 1e-6;

/// The CAD file as triangles.
#[derive(Debug, Clone)]
pub struct CadSurface {
    /// Triangles in metres, in the frame of the file.
    pub triangles: Vec<Triangle>,
    /// The tessellation as a binary STL (metres) in the work directory.
    pub stl: PathBuf,
}

#[derive(Serialize)]
struct ExtractParams {
    mode: &'static str,
    step_file: String,
    unit_scale: f64,
    heal_tol: f64,
    affine: [f64; 12],
    tess_size: f64,
    divisions: f64,
    curvature: usize,
    tess_stl: String,
    report_json: String,
}

/// What determines the tessellation (the paths do not).
#[derive(Serialize)]
struct ExtractKey {
    unit_scale: f64,
    heal_tol: f64,
    tess_size: f64,
    divisions: f64,
    curvature: usize,
}

/// Hash of the tessellation: the file bytes, the parameters and the script.
pub fn extract_hash(unit_scale: f64, file_bytes: &[u8]) -> String {
    let k = ExtractKey { unit_scale, heal_tol: HEAL_TOL, tess_size: 0.0, divisions: DIVISIONS, curvature: CURVATURE_ELEMENTS };
    hash_parts(&[file_bytes, serde_json::to_string(&k).unwrap_or_default().as_bytes(), SCRIPT.as_bytes()])
}

/// Tessellates `file` into `<workdir>/cad/` (cached) and reads the triangles. `opt` supplies the tool prefix, `roots` the
/// extra places to look for the gmsh python (see [`tools::gmsh_python`]).
pub fn tessellate_cad(opt: &CfdOptions, roots: &[PathBuf], file: &Path, unit: StepUnit, workdir: &Path) -> Result<CadSurface> {
    let key = "rocket.cad";
    // The script runs in the work directory: every path it is given must be absolute.
    let file = &std::path::absolute(file).unwrap_or_else(|_| file.to_path_buf());
    let workdir = &std::path::absolute(workdir).unwrap_or_else(|_| workdir.to_path_buf());
    let format = cad_format(file).with_context(|| {
        format!(
            "[IY-C066] {key} = \"{}\": not a CAD file OpenCASCADE reads; use a STEP (.step, .stp), IGES (.iges, .igs) or BREP (.brep, .brp) file",
            file.display()
        )
    })?;
    let bytes = std::fs::read(file).with_context(|| format!("[IY-G016] cannot read the CAD file {}", file.display()))?;
    let declared = file_unit_metres(format, &bytes);
    let scale = unit_scale_for(unit, format, declared, file, "[IY-G018] rocket.cad_unit")?;
    let dir = workdir.join("cad");
    std::fs::create_dir_all(&dir).with_context(|| format!("cannot create {}", dir.display()))?;
    let stl = dir.join("cad_surface.stl");
    let report = dir.join("cad_extract_report.json");
    let stamp = dir.join("cad_extract.json");
    let hash = extract_hash(scale, &bytes);
    let cached = stl.exists() && std::fs::read_to_string(&stamp).is_ok_and(|t| t.trim() == hash);
    if !cached {
        let _ = std::fs::remove_file(&stamp);
        let prefix = tools::resolve_prefix(opt);
        let gmsh = tools::probe_gmsh(prefix.as_deref(), roots);
        let Some(py) = gmsh.path.clone().filter(|_| gmsh.ok()) else {
            bail!(
                "[IY-G017] {key} = \"{}\" is read with OpenCASCADE through the gmsh python, which is not installed: {}; \
                 install the CFD tools, or export the rocket as an STL and use rocket.stl",
                file.display(),
                gmsh.problem.as_deref().unwrap_or("unusable")
            );
        };
        let params = ExtractParams {
            mode: "extract",
            step_file: file.display().to_string(),
            unit_scale: scale,
            heal_tol: HEAL_TOL,
            affine: [1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0],
            tess_size: 0.0,
            divisions: DIVISIONS,
            curvature: CURVATURE_ELEMENTS,
            tess_stl: stl.display().to_string(),
            report_json: report.display().to_string(),
        };
        std::fs::write(dir.join("mesh_step.py"), SCRIPT)?;
        std::fs::write(dir.join("cad_extract_params.json"), serde_json::to_string_pretty(&params)?)?;
        run_python(
            &py,
            prefix.as_deref(),
            &dir,
            "mesh_step.py",
            "cad_extract_params.json",
            "cad_extract.log",
            "EXTRACT_OK",
            &format!("the import of the CAD file {}", file.display()),
        )?;
        std::fs::write(&stamp, &hash)?;
    }
    let triangles = geom::stl::read_stl(&stl).with_context(|| format!("[IY-G019] cannot read the tessellation {} of {}", stl.display(), file.display()))?;
    if triangles.is_empty() {
        let _ = std::fs::remove_file(&stamp);
        bail!("[IY-G019] the CAD file {} gave no surface triangles", file.display());
    }
    Ok(CadSurface { triangles, stl })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_hash_follows_the_file_and_the_unit() {
        let a = extract_hash(1.0, b"abc");
        assert_eq!(a, extract_hash(1.0, b"abc"));
        assert_ne!(a, extract_hash(1.0, b"abd"));
        assert_ne!(a, extract_hash(0.001, b"abc"));
    }

    #[test]
    fn a_wrong_extension_and_a_missing_file_are_coded_errors() {
        let opt = CfdOptions::default();
        let e = tessellate_cad(&opt, &[], Path::new("rocket.stl"), StepUnit::Auto, Path::new("/nonexistent")).unwrap_err().to_string();
        assert!(e.starts_with("[IY-C066] ") && e.contains("rocket.cad"), "{e}");
        let e = tessellate_cad(&opt, &[], Path::new("/nonexistent/rocket.step"), StepUnit::Auto, Path::new("/nonexistent")).unwrap_err().to_string();
        assert!(e.starts_with("[IY-G016] cannot read the CAD file"), "{e}");
    }
}
