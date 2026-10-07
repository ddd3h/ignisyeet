# SPDX-License-Identifier: GPL-3.0-or-later
# IgnisYeet volume mesher for a CAD B-rep surface (STEP, IGES or BREP; generated into the mesh work directory and run
# with the gmsh python). Usage: python mesh_step.py params.json
#
# Two modes (`mode` in params.json), driven by step_mesh.rs:
#
# prepare  imports the file with OpenCASCADE (lengths converted to metres from the file's unit, `unit_scale` applied
#          when the unit is overridden), sews loose faces into a solid when the file has no solid, fuses several solids
#          into one (internal faces disappear), moves the shape to the CFD frame (`affine`, the transform the STL gets)
#          and tessellates it. When the surface mesher cannot mesh the imported faces, the import is repeated with the
#          OpenCASCADE shape healing (degenerated edges, small edges and faces). Healing is not applied otherwise: on a
#          valid solid of revolution it removes the degenerated apex edge and the surface mesher then fails.
#          Writes the tessellation (`tess_stl`, binary STL, triangles grouped by face), the report (`report_json`:
#          solids, healing, faces with type, area, centroid, normal and bounding box, the smallest feature, the base
#          faces, the mirror-symmetry checks about y = 0 and z = 0) and the prepared shape (`prepared_brep`, with the
#          Euler tail fairing fused behind the base when `fairing` is set).
# mesh     reads the prepared shape and meshes the fluid around it:
#          * Euler (no `bl`): the solid is cut from the farfield (half ball y >= 0 for the half domain, quarter ball
#            y >= 0, z >= 0 for the quarter, else a box, as the CAD surface), tetrahedra graded from the wall (Distance +
#            Threshold field, growth `growth`, curvature refinement, smallest size `min_size`); the quarter is mirrored
#            about z = 0 and the half about y = 0 when `y_mirror` is set.
#          * RANS (`bl`): prismatic boundary layer from the wall surface mesh (bl_layer.py, imported from this
#            directory), outside it tetrahedra in a box farfield; half domain (`meshed` = half, open wall along y = 0)
#            or full domain (closed wall, no symmetry plane).
#          Markers: `wall` (every face of the shape that is not base), `base` (the base rule below), `farfield` and, for
#          a kept half domain, `symmetry`. Cells and markers are written by su2_writer.py, which orients the cells and
#          rejects inverted prisms, cells of non-positive volume and boundary faces without a marker.
#
# Base rule (the same as the STL surface, surface::split_base, and step_mesh::is_base_face): a face is base when its
# centroid lies aft of the base plane x = length by more than base_tol (a tail fairing, a protruding nozzle), or within
# base_tol of it and it is planar with |n_x| > 0.9 (the flat base, fin trailing edges flush with the base).
import json
import math
import os
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
# STEP / IGES lengths are converted to metres from the unit of the file (BREP has no unit).
gmsh.option.setString("Geometry.OCCTargetUnit", "M")
occ = gmsh.model.occ

L = p["length"]
base_tol = p["base_tol"]
mode = p["mode"]


def die(msg):
    raise SystemExit("mesh error: " + msg)


def face_plane(f):
    """(planar, unit normal at the middle of the parameter range) of face `f`. A face is planar when its normals at a
    5 x 5 grid of parameters agree within 1e-6 (a plane exported as a B-spline surface counts; the type alone does not)."""
    lo, hi = gmsh.model.getParametrizationBounds(2, f)
    uv = []
    for a in np.linspace(0.05, 0.95, 5):
        for b in np.linspace(0.05, 0.95, 5):
            uv += [lo[0] + a * (hi[0] - lo[0]), lo[1] + b * (hi[1] - lo[1])]
    n = np.asarray(gmsh.model.getNormal(f, uv), float).reshape(-1, 3)
    n /= np.maximum(np.linalg.norm(n, axis=1), 1e-300)[:, None]
    planar = gmsh.model.getType(2, f) == "Plane" or bool((np.abs(n @ n[12]) > 1.0 - 1e-6).all())
    return planar, n[12]


def is_base(cx, planar, nx):
    """The base rule (see the header)."""
    return bool(cx > L + base_tol or (abs(cx - L) <= base_tol and planar and abs(nx) > 0.9))


def classify_body_faces(faces):
    """(wall, base) faces of the shape by the base rule."""
    wall, base = [], []
    for f in faces:
        c = occ.getCenterOfMass(2, f)
        planar, n = face_plane(f)
        (base if is_base(c[0], planar, n[0]) else wall).append(f)
    return wall, base


# ---- prepare ---------------------------------------------------------------------------------------------------------
def load(heal):
    """Imports the file into a fresh model; returns (volumes, notes)."""
    gmsh.model.add("step")
    notes = []
    dt = occ.importShapes(p["step_file"], highestDimOnly=True)
    occ.synchronize()
    n_solids = len(gmsh.model.getEntities(3))
    report["solids_in_file"] = n_solids
    report["faces_in_file"] = len(gmsh.model.getEntities(2))
    if n_solids == 0:
        # Loose faces: sew them into a shell and make a solid.
        occ.healShapes(dt, p["heal_tol"], True, True, True, True, True)
        occ.synchronize()
        if not gmsh.model.getEntities(3):
            die("%s contains no closed solid (%d faces could not be sewn into one); export the rocket as a solid body"
                % (os.path.basename(p["step_file"]), report["faces_in_file"]))
        notes.append("the file has no solid: its %d faces were sewn into a solid" % report["faces_in_file"])
    elif heal:
        occ.healShapes(gmsh.model.getEntities(3), p["heal_tol"], True, True, True, False, False)
        occ.synchronize()
        notes.append("shape healing (degenerated edges, small edges and faces, tolerance %.1e m) was applied" % p["heal_tol"])
    vols = gmsh.model.getEntities(3)
    if p["unit_scale"] != 1.0:
        s = p["unit_scale"]
        occ.dilate(vols, 0, 0, 0, s, s, s)
    occ.affineTransform(vols, p["affine"])
    occ.synchronize()
    if len(vols) > 1:
        out, _ = occ.fuse(vols[:1], vols[1:])
        occ.synchronize()
        vols = [v for v in out if v[0] == 3]
        notes.append("%d solids were fused into %d" % (n_solids, len(vols)))
        if len(vols) > 1:
            notes.append("the shape consists of %d disjoint solids; all of them are meshed as walls" % len(vols))
    return vols, notes


def tessellate():
    # At most 300 elements along the longest extent: a shape in the wrong unit (1000 times too large) is still
    # tessellated quickly, and the registration names the unit.
    b = gmsh.model.getBoundingBox(-1, -1)
    h = max(p["tess_size"], max(b[3] - b[0], b[4] - b[1], b[5] - b[2]) / 300.0)
    gmsh.option.setNumber("Mesh.MeshSizeMax", h)
    gmsh.option.setNumber("Mesh.MeshSizeMin", h / 8.0)
    gmsh.option.setNumber("Mesh.MeshSizeFromCurvature", 12)
    gmsh.option.setNumber("Mesh.Algorithm", 6)
    gmsh.model.mesh.generate(2)


def write_stl(path, tris):
    with open(path, "wb") as fh:
        fh.write(b"IgnisYeet STEP tessellation".ljust(80, b" "))
        fh.write(np.uint32(len(tris)).tobytes())
        rec = np.zeros(len(tris), dtype=[("n", "<f4", 3), ("v", "<f4", (3, 3)), ("a", "<u2")])
        rec["v"] = tris
        fh.write(rec.tobytes())


def symmetry_check(axis, faces, nodes_of, tol, max_points):
    """Mirror check of the shape about y = 0 (axis 1) or z = 0 (axis 2): interior surface nodes (on the exact surface)
    are mirrored and must lie on a face of the shape within `tol` (closest point on the face's surface, inside the
    face's trimmed region)."""
    name = "y" if axis == 1 else "z"
    total = sum(len(v) for v in nodes_of.values())
    pts = []
    for f in faces:
        c = nodes_of[f]
        if not len(c):
            continue
        k = max(8, int(math.ceil(max_points * len(c) / max(total, 1))))
        step = max(1, len(c) // k)
        pts.append(c[::step])
    pts = np.vstack(pts)
    m = pts.copy()
    m[:, axis] *= -1.0
    best = np.full(len(m), np.inf)
    for f in faces:
        b = np.asarray(gmsh.model.getBoundingBox(2, f))
        idx = np.flatnonzero(np.all((m >= b[:3] - tol) & (m <= b[3:] + tol), axis=1))
        if not len(idx):
            continue
        cp, _ = gmsh.model.getClosestPoint(2, f, m[idx].ravel().tolist())
        d = np.linalg.norm(np.asarray(cp).reshape(-1, 3) - m[idx], axis=1)
        for k in np.flatnonzero(d <= tol):
            i = idx[k]
            # The closest point is taken on the untrimmed surface: the mirrored point must lie inside the face.
            if d[k] < best[i] and gmsh.model.isInside(2, f, m[i].tolist()):
                best[i] = d[k]
    bad = best > tol
    allnodes = np.vstack([v for v in nodes_of.values() if len(v)])
    if bad.any():
        # Distance of the unmatched mirrored points to the nearest surface node (an upper bound of the distance to the surface).
        q = m[bad]
        dn = np.empty(len(q))
        for i0 in range(0, len(q), 256):
            dn[i0:i0 + 256] = np.sqrt(((q[i0:i0 + 256, None, :] - allnodes[None, :, :]) ** 2).sum(axis=2)).min(axis=1)
        best[bad] = np.maximum(dn, tol * (1.0 + 1e-9))
    worst = int(np.argmax(best))
    dmax = float(best[worst])
    reason = None
    if bad.any():
        at = m[worst]
        reason = ("the mirror image of the shape about %s = 0 is not the shape: %d of %d mirrored surface points lie off it, the farthest "
                  "%.3e m (tolerance %.1e m) at (%.4f, %.4f, %.4f)" % (name, int(bad.sum()), len(m), dmax, tol, at[0], at[1], at[2]))
    return {
        "plane": name,
        "symmetric": not bool(bad.any()),
        "max_mismatch": dmax,
        "vertices": int(len(m)),
        "vertices_matched": int((~bad).sum()),
        "triangulation_identical": False,
        "tolerance": tol,
        "reason": reason,
    }


def prepare():
    notes = []
    vols, notes = load(False)
    try:
        tessellate()
    except Exception as e:  # noqa: BLE001
        gmsh.model.remove()
        vols, notes = load(True)
        notes.append("the surface mesher failed on the imported faces (%s)" % e)
        try:
            tessellate()
        except Exception as e2:  # noqa: BLE001
            die("gmsh cannot mesh the surface of %s, even after shape healing (%s); repair the CAD model or export it again"
                % (os.path.basename(p["step_file"]), e2))
    faces = [t for _, t in gmsh.model.getBoundary(vols, oriented=False, combined=True)]
    report["notes"] = notes
    report["solids"] = len(vols)
    report["volume"] = float(sum(occ.getMass(3, t) for _, t in vols))
    report["bbox"] = list(gmsh.model.getBoundingBox(-1, -1)) if len(vols) > 1 else list(gmsh.model.getBoundingBox(3, vols[0][1]))
    info, tris, counts = [], [], []
    tags, coord, _ = gmsh.model.mesh.getNodes()
    xyz = np.asarray(coord).reshape(-1, 3)
    idx = np.zeros(int(tags.max()) + 1, np.int64)
    idx[tags] = np.arange(len(tags))
    nodes_of = {}
    min_edge = (np.inf, None)
    for _, c in gmsh.model.getBoundary([(2, f) for f in faces], oriented=False, combined=False):
        ln = occ.getMass(1, abs(c))
        if 1e-9 * L < ln < min_edge[0]:
            min_edge = (ln, abs(c))
    feature = (np.inf, None)
    for f in faces:
        typ = gmsh.model.getType(2, f)
        area = occ.getMass(2, f)
        cen = occ.getCenterOfMass(2, f)
        per = sum(occ.getMass(1, abs(c)) for _, c in gmsh.model.getBoundary([(2, f)], oriented=False, combined=False))
        width = 2.0 * area / per if per > 0 else np.inf
        if width < feature[0]:
            feature = (width, f)
        planar, n = face_plane(f)
        info.append({"tag": int(f), "type": typ, "area": float(area), "centroid": [float(c) for c in cen], "normal": [float(c) for c in n],
                     "planar": bool(planar), "width": float(width), "bbox": [float(c) for c in gmsh.model.getBoundingBox(2, f)],
                     "base": is_base(cen[0], planar, n[0])})
        t = []
        for et, en in zip(*gmsh.model.mesh.getElements(2, f)[::2]):
            if et == 2:
                t.append(idx[np.asarray(en, np.int64)].reshape(-1, 3))
        t = np.vstack(t) if t else np.zeros((0, 3), np.int64)
        tris.append(xyz[t])
        counts.append(len(t))
        _, c_in, _ = gmsh.model.mesh.getNodes(2, f, includeBoundary=False)
        nodes_of[f] = np.asarray(c_in).reshape(-1, 3)
    write_stl(p["tess_stl"], np.concatenate(tris).astype(np.float32))
    report["faces"] = info
    report["face_triangles"] = counts
    report["smallest_face_width"] = float(feature[0])
    report["smallest_face"] = feature[1]
    report["smallest_edge"] = float(min_edge[0])
    report["smallest_edge_tag"] = min_edge[1]
    report["area"] = float(sum(i["area"] for i in info))
    t1 = time.time()
    tol = p["sym_tol"]
    report["symmetry_y"] = symmetry_check(1, faces, nodes_of, tol, p["sym_points"])
    report["symmetry_z"] = symmetry_check(2, faces, nodes_of, tol, p["sym_points"])
    report["seconds_symmetry"] = time.time() - t1
    print("symmetry: y = 0 %s, z = 0 %s (%.1f s)" % (report["symmetry_y"]["symmetric"], report["symmetry_z"]["symmetric"], time.time() - t1))
    gmsh.model.mesh.clear()
    # Euler tail fairing: the cone of tail_fairing base radii to a point on the axis (the slope of the fairing of the CAD
    # and STL surfaces), cut at 10 % of the base radius with a flat cap like the STL fairing (surface::fairing_cone), behind
    # the largest flat base face and fused with the body. The full cone leaves a duplicated or degenerate triangle at the
    # apex of the OpenCASCADE cone face with every surface algorithm of gmsh.
    fair = p.get("fairing")
    report["fairing"] = None
    if fair:
        flat = [i for i in info if i["base"] and i["planar"] and abs(i["centroid"][0] - L) <= base_tol]
        if not flat:
            # Reported after the registration (a shape in the wrong unit has no base at x = L either).
            report["fairing"] = {"error": "base_treatment = \"fairing\" needs a flat aft-facing base face at x = %.4f m (the base of the STL geometry), "
                                          "but the shape has none; use base_treatment = \"flat\" (the shape is kept as it is)" % L}
            fair = None
    if fair:
        big = max(flat, key=lambda i: i["area"])
        xb = big["centroid"][0]
        bb = big["bbox"]
        rb = max(abs(bb[1]), abs(bb[2]), abs(bb[4]), abs(bb[5]))
        tl = fair["tail_fairing"] * rb
        cone = occ.addCone(xb, 0, 0, 0.9 * tl, 0, 0, rb, 0.1 * rb)
        out, _ = occ.fuse(vols, [(3, cone)])
        occ.synchronize()
        vols = [v for v in out if v[0] == 3]
        report["fairing"] = {"x": xb, "radius": rb, "length": tl, "cut_at": 0.9 * tl, "base_face_area": big["area"]}
    gmsh.write(p["prepared_brep"])
    report["seconds"] = time.time() - t0
    json.dump(report, open(p["report_json"], "w"), indent=1)
    print("PREPARE_OK faces %d, solids %d, volume %.6e m^3, smallest face width %.3e m, smallest edge %.3e m"
          % (len(faces), len(vols), report["volume"], report["smallest_face_width"], report["smallest_edge"]))
    gmsh.finalize()
    sys.exit(0)


report = {}
if mode == "prepare":
    prepare()
if mode != "mesh":
    die("unknown mode %s" % mode)

# ---- mesh ------------------------------------------------------------------------------------------------------------
gmsh.model.add("rocket_step")
R = p["farfield_radius"]
x0 = p["center_x"]
h0 = p["wall_size"]
hmax = p["max_size"]
grow = p["growth"]
hmin = p["min_size"]
meshed = p["meshed"]
half = meshed in ("half", "quarter")
quarter = meshed == "quarter"
bl = p.get("bl")
if bl:
    # The default relative tolerance (1e-8 of the 2 R domain) merges the micrometre-close nodes of the first layers.
    gmsh.option.setNumber("Geometry.Tolerance", 1e-12)
dt = occ.importShapes(p["prepared_brep"], highestDimOnly=True)
occ.synchronize()
parts = gmsh.model.getEntities(3)
if not parts:
    die("the prepared shape has no solid")


def surface_mesh():
    """Meshes the surfaces (gmsh.model.mesh.generate(2)). The frontal-Delaunay mesher can leave a duplicated triangle at
    the apex of a periodic face (a nose cone of revolution exported from CAD: HXT then fails with "duplicated facets" /
    "overlapping facets"); such faces are meshed again with the Delaunay and then the MeshAdapt algorithm."""
    for alg in (None, 5, 1):
        if alg is not None:
            for t in bad:
                gmsh.model.mesh.setAlgorithm(2, t, alg)
            gmsh.model.mesh.clear()
            print("surface mesh: duplicated triangles on surfaces %s; meshing them with algorithm %d" % (bad, alg))
        gmsh.model.mesh.generate(2)
        bad = []
        for _, t in gmsh.model.getEntities(2):
            tri = [np.sort(np.asarray(en, np.int64).reshape(-1, 3), axis=1) for et, en in zip(*gmsh.model.mesh.getElements(2, t)[::2]) if et == 2]
            if not tri:
                continue
            tri = np.vstack(tri)
            _, cnt = np.unique(tri, axis=0, return_counts=True)
            if (cnt > 1).any() or ((tri[:, 0] == tri[:, 1]) | (tri[:, 1] == tri[:, 2])).any():
                bad.append(t)
        if not bad:
            return
    die("the surface mesh of the CAD faces %s has duplicated or degenerate triangles with every surface algorithm" % bad)



def size_field(surfaces):
    f = gmsh.model.mesh.field
    f.add("Distance", 1)
    f.setNumbers(1, "SurfacesList", surfaces)
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
    gmsh.option.setNumber("Mesh.MeshSizeMin", hmin)
    gmsh.option.setNumber("Mesh.MeshSizeMax", hmax)
    gmsh.option.setNumber("Mesh.Algorithm", 6)


# ---- prismatic boundary layer (RANS) ----------------------------------------------------------------------------------
# The layer itself is bl_layer.py (shared with the CAD surface mesher), for the half domain (open wall along y = 0) or the
# full domain (closed wall).
SHARP_HINT = ("the CAD surface is meshed as it is and its sharp edges (fin leading and trailing edges, rims of lugs and buttons) are not "
              "blunted, which the prism layer cannot always follow: use model = \"euler\", or round the sharp edges slightly in the CAD model")


def boundary_layer_mesh(parts):
    """Wall and base faces of the half (y >= 0) or full shape and their surface mesh; the prism layer, the tetrahedra
    outside and the SU2 file come from bl_layer.build (exits)."""
    import bl_layer

    if quarter:
        die("the prismatic boundary layer is not built on the quarter domain")
    tol = 1e-4 * L
    if half:
        box = occ.addBox(-L, 0.0, -2.0 * L, 3.0 * L, 2.0 * L, 4.0 * L)
        occ.intersect(parts, [(3, box)])
        occ.synchronize()
    vols = gmsh.model.getEntities(3)
    if half and len(vols) != 1:
        die("boolean operations produced %d volumes (expected 1); the geometry may be invalid" % len(vols))
    body, cut = [], []
    for _, t in gmsh.model.getBoundary(vols, oriented=False, combined=True):
        b = gmsh.model.getBoundingBox(2, t)
        if half and b[4] - b[1] < tol and abs(b[1]) < tol:
            cut.append(t)
        else:
            body.append(t)
    wall, cap = classify_body_faces(body)
    print("faces: wall %d, base %d, cut %d" % (len(wall), len(cap), len(cut)))
    if not wall or not cap or (half and not cut):
        die("could not classify the boundary faces (wall %d, base %d, cut %d): the shape needs aft-facing faces at its base x = %.4f m"
            % (len(wall), len(cap), len(cut), L))
    occ.remove(vols)
    if cut:
        occ.remove([(2, t) for t in cut])
    occ.synchronize()

    size_field(wall + cap)
    t1 = time.time()
    surface_mesh()

    bl_layer.build(
        p,
        wall,
        cap,
        half,
        t0,
        t1,
        invalid_hint="; " + SHARP_HINT,
        tet_hint=": the layer top probably intersects itself (narrow gaps such as a lug bore or the space between a button and "
        "the body are thinner than the layer); " + SHARP_HINT,
    )


if bl:
    boundary_layer_mesh(parts)

# ---- Euler: tetrahedra --------------------------------------------------------------------------------------------------
import su2_writer as sw  # noqa: E402

if half:
    # Half ball y >= 0 (quarter: z >= 0 too) as a sphere sector, as the CAD surface.
    domain = [(3, occ.addSphere(x0, 0, 0, R, angle1=0.0 if quarter else -math.pi / 2, angle2=math.pi / 2, angle3=math.pi))]
else:
    domain = [(3, occ.addBox(x0 - R, -R, -R, 2 * R, 2 * R, 2 * R))]
occ.cut(domain, parts)
occ.synchronize()
vols = [t for d, t in gmsh.model.getEntities(3)]
if len(vols) != 1:
    die("boolean operations produced %d volumes (expected 1); the geometry may be invalid" % len(vols))
vol = vols[0]
tol = 1e-4 * L
symm, symz, far, body = [], [], [], []
for d, t in gmsh.model.getEntities(2):
    b = gmsh.model.getBoundingBox(2, t)
    if half and b[4] - b[1] < tol and abs(b[1]) < tol:
        symm.append(t)
    elif quarter and b[5] - b[2] < tol and abs(b[2]) < tol:
        symz.append(t)
    elif max(b[3] - b[0], b[4] - b[1], b[5] - b[2]) > 0.5 * R:
        far.append(t)
    else:
        body.append(t)
wall, cap = classify_body_faces(body)
print("faces: wall %d, base %d, farfield %d, symmetry %d, symz %d" % (len(wall), len(cap), len(far), len(symm), len(symz)))
if quarter and not wall:
    # The cut with the edge y = z = 0 through the nose tip failed (the caller meshes the half domain instead).
    die("the quarter domain cannot be cut from the shape (no wall faces left); QUARTER_FAILED")
if not wall or not far or (half and not symm) or (quarter and not symz):
    die("could not classify the boundary faces")
if not cap:
    die("no base faces (aft-facing faces at the base x = %.4f m) found; the shape must end in a flat base or use base_treatment = \"fairing\"" % L)
size_field(wall + cap)
t1 = time.time()
gmsh.option.setNumber("Mesh.Algorithm3D", int(p["algorithm3d"]))
surface_mesh()
try:
    gmsh.model.mesh.generate(3)
except Exception as e:  # noqa: BLE001
    die("3D meshing failed (algorithm %d): %s" % (int(p["algorithm3d"]), e))
t2 = time.time()

tags, coord, _ = gmsh.model.mesh.getNodes()
xyz = np.asarray(coord, float).reshape(-1, 3)
look = np.full(int(tags.max()) + 1, -1, np.int64)
look[tags] = np.arange(len(tags))
etypes, etags, enodes = gmsh.model.mesh.getElements(3, vol)
tets = np.concatenate([np.asarray(n, np.int64).reshape(-1, 4) for t, n in zip(etypes, enodes) if t == 4]) if len(etypes) else np.zeros((0, 4), np.int64)
if not len(tets):
    die("the volume mesh is empty (3D meshing failed)")
tets = look[tets]


def surf(ents):
    out = []
    for t in ents:
        for k, n in zip(*gmsh.model.mesh.getElements(2, t)[::2]):
            if k == 2:
                out.append(look[np.asarray(n, np.int64).reshape(-1, 3)])
    return np.concatenate(out) if out else np.zeros((0, 3), np.int64)


if half:
    xyz[np.abs(xyz[:, 1]) <= 1e-9 * L, 1] = 0.0
if quarter:
    xyz[np.abs(xyz[:, 2]) <= 1e-9 * L, 2] = 0.0
markers = [("wall", [(sw.TRI, surf(wall))]), ("base", [(sw.TRI, surf(cap))]), ("farfield", [(sw.TRI, surf(far))])]
if half:
    markers.append(("symmetry", [(sw.TRI, surf(symm))]))
if quarter:
    markers.append(("symz", [(sw.TRI, surf(symz))]))
qs = np.asarray(gmsh.model.mesh.getElementQualities(etags[list(etypes).index(4)].tolist(), "minSICN"))
cells = {sw.TET: tets}
cells, markers, xyz, orphans = sw.compact(cells, markers, xyz)
flipped = sw.orient(cells, xyz)
bad, nonpos = sw.check(cells, xyz)
missing, extra = sw.unmarked_faces(cells, markers)
print("tetrahedra %d (%d reoriented), %d of non-positive volume, %d boundary faces without marker, %d marker faces inside"
      % (len(cells[sw.TET]), flipped, nonpos, missing, extra + orphans))
if nonpos or missing or extra or orphans:
    die("the volume mesh is invalid: %d cells of non-positive volume, %d boundary faces without marker, %d marker faces inside" % (nonpos, missing, extra + orphans))
mult = 1
if quarter:
    cells, markers, xyz = sw.mirror(cells, markers, xyz, 2, "symz", 1e-9 * L)
    mult *= 2
if p["y_mirror"]:
    cells, markers, xyz = sw.mirror(cells, markers, xyz, 1, "symmetry", 1e-9 * L)
    mult *= 2
ncells = sw.write(p["out_su2"], cells, markers, xyz)
nwall = sum(len(c) for name, fc in markers if name in ("wall", "base") for _, c in fc)
stats = {
    "nodes": int(len(xyz)),
    "cells": int(ncells),
    "by_type": {"Tetrahedron 4": int(ncells)},
    "quality_sicn_min": float(qs.min()),
    "quality_sicn_mean": float(qs.mean()),
    "cells_sicn_below_0.05": int((qs < 0.05).sum()) * mult,
    "wall_triangles": int(nwall),
    "seconds_setup": t1 - t0,
    "seconds_mesh": t2 - t1,
}
json.dump(stats, open(p["stats_json"], "w"), indent=1)
print("MESH_OK nodes=%d cells=%d minSICN=%.4f volume %.1fs" % (stats["nodes"], stats["cells"], stats["quality_sicn_min"], t2 - t1))
gmsh.finalize()
