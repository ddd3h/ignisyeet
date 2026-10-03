//! STL reader (ASCII and binary) and binary writer.

use crate::math::Vec3;
use anyhow::{bail, Context, Result};
use std::path::Path;

pub type Triangle = [Vec3; 3];

pub fn read_stl(path: &Path) -> Result<Vec<Triangle>> {
    let bytes = std::fs::read(path).with_context(|| format!("cannot read STL {}", path.display()))?;
    parse_stl(&bytes)
}

pub fn parse_stl(bytes: &[u8]) -> Result<Vec<Triangle>> {
    let is_binary_size = bytes.len() >= 84 && {
        let n = u32::from_le_bytes(bytes[80..84].try_into().unwrap()) as usize;
        bytes.len() == 84 + 50 * n
    };
    let looks_ascii = bytes.starts_with(b"solid")
        && std::str::from_utf8(&bytes[..bytes.len().min(1024)])
            .map(|s| s.contains("facet"))
            .unwrap_or(false);
    let tris = if is_binary_size && !looks_ascii { parse_binary(bytes)? } else if looks_ascii { parse_ascii(bytes)? } else if is_binary_size { parse_binary(bytes)? } else { bail!("unrecognised STL format") };
    if tris.is_empty() {
        bail!("STL contains no triangles");
    }
    Ok(tris)
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
    let text = std::str::from_utf8(bytes).context("ASCII STL is not valid UTF-8")?;
    let mut tris = Vec::new();
    let mut cur: Vec<Vec3> = Vec::with_capacity(3);
    for line in text.lines() {
        let mut it = line.split_whitespace();
        if it.next() == Some("vertex") {
            let c: Vec<f64> = it.take(3).map(|s| s.parse::<f64>()).collect::<Result<_, _>>()?;
            if c.len() != 3 {
                bail!("malformed vertex line: {line}");
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
    std::fs::write(path, out).with_context(|| format!("cannot write STL {}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ascii_and_binary_agree() {
        let ascii = b"solid t\nfacet normal 0 0 1\nouter loop\nvertex 0 0 0\nvertex 1 0 0\nvertex 0 1 0\nendloop\nendfacet\nendsolid t\n";
        let a = parse_stl(ascii).unwrap();
        let dir = std::env::temp_dir().join("ignisyeet_stl_test.stl");
        write_stl_binary(&dir, &a).unwrap();
        let b = read_stl(&dir).unwrap();
        assert_eq!(a, b);
    }
}
