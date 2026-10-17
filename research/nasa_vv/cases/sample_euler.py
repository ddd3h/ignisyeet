# SPDX-License-Identifier: GPL-3.0-or-later
"""Grid convergence of the sample rocket's Euler solution on a new three-level grid family.

The CFD itself is run with ``ignisyeet aero`` on a case.toml per level (doc/scripts/rans_sample.py
``setup`` writes it; the work directory is $HOME/.cache/nasa_vv_sample); this module only collects the output of those runs from a
work directory and writes two CSVs. Collecting is deterministic: the same work directory gives
byte-identical files, and running it needs no solver.

Work directory layout (one sub-directory per level, each holding a fetched ``ignisyeet aero`` run)::

    coarse/  medium/  fine/  medium_nomirror/
        case.toml                  aero.cfd.wall_size, fin_size, z_mirror_mesh, ranks_per_case
        done.json                  the job meta of virgo_run.sh (node, ppn, wall_seconds, ...)
        job.log                    its ``binary: <path>`` line is the evidence of the binary that ran
        out/cfd_cases.csv          one row per solved case, plus the ``combined`` rows
        out/cfd_report.json        mesh_nodes, mesh_cells
        out/cfd/<case>/done.json   mach, alpha_deg of the case
        out/cfd/<case>/history.csv SU2 history (rms[Rho])

Levels
------
============== ========= ========= ================================================
level          wall_size fin_size  note
============== ========= ========= ================================================
coarse         3.92 mm   1.96 mm
medium         2.80 mm   1.40 mm
fine           2.00 mm   1.00 mm
medium_nomirror 2.80 mm  1.40 mm   z_mirror_mesh = false; not part of the family
============== ========= ========= ================================================

Grids made by ``ignisyeet aero`` cannot be geometrically similar. The mesher fixes the outer cell
size at the farfield radius / 10 (crates/cfd/src/mesh.rs:396) and grades the volume from
``wall_size`` up to it (crates/cfd/src/mesh_gmsh.py:153), so refining ``wall_size`` and
``fin_size`` refines only the near-wall region. ``GridFamily.check_similar`` sees only the knobs,
which do scale together by 1.4; it cannot see this. Every GCI is therefore computed twice, with
h = nodes^(-1/3) (what ``study`` uses) and with h from the ``wall_size`` knob, and the spread
between the two is part of the uncertainty.

Quantities
----------
``cna``        CNa = (CN(4) - CN(-4)) / 2 / radians(4) from the raw ``cn_raw`` values where both
               signs were solved, CN(4) / radians(4) otherwise. The family levels use the second
               form (the z-mirrored mesh skips -4). ``cna_basis`` says which. ``cn_raw`` is used
               rather than the ``cn`` of the ``combined`` row because that row is CN(4) - CN(0)
               on a mirrored mesh and the odd part only when both signs exist.
``ca_alpha0``  ``ca_cfd`` at alpha = 0.
``xcp``        ``xcp_m`` of the ``combined`` row at alpha = 4: the product's own centre of
               pressure, from the combined normal force and moment.

A level's verdict is the AND over the solves its quantity uses (cna: alpha 4; ca_alpha0: alpha 0;
xcp: alpha 4 and alpha 0, because
the combined row subtracts the alpha = 0 offset). Status strings are mapped by
``existing_data._STATUS`` (shared with the committed studies): ``converged`` is True; ``accepted``
(the runner's early stop after at least 3 orders with flat coefficients) is None, so the recorded
residual drop decides through ``check_converged``; ``unconverged`` (limit cycle) and ``failed`` are
False; any other string is an error. ``residual_drop_orders`` is the largest ``rms[Rho]`` of the
case's history minus the residual at the judged row (``residual_last``). Neither gate is weakened:
``family.py`` refuses a level that is not converged.

The inputs (``case.toml`` per level) are committed in ``sample_euler_inputs/``; the header of
``results/gci_sample_euler.csv`` lists how they were made, every hand edit, the binary and its
evidence, and the sizing numbers. Nothing the CSVs state depends on a file outside the work
directory and the repository.
"""
from __future__ import annotations

import argparse
import csv
import json
import math
import pathlib
import tomllib
from dataclasses import dataclass, field

from ..family import GridFamily, GridLevel, study
from ..gci import GridConvergence, grid_convergence  # noqa: F401  (grid_convergence used via existing_data)
from .existing_data import (COLUMNS as GCI_COLUMNS, FamilyData, _last_commit, _rows, _verdict,
                            home_to_tilde,
                            _wall_size_compute)

RESULTS = pathlib.Path(__file__).resolve().parents[1] / "results"
LEVELS_CSV = RESULTS / "sample_euler_levels.csv"
GCI_CSV = RESULTS / "gci_sample_euler.csv"
DEFAULT_WORK = pathlib.Path.home() / ".cache" / "nasa_vv_sample"

FAMILY_LEVELS = ("fine", "medium", "coarse")
"""Directory names of the family, finest first."""
NOMIRROR_LEVEL = "medium_nomirror"
ALPHA = 4.0

LEVEL_COLUMNS = [
    "level", "machine", "ppn", "ranks_per_case", "mesh", "wall_size_mm", "fin_size_mm", "nodes",
    "cells", "mach", "status_a0", "status_a4", "status_am4", "drop_a0", "drop_a4", "drop_am4",
    "cn_raw_a0", "cn_raw_a4", "cn_raw_am4", "cna", "cna_basis", "cn_combined_a4", "xcp_m",
    "ca_alpha0", "history_gap_a0", "history_gap_a4", "history_gap_am4", "source_check", "job_wall_s",
]

def cna(cn_plus: float, cn_minus: float | None) -> float:
    """CNa [1/rad]: half the difference over radians(4) with both signs, else CN(4) / radians(4)."""
    rad = math.radians(ALPHA)
    if cn_minus is None:
        return cn_plus / rad
    return (cn_plus - cn_minus) / 2.0 / rad


@dataclass
class Solve:
    status: str
    iterations: int
    cn_raw: float | None
    ca: float | None
    residual_drop_orders: float | None
    wall_s: float | None
    history_gap: int | None = None
    """Rows of history.csv after the judged row (0 when the history ends where the runner judged)."""


@dataclass
class Level:
    name: str
    wall_m: float
    fin_m: float
    mirror: bool
    ranks_per_case: int
    nodes: int
    cells: int
    machine: str
    ppn: int | None
    job_wall_s: float | None
    commit: str
    """ignisyeet_commit of the job record: the remote checkout's COMMIT file, NOT the binary's source."""
    binary: str
    binary_line: str
    """The ``binary: <path>`` line of job.log, the evidence of the binary that ran."""
    cfg_machs: list[float]
    solves: dict[tuple[float, float], Solve] = field(default_factory=dict)
    combined: dict[tuple[float, float], dict[str, float | None]] = field(default_factory=dict)
    problems: list[str] = field(default_factory=list)
    """Why the files of this level cannot be trusted (truncated, stale, incomplete); empty when they pass."""

    def machs(self) -> list[float]:
        return sorted({m for m, _ in self.solves} | set(self.cfg_machs))


def _opt(row: dict[str, str], col: str) -> float | None:
    s = row.get(col, "").strip()
    return float(s) if s else None


HISTORY_TOLERANCE = 1e-6
"""Largest difference between residual_last of cfd_cases.csv and the history row it names, in log10 units."""


FORCE_WINDOW_ROWS = 120
"""Rows up to the judged row among which the recorded axial force must lie: a converged case takes the CFx of the
judged row itself, an accepted or window-averaged case the mean over a window of its last rows."""


def _check_history(path: pathlib.Path, iterations: int, residual_last: float | None,
                   ca: float | None) -> tuple[float | None, int | None, str]:
    """Peak rms[Rho], the rows after the judged row, and the reason the file is unusable ('' if fine).

    The runner stops SU2 early and SU2 keeps writing until its next periodic output, so the history can run
    past the row the case is judged on; cfd_cases.csv records the residual and the force coefficients of that
    judged row, not of the last one. The file is checked, not trusted: it must end in a newline, every row must
    be complete and finite, it must hold at least ``iterations`` rows (a shorter file is older than the case
    record), a row must exist whose rms[Rho] equals residual_last of cfd_cases.csv (that row is the judged one),
    and ca_cfd must lie within the CFx of the rows up to it (it is that row's CFx, or a window mean of the rows
    before it). The rows written after the judged row are counted as the gap and reported, not hidden.
    """
    if not path.exists():
        return None, None, f"{path.name} not found"
    text = path.read_text()
    if not text.endswith("\n"):
        return None, None, f"{path.name} does not end in a newline (truncated)"
    rows = list(csv.reader(text.splitlines()))
    head = [h.strip().strip('"') for h in rows[0]]
    for need in ("rms[Rho]", "CFx"):
        if need not in head:
            return None, None, f"{path.name} has no {need} column"
    col, cx = head.index("rms[Rho]"), head.index("CFx")
    vals, forces = [], []
    for i, r in enumerate(rows[1:], start=2):
        if len(r) != len(head):
            return None, None, f"{path.name} row {i} has {len(r)} of {len(head)} fields (truncated)"
        try:
            f = [float(x) for x in r]
        except ValueError:
            return None, None, f"{path.name} row {i} has a non-numeric field (truncated)"
        if not all(math.isfinite(x) for x in f):
            return None, None, f"{path.name} row {i} has a non-finite field"
        vals.append(f[col])
        forces.append(f[cx])
    if len(vals) < iterations:
        return None, None, (f"{path.name} has {len(vals)} rows but cfd_cases.csv records {iterations} "
                            "iterations (the history is older than the case record, or truncated)")
    judged = None
    if residual_last is not None and ca is not None:
        matches = [i for i, v in enumerate(vals, start=1) if abs(v - residual_last) <= HISTORY_TOLERANCE]
        if len(matches) > 1:
            return None, None, (f"{path.name} has {len(matches)} rows (the first are {matches[:3]}) within {HISTORY_TOLERANCE:g} "
                                f"of residual_last = {residual_last:.6g}, so the judged row is ambiguous")
        judged = matches[0] if matches else None
        if judged is None:
            return None, None, (f"{path.name} has no row with rms[Rho] = {residual_last:.6g}, the residual that "
                                "cfd_cases.csv records for the judged row")
        window = forces[max(0, judged - FORCE_WINDOW_ROWS):judged]
        slack = 1e-6 * abs(ca) + 1e-9
        if not (min(window) - slack <= ca <= max(window) + slack):
            return None, None, (f"{path.name}: ca_cfd = {ca:.6g} of cfd_cases.csv lies outside the CFx of the "
                                f"{len(window)} rows up to the judged row {judged} ({min(window):.6g} to {max(window):.6g})")
    return max(vals), len(vals) - (judged if judged is not None else len(vals)), ""


def _json(path: pathlib.Path, problems: list[str], required: bool = True) -> dict:
    if not path.exists():
        if required:
            problems.append(f"{path.name} not found")
        return {}
    try:
        return json.loads(path.read_text())
    except ValueError:
        problems.append(f"{path.name} is not valid JSON (truncated)")
        return {}


def read_level(d: pathlib.Path) -> Level:
    d = pathlib.Path(d)
    cases_path = d / "out" / "cfd_cases.csv"
    if not cases_path.exists():
        raise FileNotFoundError(f"level {d.name!r}: {cases_path} not found")
    problems: list[str] = []
    cfg = tomllib.loads((d / "case.toml").read_text())["aero"]["cfd"]
    rep = _json(d / "out" / "cfd_report.json", problems)
    job = _json(d / "done.json", problems, required=False)
    log = d / "job.log"
    binary_line = next((home_to_tilde(ln.strip()) for ln in log.read_text(errors="replace").splitlines()
                        if ln.startswith("binary: ")), "") if log.exists() else ""
    if job and (job.get("state") != "done" or job.get("exit_code") != 0):
        problems.append(f"done.json records state {job.get('state')!r} and exit code {job.get('exit_code')!r}, not a finished job with exit code 0")
    lv = Level(
        name=d.name, wall_m=float(cfg["wall_size"]), fin_m=float(cfg["fin_size"]),
        mirror=bool(cfg.get("z_mirror_mesh", True)), ranks_per_case=int(cfg.get("ranks_per_case", 0)),
        nodes=int(rep.get("mesh_nodes", 0)), cells=int(rep.get("mesh_cells", 0)),
        machine=str(job.get("node", "")), ppn=job.get("ppn"), job_wall_s=job.get("wall_seconds"),
        commit=str(job.get("ignisyeet_commit", "")), binary=str(job.get("binary", "")),
        binary_line=binary_line, cfg_machs=[float(m) for m in cfg.get("machs", [])], problems=problems)
    if lv.nodes <= 0 or lv.cells <= 0:
        problems.append("cfd_report.json has no positive mesh_nodes / mesh_cells")
    # case directories, found by the mach / alpha in their done.json
    dirs: dict[tuple[float, float], pathlib.Path] = {}
    for cd in sorted((d / "out" / "cfd").iterdir()):
        dj = cd / "done.json"
        if dj.exists():
            j = _json(dj, problems)
            if j and float(j.get("beta_deg", 0.0)) == 0.0:
                dirs[(float(j["mach"]), float(j["alpha_deg"]))] = cd
    text = cases_path.read_text()
    if not text.endswith("\n"):
        problems.append("cfd_cases.csv does not end in a newline (truncated)")
    lines = list(csv.reader(text.splitlines()))
    if len(lines) < 2:
        problems.append("cfd_cases.csv is empty or holds only its header")
        lines = lines or [[]]
    head = lines[0]
    for n, fields in enumerate(lines[1:], start=2):
        if len(fields) != len(head):
            problems.append(f"cfd_cases.csv row {n} has {len(fields)} of {len(head)} fields (truncated)")
            continue
        r = dict(zip(head, fields))
        try:
            key = (float(r["mach"]), float(r["alpha_deg"]))
            iterations = int(r["iterations"]) if r["status"] != "combined" else 0
            values = {c: _opt(r, c) for c in ("residual_last", "cn_raw", "ca_cfd", "wall_seconds", "cn", "xcp_m")}
        except (KeyError, ValueError):
            problems.append(f"cfd_cases.csv row {n} has a missing or non-numeric field (mach, alpha_deg, iterations or a value)")
            continue
        if r["status"] == "combined":
            lv.combined[key] = {"cn": values["cn"], "xcp_m": values["xcp_m"], "ca": values["ca_cfd"]}
            continue
        if r.get("beta_deg", "0") not in ("0", "0.0", "") or r.get("roll_hat", "0") not in ("0", "0.0", ""):
            continue
        last = values["residual_last"]
        drop = gap = None
        if r["status"] != "failed":
            where = f"mach {key[0]:g} alpha {key[1]:g}"
            for col in ("residual_last", "ca_cfd", "cn_raw"):
                if values[col] is None:
                    problems.append(f"{where}: {col} is blank in cfd_cases.csv for a case with status {r['status']!r}")
            if key not in dirs:
                problems.append(f"no case directory for {where}")
            elif last is not None and values["ca_cfd"] is not None:
                peak, gap, why = _check_history(dirs[key] / "history.csv", iterations, last, values["ca_cfd"])
                if why:
                    problems.append(f"{where}: {why}")
                else:
                    drop = peak - last
        lv.solves[key] = Solve(r["status"], iterations, values["cn_raw"], values["ca_cfd"], drop,
                               values["wall_seconds"], gap)
    if "cases" in rep and int(rep["cases"]) != len(lv.solves):
        problems.append(f"cfd_report.json records {rep['cases']} cases but cfd_cases.csv holds {len(lv.solves)} solves")
    machs = [float(m) for m in cfg.get("machs", [])]
    alphas = [float(a) for a in cfg.get("alphas_deg", [])]
    if not lv.mirror:
        alphas += [-a for a in alphas if a > 0]
    for m in machs:
        for a in alphas:
            if (m, a) not in lv.solves:
                problems.append(f"the solve at mach {m:g} alpha {a:g} is missing from cfd_cases.csv")
    for m in machs:
        if lv.solves.get((m, ALPHA)) is not None and lv.solves[(m, ALPHA)].status != "failed" \
                and (m, ALPHA) not in lv.combined:
            problems.append(f"the combined row at mach {m:g} alpha {ALPHA:g} is missing from cfd_cases.csv")
    return lv


def read_levels(work) -> dict[str, Level]:
    work = pathlib.Path(work)
    out = {}
    for name in (*FAMILY_LEVELS, NOMIRROR_LEVEL):
        d = work / name
        if not d.is_dir():
            raise FileNotFoundError(f"level {name!r}: directory {d} not found")
        out[name] = read_level(d)
    return out


# ------------------------------------------------------------------ rows


def _g(x, spec=".10g") -> str:
    return "" if x is None else format(x + 0.0, spec)


def level_rows(levels: dict[str, Level]) -> list[dict[str, str]]:
    rows = []
    for name in (*reversed(FAMILY_LEVELS), NOMIRROR_LEVEL):
        lv = levels[name]
        for m in lv.machs():
            s0, s4, sm = lv.solves.get((m, 0.0)), lv.solves.get((m, ALPHA)), lv.solves.get((m, -ALPHA))
            comb = lv.combined.get((m, ALPHA), {})
            cm = sm.cn_raw if sm else None
            value = cna(s4.cn_raw, cm) if s4 and s4.cn_raw is not None and (sm is None or cm is not None) else None
            rows.append({
                "level": name, "machine": lv.machine, "ppn": _g(lv.ppn), "ranks_per_case": str(lv.ranks_per_case),
                "mesh": "z-mirrored" if lv.mirror else "not mirrored",
                "wall_size_mm": _g(lv.wall_m * 1000), "fin_size_mm": _g(lv.fin_m * 1000),
                "nodes": str(lv.nodes), "cells": str(lv.cells), "mach": _g(m),
                "status_a0": s0.status if s0 else "", "status_a4": s4.status if s4 else "",
                "status_am4": sm.status if sm else "",
                "drop_a0": _g(s0.residual_drop_orders if s0 else None, ".6g"),
                "drop_a4": _g(s4.residual_drop_orders if s4 else None, ".6g"),
                "drop_am4": _g(sm.residual_drop_orders if sm else None, ".6g"),
                "cn_raw_a0": _g(s0.cn_raw if s0 else None), "cn_raw_a4": _g(s4.cn_raw if s4 else None),
                "cn_raw_am4": _g(cm), "cna": _g(value),
                "cna_basis": "(CN(4)-CN(-4))/2" if sm is not None else "CN(4)",
                "cn_combined_a4": _g(comb.get("cn")), "xcp_m": _g(comb.get("xcp_m")),
                "ca_alpha0": _g(s0.ca if s0 else None),
                "history_gap_a0": _g(s0.history_gap if s0 else None),
                "history_gap_a4": _g(s4.history_gap if s4 else None),
                "history_gap_am4": _g(sm.history_gap if sm else None),
                "source_check": "ok" if not lv.problems else "FAILED: " + " | ".join(lv.problems),
                "job_wall_s": _g(lv.job_wall_s, ".1f"),
            })
    return rows


def _glevel(lv: Level, solves: list[Solve]) -> GridLevel:
    drops = [s.residual_drop_orders for s in solves]
    drop = None if any(x is None for x in drops) else min(drops)
    return GridLevel(lv.name, lv.nodes,
                     {"wall_size_mm": lv.wall_m * 1000, "fin_size_mm": lv.fin_m * 1000},
                     residual_drop_orders=drop,
                     solver_converged=_verdict([s.status for s in solves]))


def families(levels: dict[str, Level]) -> list[tuple[str, FamilyData]]:
    """(condition, family) per Mach and quantity group, in a fixed order."""
    fam = [levels[n] for n in FAMILY_LEVELS]
    machs = fam[0].machs()
    for lv in fam:
        if lv.machs() != machs:
            raise ValueError(f"level {lv.name!r} solved Mach numbers {lv.machs()}, not {machs}")
    out = []
    for m in machs:
        groups = (
            (f"M={m:g} alpha=4 (raw)", "cna", [(m, ALPHA)],
             lambda lv, m=m: cna(lv.solves[(m, ALPHA)].cn_raw, None),
             "CNa = CN(4)/radians(4) from cn_raw; -4 not solved on a z-mirrored mesh"),
            (f"M={m:g} alpha=4 (combined)", "xcp", [(m, ALPHA), (m, 0.0)],
             lambda lv, m=m: lv.combined[(m, ALPHA)]["xcp_m"],
             "xcp_m of the combined row (CN(4) - CN(0) on a z-mirrored mesh)"),
            (f"M={m:g} alpha=0", "ca_alpha0", [(m, 0.0)],
             lambda lv, m=m: lv.solves[(m, 0.0)].ca, ""),
        )
        for cond, q, keys, getter, note in groups:
            gl = [_glevel(lv, [lv.solves[k] for k in keys]) for lv in fam]
            detail = "solves per level (" + " / ".join(f"alpha {k[1]:g}" for k in keys) + "): " + "; ".join(
                f"{lv.name} " + "/".join(lv.solves[k].status for k in keys) for lv in fam)
            out.append((cond, FamilyData(GridFamily(gl, ndim=3), {q: [getter(lv) for lv in fam]},
                                         detail, note)))
    return out


def _refused(cond: str, quantity: str, trip: str, levels: list[Level], note: str) -> list[dict[str, str]]:
    """The two rows (one per h basis) of a family that was not computed, with the reason and no numbers."""
    out = []
    for basis in ("nodes^-1/3", "wall_size"):
        row = {c: "" for c in GCI_COLUMNS}
        row.update(study="sample_euler", condition=cond, quantity=quantity, triplet=trip, h_basis=basis,
                   n1=str(levels[0].nodes), n2=str(levels[1].nodes), n3=str(levels[2].nodes),
                   note=f"refused: {note}")
        out.append(row)
    return out


def _quantities(machs: list[float]) -> list[tuple[str, str]]:
    out = []
    for m in machs:
        out += [(f"M={m:g} alpha=4 (raw)", "cna"), (f"M={m:g} alpha=4 (combined)", "xcp"),
                (f"M={m:g} alpha=0", "ca_alpha0")]
    return out


GCI_LIMIT_PCT = 100.0
"""A GCI above this (percent) means the uncertainty exceeds the value."""

EXACT_VALUES: dict[tuple[str, float], tuple[float, str]] = {
    ("ca_alpha0", 0.3): (0.0, "d'Alembert's paradox, M = 0.3, alpha = 0"),
}
"""Known exact values: (quantity, Mach) -> (value, reason). An extrapolation on the other side of it than the
finest grid value has crossed the bound."""


def _unusable(row: dict[str, str]) -> str:
    """Why a computed row is unusable ('' if it is not): one rule for every quantity."""
    if row["order"] == "":
        return ""
    reasons = []
    gci = float(row["gci_fine_pct"]) if row["gci_fine_pct"] else None
    if gci is not None and gci > GCI_LIMIT_PCT:
        reasons.append(f"gci_fine_pct = {gci:.3g} exceeds {GCI_LIMIT_PCT:g}, so the uncertainty is larger than the value")
    mach = float(row["condition"].split()[0].removeprefix("M="))
    known = EXACT_VALUES.get((row["quantity"], mach))
    if known and row["extrapolated"]:
        exact, why = known
        f1, ext = float(row["f1"]), float(row["extrapolated"])
        if (f1 - exact) * (ext - exact) < 0:
            reasons.append(f"the extrapolated value {ext:.4g} is on the other side of the exact value {exact:g} ({why}) than the finest grid value {f1:.4g}")
    return ("unusable: " + "; ".join(reasons)) if reasons else ""


def _flagged(rows: list[dict[str, str]]) -> list[dict[str, str]]:
    for r in rows:
        why = _unusable(r)
        if why:
            r["note"] = "; ".join(x for x in (r["note"], why) if x)
    return rows


def gci_rows(levels: dict[str, Level]) -> list[dict[str, str]]:
    rows = []
    trip = "/".join(n for n in FAMILY_LEVELS)
    fam = [levels[n] for n in FAMILY_LEVELS]
    bad = [lv for lv in fam if lv.problems]
    if bad:
        note = "the source files of " + "; ".join(
            f"level {lv.name!r} failed the completeness check ({' | '.join(lv.problems)})" for lv in bad)
        machs = sorted({m for lv in fam for m in lv.machs()})
        for cond, q in _quantities(machs):
            rows += _refused(cond, q, trip, fam, note)
        return rows
    for cond, fd in families(levels):
        a = _flagged(_rows("sample_euler", cond, trip, fd, lambda f: study(f.family, f.values), "nodes^-1/3"))
        b = _flagged(_rows("sample_euler", cond, trip, fd, _wall_size_compute, "wall_size"))
        for x, y in zip(a, b):
            rows += [x, y]
    return rows


# ------------------------------------------------------------------ header


def _ratios(vals: list[float]) -> str:
    return " and ".join(f"{b / a:.3g}x" for a, b in zip(vals, vals[1:]))


SIZING = {
    # Recorded before the runs by `ignisyeet cfd-check --mesh` on the local machine with 4 threads:
    # (nodes, cells, the product's memory estimate per case in GB).
    "coarse": (98157, 458692, 1.7),
    "medium": (170830, 798532, 2.8),
    "fine": (302918, 1416444, 4.8),
    "medium_nomirror": (167592, 779764, 2.7),
}
REMOTE_VERSION = "IgnisYeet 0.4.0"
"""`ssh virgo .../bin/ignisyeet-glibc-a8dff9c --version`, first words of the first line (2026-10-17)."""
LOCAL_VERSION = "IgnisYeet 0.4.0"
"""`target/release/ignisyeet --version`, first words of the first line (2026-10-17)."""
MEMORY_EARLIER_GB = 3.3
"""Peak SU2 RSS per case of the 2 mm run of doc/data/cfd/mesh_convergence.csv (1.42 M cells, the mesh of level fine)."""

BASE_INPUT = "examples/sample_cfd.toml"
SETUP_SCRIPT = "doc/scripts/rans_sample.py"
INPUTS_REL = "research/nasa_vv/cases/sample_euler_inputs"
INPUTS = pathlib.Path(__file__).resolve().parent / "sample_euler_inputs"


def _binary_lines(levels: dict[str, Level]) -> list[str]:
    names = sorted({lv.binary for lv in levels.values()})
    evidence = sorted({lv.binary_line for lv in levels.values() if lv.binary_line})
    commits = sorted({lv.commit for lv in levels.values() if lv.commit})
    lines = [
        "Binary: " + ", ".join(names or ["unknown"]) + "; evidence: the job.log line " + (
            " and ".join(f'"{e}"' for e in evidence) if evidence else "(no job.log line found)")
        + f". Version of that binary (ssh virgo ... --version): {REMOTE_VERSION}; of the local target/release/ignisyeet: {LOCAL_VERSION}. "
        "The version string carries no commit, so it does not tell the two apart.",
    ]
    suffixed = [n for n in names if n.rsplit("-", 1)[-1].isalnum() and len(n.rsplit("-", 1)[-1]) >= 7
                and all(c in "0123456789abcdef" for c in n.rsplit("-", 1)[-1])]
    if suffixed:
        lines.append(
            "The binary name carries its source commit (" + ", ".join(n.rsplit("-", 1)[-1] for n in suffixed)
            + "), built on virgo from that commit; the name is the only record of this, and no artifact verifies it. "
            "The ignisyeet_commit of the job record done.json ("
            + (", ".join(commits) if commits else "none") + ") is stale for this binary: virgo_run.sh copies it from the COMMIT file "
            "of the older remote checkout, not from the binary's source, so it is not used.")
    else:
        lines.append("The ignisyeet_commit of the job record is the remote checkout's COMMIT file ("
                     + (", ".join(commits) if commits else "none") + "); it need not be the binary's source.")
    return lines


def _input_lines() -> list[str]:
    return [
        f"Inputs: one case.toml per level, committed under {INPUTS_REL}/ (machine paths replaced by <RUN_DIR> and <CFD_PREFIX>). "
        f"Base file: {BASE_INPUT} (last commit {_last_commit(BASE_INPUT)}) through {SETUP_SCRIPT} setup (last commit {_last_commit(SETUP_SCRIPT)}) "
        "with --machs 0.3 0.8 2.0 --alphas 0 4 --model euler --wall <wall_size> --parallel 3 --ranks <ranks> --memory-gb 30 --set aero.cfd.fin_size=<fin_size> "
        "(medium_nomirror adds --set aero.cfd.z_mirror_mesh=false).",
        "Set by setup, not by hand: output.dir, output.kml = false, rocket.stl and motor.eng as absolute paths, aero.cfd.model/machs/alphas_deg [0, 4]/wall_size/fin_size/ranks_per_case/parallel_cases, "
        "aero.cfd.iterations left to the model default (the base file's `iterations = 3000` is commented out), and a [resources] block (threads = 12, memory_gb = 30; the job overrides threads with the core count).",
        "Edited by hand after setup: (1) setup writes the wall_size override into the comment line `# wall_size = 0.002 and iterations ...` of the base file and leaves the real `wall_size = 0.002` key, so the stray line was removed and the real key set (the stale trailing comment on it, '1.4 M cells, fins at 1 mm', and the orphaned comment line '2 mm RANS mesh needs far more ...' were dropped from the committed copies); "
        "(2) sample_rocket.stl and sample_motor.eng were copied beside case.toml and referenced by relative names so the directory could be synced to virgo; (3) `output.html` was removed because the remote binary of the first submission rejected the key (unknown field); "
        "(4) ranks_per_case was set per level (4, 7, 11, 7) to split the cores in proportion to the cells, and the aero run used --force and `-l` (node-local work directory).",
    ]


def _sizing_lines() -> list[str]:
    coarse = SIZING["coarse"]
    return [
        "Before the runs a local mesh check (ignisyeet cfd-check --mesh, 4 threads) gave: " + "; ".join(
            f"{k} {n} nodes, {c} cells, {g} GB per case (the product's estimate)" for k, (n, c, g) in SIZING.items())
        + ". The meshes of the runs differ by a few cells (gmsh is not bitwise repeatable); the counts in the table above are those of the runs.",
        f"Peak SU2 memory was not measured on virgo (the compute nodes cannot be reached from the login node and PBS keeps no accounting); the earlier 2 mm run of doc/data/cfd/mesh_convergence.csv (1.42 M cells, the mesh of level fine) measured {MEMORY_EARLIER_GB:g} GB per case.",
        f"Placement rule (coarse peak memory x 1.4^6 against 100 GB): {coarse[2]:g} GB x {1.4 ** 6:.2f} = {coarse[2] * 1.4 ** 6:.1f} GB, "
        "below 100 GB, so the levels would fit on the local machine (20 threads, 30 GB RAM); they ran on virgo01 at the controller's instruction "
        "to run all levels at once, and the coarse memory is the product's estimate, not a measurement.",
    ]


def _header(levels: dict[str, Level]) -> list[str]:
    coarse_first = [levels[n] for n in reversed(FAMILY_LEVELS)]
    cells = [lv.cells for lv in coarse_first]
    nodes = [lv.nodes for lv in coarse_first]
    walls = [lv.wall_m for lv in coarse_first]
    lines = [
        "Grid convergence of the sample rocket (examples/sample_rocket.stl), SU2 Euler, M = 0.3 / 0.8 / 2.0, on a three-level grid family built through ignisyeet aero.",
        "Levels (wall_size / fin_size): " + "; ".join(
            f"{lv.name} {lv.wall_m * 1000:g} / {lv.fin_m * 1000:g} mm, {lv.nodes} nodes, {lv.cells} cells"
            for lv in coarse_first) + ". The knobs scale by 1.4 per level (aero.cfd.wall_size and aero.cfd.fin_size).",
        "Runs: " + "; ".join(
            f"{lv.name} on {lv.machine or 'unknown'}"
            + (f" with {lv.ppn} cores" if lv.ppn else "")
            + (f" ({lv.ranks_per_case} ranks per case)" if lv.ranks_per_case else "")
            + (f", job wall time {lv.job_wall_s:.0f} s" if lv.job_wall_s else "")
            for lv in (*coarse_first, levels[NOMIRROR_LEVEL])) + ".",
        *_binary_lines(levels),
        *_input_lines(),
        *_sizing_lines(),
        "Celik et al. (2008) with Fs = 1.25. Every level is gated by nasa_vv.family first; a refused family has no numbers, only the refusal in note.",
        "Status mapping (existing_data._STATUS, shared with the committed studies): converged -> True; accepted -> None (the product's early stop after at least 3 orders of residual drop with flat coefficients: no verdict is claimed and the recorded residual drop decides, so a level passes with a drop of 3 orders or more); "
        "unconverged (limit cycle) and failed -> False; any other string is an error. A level's verdict is the AND over the solves its quantity uses.",
        "Every quantity is written twice, once per h_basis: nodes^-1/3 and wall_size (the knob ratio). The spread between the two is part of the uncertainty.",
        "check_similar sees only the knobs, and the knobs scale together by 1.4, so the family passes it. The grids are nevertheless not geometrically similar: "
        "the mesher fixes the outer cell size at the farfield radius / 10 (crates/cfd/src/mesh.rs:396) and grades the volume from wall_size up to it "
        "(crates/cfd/src/mesh_gmsh.py:153), so refining wall_size and fin_size refines only the near-wall region.",
        f"Measured growth per level, coarse to fine: cells {_ratios(cells)}, nodes {_ratios(nodes)}; wall_size ratio {_ratios([1 / w for w in walls])}. "
        "A geometrically similar 1.4 refinement would grow the cells by 1.4^3 = 2.74x per level.",
        "On grids whose outer cell size is fixed, neither h basis is the effective grid size; the two bracket the question rather than resolve it, so the spread between them is not a bound on the true GCI.",
        "Completeness: the runner stops SU2 while it may be writing, so every file read here is checked, not trusted: cfd_cases.csv and each history.csv must end in a newline with complete finite rows, no residual_last, ca_cfd or cn_raw may be blank on a case that did not fail, the history must reach the judged iteration and hold exactly one row at the residual_last that cfd_cases.csv records, every solve of case.toml must be present, and the job must have finished with exit code 0. A level that fails any check refuses all of its families (the reason is in note and in source_check of sample_euler_levels.csv). The recorded residual and force coefficients belong to the history row at the runner's stop decision (the judged row), and SU2 keeps writing until its next periodic output, so history.csv can run past it; the rows after the judged row are counted in history_gap_*. The surface_flow files are not read.",
        "n1..n3 are nodes (finest first). gci_fine_pct and gci_coarse_pct are relative to f1 and f2 (percent), including for xcp (a length: multiply by |f1|).",
        "cna: CNa [1/rad] = CN(4)/radians(4) from cn_raw on the z-mirrored mesh (-4 is the exact mirror of +4 and is not solved); the non-mirrored medium level uses (CN(4) - CN(-4))/2/radians(4); see cna_basis in sample_euler_levels.csv. xcp: xcp_m of the combined row. ca_alpha0: ca_cfd at alpha = 0.",
        "A note flags any computed order above 4 (twice the formal second order) or below 0.5 (a quarter of it): the triplet is then not in the asymptotic range and its GCI is not reliable.",
        f"A note marks a computed row unusable when its gci_fine_pct exceeds {GCI_LIMIT_PCT:g} (the uncertainty is larger than the value) or when its Richardson extrapolation lies on the other side of a known exact value than the finest grid "
        "(a shared rule; the known values are in EXACT_VALUES of sample_euler.py: CA at M = 0.3, alpha = 0 is 0 by d'Alembert's paradox, so an extrapolation to positive CA is past the bound).",
    ]
    return ["# " + ln for ln in lines]


def _levels_header(levels: dict[str, Level]) -> list[str]:
    lines = [
        "Per-level results of the sample-rocket Euler grid family (levels coarse, medium, fine) and the non-mirrored medium level (medium_nomirror); collected from ignisyeet aero output by nasa_vv.cases.sample_euler.",
        "wall_size_mm and fin_size_mm are aero.cfd.wall_size and aero.cfd.fin_size; nodes and cells are mesh_nodes and mesh_cells of cfd_report.json.",
        "status_*, drop_* and cn_raw_* belong to the alpha = 0, +4 and -4 solves; drop_* are the orders the density residual fell (largest rms[Rho] of the history minus the residual at the judged row).",
        "cna: CNa [1/rad]; cna_basis says whether it is (CN(4)-CN(-4))/2/radians(4) (both signs solved) or CN(4)/radians(4). cn_combined_a4 and xcp_m are the combined row of cfd_cases.csv at alpha = 4: CN(4) - CN(0) on a z-mirrored mesh, (CN(4) - CN(-4))/2 when both signs were solved.",
        "mesh: z-mirrored = aero.cfd.z_mirror_mesh true (a quarter mesh mirrored about the x-y plane, so -alpha is the exact mirror of +alpha and is not solved); not mirrored = z_mirror_mesh false (+4 and -4 both solved).",
        "medium_nomirror is not a member of the grid family; it exists for the CN antisymmetry check in consistency.csv.",
    ]
    return ["# " + ln for ln in lines]


# ------------------------------------------------------------------ driver


def main(work=None, levels_out=None, gci_out=None) -> tuple[pathlib.Path, pathlib.Path]:
    work = pathlib.Path(work) if work is not None else DEFAULT_WORK
    levels_out = pathlib.Path(levels_out) if levels_out is not None else LEVELS_CSV
    gci_out = pathlib.Path(gci_out) if gci_out is not None else GCI_CSV
    levels = read_levels(work)
    lrows, grows = level_rows(levels), gci_rows(levels)
    for out, header, cols, rows in ((levels_out, _levels_header(levels), LEVEL_COLUMNS, lrows),
                                    (gci_out, _header(levels), GCI_COLUMNS, grows)):
        out.parent.mkdir(parents=True, exist_ok=True)
        with out.open("w", newline="") as f:
            for ln in header:
                f.write(ln + "\n")
            w = csv.DictWriter(f, fieldnames=cols, lineterminator="\n")
            w.writeheader()
            w.writerows(rows)
    return levels_out, gci_out


if __name__ == "__main__":
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--work", default=str(DEFAULT_WORK), help="directory holding coarse/ medium/ fine/ medium_nomirror/")
    ap.add_argument("--collect-only", action="store_true",
                    help="accepted for clarity: this module never runs CFD, it only collects")
    ap.add_argument("--levels-out", default=None)
    ap.add_argument("--gci-out", default=None)
    a = ap.parse_args()
    for p in main(a.work, a.levels_out, a.gci_out):
        print(p)
