# SPDX-License-Identifier: GPL-3.0-or-later
"""The parts of the layer 0 driver that need no SU2 run."""
import numpy as np
import pytest

from nasa_vv.cases import layer0 as L
from nasa_vv.mesh import structured_quad_su2
from nasa_vv.su2cfg import GOLDEN_EULER, product_numerics


@pytest.mark.parametrize("name", list(L.CASES))
def test_every_case_keeps_the_golden_numerics_and_changes_one_line(name):
    cfg = L.config_text(L.CASES[name])
    assert product_numerics(cfg) == product_numerics(GOLDEN_EULER.read_text())
    assert "RESTART_ASCII" in cfg
    assert ("AXISYMMETRIC= YES" in cfg) == name.startswith("cone")
    assert ("MARKER_SYM" in cfg) == (name == "cone_sym_axis")
    assert "MARKER_ANALYZE= ( inlet, outlet )" in cfg


@pytest.mark.parametrize("name", list(L.CASES))
@pytest.mark.parametrize("level", [0, 1])
def test_every_case_grid_is_valid_and_has_a_node_at_the_corner(name, level, tmp_path):
    case = L.CASES[name]
    X, Y, k = L.grid(case, level)
    assert X.shape == (48 * 2 ** level + 1, 64 * 2 ** level + 1)
    assert abs(X[0, k]) < 1e-12 and abs(Y[0, k]) < 1e-12     # the corner or tip is a node
    assert structured_quad_su2(tmp_path / "m.su2", X, Y, L.mesh_markers(case, X.shape[1], k)) == X.size


def test_the_two_cone_variants_differ_only_in_their_markers():
    a, b = L.CASES["cone"], L.CASES["cone_sym_axis"]
    Xa, Ya, _ = L.grid(a, 0)
    Xb, Yb, _ = L.grid(b, 0)
    assert np.array_equal(Xa, Xb) and np.array_equal(Ya, Yb)
    assert a.lower_markers == ("wall",) and b.lower_markers == ("axis", "cone")


def _solution(n=11):
    x = np.linspace(0.0, 1.0, n)
    X = np.broadcast_to(x, (4, n)).copy()
    Y = np.broadcast_to(np.linspace(0, 1, 4)[:, None], (4, n)).copy()
    one = np.ones_like(X)
    rho_inf = L.P_INF / (287.05287 * L.T_INF)
    return L.Solution(X, Y, rho_inf * one, 800 * one, 0 * one, L.P_INF * one, 2.5 * one, 2.5, 1.4, rho_inf)


def test_uniform_flow_has_unit_entropy_and_zero_spread():
    sol = _solution()
    assert np.allclose(sol.entropy, 1.0, atol=1e-12)
    mean, spread = L._stat(sol, sol.p, 0.2, 0.8)
    assert mean == L.P_INF and spread == 0.0


def test_the_spread_is_relative_and_the_window_text_names_its_nodes():
    sol = _solution()
    sol.p[0] = L.P_INF * (1.0 + np.linspace(0.0, 0.1, sol.p.shape[1]))
    mean, spread = L._stat(sol, sol.p, 0.5, 1.0)
    assert spread == pytest.approx((1.1 - 1.05) / np.mean(1.0 + np.linspace(0.05, 0.1, 6)), rel=1e-12)
    assert "x = 0.5000..1.0000; 6 nodes" in L._window(sol, 0.5, 1.0)


def test_a_wave_on_the_top_boundary_stops_the_collection(monkeypatch, tmp_path):
    sol = _solution()
    sol.p[-1, 5] *= 1.01
    monkeypatch.setattr(L, "read_history", lambda path: {})
    monkeypatch.setattr(L, "read_solution", lambda case, level, work: sol)
    with pytest.raises(RuntimeError, match="top boundary"):
        L.collect_level(L.CASES["pm15"], 0, tmp_path)


def test_a_gci_note_names_the_cause_and_reports_an_extrapolation_beyond_the_exact_value():
    note = L.gci_note("cone", "wall_mach", [2.10, 2.05, 1.95], 2.12, None)
    assert "wall entropy layer" in note
    assert "lies beyond the exact value" in note          # f1 = 2.10 < 2.12 but the extrapolation is above it
    assert "error ratios" in note and "difference ratio" in note
