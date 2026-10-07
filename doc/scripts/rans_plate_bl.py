#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
"""Flat plate with the wall-normal resolution of the ignisyeet RANS meshes (2-D, structured quads).

The 3-D RANS mesh has a prism layer of total thickness 0.4 wall_size (first height for y+ 1, growth <= 1.3) and,
above it, cells of the size field h = wall_size + 0.2 d (d = wall distance). This script builds the same wall-normal
spacing on a 2-D plate (streamwise spacing = wall_size) and compares the skin friction with the correlation, to
separate the effect of the boundary-layer resolution from the rest of the 3-D setup:

  pipeline_w{W}: layers to 0.4 W, then dy = W + 0.2 y            (what the CFD mode resolves)
  resolved:      layers (same first height, ratio <= 1.15) to 40 mm, then dy = 3 mm + 0.2 y  (reference grid)

Flow: sea-level US1976 at the launch altitude of the sample (T 288.02 K, p 101085 Pa), M 0.8 and 2, plate from
x = 0 to 2.5 m, inflow at x = -0.3 m; farfield on inflow, top (y = 2 m) and outflow. Settings as rans_flatplate.py.

  rans_plate_bl.py run --work DIR [--machs 0.8 2.0] [--walls 0.003 0.005] [--jobs 2] [--ranks 4]
  rans_plate_bl.py collect --work DIR --out CSV
"""
import argparse
import concurrent.futures as cf
import json
import math
import os
import subprocess
import sys
import threading
import time

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import rans_flatplate as fp  # noqa: E402

T_SEA, P_SEA = 288.02, 101084.9684  # freestream of the sample cases (case.cfg)
H1 = 5.715124792918661e-07  # first layer height of the sample RANS meshes (mesh_stats.json)
X_PROBES = [0.25, 0.5, 0.97, 1.5]
PLATE_END = 2.5
LOCK = threading.Lock()  # rans_flatplate keeps the freestream in module globals


def layers(h1, total, rmax):
    ys, h = [0.0], h1
    n = 1
    while True:
        # ratio that reaches `total` with n layers <= rmax
        if h1 * (rmax ** n - 1) / (rmax - 1) >= total:
            break
        n += 1
    lo, hi = 1.0, rmax
    for _ in range(200):
        mid = 0.5 * (lo + hi)
        if h1 * (mid ** n - 1) / (mid - 1) < total:
            lo = mid
        else:
            hi = mid
    r = 0.5 * (lo + hi)
    for k in range(n):
        ys.append(ys[-1] + h1 * r ** k)
    return ys, r


def y_lines(kind, w):
    if kind == "pipeline":
        ys, r = layers(H1, 0.4 * w, 1.3)
        base = w
    else:
        ys, r = layers(H1, 0.04, 1.15)
        base = 0.003
    while ys[-1] < 2.0:
        ys.append(ys[-1] + base + 0.2 * ys[-1])
    ys[-1] = max(ys[-1], 2.0)
    return ys, r


def x_lines(dx):
    xs = [-0.3 + i * dx for i in range(int(round(0.3 / dx)))] + [0.0]
    xs = [x for x in xs if x < -1e-12] + [i * dx for i in range(int(round(PLATE_END / dx)) + 1)]
    return xs


def write_mesh(path, xs, ys):
    ni, nj = len(xs), len(ys)
    X = [x for _ in ys for x in xs]
    Y = [y for y in ys for _ in xs]
    tmp = path + ".p2dfmt"
    with open(tmp, "w") as f:
        f.write(f"1\n{ni} {nj}\n")
        f.write("\n".join(f"{v:.15e}" for v in X) + "\n")
        f.write("\n".join(f"{v:.15e}" for v in Y) + "\n")
    fp.write_su2(tmp, path)
    os.remove(tmp)
    return ni, nj


def set_freestream(mach):
    # rans_flatplate builds the freestream from RE_PER_M and T_INF: Re per metre of the sea-level state
    a = math.sqrt(fp.GAMMA * fp.R_AIR * T_SEA)
    rho = P_SEA / (fp.R_AIR * T_SEA)
    fp.T_INF = T_SEA
    fp.RE_PER_M = rho * mach * a / fp.sutherland(T_SEA)


def run_case(a, kind, w, mach):
    name = f"M{mach:g}_{kind}" + (f"_w{w * 1000:g}mm" if kind == "pipeline" else "")
    d = os.path.abspath(os.path.join(a.work, name))
    os.makedirs(d, exist_ok=True)
    ys, r = y_lines(kind, w)
    xs = x_lines(w if kind == "pipeline" else 0.003)
    ni, nj = write_mesh(os.path.join(d, "mesh.su2"), xs, ys)
    env = dict(os.environ, OMP_NUM_THREADS="1")
    cmd = [os.path.join(a.prefix, "bin", "mpirun"), "-np", str(a.ranks), "--bind-to", "none", os.path.join(a.prefix, "bin", "SU2_CFD")]
    t0 = time.time()
    with LOCK:
        set_freestream(mach)
        re_m = fp.RE_PER_M
        open(os.path.join(d, "startup.cfg"), "w").write(fp.config(mach, True))
    subprocess.run(cmd + ["startup.cfg"], cwd=d, env=env, stdout=open(os.path.join(d, "log_startup.txt"), "w"), stderr=subprocess.STDOUT, check=True)
    head, data = fp.read_history(os.path.join(d, "history_startup.csv"))
    peak = max(x[head.index("rms[Rho]")] for x in data[1:])
    with LOCK:
        set_freestream(mach)
        open(os.path.join(d, "final.cfg"), "w").write(fp.config(mach, False, peak - a.drop))
        open(os.path.join(d, "post.cfg"), "w").write("")
        post_cfg = fp.config(mach, False, post=True).replace("RESTART_FILENAME= restart.dat", "RESTART_FILENAME= restart_post.dat")
    subprocess.run(cmd + ["final.cfg"], cwd=d, env=env, stdout=open(os.path.join(d, "log_final.txt"), "w"), stderr=subprocess.STDOUT, check=True)
    open(os.path.join(d, "post.cfg"), "w").write(post_cfg)
    subprocess.run(cmd + ["post.cfg"], cwd=d, env=env, stdout=open(os.path.join(d, "log_post.txt"), "w"), stderr=subprocess.STDOUT, check=True)
    meta = {"kind": kind, "wall_size": w, "mach": mach, "ni": ni, "nj": nj, "layers": sum(1 for y in ys if y <= (0.4 * w if kind == "pipeline" else 0.04) + 1e-12) - 1,
            "ratio": r, "cells_below_20mm": sum(1 for y in ys if 0 < y <= 0.02), "re_per_m": re_m, "rho_peak": peak,
            "wall_s": time.time() - t0, "ranks": a.ranks}
    json.dump(meta, open(os.path.join(d, "meta.json"), "w"))
    return d


def collect(a):
    rows = []
    for name in sorted(os.listdir(a.work)):
        d = os.path.join(a.work, name)
        if not os.path.exists(os.path.join(d, "meta.json")):
            if os.path.exists(os.path.join(d, "log_final.txt")) and "diverged" in open(os.path.join(d, "log_final.txt"), errors="replace").read():
                head, data = fp.read_history(os.path.join(d, "history.csv"))
                rows.append({"mach": float(name.split("_")[0][1:]), "grid": name.split("_", 1)[1].replace("pipeline_", "pipeline_"),
                             "state": f"SU2 diverged at iteration {len(data)}"})
            continue
        m = json.load(open(os.path.join(d, "meta.json")))
        set_freestream(m["mach"])
        pts = fp.surface_cf(d)
        head, data = fp.read_history(os.path.join(d, "history.csv"))
        row = {"mach": m["mach"], "grid": m["kind"] if m["kind"] != "pipeline" else f"pipeline_w{m['wall_size'] * 1000:g}mm",
               "re_per_m": m["re_per_m"], "prism_layers": m["layers"], "cells_below_20mm": m["cells_below_20mm"],
               "iterations": len(data), "rho_drop_orders": m["rho_peak"] - data[-1][head.index("rms[Rho]")]}
        row["state"] = "converged (8 orders)" if row["rho_drop_orders"] >= 7.99 else "not converged (stopped)"
        for x in X_PROBES:
            cfx = fp.interp(pts, x)
            ref = fp.van_driest2(m["mach"], fp.RE_PER_M * x, t_inf=T_SEA)[0]
            row[f"cf_x{x:g}"] = cfx
            row[f"err_x{x:g}_percent"] = 100 * (cfx / ref - 1)
        rows.append(row)
    keys = []
    for r in rows:
        keys += [k for k in r if k not in keys]
    with open(a.out, "w") as f:
        f.write("# Flat plate with the wall-normal spacing of the ignisyeet RANS meshes (doc/scripts/rans_plate_bl.py): "
                "pipeline_wW = prism layers (first height 5.72e-7 m, growth <= 1.3) to 0.4 W, then dy = W + 0.2 y and dx = W; "
                "resolved = layers (growth <= 1.15) to 40 mm, then dy = 3 mm + 0.2 y, dx = 3 mm. Sea-level freestream of the "
                "sample cases (T 288.02 K, p 101085 Pa), plate 0..2.5 m, SU2 settings of the CFD mode, runs to 8 orders below "
                "the startup peak. err_x*: cf vs van Driest II / Karman-Schoenherr (adiabatic, r 0.89) at Re_x. "
                f"cells_below_20mm: cells within 20 mm of the wall (boundary layer about 20-30 mm at x = 1 m). {a.provenance}\n")
        f.write(",".join(keys) + "\n")
        for r in rows:
            f.write(",".join(f"{r[k]:.6g}" if isinstance(r.get(k), float) else str(r.get(k, "")) for k in keys) + "\n")
    print(open(a.out).read())


def main():
    p = argparse.ArgumentParser()
    p.add_argument("cmd", choices=["run", "collect"])
    p.add_argument("--work", required=True)
    p.add_argument("--machs", type=float, nargs="+", default=[0.8, 2.0])
    p.add_argument("--walls", type=float, nargs="+", default=[0.003, 0.005])
    p.add_argument("--resolved", action="store_true", default=True)
    p.add_argument("--jobs", type=int, default=2)
    p.add_argument("--ranks", type=int, default=4)
    p.add_argument("--drop", type=float, default=8.0)
    p.add_argument("--prefix", default=os.environ.get("IGNISYEET_CFD_PREFIX", ""))
    p.add_argument("--out", default="rans_plate_bl.csv")
    p.add_argument("--provenance", default="")
    a = p.parse_args()
    if a.cmd == "collect":
        return collect(a)
    jobs = [(k, w, m) for m in a.machs for (k, w) in [("pipeline", w) for w in a.walls] + [("resolved", 0.003)]]
    with cf.ThreadPoolExecutor(a.jobs) as ex:
        futs = {ex.submit(run_case, a, *j): j for j in jobs}
        for fu in cf.as_completed(futs):
            try:
                print("done", futs[fu], fu.result(), flush=True)
            except Exception as e:  # noqa: BLE001
                print("FAILED", futs[fu], e, flush=True)


if __name__ == "__main__":
    main()
