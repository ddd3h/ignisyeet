# SPDX-License-Identifier: GPL-3.0-or-later
# IgnisYeet prismatic boundary layer (RANS), shared by the mesher scripts (generated into the mesh directory next to them
# and imported). Node normals of the wall surface mesh, prism columns along them, the farfield box around the layer top
# filled with tetrahedra by gmsh, the validity checks of su2_writer.py and the SU2 file.
#
# `build` starts from the surface mesh of the current gmsh model (wall and base faces) and covers:
#   * the half domain (`half`, the wall is open along y = 0; the layer sides there are quadrilaterals of the `symmetry`
#     marker, which also holds the y = 0 plane of the half box around the layer section), or
#   * the full domain (a closed wall in the full box, no symmetry plane).
# Column thickness: bl["thickness"], at most bl["junction"] times the thickness plus the distance to a concave edge (the
# fin-body junction), bl["base_thickness"] on the base, limited where neighbouring columns diverge (bl["fan"] wall
# sizes of widening at the top), Lipschitz 0.5 between neighbours, and thinned where prisms would invert. The
# tetrahedra start at bl["top_size"] on the layer top and grow by `growth` per element row.
import json
import math
import os
import sys
import time

import numpy as np
import gmsh


def _die(code, msg):
    raise SystemExit("[%s] mesh error: %s" % (code, msg))


def wall_normals(tri, xyz, passes, corner=0.0, corner_passes=0, half=True, plane_tol=0.0):
    """Orients the wall triangles `tri` (indices into `xyz`) consistently and outward (into the fluid; the signed
    volume is closed by y = 0 for the half wall) and returns (tri, node normals, concave edges). The normals point to
    the most visible direction of the incident triangles (for the half wall, mirrored copies on y = 0, where nodes within
    `plane_tol` of the plane keep their normal in it) and are smoothed `passes` times where that keeps them visible, and
    `corner_passes` more times within `corner` of a concave edge (they fan out of the corner like a fillet). Concave
    edges: interior edges whose two triangles meet at a concave dihedral angle of more than 30 degrees (the fin-body
    junction)."""
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
        raise SystemExit("[IY-F030] mesh error: the wall surface mesh is not manifold (%d edges with more than two triangles)" % int((counts > 2).sum()))
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
                    raise SystemExit("[IY-F031] mesh error: the wall surface mesh is not orientable")
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
    plane = used & (np.abs(xyz[:, 1]) < plane_tol) if half else np.zeros(nn, bool)
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


def wall_hits(x, n, tri, reach, h):
    """Distance along the normal from each wall node to the wall itself (the first triangle the ray x_i + s n_i,
    0 < s <= reach, passes through; inf if none): the free height of the column over an opposite wall (a lug bore, the
    slot under a rail button flange). Triangles `tri` (indices into `x`) are binned in cubes of `h`; each ray is sampled
    every h / 2 and tested (Moeller-Trumbore) against the triangles of the cubes it passes; the triangles of the node
    itself are skipped."""
    m = len(x)
    out = np.full(m, np.inf)
    if m == 0 or not (np.isfinite(reach) and reach > 0.0 and h > 0.0):
        return out
    lo = np.minimum(x.min(axis=0), (x + reach * n).min(axis=0)) - h
    span = np.maximum(x.max(axis=0), (x + reach * n).max(axis=0)) + h - lo
    dims = np.floor(span / h).astype(np.int64) + 2
    keyf = lambda c: (c[:, 0] * dims[1] + c[:, 1]) * dims[2] + c[:, 2]  # noqa: E731
    # Cubes covered by each triangle's bounding box.
    p0, p1, p2 = x[tri[:, 0]], x[tri[:, 1]], x[tri[:, 2]]
    cmin = np.floor((np.minimum(np.minimum(p0, p1), p2) - lo) / h).astype(np.int64)
    cmax = np.floor((np.maximum(np.maximum(p0, p1), p2) - lo) / h).astype(np.int64)
    ext = cmax - cmin + 1
    tk_list, tt_list = [], []
    for ox in range(int(ext[:, 0].max())):
        for oy in range(int(ext[:, 1].max())):
            for oz in range(int(ext[:, 2].max())):
                sel = np.flatnonzero((ox < ext[:, 0]) & (oy < ext[:, 1]) & (oz < ext[:, 2]))
                tk_list.append(keyf(cmin[sel] + np.array([ox, oy, oz])))
                tt_list.append(sel)
    tkey, ttri = np.concatenate(tk_list), np.concatenate(tt_list)
    o = np.argsort(tkey, kind="stable")
    tkey, ttri = tkey[o], ttri[o]
    # (node, cube) pairs along the rays.
    steps = np.arange(0, int(np.ceil(reach / (0.5 * h))) + 1) * 0.5 * h
    nk = []
    for s_ in steps:
        nk.append(keyf(np.floor((x + s_ * n - lo) / h).astype(np.int64)))
    nk = np.stack(nk, axis=1)  # (m, k)
    node = np.repeat(np.arange(m), nk.shape[1])
    pair = np.unique(np.stack([node, nk.ravel()], axis=1), axis=0)
    eps = 1e-9
    for c0 in range(0, len(pair), 200000):
        pr = pair[c0:c0 + 200000]
        a0 = np.searchsorted(tkey, pr[:, 1], "left")
        cnt = np.searchsorted(tkey, pr[:, 1], "right") - a0
        if not cnt.any():
            continue
        ii = np.repeat(pr[:, 0], cnt)
        off = np.arange(len(ii)) - np.repeat(np.cumsum(cnt) - cnt, cnt)
        tt = ttri[np.repeat(a0, cnt) + off]
        own = (tri[tt] == ii[:, None]).any(axis=1)
        ii, tt = ii[~own], tt[~own]
        if not len(ii):
            continue
        v0, v1, v2 = x[tri[tt, 0]], x[tri[tt, 1]], x[tri[tt, 2]]
        d = n[ii]
        e1, e2 = v1 - v0, v2 - v0
        pv = np.cross(d, e2)
        det = np.einsum("ij,ij->i", e1, pv)
        good = np.abs(det) > eps * np.linalg.norm(e1, axis=1) * np.linalg.norm(e2, axis=1)
        inv = np.where(good, 1.0 / np.where(good, det, 1.0), 0.0)
        tv = x[ii] - v0
        u = np.einsum("ij,ij->i", tv, pv) * inv
        qv = np.cross(tv, e1)
        v = np.einsum("ij,ij->i", d, qv) * inv
        t = np.einsum("ij,ij->i", e2, qv) * inv
        ok = good & (u >= 0.0) & (v >= 0.0) & (u + v <= 1.0) & (t > 1e-6 * h) & (t <= reach)
        if ok.any():
            np.minimum.at(out, ii[ok], t[ok])
    return out


def column_collisions(x, n, t, hn, k=8, frac=0.3):
    """Height caps of crossing columns: column i runs from x_i along n_i for t_i; its points at k heights are compared
    with those of all other columns, and two points closer than `frac` of the smaller local edge length `hn` and than
    half the distance of the two wall nodes (the columns of one smooth wall stay about as far apart as their nodes)
    mean the columns meet. Both columns get 0.6 of the
    lower meeting height as their cap (inf where nothing meets)."""
    m = len(x)
    cap = np.full(m, np.inf)
    lev = (np.arange(1, k + 1) / k)[:, None]
    hs = (lev * t[None, :]).ravel()  # (k m) heights, level-major
    owner = np.tile(np.arange(m), k)
    pts = np.repeat(x[None, :, :], k, axis=0).reshape(-1, 3) + hs[:, None] * np.repeat(n[None, :, :], k, axis=0).reshape(-1, 3)
    c = max(float(np.median(hn)) * frac, 1e-9)
    lo = pts.min(axis=0)
    cell = np.floor((pts - lo) / c).astype(np.int64)
    dims = cell.max(axis=0) + 3
    keyf = lambda q: ((q[:, 0] + 1) * dims[1] + q[:, 1] + 1) * dims[2] + q[:, 2] + 1  # noqa: E731
    key = keyf(cell)
    order = np.argsort(key, kind="stable")
    ks = key[order]
    for dx in (-1, 0, 1):
        for dy in (-1, 0, 1):
            for dz in (-1, 0, 1):
                tk = keyf(cell + np.array([dx, dy, dz]))
                a0 = np.searchsorted(ks, tk, "left")
                cnt = np.searchsorted(ks, tk, "right") - a0
                if not cnt.any():
                    continue
                aa = np.repeat(np.arange(len(pts)), cnt)
                off = np.arange(len(aa)) - np.repeat(np.cumsum(cnt) - cnt, cnt)
                bb = order[np.repeat(a0, cnt) + off]
                i, j = owner[aa], owner[bb]
                ok = i != j
                dd = np.linalg.norm(pts[aa] - pts[bb], axis=1)
                ok &= (dd < frac * np.minimum(hn[i], hn[j])) & (dd < 0.5 * np.linalg.norm(x[i] - x[j], axis=1))
                if ok.any():
                    meet = 0.6 * np.minimum(hs[aa], hs[bb])[ok]
                    np.minimum.at(cap, i[ok], meet)
                    np.minimum.at(cap, j[ok], meet)
    return cap


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


def column_heights_padded(h1, n, thick, g):
    """Cumulative layer heights (n + 1, m) for gap mode: a column that is too thin for n layers growing by `g` from h1
    starts with layers of h1 and grows by `g` only over its top layers, so its last layer is as high as the growth
    allows (about (g - 1) / g of the column instead of about 1 / n). A column of the full geometric thickness keeps the
    plain geometric heights; the heights are scaled to the column thickness."""
    t = np.maximum(thick, n * h1 * (1.0 + 1e-9))
    mm = np.arange(n)
    tot = (n - mm) * h1 + h1 * g * (g ** mm - 1.0) / (g - 1.0)
    big = np.clip(np.searchsorted(tot, t, side="right") - 1, 0, n - 1)
    j = np.arange(n)[:, None]
    start = (n - big)[None, :]
    h = np.where(j < start, h1, h1 * g ** (j - start + 1.0))
    h = h * (t / tot[big])[None, :]
    return np.vstack([np.zeros((1, len(t))), np.cumsum(h, axis=0)])


def rerun(q, why):
    """Runs this script again in a fresh process with the parameters `q` (written next to the original ones)."""
    path = os.path.join(os.path.dirname(os.path.abspath(sys.argv[1])), "params_retry.json")
    json.dump(q, open(path, "w"), indent=1)
    print(why)
    sys.stdout.flush()
    gmsh.finalize()
    os.execv(sys.executable, [sys.executable, os.path.abspath(sys.argv[0]), path])


def build(p, wall, cap, half, t0, t1, invalid_hint=" (try a different wall_size or yplus)", tet_hint="", gap=None):
    """Prism layers on the surface mesh of the faces `wall` and `cap` (base) of the current gmsh model, tetrahedra in
    the box farfield (half box y >= 0 when `half`); writes the SU2 mesh (mirrored to the full domain for p["y_mirror"])
    and the statistics, then exits. `t0`, `t1`: start of the script and of the surface mesh (statistics). `gap`: columns
    facing an opposite wall are at most this fraction of the distance to it (see facing_gaps; None = no gap rule)."""
    import su2_writer as sw

    bl = p["bl"]
    L = p["length"]
    R = p["farfield_radius"]
    x0 = p["center_x"]
    h0 = p["wall_size"]
    hmax = p["max_size"]
    grow = p["growth"]

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
                    _die("IY-F032", "the wall surface mesh has non-triangular elements")
                tris.append(idx[np.asarray(n, np.int64)].reshape(-1, 3))
                kinds.append(np.full(len(tris[-1]), kind))
    tri, kinds = np.vstack(tris), np.concatenate(kinds)
    tri, nrm, concave = wall_normals(tri, xyz, int(bl["smoothing"]), bl["thickness"], int(bl["corner_smoothing"]), half, 1e-9 * L)
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
    if gap is not None:
        # Narrow gaps between opposite walls (a lug bore, the slot under a rail button flange): the layers of the two
        # walls would cross and the layer top intersect itself.
        reach = bl["thickness"] / gap
        dgap = wall_hits(x0w, nw, tl, reach, float(np.median(np.linalg.norm(x0w[e0[:, 0]] - x0w[e0[:, 1]], axis=1))))
        narrow = dgap < np.inf
        thick[narrow] = np.minimum(thick[narrow], gap * dgap[narrow])
        print("boundary layer: %d columns reach the wall again within %.3g m (closest %.3g m)"
              % (int(narrow.sum()), reach, float(dgap.min()) if narrow.any() else float("nan")))
    # Columns of prisms whose corners invert are made thinner (with their neighbours, Lipschitz 0.5) until none does.
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
    hnode = np.zeros(m)
    np.maximum.at(hnode, e[:, 0], elen)
    np.maximum.at(hnode, e[:, 1], elen)
    for it in range(30):
        s = column_heights(h1, n_lay, thick) if gap is None else column_heights_padded(h1, n_lay, thick, bl["ratio"])
        lay = x0w[None, :, :] + s[:, :, None] * nw[None, :, :]  # (n + 1, m, 3)
        pts_all = lay.reshape(-1, 3)
        prisms = np.concatenate([np.concatenate([tl + k * m, tl + (k + 1) * m], axis=1) for k in range(n_lay)])
        jac = sw.prism_corner_jacobians(pts_all, prisms).min(axis=1)
        bad = jac <= 0.0
        if not bad.any() and gap is not None:
            # Columns of different walls that come closer than a third of the local edge length cross (concave slots
            # such as the gap between a rail button flange and the body): both are cut below the meeting point.
            cap_ = column_collisions(x0w, nw, thick, hnode)
            hit = cap_ < thick
            if hit.any():
                print("boundary layer: %d columns cross other columns; thinning them" % int(hit.sum()))
                thick = limit(np.minimum(thick, cap_))
                continue
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
        _die("IY-F033", "the wall has %d open edges (the shape is not closed)" % len(ba))
    if half and (not (np.abs(x0w[ba, 1]) < 1e-12).all() or not (np.abs(x0w[bb, 1]) < 1e-12).all()):
        _die("IY-F034", "the wall has an open edge away from the symmetry plane")
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
    if half:
        extra_xyz[np.abs(extra_xyz[:, 1]) < 1e-12, 1] = 0.0

    def surf_tris(s_):
        out = []
        for e_, n_ in zip(*gmsh.model.mesh.getElements(2, s_)[::2]):
            if e_ != 2:
                _die("IY-F035", "unexpected element type %d on the outer surfaces" % e_)
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
    # Gap mode: the faces of the layer top over thin columns (whose last layer is far below the tetrahedra that would
    # sit on them) become separate discrete surfaces, grouped by the size of the tetrahedra they need: twice the last layer
    # height (the tetrahedra come out about 1.6 times as high as that size), but at least a third of the face (smaller
    # ones would be slivers), in halvings of top_size; each group gets its own size field growing by `growth`.
    sets, groups = list(outer_sets), []
    if gap is not None:
        h_end = s[n_lay] - s[n_lay - 1]
        tp = toplay[tl]
        edge = (np.linalg.norm(tp[:, 1] - tp[:, 0], axis=1) + np.linalg.norm(tp[:, 2] - tp[:, 1], axis=1)
                + np.linalg.norm(tp[:, 0] - tp[:, 2], axis=1)) / 3.0
        # Not below a third of the face size: smaller tetrahedra on the face would be slivers.
        want = np.maximum(2.0 * h_end[tl].mean(axis=1), edge / 3.0)
        lim = 0.5 * bl["top_size"]
        level = np.where(want < lim, np.ceil(np.log2(lim / np.maximum(want, 1e-12))), 0).astype(np.int64)
        sets[0] = top_tri[level == 0]
        for lv in sorted(set(level[level > 0].tolist())):
            sets.append(top_tri[level == lv])
            groups.append((len(sets), lim / 2.0 ** lv))
        print("boundary layer: %d of %d top faces in %d refined groups (smallest tetrahedron size %.3g m)"
              % (int((level > 0).sum()), len(level), len(groups), min([g_[1] for g_ in groups], default=lim)))
    for k, st in enumerate(sets, start=1):
        gmsh.model.addDiscreteEntity(2, k)
        if k == 1:
            gmsh.model.mesh.addNodes(2, 1, (bn + 1).tolist(), allxyz[bn].ravel().tolist())
        gmsh.model.mesh.addElementsByType(k, 2, [], (st + 1).ravel().tolist())
    tops = [1] + [g_[0] for g_ in groups]
    if half:
        gmsh.model.geo.addVolume([gmsh.model.geo.addSurfaceLoop([1, 2, 3] + tops[1:])])
    else:
        # Two closed shells: the farfield box and the layer top.
        gmsh.model.geo.addVolume([gmsh.model.geo.addSurfaceLoop([2]), gmsh.model.geo.addSurfaceLoop(tops)])
    gmsh.model.geo.synchronize()
    f.add("Distance", 1)
    f.setNumbers(1, "SurfacesList", tops)
    f.add("Threshold", 2)
    f.setNumber(2, "InField", 1)
    f.setNumber(2, "SizeMin", bl["top_size"])
    f.setNumber(2, "SizeMax", hmax)
    f.setNumber(2, "DistMin", 0.0)
    f.setNumber(2, "DistMax", (hmax - bl["top_size"]) / (grow - 1.0))
    if groups:
        mins = [2]
        for j, (ent, size) in enumerate(groups):
            fd, ft = 10 + 2 * j, 11 + 2 * j
            f.add("Distance", fd)
            f.setNumbers(fd, "SurfacesList", [ent])
            f.add("Threshold", ft)
            f.setNumber(ft, "InField", fd)
            f.setNumber(ft, "SizeMin", size)
            f.setNumber(ft, "SizeMax", hmax)
            f.setNumber(ft, "DistMin", 0.0)
            f.setNumber(ft, "DistMax", (hmax - size) / (grow - 1.0))
            mins.append(ft)
        f.add("Min", 3)
        f.setNumbers(3, "FieldsList", mins)
        f.setAsBackgroundMesh(3)
    else:
        f.setAsBackgroundMesh(2)
    gmsh.option.setNumber("Mesh.MeshSizeExtendFromBoundary", 1)
    gmsh.option.setNumber("Mesh.Algorithm3D", int(p["algorithm3d"]))
    gmsh.option.setNumber("Mesh.Renumber", 0)  # the node tags of the boundary are the global node numbers + 1
    t_surf = time.time()
    try:
        gmsh.model.mesh.generate(3)
    except Exception as e:  # noqa: BLE001
        if int(p["algorithm3d"]) == 1:
            _die("IY-F036", "the tetrahedral mesh outside the boundary layer failed (%s)%s" % (e, tet_hint))
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
            _die("IY-F035", "unexpected volume element type %d outside the boundary layer" % et)
        tets.append(gmap[np.asarray(en, np.int64)].reshape(-1, 4))
    tets = np.vstack(tets)
    if (tets < 0).any():
        _die("IY-F037", "a tetrahedron uses an unknown node")
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
    ]
    if half:
        markers.append(("symmetry", [(sw.TRI, sym_tri), (sw.QUAD, quads)]))
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
        _die("IY-F038", "the prismatic boundary layer is invalid: %s%s" % (what, invalid_hint))

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
