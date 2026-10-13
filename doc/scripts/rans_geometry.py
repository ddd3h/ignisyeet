#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
"""Geometries of the RANS verification cases (binary STL, metres; the configs use stl_scale = 1).

  cone OUT  10 degree half-angle cone + cylinder, R = 0.05 m, cylinder 0.3 m, flat base: the body of the Euler cone
            verification (crates/cfd/tests/real_su2.rs, cone_cp_matches_taylor_maccoll), nose towards -x.
  body OUT  the body of examples/sample_rocket.stl without its fins: the STL holds the body and each fin as separate
            closed shells, the largest connected shell (by triangle count) is the body.
"""
import math
import struct
import sys

import numpy as np


def write_stl(path, tris):
    tris = np.asarray(tris, dtype=np.float64)
    with open(path, "wb") as f:
        f.write(b"rans verification geometry".ljust(80, b" "))
        f.write(struct.pack("<I", len(tris)))
        for t in tris:
            n = np.cross(t[1] - t[0], t[2] - t[0])
            nn = np.linalg.norm(n)
            n = n / nn if nn > 0 else n
            f.write(struct.pack("<12fH", *n, *t[0], *t[1], *t[2], 0))


def read_stl(path):
    data = open(path, "rb").read()
    n = struct.unpack("<I", data[80:84])[0]
    rec = np.frombuffer(data[84:84 + 50 * n], dtype=np.dtype([("n", "<f4", 3), ("v", "<f4", (3, 3)), ("a", "<u2")]))
    return rec["v"].astype(np.float64)


def cone(path, r=0.05, delta_deg=10.0, lb=0.3, n=128, m=60):
    lc = r / math.tan(math.radians(delta_deg))
    ring = lambda x, rad, k: (x, rad * math.cos(2 * math.pi * k / n), rad * math.sin(2 * math.pi * k / n))
    t = []
    nc = 40  # cone split along its length so the STL facets do not limit the profile extraction
    for i in range(nc):
        x0, x1 = lc * i / nc, lc * (i + 1) / nc
        for k in range(n):
            if i == 0:
                t.append([(0.0, 0.0, 0.0), ring(x1, x1 / lc * r, k), ring(x1, x1 / lc * r, k + 1)])
            else:
                t.append([ring(x0, x0 / lc * r, k), ring(x1, x1 / lc * r, k), ring(x1, x1 / lc * r, k + 1)])
                t.append([ring(x0, x0 / lc * r, k), ring(x1, x1 / lc * r, k + 1), ring(x0, x0 / lc * r, k + 1)])
    for i in range(m):
        x0, x1 = lc + lb * i / m, lc + lb * (i + 1) / m
        for k in range(n):
            t.append([ring(x0, r, k), ring(x1, r, k), ring(x1, r, k + 1)])
            t.append([ring(x0, r, k), ring(x1, r, k + 1), ring(x0, r, k + 1)])
    xe = lc + lb
    for k in range(n):
        t.append([(xe, 0.0, 0.0), ring(xe, r, k + 1), ring(xe, r, k)])
    t = np.array(t)
    # Orient outward (radial on the side, +x on the base).
    for tri in t:
        nrm = np.cross(tri[1] - tri[0], tri[2] - tri[0])
        c = tri.mean(axis=0)
        out = np.array([1.0, 0, 0]) if abs(c[0] - xe) < 1e-12 and np.allclose(tri[:, 0], xe) else np.array([0.0, c[1], c[2]])
        if nrm @ out < 0:
            tri[[1, 2]] = tri[[2, 1]]
    write_stl(path, t)
    print(f"cone: {len(t)} triangles, cone length {lc:.5f} m, total {xe:.5f} m")


def body(src, path, scale=0.001):
    v = read_stl(src)
    pts = v.reshape(-1, 3)
    key = np.round(pts / 1e-6).astype(np.int64)
    _, inv = np.unique(key, axis=0, return_inverse=True)
    inv = inv.reshape(-1, 3)
    # union-find over vertices of each triangle
    parent = np.arange(inv.max() + 1)

    def find(a):
        while parent[a] != a:
            parent[a] = parent[parent[a]]
            a = parent[a]
        return a

    for a, b, c in inv:
        ra, rb, rc = find(a), find(b), find(c)
        parent[rb] = ra
        parent[find(rc)] = ra
    comp = np.array([find(a) for a in inv[:, 0]])
    labels, counts = np.unique(comp, return_counts=True)
    keep = labels[np.argmax(counts)]
    print(f"{len(labels)} shells with {sorted(counts.tolist(), reverse=True)} triangles; keeping the largest")
    write_stl(path, v[comp == keep] * scale)


if __name__ == "__main__":
    if sys.argv[1] == "cone":
        cone(sys.argv[2])
    elif sys.argv[1] == "body":
        body(sys.argv[2], sys.argv[3])
    else:
        sys.exit(__doc__)
