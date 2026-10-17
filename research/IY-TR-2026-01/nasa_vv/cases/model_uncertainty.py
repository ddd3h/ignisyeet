# SPDX-License-Identifier: GPL-3.0-or-later
"""Model uncertainty, Euler against RANS (spec 6.6), set against the numerical uncertainty of each.

IgnisYeet offers two flow models, Euler and RANS with the Spalart-Allmaras turbulence model
(crates/cfd/src/config.rs:12-18, crates/cfd/src/su2cfg.rs:212). The NASA tutorial's step on turbulence models asks for
several; the product cannot answer it, which is a limit of scope. This module answers the part the product can:
how far the RANS result lies from the Euler result for the same body and condition, and whether that difference is
larger than the two grid-convergence bands. A model difference inside the numerical uncertainty is not a measured
model difference, so the table records that judgement in ``separable``.

This is cross-comparison, model against model. It is not validation (no measured data enters) and not verification.

One row per (case, condition, quantity), ``difference = rans - euler`` and ``difference_pct = 100 difference / euler``
(the Euler value is the base, as in the ``cna_diff_percent`` column of rans_vs_euler.csv).

    sample_rocket   M = 0.8 and 2.0, alpha = 4, CN and xcp [m]        doc/data/cfd/rans_vs_euler.csv
    basic_finner    M = 1.832 (flat base), alpha = 2: CN, xcp [cal]   doc/data/validation/basic_finner/cfd_rans.csv
                    alpha = 0: the axial coefficient                   against cfd_euler.csv

The axial coefficient of the Basic Finner is the one quantity whose definition differs between the two columns, on
purpose: the Euler value is ``cx0`` of cfd_euler.csv (the CFD wall pressure plus the Barrowman friction, base and
fin-trailing-edge terms, because Euler has no friction and the product adds them), and the RANS value is
``ca_wall + ca_base_cfd + ca_roughness`` of cfd_rans.csv (all of it from the CFD). The difference is what a user of the
two product modes would see; it mixes the model and the Barrowman base term. The sample rocket's axial force is not
compared: rans_vs_euler.csv holds the RANS wall force without the base, and the Euler pressure drag at another angle.

Numerical bands
---------------
``euler_gci_pct`` and ``rans_gci_pct`` are GCI values (percent of the value) read from the grid-convergence CSVs of
results/ when they exist: gci_existing.csv, gci_sample_euler.csv (Task 14) and gci_basic_finner_rans.csv (Task 15).
Rows of a CSV are matched on study, Mach number, angle and quantity (parsed from ``condition``), never on the text.

- GCI is a property of a level. The band of a production value is read off the level the production value was computed
  on: ``gci_fine_pct`` (relative to f1) if it is the finest level of the triplet, ``gci_coarse_pct`` (relative to f2) if
  it is the middle one; the coarsest level has no GCI. The level is found by name in ``triplet``.
- Every quantity is computed under two grid-size definitions (``h_basis``) in the new studies. The band is the larger of
  the two; the spread between them is part of the uncertainty.
- A GCI is missing, and then so is ``separable``, when no row exists, when a row is refused, when a row carries the
  "not reliable" flag (observed order outside 0.5 to 4), or when no GCI was computed (oscillatory convergence). This
  holds if only one of the two bases is affected. A missing GCI is never written as 0, and ``separable`` is never
  False without both bands. The note says which GCI is missing and why.
- The relative GCI of CN equals the one of CNa (CNa is CN divided by a constant angle). The Euler band of the sample
  rocket's CN is the one of CNa = CN(4)/radians(4) of the raw (non-combined) solve, which differs from the combined
  CN(4) - CN(0) of rans_vs_euler.csv by CN(0), zero on the z-symmetric mesh up to round-off.
- ``separable`` is True when ``abs(difference_pct)`` exceeds ``euler_gci_pct + rans_gci_pct``. The bands are relative to
  each model's own value; for the differences here (percent level) the distinction is second order.

What the data can say now
-------------------------
Every ``rans_mesh_study`` row of gci_existing.csv is refused (levels not converged or without convergence evidence), so
the sample rocket's RANS band does not exist and the verdict of those rows stays blank whatever Task 14 delivers. The
only Euler grid study of the Basic Finner (gci_existing.csv, basic_finner_mesh_check) is at M = 2.375, with fairing and flat
bases, not at M = 1.832, and no task produces another; the Euler band of those rows is missing for good.

Run it with

    cd research && uv run --project nasa_vv python -m nasa_vv.cases.model_uncertainty

It reads the optional GCI files if they are present, so rerunning it after Tasks 14 and 15 are merged needs no change.
"""
from __future__ import annotations

import csv
import pathlib
import re
from dataclasses import dataclass

from .existing_data import ROOT, _g, _last_commit, _num, _read_table

RESULTS_DIR = pathlib.Path(__file__).resolve().parents[1] / "results"
RESULT = RESULTS_DIR / "model_uncertainty.csv"

RANS_VS_EULER_CSV = "doc/data/cfd/rans_vs_euler.csv"
FINNER_RANS_CSV = "doc/data/validation/basic_finner/cfd_rans.csv"
FINNER_EULER_CSV = "doc/data/validation/basic_finner/cfd_euler.csv"

GCI_FILES = ("gci_existing.csv", "gci_sample_euler.csv", "gci_basic_finner_rans.csv")

COLUMNS = ["case", "mach", "alpha_deg", "quantity", "euler", "rans", "difference", "difference_pct",
           "euler_gci_pct", "rans_gci_pct", "separable", "source", "note"]

COMPARISON_COLUMNS = ("mach", "rans_cn_a4", "euler_cn_a4", "rans_xcp_m", "euler_xcp_m")
FINNER_RANS_COLUMNS = ("mach", "alpha_deg", "status", "cn", "xcp_cal", "ca_wall", "ca_base_cfd", "ca_roughness")
FINNER_EULER_COLUMNS = ("base_treatment", "mach", "alpha_deg", "status", "cn", "xcp_cal", "cx0")

SAMPLE_MACHS = (0.8, 2.0)
SAMPLE_ALPHA = 4.0
FINNER_MACH = 1.832


# ------------------------------------------------------------------ the judgement


def separable(difference_pct: float, euler_gci_pct: float | None, rans_gci_pct: float | None) -> bool | None:
    """True if |difference| exceeds the sum of the two bands; None when a band is missing (no verdict)."""
    if euler_gci_pct is None or rans_gci_pct is None:
        return None
    return abs(difference_pct) > euler_gci_pct + rans_gci_pct


# ------------------------------------------------------------------ reading a GCI out of the grid-study CSVs


@dataclass(frozen=True)
class GciRef:
    """Which GCI row belongs to a production value.

    ``production`` is the name, in the ``triplet`` of the GCI file, of the level the production value was computed on."""
    study: str
    mach: float
    alpha: float
    quantity: str
    production: str


def _condition_numbers(condition: str) -> tuple[float | None, float | None]:
    m = re.search(r"\bM=([0-9]*\.?[0-9]+)", condition)
    a = re.search(r"\balpha=(-?[0-9]*\.?[0-9]+)", condition)
    return (float(m.group(1)) if m else None, float(a.group(1)) if a else None)


def _same_level(name: str, want: str) -> bool:
    """Level names are equal as text or as '<number> mm' (1.5 mm and 1.50 mm are one level)."""
    if name.strip() == want.strip():
        return True
    pat = re.compile(r"^\s*([0-9]*\.?[0-9]+)\s*mm\s*$")
    a, b = pat.match(name), pat.match(want)
    return bool(a and b and abs(float(a.group(1)) - float(b.group(1))) < 1e-9)


def _short(note: str) -> str:
    """A refusal note without the per-level detail in square brackets (that stays in the GCI file)."""
    return note.split(" [")[0].strip()


def pick_gci(rows: list[dict[str, str]], ref: GciRef) -> tuple[float | None, str]:
    """(GCI in percent, note) of the production level of ``ref``; the value is None, with the reason, if it is missing."""
    want = f"study {ref.study}, M = {ref.mach:g}, alpha = {ref.alpha:g}, quantity {ref.quantity}"
    match = []
    for r in rows:
        if r.get("study") != ref.study or r.get("quantity") != ref.quantity:
            continue
        mach, alpha = _condition_numbers(r.get("condition", ""))
        if mach is not None and alpha is not None and abs(mach - ref.mach) < 1e-9 and abs(alpha - ref.alpha) < 1e-9:
            match.append(r)
    if not match:
        return None, f"no row for {want}"
    levels = [s.strip() for s in match[0]["triplet"].split("/")]
    idx = next((i for i, n in enumerate(levels) if _same_level(n, ref.production)), None)
    if idx is None:
        return None, f"production level {ref.production} is not in the triplet {match[0]['triplet']} ({want})"
    if idx == 2:
        return None, f"production level {ref.production} is the coarsest of the triplet, which has no GCI ({want})"
    column = "gci_fine_pct" if idx == 0 else "gci_coarse_pct"
    values, bad = [], []
    for r in match:
        note = r.get("note", "")
        if note.startswith("refused:"):
            bad.append(f"{r['h_basis']}: {_short(note)}")
        elif "not reliable" in note:
            bad.append(f"{r['h_basis']}: {_short(note)}")
        elif r.get(column, "").strip() == "":
            bad.append(f"{r['h_basis']}: no GCI computed" + (f" ({_short(note)})" if note else ""))
        else:
            values.append((float(r[column]), r["h_basis"]))
    if bad:
        return None, "; ".join(bad)
    value, basis = max(values)
    return value, f"{column} of level {ref.production}, the larger of " + " and ".join(b for _, b in values) + (
        f" ({basis})" if len(values) > 1 else "")


def _read_gci_files(results_dir: pathlib.Path) -> tuple[list[dict[str, str]], dict[str, bool]]:
    rows: list[dict[str, str]] = []
    present: dict[str, bool] = {}
    for name in GCI_FILES:
        path = results_dir / name
        present[name] = path.is_file()
        if present[name]:
            lines = [ln for ln in path.read_text().splitlines() if not ln.startswith("#")]
            rows += list(csv.DictReader(lines))
    return rows, present


# ------------------------------------------------------------------ the conditions


@dataclass(frozen=True)
class Quantity:
    case: str
    mach: float
    alpha: float
    name: str
    euler: float
    rans: float
    source: str
    euler_ref: GciRef | None
    rans_ref: GciRef | None
    euler_none: str = ""
    """Why there is no Euler GCI to look up at all (the reason replaces the lookup's)."""


EULER_FINNER_NONE = ("the only Euler grid study of the Basic Finner (gci_existing.csv, basic_finner_mesh_check) is at "
                     "M = 2.375 (with fairing and flat bases), not at M = 1.832; no grid study exists for this condition")


def _row_at(rows: list[dict[str, str]], **keys: float | str) -> dict[str, str]:
    hit = []
    for r in rows:
        ok = True
        for k, v in keys.items():
            ok &= abs(float(r[k]) - v) < 1e-9 if isinstance(v, float) else r[k] == v
        if ok:
            hit.append(r)
    if len(hit) != 1:
        raise ValueError(f"expected one row for {keys}, found {len(hit)}")
    return hit[0]


def quantities(root: pathlib.Path = ROOT) -> list[Quantity]:
    out: list[Quantity] = []
    cmp_rows = _read_table(root / RANS_VS_EULER_CSV, COMPARISON_COLUMNS)
    for m in SAMPLE_MACHS:
        r = _row_at(cmp_rows, mach=m)
        for name, rc, ec, rq, eq, eqcond in (
            ("cn", "rans_cn_a4", "euler_cn_a4", "cn", "cna", "CNa = CN(4)/radians(4) of the raw solve"),
            ("xcp_m", "rans_xcp_m", "euler_xcp_m", "xcp", "xcp", "xcp_m of the combined row"),
        ):
            out.append(Quantity(
                "sample_rocket", m, SAMPLE_ALPHA, name, _num(r, ec), _num(r, rc),
                f"{RANS_VS_EULER_CSV} columns {rc} and {ec}",
                GciRef("sample_euler", m, SAMPLE_ALPHA, eq, "fine"),     # Euler at wall_size 2 mm: the knobs of Task 14's fine level (cell counts differ, see the header)
                GciRef("rans_mesh_study", m, SAMPLE_ALPHA, rq, "3 mm"),  # RANS at 3 mm: the finest of the mesh study
            ))
    rans = _read_table(root / FINNER_RANS_CSV, FINNER_RANS_COLUMNS)
    euler = _read_table(root / FINNER_EULER_CSV, FINNER_EULER_COLUMNS)

    def finner(alpha: float) -> tuple[dict[str, str], dict[str, str]]:
        r = _row_at(rans, mach=FINNER_MACH, alpha_deg=alpha)
        e = _row_at(euler, base_treatment="flat", mach=FINNER_MACH, alpha_deg=alpha)
        for label, row in (("cfd_rans.csv", r), ("cfd_euler.csv", e)):
            if row["status"] != "converged":
                raise ValueError(f"{label}: M = {FINNER_MACH:g}, alpha = {alpha:g} has status {row['status']!r}, not converged")
        return r, e

    r, e = finner(2.0)
    for name, col, rq in (("cn", "cn", "cna"), ("xcp_cal", "xcp_cal", "xcp")):
        out.append(Quantity(
            "basic_finner", FINNER_MACH, 2.0, name, _num(e, col), _num(r, col),
            f"{FINNER_RANS_CSV} and {FINNER_EULER_CSV} (base_treatment flat), column {col}",
            None, GciRef("basic_finner_rans", FINNER_MACH, 2.0, rq, "1.5 mm"), EULER_FINNER_NONE))
    r, e = finner(0.0)
    out.append(Quantity(
        "basic_finner", FINNER_MACH, 0.0, "ca", _num(e, "cx0"),
        _num(r, "ca_wall") + _num(r, "ca_base_cfd") + _num(r, "ca_roughness"),
        f"{FINNER_RANS_CSV} ca_wall + ca_base_cfd + ca_roughness against {FINNER_EULER_CSV} (base_treatment flat) cx0",
        None, GciRef("basic_finner_rans", FINNER_MACH, 0.0, "ca_alpha0", "1.5 mm"), EULER_FINNER_NONE))
    return out


# ------------------------------------------------------------------ the table


def _band(rows: list[dict[str, str]], ref: GciRef | None, none_reason: str) -> tuple[float | None, str]:
    if ref is None:
        return None, none_reason
    return pick_gci(rows, ref)


def build_rows(gci_rows: list[dict[str, str]], present: dict[str, bool], root: pathlib.Path = ROOT) -> list[dict[str, str]]:
    absent = [n for n, ok in present.items() if not ok]
    out = []
    for q in quantities(root):
        diff = q.rans - q.euler
        pct = 100.0 * diff / q.euler
        e_val, e_note = _band(gci_rows, q.euler_ref, q.euler_none)
        r_val, r_note = _band(gci_rows, q.rans_ref, "")
        sep = separable(pct, e_val, r_val)
        notes = []
        for label, val, note in (("euler", e_val, e_note), ("rans", r_val, r_note)):
            notes.append(f"{label} GCI missing: {note}" if val is None else f"{label} GCI: {note}")
        if absent and (e_val is None or r_val is None):
            notes.append(f"GCI files not present: {', '.join(absent)}")
        out.append({
            "case": q.case, "mach": f"{q.mach:g}", "alpha_deg": f"{q.alpha:g}", "quantity": q.name,
            "euler": _g(q.euler, ".8g"), "rans": _g(q.rans, ".8g"),
            "difference": _g(diff, ".8g"), "difference_pct": _g(pct, ".6g"),
            "euler_gci_pct": _g(e_val, ".8g"), "rans_gci_pct": _g(r_val, ".8g"),
            "separable": "" if sep is None else str(sep),
            "source": q.source, "note": "; ".join(notes),
        })
    return out


def _header(present: dict[str, bool]) -> list[str]:
    lines = [
        "Model uncertainty: SU2 Euler against SU2 RANS (Spalart-Allmaras, SA-neg with ft2), set against the numerical uncertainty (GCI) of each. Cross-comparison, model against model; no measured data enters (not validation).",
        "IgnisYeet offers Euler and RANS with the Spalart-Allmaras model only (crates/cfd/src/config.rs:12-18, crates/cfd/src/su2cfg.rs:212 writes KIND_TURB_MODEL= SA). The tutorial's several turbulence models are therefore a limit of scope here; the flat plate is compared with SST on SU2 directly in turbulence_models.csv.",
        "Producer: python -m nasa_vv.cases.model_uncertainty (research/IY-TR-2026-01/nasa_vv/cases/model_uncertainty.py). No CFD is run.",
        "Source files and the last commit that changed each:",
    ]
    for rel in (RANS_VS_EULER_CSV, FINNER_RANS_CSV, FINNER_EULER_CSV):
        lines.append(f"  {rel} @ {_last_commit(rel)}")
    lines.append("Grid-convergence files read for the bands (results/):")
    for name in GCI_FILES:
        lines.append(f"  {name} ({'read' if present[name] else 'absent'})")
    lines += [
        "difference = rans - euler; difference_pct = 100 difference / euler (the Euler value is the base). euler and rans are in the unit of the quantity: CN dimensionless, xcp_m in metres from the nose, xcp_cal in calibres from the nose tip, ca dimensionless.",
        "euler_gci_pct and rans_gci_pct are GCI values in percent of the value (Celik et al. (2008), Fs = 1.25), read from the level the production value was computed on: gci_fine_pct if it is the finest level of the triplet, gci_coarse_pct if the middle one. Where a study has two h_basis rows the larger is used. The relative GCI of CN equals that of CNa.",
        "separable is True when abs(difference_pct) exceeds euler_gci_pct + rans_gci_pct; False when it does not; empty when either GCI is missing. A model difference inside the numerical uncertainty is not a measured model difference. A missing, refused or flagged ('not reliable') GCI is never written as 0 and gives no verdict; the note says which GCI is missing and why.",
        "sample_rocket: RANS at wall_size 3 mm (1844927 nodes), Euler at 2 mm with fin_size auto (release b6a9bf5), both from rans_vs_euler.csv; the two meshes are not the same. CN and xcp are the mirror-combined values. The Euler band, were it available, would be that of Task 14's fine level (wall_size 2 mm, fin_size 1 mm, 1415504 cells); the production Euler value of rans_vs_euler.csv has the same wall_size but 941k cells, so the level is matched by the knob, not shown to be the same mesh. It is the band of CNa = CN(4)/radians(4) of the raw solve.",
        "sample_rocket: every rans_mesh_study triplet of gci_existing.csv is refused (levels recorded as not converged, or without convergence evidence), so the RANS band does not exist for these rows and the verdict stays empty whatever the Euler study delivers. The axial force is not compared: rans_vs_euler.csv holds the RANS wall force without the base and the Euler pressure drag at another angle.",
        "basic_finner: M = 1.832 (the only Mach number at which the RANS converged), flat base for both models, wall_size 1.5 mm. The RANS value is the medium level of Task 15's family (wall_size 2.1 / 1.5 / 1.07 mm), so its band is gci_coarse_pct. CN is at alpha = 2 (CNa = CN/radians(2) has the same relative difference); at alpha = 0 CN is zero and only the axial coefficient is compared.",
        "basic_finner ca: euler is cx0 of cfd_euler.csv (CFD wall pressure with the base excluded, plus the Barrowman friction, base and fin-trailing-edge terms); rans is ca_wall + ca_base_cfd + ca_roughness of cfd_rans.csv (all CFD; ca_roughness is 0 in the file). The difference mixes the flow model with the Barrowman base term and is what the two product modes deliver.",
        "basic_finner: the Euler band does not exist. The only Euler grid study of the Basic Finner (gci_existing.csv, basic_finner_mesh_check) is at M = 2.375 (with fairing and flat bases) and is not borrowed for M = 1.832; no task produces another.",
    ]
    return ["# " + ln for ln in lines]


def main(out: pathlib.Path | str | None = None, results_dir: pathlib.Path | str | None = None) -> pathlib.Path:
    out = pathlib.Path(out) if out is not None else RESULT
    results_dir = pathlib.Path(results_dir) if results_dir is not None else RESULTS_DIR
    gci_rows, present = _read_gci_files(results_dir)
    rows = build_rows(gci_rows, present)
    out.parent.mkdir(parents=True, exist_ok=True)
    with out.open("w", newline="") as f:
        for ln in _header(present):
            f.write(ln + "\n")
        w = csv.DictWriter(f, fieldnames=COLUMNS, lineterminator="\n")
        w.writeheader()
        w.writerows(rows)
    return out


if __name__ == "__main__":
    print(main())
