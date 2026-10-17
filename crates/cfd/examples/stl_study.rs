// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2024-2026 西濱大将 (NISHIHAMA Daisuke)
//! Development tool for the STL surface on `examples/sample_rocket.stl` (not part of the user interface).
//!
//! `cargo run --release -p cfd --example stl_study -- options.json out_dir [threads] [--mesh-only]`
//!
//! `options.json` is a `CfdOptions` object (`surface` is forced to `stl`). Meshes the sample STL, prints the
//! symmetry decision, the mesh statistics and the fidelity summary, and (without `--mesh-only`) runs the
//! case matrix and prints the solved cases.

use cfd::{CfdOptions, ResourceBudget};
use geom::stl::Triangle;
use geom::{extract, ExtractOptions};

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let mut opt: CfdOptions = serde_json::from_str(&std::fs::read_to_string(&args[1])?)?;
    opt.surface = cfd::SurfaceKind::Stl;
    let out = std::path::PathBuf::from(&args[2]);
    let threads: usize = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(4);
    let mesh_only = args.iter().any(|a| a == "--mesh-only");
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/sample_rocket.stl");
    let raw: Vec<Triangle> = geom::stl::read_stl(&path)?.into_iter().map(|t| t.map(|v| v * 0.001)).collect();
    let g = extract(&raw, &ExtractOptions::default())?;
    let aero = aero::AeroOptions { mach_max: 3.0, mach_step: 0.1, alpha_max_deg: 16.0, alpha_step_deg: 1.0, ..Default::default() };
    let budget = ResourceBudget { threads, memory_bytes: Some(24_000_000_000) };
    let t0 = std::time::Instant::now();
    if mesh_only {
        let m = cfd::mesh_only(&g, &opt, &aero, &out, &budget, Some(&raw))?;
        println!("mesh: {} cells, {} nodes, {} wall triangles ({:.0} s)", m.stats.cells, m.stats.nodes, m.stats.wall_triangles, t0.elapsed().as_secs_f64());
        if let Some(s) = &m.stl {
            println!("{}", s.fidelity.summary());
        }
        return Ok(());
    }
    let (_tab, rep) = cfd::build_table_with_progress(&g, &opt, &aero, &out, "study".into(), aero::Extrapolation::Linear, &budget, Some(&raw), &|s| {
        if let cfd::CfdStage::CaseDone { mach, alpha, converged, .. } = s {
            eprintln!("done M={mach} a={alpha} converged={converged} t={:.0}s", t0.elapsed().as_secs_f64());
        }
    })?;
    std::fs::write(out.join("cfd_report.json"), serde_json::to_string_pretty(&rep)?)?;
    println!("{}", serde_json::to_string_pretty(&rep)?);
    Ok(())
}
