#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
"""RANS verification: turbulent flat plate (NASA TMR 2DZP grid family) at M 0.2 and M 2 with the SU2 settings of
the ignisyeet RANS mode (crates/cfd/src/su2cfg.rs: SA-neg with WITHFT2, Roe, first-order startup stage then MUSCL
with the Venkatakrishnan-Wang limiter, Green-Gauss / weighted-least-squares gradients, FGMRES + ILU(0) at relative
tolerance 0.05, adaptive CFL 5 .. 200, adiabatic wall without wall function, FREESTREAM_NU_FACTOR 3).

Grids: the TMR PLOT3D files flatplate_clust2_*.p2dfmt (x in [-0.33333, 2], y in [0, 1], plate from x = 0), written
as 2-D SU2 quad meshes. Boundary conditions: plate = adiabatic no-slip wall, x < 0 on y = 0 = symmetry, inflow,
top and outflow = characteristic farfield (MARKER_FAR, the boundary of the ignisyeet meshes).

Freestream: Re = 5e6 per metre, T = 300 K (TD_CONDITIONS, pressure from Re, Sutherland viscosity).

References at x = 0.97:
  * M 0.2: SA value of CFL3D and FUN3D on the 545x385 grid of the TMR (cf_convergence.dat), and the local
    Karman-Schoenherr skin friction (derivative of Re_x CF of the implicit K-S law 0.242/sqrt(CF) = log10(Re_x CF)).
  * M 2 adiabatic: van Driest II (Hopkins-Inouye form, recovery factor 0.89) with the local K-S law as the
    incompressible law.

Usage:  rans_flatplate.py run --grids DIR --work DIR [--machs 0.2 2.0] [--levels 69 137 273] [--ranks 4] [--jobs 3]
        rans_flatplate.py collect --work DIR --out doc/data/cfd/rans_flatplate.csv
"""
import argparse
import concurrent.futures as cf
import csv
import datetime
import json
import math
import os
import resource
import subprocess
import sys
import time

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

GAMMA = 1.4
R_AIR = 287.058
RE_PER_M = 5.0e6
T_INF = 300.0
X_PROBE = 0.97
RECOVERY = 0.89
# TMR SA reference (cf_convergence.dat, 545x385 grid): CFL3D 0.270562e-2, FUN3D 0.270540e-2.
TMR_SA_CF_097 = {"CFL3D": 0.270562153e-2, "FUN3D": 0.270540473e-2}
LEVELS = {35: "flatplate_clust2_4levelsdown_35x25.p2dfmt", 69: "flatplate_clust2_3levelsdown_69x49.p2dfmt",
          137: "flatplate_clust2_2levelsdown_137x97.p2dfmt", 273: "flatplate_clust2_1leveldown_273x193.p2dfmt",
          545: "flatplate_clust2.p2dfmt"}
STARTUP_MAX = 500
STARTUP_DROP = 2.0
FINAL_ITER = 20000
FINAL_DROP = 8.0  # orders below the startup residual peak (rho), stop criterion of the verification runs


def sutherland(t):
    return 1.716e-5 * (t / 273.15) ** 1.5 * (273.15 + 110.4) / (t + 110.4)


def freestream(mach):
    a = math.sqrt(GAMMA * R_AIR * T_INF)
    u = mach * a
    mu = sutherland(T_INF)
    rho = RE_PER_M * mu / u
    return {"u": u, "mu": mu, "rho": rho, "p": rho * R_AIR * T_INF}


def ks_mean(rex):
    """Karman-Schoenherr mean CF at Re_x (0.242 / sqrt(CF) = log10(Re_x CF)), Newton on y = 1/sqrt(CF)."""
    y = 15.0
    for _ in range(100):
        f = 0.242 * y - math.log10(rex) + 2.0 * math.log10(y)
        df = 0.242 + 2.0 / (y * math.log(10.0))
        y -= f / df
    return 1.0 / (y * y)


def ks_local(rex):
    """Local cf = d(Re_x CF)/d Re_x of the Karman-Schoenherr law (central difference in ln Re_x)."""
    h = 1e-4
    r1, r2 = rex * math.exp(-h), rex * math.exp(h)
    return (r2 * ks_mean(r2) - r1 * ks_mean(r1)) / (r2 - r1)


def white_local(rex):
    return 0.455 / math.log(0.06 * rex) ** 2


def van_driest2(mach, rex, tw_te=None, r=RECOVERY, t_inf=T_INF):
    """van Driest II local cf (Hopkins-Inouye): cf = cf_inc(F_Rx Re_x) / F_c, cf_inc = local Karman-Schoenherr."""
    m = (GAMMA - 1.0) / 2.0 * mach * mach
    taw = 1.0 + r * m
    tw = taw if tw_te is None else tw_te
    a = math.sqrt(r * m / tw)
    b = taw / tw - 1.0
    den = math.sqrt(b * b + 4.0 * a * a)
    al = (2.0 * a * a - b) / den
    be = b / den
    fc = r * m / (math.asin(al) + math.asin(be)) ** 2
    frt = sutherland(t_inf) / sutherland(t_inf * tw)
    frx = frt / fc
    return ks_local(frx * rex) / fc, {"Fc": fc, "F_Rtheta": frt, "F_Rx": frx, "Tw_Te": tw}


def read_p2d(path):
    with open(path) as f:
        tok = f.read().split()
    nb = int(tok[0])
    assert nb == 1
    ni, nj = int(tok[1]), int(tok[2])
    n = ni * nj
    vals = [float(t) for t in tok[3:3 + 2 * n]]
    return ni, nj, vals[:n], vals[n:]


def write_su2(p2d, out):
    ni, nj, xs, ys = read_p2d(p2d)
    idx = lambda i, j: j * ni + i
    with open(out, "w") as f:
        f.write("NDIME= 2\n")
        f.write(f"NELEM= {(ni - 1) * (nj - 1)}\n")
        e = 0
        for j in range(nj - 1):
            for i in range(ni - 1):
                f.write(f"9\t{idx(i, j)}\t{idx(i + 1, j)}\t{idx(i + 1, j + 1)}\t{idx(i, j + 1)}\t{e}\n")
                e += 1
        f.write(f"NPOIN= {ni * nj}\n")
        for k in range(ni * nj):
            f.write(f"{xs[k]:.15e}\t{ys[k]:.15e}\t{k}\n")
        bottom = [(idx(i, 0), idx(i + 1, 0)) for i in range(ni - 1)]
        wall = [s for s in bottom if xs[s[0]] >= -1e-12 and xs[s[1]] >= -1e-12]
        sym = [s for s in bottom if s not in wall]
        marks = {
            "wall": wall,
            "symmetry": sym,
            "inlet": [(idx(0, j + 1), idx(0, j)) for j in range(nj - 1)],
            "outlet": [(idx(ni - 1, j), idx(ni - 1, j + 1)) for j in range(nj - 1)],
            "top": [(idx(i + 1, nj - 1), idx(i, nj - 1)) for i in range(ni - 1)],
        }
        f.write(f"NMARK= {len(marks)}\n")
        for name, segs in marks.items():
            f.write(f"MARKER_TAG= {name}\nMARKER_ELEMS= {len(segs)}\n")
            for a, b in segs:
                f.write(f"3\t{a}\t{b}\n")
    return ni, nj


def config(mach, startup, minval=None, post=False):
    fs = freestream(mach)
    c = [
        "SOLVER= RANS", "KIND_TURB_MODEL= SA", "SA_OPTIONS= ( NEGATIVE, WITHFT2 )", "MATH_PROBLEM= DIRECT",
        "SYSTEM_MEASUREMENTS= SI", f"RESTART_SOL= {'NO' if startup else 'YES'}",
        f"MACH_NUMBER= {mach:.6f}", "AOA= 0.0", "SIDESLIP_ANGLE= 0.0", "FLUID_MODEL= IDEAL_GAS",
        f"GAMMA_VALUE= {GAMMA}", f"GAS_CONSTANT= {R_AIR:.5f}", "INIT_OPTION= TD_CONDITIONS",
        "FREESTREAM_OPTION= TEMPERATURE_FS", f"FREESTREAM_PRESSURE= {fs['p']:.4f}", f"FREESTREAM_TEMPERATURE= {T_INF:.4f}",
        "VISCOSITY_MODEL= SUTHERLAND", "MU_REF= 1.716E-5", "MU_T_REF= 273.15", "SUTHERLAND_CONSTANT= 110.4",
        "FREESTREAM_NU_FACTOR= 3.0", "REF_DIMENSIONALIZATION= DIMENSIONAL",
        "REF_ORIGIN_MOMENT_X= 0.0", "REF_ORIGIN_MOMENT_Y= 0.0", "REF_ORIGIN_MOMENT_Z= 0.0", "REF_LENGTH= 1.0",
        "REF_AREA= 2.0",
        "MARKER_HEATFLUX= ( wall, 0.0 )", "MARKER_MONITORING= ( wall )", "MARKER_FAR= ( inlet, top, outlet )",
        "MARKER_SYM= ( symmetry )", "MARKER_PLOTTING= ( wall )",
        "NUM_METHOD_GRAD= GREEN_GAUSS", "NUM_METHOD_GRAD_RECON= WEIGHTED_LEAST_SQUARES", "CONV_NUM_METHOD_FLOW= ROE",
    ]
    if startup:
        c += ["MUSCL_FLOW= NO"]
    else:
        c += ["MUSCL_FLOW= YES", "SLOPE_LIMITER_FLOW= VENKATAKRISHNAN_WANG", "VENKAT_LIMITER_COEFF= 0.05",
              "LIMITER_ITER= 400"]
    c += [
        "USE_ACCURATE_FLUX_JACOBIANS= YES", "TIME_DISCRE_FLOW= EULER_IMPLICIT", "CONV_NUM_METHOD_TURB= SCALAR_UPWIND",
        "MUSCL_TURB= NO", "TIME_DISCRE_TURB= EULER_IMPLICIT", "CFL_NUMBER= 5.0", "CFL_ADAPT= YES",
        "CFL_ADAPT_PARAM= ( 0.5, 1.2, 1.000, 200.0 )", "LINEAR_SOLVER= FGMRES", "LINEAR_SOLVER_PREC= ILU",
        "LINEAR_SOLVER_ILU_FILL_IN= 0", "LINEAR_SOLVER_ERROR= 0.05", "LINEAR_SOLVER_ITER= 25",
    ]
    if post:
        c += ["ITER= 1", "CONV_FIELD= RMS_DENSITY", "CONV_RESIDUAL_MINVAL= -30", "SOLUTION_FILENAME= restart.dat",
              "CONV_FILENAME= history_post"]
    elif startup:
        c += [f"ITER= {STARTUP_MAX}", "CONV_FIELD= REL_RMS_DENSITY", f"CONV_RESIDUAL_MINVAL= -{STARTUP_DROP}",
              "CONV_STARTITER= 100", "CONV_FILENAME= history_startup"]
    else:
        c += [f"ITER= {FINAL_ITER}", "CONV_FIELD= RMS_DENSITY", f"CONV_RESIDUAL_MINVAL= {minval:.3f}",
              "CONV_STARTITER= 10", "SOLUTION_FILENAME= restart.dat", "CONV_FILENAME= history"]
    c += [
        "MESH_FILENAME= mesh.su2", "MESH_FORMAT= SU2", "RESTART_FILENAME= restart.dat", "SURFACE_FILENAME= surface_flow",
        "TABULAR_FORMAT= CSV", "OUTPUT_FILES= ( RESTART, SURFACE_CSV, SURFACE_PARAVIEW_ASCII )", "VOLUME_OUTPUT= ( COORDINATES, SOLUTION, PRIMITIVE )",
        "HISTORY_OUTPUT= ( ITER, RMS_RES, REL_RMS_RES, AERO_COEFF, LINSOL, CFL_NUMBER )",
        "SCREEN_OUTPUT= ( INNER_ITER, RMS_DENSITY, RMS_NU_TILDE, DRAG, AVG_CFL )",
        "SCREEN_WRT_FREQ_INNER= 50", "HISTORY_WRT_FREQ_INNER= 1", "OUTPUT_WRT_FREQ= 5000",
    ]
    return "\n".join(c) + "\n"


def read_history(path):
    with open(path) as f:
        rows = list(csv.reader(f))
    head = [h.strip().strip('"') for h in rows[0]]
    data = [[float(v) for v in r] for r in rows[1:] if r]
    return head, data


def run_case(args, level, mach):
    d = os.path.abspath(os.path.join(args.work, f"M{mach:g}_{level}"))
    os.makedirs(d, exist_ok=True)
    ni, nj = write_su2(os.path.join(args.grids, LEVELS[level]), os.path.join(d, "mesh.su2"))
    prefix = args.prefix
    env = dict(os.environ, OMP_NUM_THREADS="1")
    cmd = [os.path.join(prefix, "bin", "mpirun"), "-np", str(args.ranks), "--bind-to", "none",
           os.path.join(prefix, "bin", "SU2_CFD")]
    t0 = time.time()
    with open(os.path.join(d, "startup.cfg"), "w") as f:
        f.write(config(mach, True))
    subprocess.run(["/usr/bin/time", "-v", "-o", os.path.join(d, "time_startup.txt")] + cmd + ["startup.cfg"],
                   cwd=d, env=env, stdout=open(os.path.join(d, "log_startup.txt"), "w"), stderr=subprocess.STDOUT, check=True)
    head, data = read_history(os.path.join(d, "history_startup.csv"))
    peak = max(r[head.index("rms[Rho]")] for r in data[1:])  # iteration 0: uniform field, residual ~0
    with open(os.path.join(d, "final.cfg"), "w") as f:
        f.write(config(mach, False, peak - FINAL_DROP))
    subprocess.run(["/usr/bin/time", "-v", "-o", os.path.join(d, "time_final.txt")] + cmd + ["final.cfg"],
                   cwd=d, env=env, stdout=open(os.path.join(d, "log_final.txt"), "w"), stderr=subprocess.STDOUT, check=True)
    post_case(args, d, mach)
    meta = {"ni": ni, "nj": nj, "cells": (ni - 1) * (nj - 1), "mach": mach, "level": level, "wall_s": time.time() - t0,
            "startup_iters": len(data), "rho_peak": peak, "ranks": args.ranks}
    with open(os.path.join(d, "meta.json"), "w") as f:
        json.dump(meta, f)
    return d


def post_case(args, d, mach):
    """One more iteration from the converged restart (restart written to restart_post.dat) that writes the surface
    file with the primitive variables (Skin_Friction_Coefficient, Y_Plus)."""
    cfg = config(mach, False, post=True).replace("RESTART_FILENAME= restart.dat", "RESTART_FILENAME= restart_post.dat")
    with open(os.path.join(d, "post.cfg"), "w") as f:
        f.write(cfg)
    cmd = [os.path.join(args.prefix, "bin", "mpirun"), "-np", str(args.ranks), "--bind-to", "none",
           os.path.join(args.prefix, "bin", "SU2_CFD"), "post.cfg"]
    subprocess.run(cmd, cwd=d, stdout=open(os.path.join(d, "log_post.txt"), "w"), stderr=subprocess.STDOUT, check=True)


def read_vtk(path):
    """Points and point data (SCALARS / VECTORS) of an ASCII legacy VTK file written by SU2."""
    tok = open(path).read().split()
    i = tok.index("POINTS")
    n = int(tok[i + 1])
    vals = tok[i + 3:i + 3 + 3 * n]
    pts = [tuple(float(v) for v in vals[3 * k:3 * k + 3]) for k in range(n)]
    fields = {}
    i = tok.index("POINT_DATA")
    k = i + 2
    while k < len(tok):
        if tok[k] == "SCALARS":
            name = tok[k + 1]
            k += 4
            if tok[k] == "LOOKUP_TABLE":
                k += 2
            fields[name] = [float(v) for v in tok[k:k + n]]
            k += n
        elif tok[k] == "VECTORS":
            name = tok[k + 1]
            k += 3
            fields[name] = [tuple(float(v) for v in tok[k + 3 * j:k + 3 * j + 3]) for j in range(n)]
            k += 3 * n
        else:
            k += 1
    return pts, fields


def surface_cf(d):
    """(x, cf_x, y+) of the wall nodes, sorted by x (surface_flow.vtk holds only the wall marker)."""
    pts, f = read_vtk(os.path.join(d, "surface_flow.vtk"))
    return sorted((p[0], c[0], y) for p, c, y in zip(pts, f["Skin_Friction_Coefficient"], f["Y_Plus"]))


def interp(pts, x, k=1):
    for a, b in zip(pts, pts[1:]):
        if a[0] <= x <= b[0]:
            w = (x - a[0]) / (b[0] - a[0])
            return a[k] + w * (b[k] - a[k])
    raise ValueError(x)


def pipeline_stop(head, data, rho_peak, nu_peak, min_iter=400, window=200, drift=0.005):
    """First final-stage iteration at which the ignisyeet RANS acceptance would stop the case (rho 3 and nu 2 orders
    below the startup peaks, CD flat over the last `window` rows: half-window drift and inter-quartile range at most
    `drift` of |mean| + 1e-5), and the relative CD difference to the last iteration [percent]."""
    ir, inu, icd = head.index("rms[Rho]"), head.index("rms[nu]"), head.index("CD")
    for i in range(max(min_iter, window) - 1, len(data)):
        if rho_peak - data[i][ir] < 3.0 or nu_peak - data[i][inu] < 2.0:
            continue
        w = [r[icd] for r in data[i + 1 - window:i + 1]]
        m = sum(w) / len(w)
        d = abs(sum(w[window // 2:]) / (window - window // 2) - sum(w[:window // 2]) / (window // 2))
        q = sorted(w)
        iqr = q[3 * len(q) // 4] - q[len(q) // 4]
        tol = drift * abs(m) + 1e-5
        if d <= tol and iqr <= tol:
            return i + 1, 100.0 * (data[i][icd] / data[-1][icd] - 1.0)
    return None, None


def maxrss_mb(path):
    if not os.path.exists(path):
        return float("nan")
    for line in open(path):
        if "Maximum resident set size" in line:
            return int(line.split(":")[1]) / 1024.0
    return float("nan")


def collect(args):
    out = []
    for name in sorted(os.listdir(args.work)):
        d = os.path.join(args.work, name)
        if not os.path.exists(os.path.join(d, "meta.json")):
            continue
        meta = json.load(open(os.path.join(d, "meta.json")))
        head, data = read_history(os.path.join(d, "history.csv"))
        last = data[-1]
        rho = last[head.index("rms[Rho]")]
        nu = last[head.index("rms[nu]")]
        _, sdata = read_history(os.path.join(d, "history_startup.csv"))
        hs, _ = read_history(os.path.join(d, "history_startup.csv"))
        nu_peak = max(r[hs.index("rms[nu]")] for r in sdata[1:])
        stop_it, stop_dcd = pipeline_stop(head, data, meta["rho_peak"], nu_peak)
        import rans_replay
        hh = ["CMy" if x == "CMz" else x for x in head]  # 2-D: the pitching moment is CMz
        _, new_it = rans_replay.stops(hh, data, meta["rho_peak"], nu_peak, 1.0)
        new_dcd = 100.0 * (data[new_it - 1][head.index("CD")] / data[-1][head.index("CD")] - 1.0) if new_it else None
        pts = surface_cf(d)
        cfx = interp(pts, X_PROBE)
        yp = [p[2] for p in pts if p[0] > 0.01]
        mach = meta["mach"]
        rex = RE_PER_M * X_PROBE
        if mach < 1:
            ref = 0.5 * (TMR_SA_CF_097["CFL3D"] + TMR_SA_CF_097["FUN3D"])
            corr = ks_local(rex)
        else:
            ref = float("nan")
            corr = van_driest2(mach, rex)[0]
        out.append({
            "mach": mach, "grid": f"{meta['ni']}x{meta['nj']}", "cells": meta["cells"],
            "h": (1.0 / meta["cells"]) ** 0.5, "cf_x097": cfx,
            "tmr_sa_cf": ref, "err_vs_tmr_percent": 100 * (cfx / ref - 1) if ref == ref else float("nan"),
            "correlation_cf": corr, "err_vs_correlation_percent": 100 * (cfx / corr - 1),
            "cd_plate": last[head.index("CD")], "yplus_max": max(yp), "yplus_mean": sum(yp) / len(yp),
            "iterations_final": len(data), "iterations_startup": meta["startup_iters"],
            "rho_drop_orders": meta["rho_peak"] - rho, "nu_drop_orders": nu_peak - nu,
            "pipeline_stop_iter": stop_it, "cd_at_stop_vs_final_percent": stop_dcd,
            "trend_rule_stop_iter": new_it, "cd_at_trend_stop_vs_final_percent": new_dcd,
            "wall_s": meta["wall_s"], "ranks": meta["ranks"],
            "peak_rss_per_rank_mb": max(maxrss_mb(os.path.join(d, "time_startup.txt")), maxrss_mb(os.path.join(d, "time_final.txt"))),
        })
    out.sort(key=lambda r: (r["mach"], r["cells"]))
    keys = list(out[0].keys())
    vd, vdp = van_driest2(2.0, RE_PER_M * X_PROBE)
    hdr = (f"# RANS flat plate verification (doc/scripts/rans_flatplate.py), NASA TMR 2DZP grids, Re = 5e6 per m, "
           f"T = 300 K, adiabatic wall, SU2 settings of the ignisyeet RANS mode (SA-neg + WITHFT2, Roe, 1st-order startup "
           f"then MUSCL/VW, CFL 5..200). cf_x097 = SU2 Skin_Friction_Coefficient_x at x = 0.97 (linear interpolation). "
           f"tmr_sa_cf = mean of CFL3D/FUN3D SA on 545x385 (TMR cf_convergence.dat). correlation_cf: M0.2 local "
           f"Karman-Schoenherr ({ks_local(RE_PER_M * X_PROBE):.6e}), M2 van Driest II (Hopkins-Inouye, r = {RECOVERY}, "
           f"Fc = {vdp['Fc']:.4f}, F_Rx = {vdp['F_Rx']:.4f}) = {vd:.6e}; White 0.455/ln^2(0.06 Re_x) = "
           f"{white_local(RE_PER_M * X_PROBE):.6e}. rho/nu drop: orders below the startup-stage peak. wall_s: startup + "
           f"final stage. cd_plate: SU2 CD of the wall (0 <= x <= 2, REF_AREA 2); the characteristic farfield on the outflow "
           f"boundary at the plate end raises cf for x > 1.5 (273x193 at M 0.2: +1.4 % at x = 1.5, +6 % at 1.9 vs CFL3D), so "
           f"cd_plate is not compared with the TMR drag. pipeline_stop_iter: final-stage iteration at which the ignisyeet RANS acceptance (rho 3, nu 2 orders, CD flat over 200 rows, at least 400 rows) would have stopped, cd_at_stop_vs_final_percent: plate CD there vs the last iteration; trend_rule_*: the same for the acceptance with the trend-extrapolation check (rans_replay.py, release/v0.4.0 73d22f8). peak_rss_per_rank_mb: largest MPI rank (/usr/bin/time of mpirun = largest child). "
           f"{args.provenance}\n")
    with open(args.out, "w") as f:
        f.write(hdr)
        w = csv.DictWriter(f, fieldnames=keys)
        w.writeheader()
        for r in out:
            w.writerow({k: (f"{v:.6g}" if isinstance(v, float) else v) for k, v in r.items()})
    print(open(args.out).read())
    # cf distribution on the finest grid of each Mach number vs the TMR CFL3D SA distribution (cf_plate.dat) and the
    # correlations, at fixed stations
    if args.tmr_cf:
        ref, zone = [], None
        for line in open(args.tmr_cf):
            if "zone" in line.lower():
                zone = line
            elif line.strip() and not line.startswith("variables") and zone and "CFL3D" in zone:
                a = line.split()
                ref.append((float(a[0]), float(a[1]), 0.0))
        fine = {}
        for name in os.listdir(args.work):
            d = os.path.join(args.work, name)
            if os.path.exists(os.path.join(d, "meta.json")):
                m = json.load(open(os.path.join(d, "meta.json")))
                if m["cells"] > fine.get(m["mach"], (0, None))[0]:
                    fine[m["mach"]] = (m["cells"], d, f"{m['ni']}x{m['nj']}")
        xs = [0.02, 0.05, 0.1, 0.2, 0.3, 0.5, 0.7, 0.97, 1.2, 1.5, 1.7, 1.9]
        out2 = args.out.replace(".csv", "_cf.csv")
        with open(out2, "w") as f:
            f.write("# cf along the plate on the finest grid of each Mach number (" + ", ".join(f"M{k:g}: {v[2]}" for k, v in sorted(fine.items()))
                    + "). tmr_cfl3d_sa: TMR cf_plate.dat (CFL3D SA, 545x385, M 0.2). ks_local: local Karman-Schoenherr; "
                    "van_driest2: M 2 adiabatic. Rising cf for x > 1.5 is the farfield outflow boundary at the plate end.\n")
            f.write("x,cf_m0.2,tmr_cfl3d_sa,ks_local,cf_m2,van_driest2\n")
            p02 = surface_cf(fine[0.2][1]) if 0.2 in fine else None
            p2 = surface_cf(fine[2.0][1]) if 2.0 in fine else None
            for x in xs:
                rex = RE_PER_M * x
                f.write(f"{x},{interp(p02, x) if p02 else float('nan'):.6e},{interp(ref, x):.6e},{ks_local(rex):.6e},"
                        f"{interp(p2, x) if p2 else float('nan'):.6e},{van_driest2(2.0, rex)[0]:.6e}\n")
        print(open(out2).read())


def main():
    p = argparse.ArgumentParser()
    p.add_argument("cmd", choices=["run", "post", "collect", "refs"])
    p.add_argument("--grids")
    p.add_argument("--work", required=False)
    p.add_argument("--machs", type=float, nargs="+", default=[0.2, 2.0])
    p.add_argument("--levels", type=int, nargs="+", default=[69, 137, 273])
    p.add_argument("--ranks", type=int, default=4)
    p.add_argument("--jobs", type=int, default=3)
    p.add_argument("--prefix", default=os.environ.get("IGNISYEET_CFD_PREFIX", ""))
    p.add_argument("--out", default="rans_flatplate.csv")
    p.add_argument("--provenance", default="")
    p.add_argument("--tmr-cf", help="TMR FlatPlate/SA/cf_plate.dat for the cf distribution file")
    a = p.parse_args()
    if a.cmd == "refs":
        rex = RE_PER_M * X_PROBE
        print("Re_x", rex, "KS local", ks_local(rex), "White", white_local(rex), "KS mean", ks_mean(rex))
        print("vD2 M2", van_driest2(2.0, rex))
        print("fs", freestream(0.2), freestream(2.0))
        return
    if a.cmd == "run":
        with cf.ThreadPoolExecutor(a.jobs) as ex:
            futs = {ex.submit(run_case, a, lv, m): (lv, m) for m in a.machs for lv in a.levels}
            for fu in cf.as_completed(futs):
                try:
                    print("done", futs[fu], fu.result(), flush=True)
                except Exception as e:  # noqa: BLE001
                    print("FAILED", futs[fu], e, flush=True)
    elif a.cmd == "post":
        for name in sorted(os.listdir(a.work)):
            d = os.path.abspath(os.path.join(a.work, name))
            if os.path.exists(os.path.join(d, "meta.json")):
                m = json.load(open(os.path.join(d, "meta.json")))["mach"]
                post_case(a, d, m)
                print("post", d, flush=True)
    else:
        collect(a)


if __name__ == "__main__":
    main()
