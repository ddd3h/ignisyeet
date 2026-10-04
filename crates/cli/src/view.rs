// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025-2026 西濱大将 (NISHIHAMA Daisuke)
//! Rich (terminal) renderings of the configuration and of the results.

use crate::config::{AeroMethod, Config};
use crate::ui::{fmt_dur, pad, pad_left, short_path, ui, Panel};
use aero::AeroTable;
use console::{style, StyledObject};
use geom::Geometry;
use serde::Serialize;
use sim::dispersion::{Case, McSummary};
use sim::{Descent, Motor, Summary};
use std::time::Duration;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Sub {
    Geom,
    Aero,
    Sim,
    Dispersion,
}

/// Serialized (TOML spelling) name of an enum value.
fn name<T: Serialize>(x: &T) -> String {
    serde_json::to_value(x).ok().and_then(|v| v.as_str().map(str::to_string)).unwrap_or_default()
}

/// Bold number followed by a dim unit.
pub fn val(n: impl std::fmt::Display, unit: &str) -> String {
    if unit.is_empty() {
        style(n).bold().to_string()
    } else {
        format!("{} {}", style(n).bold(), style(unit).dim())
    }
}

fn dim(s: impl std::fmt::Display) -> StyledObject<String> {
    style(s.to_string()).dim()
}

fn sep() -> String {
    format!(" {} ", dim(ui().g().dot))
}

pub fn config_panel(cfg: &Config, config_path: &std::path::Path, sub: Sub, motor: Option<&Motor>) -> Panel {
    let mut p = Panel::new("Configuration");
    let flight = matches!(sub, Sub::Sim | Sub::Dispersion);
    let s = sep();
    p.section("Input");
    p.row("config", short_path(config_path));
    p.row("STL", format!("{} {}{}", short_path(&cfg.resolve(&cfg.rocket.stl)), dim(format!("(scale {}, nose {})", cfg.rocket.stl_scale, cfg.rocket.nose_direction)), ""));
    if flight {
        let eng = short_path(&cfg.resolve(&cfg.motor.eng));
        match motor {
            Some(m) => {
                p.row("motor", format!("{}  {}", style(&m.name).bold(), dim(eng)));
                p.row(
                    "",
                    format!(
                        "{}{s}{}{s}{}{s}{}",
                        val(format!("{:.1}", m.total_impulse()), "N s"),
                        val(format!("{:.2}", m.burn_time()), "s burn"),
                        val(format!("{:.0}", m.total_impulse() / m.burn_time().max(1e-9)), "N avg"),
                        val(format!("{:.3}", m.propellant_mass), "kg propellant")
                    ),
                );
            }
            None => {
                p.row("motor", eng);
            }
        }
    }

    p.section("Resources");
    let r = cfg.resources.resolve();
    p.row("threads", val(r.threads, &format!("of {} available", r.available)));
    p.row("memory", r.memory_bytes.map(|b| format!("{:.1} GB budget", b as f64 / 1e9)).unwrap_or_else(|| "no limit".into()));
    p.row("nice", r.nice.to_string());

    if flight {
        p.section("Rocket");
        let r = &cfg.rocket;
        p.row("dry mass", format!("{}{s}CG {}{s}Ixx {}{s}Iyy {}", val(format!("{:.3}", r.dry_mass), "kg"), val(format!("{:.3}", r.cg_dry), "m"), val(r.ixx_dry, "kg m2"), val(r.iyy_dry, "kg m2")));
        p.row("extra Cd", format!("{}{s}motor aft_x {}", val(r.extra_cd, ""), cfg.motor.aft_x.map_or("body length".to_string(), |x| format!("{x} m"))));
    } else {
        p.section("Geometry");
        let fin = if let Some(o) = &cfg.rocket.fin_override { format!("override ({} fields)", [o.count.is_some(), o.root_chord.is_some(), o.tip_chord.is_some(), o.span.is_some(), o.sweep.is_some(), o.thickness.is_some(), o.x_le_root.is_some()].iter().filter(|b| **b).count()) } else { "auto-detect".to_string() };
        p.row("slices", format!("{}{s}fin threshold {}{s}fins {}", val(cfg.aero.n_slices, ""), val(cfg.aero.fin_threshold, ""), fin));
    }

    if flight {
        let l = &cfg.launch;
        p.section("Launch");
        p.row("site", format!("{}, {}{s}{} {}", val(format!("{:.4}", l.latitude), "N"), val(format!("{:.4}", l.longitude), "E"), val(format!("{:.0}", l.altitude), "m"), dim("ASL")));
        p.row("rail", format!("{}{s}elevation {}{s}azimuth {}", val(format!("{:.1}", l.rail_length), "m"), val(format!("{:.1}", l.elevation_deg), "deg"), val(format!("{:.0}", l.azimuth_deg), "deg")));
        p.section("Recovery");
        if cfg.recovery.enabled {
            p.row("parachute", format!("Cd*S {}{s}deploys {} after apogee", val(format!("{:.2}", cfg.recovery.cd_s), "m2"), val(format!("{:.1}", cfg.recovery.delay), "s")));
        } else {
            p.row("parachute", dim("disabled (ballistic descent)").to_string());
        }
    }

    if matches!(sub, Sub::Aero | Sub::Sim | Sub::Dispersion) {
        let a = &cfg.aero;
        p.section("Aerodynamics");
        let m = style(name(&a.method)).bold().magenta().to_string();
        match a.method {
            AeroMethod::Table => {
                p.row("method", format!("{m}{s}{}", a.table.as_ref().map(|t| short_path(&cfg.resolve(t))).unwrap_or_default()));
            }
            _ => {
                p.row("method", format!("{m}{s}roughness {}{s}extra Cd {}", val(cfg.rocket.roughness * 1e6, "um"), val(cfg.rocket.extra_cd, "")));
                p.row("grid", format!("Mach {} .. {} step {}{s}alpha 0 .. {} step {} deg", val(a.mach_min, ""), val(a.mach_max, ""), val(a.mach_step, ""), val(a.alpha_max_deg, ""), val(a.alpha_step_deg, "")));
            }
        }
        if a.method == AeroMethod::Cfd {
            let c = &a.cfd;
            p.row("cfd", format!("{}{s}surface {}{s}{}", style(name(&c.model)).bold(), style(name(&c.surface)).bold(), if c.symmetry { "half model" } else { "full model" }));
            let ms: Vec<String> = c.machs.iter().map(|m| format!("{m}")).collect();
            let al: Vec<String> = c.alphas_deg.iter().map(|m| format!("{m}")).collect();
            p.row("cfd cases", format!("Mach [{}]{s}alpha [{}] deg", ms.join(", "), al.join(", ")));
            p.row("cfd mesh", format!("wall {}{s}farfield {} L{s}y+ {}", val(c.wall_size * 1e3, "mm"), val(c.farfield, ""), val(c.yplus, "")));
            p.row("cfd solver", format!("{} iter{s}CFL {}{s}res {}{s}{} ranks x {} cases", val(c.iterations, ""), val(c.cfl, ""), val(c.convergence, ""), val(c.ranks_per_case, ""), val(c.parallel_cases, "")));
        }
        if a.method == AeroMethod::Panel {
            let q = &a.panel;
            p.row("panel mesh", format!("{} x {} body{s}fin {} x {}{s}wake {} L", val(q.body_axial, ""), val(q.body_circ, ""), val(q.fin_chord, ""), val(q.fin_span, ""), val(q.wake_length, "")));
            let ms: Vec<String> = q.subsonic_machs.iter().map(|m| format!("{m}")).collect();
            p.row("panel Mach", format!("subsonic [{}]", ms.join(", ")));
            p.row("transonic", format!("blend {} .. {}", q.transonic[0], q.transonic[1]));
        }
    }

    if flight {
        p.section("Environment");
        p.row("earth", format!("{}{s}gravity {}", style(name(&cfg.earth.model)).bold(), style(name(&cfg.earth.gravity)).bold()));
        let at = &cfg.atmosphere;
        p.row(
            "atmosphere",
            match at.model {
                sim::env::AtmosphereKind::Us1976 => format!("{}{s}dT {}", style("us1976").bold(), val(at.temperature_offset, "K")),
                sim::env::AtmosphereKind::Constant => format!("{}{s}rho {}{s}a {}", style("constant").bold(), val(at.density, "kg/m3"), val(at.sound_speed, "m/s")),
            },
        );
        let w = &cfg.wind;
        let detail = match w.model {
            sim::WindModel::Constant => String::new(),
            sim::WindModel::Power => format!("{s}ref {} m, n = {}", w.ref_height, w.exponent),
            sim::WindModel::Log => format!("{s}ref {} m, z0 = {} m", w.ref_height, w.roughness_length),
            sim::WindModel::Profile => format!("{s}{}", w.profile.as_ref().map(|p| short_path(&cfg.resolve(p))).unwrap_or_default()),
        };
        p.row("wind", format!("{}{s}{} from {}{}", style(name(&w.model)).bold(), val(format!("{:.1}", w.speed), "m/s"), val(format!("{:.0}", w.direction_deg), "deg"), detail));

        p.section("Integration");
        let c = &cfg.sim;
        let tol = if c.integrator == sim::IntegratorKind::Rk45 { format!("{s}rtol {:e}{s}atol {:e}", c.rtol, c.atol) } else { String::new() };
        p.row("integrator", format!("{}{s}dt {}{tol}", style(name(&c.integrator)).bold(), val(c.dt, "s")));
        p.row("limits", format!("max time {}{s}output every {}", val(c.max_time, "s"), val(c.output_interval, "s")));
    }

    if sub == Sub::Dispersion {
        let d = &cfg.dispersion;
        p.section("Dispersion");
        let modes = if cfg.recovery.enabled { 2 } else { 1 };
        match d.mode {
            sim::DispersionMode::WindGrid => {
                let sp: Vec<String> = d.wind_speeds.iter().map(|v| format!("{v}")).collect();
                p.row("mode", format!("{}{s}{} speeds x {} directions x {} descent = {}", style("wind_grid").bold().magenta(), d.wind_speeds.len(), d.directions, modes, val(d.wind_speeds.len() * d.directions * modes, "runs")));
                p.row("speeds", format!("[{}] m/s", sp.join(", ")));
            }
            sim::DispersionMode::MonteCarlo => {
                let m = &d.monte_carlo;
                p.row("mode", format!("{}{s}{} samples x {} descent = {}{s}seed {}", style("monte_carlo").bold().magenta(), val(m.samples, ""), modes, val(m.samples * modes, "runs"), m.seed));
            }
        }
    }

    p.section("Output");
    p.row("directory", short_path(&cfg.out_dir()));
    p.row("KML", if cfg.output.kml { style("on").green().to_string() } else { dim("off").to_string() });
    p
}

fn stability(cal: f64) -> String {
    let g = ui().g();
    let txt = format!("{cal:.2} cal");
    if !cal.is_finite() {
        dim("n/a").to_string()
    } else if cal < 1.0 {
        format!("{}  {} {}", style(txt).red().bold(), style(g.bad).red().bold(), style("UNSTABLE").red().bold())
    } else if cal < 1.5 {
        format!("{}  {} {}", style(txt).yellow().bold(), style(g.warn).yellow().bold(), style("marginal (< 1.5 cal)").yellow())
    } else {
        format!("{}  {}", style(txt).green().bold(), style(g.ok).green())
    }
}

pub fn geometry_panel(g: &Geometry) -> Panel {
    let s = sep();
    let mut p = Panel::new("Geometry");
    p.row("length", val(format!("{:.4}", g.length), "m"));
    p.row("diameter", format!("{}{s}ref area {}", val(format!("{:.4}", 2.0 * g.ref_radius), "m"), val(format!("{:.6}", g.ref_area), "m2")));
    p.row("nose", format!("length {}{s}joint angle {}{s}volume ratio {}", val(format!("{:.4}", g.nose.length), "m"), val(format!("{:.2}", g.nose.joint_half_angle_deg), "deg"), val(format!("{:.3}", g.nose.volume_ratio), "")));
    if let Some(b) = &g.boattail {
        p.row("boattail", format!("length {}{s}r {:.4} -> {:.4} m", val(format!("{:.4}", b.length), "m"), b.r_fore, b.r_aft));
    }
    match &g.fins {
        Some(f) => {
            p.row("fins", format!("{} x root {} tip {} span {}", val(f.count, ""), val(format!("{:.4}", f.root_chord), "m"), val(format!("{:.4}", f.tip_chord), "m"), val(format!("{:.4}", f.span), "m")));
            p.row("", format!("sweep {:.4} m{s}thickness {:.4} m{s}at x {:.4} m", f.sweep, f.thickness, f.x_le_root));
        }
        None => {
            p.row("fins", dim("none detected").to_string());
        }
    }
    p
}

pub fn geometry_detail(g: &Geometry) -> String {
    let fins = g.fins.as_ref().map_or("no fins".to_string(), |f| format!("{} fins", f.count));
    format!("L {:.3} m {d} D {:.4} m {d} {fins}", g.length, 2.0 * g.ref_radius, d = ui().g().dot)
}

/// Coefficient preview of an aerodynamic table (alpha -> 0).
pub fn table_panel(t: &AeroTable, elapsed: Option<Duration>) -> Panel {
    let s = sep();
    let mut p = Panel::new("Aerodynamic table");
    let m = &t.meta;
    p.row("rows", format!("{}{s}Mach {} x alpha {}{}", val(t.rows(), ""), val(m.machs.len(), ""), val(m.alphas_deg.len(), ""), elapsed.map_or(String::new(), |e| format!("{s}built in {}", fmt_dur(e)))));
    p.row("reference", format!("D {}{s}A {}{s}L {}", val(format!("{:.4}", m.ref_diameter), "m"), val(format!("{:.6}", m.ref_area), "m2"), val(format!("{:.4}", m.length), "m")));
    p.text("");
    let hdr = format!("{}{}{}{}{}", pad_left("Mach", 6), pad_left("CNa [/rad]", 13), pad_left("Xcp [m]", 11), pad_left("CA power-off", 15), pad_left("CA power-on", 14));
    p.text(style(hdr).dim().underlined().to_string());
    let lo = m.machs.first().copied().unwrap_or(0.0);
    let hi = m.machs.last().copied().unwrap_or(0.0);
    for target in [0.1, 0.3, 0.6, 0.9, 1.2, 2.0, 3.0] {
        if target < lo - 1e-9 || target > hi + 1e-9 {
            continue;
        }
        let c = t.lookup(target, 0.0);
        p.text(format!(
            "{}{}{}{}{}",
            pad_left(&style(format!("{target:.1}")).cyan().bold().to_string(), 6),
            pad_left(&format!("{:.3}", c.cna), 13),
            pad_left(&format!("{:.4}", c.xcp), 11),
            pad_left(&format!("{:.3}", c.ca_off), 15),
            pad_left(&format!("{:.3}", c.ca_on), 14)
        ));
    }
    p
}

pub fn panel_report_panel(r: &panel::PanelReport) -> Panel {
    let s = sep();
    let mut p = Panel::new("Panel method");
    p.row("mesh", format!("{} panels + {} wake panels{s}solves took {}", val(r.panels, ""), val(r.wake_panels, ""), val(format!("{:.2}", r.solve_seconds), "s")));
    p.text("");
    p.text(style(format!("{}{}{}", pad_left("Mach", 6), pad_left("CNa [/rad]", 13), pad_left("Xcp [m]", 11))).dim().underlined().to_string());
    for x in &r.subsonic {
        p.text(format!("{}{}{}", pad_left(&style(format!("{:.2}", x.mach)).cyan().bold().to_string(), 6), pad_left(&format!("{:.3}", x.cna), 13), pad_left(&format!("{:.4}", x.xcp), 11)));
    }
    p
}

pub fn flight_panel(sm: &Summary, descent: Descent, g_lat_lon: bool) -> Panel {
    let s = sep();
    let title = format!("Flight result {} {} descent", ui().g().dot, if descent == Descent::Ballistic { "ballistic" } else { "parachute" });
    let mut p = Panel::new(title);
    p.row("wind", format!("{} from {}", val(format!("{:.1}", sm.wind_speed), "m/s"), val(format!("{:.0}", sm.wind_direction_deg), "deg")));
    p.row("rail exit", format!("{}{s}{}{s}stability {}", val(format!("{:.2}", sm.rail_exit_time), "s"), val(format!("{:.1}", sm.rail_exit_speed), "m/s"), stability(sm.stability_at_rail_exit)));
    p.row("burnout", val(format!("{:.2}", sm.burnout_time), "s"));
    p.row("max speed", format!("{} {}{s}max q {}", style(format!("{:.1} m/s", sm.max_speed)).bold().yellow(), dim(format!("(Mach {:.2})", sm.max_mach)), val(format!("{:.1}", sm.max_dyn_pressure / 1000.0), "kPa")));
    p.row("min stability", stability(sm.min_stability_cal));
    p.row("apogee", format!("{} {}{s}at {}", style(format!("{:.1} m", sm.apogee)).bold().green(), dim("AGL"), val(format!("{:.2}", sm.apogee_time), "s")));
    p.row("landing", format!("E {} N {}{s}{} downrange", val(format!("{:.1}", sm.landing_east), "m"), val(format!("{:.1}", sm.landing_north), "m"), style(format!("{:.0} m", sm.landing_distance)).bold().cyan()));
    p.row("", format!("t = {}{s}impact speed {}", val(format!("{:.1}", sm.landing_time), "s"), val(format!("{:.1}", sm.landing_speed), "m/s")));
    if g_lat_lon {
        p.row("", dim(format!("{:.6}, {:.6}", sm.landing_lat, sm.landing_lon)).to_string());
    }
    p
}

pub fn wind_grid_panel(cases: &[Case], results: &[Summary], elapsed: Duration) -> Panel {
    let s = sep();
    let mut p = Panel::new("Dispersion result");
    p.row("cases", format!("{}{s}{}", val(results.len(), "runs"), dim(format!("in {}", fmt_dur(elapsed)))));
    p.text("");
    let w = [11, 7, 13, 11, 22];
    let hdr = format!("{}{}{}{}{}", pad("descent", w[0]), pad_left("cases", w[1]), pad_left("max landing", w[2]), pad_left("mean", w[3]), pad_left("worst case", w[4]));
    p.text(style(hdr).dim().underlined().to_string());
    let mut modes: Vec<Descent> = Vec::new();
    for c in cases {
        if !modes.contains(&c.descent) {
            modes.push(c.descent);
        }
    }
    for &m in &modes {
        let idx: Vec<usize> = (0..cases.len()).filter(|&i| cases[i].descent == m).collect();
        let (worst, far) = idx.iter().map(|&i| (i, results[i].landing_distance)).fold((idx[0], 0.0), |a, b| if b.1 > a.1 { b } else { a });
        let mean = idx.iter().map(|&i| results[i].landing_distance).sum::<f64>() / idx.len() as f64;
        let nm = if m == Descent::Ballistic { style("ballistic").red() } else { style("parachute").green() };
        p.text(format!(
            "{}{}{}{}{}",
            pad(&nm.bold().to_string(), w[0]),
            pad_left(&idx.len().to_string(), w[1]),
            pad_left(&style(format!("{far:.1} m")).bold().cyan().to_string(), w[2]),
            pad_left(&format!("{mean:.1} m"), w[3]),
            pad_left(&dim(format!("{:.0} m/s from {:.0} deg", cases[worst].wind_speed, cases[worst].wind_direction_deg)).to_string(), w[4])
        ));
    }
    p
}

pub fn monte_carlo_panel(sm: &McSummary, elapsed: Duration, n_rows: usize) -> Panel {
    let s = sep();
    let mut p = Panel::new("Monte Carlo result");
    p.spaced = true;
    p.row("runs", format!("{} samples x {} descent modes = {}{s}{}", val(sm.samples, ""), sm.descents.len(), val(n_rows, "runs"), dim(format!("in {}", fmt_dur(elapsed)))));
    for d in &sm.descents {
        let nm = if d.descent == Descent::Ballistic { style("Ballistic").red().bold() } else { style("Parachute").green().bold() };
        p.section(&nm.to_string());
        let failed = if d.failed > 0 { style(format!("{} failed", d.failed)).red().to_string() } else { dim("0 failed").to_string() };
        p.row("samples", format!("{} ok{s}{failed}", val(d.landing.n, "")));
        if d.landing.n == 0 {
            continue;
        }
        let (e1, e3) = (d.landing.ellipse_1sigma, d.landing.ellipse_3sigma);
        p.row("mean landing", format!("E {} N {}", val(format!("{:.1}", d.landing.mean_east), "m"), val(format!("{:.1}", d.landing.mean_north), "m")));
        p.row("apogee", format!("{} {} {}", style(format!("{:.1} m", d.apogee_mean)).bold().green(), dim("+-"), val(format!("{:.1}", d.apogee_std), "m")));
        p.row("1-sigma", format!("{} x {}{s}bearing {} deg", val(format!("{:.1}", e1.semi_major), "m"), val(format!("{:.1}", e1.semi_minor), "m"), val(format!("{:.0}", e1.major_axis_bearing_deg), "")));
        p.row("3-sigma", format!("{} x {}", val(format!("{:.1}", e3.semi_major), "m"), val(format!("{:.1}", e3.semi_minor), "m")));
        p.row("max landing", format!("{}  {}", style(format!("{:.1} m", d.max_landing_distance)).bold().cyan(), dim(format!("(sample {})", d.max_landing_distance_sample.unwrap_or(0)))));
    }
    p
}
