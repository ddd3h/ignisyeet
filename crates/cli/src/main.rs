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
use sim::flight::Stability;
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
    after_help = "Exit status: 0 success; 1 error (invalid configuration, missing input, failed step); 2 usage error; \
3 the run finished and wrote its outputs but the results are incomplete or not usable (a flight did not land within sim.max_time, dispersion cases/samples failed or did not land, or the rocket is statically UNSTABLE: smallest static margin below 0 cal, or its static stability could not be evaluated; see `stable` (null when unknown), `static_margin_min_cal` and `stability_unknown` in summary.json).\n\n\
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
    Geom {
        config: PathBuf,
        #[command(flatten)]
        ov: ConfigArgs,
    },
    /// Build the aerodynamic coefficient table (cached; --force rebuilds).
    Aero {
        config: PathBuf,
        /// Rebuild the table even when the cached aero_table.csv matches the configuration (no effect for aero.method = "table").
        #[arg(long)]
        force: bool,
        #[command(flatten)]
        ov: ConfigArgs,
        #[command(flatten)]
        table: TableArg,
    },
    /// Run one 6-DoF flight with the configured wind (writes trajectory.csv, summary.json). Exits with status 3 when the rocket did not land within sim.max_time, is statically unstable (negative static margin) or its static stability could not be evaluated.
    Sim {
        config: PathBuf,
        /// Override the descent mode.
        #[arg(long, value_parser = parse_descent)]
        descent: Option<Descent>,
        #[command(flatten)]
        ov: ConfigArgs,
        #[command(flatten)]
        table: TableArg,
    },
    /// Landing dispersion: wind speed x direction grid (dispersion.csv) or Monte Carlo (dispersion_mc.csv, dispersion_summary.json). Exits with status 3 when any case or sample failed or did not land, or the nominal flight is statically unstable or its static stability could not be evaluated.
    Dispersion {
        config: PathBuf,
        #[command(flatten)]
        ov: ConfigArgs,
        #[command(flatten)]
        table: TableArg,
    },
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

/// Command-line changes of the configuration file.
#[derive(clap::Args, Clone, Default)]
struct ConfigArgs {
    /// Change a configuration key for this run, e.g. --set wind.profile_mean_speed=15.6 or --set dispersion.mode=monte_carlo.
    /// KEY is the dotted TOML key path, VALUE a TOML value; a bare word that is not a TOML value is taken as a string (quote it, '"..."', when it would read as a number or boolean).
    /// Repeatable; applied before the configuration is checked (unknown keys and bad values are errors as in the file) and part of the config hash.
    /// Relative paths in values are resolved from the config file's directory.
    #[arg(long = "set", value_name = "KEY=VALUE")]
    set: Vec<String>,
    /// Write the outputs to DIR instead of output.dir; a relative DIR is resolved from the current directory.
    #[arg(long, value_name = "DIR")]
    out: Option<PathBuf>,
}

impl ConfigArgs {
    fn overrides(&self) -> config::Overrides {
        config::Overrides { set: self.set.clone(), out: self.out.clone() }
    }
}

/// An aero table to reuse from another output directory.
#[derive(clap::Args, Clone, Default)]
struct TableArg {
    /// Reuse the aero table PATH (an aero_table.csv, or a directory containing one) built in another output directory, e.g. by `ignisyeet aero`.
    /// It must have been built for the same rocket and aero settings (the same cache key as this configuration), otherwise the run stops with an error.
    /// The table, its JSON sidecar and the geometry files next to it are copied to the output directory. Not for aero.method = "table".
    #[arg(long, value_name = "PATH")]
    aero_table: Option<PathBuf>,
}

fn parse_descent(s: &str) -> Result<Descent, String> {
    match s {
        "ballistic" => Ok(Descent::Ballistic),
        "parachute" => Ok(Descent::Parachute),
        _ => Err("expected 'ballistic' or 'parachute'".into()),
    }
}

/// Loads the config with the command-line overrides, prints the loader warnings and applies its
/// `[resources]` (thread pools, priority, memory budget).
fn load_config(path: &Path, ov: &ConfigArgs, table: &TableArg) -> Result<Config> {
    let mut cfg = Config::load_with(path, &ov.overrides())?;
    cfg.aero_table_source = table.aero_table.clone();
    print_warnings(&cfg.load_warnings);
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

/// Start of the exit-status-3 message of results that must not be used (a statically unstable flight, aero
/// lookups beyond the table's angle range).
const NOT_USABLE: &str = "RESULTS NOT USABLE";

/// " at t = 3.10 s, Mach 0.85": where the smallest static margin of the flight occurred (empty when unknown).
fn margin_location(s: &Summary) -> String {
    if s.static_margin_min_t.is_finite() && s.static_margin_min_mach.is_finite() {
        format!(" at t = {:.2} s, Mach {:.2}", s.static_margin_min_t, s.static_margin_min_mach)
    } else {
        String::new()
    }
}

/// The exit-status-3 message of a statically unstable flight, or of one whose static stability could not be
/// evaluated (None when it is stable).
fn stability_reason(s: &Summary) -> Option<String> {
    match s.stability() {
        Stability::Stable => None,
        Stability::Unstable(m) => Some(format!("{NOT_USABLE}: the rocket is statically UNSTABLE (min static margin {m:.2} cal{})", margin_location(s))),
        Stability::Unknown(why) => Some(format!("{NOT_USABLE}: static stability could not be evaluated ({why})")),
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    ui::init(cli.quiet, cli.no_progress);
    let mut incomplete: Option<String> = None;
    match cli.cmd {
        Cmd::Geom { config, ov } => {
            let cfg = load_config(&config, &ov, &TableArg::default())?;
            begin(&cfg, &config, Sub::Geom);
            let step = ui().step("Extracting geometry");
            let g = geometry(&cfg)?;
            let files = write_geometry(&cfg, &g)?;
            step.done(&view::geometry_detail(&g));
            show_geometry(&g, true);
            if ui().plain() {
                for f in &files {
                    println!("  -> {}", f.display());
                }
            }
            let mut w = g.warnings.clone();
            w.extend(size_warnings(&cfg, g.length, 2.0 * g.ref_radius));
            print_warnings(&w);
        }
        Cmd::Aero { config, force, ov, table } => {
            let cfg = load_config(&config, &ov, &table)?;
            begin(&cfg, &config, Sub::Aero);
            aero_table_for(&cfg, force, Sub::Aero)?;
            print_warnings(&grid_warnings(&cfg));
        }
        Cmd::Sim { config, descent, ov, table } => {
            let cfg = load_config(&config, &ov, &table)?;
            check_descent_override(&cfg, descent)?;
            begin(&cfg, &config, Sub::Sim);
            incomplete = match run_sim(&cfg, descent) {
                Ok(i) => i,
                Err(e) => {
                    sim_failed(&cfg, &e);
                    return Err(e);
                }
            }
        }
        Cmd::Dispersion { config, ov, table } => {
            let cfg = load_config(&config, &ov, &table)?;
            begin(&cfg, &config, Sub::Dispersion);
            incomplete = run_dispersion(&cfg)?
        }
        Cmd::CfdCheck { config, mesh } => {
            let cfg = load_config(&config, &ConfigArgs::default(), &TableArg::default())?;
            return cfd_check(&cfg, &config, mesh);
        }
        Cmd::SampleStl { path } => {
            let tris: Vec<_> = geom::sample::SampleRocket::default().mesh().iter().map(|t| t.map(|v| geom::Vec3::new(v.y, v.z, -v.x) * 1000.0)).collect();
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
        if why.starts_with(NOT_USABLE) {
            eprintln!("ignisyeet: {why} (exit status {EXIT_INCOMPLETE})");
        } else {
            eprintln!("ignisyeet: INCOMPLETE RESULTS: {why} (exit status {EXIT_INCOMPLETE})");
        }
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

/// Warning about aero table lookups beyond the tabulated Mach range (held at the edge), if there were any.
/// Lookups beyond the angle range are not a warning but [`angle_range_reason`].
fn aero_range_warning(s: &aero::table::OutOfRangeStats) -> Option<String> {
    let mach = s.mach_below + s.mach_above;
    (mach > 0)
        .then(|| format!("aero table: {mach} of {} lookups beyond the Mach range (max Mach {:.2}); Mach is held at the table edge", s.lookups, s.max_mach))
}

/// The exit-status-3 message when aero lookups went beyond the table's angle range (`alpha_max` [deg], the
/// last angle of the table): those coefficients are extrapolated, which can create an artificial trim at a
/// large angle of attack and change a ballistic descent completely.
fn angle_range_reason(s: &aero::table::OutOfRangeStats, alpha_max: f64) -> Option<String> {
    (s.angle_beyond > 0).then(|| {
        let fix = if alpha_max < 90.0 { "build the table up to 90 deg" } else { "build the table up to 180 deg (aero.alpha_max_deg)" };
        format!(
            "{NOT_USABLE}: {} of {} lookups beyond the aero table's angle range (max |alpha| {:.1} deg > alpha_max {alpha_max:.1} deg): the descent uses extrapolated coefficients; {fix}",
            s.angle_beyond, s.lookups, s.max_abs_alpha_deg
        )
    })
}

/// Largest angle of attack [deg] of a table.
fn table_alpha_max(t: &aero::AeroTable) -> f64 {
    t.meta.alphas_deg.last().copied().unwrap_or(0.0)
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
        // False when lookups went beyond the angle range: the run exits with status 3 (RESULTS NOT USABLE).
        "angles_in_range": s.angle_beyond == 0,
    })
}

/// Plausible size range of a rocket body; outside it `rocket.stl_scale` is most likely wrong.
const LENGTH_RANGE: (f64, f64) = (0.1, 20.0);
const DIAMETER_RANGE: (f64, f64) = (0.005, 3.0);

/// Warnings about the size of the body (`length`, `diameter` in metres).
/// Mach step [-] and angle-of-attack step [deg] of the aero table above which the grid is reported as coarse.
const COARSE_MACH_STEP: f64 = 0.1;
const COARSE_ALPHA_STEP_DEG: f64 = 5.0;

/// Warnings for a coarse aero table grid (the coefficients are interpolated linearly between the rows); none
/// for an external table, whose grid is its own.
fn grid_warnings(cfg: &Config) -> Vec<String> {
    let a = &cfg.aero;
    let mut w = Vec::new();
    if a.method == config::AeroMethod::Table {
        return w;
    }
    let general_barrowman = a.table_kind == aero::table::TableKind::General && a.method == config::AeroMethod::Barrowman;
    let (key, step) = if general_barrowman { ("aero.general_mach_step", a.general_mach_step) } else { ("aero.mach_step", a.mach_step) };
    if step > COARSE_MACH_STEP {
        w.push(format!(
            "{key} = {step} is coarse (> {COARSE_MACH_STEP}): the coefficients are interpolated linearly between the Mach rows, which smooths over the transonic drag rise and shifts the apogee (0.02 is typical)"
        ));
    }
    if a.alpha_step_deg > COARSE_ALPHA_STEP_DEG {
        w.push(format!(
            "aero.alpha_step_deg = {} is coarse (> {COARSE_ALPHA_STEP_DEG} deg): the normal force and centre of pressure are interpolated linearly between the rows (1 deg is typical)",
            a.alpha_step_deg
        ));
    }
    w
}

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
/// Smallest static margin of the flight including the rail-exit value [cal] (NaN when none is finite).
pub(crate) fn static_margin_min(s: &Summary) -> f64 {
    s.static_margin_min()
}

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
    if let Stability::Unknown(why) = s.stability() {
        w.push(format!("static stability could not be evaluated ({why}): the results are not usable until it is"));
    }
    let m = s.static_margin_min();
    if m.is_finite() {
        let at = margin_location(s);
        if m < 0.0 {
            w.push(format!("static margin {m:.2} cal{at} is negative: the centre of pressure is forward of the CG, the rocket is UNSTABLE"));
        } else if m < 1.0 {
            w.push(format!("minimum static margin {m:.2} cal{at} is below 1.0 cal: low static margin"));
        } else if m < 1.5 {
            w.push(format!("minimum static margin {m:.2} cal{at} is below the recommended 1.5 cal (marginal)"));
        }
    }
    if s.descent == Descent::Ballistic && s.fell_broadside() {
        w.push(format!(
            "the ballistic descent fell broadside (angle of attack 60 to 120 deg for {:.0} s): the landing depends on the large-angle aerodynamics, which are not validated",
            s.broadside_time
        ));
    }
    w.extend(rail_exit_warnings_at(cfg, s, wind_speed));
    let rec = &cfg.recovery;
    if let (true, Some(_), Some(m)) = (rec.enabled, rec.main_cd_s, rec.main_altitude) {
        if s.apogee.is_finite() && m >= s.apogee {
            w.push(format!("recovery.main_altitude = {m} m is above the apogee ({:.0} m): the main opens together with the drogue", s.apogee));
        }
    }
    let wind = &cfg.wind;
    if wind.model == sim::WindModel::Profile {
        if let Some((bottom, top, _)) = wind_profile_extent(cfg) {
            if s.apogee > top {
                w.push(format!(
                    "apogee {:.0} m is above the top of the wind profile ({top:.0} m): the wind above {top:.0} m is held at the top value",
                    s.apogee
                ));
            }
            if bottom > 0.0 {
                w.push(format!("the wind profile starts at {bottom:.0} m: the wind below is held at the lowest row"));
            }
        }
    }
    w.extend(parachute_warnings(cfg, s));
    // the profile model takes its speed from the table, not from wind.speed
    let wind_speed = if wind.model == sim::WindModel::Profile { s.wind_speed } else { wind_speed };
    if matches!(wind.model, sim::WindModel::Power | sim::WindModel::Log) && wind_speed != 0.0 && s.apogee > WIND_VALID_HEIGHT {
        w.push(format!(
            "apogee {:.0} m is above {WIND_VALID_HEIGHT:.0} m, the validity height of the {:?} wind profile: the profile is extrapolated there, and wind.speed = {} m/s is the speed at wind.ref_height = {} m, so the winds aloft and the landing point are model-dependent",
            s.apogee, wind.model, wind.speed, wind.ref_height
        ));
    }
    w
}

/// Rail exit speed [m/s] below which the launch is reported as too slow.
const MIN_RAIL_EXIT_SPEED: f64 = 15.0;
/// Wind angle of attack [deg] at rail exit above which the launch is reported as wind-sensitive.
const MAX_RAIL_EXIT_WIND_ALPHA_DEG: f64 = 15.0;

/// Warnings about the rail exit of a flight: a speed below 15 m/s, or a wind angle of attack
/// atan(wind speed at the rail-exit height / rail exit speed) above 15 degrees (the configured wind).
#[cfg(test)]
fn rail_exit_warnings(cfg: &Config, s: &Summary) -> Vec<String> {
    rail_exit_warnings_at(cfg, s, cfg.wind.speed)
}

/// [`rail_exit_warnings`] for the reference wind speed `wind_speed` [m/s] instead of `wind.speed` (the largest
/// wind-grid speed, or the nominal speed plus two sigma for Monte Carlo).
fn rail_exit_warnings_at(cfg: &Config, s: &Summary, wind_speed: f64) -> Vec<String> {
    let mut w = Vec::new();
    let v = s.rail_exit_speed;
    if !(v.is_finite() && v > 0.0) {
        return w;
    }
    if v < MIN_RAIL_EXIT_SPEED {
        w.push(format!(
            "rail exit speed {v:.1} m/s is below {MIN_RAIL_EXIT_SPEED:.0} m/s: the fins have little authority at rail exit and the rocket is easily blown off the rail line; use a longer rail or a higher thrust-to-weight ratio"
        ));
    }
    let h = cfg.launch.rail_length * cfg.launch.elevation_deg.to_radians().sin();
    let mut wind_cfg = cfg.wind_config();
    if wind_speed != cfg.wind.speed {
        wind_cfg.speed = wind_speed;
    }
    if let Ok(wind) = sim::Wind::from_config(&wind_cfg) {
        let u = wind.at(h);
        let wind_speed = u.x.hypot(u.y);
        let alpha = wind_speed.atan2(v).to_degrees();
        if alpha > MAX_RAIL_EXIT_WIND_ALPHA_DEG {
            w.push(format!(
                "the wind angle of attack at rail exit is {alpha:.1} deg (wind {wind_speed:.1} m/s at {h:.1} m, rail exit speed {v:.1} m/s), above {MAX_RAIL_EXIT_WIND_ALPHA_DEG:.0} deg: the rocket will weathercock strongly off the rail"
            ));
        }
    }
    w
}

/// Landing speed [m/s] above which a parachute descent is reported as too fast.
const MAX_PARACHUTE_LANDING_SPEED: f64 = 15.0;

/// Warnings about the parachute of a parachute-descent run that landed: it never deployed before
/// landing, or the descent speed at landing is high.
fn parachute_warnings(cfg: &Config, s: &Summary) -> Vec<String> {
    let mut w = Vec::new();
    if !(cfg.recovery.enabled && s.descent == Descent::Parachute) {
        return w;
    }
    let dual = cfg.recovery.main_cd_s.is_some() && cfg.recovery.main_altitude.is_some();
    let first = if dual { "drogue" } else { "parachute" };
    w.extend(deploy_warning(first, s.drogue_deploy_speed, s.drogue_deploy_q, "recovery.delay"));
    if dual {
        w.extend(deploy_warning("main", s.main_deploy_speed, s.main_deploy_q, "recovery.main_altitude"));
    }
    if !s.landed() {
        return w;
    }
    if s.drogue_deploy_time.is_none() {
        w.push(format!(
            "parachute descent landed at {:.0} m/s: the parachute did not deploy before landing (recovery.delay = {} s)",
            s.landing_speed, cfg.recovery.delay
        ));
    } else if s.landing_speed > MAX_PARACHUTE_LANDING_SPEED {
        w.push(format!("parachute descent landed at {:.1} m/s, which is above {MAX_PARACHUTE_LANDING_SPEED:.0} m/s: check recovery.cd_s", s.landing_speed));
    }
    w
}

/// Airspeed [m/s] and dynamic pressure [Pa] at a parachute deployment above which the canopy is unlikely to
/// survive the opening (a typical hobby canopy is built for deployment near apogee, at a few hundred Pa).
const MAX_DEPLOY_SPEED: f64 = 50.0;
const MAX_DEPLOY_Q: f64 = 2000.0;

/// Warning when the `what` canopy ("parachute", "drogue" or "main") opens faster than [`MAX_DEPLOY_SPEED`]
/// or at a dynamic pressure above [`MAX_DEPLOY_Q`]; `key` is the setting that decides when it opens.
fn deploy_warning(what: &str, speed: Option<f64>, q: Option<f64>, key: &str) -> Option<String> {
    let (v, q) = (speed?, q?);
    (v > MAX_DEPLOY_SPEED || q > MAX_DEPLOY_Q).then(|| {
        format!("the {what} opens at {v:.0} m/s (q = {:.1} kPa): a canopy is unlikely to survive; the landing point assumes it does - check {key}", q / 1000.0)
    })
}

/// Warning when a wind-profile run (`what` names the study: "wind grid" or "Monte Carlo sample")
/// scaled the profile by more than 3 or, for a nonzero wind, by less than 1/3: the largest
/// deviation among `summaries`, with the resulting winds aloft.
fn wind_scale_warning<'a>(cfg: &Config, what: &str, summaries: impl IntoIterator<Item = &'a Summary>) -> Option<String> {
    let extreme = |k: f64| (k.ln()).abs();
    let worst = summaries
        .into_iter()
        .filter_map(|s| s.wind_scale.map(|k| (k, s.wind_speed)))
        .filter(|&(k, v)| k.is_finite() && k > 0.0 && (k > 3.0 || (k < 1.0 / 3.0 && v > 0.0)))
        .max_by(|a, b| extreme(a.0).total_cmp(&extreme(b.0)))?;
    let (k, v) = worst;
    let aloft = wind_profile_extent(cfg).map_or(String::new(), |(_, _, vmax)| format!(": winds aloft up to {:.0} m/s", vmax * k));
    // The run with the largest deviation from the profile, up or down.
    let factor = if k >= 1.0 {
        // Enough decimals that the printed factor visibly exceeds the limit of 3 (3.01, not 3.0).
        (1..=6).map(|d| format!("{k:.d$}")).find(|f| f.parse::<f64>().is_ok_and(|v| v > 3.0)).unwrap_or_else(|| format!("{k}"))
    } else {
        format!("{k:.2}")
    };
    Some(format!("wind profile scaled by a factor of {factor} ({what} {v:.1} m/s vs profile mean {:.1} m/s){aloft}", v / k))
}

/// Warning when flights of a wind-profile dispersion study (`what`: "wind-grid cases" or "Monte
/// Carlo samples") reach above the top of the profile table. `flights` gives a key per flight
/// (the rows of one wind-grid wind or one Monte Carlo sample share the ascent) and its apogee (NaN
/// for a failed run).
fn profile_top_warning(cfg: &Config, what: &str, flights: impl IntoIterator<Item = ((u64, u64), f64)>) -> Option<String> {
    if cfg.wind.model != sim::WindModel::Profile {
        return None;
    }
    let (_, top, _) = wind_profile_extent(cfg)?;
    let mut apogee: std::collections::HashMap<(u64, u64), f64> = std::collections::HashMap::new();
    for (k, a) in flights {
        let e = apogee.entry(k).or_insert(f64::NAN);
        *e = e.max(a);
    }
    let above: Vec<f64> = apogee.values().copied().filter(|&a| a > top).collect();
    let highest = above.iter().copied().fold(f64::NAN, f64::max);
    (!above.is_empty()).then(|| {
        format!(
            "{} of {} {what} reach above the top of the wind profile ({top:.0} m), up to {highest:.0} m: the wind above is held at the top value",
            above.len(),
            apogee.len()
        )
    })
}

/// (lowest altitude, top altitude, largest speed) of the wind profile table, when it can be read.
fn wind_profile_extent(cfg: &Config) -> Option<(f64, f64, f64)> {
    let path = cfg.resolve(cfg.wind.profile.as_deref()?);
    let text = std::fs::read_to_string(path).ok()?;
    let rows = sim::env::parse_wind_profile(strip_bom(&text)).ok()?;
    Some((rows.first()?.height, rows.last()?.height, rows.iter().map(|p| p.speed).fold(0.0, f64::max)))
}

/// `text` without a leading UTF-8 byte-order mark.
pub(crate) fn strip_bom(text: &str) -> &str {
    text.strip_prefix('\u{feff}').unwrap_or(text)
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

/// An error when the motor does not lie inside the body: its aft end `motor.aft_x` (measured aft of
/// the nose tip, default the body length) must be at least the motor length and at most the body length.
fn check_motor_position(cfg: &Config, length: f64, motor_length: f64) -> Result<()> {
    const TOL: f64 = 1e-3;
    let a = cfg.motor.aft_x.unwrap_or(length);
    if a - motor_length < -TOL || a > length + TOL {
        anyhow::bail!(
            "{} puts the motor outside the body: the motor is {motor_length:.4} m long and the body {length:.4} m, so motor.aft_x (the motor's aft end, measured aft of the nose tip) must be between {motor_length:.4} and {length:.4} m",
            if cfg.motor.aft_x.is_some() { format!("motor.aft_x = {a} m") } else { format!("motor.aft_x (default: the body length {length:.4} m)") }
        );
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
    if let Some(w) = g.fins.as_ref().and_then(|f| geom::fin_overhang_warning(f, g.length)) {
        g.warnings.push(w);
    }
    *GEOMETRY_WARNINGS.lock().unwrap() = Some(g.warnings.clone());
    check_length(cfg, g.length).and_then(|_| check_cg(cfg, g.length)).map_err(|e| with_geometry_warnings(e, &g.warnings))?;
    Ok(g)
}

/// `e` followed by the plausibility warnings of the geometry extraction, which often name the cause of a
/// failed check (an open or odd mesh) and would otherwise be lost with the error.
fn with_geometry_warnings(e: anyhow::Error, warnings: &[String]) -> anyhow::Error {
    if warnings.is_empty() {
        return e;
    }
    let list: String = warnings.iter().map(|w| format!("\n  geometry warning: {w}")).collect();
    anyhow::anyhow!("{e:#}{list}")
}

/// Writes geometry.json and profile.csv; returns their paths.
fn write_geometry(cfg: &Config, g: &Geometry) -> Result<Vec<PathBuf>> {
    // Body axes in STL coordinates, so inertia products, CG offsets and the thrust offset can be given in body axes.
    let mut json = serde_json::to_value(g)?;
    let [bx, by, bz] = g.frame.body_basis();
    if let Some(o) = json.as_object_mut() {
        for (k, v) in [("body_x", bx), ("body_y", by), ("body_z", bz)] {
            o.insert(k.into(), serde_json::json!([v.x, v.y, v.z]));
        }
    }
    let (json_path, csv_path) = (out_path(cfg, "geometry.json")?, out_path(cfg, "profile.csv")?);
    std::fs::write(&json_path, serde_json::to_string_pretty(&json)?)?;
    let mut f = std::io::BufWriter::new(std::fs::File::create(&csv_path)?);
    writeln!(f, "x,r,fin_span")?;
    for p in &g.profile {
        writeln!(f, "{:.6},{:.6},{:.6}", p.x, p.r, p.fin_span)?;
    }
    f.flush()?;
    Ok(vec![json_path, csv_path])
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

/// Mach step of the aero table: `aero.general_mach_step` for a general Barrowman table, `aero.mach_step` otherwise.
fn mach_step(cfg: &Config) -> f64 {
    if cfg.aero.table_kind == aero::table::TableKind::General && cfg.aero.method == config::AeroMethod::Barrowman {
        cfg.aero.general_mach_step
    } else {
        cfg.aero.mach_step
    }
}

fn aero_options(cfg: &Config) -> Result<AeroOptions> {
    let a = &cfg.aero;
    Ok(AeroOptions {
        mach_min: a.mach_min,
        mach_max: a.mach_max,
        mach_step: mach_step(cfg),
        alpha_max_deg: a.alpha_max_deg,
        alpha_step_deg: a.alpha_step_deg,
        reference_altitude: cfg.launch.altitude,
        atmosphere: cfg.atmosphere.build()?,
        roughness: cfg.rocket.roughness,
        fin_le_rounded: cfg.rocket.fin_le == "rounded",
        fin_te_square: cfg.rocket.fin_te == "square",
        extra_cd: cfg.rocket.extra_cd,
        nozzle_exit_area: std::f64::consts::PI * (cfg.motor.nozzle_exit_diameter / 2.0).powi(2),
    })
}

/// Version of the aerodynamic model and of the table layout. Bump it whenever a change of the code
/// alters the coefficients computed from the same inputs, so that older cached tables are rebuilt.
const MODEL_VERSION: u32 = 4;

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
        opt: aero_options(cfg)?,
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
    if let Some(t) = cfg.aero.table.as_deref().map(|p| cfg.resolve(p)) {
        // The JSON sidecar (reference area and diameter, length, extrapolation, ...) belongs to the table.
        files.push(t.with_extension("json"));
        files.push(t);
    }
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
                let model = AeroModel::new(g, aero_options(cfg)?);
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
            if cfg.aero_table_source.is_some() {
                anyhow::bail!("--aero-table cannot be used with aero.method = \"table\": the table is aero.table");
            }
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
    if let Some(src) = &cfg.aero_table_source {
        if force {
            anyhow::bail!("--aero-table and --force cannot be used together");
        }
        let t = adopt_aero_table(src, &path, &hash)?;
        if u.plain() {
            println!("Using aero table {} ({} rows, cache key {hash}) -> {}", src.display(), t.rows(), path.display());
        }
        u.done_line("Aero table", &format!("reused {} ({} rows)", ui::short_path(src), t.rows()), t_start.elapsed());
        show_table(&t, None, sub);
        return Ok(t);
    }
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
    let model = AeroModel::new(g, aero_options(cfg)?);
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
            off.reynolds,
            off.friction,
            off.nose,
            off.base,
            off.fins,
            off.boattail,
            off.extra,
            off.total(),
            on.total(),
            c.cna,
            c.xcp
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

/// Files of the aero step that are copied with a reused table (the ones that exist).
const AERO_SIDE_FILES: [&str; 6] = ["geometry.json", "profile.csv", "aero_drag.csv", "panel_cp.csv", "cfd_report.json", "cfd_cases.csv"];

/// `--aero-table SRC`: loads the table at SRC (a file or a directory with aero_table.csv), checks that
/// its cache key is `hash` and copies it with its sidecar and the aero side files to `dest`'s directory.
fn adopt_aero_table(src: &Path, dest: &Path, hash: &str) -> Result<AeroTable> {
    let src = if src.is_dir() { src.join("aero_table.csv") } else { src.to_path_buf() };
    let t = AeroTable::load(&src).with_context(|| format!("--aero-table: cannot load aero table {}", src.display()))?;
    if t.meta.source_hash != hash {
        anyhow::bail!(
            "--aero-table: {} was built for another rocket or other aero settings (cache key {} vs {hash} of this configuration); \
             build it for this configuration with `ignisyeet aero` or leave out --aero-table",
            src.display(),
            t.meta.source_hash
        );
    }
    let same = |a: &Path, b: &Path| matches!((a.canonicalize(), b.canonicalize()), (Ok(x), Ok(y)) if x == y);
    if same(&src, dest) {
        return Ok(t);
    }
    let (sdir, ddir) = (src.parent().unwrap_or(Path::new(".")), dest.parent().unwrap_or(Path::new(".")));
    let mut files: Vec<(PathBuf, PathBuf)> = vec![(src.with_extension("json"), dest.with_extension("json"))];
    files.extend(AERO_SIDE_FILES.iter().map(|n| (sdir.join(n), ddir.join(n))).filter(|(a, _)| a.exists()));
    for (a, b) in &files {
        std::fs::copy(a, b).with_context(|| format!("cannot copy {} to {}", a.display(), b.display()))?;
        ui().record(b);
    }
    // The CSV last: a run that stops halfway leaves no table without its sidecar.
    std::fs::copy(&src, dest).with_context(|| format!("cannot copy {} to {}", src.display(), dest.display()))?;
    Ok(t)
}

fn aero_grid_len(cfg: &Config) -> usize {
    let a = &cfg.aero;
    ((a.mach_max - a.mach_min.max(0.0)) / mach_step(cfg)).round().max(1.0) as usize + 1
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
    let built = cfd::build_table_with_progress(
        &g,
        &cfg.aero.cfd,
        &aero_options(cfg)?,
        &out_dir,
        hash,
        cfg.aero.extrapolation,
        &cfd_budget(cfg),
        raw.as_deref(),
        &progress,
    );
    pb.finish_and_clear();
    let (table, report) = built?;
    table.save(path)?;
    std::fs::write(out_path(cfg, "cfd_report.json")?, serde_json::to_string_pretty(&report)?)?;
    ui().record(&out_dir.join("cfd_cases.csv"));
    if u.plain() {
        println!(
            "Aero table: CFD, {} cases ({} failed, {} unconverged), {} cells, {:.1?} -> {}",
            report.cases,
            report.failed.len(),
            report.unconverged.len(),
            report.mesh_cells,
            start.elapsed(),
            path.display()
        );
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
            (None, v) => {
                println!("  {:<21}ok       {}  [{}]", t.name, v.as_deref().unwrap_or(""), t.path.as_ref().map(|p| p.display().to_string()).unwrap_or_default())
            }
            (Some(p), _) if t.name.starts_with("manifold3d") && opt.surface != cfd::SurfaceKind::Stl => {
                println!("  {:<21}not installed (only needed for surface = \"stl\")", t.name)
            }
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
        let m = cfd::mesh_only(&g, opt, &aero_options(cfg)?, &cfg.out_dir(), &cfd_budget(cfg), raw.as_deref())?;
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
    let built = panel::build_table_with_progress(&g, &cfg.aero.panel, &aero_options(cfg)?, hash, cfg.aero.extrapolation, &progress);
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
    check_cg(cfg, table.meta.length).map_err(|e| with_geometry_warnings(e, &geometry_warnings(cfg)))?;
    let motor = load_motor(cfg)?;
    check_motor_position(cfg, table.meta.length, motor.length)?;
    let motor_aft_x = cfg.motor.aft_x.unwrap_or(table.meta.length);
    let mut geometry_warnings = geometry_warnings(cfg);
    geometry_warnings.extend(motor.warnings());
    if cfg.aero.method == config::AeroMethod::Table {
        geometry_warnings.extend(table.sanity_warnings());
    }
    Ok(Prepared { table, motor, motor_aft_x, geometry_warnings })
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

/// An error for `--descent parachute` when the recovery is disabled.
fn check_descent_override(cfg: &Config, descent: Option<Descent>) -> Result<()> {
    if descent == Some(Descent::Parachute) && !cfg.recovery.enabled {
        anyhow::bail!("--descent parachute needs recovery.enabled = true");
    }
    Ok(())
}

fn default_descent(cfg: &Config) -> Descent {
    cfg.sim.descent.unwrap_or(if cfg.recovery.enabled { Descent::Parachute } else { Descent::Ballistic })
}

/// Rough terminal speed [m/s] of a rocket of `mass` [kg] under a canopy of drag area `cd_s` [m^2] (sea-level
/// density 1.225 kg/m^3 as the progress estimate and the plain summary use).
pub(crate) fn chute_speed(mass: f64, cd_s: f64) -> f64 {
    (2.0 * mass * 9.80665 / (1.225 * cd_s.max(1e-3))).sqrt()
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
    let v_chute = chute_speed(mass, cfg.recovery.main_cd_s.unwrap_or(cfg.recovery.cd_s));
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
/// Removes the trajectory.csv and summary.json of an earlier run of the output directory, so that a failed run
/// leaves no results that look like its own (plot.py would draw them).
fn remove_stale_flight_outputs(cfg: &Config) {
    for name in ["trajectory.csv", "summary.json"] {
        let _ = std::fs::remove_file(cfg.out_dir().join(name));
    }
}

/// Writes summary.json as the failure marker `{"failed": true, "error": ...}` after a failed `sim` run: the
/// plots report the failure instead of drawing an earlier trajectory.
fn write_failure_marker(cfg: &Config, e: &anyhow::Error) {
    let dir = cfg.out_dir();
    let _ = std::fs::create_dir_all(&dir);
    let json = serde_json::json!({ "failed": true, "error": format!("{e:#}") });
    let _ = std::fs::write(dir.join("summary.json"), serde_json::to_string_pretty(&json).unwrap_or_default());
}

/// File name of the KML document in the output directory.
const KML_FILE: &str = "ignisyeet.kml";

/// Outputs of a failed `sim` run: the failure marker in summary.json, and ignisyeet.kml (when it exists)
/// rewritten without the flight layer, so that neither shows the previous run's flight.
fn sim_failed(cfg: &Config, e: &anyhow::Error) {
    write_failure_marker(cfg, e);
    if cfg.output.kml && cfg.out_dir().join(KML_FILE).is_file() {
        if let Err(k) = write_kml(cfg, kml::Parts::default(), true) {
            // Do not leave the old flight behind when the file cannot be rewritten.
            let _ = std::fs::remove_file(cfg.out_dir().join(KML_FILE));
            if ui().mode != ui::Mode::Quiet {
                eprintln!("ignisyeet.kml removed: it could not be rewritten without the flight ({k:#})");
            }
        } else if ui().mode != ui::Mode::Quiet {
            println!("  KML written without the flight layer: the simulation failed");
        }
    }
}

fn run_sim(cfg: &Config, descent: Option<Descent>) -> Result<Option<String>> {
    let u = ui();
    remove_stale_flight_outputs(cfg);
    let p = prepare(cfg)?;
    let descent = descent.unwrap_or_else(|| default_descent(cfg));
    if u.rich() {
        u.done_line(
            "Rocket model",
            &format!("L {:.3} m {d} D {:.4} m {d} {} aero rows", p.table.meta.length, p.table.meta.ref_diameter, p.table.rows(), d = u.g().dot),
            std::time::Duration::ZERO,
        );
    }
    let t0 = std::time::Instant::now();
    let r = fly(cfg, &simulation(cfg, &p)?, descent)?;
    let general = p.table.kind() == aero::table::TableKind::General;
    u.done_line("Flight simulated", &format!("{descent:?}, {} samples, {} RHS evals", r.samples.len(), r.evals), t0.elapsed());
    let path = out_path(cfg, "trajectory.csv")?;
    write_trajectory(&path, &r.samples)?;
    let s = &r.summary;
    let landed = s.landed();
    let range = p.table.out_of_range_stats();
    let mut warnings = p.geometry_warnings.clone();
    warnings.extend(size_warnings(cfg, p.table.meta.length, p.table.meta.ref_diameter));
    warnings.extend(grid_warnings(cfg));
    warnings.extend(flight_warnings(cfg, s));
    warnings.extend(aero_range_warning(&range));
    let angle = angle_range_reason(&range, table_alpha_max(&p.table));
    let mut json = summary_json(cfg, s, &warnings)?;
    json["aero_table_range"] = aero_range_json(&range);
    let summary_path = out_path(cfg, "summary.json")?;
    std::fs::write(&summary_path, serde_json::to_string_pretty(&json)?)?;
    if u.plain() {
        println!("Flight ({descent:?}, wind {:.1} m/s from {:.0} deg)", s.wind_speed, s.wind_direction_deg);
        println!("  rail exit       {:.2} s, {:.1} m/s, stability {:.2} cal", s.rail_exit_time, s.rail_exit_speed, s.stability_at_rail_exit);
        println!("  max speed       {:.1} m/s (Mach {:.2}), max q {:.1} kPa", s.max_speed, s.max_mach, s.max_dyn_pressure / 1000.0);
        println!("  min static margin {:.2} cal{} (free flight {})", static_margin_min(s), margin_location(s), free_flight_margin_text(s));
        if general {
            println!(
                "  pitch / yaw     {:.2} cal / {:.2} cal (general aero table: smallest margin in each plane)",
                s.static_margin_pitch_min_cal, s.static_margin_yaw_min_cal
            );
        }
        println!("  apogee          {:.1} m at {:.2} s", s.apogee, s.apogee_time);
        if let (Some(t), Some(v), Some(q)) = (s.drogue_deploy_time, s.drogue_deploy_speed, s.drogue_deploy_q) {
            println!("  deploy          {t:.2} s, {v:.1} m/s, q {:.2} kPa", q / 1000.0);
        }
        if let (Some(t), Some(h)) = (s.main_deploy_time, s.main_deploy_altitude) {
            let at = match (s.main_deploy_speed, s.main_deploy_q) {
                (Some(v), Some(q)) => format!(", {v:.1} m/s, q {:.2} kPa", q / 1000.0),
                _ => String::new(),
            };
            println!("  main deploy     {h:.1} m at {t:.2} s{at}");
        }
        if landed {
            println!(
                "  landing         {:.1} s, E {:.1} m, N {:.1} m ({:.6}, {:.6}), {:.1} m/s",
                s.landing_time, s.landing_east, s.landing_north, s.landing_lat, s.landing_lon, s.landing_speed
            );
        } else {
            println!("  landing         NONE: the rocket did not reach the ground within sim.max_time = {} s", cfg.sim.max_time);
        }
        println!("  -> {}", path.display());
        println!("  -> {}", summary_path.display());
    } else if u.rich() {
        println!();
        u.print_panel(&view::flight_panel(s, descent, true));
    }
    print_warnings(&warnings);
    let unstable = stability_reason(s).or(angle);
    if landed {
        write_kml(cfg, kml::Parts { flight: Some(kml::Flight { samples: r.samples, summary: r.summary }), ..Default::default() }, false)?;
        Ok(unstable)
    } else {
        // The previous run's flight must not stay in the file: write it without the flight layer.
        if cfg.output.kml {
            write_kml(cfg, kml::Parts::default(), true)?;
            if u.mode != ui::Mode::Quiet {
                println!("  KML written without the flight layer: the flight has no landing point");
            }
        }
        Ok(unstable.or_else(|| Some(format!("the flight did not land within sim.max_time = {} s", cfg.sim.max_time))))
    }
}

/// The free-flight minimum static margin for the plain summary; it has none when the airspeed never
/// exceeded 10 m/s before apogee.
fn free_flight_margin_text(s: &Summary) -> String {
    if s.min_stability_cal.is_finite() {
        format!("{:.2} cal", s.min_stability_cal)
    } else {
        "n/a (airspeed < 10 m/s)".to_string()
    }
}

/// A finite number as JSON, null otherwise.
fn finite_json(v: f64) -> serde_json::Value {
    if v.is_finite() {
        v.into()
    } else {
        serde_json::Value::Null
    }
}

/// The nominal wind of a dispersion study for dispersion_meta.json and dispersion_summary.json:
/// `wind_model`, the nominal `wind_speed` and `wind_direction_deg` (for a profile its mean speed and
/// the direction of its mean vector) and, for a profile, the nominal `wind_scale` (factor applied
/// to the CSV speeds; the wind grid and the Monte Carlo draws scale from there).
fn wind_json(w: &sim::Wind) -> serde_json::Map<String, serde_json::Value> {
    let mut m = serde_json::Map::new();
    m.insert("wind_model".into(), serde_json::to_value(w.model).unwrap_or_default());
    m.insert("wind_speed".into(), finite_json(w.speed));
    m.insert("wind_direction_deg".into(), finite_json(w.direction_deg));
    if w.model == sim::WindModel::Profile {
        m.insert("wind_scale".into(), finite_json(w.profile_scale));
    }
    m
}

/// summary.json: the flight summary plus `landed`, the run's `config_hash` and the sanity `warnings`.
fn summary_json(cfg: &Config, s: &Summary, warnings: &[String]) -> Result<serde_json::Value> {
    let mut json = serde_json::to_value(s)?;
    json["landed"] = s.landed().into();
    // Smallest static margin of the flight, including the rail exit (the rail-exit value alone when
    // `min_stability_cal` is null because the airspeed never exceeded 10 m/s); `stable` is false when it is
    // negative and null when the static stability could not be evaluated (`stability_unknown` says why).
    let margin = s.static_margin_min();
    json["static_margin_min_cal"] = margin.is_finite().then_some(margin).map_or(serde_json::Value::Null, Into::into);
    json["stable"] = s.stability().as_bool().into();
    json["fell_broadside"] = s.fell_broadside().into();
    json["max_time"] = cfg.sim.max_time.into();
    // The plots take the altitude from `up` (flat Earth: the integrated height) or from `alt` (ECEF).
    json["earth_model"] = serde_json::to_value(cfg.earth.model)?;
    json["wind_model"] = serde_json::to_value(cfg.wind.model)?;
    json["config_hash"] = run_hash(cfg, RunKind::Flight).into();
    json["warnings"] = stored_warnings(cfg, warnings);
    Ok(json)
}

/// The `warnings` array of summary.json, dispersion_meta.json and dispersion_summary.json: the
/// findings of the configuration loader (keys without effect, repeated `--set` keys; printed when the
/// configuration is read) followed by the run's own `warnings`.
fn stored_warnings(cfg: &Config, warnings: &[String]) -> serde_json::Value {
    serde_json::json!(cfg.load_warnings.iter().chain(warnings).collect::<Vec<_>>())
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
    let path = out_path(cfg, KML_FILE)?;
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
/// Also returns the summary of the nominal ballistic flight (None when it failed).
fn nominal_warnings(cfg: &Config, p: &Prepared, base: &Simulation) -> (Vec<String>, Option<Summary>) {
    let mut w = p.geometry_warnings.clone();
    w.extend(size_warnings(cfg, p.table.meta.length, p.table.meta.ref_diameter));
    w.extend(grid_warnings(cfg));
    let ref_speed = match cfg.dispersion.mode {
        sim::DispersionMode::WindGrid => cfg.dispersion.wind_speeds.iter().copied().fold(0.0, f64::max),
        sim::DispersionMode::MonteCarlo => cfg.wind.speed + 2.0 * cfg.dispersion.monte_carlo.wind_speed,
    };
    let mut nominal = None;
    match base.run(Descent::Ballistic, false) {
        Ok(r) => {
            w.extend(flight_warnings_at(cfg, &r.summary, ref_speed));
            nominal = Some(r.summary);
        }
        Err(e) => w.push(format!("the nominal flight could not be simulated ({e:#}): stability and wind-model checks were skipped")),
    }
    if cfg.recovery.enabled {
        // The parachute descent is flown as well: it may never deploy or land too fast.
        if let Ok(r) = base.run(Descent::Parachute, false) {
            w.extend(parachute_warnings(cfg, &r.summary));
        }
    }
    (w, nominal)
}

/// `stable`, `static_margin_min_cal` and `stability_unknown` of the nominal flight for the dispersion JSON files
/// (`stable` null when the nominal flight failed or its static stability could not be evaluated).
fn stability_json(nominal: Option<&Summary>) -> serde_json::Value {
    let m = nominal.map(Summary::static_margin_min).filter(|m| m.is_finite());
    let st = nominal.map(Summary::stability);
    let unknown = match &st {
        Some(Stability::Unknown(why)) => Some(why.clone()),
        _ => None,
    };
    serde_json::json!({ "stable": st.and_then(|s| s.as_bool()), "static_margin_min_cal": m, "stability_unknown": unknown })
}

/// The exit-status-3 message of the nominal flight's static stability (None when it is stable or failed).
fn nominal_stability_reason(nominal: Option<&Summary>) -> Option<String> {
    nominal.and_then(stability_reason)
}

/// Static stability of the runs of one descent mode of a dispersion study.
struct Instability {
    mode: Descent,
    /// Runs of this mode (failed ones included).
    total: usize,
    /// Runs whose smallest static margin is negative.
    unstable: usize,
    /// Runs whose static stability could not be evaluated (and that are not unstable).
    unknown: usize,
    /// The reason of the first run with unknown stability.
    first_unknown: Option<String>,
    /// Smallest static margin over the runs [cal] (NaN when none has one).
    worst: f64,
    /// Ballistic runs that fell broadside, see [`Summary::fell_broadside`].
    broadside: usize,
}

/// Counts, per descent mode (in order of appearance), the runs with a negative static margin (the smaller of the
/// free-flight minimum and the margin at rail exit) and those whose static stability could not be evaluated. A run
/// without a summary (failed) counts in `total` only.
fn instability<'a>(runs: impl IntoIterator<Item = (Descent, Option<&'a Summary>)>) -> Vec<Instability> {
    let mut v: Vec<Instability> = Vec::new();
    for (mode, s) in runs {
        let i = v.iter().position(|x| x.mode == mode).unwrap_or_else(|| {
            v.push(Instability { mode, total: 0, unstable: 0, unknown: 0, first_unknown: None, worst: f64::NAN, broadside: 0 });
            v.len() - 1
        });
        let x = &mut v[i];
        x.total += 1;
        let Some(s) = s else { continue };
        if mode == Descent::Ballistic && s.fell_broadside() {
            x.broadside += 1;
        }
        let m = s.static_margin_min();
        if m.is_finite() {
            x.worst = if x.worst.is_finite() { x.worst.min(m) } else { m };
        }
        match s.stability() {
            Stability::Stable => {}
            Stability::Unstable(_) => x.unstable += 1,
            Stability::Unknown(why) => {
                x.unknown += 1;
                x.first_unknown.get_or_insert(why);
            }
        }
    }
    v
}

/// Whether a run is statically stable (smallest static margin >= 0); None when its static stability could not
/// be evaluated.
fn stable_flag(s: &Summary) -> Option<bool> {
    s.stability().as_bool()
}

/// `unstable` and `stability_unknown_runs` (counts) and `unstable_min_static_margin_cal` (smallest margin) of each
/// descent mode for the dispersion JSON files.
fn instability_json(v: &[Instability]) -> serde_json::Value {
    let mut unstable = serde_json::Map::new();
    let mut unknown = serde_json::Map::new();
    let mut worst = serde_json::Map::new();
    let mut broadside = serde_json::Map::new();
    for x in v {
        if x.mode == Descent::Ballistic {
            broadside.insert(descent_name(x.mode).into(), x.broadside.into());
        }
        unstable.insert(descent_name(x.mode).into(), x.unstable.into());
        unknown.insert(descent_name(x.mode).into(), x.unknown.into());
        worst.insert(descent_name(x.mode).into(), x.worst.is_finite().then_some(x.worst).map_or(serde_json::Value::Null, Into::into));
    }
    serde_json::json!({ "unstable": unstable, "stability_unknown_runs": unknown, "unstable_min_static_margin_cal": worst, "broadside_runs": broadside })
}

/// One clause per descent mode with runs whose static stability could not be evaluated.
fn unknown_clauses(v: &[Instability], what: &str) -> Vec<String> {
    v.iter()
        .filter(|x| x.unknown > 0)
        .map(|x| {
            let why = x.first_unknown.as_deref().unwrap_or("");
            format!("{} of {} {what} ({}): static stability could not be evaluated (first: {why})", x.unknown, x.total, descent_name(x.mode))
        })
        .collect()
}

/// One clause per descent mode with unstable runs: "N of M <what> (mode) are statically UNSTABLE (min static margin down to X cal)".
fn instability_clauses(v: &[Instability], what: &str) -> Vec<String> {
    v.iter()
        .filter(|x| x.unstable > 0)
        .map(|x| {
            format!("{} of {} {what} ({}) are statically UNSTABLE (min static margin down to {:.2} cal)", x.unstable, x.total, descent_name(x.mode), x.worst)
        })
        .collect()
}

/// Warnings for the unstable runs of a dispersion study (they stay in the statistics) and for the runs whose static
/// stability could not be evaluated.
fn instability_warnings(v: &[Instability], what: &str) -> Vec<String> {
    instability_clauses(v, what)
        .into_iter()
        .map(|c| format!("{c}: they are kept in the statistics (conservative), so the landing distances include tumbling flights"))
        .chain(unknown_clauses(v, what))
        .chain(v.iter().filter(|x| x.broadside > 0).map(|x| {
            format!(
                "{} of {} ballistic {what} fell broadside (angle of attack 60 to 120 deg for more than {:.0} s): the landing depends on the large-angle aerodynamics, which are not validated",
                x.broadside,
                x.total,
                sim::flight::BROADSIDE_LIMIT
            )
        }))
        .collect()
}

/// The exit-status-3 message for unstable runs and runs whose static stability could not be evaluated (None when
/// there are none).
fn instability_reason(v: &[Instability], what: &str, hint: &str) -> Option<String> {
    let c = instability_clauses(v, what);
    let u = unknown_clauses(v, what);
    if !c.is_empty() {
        let more = if u.is_empty() { String::new() } else { format!("; {}", u.join("; ")) };
        Some(format!("{NOT_USABLE}: {}: {hint}{more}", c.join("; ")))
    } else {
        (!u.is_empty()).then(|| format!("{NOT_USABLE}: {}", u.join("; ")))
    }
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
    let (mut warnings, nominal) = nominal_warnings(cfg, &p, &base);
    if cfg.dispersion.mode == sim::DispersionMode::MonteCarlo {
        return run_monte_carlo(cfg, &p, &base, warnings, nominal.as_ref());
    }
    let start = std::time::Instant::now();
    let total = sim::dispersion::wind_grid_runs(&cfg.dispersion, cfg.recovery.enabled);
    let (cases, detailed) = if u.progress_on() {
        let pb = u.bar_with(total as u64, "Wind grid", DISP_T, rate_key);
        let far = FarthestLanding::new();
        let r = sim::dispersion::run_wind_grid_detailed(&cfg.dispersion, &base, cfg.recovery.enabled, &|c, s| {
            pb.set_message(far.update(c.descent, s.landing_distance));
            pb.inc(1);
        });
        pb.finish_and_clear();
        r?
    } else {
        sim::dispersion::run_wind_grid_detailed(&cfg.dispersion, &base, cfg.recovery.enabled, &|_, _| {})?
    };
    let (results, errors) = split_wind_grid(&cases, detailed);
    let mut modes: Vec<Descent> = Vec::new();
    for c in &cases {
        if !modes.contains(&c.descent) {
            modes.push(c.descent);
        }
    }
    let failed = results.iter().filter(|s| !s.landed()).count();
    let errored = errors.iter().flatten().count();
    let not_landed = failed - errored;
    if not_landed > 0 {
        warnings.push(format!(
            "{not_landed} of {} wind-grid cases did not land within sim.max_time = {} s: they are excluded from the landing distances, so the maximum landing distance is only a lower bound",
            results.len(),
            cfg.sim.max_time
        ));
    }
    if let Some(first) = errors.iter().flatten().next() {
        warnings.push(format!(
            "{errored} of {} wind-grid cases failed with an error (e.g. {first}): they are excluded from the landing distances, so the maximum landing distance is only a lower bound",
            results.len()
        ));
    }
    warnings.extend(wind_grid_error_warnings(&cases, &errors));
    warnings.extend(wind_scale_warning(cfg, "wind grid", results.iter()));
    warnings.extend(profile_top_warning(
        cfg,
        "wind-grid cases",
        cases
            .iter()
            .zip(&results)
            .zip(&errors)
            .map(|((c, s), e)| ((c.wind_speed.to_bits(), c.wind_direction_deg.to_bits()), if e.is_none() { s.apogee } else { f64::NAN })),
    ));
    if errored > 0 {
        // Rebuild the lookup statistics from the cases that completed: a diverged state is not a flight.
        p.table.reset_out_of_range_stats();
        sim::dispersion::replay_wind_grid_ok(&base, &cases, &errors);
    }
    warnings.extend(aero_range_warning(&p.table.out_of_range_stats()));
    let angle = angle_range_reason(&p.table.out_of_range_stats(), table_alpha_max(&p.table));
    let instab = instability(cases.iter().zip(&results).zip(&errors).map(|((c, s), e)| (c.descent, e.is_none().then_some(s))));
    warnings.extend(instability_warnings(&instab, "wind-grid cases"));
    let path = out_path(cfg, "dispersion.csv")?;
    write_wind_grid_csv(&path, &results, &errors)?;
    let mut meta = serde_json::json!({ "config_hash": run_hash(cfg, RunKind::WindGrid), "flight_config_hash": run_hash(cfg, RunKind::Flight), "cases": results.len(), "failed": failed, "errored": errored, "warnings": stored_warnings(cfg, &warnings), "aero_table_range": aero_range_json(&p.table.out_of_range_stats()) });
    meta.as_object_mut().unwrap().extend(stability_json(nominal.as_ref()).as_object().unwrap().clone());
    meta.as_object_mut().unwrap().extend(instability_json(&instab).as_object().unwrap().clone());
    meta.as_object_mut().unwrap().extend(wind_json(&base.wind));
    let meta_path = out_path(cfg, kml::WIND_GRID_META)?;
    std::fs::write(&meta_path, serde_json::to_string_pretty(&meta)?)?;
    u.done_line("Wind grid", &format!("{} cases, {failed} failed", results.len()), start.elapsed());
    if u.plain() {
        println!("Dispersion: {} cases in {:.2?} -> {}, {}", results.len(), start.elapsed(), path.display(), meta_path.display());
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
    let incomplete = nominal_stability_reason(nominal.as_ref())
        .or(angle)
        .or_else(|| instability_reason(&instab, "wind-grid cases", "increase the static margin"))
        .or_else(|| (failed > 0).then(|| format!("{failed} of {} wind-grid cases did not land", results.len())));
    if errored == results.len() {
        anyhow::bail!(
            "all {errored} wind-grid cases failed with an error (first: {}); the results are in {}",
            errors.iter().flatten().next().map_or("", |e| e.as_str()),
            path.display()
        );
    }
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

/// Splits the per-case results of the wind grid into summaries (a failed case becomes [`Summary::failed`],
/// which never counts as landed) and the error message of each failed case.
fn split_wind_grid(cases: &[sim::dispersion::Case], detailed: Vec<std::result::Result<Summary, String>>) -> (Vec<Summary>, Vec<Option<String>>) {
    cases
        .iter()
        .zip(detailed)
        .map(|(c, r)| match r {
            Ok(s) => (s, None),
            Err(e) => (Summary::failed(c.descent, c.wind_speed, c.wind_direction_deg), Some(e)),
        })
        .unzip()
}

/// Warnings naming the first failed wind-grid cases with their error (the rest are counted).
fn wind_grid_error_warnings(cases: &[sim::dispersion::Case], errors: &[Option<String>]) -> Vec<String> {
    const SHOWN: usize = 3;
    let failed: Vec<(&sim::dispersion::Case, &String)> = cases.iter().zip(errors).filter_map(|(c, e)| e.as_ref().map(|e| (c, e))).collect();
    let mut w: Vec<String> = failed
        .iter()
        .take(SHOWN)
        .map(|(c, e)| format!("wind-grid case {} m/s from {:.0}\u{b0} ({}): {e}", c.wind_speed, c.wind_direction_deg, descent_name(c.descent)))
        .collect();
    if failed.len() > SHOWN {
        w.push(format!("{} more wind-grid cases failed with an error (see the error column of dispersion.csv)", failed.len() - SHOWN));
    }
    w
}

/// A CSV field kept on one line (line breaks become spaces), quoted when it holds a comma or quote.
fn csv_field(s: &str) -> String {
    let s = s.replace(['\n', '\r'], " ");
    if s.contains([',', '"']) {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s
    }
}

fn write_wind_grid_csv(path: &Path, results: &[sim::Summary], errors: &[Option<String>]) -> Result<()> {
    let mut f = std::io::BufWriter::new(std::fs::File::create(path)?);
    writeln!(f, "descent,wind_speed,wind_direction_deg,landing_east,landing_north,landing_lat,landing_lon,landing_distance,landing_time,landing_speed,apogee,apogee_time,max_mach,rail_exit_speed,min_stability_cal,landed,error,stable")?;
    for (s, e) in results.iter().zip(errors) {
        let mode = if s.descent == Descent::Ballistic { "ballistic" } else { "parachute" };
        writeln!(
            f,
            "{mode},{},{},{:.2},{:.2},{:.8},{:.8},{:.2},{:.2},{:.2},{:.2},{:.2},{:.4},{:.2},{:.3},{},{},{}",
            s.wind_speed,
            s.wind_direction_deg,
            s.landing_east,
            s.landing_north,
            s.landing_lat,
            s.landing_lon,
            s.landing_distance,
            s.landing_time,
            s.landing_speed,
            s.apogee,
            s.apogee_time,
            s.max_mach,
            s.rail_exit_speed,
            s.min_stability_cal,
            s.landed(),
            csv_field(e.as_deref().unwrap_or("")),
            stable_flag(s).map_or(String::new(), |b| b.to_string())
        )?;
    }
    f.flush()?;
    Ok(())
}

/// Landed samples per descent mode below which the Monte Carlo statistics are reported as rough.
const MIN_LANDED_SAMPLES: usize = 30;

fn descent_name(d: Descent) -> &'static str {
    if d == Descent::Ballistic {
        "ballistic"
    } else {
        "parachute"
    }
}

fn run_monte_carlo(cfg: &Config, p: &Prepared, base: &Simulation, mut warnings: Vec<String>, nominal: Option<&Summary>) -> Result<Option<String>> {
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
    warnings.extend(wind_scale_warning(cfg, "Monte Carlo sample", r.rows.iter().filter_map(|row| row.result.as_ref().ok())));
    warnings.extend(profile_top_warning(
        cfg,
        "Monte Carlo samples",
        r.rows.iter().map(|row| ((row.sample as u64, 0), row.result.as_ref().map_or(f64::NAN, |s| s.apogee))),
    ));
    if r.rows.iter().any(|x| x.result.as_ref().err().is_some_and(|e| e != sim::dispersion::NOT_LANDED)) {
        // Rebuild the lookup statistics from the runs that completed: a diverged state is not a flight.
        p.table.reset_out_of_range_stats();
        // Samples that completed without landing are replayed as well (as for the wind grid).
        let completed: Vec<sim::dispersion::McRow> = r
            .rows
            .iter()
            .filter(|x| x.result.as_ref().err().is_none_or(|e| e == sim::dispersion::NOT_LANDED))
            .map(|x| sim::dispersion::McRow { result: Ok(x.result.clone().unwrap_or_else(|_| Summary::failed(x.descent, f64::NAN, f64::NAN))), ..x.clone() })
            .collect();
        sim::dispersion::replay_monte_carlo_ok(base, &completed);
    }
    let instab = instability(r.rows.iter().map(|x| (x.descent, x.result.as_ref().ok())));
    warnings.extend(instability_warnings(&instab, "Monte Carlo samples"));
    for d in r.summary.descents.iter().filter(|d| d.landing.n > 0 && d.landing.n < MIN_LANDED_SAMPLES) {
        warnings.push(format!("{:?}: only {} landed samples: the statistics are rough", d.descent, d.landing.n));
    }
    let range = p.table.out_of_range_stats();
    warnings.extend(aero_range_warning(&range));
    let angle = angle_range_reason(&range, table_alpha_max(&p.table));
    let path = out_path(cfg, "dispersion_mc.csv")?;
    write_monte_carlo_csv(&path, &r.rows)?;
    let spath = out_path(cfg, "dispersion_summary.json")?;
    let mut json = serde_json::to_value(&r.summary)?;
    json["config_hash"] = run_hash(cfg, RunKind::MonteCarlo).into();
    // Lets the plots check that this run belongs to the same flight configuration as summary.json.
    json["flight_config_hash"] = run_hash(cfg, RunKind::Flight).into();
    json["warnings"] = stored_warnings(cfg, &warnings);
    json["aero_table_range"] = aero_range_json(&range);
    json.as_object_mut().unwrap().extend(stability_json(nominal).as_object().unwrap().clone());
    json.as_object_mut().unwrap().extend(instability_json(&instab).as_object().unwrap().clone());
    json.as_object_mut().unwrap().extend(wind_json(&base.wind));
    std::fs::write(&spath, serde_json::to_string_pretty(&json)?)?;
    u.done_line("Monte Carlo", &format!("{} samples x {} descent modes", r.summary.samples, r.summary.descents.len()), start.elapsed());
    if u.plain() {
        println!(
            "Monte Carlo: {} samples x {} descent modes in {:.2?} -> {}, {}",
            r.summary.samples,
            r.summary.descents.len(),
            start.elapsed(),
            path.display(),
            spath.display()
        );
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
            let apogee_std = if d.apogee_std.is_finite() { format!("{:.1} m", d.apogee_std) } else { "n/a".to_string() };
            println!("    mean landing E {:.1} m, N {:.1} m; apogee {:.1} +- {apogee_std}", d.landing.mean_east, d.landing.mean_north, d.apogee_mean);
            if d.landing.n < 2 {
                println!("    1-sigma and 3-sigma ellipses: {}", view::ELLIPSE_NA);
            } else {
                println!(
                    "    1-sigma ellipse {:.1} x {:.1} m, 3-sigma {:.1} x {:.1} m, major axis bearing {:.0} deg",
                    e1.semi_major, e1.semi_minor, e3.semi_major, e3.semi_minor, e1.major_axis_bearing_deg
                );
            }
            if d.max_landing_distance_is_lower_bound {
                println!(
                    "    max landing distance \u{2265} {:.1} m ({} samples did not land or failed; sample {} is the farthest that landed)",
                    d.max_landing_distance,
                    d.failed,
                    d.max_landing_distance_sample.unwrap_or(0)
                );
            } else {
                println!("    max landing distance {:.1} m (sample {})", d.max_landing_distance, d.max_landing_distance_sample.unwrap_or(0));
            }
            println!(
                "    landing distance P50 {:.1} m, P90 {:.1} m, P99 {:.1} m (landed samples)",
                d.landing_distance_p50, d.landing_distance_p90, d.landing_distance_p99
            );
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
        write_kml(
            cfg,
            kml::Parts { monte_carlo: Some(kml::MonteCarlo { rows: r.rows.clone(), stats: r.summary.descents.clone() }), ..Default::default() },
            false,
        )?;
    }
    for row in r.rows.iter().filter(|x| x.result.is_err()).take(3) {
        let msg = format!("failed: sample {} ({}): {}", row.sample, descent_name(row.descent), row.result.as_ref().unwrap_err());
        match u.mode {
            ui::Mode::Plain => println!("  {msg}"),
            ui::Mode::Rich => u.warn(msg),
            ui::Mode::Quiet => eprintln!("{msg}"),
        }
    }
    Ok(nominal_stability_reason(nominal)
        .or(angle)
        .or_else(|| instability_reason(&instab, "Monte Carlo samples", "increase the static margin or reduce the CG/aero sigmas"))
        .or_else(|| (failed > 0).then(|| format!("{failed} Monte Carlo runs did not land or failed; the statistics are incomplete"))))
}

fn write_monte_carlo_csv(path: &Path, rows: &[sim::dispersion::McRow]) -> Result<()> {
    let mut f = std::io::BufWriter::new(std::fs::File::create(path)?);
    writeln!(
        f,
        "sample,descent,status,thrust_scale,burn_time_scale,dry_mass,cg,cn_scale,ca_scale,elevation_deg,azimuth_deg,roll_deg,wind_speed_delta,wind_direction_deg_delta,parachute_cd_s_scale,\
wind_speed,wind_direction_deg,landing_east,landing_north,landing_lat,landing_lon,landing_distance,landing_time,landing_speed,apogee,apogee_time,max_mach,rail_exit_speed,min_stability_cal,\
thrust_misalignment_y_deg,thrust_misalignment_z_deg,thrust_offset_y,thrust_offset_z,fin_cant_deg,cg_lateral_y,cg_lateral_z,cl_scale,clp_scale,stable"
    )?;
    for row in rows {
        let p = &row.perturbation;
        write!(
            f,
            "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},",
            row.sample,
            descent_name(row.descent),
            if row.result.is_ok() { "ok" } else { "failed" },
            p.thrust_scale,
            p.burn_time_scale,
            p.dry_mass,
            p.cg,
            p.cn_scale,
            p.ca_scale,
            p.elevation_deg,
            p.azimuth_deg,
            p.roll_deg,
            p.wind_speed,
            p.wind_direction_deg,
            p.parachute_cd_s_scale
        )?;
        match &row.result {
            Ok(s) => write!(
                f,
                "{},{},{:.2},{:.2},{:.8},{:.8},{:.2},{:.2},{:.2},{:.2},{:.2},{:.4},{:.2},{:.3}",
                s.wind_speed,
                s.wind_direction_deg,
                s.landing_east,
                s.landing_north,
                s.landing_lat,
                s.landing_lon,
                s.landing_distance,
                s.landing_time,
                s.landing_speed,
                s.apogee,
                s.apogee_time,
                s.max_mach,
                s.rail_exit_speed,
                s.min_stability_cal
            )?,
            Err(_) => write!(f, ",,,,,,,,,,,,,")?,
        }
        writeln!(
            f,
            ",{},{},{},{},{},{},{},{},{},{}",
            p.misalignment_y_deg,
            p.misalignment_z_deg,
            p.offset_y,
            p.offset_z,
            p.fin_cant_deg,
            p.cg_lateral_y,
            p.cg_lateral_z,
            p.cl_scale,
            p.clp_scale,
            row.result.as_ref().ok().and_then(stable_flag).map_or(String::new(), |b| b.to_string())
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
        let meta = aero::table::TableMeta {
            source_hash: "ext".into(),
            ref_area: 0.01,
            ref_diameter: 0.1,
            length: 1.0,
            machs: vec![0.0, 1.0],
            alphas_deg: vec![0.0, 10.0],
            ..Default::default()
        };
        let csv = tmp.join("legacy.csv");
        AeroTable::from_fn(meta, |_, a| aero::AeroCoeffs { cn: a, ca_off: 0.5, xcp: 0.6, ..Default::default() }).save(&csv).unwrap();
        let text =
            include_str!("../../../examples/sample.toml").replace("method = \"barrowman\"", &format!("method = \"table\"\ntable = \"{}\"", csv.display()));
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
            rows.push(sim::dispersion::McRow {
                sample: 0,
                descent: Descent::Ballistic,
                perturbation: sim::dispersion::Perturbation { cl_scale: 0.25, roll_deg: 1.5, ..Default::default() },
                result,
            });
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
        assert_eq!(col, lines[0].len() - 3);
        assert_eq!(lines[0][lines[0].len() - 1], "stable");
        assert_eq!(lines[1][col], "0.25");
        // The roll change of the rail tilt follows the azimuth.
        let roll = lines[0].iter().position(|c| *c == "roll_deg").unwrap();
        assert_eq!((lines[0][roll - 1], lines[1][roll]), ("azimuth_deg", "1.5"));
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
            static_margin_pitch_min_cal: 2.0,
            static_margin_yaw_min_cal: 2.0,
            static_margin_min_t: 1.0,
            static_margin_min_mach: 0.3,
            stability_unknown: None,
            broadside_time: 0.0,
            landing_time: 30.0,
            landing_east: 10.0,
            landing_north: 20.0,
            landing_lat: 35.0,
            landing_lon: 139.0,
            landing_distance: 22.0,
            landing_speed: 30.0,
            main_deploy_time: None,
            main_deploy_altitude: None,
            drogue_deploy_time: None,
            drogue_deploy_speed: None,
            drogue_deploy_q: None,
            main_deploy_speed: None,
            main_deploy_q: None,
            wind_scale: None,
        }
    }

    fn summary_with_margin(margin: f64) -> sim::Summary {
        sim::Summary { min_stability_cal: margin, stability_at_rail_exit: 2.0, ..sample_summary() }
    }

    #[test]
    fn unstable_samples_are_counted_per_mode_and_make_the_run_unusable() {
        let ok = summary_with_margin(0.5);
        let bad = summary_with_margin(-0.28);
        let slightly = summary_with_margin(-0.1);
        let runs = vec![
            (Descent::Ballistic, Some(&ok)),
            (Descent::Ballistic, Some(&bad)),
            (Descent::Ballistic, None),
            (Descent::Ballistic, Some(&slightly)),
            (Descent::Parachute, Some(&ok)),
        ];
        let v = instability(runs);
        assert_eq!((v[0].total, v[0].unstable), (4, 2));
        assert_eq!((v[1].total, v[1].unstable), (1, 0));
        let j = instability_json(&v);
        assert_eq!(j["unstable"]["ballistic"], 2);
        assert_eq!(j["unstable"]["parachute"], 0);
        assert_eq!(j["unstable_min_static_margin_cal"]["ballistic"], -0.28);
        let msg = instability_reason(&v, "Monte Carlo samples", "increase the static margin or reduce the CG/aero sigmas").unwrap();
        assert_eq!(
            msg,
            "RESULTS NOT USABLE: 2 of 4 Monte Carlo samples (ballistic) are statically UNSTABLE (min static margin down to -0.28 cal): increase the static margin or reduce the CG/aero sigmas"
        );
        let w = instability_warnings(&v, "Monte Carlo samples");
        assert!(
            w.len() == 1 && w[0].starts_with("2 of 4 Monte Carlo samples (ballistic) are statically UNSTABLE") && w[0].contains("kept in the statistics"),
            "{w:?}"
        );
        assert!(instability_reason(&instability(vec![(Descent::Ballistic, Some(&ok))]), "x", "h").is_none());
        // the rail-exit margin counts as well
        let rail = sim::Summary { min_stability_cal: 1.0, stability_at_rail_exit: -0.2, ..sample_summary() };
        assert_eq!(stable_flag(&rail), Some(false));
        assert_eq!(stable_flag(&ok), Some(true));
    }

    #[test]
    fn dispersion_csv_files_carry_a_stable_column() {
        let results = [summary_with_margin(0.5), summary_with_margin(-0.3), sim::Summary::failed(Descent::Ballistic, 1.0, 0.0)];
        let errors = [None, None, Some("boom".to_string())];
        let path = std::env::temp_dir().join(format!("ignisyeet_grid_stable_{}.csv", std::process::id()));
        write_wind_grid_csv(&path, &results, &errors).unwrap();
        let text = std::fs::read_to_string(&path).unwrap();
        let last: Vec<&str> = text.lines().map(|l| l.rsplit(',').next().unwrap()).collect();
        assert_eq!(last, ["stable", "true", "false", ""]);
        let mc: Vec<sim::dispersion::McRow> = [Ok(summary_with_margin(0.5)), Ok(summary_with_margin(-0.3)), Err("x".to_string())]
            .into_iter()
            .map(|result| sim::dispersion::McRow { sample: 0, descent: Descent::Ballistic, perturbation: Default::default(), result })
            .collect();
        write_monte_carlo_csv(&path, &mc).unwrap();
        let text = std::fs::read_to_string(&path).unwrap();
        let _ = std::fs::remove_file(&path);
        let last: Vec<&str> = text.lines().map(|l| l.rsplit(',').next().unwrap()).collect();
        assert_eq!(last, ["stable", "true", "false", ""]);
    }

    #[test]
    fn run_hash_covers_the_aero_table_sidecar() {
        let dir = std::env::temp_dir().join(format!("ignisyeet_cli_sidecar_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let meta = aero::table::TableMeta {
            source_hash: "ext".into(),
            ref_area: 0.01,
            ref_diameter: 0.1,
            length: 1.0,
            machs: vec![0.0, 1.0],
            alphas_deg: vec![0.0, 10.0],
            ..Default::default()
        };
        let csv = dir.join("t.csv");
        AeroTable::from_fn(meta, |_, a| aero::AeroCoeffs { cn: a, ca_off: 0.5, xcp: 0.6, ..Default::default() }).save(&csv).unwrap();
        let text = include_str!("../../../examples/sample.toml").replace("method = \"barrowman\"", "method = \"table\"\ntable = \"t.csv\"");
        let cfg = Config::from_value(toml::from_str(&text).unwrap(), Some(&dir)).unwrap();
        let before = run_hash(&cfg, RunKind::Flight);
        let json = std::fs::read_to_string(csv.with_extension("json")).unwrap();
        assert!(json.contains("\"ref_area\": 0.01"), "{json}");
        std::fs::write(csv.with_extension("json"), json.replace("\"ref_area\": 0.01", "\"ref_area\": 0.02")).unwrap();
        assert_ne!(before, run_hash(&cfg, RunKind::Flight));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn default_motor_position_is_checked_against_the_body() {
        let cfg = load_sample(include_str!("../../../examples/sample.toml"));
        assert!(cfg.motor.aft_x.is_none());
        check_motor_position(&cfg, 1.0, 0.5).unwrap();
        let e = check_motor_position(&cfg, 0.4, 0.5).unwrap_err().to_string();
        assert!(e.starts_with("motor.aft_x (default: the body length 0.4000 m) puts the motor outside the body"), "{e}");
    }

    #[test]
    fn rail_exit_wind_warning_uses_the_reference_speed_of_the_study() {
        let mut cfg = load_sample(include_str!("../../../examples/sample.toml"));
        cfg.wind.model = sim::WindModel::Constant;
        cfg.wind.speed = 2.0;
        let ok = sim::Summary { rail_exit_speed: 20.0, ..summary_with_margin(2.0) };
        assert!(rail_exit_warnings_at(&cfg, &ok, 2.0).is_empty());
        // 7 m/s against 20 m/s: atan(0.35) = 19.3 deg
        let w = rail_exit_warnings_at(&cfg, &ok, 7.0);
        assert!(w.len() == 1 && w[0].contains("wind angle of attack at rail exit is 19.3 deg (wind 7.0 m/s"), "{w:?}");
        assert!(flight_warnings_at(&cfg, &ok, 7.0).iter().any(|w| w.contains("wind angle of attack at rail exit")));
        assert!(!flight_warnings(&cfg, &ok).iter().any(|w| w.contains("wind angle of attack")));
    }

    #[test]
    fn a_failed_sim_leaves_a_failure_marker_not_the_old_results() {
        let dir = std::env::temp_dir().join(format!("ignisyeet_cli_stale_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("trajectory.csv"), "old").unwrap();
        std::fs::write(dir.join("summary.json"), "{}").unwrap();
        let mut cfg = load_sample(include_str!("../../../examples/sample.toml"));
        cfg.output.dir = dir.clone();
        remove_stale_flight_outputs(&cfg);
        assert!(!dir.join("trajectory.csv").exists() && !dir.join("summary.json").exists());
        write_failure_marker(&cfg, &anyhow::anyhow!("state diverged"));
        let j: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(dir.join("summary.json")).unwrap()).unwrap();
        assert_eq!(j, serde_json::json!({ "failed": true, "error": "state diverged" }));
        let _ = std::fs::remove_dir_all(&dir);
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
        for resource in [
            "prefix = \"/opt/su2\"",
            "su2 = \"SU2_CFD2\"",
            "mpi = \"mpiexec\"",
            "ranks_per_case = 3",
            "parallel_cases = 2",
            "timeout_minutes = 5.0",
            "workdir = \"elsewhere\"",
        ] {
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
        sim::Summary {
            landing_time: f64::NAN,
            landing_east: f64::NAN,
            landing_north: f64::NAN,
            landing_lat: f64::NAN,
            landing_lon: f64::NAN,
            landing_distance: f64::NAN,
            landing_speed: f64::NAN,
            ..s
        }
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
    fn motor_must_lie_inside_the_body() {
        let mut cfg = sample_config("aftx");
        let len = prepare(&cfg).unwrap().table.meta.length;
        assert!(prepare(&cfg).is_ok());
        cfg.motor.aft_x = Some(len);
        assert!(prepare(&cfg).is_ok());
        cfg.motor.aft_x = Some(0.2);
        let e = prepare(&cfg).err().unwrap().to_string();
        assert!(e.contains("motor.aft_x = 0.2 m") && e.contains("outside the body") && e.contains(&format!("{len:.4}")), "{e}");
        cfg.motor.aft_x = Some(10.0);
        assert!(prepare(&cfg).err().unwrap().to_string().contains("motor.aft_x = 10 m"));
        let _ = std::fs::remove_dir_all(&cfg.output.dir);
    }

    #[test]
    fn unstable_flight_is_not_usable() {
        let reason = |m: f64| stability_reason(&summary_with_margin(m));
        assert!(reason(1.5).is_none() && reason(0.0).is_none());
        let m = reason(-0.234).unwrap();
        assert_eq!(m, "RESULTS NOT USABLE: the rocket is statically UNSTABLE (min static margin -0.23 cal at t = 1.00 s, Mach 0.30)");
        // without a recorded location the message has none
        let s = sim::Summary { static_margin_min_t: f64::NAN, ..summary_with_margin(-0.234) };
        assert_eq!(stability_reason(&s).unwrap(), "RESULTS NOT USABLE: the rocket is statically UNSTABLE (min static margin -0.23 cal)");
        let w = flight_warnings(&load_sample(include_str!("../../../examples/sample.toml")), &summary_with_margin(-0.234));
        assert!(w.iter().any(|w| w.starts_with("static margin -0.23 cal at t = 1.00 s, Mach 0.30 is negative")), "{w:?}");
        let j = stability_json(Some(&summary_with_margin(-0.5)));
        assert_eq!((j["stable"].as_bool(), j["static_margin_min_cal"].as_f64()), (Some(false), Some(-0.5)));
        assert_eq!(stability_json(Some(&summary_with_margin(2.0)))["stable"].as_bool(), Some(true));
        assert!(stability_json(None)["stable"].is_null());
        // end to end: the sample stays usable, a CG far aft does not
        let mut cfg = sample_config("unstable");
        assert!(run_sim(&cfg, Some(Descent::Parachute)).unwrap().is_none());
        cfg.rocket.cg_dry = prepare(&cfg).unwrap().table.meta.length * 0.97;
        let why = run_sim(&cfg, Some(Descent::Parachute)).unwrap().expect("unstable flight must be incomplete");
        assert!(why.starts_with("RESULTS NOT USABLE") && why.contains("UNSTABLE"), "{why}");
        let (_, nominal) = nominal_warnings(&cfg, &prepare(&cfg).unwrap(), &simulation(&cfg, &prepare(&cfg).unwrap()).unwrap());
        assert!(nominal.unwrap().static_margin_min() < 0.0);
        let _ = std::fs::remove_dir_all(&cfg.output.dir);
    }

    #[test]
    fn parachute_override_needs_enabled_recovery() {
        let mut cfg = sample_config("chute_override");
        assert!(check_descent_override(&cfg, Some(Descent::Parachute)).is_ok());
        cfg.recovery.enabled = false;
        assert_eq!(check_descent_override(&cfg, Some(Descent::Parachute)).unwrap_err().to_string(), "--descent parachute needs recovery.enabled = true");
        assert!(check_descent_override(&cfg, Some(Descent::Ballistic)).is_ok() && check_descent_override(&cfg, None).is_ok());
    }

    #[test]
    fn free_flight_margin_without_fast_flight_is_not_a_number() {
        let s = sim::Summary { min_stability_cal: f64::NAN, ..sample_summary() };
        assert_eq!(free_flight_margin_text(&s), "n/a (airspeed < 10 m/s)");
        assert_eq!(free_flight_margin_text(&sim::Summary { min_stability_cal: 1.234, ..sample_summary() }), "1.23 cal");
    }

    #[test]
    fn aero_force_flag_has_help() {
        use clap::CommandFactory;
        let mut cmd = Cli::command();
        let aero = cmd.find_subcommand_mut("aero").unwrap();
        let force = aero.get_arguments().find(|a| a.get_id() == "force").unwrap();
        assert!(force.get_help().is_some_and(|h| h.to_string().starts_with("Rebuild the table")));
    }

    #[test]
    fn coarse_aero_grids_warn() {
        let mut cfg = load_sample(include_str!("../../../examples/sample.toml"));
        assert!(grid_warnings(&cfg).is_empty());
        cfg.aero.mach_step = 3.0;
        cfg.aero.alpha_step_deg = 89.0;
        let w = grid_warnings(&cfg);
        assert_eq!(w.len(), 2, "{w:?}");
        assert!(w[0].starts_with("aero.mach_step = 3 is coarse (> 0.1)") && w[1].starts_with("aero.alpha_step_deg = 89 is coarse (> 5 deg)"), "{w:?}");
        // A general Barrowman table is built with general_mach_step.
        cfg.aero.table_kind = aero::table::TableKind::General;
        cfg.aero.general_mach_step = 0.2;
        assert!(grid_warnings(&cfg)[0].starts_with("aero.general_mach_step = 0.2 is coarse"));
        cfg.aero.general_mach_step = 0.05;
        cfg.aero.alpha_step_deg = 1.0;
        assert!(grid_warnings(&cfg).is_empty());
        // An external table has its own grid.
        cfg.aero.method = config::AeroMethod::Table;
        cfg.aero.alpha_step_deg = 89.0;
        assert!(grid_warnings(&cfg).is_empty());
    }

    #[test]
    fn geometry_outputs_are_listed() {
        let cfg = sample_config("geom_files");
        let files = write_geometry(&cfg, &geometry(&cfg).unwrap()).unwrap();
        let names: Vec<_> = files.iter().map(|f| f.file_name().unwrap().to_string_lossy().into_owned()).collect();
        assert_eq!(names, ["geometry.json", "profile.csv"]);
        assert!(files.iter().all(|f| f.is_file()));
        let _ = std::fs::remove_dir_all(&cfg.output.dir);
    }

    #[test]
    fn ballistic_descent_beyond_the_table_angles_is_not_usable() {
        // Sample, ballistic, 4 m/s from the east: the descent turns broadside (alpha ~ 90 deg).
        let mut cfg = sample_config("angle_range");
        cfg.wind.speed = 4.0;
        cfg.wind.direction_deg = 90.0;
        cfg.output.kml = false;
        let summary =
            |cfg: &Config| -> serde_json::Value { serde_json::from_str(&std::fs::read_to_string(cfg.out_dir().join("summary.json")).unwrap()).unwrap() };
        // Table up to 90 deg (the default): every lookup inside the table.
        assert_eq!(run_sim(&cfg, Some(Descent::Ballistic)).unwrap(), None);
        let j = summary(&cfg);
        assert_eq!((j["aero_table_range"]["angle_beyond"].as_u64(), j["aero_table_range"]["angles_in_range"].as_bool()), (Some(0), Some(true)));
        let ok_distance = j["landing_distance"].as_f64().unwrap();
        // Table up to 30 deg: the extrapolated coefficients trim at a large angle; the run is not usable.
        cfg.aero.alpha_max_deg = 30.0;
        let why = run_sim(&cfg, Some(Descent::Ballistic)).unwrap().expect("lookups beyond the angle range must be reported");
        assert!(
            why.starts_with("RESULTS NOT USABLE: ") && why.contains("lookups beyond the aero table's angle range") && why.contains("> alpha_max 30.0 deg"),
            "{why}"
        );
        assert!(why.ends_with("the descent uses extrapolated coefficients; build the table up to 90 deg"), "{why}");
        let j = summary(&cfg);
        assert_eq!(j["aero_table_range"]["angles_in_range"].as_bool(), Some(false));
        assert!(j["landing_distance"].as_f64().unwrap() > 2.0 * ok_distance, "the extrapolated descent drifts far: {j}");
        let _ = std::fs::remove_dir_all(&cfg.output.dir);
    }

    #[test]
    fn general_table_unstable_in_yaw_is_not_usable() {
        // The sample's table as a general table, once as it is and once with the yaw centre of
        // pressure moved to 20 % of the length (forward of the CG); pitch is unchanged.
        let mut cfg = sample_config("yaw_unstable");
        let t = prepare(&cfg).unwrap().table;
        let (alphas, betas) = general_axes();
        let g = t.to_general(alphas, betas);
        let (d, x_yaw) = (t.meta.ref_diameter, 0.2 * t.meta.length);
        let bad = AeroTable::general_from_fn(g.meta.clone(), |m, a, b| {
            let mut c = g.lookup6(m, a.to_radians(), b.to_radians());
            c.cn = -x_yaw * c.cy / d;
            c
        });
        std::fs::create_dir_all(&cfg.output.dir).unwrap();
        let (good_path, bad_path) = (cfg.output.dir.join("good_table.csv"), cfg.output.dir.join("bad_table.csv"));
        g.save(&good_path).unwrap();
        bad.save(&bad_path).unwrap();
        cfg.aero.method = config::AeroMethod::Table;
        cfg.aero.table = Some(good_path);
        assert!(run_sim(&cfg, Some(Descent::Ballistic)).unwrap().is_none());
        let j: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(cfg.out_dir().join("summary.json")).unwrap()).unwrap();
        let (pitch, yaw) = (j["static_margin_pitch_min_cal"].as_f64().unwrap(), j["static_margin_yaw_min_cal"].as_f64().unwrap());
        assert!(pitch > 1.0 && (pitch - yaw).abs() < 1e-9 && j["stable"].as_bool() == Some(true), "{pitch} {yaw}");
        cfg.aero.table = Some(bad_path);
        let why = run_sim(&cfg, Some(Descent::Ballistic)).unwrap().expect("a yaw-unstable rocket must be reported");
        assert!(why.starts_with("RESULTS NOT USABLE") && why.contains("UNSTABLE"), "{why}");
        let j: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(cfg.out_dir().join("summary.json")).unwrap()).unwrap();
        assert!(j["static_margin_pitch_min_cal"].as_f64().unwrap() > 1.0 && j["static_margin_yaw_min_cal"].as_f64().unwrap() < 0.0, "{j}");
        assert_eq!(j["stable"].as_bool(), Some(false));
        // The dispersion verdict of the nominal flight as well.
        let p = prepare(&cfg).unwrap();
        let (_, nominal) = nominal_warnings(&cfg, &p, &simulation(&cfg, &p).unwrap());
        let nominal = nominal.unwrap();
        assert!(nominal.static_margin_yaw_min_cal < 0.0 && matches!(nominal.stability(), Stability::Unstable(_)), "{nominal:?}");
        let _ = std::fs::remove_dir_all(&cfg.output.dir);
    }

    #[test]
    fn broadside_ballistic_falls_are_warned_and_counted() {
        let cfg = load_sample(include_str!("../../../examples/sample.toml"));
        let flat = sample_summary();
        let fell = sim::Summary { broadside_time: 145.5, ..sample_summary() };
        let short = sim::Summary { broadside_time: 9.0, ..sample_summary() };
        let broadside = |s: &sim::Summary| flight_warnings(&cfg, s).into_iter().filter(|w| w.contains("broadside")).collect::<Vec<_>>();
        assert!(broadside(&flat).is_empty() && broadside(&short).is_empty());
        assert_eq!(
            broadside(&fell),
            ["the ballistic descent fell broadside (angle of attack 60 to 120 deg for 146 s): the landing depends on the large-angle aerodynamics, which are not validated"]
        );
        // the parachute descent is not checked
        assert!(broadside(&sim::Summary { descent: Descent::Parachute, ..fell.clone() }).is_empty());
        let para = sim::Summary { descent: Descent::Parachute, ..sample_summary() };
        let v = instability(vec![
            (Descent::Ballistic, Some(&fell)),
            (Descent::Ballistic, Some(&short)),
            (Descent::Ballistic, None),
            (Descent::Parachute, Some(&para)),
        ]);
        let w = instability_warnings(&v, "wind-grid cases");
        assert_eq!(
            w,
            ["1 of 3 ballistic wind-grid cases fell broadside (angle of attack 60 to 120 deg for more than 10 s): the landing depends on the large-angle aerodynamics, which are not validated"]
        );
        // it does not make the results unusable
        assert!(instability_reason(&v, "wind-grid cases", "h").is_none());
        let j = instability_json(&v);
        assert_eq!(j["broadside_runs"], serde_json::json!({ "ballistic": 1 }));
    }

    #[test]
    fn unknown_static_stability_is_not_usable() {
        let why = "at t = 0.50 s: the yaw normal-force slope dCy/dbeta is zero at Mach 0.10";
        let s = sim::Summary { stability_unknown: Some(why.into()), static_margin_yaw_min_cal: f64::NAN, ..sample_summary() };
        assert_eq!(s.stability(), Stability::Unknown(why.into()));
        assert_eq!(stability_reason(&s).unwrap(), format!("RESULTS NOT USABLE: static stability could not be evaluated ({why})"));
        assert_eq!(stable_flag(&s), None);
        let cfg = load_sample(include_str!("../../../examples/sample.toml"));
        let j = summary_json(&cfg, &s, &[]).unwrap();
        assert!(j["stable"].is_null() && j["static_margin_yaw_min_cal"].is_null() && j["stability_unknown"] == why, "{j}");
        assert!(flight_warnings(&cfg, &s).iter().any(|w| w.starts_with(&format!("static stability could not be evaluated ({why})"))));
        let n = stability_json(Some(&s));
        assert!(n["stable"].is_null() && n["stability_unknown"] == why && n["static_margin_min_cal"] == 2.0, "{n}");
        // A known negative margin is unstable even when another margin is unknown.
        let both = sim::Summary { stability_at_rail_exit: -0.5, ..s.clone() };
        assert_eq!(both.stability(), Stability::Unstable(-0.5));
        // Counts: unknown runs are counted separately and make the run unusable; failed runs are not counted.
        let ok = summary_with_margin(1.5);
        let v = instability(vec![(Descent::Ballistic, Some(&ok)), (Descent::Ballistic, Some(&s)), (Descent::Ballistic, None), (Descent::Parachute, Some(&ok))]);
        assert_eq!((v[0].total, v[0].unstable, v[0].unknown), (3, 0, 1));
        let j = instability_json(&v);
        assert_eq!((j["stability_unknown_runs"]["ballistic"].as_u64(), j["stability_unknown_runs"]["parachute"].as_u64()), (Some(1), Some(0)));
        assert_eq!(
            instability_reason(&v, "wind-grid cases", "increase the static margin").unwrap(),
            format!("RESULTS NOT USABLE: 1 of 3 wind-grid cases (ballistic): static stability could not be evaluated (first: {why})")
        );
        assert!(instability_warnings(&v, "wind-grid cases").iter().any(|w| w.contains("static stability could not be evaluated")));
        // Unstable and unknown runs together: both clauses.
        let bad = summary_with_margin(-0.3);
        let v = instability(vec![(Descent::Ballistic, Some(&bad)), (Descent::Ballistic, Some(&s))]);
        let msg = instability_reason(&v, "Monte Carlo samples", "hint").unwrap();
        assert!(
            msg.contains("1 of 2 Monte Carlo samples (ballistic) are statically UNSTABLE")
                && msg.contains("hint; 1 of 2")
                && msg.contains("could not be evaluated"),
            "{msg}"
        );
    }

    #[test]
    fn general_table_without_side_force_has_unknown_stability() {
        // The sample's table as a general table without any side force: the yaw margin cannot be evaluated.
        let mut cfg = sample_config("yaw_unknown");
        let t = prepare(&cfg).unwrap().table;
        let (alphas, betas) = general_axes();
        let g = t.to_general(alphas, betas);
        let flat = AeroTable::general_from_fn(g.meta.clone(), |m, a, b| {
            let mut c = g.lookup6(m, a.to_radians(), b.to_radians());
            (c.cy, c.cn) = (0.0, 0.0);
            c
        });
        std::fs::create_dir_all(&cfg.output.dir).unwrap();
        let path = cfg.output.dir.join("flat_table.csv");
        flat.save(&path).unwrap();
        cfg.aero.method = config::AeroMethod::Table;
        cfg.aero.table = Some(path);
        let why = run_sim(&cfg, Some(Descent::Ballistic)).unwrap().expect("unknown stability must make the run unusable");
        assert!(
            why.starts_with("RESULTS NOT USABLE: static stability could not be evaluated (at t = ") && why.contains("yaw normal-force slope dCy/dbeta is zero"),
            "{why}"
        );
        let j: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(cfg.out_dir().join("summary.json")).unwrap()).unwrap();
        assert!(j["stable"].is_null() && j["static_margin_yaw_min_cal"].is_null() && j["static_margin_pitch_min_cal"].as_f64().unwrap() > 1.0, "{j}");
        assert!(j["stability_unknown"].as_str().unwrap().contains("dCy/dbeta"));
        let _ = std::fs::remove_dir_all(&cfg.output.dir);
    }

    #[test]
    fn dual_deploy_main_above_apogee_warns() {
        let mut cfg = load_sample(include_str!("../../../examples/sample.toml"));
        let ok = sim::Summary { min_stability_cal: 2.0, stability_at_rail_exit: 2.0, apogee: 500.0, ..sample_summary() };
        cfg.recovery.enabled = true;
        cfg.recovery.main_cd_s = Some(8.0);
        cfg.recovery.main_altitude = Some(300.0);
        assert!(!flight_warnings(&cfg, &ok).iter().any(|w| w.contains("main_altitude")));
        cfg.recovery.main_altitude = Some(500.0);
        let w = flight_warnings(&cfg, &ok);
        assert!(w.iter().any(|w| w == "recovery.main_altitude = 500 m is above the apogee (500 m): the main opens together with the drogue"), "{w:?}");
    }

    #[test]
    fn rail_exit_warnings_for_speed_and_wind_angle() {
        let mut cfg = load_sample(include_str!("../../../examples/sample.toml"));
        let ok = sim::Summary { min_stability_cal: 2.0, stability_at_rail_exit: 2.0, apogee: 500.0, ..sample_summary() };
        assert!(rail_exit_warnings(&cfg, &ok).is_empty());
        let slow = sim::Summary { rail_exit_speed: 12.0, ..ok.clone() };
        let w = rail_exit_warnings(&cfg, &slow);
        assert!(w.iter().any(|w| w.starts_with("rail exit speed 12.0 m/s is below 15 m/s")), "{w:?}");
        // 8 m/s of wind at every height against 20 m/s: atan(0.4) = 21.8 deg
        cfg.wind.model = sim::WindModel::Constant;
        cfg.wind.speed = 8.0;
        let w = rail_exit_warnings(&cfg, &ok);
        assert!(w.len() == 1 && w[0].contains("wind angle of attack at rail exit is 21.8 deg") && w[0].contains("above 15 deg"), "{w:?}");
        cfg.wind.speed = 5.0; // 14.0 deg
        assert!(rail_exit_warnings(&cfg, &ok).is_empty());
    }

    #[test]
    fn aero_options_reject_an_invalid_atmosphere() {
        let mut cfg = load_sample(include_str!("../../../examples/sample.toml"));
        assert!(aero_options(&cfg).is_ok());
        cfg.atmosphere.model = sim::env::AtmosphereKind::Constant;
        cfg.atmosphere.density = 0.0;
        let e = aero_options(&cfg).unwrap_err().to_string();
        assert!(e.starts_with("atmosphere.density must be finite and positive"), "{e}");
    }

    #[test]
    fn summary_json_records_landing_hash_and_warnings() {
        let cfg = load_sample(include_str!("../../../examples/sample.toml"));
        let j = summary_json(&cfg, &sample_summary(), &["w1".to_string()]).unwrap();
        assert_eq!(j["landed"], true);
        assert_eq!(j["warnings"][0], "w1");
        // the loader's warnings come first
        let mut profile = cfg.clone();
        profile.load_warnings = vec!["wind.speed is ignored for wind.model = \"profile\"".into()];
        let j = summary_json(&profile, &sample_summary(), &["w1".to_string()]).unwrap();
        assert_eq!(j["warnings"], serde_json::json!(["wind.speed is ignored for wind.model = \"profile\"", "w1"]));
        assert_eq!(j["config_hash"], run_hash(&cfg, RunKind::Flight));
        assert_eq!(j["earth_model"], "flat");
        let mut ecef = cfg.clone();
        ecef.earth.model = sim::EarthModel::Ecef;
        assert_eq!(summary_json(&ecef, &sample_summary(), &[]).unwrap()["earth_model"], "ecef");
        let j = summary_json(&cfg, &unlanded(sample_summary()), &[]).unwrap();
        assert_eq!(j["landed"], false);
        assert!(j["landing_east"].is_null() && j["max_time"] == cfg.sim.max_time);
    }

    #[test]
    fn aero_range_warning_and_json() {
        let none = aero::table::OutOfRangeStats { lookups: 10, ..Default::default() };
        assert!(aero_range_warning(&none).is_none());
        assert_eq!(aero_range_json(&none)["out_of_range"], false);
        assert!(angle_range_reason(&none, 30.0).is_none());
        let s = aero::table::OutOfRangeStats { lookups: 100, mach_above: 7, angle_beyond: 3, max_mach: 2.5, max_abs_alpha_deg: 31.0, ..Default::default() };
        let w = aero_range_warning(&s).unwrap();
        assert_eq!(w, "aero table: 7 of 100 lookups beyond the Mach range (max Mach 2.50); Mach is held at the table edge");
        // Angles beyond the table: results not usable (exit status 3), not a warning.
        let r = angle_range_reason(&s, 30.0).unwrap();
        assert_eq!(
            r,
            "RESULTS NOT USABLE: 3 of 100 lookups beyond the aero table's angle range (max |alpha| 31.0 deg > alpha_max 30.0 deg): the descent uses extrapolated coefficients; build the table up to 90 deg"
        );
        assert!(r.starts_with(NOT_USABLE));
        assert!(aero_range_warning(&aero::table::OutOfRangeStats { mach_above: 0, ..s }).is_none());
        let wide = aero::table::OutOfRangeStats { max_abs_alpha_deg: 120.0, ..s };
        assert!(angle_range_reason(&wide, 90.0).unwrap().ends_with("build the table up to 180 deg (aero.alpha_max_deg)"));
        let j = aero_range_json(&s);
        assert_eq!((j["mach_above"].as_u64(), j["angle_beyond"].as_u64(), j["out_of_range"].as_bool()), (Some(7), Some(3), Some(true)));
        assert_eq!((j["angles_in_range"].as_bool(), aero_range_json(&none)["angles_in_range"].as_bool()), (Some(false), Some(true)));
    }

    #[test]
    fn failed_geometry_checks_keep_the_geometry_warnings() {
        let e = anyhow::anyhow!("rocket.cg_dry = 1 m is outside the body");
        assert_eq!(with_geometry_warnings(e, &[]).to_string(), "rocket.cg_dry = 1 m is outside the body");
        let e = with_geometry_warnings(anyhow::anyhow!("rocket.cg_dry = 1 m is outside the body"), &["w1".into(), "w2".into()]);
        assert_eq!(e.to_string(), "rocket.cg_dry = 1 m is outside the body\n  geometry warning: w1\n  geometry warning: w2");
        // through geometry(): a mesh with a few open edges and a CG behind the body
        let dir = std::env::temp_dir().join(format!("ignisyeet_geom_warn_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let mesh = geom::sample::SampleRocket::default().mesh();
        geom::stl::write_stl_binary(&dir.join("r.stl"), &mesh[1..]).unwrap();
        let mut cfg = load_sample(include_str!("../../../examples/sample.toml"));
        cfg.rocket.stl = dir.join("r.stl");
        cfg.rocket.stl_scale = 1.0;
        cfg.rocket.cg_dry = 5.0;
        let e = geometry(&cfg).unwrap_err().to_string();
        assert!(e.starts_with("rocket.cg_dry = 5 m is outside the body") && e.contains("\n  geometry warning: the STL surface has 3 open edges of"), "{e}");
        let _ = std::fs::remove_dir_all(&dir);
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
    fn dispersion_warning_when_flights_reach_above_the_profile_top() {
        let mut cfg = load_sample(include_str!("../../../examples/sample.toml"));
        let flights = [((0, 0), 5000.0), ((0, 0), 5000.0), ((1, 0), 6500.0), ((1, 0), 6400.0), ((2, 0), f64::NAN), ((3, 0), 7200.0)];
        // power model: no profile, no warning
        assert!(profile_top_warning(&cfg, "Monte Carlo samples", flights).is_none());
        cfg.wind.model = sim::WindModel::Profile;
        cfg.wind.profile = Some("wind_profile_example.csv".into());
        let (_, top, _) = wind_profile_extent(&cfg).unwrap();
        assert_eq!(
            profile_top_warning(&cfg, "Monte Carlo samples", flights).unwrap(),
            format!("2 of 4 Monte Carlo samples reach above the top of the wind profile ({top:.0} m), up to 7200 m: the wind above is held at the top value")
        );
        assert!(profile_top_warning(&cfg, "wind-grid cases", [((0, 0), 100.0)]).is_none());
    }

    #[test]
    fn profile_warning_when_apogee_is_above_the_table_top() {
        let mut cfg = load_sample(include_str!("../../../examples/sample.toml"));
        cfg.wind.model = sim::WindModel::Profile;
        cfg.wind.profile = Some("wind_profile_example.csv".into());
        let base = sim::Summary { min_stability_cal: 2.0, stability_at_rail_exit: 2.0, wind_speed: 4.0, ..sample_summary() };
        let low = sim::Summary { apogee: 4440.7, ..base.clone() };
        assert!(flight_warnings(&cfg, &low).is_empty());
        let high = sim::Summary { apogee: 7000.0, ..base };
        let w = flight_warnings(&cfg, &high);
        assert_eq!(w, vec!["apogee 7000 m is above the top of the wind profile (6000 m): the wind above 6000 m is held at the top value".to_string()]);
        // the extrapolation warning is gated on the effective speed of the table, not on wind.speed
        cfg.wind.model = sim::WindModel::Power;
        cfg.wind.speed = 0.0;
        assert!(flight_warnings(&cfg, &high).is_empty());
        assert_eq!(strip_bom("\u{feff}a"), "a");
    }

    #[test]
    fn profile_warning_when_the_table_starts_above_the_ground() {
        let mut cfg = load_sample(include_str!("../../../examples/sample.toml"));
        let dir = std::env::temp_dir().join(format!("ignisyeet_wprof_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("w.csv"), "altitude_m,speed,direction_deg\n150,3,270\n6000,10,270\n").unwrap();
        cfg.wind.model = sim::WindModel::Profile;
        cfg.wind.profile = Some(dir.join("w.csv"));
        let s = sim::Summary { min_stability_cal: 2.0, stability_at_rail_exit: 2.0, apogee: 500.0, ..sample_summary() };
        let w = flight_warnings(&cfg, &s);
        assert_eq!(w, vec!["the wind profile starts at 150 m: the wind below is held at the lowest row".to_string()]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn wind_scale_warning_texts() {
        let mut cfg = load_sample(include_str!("../../../examples/sample.toml"));
        let dir = std::env::temp_dir().join(format!("ignisyeet_wscale_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("w.csv"), "altitude_m,speed,direction_deg\n0,1,270\n6000,5,270\n").unwrap();
        cfg.wind.model = sim::WindModel::Profile;
        cfg.wind.profile = Some(dir.join("w.csv"));
        let run = |k: f64, v: f64| sim::Summary { wind_scale: Some(k), wind_speed: v, ..sample_summary() };
        // within 1/3 .. 3: no warning; non-profile runs have no scale
        assert!(wind_scale_warning(
            &cfg,
            "wind grid",
            [&run(1.0, 3.0), &run(2.9, 8.7), &run(0.4, 1.2), &sim::Summary { wind_scale: None, ..sample_summary() }]
        )
        .is_none());
        // calm case (v = 0, k = 0) is not a warning
        assert!(wind_scale_warning(&cfg, "wind grid", [&run(0.0, 0.0)]).is_none());
        // the largest deviation is named, with the winds aloft (largest row 5 m/s x k)
        let w = wind_scale_warning(&cfg, "wind grid", [&run(4.5, 9.0), &run(2.0, 4.0), &run(3.5, 7.0)]).unwrap();
        assert_eq!(w, "wind profile scaled by a factor of 4.5 (wind grid 9.0 m/s vs profile mean 2.0 m/s): winds aloft up to 22 m/s");
        // just above the limit: enough decimals to show it
        let w = wind_scale_warning(&cfg, "wind grid", [&run(3.014, 9.042)]).unwrap();
        assert!(w.starts_with("wind profile scaled by a factor of 3.01 ("), "{w}");
        let w = wind_scale_warning(&cfg, "wind grid", [&run(3.0004, 9.0)]).unwrap();
        assert!(w.starts_with("wind profile scaled by a factor of 3.0004 ("), "{w}");
        let w = wind_scale_warning(&cfg, "Monte Carlo sample", [&run(0.1, 0.3)]).unwrap();
        assert!(w.starts_with("wind profile scaled by a factor of 0.10 (Monte Carlo sample 0.3 m/s vs profile mean 3.0 m/s)"), "{w}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn parachute_warnings_texts() {
        let mut cfg = load_sample(include_str!("../../../examples/sample.toml"));
        cfg.recovery.enabled = true;
        cfg.recovery.delay = 5000.0;
        let base = sim::Summary { descent: Descent::Parachute, landing_time: 100.0, landing_east: 1.0, landing_north: 1.0, ..sample_summary() };
        let never = sim::Summary { drogue_deploy_time: None, landing_speed: 178.0, ..base.clone() };
        let w = parachute_warnings(&cfg, &never);
        assert_eq!(w, vec!["parachute descent landed at 178 m/s: the parachute did not deploy before landing (recovery.delay = 5000 s)".to_string()]);
        let fast = sim::Summary { drogue_deploy_time: Some(10.0), landing_speed: 22.04, ..base.clone() };
        assert_eq!(parachute_warnings(&cfg, &fast), vec!["parachute descent landed at 22.0 m/s, which is above 15 m/s: check recovery.cd_s".to_string()]);
        let ok =
            sim::Summary { drogue_deploy_time: Some(10.0), drogue_deploy_speed: Some(20.0), drogue_deploy_q: Some(200.0), landing_speed: 6.0, ..base.clone() };
        assert!(parachute_warnings(&cfg, &ok).is_empty());
        // A deployment at high speed or dynamic pressure (also when the run did not land).
        let hard = sim::Summary { drogue_deploy_speed: Some(129.0), drogue_deploy_q: Some(7200.0), ..ok.clone() };
        let want = "the parachute opens at 129 m/s (q = 7.2 kPa): a canopy is unlikely to survive; the landing point assumes it does - check recovery.delay";
        assert_eq!(parachute_warnings(&cfg, &hard), vec![want.to_string()]);
        assert_eq!(parachute_warnings(&cfg, &unlanded(hard.clone())), vec![want.to_string()]);
        let thin_air = sim::Summary { drogue_deploy_speed: Some(45.0), drogue_deploy_q: Some(2100.0), ..ok.clone() };
        assert_eq!(parachute_warnings(&cfg, &thin_air).len(), 1);
        // Dual deploy: drogue and main are named, each with the setting that decides its opening.
        cfg.recovery.main_cd_s = Some(3.0);
        cfg.recovery.main_altitude = Some(300.0);
        let main = sim::Summary { main_deploy_speed: Some(60.0), main_deploy_q: Some(2300.0), ..hard.clone() };
        assert_eq!(
            parachute_warnings(&cfg, &main),
            vec![
                want.replace("the parachute", "the drogue"),
                "the main opens at 60 m/s (q = 2.3 kPa): a canopy is unlikely to survive; the landing point assumes it does - check recovery.main_altitude"
                    .to_string()
            ]
        );
        cfg.recovery.main_cd_s = None;
        cfg.recovery.main_altitude = None;
        // ballistic runs and disabled recovery are not judged, unlanded runs only for the deployment
        assert!(parachute_warnings(&cfg, &sim::Summary { descent: Descent::Ballistic, ..never.clone() }).is_empty());
        assert!(parachute_warnings(&cfg, &unlanded(never.clone())).is_empty());
        cfg.recovery.enabled = false;
        assert!(parachute_warnings(&cfg, &hard).is_empty());
        assert!(parachute_warnings(&cfg, &never).is_empty());
    }

    #[test]
    fn min_static_margin_includes_the_rail_exit_value() {
        let s = sim::Summary { min_stability_cal: 2.0, stability_at_rail_exit: 1.1, ..sample_summary() };
        assert_eq!(static_margin_min(&s), 1.1);
        let panel = view::flight_panel(&s, Descent::Ballistic, false);
        let text = console::strip_ansi_codes(&ui().render_panel(&panel).join("\n")).to_string();
        assert!(text.contains("min margin") && text.contains("min margin     1.10 cal"), "{text}");
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
    fn failed_sim_removes_the_previous_flight_from_the_kml() {
        let mut cfg = sample_config("failed_kml");
        cfg.output.kml = true;
        // A flight that lands: the KML holds its Flight folder.
        assert_eq!(run_sim(&cfg, Some(Descent::Ballistic)).unwrap(), None);
        let path = cfg.out_dir().join(KML_FILE);
        assert!(std::fs::read_to_string(&path).unwrap().contains("<Folder><name>Flight</name>"));
        // The next run fails (missing motor file): summary.json is the failure marker and the KML has no flight.
        cfg.motor.eng = "no_such_motor.eng".into();
        let e = run_sim(&cfg, Some(Descent::Ballistic)).unwrap_err();
        sim_failed(&cfg, &e);
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.contains("Launch site") && !text.contains("<Folder><name>Flight</name>"), "{text}");
        let j: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(cfg.out_dir().join("summary.json")).unwrap()).unwrap();
        assert_eq!(j["failed"].as_bool(), Some(true));
        let _ = std::fs::remove_dir_all(cfg.out_dir());
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
        assert!((s.landing_east + 654.8).abs() < 0.05 && (s.landing_north + 7707.9).abs() < 0.05, "{} {}", s.landing_east, s.landing_north);
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
            println!(
                "reference rk4 dt=0.5ms: parachute apogee {:.3} land ({:.2}, {:.2}); ballistic apogee {:.3} land ({:.2}, {:.2})",
                refs[0].0.summary.apogee,
                refs[0].0.summary.landing_east,
                refs[0].0.summary.landing_north,
                refs[1].0.summary.apogee,
                refs[1].0.summary.landing_east,
                refs[1].0.summary.landing_north
            );
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

    #[test]
    fn dual_deploy_summary_json_and_terminal_speeds() {
        let cfg = load_sample(include_str!("../../../examples/sample.toml"));
        let none = summary_json(&cfg, &sample_summary(), &[]).unwrap();
        assert!(none["main_deploy_time"].is_null() && none["main_deploy_altitude"].is_null());
        let s = sim::Summary { main_deploy_time: Some(61.5), main_deploy_altitude: Some(300.2), ..sample_summary() };
        let j = summary_json(&cfg, &s, &[]).unwrap();
        assert_eq!((j["main_deploy_time"].as_f64(), j["main_deploy_altitude"].as_f64()), (Some(61.5), Some(300.2)));
        // a larger canopy is slower: v ~ 1/sqrt(Cd*S)
        let (drogue, main) = (chute_speed(10.0, 0.5), chute_speed(10.0, 2.0));
        assert!((drogue / main - 2.0).abs() < 1e-9 && main > 0.0);
    }

    #[test]
    fn motor_warnings_reach_the_prepared_warnings() {
        let mut cfg = sample_config("motorwarn");
        let p = prepare(&cfg).unwrap();
        assert!(p.geometry_warnings.iter().all(|w| !w.contains("thrust curve")), "{:?}", p.geometry_warnings);
        std::fs::create_dir_all(&cfg.output.dir).unwrap();
        let eng = cfg.output.dir.join("abrupt.eng");
        std::fs::write(&eng, "ABRUPT 75 500 0 3.0 4.5 X\n0.00 0\n0.20 2400\n2.00 2000\n").unwrap();
        cfg.motor.eng = eng;
        let p = prepare(&cfg).unwrap();
        assert!(p.geometry_warnings.iter().any(|w| w.contains("thrust curve ends at 2000 N")), "{:?}", p.geometry_warnings);
        let _ = std::fs::remove_dir_all(&cfg.output.dir);
    }

    #[test]
    fn wind_grid_errors_are_kept_listed_and_written_to_the_csv() {
        use sim::dispersion::Case;
        let case = |v: f64, d: f64| Case { descent: Descent::Parachute, wind_speed: v, wind_direction_deg: d };
        let cases: Vec<Case> = (0..6).map(|i| case(7.0, 90.0 * i as f64)).collect();
        let ok = sim::Summary { landing_east: 1.0, landing_north: 2.0, landing_distance: 2.2, landing_time: 10.0, ..sample_summary() };
        let detailed: Vec<std::result::Result<sim::Summary, String>> = vec![
            Ok(ok.clone()),
            Err("integration diverged, state \"NaN\"".into()),
            Ok(ok.clone()),
            Err("e2\nsecond line".into()),
            Err("e3".into()),
            Err("e4".into()),
        ];
        let (results, errors) = split_wind_grid(&cases, detailed);
        assert_eq!(errors.iter().flatten().count(), 4);
        assert!(results[0].landed() && !results[1].landed() && results[1].wind_speed == 7.0 && results[1].wind_direction_deg == 90.0);
        let w = wind_grid_error_warnings(&cases, &errors);
        assert_eq!(w.len(), 4);
        assert!(w[0].starts_with("wind-grid case 7 m/s from 90\u{b0} (parachute): integration diverged"), "{w:?}");
        assert!(w[3].starts_with("1 more wind-grid cases failed") && w[3].contains("error column"), "{w:?}");
        let path = std::env::temp_dir().join(format!("ignisyeet_cli_gridcsv_{}.csv", std::process::id()));
        write_wind_grid_csv(&path, &results, &errors).unwrap();
        let text = std::fs::read_to_string(&path).unwrap();
        let _ = std::fs::remove_file(&path);
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines.len(), 7, "{text}");
        assert!(lines[0].ends_with(",landed,error,stable"));
        assert!(lines[1].ends_with(",true,,true"), "{}", lines[1]);
        assert!(lines[2].ends_with(",false,\"integration diverged, state \"\"NaN\"\"\","), "{}", lines[2]);
        assert!(lines[4].ends_with(",false,e2 second line,"), "{}", lines[4]);
    }

    #[test]
    fn set_out_and_aero_table_flags_have_help() {
        use clap::CommandFactory;
        let mut cmd = Cli::command();
        for sub in ["geom", "aero", "sim", "dispersion"] {
            let c = cmd.find_subcommand_mut(sub).unwrap();
            let ids: Vec<String> = c.get_arguments().map(|a| a.get_id().to_string()).collect();
            assert!(ids.contains(&"set".to_string()) && ids.contains(&"out".to_string()), "{sub}: {ids:?}");
            assert_eq!(ids.contains(&"aero_table".to_string()), sub != "geom", "{sub}");
        }
        let cli = Cli::try_parse_from([
            "ignisyeet",
            "sim",
            "c.toml",
            "--set",
            "wind.speed=3",
            "--set",
            "dispersion.mode=monte_carlo",
            "--out",
            "o",
            "--aero-table",
            "t",
        ])
        .unwrap();
        let Cmd::Sim { ov, table, .. } = cli.cmd else { panic!("not sim") };
        assert_eq!(ov.set, vec!["wind.speed=3", "dispersion.mode=monte_carlo"]);
        assert_eq!((ov.out, table.aero_table), (Some(PathBuf::from("o")), Some(PathBuf::from("t"))));
    }

    #[test]
    fn overrides_enter_the_run_hash_but_out_does_not() {
        let dir = std::env::temp_dir().join(format!("ignisyeet_cli_ovhash_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("c.toml");
        let examples = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples");
        let text = include_str!("../../../examples/sample.toml")
            .replace("\"sample_rocket.stl\"", &format!("{:?}", examples.join("sample_rocket.stl")))
            .replace("\"sample_motor.eng\"", &format!("{:?}", examples.join("sample_motor.eng")));
        std::fs::write(&path, text).unwrap();
        let load = |set: &[&str], out: Option<&str>| {
            Config::load_with(&path, &config::Overrides { set: set.iter().map(|s| s.to_string()).collect(), out: out.map(PathBuf::from) }).unwrap()
        };
        let plain = load(&[], None);
        let set = load(&["wind.speed=9"], None);
        assert_ne!(run_hash(&plain, RunKind::Flight), run_hash(&set, RunKind::Flight));
        // the same value through --set or in the file: the same hash
        assert_eq!(run_hash(&load(&["wind.speed=4.0"], None), RunKind::Flight), run_hash(&plain, RunKind::Flight));
        assert_eq!(run_hash(&load(&[], Some("elsewhere")), RunKind::Flight), run_hash(&plain, RunKind::Flight));
        assert_ne!(source_hash(&load(&["aero.mach_step=0.04"], None)).unwrap(), source_hash(&plain).unwrap());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn aero_table_is_reused_from_another_directory_when_the_key_matches() {
        let a = sample_config("reuse_a");
        let mut b = sample_config("reuse_b");
        let _ = std::fs::remove_dir_all(&b.output.dir);
        let built = aero_table(&a, false).unwrap();
        b.aero_table_source = Some(a.output.dir.clone());
        let t = aero_table(&b, false).unwrap();
        assert_eq!(t.meta.source_hash, built.meta.source_hash);
        for f in ["aero_table.csv", "aero_table.json", "geometry.json", "profile.csv", "aero_drag.csv"] {
            assert_eq!(std::fs::read(a.output.dir.join(f)).unwrap(), std::fs::read(b.output.dir.join(f)).unwrap(), "{f}");
        }
        // the copy is an ordinary cache hit afterwards
        b.aero_table_source = None;
        assert_eq!(aero_table(&b, false).unwrap().meta.source_hash, built.meta.source_hash);
        // a table of other aero settings is refused
        let mut c = sample_config("reuse_c");
        c.aero.mach_step = 0.04;
        c.aero_table_source = Some(a.output.dir.join("aero_table.csv"));
        let e = aero_table(&c, false).unwrap_err().to_string();
        assert!(e.starts_with("--aero-table: ") && e.contains("was built for another rocket or other aero settings"), "{e}");
        assert!(!c.output.dir.join("aero_table.csv").exists());
        c.aero.mach_step = 0.02;
        assert!(aero_table_for(&c, true, Sub::Aero).unwrap_err().to_string().contains("--aero-table and --force"));
        let mut missing = sample_config("reuse_d");
        missing.aero_table_source = Some(missing.output.dir.join("nope.csv"));
        assert!(aero_table(&missing, false).unwrap_err().to_string().contains("--aero-table: cannot load aero table"));
        for d in [&a.output.dir, &b.output.dir, &c.output.dir, &missing.output.dir] {
            let _ = std::fs::remove_dir_all(d);
        }
    }

    #[test]
    fn profile_targets_fly_like_the_wind_grid_case() {
        let mut cfg = sample_config("ptarget");
        cfg.wind.model = sim::WindModel::Profile;
        cfg.wind.profile = Some("wind_profile_example.csv".into());
        cfg.recovery.enabled = false;
        let p = prepare(&cfg).unwrap();
        let mut grid = cfg.dispersion.clone();
        grid.mode = sim::DispersionMode::WindGrid;
        grid.wind_speeds = vec![15.6];
        grid.directions = 4;
        let (cases, results) = sim::dispersion::run(&grid, &simulation(&cfg, &p).unwrap(), false).unwrap();
        let i = cases.iter().position(|c| c.wind_direction_deg == 90.0).unwrap();
        let mut target = cfg.clone();
        target.wind.profile_mean_speed = Some(15.6);
        target.wind.profile_mean_direction_deg = Some(90.0);
        let sim = simulation(&target, &p).unwrap();
        let s = sim.run(Descent::Ballistic, false).unwrap().summary;
        assert_eq!((s.wind_speed, s.wind_direction_deg), (15.6, 90.0));
        let g = &results[i];
        assert_eq!((s.landing_east, s.landing_north, s.apogee), (g.landing_east, g.landing_north, g.apogee));
        assert_eq!(s.wind_scale, g.wind_scale);
        assert!(s.wind_scale.unwrap() > 1.0);
        // the dispersion files name the wind model and the nominal scale
        let j = serde_json::Value::Object(wind_json(&sim.wind));
        assert_eq!((j["wind_model"].as_str(), j["wind_speed"].as_f64(), j["wind_direction_deg"].as_f64()), (Some("profile"), Some(15.6), Some(90.0)));
        assert_eq!(j["wind_scale"].as_f64(), s.wind_scale);
        assert_eq!(summary_json(&target, &s, &[]).unwrap()["wind_model"], "profile");
        let power = serde_json::Value::Object(wind_json(&simulation(&sample_config("ptarget"), &p).unwrap().wind));
        assert!(power["wind_model"] == "power" && power.get("wind_scale").is_none());
        let _ = std::fs::remove_dir_all(&cfg.output.dir);
    }
}
