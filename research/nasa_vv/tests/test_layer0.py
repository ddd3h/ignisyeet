# SPDX-License-Identifier: GPL-3.0-or-later
"""The parts of the layer 0 driver that need no SU2 run."""
import hashlib

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


DEFAULT_CONFIG_SHA256 = {
    "wedge": "5fdb24a70f8b167192b8b43ce5dea2cec5299498d81b00503954c3e41aa4f959",
    "cone": "8e1ce1da7ec8fe3c8ef1d0d9374912b942d859f84140478602b4a54363fd90f8",
    "cone_sym_axis": "f87f333d777b7311c0924ccdf52a9659205fab038504c69487944dcc952a96e6",
    "pm15": "5fdb24a70f8b167192b8b43ce5dea2cec5299498d81b00503954c3e41aa4f959",
}
"""sha256 of config_text(case) as Task 10 ran it (the case.cfg of its runs); wedge and pm15 differ only in the mesh."""


@pytest.mark.parametrize("name", list(L.CASES))
def test_the_default_configuration_is_byte_for_byte_the_one_task_10_ran(name):
    assert hashlib.sha256(L.config_text(L.CASES[name]).encode()).hexdigest() == DEFAULT_CONFIG_SHA256[name]


@pytest.mark.parametrize("name", list(L.CASES))
def test_analyze_changes_only_the_marker_analyze_line(name):
    default = L.config_text(L.CASES[name])
    markers = ["inlet", "outlet", "top", "wall"]
    other = L.config_text(L.CASES[name], analyze=markers)
    changed = [(a, b) for a, b in zip(default.splitlines(), other.splitlines()) if a != b]
    assert len(default.splitlines()) == len(other.splitlines())
    assert changed == [("MARKER_ANALYZE= ( inlet, outlet )", "MARKER_ANALYZE= ( inlet, outlet, top, wall )")]


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


def test_the_bounds_column_holds_fails_and_stays_blank():
    # a GCI of 0.06 % against a finest-grid error of 0.19 % does not bound the error
    true_pct, bounds = L.bounds_columns("0.0594", 0.0965, 0.0967306)
    assert float(true_pct) == pytest.approx(100 * 0.0002306 / 0.0967306, rel=1e-6)
    assert bounds == "False"
    # a GCI of 0.011 % against an error of 0.006 % does
    assert L.bounds_columns("0.0114", 2.467646, 2.4675)[1] == "True"
    # equal values count as bounding
    assert L.bounds_columns("50", 1.5, 1.0)[1] == "True"
    # no GCI, or no exact value (or a zero one): both columns blank
    assert L.bounds_columns("", 1.0, 2.0) == ("", "")
    assert L.bounds_columns("1.0", 1.0, None) == ("", "")
    assert L.bounds_columns("1.0", 1.0, 0.0) == ("", "")


def test_the_window_label_rests_on_the_range_of_the_orders():
    assert L.window_label([0.84, 1.04, 0.91, 0.98])[0] == "stable"
    assert L.window_label([2.54, 1.33, 0.65, 1.01])[0] == "not stable"
    assert L.window_label([1.0, None, 1.0, 1.0])[0] == "not stable"
    assert L.window_label([0.84, 1.04, 0.91, 0.98])[1] == "0.84 to 1.04"


def test_the_committed_gci_file_marks_the_cone_cp_as_not_bounding():
    import csv
    lines = [ln for ln in L.RESULTS.joinpath("layer0_gci.csv").read_text().splitlines() if not ln.startswith("#")]
    rows = {(r["condition"].split()[0], r["quantity"]): r for r in csv.DictReader(lines)}
    cone = rows[("cone", "wall_cp")]
    assert cone["gci_bounds_error"] == "False"
    assert float(cone["gci_fine_pct"]) < float(cone["true_error_pct"])
    assert "must not be quoted as the uncertainty" in cone["note"]
    assert rows[("wedge", "wall_p_ratio")]["gci_bounds_error"] == "True"
    assert rows[("pm15", "wall_mach")]["gci_bounds_error"] == ""      # no GCI was computed
    header = "\n".join(ln for ln in L.RESULTS.joinpath("layer0_gci.csv").read_text().splitlines() if ln.startswith("#"))
    held = sum(r["gci_bounds_error"] == "True" for r in rows.values())
    failed = sum(r["gci_bounds_error"] == "False" for r in rows.values())
    assert f"it held for {held} and failed for {failed}" in header
