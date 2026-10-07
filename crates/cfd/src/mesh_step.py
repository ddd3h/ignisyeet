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
#          * RANS (`bl`): prismatic boundary layer built by this script from the wall surface mesh (the column code of
#            mesh_gmsh.py, copied here because mesh_gmsh.py runs on import), outside it tetrahedra in a box farfield;
#            half domain (`meshed` = half, open wall along y = 0) or full domain (closed wall, no symmetry plane).
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
    # Euler tail fairing: a cone (cut at 10 % of the base radius, flat cap) behind the largest flat base face, fused with the body.
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
        cone = occ.addCone(xb, 0, 0, tl, 0, 0, rb, 0.1 * rb)
        out, _ = occ.fuse(vols, [(3, cone)])
        occ.synchronize()
        vols = [v for v in out if v[0] == 3]
        report["fairing"] = {"x": xb, "radius": rb, "length": tl, "base_face_area": big["area"]}
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
# wall_normals, distance_to_edges, column_heights, rerun and the layer construction of boundary_layer_mesh are copies of
# the functions of mesh_gmsh.py (which executes on import and so cannot be imported). Changes: `half` selects whether
# nodes on y = 0 are symmetry-plane nodes (the full domain has a closed wall and no symmetry plane), and the outer
# surfaces are the half box with the y = 0 plane around the layer section, or the full box.
def wall_normals(tri, xyz, passes, corner=0.0, corner_passes=0):
    """Orients the wall triangles `tri` (indices into `xyz`) consistently and outward (into the fluid; the signed
    volume is closed by y = 0 for the half wall) and returns (tri, node normals, concave edges). See mesh_gmsh.py."""
    import collections

    tri = tri.copy()
    nt, nn = len(tri), len(xyz)
    a = np.concatenate([tri[:, 0], tri[:, 1], tri[:, 2]])
    b = np.concatenate([tri[:, 1], tri[:, 2], tri[:, 0]])
    owner = np.tile(np.arange(nt), 3)
    key = np.minimum(a, b) * nn + np.maximum(a, b)
    order = np.argsort(key, kind="stable")
    srt = key[order]
    starts = np.flatnonzero(np.r_[True, srt[1:] != srt[:-1]])
    counts = np.diff(np.r_[starts, len(srt)])
    if (counts > 2).any():
        die("the wall surface mesh is not manifold (%d edges with more than two triangles)" % int((counts > 2).sum()))
    pairs = starts[counts == 2]
    fwd = a < b
    same = fwd[order[pairs]] == fwd[order[pairs + 1]]
    t1, t2 = owner[order[pairs]], owner[order[pairs + 1]]
    adj = [[] for _ in range(nt)]
    for u, v, s in zip(t1.tolist(), t2.tolist(), same.tolist()):
        adj[u].append((v, s))
        adj[v].append((u, s))
    flip = np.full(nt, -1, np.int64)
    comp = np.zeros(nt, np.int64)
    ncomp = 0
    for s0 in range(nt):
        if flip[s0] >= 0:
            continue
        flip[s0] = 0
        comp[s0] = ncomp
        q = collections.deque([s0])
        while q:
            u = q.popleft()
            for v, s in adj[u]:
                want = flip[u] ^ int(s)
                if flip[v] < 0:
                    flip[v] = want
                    comp[v] = ncomp
                    q.append(v)
                elif flip[v] != want:
                    die("the wall surface mesh is not orientable")
        ncomp += 1
    tri[flip == 1] = tri[flip == 1][:, [0, 2, 1]]
    p0, p1, p2 = xyz[tri[:, 0]], xyz[tri[:, 1]], xyz[tri[:, 2]]
    cr = np.cross(p1 - p0, p2 - p0)
    vol = np.bincount(comp, weights=cr[:, 1] * (p0[:, 1] + p1[:, 1] + p2[:, 1]), minlength=ncomp)
    inward = vol[comp] < 0.0
    tri[inward] = tri[inward][:, [0, 2, 1]]
    cr[inward] *= -1.0
    fn = cr / np.linalg.norm(cr, axis=1)[:, None]
    nrm = np.zeros_like(xyz)
    for k in range(3):
        u = xyz[tri[:, (k + 1) % 3]] - xyz[tri[:, k]]
        v = xyz[tri[:, (k + 2) % 3]] - xyz[tri[:, k]]
        c = np.einsum("ij,ij->i", u, v) / (np.linalg.norm(u, axis=1) * np.linalg.norm(v, axis=1))
        np.add.at(nrm, tri[:, k], fn * np.arccos(np.clip(c, -1.0, 1.0))[:, None])
    used = np.zeros(nn, bool)
    used[tri.ravel()] = True
    plane = used & (np.abs(xyz[:, 1]) < 1e-9 * L) if half else np.zeros(nn, bool)
    ue = np.unique(key)
    ea, eb = ue // nn, ue % nn

    def fix(m):
        m[plane, 1] = 0.0
        m[used] /= np.linalg.norm(m[used], axis=1)[:, None]
        return m

    inc_n, inc_f = tri.ravel(), np.repeat(np.arange(nt), 3)
    o = np.argsort(inc_n, kind="stable")
    inc_n, inc_f = inc_n[o], inc_f[o]
    deg = np.bincount(inc_n, minlength=nn)
    first = np.r_[0, np.cumsum(deg)[:-1]]
    pad = np.full((nn, int(deg.max())), -1, np.int64)
    pad[inc_n, np.arange(len(inc_n)) - first[inc_n]] = inc_f
    nodes = np.flatnonzero(used)
    pad = pad[nodes]
    pad = np.where(pad < 0, pad[:, :1], pad)
    fan = fn[pad]
    mir = fan.copy()
    mir[plane[nodes], :, 1] *= -1.0
    fan = np.concatenate([fan, mir], axis=1)

    def vis(m):
        """Smallest cosine between the node normal and its incident triangle normals."""
        return np.einsum("ijk,ik->ij", fan, m[nodes]).min(axis=1)

    nrm = fix(nrm)
    c = nrm[nodes].copy()
    rows = np.arange(len(nodes))
    for i in range(1, 200):
        far = np.einsum("ijk,ik->ij", fan, c).argmin(axis=1)
        c += (fan[rows, far] - c) / (i + 1.0)
    nrm[nodes] = c
    nrm = fix(nrm)
    v0 = vis(nrm)
    for _ in range(passes):
        acc = nrm.copy()
        np.add.at(acc, ea, nrm[eb])
        np.add.at(acc, eb, nrm[ea])
        cand = fix(acc)
        ok = vis(cand) >= np.minimum(vis(nrm), 0.5)
        nrm[nodes[ok]] = cand[nodes[ok]]
    cen = (p0 + p1 + p2) / 3.0
    front = np.einsum("ij,ij->i", cen[t2] - cen[t1], fn[t1]) > 0.0
    sharp = np.einsum("ij,ij->i", fn[t1], fn[t2]) < math.cos(math.radians(30.0))
    ce = pairs[front & sharp]
    concave = np.stack([a[order[ce]], b[order[ce]]], axis=1) if len(ce) else np.zeros((0, 2), np.int64)
    if len(concave) and corner_passes:
        dist = distance_to_edges(xyz[nodes], xyz, concave)
        near = nodes[dist < corner]
        for _ in range(corner_passes):
            acc = nrm.copy()
            np.add.at(acc, ea, nrm[eb])
            np.add.at(acc, eb, nrm[ea])
            cand = fix(acc)
            ok = np.zeros(nn, bool)
            ok[nodes] = vis(cand) >= np.minimum(vis(nrm), 0.5)
            sel = near[ok[near]]
            nrm[sel] = cand[sel]
    v1 = vis(nrm)
    print("boundary layer normals: %d triangles, %d nodes on y = 0, smallest cosine to an incident triangle %.3f (before smoothing %.3f), %d concave edges"
          % (nt, int(plane.sum()), v1.min(), v0.min(), len(concave)))
    return tri, nrm, concave


def distance_to_edges(q, xyz, edges):
    """Distance of the points `q` to the edges (node pairs into `xyz`), sampled at 5 points per edge."""
    pa, pb = xyz[edges[:, 0]], xyz[edges[:, 1]]
    pts = np.concatenate([pa + (pb - pa) * s for s in np.linspace(0.0, 1.0, 5)])
    dist = np.full(len(q), np.inf)
    for i0 in range(0, len(q), 2000):
        dist[i0:i0 + 2000] = np.linalg.norm(q[i0:i0 + 2000, None, :] - pts[None, :, :], axis=2).min(axis=1)
    return dist


def column_heights(h1, n, thick):
    """Cumulative layer heights (n + 1, m) of columns with first height h1, n layers and total thicknesses `thick`
    (geometric growth, ratio solved per column; at least n h1)."""
    t = np.maximum(thick, n * h1 * (1.0 + 1e-9))
    lo, hi = np.ones_like(t), np.full_like(t, 2.0)
    for _ in range(80):
        r = 0.5 * (lo + hi)
        s = h1 * (r ** n - 1.0) / (r - 1.0)
        big = s > t
        hi = np.where(big, r, hi)
        lo = np.where(big, lo, r)
    r = 0.5 * (lo + hi)
    k = np.arange(n + 1)[:, None]
    return h1 * (r[None, :] ** k - 1.0) / (r[None, :] - 1.0)


def rerun(q, why):
    """Runs this script again in a fresh process with the parameters `q` (written next to the original ones)."""
    path = os.path.join(os.path.dirname(os.path.abspath(sys.argv[1])), "params_retry.json")
    json.dump(q, open(path, "w"), indent=1)
    print(why)
    sys.stdout.flush()
    gmsh.finalize()
    os.execv(sys.executable, [sys.executable, os.path.abspath(sys.argv[0]), path])


SHARP_HINT = ("the CAD surface is meshed as it is and its sharp edges (fin leading and trailing edges, rims of lugs and buttons) are not "
              "blunted, which the prism layer cannot always follow: use model = \"euler\", or round the sharp edges slightly in the CAD model")


def boundary_layer_mesh(parts):
    """Prism layers on the wall and base mesh, tetrahedra in the box farfield (half box y >= 0 for the half domain);
    writes the SU2 mesh (mirrored to the full domain for y_mirror) and the statistics, then exits."""
    import su2_writer as sw

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

    # ---- wall mesh and normals -----------------------------------------------------------------------
    tags, coord, _ = gmsh.model.mesh.getNodes()
    idx = np.zeros(int(tags.max()) + 1, np.int64)
    idx[tags] = np.arange(len(tags))
    xyz = coord.reshape(-1, 3).copy()
    if half:
        xyz[np.abs(xyz[:, 1]) < 1e-12, 1] = 0.0
    tris, kinds = [], []
    for kind, ents in ((0, wall), (1, cap)):
        for t in ents:
            for e, n in zip(*gmsh.model.mesh.getElements(2, t)[::2]):
                if e != 2:
                    die("the wall surface mesh has non-triangular elements")
                tris.append(idx[np.asarray(n, np.int64)].reshape(-1, 3))
                kinds.append(np.full(len(tris[-1]), kind))
    tri, kinds = np.vstack(tris), np.concatenate(kinds)
    tri, nrm, concave = wall_normals(tri, xyz, int(bl["smoothing"]), bl["thickness"], int(bl["corner_smoothing"]))
    used = np.unique(tri)
    m = len(used)
    loc = np.full(len(xyz), -1, np.int64)
    loc[used] = np.arange(m)
    x0w, nw, tl = xyz[used], nrm[used], loc[tri]
    ce = loc[concave]

    # ---- column thickness (see mesh_gmsh.py) ----------------------------------------------------------------
    n_lay, h1 = int(bl["layers"]), bl["first_height"]
    thick = np.full(m, bl["thickness"])
    if len(ce):
        thick = np.minimum(thick, bl["junction"] * bl["thickness"] + distance_to_edges(x0w, x0w, ce))
    e0 = np.unique(np.sort(np.concatenate([tl[:, [0, 1]], tl[:, [1, 2]], tl[:, [2, 0]]]), axis=1), axis=0)
    ang = np.arccos(np.clip(np.einsum("ij,ij->i", nw[e0[:, 0]], nw[e0[:, 1]]), -1.0, 1.0))
    spread = np.zeros(m)
    np.maximum.at(spread, e0[:, 0], ang)
    np.maximum.at(spread, e0[:, 1], ang)
    thick = np.minimum(thick, bl["fan"] * h0 / np.maximum(spread, 1e-6))
    e = e0
    elen = np.linalg.norm(x0w[e[:, 0]] - x0w[e[:, 1]], axis=1)

    def limit(t):
        for _ in range(50):
            t2 = t.copy()
            np.minimum.at(t2, e[:, 0], t[e[:, 1]] + 0.5 * elen)
            np.minimum.at(t2, e[:, 1], t[e[:, 0]] + 0.5 * elen)
            if np.array_equal(t2, t):
                break
            t = t2
        return t

    thick = limit(thick)
    for it in range(30):
        s = column_heights(h1, n_lay, thick)
        lay = x0w[None, :, :] + s[:, :, None] * nw[None, :, :]
        pts_all = lay.reshape(-1, 3)
        prisms = np.concatenate([np.concatenate([tl + k * m, tl + (k + 1) * m], axis=1) for k in range(n_lay)])
        jac = sw.prism_corner_jacobians(pts_all, prisms).min(axis=1)
        bad = jac <= 0.0
        if not bad.any():
            break
        badnodes = np.unique(prisms[bad] % m)
        print("boundary layer: %d inverted prisms at %d columns; thinning them" % (int(bad.sum()), len(badnodes)))
        thick[badnodes] *= 0.6
        thick = limit(thick)
    top = n_lay * m
    print("boundary layer: %d layers, first %.3g m, thickness %.3g m (smallest column %.3g m, %d columns thinner than half)"
          % (n_lay, h1, thick.max(), thick.min(), int((thick < 0.5 * bl["thickness"]).sum())))

    # Open edges: on y = 0 for the half wall (layer sides there are quadrilaterals), none for the full wall.
    a_ = np.concatenate([tl[:, 0], tl[:, 1], tl[:, 2]])
    b_ = np.concatenate([tl[:, 1], tl[:, 2], tl[:, 0]])
    k_ = np.minimum(a_, b_) * m + np.maximum(a_, b_)
    uk, cnt = np.unique(k_, return_counts=True)
    single = np.isin(k_, uk[cnt == 1])
    ba, bb = a_[single], b_[single]
    if not half and len(ba):
        die("the wall has %d open edges (the shape is not closed)" % len(ba))
    if half and (not (np.abs(x0w[ba, 1]) < 1e-12).all() or not (np.abs(x0w[bb, 1]) < 1e-12).all()):
        die("the wall has an open edge away from the symmetry plane")
    quads = np.concatenate([np.stack([ba + k * m, bb + k * m, bb + (k + 1) * m, ba + (k + 1) * m], axis=1) for k in range(n_lay)]) if len(ba) else np.zeros((0, 4), np.int64)
    nxt = dict(zip(ba.tolist(), bb.tolist()))
    loops, seen = [], set()
    for s0 in ba.tolist():
        if s0 in seen:
            continue
        lp, v = [], s0
        while v not in seen:
            seen.add(v)
            lp.append(v)
            v = nxt[v]
        loops.append(lp)

    # ---- outer surfaces: the box farfield (and the y = 0 plane around the layers for the half) ---------------------
    t_layers = time.time()
    gmsh.model.add("outer")
    geo = gmsh.model.geo
    toplay = lay[n_lay]
    seg = {}
    hole_loops, ptag = [], {}
    for lp in loops:
        for i, v in enumerate(lp):
            w = lp[(i + 1) % len(lp)]
            seg[v] = np.linalg.norm(toplay[w] - toplay[v])
        for v in lp:
            ptag[v] = geo.addPoint(*toplay[v].tolist(), max(seg[v], 1e-6))
        lines = []
        for i, v in enumerate(lp):
            w = lp[(i + 1) % len(lp)]
            lines.append(geo.addLine(ptag[v], ptag[w]))
        hole_loops.append((geo.addCurveLoop(lines), lines))
    xs, ys, zs = (x0 - R, x0 + R), ((0.0 if half else -R), R), (-R, R)
    c = {(i, j, k): geo.addPoint(xs[i], ys[j], zs[k], hmax) for i in (0, 1) for j in (0, 1) for k in (0, 1)}
    blines = {}

    def line(u, v):
        if (v, u) in blines:
            return -blines[(v, u)]
        if (u, v) not in blines:
            blines[(u, v)] = geo.addLine(u, v)
        return blines[(u, v)]

    def loop(q):
        return geo.addCurveLoop([line(c[q[i]], c[q[(i + 1) % 4]]) for i in range(4)])

    ylo_face = geo.addPlaneSurface([loop([(0, 0, 0), (1, 0, 0), (1, 0, 1), (0, 0, 1)])] + [h for h, _ in hole_loops])
    far = [
        geo.addPlaneSurface([loop(q)])
        for q in (
            [(0, 1, 0), (1, 1, 0), (1, 1, 1), (0, 1, 1)],
            [(0, 0, 0), (1, 0, 0), (1, 1, 0), (0, 1, 0)],
            [(0, 0, 1), (1, 0, 1), (1, 1, 1), (0, 1, 1)],
            [(0, 0, 0), (0, 1, 0), (0, 1, 1), (0, 0, 1)],
            [(1, 0, 0), (1, 1, 0), (1, 1, 1), (1, 0, 1)],
        )
    ]
    if not half:
        far.append(ylo_face)
    geo.synchronize()
    for _, lines in hole_loops:
        for ln in lines:
            gmsh.model.mesh.setTransfiniteCurve(abs(ln), 2)
    hl = [abs(ln) for _, lines in hole_loops for ln in lines]
    f = gmsh.model.mesh.field
    if hl:
        f.add("Distance", 1)
        f.setNumbers(1, "CurvesList", hl)
        f.setNumber(1, "Sampling", 4)
        f.add("Threshold", 2)
        f.setNumber(2, "InField", 1)
        f.setNumber(2, "SizeMin", h0)
        f.setNumber(2, "SizeMax", hmax)
        f.setNumber(2, "DistMin", 0.0)
        f.setNumber(2, "DistMax", (hmax - h0) / (grow - 1.0))
        f.setAsBackgroundMesh(2)
    gmsh.option.setNumber("Mesh.MeshSizeExtendFromBoundary", 1)
    gmsh.option.setNumber("Mesh.MeshSizeFromCurvature", 0)
    gmsh.model.mesh.generate(2)
    otag, ocoord, _ = gmsh.model.mesh.getNodes()
    ocoord = ocoord.reshape(-1, 3)
    gid = {}
    for v, pt in ptag.items():
        for t_ in gmsh.model.mesh.getNodes(0, pt)[0].tolist():
            gid[t_] = top + v
    extra_tags = [t_ for t_ in otag.tolist() if t_ not in gid]
    n_base = (n_lay + 1) * m
    for i, t_ in enumerate(extra_tags):
        gid[t_] = n_base + i
    opos = dict(zip(otag.tolist(), range(len(otag))))
    extra_xyz = ocoord[[opos[t_] for t_ in extra_tags]]
    if half:
        extra_xyz[np.abs(extra_xyz[:, 1]) < 1e-12, 1] = 0.0

    def surf_tris(s_):
        out = []
        for e_, n_ in zip(*gmsh.model.mesh.getElements(2, s_)[::2]):
            if e_ != 2:
                die("unexpected element type %d on the outer surfaces" % e_)
            out.append(np.array([gid[t_] for t_ in n_.tolist()], np.int64).reshape(-1, 3))
        return np.vstack(out)

    sym_tri = surf_tris(ylo_face) if half else np.zeros((0, 3), np.int64)
    far_tri = np.vstack([surf_tris(s_) for s_ in far])
    gmsh.model.remove()

    # ---- tetrahedra between the layer top and the outer surfaces (gmsh, discrete model) ------------------
    allxyz = np.vstack([pts_all, extra_xyz])
    top_tri = tl + top
    outer_sets = [top_tri, sym_tri, far_tri] if half else [top_tri, far_tri]
    bnd = np.vstack(outer_sets)
    bn = np.unique(bnd)
    gmsh.model.add("fluid")
    for k, st in enumerate(outer_sets, start=1):
        gmsh.model.addDiscreteEntity(2, k)
        if k == 1:
            gmsh.model.mesh.addNodes(2, 1, (bn + 1).tolist(), allxyz[bn].ravel().tolist())
        gmsh.model.mesh.addElementsByType(k, 2, [], (st + 1).ravel().tolist())
    if half:
        gmsh.model.geo.addVolume([gmsh.model.geo.addSurfaceLoop([1, 2, 3])])
    else:
        # Two closed shells: the farfield box and the layer top.
        gmsh.model.geo.addVolume([gmsh.model.geo.addSurfaceLoop([2]), gmsh.model.geo.addSurfaceLoop([1])])
    gmsh.model.geo.synchronize()
    f.add("Distance", 1)
    f.setNumbers(1, "SurfacesList", [1])
    f.add("Threshold", 2)
    f.setNumber(2, "InField", 1)
    f.setNumber(2, "SizeMin", h0)
    f.setNumber(2, "SizeMax", hmax)
    f.setNumber(2, "DistMin", 0.0)
    f.setNumber(2, "DistMax", (hmax - h0) / (grow - 1.0))
    f.setAsBackgroundMesh(2)
    gmsh.option.setNumber("Mesh.MeshSizeExtendFromBoundary", 1)
    gmsh.option.setNumber("Mesh.Algorithm3D", int(p["algorithm3d"]))
    gmsh.option.setNumber("Mesh.Renumber", 0)
    t_surf = time.time()
    try:
        gmsh.model.mesh.generate(3)
    except Exception as e:  # noqa: BLE001
        if int(p["algorithm3d"]) == 1:
            die("the tetrahedral mesh outside the boundary layer failed (%s): the layer top probably intersects itself (narrow gaps such as "
                "a lug bore or the space between a button and the body are thinner than the layer); %s" % (e, SHARP_HINT))
        rerun(dict(p, algorithm3d=1), "HXT failed (%s); retrying with Delaunay" % e)
    t2 = time.time()
    ntag, ncoord, _ = gmsh.model.mesh.getNodes()
    known = np.zeros(int(ntag.max()) + 1, bool)
    known[bn + 1] = True
    inner = ntag[~known[ntag]]
    ncoord = ncoord.reshape(-1, 3)
    npos = np.zeros(int(ntag.max()) + 1, np.int64)
    npos[ntag] = np.arange(len(ntag))
    gmap = np.full(int(ntag.max()) + 1, -1, np.int64)
    gmap[bn + 1] = bn
    gmap[inner] = len(allxyz) + np.arange(len(inner))
    allxyz = np.vstack([allxyz, ncoord[npos[inner]]])
    tets = []
    for et, en in zip(*gmsh.model.mesh.getElements(3)[::2]):
        if et != 4:
            die("unexpected volume element type %d outside the boundary layer" % et)
        tets.append(gmap[np.asarray(en, np.int64)].reshape(-1, 4))
    tets = np.vstack(tets)
    if (tets < 0).any():
        die("a tetrahedron uses an unknown node")

    # ---- cells, markers, checks --------------------------------------------------------------------
    cells = {sw.PRISM: prisms, sw.TET: tets}
    names = {sw.PRISM: "Prism 6", sw.TET: "Tetrahedron 4"}
    markers = [
        ("wall", [(sw.TRI, tl[kinds == 0])]),
        ("base", [(sw.TRI, tl[kinds == 1])]),
        ("farfield", [(sw.TRI, far_tri)]),
    ]
    if half:
        markers.append(("symmetry", [(sw.TRI, sym_tri), (sw.QUAD, quads)]))
    cells, markers, allxyz, orphans = sw.compact(cells, markers, allxyz)
    flipped = sw.orient(cells, allxyz)
    bad, nonpos = sw.check(cells, allxyz)
    missing, extra = sw.unmarked_faces(cells, markers)
    extra += orphans
    tt = np.sort(cells[sw.TET][:, [[0, 1, 2], [0, 1, 3], [0, 2, 3], [1, 2, 3]]].reshape(-1, 3), axis=1)
    topk = np.sort(top_tri, axis=1)
    tkey = lambda q: (q[:, 0] * len(allxyz) + q[:, 1]) * len(allxyz) + q[:, 2]  # noqa: E731
    tk, kk = tkey(tt), tkey(topk)
    o_ = np.argsort(tk)
    pos_ = np.searchsorted(tk[o_], kk)
    pos_ = np.minimum(pos_, len(tk) - 1)
    hit = tk[o_][pos_] == kk
    tet_of = o_[pos_] // 4
    vol = np.abs(sw.signed_volume(sw.TET, allxyz, cells[sw.TET][tet_of[hit]]))
    tp = allxyz[top_tri[hit]]
    area = 0.5 * np.linalg.norm(np.cross(tp[:, 1] - tp[:, 0], tp[:, 2] - tp[:, 0]), axis=1)
    h_tet = 3.0 * vol / area
    h_last = (s[n_lay] - s[n_lay - 1])[tl[hit]].mean(axis=1)
    jump = np.maximum(h_tet / h_last, h_last / h_tet)
    jmax, j99 = float(jump.max()), float(np.percentile(jump, 99))
    print("boundary layer: %s; %d cells reoriented, %d inverted prisms, %d cells of non-positive volume; symmetry %d triangles + %d quads; %d boundary faces without marker, %d marker faces inside; size ratio at the layer top: median %.2f, 99 %% %.2f, max %.2f"
          % (", ".join("%s %d" % (names[k], len(cells[k])) for k in cells), flipped, bad, nonpos, len(sym_tri), len(quads), missing, extra, float(np.median(jump)), j99, jmax))
    if bad or nonpos or missing or extra:
        what = "%d inverted prisms, %d cells of non-positive volume, %d boundary faces without marker, %d marker faces inside" % (bad, nonpos, missing, extra)
        if bl["retry"]:
            r = bl["retry"]
            rerun(dict(p, bl=r, bl_attempt=p.get("bl_attempt", 1) + 1),
                  "boundary layer invalid (%s); retrying with %d smoothing passes and junction factor %.2f" % (what, r["smoothing"], r["junction"]))
        die("the prismatic boundary layer is invalid: %s; %s" % (what, SHARP_HINT))

    mult = 1
    if p["y_mirror"]:
        cells, markers, allxyz = sw.mirror(cells, markers, allxyz, 1, "symmetry", 1e-12)
        mult = 2
    ncells = sw.write(p["out_su2"], cells, markers, allxyz)
    stats = {
        "nodes": int(len(allxyz)),
        "cells": int(ncells),
        "by_type": {names[k]: int(len(c_)) for k, c_ in cells.items()},
        "quality_sicn_min": 0.0,
        "quality_sicn_mean": 0.0,
        "cells_sicn_below_0.05": 0,
        "wall_triangles": int(sum(len(fc) for _, fc in dict(markers)["wall"])),
        "seconds_setup": t1 - t0,
        "seconds_mesh": t2 - t1,
        "bl_layers": n_lay,
        "bl_first_height": h1,
        "bl_thickness": float(thick.max()),
        "bl_invalid": bad,
        "bl_attempt": p.get("bl_attempt", 1),
        "bl_top_ratio_max": jmax,
        "bl_top_ratio_p99": j99,
    }
    q = gmsh.model.mesh.getElementQualities(gmsh.model.mesh.getElementsByType(4)[0].tolist(), "minSICN")
    stats["quality_sicn_min"], stats["quality_sicn_mean"] = float(np.min(q)), float(np.mean(q))
    stats["cells_sicn_below_0.05"] = int((np.asarray(q) < 0.05).sum()) * mult
    json.dump(stats, open(p["stats_json"], "w"), indent=1)
    print("MESH_OK nodes=%d cells=%d minSICN=%.4f layers %.1fs surfaces %.1fs volume %.1fs"
          % (stats["nodes"], stats["cells"], stats["quality_sicn_min"], t_layers - t1, t_surf - t_layers, t2 - t_surf))
    gmsh.finalize()
    sys.exit(0)


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
