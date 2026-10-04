// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025-2026 西濱大将 (NISHIHAMA Daisuke)
//! User-facing options of the CFD mode (`[aero.cfd]` in the configuration file).

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Governing equations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum FlowModel {
    /// Inviscid compressible Euler equations; viscous drag terms come from the Barrowman build-up.
    #[default]
    Euler,
    /// Reynolds-averaged Navier-Stokes with the Spalart-Allmaras turbulence model.
    Rans,
}

/// Where the wall surface comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum SurfaceKind {
    /// Watertight mesh of the extracted geometry (`panel` crate mesh generator, base closed).
    #[default]
    PanelMesh,
    /// The raw STL, transformed to the CFD frame; it must be watertight.
    Stl,
}

/// Convective flux scheme.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Scheme {
    #[default]
    Roe,
    Jst,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct CfdOptions {
    pub model: FlowModel,
    /// Solved Mach numbers (strictly increasing, positive).
    pub machs: Vec<f64>,
    /// Solved angles of attack [deg] (strictly increasing, >= 0, must contain 0).
    pub alphas_deg: Vec<f64>,
    pub surface: SurfaceKind,
    /// Half model about the pitch plane (y = 0, keeping y >= 0).
    pub symmetry: bool,
    /// Euler only: the flat base is followed by a conical tail fairing of this length in base radii
    /// (0 = flat base). The inviscid flow behind a flat base is ill-posed (asymmetric steady wake);
    /// the fairing, marker `base`, is not monitored, so the base drag comes from the Barrowman term.
    /// RANS always keeps the flat base (the wake is part of the solution).
    pub tail_fairing: f64,
    /// Farfield radius in body lengths.
    pub farfield: f64,
    /// Surface element size [m].
    pub wall_size: f64,
    /// RANS: target y+ of the first layer.
    pub yplus: f64,
    /// Maximum iterations per case.
    pub iterations: usize,
    pub cfl: f64,
    /// Target level of the RMS density residual (e.g. 1e-6 -> `CONV_RESIDUAL_MINVAL = -6`).
    pub convergence: f64,
    /// Convective scheme.
    pub scheme: Scheme,
    /// Installation prefix of the CFD tools (`<prefix>/bin/SU2_CFD`, `mpirun`, `python` with gmsh).
    /// Empty: use the `IGNISYEET_CFD_PREFIX` environment variable, else look in `PATH`.
    /// A leading `~` is expanded.
    pub prefix: String,
    /// SU2 executable name (or path).
    pub su2: String,
    /// MPI launcher (only needed when `ranks_per_case > 1`).
    pub mpi: String,
    /// MPI ranks per case; 0 = automatic (4, or fewer if the thread budget is smaller).
    pub ranks_per_case: usize,
    /// Concurrent cases; 0 = automatic (as many as fit the thread and memory budget).
    pub parallel_cases: usize,
    /// Wall-clock limit per case [minutes]; 0 disables it. A case hitting the limit is stopped and
    /// accepted only if it is practically converged (see `runner`).
    pub timeout_minutes: f64,
    /// Working directory below `output.dir`; one sub-directory per case.
    pub workdir: PathBuf,
}

impl Default for CfdOptions {
    fn default() -> Self {
        Self {
            model: FlowModel::Euler,
            machs: vec![0.3, 0.6, 0.8, 0.95, 1.1, 1.3, 1.6, 2.0, 2.5, 3.0],
            alphas_deg: vec![0.0, 2.0, 4.0, 8.0, 12.0, 16.0],
            surface: SurfaceKind::PanelMesh,
            symmetry: true,
            tail_fairing: 6.0,
            farfield: 20.0,
            wall_size: 0.004,
            yplus: 1.0,
            iterations: 3000,
            cfl: 5.0,
            convergence: 1e-6,
            scheme: Scheme::Roe,
            prefix: String::new(),
            su2: "SU2_CFD".into(),
            mpi: "mpirun".into(),
            ranks_per_case: 4,
            parallel_cases: 4,
            timeout_minutes: 0.0,
            workdir: "cfd".into(),
        }
    }
}

fn strictly_increasing(v: &[f64]) -> bool {
    v.windows(2).all(|w| w[0] < w[1])
}

impl CfdOptions {
    pub fn validate(&self) -> Result<()> {
        if self.machs.is_empty() || self.machs.iter().any(|&m| !(m.is_finite() && m > 0.0)) {
            bail!("aero.cfd.machs must be a non-empty list of positive Mach numbers");
        }
        if !strictly_increasing(&self.machs) {
            bail!("aero.cfd.machs must be sorted in increasing order without duplicates");
        }
        if self.alphas_deg.is_empty() || self.alphas_deg.iter().any(|&a| !(a.is_finite() && a >= 0.0)) {
            bail!("aero.cfd.alphas_deg must be a non-empty list of angles >= 0");
        }
        if !strictly_increasing(&self.alphas_deg) {
            bail!("aero.cfd.alphas_deg must be sorted in increasing order without duplicates");
        }
        if self.alphas_deg[0] != 0.0 {
            bail!("aero.cfd.alphas_deg must include 0 (the zero-lift axial force and the small-angle slopes)");
        }
        if self.alphas_deg.len() < 2 {
            bail!("aero.cfd.alphas_deg needs at least one angle above 0");
        }
        for (name, v) in [("farfield", self.farfield), ("wall_size", self.wall_size), ("yplus", self.yplus), ("cfl", self.cfl)] {
            if !(v.is_finite() && v > 0.0) {
                bail!("aero.cfd.{name} must be positive");
            }
        }
        if !(self.tail_fairing.is_finite() && self.tail_fairing >= 0.0) {
            bail!("aero.cfd.tail_fairing must be >= 0");
        }
        if self.farfield < 5.0 {
            bail!("aero.cfd.farfield must be at least 5 body lengths");
        }
        if !(self.convergence > 0.0 && self.convergence < 1.0) {
            bail!("aero.cfd.convergence must be in (0, 1), e.g. 1e-6");
        }
        if self.iterations == 0 {
            bail!("aero.cfd.iterations must be at least 1");
        }
        if !(self.timeout_minutes.is_finite() && self.timeout_minutes >= 0.0) {
            bail!("aero.cfd.timeout_minutes must be >= 0");
        }
        if self.su2.trim().is_empty() {
            bail!("aero.cfd.su2 must name the SU2_CFD executable");
        }
        Ok(())
    }

    /// Non-fatal remarks, e.g. oversubscribed cores.
    pub fn warnings(&self) -> Vec<String> {
        self.warnings_for(std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1))
    }

    pub fn warnings_for(&self, available: usize) -> Vec<String> {
        let mut w = Vec::new();
        let need = self.ranks_per_case * self.parallel_cases;
        if self.ranks_per_case > 0 && self.parallel_cases > 0 && need > available {
            w.push(format!(
                "aero.cfd: ranks_per_case x parallel_cases = {need} exceeds the {available} available cores; cases will be slowed down"
            ));
        }
        w
    }

    /// `CONV_RESIDUAL_MINVAL` (log10 of the RMS density residual target).
    pub fn residual_minval(&self) -> f64 {
        self.convergence.log10()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_valid_and_match_plan() {
        let o = CfdOptions::default();
        o.validate().unwrap();
        assert_eq!(o.machs.len(), 10);
        assert_eq!(o.alphas_deg, vec![0.0, 2.0, 4.0, 8.0, 12.0, 16.0]);
        assert_eq!((o.ranks_per_case, o.parallel_cases, o.iterations), (4, 4, 3000));
        assert!((o.residual_minval() + 6.0).abs() < 1e-12);
    }

    #[test]
    fn rejects_bad_grids() {
        let bad = |f: fn(&mut CfdOptions)| {
            let mut o = CfdOptions::default();
            f(&mut o);
            o.validate().unwrap_err().to_string()
        };
        assert!(bad(|o| o.machs = vec![0.0, 1.0]).contains("positive"));
        assert!(bad(|o| o.machs = vec![0.5, 0.5]).contains("sorted"));
        assert!(bad(|o| o.machs = vec![2.0, 1.0]).contains("sorted"));
        assert!(bad(|o| o.alphas_deg = vec![2.0, 4.0]).contains("include 0"));
        assert!(bad(|o| o.alphas_deg = vec![0.0, -1.0]).contains(">= 0"));
        assert!(bad(|o| o.alphas_deg = vec![0.0]).contains("at least one"));
        assert!(bad(|o| o.wall_size = 0.0).contains("wall_size"));
        assert!(bad(|o| o.convergence = 2.0).contains("convergence"));
    }

    #[test]
    fn oversubscription_is_a_warning() {
        let o = CfdOptions::default();
        assert!(o.warnings_for(20).is_empty());
        assert_eq!(o.warnings_for(8).len(), 1);
        o.validate().unwrap();
    }

    #[test]
    fn parses_and_rejects_unknown_fields() {
        let o: CfdOptions = serde_json::from_str(r#"{"model":"rans","surface":"stl","machs":[0.5,2.0]}"#).unwrap();
        assert_eq!(o.model, FlowModel::Rans);
        assert_eq!(o.surface, SurfaceKind::Stl);
        assert!(serde_json::from_str::<CfdOptions>(r#"{"bogus":1}"#).is_err());
    }
}
