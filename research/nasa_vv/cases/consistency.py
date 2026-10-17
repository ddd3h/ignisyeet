# SPDX-License-Identifier: GPL-3.0-or-later
"""The consistency checks of spec section 6.3, read from the committed CFD results.

This is verification: it checks that relations physics requires hold in the data
IgnisYeet already produced. No CFD is run here.

    cn_zero_alpha    CN(alpha = 0) = 0 on the z-symmetric rows, where the mesh
                     symmetry leaves only the discretisation offset.
    cn_antisymmetry  CN(-alpha) = -CN(alpha): the even part of the +4 / -4 pair is 0.
    ca_dalembert     CA at M = 0.3, alpha = 0 is 0 (d'Alembert's paradox).
    ca_dalembert_refined  the same CA on the refined Euler grid family of Task 14,
                     which does not exist yet, so those rows are omitted.

``antisymmetry(cn_plus, cn_minus)`` is the even part ``(cn_plus + cn_minus) / 2``.
The two CN checks share one tolerance, 1 % of ``|cn_odd_4deg|`` of the same
configuration: that is the mirror-combined normal force ``(CN(+4) - CN(-4)) / 2``,
the scale the alpha = 0 and the 4 deg offsets spoil. No tolerance is invented for
``ca_dalembert``: whether it shrinks with the grid is decided by Task 14, so its
``tolerance`` and ``passed`` are blank.

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

ASYMMETRY_COLUMNS = ("config", "mach", "cn0", "cn_even_4deg", "cn_odd_4deg")
EULER_COLUMNS = ("mach", "alpha_deg", "status", "ca_cfd")

COLUMNS = ["check", "case", "mach", "alpha_deg", "model", "quantity",
           "value", "expected", "tolerance", "passed", "source"]

CN_ZERO_ALPHA = "cn_zero_alpha"
CN_ANTISYMMETRY = "cn_antisymmetry"
CA_DALEMBERT = "ca_dalembert"

Z_SYMMETRIC = "zm"
"""Config key of the z-symmetric (quarter + mirror) mesh in asymmetry_study.csv."""

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
    """CN(0) = 0 on the z-symmetric rows, and the even part of every +4 / -4 pair."""
    rows = _read_table(ROOT / ASYMMETRY_CSV, ASYMMETRY_COLUMNS)
    commit = _last_commit(ASYMMETRY_CSV)
    zero_alpha, antisymmetric = [], []
    for r in rows:
        config, mach = r["config"], _num(r, "mach")
        tolerance = _antisymmetry_tolerance(_num(r, "cn_odd_4deg"))
        where = f"{ASYMMETRY_CSV} config={config} mach={mach:g} @ {commit}"
        if config == Z_SYMMETRIC:
            zero_alpha.append(_row(
                CN_ZERO_ALPHA, config, mach, 0.0, "euler", "cn0",
                _num(r, "cn0"), 0.0, tolerance, where))
        antisymmetric.append(_row(
            CN_ANTISYMMETRY, config, mach, 4.0, "euler", "cn_even_4deg",
            _num(r, "cn_even_4deg"), 0.0, tolerance,
            where + " (cn_even_4deg is the CSV's already-combined (CN(+4)+CN(-4))/2;"
                    " the CSV gives no separate +4 and -4 values; antisymmetry() is"
                    " not re-run on it)"))
    return zero_alpha + antisymmetric


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


def _header() -> list[str]:
    lines = [
        "Consistency checks of spec section 6.3, read from the committed CFD results; no CFD was run for this file.",
        "This is verification, not validation: it asks whether relations physics requires hold in the data.",
        "Source files and the last commit that changed each:",
    ]
    for rel in (ASYMMETRY_CSV, EULER_CASES_CSV):
        lines.append(f"  {rel} @ {_last_commit(rel)}")
    lines += [
        "cn_zero_alpha and cn_antisymmetry share one tolerance, 1 % of |cn_odd_4deg| of the same",
        "  configuration: cn_odd_4deg = (CN(+4) - CN(-4))/2 is the mirror-combined normal force, the",
        "  scale that the alpha = 0 and the 4 deg offsets spoil. passed is |value| <= tolerance.",
        "cn_zero_alpha uses the z-symmetric rows of asymmetry_study.csv, where the mesh symmetry makes the",
        "  true CN(0) exactly 0; value is that file's cn0 column.",
        "cn_antisymmetry uses every +4 / -4 pair. asymmetry_study.csv gives only the already-combined",
        "  cn_even_4deg = (CN(+4) + CN(-4))/2, not the separate +4 and -4 values, so the value is the",
        "  CSV's number and antisymmetry() is not re-run on it; each row's source says so.",
        "ca_dalembert has tolerance and passed blank: whether |CA(M=0.3, alpha=0)| shrinks as the grid is",
        "  refined is decided by the refined Euler runs of Task 14, not by a threshold here, so no pass/fail",
        "  is claimed. The expected value is 0 (d'Alembert's paradox).",
        "ca_dalembert_refined rows are omitted until Task 14's refined Euler grid family exists.",
        "The farfield mass-flux balance check of spec section 6.3 is not in this file; it belongs to Task 19.",
        "passed is the string True or False, or blank when the check has no criterion.",
    ]
    return ["# " + ln for ln in lines]


def main(out: pathlib.Path | str | None = None) -> pathlib.Path:
    out = pathlib.Path(out) if out is not None else RESULTS
    rows = cn_rows() + [ca_dalembert_row()]
    out.parent.mkdir(parents=True, exist_ok=True)
    with out.open("w", newline="") as f:
        for ln in _header():
            f.write(ln + "\n")
        w = csv.DictWriter(f, fieldnames=COLUMNS, lineterminator="\n")
        w.writeheader()
        w.writerows(rows)
    return out


if __name__ == "__main__":
    print(main())
