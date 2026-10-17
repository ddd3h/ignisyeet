# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2024-2026 西濱大将 (NISHIHAMA Daisuke)
"""Inputs shared by the three simulators of the cross-comparison.

Everything is read from the files of the repository: the geometry that `ignisyeet geom`
extracted from examples/sample_rocket.stl (geometry.json), the mass properties, the launch
and the atmosphere from examples/sample.toml, and the motor from examples/sample_motor.eng.
Only the nose-cone shape (a tangent ogive of 0.300 m, found by fitting profile.csv) is a
constant here.
"""

import json
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
EXAMPLES = ROOT / "examples"
DATA = ROOT / "doc" / "data" / "xcheck_tools"
G0 = 9.80665

NOSE_LENGTH = 0.300  # tangent ogive; rms deviation from profile.csv 0.003 mm (fit in README)
WIND_SPEED = 4.0  # m/s, constant with height
WIND_FROM_DEG = 270.0  # blowing from the west = from the side the rail leans to (launch azimuth of sample.toml)
MOTOR_NO_CASE = DATA / "sample_motor_nocase.eng"  # total mass = propellant mass: the case is part of dry_mass


def load_cfg():
    return tomllib.loads((EXAMPLES / "sample.toml").read_text())


def load_geometry(geom_json):
    g = json.loads(Path(geom_json).read_text())
    f = g["fins"]
    return {
        "length": g["length"],
        "radius": g["base_radius"],
        "nose_length": NOSE_LENGTH,
        "fin_count": f["count"],
        "fin_root": f["root_chord"],
        "fin_tip": f["tip_chord"],
        "fin_span": f["span"],
        "fin_sweep": f["sweep"],
        "fin_thickness": f["thickness"],
        "fin_x_le": f["x_le_root"],
    }


def read_eng(path):
    """Parse a RASP .eng file: (name, dia_m, len_m, prop_kg, total_kg, [(t, F), ...])."""
    rows = [ln.split() for ln in Path(path).read_text().splitlines() if ln.strip() and not ln.startswith(";")]
    h = rows[0]
    return h[0], float(h[1]) * 1e-3, float(h[2]) * 1e-3, float(h[4]), float(h[5]), [(float(a), float(b)) for a, b in rows[1:]]


def write_nocase_eng():
    """sample_motor.eng with total mass = propellant mass (the casing mass is in dry_mass)."""
    out = []
    for ln in (EXAMPLES / "sample_motor.eng").read_text().splitlines():
        p = ln.split()
        if ln.startswith(";") or len(p) != 7:
            out.append(ln)
        else:
            out.append(" ".join(p[:5] + [p[4], p[6]]))
    MOTOR_NO_CASE.parent.mkdir(parents=True, exist_ok=True)
    MOTOR_NO_CASE.write_text("\n".join(out) + "\n")
