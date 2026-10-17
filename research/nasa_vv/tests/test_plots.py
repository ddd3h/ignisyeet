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
