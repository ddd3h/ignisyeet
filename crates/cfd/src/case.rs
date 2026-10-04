// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025-2026 西濱大将 (NISHIHAMA Daisuke)
//! Case matrix, directory names, input hashes and the `done.json` status file.

use crate::config::{AlphaMode, CfdOptions};
use crate::forces::BodyCoeffs;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// One (Mach, alpha) case.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CaseSpec {
    pub mach: f64,
    pub alpha_deg: f64,
}

/// All cases ordered for warm starts: by Mach, then `alpha = 0`, the positive angles ascending and,
/// in `mirror` mode, the negative angles by increasing magnitude (each configured `alpha > 0` is
/// also solved at `-alpha`).
pub fn case_list(opt: &CfdOptions) -> Vec<CaseSpec> {
    let mut machs = opt.machs.clone();
    let mut alphas = opt.alphas_deg.clone();
    machs.sort_by(|a, b| a.partial_cmp(b).unwrap());
    alphas.sort_by(|a, b| a.partial_cmp(b).unwrap());
    // With an exactly z-symmetric mesh the -alpha solution is the mirror image of +alpha: not solved
    // (the mirror combination then reduces to the +alpha result, whose alpha = 0 offset is zero).
    if opt.alpha_mode == AlphaMode::Mirror && !opt.z_mirror_mesh {
        let neg: Vec<f64> = alphas.iter().filter(|&&a| a > 0.0).map(|&a| -a).collect();
        alphas.extend(neg);
    }
    machs.iter().flat_map(|&mach| alphas.iter().map(move |&alpha_deg| CaseSpec { mach, alpha_deg })).collect()
}

/// Index (into `cases`) of the case whose restart file seeds case `i`: the next smaller angle of the
/// same sign at the same Mach (or `alpha = 0` for the smallest), and for `alpha = 0` the `alpha = 0`
/// case of the previous Mach.
pub fn warm_start_source(cases: &[CaseSpec], i: usize) -> Option<usize> {
    let c = cases[i];
    if c.alpha_deg != 0.0 {
        let same = |x: &CaseSpec| x.mach == c.mach && (x.alpha_deg == 0.0 || x.alpha_deg.signum() == c.alpha_deg.signum()) && x.alpha_deg.abs() < c.alpha_deg.abs();
        return (0..cases.len()).filter(|&j| same(&cases[j])).max_by(|&p, &q| cases[p].alpha_deg.abs().partial_cmp(&cases[q].alpha_deg.abs()).unwrap());
    }
    let prev_mach = cases.iter().map(|x| x.mach).filter(|&m| m < c.mach).fold(f64::NEG_INFINITY, f64::max);
    if prev_mach.is_finite() { cases.iter().position(|x| x.mach == prev_mach && x.alpha_deg == 0.0) } else { None }
}

impl CaseSpec {
    /// Directory name, e.g. `m0.950_a04.00` or `m0.950_a-04.00` for a negative angle.
    pub fn dir_name(&self) -> String {
        let sign = if self.alpha_deg < 0.0 { "-" } else { "" };
        format!("m{:.3}_a{sign}{:05.2}", self.mach, self.alpha_deg.abs())
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
    fn done_file_respects_hash() {
        let dir = std::env::temp_dir().join(format!("ignisyeet_cfd_case_{}", std::process::id()));
        let h = case_hash("a", "m");
        assert_ne!(h, case_hash("b", "m"));
        let r = CaseResult {
            input_hash: h.clone(),
            mach: 2.0,
            alpha_deg: 4.0,
            coeffs: BodyCoeffs { cn: 0.1, ca: 0.3, mom: 0.08 },
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
