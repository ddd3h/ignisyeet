# SPDX-License-Identifier: GPL-3.0-or-later
"""Body-fitted structured quadrilateral grids, written as SU2 ASCII meshes.

A grid is the image of a rectangle in parameter space (xi, eta), both in [0, 1]:

    x = x0 + xi * (x1 - x0)
    y = y_wall(x) + eta * (y_top - y_wall(x))

so the lower boundary can be any function of x (a wedge, a cone, an expansion
corner) while the upper boundary stays a straight line. Refinement acts on the
parameter sequences only: ``refine`` inserts midpoints into xi and eta and the
same map is applied again. Every coarse node is then, to the bit, a fine node,
and the refinement ratio is exactly 2, which is what the constant-ratio GCI
needs. Any clustering must therefore live in xi and eta as a fixed function
(``piecewise_uniform`` is one such function).

Node numbering is ``j * ni + i`` with i along xi and j along eta. Arrays have
shape (nj, ni).
"""
from __future__ import annotations

import pathlib
from typing import Callable, Mapping, Sequence, Union

import numpy as np

Segment = Union[str, tuple[str, int, int]]
"""A part of the grid boundary: ``"xmin" | "xmax" | "ymin" | "ymax"`` for a whole side,
or ``(side, a, b)`` for the edges between the nodes a..b along that side (b - a edges).
A side runs along i for ymin / ymax and along j for xmin / xmax; a and b are indices on it."""

SIDES = ("xmin", "xmax", "ymin", "ymax")


def piecewise_uniform(knots: Sequence[float], cells: Sequence[int]) -> np.ndarray:
    """Parameter values uniform inside each interval between consecutive knots.

    ``knots`` are increasing parameter values from 0 to 1 and ``cells[k]`` is the
    number of cells between knot k and k+1. Every knot is a node, so a kink of
    the lower boundary can sit exactly on one.
    """
    if len(cells) != len(knots) - 1:
        raise ValueError("one cell count per interval between knots is needed")
    parts = [np.array([knots[0]])]
    for a, b, n in zip(knots, knots[1:], cells):
        if n < 1 or not b > a:
            raise ValueError("knots must increase and every interval needs at least one cell")
        parts.append(np.linspace(a, b, n + 1)[1:])
    return np.concatenate(parts)


def refine(xi: Sequence[float], eta: Sequence[float], factor: int = 2) -> tuple[np.ndarray, np.ndarray]:
    """Insert ``factor - 1`` evenly spaced points between every consecutive pair.

    The input points are kept exactly, so the refined grid contains the coarse one.
    """
    if factor < 2 or int(factor) != factor:
        raise ValueError("factor must be an integer of at least 2")

    def one(t: Sequence[float]) -> np.ndarray:
        t = np.asarray(t, dtype=float)
        out = np.empty((len(t) - 1) * factor + 1)
        out[::factor] = t
        for k in range(1, factor):
            out[k::factor][: len(t) - 1] = t[:-1] + (t[1:] - t[:-1]) * (k / factor)
        return out

    return one(xi), one(eta)


def body_fitted_grid(
    xi: Sequence[float],
    eta: Sequence[float],
    x_range: tuple[float, float],
    y_wall: Callable[[np.ndarray], np.ndarray],
    y_top: float,
) -> tuple[np.ndarray, np.ndarray]:
    """Node coordinates (X, Y) of shape (len(eta), len(xi)) from the map above."""
    xi = np.asarray(xi, dtype=float)
    eta = np.asarray(eta, dtype=float)
    x0, x1 = x_range
    x = x0 + xi * (x1 - x0)
    yw = np.asarray(y_wall(x), dtype=float)
    X = np.broadcast_to(x, (len(eta), len(xi))).copy()
    Y = yw[None, :] + eta[:, None] * (y_top - yw)[None, :]
    return X, Y


def _side_nodes(side: str, ni: int, nj: int) -> list[int]:
    """Node indices along a side, in the counter-clockwise direction around the domain."""
    if side == "ymin":
        return [i for i in range(ni)]
    if side == "xmax":
        return [j * ni + (ni - 1) for j in range(nj)]
    if side == "ymax":
        return [(nj - 1) * ni + i for i in range(ni - 1, -1, -1)]
    if side == "xmin":
        return [j * ni for j in range(nj - 1, -1, -1)]
    raise ValueError(f"unknown side {side!r}; use one of {SIDES}")


def _segment_edges(seg: Segment, ni: int, nj: int) -> list[tuple[int, int]]:
    if isinstance(seg, str):
        side, a, b = seg, None, None
    else:
        side, a, b = seg
    nodes = _side_nodes(side, ni, nj)
    n = len(nodes)
    if a is None:
        a, b = 0, n - 1
    # a and b are indices along the increasing-index direction of the side
    # (i for ymin / ymax, j for xmin / xmax); the node list is counter-clockwise,
    # which reverses that direction on ymax and xmin.
    if side in ("ymax", "xmin"):
        a, b = n - 1 - b, n - 1 - a
    if not 0 <= a < b <= n - 1:
        raise ValueError(f"segment {seg!r} is outside the {n} nodes of side {side}")
    return [(nodes[k], nodes[k + 1]) for k in range(a, b)]


def structured_quad_su2(
    path: str | pathlib.Path,
    x: np.ndarray,
    y: np.ndarray,
    markers: Mapping[str, Sequence[Segment]],
) -> int:
    """Write a 2-D SU2 mesh of the structured grid (x, y) and return its node count.

    ``x`` and ``y`` are the node arrays of shape (nj, ni) that ``body_fitted_grid``
    returns. ``markers`` maps a marker name to boundary segments (see ``Segment``);
    together they must cover every boundary edge exactly once. Raises ValueError
    for a cell that is degenerate or not counter-clockwise, and for a boundary that
    is not covered exactly once.
    """
    x = np.asarray(x, dtype=float)
    y = np.asarray(y, dtype=float)
    if x.shape != y.shape or x.ndim != 2:
        raise ValueError("x and y must be 2-D arrays of the same shape")
    nj, ni = x.shape
    if ni < 2 or nj < 2:
        raise ValueError("a grid needs at least 2 x 2 nodes")

    # quad (i, j): (i, j) -> (i+1, j) -> (i+1, j+1) -> (i, j+1), counter-clockwise for x, y increasing
    x00, x10, x11, x01 = x[:-1, :-1], x[:-1, 1:], x[1:, 1:], x[1:, :-1]
    y00, y10, y11, y01 = y[:-1, :-1], y[:-1, 1:], y[1:, 1:], y[1:, :-1]
    area2 = ((x00 * y10 - x10 * y00) + (x10 * y11 - x11 * y10)
             + (x11 * y01 - x01 * y11) + (x01 * y00 - x00 * y01))
    # each corner triangle must also be counter-clockwise, or the quad is not convex / is folded
    def tri(ax, ay, bx, by, cx, cy):
        return (bx - ax) * (cy - ay) - (by - ay) * (cx - ax)
    corners = np.minimum.reduce([
        tri(x00, y00, x10, y10, x01, y01), tri(x10, y10, x11, y11, x00, y00),
        tri(x11, y11, x01, y01, x10, y10), tri(x01, y01, x00, y00, x11, y11)])
    bad = (area2 <= 0.0) | (corners <= 0.0)
    if bad.any():
        j, i = map(int, np.argwhere(bad)[0])
        raise ValueError(
            f"{int(bad.sum())} cell(s) are degenerate or not counter-clockwise, the first at "
            f"(i, j) = ({i}, {j}); SU2 needs counter-clockwise quadrilaterals")

    edges: dict[str, list[tuple[int, int]]] = {}
    seen: dict[tuple[int, int], str] = {}
    for name, segs in markers.items():
        edges[name] = []
        for seg in segs:
            for e in _segment_edges(seg, ni, nj):
                key = (min(e), max(e))
                if key in seen:
                    raise ValueError(
                        f"boundary edge {e} belongs to more than one marker ({seen[key]!r} and {name!r})")
                seen[key] = name
                edges[name].append(e)
    boundary = set()
    for side in SIDES:
        nodes = _side_nodes(side, ni, nj)
        boundary.update((min(a, b), max(a, b)) for a, b in zip(nodes, nodes[1:]))
    missing = boundary - set(seen)
    if missing:
        raise ValueError(f"{len(missing)} boundary edge(s) are not covered by any marker, e.g. {sorted(missing)[0]}")

    lines = ["NDIME= 2", f"NELEM= {(ni - 1) * (nj - 1)}"]
    k = 0
    for j in range(nj - 1):
        for i in range(ni - 1):
            n0 = j * ni + i
            lines.append(f"9 {n0} {n0 + 1} {n0 + ni + 1} {n0 + ni} {k}")
            k += 1
    lines.append(f"NPOIN= {ni * nj}")
    for n in range(ni * nj):
        j, i = divmod(n, ni)
        lines.append(f"{float(x[j, i])!r} {float(y[j, i])!r} {n}")
    lines.append(f"NMARK= {len(edges)}")
    for name, es in edges.items():
        lines.append(f"MARKER_TAG= {name}")
        lines.append(f"MARKER_ELEMS= {len(es)}")
        lines.extend(f"3 {a} {b}" for a, b in es)
    pathlib.Path(path).write_text("\n".join(lines) + "\n")
    return ni * nj
