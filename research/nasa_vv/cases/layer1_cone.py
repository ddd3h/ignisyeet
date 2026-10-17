# SPDX-License-Identifier: GPL-3.0-or-later
"""Layer 1: the 10 degree cone-cylinder through IgnisYeet's whole pipeline, at M = 2.35.

Layer 0 solved the same cone with SU2 alone on a structured axisymmetric grid. Layer 1 solves it the way
the product does: CAD profile, gmsh, SU2, coefficients, on a 3-D unstructured half model. Both are
compared with the same Taylor-Maccoll solution (``nasa_vv.exact.taylor_maccoll(2.35, 10.0)``), which is
verification against an exact solution, and the difference of their errors is reported as
"pipeline error, including 3-D unstructured meshing" (``CONTRIBUTION_NAME``), never as meshing error alone.

The CFD is not run here. ``main`` collects it from a work directory (default ~/.cache/nasa_vv_t18_layer1):

    <work>/versions.txt                      free text, copied into the CSV header (binary versions)
    <work>/<level>/run/done.json             virgo_run.sh job record
    <work>/<level>/run/job.log               job log (the verdict of a failed case is read from it)
    <work>/<level>/run/out/cfd/mesh/mesh_stats.json
    <work>/<level>/run/out/cfd/m2.350_a00.00/history.csv, done.json
    <work>/<level>/cp/cone_cp.csv            doc/scripts/rans_cone.py --mach 2.35 on that case directory

for level in coarse, medium, fine (wall_size 3.92, 2.8 and 2.0 mm), and writes

    results/layer1_cone.csv        per level and bin: Cp, the exact value, the difference
    results/layer1_gci.csv         Celik GCI of each quantity, once per h basis, behind the family gate
    results/layer0_vs_layer1.csv   layer 0 (as committed in layer0.csv / layer0_gci.csv) beside layer 1

Run: ``cd research && uv run --project nasa_vv python -m nasa_vv.cases.layer1_cone``.
"""
from __future__ import annotations

import argparse
import csv
import hashlib
import io
import json
import pathlib
import functools
import re
from dataclasses import dataclass

from .. import exact
from ..family import GridFamily, GridLevel, study
from ..gci import GridConvergence, grid_convergence
from .existing_data import COLUMNS as GCI_COLUMNS
from .existing_data import FamilyData, _last_commit, _num, _read_table, _rows
from .layer0 import bounds_columns

RESULTS = pathlib.Path(__file__).resolve().parents[1] / "results"
DEFAULT_WORK = pathlib.Path.home() / ".cache" / "nasa_vv_t18_layer1"

MACH = 2.35
DELTA_DEG = 10.0
CASE_DIR = "m2.350_a00.00"
LEVELS = (("coarse", 3.92e-3), ("medium", 2.80e-3), ("fine", 2.00e-3))
"""Coarsest first, like layer0.csv. The family takes them finest first."""
FULL_WINDOW = (0.2, 0.9)
"""x / L_cone range of rans_cone.py's own mean (the brief's cp_mean)."""
LAYER0_WINDOW = (0.4, 0.9)
"""The window of layer 0's wall_cp (wall nodes 0.4 <= x <= 0.9 of a cone whose domain ends at x = 1)."""

@functools.lru_cache(maxsize=None)
def exact_cp() -> float:
    """Taylor-Maccoll surface Cp of the 10 degree cone at M = 2.35 (the shooting is slow, so it is done once)."""
    return exact.taylor_maccoll(MACH, DELTA_DEG).cp


CONTRIBUTION_NAME = "pipeline error, including 3-D unstructured meshing"
BASES = ("nodes^-1/3", "wall_size")
GROWTH_TARGET = 1.4 ** 3
"""Cell growth per level of a geometrically similar 1.4 refinement."""
QUANTISATION = 5e-6
"""rans_cone.py prints cp_mean with five decimals: a rounding of up to 5e-6."""
STUDY = "layer1"
CONDITION = "cone-cylinder M=2.35 alpha=0"


# ------------------------------------------------------------------ the binned Cp of rans_cone.py


@dataclass(frozen=True)
class Bin:
    lo: float
    hi: float
    nodes: int
    cp: float


@dataclass(frozen=True)
class Binned:
    header: str
    bins: list
    all_row: Bin
    script_cp_exact: float
    """The Taylor-Maccoll Cp that rans_cone.py printed in its own header (five decimals)."""


_HEADER_TM = re.compile(r"Cp = ([0-9.]+),")


def parse_binned(path) -> Binned:
    """rans_cone.py's CSV: a ``#`` header line, then one row per bin and a final row for the whole range."""
    path = pathlib.Path(path)
    head = [ln for ln in path.read_text().splitlines() if ln.startswith("#")]
    rows = _read_table(path, ("x_over_lc_from", "x_over_lc_to", "nodes", "cp_mean"))
    bins = [Bin(_num(r, "x_over_lc_from"), _num(r, "x_over_lc_to"), int(_num(r, "nodes")), _num(r, "cp_mean"))
            for r in rows]
    full = [b for b in bins if abs(b.lo - FULL_WINDOW[0]) < 1e-9 and abs(b.hi - FULL_WINDOW[1]) < 1e-9]
    if len(full) != 1:
        raise ValueError(f"{path}: no row for the whole range {FULL_WINDOW[0]},{FULL_WINDOW[1]}")
    m = _HEADER_TM.search(head[0]) if head else None
    if m is None:
        raise ValueError(f"{path}: the header line does not give the script's Taylor-Maccoll Cp")
    return Binned(" ".join(head), [b for b in bins if b is not full[0]], full[0], float(m.group(1)))


def _window(bins, lo: float, hi: float) -> list:
    sel = [b for b in bins if b.lo >= lo - 1e-9 and b.hi <= hi + 1e-9]
    if not sel or abs(min(b.lo for b in sel) - lo) > 1e-9 or abs(max(b.hi for b in sel) - hi) > 1e-9:
        raise ValueError(f"no whole bins cover x/L {lo} to {hi}")
    return sel


def cp_mean(bins, lo: float = FULL_WINDOW[0], hi: float = FULL_WINDOW[1]) -> float:
    """Mean Cp over the bins in [lo, hi], weighted by the wall nodes of each bin.

    Over the whole range this is the node mean that rans_cone.py prints in its last row."""
    sel = _window(bins, lo, hi)
    return sum(b.nodes * b.cp for b in sel) / sum(b.nodes for b in sel)


def cp_nearest_tip(bins) -> Bin:
    """The bin at the smallest x / L_cone."""
    return min(bins, key=lambda b: b.lo)


# ------------------------------------------------------------------ one level


@dataclass(frozen=True)
class LevelData:
    name: str
    grid: GridLevel
    cells: int
    binned: Binned
    done: dict
    cp_sha: str


def residual_drop(history: pathlib.Path) -> float:
    """Peak of log10 rms[Rho] over the history minus its final value (the convention of layer0.csv)."""
    with history.open(newline="") as f:
        rows = list(csv.reader(f))
    head = [h.strip().strip('"') for h in rows[0]]
    c = head.index("rms[Rho]")
    v = [float(r[c]) for r in rows[1:]]
    return max(v) - v[-1]


def solver_verdict(run: pathlib.Path) -> bool | None:
    """The product's own verdict on the alpha = 0 case: its done.json if it wrote one, else the failure text.

    done.json is written for an accepted case. A case the runner rejected has none, and its reason is the
    line 'first failed case M=2.35 alpha=0 ...: not converged (...)' of the job log. A failure of the
    alpha = 4 case, which is not part of this study, says nothing about the alpha = 0 level."""
    done = run / "out" / "cfd" / CASE_DIR / "done.json"
    if done.exists():
        return bool(json.loads(done.read_text())["converged"])
    log = run / "job.log"
    if log.exists():
        for ln in log.read_text().splitlines():
            if "first failed case M=2.35 alpha=0 " in ln and "not converged" in ln:
                return False
    return None


def read_level(work: pathlib.Path, name: str, wall_size: float) -> LevelData:
    run = work / name / "run"
    case = run / "out" / "cfd" / CASE_DIR
    stats = json.loads((run / "out" / "cfd" / "mesh" / "mesh_stats.json").read_text())
    cp_file = work / name / "cp" / "cone_cp.csv"
    grid = GridLevel(name, int(stats["nodes"]), {"wall_size_m": wall_size},
                     residual_drop_orders=residual_drop(case / "history.csv"),
                     solver_converged=solver_verdict(run))
    done = json.loads((run / "done.json").read_text()) if (run / "done.json").exists() else {}
    return LevelData(name, grid, int(stats["cells"]), parse_binned(cp_file), done,
                     hashlib.sha256(cp_file.read_bytes()).hexdigest()[:12])


def check_script_exact(levels: dict[str, LevelData]) -> None:
    """rans_cone.py integrates its own Taylor-Maccoll Cp; it must agree with nasa_vv.exact to the digits it prints."""
    cp = exact_cp()
    for lv in levels.values():
        if abs(lv.binned.script_cp_exact - cp) > QUANTISATION + 1e-12:
            raise ValueError(f"level {lv.name!r}: rans_cone.py printed Cp = {lv.binned.script_cp_exact} but "
                             f"nasa_vv.exact.taylor_maccoll({MACH}, {DELTA_DEG}).cp = {cp}")


# ------------------------------------------------------------------ the quantities and their GCI

QUANTITIES = (
    ("cp_mean", "0.2-0.9"),
    ("cp_mean_0.4_0.9", "0.4-0.9"),
    ("cp_nearest_tip", "0.2-0.3"),
)


def quantity_value(q: str, binned: Binned) -> float:
    if q == "cp_mean":
        return cp_mean(binned.bins, *FULL_WINDOW)
    if q == "cp_mean_0.4_0.9":
        return cp_mean(binned.bins, *LAYER0_WINDOW)
    if q == "cp_nearest_tip":
        return cp_nearest_tip(binned.bins).cp
    raise KeyError(q)


def _wall_size_compute(fd: FamilyData) -> dict[str, GridConvergence]:
    """Same gate as ``study``, then Celik's procedure with h equal to the wall_size knob."""
    fd.family.check_similar()
    h = [lv.knobs["wall_size_m"] for lv in fd.family.levels]
    return {q: grid_convergence(v[0], v[1], v[2], h[0], h[1], h[2]) for q, v in fd.values.items()}


def _nodes_compute(fd: FamilyData) -> dict[str, GridConvergence]:
    return study(fd.family, fd.values)


def family_data(levels: dict[str, LevelData]) -> FamilyData:
    fine_first = [levels[n] for n, _ in reversed(LEVELS)]
    fam = GridFamily([lv.grid for lv in fine_first], ndim=3)
    values = {q: [quantity_value(q, lv.binned) for lv in fine_first] for q, _ in QUANTITIES}
    detail = "; ".join(
        f"{lv.name}: solver_converged={lv.grid.solver_converged}, residual drop {lv.grid.residual_drop_orders:.3g} orders"
        for lv in fine_first)
    return FamilyData(fam, values, detail=detail)


def gci_rows(fd: FamilyData) -> list[dict[str, str]]:
    triplet = "/".join(lv.name for lv in fd.family.levels)
    a = _rows(STUDY, CONDITION, triplet, fd, _nodes_compute, BASES[0])
    b = _rows(STUDY, CONDITION, triplet, fd, _wall_size_compute, BASES[1])
    ecp = exact_cp()
    out = []
    for x, y in zip(a, b):
        for r in (x, y):
            t, bounds = bounds_columns(r["gci_fine_pct"], float(r["f1"]), ecp)
            r["true_error_pct"], r["gci_bounds_error"] = t, bounds
            out.append(r)
    return out


# ------------------------------------------------------------------ layer 0, as committed


def read_layer0(results: pathlib.Path) -> dict:
    """The finest cone wall_cp row of layer0.csv and the cone wall_cp row of layer0_gci.csv, unchanged."""
    rows = [r for r in _read_table(results / "layer0.csv", ("case", "level", "nodes", "quantity", "cfd", "exact", "difference_pct"))
            if r["case"] == "cone" and r["quantity"] == "wall_cp"]
    fine = max(rows, key=lambda r: int(r["nodes"]))
    g = [r for r in _read_table(results / "layer0_gci.csv", ("study", "condition", "quantity", "gci_fine_pct", "gci_bounds_error"))
         if r["study"] == "layer0" and r["condition"].startswith("cone M=") and r["quantity"] == "wall_cp"]
    if len(g) != 1:
        raise ValueError("layer0_gci.csv has no single cone wall_cp row")
    return {"fine": fine, "gci": g[0]}


# ------------------------------------------------------------------ the comparison


def pipeline_contribution_pct(layer1_diff_pct: float, layer0_diff_pct: float) -> float:
    """Percentage points: the layer 1 error minus the layer 0 error, both against the same exact value."""
    return layer1_diff_pct - layer0_diff_pct


def reading(contribution: float | None, gci0: float | None, gci1: float | None) -> str:
    """What the contribution can be said to be, given the two layers' GCI (percent, None where there is none).

    The difference is called indistinguishable at this resolution when it is smaller than the GCI of either
    layer, i.e. when at least one layer's uncertainty exceeds it. No number is quoted in that case."""
    if contribution is None:
        return "no layer 0 counterpart: nothing to subtract"
    c = abs(contribution)
    known = [g for g in (gci0, gci1) if g is not None]
    if any(c < g for g in known):
        return (f"the layers are indistinguishable at this resolution: the {CONTRIBUTION_NAME}"
                " is smaller than the GCI of at least one layer")
    if gci1 is None:
        return (f"not assessed: the layer 1 GCI is refused, so the {CONTRIBUTION_NAME}"
                " cannot be compared with its uncertainty")
    return f"the {CONTRIBUTION_NAME} exceeds the GCI of both layers"


def _f(x: float | None, spec: str = ".6g") -> str:
    return "" if x is None else format(x, spec)


def comparison_rows(levels: dict[str, LevelData], fd: FamilyData, grows: list[dict[str, str]],
                    layer0: dict) -> list[dict[str, str]]:
    ecp = exact_cp()
    fine, medium = levels["fine"], levels["medium"]
    out = []
    for q, _ in QUANTITIES:
        f1 = quantity_value(q, fine.binned)
        f2 = quantity_value(q, medium.binned)
        d1 = 100.0 * (f1 - ecp) / ecp
        for basis in BASES:
            g = next(r for r in grows if r["quantity"] == q and r["h_basis"] == basis)
            gci1 = float(g["gci_fine_pct"]) if g["gci_fine_pct"] != "" else None
            row = {c: "" for c in COMPARISON_COLUMNS}
            row.update(quantity=q, layer1_h_basis=basis, exact=_f(ecp, ".10g"),
                       layer1_fine=_f(f1, ".10g"), layer1_diff_pct=_f(d1, ".6g"),
                       layer1_gci_pct=_f(gci1),
                       layer1_medium_to_fine_change_pct=_f(100.0 * (f1 - f2) / f2, ".6g"))
            notes = []
            if q == "cp_mean_0.4_0.9":
                l0 = layer0["fine"]
                d0 = float(l0["difference_pct"])
                gci0 = float(layer0["gci"]["gci_fine_pct"]) if layer0["gci"]["gci_fine_pct"] != "" else None
                c = pipeline_contribution_pct(d1, d0)
                row.update(layer0_fine=l0["cfd"], layer0_diff_pct=l0["difference_pct"],
                           layer0_gci_pct=_f(gci0), layer0_gci_bounds_error=layer0["gci"]["gci_bounds_error"],
                           pipeline_contribution_pct=_f(c, ".6g"), reading=reading(c, gci0, gci1))
                if layer0["gci"]["gci_bounds_error"] == "False":
                    notes.append("the layer 0 GCI does not bound its own finest-grid error (layer0_gci.csv), so it is a lower bound on layer 0's uncertainty")
            else:
                row["reading"] = "no layer 0 counterpart (layer 0 measured wall_cp over x 0.4 to 0.9 only): nothing to subtract"
            if gci1 is None:
                notes.append("layer 1 GCI refused: " + g["note"])
            row["note"] = "; ".join(notes)
            out.append(row)
    return out


COMPARISON_COLUMNS = [
    "quantity", "exact", "layer0_fine", "layer0_diff_pct", "layer0_gci_pct", "layer1_fine", "layer1_diff_pct",
    "layer1_gci_pct", "pipeline_contribution_pct",
    "layer1_h_basis", "layer0_gci_bounds_error", "layer1_medium_to_fine_change_pct", "reading", "note",
]
CONE_COLUMNS = [
    "level", "nodes", "wall_size_m", "x_over_l", "cp_cfd", "cp_exact", "difference", "difference_pct",
    "residual_drop_orders", "quantity", "cells", "wall_nodes", "solver_converged",
]
GCI_OUT_COLUMNS = GCI_COLUMNS + ["true_error_pct", "gci_bounds_error"]


def cone_rows(levels: dict[str, LevelData]) -> list[dict[str, str]]:
    ecp = exact_cp()
    out = []
    for name, wall in LEVELS:
        lv = levels[name]

        def add(x_over_l: str, cp: float, quantity: str, nodes: int):
            out.append({
                "level": name, "nodes": str(lv.grid.nodes), "wall_size_m": _f(wall), "x_over_l": x_over_l,
                "cp_cfd": _f(cp, ".8g"), "cp_exact": _f(ecp, ".10g"), "difference": _f(cp - ecp, ".6g"),
                "difference_pct": _f(100.0 * (cp - ecp) / ecp, ".6g"),
                "residual_drop_orders": _f(lv.grid.residual_drop_orders, ".4g"), "quantity": quantity,
                "cells": str(lv.cells), "wall_nodes": str(nodes), "solver_converged": str(lv.grid.solver_converged),
            })

        tip = cp_nearest_tip(lv.binned.bins)
        for b in sorted(lv.binned.bins, key=lambda b: b.lo):
            add(f"{b.lo:g}-{b.hi:g}", b.cp, "cp_nearest_tip" if b is tip else "bin", b.nodes)
        for q, label in QUANTITIES[:2]:
            sel = _window(lv.binned.bins, *map(float, label.split("-")))
            add(label, quantity_value(q, lv.binned), q, sum(b.nodes for b in sel))
    return out


# ------------------------------------------------------------------ headers


def _levels_text(levels: dict[str, LevelData]) -> list[str]:
    out = []
    for name, wall in LEVELS:
        lv = levels[name]
        d = lv.done
        out.append(
            f"{name}: wall_size {wall * 1000:g} mm, {lv.grid.nodes} nodes, {lv.cells} cells, residual drop {lv.grid.residual_drop_orders:.4g} orders, "
            f"solver verdict {lv.grid.solver_converged}, job {d.get('jobid', '?')} on {d.get('node', '?')} ({d.get('ppn', '?')} cores, {d.get('wall_seconds', '?')} s, "
            f"binary commit {d.get('ignisyeet_commit', '?')}), binned Cp sha256 {lv.cp_sha}")
    return out


def _growth_text(levels: dict[str, LevelData]) -> str:
    c = [levels[n].cells for n, _ in LEVELS]
    nd = [levels[n].grid.nodes for n, _ in LEVELS]
    return (f"cells grow {c[1] / c[0]:.2f}x (coarse to medium) and {c[2] / c[1]:.2f}x (medium to fine), nodes {nd[1] / nd[0]:.2f}x and {nd[2] / nd[1]:.2f}x, "
            f"against 1.4^3 = {GROWTH_TARGET:.2f}x for a geometrically similar 1.4 refinement")


def _precision_text(levels: dict[str, LevelData]) -> str:
    parts = []
    for q, _ in QUANTITIES:
        v = [quantity_value(q, levels[n].binned) for n, _ in LEVELS]
        parts.append(f"{q} {min(abs(v[1] - v[0]), abs(v[2] - v[1])) / QUANTISATION:.3g}")
    return ("rans_cone.py writes every bin's cp_mean with five decimals (a rounding of up to 5e-6); the smaller of |f2 - f1| and |f3 - f2| in units of that rounding is "
            + ", ".join(parts) + ", so the output precision does not limit the observed order unless a value is of order 1")


def header_common(levels: dict[str, LevelData], work: pathlib.Path, extra: list[str]) -> list[str]:
    versions = (work / "versions.txt").read_text().strip().splitlines() if (work / "versions.txt").exists() else []
    notes = (work / "notes.txt").read_text().strip().splitlines() if (work / "notes.txt").exists() else []
    g0 = "research/nasa_vv/results/layer0.csv"
    g1 = "research/nasa_vv/results/layer0_gci.csv"
    lines = [
        "Layer 1 of the NASA WIND/NPARC verification study: the 10 degree cone-cylinder at M = 2.35, alpha = 0, through the whole IgnisYeet pipeline (`ignisyeet aero`: CAD profile, gmsh, SU2, coefficients), Euler, 3-D unstructured half model.",
        "Producer: python -m nasa_vv.cases.layer1_cone (research/nasa_vv/cases/layer1_cone.py), collecting CFD that was run separately; the CFD is not run by the collector.",
        "Exact reference: nasa_vv.exact.taylor_maccoll(2.35, 10.0).cp. The comparison of each layer with it is verification against an exact solution; the comparison of the two layers with each other is a cross-comparison of two models, not validation.",
        f"Layer 0 values are read, unchanged, from {g0} @ {_last_commit(g0)} (cone, finest grid) and {g1} @ {_last_commit(g1)}; layer 0 was not rerun.",
        f"The difference of the two layers' errors is reported as '{CONTRIBUTION_NAME}', never as meshing error alone: the layers differ in mesh type (structured axisymmetric against 3-D unstructured tetrahedra), in the geometry of the domain, in the code path that writes the SU2 configuration, and in how the wall Cp is extracted.",
        "",
        "Binaries. " + " | ".join(versions) if versions else "Binaries: no versions.txt in the work directory.",
        "Run: virgo_run.sh -l on virgo02, 32 cores per level, the three levels as separate jobs; each level solves the alpha = 0 and alpha = 4 cases (see below).",
    ] + _levels_text(levels) + [
        "",
        "Geometry. doc/scripts/rans_geometry.py cone (called directly with r = 0.05, delta_deg = 10.0, lb = 0.3): a 10 degree half-angle cone of base radius 0.05 m, cone length L_cone = 0.05 / tan 10 deg = 0.28356 m, a cylinder of radius 0.05 m to x = 0.58356 m, a flat base.",
        "The pipeline rebuilds the body from the extracted profile (surface = cad): (0, 0), (0.28351, 0.049991), (0.58356, 0.05), then the Euler tail fairing (tail_fairing = 6 base radii, 0.3 m) to x = 0.88356; the cone-cylinder junction is at x = 0.28351 m instead of 0.28356 m (0.02 % shorter) and the apex slope is 0.049991 / 0.28351 = tan 10.0 deg. Layer 0 has no cylinder and no tail: its cone is a bare wall y = x tan 10 deg from the tip at x = 0 to x = 1 in a domain from x = -0.2 to 1.",
        "Layer 0 reports wall_cp over the wall nodes 0.4 <= x <= 0.9 of that unit-length cone. Layer 1 bins along x / L_cone; its bins 0.4 to 0.9 are the like-for-like window (cp_mean_0.4_0.9). The rans_cone.py range 0.2 to 0.9 (cp_mean, the brief's definition) and the bin nearest the tip (cp_nearest_tip, x / L_cone 0.2 to 0.3) have no layer 0 counterpart. Layer 0's cone Cp error is dominated by the tip transient (layer0_gci.csv notes), so part of the pipeline contribution is the different placement of the window relative to the tip in cell units.",
        "Wall resolution along the cone surface, as a fraction of the cone length (layer 0: of its unit cone): layer 0 has 53 cells on x from 0 to 1 on the base grid, so 18.9e-3 on the coarse grid, 9.4e-3 on the medium and 4.7e-3 on the fine one; layer 1 has wall_size / L_cone = 13.8e-3 on the coarse, 9.9e-3 on the medium and 7.1e-3 on the fine level.",
        "Reference length and area: layer 1 REF_LENGTH = 0.1, REF_AREA = 0.003927 (half of the cross-section, half model); layer 0 REF_LENGTH = 1, REF_AREA = 1 (nasa_vv.su2cfg.layer0_config). The Cp of both is (p - p_inf) / (0.5 rho_inf V_inf^2) from the freestream state, which does not use them. Freestream: layer 1 p = 101084.97 Pa, T = 288.02 K (20 m, US1976, the product's launch altitude), layer 0 p = 101325 Pa, T = 288.15 K; the Mach number, 2.35, and gamma = 1.4 are the same.",
        "Cp: SU2 Pressure_Coefficient of the wall nodes of surface_flow.vtk whose radius is within 1 % of x tan 10 deg, binned along x / L_cone over 0.2 to 0.9 in bins of 0.1 (doc/scripts/rans_cone.py CASE --mach 2.35, unedited; its hard-coded R = 0.05 and 10 degrees match the body). cp_mean is the mean over the nodes of the bins in the range, i.e. the node-weighted mean of the bins; over 0.2 to 0.9 it is the script's own last row. cp_nearest_tip is the bin at the smallest x / L_cone.",
        _precision_text(levels) + ".",
        "The script prints its own Taylor-Maccoll Cp (0.09673 at M 2.35); it agrees with nasa_vv.exact to the five decimals it prints and only nasa_vv.exact is used for cp_exact.",
        "",
        "Grids. The grids cannot be geometrically similar. crates/cfd/src/mesh.rs:396 fixes max_size at the farfield radius / 10 and crates/cfd/src/mesh_gmsh.py:153 grades the volume mesh up to it, so refining wall_size refines only the near-wall region (the farfield radius, 20 body lengths = 11.67 m, is the same on all levels).",
        _growth_text(levels) + ".",
        "family.check_similar sees only the knobs (wall_size_m, which scales by 1.4 exactly), so it passes whether or not the grids are similar; it cannot detect this. Every GCI is therefore computed twice, once with h = nodes^(-1/3) (the family's h) and once with h = wall_size (a refinement ratio of 1.4), each with its own row and an h_basis column, and the spread between them is part of the uncertainty. Neither is the effective grid size of these grids.",
        "fin_size: the body has no fins and records none (params.json: \"fins\": null). In the mesher fin_refine = bool(fins) and hf < h0 (crates/cfd/src/mesh_gmsh.py:156-157) and the fin field is only built under it (mesh_gmsh.py:208-211 prints 'fin faces: none identified' otherwise); crates/cfd/src/mesh.rs:395 only copies fin_size_for (crates/cfd/src/config.rs:538-545) into the parameters, where it changes the case hash. A coarse mesh with fin_size = 0.00196 and one without it are identical (28887 nodes, 132328 cells). Only wall_size is therefore a knob here and fin_size is not set.",
        "alphas_deg = [0.0, 4.0] instead of the brief's [0]: both binaries refuse a single angle (aero.cfd.alphas_deg needs at least one angle above 0, crates/cfd/src/config.rs:575). Only the alpha = 0 case is used; a level's convergence is read from that case alone, a failure of the alpha = 4 case does not decide it.",
        "Generated tomls: rans_sample.py setup was run with --model euler --machs 2.35 --alphas 0 4 --wall W --ranks 16 --parallel 2 --threads 32 --memory-gb 100 --cg 0.3 --stl-scale 1; the generated case.toml has a stale duplicate wall_size line (rans_sample.py's key replacement matched a comment of examples/sample_cfd.toml), which was removed; stl and eng were made relative for the remote copy; the remote binary does not know output.html, which was removed. No SU2 setting was changed; iterations is the product's Euler default, 3000.",
        "residual_drop_orders is the peak of log10 rms[Rho] over the alpha = 0 history minus its final value (the convention of layer0.csv). solver_converged is the product's own verdict on that case (done.json 'converged', or 'not converged' in the job log of a rejected case). family.check_converged gates every level: a level the solver did not accept refuses the GCI rows, which then carry the refusal and no numbers, while the per-level Cp comparison still appears.",
    ] + notes + extra
    return ["# " + ln if ln else "#" for ln in lines]


def header_cone(levels, work, rows) -> list[str]:
    extra = [
        "Columns. level, nodes, cells: the half-model mesh the SU2 case solved. x_over_l: the bin of x / L_cone, or the range of the aggregate row. quantity: bin, cp_nearest_tip, cp_mean (0.2-0.9) or cp_mean_0.4_0.9. wall_nodes: wall nodes in the bin or range. difference = cp_cfd - cp_exact; difference_pct = 100 (cp_cfd - cp_exact) / cp_exact.",
        "solver_converged is the product's verdict on the level's alpha = 0 case (True, False, or empty if none was recorded).",
    ]
    return header_common(levels, work, extra)


def header_gci(levels, work, rows) -> list[str]:
    refused = [r for r in rows if r["note"].startswith("refused:")]
    extra = [
        "Celik et al. (2008) with Fs = 1.25 through nasa_vv.family.study (h = nodes^-1/3) and nasa_vv.gci.grid_convergence (h = wall_size), finest grid first; gci_fine_pct and gci_coarse_pct are relative to f1 and f2 (percent).",
        "n1..n3 are nodes. Each quantity has one row per h_basis. A note flags any computed order above 4 or below 0.5: the triplet is then not in the asymptotic range and its GCI is not reliable.",
        "true_error_pct = 100 |f1 - exact| / |exact| on the finest grid against Taylor-Maccoll; gci_bounds_error is True if gci_fine_pct >= true_error_pct, False otherwise. Both are empty where no GCI was computed.",
        f"Rows refused by the family gate: {len(refused)} of {len(rows)}." + (" " + refused[0]["note"] if refused else ""),
    ]
    return header_common(levels, work, extra)


def header_comparison(levels, work, rows, layer0) -> list[str]:
    extra = [
        "One row per quantity and layer 1 h basis (layer1_h_basis); layer 0 has one basis (cells^-1/2, 2-D, refinement ratio exactly 2) and the same layer 0 values repeat in both rows. layer1_fine is the fine level; its own convergence verdict is in layer1_cone.csv.",
        "diff_pct = 100 (value - exact) / exact; pipeline_contribution_pct = layer1_diff_pct - layer0_diff_pct, in percentage points, against the same Taylor-Maccoll Cp, for cp_mean_0.4_0.9 only (the like-for-like window).",
        "Reading rule: |pipeline_contribution_pct| smaller than layer0_gci_pct or layer1_gci_pct (either, i.e. smaller than the larger of the two) is called indistinguishable at this resolution and no difference is quoted. 'not assessed' means layer 1 has no GCI and layer 0's does not exceed the contribution. A GCI is an uncertainty, so 'exceeds the GCI of both layers' is the only reading that supports quoting a difference.",
        f"layer0_gci_bounds_error is the flag of layer0_gci.csv: {layer0['gci']['gci_bounds_error']} for the layer 0 cone wall_cp, whose GCI ({layer0['gci']['gci_fine_pct']} %) does not bound its own error ({layer0['fine']['difference_pct']} %), so the indistinguishable test against it is a weak one.",
        "layer1_medium_to_fine_change_pct = 100 (fine - medium) / medium, a plain description of how much the last refinement moved the value; it is not an uncertainty.",
    ]
    return header_common(levels, work, extra)


# ------------------------------------------------------------------ driver


def _write(path: pathlib.Path, header: list[str], columns: list[str], rows: list[dict[str, str]]) -> None:
    buf = io.StringIO()
    for ln in header:
        buf.write(ln + "\n")
    w = csv.DictWriter(buf, fieldnames=columns, lineterminator="\n")
    w.writeheader()
    w.writerows(rows)
    path.write_text(buf.getvalue())


def main(argv: list[str] | None = None) -> None:
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("--work", type=pathlib.Path, default=DEFAULT_WORK, help="directory holding the fetched CFD output")
    ap.add_argument("--out", type=pathlib.Path, default=RESULTS, help="directory the three CSVs are written to")
    ap.add_argument("--layer0", type=pathlib.Path, default=RESULTS, help="directory of layer0.csv and layer0_gci.csv")
    a = ap.parse_args(argv)
    work = a.work.expanduser().resolve()
    levels = {name: read_level(work, name, wall) for name, wall in LEVELS}
    check_script_exact(levels)
    fd = family_data(levels)
    layer0 = read_layer0(a.layer0)
    crows = cone_rows(levels)
    grows = gci_rows(fd)
    lrows = comparison_rows(levels, fd, grows, layer0)
    a.out.mkdir(parents=True, exist_ok=True)
    _write(a.out / "layer1_cone.csv", header_cone(levels, work, grows), CONE_COLUMNS, crows)
    _write(a.out / "layer1_gci.csv", header_gci(levels, work, grows), GCI_OUT_COLUMNS, grows)
    _write(a.out / "layer0_vs_layer1.csv", header_comparison(levels, work, lrows, layer0), COMPARISON_COLUMNS, lrows)
    print(f"wrote {a.out / 'layer1_cone.csv'}, {a.out / 'layer1_gci.csv'} and {a.out / 'layer0_vs_layer1.csv'}")


if __name__ == "__main__":
    main()
