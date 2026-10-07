#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
"""RANS mesh convergence of the sample rocket: Richardson extrapolation and grid convergence index (Celik et al. 2008,
J. Fluids Eng. 130, 078001) of CN, CA and xcp at alpha 4 from three runs of rans_sample.py (coarse, medium, fine),
and |CN(alpha = 0)| of the medium run.

  rans_mesh_study.py COARSE MEDIUM FINE --out-csv doc/data/cfd/rans_mesh_study.csv --out-json doc/data/cfd/rans_mesh_study.json

Values per level: the mirror-combined row of cfd_cases.csv at alpha 4 (cn, ca_cfd, xcp_m; CA = CFx(wall), the table adds
the Barrowman base term). h = nodes^(-1/3). Gates: GCI_fine CN <= 3 %, CA <= 5 %, xcp <= 0.01 m (absolute GCI),
|CN(0)| <= 0.005.
"""
import argparse
import csv
import json
import math
import os
import subprocess


def level(d):
    out = os.path.join(d, "out")
    rep = json.load(open(os.path.join(out, "cfd_report.json")))
    rows = list(csv.DictReader(open(os.path.join(out, "cfd_cases.csv"))))
    cfg = open(os.path.join(d, "case.toml")).read()
    wall = float(next(l for l in cfg.splitlines() if l.startswith("wall_size")).split("=")[1].split("#")[0])
    res = {"dir": os.path.basename(d), "wall_size_m": wall, "nodes": rep["mesh_nodes"], "cells": rep["mesh_cells"],
           "wall_s": rep.get("wall_seconds"), "unconverged": rep.get("unconverged"), "failed": rep.get("failed"), "m": {}}
    for r in rows:
        m, a = float(r["mach"]), float(r["alpha_deg"])
        e = res["m"].setdefault(m, {})
        if r["status"] == "combined" and abs(a - 4) < 1e-9:
            e.update(cn=float(r["cn"]), ca=float(r["ca_cfd"]), xcp=float(r["xcp_m"]))
        elif abs(a) < 1e-9 and r["cn_raw"]:
            e.update(cn0=float(r["cn_raw"]), status0=r["status"])
        elif abs(a - 4) < 1e-9 and r["status"] != "combined":
            e.update(status4=r["status"], it4=int(r["iterations"]))
        elif abs(a + 4) < 1e-9:
            e.update(status_m4=r["status"], it_m4=int(r["iterations"]))
    return res


def gci(f1, f2, f3, h1, h2, h3, fs=1.25):
    """Fine f1 / h1 ... coarse f3 / h3 (Celik 2008, iterative order for non-constant ratios)."""
    r21, r32 = h2 / h1, h3 / h2
    e21, e32 = f2 - f1, f3 - f2
    if e21 == 0 or e32 == 0 or e32 / e21 <= 0:
        return {"order": None, "extrapolated": None, "gci_fine": None, "monotone": False, "e21": e21, "e32": e32}
    s = math.copysign(1.0, e32 / e21)
    p = 2.0
    for _ in range(200):
        q = math.log((r21 ** p - s) / (r32 ** p - s))
        p = abs(math.log(abs(e32 / e21)) + q) / math.log(r21)
    ext = (r21 ** p * f1 - f2) / (r21 ** p - 1)
    return {"order": p, "extrapolated": ext, "gci_fine_abs": fs * abs(e21) / (r21 ** p - 1),
            "gci_fine": fs * abs(e21 / f1) / (r21 ** p - 1), "monotone": True, "e21": e21, "e32": e32}


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("coarse")
    ap.add_argument("medium")
    ap.add_argument("fine")
    ap.add_argument("--out-csv", required=True)
    ap.add_argument("--out-json", required=True)
    a = ap.parse_args()
    L = [level(a.fine), level(a.medium), level(a.coarse)]
    h = [lv["nodes"] ** (-1 / 3) for lv in L]
    commit = subprocess.run(["git", "-C", os.path.dirname(os.path.abspath(__file__)), "rev-parse", "--short", "HEAD"], capture_output=True, text=True).stdout.strip()
    lines = [f"# RANS mesh convergence of the sample rocket (doc/scripts/rans_mesh_study.py): ignisyeet aero method cfd, model rans, half model, "
             f"machs [0.8, 2.0], alphas [0, 4] (+ -4 solved, mirror-combined), levels {', '.join(f'{lv['wall_size_m'] * 1000:g} mm' for lv in L[::-1])}; "
             f"N^(1/3) ratios {h[1] / h[0]:.3f} (fine/medium), {h[2] / h[1]:.3f} (medium/coarse). cn, ca (CFx(wall)), xcp at alpha 4; "
             f"cn0 = raw CN of the alpha 0 case. Scripts at {commit}, 2026-10-07.",
             "wall_size_m,nodes,cells,run_wall_s,mach,cn,ca_wall,xcp_m,cn0,status_a0,status_a4,status_am4,iterations_a4"]
    for lv in L[::-1]:
        for m, e in sorted(lv["m"].items()):
            lines.append(f"{lv['wall_size_m']},{lv['nodes']},{lv['cells']},{lv['wall_s']:.0f},{m:g},{e.get('cn', '')},{e.get('ca', '')},"
                         f"{e.get('xcp', '')},{e.get('cn0', '')},{e.get('status0', '')},{e.get('status4', '')},{e.get('status_m4', '')},{e.get('it4', '')}")
    open(a.out_csv, "w").write("\n".join(lines) + "\n")
    res = {"quantity": "GCI_fine (Celik 2008, Fs 1.25) of CN, CA, xcp at alpha 4; |CN(0)| on the medium level",
           "levels": [{k: lv[k] for k in ("wall_size_m", "nodes", "cells", "wall_s", "unconverged", "failed")} for lv in L[::-1]],
           "thresholds": {"gci_cn_percent": 3.0, "gci_ca_percent": 5.0, "gci_xcp_m": 0.01, "cn0_abs": 0.005}, "mach": {}}
    ok = True
    for m in sorted(L[0]["m"]):
        r = {}
        for k in ("cn", "ca", "xcp"):
            f = [lv["m"][m].get(k) for lv in L]
            if None in f:
                r[k] = None
                ok = False
                continue
            g = gci(f[0], f[1], f[2], h[0], h[1], h[2])
            r[k] = {"fine": f[0], "medium": f[1], "coarse": f[2], **g}
        cn0 = L[1]["m"][m].get("cn0")
        r["cn0_medium"] = cn0
        passes = {
            "cn": bool(r["cn"] and r["cn"]["gci_fine"] is not None and 100 * r["cn"]["gci_fine"] <= 3.0),
            "ca": bool(r["ca"] and r["ca"]["gci_fine"] is not None and 100 * r["ca"]["gci_fine"] <= 5.0),
            "xcp": bool(r["xcp"] and r["xcp"]["gci_fine"] is not None and r["xcp"]["gci_fine_abs"] <= 0.01),
            "cn0": cn0 is not None and abs(cn0) <= 0.005}
        r["pass"] = passes
        ok = ok and all(passes.values())
        res["mach"][f"{m:g}"] = r
    res["pass"] = ok
    res["provenance"] = os.path.basename(a.out_csv)
    json.dump(res, open(a.out_json, "w"), indent=1)
    print(open(a.out_csv).read())
    print(json.dumps(res, indent=1))


if __name__ == "__main__":
    main()
