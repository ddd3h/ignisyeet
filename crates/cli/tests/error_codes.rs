// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2024-2026 西濱大将 (NISHIHAMA Daisuke)
//! Workspace-wide consistency of the stable error and warning codes `IY-<area><nnn>`: every code that appears in the
//! sources is documented in `doc/source/errors/<area>.rst`, every documented code is used, each code is documented
//! once, the area letter matches the file, and the severity in the documentation matches the number range
//! (001-499 errors, 500-999 warnings).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

const AREAS: [(char, &str); 7] = [('C', "config"), ('G', "geometry"), ('A', "aero"), ('S', "sim"), ('O', "cli"), ('F', "cfd"), ('I', "install")];

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize().unwrap()
}

fn collect(dir: &Path, exts: &[&str], out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            if p.file_name().is_some_and(|n| n != "target" && n != ".git") {
                collect(&p, exts, out);
            }
        } else if p.extension().is_some_and(|x| exts.iter().any(|e| x == *e)) {
            out.push(p);
        }
    }
}

/// Every `IY-Xnnn` code (X an area letter, nnn three digits, not followed by another digit) in `text`.
fn codes_in(text: &str) -> Vec<String> {
    let b = text.as_bytes();
    let mut v = Vec::new();
    let mut i = 0;
    while let Some(k) = text[i..].find("IY-") {
        let s = i + k;
        let t = &b[s + 3..];
        if t.len() >= 4 && AREAS.iter().any(|(c, _)| t[0] == *c as u8) && t[1..4].iter().all(u8::is_ascii_digit) && !t.get(4).is_some_and(u8::is_ascii_digit) {
            v.push(text[s..s + 7].to_string());
        }
        i = s + 3;
    }
    v
}

fn used_in_sources() -> BTreeMap<String, usize> {
    let root = root();
    let mut files = Vec::new();
    collect(&root.join("crates"), &["rs", "py"], &mut files);
    collect(&root.join("scripts"), &["sh"], &mut files);
    files.push(root.join("install.sh"));
    let mut m = BTreeMap::new();
    for f in files {
        let this = f.file_name().is_some_and(|n| n == "error_codes.rs");
        let Ok(text) = std::fs::read_to_string(&f) else { continue };
        if this {
            continue;
        }
        for c in codes_in(&text) {
            *m.entry(c).or_insert(0) += 1;
        }
    }
    m
}

/// code -> (file stem, severity word found in its section)
type Docs = BTreeMap<String, Vec<(String, String)>>;

fn documented() -> (Docs, Vec<String>) {
    let dir = root().join("doc/source/errors");
    let mut m = Docs::new();
    let mut problems = Vec::new();
    for (_, name) in AREAS {
        let text = std::fs::read_to_string(dir.join(format!("{name}.rst"))).unwrap_or_else(|e| panic!("{name}.rst: {e}"));
        let lines: Vec<&str> = text.lines().collect();
        let anchors: Vec<(usize, String)> = lines
            .iter()
            .enumerate()
            .filter_map(|(i, l)| {
                let c = l.strip_prefix(".. _")?.strip_suffix(':')?;
                (codes_in(c).first().map(String::as_str) == Some(c) && c.len() == 7).then(|| (i, c.to_string()))
            })
            .collect();
        for (k, (start, code)) in anchors.iter().enumerate() {
            let end = anchors.get(k + 1).map_or(lines.len(), |a| a.0);
            let sect = lines[*start..end].join("\n");
            let sev = ["エラー", "ワーニング"]
                .into_iter()
                .filter_map(|w| ["重大度: ", "重大度\n   "].iter().find_map(|p| sect.find(&format!("{p}{w}")).map(|_| w)))
                .next();
            match sev {
                Some(w) => m.entry(code.clone()).or_default().push((name.to_string(), w.to_string())),
                None => problems.push(format!("{code} in {name}.rst has no severity line")),
            }
        }
    }
    (m, problems)
}

#[test]
fn every_code_is_documented_once_in_its_area_with_the_right_severity() {
    let used = used_in_sources();
    let (docs, mut problems) = documented();
    for c in used.keys() {
        if !docs.contains_key(c) {
            problems.push(format!("{c} is used in the sources but not documented"));
        }
    }
    for (c, entries) in &docs {
        if !used.contains_key(c) {
            problems.push(format!("{c} is documented but not used in the sources"));
        }
        if entries.len() != 1 {
            problems.push(format!("{c} is documented {} times", entries.len()));
        }
        let area = c.as_bytes()[3] as char;
        let want = AREAS.iter().find(|(a, _)| *a == area).unwrap().1;
        let n: u32 = c[4..].parse().unwrap();
        for (file, sev) in entries {
            if file != want {
                problems.push(format!("{c} is documented in {file}.rst, expected {want}.rst"));
            }
            let want_sev = if n < 500 { "エラー" } else { "ワーニング" };
            if sev != want_sev {
                problems.push(format!("{c} is {sev} in the documentation but its number says {want_sev}"));
            }
        }
    }
    assert!(problems.is_empty(), "error-code inconsistencies:\n{}", problems.join("\n"));
    assert!(docs.len() > 400, "suspiciously few documented codes: {}", docs.len());
}
