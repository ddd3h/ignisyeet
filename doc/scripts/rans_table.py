#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
"""RANS coefficient table of the sample rocket (rans_sample.py run of the doc's CFD matrix) against the Euler CFD
table (doc/data/cfd/sample_euler_vs_models.csv): run statistics and the comparison across Mach number.

  rans_table.py RUN_DIR --euler doc/data/cfd/sample_euler_vs_models.csv --out-csv ... --out-json ...

Per solved case: status (converged / unconverged / failed), iterations, wall time. Per Mach number at alpha 4 and 8:
CN, xcp (mirror-combined rows of cfd_cases.csv) and the power-off axial coefficient ca_off of aero_table.csv (RANS:
CFx(wall) + Barrowman base + extra; Euler: pressure CA + Barrowman friction, base, fin terms) against the Euler CFD.
Gate: at least 90 % of the cases converged (unconverged = accepted but not converged; failed = no result).
"""
import argparse
import csv
import json
import os


def rows(path):
    with open(path) as f:
        return list(csv.DictReader(l for l in f if not l.startswith("#")))


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("run")
    ap.add_argument("--euler", required=True)
    ap.add_argument("--out-csv", required=True)
    ap.add_argument("--out-json", required=True)
    ap.add_argument("--note", default="")
    a = ap.parse_args()
    out = os.path.join(a.run, "out")
    cases = rows(os.path.join(out, "cfd_cases.csv"))
    rep = json.load(open(os.path.join(out, "cfd_report.json")))
    meta = json.load(open(os.path.join(a.run, "run.json")))
    table = rows(os.path.join(out, "aero_table.csv"))
    eu = rows(a.euler)
    solved = [c for c in cases if c["status"] != "combined"]
    count = {s: sum(1 for c in solved if c["status"] == s) for s in ("converged", "unconverged", "accepted", "failed")}
    frac = count["converged"] / len(solved)

    def tab(m, al, k):
        r = [x for x in table if abs(float(x["mach"]) - m) < 1e-9 and abs(float(x["alpha_deg"]) - al) < 1e-9]
        return float(r[0][k]) if r else float("nan")

    lines, comp = [], []
    for c in cases:
        if c["status"] != "combined":
            continue
        m, al = float(c["mach"]), float(c["alpha_deg"])
        e = [x for x in eu if abs(float(x["mach"]) - m) < 1e-9 and abs(float(x["alpha_deg"]) - al) < 1e-9]
        if not e:
            continue
        e = e[0]
        r = {"mach": m, "alpha_deg": al, "rans_cn": float(c["cn"]), "euler_cn": float(e["cfd_cn"]),
             "rans_xcp_m": float(c["xcp_m"]), "euler_xcp_m": float(e["cfd_xcp_m"]),
             "rans_ca_off": tab(m, al, "ca_off"), "euler_ca_off": float(e["cfd_ca_off"])}
        r["cn_diff_percent"] = 100 * (r["rans_cn"] / r["euler_cn"] - 1)
        r["xcp_diff_m"] = r["rans_xcp_m"] - r["euler_xcp_m"]
        r["ca_off_diff_percent"] = 100 * (r["rans_ca_off"] / r["euler_ca_off"] - 1)
        comp.append(r)
    for m in sorted({float(c["mach"]) for c in cases}):
        e = [x for x in eu if abs(float(x["mach"]) - m) < 1e-9 and abs(float(x["alpha_deg"])) < 1e-9]
        if e:
            r = {"mach": m, "alpha_deg": 0.0, "rans_ca_off": tab(m, 0, "ca_off"), "euler_ca_off": float(e[0]["cfd_ca_off"])}
            r["ca_off_diff_percent"] = 100 * (r["rans_ca_off"] / r["euler_ca_off"] - 1)
            comp.append(r)
    comp.sort(key=lambda r: (r["mach"], r["alpha_deg"]))
    keys = []
    for r in comp:
        keys += [k for k in r if k not in keys]
    with open(a.out_csv, "w") as f:
        f.write(f"# RANS table of the sample rocket vs the Euler CFD table (doc/scripts/rans_table.py). {a.note} Cases: {len(solved)} solved, "
                f"{count['converged']} converged, {count['unconverged']} unconverged (accepted), {count['failed']} failed; run wall time "
                f"{meta['wall_s'] / 3600:.2f} h; mesh {rep.get('mesh_nodes')} nodes.\n")
        f.write(",".join(keys) + "\n")
        for r in comp:
            f.write(",".join(f"{r[k]:.5g}" if k in r else "" for k in keys) + "\n")
    with open(a.out_csv.replace(".csv", "_cases.csv"), "w") as f:
        f.write(f"# Solved cases of the RANS table run (doc/scripts/rans_table.py). {a.note}\n")
        f.write("mach,alpha_deg,status,iterations,wall_seconds,warm_start\n")
        for c in solved:
            f.write(f"{c['mach']},{c['alpha_deg']},{c['status']},{c['iterations']},{c['wall_seconds']},{c['warm_start']}\n")
    res = {"cases": len(solved), **count, "converged_fraction": frac, "threshold": 0.9, "pass": frac >= 0.9,
           "run_wall_h": meta["wall_s"] / 3600, "mesh_nodes": rep.get("mesh_nodes"), "comparison": comp, "note": a.note,
           "provenance": os.path.basename(a.out_csv)}
    json.dump(res, open(a.out_json, "w"), indent=1)
    print(open(a.out_csv).read())


if __name__ == "__main__":
    main()
