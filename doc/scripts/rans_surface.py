#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
"""Pressure / friction split of the axial force of a RANS case from its surface_flow.vtk (wall and base markers).

  rans_surface.py CASE_DIR [--json]

Integrates over the surface triangles (nodal values averaged per triangle):
  CA_friction = sum(cf_x A) / REF_AREA,   CA_pressure = -sum(Cp n_x A) / REF_AREA   (n outward from the body)
separately for the wall (all cells but the flat base cap) and the base cap (cells with every node at the largest x
and the normal along x). The orientation is taken from the cylinder: the triangle winding is flipped globally when
the normals there point to the axis. REF_AREA comes from case.cfg (S/2 on a half model). The sum of the wall
terms is a check against CFx(wall) of the SU2 history.
"""
import argparse
import json
import os
import sys

import numpy as np

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from rans_flatplate import read_vtk  # noqa: E402


def cells(path):
    tok = open(path).read().split()
    i = tok.index("CELLS")
    n = int(tok[i + 1])
    k, out = i + 3, []
    for _ in range(n):
        m = int(tok[k])
        out.append([int(t) for t in tok[k + 1:k + 1 + m]])
        k += m + 1
    return out


def split(case):
    cfg = dict(l.split("=", 1) for l in open(os.path.join(case, "case.cfg")) if "=" in l)
    aref = float(cfg["REF_AREA"])
    vtk = os.path.join(case, "surface_flow.vtk")
    pts, f = read_vtk(vtk)
    P = np.array(pts)
    cp = np.array(f["Pressure_Coefficient"])
    cf = np.array(f["Skin_Friction_Coefficient"])
    yp = np.array(f["Y_Plus"])
    xmax = P[:, 0].max()
    acc = {k: 0.0 for k in ("wall_fric_x", "wall_pres_x", "base_fric_x", "base_pres_x", "wall_fric_z", "wall_pres_z", "wall_area")}
    tris = []
    for c in cells(vtk):
        for j in range(1, len(c) - 1):  # fan for quads
            tris.append((c[0], c[j], c[j + 1]))
    T = np.array(tris)
    a, b, c = P[T[:, 0]], P[T[:, 1]], P[T[:, 2]]
    nA = 0.5 * np.cross(b - a, c - a)  # area-weighted normal
    cen = (a + b + c) / 3.0
    base = (np.abs(a[:, 0] - xmax) < 1e-6) & (np.abs(b[:, 0] - xmax) < 1e-6) & (np.abs(c[:, 0] - xmax) < 1e-6)
    # orientation from mid-body cylinder cells: outward normal has a positive radial component
    rad = np.stack([np.zeros(len(cen)), cen[:, 1], cen[:, 2]], axis=1)
    mid = (~base) & (cen[:, 0] > 0.4 * xmax) & (cen[:, 0] < 0.6 * xmax)
    if np.sum(np.einsum("ij,ij->i", nA[mid], rad[mid])) < 0:
        nA = -nA
    cpt = cp[T].mean(axis=1)
    cft = cf[T].mean(axis=1)
    area = np.linalg.norm(nA, axis=1)
    w = ~base
    out = {
        "ref_area": aref,
        "wall_friction_ca": float((cft[w, 0] * area[w]).sum() / aref),
        "wall_pressure_ca": float(-(cpt[w] * nA[w, 0]).sum() / aref),
        "base_pressure_ca": float(-(cpt[base] * nA[base, 0]).sum() / aref),
        "base_friction_ca": float((cft[base, 0] * area[base]).sum() / aref),
        "wall_friction_cn": float((cft[w, 2] * area[w]).sum() / aref),
        "wall_pressure_cn": float(-(cpt[w] * nA[w, 2]).sum() / aref),
        "wall_area_m2": float(area[w].sum()),
        "yplus_p95": float(np.percentile(yp, 95)),
        "yplus_max": float(yp.max()),
    }
    out["wall_ca_sum"] = out["wall_friction_ca"] + out["wall_pressure_ca"]
    return out


if __name__ == "__main__":
    p = argparse.ArgumentParser()
    p.add_argument("case")
    a = p.parse_args()
    print(json.dumps(split(a.case), indent=1))
