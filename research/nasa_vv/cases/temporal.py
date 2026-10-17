# SPDX-License-Identifier: GPL-3.0-or-later
"""Temporal convergence of the 6-DOF flight integrator (spec 6.5).

The CFD is steady, so the tutorial's temporal convergence does not apply to it. The
time-dependent part of IgnisYeet is the flight integrator, and the step study is done
there: ``examples/sample.toml`` with ``sim.integrator = "rk4"`` and ``sim.dt`` halved
from 0.008 s to 0.001 s, so the refinement ratio is exactly 2 and ``constant_ratio``
applies.

Configuration keys, as spelled by the product:

- ``sim.integrator`` (``"rk4"`` is the fixed-step method) and ``sim.dt`` [s]:
  crates/cli/src/config.rs:378-384, defaults ``rk4`` and 0.002 at :403-405, range
  1e-5 s to 0.05 s at :707-713.
- ``sim.max_time`` [s] (config.rs:390): a flight still airborne then ends there, the
  step cut to land on it exactly (flight.rs:1267-1279), and ``sim`` exits with status 3.
  That gives the state at a fixed time without an event.

Run directories under the work directory (one product run each)::

    <case>_dt<dt>        the whole flight; summary.json, trajectory.csv
    <case>_dt<dt>_t10    the same with sim.max_time = 10; trajectory.csv ends at t = 10 s

``<case>`` is ``sample`` (examples/sample.toml unchanged) or one of three diagnostics on a modified
copy of sample_motor.eng (CASE_DTS): ``noramp``, ``first300`` and ``nested``; the first thrust sample is the
only change, and the copy is written by code.

Quantities, from summary.json at full precision (events) and from trajectory.csv at the
resolution it is written with (fixed time):

- ``apogee_m``, ``apogee_time_s``, ``landing_distance_m``, ``flight_time_s``,
  ``rail_exit_time_s``, ``rail_exit_speed_ms``: events located inside a step.
- ``alt_m_at_t1``, ``airspeed_ms_at_t1``, ``alt_m_at_t10``, ``airspeed_ms_at_t10``:
  altitude above sea level and airspeed at t = 1 s and t = 10 s, times every step
  reaches exactly, so no event handling enters the quantity itself. Limited by the
  rounding of trajectory.csv.
- ``height_m_at_t10``, ``range_m_at_t10``: height above the launch site and horizontal
  distance from it at t = 10 s, at full precision. The run is cut at ``sim.max_time = 10``
  while the rocket still climbs, and summary.json's apogee entries then hold the state of
  the last step, which ends exactly at 10 s.
"""
from __future__ import annotations

import csv
import json
import math
import pathlib
import subprocess

from ..gci import GridConvergence, constant_ratio
from .existing_data import ROOT, _last_commit

RESULTS = pathlib.Path(__file__).resolve().parents[1] / "results" / "temporal.csv"
WORKDIR = pathlib.Path.home() / ".cache" / "nasa_vv_t17_dt"

BINARY = ROOT / "target" / "release" / "ignisyeet"
CONFIG = ROOT / "examples" / "sample.toml"
MOTOR = ROOT / "examples" / "sample_motor.eng"

DTS = (0.008, 0.004, 0.002, 0.001)
"""The sweep, coarse to fine. Each is half the one before: r = 2."""
R = 2.0
T_FIXED = (1.0, 10.0)
"""Times [s] of the fixed-time quantities. 1 s is a thrust-curve knot, 10 s is sim.max_time."""
NESTED_DTS = (0.01, 0.005, 0.0025, 0.00125)
"""The sweep of the ``nested`` control: every step divides every knot of the sample thrust curve."""
CASE_DTS = {"sample": DTS, "noramp": DTS, "first300": DTS, "nested": NESTED_DTS}
CASES = tuple(CASE_DTS)
"""``sample`` is the product's sample. The others are diagnostics on a modified copy of its motor file:
``noramp`` (first sample 2300 N: no hold on the rail and no ramp), ``first300`` (first sample 300 N, above
the 11 kg rocket's weight of about 108 N on the rail: no hold, ramp kept) and ``nested`` (the ``noramp`` motor
with a sweep whose steps all divide the knots)."""
PREFIX = {"sample": "", "noramp": "noramp_", "first300": "first300_", "nested": "nested_"}
FIRST300_N = 300.0
FORMAL_ORDER = 4.0
ORDER_TOLERANCE = 1.0
ROUNDOFF = 1e-11
"""Nominal: a relative difference below this is taken as round-off (double precision accumulated over
1e5 to 1e6 steps). It is not the observed floor, which is much higher (NOISE_FLOOR)."""
NOISE_FLOOR = 1e-9
"""Observed in the controls: the finest triplets of the full-precision quantities stop converging and
oscillate at about this relative size, also on the fully nested sweep. The cause is not identified. A
difference below it carries no information on the order."""

COLUMNS = ["quantity", "dt1", "dt2", "dt3", "f1", "f2", "f3", "order", "extrapolated",
           "gci_fine_pct", "asymptotic_ratio", "note"]

EVENT_KEYS = {"apogee_m": "apogee", "apogee_time_s": "apogee_time",
              "landing_distance_m": "landing_distance", "flight_time_s": "landing_time",
              "rail_exit_time_s": "rail_exit_time", "rail_exit_speed_ms": "rail_exit_speed"}
FIXED_COLUMNS = {"alt_m": ("alt", 1e-3), "airspeed_ms": ("airspeed", 1e-4)}
"""trajectory.csv column and its resolution (the product writes alt with 3 and airspeed with 4 decimals)."""
FULL_PRECISION_T10 = {"height_m": "height", "range_m": "range"}
"""Height above the launch site and horizontal range at t = 10 s, read from the summary.json of the
run cut at sim.max_time = 10 s, so at full precision."""


# ------------------------------------------------------------------ the runs

def run_dir(workdir, case: str, dt: float, t10: bool = False) -> pathlib.Path:
    return pathlib.Path(workdir) / f"{case}_dt{dt:g}{'_t10' if t10 else ''}"


def thrust_times(eng: pathlib.Path) -> list[float]:
    """Times of the thrust samples of an .eng file (comment lines start with ';')."""
    lines = [ln.split() for ln in pathlib.Path(eng).read_text().splitlines()
             if ln.strip() and not ln.lstrip().startswith(";")]
    return [float(ln[0]) for ln in lines[1:]]


def non_dividing(times: list[float], dts) -> dict[float, list[float]]:
    """For each dt, the thrust-curve times that are not a whole number of steps."""
    def divides(t, dt):
        n = t / dt
        return abs(n - round(n)) < 1e-9
    return {dt: [t for t in times if not divides(t, dt)] for dt in dts}


def write_noramp_motor(src: pathlib.Path, dst: pathlib.Path) -> pathlib.Path:
    """Copy of an .eng file whose first sample has the thrust of the second.

    The sample motor starts at 0 N and reaches 2300 N at 0.05 s; the 11 kg rocket is held
    on the rail until the thrust passes its weight. Without that ramp it leaves at once.
    """
    return write_first_sample_motor(src, dst, None)


def write_first_sample_motor(src: pathlib.Path, dst: pathlib.Path, thrust: float | None) -> pathlib.Path:
    """Copy of an .eng file with the thrust of the first sample replaced (nothing else changed).

    ``thrust`` None takes the thrust of the second sample (no ramp). A value above the weight of
    the rocket on the rail removes the hold at rest but keeps a ramp up to the second sample.
    """
    lines = pathlib.Path(src).read_text().splitlines()
    data = [i for i, ln in enumerate(lines) if ln.strip() and not ln.lstrip().startswith(";")]
    first, second = data[1], data[2]
    value = lines[second].split()[1] if thrust is None else f"{thrust:g}"
    lines[first] = f"{lines[first].split()[0]} {value}"
    dst = pathlib.Path(dst)
    dst.write_text("\n".join(lines) + "\n")
    return dst


def run_sweep(workdir=WORKDIR, cases=CASES) -> pathlib.Path:
    """Run the product for every case and step. Needs target/release/ignisyeet."""
    workdir = pathlib.Path(workdir)
    workdir.mkdir(parents=True, exist_ok=True)
    version = subprocess.run([str(BINARY), "--version"], capture_output=True, text=True, check=True)
    (workdir / "binary_version.txt").write_text(version.stdout.splitlines()[0] + "\n")
    noramp = write_noramp_motor(MOTOR, workdir / "sample_motor_noramp.eng")
    first300 = write_first_sample_motor(MOTOR, workdir / "sample_motor_first300.eng", FIRST300_N)
    motors = {"noramp": noramp, "nested": noramp, "first300": first300}
    for case in cases:
        for dt in CASE_DTS[case]:
            for t10 in (False, True):
                cmd = [str(BINARY), "--no-progress", "-q", "sim", str(CONFIG),
                       "--set", 'sim.integrator="rk4"', "--set", f"sim.dt={dt:g}",
                       "--out", str(run_dir(workdir, case, dt, t10))]
                if t10:
                    cmd[-2:-2] = ["--set", "sim.max_time=10.0"]
                if case in motors:
                    cmd[-2:-2] = ["--set", f'motor.eng="{motors[case]}"']
                r = subprocess.run(cmd, capture_output=True, text=True)
                # a run cut at sim.max_time exits with status 3 by design
                expected = 3 if t10 else 0
                if r.returncode != expected:
                    raise RuntimeError(f"{' '.join(cmd)} exited with {r.returncode}:\n{r.stderr}")
    return workdir


# ------------------------------------------------------------------ readers

def read_summary(run: pathlib.Path) -> dict[str, float]:
    path = pathlib.Path(run) / "summary.json"
    data = json.loads(path.read_text())
    if not data.get("landed", False):
        raise ValueError(f"{path}: the flight did not land")
    out = {}
    for key in EVENT_KEYS.values():
        if key not in data:
            raise ValueError(f"{path}: no key {key}")
        out[key] = float(data[key])
    return out


def read_state_at(run: pathlib.Path, t: float) -> dict[str, float]:
    """Altitude and airspeed of the trajectory.csv row at time t (written with 4 decimals)."""
    path = pathlib.Path(run) / "trajectory.csv"
    with path.open(newline="") as f:
        for row in csv.DictReader(f):
            if abs(float(row["t"]) - t) < 5e-5:
                return {"alt": float(row["alt"]), "airspeed": float(row["airspeed"])}
    raise ValueError(f"{path}: no row at t = {t:g}")


def read_state_at_max_time(run: pathlib.Path, t: float) -> dict[str, float]:
    """Height above the launch site and horizontal range at t, of a flight cut at sim.max_time = t.

    While the rocket climbs, summary.json's ``apogee`` entries follow the latest step, and the
    last step ends exactly on max_time, so they hold the state at t at full precision. They do
    so only while the rocket has not passed its apogee, which ``apogee_time`` shows.
    """
    path = pathlib.Path(run) / "summary.json"
    data = json.loads(path.read_text())
    if data.get("landed", True):
        raise ValueError(f"{path}: the flight landed; it was not cut at max_time")
    if abs(float(data["apogee_time"]) - t) > 1e-9:
        raise ValueError(f"{path}: apogee_time {data['apogee_time']} is not {t:g}: "
                         "the rocket was not still climbing at max_time")
    return {"height": float(data["apogee"]),
            "range": math.hypot(float(data["apogee_east"]), float(data["apogee_north"]))}


# ------------------------------------------------------------------ rows

def _g(x: float | None, spec: str = ".6g") -> str:
    return "" if x is None else format(x, spec)


def _notes(g: GridConvergence, f: tuple[float, float, float], resolution: float | None) -> str:
    notes = [g.note] if g.note else []
    scale = max(abs(f[0]), abs(f[1]), abs(f[2]), 1e-300)
    if resolution is None and max(abs(g.e21), abs(g.e32)) / scale < ROUNDOFF:
        notes.append("the differences are at round-off")
    if resolution is not None and min(abs(g.e21), abs(g.e32)) <= 2 * resolution:
        notes.append(f"a difference is within 2 units of the output resolution of trajectory.csv "
                     f"({resolution:g}); the rounding of the file limits this order")
    if resolution is None and min(abs(g.e21), abs(g.e32)) / scale < NOISE_FLOOR:
        notes.append(f"a difference is below the relative floor of about {NOISE_FLOOR:g} at which the finest "
                     "triplets of the controls stop converging (cause unidentified); an order from it is not "
                     "evidence of the formal order")
    if g.order is not None and abs(g.order - FORMAL_ORDER) > ORDER_TOLERANCE:
        notes.append(f"observed order p = {g.order:.3g} differs from the formal order {FORMAL_ORDER:g} "
                     f"of rk4 by more than {ORDER_TOLERANCE:g}")
    return "; ".join(notes)


def _row(name: str, dts: tuple[float, float, float], f: tuple[float, float, float],
         resolution: float | None) -> dict[str, str]:
    g = constant_ratio(f[0], f[1], f[2], R)
    return {
        "quantity": name, "dt1": f"{dts[0]:g}", "dt2": f"{dts[1]:g}", "dt3": f"{dts[2]:g}",
        "f1": _g(f[0], ".15g"), "f2": _g(f[1], ".15g"), "f3": _g(f[2], ".15g"),
        "order": _g(g.order), "extrapolated": _g(g.extrapolated, ".15g"),
        "gci_fine_pct": _g(None if g.gci_fine is None else g.gci_fine * 100),
        "asymptotic_ratio": _g(g.asymptotic_ratio),
        "note": _notes(g, f, resolution),
    }


def triplets(dts=DTS) -> list[tuple[float, float, float]]:
    """Finest first. The finest three steps, then the coarsest three (the stability check)."""
    fine_to_coarse = sorted(dts)
    return [tuple(fine_to_coarse[i:i + 3]) for i in range(len(fine_to_coarse) - 2)]


def _require(path: pathlib.Path) -> pathlib.Path:
    if not path.is_dir():
        raise FileNotFoundError(f"run directory {path} does not exist; run temporal.run_sweep() first")
    return path


def case_rows(workdir: pathlib.Path, case: str) -> list[dict[str, str]]:
    prefix = PREFIX[case]
    dts = CASE_DTS[case]
    events = {dt: read_summary(_require(run_dir(workdir, case, dt))) for dt in dts}
    fixed = {}
    for t in T_FIXED:
        for dt in dts:
            fixed[t, dt] = read_state_at(_require(run_dir(workdir, case, dt, t10=(t == 10.0))), t)
    cut = {dt: read_state_at_max_time(_require(run_dir(workdir, case, dt, t10=True)), 10.0) for dt in dts}
    rows = []
    for tri in triplets(dts):
        for name, key in EVENT_KEYS.items():
            rows.append(_row(prefix + name, tri, tuple(events[dt][key] for dt in tri), None))
        for t in T_FIXED:
            for stem, (col, res) in FIXED_COLUMNS.items():
                rows.append(_row(f"{prefix}{stem}_at_t{t:g}", tri,
                                 tuple(fixed[t, dt][col] for dt in tri), res))
        for stem, key in FULL_PRECISION_T10.items():
            rows.append(_row(f"{prefix}{stem}_at_t10", tri, tuple(cut[dt][key] for dt in tri), None))
    return rows


# ------------------------------------------------------------------ output

def _header(workdir: pathlib.Path, cases: list[str]) -> list[str]:
    version_file = workdir / "binary_version.txt"
    version = version_file.read_text().strip().split(" — ")[0] if version_file.is_file() else "not recorded"
    bad = non_dividing(thrust_times(MOTOR), DTS)
    knots = "; ".join(f"dt = {dt:g} s does not divide {', '.join(f'{t:g}' for t in ts)}"
                      for dt, ts in bad.items() if ts) or "every dt divides every knot"
    src = [f"  {rel} @ {_last_commit(rel)}" for rel in (
        "crates/cli/src/config.rs", "crates/sim/src/flight.rs", "crates/sim/src/stepper.rs",
        "examples/sample.toml", "examples/sample_motor.eng")]
    control_lines = [] if cases == ["sample"] else [
        "Controls (rows with a prefix) are diagnostics on a modified input, not the product's sample. Each is the same sweep with a "
        "copy of examples/sample_motor.eng written to the work directory by code (first thrust sample changed, nothing else); "
        "the product and examples/ are untouched.",
        "noramp_: first sample 2300 N. Removes both the hold on the rail and the 0 to 0.05 s ramp. "
        "first300_: first sample 300 N, above the 11 kg rocket's weight of about 108 N on the rail, ramp 300 to 2300 N kept. "
        "Removes only the hold. nested_: the noramp motor with dt = 0.01, 0.005, 0.0025, 0.00125 s, every dt dividing every knot "
        "(no cut steps at the knots).",
        "What they show: the sample's rail-exit speed differs by 4.5e-4 m/s between dt 0.008 and 0.004 and every sample row is refused. "
        "With only the hold removed (first300_) that difference is 4.1e-8 m/s, which supports the hold as the main cause of the rail-exit scatter. "
        "The first300_ coarse-triplet apogee is still non-monotone (0.004 minus 0.002: -1.1e-4 m; 0.008 minus 0.004: +4.2e-4 m), so a second, smaller contribution "
        "is not excluded. With hold and ramp removed (noramp_) the coarse triplet gives p = 4.01 for apogee_m and 3.96 for flight_time_s "
        "(range at t = 10 s: 5.33, flagged).",
        "The controls' finest triplets oscillate or stop converging at about 1e-9 to 1e-8 relative (1e-5 m on apogee_m). "
        "The floor persists on the fully nested sweep (nested_), so non-nesting does not explain it. Its cause is unidentified.",
        "The one order from a finest triplet, noramp_apogee_time_s with p = 4.75, has differences (2.2e-9 s and 6.0e-8 s) below that floor, "
        "and the same runs' apogee_m oscillates; it is not evidence of order 4.",
    ]
    lines = [
        "Temporal convergence of the 6-DOF flight integrator of IgnisYeet (spec 6.5).",
        "The CFD is steady, so the tutorial's temporal convergence does not apply to it: "
        "there is no time step in the SU2 runs. This study applies the step to the 6-DOF integrator instead.",
        f"Product: {version} (target/release/ignisyeet). Sources and the last commit that changed each:",
        *src,
        "Runs: examples/sample.toml with sim.integrator = \"rk4\" and sim.dt = 0.008, 0.004, 0.002, 0.001 s "
        "(crates/cli/src/config.rs:378-384), parachute descent, wind 4 m/s. Refinement ratio r = 2, "
        "constant_ratio (NASA tutorial form) with Fs = 1.25.",
        "dt1 is the finest step of a triplet and f1 its value. Each quantity has two triplets: "
        "dt 0.001/0.002/0.004 (the finest three, the result) and dt 0.002/0.004/0.008 (the coarsest three, a stability check).",
        "gci_fine_pct is relative to f1 (percent). asymptotic_ratio is r^p GCI21/GCI32; near 1 means the asymptotic range.",
        "Event quantities (summary.json, full precision): apogee_m, apogee_time_s, landing_distance_m, flight_time_s (= landing time), rail_exit_time_s, rail_exit_speed_ms.",
        "Fixed-time quantities (trajectory.csv, resolution 1e-3 m for alt and 1e-4 m/s for airspeed): "
        "alt_m and airspeed_ms at t = 1 s (a thrust-curve knot) and t = 10 s (the run is cut at sim.max_time = 10 s, "
        "crates/cli/src/config.rs:390; sim exits with status 3 by design). Every step reaches these times exactly.",
        "height_m_at_t10 and range_m_at_t10 are the height above the launch site and the horizontal distance from it at t = 10 s at full precision: "
        "while the rocket still climbs, the apogee entries of summary.json of the run cut at 10 s hold the state of the last step, which ends at 10 s "
        "(apogee_time = 10 s is checked).",
        "How events are located: rail exit, apogee, main deployment and landing are found inside the step on a cubic Hermite "
        "interpolant of the position (bisection or the zero of the interpolated vertical velocity: "
        "crates/sim/src/flight.rs:1357, :1388, :1414; crates/sim/src/stepper.rs:5-20), so they do not snap to the step. "
        "The parachute opens at apogee time + delay exactly. Steps are cut to end on the thrust-curve knots, "
        "sim.max_time and the deployment time (flight.rs:1267-1279).",
        f"Thrust-curve knots of examples/sample_motor.eng: {knots}. For those the step is cut short to end on the knot "
        "(it is not skipped), so the step grids of the four runs are not nested there; the apogee-delay deployment time "
        "is another such cut. The finest triplet (0.004, 0.002, 0.001) is affected through 0.004 (0.05/0.004 = 12.5). "
        "The nested control (below) shows this is not the dominant cause of the oscillation.",
        "Lift-off, attributed on the evidence of the controls below: the sample motor starts at 0 N and reaches 2300 N at 0.05 s. "
        "The rail model holds the rocket at rest while the thrust is below its weight (a = 0 for v <= 0, "
        "crates/sim/src/flight.rs:826), which puts a kink in the acceleration at about 2.3 ms that the integrator does not locate. "
        "It falls in the first step for dt = 0.008 and 0.004, the second for 0.002 and the third for 0.001, "
        "so it sits differently against the step ends in each run.",
        "Rail exit: rail_exit_time_s and rail_exit_speed_ms are in summary.json at full precision and have their own rows.",
        *control_lines,
        "A note flags an observed order more than 1 away from the formal order 4, a difference within 2 units of the "
        "trajectory.csv resolution (fixed-time rows), a difference at round-off (event and t = 10 s summary rows: relative below "
        f"{ROUNDOFF:g}, a nominal threshold, not the observed floor), and a full-precision difference below {NOISE_FLOOR:g} relative, "
        "the level at which the later-flight quantities of the controls stop converging (cause unidentified). "
        "A refused row (constant_ratio's screen) carries the reason in note and has no order, extrapolated value or GCI.",
        "This is verification of the integrator by step refinement. It is not a comparison with measured data.",
    ]
    return ["# " + ln for ln in lines]


def main(out: pathlib.Path | str | None = None, workdir: pathlib.Path | str | None = None) -> pathlib.Path:
    out = pathlib.Path(out) if out is not None else RESULTS
    workdir = pathlib.Path(workdir) if workdir is not None else WORKDIR
    cases = [c for c in CASES if c == "sample" or run_dir(workdir, c, CASE_DTS[c][0]).is_dir()]
    rows = []
    for case in cases:
        rows += case_rows(workdir, case)
    out.parent.mkdir(parents=True, exist_ok=True)
    with out.open("w", newline="") as f:
        for ln in _header(workdir, cases):
            f.write(ln + "\n")
        w = csv.DictWriter(f, fieldnames=COLUMNS, lineterminator="\n")
        w.writeheader()
        w.writerows(rows)
    return out


if __name__ == "__main__":
    print(main())
