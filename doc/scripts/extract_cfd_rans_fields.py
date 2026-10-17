# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2024-2026 西濱大将 (NISHIHAMA Daisuke)
"""Extract the Euler / RANS comparison data of the CFD chapter from two finished CFD runs.

Both runs solve the same sample rocket at one (Mach, alpha) point (``ignisyeet aero`` with ``aero.cfd.model`` =
"euler" and "rans"). The script writes to ``doc/data/cfd/fields_rans/``:

* ``<model>_base.npz``     Mach number on the symmetry plane (y = 0) around the aft body and the wake
* ``<model>_wallmesh.npz`` edges of the volume-mesh section on the symmetry plane near the wall at a body station
* ``<model>_profile.npz``  velocity |V| / U_inf along the wall normal (+z side) at the same station

Usage: python extract_cfd_rans_fields.py <euler cfd dir> <rans cfd dir> <case> <doc/data/cfd/fields_rans>
"""

import sys
from pathlib import Path

import numpy as np
from matplotlib.tri import LinearTriInterpolator, Triangulation

from extract_cfd_fields import mach_cp, read_cfg, read_restart

X_STATION = 0.75  # body station of the wall-normal profile and the mesh zoom [m] (half the body length)
MESH_WIN = ((0.72, 0.78), (0.0, 0.03))  # x range, height above the wall [m] of the mesh zoom
BASE_WIN = ((1.15, 2.35), (-0.30, 0.30))  # x, z range of the stored Mach field [m]


def read_symmetry(path: Path):
    """Return (points[n,3], faces) of the symmetry marker of an ASCII SU2 mesh; faces are lists of node ids (tri or quad)."""
    lines = path.read_text().splitlines()
    pts, faces = None, []
    for i, s in enumerate(lines):
        if s.startswith("NPOIN="):
            n = int(s.split("=")[1].split()[0])
            pts = np.loadtxt(lines[i + 1:i + 1 + n], usecols=(0, 1, 2))
        elif s.strip() == "MARKER_TAG= symmetry":
            m = int(lines[i + 1].split("=")[1])
            for ln in lines[i + 2:i + 2 + m]:
                v = ln.split()
                faces.append([int(t) for t in v[1:1 + (3 if v[0] == "5" else 4)]])
            break
    return pts, faces


def main() -> None:
    dirs = {"euler": Path(sys.argv[1]), "rans": Path(sys.argv[2])}
    case, out = sys.argv[3], Path(sys.argv[4])
    out.mkdir(parents=True, exist_ok=True)
    for model, cdir in dirs.items():
        cfg = read_cfg(cdir / case / "case.cfg")
        minf, alpha = float(cfg["MACH_NUMBER"]), float(cfg["AOA"])
        pts, faces = read_symmetry(cdir / "mesh" / "mesh.su2")
        sol = read_restart(cdir / case / "restart.dat")
        mach, _ = mach_cp(sol, cfg)
        g, p_inf, t_inf = float(cfg["GAMMA_VALUE"]), float(cfg["FREESTREAM_PRESSURE"]), float(cfg["FREESTREAM_TEMPERATURE"])
        u_inf = minf * np.sqrt(g * float(cfg["GAS_CONSTANT"]) * t_inf)
        speed = np.sqrt(sol["Momentum_x"] ** 2 + sol["Momentum_y"] ** 2 + sol["Momentum_z"] ** 2) / sol["Density"] / u_inf

        used = np.unique(np.concatenate([np.array(f) for f in faces]))
        remap = np.full(len(pts), -1, dtype=np.int64)
        remap[used] = np.arange(len(used))
        xz = pts[used][:, [0, 2]]
        quads = [f for f in faces if len(f) == 4]
        tris = [f for f in faces if len(f) == 3] + [q[:3] for q in quads] + [[q[0], q[2], q[3]] for q in quads]
        tri = remap[np.array(tris)]

        # (a) Mach number around the aft body and the wake
        sel = (xz[:, 0] > BASE_WIN[0][0]) & (xz[:, 0] < BASE_WIN[0][1]) & (np.abs(xz[:, 1]) < BASE_WIN[1][1])
        keep = sel[tri].any(axis=1)
        sub = np.unique(tri[keep])
        rm = np.full(len(xz), -1, dtype=np.int64)
        rm[sub] = np.arange(len(sub))
        np.savez_compressed(out / f"{model}_base.npz", points=xz[sub].astype(np.float32), triangles=rm[tri[keep]].astype(np.int32),
                            mach=mach[used][sub].astype(np.float16))

        # (b) volume-mesh edges on the symmetry plane near the wall
        radius = xz[(np.abs(xz[:, 0] - X_STATION) < 6e-3) & (xz[:, 1] > 0)][:, 1].min()
        edges = set()
        for f in faces:
            for a, b in zip(f, f[1:] + f[:1]):
                edges.add((min(a, b), max(a, b)))
        e = np.array(sorted(edges))
        p0, p1 = pts[e[:, 0]][:, [0, 2]], pts[e[:, 1]][:, [0, 2]]
        inwin = lambda p: (p[:, 0] > MESH_WIN[0][0]) & (p[:, 0] < MESH_WIN[0][1]) & (p[:, 1] > radius - 1e-4) & (p[:, 1] < radius + MESH_WIN[1][1])
        k = inwin(p0) & inwin(p1)
        seg = np.stack([p0[k], p1[k]], axis=1)
        np.savez_compressed(out / f"{model}_wallmesh.npz", segments=seg.astype(np.float32), radius=radius)

        # (c) |V| / U_inf along the wall normal at the station (+z side)
        tri_i = Triangulation(xz[:, 0], xz[:, 1], tri)
        d = np.concatenate([[0.0], np.logspace(-6.5, np.log10(0.10), 600)])
        z = radius + d
        v = LinearTriInterpolator(tri_i, speed[used])(np.full_like(z, X_STATION), z)
        m = LinearTriInterpolator(tri_i, mach[used])(np.full_like(z, X_STATION), z)
        ok = ~np.ma.getmaskarray(v)
        np.savez_compressed(out / f"{model}_profile.npz", d=d[ok], v=np.asarray(v)[ok], mach=np.asarray(m)[ok], radius=radius, x=X_STATION,
                            mach_inf=minf, alpha_deg=alpha)
        print(f"{model}: {len(used)} symmetry nodes, radius {radius:.4f} m, {k.sum()} mesh edges in the zoom")


if __name__ == "__main__":
    main()
