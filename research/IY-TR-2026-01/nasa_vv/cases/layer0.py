# SPDX-License-Identifier: GPL-3.0-or-later
"""Layer 0: SU2 with IgnisYeet's numerics on three problems that have exact solutions.

    wedge   15 degree wedge, M = 2.5      oblique_shock(2.5, 15)       wall p/p_inf, shock angle
    cone    10 degree cone,  M = 2.35     taylor_maccoll(2.35, 10)     wall Cp, wall Mach number (also cone_sym_axis)
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

The whole lower boundary is one MARKER_EULER marker. Where a convex corner (the expansion) or the
cone tip (axis and cone surface) joined two markers, the junction node became a stagnation node
(u = v = 0). One marker removes it, which is consistent with the junction node receiving both
boundary conditions (the axis imposes v = 0, the wall zero normal velocity) but does not prove it.
The cone is therefore run twice: 'cone' with axis and surface as one Euler wall (an inviscid slip
wall on r = 0 imposes v = 0, as symmetry does), and 'cone_sym_axis' with the axis as MARKER_SYM,
kept to show the effect.

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
import io
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
from ..gci import constant_ratio
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
    rho_inf: float

    @property
    def entropy(self) -> np.ndarray:
        """s / s_inf = (p / rho^gamma) / (p_inf / rho_inf^gamma)."""
        g = self.gamma
        return (self.p / self.rho ** g) / (P_INF / self.rho_inf ** g)


@dataclass
class Meas:
    """One row of layer0.csv: a compared quantity, or (diag) a number that says what a compared one measures."""
    q: str
    cfd: float
    exact: float | None
    window: str
    spread: float | None = None
    diag: bool = False


@dataclass
class Measured:
    meas: list
    alt: dict       # quantity -> {(lo, hi): value} over the alternative wall windows


ALT_WINDOWS = ((0.3, 1.0), (0.4, 1.0), (0.5, 1.0), (0.6, 1.0))
"""Wall windows of the sensitivity check; the end x = 1 is the outlet node."""
UPSTREAM_WINDOW = {False: (-0.4, -0.1), True: (-0.15, -0.05)}   # wall x < 0, away from the inlet and the corner / tip
ROW_OFF_WALL = 10
ROW_NEAR_WALL = 3


def _sel(sol: Solution, lo: float, hi: float) -> np.ndarray:
    return (sol.X[0] >= lo - 1e-12) & (sol.X[0] <= hi + 1e-12)


def _window(sol: Solution, lo: float, hi: float, row: int = 0) -> str:
    x = sol.X[0][_sel(sol, lo, hi)]
    where = "wall" if row == 0 else f"row {row} off the wall"
    return f"{where} nodes {lo:g} <= x <= {hi:g} (x = {x[0]:.4f}..{x[-1]:.4f}; {len(x)} nodes)"


def _stat(sol: Solution, field: np.ndarray, lo: float, hi: float, row: int = 0) -> tuple[float, float]:
    """Mean of a field over the nodes of a row with lo <= x <= hi, and its relative spread (max - min) / |mean|."""
    f = field[row, _sel(sol, lo, hi)]
    return float(np.mean(f)), float((f.max() - f.min()) / abs(f.mean()))


def _exact_entropy(kind: str) -> float:
    """s / s_inf of the exact region the wall window lies in."""
    g = 1.4
    if kind == "wedge":
        ex = exact.oblique_shock(2.5, 15.0)
        return ex.p2_p1 / ex.rho2_rho1 ** g
    if kind == "cone":
        ex = exact.taylor_maccoll(2.35, 10.0)
        return ex.p_pinf / ex.rho_rhoinf ** g     # the entropy is constant along the surface streamline
    return 1.0                                    # the expansion is isentropic


def _diagnostics(sol: Solution, kind: str, lo: float, hi: float, mach_exact: float | None) -> list[Meas]:
    """Rows that say what the wall values contain: entropy, near-wall Mach number, the corner or tip node."""
    s = sol.entropy
    s_ex = _exact_entropy(kind)
    ul, uh = UPSTREAM_WINDOW[kind == "cone"]
    rows = []
    m, sp = _stat(sol, s, ul, uh)
    rows.append(Meas("diag_entropy_wall_upstream", m, 1.0, _window(sol, ul, uh), sp, True))
    m, sp = _stat(sol, s, 0.0, 0.1)
    rows.append(Meas("diag_entropy_wall_tip_0_0.1", m, None, _window(sol, 0.0, 0.1), sp, True))
    m, sp = _stat(sol, s, lo, hi)
    rows.append(Meas("diag_entropy_wall_window", m, s_ex, _window(sol, lo, hi), sp, True))
    m, sp = _stat(sol, s, lo, hi, ROW_OFF_WALL)
    rows.append(Meas(f"diag_entropy_row{ROW_OFF_WALL}_window", m, s_ex, _window(sol, lo, hi, ROW_OFF_WALL), sp, True))
    m, sp = _stat(sol, sol.mach_no, lo, hi, ROW_NEAR_WALL)
    rows.append(Meas(f"diag_mach_row{ROW_NEAR_WALL}_window", m, mach_exact, _window(sol, lo, hi, ROW_NEAR_WALL), sp, True))
    k = int(np.argmin(np.abs(sol.X[0])))
    assert abs(sol.X[0, k]) < 1e-12, "the corner or tip is not a node"
    rows.append(Meas("diag_corner_node_mach", float(sol.mach_no[0, k]), None, "wall node x = 0", None, True))
    rows.append(Meas("diag_corner_node_p_ratio", float(sol.p[0, k] / P_INF), None, "wall node x = 0", None, True))
    return rows


def _measure_wedge(sol: Solution) -> Measured:
    ex = exact.oblique_shock(2.5, 15.0)
    lo, hi = 0.3, 0.9
    p_wall, sp = _stat(sol, sol.p, lo, hi)
    p_wall /= P_INF
    # shock angle: on each horizontal line y = c the x where p crosses the middle of its jump
    p_mid = 0.5 * (P_INF + p_wall * P_INF)
    ys = np.round(np.arange(0.15, 0.60 + 1e-9, 0.05), 10)
    xs = []
    for c in ys:
        col_p = np.array([np.nan if sol.Y[0, i] > c else np.interp(c, sol.Y[:, i], sol.p[:, i])
                          for i in range(sol.X.shape[1])])   # a column below the ramp at this height has no data
        k = int(np.argmax(col_p >= p_mid))
        if not col_p[k] >= p_mid or k == 0 or np.isnan(col_p[k - 1]):
            raise RuntimeError(f"no pressure jump found along y = {c}")
        t = (p_mid - col_p[k - 1]) / (col_p[k] - col_p[k - 1])
        xs.append(sol.X[0, k - 1] + t * (sol.X[0, k] - sol.X[0, k - 1]))
    slope, icpt = np.polyfit(xs, ys, 1)
    beta = math.degrees(math.atan(slope))
    # scatter about the fit: the angle of the line from the fit's own foot (x0, 0) to each crossing
    x0 = -icpt / slope
    ang = np.degrees(np.arctan2(ys, np.array(xs) - x0))
    spread = float((ang.max() - ang.min()) / ang.mean())
    shock_win = (f"{len(ys)} lines y = {ys[0]:g}..{ys[-1]:g} step 0.05; x where p = (p_inf + p_wall)/2; "
                 "free least-squares line y = a x + b, angle = atan(a); window_spread: spread of the angles from "
                 "the line's foot to each crossing")
    meas = [
        Meas("wall_p_ratio", p_wall, ex.p2_p1, _window(sol, lo, hi), sp),
        Meas("shock_angle_deg", beta, ex.beta_deg, shock_win, spread),
        Meas("diag_shock_fit_intercept", float(icpt), 0.0, "y of the fitted shock line at x = 0 (the leading edge)", None, True),
    ] + _diagnostics(sol, "wedge", lo, hi, ex.mach2)
    alt = {"wall_p_ratio": {w: _stat(sol, sol.p, *w)[0] / P_INF for w in ALT_WINDOWS}}
    return Measured(meas, alt)


def _measure_cone(sol: Solution) -> Measured:
    ex = exact.taylor_maccoll(2.35, 10.0)
    lo, hi = 0.4, 0.9
    g, m = sol.gamma, sol.mach
    cp_of = lambda p_ratio: 2.0 / (g * m * m) * (p_ratio - 1.0)
    pm, psp = _stat(sol, sol.p, lo, hi)
    mm, msp = _stat(sol, sol.mach_no, lo, hi)
    meas = [
        Meas("wall_cp", cp_of(pm / P_INF), ex.cp, _window(sol, lo, hi), psp),
        Meas("wall_mach", mm, ex.mach_surface, _window(sol, lo, hi), msp),
    ] + _diagnostics(sol, "cone", lo, hi, None)
    alt = {"wall_cp": {w: cp_of(_stat(sol, sol.p, *w)[0] / P_INF) for w in ALT_WINDOWS},
           "wall_mach": {w: _stat(sol, sol.mach_no, *w)[0] for w in ALT_WINDOWS}}
    return Measured(meas, alt)


def _measure_pm15(sol: Solution) -> Measured:
    ex = exact.prandtl_meyer_expansion(2.5, 15.0)
    lo, hi = 0.5, 0.9
    pm, psp = _stat(sol, sol.p, lo, hi)
    mm, msp = _stat(sol, sol.mach_no, lo, hi)
    meas = [
        Meas("wall_p_ratio", pm / P_INF, ex.p2_p1, _window(sol, lo, hi), psp),
        Meas("wall_mach", mm, ex.mach2, _window(sol, lo, hi), msp),
    ] + _diagnostics(sol, "pm15", lo, hi, ex.mach2)
    alt = {"wall_p_ratio": {w: _stat(sol, sol.p, *w)[0] / P_INF for w in ALT_WINDOWS},
           "wall_mach": {w: _stat(sol, sol.mach_no, *w)[0] for w in ALT_WINDOWS}}
    return Measured(meas, alt)


_FAR = ["inlet", "outlet", "top"]
_TAN10 = math.tan(math.radians(10.0))
CASES: dict[str, Case] = {
    "wedge": Case("wedge", 2.5, False, (-0.5, 1.0), 21, _wall(math.tan(math.radians(15.0))), 1.0,
                  ("wall",), {"wall": ["wall"], "far": _FAR}, _measure_wedge),
    # the cone's axis is part of the one Euler wall marker: see the module docstring
    "cone": Case("cone", 2.35, True, (-0.2, 1.0), 11, _wall(_TAN10), 1.0,
                 ("wall",), {"wall": ["wall"], "far": _FAR}, _measure_cone),
    # the axis as MARKER_SYM and the cone surface as MARKER_EULER, meeting at the tip node
    "cone_sym_axis": Case("cone_sym_axis", 2.35, True, (-0.2, 1.0), 11, _wall(_TAN10), 1.0,
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


def config_text(case: Case, iterations: int = ITERATIONS, analyze: list[str] | None = None) -> str:
    """layer0_config for the case, with the restart file written as text.

    ``analyze`` names the markers whose mass flux SU2 reports; the default is the inlet and the outlet.

    The only change to the generated configuration is the OUTPUT_FILES line, which selects
    the file format of the solution dump; the numerics are asserted unchanged.
    """
    golden = GOLDEN_EULER.read_text()
    cfg = layer0_config(
        ndim=2, axisymmetric=case.axisymmetric, mach=case.mach, markers=case.config_markers,
        viscous=False, golden=golden, analyze=["inlet", "outlet"] if analyze is None else analyze,
        iterations=iterations,
        pressure=P_INF, temperature=T_INF)
    assert cfg.count(OUTPUT_LINE) == 1, "layer0_config no longer writes the expected OUTPUT_FILES line"
    out = cfg.replace(OUTPUT_LINE, OUTPUT_LINE_ASCII)
    assert product_numerics(out) == product_numerics(golden)
    changed = [(a, b) for a, b in zip(cfg.splitlines(), out.splitlines()) if a != b]
    assert changed == [(OUTPUT_LINE, OUTPUT_LINE_ASCII)]
    return out


def run_level(case: Case, level: int, work: pathlib.Path, su2: pathlib.Path, analyze: list[str] | None = None) -> None:
    d = level_dir(work, case, level)
    d.mkdir(parents=True, exist_ok=True)
    for stale in ("history.csv", "restart.csv", "restart.dat", "surface_flow.csv"):
        (d / stale).unlink(missing_ok=True)
    X, Y, k = grid(case, level)
    structured_quad_su2(d / "mesh.su2", X, Y, mesh_markers(case, X.shape[1], k))
    (d / "case.cfg").write_text(config_text(case, analyze=analyze))
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
    return Solution(X, Y, rho, u, v, p, mach_no, case.mach, g, P_INF / (r_gas * T_INF))


# ------------------------------------------------------------------ collection


TOP_TOLERANCE = 1e-6
"""Largest |p/p_inf - 1| along the top boundary that still counts as 'no wave reached it'."""


def collect_level(case: Case, level: int, work: pathlib.Path) -> dict:
    hist = read_history(level_dir(work, case, level) / "history.csv")
    sol = read_solution(case, level, work)
    top_dev = float(np.max(np.abs(sol.p[-1] / P_INF - 1.0)))
    if not top_dev < TOP_TOLERANCE:
        raise RuntimeError(
            f"{case.name} {level_name(level)}: p/p_inf deviates by {top_dev:.3g} along the top boundary; "
            "a wave reached it, so the domain is too small for the sampled wall")
    m = case.measure(sol)
    return {"hist": hist, "meas": m.meas, "alt": m.alt, "nodes": sol.X.size,
            "cells": (sol.X.shape[1] - 1) * (sol.X.shape[0] - 1), "top_dev": top_dev}


def _g(x: float, spec: str) -> str:
    return format(x, spec)


def build_rows(results: dict) -> list[dict[str, str]]:
    rows = []
    for name, case in CASES.items():
        for level in LEVELS:
            r = results[name][level]
            for m in r["meas"]:
                has_exact = m.exact is not None
                rows.append({
                    "case": name, "mach": f"{case.mach:g}", "level": level_name(level),
                    "nodes": str(r["nodes"]), "quantity": m.q,
                    "cfd": _g(m.cfd, ".10g"), "exact": _g(m.exact, ".10g") if has_exact else "",
                    "difference": _g(m.cfd - m.exact, ".6g") if has_exact else "",
                    "difference_pct": _g(100.0 * (m.cfd - m.exact) / m.exact, ".4g") if has_exact and m.exact != 0.0 else "",
                    "residual_drop_orders": _g(r["hist"]["drop"], ".3f"),
                    "iterations": str(r["hist"]["iterations"]),
                    "stop_reason": r["hist"]["stop"], "window": m.window,
                    "window_spread": "" if m.spread is None else _g(m.spread, ".3g")})
    return rows


LAYER0_COLUMNS = ["case", "mach", "level", "nodes", "quantity", "cfd", "exact", "difference",
                  "difference_pct", "residual_drop_orders", "iterations", "stop_reason", "window",
                  "window_spread"]

CAUSE = {
    ("wedge", "wall_p_ratio"):
        "consistent with the discretisation error of the wall pressure behind the shock; the order is below 1 "
        "here because the errors do not follow a pure power law (difference ratio against error ratio below)",
    ("wedge", "shock_angle_deg"):
        "consistent with an error in the position of a captured shock, smeared over a few cells, rather than a "
        "smooth-field error; the intercept of the fitted line (diag_shock_fit_intercept) falls with the grid",
    ("cone", "wall_cp"):
        "consistent with the decay of the cone-tip transient along the wall rather than a discretisation order "
        "of the conical plateau",
    ("cone", "wall_mach"):
        "consistent with the decay of a wall entropy layer that starts at the tip (diag_entropy_wall_window falls "
        "towards the exact value with the grid) rather than a discretisation order of the isentropic Mach number",
    ("pm15", "wall_p_ratio"):
        "consistent with the decaying tail of the corner disturbance along the wall and the placement of the "
        "window rather than a discretisation order of the uniform region",
    ("pm15", "wall_mach"):
        "the wall Mach number does not approach the exact value: the wall entropy is the same on all three grids "
        "(diag_entropy_wall_window), a constant offset consistent with entropy generated at the corner and "
        "convected along the wall; there is no order to quote",
}
CAUSE[("cone_sym_axis", "wall_cp")] = (
    CAUSE[("cone", "wall_cp")] + "; the tip node is a stagnation node here (diag_corner_node_mach = 0)")
CAUSE[("cone_sym_axis", "wall_mach")] = (
    CAUSE[("cone", "wall_mach")] + "; the tip node is a stagnation node here (diag_corner_node_mach = 0)")

STABLE_RANGE = 0.5
"""The observed order is called stable across windows if the orders of the four wall windows lie within this range."""


def window_orders(alt: dict) -> list:
    """Observed order (or None) of each alternative wall window; alt is {level: {window: value}}."""
    return [constant_ratio(alt[2][w], alt[1][w], alt[0][w], 2.0).order for w in ALT_WINDOWS]


def window_label(orders: list) -> tuple[str, str]:
    """('stable' | 'not stable', 'lo to hi' or 'undefined') for the orders of the alternative windows."""
    have = [o for o in orders if o is not None]
    if len(have) < len(orders):
        return "not stable", "undefined for " + str(len(orders) - len(have)) + " window(s)"
    span = f"{min(have):.2f} to {max(have):.2f}"
    return ("stable" if max(have) - min(have) <= STABLE_RANGE else "not stable"), span


def bounds_columns(gci_fine_pct: str, f1: float, exact_value: float | None) -> tuple[str, str]:
    """(true_error_pct, gci_bounds_error) of a GCI row; both empty without a computed GCI or an exact value."""
    if gci_fine_pct == "" or exact_value is None or exact_value == 0.0:
        return "", ""
    true_pct = 100.0 * abs(f1 - exact_value) / abs(exact_value)
    return f"{true_pct:.6g}", str(float(gci_fine_pct) >= true_pct)


def gci_note(name: str, q: str, fine_first: list[float], exact_value: float, alt: dict | None) -> str:
    """What the observed order of one quantity measures, with the numbers that support it."""
    f1, f2, f3 = fine_first
    parts = [CAUSE[(name, q)]]
    ec, em, ef = f3 - exact_value, f2 - exact_value, f1 - exact_value
    parts.append(
        f"errors against exact (coarse, medium, fine) {ec:.3g}, {em:.3g}, {ef:.3g}; error ratios "
        f"{abs(ec / em):.2f} and {abs(em / ef):.2f}; difference ratio {abs((f3 - f2) / (f2 - f1)):.2f}")
    g = constant_ratio(f1, f2, f3, 2.0)
    if g.extrapolated is not None and (g.extrapolated - exact_value) * ef < 0.0:
        parts.append(f"the extrapolated value {g.extrapolated:.6g} lies beyond the exact value {exact_value:.6g} "
                     f"(by {abs(g.extrapolated - exact_value):.3g}; the fine-grid error is {abs(ef):.3g})")
    if alt:
        orders = window_orders(alt)
        shown = ", ".join("undefined" if o is None else f"{o:.2f}" for o in orders)
        label, span = window_label(orders)
        parts.append("window sensitivity: observed order over the wall windows "
                     + ", ".join(f"[{w[0]:g}, {w[1]:g}]" for w in ALT_WINDOWS)
                     + f" is {shown} (range {span}): {label} (stable means within {STABLE_RANGE:g})")
    return "; ".join(parts)


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
        stalled = [level_name(lv) for lv in fine_first if results[name][lv]["hist"]["stop"] != "residual target"]
        stall_note = "; ".join(f"grid {n} stopped at the iteration limit" for n in stalled)
        trip = "/".join(lv.name for lv in levels)
        for m0 in results[name][LEVELS[0]]["meas"]:
            if m0.diag:
                continue
            q = m0.q
            vals = [next(m.cfd for m in results[name][lv]["meas"] if m.q == q) for lv in fine_first]
            alt = ({lv: results[name][lv]["alt"][q] for lv in LEVELS} if q in results[name][LEVELS[0]]["alt"] else None)
            note = "; ".join(x for x in (stall_note, gci_note(name, q, vals, m0.exact, alt)) if x)
            fd = FamilyData(GridFamily(levels, ndim=2), {q: vals}, row_note=note)
            for row in _rows("layer0", f"{name} M={case.mach:g}", trip, fd, _flatplate_compute, "cells^-1/2 (2-D)"):
                row["true_error_pct"], row["gci_bounds_error"] = bounds_columns(row["gci_fine_pct"], vals[0], m0.exact)
                if row["gci_bounds_error"] == "False":
                    row["note"] = (f"the GCI ({float(row['gci_fine_pct']):.3g} %) is smaller than the observed finest-grid "
                                   f"error ({row['true_error_pct']} %): it does not bound the error and must not be "
                                   "quoted as the uncertainty of this quantity; " + row["note"])
                out.append(row)
    return out


def _sha(path: pathlib.Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()[:12]


def _header_common(results: dict) -> list[str]:
    top = max(r["top_dev"] for per_case in results.values() for r in per_case.values())
    nruns = sum(len(v) for v in results.values())
    return [
        "Layer 0 of the NASA WIND/NPARC verification study: SU2 8.3.0 (ignisyeet-su2-native), Euler, run directly with the numerics of a configuration `ignisyeet aero` wrote.",
        f"Producer: python -m nasa_vv.cases.layer0 (research/IY-TR-2026-01/nasa_vv/cases/layer0.py). Numerics and gas model: copied by nasa_vv.su2cfg.layer0_config from tests/data/golden_euler.cfg (sha256 {_sha(GOLDEN_EULER)}).",
        "Exact references: nasa_vv.exact.oblique_shock(2.5, 15), taylor_maccoll(2.35, 10), prandtl_meyer_expansion(2.5, 15).",
        "Grids: body-fitted, structured, exactly nested (every coarse node is a fine node); 65x49, 129x97 and 257x193 nodes, i.e. 64x48, 128x96 and 256x192 cells, so the refinement ratio is exactly 2.",
        "Boundaries: inlet, outlet and top are MARKER_FAR (supersonic in and out); the lower boundary is an Euler wall.",
        "The lower boundary of the wedge and of the expansion is one MARKER_EULER marker. Split in two at the expansion corner, the corner node became a stagnation point (u = v = 0, p = 3 p_inf) and rms[Rho] stalled near -1.5.",
        "The cone is run twice. 'cone' has the whole lower boundary, axis and cone surface, as one MARKER_EULER under AXISYMMETRIC= YES (SU2 8.3.0 accepts it without a message); an inviscid slip wall on r = 0 imposes v = 0, the same as symmetry. 'cone_sym_axis' has the axis (x < 0) as MARKER_SYM and the cone surface as MARKER_EULER. There the tip node, which belongs to both markers, is a stagnation node on every grid (diag_corner_node_mach = 0), and 'cone' is the layer 0 result.",
        f"Every wave leaves through the outlet (wedge shock at y = 0.75, cone shock at r = 0.50, expansion fan head at y = 0.44 on x = 1). Checked in each of the {nruns} runs: the largest |p/p_inf - 1| along the top boundary is {top:.1e}, and the collector refuses a run above {TOP_TOLERANCE:g}.",
        "The only edit to the layer0_config output is OUTPUT_FILES= ( RESTART_ASCII, SURFACE_CSV ) in place of RESTART, so the solution can be read as text; the numerics are asserted identical to the golden's.",
        "ITER = 20000 (a per-case budget, not a numerics key); CONV_RESIDUAL_MINVAL = -14 on rms[Rho] stops a run earlier.",
        "Every run records Avg_Massflow(inlet) and Avg_Massflow(outlet) in its history.csv; they are not analysed here.",
    ]


def _stability_text(results: dict) -> str:
    """Per case and wall quantity: the range of the observed order over the alternative windows and its label."""
    out = []
    for name in CASES:
        for q in results[name][LEVELS[0]]["alt"]:
            label, span = window_label(window_orders({lv: results[name][lv]["alt"][q] for lv in LEVELS}))
            out.append(f"{name} {q} {span} ({label})")
    return "Window-to-window range of the observed order: " + "; ".join(out) + "."


def _wedge_entropy_text(results: dict) -> str:
    def vals(q):
        return [next(m for m in results["wedge"][lv]["meas"] if m.q == q) for lv in LEVELS]
    wall, row10 = vals("diag_entropy_wall_window"), vals("diag_entropy_row10_window")
    return (f"On the wedge the wall reads s/s_inf = {', '.join(f'{m.cfd:.4f}' for m in wall)} on 65x49, 129x97 and 257x193, "
            f"ten rows off the wall {', '.join(f'{m.cfd:.4f}' for m in row10)}, and the exact post-shock value is {wall[0].exact:.4f}: "
            f"a grid-independent excess of {100 * (wall[-1].cfd / wall[-1].exact - 1):.1f} per cent at the wall.")


def write_layer0(rows, results: dict, path: pathlib.Path) -> None:
    icpt = [results["wedge"][lv]["meas"][2].cfd for lv in LEVELS]
    lines = _header_common(results) + [
        "cfd is the mean of the nodal values in the window (wall nodes of the lower boundary for wall quantities); window is stated per row with the first and last node used.",
        "The window is x-bounded and its end is not a node, so the node set differs slightly by level (for 0.3 <= x <= 0.9 the last node is x = 0.8837 on 65x49 and 0.8953 on the finer two).",
        "window_spread is the relative spread (max - min) / |mean| of the quantity over the window's nodes. It measures the variation along the window and says nothing about whether the error is a discretisation error. For shock_angle_deg it is the spread of the angles from the fitted line's foot to each crossing.",
        "The test of whether an error is a discretisation error is the stability of the observed order when the window moves (layer0_gci.csv notes): the order of the four wall windows [0.3, 1], [0.4, 1], [0.5, 1], [0.6, 1] is called stable if they lie within "
        f"{STABLE_RANGE:g} of each other. " + _stability_text(results),
        "p/p_inf uses p_inf = 101325 Pa; wall_cp = 2 (p/p_inf - 1) / (gamma M^2); wall_mach is the local Mach number of the wall node.",
        "shock_angle_deg: on each horizontal line y = c the x where p crosses (p_inf + p_wall)/2, linearly interpolated along the grid columns, with p_wall the CFD wall plateau of the same grid; a free least-squares line through the points, angle atan(slope).",
        "diag_shock_fit_intercept: y of that line at x = 0, which is 0 if the shock leaves the leading edge: "
        + ", ".join(f"{v:.2e}" for v in icpt) + " on 65x49, 129x97 and 257x193.",
        "Rows named diag_ are not compared quantities and are not in layer0_gci.csv; they say what the compared wall values contain. s/s_inf is (p / rho^gamma) / (p_inf / rho_inf^gamma).",
        "diag_entropy_wall_upstream: the wall upstream of the corner or tip (1 exactly where the flow is undisturbed). diag_entropy_wall_window: the wall in the quantity's window. diag_entropy_row10_window: ten rows off the wall at the same x. diag_entropy_wall_tip_0_0.1: the wall over 0 <= x <= 0.1.",
        "The exact column of diag_entropy_wall_window and diag_entropy_row10_window is the entropy of the exact solution in that region: 1 for the expansion, the oblique-shock value for the wedge (a real shock raises s/s_inf by about 1.03 there, so for the wedge the row 10 value near 1.03 is correct and the wall value is not), the surface value for the cone.",
        "On the coarsest wedge grid row 10 lies inside the shock layer for part of the window (its window_spread is 3 per cent there).",
        "diag_mach_row3_window: Mach number three rows off the wall (exact: the uniform-region value; none for the cone, whose Mach number varies across the layer). diag_corner_node_*: the node at x = 0, the leading edge, the corner or the tip.",
        "Reading: the wall entropy exceeds the exact value on the expansion, the cone and the wedge, while ten rows off the wall it is at the exact value and upstream it is 1. " + _wedge_entropy_text(results) + " This is consistent with entropy generated at the corner or tip (the wedge's leading edge is itself a corner) and convected along the wall; it is an interpretation, not a proof. On the expansion the excess is the same on all three grids and the Mach number three rows off the wall is still below the exact value, so the layer is thicker than three rows or something else is also at work.",
        "difference = cfd - exact; difference_pct = 100 (cfd - exact) / exact, empty where there is no exact value or it is 0.",
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


def write_gci(rows, results: dict, path: pathlib.Path) -> None:
    held = sum(r["gci_bounds_error"] == "True" for r in rows)
    failed = [r for r in rows if r["gci_bounds_error"] == "False"]
    blank = [r for r in rows if r["gci_bounds_error"] == ""]
    lines = _header_common(results) + [
        "Constant-ratio grid convergence (nasa_vv.gci.constant_ratio, r = 2, Fs = 1.25), finest grid first, through the same gate and row writer as gci_existing.csv.",
        "h_basis cells^-1/2 (2-D); n1..n3 are cells (the cells grow exactly 4x per level, so r21 = r32 = 2).",
        "gci_fine_pct and gci_coarse_pct are relative to f1 and f2 (percent).",
        "A level below 3 orders of residual drop is refused (family.MIN_RESIDUAL_DROP_ORDERS) and its quantities carry the refusal in note.",
        "A note flags any computed order above 4 (twice the formal second order) or below 0.5 (a quarter of it): the triplet is then not in the asymptotic range and its GCI is not reliable.",
        "true_error_pct = 100 |f1 - exact| / |exact| on the finest grid; gci_bounds_error is True if gci_fine_pct >= true_error_pct, False otherwise. Both are empty where no GCI was computed or there is no exact value.",
        "Layer 0 knows the exact solution, so it can test whether the Celik GCI bounds the true finest-grid error, which is the job of a GCI. "
        f"Of {held + len(failed)} rows with a computed GCI it held for {held} and failed for {len(failed)} ("
        + ", ".join(f"{r['condition'].split()[0]} {r['quantity']}: GCI {float(r['gci_fine_pct']):.3g} % against error {r['true_error_pct']} %" for r in failed)
        + f"); {len(blank)} row(s) have no GCI ("
        + ", ".join(f"{r['condition'].split()[0]} {r['quantity']}" for r in blank)
        + "). Rows flagged as outside the asymptotic range are included in the count.",
        "Every row's note names what its observed order is consistent with measuring (a discretisation error, a corner or tip transient, a captured shock position, a wall entropy layer), gives the errors against the exact value and, for wall quantities, the order over four other windows. An order that is stable across those windows (within 0.5) is necessary for the GCI to estimate the discretisation error of the quantity, and not sufficient.",
        "The cone is run twice (see above); 'cone' is the layer 0 result and 'cone_sym_axis' is kept to show the effect of the tip node.",
    ]
    foreign = _foreign_lines(path)
    buf = io.StringIO()
    for ln in lines:
        buf.write("# " + ln + "\n")
    w = csv.DictWriter(buf, fieldnames=GCI_COLUMNS + ["true_error_pct", "gci_bounds_error"], lineterminator="\n")
    w.writeheader()
    w.writerows(rows)
    path.write_bytes(_with_foreign_lines(buf.getvalue(), foreign).encode())


FOREIGN_HEADER = "# [plate] "
"""Header lines of layer0_gci.csv that nasa_vv.cases.layer0_plate owns (it inserts them before the column header)."""
FOREIGN_ROW = "layer0,plate M="
"""Rows of layer0_gci.csv that nasa_vv.cases.layer0_plate owns (it appends them)."""


def _foreign_lines(path: pathlib.Path) -> tuple[list[str], list[str]]:
    """(header lines, rows) of an existing layer0_gci.csv that another case owns; empty if there is no file."""
    if not path.exists():
        return [], []
    lines = path.read_bytes().decode().split("\n")
    return ([ln for ln in lines if ln.startswith(FOREIGN_HEADER)], [ln for ln in lines if ln.startswith(FOREIGN_ROW)])


def _with_foreign_lines(text: str, foreign: tuple[list[str], list[str]]) -> str:
    """The text with the foreign header lines in front of the column header and the foreign rows at the end, verbatim.

    This case rewrites the whole file; the lines it does not own would otherwise be lost (rerunning
    ``--collect-only`` after layer0_plate had appended its rows erased them).
    """
    head, rows = foreign
    if not head and not rows:
        return text
    lines = text.split("\n")
    at = next(k for k, ln in enumerate(lines) if ln.startswith("study,condition,"))
    return "\n".join(lines[:at] + head + lines[at:-1] + rows + [""])


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
                print(name, level_name(lv), r["hist"], [(m.q, round(m.cfd, 6)) for m in r["meas"]])
        return
    a.out.mkdir(parents=True, exist_ok=True)
    write_layer0(build_rows(results), results, a.out / "layer0.csv")
    write_gci(gci_rows(results), results, a.out / "layer0_gci.csv")
    print(f"wrote {a.out / 'layer0.csv'} and {a.out / 'layer0_gci.csv'}")


if __name__ == "__main__":
    main()
