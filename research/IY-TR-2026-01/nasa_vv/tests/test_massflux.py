# SPDX-License-Identifier: GPL-3.0-or-later
import math

import pytest

from nasa_vv.cases.massflux import flux_imbalance


def test_a_balanced_domain_has_zero_imbalance():
    assert flux_imbalance(12.5, 12.5) == pytest.approx(0.0)


def test_the_imbalance_is_relative_to_the_inflow():
    assert flux_imbalance(100.0, 99.0) == pytest.approx(0.01)


def test_more_out_than_in_is_still_positive():
    assert flux_imbalance(100.0, 101.0) == pytest.approx(0.01)


def test_zero_inflow_is_an_error():
    with pytest.raises(ValueError):
        flux_imbalance(0.0, 1.0)


@pytest.mark.parametrize("inflow, outflow", [
    (math.nan, 1.0), (1.0, math.nan), (math.nan, math.nan), (math.nan, 0.0), (0.0, math.nan)])
def test_nan_in_either_argument_is_an_error(inflow, outflow):
    with pytest.raises(ValueError):
        flux_imbalance(inflow, outflow)


# ---------------------------------------------------------------- the collector, on synthetic histories

import csv  # noqa: E402
import pathlib  # noqa: E402

from nasa_vv.cases import layer0 as L  # noqa: E402
from nasa_vv.cases import massflux as M  # noqa: E402


def test_inflow_and_outflow_are_the_positive_and_the_negative_markers():
    inflow, outflow = M.split_fluxes({"inlet": 10.0, "outlet": -9.0, "top": -1.5, "wall": 0.0, "side": 0.25})
    assert (inflow, outflow) == (10.25, 10.5)


def test_a_nan_marker_flux_is_an_error():
    with pytest.raises(ValueError):
        M.split_fluxes({"inlet": 1.0, "top": math.nan})


def test_every_boundary_marker_is_analysed():
    assert M.analysed_markers(L.CASES["wedge"]) == ["wall", "inlet", "outlet", "top"]
    assert M.analysed_markers(L.CASES["cone_sym_axis"]) == ["axis", "cone", "inlet", "outlet", "top"]


def test_the_analytic_inflow_of_the_cone_is_the_full_revolution():
    rho_u, extent = M.freestream_flux(L.CASES["cone"])
    assert extent == pytest.approx(1.0)
    assert M.analytic_inflow(L.CASES["cone"]) == pytest.approx(math.pi * rho_u)
    rho_u, extent = M.freestream_flux(L.CASES["wedge"])
    assert M.analytic_inflow(L.CASES["wedge"]) == pytest.approx(rho_u)
    assert rho_u == pytest.approx(1042.15, abs=0.01)     # the value in Task 10's wedge histories


def _write_history(path, fluxes, rms_start=-1.0, rms_end=-14.5, iterations=300):
    path.parent.mkdir(parents=True, exist_ok=True)
    names = list(fluxes)
    head = ['"Time_Iter"', '"Inner_Iter"', '"rms[Rho]"', '"Avg_Massflow"'] + [f'"Avg_Massflow({m})"' for m in names]
    rows = [[0, 0, rms_start, 0.0] + [fluxes[m] for m in names],
            [0, iterations, rms_end, 0.0] + [fluxes[m] for m in names]]
    with path.open("w", newline="") as f:
        w = csv.writer(f)
        w.writerow(head)
        w.writerows(rows)


def _synthetic_work(work, leak=1e-4, top=0.0):
    for name, case in L.CASES.items():
        inflow = M.analytic_inflow(case)
        for lv in L.LEVELS:
            fl = {m: 0.0 for m in M.analysed_markers(case)}
            fl["inlet"], fl["outlet"], fl["top"] = inflow, -inflow * (1.0 - leak / (lv + 1)), top
            _write_history(work / name / L.level_name(lv) / "history.csv", fl)
    return work


def test_collect_reads_every_case_and_level(tmp_path):
    rows = M.collect(_synthetic_work(tmp_path / "w"))
    assert [(r["case"], r["level"]) for r in rows] == [(n, L.level_name(lv)) for n in L.CASES for lv in L.LEVELS]
    first = rows[0]
    assert float(first["imbalance_rel"]) == pytest.approx(1e-4, rel=1e-3)
    assert first["resolved"] == "True" and first["stop_reason"] == "residual target"
    assert first["source"] == "wedge/65x49/history.csv (wedge 65x49)"
    assert first["method"] == M.METHOD
    assert "inlet=" in first["marker_fluxes_kg_s"] and "top=0" in first["marker_fluxes_kg_s"]


def test_flux_through_the_top_enters_the_balance_and_inlet_outlet_alone_does_not_see_it(tmp_path):
    rows = M.collect(_synthetic_work(tmp_path / "w", leak=0.0, top=-5.0))
    r = rows[0]
    assert float(r["imbalance_rel"]) == pytest.approx(5.0 / M.analytic_inflow(L.CASES["wedge"]), rel=1e-3)
    assert r["imbalance_inlet_outlet_rel"] == "<1e-09"
    assert r["resolved"] == "True"


def test_a_missing_marker_column_is_an_error(tmp_path):
    work = _synthetic_work(tmp_path / "w")
    _write_history(work / "wedge" / "65x49" / "history.csv", {"inlet": 1.0, "outlet": -1.0})
    with pytest.raises(RuntimeError, match="no flux column"):
        M.collect(work)


def test_the_reference_runs_must_agree_on_inlet_outlet_and_iterations(tmp_path):
    work = _synthetic_work(tmp_path / "w")
    ref = _synthetic_work(tmp_path / "ref")
    assert len(M.collect(work, ref)) == 12
    _write_history(ref / "cone" / "129x97" / "history.csv",
                   {"inlet": M.analytic_inflow(L.CASES["cone"]), "outlet": -1.0})
    with pytest.raises(RuntimeError, match="differ"):
        M.collect(work, ref)


def test_the_csv_is_byte_identical_when_collected_twice(tmp_path):
    work = _synthetic_work(tmp_path / "w")
    a = M.write_csv(M.collect(work), work, tmp_path / "a.csv").read_bytes()
    b = M.write_csv(M.collect(work), work, tmp_path / "b.csv").read_bytes()
    assert a == b
    text = a.decode()
    assert text.startswith("# ") and "last commit that changed it:" in text
    assert "Sign convention" in text and f"Work directory: {work}," in text.replace("~", str(pathlib.Path.home()), 1)
    body = [ln for ln in text.splitlines() if not ln.startswith("#")]
    assert body[0].split(",")[:8] == M.COLUMNS[:8]
    assert len(body) == 13


def test_main_refuses_a_work_directory_under_tmp():
    with pytest.raises(SystemExit):
        M.main(["--work", "/tmp/nasa_vv_massflux_refused"])


def test_the_header_shows_a_work_directory_under_home_with_a_tilde(tmp_path, monkeypatch):
    home = tmp_path / "home"
    work = _synthetic_work(home / ".cache" / "w")
    monkeypatch.setattr(pathlib.Path, "home", classmethod(lambda cls: home))
    text = M.write_csv(M.collect(work), work, tmp_path / "o.csv").read_text()
    assert "Work directory: ~/.cache/w," in text


def test_an_unresolved_imbalance_is_written_as_a_bound_and_a_resolved_one_as_a_number(tmp_path):
    work = _synthetic_work(tmp_path / "w", leak=0.0)
    r = M.collect(work)[0]
    assert (r["imbalance_rel"], r["imbalance_inlet_outlet_rel"], r["resolved"]) == ("<1e-09", "<1e-09", "False")
    assert M.shown(1e-4) == "0.0001" and M.shown(1e-9) == "<1e-09"
    text = M.write_csv(M.collect(work), work, tmp_path / "o.csv").read_text()
    assert "are written as '<1e-09'" in text
