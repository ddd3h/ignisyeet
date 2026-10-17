# SPDX-License-Identifier: GPL-3.0-or-later
"""Layer 0: SU2 with IgnisYeet's numerics on three problems that have exact solutions.

    wedge   15 degree wedge, M = 2.5      oblique_shock(2.5, 15)       wall p/p_inf, shock angle
    cone    10 degree cone,  M = 2.35     taylor_maccoll(2.35, 10)     wall Cp, wall Mach number
    pm15    15 degree expansion, M = 2.5  prandtl_meyer_expansion      wall p/p_inf, wall Mach number

Each case runs on three exactly nested body-fitted grids (65x49, 129x97 and 257x193 nodes,
see nasa_vv.mesh), all Euler. Run everything with

    cd research && uv run --project nasa_vv python -m nasa_vv.cases.layer0

SU2 output goes under ~/.cache/nasa_vv_layer0 (never /tmp, never the repository). With
``--collect-only`` the SU2 runs are skipped and the two CSVs are rebuilt from the output
already there, byte for byte the same as the first time.

Geometry
--------
The lower boundary is y_wall(x) (the axis for x < 0 of the cone), the upper boundary a
straight line. The corner or tip at x = 0 is always a node: the x sequence is uniform on
each side of it (piecewise_uniform), 21 + 43 cells for the wedge and the expansion, 11 + 53
for the cone. The inlet, the outlet and the top are MARKER_FAR (supersonic in and out, so
the characteristic boundary passes the freestream in and extrapolates out), because
layer0_config offers wall, far and sym only.

Reflections
-----------
Every wave leaves through the outlet. The wedge shock reaches the outlet at y = tan(36.9
deg) = 0.75, below the top at 1.0. The cone shock reaches r = tan(26.7 deg) = 0.50 at the
outlet. The leading characteristic of the expansion fan rises at the freestream Mach angle,
23.6 deg, and reaches y = 0.44 at the outlet. Nothing therefore meets the top boundary or
returns to the wall. The solution confirms it: ``top_dev`` in each run's diagnostics is the
largest |p/p_inf - 1| along the top.
"""
from __future__ import annotations

import argparse
import csv
import hashlib
import json
import math
import pathlib
import subprocess
import sys
from concurrent.futures import ThreadPoolExecutor
from dataclasses import dataclass
from typing import Callable

import numpy as np

from .. import exact
from ..family import GridFamily, GridLevel
from ..mesh import body_fitted_grid, piecewise_uniform, refine, structured_quad_su2
from ..su2cfg import GOLDEN_EULER, layer0_config, product_numerics
from .existing_data import COLUMNS as GCI_COLUMNS
from .existing_data import FamilyData, _flatplate_compute, _rows

RESULTS = pathlib.Path(__file__).resolve().parents[1] / "results"
DEFAULT_WORK = pathlib.Path.home() / ".cache" / "nasa_vv_layer0"
SU2_CFD = pathlib.Path.home() / ".local" / "share" / "ignisyeet-su2-native" / "bin" / "SU2_CFD"

BASE_CELLS = (64, 48)          # cells in xi and eta of the 65 x 49 base grid
LEVELS = (0, 1, 2)             # refinements of the base grid: coarse, medium, fine
ITERATIONS = 20000             # per-case budget; SU2 stops earlier at the residual target
P_INF = 101325.0               # layer0_config defaults
T_INF = 288.15
TARGET = -14.0                 # CONV_RESIDUAL_MINVAL of the golden configuration
MIN_DROP = 3.0                 # the NASA tutorial's minimum, family.MIN_RESIDUAL_DROP_ORDERS


# ------------------------------------------------------------------ cases


def _wall(slope: float) -> Callable[[np.ndarray], np.ndarray]:
    """Zero for x <= 0, slope * x after it; the corner node sits at x = 0 within round-off."""
    return lambda x: np.where(x < 1e-12, 0.0, slope * x)


@dataclass(frozen=True)
class Case:
    name: str
    mach: float
    axisymmetric: bool
    x_range: tuple[float, float]
    cells_up: int                      # base-grid cells upstream of the corner / tip
    y_wall: Callable[[np.ndarray], np.ndarray]
    y_top: float
    lower_markers: tuple[str, ...]     # one marker for the whole lower boundary, or (x < 0 part, x >= 0 part)
    config_markers: dict               # layer0_config kinds
    # measure(sol) -> ([(quantity, cfd value, exact value, window text)], diagnostics)
    measure: Callable


@dataclass
class Solution:
    X: np.ndarray
    Y: np.ndarray
    rho: np.ndarray
    u: np.ndarray
    v: np.ndarray
    p: np.ndarray
    mach_no: np.ndarray
    mach: float
    gamma: float


def _wall_mean(sol: Solution, field: np.ndarray, lo: float, hi: float, flat: dict | None = None,
               key: str = "") -> float:
    """Mean of a field over the wall nodes with lo <= x <= hi; the relative spread goes to `flat`."""
    sel = (sol.X[0] >= lo - 1e-12) & (sol.X[0] <= hi + 1e-12)
    f = field[0, sel]
    if flat is not None:
        flat[key] = float((f.max() - f.min()) / abs(f.mean()))
    return float(np.mean(f))


def _wall_window(lo: float, hi: float) -> str:
    return f"wall nodes {lo:g} <= x <= {hi:g}"


def _measure_wedge(sol: Solution):
    """([(quantity, cfd, exact, window)], diagnostics)."""
    ex = exact.oblique_shock(2.5, 15.0)
    lo, hi = 0.3, 0.9
    flat: dict = {}
    p_wall = _wall_mean(sol, sol.p, lo, hi, flat, "flat_wall_p_ratio") / P_INF
    # shock angle: on each horizontal line y = c the x where p crosses the middle of its jump
    p_mid = 0.5 * (P_INF + p_wall * P_INF)
    ys = np.round(np.arange(0.15, 0.60 + 1e-9, 0.05), 10)
    xs = []
    for c in ys:
        col_p = []
        for i in range(sol.X.shape[1]):
            if sol.Y[0, i] > c:      # the column is below the ramp at this height: no data
                col_p.append(np.nan)
            else:
                col_p.append(np.interp(c, sol.Y[:, i], sol.p[:, i]))
        col_p = np.array(col_p)
        k = int(np.argmax(col_p >= p_mid))
        if not col_p[k] >= p_mid or k == 0 or np.isnan(col_p[k - 1]):
            raise RuntimeError(f"no pressure jump found along y = {c}")
        t = (p_mid - col_p[k - 1]) / (col_p[k] - col_p[k - 1])
        xs.append(sol.X[0, k - 1] + t * (sol.X[0, k] - sol.X[0, k - 1]))
    slope, icpt = np.polyfit(xs, ys, 1)
    beta = math.degrees(math.atan(slope))
    flat.update(shock_fit_intercept=float(icpt), shock_crossings_x=[float(x) for x in xs])
    return [
        ("wall_p_ratio", p_wall, ex.p2_p1, _wall_window(lo, hi)),
        ("shock_angle_deg", beta, ex.beta_deg,
         f"{len(ys)} lines y = {ys[0]:g}..{ys[-1]:g} step 0.05; x where p = (p_inf + p_wall)/2; "
         "free least-squares line y = a x + b, angle = atan(a)"),
    ], flat


def _measure_cone(sol: Solution):
    ex = exact.taylor_maccoll(2.35, 10.0)
    lo, hi = 0.4, 0.9
    g, m = sol.gamma, sol.mach
    flat: dict = {}
    cp = 2.0 / (g * m * m) * (_wall_mean(sol, sol.p, lo, hi, flat, "flat_wall_p_ratio") / P_INF - 1.0)
    return [
        ("wall_cp", cp, ex.cp, _wall_window(lo, hi)),
        ("wall_mach", _wall_mean(sol, sol.mach_no, lo, hi, flat, "flat_wall_mach"), ex.mach_surface,
         _wall_window(lo, hi)),
    ], flat


def _measure_pm15(sol: Solution):
    ex = exact.prandtl_meyer_expansion(2.5, 15.0)
    lo, hi = 0.5, 0.9
    flat: dict = {}
    return [
        ("wall_p_ratio", _wall_mean(sol, sol.p, lo, hi, flat, "flat_wall_p_ratio") / P_INF, ex.p2_p1,
         _wall_window(lo, hi)),
        ("wall_mach", _wall_mean(sol, sol.mach_no, lo, hi, flat, "flat_wall_mach"), ex.mach2,
         _wall_window(lo, hi)),
    ], flat


_FAR = ["inlet", "outlet", "top"]
CASES: dict[str, Case] = {
    "wedge": Case("wedge", 2.5, False, (-0.5, 1.0), 21, _wall(math.tan(math.radians(15.0))), 1.0,
                  ("wall",), {"wall": ["wall"], "far": _FAR}, _measure_wedge),
    "cone": Case("cone", 2.35, True, (-0.2, 1.0), 11, _wall(math.tan(math.radians(10.0))), 1.0,
                 ("axis", "cone"), {"wall": ["cone"], "far": _FAR, "sym": ["axis"]}, _measure_cone),
    "pm15": Case("pm15", 2.5, False, (-0.5, 1.0), 21, _wall(-math.tan(math.radians(15.0))), 1.0,
                 ("wall",), {"wall": ["wall"], "far": _FAR}, _measure_pm15),
}


# ------------------------------------------------------------------ grids and configurations


def grid(case: Case, level: int):
    """(X, Y, k) of a level: node arrays and the node index of the corner along the lower boundary."""
    nxi, neta = BASE_CELLS
    xi = piecewise_uniform([0.0, -case.x_range[0] / (case.x_range[1] - case.x_range[0]), 1.0],
                           [case.cells_up, nxi - case.cells_up])
    eta = np.linspace(0.0, 1.0, neta + 1)
    for _ in range(level):
        xi, eta = refine(xi, eta)
    X, Y = body_fitted_grid(xi, eta, case.x_range, case.y_wall, case.y_top)
    return X, Y, case.cells_up * 2 ** level


def level_name(level: int) -> str:
    ni, nj = (BASE_CELLS[0] * 2 ** level + 1, BASE_CELLS[1] * 2 ** level + 1)
    return f"{ni}x{nj}"


def level_dir(work: pathlib.Path, case: Case, level: int) -> pathlib.Path:
    return work / case.name / level_name(level)


def mesh_markers(case: Case, ni: int, k: int) -> dict:
    if len(case.lower_markers) == 1:
        lower = {case.lower_markers[0]: [("ymin", 0, ni - 1)]}
    else:
        a, b = case.lower_markers
        lower = {a: [("ymin", 0, k)], b: [("ymin", k, ni - 1)]}
    return {**lower,
            "inlet": ["xmin"], "outlet": ["xmax"], "top": ["ymax"]}


OUTPUT_LINE = "OUTPUT_FILES= ( RESTART, SURFACE_CSV )"
OUTPUT_LINE_ASCII = "OUTPUT_FILES= ( RESTART_ASCII, SURFACE_CSV )"


def config_text(case: Case, iterations: int = ITERATIONS) -> str:
    """layer0_config for the case, with the restart file written as text.

    The only change to the generated configuration is the OUTPUT_FILES line, which selects
    the file format of the solution dump; the numerics are asserted unchanged.
    """
    golden = GOLDEN_EULER.read_text()
    cfg = layer0_config(
        ndim=2, axisymmetric=case.axisymmetric, mach=case.mach, markers=case.config_markers,
        viscous=False, golden=golden, analyze=["inlet", "outlet"], iterations=iterations,
        pressure=P_INF, temperature=T_INF)
    assert cfg.count(OUTPUT_LINE) == 1, "layer0_config no longer writes the expected OUTPUT_FILES line"
    out = cfg.replace(OUTPUT_LINE, OUTPUT_LINE_ASCII)
    assert product_numerics(out) == product_numerics(golden)
    changed = [(a, b) for a, b in zip(cfg.splitlines(), out.splitlines()) if a != b]
    assert changed == [(OUTPUT_LINE, OUTPUT_LINE_ASCII)]
    return out


def run_level(case: Case, level: int, work: pathlib.Path, su2: pathlib.Path) -> None:
    d = level_dir(work, case, level)
    d.mkdir(parents=True, exist_ok=True)
    for stale in ("history.csv", "restart.csv", "restart.dat", "surface_flow.csv"):
        (d / stale).unlink(missing_ok=True)
    X, Y, k = grid(case, level)
    structured_quad_su2(d / "mesh.su2", X, Y, mesh_markers(case, X.shape[1], k))
    (d / "case.cfg").write_text(config_text(case))
    with (d / "su2.log").open("w") as log:
        r = subprocess.run([str(su2), "case.cfg"], cwd=d, stdout=log, stderr=subprocess.STDOUT)
    if r.returncode != 0:
        raise RuntimeError(f"SU2_CFD failed for {case.name} {level_name(level)} (exit {r.returncode}); see {d / 'su2.log'}")


# ------------------------------------------------------------------ reading a run


def gas_constants() -> tuple[float, float]:
    """gamma and R from the golden configuration (the gas model layer0_config copies)."""
    vals = {}
    for ln in GOLDEN_EULER.read_text().splitlines():
        if ln.startswith(("GAMMA_VALUE=", "GAS_CONSTANT=")):
            k, v = ln.split("=")
            vals[k.strip()] = float(v)
    return vals["GAMMA_VALUE"], vals["GAS_CONSTANT"]


def read_history(path: pathlib.Path) -> dict:
    with path.open(newline="") as f:
        rows = list(csv.reader(f))
    head = [h.strip().strip('"') for h in rows[0]]
    data = np.array([[float(c) for c in r] for r in rows[1:]])
    col = {h: data[:, i] for i, h in enumerate(head)}
    for need in ("Avg_Massflow(inlet)", "Avg_Massflow(outlet)", "rms[Rho]", "Inner_Iter"):
        if need not in col:
            raise RuntimeError(f"{path}: column {need} is missing from the history")
    rms = col["rms[Rho]"]
    final = float(rms[-1])
    return {
        "drop": float(rms.max() - final),
        "iterations": int(col["Inner_Iter"][-1]),
        "stop": "residual target" if final <= TARGET else "iteration limit",
        "final_rms": final,
        "mass_in": float(col["Avg_Massflow(inlet)"][-1]),
        "mass_out": float(col["Avg_Massflow(outlet)"][-1]),
    }


def read_solution(case: Case, level: int, work: pathlib.Path) -> Solution:
    X, Y, _ = grid(case, level)
    nj, ni = X.shape
    raw = np.loadtxt(level_dir(work, case, level) / "restart.csv", delimiter=",", skiprows=1)
    if raw.shape[0] != ni * nj:
        raise RuntimeError(f"restart.csv has {raw.shape[0]} rows, the mesh has {ni * nj} nodes")
    raw = raw[np.argsort(raw[:, 0])]
    if not np.array_equal(raw[:, 0], np.arange(ni * nj)):
        raise RuntimeError("restart.csv does not hold every PointID once")
    rx, ry = raw[:, 1].reshape(nj, ni), raw[:, 2].reshape(nj, ni)
    if np.max(np.abs(rx - X)) > 1e-9 or np.max(np.abs(ry - Y)) > 1e-9:
        raise RuntimeError("the restart file's node order or coordinates do not match the mesh")
    g, r_gas = gas_constants()
    rho, mx, my, en = (raw[:, c].reshape(nj, ni) for c in (3, 4, 5, 6))
    u, v = mx / rho, my / rho
    p = (g - 1.0) * (en - 0.5 * rho * (u * u + v * v))
    mach_no = np.sqrt(u * u + v * v) / np.sqrt(g * p / rho)
    return Solution(X, Y, rho, u, v, p, mach_no, case.mach, g)


# ------------------------------------------------------------------ collection


def collect_level(case: Case, level: int, work: pathlib.Path) -> dict:
    hist = read_history(level_dir(work, case, level) / "history.csv")
    sol = read_solution(case, level, work)
    X, Y = sol.X, sol.Y
    meas, flat = case.measure(sol)
    top_dev = float(np.max(np.abs(sol.p[-1] / P_INF - 1.0)))
    diag = {"top_dev": top_dev, **{k: hist[k] for k in ("final_rms", "mass_in", "mass_out")}, **flat}
    # wall entropy s/s_inf = (p / rho^gamma) / (p_inf / rho_inf^gamma) over the x >= 0.4 wall nodes, and the Mach
    # number three rows off the wall there: the evidence for what the wall Mach number measures
    g = sol.gamma
    rho_inf = sol.rho[0, 0]
    s_ratio = (sol.p / sol.rho ** g) / (P_INF / rho_inf ** g)
    sel = X[0] >= 0.4
    diag["wall_entropy_ratio_x>=0.4"] = float(np.mean(s_ratio[0, sel]))
    diag["mach_row3_x>=0.4"] = float(np.mean(sol.mach_no[3, sel]))
    return {"hist": hist, "meas": meas, "nodes": X.size, "cells": (X.shape[1] - 1) * (X.shape[0] - 1),
            "diag": diag}


def _g(x: float, spec: str) -> str:
    return format(x, spec)


def build_rows(results: dict) -> list[dict[str, str]]:
    rows = []
    for name, case in CASES.items():
        for level in LEVELS:
            r = results[name][level]
            for q, cfd, ex, win in r["meas"]:
                rows.append({
                    "case": name, "mach": f"{case.mach:g}", "level": level_name(level),
                    "nodes": str(r["nodes"]), "quantity": q,
                    "cfd": _g(cfd, ".10g"), "exact": _g(ex, ".10g"),
                    "difference": _g(cfd - ex, ".6g"),
                    "difference_pct": _g(100.0 * (cfd - ex) / ex, ".4g"),
                    "residual_drop_orders": _g(r["hist"]["drop"], ".3f"),
                    "iterations": str(r["hist"]["iterations"]),
                    "stop_reason": r["hist"]["stop"], "window": win})
    return rows


LAYER0_COLUMNS = ["case", "mach", "level", "nodes", "quantity", "cfd", "exact", "difference",
                  "difference_pct", "residual_drop_orders", "iterations", "stop_reason", "window"]


def gci_rows(results: dict) -> list[dict[str, str]]:
    """constant_ratio(r = 2) per case and quantity, through the same gate and row writer as gci_existing.csv."""
    out = []
    for name, case in CASES.items():
        fine_first = list(reversed(LEVELS))
        levels = []
        for lv in fine_first:
            r = results[name][lv]
            nx, ny = BASE_CELLS[0] * 2 ** lv, BASE_CELLS[1] * 2 ** lv
            levels.append(GridLevel(level_name(lv), r["cells"], {"dx": 1.0 / nx, "dy": 1.0 / ny},
                                    residual_drop_orders=r["hist"]["drop"]))
        qs = [m[0] for m in results[name][LEVELS[0]]["meas"]]
        values = {q: [next(m[1] for m in results[name][lv]["meas"] if m[0] == q) for lv in fine_first]
                  for q in qs}
        extra = ""
        stalled = [level_name(lv) for lv in fine_first if results[name][lv]["hist"]["stop"] != "residual target"]
        if stalled:
            extra = "; ".join(f"grid {n} stopped at the iteration limit" for n in stalled)
        fd = FamilyData(GridFamily(levels, ndim=2), values, row_note=extra)
        trip = "/".join(lv.name for lv in levels)
        out += _rows("layer0", f"{name} M={case.mach:g}", trip, fd, _flatplate_compute, "cells^-1/2 (2-D)")
    return out


def _sha(path: pathlib.Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()[:12]


def _header_common() -> list[str]:
    return [
        "Layer 0 of the NASA WIND/NPARC verification study: SU2 8.3.0 (ignisyeet-su2-native), Euler, run directly with the numerics of a configuration `ignisyeet aero` wrote.",
        f"Producer: python -m nasa_vv.cases.layer0 (research/nasa_vv/cases/layer0.py). Numerics and gas model: copied by nasa_vv.su2cfg.layer0_config from tests/data/golden_euler.cfg (sha256 {_sha(GOLDEN_EULER)}).",
        "Exact references: nasa_vv.exact.oblique_shock(2.5, 15), taylor_maccoll(2.35, 10), prandtl_meyer_expansion(2.5, 15).",
        "Grids: body-fitted, structured, exactly nested (every coarse node is a fine node); 65x49, 129x97 and 257x193 nodes, i.e. 64x48, 128x96 and 256x192 cells, so the refinement ratio is exactly 2.",
        "Boundaries: inlet, outlet and top are MARKER_FAR (supersonic in and out); the lower boundary is an Euler wall; for the cone the axis (x < 0) is MARKER_SYM under AXISYMMETRIC= YES.",
        "The lower boundary of the wedge and of the expansion is one MARKER_EULER marker. Split in two at the expansion corner, the corner node became a stagnation point (u = v = 0, p = 3 p_inf) and rms[Rho] stalled near -1.5.",
        "Every wave leaves through the outlet (wedge shock at y = 0.75, cone shock at r = 0.50, expansion fan head at y = 0.44 on x = 1); none reaches the top or returns to the sampled wall.",
        "The only edit to the layer0_config output is OUTPUT_FILES= ( RESTART_ASCII, SURFACE_CSV ) in place of RESTART, so the solution can be read as text; the numerics are asserted identical to the golden's.",
        "ITER = 20000 (a per-case budget, not a numerics key); CONV_RESIDUAL_MINVAL = -14 on rms[Rho] stops a run earlier.",
        "Every run records Avg_Massflow(inlet) and Avg_Massflow(outlet) in its history.csv; they are not analysed here.",
    ]


def write_layer0(rows, path: pathlib.Path) -> None:
    lines = _header_common() + [
        "cfd is the mean of the nodal values in the window (wall nodes of the lower boundary for wall quantities); window is stated per row.",
        "p/p_inf uses p_inf = 101325 Pa; wall_cp = 2 (p/p_inf - 1) / (gamma M^2); wall_mach is the local Mach number of the wall node.",
        "shock_angle_deg: on each horizontal line y = c the x where p crosses (p_inf + p_wall)/2, linearly interpolated along the grid columns, with p_wall the CFD wall plateau of the same grid; a free least-squares line through the points, angle atan(slope).",
        "wall_mach is the Mach number of the wall nodes. Those nodes carry an entropy excess; on the expansion it is the same on all three grids, so wall_mach there measures that excess and not the isentropic Mach number. The wall pressure is unaffected.",
        "difference = cfd - exact; difference_pct = 100 (cfd - exact) / exact.",
        "nodes: nodes of the grid; level: nodes in each direction.",
        "residual_drop_orders: the peak of log10 rms[Rho] over the history minus its final value (the convention of rans_flatplate.csv).",
        "iterations: the last inner iteration in history.csv.",
        "stop_reason: 'residual target' if the final rms[Rho] is at most -14, otherwise 'iteration limit'.",
    ]
    with path.open("w", newline="") as f:
        for ln in lines:
            f.write("# " + ln + "\n")
        w = csv.DictWriter(f, fieldnames=LAYER0_COLUMNS, lineterminator="\n")
        w.writeheader()
        w.writerows(rows)


def write_gci(rows, path: pathlib.Path) -> None:
    lines = _header_common() + [
        "Constant-ratio grid convergence (nasa_vv.gci.constant_ratio, r = 2, Fs = 1.25), finest grid first, through the same gate and row writer as gci_existing.csv.",
        "h_basis cells^-1/2 (2-D); n1..n3 are cells (the cells grow exactly 4x per level, so r21 = r32 = 2).",
        "gci_fine_pct and gci_coarse_pct are relative to f1 and f2 (percent).",
        "A level below 3 orders of residual drop is refused (family.MIN_RESIDUAL_DROP_ORDERS) and its quantities carry the refusal in note.",
        "A note flags any computed order above 4 (twice the formal second order) or below 0.5 (a quarter of it): the triplet is then not in the asymptotic range and its GCI is not reliable.",
        "The shock angle is a fit to crossings located on a vertex grid that does not follow the shock; the cone and expansion corner are singular at the tip and corner.",
    ]
    with path.open("w", newline="") as f:
        for ln in lines:
            f.write("# " + ln + "\n")
        w = csv.DictWriter(f, fieldnames=GCI_COLUMNS, lineterminator="\n")
        w.writeheader()
        w.writerows(rows)


# ------------------------------------------------------------------ main


def main(argv: list[str] | None = None) -> None:
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("--work", type=pathlib.Path, default=DEFAULT_WORK, help="scratch directory for meshes and SU2 output")
    ap.add_argument("--collect-only", action="store_true", help="skip the SU2 runs; read the output already in --work")
    ap.add_argument("--jobs", type=int, default=4, help="SU2 runs at once")
    ap.add_argument("--su2", type=pathlib.Path, default=SU2_CFD)
    ap.add_argument("--cases", nargs="*", default=list(CASES), choices=list(CASES))
    ap.add_argument("--out", type=pathlib.Path, default=RESULTS)
    a = ap.parse_args(argv)
    work = a.work.expanduser().resolve()
    if str(work).startswith("/tmp"):
        sys.exit("--work must not be under /tmp (RAM-backed here)")

    if not a.collect_only:
        jobs = [(CASES[c], lv) for lv in reversed(LEVELS) for c in a.cases]   # finest first: the long ones start early
        with ThreadPoolExecutor(max_workers=a.jobs) as ex:
            futs = [(c.name, lv, ex.submit(run_level, c, lv, work, a.su2)) for c, lv in jobs]
            for n, lv, f in futs:
                f.result()
                print(f"ran {n} {level_name(lv)}", flush=True)

    results = {name: {lv: collect_level(CASES[name], lv, work) for lv in LEVELS} for name in a.cases}
    if set(a.cases) != set(CASES):
        # a partial run prints but writes nothing
        for name in a.cases:
            for lv in LEVELS:
                r = results[name][lv]
                print(name, level_name(lv), r["hist"], [(q, round(c, 6), round(e, 6)) for q, c, e, _ in r["meas"]], r["diag"])
        return
    a.out.mkdir(parents=True, exist_ok=True)
    write_layer0(build_rows(results), a.out / "layer0.csv")
    write_gci(gci_rows(results), a.out / "layer0_gci.csv")
    diag = {n: {level_name(lv): {"hist": results[n][lv]["hist"], **results[n][lv]["diag"]} for lv in LEVELS}
            for n in results}
    (work / "diagnostics.json").write_text(json.dumps(diag, indent=1, sort_keys=True) + "\n")
    print(f"wrote {a.out / 'layer0.csv'} and {a.out / 'layer0_gci.csv'}; diagnostics in {work / 'diagnostics.json'}")


if __name__ == "__main__":
    main()
