// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025-2026 西濱大将 (NISHIHAMA Daisuke)
//! Memory available to the dense panel solve.
//!
//! The smallest of the limits that apply to this process, each with the name of its source:
//! * Linux: `MemAvailable` in /proc/meminfo (free memory plus reclaimable caches);
//! * a cgroup memory limit (containers, systemd slices): cgroup v2 `memory.max` minus
//!   `memory.current`, or cgroup v1 `memory.limit_in_bytes` minus `memory.usage_in_bytes`;
//! * the address-space limit `RLIMIT_AS` (`ulimit -v`) minus the address space already mapped
//!   (`VmSize` in /proc/self/status, where available);
//! * macOS: the physical memory (`sysctl hw.memsize`), the only figure available there.
//!
//! Where none is available (other systems), there is no check; the dense allocation itself is still
//! fallible (see `solver::reserve_dense`), so a solve that does not fit fails with an error.

/// A memory limit [bytes] and its source.
#[derive(Debug, Clone, PartialEq)]
pub struct Available {
    pub bytes: u64,
    pub source: String,
}

/// `MemAvailable` [bytes] from the text of /proc/meminfo.
pub fn parse_meminfo_available(text: &str) -> Option<u64> {
    kb_field(text, "MemAvailable:")
}

fn kb_field(text: &str, key: &str) -> Option<u64> {
    let line = text.lines().find(|l| l.starts_with(key))?;
    let kb: u64 = line.split_whitespace().nth(1)?.parse().ok()?;
    Some(kb * 1024)
}

/// A cgroup limit file (`memory.max`, `memory.limit_in_bytes`): `None` for "max" (no limit) or
/// for the huge values cgroup v1 reports when unlimited.
pub fn parse_cgroup_limit(text: &str) -> Option<u64> {
    let t = text.trim();
    if t == "max" {
        return None;
    }
    let v: u64 = t.parse().ok()?;
    (v < 1 << 60).then_some(v)
}

/// Memory left below a cgroup limit, from the limit and usage files of one cgroup directory.
fn cgroup_remaining(dir: &str, limit: &str, usage: &str) -> Option<u64> {
    let lim = parse_cgroup_limit(&std::fs::read_to_string(format!("{dir}/{limit}")).ok()?)?;
    let used = std::fs::read_to_string(format!("{dir}/{usage}")).ok().and_then(|t| t.trim().parse::<u64>().ok()).unwrap_or(0);
    Some(lim.saturating_sub(used))
}

/// The cgroup directory of this process: the v2 entry (`0::/path`) of /proc/self/cgroup under
/// /sys/fs/cgroup, falling back to the namespace root.
fn cgroup_v2_dir() -> String {
    let path = std::fs::read_to_string("/proc/self/cgroup")
        .ok()
        .and_then(|t| t.lines().find_map(|l| l.strip_prefix("0::").map(|p| p.trim().to_string())))
        .unwrap_or_default();
    let dir = format!("/sys/fs/cgroup{}", if path == "/" { "" } else { path.as_str() });
    if std::path::Path::new(&format!("{dir}/memory.max")).exists() {
        dir
    } else {
        "/sys/fs/cgroup".into()
    }
}

#[cfg(unix)]
fn address_space_remaining() -> Option<u64> {
    let mut r = libc::rlimit { rlim_cur: 0, rlim_max: 0 };
    // SAFETY: getrlimit writes into the struct we pass.
    if unsafe { libc::getrlimit(libc::RLIMIT_AS, &mut r) } != 0 || r.rlim_cur == libc::RLIM_INFINITY {
        return None;
    }
    let mapped = std::fs::read_to_string("/proc/self/status").ok().and_then(|t| kb_field(&t, "VmSize:")).unwrap_or(0);
    #[allow(clippy::unnecessary_cast)] // rlim_t is not u64 on every unix
    let cur = r.rlim_cur as u64;
    Some(cur.saturating_sub(mapped))
}

#[cfg(not(unix))]
fn address_space_remaining() -> Option<u64> {
    None
}

#[cfg(target_os = "macos")]
fn macos_memsize() -> Option<u64> {
    let out = std::process::Command::new("sysctl").args(["-n", "hw.memsize"]).output().ok()?;
    String::from_utf8(out.stdout).ok()?.trim().parse().ok()
}

#[cfg(not(target_os = "macos"))]
fn macos_memsize() -> Option<u64> {
    None
}

/// All limits that apply to this process.
pub fn limits() -> Vec<Available> {
    let mut v = Vec::new();
    let mut add = |bytes: Option<u64>, source: &str| {
        if let Some(bytes) = bytes {
            v.push(Available { bytes, source: source.into() });
        }
    };
    add(std::fs::read_to_string("/proc/meminfo").ok().and_then(|t| parse_meminfo_available(&t)), "available memory, MemAvailable in /proc/meminfo");
    let d2 = cgroup_v2_dir();
    add(cgroup_remaining(&d2, "memory.max", "memory.current"), "cgroup memory.max minus memory.current");
    add(
        cgroup_remaining("/sys/fs/cgroup/memory", "memory.limit_in_bytes", "memory.usage_in_bytes"),
        "cgroup v1 memory.limit_in_bytes minus memory.usage_in_bytes",
    );
    add(address_space_remaining(), "address-space limit RLIMIT_AS (ulimit -v) minus the mapped address space");
    add(macos_memsize(), "physical memory, sysctl hw.memsize");
    v
}

/// The smallest limit, if any applies.
pub fn available() -> Option<Available> {
    limits().into_iter().min_by_key(|a| a.bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_meminfo_and_cgroup_files() {
        let mi = "MemTotal:       32000000 kB\nMemFree:         1000000 kB\nMemAvailable:   20000000 kB\n";
        assert_eq!(parse_meminfo_available(mi), Some(20_000_000 * 1024));
        assert_eq!(parse_meminfo_available("MemTotal: 5 kB\n"), None);
        assert_eq!(parse_cgroup_limit("max\n"), None);
        assert_eq!(parse_cgroup_limit("4294967296\n"), Some(4_294_967_296));
        assert_eq!(parse_cgroup_limit("9223372036854771712\n"), None); // cgroup v1 "unlimited"
        assert_eq!(parse_cgroup_limit("garbage"), None);
    }

    #[test]
    fn limits_are_positive_and_named() {
        for a in limits() {
            assert!(!a.source.is_empty());
        }
        if std::path::Path::new("/proc/meminfo").exists() {
            let a = available().expect("MemAvailable on Linux");
            assert!(a.bytes > 0);
        }
    }
}
