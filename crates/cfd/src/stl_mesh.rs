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
//! 4. [`crate::symmetry`] checks the mirror symmetry of the union about `y = 0` and `z = 0` and decides, with
//!    `shape_symmetry`, the symmetry in effect (an explicit symmetry that fails the check is an error). The
//!    options of the run (shape symmetry, quarter mesh, SU2 half domain) follow from it ([`ensure_mesh_stl`] returns them).
//! 5. `mesh_stl.py` (gmsh) prepares the wall surface (`stl_remesh`: classify + remesh at `wall_size` and at `fin_size` on the
//!    classified fin surfaces, or as is),
//!    meshes the box farfield around the union (a quarter or half box around the union clipped at the symmetry
//!    planes, `union_clip.stl`, when the surface is symmetric; the quarter is mirrored about z = 0, the half is kept or
//!    mirrored about y = 0) and writes the SU2 mesh with the markers `wall`, `base` (aft-facing facets, see
//!    [`crate::surface::split_base`]), `farfield` and, for a half domain, `symmetry`.
//! 6. [`crate::fidelity`] reads the wall back from the mesh and compares it with the input and the union (clipped
//!    like the wall when the half domain is kept; `stl_fidelity.json`); an error above `max_geometry_error` fails the run.
//!
//! The mesh hash contains the mesh options (with the chosen symmetry and domain), the hash of the repaired
//! components (so a different STL or base treatment re-runs the union and the mesh) and both scripts.
//!
//! # Files written for `surface = "stl"` (below `<workdir>/mesh/`)
//!
//! * `repair_report.json`: `input_triangles`, `weld_tolerance`, `vertices` (after welding), `degenerate_dropped`,
//!   `duplicate_dropped`, and per closed shell in `components`: `triangles`, `volume` (m^3), `flipped` (inside-out, all
//!   facets reversed), `reoriented_facets`, `bbox_min`, `bbox_max`. Only topology is repaired; no vertex moves except
//!   by welding. The repaired shells are written as `component_0.stl`, ...
//! * `union.stl` and `union_report.json`: the boolean union of the shells and its report: `components` (`index`,
//!   `status`, `volume`, `triangles`), `volume`, `surface_area`, `genus`, `triangles`, `vertices`,
//!   `volume_sum_of_components`, `overlap_volume`, `bbox_min`, `bbox_max`.
//! * `stl_fidelity.json`: `mesh_hash` and `fidelity`: `input` / `union` / `cfd` (`triangles`, `area`, `volume`: null for
//!   the input, whose shells overlap, and for the union and the CFD wall when a half domain is solved as it is,
//!   `bbox_min`, `bbox_max`), `cfd_wall_triangles`, `cfd_base_triangles`, `cfd_vs_union` and `cfd_wall_vs_input`
//!   (distance statistics (m): `max`, `max_at`, `p99`, `rms`, `samples`), `wall_size`, `warn_above` (0.5 `wall_size`),
//!   `fail_above` (`max_geometry_error`), `genus`, `warnings`, `symmetry` (as `symmetry_check` of `cfd_report.json`).

use crate::case::hash_parts;
use crate::config::{BaseTreatment, CfdOptions, FlowModel};
use crate::fidelity::{self, Fidelity, FidelityInput};
use crate::mesh::{mesh_hash, run_python, MeshParams, MeshResult, MeshStats};
use crate::surface;
use crate::symmetry;
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
    /// Mirror-symmetry check of the union and the domain chosen from it.
    pub symmetry: symmetry::SymmetryDecision,
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
                "[IY-F511] base_treatment = fairing: the input STL is MODIFIED, a closed cone of {} base radii (cut at 10 % of the base radius) is united with the body behind the base; \
                 the base is not integrated, the base drag comes from the empirical base term",
                opt.tail_fairing
            )],
        ),
        _ => (
            "flat",
            vec!["[IY-F510] base_treatment = flat: the flat base is kept; the inviscid flow behind it is ill-posed, so the Euler solution is likely not to converge (cases that do not converge fail); the base is not integrated, the base drag comes from the empirical base term".to_string()],
        ),
    }
}

struct Prepared {
    input: Vec<Triangle>,
    repaired: surface::Repaired,
    components: Vec<Vec<Triangle>>,
    union_hash: String,
}

/// Repairs the input and builds the components (plus the fairing cone) of the union.
fn prepare(opt: &CfdOptions, geom: &Geometry, raw: &[Triangle], treatment: &str) -> Result<Prepared> {
    let tol = 1e-6 * geom.length;
    let input = surface::to_cfd_frame(raw, geom);
    let repaired = surface::repair(&input, tol)?;
    let mut components = repaired.components.clone();
    if treatment == "fairing" {
        let ring = repaired.components.iter().filter_map(|c| surface::base_ring(c, geom.length, 1e-4 * geom.length)).max_by_key(|r| r.len());
        components.push(surface::fairing_cone(geom.length, geom.base_radius, opt.tail_fairing * geom.base_radius, opt.wall_size, ring.as_deref()));
    }
    let union_hash = hash_parts(&[&bytes_of(&components), UNION_SCRIPT.as_bytes()]);
    Ok(Prepared { input, repaired, components, union_hash })
}

/// Runs the union script (manifold3d) unless `union.stl` of this hash exists.
fn ensure_union(opt: &CfdOptions, dir: &Path, p: &Prepared) -> Result<()> {
    let union_stamp = dir.join("union.json");
    if dir.join("union.stl").exists() && std::fs::read_to_string(&union_stamp).is_ok_and(|t| t.trim() == p.union_hash) {
        return Ok(());
    }
    let _ = std::fs::remove_file(&union_stamp);
    let tools = tools::check_tools(opt, &[]);
    if !tools.gmsh.ok() {
        bail!("{}", crate::not_installed_message(&tools, opt));
    }
    if !tools.manifold.ok() {
        bail!("[IY-F007] {}", tools.manifold.problem.clone().unwrap_or_default());
    }
    let py = tools.gmsh.path.clone().context("no python with gmsh")?;
    let mut files = Vec::new();
    for (i, c) in p.components.iter().enumerate() {
        let name = format!("component_{i}.stl");
        geom::stl::write_stl_binary(&dir.join(&name), c)?;
        files.push(name);
    }
    std::fs::write(dir.join("stl_union.py"), UNION_SCRIPT)?;
    std::fs::write(
        dir.join("union_params.json"),
        serde_json::to_string_pretty(&serde_json::json!({ "components": files, "out_stl": "union.stl", "report_json": "union_report.json" }))?,
    )?;
    run_python(&py, tools.prefix.as_deref(), dir, "stl_union.py", "union_params.json", "union.log", "UNION_OK", "the STL union (manifold3d)")?;
    std::fs::write(&union_stamp, &p.union_hash)?;
    Ok(())
}

/// Repairs and unites the STL (cached below `<workdir>/mesh/`) and checks the mirror symmetry of the union, without meshing.
/// Returns the decision `shape_symmetry` leads to (an error for an explicit symmetry that fails the check).
pub fn check_symmetry(opt: &CfdOptions, geom: &Geometry, raw: &[Triangle], workdir: &Path) -> Result<symmetry::SymmetryDecision> {
    opt.validate()?;
    let dir = workdir.join("mesh");
    std::fs::create_dir_all(&dir).with_context(|| format!("cannot create {}", dir.display()))?;
    let prep = prepare(opt, geom, raw, base_treatment_notes(opt).0)?;
    ensure_union(opt, &dir, &prep)?;
    let union_tris = geom::stl::read_stl(&dir.join("union.stl"))?;
    let (y, z) = symmetry::check_planes(&union_tris, geom.length);
    symmetry::decide(opt, &y, &z)
}

/// Meshes the STL surface (see the module documentation). `raw` is the STL in metres. Returns the mesh and the
/// options with the symmetry of the surface resolved ([`crate::symmetry::decide`]): the case matrix and the SU2
/// configuration must be made from them.
pub fn ensure_mesh_stl(opt: &CfdOptions, geom: &Geometry, raw: &[Triangle], workdir: &Path, threads: usize) -> Result<(MeshResult, CfdOptions)> {
    opt.validate()?;
    let dir = workdir.join("mesh");
    std::fs::create_dir_all(&dir).with_context(|| format!("cannot create {}", dir.display()))?;
    let (treatment, warnings) = base_treatment_notes(opt);
    for w in &warnings {
        eprintln!("{}", crate::coded_line("warning", "aero.cfd:", w));
    }
    let prep = prepare(opt, geom, raw, treatment)?;
    ensure_union(opt, &dir, &prep)?;
    let union_tris = geom::stl::read_stl(&dir.join("union.stl"))?;
    let (ycheck, zcheck) = symmetry::check_planes(&union_tris, geom.length);
    let decision = symmetry::decide(opt, &ycheck, &zcheck)?;
    eprintln!("note: aero.cfd: {}", decision.summary());
    let ropt = symmetry::apply(opt, &decision);
    // The quarter / half of the union is meshed; the half is kept (SU2 `symmetry` marker) or mirrored to the full domain.
    let (clip_y, clip_z) = (decision.meshed != "full", decision.meshed == "quarter");
    let y_mirror = clip_y && !ropt.symmetry;
    let domain = match (decision.meshed.as_str(), ropt.symmetry) {
        ("quarter", true) => "stl_quarter_half",
        ("quarter", false) => "stl_quarter_full",
        ("half", true) => "stl_half",
        ("half", false) => "stl_half_full",
        _ => "stl_box",
    };
    let tol = 1e-7 * geom.length;
    let mut meshed = union_tris.clone();
    if clip_y {
        meshed = surface::clip_plane(&meshed, 1, tol);
    }
    if clip_z {
        meshed = surface::clip_plane(&meshed, 2, tol);
    }
    if clip_y {
        surface::check_clipped(&meshed, clip_y, clip_z, geom.length)
            .context("the united STL surface cannot be clipped at the symmetry plane; use shape_symmetry = \"none\"")?;
    }
    let surface_file = if clip_y { "union_clip.stl" } else { "union.stl" };
    let mut params = MeshParams::new(&ropt, geom, &dir, threads);
    params.kind = "stl".into();
    params.domain = domain.into();
    params.union_hash = prep.union_hash.clone();
    params.stl_surface = surface_file.into();
    params.stl_remesh = format!("{:?}", opt.stl_remesh).to_lowercase();
    params.symmetry = ropt.symmetry;
    params.y_mirror = y_mirror;
    params.z_mirror = clip_z;
    // Curvature-driven sizes below the wall size make the remeshed discrete surface contain inverted facets
    // that HXT rejects on some STLs (the 24 elements per circle and wall_size / 8 of the CAD mesh failed on the
    // sample STL with a few 1e-7 m of coordinate noise): 12 elements per circle and wall_size / 5 are robust.
    params.curvature_elements = 12;
    params.min_size_divisor = 5.0;
    params.tail_length = 0.0;
    // The extracted profile and fins only serve to find the fin surfaces of the refinement (fin_size < wall_size, the
    // classified surface; `stl_remesh = "none"` keeps the STL triangles).
    if params.fin_size >= params.wall_size || opt.stl_remesh == crate::config::StlRemesh::None {
        params.fin_size = params.wall_size;
        params.profile = Vec::new();
        params.fins = None;
    }
    let hash = mesh_hash(&params);
    let su2 = dir.join("mesh.su2");
    let stamp_path = dir.join("mesh.json");
    let cached = std::fs::read_to_string(&stamp_path).ok().and_then(|t| serde_json::from_str::<Stamp>(&t).ok()).filter(|st| st.hash == hash && su2.exists());
    let was_cached = cached.is_some();
    let stats = match cached {
        Some(st) => st.stats,
        None => {
            let _ = std::fs::remove_file(&stamp_path);
            let tools = tools::check_tools(opt, &[]);
            if !tools.gmsh.ok() {
                bail!("{}", crate::not_installed_message(&tools, opt));
            }
            let py = tools.gmsh.path.clone().context("no python with gmsh")?;
            let pre = tools.prefix.as_deref();
            if clip_y {
                geom::stl::write_stl_binary(&dir.join(surface_file), &meshed)?;
            }
            std::fs::write(dir.join("repair_report.json"), serde_json::to_string_pretty(&prep.repaired.report)?)?;
            std::fs::write(dir.join("mesh_stl.py"), SCRIPT)?;
            std::fs::write(dir.join("params.json"), serde_json::to_string_pretty(&params)?)?;
            if let Err(e) = run_python(&py, pre, &dir, "mesh_stl.py", "params.json", "mesh.log", "MESH_OK", "gmsh meshing of the STL surface") {
                // HXT can fail on a surface mesh it cannot recover; Delaunay (a fresh process) is slower but more tolerant.
                eprintln!("note: aero.cfd: HXT could not mesh the volume, retrying with Delaunay");
                let retry = MeshParams { algorithm3d: 1, ..params.clone() };
                std::fs::write(dir.join("params.json"), serde_json::to_string_pretty(&retry)?)?;
                run_python(&py, pre, &dir, "mesh_stl.py", "params.json", "mesh.log", "MESH_OK", "gmsh meshing of the STL surface (Delaunay retry)").map_err(
                    |e2| {
                        let hint = if clip_y {
                            format!(
                                "\nthe {} domain of the symmetric surface could not be meshed; shape_symmetry = \"none\" meshes the full domain",
                                decision.meshed
                            )
                        } else {
                            String::new()
                        };
                        anyhow::anyhow!("{e}\n{e2}{hint}")
                    },
                )?;
            }
            let stats: MeshStats = serde_json::from_str(&std::fs::read_to_string(&params.stats_json)?).context("bad mesh_stats.json")?;
            std::fs::write(&stamp_path, serde_json::to_string(&Stamp { hash: hash.clone(), stats: stats.clone() })?)?;
            stats
        }
    };
    // Half domain kept: the wall is compared with the clipped union (y >= 0), otherwise with the full union.
    let compare_union = if ropt.symmetry { surface::clip_plane(&union_tris, 1, tol) } else { union_tris.clone() };
    let fidelity = fidelity_for(opt, &dir, &su2, &hash, &prep.input, &compare_union, ropt.symmetry, &decision)?;
    let union_report: serde_json::Value =
        std::fs::read_to_string(dir.join("union_report.json")).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default();
    let mut notes = warnings;
    if opt.stl_remesh == crate::config::StlRemesh::None {
        notes.push(
            "stl_remesh = none: the triangles of the STL are meshed as they are; flat faces made of a few huge triangles (fins) are under-resolved".to_string(),
        );
    }
    notes.extend(fidelity.warnings.clone());
    let info = StlMeshInfo {
        fidelity: fidelity.clone(),
        union_report,
        repair: serde_json::to_value(&prep.repaired.report)?,
        base_treatment: treatment.into(),
        warnings: notes,
        symmetry: decision,
    };
    let res = MeshResult { su2, hash, stats, cached: was_cached, stl: Some(info), step: None };
    fidelity.check()?;
    Ok((res, ropt))
}

/// Fidelity of the CFD wall (cached in `stl_fidelity.json` per mesh hash; the thresholds are applied on every call).
#[allow(clippy::too_many_arguments)]
fn fidelity_for(
    opt: &CfdOptions,
    dir: &Path,
    su2: &Path,
    mesh_hash: &str,
    input: &[Triangle],
    union_tris: &[Triangle],
    partial: bool,
    decision: &symmetry::SymmetryDecision,
) -> Result<Fidelity> {
    let path = dir.join("stl_fidelity.json");
    let max_error = opt.max_geometry_error.unwrap_or(opt.wall_size);
    let apply = |mut f: Fidelity| {
        f.wall_size = opt.wall_size;
        f.warn_above = 0.5 * opt.wall_size;
        f.fail_above = max_error;
        f.warnings.clear();
        f.symmetry = Some(decision.clone());
        if f.max_error() > f.warn_above {
            f.warnings.push(format!("[IY-F513] geometry error {:.3e} m exceeds half the wall size ({:.3e} m)", f.max_error(), f.warn_above));
        }
        f
    };
    if let Some(c) = std::fs::read_to_string(&path).ok().and_then(|t| serde_json::from_str::<FidelityFile>(&t).ok()).filter(|c| c.mesh_hash == mesh_hash) {
        return Ok(apply(c.fidelity));
    }
    let report: serde_json::Value = std::fs::read_to_string(dir.join("union_report.json")).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default();
    let mut m = fidelity::read_su2_markers(su2, &["wall", "base"])?;
    let wall = m.remove("wall").unwrap_or_default();
    let base = m.remove("base").unwrap_or_default();
    if wall.is_empty() {
        bail!("[IY-F078] the mesh {} has no wall facets", su2.display());
    }
    let f = fidelity::compute(&FidelityInput {
        input,
        union: union_tris,
        union_volume: report["volume"].as_f64().unwrap_or(0.0),
        genus: report["genus"].as_i64(),
        wall: &wall,
        base: &base,
        wall_size: opt.wall_size,
        max_error,
        partial,
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
