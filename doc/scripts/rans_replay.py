#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
"""Replay of the RANS acceptance rule of crates/cfd (runner::rans_verdict, History::rans_coefficients_stable and
History::rans_trend_settled; pitch-plane cases, no sideslip or roll) on a final-stage history.csv.

  old rule: density residual 3 and SA residual 2 orders below the reference peak and the coefficients flat over the
            last `window` rows (half-window drift and inter-quartile range of CFx, CFz <= 0.5 % of the force scale
            + 1e-5; centre of pressure -CMy/CFz within 0.002 body lengths, or CMy within tol * L/d near CN = 0)
  new rule: the old rule and the trend-extrapolated change |LSQ slope| * tau within the same tolerances,
            tau = 1 / (r ln 10), r = slower decay rate of the density and SA residuals over the window, at most the
            iteration budget (release/v0.4.0, 73d22f8)

As a module: `stops(head, rows, ref_rho, ref_nu, length_ratio, ...)` returns the first row count at which each rule
accepts (None if never), checked from `min_iter` rows on.
"""
import math

import numpy as np

FORCE_DRIFT = 0.005
XCP_DRIFT = 0.002
RHO_DROP, NU_DROP = 3.0, 2.0


def _stats(v):
    v = np.asarray(v)
    h = len(v) // 2
    s = np.sort(v)
    return v.mean(), abs(v[h:].mean() - v[:h].mean()), s[3 * len(s) // 4] - s[len(s) // 4]


def _slope(v):
    v = np.asarray(v)
    n = len(v)
    x = np.arange(n) - (n - 1) / 2.0
    return float((x * (v - v.mean())).sum() / (x * x).sum())


def flat(fx, fz, my, length_ratio):
    sx, sz = _stats(fx), _stats(fz)
    scale = max(abs(sx[0]), abs(sz[0]))
    tol = FORCE_DRIFT * scale + 1e-5
    ok = lambda s, t: s[1] <= t and s[2] <= t
    good = ok(sx, tol) and ok(sz, tol)
    if abs(sz[0]) > 20 * tol:
        good = good and ok(_stats(-np.asarray(my) / np.asarray(fz)), XCP_DRIFT * length_ratio)
    else:
        good = good and ok(_stats(my), tol * length_ratio)
    return good


def trend(fx, fz, my, rho, nu, length_ratio, budget):
    rate = min(-_slope(rho), -_slope(nu))
    tau = min(1.0 / (rate * math.log(10)), budget) if rate > 0 else budget
    scale = max(abs(np.mean(fx)), abs(np.mean(fz)))
    tol = FORCE_DRIFT * scale + 1e-5
    ok = abs(_slope(fx)) * tau <= tol and abs(_slope(fz)) * tau <= tol
    if abs(np.mean(fz)) > 20 * tol:
        ok = ok and abs(_slope(-np.asarray(my) / np.asarray(fz))) * tau <= XCP_DRIFT * length_ratio
    else:
        ok = ok and abs(_slope(my)) * tau <= tol * length_ratio
    return ok


def stops(head, rows, ref_rho, ref_nu, length_ratio, window=200, min_iter=400, budget=4000):
    col = lambda *names: next(head.index(n) for n in names if n in head)
    a = np.asarray(rows)
    rho, nu = a[:, col("rms[Rho]")], a[:, col("rms[nu]")]
    fx, fz, my = a[:, col("CFx", "CD")], a[:, col("CFz", "CL")], a[:, col("CMy")]
    pk_rho, pk_nu = np.maximum.accumulate(rho), np.maximum.accumulate(nu)
    old = new = None
    for n in range(max(min_iter, window), len(a) + 1):
        i = n - 1
        if max(ref_rho, pk_rho[i]) - rho[i] < RHO_DROP or max(ref_nu, pk_nu[i]) - nu[i] < NU_DROP:
            continue
        s = slice(n - window, n)
        if not flat(fx[s], fz[s], my[s], length_ratio):
            continue
        if old is None:
            old = n
        if trend(fx[s], fz[s], my[s], rho[s], nu[s], length_ratio, budget):
            new = n
            break
    return old, new
