// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025-2026 西濱大将 (NISHIHAMA Daisuke)
//! `ignisyeet report`: one self-contained HTML file from the result files of an output directory.
//!
//! The report reads only files that exist; every section is left out when its files are absent. Nothing is
//! computed again except values that follow directly from a result file (peak acceleration, table statistics).

use crate::config::Config;
use crate::report_svg::{self as svg, esc, Chart, Mark, Series};
use anyhow::{bail, Context as _, Result};
use serde_json::Value;
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

/// Run hashes of the current configuration, one per kind of result file.
pub struct Hashes {
    pub flight: String,
    pub wind_grid: String,
    pub monte_carlo: String,
}

/// The motor of the configuration, when its file could be read.
pub struct MotorInfo {
    pub name: String,
    pub total_impulse: f64,
    pub burn_time: f64,
    pub propellant_mass: f64,
}

pub struct Context<'a> {
    pub config_path: &'a Path,
    pub version: &'static str,
    /// Seconds since the Unix epoch of the generation time (`SOURCE_DATE_EPOCH` when set).
    pub epoch: u64,
    pub hashes: Hashes,
    pub motor: Option<MotorInfo>,
    /// Output directory the results are read from.
    pub dir: PathBuf,
    /// Directory of the report file; files are linked only when it is `dir`.
    pub report_dir: PathBuf,
    pub embed_plots: bool,
}

pub struct Report {
    pub html: String,
    /// Results computed with another configuration than the current one (for the [IY-O545] warning).
    pub mismatches: Vec<String>,
    /// Names of the sections written.
    pub sections: Vec<&'static str>,
}

/// Result files that make a directory a result directory.
const RESULT_FILES: [&str; 6] = ["summary.json", "dispersion.csv", "dispersion_mc.csv", "dispersion_summary.json", "aero_table.json", "geometry.json"];

// ---------------------------------------------------------------- reading

fn read_json(dir: &Path, name: &str) -> Result<Option<Value>> {
    let p = dir.join(name);
    if !p.is_file() {
        return Ok(None);
    }
    let text = std::fs::read_to_string(&p).with_context(|| format!("[IY-O043] cannot read {}", p.display()))?;
    let v = serde_json::from_str(&text).with_context(|| format!("[IY-O043] cannot read {}: not valid JSON", p.display()))?;
    Ok(Some(v))
}

/// Splits one CSV line (comma separated, double quotes around fields that hold commas).
fn split_csv(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut q = false;
    let mut it = line.chars().peekable();
    while let Some(c) = it.next() {
        match c {
            '"' if q && it.peek() == Some(&'"') => {
                cur.push('"');
                it.next();
            }
            '"' => q = !q,
            ',' if !q => out.push(std::mem::take(&mut cur)),
            c => cur.push(c),
        }
    }
    out.push(cur);
    out
}

/// The wanted columns of a CSV file (all other columns are dropped while reading).
struct Csv {
    names: Vec<String>,
    cols: Vec<Vec<String>>,
}

impl Csv {
    fn load(path: &Path, want: &[&str]) -> Result<Option<Csv>> {
        if !path.is_file() {
            return Ok(None);
        }
        let text = std::fs::read_to_string(path).with_context(|| format!("[IY-O043] cannot read {}", path.display()))?;
        let mut lines = text.lines();
        let Some(head) = lines.next() else { return Ok(None) };
        let head = split_csv(head);
        let idx: Vec<(usize, &str)> = want.iter().filter_map(|w| head.iter().position(|h| h == w).map(|i| (i, *w))).collect();
        let mut cols = vec![Vec::new(); idx.len()];
        for l in lines.filter(|l| !l.trim().is_empty()) {
            let f = split_csv(l);
            if f.len() < head.len() {
                continue;
            }
            for (c, (i, _)) in cols.iter_mut().zip(&idx) {
                c.push(f[*i].clone());
            }
        }
        Ok(Some(Csv { names: idx.iter().map(|(_, n)| n.to_string()).collect(), cols }))
    }

    fn rows(&self) -> usize {
        self.cols.first().map_or(0, Vec::len)
    }

    fn text(&self, name: &str) -> Option<&[String]> {
        self.names.iter().position(|n| n == name).map(|i| self.cols[i].as_slice())
    }

    fn num(&self, name: &str) -> Option<Vec<f64>> {
        self.text(name).map(|c| c.iter().map(|s| s.trim().parse::<f64>().unwrap_or(f64::NAN)).collect())
    }
}

fn get(v: &Value, k: &str) -> Option<f64> {
    v.get(k).and_then(Value::as_f64).filter(|x| x.is_finite())
}

fn get_str<'a>(v: &'a Value, k: &str) -> Option<&'a str> {
    v.get(k).and_then(Value::as_str)
}

// ---------------------------------------------------------------- formatting

/// A number with `dec` decimals and its unit, "n/a" when it is missing.
fn q(v: Option<f64>, dec: usize, unit: &str) -> String {
    match v.filter(|x| x.is_finite()) {
        Some(x) if unit.is_empty() => format!("{x:.dec$}"),
        Some(x) if unit.starts_with('\u{b0}') => format!("{x:.dec$}{unit}"),
        Some(x) => format!("{x:.dec$}\u{a0}{unit}"),
        None => "n/a".into(),
    }
}

/// UTC date and time of a Unix time.
pub fn format_utc(epoch: u64) -> String {
    let days = (epoch / 86400) as i64;
    let rem = epoch % 86400;
    // Days since 1970-01-01 to a civil date (proleptic Gregorian).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y:04}-{m:02}-{d:02} {:02}:{:02} UTC", rem / 3600, rem % 3600 / 60)
}

fn base64(bytes: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut o = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for c in bytes.chunks(3) {
        let n = (u32::from(c[0]) << 16) | (u32::from(*c.get(1).unwrap_or(&0)) << 8) | u32::from(*c.get(2).unwrap_or(&0));
        o.push(T[(n >> 18) as usize & 63] as char);
        o.push(T[(n >> 12) as usize & 63] as char);
        o.push(if c.len() > 1 { T[(n >> 6) as usize & 63] as char } else { '=' });
        o.push(if c.len() > 2 { T[n as usize & 63] as char } else { '=' });
    }
    o
}

fn kv(rows: &[(&str, String)]) -> String {
    let mut o = String::from("<table class=\"kv\"><tbody>");
    for (k, v) in rows {
        let _ = write!(o, "<tr><th scope=\"row\">{}</th><td>{}</td></tr>", esc(k), v);
    }
    o.push_str("</tbody></table>");
    o
}

/// Value cell text: escaped plain text (units contain no markup).
fn t(s: impl AsRef<str>) -> String {
    esc(s.as_ref())
}

fn name_of<T: serde::Serialize>(x: &T) -> String {
    serde_json::to_value(x).ok().and_then(|v| v.as_str().map(str::to_string)).unwrap_or_default()
}

fn size_text(n: u64) -> String {
    match n {
        0..=1023 => format!("{n} B"),
        1024..=1_048_575 => format!("{:.1} kB", n as f64 / 1024.0),
        _ => format!("{:.1} MB", n as f64 / 1_048_576.0),
    }
}

fn code_link(code: &str) -> String {
    let area = match code.as_bytes().get(3) {
        Some(b'C') => "config",
        Some(b'G') => "geometry",
        Some(b'A') => "aero",
        Some(b'S') => "sim",
        Some(b'O') => "cli",
        Some(b'F') => "cfd",
        _ => "index",
    };
    format!("<a class=\"code\" href=\"https://www.ddd3h.com/ignisyeet/errors/{area}.html#{}\">{}</a>", esc(&code.to_lowercase()), esc(code))
}

/// One warning line: the leading `[IY-Xnnn]` becomes a link to the code's documentation.
fn warning_item(w: &str) -> String {
    if let Some(rest) = w.strip_prefix("[IY-") {
        if let Some(end) = rest.find(']') {
            let code = format!("IY-{}", &rest[..end]);
            if code.len() == 7 {
                return format!("<li>{} {}</li>", code_link(&code), esc(rest[end + 1..].trim_start()));
            }
        }
    }
    format!("<li>{}</li>", esc(w))
}

// ---------------------------------------------------------------- build

struct Page {
    sections: Vec<(&'static str, &'static str, String)>,
}

impl Page {
    fn add(&mut self, id: &'static str, title: &'static str, body: String) {
        if !body.trim().is_empty() {
            self.sections.push((id, title, body));
        }
    }
}

/// The output directory must exist and hold at least one result file.
pub fn check_dir(dir: &Path) -> Result<()> {
    if !dir.is_dir() {
        bail!(
            "[IY-O041] the output directory {} does not exist; run sim, dispersion or aero first (or give the directory of the run with --out)",
            dir.display()
        );
    }
    if !RESULT_FILES.iter().any(|f| dir.join(f).is_file()) {
        bail!("[IY-O042] no result files in {} (looked for {}); run geom, aero, sim or dispersion first", dir.display(), RESULT_FILES.join(", "));
    }
    Ok(())
}

pub fn build(cfg: &Config, ctx: &Context) -> Result<Report> {
    let dir = &ctx.dir;
    check_dir(dir)?;
    let summary = read_json(dir, "summary.json")?;
    let meta = read_json(dir, "dispersion_meta.json")?;
    let mc = read_json(dir, "dispersion_summary.json")?;
    let geometry = read_json(dir, "geometry.json")?;
    let aero_json = read_json(dir, "aero_table.json")?;

    let mut mismatches = Vec::new();
    let mut check = |what: &str, v: &Option<Value>, key: &str, now: &str| {
        if let Some(h) = v.as_ref().and_then(|v| get_str(v, key)) {
            if h != now {
                mismatches.push(format!("{what} (recorded {h}, current {now})"));
            }
        }
    };
    check("summary.json", &summary, "config_hash", &ctx.hashes.flight);
    check("dispersion_meta.json", &meta, "config_hash", &ctx.hashes.wind_grid);
    check("dispersion_summary.json", &mc, "config_hash", &ctx.hashes.monte_carlo);

    let failed = summary.as_ref().is_some_and(|s| s.get("failed").and_then(Value::as_bool) == Some(true));
    let traj = if failed { None } else { Traj::load(&dir.join("trajectory.csv"))? };

    let mut page = Page { sections: Vec::new() };
    page.add("inputs", "Inputs", inputs_section(cfg, ctx));
    page.add("geometry", "Geometry", geometry_section(geometry.as_ref(), dir, traj.as_ref(), cfg)?);
    page.add("aero", "Aerodynamics", aero_section(cfg, aero_json.as_ref(), dir)?);
    page.add("flight", "Flight", flight_section(summary.as_ref(), traj.as_ref(), &mismatches));
    page.add("dispersion", "Landing dispersion", dispersion_section(cfg, meta.as_ref(), mc.as_ref(), dir)?);
    page.add(
        "warnings",
        "Warnings",
        warnings_section(&[("summary.json", &summary), ("dispersion_meta.json", &meta), ("dispersion_summary.json", &mc), ("geometry.json", &geometry)]),
    );
    if ctx.embed_plots {
        page.add("plots", "Further plots", plots_section(dir));
    }
    page.add("files", "Output files", files_section(dir, ctx));

    let glance = glance(summary.as_ref(), mc.as_ref(), meta.as_ref());
    let sections: Vec<&'static str> = page.sections.iter().map(|s| s.0).collect();
    let html = document(cfg, ctx, &summary, &page, &glance, &mismatches);
    Ok(Report { html, mismatches, sections })
}

// ---------------------------------------------------------------- page

const CSS: &str = r#":root{color-scheme:light dark;--bg:#fafaf8;--fg:#1c2128;--muted:#5b6470;--card:#ffffff;--border:#d9dde3;--accent:#1f5fbf;--grid:#e6e8ec;--axis:#9aa3ae;--warn:#8a5a00;--warnbg:#fff6df;--c0:#1f5fbf;--c1:#c2410c;--c2:#2b8a3e;--c3:#7c3aa8}
@media (prefers-color-scheme:dark){:root{--bg:#14171c;--fg:#e6e9ee;--muted:#98a2b0;--card:#1b1f26;--border:#303744;--accent:#7fb0ff;--grid:#2a303a;--axis:#5d6776;--warn:#f2c46b;--warnbg:#2e2614;--c0:#7fb0ff;--c1:#ff9a5c;--c2:#6fd286;--c3:#d49cf2}}
*{box-sizing:border-box}
body{margin:0;background:var(--bg);color:var(--fg);font:16px/1.55 system-ui,-apple-system,"Segoe UI",Roboto,sans-serif}
main{max-width:1000px;margin:0 auto;padding:0 16px 48px}
header.top{padding:28px 0 8px}
h1{font-size:1.7rem;margin:0 0 4px}
h2{font-size:1.25rem;margin:0 0 12px;padding-top:4px}
h3{font-size:1rem;margin:20px 0 8px;color:var(--muted);font-weight:600}
p{margin:8px 0}
a{color:var(--accent)}
nav.toc{display:flex;flex-wrap:wrap;gap:6px 16px;margin:14px 0 4px;font-size:.92rem}
section{background:var(--card);border:1px solid var(--border);border-radius:10px;padding:18px 20px;margin:18px 0}
.sub{color:var(--muted);font-size:.92rem}
.meta{display:grid;grid-template-columns:repeat(auto-fit,minmax(230px,1fr));gap:2px 24px;font-size:.92rem;margin-top:10px}
.meta div{overflow-wrap:anywhere}
.meta b{color:var(--muted);font-weight:500}
.cards{display:grid;grid-template-columns:repeat(auto-fit,minmax(170px,1fr));gap:10px;margin:14px 0 4px}
.card{background:var(--card);border:1px solid var(--border);border-radius:10px;padding:10px 14px}
.card .n{font-size:1.45rem;font-weight:650;font-variant-numeric:tabular-nums}
.card .l{color:var(--muted);font-size:.85rem}
table{border-collapse:collapse;width:100%;font-size:.93rem;font-variant-numeric:tabular-nums}
th,td{text-align:left;padding:5px 10px;border-bottom:1px solid var(--border);vertical-align:top}
table.kv th{width:46%;font-weight:500;color:var(--muted)}
table.grid th[scope=col]{background:transparent;color:var(--muted);font-weight:600}
td.r,th.r{text-align:right}
.scroll{overflow-x:auto}
.cols{display:grid;grid-template-columns:repeat(auto-fit,minmax(300px,1fr));gap:4px 28px}
.figs{display:grid;grid-template-columns:repeat(auto-fit,minmax(340px,1fr));gap:14px 22px;margin-top:8px}
@media (max-width:480px){.figs{grid-template-columns:1fr}.cols{grid-template-columns:1fr}}
figure{margin:0}
section>figure{margin-top:16px}
figcaption{color:var(--muted);font-size:.85rem;margin-top:2px}
.chart{width:100%;height:auto;display:block}
.chart .tx{fill:var(--fg);font:11px system-ui,sans-serif}
.chart .sm{font-size:10px;fill:var(--muted)}
.chart .grid{stroke:var(--grid);stroke-width:1}
.chart .ax{stroke:var(--axis);stroke-width:1}
.chart .mark{stroke:var(--muted);stroke-width:1;stroke-dasharray:3 3}
.chart .mk{stroke:var(--fg);stroke-width:1.5}
.chart .line{stroke-width:1.8;stroke-linejoin:round}
.chart .fin{stroke:var(--c1);stroke-width:1.4}
.s0{stroke:var(--c0)}.s1{stroke:var(--c1)}.s2{stroke:var(--c2)}.s3{stroke:var(--c3)}
.f0{fill:var(--c0)}.f1{fill:var(--c1)}.f2{fill:var(--c2)}.f3{fill:var(--c3)}
.legend{margin:2px 0 0;font-size:.82rem;color:var(--muted);display:flex;flex-wrap:wrap;gap:2px 14px}
.sw{display:inline-block;width:14px;height:3px;margin-right:5px;vertical-align:middle;border-radius:2px}
.b0{background:var(--c0)}.b1{background:var(--c1)}.b2{background:var(--c2)}.b3{background:var(--c3)}
ul.warn{list-style:none;padding:0;margin:8px 0}
ul.warn li{background:var(--warnbg);color:var(--fg);border-left:4px solid var(--warn);padding:7px 12px;margin:6px 0;border-radius:4px;font-size:.92rem;overflow-wrap:anywhere}
.note{background:var(--warnbg);border-left:4px solid var(--warn);padding:7px 12px;margin:10px 0;border-radius:4px;font-size:.92rem}
.code{font-family:ui-monospace,Menlo,Consolas,monospace;font-weight:600}
.ok{color:var(--c2);font-weight:600}.bad{color:var(--c1);font-weight:600}
code{font-family:ui-monospace,Menlo,Consolas,monospace;font-size:.88em}
details{margin:8px 0}
summary{cursor:pointer;color:var(--accent)}
img.plot{max-width:100%;height:auto;background:#fff;border:1px solid var(--border);border-radius:6px}
footer{color:var(--muted);font-size:.85rem;margin-top:24px}
@media print{:root{--bg:#fff;--fg:#000;--muted:#444;--card:#fff;--border:#bbb;--accent:#000;--grid:#ddd;--axis:#777;--warn:#555;--warnbg:#f3f3f3;--c0:#1f5fbf;--c1:#c2410c;--c2:#2b8a3e;--c3:#7c3aa8}
body{font-size:11pt}main{max-width:none;padding:0}section{break-inside:auto;border-color:#bbb}figure,table{break-inside:avoid}nav.toc{display:none}a{text-decoration:none}}"#;

fn document(cfg: &Config, ctx: &Context, summary: &Option<Value>, page: &Page, glance: &str, mismatches: &[String]) -> String {
    let stem = ctx.config_path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    let rocket = cfg.rocket.geometry_file().file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    let mut o = String::with_capacity(64 * 1024);
    let _ = write!(
        o,
        "<!DOCTYPE html>\n<html lang=\"en\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"><meta name=\"color-scheme\" content=\"light dark\"><meta name=\"generator\" content=\"IgnisYeet {}\"><title>IgnisYeet report: {}</title><style>{CSS}</style></head><body><main>",
        esc(ctx.version),
        esc(&stem)
    );
    let _ = write!(o, "<header class=\"top\"><h1>IgnisYeet report: {}</h1><div class=\"sub\">Rocket {}</div>", esc(&stem), esc(&rocket));
    o.push_str("<div class=\"meta\">");
    let mut meta = vec![
        ("IgnisYeet", esc(ctx.version)),
        ("Generated", format!("<time id=\"generated\">{}</time>", format_utc(ctx.epoch))),
        ("Configuration", format!("<code>{}</code>", esc(&ctx.config_path.display().to_string()))),
        ("Configuration hash", format!("<code>{}</code>", esc(&ctx.hashes.flight))),
    ];
    if let Some(h) = summary.as_ref().and_then(|s| get_str(s, "config_hash")) {
        meta.push(("Hash of the flight results", format!("<code>{}</code>", esc(h))));
    }
    meta.push(("Results directory", format!("<code>{}</code>", esc(&ctx.dir.display().to_string()))));
    for (k, v) in meta {
        let _ = write!(o, "<div><b>{}</b><br>{}</div>", esc(k), v);
    }
    o.push_str("</div>");
    if !mismatches.is_empty() {
        let _ = write!(
            o,
            "<p class=\"note\">The configuration has changed since these results were computed: {}. The Inputs section shows the current configuration; run the commands again to bring the results up to date.</p>",
            esc(&mismatches.join("; "))
        );
    }
    o.push_str("<nav class=\"toc\" aria-label=\"Sections\">");
    for (id, title, _) in &page.sections {
        let _ = write!(o, "<a href=\"#{id}\">{title}</a>");
    }
    o.push_str("</nav>");
    o.push_str(glance);
    o.push_str("</header>");
    for (id, title, body) in &page.sections {
        let _ = write!(o, "<section id=\"{id}\"><h2>{title}</h2>{body}</section>");
    }
    let _ = writeln!(o, "<footer>Written by <code>ignisyeet report</code>. The numbers are model results; read the warnings above before using them.</footer></main></body></html>");
    o
}

fn card(n: String, l: &str) -> String {
    format!("<div class=\"card\"><div class=\"n\">{n}</div><div class=\"l\">{}</div></div>", esc(l))
}

fn glance(summary: Option<&Value>, mc: Option<&Value>, meta: Option<&Value>) -> String {
    let mut cards = String::new();
    if let Some(s) = summary.filter(|s| s.get("failed").is_none()) {
        cards += &card(q(get(s, "apogee"), 0, "m"), "Apogee above the launch site");
        cards += &card(q(get(s, "max_speed"), 0, "m/s"), &format!("Maximum speed (Mach {})", q(get(s, "max_mach"), 2, "")));
        cards += &card(q(get(s, "static_margin_min_cal"), 2, "cal"), "Smallest static margin");
        let landed = s.get("landed").and_then(Value::as_bool) == Some(true);
        cards += &card(if landed { q(get(s, "landing_distance"), 0, "m") } else { "no landing".into() }, "Landing distance from the pad");
    }
    if summary.is_none() {
        let far = farthest(mc, meta);
        if let Some((d, what)) = far {
            cards += &card(q(Some(d), 0, "m"), &what);
        }
    }
    if cards.is_empty() {
        String::new()
    } else {
        format!("<div class=\"cards\">{cards}</div>")
    }
}

fn farthest(mc: Option<&Value>, meta: Option<&Value>) -> Option<(f64, String)> {
    let m = mc?.get("descents")?.as_array()?.iter().filter_map(|d| get(d, "max_landing_distance")).fold(f64::NAN, f64::max);
    let _ = meta;
    m.is_finite().then(|| (m, "Farthest Monte Carlo landing".to_string()))
}

// ---------------------------------------------------------------- inputs

fn inputs_section(cfg: &Config, ctx: &Context) -> String {
    let r = &cfg.rocket;
    let geom_kind = if r.cad.is_some() { "CAD file" } else { "STL file" };
    let mut rocket = vec![
        (geom_kind, format!("<code>{}</code>", esc(&r.geometry_file().display().to_string()))),
        ("Nose direction", t(&r.nose_direction)),
        ("Dry mass", t(q(Some(r.dry_mass), 3, "kg"))),
        ("Dry centre of gravity", t(format!("{} aft of the nose tip", q(Some(r.cg_dry), 3, "m")))),
        ("Roll inertia (dry)", t(q(Some(r.ixx_dry), 4, "kg m\u{b2}"))),
        ("Pitch/yaw inertia (dry)", t(q(Some(r.iyy_dry), 4, "kg m\u{b2}"))),
    ];
    rocket.push(("Extra drag Cd", t(q(Some(r.extra_cd), 3, ""))));
    let mut motor = vec![("Motor file", format!("<code>{}</code>", esc(&cfg.motor.eng.display().to_string())))];
    if let Some(m) = &ctx.motor {
        motor.push(("Motor", t(&m.name)));
        motor.push(("Total impulse", t(q(Some(m.total_impulse), 1, "N s"))));
        motor.push(("Burn time", t(q(Some(m.burn_time), 2, "s"))));
        motor.push(("Average thrust", t(q(Some(m.total_impulse / m.burn_time.max(1e-9)), 0, "N"))));
        motor.push(("Propellant mass", t(q(Some(m.propellant_mass), 3, "kg"))));
    }
    let a = &cfg.aero;
    let method = name_of(&a.method);
    let aero = vec![
        ("Method", t(&method)),
        ("Table kind", t(name_of(&a.table_kind))),
        ("Mach range", t(format!("{} to {}, step {}", q(Some(a.mach_min), 2, ""), q(Some(a.mach_max), 2, ""), q(Some(a.mach_step), 3, "")))),
        ("Largest angle of attack", t(q(Some(a.alpha_max_deg), 0, "deg"))),
        ("Extrapolation outside the table", t(name_of(&a.extrapolation))),
    ];
    let l = &cfg.launch;
    let launch = vec![
        ("Latitude, longitude", t(format!("{:.5}\u{b0}, {:.5}\u{b0}", l.latitude, l.longitude))),
        ("Altitude above sea level", t(q(Some(l.altitude), 1, "m"))),
        ("Rail length", t(q(Some(l.rail_length), 2, "m"))),
        ("Rail elevation", t(q(Some(l.elevation_deg), 1, "deg"))),
        ("Rail azimuth (clockwise from north)", t(q(Some(l.azimuth_deg), 1, "deg"))),
    ];
    let rec = &cfg.recovery;
    let mut recovery = vec![("Parachute", t(if rec.enabled { "enabled" } else { "disabled (ballistic descent)" }))];
    if rec.enabled {
        recovery.push(("Drogue / parachute Cd\u{b7}S", t(q(Some(rec.cd_s), 2, "m\u{b2}"))));
        recovery.push(("Deployment delay after apogee", t(q(Some(rec.delay), 2, "s"))));
        if let (Some(c), Some(h)) = (rec.main_cd_s, rec.main_altitude) {
            recovery.push(("Main canopy Cd\u{b7}S", t(q(Some(c), 2, "m\u{b2}"))));
            recovery.push(("Main deployment height", t(q(Some(h), 0, "m above the launch site"))));
        }
    }
    let w = &cfg.wind;
    let mut wind = vec![("Wind model", t(name_of(&w.model)))];
    if name_of(&w.model) == "profile" {
        wind.push(("Wind profile file", format!("<code>{}</code>", esc(&w.profile.as_deref().map(|p| p.display().to_string()).unwrap_or_default()))));
    } else {
        wind.push(("Speed at the reference height", t(q(Some(w.speed), 2, "m/s"))));
        wind.push(("Blowing from (clockwise from north)", t(q(Some(w.direction_deg), 0, "deg"))));
        wind.push(("Reference height", t(q(Some(w.ref_height), 1, "m"))));
    }
    let s = &cfg.sim;
    let mut sim = vec![
        ("Integrator", t(name_of(&s.integrator))),
        ("Descent", t(s.descent.map(|d| name_of(&d)).unwrap_or_else(|| if rec.enabled { "parachute".into() } else { "ballistic".into() }))),
    ];
    if name_of(&s.integrator) == "rk4" {
        sim.push(("Time step", t(q(Some(s.dt), 4, "s"))));
    } else {
        sim.push(("Tolerances (relative, absolute)", t(format!("{:e}, {:e}", s.rtol, s.atol))));
    }
    sim.push(("Time limit", t(q(Some(s.max_time), 0, "s"))));
    sim.push(("Earth model, gravity", t(format!("{}, {}", name_of(&cfg.earth.model), name_of(&cfg.earth.gravity)))));
    sim.push(("Atmosphere", t(name_of(&cfg.atmosphere.model))));
    let d = &cfg.dispersion;
    let mode = name_of(&d.mode);
    let mut disp = vec![("Mode", t(&mode))];
    if mode == "monte_carlo" {
        disp.push(("Samples, seed", t(format!("{}, {}", d.monte_carlo.samples, d.monte_carlo.seed))));
    } else {
        let sp: Vec<String> = d.wind_speeds.iter().map(|v| format!("{v}")).collect();
        disp.push(("Wind speeds", t(format!("{} m/s", sp.join(", ")))));
        disp.push(("Wind directions", t(format!("{} (evenly spaced)", d.directions))));
    }
    let col = |title: &str, rows: &[(&str, String)]| format!("<div><h3>{}</h3>{}</div>", esc(title), kv(rows));
    format!(
        "<p class=\"sub\">The configuration as it reads now. Relative paths are resolved from the directory of the configuration file.</p><div class=\"cols\">{}{}{}{}{}{}{}{}</div>",
        col("Rocket", &rocket),
        col("Motor", &motor),
        col("Aerodynamics", &aero),
        col("Launch site and rail", &launch),
        col("Recovery", &recovery),
        col("Wind", &wind),
        col("Simulation", &sim),
        col("Dispersion", &disp)
    )
}

// ---------------------------------------------------------------- geometry

fn geometry_section(g: Option<&Value>, dir: &Path, traj: Option<&Traj>, cfg: &Config) -> Result<String> {
    let Some(g) = g else { return Ok(String::new()) };
    let mut rows = vec![
        ("Length", t(q(get(g, "length"), 3, "m"))),
        ("Reference diameter", t(q(get(g, "ref_radius").map(|r| 2.0 * r), 4, "m"))),
        ("Reference area", t(q(get(g, "ref_area"), 5, "m\u{b2}"))),
        ("Wetted body area", t(q(get(g, "wetted_area_body"), 4, "m\u{b2}"))),
        ("Volume", t(q(get(g, "volume"), 5, "m\u{b3}"))),
    ];
    if let Some(n) = g.get("nose").filter(|n| n.is_object()) {
        rows.push(("Nose length", t(q(get(n, "length"), 3, "m"))));
        rows.push(("Nose base radius", t(q(get(n, "base_radius"), 4, "m"))));
    }
    let mut fins = Vec::new();
    if let Some(f) = g.get("fins").filter(|f| f.is_object()) {
        let count = f.get("count").and_then(Value::as_u64).map_or("n/a".to_string(), |c| c.to_string());
        fins.push(("Number of fins", t(count)));
        fins.push(("Root chord", t(q(get(f, "root_chord"), 4, "m"))));
        fins.push(("Tip chord", t(q(get(f, "tip_chord"), 4, "m"))));
        fins.push(("Span (beyond the body surface)", t(q(get(f, "span"), 4, "m"))));
        fins.push(("Leading-edge sweep", t(q(get(f, "sweep"), 4, "m"))));
        fins.push(("Thickness", t(q(get(f, "thickness"), 4, "m"))));
        fins.push(("Root leading edge, aft of the nose tip", t(q(get(f, "x_le_root"), 3, "m"))));
        if let Some(c) = get(f, "cant_deg").filter(|c| *c != 0.0) {
            fins.push(("Cant angle", t(q(Some(c), 2, "deg"))));
        }
    } else {
        fins.push(("Fins", t("none")));
    }
    if g.get("boattail").is_some_and(Value::is_object) {
        let b = &g["boattail"];
        fins.push(("Boattail length", t(q(get(b, "length"), 3, "m"))));
    }
    let mut o = format!("<div class=\"cols\"><div>{}</div><div>{}</div></div>", kv(&rows), kv(&fins));
    if let Some(p) = Csv::load(&dir.join("profile.csv"), &["x", "r", "fin_span"])? {
        let (x, r, f) = (p.num("x"), p.num("r"), p.num("fin_span"));
        if let (Some(x), Some(r), Some(f)) = (x, r, f) {
            let prof: Vec<(f64, f64, f64)> = (0..p.rows()).map(|i| (x[i], r[i], f[i])).collect();
            let cg = traj.and_then(|t| t.first("x_cg")).or(Some(cfg.rocket.cg_dry));
            let cp = traj.and_then(|t| t.first("x_cp"));
            if let Some(s) = svg::rocket_profile(&prof, cg, cp) {
                let cap = if traj.is_some() {
                    "Profile from profile.csv, to scale. CG and CP are the values at ignition (trajectory.csv)."
                } else {
                    "Profile from profile.csv, to scale. CG is the dry value of the configuration."
                };
                let _ = write!(o, "<figure>{s}<figcaption>{}</figcaption></figure>", esc(cap));
            }
        }
    }
    Ok(o)
}

// ---------------------------------------------------------------- aerodynamics

fn aero_section(cfg: &Config, a: Option<&Value>, dir: &Path) -> Result<String> {
    // Mach curves: aero_drag.csv has one row per Mach number; otherwise the alpha = 0 rows of the table.
    let mut mach = Vec::new();
    let (mut cd_on, mut cd_off, mut cna, mut xcp) = (Vec::new(), Vec::new(), Vec::new(), Vec::new());
    let mut source = "";
    if let Some(c) = Csv::load(&dir.join("aero_drag.csv"), &["mach", "cd_off", "cd_on", "cna", "xcp"])? {
        if let (Some(m), Some(d0), Some(d1), Some(n), Some(x)) = (c.num("mach"), c.num("cd_off"), c.num("cd_on"), c.num("cna"), c.num("xcp")) {
            (mach, cd_off, cd_on, cna, xcp) = (m, d0, d1, n, x);
            source = "aero_drag.csv";
        }
    }
    if mach.is_empty() {
        if let Some(c) = Csv::load(&dir.join("aero_table.csv"), &["mach", "alpha_deg", "ca_on", "ca_off", "xcp", "cna"])? {
            if let (Some(m), Some(al), Some(c1), Some(c0), Some(x), Some(n)) =
                (c.num("mach"), c.num("alpha_deg"), c.num("ca_on"), c.num("ca_off"), c.num("xcp"), c.num("cna"))
            {
                for i in (0..c.rows()).filter(|&i| al[i] == 0.0) {
                    mach.push(m[i]);
                    cd_on.push(c1[i]);
                    cd_off.push(c0[i]);
                    xcp.push(x[i]);
                    cna.push(n[i]);
                }
                source = "aero_table.csv (angle of attack 0)";
            }
        }
    }
    if a.is_none() && mach.is_empty() {
        return Ok(String::new());
    }
    let mut rows = vec![("Method", t(name_of(&cfg.aero.method)))];
    if let Some(a) = a {
        rows.push(("Table kind", t(get_str(a, "kind").unwrap_or("n/a"))));
        let machs = a.get("machs").and_then(Value::as_array).map(|v| v.iter().filter_map(Value::as_f64).collect::<Vec<_>>()).unwrap_or_default();
        if let (Some(lo), Some(hi)) = (machs.first(), machs.last()) {
            rows.push(("Mach range of the table", t(format!("{lo} to {hi} ({} values)", machs.len()))));
        }
        let al = a.get("alphas_deg").and_then(Value::as_array).map(|v| v.iter().filter_map(Value::as_f64).collect::<Vec<_>>()).unwrap_or_default();
        if let (Some(lo), Some(hi)) = (al.first(), al.last()) {
            rows.push(("Angle-of-attack range of the table", t(format!("{lo} to {hi} deg ({} values)", al.len()))));
        }
        rows.push(("Reference area", t(q(get(a, "ref_area"), 5, "m\u{b2}"))));
        rows.push(("Reference diameter", t(q(get(a, "ref_diameter"), 4, "m"))));
        rows.push(("Body length", t(q(get(a, "length"), 3, "m"))));
        rows.push(("Moment reference", t(get_str(a, "moment_ref").unwrap_or("n/a"))));
        if let Some(h) = get_str(a, "source_hash") {
            rows.push(("Table hash", format!("<code>{}</code>", esc(h))));
        }
    }
    let mut o = kv(&rows);
    if !mach.is_empty() {
        let pts = |v: &[f64]| mach.iter().copied().zip(v.iter().copied()).collect::<Vec<_>>();
        let mut figs = String::new();
        let mut c = Chart::new("Drag coefficient against Mach number", "Mach number [-]", "Drag coefficient CD [-]");
        c.y_zero = true;
        c.series.push(Series::line("power off (coasting)", 0, pts(&cd_off)));
        c.series.push(Series::line("power on (motor burning)", 1, pts(&cd_on)));
        figs += &svg::figure(&c, "Drag coefficient at zero angle of attack, referred to the reference area.");
        let mut c = Chart::new("Normal-force slope against Mach number", "Mach number [-]", "CN\u{3b1} [1/rad]");
        c.series.push(Series::line("CN\u{3b1}", 0, pts(&cna)));
        figs += &svg::figure(&c, "Slope of the normal-force coefficient at zero angle of attack.");
        let mut c = Chart::new("Centre of pressure against Mach number", "Mach number [-]", "Xcp from the nose tip [m]");
        c.series.push(Series::line("Xcp", 0, pts(&xcp)));
        figs += &svg::figure(&c, "Centre of pressure at zero angle of attack, measured aft of the nose tip.");
        let _ = write!(o, "<div class=\"figs\">{figs}</div><p class=\"sub\">Curves read from {}.</p>", esc(source));
    }
    Ok(o)
}

// ---------------------------------------------------------------- flight

struct Traj {
    cols: BTreeMap<&'static str, Vec<f64>>,
    phase: Vec<String>,
}

const TRAJ_COLS: [&str; 14] = ["t", "east", "north", "up", "vel_e", "vel_n", "vel_u", "airspeed", "mach", "stability_cal", "x_cg", "x_cp", "alt", "alpha_deg"];

impl Traj {
    fn load(path: &Path) -> Result<Option<Traj>> {
        let mut want: Vec<&str> = TRAJ_COLS.to_vec();
        want.push("phase");
        let Some(c) = Csv::load(path, &want)? else { return Ok(None) };
        if c.rows() < 2 {
            return Ok(None);
        }
        let mut cols = BTreeMap::new();
        for k in TRAJ_COLS {
            if let Some(v) = c.num(k) {
                cols.insert(k, v);
            }
        }
        if !cols.contains_key("t") {
            return Ok(None);
        }
        Ok(Some(Traj { cols, phase: c.text("phase").map(<[String]>::to_vec).unwrap_or_default() }))
    }

    fn col(&self, k: &str) -> Option<&[f64]> {
        self.cols.get(k).map(Vec::as_slice)
    }

    fn first(&self, k: &str) -> Option<f64> {
        self.col(k).and_then(|c| c.first().copied()).filter(|v| v.is_finite())
    }

    fn series(&self, y: &str, up_to: f64, scale: f64) -> Vec<(f64, f64)> {
        let (Some(t), Some(y)) = (self.col("t"), self.col(y)) else { return Vec::new() };
        t.iter().zip(y).filter(|(t, _)| **t <= up_to).map(|(t, y)| (*t, *y * scale)).collect()
    }

    /// Peak change of velocity per second between consecutive samples: (value, time).
    fn peak_acceleration(&self) -> Option<(f64, f64)> {
        let (t, e, n, u) = (self.col("t")?, self.col("vel_e")?, self.col("vel_n")?, self.col("vel_u")?);
        let mut best: Option<(f64, f64)> = None;
        for i in 1..t.len() {
            let dt = t[i] - t[i - 1];
            if dt <= 1e-9 {
                continue;
            }
            let a = ((e[i] - e[i - 1]).powi(2) + (n[i] - n[i - 1]).powi(2) + (u[i] - u[i - 1]).powi(2)).sqrt() / dt;
            if a.is_finite() && best.is_none_or(|b| a > b.0) {
                best = Some((a, t[i]));
            }
        }
        best
    }

    /// Phases in order of appearance with their first and last sample time.
    fn phases(&self) -> Vec<(String, f64, f64)> {
        let Some(t) = self.col("t") else { return Vec::new() };
        let mut out: Vec<(String, f64, f64)> = Vec::new();
        for (p, t) in self.phase.iter().zip(t) {
            match out.last_mut() {
                Some(last) if last.0 == *p => last.2 = *t,
                _ => out.push((p.clone(), *t, *t)),
            }
        }
        out
    }
}

fn flight_section(s: Option<&Value>, traj: Option<&Traj>, mismatches: &[String]) -> String {
    let Some(s) = s else { return String::new() };
    if s.get("failed").and_then(Value::as_bool) == Some(true) {
        let why = get_str(s, "error").unwrap_or("no reason recorded");
        return format!(
            "<p class=\"note\"><strong>The flight simulation failed.</strong> summary.json holds the failure marker instead of results: {}</p>",
            esc(why)
        );
    }
    let _ = mismatches;
    let landed = s.get("landed").and_then(Value::as_bool) == Some(true);
    let stable = s.get("stable").and_then(Value::as_bool);
    let verdict = match stable {
        Some(true) => "<span class=\"ok\">stable</span>".to_string(),
        Some(false) => "<span class=\"bad\">statically UNSTABLE: the results are not usable</span>".to_string(),
        None => format!("<span class=\"bad\">could not be evaluated</span> ({})", esc(get_str(s, "stability_unknown").unwrap_or("no reason recorded"))),
    };
    let mut ascent = vec![
        ("Descent mode", t(get_str(s, "descent").unwrap_or("n/a"))),
        (
            "Wind",
            t(format!(
                "{} from {}, model {}",
                q(get(s, "wind_speed"), 2, "m/s"),
                q(get(s, "wind_direction_deg"), 0, "deg"),
                get_str(s, "wind_model").unwrap_or("n/a")
            )),
        ),
        ("Rail exit", t(format!("{} at {}", q(get(s, "rail_exit_speed"), 1, "m/s"), q(get(s, "rail_exit_time"), 2, "s")))),
        ("Static margin at rail exit", t(q(get(s, "stability_at_rail_exit"), 2, "cal"))),
        ("Burnout", t(q(get(s, "burnout_time"), 2, "s"))),
        ("Maximum speed", t(format!("{} (Mach {})", q(get(s, "max_speed"), 1, "m/s"), q(get(s, "max_mach"), 3, "")))),
        ("Maximum dynamic pressure", t(q(get(s, "max_dyn_pressure").map(|p| p / 1000.0), 1, "kPa"))),
    ];
    if let Some((a, at)) = traj.and_then(Traj::peak_acceleration) {
        ascent.push((
            "Peak acceleration (from trajectory.csv)",
            t(format!("{} = {} at {}", q(Some(a), 0, "m/s\u{b2}"), q(Some(a / 9.80665), 1, "g"), q(Some(at), 2, "s"))),
        ));
    }
    ascent.push(("Apogee above the launch site", t(format!("{} at {}", q(get(s, "apogee"), 1, "m"), q(get(s, "apogee_time"), 2, "s")))));
    ascent.push(("Apogee offset east, north", t(format!("{}, {}", q(get(s, "apogee_east"), 0, "m"), q(get(s, "apogee_north"), 0, "m")))));
    let mut stab = vec![
        ("Static stability", verdict),
        ("Smallest static margin", t(format!("{}{}", q(get(s, "static_margin_min_cal"), 2, "cal"), margin_at(s)))),
        ("Smallest margin in free flight (airspeed above 10 m/s)", t(q(get(s, "min_stability_cal"), 2, "cal"))),
    ];
    if get(s, "static_margin_pitch_min_cal").is_some() && get(s, "static_margin_pitch_min_cal") != get(s, "static_margin_yaw_min_cal") {
        stab.push((
            "Smallest margin, pitch plane / yaw plane",
            t(format!("{} / {}", q(get(s, "static_margin_pitch_min_cal"), 2, "cal"), q(get(s, "static_margin_yaw_min_cal"), 2, "cal"))),
        ));
    }
    let mut descent = Vec::new();
    if let Some(tm) = get(s, "drogue_deploy_time") {
        descent.push((
            "Parachute deployed",
            t(format!(
                "{} at {}, dynamic pressure {}",
                q(get(s, "drogue_deploy_speed"), 1, "m/s"),
                q(Some(tm), 2, "s"),
                q(get(s, "drogue_deploy_q").map(|p| p / 1000.0), 2, "kPa")
            )),
        ));
    }
    if let Some(tm) = get(s, "main_deploy_time") {
        descent.push((
            "Main canopy deployed",
            t(format!("{} height at {}, {}", q(get(s, "main_deploy_altitude"), 0, "m"), q(Some(tm), 2, "s"), q(get(s, "main_deploy_speed"), 1, "m/s"))),
        ));
    }
    if landed {
        descent.push(("Landing time", t(q(get(s, "landing_time"), 1, "s"))));
        descent.push(("Landing speed", t(q(get(s, "landing_speed"), 2, "m/s"))));
        descent.push(("Landing point east, north of the pad", t(format!("{}, {}", q(get(s, "landing_east"), 0, "m"), q(get(s, "landing_north"), 0, "m")))));
        descent.push(("Landing latitude, longitude", t(format!("{}, {}", q(get(s, "landing_lat"), 6, "\u{b0}"), q(get(s, "landing_lon"), 6, "\u{b0}")))));
        descent.push(("Distance from the pad", t(q(get(s, "landing_distance"), 0, "m"))));
    } else {
        descent.push(("Landing", format!("<span class=\"bad\">the rocket did not land within {}</span>", t(q(get(s, "max_time"), 0, "s")))));
    }
    if get(s, "broadside_time").is_some_and(|b| b > 0.0) {
        descent.push(("Time fallen broadside", t(q(get(s, "broadside_time"), 1, "s"))));
    }
    let mut o = format!(
        "<div class=\"cols\"><div><h3>Ascent</h3>{}</div><div><h3>Stability</h3>{}<h3>Descent and landing</h3>{}</div></div>",
        kv(&ascent),
        kv(&stab),
        kv(&descent)
    );
    if let Some(tr) = traj {
        o += &flight_figures(s, tr, landed);
        let ph = tr.phases();
        if !ph.is_empty() {
            let mut tbl = String::from("<h3>Flight phases</h3><div class=\"scroll\"><table class=\"grid\"><thead><tr><th scope=\"col\">Phase</th><th scope=\"col\" class=\"r\">From [s]</th><th scope=\"col\" class=\"r\">To [s]</th></tr></thead><tbody>");
            for (p, a, b) in ph {
                let _ = write!(tbl, "<tr><td>{}</td><td class=\"r\">{a:.2}</td><td class=\"r\">{b:.2}</td></tr>", esc(&p));
            }
            tbl.push_str("</tbody></table></div>");
            o += &tbl;
        }
    }
    o
}

fn margin_at(s: &Value) -> String {
    match (get(s, "static_margin_min_t"), get(s, "static_margin_min_mach")) {
        (Some(t), Some(m)) => format!(" at t = {t:.2} s, Mach {m:.2}"),
        _ => String::new(),
    }
}

fn flight_figures(s: &Value, tr: &Traj, landed: bool) -> String {
    let t_end = tr.col("t").and_then(|t| t.last().copied()).unwrap_or(0.0);
    let t_apogee = get(s, "apogee_time").unwrap_or(t_end);
    let mut marks = vec![Mark { x: t_apogee, y: None, label: "apogee".into() }];
    if let Some(d) = get(s, "drogue_deploy_time") {
        marks.push(Mark { x: d, y: None, label: "deploy".into() });
    }
    let m = |marks: &[Mark]| marks.iter().map(|m| Mark { x: m.x, y: None, label: m.label.clone() }).collect::<Vec<_>>();
    let up = if tr.col("up").is_some() { "up" } else { "alt" };
    let mut figs = String::new();
    let mut c = Chart::new("Height against time", "Time [s]", "Height above the launch site [m]");
    c.series.push(Series::line("height", 0, svg::decimate(&tr.series(up, t_end, 1.0), 900)));
    c.marks = m(&marks);
    figs += &svg::figure(&c, "Height above the launch site over the whole flight (trajectory.csv).");
    // Ground track.
    let (e, n) = (tr.col("east"), tr.col("north"));
    if let (Some(e), Some(n)) = (e, n) {
        let span = e.iter().chain(n).fold(0.0f64, |a, v| if v.is_finite() { a.max(v.abs()) } else { a });
        let (k, unit) = if span > 4000.0 { (1e-3, "km") } else { (1.0, "m") };
        let pts: Vec<(f64, f64)> = e.iter().zip(n).map(|(e, n)| (e * k, n * k)).collect();
        let mut c = Chart::new("Ground track", &format!("East [{unit}]"), &format!("North [{unit}]"));
        c.equal = true;
        c.series.push(Series::line("ground track", 0, svg::decimate(&pts, 900)));
        c.marks.push(Mark { x: 0.0, y: Some(0.0), label: "pad".into() });
        if let (Some(ae), Some(an)) = (get(s, "apogee_east"), get(s, "apogee_north")) {
            c.marks.push(Mark { x: ae * k, y: Some(an * k), label: "apogee".into() });
        }
        if landed {
            if let (Some(le), Some(ln)) = (get(s, "landing_east"), get(s, "landing_north")) {
                c.marks.push(Mark { x: le * k, y: Some(ln * k), label: "landing".into() });
            }
        }
        figs += &svg::figure(&c, "Track over the ground, north up. The wind blows towards the side the rocket drifts to.");
    }
    let mut c = Chart::new("Airspeed against time", "Time [s]", "Airspeed [m/s]");
    c.y_zero = true;
    c.series.push(Series::line("airspeed", 0, svg::decimate(&tr.series("airspeed", t_end, 1.0), 900)));
    c.marks = m(&marks);
    figs += &svg::figure(&c, "Speed relative to the air over the whole flight.");
    let t_asc = (t_apogee * 1.02).min(t_end);
    let mut c = Chart::new("Airspeed and Mach number during the ascent", "Time [s]", "Airspeed [m/s]");
    c.y_zero = true;
    c.series.push(Series::line("airspeed", 0, svg::decimate(&tr.series("airspeed", t_asc, 1.0), 900)));
    figs += &svg::figure(&c, "Airspeed from launch to apogee.");
    let mut c = Chart::new("Mach number during the ascent", "Time [s]", "Mach number [-]");
    c.y_zero = true;
    c.series.push(Series::line("Mach number", 1, svg::decimate(&tr.series("mach", t_asc, 1.0), 900)));
    figs += &svg::figure(&c, "Mach number from launch to apogee.");
    // Static margin while the airspeed is high enough for it to mean something.
    if let (Some(t), Some(m), Some(v)) = (tr.col("t"), tr.col("stability_cal"), tr.col("airspeed")) {
        let pts: Vec<(f64, f64)> = (0..t.len()).filter(|&i| t[i] <= t_asc && v[i] >= 10.0).map(|i| (t[i], m[i])).collect();
        let mut c = Chart::new("Static margin at the current angle of attack", "Time [s]", "Static margin [cal]");
        c.series.push(Series::line("static margin", 2, svg::decimate(&pts, 900)));
        figs += &svg::figure(&c, "Column stability_cal of trajectory.csv: centre of pressure minus centre of gravity in body diameters at the angle of attack the rocket flies at, while the airspeed is above 10 m/s. The smallest static margin above is the small-angle value and can differ.");
    }
    format!("<div class=\"figs\">{figs}</div>")
}

// ---------------------------------------------------------------- dispersion

fn dispersion_section(cfg: &Config, meta: Option<&Value>, mc: Option<&Value>, dir: &Path) -> Result<String> {
    let mut o = String::new();
    let _ = cfg;
    if let Some(m) = meta {
        o += &wind_grid(m, dir)?;
    }
    if let Some(m) = mc {
        o += &monte_carlo(m, dir)?;
    }
    Ok(o)
}

fn descent_order(d: &str) -> usize {
    usize::from(d != "ballistic")
}

/// Axis scale of landing charts: kilometres for large scatter.
fn unit_scale(pts: &[(f64, f64)]) -> (f64, &'static str) {
    let span = pts.iter().fold(0.0f64, |a, p| a.max(p.0.abs()).max(p.1.abs()));
    if span > 4000.0 {
        (1e-3, "km")
    } else {
        (1.0, "m")
    }
}

fn wind_grid(meta: &Value, dir: &Path) -> Result<String> {
    let Some(c) = Csv::load(
        &dir.join("dispersion.csv"),
        &[
            "descent",
            "wind_speed",
            "wind_direction_deg",
            "landing_east",
            "landing_north",
            "landing_distance",
            "landing_speed",
            "landing_time",
            "apogee",
            "landed",
            "error",
        ],
    )?
    else {
        return Ok(String::new());
    };
    let (Some(desc), Some(sp), Some(dr), Some(e), Some(n), Some(dist)) =
        (c.text("descent"), c.num("wind_speed"), c.num("wind_direction_deg"), c.num("landing_east"), c.num("landing_north"), c.num("landing_distance"))
    else {
        return Ok(String::new());
    };
    let apogee = c.num("apogee");
    let mut by: BTreeMap<usize, (String, Vec<usize>)> = BTreeMap::new();
    for (i, d) in desc.iter().enumerate() {
        if e[i].is_finite() && n[i].is_finite() {
            by.entry(descent_order(d)).or_insert_with(|| (d.clone(), Vec::new())).1.push(i);
        }
    }
    let total = c.rows();
    let landed = by.values().map(|v| v.1.len()).sum::<usize>();
    let mut o = String::from("<h3>Wind grid</h3>");
    let _ = write!(
        o,
        "<p>{total} cases (wind speed by wind direction, each flown with every descent mode); {landed} landed. Wind speeds are speeds at the wind reference height; directions are the directions the wind blows from.</p>"
    );
    if let Some(cases) = meta.get("cases").and_then(Value::as_u64) {
        let (f, e2) = (meta.get("failed").and_then(Value::as_u64).unwrap_or(0), meta.get("errored").and_then(Value::as_u64).unwrap_or(0));
        if f + e2 > 0 {
            let _ = write!(
                o,
                "<p class=\"note\">{f} of {cases} cases did not land and {e2} stopped with an error; they are missing from the charts and tables.</p>"
            );
        }
    }
    // Table: farthest landing per wind speed and descent.
    let mut speeds: Vec<f64> = sp.iter().copied().filter(|v| v.is_finite()).collect();
    speeds.sort_by(f64::total_cmp);
    speeds.dedup();
    let mut tbl = String::from("<div class=\"scroll\"><table class=\"grid\"><thead><tr><th scope=\"col\" class=\"r\">Wind speed [m/s]</th>");
    for (name, _) in by.values() {
        let _ =
            write!(tbl, "<th scope=\"col\" class=\"r\">{} farthest landing [m]</th><th scope=\"col\" class=\"r\">from wind direction [deg]</th>", esc(name));
    }
    tbl.push_str("</tr></thead><tbody>");
    for s in &speeds {
        let _ = write!(tbl, "<tr><td class=\"r\">{s}</td>");
        for (_, idx) in by.values() {
            let best = idx.iter().filter(|&&i| sp[i] == *s && dist[i].is_finite()).max_by(|&&a, &&b| dist[a].total_cmp(&dist[b]));
            match best {
                Some(&i) => {
                    let _ = write!(tbl, "<td class=\"r\">{:.0}</td><td class=\"r\">{:.0}</td>", dist[i], dr[i]);
                }
                None => tbl.push_str("<td class=\"r\">n/a</td><td class=\"r\">n/a</td>"),
            }
        }
        tbl.push_str("</tr>");
    }
    tbl.push_str("</tbody></table></div>");
    // Farthest landing overall (relevant to a keep-out distance).
    let mut far = String::new();
    for (name, idx) in by.values() {
        if let Some(&i) = idx.iter().filter(|&&i| dist[i].is_finite()).max_by(|&&a, &&b| dist[a].total_cmp(&dist[b])) {
            let _ = write!(
                far,
                "<li>{}: <strong>{:.0} m</strong> from the pad, at wind speed {} m/s from {:.0}\u{b0}.{}</li>",
                esc(name),
                dist[i],
                sp[i],
                dr[i],
                apogee.as_ref().map_or(String::new(), |a| format!(" Apogee {:.0} m.", a[i]))
            );
        }
    }
    if !far.is_empty() {
        let _ = write!(o, "<h3>Farthest landing of the grid</h3><ul>{far}</ul>");
    }
    o += &tbl;
    let mut figs = String::new();
    let nspeeds = speeds.len().max(1);
    for (name, idx) in by.values() {
        let all: Vec<(f64, f64)> = idx.iter().map(|&i| (e[i], n[i])).collect();
        let (k, unit) = unit_scale(&all);
        let color = descent_order(name);
        let mut ch = Chart::new(&format!("Landing points, {name} descent"), &format!("East [{unit}]"), &format!("North [{unit}]"));
        ch.equal = true;
        for (j, s) in speeds.iter().enumerate() {
            let mut ring: Vec<usize> = idx.iter().copied().filter(|&i| sp[i] == *s).collect();
            ring.sort_by(|&a, &b| dr[a].total_cmp(&dr[b]));
            let pts: Vec<(f64, f64)> = ring.iter().map(|&i| (e[i] * k, n[i] * k)).collect();
            let op = 0.3 + 0.7 * (j + 1) as f64 / nspeeds as f64;
            ch.series.push(Series::ring("", color, pts.clone(), op * 0.8));
            ch.series.push(Series::dots(if j == 0 { name.as_str() } else { "" }, color, pts, op));
        }
        ch.marks.push(Mark { x: 0.0, y: Some(0.0), label: "pad".into() });
        figs += &svg::figure(
            &ch,
            &format!(
                "One ring per wind speed ({} m/s to {} m/s, outer rings are stronger winds); one point per wind direction.",
                speeds.first().copied().unwrap_or(0.0),
                speeds.last().copied().unwrap_or(0.0)
            ),
        );
    }
    let _ = write!(o, "<div class=\"figs\">{figs}</div>");
    Ok(o)
}

fn monte_carlo(sum: &Value, dir: &Path) -> Result<String> {
    let descents: Vec<&Value> = sum.get("descents").and_then(Value::as_array).map(|a| a.iter().collect()).unwrap_or_default();
    let mut o = String::from("<h3>Monte Carlo</h3>");
    let _ = write!(
        o,
        "<p>{} samples, seed {}. Each sample draws the uncertain inputs of the configuration and flies the rocket with every descent mode.{}</p>",
        sum.get("samples").and_then(Value::as_u64).map_or("n/a".into(), |v| v.to_string()),
        sum.get("seed").and_then(Value::as_u64).map_or("n/a".into(), |v| v.to_string()),
        if get(sum, "truncate_sigma").is_some() {
            format!(" Draws are clipped at {} sigma.", get(sum, "truncate_sigma").unwrap_or(0.0))
        } else {
            String::new()
        }
    );
    let mut cols = String::new();
    let mut rows: Vec<Vec<String>> = Vec::new();
    let labels = [
        "Samples landed",
        "Failed or not landed",
        "Mean landing point east, north",
        "Apogee mean \u{b1} standard deviation",
        "1\u{3c3} ellipse semi-axes (major, minor)",
        "1\u{3c3} ellipse major axis, bearing",
        "3\u{3c3} ellipse semi-axes (major, minor)",
        "Landing distance, median",
        "Landing distance, 90th percentile",
        "Landing distance, 99th percentile",
        "Landing distance, 99.9th percentile",
        "Largest landing distance",
    ];
    let mut names = Vec::new();
    for d in &descents {
        names.push(get_str(d, "descent").unwrap_or("n/a").to_string());
        let lb = d.get("max_landing_distance_is_lower_bound").and_then(Value::as_bool) == Some(true);
        let e1 = d.get("ellipse_1sigma").cloned().unwrap_or(Value::Null);
        let e3 = d.get("ellipse_3sigma").cloned().unwrap_or(Value::Null);
        rows.push(vec![
            d.get("n").and_then(Value::as_u64).map_or("n/a".into(), |v| v.to_string()),
            format!("{} / {}", d.get("failed").and_then(Value::as_u64).unwrap_or(0), d.get("not_landed").and_then(Value::as_u64).unwrap_or(0)),
            format!("{}, {}", q(get(d, "mean_east"), 0, "m"), q(get(d, "mean_north"), 0, "m")),
            format!("{} \u{b1} {}", q(get(d, "apogee_mean"), 0, "m"), q(get(d, "apogee_std"), 0, "m")),
            format!("{}, {}", q(get(&e1, "semi_major"), 0, "m"), q(get(&e1, "semi_minor"), 0, "m")),
            q(get(&e1, "major_axis_bearing_deg"), 0, "deg"),
            format!("{}, {}", q(get(&e3, "semi_major"), 0, "m"), q(get(&e3, "semi_minor"), 0, "m")),
            q(get(d, "landing_distance_p50"), 0, "m"),
            q(get(d, "landing_distance_p90"), 0, "m"),
            q(get(d, "landing_distance_p99"), 0, "m"),
            q(get(d, "landing_distance_p999"), 0, "m"),
            format!("{}{}", q(get(d, "max_landing_distance"), 0, "m"), if lb { " (lower bound)" } else { "" }),
        ]);
    }
    if !rows.is_empty() {
        cols.push_str("<div class=\"scroll\"><table class=\"grid\"><thead><tr><th scope=\"col\"></th>");
        for n in &names {
            let _ = write!(cols, "<th scope=\"col\" class=\"r\">{} descent</th>", esc(n));
        }
        cols.push_str("</tr></thead><tbody>");
        for (i, l) in labels.iter().enumerate() {
            let _ = write!(cols, "<tr><th scope=\"row\">{}</th>", esc(l));
            for r in &rows {
                let _ = write!(cols, "<td class=\"r\">{}</td>", esc(&r[i]));
            }
            cols.push_str("</tr>");
        }
        cols.push_str("</tbody></table></div>");
    }
    o += &cols;
    if let Some(c) = Csv::load(&dir.join("dispersion_mc.csv"), &["descent", "status", "landing_east", "landing_north"])? {
        if let (Some(desc), Some(st), Some(e), Some(n)) = (c.text("descent"), c.text("status"), c.num("landing_east"), c.num("landing_north")) {
            let mut figs = String::new();
            for d in &descents {
                let name = get_str(d, "descent").unwrap_or("");
                let pts: Vec<(f64, f64)> =
                    (0..c.rows()).filter(|&i| desc[i] == name && st[i] == "ok" && e[i].is_finite() && n[i].is_finite()).map(|i| (e[i], n[i])).collect();
                if pts.is_empty() {
                    continue;
                }
                let (k, unit) = unit_scale(&pts);
                let color = descent_order(name);
                let scaled: Vec<(f64, f64)> = pts.iter().map(|p| (p.0 * k, p.1 * k)).collect();
                let mut ch = Chart::new(&format!("Landing scatter, {name} descent"), &format!("East [{unit}]"), &format!("North [{unit}]"));
                ch.equal = true;
                ch.series.push(Series::dots(&format!("{} landings", pts.len()), color, scaled, 0.35));
                let (me, mn) = (get(d, "mean_east").unwrap_or(0.0) * k, get(d, "mean_north").unwrap_or(0.0) * k);
                for (key, label, col, op) in [("ellipse_1sigma", "1\u{3c3} ellipse", 3, 1.0), ("ellipse_3sigma", "3\u{3c3} ellipse", 2, 1.0)] {
                    if let Some(el) = d.get(key) {
                        if let (Some(a), Some(b), Some(br)) = (get(el, "semi_major"), get(el, "semi_minor"), get(el, "major_axis_bearing_deg")) {
                            ch.series.push(Series {
                                name: label.into(),
                                pts: svg::ellipse(me, mn, a * k, b * k, br),
                                color: col,
                                kind: svg::Kind::Line,
                                opacity: op,
                            });
                        }
                    }
                }
                ch.marks.push(Mark { x: 0.0, y: Some(0.0), label: "pad".into() });
                if get(d, "mean_east").is_some() {
                    ch.marks.push(Mark { x: me, y: Some(mn), label: "mean".into() });
                }
                figs += &svg::figure(&ch, "One point per sample that landed, north up. Ellipses are computed from the covariance of the landing points.");
            }
            let _ = write!(o, "<div class=\"figs\">{figs}</div>");
        }
    }
    Ok(o)
}

// ---------------------------------------------------------------- warnings, plots, files

fn warnings_section(sources: &[(&str, &Option<Value>)]) -> String {
    let mut seen: Vec<String> = Vec::new();
    let mut items = String::new();
    for (_, v) in sources {
        for w in v.as_ref().and_then(|v| v.get("warnings")).and_then(Value::as_array).into_iter().flatten().filter_map(Value::as_str) {
            if !seen.iter().any(|s| s == w) {
                seen.push(w.to_string());
                items += &warning_item(w);
            }
        }
    }
    if items.is_empty() {
        let any = sources.iter().any(|(_, v)| v.is_some());
        return if any { "<p>The run recorded no warnings.</p>".into() } else { String::new() };
    }
    let from: Vec<&str> = sources.iter().filter(|(_, v)| v.is_some()).map(|(n, _)| *n).collect();
    format!("<p class=\"sub\">Recorded in {}. The code links to its explanation (Japanese).</p><ul class=\"warn\">{items}</ul>", esc(&from.join(", ")))
}

fn mtime(p: &Path) -> Option<std::time::SystemTime> {
    std::fs::metadata(p).and_then(|m| m.modified()).ok()
}

/// Plots of `make plot` that the report does not draw itself, when they are not older than the results.
fn plots_section(dir: &Path) -> String {
    const OWN: [&str; 4] = ["geometry.png", "aero.png", "trajectory.png", "dispersion.png"];
    let newest = ["summary.json", "dispersion_meta.json", "dispersion_summary.json", "aero_table.json"].iter().filter_map(|f| mtime(&dir.join(f))).max();
    let Ok(rd) = std::fs::read_dir(dir.join("plots")) else { return String::new() };
    let mut files: Vec<PathBuf> = rd
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "png") && p.file_name().is_some_and(|n| !OWN.iter().any(|o| n == *o)))
        .collect();
    files.sort();
    let mut o = String::new();
    for p in files {
        if newest.is_some_and(|n| mtime(&p).is_some_and(|m| m < n)) {
            continue;
        }
        let Ok(bytes) = std::fs::read(&p) else { continue };
        let name = p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        let _ = write!(
            o,
            "<figure><img class=\"plot\" alt=\"{0}\" src=\"data:image/png;base64,{1}\"><figcaption>plots/{0}</figcaption></figure>",
            esc(&name),
            base64(&bytes)
        );
    }
    if o.is_empty() {
        String::new()
    } else {
        format!(
            "<details><summary>Show the plots written by the plot command ({} kB embedded)</summary><div class=\"figs\">{o}</div></details>",
            o.len() / 1024
        )
    }
}

fn files_section(dir: &Path, ctx: &Context) -> String {
    let link = ctx.report_dir == ctx.dir;
    let known = [
        ("geometry.json", "extracted geometry"),
        ("profile.csv", "body radius and fin height along the axis"),
        ("aero_table.csv", "aerodynamic coefficient table"),
        ("aero_table.json", "table metadata"),
        ("aero_drag.csv", "drag build-up by Mach number"),
        ("trajectory.csv", "flight time history"),
        ("summary.json", "flight summary"),
        ("dispersion.csv", "wind-grid landing points"),
        ("dispersion_meta.json", "wind-grid metadata"),
        ("dispersion_mc.csv", "Monte Carlo samples"),
        ("dispersion_summary.json", "Monte Carlo statistics"),
        ("ignisyeet.kml", "Google Earth file"),
    ];
    let mut o = String::from("<div class=\"scroll\"><table class=\"grid\"><thead><tr><th scope=\"col\">File</th><th scope=\"col\">Content</th><th scope=\"col\" class=\"r\">Size</th></tr></thead><tbody>");
    let mut n = 0;
    for (f, what) in known {
        if let Ok(m) = std::fs::metadata(dir.join(f)) {
            n += 1;
            let name = if link { format!("<a href=\"{0}\">{0}</a>", esc(f)) } else { format!("<code>{}</code>", esc(f)) };
            let _ = write!(o, "<tr><td>{name}</td><td>{}</td><td class=\"r\">{}</td></tr>", esc(what), size_text(m.len()));
        }
    }
    o.push_str("</tbody></table></div>");
    if n == 0 {
        String::new()
    } else {
        o
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn utc_formatting() {
        assert_eq!(format_utc(0), "1970-01-01 00:00 UTC");
        assert_eq!(format_utc(1_700_000_000), "2023-11-14 22:13 UTC");
        assert_eq!(format_utc(951_782_400), "2000-02-29 00:00 UTC");
    }

    #[test]
    fn base64_matches_the_rfc_vectors() {
        assert_eq!(base64(b""), "");
        assert_eq!(base64(b"f"), "Zg==");
        assert_eq!(base64(b"fo"), "Zm8=");
        assert_eq!(base64(b"foo"), "Zm9v");
        assert_eq!(base64(b"foobar"), "Zm9vYmFy");
    }

    #[test]
    fn csv_fields_with_quotes_and_commas() {
        assert_eq!(split_csv(r#"a,"b,c","d ""e""",,f"#), vec!["a", "b,c", "d \"e\"", "", "f"]);
    }

    #[test]
    fn warnings_link_their_code_and_escape_the_text() {
        let w = warning_item("[IY-O514] apogee <script>alert(1)</script> & more");
        assert!(w.contains("errors/cli.html#iy-o514") && w.contains("&lt;script&gt;") && !w.contains("<script>"), "{w}");
        assert!(warning_item("plain <b>").contains("plain &lt;b&gt;"));
    }

    #[test]
    fn missing_values_read_n_a() {
        assert_eq!(q(None, 1, "m"), "n/a");
        assert_eq!(q(Some(f64::NAN), 1, "m"), "n/a");
        assert_eq!(q(Some(1.25), 1, "m"), "1.2\u{a0}m");
    }

    use crate::test_tmp::TempDir;

    fn sample_cfg() -> Config {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples");
        Config::from_value(toml::from_str(include_str!("../../../examples/sample.toml")).unwrap(), Some(&dir)).unwrap()
    }

    fn ctx<'a>(dir: &Path, config_path: &'a Path) -> Context<'a> {
        Context {
            config_path,
            version: "0.0.0-test",
            epoch: 1_700_000_000,
            hashes: Hashes { flight: "aaaa".into(), wind_grid: "bbbb".into(), monte_carlo: "cccc".into() },
            motor: None,
            dir: dir.to_path_buf(),
            report_dir: dir.to_path_buf(),
            embed_plots: true,
        }
    }

    fn summary(extra: &str) -> String {
        format!(
            r#"{{"apogee":4440.476,"apogee_time":26.1,"apogee_east":-630.0,"apogee_north":359.0,"max_speed":518.75,"max_mach":1.5423,"max_dyn_pressure":152452.4,
            "burnout_time":3.1,"rail_exit_time":0.25,"rail_exit_speed":45.66,"stability_at_rail_exit":1.99,"min_stability_cal":1.99,"static_margin_min_cal":1.99,
            "static_margin_min_t":0.25,"static_margin_min_mach":0.13,"stable":true,"landed":true,"landing_time":685.0,"landing_east":-655.6,"landing_north":-7707.7,
            "landing_lat":34.665,"landing_lon":139.413,"landing_distance":7735.5,"landing_speed":7.06,"descent":"parachute","wind_speed":4.0,"wind_direction_deg":0.0,
            "wind_model":"power","max_time":3600.0,"config_hash":"aaaa","warnings":["[IY-O514] apogee above the profile"]{extra}}}"#
        )
    }

    fn write_trajectory(dir: &Path) {
        let mut s = String::from("t,phase,east,north,up,vel_e,vel_n,vel_u,airspeed,mach,stability_cal,x_cg,x_cp\n");
        for i in 0..60 {
            let t = i as f64 * 0.5;
            let up = (4440.0 * (1.0 - ((t - 26.0) / 26.0).powi(2))).max(0.0);
            let vu: f64 = if t < 26.0 { 100.0 } else { -8.0 };
            let ph = if t < 0.5 {
                "rail"
            } else if t < 26.0 {
                "free"
            } else {
                "parachute"
            };
            s += &format!("{t},{ph},{},{},{up},0,0,{vu},{},{},2.5,0.959,1.161\n", -t * 3.0, -t * 10.0, vu.abs(), vu.abs() / 340.0);
        }
        std::fs::write(dir.join("trajectory.csv"), s).unwrap();
    }

    fn write_full(dir: &Path) {
        std::fs::write(dir.join("summary.json"), summary("")).unwrap();
        write_trajectory(dir);
        std::fs::write(
            dir.join("geometry.json"),
            r#"{"length":1.5,"ref_radius":0.05,"ref_area":0.00785,"fins":{"count":4,"root_chord":0.2,"tip_chord":0.1,"span":0.1,"sweep":0.08,"thickness":0.004,"x_le_root":1.3,"cant_deg":0.0},"nose":{"length":0.29,"base_radius":0.05},"warnings":[]}"#,
        )
        .unwrap();
        std::fs::write(dir.join("profile.csv"), "x,r,fin_span\n0.0,0.0,0.0\n0.3,0.05,0.0\n1.3,0.05,0.0\n1.4,0.05,0.1\n1.5,0.05,0.0\n").unwrap();
        std::fs::write(dir.join("aero_drag.csv"), "mach,cd_off,cd_on,cna,xcp\n0,0.48,0.47,11.6,1.16\n1,0.7,0.69,12.5,1.18\n2,0.55,0.54,10.0,1.14\n").unwrap();
        std::fs::write(
            dir.join("aero_table.json"),
            r#"{"kind":"axisymmetric","machs":[0,1,2],"alphas_deg":[0,5],"ref_area":0.00785,"ref_diameter":0.1,"length":1.5,"moment_ref":"nose_tip"}"#,
        )
        .unwrap();
        let mut d = String::from("descent,wind_speed,wind_direction_deg,landing_east,landing_north,landing_lat,landing_lon,landing_distance,landing_time,landing_speed,apogee,landed,error\n");
        for desc in ["ballistic", "parachute"] {
            for sp in [2, 4] {
                for dir_deg in [0, 90, 180, 270] {
                    let k = if desc == "ballistic" { 300.0 } else { 2000.0 } * sp as f64;
                    let (e, n) = match dir_deg {
                        0 => (0.0, -k),
                        90 => (-k, 0.0),
                        180 => (0.0, k),
                        _ => (k, 0.0),
                    };
                    d += &format!("{desc},{sp},{dir_deg},{e},{n},34.0,139.0,{k},60,8,4000,true,\n");
                }
            }
        }
        std::fs::write(dir.join("dispersion.csv"), d).unwrap();
        std::fs::write(
            dir.join("dispersion_meta.json"),
            r#"{"cases":16,"failed":0,"errored":0,"config_hash":"bbbb","warnings":["[IY-O514] apogee above the profile"]}"#,
        )
        .unwrap();
    }

    #[test]
    fn missing_or_empty_output_directory_is_a_coded_error() {
        let d = TempDir::new("report_empty");
        let cfg = sample_cfg();
        let cp = Path::new("c.toml");
        let e = build(&cfg, &ctx(&d.join("nope"), cp)).err().unwrap().to_string();
        assert!(e.starts_with("[IY-O041]"), "{e}");
        let e = build(&cfg, &ctx(&d, cp)).err().unwrap().to_string();
        assert!(e.starts_with("[IY-O042]"), "{e}");
        std::fs::write(d.join("summary.json"), "{not json").unwrap();
        let e = format!("{:#}", build(&cfg, &ctx(&d, cp)).err().unwrap());
        assert!(e.contains("[IY-O043]"), "{e}");
    }

    #[test]
    fn a_report_of_a_full_run_has_every_section_and_the_key_numbers() {
        let d = TempDir::new("report_full");
        write_full(&d);
        let cfg = sample_cfg();
        let cp = Path::new("sample.toml");
        let r = build(&cfg, &ctx(&d, cp)).unwrap();
        assert_eq!(r.sections, vec!["inputs", "geometry", "aero", "flight", "dispersion", "warnings", "files"]);
        assert!(r.mismatches.is_empty());
        for needle in [
            "4440.5\u{a0}m",
            "518.8\u{a0}m/s (Mach 1.542)",
            "7736\u{a0}m",
            "1.99\u{a0}cal",
            "2023-11-14 22:13 UTC",
            "<svg",
            "Ground track",
            "8000",
            "600",
            "IY-O514",
            "href=\"summary.json\"",
        ] {
            assert!(r.html.contains(needle), "missing {needle:?}");
        }
        // Self-contained: no script, no external resource.
        assert!(!r.html.contains("<script") && !r.html.contains("src=\"http") && !r.html.contains("<link"));
        assert_eq!(r.html.matches("<svg").count(), r.html.matches("</svg>").count());
        // Same inputs, same bytes.
        assert_eq!(r.html, build(&cfg, &ctx(&d, cp)).unwrap().html);
    }

    #[test]
    fn sections_without_files_are_left_out() {
        let d = TempDir::new("report_sim_only");
        std::fs::write(d.join("summary.json"), summary("")).unwrap();
        write_trajectory(&d);
        let cfg = sample_cfg();
        let r = build(&cfg, &ctx(&d, Path::new("sample.toml"))).unwrap();
        assert_eq!(r.sections, vec!["inputs", "flight", "warnings", "files"]);
        assert!(r.html.contains("Height against time") && !r.html.contains("Landing dispersion") && !r.html.contains("Drag coefficient"));
        // Without the trajectory the flight section has its table only.
        std::fs::remove_file(d.join("trajectory.csv")).unwrap();
        let r = build(&cfg, &ctx(&d, Path::new("sample.toml"))).unwrap();
        assert!(r.sections.contains(&"flight") && !r.html.contains("Height against time"));
        // Dispersion alone.
        let d2 = TempDir::new("report_dispersion_only");
        write_full(&d2);
        for f in ["summary.json", "trajectory.csv", "geometry.json", "profile.csv", "aero_drag.csv", "aero_table.json"] {
            std::fs::remove_file(d2.join(f)).unwrap();
        }
        let r = build(&cfg, &ctx(&d2, Path::new("sample.toml"))).unwrap();
        assert_eq!(r.sections, vec!["inputs", "dispersion", "warnings", "files"]);
    }

    #[test]
    fn text_from_the_results_and_the_configuration_is_escaped() {
        let d = TempDir::new("report_escape");
        std::fs::write(d.join("summary.json"), summary("").replace("apogee above the profile", "<script>alert(1)</script> & <img src=x onerror=y>")).unwrap();
        let cfg = sample_cfg();
        let cp = Path::new("<blink>\"x\".toml");
        let r = build(&cfg, &ctx(&d, cp)).unwrap();
        assert!(!r.html.contains("<script") && !r.html.contains("<img") && !r.html.contains("<blink>"), "unescaped markup");
        assert!(r.html.contains("&lt;script&gt;alert(1)&lt;/script&gt; &amp; &lt;img src=x onerror=y&gt;"));
        assert!(r.html.contains("&lt;blink&gt;&quot;x&quot;.toml"));
    }

    #[test]
    fn a_failed_flight_shows_the_failure_and_changed_configurations_are_noted() {
        let d = TempDir::new("report_failed");
        std::fs::write(d.join("summary.json"), r#"{"failed":true,"error":"[IY-S001] boom <b>"}"#).unwrap();
        let cfg = sample_cfg();
        let r = build(&cfg, &ctx(&d, Path::new("sample.toml"))).unwrap();
        assert!(r.html.contains("The flight simulation failed") && r.html.contains("boom &lt;b&gt;"));
        std::fs::write(d.join("summary.json"), summary("")).unwrap();
        let mut c = ctx(&d, Path::new("sample.toml"));
        c.hashes.flight = "zzzz".into();
        let r = build(&cfg, &c).unwrap();
        assert_eq!(r.mismatches.len(), 1);
        assert!(r.html.contains("The configuration has changed since these results were computed"));
    }

    #[test]
    fn monte_carlo_results_get_ellipses_and_percentiles() {
        let d = TempDir::new("report_mc");
        std::fs::write(
            d.join("dispersion_summary.json"),
            r#"{"samples":3,"seed":1,"config_hash":"cccc","warnings":[],"descents":[{"descent":"ballistic","n":3,"failed":0,"not_landed":0,"mean_east":-1000.0,"mean_north":300.0,"apogee_mean":4400.0,"apogee_std":400.0,"ellipse_1sigma":{"semi_major":200.0,"semi_minor":100.0,"major_axis_bearing_deg":130.0},"ellipse_3sigma":{"semi_major":600.0,"semi_minor":300.0,"major_axis_bearing_deg":130.0},"landing_distance_p50":1100.0,"landing_distance_p90":1400.0,"landing_distance_p99":1800.0,"landing_distance_p999":1990.0,"max_landing_distance":2141.0,"max_landing_distance_is_lower_bound":false}]}"#,
        )
        .unwrap();
        std::fs::write(
            d.join("dispersion_mc.csv"),
            "sample,descent,status,landing_east,landing_north\n0,ballistic,ok,-900,200\n1,ballistic,ok,-1100,400\n2,ballistic,failed,,\n",
        )
        .unwrap();
        let r = build(&sample_cfg(), &ctx(&d, Path::new("sample.toml"))).unwrap();
        assert!(r.sections.contains(&"dispersion"));
        for needle in ["2141\u{a0}m", "1990\u{a0}m", "2 landings", "1\u{3c3} ellipse", "Farthest Monte Carlo landing"] {
            assert!(r.html.contains(needle), "missing {needle:?}");
        }
    }
}
