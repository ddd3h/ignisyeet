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
pub mod forces;
pub mod history;
pub mod mesh;
pub mod runner;
pub mod surface;
pub mod su2cfg;
pub mod table;
pub mod tools;

pub use config::{CfdOptions, FlowModel, Scheme, SurfaceKind};

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
    Case { index: usize, total: usize, mach: f64, alpha: f64, iter: usize, residual: f64 },
    CaseDone { index: usize, total: usize, mach: f64, alpha: f64, converged: bool },
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
    /// Removed alpha = 0 offsets per Mach: `(mach, CN0, nose moment0 [m])`, see [`table::remove_zero_offset`].
    pub zero_offsets: Vec<(f64, f64, f64)>,
    /// Resolved MPI ranks per case, concurrent cases and the estimated memory per case [bytes].
    pub ranks_per_case: usize,
    pub parallel_cases: usize,
    pub bytes_per_case: u64,
    /// RANS: estimated y+ of the first cell centre of the wall mesh (see [`mesh::wall_yplus`]).
    pub est_wall_yplus: Option<f64>,
}

/// Message of the missing-tools error.
pub fn not_installed_message(report: &tools::ToolReport, opt: &CfdOptions) -> String {
    let lines: Vec<String> = report.missing(opt).iter().map(|t| format!("  - {}: {}", t.name, t.problem.as_deref().unwrap_or("unusable"))).collect();
    format!("SU2 is not installed (see doc / cfd/install.sh)\n{}", lines.join("\n"))
}

/// Fails with the "not installed" error unless SU2, MPI (for more than one rank) and gmsh work.
pub fn check_tools(opt: &CfdOptions) -> Result<tools::ToolReport> {
    let r = tools::check_tools(opt, &[]);
    if !r.ready(opt) {
        bail!("{}", not_installed_message(&r, opt));
    }
    Ok(r)
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

/// Estimated SU2 memory of one case with `cells` volume cells on `ranks` ranks.
///
/// Measured (SU2 8.3, Euler, Roe + MUSCL, implicit FGMRES/ILU, tetrahedra, 4 ranks, peak RSS per
/// rank from `/usr/bin/time -v`): 87 MB at 80 k cells and 165 MB at 197 k cells, i.e. about 40 MB
/// per rank plus 2.7 kB per cell (summed over ranks). RANS carries the turbulence variable, its
/// gradients and a larger linear system: [`RANS_MEMORY_FACTOR`] times the per-cell part.
pub fn case_memory_bytes(model: FlowModel, cells: usize, ranks: usize) -> u64 {
    let per_cell = match model {
        FlowModel::Euler => EULER_BYTES_PER_CELL,
        FlowModel::Rans => EULER_BYTES_PER_CELL * RANS_MEMORY_FACTOR,
    };
    (ranks as f64 * 40e6 + per_cell * cells as f64) as u64
}

pub const EULER_BYTES_PER_CELL: f64 = 2.7e3;
pub const RANS_MEMORY_FACTOR: f64 = 1.5;

/// Resolves `ranks_per_case` / `parallel_cases` (0 = auto) against the budget.
///
/// * auto ranks: `min(4, threads)`; auto parallel: `threads / ranks`.
/// * `ranks x parallel` is capped to `threads`: `parallel` is reduced first, then `ranks`.
/// * with a memory budget, `parallel` is reduced until the estimated memory of the concurrent cases
///   fits (at least one case always runs; a note says when even one exceeds the budget).
/// * `parallel` never exceeds the number of cases.
pub fn plan_resources(opt: &CfdOptions, budget: &ResourceBudget, cells: usize, n_cases: usize) -> RunPlan {
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
    let per = case_memory_bytes(opt.model, cells, ranks);
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

/// `surface = "stl"` is refused: the OpenCASCADE mesher rebuilds the geometry from the extracted
/// body profile and fin set, and a raw STL cannot be united or remeshed reliably (the sample STL
/// models its fins as closed boxes embedded in the body, i.e. separate overlapping solids).
fn reject_stl(cfd: &CfdOptions, _raw: Option<&[geom::stl::Triangle]>) -> Result<()> {
    if !cfd.symmetry {
        bail!(
            "aero.cfd.symmetry = false is not supported: gmsh/HXT fails to tetrahedralise the full (periodic) domain. \
             The flow is mirror symmetric about the pitch plane for the supported layouts, so the half model is exact; use symmetry = true."
        );
    }
    if cfd.surface == SurfaceKind::Stl {
        bail!(
            "aero.cfd.surface = \"stl\" is not supported: STL triangulations cannot be united (fins as separate closed solids) \
             or remeshed reliably, so the CFD geometry is rebuilt from the extracted body profile and fins. \
             Use surface = \"panel_mesh\" (the default)."
        );
    }
    Ok(())
}

/// Surface export and volume meshing only (`cfd-check --mesh`).
pub fn mesh_only(geom: &Geometry, cfd: &CfdOptions, _aero: &AeroOptions, out_dir: &Path, budget: &ResourceBudget, raw: Option<&[geom::stl::Triangle]>) -> Result<mesh::MeshResult> {
    cfd.validate()?;
    check_tools(cfd)?;
    reject_stl(cfd, raw)?;
    mesh::ensure_mesh(cfd, geom, &work_dir(cfd, out_dir), budget.threads)
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
    let tools = check_tools(cfd)?;
    let t0 = std::time::Instant::now();
    progress(CfdStage::Mesh);
    reject_stl(cfd, raw)?;
    let atm = aero.atmosphere.at(aero.reference_altitude);
    let work = work_dir(cfd, out_dir);
    let mesh = mesh::ensure_mesh(cfd, geom, &work, budget.threads)?;
    let dims = ref_dims(geom);
    let plan = plan_resources(cfd, budget, mesh.stats.cells, case::case_list(cfd).len());
    for n in &plan.notes {
        eprintln!("note: aero.cfd: {n}");
    }
    let run = runner::RunSetup { opt: cfd, dims, atm, mesh_hash: mesh.hash.clone(), mesh_file: mesh.su2.clone(), work: work.clone(), ranks: plan.ranks, parallel: plan.parallel, tools: &tools };
    let results = runner::run_all(&run, progress)?;
    progress(CfdStage::Table);
    let mut points: Vec<table::SolvedPoint> = results
        .iter()
        .map(|r| table::SolvedPoint { mach: r.spec.mach, alpha_deg: r.spec.alpha_deg, coeffs: r.result.as_ref().map(|x| x.coeffs) })
        .collect();
    let offsets = table::remove_zero_offset(&mut points);
    let model = aero::AeroModel::new(geom.clone(), aero.clone());
    let (tab, fill) = table::build_from_points(&points, &model, cfd.model, hash, extrapolation).map_err(|e| {
        match results.iter().find_map(|r| r.error.as_ref().map(|m| (r.spec, m))) {
            Some((s, m)) => anyhow::anyhow!("{e}\nfirst failed case M={} alpha={} deg ({}): {m}", s.mach, s.alpha_deg, s.dir(&work).display()),
            None => e,
        }
    })?;
    runner::write_cases_csv(&out_dir.join("cfd_cases.csv"), &results, &dims)?;
    let report = CfdReport {
        cases: results.len(),
        failed: results.iter().filter(|r| r.result.is_none()).map(|r| (r.spec.mach, r.spec.alpha_deg)).collect(),
        unconverged: results.iter().filter(|r| r.result.as_ref().is_some_and(|x| !x.converged)).map(|r| (r.spec.mach, r.spec.alpha_deg)).collect(),
        wall_seconds: t0.elapsed().as_secs_f64(),
        mesh_nodes: mesh.stats.nodes,
        mesh_cells: mesh.stats.cells,
        case_seconds: results.iter().filter_map(|r| r.result.as_ref().map(|x| x.wall_seconds)).sum(),
        filled: fill.filled,
        zero_offsets: offsets,
        ranks_per_case: plan.ranks,
        parallel_cases: plan.parallel,
        bytes_per_case: plan.bytes_per_case,
        est_wall_yplus: (cfd.model == FlowModel::Rans).then(|| mesh::wall_yplus(cfd, geom.length, &atm)),
    };
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
        // 100 k cells: about 0.43 GB per case -> 4 fit into 2 GB.
        let p = plan_resources(&opts(4, 8), &ResourceBudget { threads: 40, ..b }, 100_000, 12);
        assert_eq!(p.parallel, 4, "{p:?}");
        assert!(case_memory_bytes(FlowModel::Rans, 100_000, 4) > case_memory_bytes(FlowModel::Euler, 100_000, 4));
    }

    #[test]
    fn missing_tools_give_clear_error() {
        let g = extract(&SampleRocket::default().mesh(), &ExtractOptions::default()).unwrap();
        let o = CfdOptions { su2: "no_such_su2_cfd".into(), ..Default::default() };
        let e = build_table_with_progress(&g, &o, &AeroOptions::default(), Path::new("."), "h".into(), Extrapolation::Linear, &ResourceBudget::default(), None, &|_| {})
            .err()
            .unwrap()
            .to_string();
        assert!(e.starts_with("SU2 is not installed (see doc / cfd/install.sh)"), "{e}");
        assert!(e.contains("no_such_su2_cfd"));
    }
}
