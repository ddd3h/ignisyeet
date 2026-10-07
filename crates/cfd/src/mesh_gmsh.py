# SPDX-License-Identifier: GPL-3.0-or-later
# IgnisYeet CFD volume mesher (generated into the case work directory and run with the gmsh python).
# Usage: python mesh_gmsh.py params.json
#
# Builds the rocket as an OpenCASCADE solid (body of revolution united with the fins), cuts it out
# of the farfield ball (or half ball with the y = 0 symmetry plane), meshes the volume with
# tetrahedra graded away from the wall and writes an SU2 mesh with the markers wall, base, farfield,
# symmetry. Full domain: a y-symmetric layout is meshed as the half ball and mirrored about y = 0
# (the `symmetry` marker is dropped; a full ball fails in HXT at the periodic seam), any other layout
# uses a box farfield (the OpenCASCADE box shares its edges between the faces). Without `bl` the volume is all
# tetrahedra.
#
# With `bl` (RANS): prismatic boundary layer. The half solid y >= 0 (both halves for a layout that is not symmetric
# about y = 0) is meshed on its surface, the layers are extruded along node normals
# (gmsh.model.geo.extrudeBoundaryLayer), the rest of the box farfield is filled with tetrahedra and the mesh is
# written by su2_writer (see boundary_layer_mesh).
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
half = bool(p["symmetry"]) or bool(p["y_mirror"])
# z_mirror: mesh only the quarter y >= 0, z >= 0 and mirror it about z = 0 afterwards, so the half-model mesh
# is exactly symmetric about the x-y plane (a Delaunay mesh of the half model is not).
zmirror = bool(p["z_mirror"]) and half
h0 = p["wall_size"]
hmax = p["max_size"]
grow = p["growth"]
fins = p["fins"]
bl = p.get("bl")
if bl:
    # The default relative tolerance (1e-8 of the 2 R domain) merges the micrometre-close nodes of the first layers.
    gmsh.option.setNumber("Geometry.Tolerance", 1e-12)

# ---- rocket solid ----------------------------------------------------------------------------------
# Body of revolution: the profile lies in a half plane at azimuth `seam`, away from the fin planes
# and the symmetry plane so that no surface seam coincides with y = 0.
nf = fins["count"] if fins else 0
roll = math.radians(p["fin_roll_deg"])
seam = math.radians(45.0 if nf == 0 else 90.0 + 180.0 / nf) + roll
if abs(math.sin(seam)) < 0.2:
    seam += math.radians(30.0)
prof = p["profile"]


def profile_face(az):
    """Planar face of the body profile in the half plane at azimuth `az`."""
    cs, sn = math.cos(az), math.sin(az)
    pts = [occ.addPoint(x, r * cs, r * sn) for x, r in prof]
    axis0 = pts[0] if abs(prof[0][1]) < 1e-12 else occ.addPoint(prof[0][0], 0, 0)
    axis1 = pts[-1] if abs(prof[-1][1]) < 1e-12 else occ.addPoint(prof[-1][0], 0, 0)
    segs = [occ.addLine(pts[i], pts[i + 1]) for i in range(len(pts) - 1)]
    if axis1 != pts[-1]:
        segs.append(occ.addLine(pts[-1], axis1))
    segs.append(occ.addLine(axis1, axis0))
    if abs(prof[0][1]) > 1e-12:
        segs.append(occ.addLine(axis0, pts[0]))
    return occ.addPlaneSurface([occ.addWire(segs)])


def revolve(az, angle):
    face = profile_face(az)
    rev = occ.revolve([(2, face)], 0, 0, 0, 1, 0, 0, angle)
    occ.remove([(2, face)], recursive=False)
    return [(3, t) for d, t in rev if d == 3][0]


if half or bl:
    parts = [revolve(seam, 2 * math.pi)]
else:
    # A full body of revolution is a periodic surface with a degenerate apex: HXT rejects its surface
    # mesh ("overlapping facets"). Two half revolutions give non-periodic faces. They (and the fins)
    # are all cut from the farfield box, which unites them implicitly; fusing the halves first fails
    # in OpenCASCADE (BOPAlgo_AlertUnableToOrientTheShape).
    parts = [revolve(seam, math.pi), revolve(seam + math.pi, math.pi)]


def lens(x_le, x_te, r_pos, phi, thick):
    """Biconvex section (parabolic thickness) in the plane at distance r_pos, azimuth phi."""
    u = (math.cos(phi), math.sin(phi))  # span direction in (y, z)
    w = (-math.sin(phi), math.cos(phi))  # thickness direction
    n = 8
    if bl and bl["edge_blunt"] > 0.0:
        return blunt_lens(x_le, x_te, r_pos, u, w, thick, n, bl["edge_blunt"])

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


def blunt_lens(x_le, x_te, r_pos, u, w, thick, n, b):
    """Biconvex section with blunt edges: the thickness 4 xi (1 - xi) for xi in [b, 1 - b] stretched over the chord,
    closed by short straight edges at the leading and trailing edge (prism layers fold at a knife edge)."""

    def pt(s, side):
        xi = b + (1.0 - 2.0 * b) * s
        x = x_le + s * (x_te - x_le)
        h = 0.5 * thick * 4.0 * xi * (1.0 - xi) * side
        return occ.addPoint(x, r_pos * u[0] + h * w[0], r_pos * u[1] + h * w[1])

    up = [pt(i / n, 1) for i in range(n + 1)]
    lo = [pt(i / n, -1) for i in range(n + 1)]
    c1 = occ.addSpline(up)
    c2 = occ.addSpline(lo)
    return occ.addWire([occ.addLine(lo[0], up[0]), c1, occ.addLine(up[-1], lo[-1]), c2])


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
if (half or bl) and len(parts) > 1:
    u_out, _ = occ.fuse([parts[0]], parts[1:], removeObject=True, removeTool=True)
    parts = [t for t in u_out if t[0] == 3]
occ.synchronize()


# ---- prismatic boundary layer (RANS) ----------------------------------------------------------------
def bl_normals(src, passes):
    """Node normals of the meshed wall surfaces `src`, pointing into the fluid, as a list view of triangles (each
    triangle carries the normals of its three nodes; gmsh reads the extrusion direction from it). The triangle
    orientation is made consistent across shared edges and outward (signed volume closed by y = 0); the normals are
    angle-weighted node averages, tangent to y = 0 on the nodes of that plane, smoothed `passes` times."""
    import collections

    import numpy as np

    tags, coord, _ = gmsh.model.mesh.getNodes()
    idx = np.zeros(int(tags.max()) + 1, np.int64)
    idx[tags] = np.arange(len(tags))
    xyz = coord.reshape(-1, 3)
    tris = []
    for t in src:
        for e, n in zip(*gmsh.model.mesh.getElements(2, t)[::2]):
            if e != 2:
                raise SystemExit("mesh error: the wall surface mesh has non-triangular elements")
            tris.append(idx[np.asarray(n, np.int64)].reshape(-1, 3))
    tri = np.vstack(tris)
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
        raise SystemExit("mesh error: the wall surface mesh is not manifold (%d edges with more than two triangles)" % int((counts > 2).sum()))
    pairs = starts[counts == 2]
    fwd = a < b
    same = fwd[order[pairs]] == fwd[order[pairs + 1]]
    adj = [[] for _ in range(nt)]
    for u, v, s in zip(owner[order[pairs]].tolist(), owner[order[pairs + 1]].tolist(), same.tolist()):
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
                    raise SystemExit("mesh error: the wall surface mesh is not orientable")
        ncomp += 1
    tri[flip == 1] = tri[flip == 1][:, [0, 2, 1]]
    p0, p1, p2 = xyz[tri[:, 0]], xyz[tri[:, 1]], xyz[tri[:, 2]]
    cr = np.cross(p1 - p0, p2 - p0)
    vol = np.bincount(comp, weights=cr[:, 1] * (p0[:, 1] + p1[:, 1] + p2[:, 1]), minlength=ncomp)
    cr *= np.where(vol[comp] < 0.0, -1.0, 1.0)[:, None]
    fn = cr / np.linalg.norm(cr, axis=1)[:, None]
    nrm = np.zeros_like(xyz)
    for k in range(3):
        u = xyz[tri[:, (k + 1) % 3]] - xyz[tri[:, k]]
        v = xyz[tri[:, (k + 2) % 3]] - xyz[tri[:, k]]
        c = np.einsum("ij,ij->i", u, v) / (np.linalg.norm(u, axis=1) * np.linalg.norm(v, axis=1))
        np.add.at(nrm, tri[:, k], fn * np.arccos(np.clip(c, -1.0, 1.0))[:, None])
    used = np.zeros(nn, bool)
    used[tri.ravel()] = True
    plane = used & (np.abs(xyz[:, 1]) < 1e-9 * L) & half
    ue = np.unique(key)
    ea, eb = ue // nn, ue % nn

    def fix(m):
        m[plane, 1] = 0.0
        m[used] /= np.linalg.norm(m[used], axis=1)[:, None]
        return m

    # Incident triangle normals per node (padded with the first one; mirrored copies on y = 0, where the normal must
    # see the mirror image of the wall too).
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

    # Most visible direction (axis of the smallest cap of the incident normals, Badoiu-Clarkson iteration) starting
    # from the angle-weighted mean: the layers do not collapse on any incident triangle.
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
    v1 = vis(nrm)
    print("boundary layer normals: smallest cosine to an incident triangle %.3f (before smoothing %.3f), %d nodes below 0.3" % (v1.min(), v0.min(), int((v1 < 0.3).sum())))
    # gmsh looks the normal up by position. A list of triangles is ambiguous where two walls face each other within the
    # bounding box of a triangle (the two sides of a rolled fin): the point is projected onto the plane of the other
    # side. A thin shell of tetrahedra on the outside of each triangle (3 per triangle, height 0.02 wall_size, well
    # below the distance of the nearest other wall node) contains each wall node only within its own fan. (Much
    # thinner shells make the point location fail on some nodes: zero normal, degenerate prisms.)
    eps = 0.02 * h0
    v6 = np.concatenate([xyz[tri], xyz[tri] + eps * nrm[tri]], axis=1)  # (nt, 6, 3): a b c a' b' c'
    n6 = np.concatenate([nrm[tri], nrm[tri]], axis=1)
    tets = [[0, 1, 2, 3], [1, 2, 3, 4], [2, 3, 4, 5]]
    xs = np.concatenate([v6[:, t] for t in tets])  # (3 nt, 4, 3)
    ns = np.concatenate([n6[:, t] for t in tets])
    view = gmsh.view.add("bl_normals")
    data = np.concatenate([xs.transpose(0, 2, 1).reshape(-1, 12), ns.reshape(-1, 12)], axis=1)
    gmsh.view.addListData(view, "VS", len(xs), data.ravel().tolist())
    print("boundary layer normals: %d triangles, %d surface components, %d nodes on y = 0 (max |y| %.3g)" % (nt, ncomp, int(plane.sum()), float(np.abs(xyz[plane, 1]).max(initial=0.0))))
    return view


def rerun(q, why):
    """Runs this script again in a fresh process with the parameters `q` (written next to the original ones)."""
    import os

    path = os.path.join(os.path.dirname(os.path.abspath(sys.argv[1])), "params_retry.json")
    json.dump(q, open(path, "w"), indent=1)
    print(why)
    sys.stdout.flush()
    gmsh.finalize()
    os.execv(sys.executable, [sys.executable, os.path.abspath(sys.argv[0]), path])


def boundary_layer_mesh(parts):
    """Half model y >= 0 with prism layers on wall and base, tetrahedra in the half box farfield; writes the SU2 mesh
    (mirrored to the full domain for y_mirror) and the statistics, then exits."""
    import numpy as np

    import su2_writer as sw

    if zmirror:
        raise SystemExit("mesh error: the prismatic boundary layer cannot be meshed as a quarter model (z_mirror)")
    tol = 1e-4 * L
    # The half solid y >= 0 (the cut faces at y = 0 are dropped; the symmetry plane is rebuilt around the layers). The
    # full domain uses both halves, made conformal: the apex of the periodic body of revolution meshes as a
    # non-manifold fan otherwise.
    upper = occ.addBox(-L, 0.0, -2.0 * L, 3.0 * L, 2.0 * L, 4.0 * L)
    if half:
        occ.intersect(parts, [(3, upper)])
    else:
        lower = occ.addBox(-L, -2.0 * L, -2.0 * L, 3.0 * L, 2.0 * L, 4.0 * L)
        a, _ = occ.intersect(occ.copy(parts), [(3, upper)])
        b, _ = occ.intersect(parts, [(3, lower)])
        occ.fragment(a, b)
    occ.synchronize()
    vols = gmsh.model.getEntities(3)
    if len(vols) != (1 if half else 2):
        raise SystemExit("mesh error: boolean operations produced %d volumes (expected %d); the geometry may be invalid" % (len(vols), 1 if half else 2))
    wall, cap = [], []
    outer = [t for _, t in gmsh.model.getBoundary(vols, combined=True, oriented=False)]
    cut = [t for _, t in gmsh.model.getEntities(2) if t not in outer]
    for t in outer:
        b = gmsh.model.getBoundingBox(2, t)
        if half and b[4] - b[1] < tol and abs(b[1]) < tol:
            cut.append(t)
        elif b[0] > L - tol:
            cap.append(t)
        else:
            wall.append(t)
    print("faces: wall %d, base %d, cut %d" % (len(wall), len(cap), len(cut)))
    if not wall or not cap or not cut:
        raise SystemExit("mesh error: could not classify the boundary faces")
    occ.remove(vols)
    occ.remove([(2, t) for t in cut])
    occ.synchronize()

    f = gmsh.model.mesh.field
    f.add("Distance", 1)
    f.setNumbers(1, "SurfacesList", wall + cap)
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
    t1 = time.time()
    gmsh.model.mesh.generate(2)

    view = bl_normals(wall + cap, int(bl["smoothing"]))
    heights, h, s = [], bl["first_height"], 0.0
    for _ in range(int(bl["layers"])):
        s += h
        heights.append(s)
        h *= bl["ratio"]
    geo = gmsh.model.geo
    ext = geo.extrudeBoundaryLayer([(2, t) for t in wall + cap], [1] * len(heights), heights, True, False, gmsh.view.getIndex(view))
    top = [ext[i - 1] for i in range(1, len(ext)) if ext[i][0] == 3]
    lateral = [e[1] for e in ext if e[0] == 2 and e not in top]
    geo.synchronize()
    # Box farfield |x - x0|, |y|, |z| <= R (half: y >= 0).
    xs, ys, zs = (x0 - R, x0 + R), (0.0 if half else -R, R), (-R, R)
    c = {(i, j, k): geo.addPoint(xs[i], ys[j], zs[k], hmax) for i in (0, 1) for j in (0, 1) for k in (0, 1)}
    lines = {}

    def line(u, v):
        if (v, u) in lines:
            return -lines[(v, u)]
        if (u, v) not in lines:
            lines[(u, v)] = geo.addLine(u, v)
        return lines[(u, v)]

    def loop(q):
        return geo.addCurveLoop([line(c[q[i]], c[q[(i + 1) % 4]]) for i in range(4)])

    y0 = [(0, 0, 0), (1, 0, 0), (1, 0, 1), (0, 0, 1)]
    if half:
        # The symmetry plane with the section of the layers as holes.
        holes = geo.addCurveLoops([c[1] for c in gmsh.model.getBoundary(top, combined=True)])
        sym = [geo.addPlaneSurface([loop(y0)] + list(holes))]
    else:
        sym = []
    far = ([] if half else [geo.addPlaneSurface([loop(y0)])]) + [
        geo.addPlaneSurface([loop(q)])
        for q in (
            [(0, 1, 0), (1, 1, 0), (1, 1, 1), (0, 1, 1)],
            [(0, 0, 0), (1, 0, 0), (1, 1, 0), (0, 1, 0)],
            [(0, 0, 1), (1, 0, 1), (1, 1, 1), (0, 1, 1)],
            [(0, 0, 0), (0, 1, 0), (0, 1, 1), (0, 0, 1)],
            [(1, 0, 0), (1, 1, 0), (1, 1, 1), (1, 0, 1)],
        )
    ]
    if half:
        geo.addVolume([geo.addSurfaceLoop([t for _, t in top] + sym + far)])
    else:
        geo.addVolume([geo.addSurfaceLoop(far), geo.addSurfaceLoop([t for _, t in top])])
    geo.synchronize()

    gmsh.option.setNumber("Mesh.Algorithm3D", int(p["algorithm3d"]))
    t_surf = time.time()
    try:
        gmsh.model.mesh.generate(3)
    except Exception as e:  # noqa: BLE001
        if int(p["algorithm3d"]) == 1:
            raise SystemExit("mesh error: the tetrahedral mesh outside the boundary layer failed (%s)" % e)
        rerun(dict(p, algorithm3d=1), "HXT failed (%s); retrying with Delaunay" % e)
    t2 = time.time()

    # ---- cells, markers, checks --------------------------------------------------------------------
    tags, coord, _ = gmsh.model.mesh.getNodes()
    idx = np.zeros(int(tags.max()) + 1, np.int64)
    idx[tags] = np.arange(len(tags))
    xyz = coord.reshape(-1, 3).copy()
    xyz[np.abs(xyz[:, 1]) < 1e-12, 1] = 0.0  # the symmetry plane exactly (OCC leaves a few 1e-16 m)
    cells, names, etags_all = {}, {}, []
    for et, et_tags, en in zip(*gmsh.model.mesh.getElements(3)):
        kind = sw.GMSH.get(int(et))
        if kind not in (sw.TET, sw.PRISM, sw.PYRAMID):
            raise SystemExit("mesh error: unexpected volume element type %d" % et)
        cells[kind] = idx[np.asarray(en, np.int64)].reshape(-1, sw.NODES[kind])
        names[kind] = gmsh.model.mesh.getElementProperties(int(et))[0]
        if kind != sw.PRISM:
            etags_all.append(et_tags)  # the layer prisms are thin by design; they are checked by their corner Jacobians

    def faces(ents, keep=None):
        out = []
        for t in ents:
            for e, n in zip(*gmsh.model.mesh.getElements(2, t)[::2]):
                kind = sw.GMSH[int(e)]
                fc = idx[np.asarray(n, np.int64)].reshape(-1, sw.NODES[kind])
                out.append((kind, fc if keep is None else fc[keep(fc)]))
        return out

    on_plane = lambda fc: (np.abs(xyz[fc][:, :, 1]) < 1e-12).all(axis=1)  # noqa: E731
    markers = [("wall", faces(wall)), ("base", faces(cap)), ("farfield", faces(far))]
    if half:
        markers.append(("symmetry", faces(sym) + faces(lateral, on_plane)))
    cells, markers, xyz, orphans = sw.compact(cells, markers, xyz)
    flipped = sw.orient(cells, xyz)
    bad, nonpos = sw.check(cells, xyz)
    missing, extra = sw.unmarked_faces(cells, markers)
    extra += orphans
    nsym = {k: sum(len(fc) for kk, fc in dict(markers).get("symmetry", []) if kk == k) for k in (sw.TRI, sw.QUAD)}
    print("boundary layer: %d layers, first %.3g m, ratio %.4f, thickness %.3g m; %s; %d cells reoriented, %d inverted prisms, %d cells of non-positive volume; symmetry %d triangles + %d quads; %d boundary faces without marker, %d marker faces inside"
          % (len(heights), heights[0], bl["ratio"], heights[-1], ", ".join("%s %d" % (names[k], len(cells[k])) for k in cells), flipped, bad, nonpos,
             nsym[sw.TRI], nsym[sw.QUAD], missing, extra))
    if bad or nonpos or missing or extra:
        what = "%d inverted prisms, %d cells of non-positive volume, %d boundary faces without marker, %d marker faces inside" % (bad, nonpos, missing, extra)
        if bl["retry"]:
            r = bl["retry"]
            rerun(dict(p, bl=r), "boundary layer invalid (%s); retrying with %d smoothing passes and thickness %.3g m" % (what, r["smoothing"], r["thickness"]))
        raise SystemExit("mesh error: the prismatic boundary layer is invalid: %s (try a different wall_size or yplus)" % what)

    q = gmsh.model.mesh.getElementQualities(np.concatenate(etags_all).tolist(), "minSICN")
    q_min, q_mean, q_bad = float(np.min(q)), float(np.mean(q)), int((np.asarray(q) < 0.05).sum())
    mult = 1
    if p["y_mirror"]:
        cells, markers, xyz = sw.mirror(cells, markers, xyz, 1, "symmetry", 1e-12)
        mult = 2
    ncells = sw.write(p["out_su2"], cells, markers, xyz)
    stats = {
        "nodes": int(len(xyz)),
        "cells": int(ncells),
        "by_type": {names[k]: int(len(c)) for k, c in cells.items()},
        "quality_sicn_min": q_min,
        "quality_sicn_mean": q_mean,
        "cells_sicn_below_0.05": q_bad * mult,
        "wall_triangles": int(sum(len(fc) for _, fc in dict(markers)["wall"])),
        "seconds_setup": t1 - t0,
        "seconds_mesh": t2 - t1,
        "bl_layers": len(heights),
        "bl_first_height": heights[0],
        "bl_thickness": heights[-1],
        "bl_invalid": bad,
    }
    json.dump(stats, open(p["stats_json"], "w"), indent=1)
    print("MESH_OK nodes=%d cells=%d minSICN=%.4f surface %.1fs volume %.1fs" % (stats["nodes"], stats["cells"], q_min, t_surf - t1, t2 - t_surf))
    gmsh.finalize()
    sys.exit(0)


if bl:
    boundary_layer_mesh(parts)

if half:
    # Half ball y >= 0 as a sphere sector (azimuth 0..pi about the z axis): no boolean on the sphere.
    # Quarter ball (z >= 0 too) for z_mirror: latitude 0..pi/2.
    domain = [(3, occ.addSphere(x0, 0, 0, R, angle1=0.0 if zmirror else -math.pi / 2, angle2=math.pi / 2, angle3=math.pi))]
else:
    domain = [(3, occ.addBox(x0 - R, -R, -R, 2 * R, 2 * R, 2 * R))]
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
    elif max(b[3] - b[0], b[4] - b[1], b[5] - b[2]) > 0.5 * R:
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
if zmirror:
    mirrored = mirror_su2(p["out_su2"], 2, "symz", 1e-9 * L)
    wall_mult *= 2
if p["y_mirror"]:
    mirrored = mirror_su2(p["out_su2"], 1, "symmetry", 1e-9 * L)
    wall_mult *= 2

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
    stats["cells"], stats["nodes"], stats["wall_triangles"] = mirrored[0], mirrored[1], wall_mult * nwall
    stats["by_type"] = {"Tetrahedron 4": mirrored[0]}
if stats["cells"] == 0:
    raise SystemExit("mesh error: the volume mesh is empty (3D meshing failed)")
json.dump(stats, open(p["stats_json"], "w"), indent=1)
print("MESH_OK nodes=%d cells=%d minSICN=%.4f surface %.1fs volume %.1fs" % (stats["nodes"], stats["cells"], q_min, t_surf - t1, t2 - t_surf))
gmsh.finalize()
