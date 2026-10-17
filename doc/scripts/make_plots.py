# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2024-2026 西濱大将 (NISHIHAMA Daisuke)
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
import matplotlib.ticker  # noqa: E402
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
    err = np.logspace(-5, 4, 600)  # wide enough to show both clamps (err < 1.9e-4: 5, err > 1.8e3: 0.2)
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


# ---------------------------------------------------------------- free rigid body: RKMK vs RK4 + normalisation
RB_INERTIA = np.array([1.0, 2.0, 3.0])


def qmul(a, b):
    aw, av, bw, bv = a[0], a[1:], b[0], b[1:]
    return np.concatenate([[aw * bw - av @ bv], aw * bv + bw * av + np.cross(av, bv)])


def qconj(q):
    return np.array([q[0], -q[1], -q[2], -q[3]])


def exp_so3(th):
    n = np.linalg.norm(th)
    if n < 1e-8:
        return np.concatenate([[1.0 - n * n / 8.0], 0.5 * th])
    return np.concatenate([[np.cos(0.5 * n)], np.sin(0.5 * n) / n * th])


def log_so3(q):
    if q[0] < 0:
        q = -q
    n = np.linalg.norm(q[1:])
    return np.zeros(3) if n < 1e-14 else 2.0 * np.arctan2(n, q[0]) / n * q[1:]


def dexpinv(u, v):
    s_ = np.linalg.norm(u)
    g = 1.0 / 12.0 + s_**2 / 720.0 if s_ < 0.1 else (1.0 - 0.5 * s_ / np.tan(0.5 * s_)) / s_**2
    uv = np.cross(u, v)
    return v - 0.5 * uv + g * np.cross(u, uv)


def euler_rhs(w):
    i1, i2, i3 = RB_INERTIA
    return np.array([(i2 - i3) / i1 * w[1] * w[2], (i3 - i1) / i2 * w[2] * w[0], (i1 - i2) / i3 * w[0] * w[1]])


RK4_A = [[], [0.5], [0.0, 0.5], [0.0, 0.0, 1.0]]
RK4_B = [1 / 6, 1 / 3, 1 / 3, 1 / 6]
DP5_A = [[], [1 / 5], [3 / 40, 9 / 40], [44 / 45, -56 / 15, 32 / 9],
         [19372 / 6561, -25360 / 2187, 64448 / 6561, -212 / 729],
         [9017 / 3168, -355 / 33, 46732 / 5247, 49 / 176, -5103 / 18656],
         [35 / 384, 0, 500 / 1113, 125 / 192, -2187 / 6784, 11 / 84]]
DP5_B = [35 / 384, 0, 500 / 1113, 125 / 192, -2187 / 6784, 11 / 84, 0]


def rkmk_step(a_tab, b_tab, w, q, h):
    kx, kt = [], []
    for i in range(len(b_tab)):
        th = h * sum((a * kt[j] for j, a in enumerate(a_tab[i])), np.zeros(3))
        wi = w + h * sum((a * kx[j] for j, a in enumerate(a_tab[i])), np.zeros(3))
        kx.append(euler_rhs(wi))
        kt.append(dexpinv(-th, wi))
    wn = w + h * sum(b * k for b, k in zip(b_tab, kx))
    th = h * sum(b * k for b, k in zip(b_tab, kt))
    return wn, qmul(q, exp_so3(th))


def rk4_normalise_step(w, q, h):
    kw, kq = [], []
    for i in range(4):
        wi = w + h * sum((a * kw[j] for j, a in enumerate(RK4_A[i])), np.zeros(3))
        qi = q + h * sum((a * kq[j] for j, a in enumerate(RK4_A[i])), np.zeros(4))
        kw.append(euler_rhs(wi))
        kq.append(0.5 * qmul(qi, np.concatenate([[0.0], wi])))
    wn = w + h * sum(b * k for b, k in zip(RK4_B, kw))
    qn = q + h * sum(b * k for b, k in zip(RK4_B, kq))
    return wn, qn


def rigid_body(f):
    w0 = np.array([0.3, 1.0, 0.4])
    q0 = exp_so3(np.array([0.4, -0.7, 0.2]))
    h, tend, href = 0.05, 20.0, 0.001
    n, r = int(round(tend / h)), int(round(h / href))
    # reference: RKMK with the 5th-order Dormand-Prince tableau, h = 1 ms
    wr, qr = w0.copy(), q0.copy()
    ref = [qr]
    for _ in range(n):
        for _ in range(r):
            wr, qr = rkmk_step(DP5_A, DP5_B, wr, qr, href)
        ref.append(qr)
    t = h * np.arange(n + 1)
    # RKMK + RK4
    w, q = w0.copy(), q0.copy()
    e_lie, d_lie = [0.0], [0.0]
    for k in range(n):
        w, q = rkmk_step(RK4_A, RK4_B, w, q, h)
        e_lie.append(np.linalg.norm(log_so3(qmul(qconj(ref[k + 1]), q))))
        d_lie.append(abs(np.linalg.norm(q) - 1.0))
    # RK4 on (w, q) in R^3 x R^4, renormalised after every step
    w, q = w0.copy(), q0.copy()
    e_nrm, d_nrm = [0.0], [0.0]
    for k in range(n):
        w, q = rk4_normalise_step(w, q, h)
        d_nrm.append(abs(np.linalg.norm(q) - 1.0))  # deviation removed by the renormalisation
        q = q / np.linalg.norm(q)
        e_nrm.append(np.linalg.norm(log_so3(qmul(qconj(ref[k + 1]), q))))
    print(f"rigid body h={h}: final attitude error RKMK {e_lie[-1]:.3e} rad, RK4+normalise {e_nrm[-1]:.3e} rad; "
          f"max |q|-1 RKMK {max(d_lie):.2e}, pre-normalisation RK4 {max(d_nrm):.2e}")
    fig, axs = plt.subplots(1, 2, figsize=(9.0, 3.4))
    axs[0].semilogy(t[1:], e_nrm[1:], color=SERIES[1], label="RK4 + 正規化")
    axs[0].semilogy(t[1:], e_lie[1:], color=SERIES[0], label="RKMK + RK4")
    axs[0].set_xlabel("時刻 $t$ [s]")
    axs[0].set_ylabel("姿勢の誤差 [rad]")
    axs[0].legend(loc="lower right")
    # exact zeros (|q| = 1 to the last bit) have no place on a log axis: those steps are left out (gaps)
    axs[1].semilogy(t[1:], np.where(np.array(d_nrm[1:]) > 0, d_nrm[1:], np.nan), color=SERIES[1], label="RK4（正規化前）")
    axs[1].semilogy(t[1:], np.where(np.array(d_lie[1:]) > 0, d_lie[1:], np.nan), color=SERIES[0], label="RKMK + RK4")
    axs[1].text(0.02, 0.02, "厳密に 0 の時刻は描かない（線の切れ目）", transform=axs[1].transAxes, color=INK2, fontsize=8)
    axs[1].set_xlabel("時刻 $t$ [s]")
    axs[1].set_ylabel("$\\left|\\,|q|-1\\right|$")
    axs[1].legend(loc="lower right")
    fig.tight_layout()
    f.save(fig, "rigid_body_attitude")


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
    # The rail run (5 m, a fraction of a second) is too short to see at this scale: drawn, but not in the legend.
    for ph, c, lab in (("rail", SERIES[2], None), ("free", SERIES[0], "自由飛行"), ("parachute", SERIES[1], "パラシュート降下")):
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

    # Flight states during the ascent (powered flight and coast up to apogee).
    asc = body[(body.vel_u >= 0) & (body.airspeed > 10)]
    free = asc[asc.phase == "free"]
    # every panel shows the ascent only (vertical velocity >= 0, airspeed > 10 m/s); alpha from rail exit on
    panels = [("mach", "Mach 数", asc), ("alpha_deg", "迎角 $\\alpha$ [deg]", free), ("stability_cal", "静安定余裕 [cal]", asc), ("dyn_pressure", "動圧 $q$ [kPa]", asc)]
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


def fin_side_quads(x, rho):
    """Panel outlines (n, 4, 2) in (x, rho) of one fin side, rebuilt from its panel centroids; None if not a structured grid.

    The panel mesh writes the side panels of a fin chord station by chord station, root to tip within
    a station (mesh.rs), so the centroids form an n_chord x n_span grid in file order. The corners are
    the means of the four surrounding centroids, with one row of centroids extrapolated linearly
    beyond every edge of the grid (root, tip, leading and trailing edge).
    """
    x, rho = np.asarray(x, float), np.asarray(rho, float)
    drops = np.flatnonzero(np.diff(rho) < 0)
    if len(x) < 4 or len(drops) == 0:
        return None
    ns = int(drops[0]) + 1
    if ns < 2 or len(x) % ns:
        return None
    nc = len(x) // ns
    g = np.stack([x, rho], axis=-1).reshape(nc, ns, 2)
    if nc < 2 or np.any(np.diff(g[:, :, 1], axis=1) <= 0) or np.any(np.diff(g[:, :, 0], axis=0) <= 0):
        return None
    p = np.empty((nc + 2, ns + 2, 2))
    p[1:-1, 1:-1] = g
    p[0, 1:-1], p[-1, 1:-1] = 2 * g[0] - g[1], 2 * g[-1] - g[-2]
    p[:, 0], p[:, -1] = 2 * p[:, 1] - p[:, 2], 2 * p[:, -2] - p[:, -3]
    v = 0.25 * (p[:-1, :-1] + p[1:, :-1] + p[:-1, 1:] + p[1:, 1:])
    return np.stack([v[:-1, :-1], v[1:, :-1], v[1:, 1:], v[:-1, 1:]], axis=2).reshape(-1, 4, 2)


def panel_method(f, out, pn):
    """Barrowman (out) versus panel method (pn): coefficients, surface Cp and flight histories."""
    from matplotlib.collections import PolyCollection
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

    def tri(ax, x, y, c, max_dy, y_scale=1.0, shading="gouraud"):
        # Delaunay on (x, y_scale*y) so the panels are near-square (no slivers along narrow columns).
        x, y = np.asarray(x), np.asarray(y)
        t = mtri.Triangulation(x, y, mtri.Triangulation(x, y * y_scale).triangles)
        t.set_mask(np.ptp(np.asarray(y)[t.triangles], axis=1) > max_dy)
        return ax.tripcolor(t, np.asarray(c), cmap="RdBu_r", norm=norm, shading=shading, rasterized=True)

    body, fin = d[d.part == "body"], d[d.part == "fin"]
    fig = plt.figure(figsize=(7.8, 5.4), layout="constrained")
    gs = fig.add_gridspec(2, 2, height_ratios=[1, 1.15])
    ax = fig.add_subplot(gs[0, :])
    th = np.degrees(np.arctan2(body.z, body.y))
    r_body = float(np.median(np.hypot(body.y, body.z)))
    sc = tri(ax, body.x.values, th.values, body.cp.values, 25.0, y_scale=np.radians(1.0) * r_body)
    ax.set_xlabel("先端からの距離 $x$ [m]")
    ax.set_ylabel("周方向角 $\\theta$ [deg]")
    ax.set_yticks([-180, -90, 0, 90, 180])
    ax.grid(False)
    phi = np.arctan2(fin.z, fin.y)
    k = fin[np.abs(np.angle(np.exp(1j * (phi - np.radians(45))))) < np.radians(20)]
    side = k.ny * (-np.sin(np.radians(45))) + k.nz * np.cos(np.radians(45))
    rho = np.hypot(k.y, k.z)
    # |side| > 0.5 keeps the two side faces; the tip-cap panels (radial normal, side ~ 0) belong to neither.
    for col, (sel, lab) in enumerate(((side > 0.5, "フィン（$+$ 側の面）"), (side < -0.5, "フィン（$-$ 側の面）"))):
        a = fig.add_subplot(gs[1, col])
        x, r, c = k.x[sel].values, rho[sel].values, k.cp[sel].values
        quads = fin_side_quads(x, r)
        if quads is not None:
            # one flat-coloured quad per panel (outline rebuilt from the centroid grid), no interpolation
            a.add_collection(PolyCollection(quads, array=c, cmap="RdBu_r", norm=norm, edgecolors="face", linewidths=0.2, rasterized=True))
            a.autoscale_view()
        else:
            tri(a, x, r, c, 1e9, shading="flat")
        a.set_xlabel("先端からの距離 $x$ [m]")
        a.set_ylabel("軸からの距離 [m]")
        a.text(0.03, 0.96, lab, transform=a.transAxes, va="top", color=INK, fontsize=9)
        a.set_aspect("equal", adjustable="datalim")
        a.grid(False)
    cb = fig.colorbar(sc, ax=fig.axes, shrink=0.85, pad=0.02, extend="both")
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
    fig, ax = plt.subplots(figsize=(5.2, 3.5))
    for m, c, mk in ((2.0, SERIES[0], "o"), (0.8, SERIES[1], "s")):
        s = mc[mc.mach == m].sort_values("wall_size_m")
        ax.plot(s.wall_size_m * 1e3, s.cn, "-" + mk, color=c, markersize=6, linewidth=1.0, label=f"$M={m:g}$")
        ax.axhline(s.cn.iloc[0], color=c, linewidth=0.7, linestyle=":")
    ax.set_xlim(0, 10.5)
    ax.set_xlabel("壁面の要素の大きさ wall_size [mm]（フィンの面は 1 mm）")
    ax.set_ylabel("$C_N$（$\\alpha=4^\\circ$）")
    ax.legend(loc="center right")
    fig.tight_layout()
    f.save(fig, "cfd_mesh_convergence")

    asy = pd.read_csv(data / "asymmetry_study.csv", comment="#")
    order = [("base", "半モデル"), ("roll45", "フィン\n45° 回転"), ("jst", "JST"), ("tight", "残差\n$10^{-9}$"), ("zm", "z 対称\nメッシュ")]
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


def orient_outward(p, t):
    """Triangles `t` (indices into points `p`) with a consistent winding whose normals point out of the body.

    The wall mesh of the CFD case is not wound consistently (the fin patches run opposite to the body).
    The winding is propagated across every edge shared by exactly two triangles; each connected piece
    is then turned so that its enclosed signed volume, taken about the piece's centroid, is positive.
    """
    from collections import deque

    t = np.asarray(t).copy()
    n = len(t)
    e = np.concatenate([t[:, [0, 1]], t[:, [1, 2]], t[:, [2, 0]]])
    owner = np.tile(np.arange(n), 3)
    key = np.sort(e, axis=1).astype(np.int64)
    key = key[:, 0] * len(p) + key[:, 1]
    order = np.argsort(key, kind="stable")
    _, first, count = np.unique(key[order], return_index=True, return_counts=True)
    a, b = order[first[count == 2]], order[first[count == 2] + 1]
    # two neighbours wound the same way traverse their shared edge in opposite directions
    flip_rel = e[a, 0] == e[b, 0]
    nbr = [[] for _ in range(n)]
    for i, j, s in zip(owner[a], owner[b], flip_rel):
        nbr[i].append((j, s))
        nbr[j].append((i, s))
    flip = np.full(n, -1, dtype=np.int8)
    comp = np.full(n, -1)
    for seed in range(n):
        if flip[seed] >= 0:
            continue
        flip[seed], comp[seed] = 0, seed
        q = deque([seed])
        while q:
            u = q.popleft()
            for v, s in nbr[u]:
                if flip[v] < 0:
                    flip[v], comp[v] = flip[u] ^ s, seed
                    q.append(v)
    t[flip == 1] = t[flip == 1][:, ::-1]
    tri = p[t]
    nrm = np.cross(tri[:, 1] - tri[:, 0], tri[:, 2] - tri[:, 0])
    cen = tri.mean(axis=1)
    for c in np.unique(comp):
        m = comp == c
        if np.einsum("ij,ij->", cen[m] - cen[m].mean(axis=0), nrm[m]) < 0:
            t[m] = t[m][:, ::-1]
    return t


def facing(tri3, elev, azim):
    """Mask of the triangles (n, 3, 3) whose normal points towards an orthographic camera at (elev, azim) [deg]."""
    e, a = np.radians(elev), np.radians(azim)
    eye = np.array([np.cos(e) * np.cos(a), np.cos(e) * np.sin(a), np.sin(e)])
    return np.cross(tri3[:, 1] - tri3[:, 0], tri3[:, 2] - tri3[:, 0]) @ eye > 0


def cfd_euler_rans(f, fields):
    """Euler and RANS of the sample rocket at one condition (doc/data/cfd/fields_rans, extracted by extract_cfd_rans_fields.py)."""
    from matplotlib.collections import LineCollection
    from matplotlib.colors import LinearSegmentedColormap, TwoSlopeNorm
    from matplotlib.tri import Triangulation

    div_cmap = LinearSegmentedColormap.from_list("div", ["#104281", "#5598e7", "#f0efec", "#f19a6c", "#b23f0f"])
    body_c = "#c9c8c3"
    models = [("euler", "Euler"), ("rans", "RANS")]
    col = {"euler": SEQ[4], "rans": "#b23f0f"}
    prof = {m: np.load(fields / f"{m}_profile.npz") for m, _ in models}
    minf, x_st = float(prof["euler"]["mach_inf"]), float(prof["euler"]["x"])
    note = dict(color=INK2, fontsize=8.5, linespacing=1.3)

    fig = plt.figure(figsize=(10, 7.8))
    gs = fig.add_gridspec(3, 3, width_ratios=[1, 1, 0.03], height_ratios=[1.0, 0.85, 1.0], hspace=0.36, wspace=0.12,
                          left=0.07, right=0.93, top=0.95, bottom=0.07)
    # (a) Mach number on the symmetry plane behind the body
    lev = np.linspace(0.0, 1.0, 41)
    norm = TwoSlopeNorm(vmin=0.0, vcenter=minf, vmax=1.0)
    for k, (m, lab) in enumerate(models):
        ax = fig.add_subplot(gs[0, k])
        d = np.load(fields / f"{m}_base.npz")
        tr = Triangulation(d["points"][:, 0], d["points"][:, 1], d["triangles"])
        mach = d["mach"].astype(float)
        ax.set_facecolor(body_c)
        cs = ax.tricontourf(tr, np.clip(mach, 0.0, 1.0), levels=lev, cmap=div_cmap, norm=norm)
        cs.set_rasterized(True)
        ax.tricontour(tr, mach, levels=[0.3], colors=INK, linewidths=0.6)
        ax.set_xlim(1.25, 2.1); ax.set_ylim(-0.2, 0.2)
        ax.set_aspect("equal"); ax.grid(False)
        ax.set_xlabel("$x$ [m]")
        if k == 0:
            ax.set_ylabel("$z$ [m]")
        ax.set_title(lab, color=INK, fontsize=11, loc="left")
    axs = fig.axes
    axs[0].text(1.56, 0.075, "底面を覆う円すい\n（流れは剝離しない）", ha="left", va="center", **note)
    axs[1].text(1.63, 0.0, "剝離域", ha="left", va="center", color="white", fontsize=9)
    axs[1].text(1.62, 0.13, "平らな底面と後流\n（黒線は $M=0.3$）", ha="left", va="center", **note)
    axs[0].text(1.32, -0.185, "灰色は機体（フィンの断面を含む）", ha="left", va="bottom", **note)
    cax = fig.add_subplot(gs[0, 2])
    cb = fig.colorbar(cs, cax=cax, ticks=[0.0, 0.4, 0.8, 1.0])
    cb.set_label("Mach 数", color=INK2)
    cb.ax.tick_params(labelsize=8)

    # (b) volume-mesh section on the symmetry plane at the mid-body station
    for k, (m, lab) in enumerate(models):
        ax = fig.add_subplot(gs[1, k])
        w = np.load(fields / f"{m}_wallmesh.npz")
        r = float(w["radius"])
        seg = w["segments"].astype(float)
        seg[:, :, 1] = (seg[:, :, 1] - r) * 1e3
        ax.add_collection(LineCollection(seg, colors=SEQ[7], linewidths=0.4, rasterized=True))
        ax.fill_between([0.72, 0.78], -4, 0, color=body_c, zorder=0)
        ax.set_xlim(0.726, 0.777); ax.set_ylim(-1.5, 24)
        ax.grid(False)
        ax.set_xlabel("$x$ [m]")
        if k == 0:
            ax.set_ylabel("壁からの距離 [mm]")
    axs = fig.axes[-2:]
    axs[0].text(0.7275, 22.5, "四面体", va="top", bbox=dict(boxstyle="round,pad=0.25", fc="white", ec="none", alpha=0.85), **note)
    axs[1].text(0.7275, 22.5, "壁の上のプリズム層", va="top", bbox=dict(boxstyle="round,pad=0.25", fc="white", ec="none", alpha=0.85), **note)

    ins = axs[1].inset_axes([0.56, 0.30, 0.41, 0.46])
    wr = np.load(fields / "rans_wallmesh.npz")
    sg = wr["segments"].astype(float)
    sg[:, :, 1] = (sg[:, :, 1] - float(wr["radius"])) * 1e3
    ins.add_collection(LineCollection(sg, colors=SEQ[7], linewidths=0.4, rasterized=True))
    ins.fill_between([0.72, 0.78], -1, 0, color=body_c, zorder=0)
    ins.set_xlim(0.742, 0.762); ins.set_ylim(-0.03, 0.6)
    ins.set_facecolor("white"); ins.grid(False)
    ins.tick_params(labelsize=7)
    ins.set_xticks([]); ins.set_yticks([0.0, 0.3, 0.6])
    ins.set_title("壁際の拡大 [mm]", fontsize=8, color=INK2, pad=7, bbox=dict(fc="white", ec="none", pad=1.5))
    axs[1].indicate_inset_zoom(ins, edgecolor=INK2, linewidth=0.6)

    # (c) velocity profile at the same station
    ax1 = fig.add_subplot(gs[2, 0])
    ax2 = fig.add_subplot(gs[2, 1])
    for m, lab in models:
        p = prof[m]
        ls = "--" if m == "euler" else "-"
        for ax in (ax1, ax2):
            ax.plot(p["v"], p["d"] * 1e3, color=col[m], label=lab, linewidth=1.8, linestyle=ls)
    ax1.set_ylim(0, 16); ax1.set_ylabel("壁からの距離 [mm]")
    ax2.set_yscale("log"); ax2.set_ylim(1e-4, 40)
    pr = prof["rans"]
    d99 = float(pr["d"][np.argmax(pr["v"] >= 0.99 * pr["v"][-1])]) * 1e3  # boundary-layer thickness delta_99 [mm]
    for ax, ttl in ((ax1, "(線形軸)"), (ax2, "(対数軸)")):
        ax.set_xlabel("$|V|/U_\\infty$"); ax.set_xlim(-0.03, 1.5)
        ax.axvline(0.0, color=INK2, linewidth=0.8)
        ax.set_title(f"$x={x_st:g}$ m の壁に垂直な速度分布 {ttl}", fontsize=9, color=INK2, loc="left")
        ax.axhline(d99, color=col["rans"], linewidth=0.7, linestyle=":")
        ax.text(0.03, d99 * (0.8 if ax is ax2 else 1.0) + (0.0 if ax is ax2 else 0.3), f"RANS の境界層厚さ $\\delta_{{99}}={d99:.1f}$ mm", ha="left", va="top" if ax is ax2 else "bottom", color=col["rans"], fontsize=8.5)
    ax1.legend(loc="upper left", fontsize=9)
    ax1.text(0.04, 0.22, "壁で $|V|=0$", transform=ax1.transAxes, color=col["rans"], fontsize=9)
    ax1.text(1.06, 3.0, "Euler は壁でも\n$|V|\\approx U_\\infty$（滑る）", color=col["euler"], fontsize=8.5, linespacing=1.3)
    f.save(fig, "cfd_euler_rans")


def cfd_fields(f, fields):
    """Sample-rocket CFD meshes and flow fields (doc/data/cfd/fields, extracted by extract_cfd_fields.py)."""
    from matplotlib.colors import LinearSegmentedColormap, TwoSlopeNorm
    from matplotlib.tri import Triangulation
    from mpl_toolkits.mplot3d.art3d import Poly3DCollection

    seq_cmap = LinearSegmentedColormap.from_list("seq", ["#f4f8fd", "#86b6ef", "#2a78d6", "#104281", "#08223f"])
    div_cmap = LinearSegmentedColormap.from_list("div", ["#104281", "#5598e7", "#f0efec", "#f19a6c", "#b23f0f"])
    body_c = "#c9c8c3"
    cases = [("m0.800_a04.00", "$M_\\infty=0.8$"), ("m2.000_a04.00", "$M_\\infty=2.0$")]
    VIEW_NOSE, VIEW_FIN = (20, 60), (25, 120)  # camera on the +y side: the outer face of the half model (y >= 0)

    def shaded(tri3, base):
        n = np.cross(tri3[:, 1] - tri3[:, 0], tri3[:, 2] - tri3[:, 0])
        n /= np.linalg.norm(n, axis=1, keepdims=True) + 1e-30
        light = np.array([0.3, 0.8, 0.5]) / np.linalg.norm([0.3, 0.8, 0.5])
        k = 0.55 + 0.45 * np.abs(n @ light)
        return np.clip(base * k[:, None], 0, 1)

    def frame3d(ax, pts):
        lo, hi = pts.min(axis=0), pts.max(axis=0)
        ax.set_xlim(lo[0], hi[0]); ax.set_ylim(lo[1], hi[1]); ax.set_zlim(lo[2], hi[2])
        ax.set_box_aspect(hi - lo)
        ax.set_axis_off()

    # surface mesh: nose tip and fin root (half model, outer side)
    w = np.load(fields / "mesh_wall.npz")
    p = w["points"].astype(float)
    t = orient_outward(p, w["triangles"])  # outward normals: faces turned away from the camera are culled below
    cen = p[t].mean(axis=1)
    fig = plt.figure(figsize=(10, 4.2))
    for k, (rect, sel, view, lab) in enumerate((
        ([0.0, 0.02, 0.4, 0.96], (cen[:, 0] < 0.09), VIEW_NOSE, "ノーズ先端"),
        ([0.42, 0.0, 0.58, 1.0], (cen[:, 0] > 1.27) & (cen[:, 0] < 1.40) & (cen[:, 2] > -0.01) & (cen[:, 1] > -1e-3) & (np.hypot(cen[:, 1], cen[:, 2]) < 0.085), VIEW_FIN, "フィンの付け根（前縁側）"),
    )):
        ax = fig.add_axes(rect, projection="3d")
        tri3 = p[t[sel]]
        frame3d(ax, tri3.reshape(-1, 3))
        tri3 = tri3[facing(tri3, *view)]
        face = shaded(tri3, np.array(matplotlib.colors.to_rgb("#e3eefb")))
        ax.add_collection3d(Poly3DCollection(tri3, facecolors=face, edgecolors=(0.06, 0.16, 0.32, 0.9), linewidths=0.25, rasterized=True))
        ax.set_proj_type("ortho")
        ax.view_init(elev=view[0], azim=view[1])
        ax.text2D(0.03, 0.95, lab, transform=ax.transAxes, color=INK2, fontsize=10)
    f.save(fig, "cfd_mesh_surface")

    # volume mesh: section on the symmetry plane
    m = np.load(fields / "mesh_symmetry.npz")
    tr = Triangulation(m["points"][:, 0], m["points"][:, 1], m["triangles"])
    fig, axs = plt.subplots(1, 3, figsize=(10.5, 3.6), gridspec_kw={"width_ratios": [1, 1.25, 1.25]})
    wins = [None, ((-0.6, 2.4), (-1.0, 1.0)), ((1.15, 1.85), (-0.2, 0.2))]
    lws = [0.15, 0.12, 0.15]
    for ax, win, lw in zip(axs, wins, lws):
        ax.set_facecolor(body_c)
        ax.triplot(tr, color=SEQ[7], linewidth=lw, rasterized=True)
        if win:
            ax.set_xlim(*win[0]); ax.set_ylim(*win[1])
        else:
            ax.set_facecolor(SURFACE)
        ax.set_aspect("equal")
        ax.grid(False)
        ax.set_xlabel("$x$ [m]")
    axs[0].set_ylabel("$z$ [m]")
    f.save(fig, "cfd_mesh_section")

    # Mach number on the symmetry plane, relative to the free stream
    fig, axs = plt.subplots(2, 1, figsize=(10, 5.6), sharex=True)
    for ax, (case, lab) in zip(axs, cases):
        d = np.load(fields / f"field_{case}.npz")
        minf = float(case[1:6])
        mach = d["sym_mach"].astype(float)
        span = float(np.percentile(np.abs(mach - minf), 96.0))
        norm = TwoSlopeNorm(vmin=minf - span, vcenter=minf, vmax=minf + span)
        lev = np.linspace(minf - span, minf + span, 41)
        ax.set_facecolor(body_c)
        cs = ax.tricontourf(tr, mach, levels=lev, cmap=div_cmap, norm=norm, extend="both")
        cs.set_rasterized(True)
        ax.set_xlim(-0.2, 2.15); ax.set_ylim(-0.32, 0.32)
        ax.set_aspect("equal"); ax.grid(False)
        ax.set_ylabel("$z$ [m]")
        ax.text(0.0, 1.03, lab + "、$\\alpha=4^\\circ$", transform=ax.transAxes, fontsize=10, color=INK)
        cb = fig.colorbar(cs, ax=ax, fraction=0.025, pad=0.01, ticks=np.round(np.linspace(minf - span, minf + span, 5), 2))
        cb.set_label("Mach 数", color=INK2)
        cb.ax.tick_params(labelsize=8)
    axs[-1].set_xlabel("$x$ [m]")
    f.save(fig, "cfd_mach_field")

    # wall pressure coefficient (half model mirrored to the full body), nose and tail, seen from the windward side
    fig = plt.figure(figsize=(10, 6.4))
    for k, (case, lab) in enumerate(cases):
        d = np.load(fields / f"field_{case}.npz")
        cp = d["wall_cp"].astype(float)
        pm = p * np.array([1, -1, 1])
        cpf_all = np.concatenate([cp[t].mean(axis=1)] * 2)
        # the mirror image reverses the winding: reverse the vertex order to keep the normals outward
        tri_all = np.concatenate([p[t], pm[t[:, ::-1]]])
        cen_all = tri_all.mean(axis=1)[:, 0]
        lim = float(np.percentile(np.abs(cpf_all), 99.5))
        norm = TwoSlopeNorm(vmin=-lim, vcenter=0.0, vmax=lim)
        for j, (sel, rect, view) in enumerate((
            (cen_all < 0.32, [0.0, 0.52 - 0.5 * k, 0.32, 0.44], (-20, -60)),
            (cen_all > 1.2, [0.3, 0.5 - 0.5 * k, 0.56, 0.48], (-22, -55)),
        )):
            ax = fig.add_axes(rect, projection="3d")
            frame3d(ax, tri_all[sel].reshape(-1, 3))
            sel = sel & facing(tri_all, *view)  # back-face culling; the depth sort hides the rest
            tri3 = tri_all[sel]
            face = shaded(tri3, div_cmap(norm(cpf_all[sel]))[:, :3])
            ax.add_collection3d(Poly3DCollection(tri3, facecolors=face, edgecolors=face, linewidths=0.3, rasterized=True))
            ax.set_proj_type("ortho")
            ax.view_init(elev=view[0], azim=view[1])
            if j == 0:
                ax.text2D(0.0, 1.0, lab + "、$\\alpha=4^\\circ$", transform=ax.transAxes, color=INK, fontsize=10)
        cax = fig.add_axes([0.9, 0.58 - 0.5 * k, 0.012, 0.34])
        cb = fig.colorbar(matplotlib.cm.ScalarMappable(norm=norm, cmap=div_cmap), cax=cax)
        cb.set_label("$C_p$", color=INK2)
        cb.ax.tick_params(labelsize=8)
    f.save(fig, "cfd_cp_surface")

    # convergence histories
    fig, axs = plt.subplots(1, 2, figsize=(10, 3.6))
    for i, case in enumerate(["m0.800_a00.00", "m0.800_a04.00", "m2.000_a00.00", "m2.000_a04.00"]):
        h = pd.read_csv(fields / f"history_{case}.csv", skipinitialspace=True)
        h.columns = [c.strip().strip('"') for c in h.columns]
        mach, alpha = float(case[1:6]), float(case[8:])
        lab = f"$M={mach:g}$、$\\alpha={alpha:g}^\\circ$"
        ls = "-" if alpha > 0 else "--"
        axs[0].plot(h["Inner_Iter"], h["rms[Rho]"], ls, color=SERIES[i // 2], lw=1.4, label=lab)
        if alpha > 0:
            axs[1].plot(h["Inner_Iter"], h["CL"], color=SERIES[i // 2], lw=1.4, label=f"$M={mach:g}$")
    axs[0].set_xlabel("反復回数"); axs[0].set_ylabel("密度の残差 $\\log_{10}$ RMS")
    axs[1].set_xlabel("反復回数"); axs[1].set_ylabel("揚力係数 $C_L$（$\\alpha=4^\\circ$）")
    axs[1].set_ylim(0.7, 1.4)
    axs[0].legend(); axs[1].legend()
    f.save(fig, "cfd_convergence")


def cfd_stl_check(f, data):
    """Direct-STL CFD versus the rebuilt (CAD) geometry and the Barrowman model (doc/data/cfd/stl_cross_check*)."""
    t = pd.read_csv(data / "stl_cross_check.csv", comment="#")
    clp = pd.read_csv(data / "stl_cross_check_clp.csv", comment="#")
    d = 0.1
    t["cn_n"] = -t.cz
    t["xcp"] = np.where(t.cz.abs() > 1e-6, t.cm * d / t.cz.where(t.cz.abs() > 1e-6, 1.0), np.nan)
    sty = {"barrowman": ("部品積み上げ法", SERIES[0]), "cad": ("CFD（形状を作り直し）", SERIES[1]), "stl": ("CFD（STL をそのまま）", SERIES[2])}
    fig, axs = plt.subplots(1, 3, figsize=(10.5, 3.6), gridspec_kw={"width_ratios": [1.15, 1.15, 0.9]})
    for mach, ls, mk in ((0.8, "-", "o"), (2.0, "--", "s")):
        b = t[(t.method == "barrowman") & (t.mach == mach)].sort_values("alpha_deg")
        axs[0].plot(b.alpha_deg, b.cn_n, ls, color=sty["barrowman"][1], lw=1.6)
        bb = b[b.alpha_deg >= 2]
        axs[1].plot(bb.alpha_deg, bb.xcp, ls, color=sty["barrowman"][1], lw=1.6)
        for meth in ("cad", "stl"):
            c = t[(t.method == meth) & (t.mach == mach) & (t.beta_deg == 0)].sort_values("alpha_deg")
            off = -0.15 if meth == "cad" else 0.15
            axs[0].plot(c.alpha_deg + off, c.cn_n, mk, color=sty[meth][1], markersize=6, mfc=sty[meth][1] if mach == 0.8 else "white", mew=1.5)
            cc = c[c.alpha_deg > 0]
            axs[1].plot(cc.alpha_deg + off, cc.xcp, mk, color=sty[meth][1], markersize=6, mfc=sty[meth][1] if mach == 0.8 else "white", mew=1.5)
    axs[0].set_xlabel("迎角 $\\alpha$ [deg]"); axs[0].set_ylabel("法線力係数 $C_N=-C_Z$")
    axs[1].set_xlabel("迎角 $\\alpha$ [deg]"); axs[1].set_ylabel("圧力中心 $x_{cp}$ [m]（先端から）")
    w = 0.25
    for k, meth in enumerate(("barrowman", "cad", "stl")):
        c = clp[clp.method == meth].sort_values("mach")
        xs = np.arange(len(c)) + (k - 1) * w
        axs[2].bar(xs, c.clp, width=w * 0.9, color=sty[meth][1], label=sty[meth][0])
    axs[2].set_xticks([0, 1]); axs[2].set_xticklabels(["$M=0.8$", "$M=2.0$"])
    axs[2].set_ylabel("ロール減衰 $C_{lp}$"); axs[2].grid(axis="x", visible=False)
    from matplotlib.lines import Line2D
    handles = [Line2D([], [], color=sty[m][1], marker="o" if m != "barrowman" else None, ls="-" if m == "barrowman" else "none", label=sty[m][0]) for m in sty]
    handles += [Line2D([], [], color=INK2, ls="-", marker="o", label="$M=0.8$（実線・塗り）"), Line2D([], [], color=INK2, ls="--", marker="s", mfc="white", label="$M=2.0$（破線・白抜き）")]
    fig.legend(handles=handles, loc="lower center", ncol=5, bbox_to_anchor=(0.5, -0.08), fontsize=8.5)
    fig.tight_layout()
    f.save(fig, "cfd_stl_check")


def rans_study(f, data):
    """RANS verification (doc/data/cfd/rans_*, committed): flat-plate friction, mesh study, RANS vs Euler across Mach."""
    if (data / "rans_flatplate_cf.csv").exists():
        cf = pd.read_csv(data / "rans_flatplate_cf.csv", comment="#")
        cf = cf[cf.x <= 1.0]
        rex = cf.x * 5e6
        fig, axs = plt.subplots(1, 2, figsize=(7.8, 3.3), sharey=True)
        axs[0].plot(rex, cf["tmr_cfl3d_sa"] * 1e3, color=SERIES[0], label="NASA TMR（CFL3D SA、545×385）")
        axs[0].plot(rex, cf["ks_local"] * 1e3, color=INK2, linestyle=":", linewidth=1.4, label="Kármán–Schoenherr（局所）")
        axs[0].plot(rex, cf["cf_m0.2"] * 1e3, "o", color=SERIES[1], markersize=6, label="IgnisYeet の RANS（273×193）")
        axs[1].plot(rex, cf["van_driest2"] * 1e3, color=SERIES[0], label="van Driest II（断熱壁）")
        axs[1].plot(rex, cf["cf_m2"] * 1e3, "o", color=SERIES[1], markersize=6, label="IgnisYeet の RANS（273×193）")
        for ax, m in zip(axs, ("0.2", "2")):
            ax.set_xscale("log")
            ax.set_xlabel("局所 Reynolds 数 $Re_x$")
            ax.text(0.97, 0.95, f"$M={m}$", transform=ax.transAxes, ha="right", va="top", color=INK2)
            ax.legend(loc="lower left")
        axs[0].set_ylabel("摩擦係数 $c_f$ [$10^{-3}$]")
        fig.tight_layout()
        f.save(fig, "rans_flatplate")

    if (data / "rans_mesh_study.json").exists() and (data / "rans_verification.json").exists():
        ms = pd.read_csv(data / "rans_mesh_study.csv", comment="#")
        v6 = json.load(open(data / "rans_verification.json"))["gates"]["V6_sample_mesh_convergence"]["mach"]
        fig, axs = plt.subplots(1, 3, figsize=(9.6, 3.3))
        cols = (("cn", "cn", "$C_N$ の外挿値との差 [%]", 100.0, True), ("ca_wall", "ca", "$C_{Fx}(\\mathrm{wall})$ の外挿値との差 [%]", 100.0, True),
                ("xcp_m", "xcp", "$x_{cp}$ の外挿値との差 [mm]", 1e3, False))
        for ax, (col, key, lab, sc, rel) in zip(axs, cols):
            for m, c, mk in ((0.8, SERIES[1], "s"), (2.0, SERIES[0], "o")):
                s = ms[ms.mach == m].sort_values("nodes")
                h = s.nodes.values ** (-1.0 / 3.0) * 1e2
                ex = v6["0.8" if m == 0.8 else "2"][key]["extrapolated"]
                d = (s[col].values / ex - 1.0) * sc if rel else (s[col].values - ex) * sc
                ax.plot(h, d, mk + "-", color=c, markersize=6, linewidth=1.2, label=f"$M={m:g}$")
                ax.plot([0, h.min()], [0, d[-1]], ":", color=c, linewidth=1.0)
            ax.plot([0], [0], "D", color=INK2, markersize=6, markerfacecolor="white", label="Richardson の外挿値")
            ax.axhline(0, color=INK2, linewidth=0.7)
            ax.set_xlim(left=-0.03)
            ax.set_xlabel("代表格子幅 $N_{node}^{-1/3}$ [$10^{-2}$]")
            ax.set_ylabel(lab)
        h_, l_ = axs[0].get_legend_handles_labels()
        fig.legend(h_[:2] + h_[-1:], l_[:2] + l_[-1:], loc="lower center", ncol=3, bbox_to_anchor=(0.5, 0.0))
        fig.tight_layout(rect=(0, 0.08, 1, 1))
        f.save(fig, "rans_mesh_study")

    if (data / "rans_table.csv").exists():
        t = pd.read_csv(data / "rans_table.csv", comment="#")
        rc = pd.read_csv(data / "rans_table_roughness.csv", comment="#") if (data / "rans_table_roughness.csv").exists() else None
        ve = pd.read_csv(data / "rans_vs_euler.csv", comment="#") if (data / "rans_vs_euler.csv").exists() else None
        a4 = t[t.alpha_deg == 4].sort_values("mach")
        a0 = t[t.alpha_deg == 0].sort_values("mach")
        fig, axs = plt.subplots(1, 3, figsize=(9.6, 3.4))
        sty_e = dict(color=SERIES[0], marker="o", markersize=6, linewidth=1.2)
        sty_r = dict(color=SERIES[1], marker="s", markersize=6, linewidth=1.2)
        axs[0].plot(a4.mach, a4.euler_cn, label="Euler（2 mm）", **sty_e)
        axs[0].plot(a4.mach, a4.rans_cn, label="RANS（5 mm）", **sty_r)
        axs[1].plot(a4.mach, a4.euler_xcp_m, **sty_e)
        axs[1].plot(a4.mach, a4.rans_xcp_m, **sty_r)
        if ve is not None:
            axs[0].plot(ve.mach, ve.rans_cn_a4, "^", color=SERIES[2], markersize=7, label="RANS（3 mm）")
            axs[1].plot(ve.mach, ve.rans_xcp_m, "^", color=SERIES[2], markersize=7)
        axs[2].plot(a0.mach, a0.euler_ca_off, **sty_e)
        axs[2].plot(a0.mach, a0.rans_ca_off, color=SERIES[1], marker="s", markersize=6, linewidth=1.0, linestyle="--", markerfacecolor="white", label="RANS（5 mm、粗さとフィンの縁の増分なし）")
        if rc is not None:
            r0 = rc[rc.alpha_deg == 0].sort_values("mach")
            axs[2].plot(r0.mach, r0.rans_ca_off_corrected, label="RANS（5 mm、増分を加えた再計算）", **sty_r)
        axs[0].set_ylabel("$C_N$（$\\alpha=4^\\circ$）")
        axs[1].set_ylabel("$x_{cp}$（$\\alpha=4^\\circ$）[m]")
        axs[2].set_ylabel("軸力係数 $C_A$（$\\alpha=0$、燃焼後）")
        for ax in axs:
            ax.set_xlabel("Mach 数 $M$")
            ax.set_xlim(0, 2.2)
        h0, l0 = axs[0].get_legend_handles_labels()
        h2, l2 = axs[2].get_legend_handles_labels()
        fig.legend(h0 + h2, l0 + l2, loc="lower center", ncol=5, bbox_to_anchor=(0.5, 0.0))
        fig.tight_layout(rect=(0, 0.08, 1, 1))
        f.save(fig, "rans_vs_euler")


def read_stl(path):
    """Triangles of a binary or ASCII STL as an array of shape (n, 3, 3)."""
    data = Path(path).read_bytes()
    if data[:5] == b"solid" and b"facet" in data[:1024]:
        v = [list(map(float, l.split()[1:4])) for l in data.decode().splitlines() if l.strip().startswith("vertex")]
        return np.array(v).reshape(-1, 3, 3)
    n = int.from_bytes(data[80:84], "little")
    rec = np.frombuffer(data[84:84 + 50 * n], dtype=np.dtype([("n", "<f4", 3), ("v", "<f4", (3, 3)), ("a", "<u2")]))
    return rec["v"].astype(float)


# >>> geometry
# ---------------------------------------------------------------- geometry chapter (doc/data/geometry, examples/)

EXAMPLES = Path(__file__).resolve().parents[2] / "examples"
GDATA = Path(__file__).resolve().parents[1] / "data" / "geometry"
FACE_COLORS = ["#86b6ef", "#f0a37d", "#7fd2b4", "#f3cd6b", "#eeb0c9", "#a79fd8"]


def g_json(name):
    return json.loads((GDATA / name / "geometry.json").read_text())


def g_body(tris, g):
    """Triangles (n, 3, 3) in the frame of the file -> (distance aft of the nose tip, y, z) of the body frame."""
    fr = g["frame"]
    c, aft = np.array(fr["axis_point"]), np.array(fr["aft_direction"])
    ey, ez = np.array(g["body_y"]), np.array(g["body_z"])
    d = tris - c
    return np.stack([d @ aft - fr["nose_offset"], d @ ey, d @ ez], axis=-1)


def g_rot(az, el):
    a, e = np.radians(az), np.radians(el)
    rz = np.array([[np.cos(a), -np.sin(a), 0], [np.sin(a), np.cos(a), 0], [0, 0, 1]])
    rx = np.array([[1, 0, 0], [0, np.cos(e), -np.sin(e)], [0, np.sin(e), np.cos(e)]])
    return rx @ rz


def g_clip_x(tri, x0, x1):
    """Triangles cut to the slab x0 <= X <= x1, as polygons of five vertices (the last one repeated)."""
    lo, hi = tri[:, :, 0].min(axis=1), tri[:, :, 0].max(axis=1)
    inside = (lo >= x0) & (hi <= x1)
    cross = ~inside & (hi >= x0) & (lo <= x1)
    out = [np.concatenate([tri[inside], tri[inside][:, :1].repeat(2, axis=1)], axis=1)]
    for q in tri[cross]:
        poly = [v for v in q]
        for axis_sign, lim in ((1.0, x0), (-1.0, x1)):
            res = []
            for i, a in enumerate(poly):
                b = poly[(i + 1) % len(poly)]
                da, db = axis_sign * (a[0] - lim), axis_sign * (b[0] - lim)
                if da >= 0:
                    res.append(a)
                if (da >= 0) != (db >= 0):
                    res.append(a + (b - a) * (da / (da - db)))
            poly = res
            if len(poly) < 3:
                break
        if len(poly) >= 3:
            poly = (poly + [poly[-1]] * 5)[:5]
            out.append(np.array(poly)[None])
    return np.concatenate(out)


def g_draw_tris(ax, tri, colors, view=(0, 0), edge=None, lw=0.2, light=(-0.4, -0.6, 0.7)):
    """Orthographic view of triangles given in the body frame (X aft along the axis, Y, Z), depth-sorted.

    Screen: horizontal = X', vertical = Z'' after turning by `view` = (azimuth, elevation) in degrees."""
    from matplotlib.collections import PolyCollection

    r = g_rot(*view)
    p = tri @ r.T  # (n, 3, 3): X', depth, Z''
    n = np.cross(p[:, 1] - p[:, 0], p[:, 2] - p[:, 0])
    n /= np.linalg.norm(n, axis=1, keepdims=True) + 1e-30
    li = np.array(light) / np.linalg.norm(light)
    sh = (0.5 + 0.5 * np.clip(np.abs(n @ li), 0, 1) ** 1.1)[:, None]
    order = np.argsort(-p[:, :, 1].mean(axis=1))
    face = np.clip(np.asarray(colors)[order] * sh[order], 0, 1)
    poly = p[order][:, :, [0, 2]]
    ec = edge if edge is not None else face
    col = PolyCollection(poly, facecolors=face, edgecolors=ec, linewidths=lw if edge is not None else 0.25)
    col.set_rasterized(True)
    ax.add_collection(col)
    lo, hi = poly.reshape(-1, 2).min(axis=0), poly.reshape(-1, 2).max(axis=0)
    ax.set_xlim(lo[0], hi[0])
    ax.set_ylim(lo[1], hi[1])
    ax.set_aspect("equal")
    ax.set_axis_off()
    return r


def g_rgb(c):
    return np.array(matplotlib.colors.to_rgb(c))


def geometry_cad_to_triangles(f):
    """The sample rocket as B-rep faces, as the triangles of the CAD tessellation and as the facets of the STL."""
    gc, gs = g_json("sample_cad"), g_json("sample_stl")
    faces = json.loads((GDATA / "sample_cad_faces.json").read_text())
    coarse = np.load(GDATA / "sample_cad_coarse.npz")
    tail = np.load(GDATA / "sample_cad_tail.npz")
    stl = read_stl(EXAMPLES / "sample_rocket.stl") * 0.001

    def cad_tris(z):
        return z["verts"].astype(float)[z["tris"]]

    ct = g_body(cad_tris(coarse), gc)
    fc = np.array([g_rgb(FACE_COLORS[k % 6]) for k in coarse["face"]])
    tt = g_body(cad_tris(tail), gc)
    st = g_body(stl, gs)
    x0, x1 = 1.33, 1.50
    ptt, pst = g_clip_x(tt, x0, x1), g_clip_x(st, x0, x1)

    fig = plt.figure(figsize=(8.0, 5.0))
    ax0 = fig.add_axes([0.0, 0.60, 1.0, 0.33])
    r0 = g_draw_tris(ax0, ct, fc, view=(-28, 22))
    ax1 = fig.add_axes([0.0, 0.0, 0.5, 0.54])
    ax2 = fig.add_axes([0.5, 0.0, 0.5, 0.54])
    edge = (0.05, 0.10, 0.25, 0.6)
    col = g_rgb("#9ec5f4")
    view = (-52, 24)
    g_draw_tris(ax1, ptt, np.tile(col, (len(ptt), 1)), view=view, edge=edge, lw=0.1)
    g_draw_tris(ax2, pst, np.tile(col, (len(pst), 1)), view=view, edge=edge, lw=0.4)
    fig.text(0.0, 0.985, f"CAD ファイルの面（{faces['faces']} 面、色は面ごと）", fontsize=11, color=INK, va="center")
    fig.text(0.0, 0.575, f"CAD を分割した三角形（全体で {faces['triangles_fine']:,} 個）", fontsize=11, color=INK, va="center")
    fig.text(0.5, 0.575, f"STL の三角形（全体で {len(stl):,} 個）", fontsize=11, color=INK, va="center")
    fl = {i["index"]: i["bbox_m"] for i in faces["face_list"]}

    def centre(idx):
        b = np.array([fl[i] for i in idx])
        pt = np.array([(b[:, 0].min() + b[:, 3].max()) / 2, (b[:, 1].min() + b[:, 4].max()) / 2, (b[:, 2].min() + b[:, 5].max()) / 2])
        q = g_body(pt[None, None, :], gc)[0, 0] @ r0.T
        return q[0], q[2]

    for idx, label, off in (([40, 41, 42, 43], "ランチラグ", (0, 24)), ([36, 37, 38, 39], "レールボタン", (0, 24))):
        ax0.annotate(label, centre(idx), xytext=off, textcoords="offset points", fontsize=9, color=INK2, ha="center",
                     arrowprops=dict(arrowstyle="-", color=INK2, lw=0.7))
    # the part shown below
    q = (ct @ r0.T)[(ct[:, :, 0].min(axis=1) >= x0)]
    lo, hi = q[:, :, [0, 2]].reshape(-1, 2).min(axis=0), q[:, :, [0, 2]].reshape(-1, 2).max(axis=0)
    ax0.add_patch(plt.Rectangle((lo[0] - 0.004, lo[1] - 0.004), hi[0] - lo[0] + 0.008, hi[1] - lo[1] + 0.008, fill=False, ec=INK2, lw=0.9, ls="--"))
    ax0.annotate("下に拡大", (lo[0] - 0.004, lo[1] - 0.004), xytext=(-4, 0), textcoords="offset points", fontsize=9, color=INK2, ha="right", va="bottom", annotation_clip=False)
    f.save(fig, "geometry_cad_to_triangles")


def geometry_axis(f):
    """The principal axis of the triangles of the sample rocket (turned in space) and the cues for the nose direction."""
    from matplotlib.collections import PolyCollection

    ax_data = json.loads((GDATA / "sample_axis.json").read_text())
    R = np.array(ax_data["rotation"])
    tris = read_stl(EXAMPLES / "sample_rocket.stl") * 0.001 @ R.T
    c, e1 = np.array(ax_data["centroid_m"]), np.array(ax_data["principal_axis"])
    lam = ax_data["eigenvalues_m2_relative"]
    gs = g_json("sample_stl")
    fig = plt.figure(figsize=(8.0, 3.7))
    ax = fig.add_axes([0.07, 0.13, 0.36, 0.80])
    poly = tris[:, :, [0, 2]]
    pc = PolyCollection(poly, facecolors="#b8d3f5", edgecolors="#b8d3f5", linewidths=0.3)
    pc.set_rasterized(True)
    ax.add_collection(pc)
    ax.set_xlim(-0.15, 1.1)
    ax.set_ylim(-1.25, 0.2)
    ax.set_aspect("equal")
    s = np.array([-0.82, 0.82])
    ax.plot(c[0] + s * e1[0], c[2] + s * e1[2], color=SERIES[1], lw=1.6, ls=(0, (6, 3)), zorder=3)
    ax.plot(*c[[0, 2]], "o", color=SERIES[1], ms=5, zorder=4)
    ax.annotate("面積で重みを付けた重心 $\\mathbf{c}$", c[[0, 2]], xytext=(52, 22), textcoords="offset points", fontsize=9, color=INK2, ha="center",
                arrowprops=dict(arrowstyle="-", color=INK2, lw=0.7))
    ax.annotate("第 1 主軸", (c[0] - 0.45 * e1[0], c[2] - 0.45 * e1[2]), xytext=(-12, -16), textcoords="offset points", fontsize=9, color=SERIES[1], ha="right")
    ax.set_xlabel("$x$ [m]（STL の座標）")
    ax.set_ylabel("$z$ [m]")
    ax.text(0.97, 0.97, f"分散の割合\n第 1 軸 {100 * lam[0]:.1f} %\n第 2・3 軸 {100 * lam[1]:.1f} %、{100 * lam[2]:.1f} %", transform=ax.transAxes, ha="right", va="top",
            fontsize=8.5, color=INK2, linespacing=1.35)
    # right: the cues for the direction
    ax2 = fig.add_axes([0.52, 0.34, 0.46, 0.44])
    prof = pd.DataFrame(gs["profile"])
    L = gs["length"]
    xs = prof.x / L
    ax2.fill_between(xs, -prof.r * 1000, prof.r * 1000, color="#b8d3f5", lw=0)
    fin = prof[prof.fin_span > 0]
    ax2.fill_between(fin.x / L, (fin.r) * 1000, (fin.r + fin.fin_span) * 1000, color="#f4c3a8", lw=0)
    ax2.fill_between(fin.x / L, -(fin.r) * 1000, -(fin.r + fin.fin_span) * 1000, color="#f4c3a8", lw=0)
    ax2.axvspan(0.5 - 0.08, 0.5 + 0.08, color=GRID, alpha=0.8, lw=0, zorder=0)
    ax2.text(0.5, 190, "判定に使わない範囲\n（中央から 0.08）", ha="center", va="top", fontsize=8.5, color=INK2, linespacing=1.3)
    pf = ax_data["fin_centre_fraction"]
    ax2.plot([pf, pf], [-175, 175], color=SERIES[1], lw=1.4)
    ax2.annotate(f"フィン根元の中央\n全長の {100 * pf:.0f} %", (pf, 150), xytext=(-8, 4), textcoords="offset points", ha="right", fontsize=8.5, color=SERIES[1], linespacing=1.3)
    ax2.set_xlim(0, 1)
    ax2.set_ylim(-190, 190)
    ax2.set_yticks([])
    ax2.grid(False)
    ax2.spines["left"].set_visible(False)
    ax2.set_xlabel("軸に沿った位置 / 全長")
    ax2.set_title("向きの判定（1. フィンの位置）", fontsize=10, color=INK, loc="left")
    e0, e1_ = ax_data["end_radius_sum_nose_end_m"] * 1000, ax_data["end_radius_sum_tail_end_m"] * 1000
    fig.text(0.52, 0.12, f"2. 端の半径（フィンで決まらないとき）：両端の 2 断面の半径の和は {e0:.1f} mm と {e1_:.1f} mm。\n    小さい方の端をノーズとする。", fontsize=8.5, color=INK2, va="center", linespacing=1.4)
    f.save(fig, "geometry_axis")


def geometry_slices(f):
    """Slices of the sample STL: the plane positions, four sections with the body radius and the fin points, and r_b(x)."""
    from matplotlib.collections import LineCollection, PolyCollection

    gs = g_json("sample_stl")
    prov = json.loads((GDATA / "provenance.json").read_text())["sections"]
    sec = np.load(GDATA / "sample_sections.npz")
    st = g_body(read_stl(EXAMPLES / "sample_rocket.stl") * 0.001, gs)
    prof = pd.DataFrame(gs["profile"])
    L, delta = gs["length"], prov["fin_threshold_m"]
    stations = prov["stations"]
    fig = plt.figure(figsize=(8.0, 4.7))
    ax = fig.add_axes([0.07, 0.78, 0.91, 0.19])
    pc = PolyCollection(st[:, :, :2], facecolors="#b8d3f5", edgecolors="#b8d3f5", linewidths=0.3)
    pc.set_rasterized(True)
    ax.add_collection(pc)
    for x in prof.x[::20]:
        ax.axvline(x, color=INK2, lw=0.3, alpha=0.5)
    for s in stations:
        ax.axvline(s["x_m"], color=SERIES[1], lw=1.4)
        ax.text(s["x_m"], 0.175, s["name"], ha="center", va="bottom", fontsize=11, color=SERIES[1], fontweight="bold")
    ax.set_xlim(0, L)
    ax.set_ylim(-0.17, 0.2)
    ax.set_aspect("equal")
    ax.set_axis_off()
    fig.text(0.07, 0.985, "機軸に垂直な断面", fontsize=9.5, color=INK2, va="center")
    w = 0.165
    for k, s in enumerate(stations):
        a = fig.add_axes([0.07 + k * 0.235 + 0.02, 0.43, w, w * 8.0 / 4.7])
        seg = sec[s["name"]].astype(float) * 1000
        a.add_collection(LineCollection(seg, colors=INK2, linewidths=0.9))
        rb = s["r_body_program_m"] * 1000
        th = np.linspace(0, 2 * np.pi, 361)
        a.plot(rb * np.cos(th), rb * np.sin(th), color=SERIES[0], lw=1.3, ls=(0, (5, 3)))
        lim = (rb + delta * 1000)
        pts = seg.reshape(-1, 2)
        far = pts[np.hypot(pts[:, 0], pts[:, 1]) > lim]
        a.plot(far[:, 0], far[:, 1], ".", color=SERIES[1], ms=2.6)
        a.set_xlim(-160, 160)
        a.set_ylim(-160, 160)
        a.set_aspect("equal")
        a.grid(False)
        a.set_xticks([-100, 0, 100])
        a.set_yticks([-100, 0, 100] if k == 0 else [])
        a.tick_params(labelsize=8)
        for sp in a.spines.values():
            sp.set_visible(True)
            sp.set_color("#bdbcb6")
        a.set_title(f"{s['name']}：$x={s['x_m']:.2f}$ m\n$r_b={rb:.1f}$ mm", fontsize=9.5, color=INK)
        if k == 0:
            a.set_ylabel("mm")
    a = fig.get_axes()[-1]
    a.annotate("フィン点", far[np.argmin(far[:, 0] + far[:, 1])], xytext=(-102, 0), textcoords="data", fontsize=8.5, color=SERIES[1], ha="center", va="center",
               arrowprops=dict(arrowstyle="-", color=SERIES[1], lw=0.7))
    a.text(0, -148, "破線は $r_b$", fontsize=8.5, color=SERIES[0], ha="center")
    ax3 = fig.add_axes([0.07, 0.105, 0.91, 0.24])
    ax3.plot(prof.x, prof.r * 1000, color=SERIES[0], label="胴体半径 $r_b$")
    fin = prof[prof.fin_span > 0]
    ax3.plot(fin.x, (fin.r + fin.fin_span) * 1000, color=SERIES[1], lw=1.4, label="フィンの外側の半径 $r_b+$ フィン高さ")
    for s in stations:
        ax3.plot(s["x_m"], s["r_body_program_m"] * 1000, "o", color=SERIES[1], ms=5, zorder=4)
        ax3.annotate(s["name"], (s["x_m"], s["r_body_program_m"] * 1000), xytext=(0, -15), textcoords="offset points", ha="center", fontsize=10, color=SERIES[1], fontweight="bold")
    ax3.set_xlim(0, L)
    ax3.set_ylim(0, 165)
    ax3.set_xlabel("先端からの距離 $x$ [m]")
    ax3.set_ylabel("半径 [mm]")
    ax3.legend(loc="upper left", ncol=1)
    f.save(fig, "geometry_slices")


def g_fin_outline(tri_body, g):
    """Fin of the triangles (body frame): the projection (x, height above the axis) of the triangles that lie in the plane of one fin,
    and the half width of the fin."""
    rb, span = g["fins"]["body_radius"], g["fins"]["span"]
    v = tri_body.reshape(-1, 3)
    r = np.hypot(v[:, 1], v[:, 2])
    th = np.arctan2(v[:, 2], v[:, 1])
    far = r > rb + 0.7 * span
    ph = th[far][0]
    near = far & (np.cos(th - ph) > 0.97)
    ph = np.angle(np.exp(1j * th[near]).mean())
    c, s_ = np.cos(ph), np.sin(ph)
    d = -tri_body[:, :, 1] * s_ + tri_body[:, :, 2] * c
    s = tri_body[:, :, 1] * c + tri_body[:, :, 2] * s_
    half = np.abs((-v[:, 1] * s_ + v[:, 2] * c)[near]).max() * 1.05
    keep = (np.abs(d) <= half).all(axis=1) & (tri_body[:, :, 0].min(axis=1) > 0)
    return np.stack([tri_body[keep][:, :, 0], s[keep]], axis=-1), half


def geometry_fins(f):
    """The fin of three inputs in its plane, the trapezoid that is extracted from it and its dimensions."""
    from matplotlib.collections import PolyCollection

    sc, ss, sf = g_json("sample_cad"), g_json("sample_stl"), g_json("basic_finner_stl")
    tail = np.load(GDATA / "sample_cad_tail.npz")
    cad = tail["verts"].astype(float)[tail["tris"]]
    bf = read_stl(EXAMPLES / "basic_finner" / "basic_finner.stl") * 0.001
    cases = [
        ("サンプル機体（STL）", ss, g_body(read_stl(EXAMPLES / "sample_rocket.stl") * 0.001, ss)),
        ("サンプル機体（CAD）", sc, g_body(cad, sc)),
        ("Basic Finner（STL）", sf, g_body(bf, sf)),
    ]
    fig, axs = plt.subplots(1, 3, figsize=(8.2, 3.3), gridspec_kw={"wspace": 0.12})
    for ax, (ttl, g, tb) in zip(axs, cases):
        fn = g["fins"]
        sil, half = g_fin_outline(tb, g)
        rb, a, b, sp, m, xle, tk = fn["body_radius"], fn["root_chord"], fn["tip_chord"], fn["span"], fn["sweep"], fn["x_le_root"], fn["thickness"]
        u = 1000.0
        pc = PolyCollection(sil * u, facecolors="#b8d3f5", edgecolors="#b8d3f5", linewidths=0.3)
        pc.set_rasterized(True)
        ax.add_collection(pc)
        xs = np.array([xle, xle + m, xle + m + b, xle + a]) * u
        ys = np.array([rb, rb + sp, rb + sp, rb]) * u
        ax.fill_between([xle * u - 0.2 * a * u, (xle + a) * u + 0.2 * a * u], rb * u - 0.4 * sp * u, rb * u, color=GRID, lw=0, zorder=3)
        ax.plot(np.r_[xs, xs[0]], np.r_[ys, ys[0]], color=SERIES[1], lw=1.6)
        ax.set_xlim(xle * u - 0.12 * a * u, (xle + a) * u + 0.12 * a * u)
        ax.set_ylim((rb - 0.3 * sp) * u, (rb + 1.6 * sp) * u)
        ax.set_aspect("equal")
        ax.grid(False)
        ax.set_xticks([])
        ax.set_yticks([])
        for sp_ in ax.spines.values():
            sp_.set_visible(False)
        arr = dict(arrowstyle="<->", color=INK, lw=0.8, shrinkA=0, shrinkB=0)
        yb = (rb - 0.1 * sp) * u
        ax.annotate("", (xle * u, yb), (xle * u + a * u, yb), arrowprops=arr)
        ax.text((xle + a / 2) * u, yb - 0.04 * sp * u, f"$a$ = {a * u:.1f} mm", ha="center", va="top", fontsize=9, color=INK)
        yt = (rb + 1.12 * sp) * u
        ax.annotate("", ((xle + m) * u, yt), ((xle + m + b) * u, yt), arrowprops=arr)
        ax.text((xle + m + b / 2) * u, yt + 0.04 * sp * u, f"$b$ = {b * u:.1f} mm", ha="center", va="bottom", fontsize=9, color=INK)
        if m * u > 1.0:
            ym = (rb + 1.36 * sp) * u
            ax.plot([xle * u, xle * u], [rb * u, ym + 0.02 * sp * u], color=INK2, lw=0.5, ls=":")
            ax.annotate("", (xle * u, ym), ((xle + m) * u, ym), arrowprops=arr)
            ax.text((xle + m / 2) * u, ym + 0.04 * sp * u, f"$m$ = {m * u:.1f} mm", ha="center", va="bottom", fontsize=9, color=INK)
            ax.plot([(xle + m) * u] * 2, [(rb + sp) * u, ym + 0.02 * sp * u], color=INK2, lw=0.5, ls=":")
        xr = (xle + a) * u + 0.07 * a * u
        ax.annotate("", (xr, rb * u), (xr, (rb + sp) * u), arrowprops=arr)
        ax.text(xr + 0.02 * a * u, (rb + sp / 2) * u, f"$s$ = {sp * u:.1f} mm", ha="left", va="center", fontsize=9, color=INK, rotation=90)
        ax.text(0.5, -0.03, f"$t$ = {tk * u:.2f} mm（中央値）" + ("" if m * u > 1.0 else "、$m$ = 0") , transform=ax.transAxes, ha="center", va="top", fontsize=9, color=INK2)
        ax.set_anchor("N")
        ax.set_title(ttl, fontsize=10, color=INK, pad=4)
        ax.text(xle * u - 0.1 * a * u, (rb - 0.3 * sp) * u, "胴体", fontsize=8.5, color=INK2, va="bottom", zorder=4)
    f.save(fig, "geometry_fins")


def g_differences(gs, gc):
    """Relative differences (%) CAD against STL of the extracted quantities: lengths against the STL value, the leading edge
    position against the length and the sweep against the root chord."""
    fs, fc = gs["fins"], gc["fins"]
    q = [
        ("全長 $L$", gs["length"], gc["length"], gs["length"]),
        ("基準直径 $d$", 2 * gs["ref_radius"], 2 * gc["ref_radius"], 2 * gs["ref_radius"]),
        ("ノーズ長 $L_n$", gs["nose"]["length"], gc["nose"]["length"], gs["nose"]["length"]),
        ("根元翼弦 $a$", fs["root_chord"], fc["root_chord"], fs["root_chord"]),
        ("端翼弦 $b$", fs["tip_chord"], fc["tip_chord"], fs["tip_chord"]),
        ("スパン $s$", fs["span"], fc["span"], fs["span"]),
        ("後退長 $m$（$a$ 比）", fs["sweep"], fc["sweep"], fs["root_chord"]),
        ("前縁位置 $x_{le}$（$L$ 比）", fs["x_le_root"], fc["x_le_root"], gs["length"]),
        ("厚さ $t$", fs["thickness"], fc["thickness"], fs["thickness"]),
    ]
    return [(n, 100 * abs(c - s) / ref) for n, s, c, ref in q]


def geometry_compare(f):
    """CAD against STL: difference of the radius profile along the body, and of the extracted quantities."""
    bs, bc = g_json("basic_finner_stl"), g_json("basic_finner_cad")
    ss, sc = g_json("sample_stl"), g_json("sample_cad")
    bar = pd.read_csv(GDATA / "basic_finner_barrowman.csv", comment="#")
    fig, axs = plt.subplots(1, 2, figsize=(8.2, 3.4), gridspec_kw={"width_ratios": [1, 1.1], "wspace": 1.0})
    ax = axs[0]
    for (a, b, lab, col) in ((bs, bc, "Basic Finner（同じモデル）", SERIES[0]), (ss, sc, "サンプル機体（モデルが違う）", SERIES[1])):
        ra = np.array([p["r"] for p in a["profile"]])
        rb_ = np.array([p["r"] for p in b["profile"]])
        x = np.array([p["x"] for p in a["profile"]]) / a["length"]
        ax.plot(x, 100 * (rb_ - ra) / a["ref_radius"], color=col, lw=1.4, label=lab)
    ax.axhline(0, color=INK2, lw=0.6)
    ax.set_xlabel("先端からの距離 / 全長")
    ax.set_ylabel("胴体半径の差 (CAD − STL) / $r_{\\max}$ [%]")
    ax.legend(loc="upper center", bbox_to_anchor=(0.5, -0.23), fontsize=8.5)
    ax.set_xlim(0, 1)
    # right: relative differences of the quantities
    ax = axs[1]
    d_b, d_s = g_differences(bs, bc), g_differences(ss, sc)
    cna = np.abs(bar.cna_cad - bar.cna_stl) / bar.cna_stl
    xcp = np.abs(bar.xcp_cad_m - bar.xcp_stl_m) / bs["length"]
    d_b += [("$C_{N\\alpha}$（部品積み上げ法）", 100 * cna.max()), ("圧力中心 $x_{cp}$（$L$ 比）", 100 * xcp.max())]
    names = [n for n, _ in d_b]
    y = np.arange(len(names))[::-1]
    floor = 1e-4
    vb = np.maximum([v for _, v in d_b], floor)
    vs = np.maximum([v for _, v in d_s], floor)
    ax.plot(vb, y, "o", color=SERIES[0], ms=6, label="Basic Finner", zorder=3)
    ax.plot(vs, y[: len(vs)], "o", mfc="white", mec=SERIES[1], mew=1.6, ms=6, label="サンプル機体", zorder=3)
    for yy, v in zip(y, vb):
        ax.plot([floor, v], [yy, yy], color=GRID, lw=1.0, zorder=1)
    ax.set_xscale("log")
    ax.set_xlim(floor, 30)
    ax.set_yticks(y)
    ax.set_yticklabels(names, fontsize=8.5)
    ax.set_xlabel("CAD と STL の差 [%]")
    ax.grid(axis="y", visible=False)
    ax.legend(loc="upper center", bbox_to_anchor=(0.45, -0.2), ncol=2, fontsize=8.5)
    f.save(fig, "geometry_compare")


def geometry_sensitivity(f):
    """Fin values of the sample rocket against the number of slices and the fin threshold (STL and CAD)."""
    ns = pd.read_csv(GDATA / "sweep_n_slices.csv", comment="#")
    th = pd.read_csv(GDATA / "sweep_fin_threshold.csv", comment="#")
    th = th[th.fin_threshold > 0]
    th = th[th.fins == 4]
    rows = [("root_chord_m", "根元翼弦 [mm]", 200.0), ("sweep_m", "後退長 [mm]", 80.0),
            ("x_le_root_m", "前縁位置 [mm]", 1300.0)]
    fig, axs = plt.subplots(len(rows), 2, figsize=(8.0, 4.2), sharex="col", gridspec_kw={"hspace": 0.2, "wspace": 0.28})
    for i, (col, lab, design) in enumerate(rows):
        for j, (df, key) in enumerate(((ns, "n_slices"), (th, "fin_threshold"))):
            ax = axs[i, j]
            ax.axhline(design, color=INK2, lw=0.9, ls=":")
            for inp, c, mk, name in (("stl", SERIES[0], "o", "STL"), ("cad", SERIES[1], "s", "CAD")):
                d = df[df.input == inp]
                ax.plot(d[key], d[col] * 1000, marker=mk, ms=4.5, color=c, label=name)
            dflt = 600 if key == "n_slices" else 0.05
            ax.axvline(dflt, color=SERIES[2], lw=0.9, ls="--")
            if key == "n_slices":
                ax.set_xscale("log")
            if j == 0:
                ax.set_ylabel(lab, fontsize=8.5)
            if i == 0:
                ax.set_title("断面の数 ``n_slices``".replace("``", "") if key == "n_slices" else "しきい値 ``fin_threshold``".replace("``", ""), fontsize=10, color=INK, loc="left")
    axs[0, 1].legend(loc="upper right", fontsize=8.5, ncol=2)
    axs[-1, 0].set_xlabel("断面の数 $N_s$")
    axs[-1, 1].set_xlabel("しきい値 $\\delta / r_{\\max}$")
    f.save(fig, "geometry_sensitivity")


def geometry_figures(f):
    geometry_cad_to_triangles(f)
    geometry_axis(f)
    geometry_slices(f)
    geometry_fins(f)
    geometry_compare(f)
    geometry_sensitivity(f)
# <<< geometry


def basic_finner_validation(f, vdir):
    """Basic Finner: model vs the free-flight data of DREV-TM-9703 (doc/data/validation/basic_finner)."""
    t = pd.read_csv(vdir / "summary_vs_measured.csv", comment="#")
    meas = pd.read_csv(vdir / "measured.csv", comment="#")
    regan = meas[(meas.table == "2 (converted)") & (meas.quantity == "Clp")]
    sty = [("barrowman", "部品積み上げ法", SERIES[0], "-", None), ("panel", "パネル法", SERIES[2], "--", None),
           ("euler", "CFD（Euler）", SERIES[1], "none", "o"), ("rans", "CFD（RANS）", SERIES[6], "none", "s")]
    panels = [("CNa", "$C_{N\\alpha}$ [1/rad]"), ("Cma", "$C_{m\\alpha}$（5.5 口径まわり）[1/rad]"),
              ("CX0", "$C_{X0}$"), ("Clp", "$C_{lp}$（$pd/(2V)$ あたり）")]
    fig, axs = plt.subplots(2, 2, figsize=(9.6, 7.0))
    for ax, (q, label) in zip(axs.flat, panels):
        d = t[t.q == q].sort_values("mach")
        pe = pd.to_numeric(d.probable_error_pct, errors="coerce").fillna(0.0)
        ax.errorbar(d.mach, d.measured, yerr=d.measured.abs() * pe / 100, fmt="D", color=INK, mfc=INK, ms=5,
                    capsize=3, lw=1.2, label="実測（DREV 射場、Table IX）", zorder=5)
        if q == "Clp":
            ax.errorbar(regan.mach, regan.value, yerr=regan.value.abs() * regan.probable_error_pct / 100, fmt="D",
                        color=INK2, mfc="white", ms=5, capsize=3, lw=1.0, label="実測（NOL 風洞、換算）", zorder=4)
        for key, name, col, ls, mk in sty:
            v = pd.to_numeric(d[key], errors="coerce")
            if v.notna().sum() == 0:
                continue
            if mk is None:
                ax.plot(d.mach, v, ls=ls, marker=".", ms=6, color=col, lw=1.6, label=name)
            else:
                ax.plot(d.mach, v, marker=mk, ls="none", color=col, ms=7, mfc=col if key == "euler" else "white",
                        mew=1.6, label=name)
        ax.set_xlabel("Mach 数")
        ax.set_ylabel(label)
    handles, labels = [], []
    for ax in axs.flat:
        for h, lab in zip(*ax.get_legend_handles_labels()):
            if lab not in labels:
                handles.append(h)
                labels.append(lab)
    fig.legend(handles, labels, loc="lower center", ncol=3, bbox_to_anchor=(0.5, -0.06), fontsize=9)
    fig.tight_layout()
    f.save(fig, "validation_basic_finner")


def aero_cost_accuracy(f, data):
    """Table time [s] against the C_Nalpha difference [%] to Euler CFD, per method (marker) and Mach number (colour)."""
    vm = pd.read_csv(data / "sample_euler_vs_models.csv", comment="#")
    z = vm[vm.alpha_deg == 0].sort_values("mach")
    cost = data.parent / "cost"
    st = cost_csv(cost, "stages.csv")
    qt = pd.read_csv(cost / "cfd_quoted.csv", comment="#").set_index("item")
    times = {  # whole table, sample rocket: 4 threads (doc/data/cost/stages.csv), CFD 4 ranks (doc/data/cfd/run_summary.json)
        "barrowman": float(st[(st.stage == "aero") & (st.variant == "barrowman")].wall_s.iloc[0]),
        "panel": float(st[(st.stage == "aero") & (st.variant == "panel")].wall_s.iloc[0]),
        "cfd": float(qt.loc["euler_matrix_16_cases", "wall_s"]),
    }
    fig, ax = plt.subplots(figsize=(6.0, 3.6))
    machs = [0.3, 0.8, 1.2, 2.0]
    for c, m in zip(SERIES, machs):
        r = z[np.isclose(z.mach, m)].iloc[0]
        for key, mk in (("barrowman", "o"), ("panel", "s"), ("cfd", "D")):
            d = 0.0 if key == "cfd" else 100.0 * (r[f"{key}_cna_per_rad"] / r.cfd_cna_per_rad - 1.0)
            ax.plot(times[key], d, mk, color=c, markersize=8, markeredgecolor="white", markeredgewidth=0.6)
    ax.axhline(0, color=INK2, linewidth=0.8)
    ax.set_xscale("log")
    ax.set_xlim(0.08, 3e4)
    ax.set_ylim(-34, 24)
    ax.set_xlabel("表全体の計算時間 [s]（対数）")
    ax.set_ylabel("CFD（Euler）に対する $C_{N\\alpha}$ の差 [%]")
    for key, lab in (("barrowman", "部品積み上げ法"), ("panel", "パネル法"), ("cfd", "CFD（Euler）")):
        ax.text(times[key], 21.5, lab, ha="center", color=INK2, fontsize=9)
    from matplotlib.lines import Line2D
    h = [Line2D([], [], marker="o", linestyle="", color=c, markeredgecolor="white", markersize=8, label=f"$M={m:g}$") for c, m in zip(SERIES, machs)]
    leg_m = ax.legend(handles=h, title="色：Mach 数", loc="upper center", ncol=4, bbox_to_anchor=(0.5, -0.17), columnspacing=1.2, handletextpad=0.3)
    ax.add_artist(leg_m)
    hs = [Line2D([], [], marker=mk, linestyle="", color=INK2, markeredgecolor="white", markersize=8, label=lab)
          for mk, lab in (("o", "部品積み上げ法"), ("s", "パネル法"), ("D", "CFD（Euler）"))]
    ax.legend(handles=hs, title="記号：手法", loc="upper center", ncol=3, bbox_to_anchor=(0.5, -0.30), columnspacing=1.2, handletextpad=0.3)
    fig.subplots_adjust(left=0.13, right=0.97, top=0.97, bottom=0.12)
    f.save(fig, "aero_cost_accuracy")


def panel_mesh_cost(f, data):
    """Panel count n against the change of C_Nalpha (change relative to the finest mesh)."""
    d = pd.read_csv(data / "panel" / "mesh_cost.csv", comment="#")
    fig, ax = plt.subplots(figsize=(4.6, 3.3))
    for c, col, lab in ((SERIES[0], "cna_m03", "$M=0.3$"), (SERIES[1], "cna_m08", "$M=0.8$")):
        ref = d[col].iloc[-1]
        ax.plot(d.panels, 100.0 * (d[col] / ref - 1.0), "-o", color=c, markersize=7, label=lab)
    ax.axhline(0, color=INK2, linewidth=0.8)
    ax.set_xlabel("表面パネル数 $n$")
    ax.set_ylabel("$C_{N\\alpha}$ の変化 [%]（最も細かいメッシュ基準）")
    ax.legend(loc="lower right")
    fig.tight_layout()
    f.save(fig, "panel_mesh_cost")


def cfd_fin_size_cost(f, data):
    """fin_size against cell count, time of one case and the spurious C_N at alpha = 0 (cad, half model)."""
    d = pd.read_csv(data / "fin_resolution.csv", comment="#")
    d = d[(d.surface == "cad") & np.isclose(d.mach, 0.8)].drop_duplicates("run")
    d = d.assign(wall=d.wall_size_mm)
    fig, axs = plt.subplots(1, 3, figsize=(8.4, 3.4))
    for c, w, lab in ((SERIES[0], 6, "wall_size 6 mm"), (SERIES[1], 2, "wall_size 2 mm")):
        s = d[d.wall == w].sort_values("fin_size_mm", ascending=True)
        axs[0].plot(s.fin_size_mm, s.cells / 1e6, "o", color=c, markersize=8, label=lab)
        axs[1].plot(s.fin_size_mm, (s.case_s_a0 + s.case_s_a4) / 60.0, "o", color=c, markersize=8, label=lab)
        axs[2].plot(s.fin_size_mm, s.cn0, "o", color=c, markersize=8, label=lab)
    axs[2].axhline(0, color=INK2, linewidth=0.8)
    for ax, lab in zip(axs, ("セル数 [$10^6$]", "計算時間 [min]\n($\\alpha=0$ と $4^\\circ$ の合計)", "$\\alpha=0$ の $C_N$\n（真値は 0）")):
        ax.set_xlabel("fin_size [mm]")
        ax.set_ylabel(lab)
        ax.set_xticks([1, 2, 6])
    axs[0].legend(loc="upper left")
    fig.tight_layout()
    f.save(fig, "cfd_fin_size_cost")


def integrator_cost_accuracy(f, data):
    """Right-hand-side evaluations against the landing-point difference [m] for rk4 / rk45 / dop853."""
    d = pd.read_csv(data / "sim" / "integrator_cost_accuracy.csv", comment="#")
    fig, ax = plt.subplots(figsize=(6.0, 3.6))
    floor = 1e-4
    for c, k, mk, lab in ((SERIES[0], "rk4", "o", "rk4（刻み幅を変える）"), (SERIES[1], "rk45", "s", "rk45（許容誤差を変える）"), (SERIES[2], "dop853", "^", "dop853（許容誤差を変える）")):
        s = d[d.integrator == k].sort_values("rhs_evals")
        ax.plot(s.rhs_evals, np.maximum(s.landing_diff_m, floor), "-" + mk, color=c, markersize=7, linewidth=1.2, label=lab)
    ax.axhline(floor, color=GRID, linewidth=0.8)
    ax.text(2.5e5, floor * 1.25, "$10^{-4}$ m 以下（参照解と一致）", color=INK2, fontsize=8.5)
    ax.set_xscale("log")
    ax.set_yscale("log")
    ax.set_xlabel("右辺の評価回数（1 回の飛翔）")
    ax.set_ylabel("参照解との着地点の差 [m]")
    ax.set_xlim(4e4, 4e6)
    ax.legend(loc="upper right")
    fig.tight_layout()
    f.save(fig, "integrator_cost_accuracy")


def mc_sample_convergence(f, data):
    """Maximum, P99.9 and P99 of the landing distance against the sample count N (ballistic and parachute)."""
    d = pd.read_csv(data / "dispersion" / "sample_convergence.csv", comment="#")
    fig, axs = plt.subplots(1, 2, figsize=(7.6, 3.3))
    for ax, k, ttl in ((axs[0], "ballistic", "弾道落下"), (axs[1], "parachute", "パラシュート降下")):
        for c, col, mk, lab in ((SERIES[3], "max", "o", "最大"), (SERIES[1], "p999", "s", "P99.9"), (SERIES[0], "p99", "^", "P99")):
            ax.plot(d.samples, d[f"{k}_{col}"] / 1e3, "-" + mk, color=c, markersize=7, linewidth=1.2, label=lab)
        ax.set_xscale("log")
        ax.set_xlabel("サンプル数 $N$")
        ax.set_ylabel("着地距離 [km]")
        ax.set_ylim(0, None)
        ax.text(0.04, 0.95, ttl, transform=ax.transAxes, va="top", color=INK2, fontsize=9.5)
    axs[1].legend(loc="lower right")
    fig.tight_layout()
    f.save(fig, "mc_sample_convergence")


# ---------------------------------------------------------------- testing chapter (doc/data/testing)

def test_counts(f, data):
    """Test functions per crate, split into the ones that run by default and the ignored ones."""
    d = pd.read_csv(data / "testing" / "test_counts.csv", comment="#").sort_values("default", ascending=True)
    fig, ax = plt.subplots(figsize=(6.4, 3.0))
    y = np.arange(len(d))
    ax.barh(y, d["default"], color=SERIES[0], label="既定で実行")
    ax.barh(y, d["ignored"], left=d["default"], color=SERIES[1], label="ignored（外部ツール・計時）")
    for yi, (a, b) in enumerate(zip(d["default"], d["ignored"])):
        ax.text(a + b + 3, yi, f"{a + b}", va="center", color=INK2, fontsize=9)
    ax.set_yticks(y, [f"{c}" for c in d["crate"]])
    ax.set_xlabel("テスト関数の数")
    ax.set_xlim(0, d[["default", "ignored"]].sum(axis=1).max() * 1.12)
    ax.grid(axis="y", visible=False)
    ax.legend(loc="lower right")
    fig.tight_layout()
    f.save(fig, "test_counts")


def test_margins(f, data):
    """Measured deviation divided by the bound of the assertion, for the tests that print their value."""
    d = pd.read_csv(data / "testing" / "test_margins.csv", comment="#")
    d["ratio"] = d["value"] / d["bound"]
    col = {"analytic": SERIES[0], "invariant": SERIES[2], "refine": SERIES[3], "cross": SERIES[1]}
    lab = {"analytic": "解析解・理論式", "invariant": "保存量", "refine": "メッシュの細分", "cross": "他モデルとの相互比較"}
    fig, ax = plt.subplots(figsize=(8.0, 4.4))
    y = np.arange(len(d))[::-1]
    for k in ("analytic", "invariant", "refine", "cross"):
        m = d["kind"] == k
        ax.plot(d.ratio[m], y[m.values], "o", color=col[k], markersize=7, label=lab[k])
    ax.axvline(1.0, color=INK, linewidth=1.0)
    ax.text(1.0, len(d) - 0.35, " 判定幅（これを超えると失敗）", color=INK, fontsize=8.5, ha="left", va="bottom")
    ax.set_xscale("log")
    ax.set_xlim(1e-7, 5)
    ax.set_ylim(-0.7, len(d) + 0.2)
    ax.set_yticks(y, d["quantity"], fontsize=7.5)
    ax.set_xlabel("測った偏差 ÷ テストの判定幅")
    ax.legend(loc="lower left", fontsize=8.5)
    fig.tight_layout()
    f.save(fig, "test_margins")


def test_order(f, data):
    """Attitude error of the Lie-group integrators at the steps h and h/2, with the slopes of orders 4 and 5."""
    o = pd.read_csv(data / "testing" / "test_order.csv", comment="#")
    fig, ax = plt.subplots(figsize=(4.6, 3.4))
    for i, r in o.iterrows():
        c = SERIES[i]
        hs = np.array([r.h, r.h / 2])
        es = np.array([r.err_h, r.err_h2])
        ax.loglog(hs, es, "o", color=c, markersize=7)
        p = np.round(r.order)
        xs = np.array([r.h / 2.2, r.h * 1.15])
        ax.loglog(xs, r.err_h * (xs / r.h) ** p, "--", color=c, linewidth=1.0)
        ax.text(r.h * 1.2, r.err_h, f"{r.method}\n観測 {r.order:.2f}\n許容 {r.lo}–{r.hi}", color=c, fontsize=8.5, va="center")
    ax.set_xlim(0.018, 0.2)
    ax.set_xticks([0.025, 0.05], ["0.025", "0.05"])
    ax.xaxis.set_minor_formatter(matplotlib.ticker.NullFormatter())
    ax.set_xlabel("刻み幅 $h$ [s]")
    ax.set_ylabel("20 s 後の姿勢の誤差 [rad]")
    ax.grid(which="minor", visible=False)
    fig.tight_layout()
    f.save(fig, "test_order")


# ---------------------------------------------------------------- cross-comparison with OpenRocket and RocketPy

XC_STYLE = {  # key: (label, colour, linestyle)
    "ign": ("IgnisYeet", SERIES[0], "-"),
    "or": ("OpenRocket", SERIES[1], "-"),
    "rp_or": ("RocketPy（OpenRocket の抗力）", SERIES[2], "--"),
    "rp_ign": ("RocketPy（IgnisYeet の抗力）", SERIES[3], "--"),
}


def xcheck_static(f, xd):
    """CNalpha, centre of pressure and zero-lift drag against Mach for IgnisYeet, OpenRocket and RocketPy."""
    from matplotlib.lines import Line2D
    from matplotlib.patches import Patch

    d = pd.read_csv(xd / "static_aero.csv")
    fig, axs = plt.subplots(1, 3, figsize=(9.6, 3.3))
    for ax, (q, lab) in zip(axs[:2], (("cna", "$C_{N\\alpha}$ [1/rad]"), ("xcp", "圧力中心 $x_{cp}$（先端から）[m]"))):
        ax.plot(d.mach, d[f"{q}_ign"], color=SERIES[0])
        ax.plot(d.mach, d[f"{q}_or"], color=SERIES[1])
        ax.plot(d.mach, d[f"{q}_rp"], color=SERIES[2], ls="--")
        ax.set_ylabel(lab)
    ax = axs[2]
    band = d[["cd_or", "cd_or_square", "cd_or_airfoil"]]
    ax.fill_between(d.mach, band.min(axis=1), band.max(axis=1), color=SERIES[1], alpha=0.18, lw=0)
    ax.plot(d.mach, d.cd_or, color=SERIES[1])
    ax.plot(d.mach, d.cd_off_ign, color=SERIES[0])
    ax.plot(d.mach, d.cd_on_ign, color=SERIES[0], ls=":")
    ax.set_ylabel("零揚力抗力係数 $C_D$")
    for ax in axs:
        ax.set_xlabel("Mach 数")
        ax.set_xlim(0, 3)
    handles = [Line2D([], [], color=SERIES[0]), Line2D([], [], color=SERIES[0], ls=":"), Line2D([], [], color=SERIES[1]),
               Patch(color=SERIES[1], alpha=0.18, lw=0), Line2D([], [], color=SERIES[2], ls="--")]
    labels = ["IgnisYeet（抗力は燃焼後）", "IgnisYeet（抗力は燃焼中）", "OpenRocket（フィン断面 rounded）", "OpenRocket（フィン断面 3 種の幅）", "RocketPy"]
    fig.legend(handles, labels, loc="lower center", ncol=3, bbox_to_anchor=(0.5, -0.12), fontsize=8.5)
    fig.tight_layout()
    f.save(fig, "xcheck_static")


def xcheck_flight(f, xd):
    """Altitude and speed against time (no wind), and the differences between pairs of programs."""
    d = pd.read_csv(xd / "flight_nowind.csv")
    fig, axs = plt.subplots(3, 2, figsize=(9.0, 7.4), sharex=True, gridspec_kw={"height_ratios": [2.2, 1.2, 1.2]})
    for j, (q, lab, unit) in enumerate((("alt", "高度 [m]", "m"), ("v", "速さ [m/s]", "m/s"))):
        ax = axs[0, j]
        for k, (name, col, ls) in XC_STYLE.items():
            ax.plot(d.t, d[f"{q}_{k}"], color=col, ls=ls, lw=1.8 if k in ("ign", "or") else 1.4, label=name)
        ax.set_ylabel(lab)
        ax = axs[1, j]
        ax.axhline(0, color=INK2, lw=0.8)
        ax.plot(d.t, d[f"{q}_or"] - d[f"{q}_ign"], color=SERIES[1], label="OpenRocket − IgnisYeet")
        ax.set_ylabel(f"差 [{unit}]")
        ax = axs[2, j]
        ax.axhline(0, color=INK2, lw=0.8)
        ax.plot(d.t, d[f"{q}_rp_ign"] - d[f"{q}_ign"], color=SERIES[3], ls="--", label="RocketPy（IgnisYeet の抗力）− IgnisYeet")
        ax.plot(d.t, d[f"{q}_rp_or"] - d[f"{q}_or"], color=SERIES[2], ls="--", label="RocketPy（OpenRocket の抗力）− OpenRocket")
        ax.set_ylabel(f"差 [{unit}]")
        ax.set_xlabel("時刻 [s]")
        ax.set_xlim(0, 28)
    h, lab = axs[0, 0].get_legend_handles_labels()
    h1, lab1 = axs[1, 0].get_legend_handles_labels()
    h2, lab2 = axs[2, 0].get_legend_handles_labels()
    fig.legend(h + h1 + h2, lab + lab1 + lab2, loc="lower center", ncol=2, bbox_to_anchor=(0.5, -0.115), fontsize=8.5)
    fig.tight_layout()
    f.save(fig, "xcheck_flight")


def xcheck_diff(f, xd):
    """Relative differences of the ascent metrics between pairs of programs (no wind and 4 m/s wind)."""
    s = pd.read_csv(xd / "summary.csv").pivot_table(index=["case", "metric"], columns="tool", values="value")
    metrics = [("apogee_m", "頂点高度"), ("apogee_time_s", "頂点時刻"), ("max_velocity_ms", "最大速さ"), ("burnout_altitude_m", "燃焼終了高度"),
               ("burnout_velocity_ms", "燃焼終了速さ"), ("rail_exit_speed_ms", "ランチャ離脱速さ"), ("apogee_horizontal_m", "頂点の水平距離")]
    pairs = [("or", "ign", "OpenRocket / IgnisYeet", SERIES[1], "o"), ("rp_ign", "ign", "RocketPy（IgnisYeet の抗力）/ IgnisYeet", SERIES[3], "s"),
             ("rp_or", "or", "RocketPy（OpenRocket の抗力）/ OpenRocket", SERIES[2], "^")]
    fig, axs = plt.subplots(1, 2, figsize=(9.6, 3.9), sharey=True)
    y = np.arange(len(metrics))[::-1]
    for ax, case, ttl in zip(axs, ("nowind", "wind"), ("無風", "風 4 m/s")):
        ax.axvline(0, color=INK2, lw=0.8)
        for a, b, lab, col, mk in pairs:
            v = [100 * (s.loc[(case, m), a] / s.loc[(case, m), b] - 1) for m, _ in metrics]
            ax.plot(v, y, mk, color=col, ms=7, ls="none", label=lab)
        ax.set_xscale("symlog", linthresh=0.1, linscale=0.6)
        ax.set_xlabel("相対差 [%]（左の分子 / 右の分母 − 1）")
        ax.set_title(ttl, color=INK2, fontsize=10)
    axs[0].set_yticks(y)
    axs[0].set_yticklabels([n for _, n in metrics])
    h, lab = axs[0].get_legend_handles_labels()
    fig.legend(h, lab, loc="lower center", ncol=3, bbox_to_anchor=(0.5, -0.06), fontsize=8.5)
    fig.tight_layout()
    f.save(fig, "xcheck_diff")


def xcheck_alpha(f, xd):
    """Angle of attack after the rail exit with a 4 m/s side wind."""
    d = pd.read_csv(xd / "alpha_wind.csv")
    d = d[(d.t >= 0.25) & (d.t <= 1.6)]
    fig, ax = plt.subplots(figsize=(6.4, 3.3))
    ax.plot(d.t, d.aoa_ign, color=SERIES[0], lw=2.0, label="IgnisYeet")
    ax.plot(d.t, d.aoa_or, color=SERIES[1], lw=2.0, label="OpenRocket")
    ax.plot(d.t, d.aoa_rp_ign, color=SERIES[3], ls="--", label="RocketPy")
    ax.set_xlabel("時刻 [s]")
    ax.set_ylabel("迎角 [°]")
    ax.legend()
    fig.tight_layout()
    f.save(fig, "xcheck_alpha")


def xcheck_basic_finner(f, xd):
    """Basic Finner: CNalpha, centre of pressure and CX0 of the measured data and of three programs."""
    d = pd.read_csv(xd / "basic_finner.csv")
    fig, axs = plt.subplots(1, 3, figsize=(9.6, 3.3))
    for ax, (q, lab) in zip(axs, (("cna", "$C_{N\\alpha}$ [1/rad]"), ("xcp", "圧力中心（先端から）[口径]"), ("cx0", "$C_{X0}$"))):
        ax.plot(d.mach, d[f"{q}_measured"], "D", color=INK, ms=5, label="実測（DREV 射場）", zorder=5)
        ax.plot(d.mach, d[f"{q}_ign"], "-o", color=SERIES[0], ms=4, label="IgnisYeet")
        ax.plot(d.mach, d[f"{q}_or"], "-s", color=SERIES[1], ms=4, label="OpenRocket")
        if q != "cx0":
            ax.plot(d.mach, d[f"{q}_rp"], "--^", color=SERIES[2], ms=4, label="RocketPy")
        ax.set_xlabel("Mach 数")
        ax.set_ylabel(lab)
    h, lab = axs[1].get_legend_handles_labels()
    fig.legend(h, lab, loc="lower center", ncol=4, bbox_to_anchor=(0.5, -0.06), fontsize=8.5)
    fig.tight_layout()
    f.save(fig, "xcheck_basic_finner")


def xcheck_figures(f, xd):
    xcheck_static(f, xd)
    xcheck_flight(f, xd)
    xcheck_diff(f, xd)
    xcheck_alpha(f, xd)
    xcheck_basic_finner(f, xd)


# ---------------------------------------------------------------- cost chapter (doc/data/cost)

def cost_csv(d, name):
    """A data file of doc/data/cost; the run on a quiet machine (<name>_quiet.csv) is used when it exists."""
    q = d / name.replace(".csv", "_quiet.csv")
    return pd.read_csv(q if q.exists() else d / name, comment="#")


def cost_stages(f, d, cfd):
    """Run time of every stage on the sample rocket (log axis): measured here (4 threads) and quoted CFD records."""
    st = cost_csv(d, "stages.csv")
    q = pd.read_csv(d / "cfd_quoted.csv", comment="#").set_index("item")

    def t(stage, variant):
        return float(st[(st.stage == stage) & (st.variant == variant)].wall_s.iloc[0])

    rows = [
        ("形状の抽出（geom）", t("geom", "-"), "m"),
        ("係数表：部品積み上げ法", t("aero", "barrowman"), "m"),
        ("係数表：パネル法", t("aero", "panel"), "m"),
        ("係数表：CFD（Euler、16 ケース）", float(q.loc["euler_matrix_16_cases", "wall_s"]), "q"),
        ("係数表：CFD（RANS、20 ケース）", float(q.loc["rans_table_20_cases", "wall_s"]), "q"),
        ("飛翔 1 回（rk4）", t("sim", "rk4"), "m"),
        ("飛翔 1 回（rk45）", t("sim", "rk45"), "m"),
        ("落下分散：風の格子 112 飛翔（rk4）", t("dispersion", "wind_grid 112 flights, rk4"), "m"),
        ("落下分散：風の格子 112 飛翔（rk45）", t("dispersion", "wind_grid 112 flights, rk45"), "m"),
        ("落下分散：モンテカルロ 1000×2 飛翔（rk45）", t("dispersion", "monte_carlo 1000 x 2, rk45"), "m"),
    ]
    fig, ax = plt.subplots(figsize=(7.2, 3.9))
    y = np.arange(len(rows))[::-1]
    for yi, (lab, v, kind) in zip(y, rows):
        ax.barh(yi, v, left=1e-3, height=0.62, color=SERIES[0] if kind == "m" else SERIES[1], edgecolor="white")
        txt = f"{v:.2f} s" if v < 10 else (f"{v:.0f} s" if v < 100 else f"{v:.0f} s（{v / 3600:.1f} 時間）" if v >= 3600 else f"{v:.0f} s")
        ax.text(v * 1.25, yi, txt, va="center", fontsize=8.5, color=INK2)
    ax.set_yticks(y)
    ax.set_yticklabels([r[0] for r in rows], fontsize=8.5)
    ax.set_xscale("log")
    ax.set_xlim(1e-2, 3e5)
    ax.set_xticks([1e-2, 1e-1, 1, 10, 60, 600, 3600, 36000])
    ax.set_xticklabels(["0.01 s", "0.1 s", "1 s", "10 s", "1 分", "10 分", "1 時間", "10 時間"], fontsize=8.5)
    ax.xaxis.set_minor_locator(matplotlib.ticker.NullLocator())
    ax.grid(axis="y", visible=False)
    from matplotlib.patches import Patch
    ax.legend(handles=[Patch(color=SERIES[0], label="この章で測定（4 スレッド）"), Patch(color=SERIES[1], label="保存した計測の引用（SU2、4 ランク）")], loc="upper right")
    fig.tight_layout()
    f.save(fig, "cost_stages")


def cost_threads(f, d):
    """Speed-up against the thread count: the quoted profile of 2026-10-08, this chapter's quiet-machine run and its busy-machine run."""
    series = (
        ("panel_table", "panel_table", "パネル法の係数表", SERIES[0], "o"),
        ("wind_grid_rk4_112", "wind_grid_rk4_224", "風の格子（rk4）", SERIES[1], "s"),
        ("monte_carlo_rk45_2000", "monte_carlo_rk45_4000", "モンテカルロ（rk45）", SERIES[2], "^"),
    )
    sets = [("引用", pd.read_csv(d / "threads_quoted.csv", comment="#"), 0)]
    if (d / "threads_quiet.csv").exists():
        q = pd.read_csv(d / "threads_quiet.csv", comment="#")
        sets.append((f"この章、負荷小（負荷平均 {q.load.min():.0f}〜{q.load.max():.0f}）", q, 1))
    b = pd.read_csv(d / "threads.csv", comment="#")
    sets.append((f"この章、負荷大（負荷平均 {b.load.min():.0f}〜{b.load.max():.0f}）", b, 1))
    sets[0] = (f"引用（負荷平均 12〜15）", sets[0][1], 0)
    fig, axs = plt.subplots(1, len(sets), figsize=(2.6 * len(sets) + 0.6, 3.5), sharey=True)
    tt = np.array([1, 2, 4, 8, 20])
    for ax, (ttl, df, kind) in zip(axs, sets):
        ax.plot(tt, tt, color=INK2, linewidth=0.9, linestyle="--", label="理想")
        ax.set_xscale("log", base=2)
        ax.set_yscale("log", base=2)
        ax.set_xticks(tt)
        ax.set_xticklabels([str(v) for v in tt])
        ax.set_yticks([1, 2, 4, 8, 16])
        ax.set_yticklabels(["1", "2", "4", "8", "16"])
        ax.set_xlabel("resources.threads")
        ax.text(0.04, 0.95, ttl, transform=ax.transAxes, va="top", color=INK2, fontsize=8)
        for qk, tk, lab, c, mk in series:
            s = df[df.task == (qk if kind == 0 else tk)].sort_values("threads")
            ax.plot(s.threads, s.wall_s.iloc[0] / s.wall_s, "-" + mk, color=c, markersize=6, linewidth=1.2, label=lab)
    axs[0].set_ylabel("速くなる倍率（1 スレッド比）")
    axs[0].legend(loc="lower right", fontsize=7.5)
    fig.tight_layout()
    f.save(fig, "cost_threads")


def cost_scaling(f, d, cfd):
    """Scaling laws: panel count (time, memory), number of flights, CFD cells (time per iteration)."""
    pm = cost_csv(d, "panel_mesh.csv").sort_values("panels")
    ds = cost_csv(d, "dispersion_scaling.csv")
    mc = pd.read_csv(cfd / "mesh_convergence.csv", comment="#")
    fig, axs = plt.subplots(2, 2, figsize=(7.6, 6.0))
    ax = axs[0, 0]
    ax.plot(pm.panels, pm.cpu_s, "o-", color=SERIES[0], markersize=6, linewidth=1.2, label="CPU 時間")
    ax.plot(pm.panels, pm.wall_s, "s", color=SERIES[1], markersize=6, label="壁時計の時間")
    n = np.array([pm.panels.min(), pm.panels.max()], dtype=float)
    n0 = float(pm.panels.iloc[len(pm) // 2])
    c0 = float(pm.cpu_s.iloc[len(pm) // 2])
    ax.plot(n, c0 * (n / n0), color=INK2, linewidth=0.8, linestyle=":", label="$n$")
    ax.plot(n, c0 * (n / n0) ** 2, color=INK2, linewidth=0.8, linestyle="-.", label="$n^2$")
    ax.plot(n, c0 * (n / n0) ** 3, color=INK2, linewidth=0.8, linestyle="--", label="$n^3$")
    ax.set_xscale("log")
    ax.set_yscale("log")
    ax.set_xlabel("表面パネル数 $n$")
    ax.set_ylabel("時間 [s]（4 スレッド）")
    ax.set_xticks([2000, 4000, 8000])
    ax.set_xticklabels(["2000", "4000", "8000"])
    ax.xaxis.set_minor_locator(matplotlib.ticker.NullLocator())
    ax.legend(loc="upper left", fontsize=7.5)
    ax = axs[0, 1]
    nn = np.linspace(pm.panels.min() * 0.9, pm.panels.max() * 1.05, 100)
    ax.plot(nn, (8 * nn**2 + 64 * nn) / 1e6, color=SERIES[0], linewidth=1.2, label="$8n^2+64n$")
    ax.plot(pm.panels, pm.rss_mb, "o", color=SERIES[1], markersize=7, label="プロセスの最大常駐メモリ")
    ax.set_xlabel("表面パネル数 $n$")
    ax.set_ylabel("メモリ [MB]")
    ax.legend(loc="upper left", fontsize=8)
    ax = axs[1, 0]
    for key, lab, c, mk in (("wind_grid", "風の格子", SERIES[0], "o"), ("monte_carlo", "モンテカルロ", SERIES[1], "s")):
        s = ds[ds["mode"] == key]
        ax.plot(s.flights, s.wall_s, "-" + mk, color=c, markersize=6, linewidth=1.2, label=lab)
    ax.set_xscale("log")
    ax.set_yscale("log")
    ax.set_xlabel("飛翔の数")
    ax.set_ylabel("時間 [s]（rk45、4 スレッド）")
    ax.legend(loc="upper left")
    ax = axs[1, 1]
    for m, c, mk in ((0.8, SERIES[0], "o"), (2.0, SERIES[1], "s")):
        s = mc[np.isclose(mc.mach, m)].copy()
        it0 = s.iterations.str.split("/").str[0].astype(float)
        it4 = s.iterations.str.split("/").str[1].astype(float)
        ax.plot(s.cells / 1e6, s.case_wall_s_a0 / it0, "-" + mk, color=c, markersize=6, linewidth=1.2, label=f"$M={m:g}$、$\\alpha=0$")
        ax.plot(s.cells / 1e6, s.case_wall_s_a4 / it4, "--" + mk, color=c, markersize=6, linewidth=1.0, mfc="white", label=f"$M={m:g}$、$\\alpha=4^\\circ$")
    ax.set_xlabel("セル数 [$10^6$]")
    ax.set_ylabel("1 反復の時間 [s]（Euler、4 ランク）")
    ax.set_ylim(0, None)
    ax.legend(loc="upper left", fontsize=7.5)
    for ax, lab in zip(axs.flat, "abcd"):
        ax.text(0.97, 0.05, lab, transform=ax.transAxes, ha="right", color=INK2, fontsize=10)
    fig.tight_layout()
    f.save(fig, "cost_scaling")


def main():
    if sys.argv[1] == "--geometry":
        geometry_figures(Figs(Path(sys.argv[2])))
        return
    out, figdir = Path(sys.argv[1]), Path(sys.argv[2])
    f = Figs(figdir)
    theory(f)
    rigid_body(f)
    from_outputs(f, out)
    if len(sys.argv) > 3:
        monte_carlo(f, Path(sys.argv[3]))
    if len(sys.argv) > 4 and (Path(sys.argv[4]) / "aero_table.csv").exists():
        panel_method(f, out, Path(sys.argv[4]))
    data = Path(__file__).resolve().parent.parent / "data" / "cfd"
    if (data / "sample_euler_vs_models.csv").exists():
        pn = Path(sys.argv[4]) if len(sys.argv) > 4 else None
        cfd_study(f, out, pn, data)
    if (data / "stl_cross_check.csv").exists():
        cfd_stl_check(f, data)
    if any(data.glob("rans_*.csv")):
        rans_study(f, data)
    if (data / "fields" / "mesh_symmetry.npz").exists():
        cfd_fields(f, data / "fields")
    if (data / "fields_rans" / "rans_profile.npz").exists():
        cfd_euler_rans(f, data / "fields_rans")
    vdir = data.parent / "validation" / "basic_finner"
    if (vdir / "summary_vs_measured.csv").exists():
        basic_finner_validation(f, vdir)
    if (data.parent / "geometry" / "provenance.json").exists():
        geometry_figures(f)
    # Figures of the decision chapters (data committed under doc/data).
    root = data.parent
    aero_cost_accuracy(f, data)
    if (root / "panel" / "mesh_cost.csv").exists():
        panel_mesh_cost(f, root)
    cfd_fin_size_cost(f, data)
    integrator_cost_accuracy(f, root)
    mc_sample_convergence(f, root)
    if (root / "testing" / "test_counts.csv").exists():
        test_counts(f, root)
        test_margins(f, root)
        test_order(f, root)
    if (root / "xcheck_tools" / "summary.csv").exists():
        xcheck_figures(f, root / "xcheck_tools")
    if (root / "cost" / "stages.csv").exists():
        cost_stages(f, root / "cost", data)
        cost_threads(f, root / "cost")
        cost_scaling(f, root / "cost", data)

if __name__ == "__main__":
    main()
