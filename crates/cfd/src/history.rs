// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025-2026 西濱大将 (NISHIHAMA Daisuke)
//! Parser of SU2's `history.csv`.
//!
//! SU2 v8 writes quoted, space-padded headers such as
//! `"Time_Iter","Outer_Iter","Inner_Iter",   "rms[Rho]",   "rms[RhoU]", ... "CL", "CD", "CMy", "CFx", "CFz"`.
//! Older versions use `"Iteration"` / `"Res_Flow[0]"`. Missing columns are tolerated.

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default)]
pub struct History {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<f64>>,
}

fn clean(s: &str) -> String {
    s.trim().trim_matches('"').trim().to_string()
}

impl History {
    pub fn parse(text: &str) -> Result<History> {
        let mut lines = text.lines().filter(|l| !l.trim().is_empty());
        let Some(head) = lines.next() else { bail!("empty history file") };
        let columns: Vec<String> = head.split(',').map(clean).collect();
        if columns.len() < 2 {
            bail!("history header has no columns");
        }
        // A last line without its newline is still being written (or was cut off by a stopped solver): skipped,
        // since a cut number can still give the right column count.
        let complete = text.ends_with('\n');
        let lines: Vec<&str> = lines.collect();
        let n_lines = lines.len();
        let mut rows = Vec::new();
        for (k, l) in lines.into_iter().enumerate() {
            if k + 1 == n_lines && !complete {
                continue;
            }
            let v: Vec<f64> = l.split(',').map(|c| c.trim().trim_matches('"').parse::<f64>().unwrap_or(f64::NAN)).collect();
            // A row cut off by a running solver is skipped.
            if v.len() == columns.len() {
                rows.push(v);
            }
        }
        Ok(History { columns, rows })
    }

    pub fn read(path: &std::path::Path) -> Result<History> {
        Self::parse(&std::fs::read_to_string(path).map_err(|e| anyhow::anyhow!("cannot read {}: {e}", path.display()))?)
    }

    /// Column index by name (case-insensitive, first match among `names`).
    pub fn col(&self, names: &[&str]) -> Option<usize> {
        names.iter().find_map(|n| self.columns.iter().position(|c| c.eq_ignore_ascii_case(n)))
    }
}

/// Averaged coefficients over the last rows, with convergence information.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HistorySummary {
    /// Number of iterations performed (last inner iteration + 1).
    pub iterations: usize,
    pub cl: Option<f64>,
    pub cd: Option<f64>,
    pub cfx: Option<f64>,
    pub cfy: Option<f64>,
    pub cfz: Option<f64>,
    pub cmx: Option<f64>,
    pub cmy: Option<f64>,
    pub cmz: Option<f64>,
    /// log10 of the RMS density residual in the first / last row.
    pub residual_first: Option<f64>,
    pub residual_last: Option<f64>,
}

impl HistorySummary {
    /// Orders of magnitude the density residual dropped.
    pub fn residual_drop(&self) -> Option<f64> {
        Some(self.residual_first? - self.residual_last?)
    }

    /// Final residual at or below `minval` (log10).
    pub fn converged(&self, minval: f64) -> bool {
        self.residual_last.is_some_and(|r| r <= minval)
    }
}

/// Column names of the log10 RMS density residual.
const RHO: [&str; 3] = ["rms[Rho]", "Res_Flow[0]", "rms[P]"];
/// Column names of the log10 RMS residual of the SA working variable (SU2 8.3: `rms[nu]`).
const NU: [&str; 3] = ["rms[nu]", "rms[Nu_Tilde]", "rms[nu_tilde]"];

/// RANS coefficient-stability tolerances (see [`History::rans_coefficients_stable`]).
pub const RANS_FORCE_DRIFT: f64 = 0.005;
/// Largest drift of the centre of pressure, in body lengths.
pub const RANS_XCP_DRIFT: f64 = 0.002;
/// RANS divergence ([`History::rans_divergence`]): rows of the median ...
pub const DIVERGENCE_ROWS: usize = 5;
/// ... rise of the density residual above its minimum (orders) ...
pub const DIVERGENCE_RHO_RISE: f64 = 2.0;
/// ... rise of the SA residual above its minimum (orders) ...
pub const DIVERGENCE_NU_RISE: f64 = 3.0;
/// ... and largest plausible force coefficient magnitude.
pub const ABSURD_FORCE: f64 = 100.0;
/// Limit cycle: largest inter-quartile range of the forces, as a fraction of the force scale.
pub const LIMIT_CYCLE_SPREAD: f64 = 0.01;

/// Least-squares slope of `v` per row (0 for fewer than two values).
fn slope(v: &[f64]) -> f64 {
    let n = v.len() as f64;
    if v.len() < 2 {
        return 0.0;
    }
    let xm = (n - 1.0) / 2.0;
    let ym = v.iter().sum::<f64>() / n;
    let (sxy, sxx) = v.iter().enumerate().fold((0.0, 0.0), |(a, b), (i, y)| (a + (i as f64 - xm) * (y - ym), b + (i as f64 - xm).powi(2)));
    sxy / sxx
}

/// (mean, drift between the two half-window means, inter-quartile range) of `v`; infinite spread for non-finite values.
fn window_stats(v: &[f64]) -> (f64, f64, f64) {
    if v.is_empty() || v.iter().any(|x| !x.is_finite()) {
        return (f64::NAN, f64::INFINITY, f64::INFINITY);
    }
    let mean = |s: &[f64]| s.iter().sum::<f64>() / s.len() as f64;
    let (a, b) = v.split_at(v.len() / 2);
    let mut s = v.to_vec();
    s.sort_by(|x, y| x.partial_cmp(y).unwrap());
    let iqr = s[3 * s.len() / 4] - s[s.len() / 4];
    (mean(v), (mean(b) - mean(a)).abs(), iqr)
}

impl History {
    /// Largest finite value of the first existing column of `names`.
    fn peak(&self, names: &[&str]) -> Option<f64> {
        let c = self.col(names)?;
        let p = self.rows.iter().map(|r| r[c]).filter(|x| x.is_finite()).fold(f64::NEG_INFINITY, f64::max);
        p.is_finite().then_some(p)
    }

    fn last(&self, names: &[&str]) -> Option<f64> {
        let c = self.col(names)?;
        self.rows.last().map(|r| r[c]).filter(|x| x.is_finite())
    }

    /// Largest log10 RMS density residual of the run.
    pub fn residual_peak(&self) -> Option<f64> {
        self.peak(&RHO)
    }

    /// Largest log10 RMS residual of the SA variable (`None` without a turbulence model).
    pub fn nu_peak(&self) -> Option<f64> {
        self.peak(&NU)
    }

    /// log10 RMS residual of the SA variable in the last row.
    pub fn last_nu(&self) -> Option<f64> {
        self.last(&NU)
    }

    /// Values of column `names` over the last `window` rows (`None` when the column is missing).
    fn tail(&self, names: &[&str], window: usize) -> Option<Vec<f64>> {
        let c = self.col(names)?;
        Some(self.rows[self.rows.len().saturating_sub(window)..].iter().map(|r| r[c]).collect())
    }

    /// RANS: the coefficients are flat over the last `window` rows. The two forces (`CFx`, `CFz`; with sideslip
    /// also `CFy`) drift by at most [`RANS_FORCE_DRIFT`] of the force scale (the larger mean) between the half
    /// windows, with an inter-quartile range of at most the same; the centre of pressure `-CMy / CFz` (with
    /// sideslip also `CMz / CFy`) drifts by at most [`RANS_XCP_DRIFT`] body lengths (`length_ratio` = body
    /// length / reference length). Near zero normal force, where the centre of pressure is undefined, the
    /// moment may drift by the force tolerance times the body length. Roll cases add the Euler `CMx` rule.
    pub fn rans_coefficients_stable(&self, window: usize, kind: StabilityKind, length_ratio: f64) -> bool {
        self.rans_window_check(window, kind, length_ratio, false)
    }

    /// RANS limit cycle: over the last `window` rows the forces oscillate with an inter-quartile range of at most
    /// [`LIMIT_CYCLE_SPREAD`] of the force scale (centre of pressure: twice [`RANS_XCP_DRIFT`]) without a drift requirement.
    pub fn limit_cycle(&self, window: usize, kind: StabilityKind, length_ratio: f64) -> bool {
        self.rans_window_check(window, kind, length_ratio, true)
    }

    fn rans_window_check(&self, window: usize, kind: StabilityKind, length_ratio: f64, cycle: bool) -> bool {
        if self.rows.len() < window.max(4) {
            return false;
        }
        let side = kind == StabilityKind::Sideslip;
        let fx = self.tail(&["CFx", "CD"], window).map(|v| window_stats(&v));
        let fz = self.tail(&["CFz", "CL"], window).map(|v| window_stats(&v));
        let fy = if side { self.tail(&["CFy"], window).map(|v| window_stats(&v)) } else { None };
        if fx.is_none() && fz.is_none() {
            return false;
        }
        let scale = [fx, fz, fy].iter().flatten().map(|s| s.0.abs()).fold(0.0, f64::max);
        let tol = if cycle { 0.5 * LIMIT_CYCLE_SPREAD } else { RANS_FORCE_DRIFT } * scale + 1e-5;
        // stable: drift and spread <= tol; limit cycle: spread <= 2 tol, any drift
        let spread_factor = if cycle { 2.0 } else { 1.0 };
        let ok = |s: Option<(f64, f64, f64)>, tol: f64| s.is_none_or(|(_, drift, spread)| (cycle || drift <= tol) && spread <= spread_factor * tol);
        let mut stable = ok(fx, tol) && ok(fz, tol) && ok(fy, tol);
        // centre of pressure of a force / moment pair, in reference lengths
        let xcp_tol = RANS_XCP_DRIFT * length_ratio;
        let mut moment = |force: &[&str], mom: &[&str], sign: f64| {
            let (Some(f), Some(m)) = (self.tail(force, window), self.tail(mom, window)) else { return };
            let fs = window_stats(&f);
            if fs.0.abs() > 20.0 * tol {
                let x: Vec<f64> = f.iter().zip(&m).map(|(f, m)| sign * m / f).collect();
                stable = stable && ok(Some(window_stats(&x)), xcp_tol);
            } else {
                stable = stable && ok(Some(window_stats(&m)), tol * length_ratio);
            }
        };
        moment(&["CFz", "CL"], &["CMy"], -1.0);
        if side {
            moment(&["CFy"], &["CMz"], 1.0);
        }
        if kind == StabilityKind::Roll {
            let mx = self.tail(&["CMx"], window).map(|v| window_stats(&v));
            let t = 0.03 * mx.map_or(0.0, |s| s.0.abs()) + 2e-5;
            stable = stable && ok(mx, t);
        }
        stable
    }

    /// RANS: the coefficients have settled: the change still to come, extrapolated from their trend over the last
    /// `window` rows, is within the tolerances of [`History::rans_coefficients_stable`]. A coefficient that converges
    /// with the residuals approaches its limit like `exp(-i / tau)`, so its remaining change is `|slope| tau`;
    /// `tau = 1 / (r ln 10)` with `r` the slower of the density and SA residual decay rates over the window
    /// (orders per iteration), at most `budget` iterations (a stalled residual leaves the trend over the whole budget).
    ///
    /// The flatness check alone stopped a flat-plate case whose drag still crept by 1.9 % (TMR 273 x 193 grid,
    /// M 0.2): over a window of 200 iterations the drift of a slowly converging coefficient stays below 0.5 %.
    pub fn rans_trend_settled(&self, window: usize, kind: StabilityKind, length_ratio: f64, budget: usize) -> bool {
        if self.rows.len() < window.max(4) {
            return false;
        }
        let rate = [&RHO[..], &NU[..]].iter().filter_map(|c| self.tail(c, window)).map(|v| -slope(&v)).fold(f64::INFINITY, f64::min);
        let tau = if rate.is_finite() && rate > 0.0 { (1.0 / (rate * std::f64::consts::LN_10)).min(budget as f64) } else { budget as f64 };
        let side = kind == StabilityKind::Sideslip;
        let fx = self.tail(&["CFx", "CD"], window);
        let fz = self.tail(&["CFz", "CL"], window);
        let fy = if side { self.tail(&["CFy"], window) } else { None };
        if fx.is_none() && fz.is_none() {
            return false;
        }
        let mean = |v: &[f64]| v.iter().sum::<f64>() / v.len() as f64;
        let scale = [&fx, &fz, &fy].iter().filter_map(|v| v.as_ref()).map(|v| mean(v).abs()).fold(0.0, f64::max);
        let tol = RANS_FORCE_DRIFT * scale + 1e-5;
        let change = |v: &[f64]| (slope(v) * tau).abs();
        let mut ok = [&fx, &fz, &fy].iter().filter_map(|v| v.as_ref()).all(|v| change(v) <= tol);
        let mut moment = |force: &[&str], mom: &[&str], sign: f64| {
            let (Some(f), Some(m)) = (self.tail(force, window), self.tail(mom, window)) else { return };
            if mean(&f).abs() > 20.0 * tol {
                let x: Vec<f64> = f.iter().zip(&m).map(|(f, m)| sign * m / f).collect();
                ok = ok && change(&x) <= RANS_XCP_DRIFT * length_ratio;
            } else {
                ok = ok && change(&m) <= tol * length_ratio;
            }
        };
        moment(&["CFz", "CL"], &["CMy"], -1.0);
        if side {
            moment(&["CFy"], &["CMz"], 1.0);
        }
        ok
    }

    /// RANS divergence (the reason), judged on the median of the last [`DIVERGENCE_ROWS`] rows: the density residual
    /// [`DIVERGENCE_RHO_RISE`] or the SA residual [`DIVERGENCE_NU_RISE`] orders above its minimum since row 10, a
    /// non-finite residual or coefficient, or a force coefficient above [`ABSURD_FORCE`] in magnitude.
    pub fn rans_divergence(&self) -> Option<String> {
        let n = self.rows.len();
        if n < DIVERGENCE_ROWS + 10 {
            return None;
        }
        let median = |c: usize| {
            let mut v: Vec<f64> = self.rows[n - DIVERGENCE_ROWS..].iter().map(|r| r[c]).collect();
            if v.iter().any(|x| !x.is_finite()) {
                return f64::NAN;
            }
            v.sort_by(|a, b| a.partial_cmp(b).unwrap());
            v[v.len() / 2]
        };
        for (names, what, rise) in [(&RHO[..], "density", DIVERGENCE_RHO_RISE), (&NU[..], "SA", DIVERGENCE_NU_RISE)] {
            let Some(c) = self.col(names) else { continue };
            let now = median(c);
            if !now.is_finite() {
                return Some(format!("the {what} residual is not finite"));
            }
            let min = self.rows[10..].iter().map(|r| r[c]).filter(|x| x.is_finite()).fold(f64::INFINITY, f64::min);
            if now - min >= rise {
                return Some(format!("the {what} residual rose {:.1} orders above its minimum ({min:.2} -> {now:.2})", now - min));
            }
        }
        for name in ["CFx", "CFy", "CFz", "CMx", "CMy", "CMz"] {
            let Some(c) = self.col(&[name]) else { continue };
            let now = median(c);
            if !now.is_finite() {
                return Some(format!("{name} is not finite"));
            }
            if name.starts_with("CF") && now.abs() > ABSURD_FORCE {
                return Some(format!("{name} = {now:.3e}"));
            }
        }
        None
    }

    /// log10 RMS density residual of the last row.
    pub fn last_residual(&self) -> Option<f64> {
        self.last(&RHO)
    }

    /// Orders of magnitude between the largest density residual of the run and the last one.
    pub fn residual_drop_from_peak(&self) -> Option<f64> {
        Some(self.residual_peak()? - self.last_residual()?).filter(|d| d.is_finite())
    }

    /// Latest inner iteration number (row count when there is no iteration column).
    pub fn last_iteration(&self) -> usize {
        match (self.col(&["Inner_Iter", "Iteration", "Iter"]), self.rows.last()) {
            (Some(c), Some(r)) if r[c].is_finite() => r[c] as usize,
            _ => self.rows.len().saturating_sub(1),
        }
    }

    /// The force coefficients (`CFx`, `CFz`, `CMy`, or `CD`, `CL`) are flat over the last `window`
    /// rows: the means of the two half windows differ by less than 2 % of the force scale (+ 5e-4)
    /// and the inter-quartile range stays below 4 times that. Moment tolerance: 25 % of the force scale.
    ///
    /// `kind` adds the coefficients the case is run for: `CFy` and `CMz` for sideslip (same tolerances as
    /// the normal force and the pitching moment), `CMx` for roll-rate cases (3 % of its own mean + 2e-5,
    /// since the rolling moment is much smaller than the force scale).
    pub fn coefficients_stable(&self, window: usize, kind: StabilityKind) -> bool {
        let n = self.rows.len();
        if n < window.max(4) {
            return false;
        }
        // (mean, drift between the two half-window means, inter-quartile range) of a column.
        let stats = |names: &[&str]| -> Option<(f64, f64, f64)> {
            let c = self.col(names)?;
            let v: Vec<f64> = self.rows[n - window..].iter().map(|r| r[c]).collect();
            if v.iter().any(|x| !x.is_finite()) {
                return Some((f64::NAN, f64::INFINITY, f64::INFINITY));
            }
            let mean = |s: &[f64]| s.iter().sum::<f64>() / s.len() as f64;
            let (a, b) = v.split_at(v.len() / 2);
            let mut s = v.clone();
            s.sort_by(|x, y| x.partial_cmp(y).unwrap());
            let iqr = s[3 * s.len() / 4] - s[s.len() / 4];
            Some((mean(&v), (mean(b) - mean(a)).abs(), iqr))
        };
        let (fx, fz, my) = (stats(&["CFx", "CD"]), stats(&["CFz", "CL"]), stats(&["CMy"]));
        if fx.is_none() && fz.is_none() && my.is_none() {
            return false;
        }
        // The two forces share the larger mean as scale, so a noisy near-zero normal force at
        // alpha = 0 is judged against the axial force; the nose-origin moment has the scale of
        // about 12 reference lengths times that. The inter-quartile range ignores isolated spikes.
        let scale = [fx, fz].iter().flatten().map(|s| s.0.abs()).fold(0.0, f64::max);
        let ok = |s: Option<(f64, f64, f64)>, tol: f64| s.is_none_or(|(_, drift, spread)| drift <= tol && spread <= 4.0 * tol);
        let mut stable = ok(fx, 0.02 * scale + 5e-4) && ok(fz, 0.02 * scale + 5e-4) && ok(my, 0.25 * scale + 5e-3);
        match kind {
            StabilityKind::Pitch => {}
            StabilityKind::Sideslip => {
                let (fy, mz) = (stats(&["CFy"]), stats(&["CMz"]));
                let scale = scale.max(fy.map_or(0.0, |s| s.0.abs()));
                stable = stable && ok(fy, 0.02 * scale + 5e-4) && ok(mz, 0.25 * scale + 5e-3);
            }
            StabilityKind::Roll => {
                let mx = stats(&["CMx"]);
                let tol = 0.03 * mx.map_or(0.0, |s| s.0.abs()) + 2e-5;
                stable = stable && ok(mx, tol);
            }
        }
        stable
    }
}

/// Which extra coefficients [`History::coefficients_stable`] checks besides the axial / normal force and pitching moment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StabilityKind {
    /// Angle of attack only (`CFx`, `CFz`, `CMy`).
    Pitch,
    /// Sideslip case: also `CFy` and `CMz`.
    Sideslip,
    /// Roll-rate case: also `CMx` (the roll-damping signal).
    Roll,
}

impl StabilityKind {
    pub fn of(spec: &crate::case::CaseSpec) -> Self {
        if spec.is_roll() {
            Self::Roll
        } else if spec.beta_deg != 0.0 {
            Self::Sideslip
        } else {
            Self::Pitch
        }
    }
}

/// Summary of `h`; coefficients are averaged over the last `window` rows (1 = last row only).
pub fn summarize(h: &History, window: usize) -> Option<HistorySummary> {
    let n = h.rows.len();
    if n == 0 {
        return None;
    }
    let w = window.clamp(1, n);
    let avg = |names: &[&str]| -> Option<f64> {
        let c = h.col(names)?;
        let v: Vec<f64> = h.rows[n - w..].iter().map(|r| r[c]).filter(|x| x.is_finite()).collect();
        (!v.is_empty()).then(|| v.iter().sum::<f64>() / v.len() as f64)
    };
    let iterations = match h.col(&["Inner_Iter", "Iteration", "Iter"]) {
        Some(c) if h.rows[n - 1][c].is_finite() => h.rows[n - 1][c] as usize + 1,
        _ => n,
    };
    let rc = h.col(&["rms[Rho]", "Res_Flow[0]", "rms[P]"]);
    let res = |i: usize| rc.map(|c| h.rows[i][c]).filter(|x| x.is_finite());
    Some(HistorySummary {
        iterations,
        cl: avg(&["CL"]),
        cd: avg(&["CD"]),
        cfx: avg(&["CFx"]),
        cfy: avg(&["CFy"]),
        cfz: avg(&["CFz"]),
        cmx: avg(&["CMx"]),
        cmy: avg(&["CMy"]),
        cmz: avg(&["CMz"]),
        residual_first: res(0),
        residual_last: res(n - 1),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const V8: &str = r#""Time_Iter","Outer_Iter","Inner_Iter",   "rms[Rho]",   "rms[RhoU]",   "rms[RhoV]",   "rms[RhoW]",   "rms[RhoE]",   "CL",   "CD",   "CSF",   "CMx",   "CMy",   "CMz",   "CFx",   "CFy",   "CFz",   "CEff",   "Linear_Solver_Iterations",   "CFL_Number",   "Time(min)"
0,0,0,  -2.5,  -2.4,  -2.4,  -2.4,  -2.0,  0.1,  0.30,  0.0,  0.0,  -0.50,  0.0,  0.30,  0.0,  0.10,  0.3,  5,  5.0,  0.01
0,0,1,  -3.5,  -3.4,  -3.4,  -3.4,  -3.0,  0.2,  0.28,  0.0,  0.0,  -0.60,  0.0,  0.28,  0.0,  0.20,  0.7,  5,  6.0,  0.02
0,0,2,  -6.5,  -6.4,  -6.4,  -6.4,  -6.0,  0.4,  0.26,  0.0,  0.0,  -0.70,  0.0,  0.26,  0.0,  0.40,  1.5,  5,  7.0,  0.03
"#;

    #[test]
    fn parses_v8_history() {
        let h = History::parse(V8).unwrap();
        assert_eq!(h.rows.len(), 3);
        assert_eq!(h.columns[3], "rms[Rho]");
        let s = summarize(&h, 1).unwrap();
        assert_eq!(s.iterations, 3);
        assert_eq!(s.cfz, Some(0.40));
        assert_eq!(s.cmy, Some(-0.70));
        assert_eq!(s.cfx, Some(0.26));
        assert!((s.residual_drop().unwrap() - 4.0).abs() < 1e-12);
        assert!(s.converged(-6.0) && !s.converged(-7.0));
        let a = summarize(&h, 2).unwrap();
        assert!((a.cfz.unwrap() - 0.30).abs() < 1e-12);
    }

    #[test]
    fn an_unterminated_last_row_is_skipped() {
        // Cut off in the middle of the last field: right column count, wrong value.
        let h = History::parse("\"Iteration\",\"rms[Rho]\",\"CFx\"\n0,-1.0,0.31\n1,-2.0,0.3").unwrap();
        assert_eq!(h.rows.len(), 1);
        assert_eq!(History::parse("\"Iteration\",\"rms[Rho]\",\"CFx\"\n0,-1.0,0.31\n1,-2.0,0.3\n").unwrap().rows.len(), 2);
    }

    #[test]
    fn tolerates_missing_columns_and_partial_rows() {
        let t = "\"Iteration\",\"Res_Flow[0]\",\"CD\"\n0,-1.0,0.5\n1,-2.0,0.4\n2,-3.0";
        let h = History::parse(t).unwrap();
        assert_eq!(h.rows.len(), 2);
        let s = summarize(&h, 1).unwrap();
        assert_eq!(s.iterations, 2);
        assert_eq!(s.cd, Some(0.4));
        assert!(s.cfz.is_none() && s.cmy.is_none());
        assert!(summarize(&History::parse("\"a\",\"b\"\n").unwrap(), 1).is_none());
        assert!(History::parse("").is_err());
    }

    /// Synthetic history of `n` rows: constant forces, `CMx` ramping from `mx0` to `mx1`, `CFy` from `fy0` to `fy1`.
    fn synthetic(n: usize, mx: (f64, f64), fy: (f64, f64)) -> History {
        let mut t = String::from("\"Iteration\",\"rms[Rho]\",\"CFx\",\"CFy\",\"CFz\",\"CMx\",\"CMy\",\"CMz\"\n");
        for i in 0..n {
            let f = i as f64 / (n - 1) as f64;
            t += &format!("{i},{},0.3,{},0.1,{},-0.5,{}\n", -1.0 - 0.1 * i as f64, fy.0 + (fy.1 - fy.0) * f, mx.0 + (mx.1 - mx.0) * f, 0.2);
        }
        History::parse(&t).unwrap()
    }

    #[test]
    fn roll_and_sideslip_cases_check_their_own_coefficients() {
        // CMx still drifting by 50 % over the window: the force-only check would accept it.
        let h = synthetic(60, (1e-3, 1.5e-3), (0.0, 0.0));
        assert!(h.coefficients_stable(40, StabilityKind::Pitch));
        assert!(h.coefficients_stable(40, StabilityKind::Sideslip));
        assert!(!h.coefficients_stable(40, StabilityKind::Roll));
        assert!(synthetic(60, (1e-3, 1e-3), (0.0, 0.0)).coefficients_stable(40, StabilityKind::Roll));
        // CFy drifting (sideslip only).
        let h = synthetic(60, (0.0, 0.0), (0.05, 0.15));
        assert!(h.coefficients_stable(40, StabilityKind::Pitch));
        assert!(!h.coefficients_stable(40, StabilityKind::Sideslip));
        let spec = |a: f64, b: f64, r: f64| crate::case::CaseSpec { mach: 1.0, alpha_deg: a, beta_deg: b, roll_hat: r };
        assert_eq!(StabilityKind::of(&spec(2.0, 0.0, 0.0)), StabilityKind::Pitch);
        assert_eq!(StabilityKind::of(&spec(2.0, 3.0, 0.0)), StabilityKind::Sideslip);
        assert_eq!(StabilityKind::of(&spec(0.0, 0.0, 0.01)), StabilityKind::Roll);
    }
    /// `n` rows: CFx 0.3, CFz `fz`, CMy = -fz * xcp with xcp ramping from `x0` to `x1` reference lengths, residual columns.
    fn rans_rows(n: usize, fz: f64, x: (f64, f64), fx_ramp: f64) -> History {
        let mut t = String::from("\"Inner_Iter\",\"rms[Rho]\",\"rms[nu]\",\"CFx\",\"CFz\",\"CMy\"\n");
        for i in 0..n {
            let f = i as f64 / (n - 1) as f64;
            let xcp = x.0 + (x.1 - x.0) * f;
            t += &format!("{i},{},{},{},{fz},{}\n", -2.0 - 0.1 * i as f64, -5.0 - 0.05 * i as f64, 0.3 * (1.0 + fx_ramp * f), -fz * xcp);
        }
        History::parse(&t).unwrap()
    }

    #[test]
    fn rans_tolerances_force_drift_and_centre_of_pressure() {
        let k = StabilityKind::Pitch;
        // L / d = 18: 0.002 L = 0.036 d. The window is the last half of the rows, so the half-window drift is
        // a quarter of the ramp.
        assert!(rans_rows(80, 0.2, (10.0, 10.0), 0.0).rans_coefficients_stable(40, k, 18.0));
        assert!(rans_rows(80, 0.2, (10.0, 10.1), 0.0).rans_coefficients_stable(40, k, 18.0)); // drift 0.025 d
        assert!(!rans_rows(80, 0.2, (10.0, 10.4), 0.0).rans_coefficients_stable(40, k, 18.0)); // drift 0.1 d
                                                                                               // Axial force: 0.5 % of the scale.
        assert!(rans_rows(80, 0.2, (10.0, 10.0), 0.01).rans_coefficients_stable(40, k, 18.0));
        assert!(!rans_rows(80, 0.2, (10.0, 10.0), 0.04).rans_coefficients_stable(40, k, 18.0));
        // The Euler rule accepts the 0.1 d centre-of-pressure drift and the 1 % force drift.
        assert!(rans_rows(80, 0.2, (10.0, 10.4), 0.04).coefficients_stable(40, k));
        // Zero normal force: the moment itself is checked (scale 0.3: tolerance 0.0015 x 18).
        let flat = rans_rows(80, 0.0, (10.0, 10.0), 0.0);
        assert!(flat.rans_coefficients_stable(40, k, 18.0) && flat.limit_cycle(40, k, 18.0));
        // Residual peaks and the SA column.
        assert_eq!((flat.residual_peak(), flat.nu_peak()), (Some(-2.0), Some(-5.0)));
        assert!((flat.last_nu().unwrap() + 5.0 + 0.05 * 79.0).abs() < 1e-9);
        assert_eq!(History::parse(V8).unwrap().nu_peak(), None);
        // Too few rows.
        assert!(!rans_rows(10, 0.2, (10.0, 10.0), 0.0).rans_coefficients_stable(40, k, 18.0));
    }

    #[test]
    fn rans_divergence_detection() {
        let mk = |rows: &[(f64, f64, f64)]| {
            let mut t = String::from("\"Inner_Iter\",\"rms[Rho]\",\"rms[nu]\",\"CFx\",\"CFz\",\"CMy\"\n");
            for (i, (r, nu, fz)) in rows.iter().enumerate() {
                t += &format!("{i},{r},{nu},0.39,{fz},0.07\n");
            }
            History::parse(&t).unwrap()
        };
        // Stalled limiter chatter (residual swinging by about one order): not a divergence.
        let stall: Vec<_> = (0..400).map(|i| (if i % 50 < 5 { -5.1 } else { -4.0 }, -7.8, -0.005)).collect();
        assert_eq!(mk(&stall).rans_divergence(), None);
        // The M 0.8 sample case after the limiter freeze: the SA residual jumps from -7.7 to -2.2.
        let mut blow = stall.clone();
        blow.extend((0..20).map(|i| (-4.0 + 0.05 * i as f64, -7.7 + 0.3 * i as f64, 0.1 * i as f64)));
        let why = mk(&blow).rans_divergence().unwrap();
        assert!(why.contains("SA residual rose"), "{why}");
        // Density residual 2 orders above its minimum.
        let mut up = stall.clone();
        up.extend((0..10).map(|_| (-3.0, -7.8, -0.005)));
        assert!(mk(&up).rans_divergence().unwrap().contains("density residual rose 2.1"));
        // One spike is not enough (median of the last rows).
        let mut spike = stall.clone();
        spike.extend([(-1.0, -1.0, 0.0), (-4.0, -7.8, -0.005), (-4.0, -7.8, -0.005)]);
        assert_eq!(mk(&spike).rans_divergence(), None);
        // Absurd and non-finite coefficients.
        let mut big = stall.clone();
        big.extend((0..5).map(|_| (-4.0, -7.8, -394.0)));
        assert!(mk(&big).rans_divergence().unwrap().contains("CFz = -3.940e2"));
        let mut nan = stall.clone();
        nan.extend((0..5).map(|_| (f64::NAN, -7.8, 0.0)));
        assert!(mk(&nan).rans_divergence().unwrap().contains("not finite"));
        // Too short to judge.
        assert_eq!(mk(&blow[..12]).rans_divergence(), None);
    }
}
