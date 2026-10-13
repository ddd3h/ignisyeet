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
# trailing edges flush with the base). The marker `farfield` holds the box faces.
#
# Symmetry: when the surface is mirror symmetric (verified by the caller, see symmetry.rs) `stl_surface` is
# the union clipped at y = 0 (half: `symmetry` or `y_mirror`) and additionally at z = 0 (quarter: `z_mirror`).
# The clipped surface is open along the cut; its boundary curves (classifySurfaces splits them at the
# corners, in particular where the cut crosses the axis) close the symmetry faces of the box, which are
# plane surfaces whose loops mix box lines and those discrete curves. The faces get the markers `symmetry`
# (y = 0) and `symz` (z = 0). The mesh is then finished like the CAD mesh: the quarter is mirrored about z = 0
# (an exactly z-symmetric half mesh, so -alpha is the mirror image of +alpha) and the half is kept with its
# `symmetry` marker (`symmetry`) or mirrored about y = 0 to the full domain (`y_mirror`); the interior
# symmetry markers are dropped.
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
    raise SystemExit("[IY-F050] mesh error: stl_remesh must be classify or none, got %s" % mode)
half = bool(p["symmetry"]) or bool(p["y_mirror"])
quarter = bool(p["z_mirror"]) and half
tol_plane = 1e-7 * L


surface_file = p["stl_surface"]
gmsh.merge(surface_file)
if remesh:
    try:
        # Boundary curves are split where the line direction changes by more than 0.1 rad (about 6 deg; the cut curves of
        # a symmetric half / quarter must end at the axis points, which classifySurfaces does not split at otherwise);
        # the closed surface has no boundary and is classified as before.
        curve_angle = 0.1 if half else math.pi
        gmsh.model.mesh.classifySurfaces(40 * math.pi / 180, True, True, curve_angle)
        gmsh.model.mesh.createGeometry()
    except Exception as e:  # noqa: BLE001
        raise SystemExit(
            "[IY-F051] mesh error: gmsh cannot parametrise the united surface (%s). Sharp apexes and facets of very poor quality "
            "do that; stl_remesh = \"none\" meshes the triangles as they are (the fins are then under-resolved)." % e
        )
gmsh.model.geo.synchronize()
body = [t for d, t in gmsh.model.getEntities(2)]
if not body:
    raise SystemExit("[IY-F052] mesh error: the STL surface has no triangles")

# ---- farfield box with shared curves -----------------------------------------------------------------
geo = gmsh.model.geo
xa, xb = x0 - R, x0 + R
ylo = 0.0 if half else -R
zlo = 0.0 if quarter else -R
pts = {}
for ix, x in enumerate((xa, xb)):
    for iy, y in enumerate((ylo, R)):
        for iz, z in enumerate((zlo, R)):
            pts[(ix, iy, iz)] = geo.addPoint(x, y, z)
lines = {}


def line(a, b):
    k = (min(a, b), max(a, b))
    if k not in lines:
        lines[k] = geo.addLine(k[0], k[1])
    return lines[k] if a < b else -lines[k]


def loop_of(*vertex_chain):
    """Closed curve loop through the box points of the chain (tags)."""
    n = len(vertex_chain)
    return [line(vertex_chain[i], vertex_chain[(i + 1) % n]) for i in range(n)]


def quad(a, b, c, d):
    return geo.addPlaneSurface([geo.addCurveLoop(loop_of(a, b, c, d))])


q = lambda ix, iy, iz: pts[(ix, iy, iz)]  # noqa: E731
sym_faces = []   # (face, marker name)
cut_y, cut_z = [], []


def boundary_curves():
    """Open boundary curves of the clipped wall (one adjacent surface) split into the plane y = 0 and the plane z = 0."""
    cy, cz = [], []
    for d, c in gmsh.model.getEntities(1):
        up, _ = gmsh.model.getAdjacencies(1, c)
        if len(up) != 1:
            continue
        bb = gmsh.model.getBoundingBox(1, c)
        in_y = bb[1] >= -tol_plane and bb[4] <= tol_plane
        in_z = bb[2] >= -tol_plane and bb[5] <= tol_plane
        if in_y and not in_z:
            cy.append(c)
        elif in_z and not in_y:
            cz.append(c)
        else:
            raise SystemExit(
                "[IY-F053] mesh error: a boundary curve of the clipped surface (bounding box %s) lies in neither or both symmetry planes" % str(bb)
            )
    return cy, cz


def oriented_ends(c):
    b = gmsh.model.getBoundary([(1, c)], oriented=True)
    return abs(b[0][1]), abs(b[1][1])


def chain_loops(curves):
    """Orders `curves` into closed loops [(signed tags)] by their end points."""
    ends = {c: oriented_ends(c) for c in curves}
    rest = set(curves)
    loops = []
    while rest:
        c0 = min(rest)
        rest.remove(c0)
        loop, start, cur = [c0], ends[c0][0], ends[c0][1]
        while cur != start:
            for c in sorted(rest):
                s, e = ends[c]
                if s == cur:
                    loop.append(c); cur = e; rest.remove(c); break
                if e == cur:
                    loop.append(-c); cur = s; rest.remove(c); break
            else:
                raise SystemExit("[IY-F054] mesh error: the cut of the clipped surface is not a closed curve")
        loops.append(loop)
    return loops


def chain_open(curves, point_ok):
    """Orders `curves` into one open chain between two end points that satisfy `point_ok`; returns (signed tags, first, last)."""
    ends = {c: oriented_ends(c) for c in curves}
    count = {}
    for s, e in ends.values():
        count[s] = count.get(s, 0) + 1
        count[e] = count.get(e, 0) + 1
    free = [pt for pt, n in count.items() if n == 1]
    if len(free) != 2 or not all(point_ok(pt) for pt in free):
        raise SystemExit("[IY-F055] mesh error: the cut of the clipped surface is not one open curve between two points on the axis (end points %s)" % free)
    first, last = sorted(free, key=lambda pt: gmsh.model.getValue(0, pt, [])[0])
    chain, cur, rest = [], first, set(curves)
    while cur != last:
        for c in sorted(rest):
            s, e = ends[c]
            if s == cur:
                chain.append(c); cur = e; rest.remove(c); break
            if e == cur:
                chain.append(-c); cur = s; rest.remove(c); break
        else:
            raise SystemExit("[IY-F056] mesh error: broken cut curve")
    return chain, first, last


if not half:
    faces = [
        quad(q(0, 0, 0), q(0, 1, 0), q(0, 1, 1), q(0, 0, 1)), quad(q(1, 0, 0), q(1, 0, 1), q(1, 1, 1), q(1, 1, 0)),
        quad(q(0, 0, 0), q(1, 0, 0), q(1, 0, 1), q(0, 0, 1)), quad(q(0, 1, 0), q(0, 1, 1), q(1, 1, 1), q(1, 1, 0)),
        quad(q(0, 0, 0), q(0, 1, 0), q(1, 1, 0), q(1, 0, 0)), quad(q(0, 0, 1), q(1, 0, 1), q(1, 1, 1), q(0, 1, 1)),
    ]
    far = list(faces)
else:
    cut_y, cut_z = boundary_curves()
    if not cut_y or (quarter and not cut_z) or (not quarter and cut_z):
        raise SystemExit("[IY-F057] mesh error: unexpected cut curves of the clipped surface (y = 0: %d, z = 0: %d)" % (len(cut_y), len(cut_z)))
    print("cut curves: y = 0 %d, z = 0 %d" % (len(cut_y), len(cut_z)))
    far = []
    # x = xa and x = xb faces, y = R and z = R faces: plain quadrilaterals over the quarter / half box.
    far.append(quad(q(0, 0, 0), q(0, 1, 0), q(0, 1, 1), q(0, 0, 1)))
    far.append(quad(q(1, 0, 0), q(1, 0, 1), q(1, 1, 1), q(1, 1, 0)))
    far.append(quad(q(0, 1, 0), q(0, 1, 1), q(1, 1, 1), q(1, 1, 0)))
    far.append(quad(q(0, 0, 1), q(1, 0, 1), q(1, 1, 1), q(0, 1, 1)))
    if quarter:
        axis_ok = lambda pt: all(abs(c) <= tol_plane for c in gmsh.model.getValue(0, pt, [])[1:])  # noqa: E731
        chain_y, ny, ty = chain_open(cut_y, axis_ok)
        chain_z, nz, tz = chain_open(cut_z, axis_ok)
        if (ny, ty) != (nz, tz):
            raise SystemExit("[IY-F058] mesh error: the cuts at y = 0 and z = 0 do not meet at the same points of the axis")
        A, B = q(0, 0, 0), q(1, 0, 0)
        # y = 0 face (z >= 0): axis from the corner to the nose, the cut curve, the axis to the far corner, then the rectangle.
        loop = [line(A, ny)] + chain_y + [line(ty, B)] + [line(B, q(1, 0, 1)), line(q(1, 0, 1), q(0, 0, 1)), line(q(0, 0, 1), A)]
        fy = geo.addPlaneSurface([geo.addCurveLoop(loop)])
        loop = [line(A, ny)] + chain_z + [line(ty, B)] + [line(B, q(1, 1, 0)), line(q(1, 1, 0), q(0, 1, 0)), line(q(0, 1, 0), A)]
        fz = geo.addPlaneSurface([geo.addCurveLoop(loop)])
        sym_faces = [(fy, "symmetry"), (fz, "symz")]
    else:
        outer = geo.addCurveLoop(loop_of(q(0, 0, 0), q(1, 0, 0), q(1, 0, 1), q(0, 0, 1)))
        holes = [geo.addCurveLoop(lp) for lp in chain_loops(cut_y)]
        fy = geo.addPlaneSurface([outer] + holes)
        sym_faces = [(fy, "symmetry")]
        # The z = -R face of the half box.
        far.append(quad(q(0, 0, 0), q(1, 0, 0), q(1, 1, 0), q(0, 1, 0)))
all_faces = far + [f for f, _ in sym_faces]
vol = geo.addVolume([geo.addSurfaceLoop(all_faces), geo.addSurfaceLoop(body)])
geo.synchronize()
gmsh.model.addPhysicalGroup(2, body, name="wall")
gmsh.model.addPhysicalGroup(2, far, name="farfield")
for f, name in sym_faces:
    gmsh.model.addPhysicalGroup(2, [f], name=name)
gmsh.model.addPhysicalGroup(3, [vol], name="fluid")

# ---- size field --------------------------------------------------------------------------------------
def profile_radius(x):
    """Body radius of the CFD profile (tail fairing included) at station x."""
    prof = p["profile"]
    if x <= prof[0][0]:
        return prof[0][1]
    for (xa, ra), (xb, rb) in zip(prof[:-1], prof[1:]):
        if x <= xb:
            return ra + (rb - ra) * (x - xa) / max(xb - xa, 1e-300)
    return prof[-1][1]


def fin_surfaces(surfaces):
    """Classified surfaces whose area centroid (of the STL triangles they still carry) lies outside the body by more than
    a tenth of the fin span."""
    out = []
    for t in surfaces:
        tags, coord, _ = gmsh.model.mesh.getNodes(2, t, includeBoundary=True)
        ety, _, enodes = gmsh.model.mesh.getElements(2, t)
        tri = [np.asarray(n, np.int64).reshape(-1, 3) for e, n in zip(ety, enodes) if e == 2]
        if not tri or not len(tags):
            continue
        tri = np.vstack(tri)
        look = {int(k): i for i, k in enumerate(tags)}
        xyz = np.asarray(coord, float).reshape(-1, 3)
        v = xyz[np.vectorize(look.get)(tri)]
        area = 0.5 * np.linalg.norm(np.cross(v[:, 1] - v[:, 0], v[:, 2] - v[:, 0]), axis=1)
        if area.sum() <= 0.0:
            continue
        c = (v.mean(axis=1) * area[:, None]).sum(axis=0) / area.sum()
        if math.hypot(c[1], c[2]) > profile_radius(c[0]) + 0.1 * p["fins"]["span"]:
            out.append(t)
    return out


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
hmin = h0 / p["min_size_divisor"]
# Fin faces (classify only, fin_size < wall_size): a second threshold of the distance to the fin surfaces, fin_size on
# them, graded by `growth` (the minimum of both fields). A fin surface is a classified surface whose area centroid lies
# outside the body profile by more than a tenth of the span (body surfaces have theirs inside the body).
hf = min(p["fin_size"], h0)
finf = fin_surfaces(body) if remesh and p["fins"] and hf < h0 * (1.0 - 1e-9) else []
if finf:
    f.add("Distance", 3)
    f.setNumbers(3, "SurfacesList", finf)
    f.setNumber(3, "Sampling", 200)
    f.add("Threshold", 4)
    f.setNumber(4, "InField", 3)
    f.setNumber(4, "SizeMin", hf)
    f.setNumber(4, "SizeMax", hmax)
    f.setNumber(4, "DistMin", 0.0)
    f.setNumber(4, "DistMax", (hmax - hf) / (grow - 1.0))
    f.add("Min", 5)
    f.setNumbers(5, "FieldsList", [2, 4])
    f.setAsBackgroundMesh(5)
    hmin = min(hmin, hf)
    print("fin surfaces: %d of %d wall surfaces meshed at fin_size %.4g m (wall_size %.4g m)" % (len(finf), len(body), hf, h0))
else:
    if remesh and p["fins"] and hf < h0 * (1.0 - 1e-9):
        print("fin surfaces: none identified; fin_size %.4g m not applied" % hf)
    f.setAsBackgroundMesh(2)
gmsh.option.setNumber("Mesh.MeshSizeFromPoints", 0)
gmsh.option.setNumber("Mesh.MeshSizeExtendFromBoundary", 0)
if remesh:
    gmsh.option.setNumber("Mesh.MeshSizeFromCurvature", int(p["curvature_elements"]))
    gmsh.option.setNumber("Mesh.MeshSizeMin", hmin)
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
    raise SystemExit("[IY-F024] mesh error: 3D meshing failed (algorithm %d): %s" % (int(p["algorithm3d"]), e))
t2 = time.time()

# ---- extraction --------------------------------------------------------------------------------------
etypes, etags, enodes = gmsh.model.mesh.getElements(3)
tets = np.concatenate([np.asarray(n, dtype=np.int64).reshape(-1, 4) for t, n in zip(etypes, enodes) if t == 4]) if len(etypes) else np.zeros((0, 4), dtype=np.int64)
if len(tets) == 0:
    raise SystemExit("[IY-F023] mesh error: the volume mesh is empty (3D meshing failed)")
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
far_tri = surface_triangles(far)
sym_tri = [(name, surface_triangles([fc])) for fc, name in sym_faces]
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
    raise SystemExit("[IY-F059] mesh error: no base facets (aft-facing facets at x = length) found; the STL must end in a flat base or use base_treatment = fairing")
print("meshed domain (%s): cells %d, nodes %d" % ("quarter" if quarter else "half" if half else "full", len(tets), len(used)))
print("facets: wall %d, base %d, farfield %d" % (len(wall_tri), len(base_tri), len(far_tri)))
if len(wall_tri) == 0 or len(far_tri) == 0:
    raise SystemExit("[IY-F022] mesh error: could not classify the boundary facets")

remap = np.full(len(xyz), -1, dtype=np.int64)
remap[used] = np.arange(len(used))
# Nodes on a symmetry plane are exactly on it (the mirror copies share them).
coords = xyz[used].copy()
if half:
    coords[np.abs(coords[:, 1]) <= 1e-9 * L, 1] = 0.0
if quarter:
    coords[np.abs(coords[:, 2]) <= 1e-9 * L, 2] = 0.0
with open(p["out_su2"], "w") as fh:
    fh.write("NDIME= 3\nNELEM= %d\n" % len(tets))
    t_new = remap[tets]
    fh.write("\n".join("10 %d %d %d %d %d" % (r[0], r[1], r[2], r[3], i) for i, r in enumerate(t_new.tolist())))
    fh.write("\nNPOIN= %d\n" % len(used))
    fh.write("\n".join("%.12g %.12g %.12g %d" % (r[0], r[1], r[2], i) for i, r in enumerate(coords.tolist())))
    marks = [("wall", wall_tri), ("farfield", far_tri)] + ([("base", base_tri)] if len(base_tri) else []) + [(n, t) for n, t in sym_tri]
    fh.write("\nNMARK= %d\n" % len(marks))
    for name, tri in marks:
        fh.write("MARKER_TAG= %s\nMARKER_ELEMS= %d\n" % (name, len(tri)))
        fh.write("\n".join("5 %d %d %d" % (r[0], r[1], r[2]) for r in remap[tri].tolist()))
        fh.write("\n")


def mirror_su2(path, axis, drop, tol):
    """Mirrors a mesh about the plane x[axis] = 0 (keeping the nodes on it) and drops the marker `drop`."""
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
    for k, c in enumerate(nodes):
        if abs(c[axis]) > tol:
            m[k] = nn + len(extra)
            r = list(c)
            r[axis] = -r[axis]
            extra.append(r)
        else:
            c[axis] = 0.0
    out = ["NDIME= 3"]
    # Tetrahedra (type 10): swapping two vertices keeps the orientation of the mirrored cells positive.
    allel = elems + [[e[0], m[e[1]], m[e[3]], m[e[2]], m[e[4]]] + [0] for e in elems]
    out.append("NELEM= %d" % len(allel))
    out += ["%d %d %d %d %d %d" % (e[0], e[1], e[2], e[3], e[4], k) for k, e in enumerate(allel)]
    out.append("NPOIN= %d" % (nn + len(extra)))
    allp = nodes + extra
    out += ["%.12g %.12g %.12g %d" % (x, y, z, k) for k, (x, y, z) in enumerate(allp)]
    keep = [(t, e) for t, e in marks if t != drop]
    out.append("NMARK= %d" % len(keep))
    for t, e in keep:
        both = e + [[x[0], m[x[1]], m[x[3]], m[x[2]]] for x in e]
        out.append("MARKER_TAG= " + t)
        out.append("MARKER_ELEMS= %d" % len(both))
        out += ["%d %d %d %d" % tuple(x) for x in both]
    open(path, "w").write("\n".join(out) + "\n")
    return len(allel), nn + len(extra)


mirrored = None
wall_mult = 1
if quarter:
    mirrored = mirror_su2(p["out_su2"], 2, "symz", 1e-9 * L)
    wall_mult *= 2
if p["y_mirror"]:
    mirrored = mirror_su2(p["out_su2"], 1, "symmetry", 1e-9 * L)
    wall_mult *= 2

# ---- statistics --------------------------------------------------------------------------------------
q_min, q_sum, q_n, q_bad = 1.0, 0.0, 0, 0
for et, tags in zip(etypes, etags):
    qs = gmsh.model.mesh.getElementQualities(list(tags), "minSICN")
    q_min = min(q_min, float(min(qs)))
    q_sum += float(sum(qs))
    q_n += len(qs)
    q_bad += int(sum(1 for v in qs if v < 0.05))
cells, nodes_out = (mirrored[0], mirrored[1]) if mirrored else (int(len(tets)), int(len(used)))
stats = {
    "nodes": int(nodes_out),
    "cells": int(cells),
    "by_type": {"Tetrahedron 4": int(cells)},
    "quality_sicn_min": q_min,
    "quality_sicn_mean": q_sum / max(q_n, 1),
    "cells_sicn_below_0.05": q_bad * wall_mult,
    "wall_triangles": int((len(wall_tri) + len(base_tri)) * wall_mult),
    "seconds_setup": t1 - t0,
    "seconds_mesh": t2 - t1,
}
json.dump(stats, open(p["stats_json"], "w"), indent=1)
print("MESH_OK nodes=%d cells=%d minSICN=%.4f volume %.1fs" % (stats["nodes"], stats["cells"], q_min, t2 - t1))
gmsh.finalize()
