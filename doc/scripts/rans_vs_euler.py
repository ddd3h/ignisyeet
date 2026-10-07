#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
"""Cross-check of the RANS sample results against the Euler CFD and the Barrowman model (comparison between models).

  rans_vs_euler.py RANS_RUN_DIR --euler doc/data/cfd/sample_euler_vs_models.csv --euler-mesh doc/data/cfd/mesh_convergence.csv
                   --barrowman-smooth BARROWMAN_OUT/aero_drag.csv --out-csv ... --out-json ...

* CN at alpha 4 and CN_alpha = CN(4) / 4 deg (both models; Euler 2 mm, 941k cells, sharp fin edges, conical tail fairing);
  xcp at alpha 4. Gates: CN_alpha within 8 %, xcp within 0.03 m.
* Skin friction: RANS friction part of the wall axial force (surface_flow.vtk of the alpha 0 case, rans_surface.py;
  body and fins, the base cap excluded) vs the Barrowman friction term of a smooth wall (roughness 0; the RANS wall is
  smooth). Gate 25 %. Also CFx(wall) of RANS minus the Euler pressure CA of the same Mach number (mesh_convergence.csv
  at 2 mm, alpha 4) as a second estimate of the RANS friction.
"""
import argparse
import csv
import json
import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from rans_surface import split  # noqa: E402


def rows(path):
    with open(path) as f:
        return list(csv.DictReader(l for l in f if not l.startswith("#")))


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("run")
    ap.add_argument("--euler", required=True)
    ap.add_argument("--euler-mesh", required=True)
    ap.add_argument("--barrowman-smooth", required=True)
    ap.add_argument("--out-csv", required=True)
    ap.add_argument("--out-json", required=True)
    ap.add_argument("--note", default="")
    a = ap.parse_args()
    out = os.path.join(a.run, "out")
    cases = rows(os.path.join(out, "cfd_cases.csv"))
    rep = json.load(open(os.path.join(out, "cfd_report.json")))
    eu = rows(a.euler)
    em = rows(a.euler_mesh)
    bd = rows(a.barrowman_smooth)
    res, lines = {"mach": {}}, []
    ok = True
    for c in cases:
        if c["status"] != "combined" or abs(float(c["alpha_deg"]) - 4) > 1e-9:
            continue
        m = float(c["mach"])
        cn, xcp = float(c["cn"]), float(c["xcp_m"])
        e = next(x for x in eu if abs(float(x["mach"]) - m) < 1e-9 and abs(float(x["alpha_deg"]) - 4) < 1e-9)
        ecn, excp = float(e["cfd_cn"]), float(e["cfd_xcp_m"])
        bcn, bxcp = float(e["barrowman_cn"]), float(e["barrowman_xcp_m"])
        a0 = os.path.join(out, "cfd", f"m{m:.3f}_a00.00")
        s = split(a0)
        h = list(csv.reader(open(os.path.join(a0, "history.csv"))))
        head = [x.strip().strip('"') for x in h[0]]
        cfx_wall = float(h[-1][head.index("CFx(wall)")])
        epca = float(next(x for x in em if abs(float(x["mach"]) - m) < 1e-9 and abs(float(x["wall_size_m"]) - 0.002) < 1e-9)["ca_cfd"])
        bf = float(next(x for x in bd if abs(float(x["mach"]) - m) < 1e-9)["friction"])
        r = {"rans_cn_a4": cn, "euler_cn_a4": ecn, "barrowman_cn_a4": bcn,
             "cna_diff_percent": 100 * (cn / ecn - 1), "rans_xcp_m": xcp, "euler_xcp_m": excp, "barrowman_xcp_m": bxcp,
             "xcp_diff_m": xcp - excp, "rans_friction_ca_a0": s["wall_friction_ca"], "rans_cfx_wall_a0": cfx_wall,
             "euler_pressure_ca_a4_2mm": epca, "rans_cfx_wall_minus_euler_pressure": cfx_wall - epca,
             "barrowman_friction_smooth": bf, "friction_diff_percent": 100 * (s["wall_friction_ca"] / bf - 1)}
        r["pass"] = {"cna": abs(r["cna_diff_percent"]) <= 8.0, "xcp": abs(r["xcp_diff_m"]) <= 0.03,
                     "friction": abs(r["friction_diff_percent"]) <= 25.0}
        ok = ok and all(r["pass"].values())
        res["mach"][f"{m:g}"] = r
        lines.append(f"{m:g}," + ",".join(f"{v:.5g}" for k, v in r.items() if k != "pass"))
    res.update({"pass": ok, "thresholds": {"cna_percent": 8.0, "xcp_m": 0.03, "friction_percent": 25.0},
                "rans_run": {"mesh_nodes": rep.get("mesh_nodes"), "unconverged": rep.get("unconverged")},
                "geometry": "RANS: fins with edges blunted at 2 % chord, flat base (rans_base model adds the Barrowman base term); "
                            "Euler: sharp biconvex fins, conical tail fairing (6 base radii), base drag from Barrowman",
                "note": a.note, "provenance": os.path.basename(a.out_csv)})
    keys = [k for k in next(iter(res["mach"].values())) if k != "pass"]
    with open(a.out_csv, "w") as f:
        f.write("# RANS vs Euler CFD vs Barrowman for the sample rocket (doc/scripts/rans_vs_euler.py). " + a.note + "\n")
        f.write("mach," + ",".join(keys) + "\n")
        f.write("\n".join(lines) + "\n")
    json.dump(res, open(a.out_json, "w"), indent=1)
    print(open(a.out_csv).read())


if __name__ == "__main__":
    main()
