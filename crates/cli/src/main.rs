// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025-2026 西濱大将 (NISHIHAMA Daisuke)
mod config;
mod kml;
mod ui;
mod view;

use aero::{AeroModel, AeroOptions, AeroTable};
use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use config::Config;
use geom::{ExtractOptions, Geometry};
use serde::Serialize;
use sim::{Descent, Motor, Simulation, Summary};
use std::io::Write;
use std::path::{Path, PathBuf};
use ui::ui;
use view::Sub;

/// `--version` output; doubles as the Appropriate Legal Notice (GPLv3 section 7(b)).
const LONG_VERSION: &str = concat!(
    env!("CARGO_PKG_VERSION"),
    " \u{2014} Copyright (C) 2025-2026 西濱大将 (NISHIHAMA Daisuke).\n",
    "License GPL-3.0-or-later: <https://www.gnu.org/licenses/>.\n",
    "This is free software; there is NO WARRANTY."
);

#[derive(Parser)]
#[command(
    name = "ignisyeet",
    display_name = "IgnisYeet",
    version,
    long_version = LONG_VERSION,
    about = "IgnisYeet by 西濱大将 (NISHIHAMA Daisuke): STL-based rocket aerodynamics, 6-DoF trajectory and landing dispersion",
    after_help = "Exit status: 0 success (a flight that landed exits 0 even when it is statically unstable: check `stable` and `static_margin_min_cal` in summary.json); 1 error (invalid configuration, missing input, failed step); 2 usage error; \
3 the run finished and wrote its outputs but the results are incomplete (a flight did not land within sim.max_time, or dispersion cases/samples failed or did not land).\n\n\
Copyright (C) 2025-2026 西濱大将 (NISHIHAMA Daisuke). License GPL-3.0-or-later: <https://www.gnu.org/licenses/>. This is free software; there is NO WARRANTY."
)]
struct Cli {
    /// Only print errors and the paths of the written files.
    #[arg(short, long, global = true)]
    quiet: bool,
    /// Do not draw progress bars and spinners (the styled summaries stay).
    #[arg(long, global = true)]
    no_progress: bool,
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Extract body profile and fins from the STL (writes geometry.json, profile.csv).
    Geom { config: PathBuf },
    /// Build the aerodynamic coefficient table (cached; --force rebuilds).
    Aero {
        config: PathBuf,
        #[arg(long)]
        force: bool,
    },
    /// Run one 6-DoF flight with the configured wind (writes trajectory.csv, summary.json). Exits with status 3 when the rocket did not land within sim.max_time.
    Sim {
        config: PathBuf,
        /// Override the descent mode.
        #[arg(long, value_parser = parse_descent)]
        descent: Option<Descent>,
    },
    /// Landing dispersion: wind speed x direction grid (dispersion.csv) or Monte Carlo (dispersion_mc.csv, dispersion_summary.json). Exits with status 3 when any case or sample failed or did not land.
    Dispersion { config: PathBuf },
    /// Check that the CFD tools (SU2, MPI, gmsh) are installed and report their versions.
    CfdCheck {
        config: PathBuf,
        /// Also export the wall surface and build the volume mesh (reports node/cell counts and quality).
        #[arg(long)]
        mesh: bool,
    },
    /// Write the built-in sample rocket as a binary STL in millimetres, nose towards +z.
    SampleStl { path: PathBuf },
}

fn parse_descent(s: &str) -> Result<Descent, String> {
    match s {
        "ballistic" => Ok(Descent::Ballistic),
        "parachute" => Ok(Descent::Parachute),
        _ => Err("expected 'ballistic' or 'parachute'".into()),
    }
}

/// Loads the config and applies its `[resources]` (thread pools, priority, memory budget).
fn load_config(path: &Path) -> Result<Config> {
    let cfg = Config::load(path)?;
    apply_resources(&cfg.resources.resolve());
    Ok(cfg)
}

fn apply_resources(r: &config::Resolved) {
    if let Some(w) = &r.warning {
        match ui().mode {
            ui::Mode::Quiet => eprintln!("warning: {w}"),
            _ => ui().warn(w),
        }
    }
    // Must run before any rayon use; an already-built pool (tests) is left alone.
    let _ = rayon::ThreadPoolBuilder::new().num_threads(r.threads).build_global();
    panel::set_parallelism(r.threads);
    panel::set_memory_limit(r.memory_bytes);
    if r.nice > 0 {
        set_nice(r.nice);
    }
}

#[cfg(unix)]
fn set_nice(nice: i32) {
    // SAFETY: plain syscall; children (SU2, mpirun, gmsh) inherit the priority.
    let rc = unsafe { libc::setpriority(libc::PRIO_PROCESS, 0, nice) };
    if rc != 0 {
        ui().warn(format!("resources.nice = {nice} could not be applied: {}", std::io::Error::last_os_error()));
    }
}

#[cfg(not(unix))]
fn set_nice(_nice: i32) {
    ui().warn("resources.nice is ignored on this platform");
}

fn cfd_budget(cfg: &Config) -> cfd::ResourceBudget {
    let r = cfg.resources.resolve();
    cfd::ResourceBudget { threads: r.threads, memory_bytes: r.memory_bytes }
}

/// Exit status of a run that finished and wrote its outputs but whose results are incomplete.
const EXIT_INCOMPLETE: i32 = 3;

fn main() -> Result<()> {
    let cli = Cli::parse();
    ui::init(cli.quiet, cli.no_progress);
    let mut incomplete: Option<String> = None;
    match cli.cmd {
        Cmd::Geom { config } => {
            let cfg = load_config(&config)?;
            begin(&cfg, &config, Sub::Geom);
            let step = ui().step("Extracting geometry");
            let g = geometry(&cfg)?;
            write_geometry(&cfg, &g)?;
            step.done(&view::geometry_detail(&g));
            show_geometry(&g, true);
            let mut w = g.warnings.clone();
            w.extend(size_warnings(&cfg, g.length, 2.0 * g.ref_radius));
            print_warnings(&w);
        }
        Cmd::Aero { config, force } => {
            let cfg = load_config(&config)?;
            begin(&cfg, &config, Sub::Aero);
            aero_table_for(&cfg, force, Sub::Aero)?;
        }
        Cmd::Sim { config, descent } => {
            let cfg = load_config(&config)?;
            begin(&cfg, &config, Sub::Sim);
            incomplete = run_sim(&cfg, descent)?
        }
        Cmd::Dispersion { config } => {
            let cfg = load_config(&config)?;
            begin(&cfg, &config, Sub::Dispersion);
            incomplete = run_dispersion(&cfg)?
        }
        Cmd::CfdCheck { config, mesh } => {
            let cfg = load_config(&config)?;
            return cfd_check(&cfg, &config, mesh);
        }
        Cmd::SampleStl { path } => {
            let tris: Vec<_> = geom::sample::SampleRocket::default()
                .mesh()
                .iter()
                .map(|t| t.map(|v| geom::Vec3::new(v.y, v.z, -v.x) * 1000.0))
                .collect();
            geom::stl::write_stl_binary(&path, &tris)?;
            if ui().mode != ui::Mode::Quiet {
                println!("wrote {} ({} triangles)", path.display(), tris.len());
            } else {
                println!("{}", path.display());
            }
            return Ok(());
        }
    }
    ui().finish();
    if let Some(why) = incomplete {
        eprintln!("ignisyeet: INCOMPLETE RESULTS: {why} (exit status {EXIT_INCOMPLETE})");
        std::process::exit(EXIT_INCOMPLETE);
    }
    Ok(())
}

/// Banner and configuration summary (rich mode only).
fn begin(cfg: &Config, config_path: &Path, sub: Sub) {
    if ui().plain() {
        let r = cfg.resources.resolve();
        let mem = r.memory_bytes.map(|b| format!("{:.1} GB", b as f64 / 1e9)).unwrap_or_else(|| "no limit".into());
        println!("Resources: threads {} of {} available, memory {mem}, nice {}", r.threads, r.available, r.nice);
    }
    if !ui().rich() {
        return;
    }
    ui().banner();
    let motor = if matches!(sub, Sub::Sim | Sub::Dispersion) { load_motor(cfg).ok() } else { None };
    ui().print_panel(&view::config_panel(cfg, config_path, sub, motor.as_ref()));
    println!();
}

/// Prints sanity warnings in every output mode (stdout in plain and rich mode, stderr in quiet mode).
fn print_warnings(warnings: &[String]) {
    for w in warnings {
        match ui().mode {
            ui::Mode::Quiet => eprintln!("warning: {w}"),
            _ => ui().warn(w),
        }
    }
}

/// Plausibility findings of the last geometry extraction of this process.
static GEOMETRY_WARNINGS: std::sync::Mutex<Option<Vec<String>>> = std::sync::Mutex::new(None);

/// Geometry findings for the current run: those of an extraction made in this process, otherwise those
/// stored in the geometry.json next to the cached aero table (none for an external table).
fn geometry_warnings(cfg: &Config) -> Vec<String> {
    if let Some(w) = GEOMETRY_WARNINGS.lock().unwrap().clone() {
        return w;
    }
    stored_geometry_warnings(cfg)
}

fn stored_geometry_warnings(cfg: &Config) -> Vec<String> {
    if cfg.aero.method == config::AeroMethod::Table {
        return Vec::new();
    }
    let text = std::fs::read_to_string(cfg.out_dir().join("geometry.json")).ok();
    let json = text.and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok());
    json.and_then(|j| j.get("warnings").cloned()).and_then(|w| serde_json::from_value(w).ok()).unwrap_or_default()
}

/// Warning about aero table lookups outside the tabulated range, if there were any.
fn aero_range_warning(s: &aero::table::OutOfRangeStats) -> Option<String> {
    if !s.any() {
        return None;
    }
    let mach = s.mach_below + s.mach_above;
    let mut parts = Vec::new();
    if mach > 0 {
        parts.push(format!("{mach} of {} lookups beyond the Mach range", s.lookups));
    }
    if s.angle_beyond > 0 {
        parts.push(format!("{} of {} lookups beyond the angle range", s.angle_beyond, s.lookups));
    }
    Some(format!(
        "aero table: {} (max Mach {:.2}, max |alpha| {:.1} deg); Mach is held at the table edge, angles are extrapolated",
        parts.join(", "),
        s.max_mach,
        s.max_abs_alpha_deg
    ))
}

/// The `aero_table_range` object of summary.json.
fn aero_range_json(s: &aero::table::OutOfRangeStats) -> serde_json::Value {
    serde_json::json!({
        "lookups": s.lookups,
        "mach_below": s.mach_below,
        "mach_above": s.mach_above,
        "angle_beyond": s.angle_beyond,
        "max_mach": s.max_mach,
        "max_abs_alpha_deg": s.max_abs_alpha_deg,
        "out_of_range": s.any(),
    })
}

/// Plausible size range of a rocket body; outside it `rocket.stl_scale` is most likely wrong.
const LENGTH_RANGE: (f64, f64) = (0.1, 20.0);
const DIAMETER_RANGE: (f64, f64) = (0.005, 3.0);

/// Warnings about the size of the body (`length`, `diameter` in metres).
fn size_warnings(cfg: &Config, length: f64, diameter: f64) -> Vec<String> {
    let mut w = Vec::new();
    if !(LENGTH_RANGE.0..=LENGTH_RANGE.1).contains(&length) {
        w.push(format!(
            "body length {length:.4} m is outside the plausible {}-{} m range: check rocket.stl_scale (now {}; 0.001 converts millimetres to metres)",
            LENGTH_RANGE.0, LENGTH_RANGE.1, cfg.rocket.stl_scale
        ));
    }
    if !(DIAMETER_RANGE.0..=DIAMETER_RANGE.1).contains(&diameter) {
        w.push(format!(
            "body diameter {diameter:.4} m is outside the plausible {}-{} m range: check rocket.stl_scale (now {})",
            DIAMETER_RANGE.0, DIAMETER_RANGE.1, cfg.rocket.stl_scale
        ));
    }
    w
}

/// Height above ground up to which the power-law and log wind profiles are meaningful [m].
const WIND_VALID_HEIGHT: f64 = 1000.0;

/// Warnings about the flight itself: static margin, centre of pressure, wind-model validity, no landing.
fn flight_warnings(cfg: &Config, s: &Summary) -> Vec<String> {
    flight_warnings_at(cfg, s, cfg.wind.speed)
}

/// [`flight_warnings`] for a run whose largest reference wind speed is `wind_speed` [m/s]: the extrapolation
/// of the wind profile only matters when there is wind.
fn flight_warnings_at(cfg: &Config, s: &Summary, wind_speed: f64) -> Vec<String> {
    let mut w = Vec::new();
    if !s.landed() {
        w.push(format!(
            "THE FLIGHT DID NOT LAND within sim.max_time = {} s: there is no landing point; raise sim.max_time or check the descent (parachute, drag area, launch angle)",
            cfg.sim.max_time
        ));
    }
    let margins = [s.min_stability_cal, s.stability_at_rail_exit].into_iter().filter(|m| m.is_finite());
    if let Some(m) = margins.min_by(f64::total_cmp) {
        if m < 0.0 {
            w.push(format!("static margin {m:.2} cal is negative: the centre of pressure is forward of the CG, the rocket is UNSTABLE"));
        } else if m < 1.0 {
            w.push(format!("minimum static margin {m:.2} cal is below 1.0 cal: low static margin"));
        } else if m < 1.5 {
            w.push(format!("minimum static margin {m:.2} cal is below the recommended 1.5 cal (marginal)"));
        }
    }
    let wind = &cfg.wind;
    if matches!(wind.model, sim::WindModel::Power | sim::WindModel::Log) && wind_speed != 0.0 && s.apogee > WIND_VALID_HEIGHT {
        w.push(format!(
            "apogee {:.0} m is above {WIND_VALID_HEIGHT:.0} m, the validity height of the {:?} wind profile: the profile is extrapolated there, and wind.speed = {} m/s is the speed at wind.ref_height = {} m, so the winds aloft and the landing point are model-dependent",
            s.apogee, wind.model, wind.speed, wind.ref_height
        ));
    }
    w
}

/// An error when the body length (metres, after `rocket.stl_scale`) is not that of a rocket.
fn check_length(cfg: &Config, length: f64) -> Result<()> {
    if !(LENGTH_RANGE.0..=LENGTH_RANGE.1).contains(&length) {
        anyhow::bail!(
            "body length {length:.4} m is outside the plausible {}-{} m range: check the STL units and rocket.stl_scale (now {}; 0.001 converts millimetres to metres)",
            LENGTH_RANGE.0, LENGTH_RANGE.1, cfg.rocket.stl_scale
        );
    }
    Ok(())
}

/// An error when the dry CG lies outside the body (`length` in metres, CG measured aft of the nose tip).
fn check_cg(cfg: &Config, length: f64) -> Result<()> {
    let cg = cfg.rocket.cg_dry;
    if cg > length {
        anyhow::bail!("rocket.cg_dry = {cg} m is outside the body: it is measured aft of the nose tip and the body is {length:.4} m long (check rocket.stl_scale and the unit of cg_dry)");
    }
    Ok(())
}

fn load_motor(cfg: &Config) -> Result<Motor> {
    let eng = cfg.resolve(&cfg.motor.eng);
    let mut motor = Motor::load(&eng).with_context(|| format!("cannot read motor file {}", eng.display()))?;
    if let Some(m) = cfg.motor.propellant_mass {
        motor.propellant_mass = m;
    }
    Ok(motor)
}

fn show_geometry(g: &Geometry, panel: bool) {
    if ui().plain() {
        print_geometry(g);
    } else if panel && ui().rich() {
        println!();
        ui().print_panel(&view::geometry_panel(g));
    }
}

fn out_path(cfg: &Config, name: &str) -> Result<PathBuf> {
    let dir = cfg.out_dir();
    std::fs::create_dir_all(&dir).with_context(|| format!("cannot create {}", dir.display()))?;
    let p = dir.join(name);
    ui().record(&p);
    Ok(p)
}

fn geometry(cfg: &Config) -> Result<Geometry> {
    let stl = cfg.resolve(&cfg.rocket.stl);
    let s = cfg.rocket.stl_scale;
    let tris: Vec<_> = geom::stl::read_stl(&stl).with_context(|| format!("cannot read STL {}", stl.display()))?.into_iter().map(|t| t.map(|v| v * s)).collect();
    let opt = ExtractOptions { n_slices: cfg.aero.n_slices, nose_direction: cfg.nose_direction()?, fin_threshold: cfg.aero.fin_threshold };
    let mut g = geom::extract(&tris, &opt).with_context(|| format!("geometry extraction failed for {}", stl.display()))?;
    *GEOMETRY_WARNINGS.lock().unwrap() = Some(g.warnings.clone());
    if let Some(o) = &cfg.rocket.fin_override {
        let r_body = g.ref_radius;
        let f = g.fins.get_or_insert(geom::FinSet {
            count: 0,
            root_chord: 0.0,
            tip_chord: 0.0,
            span: 0.0,
            sweep: 0.0,
            thickness: 0.0,
            x_le_root: 0.0,
            body_radius: r_body,
            cant_deg: 0.0,
        });
        f.count = o.count.unwrap_or(f.count);
        f.root_chord = o.root_chord.unwrap_or(f.root_chord);
        f.tip_chord = o.tip_chord.unwrap_or(f.tip_chord);
        f.span = o.span.unwrap_or(f.span);
        f.sweep = o.sweep.unwrap_or(f.sweep);
        f.thickness = o.thickness.unwrap_or(f.thickness);
        f.x_le_root = o.x_le_root.unwrap_or(f.x_le_root);
        if f.count == 0 || f.span <= 0.0 {
            g.fins = None;
        }
    }
    // The configured cant is part of the geometry the aero tables are built for.
    if let Some(f) = g.fins.as_mut() {
        f.cant_deg = cfg.rocket.cant_deg();
    }
    check_length(cfg, g.length)?;
    check_cg(cfg, g.length)?;
    Ok(g)
}

fn write_geometry(cfg: &Config, g: &Geometry) -> Result<()> {
    // Body axes in STL coordinates, so inertia products, CG offsets and the thrust offset can be given in body axes.
    let mut json = serde_json::to_value(g)?;
    let [bx, by, bz] = g.frame.body_basis();
    if let Some(o) = json.as_object_mut() {
        for (k, v) in [("body_x", bx), ("body_y", by), ("body_z", bz)] {
            o.insert(k.into(), serde_json::json!([v.x, v.y, v.z]));
        }
    }
    std::fs::write(out_path(cfg, "geometry.json")?, serde_json::to_string_pretty(&json)?)?;
    let mut f = std::io::BufWriter::new(std::fs::File::create(out_path(cfg, "profile.csv")?)?);
    writeln!(f, "x,r,fin_span")?;
    for p in &g.profile {
        writeln!(f, "{:.6},{:.6},{:.6}", p.x, p.r, p.fin_span)?;
    }
    Ok(())
}

fn print_geometry(g: &Geometry) {
    println!("Geometry");
    println!("  length          {:.4} m", g.length);
    println!("  diameter        {:.4} m (ref area {:.6} m^2)", 2.0 * g.ref_radius, g.ref_area);
    println!("  nose            length {:.4} m, joint angle {:.2} deg, volume ratio {:.3}", g.nose.length, g.nose.joint_half_angle_deg, g.nose.volume_ratio);
    if let Some(b) = &g.boattail {
        println!("  boattail        length {:.4} m, r {:.4} -> {:.4} m", b.length, b.r_fore, b.r_aft);
    }
    match &g.fins {
        Some(f) => println!(
            "  fins            {} x root {:.4} tip {:.4} span {:.4} sweep {:.4} t {:.4} at x {:.4} m",
            f.count, f.root_chord, f.tip_chord, f.span, f.sweep, f.thickness, f.x_le_root
        ),
        None => println!("  fins            none detected"),
    }
}

fn fnv1a(h: &mut u64, bytes: &[u8]) {
    for b in bytes {
        *h ^= *b as u64;
        *h = h.wrapping_mul(0x100_0000_01b3);
    }
}

fn aero_options(cfg: &Config) -> AeroOptions {
    let a = &cfg.aero;
    AeroOptions {
        mach_min: a.mach_min,
        mach_max: a.mach_max,
        mach_step: if cfg.aero.table_kind == aero::table::TableKind::General && a.method == config::AeroMethod::Barrowman { a.general_mach_step } else { a.mach_step },
        alpha_max_deg: a.alpha_max_deg,
        alpha_step_deg: a.alpha_step_deg,
        reference_altitude: cfg.launch.altitude,
        atmosphere: cfg.atmosphere.build().unwrap_or_default(),
        roughness: cfg.rocket.roughness,
        fin_le_rounded: cfg.rocket.fin_le == "rounded",
        fin_te_square: cfg.rocket.fin_te == "square",
        extra_cd: cfg.rocket.extra_cd,
        nozzle_exit_area: std::f64::consts::PI * (cfg.motor.nozzle_exit_diameter / 2.0).powi(2),
    }
}

/// Version of the aerodynamic model and of the table layout. Bump it whenever a change of the code
/// alters the coefficients computed from the same inputs, so that older cached tables are rebuilt.
const MODEL_VERSION: u32 = 2;

/// The part of the aero settings that determines the table: unused method blocks and the
/// resource/path-only CFD fields do not change the coefficients and are left out.
fn aero_inputs(cfg: &Config) -> Result<serde_json::Value> {
    let mut v = serde_json::to_value(&cfg.aero)?;
    if let Some(o) = v.as_object_mut() {
        o.remove("table");
        if cfg.aero.method != config::AeroMethod::Panel {
            o.remove("panel");
        }
        if cfg.aero.method != config::AeroMethod::Cfd {
            o.remove("cfd");
        } else if let Some(c) = o.get_mut("cfd").and_then(|c| c.as_object_mut()) {
            for k in ["prefix", "su2", "mpi", "ranks_per_case", "parallel_cases", "timeout_minutes", "workdir"] {
                c.remove(k);
            }
        }
    }
    Ok(v)
}

fn source_hash(cfg: &Config) -> Result<String> {
    let mut h = 0xcbf2_9ce4_8422_2325u64;
    let stl = cfg.resolve(&cfg.rocket.stl);
    fnv1a(&mut h, &std::fs::read(&stl).with_context(|| format!("cannot read STL {}", stl.display()))?);
    // The extracted geometry (after scale, nose direction, fin override and cant) enters as well, so a change
    // of the extraction code invalidates cached tables without a MODEL_VERSION bump.
    fnv1a(&mut h, &serde_json::to_vec(&geometry(cfg)?)?);
    #[derive(Serialize)]
    struct Inputs<'a> {
        scale: f64,
        nose: &'a str,
        fins: &'a Option<config::FinOverride>,
        cant_deg: f64,
        aero: serde_json::Value,
        atmosphere: &'a sim::AtmosphereConfig,
        opt: AeroOptions,
        version: &'a str,
        model_version: u32,
    }
    let inputs = Inputs {
        scale: cfg.rocket.stl_scale,
        nose: &cfg.rocket.nose_direction,
        fins: &cfg.rocket.fin_override,
        cant_deg: cfg.rocket.cant_deg(),
        aero: aero_inputs(cfg)?,
        atmosphere: &cfg.atmosphere,
        opt: aero_options(cfg),
        version: env!("CARGO_PKG_VERSION"),
        model_version: MODEL_VERSION,
    };
    fnv1a(&mut h, serde_json::to_string(&inputs)?.as_bytes());
    Ok(format!("{h:016x}"))
}

/// Which result file a run hash belongs to; each depends on the part of the configuration that produced it.
#[derive(Clone, Copy, PartialEq, Eq)]
enum RunKind {
    Flight,
    WindGrid,
    MonteCarlo,
}

/// Identifies the configuration (and the input files) a result was computed with: written into
/// summary.json, dispersion_meta.json and dispersion_summary.json, and compared when the KML is
/// assembled from files of earlier runs. Output location and resources do not enter.
fn run_hash(cfg: &Config, kind: RunKind) -> String {
    let mut v = serde_json::to_value(cfg).unwrap_or_default();
    if let Some(o) = v.as_object_mut() {
        o.remove("output");
        o.remove("resources");
        match kind {
            RunKind::Flight => {
                o.remove("dispersion");
            }
            RunKind::WindGrid | RunKind::MonteCarlo => {
                if let Some(d) = o.get_mut("dispersion").and_then(|d| d.as_object_mut()) {
                    d.remove("mode");
                    let drop: &[&str] = if kind == RunKind::WindGrid { &["monte_carlo"] } else { &["wind_speeds", "directions"] };
                    for k in drop {
                        d.remove(*k);
                    }
                }
            }
        }
    }
    let mut h = 0xcbf2_9ce4_8422_2325u64;
    fnv1a(&mut h, v.to_string().as_bytes());
    fnv1a(&mut h, env!("CARGO_PKG_VERSION").as_bytes());
    fnv1a(&mut h, &MODEL_VERSION.to_le_bytes());
    let mut files = vec![cfg.resolve(&cfg.rocket.stl), cfg.resolve(&cfg.motor.eng)];
    files.extend(cfg.wind.profile.as_deref().map(|p| cfg.resolve(p)));
    files.extend(cfg.aero.table.as_deref().map(|p| cfg.resolve(p)));
    for f in files {
        // An unreadable file hashes as empty: the run itself reports the real error.
        fnv1a(&mut h, &std::fs::read(&f).unwrap_or_default());
    }
    format!("{h:016x}")
}

/// Load the cached table when its inputs are unchanged, otherwise rebuild and save it.
fn aero_table(cfg: &Config, force: bool) -> Result<AeroTable> {
    aero_table_for(cfg, force, Sub::Sim)
}

const BAR_T: &str = "  {spinner:.cyan} {prefix:<12.bold} {bar} {pos}/{len} {percent:>3}%  {elapsed:.dim}  ETA {eta_precise:.yellow}\n      {msg:.dim}";

/// Axis grids of the general tables expanded from axisymmetric ones.
fn general_axes() -> (Vec<f64>, Vec<f64>) {
    (aero::table::angle_axis(180.0, 30.0, 2.0, 10.0), aero::table::angle_axis(90.0, 30.0, 2.0, 10.0))
}

fn aero_table_for(cfg: &Config, force: bool, sub: Sub) -> Result<AeroTable> {
    let mut t = aero_table_inner(cfg, force, sub)?;
    if cfg.aero.method == config::AeroMethod::Table && !t.has_roll_terms() {
        // External table without roll data: use the Barrowman roll terms of the current geometry.
        match geometry(cfg) {
            Ok(g) => {
                let model = AeroModel::new(g, aero_options(cfg));
                if model.roll_terms(0.5) != (0.0, 0.0) {
                    t.set_roll_terms(|m| model.roll_terms(m));
                    eprintln!("note: the external aero table has no roll terms; cant forcing and roll damping were filled from Barrowman for the current fins");
                }
            }
            Err(e) => eprintln!("note: the external aero table has no roll terms and the geometry is unavailable ({e:#}); the rocket will not roll"),
        }
    }
    if cfg.aero.table_kind == aero::table::TableKind::General && t.kind() == aero::table::TableKind::Axisymmetric {
        let start = std::time::Instant::now();
        let u = ui();
        let (alphas, betas) = general_axes();
        t = t.to_general(alphas, betas);
        let (nm, na, nb) = (t.meta.machs.len(), t.meta.alphas_deg.len(), t.meta.betas_deg.len());
        if cfg.aero.method != config::AeroMethod::Table {
            let path = out_path(cfg, "aero_table.csv")?;
            t.save(&path)?;
            if u.plain() {
                println!("General table: {nm} Mach x {na} alpha x {nb} beta = {} rows in {:.2?} -> {}", t.rows(), start.elapsed(), path.display());
            }
        }
        u.done_line("General table", &format!("{nm} Mach x {na} alpha x {nb} beta ({} rows)", t.rows()), start.elapsed());
    }
    // Lookups made while building the table (panel, CFD, the drag file) are not flight lookups.
    t.reset_out_of_range_stats();
    Ok(t)
}

fn aero_table_inner(cfg: &Config, force: bool, sub: Sub) -> Result<AeroTable> {
    let u = ui();
    let t_start = std::time::Instant::now();
    match cfg.aero.method {
        config::AeroMethod::Barrowman | config::AeroMethod::Panel | config::AeroMethod::Cfd => {}
        config::AeroMethod::Table => {
            let path = cfg.resolve(cfg.aero.table.as_deref().context("aero.table is required when aero.method = \"table\"")?);
            let t = AeroTable::load(&path).with_context(|| format!("cannot load aero table {}", path.display()))?;
            if u.plain() {
                println!("Using external aero table {} ({} rows)", path.display(), t.rows());
            }
            u.done_line("Aero table", &format!("external {} ({} rows)", ui::short_path(&path), t.rows()), t_start.elapsed());
            show_table(&t, None, sub);
            return Ok(t);
        }
    }
    let path = out_path(cfg, "aero_table.csv")?;
    let hash = source_hash(cfg)?;
    if !force && path.exists() {
        if let Ok(t) = AeroTable::load(&path) {
            if t.meta.source_hash == hash {
                if u.plain() {
                    println!("Using cached aero table {} ({} rows)", path.display(), t.rows());
                }
                u.done_line("Aero table", &format!("cached {} ({} rows)", ui::short_path(&path), t.rows()), t_start.elapsed());
                show_table(&t, None, sub);
                return Ok(t);
            }
        }
    }
    if cfg.aero.method == config::AeroMethod::Cfd {
        print_warnings(&cfg.aero.cfd.warnings());
        // Fail early (before the geometry step) when the tools are missing.
        let tools = cfd::check_tools(&cfg.aero.cfd)?;
        print_warnings(&tools.warnings());
    }
    let start = std::time::Instant::now();
    let step = u.step("Extracting geometry");
    let g = geometry(cfg)?;
    write_geometry(cfg, &g)?;
    step.done(&view::geometry_detail(&g));
    show_geometry(&g, sub == Sub::Aero);
    if sub == Sub::Aero {
        print_warnings(&g.warnings);
    }
    if cfg.aero.method == config::AeroMethod::Panel {
        return panel_table(cfg, g, hash, &path, start, sub);
    }
    if cfg.aero.method == config::AeroMethod::Cfd {
        return cfd_table(cfg, g, hash, &path, start, sub);
    }
    let model = AeroModel::new(g, aero_options(cfg));
    let n_rows = aero_grid_len(cfg);
    let pb = u.bar(n_rows as u64, "Barrowman", BAR_T);
    pb.set_message("Mach rows of the analytic table");
    let table = AeroTable::build_with_progress(&model, hash, cfg.aero.extrapolation, &|done, _| pb.set_position(done as u64));
    pb.finish_and_clear();
    table.save(&path)?;

    let mut f = std::io::BufWriter::new(std::fs::File::create(out_path(cfg, "aero_drag.csv")?)?);
    writeln!(f, "mach,reynolds,friction,nose,base,fins,boattail,extra,cd_off,cd_on,cna,xcp")?;
    for &m in &table.meta.machs {
        let off = model.cd0(m, false);
        let on = model.cd0(m, true);
        let c = table.lookup(m, 0.0);
        writeln!(
            f,
            "{m},{:.6e},{:.6e},{:.6e},{:.6e},{:.6e},{:.6e},{:.6e},{:.6e},{:.6e},{:.6e},{:.6e}",
            off.reynolds, off.friction, off.nose, off.base, off.fins, off.boattail, off.extra, off.total(), on.total(), c.cna, c.xcp
        )?;
    }
    let c = table.lookup(0.3, 0.0);
    if u.plain() {
        println!("Aero table: {} rows in {:.2?} -> {}", table.rows(), start.elapsed(), path.display());
        println!("  M=0.3: CNa {:.3} /rad, Xcp {:.4} m, CD0 {:.3}", c.cna, c.xcp, c.ca_off);
    }
    u.done_line("Aero table", &format!("Barrowman, {} rows", table.rows()), start.elapsed());
    show_table(&table, Some(start.elapsed()), sub);
    Ok(table)
}

fn aero_grid_len(cfg: &Config) -> usize {
    let a = &cfg.aero;
    ((a.mach_max - a.mach_min.max(0.0)) / aero_options(cfg).mach_step).round().max(1.0) as usize + 1
}

fn show_table(t: &AeroTable, elapsed: Option<std::time::Duration>, sub: Sub) {
    if sub == Sub::Aero && ui().rich() {
        println!();
        ui().print_panel(&view::table_panel(t, elapsed));
    }
}

/// Raw STL in metres (only needed for `surface = "stl"`).
fn raw_stl(cfg: &Config) -> Result<Vec<geom::stl::Triangle>> {
    let s = cfg.rocket.stl_scale;
    let stl = cfg.resolve(&cfg.rocket.stl);
    Ok(geom::stl::read_stl(&stl).with_context(|| format!("cannot read STL {}", stl.display()))?.into_iter().map(|t| t.map(|v| v * s)).collect())
}

/// Build the coefficient table with SU2.
fn cfd_table(cfg: &Config, g: Geometry, hash: String, path: &Path, start: std::time::Instant, sub: Sub) -> Result<AeroTable> {
    use cfd::CfdStage as St;
    use std::sync::atomic::{AtomicUsize, Ordering};
    let u = ui();
    let pb = u.bar(1, "CFD", BAR_T);
    let finished = AtomicUsize::new(0);
    let progress = |st: St| match st {
        St::Mesh => pb.set_message("surface and volume mesh"),
        St::Case { index, total, mach, alpha, iter, residual } => {
            pb.set_length(total as u64);
            let msg = if iter == 0 {
                format!("case {}/{total}: M={mach:.2} alpha={alpha:.1} deg, starting", index + 1)
            } else {
                format!("case {}/{total}: M={mach:.2} alpha={alpha:.1} deg, iter {iter}, log10 res {residual:.2}", index + 1)
            };
            pb.set_message(msg);
        }
        St::CaseDone { total, .. } => {
            pb.set_length(total as u64);
            pb.set_position(finished.fetch_add(1, Ordering::SeqCst) as u64 + 1);
        }
        St::Table => pb.set_message("filling the aero table"),
        St::Done => {}
    };
    let out_dir = cfg.out_dir();
    let raw = if cfg.aero.cfd.surface == cfd::SurfaceKind::Stl { Some(raw_stl(cfg)?) } else { None };
    let built = cfd::build_table_with_progress(&g, &cfg.aero.cfd, &aero_options(cfg), &out_dir, hash, cfg.aero.extrapolation, &cfd_budget(cfg), raw.as_deref(), &progress);
    pb.finish_and_clear();
    let (table, report) = built?;
    table.save(path)?;
    std::fs::write(out_path(cfg, "cfd_report.json")?, serde_json::to_string_pretty(&report)?)?;
    ui().record(&out_dir.join("cfd_cases.csv"));
    if u.plain() {
        println!("Aero table: CFD, {} cases ({} failed, {} unconverged), {} cells, {:.1?} -> {}", report.cases, report.failed.len(), report.unconverged.len(), report.mesh_cells, start.elapsed(), path.display());
    }
    u.done_line("CFD", &format!("{} cases, {} failed, {} cells", report.cases, report.failed.len(), report.mesh_cells), start.elapsed());
    show_table(&table, Some(start.elapsed()), sub);
    Ok(table)
}

/// `cfd-check`: report the CFD tools and their versions; with `mesh`, also build the volume mesh.
fn cfd_check(cfg: &Config, config_path: &Path, mesh: bool) -> Result<()> {
    let opt = &cfg.aero.cfd;
    let roots = [cfg.base_dir.clone()];
    let r = cfd::tools::check_tools(opt, &roots);
    println!("CFD tools for {}", config_path.display());
    match &r.prefix {
        Some(p) => println!("  prefix      {}", p.display()),
        None => println!("  prefix      (none; using PATH)"),
    }
    for t in [&r.su2, &r.mpi, &r.gmsh, &r.manifold] {
        match (&t.problem, &t.version) {
            (None, v) => println!("  {:<21}ok       {}  [{}]", t.name, v.as_deref().unwrap_or(""), t.path.as_ref().map(|p| p.display().to_string()).unwrap_or_default()),
            (Some(p), _) if t.name.starts_with("manifold3d") && opt.surface != cfd::SurfaceKind::Stl => println!("  {:<21}not installed (only needed for surface = \"stl\")", t.name),
            (Some(p), _) => println!("  {:<21}MISSING  {p}", t.name),
        }
    }
    for w in opt.warnings().into_iter().chain(r.warnings()) {
        println!("  warning: {w}");
    }
    if r.ready(opt) {
        let n = cfd::case::case_list(opt).len();
        if opt.surface == cfd::SurfaceKind::Stl && matches!(opt.shape_symmetry, None | Some(cfd::config::ShapeSymmetry::Auto)) {
            // The mirror symmetry of an STL is only known after the union (`--mesh`); without it the count is the
            // full-domain upper bound.
            println!("CFD mode is ready (up to {n} cases for model {:?}; mirror symmetry of the STL reduces this, see --mesh).", opt.model);
        } else {
            println!("CFD mode is ready ({n} cases for model {:?}).", opt.model);
        }
    } else {
        println!("CFD mode is not available: {}", cfd::not_installed_message(&r, opt).lines().next().unwrap_or(""));
        anyhow::bail!("CFD tools are missing");
    }
    if mesh {
        let g = geometry(cfg)?;
        let raw = if opt.surface == cfd::SurfaceKind::Stl { Some(raw_stl(cfg)?) } else { None };
        let t = std::time::Instant::now();
        let m = cfd::mesh_only(&g, opt, &aero_options(cfg), &cfg.out_dir(), &cfd_budget(cfg), raw.as_deref())?;
        let s = &m.stats;
        println!(
            "Mesh {} ({}): {} nodes, {} cells, {} wall triangles, {:.1?}",
            m.su2.display(),
            if m.cached { "cached" } else { "built" },
            s.nodes,
            s.cells,
            s.wall_triangles,
            t.elapsed()
        );
        println!("  cell types: {:?}", s.by_type);
        println!("  quality (SICN, 1 = ideal): min {:.3}, mean {:.3}, {} cells below 0.05", s.quality_sicn_min, s.quality_sicn_mean, s.cells_bad);
        if let Some(info) = &m.stl {
            println!("STL geometry fidelity (base treatment: {}):", info.base_treatment);
            println!("{}", info.fidelity.summary());
        }
    }
    Ok(())
}

/// Build the coefficient table with the panel method and write its diagnostics.
fn panel_table(cfg: &Config, g: Geometry, hash: String, path: &Path, start: std::time::Instant, sub: Sub) -> Result<AeroTable> {
    use panel::PanelStage as St;
    let u = ui();
    let pb = u.bar(1, "Panel method", BAR_T);
    let stage_t = std::sync::Mutex::new(std::time::Instant::now());
    let progress = |st: St| match st {
        St::Mesh { subsonic_solves } => {
            pb.set_length(subsonic_solves as u64 + 3);
            pb.set_message("meshing body, fins and wake");
        }
        St::MeshReady { panels, wake_panels } => {
            pb.inc(1);
            pb.set_message(format!("mesh ready: {panels} panels + {wake_panels} wake panels"));
        }
        St::Subsonic { index, total, mach } => {
            *stage_t.lock().unwrap() = std::time::Instant::now();
            pb.set_message(format!("subsonic M={mach:.2} ({}/{total}): assembling, LU factorisation, solving", index + 1));
        }
        St::SubsonicDone { index, total, mach } => {
            pb.inc(1);
            pb.set_message(format!("subsonic M={mach:.2} ({}/{total}) solved in {}", index + 1, ui::fmt_dur(stage_t.lock().unwrap().elapsed())));
        }
        St::Supersonic => pb.set_message("supersonic local-inclination table"),
        St::SupersonicDone => pb.inc(1),
        St::Transonic => pb.set_message("transonic blend"),
        St::Columns { done, total } => pb.set_message(format!("filling Mach x alpha table {done}/{total}")),
        St::Done => pb.inc(1),
    };
    let built = panel::build_table_with_progress(&g, &cfg.aero.panel, &aero_options(cfg), hash, cfg.aero.extrapolation, &progress);
    pb.finish_and_clear();
    let (table, report) = built?;
    table.save(path)?;
    panel::write_surface_cp(&out_path(cfg, "panel_cp.csv")?, &report)?;
    let mut f = std::io::BufWriter::new(std::fs::File::create(out_path(cfg, "aero_drag.csv")?)?);
    writeln!(f, "mach,cd_off,cd_on,cna,xcp")?;
    for &m in &table.meta.machs {
        let c = table.lookup(m, 0.0);
        writeln!(f, "{m},{:.6e},{:.6e},{:.6e},{:.6e}", c.ca_off, c.ca_on, c.cna, c.xcp)?;
    }
    let c = table.lookup(0.3, 0.0);
    if u.plain() {
        println!("Panel method: {} panels + {} wake panels, subsonic solves {:.2} s", report.panels, report.wake_panels, report.solve_seconds);
        for s in &report.subsonic {
            println!("  M={:.2}: CNa {:.3} /rad, Xcp {:.4} m", s.mach, s.cna, s.xcp);
        }
        println!("Aero table: {} rows in {:.2?} -> {}", table.rows(), start.elapsed(), path.display());
        println!("  M=0.3: CNa {:.3} /rad, Xcp {:.4} m, CD0 {:.3}", c.cna, c.xcp, c.ca_off);
    }
    u.done_line("Panel method", &format!("{} panels, {} subsonic solves", report.panels, report.subsonic.len()), start.elapsed());
    if sub == Sub::Aero && u.rich() {
        println!();
        u.print_panel(&view::panel_report_panel(&report));
    }
    show_table(&table, Some(start.elapsed()), sub);
    Ok(table)
}

struct Prepared {
    table: AeroTable,
    motor: Motor,
    motor_aft_x: f64,
    geometry_warnings: Vec<String>,
}

fn prepare(cfg: &Config) -> Result<Prepared> {
    let table = aero_table(cfg, false)?;
    check_cg(cfg, table.meta.length)?;
    let motor = load_motor(cfg)?;
    let motor_aft_x = cfg.motor.aft_x.unwrap_or(table.meta.length);
    Ok(Prepared { table, motor, motor_aft_x, geometry_warnings: geometry_warnings(cfg) })
}

fn simulation<'a>(cfg: &Config, p: &'a Prepared) -> Result<Simulation<'a>> {
    let r = &cfg.rocket;
    Ok(Simulation {
        table: &p.table,
        mass: r.mass_properties(),
        motor: &p.motor,
        motor_aft_x: p.motor_aft_x,
        launch: cfg.launch.clone(),
        recovery: cfg.recovery.clone(),
        wind: sim::Wind::from_config(&cfg.wind_config()).with_context(|| match &cfg.wind.profile {
            Some(p) => format!("cannot load wind profile {}", cfg.resolve(p).display()),
            None => "invalid wind configuration".to_string(),
        })?,
        atmosphere: cfg.atmosphere.build()?,
        earth: sim::Earth::from_config(&cfg.earth)?,
        settings: sim::Settings {
            dt: cfg.sim.dt,
            max_time: cfg.sim.max_time,
            output_interval: cfg.sim.output_interval,
            integrator: cfg.sim.integrator,
            attitude: cfg.sim.attitude,
            rtol: cfg.sim.rtol,
            atol: cfg.sim.atol,
        },
        aero_scale: sim::AeroScale::default(),
        thrust_line: cfg.motor.thrust_line(),
        fin_cant_deg: r.cant_deg(),
    })
}

fn default_descent(cfg: &Config) -> Descent {
    cfg.sim.descent.unwrap_or(if cfg.recovery.enabled { Descent::Parachute } else { Descent::Ballistic })
}

const SIM_T: &str = "  {spinner:.cyan} {prefix:<12.bold} {bar} {percent:>3}%  T+{simt:.cyan}  {elapsed:.dim}  ETA {eta_precise:.yellow}\n      {msg}";

/// Flies the configured rocket once, with a live progress bar when a terminal is attached.
fn fly(cfg: &Config, sim: &Simulation, descent: Descent) -> Result<sim::SimResult> {
    let u = ui();
    if !u.progress_on() {
        return sim.run(descent, true);
    }
    use console::style;
    use sim::flight::{Phase, Sample};
    const G: f64 = 9.80665;
    let alt0 = cfg.launch.altitude;
    let mass = cfg.rocket.dry_mass + sim.motor.propellant_mass;
    // Ideal burnout speed (reduced for drag) before the real one is known.
    let dv_est = (sim.motor.total_impulse() / (mass - 0.5 * sim.motor.propellant_mass).max(1e-3) - G * sim.motor.burn_time()).max(50.0);
    let v_chute = (2.0 * mass * G / (1.1 * cfg.recovery.cd_s.max(1e-3))).sqrt();
    let max_time = cfg.sim.max_time;
    let descent_time = move |h: f64| match descent {
        Descent::Parachute => h / v_chute + cfg.recovery.delay,
        Descent::Ballistic => 1.5 * (2.0 * h / G).sqrt(),
    };
    let pb = u.bar_with(1000, "Flight", SIM_T, |st| {
        st.with_key("simt", |s: &indicatif::ProgressState, w: &mut dyn std::fmt::Write| {
            let _ = write!(w, "{:5.1}s/{:<5}", s.pos() as f64 / 100.0, format!("~{:.0}s", s.len().unwrap_or(0) as f64 / 100.0));
        })
    });
    let mut past_apogee = false;
    let mut obs = |smp: &Sample| {
        let h = (smp.alt - alt0).max(0.0);
        if smp.t > 0.2 && smp.phase != Phase::Rail && smp.vel_u < 0.0 {
            past_apogee = true;
        }
        let remaining = if past_apogee {
            descent_time(h)
        } else {
            let vz = if smp.thrust > 0.0 || smp.phase == Phase::Rail { smp.vel_u.max(dv_est) * 0.7 } else { smp.vel_u.max(0.0) } * 0.7;
            vz / G + descent_time(h + vz * vz / (2.0 * G))
        };
        let total = (smp.t + remaining).clamp(smp.t + 0.5, max_time.max(smp.t + 0.5));
        pb.set_length((total * 100.0) as u64);
        pb.set_position((smp.t * 100.0) as u64);
        let st = match smp.phase {
            Phase::Rail => style("rail").yellow(),
            Phase::Free if smp.thrust > 0.0 => style("powered").red().bold(),
            Phase::Free if past_apogee => style("descent").magenta(),
            Phase::Free => style("coast").cyan(),
            Phase::Parachute => style("parachute").green().bold(),
        };
        let stab = if smp.phase == Phase::Parachute { String::new() } else { format!("   stab {:.1} cal", smp.stability_cal) };
        pb.set_message(format!("{}  alt {:>7.1} m   v {:>6.1} m/s   Mach {:.2}{stab}", ui::pad(&st.to_string(), 9), h, smp.airspeed, smp.mach));
    };
    let r = sim.run_with_observer(descent, true, &mut obs);
    pb.finish_and_clear();
    r
}

/// Flies once and writes trajectory.csv, summary.json and the KML. Returns why the results are
/// incomplete (the flight did not land), which makes the process exit with [`EXIT_INCOMPLETE`].
fn run_sim(cfg: &Config, descent: Option<Descent>) -> Result<Option<String>> {
    let u = ui();
    let p = prepare(cfg)?;
    let descent = descent.unwrap_or_else(|| default_descent(cfg));
    if u.rich() {
        u.done_line("Rocket model", &format!("L {:.3} m {d} D {:.4} m {d} {} aero rows", p.table.meta.length, p.table.meta.ref_diameter, p.table.rows(), d = u.g().dot), std::time::Duration::ZERO);
    }
    let t0 = std::time::Instant::now();
    let r = fly(cfg, &simulation(cfg, &p)?, descent)?;
    u.done_line("Flight simulated", &format!("{descent:?}, {} samples, {} RHS evals", r.samples.len(), r.evals), t0.elapsed());
    let path = out_path(cfg, "trajectory.csv")?;
    write_trajectory(&path, &r.samples)?;
    let s = &r.summary;
    let landed = s.landed();
    let range = p.table.out_of_range_stats();
    let mut warnings = p.geometry_warnings.clone();
    warnings.extend(size_warnings(cfg, p.table.meta.length, p.table.meta.ref_diameter));
    warnings.extend(flight_warnings(cfg, s));
    warnings.extend(aero_range_warning(&range));
    let mut json = summary_json(cfg, s, &warnings)?;
    json["aero_table_range"] = aero_range_json(&range);
    std::fs::write(out_path(cfg, "summary.json")?, serde_json::to_string_pretty(&json)?)?;
    if u.plain() {
        println!("Flight ({descent:?}, wind {:.1} m/s from {:.0} deg)", s.wind_speed, s.wind_direction_deg);
        println!("  rail exit       {:.2} s, {:.1} m/s, stability {:.2} cal", s.rail_exit_time, s.rail_exit_speed, s.stability_at_rail_exit);
        println!("  max speed       {:.1} m/s (Mach {:.2}), max q {:.1} kPa", s.max_speed, s.max_mach, s.max_dyn_pressure / 1000.0);
        println!("  min stability   {:.2} cal", s.min_stability_cal);
        println!("  apogee          {:.1} m at {:.2} s", s.apogee, s.apogee_time);
        if landed {
            println!("  landing         {:.1} s, E {:.1} m, N {:.1} m ({:.6}, {:.6}), {:.1} m/s", s.landing_time, s.landing_east, s.landing_north, s.landing_lat, s.landing_lon, s.landing_speed);
        } else {
            println!("  landing         NONE: the rocket did not reach the ground within sim.max_time = {} s", cfg.sim.max_time);
        }
        println!("  -> {}", path.display());
    } else if u.rich() {
        println!();
        u.print_panel(&view::flight_panel(s, descent, true));
    }
    print_warnings(&warnings);
    if landed {
        write_kml(cfg, kml::Parts { flight: Some(kml::Flight { samples: r.samples, summary: r.summary }), ..Default::default() }, false)?;
        Ok(None)
    } else {
        // The previous run's flight must not stay in the file: write it without the flight layer.
        if cfg.output.kml {
            write_kml(cfg, kml::Parts::default(), true)?;
            if u.mode != ui::Mode::Quiet {
                println!("  KML written without the flight layer: the flight has no landing point");
            }
        }
        Ok(Some(format!("the flight did not land within sim.max_time = {} s", cfg.sim.max_time)))
    }
}

/// summary.json: the flight summary plus `landed`, the run's `config_hash` and the sanity `warnings`.
fn summary_json(cfg: &Config, s: &Summary, warnings: &[String]) -> Result<serde_json::Value> {
    let mut json = serde_json::to_value(s)?;
    json["landed"] = s.landed().into();
    // Smallest static margin of the flight, including the rail exit; `stable` is false when it is negative.
    let margin = [s.min_stability_cal, s.stability_at_rail_exit].into_iter().filter(|m| m.is_finite()).min_by(f64::total_cmp);
    json["static_margin_min_cal"] = margin.map_or(serde_json::Value::Null, Into::into);
    json["stable"] = margin.is_none_or(|m| m >= 0.0).into();
    json["max_time"] = cfg.sim.max_time.into();
    json["config_hash"] = run_hash(cfg, RunKind::Flight).into();
    json["warnings"] = serde_json::json!(warnings);
    Ok(json)
}

/// Writes `ignisyeet.kml`: the parts given are used as they are, the others are read back from the
/// output files already in the output directory (a missing part is left out, an unreadable one is
/// noted in plain mode).
fn write_kml(cfg: &Config, mut parts: kml::Parts, skip_flight: bool) -> Result<()> {
    if !cfg.output.kml {
        return Ok(());
    }
    let u = ui();
    let dir = cfg.out_dir();
    // A file of an earlier run with a different configuration is stale: it is left out and always reported.
    let note = |what: &str, e: String| {
        let stale = e.starts_with(kml::STALE);
        let msg = format!("KML: {what} left out ({e})");
        match u.mode {
            ui::Mode::Plain if stale => u.warn(msg),
            ui::Mode::Plain => println!("  {msg}"),
            ui::Mode::Rich => u.warn(msg),
            ui::Mode::Quiet if stale => eprintln!("warning: {msg}"),
            ui::Mode::Quiet => {}
        }
    };
    if parts.flight.is_none() && !skip_flight {
        match kml::load_flight(&dir, &run_hash(cfg, RunKind::Flight)) {
            Ok(f) => parts.flight = f,
            Err(e) => note("flight", e),
        }
    }
    if parts.wind_grid.is_none() {
        match kml::load_wind_grid(&dir, &run_hash(cfg, RunKind::WindGrid)) {
            Ok(w) => parts.wind_grid = w,
            Err(e) => note("wind grid", e),
        }
    }
    if parts.monte_carlo.is_none() {
        match kml::load_monte_carlo(&dir, &run_hash(cfg, RunKind::MonteCarlo)) {
            Ok(m) => parts.monte_carlo = m,
            Err(e) => note("Monte Carlo", e),
        }
    }
    let path = out_path(cfg, "ignisyeet.kml")?;
    let tmp = path.with_extension("kml.tmp");
    std::fs::write(&tmp, kml::combined(&cfg.launch, &parts))?;
    std::fs::rename(&tmp, &path)?;
    if u.plain() {
        println!("  -> {}", path.display());
    }
    Ok(())
}

fn write_trajectory(path: &Path, samples: &[sim::flight::Sample]) -> Result<()> {
    let mut f = std::io::BufWriter::new(std::fs::File::create(path)?);
    writeln!(f, "t,phase,east,north,up,lat,lon,alt,vel_e,vel_n,vel_u,airspeed,mach,alpha_deg,thrust,mass,x_cg,x_cp,stability_cal,dyn_pressure,cn,ca,pitch_deg,heading_deg,wx,wy,wz,beta_deg,roll_rate,cl,cy")?;
    for s in samples {
        let phase = match s.phase {
            sim::flight::Phase::Rail => "rail",
            sim::flight::Phase::Free => "free",
            sim::flight::Phase::Parachute => "parachute",
        };
        writeln!(
            f,
            "{:.4},{phase},{:.3},{:.3},{:.3},{:.8},{:.8},{:.3},{:.4},{:.4},{:.4},{:.4},{:.5},{:.4},{:.3},{:.4},{:.5},{:.5},{:.4},{:.2},{:.5},{:.5},{:.4},{:.4},{:.5},{:.5},{:.5},{:.4},{:.5},{:.6},{:.5}",
            s.t, s.east, s.north, s.up, s.lat, s.lon, s.alt, s.vel_e, s.vel_n, s.vel_u, s.airspeed, s.mach, s.alpha_deg, s.thrust, s.mass,
            s.x_cg, s.x_cp, s.stability_cal, s.dyn_pressure, s.cn, s.ca, s.pitch_deg, s.heading_deg, s.wx, s.wy, s.wz, s.beta_deg, s.roll_rate, s.cl, s.cy
        )?;
    }
    Ok(())
}

const DISP_T: &str = "  {spinner:.cyan} {prefix:<12.bold} {bar} {pos}/{len} {percent:>3}%  {rate:.dim}  {elapsed:.dim}  ETA {eta_precise:.yellow}\n      {msg}";

/// Template key `{rate}`: finished runs per second.
fn rate_key(st: indicatif::ProgressStyle) -> indicatif::ProgressStyle {
    st.with_key("rate", |s: &indicatif::ProgressState, w: &mut dyn std::fmt::Write| {
        let _ = write!(w, "{:.0}/s", s.per_sec());
    })
}

/// Tracks the farthest landing per descent mode and the progress message built from it.
struct FarthestLanding {
    max: std::sync::Mutex<[f64; 2]>,
}

impl FarthestLanding {
    fn new() -> Self {
        Self { max: std::sync::Mutex::new([0.0; 2]) }
    }
    fn update(&self, descent: Descent, d: f64) -> String {
        use console::style;
        let mut m = self.max.lock().unwrap();
        let i = (descent == Descent::Parachute) as usize;
        if d.is_finite() && d > m[i] {
            m[i] = d;
        }
        let par = if m[1] > 0.0 { format!("   {} {:.0} m", style("parachute").green(), m[1]) } else { String::new() };
        format!("max landing distance so far:  {} {:.0} m{par}", style("ballistic").red(), m[0])
    }
}

/// Warnings of the nominal configuration: body size, and stability, wind-model validity and landing of the nominal flight.
fn nominal_warnings(cfg: &Config, p: &Prepared, base: &Simulation) -> Vec<String> {
    let mut w = p.geometry_warnings.clone();
    w.extend(size_warnings(cfg, p.table.meta.length, p.table.meta.ref_diameter));
    let ref_speed = if cfg.dispersion.mode == sim::DispersionMode::WindGrid { cfg.dispersion.wind_speeds.iter().copied().fold(0.0, f64::max) } else { cfg.wind.speed };
    match base.run(Descent::Ballistic, false) {
        Ok(r) => w.extend(flight_warnings_at(cfg, &r.summary, ref_speed)),
        Err(e) => w.push(format!("the nominal flight could not be simulated ({e:#}): stability and wind-model checks were skipped")),
    }
    w
}

/// Max landing distance line of one descent mode of the wind grid; `distances` are those of the cases that landed.
fn wind_grid_line(mode: Descent, distances: &[f64], cases: usize) -> String {
    let failed = cases - distances.len();
    let far = distances.iter().copied().fold(0.0, f64::max);
    if distances.is_empty() {
        format!("{mode:?}: NO CASE LANDED ({failed} of {cases} failed): no landing distance available")
    } else if failed > 0 {
        format!("{mode:?}: max landing distance \u{2265} {far:.1} m ({failed} of {cases} cases did not land: lower bound)")
    } else {
        format!("{mode:?}: max landing distance {far:.1} m")
    }
}

fn run_dispersion(cfg: &Config) -> Result<Option<String>> {
    let u = ui();
    let p = prepare(cfg)?;
    let base = simulation(cfg, &p)?;
    let mut warnings = nominal_warnings(cfg, &p, &base);
    if cfg.dispersion.mode == sim::DispersionMode::MonteCarlo {
        return run_monte_carlo(cfg, &p, &base, warnings);
    }
    let start = std::time::Instant::now();
    let total = sim::dispersion::wind_grid_runs(&cfg.dispersion, cfg.recovery.enabled);
    let (cases, results) = if u.progress_on() {
        let pb = u.bar_with(total as u64, "Wind grid", DISP_T, rate_key);
        let far = FarthestLanding::new();
        let r = sim::dispersion::run_with_progress(&cfg.dispersion, &base, cfg.recovery.enabled, &|c, s| {
            pb.set_message(far.update(c.descent, s.landing_distance));
            pb.inc(1);
        });
        pb.finish_and_clear();
        r?
    } else {
        sim::dispersion::run(&cfg.dispersion, &base, cfg.recovery.enabled)?
    };
    let mut modes: Vec<Descent> = Vec::new();
    for c in &cases {
        if !modes.contains(&c.descent) {
            modes.push(c.descent);
        }
    }
    let failed = results.iter().filter(|s| !s.landed()).count();
    if failed > 0 {
        warnings.push(format!(
            "{failed} of {} wind-grid cases did not land within sim.max_time = {} s: they are excluded from the landing distances, so the maximum landing distance is only a lower bound",
            results.len(),
            cfg.sim.max_time
        ));
    }
    warnings.extend(aero_range_warning(&p.table.out_of_range_stats()));
    let path = out_path(cfg, "dispersion.csv")?;
    write_wind_grid_csv(&path, &results)?;
    let meta = serde_json::json!({ "config_hash": run_hash(cfg, RunKind::WindGrid), "cases": results.len(), "failed": failed, "warnings": warnings, "aero_table_range": aero_range_json(&p.table.out_of_range_stats()) });
    std::fs::write(out_path(cfg, kml::WIND_GRID_META)?, serde_json::to_string_pretty(&meta)?)?;
    u.done_line("Wind grid", &format!("{} cases, {failed} failed", results.len()), start.elapsed());
    if u.plain() {
        println!("Dispersion: {} cases in {:.2?} -> {}", results.len(), start.elapsed(), path.display());
        for &m in &modes {
            let mine: Vec<&Summary> = results.iter().filter(|s| s.descent == m).collect();
            let landed: Vec<f64> = mine.iter().filter(|s| s.landed()).map(|s| s.landing_distance).collect();
            println!("  {}", wind_grid_line(m, &landed, mine.len()));
        }
    } else if u.rich() {
        println!();
        u.print_panel(&view::wind_grid_panel(&cases, &results, start.elapsed()));
    }
    print_warnings(&warnings);
    let incomplete = (failed > 0).then(|| format!("{failed} of {} wind-grid cases did not land", results.len()));
    let (cases, results): (Vec<_>, Vec<_>) = cases.into_iter().zip(results).filter(|(_, s)| s.landed()).unzip();
    if results.is_empty() {
        if u.mode != ui::Mode::Quiet {
            println!("  KML: wind grid left out (no case landed)");
        }
        return Ok(incomplete);
    }
    write_kml(cfg, kml::Parts { wind_grid: Some(kml::WindGrid { cases, results }), ..Default::default() }, false)?;
    Ok(incomplete)
}

fn write_wind_grid_csv(path: &Path, results: &[sim::Summary]) -> Result<()> {
    let mut f = std::io::BufWriter::new(std::fs::File::create(path)?);
    writeln!(f, "descent,wind_speed,wind_direction_deg,landing_east,landing_north,landing_lat,landing_lon,landing_distance,landing_time,landing_speed,apogee,apogee_time,max_mach,rail_exit_speed,min_stability_cal,landed")?;
    for s in results {
        let mode = if s.descent == Descent::Ballistic { "ballistic" } else { "parachute" };
        writeln!(
            f,
            "{mode},{},{},{:.2},{:.2},{:.8},{:.8},{:.2},{:.2},{:.2},{:.2},{:.2},{:.4},{:.2},{:.3},{}",
            s.wind_speed, s.wind_direction_deg, s.landing_east, s.landing_north, s.landing_lat, s.landing_lon, s.landing_distance,
            s.landing_time, s.landing_speed, s.apogee, s.apogee_time, s.max_mach, s.rail_exit_speed, s.min_stability_cal, s.landed()
        )?;
    }
    f.flush()?;
    Ok(())
}

fn descent_name(d: Descent) -> &'static str {
    if d == Descent::Ballistic {
        "ballistic"
    } else {
        "parachute"
    }
}

fn run_monte_carlo(cfg: &Config, p: &Prepared, base: &Simulation, mut warnings: Vec<String>) -> Result<Option<String>> {
    let u = ui();
    let start = std::time::Instant::now();
    let mc = &cfg.dispersion.monte_carlo;
    let r = if u.progress_on() {
        let total = mc.samples * if cfg.recovery.enabled { 2 } else { 1 };
        let pb = u.bar_with(total as u64, "Monte Carlo", DISP_T, rate_key);
        let far = FarthestLanding::new();
        let r = sim::dispersion::run_monte_carlo_with_progress(mc, base, cfg.recovery.enabled, &|row| {
            let d = row.result.as_ref().map_or(f64::NAN, |s| s.landing_distance);
            pb.set_message(far.update(row.descent, d));
            pb.inc(1);
        });
        pb.finish_and_clear();
        r?
    } else {
        sim::dispersion::run_monte_carlo(mc, base, cfg.recovery.enabled)?
    };
    let failed: usize = r.summary.descents.iter().map(|d| d.failed).sum();
    for d in r.summary.descents.iter().filter(|d| d.failed > 0) {
        warnings.push(format!(
            "{:?}: {} of {} Monte Carlo samples did not land or failed ({} did not land within sim.max_time = {} s): the statistics are INCOMPLETE and the maximum landing distance is only a lower bound",
            d.descent, d.failed, r.summary.samples, d.not_landed, cfg.sim.max_time
        ));
    }
    let range = p.table.out_of_range_stats();
    warnings.extend(aero_range_warning(&range));
    let path = out_path(cfg, "dispersion_mc.csv")?;
    write_monte_carlo_csv(&path, &r.rows)?;
    let spath = out_path(cfg, "dispersion_summary.json")?;
    let mut json = serde_json::to_value(&r.summary)?;
    json["config_hash"] = run_hash(cfg, RunKind::MonteCarlo).into();
    json["warnings"] = serde_json::json!(warnings);
    json["aero_table_range"] = aero_range_json(&range);
    std::fs::write(&spath, serde_json::to_string_pretty(&json)?)?;
    u.done_line("Monte Carlo", &format!("{} samples x {} descent modes", r.summary.samples, r.summary.descents.len()), start.elapsed());
    if u.plain() {
        println!("Monte Carlo: {} samples x {} descent modes in {:.2?} -> {}, {}", r.summary.samples, r.summary.descents.len(), start.elapsed(), path.display(), spath.display());
        for d in &r.summary.descents {
            println!("  {:?}: {} ok, {} failed or did not land", d.descent, d.landing.n, d.failed);
            if !d.complete {
                println!("    STATISTICS INCOMPLETE: {} of {} samples are missing from them", d.failed, r.summary.samples);
            }
            if d.landing.n == 0 {
                println!("    no sample landed: no landing statistics available");
                continue;
            }
            let (e1, e3) = (d.landing.ellipse_1sigma, d.landing.ellipse_3sigma);
            println!("    mean landing E {:.1} m, N {:.1} m; apogee {:.1} +- {:.1} m", d.landing.mean_east, d.landing.mean_north, d.apogee_mean, d.apogee_std);
            println!("    1-sigma ellipse {:.1} x {:.1} m, 3-sigma {:.1} x {:.1} m, major axis bearing {:.0} deg", e1.semi_major, e1.semi_minor, e3.semi_major, e3.semi_minor, e1.major_axis_bearing_deg);
            if d.max_landing_distance_is_lower_bound {
                println!("    max landing distance \u{2265} {:.1} m ({} samples did not land or failed; sample {} is the farthest that landed)", d.max_landing_distance, d.failed, d.max_landing_distance_sample.unwrap_or(0));
            } else {
                println!("    max landing distance {:.1} m (sample {})", d.max_landing_distance, d.max_landing_distance_sample.unwrap_or(0));
            }
            println!("    landing distance P50 {:.1} m, P90 {:.1} m, P99 {:.1} m (landed samples)", d.landing_distance_p50, d.landing_distance_p90, d.landing_distance_p99);
        }
    } else if u.rich() {
        println!();
        u.print_panel(&view::monte_carlo_panel(&r.summary, start.elapsed(), r.rows.len()));
    }
    print_warnings(&warnings);
    if r.summary.descents.iter().all(|d| d.landing.n == 0) {
        if u.mode != ui::Mode::Quiet {
            println!("  KML: Monte Carlo left out (no sample landed)");
        }
    } else {
        write_kml(cfg, kml::Parts { monte_carlo: Some(kml::MonteCarlo { rows: r.rows.clone(), stats: r.summary.descents.clone() }), ..Default::default() }, false)?;
    }
    for row in r.rows.iter().filter(|x| x.result.is_err()).take(3) {
        let msg = format!("failed: sample {} ({}): {}", row.sample, descent_name(row.descent), row.result.as_ref().unwrap_err());
        match u.mode {
            ui::Mode::Plain => println!("  {msg}"),
            ui::Mode::Rich => u.warn(msg),
            ui::Mode::Quiet => eprintln!("{msg}"),
        }
    }
    Ok((failed > 0).then(|| format!("{failed} Monte Carlo runs did not land or failed; the statistics are incomplete")))
}

fn write_monte_carlo_csv(path: &Path, rows: &[sim::dispersion::McRow]) -> Result<()> {
    let mut f = std::io::BufWriter::new(std::fs::File::create(path)?);
    writeln!(
        f,
        "sample,descent,status,thrust_scale,burn_time_scale,dry_mass,cg,cn_scale,ca_scale,elevation_deg,azimuth_deg,wind_speed_delta,wind_direction_deg_delta,parachute_cd_s_scale,\
wind_speed,wind_direction_deg,landing_east,landing_north,landing_lat,landing_lon,landing_distance,landing_time,landing_speed,apogee,apogee_time,max_mach,rail_exit_speed,min_stability_cal,\
thrust_misalignment_y_deg,thrust_misalignment_z_deg,thrust_offset_y,thrust_offset_z,fin_cant_deg,cg_lateral_y,cg_lateral_z,cl_scale,clp_scale"
    )?;
    for row in rows {
        let p = &row.perturbation;
        write!(
            f,
            "{},{},{},{},{},{},{},{},{},{},{},{},{},{},",
            row.sample,
            descent_name(row.descent),
            if row.result.is_ok() { "ok" } else { "failed" },
            p.thrust_scale, p.burn_time_scale, p.dry_mass, p.cg, p.cn_scale, p.ca_scale, p.elevation_deg, p.azimuth_deg, p.wind_speed, p.wind_direction_deg, p.parachute_cd_s_scale
        )?;
        match &row.result {
            Ok(s) => write!(
                f,
                "{},{},{:.2},{:.2},{:.8},{:.8},{:.2},{:.2},{:.2},{:.2},{:.2},{:.4},{:.2},{:.3}",
                s.wind_speed, s.wind_direction_deg, s.landing_east, s.landing_north, s.landing_lat, s.landing_lon, s.landing_distance,
                s.landing_time, s.landing_speed, s.apogee, s.apogee_time, s.max_mach, s.rail_exit_speed, s.min_stability_cal
            )?,
            Err(_) => write!(f, ",,,,,,,,,,,,,")?,
        }
        writeln!(
            f,
            ",{},{},{},{},{},{},{},{},{}",
            p.misalignment_y_deg, p.misalignment_z_deg, p.offset_y, p.offset_z, p.fin_cant_deg, p.cg_lateral_y, p.cg_lateral_z, p.cl_scale, p.clp_scale
        )?;
    }
    f.flush()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn table_method_loads_external_table() {
        let dir = std::env::temp_dir().join("ignisyeet_cli_table_test");
        std::fs::create_dir_all(&dir).unwrap();
        let meta = aero::table::TableMeta {
            source_hash: "ext".into(),
            ref_area: 0.01,
            ref_diameter: 0.1,
            length: 1.0,
            machs: vec![0.0, 1.0],
            alphas_deg: vec![0.0, 10.0],
            extrapolation: aero::Extrapolation::Clamp,
            ..Default::default()
        };
        AeroTable::from_fn(meta, |_, a| aero::AeroCoeffs { cn: a, ca_off: 0.5, xcp: 0.6, ..Default::default() }).save(&dir.join("cfd.csv")).unwrap();
        let text = include_str!("../../../examples/sample.toml").replace("method = \"barrowman\"", "method = \"table\"\ntable = \"cfd.csv\"");
        let mut cfg = Config::from_value(toml::from_str(&text).unwrap(), Some(&dir)).unwrap();
        let t = aero_table(&cfg, false).unwrap();
        assert_eq!(t.meta.source_hash, "ext");
        assert!((t.lookup(0.5, 5f64.to_radians()).cn - 5.0).abs() < 1e-6);
        // A panel table needs the STL, which this directory does not have.
        cfg.aero.method = config::AeroMethod::Panel;
        assert!(aero_table(&cfg, false).is_err());
    }

    #[test]
    fn external_table_without_roll_terms_gets_barrowman_roll_terms() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples");
        let tmp = std::env::temp_dir().join(format!("ignisyeet_cli_legacy_{}", std::process::id()));
        std::fs::create_dir_all(&tmp).unwrap();
        let meta = aero::table::TableMeta { source_hash: "ext".into(), ref_area: 0.01, ref_diameter: 0.1, length: 1.0, machs: vec![0.0, 1.0], alphas_deg: vec![0.0, 10.0], ..Default::default() };
        let csv = tmp.join("legacy.csv");
        AeroTable::from_fn(meta, |_, a| aero::AeroCoeffs { cn: a, ca_off: 0.5, xcp: 0.6, ..Default::default() }).save(&csv).unwrap();
        let text = include_str!("../../../examples/sample.toml").replace("method = \"barrowman\"", &format!("method = \"table\"\ntable = \"{}\"", csv.display()));
        let cfg = Config::from_value(toml::from_str(&text).unwrap(), Some(&dir)).unwrap();
        let t = aero_table(&cfg, false).unwrap();
        let c = t.lookup(0.5, 0.0);
        assert!(c.cl_delta > 0.0 && c.clp < 0.0, "{c:?}");
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn monte_carlo_csv_rows_match_the_header() {
        let mut rows = Vec::new();
        for ok in [true, false] {
            let result = if ok {
                let cfg = sample_summary();
                Ok(cfg)
            } else {
                Err("failed".to_string())
            };
            rows.push(sim::dispersion::McRow { sample: 0, descent: Descent::Ballistic, perturbation: sim::dispersion::Perturbation { cl_scale: 0.25, ..Default::default() }, result });
        }
        let path = std::env::temp_dir().join(format!("ignisyeet_mc_csv_{}.csv", std::process::id()));
        write_monte_carlo_csv(&path, &rows).unwrap();
        let text = std::fs::read_to_string(&path).unwrap();
        let _ = std::fs::remove_file(&path);
        let lines: Vec<Vec<&str>> = text.lines().map(|l| l.split(',').collect()).collect();
        assert_eq!(lines.len(), 3);
        assert!(lines.iter().all(|l| l.len() == lines[0].len()), "{:?}", lines.iter().map(Vec::len).collect::<Vec<_>>());
        // New columns are appended at the end.
        let col = lines[0].iter().position(|c| *c == "cl_scale").unwrap();
        assert_eq!(col, lines[0].len() - 2);
        assert_eq!(lines[1][col], "0.25");
    }

    fn sample_summary() -> sim::Summary {
        sim::Summary {
            descent: Descent::Ballistic,
            wind_speed: 1.0,
            wind_direction_deg: 2.0,
            rail_exit_time: 0.1,
            rail_exit_speed: 20.0,
            burnout_time: 2.0,
            max_speed: 100.0,
            max_mach: 0.3,
            max_dyn_pressure: 1.0,
            apogee_time: 10.0,
            apogee: 500.0,
            apogee_east: 1.0,
            apogee_north: 1.0,
            min_stability_cal: 2.0,
            stability_at_rail_exit: 2.0,
            landing_time: 30.0,
            landing_east: 10.0,
            landing_north: 20.0,
            landing_lat: 35.0,
            landing_lon: 139.0,
            landing_distance: 22.0,
            landing_speed: 30.0,
            main_deploy_time: None,
            main_deploy_altitude: None,
        }
    }

    #[test]
    fn resources_do_not_change_the_hash() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples");
        let text = include_str!("../../../examples/sample.toml");
        let load = |t: &str| Config::from_value(toml::from_str(t).unwrap(), Some(&dir)).unwrap();
        let a = load(text);
        let b = load(&text.replace("threads = 0 ", "threads = 1 ").replace("memory_gb = 0 ", "memory_gb = 4 ").replace("nice = 0 ", "nice = 5 "));
        assert_eq!((b.resources.threads, b.resources.nice), (1, 5));
        assert_eq!(source_hash(&a).unwrap(), source_hash(&b).unwrap());
    }

    fn load_sample(text: &str) -> Config {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples");
        Config::from_value(toml::from_str(text).unwrap(), Some(&dir)).unwrap()
    }

    #[test]
    fn cache_key_ignores_unused_blocks_and_cfd_resources_but_not_the_model() {
        let text = include_str!("../../../examples/sample.toml");
        let base = source_hash(&load_sample(text)).unwrap();
        // Panel settings are not used by the Barrowman method.
        assert_eq!(base, source_hash(&load_sample(&text.replace("body_axial = 80", "body_axial = 90"))).unwrap());
        // A [aero.cfd] block is unused unless method = "cfd".
        let with_cfd = |extra: &str| format!("{text}\n[aero.cfd]\nmodel = \"rans\"\nmachs = [0.5, 2.0]\nalphas_deg = [0, 4]\n{extra}");
        let barrowman_cfd = source_hash(&load_sample(&with_cfd(""))).unwrap();
        assert_eq!(base, barrowman_cfd);
        // With method = "cfd" only the resource/path fields are ignored; physical settings are not.
        let cfd_text = |extra: &str| with_cfd(extra).replace("method = \"barrowman\"", "method = \"cfd\"");
        let plain = source_hash(&load_sample(&cfd_text(""))).unwrap();
        for resource in ["prefix = \"/opt/su2\"", "su2 = \"SU2_CFD2\"", "mpi = \"mpiexec\"", "ranks_per_case = 3", "parallel_cases = 2", "timeout_minutes = 5.0", "workdir = \"elsewhere\""] {
            assert_eq!(plain, source_hash(&load_sample(&cfd_text(resource))).unwrap(), "{resource}");
        }
        assert_ne!(plain, source_hash(&load_sample(&cfd_text("iterations = 77"))).unwrap());
        assert_ne!(base, plain);
        // The aero settings of the Barrowman method still matter.
        assert_ne!(base, source_hash(&load_sample(&text.replace("mach_step = 0.02", "mach_step = 0.04"))).unwrap());
    }

    #[test]
    fn model_version_is_part_of_the_cache_key() {
        // The version constant enters the hashed inputs; this guards against removing it.
        let src = include_str!("main.rs");
        assert!(src.contains("model_version: MODEL_VERSION"));
    }

    #[test]
    fn run_hash_follows_the_configuration_but_not_output_or_resources() {
        let text = include_str!("../../../examples/sample.toml");
        let a = load_sample(text);
        let flight = |c: &Config| run_hash(c, RunKind::Flight);
        let same = load_sample(&text.replace("dir = \"out\"", "dir = \"elsewhere\"").replace("threads = 0 ", "threads = 1 "));
        assert_eq!(flight(&a), flight(&same));
        let windier = load_sample(&text.replace("speed = 4.0 ", "speed = 5.0 "));
        assert_ne!(flight(&a), flight(&windier));
        // Dispersion settings do not make a flight stale, but do make the wind grid stale.
        let other_grid = load_sample(&text.replace("directions = 8", "directions = 12"));
        assert_eq!(flight(&a), flight(&other_grid));
        assert_ne!(run_hash(&a, RunKind::WindGrid), run_hash(&other_grid, RunKind::WindGrid));
        assert_eq!(run_hash(&a, RunKind::MonteCarlo), run_hash(&other_grid, RunKind::MonteCarlo));
    }

    fn unlanded(s: sim::Summary) -> sim::Summary {
        sim::Summary { landing_time: f64::NAN, landing_east: f64::NAN, landing_north: f64::NAN, landing_lat: f64::NAN, landing_lon: f64::NAN, landing_distance: f64::NAN, landing_speed: f64::NAN, ..s }
    }

    #[test]
    fn stability_wind_and_landing_warnings() {
        let cfg = load_sample(include_str!("../../../examples/sample.toml"));
        let ok = sim::Summary { min_stability_cal: 2.0, stability_at_rail_exit: 2.0, apogee: 500.0, ..sample_summary() };
        assert!(flight_warnings(&cfg, &ok).is_empty());
        let marginal = sim::Summary { min_stability_cal: 1.2, ..ok.clone() };
        assert!(flight_warnings(&cfg, &marginal).iter().any(|w| w.contains("1.20 cal") && w.contains("marginal")));
        let unstable = sim::Summary { min_stability_cal: 0.6, ..ok.clone() };
        assert!(flight_warnings(&cfg, &unstable).iter().any(|w| w.contains("low static margin")));
        let forward = sim::Summary { min_stability_cal: -0.4, ..ok.clone() };
        assert!(flight_warnings(&cfg, &forward).iter().any(|w| w.contains("forward of the CG")));
        // The margin at rail exit counts too.
        let rail = sim::Summary { stability_at_rail_exit: 0.9, ..ok.clone() };
        assert!(flight_warnings(&cfg, &rail).iter().any(|w| w.contains("0.90 cal")));
        // The sample wind is a power law: apogees above 1000 m are an extrapolation.
        let high = sim::Summary { apogee: 4440.7, ..ok.clone() };
        let w = flight_warnings(&cfg, &high);
        assert!(w.iter().any(|w| w.contains("extrapolated") && w.contains("wind.ref_height")), "{w:?}");
        let mut constant = cfg.clone();
        constant.wind.model = sim::WindModel::Constant;
        assert!(flight_warnings(&constant, &high).is_empty());
        let w = flight_warnings(&cfg, &unlanded(ok));
        assert!(w.iter().any(|w| w.contains("DID NOT LAND") && w.contains("sim.max_time")), "{w:?}");
    }

    #[test]
    fn implausible_size_and_cg_outside_the_body() {
        let cfg = load_sample(include_str!("../../../examples/sample.toml"));
        assert!(size_warnings(&cfg, 2.0, 0.1).is_empty());
        let w = size_warnings(&cfg, 2000.0, 100.0);
        assert!(w.len() == 2 && w.iter().all(|w| w.contains("stl_scale")), "{w:?}");
        assert!(size_warnings(&cfg, 0.01, 0.1).iter().any(|w| w.contains("length")));
        let mut c = cfg.clone();
        c.rocket.cg_dry = 3.0;
        let e = check_cg(&c, 2.0).unwrap_err().to_string();
        assert!(e.contains("rocket.cg_dry") && e.contains("outside the body"), "{e}");
        assert!(check_cg(&cfg, 2.0).is_ok());
        // The real geometry check accepts the sample and rejects a CG beyond its length.
        let g = geometry(&cfg).unwrap();
        c.rocket.cg_dry = g.length + 0.5;
        assert!(geometry(&c).unwrap_err().to_string().contains("rocket.cg_dry"));
    }

    #[test]
    fn summary_json_records_landing_hash_and_warnings() {
        let cfg = load_sample(include_str!("../../../examples/sample.toml"));
        let j = summary_json(&cfg, &sample_summary(), &["w1".to_string()]).unwrap();
        assert_eq!(j["landed"], true);
        assert_eq!(j["warnings"][0], "w1");
        assert_eq!(j["config_hash"], run_hash(&cfg, RunKind::Flight));
        let j = summary_json(&cfg, &unlanded(sample_summary()), &[]).unwrap();
        assert_eq!(j["landed"], false);
        assert!(j["landing_east"].is_null() && j["max_time"] == cfg.sim.max_time);
    }

    #[test]
    fn aero_range_warning_and_json() {
        let none = aero::table::OutOfRangeStats { lookups: 10, ..Default::default() };
        assert!(aero_range_warning(&none).is_none());
        assert_eq!(aero_range_json(&none)["out_of_range"], false);
        let s = aero::table::OutOfRangeStats { lookups: 100, mach_above: 7, angle_beyond: 3, max_mach: 2.5, max_abs_alpha_deg: 31.0, ..Default::default() };
        let w = aero_range_warning(&s).unwrap();
        assert!(w.contains("7 of 100 lookups beyond the Mach range") && w.contains("3 of 100 lookups beyond the angle range"), "{w}");
        assert!(w.contains("max Mach 2.50") && w.contains("31.0 deg") && w.contains("held at the table edge"), "{w}");
        let j = aero_range_json(&s);
        assert_eq!((j["mach_above"].as_u64(), j["angle_beyond"].as_u64(), j["out_of_range"].as_bool()), (Some(7), Some(3), Some(true)));
    }

    #[test]
    fn geometry_warnings_are_read_back_from_geometry_json() {
        let tmp = std::env::temp_dir().join(format!("ignisyeet_cli_gw_{}", std::process::id()));
        std::fs::create_dir_all(&tmp).unwrap();
        let mut cfg = load_sample(include_str!("../../../examples/sample.toml"));
        cfg.output.dir = tmp.to_string_lossy().into_owned().into();
        std::fs::write(tmp.join("geometry.json"), r#"{"warnings":["fins look reversed"]}"#).unwrap();
        assert_eq!(stored_geometry_warnings(&cfg), vec!["fins look reversed".to_string()]);
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn low_static_margin_is_not_called_unstable() {
        let cfg = load_sample(include_str!("../../../examples/sample.toml"));
        let ok = sim::Summary { min_stability_cal: 2.0, stability_at_rail_exit: 2.0, apogee: 500.0, ..sample_summary() };
        let low = sim::Summary { min_stability_cal: 0.6, ..ok.clone() };
        let w = flight_warnings(&cfg, &low);
        assert!(w.iter().any(|w| w.contains("low static margin")) && !w.iter().any(|w| w.contains("UNSTABLE")), "{w:?}");
        let neg = sim::Summary { min_stability_cal: -0.1, ..ok.clone() };
        assert!(flight_warnings(&cfg, &neg).iter().any(|w| w.contains("UNSTABLE")));
        // summary.json: the smallest margin and a stable flag.
        let j = summary_json(&cfg, &low, &[]).unwrap();
        assert_eq!((j["static_margin_min_cal"].as_f64(), j["stable"].as_bool()), (Some(0.6), Some(true)));
        let j = summary_json(&cfg, &neg, &[]).unwrap();
        assert_eq!((j["static_margin_min_cal"].as_f64(), j["stable"].as_bool()), (Some(-0.1), Some(false)));
        let rail = sim::Summary { stability_at_rail_exit: -0.3, ..ok };
        assert_eq!(summary_json(&cfg, &rail, &[]).unwrap()["stable"].as_bool(), Some(false));
    }

    #[test]
    fn wind_extrapolation_warning_needs_wind() {
        let mut cfg = load_sample(include_str!("../../../examples/sample.toml"));
        let high = sim::Summary { min_stability_cal: 2.0, stability_at_rail_exit: 2.0, apogee: 4440.7, ..sample_summary() };
        assert!(!flight_warnings(&cfg, &high).is_empty());
        assert!(flight_warnings_at(&cfg, &high, 0.0).is_empty());
        cfg.wind.speed = 0.0;
        assert!(flight_warnings(&cfg, &high).is_empty());
    }

    #[test]
    fn missing_input_files_name_the_path() {
        let mut cfg = load_sample(include_str!("../../../examples/sample.toml"));
        cfg.rocket.stl = "no_such_rocket.stl".into();
        for e in [source_hash(&cfg).unwrap_err(), geometry(&cfg).unwrap_err()] {
            assert!(format!("{e:#}").contains("cannot read STL") && format!("{e:#}").contains("no_such_rocket.stl"), "{e:#}");
        }
        let mut cfg = load_sample(include_str!("../../../examples/sample.toml"));
        cfg.motor.eng = "no_such_motor.eng".into();
        assert!(format!("{:#}", load_motor(&cfg).unwrap_err()).contains("no_such_motor.eng"));
        let mut cfg = sample_config("wprof");
        let p = prepare(&cfg).unwrap();
        cfg.wind.model = sim::WindModel::Profile;
        cfg.wind.profile = Some("no_such_wind.csv".into());
        let e = simulation(&cfg, &p).err().expect("missing wind profile accepted");
        assert!(format!("{e:#}").contains("no_such_wind.csv"), "{e:#}");
        let _ = std::fs::remove_dir_all(&cfg.output.dir);
    }

    #[test]
    fn implausible_body_length_stops_with_a_unit_hint() {
        let mut cfg = sample_config("size");
        cfg.rocket.stl_scale = 1000.0;
        let e = format!("{:#}", geometry(&cfg).unwrap_err());
        assert!(e.contains("body length") && e.contains("rocket.stl_scale") && e.contains("0.1-20 m"), "{e}");
        cfg.rocket.stl_scale = 1e-6;
        assert!(geometry(&cfg).is_err());
    }

    #[test]
    fn cache_key_includes_the_extracted_geometry() {
        let src = include_str!("main.rs");
        let body = &src[src.find("fn source_hash").unwrap()..];
        let body = &body[..body.find("let inputs = Inputs").unwrap()];
        assert!(body.contains("geometry(cfg)"), "source_hash must hash the extracted geometry");
        // Same STL bytes, different extraction input: different key.
        let cfg = sample_config("key");
        let mut other = cfg.clone();
        other.aero.n_slices += 10;
        assert_ne!(source_hash(&cfg).unwrap(), source_hash(&other).unwrap());
    }

    #[test]
    fn unlanded_flight_replaces_a_stale_kml_without_the_flight() {
        let mut cfg = sample_config("kml");
        cfg.output.kml = true;
        let dir = cfg.out_dir();
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("ignisyeet.kml");
        std::fs::write(&path, "STALE_FLIGHT_MARKER").unwrap();
        write_kml(&cfg, kml::Parts::default(), true).unwrap();
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(!text.contains("STALE_FLIGHT_MARKER") && text.contains("Launch site") && !text.contains("<name>Flight</name>"), "{text}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn wind_grid_line_marks_failures() {
        assert_eq!(wind_grid_line(Descent::Ballistic, &[10.0, 30.0], 2), "Ballistic: max landing distance 30.0 m");
        let l = wind_grid_line(Descent::Ballistic, &[10.0, 30.0], 3);
        assert!(l.contains("\u{2265} 30.0 m") && l.contains("1 of 3"), "{l}");
        let l = wind_grid_line(Descent::Parachute, &[], 4);
        assert!(l.contains("NO CASE LANDED") && !l.contains("0.0 m"), "{l}");
    }

    #[test]
    fn sample_flight_lands_and_warns_about_the_extrapolated_wind() {
        let cfg = sample_config("warn");
        let p = prepare(&cfg).unwrap();
        let (r, _) = fly_sample(&cfg, &p, Descent::Parachute);
        assert!(r.summary.landed());
        let w = flight_warnings(&cfg, &r.summary);
        assert_eq!(w.len(), 1, "{w:?}");
        assert!(w[0].contains("extrapolated"));
        let _ = std::fs::remove_dir_all(&cfg.output.dir);
    }

    /// examples/sample.toml (barrowman aero), output in a private temp directory.
    fn sample_config(tag: &str) -> Config {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples");
        let mut cfg = Config::from_value(toml::from_str(include_str!("../../../examples/sample.toml")).unwrap(), Some(&dir)).unwrap();
        cfg.output.dir = std::env::temp_dir().join(format!("ignisyeet_cli_{tag}_{}", std::process::id()));
        cfg
    }

    fn fly_sample(cfg: &Config, p: &Prepared, descent: Descent) -> (sim::SimResult, std::time::Duration) {
        let t0 = std::time::Instant::now();
        let r = simulation(cfg, p).unwrap().run(descent, false).unwrap();
        (r, t0.elapsed())
    }

    #[test]
    fn sample_default_results_are_unchanged() {
        let cfg = sample_config("default");
        let p = prepare(&cfg).unwrap();
        let (r, _) = fly_sample(&cfg, &p, Descent::Parachute);
        let s = &r.summary;
        assert!((s.apogee - 4440.7).abs() < 0.05 && (s.apogee_time - 26.11).abs() < 0.005, "{} at {}", s.apogee, s.apogee_time);
        assert!((s.landing_east + 654.8).abs() < 0.05 && (s.landing_north + 7708.5).abs() < 0.05, "{} {}", s.landing_east, s.landing_north);
        let _ = std::fs::remove_dir_all(&cfg.output.dir);
    }

    #[test]
    fn general_table_sample_matches_axisymmetric_apogee() {
        let mut cfg = sample_config("general");
        cfg.aero.table_kind = aero::table::TableKind::General;
        let p = prepare(&cfg).unwrap();
        assert_eq!(p.table.kind(), aero::table::TableKind::General);
        let (r, _) = fly_sample(&cfg, &p, Descent::Parachute);
        assert!((r.summary.apogee - 4440.7).abs() < 0.5, "apogee {}", r.summary.apogee);
        let _ = std::fs::remove_dir_all(&cfg.output.dir);
    }

    #[test]
    fn fin_cant_rolls_the_sample_and_reaches_the_geometry() {
        let mut cfg = sample_config("cant");
        cfg.rocket.fin_cant_deg = 1.0;
        assert_eq!(geometry(&cfg).unwrap().fins.unwrap().cant_deg, 1.0);
        let p = prepare(&cfg).unwrap();
        assert_eq!(p.table.meta.fin_cant_ref_deg, 1.0);
        let sim = simulation(&cfg, &p).unwrap();
        assert_eq!(sim.fin_cant_deg, 1.0);
        let r = sim.run(Descent::Ballistic, true).unwrap();
        let p_max = r.samples.iter().map(|s| s.roll_rate).fold(0.0f64, f64::max);
        assert!(p_max > 1.0, "max roll rate {p_max}");
        let _ = std::fs::remove_dir_all(&cfg.output.dir);
    }

    /// Prints the integrator comparison table (run with `--ignored --nocapture`).
    #[test]
    #[ignore]
    fn integrator_matrix_report() {
        use sim::{AttitudeKind as A, IntegratorKind as I};
        let mut cfg = sample_config("matrix");
        let p = prepare(&cfg).unwrap();
        for wind in [4.0, 0.0] {
        println!("wind {wind} m/s");
        cfg.wind.speed = wind;
        cfg.sim.integrator = I::Rk4;
        cfg.sim.attitude = A::Normalize;
        cfg.sim.dt = 0.0005;
        let refs: Vec<_> = [Descent::Parachute, Descent::Ballistic].iter().map(|&d| fly_sample(&cfg, &p, d)).collect();
        cfg.sim.dt = 0.002;
        println!("reference rk4 dt=0.5ms: parachute apogee {:.3} land ({:.2}, {:.2}); ballistic apogee {:.3} land ({:.2}, {:.2})", refs[0].0.summary.apogee, refs[0].0.summary.landing_east, refs[0].0.summary.landing_north, refs[1].0.summary.apogee, refs[1].0.summary.landing_east, refs[1].0.summary.landing_north);
        for (rtol, atol) in [(1e-7, 1e-6), (1e-9, 1e-9), (1e-10, 1e-10)] {
            cfg.sim.rtol = rtol;
            cfg.sim.atol = atol;
            for i in [I::Rk4, I::Rk45, I::Dop853] {
                if i == I::Rk4 && rtol != 1e-7 {
                    continue;
                }
                for a in [A::Normalize, A::LieGroup] {
                    cfg.sim.integrator = i;
                    cfg.sim.attitude = a;
                    for (k, d) in [Descent::Parachute, Descent::Ballistic].into_iter().enumerate() {
                        match simulation(&cfg, &p).unwrap().run(d, false) {
                            Ok(r) => {
                                let t0 = std::time::Instant::now();
                                let _ = simulation(&cfg, &p).unwrap().run(d, false);
                                let wall = t0.elapsed();
                                let (s, rf) = (&r.summary, &refs[k].0.summary);
                                println!("tol {rtol:e}/{atol:e} {i:?}/{a:?} {d:?}: apogee {:.3} ({:+.3}) land E {:.2} N {:.2} (d {:.3}) evals {} wall {:.1} ms qerr {:.1e}", s.apogee, s.apogee - rf.apogee, s.landing_east, s.landing_north, (s.landing_east - rf.landing_east).hypot(s.landing_north - rf.landing_north), r.evals, wall.as_secs_f64() * 1e3, r.max_quat_norm_error);
                            }
                            Err(e) => println!("tol {rtol:e}/{atol:e} {i:?}/{a:?} {d:?}: FAILED {e}"),
                        }
                    }
                }
            }
        }
        }
        let _ = std::fs::remove_dir_all(&cfg.output.dir);
    }
}
