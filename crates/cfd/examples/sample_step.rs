// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2024-2026 西濱大将 (NISHIHAMA Daisuke)
//! Development tool: writes `examples/sample_rocket.step`, the B-rep fixture of `surface = "step"`.
//!
//! `cargo run --release -p cfd --example sample_step -- [out.step] [--no-details]`
//!
//! The geometry is extracted from `examples/sample_rocket.stl` (millimetres, as in `examples/sample_cfd.toml`) and
//! rebuilt like the CAD surface of the CFD mesh (body of revolution fused with lens fins, rolled like the fins of the
//! STL), plus a launch lug and two rail buttons (`--no-details`: without them, a shape symmetric about both planes).
//! The solid is moved back to the coordinates of the STL (the inverse of the CFD-frame transform) and written in
//! millimetres by `sample_step.py` (gmsh python of the CFD tools). The fixture only changes when this tool is run again.

use cfd::step_mesh::cfd_affine;
use geom::stl::Triangle;
use geom::{extract, ExtractOptions};

fn main() -> anyhow::Result<()> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let args: Vec<String> = std::env::args().skip(1).collect();
    let details = !args.iter().any(|a| a == "--no-details");
    let out = args.iter().find(|a| !a.starts_with("--")).map(std::path::PathBuf::from).unwrap_or_else(|| root.join("examples/sample_rocket.step"));
    let raw: Vec<Triangle> = geom::stl::read_stl(&root.join("examples/sample_rocket.stl"))?.into_iter().map(|t| t.map(|v| v * 0.001)).collect();
    let g = extract(&raw, &ExtractOptions::default())?;
    let (profile, fins) = cfd::mesh::cad_description(&g);
    // Roll of the STL fins in the CFD frame: azimuth of the points beyond half the span, folded onto one fin.
    let cfd_tris = cfd::surface::to_cfd_frame(&raw, &g);
    let f = fins.as_ref().expect("the sample has fins");
    let (mut s, mut c) = (0.0, 0.0);
    for v in cfd_tris.iter().flatten() {
        let r = v.y.hypot(v.z);
        if r > f.body_radius + 0.5 * f.span {
            let a = v.z.atan2(v.y) - std::f64::consts::FRAC_PI_2;
            let k = f.count as f64;
            s += (k * a).sin();
            c += (k * a).cos();
        }
    }
    let roll_deg = s.atan2(c).to_degrees() / f.count as f64;
    let params = serde_json::json!({
        "profile": profile,
        "fins": fins,
        "fin_roll_deg": roll_deg,
        "length": g.length,
        "affine": cfd_affine(&g),
        "out": out,
        "details": details,
    });
    let dir = std::env::temp_dir().join(format!("ignisyeet_sample_step_{}", std::process::id()));
    std::fs::create_dir_all(&dir)?;
    let pj = dir.join("params.json");
    std::fs::write(&pj, serde_json::to_string_pretty(&params)?)?;
    let script = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/sample_step.py");
    let tools = cfd::tools::check_tools(&cfd::CfdOptions::default(), &[]);
    let py = tools.gmsh.path.clone().ok_or_else(|| anyhow::anyhow!("no python with gmsh (set IGNISYEET_CFD_PREFIX)"))?;
    let st = cfd::tools::command(&py, tools.prefix.as_deref()).arg(&script).arg(&pj).status()?;
    std::fs::remove_dir_all(&dir).ok();
    anyhow::ensure!(st.success(), "sample_step.py failed");
    println!("fin roll of the STL {roll_deg:.3} deg; wrote {}", out.display());
    Ok(())
}
