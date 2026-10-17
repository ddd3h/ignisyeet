# SPDX-License-Identifier: GPL-3.0-or-later
"""Iterative error against the grid uncertainty, on small synthetic inputs (no CFD is run)."""
import csv
import math
import pathlib

import pytest

from nasa_vv.cases import iterative as it
from test_sample_euler import make_level

GCI_COLS = ("study,condition,quantity,triplet,h_basis,n1,n2,n3,r21,r32,f1,f2,f3,order,extrapolated,"
            "gci_fine_pct,gci_coarse_pct,asymptotic_ratio,monotone,note").split(",")


def gci_row(condition, quantity, h_basis="nodes^-1/3", gci="0.5", gci_coarse="1.0", note="", study="s"):
    r = dict.fromkeys(GCI_COLS, "")
    r.update(study=study, condition=condition, quantity=quantity, h_basis=h_basis, gci_fine_pct=gci,
             gci_coarse_pct=gci_coarse, note=note)
    return r


# ------------------------------------------------------------------ arithmetic


def test_iterative_error_is_the_relative_difference_in_percent():
    assert it.iterative_error_pct(1.19556, 1.20218) == pytest.approx(100 * 0.00662 / 1.20218)
    assert it.iterative_error_pct(1.2, 1.0) == pytest.approx(20.0)
    assert it.iterative_error_pct(0.8, 1.0) == pytest.approx(20.0)  # a magnitude, never negative
    assert it.iterative_error_pct(-0.0128, -0.01) == pytest.approx(28.0)  # relative to |converged|


def test_ratio_is_the_iterative_error_over_the_gci():
    assert it.ratio(0.05, 0.5) == pytest.approx(0.1)
    assert it.ratio(1.8943, 0.037551523) == pytest.approx(50.4467, rel=1e-4)


def test_ratio_is_none_without_a_usable_gci_and_never_zero():
    assert it.ratio(0.05, None) is None
    assert it.ratio(0.05, 0.0) is None  # no division by a zero uncertainty, no invented number


def test_a_zero_iterative_error_prints_a_ratio_of_zero_only_when_the_error_is_zero():
    rows = it.build_rows(case="c", mach=0.8, alpha_deg=4, model="Euler", quantity="q", v_stop=1.0,
                         v_conv=1.0, refs=[it.GciRef("b", 0.5, "", "")], grid_match="same grid", source="s")
    assert rows[0]["iterative_error_pct"] == "0" and rows[0]["ratio"] == "0"


def test_small_errors_do_not_print_as_zero():
    rows = it.build_rows(case="c", mach=0.3, alpha_deg=0, model="Euler", quantity="q", v_stop=1.0 + 3e-9,
                         v_conv=1.0, refs=[it.GciRef("b", 100.0, "", "")], grid_match="same grid", source="s")
    assert float(rows[0]["iterative_error_pct"]) == pytest.approx(3e-7, rel=1e-3)
    assert float(rows[0]["ratio"]) == pytest.approx(3e-9, rel=1e-3)


# ------------------------------------------------------------------ the GCI lookup and the blank path


def test_find_gci_returns_one_reference_per_h_basis():
    rows = [gci_row("M=0.8 alpha=0", "ca_alpha0", "nodes^-1/3", "309.0"),
            gci_row("M=0.8 alpha=0", "ca_alpha0", "wall_size", "393.0"),
            gci_row("M=0.3 alpha=0", "ca_alpha0", "wall_size", "1.0")]
    refs = it.find_gci(rows, "s", "M=0.8 alpha=0", "ca_alpha0")
    assert [(r.h_basis, r.gci_fine_pct) for r in refs] == [("nodes^-1/3", 309.0), ("wall_size", 393.0)]


def test_find_gci_raises_when_the_condition_is_absent():
    with pytest.raises(LookupError):
        it.find_gci([gci_row("M=0.8 alpha=0", "q")], "s", "M=2 alpha=0", "q")


@pytest.mark.parametrize("note", [
    "refused: level 'medium' was recorded as not converged by the solver [solves per level: x]",
    "oscillatory convergence: the differences change sign; xcp_m of the combined row",
    "the finer pair changes at least as much as the coarser pair; the solution is not converging under refinement",
])
def test_a_blank_gci_leaves_gci_and_ratio_blank_and_says_why(note):
    refs = it.find_gci([gci_row("M=2 alpha=4 (raw)", "cna", "nodes^-1/3", "", note=note),
                        gci_row("M=2 alpha=4 (raw)", "cna", "wall_size", "", note=note)],
                       "s", "M=2 alpha=4 (raw)", "cna")
    assert all(r.gci_fine_pct is None for r in refs)
    rows = it.build_rows(case="c", mach=2, alpha_deg=4, model="Euler", quantity="cna", v_stop=0.93,
                         v_conv=0.94, refs=refs, grid_match="different grid (x)", source="s")
    assert len(rows) == 1  # identical blank reasons for both bases do not duplicate the row
    r = rows[0]
    assert r["gci_fine_pct"] == "" and r["ratio"] == ""
    assert note.split(" [")[0] in r["note"]
    assert r["iterative_error_pct"] != ""  # the iterative error itself is still reported


def test_a_blank_gci_for_one_basis_only_keeps_both_rows():
    refs = [it.GciRef("nodes^-1/3", None, "refused: a", ""), it.GciRef("wall_size", 2.0, "", "")]
    rows = it.build_rows(case="c", mach=2, alpha_deg=4, model="Euler", quantity="q", v_stop=1.01,
                         v_conv=1.0, refs=refs, grid_match="same grid", source="s")
    assert [(r["h_basis"], r["gci_fine_pct"] == "", r["ratio"] == "") for r in rows] == [
        ("nodes^-1/3", True, True), ("wall_size", False, False)]
    assert float(rows[1]["ratio"]) == pytest.approx(0.5)


def test_a_gci_flagged_unreliable_stays_in_the_ratio_but_the_note_says_so():
    refs = it.find_gci([gci_row("c", "q", "wall_size", "4250.9",
                                note="observed order p = 0.066 is below a quarter of the formal order of 2; the triplet "
                                     "is not in the asymptotic range and this GCI is not reliable")], "s", "c", "q")
    rows = it.build_rows(case="c", mach=0.8, alpha_deg=0, model="Euler", quantity="q", v_stop=1.1, v_conv=1.0,
                         refs=refs, grid_match="same grid", source="s")
    assert rows[0]["ratio"] != "" and "not reliable" in rows[0]["note"]


def test_a_gci_above_a_hundred_percent_is_flagged_in_the_note():
    refs = [it.GciRef("nodes^-1/3", 309.0, "", "")]
    rows = it.build_rows(case="c", mach=0.3, alpha_deg=0, model="Euler", quantity="q", v_stop=1.1, v_conv=1.0,
                         refs=refs, grid_match="same grid", source="s")
    assert "exceeds 100 %" in rows[0]["note"]


# ------------------------------------------------------------------ judgement and the header


def _row(case, ratio, h_basis=""):
    return {"case": case, "mach": "0.2", "quantity": "cd_plate", "ratio": ratio, "h_basis": h_basis}


def test_judge_sorts_the_rows_by_ratio():
    rows = [_row("tiny", "0.01"), _row("mid", "0.5"), _row("edge", "1"), _row("big", "3.2"), _row("blank", ""),
            _row("tenth", "0.1")]
    j = it.judge(rows)
    assert [n for n, _ in j.above_one] == [it.row_name(_row("big", ""))]
    assert [n for n, _ in j.above_tenth] == [it.row_name(_row("mid", "")), it.row_name(_row("edge", "")),
                                             it.row_name(_row("tenth", ""))]
    assert [n for n, _ in j.below_tenth] == [it.row_name(_row("tiny", ""))]
    assert [n for n in j.blank] == [it.row_name(_row("blank", ""))]


def test_the_header_lists_every_row_above_one_by_name_in_plain_words():
    rows = [_row("rans_flatplate 69x49", "3.1"), _row("rans_flatplate 137x97", "16.6"), _row("ok", "0.02")]
    text = "\n".join(it.judgement_lines(it.judge(rows)))
    assert "rans_flatplate 69x49" in text and "rans_flatplate 137x97" in text
    assert "ratio 0.02" not in text  # a row at 0.02 is not listed among those above 1.0
    assert "ratio above 1.0" in text and "3.1" in text and "16.6" in text


def test_the_header_says_none_when_no_row_exceeds_one():
    text = "\n".join(it.judgement_lines(it.judge([_row("a", "0.01"), _row("b", "")])))
    assert "above 1.0: none" in text


# ------------------------------------------------------------------ flat plate


def _flat_csv(tmp_path):
    p = tmp_path / "rans_flatplate.csv"
    p.write_text(
        "# header\n"
        "mach,grid,cells,cd_plate,cd_at_stop_vs_final_percent\n"
        "0.2,69x49,3264,0.00299974,0.116789\n"
        "0.2,137x97,13056,0.00298455,0.622387\n"
        "0.2,273x193,52224,0.00297412,1.8943\n")
    return p


def test_flatplate_pairs_the_stop_error_with_the_flatplate_gci_of_the_same_mach(tmp_path):
    gci = [gci_row("M=0.2", "cf_x097", "cells^-1/2 (2-D)", "0.037551523", gci_coarse="0.15", study="rans_flatplate")]
    rows = it.flatplate_rows(_flat_csv(tmp_path), gci, source="x")
    assert [r["case"] for r in rows] == ["rans_flatplate 69x49", "rans_flatplate 137x97", "rans_flatplate 273x193"]
    fine = rows[2]
    assert float(fine["iterative_error_pct"]) == pytest.approx(1.8943)
    assert float(fine["ratio"]) == pytest.approx(1.8943 / 0.037551523)
    assert float(fine["value_converged"]) == pytest.approx(0.00297412)
    assert float(fine["value_at_stop"]) == pytest.approx(0.00297412 * (1 + 1.8943 / 100))
    assert fine["grid_match"].startswith("same grid") and rows[0]["grid_match"].startswith("different grid")
    assert "cf_x097" in fine["note"] and "cd_plate" in fine["note"]  # the quantity differs and is said so
    assert "0.15" in rows[1]["note"]  # the 137x97 grid's own GCI (gci_coarse_pct)


# ------------------------------------------------------------------ the Task 14 levels


def _grow_history(level_dir, mach, alpha, extra):
    """Rewrite a synthetic history with the CFz column and rows after the judged row."""
    cd = next(p for p in (level_dir / "out" / "cfd").iterdir() if p.name == f"m{mach:.3f}_a{alpha:+06.2f}")
    lines = (cd / "history.csv").read_text().splitlines()
    cn = float(next(r for r in (level_dir / "out" / "cfd_cases.csv").read_text().splitlines()[1:]
                    if r.startswith(f"{mach},{alpha},")).split(",")[6])
    ca = float(lines[1].split(",")[4])
    out = ['"Time_Iter","Outer_Iter","Inner_Iter","rms[Rho]","CFx","CFz"']
    for ln in lines[1:]:
        out.append(ln + f",{cn:.8e}")
    for i, (dca, dcn) in enumerate(extra, start=3):
        out.append(f"0,0,{i},-9.0,{ca * (1 + dca):.8e},{cn * (1 + dcn):.8e}")
    (cd / "history.csv").write_text("\n".join(out) + "\n")


def test_level_rows_compare_the_judged_row_with_the_last_written_row(tmp_path):
    from nasa_vv.cases import sample_euler as se
    w = tmp_path / "work"
    w.mkdir()
    for name, wall, fin in (("coarse", 3.92, 1.96), ("medium", 2.8, 1.4), ("fine", 2.0, 1.0)):
        d = make_level(w, name, wall, fin, 100_000, 500_000, cn_slope=1.25, ca0=-0.03, machs=(0.8,))
        _grow_history(d, 0.8, 4.0, [(0, 1e-3)])
        _grow_history(d, 0.8, 0.0, [(0, 0), (-2e-3, 0)])
    make_level(w, "medium_nomirror", 2.8, 1.4, 100_000, 500_000, mirror=False, machs=(0.8,))
    levels = se.read_levels(w)
    gci = [gci_row("M=0.8 alpha=4 (raw)", "cna", "nodes^-1/3", "", note="the solution is not converging under refinement",
                   study="sample_euler"),
           gci_row("M=0.8 alpha=0", "ca_alpha0", "nodes^-1/3", "10", study="sample_euler")]
    rows = it.level_rows(levels, w, gci, source="x")
    assert {r["case"] for r in rows} == {f"sample_euler {n} level; rows after the judged row"
                                         for n in ("coarse", "medium", "fine")}
    fine_cn = next(r for r in rows if "fine" in r["case"] and r["quantity"] == "cn_a4")
    assert float(fine_cn["iterative_error_pct"]) == pytest.approx(100 * 1e-3 / 1.001, rel=1e-5)  # relative to the last row
    assert fine_cn["ratio"] == "" and fine_cn["gci_fine_pct"] == ""
    assert fine_cn["grid_match"].startswith("same grid")
    coarse_ca = next(r for r in rows if "coarse" in r["case"] and r["quantity"] == "ca_alpha0")
    assert float(coarse_ca["iterative_error_pct"]) == pytest.approx(100 * 2e-3 / 0.998, rel=1e-5)
    assert float(coarse_ca["ratio"]) == pytest.approx(100 * 2e-3 / 0.998 / 10, rel=1e-5)
    assert coarse_ca["grid_match"].startswith("different grid")
    assert not any("nomirror" in r["case"] for r in rows)  # not a family member


# ------------------------------------------------------------------ the whole collector


def test_main_refuses_a_missing_work_directory(tmp_path):
    with pytest.raises(FileNotFoundError):
        it.main(out=tmp_path / "x.csv", work=tmp_path / "nowhere")


@pytest.mark.skipif(not it.DEFAULT_WORK.is_dir(), reason="the Task 14 work directory is not on this machine")
def test_main_is_deterministic(tmp_path):
    a, b = tmp_path / "a.csv", tmp_path / "b.csv"
    it.main(out=a)
    it.main(out=b)
    assert a.read_bytes() == b.read_bytes()
    text = a.read_text()
    assert "/home/" not in text  # no machine-specific path
    head = [ln for ln in text.splitlines() if ln.startswith("#")]
    assert any("layer0_plate" in ln for ln in head)  # the layer-0 plate has no iterative measure; the header says so
    assert any("ratio above 1.0" in ln for ln in head)
    rows = list(csv.DictReader(ln for ln in text.splitlines() if not ln.startswith("#")))
    assert rows[0].keys() >= {"case", "mach", "alpha_deg", "model", "quantity", "value_at_stop", "value_converged",
                              "iterative_error_pct", "gci_fine_pct", "ratio", "grid_match", "source", "h_basis", "note"}
    for r in rows:
        if r["gci_fine_pct"] == "":
            assert r["ratio"] == "", r["case"]
        assert r["ratio"] != "0" or float(r["iterative_error_pct"]) == 0.0
