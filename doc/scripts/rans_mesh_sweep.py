#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
"""RANS mesh sweep of the sample rocket: `ignisyeet cfd-check --mesh` for several wall sizes (model = "rans",
machs [0.8, 2.0], half model), timed with /usr/bin/time -v (wall time and peak RSS of the largest process, i.e.
the gmsh python process), statistics from mesh_stats.json.

  rans_mesh_sweep.py --work DIR --exe target/release/ignisyeet --out doc/data/cfd/rans_mesh_sweep.csv [--walls ...]
"""
import argparse
import json
import os
import re
import subprocess
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import rans_sample  # noqa: E402


def main():
    p = argparse.ArgumentParser()
    p.add_argument("--work", required=True)
    p.add_argument("--exe", required=True)
    p.add_argument("--out", required=True)
    p.add_argument("--walls", type=float, nargs="+", default=[0.008, 0.007, 0.006, 0.005, 0.004, 0.0035, 0.003, 0.0025])
    a = p.parse_args()
    commit = subprocess.run(["git", "-C", rans_sample.REPO, "rev-parse", "--short", "HEAD"], capture_output=True, text=True).stdout.strip()
    rows = []
    for w in a.walls:
        d = os.path.join(os.path.abspath(a.work), f"w{w * 1000:g}mm")
        ns = argparse.Namespace(dir=d, model="rans", machs=[0.8, 2.0], alphas=[0.0, 4.0], wall=w, ranks=4, parallel=1,
                                threads=1, memory_gb=0.0, stl=None, stl_scale=0.001, cg=None, nose=None, set=None)
        rans_sample.setup(ns)
        tfile = os.path.join(d, "time.txt")
        log = os.path.join(d, "check.log")
        env = dict(os.environ, IGNISYEET_CFD_PREFIX=rans_sample.PREFIX)
        with open(log, "w") as f:
            rc = subprocess.run(["/usr/bin/time", "-v", "-o", tfile, a.exe, "--no-progress", "cfd-check", "--mesh",
                                 os.path.join(d, "case.toml")], stdout=f, stderr=subprocess.STDOUT, env=env).returncode
        t = open(tfile).read()
        rss = int(re.search(r"Maximum resident set size \(kbytes\): (\d+)", t).group(1)) / 1024.0
        el = re.search(r"Elapsed \(wall clock\) time \(h:mm:ss or m:ss\): ([\d:.]+)", t).group(1).split(":")
        wall_s = sum(float(x) * 60 ** i for i, x in enumerate(reversed(el)))
        st = os.path.join(d, "out", "cfd", "mesh", "mesh_stats.json")
        row = {"wall_size_m": w, "status": "ok" if rc == 0 and os.path.exists(st) else "mesh error", "wall_s": round(wall_s, 1),
               "peak_rss_mb": round(rss, 1)}
        if os.path.exists(st) and rc == 0:
            s = json.load(open(st))
            row.update({"nodes": s["nodes"], "cells": s["cells"], "prisms": s["by_type"].get("Prism 6", 0),
                        "tetrahedra": s["by_type"].get("Tetrahedron 4", 0), "pyramids": s["by_type"].get("Pyramid 5", 0),
                        "wall_triangles": s["wall_triangles"], "bl_layers": s["bl_layers"],
                        "bl_first_height_m": s["bl_first_height"], "bl_thickness_m": round(s["bl_thickness"], 6),
                        "bl_invalid": s["bl_invalid"], "sicn_min": round(s["quality_sicn_min"], 4)})
        else:
            row["error"] = next((l.strip() for l in open(log) if "error" in l.lower()), "").replace(",", ";")
        rows.append(row)
        print(row, flush=True)
    keys = []
    for r in rows:
        keys += [k for k in r if k not in keys]
    with open(a.out, "w") as f:
        f.write(f"# RANS mesh sweep of the sample rocket (examples/sample_cfd.toml with model = rans, machs [0.8, 2.0], half model): "
                f"ignisyeet cfd-check --mesh per wall size (doc/scripts/rans_mesh_sweep.py), gmsh 4.15.2 single thread, commit {commit}, "
                f"2026-10-07, i7-12700 shared with other jobs. wall_s / peak_rss_mb: /usr/bin/time -v of cfd-check (largest process = gmsh).\n")
        f.write(",".join(keys) + "\n")
        for r in rows:
            f.write(",".join(str(r.get(k, "")) for k in keys) + "\n")


if __name__ == "__main__":
    main()
