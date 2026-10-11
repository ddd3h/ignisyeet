# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2025-2026 西濱大将 (NISHIHAMA Daisuke)
"""Collect the results of the three simulators into doc/data/xcheck_tools/.

Usage: python make_comparison.py <work dir>
<work dir> holds ign/ (run_ignisyeet.sh), or/ (run_openrocket.py) and rp/ (run_rocketpy.py).

Writes static_aero.csv, flight_nowind.csv, flight_wind.csv, summary.csv, sensitivity.csv.
The tools are labelled ign (IgnisYeet), or (OpenRocket), rp_or and rp_ign (RocketPy with the drag
coefficient of OpenRocket and of IgnisYeet).
"""

import json
import sys
from pathlib import Path

import numpy as np
import pandas as pd

sys.path.insert(0, str(Path(__file__).resolve().parent))
import common  # noqa: E402

BURNOUT = 3.1  # s, end of sample_motor.eng
GRID = np.round(np.arange(0.0, 30.0001, 0.05), 3)


def load_ign(d):
    tr = pd.read_csv(d / "trajectory.csv")
    s = json.loads((d / "summary.json").read_text())
    k = int(tr["up"].idxmax())
    tr = tr.iloc[: k + 1]
    t = tr.t.to_numpy()
    vel = tr[["vel_e", "vel_n", "vel_u"]].to_numpy()
    acc = np.linalg.norm(np.gradient(vel, t, axis=0), axis=1)
    return {
        "t": t, "alt": tr.up.to_numpy(), "v": np.linalg.norm(vel, axis=1), "mach": tr.mach.to_numpy(), "acc": acc,
        "east": tr.east.to_numpy(), "north": tr.north.to_numpy(), "aoa": tr.alpha_deg.to_numpy(),
        "rail_exit_time": s["rail_exit_time"], "rail_exit_speed": s["rail_exit_speed"],
    }


def load_or(d, tag):
    f = pd.read_csv(d / f"or_flight_{tag}.csv")
    j = json.loads((d / f"or_summary_{tag.split('_', 1)[1]}.json").read_text())[tag.split("_", 1)[0]]
    return {
        "t": f.t.to_numpy(), "alt": f.alt.to_numpy(), "v": f.v.to_numpy(), "mach": f.mach.to_numpy(), "acc": f.acc.to_numpy(), "aoa": np.degrees(f.aoa.to_numpy()),
        "px": f.px.to_numpy(), "py": f.py.to_numpy(),
        "rail_exit_time": j["events"]["Launch rod clearance"], "rail_exit_speed": j["launch_rod_velocity"],
    }


def load_rp(d, variant, case):
    f = pd.read_csv(d / f"rp_flight_{variant}_{case}.csv")
    j = json.loads((d / "rp_summary.json").read_text())[variant][case]
    return {
        "t": f.t.to_numpy(), "alt": f.alt.to_numpy(), "v": f.v.to_numpy(), "mach": f.mach.to_numpy(), "acc": f.acc.to_numpy(), "aoa": f.aoa.to_numpy(),
        "px": f.px.to_numpy(), "py": f.py.to_numpy(),
        "rail_exit_time": j["rail_exit_time"], "rail_exit_speed": j["rail_exit_speed"],
    }


def metrics(s):
    t = s["t"]
    k = int(np.argmax(s["alt"]))
    horiz = np.hypot(s["east"][k], s["north"][k]) if "east" in s else np.hypot(s["px"][k], s["py"][k])
    return {
        "apogee_m": float(s["alt"][k]),
        "apogee_time_s": float(t[k]),
        "max_velocity_ms": float(np.max(s["v"])),
        "max_mach": float(np.max(s["mach"])),
        "max_acceleration_ms2": float(np.nanmax(s["acc"][t <= BURNOUT - 0.05])),  # RocketPy gives spikes at the burnout step
        "rail_exit_speed_ms": float(s["rail_exit_speed"]),
        "rail_exit_time_s": float(s["rail_exit_time"]),
        "burnout_altitude_m": float(np.interp(BURNOUT, t, s["alt"])),
        "burnout_velocity_ms": float(np.interp(BURNOUT, t, s["v"])),
        "apogee_horizontal_m": float(horiz),
    }


def on_grid(s, key):
    out = np.interp(GRID, s["t"], s[key])
    out[GRID > s["t"][-1] + 1e-9] = np.nan
    return out


def main():
    work = Path(sys.argv[1])
    out = common.DATA
    out.mkdir(parents=True, exist_ok=True)
    ign, orr, rp = work / "ign", work / "or", work / "rp"

    # static aerodynamics (alpha = 0 for the drag, a small alpha for the slope and the centre of pressure)
    a = pd.read_csv(ign / "nowind" / "aero_drag.csv")
    o = pd.read_csv(orr / "or_static_rounded.csv")
    r = pd.read_csv(rp / "rp_static.csv")
    m = o["mach"].to_numpy()
    st = pd.DataFrame({
        "mach": m,
        "cna_ign": np.interp(m, a.mach, a.cna), "xcp_ign": np.interp(m, a.mach, a.xcp),
        "cd_off_ign": np.interp(m, a.mach, a.cd_off), "cd_on_ign": np.interp(m, a.mach, a.cd_on),
        "cna_or": o.cna, "xcp_or": o.xcp, "cd_or": o.cd_total,
        "cd_friction_or": o.cd_friction, "cd_pressure_or": o.cd_pressure, "cd_base_or": o.cd_base,
        "cna_rp": r.cna, "xcp_rp": r.xcp,
    })
    for fins in ("square", "airfoil"):
        st[f"cd_or_{fins}"] = pd.read_csv(orr / f"or_static_{fins}.csv").cd_total
    st.round(6).to_csv(out / "static_aero.csv", index=False)

    rows, flights = [], {}
    for case in ("nowind", "wind"):
        series = {
            "ign": load_ign(ign / case), "or": load_or(orr, f"{case}_rounded"),
            "rp_or": load_rp(rp, "or", case), "rp_ign": load_rp(rp, "ign", case),
        }
        fl = pd.DataFrame({"t": GRID})
        for k, s in series.items():
            mt = metrics(s)
            rows += [{"case": case, "tool": k, "metric": q, "value": v} for q, v in mt.items()]
            fl[f"alt_{k}"] = on_grid(s, "alt")
            fl[f"v_{k}"] = on_grid(s, "v")
        fl.round(4).to_csv(out / f"flight_{case}.csv", index=False)
        flights[case] = series
        if case == "wind":  # angle of attack in the first seconds (rail exit and weathercocking)
            ga = np.round(np.arange(0.0, 3.0001, 0.01), 3)
            al = pd.DataFrame({"t": ga})
            for k, s in series.items():
                al[f"aoa_{k}"] = np.interp(ga, s["t"], s["aoa"])
            al.round(4).to_csv(out / "alpha_wind.csv", index=False)
    pd.DataFrame(rows).round(6).to_csv(out / "summary.csv", index=False)

    # sensitivity of the apogee to inputs that have no counterpart in the other tools
    sens = []
    for case in ("nowind", "wind"):
        sens.append({"case": case, "variant": "ign extra_cd 0.02 (sample as is)", "apogee_m": metrics(load_ign(ign / f"{case}_cd"))["apogee_m"]})
        for fins in ("rounded", "square", "airfoil"):
            sens.append({"case": case, "variant": f"or fins {fins}", "apogee_m": metrics(load_or(orr, f"{case}_{fins}"))["apogee_m"]})
        sens.append({"case": case, "variant": "or fins rounded, dt 0.002 s", "apogee_m": metrics(load_or(orr, f"{case}_rounded_dt0.002"))["apogee_m"]})
    pd.DataFrame(sens).round(3).to_csv(out / "sensitivity.csv", index=False)
    print(pd.DataFrame(rows).pivot(index="metric", columns=["case", "tool"], values="value").round(3).to_string())


if __name__ == "__main__":
    main()
