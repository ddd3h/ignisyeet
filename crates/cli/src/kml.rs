//! The single KML (Google Earth) document `ignisyeet.kml`: launch site, flight trajectory and landing dispersion.
//!
//! Parts that a command did not just compute are read back from the CSV/JSON files in the output directory.
//!
//! Altitudes: `Sample::alt` is the geodetic height above the WGS84 ellipsoid in both earth models
//! (flat: `geo::enu_to_lla` of the launch-site ENU offset; ecef: `ecef_to_lla`), starting from
//! `launch.altitude`. KML `absolute` altitudes are nominally above mean sea level (EGM96); the geoid
//! undulation is not modelled, so `launch.altitude` should be the MSL height of the site, which is
//! what the sim treats as the starting height. Colours are KML `aabbggrr` hex.

use sim::dispersion::{Case, DescentStats, Ellipse, McRow, Perturbation, PointStats};
use sim::flight::{Phase, Sample};
use sim::{Descent, Launch, Summary};
use std::collections::HashMap;
use std::fmt::Write;
use std::path::Path;

/// Escapes XML special characters.
pub fn esc(s: &str) -> String {
    let mut o = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => o.push_str("&amp;"),
            '<' => o.push_str("&lt;"),
            '>' => o.push_str("&gt;"),
            '"' => o.push_str("&quot;"),
            '\'' => o.push_str("&apos;"),
            c => o.push(c),
        }
    }
    o
}

/// KML colour `aabbggrr` from 8-bit r, g, b and alpha.
pub fn color(r: u8, g: u8, b: u8, a: u8) -> String {
    format!("{a:02x}{b:02x}{g:02x}{r:02x}")
}

/// One `lon,lat,alt` tuple (KML order).
fn coord(lat: f64, lon: f64, alt: f64) -> String {
    format!("{lon:.8},{lat:.8},{alt:.2}")
}

/// HTML description (CDATA); every line is escaped and lines are joined with `<br>`.
fn desc(lines: &[String]) -> String {
    let body: Vec<String> = lines.iter().map(|l| esc(l)).collect();
    format!("<description><![CDATA[{}]]></description>", body.join("<br>"))
}

fn line_style(c: &str, width: f64) -> String {
    format!("<Style><LineStyle><color>{c}</color><width>{width}</width></LineStyle></Style>")
}

fn icon_style(c: &str, scale: f64, icon: &str) -> String {
    format!("<Style><IconStyle><color>{c}</color><scale>{scale}</scale><Icon><href>{icon}</href></Icon></IconStyle><LabelStyle><scale>{}</scale></LabelStyle></Style>", if scale < 0.5 { 0 } else { 1 })
}

const PIN: &str = "http://maps.google.com/mapfiles/kml/paddle/wht-blank.png";
const DOT: &str = "http://maps.google.com/mapfiles/kml/shapes/shaded_dot.png";

fn point_placemark(name: &str, lines: &[String], style: &str, lat: f64, lon: f64, alt: f64, ground: bool) -> String {
    let mode = if ground { "clampToGround" } else { "absolute" };
    format!(
        "<Placemark><name>{}</name>{}{style}<Point><altitudeMode>{mode}</altitudeMode><coordinates>{}</coordinates></Point></Placemark>\n",
        esc(name),
        desc(lines),
        coord(lat, lon, if ground { 0.0 } else { alt })
    )
}

fn document(name: &str, description: &[String], body: &str) -> String {
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<kml xmlns=\"http://www.opengis.net/kml/2.2\">\n<Document>\n<name>{}</name>\n{}\n{body}</Document>\n</kml>\n",
        esc(name),
        desc(description)
    )
}

fn folder(name: &str, description: &[String], body: &str) -> String {
    format!("<Folder><name>{}</name>{}\n{body}</Folder>\n", esc(name), desc(description))
}

/// The flight: trajectory samples and their summary.
pub struct Flight {
    pub samples: Vec<Sample>,
    pub summary: Summary,
}

/// Wind-grid dispersion: one summary per case.
pub struct WindGrid {
    pub cases: Vec<Case>,
    pub results: Vec<Summary>,
}

/// Monte Carlo dispersion: every sample row and the per-descent-mode statistics.
pub struct MonteCarlo {
    pub rows: Vec<McRow>,
    pub stats: Vec<DescentStats>,
}

/// The parts of `ignisyeet.kml`; a part that is `None` has no folder.
#[derive(Default)]
pub struct Parts {
    pub flight: Option<Flight>,
    pub wind_grid: Option<WindGrid>,
    pub monte_carlo: Option<MonteCarlo>,
}

/// The combined document: folders "Launch site", "Flight", "Dispersion - wind grid" and "Dispersion - Monte Carlo".
pub fn combined(site: &Launch, parts: &Parts) -> String {
    let mut body = folder("Launch site", &[], &site_placemark(site));
    let mut d = vec!["IgnisYeet results. Altitudes are WGS84 geodetic heights (absolute); landing and dispersion figures are clamped to the ground.".to_string()];
    if let Some(f) = &parts.flight {
        body.push_str(&flight_folder(site, f, &mut d));
    }
    if let Some(w) = &parts.wind_grid {
        body.push_str(&wind_grid_folder(&w.cases, &w.results, &mut d));
    }
    if let Some(m) = &parts.monte_carlo {
        body.push_str(&monte_carlo_folder(site, &m.rows, &m.stats, &mut d));
    }
    document("IgnisYeet", &d, &body)
}

fn site_placemark(site: &Launch) -> String {
    point_placemark(
        "Launch site",
        &[format!("lat {:.6}, lon {:.6}, alt {:.1} m", site.latitude, site.longitude, site.altitude), format!("rail {:.1} m, elevation {:.1} deg, azimuth {:.1} deg", site.rail_length, site.elevation_deg, site.azimuth_deg)],
        &icon_style(&color(255, 255, 255, 255), 1.1, PIN),
        site.latitude,
        site.longitude,
        site.altitude,
        true,
    )
}

#[derive(Clone, Copy, PartialEq)]
enum Leg {
    Rail,
    Powered,
    Coast,
    Parachute,
}

impl Leg {
    fn of(s: &Sample) -> Leg {
        match s.phase {
            Phase::Rail => Leg::Rail,
            Phase::Parachute => Leg::Parachute,
            Phase::Free if s.thrust > 0.0 => Leg::Powered,
            Phase::Free => Leg::Coast,
        }
    }
    fn name(self) -> &'static str {
        match self {
            Leg::Rail => "Rail",
            Leg::Powered => "Powered flight",
            Leg::Coast => "Coast",
            Leg::Parachute => "Parachute descent",
        }
    }
    fn rgb(self) -> (u8, u8, u8) {
        match self {
            Leg::Rail => (255, 220, 0),
            Leg::Powered => (255, 40, 40),
            Leg::Coast => (0, 128, 255),
            Leg::Parachute => (0, 200, 0),
        }
    }
}

fn speed(s: &Sample) -> f64 {
    (s.vel_e * s.vel_e + s.vel_n * s.vel_n + s.vel_u * s.vel_u).sqrt()
}

fn event(name: &str, s: &Sample, site: &Launch, extra: &[String]) -> String {
    let mut lines = vec![format!("t = {:.2} s", s.t), format!("altitude {:.1} m MSL ({:.1} m above launch)", s.alt, s.alt - site.altitude), format!("speed {:.1} m/s", speed(s))];
    lines.extend_from_slice(extra);
    point_placemark(name, &lines, &icon_style(&color(255, 255, 255, 255), 0.9, PIN), s.lat, s.lon, s.alt, false)
}

/// Folder "Flight": per-phase flight path (absolute altitude), event placemarks and landing.
fn flight_folder(site: &Launch, f: &Flight, doc: &mut Vec<String>) -> String {
    let (samples, sum, descent) = (&f.samples[..], &f.summary, f.summary.descent);
    let mut body = String::new();

    // Split into legs; consecutive legs share their boundary sample so the path has no gaps.
    let mut i = 0;
    while i < samples.len() {
        let leg = Leg::of(&samples[i]);
        let start = i.saturating_sub(1);
        let mut j = i;
        while j + 1 < samples.len() && Leg::of(&samples[j + 1]) == leg {
            j += 1;
        }
        let pts = &samples[start..=j];
        if pts.len() >= 2 {
            let (r, g, b) = leg.rgb();
            let coords: Vec<String> = pts.iter().map(|s| coord(s.lat, s.lon, s.alt)).collect();
            let _ = writeln!(
                body,
                "<Placemark><name>{}</name>{}{}<LineString><tessellate>1</tessellate><altitudeMode>absolute</altitudeMode><coordinates>{}</coordinates></LineString></Placemark>",
                leg.name(),
                desc(&[format!("t = {:.2} s to {:.2} s", pts[0].t, pts[pts.len() - 1].t)]),
                line_style(&color(r, g, b, 255), 3.0),
                coords.join(" ")
            );
        }
        i = j + 1;
    }

    if let Some(s) = samples.iter().find(|s| s.phase != Phase::Rail) {
        body.push_str(&event("Rail exit", s, site, &[format!("stability {:.2} cal", sum.stability_at_rail_exit)]));
    }
    if sum.burnout_time.is_finite() {
        if let Some(s) = samples.iter().find(|s| s.t >= sum.burnout_time) {
            body.push_str(&event("Burnout", s, site, &[]));
        }
    }
    if let Some(s) = samples.iter().max_by(|a, b| a.alt.total_cmp(&b.alt)) {
        body.push_str(&event("Apogee", s, site, &[format!("apogee {:.1} m above launch", sum.apogee)]));
    }
    if let Some(s) = samples.iter().find(|s| s.phase == Phase::Parachute) {
        body.push_str(&event("Parachute deploy", s, site, &[]));
    }
    body.push_str(&point_placemark(
        "Landing",
        &[
            format!("t = {:.1} s", sum.landing_time),
            format!("distance from launch {:.1} m (E {:.1} m, N {:.1} m)", sum.landing_distance, sum.landing_east, sum.landing_north),
            format!("impact speed {:.1} m/s", sum.landing_speed),
        ],
        &icon_style(&color(255, 255, 255, 255), 1.0, PIN),
        sum.landing_lat,
        sum.landing_lon,
        0.0,
        true,
    ));

    let legend = [
        format!("{descent:?} descent, wind {:.1} m/s from {:.0} deg", sum.wind_speed, sum.wind_direction_deg),
        "Legend: yellow = rail, red = powered flight, blue = coast, green = parachute descent.".into(),
    ];
    doc.push(format!("Flight: {}", legend[1]));
    folder("Flight", &legend, &body)
}

/// Linear blend of two RGB colours, t in [0, 1].
fn blend(light: (u8, u8, u8), dark: (u8, u8, u8), t: f64) -> (u8, u8, u8) {
    let f = |a: u8, b: u8| (a as f64 + (b as f64 - a as f64) * t).round() as u8;
    (f(light.0, dark.0), f(light.1, dark.1), f(light.2, dark.2))
}

fn mode_palette(d: Descent) -> ((u8, u8, u8), (u8, u8, u8)) {
    if d == Descent::Ballistic {
        ((170, 205, 255), (10, 40, 150))
    } else {
        ((255, 215, 160), (190, 70, 0))
    }
}

fn mode_name(d: Descent) -> &'static str {
    if d == Descent::Ballistic {
        "ballistic"
    } else {
        "parachute"
    }
}

fn ring_coords(pts: &[(f64, f64)]) -> String {
    pts.iter().map(|&(lat, lon)| coord(lat, lon, 0.0)).collect::<Vec<_>>().join(" ")
}

fn ground_line(name: &str, lines: &[String], c: &str, width: f64, pts: &[(f64, f64)]) -> String {
    format!(
        "<Placemark><name>{}</name>{}{}<LineString><tessellate>1</tessellate><altitudeMode>clampToGround</altitudeMode><coordinates>{}</coordinates></LineString></Placemark>\n",
        esc(name),
        desc(lines),
        line_style(c, width),
        ring_coords(pts)
    )
}

/// Folder "Dispersion - wind grid": a folder per descent mode, a sub-folder per wind speed.
fn wind_grid_folder(cases: &[Case], results: &[Summary], doc: &mut Vec<String>) -> String {
    let mut body = String::new();
    let mut modes: Vec<Descent> = Vec::new();
    for c in cases {
        if !modes.contains(&c.descent) {
            modes.push(c.descent);
        }
    }
    for &m in &modes {
        let mut speeds: Vec<f64> = Vec::new();
        for c in cases.iter().filter(|c| c.descent == m) {
            if !speeds.contains(&c.wind_speed) {
                speeds.push(c.wind_speed);
            }
        }
        let (light, dark) = mode_palette(m);
        let _ = writeln!(body, "<Folder><name>{}</name>", mode_name(m));
        for (k, &ws) in speeds.iter().enumerate() {
            let t = if speeds.len() > 1 { k as f64 / (speeds.len() - 1) as f64 } else { 1.0 };
            let (r, g, b) = blend(light, dark, t);
            let c = color(r, g, b, 255);
            let mut rows: Vec<&Summary> = cases.iter().zip(results).filter(|(c, _)| c.descent == m && c.wind_speed == ws).map(|(_, s)| s).collect();
            rows.sort_by(|a, b| a.wind_direction_deg.total_cmp(&b.wind_direction_deg));
            let _ = writeln!(body, "<Folder><name>{} m/s</name>", ws);
            let mut ring: Vec<(f64, f64)> = rows.iter().map(|s| (s.landing_lat, s.landing_lon)).collect();
            if let Some(&first) = ring.first() {
                ring.push(first);
            }
            body.push_str(&ground_line(&format!("{ws} m/s landing locus"), &[format!("{} landings in wind-direction order", rows.len())], &c, 2.5, &ring));
            for s in &rows {
                body.push_str(&point_placemark(
                    &format!("{} m/s from {:.0}\u{b0}", s.wind_speed, s.wind_direction_deg),
                    &[
                        format!("{} descent", mode_name(m)),
                        format!("landing E {:.1} m, N {:.1} m ({:.1} m from launch)", s.landing_east, s.landing_north, s.landing_distance),
                        format!("landing t = {:.1} s, speed {:.1} m/s", s.landing_time, s.landing_speed),
                        format!("apogee {:.1} m at {:.2} s", s.apogee, s.apogee_time),
                    ],
                    &icon_style(&c, 0.7, DOT),
                    s.landing_lat,
                    s.landing_lon,
                    0.0,
                    true,
                ));
            }
            body.push_str("</Folder>\n");
        }
        body.push_str("</Folder>\n");
    }
    let d = [
        "Wind-grid landing dispersion. One folder per descent mode and wind speed; the outline joins the landing points in wind-direction order.".to_string(),
        "Ballistic: light to dark blue with increasing wind speed. Parachute: light to dark orange.".to_string(),
    ];
    doc.push(format!("Wind grid: {}", d[1]));
    folder("Dispersion \u{2013} wind grid", &d, &body)
}

/// Closed ring (n + 1 vertices, first repeated) of the ellipse around a landing-point mean, as (lat, lon).
/// `mean_east`/`mean_north` are ENU offsets [m] from the launch site `origin`; the major axis points
/// along `bearing` (clockwise from north).
pub fn ellipse_ring(origin: (f64, f64, f64), mean_east: f64, mean_north: f64, e: &Ellipse, n: usize) -> Vec<(f64, f64)> {
    let b = e.major_axis_bearing_deg.to_radians();
    let (d1, d2) = ((b.sin(), b.cos()), (b.cos(), -b.sin()));
    let mut ring: Vec<(f64, f64)> = (0..n)
        .map(|i| {
            let p = std::f64::consts::TAU * i as f64 / n as f64;
            let (a, c) = (e.semi_major * p.cos(), e.semi_minor * p.sin());
            let east = mean_east + a * d1.0 + c * d2.0;
            let north = mean_north + a * d1.1 + c * d2.1;
            let (lat, lon, _) = sim::geo::enu_to_lla(origin, geom::Vec3::new(east, north, 0.0));
            (lat, lon)
        })
        .collect();
    ring.push(ring[0]);
    ring
}

/// Folder "Dispersion - Monte Carlo": per descent mode landing points, mean, 1-sigma and 3-sigma ellipses.
fn monte_carlo_folder(site: &Launch, rows: &[McRow], stats: &[DescentStats], doc: &mut Vec<String>) -> String {
    let origin = (site.latitude, site.longitude, site.altitude);
    let mut body = String::new();
    for st in stats {
        let m = st.descent;
        let (light, dark) = mode_palette(m);
        let (mid, c3) = (blend(light, dark, 0.8), blend(light, dark, 0.45));
        let pt = icon_style(&color(mid.0, mid.1, mid.2, 200), 0.35, DOT);
        let _ = writeln!(body, "<Folder><name>{}</name>", mode_name(m));
        let _ = writeln!(body, "{}", desc(&[format!("{} successful samples, {} failed (excluded)", st.landing.n, st.failed)]));
        if st.landing.n > 0 {
            let l = &st.landing;
            let (lat, lon, _) = sim::geo::enu_to_lla(origin, geom::Vec3::new(l.mean_east, l.mean_north, 0.0));
            body.push_str(&point_placemark(
                "Mean landing",
                &[format!("E {:.1} m, N {:.1} m from launch", l.mean_east, l.mean_north), format!("{} samples", l.n)],
                &icon_style(&color(255, 255, 255, 255), 1.0, PIN),
                lat,
                lon,
                0.0,
                true,
            ));
            for (k, name) in [(&l.ellipse_1sigma, "1-sigma ellipse"), (&l.ellipse_3sigma, "3-sigma ellipse")] {
                let ring = ellipse_ring(origin, l.mean_east, l.mean_north, k, 72);
                let col = if name.starts_with('1') { mid } else { c3 };
                body.push_str(&ground_line(
                    name,
                    &[format!("semi-axes {:.1} x {:.1} m, major axis bearing {:.0} deg", k.semi_major, k.semi_minor, k.major_axis_bearing_deg)],
                    &color(col.0, col.1, col.2, 255),
                    2.5,
                    &ring,
                ));
            }
        }
        let _ = writeln!(body, "<Folder><name>Landing points</name>");
        for r in rows.iter().filter(|r| r.descent == m) {
            if let Ok(s) = &r.result {
                body.push_str(&point_placemark(
                    &format!("#{}", r.sample),
                    &[format!("landing E {:.1} m, N {:.1} m ({:.1} m from launch)", s.landing_east, s.landing_north, s.landing_distance), format!("apogee {:.1} m", s.apogee)],
                    &pt,
                    s.landing_lat,
                    s.landing_lon,
                    0.0,
                    true,
                ));
            }
        }
        body.push_str("</Folder>\n</Folder>\n");
    }
    let d = ["Monte Carlo landing dispersion; failed samples are excluded. Ellipses are 1-sigma and 3-sigma of the landing-point covariance.".to_string()];
    doc.push("Monte Carlo: failed samples are excluded; ellipses are 1-sigma and 3-sigma of the landing-point covariance.".to_string());
    folder("Dispersion \u{2013} Monte Carlo", &d, &body)
}

// ---------------------------------------------------------------------------------------------
// Loaders: read the parts of the document back from the output files of earlier commands.
// Each returns Ok(None) when the file is absent and Err(reason) when it cannot be parsed.
// ---------------------------------------------------------------------------------------------

/// A CSV file with a header row; cells are looked up by column name.
struct Csv {
    cols: HashMap<String, usize>,
    rows: Vec<Vec<String>>,
}

impl Csv {
    fn read(path: &Path) -> Result<Option<Csv>, String> {
        if !path.exists() {
            return Ok(None);
        }
        let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
        let mut lines = text.lines().filter(|l| !l.trim().is_empty());
        let head = lines.next().ok_or_else(|| format!("{}: empty", path.display()))?;
        let cols = head.split(',').enumerate().map(|(i, c)| (c.trim().to_string(), i)).collect();
        let rows = lines.map(|l| l.split(',').map(|c| c.trim().to_string()).collect()).collect();
        Ok(Some(Csv { cols, rows }))
    }
    fn text<'a>(&self, row: &'a [String], col: &str) -> Option<&'a str> {
        row.get(*self.cols.get(col)?).map(|s| s.as_str())
    }
    fn num(&self, row: &[String], col: &str) -> Option<f64> {
        self.text(row, col)?.parse().ok()
    }
    fn req(&self, row: &[String], col: &str) -> Result<f64, String> {
        self.num(row, col).ok_or_else(|| format!("column {col} missing or not a number"))
    }
}

fn parse_descent(s: &str) -> Option<Descent> {
    match s {
        "ballistic" => Some(Descent::Ballistic),
        "parachute" => Some(Descent::Parachute),
        _ => None,
    }
}

fn read_json(path: &Path) -> Result<serde_json::Value, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))
}

/// `trajectory.csv` + `summary.json` in `dir`.
pub fn load_flight(dir: &Path) -> Result<Option<Flight>, String> {
    let (csv, json) = (dir.join("trajectory.csv"), dir.join("summary.json"));
    if !json.exists() {
        return Ok(None);
    }
    let Some(t) = Csv::read(&csv)? else { return Ok(None) };
    let summary = summary_from_json(&read_json(&json)?).map_err(|e| format!("{}: {e}", json.display()))?;
    let mut samples = Vec::with_capacity(t.rows.len());
    for r in &t.rows {
        let phase = match t.text(r, "phase") {
            Some("rail") => Phase::Rail,
            Some("free") => Phase::Free,
            Some("parachute") => Phase::Parachute,
            _ => return Err(format!("{}: bad phase", csv.display())),
        };
        let g = |c: &str| t.num(r, c).unwrap_or(0.0);
        let p = |c: &str| t.req(r, c).map_err(|e| format!("{}: {e}", csv.display()));
        samples.push(Sample {
            t: p("t")?, phase, east: g("east"), north: g("north"), up: g("up"), lat: p("lat")?, lon: p("lon")?, alt: p("alt")?, vel_e: g("vel_e"), vel_n: g("vel_n"), vel_u: g("vel_u"),
            airspeed: g("airspeed"), mach: g("mach"), alpha_deg: g("alpha_deg"), thrust: g("thrust"), mass: g("mass"), x_cg: g("x_cg"), x_cp: g("x_cp"), stability_cal: g("stability_cal"),
            dyn_pressure: g("dyn_pressure"), cn: g("cn"), ca: g("ca"), pitch_deg: g("pitch_deg"), heading_deg: g("heading_deg"), wx: g("wx"), wy: g("wy"), wz: g("wz"),
        });
    }
    if samples.is_empty() {
        return Err(format!("{}: no samples", csv.display()));
    }
    Ok(Some(Flight { samples, summary }))
}

/// A `Summary` from `summary.json`; null (NaN written by serde_json) and missing numbers become NaN.
fn summary_from_json(v: &serde_json::Value) -> Result<Summary, String> {
    let f = |k: &str| v.get(k).and_then(|x| x.as_f64()).unwrap_or(f64::NAN);
    let descent = v.get("descent").and_then(|d| d.as_str()).and_then(parse_descent).ok_or("bad descent")?;
    let s = Summary {
        descent, wind_speed: f("wind_speed"), wind_direction_deg: f("wind_direction_deg"), rail_exit_time: f("rail_exit_time"), rail_exit_speed: f("rail_exit_speed"), burnout_time: f("burnout_time"),
        max_speed: f("max_speed"), max_mach: f("max_mach"), max_dyn_pressure: f("max_dyn_pressure"), apogee_time: f("apogee_time"), apogee: f("apogee"), apogee_east: f("apogee_east"),
        apogee_north: f("apogee_north"), min_stability_cal: f("min_stability_cal"), stability_at_rail_exit: f("stability_at_rail_exit"), landing_time: f("landing_time"), landing_east: f("landing_east"),
        landing_north: f("landing_north"), landing_lat: f("landing_lat"), landing_lon: f("landing_lon"), landing_distance: f("landing_distance"), landing_speed: f("landing_speed"),
    };
    if !(s.landing_lat.is_finite() && s.landing_lon.is_finite()) {
        return Err("no landing position".into());
    }
    Ok(s)
}

/// A `Summary` from the landing columns shared by `dispersion.csv` and `dispersion_mc.csv`.
fn summary_from_row(t: &Csv, r: &[String]) -> Result<Summary, String> {
    let descent = t.text(r, "descent").and_then(parse_descent).ok_or("bad descent")?;
    let g = |c: &str| t.num(r, c).unwrap_or(0.0);
    Ok(Summary {
        descent, wind_speed: t.req(r, "wind_speed")?, wind_direction_deg: t.req(r, "wind_direction_deg")?, rail_exit_time: 0.0, rail_exit_speed: g("rail_exit_speed"), burnout_time: f64::NAN, max_speed: 0.0,
        max_mach: g("max_mach"), max_dyn_pressure: 0.0, apogee_time: g("apogee_time"), apogee: g("apogee"), apogee_east: 0.0, apogee_north: 0.0, min_stability_cal: g("min_stability_cal"),
        stability_at_rail_exit: 0.0, landing_time: g("landing_time"), landing_east: t.req(r, "landing_east")?, landing_north: t.req(r, "landing_north")?, landing_lat: t.req(r, "landing_lat")?,
        landing_lon: t.req(r, "landing_lon")?, landing_distance: g("landing_distance"), landing_speed: g("landing_speed"),
    })
}

/// `dispersion.csv` in `dir`.
pub fn load_wind_grid(dir: &Path) -> Result<Option<WindGrid>, String> {
    let path = dir.join("dispersion.csv");
    let Some(t) = Csv::read(&path)? else { return Ok(None) };
    let (mut cases, mut results) = (Vec::new(), Vec::new());
    for r in &t.rows {
        let s = summary_from_row(&t, r).map_err(|e| format!("{}: {e}", path.display()))?;
        cases.push(Case { descent: s.descent, wind_speed: s.wind_speed, wind_direction_deg: s.wind_direction_deg });
        results.push(s);
    }
    if cases.is_empty() {
        return Err(format!("{}: no rows", path.display()));
    }
    Ok(Some(WindGrid { cases, results }))
}

/// `dispersion_mc.csv` + `dispersion_summary.json` in `dir`.
pub fn load_monte_carlo(dir: &Path) -> Result<Option<MonteCarlo>, String> {
    let (csv, json) = (dir.join("dispersion_mc.csv"), dir.join("dispersion_summary.json"));
    if !json.exists() {
        return Ok(None);
    }
    let Some(t) = Csv::read(&csv)? else { return Ok(None) };
    let v = read_json(&json)?;
    let bad = |what: &str| format!("{}: {what}", json.display());
    let ell = |e: &serde_json::Value| Ellipse {
        semi_major: e["semi_major"].as_f64().unwrap_or(f64::NAN),
        semi_minor: e["semi_minor"].as_f64().unwrap_or(f64::NAN),
        major_axis_bearing_deg: e["major_axis_bearing_deg"].as_f64().unwrap_or(f64::NAN),
    };
    let mut stats = Vec::new();
    for d in v.get("descents").and_then(|d| d.as_array()).ok_or_else(|| bad("no descents"))? {
        let f = |k: &str| d.get(k).and_then(|x| x.as_f64()).unwrap_or(f64::NAN);
        let cov = |i: usize, j: usize| d["covariance"][i][j].as_f64().unwrap_or(f64::NAN);
        stats.push(DescentStats {
            descent: d.get("descent").and_then(|x| x.as_str()).and_then(parse_descent).ok_or_else(|| bad("bad descent"))?,
            failed: d.get("failed").and_then(|x| x.as_u64()).unwrap_or(0) as usize,
            landing: PointStats {
                n: d.get("n").and_then(|x| x.as_u64()).ok_or_else(|| bad("no n"))? as usize,
                mean_east: f("mean_east"),
                mean_north: f("mean_north"),
                covariance: [[cov(0, 0), cov(0, 1)], [cov(1, 0), cov(1, 1)]],
                ellipse_1sigma: ell(&d["ellipse_1sigma"]),
                ellipse_3sigma: ell(&d["ellipse_3sigma"]),
            },
            max_landing_distance: f("max_landing_distance"),
            max_landing_distance_sample: d.get("max_landing_distance_sample").and_then(|x| x.as_u64()).map(|x| x as usize),
            apogee_mean: f("apogee_mean"),
            apogee_std: f("apogee_std"),
        });
    }
    let mut rows = Vec::new();
    for r in &t.rows {
        let sample = t.num(r, "sample").ok_or_else(|| format!("{}: bad sample", csv.display()))? as usize;
        let descent = t.text(r, "descent").and_then(parse_descent).ok_or_else(|| format!("{}: bad descent", csv.display()))?;
        let g = |c: &str| t.num(r, c).unwrap_or(0.0);
        let perturbation = Perturbation {
            thrust_scale: g("thrust_scale"), burn_time_scale: g("burn_time_scale"), dry_mass: g("dry_mass"), cg: g("cg"), cn_scale: g("cn_scale"), ca_scale: g("ca_scale"), elevation_deg: g("elevation_deg"),
            azimuth_deg: g("azimuth_deg"), wind_speed: g("wind_speed_delta"), wind_direction_deg: g("wind_direction_deg_delta"), parachute_cd_s_scale: g("parachute_cd_s_scale"),
        };
        let result = if t.text(r, "status") == Some("ok") { Ok(summary_from_row(&t, r).map_err(|e| format!("{}: {e}", csv.display()))?) } else { Err("failed".to_string()) };
        rows.push(McRow { sample, descent, perturbation, result });
    }
    Ok(Some(MonteCarlo { rows, stats }))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Minimal well-formedness check: balanced tags, CDATA and comments skipped, attributes ignored.
    fn check_xml(s: &str) {
        let mut stack: Vec<String> = Vec::new();
        let mut rest = s;
        let mut roots = 0;
        while let Some(i) = rest.find('<') {
            assert!(!rest[..i].contains('>') || rest[..i].trim().is_empty() || !rest[..i].contains('<'), "stray text");
            rest = &rest[i..];
            if rest.starts_with("<![CDATA[") {
                rest = &rest[rest.find("]]>").expect("unterminated CDATA") + 3..];
                continue;
            }
            if rest.starts_with("<?") {
                rest = &rest[rest.find("?>").expect("unterminated PI") + 2..];
                continue;
            }
            let end = rest.find('>').expect("unterminated tag");
            let tag = &rest[1..end];
            if let Some(name) = tag.strip_prefix('/') {
                assert_eq!(stack.pop().as_deref(), Some(name), "mismatched close tag {name}");
            } else if !tag.ends_with('/') {
                if stack.is_empty() {
                    roots += 1;
                }
                stack.push(tag.split_whitespace().next().unwrap().to_string());
            }
            rest = &rest[end + 1..];
            // Text up to the next tag must not contain a raw '&' that is not an entity.
            let text = &rest[..rest.find('<').unwrap_or(rest.len())];
            for (k, _) in text.match_indices('&') {
                let ent = &text[k..];
                assert!(["&amp;", "&lt;", "&gt;", "&quot;", "&apos;"].iter().any(|e| ent.starts_with(e)), "bad entity in {text}");
            }
        }
        assert!(stack.is_empty() && roots == 1, "unbalanced document");
    }

    fn site() -> Launch {
        Launch { latitude: 34.7347, longitude: 139.4204, altitude: 20.0, rail_length: 5.0, elevation_deg: 85.0, azimuth_deg: 270.0 }
    }

    fn sample(t: f64, phase: Phase, thrust: f64, alt: f64) -> Sample {
        Sample {
            t, phase, east: 0.0, north: 0.0, up: alt - 20.0, lat: 34.7347 + t * 1e-4, lon: 139.4204 - t * 1e-4, alt, vel_e: 0.0, vel_n: 0.0, vel_u: 10.0, airspeed: 10.0,
            mach: 0.0, alpha_deg: 0.0, thrust, mass: 1.0, x_cg: 0.0, x_cp: 0.0, stability_cal: 1.0, dyn_pressure: 0.0, cn: 0.0, ca: 0.0, pitch_deg: 0.0, heading_deg: 0.0, wx: 0.0, wy: 0.0, wz: 0.0,
        }
    }

    fn summary() -> Summary {
        Summary {
            descent: Descent::Parachute, wind_speed: 4.0, wind_direction_deg: 0.0, rail_exit_time: 0.2, rail_exit_speed: 20.0, burnout_time: 1.0, max_speed: 1.0, max_mach: 1.0,
            max_dyn_pressure: 1.0, apogee_time: 3.0, apogee: 100.0, apogee_east: 0.0, apogee_north: 0.0, min_stability_cal: 1.0, stability_at_rail_exit: 1.5, landing_time: 6.0,
            landing_east: -10.0, landing_north: 5.0, landing_lat: 34.7348, landing_lon: 139.4203, landing_distance: 11.2, landing_speed: 5.0,
        }
    }

    fn flight() -> Vec<Sample> {
        vec![
            sample(0.0, Phase::Rail, 100.0, 20.0),
            sample(0.1, Phase::Rail, 100.0, 21.0),
            sample(0.2, Phase::Free, 100.0, 25.0),
            sample(1.0, Phase::Free, 0.0, 80.0),
            sample(3.0, Phase::Free, 0.0, 120.0),
            sample(4.0, Phase::Parachute, 0.0, 100.0),
            sample(6.0, Phase::Parachute, 0.0, 20.0),
        ]
    }

    #[test]
    fn escaping() {
        assert_eq!(esc("a<b>&\"c'"), "a&lt;b&gt;&amp;&quot;c&apos;");
        let doc = document("A & B <x>", &["1 < 2 & 3".into()], "");
        assert!(doc.contains("<name>A &amp; B &lt;x&gt;</name>") && doc.contains("1 &lt; 2 &amp; 3"));
        check_xml(&doc);
    }

    #[test]
    fn colors_are_aabbggrr() {
        assert_eq!(color(0x11, 0x22, 0x33, 0xff), "ff332211");
        let kml = combined(&site(), &Parts { flight: Some(Flight { samples: flight(), summary: summary() }), ..Default::default() });
        // Powered leg is red (r = 255): ff2828ff.
        assert!(kml.contains("<color>ff2828ff</color>"));
    }

    #[test]
    fn trajectory_structure() {
        let kml = combined(&site(), &Parts { flight: Some(Flight { samples: flight(), summary: summary() }), ..Default::default() });
        check_xml(&kml);
        for n in ["Launch site", "Rail exit", "Burnout", "Apogee", "Parachute deploy", "Landing", "Rail<", "Powered flight", "Coast", "Parachute descent"] {
            assert!(kml.contains(&format!("<name>{}", n.trim_end_matches('<'))), "{n}");
        }
        assert_eq!(kml.matches("<LineString>").count(), 4);
        assert!(kml.contains("<altitudeMode>absolute</altitudeMode>") && kml.contains("<altitudeMode>clampToGround</altitudeMode>"));
        // lon,lat,alt order of the apogee sample (t = 3, alt 120).
        assert!(kml.contains("139.42010000,34.73500000,120.00"));
        assert!(kml.contains("139.42040000,34.73470000,20.00"));
        // Ballistic flights have no parachute placemark.
        let mut s = flight();
        s.truncate(4);
        let ballistic = Summary { descent: Descent::Ballistic, ..summary() };
        assert!(!combined(&site(), &Parts { flight: Some(Flight { samples: s, summary: ballistic }), ..Default::default() }).contains("Parachute deploy"));
    }

    #[test]
    fn ellipse_closes_and_has_right_radii() {
        let origin = (34.7347, 139.4204, 20.0);
        let e = Ellipse { semi_major: 300.0, semi_minor: 100.0, major_axis_bearing_deg: 30.0 };
        let (me, mn) = (-654.8, -7708.5);
        let ring = ellipse_ring(origin, me, mn, &e, 72);
        assert_eq!(ring.len(), 73);
        assert_eq!(ring[0], ring[72]);
        let o = sim::geo::lla_to_ecef(origin.0, origin.1, origin.2);
        let [be, bn, _] = sim::geo::enu_basis(origin.0, origin.1);
        let (mut rmin, mut rmax) = (f64::MAX, 0.0f64);
        for &(lat, lon) in &ring[..72] {
            let p = sim::geo::lla_to_ecef(lat, lon, 20.0) - o;
            let (pe, pn) = (p.dot(be) - me, p.dot(bn) - mn);
            let r = pe.hypot(pn);
            rmin = rmin.min(r);
            rmax = rmax.max(r);
        }
        // Local ENU is not exactly the ECEF difference at 7.7 km (earth curvature): allow 0.5 m.
        assert!((rmax - 300.0).abs() < 0.5 && (rmin - 100.0).abs() < 0.5, "{rmin} {rmax}");
        // Vertex 0 lies on the major axis at bearing 30 deg.
        let p = sim::geo::lla_to_ecef(ring[0].0, ring[0].1, 20.0) - o;
        let (pe, pn) = (p.dot(be) - me, p.dot(bn) - mn);
        assert!((pe - 300.0 * 30f64.to_radians().sin()).abs() < 0.5 && (pn - 300.0 * 30f64.to_radians().cos()).abs() < 0.5);
    }

    fn wind_grid_part() -> WindGrid {
        let cases: Vec<Case> = [(1.0, 0.0), (1.0, 90.0), (1.0, 180.0), (2.0, 0.0), (2.0, 90.0)].iter().map(|&(s, d)| Case { descent: Descent::Ballistic, wind_speed: s, wind_direction_deg: d }).collect();
        let results: Vec<Summary> = cases.iter().map(|c| Summary { descent: c.descent, wind_speed: c.wind_speed, wind_direction_deg: c.wind_direction_deg, ..summary() }).collect();
        WindGrid { cases, results }
    }

    fn mc_part() -> MonteCarlo {
        let mut rows = Vec::new();
        for i in 0..6 {
            let result = if i == 4 { Err("never left the rail".to_string()) } else { Ok(Summary { landing_east: -50.0 + 20.0 * i as f64, landing_north: 30.0 - 7.0 * i as f64, ..summary() }) };
            rows.push(McRow { sample: i, descent: Descent::Parachute, perturbation: Perturbation::default(), result });
        }
        let pts: Vec<(f64, f64)> = rows.iter().filter_map(|r| r.result.as_ref().ok()).map(|s| (s.landing_east, s.landing_north)).collect();
        let landing = sim::dispersion::landing_statistics(&pts);
        let stats = vec![DescentStats { descent: Descent::Parachute, failed: 1, landing, max_landing_distance: 90.0, max_landing_distance_sample: Some(5), apogee_mean: 100.0, apogee_std: 0.0 }];
        MonteCarlo { rows, stats }
    }

    #[test]
    fn lines_only_no_fill_no_curtain() {
        let kml = combined(&site(), &Parts { flight: Some(Flight { samples: flight(), summary: summary() }), wind_grid: Some(wind_grid_part()), monte_carlo: Some(mc_part()) });
        assert!(!kml.contains("<extrude>1</extrude>") && !kml.contains("<extrude>"));
        assert!(!kml.contains("<PolyStyle>") && !kml.contains("<Polygon>") && !kml.contains("<fill>"));
        assert!(kml.contains("<LineStyle>") && kml.contains("1-sigma ellipse") && kml.contains("<name>Powered flight"));
    }

    fn folders(kml: &str) -> Vec<String> {
        ["Launch site", "Flight", "Dispersion \u{2013} wind grid", "Dispersion \u{2013} Monte Carlo"].iter().filter(|n| kml.contains(&format!("<Folder><name>{n}</name>"))).map(|n| n.to_string()).collect()
    }

    #[test]
    fn combined_document_folders() {
        let flight = || Some(Flight { samples: flight(), summary: summary() });
        let mk = |f: bool, w: bool, m: bool| {
            let kml = combined(&site(), &Parts { flight: if f { flight() } else { None }, wind_grid: w.then(wind_grid_part), monte_carlo: m.then(mc_part) });
            check_xml(&kml);
            assert_eq!(kml.matches("<Document>").count(), 1);
            folders(&kml).iter().map(|s| s.replace("Dispersion \u{2013} ", "")).collect::<Vec<_>>()
        };
        assert_eq!(mk(true, false, false), ["Launch site", "Flight"]);
        assert_eq!(mk(true, true, false), ["Launch site", "Flight", "wind grid"]);
        assert_eq!(mk(true, false, true), ["Launch site", "Flight", "Monte Carlo"]);
        assert_eq!(mk(false, true, true), ["Launch site", "wind grid", "Monte Carlo"]);
        assert_eq!(mk(false, true, false), ["Launch site", "wind grid"]);
        let kml = combined(&site(), &Parts { wind_grid: Some(wind_grid_part()), ..Default::default() });
        assert!(kml.contains("1 m/s from 90\u{b0}") && kml.matches("<Folder>").count() == 5, "{}", kml.matches("<Folder>").count());
        let kml = combined(&site(), &Parts { monte_carlo: Some(mc_part()), ..Default::default() });
        assert!(!kml.contains("<Polygon>"));
        assert!(kml.contains("1-sigma ellipse") && kml.contains("Mean landing") && kml.contains("<name>#5</name>") && !kml.contains("<name>#4</name>"));
    }

    fn temp_dir(name: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("ignisyeet_kml_{name}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn loaders_round_trip_written_files() {
        let dir = temp_dir("roundtrip");
        // Nothing written yet: every part is absent, not an error.
        assert!(load_flight(&dir).unwrap().is_none() && load_wind_grid(&dir).unwrap().is_none() && load_monte_carlo(&dir).unwrap().is_none());

        let (f, w, m) = (Flight { samples: flight(), summary: summary() }, wind_grid_part(), mc_part());
        crate::write_trajectory(&dir.join("trajectory.csv"), &f.samples).unwrap();
        std::fs::write(dir.join("summary.json"), serde_json::to_string_pretty(&f.summary).unwrap()).unwrap();
        crate::write_wind_grid_csv(&dir.join("dispersion.csv"), &w.results).unwrap();
        crate::write_monte_carlo_csv(&dir.join("dispersion_mc.csv"), &m.rows).unwrap();
        let mc_summary = sim::dispersion::summarize(&m.rows, m.rows.len(), 1);
        std::fs::write(dir.join("dispersion_summary.json"), serde_json::to_string_pretty(&mc_summary).unwrap()).unwrap();

        let lf = load_flight(&dir).unwrap().unwrap();
        assert_eq!(lf.samples.len(), f.samples.len());
        for (a, b) in lf.samples.iter().zip(&f.samples) {
            assert!(a.phase == b.phase && (a.t - b.t).abs() < 1e-3 && (a.alt - b.alt).abs() < 1e-2 && (a.lat - b.lat).abs() < 1e-7 && (a.thrust - b.thrust).abs() < 1e-2);
        }
        assert!((lf.summary.apogee - 100.0).abs() < 1e-9 && lf.summary.descent == Descent::Parachute && (lf.summary.landing_lat - 34.7348).abs() < 1e-9);

        let lw = load_wind_grid(&dir).unwrap().unwrap();
        assert_eq!(lw.cases.len(), 5);
        assert!(lw.cases[1].descent == Descent::Ballistic || lw.cases[1].descent == Descent::Parachute);
        assert!((lw.results[1].wind_direction_deg - 90.0).abs() < 1e-9 && (lw.results[1].landing_east + 10.0).abs() < 1e-2);

        let lm = load_monte_carlo(&dir).unwrap().unwrap();
        assert_eq!(lm.rows.len(), 6);
        assert!(lm.rows[4].result.is_err() && lm.rows[5].result.is_ok());
        assert_eq!(lm.stats.len(), 1);
        let (a, b) = (&lm.stats[0], &mc_summary.descents[0]);
        assert!(a.landing.n == 5 && a.failed == 1 && (a.landing.mean_east - b.landing.mean_east).abs() < 1e-9 && (a.landing.ellipse_3sigma.semi_major - b.landing.ellipse_3sigma.semi_major).abs() < 1e-9);

        // The loaded parts give the same document as the in-memory ones (up to the CSV precision).
        let live = combined(&site(), &Parts { flight: Some(f), wind_grid: Some(w), monte_carlo: Some(m) });
        let back = combined(&site(), &Parts { flight: Some(lf), wind_grid: Some(lw), monte_carlo: Some(lm) });
        check_xml(&back);
        assert_eq!(folders(&live), folders(&back));
        assert_eq!(live.matches("<Placemark>").count(), back.matches("<Placemark>").count());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn loaders_tolerate_bad_files() {
        let dir = temp_dir("bad");
        std::fs::write(dir.join("trajectory.csv"), "t,phase\n0,sideways\n").unwrap();
        std::fs::write(dir.join("summary.json"), "{ not json").unwrap();
        std::fs::write(dir.join("dispersion.csv"), "descent,wind_speed\nballistic,1\n").unwrap();
        assert!(load_flight(&dir).is_err() && load_wind_grid(&dir).is_err());
        // A summary without its CSV counts as absent.
        std::fs::write(dir.join("dispersion_summary.json"), "{}").unwrap();
        assert!(load_monte_carlo(&dir).unwrap().is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
