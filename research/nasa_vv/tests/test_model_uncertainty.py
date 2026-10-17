# SPDX-License-Identifier: GPL-3.0-or-later
"""Model uncertainty, Euler against RANS (spec 6.6). Paths resolve from this file, not the cwd.

The GCI inputs are synthetic here, so the separability logic is tested without waiting for the grid studies; the
real committed values are only read where a test says so."""
import csv
import pathlib
import shutil

import pytest

from nasa_vv.cases import model_uncertainty as mu
from nasa_vv.cases.existing_data import COLUMNS as GCI_COLUMNS

RESULTS = pathlib.Path(__file__).resolve().parents[1] / "results"
ROOT = pathlib.Path(__file__).resolve().parents[3]


def gci_row(study, condition, quantity, *, triplet="fine/medium/coarse", h_basis="nodes^-1/3", fine="", coarse="", note=""):
    row = {c: "" for c in GCI_COLUMNS}
    row.update(study=study, condition=condition, quantity=quantity, triplet=triplet, h_basis=h_basis,
               gci_fine_pct=str(fine), gci_coarse_pct=str(coarse), note=note)
    return row


def write_gci(path, rows):
    with open(path, "w", newline="") as f:
        f.write("# synthetic\n")
        w = csv.DictWriter(f, fieldnames=GCI_COLUMNS, lineterminator="\n")
        w.writeheader()
        w.writerows(rows)


def read(path):
    lines = [ln for ln in pathlib.Path(path).read_text().splitlines() if not ln.startswith("#")]
    return list(csv.DictReader(lines))


# ---------------------------------------------------------------- the judgement

@pytest.mark.parametrize("diff, e, r, expected", [
    (5.0, 1.0, 2.0, True),        # 5 > 3
    (-5.0, 1.0, 2.0, True),       # the sign does not matter
    (2.0, 1.0, 2.0, False),       # 2 < 3
    (3.0, 1.0, 2.0, False),       # equal to the sum: not exceeded
    (0.0, 0.0, 0.0, False),       # a zero difference with exact bands is not separable
    (5.0, None, 2.0, None),       # a missing band is not a verdict
    (5.0, 1.0, None, None),
    (5.0, None, None, None),
])
def test_separable_needs_both_bands_and_compares_the_sum(diff, e, r, expected):
    assert mu.separable(diff, e, r) is expected


# ---------------------------------------------------------------- picking a GCI out of the rows

REF = mu.GciRef(study="s", mach=0.8, alpha=4.0, quantity="cn", production="fine")


def test_the_fine_level_uses_the_fine_gci_and_the_middle_level_the_coarse_one():
    rows = [gci_row("s", "M=0.8 alpha=4 (raw)", "cn", fine=1.5, coarse=3.0)]
    assert mu.pick_gci(rows, REF)[0] == pytest.approx(1.5)
    rows = [gci_row("s", "M=0.8 alpha=4 (raw)", "cn", triplet="1.07 mm/1.5 mm/2.1 mm", fine=1.5, coarse=3.0)]
    ref = mu.GciRef("s", 0.8, 4.0, "cn", production="1.50 mm")
    value, note = mu.pick_gci(rows, ref)
    assert value == pytest.approx(3.0) and "gci_coarse_pct" in note


def test_the_coarsest_level_has_no_gci():
    rows = [gci_row("s", "M=0.8 alpha=4", "cn", fine=1.5, coarse=3.0)]
    value, note = mu.pick_gci(rows, mu.GciRef("s", 0.8, 4.0, "cn", production="coarse"))
    assert value is None and "coarsest" in note


def test_a_production_level_that_is_not_in_the_triplet_is_missing_not_guessed():
    rows = [gci_row("s", "M=0.8 alpha=4", "cn", fine=1.5, coarse=3.0)]
    value, note = mu.pick_gci(rows, mu.GciRef("s", 0.8, 4.0, "cn", production="2 mm"))
    assert value is None and "2 mm" in note


def test_the_larger_of_the_two_h_bases_is_used():
    rows = [gci_row("s", "M=0.8 alpha=4", "cn", h_basis="nodes^-1/3", fine=1.0),
            gci_row("s", "M=0.8 alpha=4", "cn", h_basis="wall_size", fine=2.5)]
    value, note = mu.pick_gci(rows, REF)
    assert value == pytest.approx(2.5) and "wall_size" in note


def test_a_refused_row_makes_the_gci_missing_even_if_the_other_basis_has_one():
    rows = [gci_row("s", "M=0.8 alpha=4", "cn", h_basis="nodes^-1/3", fine=1.0),
            gci_row("s", "M=0.8 alpha=4", "cn", h_basis="wall_size", note="refused: level 'x' did not converge [solves: a; b]")]
    value, note = mu.pick_gci(rows, REF)
    assert value is None and "refused" in note and "level 'x' did not converge" in note
    assert "[solves" not in note       # the long per-level detail stays in the GCI file


def test_a_flagged_row_makes_the_gci_missing():
    flag = "observed order p = 8.45 exceeds twice the formal order of 2; the triplet is not in the asymptotic range and this GCI is not reliable"
    rows = [gci_row("s", "M=0.8 alpha=4", "cn", fine=0.03, note=flag)]
    value, note = mu.pick_gci(rows, REF)
    assert value is None and "not reliable" in note


def test_an_oscillatory_row_with_no_gci_is_missing():
    rows = [gci_row("s", "M=0.8 alpha=4", "cn", note="oscillatory convergence: the differences change sign")]
    value, note = mu.pick_gci(rows, REF)
    assert value is None and "oscillatory" in note


def test_no_matching_row_is_missing_and_says_what_was_looked_for():
    rows = [gci_row("other", "M=0.8 alpha=4", "cn", fine=1.0),
            gci_row("s", "M=2 alpha=4", "cn", fine=1.0),
            gci_row("s", "M=0.8 alpha=2", "cn", fine=1.0),
            gci_row("s", "M=0.8 alpha=4", "xcp", fine=1.0)]
    value, note = mu.pick_gci(rows, REF)
    assert value is None and "no row" in note


def test_mach_is_matched_as_a_number():
    rows = [gci_row("s", "M=2 alpha=4 (mirror)", "cn", fine=1.0)]
    assert mu.pick_gci(rows, mu.GciRef("s", 2.0, 4.0, "cn", "fine"))[0] == pytest.approx(1.0)
    assert mu.pick_gci(rows, mu.GciRef("s", 0.2, 4.0, "cn", "fine"))[0] is None


# ---------------------------------------------------------------- the whole table

@pytest.fixture
def empty_results(tmp_path):
    d = tmp_path / "results"
    d.mkdir()
    return d


def rows_of(tmp_path, results):
    return read(mu.main(tmp_path / "mu.csv", results_dir=results))


def find(rows, case, mach, alpha, quantity):
    (r,) = [r for r in rows if r["case"] == case and float(r["mach"]) == mach and float(r["alpha_deg"]) == alpha
            and r["quantity"] == quantity]
    return r


def test_the_columns_are_the_briefs_plus_a_note(tmp_path, empty_results):
    p = mu.main(tmp_path / "mu.csv", results_dir=empty_results)
    lines = [ln for ln in p.read_text().splitlines() if not ln.startswith("#")]
    assert next(csv.reader(lines)) == [
        "case", "mach", "alpha_deg", "quantity", "euler", "rans", "difference", "difference_pct",
        "euler_gci_pct", "rans_gci_pct", "separable", "source", "note"]


def test_the_conditions_are_the_briefs(tmp_path, empty_results):
    rows = rows_of(tmp_path, empty_results)
    got = {(r["case"], float(r["mach"]), float(r["alpha_deg"])) for r in rows}
    assert got == {("sample_rocket", 0.8, 4.0), ("sample_rocket", 2.0, 4.0),
                   ("basic_finner", 1.832, 0.0), ("basic_finner", 1.832, 2.0)}


def test_the_values_are_those_of_the_committed_files_and_the_difference_is_relative_to_euler(tmp_path, empty_results):
    rows = rows_of(tmp_path, empty_results)
    r = find(rows, "sample_rocket", 0.8, 4.0, "cn")
    assert float(r["rans"]) == pytest.approx(1.2621) and float(r["euler"]) == pytest.approx(1.2021)
    assert float(r["difference"]) == pytest.approx(1.2621 - 1.2021)
    assert float(r["difference_pct"]) == pytest.approx(100 * (1.2621 - 1.2021) / 1.2021)
    r = find(rows, "basic_finner", 1.832, 2.0, "cn")
    assert float(r["rans"]) == pytest.approx(0.40013) and float(r["euler"]) == pytest.approx(0.39924)   # flat base rows
    r = find(rows, "basic_finner", 1.832, 0.0, "ca")
    assert float(r["rans"]) == pytest.approx(0.26637 + 0.38134 + 0)
    assert float(r["euler"]) == pytest.approx(0.42037)


def test_a_missing_gci_leaves_the_band_and_the_verdict_blank_and_the_note_names_it(tmp_path, empty_results):
    rows = rows_of(tmp_path, empty_results)
    assert rows
    for r in rows:
        assert r["euler_gci_pct"] == "" and r["rans_gci_pct"] == "" and r["separable"] == ""
        assert "euler GCI missing" in r["note"] and "rans GCI missing" in r["note"]


def test_never_a_verdict_without_both_bands(tmp_path, empty_results):
    write_gci(empty_results / "gci_sample_euler.csv", [
        gci_row("sample_euler", "M=0.8 alpha=4 (raw)", "cna", fine=0.2),
        gci_row("sample_euler", "M=0.8 alpha=4 (combined)", "xcp", fine=0.1)])
    rows = rows_of(tmp_path, empty_results)
    r = find(rows, "sample_rocket", 0.8, 4.0, "cn")
    assert r["euler_gci_pct"] == "0.2" and r["rans_gci_pct"] == "" and r["separable"] == ""
    assert "rans GCI missing" in r["note"] and "euler GCI missing" not in r["note"]
    for r in rows:
        if r["euler_gci_pct"] == "" or r["rans_gci_pct"] == "":
            assert r["separable"] == ""


def test_both_bands_present_gives_true_when_the_difference_exceeds_their_sum(tmp_path, empty_results):
    # sample rocket M = 0.8: cn differs by +4.99 %
    write_gci(empty_results / "gci_sample_euler.csv", [
        gci_row("sample_euler", "M=0.8 alpha=4 (raw)", "cna", fine=1.0)])
    write_gci(empty_results / "gci_existing.csv", [
        gci_row("rans_mesh_study", "M=0.8 alpha=4 (mirror)", "cn", triplet="3 mm/4.8 mm/8 mm", fine=2.0, coarse=9.0)])
    r = find(rows_of(tmp_path, empty_results), "sample_rocket", 0.8, 4.0, "cn")
    assert float(r["euler_gci_pct"]) == 1.0 and float(r["rans_gci_pct"]) == 2.0     # 3 mm is the first (finest) level
    assert r["separable"] == "True"


def test_both_bands_present_gives_false_when_the_bands_cover_the_difference(tmp_path, empty_results):
    write_gci(empty_results / "gci_sample_euler.csv", [
        gci_row("sample_euler", "M=0.8 alpha=4 (raw)", "cna", fine=3.0)])
    write_gci(empty_results / "gci_existing.csv", [
        gci_row("rans_mesh_study", "M=0.8 alpha=4 (mirror)", "cn", triplet="3 mm/4.8 mm/8 mm", fine=2.5)])
    r = find(rows_of(tmp_path, empty_results), "sample_rocket", 0.8, 4.0, "cn")
    assert r["separable"] == "False"      # 4.99 < 3.0 + 2.5


def test_the_basic_finner_rans_value_sits_on_the_medium_level(tmp_path, empty_results):
    write_gci(empty_results / "gci_basic_finner_rans.csv", [
        gci_row("basic_finner_rans", "M=1.832 flat base alpha=2", "cna", triplet="1.07 mm/1.5 mm/2.1 mm", fine=0.1, coarse=0.7)])
    r = find(rows_of(tmp_path, empty_results), "basic_finner", 1.832, 2.0, "cn")
    assert float(r["rans_gci_pct"]) == pytest.approx(0.7)      # wall_size 1.5 mm is the medium level
    assert r["euler_gci_pct"] == "" and r["separable"] == ""
    assert "euler GCI missing" in r["note"]


def test_the_euler_gci_of_the_basic_finner_at_another_condition_is_not_borrowed(tmp_path):
    # the real gci_existing.csv has basic_finner_mesh_check rows (Euler, M = 2.375, fairing and flat bases)
    d = tmp_path / "results"
    d.mkdir()
    shutil.copy(RESULTS / "gci_existing.csv", d / "gci_existing.csv")
    rows = rows_of(tmp_path, d)
    for r in rows:
        if r["case"] == "basic_finner":
            assert r["euler_gci_pct"] == "" and r["separable"] == ""
            assert "M = 2.375 (with fairing and flat bases)" in r["note"]


def test_with_only_the_existing_gcis_every_rans_band_of_the_sample_rocket_is_refused(tmp_path):
    d = tmp_path / "results"
    d.mkdir()
    shutil.copy(RESULTS / "gci_existing.csv", d / "gci_existing.csv")
    rows = rows_of(tmp_path, d)
    for r in rows:
        if r["case"] == "sample_rocket":
            assert r["rans_gci_pct"] == "" and r["separable"] == ""
            assert "refused" in r["note"]


def test_a_refused_triplet_in_a_new_file_stays_blank(tmp_path, empty_results):
    write_gci(empty_results / "gci_basic_finner_rans.csv", [
        gci_row("basic_finner_rans", "M=1.832 flat base alpha=2", "cna", triplet="1.07 mm/1.5 mm/2.1 mm", note="refused: level(s) fine not run; RANS GCI not obtained at M = 1.832")])
    r = find(rows_of(tmp_path, empty_results), "basic_finner", 1.832, 2.0, "cn")
    assert r["rans_gci_pct"] == "" and "RANS GCI not obtained" in r["note"]


# ---------------------------------------------------------------- housekeeping

def test_the_header_names_the_gci_files_that_were_read_and_those_that_were_absent(tmp_path, empty_results):
    write_gci(empty_results / "gci_existing.csv", [])
    text = mu.main(tmp_path / "mu.csv", results_dir=empty_results).read_text()
    head = "\n".join(ln for ln in text.splitlines() if ln.startswith("#"))
    assert "gci_existing.csv (read)" in head
    assert "gci_sample_euler.csv (absent)" in head and "gci_basic_finner_rans.csv (absent)" in head
    assert "doc/data/cfd/rans_vs_euler.csv" in head and "@" in head


def test_running_main_twice_is_byte_identical(tmp_path):
    a = mu.main(tmp_path / "a.csv").read_bytes()
    b = mu.main(tmp_path / "b.csv").read_bytes()
    assert a == b


def test_no_path_depends_on_the_working_directory(tmp_path, monkeypatch):
    here = mu.main(tmp_path / "a.csv").read_bytes()
    elsewhere = tmp_path / "elsewhere"
    elsewhere.mkdir()
    monkeypatch.chdir(elsewhere)
    assert mu.main(tmp_path / "b.csv").read_bytes() == here
