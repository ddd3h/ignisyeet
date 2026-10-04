// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025-2026 西濱大将 (NISHIHAMA Daisuke)
//! Development tool for CFD studies on the sample rocket (not part of the user interface).
//!
//! `cargo run --release -p cfd --example study -- options.json out_dir [threads]`
//!
//! `options.json` is a `CfdOptions` object (`prefix`, `machs`, `alphas_deg`, `wall_size`, ...). Runs the
//! normal pipeline and prints the solved cases and the combined table slopes.

use cfd::{CfdOptions, ResourceBudget};
use geom::sample::SampleRocket;
use geom::{extract, ExtractOptions};

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let opt: CfdOptions = serde_json::from_str(&std::fs::read_to_string(&args[1])?)?;
    let out = std::path::PathBuf::from(&args[2]);
    let threads: usize = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(16);
    let g = extract(&SampleRocket::default().mesh(), &ExtractOptions::default())?;
    let aero = aero::AeroOptions { mach_max: 3.0, mach_step: 0.1, alpha_max_deg: 16.0, alpha_step_deg: 1.0, ..Default::default() };
    let t0 = std::time::Instant::now();
    let budget = ResourceBudget { threads, memory_bytes: Some(24_000_000_000) };
    let (tab, rep) = cfd::build_table_with_progress(&g, &opt, &aero, &out, "study".into(), aero::Extrapolation::Linear, &budget, None, &|s| {
        if let cfd::CfdStage::CaseDone { mach, alpha, converged, .. } = s {
            eprintln!("done M={mach} a={alpha} converged={converged} t={:.0}s", t0.elapsed().as_secs_f64());
        }
    })?;
    let mut csv = String::from("mach,alpha_deg,cn,xcp_m,ca_off,cna_per_rad\n");
    for &m in &opt.machs {
        for &a in &opt.alphas_deg {
            let c = tab.lookup(m, a.to_radians());
            csv.push_str(&format!("{m},{a},{:.5},{:.5},{:.5},{:.4}\n", c.cn, c.xcp, c.ca_off, c.cna));
        }
    }
    std::fs::write(out.join("table_lookup.csv"), csv)?;
    std::fs::write(out.join("cfd_report.json"), serde_json::to_string_pretty(&rep)?)?;
    println!("{}", serde_json::to_string_pretty(&rep)?);
    Ok(())
}
