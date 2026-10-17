# SPDX-License-Identifier: GPL-3.0-or-later
"""Reads the committed CSVs. Paths are resolved from this file, not the working directory."""
import json
import pathlib

import pytest

from nasa_vv.cases import existing_data as ed
from nasa_vv.cases.existing_data import (
    read_basic_finner_mesh_check,
    read_flatplate,
    read_rans_mesh_study,
)
from nasa_vv.gci import grid_convergence

ROOT = pathlib.Path(__file__).resolve().parents[3]
RANS = ROOT / "doc/data/cfd/rans_mesh_study.csv"
RANS_JSON = ROOT / "doc/data/cfd/rans_mesh_study.json"
PLATE = ROOT / "doc/data/cfd/rans_flatplate.csv"
FINNER = ROOT / "doc/data/validation/basic_finner/mesh_check.csv"


# ---------------------------------------------------------------- status mapping


def test_every_status_string_is_mapped_explicitly():
    assert ed._verdict(["converged", "converged"]) is True
    assert ed._verdict(["converged", "unconverged"]) is False
    assert ed._verdict(["converged", "rerun (direct)"]) is None


def test_a_false_solve_outranks_an_unrecorded_one():
    assert ed._verdict(["rerun (direct)", "unconverged"]) is False


def test_an_unknown_status_string_is_an_error_not_a_guess():
    with pytest.raises(ValueError, match="solved"):
        ed._verdict(["converged", "solved"])


def test_a_missing_column_is_named(tmp_path):
    p = tmp_path / "x.csv"
    p.write_text("# comment\na,b\n1,2\n")
    with pytest.raises(ValueError, match="'c'"):
        ed._read_table(p, ("a", "c"))


# ---------------------------------------------------------------- RANS sample rocket


def test_rans_mesh_study_gives_one_family_per_mach_and_quantity_group():
    """Keys are (Mach, group); every family has three levels, finest first."""
    fams = read_rans_mesh_study(RANS)
    assert set(fams) == {(0.8, "alpha4"), (0.8, "alpha0"), (2.0, "alpha4"), (2.0, "alpha0")}
    for fd in fams.values():
        assert [lv.name for lv in fd.family.levels] == ["3 mm", "4.8 mm", "8 mm"]
        n = [lv.nodes for lv in fd.family.levels]
        assert n == sorted(n, reverse=True) and n[0] == 1844927
    assert set(fams[(0.8, "alpha4")].values) == {"cn", "ca", "xcp"}
    assert set(fams[(0.8, "alpha0")].values) == {"cn0"}


def test_a_level_carries_the_and_of_the_solves_its_quantities_use():
    fams = read_rans_mesh_study(RANS)
    v = lambda key: [lv.solver_converged for lv in fams[key].family.levels]  # noqa: E731
    # finest first: 3 mm, 4.8 mm, 8 mm
    assert v((0.8, "alpha4")) == [True, True, False]   # 8 mm: alpha -4 unconverged
    assert v((0.8, "alpha0")) == [True, False, False]  # alpha 0 unconverged at 8 and 4.8 mm
    assert v((2.0, "alpha4")) == [None, True, True]    # 3 mm: alpha -4 is "rerun (direct)"
    assert v((2.0, "alpha0")) == [True, False, False]


def test_rans_mach_08_alpha4_is_refused_and_names_the_unconverged_level():
    """The plan asserted check_similar() passes for both Mach numbers. It does not
    at M = 0.8: the 8 mm level combines an unconverged alpha -4 solve, so the
    gatekeeper is right to refuse it. This test asserts what the data supports."""
    fd = read_rans_mesh_study(RANS)[(0.8, "alpha4")]
    with pytest.raises(ValueError, match=r"level '8 mm' was recorded as not converged"):
        fd.family.check_similar()


def test_rans_mach_2_alpha4_is_refused_because_the_direct_rerun_has_no_verdict():
    """'rerun (direct)' is mapped to None: the rerun bypassed the runner's
    acceptance, no residual drop or verdict is committed, and the header's
    'stationary to 1e-4 from iteration 400' covers only the coefficient leg."""
    fd = read_rans_mesh_study(RANS)[(2.0, "alpha4")]
    with pytest.raises(ValueError, match=r"level '3 mm' carries no convergence evidence"):
        fd.family.check_similar()


def test_rans_alpha0_families_are_refused_at_both_mach_numbers():
    fams = read_rans_mesh_study(RANS)
    for m in (0.8, 2.0):
        with pytest.raises(ValueError, match="not converged"):
            fams[(m, "alpha0")].family.check_similar()


def test_the_reader_reproduces_the_published_gci_arithmetic_to_full_precision():
    """Reader check, bypassing family.py: grid_convergence on the values and node
    counts the reader returns equals what rans_mesh_study.json recorded. This
    proves the reader reads the right numbers; it says nothing about convergence."""
    pub = json.loads(RANS_JSON.read_text())["mach"]
    fams = read_rans_mesh_study(RANS)
    for m, key in ((0.8, "0.8"), (2.0, "2")):
        fd = fams[(m, "alpha4")]
        h = fd.family.h()
        for q in ("cn", "ca", "xcp"):
            g = grid_convergence(*fd.values[q], *h)
            ref = pub[key][q]
            assert g.order == pytest.approx(ref["order"], rel=1e-9)
            assert g.extrapolated == pytest.approx(ref["extrapolated"], rel=1e-9)
            assert g.gci_fine == pytest.approx(ref["gci_fine"], rel=1e-9)


# ---------------------------------------------------------------- flat plate


def test_flatplate_family_has_a_refinement_ratio_of_two():
    fd = read_flatplate(PLATE)[0.2]
    h = fd.family.h()
    assert fd.family.ndim == 2
    assert h[1] / h[0] == pytest.approx(2.0, rel=1e-9)
    assert h[2] / h[1] == pytest.approx(2.0, rel=1e-9)
    assert [lv.name for lv in fd.family.levels] == ["273x193", "137x97", "69x49"]


def test_flatplate_h_is_the_csv_column_and_the_drop_is_the_weaker_field():
    fd = read_flatplate(PLATE)[0.2]
    assert fd.family.h() == pytest.approx([0.00437588, 0.00875175, 0.0175035], rel=1e-5)
    # rho 8.00061 / nu 8.06735 on the finest grid: the minimum governs
    assert fd.family.levels[0].residual_drop_orders == pytest.approx(8.00061)
    assert all(lv.solver_converged is None for lv in fd.family.levels)
    assert all(lv.basis == "residual" for lv in fd.family.levels)


def test_flatplate_mach_2_middle_grid_uses_its_own_smaller_drop():
    fd = read_flatplate(PLATE)[2.0]
    assert fd.family.levels[1].residual_drop_orders == pytest.approx(5.7606)
    fd.family.check_similar()


def test_flatplate_gci_matches_the_variable_ratio_formula_at_r_equal_two():
    res = ed.flatplate_results(read_flatplate(PLATE))
    row = next(r for r in res if r["condition"] == "M=0.2" and r["quantity"] == "cf_x097")
    fd = read_flatplate(PLATE)[0.2]
    g = grid_convergence(*fd.values["cf_x097"], *fd.family.h())
    assert float(row["order"]) == pytest.approx(g.order, rel=1e-6)
    assert float(row["gci_fine_pct"]) == pytest.approx(g.gci_fine * 100, rel=1e-6)


# ---------------------------------------------------------------- Basic Finner


def test_basic_finner_has_four_euler_levels():
    d = read_basic_finner_mesh_check(FINNER)
    assert d["levels"] == ["L1", "L2", "L3", "L4"]


def test_basic_finner_triplets_are_explicit_and_finest_first():
    d = read_basic_finner_mesh_check(FINNER)
    fd = d["conditions"][("fairing", 4.0)]
    t = ed.triplets(fd)
    assert set(t) == {"L1-L3", "L2-L4"}
    assert [lv.name for lv in t["L1-L3"].family.levels] == ["L3", "L2", "L1"]
    assert [lv.name for lv in t["L2-L4"].family.levels] == ["L4", "L3", "L2"]
    for ft in t.values():
        assert len(ft.family.levels) == 3
        ft.family.check_similar()


def test_basic_finner_has_no_cn_or_xcp_at_alpha_zero():
    d = read_basic_finner_mesh_check(FINNER)
    assert set(d["conditions"][("fairing", 0.0)].values) == {"ca_pressure"}
    assert set(d["conditions"][("fairing", 4.0)].values) == {"cn", "ca_pressure", "xcp_cal"}


def test_basic_finner_two_triplets_disagree_on_the_axial_force_order():
    res = ed.basic_finner_results(read_basic_finner_mesh_check(FINNER))
    pick = lambda trip: next(  # noqa: E731
        r for r in res if r["condition"] == "fairing alpha=4"
        and r["quantity"] == "ca_pressure" and r["triplet"] == trip)
    assert float(pick("L1-L3")["order"]) == pytest.approx(1.55, abs=0.01)
    assert float(pick("L2-L4")["order"]) == pytest.approx(2.69, abs=0.01)


# ---------------------------------------------------------------- the CSV


def test_main_is_byte_identical_on_rerun_and_independent_of_the_working_directory(
    tmp_path, monkeypatch
):
    monkeypatch.chdir(tmp_path)
    a, b = tmp_path / "a.csv", tmp_path / "b.csv"
    ed.main(a)
    ed.main(b)
    assert a.read_bytes() == b.read_bytes()


def test_every_row_has_the_columns_and_every_blank_result_has_a_note(tmp_path):
    import csv

    out = tmp_path / "o.csv"
    ed.main(out)
    lines = [ln for ln in out.read_text().splitlines() if not ln.startswith("#")]
    rows = list(csv.DictReader(lines))
    assert list(rows[0]) == ed.COLUMNS
    assert rows
    for r in rows:
        assert len(r) == len(ed.COLUMNS)
        if r["order"] == "":
            assert r["note"] != "", r
    head = [ln for ln in out.read_text().splitlines() if ln.startswith("#")]
    assert any("rans_mesh_study.csv" in ln for ln in head)
    assert any("rans_flatplate.csv" in ln for ln in head)
    assert any("mesh_check.csv" in ln for ln in head)


def test_the_rans_sample_rows_are_refused_with_the_gatekeepers_message(tmp_path):
    import csv

    out = tmp_path / "o.csv"
    ed.main(out)
    lines = [ln for ln in out.read_text().splitlines() if not ln.startswith("#")]
    rows = [r for r in csv.DictReader(lines) if r["study"] == "rans_mesh_study"]
    assert rows
    assert all(r["order"] == "" and r["note"].startswith("refused: ") for r in rows)
    m08 = next(r for r in rows if r["condition"].startswith("M=0.8 alpha=4"))
    assert "level '8 mm' was recorded as not converged" in m08["note"]
    m2 = next(r for r in rows if r["condition"].startswith("M=2 alpha=4"))
    assert "level '3 mm' carries no convergence evidence" in m2["note"]
