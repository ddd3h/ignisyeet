# SPDX-License-Identifier: GPL-3.0-or-later
# IgnisYeet volume mesher for a direct STL surface (generated into the mesh work directory and run with
# the gmsh python). Usage: python mesh_stl.py params.json
#
# Reads the united, outward-oriented surface `stl_surface` (binary STL in the CFD frame: x aft from the
# nose tip, written by stl_union.py) and prepares the wall surface according to `stl_remesh`:
#   classify (default) classifySurfaces at 40 deg + createGeometry, then gmsh's surface mesher at the size
#            field. The STL of a rocket triangulates curved faces into long thin facets and flat faces (fins) into
#            a handful of huge ones; meshed as they are they under-resolve the fins (normal force 30 % low at
#            M 2 on the sample), so the surface is remeshed at `wall_size`. An error (gmsh cannot parametrise
#            the surface) aborts: use `none` knowingly.
#   none     the triangles of the union as they are.
# Then builds a box farfield with the geo kernel (its faces must share their curves, otherwise HXT fails with
# "vertex lies in a segment"), meshes the volume with tetrahedra graded away from the wall (same size field
# as mesh_gmsh.py; single-thread HXT, Delaunay when run with algorithm3d = 1) and writes an ASCII SU2 mesh.
#
# The wall triangles are split into the markers `wall` and `base` by facet centroid and normal (the same rule
# as surface::split_base): a facet is base when its centroid lies aft of the base plane x = length by more
# than 1e-4 length (a tail fairing), or within that distance of it with a normal along +x (flat base, fin
# trailing edges flush with the base). The marker `farfield` holds the box faces. No symmetry marker.
import json
import math
import sys
import time

import numpy as np
import gmsh

p = json.load(open(sys.argv[1]))
t0 = time.time()
gmsh.initialize()
gmsh.option.setNumber("General.Terminal", 1)
# A single thread keeps the mesh (and so the CFD results) reproducible.
gmsh.option.setNumber("General.NumThreads", 1)
gmsh.option.setNumber("Mesh.MaxNumThreads1D", 1)
gmsh.option.setNumber("Mesh.MaxNumThreads2D", 1)
gmsh.option.setNumber("Mesh.MaxNumThreads3D", 1)
gmsh.model.add("rocket_stl")

L = p["length"]
R = p["farfield_radius"]
x0 = p["center_x"]
h0 = p["wall_size"]
hmax = p["max_size"]
grow = p["growth"]
mode = p["stl_remesh"]
remesh = mode == "classify"
if mode not in ("classify", "none"):
    raise SystemExit("mesh error: stl_remesh must be classify or none, got %s" % mode)


surface_file = p["stl_surface"]
gmsh.merge(surface_file)
if remesh:
    try:
        gmsh.model.mesh.classifySurfaces(40 * math.pi / 180, True, True, math.pi)
        gmsh.model.mesh.createGeometry()
    except Exception as e:  # noqa: BLE001
        raise SystemExit(
            "mesh error: gmsh cannot parametrise the united surface (%s). Sharp apexes and facets of very poor quality "
            "do that; stl_remesh = \"none\" meshes the triangles as they are (the fins are then under-resolved)." % e
        )
gmsh.model.geo.synchronize()
body = [t for d, t in gmsh.model.getEntities(2)]
if not body:
    raise SystemExit("mesh error: the STL surface has no triangles")

# ---- farfield box with shared curves -----------------------------------------------------------------
geo = gmsh.model.geo
pts = []
for dz in (-R, R):
    for dy in (-R, R):
        for dx in (-R, R):
            pts.append(geo.addPoint(x0 + dx, dy, dz))
lines = {}


def line(a, b):
    k = (min(a, b), max(a, b))
    if k not in lines:
        lines[k] = geo.addLine(k[0], k[1])
    return lines[k] if a < b else -lines[k]


def quad(a, b, c, d):
    return geo.addPlaneSurface([geo.addCurveLoop([line(a, b), line(b, c), line(c, d), line(d, a)])])


q = pts
faces = [
    quad(q[0], q[1], q[3], q[2]), quad(q[4], q[6], q[7], q[5]), quad(q[0], q[4], q[5], q[1]),
    quad(q[2], q[3], q[7], q[6]), quad(q[0], q[2], q[6], q[4]), quad(q[1], q[5], q[7], q[3]),
]
vol = geo.addVolume([geo.addSurfaceLoop(faces), geo.addSurfaceLoop(body)])
geo.synchronize()
gmsh.model.addPhysicalGroup(2, body, name="wall")
gmsh.model.addPhysicalGroup(2, faces, name="farfield")
gmsh.model.addPhysicalGroup(3, [vol], name="fluid")

# ---- size field --------------------------------------------------------------------------------------
f = gmsh.model.mesh.field
f.add("Distance", 1)
f.setNumbers(1, "SurfacesList", body)
f.setNumber(1, "Sampling", 200)
f.add("Threshold", 2)
f.setNumber(2, "InField", 1)
f.setNumber(2, "SizeMin", h0)
f.setNumber(2, "SizeMax", hmax)
f.setNumber(2, "DistMin", 0.0)
f.setNumber(2, "DistMax", (hmax - h0) / (grow - 1.0))
f.setAsBackgroundMesh(2)
gmsh.option.setNumber("Mesh.MeshSizeFromPoints", 0)
gmsh.option.setNumber("Mesh.MeshSizeExtendFromBoundary", 0)
if remesh:
    gmsh.option.setNumber("Mesh.MeshSizeFromCurvature", int(p["curvature_elements"]))
    gmsh.option.setNumber("Mesh.MeshSizeMin", h0 / p["min_size_divisor"])
    gmsh.option.setNumber("Mesh.MeshSizeMax", hmax)
    gmsh.option.setNumber("Mesh.Algorithm", 6)

# ---- volume mesh -------------------------------------------------------------------------------------
t1 = time.time()
# 10 = HXT, 1 = Delaunay. The caller (mesh_stl.rs) runs the script again with 1 when HXT fails: a failed HXT run
# leaves the gmsh model in a state that a Delaunay retry in the same process does not survive.
gmsh.option.setNumber("Mesh.Algorithm3D", int(p["algorithm3d"]))
try:
    gmsh.model.mesh.generate(3)
except Exception as e:  # noqa: BLE001
    raise SystemExit("mesh error: 3D meshing failed (algorithm %d): %s" % (int(p["algorithm3d"]), e))
t2 = time.time()

# ---- extraction --------------------------------------------------------------------------------------
etypes, etags, enodes = gmsh.model.mesh.getElements(3)
tets = np.concatenate([np.asarray(n, dtype=np.int64).reshape(-1, 4) for t, n in zip(etypes, enodes) if t == 4]) if len(etypes) else np.zeros((0, 4), dtype=np.int64)
if len(tets) == 0:
    raise SystemExit("mesh error: the volume mesh is empty (3D meshing failed)")
ntags, ncoord, _ = gmsh.model.mesh.getNodes()
ntags = np.asarray(ntags, dtype=np.int64)
xyz = np.asarray(ncoord, dtype=np.float64).reshape(-1, 3)
lookup = np.full(int(ntags.max()) + 1, -1, dtype=np.int64)
lookup[ntags] = np.arange(len(ntags))
tets = lookup[tets]
used = np.unique(tets)


def surface_triangles(ents):
    out = []
    for t in ents:
        ty, tg, nd = gmsh.model.mesh.getElements(2, t)
        for k, n in zip(ty, nd):
            if k == 2:
                out.append(lookup[np.asarray(n, dtype=np.int64).reshape(-1, 3)])
    return np.concatenate(out) if out else np.zeros((0, 3), dtype=np.int64)


wall_all = surface_triangles(body)
far = surface_triangles(faces)
# Positive orientation of the tetrahedra.
a, b, c, d = (xyz[tets[:, i]] for i in range(4))
vol6 = np.einsum("ij,ij->i", np.cross(b - a, c - a), d - a)
neg = vol6 < 0
tets[neg, 2], tets[neg, 3] = tets[neg, 3].copy(), tets[neg, 2].copy()
# wall / base split.
tol = 1e-4 * L
v0, v1, v2 = (xyz[wall_all[:, i]] for i in range(3))
cen = (v0 + v1 + v2) / 3.0
nrm = np.cross(v1 - v0, v2 - v0)
nlen = np.linalg.norm(nrm, axis=1)
flat = (cen[:, 0] >= L - tol) & (np.abs(nrm[:, 0]) > 0.9 * nlen)
is_base = (cen[:, 0] > L + tol) | flat
wall_tri, base_tri = wall_all[~is_base], wall_all[is_base]
if len(base_tri) == 0:
    raise SystemExit("mesh error: no base facets (aft-facing facets at x = length) found; the STL must end in a flat base or use base_treatment = fairing")
print("facets: wall %d, base %d, farfield %d" % (len(wall_tri), len(base_tri), len(far)))
if len(wall_tri) == 0 or len(far) == 0:
    raise SystemExit("mesh error: could not classify the boundary facets")

remap = np.full(len(xyz), -1, dtype=np.int64)
remap[used] = np.arange(len(used))
with open(p["out_su2"], "w") as fh:
    fh.write("NDIME= 3\nNELEM= %d\n" % len(tets))
    t_new = remap[tets]
    fh.write("\n".join("10 %d %d %d %d %d" % (r[0], r[1], r[2], r[3], i) for i, r in enumerate(t_new.tolist())))
    fh.write("\nNPOIN= %d\n" % len(used))
    fh.write("\n".join("%.12g %.12g %.12g %d" % (r[0], r[1], r[2], i) for i, r in enumerate(xyz[used].tolist())))
    marks = [("wall", wall_tri), ("farfield", far)] + ([("base", base_tri)] if len(base_tri) else [])
    fh.write("\nNMARK= %d\n" % len(marks))
    for name, tri in marks:
        fh.write("MARKER_TAG= %s\nMARKER_ELEMS= %d\n" % (name, len(tri)))
        fh.write("\n".join("5 %d %d %d" % (r[0], r[1], r[2]) for r in remap[tri].tolist()))
        fh.write("\n")

# ---- statistics --------------------------------------------------------------------------------------
q_min, q_sum, q_n, q_bad = 1.0, 0.0, 0, 0
for et, tags in zip(etypes, etags):
    qs = gmsh.model.mesh.getElementQualities(list(tags), "minSICN")
    q_min = min(q_min, float(min(qs)))
    q_sum += float(sum(qs))
    q_n += len(qs)
    q_bad += int(sum(1 for v in qs if v < 0.05))
stats = {
    "nodes": int(len(used)),
    "cells": int(len(tets)),
    "by_type": {"Tetrahedron 4": int(len(tets))},
    "quality_sicn_min": q_min,
    "quality_sicn_mean": q_sum / max(q_n, 1),
    "cells_sicn_below_0.05": q_bad,
    "wall_triangles": int(len(wall_tri) + len(base_tri)),
    "seconds_setup": t1 - t0,
    "seconds_mesh": t2 - t1,
}
json.dump(stats, open(p["stats_json"], "w"), indent=1)
print("MESH_OK nodes=%d cells=%d minSICN=%.4f volume %.1fs" % (stats["nodes"], stats["cells"], q_min, t2 - t1))
gmsh.finalize()
