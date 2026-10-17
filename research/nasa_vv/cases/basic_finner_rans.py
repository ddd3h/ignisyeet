# SPDX-License-Identifier: GPL-3.0-or-later
"""The Basic Finner RANS grid family at Mach 1.832 (spec 6.4).

``setup`` writes one self-contained case directory per level from
examples/basic_finner/basic_finner.toml. The levels are solved with
``ignisyeet aero`` on virgo, one fetched run per level in ``<work>/<level>/``
(``done.json`` and ``out/``); ``main`` reads them and writes

    results/basic_finner_rans_levels.csv   one row per solve: counts, status, residual drops, coefficients
    results/gci_basic_finner_rans.csv      the grid convergence, in the columns of gci_existing.csv

    cd research && uv run --project nasa_vv python -m nasa_vv.cases.basic_finner_rans --work WORK

A family whose levels do not all pass ``family.check_converged`` has no numbers, only the refusal.
"""
from __future__ import annotations

import argparse
import csv
import json
import math
import pathlib
import re
import shutil

from ..family import GridFamily, GridLevel, study
from .existing_data import (
    COLUMNS as GCI_COLUMNS,
    ROOT,
    FamilyData,
    _g,
    _last_commit,
    home_to_tilde,
    _read_table,
    _rows,
    _wall_size_compute,
)

EXAMPLE = ROOT / "examples" / "basic_finner"
SOURCE_TOML = EXAMPLE / "basic_finner.toml"
MOTOR = ROOT / "examples" / "sample_motor.eng"
CASE_FILES = ("basic_finner.step", "basic_finner.stl")

MACH = 1.832
ALPHAS = (0.0, 2.0)

LEVELS: tuple[tuple[str, float], ...] = (
    ("coarse", 0.0021),
    ("medium", 0.0015),
    ("fine", 0.00107),
)
"""(name, wall_size) in metres, coarse first; wall_size scales by 1.4 per level.

fin_size stays at ``"auto"``, which for RANS is wall_size, so it scales with wall_size too. The values
of the brief (0.47, 0.336, 0.24 mm, about 0.22 wall_size) are not used: ``ignisyeet aero`` with
model = rans, surface = step and fin_size < wall_size ends in a gmsh heap corruption at 3-D meshing
(signal 6, "corrupted size vs. prev_size") at the coarse and the medium level, on the remote binary (2f31ab9)
and on the local build alike.
"""


def _toml_value(v) -> str:
    if isinstance(v, str):
        return '"' + v + '"'
    if isinstance(v, (list, tuple)):
        return "[" + ", ".join(_toml_value(x) for x in v) + "]"
    return repr(v)


def _set_key(text: str, section: str, key: str, value) -> str:
    """Set ``key`` in ``[section]``: replace the line when it exists, else append it to the section."""
    lines = text.splitlines()
    cur = None
    last = None  # index of the last non-blank, non-comment line of the section
    for i, ln in enumerate(lines):
        m = re.match(r"^\[([^\]]+)\]", ln)
        if m:
            if cur == section:
                break
            cur = m.group(1)
            if cur == section:
                last = i
            continue
        if cur == section and ln.strip() and not ln.lstrip().startswith("#"):
            if re.match(rf"^\s*{re.escape(key)}\s*=", ln):
                lines[i] = f"{key} = {_toml_value(value)}"
                return "\n".join(lines) + "\n"
            last = i
    if last is None:
        raise ValueError(f"section [{section}] not found")
    lines.insert(last + 1, f"{key} = {_toml_value(value)}")
    return "\n".join(lines) + "\n"


def setup(work, ranks_per_case: int = 16, parallel_cases: int = 2) -> list[pathlib.Path]:
    """Write ``work/<level>/case.toml`` and the files it names; return the case.toml paths."""
    work = pathlib.Path(work)
    src = SOURCE_TOML.read_text()
    out = []
    for name, wall in LEVELS:
        d = work / name
        d.mkdir(parents=True, exist_ok=True)
        t = src
        for sec, key, val in (
            ("aero", "method", "cfd"),
            ("motor", "eng", "sample_motor.eng"),
            ("aero.cfd", "model", "rans"),
            ("aero.cfd", "machs", [MACH]),
            ("aero.cfd", "alphas_deg", list(ALPHAS)),
            ("aero.cfd", "wall_size", wall),
            ("aero.cfd", "ranks_per_case", ranks_per_case),
            ("aero.cfd", "parallel_cases", parallel_cases),
        ):
            t = _set_key(t, sec, key, val)
        (d / "case.toml").write_text(t)
        for f in CASE_FILES:
            shutil.copyfile(EXAMPLE / f, d / f)
        shutil.copyfile(MOTOR, d / "sample_motor.eng")
        out.append(d / "case.toml")
    return out


# ------------------------------------------------------------------ collection

RESULTS_DIR = pathlib.Path(__file__).resolve().parents[1] / "results"
LEVELS_CSV = RESULTS_DIR / "basic_finner_rans_levels.csv"
GCI_CSV = RESULTS_DIR / "gci_basic_finner_rans.csv"

D_REF = 0.030
"""Reference diameter [m] (DREV-TM-9703); xcp is reported in calibres from the nose tip."""
EXPECTED_BINARY = "ignisyeet-glibc-a8dff9c"
"""The only binary whose results are accepted (built on virgo from a8dff9c; see the header)."""
NOT_OBTAINED = "RANS GCI not obtained at M = 1.832"
CELL_GROWTH_UNIFORM = 1.4 ** 3
"""Cell growth per level of a geometrically similar 1.4 refinement."""

_STATUS: dict[str, bool | None] = {
    "converged": True,
    "unconverged": False,  # accepted as a limit cycle or a stall: the product's rule was not met
    "accepted": None,      # stopped early after >= 3 orders with flat coefficients: the residual drop decides
    "failed": False,
}
"""Mirrors the rule Task 14 adds to existing_data._STATUS (its branch does not exist here yet): 'accepted' is
None, so family.check_converged applies the 3-order floor to the recorded drop; 'unconverged' stays False."""

LEVEL_COLUMNS = [
    "level", "wall_size_mm", "fin_size_mm", "nodes", "cells", "nodes_growth", "cells_growth",
    "mach", "alpha_deg", "status", "solver_converged", "limit_cycle", "iterations",
    "rho_drop_orders", "nu_drop_orders", "residual_drop_orders", "product_residual_drop", "rans_verdict", "yplus_p95", "cn_raw",
    "ca_wall", "ca_base", "xcp_cal", "wall_seconds", "note",
]


def cna(cn: float, alpha_deg: float = 2.0) -> float:
    """CNa as the existing Basic Finner tables define it: CN(alpha) / alpha in radians."""
    return cn / math.radians(alpha_deg)


def _history(path: pathlib.Path) -> tuple[list[str], list[list[float]]]:
    """A final-stage history, refused unless complete: it ends in a newline and every row has all the fields."""
    raw = pathlib.Path(path).read_text()
    if not raw.endswith("\n"):
        raise ValueError(f"{path}: the last row is cut off (no end of line)")
    rows = list(csv.reader(raw.splitlines()))
    head = [h.strip().strip('"') for h in rows[0]]
    data = []
    for i, r in enumerate(rows[1:], start=2):
        if len(r) != len(head):
            raise ValueError(f"{path}: row {i} has {len(r)} fields, the header has {len(head)}; the file is truncated")
        try:
            data.append([float(v) for v in r])
        except ValueError as e:
            raise ValueError(f"{path}: row {i} does not parse ({e}); the file is truncated") from e
    return head, data


def check_history(path: pathlib.Path, iterations: int) -> None:
    """The history must hold exactly the iterations the case reports, ending at iteration ``iterations - 1``."""
    head, data = _history(path)
    if len(data) != iterations:
        raise ValueError(f"{path}: {len(data)} rows but the case reports {iterations} iterations; "
                         "the history is cut off or from another run")
    last = data[-1][head.index("Inner_Iter")]
    if int(last) != iterations - 1:
        raise ValueError(f"{path}: the last row is iteration {int(last)}, expected {iterations - 1}")


def residual_drops(case_dir: pathlib.Path, done: dict) -> tuple[float | None, float | None]:
    """Orders the density and the SA residual fell below their peak (the reference peak of the
    start-up stage or the final stage's own maximum, whichever is larger), as doc/scripts/rans_sample.py."""
    hist = case_dir / "history.csv"
    ref = done.get("reference_peak")
    if not hist.is_file() or not ref:
        return None, None
    head, data = _history(hist)
    if not data:
        return None, None
    out = []
    for col, key in (("rms[Rho]", "rho"), ("rms[nu]", "nu")):
        i = head.index(col)
        peak = max(ref[key], max(r[i] for r in data))
        out.append(peak - data[-1][i])
    return out[0], out[1]


def _fnum(s):
    s = (s or "").strip()
    return float(s) if s else None


def check_job(run: pathlib.Path, binary: str = EXPECTED_BINARY) -> dict:
    """The job record of a fetched run, refused unless it is a finished job of the expected binary.

    ``done.json`` must exist, say state "done" and exit_code 0 and name ``binary``; the first "binary:" line of
    ``job.log`` must end in the same name. Returns the record with ``log_binary`` and ``log_cmd`` added
    (the two log lines, with the home directory of the account that ran the job written as ``~``)."""
    run = pathlib.Path(run)
    if not (run / "done.json").is_file():
        raise ValueError(f"{run}: no done.json, so the job is not known to have finished")
    try:
        job = json.loads((run / "done.json").read_text())
    except ValueError as e:
        raise ValueError(f"{run / 'done.json'}: does not parse ({e})") from e
    if job.get("state") != "done" or job.get("exit_code") != 0:
        raise ValueError(f"{run / 'done.json'}: state {job.get('state')!r}, exit_code {job.get('exit_code')!r}; "
                         "only a job in state 'done' with exit_code 0 is read")
    if job.get("binary") != binary:
        raise ValueError(f"{run / 'done.json'}: binary {job.get('binary')!r}, not {binary!r}; results of another binary are discarded")
    if not (run / "job.log").is_file():
        raise ValueError(f"{run}: no job.log, so the binary cannot be confirmed")
    lines = (run / "job.log").read_text(errors="replace").splitlines()
    log_binary = next((ln for ln in lines if ln.startswith("binary:")), None)
    if log_binary is None or pathlib.PurePosixPath(log_binary.split(":", 1)[1].strip()).name != binary:
        raise ValueError(f"{run / 'job.log'}: binary line {log_binary!r} does not name {binary!r}")
    cmd = next((ln for ln in lines if ln.startswith("+ ") and " aero " in ln), "")
    return dict(job, log_binary=home_to_tilde(log_binary), log_cmd=home_to_tilde(cmd))


def read_level(run: pathlib.Path, binary: str = EXPECTED_BINARY) -> dict:
    """One level from its fetched run directory (``out/``, ``done.json`` and ``job.log`` of virgo_fetch.sh)."""
    run = pathlib.Path(run)
    out = run / "out"
    job = check_job(run, binary)
    rep = json.loads((out / "cfd_report.json").read_text())
    for k in ("mesh_nodes", "mesh_cells", "axial_breakdown"):
        if k not in rep:
            raise ValueError(f"{out / 'cfd_report.json'}: key {k!r} missing; the file is incomplete")
    raw = (out / "cfd_cases.csv").read_text()
    if not raw.endswith("\n"):
        raise ValueError(f"{out / 'cfd_cases.csv'}: the last row is cut off (no end of line)")
    rows = list(csv.DictReader(raw.splitlines()))
    for r in rows:
        if None in r.values() or None in r:
            raise ValueError(f"{out / 'cfd_cases.csv'}: a row is shorter or longer than the header: {r}")
    if not rows:
        raise ValueError(f"{out / 'cfd_cases.csv'}: no case rows")
    cases: dict[float, dict] = {}
    by_dir = {}
    for d in sorted((out / "cfd").glob("m*_a*")):
        if (d / "done.json").is_file():
            dj = json.loads((d / "done.json").read_text())
            by_dir[(float(dj["mach"]), float(dj["alpha_deg"]))] = (d, dj)
    brk = {(float(b["mach"]), float(b["alpha_deg"])): b for b in rep.get("axial_breakdown", [])}
    for r in rows:
        if r["status"] == "combined":
            continue
        key = (float(r["mach"]), float(r["alpha_deg"]))
        if r["status"] not in _STATUS:
            raise ValueError(f"unknown case status {r['status']!r} in {out / 'cfd_cases.csv'}")
        if (r.get("rans_verdict") or "").strip() not in ("", "converged", "limit_cycle", "stalled"):
            raise ValueError(f"{out / 'cfd_cases.csv'}: rans_verdict {r.get('rans_verdict')!r} is not a verdict of the "
                             "product; the last row is cut off")
        d, dj = by_dir.get(key, (None, {}))
        b = brk.get(key, {})
        if r["status"] != "failed":
            if d is None:
                raise ValueError(f"{out}: case {key} has no done.json in its directory")
            if int(dj.get("iterations", -1)) != int(r["iterations"]):
                raise ValueError(f"{d}: done.json has {dj.get('iterations')} iterations, cfd_cases.csv {r['iterations']}")
            if not (d / "history.csv").is_file():
                raise ValueError(f"{d}: history.csv missing")
            check_history(d / "history.csv", int(r["iterations"]))
            if key not in brk:
                raise ValueError(f"{out / 'cfd_report.json'}: no axial_breakdown for {key}")
        rho, nu = residual_drops(d, dj) if d is not None else (None, None)
        drop = None if rho is None or nu is None else min(rho, nu)
        solved = r["status"] != "failed"
        if solved and "converged" in dj and bool(dj["converged"]) != (r["status"] == "converged"):
            raise ValueError(f"{key}: cfd_cases.csv status {r['status']!r} and done.json converged "
                             f"{dj['converged']!r} disagree")
        cn = _fnum(r["cn_raw"]) if solved else None
        xcp = _fnum(r["xcp_raw_m"]) if solved else None
        cases[key[1]] = {
            "mach": key[0], "alpha_deg": key[1], "status": r["status"],
            "solver_converged": _STATUS[r["status"]],
            "limit_cycle": bool(dj.get("limit_cycle", False)),
            "iterations": int(r["iterations"]),
            "rho_drop": rho, "nu_drop": nu, "drop": drop,
            "product_drop": _fnum(r.get("residual_drop")),
            "rans_verdict": (r.get("rans_verdict") or "").strip(),
            "yplus_p95": dj.get("yplus_p95"),
            "cn_raw": cn,
            "xcp_cal": None if xcp is None or key[1] == 0.0 else xcp / D_REF,
            "ca_wall": b.get("cfd_wall_ca") if solved else None,
            "ca_base": b.get("cfd_base_ca") if solved else None,
            "wall_seconds": _fnum(r["wall_seconds"]),
        }
    for a in ALPHAS:
        if a not in cases:
            raise ValueError(f"{out / 'cfd_cases.csv'}: no row for alpha = {a:g}; the table is cut off or the case was not solved")
    return {
        "nodes": int(rep["mesh_nodes"]), "cells": int(rep["mesh_cells"]), "cases": cases,
        "job": job, "ranks_per_case": rep.get("ranks_per_case"),
        "parallel_cases": rep.get("parallel_cases"),
    }


def _mm(x: float) -> float:
    return round(x * 1000.0, 6)


def _level_rows(data: dict[str, dict], refused: dict[str, str] | None = None) -> list[dict[str, str]]:
    out = []
    prev = None
    for name, wall in LEVELS:
        lv = data.get(name)
        if lv is None:
            if refused and name in refused:
                row = {k: "" for k in LEVEL_COLUMNS}
                row.update(level=f"{_mm(wall):g} mm", wall_size_mm=f"{_mm(wall):g}", fin_size_mm=f"{_mm(wall):g}",
                           note=f"source files refused: {refused[name]}; {NOT_OBTAINED}")
                out.append(row)
            continue
        # growth against the next coarser level that was run (rows go coarse -> fine)
        ng = "" if prev is None else _g(lv["nodes"] / prev["nodes"], ".4g")
        cg = "" if prev is None else _g(lv["cells"] / prev["cells"], ".4g")
        for alpha in sorted(lv["cases"]):
            c = lv["cases"][alpha]
            row = {k: "" for k in LEVEL_COLUMNS}
            note = ""
            if not c["solver_converged"]:
                note = f"not converged ({c['status']}); {NOT_OBTAINED} from this level"
            row.update(
                level=f"{_mm(wall):g} mm", wall_size_mm=f"{_mm(wall):g}", fin_size_mm=f"{_mm(wall):g}",
                nodes=str(lv["nodes"]), cells=str(lv["cells"]), nodes_growth=ng, cells_growth=cg,
                mach=f"{c['mach']:g}", alpha_deg=f"{alpha:g}", status=c["status"],
                solver_converged=str(c["solver_converged"]), limit_cycle=str(c["limit_cycle"]),
                iterations=str(c["iterations"]), rho_drop_orders=_g(c["rho_drop"], ".4g"),
                nu_drop_orders=_g(c["nu_drop"], ".4g"), residual_drop_orders=_g(c["drop"], ".4g"),
                product_residual_drop=_g(c["product_drop"], ".4g"), rans_verdict=c["rans_verdict"],
                yplus_p95=_g(c["yplus_p95"], ".4g"), cn_raw=_g(c["cn_raw"], ".8g"),
                ca_wall=_g(c["ca_wall"], ".8g"), ca_base=_g(c["ca_base"], ".8g"),
                xcp_cal=_g(c["xcp_cal"], ".8g"), wall_seconds=_g(c["wall_seconds"], ".1f"), note=note)
            out.append(row)
        prev = lv
    return out


def _group_families(data: dict[str, dict]) -> dict[str, FamilyData]:
    """The two quantity groups, each a family of the levels in ``data`` (finest first).

    ``alpha2``: cna and xcp, from the alpha = +2 solve. ``alpha0``: the axial force, from the alpha = 0
    solve. A level's verdict is the product's own verdict of that solve (``status``)."""
    names = [n for n, _ in reversed(LEVELS) if n in data]
    wall = dict(LEVELS)
    out = {}
    for group, alpha in (("alpha2", 2.0), ("alpha0", 0.0)):
        levels, cs = [], []
        for n in names:
            lv, c = data[n], data[n]["cases"][alpha]
            levels.append(GridLevel(
                f"{_mm(wall[n]):g} mm", lv["nodes"],
                {"wall_size_mm": _mm(wall[n]), "fin_size_mm": _mm(wall[n])},
                residual_drop_orders=c["drop"], solver_converged=c["solver_converged"]))
            cs.append(c)
        if group == "alpha2":
            vals = {"cna": [None if c["cn_raw"] is None else cna(c["cn_raw"], 2.0) for c in cs],
                    "xcp": [c["xcp_cal"] for c in cs]}
        else:
            vals = {
                "ca_alpha0": [None if c["ca_wall"] is None else c["ca_wall"] + c["ca_base"] for c in cs],
                "ca_wall_alpha0": [c["ca_wall"] for c in cs],
                "ca_base_alpha0": [c["ca_base"] for c in cs],
            }
        detail = f"solves per level (alpha {alpha:g}): " + "; ".join(
            f"{lv.name} {c['status']}" + ("" if c["drop"] is None else f" (residual drop {c['drop']:.3g} orders)")
            for lv, c in zip(levels, cs))
        out[group] = FamilyData(GridFamily(levels, ndim=3), vals, detail)
    return out


def _missing_rows(group: str, quantities, missing: list[str], condition: str, h_basis: str,
                  reasons: dict[str, str] | None = None) -> list[dict[str, str]]:
    rows = []
    for q in quantities:
        row = {c: "" for c in GCI_COLUMNS}
        row.update(study="basic_finner_rans", condition=condition, quantity=q, h_basis=h_basis,
                   triplet="/".join(f"{_mm(w):g} mm" for n, w in reversed(LEVELS)),
                   note=f"refused: level(s) {', '.join(missing)} not run; {NOT_OBTAINED}"
                   + "".join(f" [level {n}: source files refused: {why}]" for n, why in (reasons or {}).items()))
        rows.append(row)
    return rows


def gci_rows(data: dict[str, dict], refused: dict[str, str] | None = None) -> list[dict[str, str]]:
    """Every quantity under both h bases, the two rows of a quantity next to each other."""
    refused = refused or {}
    missing = [n for n, _ in LEVELS if n not in data]
    conds = {"alpha2": "M=1.832 flat base alpha=2", "alpha0": "M=1.832 flat base alpha=0"}
    quantities = {"alpha2": ["cna", "xcp"], "alpha0": ["ca_alpha0", "ca_wall_alpha0", "ca_base_alpha0"]}
    out = []
    if missing:
        for g in ("alpha2", "alpha0"):
            for q in quantities[g]:
                out += _missing_rows(g, [q], missing, conds[g], "nodes^-1/3", refused)
                out += _missing_rows(g, [q], missing, conds[g], "wall_size", refused)
        return out
    fams = _group_families(data)
    for g in ("alpha2", "alpha0"):
        fd = fams[g]
        trip = "/".join(lv.name for lv in fd.family.levels)
        a = _rows("basic_finner_rans", conds[g], trip, fd, lambda f: study(f.family, f.values), "nodes^-1/3")
        b = _rows("basic_finner_rans", conds[g], trip, fd, _wall_size_compute, "wall_size")
        for x, y in zip(a, b):
            for row in (x, y):
                if row["note"].startswith("refused:"):
                    row["note"] += f"; {NOT_OBTAINED}"
            out += [x, y]
    return out


PUBLISHED_CSV = "doc/data/validation/basic_finner/cfd_rans.csv"
PUBLISHED_RST = "doc/source/validation.rst"


def _published_line(data: dict[str, dict]) -> str:
    row = _read_table(ROOT / PUBLISHED_CSV, ("nodes", "cells"))[0]
    pn, pc = int(row["nodes"]), int(row["cells"])
    lv = data.get("medium")
    if lv is None:
        return f"Published run: {pn} nodes, {pc} cells at wall_size 1.5 mm; the medium level was not read."
    return (f"The medium level (wall_size = fin_size = 1.5 mm, the published settings) has {lv['nodes']} nodes and {lv['cells']} cells "
            f"against the published {pn} nodes and {pc} cells ({100 * (lv['nodes'] / pn - 1):+.1f} % nodes, {100 * (lv['cells'] / pc - 1):+.1f} % cells): "
            "the same settings, a different mesh, so the medium level is the nearest level to the published run but is not that run.")


def _provenance(work) -> list[str]:
    """Lines of ``<work>/provenance.txt``: the ``--version`` outputs and the COMMIT files, captured when the
    runs were made (the collector does not call ssh)."""
    f = pathlib.Path(work) / "provenance.txt"
    if not f.is_file():
        return ["provenance.txt is absent from the work directory: no --version or COMMIT record."]
    return [ln.strip() for ln in f.read_text().splitlines() if ln.strip()]


def _drop_example(data: dict[str, dict]) -> str:
    best = None
    for name, lv in data.items():
        for alpha, c in lv["cases"].items():
            if c["product_drop"] is not None and c["drop"] is not None:
                gap = c["product_drop"] - c["drop"]
                if best is None or gap > best[0]:
                    best = (gap, name, alpha, c)
    if best is None:
        return ""
    _, name, alpha, c = best
    return (f" Largest gap: {name} level, alpha {alpha:g}: product {c['product_drop']:.3g} orders ({c['rans_verdict']}), "
            f"gate {c['drop']:.3g} (rho {c['rho_drop']:.4g}, nu {c['nu_drop']:.4g}).")


def _common_header(data: dict[str, dict], prov: list[str]) -> list[str]:
    commits = sorted({lv["job"].get("ignisyeet_commit", "?") for lv in data.values()})
    lines = [
        "Basic Finner, SU2 RANS Spalart-Allmaras grid family at Mach 1.832, alpha 0 and 2 deg: wall_size 2.1, 1.5 and 1.07 mm (a factor 1.4 per level), solved with `ignisyeet aero` on virgo.",
        "Geometry, CG (cg_dry 0.165 m), nose direction (-x), STL scale, STEP surface and reference values are those of examples/basic_finner/basic_finner.toml; the case files are written by nasa_vv.cases.basic_finner_rans.setup.",
        "Keys set on top of that file by setup: aero.method = cfd (the file has barrowman), motor.eng = sample_motor.eng (copied next to case.toml; the file has ../sample_motor.eng), aero.cfd.model = rans, machs = [1.832], alphas_deg = [0, 2], wall_size per level, ranks_per_case and parallel_cases; fin_size is not set (auto). Set by the runner on the command line: resources.threads = the reserved cores, aero.cfd.prefix = \"\" (the environment's SU2 is used) and, with -l, aero.cfd.workdir on the node's local disk. resources.memory_gb is absent (the published run had 30).",
        "Invocation per level: VIRGO_BIN=ignisyeet-glibc-a8dff9c virgo_run.sh -w -l -n t15-n-<level> <level>/case.toml virgo02 <cores> aero --force, the three levels submitted together on virgo02; the exact `ignisyeet` command of each job is quoted below from its job.log.",
        f"Compared with the published RANS run ({PUBLISHED_CSV}; {PUBLISHED_RST}:704-712): the same toml, the same model (rans, Spalart-Allmaras), surface = step, yplus = 1, flat base (a RANS mesh always has one) and half domain, with fin_size left at 'auto', which for RANS is wall_size.",
        "Settings that differ from the published run: wall_size at the coarse and fine levels (the medium level has the published wall_size and fin_size); alpha_mode = mirror (the published run set alpha_mode = single, so these runs also solve alpha = -2, which is listed but not used); the CFD work directory on the node's local disk; ranks_per_case 32 at the fine level (the published run used 16 x 2 cases and 32 threads); and the binary (the published run used the remote binary built from 2f31ab9). cn_raw, xcp_raw_m and the axial force of the alpha = +2 solve are read raw, as in the published table.",
        _published_line(data),
        "Files read: done.json and job.log of each job, cfd_cases.csv, cfd_report.json and, per case, done.json and history.csv; the surface_flow files are not read. Each is checked: the job is in state 'done' with exit_code 0 and names ignisyeet-glibc-a8dff9c in both done.json and the first 'binary:' line of job.log; rows are whole, the history holds the reported iterations, both alpha = 0 and 2 are present. A level that fails a check is refused with the reason in its note; it never raises.",
        "The fin_size of the brief (0.47, 0.336 and 0.24 mm, about 0.22 wall_size) was not used. With model = rans and surface = step and fin_size < wall_size, `ignisyeet aero` stops with error[IY-F011] 'gmsh meshing of the CAD surface failed (killed by signal 6)': the mesh.log of the 3-D meshing shows 'Recovering 2427 missing facet(s)', Steiner points, then glibc 'corrupted size vs. prev_size'. gmsh 4.15.2 in the virgo and in the local environment. Tried: coarse (0.47 mm) and medium (0.336 mm) on ignisyeet-glibc-a8dff9c at 16 cores, on the older remote binary ignisyeet-glibc (coarse at 32 and 16 cores, medium at 16) and the coarse level on the local build with resources.threads = 2; the fine level was not tried. The logs are not committed. The product was not modified.",
        "Grids built through ignisyeet aero cannot be geometrically similar: max_size is fixed at farfield radius / 10 (crates/cfd/src/mesh.rs:396) and the volume is graded up to it (crates/cfd/src/mesh_gmsh.py:153), so refining wall_size refines only the near-wall region. nasa_vv.family.check_similar sees only the recorded knobs (wall_size and fin_size, both scaled by 1.4), not the cell counts.",
        f"A geometrically similar 1.4 refinement would grow the cells by 1.4^3 = {CELL_GROWTH_UNIFORM:.2f}x per level; the measured growth is in the cells_growth column and below.",
    ]
    prev = None
    for name, wall in LEVELS:
        lv = data.get(name)
        if lv is None:
            lines.append(f"Level {name} ({_mm(wall):g} mm): not read.")
            continue
        job = lv["job"]
        g = "" if prev is None else (
            f"; cells x{lv['cells'] / prev['cells']:.3f} and nodes x{lv['nodes'] / prev['nodes']:.3f} against the next coarser level (uniform: {CELL_GROWTH_UNIFORM:.2f})")
        lines.append(
            f"Level {name} ({_mm(wall):g} mm): {lv['nodes']} nodes, {lv['cells']} cells{g}; run on {job.get('node', '?')} "
            f"with {job.get('ppn', '?')} cores reserved, ranks_per_case {lv['ranks_per_case']}, parallel_cases {lv['parallel_cases']}; "
            f"job wall time {job.get('wall_seconds', '?')} s; job.log: '{job['log_binary']}'; command: {job['log_cmd'][2:] or '?'}")
        prev = lv
    lines += [
        "Binary provenance (captured when the runs were made; provenance.txt of the work directory):",
        *["  " + ln for ln in prov],
        f"done.json's ignisyeet_commit is {', '.join(commits)} for the jobs read: it is the COMMIT file of the repository checkout on virgo (repo/COMMIT, the older build), not a property of ignisyeet-glibc-a8dff9c, and it was not used.",
        "'Built from a8dff9c' rests on the binary's name and on the controller's build record: PBS job 1627 built it from a8dff9c sources uploaded to repo-a8dff9c on virgo, whose COMMIT file reads a8dff9c (above). `--version` prints only 0.4.0 for every binary, so nothing in the binary itself shows the commit.",
        "Observed, not explained: the coarse level's verdicts differ between two runs. t15-bf-coarse-auto (older binary ignisyeet-glibc, 16 cores, ranks_per_case 8, parallel_cases 2; not part of these tables): alpha 0 unconverged (554 iterations), alpha 2 converged (405). The coarse run read here (ignisyeet-glibc-a8dff9c, 32 cores, ranks_per_case 16, parallel_cases 2): alpha 0 converged (406), alpha 2 unconverged (505). The runs differ in binary and in domain decomposition, so the comparison is confounded and no cause is claimed.",
        f"Source files and the last commit that changed each: {PUBLISHED_CSV} @ {_last_commit(PUBLISHED_CSV)}, examples/basic_finner/basic_finner.toml @ {_last_commit('examples/basic_finner/basic_finner.toml')}.",
    ]
    return lines


def _header(data: dict[str, dict], what: str, prov: list[str]) -> list[str]:
    lines = _common_header(data, prov) + [
        "residual_drop_orders is the gate's number: min(rho_drop_orders, nu_drop_orders), each measured from max(reference_peak of the case's done.json, the largest value in history.csv) down to the last history row, following Task 8's convention (doc/scripts/rans_sample.py collect, as used for the flat plate). solver_converged is the product's verdict of the solve (status = converged); nasa_vv.family.check_converged gates on it, and where the verdict is absent on the drop. Both are listed whether the level passed or not.",
        "product_residual_drop and rans_verdict are the product's own columns residual_drop and rans_verdict of cfd_cases.csv: residual_drop is the drop of the density residual only, from the largest density residual of the run to the last row (crates/cfd/src/history.rs:378, residual_drop_from_peak), and rans_verdict is converged, limit_cycle or stalled. The SA (nu) residual is not in it, so it is larger than the gate's number wherever nu fell less than rho." + _drop_example(data) + " That the product's density-only number differs from rho_drop_orders by a few hundredths (for example 3.906 against 3.944) is not established; the peak it measures from was not recovered from the files.",
        what,
    ]
    return ["# " + ln for ln in lines]


def _gci_header(data: dict[str, dict], prov: list[str]) -> list[str]:
    lines = [
        "Grid convergence of the Basic Finner RANS levels at Mach 1.832 (Celik et al. (2008), Fs = 1.25). Row and column conventions as results/gci_existing.csv; the per-solve evidence is basic_finner_rans_levels.csv.",
        "cna = CN(2 deg) / radians(2 deg) from cn_raw; xcp in calibres from the nose tip at alpha = 2 deg; ca_alpha0 = CFD wall + CFD base axial force at alpha = 0 (ca_wall_alpha0 and ca_base_alpha0 are its two parts). gci_fine_pct and gci_coarse_pct are relative to f1 and f2 (percent).",
        "n1..n3 are nodes. Every quantity is computed twice, once per h_basis (nodes^-1/3 and the wall_size knob); the spread between the two is part of the uncertainty and neither basis is the effective grid size.",
        "A refused family has no numbers, only the refusal in note. A note flags any computed order above 4 or below 0.5: the triplet is then not in the asymptotic range and its GCI is not reliable. The gate is nasa_vv.family.check_converged on the product's verdict, with min(rho, nu) of the history as the recorded drop.",
    ] + _common_header(data, prov)
    return ["# " + ln for ln in lines]


def collect(work, binary: str = EXPECTED_BINARY) -> tuple[dict[str, dict], dict[str, str]]:
    """Read every level whose run directory exists below ``work``.

    A level whose source files fail the completeness checks is not read: it is returned in the second dict,
    with the reason, and gets no numbers."""
    work = pathlib.Path(work)
    data, refused = {}, {}
    for n, _ in LEVELS:
        if not (work / n).is_dir():
            continue
        try:
            data[n] = read_level(work / n, binary)
        except (ValueError, KeyError, OSError, IndexError) as e:
            refused[n] = str(e).replace("\n", " ")
    return data, refused


def _write(path: pathlib.Path, header: list[str], columns: list[str], rows: list[dict[str, str]]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", newline="") as f:
        for ln in header:
            f.write(ln + "\n")
        w = csv.DictWriter(f, fieldnames=columns, lineterminator="\n")
        w.writeheader()
        w.writerows(rows)


def main(work=None, levels_out=None, gci_out=None) -> tuple[pathlib.Path, pathlib.Path]:
    if work is None:
        ap = argparse.ArgumentParser(description=__doc__)
        ap.add_argument("--work", required=True, help="directory with one fetched run per level: <work>/<level>/{done.json,out/}")
        ap.add_argument("--levels-out", default=str(LEVELS_CSV))
        ap.add_argument("--gci-out", default=str(GCI_CSV))
        a = ap.parse_args()
        work, levels_out, gci_out = a.work, a.levels_out, a.gci_out
    levels_out = pathlib.Path(levels_out or LEVELS_CSV)
    gci_out = pathlib.Path(gci_out or GCI_CSV)
    data, refused_levels = collect(work)
    rows = gci_rows(data, refused_levels)
    computed = [r for r in rows if r["order"] != "" or r["extrapolated"] != "" or r["gci_fine_pct"] != ""]
    refused = [r for r in rows if r["note"].startswith("refused:")]
    if refused and not computed:
        what = f"{NOT_OBTAINED}: every family was refused (see the note column of each level and of the GCI file)."
    elif refused:
        what = f"{NOT_OBTAINED} for the refused families; the others carry a GCI."
    else:
        what = "A GCI was obtained for every family."
    prov = _provenance(work)
    _write(levels_out, _header(data, what, prov), LEVEL_COLUMNS, _level_rows(data, refused_levels))
    _write(gci_out, _gci_header(data, prov), GCI_COLUMNS, rows)
    return levels_out, gci_out


if __name__ == "__main__":
    print(*main())
