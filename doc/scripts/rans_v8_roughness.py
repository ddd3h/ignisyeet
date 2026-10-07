#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
"""Post-hoc roughness correction of the RANS table comparison (V8, doc/data/cfd/rans_table.csv).

  rans_v8_roughness.py --bin target/release/ignisyeet --config examples/sample_cfd.toml
                       --table doc/data/cfd/rans_table.csv --out-csv doc/data/cfd/rans_table_roughness.csv

The RANS table of rans_table.csv was solved while the RANS mode ignored the configured wall roughness: its power-off
axial coefficient was ca_off = CFx(wall) + k(alpha) (base + extra), with the Barrowman base drag and extra_cd of the
configuration and the axial angle factor k(alpha) (aero::model::axial_alpha_factor). The CFD mode now adds the
Barrowman rough-minus-smooth friction increment (cfd::forces::roughness_increment), also multiplied by k(alpha):

  ca_off,corrected = ca_off,stored + k(alpha) dCf,rough,   dCf,rough = friction(roughness) - friction(0)

The increment is linear in the skin-friction coefficient (friction = cf S_wet / S_ref with one Reynolds number), so
it is the difference of the Barrowman friction terms of two `ignisyeet aero` runs of the same configuration with
method = "barrowman", one with the configured roughness and one with roughness 0 (aero_drag.csv). The script also
checks the friction ratio of the two runs against the skin_friction law of aero::model at the Reynolds number of the
aero_drag.csv row (--length: body length for the relative roughness).
CFx(wall) of each row is recovered as ca_off,stored - k(alpha) (base + extra). No case is solved again: the RANS
forces are the stored ones, and the Euler values are those stored in rans_table.csv with them.
"""
import argparse
import csv
import math
import os
import re
import shutil
import subprocess
import tempfile


def rows(path):
    with open(path) as f:
        return list(csv.DictReader(l for l in f if not l.startswith("#")))


def hermite(x, x0, x1, f0, f1, d0, d1):
    h = x1 - x0
    t = (x - x0) / h
    t2, t3 = t * t, t * t * t
    return (2 * t3 - 3 * t2 + 1) * f0 + (t3 - 2 * t2 + t) * h * d0 + (-2 * t3 + 3 * t2) * f1 + (t3 - t2) * h * d1


def axial_alpha_factor(a):
    """aero::model::axial_alpha_factor (a in rad)."""
    a = min(abs(a), math.pi / 2)
    a17 = math.radians(17)
    return hermite(a, 0, a17, 1.0, 1.3, 0, 0) if a <= a17 else hermite(a, a17, math.pi / 2, 1.3, 0.0, 0, 0)


def skin_friction(re_, m, rel):
    """aero::model::skin_friction."""
    if re_ < 1e4:
        return 1.48e-2
    turb = 1 / (1.50 * math.log(re_) - 5.6) ** 2
    turb = turb * (1 - 0.1 * m * m) if m < 1 else turb / (1 + 0.15 * m * m) ** 0.58
    if rel <= 0:
        return turb
    rough = 0.032 * rel ** 0.2
    rough = rough * (1 - 0.1 * m * m) if m < 1 else rough / (1 + 0.18 * m * m)
    return max(turb, rough)


def barrowman_drag(binary, config, roughness, work):
    """aero_drag.csv rows of `ignisyeet aero` with method barrowman and the given roughness."""
    base = os.path.dirname(os.path.abspath(config))
    out = os.path.join(work, f"out_{roughness:g}")
    txt = open(config).read()
    txt = re.sub(r'^(stl|eng) = "([^"]+)"', lambda m: f'{m.group(1)} = "{os.path.join(base, m.group(2))}"', txt, flags=re.M)
    txt = re.sub(r'^dir = .*$', f'dir = "{out}"', txt, count=1, flags=re.M)
    txt = re.sub(r'^method = "cfd".*$', 'method = "barrowman"', txt, count=1, flags=re.M)
    txt, n = re.subn(r'^roughness = [^\s#]+', f'roughness = {roughness!r}', txt, count=1, flags=re.M)
    assert n == 1, "no roughness line in the configuration"
    cfg = os.path.join(work, f"rough_{roughness:g}.toml")
    open(cfg, "w").write(txt)
    subprocess.run([binary, "aero", cfg], check=True, stdout=subprocess.DEVNULL)
    return rows(os.path.join(out, "aero_drag.csv"))


def at_mach(drag, m):
    r = [x for x in drag if abs(float(x["mach"]) - m) < 1e-9]
    assert r, f"Mach {m} is not a node of aero_drag.csv"
    return r[0]


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--bin", required=True)
    ap.add_argument("--config", required=True)
    ap.add_argument("--table", required=True)
    ap.add_argument("--out-csv", required=True)
    ap.add_argument("--length", type=float, default=1.5, help="body length of the sample [m] (cross-check only)")
    a = ap.parse_args()
    rough_m = float(re.search(r'^roughness = ([^\s#]+)', open(a.config).read(), re.M).group(1))
    work = tempfile.mkdtemp(prefix="rans_v8_roughness_")
    try:
        rough = barrowman_drag(a.bin, a.config, rough_m, work)
        smooth = barrowman_drag(a.bin, a.config, 0.0, work)
    finally:
        shutil.rmtree(work, ignore_errors=True)
    commit = subprocess.run(["git", "rev-parse", "--short", "HEAD"], capture_output=True, text=True,
                            cwd=os.path.dirname(os.path.abspath(__file__))).stdout.strip()
    keys = ["mach", "alpha_deg", "k_alpha", "cfx_wall", "base", "extra", "barrowman_fins", "roughness_increment", "rans_ca_off_stored",
            "rans_ca_off_corrected", "euler_ca_off", "diff_stored_percent", "diff_corrected_percent"]
    out = []
    for t in rows(a.table):
        m, al = float(t["mach"]), float(t["alpha_deg"])
        r, s = at_mach(rough, m), at_mach(smooth, m)
        inc = float(r["friction"]) - float(s["friction"])
        # roughness_increment is friction (1 - cf(smooth) / cf(rough)) at one Reynolds number: the two runs must share
        # the Reynolds number and every other term, and their friction ratio must be the ratio of the skin_friction law.
        re_ = float(r["reynolds"])
        assert abs(re_ / float(s["reynolds"]) - 1) < 1e-9 and r["base"] == s["base"] and r["extra"] == s["extra"]
        ratio = skin_friction(re_, m, 0) / skin_friction(re_, m, rough_m / a.length)
        assert abs(float(s["friction"]) / float(r["friction"]) - ratio) < 1e-4, (m, ratio)
        k = axial_alpha_factor(math.radians(al))
        stored, eu = float(t["rans_ca_off"]), float(t["euler_ca_off"])
        model = float(r["base"]) + float(r["extra"])
        corr = stored + k * inc
        out.append({"mach": m, "alpha_deg": al, "k_alpha": k, "cfx_wall": stored - k * model, "base": float(r["base"]),
                    "extra": float(r["extra"]), "barrowman_fins": float(r["fins"]), "roughness_increment": inc, "rans_ca_off_stored": stored,
                    "rans_ca_off_corrected": corr, "euler_ca_off": eu, "diff_stored_percent": 100 * (stored / eu - 1),
                    "diff_corrected_percent": 100 * (corr / eu - 1)})
    with open(a.out_csv, "w") as f:
        f.write(f"# Post-hoc roughness correction of the RANS table comparison (doc/scripts/rans_v8_roughness.py, commit {commit}): "
                f"not a rerun. rans_ca_off_stored and euler_ca_off are the values of rans_table.csv (RANS 5 mm table solved without "
                f"the roughness term; Euler 2 mm). roughness_increment = Barrowman friction with roughness {rough_m:g} m minus "
                f"roughness 0 (ignisyeet aero, method barrowman, {os.path.basename(a.config)}; = cfd::forces::roughness_increment). "
                f"rans_ca_off_corrected = stored + k_alpha * roughness_increment (k_alpha = axial_alpha_factor). "
                f"cfx_wall = stored - k_alpha (base + extra), the Barrowman base and extra_cd terms of the same configuration. barrowman_fins: the Barrowman fin-edge term (in the Euler ca_off, not in RANS).\n")
        f.write(",".join(keys) + "\n")
        for r in out:
            f.write(",".join(f"{r[k]:.6g}" for k in keys) + "\n")
    print(open(a.out_csv).read())


if __name__ == "__main__":
    main()
