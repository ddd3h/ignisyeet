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
}

impl ToolReport {
    /// SU2, the MPI launcher (only when `ranks_per_case > 1`) and gmsh are all usable.
    pub fn ready(&self, opt: &CfdOptions) -> bool {
        self.missing(opt).is_empty()
    }

    pub fn missing(&self, opt: &CfdOptions) -> Vec<&ToolInfo> {
        let mut v = vec![&self.su2];
        if opt.ranks_per_case > 1 {
            v.push(&self.mpi);
        }
        v.push(&self.gmsh);
        v.into_iter().filter(|t| !t.ok()).collect()
    }
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

fn is_exec(p: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    p.metadata().map(|m| m.is_file() && m.permissions().mode() & 0o111 != 0).unwrap_or(false)
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

const HINT: &str = "see doc / cfd/install.sh";

/// Locates the tools and probes their versions.
pub fn check_tools(opt: &CfdOptions, roots: &[PathBuf]) -> ToolReport {
    let prefix = resolve_prefix(opt);
    let pre = prefix.as_deref();
    // `SU2_CFD --help` prints the banner `SU2 v8.x.y "Name", ...` first.
    let su2 = probe("SU2_CFD", &opt.su2, pre, &["--help"], HINT);
    let mpi = probe("mpirun", &opt.mpi, pre, &["--version"], HINT);
    let mut gmsh = ToolInfo { name: "gmsh (python)", path: None, version: None, problem: Some(format!("no python with the gmsh module found ({HINT})")) };
    for py in gmsh_python(pre, roots) {
        if let Ok(out) = command(&py, pre).args(["-c", "import gmsh; print(gmsh.__version__)"]).output() {
            if out.status.success() {
                if let Some(v) = first_line(&out) {
                    gmsh = ToolInfo { name: "gmsh (python)", path: Some(py), version: Some(format!("gmsh {v}")), problem: None };
                    break;
                }
            }
        }
    }
    ToolReport { prefix, su2, mpi, gmsh }
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
    fn mpi_optional_for_single_rank() {
        let o = CfdOptions { su2: "sh".into(), mpi: "definitely_not_mpirun".into(), ranks_per_case: 1, ..Default::default() };
        let r = check_tools(&o, &[]);
        assert!(r.missing(&o).iter().all(|t| t.name != "mpirun"));
    }

    #[test]
    fn prefix_and_tilde_expansion() {
        let o = CfdOptions { prefix: "~/x".into(), ..Default::default() };
        let p = resolve_prefix(&o).unwrap();
        assert!(p.ends_with("x") && !p.to_string_lossy().starts_with('~'));
        assert_eq!(find_exe("sh", None).is_some(), true);
    }
}
