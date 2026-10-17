# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2024-2026 西濱大将 (NISHIHAMA Daisuke)
"""Compare IgnisYeet coefficient tables of the Basic Finner with the measured data.

Usage:
  python doc/scripts/basic_finner_compare.py <out.csv> "<header line>" <method>=<ignisyeet out dir> ...

Each <ignisyeet out dir> holds the aero_table.csv written by `ignisyeet aero
examples/basic_finner/basic_finner.toml` (any aero.method). The table is read at alpha_t = 0 and
interpolated linearly in Mach to the Mach numbers of DREV-TM-9703 Table IX (and of the converted
NAVORD-6652 Table 2 for Clp, rows "Clp(Regan)", Mach <= the table's mach_max).

Quantities (reference d = 30 mm, S = pi d^2 / 4, moments about x_cg = 0.165 m = 5.50 cal):
  CNa      cna (1/rad)
  Cma      -CNa (xcp - x_cg) / d (1/rad)
  xcp      xcp / d (cal from the nose tip)
  CX0      ca_off (no motor)
  Clp      clp (per p d/(2V))
  Cmq      -2 K / d^2, K = S2 - 2 x_cg S1 + x_cg^2 S0 (damping sums, per q d/(2V))
  Cmq+lag  Cmq + 4 (x_cg L0 - L1) / d^2: adds the apparent-mass lag (alpha-dot) and the convective rotation
           term of the flight model, each 2 (x_cg L0 - L1) / d^2, to compare with the free-flight Cmq + Cmalphadot
diff_pct = (model - measured) / |measured| * 100.
"""

import csv
import sys
from pathlib import Path

import numpy as np

D = 0.030
XCG = 0.165
HERE = Path(__file__).resolve().parents[1] / "data" / "validation" / "basic_finner"


def measured():
    rows = [r for r in csv.DictReader(line for line in open(HERE / "measured.csv") if not line.startswith("#"))]
    ix = [r for r in rows if r["table"] in ("IX", "IX (derived)")]
    regan = [r for r in rows if r["table"] == "2 (converted)" and r["quantity"] == "Clp"]
    return ix, regan


def table_at_zero(out):
    t = np.genfromtxt(Path(out) / "aero_table.csv", delimiter=",", names=True)
    t = t[t["alpha_deg"] == 0]
    return t[np.argsort(t["mach"])]


def model_values(t, m):
    def f(col):
        return float(np.interp(m, t["mach"], t[col]))

    cna, xcp = f("cna"), f("xcp")
    k = f("damp_s2") - 2 * XCG * f("damp_s1") + XCG**2 * f("damp_s0")
    cmq = -2 * k / D**2
    lag = 4 * (XCG * f("lag_l0") - f("lag_l1")) / D**2
    return {
        "CNa": cna,
        "Cma": -cna * (xcp - XCG) / D,
        "xcp": xcp / D,
        "CX0": f("ca_off"),
        "Clp": f("clp"),
        "Cmq": cmq,
        "Cmq+lag": cmq + lag,
    }


def main():
    out_csv, header = sys.argv[1], sys.argv[2]
    methods = [a.split("=", 1) for a in sys.argv[3:]]
    ix, regan = measured()
    lines = []
    for name, out in methods:
        t = table_at_zero(out)
        for r in ix:
            q = r["quantity"]
            if q not in ("CNa", "Cma", "xcp", "CX0", "Clp", "Cmq"):
                continue
            m = float(r["mach"])
            mv = model_values(t, m)
            meas = float(r["value"])
            pe = r["probable_error_pct"]
            for qq in ([q, "Cmq+lag"] if q == "Cmq" else [q]):
                v = mv[qq]
                lines.append([name, f"{m:g}", qq, f"{meas:g}", pe, f"{v:.4g}", f"{(v - meas) / abs(meas) * 100:.1f}"])
        for r in regan:
            m = float(r["mach"])
            if m > t["mach"].max():
                continue
            v = model_values(t, m)["Clp"]
            meas = float(r["value"])
            lines.append([name, f"{m:g}", "Clp(Regan)", f"{meas:g}", r["probable_error_pct"], f"{v:.4g}",
                          f"{(v - meas) / abs(meas) * 100:.1f}"])
    with open(out_csv, "w", newline="") as fh:
        fh.write(header.rstrip("\n") + "\n")
        w = csv.writer(fh, lineterminator="\n")
        w.writerow(["method", "mach", "q", "measured", "probable_error_pct", "model", "diff_pct"])
        w.writerows(lines)


if __name__ == "__main__":
    main()
