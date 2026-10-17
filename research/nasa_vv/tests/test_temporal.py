# SPDX-License-Identifier: GPL-3.0-or-later
"""Temporal convergence of the flight integrator (spec 6.5).

The runs are synthetic: small files written to tmp_path in the formats the product
writes (summary.json, trajectory.csv), with values that follow f0 + C dt^p.
"""
import csv
import json
import pathlib

import pytest

from nasa_vv.cases import temporal

# summary.json of `ignisyeet sim examples/sample.toml --set sim.dt=0.004`, parachute flight
# (the keys the collector does not read are kept so the file has the real shape).
REAL_SUMMARY = {
    "apogee": 4440.467333643025, "apogee_east": -630.090965166001, "apogee_north": 358.9674389704263,
    "apogee_time": 26.101127429246667, "burnout_time": 3.1, "descent": "parachute",
    "drogue_deploy_time": 27.101127429246667, "landed": True,
    "landing_distance": 7735.510004665515, "landing_east": -655.567330850025,
    "landing_north": -7707.681007086532, "landing_speed": 7.059229506046819,
    "landing_time": 684.953678402196, "max_time": 3600.0, "rail_exit_time": 0.2471570010032786,
    "warnings": [],
}
TRAJ_HEADER = ("t,phase,east,north,up,lat,lon,alt,vel_e,vel_n,vel_u,airspeed,mach,alpha_deg,thrust,"
               "mass,x_cg,x_cp,stability_cal,dyn_pressure,cn,ca,pitch_deg,heading_deg,wx,wy,wz,"
               "beta_deg,roll_rate,cl,cy")


def _row(t, alt, speed):
    # the formats of the real file: t 4 decimals, alt 3, airspeed 4
    cells = ["0"] * 31
    cells[0], cells[7], cells[11] = f"{t:.4f}", f"{alt:.3f}", f"{speed:.4f}"
    return ",".join(cells)


def _traj(path, rows):
    path.mkdir(parents=True, exist_ok=True)
    (path / "trajectory.csv").write_text(TRAJ_HEADER + "\n" + "\n".join(_row(*r) for r in rows) + "\n")


def _summary(path, **values):
    path.mkdir(parents=True, exist_ok=True)
    (path / "summary.json").write_text(json.dumps({**REAL_SUMMARY, **values}, indent=2))


def make_workdir(root, case="sample", p=4.0, fn=None, alt_c=1e10):
    """Runs whose every quantity is f0 + C dt^p; fn(name, dt) overrides a value."""
    ev0 = {"apogee": 4440.0, "apogee_time": 26.0, "landing_distance": 7735.0, "landing_time": 684.0}
    ev_c = {"apogee": 5e7, "apogee_time": 1e5, "landing_distance": 3e8, "landing_time": 4e6}
    for dt in temporal.DTS:
        base = {k: ev0[k] + ev_c[k] * dt**p for k in ev0}
        if fn:
            base = {k: fn(k, dt, v) for k, v in base.items()}
        d = root / f"{case}_dt{dt:g}"
        _summary(d, **base)
        _traj(d, [(0.2, 3.07, 35.65), (1.0, 3000.0 + alt_c * dt**p, 200.0 + 1e9 * dt**p), (1.05, 3.0, 4.0)])
        d10 = root / f"{case}_dt{dt:g}_t10"
        _summary(d10, landed=False)
        _traj(d10, [(9.9, 1.0, 1.0), (10.0, 3050.0 + alt_c * dt**p, 199.0 + 1e9 * dt**p)])
    return root


# ------------------------------------------------------------------ readers

def test_read_summary_takes_the_four_event_quantities(tmp_path):
    _summary(tmp_path)
    s = temporal.read_summary(tmp_path)
    assert s == {"apogee": 4440.467333643025, "apogee_time": 26.101127429246667,
                 "landing_distance": 7735.510004665515, "landing_time": 684.953678402196}


def test_read_summary_refuses_a_flight_that_did_not_land(tmp_path):
    _summary(tmp_path, landed=False)
    with pytest.raises(ValueError, match="did not land"):
        temporal.read_summary(tmp_path)


def test_read_summary_names_a_missing_key(tmp_path):
    _summary(tmp_path)
    d = json.loads((tmp_path / "summary.json").read_text())
    del d["landing_distance"]
    (tmp_path / "summary.json").write_text(json.dumps(d))
    with pytest.raises(ValueError, match="landing_distance"):
        temporal.read_summary(tmp_path)


def test_read_state_at_finds_the_row_of_the_time(tmp_path):
    _traj(tmp_path, [(0.2, 3.07, 35.65), (1.0, 99.323, 206.5436), (1.05, 110.0, 217.0)])
    assert temporal.read_state_at(tmp_path, 1.0) == {"alt": 99.323, "airspeed": 206.5436}


def test_read_state_at_refuses_a_time_that_is_not_in_the_file(tmp_path):
    _traj(tmp_path, [(0.2, 3.07, 35.65), (1.05, 110.0, 217.0)])
    with pytest.raises(ValueError, match="t = 1"):
        temporal.read_state_at(tmp_path, 1.0)


# ------------------------------------------------------------------ thrust curve

def test_thrust_times_read_the_samples_of_an_eng_file(tmp_path):
    eng = tmp_path / "m.eng"
    eng.write_text("; comment\nIY 75 500 0 3.0 4.5 X\n0.00 0\n0.05 2300\n0.20 2400\n3.10 0\n")
    assert temporal.thrust_times(eng) == [0.0, 0.05, 0.2, 3.1]


def test_dividing_steps_are_told_from_the_others():
    bad = temporal.non_dividing([0.0, 0.05, 0.2, 1.0, 2.9], (0.008, 0.004, 0.002, 0.001))
    assert bad == {0.008: [0.05, 2.9], 0.004: [0.05], 0.002: [], 0.001: []}


def test_the_lift_off_motor_has_thrust_from_the_first_instant(tmp_path):
    src = tmp_path / "m.eng"
    src.write_text("; c\nIY 75 500 0 3.0 4.5 X\n0.00 0\n0.05 2300\n3.10 0\n")
    out = temporal.write_noramp_motor(src, tmp_path / "n.eng")
    assert out.read_text().splitlines()[2:] == ["0.00 2300", "0.05 2300", "3.10 0"]


# ------------------------------------------------------------------ collector

def _parse(path):
    lines = path.read_text().splitlines()
    head = [ln for ln in lines if ln.startswith("#")]
    body = [ln for ln in lines if not ln.startswith("#")]
    return head, list(csv.DictReader(body))


def test_main_writes_the_exact_columns_after_a_provenance_header(tmp_path):
    work = make_workdir(tmp_path / "w")
    out = temporal.main(tmp_path / "t.csv", workdir=work)
    lines = out.read_text().splitlines()
    first_data = next(i for i, ln in enumerate(lines) if not ln.startswith("#"))
    assert first_data > 0 and all(ln.startswith("#") for ln in lines[:first_data])
    assert lines[first_data] == ("quantity,dt1,dt2,dt3,f1,f2,f3,order,extrapolated,"
                                 "gci_fine_pct,asymptotic_ratio,note")
    assert temporal.COLUMNS == lines[first_data].split(",")


def test_the_header_says_the_cfd_is_steady(tmp_path):
    work = make_workdir(tmp_path / "w")
    head, _ = _parse(temporal.main(tmp_path / "t.csv", workdir=work))
    text = "\n".join(head)
    assert "steady" in text and "6-DOF" in text and "does not apply" in text


def test_a_fourth_order_run_gives_order_four_on_every_quantity_and_triplet(tmp_path):
    work = make_workdir(tmp_path / "w")
    _, rows = _parse(temporal.main(tmp_path / "t.csv", workdir=work))
    names = {r["quantity"] for r in rows if not r["quantity"].startswith("noramp_")}
    assert names == {"apogee_m", "apogee_time_s", "landing_distance_m", "flight_time_s",
                     "alt_m_at_t1", "airspeed_ms_at_t1", "alt_m_at_t10", "airspeed_ms_at_t10"}
    for r in rows:
        assert float(r["order"]) == pytest.approx(4.0, abs=0.15), r["quantity"]


def test_the_fine_and_the_coarse_triplet_are_both_reported(tmp_path):
    work = make_workdir(tmp_path / "w")
    _, rows = _parse(temporal.main(tmp_path / "t.csv", workdir=work))
    ap = [r for r in rows if r["quantity"] == "apogee_m"]
    assert [(r["dt1"], r["dt2"], r["dt3"]) for r in ap] == [("0.001", "0.002", "0.004"),
                                                          ("0.002", "0.004", "0.008")]
    # f1 belongs to dt1, the finest of the triplet
    assert float(ap[0]["f1"]) < float(ap[0]["f2"]) < float(ap[0]["f3"])


def test_the_extrapolated_value_is_the_dt_to_zero_limit(tmp_path):
    work = make_workdir(tmp_path / "w")
    _, rows = _parse(temporal.main(tmp_path / "t.csv", workdir=work))
    r = next(r for r in rows if r["quantity"] == "apogee_m")
    assert float(r["extrapolated"]) == pytest.approx(4440.0, abs=1e-6)
    assert float(r["asymptotic_ratio"]) == pytest.approx(1.0, abs=0.05)


def test_an_oscillating_sequence_carries_the_refusal_and_no_numbers(tmp_path):
    # the finest step is not the closest to the limit: the differences change sign
    def wobble(name, dt, v):
        if name != "apogee":
            return v
        return 4440.0 + {0.008: 0.0183, 0.004: -0.0004, 0.002: 0.0087, 0.001: 0.0073}[dt]

    work = make_workdir(tmp_path / "w", fn=wobble)
    _, rows = _parse(temporal.main(tmp_path / "t.csv", workdir=work))
    for r in (r for r in rows if r["quantity"] == "apogee_m"):
        assert r["order"] == "" and r["extrapolated"] == "" and r["gci_fine_pct"] == ""
        assert "differences change sign" in r["note"]


def test_a_difference_at_round_off_is_said_and_refused(tmp_path):
    def same(name, dt, v):
        return 4440.0 if name == "apogee" else v

    work = make_workdir(tmp_path / "w", fn=same)
    _, rows = _parse(temporal.main(tmp_path / "t.csv", workdir=work))
    r = next(r for r in rows if r["quantity"] == "apogee_m")
    assert r["order"] == ""
    assert "no change between two of the grids" in r["note"]
    assert "round-off" in r["note"]


def test_a_difference_at_the_output_resolution_is_flagged(tmp_path):
    # the altitude differences of the finest pair are 2e-3 m, two units of the 3 decimals of trajectory.csv
    work = make_workdir(tmp_path / "w", p=4.0, alt_c=1e8)
    _, rows = _parse(temporal.main(tmp_path / "t.csv", workdir=work))
    fine = next(r for r in rows if r["quantity"] == "alt_m_at_t10" and r["dt1"] == "0.001")
    assert "output resolution" in fine["note"]


def test_a_second_order_run_is_reported_as_it_is(tmp_path):
    work = make_workdir(tmp_path / "w", p=2.0)
    _, rows = _parse(temporal.main(tmp_path / "t.csv", workdir=work))
    r = next(r for r in rows if r["quantity"] == "apogee_m")
    assert float(r["order"]) == pytest.approx(2.0, abs=0.01)
    assert "formal order" in r["note"]


def test_the_control_runs_are_labelled_when_present(tmp_path):
    work = make_workdir(tmp_path / "w")
    make_workdir(work, case="noramp")
    _, rows = _parse(temporal.main(tmp_path / "t.csv", workdir=work))
    assert any(r["quantity"] == "noramp_apogee_m" for r in rows)
    assert len([r for r in rows if r["quantity"].startswith("noramp_")]) == 16


def test_a_missing_run_is_an_error_naming_the_directory(tmp_path):
    work = make_workdir(tmp_path / "w")
    for f in (work / "sample_dt0.002").iterdir():
        f.unlink()
    (work / "sample_dt0.002").rmdir()
    with pytest.raises(FileNotFoundError, match="sample_dt0.002"):
        temporal.main(tmp_path / "t.csv", workdir=work)


def test_the_collector_is_deterministic(tmp_path):
    work = make_workdir(tmp_path / "w")
    make_workdir(work, case="noramp")
    a = temporal.main(tmp_path / "a.csv", workdir=work).read_bytes()
    b = temporal.main(tmp_path / "b.csv", workdir=work).read_bytes()
    assert a == b


def test_the_default_paths_do_not_depend_on_the_working_directory():
    assert temporal.RESULTS.is_absolute() and temporal.RESULTS.name == "temporal.csv"
    assert temporal.ROOT.joinpath("examples", "sample.toml").is_file()
    assert temporal.ROOT.joinpath("examples", "sample_motor.eng").is_file()


def test_a_fixed_time_difference_lost_in_the_file_rounding_is_not_called_round_off(tmp_path):
    # alt differences far below the 3 decimals of trajectory.csv: the file shows no change
    work = make_workdir(tmp_path / "w", alt_c=1e3)
    _, rows = _parse(temporal.main(tmp_path / "t.csv", workdir=work))
    r = next(r for r in rows if r["quantity"] == "alt_m_at_t10" and r["dt1"] == "0.001")
    assert r["order"] == ""
    assert "output resolution" in r["note"] and "round-off" not in r["note"]
