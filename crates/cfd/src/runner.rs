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
//! (RANS cold starts also `case_startup.cfg`, `su2_startup.log`, `history_startup.csv` of the first-order stage)
//! and, when finished, `done.json` (input hash, coefficients, convergence). A valid `done.json`
//! makes the case a no-op on the next run (resume). The SU2 command is
//! `mpirun -n <ranks> --bind-to none SU2_CFD case.cfg`, or plain `SU2_CFD case.cfg` for one rank.
//! `history.csv` is polled once a second for the live progress.
//!
//! # Acceptance (Euler)
//!
//! SU2 runs to a residual floor far below the target (`CONV_RESIDUAL_MINVAL = min(log10 convergence - 2, -14)`, see
//! `su2cfg`); the runner decides. "Settled" is [`History::euler_trend_settled`]: over the last `stable_window` rows the
//! range of `CFx`, `CFz` (`CFy` with sideslip) is within 0.2 % of the force scale and the centre of pressure within
//! 0.002 body lengths, and so is their change still to come, extrapolated from the trend over the window with the
//! residual decay rate.
//!
//! * Early stop, checked once a second: the residual target reached (and at least `stable_window` rows), or practically
//!   converged (after `min_iterations`, the density residual 3 orders below its peak and the coefficients flat over
//!   `stable_window`, see `History::coefficients_stable`), each with the coefficients settled. SU2 is stopped after
//!   its next periodic output (restart file of the warm successor, surface file); the case is judged on the history
//!   and the verdict of that decision (the rows written until the output cannot overturn it). Target reached:
//!   `converged`, coefficients of the last row; practically converged: accepted, `converged = false`, coefficients
//!   averaged over the last `n / 4` rows (2 to 100).
//! * Iteration limit / timeout with the target reached but the coefficients not settled: accepted, `converged = false`,
//!   coefficients averaged over the last `n / 4` rows (2 to 100), with a warning.
//! * Iteration limit / timeout without the target: accepted (`converged = false`) when the density residual dropped at
//!   least 3 orders from its peak and `CFx`, `CFz`, `CMy` varied by less than 1 % over the last `n / 4` rows (2 to
//!   100); the coefficients are averaged over those rows (with a warning when they have not settled).
//! * Otherwise (SU2 error, no history, not converged and not stable) the case is *failed*: no `done.json`, the table
//!   builder bridges it by interpolation.
//!
//! # RANS
//!
//! A cold case runs the first-order startup and then the second-order stage in its directory (see `su2cfg`); a
//! warm start (alpha and beta chains of one Mach number; every Mach number starts cold, see
//! [`crate::case::warm_start_source_for`]) runs the second-order stage only, from the neighbour's restart file,
//! which must carry the SA variable (an Euler solution never seeds a RANS case). A warm start that fails is
//! solved again cold. A diverging startup or second-order stage of a cold case is repeated once with conservative
//! settings (see `su2cfg`). The reference of the acceptance is the residual peak of the cold startup stage
//! ([`ResidualPeak`], stored in `done.json` and passed along the warm chain), raised to the run's own peak when
//! that is higher; a residual drop measured from a warm run's own (low) peak would accept it too early. The drops
//! are taken from the largest residual of the last [`RANS_RESIDUAL_ROWS`] rows. [`rans_verdict`] gives one of:
//!
//! * [`RansVerdict::Converged`]: density residual 3 and SA residual 2 orders below the reference, the coefficients
//!   flat over `stable_window` iterations ([`History::rans_coefficients_stable`]: forces 0.5 % of the force scale,
//!   centre of pressure 0.002 body lengths), and their change still to come, extrapolated from the trend over the
//!   window with the residual decay rate ([`History::rans_trend_settled`]), within the same tolerances. On the TMR
//!   flat plate (69 x 49 to 273 x 193, M 0.2 and M 2) the drag at the stop is then within 0.3 % of the value 8 orders
//!   down (flatness alone: up to 1.9 %), at the cost of up to 950 more iterations on the finest grid.
//! * [`RansVerdict::Stalled`]: the median density residual of the last `stable_window` rows at least
//!   [`RANS_STALL_RHO_DROP`] orders below the reference and no more than [`RANS_STALL_PROGRESS`] below the median of
//!   the window before, with the coefficients flat and settled over both windows: accepted, `converged = false`,
//!   coefficients averaged over both windows, the residual level and the force spread in a warning; `done.json`
//!   (`stalled`, `residual_drop`), `cfd_cases.csv` (`rans_verdict = stalled`) and `cfd_report.json`
//!   (`stalled_cases`) record it.
//! * [`RansVerdict::LimitCycle`]: density residual 2 orders below the reference and the forces oscillating within
//!   1 % of the force scale ([`History::limit_cycle`]): accepted with the window average, `converged = false`.
//! * [`RansVerdict::Rejected`]: failed (a warm start is solved again cold first).
//!
//! The runner checks the verdict once a second after `min_iterations` and stops SU2 at `Converged` or `Stalled`,
//! after its next periodic output; the case is judged on the history and verdict of that decision. Without a stop the
//! verdict is taken at the end. Stalled and limit-cycle cases are listed as unconverged (`status = unconverged` in
//! `cfd_cases.csv`). After acceptance the wall y+ of `surface_flow.vtk` is checked: a 95th percentile above 3
//! (resolved wall) is recorded as a warning in `done.json`.
//!
//! Children run in their own process group and are killed on Ctrl-C / SIGTERM; finished cases
//! keep their `done.json`.
//!
//! # Commands, divergence and stopping
//!
//! The command line is `mpirun -n <ranks> --oversubscribe --bind-to none SU2_CFD case.cfg`; a run is a single node, so
//! MPI communicates through shared memory only. The second-order stage of a RANS case is read once a second; a
//! divergence stops SU2 at once ([`History::rans_divergence`]: on the median of the last five rows the density
//! residual 2 orders (the SA residual 3 orders) above its minimum since row 10, a non-finite value, or a force
//! coefficient above 100). A diverged first-order stage is repeated once with the adaptive CFL capped at `4 cfl`
//! (normally `40 cfl`) and the SA equation at 0.2 of the flow CFL; the second-order stage keeps those settings. A
//! diverged second-order stage of a cold case is repeated once from the startup solution with the same settings.
//! Every repeat is recorded as a warning; a second divergence fails the case. SU2 writes nothing when it is
//! stopped by SIGTERM, so an early stop waits for the next periodic output (every 50 iterations) and one more history
//! row, at most 105 rows, so that the warm-start solution and the wall `y+` distribution are those of the accepted
//! state. Failed cases are reported together at the end as `error[IY-F1nn]: aero.cfd: case ... failed: <reason>`;
//! a kill by signal 9 adds the memory estimate per case to the reason. Case directories are named
//! `m0.800_a04.00` (negative angles `a-04.00`, sideslip `_b04.00`, roll cases `_r+0.0100` and `_r-0.0100`).
//!
//! # Verification record
//!
//! The acceptance rules were set with the measurements in `doc/data/cfd` (`rans_flatplate.csv` for the flat-plate
//! replay of the stopping rule, `rans_table_cases.csv` for the 20-case RANS table, `fin_resolution.csv` and
//! `mesh_convergence.csv` for the Euler mesh studies). Euler: 0.56 % low `CN` at the iteration where the density
//! residual first reached 1e-6 (M 0.8 sample rocket), which is why the residual alone does not stop a case.

use crate::case::{case_hash, case_list, read_done, warm_start_source_for, write_done, AxialRecord, CaseResult, CaseSpec, ResidualPeak};
use crate::config::{AlphaMode, CfdOptions, FlowModel};
use crate::forces::{BodyCoeffs, BodyCoeffs6, MarkerCoeffs, Su2Coeffs, Su2Coeffs6};
use crate::history::{summarize, History, HistorySummary, StabilityKind};
use crate::su2cfg::{su2_config_case, CaseFiles, RefDims, Stage, STARTUP_RESTART};
use crate::table::SolvedPoint;
use crate::tools::{self, ToolReport};
use crate::CfdStage;
use aero::atmosphere::Atmosphere;
use anyhow::{anyhow, bail, Context, Result};
use std::path::{Path, PathBuf};
use std::process::{Child, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::time::{Duration, Instant};

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

/// Unix process-group and signal handling. The CFD mode needs a Unix-like system; elsewhere `run_all` fails early.
#[cfg(unix)]
mod sys {
    use super::CANCEL;
    use std::os::unix::process::CommandExt;
    use std::process::{Child, Command};
    use std::sync::atomic::Ordering;

    pub const SIGTERM: i32 = libc::SIGTERM;
    pub const SIGKILL: i32 = libc::SIGKILL;

    extern "C" fn on_signal(_: libc::c_int) {
        CANCEL.store(true, Ordering::SeqCst);
    }

    /// Installs the Ctrl-C / SIGTERM handlers for the duration of a run.
    pub struct SignalGuard;

    impl SignalGuard {
        pub fn new() -> Self {
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

    pub fn own_group(c: &mut Command) -> &mut Command {
        c.process_group(0)
    }

    pub fn kill_group(child: &Child, sig: i32) {
        unsafe {
            libc::kill(-(child.id() as i32), sig);
        }
    }
}

#[cfg(not(unix))]
mod sys {
    use super::CANCEL;
    use std::process::{Child, Command};
    use std::sync::atomic::Ordering;

    pub const SIGTERM: i32 = 15;
    pub const SIGKILL: i32 = 9;

    pub struct SignalGuard;

    impl SignalGuard {
        pub fn new() -> Self {
            CANCEL.store(false, Ordering::SeqCst);
            SignalGuard
        }
    }

    pub fn own_group(c: &mut Command) -> &mut Command {
        c
    }

    pub fn kill_group(child: &Child, _sig: i32) {
        // Not reachable in practice: `run_all` refuses to start. Best effort only.
        let _ = child.id();
    }
}

use sys::SignalGuard;

/// A child process group that is killed when dropped.
struct Group(Child);

impl Group {
    fn kill(&mut self, sig: i32) {
        sys::kill_group(&self.0, sig);
    }
}

impl Drop for Group {
    fn drop(&mut self) {
        if matches!(self.0.try_wait(), Ok(None)) {
            self.kill(sys::SIGTERM);
            std::thread::sleep(Duration::from_millis(200));
            self.kill(sys::SIGKILL);
            let _ = self.0.wait();
        }
    }
}

/// Text the input hash is taken of: the SU2 configuration and, for RANS, the base treatment (it decides which
/// force the stored axial coefficient holds but is not part of the SU2 configuration).
fn hash_text(setup: &RunSetup, spec: &CaseSpec) -> String {
    let cfg = base_config(setup, spec);
    if setup.opt.model == FlowModel::Rans {
        format!("{cfg}\n# base_from_cfd = {}", setup.opt.base_from_cfd())
    } else {
        cfg
    }
}

/// SU2 configuration text of a case without the restart line.
fn base_config(setup: &RunSetup, spec: &CaseSpec) -> String {
    su2_config_case(
        setup.opt,
        spec,
        &setup.dims,
        &setup.atm,
        &CaseFiles { mesh: setup.mesh_file.display().to_string(), restart_from: None, stage: Stage::Final, conservative: false },
    )
}

/// Number of grid levels SU2 reports in its "MG Level | CVs | Aggl. Rate" table (fine grid included; 0 when no table).
pub fn multigrid_levels_built(log_text: &str) -> usize {
    let Some(i) = log_text.find("Aggl. Rate") else { return 0 };
    log_text[i..].lines().skip(2).take_while(|l| l.starts_with('|')).filter(|l| l.split('|').nth(1).is_some_and(|c| c.trim().parse::<usize>().is_ok())).count()
}

/// SU2 builds the coarse grids by agglomeration and silently drops the levels when the mesh does not coarsen well
/// (wall-dominated tetrahedral meshes do not): say so once per run when `multigrid` was requested.
fn note_multigrid(opt: &CfdOptions, log: &Path) {
    static NOTED: AtomicBool = AtomicBool::new(false);
    let want = opt.multigrid_levels();
    if want == 0 || NOTED.load(Ordering::Relaxed) {
        return;
    }
    let built = multigrid_levels_built(&std::fs::read_to_string(log).unwrap_or_default()).saturating_sub(1);
    if built < want && !NOTED.swap(true, Ordering::Relaxed) {
        eprintln!(
            "note: aero.cfd: multigrid = {want} requested, but SU2 built {built} coarse grid level{} for this mesh (agglomeration of a wall-dominated tetrahedral mesh is poor); \
             {}",
            if built == 1 { "" } else { "s" },
            if built == 0 { "the cases run on the fine grid only, multigrid has no effect" } else { "the cases use the levels that exist" }
        );
    }
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
    Finished { i: usize, outcome: Box<CaseOutcome> },
}

fn command(setup: &RunSetup, dir: &Path, cfg: &str, log: &str) -> Result<std::process::Command> {
    let t = setup.tools;
    let su2 = t.su2.path.clone().context("SU2_CFD not found")?;
    let mut c = if setup.ranks > 1 {
        let mpi = t.mpi.path.clone().context("mpirun not found")?;
        let mut c = tools::command(&mpi, t.prefix.as_deref());
        c.arg("-n").arg(setup.ranks.to_string()).args(["--oversubscribe", "--bind-to", "none"]).arg(&su2).arg(cfg);
        c
    } else {
        let mut c = tools::command(&su2, t.prefix.as_deref());
        c.arg(cfg);
        c
    };
    let log = std::fs::File::create(dir.join(log))?;
    // Single-node runs: shared-memory transport only (the TCP/UCX components probe network interfaces
    // and fail or spam errors on machines with virtual / unreachable interfaces).
    c.env("OMPI_MCA_btl", "self,sm").env("OMPI_MCA_osc", "^ucx").env("OMPI_MCA_pml", "ob1");
    c.current_dir(dir).env("OMP_NUM_THREADS", "1").stdin(Stdio::null()).stdout(log.try_clone()?).stderr(log);
    sys::own_group(&mut c);
    Ok(c)
}

/// What the early-stop check of [`solve`] asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StopAction {
    Continue,
    /// Stop now (SU2 exits on SIGTERM without writing its output files).
    Now,
    /// Stop once SU2 has written its next periodic output (`OUTPUT_WRT_FREQ`): the restart file of a warm start
    /// and the surface file of the y+ check are then those of the accepted solution. At most this many more rows.
    AfterOutput(usize),
}

/// Files whose rewrite [`StopAction::AfterOutput`] waits for.
const OUTPUT_FILES: [&str; 2] = ["restart.dat", "surface_flow.vtk"];

/// One SU2 process of a case.
struct Solve {
    /// `None`: stopped by the runner (early stop or timeout).
    status: Option<std::process::ExitStatus>,
    timed_out: bool,
}

/// Writes `cfg_text` to `dir/cfg`, runs SU2 on it (log `dir/log`) and polls `dir/history` once a second for the
/// progress and for `stop`, which ends the run gracefully (SU2 writes its restart and surface files on SIGTERM).
/// Returns `Err` only when cancelled.
#[allow(clippy::too_many_arguments)]
fn solve(
    setup: &RunSetup,
    i: usize,
    dir: &Path,
    (cfg, log, history): (&str, &str, &str),
    cfg_text: &str,
    deadline: Option<Instant>,
    stop: &dyn Fn(&History) -> StopAction,
    tx: &mpsc::Sender<Event>,
) -> Result<std::result::Result<Solve, String>> {
    std::fs::write(dir.join(cfg), cfg_text)?;
    // A history left by an earlier attempt must not be judged as this run's.
    let _ = std::fs::remove_file(dir.join(history));
    let mut cmd = command(setup, dir, cfg, log)?;
    let mut child = match cmd.spawn() {
        Ok(c) => Group(c),
        Err(e) => return Ok(Err(format!("[IY-F110] cannot start SU2: {e}"))),
    };
    let mut last_poll = Instant::now() - Duration::from_secs(10);
    // AfterOutput: (time of the decision, rows then, row limit, rows when the new output was first seen)
    let mut pending: Option<(std::time::SystemTime, usize, usize, Option<usize>)> = None;
    let written_after = |t: std::time::SystemTime| OUTPUT_FILES.iter().all(|f| std::fs::metadata(dir.join(f)).and_then(|m| m.modified()).is_ok_and(|m| m > t));
    loop {
        if CANCEL.load(Ordering::SeqCst) {
            child.kill(sys::SIGTERM);
            std::thread::sleep(Duration::from_millis(300));
            child.kill(sys::SIGKILL);
            let _ = child.0.wait();
            bail!("[IY-F111] interrupted");
        }
        if let Some(st) = child.0.try_wait()? {
            return Ok(Ok(Solve { status: Some(st), timed_out: false }));
        }
        if deadline.is_some_and(|d| Instant::now() > d) {
            child.kill(sys::SIGTERM);
            std::thread::sleep(Duration::from_millis(500));
            child.kill(sys::SIGKILL);
            let _ = child.0.wait();
            return Ok(Ok(Solve { status: None, timed_out: true }));
        }
        if last_poll.elapsed() >= Duration::from_secs(1) {
            last_poll = Instant::now();
            // Checked before the history is read: the row count read next then includes the row of the output
            // iteration (SU2 writes the history row before the output files), so "one more row" is a row SU2 wrote
            // after finishing the files. Checked after the read, a rewrite that lands in between would be paired with
            // the stale count, and the stop could come while the files are still being written.
            let output_seen = matches!(pending, Some((t, _, _, None)) if written_after(t));
            if let Ok(h) = History::read(&dir.join(history)) {
                if let Some(r) = h.last_residual() {
                    let _ = tx.send(Event::Progress { i, iter: h.last_iteration(), residual: r });
                }
                let now = match pending {
                    None => match stop(&h) {
                        StopAction::Continue => false,
                        StopAction::Now => true,
                        StopAction::AfterOutput(extra) => {
                            pending = Some((std::time::SystemTime::now(), h.rows.len(), extra, None));
                            false
                        }
                    },
                    Some((t, rows, extra, seen)) => {
                        // One more history row after the rewrite was seen: the files are complete.
                        let done = seen.is_some_and(|r| h.rows.len() > r);
                        if seen.is_none() && output_seen {
                            pending = Some((t, rows, extra, Some(h.rows.len())));
                        }
                        done || h.rows.len() > rows + extra
                    }
                };
                if now {
                    child.kill(sys::SIGTERM);
                    let t = Instant::now();
                    while t.elapsed() < Duration::from_secs(30) && child.0.try_wait()?.is_none() {
                        std::thread::sleep(Duration::from_millis(100));
                    }
                    return Ok(Ok(Solve { status: None, timed_out: false }));
                }
            }
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}

/// The history a finished run is judged on and its verdict. With an early-stop `decision` (rows of the history then,
/// the verdict then): the history up to that row and that verdict, so that the rows SU2 runs on until its next periodic
/// output cannot overturn the decision the runner stopped it on. Without one: the whole history and `judge` of it.
fn judged_at_stop<V>(h: &History, decision: Option<(usize, V)>, judge: impl FnOnce(&History) -> V) -> (History, V) {
    match decision {
        Some((rows, v)) if rows >= 1 && rows <= h.rows.len() => (History { columns: h.columns.clone(), rows: h.rows[..rows].to_vec() }, v),
        _ => (h.clone(), judge(h)),
    }
}

/// Coefficients of a finished case: the body coefficients and, for RANS, the per-marker axial record.
type CaseCoeffs = (HistorySummary, BodyCoeffs, Option<BodyCoeffs6>, Option<AxialRecord>);

/// Body coefficients of a case from the last `window` rows of its history (the single place where the history
/// becomes [`BodyCoeffs`] / [`BodyCoeffs6`]). RANS: the axial coefficient is the wall marker (plus the base marker
/// with `rans_base = "cfd"`), averaged over the same rows as the totals; without marker columns it is the total.
fn coefficients_from_history(setup: &RunSetup, spec: &CaseSpec, h: &History, window: usize) -> std::result::Result<CaseCoeffs, String> {
    let s = summarize(h, window).ok_or("empty history")?;
    let mut su2 = Su2Coeffs::from_summary(&s, spec.alpha_deg).map_err(|e| e.to_string())?;
    let mut su26 = Su2Coeffs6::from_summary(&s).ok();
    let mut axial = None;
    if setup.opt.model == FlowModel::Rans {
        let m = MarkerCoeffs::from_history(h, window);
        let base = setup.opt.base_from_cfd();
        let (c, split) = su2.with_markers(&m, base);
        su2 = c;
        su26 = su26.map(|c6| c6.with_markers(&m, base).0);
        axial = Some(AxialRecord::from(&split));
    }
    let coeffs = BodyCoeffs::from_su2(&su2, setup.dims.length);
    // Half domain: the symmetry plane makes the side force and the roll / yaw moments meaningless (zero by symmetry).
    let coeffs6 = su26.map(|c| BodyCoeffs6::from_su2(&c)).map(|c| if setup.opt.symmetry { c.half_model() } else { c });
    if coeffs6.is_none() && (spec.is_roll() || setup.opt.general_table()) {
        return Err("[IY-F112] the history has no CFx, CFy, CFz, CMx, CMy, CMz columns (needed for sideslip, roll and general tables)".into());
    }
    Ok((s, coeffs, coeffs6, axial))
}

/// Warm start of a case: the neighbour's restart file and, for RANS, the reference peak of its chain.
#[derive(Debug, Clone)]
struct WarmStart {
    restart: PathBuf,
    reference: Option<ResidualPeak>,
}

/// RANS acceptance: residual drops below the cold reference peak (orders).
pub const RANS_RHO_DROP: f64 = 3.0;
pub const RANS_NU_DROP: f64 = 2.0;
/// RANS limit cycle: density residual drop below the cold reference peak (orders).
pub const RANS_CYCLE_RHO_DROP: f64 = 2.0;
/// RANS y+ post-check: largest accepted 95th percentile of the wall y+ (resolved wall).
pub const RANS_YPLUS_P95_MAX: f64 = 3.0;
/// Body length / reference length when the mesh parameters do not give the body length.
const FALLBACK_LENGTH_RATIO: f64 = 15.0;

/// The reference peak a RANS final stage is judged against: the cold reference, raised to the run's own peaks
/// when those are higher (a warm start that jumps above its reference is judged against its own peak).
pub fn effective_reference(reference: ResidualPeak, h: &History) -> ResidualPeak {
    ResidualPeak { rho: reference.rho.max(h.residual_peak().unwrap_or(f64::NEG_INFINITY)), nu: reference.nu.max(h.nu_peak().unwrap_or(f64::NEG_INFINITY)) }
}

/// Outcome of the RANS acceptance rule.
#[derive(Debug, Clone, PartialEq)]
pub enum RansVerdict {
    /// Density residual [`RANS_RHO_DROP`] and SA residual [`RANS_NU_DROP`] orders below the reference peak, the
    /// coefficients flat ([`History::rans_coefficients_stable`]) and their extrapolated remaining change within the
    /// same tolerances ([`History::rans_trend_settled`]).
    Converged,
    /// Density residual [`RANS_CYCLE_RHO_DROP`] orders below the reference and the coefficients oscillating
    /// within [`History::limit_cycle`]: accepted, listed as unconverged, coefficients averaged over the window.
    LimitCycle,
    /// The residual has stalled ([`RANS_STALL_RHO_DROP`] orders or more below the reference, its median over the
    /// last window no lower than over the window before) while the coefficients are stationary over both windows:
    /// flat and without a trend extrapolated over the iteration budget. Accepted, listed as unconverged, with the
    /// spread of the forces (the text).
    Stalled(String),
    /// Not acceptable (the reason).
    Rejected(String),
}

/// RANS acceptance: the residual drops are measured from the largest residual of this many last rows.
pub const RANS_RESIDUAL_ROWS: usize = 20;
/// RANS stalled residual: least density-residual drop below the reference (orders, median of the window).
pub const RANS_STALL_RHO_DROP: f64 = 0.5;
/// RANS stalled residual: the median over the last window is not this much lower than over the window before.
pub const RANS_STALL_PROGRESS: f64 = 0.2;

/// Median of the column `names` over rows `from..to` of `h`.
fn median_rows(h: &History, names: &[&str], from: usize, to: usize) -> Option<f64> {
    let c = h.col(names)?;
    let mut v: Vec<f64> = h.rows[from..to].iter().map(|r| r[c]).filter(|x| x.is_finite()).collect();
    if v.is_empty() {
        return None;
    }
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    Some(v[v.len() / 2])
}

/// "`name` mean (inter-quartile range ...)" of the finite `values` (`None` when there is none).
fn spread_text(name: &str, values: impl Iterator<Item = f64>) -> Option<String> {
    let mut v: Vec<f64> = values.filter(|x| x.is_finite()).collect();
    if v.is_empty() {
        return None;
    }
    v.sort_by(f64::total_cmp);
    let mean = v.iter().sum::<f64>() / v.len() as f64;
    Some(format!("{name} {mean:.5} (inter-quartile range {:.2e})", v[3 * v.len() / 4] - v[v.len() / 4]))
}

/// Density residual of a stalled RANS run: the drop of the median over the last `window` rows below the reference
/// (orders) and the progress of that median from the window before (`None` with fewer than two windows of rows).
fn rans_stall_levels(h: &History, reference: ResidualPeak, window: usize) -> Option<(f64, f64)> {
    let n = h.rows.len();
    if window < 2 || n < 2 * window {
        return None;
    }
    let rho = ["rms[Rho]", "Res_Flow[0]", "rms[P]"];
    let now = median_rows(h, &rho, n - window, n)?;
    let before = median_rows(h, &rho, n - 2 * window, n - window)?;
    Some((effective_reference(reference, h).rho - now, before - now))
}

/// Density-residual drop of a RANS run below the reference (orders), from the largest residual of the last
/// [`RANS_RESIDUAL_ROWS`] rows: the drop [`RansVerdict::Converged`] and [`RansVerdict::LimitCycle`] are judged on.
pub fn rans_density_drop(h: &History, reference: ResidualPeak) -> Option<f64> {
    rans_worst_drop(h, effective_reference(reference, h).rho, &["rms[Rho]", "Res_Flow[0]", "rms[P]"])
}

/// `peak` minus the largest value of the column `names` over the last [`RANS_RESIDUAL_ROWS`] rows: a dip of a
/// chattering residual (e.g. while the CFL is cut) is no drop.
fn rans_worst_drop(h: &History, peak: f64, names: &[&str]) -> Option<f64> {
    let c = h.col(names)?;
    let v = h.rows[h.rows.len().saturating_sub(RANS_RESIDUAL_ROWS)..].iter().map(|r| r[c]).fold(f64::NEG_INFINITY, f64::max);
    v.is_finite().then_some(peak - v)
}

/// The density-residual drop the verdict of a RANS case rests on: the median drop of [`RansVerdict::Stalled`], else
/// [`rans_density_drop`].
pub fn rans_verdict_drop(h: &History, reference: ResidualPeak, window: usize, verdict: &RansVerdict) -> Option<f64> {
    match verdict {
        RansVerdict::Stalled(_) => rans_stall_levels(h, reference, window).map(|(d, _)| d),
        _ => rans_density_drop(h, reference),
    }
}

/// [`RansVerdict::Stalled`] check; the text gives the residual level and the spread of the forces.
fn rans_stalled(h: &History, reference: ResidualPeak, window: usize, kind: StabilityKind, length_ratio: f64, budget: usize) -> Option<String> {
    let n = h.rows.len();
    let (drop, progress) = rans_stall_levels(h, reference, window)?;
    if drop < RANS_STALL_RHO_DROP || progress > RANS_STALL_PROGRESS {
        return None;
    }
    if !(h.rans_coefficients_stable(2 * window, kind, length_ratio) && h.rans_trend_settled(2 * window, kind, length_ratio, budget)) {
        return None;
    }
    let spread = |names: &[&str]| -> Option<String> {
        let c = h.col(names)?;
        spread_text(names[0], h.rows[n - 2 * window..].iter().map(|r| r[c]))
    };
    let forces: Vec<String> = [&["CFx", "CD"][..], &["CFz", "CL"][..]].iter().filter_map(|c| spread(c)).collect();
    Some(format!(
        "residual stalled {drop:.1} orders below the reference (median of the last {window} iterations), coefficients stationary over {} iterations: {}",
        2 * window,
        forces.join(", ")
    ))
}

/// Judges the final-stage history `h` of a RANS case against `reference` over the last `window` rows; `budget` is
/// the iteration limit of the stage (the longest trend extrapolation, see [`History::rans_trend_settled`]).
pub fn rans_verdict(h: &History, reference: ResidualPeak, window: usize, kind: StabilityKind, length_ratio: f64, budget: usize) -> RansVerdict {
    let r = effective_reference(reference, h);
    let rho = rans_density_drop(h, reference);
    let nu = rans_worst_drop(h, r.nu, &["rms[nu]", "rms[Nu_Tilde]", "rms[nu_tilde]"]);
    let flat = h.rans_coefficients_stable(window, kind, length_ratio);
    let stable = flat && h.rans_trend_settled(window, kind, length_ratio, budget);
    if rho.is_some_and(|d| d >= RANS_RHO_DROP) && nu.is_some_and(|d| d >= RANS_NU_DROP) && stable {
        return RansVerdict::Converged;
    }
    if let Some(why) = rans_stalled(h, reference, window, kind, length_ratio, budget) {
        return RansVerdict::Stalled(why);
    }
    if rho.is_some_and(|d| d >= RANS_CYCLE_RHO_DROP) && h.limit_cycle(window, kind, length_ratio) {
        return RansVerdict::LimitCycle;
    }
    RansVerdict::Rejected(format!(
        "density residual {:.1} and SA residual {:.1} orders below the cold reference peak (need {RANS_RHO_DROP} and {RANS_NU_DROP}), coefficients {}",
        rho.unwrap_or(f64::NAN),
        nu.unwrap_or(f64::NAN),
        if stable {
            "stable"
        } else if flat {
            "flat but still trending"
        } else if h.limit_cycle(window, kind, length_ratio) {
            "in a limit cycle"
        } else {
            "still changing"
        }
    ))
}

/// Body length / reference length: the body length from the mesh parameters (`params.json` next to the mesh).
fn length_ratio(setup: &RunSetup) -> f64 {
    let p = setup.mesh_file.with_file_name("params.json");
    std::fs::read_to_string(p)
        .ok()
        .and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok())
        .and_then(|v| v.get("length").and_then(|l| l.as_f64()))
        .filter(|l| *l > 0.0 && setup.dims.length > 0.0)
        .map_or(FALLBACK_LENGTH_RATIO, |l| l / setup.dims.length)
}

/// Whether an SU2 restart file carries the SA variable (an Euler restart cannot seed a RANS case).
fn restart_has_turbulence(path: &Path) -> bool {
    use std::io::Read;
    let mut head = vec![0u8; 1 << 16];
    let n = std::fs::File::open(path).and_then(|mut f| f.read(&mut head)).unwrap_or(0);
    head[..n].windows(8).any(|w| w == b"Nu_Tilde")
}

/// 95th percentile of `Y_Plus` over the surface points of SU2's legacy-VTK surface file, without the base cap
/// (the points at the largest x: the flat base at `x = L`).
pub fn wall_yplus_p95(vtk: &str) -> Option<f64> {
    let mut lines = vtk.lines();
    let n: usize = lines.by_ref().find(|l| l.starts_with("POINTS"))?.split_whitespace().nth(1)?.parse().ok()?;
    let mut xyz = Vec::with_capacity(3 * n);
    for l in lines.by_ref() {
        xyz.extend(l.split_whitespace().filter_map(|t| t.parse::<f64>().ok()));
        if xyz.len() >= 3 * n {
            break;
        }
    }
    lines.by_ref().find(|l| l.split_whitespace().take(2).eq(["SCALARS", "Y_Plus"]))?;
    let mut y = Vec::with_capacity(n);
    for l in lines {
        if l.starts_with("LOOKUP_TABLE") {
            continue;
        }
        y.extend(l.split_whitespace().filter_map(|t| t.parse::<f64>().ok()));
        if y.len() >= n {
            break;
        }
    }
    if xyz.len() < 3 * n || y.len() < n || n == 0 {
        return None;
    }
    let x: Vec<f64> = (0..n).map(|k| xyz[3 * k]).collect();
    let (lo, hi) = x.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), &v| (a.min(v), b.max(v)));
    let cut = hi - 1e-6 * (hi - lo).max(1e-12);
    let mut v: Vec<f64> = (0..n).filter(|&k| x[k] < cut && y[k].is_finite()).map(|k| y[k]).collect();
    if v.is_empty() {
        return None;
    }
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    Some(v[((0.95 * v.len() as f64).ceil() as usize).clamp(1, v.len()) - 1])
}

/// Signal that killed SU2: from the exit status (a signal, or the shell convention `128 + n`) or from the MPI
/// launcher's report in the log ("... exited on signal 9 ...").
pub fn killing_signal(status: &std::process::ExitStatus, log: &str) -> Option<i32> {
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        if let Some(sig) = status.signal() {
            return Some(sig);
        }
    }
    if let Some(c) = status.code().filter(|c| (129..=159).contains(c)) {
        return Some(c - 128);
    }
    log.lines().filter(|l| l.contains("on signal")).find_map(|l| {
        let rest = &l[l.find("on signal")? + "on signal".len()..];
        rest.split(|ch: char| !ch.is_ascii_digit()).find(|t| !t.is_empty())?.parse().ok()
    })
}

/// Estimated memory of one case [bytes] from the mesh statistics next to the mesh (`mesh_stats.json`).
fn case_memory_estimate(setup: &RunSetup) -> Option<u64> {
    let t = std::fs::read_to_string(setup.mesh_file.with_file_name("mesh_stats.json")).ok()?;
    let v: serde_json::Value = serde_json::from_str(&t).ok()?;
    let cells = v.get("cells")?.as_u64()? as usize;
    let nodes = v.get("nodes").and_then(|n| n.as_u64()).unwrap_or(0) as usize;
    Some(crate::case_memory_bytes_mesh(setup.opt.model, cells, nodes, setup.ranks.max(1)))
}

/// Failure reason of an SU2 run that exited unsuccessfully (`what`: "SU2" or "SU2 (first-order startup)"). A kill by a
/// signal is named, SIGKILL with the out-of-memory hint.
fn exit_message(setup: &RunSetup, what: &str, st: &std::process::ExitStatus, log: &Path) -> String {
    let text = std::fs::read_to_string(log).unwrap_or_default();
    let name = log.file_name().map_or(String::new(), |n| n.to_string_lossy().into_owned());
    match killing_signal(st, &text) {
        Some(9) => format!(
            "[IY-F120] {what} was killed by signal 9 (SIGKILL; most likely out of memory: about {} per case with {} ranks and {} concurrent cases; \
             lower parallel_cases or ranks_per_case); end of {name}:\n{}",
            case_memory_estimate(setup).map_or("? GB".to_string(), |b| format!("{:.1} GB", b as f64 / 1e9)),
            setup.ranks,
            setup.parallel,
            tail(log, 8)
        ),
        Some(sig) => format!("[IY-F121] {what} was killed by signal {sig}; end of {name}:\n{}", tail(log, 8)),
        None => format!("[IY-F122] {what} exited with {st}; end of {name}:\n{}", tail(log, 8)),
    }
}

/// SU2 stopped because the residual exceeded its divergence limit or became non-finite.
fn su2_aborted_on_divergence(log: &Path) -> bool {
    let t = std::fs::read_to_string(log).unwrap_or_default();
    t.contains("SU2 has diverged") || t.contains("Residual > 10^20")
}

/// Relative path of a sibling case's restart file (SU2 aborts with "buffer overflow detected" on long paths).
fn restart_path(dir: &Path, p: &Path) -> String {
    match p.parent().and_then(|d| d.file_name()) {
        Some(case) if p.parent().and_then(|d| d.parent()) == dir.parent() => format!("../{}/restart.dat", case.to_string_lossy()),
        _ => p.display().to_string(),
    }
}

const CASE_FILES: [&str; 7] =
    ["history.csv", "history_startup.csv", "done.json", "surface_flow.csv", "surface_flow.vtk", "restart.dat", crate::su2cfg::STARTUP_RESTART];

/// Runs one case to completion (or failure). Returns `Err` only when cancelled.
fn run_case(setup: &RunSetup, i: usize, spec: CaseSpec, hash: &str, warm: Option<WarmStart>, tx: &mpsc::Sender<Event>) -> Result<CaseOutcome> {
    let dir = spec.dir(&setup.work);
    std::fs::create_dir_all(&dir)?;
    for f in CASE_FILES {
        let _ = std::fs::remove_file(dir.join(f));
    }
    if setup.opt.model == crate::config::FlowModel::Rans {
        return run_rans_case(setup, i, spec, hash, warm, tx);
    }
    let warm_flag = warm.is_some();
    let kind = StabilityKind::of(&spec);
    let fail = |msg: String| CaseOutcome { spec, result: None, error: Some(msg), cached: false, warm: warm_flag };
    let files = CaseFiles {
        mesh: setup.mesh_file.display().to_string(),
        restart_from: warm.map(|w| restart_path(&dir, &w.restart)),
        stage: Stage::Final,
        conservative: false,
    };
    let t0 = Instant::now();
    let deadline = (setup.opt.timeout_minutes > 0.0).then(|| t0 + Duration::from_secs_f64(setup.opt.timeout_minutes * 60.0));
    let min_drop = 3.0;
    let minval = setup.opt.residual_minval();
    let ratio = length_ratio(setup);
    let budget = setup.opt.iterations;
    // SU2 runs to a floor far below the target (see `su2cfg`); the runner stops it, after its next periodic output
    // (the restart file of the warm starts), when
    // * the residual reached the target and the coefficient trend has settled ([`History::euler_trend_settled`]), or
    // * practically converged (typical for Euler wake limit cycles): residual down 3 orders, the coefficients flat over
    //   `stable_window` iterations and their trend settled.
    // The stop decision: rows of the history then and whether the target was reached (see `judged_at_stop`).
    let decision: std::cell::Cell<Option<(usize, bool)>> = std::cell::Cell::new(None);
    let stop = |h: &History| {
        let w = setup.opt.stable_window;
        let settled = || h.euler_trend_settled(w, kind, ratio, budget);
        let reached = h.last_residual().is_some_and(|r| r <= minval) && h.rows.len() >= w;
        let practical =
            h.rows.len() >= setup.opt.min_iterations && h.residual_drop_from_peak().is_some_and(|d| d >= min_drop) && h.coefficients_stable(w, kind);
        if (reached || practical) && settled() {
            decision.set(Some((h.rows.len(), reached)));
            StopAction::AfterOutput(2 * crate::su2cfg::OUTPUT_FREQ + 5)
        } else {
            StopAction::Continue
        }
    };
    let cfg = su2_config_case(setup.opt, &spec, &setup.dims, &setup.atm, &files);
    let run = match solve(setup, i, &dir, ("case.cfg", "su2.log", "history.csv"), &cfg, deadline, &stop, tx)? {
        Ok(r) => r,
        Err(e) => return Ok(fail(e)),
    };
    let wall = t0.elapsed().as_secs_f64();
    let log = dir.join("su2.log");
    if let Some(st) = run.status {
        if !st.success() {
            return Ok(fail(exit_message(setup, "SU2", &st, &log)));
        }
    }
    note_multigrid(setup.opt, &log);
    let h = match History::read(&dir.join("history.csv")) {
        Ok(h) if !h.rows.is_empty() => h,
        _ => return Ok(fail(format!("[IY-F113] SU2 produced no history.csv; end of su2.log:\n{}", tail(&log, 8)))),
    };
    let full = h;
    // Stopped by the runner: accepted as decided (target reached, or practically converged), settled.
    let (h, stopped_reached) = judged_at_stop(&full, decision.get().map(|(rows, r)| (rows, Some(r))), |_| None);
    let n = h.rows.len();
    let window = (n / 4).clamp(2, 100);
    let (reached, settled, accepted) = match stopped_reached {
        Some(r) => (r, true, !r),
        None => {
            let reached = h.last_residual().is_some_and(|r| r <= minval);
            let settled = h.euler_trend_settled(setup.opt.stable_window.min(n).max(2), kind, ratio, budget);
            (reached, settled, !reached && h.residual_drop_from_peak().is_some_and(|d| d >= min_drop) && h.coefficients_stable(window, kind))
        }
    };
    let mut warnings = Vec::new();
    if (reached || accepted) && !settled {
        warnings.push(format!(
            "[IY-F524] the coefficients still trend or oscillate at the end ({} iterations): their range over the last \
             stable_window iterations or their change extrapolated from the residual decay exceeds {} % of the force scale \
             (centre of pressure: {} body lengths); coefficients averaged over the last {window} iterations",
            n,
            100.0 * crate::history::EULER_TREND_FORCE,
            crate::history::EULER_TREND_XCP
        ));
    }
    if !reached && !accepted {
        let why = if run.timed_out { "timeout" } else { "iteration limit" };
        return Ok(fail(format!(
            "[IY-F119] not converged ({why}): residual {:.2} (target {minval}), drop {:.1} orders, coefficients {}",
            h.last_residual().unwrap_or(f64::NAN),
            h.residual_drop_from_peak().unwrap_or(f64::NAN),
            if h.coefficients_stable(window, kind) { "stable" } else { "still changing" }
        )));
    }
    let (s, coeffs, coeffs6, axial) = match coefficients_from_history(setup, &spec, &h, if reached && settled { 1 } else { window }) {
        Ok(c) => c,
        Err(e) => return Ok(fail(e)),
    };
    let first = h.col(&["rms[Rho]", "Res_Flow[0]", "rms[P]"]).map(|c| h.rows[0][c]);
    let result = CaseResult {
        input_hash: hash.to_string(),
        mach: spec.mach,
        alpha_deg: spec.alpha_deg,
        beta_deg: spec.beta_deg,
        roll_hat: spec.roll_hat,
        coeffs,
        coeffs6,
        iterations: summarize(&full, 1).map_or(s.iterations, |f| f.iterations),
        converged: reached && settled,
        residual_first: first,
        residual_last: h.last_residual(),
        wall_seconds: wall,
        reference_peak: None,
        limit_cycle: false,
        stalled: false,
        residual_drop: None,
        yplus_p95: None,
        axial,
        warnings,
    };
    for w in &result.warnings {
        eprintln!("{}", crate::coded_line("warning", &format!("aero.cfd: case {}:", spec.dir_name()), w));
    }
    write_done(&dir, &result)?;
    Ok(CaseOutcome { spec, result: Some(result), error: None, cached: false, warm: warm_flag })
}

/// RANS case: cold start in two stages (first-order startup, then second order restarted from it) or, with a warm
/// start, the second-order stage only; a failed warm start is repeated cold. Judged by [`rans_verdict`].
fn run_rans_case(setup: &RunSetup, i: usize, spec: CaseSpec, hash: &str, warm: Option<WarmStart>, tx: &mpsc::Sender<Event>) -> Result<CaseOutcome> {
    let dir = spec.dir(&setup.work);
    let opt = setup.opt;
    let kind = StabilityKind::of(&spec);
    let ratio = length_ratio(setup);
    let mesh = setup.mesh_file.display().to_string();
    let t0 = Instant::now();
    let deadline = (opt.timeout_minutes > 0.0).then(|| t0 + Duration::from_secs_f64(opt.timeout_minutes * 60.0));
    // Only a RANS restart with a known reference can seed the case (never an Euler solution).
    let mut warm = warm.and_then(|w| Some((w.reference?, w.restart))).filter(|(_, p)| restart_has_turbulence(p));
    let mut notes: Vec<String> = Vec::new();
    // Cold start whose startup result can be reused (the conservative retry after a divergence).
    let mut startup: Option<ResidualPeak> = None;
    // Conservative settings: after a divergence of the startup (then for both stages) or of the second-order stage.
    let mut conservative = false;
    let mut startup_was_conservative = false;
    loop {
        let is_warm = warm.is_some();
        let fail = |msg: String| CaseOutcome { spec, result: None, error: Some(msg), cached: false, warm: is_warm };
        let (reference, restart_from) = match (warm.take(), startup) {
            (Some((r, p)), _) => (r, restart_path(&dir, &p)),
            (None, Some(r)) => (r, STARTUP_RESTART.to_string()),
            (None, None) => {
                for f in CASE_FILES {
                    let _ = std::fs::remove_file(dir.join(f));
                }
                let files = CaseFiles { mesh: mesh.clone(), restart_from: None, stage: Stage::Startup, conservative };
                let cfg = su2_config_case(opt, &spec, &setup.dims, &setup.atm, &files);
                let names = ("case_startup.cfg", "su2_startup.log", "history_startup.csv");
                let stop = |h: &History| if h.rans_divergence().is_some() { StopAction::Now } else { StopAction::Continue };
                let run = match solve(setup, i, &dir, names, &cfg, deadline, &stop, tx)? {
                    Ok(r) => r,
                    Err(e) => return Ok(fail(e)),
                };
                let log = dir.join("su2_startup.log");
                // Diverged: stopped by the runner, or SU2 aborted on its own ("Residual > 10^20", non-finite values).
                let killed = run.status.as_ref().is_some_and(|st| killing_signal(st, &std::fs::read_to_string(&log).unwrap_or_default()).is_some());
                let diverged = History::read(&dir.join("history_startup.csv")).ok().and_then(|h| h.rans_divergence()).or_else(|| {
                    (run.status.is_some_and(|st| !st.success()) && !killed && su2_aborted_on_divergence(&log))
                        .then(|| "SU2 stopped on a residual above 1e20".to_string())
                });
                if let Some(why) = diverged {
                    if !conservative {
                        notes.push(format!(
                            "[IY-F526] the first-order startup diverged ({why}); repeated once with the adaptive CFL capped at {} instead of {} \
                             (the SA equation stays at {} of the flow CFL); the second-order stage keeps these settings",
                            crate::su2cfg::RANS_CONSERVATIVE_CFL_FACTOR * opt.cfl,
                            crate::su2cfg::RANS_CFL_MAX_FACTOR * opt.cfl,
                            crate::su2cfg::RANS_CONSERVATIVE_TURB_CFL
                        ));
                        conservative = true;
                        continue;
                    }
                    return Ok(fail(format!("[IY-F114] the first-order startup diverged twice (the second time with conservative settings): {why}")));
                }
                if let Some(st) = run.status.filter(|s| !s.success()) {
                    return Ok(fail(exit_message(setup, "SU2 (first-order startup)", &st, &log)));
                }
                if run.timed_out {
                    return Ok(fail("[IY-F119] not converged (timeout during the first-order startup)".into()));
                }
                let h = History::read(&dir.join("history_startup.csv")).ok().filter(|h| !h.rows.is_empty());
                let Some(r) = h.as_ref().and_then(|h| Some(ResidualPeak { rho: h.residual_peak()?, nu: h.nu_peak()? })) else {
                    return Ok(fail(format!(
                        "[IY-F115] the first-order startup produced no density and SA residuals; end of su2_startup.log:\n{}",
                        tail(&log, 8)
                    )));
                };
                if !dir.join(STARTUP_RESTART).exists() {
                    return Ok(fail(format!("[IY-F116] the first-order startup wrote no {STARTUP_RESTART}")));
                }
                startup = Some(r);
                startup_was_conservative = conservative;
                (r, STARTUP_RESTART.to_string())
            }
        };
        let files = CaseFiles { mesh: mesh.clone(), restart_from: Some(restart_from), stage: Stage::Final, conservative };
        let cfg = su2_config_case(opt, &spec, &setup.dims, &setup.atm, &files);
        // Stop when converged, or early when the solution diverges (instead of running into the iteration limit). The
        // decision (rows then, verdict) is what the case is judged on (see `judged_at_stop`).
        let decision: std::cell::RefCell<Option<(usize, RansVerdict)>> = std::cell::RefCell::new(None);
        let stop = |h: &History| {
            if h.rans_divergence().is_some() {
                return StopAction::Now;
            }
            if h.rows.len() >= opt.min_iterations {
                let v = rans_verdict(h, reference, opt.stable_window, kind, ratio, opt.iterations);
                if matches!(v, RansVerdict::Converged | RansVerdict::Stalled(_)) {
                    *decision.borrow_mut() = Some((h.rows.len(), v));
                    return StopAction::AfterOutput(2 * crate::su2cfg::OUTPUT_FREQ + 5);
                }
            }
            StopAction::Continue
        };
        let run = match solve(setup, i, &dir, ("case.cfg", "su2.log", "history.csv"), &cfg, deadline, &stop, tx)? {
            Ok(r) => r,
            Err(e) => return Ok(fail(e)),
        };
        let log = dir.join("su2.log");
        let retry = |why: String, notes: &mut Vec<String>| {
            notes.push(format!("[IY-F525] the warm start failed ({why}); solved again from the freestream"));
        };
        if let Some(why) = History::read(&dir.join("history.csv")).ok().and_then(|h| h.rans_divergence()) {
            let _ = std::fs::remove_file(dir.join("restart.dat"));
            if is_warm {
                retry(format!("diverged: {why}"), &mut notes);
                continue;
            }
            if !conservative {
                notes.push(format!(
                    "[IY-F527] the second-order stage diverged ({why}); repeated once from the first-order solution with the adaptive CFL capped at {} \
                     instead of {} and the SA equation at {} of the flow CFL instead of 1",
                    crate::su2cfg::RANS_CONSERVATIVE_CFL_FACTOR * opt.cfl,
                    crate::su2cfg::RANS_CFL_MAX_FACTOR * opt.cfl,
                    crate::su2cfg::RANS_CONSERVATIVE_TURB_CFL
                ));
                conservative = true;
                continue;
            }
            if startup_was_conservative {
                return Ok(fail(format!(
                    "[IY-F117] the second-order stage diverged with the conservative settings of the repeated first-order startup: {why}"
                )));
            }
            return Ok(fail(format!("[IY-F118] diverged twice (the second time with conservative settings): {why}")));
        }
        if let Some(st) = run.status.filter(|s| !s.success()) {
            let msg = exit_message(setup, "SU2", &st, &log);
            if is_warm && !run.timed_out && killing_signal(&st, &std::fs::read_to_string(&log).unwrap_or_default()).is_none() {
                retry(format!("SU2 exited with {st}"), &mut notes);
                continue;
            }
            return Ok(fail(msg));
        }
        note_multigrid(opt, &log);
        let full = match History::read(&dir.join("history.csv")) {
            Ok(h) if !h.rows.is_empty() => h,
            _ => return Ok(fail(format!("[IY-F113] SU2 produced no history.csv; end of su2.log:\n{}", tail(&log, 8)))),
        };
        let judge = |h: &History| rans_verdict(h, reference, opt.stable_window.min(h.rows.len()).max(2), kind, ratio, opt.iterations);
        let (h, verdict) = judged_at_stop(&full, decision.take(), judge);
        let window = opt.stable_window.min(h.rows.len()).max(2);
        let drop = rans_verdict_drop(&h, reference, window, &verdict);
        let mut stalled: Option<String> = None;
        let limit_cycle = match verdict {
            RansVerdict::Converged => false,
            RansVerdict::LimitCycle => true,
            RansVerdict::Stalled(why) => {
                stalled = Some(why);
                true
            }
            RansVerdict::Rejected(why) => {
                if is_warm && !run.timed_out {
                    retry(why, &mut notes);
                    continue;
                }
                let when = if run.timed_out { "timeout" } else { "iteration limit" };
                return Ok(fail(format!("[IY-F119] not converged ({when}): {why}")));
            }
        };
        // Stalled: averaged over both windows of the check.
        let avg = if stalled.is_some() { (2 * window).min(h.rows.len()) } else { window };
        let (s, coeffs, coeffs6, axial) = match coefficients_from_history(setup, &spec, &h, if limit_cycle { avg } else { 1 }) {
            Ok(c) => c,
            Err(e) => return Ok(fail(e)),
        };
        let mut warnings = notes.clone();
        if let Some(why) = &stalled {
            warnings.push(format!("[IY-F520] unconverged: {why}; coefficients averaged over the last {avg} iterations"));
        } else if limit_cycle {
            warnings.push(format!("[IY-F521] limit cycle: coefficients averaged over the last {window} iterations"));
        }
        let yplus = std::fs::read_to_string(dir.join("surface_flow.vtk")).ok().as_deref().and_then(wall_yplus_p95);
        match yplus {
            Some(y) if !opt.wall_function() && y > RANS_YPLUS_P95_MAX => warnings.push(format!(
                "[IY-F522] wall y+ P95 = {y:.2} exceeds {RANS_YPLUS_P95_MAX} (target yplus = {}): the first cell is too thick for the resolved wall",
                opt.yplus
            )),
            Some(_) => {}
            None => warnings.push("[IY-F523] wall y+ not checked: no Y_Plus in surface_flow.vtk".into()),
        }
        for w in &warnings {
            eprintln!("{}", crate::coded_line("warning", &format!("aero.cfd: case {}:", spec.dir_name()), w));
        }
        let result = CaseResult {
            input_hash: hash.to_string(),
            mach: spec.mach,
            alpha_deg: spec.alpha_deg,
            beta_deg: spec.beta_deg,
            roll_hat: spec.roll_hat,
            coeffs,
            coeffs6,
            iterations: summarize(&full, 1).map_or(s.iterations, |f| f.iterations),
            converged: !limit_cycle,
            residual_first: h.col(&["rms[Rho]", "Res_Flow[0]", "rms[P]"]).map(|c| h.rows[0][c]),
            residual_last: h.last_residual(),
            wall_seconds: t0.elapsed().as_secs_f64(),
            reference_peak: Some(effective_reference(reference, &h)),
            limit_cycle,
            stalled: stalled.is_some(),
            residual_drop: drop,
            yplus_p95: yplus,
            axial,
            warnings,
        };
        write_done(&dir, &result)?;
        let _ = std::fs::remove_file(dir.join(STARTUP_RESTART));
        return Ok(CaseOutcome { spec, result: Some(result), error: None, cached: false, warm: is_warm });
    }
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
    if !cfg!(unix) {
        bail!("[IY-F123] the CFD mode needs a Unix-like system (Linux, macOS or WSL)");
    }
    let cases = case_list(setup.opt);
    let n = cases.len();
    let _sig = SignalGuard::new();
    std::fs::create_dir_all(&setup.work)?;
    let hashes: Vec<String> = cases.iter().map(|c| case_hash(&hash_text(setup, c), &setup.mesh_hash)).collect();
    let sources: Vec<Option<usize>> = (0..n).map(|i| warm_start_source_for(&cases, i, setup.opt.model != FlowModel::Rans)).collect();
    let mut outcomes: Vec<Option<CaseOutcome>> = vec![None; n];
    let mut state = vec![St::Pending; n];
    for i in 0..n {
        // Results stored before the six coefficients existed cannot feed a general table or the roll pair.
        let need6 = cases[i].is_roll() || setup.opt.general_table();
        if let Some(r) = read_done(&cases[i].dir(&setup.work), &hashes[i]).filter(|r| !need6 || r.coeffs6.is_some()) {
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
                        .and_then(|s| outcomes[s].as_ref().and_then(|o| o.result.as_ref()).map(|r| (s, r.reference_peak)))
                        .map(|(s, reference)| WarmStart { restart: cases[s].dir(&setup.work).join("restart.dat"), reference })
                        .filter(|w| w.restart.exists());
                    progress(CfdStage::Case { index: i, total: n, mach: cases[i].mach, alpha: cases[i].alpha_deg, iter: 0, residual: 0.0 });
                    let tx = tx.clone();
                    let (spec, hash) = (cases[i], hashes[i].clone());
                    sc.spawn(move || {
                        let r = run_case(setup, i, spec, &hash, restart, &tx);
                        let outcome = r.unwrap_or_else(|e| CaseOutcome { spec, result: None, error: Some(e.to_string()), cached: false, warm: false });
                        let _ = tx.send(Event::Finished { i, outcome: Box::new(outcome) });
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
                        error = Some(anyhow!("[IY-F111] interrupted"));
                    } else {
                        let conv = outcome.result.as_ref().is_some_and(|r| r.converged);
                        progress(CfdStage::CaseDone { index: i, total: n, mach: cases[i].mach, alpha: cases[i].alpha_deg, converged: conv });
                    }
                    outcomes[i] = Some(*outcome);
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
        bail!("[IY-F111] interrupted");
    }
    let outcomes: Vec<CaseOutcome> = outcomes.into_iter().map(|o| o.expect("every case has an outcome")).collect();
    for line in failure_summary(&outcomes) {
        eprintln!("{line}");
    }
    Ok(outcomes)
}

/// One line per failed case with its reason (first line of the error), printed at the end of a run.
pub fn failure_summary(outcomes: &[CaseOutcome]) -> Vec<String> {
    outcomes
        .iter()
        .filter(|o| o.result.is_none())
        .map(|o| {
            let why = o.error.as_deref().unwrap_or("unknown reason").lines().next().unwrap_or("");
            crate::coded_line("error", &format!("aero.cfd: case {} failed:", o.spec.dir_name()), why)
        })
        .collect()
}

/// `rans_verdict` column of `cfd_cases.csv`: `converged`, `limit_cycle` or `stalled` for a RANS result, empty otherwise.
fn rans_verdict_name(r: Option<&CaseResult>) -> &'static str {
    match r {
        Some(r) if r.reference_peak.is_none() => "",
        Some(r) if r.stalled => "stalled",
        Some(r) if r.limit_cycle => "limit_cycle",
        Some(_) => "converged",
        None => "",
    }
}

/// Writes `cfd_cases.csv`: every solved case (raw, including the negative angles of `mirror` mode)
/// with its convergence information, followed in `mirror` mode by the combined odd/even rows
/// (`status = combined`, `corrected` are the points of [`crate::table::apply_alpha_mode`]).
/// `cn`, `nose_moment`, `xcp_m` are the values the table uses: offset-corrected in `offset` mode,
/// equal to the raw ones in `single` mode, empty on raw `mirror` rows. `residual_first` / `residual_last` are the
/// first density residual and the one at the judged row (the stop decision, else the last row) of the (RANS: second-order) history; RANS rows add the reference peak of the
/// acceptance (`residual_reference`), the drop below it the verdict rests on (`residual_drop`) and the verdict
/// (`rans_verdict`: `converged`, `limit_cycle`, `stalled`), which are empty for Euler.
pub fn write_cases_csv(path: &Path, outcomes: &[CaseOutcome], corrected: &[SolvedPoint], mode: AlphaMode) -> Result<()> {
    use std::fmt::Write;
    let mut s = String::from(
        "mach,alpha_deg,status,iterations,residual_first,residual_last,cn_raw,ca_cfd,nose_moment_raw,xcp_raw_m,cn,nose_moment,xcp_m,wall_seconds,warm_start,beta_deg,roll_hat,cx,cy,cz,cl,cm,cn_yaw,\
         residual_reference,residual_drop,rans_verdict\n",
    );
    let f = |v: Option<f64>| v.map_or(String::new(), |x| format!("{x:.8e}"));
    let corr = |o: &CaseOutcome| {
        if o.spec.beta_deg != 0.0 || o.spec.is_roll() {
            return None;
        }
        corrected.iter().find(|p| p.mach == o.spec.mach && p.alpha_deg == o.spec.alpha_deg).and_then(|p| p.coeffs)
    };
    let used = |c: &BodyCoeffs| (Some(c.cn), Some(c.mom), c.xcp());
    for o in outcomes {
        let status = match &o.result {
            Some(r) if r.converged => "converged",
            Some(r) if r.limit_cycle => "unconverged",
            Some(_) => "accepted",
            None => "failed",
        };
        let r = o.result.as_ref();
        let c6 = r.and_then(|r| r.coeffs6);
        let (cn, mom, xcp) = if mode == AlphaMode::Mirror { (None, None, None) } else { corr(o).as_ref().map_or((None, None, None), used) };
        let _ = writeln!(
            s,
            "{},{},{},{},{},{},{},{},{},{},{},{},{},{:.1},{},{},{},{},{},{},{},{},{},{},{},{}",
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
            o.warm,
            o.spec.beta_deg,
            o.spec.roll_hat,
            f(c6.map(|c| c.cx)),
            f(c6.map(|c| c.cy)),
            f(c6.map(|c| c.cz)),
            f(c6.map(|c| c.cl)),
            f(c6.map(|c| c.cm)),
            f(c6.map(|c| c.cn)),
            f(r.and_then(|r| r.reference_peak).map(|p| p.rho)),
            f(r.and_then(|r| r.residual_drop)),
            rans_verdict_name(r)
        );
    }
    if mode == AlphaMode::Mirror {
        for p in corrected.iter().filter(|p| p.alpha_deg > 0.0) {
            let (cn, mom, xcp) = p.coeffs.as_ref().map_or((None, None, None), used);
            let _ = writeln!(s, "{},{},combined,,,,,{},,,{},{},{},,,,,,,,,,,,,", p.mach, p.alpha_deg, f(p.coeffs.map(|c| c.ca)), f(cn), f(mom), f(xcp));
        }
    }
    std::fs::write(path, s).with_context(|| format!("cannot write {}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_tmp::TempDir;
    use crate::tools::ToolInfo;
    use aero::atmosphere::AtmosphereModel;
    use std::os::unix::fs::PermissionsExt;

    #[test]
    fn multigrid_levels_are_read_from_the_su2_log() {
        let none = "Setting the multigrid structure.\nFinding max control volume width.\n";
        assert_eq!(multigrid_levels_built(none), 0);
        let two = "Setting the multigrid structure.\n+----+\n|  MG Level|       CVs|Aggl. Rate|       CFL|\n+----+\n|         0|     29545|    1/1.00|         5|\n|         1|     11308|    1/2.61|   5.44538|\n+----+\nFinding max control volume width.\n";
        assert_eq!(multigrid_levels_built(two), 2);
    }

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
case "$mode" in slow|trend|trendlimit) live=1;; *) live=0;; esac
if [ $live = 1 ]; then
  # Like SU2 8.3: the history grows row by row, the output files are written every OUTPUT_WRT_FREQ iterations,
  # and SIGTERM ends the run without writing anything.
  trap 'exit 1' TERM
  sol=$(grep '^SOLUTION_FILENAME' $cfg | cut -d' ' -f2)
  [ -n "$sol" ] && { [ -f "$sol" ] && echo "$mach $aoa $sol $(cat $sol)" || echo "$mach $aoa $sol missing"; } >> "$(dirname "$0")/solutions"
  freq=$(grep '^OUTPUT_WRT_FREQ' $cfg | cut -d' ' -f2)
  echo '"Inner_Iter","rms[Rho]","CFx","CFz","CMy"' > history.csv
  # Rows are written by shell builtins only: the runner's SIGTERM goes to the whole process group, and a child
  # killed inside a row (e.g. bc) would leave an empty field, which SU2 never writes.
  fz=$(echo "$aoa * 0.05" | bc -l); my=$(echo "0 - $aoa * 0.08" | bc -l)
  iters=$(grep '^ITER' $cfg | cut -d' ' -f2); [ $iters -gt 1000 ] && iters=1000
  i=0
  while [ $i -lt $iters ]; do
    if [ "$mode" = slow ]; then
      r=$((100 + 2 * i))
      printf '%d,-%d.%02d,0.3,%s,%s\n' $i $((r / 100)) $((r % 100)) $fz $my >> history.csv
    else
      # residual 1e-6 at row 200; CFz (1e-9 units) still falling by 3e-5 per row until row 300 (trendlimit: until 600)
      r=$((100 + 5 * i / 2))
      flat=300; [ "$mode" = trendlimit ] && flat=600
      d=$((flat - i)); [ $d -lt 0 ] && d=0
      c=$((200000000 + 30000 * d))
      printf '%d,-%d.%02d,0.3,%d.%09d,-0.32\n' $i $((r / 100)) $((r % 100)) $((c / 1000000000)) $((c % 1000000000)) >> history.csv
    fi
    [ $(( (i + 1) % freq )) -eq 0 ] && { echo "restart $i" > restart.dat; echo "surface $i" > surface_flow.vtk; }
    sleep 0.005
    i=$((i+1))
  done
  echo "restart end" > restart.dat; echo "surface end" > surface_flow.vtk
  exit 0
fi
if [ "$mode" = eflip ]; then
  # converged rows at once (the runner decides to stop), then rows that would fail the acceptance on their own
  echo '"Inner_Iter","rms[Rho]","CFx","CFz","CMy"' > history.part
  i=0
  while [ $i -lt 80 ]; do echo "$i,-$((10 + i / 10)).$((i % 10)),0.3,0.2,-0.32" >> history.part; i=$((i+1)); done
  mv history.part history.csv
  sleep 3
  while [ $i -lt 100 ]; do echo "$i,-3.0,0.3,0.$((2 + 3 * (i % 2))),-0.32" >> history.csv; i=$((i+1)); done
  echo restart > restart.dat
  exit 0
fi
echo '"Inner_Iter","rms[Rho]","CFx","CFz","CMy"' > history.part
i=0
while [ $i -lt 30 ]; do
  case $mode in
    stall) r=$(echo "-1.0 - 0.02*$i" | bc -l);;
    unstable) r=$(echo "-1.0 - 0.2*$i" | bc -l);;
    *) r=$(echo "-1.0 - 0.25*$i" | bc -l);;
  esac
  fx=0.3; fz=$(echo "$aoa * 0.05" | bc -l); my=$(echo "0 - $aoa * 0.08" | bc -l)
  [ "$mode" = unstable ] && { par=$((i % 2)); fz=$(echo "$aoa * 0.05 + $par * 0.3" | bc -l); }
  echo "$i,$r,$fx,$fz,$my" >> history.part
  [ "$mode" = ok ] && [ $i -ge 24 ] && break
  i=$((i+1))
done
echo restart > restart.dat
# published at once, so that the early-stop poll never judges a part of it
mv history.part history.csv
exit 0
"#;

    fn tools_with(script: &Path) -> ToolReport {
        let ok = |p: Option<PathBuf>| ToolInfo { name: "x", path: p, version: Some("mock".into()), problem: None };
        ToolReport { prefix: None, su2: ok(Some(script.to_path_buf())), mpi: ok(None), gmsh: ok(None), manifold: ok(None) }
    }

    struct Env {
        dir: TempDir,
        tools: ToolReport,
        opt: CfdOptions,
    }

    fn env(name: &str, mode: &str) -> Env {
        let dir = TempDir::new(&format!("runner_{name}"));
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
        let pts: Vec<SolvedPoint> =
            out.iter().map(|o| SolvedPoint { mach: o.spec.mach, alpha_deg: o.spec.alpha_deg, coeffs: o.result.as_ref().map(|r| r.coeffs) }).collect();
        let (corrected, _) = crate::table::apply_alpha_mode(&pts, AlphaMode::Single);
        write_cases_csv(&e.dir.join("cases.csv"), &out, &corrected, AlphaMode::Single).unwrap();
        let csv = std::fs::read_to_string(e.dir.join("cases.csv")).unwrap();
        assert_eq!(csv.lines().count(), 7);
        assert!(csv.contains("converged"));
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
        let pts: Vec<SolvedPoint> =
            out.iter().map(|o| SolvedPoint { mach: o.spec.mach, alpha_deg: o.spec.alpha_deg, coeffs: o.result.as_ref().map(|r| r.coeffs) }).collect();
        let (corrected, asym) = crate::table::apply_alpha_mode(&pts, AlphaMode::Mirror);
        assert_eq!((corrected.len(), asym.len()), (4, 2));
        assert!((corrected.iter().find(|p| p.alpha_deg == 4.0).unwrap().coeffs.unwrap().cn - 0.2).abs() < 1e-9);
        write_cases_csv(&e.dir.join("cases.csv"), &out, &corrected, AlphaMode::Mirror).unwrap();
        let csv = std::fs::read_to_string(e.dir.join("cases.csv")).unwrap();
        assert_eq!(csv.lines().count(), 1 + 6 + 2);
        assert_eq!(csv.lines().filter(|l| l.contains(",combined,")).count(), 2);
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
    }

    #[test]
    fn unconverged_cases_are_accepted_only_when_practically_converged() {
        let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
        // `stall`: residual -1 .. -1.6 (drop < 3 orders), coefficients constant -> failed.
        let e = env("stall", "stall");
        let out = run_all(&e.setup(), &|_| {}).unwrap();
        assert!(out.iter().all(|o| o.result.is_none()));
        assert!(out[0].error.as_ref().unwrap().contains("not converged"));
        // `slowdrop`: reaches -1 - 0.25 * 29 = -8.25 > ... use a looser target so the loop ends above it.
        let mut e = env("accept", "ok");
        e.opt.convergence = 1e-12; // target -12, never reached: residual ends at -7.25
        let out = run_all(&e.setup(), &|_| {}).unwrap();
        let r = out[1].result.as_ref().expect("accepted");
        assert!(!r.converged && (r.coeffs.cn - 0.2).abs() < 1e-9);
        // Unstable coefficients with a big residual drop are rejected.
        let mut e = env("unstable", "unstable");
        e.opt.convergence = 1e-12;
        let out = run_all(&e.setup(), &|_| {}).unwrap();
        let a4 = out.iter().find(|o| o.spec.alpha_deg == 4.0).unwrap();
        assert!(a4.result.is_none() && a4.error.as_ref().unwrap().contains("still changing"), "{:?}", a4.error);
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
    }
    /// Mock SU2 for RANS: startup stage (`CONV_FILENAME= history_startup`) with peaks rho -1, nu -4; final stage
    /// cold from -2 / -5 (or a limit cycle), warm from -4.5 / -6.5. Modes: ok | cycle | warmfail | thick (y+ 4) |
    /// sdiverge / sabort (the startup diverges unless conservative; sabort ends like SU2's own "Residual > 10^20"
    /// abort) | sdiverge2 (the startup always diverges) |
    /// diverge (the cold second-order stage diverges unless conservative) | diverge2 (always) | kill9 (SIGKILL in
    /// the cold second-order stage).
    const MOCK_RANS: &str = r#"#!/bin/sh
cfg=$1
rf=$(grep '^RESTART_FILENAME' $cfg | cut -d' ' -f2)
# Like SU2 8.3: SIGTERM ends the run without writing any output.
trap 'exit 1' TERM
freq=$(grep '^OUTPUT_WRT_FREQ' $cfg | cut -d' ' -f2)
mach=$(grep '^MACH_NUMBER' $cfg | cut -d' ' -f2)
aoa=$(grep '^AOA' $cfg | cut -d' ' -f2)
sol=$(grep '^SOLUTION_FILENAME' $cfg | cut -d' ' -f2)
conv=$(grep '^CONV_FILENAME' $cfg | cut -d' ' -f2)
mode=$(cat "$(dirname "$0")/mode")
echo "$mach $aoa $conv ${sol:-none}" >> "$(dirname "$0")/calls"
case "$sol" in ../*) warm=1;; *) warm=0;; esac
cons=0; grep -q '^CFL_ADAPT_PARAM=.*, 20.0 )' $cfg && cons=1
echo "$mach $aoa $conv cons=$cons" >> "$(dirname "$0")/stages"
[ "$mode" = kill9 ] && [ "$conv" = history ] && [ $warm = 0 ] && kill -9 $$
if [ "$conv" = history ] && [ $warm = 1 ]; then
  [ "$mode" = warmfail ] && { echo "Error: warm boom" >&2; exit 1; }
  grep -q Nu_Tilde "$sol" || { echo "Error: no SA variable in the restart" >&2; exit 1; }
fi
case "$mode" in markers*) mk=1;; *) mk=0;; esac
# The history appears at once when the run is complete (except the slow mode), so the runner's once-a-second poll
# never judges a part of it: the outcome does not depend on the timing (as with a real run, whose early stop the
# slow mode tests).
hf=$conv.csv.part; [ "$mode" = slow ] && [ "$conv" = history ] && hf=$conv.csv
case "$mode" in cycle|markers_cycle) cyc=1;; *) cyc=0;; esac
if [ $mk = 1 ]; then echo '"Inner_Iter","rms[Rho]","rms[nu]","CFx","CFz","CMy","CFx(wall)","CFx(base)"' > $hf
else echo '"Inner_Iter","rms[Rho]","rms[nu]","CFx","CFz","CMy"' > $hf; fi
n=40; [ "$mode" = slow ] && [ "$conv" = history ] && n=400
i=0
while [ $i -lt $n ]; do
  if [ "$conv" = history_startup ]; then r=$(echo "-1.0 - 0.05*$i" | bc -l); nu=$(echo "-4.0 - 0.05*$i" | bc -l)
  elif [ $warm = 1 ]; then r=$(echo "-4.5 - 0.1*$i" | bc -l); nu=$(echo "-6.5 - 0.1*$i" | bc -l)
  elif [ $cyc = 1 ]; then r=$(echo "-2.0 - 0.08*$i" | bc -l); nu=$(echo "-5.0 - 0.02*$i" | bc -l)
  else r=$(echo "-2.0 - 0.2*$i" | bc -l); nu=$(echo "-5.0 - 0.2*$i" | bc -l)
  fi
  if [ "$conv" = history ] && [ $warm = 0 ] && [ $i -ge 20 ] && { [ "$mode" = diverge2 ] || { [ "$mode" = diverge ] && [ $cons = 0 ]; }; }; then
    nu=$(echo "-9.0 + 0.5*($i - 20)" | bc -l)
  fi
  # startup divergence: the SA residual climbs (sample rocket at M 2 on the 8 mm mesh); "abort" ends like SU2 itself
  if [ "$conv" = history_startup ] && [ $i -ge 20 ] && { [ "$mode" = sdiverge2 ] || { [ "$mode" = sdiverge ] && [ $cons = 0 ]; } || { [ "$mode" = sabort ] && [ $cons = 0 ]; }; }; then
    nu=$(echo "-6.0 + 0.5*($i - 20)" | bc -l)
    if [ "$mode" = sabort ] && [ $i -ge 30 ]; then
      mv $hf $conv.csv; echo "SU2 has diverged. (Residual > 10^20 detected)."; exit 1
    fi
  fi
  fx=0.3; fz=$(echo "$aoa * 0.05" | bc -l); my=$(echo "0 - $aoa * 0.08" | bc -l)
  if [ "$mode" = cycle ] && [ "$conv" = history ] && [ $warm = 0 ]; then par=$((i % 2)); fx=$(echo "0.3 + $par * 0.002" | bc -l); fi
  if [ $mk = 1 ]; then
    # wall 0.2 (drifting 1e-6 per row, 0.002 apart in the cycle), base 0.1 + 0.05 for the total
    wx=$(echo "0.2 + $i * 0.000001" | bc -l); bx=0.1
    if [ "$mode" = markers_cycle ] && [ "$conv" = history ] && [ $warm = 0 ]; then wx=$(echo "0.2 + $((i % 2)) * 0.002" | bc -l); fi
    fx=$(echo "$wx + $bx" | bc -l)
    echo "$i,$r,$nu,$fx,$fz,$my,$wx,$bx" >> $hf
  else
    echo "$i,$r,$nu,$fx,$fz,$my" >> $hf
  fi
  if [ "$mode" = slow ] && [ "$conv" = history ]; then
    # periodic output (OUTPUT_WRT_FREQ) after the history row of the iteration, then the next iteration
    [ $(( (i + 1) % freq )) -eq 0 ] && { printf "Density Nu_Tilde restart %s" $i > $rf; printf '# vtk %s\n' $i > surface_flow.vtk; }
    sleep 0.02
  fi
  i=$((i+1))
done
if [ "$mode" = flip ] && [ "$conv" = history ] && [ $warm = 0 ]; then
  # converged rows at once (the runner decides to stop), then rows that would be rejected on their own
  mv $hf $conv.csv; hf=$conv.csv
  sleep 3
  while [ $i -lt 70 ]; do echo "$i,$r,$nu,0.3$((5 * (i % 2))),$fz,$my" >> $hf; i=$((i+1)); done
fi
printf 'Density Nu_Tilde restart' > $rf
yp=1.0; [ "$mode" = thick ] && yp=4.0
printf '# vtk DataFile Version 3.0\nvtk output\nASCII\nDATASET UNSTRUCTURED_GRID\nPOINTS 3 double\n0 0 0 0.5 0 0\n1 0 0\nPOINT_DATA 3\nSCALARS Y_Plus double 1\nLOOKUP_TABLE default\n%s %s 0.1\n' $yp $yp > surface_flow.vtk
[ "$hf" != "$conv.csv" ] && mv $hf $conv.csv
exit 0
"#;

    fn env_rans(name: &str, mode: &str) -> Env {
        let mut e = env(name, mode);
        let script = e.dir.join("su2_mock_rans.sh");
        std::fs::write(&script, MOCK_RANS).unwrap();
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
        e.tools = tools_with(&script);
        e.opt = CfdOptions { min_iterations: 20, stable_window: 10, iterations: 100, ..CfdOptions { model: FlowModel::Rans, ..e.opt.clone() } };
        e
    }

    fn done(e: &Env, o: &CaseOutcome) -> serde_json::Value {
        serde_json::from_str(&std::fs::read_to_string(o.spec.dir(&e.dir.join("work")).join("done.json")).unwrap()).unwrap()
    }

    #[test]
    fn rans_cold_two_stage_warm_chain_and_reference_peak() {
        let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
        let e = env_rans("rans_ok", "ok");
        let out = run_all(&e.setup(), &|_| {}).unwrap();
        assert!(out.iter().all(|o| o.result.as_ref().is_some_and(|r| r.converged && !r.limit_cycle)), "{out:?}");
        // Every Mach number starts cold (startup + final stage), the alpha chain is warm.
        let calls = e.calls();
        assert_eq!(calls.iter().filter(|c| c.contains(" history_startup none")).count(), 2, "{calls:?}");
        assert_eq!(calls.iter().filter(|c| c.ends_with(" history restart_startup.dat")).count(), 2, "{calls:?}");
        assert!(calls.contains(&"0.500000 8.000000 history ../m0.500_a04.00/restart.dat".to_string()), "{calls:?}");
        assert_eq!(calls.len(), 2 * 2 + 4);
        assert!(!out[0].warm && out[1].warm && out[2].warm && !out[3].warm);
        // The cold reference peak travels along the warm chain into done.json.
        let peak = ResidualPeak { rho: -1.0, nu: -4.0 };
        for o in &out {
            assert_eq!(o.result.as_ref().unwrap().reference_peak, Some(peak));
            assert!((o.result.as_ref().unwrap().yplus_p95.unwrap() - 1.0).abs() < 1e-12);
        }
        assert_eq!(done(&e, &out[2])["reference_peak"]["rho"], -1.0);
        assert!(done(&e, &out[2]).get("warnings").is_none());
        // Resume: the restart files of finished chains are gone, so a re-run case starts cold.
        let victim = out[2].spec.dir(&e.dir.join("work")).join("done.json");
        std::fs::remove_file(&victim).unwrap();
        let n0 = e.calls().len();
        let again = run_all(&e.setup(), &|_| {}).unwrap();
        assert_eq!(e.calls().len(), n0 + 2);
        assert!(!again[2].warm && again[2].result.as_ref().unwrap().reference_peak == Some(peak));
    }

    #[test]
    fn rans_failed_warm_start_is_solved_cold() {
        let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
        let e = env_rans("rans_warmfail", "warmfail");
        let out = run_all(&e.setup(), &|_| {}).unwrap();
        assert!(out.iter().all(|o| o.result.as_ref().is_some_and(|r| r.converged)), "{out:?}");
        // 2 cold cases (2 calls each) + 4 cases with a failed warm attempt and a cold rerun (3 calls each).
        assert_eq!(e.calls().len(), 2 * 2 + 4 * 3, "{:?}", e.calls());
        let r = out[1].result.as_ref().unwrap();
        assert!(!out[1].warm && r.warnings.iter().any(|w| w.contains("warm start failed") && w.contains("freestream")), "{r:?}");
    }

    #[test]
    fn rans_limit_cycle_is_accepted_unconverged_and_thick_wall_warns() {
        let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
        let e = env_rans("rans_cycle", "cycle");
        let out = run_all(&e.setup(), &|_| {}).unwrap();
        // The cold alpha = 0 cases cycle; the warm ones converge.
        let r0 = out[0].result.as_ref().expect("limit cycle accepted");
        assert!(!r0.converged && r0.limit_cycle && r0.warnings.iter().any(|w| w.contains("limit cycle")), "{r0:?}");
        assert!((r0.coeffs.ca - 0.301).abs() < 1e-3, "window average {}", r0.coeffs.ca);
        assert!(out[1].result.as_ref().unwrap().converged);
        write_cases_csv(&e.dir.join("cases.csv"), &out, &[], AlphaMode::Single).unwrap();
        let csv = std::fs::read_to_string(e.dir.join("cases.csv")).unwrap();
        assert_eq!(csv.lines().filter(|l| l.contains(",unconverged,")).count(), 2, "{csv}");
        // RANS columns: residual_first is the first row of the second-order history (cold: -2.0), the reference is
        // the startup peak (-1.0), then the drop and the verdict.
        let head: Vec<&str> = csv.lines().next().unwrap().split(',').collect();
        let row: Vec<&str> = csv.lines().nth(1).unwrap().split(',').collect();
        let col = |name: &str| row[head.iter().position(|h| *h == name).unwrap()];
        assert_eq!(head.len(), row.len());
        assert_eq!((col("residual_first"), col("residual_reference"), col("rans_verdict")), ("-2.00000000e0", "-1.00000000e0", "limit_cycle"), "{csv}");
        assert!(col("residual_drop").parse::<f64>().unwrap() > 2.0, "{csv}");
        assert_eq!(csv.lines().nth(2).unwrap().split(',').next_back(), Some("converged"));
        let e = env_rans("rans_thick", "thick");
        let out = run_all(&e.setup(), &|_| {}).unwrap();
        let r = out[0].result.as_ref().unwrap();
        assert!(r.converged && (r.yplus_p95.unwrap() - 4.0).abs() < 1e-12);
        assert!(r.warnings.iter().any(|w| w.contains("y+ P95 = 4.00 exceeds 3")), "{r:?}");
        assert!(done(&e, &out[0])["warnings"][0].as_str().unwrap().contains("y+"));
    }

    fn sample_model() -> aero::AeroModel {
        use geom::sample::SampleRocket;
        let g = geom::extract(&SampleRocket::default().mesh(), &geom::ExtractOptions::default()).unwrap();
        aero::AeroModel::new(g, aero::AeroOptions { mach_max: 3.0, mach_step: 0.25, alpha_max_deg: 16.0, alpha_step_deg: 2.0, ..Default::default() })
    }

    /// Axisymmetric table of the outcomes, built the way `lib.rs` does (base decision included).
    fn table_of(e: &Env, out: &[CaseOutcome], model: &aero::AeroModel) -> (aero::AeroTable, bool) {
        let missing = crate::rans_markers_missing(out.iter().filter_map(|o| o.result.as_ref()));
        let base = crate::table_base_from_cfd(e.opt.base_from_cfd(), missing);
        let pts: Vec<SolvedPoint> =
            out.iter().map(|o| SolvedPoint { mach: o.spec.mach, alpha_deg: o.spec.alpha_deg, coeffs: o.result.as_ref().map(|r| r.coeffs) }).collect();
        let t = crate::table::build_from_points_roll(&pts, model, FlowModel::Rans, "h".into(), aero::Extrapolation::Linear, &[], base, false).unwrap().0;
        (t, base)
    }

    /// Marker columns present: the axial coefficient is the wall marker (the row the totals come from), the table adds
    /// the Barrowman base; with `rans_base = "cfd"` it is wall + base and the table adds no base; a changed base
    /// treatment re-runs the finished cases.
    #[test]
    fn rans_marker_columns_decide_the_base_term_of_the_table() {
        let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
        let mut e = env_rans("rans_markers", "markers");
        let model = sample_model();
        let b = model.cd0(0.5, false);
        let out = run_all(&e.setup(), &|_| {}).unwrap();
        let n_calls = e.calls().len();
        // Converged cases: the row the totals come from (the last row read, wall 0.2 + i 1e-6 with i <= 39 when the
        // early stop reads a partial history), base 0.1.
        for o in &out {
            let r = o.result.as_ref().unwrap();
            let a = r.axial.expect("RANS cases record the markers");
            let wall = a.wall_cfx.unwrap();
            assert!((0.2..=0.2 + 39e-6 + 1e-12).contains(&wall) && (a.base_cfx.unwrap() - 0.1).abs() < 1e-12, "{a:?}");
            assert!(!a.base_in_cfx && !a.markers_missing);
            assert!((r.coeffs.ca - wall).abs() < 1e-9, "ca {} is the wall marker of the same row", r.coeffs.ca);
            assert!(done(&e, o)["axial"]["wall_cfx"].is_number() && done(&e, o)["axial"]["base_in_cfx"] == false);
        }
        let wall_at = |out: &[CaseOutcome]| {
            let o = out.iter().find(|o| o.spec.mach == 0.5 && o.spec.alpha_deg == 0.0).expect("case M 0.5, alpha 0");
            o.result.as_ref().unwrap().axial.unwrap().wall_cfx.unwrap()
        };
        let (t, base) = table_of(&e, &out, &model);
        assert!(!base);
        let wall = wall_at(&out);
        assert!(
            (t.lookup(0.5, 0.0).ca_off
                - (wall + b.base + b.extra + (crate::forces::roughness_increment(&model, 0.5) + crate::forces::fin_edge_increment(&model, 0.5, false))))
            .abs()
                < 1e-6,
            "{}",
            t.lookup(0.5, 0.0).ca_off
        );
        // rans_base = "cfd": wall + base, and no Barrowman base on top.
        e.opt.rans_base = crate::config::RansBase::Cfd;
        let out = run_all(&e.setup(), &|_| {}).unwrap();
        assert!(e.calls().len() > n_calls, "the base treatment is part of the case hash");
        for o in &out {
            let r = o.result.as_ref().unwrap();
            let a = r.axial.unwrap();
            assert!(a.base_in_cfx && !a.markers_missing);
            assert!((r.coeffs.ca - (a.wall_cfx.unwrap() + 0.1)).abs() < 1e-9, "ca {}", r.coeffs.ca);
        }
        let (t, base) = table_of(&e, &out, &model);
        assert!(base);
        let wall = wall_at(&out);
        assert!(
            (t.lookup(0.5, 0.0).ca_off
                - (wall + 0.1 + b.extra + (crate::forces::roughness_increment(&model, 0.5) + crate::forces::fin_edge_increment(&model, 0.5, false))))
            .abs()
                < 1e-6
        );
    }

    /// No marker columns: the monitored total holds the base cap, so the table must not add the Barrowman base.
    #[test]
    fn rans_without_marker_columns_uses_the_total_and_adds_no_base() {
        let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
        let e = env_rans("rans_nomarkers", "ok");
        let model = sample_model();
        let out = run_all(&e.setup(), &|_| {}).unwrap();
        for o in &out {
            let r = o.result.as_ref().unwrap();
            let a = r.axial.unwrap();
            assert!(a.wall_cfx.is_none() && a.base_cfx.is_none() && a.base_in_cfx && a.markers_missing, "{a:?}");
            assert!((r.coeffs.ca - 0.3).abs() < 1e-12);
            assert_eq!(done(&e, o)["axial"]["base_in_cfx"], true);
            assert!(done(&e, o)["axial"].get("wall_cfx").is_some_and(|v| v.is_null()));
        }
        let (t, base) = table_of(&e, &out, &model);
        assert!(base, "no double count of the base");
        let b = model.cd0(0.5, false);
        assert!(
            (t.lookup(0.5, 0.0).ca_off
                - (0.3 + b.extra + (crate::forces::roughness_increment(&model, 0.5) + crate::forces::fin_edge_increment(&model, 0.5, false))))
            .abs()
                < 1e-6
        );
    }

    /// Limit cycle: the markers are averaged over the window the totals are averaged over.
    #[test]
    fn rans_limit_cycle_markers_use_the_window_of_the_totals() {
        let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
        let e = env_rans("rans_mk_cycle", "markers_cycle");
        let out = run_all(&e.setup(), &|_| {}).unwrap();
        let r0 = out[0].result.as_ref().expect("limit cycle accepted");
        assert!(r0.limit_cycle);
        let a = r0.axial.unwrap();
        assert!((a.wall_cfx.unwrap() - 0.201).abs() < 1e-9 && (r0.coeffs.ca - 0.201).abs() < 1e-9, "{a:?} {}", r0.coeffs.ca);
        assert!((a.base_cfx.unwrap() - 0.1).abs() < 1e-12);
    }

    #[test]
    fn euler_done_json_has_no_axial_record() {
        let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
        let e = env("euler_noaxial", "ok");
        let out = run_all(&e.setup(), &|_| {}).unwrap();
        assert!(out.iter().all(|o| o.result.as_ref().unwrap().axial.is_none() && done(&e, o).get("axial").is_none()));
    }

    fn hist(rows: &[(f64, f64, f64, f64, f64)]) -> History {
        let mut t = String::from("\"Inner_Iter\",\"rms[Rho]\",\"rms[nu]\",\"CFx\",\"CFz\",\"CMy\"\n");
        for (i, (r, nu, fx, fz, my)) in rows.iter().enumerate() {
            t += &format!("{i},{r},{nu},{fx},{fz},{my}\n");
        }
        History::parse(&t).unwrap()
    }

    /// `n` rows, residuals linear from (rho0, nu0) to (rho1, nu1) and then flat, constant coefficients unless `cycle`.
    fn ramp(n: usize, rho: (f64, f64), nu: (f64, f64), cycle: f64) -> History {
        let rows: Vec<_> = (0..n)
            .map(|i| {
                // linear, then flat over the last RANS_RESIDUAL_ROWS rows (the rows the drops are measured on)
                let f = (i as f64 / n.saturating_sub(RANS_RESIDUAL_ROWS + 1).max(1) as f64).min(1.0);
                let c = if i % 2 == 1 { cycle } else { 0.0 };
                (rho.0 + (rho.1 - rho.0) * f, nu.0 + (nu.1 - nu.0) * f, 0.3 + c, 0.2, -1.6)
            })
            .collect();
        hist(&rows)
    }

    #[test]
    fn rans_acceptance_rule() {
        let k = StabilityKind::Pitch;
        let reference = ResidualPeak { rho: -1.0, nu: -4.0 };
        // Warm chain: judged against the cold reference, not the run's own (low) peak. Own drop 1.1 orders,
        // 3.1 below the reference: converged; 2.9 below the reference (own drop 2.9): only a limit cycle.
        assert_eq!(rans_verdict(&ramp(50, (-3.0, -4.1), (-6.0, -7.0), 0.0), reference, 20, k, 18.0, 4000), RansVerdict::Converged);
        assert_eq!(rans_verdict(&ramp(50, (-1.0, -3.9), (-6.0, -7.0), 0.0), reference, 20, k, 18.0, 4000), RansVerdict::LimitCycle);
        // Less than 2 orders below the reference: rejected even with flat coefficients.
        assert!(matches!(rans_verdict(&ramp(50, (-1.5, -2.9), (-6.0, -7.0), 0.0), reference, 20, k, 18.0, 4000), RansVerdict::Rejected(_)));
        // A run whose own peak is above the reference is judged like a cold start, from its own peak (3.7 orders;
        // 2.2 below the cold reference).
        assert_eq!(rans_verdict(&ramp(50, (0.5, -3.2), (-6.0, -7.0), 0.0), reference, 20, k, 18.0, 4000), RansVerdict::Converged);
        assert_eq!(effective_reference(reference, &ramp(10, (0.5, -3.2), (-6.0, -7.0), 0.0)), ResidualPeak { rho: 0.5, nu: -4.0 });
        // The SA residual must drop 2 orders too: with 1.5 orders the case is at most a limit cycle.
        assert_eq!(rans_verdict(&ramp(50, (-3.0, -6.0), (-4.0, -5.5), 0.0), reference, 20, k, 18.0, 4000), RansVerdict::LimitCycle);
        let v = rans_verdict(&ramp(50, (-1.0, -2.5), (-4.0, -5.5), 0.0), reference, 20, k, 18.0, 4000);
        assert!(matches!(&v, RansVerdict::Rejected(m) if m.contains("SA residual 1.5") && m.contains("density residual 1.5")), "{v:?}");
        assert_eq!(rans_verdict(&ramp(50, (-3.0, -6.0), (-4.0, -6.1), 0.0), reference, 20, k, 18.0, 4000), RansVerdict::Converged);
        // Limit cycle: 2 orders and a spread within 1 % of the force scale; a larger spread is rejected.
        assert_eq!(rans_verdict(&ramp(50, (-2.0, -3.5), (-5.0, -5.5), 0.002), reference, 20, k, 18.0, 4000), RansVerdict::LimitCycle);
        assert!(matches!(rans_verdict(&ramp(50, (-2.0, -3.5), (-5.0, -5.5), 0.02), reference, 20, k, 18.0, 4000), RansVerdict::Rejected(_)));
        assert!(matches!(rans_verdict(&ramp(50, (-2.0, -2.5), (-5.0, -5.5), 0.002), reference, 20, k, 18.0, 4000), RansVerdict::Rejected(_)));
        // A converged residual with an oscillating force is a limit cycle, not converged.
        assert_eq!(rans_verdict(&ramp(50, (-3.0, -6.0), (-4.0, -7.0), 0.002), reference, 20, k, 18.0, 4000), RansVerdict::LimitCycle);
    }

    #[test]
    fn yplus_percentile_and_restart_kind() {
        let vtk = "# vtk DataFile Version 3.0\nvtk output\nASCII\nDATASET UNSTRUCTURED_GRID\nPOINTS 5 double\n0 0 0 0.25 0 0 0.5 0 0\n0.75 0 0 1 0 0\nCELLS 1 2\n1 0\nPOINT_DATA 5\nSCALARS Density double 1\nLOOKUP_TABLE default\n1 1 1 1 1\nSCALARS Y_Plus double 1\nLOOKUP_TABLE default\n0.5 1.0 2.0 4.0\n90\n";
        // The base point (largest x, y+ 90) is left out; the 95th percentile of [0.5, 1, 2, 4] is 4.
        assert_eq!(wall_yplus_p95(vtk), Some(4.0));
        assert_eq!(wall_yplus_p95("# vtk\nPOINTS 1 double\n0 0 0\n"), None);
        let dir = TempDir::new("restart_kind");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("rans.dat"), b"\x00\x01Density\x00Nu_Tilde\x00").unwrap();
        std::fs::write(dir.join("euler.dat"), b"\x00\x01Density\x00Energy\x00").unwrap();
        assert!(restart_has_turbulence(&dir.join("rans.dat")) && !restart_has_turbulence(&dir.join("euler.dat")));
        assert!(!restart_has_turbulence(&dir.join("missing.dat")));
    }

    /// Flat-plate model (TMR 273 x 193, M 0.2): the drag converges like `exp(-i / 310)`, the density residual by one
    /// order per 1320 iterations. The flatness check alone accepts at about 1500 iterations with the drag still
    /// about 1.9 % high; with the trend extrapolation the accepted drag is within 0.5 %.
    #[test]
    fn rans_slow_drift_is_not_accepted_until_the_trend_has_settled() {
        let reference = ResidualPeak { rho: -2.7, nu: -5.36 };
        let cd = |i: f64| 0.003 * (1.0 + 2.0 * (-i / 310.0).exp());
        let mut t = String::from("\"Inner_Iter\",\"rms[Rho]\",\"rms[nu]\",\"CFx\",\"CFz\",\"CMy\"\n");
        for i in 0..6000 {
            let x = i as f64;
            t += &format!("{i},{},{},{},0.0,0.0\n", -4.0 - x / 1320.0, -7.0 - x / 1100.0, cd(x));
        }
        let full = History::parse(&t).unwrap();
        let at = |n: usize| History { columns: full.columns.clone(), rows: full.rows[..n].to_vec() };
        let k = StabilityKind::Pitch;
        let first = |f: &dyn Fn(&History) -> bool| (400..6000).step_by(10).find(|&n| f(&at(n))).unwrap();
        let flat = first(&|h| h.rans_coefficients_stable(200, k, 18.0));
        let err = |n: usize| cd(n as f64 - 1.0) / 0.003 - 1.0;
        assert!(err(flat) > 0.01, "flatness alone stops at {flat} with {:.3} %", 100.0 * err(flat));
        assert!(!matches!(rans_verdict(&at(flat), reference, 200, k, 18.0, 4000), RansVerdict::Converged));
        let conv = first(&|h| rans_verdict(h, reference, 200, k, 18.0, 4000) == RansVerdict::Converged);
        assert!(err(conv) < 0.005 && conv > flat, "converged at {conv} with {:.3} % (flat at {flat})", 100.0 * err(conv));
        // Without residual decay the trend is extrapolated over the whole budget: a slope that is small over the
        // window but not over the budget keeps the case open.
        let mut t = String::from("\"Inner_Iter\",\"rms[Rho]\",\"rms[nu]\",\"CFx\",\"CFz\",\"CMy\"\n");
        for i in 0..600 {
            t += &format!("{i},-6.0,-8.0,{},0.0,0.0\n", 0.3 * (1.0 + 2e-6 * i as f64));
        }
        let stalled = History::parse(&t).unwrap();
        assert!(stalled.rans_coefficients_stable(200, k, 18.0));
        assert!(!stalled.rans_trend_settled(200, k, 18.0, 4000) && stalled.rans_trend_settled(200, k, 18.0, 1000));
    }

    #[test]
    fn rans_divergence_is_retried_conservatively_then_fails() {
        let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
        let stages = |e: &Env| std::fs::read_to_string(e.dir.join("stages")).unwrap_or_default();
        // Diverges with the normal settings, converges with the conservative ones (startup not repeated).
        let mut e = env_rans("rans_diverge", "diverge");
        e.opt.machs = vec![0.5];
        e.opt.alphas_deg = vec![0.0, 4.0];
        let out = run_all(&e.setup(), &|_| {}).unwrap();
        let r = out[0].result.as_ref().unwrap_or_else(|| panic!("{:?} {}", out[0].error, stages(&e)));
        assert!(
            r.converged
                && r.warnings.iter().any(|w| w.contains("second-order stage diverged")
                    && w.contains("SA residual rose")
                    && w.contains("capped at 20 instead of 200")
                    && w.contains("SA equation at 0.2 of the flow CFL instead of 1")),
            "{r:?}"
        );
        let st = stages(&e);
        assert_eq!(
            st.lines().filter(|l| l.starts_with("0.500000 0.000000")).collect::<Vec<_>>(),
            ["0.500000 0.000000 history_startup cons=0", "0.500000 0.000000 history cons=0", "0.500000 0.000000 history cons=1"]
        );
        // The warm alpha = 4 case does not diverge and seeds from the retried solution.
        assert!(out[1].warm && out[1].result.as_ref().unwrap().converged);
        assert!(!out[0].spec.dir(&e.dir.join("work")).join(crate::su2cfg::STARTUP_RESTART).exists());
        // Diverges with both: failed with the reason, after one retry, not after the iteration limit.
        let mut e = env_rans("rans_diverge2", "diverge2");
        e.opt.machs = vec![0.5];
        e.opt.alphas_deg = vec![0.0, 4.0];
        let out = run_all(&e.setup(), &|_| {}).unwrap();
        let msg = out[0].error.as_ref().unwrap();
        assert!(out[0].result.is_none() && msg.contains("diverged twice") && msg.contains("SA residual rose"), "{msg}");
        assert_eq!(stages(&e).lines().filter(|l| l.starts_with("0.500000 0.000000 history ")).count(), 2);
        assert!(failure_summary(&out)[0].starts_with("error[IY-F118]: aero.cfd: case m0.500_a00.00 failed: diverged twice"), "{:?}", failure_summary(&out));
    }

    #[test]
    fn killed_su2_names_the_signal_and_the_memory_hint() {
        let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
        let mut e = env_rans("rans_kill9", "kill9");
        e.opt.machs = vec![0.5];
        e.opt.alphas_deg = vec![0.0, 4.0];
        std::fs::write(e.dir.join("mesh_stats.json"), r#"{"nodes": 405590, "cells": 901081}"#).unwrap();
        let out = run_all(&e.setup(), &|_| {}).unwrap();
        let msg = out[0].error.as_ref().unwrap();
        assert!(msg.starts_with("[IY-F120] SU2 was killed by signal 9 (SIGKILL; most likely out of memory: about ") && msg.contains("GB per case"), "{msg}");
        assert!(msg.contains("lower parallel_cases or ranks_per_case"), "{msg}");
        let sum = failure_summary(&out);
        assert!(sum.iter().any(|l| l.starts_with("error[IY-F120]: aero.cfd: case m0.500_a00.00 failed: SU2 was killed by signal 9")), "{sum:?}");
        // mpirun reports a killed rank in its log and exits with an ordinary code.
        use std::os::unix::process::ExitStatusExt;
        let st = std::process::ExitStatus::from_raw(1 << 8);
        let log = "prterun noticed that process rank 2 with PID 0 on node kuro exited on signal 9 (Killed).";
        assert_eq!(killing_signal(&st, log), Some(9));
        assert_eq!(killing_signal(&std::process::ExitStatus::from_raw(137 << 8), ""), Some(9));
        assert_eq!(killing_signal(&std::process::ExitStatus::from_raw(9), ""), Some(9));
        assert_eq!(killing_signal(&st, "Error: boom"), None);
    }

    #[test]
    fn rans_stalled_residual_with_stationary_coefficients_is_accepted_unconverged() {
        let k = StabilityKind::Pitch;
        let reference = ResidualPeak { rho: -3.5, nu: -7.0 };
        // Residual chattering between -4.1 and -5.9 (the sample rocket at M 0.8), forces with a small oscillation.
        let mk = |n: usize, cfx_slope: f64, rho_slope: f64| {
            let mut t = String::from("\"Inner_Iter\",\"rms[Rho]\",\"rms[nu]\",\"CFx\",\"CFz\",\"CMy\"\n");
            for i in 0..n {
                let rho = if i % 40 < 8 { -5.9 } else { -4.4 } - rho_slope * i as f64;
                let w = 1e-4 * ((i % 7) as f64 - 3.0);
                t += &format!("{i},{rho},-8.3,{},{},{}\n", 0.39 + w + cfx_slope * i as f64, 0.1 + w, -1.2 - 12.0 * w);
            }
            History::parse(&t).unwrap()
        };
        let v = rans_verdict(&mk(800, 0.0, 0.0), reference, 200, k, 18.0, 4000);
        let RansVerdict::Stalled(why) = &v else { panic!("{v:?}") };
        assert!(why.contains("residual stalled 0.9 orders"), "{why}");
        assert!(why.contains("CFx 0.39") && why.contains("inter-quartile range"), "{why}");
        // The drop recorded with the verdict is the median one (-3.5 - -4.4), not the worst of the last rows.
        assert!((rans_verdict_drop(&mk(800, 0.0, 0.0), reference, 200, &v).unwrap() - 0.9).abs() < 1e-9);
        assert!((rans_verdict_drop(&mk(800, 0.0, 0.0), reference, 200, &RansVerdict::Converged).unwrap() - 0.9).abs() < 1e-9);
        assert!((rans_density_drop(&ramp(50, (-1.0, -4.1), (-6.0, -7.0), 0.0), reference).unwrap() - 3.1).abs() < 1e-9, "own peak -1 above the reference");
        // Too few rows for two windows: not yet.
        assert!(!matches!(rans_verdict(&mk(350, 0.0, 0.0), reference, 200, k, 18.0, 4000), RansVerdict::Stalled(_)));
        // A force that still drifts (1e-6 per iteration, 1 % over the budget) is not stationary.
        assert!(matches!(rans_verdict(&mk(800, 1e-6, 0.0), reference, 200, k, 18.0, 4000), RansVerdict::Rejected(_)));
        // A residual that is still falling is not stalled (it may converge).
        assert!(!matches!(rans_verdict(&mk(800, 0.0, 0.002), reference, 200, k, 18.0, 4000), RansVerdict::Stalled(_)));
        // Stalled at the level of the reference (no drop): rejected.
        let high = ResidualPeak { rho: -4.6, nu: -7.0 };
        assert!(matches!(rans_verdict(&mk(800, 0.0, 0.0), high, 200, k, 18.0, 4000), RansVerdict::Rejected(_)));
    }

    #[test]
    fn a_stopped_case_is_judged_at_the_stop_decision() {
        let h = hist(&[(-1.0, -4.0, 0.3, 0.2, -1.6), (-5.0, -8.0, 0.3, 0.2, -1.6), (-2.0, -5.0, 0.9, 0.2, -1.6)]);
        let (j, v) = judged_at_stop(&h, Some((2, "decided")), |_| panic!("not judged again"));
        assert_eq!((j.rows.len(), v), (2, "decided"));
        let (j, v) = judged_at_stop(&h, None, |h| h.rows.len());
        assert_eq!((j.rows.len(), v), (3, 3));
        // A decision beyond the history (cannot happen) falls back to judging the whole history.
        assert_eq!(judged_at_stop(&h, Some((9, 0)), |_| 1).1, 1);
    }

    #[test]
    fn euler_case_stopped_by_the_runner_keeps_its_verdict() {
        let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
        // Target reached and settled at the decision; the rows SU2 writes afterwards (residual back at -3, CFz
        // alternating 0.2 / 0.5) would fail the case on their own.
        let mut e = env("euler_eflip", "eflip");
        e.opt.machs = vec![0.5];
        e.opt.alphas_deg = vec![0.0];
        let out = run_all(&e.setup(), &|_| {}).unwrap();
        let r = out[0].result.as_ref().unwrap_or_else(|| panic!("{:?}", out[0].error));
        assert!(r.converged && r.warnings.is_empty(), "{r:?}");
        assert!(
            (r.coeffs.cn - 0.2).abs() < 1e-9 && r.iterations == 100 && r.residual_last == Some(-17.9),
            "residual_last is the residual at the judged row, not of the rows written after the stop: {r:?}"
        );
    }

    #[test]
    fn rans_case_stopped_by_the_runner_keeps_its_verdict() {
        let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
        // Converged at the decision; the rows SU2 writes afterwards (CFx alternating 0.30 / 0.35) would be rejected on
        // their own and fail the cold case.
        let mut e = env_rans("rans_flip", "flip");
        e.opt.machs = vec![0.5];
        e.opt.alphas_deg = vec![0.0];
        let out = run_all(&e.setup(), &|_| {}).unwrap();
        let r = out[0].result.as_ref().unwrap_or_else(|| panic!("{:?}", out[0].error));
        assert!(r.converged && !r.limit_cycle && (r.coeffs.ca - 0.3).abs() < 1e-9 && r.iterations == 70, "{r:?}");
    }

    #[test]
    fn force_spread_ignores_non_finite_values() {
        let v = [0.3, f64::NAN, 0.31, f64::INFINITY, 0.29, 0.3];
        assert_eq!(spread_text("CFx", v.into_iter()).unwrap(), "CFx 0.30000 (inter-quartile range 1.00e-2)");
        assert_eq!(spread_text("CFx", [f64::NAN].into_iter()), None);
    }

    #[test]
    fn rans_residual_dip_is_not_a_drop() {
        // The sample rocket at M 0.8: the residual chatters near -5.9 with a dip to -7.1 in the last row while the CFL
        // is cut; the reference is -3.65. Judged on the last row it would be 3.5 orders, i.e. converged.
        let k = StabilityKind::Pitch;
        let reference = ResidualPeak { rho: -3.65, nu: -7.96 };
        let mut rows: Vec<_> = (0..421).map(|i| (if i % 30 < 3 { -6.0 } else { -5.9 }, -9.1, 0.4366, -0.0121, 0.172)).collect();
        rows.push((-7.13, -9.95, 0.4366, -0.0121, 0.172));
        let v = rans_verdict(&hist(&rows), reference, 200, k, 18.0, 4000);
        assert!(!matches!(v, RansVerdict::Converged), "{v:?}");
        assert!(matches!(&v, RansVerdict::Stalled(m) if m.contains("residual stalled 2.3 orders")), "{v:?}");
    }

    #[test]
    fn rans_early_stop_waits_for_the_periodic_output() {
        let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
        let mut e = env_rans("rans_slow", "slow");
        e.opt.machs = vec![0.5];
        e.opt.alphas_deg = vec![0.0];
        let out = run_all(&e.setup(), &|_| {}).unwrap();
        let r = out[0].result.as_ref().unwrap_or_else(|| panic!("{:?}", out[0].error));
        assert!(r.converged);
        let dir = out[0].spec.dir(&e.dir.join("work"));
        // Stopped early (400 rows available) but only after a periodic output following the decision: the output
        // files are those written at a multiple of OUTPUT_WRT_FREQ = 50 iterations, at or after the stop decision.
        let rows = History::read(&dir.join("history.csv")).unwrap().rows.len();
        assert!(rows < 400, "{rows}");
        // (restart.dat itself is removed once no case needs it; the surface file is written with it.)
        let vtk = std::fs::read_to_string(dir.join("surface_flow.vtk")).unwrap();
        let written: usize = vtk.trim().rsplit(' ').next().unwrap().parse().unwrap();
        assert_eq!((written + 1) % crate::su2cfg::OUTPUT_FREQ, 0);
        assert!(written >= e.opt.min_iterations && rows > written + 1, "written {written}, rows {rows}");
    }

    #[test]
    fn euler_early_stop_keeps_the_restart_file_for_the_warm_successor() {
        let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
        let mut e = env("euler_slow", "slow");
        e.opt.machs = vec![0.5];
        e.opt.alphas_deg = vec![0.0, 4.0];
        e.opt.iterations = 1000;
        e.opt.convergence = 1e-30; // only the early stop ends the runs
        let out = run_all(&e.setup(), &|_| {}).unwrap();
        for o in &out {
            let r = o.result.as_ref().unwrap_or_else(|| panic!("{:?}", o.error));
            assert!(!r.converged && r.iterations < 1000, "{r:?}");
        }
        // The successor started from the restart file the predecessor wrote at its last periodic output (a
        // multiple of OUTPUT_WRT_FREQ = 50 iterations, after the stop decision at 150 iterations or later).
        let sols = std::fs::read_to_string(e.dir.join("solutions")).unwrap();
        let line = sols.lines().find(|l| l.starts_with("0.500000 4.000000 ")).unwrap_or_else(|| panic!("{sols}"));
        assert!(line.contains(" ../m0.500_a00.00/restart.dat restart "), "{line}");
        let it: usize = line.rsplit(' ').next().unwrap().parse().unwrap();
        assert!((it + 1).is_multiple_of(crate::su2cfg::OUTPUT_FREQ) && it >= 149, "{line}");
        assert!(out[1].warm);
        // The early-stopped case keeps its surface file of the same iteration.
        let vtk = std::fs::read_to_string(out[1].spec.dir(&e.dir.join("work")).join("surface_flow.vtk")).unwrap();
        assert!(vtk.starts_with("surface "), "{vtk}");
    }

    #[test]
    fn rans_startup_divergence_is_retried_conservatively_then_fails() {
        let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
        let stages = |e: &Env| std::fs::read_to_string(e.dir.join("stages")).unwrap_or_default();
        for mode in ["sdiverge", "sabort"] {
            let mut e = env_rans(&format!("rans_{mode}"), mode);
            e.opt.machs = vec![2.0];
            e.opt.alphas_deg = vec![0.0, 4.0];
            let out = run_all(&e.setup(), &|_| {}).unwrap();
            let r = out[0].result.as_ref().unwrap_or_else(|| panic!("{mode}: {:?} {}", out[0].error, stages(&e)));
            assert!(r.converged, "{mode}: {r:?}");
            assert!(
                r.warnings
                    .iter()
                    .any(|w| w.contains("first-order startup diverged") && w.contains("capped at 20 instead of 200 (the SA equation stays at 0.2")),
                "{mode}: {r:?}"
            );
            // startup, startup again (conservative), then the second-order stage with the conservative settings
            let st = stages(&e);
            let a0: Vec<&str> = st.lines().filter(|l| l.starts_with("2.000000 0.000000")).collect();
            assert_eq!(
                a0,
                ["2.000000 0.000000 history_startup cons=0", "2.000000 0.000000 history_startup cons=1", "2.000000 0.000000 history cons=1"],
                "{mode}"
            );
            assert!(out[1].warm && out[1].result.as_ref().unwrap().converged);
        }
        let mut e = env_rans("rans_sdiverge2", "sdiverge2");
        e.opt.machs = vec![2.0];
        e.opt.alphas_deg = vec![0.0];
        let out = run_all(&e.setup(), &|_| {}).unwrap();
        let msg = out[0].error.as_ref().unwrap();
        assert!(msg.contains("first-order startup diverged twice") && msg.contains("SA residual rose"), "{msg}");
        assert_eq!(stages(&e).lines().count(), 2);
    }

    #[test]
    fn euler_runs_past_the_target_until_the_coefficient_trend_has_settled() {
        let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
        // CFz still falls by 3e-5 per iteration (0.2 % of the force scale over 20 iterations) when the residual
        // reaches the 1e-6 target at row 200, and is flat from row 300: the case runs on and is accepted once the
        // trend has settled. The rows are few so that the runner, which looks at the history once a second, stops the
        // mock well before its last row even on a machine on which the mock writes rows quickly.
        let mut e = env("euler_trend", "trend");
        e.opt.machs = vec![0.8];
        e.opt.alphas_deg = vec![0.0, 4.0];
        e.opt.alpha_mode = AlphaMode::Single;
        e.opt.iterations = 1000;
        let out = run_all(&e.setup(), &|_| {}).unwrap();
        let r = out[1].result.as_ref().unwrap_or_else(|| panic!("{:?}", out[1].error));
        assert!(r.converged && r.warnings.is_empty(), "{r:?}");
        assert!(r.iterations > 300 && r.iterations < 1000, "{}", r.iterations);
        assert!((r.coeffs.cn - 0.2).abs() < 1e-6, "{}", r.coeffs.cn);
        let _ = std::fs::remove_dir_all(&e.dir);
        // Still trending at the iteration limit: accepted with the residual target reached, but not converged, with
        // a warning.
        let mut e = env("euler_trendlimit", "trendlimit");
        e.opt.machs = vec![0.8];
        e.opt.alphas_deg = vec![0.0, 4.0];
        e.opt.alpha_mode = AlphaMode::Single;
        e.opt.iterations = 400;
        let out = run_all(&e.setup(), &|_| {}).unwrap();
        let r = out[1].result.as_ref().unwrap_or_else(|| panic!("{:?}", out[1].error));
        assert!(!r.converged && r.iterations == 400, "{r:?}");
        assert!(r.warnings.iter().any(|w| w.contains("still trend") && w.contains("averaged over the last 100 iterations")), "{r:?}");
        // Not settled: the window average (rows 300..399, CFz = 0.2 + 3e-5 (600 - i)), not the last row (0.20603).
        assert!((r.coeffs.cn - (0.2 + 3e-5 * 250.5)).abs() < 1e-6, "{}", r.coeffs.cn);
        let _ = std::fs::remove_dir_all(&e.dir);
    }
}
