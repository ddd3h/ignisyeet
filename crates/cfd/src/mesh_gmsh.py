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
def wall_normals(tri, xyz, passes, corner=0.0, corner_passes=0):
    """Orients the wall triangles `tri` (indices into `xyz`) consistently and outward (into the fluid; the signed
    volume is closed by y = 0) and returns (tri, node normals, concave edges). The normals point to the most visible
    direction of the incident triangles (mirrored copies on y = 0, where they lie in the plane) and are smoothed
    `passes` times where that keeps them visible, and `corner_passes` more times within `corner` of a concave edge
    (they fan out of the corner like a fillet). Concave edges: interior edges whose two triangles meet at a concave
    dihedral angle of more than 30 degrees (the fin-body junction)."""
    import collections

    import numpy as np

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
        raise SystemExit("mesh error: the wall surface mesh is not manifold (%d edges with more than two triangles)" % int((counts > 2).sum()))
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
                    raise SystemExit("mesh error: the wall surface mesh is not orientable")
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
    plane = used & (np.abs(xyz[:, 1]) < 1e-9 * L)
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
    # Concave edges: the other triangle lies in front of this one.
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
    import numpy as np

    pa, pb = xyz[edges[:, 0]], xyz[edges[:, 1]]
    pts = np.concatenate([pa + (pb - pa) * s for s in np.linspace(0.0, 1.0, 5)])
    dist = np.full(len(q), np.inf)
    for i0 in range(0, len(q), 2000):
        dist[i0:i0 + 2000] = np.linalg.norm(q[i0:i0 + 2000, None, :] - pts[None, :, :], axis=2).min(axis=1)
    return dist


def column_heights(h1, n, thick):
    """Cumulative layer heights (n + 1, m) of columns with first height h1, n layers and total thicknesses `thick`
    (geometric growth, ratio solved per column; at least n h1)."""
    import numpy as np

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
    import os

    path = os.path.join(os.path.dirname(os.path.abspath(sys.argv[1])), "params_retry.json")
    json.dump(q, open(path, "w"), indent=1)
    print(why)
    sys.stdout.flush()
    gmsh.finalize()
    os.execv(sys.executable, [sys.executable, os.path.abspath(sys.argv[0]), path])


def boundary_layer_mesh(parts):
    """Half model y >= 0: prism layers built here on the wall and base mesh, tetrahedra in the half box farfield
    (gmsh); writes the SU2 mesh (mirrored to the full domain for y_mirror) and the statistics, then exits."""
    import numpy as np

    import su2_writer as sw

    if not half or zmirror:
        raise SystemExit("mesh error: the prismatic boundary layer needs the half model (a fin layout symmetric about y = 0) without z_mirror")
    tol = 1e-4 * L
    box = occ.addBox(-L, 0.0, -2.0 * L, 3.0 * L, 2.0 * L, 4.0 * L)
    occ.intersect(parts, [(3, box)])
    occ.synchronize()
    vols = gmsh.model.getEntities(3)
    if len(vols) != 1:
        raise SystemExit("mesh error: boolean operations produced %d volumes (expected 1); the geometry may be invalid" % len(vols))
    wall, cap, cut = [], [], []
    for _, t in gmsh.model.getBoundary(vols, oriented=False):
        b = gmsh.model.getBoundingBox(2, t)
        if b[4] - b[1] < tol and abs(b[1]) < tol:
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

    # ---- wall mesh and normals -----------------------------------------------------------------------
    tags, coord, _ = gmsh.model.mesh.getNodes()
    idx = np.zeros(int(tags.max()) + 1, np.int64)
    idx[tags] = np.arange(len(tags))
    xyz = coord.reshape(-1, 3).copy()
    xyz[np.abs(xyz[:, 1]) < 1e-12, 1] = 0.0
    tris, kinds = [], []
    for kind, ents in ((0, wall), (1, cap)):
        for t in ents:
            for e, n in zip(*gmsh.model.mesh.getElements(2, t)[::2]):
                if e != 2:
                    raise SystemExit("mesh error: the wall surface mesh has non-triangular elements")
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

    # ---- column thickness ------------------------------------------------------------------------------
    # The layers grow at bl["ratio"] from bl["first_height"] up to a last layer of about the size of the tetrahedra
    # outside (bl["thickness"] in all). Near a concave edge (fin-body junction) the columns of the two walls would
    # cross: there the thickness is at most bl["junction"] times the distance to the edge (the layer count stays,
    # the growth ratio of the column is smaller).
    n_lay, h1 = int(bl["layers"]), bl["first_height"]
    thick = np.full(m, bl["thickness"])
    if len(ce):
        thick = np.minimum(thick, bl["junction"] * bl["thickness"] + distance_to_edges(x0w, x0w, ce))
    # Columns on the flat base: at most bl["base_thickness"] (the wake next to the base).
    on_base = np.zeros(m, bool)
    on_base[tl[kinds == 1].ravel()] = True
    thick[on_base] = np.minimum(thick[on_base], bl["base_thickness"])
    # Where neighbouring columns diverge by an angle a (fanning round a fin edge or tip), the layer top widens by
    # about thickness * a: at most bl["fan"] wall sizes, so the tetrahedra on the top stay well shaped.
    e0 = np.unique(np.sort(np.concatenate([tl[:, [0, 1]], tl[:, [1, 2]], tl[:, [2, 0]]]), axis=1), axis=0)
    ang = np.arccos(np.clip(np.einsum("ij,ij->i", nw[e0[:, 0]], nw[e0[:, 1]]), -1.0, 1.0))
    spread = np.zeros(m)
    np.maximum.at(spread, e0[:, 0], ang)
    np.maximum.at(spread, e0[:, 1], ang)
    thick = np.minimum(thick, bl["fan"] * h0 / np.maximum(spread, 1e-6))
    # Columns of prisms whose corners invert are made thinner (with their neighbours, Lipschitz 0.5) until none does.
    e = np.unique(np.sort(np.concatenate([tl[:, [0, 1]], tl[:, [1, 2]], tl[:, [2, 0]]]), axis=1), axis=0)
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
    nt = len(tl)
    for it in range(30):
        s = column_heights(h1, n_lay, thick)
        lay = x0w[None, :, :] + s[:, :, None] * nw[None, :, :]  # (n + 1, m, 3)
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

    # Layer sides on y = 0 (quadrilaterals) and the boundary loops of the layer top there.
    a_ = np.concatenate([tl[:, 0], tl[:, 1], tl[:, 2]])
    b_ = np.concatenate([tl[:, 1], tl[:, 2], tl[:, 0]])
    k_ = np.minimum(a_, b_) * m + np.maximum(a_, b_)
    uk, cnt = np.unique(k_, return_counts=True)
    single = np.isin(k_, uk[cnt == 1])
    ba, bb = a_[single], b_[single]
    if not (np.abs(x0w[ba, 1]) < 1e-12).all() or not (np.abs(x0w[bb, 1]) < 1e-12).all():
        raise SystemExit("mesh error: the wall has an open edge away from the symmetry plane")
    quads = np.concatenate([np.stack([ba + k * m, bb + k * m, bb + (k + 1) * m, ba + (k + 1) * m], axis=1) for k in range(n_lay)])
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

    # ---- outer surfaces: y = 0 plane around the layers and the box farfield (gmsh, own model) -------------
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
    xs, ys, zs = (x0 - R, x0 + R), (0.0, R), (-R, R)
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

    sym = geo.addPlaneSurface([loop([(0, 0, 0), (1, 0, 0), (1, 0, 1), (0, 0, 1)])] + [h for h, _ in hole_loops])
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
    geo.synchronize()
    for _, lines in hole_loops:
        for ln in lines:
            gmsh.model.mesh.setTransfiniteCurve(abs(ln), 2)
    hl = [abs(ln) for _, lines in hole_loops for ln in lines]
    f.add("Distance", 1)
    f.setNumbers(1, "CurvesList", hl)
    f.setNumber(1, "Sampling", 4)
    f.add("Threshold", 2)
    f.setNumber(2, "InField", 1)
    f.setNumber(2, "SizeMin", bl["top_size"])
    f.setNumber(2, "SizeMax", hmax)
    f.setNumber(2, "DistMin", 0.0)
    f.setNumber(2, "DistMax", (hmax - bl["top_size"]) / (grow - 1.0))
    f.setAsBackgroundMesh(2)
    gmsh.option.setNumber("Mesh.MeshSizeExtendFromBoundary", 1)
    gmsh.option.setNumber("Mesh.MeshSizeFromCurvature", 0)
    gmsh.model.mesh.generate(2)
    otag, ocoord, _ = gmsh.model.mesh.getNodes()
    ocoord = ocoord.reshape(-1, 3)
    # Global node numbers: the layer nodes (level k of wall node i: k m + i), then the outer nodes off the hole.
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
    extra_xyz[np.abs(extra_xyz[:, 1]) < 1e-12, 1] = 0.0

    def surf_tris(s_):
        out = []
        for e_, n_ in zip(*gmsh.model.mesh.getElements(2, s_)[::2]):
            if e_ != 2:
                raise SystemExit("mesh error: unexpected element type %d on the outer surfaces" % e_)
            out.append(np.array([gid[t_] for t_ in n_.tolist()], np.int64).reshape(-1, 3))
        return np.vstack(out)

    sym_tri = surf_tris(sym)
    far_tri = np.vstack([surf_tris(s_) for s_ in far])
    gmsh.model.remove()

    # ---- tetrahedra between the layer top and the outer surfaces (gmsh, discrete model) ------------------
    allxyz = np.vstack([pts_all, extra_xyz])
    top_tri = tl + top
    bnd = np.vstack([top_tri, sym_tri, far_tri])
    bn = np.unique(bnd)
    gmsh.model.add("fluid")
    gmsh.model.addDiscreteEntity(2, 1)
    gmsh.model.mesh.addNodes(2, 1, (bn + 1).tolist(), allxyz[bn].ravel().tolist())
    gmsh.model.mesh.addElementsByType(1, 2, [], (top_tri + 1).ravel().tolist())
    gmsh.model.addDiscreteEntity(2, 2)
    gmsh.model.mesh.addElementsByType(2, 2, [], (sym_tri + 1).ravel().tolist())
    gmsh.model.addDiscreteEntity(2, 3)
    gmsh.model.mesh.addElementsByType(3, 2, [], (far_tri + 1).ravel().tolist())
    gmsh.model.geo.addVolume([gmsh.model.geo.addSurfaceLoop([1, 2, 3])])
    gmsh.model.geo.synchronize()
    f.add("Distance", 1)
    f.setNumbers(1, "SurfacesList", [1])
    f.add("Threshold", 2)
    f.setNumber(2, "InField", 1)
    f.setNumber(2, "SizeMin", bl["top_size"])
    f.setNumber(2, "SizeMax", hmax)
    f.setNumber(2, "DistMin", 0.0)
    f.setNumber(2, "DistMax", (hmax - bl["top_size"]) / (grow - 1.0))
    f.setAsBackgroundMesh(2)
    gmsh.option.setNumber("Mesh.MeshSizeExtendFromBoundary", 1)
    gmsh.option.setNumber("Mesh.Algorithm3D", int(p["algorithm3d"]))
    gmsh.option.setNumber("Mesh.Renumber", 0)  # the node tags of the boundary are the global node numbers + 1
    t_surf = time.time()
    try:
        gmsh.model.mesh.generate(3)
    except Exception as e:  # noqa: BLE001
        if int(p["algorithm3d"]) == 1:
            raise SystemExit("mesh error: the tetrahedral mesh outside the boundary layer failed (%s)" % e)
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
            raise SystemExit("mesh error: unexpected volume element type %d outside the boundary layer" % et)
        tets.append(gmap[np.asarray(en, np.int64)].reshape(-1, 4))
    tets = np.vstack(tets)
    if (tets < 0).any():
        raise SystemExit("mesh error: a tetrahedron uses an unknown node")
    q = gmsh.model.mesh.getElementQualities(gmsh.model.mesh.getElementsByType(4)[0].tolist(), "minSICN")
    q_min, q_mean, q_bad = float(np.min(q)), float(np.mean(q)), int((np.asarray(q) < 0.05).sum())
    del q, ntag, ncoord, npos, gmap, known
    gmsh.model.remove()  # the tetrahedra are copied: free gmsh's mesh before the checks

    # ---- cells, markers, checks --------------------------------------------------------------------
    cells = {sw.PRISM: prisms, sw.TET: tets}
    names = {sw.PRISM: "Prism 6", sw.TET: "Tetrahedron 4"}
    markers = [
        ("wall", [(sw.TRI, tl[kinds == 0])]),
        ("base", [(sw.TRI, tl[kinds == 1])]),
        ("farfield", [(sw.TRI, far_tri)]),
        ("symmetry", [(sw.TRI, sym_tri), (sw.QUAD, quads)]),
    ]
    cells, markers, allxyz, orphans = sw.compact(cells, markers, allxyz)
    flipped = sw.orient(cells, allxyz)
    bad, nonpos = sw.check(cells, allxyz)
    missing, extra = sw.unmarked_faces(cells, markers)
    extra += orphans
    # Size jump at the layer top: height of the tetrahedron on each top triangle over the last layer height there.
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
        raise SystemExit("mesh error: the prismatic boundary layer is invalid: %s (try a different wall_size or yplus)" % what)

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
    stats["quality_sicn_min"], stats["quality_sicn_mean"] = q_min, q_mean
    stats["cells_sicn_below_0.05"] = q_bad * mult
    json.dump(stats, open(p["stats_json"], "w"), indent=1)
    print("MESH_OK nodes=%d cells=%d minSICN=%.4f layers %.1fs surfaces %.1fs volume %.1fs"
          % (stats["nodes"], stats["cells"], stats["quality_sicn_min"], t_layers - t1, t_surf - t_layers, t2 - t_surf))
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
