# SPDX-License-Identifier: GPL-3.0-or-later
"""The flat plate under Spalart-Allmaras and SST (spec 6.6): the grids, the configurations, and the collector, with synthetic runs.
Paths resolve from this file, not the cwd."""
import csv
import math
import pathlib

import numpy as np
import pytest

from nasa_vv.cases import turbulence_models as tm
from nasa_vv.gci import constant_ratio
from nasa_vv.su2cfg import GOLDEN_RANS, rans_gas_model

DATA = pathlib.Path(__file__).parent / "data"
ROOT = pathlib.Path(__file__).resolve().parents[3]


def active(text):
    return {ln.split("=", 1)[0].strip(): ln.split("=", 1)[1].strip()
            for ln in text.splitlines() if ln.strip() and not ln.startswith("%") and "=" in ln}


# ---------------------------------------------------------------- the grids

def test_the_node_counts_are_those_of_the_tmr_family_and_the_cells_those_of_the_existing_data():
    assert [tm.level_name(lv) for lv in tm.LEVELS] == ["69x49", "137x97", "273x193"]
    assert [tm.cells(lv) for lv in tm.LEVELS] == [3264, 13056, 52224]
    for lv in tm.LEVELS:
        X, Y, k = tm.grid(lv)
        assert X.shape == (tm.BASE_CELLS_Y * 2 ** lv + 1, tm.BASE_CELLS_X * 2 ** lv + 1)
        assert (X.shape[0] - 1) * (X.shape[1] - 1) == tm.cells(lv)
    rows = [r for r in csv.DictReader(l for l in (ROOT / "doc/data/cfd/rans_flatplate.csv").read_text().splitlines()
                                      if not l.startswith("#")) if float(r["mach"]) == 0.2]
    assert sorted(int(r["cells"]) for r in rows) == [tm.cells(lv) for lv in tm.LEVELS]
    assert {r["grid"] for r in rows} == {tm.level_name(lv) for lv in tm.LEVELS}


def test_the_extent_is_that_of_the_tmr_plate():
    X, Y, _ = tm.grid(0)
    assert X[0, 0] == pytest.approx(-1 / 3) and X[0, -1] == pytest.approx(2.0)
    assert Y[0, 0] == 0.0 and Y[-1, 0] == pytest.approx(1.0)


def test_every_coarse_node_is_a_fine_node_to_the_bit():
    for lv in (0, 1):
        Xc, Yc, _ = tm.grid(lv)
        Xf, Yf, _ = tm.grid(lv + 1)
        assert np.array_equal(Xf[::2, ::2], Xc) and np.array_equal(Yf[::2, ::2], Yc)


def test_the_leading_edge_and_the_probe_station_are_nodes_of_every_level():
    for lv in tm.LEVELS:
        X, _, k = tm.grid(lv)
        x = X[0]
        assert abs(x[k]) < 1e-15
        assert np.min(np.abs(x - tm.X_PROBE)) < 1e-12
        assert tm.X_PROBE < x[-1]


def test_the_cells_cluster_towards_the_leading_edge_and_the_wall():
    X, Y, k = tm.grid(0)
    dx = np.diff(X[0])
    assert dx[k] < dx[k + 5] < dx[k + 30]            # plate cells grow away from the leading edge
    assert dx[k - 1] < dx[0]                         # upstream cells shrink towards it
    assert np.all(np.diff(Y[:, 0]) > 0)
    dy = np.diff(Y[:, 0])
    assert dy[0] < dy[10] < dy[-1]


def test_the_first_cell_height_halves_with_each_level():
    h = [np.diff(tm.grid(lv)[1][:, 0])[0] for lv in tm.LEVELS]
    assert 1.8 < h[0] / h[1] < 2.3 and 1.8 < h[1] / h[2] < 2.3
    assert h[0] < 6e-6      # y+ of the order of one on the coarse grid at Re = 5e6 per metre


def test_the_mesh_markers_cover_the_boundary(tmp_path):
    from nasa_vv.mesh import structured_quad_su2
    X, Y, k = tm.grid(0)
    structured_quad_su2(tmp_path / "m.su2", X, Y, tm.mesh_markers(X.shape[1], k))     # raises if a boundary edge is uncovered or doubled


# ---------------------------------------------------------------- the configurations

def test_the_freestream_gives_the_reynolds_number_per_metre_of_the_case():
    g = rans_gas_model(GOLDEN_RANS.read_text())
    p = tm.freestream_pressure(GOLDEN_RANS.read_text())
    rho = p / (g["gas_constant"] * tm.T_INF)
    u = tm.MACH * math.sqrt(g["gamma"] * g["gas_constant"] * tm.T_INF)
    mu = g["mu_ref"] * (tm.T_INF / g["t_ref"]) ** 1.5 * (g["t_ref"] + g["sutherland"]) / (tm.T_INF + g["sutherland"])
    assert rho * u / mu == pytest.approx(5.0e6, rel=1e-9)


def test_the_two_models_differ_in_the_model_keys_only_in_both_stages():
    for stage in (tm.startup_config, lambda m: tm.final_config(m, -9.0)):
        sa, sst = active(stage("SA")), active(stage("SST"))
        assert set(sa) - set(sst) == {"SA_OPTIONS", "FREESTREAM_NU_FACTOR"}
        assert set(sst) - set(sa) == set(tm.SST_KEYS)
        assert {k for k in sa if k in sst and sa[k] != sst[k]} == {"KIND_TURB_MODEL"}


def test_the_sst_choices_are_the_ones_of_su2s_own_tmr_case():
    assert tm.SST == {"SST_OPTIONS": "V1994m", "FREESTREAM_TURBULENCEINTENSITY": "0.00038729", "FREESTREAM_TURB2LAMVISCRATIO": "0.009"}
    # k = 1.5 (Tu U)^2 = 2.25e-7 U^2 and omega = rho k / (mu ratio) = 125 U / L at Re = 5e6 per metre
    tu, ratio = 0.00038729, 0.009
    assert 1.5 * tu ** 2 == pytest.approx(2.25e-7, rel=1e-3)
    assert 1.5 * tu ** 2 * 5.0e6 / ratio == pytest.approx(125.0, rel=1e-3)


def test_the_numerics_of_both_stages_are_the_goldens():
    golden_final, golden_startup = active(GOLDEN_RANS.read_text()), active((DATA / "golden_rans_startup.cfg").read_text())
    for model in tm.MODELS:
        final, startup = active(tm.final_config(model, -9.0)), active(tm.startup_config(model))
        for key in ("CFL_NUMBER", "CFL_ADAPT_PARAM", "SLOPE_LIMITER_FLOW", "CONV_NUM_METHOD_FLOW", "LINEAR_SOLVER_ERROR",
                    "CONV_NUM_METHOD_TURB", "MUSCL_TURB", "GAS_CONSTANT"):
            assert final[key] == golden_final[key]
        assert startup["MUSCL_FLOW"] == "NO" and startup["CFL_REDUCTION_TURB"] == golden_startup["CFL_REDUCTION_TURB"] == "0.2"
        assert startup["ITER"] == golden_startup["ITER"] == "500"


def test_the_final_stop_rule_is_density_only_with_the_given_floor_for_both_models():
    for model in tm.MODELS:
        c = active(tm.final_config(model, -9.5))
        assert c["CONV_FIELD"] == "RMS_DENSITY" and c["CONV_RESIDUAL_MINVAL"] == "-9.500" and c["ITER"] == str(tm.ITER_FINAL)
        assert c["RESTART_SOL"] == "YES"


def test_no_more_than_eight_ranks():
    assert tm.JOBS * tm.RANKS <= 8
    with pytest.raises(ValueError, match="8"):
        tm.run_all(pathlib.Path("/nonexistent"), jobs=3, ranks=4)


# ---------------------------------------------------------------- reading a run (synthetic output)

def vtk(path, xs, cf, yp):
    n = len(xs)
    lines = ["# vtk DataFile Version 3.0", "x", "ASCII", "DATASET UNSTRUCTURED_GRID", f"POINTS {n} double"]
    lines += [f"{x!r} 0.0 0.0" for x in xs]
    lines += [f"POINT_DATA {n}", "SCALARS Density double 1", "LOOKUP_TABLE default"] + ["1.0"] * n
    lines += ["VECTORS Skin_Friction_Coefficient double"] + [f"{c!r} 0.0 0.0" for c in cf]
    lines += ["SCALARS Y_Plus double 1", "LOOKUP_TABLE default"] + [repr(y) for y in yp]
    path.write_text("\n".join(lines) + "\n")


def history(path, cols, rows):
    head = ",".join(f'"{c}"' for c in cols)
    path.write_text(head + "\n" + "\n".join(",".join(repr(v) for v in r) for r in rows) + "\n")


def fake_run(work, model, level, *, cf, cd, drop_rho=8.0, drop_turb=7.0, final_rho=-9.5, iters=700):
    d = tm.case_dir(work, model, level)
    d.mkdir(parents=True)
    turb = list(tm.TURBULENCE_RESIDUALS[model])
    cols = ["Inner_Iter", "rms[Rho]", *turb, "CD"]
    history(d / "history_startup.csv", cols, [[0, 0.0] + [0.0] * len(turb) + [0.1]] + [[i, -1.0 - i * 0.01] + [-2.0] * len(turb) + [0.1] for i in range(1, 11)])
    # the startup peak of rms[Rho] is -1.01 (iteration 0 is excluded), of the turbulence residuals -2
    rows = [[i, -1.01 - drop_rho * (i / iters)] + [-2.0 - drop_turb * (i / iters)] * len(turb) + [cd * (1.0 + 1e-3 * (iters - i) / iters)] for i in range(iters)]
    rows[-1][1] = final_rho
    history(d / "history.csv", cols, rows)
    xs = [0.0, 0.5, tm.X_PROBE, 1.5, 2.0]
    vtk(d / "surface_flow.vtk", xs, [9.0, 3.0, cf, 2.0, 1.0], [5.0, 0.9, 0.8, 0.7, 0.6])
    (d / "final.cfg").write_text("CONV_RESIDUAL_MINVAL= -9.010\n")
    (d / "wall_seconds.txt").write_text("12.5\n")
    return d


def test_read_run_takes_cf_at_the_probe_node_and_the_weakest_residual_drop(tmp_path):
    fake_run(tmp_path, "SST", 1, cf=0.00265, cd=0.0029, drop_rho=8.0, drop_turb=5.0)
    r = tm.read_run(tmp_path, "SST", 1)
    assert r.cf == pytest.approx(0.00265) and r.cd == pytest.approx(0.0029, rel=1e-3)
    assert r.rho_drop == pytest.approx(9.5 - 1.01)         # the startup peak of rms[Rho] is -1.01
    assert set(r.turb_drops) == {"rms[k]", "rms[w]"}
    assert r.residual_drop == pytest.approx(min(r.rho_drop, *r.turb_drops.values()))
    assert r.residual_drop < r.rho_drop
    assert r.yplus_max == pytest.approx(0.9) and r.stop == "residual target" and r.iterations == 700
    assert r.wall_s == 12.5 and r.grid == "137x97" and r.cells == 13056


def test_a_run_that_did_not_reach_the_floor_is_reported_as_stopped_by_the_iteration_limit(tmp_path):
    fake_run(tmp_path, "SA", 0, cf=0.0027, cd=0.003, final_rho=-7.0)
    assert tm.read_run(tmp_path, "SA", 0).stop == "iteration limit"


def test_a_wall_without_a_node_at_the_probe_station_is_refused(tmp_path):
    d = fake_run(tmp_path, "SA", 0, cf=0.0027, cd=0.003)
    vtk(d / "surface_flow.vtk", [0.0, 0.5, 0.96, 1.5, 2.0], [9.0, 3.0, 2.0, 2.0, 1.0], [5.0, 0.9, 0.8, 0.7, 0.6])
    with pytest.raises(ValueError, match="node"):
        tm.read_run(tmp_path, "SA", 0)


def test_a_missing_case_says_to_run_it(tmp_path):
    with pytest.raises(FileNotFoundError, match="--run"):
        tm.read_run(tmp_path, "SA", 0)


# ---------------------------------------------------------------- the table

def runs_from(values):
    """values[model] = [(cf, cd) for levels 0, 1, 2]"""
    out = {}
    for m, vs in values.items():
        out[m] = [tm.Run(m, lv, cf, cd, 1.2, 0.8, 8.0, {c: 7.0 for c in tm.TURBULENCE_RESIDUALS[m]}, 1000, 100, "residual target", 0.001, 10.0)
                  for lv, (cf, cd) in enumerate(vs)]
    return out


SA = [(0.002740, 0.003030), (0.002730, 0.003013), (0.002726, 0.003007)]
SST = [(0.002640, 0.002930), (0.002610, 0.002890), (0.002600, 0.002880)]
REFERENCE = {"69x49": {"cf_x097": 0.00272236, "cd_plate": 0.00299974, "tmr_sa_cf": 0.00270551},
             "137x97": {"cf_x097": 0.00272564, "cd_plate": 0.00298455, "tmr_sa_cf": 0.00270551},
             "273x193": {"cf_x097": 0.00272692, "cd_plate": 0.00297412, "tmr_sa_cf": 0.00270551}}


def rows(values=None, ref=REFERENCE):
    return tm.build_rows(runs_from(values or {"SA": SA, "SST": SST}), ref)


def pick(rs, **kw):
    hit = [r for r in rs if all(r[k] == v for k, v in kw.items())]
    assert len(hit) == 1, (kw, len(hit))
    return hit[0]


def test_the_gci_is_the_constant_ratio_form_with_r_2_per_model_and_quantity():
    g = pick(rows(), kind="gci", model="SA", quantity="cf_x097")
    ref = constant_ratio(SA[2][0], SA[1][0], SA[0][0], 2.0)
    assert float(g["order"]) == pytest.approx(ref.order, rel=1e-6)
    assert float(g["gci_fine_pct"]) == pytest.approx(100 * ref.gci_fine, rel=1e-6)
    assert float(g["extrapolated"]) == pytest.approx(ref.extrapolated, rel=1e-6)
    assert g["grid"] == "273x193/137x97/69x49" and g["cells"] == "52224"


def test_the_model_difference_is_sst_minus_sa_and_is_stated_against_both_bands():
    rs = rows()
    d = pick(rs, kind="model_difference", quantity="cf_x097", grid="273x193")
    assert float(d["value"]) == pytest.approx(SST[2][0] - SA[2][0])
    assert float(d["difference_pct"]) == pytest.approx(100 * (SST[2][0] - SA[2][0]) / SA[2][0], rel=1e-3)
    ga = float(pick(rs, kind="gci", model="SA", quantity="cf_x097")["gci_fine_pct"])
    gs = float(pick(rs, kind="gci", model="SST", quantity="cf_x097")["gci_fine_pct"])
    assert float(d["sa_gci_pct"]) == pytest.approx(ga, rel=1e-4) and float(d["sst_gci_pct"]) == pytest.approx(gs, rel=1e-4)
    assert d["separable"] == str(abs(float(d["difference_pct"])) > ga + gs)


def test_a_difference_inside_the_bands_is_not_separable_and_one_outside_is():
    close = {"SA": SA, "SST": [(cf * 1.0001, cd * 1.0001) for cf, cd in SA[:2]] + [(SA[2][0] * 1.00001, SA[2][1] * 1.00001)]}
    assert pick(rows(close), kind="model_difference", quantity="cf_x097", grid="273x193")["separable"] == "False"
    assert pick(rows(), kind="model_difference", quantity="cf_x097", grid="273x193")["separable"] == "True"


@pytest.mark.parametrize("diff, a, b, expected", [
    (5.0, 1.0, 2.0, True), (-5.0, 1.0, 2.0, True), (2.5, 1.0, 2.0, False),      # between a and a + b, and between b and a + b
    (3.0, 1.0, 2.0, False), (1.5, 2.0, 1.0, False), (None, None, None, None),
    (5.0, None, 2.0, None), (5.0, 1.0, None, None)])
def test_the_verdict_compares_the_difference_with_the_sum_of_both_bands(diff, a, b, expected):
    if diff is None:
        assert tm.separable(5.0, None, None) is None
    else:
        assert tm.separable(diff, a, b) is expected


def test_the_coarsest_level_has_no_band_and_no_verdict():
    d = pick(rows(), kind="model_difference", quantity="cf_x097", grid="69x49")
    assert d["sa_gci_pct"] == "" and d["sst_gci_pct"] == "" and d["separable"] == ""


def test_the_medium_level_uses_the_coarse_gci():
    rs = rows()
    d = pick(rs, kind="model_difference", quantity="cd_plate", grid="137x97")
    ga = float(pick(rs, kind="gci", model="SA", quantity="cd_plate")["gci_coarse_pct"])
    assert float(d["sa_gci_pct"]) == pytest.approx(ga, rel=1e-4)


def test_a_band_flagged_not_reliable_gives_no_verdict_but_the_note_says_what_is_still_certain():
    flagged = {"SA": [(0.002730068, 0.003015), (0.002727161, 0.002990), (0.002727089, 0.002977)],
               "SST": [(0.002628948, 0.002916), (0.002680178, 0.002949), (0.002705489, 0.002964)]}
    rs = rows(flagged)
    g = pick(rs, kind="gci", model="SA", quantity="cf_x097")
    assert "not reliable" in g["note"] and float(g["order"]) > 4
    d = pick(rs, kind="model_difference", quantity="cf_x097", grid="273x193")
    assert d["sa_gci_pct"] == "" and d["separable"] == ""
    assert float(d["sst_gci_pct"]) > abs(float(d["difference_pct"]))
    assert "SA band not used" in d["note"] and "not larger than the SST band alone" in d["note"]
    assert "not reliable" in d["note"]


def test_the_note_gives_the_change_of_the_last_refinement_of_each_model():
    d = pick(rows(), kind="model_difference", quantity="cf_x097", grid="273x193")
    ch_sa = abs(100 * (SA[2][0] - SA[1][0]) / SA[2][0])
    ch_sst = abs(100 * (SST[2][0] - SST[1][0]) / SST[2][0])
    assert f"changed SA by {ch_sa:.3g} % and SST by {ch_sst:.3g} % of this level's value" in d["note"]


def test_the_extrapolated_difference_is_there_when_both_orders_exist():
    d = pick(rows(), kind="model_difference", quantity="cf_x097", grid="extrapolated")
    assert "Richardson" in d["note"] and d["separable"] == ""


def test_non_monotone_runs_give_a_gci_refusal_and_no_verdict_not_a_number():
    wiggly = {"SA": [(0.00272, 0.003), (0.00273, 0.003), (0.002725, 0.003)], "SST": SST}
    rs = rows(wiggly)
    g = pick(rs, kind="gci", model="SA", quantity="cf_x097")
    assert g["gci_fine_pct"] == "" and g["note"] != ""
    d = pick(rs, kind="model_difference", quantity="cf_x097", grid="273x193")
    assert d["sa_gci_pct"] == "" and d["separable"] == ""


def test_an_unconverged_level_is_refused_by_the_family_gate():
    runs = runs_from({"SA": SA, "SST": SST})
    runs["SA"][1].rho_drop = 1.0                   # 1 order is below the 3 the gate asks for
    rs = tm.build_rows(runs, REFERENCE)
    g = pick(rs, kind="gci", model="SA", quantity="cf_x097")
    assert g["gci_fine_pct"] == "" and "refused" in g["note"]


def test_the_cross_checks_name_the_doc_data_sa_value_and_the_tmr_value():
    rs = rows()
    c = pick(rs, kind="cross_check", quantity="cf_x097", grid="273x193", reference="0.00272692")
    assert float(c["difference_pct"]) == pytest.approx(100 * (SA[2][0] / 0.00272692 - 1), rel=1e-3)
    t = pick(rs, kind="cross_check", quantity="cf_x097", grid="273x193", reference="0.00270551")
    assert "TMR" in t["note"] and "not validation" in t["note"]
    assert all(r["model"] == "SA" for r in rs if r["kind"] == "cross_check")        # SST has no published reference here


def test_levels_carry_the_residual_evidence():
    l = pick(rows(), kind="level", model="SST", quantity="cf_x097", grid="137x97")
    assert float(l["residual_drop_orders"]) == pytest.approx(7.0) and l["iterations"] == "1000"
    assert float(l["value"]) == pytest.approx(SST[1][0])


def test_the_reference_is_read_from_the_committed_sa_data():
    ref = tm.read_reference()
    assert set(ref) == {"69x49", "137x97", "273x193"}
    assert ref["273x193"]["cf_x097"] == pytest.approx(0.00272692) and ref["273x193"]["tmr_sa_cf"] == pytest.approx(0.00270551)


def test_write_is_byte_identical_on_a_rerun_and_does_not_depend_on_the_cwd(tmp_path, monkeypatch):
    work = tmp_path / "work"
    for m, vs in (("SA", SA), ("SST", SST)):
        for lv, (cf, cd) in enumerate(vs):
            fake_run(work, m, lv, cf=cf, cd=cd)
    a = tm.write(work, tmp_path / "a.csv").read_bytes()
    elsewhere = tmp_path / "elsewhere"
    elsewhere.mkdir()
    monkeypatch.chdir(elsewhere)
    b = tm.write(work, tmp_path / "b.csv").read_bytes()
    assert a == b
    head = [ln for ln in a.decode().splitlines() if ln.startswith("#")]
    assert head and any("THE GRIDS ARE NOT THE TMR'S" in ln for ln in head)
    assert any("config.rs:12-18" in ln for ln in head)
    cols = next(csv.reader(l for l in a.decode().splitlines() if not l.startswith("#")))
    assert cols == tm.ROW_COLUMNS


REAL_LOG_LINE = '''|   / __| | | |_  )   Release 8.3.0 "Harrier"                           |'''


def test_the_su2_version_is_read_from_the_banner_of_a_real_log(tmp_path):
    d = tm.case_dir(tmp_path, "SA", 0)
    d.mkdir(parents=True)
    (d / "log_final.txt").write_text("|   ___ _   _ ___  |\n" + REAL_LOG_LINE + "\n")
    assert tm.su2_version(tmp_path) == "8.3.0"
    (d / "log_final.txt").write_text("no banner\n")
    assert tm.su2_version(tmp_path) == "unknown"


def test_the_sst_gci_note_says_it_is_first_order_and_how_far_the_last_refinement_moved_the_value():
    rs = rows()
    g = pick(rs, kind="gci", model="SST", quantity="cf_x097")
    ch = abs(100 * (SST[1][0] - SST[2][0]) / SST[2][0])
    assert "first order" in g["note"] and f"moved cf_x097 by {ch:.3g} %" in g["note"] and "GCI of" in g["note"]
    assert "first order" not in pick(rs, kind="gci", model="SA", quantity="cf_x097")["note"]
