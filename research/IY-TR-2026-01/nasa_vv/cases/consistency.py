# SPDX-License-Identifier: GPL-3.0-or-later
"""The consistency checks of spec section 6.3, read from the committed CFD results.

This is verification: it checks that relations physics requires hold in the data
IgnisYeet already produced. No CFD is run here.

    cn_zero_alpha    CN(alpha = 0) = 0 on every configuration of asymmetry_study.csv
                     and on the production runs of sample_euler_cases.csv.
    cn_antisymmetry  CN(-alpha) = -CN(alpha): the even part of the +4 / -4 pair is 0.
    ca_dalembert     CA at M = 0.3, alpha = 0 is 0 (d'Alembert's paradox).
    ca_dalembert_refined  the same CA on each level of the refined Euler grid family
                     (results/sample_euler_levels.csv), one row per level, coarse first.

``antisymmetry(cn_plus, cn_minus)`` is the even part ``(cn_plus + cn_minus) / 2``.
The two CN checks on asymmetry_study.csv share one tolerance, 1 % of ``|cn_odd_4deg|``
of the same row: that is the mirror-combined normal force ``(CN(+4) - CN(-4)) / 2``,
the scale the alpha = 0 and the 4 deg offsets spoil. The production ``cn_zero_alpha``
rows use 1 % of ``|cn_raw|`` of the same Mach number's alpha = 4 row. No tolerance is invented for
``ca_dalembert`` and ``ca_dalembert_refined``: whether |CA| shrinks with the grid is stated in
the header, so their ``tolerance`` and ``passed`` are blank. The ``cn_antisymmetry`` rows of the
non-mirrored medium level of the refined family use the same 1 % of the odd part as the
asymmetry-study rows.

Run it with

    cd research && uv run --project nasa_vv python -m nasa_vv.cases.consistency

and read ``results/consistency.csv``. Source files and the last commit that changed
each are recorded in the CSV header; every row names its file, its row and the commit.
"""
from __future__ import annotations

import csv
import pathlib

from .existing_data import ROOT, _last_commit, _num, _read_table

RESULTS = pathlib.Path(__file__).resolve().parents[1] / "results" / "consistency.csv"

ASYMMETRY_CSV = "doc/data/cfd/asymmetry_study.csv"
EULER_CASES_CSV = "doc/data/cfd/sample_euler_cases.csv"
SAMPLE_LEVELS_REL = "research/IY-TR-2026-01/nasa_vv/results/sample_euler_levels.csv"
SAMPLE_LEVELS_CSV = pathlib.Path(__file__).resolve().parents[1] / "results" / "sample_euler_levels.csv"
SAMPLE_LEVELS_COLUMNS = ("level", "mach", "wall_size_mm", "nodes", "status_a0", "status_a4",
                         "status_am4", "ca_alpha0", "cn_raw_a0", "cn_raw_a4", "cn_raw_am4")
FAMILY_LEVELS = ("coarse", "medium", "fine")
NOMIRROR_LEVEL = "medium_nomirror"

ASYMMETRY_COLUMNS = ("config", "mach", "cn0", "cn_even_4deg", "cn_odd_4deg")
EULER_COLUMNS = ("mach", "alpha_deg", "status", "cn_raw", "ca_cfd")

COLUMNS = ["check", "case", "mach", "alpha_deg", "model", "quantity",
           "value", "expected", "tolerance", "passed", "source"]

CN_ZERO_ALPHA = "cn_zero_alpha"
CN_ANTISYMMETRY = "cn_antisymmetry"
CA_DALEMBERT = "ca_dalembert"
CA_DALEMBERT_REFINED = "ca_dalembert_refined"

Z_SYMMETRIC = "zm"
"""Config key of the z-symmetric (quarter + mirror) mesh in asymmetry_study.csv."""

MIRROR_NOTE = ("zm is the quarter-plus-mirror mesh, so CN(0) = 0 by construction (mesh symmetry) and "
               "this row tests nothing about the solver; value rounded to 5 decimals")
ANTISYMMETRY_MIRROR_NOTE = ("zm is the quarter-plus-mirror mesh, where -alpha is the exact mirror of "
                            "+alpha, so this row is near zero partly by construction; "
                            "value rounded to 5 decimals")
PRODUCTION_MESH_NOTE = ("production runs use a z-symmetric mesh (alpha_mode mirror, per the file's "
                        "header), so CN(0) = 0 by mesh symmetry up to solver round-off")

TOLERANCE_FRACTION = 0.01
"""CN tolerance, as a fraction of |cn_odd_4deg|; see the module docstring."""

CA_DALEMBERT_CASE = (0.3, 0.0)
"""The M = 0.3, alpha = 0 row of sample_euler_cases.csv that carries ca_cfd."""


def antisymmetry(cn_plus: float, cn_minus: float) -> float:
    """Even part of a +alpha / -alpha pair, ``(cn_plus + cn_minus) / 2``; 0 if antisymmetric."""
    return (cn_plus + cn_minus) / 2.0


def _value(x: float) -> str:
    """Significant-digit text; a negative zero from the CSV is written as 0."""
    return format(x + 0.0, ".10g")


def _row(check, case, mach, alpha_deg, model, quantity, value, expected, tolerance, source):
    if tolerance is None:
        tolerance_text, passed = "", ""
    else:
        tolerance_text, passed = _value(tolerance), str(abs(value) <= tolerance)
    return {"check": check, "case": case, "mach": _value(mach), "alpha_deg": _value(alpha_deg),
            "model": model, "quantity": quantity, "value": _value(value),
            "expected": _value(expected), "tolerance": tolerance_text, "passed": passed,
            "source": source}


def _antisymmetry_tolerance(cn_odd_4deg: float) -> float:
    return TOLERANCE_FRACTION * abs(cn_odd_4deg)


def cn_rows() -> list[dict[str, str]]:
    """CN(0) = 0 on every asymmetry-study row and every production Mach, and the even part
    of every +4 / -4 pair."""
    rows = _read_table(ROOT / ASYMMETRY_CSV, ASYMMETRY_COLUMNS)
    commit = _last_commit(ASYMMETRY_CSV)
    zero_alpha, antisymmetric = [], []
    for r in rows:
        config, mach = r["config"], _num(r, "mach")
        mirrored = config == Z_SYMMETRIC
        tolerance = _antisymmetry_tolerance(_num(r, "cn_odd_4deg"))
        where = f"{ASYMMETRY_CSV} config={config} mach={mach:g} @ {commit}"
        zero_alpha.append(_row(
            CN_ZERO_ALPHA, config, mach, 0.0, "euler", "cn0",
            _num(r, "cn0"), 0.0, tolerance, where + (f"; {MIRROR_NOTE}" if mirrored else "")))
        antisymmetric.append(_row(
            CN_ANTISYMMETRY, config, mach, 4.0, "euler", "cn_even_4deg",
            _num(r, "cn_even_4deg"), 0.0, tolerance,
            where + "; cn_even_4deg combined value"
            + (f"; {ANTISYMMETRY_MIRROR_NOTE}" if mirrored else "")))
    return zero_alpha + production_zero_alpha_rows() + antisymmetric


def production_zero_alpha_rows() -> list[dict[str, str]]:
    """CN(0) = 0 on the production runs: cn_raw at alpha = 0, full precision, per Mach.

    The tolerance is 1 % of |cn_raw| of the same Mach number's alpha = 4 row. A Mach
    number with no alpha = 4 row gets a blank tolerance and a blank passed.
    """
    rows = [r for r in _read_table(ROOT / EULER_CASES_CSV, EULER_COLUMNS)
            if r["status"].strip() == "converged" and r["cn_raw"].strip()]
    commit = _last_commit(EULER_CASES_CSV)
    out = []
    for r in rows:
        if _num(r, "alpha_deg") != 0.0:
            continue
        mach = _num(r, "mach")
        four = [q for q in rows if _num(q, "mach") == mach and _num(q, "alpha_deg") == 4.0]
        where = (f"{EULER_CASES_CSV} mach={mach:g} alpha_deg=0 @ {commit}; {PRODUCTION_MESH_NOTE}")
        if len(four) == 1:
            tolerance = _antisymmetry_tolerance(_num(four[0], "cn_raw"))
        elif not four:
            tolerance = None
            where += f"; no alpha_deg=4 row at mach={mach:g}, so no tolerance basis and no pass/fail"
        else:
            raise ValueError(f"{EULER_CASES_CSV}: {len(four)} alpha_deg=4 rows at mach={mach:g}")
        out.append(_row(CN_ZERO_ALPHA, "sample rocket", mach, 0.0, "euler", "cn_raw",
                        _num(r, "cn_raw"), 0.0, tolerance, where))
    return out


def ca_dalembert_row() -> dict[str, str]:
    """CA at M = 0.3, alpha = 0; expected 0, with no tolerance and no pass/fail."""
    mach, alpha = CA_DALEMBERT_CASE
    rows = _read_table(ROOT / EULER_CASES_CSV, EULER_COLUMNS)
    sel = [r for r in rows
           if _num(r, "mach") == mach and _num(r, "alpha_deg") == alpha and r["ca_cfd"].strip()]
    if len(sel) != 1:
        raise ValueError(
            f"{EULER_CASES_CSV}: expected one M = {mach:g}, alpha = {alpha:g} row with a "
            f"ca_cfd, found {len(sel)}")
    r = sel[0]
    commit = _last_commit(EULER_CASES_CSV)
    return _row(CA_DALEMBERT, "sample rocket", mach, alpha, "euler", "ca_cfd",
                _num(r, "ca_cfd"), 0.0, None,
                f"{EULER_CASES_CSV} mach={mach:g} alpha_deg={alpha:g} ({r['status']}) @ {commit}")


def _sample_rows(levels_csv) -> list[dict[str, str]]:
    return _read_table(levels_csv, SAMPLE_LEVELS_COLUMNS)


def refined_rows(levels_csv=SAMPLE_LEVELS_CSV) -> list[dict[str, str]]:
    """CA at M = 0.3, alpha = 0 on each level of the refined family, coarse first; no pass/fail."""
    mach, alpha = CA_DALEMBERT_CASE
    rows = _sample_rows(levels_csv)
    out = []
    for name in FAMILY_LEVELS:
        sel = [r for r in rows if r["level"] == name and _num(r, "mach") == mach]
        if len(sel) != 1:
            raise ValueError(f"{levels_csv}: expected one row for level {name} at M = {mach:g}, found {len(sel)}")
        r = sel[0]
        wall = _num(r, "wall_size_mm")
        out.append(_row(
            CA_DALEMBERT_REFINED, f"{name} ({wall:g} mm)", mach, alpha, "euler", "ca_cfd",
            _num(r, "ca_alpha0"), 0.0, None,
            f"{SAMPLE_LEVELS_REL} level={name} mach={mach:g} ({r['status_a0']}) nodes={r['nodes']}"))
    return out


def nomirror_antisymmetry_rows(levels_csv=SAMPLE_LEVELS_CSV) -> list[dict[str, str]]:
    """Even part of the solved +4 / -4 pair on the medium level meshed without the z mirror."""
    out = []
    for r in _sample_rows(levels_csv):
        if r["level"] != NOMIRROR_LEVEL:
            continue
        plus, minus = _num(r, "cn_raw_a4"), _num(r, "cn_raw_am4")
        out.append(_row(
            CN_ANTISYMMETRY, "medium (z_mirror_mesh=false)", _num(r, "mach"), 4.0, "euler",
            "cn_even_4deg", antisymmetry(plus, minus), 0.0,
            _antisymmetry_tolerance((plus - minus) / 2.0),
            f"{SAMPLE_LEVELS_REL} level={NOMIRROR_LEVEL} mach={_num(r, 'mach'):g} "
            f"({r['status_a4']}/{r['status_am4']}); +4 and -4 both solved on a mesh that is not mirrored"))
    if not out:
        raise ValueError(f"{levels_csv}: no rows for level {NOMIRROR_LEVEL}")
    return out


def _offset_sentence(levels_csv) -> str:
    """The alpha = 0 normal force of the non-mirrored level next to the even part it explains."""
    parts = [f"M = {_num(r, 'mach'):g}: cn_raw(0) = {_num(r, 'cn_raw_a0'):.4g}"
             for r in _sample_rows(levels_csv) if r["level"] == NOMIRROR_LEVEL]
    return "; ".join(parts)


def _shrink_sentence(refined: list[dict[str, str]]) -> str:
    vals = [abs(float(r["value"])) for r in refined]
    text = " -> ".join(f"{v:.4g}" for v in vals)
    shrinks = all(b < a for a, b in zip(vals, vals[1:]))
    verb = "shrinks" if shrinks else "does not shrink"
    return (f"|CA| {verb} under refinement at M = 0.3, alpha = 0 (coarse to fine, |ca_cfd|): {text}.")


def _header(refined: list[dict[str, str]], offsets: str) -> list[str]:
    lines = [
        "Consistency checks of spec section 6.3, read from the committed CFD results; no CFD was run for this file.",
        "This is verification, not validation: it asks whether relations physics requires hold in the data.",
        "Source files and the last commit that changed each:",
    ]
    for rel in (ASYMMETRY_CSV, EULER_CASES_CSV):
        lines.append(f"  {rel} @ {_last_commit(rel)}")
    lines += [
        "cn_zero_alpha and cn_antisymmetry rows from asymmetry_study.csv share one tolerance, 1 % of",
        "  |cn_odd_4deg| of the same row: cn_odd_4deg = (CN(+4) - CN(-4))/2 is the mirror-combined",
        "  normal force, the scale that the alpha = 0 and the 4 deg offsets spoil.",
        "  passed is |value| <= tolerance.",
        "cn_zero_alpha has one row per configuration and Mach of asymmetry_study.csv (base, roll45, jst,",
        "  tight, zm; 15 rows); value is that file's cn0 column.",
        "asymmetry_study.csv values are rounded to 5 decimals in the source, so a value shown as 0 here",
        "  means |x| < 5e-6, not exactly 0; the values are kept as written there.",
        "zm is the quarter-plus-mirror mesh: CN(0) = 0 by construction (mesh symmetry), so the zm",
        "  cn_zero_alpha rows pass by construction and test nothing about the solver.",
        "The zm cn_antisymmetry rows are near zero partly by construction too: on the mirrored mesh,",
        "  -alpha is the exact mirror of +alpha.",
        "cn_zero_alpha also has one row per Mach for the production runs of sample_euler_cases.csv:",
        "  the alpha_deg = 0 row, value = full-precision cn_raw. Their tolerance is 1 % of |cn_raw| of",
        "  the same Mach's alpha_deg = 4 row (cn_raw at alpha_deg = 4); a Mach with no alpha_deg = 4 row",
        "  gets blank tolerance and passed. That file's header says the production mesh is z-symmetric",
        "  (alpha_mode mirror), so these rows are zero by mesh symmetry up to solver round-off.",
        "cn_antisymmetry uses every +4 / -4 pair. asymmetry_study.csv gives only the already-combined",
        "  cn_even_4deg = (CN(+4) + CN(-4))/2, not the separate +4 and -4 values, so the value is the",
        "  CSV's number and antisymmetry() is not re-run on it; each such row's source says",
        "  cn_even_4deg combined value.",
        "ca_dalembert has tolerance and passed blank: whether |CA(M=0.3, alpha=0)| shrinks as the grid is",
        "  refined is read from the refined Euler runs (ca_dalembert_refined below), not decided by a threshold, so no pass/fail",
        "  is claimed. The expected value is 0 (d'Alembert's paradox).",
        "ca_dalembert_refined has one row per level of the refined Euler grid family (coarse, medium, fine) from",
        f"  {SAMPLE_LEVELS_REL}, whose own header records the runs; value is that level's ca_cfd, tolerance and passed are blank.",
        "  " + _shrink_sentence(refined),
        "  The grids are not geometrically similar (see gci_sample_euler.csv), so this is a trend over three grids, not an order of convergence.",
        "cn_antisymmetry also has one row per Mach for the medium level meshed without the z mirror (z_mirror_mesh=false):",
        "  +4 and -4 are both solved there, so the even part is a test of the solver; value is (cn_raw(+4) + cn_raw(-4))/2",
        "  from the levels file, tolerance 1 % of |(cn_raw(+4) - cn_raw(-4))/2|, passed is |value| <= tolerance.",
        "  The even part of a pair is the spurious normal force that a mesh without z symmetry carries at every incidence, so it is of the size of that mesh's alpha = 0 value,",
        "  here (levels file, cn_raw_a0): " + offsets + ".",
        "  A solve with status accepted (the runner's acceptance rule was not met) enters the pair as written; its status is in source.",
        "The farfield mass-flux balance check of spec section 6.3 is not in this file; it belongs to Task 19.",
        "passed is the string True or False, or blank when the check has no criterion.",
    ]
    return ["# " + ln for ln in lines]


def main(out: pathlib.Path | str | None = None, levels_csv=None) -> pathlib.Path:
    out = pathlib.Path(out) if out is not None else RESULTS
    levels_csv = pathlib.Path(levels_csv) if levels_csv is not None else SAMPLE_LEVELS_CSV
    refined = refined_rows(levels_csv)
    rows = cn_rows() + [ca_dalembert_row()] + refined + nomirror_antisymmetry_rows(levels_csv)
    out.parent.mkdir(parents=True, exist_ok=True)
    with out.open("w", newline="") as f:
        for ln in _header(refined, _offset_sentence(levels_csv)):
            f.write(ln + "\n")
        w = csv.DictWriter(f, fieldnames=COLUMNS, lineterminator="\n")
        w.writeheader()
        w.writerows(rows)
    return out


if __name__ == "__main__":
    print(main())
