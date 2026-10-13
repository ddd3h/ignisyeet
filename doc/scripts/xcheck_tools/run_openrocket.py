# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2025-2026 西濱大将 (NISHIHAMA Daisuke)
"""Run OpenRocket on the sample rocket: static aerodynamics and the ascent flights.

Usage: OPENROCKET_JAR=OpenRocket-24.12.jar python run_openrocket.py <geometry.json> <out_dir> [--ork file.ork]

Writes to <out_dir>:
  or_static.csv        mach, cd_total, cd_friction, cd_pressure, cd_base, cna (1/rad), xcp (m from the nose tip)
                       at alpha = 0 (cd) and alpha = 1 deg (cna, xcp)
  or_flight_<case>_<fins>.csv  time series up to apogee (case: nowind, wind)
  or_components_<fins>.csv     nose, body tube and fins: CNa, CP and drag contributions at five Mach numbers
  or_summary_<fins>.json       the scalar results of each case
"""

import argparse
import csv
import json
import math
import sys
from pathlib import Path

import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parent))
import build_openrocket_model as bom  # noqa: E402
import common  # noqa: E402

COLS = {
    "t": "TYPE_TIME", "alt": "TYPE_ALTITUDE", "v": "TYPE_VELOCITY_TOTAL", "vz": "TYPE_VELOCITY_Z", "mach": "TYPE_MACH_NUMBER",
    "acc": "TYPE_ACCELERATION_TOTAL", "az": "TYPE_ACCELERATION_Z", "cd": "TYPE_DRAG_COEFF", "cd_ax": "TYPE_AXIAL_DRAG_COEFF",
    "xcp": "TYPE_CP_LOCATION", "xcg": "TYPE_CG_LOCATION", "stab": "TYPE_STABILITY", "aoa": "TYPE_AOA", "px": "TYPE_POSITION_X",
    "py": "TYPE_POSITION_Y", "thrust": "TYPE_THRUST_FORCE", "mass": "TYPE_MASS", "rho": "TYPE_AIR_DENSITY", "g": "TYPE_GRAVITY",
    "re": "TYPE_REYNOLDS_NUMBER", "cn": "TYPE_NORMAL_FORCE_COEFF",
}


def static_aero(rocket, fcid, cfg, mach_grid):
    from info.openrocket.core.aerodynamics import BarrowmanCalculator, FlightConditions
    from info.openrocket.core.models.atmosphere import ExtendedISAModel
    from info.openrocket.core.util import Coordinate  # noqa: F401
    from info.openrocket.core.logging import WarningSet

    config = rocket.getFlightConfiguration(fcid)
    calc = BarrowmanCalculator()
    atm = ExtendedISAModel().getConditions(cfg["launch"]["altitude"])
    rows = []
    for m in mach_grid:
        out = {"mach": float(m)}
        for tag, aoa in (("0", 0.0), ("1", math.radians(1.0))):
            fc = FlightConditions(config)
            fc.setAtmosphericConditions(atm)
            fc.setMach(float(m))
            fc.setAOA(aoa)
            f = calc.getAerodynamicForces(config, fc, WarningSet())
            if tag == "0":
                out.update(cd_total=float(f.getCD()), cd_friction=float(f.getFrictionCD()), cd_pressure=float(f.getPressureCD()), cd_base=float(f.getBaseCD()))
            else:
                out.update(cna=float(f.getCN()) / aoa, xcp=float(f.getCP().x))
        rows.append(out)
    return rows


def components(rocket, fcid, cfg, machs, fin_count):
    """Per-component contributions (nose, body tube, fins) to CNa, CP and the drag coefficients at alpha = 1 deg.

    OpenRocket reports the drag of one fin; the contribution to the rocket is fin_count times that.
    """
    from info.openrocket.core.aerodynamics import BarrowmanCalculator, FlightConditions
    from info.openrocket.core.logging import WarningSet
    from info.openrocket.core.models.atmosphere import ExtendedISAModel

    config = rocket.getFlightConfiguration(fcid)
    calc = BarrowmanCalculator()
    atm = ExtendedISAModel().getConditions(cfg["launch"]["altitude"])
    rows = []
    aoa = math.radians(1.0)
    for m in machs:
        fc = FlightConditions(config)
        fc.setAtmosphericConditions(atm)
        fc.setMach(float(m))
        fc.setAOA(aoa)
        res = calc.getForceAnalysis(config, fc, WarningSet())
        for key in res.keySet():
            name = str(key.getName())
            if name not in ("Nose", "Body", "Fins"):
                continue
            f = res.get(key)
            n = fin_count if name == "Fins" else 1
            rows.append({"mach": float(m), "component": name, "cna": float(f.getCN()) / aoa, "xcp_m": float(f.getCP().x),
                         "cd_pressure": n * float(f.getPressureCD()), "cd_friction": n * float(f.getFrictionCD()), "cd_base": n * float(f.getBaseCD())})
    return rows


def flight(doc, rocket, fcid, cfg, wind, dt=0.01):
    from info.openrocket.core.document import Simulation
    from info.openrocket.core.simulation import FlightDataType as F
    from info.openrocket.core.simulation.listeners.system import ApogeeEndListener
    from info.openrocket.core.models.wind import WindModelType
    from info.openrocket.core.util import GeodeticComputationStrategy

    sim = Simulation(doc, rocket)
    sim.setFlightConfigurationId(fcid)
    o = sim.getOptions()
    la = cfg["launch"]
    o.setISAAtmosphere(True)
    o.setLaunchAltitude(la["altitude"])
    o.setLaunchLatitude(la["latitude"])
    o.setLaunchLongitude(la["longitude"])
    o.setLaunchRodLength(la["rail_length"])
    o.setLaunchRodAngle(math.radians(90.0 - la["elevation_deg"]))
    o.setGeodeticComputation(GeodeticComputationStrategy.FLAT)
    o.setWindModelType(WindModelType.AVERAGE)
    o.setWindSpeedAverage(common.WIND_SPEED if wind else 0.0)
    o.setWindSpeedDeviation(0.0)
    o.setWindTurbulenceIntensity(0.0)
    o.setWindDirection(math.radians(common.WIND_FROM_DEG))
    o.setLaunchIntoWind(True)
    o.setTimeStep(dt)
    o.setMaximumStepAngle(math.radians(3.0))
    sim.simulate([ApogeeEndListener()])
    data = sim.getSimulatedData()
    br = data.getBranch(0)
    n = br.getLength()
    series = {}
    for k, v in COLS.items():
        arr = br.get(getattr(F, v))
        series[k] = np.array([float(x) for x in arr]) if arr is not None else np.full(n, np.nan)
    events = {str(e.getType()): float(e.getTime()) for e in br.getEvents()}
    return series, events, float(data.getMaxAltitude()), float(data.getMaxVelocity()), float(data.getMaxMachNumber()), float(data.getMaxAcceleration()), float(data.getLaunchRodVelocity())


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("geometry")
    ap.add_argument("out")
    ap.add_argument("--ork")
    ap.add_argument("--fins", default="rounded", help="fin cross section: square | rounded | airfoil")
    ap.add_argument("--dt", type=float, default=0.01, help="OpenRocket RK4 time step [s]")
    a = ap.parse_args()
    out = Path(a.out)
    out.mkdir(parents=True, exist_ok=True)
    cfg = common.load_cfg()
    doc, rocket, fcid = bom.build(common.load_geometry(a.geometry), cfg, a.fins)
    if a.ork:
        bom.save(doc, a.ork)

    machs = np.round(np.arange(0.02, 3.0001, 0.02), 4)
    rows = static_aero(rocket, fcid, cfg, machs)
    tag = a.fins if a.dt == 0.01 else f"{a.fins}_dt{a.dt:g}"
    with open(out / f"or_static_{a.fins}.csv", "w", newline="") as fh:
        w = csv.DictWriter(fh, fieldnames=list(rows[0]))
        w.writeheader()
        w.writerows(rows)

    rows = components(rocket, fcid, cfg, [0.3, 0.8, 1.2, 2.0, 3.0], common.load_geometry(a.geometry)["fin_count"])
    with open(out / f"or_components_{a.fins}.csv", "w", newline="") as fh:
        w = csv.DictWriter(fh, fieldnames=list(rows[0]))
        w.writeheader()
        w.writerows(rows)

    from info.openrocket.core.masscalc import MassCalculator

    config = rocket.getFlightConfiguration(fcid)
    mc = MassCalculator()
    mass = {}
    for key, fn in (("structure", mc.calculateStructure), ("launch", mc.calculateLaunch)):
        rb = fn(config)
        mass[key] = {"mass_kg": float(rb.getMass()), "cg_x_m": float(rb.getCM().x), "ixx_kgm2": float(rb.getIxx()), "iyy_kgm2": float(rb.getIyy())}

    summary = {"mass_properties": mass}
    for case, wind in (("nowind", False), ("wind", True)):
        s, ev, apo, vmax, mmax, amax, vrod = flight(doc, rocket, fcid, cfg, wind, a.dt)
        with open(out / f"or_flight_{case}_{tag}.csv", "w", newline="") as fh:
            w = csv.writer(fh)
            w.writerow(list(s))
            w.writerows(zip(*[s[k] for k in s]))
        summary[case] = {"events": ev, "apogee_m": apo, "max_velocity": vmax, "max_mach": mmax, "max_acceleration": amax,
                         "launch_rod_velocity": vrod, "n_steps": int(len(s["t"]))}
    (out / f"or_summary_{tag}.json").write_text(json.dumps(summary, indent=2))
    print(json.dumps(summary, indent=2))


if __name__ == "__main__":
    main()
