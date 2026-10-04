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
        let mut rows = Vec::new();
        for l in lines {
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

impl History {
    /// log10 RMS density residual of the last row.
    pub fn last_residual(&self) -> Option<f64> {
        let c = self.col(&["rms[Rho]", "Res_Flow[0]", "rms[P]"])?;
        self.rows.last().map(|r| r[c]).filter(|x| x.is_finite())
    }

    /// Orders of magnitude between the largest density residual of the run and the last one.
    pub fn residual_drop_from_peak(&self) -> Option<f64> {
        let c = self.col(&["rms[Rho]", "Res_Flow[0]", "rms[P]"])?;
        let peak = self.rows.iter().map(|r| r[c]).filter(|x| x.is_finite()).fold(f64::NEG_INFINITY, f64::max);
        Some(peak - self.last_residual()?).filter(|d| d.is_finite())
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
    pub fn coefficients_stable(&self, window: usize) -> bool {
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
        ok(fx, 0.02 * scale + 5e-4) && ok(fz, 0.02 * scale + 5e-4) && ok(my, 0.25 * scale + 5e-3)
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
}
