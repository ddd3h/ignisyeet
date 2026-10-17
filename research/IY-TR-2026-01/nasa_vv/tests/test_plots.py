# SPDX-License-Identifier: GPL-3.0-or-later
"""A blank probable error means unknown, and 0 means below 0.5 per cent.

doc/source/validation.rst:580 states the 0 convention. doc/scripts/make_plots.py:1610
uses fillna(0.0) and so draws a zero-length bar for an unknown error; the research
figures must not.

The grid-sensitivity box is drawn only for a point that has a row for the same
quantity, configuration and Mach number in results/gci_existing.csv; rows of other
studies (the V6 rocket, the flat plate) never supply a band for the Basic Finner.
"""
import math

import pytest

from nasa_vv.plots import BAND_RULES, Band, ErrorBar, band_for, error_bar_spec, read_gci_rows


def test_a_blank_probable_error_draws_no_bar():
    assert error_bar_spec("", 18.47) is None
    assert error_bar_spec(None, 18.47) is None
    assert error_bar_spec("  ", 18.47) is None


def test_a_numeric_probable_error_scales_with_the_value():
    b = error_bar_spec(2.0, 18.47)
    assert b.half_height == pytest.approx(18.47 * 0.02)
    assert b.bound is False


def test_a_string_number_is_read_as_a_number():
    assert error_bar_spec("2", 18.47) == ErrorBar(half_height=pytest.approx(18.47 * 0.02), bound=False)
    b = error_bar_spec("0", 18.47)
    assert b.bound is True


def test_zero_means_a_half_per_cent_bound_not_zero_error():
    b = error_bar_spec(0, 18.47)
    assert b.half_height == pytest.approx(18.47 * 0.005)
    assert b.bound is True


def test_a_negative_value_uses_its_magnitude():
    b = error_bar_spec(2.0, -52.648)
    assert b.half_height == pytest.approx(52.648 * 0.02)


@pytest.mark.parametrize("bad", [-1.0, "-0.5", "abc", "2 %", float("nan"), float("inf")])
def test_a_negative_or_non_numeric_probable_error_is_refused(bad):
    with pytest.raises(ValueError):
        error_bar_spec(bad, 18.47)


# ---- band selection ------------------------------------------------------------------------------------

def _row(study, condition, quantity, triplet, basis, gci):
    return {"study": study, "condition": condition, "quantity": quantity, "triplet": triplet,
            "h_basis": basis, "gci_fine_pct": "99", "gci_coarse_pct": gci}


MESH = "basic_finner_mesh_check"
# the file has no Mach column: the Mach number of the study is that of its source CSV
MACHS = {MESH: {2.375}}
CN_ROWS = [
    _row(MESH, "fairing alpha=2", "cn", "L1-L3", "cells^-1/3", "0.03"),
    _row(MESH, "fairing alpha=2", "cn", "L1-L3", "wall_size", "0.04"),
    _row(MESH, "fairing alpha=2", "cn", "L2-L4", "cells^-1/3", ""),   # refused: no number
    _row(MESH, "fairing alpha=2", "cn", "L2-L4", "wall_size", ""),
]


def test_the_band_uses_the_gci_of_the_plotted_grid_not_the_fine_one():
    # _row puts 99 in gci_fine_pct; the plotted run is the medium grid of the L1-L3 triplet
    band = band_for(CN_ROWS, "euler", "CNa", 2.375, study_machs=MACHS)
    assert band.high_pct < 1.0


def test_a_point_with_a_matching_row_gets_the_envelope_over_both_bases_and_triplets():
    rows = CN_ROWS + [
        _row(MESH, "fairing alpha=2", "cn", "L2-L4", "cells^-1/3", "0.5"),
        _row(MESH, "fairing alpha=2", "cn", "L2-L4", "wall_size", "0.2"),
    ]
    band = band_for(rows, "euler", "CNa", 2.375, study_machs=MACHS)
    assert isinstance(band, Band)
    assert band.low_pct == pytest.approx(0.03)
    assert band.high_pct == pytest.approx(0.5)
    assert band.n_rows == 4
    assert band.established is False


def test_blank_gci_rows_contribute_nothing_and_an_all_blank_set_gives_no_band():
    only_blank = [r for r in CN_ROWS if r["gci_coarse_pct"] == ""]
    assert band_for(only_blank, "euler", "CNa", 2.375, study_machs=MACHS) is None
    band = band_for(CN_ROWS, "euler", "CNa", 2.375, study_machs=MACHS)
    assert (band.low_pct, band.high_pct, band.n_rows) == (pytest.approx(0.03), pytest.approx(0.04), 2)


def test_a_point_without_a_matching_row_gets_none():
    # another Mach number, another series, a quantity without a rule
    assert band_for(CN_ROWS, "euler", "CNa", 1.832, study_machs=MACHS) is None
    assert band_for(CN_ROWS, "rans", "CNa", 2.375, study_machs=MACHS) is None
    assert band_for(CN_ROWS, "euler", "CX0", 2.375, study_machs=MACHS) is None
    assert band_for([], "euler", "CNa", 2.375, study_machs=MACHS) is None


def test_rows_of_another_configuration_or_quantity_never_match():
    rows = [
        _row(MESH, "flat alpha=2", "cn", "L2-L4", "wall_size", "0.9"),
        _row(MESH, "fairing alpha=4", "cn", "L2-L4", "wall_size", "0.9"),
        _row(MESH, "fairing alpha=2", "ca_pressure", "L2-L4", "wall_size", "0.9"),
    ]
    assert band_for(rows, "euler", "CNa", 2.375, study_machs=MACHS) is None


def test_v6_and_flat_plate_rows_never_match_even_with_the_same_condition_and_quantity():
    rows = [
        _row("rans_mesh_study", "fairing alpha=2", "cn", "3 mm/4.8 mm/8 mm", "nodes^-1/3", "0.7"),
        _row("rans_flatplate", "fairing alpha=2", "cn", "273x193/137x97/69x49", "cells^-1/2 (2-D)", "0.7"),
    ]
    assert band_for(rows, "euler", "CNa", 2.375, study_machs=MACHS) is None
    # and a study whose Mach number is not on record cannot match
    assert band_for(CN_ROWS, "euler", "CNa", 2.375, study_machs={}) is None


def test_the_committed_gci_file_gives_a_band_for_cna_at_mach_2375_only():
    rows = read_gci_rows()
    band = band_for(rows, "euler", "CNa", 2.375)
    assert band is not None and not band.established
    assert 0 < band.low_pct <= band.high_pct < 1.0
    for mach in (1.077, 1.293, 1.832, 2.718, 3.147, 3.734, 4.3):
        assert band_for(rows, "euler", "CNa", mach) is None
    # CX0 is ca_pressure plus modelled terms, so no ca_pressure GCI is a CX0 GCI
    assert band_for(rows, "euler", "CX0", 2.375) is None
    assert band_for(rows, "rans", "CNa", 1.832) is None


def test_adding_an_established_source_is_a_table_row():
    from dataclasses import replace
    rule = replace(BAND_RULES[0], series="rans", study="future_rans_gci", established=True)
    rows = [_row("future_rans_gci", "fairing alpha=2", "cn", "T", "b", "1.5")]
    band = band_for(rows, "rans", "CNa", 1.832, rules=(rule,), study_machs={"future_rans_gci": {1.832}})
    assert band.established is True and math.isclose(band.high_pct, 1.5)


# ---- observed change between grid levels -----------------------------------------------------------------

def test_the_largest_observed_change_is_pinned_against_mesh_check_csv():
    import csv

    from nasa_vv.plots import MESH_CHECK_CSV, observed_change

    with open(MESH_CHECK_CSV, newline="") as fh:
        rows = list(csv.DictReader(l for l in fh if not l.startswith("#")))
    cn = {r["level"]: float(r["cn"]) for r in rows
          if r["base_treatment"] == "fairing" and float(r["mach"]) == 2.375 and float(r["alpha_deg"]) == 2}
    assert set(cn) == {"L1", "L2", "L3", "L4"}
    ch = observed_change("euler", "CNa", 2.375)
    assert ch.plotted_level == "L2"          # the run plotted in summary_vs_measured.csv (586094 cells)
    assert ch.other_level == "L4"
    assert ch.pct == pytest.approx(abs(cn["L4"] - cn["L2"]) / abs(cn["L2"]) * 100)
    assert 0.46 < ch.pct < 0.48
    assert ch.pct > max(b.high_pct for b in [band_for(read_gci_rows(), "euler", "CNa", 2.375)])


def test_no_observed_change_where_there_is_no_box():
    from nasa_vv.plots import observed_change

    assert observed_change("euler", "CNa", 1.832) is None
    assert observed_change("euler", "CX0", 2.375) is None
    assert observed_change("rans", "CNa", 2.375) is None


# ---- the figure's text: no implementation names, no overstated error bars ----------------------------------

def test_the_validation_footnote_names_no_file_column_or_code():
    from nasa_vv.plots import FOOTNOTE

    for name in ("results/", ".csv", "ca_pressure", "cells^-1/3", "wall_size", "gci_existing"):
        assert name not in FOOTNOTE, name
    assert "excludes round-to-round scatter and systematic error" in FOOTNOTE
    assert "50 % bound" in FOOTNOTE


def test_the_legend_places_the_triangles_beside_the_symbol_not_at_the_bound():
    import re
    from pathlib import Path

    src = Path(__import__("nasa_vv.plots", fromlist=["x"]).__file__).read_text()
    assert "triangles beside the symbol" in src
    assert not re.search(r"triangles at the 0\.5", src)


def test_the_committed_figures_are_what_the_code_draws(tmp_path):
    from nasa_vv.plots import DEFAULT_OUT, LAYER1_OUT, layer1_cp_figure, validation_figure

    validation_figure(tmp_path / "v.svg")
    fig = layer1_cp_figure(tmp_path / "c.svg")
    import matplotlib.pyplot as plt
    plt.close(fig)
    assert (tmp_path / "v.svg").read_bytes() == DEFAULT_OUT.read_bytes()
    assert (tmp_path / "c.svg").read_bytes() == LAYER1_OUT.read_bytes()


# ---- layer 1 Cp figure ------------------------------------------------------------------------------------

def test_layer1_bins_are_read_from_the_committed_results_file():
    from nasa_vv.plots import read_layer1

    levels, exact = read_layer1()
    assert [lv.name for lv in levels] == ["coarse", "medium", "fine"]
    assert exact == pytest.approx(0.0967306, abs=1e-7)
    for lv in levels:
        assert lv.x == pytest.approx((0.25, 0.35, 0.45, 0.55, 0.65, 0.75, 0.85))
        assert len(lv.cp) == len(lv.wall_nodes) == 7
    coarse, medium, fine = levels
    assert coarse.cp[0] == pytest.approx(0.08687) and fine.cp[-1] == pytest.approx(0.09658)
    assert (coarse.converged, medium.converged, fine.converged) == (True, True, False)
    assert (fine.cp_source, fine.source_iteration, fine.history_iterations) == ("periodic snapshot", 200, 210)
    assert fine.residual_drop == pytest.approx(2.442)


def test_layer1_bins_exclude_the_two_means_over_several_bins():
    from nasa_vv.plots import read_layer1

    levels, _ = read_layer1()
    # the 0.2-0.9 and 0.4-0.9 means are not bins: 7 points per level, none at the centre 0.55 of 0.2-0.9
    assert all(len(lv.x) == 7 for lv in levels)


def test_the_layer1_figure_marks_the_fine_level_unaccepted_and_draws_no_band(tmp_path):
    import matplotlib.pyplot as plt

    from nasa_vv.plots import layer1_cp_figure

    fig = layer1_cp_figure(tmp_path / "c.svg")
    try:
        ax = fig.axes[0]
        labels = [t.get_text() for t in ax.get_legend().get_texts()]
        assert any("Taylor-Maccoll" in t for t in labels)
        fine = next(t for t in labels if t.startswith("fine"))
        assert "NOT accepted" in fine and "iteration-200 snapshot" in fine
        assert all("NOT accepted" not in t for t in labels if not t.startswith("fine"))
        assert not ax.patches and not ax.collections          # no band, no fill
        fine_line = next(l for l in ax.lines if l.get_label().startswith("fine"))
        assert fine_line.get_linestyle() == "--" and fine_line.get_markerfacecolor() == "white"
        assert len([l for l in ax.lines if l.get_label().startswith(("coarse", "medium", "fine"))]) == 3
    finally:
        plt.close(fig)


def test_a_layer1_file_with_one_exact_value_per_level_only(tmp_path):
    from nasa_vv.plots import LAYER1_CSV, read_layer1

    lines = LAYER1_CSV.read_text().splitlines(True)
    bad = tmp_path / "l1.csv"
    head = next(i for i, ln in enumerate(lines) if ln.startswith("level,"))
    rows = lines[head + 1:]
    rows[3] = rows[3].replace("0.09673061296", "0.0970")
    bad.write_text("".join(lines[:head + 1] + rows))
    with pytest.raises(ValueError, match="cp_exact"):
        read_layer1(bad)
