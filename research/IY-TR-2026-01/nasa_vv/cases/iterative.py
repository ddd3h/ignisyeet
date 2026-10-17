# SPDX-License-Identifier: GPL-3.0-or-later
"""Iterative error against the grid uncertainty (spec section 6.2).

The point of a convergence-depth study is not that the residual fell but that what is left is small
next to the grid uncertainty. For every iterative-error measure that the committed history files and
the Task 14 work directory support, this module writes the error as a percent of the converged value
and its ratio to the GCI of the matching condition::

    iterative_error_pct = 100 |value_at_stop - value_converged| / |value_converged|
    ratio               = iterative_error_pct / gci_fine_pct

A ratio below 0.1 means the iterative error does not disturb the grid study; a ratio above 1.0 means
the iterative error exceeds the grid uncertainty and that GCI cannot be trusted for the condition.

The ratio is formed only when a GCI number exists for the matching condition. A GCI that was refused,
that is oscillatory or that does not converge under refinement has no number; ``gci_fine_pct`` and
``ratio`` are then blank and the note says why, never 0.

No CFD is run. Everything is read from committed CSVs and from the Task 14 work directory
($HOME/.cache/nasa_vv_sample, one sub-directory per level). Collecting is deterministic: the same
inputs give a byte-identical file. Every path is resolved from this file, not from the working
directory.
"""
from __future__ import annotations

import argparse
import csv
import json
import math
import pathlib
import re
from dataclasses import dataclass

from . import sample_euler as se
from .existing_data import ROOT, _last_commit, _num, _read_table

RESULTS = pathlib.Path(__file__).resolve().parents[1] / "results"
OUT_CSV = RESULTS / "iterative.csv"
GCI_EXISTING = RESULTS / "gci_existing.csv"
GCI_SAMPLE = RESULTS / "gci_sample_euler.csv"
LEVELS_CSV = RESULTS / "sample_euler_levels.csv"
DEFAULT_WORK = se.DEFAULT_WORK

DEPTH_CSV = "doc/data/cfd/convergence_depth.csv"
FLATPLATE_CSV = "doc/data/cfd/rans_flatplate.csv"
FIELDS_DIR = "doc/data/cfd/fields"
PROVENANCE_JSON = FIELDS_DIR + "/provenance.json"
REL = lambda p: str(pathlib.Path(p).relative_to(ROOT))  # noqa: E731

COLUMNS = ["case", "mach", "alpha_deg", "model", "quantity", "value_at_stop", "value_converged",
           "iterative_error_pct", "gci_fine_pct", "ratio", "grid_match", "quantity_match", "source", "h_basis", "note"]
"""The brief's columns, with grid_match and quantity_match before source; h_basis and note are appended."""

NO_DISTURBANCE = 0.1
INVALIDATES = 1.0
FIELDS_STOP_LOG10 = -6.0
"""The residual at which the earlier acceptance rule stopped (log10 rms[Rho])."""

DEPTH_REFERENCE = ("freestream start, continued", "residual 1e-9")
"""The deepest run of convergence_depth.csv: SU2 restarted from the freestream start to 1e-9."""

FIELDS_CASES = (  # (history file stem, mach, alpha, quantity, history column)
    ("m0.800_a00.00", 0.8, 0.0, "ca_alpha0", "CFx"),
    ("m2.000_a04.00", 2.0, 4.0, "cn_a4", "CFz"),
    ("m2.000_a00.00", 2.0, 0.0, "ca_alpha0", "CFx"),
)
"""Fields histories that convergence_depth.csv does not already cover (it holds M 0.8, alpha 4)."""

LEVEL_QUANTITIES = (  # (alpha, quantity, history column, GCI quantity, GCI condition format)
    (4.0, "cn_a4", "CFz", "cna", "M={m:g} alpha=4 (raw)"),
    (0.0, "ca_alpha0", "CFx", "ca_alpha0", "M={m:g} alpha=0"),
)

# ------------------------------------------------------------------ arithmetic


def iterative_error_pct(v_stop: float, v_conv: float) -> float:
    """Relative difference of the value at the stop from the converged value, in percent (a magnitude)."""
    if v_conv == 0.0:
        raise ValueError("the converged value is 0; a relative iterative error is undefined")
    return 100.0 * abs(v_stop - v_conv) / abs(v_conv)


def ratio(error_pct: float, gci_pct: float | None) -> float | None:
    """Iterative error over GCI, or None where there is no GCI number to divide by (blank or 0)."""
    if gci_pct is None or gci_pct == 0.0:
        return None
    return error_pct / gci_pct


# ------------------------------------------------------------------ the GCI lookup


@dataclass(frozen=True)
class GciRef:
    """The GCI of one h_basis for a condition. ``gci_fine_pct`` is None where the GCI has no number."""
    h_basis: str
    gci_fine_pct: float | None
    reason: str
    """Why the GCI has no number (the GCI row's own note); empty when there is one."""
    flag: str
    """The GCI row's own reliability warning (observed order outside 0.5..4); empty when there is none."""
    gci_coarse_pct: float | None = None


def _float(s: str) -> float | None:
    s = s.strip()
    return float(s) if s else None


def find_gci(rows: list[dict[str, str]], study: str, condition: str, quantity: str) -> list[GciRef]:
    """The GCI rows of a study, condition and quantity, one reference per h_basis in file order."""
    out = []
    for r in rows:
        if r["study"] != study or r["condition"] != condition or r["quantity"] != quantity:
            continue
        gci = _float(r["gci_fine_pct"])
        note = r.get("note", "").split(" [")[0].strip()
        flagged = note.startswith("unusable") or "not reliable" in note
        if gci is None:
            reason = note or "the GCI row carries no gci_fine_pct"
        elif flagged:
            # the GCI file marks this number unusable or outside the asymptotic range: it is treated like a refused GCI
            reason = f"the GCI row has a number ({gci:.6g} %) that the GCI file flags: {note}"
            gci = None
        else:
            reason = ""
        flag = ""
        out.append(GciRef(r["h_basis"], gci, reason, flag, _float(r.get("gci_coarse_pct", ""))))
    if not out:
        raise LookupError(f"no GCI row for study {study!r}, condition {condition!r}, quantity {quantity!r}")
    return out


# ------------------------------------------------------------------ rows


def _f(x: float, spec: str) -> str:
    return format(x + 0.0, spec)


def _plain(s: str) -> str:
    """Commas out of the identifying columns, so that ``column -s, -t`` lines the table up; the last column keeps them."""
    return s.replace(", ", "; ").replace(",", ";")


def build_rows(*, case: str, mach: float, alpha_deg: float, model: str, quantity: str, v_stop: float,
               v_conv: float, refs: list[GciRef], grid_match: str, source: str, note: str = "",
               extra_note=None, quantity_match: str = "same") -> list[dict[str, str]]:
    """One row per h_basis of the matching GCI; one row only when no basis has a GCI number.

    ``extra_note(ref, error_pct)`` may add a sentence to the note of a row (it may return '').
    """
    err = iterative_error_pct(v_stop, v_conv)
    base = {"case": _plain(case), "mach": f"{mach:g}", "alpha_deg": f"{alpha_deg:g}", "model": model,
            "quantity": quantity, "value_at_stop": _f(v_stop, ".10g"), "value_converged": _f(v_conv, ".10g"),
            "iterative_error_pct": _f(err, ".6g"), "grid_match": _plain(grid_match), "quantity_match": _plain(quantity_match), "source": _plain(source)}

    def one(ref: GciRef | None, reason: str) -> dict[str, str]:
        r = dict(base)
        parts = [note] if note else []
        rat = ratio(err, ref.gci_fine_pct) if ref else None
        r["h_basis"] = ref.h_basis if ref else ""
        r["gci_fine_pct"] = _f(ref.gci_fine_pct, ".8g") if ref and ref.gci_fine_pct is not None else ""
        r["ratio"] = _f(rat, ".6g") if rat is not None else ""
        if reason:
            parts.append("no ratio, the matching GCI has no number: " + reason)
        elif ref and ref.gci_fine_pct == 0.0:
            parts.append("no ratio: the matching GCI is 0")
        if ref and ref.gci_fine_pct is not None:
            if ref.flag:
                parts.append("the GCI itself is flagged: " + ref.flag)
            if ref.gci_fine_pct > 100.0:
                parts.append("the GCI exceeds 100 % of the value, so it does not constrain the quantity and a small "
                             "ratio says nothing about disturbance")
            if extra_note:
                add = extra_note(ref, err)
                if add:
                    parts.append(add)
        r["note"] = "; ".join(parts)
        return r

    if all(ref.gci_fine_pct is None for ref in refs):
        reasons = []
        for ref in refs:
            if ref.reason not in reasons:
                reasons.append(ref.reason)
        return [one(None, " | ".join(reasons))]
    return [one(ref, ref.reason) for ref in refs]


# ------------------------------------------------------------------ judgement


def row_name(row: dict[str, str]) -> str:
    name = f"{row['case']}; {row['quantity']}; M={row['mach']}"
    return name + (f"; h_basis {row['h_basis']}" if row.get("h_basis") else "")


@dataclass
class Judgement:
    above_one: list[tuple[str, float]]
    """Ratio above 1.0 on a matched row (same grid and same quantity): the iterative error invalidates that GCI."""
    above_one_indicative: list[tuple[str, float]]
    """Ratio above 1.0 on a row whose grid or quantity differs from the GCI's: indicative only."""
    above_tenth: list[tuple[str, float]]
    below_tenth: list[tuple[str, float]]
    blank: list[str]


def is_matched(row: dict[str, str]) -> bool:
    """Same grid and same quantity as the GCI."""
    return row.get("grid_match", "same grid").startswith("same") and row.get("quantity_match", "same").startswith("same")


def judge(rows: list[dict[str, str]]) -> Judgement:
    j = Judgement([], [], [], [], [])
    for r in rows:
        name = row_name(r)
        if r["ratio"] == "":
            j.blank.append(name)
            continue
        v = float(r["ratio"])
        if v > INVALIDATES:
            (j.above_one if is_matched(r) else j.above_one_indicative).append((name, v))
        elif v >= NO_DISTURBANCE:
            j.above_tenth.append((name, v))
        else:
            j.below_tenth.append((name, v))
    return j


def judgement_lines(j: Judgement) -> list[str]:
    lines = ["Judgement: a ratio below 0.1 means the iterative error does not disturb the grid study; a ratio above 1.0 "
             "on a matched row (same grid and same quantity) means the iterative error is larger than the grid "
             "uncertainty, so that GCI is not valid for the condition. A ratio on a row whose grid or quantity differs "
             "is indicative only."]
    if j.above_one:
        lines.append(f"(a) Matched rows with a ratio above 1.0, where the ratio invalidates that GCI ({len(j.above_one)} rows): "
                     + "; ".join(f"{n} (ratio {v:.3g})" for n, v in j.above_one) + ".")
    else:
        lines.append("(a) Matched rows with a ratio above 1.0, where the ratio invalidates that GCI: none.")
    if j.above_one_indicative:
        lines.append(
            f"(b) Indicative rows with a ratio above 1.0 ({len(j.above_one_indicative)} rows): "
            + "; ".join(f"{n} (ratio {v:.3g})" for n, v in j.above_one_indicative)
            + ". Plainly: the CD at the acceptance stop differs from the final CD by more than the Cf grid uncertainty, "
              "so the published plate's stop rule leaves an iterative error larger than its grid uncertainty would "
              "suggest. This is indicative because the quantities differ (the iterative error is that of the plate CD, "
              "the GCI that of cf_x097); it does not invalidate the Cf GCI. No committed source holds a GCI for the "
              "plate CD itself (gci_existing.csv and layer0_gci.csv carry cf_x097 and other quantities only), so no "
              "matched pairing exists for these rows.")
    else:
        lines.append("(b) Indicative rows with a ratio above 1.0: none.")
    if j.above_tenth:
        lines.append(f"Rows with a ratio from 0.1 to 1.0 ({len(j.above_tenth)} rows): "
                     + "; ".join(f"{n} (ratio {v:.3g})" for n, v in j.above_tenth) + ".")
    else:
        lines.append("Rows with a ratio from 0.1 to 1.0: none.")
    lines.append(f"Rows with a ratio below 0.1: {len(j.below_tenth)}. Rows without a ratio (the matching GCI has no number): "
                 f"{len(j.blank)}.")
    return lines


# ------------------------------------------------------------------ history files


def _read_history(path: pathlib.Path) -> tuple[list[str], list[list[float]]]:
    """A SU2 history.csv as (header, rows); an incomplete or non-finite file is an error."""
    text = pathlib.Path(path).read_text()
    if not text.endswith("\n"):
        raise ValueError(f"{path} does not end in a newline (truncated)")
    lines = list(csv.reader(text.splitlines()))
    head = [h.strip().strip('"') for h in lines[0]]
    rows = []
    for n, r in enumerate(lines[1:], start=2):
        if len(r) != len(head):
            raise ValueError(f"{path} row {n} has {len(r)} of {len(head)} fields (truncated)")
        v = [float(x) for x in r]
        if not all(math.isfinite(x) for x in v):
            raise ValueError(f"{path} row {n} has a non-finite field")
        rows.append(v)
    if not rows:
        raise ValueError(f"{path} has no rows")
    return head, rows


# ------------------------------------------------------------------ convergence_depth.csv and the fields histories


def _cells(text: str, what: str) -> int:
    m = re.search(r"(\d+) cells", text)
    if not m:
        raise ValueError(f"{what}: no '<n> cells' in the mesh description")
    return int(m.group(1))


def _fine_level(levels_csv: pathlib.Path) -> dict[str, str]:
    rows = [r for r in _read_table(levels_csv, ("level", "cells", "wall_size_mm", "fin_size_mm")) if r["level"] == "fine"]
    if not rows:
        raise ValueError(f"{levels_csv}: no fine level")
    return rows[0]


def grid_match_euler(cells: int, fine: dict[str, str]) -> str:
    """'same grid' only when the cell count of the iterative-error mesh equals the fine level of the GCI family."""
    if cells == int(fine["cells"]):
        return "same grid"
    return (f"different grid (fine level of the sample_euler family: {fine['cells']} cells at wall_size "
            f"{fine['wall_size_mm']} mm and fin_size {fine['fin_size_mm']} mm; this mesh has {cells} cells "
            "at the same settings)")


def depth_rows(path, gci_rows, fine, source) -> list[dict[str, str]]:
    path = pathlib.Path(path)
    cells = _cells(path.read_text().splitlines()[0], str(path))
    table = _read_table(path, ("start", "criterion", "cn"))
    ref = [r for r in table if (r["start"], r["criterion"]) == DEPTH_REFERENCE]
    if len(ref) != 1:
        raise ValueError(f"{path}: expected exactly one reference row {DEPTH_REFERENCE}, found {len(ref)}")
    v_conv = _num(ref[0], "cn")
    refs = find_gci(gci_rows, "sample_euler", "M=0.8 alpha=4 (raw)", "cna")
    gm = grid_match_euler(cells, fine)
    out = []
    for r in table:
        if r is ref[0]:
            continue
        out += build_rows(
            case=f"convergence_depth: {r['start']}, {r['criterion']}", mach=0.8, alpha_deg=4.0, model="Euler",
            quantity="cn_a4", v_stop=_num(r, "cn"), v_conv=v_conv, refs=refs, grid_match=gm, source=source,
            note=(f"value_converged is the deepest run of the file ({DEPTH_REFERENCE[0]}, {DEPTH_REFERENCE[1]}); "
                  "cn_a4 is CN(4 deg) of the wall marker, the quantity of cna = CN(4)/radians(4) up to a constant, "
                  "so the relative errors are the same"))
    return out


def fields_rows(fields_dir, provenance, gci_rows, fine, source) -> list[dict[str, str]]:
    fields_dir = pathlib.Path(fields_dir)
    cells = _cells(json.loads(pathlib.Path(provenance).read_text())["source"], str(provenance))
    gm = grid_match_euler(cells, fine)
    out = []
    for stem, mach, alpha, quantity, col in FIELDS_CASES:
        head, rows = _read_history(fields_dir / f"history_{stem}.csv")
        ir, ii, ic = head.index("rms[Rho]"), head.index("Inner_Iter"), head.index(col)
        stop = next((i for i, r in enumerate(rows) if r[ir] <= FIELDS_STOP_LOG10), None)
        if stop is None:
            raise ValueError(f"{stem}: rms[Rho] never reached {FIELDS_STOP_LOG10}")
        last = len(rows) - 1
        gq, gc = next((q[3], q[4]) for q in LEVEL_QUANTITIES if q[1] == quantity)
        refs = find_gci(gci_rows, "sample_euler", gc.format(m=mach), gq)
        out += build_rows(
            case=(f"fields history: freestream start, residual 1e-6 stop (iteration {int(rows[stop][ii])}) "
                  f"against iteration {int(rows[last][ii])}"),
            mach=mach, alpha_deg=alpha, model="Euler", quantity=quantity, v_stop=rows[stop][ic],
            v_conv=rows[last][ic], refs=refs, grid_match=gm, source=source,
            note=(f"the stop is the earlier residual-1e-6 rule, not the current acceptance; iteration "
                  f"{int(rows[last][ii])} is not converged either (log10 rms[Rho] {rows[last][ir]:.2f}), so the error is "
                  f"against the end of a {len(rows)}-row run"))
    return out


# ------------------------------------------------------------------ rans_flatplate.csv


def flatplate_rows(path, gci_rows, source) -> list[dict[str, str]]:
    table = _read_table(path, ("mach", "grid", "cells", "cd_plate", "cd_at_stop_vs_final_percent"))
    out = []
    for r in table:
        mach = _num(r, "mach")
        same = [x for x in table if _num(x, "mach") == mach]
        cells = sorted((_num(x, "cells") for x in same), reverse=True)
        mine = _num(r, "cells")
        pct = _num(r, "cd_at_stop_vs_final_percent")
        v_conv = _num(r, "cd_plate")
        v_stop = v_conv * (1.0 + pct / 100.0)
        refs = find_gci(gci_rows, "rans_flatplate", f"M={mach:g}", "cf_x097")
        if mine == cells[0]:
            gm = "same grid (the finest grid of the GCI triplet)"
        else:
            gm = f"different grid (the GCI refers to the finest grid {int(cells[0])} cells; this row is {r['grid']})"

        def extra(ref, err, mine=mine, cells=cells, grid=r["grid"]):
            if mine == cells[1] and ref.gci_coarse_pct is not None and ref.gci_coarse_pct != 0.0:
                return (f"gci_coarse_pct of this triplet, the GCI of the {grid} grid itself, is "
                        f"{ref.gci_coarse_pct:.6g}, ratio to it {err / ref.gci_coarse_pct:.4g}")
            return ""

        out += build_rows(
            case=f"rans_flatplate {r['grid']}", mach=mach, alpha_deg=0.0, model="RANS (SA-neg)", quantity="cd_plate",
            v_stop=v_stop, v_conv=v_conv, refs=refs, grid_match=gm, source=source, extra_note=extra,
            quantity_match="different (iterative error of cd_plate; GCI of cf_x097)",
            note=("iterative error of the plate CD (cd_at_stop_vs_final_percent: CD at the iteration where the product's "
                  "acceptance rule would stop against the last iteration) but the GCI is of cf_x097, a different "
                  "quantity on the same grid, so the ratio is indicative only because the quantities differ (no committed "
                  "GCI exists for cd_plate); value_at_stop = cd_plate x "
                  "(1 + cd_at_stop_vs_final_percent / 100)"))
    return out


# ------------------------------------------------------------------ the Task 14 levels


def _case_dir(level_dir: pathlib.Path, mach: float, alpha: float) -> pathlib.Path:
    for cd in sorted((level_dir / "out" / "cfd").iterdir()):
        dj = cd / "done.json"
        if dj.exists():
            j = json.loads(dj.read_text())
            if float(j["mach"]) == mach and float(j["alpha_deg"]) == alpha and float(j.get("beta_deg", 0.0)) == 0.0:
                return cd
    raise FileNotFoundError(f"{level_dir}: no case directory for mach {mach:g} alpha {alpha:g}")


def level_rows(levels, work, gci_rows, source) -> list[dict[str, str]]:
    """The rows written after the judged row of each solve of the family levels, as an iterative-error measure.

    ``value_at_stop`` is the value the runner recorded at its stop decision (the judged row);
    ``value_converged`` is the last row SU2 wrote. That row is not a converged value, so the error is the change
    over the rows written after the stop and a lower bound of the iterative error.
    """
    work = pathlib.Path(work)
    out = []
    for name in se.FAMILY_LEVELS:
        lv = levels[name]
        if lv.problems:
            raise ValueError(f"level {name!r} cannot be trusted: " + "; ".join(lv.problems))
        for mach in lv.machs():
            for alpha, quantity, col, gq, gc in LEVEL_QUANTITIES:
                s = lv.solves.get((mach, alpha))
                if s is None or s.history_gap is None:
                    continue
                recorded = s.cn_raw if quantity == "cn_a4" else s.ca
                if recorded is None:
                    continue
                head, rows = _read_history(_case_dir(work / name, mach, alpha) / "history.csv")
                judged = len(rows) - s.history_gap - 1
                v_judged, v_last = rows[judged][head.index(col)], rows[-1][head.index(col)]
                if abs(v_judged - recorded) > 1e-4 * abs(recorded) + 1e-8:
                    raise ValueError(f"{name} mach {mach:g} alpha {alpha:g}: the {col} of history row {judged + 1} "
                                     f"({v_judged:.8g}) is not the recorded value ({recorded:.8g}); the work directory "
                                     "does not match the committed levels")
                refs = find_gci(gci_rows, "sample_euler", gc.format(m=mach), gq)
                gm = ("same grid" if name == "fine" else
                      f"different grid (the {name} level; gci_fine_pct belongs to the fine level of the family)")
                out += build_rows(
                    case=f"sample_euler {name} level; rows after the judged row", mach=mach, alpha_deg=alpha,
                    model="Euler", quantity=quantity, v_stop=recorded, v_conv=v_last, refs=refs, grid_match=gm,
                    source=source,
                    note=(f"{s.history_gap} rows written after the judged row; value_converged is the last of them "
                          f"(log10 rms[Rho] {rows[-1][head.index('rms[Rho]')]:.2f}), not a converged value, so this error "
                          f"is a lower bound; solve status {s.status}; cn_a4 is CN(4 deg), the quantity of cna = "
                          "CN(4)/radians(4) up to a constant"))
    return out


# ------------------------------------------------------------------ driver


def _header(rows, depth_cells, fine) -> list[str]:
    j = judge(rows)
    srcs = [DEPTH_CSV, FLATPLATE_CSV, PROVENANCE_JSON,
            *sorted(f"{FIELDS_DIR}/history_{s}.csv" for s, *_ in FIELDS_CASES),
            REL(GCI_EXISTING), REL(GCI_SAMPLE), REL(LEVELS_CSV)]
    lines = [
        "Iterative error against the grid uncertainty (spec section 6.2). No CFD was run for this file: every number is read from the committed files below or from the Task 14 work directory.",
        "Source files and the last commit that changed each:",
        *[f"  {s} @ {_last_commit(s)}" for s in srcs],
        "  Task 14 work directory $HOME/.cache/nasa_vv_sample/<level>/out/cfd/<case>/history.csv (levels coarse, medium, fine; medium_nomirror is not a family member and has no row); its judged rows and history_gap_* are those of sample_euler_levels.csv, and a history whose judged row does not match the recorded value is refused.",
        "iterative_error_pct = 100 |value_at_stop - value_converged| / |value_converged|; ratio = iterative_error_pct / gci_fine_pct. gci_fine_pct is in percent of the fine-grid value, as in the GCI files.",
        "A GCI is paired with an iterative error only for the same body, condition and quantity. grid_match says whether the grid is the same and quantity_match whether the quantity is ('same' or 'different (<which>)'): where either differs, the ratio is indicative only and the judgement below keeps those rows apart from the matched ones.",
        "A matching GCI without a number (refused, oscillatory, not converging under refinement) or whose number the GCI file flags (a note starting 'unusable' or saying the GCI is not reliable) is treated as refused: it leaves gci_fine_pct and ratio blank and the note gives the GCI row's own reason; the ratio is never 0 for lack of a GCI.",
        "h_basis is the h basis of the GCI row (nodes^-1/3 and wall_size for the Euler sample rocket, cells^-1/2 for the flat plate); a condition with two bases has two rows, and the spread between them is part of the uncertainty. When no basis has a number there is one row with h_basis blank. note is appended after source.",
        "value_converged: convergence_depth.csv rows use its deepest run (freestream start continued to residual 1e-9, CN 1.20218); the fields histories use their last iteration (300), which is not converged either, and their M = 2 rows (convergence_depth.csv has no M = 2) compare the earlier residual-1e-6 stop rule against iteration 300; the Task 14 level rows use the last row SU2 wrote after the stop, which is not converged, so their error is a lower bound of the iterative error; rans_flatplate rows have value_at_stop = cd_plate x (1 + cd_at_stop_vs_final_percent / 100) and value_converged = cd_plate (the last iteration).",
        f"Nearest matching family for the Euler sample rocket: results/gci_sample_euler.csv (Task 14), the fine level of which has the settings of convergence_depth.csv and the fields histories (wall_size 2 mm, fin_size 1 mm) but another mesh ({fine['cells']} cells against {depth_cells} cells), so grid_match reads 'different grid' for those rows. The rans_mesh_study rows of gci_existing.csv are not used: they are a different mesh series (3, 4.8 and 8 mm wall_size) and are refused anyway.",
        "convergence_depth.csv holds M = 0.8, alpha = 4 deg only; M = 2 enters through the fields histories (freestream start, residual 1e-6 stop, the earlier rule, against iteration 300). The M = 0.8, alpha = 4 deg history of fields/ is not repeated: its iteration-102 value 1.195565 is the 'freestream start (iteration 102)' row of convergence_depth.csv (1.19556).",
        "rans_flatplate: the iterative error is that of the plate CD (cd_at_stop_vs_final_percent) and the GCI is that of cf_x097, the quantity of the flat-plate rows of gci_existing.csv, which has no iterative-error record; the ratio pairs two quantities on the same grids, quantity_match reads 'different' on these rows, and the ratio is indicative only; no committed source holds a GCI for the plate CD (gci_existing.csv and layer0_gci.csv carry cf_x097 and other quantities). The GCI was built from the final-iteration cf_x097 (5.8 to 8.1 orders of residual drop on the six grids), so this ratio describes a run stopped by the product's acceptance rule, not the inputs of the GCI. The coarse and middle grids are paired with the GCI of the fine grid (gci_fine_pct); the middle grid's own GCI is gci_coarse_pct, given in its note.",
        "Task 14 levels: history_gap_* counts the rows SU2 wrote after the stop. value_at_stop is the recorded value (the judged row), value_converged the last row. Pairing with the family GCI: the fine level is the grid gci_fine_pct belongs to; the coarse and medium levels are inputs of the same family and read 'different grid'. CN(4 deg) of the wall marker is paired with cna = CN(4)/radians(4) (the same relative error); ca_alpha0 with ca_alpha0. The ca_alpha0 GCIs of M = 0.3 and 0.8 and the wall_size cna GCI of M = 2 are flagged by the GCI file, so those rows are blank; only the nodes^-1/3 cna GCI of M = 2 (observed order 0.53) carries a usable number.",
        "Layer-0 plate (results/layer0_plate.csv): no row. Its header records that window_spread is empty on every row and that the history holds no per-iteration value of the compared quantities (only residuals and integrated force coefficients); the iterative convergence it records is the residual drop, which is not an iterative error in percent.",
        *judgement_lines(j),
    ]
    return ["# " + ln for ln in lines]


def collect(work, gci_existing=None, gci_sample=None, levels_csv=None) -> list[dict[str, str]]:
    work = pathlib.Path(work)
    if not work.is_dir():
        raise FileNotFoundError(f"work directory {work} not found: the Task 14 histories are needed for the level rows")
    existing = _read_table(gci_existing or GCI_EXISTING, ("study", "condition", "quantity", "h_basis", "gci_fine_pct"))
    sample = _read_table(gci_sample or GCI_SAMPLE, ("study", "condition", "quantity", "h_basis", "gci_fine_pct"))
    fine = _fine_level(levels_csv or LEVELS_CSV)
    levels = se.read_levels(work)
    rows = (depth_rows(ROOT / DEPTH_CSV, sample, fine, f"{DEPTH_CSV} @ {_last_commit(DEPTH_CSV)}; GCI results/gci_sample_euler.csv @ {_last_commit(REL(GCI_SAMPLE))}")
            + fields_rows(ROOT / FIELDS_DIR, ROOT / PROVENANCE_JSON, sample, fine,
                          f"{FIELDS_DIR}/history_*.csv @ {_last_commit(FIELDS_DIR)}; GCI results/gci_sample_euler.csv @ {_last_commit(REL(GCI_SAMPLE))}")
            + flatplate_rows(ROOT / FLATPLATE_CSV, existing, f"{FLATPLATE_CSV} @ {_last_commit(FLATPLATE_CSV)}; GCI results/gci_existing.csv @ {_last_commit(REL(GCI_EXISTING))}")
            + level_rows(levels, work, sample, f"$HOME/.cache/nasa_vv_sample history.csv, results/sample_euler_levels.csv @ {_last_commit(REL(LEVELS_CSV))}; GCI results/gci_sample_euler.csv @ {_last_commit(REL(GCI_SAMPLE))}"))
    return rows


def main(out=None, work=None) -> pathlib.Path:
    out = pathlib.Path(out) if out is not None else OUT_CSV
    work = pathlib.Path(work) if work is not None else DEFAULT_WORK
    rows = collect(work)
    fine = _fine_level(LEVELS_CSV)
    depth_cells = _cells(( ROOT / DEPTH_CSV).read_text().splitlines()[0], DEPTH_CSV)
    out.parent.mkdir(parents=True, exist_ok=True)
    with out.open("w", newline="") as f:
        for ln in _header(rows, depth_cells, fine):
            f.write(ln + "\n")
        w = csv.DictWriter(f, fieldnames=COLUMNS, lineterminator="\n")
        w.writeheader()
        w.writerows(rows)
    return out


if __name__ == "__main__":
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--work", default=str(DEFAULT_WORK), help="the Task 14 work directory")
    ap.add_argument("--out", default=str(OUT_CSV))
    a = ap.parse_args()
    print(main(a.out, a.work))
