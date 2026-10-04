// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025-2026 西濱大将 (NISHIHAMA Daisuke)
//! Fills the regular [`AeroTable`] grid from the solved (Mach, alpha) points.
//!
//! Per solved Mach column (alpha direction): `CN` and the nose moment `CN xcp` are odd in alpha
//! and anchored at `(0, 0)`; they and the CFD axial coefficient are interpolated with monotone
//! cubics (PCHIP) in alpha. Beyond the solved alpha range the end slope is continued
//! (`Extrapolation::Linear`) or the end value is held (`Clamp`). `CNa` and `xcp(0)` come from an
//! odd cubic fit `c1 a + c3 a^3` through the two lowest positive solved angles. Failed cases
//! (`coeffs = None`) are simply left out of their column. Across Mach number every quantity is then
//! interpolated with PCHIP on the table's Mach grid (same extrapolation rule), so a failed
//! interior Mach column is bridged by its neighbours; a failed first or last Mach column cannot be
//! bridged and is an error. Finally hybrid viscous terms ([`crate::forces::hybrid_axial`]) are
//! added and the pitch-damping sums are scaled to the CFD slope and centroid.

use crate::config::FlowModel;
use crate::forces::{hybrid_axial, scale_damping, BodyCoeffs};
use aero::table::{AeroCoeffs, TableMeta};
use aero::{AeroModel, AeroTable, Extrapolation};
use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};

/// One solved (or failed) point of the case matrix.
#[derive(Debug, Clone, Copy)]
pub struct SolvedPoint {
    pub mach: f64,
    pub alpha_deg: f64,
    pub coeffs: Option<BodyCoeffs>,
}

/// Removes the numerical zero-angle offset of the normal force and nose moment.
///
/// The body is axisymmetric / mirror symmetric, so `CN(0) = 0` exactly; the steady solution on an
/// unstructured mesh can nevertheless carry a (sometimes large, mesh-dependent) asymmetric offset,
/// most visibly in the subsonic Euler wake. Per Mach number the `alpha = 0` values of `CN` and the
/// nose moment are subtracted from all angles of that Mach number (and the `alpha = 0` point itself
/// becomes 0). The axial coefficient is left alone. Returns the removed `(mach, CN0, moment0)`.
pub fn remove_zero_offset(points: &mut [SolvedPoint]) -> Vec<(f64, f64, f64)> {
    let mut removed = Vec::new();
    let zero: Vec<(f64, BodyCoeffs)> = points.iter().filter(|p| p.alpha_deg == 0.0).filter_map(|p| p.coeffs.map(|c| (p.mach, c))).collect();
    for (m, c0) in zero {
        for p in points.iter_mut().filter(|p| p.mach == m) {
            if let Some(c) = &mut p.coeffs {
                c.cn -= c0.cn;
                c.mom -= c0.mom;
            }
        }
        removed.push((m, c0.cn, c0.mom));
    }
    removed
}

/// What the table builder derived.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TableFill {
    /// Cases that failed and were bridged by interpolation.
    pub filled: Vec<(f64, f64)>,
    /// Per usable solved Mach: `(mach, CNa [1/rad], xcp at alpha -> 0 [m])`.
    pub slopes: Vec<(f64, f64, f64)>,
}

/// Monotone cubic (Fritsch-Carlson) interpolant with selectable extrapolation.
struct Pchip {
    xs: Vec<f64>,
    ys: Vec<f64>,
    m: Vec<f64>,
}

impl Pchip {
    fn new(xs: Vec<f64>, ys: Vec<f64>) -> Self {
        let n = xs.len();
        let mut m = vec![0.0; n];
        if n >= 2 {
            let h: Vec<f64> = xs.windows(2).map(|w| w[1] - w[0]).collect();
            let d: Vec<f64> = (0..n - 1).map(|i| (ys[i + 1] - ys[i]) / h[i]).collect();
            if n == 2 {
                m = vec![d[0]; 2];
            } else {
                for i in 1..n - 1 {
                    if d[i - 1] * d[i] > 0.0 {
                        let (w1, w2) = (2.0 * h[i] + h[i - 1], h[i] + 2.0 * h[i - 1]);
                        m[i] = (w1 + w2) / (w1 / d[i - 1] + w2 / d[i]);
                    }
                }
                let end = |h0: f64, h1: f64, d0: f64, d1: f64| {
                    let s = ((2.0 * h0 + h1) * d0 - h0 * d1) / (h0 + h1);
                    if s * d0 <= 0.0 {
                        0.0
                    } else if d0 * d1 <= 0.0 && s.abs() > 3.0 * d0.abs() {
                        3.0 * d0
                    } else {
                        s
                    }
                };
                m[0] = end(h[0], h[1], d[0], d[1]);
                m[n - 1] = end(h[n - 2], h[n - 3], d[n - 2], d[n - 3]);
            }
        }
        Self { xs, ys, m }
    }

    fn eval(&self, x: f64, linear: bool) -> f64 {
        let n = self.xs.len();
        if x <= self.xs[0] {
            return self.ys[0] + if linear { self.m[0] * (x - self.xs[0]) } else { 0.0 };
        }
        if x >= self.xs[n - 1] {
            return self.ys[n - 1] + if linear { self.m[n - 1] * (x - self.xs[n - 1]) } else { 0.0 };
        }
        let i = self.xs.partition_point(|&v| v <= x) - 1;
        geom::math::hermite(x, self.xs[i], self.xs[i + 1], self.ys[i], self.ys[i + 1], self.m[i], self.m[i + 1])
    }
}

fn grid(lo: f64, hi: f64, step: f64) -> Vec<f64> {
    let n = ((hi - lo) / step).round().max(1.0) as usize;
    (0..=n).map(|i| lo + (hi - lo) * i as f64 / n as f64).collect()
}

/// Slope at alpha -> 0 of an odd function from samples `(alpha_rad, value)`: fit `c1 a + c3 a^3`
/// through the two lowest points (falls back to the secant when the fit looks unreasonable).
fn odd_slope(pts: &[(f64, f64)]) -> f64 {
    let (a1, v1) = pts[0];
    let r1 = v1 / a1;
    if pts.len() < 2 {
        return r1;
    }
    let (a2, v2) = pts[1];
    let r2 = v2 / a2;
    let c3 = (r2 - r1) / (a2 * a2 - a1 * a1);
    let c1 = r1 - c3 * a1 * a1;
    if c1.is_finite() && (c1 - r1).abs() <= 0.5 * r1.abs().max(1e-9) { c1 } else { r1 }
}

/// Table-grid values of one solved Mach column.
struct Column {
    mach: f64,
    cn: Vec<f64>,
    mom: Vec<f64>,
    ca: Vec<f64>,
    cna: f64,
    xcp0: f64,
}

fn column(mach: f64, pts: &[(f64, BodyCoeffs)], alphas: &[f64], linear: bool) -> Option<Column> {
    let pos: Vec<&(f64, BodyCoeffs)> = pts.iter().filter(|p| p.0 > 0.0).collect();
    if pos.is_empty() {
        return None;
    }
    let mut xs = vec![0.0];
    xs.extend(pos.iter().map(|p| p.0));
    let cn_p = Pchip::new(xs.clone(), std::iter::once(0.0).chain(pos.iter().map(|p| p.1.cn)).collect());
    let mom_p = Pchip::new(xs, std::iter::once(0.0).chain(pos.iter().map(|p| p.1.mom)).collect());
    let ca_p = Pchip::new(pts.iter().map(|p| p.0).collect(), pts.iter().map(|p| p.1.ca).collect());
    let rad = |p: &&(f64, BodyCoeffs)| p.0.to_radians();
    let cn_pts: Vec<(f64, f64)> = pos.iter().map(|p| (rad(p), p.1.cn)).collect();
    let mom_pts: Vec<(f64, f64)> = pos.iter().map(|p| (rad(p), p.1.mom)).collect();
    let cna = odd_slope(&cn_pts);
    let m1 = odd_slope(&mom_pts);
    let xcp0 = if cna.abs() > 1e-9 { m1 / cna } else { 0.0 };
    Some(Column {
        mach,
        cn: alphas.iter().map(|&a| cn_p.eval(a, linear)).collect(),
        mom: alphas.iter().map(|&a| mom_p.eval(a, linear)).collect(),
        ca: alphas.iter().map(|&a| ca_p.eval(a, linear)).collect(),
        cna,
        xcp0,
    })
}

/// Builds the table on the grid of `model.opt` (Mach `mach_min..mach_max`, alpha `0..alpha_max`).
pub fn build_from_points(
    points: &[SolvedPoint],
    model: &AeroModel,
    flow: FlowModel,
    source_hash: String,
    extrapolation: Extrapolation,
) -> Result<(AeroTable, TableFill)> {
    let linear = extrapolation == Extrapolation::Linear;
    let o = &model.opt;
    let machs = grid(o.mach_min.max(0.0), o.mach_max, o.mach_step);
    let alphas = grid(0.0, o.alpha_max_deg, o.alpha_step_deg);

    let mut solved: Vec<f64> = points.iter().map(|p| p.mach).collect();
    solved.sort_by(|a, b| a.partial_cmp(b).unwrap());
    solved.dedup();
    if solved.is_empty() {
        bail!("no CFD cases to build a table from");
    }
    let mut fill = TableFill::default();
    let mut cols: Vec<Column> = Vec::new();
    for &m in &solved {
        let mut pts: Vec<(f64, BodyCoeffs)> = Vec::new();
        for p in points.iter().filter(|p| p.mach == m) {
            match p.coeffs {
                Some(c) => pts.push((p.alpha_deg, c)),
                None => fill.filled.push((m, p.alpha_deg)),
            }
        }
        pts.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        if let Some(c) = column(m, &pts, &alphas, linear) {
            fill.slopes.push((m, c.cna, c.xcp0));
            cols.push(c);
        }
    }
    for (edge, m) in [("lowest", solved[0]), ("highest", *solved.last().unwrap())] {
        if !cols.iter().any(|c| c.mach == m) {
            bail!("all CFD cases at the {edge} Mach number ({m}) failed (or have no angle of attack above 0); the table cannot be bridged there");
        }
    }

    let mx: Vec<f64> = cols.iter().map(|c| c.mach).collect();
    let across = |f: &dyn Fn(&Column) -> f64| Pchip::new(mx.clone(), cols.iter().map(f).collect());
    let cna_p = across(&|c| c.cna);
    let x0_p = across(&|c| c.xcp0);
    let per_alpha = |sel: fn(&Column) -> &Vec<f64>| -> Vec<Pchip> {
        (0..alphas.len()).map(|j| Pchip::new(mx.clone(), cols.iter().map(|c| sel(c)[j]).collect())).collect()
    };
    let (cn_p, mom_p, ca_p) = (per_alpha(|c| &c.cn), per_alpha(|c| &c.mom), per_alpha(|c| &c.ca));

    let mut rows: Vec<Vec<AeroCoeffs>> = Vec::with_capacity(machs.len());
    for &m in &machs {
        let (cna, xcp0) = (cna_p.eval(m, linear), x0_p.eval(m, linear));
        let damp = scale_damping(model.damping_sums(m), cna, xcp0);
        let row = alphas
            .iter()
            .enumerate()
            .map(|(j, &a)| {
                let (cn, mom) = if a == 0.0 { (0.0, 0.0) } else { (cn_p[j].eval(m, linear), mom_p[j].eval(m, linear)) };
                let xcp = if cn.abs() > 1e-9 { mom / cn } else { xcp0 };
                let (ca_on, ca_off) = hybrid_axial(flow, model, m, a.to_radians(), ca_p[j].eval(m, linear));
                AeroCoeffs { cn, ca_on, ca_off, xcp, cna, damp }
            })
            .collect();
        rows.push(row);
    }
    let g = &model.geom;
    let meta = TableMeta {
        source_hash,
        ref_area: g.ref_area,
        ref_diameter: 2.0 * g.ref_radius,
        length: g.length,
        machs: machs.clone(),
        alphas_deg: alphas.clone(),
        extrapolation,
    };
    let table = AeroTable::from_fn(meta, |m, a| {
        let i = machs.iter().position(|&v| v == m).unwrap();
        let j = alphas.iter().position(|&v| v == a).unwrap();
        rows[i][j]
    });
    Ok((table, fill))
}

#[cfg(test)]
mod tests {
    use super::*;
    use aero::AeroOptions;
    use geom::sample::SampleRocket;
    use geom::{extract, ExtractOptions};

    fn model() -> AeroModel {
        let g = extract(&SampleRocket::default().mesh(), &ExtractOptions::default()).unwrap();
        AeroModel::new(g, AeroOptions { mach_max: 3.0, mach_step: 0.25, alpha_max_deg: 16.0, alpha_step_deg: 2.0, ..Default::default() })
    }

    // Analytic field: CN = (2 + 0.3 M) sin 2a, nose moment = CN (1.0 + 0.05 M), CA = 0.2 + 0.1 M + 0.002 a^2 (a in deg).
    fn cn(m: f64, a: f64) -> f64 {
        (2.0 + 0.3 * m) * (2.0 * a.to_radians()).sin()
    }
    fn xcp(m: f64) -> f64 {
        1.0 + 0.05 * m
    }
    fn ca(m: f64, a: f64) -> f64 {
        0.2 + 0.1 * m + 0.002 * a * a
    }

    fn points(machs: &[f64], alphas: &[f64]) -> Vec<SolvedPoint> {
        let mut v = Vec::new();
        for &m in machs {
            for &a in alphas {
                v.push(SolvedPoint { mach: m, alpha_deg: a, coeffs: Some(BodyCoeffs { cn: cn(m, a), ca: ca(m, a), mom: cn(m, a) * xcp(m) }) });
            }
        }
        v
    }

    const MACHS: [f64; 6] = [0.5, 1.0, 1.5, 2.0, 2.5, 3.0];
    const ALPHAS: [f64; 6] = [0.0, 2.0, 4.0, 8.0, 12.0, 16.0];

    #[test]
    fn zero_offset_is_removed_per_mach() {
        let mut p = vec![
            SolvedPoint { mach: 0.5, alpha_deg: 0.0, coeffs: Some(BodyCoeffs { cn: 0.3, ca: 0.1, mom: 0.4 }) },
            SolvedPoint { mach: 0.5, alpha_deg: 4.0, coeffs: Some(BodyCoeffs { cn: 1.3, ca: 0.1, mom: 1.9 }) },
            SolvedPoint { mach: 2.0, alpha_deg: 0.0, coeffs: None },
            SolvedPoint { mach: 2.0, alpha_deg: 4.0, coeffs: Some(BodyCoeffs { cn: 1.0, ca: 0.2, mom: 1.0 }) },
        ];
        let r = remove_zero_offset(&mut p);
        assert_eq!(r, vec![(0.5, 0.3, 0.4)]);
        assert_eq!(p[0].coeffs.unwrap().cn, 0.0);
        assert!((p[1].coeffs.unwrap().cn - 1.0).abs() < 1e-12 && (p[1].coeffs.unwrap().mom - 1.5).abs() < 1e-12);
        assert_eq!(p[1].coeffs.unwrap().ca, 0.1);
        assert_eq!(p[3].coeffs.unwrap().cn, 1.0, "no alpha = 0 result: unchanged");
    }

    #[test]
    fn matches_analytic_field() {
        let m = model();
        let (t, fill) = build_from_points(&points(&MACHS, &ALPHAS), &m, FlowModel::Rans, "h".into(), Extrapolation::Linear).unwrap();
        assert!(fill.filled.is_empty());
        for &mach in &[0.75, 1.0, 1.75, 2.5] {
            for &a in &[2.0f64, 5.0, 10.0, 16.0] {
                let c = t.lookup(mach, a.to_radians());
                assert!((c.cn - cn(mach, a)).abs() < 4e-3, "cn at M={mach} a={a}: {} vs {}", c.cn, cn(mach, a));
                assert!((c.xcp - xcp(mach)).abs() < 3e-3, "xcp {}", c.xcp);
                let k = aero::model::axial_alpha_factor(a.to_radians());
                let extra = m.cd0(mach, false).extra * k;
                assert!((c.ca_off - (ca(mach, a) + extra)).abs() < 3e-3, "ca {} vs {}", c.ca_off, ca(mach, a));
            }
        }
        let c0 = t.lookup(1.5, 0.0);
        assert!(c0.cn.abs() < 1e-12);
        assert!((c0.cna - 2.0 * (2.0 + 0.45)).abs() < 2e-3, "cna {}", c0.cna);
        assert!((c0.xcp - xcp(1.5)).abs() < 1e-6);
        assert!((c0.damp[0] - c0.cna).abs() < 1e-12 && (c0.damp[1] / c0.damp[0] - c0.xcp).abs() < 1e-9);
        // Beyond the solved alpha range the end slope is continued (linear) or held (clamp) up to the table edge only;
        // the table grid ends at 16 deg here, so check the lookup extrapolation instead.
        assert!(t.lookup(1.5, 20f64.to_radians()).cn > t.lookup(1.5, 16f64.to_radians()).cn - 0.05);
    }

    #[test]
    fn extrapolates_in_alpha_and_mach() {
        let g = extract(&SampleRocket::default().mesh(), &ExtractOptions::default()).unwrap();
        let o = AeroOptions { mach_min: 0.0, mach_max: 4.0, mach_step: 0.5, alpha_max_deg: 24.0, alpha_step_deg: 4.0, ..Default::default() };
        let m = AeroModel::new(g, o);
        let (lin, _) = build_from_points(&points(&MACHS, &ALPHAS), &m, FlowModel::Rans, "h".into(), Extrapolation::Linear).unwrap();
        let (cl, _) = build_from_points(&points(&MACHS, &ALPHAS), &m, FlowModel::Rans, "h".into(), Extrapolation::Clamp).unwrap();
        let (a16, a24) = (16f64.to_radians(), 24f64.to_radians());
        assert!((cl.lookup(2.0, a24).cn - cl.lookup(2.0, a16).cn).abs() < 1e-12);
        assert!(lin.lookup(2.0, a24).cn != cl.lookup(2.0, a24).cn);
        // Mach beyond the last solved value: clamp holds the M = 3 value.
        assert!((cl.lookup(4.0, a16).cn - cl.lookup(3.0, a16).cn).abs() < 1e-12);
    }

    #[test]
    fn failed_cases_are_bridged() {
        let m = model();
        let mut p = points(&MACHS, &ALPHAS);
        for q in &mut p {
            if (q.mach == 1.5 && q.alpha_deg == 8.0) || (q.mach == 2.0) {
                q.coeffs = None;
            }
        }
        let (t, fill) = build_from_points(&p, &m, FlowModel::Euler, "h".into(), Extrapolation::Linear).unwrap();
        assert_eq!(fill.filled.len(), 1 + ALPHAS.len());
        assert_eq!(fill.slopes.len(), MACHS.len() - 1);
        let c = t.lookup(2.0, 8f64.to_radians());
        assert!((c.cn - cn(2.0, 8.0)).abs() < 8e-3, "{} vs {}", c.cn, cn(2.0, 8.0));
        let c = t.lookup(1.5, 8f64.to_radians());
        assert!((c.cn - cn(1.5, 8.0)).abs() < 8e-3);
    }

    #[test]
    fn missing_edge_column_is_an_error() {
        let m = model();
        let mut p = points(&MACHS, &ALPHAS);
        for q in &mut p {
            if q.mach == 3.0 {
                q.coeffs = None;
            }
        }
        let e = build_from_points(&p, &m, FlowModel::Euler, "h".into(), Extrapolation::Linear).unwrap_err().to_string();
        assert!(e.contains("highest") && e.contains('3'), "{e}");
        assert!(build_from_points(&[], &m, FlowModel::Euler, "h".into(), Extrapolation::Linear).is_err());
    }
}
