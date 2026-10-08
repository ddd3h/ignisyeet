// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025-2026 西濱大将 (NISHIHAMA Daisuke)
//! Every configuration key has exactly one reference entry. The keys are read from the field declarations of the
//! configuration structs; the reference page labels are `cfg-<table>-<key>` (the dotted table path joined with `-`),
//! and each table has a label `cfg-<table>`. A label without a key (and a key without a label) fails the test.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

/// (source file, struct, table path). The top-level tables of the configuration file are listed here; a field that
/// is itself a table of this list is skipped when the fields of its parent are read.
const TABLES: [(&str, &str, &str); 17] = [
    ("crates/cli/src/config.rs", "OutputCfg", "output"),
    ("crates/cli/src/config.rs", "ResourcesCfg", "resources"),
    ("crates/cli/src/config.rs", "RocketCfg", "rocket"),
    ("crates/cli/src/config.rs", "FinOverride", "rocket.fin_override"),
    ("crates/cli/src/config.rs", "MotorCfg", "motor"),
    ("crates/sim/src/flight.rs", "Launch", "launch"),
    ("crates/sim/src/flight.rs", "Recovery", "recovery"),
    ("crates/cli/src/config.rs", "AeroCfg", "aero"),
    ("crates/panel/src/lib.rs", "PanelOptions", "aero.panel"),
    ("crates/cfd/src/config.rs", "CfdOptions", "aero.cfd"),
    ("crates/sim/src/env.rs", "EarthConfig", "earth"),
    ("crates/sim/src/env.rs", "AtmosphereConfig", "atmosphere"),
    ("crates/sim/src/env.rs", "WindConfig", "wind"),
    ("crates/cli/src/config.rs", "SimCfg", "sim"),
    ("crates/sim/src/dispersion.rs", "DispersionConfig", "dispersion"),
    ("crates/sim/src/dispersion.rs", "MonteCarloConfig", "dispersion.monte_carlo"),
    ("crates/cli/src/config.rs", "Config", ""),
];

/// Fields that hold a table of `TABLES` (not a key of their own).
const NESTED: [(&str, &str); 5] =
    [("RocketCfg", "fin_override"), ("AeroCfg", "panel"), ("AeroCfg", "cfd"), ("DispersionConfig", "monte_carlo"), ("Config", "*")];

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize().unwrap()
}

/// Keys of one struct: `pub name:` fields that are not `#[serde(skip)]`, honouring `#[serde(rename = "...")]`.
fn fields(text: &str, name: &str) -> Vec<String> {
    let head = format!("pub struct {name} {{");
    let start = text.find(&head).unwrap_or_else(|| panic!("struct {name} not found")) + head.len();
    let body = &text[start..];
    let end = body.find("\n}").expect("struct end");
    let mut out = Vec::new();
    let mut attrs = String::new();
    for line in body[..end].lines() {
        let t = line.trim();
        if t.starts_with("#[") {
            attrs.push_str(t);
        } else if t.starts_with("///") || t.is_empty() {
        } else {
            if let Some(rest) = t.strip_prefix("pub ") {
                if let Some((f, _)) = rest.split_once(':') {
                    let skipped = attrs.contains("serde(skip)") || attrs.contains("skip,") && !attrs.contains("skip_serializing_if");
                    let mut key = f.trim().to_string();
                    if let Some(p) = attrs.find("rename = \"") {
                        key = attrs[p + 10..].split('"').next().unwrap().to_string();
                    }
                    if !skipped && !NESTED.iter().any(|(s, n)| *s == name && (*n == key || *n == "*")) {
                        out.push(key);
                    }
                }
            }
            attrs.clear();
        }
    }
    out
}

/// Table path -> keys.
fn config_keys() -> BTreeMap<String, Vec<String>> {
    let root = root();
    let mut m = BTreeMap::new();
    for (file, st, table) in TABLES {
        if st == "Config" {
            continue;
        }
        let text = std::fs::read_to_string(root.join(file)).unwrap();
        m.insert(table.to_string(), fields(&text, st));
    }
    m
}

/// Every `.. _cfg-...:` label in the reference pages with the file it is in.
fn labels() -> BTreeMap<String, Vec<String>> {
    let dir = root().join("doc/source/reference/config");
    let mut m: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for e in std::fs::read_dir(dir).unwrap().flatten() {
        let p = e.path();
        if p.extension().is_none_or(|x| x != "rst") {
            continue;
        }
        let name = p.file_name().unwrap().to_string_lossy().to_string();
        for line in std::fs::read_to_string(&p).unwrap().lines() {
            if let Some(l) = line.trim().strip_prefix(".. _cfg-").and_then(|r| r.strip_suffix(':')) {
                m.entry(format!("cfg-{l}")).or_default().push(name.clone());
            }
        }
    }
    m
}

#[test]
fn every_config_key_has_one_reference_label_and_no_label_is_stale() {
    let keys = config_keys();
    let labels = labels();
    let mut expected = BTreeSet::new();
    for (table, ks) in &keys {
        let tl = table.replace('.', "-");
        expected.insert(format!("cfg-{tl}"));
        for k in ks {
            expected.insert(format!("cfg-{tl}-{k}"));
        }
    }
    let missing: Vec<_> = expected.iter().filter(|l| !labels.contains_key(*l)).collect();
    let stale: Vec<_> = labels.keys().filter(|l| !expected.contains(*l)).collect();
    let dup: Vec<_> = labels.iter().filter(|(_, v)| v.len() > 1).map(|(k, v)| format!("{k} in {v:?}")).collect();
    assert!(missing.is_empty(), "configuration keys without a reference label: {missing:#?}");
    assert!(stale.is_empty(), "reference labels without a configuration key: {stale:#?}");
    assert!(dup.is_empty(), "labels defined more than once: {dup:#?}");
}

#[test]
fn key_lists_are_not_empty() {
    // Guards the field reader itself: a refactor of the structs that makes it read nothing must not pass silently.
    let keys = config_keys();
    for (t, n) in [("rocket", 15), ("aero.cfd", 35), ("dispersion.monte_carlo", 20), ("wind", 8), ("sim", 8), ("launch", 6)] {
        assert!(keys[t].len() >= n, "{t}: only {} keys read", keys[t].len());
    }
}
