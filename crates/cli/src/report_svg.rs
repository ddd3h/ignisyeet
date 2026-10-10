// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025-2026 西濱大将 (NISHIHAMA Daisuke)
//! Small inline-SVG charts for the HTML report. Colours come from CSS classes of the page (`.s0`..`.s3`,
//! `.ax`, `.grid`, `.tx`), so the same SVG follows the light and dark colour schemes and the print style.

use std::fmt::Write;

/// Escapes text for HTML and SVG (element content and attribute values).
pub fn esc(s: &str) -> String {
    let mut o = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => o.push_str("&amp;"),
            '<' => o.push_str("&lt;"),
            '>' => o.push_str("&gt;"),
            '"' => o.push_str("&quot;"),
            '\'' => o.push_str("&#39;"),
            c => o.push(c),
        }
    }
    o
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Line,
    Dots,
    /// Closed polygon (a ring of landing points of one wind speed).
    Ring,
}

pub struct Series {
    pub name: String,
    pub pts: Vec<(f64, f64)>,
    /// Colour class 0..=3.
    pub color: usize,
    pub kind: Kind,
    /// 0..1: stroke or fill opacity.
    pub opacity: f64,
}

impl Series {
    pub fn line(name: &str, color: usize, pts: Vec<(f64, f64)>) -> Self {
        Self { name: name.into(), pts, color, kind: Kind::Line, opacity: 1.0 }
    }
    pub fn dots(name: &str, color: usize, pts: Vec<(f64, f64)>, opacity: f64) -> Self {
        Self { name: name.into(), pts, color, kind: Kind::Dots, opacity }
    }
    pub fn ring(name: &str, color: usize, pts: Vec<(f64, f64)>, opacity: f64) -> Self {
        Self { name: name.into(), pts, color, kind: Kind::Ring, opacity }
    }
}

/// A labelled mark at a data position (vertical rule for lines charts, marker for equal-aspect charts).
pub struct Mark {
    pub x: f64,
    pub y: Option<f64>,
    pub label: String,
}

pub struct Chart {
    pub title: String,
    pub xlabel: String,
    pub ylabel: String,
    pub series: Vec<Series>,
    pub marks: Vec<Mark>,
    /// Equal scale on both axes (maps, scatter plots).
    pub equal: bool,
    /// Force the y range to start at 0 (when all data are positive).
    pub y_zero: bool,
}

impl Chart {
    pub fn new(title: &str, xlabel: &str, ylabel: &str) -> Self {
        Self { title: title.into(), xlabel: xlabel.into(), ylabel: ylabel.into(), series: Vec::new(), marks: Vec::new(), equal: false, y_zero: false }
    }
}

/// "Nice" tick positions covering `[lo, hi]` with about `n` intervals.
pub fn ticks(lo: f64, hi: f64, n: usize) -> Vec<f64> {
    if !(lo.is_finite() && hi.is_finite()) || hi <= lo {
        return vec![lo];
    }
    let raw = (hi - lo) / n.max(1) as f64;
    let mag = 10f64.powf(raw.log10().floor());
    let r = raw / mag;
    let step = mag
        * if r <= 1.0 {
            1.0
        } else if r <= 2.0 {
            2.0
        } else if r <= 5.0 {
            5.0
        } else {
            10.0
        };
    let mut t = (lo / step).ceil() * step;
    let mut v = Vec::new();
    while t <= hi + step * 1e-9 {
        v.push(if t.abs() < step * 1e-9 { 0.0 } else { t });
        t += step;
    }
    v
}

/// Tick label with just the decimals the step needs.
pub fn tick_label(v: f64, step: f64) -> String {
    let dec = if step >= 1.0 { 0 } else { (-step.log10().floor()) as usize };
    let s = format!("{v:.dec$}");
    if s == "-0" {
        "0".into()
    } else {
        s
    }
}

fn step_of(t: &[f64]) -> f64 {
    if t.len() >= 2 {
        (t[1] - t[0]).abs()
    } else {
        1.0
    }
}

/// Reduces a polyline to about `max` points keeping the extremes of each bucket, so peaks survive.
pub fn decimate(pts: &[(f64, f64)], max: usize) -> Vec<(f64, f64)> {
    if pts.len() <= max || max < 4 {
        return pts.to_vec();
    }
    let buckets = max / 2;
    let per = pts.len() as f64 / buckets as f64;
    let mut out = Vec::with_capacity(max + 2);
    out.push(pts[0]);
    for b in 0..buckets {
        let a = (b as f64 * per) as usize;
        let z = (((b + 1) as f64 * per) as usize).min(pts.len());
        if a >= z {
            continue;
        }
        let s = &pts[a..z];
        let (mut imin, mut imax) = (0, 0);
        for (i, p) in s.iter().enumerate() {
            if p.1 < s[imin].1 {
                imin = i;
            }
            if p.1 > s[imax].1 {
                imax = i;
            }
        }
        let (i, j) = if imin < imax { (imin, imax) } else { (imax, imin) };
        out.push(s[i]);
        if j != i {
            out.push(s[j]);
        }
    }
    out.push(pts[pts.len() - 1]);
    out
}

const W: f64 = 480.0;
const H: f64 = 320.0;
const ML: f64 = 56.0;
const MR: f64 = 16.0;
const MT: f64 = 14.0;
const MB: f64 = 46.0;

fn num(v: f64) -> String {
    let s = format!("{v:.2}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s.is_empty() || s == "-0" {
        "0".into()
    } else {
        s.into()
    }
}

/// Renders the chart; None when no series holds a finite point.
pub fn render(c: &Chart) -> Option<String> {
    let finite = |p: &&(f64, f64)| p.0.is_finite() && p.1.is_finite();
    let all: Vec<&(f64, f64)> = c.series.iter().flat_map(|s| s.pts.iter()).filter(finite).collect();
    if all.is_empty() {
        return None;
    }
    let (mut x0, mut x1) = (f64::INFINITY, f64::NEG_INFINITY);
    let (mut y0, mut y1) = (f64::INFINITY, f64::NEG_INFINITY);
    for p in &all {
        x0 = x0.min(p.0);
        x1 = x1.max(p.0);
        y0 = y0.min(p.1);
        y1 = y1.max(p.1);
    }
    for m in &c.marks {
        if c.equal {
            x0 = x0.min(m.x);
            x1 = x1.max(m.x);
            if let Some(y) = m.y {
                y0 = y0.min(y);
                y1 = y1.max(y);
            }
        }
    }
    if c.y_zero && y0 > 0.0 {
        y0 = 0.0;
    }
    let pad = |lo: f64, hi: f64, f: f64| {
        let d = if hi > lo { (hi - lo) * f } else { lo.abs().max(1.0) * 0.5 };
        (lo - d, hi + d)
    };
    let (pw, ph) = (W - ML - MR, H - MT - MB);
    if c.equal {
        let span = (x1 - x0).max(y1 - y0).max(1e-9) * 1.12;
        let (cx, cy) = ((x0 + x1) / 2.0, (y0 + y1) / 2.0);
        let (sx, sy) = if pw >= ph { (span * pw / ph, span) } else { (span, span * ph / pw) };
        (x0, x1, y0, y1) = (cx - sx / 2.0, cx + sx / 2.0, cy - sy / 2.0, cy + sy / 2.0);
    } else {
        (x0, x1) = if x1 > x0 { (x0, x1) } else { pad(x0, x1, 0.0) };
        let (a, b) = pad(y0, y1, 0.06);
        y1 = b;
        if !(c.y_zero && y0 == 0.0) {
            y0 = a;
        }
    }
    let tx = ticks(x0, x1, 5);
    let ty = ticks(y0, y1, 5);
    let (sx, sy) = (step_of(&tx), step_of(&ty));
    let px = |x: f64| ML + (x - x0) / (x1 - x0) * pw;
    let py = |y: f64| MT + ph - (y - y0) / (y1 - y0) * ph;
    let mut o = String::new();
    let _ = write!(o, "<svg class=\"chart\" viewBox=\"0 0 {W} {H}\" role=\"img\" aria-label=\"{}\"><title>{}</title>", esc(&c.title), esc(&c.title));
    for &t in &ty {
        let y = py(t);
        let _ = write!(o, "<line class=\"grid\" x1=\"{ML}\" x2=\"{}\" y1=\"{y:.1}\" y2=\"{y:.1}\"/>", W - MR);
        let _ = write!(o, "<text class=\"tx\" x=\"{}\" y=\"{:.1}\" text-anchor=\"end\">{}</text>", ML - 6.0, y + 4.0, tick_label(t, sy));
    }
    for &t in &tx {
        let x = px(t);
        let _ = write!(o, "<line class=\"grid\" x1=\"{x:.1}\" x2=\"{x:.1}\" y1=\"{MT}\" y2=\"{}\"/>", H - MB);
        let _ = write!(o, "<text class=\"tx\" x=\"{x:.1}\" y=\"{}\" text-anchor=\"middle\">{}</text>", H - MB + 16.0, tick_label(t, sx));
    }
    let _ = write!(o, "<rect class=\"ax\" x=\"{ML}\" y=\"{MT}\" width=\"{pw}\" height=\"{ph}\" fill=\"none\"/>");
    let _ = write!(o, "<text class=\"tx\" x=\"{}\" y=\"{}\" text-anchor=\"middle\">{}</text>", ML + pw / 2.0, H - 8.0, esc(&c.xlabel));
    let _ = write!(o, "<text class=\"tx\" transform=\"translate(14 {}) rotate(-90)\" text-anchor=\"middle\">{}</text>", MT + ph / 2.0, esc(&c.ylabel));
    let _ = write!(
        o,
        "<clipPath id=\"cp{}\"><rect x=\"{ML}\" y=\"{MT}\" width=\"{pw}\" height=\"{ph}\"/></clipPath><g clip-path=\"url(#cp{})\">",
        id_of(&c.title),
        id_of(&c.title)
    );
    for (k, m) in c.marks.iter().enumerate() {
        if c.equal {
            continue;
        }
        let x = px(m.x);
        if (ML..=W - MR).contains(&x) {
            let _ = write!(o, "<line class=\"mark\" x1=\"{x:.1}\" x2=\"{x:.1}\" y1=\"{MT}\" y2=\"{}\"/>", H - MB);
            let _ = write!(
                o,
                "<text class=\"tx sm\" x=\"{:.1}\" y=\"{}\" text-anchor=\"{}\">{}</text>",
                x + if x > W * 0.7 { -4.0 } else { 4.0 },
                MT + 11.0 + 11.0 * k as f64,
                if x > W * 0.7 { "end" } else { "start" },
                esc(&m.label)
            );
        }
    }
    for s in &c.series {
        let pts: Vec<(f64, f64)> = s.pts.iter().copied().filter(|p| p.0.is_finite() && p.1.is_finite()).collect();
        if pts.is_empty() {
            continue;
        }
        match s.kind {
            Kind::Line | Kind::Ring => {
                let mut d = String::new();
                for (i, p) in pts.iter().enumerate() {
                    let _ = write!(d, "{}{:.1} {:.1}", if i == 0 { "M" } else { "L" }, px(p.0), py(p.1));
                }
                if s.kind == Kind::Ring {
                    d.push('Z');
                }
                let _ = write!(o, "<path class=\"s{} line\" d=\"{d}\" fill=\"none\" stroke-opacity=\"{}\"/>", s.color, num(s.opacity));
            }
            Kind::Dots => {
                let _ = write!(o, "<g class=\"f{}\" fill-opacity=\"{}\">", s.color, num(s.opacity));
                for p in &pts {
                    let _ = write!(o, "<circle cx=\"{:.1}\" cy=\"{:.1}\" r=\"2.2\"/>", px(p.0), py(p.1));
                }
                o.push_str("</g>");
            }
        }
    }
    if c.equal {
        for m in &c.marks {
            if let Some(y) = m.y {
                let (x, y) = (px(m.x), py(y));
                let _ = write!(o, "<path class=\"mk\" d=\"M{:.1} {:.1}l5 5m-10 0l5 -5m0 -5l0 10m-5 -5l10 0\" fill=\"none\"/>", x, y);
                let _ = write!(o, "<text class=\"tx sm\" x=\"{:.1}\" y=\"{:.1}\">{}</text>", x + 8.0, y - 7.0, esc(&m.label));
            }
        }
    }
    o.push_str("</g></svg>");
    Some(o)
}

fn id_of(title: &str) -> String {
    title.bytes().fold(5381u32, |h, b| h.wrapping_mul(33) ^ u32::from(b)).to_string()
}

/// Legend paragraph (HTML) for the series of a chart.
pub fn legend(c: &Chart) -> String {
    let mut o = String::from("<p class=\"legend\">");
    for s in c.series.iter().filter(|s| !s.name.is_empty()) {
        let _ = write!(o, "<span><i class=\"sw b{}\"></i>{}</span>", s.color, esc(&s.name));
    }
    o.push_str("</p>");
    o
}

/// A figure: chart, legend and caption. Empty when there is nothing to draw.
pub fn figure(c: &Chart, caption: &str) -> String {
    match render(c) {
        Some(svg) => format!("<figure>{svg}{}<figcaption>{}</figcaption></figure>", legend(c), esc(caption)),
        None => String::new(),
    }
}

/// Ellipse outline: centre, semi axes, bearing of the major axis (clockwise from north).
pub fn ellipse(cx: f64, cy: f64, semi_major: f64, semi_minor: f64, bearing_deg: f64) -> Vec<(f64, f64)> {
    let th = bearing_deg.to_radians();
    let (ux, uy) = (th.sin(), th.cos());
    let (vx, vy) = (th.cos(), -th.sin());
    (0..=72)
        .map(|i| {
            let t = i as f64 / 72.0 * std::f64::consts::TAU;
            let (a, b) = (semi_major * t.cos(), semi_minor * t.sin());
            (cx + a * ux + b * vx, cy + a * uy + b * vy)
        })
        .collect()
}

/// Rocket silhouette: body radius and fin outer radius (metres) along the axis, with markers for the
/// centre of gravity and the centre of pressure.
pub fn rocket_profile(profile: &[(f64, f64, f64)], cg: Option<f64>, cp: Option<f64>) -> Option<String> {
    let valid: Vec<&(f64, f64, f64)> = profile.iter().filter(|p| p.0.is_finite() && p.1.is_finite() && p.2.is_finite()).collect();
    if valid.len() < 2 {
        return None;
    }
    let len = valid.iter().map(|p| p.0).fold(0.0, f64::max);
    let rmax = valid.iter().map(|p| p.1 + p.2).fold(0.0, f64::max).max(1e-6);
    if len <= 0.0 {
        return None;
    }
    let (w, ml) = (860.0, 20.0);
    let scale = (w - 2.0 * ml) / len;
    let half = (rmax * scale).max(20.0);
    let h = 2.0 * half + 56.0;
    let cy = half + 14.0;
    let outline = |f: &dyn Fn(&(f64, f64, f64)) -> f64| -> String {
        let mut d = String::new();
        for (i, p) in valid.iter().enumerate() {
            let _ = write!(d, "{}{:.1} {:.1}", if i == 0 { "M" } else { "L" }, ml + p.0 * scale, cy - f(p) * scale);
        }
        d
    };
    let mut o = String::new();
    let _ = write!(o, "<svg class=\"chart wide\" viewBox=\"0 0 {w} {h:.0}\" role=\"img\" aria-label=\"Rocket profile\"><title>Rocket profile</title>");
    let _ = write!(o, "<line class=\"grid\" x1=\"{ml}\" x2=\"{}\" y1=\"{cy:.1}\" y2=\"{cy:.1}\" stroke-dasharray=\"4 3\"/>", w - ml);
    for sign in [1.0, -1.0] {
        let mut d = String::new();
        for (i, p) in valid.iter().enumerate() {
            let _ = write!(d, "{}{:.1} {:.1}", if i == 0 { "M" } else { "L" }, ml + p.0 * scale, cy - sign * (p.1 + p.2) * scale);
        }
        let _ = write!(o, "<path class=\"fin\" d=\"{d}\" fill=\"none\"/>");
        let _ = write!(o, "<path class=\"s0 line\" d=\"{}\" fill=\"none\"/>", outline(&|p| sign * p.1));
    }
    let tick = |o: &mut String, x: f64, label: &str, cls: &str, row: f64| {
        let px = ml + x * scale;
        let _ = write!(o, "<line class=\"mark\" x1=\"{px:.1}\" x2=\"{px:.1}\" y1=\"{:.1}\" y2=\"{:.1}\"/>", cy - half - 6.0, cy + half + 6.0);
        let _ = write!(o, "<text class=\"tx sm {cls}\" x=\"{px:.1}\" y=\"{:.1}\" text-anchor=\"middle\">{}</text>", cy + half + 18.0 + row, esc(label));
    };
    if let Some(x) = cg {
        tick(&mut o, x, &format!("CG {x:.3} m"), "", 0.0);
    }
    if let Some(x) = cp {
        tick(&mut o, x, &format!("CP {x:.3} m"), "", if cg.is_some_and(|g| (g - x).abs() * scale < 90.0) { 14.0 } else { 0.0 });
    }
    let _ = write!(o, "<text class=\"tx sm\" x=\"{ml}\" y=\"{:.1}\">nose tip</text>", cy - half - 8.0);
    let _ = write!(o, "<text class=\"tx sm\" x=\"{}\" y=\"{:.1}\" text-anchor=\"end\">{len:.3} m</text>", w - ml, cy - half - 8.0);
    o.push_str("</svg>");
    Some(o)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escape_covers_html_and_attribute_characters() {
        assert_eq!(esc(r#"<a href="x">&'"#), "&lt;a href=&quot;x&quot;&gt;&amp;&#39;");
    }

    #[test]
    fn ticks_are_round_and_inside() {
        let t = ticks(0.0, 4440.0, 6);
        assert_eq!(t, vec![0.0, 1000.0, 2000.0, 3000.0, 4000.0]);
        assert_eq!(tick_label(0.5, 0.5), "0.5");
        assert_eq!(tick_label(-0.0, 1.0), "0");
    }

    #[test]
    fn decimation_keeps_the_peak() {
        let pts: Vec<(f64, f64)> = (0..10_000).map(|i| (i as f64, if i == 4321 { 99.0 } else { 1.0 })).collect();
        let d = decimate(&pts, 400);
        assert!(d.len() <= 410);
        assert!(d.iter().any(|p| p.1 == 99.0));
    }

    #[test]
    fn chart_has_title_series_and_no_nan() {
        let mut c = Chart::new("Height <b>", "t [s]", "h [m]");
        c.series.push(Series::line("h", 0, vec![(0.0, 0.0), (1.0, 5.0), (2.0, f64::NAN), (3.0, 4.0)]));
        let svg = render(&c).unwrap();
        assert!(svg.contains("Height &lt;b&gt;") && !svg.contains("NaN") && svg.contains("class=\"s0 line\""));
        assert!(render(&Chart::new("empty", "", "")).is_none());
    }

    #[test]
    fn ellipse_axes_follow_the_bearing() {
        // Major axis towards the east (bearing 90 deg).
        let e = ellipse(0.0, 0.0, 10.0, 2.0, 90.0);
        let (x, y) = e[0];
        assert!((x - 10.0).abs() < 1e-9 && y.abs() < 1e-9);
    }
}
