// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025-2026 西濱大将 (NISHIHAMA Daisuke)
//! Lookup and version probing of the external tools (SU2, MPI, gmsh python).

use crate::config::CfdOptions;
use std::path::{Path, PathBuf};
use std::process::Command;

/// One located (or missing) tool.
#[derive(Debug, Clone)]
pub struct ToolInfo {
    pub name: &'static str,
    pub path: Option<PathBuf>,
    /// First informative output line (version), when the tool could be run.
    pub version: Option<String>,
    /// Why the tool is unusable (missing, or failed to run).
    pub problem: Option<String>,
}

impl ToolInfo {
    pub fn ok(&self) -> bool {
        self.problem.is_none()
    }
}

#[derive(Debug, Clone)]
pub struct ToolReport {
    pub prefix: Option<PathBuf>,
    pub su2: ToolInfo,
    pub mpi: ToolInfo,
    pub gmsh: ToolInfo,
    /// Python module manifold3d (union of the STL shells); required only for `surface = "stl"`.
    pub manifold: ToolInfo,
}

impl ToolReport {
    /// SU2, the MPI launcher (only when `ranks_per_case > 1`) and gmsh are all usable.
    pub fn ready(&self, opt: &CfdOptions) -> bool {
        self.missing(opt).is_empty()
    }

    /// Every tool required by `opt` is present and usable (the same as [`ToolReport::ready`]; use it for the exit
    /// status of `cfd-check`).
    pub fn all_required_present(&self, opt: &CfdOptions) -> bool {
        self.ready(opt)
    }

    /// Non-fatal findings: an SU2 version other than the documented 8.3.x (the validation and the case set-up
    /// are pinned to SU2 8.3.0), and an SU2 compiled by the installer for another CPU than the current one.
    pub fn warnings(&self) -> Vec<String> {
        let mut w = Vec::new();
        if let (true, Some(v)) = (self.su2.ok(), self.su2.version.as_deref()) {
            match su2_version(v) {
                Some((8, 3, _)) => {}
                Some((a, b, c)) => {
                    w.push(format!("[IY-F501] SU2 {a}.{b}.{c} found; the documentation and validation are pinned to SU2 8.3.0, results may differ"))
                }
                None => w.push(format!("[IY-F502] cannot read the SU2 version from \"{v}\"; the documentation and validation are pinned to SU2 8.3.0")),
            }
        }
        if let Some(p) = self.su2.path.as_deref().filter(|_| self.su2.ok()) {
            let info = p.parent().and_then(Path::parent).map(|d| d.join("build-info.txt"));
            if let Some(text) = info.and_then(|f| std::fs::read_to_string(f).ok()) {
                w.extend(cpu_mismatch(&text, cpu_model().as_deref(), p));
            }
        }
        w
    }

    pub fn missing(&self, opt: &CfdOptions) -> Vec<&ToolInfo> {
        let mut v = vec![&self.su2];
        if opt.ranks_per_case > 1 {
            v.push(&self.mpi);
        }
        v.push(&self.gmsh);
        if opt.surface == crate::config::SurfaceKind::Stl {
            v.push(&self.manifold);
        }
        v.into_iter().filter(|t| !t.ok()).collect()
    }
}

/// `(major, minor, patch)` from a banner such as `SU2 v8.3.0 "Harrier"`, or the first `v<digits>.<digits>` token.
pub fn su2_version(banner: &str) -> Option<(u32, u32, u32)> {
    let tok = banner.split_whitespace().find_map(|t| {
        let t = t.trim_start_matches('v').trim_end_matches(|c: char| !c.is_ascii_digit());
        (t.split('.').count() >= 2 && t.split('.').all(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_digit()))).then_some(t)
    })?;
    let mut it = tok.split('.').map(|p| p.parse::<u32>().ok());
    Some((it.next()??, it.next()??, it.next().flatten().unwrap_or(0)))
}

/// Expands a leading `~`.
fn expand(p: &str) -> PathBuf {
    if let Some(rest) = p.strip_prefix('~') {
        if let Ok(home) = std::env::var("HOME") {
            return PathBuf::from(home).join(rest.trim_start_matches('/'));
        }
    }
    PathBuf::from(p)
}

/// Tool prefix from the options, else `IGNISYEET_CFD_PREFIX`, else none (use `PATH`).
pub fn resolve_prefix(opt: &CfdOptions) -> Option<PathBuf> {
    let from_opt = opt.prefix.trim();
    if !from_opt.is_empty() {
        return Some(expand(from_opt));
    }
    std::env::var("IGNISYEET_CFD_PREFIX").ok().filter(|s| !s.trim().is_empty()).map(|s| expand(s.trim()))
}

/// Default of `[aero.cfd] su2`.
pub const DEFAULT_SU2: &str = "SU2_CFD";

/// Directory (below an install prefix) where `install.sh --cfd` puts the SU2 it compiles for this CPU.
pub const NATIVE_SU2_DIR: &str = "share/ignisyeet-su2-native";

/// SU2 executable (name or path), first match of:
/// 1. `[aero.cfd] su2` when it differs from the default `SU2_CFD`;
/// 2. the `IGNISYEET_SU2_CFD` environment variable;
/// 3. the SU2 compiled by the installer ([`find_native_su2`]);
/// 4. `SU2_CFD` (looked up in the prefix and `PATH`).
///
/// The prefix still supplies `mpirun`, the gmsh python and `<prefix>/lib`.
pub fn resolve_su2(opt: &CfdOptions) -> String {
    su2_choice(&opt.su2, std::env::var("IGNISYEET_SU2_CFD").ok().as_deref(), find_native_su2().as_deref())
}

fn su2_choice(opt: &str, env: Option<&str>, native: Option<&Path>) -> String {
    let opt = opt.trim();
    if opt != DEFAULT_SU2 {
        return opt.to_string();
    }
    if let Some(e) = env.map(str::trim).filter(|s| !s.is_empty()) {
        return e.to_string();
    }
    native.map(|p| p.display().to_string()).unwrap_or_else(|| DEFAULT_SU2.to_string())
}

/// `SU2_CFD` of the installer's source build: `<prefix>/share/ignisyeet-su2-native/bin/SU2_CFD` for the install
/// prefix that holds this executable (any ancestor of its path, symlinks resolved or not), `IGNISYEET_PREFIX`, or
/// `~/.local` (the installer's default prefix).
pub fn find_native_su2() -> Option<PathBuf> {
    let mut roots: Vec<PathBuf> = Vec::new();
    if let Ok(exe) = std::env::current_exe() {
        let canon = exe.canonicalize().ok();
        for e in std::iter::once(exe).chain(canon) {
            roots.extend(e.ancestors().skip(1).map(Path::to_path_buf));
        }
    }
    if let Some(p) = std::env::var_os("IGNISYEET_PREFIX").filter(|s| !s.is_empty()) {
        roots.push(PathBuf::from(p));
    }
    if let Some(h) = std::env::var_os("HOME").filter(|s| !s.is_empty()) {
        roots.push(Path::new(&h).join(".local"));
    }
    native_su2_in(&roots)
}

/// First `<root>/share/ignisyeet-su2-native/bin/SU2_CFD` that is executable.
fn native_su2_in(roots: &[PathBuf]) -> Option<PathBuf> {
    roots.iter().map(|r| r.join(NATIVE_SU2_DIR).join("bin/SU2_CFD")).find(|p| is_exec(p))
}

/// Model name of the first CPU in `/proc/cpuinfo` (Linux), else `None`.
fn cpu_model() -> Option<String> {
    let text = std::fs::read_to_string("/proc/cpuinfo").ok()?;
    text.lines().find_map(|l| l.strip_prefix("model name")).and_then(|r| r.split_once(':')).map(|(_, v)| v.trim().to_string())
}

/// Warning when the build record next to a compiled SU2 (`build-info.txt`, line `cpu: <model>`) names another CPU
/// than the current one. `None` when they agree or either is unknown.
fn cpu_mismatch(build_info: &str, current: Option<&str>, su2: &Path) -> Option<String> {
    let built = build_info.lines().find_map(|l| l.strip_prefix("cpu:")).map(str::trim).filter(|s| !s.is_empty())?;
    let cur = current?.trim();
    (!cur.is_empty() && built != cur).then(|| {
        format!(
            "[IY-F503] {} was compiled for the CPU \"{built}\" but this machine has \"{cur}\"; it may stop with an illegal instruction. \
             Run `install.sh --cfd --force` here to rebuild it, or `install.sh --cfd --su2-binary` for the portable conda-forge SU2",
            su2.display()
        )
    })
}

#[cfg(unix)]
fn is_exec(p: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    p.metadata().map(|m| m.is_file() && m.permissions().mode() & 0o111 != 0).unwrap_or(false)
}

#[cfg(not(unix))]
fn is_exec(p: &Path) -> bool {
    p.metadata().map(|m| m.is_file()).unwrap_or(false)
}

/// `which`-like lookup: a name containing `/` is checked directly, otherwise `<prefix>/bin` and `PATH`.
pub fn find_exe(name: &str, prefix: Option<&Path>) -> Option<PathBuf> {
    if name.contains('/') {
        let p = expand(name);
        return is_exec(&p).then_some(p);
    }
    if let Some(pre) = prefix {
        let p = pre.join("bin").join(name);
        if is_exec(&p) {
            return Some(p);
        }
    }
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path).map(|d| d.join(name)).find(|p| is_exec(p))
}

/// Command with `<prefix>/bin` first in `PATH` and `<prefix>/lib` in `LD_LIBRARY_PATH`.
pub fn command(exe: &Path, prefix: Option<&Path>) -> Command {
    let mut c = Command::new(exe);
    if let Some(pre) = prefix {
        let mut paths = vec![pre.join("bin")];
        if let Some(old) = std::env::var_os("PATH") {
            paths.extend(std::env::split_paths(&old));
        }
        if let Ok(p) = std::env::join_paths(paths) {
            c.env("PATH", p);
        }
        let lib = pre.join("lib");
        let ld = match std::env::var("LD_LIBRARY_PATH") {
            Ok(old) if !old.is_empty() => format!("{}:{old}", lib.display()),
            _ => lib.display().to_string(),
        };
        c.env("LD_LIBRARY_PATH", ld);
    }
    c
}

fn first_line(out: &std::process::Output) -> Option<String> {
    let text = format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
    text.lines().map(str::trim).find(|l| !l.is_empty()).map(str::to_string)
}

fn probe(name: &'static str, exe: &str, prefix: Option<&Path>, args: &[&str], hint: &str) -> ToolInfo {
    let Some(path) = find_exe(exe, prefix) else {
        return ToolInfo { name, path: None, version: None, problem: Some(format!("`{exe}` not found ({hint})")) };
    };
    match command(&path, prefix).args(args).output() {
        Ok(out) => match first_line(&out) {
            Some(l) => ToolInfo { name, path: Some(path), version: Some(l), problem: None },
            None => ToolInfo { name, path: Some(path.clone()), version: None, problem: Some(format!("`{}` printed nothing", path.display())) },
        },
        Err(e) => ToolInfo { name, path: Some(path.clone()), version: None, problem: Some(format!("cannot run {}: {e}", path.display())) },
    }
}

/// Python interpreter with gmsh: `<prefix>/bin/python`, then `cfd/.venv/bin/python` below
/// `roots` (or the current directory), then `python3` in `PATH`.
pub fn gmsh_python(prefix: Option<&Path>, roots: &[PathBuf]) -> Vec<PathBuf> {
    let mut v = Vec::new();
    if let Some(p) = prefix {
        v.push(p.join("bin/python"));
    }
    let mut roots: Vec<PathBuf> = roots.to_vec();
    if let Ok(cwd) = std::env::current_dir() {
        roots.push(cwd);
    }
    v.extend(roots.iter().map(|r| r.join("cfd/.venv/bin/python")));
    v.extend(find_exe("python3", None));
    v.into_iter().filter(|p| is_exec(p)).collect()
}

/// Pointer appended to the "not found" problems; also the first line of the not-installed error.
pub const INSTALL_HINT: &str = "install with `install.sh --cfd`, see the CFD chapter of the documentation";
const HINT: &str = INSTALL_HINT;

/// The python with the gmsh module (see [`gmsh_python`]), probed by importing it.
pub fn probe_gmsh(prefix: Option<&Path>, roots: &[PathBuf]) -> ToolInfo {
    let mut gmsh = ToolInfo { name: "gmsh (python)", path: None, version: None, problem: Some(format!("no python with the gmsh module found ({HINT})")) };
    for py in gmsh_python(prefix, roots) {
        if let Ok(out) = command(&py, prefix).args(["-c", "import gmsh; print(gmsh.__version__)"]).output() {
            if out.status.success() {
                if let Some(v) = first_line(&out) {
                    gmsh = ToolInfo { name: "gmsh (python)", path: Some(py), version: Some(format!("gmsh {v}")), problem: None };
                    break;
                }
            }
        }
    }
    gmsh
}

/// Locates the tools and probes their versions.
pub fn check_tools(opt: &CfdOptions, roots: &[PathBuf]) -> ToolReport {
    let prefix = resolve_prefix(opt);
    let pre = prefix.as_deref();
    // `SU2_CFD --help` prints the banner `SU2 v8.x.y "Name", ...` first.
    let su2 = probe("SU2_CFD", &resolve_su2(opt), pre, &["--help"], HINT);
    let mpi = probe("mpirun", &opt.mpi, pre, &["--version"], HINT);
    let gmsh = probe_gmsh(pre, roots);
    // manifold3d is imported by the same python that has gmsh.
    let mut manifold = ToolInfo {
        name: "manifold3d (python)",
        path: None,
        version: None,
        problem: Some("no python with the gmsh module found, so manifold3d cannot be checked".into()),
    };
    if let Some(py) = gmsh.path.clone() {
        let out = command(&py, pre).args(["-c", "import manifold3d; print('manifold3d', getattr(manifold3d, '__version__', 'installed'))"]).output();
        manifold = match out {
            Ok(o) if o.status.success() => ToolInfo { name: "manifold3d (python)", path: Some(py), version: first_line(&o), problem: None },
            _ => ToolInfo {
                name: "manifold3d (python)",
                path: Some(py.clone()),
                version: None,
                problem: Some(format!(
                    "the python module manifold3d is missing (needed for surface = \"stl\"); install it with: {} -m pip install manifold3d",
                    py.display()
                )),
            },
        };
    }
    ToolReport { prefix, su2, mpi, gmsh, manifold }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_tool_is_reported() {
        let o = CfdOptions { su2: "definitely_not_su2_cfd".into(), mpi: "definitely_not_mpirun".into(), ..Default::default() };
        let r = check_tools(&o, &[]);
        assert!(!r.su2.ok() && !r.mpi.ok());
        assert!(r.su2.problem.as_ref().unwrap().contains("not found"));
        assert!(!r.ready(&o));
        assert!(r.missing(&o).iter().any(|t| t.name == "SU2_CFD"));
    }

    #[test]
    fn manifold_is_required_only_for_stl() {
        let ok = |name| ToolInfo { name, path: None, version: Some("x".into()), problem: None };
        let r = ToolReport {
            prefix: None,
            su2: ok("SU2_CFD"),
            mpi: ok("mpirun"),
            gmsh: ok("gmsh"),
            manifold: ToolInfo { name: "manifold3d (python)", path: None, version: None, problem: Some("missing".into()) },
        };
        let cad = CfdOptions::default();
        let stl = CfdOptions { surface: crate::config::SurfaceKind::Stl, ..Default::default() };
        assert!(r.ready(&cad) && !r.ready(&stl));
        assert_eq!(r.missing(&stl)[0].name, "manifold3d (python)");
    }

    #[test]
    fn su2_version_warning() {
        assert_eq!(su2_version("SU2 v8.3.0 \"Harrier\", Copyright"), Some((8, 3, 0)));
        assert_eq!(su2_version("SU2 v8.1.0 \"Harrier\""), Some((8, 1, 0)));
        assert_eq!(su2_version("no version"), None);
        let ok = |name, v: Option<&str>| ToolInfo { name, path: None, version: v.map(Into::into), problem: None };
        let mk = |v: &str| ToolReport { prefix: None, su2: ok("SU2_CFD", Some(v)), mpi: ok("mpirun", None), gmsh: ok("gmsh", None), manifold: ok("m", None) };
        assert!(mk("SU2 v8.3.0 \"Harrier\"").warnings().is_empty());
        assert!(mk("SU2 v8.3.2 \"Harrier\"").warnings().is_empty());
        let w = mk("SU2 v8.1.0 \"Harrier\"").warnings();
        assert!(w.len() == 1 && w[0].starts_with("[IY-F501] ") && w[0].contains("8.3.0"), "{w:?}");
        let g = mk("garbled").warnings();
        assert!(g.len() == 1 && g[0].starts_with("[IY-F502] "), "{g:?}");
    }

    #[test]
    fn all_required_present_follows_the_options() {
        let o = CfdOptions { su2: "definitely_not_su2_cfd".into(), ..Default::default() };
        assert!(!check_tools(&o, &[]).all_required_present(&o));
    }

    #[test]
    fn mpi_optional_for_single_rank() {
        let o = CfdOptions { su2: "sh".into(), mpi: "definitely_not_mpirun".into(), ranks_per_case: 1, ..Default::default() };
        let r = check_tools(&o, &[]);
        assert!(r.missing(&o).iter().all(|t| t.name != "mpirun"));
    }

    #[test]
    fn su2_from_option_then_environment_then_installed_build() {
        let native = PathBuf::from("/home/u/.local/share/ignisyeet-su2-native/bin/SU2_CFD");
        let n = Some(native.as_path());
        assert_eq!(su2_choice("SU2_CFD", None, None), "SU2_CFD");
        assert_eq!(su2_choice("SU2_CFD", Some("  "), None), "SU2_CFD");
        assert_eq!(su2_choice("SU2_CFD", Some("/opt/su2-native/bin/SU2_CFD"), None), "/opt/su2-native/bin/SU2_CFD");
        // The installer's build is used when nothing else is set ...
        assert_eq!(su2_choice("SU2_CFD", None, n), native.display().to_string());
        assert_eq!(su2_choice("SU2_CFD", Some(" "), n), native.display().to_string());
        // ... the environment variable wins over it, and an explicit [aero.cfd] su2 wins over both.
        assert_eq!(su2_choice("SU2_CFD", Some("/opt/su2/bin/SU2_CFD"), n), "/opt/su2/bin/SU2_CFD");
        assert_eq!(su2_choice("~/su2/bin/SU2_CFD", Some("/opt/su2/bin/SU2_CFD"), n), "~/su2/bin/SU2_CFD");
        assert_eq!(su2_choice("SU2_CFD ", None, n), native.display().to_string());
        // A path is found directly, also with a prefix set (the prefix supplies mpirun and lib only).
        let sh = find_exe("sh", None).unwrap();
        let o = CfdOptions { su2: sh.display().to_string(), prefix: "/nonexistent".into(), ..Default::default() };
        assert_eq!(check_tools(&o, &[]).su2.path.as_deref(), Some(sh.as_path()));
    }

    #[cfg(unix)]
    #[test]
    fn installed_build_is_found_below_the_install_prefix() {
        use std::os::unix::fs::PermissionsExt;
        let tmp = std::env::temp_dir().join(format!("ignisyeet-native-su2-{}", std::process::id()));
        let (a, b) = (tmp.join("a"), tmp.join("b"));
        for r in [&a, &b] {
            std::fs::create_dir_all(r.join(NATIVE_SU2_DIR).join("bin")).unwrap();
        }
        let exe = b.join(NATIVE_SU2_DIR).join("bin/SU2_CFD");
        // Present but not executable: not a candidate.
        std::fs::write(&exe, "#!/bin/sh\n").unwrap();
        assert_eq!(native_su2_in(&[a.clone(), b.clone()]), None);
        std::fs::set_permissions(&exe, std::fs::Permissions::from_mode(0o755)).unwrap();
        assert_eq!(native_su2_in(&[a.clone(), b.clone()]), Some(exe.clone()));
        // The first root with the build wins.
        let exe_a = a.join(NATIVE_SU2_DIR).join("bin/SU2_CFD");
        std::fs::write(&exe_a, "#!/bin/sh\n").unwrap();
        std::fs::set_permissions(&exe_a, std::fs::Permissions::from_mode(0o755)).unwrap();
        assert_eq!(native_su2_in(&[a.clone(), b]), Some(exe_a));
        assert_eq!(native_su2_in(&[tmp.join("none")]), None);
        std::fs::remove_dir_all(&tmp).unwrap();
    }

    #[test]
    fn cpu_mismatch_warning() {
        let p = Path::new("/x/bin/SU2_CFD");
        let info = "su2: v8.3.0\ncpu: AMD EPYC 7742 64-Core Processor\nbuilt: today\n";
        assert!(cpu_mismatch(info, Some("AMD EPYC 7742 64-Core Processor"), p).is_none());
        let w = cpu_mismatch(info, Some("Intel(R) Xeon(R) Gold 6130"), p).unwrap();
        assert!(w.starts_with("[IY-F503] ") && w.contains("EPYC 7742") && w.contains("Xeon") && w.contains("install.sh --cfd --force"));
        // Unknown on either side: no warning.
        assert!(cpu_mismatch(info, None, p).is_none());
        assert!(cpu_mismatch("su2: v8.3.0\n", Some("Intel"), p).is_none());
        assert!(cpu_mismatch("cpu:\n", Some("Intel"), p).is_none());
    }

    #[test]
    fn prefix_and_tilde_expansion() {
        let o = CfdOptions { prefix: "~/x".into(), ..Default::default() };
        let p = resolve_prefix(&o).unwrap();
        assert!(p.ends_with("x") && !p.to_string_lossy().starts_with('~'));
        assert!(find_exe("sh", None).is_some());
    }
}
