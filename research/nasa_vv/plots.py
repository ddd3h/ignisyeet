# SPDX-License-Identifier: GPL-3.0-or-later
"""Validation figure: CNa and CX0 of the Basic Finner against the measured data.

Comparison with the measured data (DREV-TM-9703 Table IX) is validation; the figure
compares every CFD point with the measured value only; no difference between CFD models is
computed or drawn. Two kinds of uncertainty are
drawn and they must not be confused:

- the measured probable error (``error_bar_spec``). A blank cell is an unknown error
  and gets no bar; 0 is tabulated as "below 0.5 %" (doc/source/validation.rst:580) and
  gets a 0.5 % bound, drawn differently from a measured error (a hollow diamond with
  triangles beside it; the bound is smaller than the symbol). doc/scripts/make_plots.py:1610
  uses ``fillna(0.0)`` and so draws a zero-length bar for an unknown error.
- the grid sensitivity of the CFD value (``band_for``), only where a row of
  results/gci_existing.csv belongs to the plotted point.

What gci_existing.csv says about the Basic Finner mesh check (its ``#`` header): the
grids are not geometrically similar, the L1-L3 and L2-L4 orders disagree, and the study
"does not establish a reliable GCI". The box is therefore hatched and unfilled, labelled
as grid sensitivity, and never drawn as a solid band. The RANS GCI does not exist yet;
a source of established GCIs is added as a row of ``BAND_RULES``.

Which GCI quantity belongs to which plotted quantity (doc/scripts/basic_finner_cfd.py):

- CNa = ``cn_raw`` at alpha = 2 deg divided by 2 deg in rad (lines 51, 103, 105, 107), and
  the mesh check's ``cn`` is the same ``cn_raw`` (line 177). The relative GCI of ``cn`` at
  alpha = 2 carries over to CNa unchanged.
- CX0 = ``ca_cfd`` at alpha = 0 plus the friction, base and fin trailing-edge terms of
  the Barrowman build-up (line 110, docs at lines 31-33). The mesh check's
  ``ca_pressure`` is only the ``ca_cfd`` part (line 178), so its relative GCI is not a
  relative GCI of CX0 and no CX0 box is drawn.
- The plotted Euler run is the 1.5 mm, 586094-cell run of cfd_euler.csv (``fairing``,
  Mach 2.375); it is level L2 of the mesh check, with the same cn (0.32852). It is the
  middle grid f2 of the L1-L3 triplet, so the GCI of that grid is ``gci_coarse_pct``
  (relative to f2), not ``gci_fine_pct`` (relative to the finest grid of the triplet).
"""
from __future__ import annotations

import argparse
import csv
import math
import textwrap
from dataclasses import dataclass
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = Path(__file__).resolve().parents[2]
SUMMARY_CSV = ROOT / "doc" / "data" / "validation" / "basic_finner" / "summary_vs_measured.csv"
EULER_CSV = ROOT / "doc" / "data" / "validation" / "basic_finner" / "cfd_euler.csv"
MESH_CHECK_CSV = ROOT / "doc" / "data" / "validation" / "basic_finner" / "mesh_check.csv"
GCI_CSV = HERE / "results" / "gci_existing.csv"
DEFAULT_OUT = HERE / "figures" / "validation_bands.svg"

BOUND_FRACTION = 0.005  # a tabulated probable error of 0 means "below 0.5 %"


@dataclass(frozen=True)
class ErrorBar:
    half_height: float
    bound: bool  # True: tabulated 0, drawn as the 0.5 % bound and not as a measured error


def error_bar_spec(probable_error_pct: str | float | None, value: float) -> ErrorBar | None:
    """Half height of the bar for ``value``; None where the probable error is unknown."""
    if probable_error_pct is None:
        return None
    if isinstance(probable_error_pct, str):
        text = probable_error_pct.strip()
        if text == "":
            return None
    else:
        text = probable_error_pct
    try:
        pct = float(text)
    except (TypeError, ValueError):
        raise ValueError(f"probable error {probable_error_pct!r} is not a number") from None
    if not math.isfinite(pct):
        raise ValueError(f"probable error {probable_error_pct!r} is not finite")
    if pct < 0:
        raise ValueError(f"probable error {pct} is negative")
    if pct == 0:
        return ErrorBar(abs(value) * BOUND_FRACTION, True)
    return ErrorBar(abs(value) * pct / 100.0, False)


@dataclass(frozen=True)
class BandRule:
    """A plotted point (series, quantity) takes its band from rows of one study."""
    series: str          # column of summary_vs_measured.csv
    quantity: str        # plotted quantity
    study: str           # ``study`` column of gci_existing.csv
    condition: str       # ``condition`` column: configuration and angle of attack
    gci_quantity: str    # ``quantity`` column
    gci_column: str      # which GCI applies to the plotted grid
    established: bool    # False: grid sensitivity only, drawn hatched and unfilled


# Data, not code: an established GCI (the RANS one, after Task 15) is another row here.
# The Euler CNa rule is the only one the existing grid study supports (see the module docstring).
BAND_RULES = (
    BandRule("euler", "CNa", "basic_finner_mesh_check", "fairing alpha=2", "cn", "gci_coarse_pct", False),
)

# Source CSV of each study whose rows carry no Mach column.
STUDY_MACH_SOURCES = {"basic_finner_mesh_check": MESH_CHECK_CSV}


@dataclass(frozen=True)
class Band:
    low_pct: float
    high_pct: float
    n_rows: int
    established: bool


def _data_lines(path: Path):
    with open(path, newline="") as fh:
        yield from (line for line in fh if not line.startswith("#"))


def read_gci_rows(path: Path = GCI_CSV) -> list[dict]:
    return list(csv.DictReader(_data_lines(path)))


def read_study_machs(sources: dict[str, Path] = STUDY_MACH_SOURCES) -> dict[str, set[float]]:
    out = {}
    for study, path in sources.items():
        out[study] = {float(r["mach"]) for r in csv.DictReader(_data_lines(path))}
    return out


def band_for(rows, series: str, quantity: str, mach: float, rules=BAND_RULES,
             study_machs: dict[str, set[float]] | None = None) -> Band | None:
    """Envelope of the GCI over every matching row (both h_basis, both triplets); None without one.

    A row matches on the study, the condition (configuration and alpha) and the
    quantity of the rule, and the study must have been run at this Mach number.
    Rows without a number (a refused or non-converging triplet) add nothing.
    """
    if study_machs is None:
        study_machs = read_study_machs()
    for rule in rules:
        if (rule.series, rule.quantity) != (series, quantity):
            continue
        if not any(math.isclose(mach, m, abs_tol=1e-9) for m in study_machs.get(rule.study, ())):
            continue
        values = []
        for r in rows:
            if (r["study"], r["condition"], r["quantity"]) != (rule.study, rule.condition, rule.gci_quantity):
                continue
            cell = (r.get(rule.gci_column) or "").strip()
            if cell:
                values.append(float(cell))
        if values:
            return Band(min(values), max(values), len(values), rule.established)
    return None


@dataclass(frozen=True)
class Change:
    """Largest observed relative change of the plotted quantity from the plotted grid to another level."""
    pct: float
    plotted_level: str
    other_level: str


def observed_change(series: str, quantity: str, mach: float, rules=BAND_RULES, run_csv: Path = EULER_CSV,
                    mesh_csv: Path = MESH_CHECK_CSV) -> Change | None:
    """The plotted run is found in the mesh check by its cell count (same configuration, Mach, alpha).

    The GCI box covers the triplets that give a number; this is the plain change of the
    quantity between levels, whether or not their triplet converges.
    """
    for rule in rules:
        if (rule.series, rule.quantity) != (series, quantity):
            continue
        config, alpha_text = rule.condition.split()
        alpha = float(alpha_text.split("=")[1])

        def same(r):
            return (r["base_treatment"] == config and math.isclose(float(r["mach"]), mach, abs_tol=1e-9)
                    and math.isclose(float(r["alpha_deg"]), alpha, abs_tol=1e-9))

        runs = [r for r in csv.DictReader(_data_lines(run_csv)) if same(r)]
        levels = [r for r in csv.DictReader(_data_lines(mesh_csv)) if same(r)]
        if len(runs) != 1 or len(levels) < 2:
            return None
        plotted = [r for r in levels if r["cells"] == runs[0]["cells"]]
        if len(plotted) != 1:
            return None
        v0 = float(plotted[0][rule.gci_quantity])
        others = [(abs(float(r[rule.gci_quantity]) - v0) / abs(v0) * 100.0, r["level"]) for r in levels if r is not plotted[0]]
        pct, level = max(others)
        return Change(pct, plotted[0]["level"], level)
    return None


def _read_summary(path: Path) -> dict[str, list[dict]]:
    out: dict[str, list[dict]] = {}
    for r in csv.DictReader(_data_lines(path)):
        out.setdefault(r["q"], []).append(r)
    for rs in out.values():
        rs.sort(key=lambda r: float(r["mach"]))
    return out


def _num(cell: str) -> float | None:
    return float(cell) if cell.strip() else None


FOOTNOTE = (
    "Comparison with the measured free-flight data (DREV-TM-9703, Table IX) is validation; every CFD point is compared with "
    "the measured value only, and no difference between CFD models is computed or drawn. Measured bars are the tabulated "
    "probable error. A hollow diamond with triangles beside it is a tabulated probable error of 0, drawn as the 0.5 % bound "
    "(smaller than the symbol), not as a measured error; a point with no bar has no probable error tabulated. "
    "Hatched box: grid sensitivity of the Euler CNa at Mach 2.375 (fairing) from the Basic Finner mesh check, the envelope "
    "of the GCI of the plotted grid over both h definitions (cells^-1/3, wall_size) and every triplet that gives a number "
    "({n} rows); it is not an established GCI: the grids are not geometrically similar and the observed orders disagree "
    "(results/gci_existing.csv). Its extent, ±{hi:.2g} % of the value, is too small to see at this scale, and the box "
    "understates the sensitivity: {change} "
    "Mapping: CNa = CN(alpha = 2 deg)/alpha of the same runs, so the relative GCI of cn carries over. CX0 = ca_pressure + "
    "modelled friction, base and fin trailing-edge terms, so the ca_pressure GCI is not a CX0 GCI: no box on CX0. "
    "No box on any other Mach number (no mesh study there), nor on RANS (no RANS GCI yet)."
)


def validation_figure(out: Path = DEFAULT_OUT, summary_csv: Path = SUMMARY_CSV, gci_csv: Path = GCI_CSV,
                      rules=BAND_RULES) -> None:
    import matplotlib

    matplotlib.use("Agg")
    import matplotlib.pyplot as plt
    from matplotlib.lines import Line2D
    from matplotlib.patches import Patch, Rectangle
    from matplotlib.transforms import offset_copy

    summary = _read_summary(summary_csv)
    gci_rows = read_gci_rows(gci_csv)
    machs = read_study_machs()

    ink, blue, orange, grey = "#222222", "#0072B2", "#D55E00", "#6b6b6b"
    plt.rcParams.update({"svg.fonttype": "path", "svg.hashsalt": "nasa-vv", "font.size": 10})
    fig, axs = plt.subplots(1, 2, figsize=(11.0, 5.6))
    fig.subplots_adjust(left=0.07, right=0.985, top=0.80, bottom=0.37, wspace=0.22)
    cfd = [("euler", "Euler (SU2)", blue, "o"), ("rans", "RANS (SU2), base term from the model", orange, "s"),
           ("rans_base_cfd", "RANS (SU2), CFD base pressure", orange, "^")]
    used_bands: list[Band] = []
    changes: list[Change] = []

    for ax, (q, ylabel) in zip(axs, [("CNa", r"$C_{N\alpha}$ [1/rad]"), ("CX0", r"$C_{X0}$")]):
        rows = summary[q]
        for r in rows:
            m, v = float(r["mach"]), float(r["measured"])
            bar = error_bar_spec(r["probable_error_pct"], v)
            hollow = bar is not None and bar.bound
            ax.plot(m, v, "D", color=ink, mfc="white" if hollow else ink, ms=5.5, mew=1.2, zorder=5)
            if bar is None:
                continue
            if bar.bound:
                # the bound is smaller than the symbol: triangles sit 8 pt outside it, at the bound's ends
                for sign, mk in ((1, "^"), (-1, "v")):
                    ax.plot([m], [v + sign * bar.half_height], marker=mk, color=ink, mfc="white", ms=5, mew=1.1, zorder=4,
                            transform=offset_copy(ax.transData, fig=fig, y=sign * 8, units="points"))
            else:
                ax.errorbar(m, v, yerr=bar.half_height, fmt="none", ecolor=ink, elinewidth=1.2, capsize=3.5, zorder=4)
        for key, _, col, mk in cfd:
            pts = [(float(r["mach"]), _num(r[key])) for r in rows if _num(r[key]) is not None]
            if q != "CX0" and key == "rans_base_cfd":
                continue
            if not pts:
                continue
            series = "rans" if key.startswith("rans") else key
            for m, v in pts:
                ax.plot(m, v, mk, color=col, mfc=col if key == "euler" else "white", mew=1.5, ms=7, zorder=6)
                band = band_for(gci_rows, series, q, m, rules=rules, study_machs=machs)
                if band is None:
                    continue
                used_bands.append(band)
                half = band.high_pct / 100.0 * abs(v)
                ax.add_patch(Rectangle((m - 0.07, v - half), 0.14, 2 * half, fill=band.established, facecolor=col,
                                       hatch=None if band.established else "//////", edgecolor=col, lw=1.0, zorder=3))
                ch = observed_change(series, q, m, rules=rules)
                text = f"grid sensitivity ±{band.high_pct:.2g} % (not a GCI)"
                if ch is not None:
                    changes.append(ch)
                    text += f"\n{ch.plotted_level} to {ch.other_level} changes cn by {ch.pct:.2g} %:\nmore than the box"
                ax.annotate(text, xy=(m, v), xytext=(28, 34), textcoords="offset points", fontsize=8, color=col,
                            arrowprops={"arrowstyle": "-", "color": col, "lw": 0.8})
        ax.set_xlabel("Mach number")
        ax.set_ylabel(ylabel)
        ax.grid(True, color="#dddddd", lw=0.6)
        ax.set_axisbelow(True)
        ax.margins(y=0.12)
        ax.set_title(q, loc="left", fontsize=11)

    handles = [
        Line2D([], [], marker="D", ls="-", color=ink, ms=5, lw=1.2, label="measured, probable error as tabulated"),
        Line2D([], [], marker="D", ls="", color=ink, mfc="white", mew=1.2, ms=5.5,
               label="measured, tabulated 0: hollow diamond, triangles at the 0.5 % bound"),
        Line2D([], [], marker="D", ls="", color=ink, ms=5, label="measured, no probable error tabulated (no bar)"),
        Patch(facecolor="none", edgecolor=blue, hatch="//////", label="grid sensitivity (not an established GCI)"),
        Line2D([], [], marker="o", ls="", color=blue, ms=7, label="CFD, Euler"),
        Line2D([], [], marker="s", ls="", color=orange, mfc="white", mew=1.5, ms=7, label="CFD, RANS (base term from the model)"),
        Line2D([], [], marker="^", ls="", color=orange, mfc="white", mew=1.5, ms=7, label="CFD, RANS (CFD base pressure)"),
    ]
    fig.legend(handles=handles, loc="upper center", ncol=2, frameon=False, fontsize=9, bbox_to_anchor=(0.5, 0.995))
    hi = max((b.high_pct for b in used_bands), default=0.0)
    n = max((b.n_rows for b in used_bands), default=0)
    ch = max(changes, key=lambda c: c.pct) if changes else None
    change = (f"{ch.plotted_level} to {ch.other_level} changes cn by {ch.pct:.2g} %, more than the box (that triplet does not "
              f"converge, so it gives no GCI). " if ch else "")
    note = FOOTNOTE.format(n=n, hi=hi, change=change)
    if not used_bands:
        note = note.split("Hatched box:")[0] + "No CFD point has a grid-sensitivity box: no row of the existing grid study belongs to it."
    fig.text(0.07, 0.015, "\n".join(textwrap.wrap(note, 140)), ha="left", va="bottom", fontsize=8, color=grey)
    out = Path(out)
    out.parent.mkdir(parents=True, exist_ok=True)
    fig.savefig(out, format=out.suffix.lstrip(".") or "svg", metadata={"Date": None} if out.suffix == ".svg" else None)


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--out", type=Path, default=DEFAULT_OUT)
    validation_figure(ap.parse_args().out)


if __name__ == "__main__":
    main()
