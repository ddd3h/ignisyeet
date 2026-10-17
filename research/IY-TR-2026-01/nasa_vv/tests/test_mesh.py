# SPDX-License-Identifier: GPL-3.0-or-later
import math

import numpy as np
import pytest

from nasa_vv.mesh import (body_fitted_grid, piecewise_uniform, refine,
                          structured_quad_su2)

TAN15 = math.tan(math.radians(15.0))


def _ramp(x):
    """Lower boundary of a 15 degree wedge: flat for x < 0, a ramp for x >= 0."""
    return np.where(x < 0.0, 0.0, x * TAN15)


def _wedge_grid(n_up=4, n_dn=8, n_eta=6):
    xi = piecewise_uniform([0.0, 1.0 / 3.0, 1.0], [n_up, n_dn])
    eta = np.linspace(0.0, 1.0, n_eta + 1)
    return xi, eta, body_fitted_grid(xi, eta, (-0.5, 1.0), _ramp, 1.0)


def _read(path):
    """Quads, nodes and markers of a .su2 file as plain Python."""
    lines = path.read_text().splitlines()
    out = {"quads": [], "nodes": [], "markers": {}}
    mode, tag = None, None
    for ln in lines:
        if ln.startswith("NELEM="):
            mode = "quads"
        elif ln.startswith("NPOIN="):
            mode = "nodes"
        elif ln.startswith("MARKER_TAG="):
            tag = ln.split("=")[1].strip()
            out["markers"][tag] = []
            mode = "marker"
        elif ln.startswith(("NDIME", "NMARK", "MARKER_ELEMS")):
            continue
        elif mode == "quads":
            out["quads"].append([int(t) for t in ln.split()[1:5]])
        elif mode == "nodes":
            out["nodes"].append([float(t) for t in ln.split()[:2]])
        elif mode == "marker":
            out["markers"][tag].append([int(t) for t in ln.split()[1:3]])
    return out


def test_a_small_grid_writes_the_expected_counts(tmp_path):
    xi = eta = np.array([0.0, 0.5, 1.0])
    X, Y = body_fitted_grid(xi, eta, (0.0, 1.0), lambda x: 0.0 * x, 1.0)
    p = tmp_path / "g.su2"
    n = structured_quad_su2(p, X, Y,
                            {"wall": ["ymin"], "far": ["ymax", "xmin", "xmax"]})
    text = p.read_text()
    assert n == 9
    assert "NDIME= 2" in text
    assert "NPOIN= 9" in text
    assert "NELEM= 4" in text
    assert "NMARK= 2" in text
    assert "MARKER_TAG= wall" in text
    assert "MARKER_ELEMS= 2" in text      # ymin: two edges
    assert "MARKER_ELEMS= 6" in text      # the other three sides: 2 + 2 + 2


def test_every_quadrilateral_is_counter_clockwise_on_a_non_rectangular_grid(tmp_path):
    _, _, (X, Y) = _wedge_grid()
    p = tmp_path / "g.su2"
    structured_quad_su2(p, X, Y, {"w": ["ymin"], "f": ["xmin", "xmax", "ymax"]})
    d = _read(p)
    assert len(d["quads"]) == X.shape[0] * X.shape[1] - X.shape[0] - X.shape[1] + 1
    for quad in d["quads"]:
        pts = [d["nodes"][k] for k in quad]
        area2 = sum(pts[k][0] * pts[(k + 1) % 4][1] - pts[(k + 1) % 4][0] * pts[k][1]
                    for k in range(4))
        assert area2 > 0.0


def test_boundary_edges_run_counter_clockwise_around_the_domain(tmp_path):
    """The interior lies to the left of every boundary edge."""
    _, _, (X, Y) = _wedge_grid()
    p = tmp_path / "g.su2"
    structured_quad_su2(p, X, Y, {"w": ["ymin"], "f": ["xmin", "xmax", "ymax"]})
    d = _read(p)
    cx = sum(n[0] for n in d["nodes"]) / len(d["nodes"])
    cy = sum(n[1] for n in d["nodes"]) / len(d["nodes"])
    for edges in d["markers"].values():
        for a, b in edges:
            (xa, ya), (xb, yb) = d["nodes"][a], d["nodes"][b]
            assert (xb - xa) * (cy - ya) - (yb - ya) * (cx - xa) > 0.0


def test_a_partial_side_gives_the_marker_its_own_edges(tmp_path):
    """The ymin side splits at the node index 4: x < 0 in one marker, x >= 0 in the other."""
    xi, _, (X, Y) = _wedge_grid(n_up=4, n_dn=8)
    p = tmp_path / "g.su2"
    structured_quad_su2(p, X, Y, {"axis": [("ymin", 0, 4)], "ramp": [("ymin", 4, 12)],
                                  "far": ["xmin", "xmax", "ymax"]})
    d = _read(p)
    assert len(d["markers"]["axis"]) == 4
    assert len(d["markers"]["ramp"]) == 8
    assert all(d["nodes"][a][0] <= 0.0 + 1e-15 and d["nodes"][b][0] <= 1e-15
               for a, b in d["markers"]["axis"])
    assert all(d["nodes"][a][0] >= -1e-15 for a, b in d["markers"]["ramp"])


def test_an_unmarked_boundary_edge_is_refused(tmp_path):
    _, _, (X, Y) = _wedge_grid()
    with pytest.raises(ValueError, match="not covered"):
        structured_quad_su2(tmp_path / "g.su2", X, Y, {"w": ["ymin"], "f": ["xmin", "xmax"]})


def test_an_edge_in_two_markers_is_refused(tmp_path):
    _, _, (X, Y) = _wedge_grid()
    with pytest.raises(ValueError, match="more than one"):
        structured_quad_su2(tmp_path / "g.su2", X, Y,
                            {"w": ["ymin"], "f": ["xmin", "xmax", "ymax", "ymin"]})


def test_an_inverted_cell_is_refused(tmp_path):
    xi = eta = np.linspace(0.0, 1.0, 4)
    X, Y = body_fitted_grid(xi, eta, (0.0, 1.0), lambda x: 0.0 * x, 1.0)
    X[1, 1], Y[1, 1] = X[2, 2] + 0.1, Y[2, 2] + 0.1   # fold node (1, 1) over node (2, 2)
    with pytest.raises(ValueError, match="not counter-clockwise|degenerate"):
        structured_quad_su2(tmp_path / "g.su2", X, Y,
                            {"w": ["ymin"], "f": ["xmin", "xmax", "ymax"]})


def test_a_mirrored_grid_is_refused(tmp_path):
    """Arrays with x decreasing along i are clockwise as a whole."""
    xi = eta = np.linspace(0.0, 1.0, 4)
    X, Y = body_fitted_grid(xi, eta, (0.0, 1.0), lambda x: 0.0 * x, 1.0)
    with pytest.raises(ValueError, match="not counter-clockwise|degenerate"):
        structured_quad_su2(tmp_path / "g.su2", X[:, ::-1], Y[:, ::-1],
                            {"w": ["ymin"], "f": ["xmin", "xmax", "ymax"]})


def test_the_wall_follows_the_boundary_function_exactly():
    _, _, (X, Y) = _wedge_grid()
    assert np.array_equal(Y[0], _ramp(X[0]))
    assert np.all(Y[-1] == 1.0)
    assert np.any(np.abs(X[0]) < 1e-14)   # the corner is a node, not inside a cell


def test_refinement_is_exactly_nested():
    """Every coarse point survives in the fine grid, to the bit, so r = 2 exactly."""
    xi, eta, (X, Y) = _wedge_grid()
    fxi, feta = refine(xi, eta, factor=2)
    assert len(fxi) == 2 * len(xi) - 1
    assert len(feta) == 2 * len(eta) - 1
    FX, FY = body_fitted_grid(fxi, feta, (-0.5, 1.0), _ramp, 1.0)
    assert np.array_equal(FX[::2, ::2], X)
    assert np.array_equal(FY[::2, ::2], Y)


def test_refining_twice_gives_a_ratio_of_four():
    xi, eta = np.array([0.0, 1.0]), np.array([0.0, 1.0])
    once = refine(xi, eta)
    twice = refine(*once)
    assert len(twice[0]) == 5 and len(twice[1]) == 5
    assert np.array_equal(twice[0], [0.0, 0.25, 0.5, 0.75, 1.0])


def test_piecewise_uniform_hits_its_knots_and_counts():
    xi = piecewise_uniform([0.0, 0.25, 1.0], [3, 6])
    assert len(xi) == 10
    assert xi[3] == 0.25 and xi[0] == 0.0 and xi[-1] == 1.0
    assert np.all(np.diff(xi) > 0.0)
