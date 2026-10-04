// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025-2026 西濱大将 (NISHIHAMA Daisuke)
//! CFD aerodynamics mode: wall surface, SU2 cases, post-processing and table assembly.
//!
//! Step 1 (this version) provides everything that works without SU2 / gmsh: options, SU2
//! configuration text, history parsing, force conversion, table filling, surface export and the
//! case bookkeeping. Meshing and the case runner are added later; [`build_table_with_progress`]
//! already checks for the tools and fails cleanly when they are missing.
//!
//! Frames and sign conventions are documented in [`su2cfg`] and [`forces`].

pub mod case;
pub mod config;
pub mod forces;
pub mod history;
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

/// Builds the aerodynamic table with SU2.
///
/// The meshing and runner stages are placeholders in this version: after the tool check they
/// return an error.
#[allow(clippy::too_many_arguments)]
pub fn build_table_with_progress(
    geom: &Geometry,
    cfd: &CfdOptions,
    aero: &AeroOptions,
    out_dir: &Path,
    hash: String,
    extrapolation: Extrapolation,
    progress: &(dyn Fn(CfdStage) + Sync),
) -> Result<(AeroTable, CfdReport)> {
    cfd.validate()?;
    check_tools(cfd)?;
    let _ = (geom, aero, out_dir, hash, extrapolation);
    progress(CfdStage::Mesh);
    bail!("the CFD meshing and case runner are not implemented yet in this version")
}

#[cfg(test)]
mod tests {
    use super::*;
    use geom::sample::SampleRocket;
    use geom::{extract, ExtractOptions};

    #[test]
    fn missing_tools_give_clear_error() {
        let g = extract(&SampleRocket::default().mesh(), &ExtractOptions::default()).unwrap();
        let o = CfdOptions { su2: "no_such_su2_cfd".into(), ..Default::default() };
        let e = build_table_with_progress(&g, &o, &AeroOptions::default(), Path::new("."), "h".into(), Extrapolation::Linear, &|_| {})
            .err()
            .unwrap()
            .to_string();
        assert!(e.starts_with("SU2 is not installed (see doc / cfd/install.sh)"), "{e}");
        assert!(e.contains("no_such_su2_cfd"));
    }
}
