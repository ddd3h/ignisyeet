#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
"""Skin-friction axial force of a RANS case against the flat-plate estimate over the same wetted surface.

  rans_friction.py CASE_DIR PROFILE_CSV [--fins-area A]

CFD: friction part of the wall axial force from surface_flow.vtk (rans_surface.py). Estimate: turbulent flat plate
from the nose tip (fully turbulent, as the SA model with freestream nu factor 3), local skin friction of the
compressible van Driest II law (Hopkins-Inouye, adiabatic wall, recovery 0.89, local Karman-Schoenherr as the
incompressible law; this reduces to Karman-Schoenherr at low Mach) at Re_x = Re_per_m * s, s = arc length along the
profile, integrated over the body of revolution of PROFILE_CSV (geometry profile.csv: x, r) and divided by REF_AREA
(the half model integrates over half the circumference). Fins (optional): flat plate on both sides of a
trapezoid of plan area A per fin side at mean chord c, CF = mean Karman-Schoenherr / van Driest over the chord.
Freestream (pressure, temperature, Mach) from case.cfg, Sutherland viscosity.
"""
import argparse
import csv
import json
import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import rans_flatplate as fp  # noqa: E402
from rans_surface import split  # noqa: E402


def estimate(case, profile, fin_area=0.0, fin_chord=0.0, fins_half=0):
    cfg = dict((k.strip(), v.strip()) for k, v in (l.split("=", 1) for l in open(os.path.join(case, "case.cfg")) if "=" in l))
    mach, p, t = float(cfg["MACH_NUMBER"]), float(cfg["FREESTREAM_PRESSURE"]), float(cfg["FREESTREAM_TEMPERATURE"])
    aref = float(cfg["REF_AREA"])
    rho = p / (fp.R_AIR * t)
    u = mach * math.sqrt(fp.GAMMA * fp.R_AIR * t)
    re_m = rho * u / fp.sutherland(t)
    rows = [(float(r["x"]), float(r["r"])) for r in csv.DictReader(open(profile))]
    rows = [(0.0, 0.0)] + rows
    s, total, wet = 0.0, 0.0, 0.0
    for (x0, r0), (x1, r1) in zip(rows, rows[1:]):
        ds = math.hypot(x1 - x0, r1 - r0)
        sm = s + 0.5 * ds
        cf = fp.van_driest2(mach, re_m * sm, t_inf=t)[0]
        rm = 0.5 * (r0 + r1)
        dA = math.pi * rm * ds  # half circumference (half model)
        cos = (x1 - x0) / ds if ds > 0 else 1.0
        total += cf * dA * cos
        wet += dA
        s += ds
    fin = 0.0
    if fin_area > 0:
        # mean CF over the chord: (1/c) int cf dx
        n = 200
        cfm = sum(fp.van_driest2(mach, re_m * fin_chord * (k + 0.5) / n, t_inf=t)[0] for k in range(n)) / n
        fin = cfm * 2.0 * fin_area * fins_half
    return {"mach": mach, "re_per_m": re_m, "body_friction_ca_est": total / aref, "fin_friction_ca_est": fin / aref,
            "friction_ca_est": (total + fin) / aref, "body_wetted_half_area_m2": wet}


if __name__ == "__main__":
    ap = argparse.ArgumentParser()
    ap.add_argument("case")
    ap.add_argument("profile")
    ap.add_argument("--fin-area", type=float, default=0.0, help="plan area of one fin side [m^2]")
    ap.add_argument("--fin-chord", type=float, default=0.0, help="mean chord [m]")
    ap.add_argument("--fins-half", type=float, default=0, help="number of fins in the half model (fin in the symmetry plane = 0.5)")
    a = ap.parse_args()
    e = estimate(a.case, a.profile, a.fin_area, a.fin_chord, a.fins_half)
    c = split(a.case)
    e.update(c)
    e["friction_cfd_vs_est_percent"] = 100 * (c["wall_friction_ca"] / e["friction_ca_est"] - 1)
    print(json.dumps(e, indent=1))
