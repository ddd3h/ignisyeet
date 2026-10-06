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
    let opt = CfdOptions { machs: vec![2.0], alphas_deg: vec![0.0, 2.0], wall_size: 0.005, ranks_per_case: 4, parallel_cases: 1, ..Default::default() };
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

fn sample_stl() -> (Vec<Triangle>, geom::Geometry) {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/sample_rocket.stl");
    let raw: Vec<Triangle> = geom::stl::read_stl(&path).unwrap().into_iter().map(|t| t.map(|v| v * 0.001)).collect();
    let g = extract(&raw, &ExtractOptions::default()).unwrap();
    (raw, g)
}

fn stl_options() -> CfdOptions {
    CfdOptions {
        surface: cfd::SurfaceKind::Stl,
        symmetry: false,
        base_treatment: Some(cfd::BaseTreatment::Fairing),
        machs: vec![0.8],
        alphas_deg: vec![0.0, 4.0],
        wall_size: 0.005,
        ranks_per_case: 4,
        parallel_cases: 1,
        roll_damping: true,
        ..Default::default()
    }
}

#[test]
#[ignore = "needs gmsh and manifold3d (IGNISYEET_CFD_PREFIX)"]
fn stl_union_is_genus_zero_and_meshes() {
    let (raw, g) = sample_stl();
    let dir = std::env::temp_dir().join(format!("ignisyeet_cfd_stl_mesh_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let aero = aero::AeroOptions::default();
    let m = cfd::mesh_only(&g, &stl_options(), &aero, &dir, &ResourceBudget::default(), Some(&raw)).unwrap();
    let info = m.stl.as_ref().unwrap();
    // Five input shells (body + 4 fin boxes) and the fairing cone, united into one solid.
    assert_eq!(info.union_report["components"].as_array().unwrap().len(), 6, "{}", info.union_report);
    assert_eq!(info.union_report["genus"].as_i64(), Some(0));
    assert!(info.repair["components"].as_array().unwrap().iter().all(|c| c["flipped"] == true));
    assert!(m.stats.cells > 10_000 && m.stats.wall_triangles > 1000);
    // The remeshed wall stays within the wall size of the union and encloses its volume.
    assert!(info.fidelity.max_error() < 0.01, "{:?}", info.fidelity);
    assert!((info.fidelity.cfd.volume.unwrap() / info.fidelity.union.volume.unwrap() - 1.0).abs() < 5e-3);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
#[ignore = "needs SU2, gmsh, manifold3d and MPI (IGNISYEET_CFD_PREFIX)"]
fn stl_euler_case_and_roll_pair() {
    let (raw, g) = sample_stl();
    let dir = std::env::temp_dir().join(format!("ignisyeet_cfd_stl_run_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let aero = aero::AeroOptions { mach_max: 1.0, mach_step: 0.5, ..Default::default() };
    let opt = stl_options();
    let (table, report) =
        cfd::build_table_with_progress(&g, &opt, &aero, &dir, "h".into(), aero::Extrapolation::Linear, &ResourceBudget::default(), Some(&raw), &|_| {})
            .unwrap();
    assert!(report.failed.is_empty(), "{report:?}");
    assert_eq!(report.table_kind, "general");
    assert_eq!(report.clp_source, "cfd", "{:?}", report.clp);
    let clp = report.clp[0].cfd.unwrap();
    assert!(clp < -5.0 && clp > -60.0, "Clp {clp}");
    // Positive alpha: negative CZ (the body-axis normal force is -CN along z) of a plausible size.
    let c = table.lookup6(0.8, 4f64.to_radians(), 0.0);
    assert!(c.cz < -0.3 && c.cz > -3.0, "CZ {}", c.cz);
    assert!((table.lookup6(0.8, -4f64.to_radians(), 0.0).cz + c.cz).abs() < 0.2 * c.cz.abs());
    assert!(report.fidelity.is_some() && report.warnings.iter().any(|w| w.contains("MODIFIED")));
    let _ = std::fs::remove_dir_all(&dir);
}

/// Cells of the volume mesh written by the STL mesher (`meshed domain (quarter|half|full): cells N` in `mesh.log`).
fn meshed_cells(dir: &std::path::Path) -> (String, usize) {
    let log = std::fs::read_to_string(dir.join("cfd/mesh/mesh.log")).unwrap();
    let line = log.lines().find(|l| l.starts_with("meshed domain")).expect("meshed domain line");
    let kind = line.split(['(', ')']).nth(1).unwrap().to_string();
    let cells = line.split("cells ").nth(1).unwrap().split(',').next().unwrap().trim().parse().unwrap();
    (kind, cells)
}

#[test]
#[ignore = "needs gmsh and manifold3d (IGNISYEET_CFD_PREFIX)"]
fn stl_symmetry_is_verified_and_the_domain_follows() {
    let (raw, g) = sample_stl();
    let aero = aero::AeroOptions::default();
    let base = CfdOptions { roll_damping: false, symmetry: true, ..stl_options() };
    let run = |name: &str, o: &CfdOptions| {
        let dir = std::env::temp_dir().join(format!("ignisyeet_cfd_stl_sym_{name}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let m = cfd::mesh_only(&g, o, &aero, &dir, &ResourceBudget::default(), Some(&raw)).unwrap();
        (m, dir)
    };
    // auto: the sample STL is mirror symmetric about both planes (the vertex sets exactly; the triangulation of the
    // facets differs, so the check falls back to the surface itself).
    let (half, d1) = run("half", &base);
    let sym = &half.stl.as_ref().unwrap().symmetry;
    assert_eq!((sym.chosen.as_str(), sym.meshed.as_str()), ("axisymmetric", "quarter"), "{sym:?}");
    assert!(sym.y.symmetric && sym.z.symmetric && sym.y.max_mismatch < 1e-6 && sym.z.max_mismatch < 1e-6, "{sym:?}");
    let (kind, quarter) = meshed_cells(&d1);
    assert_eq!(kind, "quarter");
    // Half kept (SU2 symmetry marker), the same quarter mirrored to the full domain, and the full domain meshed directly.
    let (full_mirror, d2) = run("fullmirror", &CfdOptions { symmetry: false, ..base.clone() });
    let (full, d3) = run("full", &CfdOptions { shape_symmetry: Some(cfd::ShapeSymmetry::None), ..base.clone() });
    assert_eq!(meshed_cells(&d3).0, "full");
    let (c_half, c_mirror, c_full) = (half.stats.cells as f64, full_mirror.stats.cells as f64, full.stats.cells as f64);
    assert!((quarter as f64 / c_full - 0.25).abs() < 0.04, "quarter {quarter} vs full {c_full}");
    assert!((c_half / c_full - 0.5).abs() < 0.06, "half {c_half} vs full {c_full}");
    assert!((c_mirror / c_full - 1.0).abs() < 0.1, "mirrored full {c_mirror} vs full {c_full}");
    assert!((c_mirror - 2.0 * c_half).abs() < 0.02 * c_mirror, "{c_mirror} {c_half}");
    // The mirrored wall reproduces the union as well as the full-domain wall does.
    let f = &full_mirror.stl.as_ref().unwrap().fidelity;
    assert!((f.cfd.volume.unwrap() / f.union.volume.unwrap() - 1.0).abs() < 5e-3, "{f:?}");
    for d in [d1, d2, d3] {
        let _ = std::fs::remove_dir_all(&d);
    }
}

#[test]
#[ignore = "needs gmsh and manifold3d (IGNISYEET_CFD_PREFIX)"]
fn explicit_symmetry_on_an_asymmetric_stl_is_an_error() {
    let (raw, g) = sample_stl();
    // One fin is skewed (its outer vertices move 2 mm along the axis): the shell stays closed, the union is one solid,
    // and the surface is no longer mirror symmetric about either plane.
    let skewed: Vec<Triangle> = raw.iter().map(|t| t.map(|v| if v.x > 0.055 && v.y > 0.055 { Vec3::new(v.x, v.y, v.z + 0.002) } else { v })).collect();
    let dir = std::env::temp_dir().join(format!("ignisyeet_cfd_stl_asym_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let auto = cfd::stl_mesh::check_symmetry(&CfdOptions { roll_damping: false, ..stl_options() }, &g, &skewed, &dir).unwrap();
    assert!(!auto.y.symmetric && !auto.z.symmetric && auto.y.max_mismatch > 1e-4, "{auto:?}");
    assert_eq!((auto.chosen.as_str(), auto.meshed.as_str()), ("none", "full"));
    let e = cfd::stl_mesh::check_symmetry(&CfdOptions { shape_symmetry: Some(cfd::ShapeSymmetry::Axisymmetric), ..stl_options() }, &g, &skewed, &dir)
        .unwrap_err()
        .to_string();
    assert!(e.contains("axisymmetric") && e.contains("max mismatch") && e.contains("y = 0"), "{e}");
    let e = cfd::stl_mesh::check_symmetry(&CfdOptions { shape_symmetry: Some(cfd::ShapeSymmetry::PitchPlane), ..stl_options() }, &g, &skewed, &dir)
        .unwrap_err()
        .to_string();
    assert!(e.contains("pitch_plane"), "{e}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
#[ignore = "needs SU2, gmsh, manifold3d and MPI (IGNISYEET_CFD_PREFIX)"]
fn stl_half_domain_matches_the_full_domain_in_cn() {
    let (raw, g) = sample_stl();
    let aero = aero::AeroOptions { mach_max: 1.0, mach_step: 0.5, ..Default::default() };
    let base = CfdOptions { roll_damping: false, symmetry: true, ..stl_options() };
    let solve = |dir: &std::path::Path, o: &CfdOptions| {
        let (table, report) =
            cfd::build_table_with_progress(&g, o, &aero, dir, "h".into(), aero::Extrapolation::Linear, &ResourceBudget::default(), Some(&raw), &|_| {})
                .unwrap();
        assert!(report.failed.is_empty(), "{report:?}");
        (table, report)
    };
    let a = 4f64.to_radians();
    // Half domain (SU2 symmetry marker) and the same quarter mesh mirrored to the full domain, in one work directory
    // so that both use the same union (manifold3d does not order its triangles reproducibly) and so the same mesh.
    let dir = std::env::temp_dir().join(format!("ignisyeet_cfd_stl_cn_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let (half, rh) = solve(&dir, &base);
    assert_eq!((rh.shape_symmetry.as_str(), rh.domain.as_str()), ("axisymmetric", "half"));
    assert_eq!(rh.symmetry_check.as_ref().unwrap()["chosen"], "axisymmetric");
    let (mirrored, rm) = solve(&dir, &CfdOptions { symmetry: false, ..base.clone() });
    assert_eq!((rm.shape_symmetry.as_str(), rm.domain.as_str()), ("axisymmetric", "full"));
    let (ch, cm) = (half.lookup6(0.8, a, 0.0), mirrored.lookup6(0.8, a, 0.0));
    assert!((ch.cz / cm.cz - 1.0).abs() < 0.01, "CN half {} vs mirrored full {}", -ch.cz, -cm.cz);
    // -alpha is the mirror image of +alpha in the exactly z-symmetric mesh.
    assert!((half.lookup6(0.8, -a, 0.0).cz + ch.cz).abs() < 1e-3 * ch.cz.abs());
    let _ = std::fs::remove_dir_all(&dir);
    // The directly meshed full domain is a different mesh (and a different union); at 5 mm the normal force of two such
    // meshes scatters by a few per cent, so this is a plausibility bound only.
    let dir = std::env::temp_dir().join(format!("ignisyeet_cfd_stl_cn_full_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let (full, rf) = solve(&dir, &CfdOptions { shape_symmetry: Some(cfd::ShapeSymmetry::None), ..base });
    assert_eq!((rf.shape_symmetry.as_str(), rf.domain.as_str()), ("none", "full"));
    let cf = full.lookup6(0.8, a, 0.0);
    assert!((ch.cz / cf.cz - 1.0).abs() < 0.06, "CN half {} vs full {}", -ch.cz, -cf.cz);
    let _ = std::fs::remove_dir_all(&dir);
}
