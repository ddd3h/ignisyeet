# SPDX-License-Identifier: GPL-3.0-or-later
"""The collector of layer 1 (the cone-cylinder through the IgnisYeet pipeline), on synthetic work directories."""
import csv
import json
import pathlib

import pytest

from nasa_vv.cases import layer1_cone as L1

BINNED = """\
# layer 1 test. Taylor-Maccoll (integrated in rans_cone.py): Cp = 0.09673, shock angle 26.737 deg, cone surface Mach 2.1468. Cp ...
x_over_lc_from,x_over_lc_to,nodes,cp_mean,error_vs_taylor_maccoll_percent
0.2,0.3,100,0.09000,-6.95
0.3,0.4,200,0.09300,-3.85
0.4,0.5,300,0.09500,-1.79
0.5,0.6,400,0.09600,-0.75
0.6,0.7,500,0.09620,-0.55
0.7,0.8,600,0.09640,-0.34
0.8,0.9,700,0.09660,-0.13
0.2,0.9,2800,0.09574,-1.02
"""


def _binned(tmp_path, text=BINNED):
    p = tmp_path / "cone_cp.csv"
    p.write_text(text)
    return L1.parse_binned(p)


def test_the_binned_table_is_read_with_its_header_and_the_all_row_is_not_a_bin(tmp_path):
    b = _binned(tmp_path)
    assert [x.lo for x in b.bins] == pytest.approx([0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8])
    assert [x.nodes for x in b.bins] == [100, 200, 300, 400, 500, 600, 700]
    assert b.script_cp_exact == pytest.approx(0.09673)
    assert b.all_row.nodes == 2800 and b.all_row.cp == pytest.approx(0.09574)


def test_cp_mean_weights_the_bins_by_their_nodes_and_reproduces_the_scripts_own_mean(tmp_path):
    b = _binned(tmp_path)
    full = L1.cp_mean(b.bins, 0.2, 0.9)
    # node-weighted: sum(n cp) / sum(n); the unweighted mean of the seven bins is 0.09474
    expected = (100 * 0.09 + 200 * 0.093 + 300 * 0.095 + 400 * 0.096 + 500 * 0.0962 + 600 * 0.0964 + 700 * 0.0966) / 2800
    assert full == pytest.approx(expected, abs=1e-15)
    assert full != pytest.approx(sum(x.cp for x in b.bins) / 7, abs=1e-4)
    assert full == pytest.approx(b.all_row.cp, abs=5e-6)       # the script rounds to five decimals


def test_cp_mean_over_a_sub_range_uses_only_the_bins_inside_it(tmp_path):
    b = _binned(tmp_path)
    got = L1.cp_mean(b.bins, 0.4, 0.9)
    expected = (300 * 0.095 + 400 * 0.096 + 500 * 0.0962 + 600 * 0.0964 + 700 * 0.0966) / 2500
    assert got == pytest.approx(expected, abs=1e-15)


def test_a_range_that_matches_no_bin_edges_is_refused(tmp_path):
    b = _binned(tmp_path)
    with pytest.raises(ValueError):
        L1.cp_mean(b.bins, 0.45, 0.9)
    with pytest.raises(ValueError):
        L1.cp_mean(b.bins, 0.95, 1.0)


def test_cp_nearest_tip_is_the_bin_with_the_smallest_x_over_l_whatever_the_row_order(tmp_path):
    b = _binned(tmp_path)
    assert L1.cp_nearest_tip(b.bins).cp == pytest.approx(0.09)
    assert L1.cp_nearest_tip(list(reversed(b.bins))).cp == pytest.approx(0.09)


def test_a_table_without_the_all_row_or_with_a_missing_column_is_named(tmp_path):
    p = tmp_path / "x.csv"
    p.write_text(BINNED.replace("0.2,0.9,2800,0.09574,-1.02\n", ""))
    with pytest.raises(ValueError, match="0.2,0.9"):
        L1.parse_binned(p)
    p.write_text(BINNED.replace("nodes,cp_mean", "n,cp_mean"))
    with pytest.raises(ValueError, match="nodes"):
        L1.parse_binned(p)


def test_pipeline_contribution_is_layer1_minus_layer0_in_percentage_points():
    assert L1.pipeline_contribution_pct(-0.87, -0.1877) == pytest.approx(-0.6823)
    assert L1.pipeline_contribution_pct(0.5, -0.25) == pytest.approx(0.75)
    assert L1.pipeline_contribution_pct(-0.2, -0.2) == 0.0


@pytest.mark.parametrize(
    "contribution,g0,g1,word",
    [
        (0.3, 0.5, 0.1, "indistinguishable"),     # smaller than layer 0's GCI only
        (0.3, 0.1, 0.5, "indistinguishable"),     # smaller than layer 1's GCI only
        (0.3, 0.5, 0.6, "indistinguishable"),     # smaller than both
        (-0.3, 0.5, 0.1, "indistinguishable"),    # the sign does not matter
        (0.9, 0.5, 0.6, "exceeds"),               # larger than both
        (0.3, 0.1, None, "not assessed"),         # layer 1 has no GCI and layer 0's does not cover it
        (0.3, 0.5, None, "indistinguishable"),    # layer 1 has no GCI but layer 0's already does
        (0.3, None, None, "not assessed"),
    ],
)
def test_the_reading_says_indistinguishable_when_either_gci_exceeds_the_contribution(contribution, g0, g1, word):
    text = L1.reading(contribution, g0, g1)
    assert word in text
    if word == "indistinguishable":
        assert "at this resolution" in text
        assert f"{abs(contribution):g}" not in text      # no difference is quoted that the uncertainty cannot support


def test_the_reading_never_calls_the_pipeline_contribution_meshing_error():
    for g1 in (None, 0.05, 2.0):
        assert "meshing error" not in L1.reading(0.3, 0.1, g1).replace("including 3-D unstructured meshing", "")
    assert L1.CONTRIBUTION_NAME == "pipeline error, including 3-D unstructured meshing"


# ----------------------------------------------------------------- a synthetic work directory

import math

GAMMA, MACH_IN, P_INF = 1.4, 2.35, 101084.9684
LC = 0.05 / math.tan(math.radians(10.0))


def _cp_to_p(cp):
    return P_INF * (1.0 + cp * GAMMA * MACH_IN ** 2 / 2.0)


def _surface(case, *, n=30, vtk_cut=None, csv_cut=None, stale=False, freq=10, cfg=True, point_data=None):
    """A synthetic surface_flow.vtk / surface_flow.csv / case.cfg of n wall nodes on the cone.

    vtk_cut: 'cp' cuts the file inside the Pressure_Coefficient values, 'velocity' inside the Velocity vectors,
    'before_cp' before the Pressure_Coefficient header. csv_cut: 'row' cuts the last row mid-number.
    stale: the csv holds a later state than the vtk (a periodic snapshot)."""
    xs = [LC * (0.05 + 0.9 * k / (n - 1)) for k in range(n)]
    pts = [(x, x * math.tan(math.radians(10.0)), 0.0) for x in xs]
    cp = [0.09 + 1e-4 * k for k in range(n)]
    pres = [_cp_to_p(c) for c in cp]
    rho = [1.0 + 0.01 * k for k in range(n)]
    f = lambda a: "\t".join(f"{v:.6e}" for v in a) + "\t"
    pd = n if point_data is None else point_data
    lines = ["# vtk DataFile Version 3.0", "vtk output", "ASCII", "DATASET UNSTRUCTURED_GRID", f"POINTS {n} double",
             f([c for p in pts for c in p]), "CELLS 0 0", "CELL_TYPES 0", "", f"POINT_DATA {pd}", "",
             "SCALARS Density double 1", "LOOKUP_TABLE default", f(rho),
             "SCALARS Pressure double 1", "LOOKUP_TABLE default", f(pres),
             "SCALARS Pressure_Coefficient double 1", "LOOKUP_TABLE default", f(cp),
             "VECTORS Velocity double", f([300.0] * (3 * n))]
    text = "\n".join(lines)
    if vtk_cut == "cp":
        i = text.index("SCALARS Pressure_Coefficient")
        j = text.index("\n", text.index("LOOKUP_TABLE", i)) + 1
        text = text[: j + 5 * 13 + 4]
    elif vtk_cut == "velocity":
        i = text.index("VECTORS Velocity")
        text = text[: i + 40]
    elif vtk_cut == "before_cp":
        text = text[: text.index("SCALARS Pressure_Coefficient") + 12]
    (case / "surface_flow.vtk").write_text(text)
    scale = 1.5 if stale else 1.0
    rows = ['"PointID","x","y","z","Density","Momentum_x","Momentum_y","Momentum_z","Energy"']
    for k in range(n):
        pcsv = _cp_to_p(cp[k] + (1e-3 if stale else 0.0))
        rows.append(f"{k}, {pts[k][0]:.15e}, {pts[k][1]:.15e}, {pts[k][2]:.15e}, {rho[k] * scale:.15e}, 0.0, 0.0, 0.0, {pcsv / 0.4:.15e}")
    ctext = "\n".join(rows) + "\n"
    if csv_cut == "row":
        ctext = ctext[: ctext.rindex(",") - 5]
    (case / "surface_flow.csv").write_text(ctext)
    if cfg:
        (case / "case.cfg").write_text(f"MACH_NUMBER= {MACH_IN}\nFREESTREAM_PRESSURE= {P_INF}\nGAMMA_VALUE= 1.4\nOUTPUT_WRT_FREQ= {freq}\n")


def _history(path, peak, final, n=40):
    rows = ['"Time_Iter","Outer_Iter","Inner_Iter","rms[Rho]","CD"']
    for i in range(n):
        v = peak - (peak - final) * i / (n - 1) if i else peak
        rows.append(f"0,0,{i},{v:.6f},0.09")
    path.write_text("\n".join(rows) + "\n")


def _level(work, name, nodes, cells, *, peak=-3.0, final=-9.0, converged=True, cp_scale=1.0, surface=None):
    run = work / name / "run"
    case = run / "out" / "cfd" / L1.CASE_DIR
    mesh = run / "out" / "cfd" / "mesh"
    case.mkdir(parents=True)
    mesh.mkdir(parents=True)
    (mesh / "mesh_stats.json").write_text(json.dumps({"nodes": nodes, "cells": cells}))
    _history(case / "history.csv", peak, final)
    _surface(case, **(surface or {}))
    (run / "done.json").write_text(json.dumps(
        {"state": "done", "exit_code": 0, "wall_seconds": 12.5, "node": "virgo02", "ppn": 32,
         "jobid": "1.virgo", "binary": "ignisyeet-glibc-a8dff9c"}))
    if converged is True:
        (case / "done.json").write_text(json.dumps({"converged": True, "iterations": 39, "residual_last": final}))
        (run / "job.log").write_text("Aero table: CFD, 2 cases (0 failed, 0 unconverged)\n")
    elif converged is False:
        (run / "job.log").write_text(
            "Error: all CFD cases at the lowest Mach number (2.35) failed\n"
            "first failed case M=2.35 alpha=0 beta=0 deg (/tmp/x/m2.350_a00.00): not converged "
            "(iteration limit): residual -4.62 (target -6), drop 2.1 orders, coefficients stable\n")
    cp = work / name / "cp"
    cp.mkdir(parents=True)
    text = BINNED
    # scale the cp column so that the three levels differ
    out = []
    for ln in text.splitlines():
        parts = ln.split(",")
        if ln.startswith("#") or ln.startswith("x_over") or len(parts) != 5:
            out.append(ln)
        else:
            out.append(",".join(parts[:3] + [f"{float(parts[3]) * cp_scale:.5f}", parts[4]]))
    (cp / "cone_cp.csv").write_text("\n".join(out) + "\n")


def _work(tmp_path, *, coarse_converged=True, fine_converged=True, fine_surface=None):
    work = tmp_path / "work"
    work.mkdir()
    (work / "versions.txt").write_text("remote: IgnisYeet 0.4.0 (commit 2f31ab9)\nlocal: IgnisYeet 0.4.0\n")
    _level(work, "coarse", 28887, 132328, converged=coarse_converged,
           peak=-2.5, final=-4.6 if not coarse_converged else -9.0, cp_scale=0.97)
    _level(work, "medium", 45258, 209052, cp_scale=0.99)
    _level(work, "fine", 75796, 350868, cp_scale=1.0, converged=fine_converged,
           final=-9.0 if fine_converged else -5.9, surface=fine_surface)
    return work


def _read_csv(path):
    lines = [ln for ln in pathlib.Path(path).read_text().splitlines() if not ln.startswith("#")]
    return list(csv.DictReader(lines))


def test_a_level_carries_its_nodes_its_residual_drop_and_the_solver_verdict(tmp_path):
    work = _work(tmp_path, coarse_converged=False)
    fine = L1.read_level(work, "fine", 0.002)
    coarse = L1.read_level(work, "coarse", 0.00392)
    assert fine.grid.nodes == 75796 and fine.grid.knobs == {"wall_size_m": 0.002}
    assert fine.grid.solver_converged is True
    assert fine.grid.residual_drop_orders == pytest.approx(6.0)
    assert coarse.grid.solver_converged is False
    assert coarse.grid.residual_drop_orders == pytest.approx(2.1)
    assert fine.cells == 350868


def test_a_run_with_neither_a_done_file_nor_a_recognisable_failure_has_no_verdict(tmp_path):
    work = _work(tmp_path)
    (work / "medium" / "run" / "out" / "cfd" / L1.CASE_DIR / "done.json").unlink()
    (work / "medium" / "run" / "job.log").write_text("something else\n")
    assert L1.read_level(work, "medium", 0.0028).grid.solver_converged is None


def test_the_failure_of_the_alpha_4_case_does_not_decide_the_alpha_0_level(tmp_path):
    work = _work(tmp_path)
    (work / "fine" / "run" / "job.log").write_text(
        "first failed case M=2.35 alpha=4 beta=0 deg (/tmp/x/m2.350_a04.00): SU2 exited with exit status: 1\n")
    assert L1.read_level(work, "fine", 0.002).grid.solver_converged is True


def test_the_collector_writes_per_level_cp_and_gci_rows_with_both_h_bases(tmp_path):
    work = _work(tmp_path)
    out = tmp_path / "out"
    L1.main(["--work", str(work), "--out", str(out)])
    rows = _read_csv(out / "layer1_cone.csv")
    assert set(rows[0]) >= {"level", "nodes", "wall_size_m", "x_over_l", "cp_cfd", "cp_exact", "difference",
                            "difference_pct", "residual_drop_orders"}
    assert {r["level"] for r in rows} == {"coarse", "medium", "fine"}
    fine_mean = next(r for r in rows if r["level"] == "fine" and r["x_over_l"] == "0.2-0.9")
    assert float(fine_mean["cp_exact"]) == pytest.approx(L1.exact.taylor_maccoll(2.35, 10.0).cp)
    assert float(fine_mean["difference_pct"]) == pytest.approx(
        100.0 * (float(fine_mean["cp_cfd"]) - float(fine_mean["cp_exact"])) / float(fine_mean["cp_exact"]), abs=1e-5)
    g = _read_csv(out / "layer1_gci.csv")
    assert {r["h_basis"] for r in g} == {"nodes^-1/3", "wall_size"}
    assert {r["quantity"] for r in g} == {"cp_mean", "cp_mean_0.4_0.9", "cp_nearest_tip"}
    assert all(r["gci_fine_pct"] != "" for r in g)
    # r21 differs between the bases: the knob ratio is 1.4, the node ratio is not
    r_nodes = float(next(r for r in g if r["h_basis"] == "nodes^-1/3")["r21"])
    r_knob = float(next(r for r in g if r["h_basis"] == "wall_size")["r21"])
    assert r_knob == pytest.approx(1.4) and abs(r_nodes - r_knob) > 0.05


def test_a_level_the_solver_did_not_accept_refuses_every_gci_row_and_keeps_the_cp_comparison(tmp_path):
    work = _work(tmp_path, coarse_converged=False)
    out = tmp_path / "out"
    L1.main(["--work", str(work), "--out", str(out)])
    g = _read_csv(out / "layer1_gci.csv")
    assert len(g) == 6
    for r in g:
        assert r["note"].startswith("refused:") and "coarse" in r["note"]
        assert r["gci_fine_pct"] == "" and r["order"] == "" and r["extrapolated"] == ""
    rows = _read_csv(out / "layer1_cone.csv")
    coarse = [r for r in rows if r["level"] == "coarse"]
    assert coarse and all(r["difference_pct"] != "" for r in coarse)
    assert all(r["solver_converged"] == "False" for r in coarse)


def test_the_comparison_row_follows_the_definition_and_one_row_per_layer1_basis(tmp_path):
    work = _work(tmp_path)
    out = tmp_path / "out"
    L1.main(["--work", str(work), "--out", str(out)])
    rows = _read_csv(out / "layer0_vs_layer1.csv")
    matched = [r for r in rows if r["quantity"] == "cp_mean_0.4_0.9"]
    assert {r["layer1_h_basis"] for r in matched} == {"nodes^-1/3", "wall_size"}
    for r in matched:
        assert float(r["pipeline_contribution_pct"]) == pytest.approx(
            float(r["layer1_diff_pct"]) - float(r["layer0_diff_pct"]), abs=1e-5)
        assert float(r["layer0_fine"]) == pytest.approx(0.09654900864)     # layer0.csv, as committed
        assert float(r["layer0_diff_pct"]) == pytest.approx(-0.1877, abs=1e-4)
        assert float(r["layer0_gci_pct"]) == pytest.approx(0.05937555, rel=1e-6)
        assert r["layer0_gci_bounds_error"] == "False"
    for r in rows:
        if r["quantity"] != "cp_mean_0.4_0.9":
            assert r["layer0_fine"] == "" and r["pipeline_contribution_pct"] == ""


def test_the_indistinguishable_note_reaches_the_csv_when_the_contribution_is_below_a_gci(tmp_path):
    work = _work(tmp_path)
    out = tmp_path / "out"
    L1.main(["--work", str(work), "--out", str(out)])
    for r in _read_csv(out / "layer0_vs_layer1.csv"):
        if r["quantity"] != "cp_mean_0.4_0.9" or r["pipeline_contribution_pct"] == "":
            continue
        c = abs(float(r["pipeline_contribution_pct"]))
        gcis = [float(r[k]) for k in ("layer0_gci_pct", "layer1_gci_pct") if r[k] != ""]
        assert ("indistinguishable" in r["reading"]) == any(c < x for x in gcis)


def test_the_comparison_names_a_refused_layer1_instead_of_inventing_its_uncertainty(tmp_path):
    work = _work(tmp_path, coarse_converged=False)
    out = tmp_path / "out"
    L1.main(["--work", str(work), "--out", str(out)])
    for r in _read_csv(out / "layer0_vs_layer1.csv"):
        assert r["layer1_gci_pct"] == ""
        if r["quantity"] == "cp_mean_0.4_0.9":
            assert r["pipeline_contribution_pct"] != ""
            assert "GCI refused" in r["note"] or "not assessed" in r["reading"] or "indistinguishable" in r["reading"]


def test_a_fine_level_the_solver_did_not_accept_gives_no_pipeline_contribution(tmp_path):
    work = _work(tmp_path, fine_converged=False)
    out = tmp_path / "out"
    L1.main(["--work", str(work), "--out", str(out)])
    for r in _read_csv(out / "layer1_gci.csv"):
        assert r["note"].startswith("refused:") and "fine" in r["note"]
    rows = [r for r in _read_csv(out / "layer0_vs_layer1.csv") if r["quantity"] == "cp_mean_0.4_0.9"]
    assert rows
    for r in rows:
        assert r["layer1_diff_pct"] != "" and r["pipeline_contribution_pct"] == ""
        assert "not assessed" in r["reading"] and "fine level" in r["reading"]
    assert "not accepted" in L1.reading(0.3, 0.5, None, fine_accepted=False)


def _run(tmp_path, **kw):
    work = _work(tmp_path, **kw)
    out = tmp_path / "out"
    L1.main(["--work", str(work), "--out", str(out)])
    return work, out


def test_complete_files_are_a_final_write_at_the_last_history_iteration(tmp_path):
    work = _work(tmp_path)
    src = L1.read_level(work, "fine", 0.002).source
    assert src.ok and src.problems == ()
    assert src.kind == "final write" and src.iteration == 40 == src.history_rows
    assert src.points == 30 and src.csv_rows_complete == 30 and src.compared == 30


def test_a_vtk_cut_inside_the_cp_field_refuses_the_level_with_a_note(tmp_path):
    work, out = _run(tmp_path, fine_surface={"vtk_cut": "cp"})
    src = L1.read_level(work, "fine", 0.002).source
    assert not src.ok and any("Pressure_Coefficient" in p for p in src.problems)
    for r in _read_csv(out / "layer1_gci.csv"):
        assert r["note"].startswith("refused:") and "Cp source" in r["note"] and "Pressure_Coefficient" in r["note"]
        assert "refused: refused" not in r["note"]
    fine = [r for r in _read_csv(out / "layer1_cone.csv") if r["level"] == "fine"]
    assert fine and all(r["cp_source_check"].startswith("failed") for r in fine)       # the Cp comparison still appears
    assert all(r["difference_pct"] != "" for r in fine)
    comp = [r for r in _read_csv(out / "layer0_vs_layer1.csv") if r["quantity"] == "cp_mean_0.4_0.9"]
    assert all(r["pipeline_contribution_pct"] == "" and "not assessed" in r["reading"] for r in comp)


def test_a_vtk_cut_before_the_cp_header_is_refused(tmp_path):
    work = _work(tmp_path, fine_surface={"vtk_cut": "before_cp"})
    src = L1.read_level(work, "fine", 0.002).source
    assert not src.ok and any("Pressure_Coefficient" in p for p in src.problems)


def test_a_vtk_whose_point_data_count_differs_from_its_points_is_refused(tmp_path):
    work = _work(tmp_path, fine_surface={"point_data": 29})
    src = L1.read_level(work, "fine", 0.002).source
    assert not src.ok and any("POINT_DATA" in p for p in src.problems)


def test_a_vtk_cut_in_a_field_after_cp_keeps_the_level_and_records_the_field(tmp_path):
    work = _work(tmp_path, fine_surface={"vtk_cut": "velocity"})
    src = L1.read_level(work, "fine", 0.002).source
    assert src.ok and any("Velocity" in n for n in src.notes)


def test_a_csv_cut_mid_row_is_recorded_and_the_complete_rows_decide_the_freshness(tmp_path):
    work = _work(tmp_path, fine_surface={"csv_cut": "row"})
    src = L1.read_level(work, "fine", 0.002).source
    assert src.ok and src.csv_rows_complete == 29 and src.csv_rows_expected == 30 and src.compared == 29
    assert src.kind == "final write" and any("surface_flow.csv" in n and "29 of 30" in n for n in src.notes)


def test_a_vtk_older_than_the_csv_is_a_periodic_snapshot_with_its_iteration_and_cp_difference(tmp_path):
    work = _work(tmp_path, fine_surface={"stale": True, "freq": 15})
    src = L1.read_level(work, "fine", 0.002).source
    assert src.ok and src.kind == "periodic snapshot"
    assert src.iteration == 30 and src.history_rows == 40          # 40 rows, written every 15: the snapshot of iteration 30
    assert src.snapshot_cp_difference is not None
    d = src.snapshot_cp_difference
    assert d["nodes"] > 0 and d["window_nodes"] >= d["nodes"]
    assert d["mean_abs_cp"] == pytest.approx(1e-3, rel=1e-3)


def test_a_snapshot_is_written_into_the_cone_csv_and_the_header(tmp_path):
    work, out = _run(tmp_path, fine_surface={"stale": True, "freq": 15})
    rows = [r for r in _read_csv(out / "layer1_cone.csv") if r["level"] == "fine"]
    assert {r["cp_source"] for r in rows} == {"periodic snapshot"}
    assert {r["cp_source_iteration"] for r in rows} == {"30"} and {r["history_iterations"] for r in rows} == {"40"}
    assert {r["cp_source_check"] for r in rows} == {"ok"}
    assert {r["cp_source"] for r in _read_csv(out / "layer1_cone.csv") if r["level"] == "coarse"} == {"final write"}
    head = "\n".join(ln for ln in (out / "layer1_cone.csv").read_text().splitlines() if ln.startswith("#"))
    assert "periodic snapshot" in head and "iteration 30" in head


def test_an_unaccepted_fine_level_carries_the_note_on_every_quantity_row(tmp_path):
    work, out = _run(tmp_path, fine_converged=False)
    rows = _read_csv(out / "layer0_vs_layer1.csv")
    assert rows
    for r in rows:
        assert "solver did not accept" in r["note"] and "refused: refused" not in r["note"]


def test_the_header_states_the_decomposition_and_the_truncation_as_observed(tmp_path):
    work, out = _run(tmp_path)
    head = "\n".join(ln for ln in (out / "layer1_cone.csv").read_text().splitlines() if ln.startswith("#"))
    assert "domain decomposition" in head and "rank count" in head and "not tested" in head
    assert "cut when the run is stopped" in head


def test_rerunning_the_collector_gives_byte_identical_files(tmp_path):
    work = _work(tmp_path)
    a, b = tmp_path / "a", tmp_path / "b"
    L1.main(["--work", str(work), "--out", str(a)])
    L1.main(["--work", str(work), "--out", str(b)])
    for name in ("layer1_cone.csv", "layer1_gci.csv", "layer0_vs_layer1.csv"):
        assert (a / name).read_bytes() == (b / name).read_bytes()


def test_the_header_states_the_scope_wording_and_the_versions(tmp_path):
    work = _work(tmp_path)
    out = tmp_path / "out"
    L1.main(["--work", str(work), "--out", str(out)])
    head = "\n".join(ln for ln in (out / "layer1_cone.csv").read_text().splitlines() if ln.startswith("#"))
    assert "verification against an exact solution" in head
    assert "pipeline error, including 3-D unstructured meshing" in head
    assert "check_similar" in head and "1.4^3" in head
    assert "remote: IgnisYeet 0.4.0 (commit 2f31ab9)" in head
    assert "fin_size" in head and "mesh_gmsh.py:156" in head
    assert "binary ignisyeet-glibc-a8dff9c)" in head


def test_paths_are_resolved_from_the_module_and_not_from_the_working_directory(tmp_path, monkeypatch):
    work = _work(tmp_path)
    cwd = tmp_path / "elsewhere"
    cwd.mkdir()
    monkeypatch.chdir(cwd)
    out = tmp_path / "out"
    L1.main(["--work", str(work), "--out", str(out)])
    assert (out / "layer0_vs_layer1.csv").exists()
    assert L1.RESULTS.is_absolute() and L1.RESULTS.name == "results"


def test_the_default_work_directory_is_on_disk_and_the_collector_only_reads_it():
    assert not str(L1.DEFAULT_WORK).startswith("/tmp") and L1.DEFAULT_WORK.is_absolute()
