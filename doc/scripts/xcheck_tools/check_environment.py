# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2024-2026 西濱大将 (NISHIHAMA Daisuke)
"""Compare the air density and gravity that the three programs used in the no-wind flight.

Usage: python check_environment.py <work dir>
Writes doc/data/xcheck_tools/environment.csv: for heights above sea level, the density of the
standard atmosphere in IgnisYeet (from the dynamic pressure and airspeed of the flight), in OpenRocket
(its air-density output) and in RocketPy (Environment.density, standard and custom atmosphere), and
the gravitational acceleration of OpenRocket and RocketPy.
"""

import sys
from pathlib import Path

import numpy as np
import pandas as pd

sys.path.insert(0, str(Path(__file__).resolve().parent))
import common  # noqa: E402

work = Path(sys.argv[1])
cfg = common.load_cfg()
el = cfg["launch"]["altitude"]
tr = pd.read_csv(work / "ign" / "nowind" / "trajectory.csv")
tr = tr[tr.airspeed > 20]
tr = tr.iloc[: int(tr.up.idxmax()) + 1]
rho_ign = 2 * tr.dyn_pressure / tr.airspeed**2
orf = pd.read_csv(work / "or" / "or_flight_nowind_rounded.csv")
orf = orf[orf.t > 0.5]

from rocketpy import Environment  # noqa: E402

la = cfg["launch"]
envs = {}
for name, kw in (("std", dict(type="standard_atmosphere")), ("custom", dict(type="custom_atmosphere", wind_u=4.0, wind_v=0.0))):
    e = Environment(latitude=la["latitude"], longitude=la["longitude"], elevation=el)
    e.set_atmospheric_model(**kw)
    envs[name] = e

rows = []
for z in (500.0, 1000.0, 2000.0, 3000.0, 4000.0):  # height above the launch site
    zs = z + el
    rows.append({
        "height_above_launch_m": z,
        "rho_ign": float(np.interp(z, tr.up, rho_ign)),
        "rho_or": float(np.interp(z, orf.alt, orf.rho)),
        "rho_rp_standard": float(envs["std"].density(zs)),
        "rho_rp_custom": float(envs["custom"].density(zs)),
        "g_or": float(np.interp(z, orf.alt, orf.g)),
        "g_rp": float(envs["std"].gravity(zs)),
    })
out = pd.DataFrame(rows)
out.round(6).to_csv(common.DATA / "environment.csv", index=False)
print(out.round(5).to_string())
