# SPDX-License-Identifier: GPL-3.0-or-later
"""Consistency checks (spec 6.3). Paths resolve from this file, not the cwd."""
import csv
import pathlib

import pytest

from nasa_vv.cases import consistency
from nasa_vv.cases.consistency import antisymmetry

ROOT = pathlib.Path(__file__).resolve().parents[3]


def test_a_perfectly_antisymmetric_pair_has_a_zero_even_part():
    assert antisymmetry(1.2021, -1.2021) == pytest.approx(0.0)


def test_the_even_part_is_half_the_sum():
    assert antisymmetry(1.20, -1.18) == pytest.approx(0.01)


def test_the_even_part_is_signed():
    assert antisymmetry(1.18, -1.20) == pytest.approx(-0.01)


def _rows(path):
    lines = [ln for ln in path.read_text().splitlines() if not ln.startswith("#")]
    return list(csv.DictReader(lines))


def test_main_writes_the_expected_columns(tmp_path):
    path = consistency.main(tmp_path / "consistency.csv")
    lines = [ln for ln in path.read_text().splitlines() if not ln.startswith("#")]
    assert next(csv.reader(lines)) == [
        "check", "case", "mach", "alpha_deg", "model", "quantity",
        "value", "expected", "tolerance", "passed", "source"]


def test_every_row_source_names_an_existing_file(tmp_path):
    rows = _rows(consistency.main(tmp_path / "consistency.csv"))
    assert rows
    for row in rows:
        rel = row["source"].split()[0]
        assert (ROOT / rel).exists(), row["source"]


def test_the_dalembert_row_records_the_committed_value_with_no_threshold(tmp_path):
    rows = _rows(consistency.main(tmp_path / "consistency.csv"))
    (row,) = [r for r in rows if r["check"] == "ca_dalembert"]
    assert float(row["value"]) == pytest.approx(-0.0313698460, abs=1e-9)
    assert row["tolerance"] == "" and row["passed"] == ""


def test_running_main_twice_is_byte_identical(tmp_path):
    a = consistency.main(tmp_path / "a.csv").read_bytes()
    b = consistency.main(tmp_path / "b.csv").read_bytes()
    assert a == b


def _header_text(path):
    return "\n".join(ln for ln in path.read_text().splitlines() if ln.startswith("#"))


def _source_table(rel):
    """Read a committed data file with the csv module alone, no helper of consistency.py."""
    lines = [ln for ln in (ROOT / rel).read_text().splitlines() if not ln.startswith("#")]
    return list(csv.DictReader(lines))


def test_cn_zero_alpha_covers_every_configuration_and_mach_of_the_asymmetry_study(tmp_path):
    out = _rows(consistency.main(tmp_path / "consistency.csv"))
    source = _source_table("doc/data/cfd/asymmetry_study.csv")
    assert len(source) == 15
    zero = {(r["case"], float(r["mach"])): r for r in out
            if r["check"] == "cn_zero_alpha" and r["source"].startswith("doc/data/cfd/asymmetry")}
    assert len(zero) == 15
    assert {c for c, _ in zero} == {"base", "roll45", "jst", "tight", "zm"}
    for s in source:
        row = zero[(s["config"], float(s["mach"]))]
        tolerance = 0.01 * abs(float(s["cn_odd_4deg"]))
        assert float(row["tolerance"]) == pytest.approx(tolerance, rel=1e-5)
        assert float(row["value"]) == pytest.approx(float(s["cn0"]), abs=1e-12)
        assert row["expected"] == "0"
        assert row["passed"] == str(abs(float(s["cn0"])) <= tolerance)


def test_cn_zero_alpha_passes_on_the_mirrored_mesh_and_fails_on_the_half_model(tmp_path):
    out = _rows(consistency.main(tmp_path / "consistency.csv"))
    zero = [r for r in out
            if r["check"] == "cn_zero_alpha" and r["source"].startswith("doc/data/cfd/asymmetry")]
    assert {r["passed"] for r in zero if r["case"] == "zm"} == {"True"}
    assert {r["passed"] for r in zero if r["case"] == "base"} == {"False"}
    for r in zero:
        if r["case"] == "zm":
            assert "mesh symmetry" in r["source"] and "by construction" in r["source"]
        else:
            assert "mesh symmetry" not in r["source"]


def test_cn_zero_alpha_has_a_row_for_every_production_mach(tmp_path):
    out = _rows(consistency.main(tmp_path / "consistency.csv"))
    prod = [r for r in out
            if r["check"] == "cn_zero_alpha" and r["source"].startswith("doc/data/cfd/sample_euler")]
    cases = [r for r in _source_table("doc/data/cfd/sample_euler_cases.csv")
             if r["status"] == "converged"]
    zero = {float(r["mach"]): r for r in cases if float(r["alpha_deg"]) == 0.0}
    four = {float(r["mach"]): r for r in cases if float(r["alpha_deg"]) == 4.0}
    assert len(zero) == 4 and set(zero) == set(four)
    assert len(prod) == 4
    for r in prod:
        mach = float(r["mach"])
        assert r["alpha_deg"] == "0" and r["quantity"] == "cn_raw"
        assert float(r["value"]) == float(zero[mach]["cn_raw"])
        assert float(r["tolerance"]) == pytest.approx(0.01 * abs(float(four[mach]["cn_raw"])),
                                                      rel=1e-9)
        assert r["passed"] == str(abs(float(r["value"])) <= float(r["tolerance"]))
        assert "z-symmetric mesh" in r["source"]


def test_the_header_states_the_five_decimal_resolution_of_the_asymmetry_study(tmp_path):
    header = _header_text(consistency.main(tmp_path / "consistency.csv"))
    assert "5 decimals" in header and "5e-6" in header


def test_the_header_says_the_mirrored_antisymmetry_rows_are_near_zero_by_construction(tmp_path):
    path = consistency.main(tmp_path / "consistency.csv")
    assert "by construction" in _header_text(path)
    zm = [r for r in _rows(path) if r["check"] == "cn_antisymmetry" and r["case"] == "zm"]
    assert len(zm) == 3
    for r in zm:
        assert "by construction" in r["source"]


def test_the_header_names_the_basis_of_the_production_tolerance(tmp_path):
    header = _header_text(consistency.main(tmp_path / "consistency.csv"))
    assert "cn_raw at alpha_deg = 4" in header


def test_the_long_note_on_combined_values_is_in_the_header_not_in_each_source(tmp_path):
    path = consistency.main(tmp_path / "consistency.csv")
    for r in _rows(path):
        assert "antisymmetry() is not re-run" not in r["source"]
        assert "already-combined" not in r["source"]
    assert "antisymmetry() is not re-run" in _header_text(path)


def test_the_committed_results_file_is_what_main_writes(tmp_path):
    committed = pathlib.Path(consistency.__file__).resolve().parents[1] / "results" / "consistency.csv"
    assert consistency.main(tmp_path / "consistency.csv").read_bytes() == committed.read_bytes()


# ---------------------------------------------------------------- refined Euler family (Task 14)

LEVEL_COLS = ["level", "mach", "wall_size_mm", "nodes", "status_a0", "status_a4", "status_am4",
              "ca_alpha0", "cn_raw_a0", "cn_raw_a4", "cn_raw_am4"]


def _levels_csv(path, ca=(-0.030, -0.020, -0.010), plus=1.2, minus=-1.19):
    rows = [LEVEL_COLS]
    for name, wall, nodes, c in zip(("coarse", "medium", "fine"), (3.92, 2.8, 2.0),
                                    (400000, 520000, 700000), ca):
        for m in (0.3, 0.8, 2):
            rows.append([name, m, wall, nodes, "converged", "converged", "", c if m == 0.3 else 0.1,
                         0.0, 1.2, ""])
    for m in (0.3, 0.8, 2):
        rows.append(["medium_nomirror", m, 2.8, 520000, "converged", "converged", "converged",
                     0.1, -0.03, plus, minus])
    path.write_text("# header\n" + "\n".join(",".join(map(str, r)) for r in rows) + "\n")
    return path


def test_the_refined_dalembert_rows_are_one_per_level_with_blank_tolerance(tmp_path):
    out = consistency.main(tmp_path / "c.csv", _levels_csv(tmp_path / "l.csv"))
    rows = [r for r in _rows(out) if r["check"] == "ca_dalembert_refined"]
    assert [r["case"] for r in rows] == ["coarse (3.92 mm)", "medium (2.8 mm)", "fine (2 mm)"]
    assert [float(r["value"]) for r in rows] == [-0.03, -0.02, -0.01]
    assert all(r["tolerance"] == "" and r["passed"] == "" and r["mach"] == "0.3"
               and r["alpha_deg"] == "0" and r["expected"] == "0" for r in rows)


def test_the_header_says_whether_ca_shrinks(tmp_path):
    shrink = _header_text(consistency.main(tmp_path / "a.csv", _levels_csv(tmp_path / "l.csv")))
    assert "|CA| shrinks under refinement" in shrink
    grow = _header_text(consistency.main(
        tmp_path / "b.csv", _levels_csv(tmp_path / "m.csv", ca=(-0.01, -0.02, -0.03))))
    assert "|CA| does not shrink under refinement" in grow


def test_the_nonmirrored_antisymmetry_rows(tmp_path):
    out = consistency.main(tmp_path / "c.csv", _levels_csv(tmp_path / "l.csv"))
    rows = [r for r in _rows(out) if r["check"] == "cn_antisymmetry" and r["case"].startswith("medium")]
    assert len(rows) == 3
    for r in rows:
        assert float(r["value"]) == pytest.approx(0.005)
        assert float(r["tolerance"]) == pytest.approx(0.01 * 1.195)
        assert r["passed"] == "True"
        assert "z_mirror_mesh=false" in r["case"]


def test_existing_rows_are_unchanged_by_the_refined_rows(tmp_path):
    committed = [ln for ln in (pathlib.Path(consistency.__file__).resolve().parents[1] / "results"
                               / "consistency.csv").read_text().splitlines() if not ln.startswith("#")]
    new = [ln for ln in consistency.main(tmp_path / "c.csv", _levels_csv(tmp_path / "l.csv"))
           .read_text().splitlines() if not ln.startswith("#")]
    old = [ln for ln in committed if not ln.startswith(("ca_dalembert_refined", "cn_antisymmetry,medium"))]
    assert new[:len(old)] == old
