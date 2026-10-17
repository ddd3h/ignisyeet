# SPDX-License-Identifier: GPL-3.0-or-later
"""The parts of the laminar plate driver that need no SU2 run."""
import csv
import hashlib
import math

import numpy as np
import pytest

from nasa_vv import exact
from nasa_vv.cases import layer0_plate as P
from nasa_vv.mesh import structured_quad_su2
from nasa_vv.su2cfg import GOLDEN_RANS, rans_gas_model, rans_numerics


@pytest.mark.parametrize("level", [0, 1])
def test_each_grid_contains_the_coarser_one_to_the_bit(level):
    Xc, Yc = P.grid(level)
    Xf, Yf = P.grid(level + 1)
    assert np.array_equal(Xf[::2, ::2], Xc)
    assert np.array_equal(Yf[::2, ::2], Yc)


@pytest.mark.parametrize("level", [0, 1, 2])
def test_the_leading_edge_and_every_station_are_nodes_of_every_level(level):
    X, Y = P.grid(level)
    assert abs(X[0, P.le_index(level)]) < 1e-12
    for x in (P.STATION,) + P.ALT_STATIONS:
        assert abs(X[0, P.station_index(level, x)] - x) < 1e-12
    assert np.all(Y[0] == 0.0)
    assert X.shape == (P.BASE_CELLS_Y * 2 ** level + 1, P.MAIN.cells_x * 2 ** level + 1)
    assert P.level_name(level) == f"{X.shape[1]}x{X.shape[0]}"


def test_a_station_off_the_base_grid_is_refused():
    with pytest.raises(ValueError):
        P.station_index(0, 0.51)


def test_the_wall_clustering_is_fixed_in_parameter_space_and_puts_nodes_in_the_boundary_layer():
    s = np.linspace(0.0, 1.0, 49)
    y = P.stretch(s)
    assert y[0] == 0.0 and y[-1] == pytest.approx(1.0) and np.all(np.diff(y) > 0.0)
    # refining s and clustering afterwards: each level has the same function, first cell shrinks by about 4
    Y0 = P.grid(0)[1][:, 0]
    Y2 = P.grid(2)[1][:, 0]
    assert 3.9 < Y0[1] / Y2[1] < 4.5
    # nodes inside the 99 % layer at x = 0.5 (eta <= 4.91 -> y <= 4.91 sqrt(x / Re_unit)) on the coarsest grid
    delta = P.ETA99 * math.sqrt(P.STATION / P.RE_UNIT)
    assert int(np.sum(Y0 <= delta)) >= 10


@pytest.mark.parametrize("level", [0, 1])
def test_the_mesh_is_valid_and_every_boundary_edge_has_a_marker(level, tmp_path):
    X, Y = P.grid(level)
    n = structured_quad_su2(tmp_path / "m.su2", X, Y, P.mesh_markers(X.shape[1], P.le_index(level)))
    assert n == X.size
    text = (tmp_path / "m.su2").read_text()
    for name in ("upstream", "plate", "inlet", "outlet", "top"):
        assert f"MARKER_TAG= {name}" in text


def test_the_configuration_has_the_rans_numerics_and_no_turbulence_model():
    cfg = P.config_text()
    golden = GOLDEN_RANS.read_text()
    assert rans_numerics(cfg) == rans_numerics(golden)
    assert "SOLVER= NAVIER_STOKES" in cfg and "SOLVER= RANS" not in cfg
    assert "KIND_TURB_MODEL" not in cfg and "NU_TILDE" not in cfg and "SA_OPTIONS" not in cfg
    assert "MARKER_HEATFLUX= ( plate, 0.0 )" in cfg and "MARKER_SYM= ( upstream )" in cfg
    assert "PRANDTL" not in cfg
    assert "RESTART_ASCII" in cfg and "SURFACE_PARAVIEW_ASCII" in cfg
    assert "MACH_NUMBER= 0.200000" in cfg and "ITER= 40000" in cfg


def test_the_free_stream_has_the_stated_reynolds_number_from_the_goldens_viscosity_law():
    fs = P.freestream()
    gas = rans_gas_model(GOLDEN_RANS.read_text())
    assert fs.u == pytest.approx(P.MACH * math.sqrt(gas["gamma"] * gas["gas_constant"] * P.T_INF))
    assert fs.rho * fs.u / fs.mu == pytest.approx(P.RE_UNIT, rel=1e-7)
    assert fs.mu == pytest.approx(P.sutherland(P.T_INF, gas))
    assert fs.p == pytest.approx(fs.rho * gas["gas_constant"] * fs.t)
    assert f"FREESTREAM_PRESSURE= {fs.p:.4f}" in P.config_text()


def test_the_compressibility_estimate_is_small_and_vanishes_with_the_mach_number(monkeypatch):
    gas = rans_gas_model(GOLDEN_RANS.read_text())
    est = P.eckert_estimate(P.freestream(), gas)
    assert 0.9990 < est["cf_factor"] < 0.9999         # a decrease of about 0.06 %
    # T_aw / T_inf = 1 + sqrt(Pr) (gamma - 1) M^2 / 2 = 1 + sqrt(0.72) * 0.2 * 0.04
    assert est["t_aw_over_t"] == pytest.approx(1.0 + math.sqrt(0.72) * 0.008, rel=1e-12)
    assert est["delta_star_factor"] > 1.0 and 0.0 < est["profile_shift"] < 0.01
    monkeypatch.setattr(P, "MACH", 1e-3)
    zero = P.eckert_estimate(P.freestream(), gas)
    assert zero["cf_factor"] == pytest.approx(1.0, abs=1e-6)
    assert zero["profile_shift"] < 1e-5


def test_profile_stats_of_the_exact_profile_are_zero_deviation_and_the_exact_integrals():
    ref = P.blasius_reference()
    eta = np.array(ref["eta"][::40])        # a coarse sample of the exact profile, up to eta = 10
    st = P.profile_stats(eta, np.array(ref["fp"][::40]))
    assert st["max_dev"] < 1e-9 and st["rms_dev"] < 1e-9
    assert st["delta_star"] == pytest.approx(ref["delta_star"], abs=2e-4)   # trapezoid rule on a 0.1 spacing
    assert st["theta"] == pytest.approx(ref["theta"], abs=2e-4)
    # the truncated integrals are those of the infinite profile up to the tail beyond eta = 8
    ds, th = exact.blasius_thicknesses()
    assert ref["delta_star"] == pytest.approx(ds, abs=2e-5) and ref["theta"] == pytest.approx(th, abs=2e-5)


def test_profile_stats_see_a_shifted_profile():
    ref = P.blasius_reference()
    eta = np.array(ref["eta"][::40])
    fp = np.array(ref["fp"][::40])
    shifted = np.interp(eta * 0.98, eta, fp)          # u(eta) = f'(0.98 eta): the layer is 2 % thicker
    st = P.profile_stats(eta, shifted)
    assert st["max_dev"] > 0.003
    assert st["delta_star"] == pytest.approx(ref["delta_star"] / 0.98, rel=5e-3)


def test_the_gci_stage_labels_follow_the_range_of_the_orders():
    assert P.station_label([0.84, 1.04, 0.91, 0.98])[0] == "stable"
    assert P.station_label([2.54, 1.33, 0.65, 1.01])[0] == "not stable"
    assert P.station_label([1.0, None, 1.0, 1.0])[0] == "not stable"


# ---- appending to layer0_gci.csv

COLS = ["study", "condition", "quantity", "triplet", "h_basis", "n1", "n2", "n3", "r21", "r32", "f1", "f2", "f3",
        "order", "extrapolated", "gci_fine_pct", "gci_coarse_pct", "asymptotic_ratio", "monotone", "note",
        "true_error_pct", "gci_bounds_error"]

BASE = (
    "# header line of the other cases\n"
    "# Of 2 rows with a computed GCI it held for 1 and failed for 1 (x)\n"
    + ",".join(COLS) + "\n"
    'layer0,wedge M=2.5,a,t,h,3,2,1,2,2,1,2,3,1,1,1,1,1,True,"note, with comma and ""quotes""",0.1,True\n'
    "layer0,cone M=2.35,b,t,h,3,2,1,2,2,1,2,3,1,1,1,1,1,True,note,0.1,False\n"
)


def plate_rows(flags):
    rows = []
    for q, flag in zip(("cf_x0.5", "delta_star_eta"), flags):
        r = {c: "" for c in COLS}
        r.update(study="layer0", condition=P.CONDITION, quantity=q, f1="1", gci_fine_pct="0.5", note="a plate note",
                 true_error_pct="0.4", gci_bounds_error=flag)
        rows.append(r)
    return rows


def test_merging_keeps_every_other_byte_and_is_idempotent():
    rows = plate_rows(["True", "False"])
    once = P.merge_gci(BASE, rows)
    assert P.strip_plate(once) == BASE                       # the other cases' header and rows are byte for byte intact
    assert once.startswith("# header line of the other cases\n# Of 2 rows")
    assert P.merge_gci(once, rows) == once                   # a rerun neither duplicates nor changes anything
    assert once.count("layer0,plate M=") == 2
    twice_other = P.merge_gci(once, plate_rows(["True", "True"]))
    assert P.strip_plate(twice_other) == BASE and twice_other.count("layer0,plate M=") == 2


def test_the_merged_header_states_the_whole_file_counts_and_precedes_the_column_header():
    rows = plate_rows(["True", "False"])
    merged = P.merge_gci(BASE, rows)
    lines = merged.split("\n")
    col = next(k for k, ln in enumerate(lines) if ln.startswith("study,condition,"))
    mine = [ln for ln in lines if ln.startswith(P.PLATE_HEADER)]
    assert mine and all(k < col for k, ln in enumerate(lines) if ln.startswith(P.PLATE_HEADER))
    header = "\n".join(ln for ln in lines if ln.startswith("#"))
    # the file-wide count of the layer 0 test: wedge True, cone False, plate True and False
    assert "it held for 2 and failed for 2" in header
    assert "IgnisYeet has no laminar mode" in header
    parsed = list(csv.DictReader(ln for ln in lines if ln and not ln.startswith("#")))
    assert [r["quantity"] for r in parsed] == ["a", "b", "cf_x0.5", "delta_star_eta"]


def test_merging_refuses_a_file_without_a_column_header():
    with pytest.raises(ValueError):
        P.merge_gci("# only a header\n", plate_rows(["True", "True"]))


def test_the_committed_gci_file_keeps_the_other_rows_and_carries_the_plate_rows_once():
    text = (P.RESULTS / "layer0_gci.csv").read_text()
    other = P.strip_plate(text)
    assert hashlib.sha256(other.encode()).hexdigest()[:16] == TASK10_SHA
    assert P.merge_gci(text, [r for r in P.parse_gci_rows(text) if r["condition"] == P.CONDITION]) == text
    plate = [r for r in P.parse_gci_rows(text) if r["condition"] == P.CONDITION]
    assert [r["quantity"] for r in plate] == ["cf_x0.5", "delta_star_eta", "theta_eta"]
    for r in plate:
        assert r["gci_bounds_error"] in ("True", "False", "")
        assert "IgnisYeet has no laminar mode" in r["note"]


TASK10_SHA = "310776aa0ac0c8f8"   # sha256 of layer0_gci.csv as Task 10 committed it (bcabad4), the first 16 digits
