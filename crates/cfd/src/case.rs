// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025-2026 西濱大将 (NISHIHAMA Daisuke)
//! Case matrix, directory names, input hashes and the `done.json` status file.

use crate::config::{AlphaMode, CfdOptions, ShapeSymmetry};
use crate::forces::BodyCoeffs6;
use crate::forces::BodyCoeffs;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// One case of the matrix: Mach number, angle of attack, sideslip angle and, for the roll-damping
/// runs, the non-dimensional roll rate `p d / (2 V)` (0 = no rotation).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CaseSpec {
    pub mach: f64,
    pub alpha_deg: f64,
    #[serde(default)]
    pub beta_deg: f64,
    #[serde(default)]
    pub roll_hat: f64,
}

impl CaseSpec {
    /// A case at zero sideslip without rotation.
    pub fn new(mach: f64, alpha_deg: f64) -> Self {
        Self { mach, alpha_deg, beta_deg: 0.0, roll_hat: 0.0 }
    }

    /// Roll-damping run (`alpha = beta = 0`) at the roll rate `roll_hat`.
    pub fn roll(mach: f64, roll_hat: f64) -> Self {
        Self { mach, alpha_deg: 0.0, beta_deg: 0.0, roll_hat }
    }

    pub fn is_roll(&self) -> bool {
        self.roll_hat != 0.0
    }
}

/// `[0, +x ascending, -x by increasing magnitude]` of the non-negative list `pos` (which starts with 0).
fn with_negatives(pos: &[f64], negatives: bool) -> Vec<f64> {
    let mut v = pos.to_vec();
    if negatives {
        v.extend(pos.iter().filter(|&&a| a > 0.0).map(|&a| -a));
    }
    v
}

/// All cases ordered for warm starts: by Mach, then sideslip (`0`, the positive angles ascending, the
/// negative ones by magnitude), then the angles of attack in the same order, and finally the roll pair
/// of the Mach number (`alpha = beta = 0`, `+-roll_rate_hat`) when `roll_damping` is on.
///
/// Which signs are solved follows the shape symmetry ([`CfdOptions::shape`]):
///
/// * `axisymmetric` (mirror symmetric about y = 0 and z = 0): `alpha >= 0` and `beta >= 0` only; in
///   `mirror` mode without an exactly z-symmetric mesh each `alpha > 0` is also solved at `-alpha`
///   (odd/even combination, see [`crate::table::apply_alpha_mode`]).
/// * `pitch_plane` (y = 0 only): `+-alpha`, `beta >= 0`.
/// * `none`: `+-alpha` and `+-beta`.
pub fn case_list(opt: &CfdOptions) -> Vec<CaseSpec> {
    let mut machs = opt.machs.clone();
    let mut alphas = opt.alphas_deg.clone();
    let mut betas = opt.betas_deg.clone();
    machs.sort_by(|a, b| a.partial_cmp(b).unwrap());
    alphas.sort_by(|a, b| a.partial_cmp(b).unwrap());
    betas.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let shape = opt.shape();
    // With an exactly z-symmetric mesh the -alpha solution is the mirror image of +alpha: not solved
    // (the mirror combination then reduces to the +alpha result, whose alpha = 0 offset is zero).
    let neg_alpha = shape != ShapeSymmetry::Axisymmetric || (opt.alpha_mode == AlphaMode::Mirror && !opt.z_mirror_mesh);
    let alphas = with_negatives(&alphas, neg_alpha);
    let betas = with_negatives(&betas, shape == ShapeSymmetry::None);
    let mut out = Vec::new();
    for &mach in &machs {
        for &beta_deg in &betas {
            for &alpha_deg in &alphas {
                out.push(CaseSpec { mach, alpha_deg, beta_deg, roll_hat: 0.0 });
            }
        }
        if opt.roll_damping {
            out.push(CaseSpec::roll(mach, opt.roll_rate_hat));
            out.push(CaseSpec::roll(mach, -opt.roll_rate_hat));
        }
    }
    out
}

/// Index (into `cases`) of the case whose restart file seeds case `i`:
///
/// * roll runs: the non-rotating `alpha = beta = 0` case of the same Mach;
/// * `alpha != 0`: the next smaller angle of the same sign at the same Mach and sideslip (or `alpha = 0`);
/// * `alpha = 0`, `beta != 0`: the next smaller sideslip of the same sign at `alpha = 0` (or `beta = 0`);
/// * `alpha = beta = 0`: the same case at the previous Mach.
pub fn warm_start_source(cases: &[CaseSpec], i: usize) -> Option<usize> {
    let c = cases[i];
    let plain = |x: &CaseSpec| !x.is_roll() && x.mach == c.mach;
    let smaller = |v: f64, w: f64| (v == 0.0 || v.signum() == w.signum()) && v.abs() < w.abs();
    if c.is_roll() {
        return cases.iter().position(|x| plain(x) && x.alpha_deg == 0.0 && x.beta_deg == 0.0);
    }
    let best = |sel: &dyn Fn(&CaseSpec) -> bool, key: &dyn Fn(&CaseSpec) -> f64| {
        (0..cases.len()).filter(|&j| sel(&cases[j])).max_by(|&p, &q| key(&cases[p]).abs().partial_cmp(&key(&cases[q]).abs()).unwrap())
    };
    if c.alpha_deg != 0.0 {
        return best(&|x| plain(x) && x.beta_deg == c.beta_deg && smaller(x.alpha_deg, c.alpha_deg), &|x| x.alpha_deg);
    }
    if c.beta_deg != 0.0 {
        return best(&|x| plain(x) && x.alpha_deg == 0.0 && smaller(x.beta_deg, c.beta_deg), &|x| x.beta_deg);
    }
    let prev_mach = cases.iter().map(|x| x.mach).filter(|&m| m < c.mach).fold(f64::NEG_INFINITY, f64::max);
    if prev_mach.is_finite() {
        cases.iter().position(|x| !x.is_roll() && x.mach == prev_mach && x.alpha_deg == 0.0 && x.beta_deg == 0.0)
    } else {
        None
    }
}

fn signed(v: f64, width: usize, prec: usize) -> String {
    format!("{}{:0width$.prec$}", if v < 0.0 { "-" } else { "" }, v.abs())
}

impl CaseSpec {
    /// Directory name, e.g. `m0.950_a04.00` or `m0.950_a-04.00` for a negative angle; a sideslip adds
    /// `_b04.00`, a roll-damping run `_r+0.0100` / `_r-0.0100`.
    pub fn dir_name(&self) -> String {
        let mut s = format!("m{:.3}_a{}", self.mach, signed(self.alpha_deg, 5, 2));
        if self.beta_deg != 0.0 {
            s += &format!("_b{}", signed(self.beta_deg, 5, 2));
        }
        if self.roll_hat != 0.0 {
            s += &format!("_r{}{:.4}", if self.roll_hat < 0.0 { "-" } else { "+" }, self.roll_hat.abs());
        }
        s
    }

    pub fn dir(&self, workdir: &Path) -> PathBuf {
        workdir.join(self.dir_name())
    }
}

/// FNV-1a hash of arbitrary parts, 16 hex digits.
pub fn hash_parts(parts: &[&[u8]]) -> String {
    let mut h = 0xcbf2_9ce4_8422_2325u64;
    for p in parts {
        for b in p.iter().chain(&[0xffu8]) {
            h ^= *b as u64;
            h = h.wrapping_mul(0x100_0000_01b3);
        }
    }
    format!("{h:016x}")
}

/// Hash of everything that determines a case result: its SU2 configuration text and the mesh hash.
pub fn case_hash(cfg_text: &str, mesh_hash: &str) -> String {
    hash_parts(&[cfg_text.as_bytes(), mesh_hash.as_bytes()])
}

/// Result of a finished case, stored as `done.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaseResult {
    pub input_hash: String,
    pub mach: f64,
    pub alpha_deg: f64,
    pub coeffs: BodyCoeffs,
    /// All six coefficients (absent in results written before they were stored).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub coeffs6: Option<BodyCoeffs6>,
    #[serde(default)]
    pub beta_deg: f64,
    #[serde(default)]
    pub roll_hat: f64,
    pub iterations: usize,
    /// The residual reached the target (`false`: accepted as practically converged, see `runner`).
    pub converged: bool,
    pub residual_first: Option<f64>,
    pub residual_last: Option<f64>,
    pub wall_seconds: f64,
}

const DONE: &str = "done.json";

pub fn write_done(dir: &Path, r: &CaseResult) -> Result<()> {
    std::fs::create_dir_all(dir).with_context(|| format!("cannot create {}", dir.display()))?;
    std::fs::write(dir.join(DONE), serde_json::to_string_pretty(r)?).with_context(|| format!("cannot write {}", dir.join(DONE).display()))
}

/// The stored result when it exists, parses and was produced from `input_hash`.
pub fn read_done(dir: &Path, input_hash: &str) -> Option<CaseResult> {
    let r: CaseResult = serde_json::from_str(&std::fs::read_to_string(dir.join(DONE)).ok()?).ok()?;
    (r.input_hash == input_hash).then_some(r)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ShapeSymmetry;

    #[test]
    fn ordering_names_and_warm_starts() {
        let o = CfdOptions { machs: vec![0.5, 2.0], alphas_deg: vec![0.0, 4.0, 8.0], alpha_mode: AlphaMode::Single, ..Default::default() };
        let c = case_list(&o);
        assert_eq!(c.len(), 6);
        assert_eq!((c[0].mach, c[0].alpha_deg, c[5].mach, c[5].alpha_deg), (0.5, 0.0, 2.0, 8.0));
        assert_eq!(c[1].dir_name(), "m0.500_a04.00");
        assert_eq!(warm_start_source(&c, 0), None);
        assert_eq!(warm_start_source(&c, 2), Some(1));
        assert_eq!(warm_start_source(&c, 3), Some(0));
        assert_eq!(case_list(&CfdOptions { alpha_mode: AlphaMode::Offset, ..o.clone() }), c);
    }

    #[test]
    fn mirror_mode_adds_negative_angles() {
        let o = CfdOptions { machs: vec![2.0, 0.5], alphas_deg: vec![0.0, 4.0, 8.0], z_mirror_mesh: false, ..Default::default() };
        assert_eq!(o.alpha_mode, AlphaMode::Mirror);
        // z-symmetric mesh: no negative angles needed.
        assert_eq!(case_list(&CfdOptions { z_mirror_mesh: true, ..o.clone() }).len(), 6);
        let c = case_list(&o);
        let a: Vec<f64> = c.iter().map(|x| x.alpha_deg).collect();
        assert_eq!(c.len(), 10);
        assert_eq!(&a[..5], &[0.0, 4.0, 8.0, -4.0, -8.0]);
        assert_eq!((c[0].mach, c[5].mach), (0.5, 2.0));
        let names: Vec<String> = c.iter().map(|x| x.dir_name()).collect();
        assert_eq!(names[3], "m0.500_a-04.00");
        assert_eq!(names.iter().collect::<std::collections::BTreeSet<_>>().len(), c.len());
        // Warm starts: both branches grow from alpha = 0; alpha = 0 chains over Mach.
        assert_eq!(warm_start_source(&c, 0), None);
        assert_eq!(warm_start_source(&c, 1), Some(0));
        assert_eq!(warm_start_source(&c, 2), Some(1));
        assert_eq!(warm_start_source(&c, 3), Some(0));
        assert_eq!(warm_start_source(&c, 4), Some(3));
        assert_eq!(warm_start_source(&c, 5), Some(0));
        assert_eq!(warm_start_source(&c, 8), Some(5));
        assert_eq!(warm_start_source(&c, 9), Some(8));
        // The case hash distinguishes the sign of the angle.
        let cfg = |alpha| crate::su2cfg::su2_config(&o, 0.5, alpha, &crate::su2cfg::RefDims { length: 0.1, area: 0.01 }, &aero::atmosphere::AtmosphereModel::default().at(0.0), &Default::default());
        assert_ne!(case_hash(&cfg(4.0), "m"), case_hash(&cfg(-4.0), "m"));
    }

    #[test]
    fn sideslip_and_roll_cases() {
        let base = CfdOptions { machs: vec![0.8, 2.0], alphas_deg: vec![0.0, 4.0], betas_deg: vec![0.0, 4.0], alpha_mode: AlphaMode::Single, ..Default::default() };
        // Axisymmetric shape: only the positive quadrant (mirror images are not solved).
        let c = case_list(&base);
        assert_eq!(c.len(), 8);
        // Pitch-plane symmetry: +-alpha, beta >= 0. No symmetry: +-alpha and +-beta.
        let pp = case_list(&CfdOptions { shape_symmetry: Some(ShapeSymmetry::PitchPlane), ..base.clone() });
        assert_eq!(pp.len(), 2 * 3 * 2);
        let none = case_list(&CfdOptions { shape_symmetry: Some(ShapeSymmetry::None), ..base.clone() });
        assert_eq!(none.len(), 2 * 3 * 3);
        assert!(none.iter().any(|x| x.beta_deg == -4.0 && x.alpha_deg == -4.0));
        let names: std::collections::BTreeSet<String> = none.iter().map(|x| x.dir_name()).collect();
        assert_eq!(names.len(), none.len());
        assert!(names.contains("m0.800_a-04.00_b-04.00") && names.contains("m2.000_a00.00"));
        // Existing names are unchanged without sideslip and roll.
        assert_eq!(CaseSpec::new(0.95, 4.0).dir_name(), "m0.950_a04.00");
        // Roll pair after the matrix of each Mach number.
        let r = case_list(&CfdOptions { roll_damping: true, roll_rate_hat: 0.01, ..base.clone() });
        assert_eq!(r.len(), 12);
        assert_eq!((r[4], r[5]), (CaseSpec::roll(0.8, 0.01), CaseSpec::roll(0.8, -0.01)));
        assert_eq!(r[4].dir_name(), "m0.800_a00.00_r+0.0100");
        assert_eq!(r[5].dir_name(), "m0.800_a00.00_r-0.0100");
        // Warm starts: sideslip chains from alpha = 0, alpha from the same sideslip, roll from the plain (0, 0).
        let at = |m: f64, a: f64, b: f64| r.iter().position(|x| *x == CaseSpec { mach: m, alpha_deg: a, beta_deg: b, roll_hat: 0.0 }).unwrap();
        assert_eq!(warm_start_source(&r, at(0.8, 4.0, 4.0)), Some(at(0.8, 0.0, 4.0)));
        assert_eq!(warm_start_source(&r, at(0.8, 0.0, 4.0)), Some(at(0.8, 0.0, 0.0)));
        assert_eq!(warm_start_source(&r, 4), Some(at(0.8, 0.0, 0.0)));
        assert_eq!(warm_start_source(&r, 5), Some(at(0.8, 0.0, 0.0)));
        assert_eq!(warm_start_source(&r, at(2.0, 0.0, 0.0)), Some(at(0.8, 0.0, 0.0)));
    }

    #[test]
    fn done_file_respects_hash() {
        let dir = std::env::temp_dir().join(format!("ignisyeet_cfd_case_{}", std::process::id()));
        let h = case_hash("a", "m");
        assert_ne!(h, case_hash("b", "m"));
        let r = CaseResult {
            input_hash: h.clone(),
            mach: 2.0,
            alpha_deg: 4.0,
            coeffs: BodyCoeffs { cn: 0.1, ca: 0.3, mom: 0.08 },
            coeffs6: None,
            beta_deg: 0.0,
            roll_hat: 0.0,
            iterations: 120,
            converged: true,
            residual_first: Some(-2.0),
            residual_last: Some(-6.1),
            wall_seconds: 12.5,
        };
        write_done(&dir, &r).unwrap();
        let back = read_done(&dir, &h).unwrap();
        assert_eq!(back.coeffs, r.coeffs);
        assert!(read_done(&dir, "other").is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
