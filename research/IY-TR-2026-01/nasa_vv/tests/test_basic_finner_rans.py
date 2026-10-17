# SPDX-License-Identifier: GPL-3.0-or-later
"""The Basic Finner RANS family: case setup, collection, the CNa formula, the refusal path."""
import math
import pathlib
import tomllib

from nasa_vv.cases import basic_finner_rans as bf

ROOT = pathlib.Path(__file__).resolve().parents[4]


def test_setup_writes_one_self_contained_case_per_level(tmp_path):
    bf.setup(tmp_path)
    for name, wall in (("coarse", 0.0021), ("medium", 0.0015), ("fine", 0.00107)):
        d = tmp_path / name
        cfg = tomllib.loads((d / "case.toml").read_text())
        cfd = cfg["aero"]["cfd"]
        assert cfg["aero"]["method"] == "cfd" and cfd["model"] == "rans"
        assert cfd["machs"] == [1.832] and cfd["alphas_deg"] == [0.0, 2.0]
        assert cfd["wall_size"] == wall
        # fin_size is left at "auto" (= wall_size for RANS): a smaller fin_size crashes gmsh
        assert "fin_size" not in cfd
        # geometry, CG and nose come from examples/basic_finner/basic_finner.toml
        src = tomllib.loads((ROOT / "examples/basic_finner/basic_finner.toml").read_text())
        for k in ("cg_dry", "nose_direction", "stl_scale", "stl"):
            assert cfg["rocket"][k] == src["rocket"][k]
        assert cfd["surface"] == "step" and cfd["step_file"] == "basic_finner.step"
        for f in ("basic_finner.step", "basic_finner.stl", "sample_motor.eng"):
            assert (d / f).is_file()
        assert cfg["motor"]["eng"] == "sample_motor.eng"


# ---------------------------------------------------------------- collection, on a synthetic work directory

import csv  # noqa: E402
import json  # noqa: E402

import pytest  # noqa: E402


def _history(path, drop_rho, drop_nu, n=400):
    """n rows, iteration 0 .. n-1; the residuals end ``drop`` orders below the reference peak."""
    rows = ['"Inner_Iter",    "rms[Rho]"    ,     "rms[nu]"    ']
    for i in range(n - 1):
        rows.append(f"{i}, {-2.0}, {-6.0}")
    rows.append(f"{n - 1}, {-2.0 - drop_rho}, {-6.0 - drop_nu}")
    path.write_text("\n".join(rows) + "\n")


def make_level(work, name, nodes, cells, cn2, xcp_m, ca_wall, ca_base, *, converged=(True, True),
               drops=(5.0, 5.0)):
    """A fetched run directory with the files the collector reads, for alpha 0 and 2."""
    out = work / name / "out"
    cases = out / "cfd"
    out.mkdir(parents=True)
    (work / name / "done.json").write_text(json.dumps(
        {"state": "done", "exit_code": 0, "wall_seconds": 100.0, "node": "virgoXX", "ppn": 32,
         "binary": "ignisyeet-glibc-a8dff9c", "ignisyeet_commit": "2f31ab9"}))
    (work / name / "job.log").write_text(
        "binary: /home/x/bin/ignisyeet-glibc-a8dff9c\n+ /home/x/bin/ignisyeet-glibc-a8dff9c --no-progress aero case.toml --force\n")
    rep = {"mesh_nodes": nodes, "mesh_cells": cells, "ranks_per_case": 16, "parallel_cases": 2,
           "axial_breakdown": [
               {"mach": 1.832, "alpha_deg": 0.0, "cfd_wall_ca": ca_wall, "cfd_base_ca": ca_base},
               {"mach": 1.832, "alpha_deg": 2.0, "cfd_wall_ca": ca_wall, "cfd_base_ca": ca_base}]}
    (out / "cfd_report.json").write_text(json.dumps(rep))
    lines = ["mach,alpha_deg,status,iterations,cn_raw,xcp_raw_m,wall_seconds,residual_drop,rans_verdict"]
    for alpha, ok, drop in ((0.0, converged[0], drops[0]), (2.0, converged[1], drops[1])):
        status = "converged" if ok else "unconverged"
        lines.append(f"1.832,{alpha:g},{status},400,{cn2 if alpha else 1e-4},{xcp_m if alpha else 0.4},10.0,{drop + 0.5},{'converged' if ok else 'stalled'}")
        d = cases / f"m1.832_a{alpha:05.2f}"
        d.mkdir(parents=True)
        (d / "done.json").write_text(json.dumps(
            {"mach": 1.832, "alpha_deg": alpha, "converged": ok, "iterations": 400, "limit_cycle": not ok,
             "reference_peak": {"rho": -2.0, "nu": -6.0}, "yplus_p95": 1.0}))
        _history(d / "history.csv", drop, drop)
    (out / "cfd_cases.csv").write_text("\n".join(lines) + "\n")


def make_work(work, **bad):
    # each quantity approaches its limit with halving steps as the grid is refined
    for i, (name, nodes, cells) in enumerate((("coarse", 330000, 810000), ("medium", 560000, 1300000),
                                              ("fine", 1100000, 2700000))):
        kw = bad.get(name, {})
        e = 0.5 ** i
        make_level(work, name, nodes, cells, 0.410 - 0.010 * e, 0.2360 - 0.002 * e, 0.2650 - 0.003 * e,
                   0.3710 - 0.004 * e, **kw)


def test_cna_is_cn_over_two_degrees_in_radians():
    assert bf.cna(0.40013) == pytest.approx(0.40013 / math.radians(2.0))
    assert bf.cna(0.40013) == pytest.approx(11.463, abs=1e-3)  # doc/data/validation/basic_finner/cfd_rans.csv
    assert bf.cna(1.0, 4.0) == pytest.approx(1.0 / math.radians(4.0))


def test_residual_drop_is_measured_below_the_reference_peak(tmp_path):
    make_work(tmp_path)
    d = tmp_path / "coarse/out/cfd/m1.832_a02.00"
    done = json.loads((d / "done.json").read_text())
    assert bf.residual_drops(d, done) == (pytest.approx(5.0), pytest.approx(5.0))


def test_levels_and_gci_files_of_a_converged_family(tmp_path):
    make_work(tmp_path)
    lv, gci = bf.main(tmp_path, tmp_path / "l.csv", tmp_path / "g.csv")
    rows = list(csv.DictReader(l for l in gci.read_text().splitlines() if not l.startswith("#")))
    assert {r["quantity"] for r in rows} == {"cna", "xcp", "ca_alpha0", "ca_wall_alpha0", "ca_base_alpha0"}
    assert {r["h_basis"] for r in rows} == {"nodes^-1/3", "wall_size"}
    cna_rows = [r for r in rows if r["quantity"] == "cna"]
    assert len(cna_rows) == 2 and all(r["study"] == "basic_finner_rans" for r in cna_rows)
    assert float(cna_rows[0]["f1"]) == pytest.approx(bf.cna(0.410 - 0.010 * 0.25))  # finest first
    assert cna_rows[0]["gci_fine_pct"] != ""
    assert cna_rows[0]["n1"] == "1100000"
    text = lv.read_text()
    assert "RANS GCI not obtained" not in text.split("level,")[1]  # no note on the data rows


def test_a_level_that_fails_the_gate_gets_no_gci_and_its_drop_is_still_recorded(tmp_path):
    make_work(tmp_path, coarse={"converged": (False, True), "drops": (1.3, 5.0)})
    lv, gci = bf.main(tmp_path, tmp_path / "l.csv", tmp_path / "g.csv")
    grows = list(csv.DictReader(l for l in gci.read_text().splitlines() if not l.startswith("#")))
    a0 = [r for r in grows if r["quantity"] == "ca_alpha0"]
    assert all(r["note"].startswith("refused:") and "RANS GCI not obtained at M = 1.832" in r["note"]
               and r["order"] == "" and r["gci_fine_pct"] == "" for r in a0)
    a2 = [r for r in grows if r["quantity"] == "cna"]
    assert all(r["gci_fine_pct"] != "" for r in a2)  # the alpha 2 solves converged: independent family
    lrows = list(csv.DictReader(l for l in lv.read_text().splitlines() if not l.startswith("#")))
    bad = next(r for r in lrows if r["level"] == "2.1 mm" and r["alpha_deg"] == "0")
    assert bad["residual_drop_orders"] == "1.3" and bad["solver_converged"] == "False"
    assert "RANS GCI not obtained at M = 1.832" in bad["note"]


def test_missing_levels_are_refused_not_guessed(tmp_path):
    make_level(tmp_path, "coarse", 330000, 810000, 0.4, 0.234, 0.262, 0.367)
    _, gci = bf.main(tmp_path, tmp_path / "l.csv", tmp_path / "g.csv")
    rows = list(csv.DictReader(l for l in gci.read_text().splitlines() if not l.startswith("#")))
    assert rows and all("medium, fine not run" in r["note"] and r["order"] == "" for r in rows)


def test_an_unknown_case_status_is_an_error(tmp_path):
    make_work(tmp_path)
    p = tmp_path / "coarse/out/cfd_cases.csv"
    p.write_text(p.read_text().replace("unconverged", "mystery").replace("1.832,2,converged", "1.832,2,mystery"))
    with pytest.raises(ValueError, match="mystery"):
        bf.read_level(tmp_path / "coarse")


def test_status_and_done_json_must_agree(tmp_path):
    make_work(tmp_path)
    p = tmp_path / "coarse/out/cfd/m1.832_a02.00/done.json"
    d = json.loads(p.read_text())
    d["converged"] = False
    p.write_text(json.dumps(d))
    with pytest.raises(ValueError, match="disagree"):
        bf.read_level(tmp_path / "coarse")


def test_the_collector_is_deterministic(tmp_path):
    make_work(tmp_path)
    a = bf.main(tmp_path, tmp_path / "l1.csv", tmp_path / "g1.csv")
    b = bf.main(tmp_path, tmp_path / "l2.csv", tmp_path / "g2.csv")
    assert a[0].read_bytes() == b[0].read_bytes()
    assert a[1].read_bytes() == b[1].read_bytes()


def test_paths_do_not_depend_on_the_working_directory(tmp_path, monkeypatch):
    monkeypatch.chdir(tmp_path)
    assert bf.SOURCE_TOML.is_file() and bf.RESULTS_DIR.is_dir()


# ---------------------------------------------------------------- completeness of the source files


def _truncate(path, n):
    path.write_bytes(path.read_bytes()[:-n])


@pytest.mark.parametrize("cut", [1, 7, 40])
def test_a_history_cut_off_mid_row_refuses_the_level(tmp_path, cut):
    make_work(tmp_path)
    _truncate(tmp_path / "medium/out/cfd/m1.832_a02.00/history.csv", cut)
    with pytest.raises(ValueError):
        bf.read_level(tmp_path / "medium")
    lv, gci = bf.main(tmp_path, tmp_path / "l.csv", tmp_path / "g.csv")
    rows = list(csv.DictReader(l for l in gci.read_text().splitlines() if not l.startswith("#")))
    assert all(r["order"] == "" and "medium" in r["note"] and "source files refused" in r["note"] for r in rows)
    text = lv.read_text()
    assert "source files refused" in text and "RANS GCI not obtained at M = 1.832" in text


def test_a_history_that_stops_early_is_not_the_run_it_claims_to_be(tmp_path):
    make_work(tmp_path)
    p = tmp_path / "fine/out/cfd/m1.832_a00.00/history.csv"
    lines = p.read_text().splitlines(keepends=True)
    p.write_text("".join(lines[:-1]))  # complete rows, but one fewer than the reported iterations
    with pytest.raises(ValueError, match="rows but the case reports"):
        bf.read_level(tmp_path / "fine")


def test_a_cut_off_cases_table_is_refused(tmp_path):
    make_work(tmp_path)
    _truncate(tmp_path / "coarse/out/cfd_cases.csv", 6)
    with pytest.raises(ValueError, match="cut off|shorter"):
        bf.read_level(tmp_path / "coarse")
    p = tmp_path / "coarse/out/cfd_cases.csv"
    p.write_text(p.read_text()[:-3] + "\n")
    with pytest.raises(ValueError):
        bf.read_level(tmp_path / "coarse")


def test_done_json_and_cases_table_must_report_the_same_iterations(tmp_path):
    make_work(tmp_path)
    p = tmp_path / "coarse/out/cfd/m1.832_a02.00/done.json"
    d = json.loads(p.read_text())
    d["iterations"] = 399
    p.write_text(json.dumps(d))
    with pytest.raises(ValueError, match="iterations"):
        bf.read_level(tmp_path / "coarse")


def test_an_incomplete_report_is_refused(tmp_path):
    make_work(tmp_path)
    (tmp_path / "coarse/out/cfd_report.json").write_text(json.dumps({"mesh_nodes": 1}))
    with pytest.raises(ValueError, match="incomplete"):
        bf.read_level(tmp_path / "coarse")


def test_accepted_is_decided_by_the_recorded_residual_drop(tmp_path):
    for drop, expect_numbers in ((3.5, True), (2.0, False)):
        w = tmp_path / f"d{drop}"
        w.mkdir()
        make_work(w, coarse={"drops": (drop, drop)})
        for a in ("m1.832_a00.00", "m1.832_a02.00"):
            p = w / "coarse/out/cfd/" / a / "done.json"
            d = json.loads(p.read_text())
            d["converged"] = False
            p.write_text(json.dumps(d))
        c = w / "coarse/out/cfd_cases.csv"
        c.write_text(c.read_text().replace(",converged,400", ",accepted,400").replace(",unconverged,400", ",accepted,400"))
        assert bf.read_level(w / "coarse")["cases"][2.0]["solver_converged"] is None
        _, gci = bf.main(w, w / "l.csv", w / "g.csv")
        rows = [r for r in csv.DictReader(l for l in gci.read_text().splitlines() if not l.startswith("#"))
                if r["quantity"] == "cna"]
        assert (rows[0]["gci_fine_pct"] != "") == expect_numbers


# ---------------------------------------------------------------- the job record (freshness)


def _edit_json(path, **kv):
    d = json.loads(path.read_text())
    d.update(kv)
    path.write_text(json.dumps(d))


@pytest.mark.parametrize("how", ["missing", "running", "failed_exit", "old_binary", "no_log", "log_names_other"])
def test_a_level_whose_job_record_is_not_a_finished_a8dff9c_job_is_refused(tmp_path, how):
    make_work(tmp_path)
    run = tmp_path / "medium"
    if how == "missing":
        (run / "done.json").unlink()
    elif how == "running":
        _edit_json(run / "done.json", state="running")
    elif how == "failed_exit":
        _edit_json(run / "done.json", exit_code=1)
    elif how == "old_binary":
        _edit_json(run / "done.json", binary="ignisyeet-glibc")
    elif how == "no_log":
        (run / "job.log").unlink()
    else:
        (run / "job.log").write_text("binary: /home/x/bin/ignisyeet-glibc\n")
    with pytest.raises(ValueError):
        bf.read_level(run)
    lv, gci = bf.main(tmp_path, tmp_path / "l.csv", tmp_path / "g.csv")  # refuses, does not raise
    rows = list(csv.DictReader(l for l in gci.read_text().splitlines() if not l.startswith("#")))
    assert rows and all(r["order"] == "" and "level medium" in r["note"] for r in rows)
    assert "source files refused" in lv.read_text()


def test_the_job_record_check_accepts_the_expected_binary_only(tmp_path):
    make_work(tmp_path)
    assert bf.read_level(tmp_path / "coarse")["job"]["log_binary"].endswith("ignisyeet-glibc-a8dff9c")
    with pytest.raises(ValueError, match="not 'other'"):
        bf.read_level(tmp_path / "coarse", binary="other")


# ---------------------------------------------------------------- a cases table that is whole but short


@pytest.mark.parametrize("keep", [0, 1])
def test_a_cases_table_with_rows_missing_is_refused_not_raised(tmp_path, keep):
    """Header only, or rows removed at a line boundary: the table looks complete line by line."""
    make_work(tmp_path)
    p = tmp_path / "coarse/out/cfd_cases.csv"
    p.write_text("".join(p.read_text().splitlines(keepends=True)[: 1 + keep]))
    with pytest.raises(ValueError):
        bf.read_level(tmp_path / "coarse")
    lv, gci = bf.main(tmp_path, tmp_path / "l.csv", tmp_path / "g.csv")
    rows = list(csv.DictReader(l for l in gci.read_text().splitlines() if not l.startswith("#")))
    assert rows and all(r["order"] == "" and "level coarse" in r["note"] for r in rows)


def test_a_missing_case_directory_is_refused(tmp_path):
    make_work(tmp_path)
    import shutil
    shutil.rmtree(tmp_path / "fine/out/cfd/m1.832_a02.00")
    with pytest.raises(ValueError, match="no done.json"):
        bf.read_level(tmp_path / "fine")
    _, gci = bf.main(tmp_path, tmp_path / "l.csv", tmp_path / "g.csv")
    assert "level fine" in gci.read_text()


def test_an_empty_level_directory_is_refused_not_raised(tmp_path):
    make_work(tmp_path)
    import shutil
    shutil.rmtree(tmp_path / "medium/out")
    bf.main(tmp_path, tmp_path / "l.csv", tmp_path / "g.csv")
    assert "source files refused" in (tmp_path / "l.csv").read_text()


# ---------------------------------------------------------------- the committed provenance


def test_both_headers_carry_provenance_the_flip_and_the_product_columns(tmp_path):
    make_work(tmp_path)
    (tmp_path / "provenance.txt").write_text("remote x --version: IgnisYeet 0.4.0\nremote repo-a8dff9c/COMMIT: a8dff9c\n")
    lv, gci = bf.main(tmp_path, tmp_path / "l.csv", tmp_path / "g.csv")
    for f in (lv, gci):
        head = "\n".join(l for l in f.read_text().splitlines() if l.startswith("#"))
        for needle in ("job.log: 'binary: ~/bin/ignisyeet-glibc-a8dff9c'", "remote x --version: IgnisYeet 0.4.0",
                       "done.json's ignisyeet_commit is 2f31ab9", "not a property of ignisyeet-glibc-a8dff9c",
                       "PBS job 1627", "follows the run setup, not the build", "t15-bf-coarse-r8-v040", "IY-F011",
                       "VIRGO_BIN=ignisyeet-glibc-a8dff9c", "memory_gb", "aero.method = cfd"):
            assert needle in head, (f.name, needle)
    for f in (lv, gci):
        assert "/home/" not in f.read_text(), f.name  # the account that ran the job is not named
    head = "\n".join(l for l in lv.read_text().splitlines() if l.startswith("#"))
    assert "min(rho_drop_orders, nu_drop_orders)" in head and "Task 8" in head and "not established" in head
    rows = list(csv.DictReader(l for l in lv.read_text().splitlines() if not l.startswith("#")))
    assert {"product_residual_drop", "rans_verdict"} <= set(rows[0])
    assert rows[0]["rans_verdict"] in ("converged", "stalled") and rows[0]["product_residual_drop"] != ""
