# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2025-2026 西濱大将 (NISHIHAMA Daisuke)
"""Pressure axial force of an SU2 Euler case split by wall region (fin leading/trailing/side/tip faces, body).
Used for doc/data/cfd/stl_fin_edge_axial.csv (the fin regions assume the sample rocket: body radius 0.05 m, fins from x = 1.25 m to the base at 1.5 m).

Usage: python cfd_axial_split.py <output.dir>/cfd <case directory name, e.g. m0.800_a00.00>"""
import sys
from pathlib import Path

import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parent))
from extract_cfd_fields import read_cfg  # noqa: E402


def read_mesh(path):
    lines = path.read_text().splitlines()
    i = 0; pts = None; tets = None; mk = {}
    while i < len(lines):
        s = lines[i].strip()
        if s.startswith("NELEM="):
            n = int(s.split("=")[1])
            a = np.array([l.split()[:5] for l in lines[i+1:i+1+n]], dtype=np.int64)
            tets = a[a[:, 0] == 10][:, 1:5]
            i += n + 1; continue
        if s.startswith("NPOIN="):
            n = int(s.split("=")[1].split()[0])
            pts = np.array([l.split()[:3] for l in lines[i+1:i+1+n]], dtype=float); i += n + 1; continue
        if s.startswith("MARKER_TAG="):
            tag = s.split("=")[1].strip(); m = int(lines[i+1].split("=")[1])
            mk[tag] = np.array([l.split()[1:4] for l in lines[i+2:i+2+m]], dtype=np.int64); i += m + 2; continue
        i += 1
    return pts, tets, mk

wd, case = Path(sys.argv[1]), sys.argv[2]
pts, tets, mk = read_mesh(wd / "mesh" / "mesh.su2")
cfg = read_cfg(wd / case / "case.cfg")
sf = np.genfromtxt(wd / case / "surface_flow.csv", delimiter=",", skip_header=1)
g = float(cfg["GAMMA_VALUE"]); pinf = float(cfg["FREESTREAM_PRESSURE"]); minf = float(cfg["MACH_NUMBER"])
rho = sf[:, 4]; m2 = sf[:, 5]**2 + sf[:, 6]**2 + sf[:, 7]**2
p = np.full(len(pts), np.nan); p[sf[:, 0].astype(np.int64)] = (g - 1) * (sf[:, 8] - 0.5 * m2 / rho)
assert np.allclose(pts[sf[:, 0].astype(np.int64)], sf[:, 1:4], atol=1e-6), "PointID does not index the mesh nodes"
q = 0.5 * g * pinf * minf**2
S = float(cfg.get("REF_AREA", "0.0078539816"))
# fluid-side node of each wall triangle from its tetrahedron
key = lambda t: tuple(sorted(t))
faces = {}
for name in ("wall", "base"):
    if name in mk:
        for k, t in enumerate(mk[name]): faces[key(t)] = (name, k)
fluid = {}
for tt in tets:
    for drop in range(4):
        f = key(np.delete(tt, drop))
        if f in faces: fluid[f] = tt[drop]
res = {}
for name in ("wall", "base"):
    if name not in mk: continue
    T = mk[name]; v = pts[T]
    n = 0.5 * np.cross(v[:, 1] - v[:, 0], v[:, 2] - v[:, 0])
    c = v.mean(1)
    other = np.array([fluid[key(t)] for t in T])
    s = np.sign(np.einsum("ij,ij->i", pts[other] - c, n))  # +: n points into the fluid = outward from the body
    n *= s[:, None]
    dp = p[T].mean(1) - pinf
    fx = -(dp * n[:, 0]) / (q * S)  # axial force (drag +)
    A = np.linalg.norm(n, axis=1); nh = n / A[:, None]
    r = np.hypot(c[:, 1], c[:, 2])
    fin = (r > 0.0505) & (c[:, 0] > 1.25) & (c[:, 0] <= 1.5 + 1e-6)
    if name == "base":
        res["base marker (fairing / flush faces)"] = fx.sum(); continue
    res["fin forward-facing (LE) n_x<-0.5"] = fx[fin & (nh[:, 0] < -0.5)].sum()
    res["fin aft-facing (TE) n_x>0.5"] = fx[fin & (nh[:, 0] > 0.5)].sum()
    res["fin other (sides, tip)"] = fx[fin & (np.abs(nh[:, 0]) <= 0.5)].sum()
    res["body"] = fx[~fin].sum()
    res["wall total"] = fx.sum()
    res["fin LE area m2"] = A[fin & (nh[:, 0] < -0.5)].sum()
    res["fin TE area m2"] = A[fin & (nh[:, 0] > 0.5)].sum()
    res["fin LE mean Cp"] = (dp[fin & (nh[:, 0] < -0.5)] * A[fin & (nh[:, 0] < -0.5)]).sum() / res["fin LE area m2"] / q
    res["fin TE mean Cp"] = (dp[fin & (nh[:, 0] > 0.5)] * A[fin & (nh[:, 0] > 0.5)]).sum() / max(res["fin TE area m2"], 1e-30) / q
    res["wall triangles"] = len(T)
for k, v in res.items(): print(f"{k:40s} {v: .5f}" if isinstance(v, float) else f"{k:40s} {v}")
