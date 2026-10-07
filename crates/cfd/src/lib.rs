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

pub mod case;
pub mod config;
pub mod fidelity;
pub mod forces;
pub mod general;
pub mod history;
pub mod mesh;
pub mod runner;
pub mod stl_mesh;
pub mod su2cfg;
pub mod surface;
pub mod symmetry;
pub mod table;
pub mod tools;

pub use config::{y_symmetric_layout, z_symmetric_layout, AlphaMode, BaseTreatment, CfdOptions, FlowModel, Scheme, ShapeSymmetry, StlRemesh, SurfaceKind};

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

/// Summary written to `cfd_report.json`.
#[derive(Debug, Clone, Default, Serialize)]
pub struct CfdReport {
    pub cases: usize,
    pub failed: Vec<(f64, f64)>,
    pub unconverged: Vec<(f64, f64)>,
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
    /// STL surface: mirror-symmetry check of the united surface and the domain chosen from it.
    pub symmetry_check: Option<serde_json::Value>,
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
/// * `shape_symmetry`: `auto` becomes `none` for an STL surface; for the rebuilt geometry it is
///   `axisymmetric` (the table is roll-averaged) unless sideslip is solved, where the fin layout decides
///   (`axisymmetric`: symmetric about y = 0 and z = 0, `pitch_plane`: about y = 0 only, else `none`).
///   For an STL surface it stays as configured: the symmetry is resolved from the united surface by
///   [`stl_mesh::ensure_mesh_stl`] ([`symmetry::decide`]).
/// * `symmetry` (the SU2 half domain) is kept only with a layout symmetric about y = 0 (rebuilt geometry), a shape
///   symmetry that includes y = 0, `betas_deg = [0]` and no roll cases; otherwise the full domain is solved (the
///   mesher mirrors the half mesh when the layout is symmetric, else it uses a box farfield).
/// * `z_mirror_mesh` stays on only for layouts symmetric about both planes.
pub fn resolve_options(geom: &Geometry, cfd: &CfdOptions) -> CfdOptions {
    let mut o = cfd.clone();
    let stl = o.surface == SurfaceKind::Stl;
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
        o.z_mirror_mesh = o.z_mirror_mesh && ysym && zsym;
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
    /// Memory budget for the concurrent SU2 runs; `None` = do not limit.
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
        FlowModel::Rans => RANS_BYTES_PER_NODE * if nodes > 0 { nodes } else { estimate_nodes(cells) } as f64,
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
/// RANS bytes per mesh node (summed over ranks): the Euler per-cell figure scaled by 1.5 for the RANS
/// state and 5.5 cells per node of a tetrahedral mesh. It is refit from measured peak RSS of RANS runs.
pub const RANS_BYTES_PER_NODE: f64 = EULER_BYTES_PER_CELL * 1.5 * 5.5;

/// Resolves `ranks_per_case` / `parallel_cases` (0 = auto) against the budget.
///
/// * auto ranks: `min(4, threads)`; auto parallel: `threads / ranks`.
/// * `ranks x parallel` is capped to `threads`: `parallel` is reduced first, then `ranks`.
/// * with a memory budget, `parallel` is reduced until the estimated memory of the concurrent cases
///   fits (at least one case always runs; a note says when even one exceeds the budget).
/// * `parallel` never exceeds the number of cases.
pub fn plan_resources(opt: &CfdOptions, budget: &ResourceBudget, cells: usize, n_cases: usize) -> RunPlan {
    plan_resources_mesh(opt, budget, cells, 0, n_cases)
}

/// [`plan_resources`] with the node count of the mesh (0 = estimate from `cells`).
pub fn plan_resources_mesh(opt: &CfdOptions, budget: &ResourceBudget, cells: usize, nodes: usize, n_cases: usize) -> RunPlan {
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
    if let Some(mem) = budget.memory_bytes {
        let fit = ((mem / per.max(1)) as usize).max(1);
        if fit < parallel {
            notes.push(format!("parallel_cases reduced from {parallel} to {fit} (memory budget, about {:.1} GB per case)", per as f64 / 1e9));
            parallel = fit;
        }
        if per > mem {
            notes.push(format!("a single case needs about {:.1} GB, more than the memory budget {:.1} GB", per as f64 / 1e9, mem as f64 / 1e9));
        }
    }
    RunPlan { ranks, parallel, bytes_per_case: per, notes }
}

/// Per-case axial breakdown of the RANS report, read back from the case histories.
fn axial_breakdown(results: &[runner::CaseOutcome], model: &aero::AeroModel, work: &Path, base_from_cfd: bool) -> Vec<forces::AxialReportEntry> {
    results
        .iter()
        .filter(|r| r.result.is_some() && !r.spec.is_roll())
        .filter_map(|r| {
            let h = history::History::read(&r.spec.dir(work).join("history.csv")).ok()?;
            let window = (h.rows.len() / 4).clamp(2, 100);
            let m = forces::MarkerCoeffs::from_history(&h, window);
            Some(forces::AxialReportEntry::new(model, r.spec.mach, r.spec.alpha_deg, &m, base_from_cfd))
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

/// Meshes the rebuilt geometry or the STL surface according to `cfd.surface` (`raw` in metres for the STL). Returns
/// the mesh and the options to run with: for an STL surface the symmetry is resolved from the united surface.
fn ensure_mesh_any(
    cfd: &CfdOptions,
    geom: &Geometry,
    raw: Option<&[geom::stl::Triangle]>,
    work: &Path,
    threads: usize,
) -> Result<(mesh::MeshResult, CfdOptions)> {
    match cfd.surface {
        SurfaceKind::Stl => {
            stl_mesh::ensure_mesh_stl(cfd, geom, raw.ok_or_else(|| anyhow::anyhow!("surface = \"stl\" needs the STL triangles"))?, work, threads)
        }
        SurfaceKind::PanelMesh => Ok((mesh::ensure_mesh(cfd, geom, work, threads)?, cfd.clone())),
    }
}

/// Surface export and volume meshing only (`cfd-check --mesh`).
pub fn mesh_only(
    geom: &Geometry,
    cfd: &CfdOptions,
    _aero: &AeroOptions,
    out_dir: &Path,
    budget: &ResourceBudget,
    raw: Option<&[geom::stl::Triangle]>,
) -> Result<mesh::MeshResult> {
    cfd.validate()?;
    let cfd = &resolve_options(geom, cfd);
    check_tools(cfd)?;
    Ok(ensure_mesh_any(cfd, geom, raw, &work_dir(cfd, out_dir), budget.threads)?.0)
}

/// Builds the aerodynamic table with SU2: surface, mesh, case runner, table fill.
///
/// `raw` is the STL in metres, needed for `surface = "stl"`.
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
    let (mesh, resolved) = ensure_mesh_any(cfd, geom, raw, &work, budget.threads)?;
    let cfd = &resolved;
    let dims = ref_dims(geom);
    let plan = plan_resources_mesh(cfd, budget, mesh.stats.cells, mesh.stats.nodes, case::case_list(cfd).len());
    for n in &plan.notes {
        eprintln!("note: aero.cfd: {n}");
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
    // One-line switch: `cfd.base_from_cfd()` once the option exists.
    let base_from_cfd = forces::BASE_FROM_CFD_DEFAULT;
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
    if cfd.model == FlowModel::Rans {
        report.axial_breakdown = axial_breakdown(&results, &model, &work, base_from_cfd);
        if report.axial_breakdown.iter().any(|e| e.cfd_wall_ca.is_none()) {
            report.warnings.push("RANS: the history has no per-marker CFx(wall) column; the axial force is the monitored total, which includes the base cap".into());
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
        // The rebuilt geometry has no fin cant, so its roll forcing is the model's: reference cant 0.
        let cant_ref = if cfd.surface == SurfaceKind::Stl { geom.fins.as_ref().map_or(0.0, |f| f.cant_deg) } else { 0.0 };
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
            fin_edges_in_cfd: cfd.surface == SurfaceKind::Stl,
            base_from_cfd,
        };
        let (tab, fill) = general::build_general(&inp).map_err(first_error)?;
        runner::write_cases_csv(&out_dir.join("cfd_cases.csv"), &results, &[], AlphaMode::Single)?;
        report.table_kind = "general".into();
        report.filled = fill.filled.iter().map(|f| (f.0, f.1)).collect();
        report.cfd_region = Some(fill.region);
        report.model_outside_cfd_region = true;
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
        let (tab, fill) = table::build_from_points_roll(&points, &model, cfd.model, hash, extrapolation, &clp, base_from_cfd).map_err(first_error)?;
        runner::write_cases_csv(&out_dir.join("cfd_cases.csv"), &results, &points, cfd.alpha_mode)?;
        report.table_kind = "axisymmetric".into();
        report.filled = fill.filled;
        report.asymmetry = asymmetry;
        report.diagnostics =
            fill.slopes.iter().map(|&(mach, cna, xcp)| general::Slopes { mach, cna_pitch: cna, xcp_pitch: xcp, cna_yaw: None, xcp_yaw: None }).collect();
        report.fin_cant_ref_deg = tab.meta.fin_cant_ref_deg;
        tab
    };
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

    #[test]
    fn resource_plan_caps_threads_and_resolves_auto() {
        let b = |t, m| ResourceBudget { threads: t, memory_bytes: m };
        let p = plan_resources(&opts(4, 4), &b(20, None), 100_000, 12);
        assert_eq!((p.ranks, p.parallel), (4, 4));
        assert!(p.notes.is_empty());
        // parallel is reduced first, then ranks
        let p = plan_resources(&opts(4, 4), &b(10, None), 100_000, 12);
        assert_eq!((p.ranks, p.parallel), (4, 2));
        let p = plan_resources(&opts(4, 4), &b(3, None), 100_000, 12);
        assert_eq!((p.ranks, p.parallel), (3, 1));
        // auto
        let p = plan_resources(&opts(0, 0), &b(20, None), 100_000, 12);
        assert_eq!((p.ranks, p.parallel), (4, 5));
        let p = plan_resources(&opts(0, 0), &b(2, None), 100_000, 12);
        assert_eq!((p.ranks, p.parallel), (2, 1));
        let p = plan_resources(&opts(2, 0), &b(8, None), 100_000, 3);
        assert_eq!((p.ranks, p.parallel), (2, 3), "never more than the number of cases");
    }

    #[test]
    fn resource_plan_respects_memory() {
        let b = ResourceBudget { threads: 20, memory_bytes: Some(2_000_000_000) };
        // 1 M cells, 4 ranks: about 2.9 GB per case -> one case, with a note.
        let p = plan_resources(&opts(4, 4), &b, 1_000_000, 12);
        assert_eq!(p.parallel, 1);
        assert!(p.notes.iter().any(|n| n.contains("single case")));
        // 100 k cells: about 0.49 GB per case -> 4 fit into 2.2 GB.
        let p = plan_resources(&opts(4, 8), &ResourceBudget { threads: 40, memory_bytes: Some(2_200_000_000) }, 100_000, 12);
        assert_eq!(p.parallel, 4, "{p:?}");
        assert!(case_memory_bytes(FlowModel::Rans, 100_000, 4) > case_memory_bytes(FlowModel::Euler, 100_000, 4));
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
        // RANS of a tetrahedral mesh: 5.5 cells per node reproduces 1.5 x the Euler per-cell figure.
        let tet = case_memory_bytes(FlowModel::Rans, 550_000, 4) as f64;
        assert!((tet - (4.0 * 40e6 + 1.5 * 3.3e3 * 550_000.0)).abs() < 1e5);
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
