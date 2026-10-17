# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2024-2026 西濱大将 (NISHIHAMA Daisuke)
"""Collect the CFD runs of the Basic Finner and compare them with the measured data.

Usage:
  python doc/scripts/basic_finner_cfd.py <spec.json>

spec.json:
  {"rev": "<git revision>",
   "barrowman": "<ignisyeet out dir, aero.method = barrowman>",      # aero_drag.csv: friction and base terms
   "panel": "<ignisyeet out dir, aero.method = panel>",
   "euler": {"fairing": {"dir": "<out dir>", "code": {"1.077": "<rev>", "default": "<rev>"}},
             "flat": {"dir": "<out dir>", "code": {"default": "<rev>"}}},   # full Mach x alpha matrix; code: IgnisYeet
                                                                            # revision per Mach number (column ignisyeet)
   "euler_primary": "fairing",                                       # the base treatment used in the summary
   "euler_note": "<text>",                                           # run provenance, header of cfd_euler.csv
   "mesh": [{"label": "L1", "base_treatment": "fairing", "wall_size": 0.0021, "fin_size": 0.00047, "dir": "..."}, ...],
   "mesh_rev": "<rev>",                                              # header of mesh_check.csv
   "roll": "<out dir with roll_damping = true>",                     # optional; merged cfd_report.json
   "roll_note": "<text>",                                            # roll-run settings, header of the summary
   "rans": "<out dir, model = rans>",                                # optional; cfd_cases.csv, cfd_report.json
   "rans_rev": "<text>"}                                             # IgnisYeet revision of the RANS runs

Writes into doc/data/validation/basic_finner/: cfd_euler.csv, mesh_check.csv, cfd_rans.csv (when "rans" is given)
and summary_vs_measured.csv.

Definitions (d = 30 mm, S = pi d^2 / 4, moment reference x_cg = 0.165 m = 5.50 cal):
  CNa      CN(alpha = 2 deg) / (2 deg in rad) of the CFD case (CNa_4deg from alpha = 4 deg in cfd_euler.csv)
  xcp      x_cp / d at alpha = 2 deg (cal from the nose tip)
  Cma      -CNa (x_cp - x_cg) / d
  CX0      Euler: ca_cfd(alpha = 0) (pressure on the wall: nose, body, fins; base excluded)
                  + friction + base of the Barrowman build-up (aero_drag.csv, linear in Mach) + the base drag of the flush fin
                  trailing edges (base_cd(M) n b t / S), as the CFD table does
           RANS:  wall axial force (pressure and friction) + base term; rans_base = "model" adds the Barrowman base
                  term and the flush fin trailing-edge term, rans_base = "cfd" the pressure integrated on the base
  Clp      the CFD roll-damping pair (cfd_report.json, per p d/(2V))
diff_pct = (model - measured) / |measured| x 100.
"""

import csv
import json
import math
import sys
from pathlib import Path

import numpy as np

D = 0.030
XCG = 0.165
HERE = Path(__file__).resolve().parents[1] / "data" / "validation" / "basic_finner"
A2 = math.radians(2.0)
A4 = math.radians(4.0)


def read_csv(path):
    return list(csv.DictReader(line for line in open(path) if not line.startswith("#")))


def measured():
    out = {}
    for r in read_csv(HERE / "measured.csv"):
        if r["table"] in ("IX", "IX (derived)") and r["quantity"] in ("CNa", "Cma", "xcp", "CX0", "Clp"):
            out[(r["quantity"], float(r["mach"]))] = (float(r["value"]), r["probable_error_pct"])
    return out


def drag_terms(barrowman):
    t = np.genfromtxt(Path(barrowman) / "aero_drag.csv", delimiter=",", names=True)
    return lambda m: (float(np.interp(m, t["mach"], t["friction"])), float(np.interp(m, t["mach"], t["base"])))


def fin_te_base(barrowman):
    """Barrowman square trailing-edge base drag of the fins flush with the base: base_cd(M) n b t / S_ref with the
    extracted mean fin thickness (the term IgnisYeet adds to the Euler and rans_base = model tables on STEP surfaces)."""
    g = json.load(open(Path(barrowman) / "geometry.json"))
    f = g["fins"]
    k = f["count"] * f["span"] * f["thickness"] / g["ref_area"]
    return lambda m: (0.12 + 0.13 * m * m if m < 1 else 0.25 / m) * k


def cases(out):
    rows = read_csv(Path(out) / "cfd_cases.csv")
    rep = json.load(open(Path(out) / "cfd_report.json"))
    return rows, rep


def per_mach(rows):
    by = {}
    for r in rows:
        if float(r.get("roll_hat") or 0) != 0 or float(r.get("beta_deg") or 0) != 0:
            continue
        if r["status"] == "failed":
            continue
        by.setdefault(float(r["mach"]), {})[float(r["alpha_deg"])] = r
    return by


def derived(by_alpha, ca0_extra):
    """CNa, xcp (cal), Cma, CX0 at one Mach from the cases at alpha 0, 2 (and 4) deg; None when a case is missing."""
    if 2.0 not in by_alpha or 0.0 not in by_alpha:
        return None
    c2 = by_alpha[2.0]
    cn2 = float(c2["cn_raw"])
    xcp = float(c2["xcp_raw_m"])
    cna = cn2 / A2
    return {
        "CNa": cna,
        "xcp": xcp / D,
        "Cma": -cna * (xcp - XCG) / D,
        "CX0": float(by_alpha[0.0]["ca_cfd"]) + ca0_extra,
    }


def fmt(v, n=5):
    return "" if v is None else f"{v:.{n}g}"


def write(path, header, cols, rows):
    with open(path, "w", newline="") as fh:
        fh.write(header.rstrip("\n") + "\n")
        w = csv.writer(fh, lineterminator="\n")
        w.writerow(cols)
        w.writerows(rows)


def main():
    spec = json.load(open(sys.argv[1]))
    rev = spec["rev"]
    terms = drag_terms(spec["barrowman"])
    te_term = fin_te_base(spec["barrowman"])
    meas = measured()

    # ---- Euler matrix -------------------------------------------------------------------------------------------
    euler_rows, euler_derived = [], {}
    for bt, ent in spec["euler"].items():
        out, code = ent["dir"], ent["code"]
        rows, rep = cases(out)
        cells = rep.get("mesh_cells")
        for r in rows:
            m = float(r["mach"])
            fr, ba = terms(m)
            te = te_term(m)
            a = float(r["alpha_deg"])
            if r["status"] == "failed":
                euler_rows.append([bt, f"{m:g}", f"{a:g}", "failed"] + [""] * 11 + [code.get(f"{m:g}", code["default"])])
                continue
            cn = float(r["cn_raw"])
            euler_rows.append([bt, f"{m:g}", f"{a:g}", r["status"], r["iterations"], r["residual_last"], fmt(cn),
                               fmt(float(r["ca_cfd"])), fmt(float(r["xcp_raw_m"]) / D) if a > 0 else "",
                               fmt(cn / math.radians(a)) if a > 0 else "", fmt(fr), fmt(ba), fmt(te),
                               fmt(float(r["ca_cfd"]) + fr + ba + te) if a == 0 else "", cells, r["wall_seconds"],
                               code.get(f"{m:g}", code["default"])])
        for m, by in per_mach(rows).items():
            fr, ba = terms(m)
            euler_derived[(bt, m)] = derived(by, fr + ba + te_term(m))
    write(HERE / "cfd_euler.csv",
          f"# Basic Finner, SU2 Euler ({spec['euler_note']}; examples/basic_finner/basic_finner.toml with aero.method = cfd, "
          f"surface = step, wall_size 1.5 mm, fin_size auto (0.34 mm = extracted fin thickness / 4), quarter mesh, "
          f"alpha_mode mirror; base_treatment flat and fairing). Generated by doc/scripts/basic_finner_cfd.py. "
          f"cn, ca_pressure: CFD wall integrals (base excluded; the base marker holds the body base and the fin trailing-edge "
          f"faces flush with it); xcp in cal from the nose tip; cna = cn / alpha; "
          f"friction_model, base_model: Barrowman build-up terms added by the CFD table (aero_drag.csv of the barrowman run); "
          f"fin_te_base_model: base drag of the flush fin trailing edges (base_cd(M) n b t / S, t = extracted mean fin thickness); "
          f"cx0 = ca_pressure + friction_model + base_model + fin_te_base_model at alpha 0.",
          ["base_treatment", "mach", "alpha_deg", "status", "iterations", "residual_last", "cn", "ca_pressure", "xcp_cal",
           "cna", "friction_model", "base_model", "fin_te_base_model", "cx0", "cells", "wall_seconds", "ignisyeet"], euler_rows)

    # ---- mesh check ---------------------------------------------------------------------------------------------
    mesh_rows = []
    for lv in spec["mesh"]:
        rows, rep = cases(lv["dir"])
        for r in rows:
            a = float(r["alpha_deg"])
            if abs(float(r["mach"]) - 2.375) > 1e-9:
                continue
            mesh_rows.append([lv["label"], lv["base_treatment"], f"{lv['wall_size'] * 1000:.3g}", f"{lv['fin_size'] * 1000:.3g}",
                              rep.get("mesh_cells"), f"{float(r['mach']):g}", f"{a:g}", r["status"], fmt(float(r["cn_raw"])),
                              fmt(float(r["ca_cfd"])), fmt(float(r["xcp_raw_m"]) / D) if a > 0 else "", r["wall_seconds"]])
    write(HERE / "mesh_check.csv",
          f"# Basic Finner, SU2 Euler mesh check at Mach 2.375 (IgnisYeet {spec['mesh_rev']}, as cfd_euler.csv; wall_size and fin_size "
          f"refined together by a factor 1.4 per level). cn, ca_pressure: CFD wall integrals (base excluded); xcp in cal. "
          f"Generated by doc/scripts/basic_finner_cfd.py.",
          ["level", "base_treatment", "wall_size_mm", "fin_size_mm", "cells", "mach", "alpha_deg", "status", "cn",
           "ca_pressure", "xcp_cal", "wall_seconds"], mesh_rows)

    # ---- roll damping -------------------------------------------------------------------------------------------
    clp = {}
    if spec.get("roll"):
        rep = json.load(open(Path(spec["roll"]) / "cfd_report.json"))
        for e in rep.get("clp", []):
            if e.get("cfd") is not None:
                clp[float(e["mach"])] = float(e["cfd"])

    # ---- RANS ---------------------------------------------------------------------------------------------------
    rans_derived = {}
    if spec.get("rans"):
        out = Path(spec["rans"])
        rows, rep = cases(out)
        brk = {(float(b["mach"]), float(b["alpha_deg"])): b for b in rep.get("axial_breakdown", [])}
        rrows = []
        for r in rows:
            m, a = float(r["mach"]), float(r["alpha_deg"])
            b = brk.get((m, a), {})
            fr, ba = terms(m)
            if r["status"] == "failed":
                rrows.append([f"{m:g}", f"{a:g}", r["status"], r["iterations"]] + [""] * 11)
                continue
            rrows.append([f"{m:g}", f"{a:g}", r["status"], r["iterations"], fmt(float(r["cn_raw"])),
                          fmt(float(r["xcp_raw_m"]) / D) if a > 0 else "",
                          fmt(b.get("cfd_wall_ca")), fmt(b.get("cfd_base_ca")), fmt(b.get("roughness_increment")),
                          fmt(fr), fmt(ba), fmt(te_term(m)), rep.get("mesh_nodes"),
                          rep.get("mesh_cells"), r["wall_seconds"]])
        write(HERE / "cfd_rans.csv",
              f"# Basic Finner, SU2 RANS Spalart-Allmaras (IgnisYeet {spec['rans_rev']}, examples/basic_finner/basic_finner.toml with "
              f"aero.method = cfd, model = rans, surface = step, wall_size 1.5 mm, yplus 1, flat base, half domain). "
              f"Generated by doc/scripts/basic_finner_cfd.py. ca_wall = wall axial force (pressure + friction, fin edges); "
              f"ca_base_cfd = pressure on the base; ca_roughness = Barrowman rough-minus-smooth friction increment added to "
              f"the smooth-wall RANS value; friction_model, base_model and fin_te_base_model = Barrowman terms (axial_breakdown of cfd_report.json; fin_te_base_model as in cfd_euler.csv).",
              ["mach", "alpha_deg", "status", "iterations", "cn", "xcp_cal", "ca_wall", "ca_base_cfd", "ca_roughness",
               "friction_model", "base_model", "fin_te_base_model", "nodes", "cells",
               "wall_seconds"], rrows)
        for m, by in per_mach(rows).items():
            d = derived(by, 0.0)
            b = brk.get((m, 0.0))
            if d is None or b is None:
                continue
            extra = b.get("roughness_increment") or 0.0
            d["CX0"] = b["cfd_wall_ca"] + extra + b["barrowman_base"] + te_term(m)
            d["CX0_base_cfd"] = b["cfd_wall_ca"] + extra + b["cfd_base_ca"]
            rans_derived[m] = d

    # ---- summary ------------------------------------------------------------------------------------------------
    def table_vals(out):
        t = np.genfromtxt(Path(out) / "aero_table.csv", delimiter=",", names=True)
        t = t[t["alpha_deg"] == 0]
        t = t[np.argsort(t["mach"])]

        def at(m):
            f = lambda c: float(np.interp(m, t["mach"], t[c]))  # noqa: E731
            cna, xcp = f("cna"), f("xcp")
            return {"CNa": cna, "xcp": xcp / D, "Cma": -cna * (xcp - XCG) / D, "CX0": f("ca_off"), "Clp": f("clp")}
        return at

    bm, pn = table_vals(spec["barrowman"]), table_vals(spec["panel"])
    prim = spec["euler_primary"]
    srows = []
    for (q, m), (mv, pe) in sorted(meas.items(), key=lambda k: (["CNa", "Cma", "xcp", "CX0", "Clp"].index(k[0][0]), k[0][1])):
        vals = {"barrowman": bm(m)[q], "panel": pn(m)[q]}
        e = euler_derived.get((prim, m))
        vals["euler"] = (clp.get(m) if q == "Clp" else (e or {}).get(q))
        r = rans_derived.get(m)
        vals["rans"] = None if q == "Clp" or r is None else r[q]
        vals["rans_base_cfd"] = r["CX0_base_cfd"] if (q == "CX0" and r is not None) else None
        row = [q, f"{m:g}", f"{mv:g}", pe]
        for k in ("barrowman", "panel", "euler", "rans", "rans_base_cfd"):
            v = vals[k]
            row += [fmt(v, 4), "" if v is None else f"{(v - mv) / abs(mv) * 100:.1f}"]
        srows.append(row)
    write(HERE / "summary_vs_measured.csv",
          f"# Basic Finner: model vs DREV-TM-9703 Table IX (IgnisYeet {rev}). barrowman, panel: aero.method = barrowman / "
          f"panel with the fins extracted from the STL (barrowman_panel_vs_measured.csv); euler: cfd_euler.csv with "
          f"base_treatment = {prim} (Clp: {spec['roll_note']}); rans: cfd_rans.csv with "
          f"rans_base = model; rans_base_cfd: CX0 with rans_base = cfd. Definitions in doc/scripts/basic_finner_cfd.py; "
          f"*_pct = (model - measured) / |measured| x 100; probable_error_pct as tabulated (blank: held fixed or derived).",
          ["q", "mach", "measured", "probable_error_pct", "barrowman", "barrowman_pct", "panel", "panel_pct", "euler",
           "euler_pct", "rans", "rans_pct", "rans_base_cfd", "rans_base_cfd_pct"], srows)


if __name__ == "__main__":
    main()
