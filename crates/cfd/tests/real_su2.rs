// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025-2026 西濱大将 (NISHIHAMA Daisuke)
//! Integration test with the real SU2 / gmsh / MPI tools. Needs `IGNISYEET_CFD_PREFIX` (or the tools
//! in `PATH`); run with `cargo test --release -p cfd -- --ignored`.

use cfd::{CfdOptions, ResourceBudget};
use geom::stl::Triangle;
use geom::{extract, ExtractOptions, Vec3};

/// Closed 10 degree half-angle cone (tip at the origin, +x aft) + cylinder, R = 0.05 m.
fn cone_cylinder() -> (Vec<Triangle>, f64) {
    let (r, delta, lb, n) = (0.05, 10f64.to_radians(), 0.3, 64);
    let lc = r / delta.tan();
    let ring = |x: f64, rad: f64, k: usize| {
        let a = 2.0 * std::f64::consts::PI * k as f64 / n as f64;
        Vec3::new(x, rad * a.cos(), rad * a.sin())
    };
    let mut t = Vec::new();
    for k in 0..n {
        t.push([Vec3::new(0.0, 0.0, 0.0), ring(lc, r, k), ring(lc, r, k + 1)]);
    }
    let m = 30;
    for i in 0..m {
        let (x0, x1) = (lc + lb * i as f64 / m as f64, lc + lb * (i + 1) as f64 / m as f64);
        for k in 0..n {
            t.push([ring(x0, r, k), ring(x0, r, k + 1), ring(x1, r, k + 1)]);
            t.push([ring(x0, r, k), ring(x1, r, k + 1), ring(x1, r, k)]);
        }
    }
    for k in 0..n {
        t.push([Vec3::new(lc + lb, 0.0, 0.0), ring(lc + lb, r, k + 1), ring(lc + lb, r, k)]);
    }
    // Orient outward.
    for (i, tri) in t.iter_mut().enumerate() {
        let nrm = (tri[1] - tri[0]).cross(tri[2] - tri[0]);
        let c = (tri[0] + tri[1] + tri[2]) / 3.0;
        let out = if i >= t_len(n, m) - n { Vec3::new(1.0, 0.0, 0.0) } else { Vec3::new(0.0, c.y, c.z) };
        if nrm.dot(out) < 0.0 {
            tri.swap(1, 2);
        }
    }
    (t, lc)
}

fn t_len(n: usize, m: usize) -> usize {
    n + 2 * n * m + n
}

#[test]
#[ignore = "needs SU2, gmsh and MPI (IGNISYEET_CFD_PREFIX)"]
fn cone_cp_matches_taylor_maccoll() {
    let (tris, lc) = cone_cylinder();
    let g = extract(&tris, &ExtractOptions::default()).unwrap();
    let opt = CfdOptions {
        machs: vec![2.0],
        alphas_deg: vec![0.0, 2.0],
        wall_size: 0.005,
        ranks_per_case: 4,
        parallel_cases: 1,
        ..Default::default()
    };
    let dir = std::env::temp_dir().join(format!("ignisyeet_cfd_real_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let aero = aero::AeroOptions { mach_max: 3.0, mach_step: 0.5, alpha_max_deg: 4.0, alpha_step_deg: 1.0, ..Default::default() };
    let (table, report) =
        cfd::build_table_with_progress(&g, &opt, &aero, &dir, "h".into(), aero::Extrapolation::Linear, &ResourceBudget::default(), None, &|_| {}).unwrap();
    assert!(report.failed.is_empty(), "{report:?}");
    let c = table.lookup(2.0, 2f64.to_radians());
    assert!(c.cn > 0.05 && c.cn < 0.2, "CN at 2 deg {}", c.cn);

    // Surface Cp of the cone from the conservative variables of surface_flow.csv.
    let atm = aero.atmosphere.at(0.0);
    let q = 0.5 * 1.4 * atm.pressure * 4.0;
    let text = std::fs::read_to_string(dir.join("cfd/m2.000_a00.00/surface_flow.csv")).unwrap();
    let mut lines = text.lines();
    let head: Vec<String> = lines.next().unwrap().split(',').map(|s| s.trim().trim_matches('"').to_string()).collect();
    let col = |n: &str| head.iter().position(|h| h == n).unwrap();
    let (cx, cy, cz, cr, cu, cv, cw, ce) =
        (col("x"), col("y"), col("z"), col("Density"), col("Momentum_x"), col("Momentum_y"), col("Momentum_z"), col("Energy"));
    let delta = 10f64.to_radians();
    let mut cps = Vec::new();
    for l in lines {
        let v: Vec<f64> = l.split(',').map(|s| s.trim().parse().unwrap()).collect();
        let rad = v[cy].hypot(v[cz]);
        if v[cx] > 0.4 * lc && v[cx] < 0.95 * lc && (rad - v[cx] * delta.tan()).abs() < 0.01 * rad + 1e-3 * lc {
            let p = 0.4 * (v[ce] - 0.5 * (v[cu] * v[cu] + v[cv] * v[cv] + v[cw] * v[cw]) / v[cr]);
            cps.push((p - atm.pressure) / q);
        }
    }
    assert!(cps.len() > 100);
    let mean = cps.iter().sum::<f64>() / cps.len() as f64;
    let tm = panel::supersonic::cone_cp(2.0, delta).unwrap();
    assert!((mean - tm).abs() / tm < 0.10, "cone Cp {mean} vs Taylor-Maccoll {tm}");
    let _ = std::fs::remove_dir_all(&dir);
}
