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
    assert next(csv.reader(lines)) == consistency.COLUMNS


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
