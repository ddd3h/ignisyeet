# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2025-2026 西濱大将 (NISHIHAMA Daisuke)
"""Static coefficients of the Basic Finner in OpenRocket and RocketPy, next to IgnisYeet and the measured values.

Usage: OPENROCKET_JAR=OpenRocket-24.12.jar python run_basic_finner.py
Reads the measured values and the IgnisYeet (Barrowman) values from doc/data/validation/basic_finner/
and writes doc/data/xcheck_tools/basic_finner.csv: CNa [1/rad], xcp [calibres from the nose tip] and
CX0 (the drag coefficient at zero angle of attack, power off) at the Mach numbers of the range tests.
Geometry: DREV-TM-9703 as in examples/basic_finner/basic_finner.toml (10 cal body, 10 deg half-angle cone
of 2.84 cal, four rectangular fins of 1 cal chord and 1 cal span, 1.3 mm thick, roughness 2 um).
"""

import math
import sys
from pathlib import Path

import numpy as np
import pandas as pd

sys.path.insert(0, str(Path(__file__).resolve().parent))
import build_openrocket_model as bom  # noqa: E402
import common  # noqa: E402

D = 0.030
GEOM = {"length": 10 * D, "radius": D / 2, "nose_length": 2.84 * D, "fin_count": 4, "fin_root": D, "fin_tip": D, "fin_span": D,
        "fin_sweep": 0.0, "fin_thickness": 1.3e-3, "fin_x_le": 10 * D - D}
ROUGHNESS = 2e-6
VAL = common.ROOT / "doc" / "data" / "validation" / "basic_finner"


def openrocket(machs, fins):
    bom.or_session.start()
    from info.openrocket.core.aerodynamics import BarrowmanCalculator, FlightConditions
    from info.openrocket.core.logging import WarningSet
    from info.openrocket.core.models.atmosphere import ExtendedISAModel

    doc, rocket, body = bom.airframe(GEOM, ROUGHNESS, fins, nose="conical")
    config = rocket.getSelectedConfiguration()
    calc = BarrowmanCalculator()
    atm = ExtendedISAModel().getConditions(0.0)
    out = []
    for m in machs:
        row = {}
        for aoa in (0.0, math.radians(1.0)):
            fc = FlightConditions(config)
            fc.setAtmosphericConditions(atm)
            fc.setMach(float(m))
            fc.setAOA(aoa)
            f = calc.getAerodynamicForces(config, fc, WarningSet())
            if aoa == 0.0:
                row["cx0"] = float(f.getCD())
            else:
                row["cna"] = float(f.getCN()) / aoa
                row["xcp"] = float(f.getCP().x) / D
        out.append(row)
    return pd.DataFrame(out)


def rocketpy(machs):
    from rocketpy import Rocket

    rocket = Rocket(radius=D / 2, mass=1.58, inertia=(9.785e-3, 9.785e-3, 1.92e-4), power_off_drag=[[0, 0.5], [5, 0.5]], power_on_drag=[[0, 0.5], [5, 0.5]],
                    center_of_mass_without_motor=0.165, coordinate_system_orientation="nose_to_tail")
    rocket.add_nose(length=GEOM["nose_length"], kind="conical", position=0.0)
    rocket.add_trapezoidal_fins(n=4, root_chord=D, tip_chord=D, span=D, position=GEOM["fin_x_le"], sweep_length=0.0)
    return pd.DataFrame({"cna": [float(rocket.total_lift_coeff_der(m)) for m in machs], "xcp": [float(rocket.cp_position(m)) / D for m in machs]})


def main():
    s = pd.read_csv(VAL / "summary_vs_measured.csv", comment="#")
    machs = sorted(s[s.q == "CNa"].mach)
    res = pd.DataFrame({"mach": machs})
    for q, name in (("CNa", "cna"), ("xcp", "xcp"), ("CX0", "cx0")):
        t = s[s.q == q].set_index("mach")
        res[f"{name}_measured"] = [t.loc[m, "measured"] for m in machs]
        res[f"{name}_ign"] = [t.loc[m, "barrowman"] for m in machs]
    rp = rocketpy(machs)
    res["cna_rp"], res["xcp_rp"] = rp.cna, rp.xcp
    for fins in ("rounded", "square", "airfoil"):
        o = openrocket(machs, fins)
        suffix = "or" if fins == "rounded" else f"or_{fins}"
        res[f"cna_{suffix}"], res[f"xcp_{suffix}"], res[f"cx0_{suffix}"] = o.cna, o.xcp, o.cx0
    res.round(4).to_csv(common.DATA / "basic_finner.csv", index=False)
    print(res.round(3).to_string())


if __name__ == "__main__":
    main()
