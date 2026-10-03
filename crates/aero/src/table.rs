// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025-2026 西濱大将 (NISHIHAMA Daisuke)
//! Pre-computed aerodynamic coefficient table on a (Mach, alpha) grid, with CSV/JSON
//! persistence and bilinear interpolation / linear extrapolation.

use crate::model::{axial_alpha_factor, AeroModel};
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Extrapolation {
    /// Continue the slope of the outermost grid cell.
    #[default]
    Linear,
    /// Hold the value at the grid edge.
    Clamp,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableMeta {
    /// Hash of the inputs (mesh + options) the table was built from.
    pub source_hash: String,
    pub ref_area: f64,
    pub ref_diameter: f64,
    pub length: f64,
    pub machs: Vec<f64>,
    pub alphas_deg: Vec<f64>,
    pub extrapolation: Extrapolation,
}

/// Coefficients at one flight condition.
#[derive(Debug, Clone, Copy, Default)]
pub struct AeroCoeffs {
    pub cn: f64,
    pub ca_on: f64,
    pub ca_off: f64,
    /// Centre of pressure aft of the nose tip [m].
    pub xcp: f64,
    /// Normal-force slope at zero alpha [1/rad].
    pub cna: f64,
    /// Pitch-damping sums Σ CNa_i, Σ CNa_i x_i, Σ CNa_i x_i².
    pub damp: [f64; 3],
}

const N_FIELDS: usize = 8;
const HEADER: &str = "mach,alpha_deg,cn,ca_on,ca_off,xcp,cna,damp_s0,damp_s1,damp_s2";

#[derive(Debug, Clone)]
pub struct AeroTable {
    pub meta: TableMeta,
    /// Row-major [mach][alpha][field] values: cn, ca_on, ca_off, xcp, cna, s0, s1, s2.
    data: Vec<[f64; N_FIELDS]>,
}

fn grid(lo: f64, hi: f64, step: f64) -> Vec<f64> {
    let n = ((hi - lo) / step).round().max(1.0) as usize;
    (0..=n).map(|i| lo + (hi - lo) * i as f64 / n as f64).collect()
}

impl AeroTable {
    pub fn build(model: &AeroModel, source_hash: String, extrapolation: Extrapolation) -> Self {
        Self::build_with_progress(model, source_hash, extrapolation, &|_, _| {})
    }

    /// [`build`](Self::build) calling `progress(done_rows, total_rows)` after every Mach row.
    pub fn build_with_progress(model: &AeroModel, source_hash: String, extrapolation: Extrapolation, progress: &dyn Fn(usize, usize)) -> Self {
        let o = &model.opt;
        let machs = grid(o.mach_min.max(0.0), o.mach_max, o.mach_step);
        let alphas_deg = grid(0.0, o.alpha_max_deg, o.alpha_step_deg);
        let mut data = Vec::with_capacity(machs.len() * alphas_deg.len());
        for (row, &m) in machs.iter().enumerate() {
            let cd_on = model.cd0(m, true).total();
            let cd_off = model.cd0(m, false).total();
            let cna = model.cna(m);
            let s = model.damping_sums(m);
            for &a in &alphas_deg {
                let ar = a.to_radians();
                let (cn, xcp) = if a == 0.0 { (0.0, s[1] / s[0]) } else { model.normal(m, ar) };
                let k = axial_alpha_factor(ar);
                data.push([cn, cd_on * k, cd_off * k, xcp, cna, s[0], s[1], s[2]]);
            }
            progress(row + 1, machs.len());
        }
        let g = &model.geom;
        let meta = TableMeta {
            source_hash,
            ref_area: g.ref_area,
            ref_diameter: 2.0 * g.ref_radius,
            length: g.length,
            machs,
            alphas_deg,
            extrapolation,
        };
        Self { meta, data }
    }

    /// Table from an arbitrary function of (Mach, alpha [deg]); used for tests and custom data.
    pub fn from_fn(meta: TableMeta, f: impl Fn(f64, f64) -> AeroCoeffs) -> Self {
        let mut data = Vec::new();
        for &m in &meta.machs {
            for &a in &meta.alphas_deg {
                let c = f(m, a);
                data.push([c.cn, c.ca_on, c.ca_off, c.xcp, c.cna, c.damp[0], c.damp[1], c.damp[2]]);
            }
        }
        Self { meta, data }
    }

    pub fn save(&self, csv_path: &Path) -> Result<()> {
        let mut f = std::io::BufWriter::new(std::fs::File::create(csv_path).with_context(|| format!("cannot write {}", csv_path.display()))?);
        writeln!(f, "{HEADER}")?;
        let na = self.meta.alphas_deg.len();
        for (i, row) in self.data.iter().enumerate() {
            write!(f, "{},{}", self.meta.machs[i / na], self.meta.alphas_deg[i % na])?;
            for v in row {
                write!(f, ",{v:.9e}")?;
            }
            writeln!(f)?;
        }
        std::fs::write(csv_path.with_extension("json"), serde_json::to_string_pretty(&self.meta)?)?;
        Ok(())
    }

    pub fn load(csv_path: &Path) -> Result<Self> {
        let meta: TableMeta = serde_json::from_str(&std::fs::read_to_string(csv_path.with_extension("json"))?)?;
        let text = std::fs::read_to_string(csv_path).with_context(|| format!("cannot read {}", csv_path.display()))?;
        let mut data = Vec::new();
        for line in text.lines().skip(1).filter(|l| !l.trim().is_empty()) {
            let v: Vec<f64> = line.split(',').map(|s| s.trim().parse::<f64>()).collect::<Result<_, _>>()?;
            if v.len() != N_FIELDS + 2 {
                bail!("bad row in {}: {line}", csv_path.display());
            }
            data.push(v[2..].try_into().unwrap());
        }
        if data.len() != meta.machs.len() * meta.alphas_deg.len() {
            bail!("aero table size does not match its metadata");
        }
        Ok(Self { meta, data })
    }

    /// Coefficients at Mach `m` and total angle of attack `alpha` [rad] (≥ 0).
    pub fn lookup(&self, m: f64, alpha: f64) -> AeroCoeffs {
        let clamp = self.meta.extrapolation == Extrapolation::Clamp;
        let (i, tm) = locate(&self.meta.machs, m, clamp);
        let (j, ta) = locate(&self.meta.alphas_deg, alpha.abs().to_degrees(), clamp);
        let na = self.meta.alphas_deg.len();
        let at = |ii: usize, jj: usize| &self.data[ii * na + jj];
        let mut out = [0.0; N_FIELDS];
        let (i1, j1) = ((i + 1).min(self.meta.machs.len() - 1), (j + 1).min(na - 1));
        for (k, o) in out.iter_mut().enumerate() {
            let v00 = at(i, j)[k];
            let v01 = at(i, j1)[k];
            let v10 = at(i1, j)[k];
            let v11 = at(i1, j1)[k];
            let a0 = v00 + (v01 - v00) * ta;
            let a1 = v10 + (v11 - v10) * ta;
            *o = a0 + (a1 - a0) * tm;
        }
        AeroCoeffs {
            cn: out[0],
            ca_on: out[1].max(0.0),
            ca_off: out[2].max(0.0),
            xcp: out[3],
            cna: out[4],
            damp: [out[5], out[6], out[7]],
        }
    }

    pub fn rows(&self) -> usize {
        self.data.len()
    }
}

/// Cell index and fractional position; the fraction leaves [0, 1] when extrapolating.
fn locate(g: &[f64], x: f64, clamp: bool) -> (usize, f64) {
    if g.len() < 2 {
        return (0, 0.0);
    }
    let i = match g.iter().position(|&v| v > x) {
        Some(0) => 0,
        Some(k) => k - 1,
        None => g.len() - 2,
    }
    .min(g.len() - 2);
    let t = (x - g[i]) / (g[i + 1] - g[i]);
    (i, if clamp { t.clamp(0.0, 1.0) } else { t })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::AeroOptions;
    use geom::sample::SampleRocket;
    use geom::{extract, ExtractOptions};

    fn table(ext: Extrapolation) -> (AeroTable, AeroModel) {
        let g = extract(&SampleRocket::default().mesh(), &ExtractOptions::default()).unwrap();
        let opt = AeroOptions { mach_max: 2.0, mach_step: 0.1, alpha_max_deg: 10.0, alpha_step_deg: 2.0, ..Default::default() };
        let model = AeroModel::new(g, opt);
        (AeroTable::build(&model, "h".into(), ext), model)
    }

    #[test]
    fn interpolates_on_and_between_nodes() {
        let (t, m) = table(Extrapolation::Linear);
        let c = t.lookup(0.5, 4f64.to_radians());
        assert!((c.cn - m.normal(0.5, 4f64.to_radians()).0).abs() < 1e-9);
        let mid = t.lookup(0.55, 5f64.to_radians());
        let lo = t.lookup(0.5, 4f64.to_radians()).cn;
        let hi = t.lookup(0.6, 6f64.to_radians()).cn;
        assert!(mid.cn > lo && mid.cn < hi);
    }

    #[test]
    fn extrapolates_linearly_or_clamps() {
        let (t, _) = table(Extrapolation::Linear);
        let a8 = t.lookup(0.3, 8f64.to_radians()).cn;
        let a10 = t.lookup(0.3, 10f64.to_radians()).cn;
        let a12 = t.lookup(0.3, 12f64.to_radians()).cn;
        assert!((a12 - (2.0 * a10 - a8)).abs() < 1e-9);
        let (tc, _) = table(Extrapolation::Clamp);
        assert!((tc.lookup(0.3, 12f64.to_radians()).cn - a10).abs() < 1e-9);
        assert!((tc.lookup(5.0, 0.0).ca_off - tc.lookup(2.0, 0.0).ca_off).abs() < 1e-12);
    }

    #[test]
    fn save_load_roundtrip() {
        let (t, _) = table(Extrapolation::Linear);
        let p = std::env::temp_dir().join("ignisyeet_aero_test.csv");
        t.save(&p).unwrap();
        let u = AeroTable::load(&p).unwrap();
        let (a, b) = (t.lookup(1.23, 0.07), u.lookup(1.23, 0.07));
        assert!((a.cn - b.cn).abs() < 1e-7 && (a.xcp - b.xcp).abs() < 1e-7);
        assert_eq!(u.meta.source_hash, "h");
    }
}
