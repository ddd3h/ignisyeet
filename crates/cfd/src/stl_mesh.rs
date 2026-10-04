// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025-2026 西濱大将 (NISHIHAMA Daisuke)
//! Direct STL surface (`surface = "stl"`): repair, union, volume mesh and geometry fidelity.
//!
//! # Pipeline
//!
//! 1. [`crate::surface::to_cfd_frame`] moves the STL (metres) to the CFD frame (x aft from the nose tip, the
//!    roll of the STL is kept) and [`crate::surface::repair`] welds it, drops degenerate facets, splits the
//!    connected components and orients every component outward by its signed volume (the shells of the sample
//!    STL are all inside-out); each component must be watertight.
//! 2. Euler + `base_treatment = "fairing"`: a closed cone behind the base is added as one more component
//!    ([`crate::surface::fairing_cone`]).
//! 3. `stl_union.py` (manifold3d, exact `batch_boolean(Add)`) writes `union.stl` and `union_report.json`
//!    (volume, area, genus); a detached body or an invalid component aborts.
//! 4. `mesh_stl.py` (gmsh) prepares the wall surface (`stl_remesh`: classify + remesh at `wall_size`, or as is),
//!    meshes the box farfield around the union and writes the SU2
//!    mesh with the markers `wall`, `base` (aft-facing facets, see [`crate::surface::split_base`]) and `farfield`.
//! 5. [`crate::fidelity`] reads the wall back from the mesh and compares it with the input and the union
//!    (`stl_fidelity.json`); an error above `max_geometry_error` fails the run.
//!
//! The mesh hash contains the mesh options, the hash of the repaired components (so a different STL or
//! base treatment re-runs the union and the mesh) and both scripts.

use crate::case::hash_parts;
use crate::config::{BaseTreatment, CfdOptions, FlowModel};
use crate::fidelity::{self, Fidelity, FidelityInput};
use crate::mesh::{mesh_hash, run_python, MeshParams, MeshResult, MeshStats};
use crate::surface;
use crate::tools;
use anyhow::{bail, Context, Result};
use geom::stl::Triangle;
use geom::Geometry;
use serde::{Deserialize, Serialize};
use std::path::Path;

pub const UNION_SCRIPT: &str = include_str!("stl_union.py");
pub const SCRIPT: &str = include_str!("mesh_stl.py");

/// What the STL pipeline derived (stored in `cfd_report.json` and shown by `cfd-check --mesh`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StlMeshInfo {
    pub fidelity: Fidelity,
    /// `union_report.json` of `stl_union.py`.
    pub union_report: serde_json::Value,
    /// Components, flipped shells and dropped facets of the repair step.
    pub repair: serde_json::Value,
    /// `fairing`, `flat` or `none` (RANS keeps the flat base).
    pub base_treatment: String,
    pub warnings: Vec<String>,
}

#[derive(Serialize, Deserialize)]
struct Stamp {
    hash: String,
    stats: MeshStats,
}

#[derive(Serialize, Deserialize)]
struct FidelityFile {
    mesh_hash: String,
    fidelity: Fidelity,
}

fn bytes_of(components: &[Vec<Triangle>]) -> Vec<u8> {
    let mut b = Vec::new();
    for c in components {
        b.extend_from_slice(&(c.len() as u64).to_le_bytes());
        for v in c.iter().flatten() {
            for x in [v.x, v.y, v.z] {
                b.extend_from_slice(&(x as f32).to_le_bytes());
            }
        }
    }
    b
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
                "base_treatment = fairing: the input STL is MODIFIED, a closed cone of {} base radii (cut at 10 % of the base radius) is united with the body behind the base; \
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

/// Meshes the STL surface (see the module documentation). `raw` is the STL in metres.
pub fn ensure_mesh_stl(opt: &CfdOptions, geom: &Geometry, raw: &[Triangle], workdir: &Path, threads: usize) -> Result<MeshResult> {
    opt.validate()?;
    let dir = workdir.join("mesh");
    std::fs::create_dir_all(&dir).with_context(|| format!("cannot create {}", dir.display()))?;
    let (treatment, warnings) = base_treatment_notes(opt);
    for w in &warnings {
        eprintln!("warning: aero.cfd: {w}");
    }
    let tol = 1e-6 * geom.length;
    let input = surface::to_cfd_frame(raw, geom);
    let repaired = surface::repair(&input, tol)?;
    let mut components = repaired.components.clone();
    if treatment == "fairing" {
        let ring = repaired.components.iter().filter_map(|c| surface::base_ring(c, geom.length, 1e-4 * geom.length)).max_by_key(|r| r.len());
        components.push(surface::fairing_cone(geom.length, geom.base_radius, opt.tail_fairing * geom.base_radius, opt.wall_size, ring.as_deref()));
    }
    let union_hash = hash_parts(&[&bytes_of(&components), UNION_SCRIPT.as_bytes()]);
    let mut params = MeshParams::new(opt, geom, &dir, threads);
    params.kind = "stl".into();
    params.domain = "stl_box".into();
    params.union_hash = union_hash.clone();
    params.stl_surface = "union.stl".into();
    params.stl_remesh = format!("{:?}", opt.stl_remesh).to_lowercase();
    params.symmetry = false;
    params.y_mirror = false;
    params.z_mirror = false;
    params.tail_length = 0.0;
    params.profile = Vec::new();
    params.fins = None;
    let hash = mesh_hash(&params);
    let su2 = dir.join("mesh.su2");
    let stamp_path = dir.join("mesh.json");
    let tools = tools::check_tools(opt, &[]);
    let cached = std::fs::read_to_string(&stamp_path).ok().and_then(|t| serde_json::from_str::<Stamp>(&t).ok()).filter(|st| st.hash == hash && su2.exists());
    let was_cached = cached.is_some();
    let stats = match cached {
        Some(st) => st.stats,
        None => {
            let _ = std::fs::remove_file(&stamp_path);
            if !tools.gmsh.ok() {
                bail!("{}", crate::not_installed_message(&tools, opt));
            }
            if !tools.manifold.ok() {
                bail!("{}", tools.manifold.problem.clone().unwrap_or_default());
            }
            let py = tools.gmsh.path.clone().context("no python with gmsh")?;
            let pre = tools.prefix.as_deref();
            // Union.
            let union_stamp = dir.join("union.json");
            let union_ok = dir.join("union.stl").exists() && std::fs::read_to_string(&union_stamp).is_ok_and(|t| t.trim() == union_hash);
            if !union_ok {
                let _ = std::fs::remove_file(&union_stamp);
                let mut files = Vec::new();
                for (i, c) in components.iter().enumerate() {
                    let name = format!("component_{i}.stl");
                    geom::stl::write_stl_binary(&dir.join(&name), c)?;
                    files.push(name);
                }
                std::fs::write(dir.join("stl_union.py"), UNION_SCRIPT)?;
                std::fs::write(dir.join("union_params.json"), serde_json::to_string_pretty(&serde_json::json!({ "components": files, "out_stl": "union.stl", "report_json": "union_report.json" }))?)?;
                run_python(&py, pre, &dir, "stl_union.py", "union_params.json", "union.log", "UNION_OK", "the STL union (manifold3d)")?;
                std::fs::write(&union_stamp, &union_hash)?;
            }
            std::fs::write(dir.join("repair_report.json"), serde_json::to_string_pretty(&repaired.report)?)?;
            std::fs::write(dir.join("mesh_stl.py"), SCRIPT)?;
            std::fs::write(dir.join("params.json"), serde_json::to_string_pretty(&params)?)?;
            if let Err(e) = run_python(&py, pre, &dir, "mesh_stl.py", "params.json", "mesh.log", "MESH_OK", "gmsh meshing of the STL surface") {
                // HXT can fail on a surface mesh it cannot recover; Delaunay (a fresh process) is slower but more tolerant.
                eprintln!("note: aero.cfd: HXT could not mesh the volume, retrying with Delaunay");
                let retry = MeshParams { algorithm3d: 1, ..params.clone() };
                std::fs::write(dir.join("params.json"), serde_json::to_string_pretty(&retry)?)?;
                run_python(&py, pre, &dir, "mesh_stl.py", "params.json", "mesh.log", "MESH_OK", "gmsh meshing of the STL surface (Delaunay retry)").map_err(|e2| anyhow::anyhow!("{e}\n{e2}"))?;
            }
            let stats: MeshStats = serde_json::from_str(&std::fs::read_to_string(&params.stats_json)?).context("bad mesh_stats.json")?;
            std::fs::write(&stamp_path, serde_json::to_string(&Stamp { hash: hash.clone(), stats: stats.clone() })?)?;
            stats
        }
    };
    let fidelity = fidelity_for(opt, &dir, &su2, &hash, &input)?;
    let union_report: serde_json::Value = std::fs::read_to_string(dir.join("union_report.json")).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default();
    let mut notes = warnings;
    if opt.stl_remesh == crate::config::StlRemesh::None {
        notes.push("stl_remesh = none: the triangles of the STL are meshed as they are; flat faces made of a few huge triangles (fins) are under-resolved".to_string());
    }
    notes.extend(fidelity.warnings.clone());
    let info = StlMeshInfo { fidelity: fidelity.clone(), union_report, repair: serde_json::to_value(&repaired.report)?, base_treatment: treatment.into(), warnings: notes };
    let res = MeshResult { su2, hash, stats, cached: was_cached, stl: Some(info) };
    fidelity.check()?;
    Ok(res)
}

/// Fidelity of the CFD wall (cached in `stl_fidelity.json` per mesh hash; the thresholds are applied on every call).
fn fidelity_for(opt: &CfdOptions, dir: &Path, su2: &Path, mesh_hash: &str, input: &[Triangle]) -> Result<Fidelity> {
    let path = dir.join("stl_fidelity.json");
    let max_error = opt.max_geometry_error.unwrap_or(opt.wall_size);
    let apply = |mut f: Fidelity| {
        f.wall_size = opt.wall_size;
        f.warn_above = 0.5 * opt.wall_size;
        f.fail_above = max_error;
        f.warnings.clear();
        if f.max_error() > f.warn_above {
            f.warnings.push(format!("geometry error {:.3e} m exceeds half the wall size ({:.3e} m)", f.max_error(), f.warn_above));
        }
        f
    };
    if let Some(c) = std::fs::read_to_string(&path).ok().and_then(|t| serde_json::from_str::<FidelityFile>(&t).ok()).filter(|c| c.mesh_hash == mesh_hash) {
        return Ok(apply(c.fidelity));
    }
    let union_tris = geom::stl::read_stl(&dir.join("union.stl"))?;
    let report: serde_json::Value = std::fs::read_to_string(dir.join("union_report.json")).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default();
    let mut m = fidelity::read_su2_markers(su2, &["wall", "base"])?;
    let wall = m.remove("wall").unwrap_or_default();
    let base = m.remove("base").unwrap_or_default();
    if wall.is_empty() {
        bail!("the mesh {} has no wall facets", su2.display());
    }
    let f = fidelity::compute(&FidelityInput {
        input,
        union: &union_tris,
        union_volume: report["volume"].as_f64().unwrap_or(0.0),
        genus: report["genus"].as_i64(),
        wall: &wall,
        base: &base,
        wall_size: opt.wall_size,
        max_error,
    });
    std::fs::write(&path, serde_json::to_string_pretty(&FidelityFile { mesh_hash: mesh_hash.to_string(), fidelity: f.clone() })?)?;
    Ok(apply(f))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base_treatment_warnings() {
        let stl = CfdOptions { surface: crate::config::SurfaceKind::Stl, ..Default::default() };
        let (t, w) = base_treatment_notes(&CfdOptions { base_treatment: Some(BaseTreatment::Fairing), ..stl.clone() });
        assert_eq!(t, "fairing");
        assert!(w[0].contains("MODIFIED"));
        let (t, w) = base_treatment_notes(&CfdOptions { base_treatment: Some(BaseTreatment::Flat), ..stl.clone() });
        assert_eq!(t, "flat");
        assert!(w[0].contains("not to converge"));
        assert_eq!(base_treatment_notes(&CfdOptions { model: FlowModel::Rans, ..stl }).0, "none");
    }

    #[test]
    fn component_bytes_change_with_the_geometry() {
        let t: Triangle = [geom::Vec3::new(0.0, 0.0, 0.0), geom::Vec3::new(1.0, 0.0, 0.0), geom::Vec3::new(0.0, 1.0, 0.0)];
        let a = bytes_of(&[vec![t]]);
        let mut u = t;
        u[2].y = 1.0001;
        assert_ne!(a, bytes_of(&[vec![u]]));
        assert_ne!(a, bytes_of(&[vec![t], vec![t]]));
    }
}
