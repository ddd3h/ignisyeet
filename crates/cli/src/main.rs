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
use sim::{Descent, Motor, Simulation};
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
    after_help = "Copyright (C) 2025-2026 西濱大将 (NISHIHAMA Daisuke). License GPL-3.0-or-later: <https://www.gnu.org/licenses/>. This is free software; there is NO WARRANTY."
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
    /// Run one 6-DoF flight with the configured wind (writes trajectory.csv, summary.json).
    Sim {
        config: PathBuf,
        /// Override the descent mode.
        #[arg(long, value_parser = parse_descent)]
        descent: Option<Descent>,
    },
    /// Landing dispersion: wind speed x direction grid (dispersion.csv) or Monte Carlo (dispersion_mc.csv, dispersion_summary.json).
    Dispersion { config: PathBuf },
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

fn main() -> Result<()> {
    let cli = Cli::parse();
    ui::init(cli.quiet, cli.no_progress);
    match cli.cmd {
        Cmd::Geom { config } => {
            let cfg = Config::load(&config)?;
            begin(&cfg, &config, Sub::Geom);
            let step = ui().step("Extracting geometry");
            let g = geometry(&cfg)?;
            write_geometry(&cfg, &g)?;
            step.done(&view::geometry_detail(&g));
            show_geometry(&g, true);
        }
        Cmd::Aero { config, force } => {
            let cfg = Config::load(&config)?;
            begin(&cfg, &config, Sub::Aero);
            aero_table_for(&cfg, force, Sub::Aero)?;
        }
        Cmd::Sim { config, descent } => {
            let cfg = Config::load(&config)?;
            begin(&cfg, &config, Sub::Sim);
            run_sim(&cfg, descent)?
        }
        Cmd::Dispersion { config } => {
            let cfg = Config::load(&config)?;
            begin(&cfg, &config, Sub::Dispersion);
            run_dispersion(&cfg)?
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
    Ok(())
}

/// Banner and configuration summary (rich mode only).
fn begin(cfg: &Config, config_path: &Path, sub: Sub) {
    if !ui().rich() {
        return;
    }
    ui().banner();
    let motor = if matches!(sub, Sub::Sim | Sub::Dispersion) { load_motor(cfg).ok() } else { None };
    ui().print_panel(&view::config_panel(cfg, config_path, sub, motor.as_ref()));
    println!();
}

fn load_motor(cfg: &Config) -> Result<Motor> {
    let mut motor = Motor::load(&cfg.resolve(&cfg.motor.eng))?;
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
    let tris: Vec<_> = geom::stl::read_stl(&stl)?.into_iter().map(|t| t.map(|v| v * s)).collect();
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
    Ok(g)
}

fn write_geometry(cfg: &Config, g: &Geometry) -> Result<()> {
    std::fs::write(out_path(cfg, "geometry.json")?, serde_json::to_string_pretty(g)?)?;
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
        mach_step: a.mach_step,
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

fn source_hash(cfg: &Config) -> Result<String> {
    let mut h = 0xcbf2_9ce4_8422_2325u64;
    fnv1a(&mut h, &std::fs::read(cfg.resolve(&cfg.rocket.stl))?);
    #[derive(Serialize)]
    struct Inputs<'a> {
        scale: f64,
        nose: &'a str,
        fins: &'a Option<config::FinOverride>,
        aero: &'a config::AeroCfg,
        atmosphere: &'a sim::AtmosphereConfig,
        opt: AeroOptions,
        version: &'a str,
    }
    let inputs = Inputs {
        scale: cfg.rocket.stl_scale,
        nose: &cfg.rocket.nose_direction,
        fins: &cfg.rocket.fin_override,
        aero: &cfg.aero,
        atmosphere: &cfg.atmosphere,
        opt: aero_options(cfg),
        version: env!("CARGO_PKG_VERSION"),
    };
    fnv1a(&mut h, serde_json::to_string(&inputs)?.as_bytes());
    Ok(format!("{h:016x}"))
}

/// Load the cached table when its inputs are unchanged, otherwise rebuild and save it.
fn aero_table(cfg: &Config, force: bool) -> Result<AeroTable> {
    aero_table_for(cfg, force, Sub::Sim)
}

const BAR_T: &str = "  {spinner:.cyan} {prefix:<12.bold} {bar} {pos}/{len} {percent:>3}%  {elapsed:.dim}  ETA {eta_precise:.yellow}\n      {msg:.dim}";

fn aero_table_for(cfg: &Config, force: bool, sub: Sub) -> Result<AeroTable> {
    let u = ui();
    let t_start = std::time::Instant::now();
    match cfg.aero.method {
        config::AeroMethod::Barrowman | config::AeroMethod::Panel => {}
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
    let start = std::time::Instant::now();
    let step = u.step("Extracting geometry");
    let g = geometry(cfg)?;
    write_geometry(cfg, &g)?;
    step.done(&view::geometry_detail(&g));
    show_geometry(&g, sub == Sub::Aero);
    if cfg.aero.method == config::AeroMethod::Panel {
        return panel_table(cfg, g, hash, &path, start, sub);
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
    ((a.mach_max - a.mach_min.max(0.0)) / a.mach_step).round().max(1.0) as usize + 1
}

fn show_table(t: &AeroTable, elapsed: Option<std::time::Duration>, sub: Sub) {
    if sub == Sub::Aero && ui().rich() {
        println!();
        ui().print_panel(&view::table_panel(t, elapsed));
    }
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
}

fn prepare(cfg: &Config) -> Result<Prepared> {
    let table = aero_table(cfg, false)?;
    let motor = load_motor(cfg)?;
    let motor_aft_x = cfg.motor.aft_x.unwrap_or(table.meta.length);
    Ok(Prepared { table, motor, motor_aft_x })
}

fn simulation<'a>(cfg: &Config, p: &'a Prepared) -> Result<Simulation<'a>> {
    let r = &cfg.rocket;
    Ok(Simulation {
        table: &p.table,
        mass: sim::MassProperties { dry_mass: r.dry_mass, cg_dry: r.cg_dry, ixx_dry: r.ixx_dry, iyy_dry: r.iyy_dry },
        motor: &p.motor,
        motor_aft_x: p.motor_aft_x,
        launch: cfg.launch.clone(),
        recovery: cfg.recovery.clone(),
        wind: sim::Wind::from_config(&cfg.wind_config())?,
        atmosphere: cfg.atmosphere.build()?,
        earth: sim::Earth::from_config(&cfg.earth)?,
        settings: sim::Settings {
            dt: cfg.sim.dt,
            max_time: cfg.sim.max_time,
            output_interval: cfg.sim.output_interval,
            integrator: cfg.sim.integrator,
            rtol: cfg.sim.rtol,
            atol: cfg.sim.atol,
        },
        aero_scale: sim::AeroScale::default(),
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

fn run_sim(cfg: &Config, descent: Option<Descent>) -> Result<()> {
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
    std::fs::write(out_path(cfg, "summary.json")?, serde_json::to_string_pretty(&r.summary)?)?;
    let s = &r.summary;
    if u.plain() {
        println!("Flight ({descent:?}, wind {:.1} m/s from {:.0} deg)", s.wind_speed, s.wind_direction_deg);
        println!("  rail exit       {:.2} s, {:.1} m/s, stability {:.2} cal", s.rail_exit_time, s.rail_exit_speed, s.stability_at_rail_exit);
        println!("  max speed       {:.1} m/s (Mach {:.2}), max q {:.1} kPa", s.max_speed, s.max_mach, s.max_dyn_pressure / 1000.0);
        println!("  min stability   {:.2} cal", s.min_stability_cal);
        println!("  apogee          {:.1} m at {:.2} s", s.apogee, s.apogee_time);
        println!("  landing         {:.1} s, E {:.1} m, N {:.1} m ({:.6}, {:.6}), {:.1} m/s", s.landing_time, s.landing_east, s.landing_north, s.landing_lat, s.landing_lon, s.landing_speed);
        println!("  -> {}", path.display());
    } else if u.rich() {
        println!();
        u.print_panel(&view::flight_panel(s, descent, true));
        if s.min_stability_cal < 1.5 {
            u.warn(format!("minimum stability {:.2} cal is below the recommended 1.5 cal", s.min_stability_cal));
        }
    }
    write_kml(cfg, kml::Parts { flight: Some(kml::Flight { samples: r.samples, summary: r.summary }), ..Default::default() })
}

/// Writes `ignisyeet.kml`: the parts given are used as they are, the others are read back from the
/// output files already in the output directory (a missing part is left out, an unreadable one is
/// noted in plain mode).
fn write_kml(cfg: &Config, mut parts: kml::Parts) -> Result<()> {
    if !cfg.output.kml {
        return Ok(());
    }
    let u = ui();
    let dir = cfg.out_dir();
    let note = |what: &str, e: String| {
        let msg = format!("KML: {what} left out ({e})");
        match u.mode {
            ui::Mode::Plain => println!("  {msg}"),
            ui::Mode::Rich => u.warn(msg),
            ui::Mode::Quiet => {}
        }
    };
    if parts.flight.is_none() {
        match kml::load_flight(&dir) {
            Ok(f) => parts.flight = f,
            Err(e) => note("flight", e),
        }
    }
    if parts.wind_grid.is_none() {
        match kml::load_wind_grid(&dir) {
            Ok(w) => parts.wind_grid = w,
            Err(e) => note("wind grid", e),
        }
    }
    if parts.monte_carlo.is_none() {
        match kml::load_monte_carlo(&dir) {
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
    writeln!(f, "t,phase,east,north,up,lat,lon,alt,vel_e,vel_n,vel_u,airspeed,mach,alpha_deg,thrust,mass,x_cg,x_cp,stability_cal,dyn_pressure,cn,ca,pitch_deg,heading_deg,wx,wy,wz")?;
    for s in samples {
        let phase = match s.phase {
            sim::flight::Phase::Rail => "rail",
            sim::flight::Phase::Free => "free",
            sim::flight::Phase::Parachute => "parachute",
        };
        writeln!(
            f,
            "{:.4},{phase},{:.3},{:.3},{:.3},{:.8},{:.8},{:.3},{:.4},{:.4},{:.4},{:.4},{:.5},{:.4},{:.3},{:.4},{:.5},{:.5},{:.4},{:.2},{:.5},{:.5},{:.4},{:.4},{:.5},{:.5},{:.5}",
            s.t, s.east, s.north, s.up, s.lat, s.lon, s.alt, s.vel_e, s.vel_n, s.vel_u, s.airspeed, s.mach, s.alpha_deg, s.thrust, s.mass,
            s.x_cg, s.x_cp, s.stability_cal, s.dyn_pressure, s.cn, s.ca, s.pitch_deg, s.heading_deg, s.wx, s.wy, s.wz
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

fn run_dispersion(cfg: &Config) -> Result<()> {
    let u = ui();
    let p = prepare(cfg)?;
    let base = simulation(cfg, &p)?;
    if cfg.dispersion.mode == sim::DispersionMode::MonteCarlo {
        return run_monte_carlo(cfg, &base);
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
    let path = out_path(cfg, "dispersion.csv")?;
    write_wind_grid_csv(&path, &results)?;
    u.done_line("Wind grid", &format!("{} cases", results.len()), start.elapsed());
    if u.plain() {
        println!("Dispersion: {} cases in {:.2?} -> {}", results.len(), start.elapsed(), path.display());
        let mut modes: Vec<Descent> = Vec::new();
        for c in &cases {
            if !modes.contains(&c.descent) {
                modes.push(c.descent);
            }
        }
        for &m in &modes {
            let far = results.iter().filter(|s| s.descent == m).map(|s| s.landing_distance).fold(0.0, f64::max);
            println!("  {m:?}: max landing distance {far:.1} m");
        }
    } else if u.rich() {
        println!();
        u.print_panel(&view::wind_grid_panel(&cases, &results, start.elapsed()));
    }
    write_kml(cfg, kml::Parts { wind_grid: Some(kml::WindGrid { cases, results }), ..Default::default() })
}

fn write_wind_grid_csv(path: &Path, results: &[sim::Summary]) -> Result<()> {
    let mut f = std::io::BufWriter::new(std::fs::File::create(path)?);
    writeln!(f, "descent,wind_speed,wind_direction_deg,landing_east,landing_north,landing_lat,landing_lon,landing_distance,landing_time,landing_speed,apogee,apogee_time,max_mach,rail_exit_speed,min_stability_cal")?;
    for s in results {
        let mode = if s.descent == Descent::Ballistic { "ballistic" } else { "parachute" };
        writeln!(
            f,
            "{mode},{},{},{:.2},{:.2},{:.8},{:.8},{:.2},{:.2},{:.2},{:.2},{:.2},{:.4},{:.2},{:.3}",
            s.wind_speed, s.wind_direction_deg, s.landing_east, s.landing_north, s.landing_lat, s.landing_lon, s.landing_distance,
            s.landing_time, s.landing_speed, s.apogee, s.apogee_time, s.max_mach, s.rail_exit_speed, s.min_stability_cal
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

fn run_monte_carlo(cfg: &Config, base: &Simulation) -> Result<()> {
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
    let path = out_path(cfg, "dispersion_mc.csv")?;
    write_monte_carlo_csv(&path, &r.rows)?;
    let spath = out_path(cfg, "dispersion_summary.json")?;
    std::fs::write(&spath, serde_json::to_string_pretty(&r.summary)?)?;
    u.done_line("Monte Carlo", &format!("{} samples x {} descent modes", r.summary.samples, r.summary.descents.len()), start.elapsed());
    if u.plain() {
        println!("Monte Carlo: {} samples x {} descent modes in {:.2?} -> {}, {}", r.summary.samples, r.summary.descents.len(), start.elapsed(), path.display(), spath.display());
        for d in &r.summary.descents {
            println!("  {:?}: {} ok, {} failed", d.descent, d.landing.n, d.failed);
            if d.landing.n == 0 {
                continue;
            }
            let (e1, e3) = (d.landing.ellipse_1sigma, d.landing.ellipse_3sigma);
            println!("    mean landing E {:.1} m, N {:.1} m; apogee {:.1} +- {:.1} m", d.landing.mean_east, d.landing.mean_north, d.apogee_mean, d.apogee_std);
            println!("    1-sigma ellipse {:.1} x {:.1} m, 3-sigma {:.1} x {:.1} m, major axis bearing {:.0} deg", e1.semi_major, e1.semi_minor, e3.semi_major, e3.semi_minor, e1.major_axis_bearing_deg);
            println!("    max landing distance {:.1} m (sample {})", d.max_landing_distance, d.max_landing_distance_sample.unwrap_or(0));
        }
    } else if u.rich() {
        println!();
        u.print_panel(&view::monte_carlo_panel(&r.summary, start.elapsed(), r.rows.len()));
    }
    write_kml(cfg, kml::Parts { monte_carlo: Some(kml::MonteCarlo { rows: r.rows.clone(), stats: r.summary.descents.clone() }), ..Default::default() })?;
    for row in r.rows.iter().filter(|x| x.result.is_err()).take(3) {
        let msg = format!("failed: sample {} ({}): {}", row.sample, descent_name(row.descent), row.result.as_ref().unwrap_err());
        match u.mode {
            ui::Mode::Plain => println!("  {msg}"),
            ui::Mode::Rich => u.warn(msg),
            ui::Mode::Quiet => eprintln!("{msg}"),
        }
    }
    Ok(())
}

fn write_monte_carlo_csv(path: &Path, rows: &[sim::dispersion::McRow]) -> Result<()> {
    let mut f = std::io::BufWriter::new(std::fs::File::create(path)?);
    writeln!(
        f,
        "sample,descent,status,thrust_scale,burn_time_scale,dry_mass,cg,cn_scale,ca_scale,elevation_deg,azimuth_deg,wind_speed_delta,wind_direction_deg_delta,parachute_cd_s_scale,\
wind_speed,wind_direction_deg,landing_east,landing_north,landing_lat,landing_lon,landing_distance,landing_time,landing_speed,apogee,apogee_time,max_mach,rail_exit_speed,min_stability_cal"
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
            Ok(s) => writeln!(
                f,
                "{},{},{:.2},{:.2},{:.8},{:.8},{:.2},{:.2},{:.2},{:.2},{:.2},{:.4},{:.2},{:.3}",
                s.wind_speed, s.wind_direction_deg, s.landing_east, s.landing_north, s.landing_lat, s.landing_lon, s.landing_distance,
                s.landing_time, s.landing_speed, s.apogee, s.apogee_time, s.max_mach, s.rail_exit_speed, s.min_stability_cal
            )?,
            Err(_) => writeln!(f, ",,,,,,,,,,,,,")?,
        }
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
}
