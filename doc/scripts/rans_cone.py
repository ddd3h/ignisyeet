#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
"""RANS cone verification: surface Cp on the 10 degree cone of rans_geometry.py at M 2, alpha 0, against the
Taylor-Maccoll solution (integrated here, independent of panel::supersonic::cone_cp).

  rans_cone.py CASE_DIR [--out CSV]   CASE_DIR = <output>/cfd/m2.000_a+00.00 (surface_flow.vtk of the wall)

Cp is binned along x / L_cone over 0.2 .. 0.9 (L_cone = 0.05 / tan 10 deg); the wall nodes are the points of the
wall marker with radius within 1 % of the cone radius at their x.
"""
import argparse
import math
import os
import sys

GAMMA = 1.4


def taylor_maccoll(mach, delta_deg):
    """Surface Cp of a sharp cone (attached shock) by the Taylor-Maccoll equation, shooting on the shock angle."""
    g = GAMMA
    delta = math.radians(delta_deg)

    def cone_angle(beta):
        # oblique shock
        mn1 = mach * math.sin(beta)
        mn2 = math.sqrt((1 + 0.5 * (g - 1) * mn1 ** 2) / (g * mn1 ** 2 - 0.5 * (g - 1)))
        theta = math.atan(2 / math.tan(beta) * (mn1 ** 2 - 1) / (mach ** 2 * (g + math.cos(2 * beta)) + 2))
        m2 = mn2 / math.sin(beta - theta)
        v = 1.0 / math.sqrt(2.0 / ((g - 1) * m2 ** 2) + 1.0)  # V / V_max
        vr, vt = v * math.cos(beta - theta), -v * math.sin(beta - theta)

        def f(th, y):
            vr, vt = y
            a2 = 0.5 * (g - 1) * (1 - vr * vr - vt * vt)
            dvt = (vt * vt * vr - a2 * (2 * vr + vt / math.tan(th))) / (a2 - vt * vt)
            return (vt, dvt)

        th, y, h = beta, (vr, vt), -1e-5
        while y[1] < 0:
            k1 = f(th, y)
            k2 = f(th + h / 2, (y[0] + h / 2 * k1[0], y[1] + h / 2 * k1[1]))
            k3 = f(th + h / 2, (y[0] + h / 2 * k2[0], y[1] + h / 2 * k2[1]))
            k4 = f(th + h, (y[0] + h * k3[0], y[1] + h * k3[1]))
            yn = (y[0] + h / 6 * (k1[0] + 2 * k2[0] + 2 * k3[0] + k4[0]), y[1] + h / 6 * (k1[1] + 2 * k2[1] + 2 * k3[1] + k4[1]))
            if yn[1] >= 0:
                w = -y[1] / (yn[1] - y[1])
                return th + w * h, y[0] + w * (yn[0] - y[0]), mn1
            th, y = th + h, yn
        return th, y[0], mn1

    lo, hi = math.asin(1 / mach) + 1e-6, math.radians(80)
    for _ in range(80):
        mid = 0.5 * (lo + hi)
        if cone_angle(mid)[0] < delta:
            lo = mid
        else:
            hi = mid
    beta = 0.5 * (lo + hi)
    _, vc, mn1 = cone_angle(beta)
    mc = math.sqrt(2.0 / (g - 1) * vc * vc / (1 - vc * vc))
    # p_c / p_inf = (p02 / p_inf) / (p02 / p_c), total pressure loss across the shock
    p2p1 = 1 + 2 * g / (g + 1) * (mn1 ** 2 - 1)
    p02_p01 = ((g + 1) * mn1 ** 2 / ((g - 1) * mn1 ** 2 + 2)) ** (g / (g - 1)) * ((g + 1) / (2 * g * mn1 ** 2 - (g - 1))) ** (1 / (g - 1))
    p01_p1 = (1 + 0.5 * (g - 1) * mach ** 2) ** (g / (g - 1))
    p0c_pc = (1 + 0.5 * (g - 1) * mc ** 2) ** (g / (g - 1))
    pc = p02_p01 * p01_p1 / p0c_pc
    return 2 / (g * mach ** 2) * (pc - 1), math.degrees(beta), mc


def main():
    sys.path.insert(0, os.path.dirname(__file__))
    from rans_flatplate import read_vtk

    p = argparse.ArgumentParser()
    p.add_argument("case")
    p.add_argument("--out")
    p.add_argument("--header", default="")
    p.add_argument("--mach", type=float, default=2.0)
    a = p.parse_args()
    cp_tm, beta, mc = taylor_maccoll(a.mach, 10.0)
    r, delta = 0.05, math.radians(10.0)
    lc = r / math.tan(delta)
    pts, f = read_vtk(os.path.join(a.case, "surface_flow.vtk"))
    cpk = "Pressure_Coefficient"
    bins = [(0.2 + 0.1 * i, 0.3 + 0.1 * i) for i in range(7)]
    rows, allv = [], []
    for lo, hi in bins:
        v = [c for (x, y, z), c in zip(pts, f[cpk]) if lo * lc <= x < hi * lc and abs(math.hypot(y, z) - x * math.tan(delta)) < 0.01 * x * math.tan(delta) + 1e-6]
        allv += v
        m = sum(v) / len(v)
        rows.append((lo, hi, len(v), m, 100 * (m / cp_tm - 1)))
    mean = sum(allv) / len(allv)
    worst = max(abs(r_[4]) for r_ in rows)
    out = [f"# {a.header} Taylor-Maccoll (integrated in rans_cone.py): Cp = {cp_tm:.5f}, shock angle {beta:.3f} deg, "
           f"cone surface Mach {mc:.4f}. Cp from SU2 Pressure_Coefficient of the wall nodes on the cone (radius within 1 % "
           f"of x tan 10 deg), binned along x / L_cone.",
           "x_over_lc_from,x_over_lc_to,nodes,cp_mean,error_vs_taylor_maccoll_percent"]
    out += [f"{lo:.1f},{hi:.1f},{n},{m:.5f},{e:.2f}" for lo, hi, n, m, e in rows]
    out += [f"0.2,0.9,{len(allv)},{mean:.5f},{100 * (mean / cp_tm - 1):.2f}"]
    text = "\n".join(out) + "\n"
    print(text)
    print(f"largest bin error {worst:.2f} %")
    if a.out:
        open(a.out, "w").write(text)


if __name__ == "__main__":
    main()
