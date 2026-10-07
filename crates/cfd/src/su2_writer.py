# SPDX-License-Identifier: GPL-3.0-or-later
# IgnisYeet SU2 mesh writer (generated into the mesh directory next to the mesher scripts).
#
# Mixed meshes from gmsh: tetrahedra, prisms and pyramids, boundary triangles and quadrilaterals. Cells are
# oriented by their signed volume, written in the SU2 (VTK) node order, optionally mirrored about a
# coordinate plane, and checked for inverted prisms.
import numpy as np

# SU2 / VTK element types.
TRI, QUAD, TET, PRISM, PYRAMID = 5, 9, 10, 13, 14
NODES = {TRI: 3, QUAD: 4, TET: 4, PRISM: 6, PYRAMID: 5}
# gmsh element type -> SU2 type.
GMSH = {2: TRI, 3: QUAD, 4: TET, 6: PRISM, 7: PYRAMID}
# Node permutation that reverses the orientation of an element (gmsh / SU2 conventions alike).
FLIP = {
    TRI: [0, 2, 1],
    QUAD: [0, 3, 2, 1],
    TET: [0, 2, 1, 3],
    PRISM: [0, 2, 1, 3, 5, 4],
    PYRAMID: [0, 3, 2, 1, 4],
}
# gmsh node order -> SU2 (VTK) node order of a positively oriented cell. A gmsh prism has its bottom triangle
# (0, 1, 2) turning counter-clockwise when seen from the top triangle (3, 4, 5); VTK wants the opposite.
TO_SU2 = {TET: [0, 1, 2, 3], PRISM: [0, 2, 1, 3, 5, 4], PYRAMID: [0, 1, 2, 3, 4]}


def _det(a, b, c):
    return np.einsum("ij,ij->i", np.cross(a, b), c)


def signed_volume(kind, xyz, conn):
    """Signed volume of the cells `conn` (gmsh node order, indices into `xyz`); positive = gmsh orientation."""
    p = [xyz[conn[:, k]] for k in range(conn.shape[1])]
    if kind == TET:
        return _det(p[1] - p[0], p[2] - p[0], p[3] - p[0]) / 6.0
    if kind == PRISM:
        return (
            _det(p[1] - p[0], p[2] - p[0], p[3] - p[0]) + _det(p[2] - p[1], p[3] - p[1], p[4] - p[1]) + _det(p[3] - p[2], p[4] - p[2], p[5] - p[2])
        ) / 6.0
    if kind == PYRAMID:
        return (_det(p[1] - p[0], p[2] - p[0], p[4] - p[0]) + _det(p[2] - p[0], p[3] - p[0], p[4] - p[0])) / 6.0
    raise ValueError(kind)


def prism_corner_jacobians(xyz, conn):
    """Corner Jacobians (6 per prism, gmsh order): all positive for a valid gmsh-oriented prism."""
    p = [xyz[conn[:, k]] for k in range(6)]
    out = []
    for (a, b, c, up) in ((0, 1, 2, 3), (1, 2, 0, 4), (2, 0, 1, 5)):
        out.append(_det(p[b] - p[a], p[c] - p[a], p[up] - p[a]))
    for (a, b, c, down) in ((3, 4, 5, 0), (4, 5, 3, 1), (5, 3, 4, 2)):
        out.append(_det(p[b] - p[a], p[c] - p[a], p[a] - p[down]))
    return np.stack(out, axis=1)


def orient(cells, xyz):
    """Flips the cells of negative signed volume to the gmsh orientation; returns the flip count."""
    flipped = 0
    for kind, conn in cells.items():
        neg = signed_volume(kind, xyz, conn) < 0.0
        if neg.any():
            conn[neg] = conn[neg][:, FLIP[kind]]
            flipped += int(neg.sum())
    return flipped


def check(cells, xyz):
    """(inverted prisms, cells of non-positive volume): prisms whose corner Jacobians are not all positive and
    cells whose signed volume is not positive (after `orient`)."""
    bad_prisms = 0
    if PRISM in cells and len(cells[PRISM]):
        j = prism_corner_jacobians(xyz, cells[PRISM])
        bad_prisms = int((j.min(axis=1) <= 0.0).sum())
    nonpos = sum(int((signed_volume(k, xyz, c) <= 0.0).sum()) for k, c in cells.items())
    return bad_prisms, nonpos


# Faces of each cell type (gmsh node order): triangles and quadrilaterals.
CELL_FACES = {
    TET: ([[0, 1, 2], [0, 1, 3], [0, 2, 3], [1, 2, 3]], []),
    PRISM: ([[0, 1, 2], [3, 4, 5]], [[0, 1, 4, 3], [1, 2, 5, 4], [0, 2, 5, 3]]),
    PYRAMID: ([[0, 1, 4], [1, 2, 4], [2, 3, 4], [0, 3, 4]], [[0, 1, 2, 3]]),
}


def unmarked_faces(cells, markers):
    """(boundary faces of the cells that are in no marker, marker faces that are not boundary faces)."""
    missing = extra = 0
    for kind, slot in ((TRI, 0), (QUAD, 1)):
        n = NODES[kind]
        allf = [np.sort(c[:, f], axis=1) for k, c in cells.items() for f in CELL_FACES[k][slot]]
        allf = np.vstack(allf) if allf else np.zeros((0, n), np.int64)
        u, cnt = np.unique(allf, axis=0, return_counts=True)
        bnd = u[cnt == 1]
        mk = [np.sort(c, axis=1) for _, faces in markers for k, c in faces if k == kind and len(c)]
        mk = np.unique(np.vstack(mk), axis=0) if mk else np.zeros((0, n), np.int64)
        both = np.unique(np.vstack([bnd, mk]), axis=0, return_counts=True)[1]
        common = int((both == 2).sum())
        missing += len(bnd) - common
        extra += len(mk) - common
    return missing, extra


def compact(cells, markers, xyz):
    """Drops the nodes no cell uses and renumbers from 0. Returns (cells, markers, xyz, orphans): marker faces with a
    node of no cell are dropped and counted (a wall face that gmsh did not extrude)."""
    used = np.zeros(len(xyz), bool)
    for c in cells.values():
        used[c.ravel()] = True
    new = np.full(len(xyz), -1, np.int64)
    new[used] = np.arange(int(used.sum()))
    cells = {k: new[c] for k, c in cells.items()}
    orphans = 0
    out = []
    for t, faces in markers:
        kept = []
        for k, c in faces:
            c = new[c]
            ok = (c >= 0).all(axis=1)
            orphans += int((~ok).sum())
            kept.append((k, c[ok]))
        out.append((t, kept))
    return cells, out, xyz[used], orphans


def mirror(cells, markers, xyz, axis, drop, tol):
    """Mirrors the mesh about the plane x[axis] = 0 (the nodes on it are shared) and drops the marker `drop`."""
    n = len(xyz)
    off = np.abs(xyz[:, axis]) > tol
    m = np.arange(n)
    m[off] = n + np.arange(int(off.sum()))
    ext = xyz[off].copy()
    ext[:, axis] = -ext[:, axis]
    out_cells = {k: np.vstack([c, m[c][:, FLIP[k]]]) for k, c in cells.items()}
    out_marks = [(t, [(k, np.vstack([c, m[c][:, FLIP[k]]])) for k, c in faces]) for t, faces in markers if t != drop]
    return out_cells, out_marks, np.vstack([xyz, ext])


def write(path, cells, markers, xyz):
    """Writes the SU2 file. `cells`: {type: (n, k) gmsh-ordered positive cells}; `markers`: [(tag, [(type, faces)])]."""
    with open(path, "w") as f:
        f.write("NDIME= 3\n")
        total = sum(len(c) for c in cells.values())
        f.write("NELEM= %d\n" % total)
        k0 = 0
        for kind in (TET, PRISM, PYRAMID):
            if kind not in cells or not len(cells[kind]):
                continue
            c = cells[kind][:, TO_SU2[kind]]
            n = len(c)
            data = np.column_stack([np.full(n, kind), c, np.arange(k0, k0 + n)])
            np.savetxt(f, data, fmt="%d")
            k0 += n
        f.write("NPOIN= %d\n" % len(xyz))
        np.savetxt(f, np.column_stack([xyz, np.arange(len(xyz))]), fmt=["%.17g", "%.17g", "%.17g", "%d"])
        f.write("NMARK= %d\n" % len(markers))
        for tag, faces in markers:
            f.write("MARKER_TAG= %s\n" % tag)
            f.write("MARKER_ELEMS= %d\n" % sum(len(c) for _, c in faces))
            for kind, c in faces:
                if len(c):
                    np.savetxt(f, np.column_stack([np.full(len(c), kind), c]), fmt="%d")
    return total
