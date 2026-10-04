# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2025-2026 西濱大将 (NISHIHAMA Daisuke)
"""Generate the matplotlib figures of the documentation (SVG for HTML, PDF for LaTeX; no titles).

Usage: python scripts/make_plots.py <ignisyeet out dir> <figure dir> [<monte carlo out dir> [<panel out dir>]]

Figures built from simulation output read the files that `ignisyeet aero/sim/dispersion`
wrote for examples/sample.toml; purely theoretical curves are evaluated here with the
same formulas the Rust code uses.
"""

import json
import sys
from pathlib import Path

import matplotlib

matplotlib.use("Agg")
import matplotlib.pyplot as plt  # noqa: E402
import numpy as np  # noqa: E402
import pandas as pd  # noqa: E402

SERIES = ["#2a78d6", "#eb6834", "#1baf7a", "#eda100", "#e87ba4", "#008300", "#4a3aa7", "#e34948"]
SEQ = ["#86b6ef", "#6da7ec", "#5598e7", "#3987e5", "#2a78d6", "#256abf", "#1c5cab", "#184f95", "#104281", "#0d366b"]
INK, INK2, GRID, SURFACE = "#0b0b0b", "#52514e", "#e4e3df", "#ffffff"

plt.rcParams.update({
    "font.family": ["IPAexGothic", "IPAGothic", "DejaVu Sans"],
    "mathtext.fontset": "cm",
    "axes.unicode_minus": True,
    "figure.facecolor": SURFACE,
    "axes.facecolor": SURFACE,
    "savefig.facecolor": SURFACE,
    "axes.edgecolor": "#bdbcb6",
    "axes.labelcolor": INK2,
    "axes.labelsize": 10.5,
    "axes.grid": True,
    "grid.color": GRID,
    "grid.linewidth": 0.6,
    "axes.spines.top": False,
    "axes.spines.right": False,
    "xtick.color": INK2,
    "ytick.color": INK2,
    "xtick.labelsize": 9,
    "ytick.labelsize": 9,
    "legend.frameon": False,
    "legend.fontsize": 9,
    "lines.linewidth": 2.0,
    "svg.fonttype": "path",
    "pdf.fonttype": 42,
})


def seq(n):
    if n == 1:
        return [SEQ[5]]
    return [SEQ[round(1 + i * (len(SEQ) - 2) / (n - 1))] for i in range(n)]


class Figs:
    def __init__(self, out):
        self.out = out
        out.mkdir(parents=True, exist_ok=True)

    def save(self, fig, name):
        fig.savefig(self.out / f"{name}.svg", bbox_inches="tight")
        fig.savefig(self.out / f"{name}.pdf", bbox_inches="tight")
        plt.close(fig)
        print(f"plot  {name}.svg")


# ---------------------------------------------------------------- formulas (mirror Rust)

def us76(z):
    layers = [(0.0, 288.15, -0.0065, 101325.0), (11000.0, 216.65, 0.0, 22632.06), (20000.0, 216.65, 0.001, 5474.889),
              (32000.0, 228.65, 0.0028, 868.0187), (47000.0, 270.65, 0.0, 110.9063), (51000.0, 270.65, -0.0028, 66.93887),
              (71000.0, 214.65, -0.002, 3.956420)]
    g0, R = 9.80665, 287.05287
    h = np.clip(6356766.0 * z / (6356766.0 + z), -5000, 84852)
    T, p = np.empty_like(h), np.empty_like(h)
    for k, hk in enumerate(h):
        hb, tb, lr, pb = [l for l in layers if hk >= l[0]][-1]
        t = tb + lr * (hk - hb)
        T[k] = t
        p[k] = pb * np.exp(-g0 * (hk - hb) / (R * tb)) if lr == 0 else pb * (tb / t) ** (g0 / (R * lr))
    rho = p / (R * T)
    return T, p, rho, np.sqrt(1.4 * R * T)


def hermite(x, x0, x1, f0, f1, d0, d1):
    h = x1 - x0
    t = (x - x0) / h
    return (2 * t**3 - 3 * t**2 + 1) * f0 + (t**3 - 2 * t**2 + t) * h * d0 + (-2 * t**3 + 3 * t**2) * f1 + (t**3 - t**2) * h * d1


def axial_factor(a):
    a = np.minimum(np.abs(a), np.pi / 2)
    a17 = np.radians(17)
    return np.where(a <= a17, hermite(a, 0, a17, 1.0, 1.3, 0, 0), hermite(a, a17, np.pi / 2, 1.3, 0.0, 0, 0))


def skin_friction(re, m, rel):
    turb = 1.0 / (1.50 * np.log(re) - 5.6) ** 2
    turb = np.where(m < 1, turb * (1 - 0.1 * m**2), turb / (1 + 0.15 * m**2) ** 0.58)
    if rel > 0:
        rough = 0.032 * rel**0.2
        rough = rough * (1 - 0.1 * m**2) if m < 1 else rough / (1 + 0.18 * m**2)
        turb = np.maximum(turb, rough)
    return np.where(re < 1e4, 1.48e-2, turb)


def us76_offset(z, dt):
    """Standard atmosphere shifted by dt K with the hydrostatic pressure recomputed (as aero/atmosphere.rs)."""
    layers = [(0.0, 288.15, -0.0065), (11000.0, 216.65, 0.0), (20000.0, 216.65, 0.001), (32000.0, 228.65, 0.0028),
              (47000.0, 270.65, 0.0), (51000.0, 270.65, -0.0028), (71000.0, 214.65, -0.002)]
    g0, R = 9.80665, 287.05287
    out = []
    for zz in np.atleast_1d(z):
        h = float(np.clip(6356766.0 * zz / (6356766.0 + zz), -5000, 84852))
        p = 101325.0
        for i, (hb, tb, lr) in enumerate(layers):
            top = layers[i + 1][0] if i + 1 < len(layers) else np.inf
            hh = min(h, top)
            if hh <= hb and i > 0:
                break
            t0 = tb + dt
            p *= np.exp(-g0 * (hh - hb) / (R * t0)) if lr == 0 else ((t0 + lr * (hh - hb)) / t0) ** (-g0 / (R * lr))
            if h <= top:
                break
        k = max(i for i, l in enumerate(layers) if h >= l[0])
        t = layers[k][1] + layers[k][2] * (h - layers[k][0]) + dt
        out.append((t, p / (R * t)))
    return np.array(out)


def ellipse_xy(me, mn, a, b, bearing, n=200):
    th = np.radians(bearing)
    u, v = np.array([np.sin(th), np.cos(th)]), np.array([np.cos(th), -np.sin(th)])
    t = np.linspace(0, 2 * np.pi, n)
    pts = np.outer(np.cos(t) * a, u) + np.outer(np.sin(t) * b, v)
    return me + pts[:, 0], mn + pts[:, 1]


# ---------------------------------------------------------------- figures

def theory(f):
    # Atmosphere: three panels sharing the altitude axis.
    z = np.linspace(0, 86000, 600)
    T, p, rho, a = us76(z)
    fig, axs = plt.subplots(1, 3, figsize=(7.8, 3.4), sharey=True)
    for ax, val, lab in zip(axs, (T, rho, a), ("温度 $T$ [K]", "密度 $\\rho$ [kg/m$^3$]", "音速 $a$ [m/s]")):
        ax.plot(val, z / 1000, color=SERIES[0])
        ax.set_xlabel(lab)
    axs[1].set_xscale("log")
    axs[0].set_ylabel("幾何高度 $z$ [km]")
    fig.tight_layout()
    f.save(fig, "atmosphere")

    # Gravity ratio.
    h = np.linspace(0, 1000, 500)
    r = (6371.0 / (6371.0 + h)) ** 2
    fig, ax = plt.subplots(figsize=(5.6, 3.4))
    ax.plot(h, r * 100, color=SERIES[0])
    for hh in (100, 300, 1000):
        v = (6371 / (6371 + hh)) ** 2 * 100
        ax.plot([hh], [v], "o", color=SERIES[0], markersize=7, markerfacecolor=SURFACE, markeredgewidth=2)
        ax.annotate(f"{hh} km: {v:.1f}%", (hh, v), xytext=(6, 6), textcoords="offset points", color=INK2, fontsize=9)
    ax.set_xlabel("高度 $h$ [km]")
    ax.set_ylabel("$g(h)/g_0$ [%]")
    f.save(fig, "gravity")

    # Wind profiles for several exponents.
    hh = np.linspace(0, 3000, 400)
    fig, ax = plt.subplots(figsize=(5.6, 3.6))
    ns = [3, 4.5, 6, 7, 10]
    for n, c in zip(ns, seq(len(ns))):
        ax.plot(4.0 * (hh / 2.0) ** (1 / n), hh, color=c, label=f"$n={n:g}$")
    ax.set_xlabel("風速 $w(h)$ [m/s]（$w_\\mathrm{ref}=4$ m/s, $h_\\mathrm{ref}=2$ m）")
    ax.set_ylabel("地上高度 $h$ [m]")
    ax.legend(loc="lower right")
    f.save(fig, "wind_profile")

    # ENU flat-earth height error.
    d = np.linspace(0, 30, 300)
    fig, ax = plt.subplots(figsize=(5.6, 3.4))
    ax.plot(d, (d * 1000) ** 2 / (2 * 6.371e6), color=SERIES[0])
    for eps, c in ((1, SERIES[1]), (10, SERIES[2])):
        dmax = np.sqrt(2 * 6.371e6 * eps) / 1000
        ax.axhline(eps, color=c, linewidth=1.2, linestyle="--")
        ax.plot([dmax], [eps], "o", color=c, markersize=7, markerfacecolor=SURFACE, markeredgewidth=2)
        ax.annotate(f"$\\varepsilon={eps}$ m → $d\\leq{dmax:.1f}$ km", (dmax, eps), xytext=(8, -6), textcoords="offset points", va="top", color=INK2, fontsize=9)
    ax.set_ylim(-6, 75)
    ax.set_xlabel("基準点からの水平距離 $d$ [km]")
    ax.set_ylabel("接平面からのずれ $\\Delta h$ [m]")
    f.save(fig, "enu_error")

    # Temperature offset: density ratio and temperature.
    z = np.linspace(0, 20000, 201)
    base = us76_offset(z, 0.0)
    fig, axs = plt.subplots(1, 2, figsize=(7.8, 3.4), sharey=True)
    for dT, c in ((-15.0, SERIES[0]), (0.0, INK2), (15.0, SERIES[1])):
        a = us76_offset(z, dT)
        axs[0].plot(a[:, 0], z / 1000, color=c, label=f"$\\Delta T={dT:+.0f}$ K" if dT else "標準（$\\Delta T=0$）")
        axs[1].plot((a[:, 1] / base[:, 1] - 1) * 100, z / 1000, color=c)
    axs[0].set_xlabel("温度 $T$ [K]")
    axs[1].set_xlabel("標準大気に対する密度の差 [%]")
    axs[0].set_ylabel("幾何高度 $z$ [km]")
    axs[0].legend(loc="upper right")
    fig.tight_layout()
    f.save(fig, "atmosphere_offset")

    # Wind models: power law, logarithmic law and a tabulated profile with veer.
    hh = np.linspace(0.0, 300, 500)
    fig, axs = plt.subplots(1, 2, figsize=(7.8, 4.1))
    ax = axs[0]
    ax.plot(np.full_like(hh, 4.0), hh, color=SERIES[3], label="一定風")
    ax.plot(4.0 * (hh / 2.0) ** (1 / 6), hh, color=SERIES[0], label="べき法則（$n=6$, $h_\\mathrm{ref}=2$ m）")
    for z0, c, ls in ((0.03, SERIES[1], "-"), (0.3, SERIES[2], "-")):
        v = 4.0 * np.log(np.maximum(hh, z0) / z0) / np.log(2.0 / z0)
        ax.plot(v, hh, color=c, linestyle=ls, label=f"対数則（$z_0={z0:g}$ m）")
    ax.set_xlabel("風速 $w$ [m/s]（$w_\\mathrm{ref}=4$ m/s, $h_\\mathrm{ref}=2$ m）")
    ax.set_ylabel("地上高度 $h$ [m]")
    ax.legend(loc="upper center", bbox_to_anchor=(0.5, -0.2), ncol=2, fontsize=9)
    ax = axs[1]
    hh = np.linspace(0.0, 1000, 500)
    ph = np.array([0, 100, 300, 600, 1000.0])
    pv = np.array([3.0, 6.0, 9.0, 12.0, 13.0])
    pd_ = np.array([270, 270, 285, 300, 310.0])
    pe, pn = -pv * np.sin(np.radians(pd_)), -pv * np.cos(np.radians(pd_))
    e, n = np.interp(hh, ph, pe), np.interp(hh, ph, pn)
    ax.plot(e, hh, color=SERIES[0], label="東向き成分 $w_E$")
    ax.plot(n, hh, color=SERIES[1], label="北向き成分 $w_N$")
    ax.plot(pe, ph, "o", color=SERIES[0], markersize=6, markerfacecolor=SURFACE, markeredgewidth=2)
    ax.plot(pn, ph, "o", color=SERIES[1], markersize=6, markerfacecolor=SURFACE, markeredgewidth=2)
    ax.set_xlabel("風速ベクトルの成分 [m/s]（点: 表の値）")
    ax.set_ylabel("地上高度 $h$ [m]")
    ax.legend(loc="upper center", bbox_to_anchor=(0.5, -0.2), ncol=2, fontsize=9)
    fig.tight_layout()
    f.save(fig, "wind_models")

    # Adaptive step control: safety factor curve.
    err = np.logspace(-3, 2, 400)
    fac = np.clip(0.9 * err ** (-0.2), 0.2, 5.0)
    fig, ax = plt.subplots(figsize=(5.6, 3.3))
    ax.plot(err, fac, color=SERIES[0])
    ax.axvline(1.0, color=INK2, linewidth=1, linestyle=":")
    ax.annotate("err $=1$（受理の境界）", (1.0, 3.2), xytext=(6, 0), textcoords="offset points", color=INK2, fontsize=9)
    ax.set_xscale("log")
    ax.set_xlabel("規格化誤差 err")
    ax.set_ylabel("刻み幅の倍率 $\\min(5,\\max(0.2,\\ 0.9\\,\\mathrm{err}^{-1/5}))$")
    f.save(fig, "step_factor")

    # Axial-force multiplier versus angle of attack.
    al = np.linspace(0, 90, 361)
    fig, ax = plt.subplots(figsize=(5.6, 3.3))
    ax.plot(al, axial_factor(np.radians(al)), color=SERIES[0])
    ax.axvline(17, color=INK2, linewidth=1, linestyle=":")
    ax.annotate("$17^\\circ$, 1.3", (17, 1.3), xytext=(6, 4), textcoords="offset points", color=INK2, fontsize=9)
    ax.set_ylim(top=1.42)
    ax.set_xlabel("迎角 $\\alpha$ [deg]")
    ax.set_ylabel("$C_A(\\alpha)/C_{D0}$")
    f.save(fig, "axial_factor")

    # Skin friction coefficient versus Reynolds number.
    re = np.logspace(4, 9, 400)
    fig, ax = plt.subplots(figsize=(5.6, 3.5))
    cases = [(0.0, "滑面（乱流）"), (1e-6, "$R_s/L=10^{-6}$"), (1e-5, "$R_s/L=10^{-5}$"), (1e-4, "$R_s/L=10^{-4}$")]
    for (rel, lab), c in zip(cases, seq(len(cases))):
        ax.plot(re, skin_friction(re, 0.3, rel), color=c, label=lab)
    ax.set_xscale("log")
    ax.set_xlabel("レイノルズ数 $Re=VL/\\nu$")
    ax.set_ylabel("摩擦抗力係数 $C_f$（$M=0.3$）")
    ax.legend(loc="upper right")
    f.save(fig, "skin_friction")


def from_outputs(f, out):
    geo = json.loads((out / "geometry.json").read_text())
    prof = pd.read_csv(out / "profile.csv")
    drag = pd.read_csv(out / "aero_drag.csv")
    table = pd.read_csv(out / "aero_table.csv")
    traj = pd.read_csv(out / "trajectory.csv")
    summ = json.loads((out / "summary.json").read_text())
    disp = pd.read_csv(out / "dispersion.csv")

    # Extracted profile versus the nominal sample rocket.
    L, R, Ln = 1.5, 0.05, 0.30
    rho = (R * R + Ln * Ln) / (2 * R)
    xs = np.linspace(0, L, 800)
    rn = np.where(xs < Ln, np.sqrt(np.maximum(rho**2 - (Ln - xs) ** 2, 0)) + R - rho, R)
    fig, ax = plt.subplots(figsize=(7.8, 3.0))
    ax.plot(xs, rn * 1000, color=INK2, linewidth=1.2, linestyle="--", zorder=3, label="設計値（接線オジブ + 円筒）")
    ax.plot(prof.x, prof.r * 1000, color=SERIES[0], label="STL から抽出した胴体半径 $r_b$")
    fin = prof[prof.fin_span > 0]
    ax.plot(fin.x, (fin.r + fin.fin_span) * 1000, ".", color=SERIES[1], markersize=3, label="フィン先端半径（各断面）")
    fs = geo["fins"]
    rb = fs["body_radius"]
    px = [fs["x_le_root"], fs["x_le_root"] + fs["sweep"], fs["x_le_root"] + fs["sweep"] + fs["tip_chord"], fs["x_le_root"] + fs["root_chord"]]
    py = [rb, rb + fs["span"], rb + fs["span"], rb]
    ax.plot(px, np.array(py) * 1000, color=SERIES[1], linewidth=1.5, label="抽出した台形フィン")
    ax.axvline(geo["nose"]["length"], color=SERIES[2], linewidth=1, linestyle=":")
    ax.annotate("ノーズ終端（検出）", (geo["nose"]["length"], 20), xytext=(6, 0), textcoords="offset points", color=INK2, fontsize=9)
    ax.set_xlabel("先端からの距離 $x$ [m]")
    ax.set_ylabel("半径 $r$ [mm]")
    ax.set_ylim(0, 165)
    ax.legend(loc="upper left", ncol=2)
    f.save(fig, "geometry_profile")

    # CNa and Xcp versus Mach (two separate panels, never a dual axis).
    fig, axs = plt.subplots(1, 2, figsize=(7.8, 3.3))
    axs[0].plot(drag.mach, drag.cna, color=SERIES[0])
    axs[0].set_ylabel("$C_{N\\alpha}$ [1/rad]")
    axs[1].plot(drag.mach, drag.xcp, color=SERIES[0])
    axs[1].set_ylabel("$x_{cp}$（$\\alpha\\to0$）[m]")
    for ax in axs:
        ax.set_xlabel("Mach 数 $M$")
        for m in (0.9, 1.5):
            ax.axvline(m, color=INK2, linewidth=0.8, linestyle=":")
    axs[0].annotate("遷音速\n補間区間", (1.2, drag.cna.min() + 0.3), ha="center", color=INK2, fontsize=9, linespacing=1.2)
    fig.tight_layout()
    f.save(fig, "aero_cna_xcp")

    # Drag build-up.
    parts = [("friction", "表面摩擦"), ("nose", "ノーズ圧力・造波"), ("base", "底面"), ("fins", "フィン前縁・後縁"), ("boattail", "ボートテール"), ("extra", "その他（extra_cd）")]
    parts = [(k, l) for k, l in parts if drag[k].abs().max() > 0]
    fig, ax = plt.subplots(figsize=(7.0, 3.8))
    ax.stackplot(drag.mach, *[drag[k] for k, _ in parts], labels=[l for _, l in parts], colors=SERIES[: len(parts)], alpha=0.85, edgecolor=SURFACE, linewidth=1)
    ax.plot(drag.mach, drag.cd_on, color=INK, linewidth=1.3, linestyle="--", label="合計（燃焼中）")
    ax.set_xlabel("Mach 数 $M$")
    ax.set_ylabel("零揚力抗力係数 $C_{D0}$")
    ax.legend(loc="upper left", bbox_to_anchor=(1.01, 1))
    f.save(fig, "aero_drag")

    # CN versus alpha.
    machs = np.sort(table.mach.unique())
    picks = [0.3, 0.8, 1.2, 2.0, 3.0]
    fig, ax = plt.subplots(figsize=(5.6, 3.6))
    for m, c, ls in zip(picks, seq(len(picks)), ("-", "--", "-", "--", "-")):
        near = machs[np.argmin(np.abs(machs - m))]
        s = table[table.mach == near]
        ax.plot(s.alpha_deg, s.cn, color=c, linestyle=ls, label=f"$M={near:.1f}$")
    ax.set_xlabel("迎角 $\\alpha$ [deg]")
    ax.set_ylabel("法線力係数 $C_N$")
    ax.legend(loc="upper left")
    f.save(fig, "aero_cn_alpha")

    # Thrust and propellant.
    eng = np.loadtxt(Path(__file__).resolve().parents[2] / "examples" / "sample_motor.eng", comments=";", skiprows=3)
    t = np.concatenate([[0.0], eng[:, 0]])
    F = np.concatenate([[0.0], eng[:, 1]])
    I = np.concatenate([[0.0], np.cumsum(0.5 * (F[1:] + F[:-1]) * np.diff(t))])
    fig, axs = plt.subplots(1, 2, figsize=(7.8, 3.3))
    axs[0].plot(t, F, color=SERIES[0])
    axs[0].fill_between(t, F, color=SERIES[0], alpha=0.12, linewidth=0)
    axs[0].annotate(f"全力積 $I_t={I[-1]:.0f}$ N s", (1.0, 1200), color=INK2, fontsize=9)
    axs[0].set_ylabel("推力 $T$ [N]")
    axs[1].plot(t, 3.0 * (1 - I / I[-1]), color=SERIES[0])
    axs[1].set_ylabel("推進剤質量 $m_p$ [kg]")
    for ax in axs:
        ax.set_xlabel("時刻 $t$ [s]")
    fig.tight_layout()
    f.save(fig, "thrust_mass")

    # Trajectory: side view and ground track.
    body = traj[traj.phase != "parachute"]
    fig, axs = plt.subplots(1, 2, figsize=(7.8, 3.5))
    for ph, c, lab in (("rail", SERIES[2], "ランチャ"), ("free", SERIES[0], "自由飛行"), ("parachute", SERIES[1], "パラシュート降下")):
        s = traj[traj.phase == ph]
        axs[0].plot(s.t, s.up, color=c, label=lab)
        axs[1].plot(s.east, s.north, color=c, label=lab)
    axs[0].plot([summ["apogee_time"]], [summ["apogee"]], "o", color=SERIES[0], markersize=8, markerfacecolor=SURFACE, markeredgewidth=2)
    axs[0].annotate(f"頂点 {summ['apogee']:.0f} m（{summ['apogee_time']:.1f} s）", (summ["apogee_time"], summ["apogee"]), xytext=(14, 2), textcoords="offset points", color=INK2, fontsize=9)
    axs[0].set_xlabel("時刻 $t$ [s]")
    axs[0].set_ylabel("射点からの高度 $z_U$ [m]")
    axs[1].plot([0], [0], marker="*", color=INK, markersize=11, linestyle="none", label="射点")
    axs[1].set_xlabel("東 $x_E$ [m]")
    axs[1].set_ylabel("北 $y_N$ [m]")
    axs[1].set_aspect("equal", adjustable="datalim")
    axs[1].legend(loc="lower right")
    fig.tight_layout()
    f.save(fig, "trajectory")

    # Flight states during powered and coasting flight.
    asc = body[(body.vel_u >= 0) & (body.airspeed > 10)]
    free = body[body.phase == "free"]
    panels = [("mach", "Mach 数", body), ("alpha_deg", "迎角 $\\alpha$ [deg]", free), ("stability_cal", "静安定余裕 [cal]", asc), ("dyn_pressure", "動圧 $q$ [kPa]", body)]
    fig, axs = plt.subplots(2, 2, figsize=(7.8, 5.2))
    for ax, (col, lab, d) in zip(axs.flat, panels):
        y = d[col] / 1000 if col == "dyn_pressure" else d[col]
        ax.plot(d.t, y, color=SERIES[0])
        ax.set_ylabel(lab)
        ax.set_xlabel("時刻 $t$ [s]")
    fig.tight_layout()
    f.save(fig, "flight_states")

    # Dispersion.
    modes = [("ballistic", "弾道落下"), ("parachute", "パラシュート降下")]
    speeds = sorted(disp.wind_speed.unique())
    colors = dict(zip(speeds, seq(len(speeds))))
    fig, axs = plt.subplots(1, 2, figsize=(8.2, 4.6))
    for ax, (mode, lab) in zip(axs, modes):
        s = disp[disp.descent == mode]
        for v in speeds:
            r = s[s.wind_speed == v].sort_values("wind_direction_deg")
            e = np.append(r.landing_east.values, r.landing_east.values[:1]) / 1000
            n = np.append(r.landing_north.values, r.landing_north.values[:1]) / 1000
            ax.plot(e, n, "-o", color=colors[v], markersize=3.5, linewidth=1.4, label=f"{v:g} m/s")
        ax.plot([0], [0], marker="*", color=INK, markersize=11, linestyle="none", label="射点")
        ax.set_aspect("equal", adjustable="datalim")
        ax.set_xlabel("東 [km]")
        ax.set_ylabel("北 [km]")
        ax.text(0.02, 0.98, lab, transform=ax.transAxes, va="top", color=INK, fontsize=10)
    axs[1].legend(title="基準風速", loc="upper left", bbox_to_anchor=(1.01, 1))
    fig.tight_layout()
    f.save(fig, "dispersion")


def monte_carlo(f, out):
    d = pd.read_csv(out / "dispersion_mc.csv")
    summ = json.loads((out / "dispersion_summary.json").read_text())
    stats = {s["descent"]: s for s in summ["descents"]}
    labels = {"ballistic": "弾道落下", "parachute": "パラシュート降下"}
    modes = [m for m in ("ballistic", "parachute") if m in stats]
    fig, axs = plt.subplots(1, len(modes), figsize=(4.0 * len(modes), 4.4), squeeze=False)
    for ax, mode in zip(axs[0], modes):
        s = d[(d.descent == mode) & (d.status == "ok")]
        st = stats[mode]
        ax.scatter(s.landing_east / 1000, s.landing_north / 1000, s=5, color=SERIES[0], alpha=0.3, linewidths=0, label=f"着地点（$n={st['n']}$）", rasterized=True)
        for k, ls in ((1, "-"), (3, "--")):
            e = st[f"ellipse_{k}sigma"]
            x, y = ellipse_xy(st["mean_east"], st["mean_north"], e["semi_major"], e["semi_minor"], e["major_axis_bearing_deg"])
            ax.plot(x / 1000, y / 1000, color=SERIES[1], linestyle=ls, linewidth=1.5, label=f"{k}$\\sigma$ 楕円")
        ax.plot([st["mean_east"] / 1000], [st["mean_north"] / 1000], marker="+", color=SERIES[1], markersize=11, markeredgewidth=2, linestyle="none", label="平均")
        ax.plot([0], [0], marker="*", color=INK, markersize=11, linestyle="none", label="射点")
        ax.set_aspect("equal", adjustable="datalim")
        ax.set_xlabel("東 [km]")
        ax.set_ylabel("北 [km]")
        ax.text(0.02, 0.98, labels[mode], transform=ax.transAxes, va="top", color=INK, fontsize=10)
        ax.legend(loc="upper center", bbox_to_anchor=(0.5, -0.14), ncol=3, fontsize=9)
    fig.tight_layout()
    f.save(fig, "dispersion_mc")


def panel_method(f, out, pn):
    """Barrowman (out) versus panel method (pn): coefficients, surface Cp and flight histories."""
    from matplotlib.colors import TwoSlopeNorm
    import matplotlib.tri as mtri

    ta = pd.read_csv(out / "aero_table.csv")
    tb = pd.read_csv(pn / "aero_table.csv")
    ta, tb = (t[t.alpha_deg == 0].sort_values("mach") for t in (ta, tb))
    fig, axs = plt.subplots(1, 3, figsize=(7.8, 3.3))
    for ax, col, lab in zip(axs, ("cna", "xcp", "ca_off"), ("$C_{N\\alpha}$ [1/rad]", "$x_{cp}$（$\\alpha\\to0$）[m]", "零揚力抗力係数 $C_{D0}$（燃焼後）")):
        ax.plot(ta.mach, ta[col], color=SERIES[0], label="Barrowman 法")
        ax.plot(tb.mach, tb[col], color=SERIES[1], linestyle="--", label="パネル法")
        ax.set_xlabel("Mach 数 $M$")
        ax.set_ylabel(lab)
    fig.legend(*axs[0].get_legend_handles_labels(), loc="lower center", ncol=2, bbox_to_anchor=(0.5, 0.0))
    fig.tight_layout(rect=(0, 0.08, 1, 1))
    f.save(fig, "panel_compare")

    d = pd.read_csv(pn / "panel_cp.csv")
    lim = max(float(np.percentile(np.abs(d.cp), 99)), 0.1)
    norm = TwoSlopeNorm(vmin=-lim, vcenter=0.0, vmax=lim)

    def tri(ax, x, y, c, max_dy):
        t = mtri.Triangulation(x, y)
        t.set_mask(np.ptp(np.asarray(y)[t.triangles], axis=1) > max_dy)
        return ax.tripcolor(t, np.asarray(c), cmap="RdBu_r", norm=norm, shading="gouraud", rasterized=True)

    body, fin = d[d.part == "body"], d[d.part == "fin"]
    fig = plt.figure(figsize=(7.8, 5.4), layout="constrained")
    gs = fig.add_gridspec(2, 2, height_ratios=[1, 1.15])
    ax = fig.add_subplot(gs[0, :])
    th = np.degrees(np.arctan2(body.z, body.y))
    sc = tri(ax, body.x.values, th.values, body.cp.values, 25.0)
    ax.set_xlabel("先端からの距離 $x$ [m]")
    ax.set_ylabel("周方向角 $\\theta$ [deg]")
    ax.set_yticks([-180, -90, 0, 90, 180])
    ax.grid(False)
    phi = np.arctan2(fin.z, fin.y)
    k = fin[np.abs(np.angle(np.exp(1j * (phi - np.radians(45))))) < np.radians(20)]
    side = k.ny * (-np.sin(np.radians(45))) + k.nz * np.cos(np.radians(45))
    rho = np.hypot(k.y, k.z)
    for col, (sel, lab) in enumerate(((side > 0, "フィン（$+$ 側の面）"), (side <= 0, "フィン（$-$ 側の面）"))):
        a = fig.add_subplot(gs[1, col])
        tri(a, k.x[sel].values, rho[sel].values, k.cp[sel].values, 1e9)
        a.set_xlabel("先端からの距離 $x$ [m]")
        a.set_ylabel("軸からの距離 [m]")
        a.text(0.03, 0.96, lab, transform=a.transAxes, va="top", color=INK, fontsize=9)
        a.set_aspect("equal", adjustable="datalim")
        a.grid(False)
    cb = fig.colorbar(sc, ax=fig.axes, shrink=0.85, pad=0.02)
    cb.outline.set_visible(False)
    cb.set_label("圧力係数 $C_p$")
    f.save(fig, "panel_cp")

    if (pn / "trajectory.csv").exists():
        fig, axs = plt.subplots(1, 2, figsize=(7.8, 3.3))
        for path, c, ls, lab in ((out, SERIES[0], "-", "Barrowman 法"), (pn, SERIES[1], "--", "パネル法")):
            tr = pd.read_csv(path / "trajectory.csv")
            axs[0].plot(tr.t, tr.up, color=c, linestyle=ls, label=lab)
            asc = tr[(tr.phase != "parachute") & (tr.vel_u >= 0) & (tr.airspeed > 10)]
            axs[1].plot(asc.t, asc.stability_cal, color=c, linestyle=ls, label=lab)
        axs[0].set_xlabel("時刻 $t$ [s]")
        axs[0].set_ylabel("高度 [m]")
        axs[1].set_xlabel("時刻 $t$ [s]")
        axs[1].set_ylabel("静安定余裕 [cal]（上昇中）")
        axs[0].legend(loc="upper right")
        fig.tight_layout()
        f.save(fig, "panel_flight")


def cfd_study(f, out, pn, data):
    """CFD (doc/data/cfd, committed) versus Barrowman and panel; mesh convergence; zero-angle asymmetry."""
    vm = pd.read_csv(data / "sample_euler_vs_models.csv", comment="#")
    z = vm[vm.alpha_deg == 0].sort_values("mach")
    ta = pd.read_csv(out / "aero_table.csv")
    ta = ta[ta.alpha_deg == 0].sort_values("mach")
    tb = None
    if pn is not None and (pn / "aero_table.csv").exists():
        tb = pd.read_csv(pn / "aero_table.csv")
        tb = tb[tb.alpha_deg == 0].sort_values("mach")
    fig, axs = plt.subplots(1, 3, figsize=(7.8, 3.3))
    cols = (("cna", "cfd_cna_per_rad", "$C_{N\\alpha}$ [1/rad]"), ("xcp", "cfd_xcp_m", "$x_{cp}$（$\\alpha=4^\\circ$）[m]"), ("ca_off", "cfd_ca_off", "軸力係数 $C_A$（$\\alpha=0$、燃焼後）"))
    for ax, (col, ccol, lab) in zip(axs, cols):
        if col == "xcp":
            # xcp at alpha = 4 deg from the solved tables: the alpha = 0 value of the table is the small-angle limit
            a4 = pd.read_csv(out / "aero_table.csv")
            a4 = a4[a4.alpha_deg == 4].sort_values("mach")
            ax.plot(a4.mach, a4.xcp, color=SERIES[0], label="Barrowman 法")
            if tb is not None:
                b4 = pd.read_csv(pn / "aero_table.csv")
                b4 = b4[b4.alpha_deg == 4].sort_values("mach")
                ax.plot(b4.mach, b4.xcp, color=SERIES[1], linestyle="--", label="パネル法")
            c4 = vm[vm.alpha_deg == 4].sort_values("mach")
            ax.plot(c4.mach, c4.cfd_xcp_m, "o", color=SERIES[2], markersize=6, label="CFD（Euler）")
        else:
            ax.plot(ta.mach, ta[col], color=SERIES[0], label="Barrowman 法")
            if tb is not None:
                ax.plot(tb.mach, tb[col], color=SERIES[1], linestyle="--", label="パネル法")
            ax.plot(z.mach, z[ccol], "o", color=SERIES[2], markersize=6, label="CFD（Euler）")
        ax.set_xlabel("Mach 数 $M$")
        ax.set_ylabel(lab)
        ax.set_xlim(0, 3)
    fig.legend(*axs[0].get_legend_handles_labels(), loc="lower center", ncol=3, bbox_to_anchor=(0.5, 0.0))
    fig.tight_layout(rect=(0, 0.08, 1, 1))
    f.save(fig, "cfd_compare")

    mc = pd.read_csv(data / "mesh_convergence.csv", comment="#")
    summ = json.load(open(data / "run_summary.json"))["mesh_convergence"]
    fig, ax = plt.subplots(figsize=(5.2, 3.5))
    for m, c, mk in ((2.0, SERIES[0], "o"), (0.8, SERIES[1], "s")):
        s = mc[mc.mach == m].sort_values("cells")
        h = s.cells.values ** (-1.0 / 3.0) * 1e2
        ax.plot(h, s.cn, mk, color=c, markersize=6, label=f"$M={m:g}$")
        if m == 2.0:
            y, x = s.cn.values, s.cells.values ** (-1.0 / 3.0)
            best = None
            for p in np.arange(0.5, 6.0, 0.01):
                A = np.c_[np.ones_like(x), x ** p]
                cf = np.linalg.lstsq(A, y, rcond=None)[0]
                r = float(((A @ cf - y) ** 2).sum())
                if best is None or r < best[0]:
                    best = (r, p, cf)
            _, p, cf = best
            xs = np.linspace(0, x.max(), 100)
            ax.plot(xs * 1e2, cf[0] + cf[1] * xs ** p, color=c, linewidth=1.2, label=f"外挿 $C_N=C_N^\\infty+Ch^{{p}}$（$p={p:.2f}$）")
            ax.plot([0], [cf[0]], "D", color=c, markersize=6, markerfacecolor="white")
            ax.axhline(cf[0], color=c, linewidth=0.7, linestyle=":")
        else:
            ax.axhline(summ["M0.8"]["cn_extrapolated"], color=c, linewidth=0.7, linestyle=":")
    ax.set_xlim(left=-0.03)
    ax.set_xlabel("代表格子幅 $h=N_{cell}^{-1/3}$ [$10^{-2}$]")
    ax.set_ylabel("$C_N$（$\\alpha=4^\\circ$）")
    ax.legend(loc="lower left")
    fig.tight_layout()
    f.save(fig, "cfd_mesh_convergence")

    asy = pd.read_csv(data / "asymmetry_study.csv", comment="#")
    order = [("base", "半モデル\n（既定の\nメッシュ）"), ("roll45", "フィン\n45° 回転"), ("jst", "JST"), ("tight", "残差\n$10^{-9}$"), ("zm", "z 対称\nメッシュ")]
    fig, ax = plt.subplots(figsize=(6.2, 3.3))
    w = 0.26
    for i, (m, c) in enumerate(((0.3, SERIES[0]), (0.8, SERIES[1]), (2.0, SERIES[2]))):
        vals = [float(asy[(asy.config == k) & (asy.mach == m)].cn0.iloc[0]) for k, _ in order]
        ax.bar(np.arange(len(order)) + (i - 1) * w, vals, w, color=c, label=f"$M={m:g}$")
    ax.axhline(0, color=INK2, linewidth=0.8)
    ax.set_xticks(range(len(order)))
    ax.set_xticklabels([l for _, l in order])
    ax.set_ylabel("$\\alpha=0$ の法線力係数 $C_N$（真値は 0）")
    ax.set_ylim(-0.075, 0.125)
    ax.text(len(order) - 1, 0.008, "$|C_N|<10^{-3}$", ha="center", color=INK2, fontsize=9)
    ax.legend(loc="upper right", ncol=3)
    ax.grid(axis="x", visible=False)
    fig.tight_layout()
    f.save(fig, "cfd_asymmetry")


def main():
    out, figdir = Path(sys.argv[1]), Path(sys.argv[2])
    f = Figs(figdir)
    theory(f)
    from_outputs(f, out)
    if len(sys.argv) > 3:
        monte_carlo(f, Path(sys.argv[3]))
    if len(sys.argv) > 4 and (Path(sys.argv[4]) / "aero_table.csv").exists():
        panel_method(f, out, Path(sys.argv[4]))
    data = Path(__file__).resolve().parent.parent / "data" / "cfd"
    if (data / "sample_euler_vs_models.csv").exists():
        pn = Path(sys.argv[4]) if len(sys.argv) > 4 else None
        cfd_study(f, out, pn, data)


if __name__ == "__main__":
    main()
