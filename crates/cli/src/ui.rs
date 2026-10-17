// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2024-2026 西濱大将 (NISHIHAMA Daisuke)
//! Terminal presentation: output mode detection, banner, boxed panels, spinners and progress bars.
//!
//! Three modes: `Rich` (stdout is a terminal: banner, panels, colours, progress on stderr),
//! `Plain` (stdout is piped or redirected: the classic line output, no colours, no bars) and
//! `Quiet` (`--quiet`: only errors and the paths of the written files).

use console::{measure_text_width, style, truncate_str, Term};
use indicatif::{ProgressBar, ProgressDrawTarget, ProgressState, ProgressStyle};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant, SystemTime};

/// Splits a leading `[IY-Xnnn] ` error code off a message: (code without brackets, rest).
pub fn split_code(s: &str) -> Option<(&str, &str)> {
    let body = s.strip_prefix("[IY-")?;
    let (code, rest) = body.split_once("] ")?;
    (!code.is_empty() && code.chars().all(|c| c.is_ascii_alphanumeric())).then_some((&s[1..4 + code.len()], rest))
}

/// The plain/quiet line of a warning: `warning[IY-Xnnn]: text` for a coded message, `warning: text` otherwise.
pub fn warning_line(s: &str) -> String {
    match split_code(s) {
        Some((code, rest)) => format!("warning[{code}]: {rest}"),
        None => format!("warning: {s}"),
    }
}

/// The report of a fatal error: `error[IY-Xnnn]: headline` (the first coded message of the chain gives the code,
/// shown once) followed by the causes; `Error: ...` in anyhow's layout when no message of the chain is coded.
pub fn error_report(e: &anyhow::Error) -> String {
    let chain: Vec<String> = e.chain().map(|c| c.to_string()).collect();
    let Some(code) = chain.iter().find_map(|m| split_code(m).map(|(c, _)| c.to_string())) else {
        return format!("Error: {e:?}");
    };
    let strip = |m: &str| match split_code(m) {
        Some((c, rest)) if c == code => rest.to_string(),
        _ => m.to_string(),
    };
    let mut out = format!("error[{code}]: {}", strip(&chain[0]));
    if chain.len() > 1 {
        out.push_str("\n\nCaused by:");
        for (i, c) in chain[1..].iter().enumerate() {
            if chain.len() > 2 {
                out.push_str(&format!("\n    {i}: {}", strip(c)));
            } else {
                out.push_str(&format!("\n    {}", strip(c)));
            }
        }
    }
    out
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mode {
    Quiet,
    Plain,
    Rich,
}

/// Glyph set (Unicode or ASCII fallback).
pub struct Glyphs {
    pub tl: &'static str,
    pub tr: &'static str,
    pub bl: &'static str,
    pub br: &'static str,
    pub h: &'static str,
    pub v: &'static str,
    pub ok: &'static str,
    pub bad: &'static str,
    pub warn: &'static str,
    pub dot: &'static str,
    pub bullet: &'static str,
    pub bar_chars: &'static str,
    pub spinner: &'static [&'static str],
}

const UNICODE: Glyphs = Glyphs {
    tl: "╭",
    tr: "╮",
    bl: "╰",
    br: "╯",
    h: "─",
    v: "│",
    ok: "✔",
    bad: "✖",
    warn: "⚠",
    dot: "·",
    bullet: "▸",
    bar_chars: "━╸━",
    spinner: &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏", "✔"],
};

const ASCII: Glyphs = Glyphs {
    tl: "+",
    tr: "+",
    bl: "+",
    br: "+",
    h: "-",
    v: "|",
    ok: "ok",
    bad: "x",
    warn: "!",
    dot: "-",
    bullet: ">",
    bar_chars: "#>-",
    spinner: &["|", "/", "-", "\\", "+"],
};

/// Remaining time shown in the progress bars: the mean time per finished step times the steps left,
/// or `--` before the first step has finished and when the figure is not meaningful. (indicatif's own
/// estimate explodes to u64::MAX seconds when the first step takes a long time, as a CFD case does.)
pub(crate) fn eta_text(elapsed: Duration, pos: u64, len: Option<u64>) -> String {
    let Some(len) = len else { return "--".into() };
    if pos == 0 || len == 0 {
        return "--".into();
    }
    let left = len.saturating_sub(pos);
    let secs = elapsed.as_secs_f64() / pos as f64 * left as f64;
    if !secs.is_finite() || secs > 100.0 * 86400.0 {
        return "--".into();
    }
    let t = secs.round() as u64;
    let (d, h, m, s) = (t / 86400, t / 3600 % 24, t / 60 % 60, t % 60);
    if d > 0 {
        format!("{d}d {h:02}:{m:02}:{s:02}")
    } else {
        format!("{h:02}:{m:02}:{s:02}")
    }
}

pub struct Ui {
    pub mode: Mode,
    progress: bool,
    unicode: bool,
    width: usize,
    start: Instant,
    start_wall: SystemTime,
    files: Mutex<Vec<PathBuf>>,
}

static UI: OnceLock<Ui> = OnceLock::new();

fn locale_is_utf8() -> bool {
    for k in ["LC_ALL", "LC_CTYPE", "LANG"] {
        if let Ok(v) = std::env::var(k) {
            if !v.is_empty() {
                let v = v.to_ascii_lowercase();
                return v.contains("utf-8") || v.contains("utf8");
            }
        }
    }
    false
}

pub fn init(quiet: bool, no_progress: bool) {
    let no_color = std::env::var_os("NO_COLOR").is_some_and(|v| !v.is_empty());
    let dumb = std::env::var("TERM").is_ok_and(|t| t == "dumb");
    if no_color || dumb {
        console::set_colors_enabled(false);
        console::set_colors_enabled_stderr(false);
    }
    let tty = Term::stdout().is_term();
    let mode = if quiet {
        Mode::Quiet
    } else if tty {
        Mode::Rich
    } else {
        Mode::Plain
    };
    let progress = mode == Mode::Rich && !no_progress && Term::stderr().is_term();
    let linux_console = std::env::var("TERM").is_ok_and(|t| t == "linux");
    let unicode = !dumb && !linux_console && locale_is_utf8() && Term::stdout().features().is_attended();
    let width = Term::stdout().size_checked().map(|(_, w)| w as usize).or_else(|| std::env::var("COLUMNS").ok().and_then(|c| c.parse().ok())).unwrap_or(80);
    let ui = Ui { mode, progress, unicode, width, start: Instant::now(), start_wall: SystemTime::now(), files: Mutex::new(Vec::new()) };
    let _ = UI.set(ui);
}

/// The global UI; plain output when `init` was not called (unit tests).
pub fn ui() -> &'static Ui {
    UI.get_or_init(|| Ui {
        mode: Mode::Plain,
        progress: false,
        unicode: false,
        width: 80,
        start: Instant::now(),
        start_wall: SystemTime::now(),
        files: Mutex::new(Vec::new()),
    })
}

pub fn fmt_dur(d: Duration) -> String {
    let s = d.as_secs_f64();
    if s < 1.0 {
        format!("{:.0} ms", s * 1000.0)
    } else if s < 60.0 {
        format!("{s:.2} s")
    } else {
        format!("{}m {:02.0}s", (s / 60.0) as u64, s % 60.0)
    }
}

pub fn fmt_size(bytes: u64) -> String {
    let b = bytes as f64;
    if b < 1024.0 {
        format!("{bytes} B")
    } else if b < 1024.0 * 1024.0 {
        format!("{:.1} KB", b / 1024.0)
    } else {
        format!("{:.1} MB", b / 1024.0 / 1024.0)
    }
}

/// Path relative to the current directory when that is shorter.
pub fn short_path(p: &Path) -> String {
    if let Ok(cwd) = std::env::current_dir() {
        if let Ok(r) = p.strip_prefix(&cwd) {
            return r.display().to_string();
        }
    }
    p.display().to_string()
}

/// Pads a (possibly styled) string with spaces to `w` visible columns.
pub fn pad(s: &str, w: usize) -> String {
    let n = measure_text_width(s);
    if n >= w {
        s.to_string()
    } else {
        format!("{s}{}", " ".repeat(w - n))
    }
}

pub fn pad_left(s: &str, w: usize) -> String {
    let n = measure_text_width(s);
    if n >= w {
        s.to_string()
    } else {
        format!("{}{s}", " ".repeat(w - n))
    }
}

pub enum Line {
    Blank,
    Section(String),
    Row(String, String),
    Text(String),
}

pub struct Panel {
    pub title: String,
    pub lines: Vec<Line>,
    /// Blank line before every section but the first.
    pub spaced: bool,
}

impl Panel {
    pub fn new(title: impl Into<String>) -> Self {
        Self { title: title.into(), lines: Vec::new(), spaced: false }
    }
    pub fn section(&mut self, name: &str) -> &mut Self {
        if self.spaced && !self.lines.is_empty() {
            self.lines.push(Line::Blank);
        }
        self.lines.push(Line::Section(name.to_string()));
        self
    }
    pub fn row(&mut self, k: &str, v: impl Into<String>) -> &mut Self {
        self.lines.push(Line::Row(k.to_string(), v.into()));
        self
    }
    pub fn text(&mut self, t: impl Into<String>) -> &mut Self {
        self.lines.push(Line::Text(t.into()));
        self
    }
}

const KEY_W: usize = 15;

impl Ui {
    pub fn g(&self) -> &'static Glyphs {
        if self.unicode {
            &UNICODE
        } else {
            &ASCII
        }
    }
    pub fn rich(&self) -> bool {
        self.mode == Mode::Rich
    }
    pub fn plain(&self) -> bool {
        self.mode == Mode::Plain
    }
    pub fn panel_width(&self) -> usize {
        self.width.clamp(60, 92)
    }

    /// Records a path that the command may have written; listed by [`Ui::finish`].
    pub fn record(&self, p: &Path) {
        self.files.lock().unwrap().push(p.to_path_buf());
    }

    pub fn banner(&self) {
        if !self.rich() {
            return;
        }
        let g = self.g();
        let rocket: [String; 6] = [
            format!("     {}", style("/\\").white().bold()),
            format!("    {}", style("/  \\").white().bold()),
            format!("   {}{}{}", style("|").white().bold(), style(" ** ").red().bold(), style("|").white().bold()),
            format!(
                "  {}{}{}{}{}",
                style("/").white().bold(),
                style("|").white().bold(),
                style("    ").white(),
                style("|").white().bold(),
                style("\\").white().bold()
            ),
            format!(
                " {}{}{}{}{}",
                style("/_").white().bold(),
                style("|").white().bold(),
                style("_/\\_").white().bold(),
                style("|").white().bold(),
                style("_\\").white().bold()
            ),
            format!("    {}{}{}", style("(").yellow().bold(), style("vv").red().bold(), style(")").yellow().bold()),
        ];
        let text = [
            String::new(),
            format!("{}  {}", style("IgnisYeet").bold().red(), style(format!("v{}", env!("CARGO_PKG_VERSION"))).dim()),
            style("rocket flight simulator").bold().to_string(),
            style(format!("STL aero {d} 6-DoF flight {d} landing dispersion", d = g.dot)).dim().to_string(),
            String::new(),
            String::new(),
        ];
        println!();
        for (r, t) in rocket.iter().zip(text.iter()) {
            println!("{}   {}", pad(r, 12), t);
        }
        let (c, who) = if self.unicode { ("©", "西濱大将 (NISHIHAMA Daisuke)") } else { ("(C)", "NISHIHAMA Daisuke") };
        println!("{}", style(format!("{c} 2024-2026 {who} {d} GPL-3.0-or-later {d} NO WARRANTY", d = g.dot)).dim());
        println!();
    }

    pub fn print_panel(&self, p: &Panel) {
        if !self.rich() {
            return;
        }
        for l in self.render_panel(p) {
            println!("{l}");
        }
    }

    pub fn render_panel(&self, p: &Panel) -> Vec<String> {
        let g = self.g();
        let w = self.panel_width();
        let inner = w - 4;
        let mut out = Vec::new();
        let title = format!(" {} ", style(&p.title).bold());
        let fill = w.saturating_sub(measure_text_width(&title) + 3);
        out.push(format!("{}{}{}{}{}", style(g.tl).dim(), style(g.h).dim(), title, style(g.h.repeat(fill)).dim(), style(g.tr).dim()));
        let boxed = |s: String| {
            let s = if measure_text_width(&s) > inner { truncate_str(&s, inner, "…").to_string() } else { s };
            format!("{} {} {}", style(g.v).dim(), pad(&s, inner), style(g.v).dim())
        };
        for l in &p.lines {
            out.push(match l {
                Line::Blank => boxed(String::new()),
                Line::Section(s) => boxed(format!("{} {}", style(g.bullet).cyan(), style(s).cyan().bold())),
                Line::Row(k, v) => boxed(format!("  {}{}", pad(&style(k).dim().to_string(), KEY_W), v)),
                Line::Text(t) => boxed(format!("  {t}")),
            });
        }
        out.push(format!("{}{}{}", style(g.bl).dim(), style(g.h.repeat(w - 2)).dim(), style(g.br).dim()));
        out
    }

    /// Progress bar over `len` steps, drawn on stderr only in rich mode with a terminal.
    pub fn bar(&self, len: u64, prefix: &str, template: &str) -> ProgressBar {
        self.bar_with(len, prefix, template, |s| s)
    }

    /// Whether progress bars are drawn at all.
    pub fn progress_on(&self) -> bool {
        self.progress
    }

    /// Like [`Ui::bar`], letting the caller customise the style (e.g. add template keys).
    pub fn bar_with(&self, len: u64, prefix: &str, template: &str, tweak: impl FnOnce(ProgressStyle) -> ProgressStyle) -> ProgressBar {
        if !self.progress {
            return ProgressBar::hidden();
        }
        let pb = ProgressBar::with_draw_target(Some(len), ProgressDrawTarget::stderr_with_hz(15));
        let g = self.g();
        let bar_w = if self.width >= 110 {
            32
        } else if self.width >= 90 {
            24
        } else {
            16
        };
        let t = template.replace("{bar}", &format!("{{bar:{bar_w}.cyan/blue.dim}}"));
        let style = ProgressStyle::with_template(&t).unwrap().progress_chars(g.bar_chars).tick_strings(g.spinner).with_key(
            "eta_safe",
            |st: &ProgressState, w: &mut dyn std::fmt::Write| {
                let _ = w.write_str(&eta_text(st.elapsed(), st.pos(), st.len()));
            },
        );
        pb.set_style(tweak(style));
        pb.set_prefix(prefix.to_string());
        pb.enable_steady_tick(Duration::from_millis(90));
        pb
    }

    pub fn spinner(&self, msg: &str) -> ProgressBar {
        if !self.progress {
            return ProgressBar::hidden();
        }
        let pb = ProgressBar::with_draw_target(None, ProgressDrawTarget::stderr_with_hz(15));
        let g = self.g();
        pb.set_style(ProgressStyle::with_template("  {spinner:.cyan} {msg} {elapsed:.dim}").unwrap().tick_strings(g.spinner));
        pb.set_message(msg.to_string());
        pb.enable_steady_tick(Duration::from_millis(90));
        pb
    }

    /// Starts a step with a spinner; call [`Step::done`] to replace it with a check mark line.
    pub fn step(&self, label: &str) -> Step {
        Step { pb: self.spinner(label), label: label.to_string(), start: Instant::now() }
    }

    /// "✔ label  detail  time" line of a finished stage (rich mode only).
    pub fn done_line(&self, label: &str, detail: &str, elapsed: Duration) {
        if self.rich() {
            println!("  {} {}  {}  {}", style(self.g().ok).green().bold(), pad(label, 22), detail, style(fmt_dur(elapsed)).dim());
        }
    }

    pub fn warn(&self, s: impl AsRef<str>) {
        if self.mode != Mode::Quiet {
            let s = s.as_ref();
            if self.rich() {
                println!("  {} {}", style(self.g().warn).yellow().bold(), style(s).yellow());
            } else {
                println!("{}", warning_line(s));
            }
        }
    }

    /// Final lines: written files (✔ list in rich mode, bare paths in quiet mode) and total time.
    pub fn finish(&self) {
        let floor = self.start_wall.checked_sub(Duration::from_secs(1)).unwrap_or(SystemTime::UNIX_EPOCH);
        let mut seen: Vec<PathBuf> = Vec::new();
        let mut items: Vec<(PathBuf, u64)> = Vec::new();
        for p in self.files.lock().unwrap().iter() {
            if seen.contains(p) {
                continue;
            }
            seen.push(p.clone());
            if let Ok(m) = std::fs::metadata(p) {
                if m.is_file() && m.modified().is_ok_and(|t| t >= floor) {
                    items.push((p.clone(), m.len()));
                }
            }
        }
        match self.mode {
            Mode::Quiet => {
                for (p, _) in &items {
                    println!("{}", p.display());
                }
            }
            Mode::Plain => {}
            Mode::Rich => {
                let g = self.g();
                if !items.is_empty() {
                    println!();
                    println!("  {}", style("Output files").bold());
                    let w = self.panel_width();
                    let names: Vec<String> = items.iter().map(|(p, _)| short_path(p)).collect();
                    let nw = names.iter().map(|n| measure_text_width(n)).max().unwrap_or(0).min(w.saturating_sub(18));
                    for (n, (_, sz)) in names.iter().zip(&items) {
                        let n = if measure_text_width(n) > nw { truncate_str(n, nw, "…").to_string() } else { n.clone() };
                        println!("  {} {} {}", style(g.ok).green().bold(), pad(&n, nw), style(pad_left(&fmt_size(*sz), 9)).dim());
                    }
                }
                println!();
                println!("  {} {} {}", style(g.ok).green().bold(), style("Done in").bold(), style(fmt_dur(self.start.elapsed())).bold().green());
                println!();
            }
        }
    }
}

pub struct Step {
    pb: ProgressBar,
    label: String,
    start: Instant,
}

impl Step {
    pub fn done(self, detail: &str) {
        self.pb.finish_and_clear();
        ui().done_line(&self.label, detail, self.start.elapsed());
    }
}

impl Drop for Step {
    fn drop(&mut self) {
        self.pb.finish_and_clear();
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn eta_is_a_mean_rate_estimate_and_never_absurd() {
        let s = |n| Duration::from_secs(n);
        assert_eq!(eta_text(s(100), 0, Some(16)), "--", "no step finished yet");
        assert_eq!(eta_text(s(600), 1, Some(16)), "02:30:00", "15 steps of 600 s");
        assert_eq!(eta_text(s(10), 16, Some(16)), "00:00:00");
        assert_eq!(eta_text(s(10), 3, None), "--", "unknown length");
        assert_eq!(eta_text(s(86400 * 3), 1, Some(3)), "6d 00:00:00");
        assert_eq!(eta_text(s(86400 * 60), 1, Some(10)), "--", "more than 100 days is not an estimate");
        assert_eq!(eta_text(Duration::MAX, 1, Some(2)), "--");
    }

    use super::*;

    #[test]
    fn durations_and_sizes() {
        assert_eq!(fmt_dur(Duration::from_millis(250)), "250 ms");
        assert_eq!(fmt_dur(Duration::from_secs_f64(4.256)), "4.26 s");
        assert_eq!(fmt_dur(Duration::from_secs(125)), "2m 05s");
        assert_eq!(fmt_size(2048), "2.0 KB");
        assert_eq!(pad("ab", 4), "ab  ");
    }

    #[test]
    fn coded_messages_render_with_the_code_once() {
        assert_eq!(split_code("[IY-O542] x y"), Some(("IY-O542", "x y")));
        assert_eq!(split_code("plain"), None);
        assert_eq!(warning_line("[IY-O500] aero.mach_step is coarse"), "warning[IY-O500]: aero.mach_step is coarse");
        assert_eq!(warning_line("plain"), "warning: plain");
        let inner = anyhow::anyhow!("[IY-S002] bad number");
        let e = inner.context("cannot read motor file m.eng");
        assert_eq!(error_report(&e), "error[IY-S002]: cannot read motor file m.eng\n\nCaused by:\n    bad number");
        let e = anyhow::anyhow!("[IY-O001] --descent parachute needs recovery.enabled = true");
        assert_eq!(error_report(&e), "error[IY-O001]: --descent parachute needs recovery.enabled = true");
        let e = anyhow::anyhow!("[IY-G003] unrecognised").context("[IY-O013] cannot read").context("outer");
        assert_eq!(error_report(&e), "error[IY-O013]: outer\n\nCaused by:\n    0: cannot read\n    1: [IY-G003] unrecognised");
        assert!(error_report(&anyhow::anyhow!("boom")).starts_with("Error: boom"));
    }
}
