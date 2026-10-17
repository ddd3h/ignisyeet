# SPDX-License-Identifier: GPL-3.0-or-later
"""Grid convergence from the three CFD studies already committed under doc/data.

No CFD is run here. Every family goes through nasa_vv.family, which refuses a
family whose levels are not geometrically similar or not converged; a refused
family appears in results/gci_existing.csv as rows with a note, never as numbers.

Source files and their exact columns (comment lines start with '#'):

doc/data/cfd/rans_mesh_study.csv (sample rocket, RANS, half model, 8 / 4.8 / 3 mm)
    wall_size_m, nodes, cells, run_wall_s, mach, cn, ca_wall, xcp_m, cn0,
    status_a0, status_a4, status_am4, iterations_a4
doc/data/cfd/rans_flatplate.csv (NASA TMR 2DZP flat plate, 69x49, 137x97, 273x193)
    mach, grid, cells, h, cf_x097, tmr_sa_cf, err_vs_tmr_percent, correlation_cf,
    err_vs_correlation_percent, cd_plate, yplus_max, yplus_mean, iterations_final,
    iterations_startup, rho_drop_orders, nu_drop_orders, pipeline_stop_iter,
    cd_at_stop_vs_final_percent, trend_rule_stop_iter, cd_at_trend_stop_vs_final_percent,
    wall_s, ranks, peak_rss_per_rank_mb
doc/data/validation/basic_finner/mesh_check.csv (Basic Finner, SU2 Euler, M = 2.375)
    level, base_treatment, wall_size_mm, fin_size_mm, cells, mach, alpha_deg, status,
    cn, ca_pressure, xcp_cal, wall_seconds

RANS sample rocket
------------------
Convergence is a property of a solve, but a grid level carries one verdict. A
quantity group is therefore a family of its own and a level's verdict is the AND
over the solves its quantities use:

- ``alpha4`` (cn, ca = ca_wall, xcp = xcp_m): the alpha +4 and alpha -4 solves
  (``status_a4`` and ``status_am4``), because the header says the alpha 4 values
  are "+ -4 solved, mirror-combined".
- ``alpha0`` (cn0): ``status_a0``. Its true value is 0 and the sign changes with
  the mesh, so a GCI would not be meaningful even for converged solves; the
  family exists so that its verdict is on record.

The AND is three-valued: any False gives False, otherwise any None gives None.

``status_*`` strings are mapped here, explicitly, because family.py takes only
True, False or None:

- ``converged``      -> True.  The runner's acceptance rule was met.
- ``unconverged``    -> False. The runner accepted the case with a window mean
  (stalled or limit cycle, doc/source/cfd.rst), i.e. the rule was not met.
- ``rerun (direct)`` -> None.  See ``RERUN_DIRECT_EVIDENCE``.

Any other string is an error.

The ``rerun (direct)`` decision
-------------------------------
The 3 mm, M = 2 alpha -4 case was killed by the out-of-memory killer and solved
again alone with doc/scripts/rans_sample.py ``direct``. That command runs the two
SU2 stages with a fixed iteration count and never applies the runner's acceptance
rule; doc/scripts/rans_mesh_study.py then writes the literal ``rerun (direct)``
for every rerun, taking the last row of its history.csv. The evidence in the
repository is the CSV header: 600 final-stage iterations, cold start,
"coefficients stationary to 1e-4 from iteration 400". That covers the
coefficient-flatness leg of the acceptance and the minimum iteration count only.
The residual-drop leg (density 3 orders, nu 2 orders below the startup peak) has
no record: the rerun's history.csv is not committed, and no JSON or rst records
a drop or a verdict for it. Without evidence that the acceptance was met the
status maps to None. With no residual drop recorded, family.py then refuses the
level.

Flat plate
----------
2-D grids, ``ndim=2``. ``cells`` doubles exactly in each direction between grids,
so ``h = cells**(-1/2)`` (the CSV's own ``h`` column) gives a refinement ratio of
exactly 2 and ``constant_ratio(r=2)`` applies. ``GridLevel.nodes`` holds the
cells, because that is what the CSV's ``h`` is defined from. The residual
evidence is ``min(rho_drop_orders, nu_drop_orders)``: every equation must have
converged, so the weaker one governs. No solver verdict is recorded. The product's
RANS acceptance asks 2 orders on nu, but family.py asks 3 when there is no
verdict, and the smaller recorded drop is 5.76, so both are met.

Basic Finner
------------
Four Euler levels L1 (coarsest) to L4 (finest); wall_size and fin_size are
recorded and scale together by about 1.4. ``study`` takes exactly three levels, so
two triplets are built explicitly: L1-L3 and L2-L4. A family is one
(base_treatment, alpha). The CSV records only ``status``. ``cn`` and ``xcp_cal``
are not studied at alpha 0 (the true cn is 0 and xcp is blank there);
``ca_pressure`` is. ``GridLevel.nodes`` holds the cells, because the CSV has no
node count. The representative h is therefore ``cells**(-1/3)`` (ratios 1.19 to
1.22), not the 1.4 of the recorded cell-size knobs: the cell count grows about
1.7x per level where a uniform 1.4 refinement would give 2.7x.
"""
from __future__ import annotations

import csv
import math
import pathlib
import subprocess
from dataclasses import dataclass, field

from ..family import GridFamily, GridLevel, study
from ..gci import GridConvergence, constant_ratio

ROOT = pathlib.Path(__file__).resolve().parents[3]
"""Repository root: cases -> nasa_vv -> research -> root."""
RESULTS = pathlib.Path(__file__).resolve().parents[1] / "results" / "gci_existing.csv"

RANS_CSV = "doc/data/cfd/rans_mesh_study.csv"
FLATPLATE_CSV = "doc/data/cfd/rans_flatplate.csv"
FINNER_CSV = "doc/data/validation/basic_finner/mesh_check.csv"

RANS_COLUMNS = ("wall_size_m", "nodes", "mach", "cn", "ca_wall", "xcp_m", "cn0",
                "status_a0", "status_a4", "status_am4")
FLATPLATE_COLUMNS = ("mach", "grid", "cells", "h", "cf_x097", "rho_drop_orders", "nu_drop_orders")
FINNER_COLUMNS = ("level", "base_treatment", "wall_size_mm", "fin_size_mm", "cells",
                  "alpha_deg", "status", "cn", "ca_pressure", "xcp_cal")

COLUMNS = ["study", "condition", "quantity", "triplet", "n1", "n2", "n3", "r21", "r32",
           "f1", "f2", "f3", "order", "extrapolated", "gci_fine_pct", "gci_coarse_pct",
           "asymptotic_ratio", "monotone", "note"]

_STATUS: dict[str, bool | None] = {
    "converged": True,
    "unconverged": False,
    "rerun (direct)": None,
}

RERUN_DIRECT_EVIDENCE = (
    "'rerun (direct)' mapped to None: the rerun ran with a fixed iteration count and "
    "no acceptance rule; the CSV header gives 600 iterations with coefficients "
    "stationary to 1e-4 from iteration 400, but no residual drop or verdict is recorded"
)


@dataclass
class FamilyData:
    family: GridFamily
    values: dict[str, list[float]]
    """Per quantity, one value per level, in the order of ``family.levels``."""
    detail: str = ""
    """Per-level solve statuses, appended to a refusal note."""


def _verdict(statuses: list[str]) -> bool | None:
    """AND of the solves a level's quantities use: False beats None beats True."""
    mapped = []
    for s in statuses:
        if s not in _STATUS:
            raise ValueError(
                f"unknown solve status {s!r}; the known strings are {sorted(_STATUS)} "
                "and each must be mapped to True, False or None explicitly"
            )
        mapped.append(_STATUS[s])
    if any(v is False for v in mapped):
        return False
    if any(v is None for v in mapped):
        return None
    return True


def _read_table(path: pathlib.Path, required: tuple[str, ...]) -> list[dict[str, str]]:
    lines = [ln for ln in pathlib.Path(path).read_text().splitlines() if not ln.startswith("#")]
    reader = csv.DictReader(lines)
    have = reader.fieldnames or []
    for c in required:
        if c not in have:
            raise ValueError(f"{path}: column {c!r} not found; the columns are {list(have)}")
    return list(reader)


def _num(row: dict[str, str], col: str) -> float:
    s = row[col].strip()
    if s == "":
        raise ValueError(f"column {col!r} is empty in row {row}")
    return float(s)


# ------------------------------------------------------------------ readers


def read_rans_mesh_study(path) -> dict[tuple[float, str], FamilyData]:
    """One family per (Mach number, quantity group); groups are 'alpha4' and 'alpha0'."""
    rows = _read_table(path, RANS_COLUMNS)
    machs = sorted({float(r["mach"]) for r in rows})
    out: dict[tuple[float, str], FamilyData] = {}
    for m in machs:
        lv = sorted((r for r in rows if float(r["mach"]) == m),
                    key=lambda r: float(r["wall_size_m"]))
        if len(lv) != 3:
            raise ValueError(f"M = {m:g}: expected three levels, found {len(lv)}")
        names = [f"{_num(r, 'wall_size_m') * 1000:g} mm" for r in lv]
        nodes = [int(r["nodes"]) for r in lv]
        knobs = [{"wall_size_m": _num(r, "wall_size_m")} for r in lv]
        groups = {
            "alpha4": (["status_a4", "status_am4"], {"cn": "cn", "ca": "ca_wall", "xcp": "xcp_m"}),
            "alpha0": (["status_a0"], {"cn0": "cn0"}),
        }
        for g, (status_cols, qmap) in groups.items():
            levels = [
                GridLevel(n, N, k, solver_converged=_verdict([r[c] for c in status_cols]))
                for n, N, k, r in zip(names, nodes, knobs, lv)
            ]
            detail = "solves per level (" + " / ".join(status_cols) + "): " + "; ".join(
                f"{n} " + "/".join(r[c] for c in status_cols) for n, r in zip(names, lv)
            )
            if any(r[c] == "rerun (direct)" for r in lv for c in status_cols):
                detail += "; " + RERUN_DIRECT_EVIDENCE
            out[(m, g)] = FamilyData(
                GridFamily(levels, ndim=3),
                {q: [_num(r, col) for r in lv] for q, col in qmap.items()},
                detail,
            )
    return out


def read_flatplate(path) -> dict[float, FamilyData]:
    """One 2-D family per Mach number, quantity cf_x097."""
    rows = _read_table(path, FLATPLATE_COLUMNS)
    out: dict[float, FamilyData] = {}
    for m in sorted({float(r["mach"]) for r in rows}):
        lv = sorted((r for r in rows if float(r["mach"]) == m),
                    key=lambda r: -int(r["cells"]))
        if len(lv) != 3:
            raise ValueError(f"M = {m:g}: expected three grids, found {len(lv)}")
        levels = []
        for r in lv:
            cells, h = int(r["cells"]), _num(r, "h")
            if abs(h - cells ** -0.5) > 1e-5 * h:
                raise ValueError(f"grid {r['grid']}: h = {h} is not cells**(-1/2) = {cells ** -0.5}")
            drop = min(_num(r, "rho_drop_orders"), _num(r, "nu_drop_orders"))
            levels.append(GridLevel(r["grid"], cells, {"h": h}, residual_drop_orders=drop))
        out[m] = FamilyData(GridFamily(levels, ndim=2), {"cf_x097": [_num(r, "cf_x097") for r in lv]})
    return out


def read_basic_finner_mesh_check(path) -> dict:
    """``{"levels": ["L1", ...], "conditions": {(base_treatment, alpha): FamilyData}}``.

    Each FamilyData holds all four levels, finest first."""
    rows = _read_table(path, FINNER_COLUMNS)
    names = sorted({r["level"] for r in rows})
    conds: dict[tuple[str, float], FamilyData] = {}
    keys = sorted({(r["base_treatment"], float(r["alpha_deg"])) for r in rows})
    for base, alpha in keys:
        sel = {r["level"]: r for r in rows
               if r["base_treatment"] == base and float(r["alpha_deg"]) == alpha}
        if sorted(sel) != names:
            raise ValueError(f"{base} alpha {alpha:g}: levels {sorted(sel)}, expected {names}")
        order = sorted(names, key=lambda n: -int(sel[n]["cells"]))  # finest first
        levels = [
            GridLevel(n, int(sel[n]["cells"]),
                      {"wall_size_mm": _num(sel[n], "wall_size_mm"),
                       "fin_size_mm": _num(sel[n], "fin_size_mm")},
                      solver_converged=_verdict([sel[n]["status"]]))
            for n in order
        ]
        qs = ["ca_pressure"] if alpha == 0.0 else ["cn", "ca_pressure", "xcp_cal"]
        conds[(base, alpha)] = FamilyData(
            GridFamily(levels, ndim=3), {q: [_num(sel[n], q) for n in order] for q in qs})
    return {"levels": names, "conditions": conds}


def triplets(fd: FamilyData) -> dict[str, FamilyData]:
    """Explicit three-level families from a longer one: L1-L3 and L2-L4 for four levels."""
    n = len(fd.family.levels)
    out: dict[str, FamilyData] = {}
    for start in range(n - 2):
        sl = slice(start, start + 3)
        lv = fd.family.levels[sl]
        out[f"{lv[-1].name}-{lv[0].name}"] = FamilyData(
            GridFamily(lv, ndim=fd.family.ndim),
            {q: v[sl] for q, v in fd.values.items()},
            fd.detail,
        )
    return dict(sorted(out.items()))


# ------------------------------------------------------------------ results


def _g(x: float | None, spec: str = ".8g") -> str:
    return "" if x is None else format(x, spec)


def _rows(study_name, condition, triplet, fd: FamilyData, compute) -> list[dict[str, str]]:
    n = [lv.nodes for lv in fd.family.levels]
    base = {"study": study_name, "condition": condition, "triplet": triplet,
            "n1": str(n[0]), "n2": str(n[1]), "n3": str(n[2])}
    try:
        results: dict[str, GridConvergence] | None = compute(fd)
        refusal = ""
    except ValueError as e:
        results, refusal = None, f"refused: {e}" + (f" [{fd.detail}]" if fd.detail else "")
    out = []
    for q, v in fd.values.items():
        row = {c: "" for c in COLUMNS}
        row.update(base, quantity=q, f1=_g(v[0], ".10g"), f2=_g(v[1], ".10g"), f3=_g(v[2], ".10g"))
        if results is None:
            row["note"] = refusal
        else:
            g = results[q]
            row.update(
                r21=_g(g.r21), r32=_g(g.r32), order=_g(g.order), extrapolated=_g(g.extrapolated, ".10g"),
                gci_fine_pct=_g(None if g.gci_fine is None else g.gci_fine * 100),
                gci_coarse_pct=_g(None if g.gci_coarse is None else g.gci_coarse * 100),
                asymptotic_ratio=_g(g.asymptotic_ratio), monotone=str(g.monotone), note=g.note,
            )
        out.append(row)
    return out


def rans_results(fams) -> list[dict[str, str]]:
    out = []
    for (m, g), fd in sorted(fams.items()):
        cond = f"M={m:g} alpha=4 (mirror)" if g == "alpha4" else f"M={m:g} alpha=0"
        trip = "/".join(lv.name for lv in fd.family.levels)
        out += _rows("rans_mesh_study", cond, trip, fd, lambda f: study(f.family, f.values))
    return out


def _flatplate_compute(fd: FamilyData) -> dict[str, GridConvergence]:
    fd.family.check_similar()
    h = fd.family.h()
    if abs(h[1] / h[0] - 2.0) > 1e-9 or abs(h[2] / h[1] - 2.0) > 1e-9:
        raise ValueError(f"the refinement ratio is {h[1] / h[0]:.6g} and {h[2] / h[1]:.6g}, not 2")
    return {q: constant_ratio(v[0], v[1], v[2], 2.0) for q, v in fd.values.items()}


def flatplate_results(fams) -> list[dict[str, str]]:
    out = []
    for m, fd in sorted(fams.items()):
        trip = "/".join(lv.name for lv in fd.family.levels)
        out += _rows("rans_flatplate", f"M={m:g}", trip, fd, _flatplate_compute)
    return out


def basic_finner_results(d: dict) -> list[dict[str, str]]:
    out = []
    for (base, alpha), fd4 in sorted(d["conditions"].items()):
        for label, ft in triplets(fd4).items():
            out += _rows("basic_finner_mesh_check", f"{base} alpha={alpha:g}", label, ft,
                         lambda f: study(f.family, f.values))
    return out


# ------------------------------------------------------------------ driver


def _last_commit(rel: str) -> str:
    try:
        r = subprocess.run(
            ["git", "-C", str(ROOT), "log", "-1", "--abbrev=7", "--format=%h", "--", rel],
            capture_output=True, text=True, check=True)
        return r.stdout.strip() or "uncommitted"
    except (OSError, subprocess.CalledProcessError):
        return "unknown"


def _header() -> list[str]:
    lines = [
        "Grid convergence from CFD results already committed; no CFD was run for this file.",
        "Source files and the last commit that changed each:",
    ]
    for rel in (RANS_CSV, "doc/data/cfd/rans_mesh_study.json", FLATPLATE_CSV, FINNER_CSV):
        lines.append(f"  {rel} @ {_last_commit(rel)}")
    lines += [
        "Celik et al. (2008) with Fs = 1.25; the flat plate uses the NASA tutorial's constant-ratio form (r = 2).",
        "Every family is gated by nasa_vv.family first. A refused family has no numbers, only the refusal in note.",
        "n1..n3 are nodes for rans_mesh_study and cells for rans_flatplate and basic_finner_mesh_check (those CSVs record cells only).",
        "gci_fine_pct and gci_coarse_pct are relative to f1 and f2 (percent), including for xcp: multiply by |f1| for a length.",
        "basic_finner_mesh_check: h = cells**(-1/3), ratios 1.19 to 1.22, whereas the recorded cell-size knobs scale by 1.4.",
        "basic_finner_mesh_check triplets are named by their coarsest and finest level: L1-L3 is the coarse triplet, L2-L4 the fine one.",
        "rans_mesh_study: " + RERUN_DIRECT_EVIDENCE + ".",
    ]
    return ["# " + ln for ln in lines]


def main(out: pathlib.Path | str | None = None) -> pathlib.Path:
    out = pathlib.Path(out) if out is not None else RESULTS
    rows = (
        rans_results(read_rans_mesh_study(ROOT / RANS_CSV))
        + flatplate_results(read_flatplate(ROOT / FLATPLATE_CSV))
        + basic_finner_results(read_basic_finner_mesh_check(ROOT / FINNER_CSV))
    )
    out.parent.mkdir(parents=True, exist_ok=True)
    with out.open("w", newline="") as f:
        for ln in _header():
            f.write(ln + "\n")
        w = csv.DictWriter(f, fieldnames=COLUMNS, lineterminator="\n")
        w.writeheader()
        w.writerows(rows)
    return out


if __name__ == "__main__":
    print(main())
