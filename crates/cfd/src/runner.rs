// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025-2026 西濱大将 (NISHIHAMA Daisuke)
//! Case runner: queue, SU2 processes, live progress, resume, failure handling.
//!
//! # Scheduling
//!
//! Cases are ordered by Mach, then alpha ([`crate::case::case_list`]). A case is *ready* when the
//! case it warm-starts from ([`crate::case::warm_start_source`]) has finished (successfully or
//! not; after a failure the dependant starts from the freestream). Up to `parallel_cases` ready
//! cases run concurrently, lowest index first. So the alpha sweeps of all Mach numbers run in
//! parallel after the lowest-alpha column has been solved Mach by Mach.
//!
//! # One case
//!
//! Directory `<workdir>/mXXX_aYY/` with `case.cfg`, `su2.log`, `history.csv`, `surface_flow.csv`
//! and, when finished, `done.json` (input hash, coefficients, convergence). A valid `done.json`
//! makes the case a no-op on the next run (resume). The SU2 command is
//! `mpirun -n <ranks> --bind-to none SU2_CFD case.cfg`, or plain `SU2_CFD case.cfg` for one rank.
//! `history.csv` is polled once a second for the live progress.
//!
//! # Acceptance
//!
//! * Residual target reached: `converged`, coefficients of the last row.
//! * Early stop: after 250 iterations, a drop of 3 orders and coefficients flat (see `History::coefficients_stable`) over the last 120
//!   iterations stop the case gracefully and accept it (`converged = false`).
//! * Iteration limit / timeout without reaching the target: accepted (`converged = false`) when the
//!   density residual dropped at least 3 orders (RANS: 1 order) from its peak and `CFx`, `CFz`, `CMy` varied by less
//!   than 1 % over the last window; the coefficients are averaged over that window.
//! * Otherwise (SU2 error, divergence, no history, not converged and not stable) the case is
//!   *failed*: no `done.json`, the table builder bridges it by interpolation.
//!
//! Children run in their own process group and are killed on Ctrl-C / SIGTERM; finished cases
//! keep their `done.json`.

use crate::case::{case_hash, case_list, read_done, warm_start_source, write_done, CaseResult, CaseSpec};
use crate::config::{AlphaMode, CfdOptions};
use crate::table::SolvedPoint;
use crate::forces::{BodyCoeffs, Su2Coeffs};
use crate::history::{summarize, History};
use crate::su2cfg::{su2_config, CaseFiles, RefDims};
use crate::tools::{self, ToolReport};
use crate::CfdStage;
use aero::atmosphere::Atmosphere;
use anyhow::{anyhow, bail, Context, Result};
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::time::{Duration, Instant};

/// Early stop: minimum iterations and averaging window of the coefficient-stability test.
const EARLY_MIN_ITER: usize = 250;
const EARLY_WINDOW: usize = 120;

/// Everything the runner needs.
pub struct RunSetup<'a> {
    pub opt: &'a CfdOptions,
    pub dims: RefDims,
    pub atm: Atmosphere,
    pub mesh_hash: String,
    pub mesh_file: PathBuf,
    pub work: PathBuf,
    /// MPI ranks per case and concurrent cases (resolved by [`crate::plan_resources`]).
    pub ranks: usize,
    pub parallel: usize,
    pub tools: &'a ToolReport,
}

/// Result of one case of the matrix.
#[derive(Debug, Clone)]
pub struct CaseOutcome {
    pub spec: CaseSpec,
    /// `None`: the case failed (see `error`).
    pub result: Option<CaseResult>,
    pub error: Option<String>,
    /// Taken from an earlier run (`done.json`).
    pub cached: bool,
    /// Started from a neighbour's restart file.
    pub warm: bool,
}

static CANCEL: AtomicBool = AtomicBool::new(false);

extern "C" fn on_signal(_: libc::c_int) {
    CANCEL.store(true, Ordering::SeqCst);
}

/// Installs the Ctrl-C / SIGTERM handlers for the duration of a run.
struct SignalGuard;

impl SignalGuard {
    fn new() -> Self {
        CANCEL.store(false, Ordering::SeqCst);
        unsafe {
            libc::signal(libc::SIGINT, on_signal as *const () as usize);
            libc::signal(libc::SIGTERM, on_signal as *const () as usize);
        }
        SignalGuard
    }
}

impl Drop for SignalGuard {
    fn drop(&mut self) {
        unsafe {
            libc::signal(libc::SIGINT, libc::SIG_DFL);
            libc::signal(libc::SIGTERM, libc::SIG_DFL);
        }
    }
}

/// A child process group that is killed when dropped.
struct Group(Child);

impl Group {
    fn kill(&mut self, sig: libc::c_int) {
        unsafe {
            libc::kill(-(self.0.id() as i32), sig);
        }
    }
}

impl Drop for Group {
    fn drop(&mut self) {
        if matches!(self.0.try_wait(), Ok(None)) {
            self.kill(libc::SIGTERM);
            std::thread::sleep(Duration::from_millis(200));
            self.kill(libc::SIGKILL);
            let _ = self.0.wait();
        }
    }
}

/// SU2 configuration text of a case without the restart line: the input of its hash.
fn base_config(setup: &RunSetup, spec: &CaseSpec) -> String {
    su2_config(setup.opt, spec.mach, spec.alpha_deg, &setup.dims, &setup.atm, &CaseFiles { mesh: setup.mesh_file.display().to_string(), restart_from: None })
}

/// Excerpt of the SU2 log for an error message: the SU2 "Error" block when there is one (MPI noise
/// follows it), else the last `n` lines.
fn tail(path: &Path, n: usize) -> String {
    let t = std::fs::read_to_string(path).unwrap_or_default();
    let lines: Vec<&str> = t.lines().collect();
    if let Some(i) = lines.iter().position(|l| l.starts_with("Error") && !l.contains("UCX")) {
        return lines[i..(i + 6).min(lines.len())].join("\n");
    }
    let v: Vec<&str> = t.lines().rev().take(n).collect();
    v.into_iter().rev().collect::<Vec<_>>().join("\n")
}

enum Event {
    Progress { i: usize, iter: usize, residual: f64 },
    Finished { i: usize, outcome: CaseOutcome },
}

fn command(setup: &RunSetup, dir: &Path) -> Result<std::process::Command> {
    let t = setup.tools;
    let su2 = t.su2.path.clone().context("SU2_CFD not found")?;
    let mut c = if setup.ranks > 1 {
        let mpi = t.mpi.path.clone().context("mpirun not found")?;
        let mut c = tools::command(&mpi, t.prefix.as_deref());
        c.arg("-n").arg(setup.ranks.to_string()).args(["--oversubscribe", "--bind-to", "none"]).arg(&su2).arg("case.cfg");
        c
    } else {
        let mut c = tools::command(&su2, t.prefix.as_deref());
        c.arg("case.cfg");
        c
    };
    let log = std::fs::File::create(dir.join("su2.log"))?;
    // Single-node runs: shared-memory transport only (the TCP/UCX components probe network interfaces
    // and fail or spam errors on machines with virtual / unreachable interfaces).
    c.env("OMPI_MCA_btl", "self,sm").env("OMPI_MCA_osc", "^ucx").env("OMPI_MCA_pml", "ob1");
    c.current_dir(dir).env("OMP_NUM_THREADS", "1").stdin(Stdio::null()).stdout(log.try_clone()?).stderr(log).process_group(0);
    Ok(c)
}

/// Runs one case to completion (or failure). Returns `Err` only when cancelled.
fn run_case(setup: &RunSetup, i: usize, spec: CaseSpec, hash: &str, restart: Option<PathBuf>, tx: &mpsc::Sender<Event>) -> Result<CaseOutcome> {
    let dir = spec.dir(&setup.work);
    let warm = restart.is_some();
    let fail = |msg: String| CaseOutcome { spec, result: None, error: Some(msg), cached: false, warm };
    std::fs::create_dir_all(&dir)?;
    for f in ["history.csv", "done.json", "surface_flow.csv", "restart.dat"] {
        let _ = std::fs::remove_file(dir.join(f));
    }
    // SU2 aborts ("buffer overflow detected") when the restart file path is long, so name the
    // sibling case directory relative to this one.
    let restart_from = restart.map(|p| match p.parent().and_then(|d| d.file_name()) {
        Some(case) if p.parent().and_then(|d| d.parent()) == dir.parent() => format!("../{}/restart.dat", case.to_string_lossy()),
        _ => p.display().to_string(),
    });
    let files = CaseFiles { mesh: setup.mesh_file.display().to_string(), restart_from };
    std::fs::write(dir.join("case.cfg"), su2_config(setup.opt, spec.mach, spec.alpha_deg, &setup.dims, &setup.atm, &files))?;
    let t0 = Instant::now();
    let mut cmd = command(setup, &dir)?;
    let mut child = match cmd.spawn() {
        Ok(c) => Group(c),
        Err(e) => return Ok(fail(format!("cannot start SU2: {e}"))),
    };
    let limit = (setup.opt.timeout_minutes > 0.0).then(|| Duration::from_secs_f64(setup.opt.timeout_minutes * 60.0));
    let mut last_poll = Instant::now() - Duration::from_secs(10);
    // RANS starts from a low residual and stalls near 1e-4..1e-5: it only has to drop one order.
    let min_drop = if setup.opt.model == crate::config::FlowModel::Rans { 1.0 } else { 3.0 };
    let mut timed_out = false;
    let status = loop {
        if CANCEL.load(Ordering::SeqCst) {
            child.kill(libc::SIGTERM);
            std::thread::sleep(Duration::from_millis(300));
            child.kill(libc::SIGKILL);
            let _ = child.0.wait();
            bail!("interrupted");
        }
        if let Some(st) = child.0.try_wait()? {
            break Some(st);
        }
        if limit.is_some_and(|l| t0.elapsed() > l) {
            child.kill(libc::SIGTERM);
            std::thread::sleep(Duration::from_millis(500));
            child.kill(libc::SIGKILL);
            let _ = child.0.wait();
            timed_out = true;
            break None;
        }
        if last_poll.elapsed() >= Duration::from_secs(1) {
            last_poll = Instant::now();
            if let Ok(h) = History::read(&dir.join("history.csv")) {
                if let Some(r) = h.last_residual() {
                    let _ = tx.send(Event::Progress { i, iter: h.last_iteration(), residual: r });
                }
                // Practically converged (typical for RANS / Euler wake limit cycles): residual down 3
                // orders and the coefficients flat over EARLY_WINDOW iterations. Stop SU2 gracefully
                // (it writes its restart file) before the residual drifts and diverges.
                if h.rows.len() >= EARLY_MIN_ITER && h.residual_drop_from_peak().is_some_and(|d| d >= min_drop) && h.coefficients_stable(EARLY_WINDOW) {
                    child.kill(libc::SIGTERM);
                    let t = Instant::now();
                    while t.elapsed() < Duration::from_secs(30) && child.0.try_wait()?.is_none() {
                        std::thread::sleep(Duration::from_millis(100));
                    }
                    break None;
                }
            }
        }
        std::thread::sleep(Duration::from_millis(100));
    };
    let wall = t0.elapsed().as_secs_f64();
    let log = dir.join("su2.log");
    if let Some(st) = status {
        if !st.success() {
            return Ok(fail(format!("SU2 exited with {st}; end of su2.log:\n{}", tail(&log, 8))));
        }
    }
    let h = match History::read(&dir.join("history.csv")) {
        Ok(h) if !h.rows.is_empty() => h,
        _ => return Ok(fail(format!("SU2 produced no history.csv; end of su2.log:\n{}", tail(&log, 8)))),
    };
    let minval = setup.opt.residual_minval();
    let n = h.rows.len();
    let reached = h.last_residual().is_some_and(|r| r <= minval);
    let window = (n / 4).clamp(2, 100);
    let accepted = !reached && h.residual_drop_from_peak().is_some_and(|d| d >= min_drop) && h.coefficients_stable(window);
    if !reached && !accepted {
        let why = if timed_out { "timeout" } else { "iteration limit" };
        return Ok(fail(format!(
            "not converged ({why}): residual {:.2} (target {minval}), drop {:.1} orders, coefficients {}",
            h.last_residual().unwrap_or(f64::NAN),
            h.residual_drop_from_peak().unwrap_or(f64::NAN),
            if h.coefficients_stable(window) { "stable" } else { "still changing" }
        )));
    }
    let s = summarize(&h, if reached { 1 } else { window }).expect("non-empty history");
    let coeffs = match Su2Coeffs::from_summary(&s, spec.alpha_deg) {
        Ok(c) => BodyCoeffs::from_su2(&c, setup.dims.length),
        Err(e) => return Ok(fail(e.to_string())),
    };
    let first = summarize(&h, 1).and_then(|_| {
        let c = h.col(&["rms[Rho]", "Res_Flow[0]", "rms[P]"])?;
        Some(h.rows[0][c])
    });
    let result = CaseResult {
        input_hash: hash.to_string(),
        mach: spec.mach,
        alpha_deg: spec.alpha_deg,
        coeffs,
        iterations: s.iterations,
        converged: reached,
        residual_first: first,
        residual_last: h.last_residual(),
        wall_seconds: wall,
    };
    write_done(&dir, &result)?;
    Ok(CaseOutcome { spec, result: Some(result), error: None, cached: false, warm })
}

#[derive(Clone, Copy, PartialEq)]
enum St {
    Pending,
    Running,
    Finished,
}

/// Runs the whole case matrix (see the module documentation). Returns the outcomes in case order;
/// failed cases are outcomes with `result = None`. Errors only on interruption or I/O problems.
pub fn run_all(setup: &RunSetup, progress: &(dyn Fn(CfdStage) + Sync)) -> Result<Vec<CaseOutcome>> {
    let cases = case_list(setup.opt);
    let n = cases.len();
    let _sig = SignalGuard::new();
    std::fs::create_dir_all(&setup.work)?;
    let hashes: Vec<String> = cases.iter().map(|c| case_hash(&base_config(setup, c), &setup.mesh_hash)).collect();
    let sources: Vec<Option<usize>> = (0..n).map(|i| warm_start_source(&cases, i)).collect();
    let mut outcomes: Vec<Option<CaseOutcome>> = vec![None; n];
    let mut state = vec![St::Pending; n];
    for i in 0..n {
        if let Some(r) = read_done(&cases[i].dir(&setup.work), &hashes[i]) {
            progress(CfdStage::CaseDone { index: i, total: n, mach: cases[i].mach, alpha: cases[i].alpha_deg, converged: r.converged });
            outcomes[i] = Some(CaseOutcome { spec: cases[i], result: Some(r), error: None, cached: true, warm: false });
            state[i] = St::Finished;
        }
    }
    let cleanup = |state: &[St]| {
        for s in 0..n {
            if state[s] == St::Finished && (0..n).filter(|&j| sources[j] == Some(s)).all(|j| state[j] == St::Finished) {
                let _ = std::fs::remove_file(cases[s].dir(&setup.work).join("restart.dat"));
            }
        }
    };
    cleanup(&state);
    let mut error: Option<anyhow::Error> = None;
    std::thread::scope(|sc| {
        let (tx, rx) = mpsc::channel::<Event>();
        let mut running = 0usize;
        loop {
            if error.is_none() && !CANCEL.load(Ordering::SeqCst) {
                while running < setup.parallel.max(1) {
                    let Some(i) = (0..n).find(|&i| state[i] == St::Pending && sources[i].is_none_or(|s| state[s] == St::Finished)) else { break };
                    state[i] = St::Running;
                    running += 1;
                    let restart = sources[i]
                        .filter(|&s| outcomes[s].as_ref().is_some_and(|o| o.result.is_some()))
                        .map(|s| cases[s].dir(&setup.work).join("restart.dat"))
                        .filter(|p| p.exists());
                    progress(CfdStage::Case { index: i, total: n, mach: cases[i].mach, alpha: cases[i].alpha_deg, iter: 0, residual: 0.0 });
                    let tx = tx.clone();
                    let (spec, hash) = (cases[i], hashes[i].clone());
                    sc.spawn(move || {
                        let r = run_case(setup, i, spec, &hash, restart, &tx);
                        let outcome = r.unwrap_or_else(|e| CaseOutcome { spec, result: None, error: Some(e.to_string()), cached: false, warm: false });
                        let _ = tx.send(Event::Finished { i, outcome });
                    });
                }
            }
            if running == 0 {
                break;
            }
            match rx.recv_timeout(Duration::from_millis(300)) {
                Ok(Event::Progress { i, iter, residual }) => {
                    progress(CfdStage::Case { index: i, total: n, mach: cases[i].mach, alpha: cases[i].alpha_deg, iter, residual })
                }
                Ok(Event::Finished { i, outcome }) => {
                    running -= 1;
                    state[i] = St::Finished;
                    if CANCEL.load(Ordering::SeqCst) {
                        error = Some(anyhow!("interrupted"));
                    } else {
                        let conv = outcome.result.as_ref().is_some_and(|r| r.converged);
                        progress(CfdStage::CaseDone { index: i, total: n, mach: cases[i].mach, alpha: cases[i].alpha_deg, converged: conv });
                    }
                    outcomes[i] = Some(outcome);
                    cleanup(&state);
                }
                Err(_) => {}
            }
        }
    });
    if let Some(e) = error {
        return Err(e);
    }
    if CANCEL.load(Ordering::SeqCst) {
        bail!("interrupted");
    }
    Ok(outcomes.into_iter().map(|o| o.expect("every case has an outcome")).collect())
}

/// Writes `cfd_cases.csv`: every solved case (raw, including the negative angles of `mirror` mode)
/// with its convergence information, followed in `mirror` mode by the combined odd/even rows
/// (`status = combined`, `corrected` are the points of [`crate::table::apply_alpha_mode`]).
/// `cn`, `nose_moment`, `xcp_m` are the values the table uses: offset-corrected in `offset` mode,
/// equal to the raw ones in `single` mode, empty on raw `mirror` rows.
pub fn write_cases_csv(path: &Path, outcomes: &[CaseOutcome], corrected: &[SolvedPoint], mode: AlphaMode) -> Result<()> {
    use std::fmt::Write;
    let mut s = String::from("mach,alpha_deg,status,iterations,residual_first,residual_last,cn_raw,ca_cfd,nose_moment_raw,xcp_raw_m,cn,nose_moment,xcp_m,wall_seconds,warm_start\n");
    let f = |v: Option<f64>| v.map_or(String::new(), |x| format!("{x:.8e}"));
    let corr = |m: f64, a: f64| corrected.iter().find(|p| p.mach == m && p.alpha_deg == a).and_then(|p| p.coeffs);
    let used = |c: &BodyCoeffs| (Some(c.cn), Some(c.mom), c.xcp());
    for o in outcomes {
        let status = match &o.result {
            Some(r) if r.converged => "converged",
            Some(_) => "accepted",
            None => "failed",
        };
        let r = o.result.as_ref();
        let (cn, mom, xcp) = if mode == AlphaMode::Mirror { (None, None, None) } else { corr(o.spec.mach, o.spec.alpha_deg).as_ref().map_or((None, None, None), used) };
        let _ = writeln!(
            s,
            "{},{},{},{},{},{},{},{},{},{},{},{},{},{:.1},{}",
            o.spec.mach,
            o.spec.alpha_deg,
            status,
            r.map_or(0, |r| r.iterations),
            f(r.and_then(|r| r.residual_first)),
            f(r.and_then(|r| r.residual_last)),
            f(r.map(|r| r.coeffs.cn)),
            f(r.map(|r| r.coeffs.ca)),
            f(r.map(|r| r.coeffs.mom)),
            f(r.and_then(|r| r.coeffs.xcp())),
            f(cn),
            f(mom),
            f(xcp),
            r.map_or(0.0, |r| r.wall_seconds),
            o.warm
        );
    }
    if mode == AlphaMode::Mirror {
        for p in corrected.iter().filter(|p| p.alpha_deg > 0.0) {
            let (cn, mom, xcp) = p.coeffs.as_ref().map_or((None, None, None), used);
            let _ = writeln!(s, "{},{},combined,,,,,{},,,{},{},{},,", p.mach, p.alpha_deg, f(p.coeffs.map(|c| c.ca)), f(cn), f(mom), f(xcp));
        }
    }
    std::fs::write(path, s).with_context(|| format!("cannot write {}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::ToolInfo;
    use aero::atmosphere::AtmosphereModel;
    use std::os::unix::fs::PermissionsExt;

    /// Mock SU2: parses MACH_NUMBER / AOA / RESTART_SOL from case.cfg, writes a fake history.csv.
    /// MODE (first line of `mode` file next to the script): ok | fail | stall | slow
    const MOCK: &str = r#"#!/bin/sh
cfg=$1
mach=$(grep '^MACH_NUMBER' $cfg | cut -d' ' -f2)
aoa=$(grep '^AOA' $cfg | cut -d' ' -f2)
restart=$(grep '^RESTART_SOL' $cfg | cut -d' ' -f2)
mode=$(cat "$(dirname "$0")/mode")
echo "$mach $aoa $restart" >> "$(dirname "$0")/calls"
echo "mock su2 $mode"
[ "$mode" = fail ] && { echo "boom" >&2; exit 1; }
echo '"Inner_Iter","rms[Rho]","CFx","CFz","CMy"' > history.csv
i=0
while [ $i -lt 30 ]; do
  case $mode in
    stall) r=$(echo "-1.0 - 0.02*$i" | bc -l);;
    unstable) r=$(echo "-1.0 - 0.2*$i" | bc -l);;
    *) r=$(echo "-1.0 - 0.25*$i" | bc -l);;
  esac
  fx=0.3; fz=$(echo "$aoa * 0.05" | bc -l); my=$(echo "0 - $aoa * 0.08" | bc -l)
  [ "$mode" = unstable ] && { par=$((i % 2)); fz=$(echo "$aoa * 0.05 + $par * 0.3" | bc -l); }
  echo "$i,$r,$fx,$fz,$my" >> history.csv
  [ "$mode" = ok ] && [ $i -ge 24 ] && break
  i=$((i+1))
done
echo restart > restart.dat
exit 0
"#;

    fn tools_with(script: &Path) -> ToolReport {
        let ok = |p: Option<PathBuf>| ToolInfo { name: "x", path: p, version: Some("mock".into()), problem: None };
        ToolReport { prefix: None, su2: ok(Some(script.to_path_buf())), mpi: ok(None), gmsh: ok(None) }
    }

    struct Env {
        dir: PathBuf,
        tools: ToolReport,
        opt: CfdOptions,
    }

    fn env(name: &str, mode: &str) -> Env {
        let dir = std::env::temp_dir().join(format!("ignisyeet_runner_{name}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let script = dir.join("su2_mock.sh");
        std::fs::write(&script, MOCK).unwrap();
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
        std::fs::write(dir.join("mode"), mode).unwrap();
        std::fs::write(dir.join("mesh.su2"), "x").unwrap();
        let opt = CfdOptions {
            machs: vec![0.5, 2.0],
            alphas_deg: vec![0.0, 4.0, 8.0],
            alpha_mode: AlphaMode::Single,
            ranks_per_case: 1,
            parallel_cases: 2,
            iterations: 100,
            ..Default::default()
        };
        Env { tools: tools_with(&script), dir, opt }
    }

    impl Env {
        fn setup(&self) -> RunSetup<'_> {
            RunSetup {
                opt: &self.opt,
                dims: RefDims { length: 0.1, area: 0.0078 },
                atm: AtmosphereModel::default().at(0.0),
                mesh_hash: "mh".into(),
                mesh_file: self.dir.join("mesh.su2"),
                work: self.dir.join("work"),
                ranks: self.opt.ranks_per_case,
                parallel: self.opt.parallel_cases,
                tools: &self.tools,
            }
        }
        fn calls(&self) -> Vec<String> {
            std::fs::read_to_string(self.dir.join("calls")).unwrap_or_default().lines().map(str::to_string).collect()
        }
    }

    // The runner tests share the process-wide cancel flag; run them one at a time.
    static SERIAL: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[test]
    fn runs_matrix_with_warm_starts_and_progress() {
        let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
        let e = env("ok", "ok");
        let events = std::sync::Mutex::new(Vec::new());
        let out = run_all(&e.setup(), &|s| events.lock().unwrap().push(s)).unwrap();
        assert_eq!(out.len(), 6);
        assert!(out.iter().all(|o| o.result.as_ref().is_some_and(|r| r.converged)), "{out:?}");
        // Mock coefficients: CFz = 0.05 alpha, CMy = -0.08 alpha, L = 0.1 -> xcp = 0.08 * 0.1 / 0.05... via forces.
        let r = out[1].result.as_ref().unwrap();
        assert!((r.coeffs.cn - 0.2).abs() < 1e-9 && (r.coeffs.ca - 0.3).abs() < 1e-9);
        assert!((r.coeffs.xcp().unwrap() - 0.08 * 4.0 * 0.1 / 0.2).abs() < 1e-9);
        // The first case is cold; alpha 4 and 8 of Mach 0.5 and alpha 0 of Mach 2 are warm.
        assert!(!out[0].warm && out[1].warm && out[2].warm && out[3].warm);
        let calls = e.calls();
        assert_eq!(calls.len(), 6);
        assert_eq!(calls.iter().filter(|c| c.ends_with(" NO")).count(), 1, "{calls:?}");
        let ev = events.lock().unwrap();
        assert_eq!(ev.iter().filter(|s| matches!(s, CfdStage::CaseDone { .. })).count(), 6);
        assert!(ev.iter().any(|s| matches!(s, CfdStage::Case { .. })));
        // Restart files are removed once nothing needs them.
        assert!(!out[0].spec.dir(&e.dir.join("work")).join("restart.dat").exists());
        let pts: Vec<SolvedPoint> = out.iter().map(|o| SolvedPoint { mach: o.spec.mach, alpha_deg: o.spec.alpha_deg, coeffs: o.result.as_ref().map(|r| r.coeffs) }).collect();
        let (corrected, _) = crate::table::apply_alpha_mode(&pts, AlphaMode::Single);
        write_cases_csv(&e.dir.join("cases.csv"), &out, &corrected, AlphaMode::Single).unwrap();
        let csv = std::fs::read_to_string(e.dir.join("cases.csv")).unwrap();
        assert_eq!(csv.lines().count(), 7);
        assert!(csv.contains("converged"));
        let _ = std::fs::remove_dir_all(&e.dir);
    }

    #[test]
    fn mirror_mode_runs_negative_angles() {
        let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
        let mut e = env("mirror", "ok");
        e.opt.alpha_mode = AlphaMode::Mirror;
        e.opt.z_mirror_mesh = false;
        e.opt.alphas_deg = vec![0.0, 4.0];
        let out = run_all(&e.setup(), &|_| {}).unwrap();
        assert_eq!(out.len(), 6);
        let neg = out.iter().find(|o| o.spec.alpha_deg == -4.0 && o.spec.mach == 0.5).unwrap();
        assert!(neg.result.is_some() && neg.warm, "{neg:?}");
        assert!(neg.spec.dir(&e.dir.join("work")).join("done.json").exists());
        assert!(e.dir.join("work/m0.500_a-04.00").is_dir());
        // Mock: CFz = 0.05 alpha (odd), so the combination reproduces CN = 0.2 at +4 deg.
        let pts: Vec<SolvedPoint> = out.iter().map(|o| SolvedPoint { mach: o.spec.mach, alpha_deg: o.spec.alpha_deg, coeffs: o.result.as_ref().map(|r| r.coeffs) }).collect();
        let (corrected, asym) = crate::table::apply_alpha_mode(&pts, AlphaMode::Mirror);
        assert_eq!((corrected.len(), asym.len()), (4, 2));
        assert!((corrected.iter().find(|p| p.alpha_deg == 4.0).unwrap().coeffs.unwrap().cn - 0.2).abs() < 1e-9);
        write_cases_csv(&e.dir.join("cases.csv"), &out, &corrected, AlphaMode::Mirror).unwrap();
        let csv = std::fs::read_to_string(e.dir.join("cases.csv")).unwrap();
        assert_eq!(csv.lines().count(), 1 + 6 + 2);
        assert_eq!(csv.lines().filter(|l| l.contains(",combined,")).count(), 2);
        let _ = std::fs::remove_dir_all(&e.dir);
    }

    #[test]
    fn resume_skips_finished_cases_and_reruns_changed_ones() {
        let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
        let mut e = env("resume", "ok");
        run_all(&e.setup(), &|_| {}).unwrap();
        assert_eq!(e.calls().len(), 6);
        // Second run: nothing is executed.
        std::fs::write(e.dir.join("mode"), "fail").unwrap();
        let out = run_all(&e.setup(), &|_| {}).unwrap();
        assert!(out.iter().all(|o| o.cached && o.result.is_some()));
        assert_eq!(e.calls().len(), 6);
        // Interrupted earlier: delete one done.json -> only that case is run again.
        std::fs::write(e.dir.join("mode"), "ok").unwrap();
        let victim = out[4].spec.dir(&e.dir.join("work")).join("done.json");
        std::fs::remove_file(&victim).unwrap();
        let out = run_all(&e.setup(), &|_| {}).unwrap();
        assert_eq!(e.calls().len(), 7);
        assert!(!out[4].cached && out[3].cached);
        // A different mesh invalidates everything.
        let setup = RunSetup { mesh_hash: "other".into(), ..e.setup() };
        let n0 = e.calls().len();
        run_all(&setup, &|_| {}).unwrap();
        assert_eq!(e.calls().len(), n0 + 6);
        e.opt.cfl = 3.0;
        run_all(&e.setup(), &|_| {}).unwrap();
        assert_eq!(e.calls().len(), n0 + 12);
        let _ = std::fs::remove_dir_all(&e.dir);
    }

    #[test]
    fn failures_are_recorded_not_fatal() {
        let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
        let e = env("fail", "fail");
        let out = run_all(&e.setup(), &|_| {}).unwrap();
        assert_eq!(out.len(), 6);
        assert!(out.iter().all(|o| o.result.is_none()));
        let msg = out[0].error.as_ref().unwrap();
        assert!(msg.contains("exited") && msg.contains("boom"), "{msg}");
        assert!(!out[0].spec.dir(&e.dir.join("work")).join("done.json").exists());
        // After a failure the dependants start cold.
        assert_eq!(e.calls().iter().filter(|c| c.ends_with(" NO")).count(), 6);
        let _ = std::fs::remove_dir_all(&e.dir);
    }

    #[test]
    fn unconverged_cases_are_accepted_only_when_practically_converged() {
        let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
        // `stall`: residual -1 .. -1.6 (drop < 3 orders), coefficients constant -> failed.
        let e = env("stall", "stall");
        let out = run_all(&e.setup(), &|_| {}).unwrap();
        assert!(out.iter().all(|o| o.result.is_none()));
        assert!(out[0].error.as_ref().unwrap().contains("not converged"));
        let _ = std::fs::remove_dir_all(&e.dir);
        // `slowdrop`: reaches -1 - 0.25 * 29 = -8.25 > ... use a looser target so the loop ends above it.
        let mut e = env("accept", "ok");
        e.opt.convergence = 1e-12; // target -12, never reached: residual ends at -7.25
        let out = run_all(&e.setup(), &|_| {}).unwrap();
        let r = out[1].result.as_ref().expect("accepted");
        assert!(!r.converged && (r.coeffs.cn - 0.2).abs() < 1e-9);
        let _ = std::fs::remove_dir_all(&e.dir);
        // Unstable coefficients with a big residual drop are rejected.
        let mut e = env("unstable", "unstable");
        e.opt.convergence = 1e-12;
        let out = run_all(&e.setup(), &|_| {}).unwrap();
        let a4 = out.iter().find(|o| o.spec.alpha_deg == 4.0).unwrap();
        assert!(a4.result.is_none() && a4.error.as_ref().unwrap().contains("still changing"), "{:?}", a4.error);
        let _ = std::fs::remove_dir_all(&e.dir);
    }

    #[test]
    fn missing_history_is_a_failure() {
        let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
        let mut e = env("nohist", "ok");
        // a solver that exits successfully without writing anything (/bin/true is not on macOS)
        let quiet = e.dir.join("su2_quiet.sh");
        std::fs::write(&quiet, "#!/bin/sh\nexit 0\n").unwrap();
        std::fs::set_permissions(&quiet, std::fs::Permissions::from_mode(0o755)).unwrap();
        e.tools.su2.path = Some(quiet);
        let out = run_all(&e.setup(), &|_| {}).unwrap();
        assert!(out.iter().all(|o| o.error.as_ref().is_some_and(|m| m.contains("no history"))));
        let _ = std::fs::remove_dir_all(&e.dir);
    }
}
