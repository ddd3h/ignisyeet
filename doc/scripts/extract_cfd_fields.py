# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2025-2026 西濱大将 (NISHIHAMA Daisuke)
"""Extract the mesh and flow-field data behind the CFD chapter's figures from a finished CFD run.

The documentation build cannot run SU2, so this script is run by hand after
``ignisyeet aero`` with ``aero.method = "cfd"`` and writes compact NumPy archives to
``doc/data/cfd/fields/``:

* ``mesh_wall.npz``      wall triangulation of the half model (surface mesh, wall Cp)
* ``mesh_symmetry.npz``  symmetry-plane triangulation (volume-mesh section, Mach field)
* ``field_<case>.npz``   symmetry-plane Mach number and wall Cp of the cases with alpha > 0
* ``history_<case>.csv`` convergence history (iteration, density residual, CL, CD)

Usage: python extract_cfd_fields.py <output.dir>/cfd <doc/data/cfd/fields>
"""

import shutil
import struct
import sys
from pathlib import Path

import numpy as np


def read_su2(path: Path):
    """Return (points[n,3], {marker: triangles[m,3]}) from an ASCII SU2 mesh (volume elements skipped)."""
    lines = path.read_text().splitlines()
    i = 0
    pts = None
    markers = {}
    while i < len(lines):
        s = lines[i].strip()
        if s.startswith("NELEM="):
            i += int(s.split("=")[1]) + 1
            continue
        if s.startswith("NPOIN="):
            n = int(s.split("=")[1].split()[0])
            pts = np.loadtxt(lines[i + 1:i + 1 + n], usecols=(0, 1, 2))
            i += n + 1
            continue
        if s.startswith("MARKER_TAG="):
            tag = s.split("=")[1].strip()
            m = int(lines[i + 1].split("=")[1])
            tri = np.loadtxt(lines[i + 2:i + 2 + m], dtype=np.int64, usecols=(1, 2, 3), ndmin=2)
            markers[tag] = tri
            i += m + 2
            continue
        i += 1
    return pts, markers


def read_restart(path: Path):
    """Read an SU2 binary restart file; return {field name: values[n]}."""
    raw = path.read_bytes()
    head = struct.unpack("5i", raw[:20])
    if head[0] != 535532:
        raise SystemExit(f"{path}: not an SU2 binary restart file")
    nvar, npt = head[1], head[2]
    off = 20
    names = []
    for _ in range(nvar):
        names.append(raw[off:off + 33].split(b"\0")[0].decode())
        off += 33
    data = np.frombuffer(raw, dtype=np.float64, count=nvar * npt, offset=off).reshape(npt, nvar)
    return {n: data[:, k] for k, n in enumerate(names)}


def read_cfg(path: Path):
    """KEY= value pairs of an SU2 configuration file."""
    cfg = {}
    for line in path.read_text().splitlines():
        if "=" in line and not line.lstrip().startswith("%"):
            k, v = line.split("=", 1)
            cfg[k.strip()] = v.strip()
    return cfg


def mach_cp(sol, cfg):
    """Mach number and pressure coefficient from the conservative variables of a dimensional Euler restart."""
    g = float(cfg["GAMMA_VALUE"])
    p_inf, m_inf = float(cfg["FREESTREAM_PRESSURE"]), float(cfg["MACH_NUMBER"])
    rho = sol["Density"]
    mom2 = sol["Momentum_x"] ** 2 + sol["Momentum_y"] ** 2 + sol["Momentum_z"] ** 2
    p = (g - 1.0) * (sol["Energy"] - 0.5 * mom2 / rho)
    mach = np.sqrt(mom2) / rho / np.sqrt(g * p / rho)
    cp = (p - p_inf) / (0.5 * g * p_inf * m_inf**2)
    return mach, cp


def compact(pts, tri):
    """Renumber the nodes used by tri; return (points, triangles, original node ids)."""
    used = np.unique(tri)
    remap = np.full(len(pts), -1, dtype=np.int64)
    remap[used] = np.arange(len(used))
    return pts[used], remap[tri], used


def main() -> None:
    cfd_dir, out = Path(sys.argv[1]), Path(sys.argv[2])
    out.mkdir(parents=True, exist_ok=True)
    pts, mk = read_su2(cfd_dir / "mesh" / "mesh.su2")
    f32 = np.float32

    wall = np.vstack([mk["wall"], mk["base"]])
    pw, tw, wall_ids = compact(pts, wall)
    np.savez_compressed(out / "mesh_wall.npz", points=pw.astype(f32), triangles=tw.astype(np.int32))

    sym = mk["symmetry"]
    ps, ts, sym_ids = compact(pts, sym)
    np.savez_compressed(out / "mesh_symmetry.npz", points=ps[:, [0, 2]].astype(f32), triangles=ts.astype(np.int32))

    for case in sorted(d for d in cfd_dir.iterdir() if d.name.startswith("m") and (d / "restart.dat").exists()):
        sol = read_restart(case / "restart.dat")
        mach, cp = mach_cp(sol, read_cfg(case / "case.cfg"))
        if not case.name.endswith("a00.00"):
            f16 = np.float16
            np.savez_compressed(out / f"field_{case.name}.npz", sym_mach=mach[sym_ids].astype(f16), wall_cp=cp[wall_ids].astype(f16))
        shutil.copyfile(case / "history.csv", out / f"history_{case.name}.csv")
        print(f"{case.name}: {len(sym_ids)} symmetry nodes, {len(wall_ids)} wall nodes")


if __name__ == "__main__":
    main()
