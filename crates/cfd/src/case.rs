// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025-2026 西濱大将 (NISHIHAMA Daisuke)
//! Case matrix, directory names, input hashes and the `done.json` status file.

use crate::config::CfdOptions;
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

/// All cases ordered for warm starts: by Mach, then alpha ascending.
pub fn case_list(opt: &CfdOptions) -> Vec<CaseSpec> {
    let mut machs = opt.machs.clone();
    let mut alphas = opt.alphas_deg.clone();
    machs.sort_by(|a, b| a.partial_cmp(b).unwrap());
    alphas.sort_by(|a, b| a.partial_cmp(b).unwrap());
    machs.iter().flat_map(|&mach| alphas.iter().map(move |&alpha_deg| CaseSpec { mach, alpha_deg })).collect()
}

/// Index (into `cases`) of the case whose restart file seeds case `i`: the previous alpha at
/// the same Mach, or for the lowest alpha the lowest-alpha case of the previous Mach.
pub fn warm_start_source(cases: &[CaseSpec], i: usize) -> Option<usize> {
    let c = cases[i];
    if i > 0 && cases[i - 1].mach == c.mach {
        return Some(i - 1);
    }
    let first_of = |m: f64| cases.iter().position(|x| x.mach == m);
    let prev_mach = cases.iter().map(|x| x.mach).filter(|&m| m < c.mach).fold(f64::NEG_INFINITY, f64::max);
    if prev_mach.is_finite() { first_of(prev_mach) } else { None }
}

impl CaseSpec {
    /// Directory name, e.g. `m0.950_a04.00` (sorts like the case order).
    pub fn dir_name(&self) -> String {
        format!("m{:.3}_a{:05.2}", self.mach, self.alpha_deg)
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
        let o = CfdOptions { machs: vec![0.5, 2.0], alphas_deg: vec![0.0, 4.0, 8.0], ..Default::default() };
        let c = case_list(&o);
        assert_eq!(c.len(), 6);
        assert_eq!((c[0].mach, c[0].alpha_deg, c[5].mach, c[5].alpha_deg), (0.5, 0.0, 2.0, 8.0));
        assert_eq!(c[1].dir_name(), "m0.500_a04.00");
        assert_eq!(warm_start_source(&c, 0), None);
        assert_eq!(warm_start_source(&c, 2), Some(1));
        assert_eq!(warm_start_source(&c, 3), Some(0));
        let mut names: Vec<String> = c.iter().map(|x| x.dir_name()).collect();
        let sorted = {
            names.sort();
            names.clone()
        };
        assert_eq!(sorted, c.iter().map(|x| x.dir_name()).collect::<Vec<_>>());
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
