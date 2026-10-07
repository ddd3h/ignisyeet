// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025-2026 西濱大将 (NISHIHAMA Daisuke)
//! `surface = "step"` with the real gmsh / SU2 tools on the fixture `examples/sample_rocket.step` (the sample rocket
//! with a launch lug and two rail buttons). Needs `IGNISYEET_CFD_PREFIX` (or the tools in `PATH`); run with
//! `cargo test --release -p cfd --test real_step -- --ignored --test-threads 1`.

use cfd::{BaseTreatment, CfdOptions, FlowModel, ResourceBudget, StepUnit, SurfaceKind};
use geom::stl::Triangle;
use geom::{extract, ExtractOptions, Vec3};
use std::path::{Path, PathBuf};

fn examples() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples")
}

fn sample_stl() -> (Vec<Triangle>, geom::Geometry) {
    let raw: Vec<Triangle> = geom::stl::read_stl(&examples().join("sample_rocket.stl")).unwrap().into_iter().map(|t| t.map(|v| v * 0.001)).collect();
    let g = extract(&raw, &ExtractOptions::default()).unwrap();
    (raw, g)
}

fn step_options() -> CfdOptions {
    CfdOptions {
        surface: SurfaceKind::Step,
        step_file: examples().join("sample_rocket.step"),
        base_treatment: Some(BaseTreatment::Fairing),
        machs: vec![0.8],
        alphas_deg: vec![0.0, 4.0],
        wall_size: 0.005,
        ranks_per_case: 4,
        parallel_cases: 1,
        ..Default::default()
    }
}

fn budget() -> ResourceBudget {
    ResourceBudget { threads: 4, memory_bytes: Some(8_000_000_000) }
}

fn tmp(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("ignisyeet_step_{name}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    d
}

/// Cells (SU2 type, nodes), node coordinates and marker tags of an SU2 mesh file.
type Su2Mesh = (Vec<(u32, Vec<usize>)>, Vec<[f64; 3]>, Vec<String>);

fn read_su2(path: &Path) -> Su2Mesh {
    let text = std::fs::read_to_string(path).unwrap();
    let mut lines = text.lines();
    let (mut cells, mut nodes, mut marks) = (Vec::new(), Vec::new(), Vec::new());
    let count = |l: &str| l.split('=').nth(1).unwrap().trim().parse::<usize>().unwrap();
    while let Some(l) = lines.next() {
        if l.starts_with("NELEM=") {
            for _ in 0..count(l) {
                let v: Vec<usize> = lines.next().unwrap().split_whitespace().map(|v| v.parse().unwrap()).collect();
                cells.push((v[0] as u32, v[1..v.len() - 1].to_vec()));
            }
        } else if l.starts_with("NPOIN=") {
            for _ in 0..count(l) {
                let v: Vec<f64> = lines.next().unwrap().split_whitespace().take(3).map(|v| v.parse().unwrap()).collect();
                nodes.push([v[0], v[1], v[2]]);
            }
        } else if let Some(tag) = l.strip_prefix("MARKER_TAG=") {
            let n = count(lines.next().unwrap());
            for _ in 0..n {
                lines.next();
            }
            marks.push(tag.trim().to_string());
        }
    }
    (cells, nodes, marks)
}

/// Signed volume of an SU2 cell (tetrahedron 10, prism 13 in the VTK node order).
fn su2_volume(kind: u32, c: &[usize], x: &[[f64; 3]]) -> f64 {
    let p = |i: usize| Vec3::new(x[c[i]][0], x[c[i]][1], x[c[i]][2]);
    let tet = |a: usize, b: usize, cc: usize, d: usize| (p(b) - p(a)).cross(p(cc) - p(a)).dot(p(d) - p(a)) / 6.0;
    match kind {
        10 => tet(0, 1, 2, 3),
        13 => tet(0, 2, 1, 3) + tet(2, 1, 3, 5) + tet(1, 3, 5, 4),
        _ => panic!("unexpected cell type {kind}"),
    }
}

/// Runs SU2 for one iteration on `mesh` and returns its log.
fn su2_one_iteration(opt: &CfdOptions, dir: &Path, mesh: &Path, markers: &[String]) -> String {
    let tools = cfd::tools::check_tools(opt, &[]);
    let su2 = tools.su2.path.clone().expect("SU2_CFD");
    let wall: Vec<&str> = markers.iter().map(String::as_str).filter(|m| *m == "wall" || *m == "base").collect();
    let sym = if markers.iter().any(|m| m == "symmetry") { "MARKER_SYM= ( symmetry )\n" } else { "" };
    let cfg = dir.join("one_iteration.cfg");
    std::fs::write(
        &cfg,
        format!(
            "SOLVER= EULER\nMACH_NUMBER= 0.8\nAOA= 4.0\nFREESTREAM_PRESSURE= 101325.0\nFREESTREAM_TEMPERATURE= 288.15\nREF_LENGTH= 0.1\nREF_AREA= 0.00785\n\
             MARKER_EULER= ( {} )\nMARKER_FAR= ( farfield )\n{sym}MARKER_MONITORING= ( {} )\nNUM_METHOD_GRAD= WEIGHTED_LEAST_SQUARES\nCFL_NUMBER= 1.0\n\
             CONV_NUM_METHOD_FLOW= ROE\nITER= 1\nMESH_FILENAME= {}\nMESH_FORMAT= SU2\nOUTPUT_FILES= NONE\n",
            wall.join(", "),
            wall.join(", "),
            mesh.display()
        ),
    )
    .unwrap();
    let out = cfd::tools::command(&su2, tools.prefix.as_deref()).arg("-t").arg("4").arg(&cfg).current_dir(dir).output().unwrap();
    let log = format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
    assert!(out.status.success() && log.contains("Exit Success"), "{log}");
    log
}

#[test]
#[ignore = "needs SU2 and gmsh (IGNISYEET_CFD_PREFIX); about 5 minutes"]
fn step_fixture_meshes_the_full_domain_and_su2_reads_it() {
    let (raw, g) = sample_stl();
    let dir = tmp("euler");
    let t = std::time::Instant::now();
    let m = cfd::mesh_only(&g, &step_options(), &aero::AeroOptions::default(), &dir, &budget(), Some(&raw)).unwrap();
    let info = m.step.as_ref().unwrap();
    eprintln!("{:?} in {:.1?}\n{}", m.stats, t.elapsed(), info.summary());
    // The lug and the buttons make the shape asymmetric about both planes: full domain.
    assert_eq!((info.symmetry.chosen.as_str(), info.symmetry.meshed.as_str(), info.domain.as_str()), ("none", "full", "full"));
    assert!(!info.symmetry.y.symmetric && !info.symmetry.z.symmetric);
    // Registered against the STL; the three details are STEP-only regions.
    assert!(info.registration.ok() && info.registration.stl_to_step.max < info.registration.threshold, "{}", info.registration.summary());
    assert_eq!(info.registration.step_only_regions, 3, "{}", info.registration.summary());
    assert!(info.registration.bbox_offsets.iter().all(|d| d.abs() < 1e-3), "{:?}", info.registration.bbox_offsets);
    assert_eq!((info.solids, info.format.as_str(), info.file_unit_m), (1, "step", Some(0.001)));
    // The lug wall (0.775 mm) sets the smallest size: the lower bound wall_size / 8.
    assert!(info.smallest_feature < 0.001 && (info.min_size - 0.005 / 8.0).abs() < 1e-12, "{} {}", info.smallest_feature, info.min_size);
    assert_eq!(info.base_treatment, "fairing");
    assert!(m.stats.cells > 200_000 && m.stats.cells_bad == 0, "{:?}", m.stats);
    let (cells, nodes, marks) = read_su2(&m.su2);
    assert_eq!(cells.len(), m.stats.cells);
    assert!(cells.iter().all(|(k, c)| su2_volume(*k, c, &nodes) > 0.0));
    assert_eq!(marks, ["wall", "base", "farfield"]);
    assert!(nodes.iter().any(|p| p[1] < -1.0) && nodes.iter().any(|p| p[2] < -1.0), "full box farfield");
    let log = su2_one_iteration(&step_options(), &dir, &m.su2, &marks);
    assert!(log.contains("All volume elements are correctly oriented") || !log.contains("elements are not correctly oriented"), "{log}");
    // The cached mesh is reused.
    let again = cfd::mesh_only(&g, &step_options(), &aero::AeroOptions::default(), &dir, &budget(), Some(&raw)).unwrap();
    assert!(again.cached && again.hash == m.hash);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
#[ignore = "needs gmsh (IGNISYEET_CFD_PREFIX)"]
fn step_registration_catches_a_wrong_unit() {
    let (raw, g) = sample_stl();
    let dir = tmp("unit");
    // The fixture is in millimetres; read as metres it is 1000 times too large.
    let opt = CfdOptions { step_unit: StepUnit::M, ..step_options() };
    let (reg, _) = cfd::step_mesh::check_step(&opt, &g, &raw, &dir).unwrap();
    assert!(!reg.ok());
    let e = reg.check(&opt.step_file).unwrap_err().to_string();
    assert!(e.contains("does not describe the same rocket") && e.contains("units") && e.contains("millimetres but read as metres"), "{e}");
    let e = cfd::mesh_only(&g, &opt, &aero::AeroOptions::default(), &dir, &budget(), Some(&raw)).unwrap_err().to_string();
    assert!(e.contains("step_unit"), "{e}");
    // The right unit given explicitly: the same shape as `auto`.
    let (reg, d) = cfd::step_mesh::check_step(&CfdOptions { step_unit: StepUnit::Mm, ..step_options() }, &g, &raw, &dir).unwrap();
    assert!(reg.ok() && d.chosen == "none", "{}", reg.summary());
    let _ = std::fs::remove_dir_all(&dir);
}

/// Writes the fixture without the lug and the buttons (symmetric about y = 0 and z = 0) with the generator script.
fn symmetric_step(dir: &Path) -> PathBuf {
    let (raw, g) = sample_stl();
    let (profile, fins) = cfd::mesh::cad_description(&g);
    let f = fins.clone().unwrap();
    let (mut s, mut c) = (0.0, 0.0);
    for v in cfd::surface::to_cfd_frame(&raw, &g).iter().flatten() {
        if v.y.hypot(v.z) > f.body_radius + 0.5 * f.span {
            let a = f.count as f64 * (v.z.atan2(v.y) - std::f64::consts::FRAC_PI_2);
            (s, c) = (s + a.sin(), c + a.cos());
        }
    }
    let out = dir.join("symmetric.step");
    std::fs::create_dir_all(dir).unwrap();
    let params = serde_json::json!({
        "profile": profile, "fins": fins, "fin_roll_deg": s.atan2(c).to_degrees() / f.count as f64, "length": g.length,
        "affine": cfd::step_mesh::cfd_affine(&g), "out": out, "details": false,
    });
    std::fs::write(dir.join("gen.json"), params.to_string()).unwrap();
    let tools = cfd::tools::check_tools(&CfdOptions::default(), &[]);
    let st = cfd::tools::command(tools.gmsh.path.as_ref().unwrap(), tools.prefix.as_deref())
        .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/sample_step.py"))
        .arg(dir.join("gen.json"))
        .status()
        .unwrap();
    assert!(st.success());
    out
}

#[test]
#[ignore = "needs gmsh (IGNISYEET_CFD_PREFIX)"]
fn symmetric_step_uses_the_quarter_and_half_domains() {
    let (raw, g) = sample_stl();
    let dir = tmp("sym");
    let file = symmetric_step(&dir.join("cad"));
    let opt = CfdOptions { step_file: file, ..step_options() };
    let (reg, d) = cfd::step_mesh::check_step(&opt, &g, &raw, &dir).unwrap();
    assert!(reg.ok() && reg.step_only_regions == 0, "{}", reg.summary());
    assert!(d.y.symmetric && d.z.symmetric, "{}", d.summary());
    assert_eq!((d.chosen.as_str(), d.meshed.as_str()), ("axisymmetric", "quarter"));
    // RANS never meshes the quarter.
    let (_, r) = cfd::step_mesh::check_step(&CfdOptions { model: FlowModel::Rans, base_treatment: None, ..opt.clone() }, &g, &raw, &dir.join("rans")).unwrap();
    assert_eq!(r.meshed, "half");
    // Euler: the quarter mirrored about z = 0 and kept as the SU2 half domain, or the half when OpenCASCADE cannot cut
    // the quarter from the imported shape (the edge y = z = 0 runs through the nose tip).
    let m = cfd::mesh_only(&g, &opt, &aero::AeroOptions::default(), &dir, &budget(), Some(&raw)).unwrap();
    let info = m.step.as_ref().unwrap();
    eprintln!("{}", info.summary());
    assert!(info.domain == "quarter_half" || (info.domain == "half" && info.symmetry.reason.contains("quarter domain cannot be cut")), "{}", info.domain);
    let (cells, nodes, marks) = read_su2(&m.su2);
    assert_eq!(marks, ["wall", "base", "farfield", "symmetry"]);
    assert!(cells.iter().all(|(k, c)| su2_volume(*k, c, &nodes) > 0.0));
    assert!(nodes.iter().all(|p| p[1] >= 0.0) && nodes.iter().any(|p| p[2] < -1.0));
    su2_one_iteration(&opt, &dir, &m.su2, &marks);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
#[ignore = "needs gmsh (IGNISYEET_CFD_PREFIX); about 10 minutes"]
fn rans_on_the_step_fixture_is_valid_or_names_the_sharp_edges() {
    let (raw, g) = sample_stl();
    let dir = tmp("rans");
    let opt = CfdOptions { model: FlowModel::Rans, base_treatment: None, machs: vec![0.8, 2.0], ..step_options() };
    let t = std::time::Instant::now();
    match cfd::mesh_only(&g, &opt, &aero::AeroOptions::default(), &dir, &budget(), Some(&raw)) {
        Ok(m) => {
            eprintln!("RANS mesh of the STEP fixture: {:?} in {:.1?}", m.stats, t.elapsed());
            let s = &m.stats;
            assert_eq!(s.bl_invalid, 0);
            // Thin columns at the lug and the rail buttons grow by the layer ratio only over their top layers, and the
            // tetrahedra on them start from their last layer (gap mode): the size jump stays below 8 at the 99th
            // percentile (14.5 without both).
            assert!(s.bl_top_ratio_p99 < 8.0, "{s:?}");
            assert!(s.by_type.get("Prism 6").copied().unwrap_or(0) > 100_000, "{:?}", s.by_type);
            let (cells, nodes, marks) = read_su2(&m.su2);
            assert!(cells.iter().all(|(k, c)| su2_volume(*k, c, &nodes) > 0.0));
            assert_eq!(marks, ["wall", "base", "farfield"]);
        }
        Err(e) => {
            let e = e.to_string();
            eprintln!("RANS mesh of the STEP fixture failed after {:.1?}: {e}", t.elapsed());
            assert!(e.contains("use model = \"euler\", or round the sharp edges"), "{e}");
        }
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
#[ignore = "needs the python of the CFD tools with numpy (IGNISYEET_CFD_PREFIX)"]
fn bl_layer_finds_opposite_walls_and_crossing_columns_and_pads_thin_columns() {
    // Two parallel plates 4 mm apart (the slot under a rail button flange) and a single plate.
    let dir = tmp("bl_gap");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("bl_layer.py"), cfd::mesh::BL_LAYER).unwrap();
    std::fs::write(
        dir.join("check.py"),
        r#"import sys
import numpy as np
import bl_layer as b
g = np.linspace(0, 0.05, 11)
X, Y = np.meshgrid(g, g)
P = np.c_[X.ravel(), Y.ravel()]
T = np.array([t for i in range(10) for j in range(10) for t in ([i * 11 + j, i * 11 + j + 1, i * 11 + j + 12], [i * 11 + j, i * 11 + j + 12, i * 11 + j + 11])])
low, up = np.c_[P, np.zeros(len(P))], np.c_[P, np.full(len(P), 0.004)]
x = np.vstack([low, up])
n = np.vstack([np.tile([0, 0, 1.0], (len(P), 1)), np.tile([0, 0, -1.0], (len(P), 1))])
tri = np.vstack([T, T + len(P)])
d = b.wall_hits(x, n, tri, 0.05, 0.005)
assert abs(d.min() - 0.004) < 1e-12 and abs(d.max() - 0.004) < 1e-12, d
assert np.isinf(b.wall_hits(low, n[: len(P)], T, 0.05, 0.005)).all()
cap = b.column_collisions(x, n, np.full(len(x), 0.01), np.full(len(x), 0.005))
assert (cap < 0.004).all(), cap
assert np.isinf(b.column_collisions(low, n[: len(P)], np.full(len(P), 0.01), np.full(len(P), 0.005))).all()
# Padded heights: the full column is the geometric one, a thin column keeps the growth and a large last layer.
h1, nl = 5.71e-7, 49
full = h1 * (1.2 ** nl - 1.0) / 0.2
s = b.column_heights_padded(h1, nl, np.array([full, 1e-4, 1e-3]), 1.2)
g = b.column_heights(h1, nl, np.array([full]))
assert np.abs(s[:, 0] - g[:, 0]).max() < 1e-5 * full
assert np.allclose(s[-1], [full, 1e-4, 1e-3])
d = np.diff(s, axis=0)
assert d.min() > 0.0 and (d[1:] / d[:-1]).max() < 1.2 + 1e-9
assert d[-1, 1] > 0.1 * 1e-4 and d[-1, 2] > 0.15 * 1e-3, d[-1]
print("BL_OK")
"#,
    )
    .unwrap();
    let tools = cfd::tools::check_tools(&CfdOptions::default(), &[]);
    let out = cfd::tools::command(tools.gmsh.path.as_ref().unwrap(), tools.prefix.as_deref()).arg(dir.join("check.py")).current_dir(&dir).output().unwrap();
    let log = format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
    assert!(out.status.success() && log.contains("BL_OK"), "{log}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
#[ignore = "needs gmsh (IGNISYEET_CFD_PREFIX)"]
fn symmetry_check_accepts_the_basic_finner_and_rejects_the_lug_and_buttons() {
    let dir = tmp("symtol");
    // Basic Finner (0.3 m): symmetric about y = 0 and z = 0; the B-spline projection accuracy exceeds SYMMETRY_TOL x L.
    let raw: Vec<Triangle> =
        geom::stl::read_stl(&examples().join("basic_finner/basic_finner.stl")).unwrap().into_iter().map(|t| t.map(|v| v * 0.001)).collect();
    let g = extract(&raw, &ExtractOptions::default()).unwrap();
    let opt = CfdOptions { step_file: examples().join("basic_finner/basic_finner.step"), base_treatment: Some(BaseTreatment::Flat), ..step_options() };
    let (_, d) = cfd::step_mesh::check_step(&opt, &g, &raw, &dir.join("finner")).unwrap();
    assert!(d.y.symmetric && d.z.symmetric, "{}", d.summary());
    assert_eq!(d.chosen, "axisymmetric");
    // The sample rocket with its lug and two rail buttons is not symmetric.
    let (raw, g) = sample_stl();
    let (_, d) = cfd::step_mesh::check_step(&step_options(), &g, &raw, &dir.join("sample")).unwrap();
    assert!(!d.y.symmetric || !d.z.symmetric, "{}", d.summary());
    assert_eq!(d.chosen, "none");
    let _ = std::fs::remove_dir_all(&dir);
}
