# SPDX-License-Identifier: GPL-3.0-or-later
"""The turbulent flat plate under two turbulence models, Spalart-Allmaras and Menter's SST, on SU2 directly.

The tutorial's step on turbulence models asks for several. IgnisYeet offers one: RANS with Spalart-Allmaras
(crates/cfd/src/config.rs:12-18, crates/cfd/src/su2cfg.rs:212 writes KIND_TURB_MODEL= SA and nothing else). That part of
the request cannot be answered through the product, so it is answered here, on SU2, for the one case simple enough to
run both models on grids that nest: the zero-pressure-gradient flat plate of the NASA Turbulence Modeling Resource
(2DZP), M = 0.2, Re = 5e6 per metre, T = 300 K, adiabatic wall, x = 0.97 the station of the TMR's reference skin friction.

SA and SST are solved on the same grids with the same numerics. Every numerics, gas-model and viscosity line is the golden
file's own (nasa_vv.su2cfg.rans_stage_config copies it: tests/data/golden_rans_startup.cfg and golden_rans.cfg, written by
`ignisyeet aero`), so the product's two-stage path is followed: a first-order startup stage and a second-order stage
restarted from it. The only differences between the two models are KIND_TURB_MODEL and the keys SST needs (SA_OPTIONS and
FREESTREAM_NU_FACTOR give way to SST_OPTIONS and the two SST freestream keys); the convection scheme, time discretisation
and CFL reduction of the turbulence equations are the golden's for both. test_su2cfg_rans.py asserts that difference.

The grids are not the TMR's
---------------------------
The TMR grid files that doc/data/cfd/rans_flatplate.csv was computed on (flatplate_clust2_*.p2dfmt, written to SU2 meshes
by doc/scripts/rans_flatplate.py) are not in the repository and not on this machine, and nothing is fetched. The grids here
are built with nasa_vv.mesh: the same extent (-1/3 <= x <= 2, 0 <= y <= 1, the plate from x = 0), the same node counts
(69x49, 137x97, 273x193) and the same boundary conditions, exactly nested (every coarse node is a fine node, the cell count
doubles in each direction, so r = 2 and the constant-ratio form of the GCI applies). The point distribution is not the TMR's;
the wall spacing is chosen to give y+ of the order of 1 on the coarse grid, as the TMR family does. The doc/data SA values
are therefore a cross-check that differs from the SA run here in the grid as well as in the numerics.

Boundaries, as in doc/scripts/rans_flatplate.py: the plate (x >= 0) is a no-slip adiabatic wall, x < 0 on y = 0 is a symmetry
plane, inflow, top and outflow are the characteristic farfield (MARKER_FAR). The farfield on the outflow boundary at the end
of the plate raises cf for x > 1.5 (doc/data/cfd/rans_flatplate.csv, header), so the drag of the plate carries that
boundary effect and is not compared with the TMR's.

Run
---
    cd research && uv run --project nasa_vv python -m nasa_vv.cases.turbulence_models --run
    cd research && uv run --project nasa_vv python -m nasa_vv.cases.turbulence_models

``--run`` solves the six cases (two at a time, four MPI ranks each, so at most 8 ranks) under
~/.cache/nasa_vv_t20_plate; without it the CSV is rebuilt from the output already there, byte for byte the same.
"""
from __future__ import annotations

import argparse
import concurrent.futures as cf
import csv
import hashlib
import math
import os
import pathlib
import re
import subprocess
import time
from dataclasses import dataclass

import numpy as np

from ..family import GridFamily, GridLevel
from ..mesh import body_fitted_grid, refine, structured_quad_su2
from ..su2cfg import (GOLDEN_RANS, GOLDEN_RANS_STARTUP, SST_KEYS, rans_gas_model, rans_stage_config,
                      rans_wall_heat_flux)
from .existing_data import FLATPLATE_CSV, ROOT, FamilyData, _flatplate_compute, _g, _last_commit, _num, _read_table, _rows

RESULTS = pathlib.Path(__file__).resolve().parents[1] / "results"
DEFAULT_WORK = pathlib.Path.home() / ".cache" / "nasa_vv_t20_plate"
SU2_CFD = pathlib.Path.home() / ".local" / "share" / "ignisyeet-su2-native" / "bin" / "SU2_CFD"
MPIRUN = pathlib.Path.home() / ".local" / "share" / "micromamba" / "envs" / "ignisyeet-cfd" / "bin" / "mpirun"

MODELS = ("SA", "SST")
MACH = 0.2
RE_UNIT = 5.0e6                 # per metre
T_INF = 300.0
X_PROBE = 0.97                  # the TMR station of the reference skin friction
REF_AREA = 2.0                  # plate length 2 times unit depth: CD is the mean skin friction
RANKS = 4
JOBS = 2                        # cases at once: JOBS * RANKS <= 8 ranks

ITER_FINAL = 30000              # a budget, the same for both models; the stop rule ends a run earlier
STOP_ORDERS = 8.0               # the final stage stops this many orders of rms[Rho] below the startup peak

# SST freestream and version: the values of SU2's own TMR 2DZP case (TestCases/vandv/rans/flatplate/turb_flatplate_sst.cfg
# of SU2 8.3.0): Tu = 3.8729e-4 and mu_t/mu = 0.009 give k = 2.25e-7 U^2 and omega = 125 U / L (k/a^2 = 9e-9, the TMR's).
SST = {"SST_OPTIONS": "V1994m", "FREESTREAM_TURBULENCEINTENSITY": "0.00038729", "FREESTREAM_TURB2LAMVISCRATIO": "0.009"}
assert tuple(SST) == SST_KEYS

# ------------------------------------------------------------------ grids

X_RANGE = (-1.0 / 3.0, 2.0)
Y_TOP = 1.0
BASE_CELLS_UP = 16              # cells of the 69x49 grid between the inlet and the leading edge
BASE_CELLS_PLATE = (40, 12)     # cells on 0 <= x <= X_PROBE and X_PROBE <= x <= 2
BASE_CELLS_Y = 48
BASE_CELLS_X = BASE_CELLS_UP + sum(BASE_CELLS_PLATE)   # 68
LEVELS = (0, 1, 2)
STRETCH_X = 2.5                 # exponential clustering towards the leading edge, upstream and on the first plate part
STRETCH_Y = 11.0                # y = (exp(b s) - 1) / (exp(b) - 1)


def _expm1_ratio(b: float, t: np.ndarray) -> np.ndarray:
    return np.expm1(b * t) / math.expm1(b)


def _uniform(cells_base: int, level: int) -> np.ndarray:
    """t in [0, 1] with ``cells_base`` cells, refined ``level`` times by midpoint insertion (the coarse values are kept to the bit)."""
    t = np.linspace(0.0, 1.0, cells_base + 1)
    for _ in range(level):
        t, _ = refine(t, t)
    return t


def xi_nodes(level: int) -> np.ndarray:
    """Parameter values xi in [0, 1] of the nodes along x on a level: a fixed function of the uniformly refined parameter.

    Three pieces, each a fixed function of its own uniform parameter t in [0, 1]: the inlet to the leading edge,
    clustered towards the leading edge; the leading edge to X_PROBE, clustered towards the leading edge; X_PROBE to the
    outlet, uniform. x = 0 and x = X_PROBE are nodes of every level."""
    x0, x1 = X_RANGE
    t = _uniform(BASE_CELLS_UP, level)
    up = x0 * _expm1_ratio(STRETCH_X, 1.0 - t)                               # -1/3 .. 0, small cells at 0
    a = X_PROBE * _expm1_ratio(STRETCH_X, _uniform(BASE_CELLS_PLATE[0], level))
    b = X_PROBE + (x1 - X_PROBE) * _uniform(BASE_CELLS_PLATE[1], level)
    x = np.concatenate([up, a[1:], b[1:]])
    return (x - x0) / (x1 - x0)


def eta_nodes(level: int) -> np.ndarray:
    return _expm1_ratio(STRETCH_Y, _uniform(BASE_CELLS_Y, level))


def grid(level: int):
    """(X, Y, k) of a level: node arrays of shape (nj, ni) and the index along the lower boundary of the leading edge."""
    X, Y = body_fitted_grid(xi_nodes(level), eta_nodes(level), X_RANGE, lambda x: np.zeros_like(x), Y_TOP)
    return X, Y, BASE_CELLS_UP * 2 ** level


def level_name(level: int) -> str:
    return f"{BASE_CELLS_X * 2 ** level + 1}x{BASE_CELLS_Y * 2 ** level + 1}"


def cells(level: int) -> int:
    return BASE_CELLS_X * BASE_CELLS_Y * 4 ** level


def mesh_markers(ni: int, k: int) -> dict:
    return {"symmetry": [("ymin", 0, k)], "wall": [("ymin", k, ni - 1)],
            "inlet": ["xmin"], "outlet": ["xmax"], "top": ["ymax"]}


# ------------------------------------------------------------------ the configurations


def freestream_pressure(golden: str) -> float:
    """p [Pa] that gives Re = RE_UNIT per metre at MACH and T_INF with the golden's gas constant and Sutherland law."""
    g = rans_gas_model(golden)
    a = math.sqrt(g["gamma"] * g["gas_constant"] * T_INF)
    u = MACH * a
    mu = g["mu_ref"] * (T_INF / g["t_ref"]) ** 1.5 * (g["t_ref"] + g["sutherland"]) / (T_INF + g["sutherland"])
    rho = RE_UNIT * mu / u
    return rho * g["gas_constant"] * T_INF


def case_keys(golden: str) -> dict[str, str]:
    return {
        "MACH_NUMBER": f"{MACH:.6f}",
        "FREESTREAM_PRESSURE": f"{freestream_pressure(golden):.4f}",
        "FREESTREAM_TEMPERATURE": f"{T_INF:.4f}",
        "REF_LENGTH": "1.0",
        "REF_AREA": f"{REF_AREA}",
        "MARKER_HEATFLUX": f"( wall, {rans_wall_heat_flux(golden)} )",
        "MARKER_MONITORING": "( wall )",
        "MARKER_FAR": "( inlet, top, outlet )",
        "MARKER_SYM": "( symmetry )",
        "MARKER_PLOTTING": "( wall )",
        "MESH_FILENAME": "mesh.su2",
        "SCREEN_OUTPUT": "( INNER_ITER, RMS_DENSITY, DRAG, AVG_CFL )",
    }


def startup_config(model: str) -> str:
    g = GOLDEN_RANS_STARTUP.read_text()
    return rans_stage_config(g, model=model, case=case_keys(g), sst=SST if model == "SST" else None)


def final_config(model: str, floor: float) -> str:
    g = GOLDEN_RANS.read_text()
    return rans_stage_config(
        g, model=model, case={**case_keys(g), "ITER": str(ITER_FINAL)},
        stop={"CONV_FIELD": "RMS_DENSITY", "CONV_RESIDUAL_MINVAL": f"{floor:.3f}"},
        sst=SST if model == "SST" else None)


def case_dir(work: pathlib.Path, model: str, level: int) -> pathlib.Path:
    return work / model / level_name(level)


# ------------------------------------------------------------------ running


def read_history(path: pathlib.Path) -> tuple[list[str], np.ndarray]:
    with path.open(newline="") as f:
        rows = list(csv.reader(f))
    head = [h.strip().strip('"') for h in rows[0]]
    return head, np.array([[float(c) for c in r] for r in rows[1:] if r])


def _mpi(cfg: str, d: pathlib.Path, ranks: int, log: str) -> None:
    env = dict(os.environ, OMP_NUM_THREADS="1")
    cmd = [str(MPIRUN), "-n", str(ranks), "--oversubscribe", "--bind-to", "none", str(SU2_CFD), cfg]
    with (d / log).open("w") as fh:
        r = subprocess.run(cmd, cwd=d, env=env, stdout=fh, stderr=subprocess.STDOUT)
    if r.returncode != 0:
        raise RuntimeError(f"SU2_CFD failed in {d} for {cfg} (exit {r.returncode}); see {d / log}")


def run_case(work: pathlib.Path, model: str, level: int, ranks: int = RANKS) -> pathlib.Path:
    d = case_dir(work, model, level)
    d.mkdir(parents=True, exist_ok=True)
    for stale in d.iterdir():
        if stale.is_file():
            stale.unlink()
    X, Y, k = grid(level)
    structured_quad_su2(d / "mesh.su2", X, Y, mesh_markers(X.shape[1], k))
    (d / "startup.cfg").write_text(startup_config(model))
    t0 = time.time()
    _mpi("startup.cfg", d, ranks, "log_startup.txt")
    head, data = read_history(d / "history_startup.csv")
    peak = float(data[1:, head.index("rms[Rho]")].max())      # iteration 0 is the uniform field
    floor = peak - STOP_ORDERS
    (d / "final.cfg").write_text(final_config(model, floor))
    _mpi("final.cfg", d, ranks, "log_final.txt")
    (d / "wall_seconds.txt").write_text(f"{time.time() - t0:.1f}\n")
    return d


def run_all(work: pathlib.Path, levels=LEVELS, jobs: int = JOBS, ranks: int = RANKS) -> None:
    """Every (model, level), coarse grids first, ``jobs`` cases at a time (jobs * ranks MPI ranks in all)."""
    if jobs * ranks > 8:
        raise ValueError(f"{jobs} jobs of {ranks} ranks is {jobs * ranks} ranks; the limit is 8")
    todo = [(m, lv) for lv in levels for m in MODELS]
    with cf.ThreadPoolExecutor(jobs) as ex:
        futs = {ex.submit(run_case, work, m, lv, ranks): (m, lv) for m, lv in todo}
        for fu in cf.as_completed(futs):
            print("done", futs[fu], fu.result(), flush=True)



# ------------------------------------------------------------------ reading a run

TURBULENCE_RESIDUALS = {"SA": ("rms[nu]",), "SST": ("rms[k]", "rms[w]")}
CD_WINDOW = 500                 # iterations over which the drift of CD is reported


def read_vtk(path: pathlib.Path) -> tuple[np.ndarray, dict[str, np.ndarray]]:
    """Points and point data of an ASCII legacy VTK file written by SU2 (the wall marker only)."""
    tok = path.read_text().split()
    i = tok.index("POINTS")
    n = int(tok[i + 1])
    pts = np.array(tok[i + 3:i + 3 + 3 * n], dtype=float).reshape(n, 3)
    fields: dict[str, np.ndarray] = {}
    k = tok.index("POINT_DATA") + 2
    while k < len(tok):
        if tok[k] == "SCALARS":
            name = tok[k + 1]
            k += 4
            if tok[k] == "LOOKUP_TABLE":
                k += 2
            fields[name] = np.array(tok[k:k + n], dtype=float)
            k += n
        elif tok[k] == "VECTORS":
            name = tok[k + 1]
            k += 3
            fields[name] = np.array(tok[k:k + 3 * n], dtype=float).reshape(n, 3)
            k += 3 * n
        else:
            k += 1
    return pts, fields


@dataclass
class Run:
    model: str
    level: int
    cf: float                   # Skin_Friction_Coefficient (x component) at x = X_PROBE
    cd: float                   # CD of the wall, last iteration
    yplus_max: float            # over the plate nodes with x > 0.01
    yplus_mean: float
    rho_drop: float             # orders of rms[Rho] below the startup-stage peak
    turb_drops: dict[str, float]
    iterations: int             # final-stage iterations
    iterations_startup: int
    stop: str                   # 'residual target' or 'iteration limit'
    cd_drift_pct: float         # 100 (CD(last) / CD(last - CD_WINDOW) - 1)
    wall_s: float | None

    @property
    def grid(self) -> str:
        return level_name(self.level)

    @property
    def cells(self) -> int:
        return cells(self.level)

    @property
    def residual_drop(self) -> float:
        """The weakest equation governs: min over rho and the turbulence equations."""
        return min(self.rho_drop, *self.turb_drops.values())


def _floor(final_cfg: str) -> float:
    m = re.search(r"^CONV_RESIDUAL_MINVAL= (\S+)", final_cfg, re.M)
    if not m:
        raise ValueError("final.cfg has no CONV_RESIDUAL_MINVAL")
    return float(m.group(1))


def read_run(work: pathlib.Path, model: str, level: int) -> Run:
    d = case_dir(work, model, level)
    for need in ("history_startup.csv", "history.csv", "surface_flow.vtk", "final.cfg"):
        if not (d / need).is_file():
            raise FileNotFoundError(f"{d / need} not found; run the case first (--run)")
    hs, ds = read_history(d / "history_startup.csv")
    h, dat = read_history(d / "history.csv")
    cols = ("rms[Rho]", *TURBULENCE_RESIDUALS[model])
    for c in cols:
        if c not in h or c not in hs:
            raise ValueError(f"{d}: column {c} missing from the history")
    peak = {c: float(ds[1:, hs.index(c)].max()) for c in cols}      # iteration 0 is the uniform field
    last = {c: float(dat[-1, h.index(c)]) for c in cols}
    drops = {c: peak[c] - last[c] for c in cols}
    cd_col = dat[:, h.index("CD")]
    window = min(CD_WINDOW, len(cd_col) - 1)
    pts, f = read_vtk(d / "surface_flow.vtk")
    order = np.argsort(pts[:, 0])
    x, cfx, yp = pts[order, 0], f["Skin_Friction_Coefficient"][order, 0], f["Y_Plus"][order]
    hit = np.flatnonzero(np.abs(x - X_PROBE) < 1e-9)
    if len(hit) != 1:
        raise ValueError(f"{d}: x = {X_PROBE} is not a node of the wall (the grid puts a node there)")
    plate = x > 0.01
    wall = d / "wall_seconds.txt"
    floor = _floor((d / "final.cfg").read_text())
    return Run(
        model, level, float(cfx[hit[0]]), float(cd_col[-1]), float(yp[plate].max()), float(yp[plate].mean()),
        drops["rms[Rho]"], {c: drops[c] for c in cols[1:]}, len(cd_col), len(ds),
        "residual target" if last["rms[Rho]"] <= floor + 1e-6 else "iteration limit",
        100.0 * (cd_col[-1] / cd_col[-1 - window] - 1.0), float(wall.read_text()) if wall.is_file() else None)


# ------------------------------------------------------------------ the table

QUANTITIES = ("cf_x097", "cd_plate")
ROW_COLUMNS = ["kind", "model", "quantity", "grid", "cells", "value", "reference", "difference_pct",
               "sa_gci_pct", "sst_gci_pct", "separable", "order", "extrapolated", "gci_fine_pct", "gci_coarse_pct",
               "asymptotic_ratio", "residual_drop_orders", "iterations", "yplus_max", "note"]


def _value(run: Run, quantity: str) -> float:
    return run.cf if quantity == "cf_x097" else run.cd


def family(runs: list[Run]) -> FamilyData:
    """The three levels of one model, finest first, with both quantities."""
    fine_first = sorted(runs, key=lambda r: -r.level)
    levels = [GridLevel(r.grid, r.cells, {"h": r.cells ** -0.5}, residual_drop_orders=r.residual_drop) for r in fine_first]
    return FamilyData(GridFamily(levels, ndim=2), {q: [_value(r, q) for r in fine_first] for q in QUANTITIES})


def gci_by_model(runs: dict[str, list[Run]]) -> dict[str, dict[str, dict[str, str]]]:
    """model -> quantity -> the row of nasa_vv.cases.existing_data._rows (the constant-ratio form, r = 2, Fs = 1.25)."""
    out = {}
    for m, rs in runs.items():
        fd = family(rs)
        trip = "/".join(lv.name for lv in fd.family.levels)
        rows = _rows("turbulence_models", f"{m} M={MACH:g}", trip, fd, _flatplate_compute, "cells^-1/2 (2-D)")
        out[m] = {r["quantity"]: r for r in rows}
    return out


def _f(row: dict[str, str], col: str) -> float | None:
    return float(row[col]) if row.get(col, "") != "" else None


def separable(difference_pct: float, sa_gci_pct: float | None, sst_gci_pct: float | None) -> bool | None:
    """True if |difference| exceeds the sum of the two bands, False if it does not, None when a band is missing."""
    if sa_gci_pct is None or sst_gci_pct is None:
        return None
    return abs(difference_pct) > sa_gci_pct + sst_gci_pct


def _usable_band(g: dict[str, str], col: str | None) -> tuple[float | None, str]:
    """(GCI in percent, "") of a study row at the level ``col`` names, or (None, why not): no level, refused, no GCI
    computed (oscillatory), or flagged 'not reliable' (observed order outside 0.5 to 4). A flagged value is not used."""
    if col is None:
        return None, "no GCI at the coarsest level"
    if g["gci_fine_pct"] == "" and g["gci_coarse_pct"] == "":
        return None, g["note"] or "no GCI computed"
    if "not reliable" in g["note"]:
        return None, f"{g['note']} (computed {col} = {float(g[col]):.3g})"
    return float(g[col]), ""


def _pct(a: float, b: float) -> float:
    return 100.0 * (a - b) / b


def _blank() -> dict[str, str]:
    return {c: "" for c in ROW_COLUMNS}


def build_rows(runs: dict[str, list[Run]], reference: dict[str, dict[str, float]]) -> list[dict[str, str]]:
    """``reference`` maps a grid name to the doc/data SA row's numbers: cf_x097, cd_plate and tmr_sa_cf."""
    gci = gci_by_model(runs)
    out: list[dict[str, str]] = []
    # per-level values
    for m in MODELS:
        for r in sorted(runs[m], key=lambda r: r.level):
            for q in QUANTITIES:
                row = _blank()
                note = f"stopped by the {r.stop}; CD changed by {r.cd_drift_pct:.2g} % over the last {CD_WINDOW} iterations"
                row.update(kind="level", model=m, quantity=q, grid=r.grid, cells=str(r.cells), value=_g(_value(r, q), ".8g"),
                           residual_drop_orders=_g(r.residual_drop, ".4g"), iterations=str(r.iterations),
                           yplus_max=_g(r.yplus_max, ".3g"), note=note)
                out.append(row)
    # grid convergence per model
    for m in MODELS:
        for q in QUANTITIES:
            g = gci[m][q]
            row = _blank()
            row.update(kind="gci", model=m, quantity=q, grid=g["triplet"], cells=g["n1"], value=g["f1"],
                       order=g["order"], extrapolated=g["extrapolated"], gci_fine_pct=g["gci_fine_pct"],
                       gci_coarse_pct=g["gci_coarse_pct"], asymptotic_ratio=g["asymptotic_ratio"], note=g["note"])
            out.append(row)
    # SST against SA
    for q in QUANTITIES:
        sa = {r.level: _value(r, q) for r in runs["SA"]}
        sst = {r.level: _value(r, q) for r in runs["SST"]}
        ga, gs = gci["SA"][q], gci["SST"][q]
        for level, band in ((2, "gci_fine_pct"), (1, "gci_coarse_pct"), (0, None)):
            row = _blank()
            pct = _pct(sst[level], sa[level])
            a, a_why = _usable_band(ga, band)
            s_, s_why = _usable_band(gs, band)
            verdict = "" if separable(pct, a, s_) is None else str(separable(pct, a, s_))
            notes = ["value = SST - SA, reference = SA"]
            if band is None:
                notes.append("the coarsest level has no GCI")
            else:
                notes.append(f"the bands are the {band.replace('_pct', '').replace('gci_', 'GCI ')} of each model at this level")
                nxt = level - 1
                notes.append(f"refining the next coarser grid into this one changed SA by {abs(_pct(sa[nxt], sa[level])):.3g} % and SST by "
                             f"{abs(_pct(sst[nxt], sst[level])):.3g} % of this level's value")
                for name, val, why, other in (("SA", a, a_why, s_), ("SST", s_, s_why, a)):
                    if val is None:
                        notes.append(f"{name} band not used: {why}")
                        if other is not None and abs(pct) <= other:
                            notes.append(f"abs(difference_pct) is not larger than the {'SST' if name == 'SA' else 'SA'} band alone, so it lies inside the sum of the bands whatever the {name} band is (stated here, not written as a verdict)")
            row.update(kind="model_difference", model="SST-SA", quantity=q, grid=level_name(level), cells=str(cells(level)),
                       value=_g(sst[level] - sa[level], ".6g"), reference=_g(sa[level], ".8g"), difference_pct=_g(pct, ".5g"),
                       sa_gci_pct=_g(a, ".5g"), sst_gci_pct=_g(s_, ".5g"), separable=verdict, note="; ".join(notes))
            out.append(row)
        ea, es = _f(ga, "extrapolated"), _f(gs, "extrapolated")
        if ea is not None and es is not None:
            row = _blank()
            row.update(kind="model_difference", model="SST-SA", quantity=q, grid="extrapolated", value=_g(es - ea, ".6g"),
                       reference=_g(ea, ".8g"), difference_pct=_g(_pct(es, ea), ".5g"),
                       note="Richardson-extrapolated values (estimates of the grid-converged value), each model with its own observed order; value = SST - SA, reference = SA; no verdict is written for an estimate")
            out.append(row)
    # cross-checks of SA
    for r in sorted(runs["SA"], key=lambda r: r.level):
        ref = reference.get(r.grid)
        if ref is None:
            continue
        for q in QUANTITIES:
            row = _blank()
            row.update(kind="cross_check", model="SA", quantity=q, grid=r.grid, cells=str(r.cells), value=_g(_value(r, q), ".8g"),
                       reference=_g(ref[q], ".8g"), difference_pct=_g(_pct(_value(r, q), ref[q]), ".4g"),
                       note=f"reference: doc/data/cfd/rans_flatplate.csv {q}, SA on the TMR grid of the same node count; the grid and the numerics differ (see the header)")
            out.append(row)
        if ref.get("tmr_sa_cf") is not None:
            row = _blank()
            row.update(kind="cross_check", model="SA", quantity="cf_x097", grid=r.grid, cells=str(r.cells), value=_g(r.cf, ".8g"),
                       reference=_g(ref["tmr_sa_cf"], ".8g"), difference_pct=_g(_pct(r.cf, ref["tmr_sa_cf"]), ".4g"),
                       note="reference: TMR 2DZP SA, mean of CFL3D and FUN3D on the 545x385 grid (tmr_sa_cf of rans_flatplate.csv); cross-comparison, not validation")
            out.append(row)
    ea = _f(gci["SA"]["cf_x097"], "extrapolated")
    tmr = next((v["tmr_sa_cf"] for v in reference.values() if v.get("tmr_sa_cf") is not None), None)
    if ea is not None and tmr is not None:
        row = _blank()
        row.update(kind="cross_check", model="SA", quantity="cf_x097", grid="extrapolated", value=_g(ea, ".8g"),
                   reference=_g(tmr, ".8g"), difference_pct=_g(_pct(ea, tmr), ".4g"),
                   note="Richardson-extrapolated cf_x097 of the SA runs against the TMR SA value on 545x385; cross-comparison, not validation")
        out.append(row)
    return out


def read_reference(root: pathlib.Path = ROOT) -> dict[str, dict[str, float]]:
    """The doc/data SA numbers of M = 0.2 by grid name: cf_x097, cd_plate and the TMR value."""
    rows = _read_table(root / FLATPLATE_CSV, ("mach", "grid", "cf_x097", "cd_plate", "tmr_sa_cf"))
    return {r["grid"]: {"cf_x097": _num(r, "cf_x097"), "cd_plate": _num(r, "cd_plate"), "tmr_sa_cf": _num(r, "tmr_sa_cf")}
            for r in rows if abs(float(r["mach"]) - MACH) < 1e-9}


def _sha(path: pathlib.Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()[:12]


def su2_version(work: pathlib.Path) -> str:
    log = case_dir(work, "SA", 0) / "log_final.txt"
    if log.is_file():
        m = re.search(r"SU2 v(\S+)", log.read_text(errors="replace"))
        if m:
            return m.group(1)
    return "unknown"


def _header(work: pathlib.Path, runs: dict[str, list[Run]], gci: dict) -> list[str]:
    from .. import mesh as _mesh  # noqa: F401  (the grids are built by nasa_vv.mesh)
    lines = [
        "The turbulent flat plate under two turbulence models, Spalart-Allmaras and Menter SST, on SU2 directly: M = 0.2, Re = 5e6 per metre, T = 300 K, adiabatic wall, the zero-pressure-gradient plate of the NASA TMR (2DZP), station x = 0.97.",
        "Why on SU2 and not through the product: IgnisYeet offers Euler and RANS with Spalart-Allmaras only (crates/cfd/src/config.rs:12-18; crates/cfd/src/su2cfg.rs:212 writes KIND_TURB_MODEL= SA and nothing else). The tutorial's several turbulence models are the part of its request the product cannot answer itself.",
        "Producer: python -m nasa_vv.cases.turbulence_models (research/nasa_vv/cases/turbulence_models.py); the configurations are built by nasa_vv.su2cfg.rans_stage_config.",
        f"Solver: SU2 {su2_version(work)} (ignisyeet-su2-native), {RANKS} MPI ranks per case, {JOBS} cases at a time (8 ranks in all), OMP_NUM_THREADS=1, on the local machine (i7-12700, 20 threads, shared with other work).",
        f"Numerics: the product's. Every numerics, gas-model and viscosity line of both stages is copied verbatim from tests/data/golden_rans_startup.cfg (sha256 {_sha(GOLDEN_RANS_STARTUP)}) and tests/data/golden_rans.cfg (sha256 {_sha(GOLDEN_RANS)}), written by `ignisyeet aero`: Roe, Green-Gauss gradients with weighted-least-squares reconstruction, MUSCL with the Van Albada edge limiter in the second stage, Euler implicit, FGMRES with ILU(0), CFL 5 adapted up to 200, scalar upwind first-order turbulence convection. The product's path is followed: a first-order startup stage (at most 500 iterations, 2 orders of REL_RMS_DENSITY, CFL_REDUCTION_TURB 0.2) and a second-order stage restarted from it.",
        "SA and SST differ only in KIND_TURB_MODEL and the keys SST needs: SA_OPTIONS= ( NEGATIVE, WITHFT2 ) and FREESTREAM_NU_FACTOR= 3.0 (the product's) give way to SST_OPTIONS= V1994m, FREESTREAM_TURBULENCEINTENSITY= 0.00038729 and FREESTREAM_TURB2LAMVISCRATIO= 0.009 (k = 2.25e-7 U^2, omega = 125 U/L; the values of SU2 8.3.0's own TMR 2DZP SST case, TestCases/vandv/rans/flatplate/turb_flatplate_sst.cfg, not checked against the TMR's text, which is not in the repository). The SST version V1994m is that case's; SU2's default is V2003m, not run here. tests/test_su2cfg_rans.py asserts this set of differences.",
        f"Stop rule, the same for both models and not the product's acceptance: the second stage ends when rms[Rho] is {STOP_ORDERS:g} orders below the startup-stage peak (CONV_FIELD= RMS_DENSITY, CONV_RESIDUAL_MINVAL set per case from the startup history) or after ITER= {ITER_FINAL} iterations. residual_drop_orders is the weakest equation: min of rho and the turbulence residuals (SA: nu; SST: k and omega), each below its own startup-stage peak. The product's own rule (rho 3 and nu 2 orders, coefficients flat) is not applied here.",
        "Freestream: the golden's gas constant (287.05287) and Sutherland law with p = " + f"{freestream_pressure(GOLDEN_RANS.read_text()):.4f}" + " Pa, T = 300 K, M = 0.2: SU2 reports Re = 5e+06 per metre (log). The doc/data runs used R = 287.058.",
        "Boundaries (as doc/scripts/rans_flatplate.py): the plate x >= 0 is a no-slip adiabatic wall (MARKER_HEATFLUX 0.0), x < 0 on y = 0 is MARKER_SYM, inflow, top and outflow are MARKER_FAR. The characteristic farfield on the outflow boundary at the end of the plate raises cf for x > 1.5 (rans_flatplate.csv header), so cd_plate carries that effect and is not compared with the TMR drag.",
        "THE GRIDS ARE NOT THE TMR'S. The TMR grid files (flatplate_clust2_*.p2dfmt) behind doc/data/cfd/rans_flatplate.csv are not in the repository or on this machine and nothing was fetched. These grids are built by nasa_vv.mesh with the same extent (-1/3 <= x <= 2, 0 <= y <= 1, plate from x = 0), the same node counts (69x49, 137x97, 273x193) and boundary conditions, exactly nested (every coarse node is a fine node; the cells double in each direction, so r = 2). The point distribution is mine: x clustered exponentially towards the leading edge (b = 2.5) on 0 <= x <= 0.97 and upstream, uniform beyond, with x = 0 and x = 0.97 nodes of every level; y = (exp(11 s) - 1)/(exp(11) - 1) with 48 base cells. y+ over the plate (x > 0.01), maximum: " + ", ".join(f"{r.yplus_max:.2f}" for r in sorted(runs['SA'], key=lambda r: r.level)) + " (SA, 69x49, 137x97, 273x193); doc/data/cfd/rans_flatplate.csv (SA): 1.24, 0.71, 0.42.",
        "Quantities: cf_x097 = SU2 Skin_Friction_Coefficient (x component) at the wall node x = 0.97 (a node of every grid; no interpolation) from surface_flow.vtk of the second stage. cd_plate = CD of the wall marker, REF_AREA 2 (the plate length times unit depth), the last iteration.",
        "GCI: the NASA tutorial's constant-ratio form (r = 2, Fs = 1.25 for three grids), nasa_vv.gci.constant_ratio through nasa_vv.family (h = cells^-1/2; the nested 2-D structured grids make r exactly 2, so the variable-ratio form and a second h basis are not needed). gci_fine_pct and gci_coarse_pct are percent of the fine-grid and medium-grid values.",
        "kind = level: one value per model and grid. kind = gci: the grid-convergence study of one model and quantity (grid is the triplet, finest first; value is the fine-grid value). kind = model_difference: SST - SA (value), in percent of SA (difference_pct), at each grid and between the extrapolated values; sa_gci_pct and sst_gci_pct are the two bands at that level (the fine level uses gci_fine_pct, the medium level gci_coarse_pct, the coarsest has none) and separable is True when abs(difference_pct) exceeds their sum and False when it does not; it is empty, and the band empty, where a band does not exist or is flagged 'not reliable' (observed order outside 0.5 to 4; the note then says what the other band and the change of the last refinement show). kind = cross_check: the SA runs against the doc/data SA row of the same node count and against the TMR SA value; these differ from the runs here in grid and numerics (R, the stop rule, the SA CFL in the startup), and are cross-comparison, not validation.",
        "SA against SST is a model-to-model difference, stated against the GCI of each. It is not validation and not an error of either model: no measured data enters. The repository holds one published reference, the TMR SA cf at x = 0.97 (CFL3D 0.270562e-2 and FUN3D 0.270540e-2 on 545x385); it holds no SST reference value, so SST is compared with SA only. The product's SA is SA-neg with ft2 (SA_OPTIONS= ( NEGATIVE, WITHFT2 )), not the TMR's plain SA.",
    ]
    return ["# " + ln for ln in lines]


def collect(work: pathlib.Path) -> tuple[dict[str, list[Run]], list[dict[str, str]]]:
    runs = {m: [read_run(work, m, lv) for lv in LEVELS] for m in MODELS}
    return runs, build_rows(runs, read_reference())


def write(work: pathlib.Path, out: pathlib.Path) -> pathlib.Path:
    runs, rows = collect(work)
    out.parent.mkdir(parents=True, exist_ok=True)
    with out.open("w", newline="") as f:
        for ln in _header(work, runs, None):
            f.write(ln + "\n")
        w = csv.DictWriter(f, fieldnames=ROW_COLUMNS, lineterminator="\n")
        w.writeheader()
        w.writerows(rows)
    return out


def main(argv: list[str] | None = None) -> None:
    p = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    p.add_argument("--work", type=pathlib.Path, default=DEFAULT_WORK)
    p.add_argument("--run", action="store_true", help="solve the six cases first")
    p.add_argument("--levels", type=int, nargs="+", default=list(LEVELS))
    p.add_argument("--out", type=pathlib.Path, default=RESULTS / "turbulence_models.csv")
    a = p.parse_args(argv)
    if a.run:
        run_all(a.work, a.levels)
        if sorted(a.levels) != list(LEVELS):
            return
    print(write(a.work, a.out))


if __name__ == "__main__":
    main()
