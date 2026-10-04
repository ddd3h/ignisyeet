# SPDX-License-Identifier: GPL-3.0-or-later
# IgnisYeet CFD volume mesher (generated into the case work directory and run with the gmsh python).
# Usage: python mesh_gmsh.py params.json
#
# Builds the rocket as an OpenCASCADE solid (body of revolution united with the fins), cuts it out
# of the farfield ball (or half ball with the y = 0 symmetry plane), meshes the volume with
# tetrahedra graded away from the wall and writes an SU2 mesh with the markers wall, base, farfield,
# symmetry. (No prismatic boundary layer: gmsh 4.15 can extrude one from OpenCASCADE surfaces only in 2D.)
import json
import math
import sys
import time

import gmsh

p = json.load(open(sys.argv[1]))
t0 = time.time()
gmsh.initialize()
gmsh.option.setNumber("General.Terminal", 1)
# A single thread keeps the mesh (and so the CFD results) reproducible: with several threads the
# surface and HXT volume meshes differ from run to run. Costs about 20 s for 600 k cells.
nthreads = 1 if p["threads"] else 1
gmsh.option.setNumber("General.NumThreads", nthreads)
gmsh.option.setNumber("Mesh.MaxNumThreads1D", nthreads)
gmsh.option.setNumber("Mesh.MaxNumThreads2D", nthreads)
gmsh.option.setNumber("Mesh.MaxNumThreads3D", nthreads)
gmsh.model.add("rocket")
occ = gmsh.model.occ

L = p["length"]
R = p["farfield_radius"]
x0 = p["center_x"]
half = bool(p["symmetry"])
# z_mirror: mesh only the quarter y >= 0, z >= 0 and mirror it about z = 0 afterwards, so the half-model mesh
# is exactly symmetric about the x-y plane (a Delaunay mesh of the half model is not).
zmirror = bool(p["z_mirror"]) and half
h0 = p["wall_size"]
hmax = p["max_size"]
grow = p["growth"]
fins = p["fins"]

# ---- rocket solid ----------------------------------------------------------------------------------
# Body of revolution: the profile lies in a half plane at azimuth `seam`, away from the fin planes
# and the symmetry plane so that no surface seam coincides with y = 0.
nf = fins["count"] if fins else 0
roll = math.radians(p["fin_roll_deg"])
seam = math.radians(45.0 if nf == 0 else 90.0 + 180.0 / nf) + roll
if abs(math.sin(seam)) < 0.2:
    seam += math.radians(30.0)
cs, sn = math.cos(seam), math.sin(seam)
prof = p["profile"]
pts = [occ.addPoint(x, r * cs, r * sn) for x, r in prof]
axis0 = pts[0] if abs(prof[0][1]) < 1e-12 else occ.addPoint(prof[0][0], 0, 0)
axis1 = pts[-1] if abs(prof[-1][1]) < 1e-12 else occ.addPoint(prof[-1][0], 0, 0)
segs = [occ.addLine(pts[i], pts[i + 1]) for i in range(len(pts) - 1)]
if axis1 != pts[-1]:
    segs.append(occ.addLine(pts[-1], axis1))
segs.append(occ.addLine(axis1, axis0))
if abs(prof[0][1]) > 1e-12:
    segs.append(occ.addLine(axis0, pts[0]))
loop = occ.addWire(segs)
face = occ.addPlaneSurface([loop])
rev = occ.revolve([(2, face)], 0, 0, 0, 1, 0, 0, 2 * math.pi)
bodies = [t for d, t in rev if d == 3]
occ.remove([(2, face)], recursive=False)
parts = [(3, bodies[0])]


def lens(x_le, x_te, r_pos, phi, thick):
    """Biconvex section (parabolic thickness) in the plane at distance r_pos, azimuth phi."""
    u = (math.cos(phi), math.sin(phi))  # span direction in (y, z)
    w = (-math.sin(phi), math.cos(phi))  # thickness direction
    n = 8

    def pt(xi, side):
        x = x_le + xi * (x_te - x_le)
        h = 0.5 * thick * 4.0 * xi * (1.0 - xi) * side
        return occ.addPoint(x, r_pos * u[0] + h * w[0], r_pos * u[1] + h * w[1])

    le = pt(0.0, 1)
    te = pt(1.0, 1)
    up = [le] + [pt(i / n, 1) for i in range(1, n)] + [te]
    lo = [le] + [pt(i / n, -1) for i in range(1, n)] + [te]
    c1 = occ.addSpline(up)
    c2 = occ.addSpline(lo)
    return occ.addWire([c1, c2])


if fins:
    f = fins
    for k in range(nf):
        phi = math.radians(90.0) + roll + 2 * math.pi * k / nf
        r_in = 0.5 * f["inner_radius"]
        r_tip = f["body_radius"] + f["span"]
        w1 = lens(f["x_le"], f["x_te"], r_in, phi, f["thickness"])
        w2 = lens(f["x_le"] + f["sweep"], f["x_le"] + f["sweep"] + f["tip_chord"], r_tip, phi, f["thickness"])
        fin = occ.addThruSections([w1, w2], makeSolid=True, makeRuled=True)
        parts.append(fin[0] if not isinstance(fin[0], int) else (3, fin[0]))
    if len(parts) > 1:
        u_out, _ = occ.fuse([parts[0]], parts[1:], removeObject=True, removeTool=True)
        parts = [t for t in u_out if t[0] == 3]
    occ.synchronize()

if half:
    # Half ball y >= 0 as a sphere sector (azimuth 0..pi about the z axis): no boolean on the sphere.
    # Quarter ball (z >= 0 too) for z_mirror: latitude 0..pi/2.
    domain = [(3, occ.addSphere(x0, 0, 0, R, angle1=0.0 if zmirror else -math.pi / 2, angle2=math.pi / 2, angle3=math.pi))]
else:
    domain = [(3, occ.addSphere(x0, 0, 0, R))]
fluid, _ = occ.cut(domain, parts)
occ.synchronize()
vols = [t for d, t in gmsh.model.getEntities(3)]
if len(vols) != 1:
    raise SystemExit("mesh error: boolean operations produced %d volumes (expected 1); the geometry may be invalid" % len(vols))
vol = vols[0]

# ---- classify the boundary faces -------------------------------------------------------------------
tol = 1e-4 * L  # bounding boxes are enlarged by the OCC tolerance
wallsurf, cap, far, symm, symz = [], [], [], [], []
for d, t in gmsh.model.getEntities(2):
    b = gmsh.model.getBoundingBox(2, t)  # xmin ymin zmin xmax ymax zmax
    if half and b[4] - b[1] < tol and abs(b[1]) < tol:
        symm.append(t)
    elif zmirror and b[5] - b[2] < tol and abs(b[2]) < tol:
        symz.append(t)
    elif b[3] - b[0] > 0.5 * R:
        far.append(t)
    elif b[0] > L - tol:
        cap.append(t)  # flat base cap or tail fairing (everything aft of the base plane)
    else:
        wallsurf.append(t)
print("faces: wall %d, base %d, farfield %d, symmetry %d" % (len(wallsurf), len(cap), len(far), len(symm)))
if not wallsurf or not far or not cap or (half and not symm):
    raise SystemExit("mesh error: could not classify the boundary faces")

# ---- size field ------------------------------------------------------------------------------------
f = gmsh.model.mesh.field
f.add("Distance", 1)
f.setNumbers(1, "SurfacesList", wallsurf + cap)
f.setNumber(1, "Sampling", 200)
f.add("Threshold", 2)
f.setNumber(2, "InField", 1)
f.setNumber(2, "SizeMin", h0)
f.setNumber(2, "SizeMax", hmax)
f.setNumber(2, "DistMin", 0.0)
f.setNumber(2, "DistMax", (hmax - h0) / (grow - 1.0))
f.setAsBackgroundMesh(2)
gmsh.option.setNumber("Mesh.MeshSizeFromPoints", 0)
gmsh.option.setNumber("Mesh.MeshSizeFromCurvature", int(p["curvature_elements"]))
gmsh.option.setNumber("Mesh.MeshSizeExtendFromBoundary", 0)
gmsh.option.setNumber("Mesh.MeshSizeMin", h0 / p["min_size_divisor"])
gmsh.option.setNumber("Mesh.MeshSizeMax", hmax)
gmsh.option.setNumber("Mesh.Algorithm", 6)

# ---- physical groups -------------------------------------------------------------------------------
pg = gmsh.model.addPhysicalGroup
gmsh.model.setPhysicalName(2, pg(2, wallsurf), "wall")
gmsh.model.setPhysicalName(2, pg(2, cap), "base")
gmsh.model.setPhysicalName(2, pg(2, far), "farfield")
if half:
    gmsh.model.setPhysicalName(2, pg(2, symm), "symmetry")
if zmirror:
    gmsh.model.setPhysicalName(2, pg(2, symz), "symz")
gmsh.model.setPhysicalName(3, pg(3, [vol]), "fluid")

# ---- volume mesh -----------------------------------------------------------------------------------
t1 = time.time()
gmsh.option.setNumber("Mesh.Algorithm3D", 10)  # HXT (parallel Delaunay)
gmsh.model.mesh.generate(2)
t_surf = time.time()
try:
    gmsh.model.mesh.generate(3)
except Exception as e:  # noqa: BLE001
    print("HXT failed (%s); retrying with Delaunay" % e)
    gmsh.model.mesh.clear()
    gmsh.option.setNumber("Mesh.Algorithm3D", 1)
    gmsh.model.mesh.generate(2)
    gmsh.model.mesh.generate(3)
t2 = time.time()
gmsh.option.setNumber("Mesh.Binary", 0)
gmsh.option.setNumber("Mesh.SaveAll", 0)
gmsh.write(p["out_su2"])


def mirror_su2(path, tol):
    """Mirrors a quarter mesh (z >= 0) about z = 0 into the half mesh; the marker `symz` is dropped."""
    lines = open(path).read().split("\n")
    i = 0
    elems, nodes, marks = [], [], []
    while i < len(lines):
        ln = lines[i]
        if ln.startswith("NELEM="):
            n = int(ln.split("=")[1])
            elems = [list(map(int, l.split())) for l in lines[i + 1:i + 1 + n]]
            i += n
        elif ln.startswith("NPOIN="):
            n = int(ln.split("=")[1].split()[0])
            nodes = [list(map(float, l.split()[:3])) for l in lines[i + 1:i + 1 + n]]
            i += n
        elif ln.startswith("MARKER_TAG="):
            tag = ln.split("=")[1].strip()
            n = int(lines[i + 1].split("=")[1])
            marks.append((tag, [list(map(int, l.split())) for l in lines[i + 2:i + 2 + n]]))
            i += 1 + n
        i += 1
    nn = len(nodes)
    m = list(range(nn))
    extra = []
    for k, (x, y, z) in enumerate(nodes):
        if abs(z) > tol:
            m[k] = nn + len(extra)
            extra.append((x, y, -z))
    out = ["NDIME= 3"]
    # Tetrahedra (type 10): swapping two vertices keeps the orientation of the mirrored cells positive.
    allel = elems + [[e[0], m[e[1]], m[e[3]], m[e[2]], m[e[4]]] + [0] for e in elems]
    out.append("NELEM= %d" % len(allel))
    out += ["%d %d %d %d %d %d" % (e[0], e[1], e[2], e[3], e[4], k) for k, e in enumerate(allel)]
    out.append("NPOIN= %d" % (nn + len(extra)))
    allp = nodes + extra
    out += ["%.12g %.12g %.12g %d" % (x, y, z, k) for k, (x, y, z) in enumerate(allp)]
    keep = [(t, e) for t, e in marks if t != "symz"]
    out.append("NMARK= %d" % len(keep))
    for t, e in keep:
        both = e + [[x[0], m[x[1]], m[x[3]], m[x[2]]] for x in e]
        out.append("MARKER_TAG= " + t)
        out.append("MARKER_ELEMS= %d" % len(both))
        out += ["%d %d %d %d" % tuple(x) for x in both]
    open(path, "w").write("\n".join(out) + "\n")
    return len(allel), nn + len(extra)


mirrored = mirror_su2(p["out_su2"], 1e-9 * L) if zmirror else None

# ---- statistics ------------------------------------------------------------------------------------
ntags = gmsh.model.mesh.getNodes()[0]
etypes, etags, _ = gmsh.model.mesh.getElements(3)
counts = {}
q_min, q_sum, q_n, q_bad = 1.0, 0.0, 0, 0
for et, tags in zip(etypes, etags):
    name = gmsh.model.mesh.getElementProperties(et)[0]
    counts[name] = counts.get(name, 0) + len(tags)
    q = gmsh.model.mesh.getElementQualities(list(tags), "minSICN")
    q_min = min(q_min, float(min(q)))
    q_sum += float(sum(q))
    q_n += len(q)
    q_bad += int(sum(1 for v in q if v < 0.05))
nwall = 0
for t in wallsurf:
    e = gmsh.model.mesh.getElements(2, t)
    nwall += sum(len(x) for x in e[1])
stats = {
    "nodes": int(len(ntags)),
    "cells": int(sum(counts.values())),
    "by_type": counts,
    "quality_sicn_min": q_min,
    "quality_sicn_mean": q_sum / max(q_n, 1),
    "cells_sicn_below_0.05": q_bad,
    "wall_triangles": int(nwall),
    "seconds_setup": t1 - t0,
    "seconds_mesh": t2 - t1,
}
if mirrored:
    stats["cells"], stats["nodes"], stats["wall_triangles"] = mirrored[0], mirrored[1], 2 * nwall
    stats["by_type"] = {"Tetrahedron 4": mirrored[0]}
json.dump(stats, open(p["stats_json"], "w"), indent=1)
print("MESH_OK nodes=%d cells=%d minSICN=%.4f surface %.1fs volume %.1fs" % (stats["nodes"], stats["cells"], q_min, t_surf - t1, t2 - t_surf))
gmsh.finalize()
