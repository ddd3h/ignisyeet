// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025-2026 西濱大将 (NISHIHAMA Daisuke)
use super::*;
use crate::mesh::Mesh;
use crate::solver::{coefficients, solve_potential};
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
    let pot = solve_potential(&mesh);
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
    let pot = solve_potential(&mesh);
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
    let pot = solve_potential(mesh);
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
    g.fins = Some(FinSet { count: 2, root_chord: c, tip_chord: c, span: s, sweep: 0.0, thickness: 0.01, x_le_root: 0.6, body_radius: r });
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
    println!("M=0: panel CNa {:.3} xcp {:.4}; panels {} wake {} solve {:.1}s", rep.subsonic[0].cna, rep.subsonic[0].xcp, rep.panels, rep.wake_panels, rep.solve_seconds);
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
        assert!(c.cn.is_finite() && dcn < 0.05 && dca < 0.05, "jump at M={m:.2}: cn {} -> {}, ca {} -> {}", prev.cn, c.cn, prev.ca_off, c.ca_off);
        prev = c;
    }
    println!("max step change over 0.02 Mach: CN {:.3}, CA {:.3}", max_cn, max_ca);
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
            c.cn, c.xcp, c.cna, c.ca_off, c.damp[0], c.damp[1], c.damp[2], model.cna(m), model.cd0(m, false).total(), model.damping_sums(m)
        );
    }
    let path = std::env::temp_dir().join("ignisyeet_panel_cp_test.csv");
    write_surface_cp(&path, &rep).unwrap();
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(text.starts_with("x,y,z,nx,ny,nz,area,cp,part") && text.contains(",fin") && text.contains(",body"));
    let vtk = std::env::temp_dir().join("ignisyeet_panel_mesh_test.vtk");
    Mesh::from_geometry(&g, &coarse()).unwrap().write_vtk(&vtk).unwrap();
}



#[test]
#[ignore = "timing run of the default mesh and full default table grid"]
fn default_table_timing() {
    let g = geometry(&SampleRocket::default());
    let t0 = std::time::Instant::now();
    let (t, rep) = build_table(&g, &PanelOptions::default(), &AeroOptions::default(), "t".into(), Extrapolation::Linear).unwrap();
    println!("default: {} panels, {} wake, solves {:.1}s, total {:.1}s, {} rows", rep.panels, rep.wake_panels, rep.solve_seconds, t0.elapsed().as_secs_f64(), t.rows());
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
    let sp = supersonic::SupersonicPanels::new(&mesh, g.ref_area, None);
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
        if (m - 1.5).abs() < 1e-9 {
            assert!((c.cna / model.cna(m) - 1.0).abs() < 0.25, "M=1.5 CNa {} vs {}", c.cna, model.cna(m));
        }
        if (m - 2.0).abs() < 1e-9 {
            assert!((c.xcp - xb).abs() < 0.06, "M=2 xcp {} vs {xb}", c.xcp);
        }
    }
    assert!(peak <= 1.35 * c08, "CNa peak {peak} vs M=0.8 value {c08}");
}
