# SPDX-License-Identifier: GPL-3.0-or-later
"""Layer 0, fourth case: the laminar flat plate at M = 0.2 against the Blasius solution.

    plate   Re = 1e5 per metre, plate 0 <= x <= 4      blasius_cf(Re_x), blasius_profile      Cf(0.5), u/U(eta) at x = 0.5

IgnisYeet has no laminar mode (``FlowModel`` is Euler or Rans). This case therefore runs
SOLVER= NAVIER_STOKES with the flow numerics and the gas model of a configuration ``ignisyeet aero`` wrote for
RANS (tests/data/golden_rans.cfg), copied verbatim by nasa_vv.su2cfg.layer0_config, with every key of the
turbulence model left out. It verifies the product's RANS discretisation with the turbulence model removed. It is
not a configuration the product itself writes.

Run everything with

    cd research && uv run --project nasa_vv python -m nasa_vv.cases.layer0_plate

SU2 output goes under ~/.cache/nasa_vv_task11/plate (never /tmp, never the repository). With ``--collect-only``
the SU2 runs are skipped and layer0_plate.csv and the plate rows of layer0_gci.csv are rebuilt from the output
already there, byte for byte the same as the first time. The plate rows are appended to layer0_gci.csv
(``merge_gci``): the bytes of every other row stay as they are, and a rerun replaces the plate rows, never
duplicating them.

Geometry
--------
Body-fitted structured grid on -1 <= x <= 4, 0 <= y <= 3 (``MAIN``). The lower boundary is MARKER_SYM for x < 0 and
the no-slip adiabatic plate for x >= 0, which runs to the outlet; inlet, outlet and top are MARKER_FAR (the flow is
subsonic). Along 0 <= x <= 1 the cells are uniform (48 on the base grid, so x = 0 and the sampling station x = 0.5
are nodes of every level); upstream and beyond x = 1 they grow exponentially towards the inlet and the outlet. y is
clustered at the wall by y = H (exp(b s) - 1) / (exp(b) - 1). Every one of these functions is applied to the uniformly
refined parameter, so every coarse node is, to the bit, a fine node and the refinement ratio is exactly 2.

The extents matter. The first design, the plate ending at the outlet at x = 1 in a box 0.25 upstream and 1 high,
accelerated the outer flow by about 1 per cent at x = 0.5 and put a favourable pressure gradient on the plate: Blasius
has none. The semi-infinite plate induces no streamwise velocity at first order, a finite plate and a nearby far-field
boundary do. ``VARIANTS`` are other extents run on the medium grid only, so that the domain's share of the difference
from Blasius is on record; the GCI is of ``MAIN``.

Leading edge
------------
The node at x = 0 belongs to the symmetry plane and to the no-slip wall. SU2 gives it the no-slip condition (u = v = 0,
Mach 0): the plate begins in a stagnation node, which is also where Blasius' boundary-layer solution is singular.
Its values are recorded per run (``diag_le_*`` rows). The sampling station is x = 0.5, where the local Reynolds number
is 5e4 and the leading-edge disturbance has decayed; ``diag_cf_resqrt_x*`` shows Cf sqrt(Re_x) against x.
A single-marker remedy as for the cone tip (axis and cone as one Euler wall) does not apply: a no-slip wall and a
symmetry plane cannot be one marker.

Compressibility
---------------
Blasius is incompressible. At M = 0.2 the adiabatic wall is warmer than the free stream by r (gamma - 1) M^2 / 2 of
T_inf with r = sqrt(Pr). ``eckert_estimate`` gives Eckert's reference-temperature estimate of the change of
Cf sqrt(Re_x) from 0.664 and a Howarth-Dorodnitsyn estimate of the change of the profile. They are estimates, stated in
the CSV header and in the notes; the CFD value is never corrected by them.
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

import numpy as np

from .. import exact
from ..family import GridFamily, GridLevel
from ..gci import constant_ratio
from ..mesh import body_fitted_grid, piecewise_uniform, refine, structured_quad_su2
from ..su2cfg import (GOLDEN_RANS, layer0_config, rans_gas_model, rans_numerics, rans_wall_heat_flux)
from .existing_data import COLUMNS as GCI_COLUMNS
from .existing_data import FamilyData, _flatplate_compute, _rows
from .layer0 import LAYER0_COLUMNS, SU2_CFD, bounds_columns, read_history

RESULTS = pathlib.Path(__file__).resolve().parents[1] / "results"
DEFAULT_WORK = pathlib.Path.home() / ".cache" / "nasa_vv_task11" / "layer0_plate"

MACH = 0.2
RE_UNIT = 1.0e5                 # Reynolds number per metre
T_INF = 288.15
PR_LAM = 0.72                   # SU2's PRANDTL_LAM default: the golden sets none

BASE_CELLS_DOWN = 48            # cells of the base grid along 0 <= x <= 1 (every station is a multiple of 1/48)
BASE_CELLS_Y = 48               # cells of the base grid across


def match_stretch(length: float, cells: int) -> float:
    """c of the exponential cell growth over `length` in `cells` cells whose first cell is as wide as the plate cells, 1/BASE_CELLS_DOWN.

    Solves c / (exp(c) - 1) = cells / (BASE_CELLS_DOWN length) by bisection; 0 if the cells are uniform already.
    """
    target = cells / (BASE_CELLS_DOWN * length)
    if target >= 1.0:
        return 0.0
    lo, hi = 1e-9, 60.0
    for _ in range(200):
        mid = 0.5 * (lo + hi)
        lo, hi = (mid, hi) if mid / math.expm1(mid) > target else (lo, mid)
    return 0.5 * (lo + hi)


@dataclass(frozen=True)
class Domain:
    """The extent of the computational domain and how its cells are distributed (cell counts are those of the base grid)."""
    name: str
    x_in: float          # the inlet is at x = -x_in
    x_out: float         # the outlet is at x = x_out (x_out = 1: the plate ends at the outlet)
    y_top: float
    stretch: float       # b of the wall clustering
    cells_up: int        # cells between the inlet and the leading edge
    cells_tail: int      # cells between x = 1 and the outlet (0 if x_out = 1)

    @property
    def cells_x(self) -> int:
        return self.cells_up + BASE_CELLS_DOWN + self.cells_tail

    @property
    def c_in(self) -> float:
        """Exponential growth of the cells towards the inlet; the cell at the leading edge is as wide as the plate cells."""
        return match_stretch(self.x_in, self.cells_up)

    @property
    def c_out(self) -> float:
        return match_stretch(self.x_out - 1.0, self.cells_tail) if self.cells_tail else 0.0


MAIN = Domain("plate", x_in=1.0, x_out=4.0, y_top=3.0, stretch=7.6, cells_up=16, cells_tail=20)
"""-1 <= x <= 4, 0 <= y <= 3; the first cell of the 65x49 base grid is about 0.26 mm high."""
VARIANTS = (
    Domain("plate_short", x_in=0.25, x_out=1.0, y_top=1.0, stretch=6.3, cells_up=16, cells_tail=0),
    Domain("plate_top1", x_in=1.0, x_out=4.0, y_top=1.0, stretch=6.3, cells_up=16, cells_tail=20),
    Domain("plate_in2", x_in=2.0, x_out=4.0, y_top=3.0, stretch=7.6, cells_up=16, cells_tail=20),
)
"""Other domains, run on the medium grid only, to show how much of the difference from Blasius is the domain: the first
design (a plate that ends at the outlet, 0.25 upstream, 1 high), the same domain with the top at 1, and the inlet at 2."""
VARIANT_LEVEL = 1
LEVELS = (0, 1, 2)              # refinements of the base grid: coarse, medium, fine
ITERATIONS = 40000              # per-case budget; SU2 stops earlier at the residual target
TARGET = -14.0                  # CONV_RESIDUAL_MINVAL of the golden configuration
STATION = 0.5                   # sampling station
ALT_STATIONS = (0.25, 0.375, 0.625, 0.75)
"""Other stations of the sensitivity check; every one is a node of every level (multiples of 1/48)."""
ETA_CMP = 8.0                   # the profile is compared and integrated on 0 <= eta <= ETA_CMP
ETA99 = 4.9096                  # u/U = 0.99 on the Blasius profile
TOP_TOLERANCE = 2.0e-3
"""Largest |p/p_inf - 1| and |u/U - 1| along the top boundary that still counts as 'the top is far away'."""

OUTPUT_LINE = "OUTPUT_FILES= ( RESTART, SURFACE_CSV )"
OUTPUT_LINE_FILES = "OUTPUT_FILES= ( RESTART_ASCII, SURFACE_CSV, SURFACE_PARAVIEW_ASCII )"
CONDITION = f"plate M={MACH:g}"
PLATE_HEADER = "# [plate] "
"""Marks the header lines this module inserts into layer0_gci.csv, so a rerun can replace them."""
PLATE_ROW = "layer0,plate M="
"""Start of every plate row of layer0_gci.csv."""


# ------------------------------------------------------------------ the free stream


@dataclass(frozen=True)
class Freestream:
    gamma: float
    gas_constant: float
    t: float
    p: float
    rho: float
    mu: float
    a: float
    u: float

    @property
    def nu(self) -> float:
        return self.mu / self.rho


def sutherland(t: float, gas: dict) -> float:
    """Dynamic viscosity of the golden's Sutherland law."""
    return gas["mu_ref"] * (t / gas["t_ref"]) ** 1.5 * (gas["t_ref"] + gas["sutherland"]) / (t + gas["sutherland"])


def freestream(golden: str | None = None) -> Freestream:
    """The free stream with Re = RE_UNIT per metre at MACH, from the golden's gas constants and viscosity law.

    The pressure is the only unknown: rho = Re mu / U, p = rho R T, rounded as layer0_config prints it.
    """
    gas = rans_gas_model(GOLDEN_RANS.read_text() if golden is None else golden)
    g, r = gas["gamma"], gas["gas_constant"]
    mu = sutherland(T_INF, gas)
    a = math.sqrt(g * r * T_INF)
    u = MACH * a
    p = round(RE_UNIT * mu / u * r * T_INF, 4)
    rho = p / (r * T_INF)
    return Freestream(g, r, T_INF, p, rho, mu, a, u)


# ------------------------------------------------------------------ compressibility


def _trapz(f: np.ndarray, x: np.ndarray) -> float:
    return float(np.sum(0.5 * (f[1:] + f[:-1]) * np.diff(x)))


def eckert_estimate(fs: Freestream, gas: dict, pr: float = PR_LAM) -> dict:
    """What the compressibility of the adiabatic plate at MACH changes with respect to Blasius (estimates).

    cf_factor: Eckert's reference temperature T* = T_e + 0.72 (T_aw - T_e), T_aw = T_e (1 + sqrt(Pr) (gamma - 1) M^2 / 2),
    and Cf sqrt(Re_x) = 0.664 sqrt(rho* mu* / (rho_e mu_e)) with perfect-gas rho* / rho_e = T_e / T* and the golden's
    Sutherland law for mu*. All with free-stream Cf and Re_x.
    delta_star_factor and profile_shift: Howarth-Dorodnitsyn with Chapman-Rubesin C = 1 and the Crocco-Busemann
    temperature T / T_e = 1 + (gamma - 1) M^2 / 2 (1 - f'^2) (recovery factor 1, which overestimates the heating
    for Pr = 0.72): y = int (T / T_e) d eta_incompressible, so the profile u/U(eta) is the Blasius profile at a smaller
    eta_incompressible. profile_shift is the largest change of u/U at a fixed eta.
    """
    g, m = fs.gamma, MACH
    t_aw = fs.t * (1.0 + math.sqrt(pr) * 0.5 * (g - 1.0) * m * m)
    t_star = fs.t + 0.72 * (t_aw - fs.t)
    c_star = (fs.t / t_star) * sutherland(t_star, gas) / sutherland(fs.t, gas)
    prof = exact.blasius_profile(n=2000, eta_max=10.0)
    eta = np.array([e for e, _ in prof])
    fp = np.array([v for _, v in prof])
    k = 0.5 * (g - 1.0) * m * m
    theta = 1.0 + k * (1.0 - fp * fp)                     # T / T_e on the incompressible coordinate
    y = np.concatenate([[0.0], np.cumsum(0.5 * (theta[1:] + theta[:-1]) * np.diff(eta))])   # physical eta
    ds_inc = _trapz(1.0 - fp, eta)
    ds_comp = _trapz((1.0 - fp) * theta, eta)
    # u/U at the physical eta: interpolate f' (as a function of the incompressible eta) back onto y
    sel = y <= ETA_CMP
    u_phys = fp[sel]                       # u/U at physical eta = y[sel] is f' at the incompressible eta[sel]
    u_inc = np.interp(y[sel], eta, fp)     # the Blasius value at the same physical eta
    shift = float(np.max(np.abs(u_phys - u_inc)))
    return {"t_aw_over_t": t_aw / fs.t, "t_star_over_t": t_star / fs.t, "c_star": c_star,
            "cf_factor": math.sqrt(c_star), "delta_star_factor": float(ds_comp / ds_inc), "profile_shift": shift}


# ------------------------------------------------------------------ grids and configurations


def stretch(s: np.ndarray, b: float | None = None) -> np.ndarray:
    """The fixed wall clustering of the parameter s in [0, 1]: (exp(b s) - 1) / (exp(b) - 1), b = MAIN.stretch."""
    b = MAIN.stretch if b is None else b
    return np.expm1(b * np.asarray(s, dtype=float)) / math.expm1(b)


def level_name(level: int, d: Domain = MAIN) -> str:
    ni = d.cells_x * 2 ** level + 1
    nj = BASE_CELLS_Y * 2 ** level + 1
    return f"{ni}x{nj}"


def level_dir(work: pathlib.Path, level: int, d: Domain = MAIN) -> pathlib.Path:
    return work / d.name / level_name(level, d)


def le_index(level: int, d: Domain = MAIN) -> int:
    """Index along the lower boundary of the leading-edge node (x = 0)."""
    return d.cells_up * 2 ** level


def station_index(level: int, x: float, d: Domain = MAIN) -> int:
    """Index along the lower boundary of the node at x >= 0; x must be a multiple of 1/48 (a node of every level)."""
    j = x * BASE_CELLS_DOWN
    if abs(j - round(j)) > 1e-9:
        raise ValueError(f"x = {x} is not a node of the base grid (multiples of 1/{BASE_CELLS_DOWN})")
    return le_index(level, d) + round(j) * 2 ** level


def _expo(t: np.ndarray, c: float) -> np.ndarray:
    """(exp(c t) - 1) / (exp(c) - 1) on [0, 1]; the identity for c = 0."""
    t = np.asarray(t, dtype=float)
    return t if c == 0.0 else np.expm1(c * t) / math.expm1(c)


def x_nodes(xp: np.ndarray, d: Domain = MAIN) -> np.ndarray:
    """x of the x parameter xp in [0, 3]: [0, 1] upstream of the leading edge, [1, 2] is 0 <= x <= 1 (uniform), [2, 3] beyond x = 1.

    A fixed function of the parameter, applied after the refinement, so that the grids nest.
    """
    xp = np.asarray(xp, dtype=float)
    up = -d.x_in * _expo(1.0 - np.clip(xp, 0.0, 1.0), d.c_in)
    tail = 1.0 + (d.x_out - 1.0) * _expo(np.clip(xp, 2.0, 3.0) - 2.0, d.c_out)
    return np.where(xp <= 1.0, up, np.where(xp <= 2.0, xp - 1.0, tail))


def grid(level: int, d: Domain = MAIN):
    """(X, Y) node arrays of a level. The parameters are refined first and the clustering applied afterwards, so the grids nest."""
    knots, cells = [0.0, 1.0, 2.0], [d.cells_up, BASE_CELLS_DOWN]
    if d.cells_tail:
        knots.append(3.0)
        cells.append(d.cells_tail)
    xp = piecewise_uniform(knots, cells)
    s = np.linspace(0.0, 1.0, BASE_CELLS_Y + 1)
    for _ in range(level):
        xp, s = refine(xp, s)
    x = x_nodes(xp, d)
    x_range = (-d.x_in, d.x_out)
    return body_fitted_grid((x - x_range[0]) / (x_range[1] - x_range[0]), stretch(s, d.stretch), x_range,
                            lambda v: np.zeros_like(v), d.y_top)


def mesh_markers(ni: int, k: int) -> dict:
    """upstream: the symmetry plane before the leading edge (nodes 0..k); plate: the wall (nodes k..ni-1)."""
    return {"upstream": [("ymin", 0, k)], "plate": [("ymin", k, ni - 1)],
            "inlet": ["xmin"], "outlet": ["xmax"], "top": ["ymax"]}


CONFIG_MARKERS = {"wall": ["plate"], "far": ["inlet", "top", "outlet"], "sym": ["upstream"]}


def config_text(iterations: int = ITERATIONS) -> str:
    """layer0_config for the plate, with the solution written as text and the surface as VTK (skin friction).

    The only change to the generated configuration is the OUTPUT_FILES line, which selects the file formats;
    the RANS numerics are asserted unchanged.
    """
    golden = GOLDEN_RANS.read_text()
    fs = freestream(golden)
    cfg = layer0_config(
        ndim=2, axisymmetric=False, mach=MACH, markers=CONFIG_MARKERS, viscous=True, golden=golden,
        analyze=["inlet", "outlet"], iterations=iterations, pressure=fs.p, temperature=fs.t)
    assert cfg.count(OUTPUT_LINE) == 1, "layer0_config no longer writes the expected OUTPUT_FILES line"
    out = cfg.replace(OUTPUT_LINE, OUTPUT_LINE_FILES)
    assert rans_numerics(out) == rans_numerics(golden)
    assert rans_wall_heat_flux(golden) == 0.0 and "MARKER_HEATFLUX= ( plate, 0.0 )" in out
    changed = [(a, b) for a, b in zip(cfg.splitlines(), out.splitlines()) if a != b]
    assert changed == [(OUTPUT_LINE, OUTPUT_LINE_FILES)]
    return out


def run_level(level: int, work: pathlib.Path, su2: pathlib.Path, dom: Domain = MAIN) -> None:
    d = level_dir(work, level, dom)
    d.mkdir(parents=True, exist_ok=True)
    for stale in ("history.csv", "restart.csv", "restart.dat", "surface_flow.csv", "surface_flow.vtk"):
        (d / stale).unlink(missing_ok=True)
    X, Y = grid(level, dom)
    structured_quad_su2(d / "mesh.su2", X, Y, mesh_markers(X.shape[1], le_index(level, dom)))
    (d / "case.cfg").write_text(config_text())
    with (d / "su2.log").open("w") as log:
        r = subprocess.run([str(su2), "case.cfg"], cwd=d, stdout=log, stderr=subprocess.STDOUT)
    if r.returncode != 0:
        raise RuntimeError(f"SU2_CFD failed for {dom.name} {level_name(level, dom)} (exit {r.returncode}); see {d / 'su2.log'}")


# ------------------------------------------------------------------ reading a run


@dataclass
class Field:
    X: np.ndarray
    Y: np.ndarray
    rho: np.ndarray
    u: np.ndarray
    v: np.ndarray
    p: np.ndarray
    t: np.ndarray


def read_field(level: int, work: pathlib.Path, fs: Freestream, dom: Domain = MAIN) -> Field:
    X, Y = grid(level, dom)
    nj, ni = X.shape
    path = level_dir(work, level, dom) / "restart.csv"
    with path.open() as f:
        head = [h.strip().strip('"') for h in f.readline().split(",")]
    raw = np.loadtxt(path, delimiter=",", skiprows=1)
    col = {h: raw[:, i] for i, h in enumerate(head)}
    for need in ("PointID", "x", "y", "Density", "Momentum_x", "Momentum_y", "Energy"):
        if need not in col:
            raise RuntimeError(f"{path}: column {need} is missing; the columns are {head}")
    order = np.argsort(col["PointID"])
    col = {h: v[order] for h, v in col.items()}
    if not np.array_equal(col["PointID"], np.arange(ni * nj)):
        raise RuntimeError(f"{path} does not hold every PointID once")
    rx, ry = col["x"].reshape(nj, ni), col["y"].reshape(nj, ni)
    if np.max(np.abs(rx - X)) > 1e-9 or np.max(np.abs(ry - Y)) > 1e-9:
        raise RuntimeError("the restart file's node order or coordinates do not match the mesh")
    rho = col["Density"].reshape(nj, ni)
    u, v = col["Momentum_x"].reshape(nj, ni) / rho, col["Momentum_y"].reshape(nj, ni) / rho
    p = (fs.gamma - 1.0) * (col["Energy"].reshape(nj, ni) - 0.5 * rho * (u * u + v * v))
    return Field(X, Y, rho, u, v, p, p / (rho * fs.gas_constant))


def read_surface_cf(level: int, work: pathlib.Path, dom: Domain = MAIN) -> tuple[np.ndarray, np.ndarray]:
    """(x, Cf) of the plate nodes from surface_flow.vtk (x component of Skin_Friction_Coefficient), sorted by x."""
    tok = (level_dir(work, level, dom) / "surface_flow.vtk").read_text().split()
    i = tok.index("POINTS")
    n = int(tok[i + 1])
    vals = [float(t) for t in tok[i + 3:i + 3 + 3 * n]]
    x = np.array(vals[0::3])
    k = tok.index("Skin_Friction_Coefficient")
    # VECTORS Skin_Friction_Coefficient double, then 3 n numbers
    start = k + 2
    cf = np.array([float(t) for t in tok[start:start + 3 * n]][0::3])
    o = np.argsort(x)
    return x[o], cf[o]


def su2_log_numbers(path: pathlib.Path) -> dict:
    """Reynolds number and laminar Prandtl number SU2 reports in its log."""
    out = {}
    for ln in path.read_text().splitlines():
        cells = [c.strip() for c in ln.split("|")]
        if "Reynolds Number" in ln and len(cells) >= 6:
            out["reynolds"] = float(cells[5])
        if "Prandtl (Lam.)" in ln and len(cells) >= 6:
            out["prandtl_lam"] = float(cells[5])
    for need in ("reynolds", "prandtl_lam"):
        if need not in out:
            raise RuntimeError(f"{path}: no {need} in the SU2 log")
    return out


# ------------------------------------------------------------------ measuring


@dataclass
class Meas:
    """One row of layer0_plate.csv: a compared quantity, or (diag) a number that says what a compared one contains."""
    q: str
    cfd: float
    exact: float | None
    window: str
    spread: float | None = None
    diag: bool = False


_REF: dict = {}


def blasius_reference() -> dict:
    """The Blasius profile on a fine grid and its integrals on 0 <= eta <= ETA_CMP (the same truncation as the CFD data)."""
    if not _REF:
        prof = exact.blasius_profile(n=8000, eta_max=10.0)
        eta = np.array([e for e, _ in prof])
        fp = np.array([v for _, v in prof])
        sel = eta <= ETA_CMP + 1e-12
        e, f = eta[sel], fp[sel]
        _REF.update(eta=eta, fp=fp, delta_star=_trapz(1.0 - f, e), theta=_trapz(f * (1.0 - f), e))
    return _REF


def _clip(eta: np.ndarray, f: np.ndarray, eta_max: float) -> tuple[np.ndarray, np.ndarray]:
    """Nodes with eta <= eta_max, plus the linearly interpolated end point at eta_max."""
    n = int(np.searchsorted(eta, eta_max, side="right"))
    if n >= len(eta):
        raise RuntimeError(f"the column ends at eta = {eta[-1]:.3g}, below the comparison limit {eta_max}")
    e, v = eta[:n], f[:n]
    if e[-1] < eta_max - 1e-12:
        w = (eta_max - eta[n - 1]) / (eta[n] - eta[n - 1])
        e = np.append(e, eta_max)
        v = np.append(v, f[n - 1] + w * (f[n] - f[n - 1]))
    return e, v


def profile_stats(eta: np.ndarray, uu: np.ndarray) -> dict:
    """Comparison of a column u/U(eta) with the Blasius profile on 0 <= eta <= ETA_CMP.

    delta_star = int (1 - u/U) d eta, theta = int u/U (1 - u/U) d eta (trapezoid rule on the nodes, the last interval
    clipped at ETA_CMP), max_dev = max |u/U - f'| over the nodes, rms_dev = sqrt(int (u/U - f')^2 d eta / ETA_CMP).
    """
    ref = blasius_reference()
    dev = uu - np.interp(eta, ref["eta"], ref["fp"])
    e, f = _clip(eta, uu, ETA_CMP)
    _, d = _clip(eta, dev, ETA_CMP)
    return {"delta_star": _trapz(1.0 - f, e), "theta": _trapz(f * (1.0 - f), e),
            "max_dev": float(np.max(np.abs(dev[eta <= ETA_CMP]))), "rms_dev": math.sqrt(_trapz(d * d, e) / ETA_CMP),
            "u_edge": float(f[-1])}


def edge_scaled_stats(eta: np.ndarray, uu: np.ndarray) -> dict:
    """profile_stats of the column scaled by its own edge velocity.

    u_e is u/U at eta = ETA_CMP. Blasius' similarity variable uses the velocity outside the layer, so the column is
    compared as u/u_e against f'(eta sqrt(u_e)): eta_e = y sqrt(u_e U / (nu x)). Where the outer flow is the free
    stream (u_e = 1) this is profile_stats. The cut stays at the same y.
    """
    _, f = _clip(eta, uu, ETA_CMP)
    u_e = float(f[-1])
    out = profile_stats(eta * math.sqrt(u_e), uu / u_e)
    out["u_e"] = u_e
    return out


def column(f: Field, fs: Freestream, i: int) -> tuple[np.ndarray, np.ndarray]:
    """(eta, u/U) of the grid column i, eta = y sqrt(U / (nu x)) = y sqrt(Re_unit / x)."""
    x = f.X[0, i]
    return f.Y[:, i] * math.sqrt(RE_UNIT / x), f.u[:, i] / fs.u


def _station_text(f: Field, level: int, x: float, dom: Domain = MAIN) -> str:
    i = station_index(level, x, dom)
    return f"x = {x:g} (node {i} of {f.X.shape[1]} along the plate)"


def measure(level: int, work: pathlib.Path, dom: Domain = MAIN) -> dict:
    fs = freestream()
    fld = read_field(level, work, fs, dom)
    xs, cfs = read_surface_cf(level, work, dom)
    X = fld.X[0]
    nj, ni = fld.X.shape

    def cf_at(x: float) -> float:
        k = int(np.argmin(np.abs(xs - x)))
        if abs(xs[k] - x) > 1e-6:
            raise RuntimeError(f"no surface node at x = {x}")
        return float(cfs[k])

    meas: list[Meas] = []
    i5 = station_index(level, STATION, dom)
    if abs(X[i5] - STATION) > 1e-12:
        raise RuntimeError(f"node {i5} is at x = {X[i5]}, not {STATION}")
    ref = blasius_reference()
    st, st_e = {}, {}
    for x in (STATION,) + ALT_STATIONS:
        eta, uu = column(fld, fs, station_index(level, x, dom))
        st[x] = profile_stats(eta, uu)
        st_e[x] = edge_scaled_stats(eta, uu)
    win_cf = f"plate node {_station_text(fld, level, STATION, dom)[4:]}; SU2 Skin_Friction_Coefficient (x component, free-stream dynamic pressure)"
    win_prof = (f"column at x = {STATION:g}, eta = y sqrt(Re_unit / x) over 0 <= eta <= {ETA_CMP:g} "
                f"({int(np.sum(column(fld, fs, i5)[0] <= ETA_CMP))} nodes; trapezoid rule, last interval clipped)")
    cf5 = cf_at(STATION)
    meas.append(Meas(f"cf_x{STATION:g}", cf5, exact.blasius_cf(RE_UNIT * STATION), win_cf))
    meas.append(Meas("delta_star_eta", st[STATION]["delta_star"], ref["delta_star"], win_prof))
    meas.append(Meas("theta_eta", st[STATION]["theta"], ref["theta"], win_prof))
    meas.append(Meas("diag_profile_max_dev", st[STATION]["max_dev"], 0.0, win_prof + "; max |u/U - f'| over the nodes", None, True))
    meas.append(Meas("diag_profile_rms_dev", st[STATION]["rms_dev"], 0.0, win_prof + "; sqrt(int (u/U - f')^2 d eta / eta_max)", None, True))
    meas.append(Meas("diag_edge_u_ratio", st[STATION]["u_edge"], 1.0, f"u/U at eta = {ETA_CMP:g}, x = {STATION:g}", None, True))
    win_edge = win_prof + "; scaled by the edge velocity u_e = u/U at eta = 8: u/u_e against f'(eta sqrt(u_e))"
    meas.append(Meas("delta_star_eta_ue", st_e[STATION]["delta_star"], ref["delta_star"], win_edge))
    meas.append(Meas("theta_eta_ue", st_e[STATION]["theta"], ref["theta"], win_edge))
    meas.append(Meas("diag_profile_max_dev_ue", st_e[STATION]["max_dev"], 0.0, win_edge + "; max |u/u_e - f'| over the nodes", None, True))
    meas.append(Meas("diag_profile_rms_dev_ue", st_e[STATION]["rms_dev"], 0.0, win_edge + "; rms as above", None, True))

    # Cf sqrt(Re_x) against x: the trend towards 0.664
    for x in (0.25, 0.5, 0.75, 1.0):
        meas.append(Meas(f"diag_cf_resqrt_x{x:g}", cf_at(x) * math.sqrt(RE_UNIT * x), 0.664,
                         f"plate node x = {x:g}: Cf sqrt(Re_x), exact 0.664", None, True))

    # Blasius assumes zero pressure gradient and a uniform outer flow
    p_inf = fs.p
    cp = lambda p: (p / p_inf - 1.0) / (0.5 * fs.gamma * MACH ** 2)
    meas.append(Meas("diag_wall_cp_x0.5", float(cp(fld.p[0, i5])), 0.0, f"wall node x = {STATION:g}", None, True))
    ip, im = i5 + 2 ** level, i5 - 2 ** level
    dcpdx = float((cp(fld.p[0, ip]) - cp(fld.p[0, im])) / (X[ip] - X[im]))
    meas.append(Meas("diag_wall_dcp_dx_x0.5", dcpdx, 0.0, f"central difference over x = {X[im]:.4f}..{X[ip]:.4f}", None, True))
    eta5, _ = column(fld, fs, i5)
    inb = eta5 <= ETA_CMP
    meas.append(Meas("diag_column_p_dev", float(np.max(np.abs(fld.p[inb, i5] / p_inf - 1.0))), 0.0,
                     f"max |p/p_inf - 1| over the column at x = {STATION:g}, eta <= {ETA_CMP:g}", None, True))

    # resolution at the station
    n99 = int(np.sum(eta5 <= ETA99))
    meas.append(Meas("diag_points_in_bl99", float(n99), None, f"nodes with eta <= {ETA99} (u/U = 0.99) at x = {STATION:g}", None, True))
    tau_w = cf5 * 0.5 * fs.rho * fs.u ** 2
    rho_w, t_w = fld.rho[0, i5], fld.t[0, i5]
    mu_w = sutherland(t_w, rans_gas_model(GOLDEN_RANS.read_text()))
    y1 = fld.Y[1, i5] - fld.Y[0, i5]
    yplus = y1 * rho_w * math.sqrt(tau_w / rho_w) / mu_w
    meas.append(Meas("diag_first_node_y_m", float(y1), None, f"height of the first node above the wall at x = {STATION:g}", None, True))
    meas.append(Meas("diag_first_node_yplus", float(yplus), None,
                     f"y1 rho_w u_tau / mu_w at x = {STATION:g} with the wall shear of the Cf above", None, True))

    # the leading-edge node (x = 0, symmetry plane and wall) and its neighbours
    k = le_index(level, dom)
    speed = lambda j, i: float(math.hypot(fld.u[j, i], fld.v[j, i]) / fs.u)
    meas.append(Meas("diag_le_node_speed_over_U", speed(0, k), None, "node x = 0, y = 0 (belongs to MARKER_SYM and the wall)", None, True))
    meas.append(Meas("diag_le_node_cp", float(cp(fld.p[0, k])), None, "node x = 0, y = 0: (p/p_inf - 1) / (gamma M^2 / 2)", None, True))
    meas.append(Meas("diag_le_node_t_ratio", float(fld.t[0, k] / fs.t), None, "node x = 0, y = 0: T / T_inf", None, True))
    meas.append(Meas("diag_le_next_wall_node_cp", float(cp(fld.p[0, k + 1])), None, "first wall node after the leading edge", None, True))
    meas.append(Meas("diag_le_node_above_u_over_U", float(fld.u[1, k] / fs.u), None, "first node above the leading-edge node", None, True))
    meas.append(Meas("diag_le_upstream_node_u_over_U", float(fld.u[0, k - 1] / fs.u), None, "last symmetry node before the leading edge", None, True))
    meas.append(Meas("diag_le_upstream_node_cp", float(cp(fld.p[0, k - 1])), None, "last symmetry node before the leading edge", None, True))
    meas.append(Meas("diag_wall_u_max", float(np.max(np.abs(fld.u[0, k:]))), 0.0, "max |u| over the plate nodes (no slip)", None, True))

    top = fld.p[-1] / p_inf - 1.0
    top_u = fld.u[-1] / fs.u - 1.0
    alt = {}
    for q, getter in ((f"cf_x{STATION:g}", lambda x: cf_at(x)),
                      ("delta_star_eta", lambda x: st[x]["delta_star"]),
                      ("theta_eta", lambda x: st[x]["theta"]),
                      ("delta_star_eta_ue", lambda x: st_e[x]["delta_star"]),
                      ("theta_eta_ue", lambda x: st_e[x]["theta"])):
        alt[q] = {x: getter(x) for x in (STATION,) + ALT_STATIONS}
    return {"meas": meas, "alt": alt, "nodes": int(fld.X.size), "cells": (ni - 1) * (nj - 1),
            "top_dev_p": float(np.max(np.abs(top))), "top_dev_u": float(np.max(np.abs(top_u))),
            "cf_alt": {x: cf_at(x) for x in (STATION,) + ALT_STATIONS}}


def collect_level(level: int, work: pathlib.Path, dom: Domain = MAIN, refuse_near_top: bool = True) -> dict:
    """Everything one run gives. A run whose top boundary is too disturbed is refused unless refuse_near_top is False
    (the other domains of VARIANTS, which exist to show what a nearby top does)."""
    d = level_dir(work, level, dom)
    hist = read_history(d / "history.csv")
    logn = su2_log_numbers(d / "su2.log")
    if abs(logn["reynolds"] - RE_UNIT) > 1e-6 * RE_UNIT:
        raise RuntimeError(f"SU2 reports Re = {logn['reynolds']}, not {RE_UNIT}")
    if logn["prandtl_lam"] != PR_LAM:
        raise RuntimeError(f"SU2 reports a laminar Prandtl number of {logn['prandtl_lam']}, not {PR_LAM}")
    m = measure(level, work, dom)
    if refuse_near_top and not (m["top_dev_p"] < TOP_TOLERANCE and m["top_dev_u"] < TOP_TOLERANCE):
        raise RuntimeError(
            f"{dom.name} {level_name(level, dom)}: the top boundary deviates by {m['top_dev_p']:.3g} in p/p_inf and {m['top_dev_u']:.3g} "
            "in u/U; it is too close to the plate")
    m["hist"] = hist
    m["log"] = logn
    return m


# ------------------------------------------------------------------ the GCI rows


NO_LAMINAR = ("IgnisYeet has no laminar mode (FlowModel is Euler or Rans): this case runs SOLVER= NAVIER_STOKES with the "
              "flow numerics and gas model of golden_rans.cfg and without the turbulence model, so it verifies the product's "
              "RANS discretisation with the turbulence model removed. It is not a configuration the product itself writes")

STABLE_RANGE = 0.5
"""The observed order is called stable across stations if the orders of the stations lie within this range."""


def station_orders(alt: dict) -> list:
    """Observed order (or None) at the stations ALT_STATIONS; alt is {level: {station: value}}."""
    return [constant_ratio(alt[2][x], alt[1][x], alt[0][x], 2.0).order for x in ALT_STATIONS]


def station_label(orders: list) -> tuple[str, str]:
    """('stable' | 'not stable', 'lo to hi' or 'undefined') for the orders of the other stations."""
    have = [o for o in orders if o is not None]
    if len(have) < len(orders):
        return "not stable", "undefined for " + str(len(orders) - len(have)) + " station(s)"
    span = f"{min(have):.2f} to {max(have):.2f}"
    return ("stable" if max(have) - min(have) <= STABLE_RANGE else "not stable"), span


MEANING = {
    f"cf_x{STATION:g}": "the skin friction coefficient of the plate node at x = 0.5 against Blasius' 0.664 / sqrt(Re_x)",
    "delta_star_eta": "the displacement thickness of the profile at x = 0.5 in similarity coordinates against Blasius' 1.7208",
    "theta_eta": "the momentum thickness of the profile at x = 0.5 in similarity coordinates against Blasius' 0.6641",
    "delta_star_eta_ue": "delta_star_eta of the profile scaled by its own edge velocity u_e = u/U at eta = 8 (u/u_e against f'(eta sqrt(u_e))), which removes the part of the difference that is the outer flow's departure from the free stream",
    "theta_eta_ue": "theta_eta of the profile scaled by its own edge velocity u_e = u/U at eta = 8, see delta_star_eta_ue",
}


def gci_note(q: str, fine_first: list[float], exact_value: float, alt: dict | None, est: dict, other: dict | None = None,
             edge: list[float] | None = None) -> str:
    """What the observed order of one plate quantity measures, with the numbers that support it.

    other maps the name of another domain to its medium-grid value of the quantity (VARIANTS). edge is u/U at
    eta = ETA_CMP on the fine, medium and coarse grid.
    """
    f1, f2, f3 = fine_first
    parts = [NO_LAMINAR, f"exact: Blasius (incompressible, zero pressure gradient, boundary-layer limit); quantity: {MEANING[q]}"]
    ec, em, ef = f3 - exact_value, f2 - exact_value, f1 - exact_value
    parts.append(
        f"errors against Blasius (coarse, medium, fine) {ec:.3g}, {em:.3g}, {ef:.3g} ({100.0 * ec / exact_value:+.3g} %, "
        f"{100.0 * em / exact_value:+.3g} %, {100.0 * ef / exact_value:+.3g} %); error ratios "
        f"{abs(ec / em):.2f} and {abs(em / ef):.2f}; difference ratio {abs((f3 - f2) / (f2 - f1)):.2f}")
    if em * ef < 0.0:
        parts.append("the errors against Blasius change sign between the medium and the fine grid: the finest-grid error is where the "
                     "sequence crosses Blasius, not a converged agreement with it, and its comparison with the GCI says little")
    elif ec * em < 0.0:
        parts.append("the errors against Blasius change sign between the coarse and the medium grid")
    g = constant_ratio(f1, f2, f3, 2.0)
    if g.extrapolated is not None:
        parts.append(f"the extrapolated value {g.extrapolated:.6g} differs from Blasius by {g.extrapolated - exact_value:.3g} "
                     f"({100.0 * (g.extrapolated - exact_value) / exact_value:+.3g} %): what remains once the estimated discretisation "
                     "error is removed, which the GCI cannot account for (the domain, compressibility, the leading edge, and the "
                     "uncertainty of the extrapolation at this order)")
    if q.startswith("cf"):
        parts.append(f"compressibility at M = {MACH:g}, Eckert estimate: Cf changes by {100.0 * (est['cf_factor'] - 1.0):.3f} % "
                     "(estimate; the CFD value is not corrected)")
        if edge:
            scaled = [v / u ** 1.5 for v, u in zip(fine_first, edge)]
            parts.append(
                f"the outer flow at eta = {ETA_CMP:g} is {edge[0]:.4f} U on every grid (Blasius assumes U), and Cf scales as "
                f"(u_e/U)^(3/2) for a boundary layer under an edge velocity u_e, an estimate of {100.0 * (edge[0] ** 1.5 - 1.0):+.2f} % "
                f"on the fine grid: Cf / (u_e/U)^(3/2) is {scaled[0]:.6g}, {scaled[1]:.6g}, {scaled[2]:.6g} on the fine, medium and "
                f"coarse grid, {100.0 * (scaled[0] - exact_value) / exact_value:+.2f} % from Blasius on the fine grid (estimate; "
                "u_e is taken at one height)")
    elif q == "delta_star_eta":
        parts.append(f"compressibility, Howarth-Dorodnitsyn estimate with Crocco-Busemann temperature: delta* changes by "
                     f"{100.0 * (est['delta_star_factor'] - 1.0):+.2f} % (estimate; not corrected)")
    if other:
        parts.append("other domains on the medium grid: " + ", ".join(
            f"{n} {v:.6g} ({100.0 * (v - f2) / f2:+.2f} % from the plate domain)" for n, v in other.items()))
    if alt:
        orders = station_orders(alt)
        shown = ", ".join("undefined" if o is None else f"{o:.2f}" for o in orders)
        label, span = station_label(orders)
        parts.append("station sensitivity: observed order at x = " + ", ".join(f"{x:g}" for x in ALT_STATIONS)
                     + f" is {shown} (range {span}): {label} (stable means within {STABLE_RANGE:g}); "
                     "an order below the formal 2 that is the same at every station does not separate the order of the scheme from "
                     "a leading-edge error that the boundary layer carries downstream, since every station inherits it")
    return "; ".join(parts)


def gci_rows(results: dict, est: dict, variants: dict | None = None) -> list[dict[str, str]]:
    """constant_ratio(r = 2) per quantity, through the same gate and row writer as gci_existing.csv and layer0_gci.csv."""
    fine_first = list(reversed(LEVELS))
    levels = []
    for lv in fine_first:
        r = results[lv]
        nx, ny = MAIN.cells_x * 2 ** lv, BASE_CELLS_Y * 2 ** lv
        levels.append(GridLevel(level_name(lv), r["cells"], {"dx": 1.0 / nx, "dy": 1.0 / ny},
                                residual_drop_orders=r["hist"]["drop"]))
    stalled = [level_name(lv) for lv in fine_first if results[lv]["hist"]["stop"] != "residual target"]
    stall_note = "; ".join(f"grid {n} stopped at the iteration limit" for n in stalled)
    trip = "/".join(lv.name for lv in levels)
    out = []
    for m0 in results[LEVELS[0]]["meas"]:
        if m0.diag:
            continue
        q = m0.q
        vals = [next(m.cfd for m in results[lv]["meas"] if m.q == q) for lv in fine_first]
        alt = {lv: results[lv]["alt"][q] for lv in LEVELS}
        other = {n: next(m.cfd for m in r["meas"] if m.q == q) for n, r in (variants or {}).items()}
        edge = [_val(results, lv, "diag_edge_u_ratio") for lv in fine_first]
        note = "; ".join(x for x in (stall_note, gci_note(q, vals, m0.exact, alt, est, other, edge)) if x)
        fd = FamilyData(GridFamily(levels, ndim=2), {q: vals}, row_note=note)
        for row in _rows("layer0", CONDITION, trip, fd, _flatplate_compute, "cells^-1/2 (2-D)"):
            row["true_error_pct"], row["gci_bounds_error"] = bounds_columns(row["gci_fine_pct"], vals[0], m0.exact)
            if row["gci_bounds_error"] == "False":
                row["note"] = (f"the GCI ({float(row['gci_fine_pct']):.3g} %) is smaller than the observed finest-grid "
                               f"error ({row['true_error_pct']} %): it does not bound the error and must not be "
                               "quoted as the uncertainty of this quantity; " + row["note"])
            out.append(row)
    return out


# ------------------------------------------------------------------ the files


def build_rows(results: dict, variants: dict | None = None) -> list[dict[str, str]]:
    """Rows of layer0_plate.csv: the main domain on every level, then the other domains on the medium level."""
    rows = []

    def add(case: str, dom: Domain, level: int, r: dict, only_compared: bool) -> None:
        for m in r["meas"]:
            if only_compared and m.diag and m.q not in VARIANT_DIAG:
                continue
            has_exact = m.exact is not None
            rows.append({
                "case": case, "mach": f"{MACH:g}", "level": level_name(level, dom),
                "nodes": str(r["nodes"]), "quantity": m.q,
                "cfd": format(m.cfd, ".10g"), "exact": format(m.exact, ".10g") if has_exact else "",
                "difference": format(m.cfd - m.exact, ".6g") if has_exact else "",
                "difference_pct": format(100.0 * (m.cfd - m.exact) / m.exact, ".4g") if has_exact and m.exact != 0.0 else "",
                "residual_drop_orders": format(r["hist"]["drop"], ".3f"),
                "iterations": str(r["hist"]["iterations"]),
                "stop_reason": r["hist"]["stop"], "window": m.window,
                "window_spread": "" if m.spread is None else format(m.spread, ".3g")})

    for level in LEVELS:
        add(MAIN.name, MAIN, level, results[level], False)
    for dom in VARIANTS:
        if variants and dom.name in variants:
            add(dom.name, dom, VARIANT_LEVEL, variants[dom.name], True)
    return rows


VARIANT_DIAG = ("diag_edge_u_ratio", "diag_wall_cp_x0.5", "diag_cf_resqrt_x0.25", "diag_cf_resqrt_x0.5", "diag_cf_resqrt_x0.75")
"""Diagnostic rows kept for the other domains."""


def _sha(path: pathlib.Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()[:12]


def _val(results: dict, level: int, q: str) -> float:
    return next(m.cfd for m in results[level]["meas"] if m.q == q)


def _trio(results: dict, q: str, spec: str) -> str:
    return ", ".join(format(_val(results, lv, q), spec) for lv in LEVELS)


def domain_text(results: dict, variants: dict) -> str:
    """The domain study in words: the same quantities on the medium grid for each domain."""
    def one(name: str, r: dict) -> str:
        cf = next(m for m in r["meas"] if m.q == f"cf_x{STATION:g}")
        ds = next(m for m in r["meas"] if m.q == "delta_star_eta")
        ue = next(m.cfd for m in r["meas"] if m.q == "diag_edge_u_ratio")
        cp = next(m.cfd for m in r["meas"] if m.q == "diag_wall_cp_x0.5")
        return (f"{name}: Cf {cf.cfd:.6e} ({100.0 * (cf.cfd - cf.exact) / cf.exact:+.2f} % from Blasius), "
                f"delta* {ds.cfd:.4f} ({100.0 * (ds.cfd - ds.exact) / ds.exact:+.2f} %), u/U at eta = {ETA_CMP:g} {ue:.4f}, wall Cp {cp:+.4f}, "
                f"largest |u/U - 1| on the top {r['top_dev_u']:.1e}")
    parts = [one(f"{MAIN.name} (the domain of the GCI)", results[VARIANT_LEVEL])]
    parts += [one(d.name, variants[d.name]) for d in VARIANTS if d.name in variants]
    return "; ".join(parts)


def domain_reading(results: dict, variants: dict) -> str:
    cf = lambda r: next(m.cfd for m in r["meas"] if m.q == f"cf_x{STATION:g}")
    main = cf(results[VARIANT_LEVEL])
    edge = lambda r: next(m.cfd for m in r["meas"] if m.q == "diag_edge_u_ratio")
    d = {n: 100.0 * (cf(r) - main) / main for n, r in variants.items()}
    return ("Against the plate domain, Cf at x = 0.5 on the medium grid is " + ", ".join(f"{v:+.2f} % in {n}" for n, v in d.items())
            + f". The outer flow at eta = {ETA_CMP:g} is {edge(results[VARIANT_LEVEL]):.4f} U in the plate domain and {edge(variants['plate_short']):.4f} U in the "
            "first design, a plate that ends at the outlet in a short domain, which also carries the favourable pressure gradient "
            "(wall Cp above) that Blasius excludes. The domain is therefore part of every difference from Blasius below, at a few "
            "tenths of a per cent for the extents tried here and not removed by them")


def reading_text(results: dict, est: dict, variants: dict | None) -> str:
    """Reading of the GCI scalars against Blasius, from the numbers."""
    parts = []
    for q, label in ((f"cf_x{STATION:g}", "Cf"), ("delta_star_eta", "delta*"), ("theta_eta", "theta"),
                     ("delta_star_eta_ue", "delta* scaled by u_e"), ("theta_eta_ue", "theta scaled by u_e")):
        m0 = next(m for m in results[LEVELS[0]]["meas"] if m.q == q)
        vals = [_val(results, lv, q) for lv in reversed(LEVELS)]
        errs = [100.0 * (v - m0.exact) / m0.exact for v in reversed(vals)]
        g = constant_ratio(vals[0], vals[1], vals[2], 2.0)
        ex = "" if g.extrapolated is None else f", extrapolated {100.0 * (g.extrapolated - m0.exact) / m0.exact:+.2f} % (observed order {g.order:.2f})"
        parts.append(f"{label} {errs[0]:+.2f} %, {errs[1]:+.2f} %, {errs[2]:+.2f} %{ex}")
    trend = ", ".join(f"{_val(results, LEVELS[-1], f'diag_cf_resqrt_x{x:g}'):.4f}" for x in (0.25, 0.5, 0.75, 1.0))
    return ("Reading: difference from Blasius on the coarse, medium and fine grid (extrapolated: Richardson extrapolation with the "
            "observed order, an estimate of the grid-converged value of the plate domain): " + "; ".join(parts)
            + f". Cf sqrt(Re_x) on the finest grid at x = 0.25, 0.5, 0.75, 1 is {trend} (Blasius 0.664): it rises through 0.664 with x instead of approaching it from one side; what sets that slope (the leading-edge region, the far end of the plate, the grid) is not separated here. A fine-grid value close to Blasius where the sequence crosses it is not agreement; the extrapolated differences are what "
            f"remains of physics, domain and the uncertainty of an extrapolation at an order of about 1, against a compressibility estimate of {100.0 * (est['cf_factor'] - 1.0):+.3f} % for Cf.")


def header_lines(results: dict, est: dict, variants: dict | None = None) -> list[str]:
    fs = freestream()
    logn = results[LEVELS[0]]["log"]
    top_p = max(r["top_dev_p"] for r in results.values())
    top_u = max(r["top_dev_u"] for r in results.values())
    names = ", ".join(level_name(lv) for lv in LEVELS)
    cells = ", ".join(f"{MAIN.cells_x * 2 ** lv}x{BASE_CELLS_Y * 2 ** lv}" for lv in LEVELS)
    cf_bl = exact.blasius_cf(RE_UNIT * STATION)
    cf_fine = _val(results, LEVELS[-1], f"cf_x{STATION:g}")
    lines = [
        "Layer 0, laminar flat plate at M = 0.2 against the Blasius solution: SU2 8.3.0 (ignisyeet-su2-native), Navier-Stokes, no turbulence model.",
        NO_LAMINAR + ".",
        f"Producer: python -m nasa_vv.cases.layer0_plate (research/nasa_vv/cases/layer0_plate.py). Flow numerics and gas model: copied by nasa_vv.su2cfg.layer0_config from tests/data/golden_rans.cfg (sha256 {_sha(GOLDEN_RANS)}), the second-order stage of a cold RANS case written by `ignisyeet aero` (a laminar case needs the keys of its numerics and the gas model, not those of the Spalart-Allmaras model).",
        "Every key that exists only because of the turbulence model is not copied (KIND_TURB_MODEL, SA_OPTIONS, FREESTREAM_NU_FACTOR, CONV_NUM_METHOD_TURB, MUSCL_TURB, TIME_DISCRE_TURB). CONV_FIELD is ( RMS_DENSITY, RMS_NU_TILDE ) in the golden: it is neither copied nor dropped but derived, the turbulence residual removed and RMS_DENSITY left (asserted). Neither file sets PRANDTL_LAM, so SU2's default 0.72 applies, as in the product.",
        "Exact reference: nasa_vv.exact.blasius_cf(Re_x) = 0.664 / sqrt(Re_x) and blasius_profile (f''' + f f'' / 2 = 0, f''(0) = 0.332057). Blasius is the incompressible, zero-pressure-gradient boundary-layer solution of a semi-infinite plate; the CFD solution is compressible Navier-Stokes on a plate with a leading edge, in a finite domain.",
        f"Free stream: M = {MACH:g}, T = {fs.t:g} K, p = {fs.p:.4f} Pa from the golden's Sutherland law and gas constants, so that Re = {RE_UNIT:g} per metre (SU2 reports {logn['reynolds']:g}); U = {fs.u:.4f} m/s. Re_x = {RE_UNIT:g} x, x from the leading edge; Cf uses the free-stream dynamic pressure.",
        f"Grids: body-fitted, structured, exactly nested (every coarse node is a fine node); {names} nodes, i.e. {cells} cells, so the refinement ratio is exactly 2. {-MAIN.x_in:g} <= x <= {MAIN.x_out:g}, 0 <= y <= {MAIN.y_top:g}; the 48 cells per base grid along 0 <= x <= 1 are uniform, the cells upstream and beyond x = 1 grow exponentially (c = {MAIN.c_in:.3f} and {MAIN.c_out:.3f}, chosen so that the first cell is as wide as the plate cells); the leading edge x = 0 and the station x = 0.5 are nodes of every level; y is clustered at the wall by the fixed function y = {MAIN.y_top:g} (exp(b s) - 1) / (exp(b) - 1), b = {MAIN.stretch:g}. Every function is applied to the uniformly refined parameter, which is what keeps the grids nested.",
        f"Resolution at x = 0.5: nodes inside the 99 % layer (eta <= {ETA99}) {_trio(results, 'diag_points_in_bl99', '.0f')}; first node at y = {_trio(results, 'diag_first_node_y_m', '.3g')} m, y+ = {_trio(results, 'diag_first_node_yplus', '.3f')} on {names}.",
        "Boundaries: x < 0 on y = 0 is MARKER_SYM, the plate (x >= 0, to the outlet) is a no-slip adiabatic wall (MARKER_HEATFLUX 0.0, the golden's own wall condition); inlet, outlet and top are MARKER_FAR (subsonic).",
        f"Leading edge: the node at x = 0 belongs to the symmetry plane and the wall; SU2 gives it the no-slip condition: speed/U = {_trio(results, 'diag_le_node_speed_over_U', '.3g')}, Cp = {_trio(results, 'diag_le_node_cp', '+.4f')}, T/T_inf = {_trio(results, 'diag_le_node_t_ratio', '.5f')} (the Cp of the node grows with refinement: a singular value that does not converge); the next wall node has Cp = {_trio(results, 'diag_le_next_wall_node_cp', '+.4f')}, the symmetry node before it u/U = {_trio(results, 'diag_le_upstream_node_u_over_U', '.3f')} (on {names}): the same on every grid, so the disturbed region is one cell wide and shrinks with the cell. The plate begins in a stagnation node with a pressure spike at the junction, and a single-marker remedy is not available (a no-slip wall and a symmetry plane cannot be one marker). The station x = 0.5 has Re_x = 5e4; diag_cf_resqrt_x* shows Cf sqrt(Re_x) against x.",
        f"Compressibility, estimates only, the CFD values are not corrected: adiabatic wall T_aw/T_inf = {est['t_aw_over_t']:.5f} (recovery factor sqrt(0.72)); Eckert reference temperature T*/T_inf = {est['t_star_over_t']:.5f}, Cf sqrt(Re_x) changes by the factor {est['cf_factor']:.5f} ({100.0 * (est['cf_factor'] - 1.0):+.3f} %, i.e. {cf_bl * (est['cf_factor'] - 1.0):+.3e} on Cf = {cf_bl:.5e}); Howarth-Dorodnitsyn with Crocco-Busemann temperature (C = 1, recovery factor 1): delta* x {est['delta_star_factor']:.5f}, largest change of u/U at a fixed eta {est['profile_shift']:.2e}. Observed: Cf at x = 0.5 on the finest grid is {cf_fine:.6e}, {100.0 * (cf_fine - cf_bl) / cf_bl:+.3f} % from Blasius.",
        f"Outer flow: the largest |p/p_inf - 1| along the top boundary over the {len(LEVELS)} runs is {top_p:.1e} and the largest |u/U - 1| is {top_u:.1e}; the collector refuses a run above {TOP_TOLERANCE:g}.",
    ]
    if variants:
        lines.append("Domain study on the medium grid (the same grid family, other extents; case names plate_*): " + domain_text(results, variants)
                     + ". " + domain_reading(results, variants))
    lines.append(reading_text(results, est, variants))
    lines += [
        "The only edit to the layer0_config output is OUTPUT_FILES= ( RESTART_ASCII, SURFACE_CSV, SURFACE_PARAVIEW_ASCII ) in place of RESTART, so the solution can be read as text and the skin friction as VTK; the numerics are asserted identical to the golden's.",
        f"ITER = {ITERATIONS} (a per-case budget, not a numerics key); CONV_RESIDUAL_MINVAL = -14 on rms[Rho] stops a run earlier. Every run records Avg_Massflow(inlet) and Avg_Massflow(outlet); they are not analysed here.",
        f"Profile comparison: eta = y sqrt(Re_unit / x) at the column x = 0.5, 0 <= eta <= {ETA_CMP:g}; delta_star_eta = int (1 - u/U) d eta, theta_eta = int u/U (1 - u/U) d eta, trapezoid rule on the nodes with the last interval clipped at eta = {ETA_CMP:g}; the exact value is the same integral of the Blasius profile with the same truncation (1.7208 and 0.6641 to infinity). diag_profile_max_dev is max |u/U - f'| over the nodes, diag_profile_rms_dev sqrt(int (u/U - f')^2 d eta / {ETA_CMP:g}); their exact value is 0. The outer flow at eta = {ETA_CMP:g} is not the free stream (diag_edge_u_ratio, {_trio(results, 'diag_edge_u_ratio', '.4f')} on {names}, the same on every grid), and the integrals of u/U over the layer carry that excess. The *_ue rows scale the column by its own edge velocity u_e = u/U at eta = 8 (u/u_e against f'(eta sqrt(u_e)), the similarity solution under an edge velocity u_e) and so remove that part: on the finest grid delta_star_eta goes from {_val(results, LEVELS[-1], 'delta_star_eta'):.4f} to {_val(results, LEVELS[-1], 'delta_star_eta_ue'):.4f} and theta_eta from {_val(results, LEVELS[-1], 'theta_eta'):.4f} to {_val(results, LEVELS[-1], 'theta_eta_ue'):.4f} (Blasius {exact.blasius_thicknesses()[0]:.4f} and {exact.blasius_thicknesses()[1]:.4f}). The GCI scalars are cf_x0.5, delta_star_eta, theta_eta, delta_star_eta_ue and theta_eta_ue.",
        "Rows named diag_ are not compared quantities and are not in layer0_gci.csv. diag_cf_resqrt_x*: Cf sqrt(Re_x) at the plate nodes x = 0.25, 0.5, 0.75, 1 (exact 0.664). diag_wall_cp_x0.5, diag_wall_dcp_dx_x0.5, diag_column_p_dev, diag_edge_u_ratio: Blasius assumes zero pressure gradient and a uniform outer flow, these show how far the CFD flow is from it. diag_points_in_bl99, diag_first_node_*: resolution at x = 0.5. diag_le_*: the leading-edge node and its neighbours (Cp = (p/p_inf - 1) / (gamma M^2 / 2)).",
        "difference = cfd - exact; difference_pct = 100 (cfd - exact) / exact, empty where there is no exact value or it is 0.",
        "nodes: nodes of the grid; level: nodes in each direction.",
        "residual_drop_orders: the peak of log10 rms[Rho] over the history minus its final value (the convention of rans_flatplate.csv).",
        "iterations: the last inner iteration in history.csv.",
        "stop_reason: 'residual target' if the final rms[Rho] is at most -14, otherwise 'iteration limit'.",
    ]
    return lines


def write_plate(rows, results: dict, est: dict, path: pathlib.Path, variants: dict | None = None) -> None:
    with path.open("w", newline="") as f:
        for ln in header_lines(results, est, variants):
            f.write("# " + ln + "\n")
        w = csv.DictWriter(f, fieldnames=LAYER0_COLUMNS, lineterminator="\n")
        w.writeheader()
        w.writerows(rows)


def _gci_counts(rows: list[dict[str, str]]) -> tuple[int, int]:
    held = sum(r["gci_bounds_error"] == "True" for r in rows)
    failed = sum(r["gci_bounds_error"] == "False" for r in rows)
    return held, failed


def parse_gci_rows(text: str) -> list[dict[str, str]]:
    return list(csv.DictReader(ln for ln in text.split("\n") if ln and not ln.startswith("#")))


def gci_header_lines(base_text: str, rows: list[dict[str, str]]) -> list[str]:
    """The header lines this module adds to layer0_gci.csv (each begins with PLATE_HEADER when written)."""
    other = parse_gci_rows(strip_plate(base_text))
    held, failed = _gci_counts(rows)
    held_all, failed_all = _gci_counts(other + rows)
    bad = [r for r in rows if r["gci_bounds_error"] == "False"]
    crossing = [r["quantity"] for r in rows if "change sign between the medium and the fine grid" in r["note"]]
    names = "/".join(level_name(lv) for lv in reversed(LEVELS))
    cells = ", ".join(f"{MAIN.cells_x * 2 ** lv}x{BASE_CELLS_Y * 2 ** lv}" for lv in LEVELS)
    return [
        f"Rows with condition '{CONDITION}' are the laminar flat plate, appended by python -m nasa_vv.cases.layer0_plate (research/nasa_vv/cases/layer0_plate.py); the rows of the other cases and the header lines above are not touched by it, and a rerun replaces these lines and rows.",
        NO_LAMINAR + "; the header lines above, which say Euler and MARKER_FAR supersonic, describe the other cases.",
        f"The plate has its own header in layer0_plate.csv (leading-edge node, compressibility estimate, resolution, domain study). Grids {names} nodes ({cells} cells), exactly nested, r = 2; h_basis, Fs and the gates are those of the other rows. The exact value is Blasius' (incompressible, semi-infinite plate, boundary-layer limit): a discretisation-error bound cannot cover the part of the difference that is the physics of the leading edge, of compressibility and of the finite domain.",
        f"Plate rows with a computed GCI: {held + failed}; the GCI bounds the finest-grid error for {held} and not for {failed}"
        + ("" if not bad else " (" + ", ".join(f"{r['quantity']}: GCI {float(r['gci_fine_pct']):.3g} % against error {r['true_error_pct']} %" for r in bad) + ")")
        + ("" if not crossing else f". For {', '.join(crossing)} the errors against Blasius change sign between the medium and the fine grid, so a finest-grid error smaller than the GCI is the sequence crossing Blasius and not evidence that the GCI bounds the error of a converged solution (see the notes)")
        + f". Over all rows of this file, plate rows included, of {held_all + failed_all} rows with a computed GCI it held for {held_all} and failed for {failed_all}.",
    ]


def strip_plate(text: str) -> str:
    """The text without the plate rows and header lines: Task 10's bytes.

    A record is one line (no field of layer0_gci.csv contains a newline; tests/test_layer0_plate.py checks it), so the
    plate rows are the lines that start with PLATE_ROW; rows another task appended after them stay.
    """
    return "\n".join(ln for ln in text.split("\n") if not ln.startswith((PLATE_HEADER, PLATE_ROW)))


def merge_gci(base_text: str, rows: list[dict[str, str]]) -> str:
    """layer0_gci.csv with the plate rows appended.

    Everything that is not a plate row or a plate header line is kept byte for byte. The plate header lines go
    in front of the column header, the plate rows at the end, written with the same columns and line terminator
    as the rows above. Merging an already merged text gives the same text.
    """
    base = strip_plate(base_text)
    if not base.endswith("\n"):
        raise ValueError("layer0_gci.csv does not end with a newline")
    lines = base.split("\n")
    at = next((k for k, ln in enumerate(lines) if ln.startswith("study,condition,")), None)
    if at is None:
        raise ValueError("layer0_gci.csv has no column header line")
    head = [PLATE_HEADER + ln for ln in gci_header_lines(base, rows)]
    body = io.StringIO()
    csv.DictWriter(body, fieldnames=GCI_COLUMNS + ["true_error_pct", "gci_bounds_error"], lineterminator="\n").writerows(rows)
    return "\n".join(lines[:at] + head + lines[at:]) + body.getvalue()


def write_gci(rows: list[dict[str, str]], path: pathlib.Path) -> None:
    path.write_bytes(merge_gci(path.read_bytes().decode(), rows).encode())


# ------------------------------------------------------------------ main


def main(argv: list[str] | None = None) -> None:
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("--work", type=pathlib.Path, default=DEFAULT_WORK, help="scratch directory for meshes and SU2 output")
    ap.add_argument("--collect-only", action="store_true", help="skip the SU2 runs; read the output already in --work")
    ap.add_argument("--jobs", type=int, default=4, help="SU2 runs at once")
    ap.add_argument("--su2", type=pathlib.Path, default=SU2_CFD)
    ap.add_argument("--out", type=pathlib.Path, default=RESULTS)
    a = ap.parse_args(argv)
    work = a.work.expanduser().resolve()
    if str(work).startswith("/tmp"):
        sys.exit("--work must not be under /tmp (RAM-backed here)")

    if not a.collect_only:
        jobs = [(MAIN, lv) for lv in reversed(LEVELS)] + [(d, VARIANT_LEVEL) for d in VARIANTS]   # finest first
        with ThreadPoolExecutor(max_workers=a.jobs) as ex:
            futs = [(d, lv, ex.submit(run_level, lv, work, a.su2, d)) for d, lv in jobs]
            for d, lv, f in futs:
                f.result()
                print(f"ran {d.name} {level_name(lv, d)}", flush=True)

    results = {lv: collect_level(lv, work) for lv in LEVELS}
    variants = {d.name: collect_level(VARIANT_LEVEL, work, d, refuse_near_top=False) for d in VARIANTS}
    est = eckert_estimate(freestream(), rans_gas_model(GOLDEN_RANS.read_text()))
    a.out.mkdir(parents=True, exist_ok=True)
    write_plate(build_rows(results, variants), results, est, a.out / "layer0_plate.csv", variants)
    write_gci(gci_rows(results, est, variants), a.out / "layer0_gci.csv")
    print(f"wrote {a.out / 'layer0_plate.csv'} and the plate rows of {a.out / 'layer0_gci.csv'}")


if __name__ == "__main__":
    main()
