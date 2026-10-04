# SPDX-License-Identifier: GPL-3.0-or-later
# IgnisYeet STL union (generated into the mesh work directory and run with the CFD python).
# Usage: python stl_union.py union_params.json
#
# Unites the closed, outward-oriented, welded components written by the Rust repair step (binary STL,
# CFD frame) with manifold3d (exact boolean, batch_boolean Add) and writes union.stl plus
# union_report.json (volume, area, genus, triangle count, bounding box, per-component volumes).
# Aborts on a manifold3d error status, a non-positive volume, or a detached body (a union that is not
# one connected solid has a negative genus: sum over disjoint shells of 1 - chi / 2).
import json
import struct
import sys

import numpy as np
import manifold3d as m3

p = json.load(open(sys.argv[1]))


def read_stl(path):
    b = open(path, "rb").read()
    n = struct.unpack("<I", b[80:84])[0]
    rec = np.frombuffer(b[84:84 + 50 * n], dtype=np.dtype([("n", "<f4", 3), ("v", "<f4", (3, 3)), ("a", "<u2")]))
    return rec["v"].astype(np.float32)


def manifold_of(tri):
    # The Rust side welded the vertices before writing, so identical float32 triples are shared vertices.
    v = tri.reshape(-1, 3)
    uniq, inv = np.unique(v, axis=0, return_inverse=True)
    mesh = m3.Mesh(vert_properties=uniq.astype(np.float32), tri_verts=inv.reshape(-1, 3).astype(np.uint32))
    return m3.Manifold(mesh)


parts = []
rows = []
for i, path in enumerate(p["components"]):
    mf = manifold_of(read_stl(path))
    st = str(mf.status())
    vol = float(mf.volume())
    rows.append({"index": i, "status": st, "volume": vol, "triangles": int(mf.num_tri())})
    if "NoError" not in st:
        raise SystemExit("union error: component %d (%s) is not a valid manifold: %s" % (i, path, st))
    if vol <= 0.0:
        raise SystemExit("union error: component %d has a non-positive volume %.3e (inside-out?)" % (i, vol))
    parts.append(mf)

u = parts[0] if len(parts) == 1 else m3.Manifold.batch_boolean(parts, m3.OpType.Add)
st = str(u.status())
if "NoError" not in st:
    raise SystemExit("union error: manifold3d returned %s" % st)
vol, area, genus = float(u.volume()), float(u.surface_area()), int(u.genus())
if vol <= 0.0:
    raise SystemExit("union error: the united solid has a non-positive volume %.3e" % vol)
if genus < 0:
    raise SystemExit(
        "union error: the united surface consists of several detached bodies (genus %d): "
        "a part of the STL (e.g. a fin) does not touch the rest" % genus
    )
sum_vol = sum(r["volume"] for r in rows)

out = u.to_mesh()
V = np.asarray(out.vert_properties)[:, :3].astype(np.float64)
T = np.asarray(out.tri_verts)
t = V[T]
nrm = np.cross(t[:, 1] - t[:, 0], t[:, 2] - t[:, 0])
ln = np.linalg.norm(nrm, axis=1)
nrm = nrm / np.maximum(ln, 1e-300)[:, None]
with open(p["out_stl"], "wb") as fh:
    fh.write(b"IgnisYeet union".ljust(80, b"\0"))
    fh.write(struct.pack("<I", len(T)))
    rec = np.zeros(len(T), dtype=np.dtype([("n", "<f4", 3), ("v", "<f4", (3, 3)), ("a", "<u2")]))
    rec["n"] = nrm
    rec["v"] = t
    fh.write(rec.tobytes())
bb = u.bounding_box()
report = {
    "components": rows,
    "volume": vol,
    "surface_area": area,
    "genus": genus,
    "triangles": int(len(T)),
    "vertices": int(len(V)),
    "volume_sum_of_components": sum_vol,
    "overlap_volume": sum_vol - vol,
    "bbox_min": [float(x) for x in bb[:3]],
    "bbox_max": [float(x) for x in bb[3:]],
}
json.dump(report, open(p["report_json"], "w"), indent=1)
print("UNION_OK volume=%.6e area=%.6e genus=%d triangles=%d components=%d" % (vol, area, genus, len(T), len(rows)))
