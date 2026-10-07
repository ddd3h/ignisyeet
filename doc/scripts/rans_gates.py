#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
"""Collects the RANS verification gates from the data files in doc/data/cfd/rans_*.csv into
doc/data/cfd/rans_verification.json (value, threshold, pass / fail / not run, provenance).

  rans_gates.py [--data doc/data/cfd]
"""
import argparse
import csv
import json
import os


def rows(path):
    with open(path) as f:
        return list(csv.DictReader(l for l in f if not l.startswith("#")))


def main():
    p = argparse.ArgumentParser()
    p.add_argument("--data", default=os.path.join(os.path.dirname(__file__), "..", "data", "cfd"))
    a = p.parse_args()
    d = os.path.abspath(a.data)
    fp = rows(os.path.join(d, "rans_flatplate.csv"))
    cone = rows(os.path.join(d, "rans_cone_cp.csv"))
    fric = rows(os.path.join(d, "rans_body_friction.csv"))
    plbl = rows(os.path.join(d, "rans_plate_bl.csv"))
    mem = rows(os.path.join(d, "rans_memory.csv"))

    def plate(m):
        r = sorted((x for x in fp if abs(float(x["mach"]) - m) < 1e-9), key=lambda x: int(x["cells"]))
        cf = [float(x["cf_x097"]) for x in r]
        mono = all(b > a_ for a_, b in zip(cf, cf[1:])) or all(b < a_ for a_, b in zip(cf, cf[1:]))
        return r, cf, mono

    g = {}
    r, cf, mono = plate(0.2)
    g["V1_flat_plate_M0.2"] = {
        "quantity": "cf at x = 0.97, finest grid (273x193) vs NASA TMR SA (CFL3D/FUN3D 545x385, 2.7055e-3)",
        "value_percent": float(r[-1]["err_vs_tmr_percent"]), "threshold_percent": 2.0,
        "vs_karman_schoenherr_percent": float(r[-1]["err_vs_correlation_percent"]),
        "grid_convergence": "monotone" if mono else "non-monotone", "cf_by_grid": dict(zip((x["grid"] for x in r), cf)),
        "pass": abs(float(r[-1]["err_vs_tmr_percent"])) <= 2.0 and mono,
        "note": "cf rises for x > 1.5 because of the farfield outflow at the plate end; plate CD not compared",
        "provenance": "rans_flatplate.csv, rans_flatplate_cf.csv (doc/scripts/rans_flatplate.py)"}
    r, cf, mono = plate(2.0)
    g["V2_flat_plate_M2_adiabatic"] = {
        "quantity": "cf at x = 0.97, finest grid (273x193) vs van Driest II (Hopkins-Inouye, r = 0.89)",
        "value_percent": float(r[-1]["err_vs_correlation_percent"]), "threshold_percent": 5.0,
        "grid_convergence": "monotone" if mono else "non-monotone", "cf_by_grid": dict(zip((x["grid"] for x in r), cf)),
        "pass": abs(float(r[-1]["err_vs_correlation_percent"])) <= 5.0 and mono,
        "provenance": "rans_flatplate.csv"}
    g["acceptance_rule_replay_flat_plate"] = {
        "quantity": "plate CD at the iteration the CFD-mode acceptance would stop vs the run to 8 orders",
        "old_rule_percent": {f"M{x['mach']} {x['grid']}": float(x["cd_at_stop_vs_final_percent"]) for x in fp},
        "trend_rule_percent": {f"M{x['mach']} {x['grid']}": float(x["cd_at_trend_stop_vs_final_percent"]) for x in fp},
        "note": "old rule = 3/2 orders + flat window; trend rule = release/v0.4.0 73d22f8 (rans_replay.py)",
        "provenance": "rans_flatplate.csv"}
    whole = next(x for x in cone if x["x_over_lc_from"] == "0.2" and x["x_over_lc_to"] == "0.9")
    worst = max((x for x in cone if x is not whole), key=lambda x: abs(float(x["error_vs_taylor_maccoll_percent"])))
    g["V4_cone_M2_Cp"] = {
        "quantity": "cone surface Cp over x/L_cone 0.2-0.9 vs Taylor-Maccoll (0.10447), 10 deg cone, wall_size 2 mm",
        "mean_percent": float(whole["error_vs_taylor_maccoll_percent"]),
        "worst_bin_percent": float(worst["error_vs_taylor_maccoll_percent"]), "threshold_percent": 2.0,
        "pass": False, "verdict": "marginal fail: mean +1.7 % within 2 %, worst bin +2.2 % (x/L 0.5-0.6)",
        "explanation": "the turbulent displacement-thickness growth adds about 0.13 deg of effective cone angle near x = 0.1 m, "
                       "which raises Cp by roughly 2 %: a viscous Cp above the inviscid Taylor-Maccoll value is expected",
        "convergence": "second-order stage stalled (rms[Rho] -2.2..-5.4, never 3 orders below the reference); "
                       "wall axial force steady (CFx(wall) 0.1299 +- 0.0006); snapshot of iteration 1000",
        "provenance": "rans_cone_cp.csv (doc/scripts/rans_cone.py, rans_geometry.py)"}
    g["V3_friction_vs_flat_plate"] = {
        "quantity": "friction part of the wall axial force vs flat-plate van Driest II over the wetted area, alpha 0",
        "values_percent": {f"{x['geometry']} M{x['mach']} w{float(x['wall_size_m']) * 1000:g}mm": float(x["error_percent"]) for x in fric},
        "threshold_percent": 10.0, "pass": False,
        "yplus_p95": {f"{x['geometry']} M{x['mach']}": float(x["yplus_p95"]) for x in fric}, "yplus_p95_threshold": 3.0,
        "state": "before the boundary-layer mesh redesign: none of the cases converged (stall; the finless body diverges at M 0.8 "
                 "after LIMITER_ITER 400 and at M 2 from about iteration 1000); values from surface snapshots while CFx(wall) was steady",
        "cause": "under-resolved boundary layer: prism layer only 0.4 wall_size thick, then a 12x jump to cells of wall_size + 0.2 d; "
                 "reproduced in 2-D (V3b)",
        "provenance": "rans_body_friction.csv (doc/scripts/rans_friction.py, rans_surface.py)"}
    g["V3b_plate_with_cfd_mode_spacing"] = {
        "quantity": "cf on a 2-D plate with the wall-normal spacing of the CFD-mode meshes vs a resolved grid (sea-level Re, M 0.8 / 2)",
        "rows": [{k: x[k] for k in ("mach", "grid", "state", "err_x0.97_percent") if k in x} for x in plbl],
        "provenance": "rans_plate_bl.csv (doc/scripts/rans_plate_bl.py)"}
    g["V5_herrin_dutton_base_flow"] = {
        "pass": None, "status": "not run",
        "reason": "validation needs the measured approach boundary layer of the M 2.46 cylinder (wind-tunnel afterbody, no nose) "
                  "and a converged base wake; the CFD mode meshes a body with a nose from an STL (no axisymmetric or wedge "
                  "domain), the flat-base wake is where the second-order stage stalls, and the boundary layer is under-resolved (V3)"}
    for k, why in (("V6_sample_mesh_convergence", "held: 5 mm M0.8 alpha 0 diverged after LIMITER_ITER 400; 3 mm cases need about 10 GB "
                    "each and were stopped (OOM kill with concurrent jobs, then held) until the boundary-layer mesh and the stage-B "
                    "numerics are revised; wall_size 3.5 mm does not mesh (1 inverted prism, 18 unmarked faces)"),
                   ("V7_rans_vs_euler", "held with V6"), ("V8_rans_table", "held with V6")):
        g[k] = {"pass": None, "status": "not run", "reason": why}
    g["memory"] = {
        "bytes_per_node": [int(x["bytes_per_node"]) for x in mem],
        "mean_bytes_per_node": round(sum(int(x["bytes_per_node"]) for x in mem) / len(mem)),
        "model": "40 MB per rank + bytes_per_node x nodes (summed over ranks, 3 and 4 ranks)",
        "provenance": "rans_memory.csv"}
    out = {"description": "RANS (SA-neg) verification of the ignisyeet CFD mode; model-vs-model and model-vs-correlation comparisons "
                          "(verification), no measured data", "date": "2026-10-07",
           "tools": "SU2 8.3.0, Open MPI 5.0.11, gmsh 4.15.2", "machine": "i7-12700 (20 threads, 30 GB), shared with other jobs",
           "commits": "flat plate 8f745ae; CLI cases 3acdb89 (release/v0.4.0 73d22f8 merged)", "gates": g}
    path = os.path.join(d, "rans_verification.json")
    open(path, "w").write(json.dumps(out, indent=1) + "\n")
    print(path)


if __name__ == "__main__":
    main()
