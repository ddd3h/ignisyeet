# SPDX-License-Identifier: GPL-3.0-or-later
"""The sample-rocket Euler collector, on a synthetic work directory (no CFD is run)."""
import csv
import json
import math
import pathlib

import pytest

from nasa_vv.cases import sample_euler as se

CASES_HEADER = ("mach,alpha_deg,status,iterations,residual_first,residual_last,cn_raw,ca_cfd,"
                "nose_moment_raw,xcp_raw_m,cn,nose_moment,xcp_m,wall_seconds,warm_start,beta_deg,"
                "roll_hat,cx,cy,cz,cl,cm,cn_yaw,residual_reference,residual_drop,rans_verdict")


def _case_row(mach, alpha, status, cn_raw, ca, last=-9.0, first=-1.0):
    return (f"{mach},{alpha},{status},3,{first:.8e},{last:.8e},{cn_raw:.8e},{ca:.8e},0,0.0,,,,"
            f"100.0,false,0,0,0,0,0,0,0,0,,,")


def _combined(mach, alpha, cn, xcp):
    return f"{mach},{alpha},combined,,,,,0.01,,,{cn:.8e},0,{xcp:.8e},,,,,,,,,,,,,"


def make_level(work, name, wall_mm, fin_mm, nodes, cells, mirror=True, cn_slope=1.2, ca0=-0.03,
               status4="converged", status0="converged", drop=8.0, xcp=1.21, cn_m4=None,
               machs=(0.3, 0.8, 2.0), node="virgo01", ppn=20, binary="ignisyeet-glibc-a8dff9c"):
    d = pathlib.Path(work) / name
    out = d / "out"
    (out / "cfd").mkdir(parents=True)
    (d / "case.toml").write_text(
        f"[aero.cfd]\nwall_size = {wall_mm / 1000}\nfin_size = {fin_mm / 1000}\n"
        f"z_mirror_mesh = {'true' if mirror else 'false'}\nalpha_mode = \"mirror\"\n"
        f"machs = {list(machs)}\nalphas_deg = [0.0, 4.0]\n"
        "ranks_per_case = 4\nparallel_cases = 3\n")
    (out / "cfd_report.json").write_text(json.dumps({"mesh_nodes": nodes, "mesh_cells": cells,
                                                    "cases": len(machs) * (2 if mirror else 3)}))
    (d / "done.json").write_text(json.dumps({
        "state": "done", "exit_code": 0, "wall_seconds": 1000.0, "node": node, "ppn": ppn,
        "ignisyeet_commit": "abc1234", "binary": binary}))
    (d / "job.log").write_text(f"binary: /remote/bin/{binary}\n+ rc=0\n")
    rows = [CASES_HEADER]
    for m in machs:
        specs = [(0.0, status0, 1e-7, ca0), (4.0, status4, cn_slope * math.radians(4), ca0)]
        if not mirror:
            specs.append((-4.0, status4, -(cn_m4 if cn_m4 is not None else cn_slope * math.radians(4)), ca0))
        for a, st, cn, ca in specs:
            rows.append(_case_row(m, a, st, cn, ca, last=-drop + 0.5))
            cd = out / "cfd" / f"m{m:.3f}_a{a:+06.2f}"
            cd.mkdir()
            (cd / "done.json").write_text(json.dumps({"mach": m, "alpha_deg": a, "beta_deg": 0.0}))
            (cd / "history.csv").write_text(
                '"Time_Iter","Outer_Iter","Inner_Iter","rms[Rho]","CFx"\n'
                f"0,0,0,-0.5,{ca:.8e}\n0,0,1,0.5,{ca:.8e}\n" + f"0,0,2,{-drop + 0.5},{ca:.8e}\n")
    for m in machs:
        rows.append(_combined(m, 4.0, cn_slope * math.radians(4), xcp))
    (out / "cfd_cases.csv").write_text("\n".join(rows) + "\n")
    return d


@pytest.fixture
def work(tmp_path):
    w = tmp_path / "work"
    w.mkdir()
    # wall 3.92 / 2.80 / 2.00 mm, fin 1.96 / 1.40 / 1.00 mm; cn(4) converging from above
    make_level(w, "coarse", 3.92, 1.96, 400_000, 2_000_000, cn_slope=1.30, ca0=-0.0300, xcp=1.20)
    make_level(w, "medium", 2.80, 1.40, 520_000, 2_700_000, cn_slope=1.26, ca0=-0.0200, xcp=1.205)
    make_level(w, "fine", 2.00, 1.00, 700_000, 3_700_000, cn_slope=1.25, ca0=-0.0150, xcp=1.207)
    make_level(w, "medium_nomirror", 2.80, 1.40, 520_000, 2_700_000, mirror=False, cn_slope=1.26,
               cn_m4=1.2598 * math.radians(4))
    return w


def _read(path):
    lines = [ln for ln in pathlib.Path(path).read_text().splitlines() if not ln.startswith("#")]
    return list(csv.DictReader(lines))


def _head(path):
    return "\n".join(ln for ln in pathlib.Path(path).read_text().splitlines() if ln.startswith("#"))


def run(work, tmp_path):
    lv, gci = tmp_path / "levels.csv", tmp_path / "gci.csv"
    se.main(work, lv, gci)
    return lv, gci


# ------------------------------------------------------------------ CNa


def test_cna_from_both_signs_is_the_half_difference_over_radians_four():
    assert se.cna(0.09, -0.07) == pytest.approx(0.08 / math.radians(4))


def test_cna_from_one_sign_is_cn_over_radians_four():
    assert se.cna(0.09, None) == pytest.approx(0.09 / math.radians(4))


def test_levels_csv_has_a_row_per_level_and_mach_with_the_cna_basis(work, tmp_path):
    lv, _ = run(work, tmp_path)
    rows = _read(lv)
    assert {r["level"] for r in rows} == {"coarse", "medium", "fine", "medium_nomirror"}
    assert len(rows) == 12
    for r in rows:
        want = "(CN(4)-CN(-4))/2" if r["level"] == "medium_nomirror" else "CN(4)"
        assert r["cna_basis"] == want
    fine = next(r for r in rows if r["level"] == "fine" and float(r["mach"]) == 0.8)
    assert float(fine["cna"]) == pytest.approx(1.25, rel=1e-6)
    assert int(fine["nodes"]) == 700_000 and int(fine["cells"]) == 3_700_000
    assert float(fine["wall_size_mm"]) == 2.0 and float(fine["fin_size_mm"]) == 1.0


def test_the_nonmirrored_level_has_both_signs_and_the_mirrored_levels_do_not(work, tmp_path):
    rows = _read(run(work, tmp_path)[0])
    for r in rows:
        assert (r["cn_raw_am4"] != "") == (r["level"] == "medium_nomirror")


# ------------------------------------------------------------------ GCI


def test_every_quantity_is_written_under_both_h_bases(work, tmp_path):
    rows = _read(run(work, tmp_path)[1])
    assert {r["h_basis"] for r in rows} == {"nodes^-1/3", "wall_size"}
    assert {r["quantity"] for r in rows} == {"cna", "ca_alpha0", "xcp"}
    by = {}
    for r in rows:
        by.setdefault((r["condition"], r["quantity"]), []).append(r["h_basis"])
    assert len(by) == 9
    assert all(sorted(v) == ["nodes^-1/3", "wall_size"] for v in by.values())


def test_the_wall_size_rows_use_the_knob_ratio(work, tmp_path):
    rows = _read(run(work, tmp_path)[1])
    r = next(r for r in rows if r["quantity"] == "cna" and r["h_basis"] == "wall_size"
             and r["condition"].startswith("M=0.8"))
    assert float(r["r21"]) == pytest.approx(1.4, rel=1e-6)
    n = next(r for r in rows if r["quantity"] == "cna" and r["h_basis"] == "nodes^-1/3"
             and r["condition"].startswith("M=0.8"))
    assert float(n["r21"]) == pytest.approx((700_000 / 520_000) ** (1 / 3), rel=1e-6)
    assert r["triplet"] == "fine/medium/coarse"


def test_the_nonmirrored_level_is_not_part_of_any_family(work, tmp_path):
    rows = _read(run(work, tmp_path)[1])
    assert all("nomirror" not in r["condition"] and "false" not in r["condition"] for r in rows)


def test_an_unconverged_level_is_refused_with_no_numbers(tmp_path):
    w = tmp_path / "w"
    w.mkdir()
    make_level(w, "coarse", 3.92, 1.96, 400_000, 2_000_000, ca0=-0.03)
    make_level(w, "medium", 2.80, 1.40, 520_000, 2_700_000, status4="unconverged", drop=1.5, ca0=-0.02)
    make_level(w, "fine", 2.00, 1.00, 700_000, 3_700_000, ca0=-0.015)
    make_level(w, "medium_nomirror", 2.80, 1.40, 520_000, 2_700_000, mirror=False)
    lv, gci = run(w, tmp_path)
    rows = _read(gci)
    cna = [r for r in rows if r["quantity"] == "cna"]
    assert cna and all(r["order"] == "" and r["gci_fine_pct"] == "" for r in cna)
    assert all(r["note"].startswith("refused: ") and "'medium'" in r["note"] for r in cna)
    # the alpha = 0 solves converged, so ca_alpha0 is still computed
    ca = [r for r in rows if r["quantity"] == "ca_alpha0"]
    assert all(r["order"] != "" for r in ca)


def test_a_low_residual_drop_without_a_verdict_is_refused_not_weakened(tmp_path):
    w = tmp_path / "w"
    w.mkdir()
    for n, wall, fin, nodes in (("coarse", 3.92, 1.96, 4e5), ("medium", 2.8, 1.4, 5.2e5),
                                ("fine", 2.0, 1.0, 7e5)):
        make_level(w, n, wall, fin, int(nodes), int(nodes * 5), drop=2.0 if n == "fine" else 8.0)
    make_level(w, "medium_nomirror", 2.8, 1.4, 520_000, 2_700_000, mirror=False)
    levels = se.read_levels(w)
    lvl = levels["fine"].solves[(0.3, 4.0)]
    assert lvl.residual_drop_orders == pytest.approx(2.0)
    assert lvl.status == "converged"  # the verdict decides: the drop floor is not applied on top


def test_unknown_status_string_is_an_error(tmp_path):
    w = tmp_path / "w"
    w.mkdir()
    make_level(w, "coarse", 3.92, 1.96, 400_000, 2_000_000, status4="solved")
    make_level(w, "medium", 2.8, 1.4, 520_000, 2_700_000)
    make_level(w, "fine", 2.0, 1.0, 700_000, 3_700_000)
    make_level(w, "medium_nomirror", 2.8, 1.4, 520_000, 2_700_000, mirror=False)
    with pytest.raises(ValueError, match="solved"):
        se.main(w, tmp_path / "a.csv", tmp_path / "b.csv")


def test_a_missing_level_is_named(tmp_path):
    w = tmp_path / "w"
    w.mkdir()
    make_level(w, "coarse", 3.92, 1.96, 400_000, 2_000_000)
    with pytest.raises(FileNotFoundError, match="level .fine."):
        se.main(w, tmp_path / "a.csv", tmp_path / "b.csv")


# ------------------------------------------------------------------ determinism and header


def test_collecting_twice_is_byte_identical(work, tmp_path):
    a, b = tmp_path / "a", tmp_path / "b"
    a.mkdir(), b.mkdir()
    la, ga = run(work, a)
    lb, gb = run(work, b)
    assert la.read_bytes() == lb.read_bytes() and ga.read_bytes() == gb.read_bytes()


def test_independent_of_the_working_directory(work, tmp_path, monkeypatch):
    la, ga = run(work, tmp_path)
    other = tmp_path / "elsewhere"
    other.mkdir()
    monkeypatch.chdir(other)
    lb, gb = tmp_path / "l2.csv", tmp_path / "g2.csv"
    se.main(work, lb, gb)
    assert la.read_bytes() == lb.read_bytes() and ga.read_bytes() == gb.read_bytes()


def test_the_header_states_that_check_similar_sees_only_the_knobs_and_the_measured_growth(work, tmp_path):
    h = _head(run(work, tmp_path)[1])
    assert "check_similar sees only the knobs" in h
    assert "2.74" in h
    assert "1.35" in h and "1.37" in h  # cells 2.0 -> 2.7 -> 3.7 M: growth 1.35x and 1.37x
    assert "crates/cfd/src/mesh.rs:396" in h and "crates/cfd/src/mesh_gmsh.py:153" in h
    assert "spread" in h


def test_the_header_names_the_binary_its_evidence_and_the_machine(work, tmp_path):
    h = _head(run(work, tmp_path)[1])
    assert "virgo01" in h
    assert 'Binary: ignisyeet-glibc-a8dff9c; evidence: the job.log line "binary: /remote/bin/ignisyeet-glibc-a8dff9c"' in h
    assert "IgnisYeet 0.4.0" in h


def test_a_binary_name_with_a_commit_suffix_never_prints_the_job_records_commit_as_its_source(work, tmp_path):
    h = _head(run(work, tmp_path)[1])
    assert "is stale for this binary" in h and "(abc1234)" in h
    assert "at ignisyeet commit" not in h
    assert "a8dff9c" in h


def test_a_binary_name_without_a_suffix_says_the_commit_need_not_be_its_source(tmp_path):
    w = tmp_path / "w"
    w.mkdir()
    for n, wall, fin, nodes in (("coarse", 3.92, 1.96, 4e5), ("medium", 2.8, 1.4, 5.2e5), ("fine", 2.0, 1.0, 7e5)):
        make_level(w, n, wall, fin, int(nodes), int(nodes * 5), binary="ignisyeet-glibc", ca0=-wall / 100)
    make_level(w, "medium_nomirror", 2.8, 1.4, 520_000, 2_700_000, mirror=False, binary="ignisyeet-glibc")
    h = _head(se.main(w, tmp_path / "l.csv", tmp_path / "g.csv")[1])
    assert "need not be the binary's source" in h and "is stale for this binary" not in h


def test_the_header_lists_the_inputs_and_every_hand_edit(work, tmp_path):
    h = _head(run(work, tmp_path)[1])
    assert "examples/sample_cfd.toml" in h and "rans_sample.py" in h
    for needle in ("# wall_size = 0.002 and iterations", "relative names", "output.html", "kml = false",
                   "iterations left to the model default", "[0, 4]", "[resources] block", "sample_euler_inputs"):
        assert needle in h, needle


def test_the_header_states_the_status_mapping_and_the_sizing(work, tmp_path):
    h = _head(run(work, tmp_path)[1])
    assert "accepted -> None" in h and "unconverged (limit cycle) and failed -> False" in h
    assert "98157 nodes" in h and "1.7 GB x 7.53 = 12.8 GB" in h


def test_the_committed_inputs_match_what_the_header_claims():
    import tomllib
    d = se.INPUTS
    got = {}
    for n in ("coarse", "medium", "fine", "medium_nomirror"):
        text = (d / f"{n}.toml").read_text()
        assert "/home/" not in text and "<RUN_DIR>" in text and "<CFD_PREFIX>" in text
        c = tomllib.loads(text)["aero"]["cfd"]
        got[n] = (c["wall_size"], c["fin_size"], c["z_mirror_mesh"])
    assert got == {"coarse": (0.00392, 0.00196, True), "medium": (0.0028, 0.0014, True),
                   "fine": (0.002, 0.001, True), "medium_nomirror": (0.0028, 0.0014, False)}
    assert "about 1.4 M cells" not in "".join((d / f"{n}.toml").read_text() for n in got)


def test_every_blank_result_has_a_note(work, tmp_path):
    for r in _read(run(work, tmp_path)[1]):
        if r["order"] == "":
            assert r["note"] != ""


def test_the_default_levels_path_resolves_from_the_module_not_the_cwd():
    assert se.LEVELS_CSV.is_absolute() and se.GCI_CSV.is_absolute()
    assert se.LEVELS_CSV.parent.name == "results"


# ------------------------------------------------------------------ truncated or stale sources


def _refused_for(work, tmp_path, needle):
    lv, gci = run(work, tmp_path)
    rows = _read(gci)
    assert rows and all(r["order"] == "" and r["gci_fine_pct"] == "" for r in rows)
    assert all(r["note"].startswith("refused: ") and needle in r["note"] for r in rows)
    check = {r["level"]: r["source_check"] for r in _read(lv)}
    return check


def _history(work, level, mach, alpha):
    return next(p for p in (work / level / "out" / "cfd").iterdir()
                if f"m{mach:.3f}_a{alpha:+06.2f}" == p.name) / "history.csv"


def test_a_history_cut_mid_row_refuses_the_level(work, tmp_path):
    h = _history(work, "fine", 0.8, 4.0)
    h.write_text(h.read_text().rstrip("\n").rsplit(",", 1)[0])  # last row loses a field and its newline
    check = _refused_for(work, tmp_path, "level 'fine' failed the completeness check")
    assert check["fine"].startswith("FAILED") and check["coarse"] == "ok"


def test_a_history_with_a_partial_last_field_is_refused(work, tmp_path):
    h = _history(work, "coarse", 0.3, 0.0)
    h.write_text(h.read_text().rstrip("\n")[:-3] + "e")  # '-8.5' cut to '-8e' style garbage, no newline
    _refused_for(work, tmp_path, "truncated")


def test_a_complete_row_missing_its_newline_is_still_refused(work, tmp_path):
    h = _history(work, "medium", 2.0, 4.0)
    h.write_text(h.read_text().rstrip("\n"))
    _refused_for(work, tmp_path, "does not end in a newline")


def test_a_history_older_than_the_case_record_is_refused(work, tmp_path):
    h = _history(work, "fine", 0.3, 4.0)
    lines = h.read_text().splitlines()
    h.write_text("\n".join(lines[:-1]) + "\n")  # the last write never happened: the previous one is on disk
    _refused_for(work, tmp_path, "has 2 rows but cfd_cases.csv records 3 iterations")


def test_a_history_whose_judged_row_disagrees_with_the_record_is_refused(work, tmp_path):
    h = _history(work, "fine", 0.3, 4.0)
    h.write_text(h.read_text().replace("-7.5", "-6.5"))
    _refused_for(work, tmp_path, "has no row with rms[Rho]")


def test_rows_after_the_judged_iteration_are_recorded_not_hidden(work, tmp_path):
    h = _history(work, "fine", 0.3, 4.0)
    h.write_text(h.read_text() + "0,0,3,-9.5,-0.0\n0,0,4,-9.6,-0.0\n")
    lv, gci = run(work, tmp_path)
    row = next(r for r in _read(lv) if r["level"] == "fine" and r["mach"] == "0.3")
    assert row["history_gap_a4"] == "2" and row["history_gap_a0"] == "0"
    assert row["source_check"] == "ok"
    assert all(r["note"] == "" or not r["note"].startswith("refused") for r in _read(gci))


def test_a_cases_file_with_a_cut_last_row_refuses_the_level(work, tmp_path):
    c = work / "medium" / "out" / "cfd_cases.csv"
    lines = c.read_text().splitlines()
    c.write_text("\n".join(lines[:-1] + [lines[-1][:30]]))
    check = _refused_for(work, tmp_path, "level 'medium' failed the completeness check")
    assert "fields (truncated)" in check["medium"]


def test_a_cases_file_missing_a_solve_refuses_the_level(work, tmp_path):
    c = work / "coarse" / "out" / "cfd_cases.csv"
    lines = [ln for ln in c.read_text().splitlines() if not ln.startswith("2.0,4.0,")]
    c.write_text("\n".join(lines) + "\n")
    _refused_for(work, tmp_path, "the solve at mach 2 alpha 4 is missing")


def test_a_cut_report_refuses_the_level(work, tmp_path):
    r = work / "fine" / "out" / "cfd_report.json"
    r.write_text(r.read_text()[:20])
    _refused_for(work, tmp_path, "cfd_report.json is not valid JSON")


def test_a_job_that_did_not_finish_refuses_the_level(work, tmp_path):
    d = work / "fine" / "done.json"
    d.write_text(json.dumps({"state": "running", "node": "virgo01"}))
    _refused_for(work, tmp_path, "state 'running'")


def test_a_missing_case_directory_refuses_the_level(work, tmp_path):
    import shutil
    shutil.rmtree(_history(work, "fine", 2.0, 0.0).parent)
    _refused_for(work, tmp_path, "no case directory for mach 2 alpha 0")


def test_a_report_that_counts_other_cases_than_the_cases_file_refuses_the_level(work, tmp_path):
    r = work / "fine" / "out" / "cfd_report.json"
    j = json.loads(r.read_text())
    j["cases"] = 7
    r.write_text(json.dumps(j))
    _refused_for(work, tmp_path, "records 7 cases but cfd_cases.csv holds 6")


def _set_cfx(path, values):
    lines = path.read_text().splitlines()
    out = [lines[0]]
    for ln, v in zip(lines[1:], values):
        f = ln.split(",")
        f[-1] = f"{v:.8e}"
        out.append(",".join(f))
    path.write_text("\n".join(out) + "\n")


def test_a_recorded_force_outside_the_history_is_refused(work, tmp_path):
    _set_cfx(_history(work, "fine", 0.3, 4.0), [-0.09, -0.09, -0.09])  # the record says -0.0150
    _refused_for(work, tmp_path, "lies outside the CFx")


def test_a_window_mean_force_between_the_rows_is_accepted(work, tmp_path):
    # the record (-0.0150) is a window mean: the rows around the judged one are on both sides of it
    _set_cfx(_history(work, "fine", 0.3, 4.0), [-0.0170, -0.0160, -0.0130])
    lv, _ = run(work, tmp_path)
    assert next(r for r in _read(lv) if r["level"] == "fine" and r["mach"] == "0.3")["source_check"] == "ok"


# ------------------------------------------------------------------ hardening (fix round 1)


def test_a_history_with_a_short_row_and_a_final_newline_is_refused_by_the_field_count(work, tmp_path):
    h = _history(work, "fine", 0.8, 4.0)
    lines = h.read_text().splitlines()
    lines[-1] = lines[-1].rsplit(",", 1)[0]  # one field missing, newline kept
    h.write_text("\n".join(lines) + "\n")
    check = _refused_for(work, tmp_path, "has 4 of 5 fields (truncated)")
    assert check["coarse"] == "ok"


def test_a_history_with_a_non_numeric_field_and_a_final_newline_is_refused(work, tmp_path):
    h = _history(work, "fine", 0.8, 4.0)
    lines = h.read_text().splitlines()
    lines[-1] = lines[-1].rsplit(",", 1)[0] + ",-0.0e"  # a cut number, same field count
    lines[-1] = lines[-1].replace(lines[-1].split(",")[3], "-7.5x")
    h.write_text("\n".join(lines) + "\n")
    _refused_for(work, tmp_path, "has a non-numeric field (truncated)")


def test_an_empty_history_file_is_refused(work, tmp_path):
    _history(work, "fine", 0.8, 4.0).write_text("")
    _refused_for(work, tmp_path, "level 'fine' failed the completeness check")


def test_a_history_holding_only_its_header_is_refused(work, tmp_path):
    h = _history(work, "fine", 0.8, 4.0)
    h.write_text(h.read_text().splitlines()[0] + "\n")
    _refused_for(work, tmp_path, "rows but cfd_cases.csv records")


def test_a_truncated_case_level_done_json_is_refused(work, tmp_path):
    d = _history(work, "fine", 0.8, 4.0).parent / "done.json"
    d.write_text(d.read_text()[:15])
    _refused_for(work, tmp_path, "done.json is not valid JSON")


def _cases_edit(work, level, fn):
    c = work / level / "out" / "cfd_cases.csv"
    c.write_text("".join(fn(ln) + "\n" for ln in c.read_text().splitlines()))


def test_a_blank_residual_last_refuses_the_level(work, tmp_path):
    def blank(ln):
        f = ln.split(",")
        if f[0] == "0.8" and f[1] == "4.0":
            f[5] = ""
        return ",".join(f)
    _cases_edit(work, "fine", blank)
    _refused_for(work, tmp_path, "residual_last is blank in cfd_cases.csv")


def test_a_blank_ca_cfd_refuses_the_level(work, tmp_path):
    def blank(ln):
        f = ln.split(",")
        if f[0] == "0.8" and f[1] == "0.0":
            f[7] = ""
        return ",".join(f)
    _cases_edit(work, "medium", blank)
    _refused_for(work, tmp_path, "ca_cfd is blank in cfd_cases.csv")


def test_an_empty_cases_file_refuses_with_a_note_instead_of_raising(work, tmp_path):
    (work / "fine" / "out" / "cfd_cases.csv").write_text("")
    rows = _read(run(work, tmp_path)[1])
    assert rows and all(r["note"].startswith("refused: ") for r in rows)
    assert all("empty or holds only its header" in r["note"] for r in rows)


def test_a_non_numeric_mach_refuses_with_a_note_instead_of_raising(work, tmp_path):
    _cases_edit(work, "coarse", lambda ln: ln.replace("0.8,4.0,", "0.8x,4.0,", 1))
    _refused_for(work, tmp_path, "non-numeric field")


def test_a_judged_row_that_matches_twice_is_ambiguous(work, tmp_path):
    h = _history(work, "fine", 0.3, 4.0)
    lines = h.read_text().splitlines()
    lines.insert(2, lines[-1])  # the last row's residual again, earlier
    h.write_text("\n".join(lines) + "\n")
    _cases_edit(work, "fine", lambda ln: ln.replace(",3,", ",4,", 1) if ln.startswith("0.3,4.0,") else ln)
    _refused_for(work, tmp_path, "the judged row is ambiguous")


# ---- accepted maps to None: the residual drop decides


def test_an_accepted_solve_with_a_large_drop_enters_the_family(tmp_path):
    w = tmp_path / "w"
    w.mkdir()
    make_level(w, "coarse", 3.92, 1.96, 400_000, 2_000_000, cn_slope=1.30, ca0=-0.03)
    make_level(w, "medium", 2.80, 1.40, 520_000, 2_700_000, cn_slope=1.26, ca0=-0.02, status4="accepted", drop=6.0)
    make_level(w, "fine", 2.00, 1.00, 700_000, 3_700_000, cn_slope=1.25, ca0=-0.015)
    make_level(w, "medium_nomirror", 2.80, 1.40, 520_000, 2_700_000, mirror=False)
    rows = _read(run(w, tmp_path)[1])
    cna = [r for r in rows if r["quantity"] == "cna"]
    assert cna and all(r["order"] != "" for r in cna)


def test_an_accepted_solve_with_a_small_drop_is_refused_by_the_gate(tmp_path):
    w = tmp_path / "w"
    w.mkdir()
    make_level(w, "coarse", 3.92, 1.96, 400_000, 2_000_000, cn_slope=1.30, ca0=-0.03)
    make_level(w, "medium", 2.80, 1.40, 520_000, 2_700_000, cn_slope=1.26, ca0=-0.02, status4="accepted", drop=2.0)
    make_level(w, "fine", 2.00, 1.00, 700_000, 3_700_000, cn_slope=1.25, ca0=-0.015)
    make_level(w, "medium_nomirror", 2.80, 1.40, 520_000, 2_700_000, mirror=False)
    rows = _read(run(w, tmp_path)[1])
    cna = [r for r in rows if r["quantity"] == "cna" and r["condition"].startswith("M=0.8")]
    assert all(r["order"] == "" and "dropped only 2.0 orders" in r["note"] for r in cna)


def test_the_status_table_is_the_one_of_existing_data():
    from nasa_vv.cases import existing_data as ed
    assert not hasattr(se, "_STATUS")
    assert ed._verdict(["converged", "accepted"]) is None
    assert ed._verdict(["accepted", "failed"]) is False


# ---- unusable rows: one shared rule


def _row(**kw):
    base = {"order": "1.0", "gci_fine_pct": "5", "extrapolated": "1", "f1": "1", "condition": "M=0.8 alpha=0",
            "quantity": "ca_alpha0", "note": ""}
    base.update(kw)
    return base


def test_a_gci_above_a_hundred_percent_is_unusable():
    assert se._unusable(_row(gci_fine_pct="309")).startswith("unusable: gci_fine_pct = 309 exceeds 100")
    assert se._unusable(_row(gci_fine_pct="99")) == ""


def test_an_extrapolation_past_the_exact_value_is_unusable_at_m03_only():
    crossing = _row(condition="M=0.3 alpha=0", f1="-0.03", extrapolated="0.045")
    assert "other side of the exact value 0" in se._unusable(crossing)
    assert se._unusable(_row(condition="M=0.3 alpha=0", f1="-0.03", extrapolated="-0.01")) == ""
    assert se._unusable(_row(condition="M=0.8 alpha=0", f1="-0.03", extrapolated="0.045")) == ""


def test_a_refused_row_is_never_flagged():
    assert se._unusable(_row(order="", gci_fine_pct="", extrapolated="")) == ""


def test_the_flag_reaches_the_note_of_the_written_row(tmp_path):
    w = tmp_path / "w"
    w.mkdir()
    # CA at M = 0.3 grows toward zero and the extrapolation crosses it
    make_level(w, "coarse", 3.92, 1.96, 400_000, 2_000_000, ca0=-0.0630)
    make_level(w, "medium", 2.80, 1.40, 520_000, 2_700_000, ca0=-0.0450)
    make_level(w, "fine", 2.00, 1.00, 700_000, 3_700_000, ca0=-0.0300)
    make_level(w, "medium_nomirror", 2.80, 1.40, 520_000, 2_700_000, mirror=False)
    rows = [r for r in _read(run(w, tmp_path)[1]) if r["quantity"] == "ca_alpha0" and r["condition"].startswith("M=0.3")]
    assert rows and all("unusable:" in r["note"] for r in rows)
    assert all("other side of the exact value 0" in r["note"] for r in rows)
