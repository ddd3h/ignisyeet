// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025-2026 西濱大将 (NISHIHAMA Daisuke)
//! CFD aerodynamics mode: wall surface, SU2 cases, post-processing and table assembly.
//!
//! Pipeline: [`mesh`] rebuilds the geometry in gmsh/OpenCASCADE (body of revolution fused with the
//! fins, cut from the farfield half ball) and writes an SU2 mesh; [`runner`] solves the (Mach, alpha)
//! matrix with SU2 (parallel MPI cases, warm starts, resume, progress); [`table`] fills the regular
//! `AeroTable` grid from the solved points; [`forces`] converts SU2 coefficients and adds the hybrid
//! viscous terms. [`surface`] holds the STL-based wall surface utilities (panel-mesh export, half-model
//! clipping); the meshing path does not use them (see [`mesh`] for why).
//!
//! Frames and sign conventions are documented in [`su2cfg`] and [`forces`].

pub mod asymmetry;
pub mod case;
pub mod config;
pub mod fidelity;
pub mod forces;
pub mod general;
pub mod history;
pub mod mesh;
pub mod runner;
pub mod step_mesh;
pub mod stl_mesh;
pub mod su2cfg;
pub mod surface;
pub mod symmetry;
pub mod table;
#[cfg(test)]
#[path = "../../../testutil/tmp.rs"]
mod test_tmp;
pub mod tools;

pub use config::{
    y_symmetric_layout, z_symmetric_layout, AlphaMode, BaseTreatment, CfdOptions, FinSize, FlowModel, Scheme, ShapeSymmetry, StepUnit, StlRemesh, SurfaceKind,
};

use aero::{AeroOptions, AeroTable, Extrapolation};
use anyhow::{bail, Result};
use geom::Geometry;
use serde::Serialize;
use std::path::Path;

/// Progress events of [`build_table_with_progress`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CfdStage {
    /// Surface export and volume meshing.
    Mesh,
    /// Case `index` of `total` (0-based) is running at its latest iteration / log10 residual.
    Case {
        index: usize,
        total: usize,
        mach: f64,
        alpha: f64,
        iter: usize,
        residual: f64,
    },
    CaseDone {
        index: usize,
        total: usize,
        mach: f64,
        alpha: f64,
        converged: bool,
    },
    /// Table assembly.
    Table,
    Done,
}

/// A RANS case accepted with a stalled residual (see [`runner::RansVerdict::Stalled`]).
#[derive(Debug, Clone, Default, Serialize)]
pub struct StalledCase {
    pub case: String,
    pub residual_drop: Option<f64>,
}

/// The warning about the RANS cases accepted with a stalled residual (`None` when there is none).
pub fn stalled_warning(stalled: &[StalledCase]) -> Option<String> {
    if stalled.is_empty() {
        return None;
    }
    let list: Vec<String> =
        stalled.iter().map(|c| format!("{} ({} orders below the reference)", c.case, c.residual_drop.map_or("?".to_string(), |d| format!("{d:.1}")))).collect();
    Some(format!(
        "{} RANS case{} accepted with a stalled residual, not converged (coefficients averaged over two windows; a stalled residual needs only \
         {} orders below the reference, convergence {}): {}; check them in cfd_cases.csv (rans_verdict, residual_drop)",
        stalled.len(),
        if stalled.len() == 1 { "" } else { "s" },
        runner::RANS_STALL_RHO_DROP,
        runner::RANS_RHO_DROP,
        list.join(", ")
    ))
}

/// Summary written to `cfd_report.json`.
#[derive(Debug, Clone, Default, Serialize)]
pub struct CfdReport {
    pub cases: usize,
    pub failed: Vec<(f64, f64)>,
    /// (Mach, alpha) of the accepted cases that did not converge; `unconverged_cases` names their directories (with
    /// sideslip and roll).
    pub unconverged: Vec<(f64, f64)>,
    pub unconverged_cases: Vec<String>,
    /// RANS cases accepted with a stalled residual and the drop of their density residual below the reference [orders].
    pub stalled_cases: Vec<StalledCase>,
    pub wall_seconds: f64,
    pub mesh_nodes: usize,
    pub mesh_cells: usize,
    /// Sum of the per-case wall times (cases run concurrently).
    pub case_seconds: f64,
    /// Failed cases bridged by interpolation.
    pub filled: Vec<(f64, f64)>,
    /// `alpha_mode`: `mirror`, `offset` or `single`.
    pub alpha_mode: String,
    /// Measured alpha = 0 asymmetry per Mach (`CN`, nose moment [m]; the body is symmetric, so the true
    /// values are 0). Removed in `offset` mode, only reported in `mirror` mode.
    pub asymmetry: Vec<table::Asymmetry>,
    /// Lateral force at alpha = beta = 0 relative to CN at 4 deg per Mach (accuracy indicator, see [`asymmetry`]).
    pub zero_angle: Vec<asymmetry::ZeroAngle>,
    /// Resolved MPI ranks per case, concurrent cases and the estimated memory per case [bytes].
    pub ranks_per_case: usize,
    pub parallel_cases: usize,
    pub bytes_per_case: u64,
    /// RANS: estimated y+ of the first cell centre of the wall mesh (see [`mesh::wall_yplus`]).
    pub est_wall_yplus: Option<f64>,
    /// `axisymmetric` or `general`.
    pub table_kind: String,
    /// Resolved shape symmetry (`axisymmetric`, `pitch_plane`, `none`) and domain (`half`, `full`).
    pub shape_symmetry: String,
    pub domain: String,
    pub betas_deg: Vec<f64>,
    /// Case directories of failed cases.
    pub failed_cases: Vec<String>,
    /// General table: the angle region that comes from CFD; everywhere else the Barrowman model,
    /// offset-matched at the boundary of the region.
    pub cfd_region: Option<general::Region>,
    pub model_outside_cfd_region: bool,
    /// Roll damping per Mach number: CFD value, Barrowman value, which one the table uses and why.
    pub clp: Vec<general::ClpEntry>,
    /// `cfd` (all Mach numbers), `barrowman` (not solved or all rejected) or `mixed`.
    pub clp_source: String,
    /// Slopes at zero angle (diagnostics: they are not inputs of a general table).
    pub diagnostics: Vec<general::Slopes>,
    /// General table without sideslip data: the yaw damping sums equal the pitch sums.
    pub yaw_damping_from_pitch: bool,
    /// Fin cant [deg] contained in the CFD geometry (reference of the roll forcing).
    pub fin_cant_ref_deg: f64,
    pub base_treatment: Option<String>,
    pub warnings: Vec<String>,
    /// STL surface: geometry fidelity of the CFD wall (see [`fidelity`]).
    pub fidelity: Option<serde_json::Value>,
    /// STL / STEP surface: mirror-symmetry check of the shape and the domain chosen from it.
    pub symmetry_check: Option<serde_json::Value>,
    /// STEP surface: import, registration against the STL, details and domain ([`step_mesh::StepMeshInfo`]).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub step: Option<serde_json::Value>,
    /// The table is constant (edge value held) above this Mach number: the highest solved one, set only when the
    /// table grid extends beyond it. CFD tables are never extrapolated in Mach.
    pub mach_clamped_above: Option<f64>,
    /// Same below the lowest solved Mach number.
    pub mach_clamped_below: Option<f64>,
    /// RANS: per solved case the CFD wall / base axial coefficients (per-marker history columns) next to the
    /// Barrowman friction and base values. Empty for Euler.
    #[serde(default)]
    pub axial_breakdown: Vec<forces::AxialReportEntry>,
}

/// Message of the missing-tools error.
pub fn not_installed_message(report: &tools::ToolReport, opt: &CfdOptions) -> String {
    let lines: Vec<String> = report.missing(opt).iter().map(|t| format!("  - {}: {}", t.name, t.problem.as_deref().unwrap_or("unusable"))).collect();
    format!("SU2 is not installed ({})\n{}", tools::INSTALL_HINT, lines.join("\n"))
}

/// Fails with the "not installed" error unless SU2, MPI (for more than one rank) and gmsh work.
pub fn check_tools(opt: &CfdOptions) -> Result<tools::ToolReport> {
    let r = tools::check_tools(opt, &[]);
    if !r.ready(opt) {
        bail!("{}", not_installed_message(&r, opt));
    }
    Ok(r)
}

/// The options with every automatic choice made.
///
/// * `shape_symmetry`: for the rebuilt geometry `auto` is `axisymmetric` (the table is roll-averaged) unless
///   sideslip is solved, where the fin layout decides (`axisymmetric`: symmetric about y = 0 and z = 0,
///   `pitch_plane`: about y = 0 only, else `none`). For an STL or STEP surface it stays as configured: the symmetry is
///   resolved from the shape by [`stl_mesh::ensure_mesh_stl`] and [`step_mesh::ensure_mesh_step`]
///   ([`symmetry::decide`]).
/// * `symmetry` (the SU2 half domain) is kept only with a layout symmetric about y = 0 (rebuilt geometry), a shape
///   symmetry that includes y = 0, `betas_deg = [0]` and no roll cases; otherwise the full domain is solved (the
///   mesher mirrors the half mesh when the layout is symmetric, else it uses a box farfield).
/// * `z_mirror_mesh` stays on only for layouts symmetric about both planes.
pub fn resolve_options(geom: &Geometry, cfd: &CfdOptions) -> CfdOptions {
    let mut o = cfd.clone();
    let stl = o.surface_is_user_shape();
    let (count, roll) = mesh::cad_description(geom).1.map_or((0, 0.0), |f| (f.count, cfd.fin_roll_deg));
    let (ysym, zsym) = (y_symmetric_layout(count, roll), z_symmetric_layout(count, roll));
    let sideslip = o.betas_deg.iter().any(|&b| b != 0.0);
    let shape = match o.shape_symmetry {
        Some(s) if s != ShapeSymmetry::Auto => s,
        _ if sideslip => match (ysym, zsym) {
            (true, true) => ShapeSymmetry::Axisymmetric,
            (true, false) => ShapeSymmetry::PitchPlane,
            _ => ShapeSymmetry::None,
        },
        _ => ShapeSymmetry::Axisymmetric,
    };
    if !stl {
        o.shape_symmetry = Some(shape);
        // The RANS boundary-layer mesh is not exactly z-symmetric, so the -alpha cases are solved, not mirrored.
        o.z_mirror_mesh = o.z_mirror_mesh && ysym && zsym && o.model != FlowModel::Rans;
        o.symmetry = o.symmetry && ysym && shape != ShapeSymmetry::None && !sideslip && !o.roll_damping;
    }
    o
}

/// Reference dimensions of the full rocket for the SU2 configuration.
pub fn ref_dims(geom: &Geometry) -> su2cfg::RefDims {
    su2cfg::RefDims { length: 2.0 * geom.ref_radius, area: geom.ref_area }
}

/// Compute resources the caller grants to the CFD mode.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ResourceBudget {
    /// Maximum number of CPU threads (MPI ranks across all concurrent cases, gmsh threads).
    pub threads: usize,
    /// Memory budget for the concurrent SU2 runs (`resources.memory_gb`); `None` = the memory available to this
    /// process ([`panel::memory::available`]: available memory, cgroup limit, address-space limit).
    pub memory_bytes: Option<u64>,
}

impl Default for ResourceBudget {
    fn default() -> Self {
        Self { threads: std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1), memory_bytes: None }
    }
}

/// Resolved parallelism of the runner.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct RunPlan {
    pub ranks: usize,
    pub parallel: usize,
    /// Estimated memory of one case [bytes].
    pub bytes_per_case: u64,
    /// Adjustments made to the requested values.
    pub notes: Vec<String>,
    /// Memory budget in effect [bytes] and its source (`None`: no budget known, the memory is not checked).
    pub memory: Option<(u64, String)>,
    /// Set when a single case is estimated to need more than the memory budget: the reason the run is refused.
    pub over_budget: Option<String>,
}

/// Estimated SU2 memory of one case with `cells` volume cells on `ranks` ranks (Euler: per cell; RANS: per node
/// from `nodes`, or from [`estimate_nodes`] when `nodes` is 0).
///
/// Measured (SU2 8.3, Euler, Roe + MUSCL, implicit FGMRES/ILU, tetrahedra, 4 ranks, peak RSS per
/// rank from `/usr/bin/time -v`): 87 MB at 80 k cells, 165 MB at 197 k cells and 819 MB at 941 k cells
/// (largest rank), i.e. about 40 MB per rank plus 3.3 kB per cell (summed over ranks). SU2 stores the
/// solution, gradients and linear system per node, so RANS (turbulence variable, wall-distance field, a larger linear
/// system) is estimated per mesh node, [`RANS_BYTES_PER_NODE`]; prism boundary-layer meshes have many more cells per
/// node than the 5.5 of a tetrahedral mesh, so a per-cell figure would overestimate them.
pub fn case_memory_bytes_mesh(model: FlowModel, cells: usize, nodes: usize, ranks: usize) -> u64 {
    let per_mesh = match model {
        FlowModel::Euler => EULER_BYTES_PER_CELL * cells as f64,
        // Never below the Euler figure of the same mesh (the RANS state is a superset of the Euler one).
        FlowModel::Rans => (RANS_BYTES_PER_NODE * if nodes > 0 { nodes } else { estimate_nodes(cells) } as f64).max(EULER_BYTES_PER_CELL * cells as f64),
    };
    (ranks as f64 * 40e6 + per_mesh) as u64
}

/// [`case_memory_bytes_mesh`] without a node count.
pub fn case_memory_bytes(model: FlowModel, cells: usize, ranks: usize) -> u64 {
    case_memory_bytes_mesh(model, cells, 0, ranks)
}

/// Node count of a tetrahedral mesh with `cells` cells (about 5.5 cells per node). A prism stack has about one
/// node per cell; pass the real count from the mesh statistics for such meshes.
pub fn estimate_nodes(cells: usize) -> usize {
    (cells as f64 / 5.5).round() as usize
}

pub const EULER_BYTES_PER_CELL: f64 = 3.3e3;
/// RANS bytes per mesh node (summed over ranks). Measured (SU2 8.3, SA, Roe + MUSCL, accurate Jacobians,
/// FGMRES/ILU, prism boundary layer + tetrahedra, peak RSS minus 40 MB per rank): 10.28 kB at 406 k nodes
/// (3 ranks), 10.32 kB at 571 k nodes (4 ranks), 10.26 kB at 323 k nodes (4 ranks); 12 kB keeps a 15 % margin.
pub const RANS_BYTES_PER_NODE: f64 = 12e3;

/// Resolves `ranks_per_case` / `parallel_cases` (0 = auto) against the budget.
///
/// * auto ranks: `min(4, threads)`; auto parallel: `threads / ranks`.
/// * `ranks x parallel` is capped to `threads`: `parallel` is reduced first, then `ranks`.
/// * memory budget: `memory_bytes`, or without it the memory available to this process
///   ([`panel::memory::available`]); `parallel` is reduced until the estimated memory of the concurrent cases fits.
///   When even one case exceeds the budget, [`RunPlan::over_budget`] says so and the run is refused.
/// * `parallel` never exceeds the number of cases.
pub fn plan_resources(opt: &CfdOptions, budget: &ResourceBudget, cells: usize, n_cases: usize) -> RunPlan {
    plan_resources_mesh(opt, budget, cells, 0, n_cases)
}

/// [`plan_resources`] with the node count of the mesh (0 = estimate from `cells`).
pub fn plan_resources_mesh(opt: &CfdOptions, budget: &ResourceBudget, cells: usize, nodes: usize, n_cases: usize) -> RunPlan {
    let available = if budget.memory_bytes.is_none() { panel::memory::available() } else { None };
    plan_resources_with(opt, budget, cells, nodes, n_cases, available)
}

/// [`plan_resources_mesh`] with the memory available to this process given (used when `budget.memory_bytes` is
/// `None`; `None` here as well: the memory is not checked).
pub fn plan_resources_with(
    opt: &CfdOptions,
    budget: &ResourceBudget,
    cells: usize,
    nodes: usize,
    n_cases: usize,
    available: Option<panel::memory::Available>,
) -> RunPlan {
    let threads = budget.threads.max(1);
    let mut notes = Vec::new();
    let mut ranks = if opt.ranks_per_case == 0 { threads.min(4) } else { opt.ranks_per_case };
    let mut parallel = if opt.parallel_cases == 0 { (threads / ranks).max(1) } else { opt.parallel_cases };
    if ranks > threads {
        notes.push(format!("ranks_per_case reduced from {ranks} to {threads} (thread budget)"));
        ranks = threads;
    }
    if ranks * parallel > threads {
        let p = (threads / ranks).max(1);
        notes.push(format!("parallel_cases reduced from {parallel} to {p} (thread budget {threads})"));
        parallel = p;
    }
    parallel = parallel.min(n_cases.max(1));
    let per = case_memory_bytes_mesh(opt.model, cells, nodes, ranks);
    let memory = match budget.memory_bytes {
        Some(b) => Some((b, "resources.memory_gb".to_string())),
        None => available.map(|a| (a.bytes, a.source)),
    };
    let mut over_budget = None;
    if let Some((mem, source)) = &memory {
        let gb = |b: u64| b as f64 / 1e9;
        let fit = ((mem / per.max(1)) as usize).max(1);
        if fit < parallel {
            notes.push(format!(
                "parallel_cases reduced from {parallel} to {fit} (memory budget {:.1} GB from {source}, about {:.1} GB per case)",
                gb(*mem),
                gb(per)
            ));
            parallel = fit;
        }
        if per > *mem {
            let nodes_text = if nodes > 0 { format!("{nodes} nodes") } else { format!("about {} nodes", estimate_nodes(cells)) };
            over_budget = Some(format!(
                "a single CFD case needs about {:.1} GB ({} on {cells} cells / {nodes_text}, {ranks} ranks), more than the memory budget \
                 of {:.1} GB ({source}); use a coarser mesh (a larger wall_size{}) or fewer ranks_per_case (about 40 MB each){}",
                gb(per),
                if opt.model == FlowModel::Rans { "RANS" } else { "Euler" },
                gb(*mem),
                if opt.model == FlowModel::Rans { format!(", RANS default {} m", config::RANS_DEFAULT_WALL_SIZE) } else { String::new() },
                if budget.memory_bytes.is_some() {
                    ", or raise resources.memory_gb"
                } else {
                    ", or set resources.memory_gb when more memory can be freed for the run"
                }
            ));
        }
    }
    RunPlan { ranks, parallel, bytes_per_case: per, notes, memory, over_budget }
}

/// Some RANS case has no per-marker `CFx` columns: its axial coefficient is the monitored total, base cap included.
pub(crate) fn rans_markers_missing<'a>(mut results: impl Iterator<Item = &'a case::CaseResult>) -> bool {
    results.any(|r| r.axial.is_some_and(|a| a.markers_missing))
}

/// The table takes the base term from the CFD axial coefficients (no Barrowman base on top): `rans_base = "cfd"`,
/// or a case whose axial coefficient is the total (a Barrowman base on top would count the base twice).
pub(crate) fn table_base_from_cfd(rans_base_cfd: bool, markers_missing: bool) -> bool {
    rans_base_cfd || markers_missing
}

/// Per-case axial breakdown of the RANS report, from the per-marker values stored with each case.
fn axial_breakdown(results: &[runner::CaseOutcome], model: &aero::AeroModel, fin_edges_in_cfd: bool) -> Vec<forces::AxialReportEntry> {
    results
        .iter()
        .filter(|r| !r.spec.is_roll())
        .filter_map(|r| {
            let a = r.result.as_ref()?.axial?;
            let m = forces::MarkerCoeffs { wall_cfx: a.wall_cfx, base_cfx: a.base_cfx, ..Default::default() };
            let mut e = forces::AxialReportEntry::new(model, r.spec.mach, r.spec.alpha_deg, &m, false, fin_edges_in_cfd);
            e.base_from_cfd = a.base_in_cfx;
            Some(e)
        })
        .collect()
}

/// Working directory of the CFD cases (`<out_dir>/<workdir>`).
pub fn work_dir(cfd: &CfdOptions, out_dir: &Path) -> std::path::PathBuf {
    let w = out_dir.join(&cfd.workdir);
    std::path::absolute(&w).unwrap_or(w)
}

/// Wall surface for the options: panel mesh or STL, clipped to the half model when `symmetry`.
pub fn prepare_surface(geom: &Geometry, cfd: &CfdOptions, raw: Option<&[geom::stl::Triangle]>) -> Result<surface::WallSurface> {
    let s = surface::build_surface(geom, cfd, raw)?;
    if cfd.symmetry {
        surface::half_surface(&s, geom.length)
    } else {
        Ok(s)
    }
}

/// Meshes the rebuilt geometry, the STL surface or the CAD B-rep according to `cfd.surface` (`raw`: the STL in metres for
/// `stl` and `step`). Returns the mesh and the options to run with: for an STL or STEP surface the symmetry is resolved
/// from the shape.
fn ensure_mesh_any(
    cfd: &CfdOptions,
    geom: &Geometry,
    raw: Option<&[geom::stl::Triangle]>,
    atm: &aero::atmosphere::Atmosphere,
    work: &Path,
    threads: usize,
) -> Result<(mesh::MeshResult, CfdOptions)> {
    match cfd.surface {
        SurfaceKind::Stl => {
            stl_mesh::ensure_mesh_stl(cfd, geom, raw.ok_or_else(|| anyhow::anyhow!("surface = \"stl\" needs the STL triangles"))?, work, threads)
        }
        SurfaceKind::Step => step_mesh::ensure_mesh_step(
            cfd,
            geom,
            raw.ok_or_else(|| anyhow::anyhow!("surface = \"step\" needs the STL triangles (the registration reference)"))?,
            atm,
            work,
            threads,
        ),
        SurfaceKind::PanelMesh => Ok((mesh::ensure_mesh(cfd, geom, atm, work, threads)?, cfd.clone())),
    }
}

/// Surface export and volume meshing only (`cfd-check --mesh`).
pub fn mesh_only(
    geom: &Geometry,
    cfd: &CfdOptions,
    aero: &AeroOptions,
    out_dir: &Path,
    budget: &ResourceBudget,
    raw: Option<&[geom::stl::Triangle]>,
) -> Result<mesh::MeshResult> {
    cfd.validate()?;
    let cfd = &resolve_options(geom, cfd);
    check_tools(cfd)?;
    let atm = aero.atmosphere.at(aero.reference_altitude);
    Ok(ensure_mesh_any(cfd, geom, raw, &atm, &work_dir(cfd, out_dir), budget.threads)?.0)
}

/// Builds the aerodynamic table with SU2: surface, mesh, case runner, table fill.
///
/// `raw` is the STL in metres, needed for `surface = "stl"` and `surface = "step"` (the registration reference).
#[allow(clippy::too_many_arguments)]
pub fn build_table_with_progress(
    geom: &Geometry,
    cfd: &CfdOptions,
    aero: &AeroOptions,
    out_dir: &Path,
    hash: String,
    extrapolation: Extrapolation,
    budget: &ResourceBudget,
    raw: Option<&[geom::stl::Triangle]>,
    progress: &(dyn Fn(CfdStage) + Sync),
) -> Result<(AeroTable, CfdReport)> {
    cfd.validate()?;
    let cfd = &resolve_options(geom, cfd);
    let tools = check_tools(cfd)?;
    let t0 = std::time::Instant::now();
    progress(CfdStage::Mesh);
    let atm = aero.atmosphere.at(aero.reference_altitude);
    let work = work_dir(cfd, out_dir);
    let (mesh, resolved) = ensure_mesh_any(cfd, geom, raw, &atm, &work, budget.threads)?;
    let cfd = &resolved;
    let dims = ref_dims(geom);
    let plan = plan_resources_mesh(cfd, budget, mesh.stats.cells, mesh.stats.nodes, case::case_list(cfd).len());
    for n in &plan.notes {
        eprintln!("note: aero.cfd: {n}");
    }
    if let Some(why) = &plan.over_budget {
        bail!("aero.cfd: {why}");
    }
    let run = runner::RunSetup {
        opt: cfd,
        dims,
        atm,
        mesh_hash: mesh.hash.clone(),
        mesh_file: mesh.su2.clone(),
        work: work.clone(),
        ranks: plan.ranks,
        parallel: plan.parallel,
        tools: &tools,
    };
    let results = runner::run_all(&run, progress)?;
    progress(CfdStage::Table);
    let model = aero::AeroModel::new(geom.clone(), aero.clone());
    let first_error = |e: anyhow::Error| match results.iter().find_map(|r| r.error.as_ref().map(|m| (r.spec, m))) {
        Some((s, m)) => anyhow::anyhow!("{e}\nfirst failed case M={} alpha={} beta={} deg ({}): {m}", s.mach, s.alpha_deg, s.beta_deg, s.dir(&work).display()),
        None => e,
    };
    // The table takes the base term from CFD when `rans_base = "cfd"` or when any RANS case carries the base cap
    // in its axial coefficient (marker columns absent: the monitored total): the Barrowman base is then not added.
    // The choice is table-wide (all cases of a run come from one SU2 configuration, so the columns are all there or all absent).
    let markers_missing = rans_markers_missing(results.iter().filter_map(|r| r.result.as_ref()));
    let base_from_cfd = table_base_from_cfd(cfd.base_from_cfd(), markers_missing);
    let base_cfd = |r: &runner::CaseOutcome| !r.spec.is_roll();
    // Roll pairs (alpha = beta = 0, +-roll_rate_hat) per Mach number.
    let roll_pairs: Vec<general::RollPair> = if cfd.roll_damping {
        let mut machs: Vec<f64> = results.iter().filter(|r| r.spec.is_roll()).map(|r| r.spec.mach).collect();
        machs.dedup();
        machs
            .into_iter()
            .map(|m| {
                let c = |sign: f64| {
                    results
                        .iter()
                        .find(|r| r.spec.is_roll() && r.spec.mach == m && r.spec.roll_hat * sign > 0.0)
                        .and_then(|r| r.result.as_ref())
                        .and_then(|r| r.coeffs6)
                };
                general::RollPair { mach: m, roll_hat: cfd.roll_rate_hat, plus: c(1.0), minus: c(-1.0) }
            })
            .collect()
    } else {
        Vec::new()
    };
    let clp = general::roll_entries(&roll_pairs, &model);
    let shape = cfd.shape();
    let mut warnings: Vec<String> = Vec::new();
    for e in clp.iter().filter(|e| e.source != "cfd") {
        warnings.push(format!("roll damping at M={}: using the Barrowman Clp ({:.3}): {}", e.mach, e.barrowman, e.reason.as_deref().unwrap_or("")));
    }
    let mut report = CfdReport {
        cases: results.len(),
        failed: results.iter().filter(|r| r.result.is_none()).map(|r| (r.spec.mach, r.spec.alpha_deg)).collect(),
        failed_cases: results.iter().filter(|r| r.result.is_none()).map(|r| r.spec.dir_name()).collect(),
        unconverged: results.iter().filter(|r| r.result.as_ref().is_some_and(|x| !x.converged)).map(|r| (r.spec.mach, r.spec.alpha_deg)).collect(),
        unconverged_cases: results.iter().filter(|r| r.result.as_ref().is_some_and(|x| !x.converged)).map(|r| r.spec.dir_name()).collect(),
        stalled_cases: results
            .iter()
            .filter_map(|r| r.result.as_ref().filter(|x| x.stalled).map(|x| StalledCase { case: r.spec.dir_name(), residual_drop: x.residual_drop }))
            .collect(),
        wall_seconds: t0.elapsed().as_secs_f64(),
        mesh_nodes: mesh.stats.nodes,
        mesh_cells: mesh.stats.cells,
        case_seconds: results.iter().filter_map(|r| r.result.as_ref().map(|x| x.wall_seconds)).sum(),
        alpha_mode: format!("{:?}", cfd.alpha_mode).to_lowercase(),
        ranks_per_case: plan.ranks,
        parallel_cases: plan.parallel,
        bytes_per_case: plan.bytes_per_case,
        est_wall_yplus: (cfd.model == FlowModel::Rans).then(|| mesh::wall_yplus(cfd, geom.length, &atm)),
        shape_symmetry: format!("{shape:?}").to_lowercase(),
        domain: if cfd.symmetry { "half" } else { "full" }.into(),
        betas_deg: cfd.betas_deg.clone(),
        clp_source: if clp.is_empty() {
            "barrowman"
        } else if clp.iter().all(|e| e.source == "cfd") {
            "cfd"
        } else if clp.iter().all(|e| e.source != "cfd") {
            "barrowman"
        } else {
            "mixed"
        }
        .into(),
        clp: clp.clone(),
        base_treatment: cfd.base_treatment.filter(|_| cfd.model == FlowModel::Euler).map(|b| format!("{b:?}").to_lowercase()),
        warnings,
        ..Default::default()
    };
    if let Some(info) = &mesh.stl {
        report.base_treatment = Some(info.base_treatment.clone());
        report.warnings.extend(info.warnings.clone());
        report.symmetry_check = serde_json::to_value(&info.symmetry).ok();
        report.fidelity = serde_json::to_value(serde_json::json!({
            "fidelity": info.fidelity,
            "union": info.union_report,
            "repair": info.repair,
        }))
        .ok();
    }
    if let Some(info) = &mesh.step {
        report.base_treatment = Some(info.base_treatment.clone());
        report.warnings.extend(info.warnings.clone());
        report.symmetry_check = serde_json::to_value(&info.symmetry).ok();
        report.step = serde_json::to_value(info).ok();
    }
    if cfd.model == FlowModel::Rans {
        report.axial_breakdown = axial_breakdown(&results, &model, cfd.surface_is_user_shape());
        if report.axial_breakdown.iter().any(|e| e.cfd_wall_ca.is_some()) && markers_missing {
            report.warnings.push("RANS: some cases have no per-marker CFx columns; the table takes the base from CFD for all of them".into());
        }
        if report.axial_breakdown.iter().any(|e| e.cfd_wall_ca.is_none()) {
            report
                .warnings
                .push("RANS: the history has no per-marker CFx(wall) column; the axial force is the monitored total, which includes the base cap".into());
        }
    }
    let tab = if cfd.general_table() {
        let points: Vec<general::SolvedPoint6> = results
            .iter()
            .filter(|r| base_cfd(r))
            .map(|r| general::SolvedPoint6 {
                mach: r.spec.mach,
                alpha_deg: r.spec.alpha_deg,
                beta_deg: r.spec.beta_deg,
                coeffs: r.result.as_ref().and_then(|x| x.coeffs6),
            })
            .collect();
        // The rebuilt geometry has no fin cant, so its roll forcing is the model's: reference cant 0. The STL and the
        // STEP contain the cant of the real fins.
        let cant_ref = if cfd.surface_is_user_shape() { geom.fins.as_ref().map_or(0.0, |f| f.cant_deg) } else { 0.0 };
        let inp = general::GeneralInput {
            points: &points,
            roll: &roll_pairs,
            model: &model,
            flow: cfd.model,
            source_hash: hash,
            extrapolation,
            shape,
            average: cfd.alpha_mode == AlphaMode::Mirror,
            offset: cfd.alpha_mode == AlphaMode::Offset,
            mach_step: cfd.general_mach_step,
            fin_cant_ref_deg: cant_ref,
            fin_edges_in_cfd: cfd.surface_is_user_shape(),
            base_from_cfd,
        };
        let (tab, fill) = general::build_general(&inp).map_err(first_error)?;
        runner::write_cases_csv(&out_dir.join("cfd_cases.csv"), &results, &[], AlphaMode::Single)?;
        report.table_kind = "general".into();
        report.filled = fill.filled.iter().map(|f| (f.0, f.1)).collect();
        report.cfd_region = Some(fill.region);
        report.model_outside_cfd_region = true;
        report.zero_angle = asymmetry::from_general(&points, &fill.slopes, shape, cfd.alpha_mode);
        report.diagnostics = fill.slopes;
        report.yaw_damping_from_pitch = fill.yaw_from_pitch;
        report.fin_cant_ref_deg = cant_ref;
        tab
    } else {
        let points: Vec<table::SolvedPoint> = results
            .iter()
            .filter(|r| base_cfd(r))
            .map(|r| table::SolvedPoint { mach: r.spec.mach, alpha_deg: r.spec.alpha_deg, coeffs: r.result.as_ref().map(|x| x.coeffs) })
            .collect();
        let (points, asymmetry) = table::apply_alpha_mode(&points, cfd.alpha_mode);
        let (tab, fill) = table::build_from_points_roll(&points, &model, cfd.model, hash, extrapolation, &clp, base_from_cfd, cfd.surface_is_user_shape())
            .map_err(first_error)?;
        runner::write_cases_csv(&out_dir.join("cfd_cases.csv"), &results, &points, cfd.alpha_mode)?;
        report.table_kind = "axisymmetric".into();
        report.filled = fill.filled;
        report.asymmetry = asymmetry;
        report.diagnostics =
            fill.slopes.iter().map(|&(mach, cna, xcp)| general::Slopes { mach, cna_pitch: cna, xcp_pitch: xcp, cna_yaw: None, xcp_yaw: None }).collect();
        report.zero_angle = asymmetry::from_axisymmetric(&report.asymmetry, &report.diagnostics, cfd.alpha_mode);
        report.fin_cant_ref_deg = tab.meta.fin_cant_ref_deg;
        tab
    };
    report.warnings.extend(asymmetry::warnings(&report.zero_angle, geom.fins.as_ref().map(|f| (cfd.wall_size, f.thickness)), cfd.model == FlowModel::Rans));
    let solved_m = results.iter().filter(|r| r.result.is_some()).map(|r| r.spec.mach);
    let (lo, hi) = solved_m.fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), m| (a.min(m), b.max(m)));
    if lo <= hi {
        let eps = 1e-9;
        report.mach_clamped_above = tab.meta.machs.last().is_some_and(|&m| m > hi + eps).then_some(hi);
        report.mach_clamped_below = tab.meta.machs.first().is_some_and(|&m| m < lo - eps).then_some(lo);
        if let Some(m) = report.mach_clamped_above {
            report.warnings.push(format!("the table is held at its M = {m} values above M = {m} (no Mach extrapolation of the CFD data)"));
        }
        if let Some(m) = report.mach_clamped_below {
            report.warnings.push(format!("the table is held at its M = {m} values below M = {m} (no Mach extrapolation of the CFD data)"));
        }
    }
    report.warnings.extend(stalled_warning(&report.stalled_cases));
    progress(CfdStage::Done);
    Ok((tab, report))
}

#[cfg(test)]
mod tests {
    use super::*;
    use geom::sample::SampleRocket;
    use geom::{extract, ExtractOptions};

    fn opts(r: usize, p: usize) -> CfdOptions {
        CfdOptions { ranks_per_case: r, parallel_cases: p, ..Default::default() }
    }

    /// [`plan_resources_with`] without a detected memory limit (thread tests independent of the machine).
    fn plan(o: &CfdOptions, b: &ResourceBudget, cells: usize, n: usize) -> RunPlan {
        plan_resources_with(o, b, cells, 0, n, None)
    }

    #[test]
    fn resource_plan_caps_threads_and_resolves_auto() {
        let b = |t, m| ResourceBudget { threads: t, memory_bytes: m };
        let p = plan(&opts(4, 4), &b(20, None), 100_000, 12);
        assert_eq!((p.ranks, p.parallel), (4, 4));
        assert!(p.notes.is_empty() && p.memory.is_none() && p.over_budget.is_none());
        // parallel is reduced first, then ranks
        let p = plan(&opts(4, 4), &b(10, None), 100_000, 12);
        assert_eq!((p.ranks, p.parallel), (4, 2));
        let p = plan(&opts(4, 4), &b(3, None), 100_000, 12);
        assert_eq!((p.ranks, p.parallel), (3, 1));
        // auto
        let p = plan(&opts(0, 0), &b(20, None), 100_000, 12);
        assert_eq!((p.ranks, p.parallel), (4, 5));
        let p = plan(&opts(0, 0), &b(2, None), 100_000, 12);
        assert_eq!((p.ranks, p.parallel), (2, 1));
        let p = plan(&opts(2, 0), &b(8, None), 100_000, 3);
        assert_eq!((p.ranks, p.parallel), (2, 3), "never more than the number of cases");
    }

    #[test]
    fn resource_plan_respects_memory() {
        let b = ResourceBudget { threads: 20, memory_bytes: Some(2_000_000_000) };
        // 1 M cells, 4 ranks: about 3.5 GB per case -> one case, refused.
        let p = plan_resources(&opts(4, 4), &b, 1_000_000, 12);
        assert_eq!(p.parallel, 1);
        let why = p.over_budget.as_deref().expect("over budget");
        assert!(why.starts_with("a single CFD case needs about 3.5 GB (Euler on 1000000 cells") && why.contains("4 ranks"), "{why}");
        assert!(
            why.contains("memory budget of 2.0 GB (resources.memory_gb)") && why.contains("larger wall_size") && why.contains("raise resources.memory_gb"),
            "{why}"
        );
        // 100 k cells: about 0.49 GB per case -> 4 fit into 2.2 GB.
        let p = plan_resources(&opts(4, 8), &ResourceBudget { threads: 40, memory_bytes: Some(2_200_000_000) }, 100_000, 12);
        assert_eq!(p.parallel, 4, "{p:?}");
        assert!(p.over_budget.is_none() && p.notes.iter().any(|n| n.contains("parallel_cases reduced from 8 to 4 (memory budget 2.2 GB")), "{p:?}");
        assert!(case_memory_bytes(FlowModel::Rans, 100_000, 4) >= case_memory_bytes(FlowModel::Euler, 100_000, 4));
    }

    #[test]
    fn stalled_cases_are_named_with_their_residual_drop() {
        assert_eq!(stalled_warning(&[]), None);
        let w = stalled_warning(&[StalledCase { case: "m2.000_a00.00".into(), residual_drop: Some(0.94) }]).unwrap();
        assert!(w.starts_with("1 RANS case accepted with a stalled residual, not converged"), "{w}");
        assert!(w.contains("needs only 0.5 orders below the reference, convergence 3") && w.contains("m2.000_a00.00 (0.9 orders below the reference)"), "{w}");
    }

    #[test]
    fn resource_plan_defaults_to_the_available_memory() {
        // The default budget (resources.memory_gb = 0) is the memory available to the process: the default RANS
        // setup (4 ranks x 4 cases, 735 k nodes at 5 mm: about 9 GB per case) on a machine with 30 GB available runs
        // 3 cases at a time instead of 4.
        let rans = CfdOptions { ranks_per_case: 4, parallel_cases: 4, ..CfdOptions::rans() };
        let avail = |gb: f64| Some(panel::memory::Available { bytes: (gb * 1e9) as u64, source: "available memory, MemAvailable in /proc/meminfo".into() });
        let b = ResourceBudget { threads: 16, memory_bytes: None };
        let p = plan_resources_with(&rans, &b, 1_000_000, 735_366, 20, avail(30.0));
        assert!((p.bytes_per_case as f64 / 1e9 - 8.98).abs() < 0.05, "{p:?}");
        assert_eq!(p.parallel, 3, "{p:?}");
        assert_eq!(p.memory.as_ref().map(|m| m.1.as_str()), Some("available memory, MemAvailable in /proc/meminfo"));
        assert!(p.notes.iter().any(|n| n.contains("memory budget 30.0 GB from available memory")), "{p:?}");
        // Less than one case: refused, with the estimate and what to change.
        let p = plan_resources_with(&rans, &b, 1_000_000, 735_366, 20, avail(8.0));
        let why = p.over_budget.expect("over budget");
        assert!(why.contains("about 9.0 GB (RANS on 1000000 cells / 735366 nodes, 4 ranks)") && why.contains("RANS default 0.005 m"), "{why}");
        assert!(why.contains("set resources.memory_gb") && why.contains("MemAvailable"), "{why}");
        // An explicit budget overrides the detected memory.
        let p = plan_resources_with(&rans, &ResourceBudget { memory_bytes: Some(40_000_000_000), ..b }, 1_000_000, 735_366, 20, avail(8.0));
        assert!(p.over_budget.is_none() && p.parallel == 4 && p.memory.as_ref().unwrap().1 == "resources.memory_gb", "{p:?}");
        // Nothing known (other systems): no memory check.
        let p = plan_resources_with(&rans, &b, 1_000_000, 735_366, 20, None);
        assert!(p.memory.is_none() && p.over_budget.is_none() && p.parallel == 4);
        // plan_resources_mesh detects the memory itself when no budget is set.
        let p = plan_resources_mesh(&rans, &b, 1_000_000, 735_366, 20);
        assert_eq!(p.memory.is_some(), panel::memory::available().is_some());
    }

    #[test]
    fn automatic_options_are_resolved_from_the_layout() {
        let g = extract(&SampleRocket::default().mesh(), &ExtractOptions::default()).unwrap();
        let d = resolve_options(&g, &CfdOptions::default());
        // Default configuration: half model, axisymmetric table, quarter mesh (unchanged behaviour).
        assert!(d.symmetry && d.z_mirror_mesh && d.shape() == ShapeSymmetry::Axisymmetric && !d.general_table());
        // Sideslip, roll cases or symmetry = false need the full domain.
        for o in [
            CfdOptions { betas_deg: vec![0.0, 4.0], ..Default::default() },
            CfdOptions { roll_damping: true, ..Default::default() },
            CfdOptions { symmetry: false, ..Default::default() },
        ] {
            let r = resolve_options(&g, &o);
            assert!(!r.symmetry && r.z_mirror_mesh, "{r:?}");
        }
        // The RANS boundary-layer mesh is not z-symmetric: the -alpha cases are solved.
        let r = resolve_options(&g, &CfdOptions { model: FlowModel::Rans, ..Default::default() });
        assert!(r.symmetry && !r.z_mirror_mesh, "{r:?}");
        // The fin layout decides the shape symmetry when sideslip is solved.
        let r = resolve_options(&g, &CfdOptions { betas_deg: vec![0.0, 4.0], fin_roll_deg: 10.0, ..Default::default() });
        assert_eq!((r.shape(), r.z_mirror_mesh, r.symmetry), (ShapeSymmetry::None, false, false));
        let three = extract(&SampleRocket { fin_count: 3, ..Default::default() }.mesh(), &ExtractOptions::default()).unwrap();
        let r = resolve_options(&three, &CfdOptions { betas_deg: vec![0.0, 4.0], ..Default::default() });
        assert_eq!((r.shape(), r.z_mirror_mesh), (ShapeSymmetry::PitchPlane, false));
        // An explicit choice wins; an STL surface keeps its options until the united surface has been checked
        // (see `symmetry`), and `shape()` is `none` until then.
        let r = resolve_options(&g, &CfdOptions { surface: SurfaceKind::Stl, ..Default::default() });
        assert_eq!((r.shape(), r.symmetry, r.z_mirror_mesh), (ShapeSymmetry::None, true, true));
        let r = resolve_options(&g, &CfdOptions { shape_symmetry: Some(ShapeSymmetry::PitchPlane), ..Default::default() });
        assert_eq!(r.shape(), ShapeSymmetry::PitchPlane);
    }

    #[test]
    fn missing_tools_give_clear_error() {
        let g = extract(&SampleRocket::default().mesh(), &ExtractOptions::default()).unwrap();
        let o = CfdOptions { su2: "no_such_su2_cfd".into(), ..Default::default() };
        let e = build_table_with_progress(
            &g,
            &o,
            &AeroOptions::default(),
            Path::new("."),
            "h".into(),
            Extrapolation::Linear,
            &ResourceBudget::default(),
            None,
            &|_| {},
        )
        .err()
        .unwrap()
        .to_string();
        assert!(e.starts_with("SU2 is not installed (install with `install.sh --cfd`"), "{e}");
        assert!(e.contains("no_such_su2_cfd"));
    }

    #[test]
    fn memory_estimate_per_node_for_rans() {
        // Euler: 40 MB per rank + 3.3 kB per cell, unchanged.
        assert_eq!(case_memory_bytes(FlowModel::Euler, 100_000, 4), (4.0 * 40e6 + 3.3e3 * 100_000.0) as u64);
        // RANS of a tetrahedral mesh without a node count (5.5 cells per node): not below the Euler figure.
        let tet = case_memory_bytes(FlowModel::Rans, 550_000, 4) as f64;
        assert!((tet - (4.0 * 40e6 + (RANS_BYTES_PER_NODE * 100_000.0).max(3.3e3 * 550_000.0))).abs() < 1e5);
        // A prism boundary-layer mesh (one node per cell, here 100 k cells and 100 k nodes) is sized by its nodes.
        let prism = case_memory_bytes_mesh(FlowModel::Rans, 100_000, 100_000, 4);
        assert!(prism > case_memory_bytes_mesh(FlowModel::Rans, 100_000, 0, 4));
        assert_eq!(prism, (4.0 * 40e6 + RANS_BYTES_PER_NODE * 100_000.0) as u64);
        // The node count does not change the Euler estimate.
        assert_eq!(case_memory_bytes_mesh(FlowModel::Euler, 100_000, 7, 4), case_memory_bytes(FlowModel::Euler, 100_000, 4));
        assert_eq!(estimate_nodes(550_000), 100_000);
    }

    #[test]
    fn rans_report_axial_breakdown_serde_default() {
        let r: forces::AxialReportEntry = serde_json::from_str("{}").unwrap();
        assert_eq!(r, forces::AxialReportEntry::default());
        assert!(serde_json::to_value(CfdReport::default()).unwrap()["axial_breakdown"].as_array().unwrap().is_empty());
    }
}
