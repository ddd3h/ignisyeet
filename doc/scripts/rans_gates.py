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
    f = lambda name: os.path.join(d, name)
    fp = rows(f("rans_flatplate.csv"))
    cone = rows(f("rans_cone_cp.csv"))
    fric = rows(f("rans_body_friction.csv"))
    mem = rows(f("rans_memory.csv"))

    def plate(data, m):
        r = sorted((x for x in data if abs(float(x["mach"]) - m) < 1e-9), key=lambda x: int(x["cells"]))
        cf = [float(x["cf_x097"]) for x in r]
        mono = all(b > a_ for a_, b in zip(cf, cf[1:])) or all(b < a_ for a_, b in zip(cf, cf[1:]))
        return r, cf, mono

    g = {}
    r, cf, mono = plate(fp, 0.2)
    g["V1_flat_plate_M0.2"] = {
        "quantity": "cf at x = 0.97, finest grid (273x193) vs NASA TMR SA (CFL3D/FUN3D 545x385, 2.7055e-3)",
        "value_percent": float(r[-1]["err_vs_tmr_percent"]), "threshold_percent": 2.0,
        "vs_karman_schoenherr_percent": float(r[-1]["err_vs_correlation_percent"]),
        "grid_convergence": "monotone" if mono else "non-monotone", "cf_by_grid": dict(zip((x["grid"] for x in r), cf)),
        "pass": abs(float(r[-1]["err_vs_tmr_percent"])) <= 2.0 and mono,
        "note": "cf rises for x > 1.5 because of the farfield outflow at the plate end; plate CD not compared",
        "provenance": "rans_flatplate.csv, rans_flatplate_cf.csv (doc/scripts/rans_flatplate.py; SU2 settings of the CFD mode, header names the commit)"}
    r, cf, mono = plate(fp, 2.0)
    g["V2_flat_plate_M2_adiabatic"] = {
        "quantity": "cf at x = 0.97, finest grid (273x193) vs van Driest II (Hopkins-Inouye, r = 0.89)",
        "value_percent": float(r[-1]["err_vs_correlation_percent"]), "threshold_percent": 5.0,
        "grid_convergence": "monotone" if mono else "non-monotone", "cf_by_grid": dict(zip((x["grid"] for x in r), cf)),
        "pass": abs(float(r[-1]["err_vs_correlation_percent"])) <= 5.0 and mono,
        "note": "137x97 stopped at the 20000-iteration cap 5.8 orders below the startup peak (the others 8 orders)",
        "provenance": "rans_flatplate.csv"}
    g["acceptance_rule_replay_flat_plate"] = {
        "quantity": "plate CD at the iteration an acceptance rule would stop vs the run to 8 orders",
        "old_rule_percent": {f"M{x['mach']} {x['grid']}": float(x["cd_at_stop_vs_final_percent"]) for x in fp},
        "trend_rule_percent": {f"M{x['mach']} {x['grid']}": float(x["cd_at_trend_stop_vs_final_percent"]) for x in fp},
        "note": "old rule = 3/2 orders + flat window; trend rule = 73d22f8 (rans_replay.py); the shipped rule adds the worst-of-20-rows drop",
        "provenance": "rans_flatplate.csv"}
    whole = next(x for x in cone if x["x_over_lc_from"] == "0.2" and x["x_over_lc_to"] == "0.9")
    bins = [x for x in cone if x is not whole]
    worst = max(bins, key=lambda x: abs(float(x["error_vs_taylor_maccoll_percent"])))
    g["V4_cone_M2_Cp"] = {
        "quantity": "cone surface Cp over x/L_cone 0.2-0.9 vs Taylor-Maccoll (0.10447), 10 deg cone, wall_size 2 mm, converged",
        "mean_percent": float(whole["error_vs_taylor_maccoll_percent"]),
        "worst_bin_percent": float(worst["error_vs_taylor_maccoll_percent"]),
        "worst_bin": f"{worst['x_over_lc_from']}-{worst['x_over_lc_to']}", "threshold_percent": 2.0,
        "bins_0.3_to_0.9_max_percent": max(float(x["error_vs_taylor_maccoll_percent"]) for x in bins if float(x["x_over_lc_from"]) >= 0.29),
        "pass": all(abs(float(x["error_vs_taylor_maccoll_percent"])) <= 2.0 for x in bins),
        "verdict": "mean within 2 %; only the bin nearest the tip exceeds it",
        "explanation": "viscous Cp above the inviscid Taylor-Maccoll value is expected: the turbulent displacement thickness adds "
                       "about 0.1 deg of effective cone angle (roughly +2 % in Cp), largest relative to the radius near the tip",
        "provenance": "rans_cone_cp.csv (doc/scripts/rans_cone.py, rans_geometry.py)"}
    g["V3_friction_vs_flat_plate"] = {
        "quantity": "friction part of the wall axial force (surface integration) vs van Driest II over the wetted area, alpha 0, converged cases",
        "values_percent": {f"{x['geometry']} M{x['mach']} w{float(x['wall_size_m']) * 1000:g}mm": float(x["error_percent"]) for x in fric},
        "threshold_percent": 10.0, "pass": all(abs(float(x["error_percent"])) <= 10.0 for x in fric),
        "yplus_p95": {f"{x['geometry']} M{x['mach']}": float(x["yplus_p95"]) for x in fric}, "yplus_p95_threshold": 3.0,
        "note": "cone estimate with the Taylor-Maccoll edge state and the turbulent Mangler rule (plain freestream law: +11.3 %)",
        "provenance": "rans_body_friction.csv, rans_body_cases.csv (doc/scripts/rans_friction.py, rans_surface.py)"}
    if os.path.exists(f("rans_before_body_friction.csv")):
        bf = rows(f("rans_before_body_friction.csv"))
        bp = rows(f("rans_before_plate_bl.csv"))
        g["before_mesh_fix"] = {
            "state": "prism layer 0.4 wall_size, Venkatakrishnan-Wang limiter frozen at iteration 400 (8f745ae / 3acdb89)",
            "friction_percent": {f"{x['geometry']} M{x['mach']}": float(x["error_percent"]) for x in bf},
            "plate_with_cfd_mode_spacing": [{k: x[k] for k in ("mach", "grid", "state", "err_x0.97_percent") if k in x} for x in bp],
            "provenance": "rans_before_*.csv"}
    g["V5_herrin_dutton_base_flow"] = {
        "pass": None, "status": "not run",
        "reason": "validation needs the measured approach boundary layer of the M 2.46 cylinder (wind-tunnel afterbody, no nose); "
                  "the CFD mode meshes a body with a nose from an STL and has no axisymmetric or wedge domain"}
    for key, fname in (("V6_sample_mesh_convergence", "rans_mesh_study.json"), ("V7_rans_vs_euler", "rans_vs_euler.json"),
                       ("V8_rans_table", "rans_table_run.json")):
        if os.path.exists(f(fname)):
            g[key] = json.load(open(f(fname)))
        else:
            g[key] = {"pass": None, "status": "not run", "reason": "waiting for the RANS startup retry (8 mm M 2 startup diverged on 5ce5795)"}
    g["memory"] = {
        "bytes_per_node": [int(x["bytes_per_node"]) for x in mem],
        "mean_bytes_per_node": round(sum(int(x["bytes_per_node"]) for x in mem) / len(mem)),
        "model": "40 MB per rank + bytes_per_node x nodes (summed over ranks)",
        "provenance": "rans_memory.csv, rans_before_memory.csv, rans_mesh_sweep.csv (mesher)"}
    out = {"description": "RANS (SA-neg) verification of the ignisyeet CFD mode; model-vs-model and model-vs-correlation comparisons "
                          "(verification), no measured data", "date": "2026-10-07",
           "tools": "SU2 8.3.0, Open MPI 5.0.11, gmsh 4.15.2", "machine": "i7-12700 (20 threads, 30 GB), shared with other jobs",
           "commits": "each data file header names its commit", "gates": g}
    path = os.path.join(d, "rans_verification.json")
    open(path, "w").write(json.dumps(out, indent=1) + "\n")
    print(path)


if __name__ == "__main__":
    main()
