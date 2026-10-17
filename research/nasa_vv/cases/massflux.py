# SPDX-License-Identifier: GPL-3.0-or-later
"""Farfield mass-flux balance of the layer 0 cases (spec section 6.3, fourth consistency check).

This is verification: in a steady flow the mass that enters a domain leaves it, so the fluxes SU2
reports through the boundary markers must add up to zero. The domain of each layer 0 case is
a rectangle whose boundaries are the inlet, the outlet, the top (all MARKER_FAR) and the lower
boundary (an Euler wall, or for ``cone_sym_axis`` a symmetry axis and an Euler cone), so the balance
is a clean integral. The 3-D rocket model is not used.

SU2 8.3.0 reports the flux itself: a configuration with MARKER_ANALYZE listing every boundary marker,
MARKER_ANALYZE_AVERAGE= MASSFLUX and SURFACE_MASSFLOW in HISTORY_OUTPUT puts one
``Avg_Massflow(<marker>)`` column per marker into history.csv (``nasa_vv.su2cfg.layer0_config``,
its ``analyze=`` parameter). This module reads the last row of each history. The sign of the column is
positive for flux into the domain; the inlet reads positive and the outlet negative. A marker with a
net positive flux counts to the inflow and one with a net negative flux to the outflow, so the balance
closes over every marker and nothing is left out by name.

The runs are those of ``nasa_vv.cases.layer0`` (its case, grid and configuration builders) with every
marker analysed; ``--run`` makes them, otherwise ``--work`` is read as it stands.

    cd research && uv run --project nasa_vv python -m nasa_vv.cases.massflux --work "$HOME/.cache/nasa_vv_t19_layer0" --run

``flux_imbalance`` is abs(inflow - outflow) / abs(inflow).
"""
from __future__ import annotations

import argparse
import csv
import math
import pathlib
import sys
from concurrent.futures import ThreadPoolExecutor

import numpy as np

from . import layer0 as L
from .existing_data import _last_commit

RESULTS = pathlib.Path(__file__).resolve().parents[1] / "results" / "massflux.csv"
DEFAULT_WORK = pathlib.Path.home() / ".cache" / "nasa_vv_t19_layer0"
LAYER0_PY = "research/nasa_vv/cases/layer0.py"
METHOD = "SU2 Avg_Massflow(marker), last history row"

PRINT_FLOOR = 1e-9
"""history.csv prints about 10 significant digits (1042.150354), so a relative imbalance at or below 1e-9 is not resolved."""
TOLERANCE = 1e-3
"""The NASA tutorial's relative mass-flux imbalance that counts as consistent."""

COLUMNS = ["case", "mach", "level", "inflow_kg_s", "outflow_kg_s", "imbalance_rel", "method", "source",
           "resolved", "imbalance_inlet_outlet_rel", "inlet_vs_freestream_rel", "marker_fluxes_kg_s",
           "residual_drop_orders", "stop_reason"]


def shown(imbalance: float) -> str:
    """The imbalance as written in the CSV: 4 significant digits, or '<1e-09' where it is not resolved."""
    return f"<{PRINT_FLOOR:g}" if imbalance <= PRINT_FLOOR else format(imbalance, ".4g")


def flux_imbalance(inflow_kg_s: float, outflow_kg_s: float) -> float:
    """abs(inflow - outflow) / abs(inflow); ValueError on a NaN or on a zero inflow."""
    if math.isnan(inflow_kg_s) or math.isnan(outflow_kg_s):
        raise ValueError(f"mass flux is NaN (inflow {inflow_kg_s}, outflow {outflow_kg_s})")
    if inflow_kg_s == 0.0:
        raise ValueError("zero inflow: the relative imbalance is undefined")
    return abs(inflow_kg_s - outflow_kg_s) / abs(inflow_kg_s)


def split_fluxes(fluxes: dict[str, float]) -> tuple[float, float]:
    """(inflow, outflow) of per-marker fluxes that are positive into the domain; both are non-negative.

    A marker with a positive net flux adds to the inflow, one with a negative net flux to the outflow.
    """
    bad = [m for m, v in fluxes.items() if math.isnan(v)]
    if bad:
        raise ValueError(f"mass flux is NaN on {bad}")
    return (sum(v for v in fluxes.values() if v > 0.0),
            sum(-v for v in fluxes.values() if v < 0.0))


def analysed_markers(case: L.Case) -> list[str]:
    """Every boundary marker of the case's mesh: the lower boundary marker(s), inlet, outlet and top."""
    return list(L.mesh_markers(case, 2, 0))


def read_marker_fluxes(path: pathlib.Path) -> dict:
    """Last history row: {'fluxes': {marker: flux}, 'iterations': n, 'drop': orders, 'stop': text}."""
    with path.open(newline="") as f:
        rows = list(csv.reader(f))
    head = [h.strip().strip('"') for h in rows[0]]
    last = [float(c) for c in rows[-1]]
    fluxes = {h[len("Avg_Massflow("):-1]: last[i] for i, h in enumerate(head) if h.startswith("Avg_Massflow(")}
    if not fluxes:
        raise RuntimeError(f"{path}: no Avg_Massflow(<marker>) column; run with MARKER_ANALYZE and SURFACE_MASSFLOW")
    hist = L.read_history(path)
    return {"fluxes": fluxes, "iterations": hist["iterations"], "drop": hist["drop"], "stop": hist["stop"],
            "mass_in": hist["mass_in"], "mass_out": hist["mass_out"]}


def freestream_flux(case: L.Case) -> tuple[float, float]:
    """(rho_inf * U_inf, extent of the inlet): the analytic mass flux density and the inlet's height or radius."""
    gamma, r_gas = L.gas_constants()
    rho = L.P_INF / (r_gas * L.T_INF)
    speed = case.mach * math.sqrt(gamma * r_gas * L.T_INF)
    extent = case.y_top - float(case.y_wall(np.array([case.x_range[0]]))[0])
    return rho * speed, extent


def analytic_inflow(case: L.Case) -> float:
    """Freestream mass flux through the inlet: rho U H planar (per metre of span), pi rho U R^2 axisymmetric (2 pi r)."""
    rho_u, extent = freestream_flux(case)
    return math.pi * rho_u * extent ** 2 if case.axisymmetric else rho_u * extent


def collect(work: pathlib.Path, reference: pathlib.Path | None = None) -> list[dict]:
    """One row per case and level from the histories under ``work``.

    With ``reference`` (the Task 10 runs) each level's inlet and outlet fluxes and its last inner iteration
    must equal the reference history's, which shows the extra analysed markers did not change the solve.
    """
    rows = []
    for name, case in L.CASES.items():
        ref_flux = analytic_inflow(case)
        markers = analysed_markers(case)
        for level in L.LEVELS:
            rel = pathlib.Path(name) / L.level_name(level) / "history.csv"
            r = read_marker_fluxes(work / rel)
            fl = r["fluxes"]
            missing = set(markers) - set(fl)
            if missing:
                raise RuntimeError(f"{work / rel}: no flux column for {sorted(missing)}")
            if reference is not None:
                ref = L.read_history(reference / rel)
                if (r["iterations"], fl["inlet"], fl["outlet"]) != (ref["iterations"], ref["mass_in"], ref["mass_out"]):
                    raise RuntimeError(f"{name} {L.level_name(level)}: inlet, outlet or iteration count differ from {reference / rel}")
            inflow, outflow = split_fluxes({m: fl[m] for m in markers})
            imb = flux_imbalance(inflow, outflow)
            io_in, io_out = split_fluxes({"inlet": fl["inlet"], "outlet": fl["outlet"]})
            io_imb = flux_imbalance(io_in, io_out)
            rows.append({
                "case": name, "mach": f"{case.mach:g}", "level": L.level_name(level),
                "inflow_kg_s": format(inflow, ".10g"), "outflow_kg_s": format(outflow, ".10g"),
                "imbalance_rel": shown(imb), "method": METHOD,
                "source": f"{rel.as_posix()} ({name} {L.level_name(level)})",
                "resolved": str(imb > PRINT_FLOOR),
                "imbalance_inlet_outlet_rel": shown(io_imb),
                "inlet_vs_freestream_rel": format((fl["inlet"] - ref_flux) / ref_flux, ".4g"),
                "marker_fluxes_kg_s": "; ".join(f"{m}={fl[m]:.10g}" for m in markers),
                "residual_drop_orders": format(r["drop"], ".3f"), "stop_reason": r["stop"],
                "_inlet": fl["inlet"], "_imbalance_rel": imb, "_imbalance_inlet_outlet_rel": io_imb})
    return rows


def _trend(rows: list[dict], name: str, column: str) -> list[float]:
    """The unrounded numbers of a column (the hidden '_<column>' key), in level order."""
    return [r["_" + column] for r in rows if r["case"] == name]


def _ratios(values: list[float]) -> str:
    if not all(v > PRINT_FLOOR for v in values):
        return "not defined (a value is at the print floor)"
    return " and ".join(f"{a / b:.2f}" for a, b in zip(values, values[1:]))


def _cone_text(rows: list[dict]) -> str:
    case = L.CASES["cone"]
    rho_u, extent = freestream_flux(case)
    inlets = [r["_inlet"] for r in rows if r["case"] == "cone"]
    full = math.pi * rho_u * extent ** 2
    return (f"Axisymmetric cone: rho_inf U_inf = {rho_u:.6f} kg/(s m^2) and the inlet radius is {extent:g} m, so the freestream through the inlet is "
            f"pi rho U R^2 = {full:.4f} kg/s for a full revolution, rho U R^2 / 2 = {rho_u * extent ** 2 / 2:.4f} kg/s per radian "
            f"and rho U R = {rho_u * extent:.4f} kg/s per unit depth. SU2's Avg_Massflow(inlet) on 65x49, 129x97 and 257x193 is "
            + ", ".join(f"{v:.4f}" for v in inlets)
            + ", i.e. " + ", ".join(f"{v / full:.6f}" for v in inlets)
            + " times the full-revolution value, so SU2 integrates 2 pi r rho u.n: the cone's flux is for the full revolution, "
            "not per radian and not per unit depth. Its inlet differs from pi rho U R^2 only by the quadrature of the r weight, "
            "which falls with the grid (inlet_vs_freestream_rel).")


def header(rows: list[dict], work: pathlib.Path, reference: pathlib.Path | None) -> list[str]:
    home = str(pathlib.Path.home())
    tilde = lambda p: str(p).replace(home, "~", 1)
    lines = [
        "Farfield mass-flux balance of the layer 0 cases: verification of the code against the conservation of mass; no model is compared with another here.",
        f"Producer: python -m nasa_vv.cases.massflux (research/nasa_vv/cases/massflux.py). Work directory: {tilde(work)}, read as it stands (one history.csv per case and level).",
        f"The runs are layer 0's (research/nasa_vv/cases/layer0.py, last commit that changed it: {_last_commit(LAYER0_PY)}): its case, grid and configuration builders with config_text(case, analyze=<every marker>), SU2 8.3.0 (ignisyeet-su2-native), Euler, 65x49, 129x97 and 257x193 nodes.",
        f"Method: {METHOD}. MARKER_ANALYZE lists every boundary marker of the mesh (the lower boundary marker(s), inlet, outlet, top), MARKER_ANALYZE_AVERAGE= MASSFLUX, SURFACE_MASSFLOW in HISTORY_OUTPUT. SU2 reports the flux itself, so no integration of rho u.n from the surface output is used.",
        "Sign convention, read from the data: Avg_Massflow is positive for flux into the domain (the inlet reads positive and the outlet negative). A marker with a net positive flux counts to inflow_kg_s and one with a net negative flux to outflow_kg_s, both written as positive numbers; the flux of a marker is its net, so a marker that passes flux in one part and out in another counts by its net. The balance therefore closes over every marker, the lower boundary included, and nothing is left out.",
        "imbalance_rel = abs(inflow_kg_s - outflow_kg_s) / abs(inflow_kg_s) (nasa_vv.cases.massflux.flux_imbalance). The NASA tutorial calls about 1e-3 relative consistent.",
        "imbalance_inlet_outlet_rel is the same quantity with only the inlet and the outlet; the difference to imbalance_rel is the flux through the top and the lower boundary. marker_fluxes_kg_s lists every analysed marker; the wall, axis and cone markers are slip walls or a symmetry axis and carry no flux by construction.",
        "Units: wedge and pm15 are planar, kg/s per metre of span (the mesh is 2-D). The cone cases are axisymmetric and the flux is for the full revolution (next line).",
        _cone_text(rows),
        "Evidence that Avg_Massflow is an integral over the marker and not a mean: the wedge outlet is about 0.73 m tall and the inlet 1 m, and both read the same flux.",
        f"Precision floor: history.csv prints about 10 significant digits, so a relative imbalance at or below {PRINT_FLOOR:g} is not resolved; resolved is False there, imbalance_rel and imbalance_inlet_outlet_rel are written as '<{PRINT_FLOOR:g}' instead of a number (not 0, not the unresolved difference), and no trend with refinement is claimed for those rows.",
        "inlet_vs_freestream_rel = (Avg_Massflow(inlet) - analytic freestream flux through the inlet) / analytic, with the analytic flux rho_inf U_inf H (planar) or pi rho_inf U_inf R^2 (axisymmetric), rho_inf from p_inf = 101325 Pa, T_inf = 288.15 K and the golden gas constants. It separates the quadrature error of the inlet from the imbalance.",
        "residual_drop_orders and stop_reason are layer 0's (peak minus final log10 rms[Rho]; 'residual target' if the final rms[Rho] is at most -14).",
    ]
    for name in L.CASES:
        imb = _trend(rows, name, "imbalance_rel")
        io = _trend(rows, name, "imbalance_inlet_outlet_rel")
        lines.append(
            f"{name}: imbalance_rel on 65x49, 129x97, 257x193 = {', '.join(shown(v) for v in imb)}"
            + (f" (successive ratios {_ratios(imb)})" if any(v > PRINT_FLOOR for v in imb) else " (all at or below the print floor)")
            + f"; inlet and outlet only = {', '.join(shown(v) for v in io)}.")
    drop = min(float(r["residual_drop_orders"]) for r in rows)
    cone_resolved = all(v > PRINT_FLOOR for n in ("cone", "cone_sym_axis") for v in _trend(rows, n, "imbalance_rel"))
    if cone_resolved:
        half = [b / a for a, b in zip(_trend(rows, "cone", "imbalance_rel"), _trend(rows, "cone_sym_axis", "imbalance_rel"))]
        dev = [float(r["inlet_vs_freestream_rel"]) for r in rows if r["case"] == "cone"]
        ratios = [a / b for n in ("cone", "cone_sym_axis") for a, b in zip(_trend(rows, n, "imbalance_rel"), _trend(rows, n, "imbalance_rel")[1:])]
        lines.append(
            f"Reading: on the planar cases the balance closes to the print floor on every grid; that is what a conservative scheme gives at a converged state "
            f"(every run dropped rms[Rho] by {drop:.1f} orders or more), so it tests the convergence and the flux bookkeeping, not the discretisation error. "
            "On the cone the flux carries the 2 pi r weight; the balance is then probably not an identity of the scheme, and its imbalance is nonzero on every grid. "
            f"It falls with the grid by a factor of {min(ratios):.1f} to {max(ratios):.1f} per refinement from {max(_trend(rows, 'cone', 'imbalance_rel')):.2g} on the coarse grid; "
            "these two ratios per case are consistent with a second-order discretisation error (a ratio of 4), which with three grids is an observation and not an observed order. "
            f"The cone inlet's own deviation from the analytic flux is {', '.join(f'{v:.2g}' for v in dev)} on the three grids, a fraction of the imbalance; the rest is in the outlet. "
            "That the weight is the cause is an interpretation consistent with these numbers, not a proof. "
            f"cone_sym_axis has {min(half):.2f} to {max(half):.2f} times the imbalance of cone on the same grid (axis as MARKER_SYM instead of part of the one Euler wall); that difference is not explained.")
    else:
        lines.append("Reading: not given; a cone imbalance is at the print floor, so there is no ratio to read.")
    over = [(r["case"], r["level"], r["imbalance_rel"]) for r in rows if r["_imbalance_rel"] > TOLERANCE]
    lines.append("Rows above the tutorial's 1e-3: " + (", ".join(f"{c} {lv} {v}" for c, lv, v in over) if over else "none."))
    if reference is not None:
        lines.append(f"Check made while collecting: for every case and level the inlet and outlet fluxes and the last inner iteration equal those of {tilde(reference)} (Task 10's runs, with only the inlet and the outlet analysed), so the extra analysed markers did not change the solve.")
    return lines


def write_csv(rows: list[dict], work: pathlib.Path, out: pathlib.Path, reference: pathlib.Path | None = None) -> pathlib.Path:
    out.parent.mkdir(parents=True, exist_ok=True)
    with out.open("w", newline="") as f:
        for ln in header(rows, work, reference):
            f.write("# " + ln + "\n")
        w = csv.DictWriter(f, fieldnames=COLUMNS, lineterminator="\n", extrasaction="ignore")
        w.writeheader()
        w.writerows(rows)
    return out


def run_all(work: pathlib.Path, su2: pathlib.Path, jobs: int) -> None:
    """SU2 on every case and level of layer 0 with every marker analysed (at most ``jobs`` serial runs at once)."""
    todo = [(c, lv) for lv in reversed(L.LEVELS) for c in L.CASES.values()]
    with ThreadPoolExecutor(max_workers=jobs) as ex:
        futs = [(c.name, lv, ex.submit(L.run_level, c, lv, work, su2, analysed_markers(c))) for c, lv in todo]
        for n, lv, f in futs:
            f.result()
            print(f"ran {n} {L.level_name(lv)}", flush=True)


def main(argv: list[str] | None = None) -> None:
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("--work", type=pathlib.Path, default=DEFAULT_WORK, help="directory of the SU2 runs (never under /tmp)")
    ap.add_argument("--run", action="store_true", help="run SU2 on every case and level first; otherwise read --work as it stands")
    ap.add_argument("--jobs", type=int, default=4, help="SU2 runs at once with --run (serial runs, so at most this many cores)")
    ap.add_argument("--su2", type=pathlib.Path, default=L.SU2_CFD)
    ap.add_argument("--reference", type=pathlib.Path, default=None,
                    help="directory of Task 10's runs: require the same inlet, outlet and iteration count")
    ap.add_argument("--out", type=pathlib.Path, default=RESULTS)
    a = ap.parse_args(argv)
    work = a.work.expanduser().resolve()
    if str(work).startswith("/tmp"):
        sys.exit("--work must not be under /tmp (RAM-backed here)")
    if a.run:
        run_all(work, a.su2, a.jobs)
    ref = a.reference.expanduser().resolve() if a.reference else None
    rows = collect(work, ref)
    print(write_csv(rows, work, a.out, ref))


if __name__ == "__main__":
    main()
