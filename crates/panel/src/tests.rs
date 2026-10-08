// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025-2026 西濱大将 (NISHIHAMA Daisuke)
use super::*;
use crate::mesh::Mesh;
use crate::solver::{coefficients, solve_potential};
use crate::test_tmp::TempDir;
use aero::AeroModel;
use geom::sample::SampleRocket;
use geom::{extract, ExtractOptions, FinSet};
use std::f64::consts::PI;

fn geometry(rk: &SampleRocket) -> Geometry {
    extract(&rk.mesh(), &ExtractOptions::default()).unwrap()
}

fn coarse() -> PanelOptions {
    PanelOptions { body_axial: 60, body_circ: 32, fin_chord: 16, fin_span: 10, subsonic_machs: vec![0.0, 0.3], ..Default::default() }
}

fn small_aero() -> AeroOptions {
    AeroOptions { mach_min: 0.0, mach_max: 0.6, mach_step: 0.3, alpha_max_deg: 4.0, alpha_step_deg: 2.0, ..Default::default() }
}

#[test]
fn sphere_cp() {
    let (na, nc) = (20, 40);
    let st: Vec<f64> = (0..=na).map(|i| 1.0 - (PI * i as f64 / na as f64).cos()).collect();
    let r: Vec<f64> = (0..=na).map(|i| (PI * i as f64 / na as f64).sin()).collect();
    let mesh = Mesh::body_of_revolution(&st, &r, nc, 0.0).unwrap();
    let (open, over) = mesh.edge_census();
    assert!(open.is_empty() && over == 0, "sphere not watertight");
    let pot = solve_potential(&mesh).unwrap();
    let mut worst: f64 = 0.0;
    for (p, v) in mesh.panels.iter().zip(&pot.vel[0]) {
        let cp = 1.0 - v.dot(*v);
        let s2 = 1.0 - p.normal.x * p.normal.x;
        worst = worst.max((cp - (1.0 - 2.25 * s2)).abs());
    }
    println!("sphere max Cp error {worst:.4}");
    assert!(worst < 0.05, "sphere Cp error {worst}");
}

#[test]
fn prolate_spheroid_axial_flow() {
    let (a, b) = (3.0f64, 0.5f64);
    let na = 48;
    let st: Vec<f64> = (0..=na).map(|i| a * (1.0 - (PI * i as f64 / na as f64).cos())).collect();
    let r: Vec<f64> = st.iter().map(|&x| b * (1.0 - (x / a - 1.0).powi(2)).max(0.0).sqrt()).collect();
    let mesh = Mesh::body_of_revolution(&st, &r, 24, 0.0).unwrap();
    let pot = solve_potential(&mesh).unwrap();
    let e = (1.0 - b * b / (a * a)).sqrt();
    let a0 = 2.0 * (1.0 - e * e) / e.powi(3) * (0.5 * ((1.0 + e) / (1.0 - e)).ln() - e);
    let k = a0 / (2.0 - a0);
    let mut worst: f64 = 0.0;
    for (p, v) in mesh.panels.iter().zip(&pot.vel[0]) {
        let xr = p.centroid.x / (2.0 * a);
        if !(0.1..0.9).contains(&xr) {
            continue;
        }
        let s2 = 1.0 - p.normal.x * p.normal.x;
        let cp = 1.0 - v.dot(*v);
        worst = worst.max((cp - (1.0 - (1.0 + k).powi(2) * s2)).abs());
    }
    println!("spheroid max Cp error {worst:.4} (k = {k:.4})");
    assert!(worst < 0.05, "spheroid Cp error {worst}");
}

fn alpha_coeffs(mesh: &Mesh, s_ref: f64) -> solver::PotentialCoeffs {
    let pot = solve_potential(mesh).unwrap();
    coefficients(mesh, &pot, 1.0, s_ref)
}

#[test]
fn slender_body_cna_is_two() {
    let rk = SampleRocket { fin_count: 0, ..Default::default() };
    let g = geometry(&rk);
    assert!(g.fins.is_none());
    let mesh = Mesh::from_geometry(&g, &coarse()).unwrap();
    let c = alpha_coeffs(&mesh, g.ref_area);
    println!("ogive-cylinder CNa = {:.4}, xcp = {:.4} m", c.cna, c.m_alpha / c.cna);
    assert!((c.cna - 2.0).abs() < 0.2, "CNa {}", c.cna);
}

#[test]
fn mesh_is_watertight() {
    let g = geometry(&SampleRocket::default());
    let mesh = Mesh::from_geometry(&g, &coarse()).unwrap();
    let (open, over) = mesh.edge_census();
    assert_eq!(over, 0, "edges shared by more than two panels");
    assert!(open.is_empty(), "{} open edges", open.len());
    println!("panels {}, wake panels {}", mesh.panels.len(), mesh.wake_panels.len());
}

#[test]
fn thin_wing_matches_helmbold() {
    let rk = SampleRocket { radius: 0.01, nose_length: 0.2, ..Default::default() };
    let mut g = geometry(&SampleRocket { fin_count: 0, ..rk.clone() });
    let (c, s, r) = (0.25, 0.5, 0.01);
    g.fins = Some(FinSet { count: 2, root_chord: c, tip_chord: c, span: s, sweep: 0.0, thickness: 0.01, x_le_root: 0.6, body_radius: r, cant_deg: 0.0 });
    let opt = PanelOptions { body_axial: 50, body_circ: 24, fin_chord: 14, fin_span: 8, ..coarse() };
    // Fins sit at +-z; turn them by 90 degrees so the flow in the x-z plane is normal to them.
    let mesh = Mesh::from_geometry(&g, &opt).unwrap().rotated_about_x(0.5 * PI);
    let cna = alpha_coeffs(&mesh, g.ref_area).cna;
    let sw = 2.0 * (s + r) * c;
    let ar = (2.0 * (s + r)).powi(2) / sw;
    let clal = cna * g.ref_area / sw;
    let helmbold = 2.0 * PI * ar / (2.0 + (ar * ar + 4.0).sqrt());
    println!("wing AR {ar:.2}: CLa = {clal:.4}, Helmbold {helmbold:.4}");
    assert!((clal - helmbold).abs() < 0.1 * helmbold, "{clal} vs {helmbold}");
}

#[test]
fn sample_rocket_vs_barrowman() {
    let g = geometry(&SampleRocket::default());
    let (table, rep) = build_table(&g, &coarse(), &small_aero(), "t".into(), Extrapolation::Linear).unwrap();
    let model = AeroModel::new(g.clone(), small_aero());
    let (bcna, bxcp) = (model.cna(0.3), model.normal(0.3, 1e-4).1);
    let s = rep.subsonic.iter().find(|s| s.mach == 0.3).unwrap();
    println!("M=0.3: panel CNa {:.3} xcp {:.4} | Barrowman CNa {bcna:.3} xcp {bxcp:.4}", s.cna, s.xcp);
    println!(
        "M=0: panel CNa {:.3} xcp {:.4}; panels {} wake {} solve {:.1}s",
        rep.subsonic[0].cna, rep.subsonic[0].xcp, rep.panels, rep.wake_panels, rep.solve_seconds
    );
    // Barrowman's fin-body interference factor (1 + R/(S+R) = 1.33) is lower than the panel
    // result (about 1.6, between Barrowman and the slender-body limit 1.78), so the panel CNa
    // is higher by roughly a quarter and the centre of pressure about 0.03 m further aft; the
    // isolated-wing test pins the fin lift itself.
    assert!((s.cna / bcna - 1.0).abs() < 0.30, "CNa {} vs {bcna}", s.cna);
    assert!((s.xcp - bxcp).abs() < 0.04, "xcp {} vs {bxcp}", s.xcp);
    let c = table.lookup(0.3, 2f64.to_radians());
    assert!(c.cn > 0.0 && c.xcp > 0.5 && c.xcp < 1.5);
}

#[test]
fn mesh_convergence() {
    let g = geometry(&SampleRocket::default());
    let run = |o: PanelOptions| {
        let mesh = Mesh::from_geometry(&g, &o).unwrap();
        alpha_coeffs(&mesh, g.ref_area).cna
    };
    let a = run(coarse());
    let b = run(PanelOptions { body_axial: 120, body_circ: 64, ..coarse() });
    println!("convergence: CNa {a:.4} -> {b:.4} ({:.2}%)", 100.0 * (b / a - 1.0));
    assert!((b / a - 1.0).abs() < 0.05);
}

#[test]
fn transonic_continuity() {
    let g = geometry(&SampleRocket::default());
    let aero = AeroOptions { mach_min: 0.6, mach_max: 1.6, mach_step: 0.02, alpha_max_deg: 4.0, alpha_step_deg: 2.0, ..Default::default() };
    let opt = PanelOptions { subsonic_machs: vec![0.0, 0.3, 0.6, 0.8], ..coarse() };
    let (t, _) = build_table(&g, &opt, &aero, "t".into(), Extrapolation::Linear).unwrap();
    let a = 2f64.to_radians();
    let mut prev = t.lookup(0.6, a);
    let mut m = 0.6;
    let (mut max_cn, mut max_ca) = (0.0f64, 0.0f64);
    while m < 1.59 {
        m += 0.02;
        let c = t.lookup(m, a);
        let dcn = (c.cn - prev.cn).abs() / prev.cn.abs();
        let dca = (c.ca_off - prev.ca_off).abs() / prev.ca_off;
        max_cn = max_cn.max(dcn);
        max_ca = max_ca.max(dca);
        // CN may change by up to 8 % per 0.02 Mach just above the band (Mach 1.2-1.3): the fin
        // leading-edge panels (biconvex, about 4.3 deg) pass the wedge-detachment angle there
        // (3.9 deg at Mach 1.2) and the attached weak-shock pressure rises steeply towards it.
        assert!(c.cn.is_finite() && dcn < 0.08 && dca < 0.05, "jump at M={m:.2}: cn {} -> {}, ca {} -> {}", prev.cn, c.cn, prev.ca_off, c.ca_off);
        prev = c;
    }
    println!("max step change over 0.02 Mach: CN {:.3}, CA {:.3}", max_cn, max_ca);
}

/// The transonic band blends the normal force and its nose moment and keeps them, and the
/// zero-angle slope with its moment, between the values at the band edges; the centre of pressure
/// is their ratio.
#[test]
fn transonic_band_is_consistent_and_does_not_overshoot() {
    let g = geometry(&SampleRocket::default());
    let aero = AeroOptions { mach_min: 0.8, mach_max: 1.2, mach_step: 0.02, alpha_max_deg: 6.0, alpha_step_deg: 1.0, ..Default::default() };
    let opt = PanelOptions { subsonic_machs: vec![0.0, 0.3, 0.6, 0.8], ..coarse() };
    let (t, _) = build_table(&g, &opt, &aero, "t".into(), Extrapolation::Linear).unwrap();
    let between = |v: f64, a: f64, b: f64| {
        let tol = 1e-9 * (1.0 + a.abs() + b.abs());
        v >= a.min(b) - tol && v <= a.max(b) + tol
    };
    for &m in t.meta.machs.iter().filter(|&&m| m > 0.8 + 1e-9 && m < 1.2 - 1e-9) {
        for &deg in &t.meta.alphas_deg {
            let a = deg.to_radians();
            let (c, lo, hi) = (t.lookup(m, a), t.lookup(0.8, a), t.lookup(1.2, a));
            assert!(between(c.cna, lo.cna, hi.cna), "cna at M={m}: {} not in [{}, {}]", c.cna, lo.cna, hi.cna);
            if deg == 0.0 {
                // zero-angle centre of pressure: the blended slope moment over the blended slope
                assert!(between(c.cna * c.xcp, lo.cna * lo.xcp, hi.cna * hi.xcp), "slope moment at M={m}");
                continue;
            }
            assert!(between(c.cn, lo.cn, hi.cn), "CN at M={m}, alpha {deg}: {} not in [{}, {}]", c.cn, lo.cn, hi.cn);
            assert!(between(c.cn * c.xcp, lo.cn * lo.xcp, hi.cn * hi.xcp), "moment at M={m}, alpha {deg}");
        }
    }
}

#[test]
fn table_is_finite() {
    let g = geometry(&SampleRocket::default());
    let aero = AeroOptions { mach_max: 3.0, mach_step: 0.25, alpha_max_deg: 30.0, alpha_step_deg: 6.0, ..Default::default() };
    let (t, rep) = build_table(&g, &coarse(), &aero, "t".into(), Extrapolation::Linear).unwrap();
    for &m in &t.meta.machs {
        for &a in &t.meta.alphas_deg {
            let c = t.lookup(m, a.to_radians());
            let v = [c.cn, c.ca_on, c.ca_off, c.xcp, c.cna, c.damp[0], c.damp[1], c.damp[2]];
            assert!(v.iter().all(|x| x.is_finite()), "non-finite at M={m} a={a}: {v:?}");
            assert!(c.ca_off >= c.ca_on - 1e-12);
        }
    }
    let model = AeroModel::new(g.clone(), aero.clone());
    for m in [0.3, 2.0] {
        let c = t.lookup(m, 5f64.to_radians());
        let (bcn, bx) = model.normal(m, 5f64.to_radians());
        println!(
            "M={m}: panel CN(5deg) {:.3} xcp {:.3} CNa {:.3} CA {:.3} S=({:.2},{:.2},{:.2}) | Barrowman CN {bcn:.3} xcp {bx:.3} CNa {:.3} CA {:.3} S={:?}",
            c.cn,
            c.xcp,
            c.cna,
            c.ca_off,
            c.damp[0],
            c.damp[1],
            c.damp[2],
            model.cna(m),
            model.cd0(m, false).total(),
            model.damping_sums(m)
        );
    }
    let td = TempDir::new("panel_cp");
    let path = td.join("cp_test.csv");
    write_surface_cp(&path, &rep).unwrap();
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(text.starts_with("x,y,z,nx,ny,nz,area,cp,part") && text.contains(",fin") && text.contains(",body"));
    let vtk = td.join("mesh_test.vtk");
    Mesh::from_geometry(&g, &coarse()).unwrap().write_vtk(&vtk).unwrap();
}

#[test]
#[ignore = "timing run of the default mesh and full default table grid"]
fn default_table_timing() {
    let g = geometry(&SampleRocket::default());
    let t0 = std::time::Instant::now();
    let (t, rep) = build_table(&g, &PanelOptions::default(), &AeroOptions::default(), "t".into(), Extrapolation::Linear).unwrap();
    println!(
        "default: {} panels, {} wake, solves {:.1}s, total {:.1}s, {} rows",
        rep.panels,
        rep.wake_panels,
        rep.solve_seconds,
        t0.elapsed().as_secs_f64(),
        t.rows()
    );
    for s in &rep.subsonic {
        println!("M={:.1}: CNa {:.3} xcp {:.4}", s.mach, s.cna, s.xcp);
    }
}

#[test]
fn cone_cylinder_cna_at_mach_2() {
    use geom::sample::NoseShape;
    let rk = SampleRocket { nose_shape: NoseShape::Cone, nose_length: 0.05 / 10f64.to_radians().tan(), fin_count: 0, ..Default::default() };
    let g = geometry(&rk);
    let mesh = Mesh::from_geometry(&g, &coarse()).unwrap();
    let sp = supersonic::SupersonicPanels::new(&mesh, g.ref_area, None, None);
    let d = sp.derivatives(2.0, g.length);
    println!("10 deg cone-cylinder M=2: CNa {:.3}", d[0]);
    assert!((d[0] - 2.0).abs() < 0.3, "CNa {}", d[0]);
}

#[test]
fn supersonic_comparison_with_barrowman() {
    let g = geometry(&SampleRocket::default());
    let aero = AeroOptions { mach_min: 0.8, mach_max: 3.0, mach_step: 0.1, alpha_max_deg: 4.0, alpha_step_deg: 2.0, ..Default::default() };
    let opt = PanelOptions { subsonic_machs: vec![0.0, 0.3, 0.5, 0.6, 0.7, 0.8], ..coarse() };
    let (t, _) = build_table(&g, &opt, &aero, "t".into(), Extrapolation::Linear).unwrap();
    let model = AeroModel::new(g.clone(), aero.clone());
    let a = 1e-4;
    let c08 = t.lookup(0.8, a).cna;
    let mut peak = 0.0f64;
    println!("M    CNa_b  CNa_p  xcp_b  xcp_p  CA_b  CA_p");
    for k in 0..=22 {
        let m = 0.8 + 0.1 * k as f64;
        let c = t.lookup(m, a);
        let (_, xb) = model.normal(m, a);
        if m <= 1.6 {
            peak = peak.max(c.cna);
        }
        if [0.8, 0.9, 1.0, 1.1, 1.2, 1.3, 1.5, 2.0, 3.0].iter().any(|v| (v - m).abs() < 1e-9) {
            println!("{m:.1}  {:.2}  {:.2}  {xb:.3}  {:.3}  {:.3}  {:.3}", model.cna(m), c.cna, c.xcp, model.cd0(m, false).total(), c.ca_off);
        }
        // Mach 1.5: the local-inclination fins carry the body upwash (weight 0.96 here, `upwash_weight`),
        // the component build-up only Barrowman's fin factor with its rectangular tip loss: the panel value
        // is 25.5 % higher (Mach 2: 17 %), where the panel value agrees with Euler CFD.
        if (m - 1.5).abs() < 1e-9 {
            assert!((c.cna / model.cna(m) - 1.0).abs() < 0.30, "M=1.5 CNa {} vs {}", c.cna, model.cna(m));
        }
        // Mach 2: Euler CFD of this rocket 13.24 (doc/data/cfd/sample_euler_vs_models.csv).
        if (m - 2.0).abs() < 1e-9 {
            assert!((c.cna / 13.24 - 1.0).abs() < 0.10, "M=2 CNa {} vs Euler 13.24", c.cna);
        }
        if (m - 2.0).abs() < 1e-9 {
            assert!((c.xcp - xb).abs() < 0.06, "M=2 xcp {} vs {xb}", c.xcp);
        }
    }
    // No unphysical transonic peak. Euler CFD of this rocket: 18.50 at Mach 1.2 against 16.51 at
    // Mach 0.8 (doc/data/cfd/sample_euler_vs_models.csv); the local-inclination methods at the band
    // edge, where the fin leading edges already sit past wedge detachment, give about 1.36 times (the
    // body upwash is faded out there, `upwash_weight`).
    assert!(peak <= 1.4 * c08, "CNa peak {peak} vs M=0.8 value {c08}");
}

#[test]
fn memory_estimate_and_check() {
    assert_eq!(dense_solve_bytes(1000), 8_000_000 + 64_000);
    // Saturates instead of overflowing u64 (8 n^2 > 2^64 from n = 2^30.5 on).
    assert_eq!(dense_solve_bytes(1 << 30), 8 * (1u64 << 60) + 64 * (1u64 << 30));
    for n in [1usize << 31, 1 << 32, usize::MAX] {
        assert_eq!(dense_solve_bytes(n), u64::MAX, "{n}");
    }
    assert!(check_memory(1 << 40, Some(1 << 40)).is_err());
    assert!(check_memory(1000, None).is_ok());
    assert!(check_memory(1000, Some(17_000_000)).is_ok());
    let e = check_memory(20_000, Some(1_000_000_000)).unwrap_err().to_string();
    assert!(e.contains("coarser") && e.contains("resources.memory_gb") && e.starts_with("[IY-A019] "), "{e}");
    // Without a budget, a solve larger than the available memory (10^7 panels: 1.6 PB) fails cleanly.
    if std::path::Path::new("/proc/meminfo").exists() {
        let e = check_memory(10_000_000, None).unwrap_err().to_string();
        assert!(e.contains("memory available to this process") && e.starts_with("[IY-A018] "), "{e}");
    }
    // The available memory applies with a budget too, and the error names its source.
    let a = memory::Available { bytes: 1_000_000, source: "cgroup memory.max minus memory.current".into() };
    let e = check_memory_with(1000, Some(1 << 40), Some(a.clone())).unwrap_err().to_string();
    assert!(e.starts_with("[IY-A018] ") && e.contains("0.01 GB for 1000 panels") && e.contains("cgroup memory.max"), "{e}");
    assert!(check_memory_with(100, None, Some(a)).is_ok());
    assert!(check_memory_with(1000, None, None).is_ok());
}

/// An allocation that cannot succeed is an error with the size, not an abort in the solver.
#[test]
fn dense_allocation_failure_is_an_error() {
    let e = solver::reserve_dense(1 << 31).unwrap_err().to_string();
    assert!(e.starts_with("[IY-A020] could not allocate the dense panel matrix") && e.contains("2147483648 panels"), "{e}");
    assert_eq!(solver::reserve_dense(10).unwrap().len(), 100);
}

#[test]
fn build_table_fails_early_over_memory_budget() {
    let g = geometry(&SampleRocket::default());
    let r = check_memory(Mesh::from_geometry(&g, &coarse()).unwrap().panels.len(), Some(1000));
    assert!(r.is_err());
}

/// Beyond the attached-flow angle the subsonic columns relax to the component build-up model, whose
/// fins carry the flat-plate crossflow force at their planform centroid: from twice that angle on the
/// normal force and centre of pressure are the Barrowman values, and the transition is continuous and
/// monotone from the potential-flow result, which is unchanged up to the attached-flow angle.
#[test]
fn subsonic_large_angles_carry_the_fin_crossflow() {
    use aero::model::FIN_ALPHA_LINEAR;
    let g = geometry(&SampleRocket::default());
    let aero = AeroOptions { mach_min: 0.0, mach_max: 0.6, mach_step: 0.3, alpha_max_deg: 120.0, alpha_step_deg: 1.0, ..Default::default() };
    let opt = PanelOptions { subsonic_machs: vec![0.0, 0.3, 0.6], ..coarse() };
    let (t, _) = build_table(&g, &opt, &aero, "t".into(), Extrapolation::Linear).unwrap();
    let model = AeroModel::new(g.clone(), aero.clone());
    let a1 = FIN_ALPHA_LINEAR.to_degrees();
    let cl = |a: f64| 1.1 * g.planform_area / g.ref_area * a.sin().powi(2);
    for &m in &t.meta.machs {
        let c0 = t.lookup(m, 0.0);
        // attached-flow rows: potential flow `sin a cos a` plus the body crossflow term
        for deg in [1.0, 5.0, 10.0, a1] {
            let a = f64::to_radians(deg);
            let c = t.lookup(m, a);
            let cn = c0.cna * a.sin() * a.cos() + cl(a);
            let mom = c0.cna * c0.xcp * a.sin() * a.cos() + cl(a) * g.planform_centroid;
            assert!((c.cn - cn).abs() < 1e-9 * cn.abs().max(1.0), "M={m} {deg}°: CN {} vs {cn}", c.cn);
            assert!((c.xcp - mom / cn).abs() < 1e-9, "M={m} {deg}°: xcp {} vs {}", c.xcp, mom / cn);
        }
        for deg in [2.0 * a1, 45.0, 60.0, 90.0, 110.0] {
            let a = f64::to_radians(deg);
            let c = t.lookup(m, a);
            let (bcn, bx) = model.normal(m, a);
            assert!((c.cn - bcn).abs() < 1e-9 * bcn.abs(), "M={m} {deg}°: CN {} vs Barrowman {bcn}", c.cn);
            assert!((c.xcp - bx).abs() < 1e-9, "M={m} {deg}°: xcp {} vs Barrowman {bx}", c.xcp);
        }
        // At 90° the fin crossflow holds the centre of pressure aft of the body planform centroid.
        assert!(t.lookup(m, f64::to_radians(90.0)).xcp > g.planform_centroid + 0.05);
        // continuous and monotone through the transition
        let mut prev = t.lookup(m, f64::to_radians(a1)).cn;
        for deg in (a1.round() as i32 + 1)..=(2.0 * a1).round() as i32 {
            let c = t.lookup(m, f64::to_radians(deg as f64)).cn;
            assert!(c > prev && c - prev < 1.0, "M={m} {deg}°: CN {prev} -> {c}");
            prev = c;
        }
    }
}

/// Unit sphere centred at x = 1 (nose tip at the origin).
fn sphere_mesh(na: usize) -> Mesh {
    let st: Vec<f64> = (0..=na).map(|i| 1.0 - (PI * i as f64 / na as f64).cos()).collect();
    let r: Vec<f64> = (0..=na).map(|i| (PI * i as f64 / na as f64).sin()).collect();
    Mesh::body_of_revolution(&st, &r, 2 * na, 0.0).unwrap()
}

/// Every pressure force on a sphere points through its centre, so the moment about the nose tip is
/// the centre's lever arm times the normal force: the nose moment must include the axial pressure
/// force on the sloped surface (`x Fz - z Fx`), not only `x Fz`. Subsonic (linearised potential flow).
#[test]
fn subsonic_nose_moment_is_the_full_cross_product() {
    let mesh = sphere_mesh(20);
    let c = alpha_coeffs(&mesh, PI);
    // the omitted term alone, for scale
    let pot = solve_potential(&mesh).unwrap();
    let mut zfx: f64 = 0.0;
    for (i, p) in mesh.panels.iter().enumerate() {
        let cp_a = -2.0 * pot.vel[0][i].dot(pot.vel[1][i]);
        zfx += cp_a * -p.normal.x * p.area / PI * p.centroid.z;
    }
    println!("sphere: CNa {:.4} M_alpha {:.4} (x = 1: {:.4}), z Fx term {zfx:.4}", c.cna, c.m_alpha, c.cna);
    assert!(zfx.abs() > 0.1, "the axial-force term is significant on a sphere: {zfx}");
    assert!((c.m_alpha - c.cna).abs() < 0.05 * zfx.abs(), "M_alpha {} vs CNa x_centre {}", c.m_alpha, c.cna);
    assert!((c.m_rot - c.n_rot).abs() < 0.05 * zfx.abs().max(c.n_rot.abs()), "M_rot {} vs N_rot x_centre {}", c.m_rot, c.n_rot);
}

/// As above for the supersonic local-inclination methods: on a sphere the centre of pressure is the
/// centre (x = 1), for the forces at any angle and for the derivatives (within the 2-3 % offset of
/// flat panels: a panel normal through its centroid misses the centre slightly).
#[test]
fn supersonic_nose_moment_is_the_full_cross_product() {
    let mesh = sphere_mesh(80);
    let sp = supersonic::SupersonicPanels::new(&mesh, PI, None, None);
    for deg in [5.0f64, 30.0, 60.0] {
        let f = sp.forces(2.0, deg.to_radians(), 0.0);
        assert!((f.mz / f.fz - 1.0).abs() < 0.03, "{deg}°: xcp {}", f.mz / f.fz);
    }
    let d = sp.derivatives(2.0, 2.0);
    println!("sphere M=2: CNa {:.4} xcp(0) {:.4} N_rot {:.4} M_rot {:.4}", d[0], d[1] / d[0], d[2], d[3]);
    assert!((d[1] / d[0] - 1.0).abs() < 0.03, "xcp(0) {}", d[1] / d[0]);
    assert!((d[3] / d[2] - 1.0).abs() < 0.03, "M_rot {} N_rot {}", d[3], d[2]);
}

/// Invalid panel options are errors (not panics, not silently wrong tables).
#[test]
fn panel_options_are_validated() {
    let ok = PanelOptions::default();
    assert!(ok.validate().is_ok());
    let nan = f64::NAN;
    let bad: Vec<(PanelOptions, &str)> = vec![
        (PanelOptions { subsonic_machs: vec![nan, 0.3], ..ok.clone() }, "subsonic_machs"),
        (PanelOptions { subsonic_machs: vec![], ..ok.clone() }, "subsonic_machs"),
        (PanelOptions { subsonic_machs: vec![0.3, 1.0], ..ok.clone() }, "subsonic_machs"),
        (PanelOptions { subsonic_machs: vec![-0.1, 0.3], ..ok.clone() }, "subsonic_machs"),
        (PanelOptions { tail_radii: -1.0, ..ok.clone() }, "tail_radii"),
        (PanelOptions { tail_radii: nan, ..ok.clone() }, "tail_radii"),
        (PanelOptions { wake_length: 0.0, ..ok.clone() }, "wake_length"),
        (PanelOptions { wake_length: -5.0, ..ok.clone() }, "wake_length"),
        (PanelOptions { wake_length: nan, ..ok.clone() }, "wake_length"),
        (PanelOptions { transonic: [nan, 1.2], ..ok.clone() }, "transonic"),
        (PanelOptions { transonic: [0.8, nan], ..ok.clone() }, "transonic"),
        (PanelOptions { transonic: [0.8, 0.95], ..ok.clone() }, "transonic"),
        (PanelOptions { transonic: [1.2, 0.8], ..ok.clone() }, "transonic"),
        (PanelOptions { transonic: [0.0, 1.2], ..ok.clone() }, "transonic"),
        (PanelOptions { body_axial: 5, ..ok.clone() }, "body_axial"),
        (PanelOptions { body_circ: 7, ..ok.clone() }, "body_circ"),
        (PanelOptions { fin_chord: 1, ..ok.clone() }, "fin_chord"),
        (PanelOptions { fin_span: 1, ..ok.clone() }, "fin_span"),
    ];
    for (o, key) in bad {
        let e = o.validate().unwrap_err().to_string();
        assert!(e.contains(&format!("aero.panel.{key}")), "{key}: {e}");
        assert!(e.starts_with("[IY-A01"), "{key}: {e}");
    }
    assert!(PanelOptions { transonic: [0.7, 1.2], ..ok.clone() }.validate().is_ok());
    // build_table reports the error instead of panicking in the sort
    let g = geometry(&SampleRocket::default());
    let o = PanelOptions { subsonic_machs: vec![nan, 0.3], ..coarse() };
    assert!(build_table(&g, &o, &small_aero(), "t".into(), Extrapolation::Linear).is_err());
}

/// The transonic band runs over the configured `[lo, hi]`: up to `lo` the columns are the subsonic
/// panel solution, from `hi` on the supersonic one.
#[test]
fn transonic_band_starts_at_its_lower_edge() {
    let g = geometry(&SampleRocket::default());
    let aero = AeroOptions { mach_min: 0.6, mach_max: 1.3, mach_step: 0.05, alpha_max_deg: 4.0, alpha_step_deg: 2.0, ..Default::default() };
    let wide = PanelOptions { subsonic_machs: vec![0.0, 0.3, 0.6, 0.8], transonic: [0.7, 1.2], ..coarse() };
    let narrow = PanelOptions { transonic: [0.8, 1.2], ..wide.clone() };
    let (tw, _) = build_table(&g, &wide, &aero, "t".into(), Extrapolation::Linear).unwrap();
    let (tn, _) = build_table(&g, &narrow, &aero, "t".into(), Extrapolation::Linear).unwrap();
    let a = 2f64.to_radians();
    for m in [0.6, 0.65, 0.7] {
        assert_eq!(tw.lookup(m, a).cn, tn.lookup(m, a).cn, "M={m}");
    }
    assert!((tw.lookup(0.75, a).cn - tn.lookup(0.75, a).cn).abs() > 1e-9);
    for m in [1.2, 1.25, 1.3] {
        assert_eq!(tw.lookup(m, a).cn, tn.lookup(m, a).cn, "M={m}");
    }
    // above the highest subsonic solution the band starts there (no extrapolation of the subsonic columns)
    let above = PanelOptions { transonic: [0.9, 1.2], ..wide.clone() };
    let (ta, _) = build_table(&g, &above, &aero, "t".into(), Extrapolation::Linear).unwrap();
    for &m in &ta.meta.machs {
        assert_eq!(ta.lookup(m, a).cn, tn.lookup(m, a).cn, "M={m}");
    }
}

/// The two-dimensional crossflow around the body cylinder: raised by `1 + r^2 / rho^2` normal to the
/// crossflow (fins normal to it), lowered by `1 - r^2 / rho^2` along it (no flow through the
/// cylinder), unchanged far away, and linear in the crossflow.
#[test]
fn cylinder_crossflow_upwash() {
    use supersonic::cylinder_crossflow as cf;
    let r = 0.5;
    let (vy, vz) = cf(1.0, 0.0, r, 0.0, 2.0);
    assert!(vy.abs() < 1e-12 && (vz - 2.0 * 1.25).abs() < 1e-12, "{vy} {vz}");
    let (vy, vz) = cf(0.0, -1.0, r, 0.0, 2.0);
    assert!(vy.abs() < 1e-12 && (vz - 2.0 * 0.75).abs() < 1e-12, "{vy} {vz}");
    // Crossflow along y: the fin in the plane z = 0 is aligned with it, the one in y = 0 normal to it.
    let (vy, vz) = cf(1.0, 0.0, r, 2.0, 0.0);
    assert!((vy - 1.5).abs() < 1e-12 && vz.abs() < 1e-12, "{vy} {vz}");
    let (vy, vz) = cf(0.0, 1.0, r, 2.0, 0.0);
    assert!((vy - 2.5).abs() < 1e-12 && vz.abs() < 1e-12, "{vy} {vz}");
    let (vy, vz) = cf(300.0, 400.0, r, 0.3, -0.7);
    assert!((vy - 0.3).abs() < 1e-6 && (vz + 0.7).abs() < 1e-6);
    // No body: the freestream crossflow.
    assert_eq!(cf(0.1, 0.2, 0.0, 0.3, -0.7), (0.3, -0.7));
    // On the surface the flow is tangential: radial component zero.
    let th = 0.7f64;
    let (y, z) = (r * th.cos(), r * th.sin());
    let (vy, vz) = cf(y, z, r, 0.4, 1.1);
    assert!((vy * th.cos() + vz * th.sin()).abs() < 1e-12);
}

/// Supersonic fin-body interference of the local-inclination method: four rectangular fins (no sweep,
/// r / (s + r) = 1/3, the proportions of the Basic Finner) on a cone-cylinder at Mach 2. The fin normal
/// force with the body upwash over that without it is the planform average of `1 + r^2 / y^2`
/// weighted with the tip relief (half loading in the tip Mach cone), about Barrowman's
/// `1 + r / (s + r) = 4/3`; and the fin normal-force slope then agrees with Barrowman's (Ackeret
/// `4 / beta` with the same tip loss and interference factor) within 6 %.
#[test]
fn supersonic_fins_see_the_body_upwash() {
    use geom::sample::NoseShape;
    let base = SampleRocket {
        nose_shape: NoseShape::Cone,
        nose_length: 0.05 / 10f64.to_radians().tan(),
        tip_chord: 0.10,
        root_chord: 0.10,
        span: 0.10,
        sweep: 0.0,
        ..Default::default()
    };
    let g = geometry(&base);
    let f = g.fins.clone().unwrap();
    let (r, s, c) = (f.body_radius, f.span, f.root_chord);
    let mesh = Mesh::from_geometry(&g, &coarse()).unwrap();
    let tip = Some((f.x_le_root + f.sweep, r + s));
    let m = 2.0;
    let with = supersonic::SupersonicPanels::new(&mesh, g.ref_area, tip, Some((r, f.x_le_root + c))).derivatives(m, g.length)[0];
    let without = supersonic::SupersonicPanels::new(&mesh, g.ref_area, tip, None).derivatives(m, g.length)[0];
    let gb = geometry(&SampleRocket { fin_count: 0, ..base.clone() });
    let mb = Mesh::from_geometry(&gb, &coarse()).unwrap();
    let body = supersonic::SupersonicPanels::new(&mb, gb.ref_area, None, None).derivatives(m, gb.length)[0];
    let ratio = (with - body) / (without - body);
    // Expected: upwash averaged over the planform with the tip relief weight.
    let beta = (m * m - 1.0f64).sqrt();
    let (n, mut num, mut den) = (400, 0.0, 0.0);
    for i in 0..n {
        for j in 0..n {
            let x = (i as f64 + 0.5) / n as f64 * c;
            let y = r + (j as f64 + 0.5) / n as f64 * s;
            let wgt = if x >= beta * (r + s - y) { 0.5 } else { 1.0 };
            num += wgt * (1.0 + r * r / (y * y));
            den += wgt;
        }
    }
    let expect = num / den;
    let model = AeroModel::new(g.clone(), AeroOptions::default());
    let fin_barrowman = model.cna(m) - model.body_cna();
    println!(
        "M=2 fins: with upwash {:.3}, without {:.3}, ratio {ratio:.4} (expected {expect:.4}, Barrowman 1 + r/(s+r) = {:.4}); Barrowman fin CNa {fin_barrowman:.3}",
        with - body,
        without - body,
        1.0 + r / (s + r)
    );
    assert!((ratio / expect - 1.0).abs() < 0.03, "ratio {ratio} vs {expect}");
    assert!(((with - body) / fin_barrowman - 1.0).abs() < 0.06, "fin CNa {} vs Barrowman {fin_barrowman}", with - body);
}

/// Validity weight of the body upwash: 0 up to q = 1/2 (whole fin in the interacting tip region),
/// 1 from q = 1 (tip Mach cone clear of the root), C1 smoothstep between; and the sample rocket gets
/// almost none at the edge of the transonic band (Mach 1.2), the full upwash at Mach 2.
#[test]
fn upwash_is_faded_out_where_the_tip_cone_covers_the_fin() {
    use supersonic::upwash_weight as w;
    assert_eq!(w(0.0), 0.0);
    assert_eq!(w(0.5), 0.0);
    assert_eq!(w(1.0), 1.0);
    assert_eq!(w(5.0), 1.0);
    assert_eq!(w(f64::INFINITY), 1.0);
    assert!((w(0.75) - 0.5).abs() < 1e-12);
    let h = 1e-6;
    for q in [0.5, 1.0] {
        assert!(((w(q + h) - w(q - h)) / (2.0 * h)).abs() < 1e-3, "slope at {q}");
    }
    let mut prev = 0.0;
    for k in 0..=200 {
        let v = w(0.4 + 0.004 * k as f64);
        assert!(v >= prev);
        prev = v;
    }
    let g = geometry(&SampleRocket::default());
    let f = g.fins.clone().unwrap();
    let mesh = Mesh::from_geometry(&g, &coarse()).unwrap();
    let sp = supersonic::SupersonicPanels::new(
        &mesh,
        g.ref_area,
        Some((f.x_le_root + f.sweep, f.body_radius + f.span)),
        Some((f.body_radius, f.x_le_root + f.root_chord)),
    );
    let (_, k12) = sp.upwash(1.2);
    let (_, k2) = sp.upwash(2.0);
    println!("sample upwash weight: M=1.2 {k12:.3}, M=2 {k2:.3}");
    assert!(k12 < 0.05 && k2 == 1.0, "{k12} {k2}");
}
