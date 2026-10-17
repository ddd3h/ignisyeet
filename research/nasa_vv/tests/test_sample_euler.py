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
    return (f"{mach},{alpha},{status},300,{first:.8e},{last:.8e},{cn_raw:.8e},{ca:.8e},0,0.0,,,,"
            f"100.0,false,0,0,0,0,0,0,0,0,,,")


def _combined(mach, alpha, cn, xcp):
    return f"{mach},{alpha},combined,,,,,0.01,,,{cn:.8e},0,{xcp:.8e},,,,,,,,,,,,,"


def make_level(work, name, wall_mm, fin_mm, nodes, cells, mirror=True, cn_slope=1.2, ca0=-0.03,
               status4="converged", status0="converged", drop=8.0, xcp=1.21, cn_m4=None,
               machs=(0.3, 0.8, 2.0), node="virgo01", ppn=20):
    d = pathlib.Path(work) / name
    out = d / "out"
    (out / "cfd").mkdir(parents=True)
    (d / "case.toml").write_text(
        f"[aero.cfd]\nwall_size = {wall_mm / 1000}\nfin_size = {fin_mm / 1000}\n"
        f"z_mirror_mesh = {'true' if mirror else 'false'}\nalpha_mode = \"mirror\"\n"
        "ranks_per_case = 4\nparallel_cases = 3\n")
    (out / "cfd_report.json").write_text(json.dumps({"mesh_nodes": nodes, "mesh_cells": cells}))
    (d / "done.json").write_text(json.dumps({
        "state": "done", "exit_code": 0, "wall_seconds": 1000.0, "node": node, "ppn": ppn,
        "ignisyeet_commit": "abc1234", "binary": "ignisyeet-glibc"}))
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
                '"Time_Iter","Outer_Iter","Inner_Iter","rms[Rho]"\n'
                "0,0,0,-0.5\n0,0,1,0.5\n" + f"0,0,2,{-drop + 0.5}\n")
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


def test_the_header_names_the_binary_and_the_machine(work, tmp_path):
    h = _head(run(work, tmp_path)[1])
    assert "abc1234" in h and "virgo01" in h


def test_every_blank_result_has_a_note(work, tmp_path):
    for r in _read(run(work, tmp_path)[1]):
        if r["order"] == "":
            assert r["note"] != ""


def test_the_default_levels_path_resolves_from_the_module_not_the_cwd():
    assert se.LEVELS_CSV.is_absolute() and se.GCI_CSV.is_absolute()
    assert se.LEVELS_CSV.parent.name == "results"
