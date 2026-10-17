// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2024-2026 西濱大将 (NISHIHAMA Daisuke)
//! STL reader (ASCII and binary) and binary writer.
//!
//! Format detection: a file whose size is exactly `84 + 50 n` bytes (`n` read from bytes 80..84)
//! is binary, unless it is a complete ASCII STL (valid UTF-8 starting with `solid` and containing
//! `facet` and `endsolid`), because many binary exporters also start the 80-byte header with
//! `solid`. Any other file is ASCII when it is UTF-8 text starting with `solid`, and otherwise a
//! truncated binary file (an error). STL carries no units; the caller applies `rocket.stl_scale`.

use crate::math::Vec3;
use anyhow::{bail, Context, Result};
use std::path::Path;

pub type Triangle = [Vec3; 3];

pub fn read_stl(path: &Path) -> Result<Vec<Triangle>> {
    let bytes = std::fs::read(path).with_context(|| format!("[IY-G001] cannot read STL {}", path.display()))?;
    parse_stl(&bytes).with_context(|| format!("cannot parse STL {}", path.display()))
}

pub fn parse_stl(bytes: &[u8]) -> Result<Vec<Triangle>> {
    let is_binary_size = bytes.len() >= 84 && {
        let n = u32::from_le_bytes(bytes[80..84].try_into().unwrap()) as usize;
        bytes.len() == 84 + 50 * n
    };
    // Binary exporters often start the 80-byte header with "solid" too: a file of the exact binary
    // size is binary unless it is a complete ASCII STL (valid UTF-8 with "facet" and "endsolid").
    let ascii_text = if bytes.starts_with(b"solid") { std::str::from_utf8(bytes).ok() } else { None };
    let complete_ascii = ascii_text.is_some_and(|t| t.contains("facet") && t.contains("endsolid"));
    let tris = if is_binary_size && !complete_ascii {
        parse_binary(bytes)?
    } else if ascii_text.is_some() {
        parse_ascii(bytes)?
    } else if bytes.len() >= 84 {
        let n = u32::from_le_bytes(bytes[80..84].try_into().unwrap()) as usize;
        bail!(
            "[IY-G002] binary STL declares {n} triangles (needs {} bytes) but the file has {} bytes: the file is truncated or corrupt",
            84usize.saturating_add(50usize.saturating_mul(n)),
            bytes.len()
        )
    } else {
        bail!("[IY-G003] unrecognised STL format")
    };
    if tris.is_empty() {
        bail!("[IY-G004] STL contains no triangles");
    }
    check_finite(&tris)?;
    Ok(tris)
}

fn check_finite(tris: &[Triangle]) -> Result<()> {
    for (i, t) in tris.iter().enumerate() {
        if t.iter().any(|v| !(v.x.is_finite() && v.y.is_finite() && v.z.is_finite())) {
            bail!("[IY-G005] triangle {i} has a non-finite (NaN or infinite) vertex coordinate");
        }
    }
    Ok(())
}

fn parse_binary(bytes: &[u8]) -> Result<Vec<Triangle>> {
    let n = u32::from_le_bytes(bytes[80..84].try_into().unwrap()) as usize;
    let f = |o: usize| f32::from_le_bytes(bytes[o..o + 4].try_into().unwrap()) as f64;
    let mut tris = Vec::with_capacity(n);
    for i in 0..n {
        let base = 84 + 50 * i + 12; // skip normal
        let v = |k: usize| Vec3::new(f(base + 12 * k), f(base + 12 * k + 4), f(base + 12 * k + 8));
        tris.push([v(0), v(1), v(2)]);
    }
    Ok(tris)
}

fn parse_ascii(bytes: &[u8]) -> Result<Vec<Triangle>> {
    let text = std::str::from_utf8(bytes).context("[IY-G007] ASCII STL is not valid UTF-8")?;
    let mut tris = Vec::new();
    let mut cur: Vec<Vec3> = Vec::with_capacity(3);
    for line in text.lines() {
        let mut it = line.split_whitespace();
        if it.next() == Some("vertex") {
            let c: Vec<f64> =
                it.take(3).map(|s| s.parse::<f64>()).collect::<Result<_, _>>().with_context(|| format!("[IY-G008] invalid number in vertex line: {line}"))?;
            if c.len() != 3 {
                bail!("[IY-G006] malformed vertex line: {line}");
            }
            cur.push(Vec3::new(c[0], c[1], c[2]));
            if cur.len() == 3 {
                tris.push([cur[0], cur[1], cur[2]]);
                cur.clear();
            }
        }
    }
    Ok(tris)
}

pub fn write_stl_binary(path: &Path, tris: &[Triangle]) -> Result<()> {
    let mut out = Vec::with_capacity(84 + 50 * tris.len());
    let mut header = [0u8; 80];
    let tag = b"IgnisYeet binary STL";
    header[..tag.len()].copy_from_slice(tag);
    out.extend_from_slice(&header);
    out.extend_from_slice(&(tris.len() as u32).to_le_bytes());
    for t in tris {
        let n = (t[1] - t[0]).cross(t[2] - t[0]).normalized();
        for v in std::iter::once(n).chain(t.iter().copied()) {
            for c in [v.x, v.y, v.z] {
                out.extend_from_slice(&(c as f32).to_le_bytes());
            }
        }
        out.extend_from_slice(&[0, 0]);
    }
    std::fs::write(path, out).with_context(|| format!("[IY-G009] cannot write STL {}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_tmp::TempDir;

    #[test]
    fn ascii_and_binary_agree() {
        let ascii = b"solid t\nfacet normal 0 0 1\nouter loop\nvertex 0 0 0\nvertex 1 0 0\nvertex 0 1 0\nendloop\nendfacet\nendsolid t\n";
        let a = parse_stl(ascii).unwrap();
        let td = TempDir::new("stl_test");
        let dir = td.join("ignisyeet_stl_test.stl");
        write_stl_binary(&dir, &a).unwrap();
        let b = read_stl(&dir).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn non_finite_vertex_is_rejected_with_triangle_index() {
        let ascii = b"solid t\nfacet normal 0 0 1\nouter loop\nvertex 0 0 0\nvertex 1 0 0\nvertex 0 1 0\nendloop\nendfacet\nfacet normal 0 0 1\nouter loop\nvertex 0 0 0\nvertex NaN 0 0\nvertex 0 1 0\nendloop\nendfacet\nendsolid t\n";
        let e = parse_stl(ascii).unwrap_err().to_string();
        assert!(e.contains("triangle 1"), "{e}");
    }

    #[test]
    fn binary_stl_with_a_solid_header_is_read_as_binary() {
        // Exporters write "solid <name>" (even "facet") into the 80-byte header of binary files.
        // Coordinates 0 and 2 (0x40000000) keep every byte valid ASCII.
        let tri = [Vec3::new(0.0, 0.0, 0.0), Vec3::new(2.0, 0.0, 0.0), Vec3::new(0.0, 2.0, 0.0)];
        let mut b = vec![b' '; 80];
        let tag = b"solid part exported with facet normals";
        b[..tag.len()].copy_from_slice(tag);
        b.extend_from_slice(&1u32.to_le_bytes());
        for v in std::iter::once(Vec3::new(0.0, 0.0, 0.0)).chain(tri) {
            for c in [v.x, v.y, v.z] {
                b.extend_from_slice(&(c as f32).to_le_bytes());
            }
        }
        b.extend_from_slice(&[0, 0]);
        assert_eq!(parse_stl(&b).unwrap(), vec![tri]);
        // An ASCII file is still ASCII, also without a trailing newline.
        let ascii = b"solid t\nfacet normal 0 0 1\nouter loop\nvertex 0 0 0\nvertex 2 0 0\nvertex 0 2 0\nendloop\nendfacet\nendsolid t";
        assert_eq!(parse_stl(ascii).unwrap(), vec![tri]);
    }

    #[test]
    fn truncated_binary_stl_is_reported_as_truncated() {
        let mut b = vec![0u8; 80];
        b.extend_from_slice(&3u32.to_le_bytes());
        b.extend(vec![0u8; 50 * 2 + 7]);
        let e = parse_stl(&b).unwrap_err().to_string();
        assert!(
            e.starts_with("[IY-G002] ")
                && e.contains("declares 3 triangles (needs 234 bytes) but the file has 191 bytes")
                && e.contains("truncated or corrupt"),
            "{e}"
        );
    }

    #[test]
    fn every_stl_error_carries_one_code() {
        let once = |e: anyhow::Error, code: &str| {
            let t = format!("{e:#}");
            assert!(t.contains(code) && t.matches("IY-").count() == 1, "{code}: {t}");
        };
        once(read_stl(Path::new("/nonexistent/ignisyeet_no_such.stl")).unwrap_err(), "[IY-G001]");
        once(parse_stl(b"").unwrap_err(), "[IY-G003]");
        once(parse_stl(b"solid x\nendsolid x\n").unwrap_err(), "[IY-G004]");
        once(parse_stl(b"solid x\nvertex 1 2\nendsolid x\n").unwrap_err(), "[IY-G006]");
        once(parse_stl(b"solid x\nvertex 1 2 abc\nendsolid x\n").unwrap_err(), "[IY-G008]");
        let td = TempDir::new("stl_write_err");
        once(write_stl_binary(&td.join("no_such_dir").join("a.stl"), &[]).unwrap_err(), "[IY-G009]");
    }

    #[test]
    fn read_errors_name_the_file() {
        let td = TempDir::new("stl_garbage");
        let p = td.join("ignisyeet_stl_garbage_test.stl");
        std::fs::write(&p, b"garbage").unwrap();
        let e = format!("{:#}", read_stl(&p).unwrap_err());
        assert!(e.contains("ignisyeet_stl_garbage_test.stl") && e.contains("[IY-G003] unrecognised") && e.matches("IY-").count() == 1, "{e}");
    }
}
