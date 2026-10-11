# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2025-2026 西濱大将 (NISHIHAMA Daisuke)
"""Run RocketPy on the sample rocket: static aerodynamics (its Barrowman part) and the ascent flights.

Usage: python run_rocketpy.py <geometry.json> <out_dir> --drag <name>=<csv> [--drag ...]

Each --drag gives one variant: a CSV with the columns mach, cd_off, cd_on (power-off and power-on
drag coefficient against Mach number at alpha = 0). RocketPy does not compute drag itself.
Writes to <out_dir>: rp_static.csv, rp_flight_<variant>_<case>.csv, rp_summary.json.
"""

import argparse
import csv
import json
import math
import sys
from pathlib import Path

import numpy as np
import pandas as pd

sys.path.insert(0, str(Path(__file__).resolve().parent))
import common  # noqa: E402


def build(geom, cfg, drag_csv):
    from rocketpy import GenericMotor, Rocket

    d = pd.read_csv(drag_csv)
    off = [[0.0, d.cd_off.iloc[0]]] + [[m, c] for m, c in zip(d["mach"], d.cd_off) if m > 0]
    on = [[0.0, d.cd_on.iloc[0]]] + [[m, c] for m, c in zip(d["mach"], d.cd_on) if m > 0]
    name, dia, length, prop, total, curve = common.read_eng(common.EXAMPLES / "sample_motor.eng")
    motor = GenericMotor(
        thrust_source=[[t, f] for t, f in curve],
        burn_time=curve[-1][0],
        chamber_radius=dia / 2,
        chamber_height=length,
        chamber_position=length / 2,  # centre of the propellant, from the nozzle exit
        propellant_initial_mass=prop,
        nozzle_radius=cfg["motor"]["nozzle_exit_diameter"] / 2,
        dry_mass=0.0,  # the casing is part of dry_mass of the rocket
        center_of_dry_mass_position=length / 2,
        dry_inertia=(0, 0, 0),
        interpolation_method="linear",
    )
    r = cfg["rocket"]
    rocket = Rocket(
        radius=geom["radius"],
        mass=r["dry_mass"],
        inertia=(r["iyy_dry"], r["iyy_dry"], r["ixx_dry"]),  # (I11, I22, I33): transverse, transverse, roll
        power_off_drag=off,
        power_on_drag=on,
        center_of_mass_without_motor=r["cg_dry"],
        coordinate_system_orientation="nose_to_tail",
    )
    rocket.add_motor(motor, position=geom["length"])  # nozzle exit at the base
    rocket.add_nose(length=geom["nose_length"], kind="tangent", position=0.0)
    rocket.add_trapezoidal_fins(
        n=geom["fin_count"], root_chord=geom["fin_root"], tip_chord=geom["fin_tip"], span=geom["fin_span"],
        position=geom["fin_x_le"], sweep_length=geom["fin_sweep"],
    )
    return rocket


def environment(cfg, wind):
    from rocketpy import Environment

    la = cfg["launch"]
    env = Environment(latitude=la["latitude"], longitude=la["longitude"], elevation=la["altitude"])
    if wind:
        w = math.radians(common.WIND_FROM_DEG)  # wind from 270 deg: blowing towards the east (+u)
        env.set_atmospheric_model(type="custom_atmosphere", wind_u=-common.WIND_SPEED * math.sin(w), wind_v=-common.WIND_SPEED * math.cos(w))
    else:
        env.set_atmospheric_model(type="standard_atmosphere")
    return env


def static(rocket, machs):
    rows = []
    for m in machs:
        rows.append({"mach": float(m), "cna": float(rocket.total_lift_coeff_der(m)), "xcp": float(rocket.cp_position(m))})
    return rows


def fly(rocket, env, cfg):
    from rocketpy import Flight

    la = cfg["launch"]
    fl = Flight(rocket=rocket, environment=env, rail_length=la["rail_length"], inclination=la["elevation_deg"], heading=la["azimuth_deg"],
                terminate_on_apogee=True, max_time=60, rtol=1e-8, atol=1e-8, time_overshoot=False)
    t = np.array(fl.time)
    el = cfg["launch"]["altitude"]
    s = {
        "t": t, "alt": np.array([fl.z(x) for x in t]) - el, "v": np.array([fl.speed(x) for x in t]), "vz": np.array([fl.vz(x) for x in t]),
        "mach": np.array([fl.mach_number(x) for x in t]), "aoa": np.array([fl.angle_of_attack(x) for x in t]), "acc": np.array([fl.acceleration(x) for x in t]),
        "px": np.array([fl.x(x) for x in t]), "py": np.array([fl.y(x) for x in t]),
        "mass": np.array([rocket.total_mass(x) for x in t]),
    }
    res = {
        "apogee_m": float(fl.apogee - el), "apogee_time": float(fl.apogee_time), "max_velocity": float(fl.max_speed),
        "max_mach": float(fl.max_mach_number), "max_acceleration": float(fl.max_acceleration), "rail_exit_time": float(fl.out_of_rail_time),
        "rail_exit_speed": float(fl.out_of_rail_velocity), "apogee_east": float(fl.x(fl.apogee_time)), "apogee_north": float(fl.y(fl.apogee_time)),
    }
    return s, res


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("geometry")
    ap.add_argument("out")
    ap.add_argument("--drag", action="append", required=True, help="name=csv with mach,cd_off,cd_on")
    a = ap.parse_args()
    out = Path(a.out)
    out.mkdir(parents=True, exist_ok=True)
    geom, cfg = common.load_geometry(a.geometry), common.load_cfg()
    summary = {}
    for spec in a.drag:
        name, csv_path = spec.split("=", 1)
        rocket = build(geom, cfg, csv_path)
        if name == a.drag[0].split("=", 1)[0]:
            rows = static(rocket, np.round(np.arange(0.02, 3.0001, 0.02), 4))
            with open(out / "rp_static.csv", "w", newline="") as fh:
                w = csv.DictWriter(fh, fieldnames=list(rows[0]))
                w.writeheader()
                w.writerows(rows)
        for case, wind in (("nowind", False), ("wind", True)):
            s, res = fly(rocket, environment(cfg, wind), cfg)
            with open(out / f"rp_flight_{name}_{case}.csv", "w", newline="") as fh:
                w = csv.writer(fh)
                w.writerow(list(s))
                w.writerows(zip(*[s[k] for k in s]))
            summary.setdefault(name, {})[case] = res
    (out / "rp_summary.json").write_text(json.dumps(summary, indent=2))
    print(json.dumps(summary, indent=2))


if __name__ == "__main__":
    main()
