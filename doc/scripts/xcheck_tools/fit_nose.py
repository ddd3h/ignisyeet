# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2024-2026 西濱大将 (NISHIHAMA Daisuke)
"""Fit nose-cone shapes to the extracted profile of the sample rocket.

Usage: python fit_nose.py <profile.csv of `ignisyeet geom`>
Writes doc/data/xcheck_tools/nose_fit.json: for each shape the nose length (0.26 to 0.34 m, step 0.5 mm)
with the smallest rms radial deviation over the points x < 0.25 m, and that deviation.
The tangent ogive with the length NOSE_LENGTH of common.py is the shape used in OpenRocket and RocketPy.
"""

import json
import sys
from pathlib import Path

import numpy as np
import pandas as pd

sys.path.insert(0, str(Path(__file__).resolve().parent))
import common  # noqa: E402

R = 0.05
p = pd.read_csv(sys.argv[1])
p = p[p.x < 0.25]
x, r = p.x.to_numpy(), p.r.to_numpy()


def shapes(L):
    xi = x / L
    th = np.arccos(1 - 2 * np.clip(xi, 0, 1))
    haack = lambda c: R / np.sqrt(np.pi) * np.sqrt(th - np.sin(2 * th) / 2 + c * np.sin(th) ** 3)  # noqa: E731
    rho = (R**2 + L**2) / (2 * R)
    return {
        "tangent ogive": np.sqrt(rho**2 - (L - x) ** 2) + R - rho,
        "parabola": R * (2 * xi - xi**2),
        "von Karman": haack(0.0),
        "LV-Haack": haack(1 / 3),
        "ellipsoid": R * np.sqrt(1 - (1 - xi) ** 2),
        "cone": R * xi,
    }


def rms(L, name):
    return float(np.sqrt(np.mean((shapes(L)[name] - r) ** 2)) * 1e3)


out = {}
for name in shapes(0.3):
    Ls = np.arange(0.26, 0.3401, 0.0005)
    e = [rms(L, name) for L in Ls]
    k = int(np.argmin(e))
    out[name] = {"best_length_m": round(float(Ls[k]), 4), "rms_mm": round(e[k], 4)}
out["tangent ogive at NOSE_LENGTH"] = {"length_m": common.NOSE_LENGTH, "rms_mm": round(rms(common.NOSE_LENGTH, "tangent ogive"), 4)}
common.DATA.mkdir(parents=True, exist_ok=True)
(common.DATA / "nose_fit.json").write_text(json.dumps(out, indent=2) + "\n")
print(json.dumps(out, indent=2))
