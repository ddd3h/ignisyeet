# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2025-2026 西濱大将 (NISHIHAMA Daisuke)
"""Data of the figures of the geometry chapter (doc/data/geometry/), computed from the example models.

Usage: python scripts/geometry_data.py [--work DIR] [--bin PATH] [--gmsh-python PATH]

Runs the built `ignisyeet geom` / `ignisyeet aero` on the STL and on the CAD (STEP) file of the sample rocket
(examples/sample_rocket.*) and of the Basic Finner (examples/basic_finner/basic_finner.*), the sweeps of the number of
slices and of the fin threshold, a rotated copy of the sample STL, and a tessellation of the sample STEP at three
element sizes. It also cuts the sample STL with the slicing of the extraction, written again in numpy, and compares the
body radius it finds with the one of the program at every slice. The committed results are small; make_plots.py reads
them, so building the documentation needs neither gmsh nor this script.

Needs the gmsh python (CFD tools) for the CAD input, and the release binary (cargo build --release).
"""

import argparse
import csv
import json
import os
import shutil
import subprocess
import sys
from pathlib import Path

import numpy as np

ROOT = Path(__file__).resolve().parents[2]
DATA = ROOT / "doc" / "data" / "geometry"
EX = ROOT / "examples"


def args():
    p = argparse.ArgumentParser()
    p.add_argument("--work", type=Path, default=ROOT / ".claude" / "tmp" / "geomdata")
    p.add_argument("--bin", type=Path, default=ROOT / "target" / "release" / "ignisyeet")
    p.add_argument("--gmsh-python", type=Path, default=Path.home() / ".local/share/micromamba/envs/ignisyeet-cfd/bin/python")
    return p.parse_args()


A = args()
WORK = A.work.resolve()
WORK.mkdir(parents=True, exist_ok=True)
ENV = dict(os.environ, IGNISYEET_CFD_PREFIX=str(A.gmsh_python.parent.parent))


def run(cmd, **kw):
    r = subprocess.run([str(c) for c in cmd], capture_output=True, text=True, env=ENV, **kw)
    if r.returncode != 0:
        sys.exit(f"failed: {' '.join(map(str, cmd))}\n{r.stdout}{r.stderr}")
    return r


# ---- configurations ---------------------------------------------------------------------------------------------------
SAMPLE = (EX / "sample.toml").read_text()
FINNER = (EX / "basic_finner" / "basic_finner.toml").read_text()


def config(name, base, repl):
    t = base
    for a, b in repl:
        assert a in t, (name, a)
        t = t.replace(a, b)
    p = WORK / f"{name}.toml"
    p.write_text(t)
    return p


motor = f'"{EX}/sample_motor.eng"'
CASES = {
    "sample_stl": config("sample_stl", SAMPLE, [('"sample_motor.eng"', motor), ('stl = "sample_rocket.stl"', f'stl = "{EX}/sample_rocket.stl"')]),
    "sample_cad": config(
        "sample_cad",
        SAMPLE,
        [('"sample_motor.eng"', motor), ('stl = "sample_rocket.stl"', f'cad = "{EX}/sample_rocket.step"'), ("stl_scale = 0.001", "")],
    ),
    "basic_finner_stl": config(
        "basic_finner_stl", FINNER, [('"../sample_motor.eng"', motor), ('stl = "basic_finner.stl"', f'stl = "{EX}/basic_finner/basic_finner.stl"')]
    ),
    "basic_finner_cad": config(
        "basic_finner_cad",
        FINNER,
        [('"../sample_motor.eng"', motor), ('stl = "basic_finner.stl"', f'cad = "{EX}/basic_finner/basic_finner.step"'), ("stl_scale = 0.001", "")],
    ),
}


def geom(case, out, *sets, cfg=None):
    cmd = [A.bin, "geom", cfg or CASES[case], "--out", out, "--no-progress", "--quiet"]
    for s in sets:
        cmd += ["--set", s]
    r = run(cmd)
    return json.loads((Path(out) / "geometry.json").read_text()), r.stdout + r.stderr


def fin_row(g):
    f = g["fins"]
    return [f["count"], f["root_chord"], f["tip_chord"], f["span"], f["sweep"], f["x_le_root"], f["thickness"]]


FIN_COLS = ["fins", "root_chord_m", "tip_chord_m", "span_m", "sweep_m", "x_le_root_m", "thickness_m"]

prov = {"commit": run(["git", "-C", ROOT, "rev-parse", "--short", "HEAD"]).stdout.strip(), "binary": str(A.bin.name)}
prov["tools"] = run([A.gmsh_python, "-c", "import gmsh;print(gmsh.__version__)"]).stdout.strip()
prov["tools"] = f"gmsh {prov['tools']} (python, OpenCASCADE)"

# ---- the four extractions -------------------------------------------------------------------------------------------
geo, warn = {}, {}
for case in CASES:
    out = WORK / f"out_{case}"
    geo[case], warn[case] = geom(case, out)
    d = DATA / case
    d.mkdir(parents=True, exist_ok=True)
    shutil.copy(out / "geometry.json", d / "geometry.json")
    if case.endswith("_cad"):
        rep = json.loads((out / "cad" / "cad_extract_report.json").read_text())
        prov[case + "_tessellation"] = {"solids": rep["solids"], "faces": rep["faces"], "triangles": rep["triangles"], "notes": rep["notes"]}
    prov[case + "_warnings"] = [w for w in geo[case]["warnings"]]

# ---- Barrowman coefficients of the two Basic Finner inputs ----------------------------------------------------------
tabs = {}
for case in ("basic_finner_stl", "basic_finner_cad"):
    out = WORK / f"aero_{case}"
    run([A.bin, "aero", CASES[case], "--out", out, "--no-progress", "--quiet"])
    rows = [r for r in csv.DictReader(open(out / "aero_table.csv")) if float(r["alpha_deg"]) == 0.0]
    tabs[case] = {float(r["mach"]): (float(r["cna"]), float(r["xcp"])) for r in rows}
machs = sorted(tabs["basic_finner_stl"])
assert machs == sorted(tabs["basic_finner_cad"])
with open(DATA / "basic_finner_barrowman.csv", "w") as fh:
    fh.write("# Barrowman CNa [1/rad] and Xcp [m from the nose tip] at alpha = 0 for the Basic Finner given as STL and as STEP\n")
    fh.write("mach,cna_stl,cna_cad,xcp_stl_m,xcp_cad_m\n")
    for m in machs:
        (a, xa), (b, xb) = tabs["basic_finner_stl"][m], tabs["basic_finner_cad"][m]
        fh.write(f"{m:g},{a:.6f},{b:.6f},{xa:.7f},{xb:.7f}\n")
prov["basic_finner_barrowman"] = {
    "max_abs_dCNa_per_rad": max(abs(tabs["basic_finner_stl"][m][0] - tabs["basic_finner_cad"][m][0]) for m in machs),
    "max_abs_dXcp_m": max(abs(tabs["basic_finner_stl"][m][1] - tabs["basic_finner_cad"][m][1]) for m in machs),
    "machs": f"{machs[0]:g} .. {machs[-1]:g} ({len(machs)} values), alpha = 0",
}

# ---- sweeps of n_slices and fin_threshold ----------------------------------------------------------------------------
SLICES = [50, 100, 200, 400, 600, 1200, 2400]
THRESH = [0.0, 0.01, 0.02, 0.05, 0.1, 0.2, 0.4, 0.6]
for kind, key, values in (("n_slices", "aero.n_slices", SLICES), ("fin_threshold", "aero.fin_threshold", THRESH)):
    with open(DATA / f"sweep_{kind}.csv", "w") as fh:
        fh.write(f"# fin values of the sample rocket (STL and CAD) for different {kind}; the other keys are the defaults\n")
        fh.write(",".join(["input", kind] + FIN_COLS + ["nose_length_m"]) + "\n")
        for case in ("sample_stl", "sample_cad"):
            for v in values:
                g, _ = geom(case, WORK / f"out_{case}_sweep", f"{key}={v}")
                f = g["fins"]
                row = [case.split("_")[1], v] + (fin_row(g) if f else [0] * 7) + [g["nose"]["length"]]
                fh.write(",".join(str(x) for x in row) + "\n")

# ---- tessellation size of the CAD input ------------------------------------------------------------------------------
script = ROOT / "crates" / "cfd" / "src" / "mesh_step.py"
shutil.copy(script, WORK / "mesh_step.py")
with open(DATA / "tessellation_sensitivity.csv", "w") as fh:
    fh.write("# sample_rocket.step tessellated with the longest edge extent / divisions (48 elements per circle), then extracted\n")
    fh.write(",".join(["divisions", "triangles", "length_m", "diameter_m", "nose_length_m"] + FIN_COLS) + "\n")
    for div in (250, 400, 1000):
        stl = WORK / f"tess_{div}.stl"
        params = {"mode": "extract", "step_file": str(EX / "sample_rocket.step"), "unit_scale": 1.0, "heal_tol": 1e-6,
                  "affine": [1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0], "tess_size": 0.0, "divisions": float(div), "curvature": 48,
                  "tess_stl": str(stl), "report_json": str(WORK / f"tess_{div}.json")}
        (WORK / f"tess_{div}_params.json").write_text(json.dumps(params))
        run([A.gmsh_python, "mesh_step.py", f"tess_{div}_params.json"], cwd=WORK)
        rep = json.loads((WORK / f"tess_{div}.json").read_text())
        cfg = config(f"tess_{div}", SAMPLE, [('"sample_motor.eng"', motor), ('stl = "sample_rocket.stl"', f'stl = "{stl}"'), ("stl_scale = 0.001", "stl_scale = 1.0")])
        g, _ = geom(None, WORK / f"out_tess_{div}", cfg=cfg)
        fh.write(",".join(str(x) for x in [div, rep["triangles"], g["length"], 2 * g["ref_radius"], g["nose"]["length"]] + fin_row(g)) + "\n")

# ---- the sample STL rotated: the axis and the frame ------------------------------------------------------------------
sys.path.insert(0, str(Path(__file__).resolve().parent))


def read_stl(path):
    data = Path(path).read_bytes()
    n = int.from_bytes(data[80:84], "little")
    rec = np.frombuffer(data[84:84 + 50 * n], dtype=np.dtype([("n", "<f4", 3), ("v", "<f4", (3, 3)), ("a", "<u2")]))
    return rec["v"].astype(float)


def write_stl(path, tris):
    rec = np.zeros(len(tris), dtype=np.dtype([("n", "<f4", 3), ("v", "<f4", (3, 3)), ("a", "<u2")]))
    rec["v"] = tris
    Path(path).write_bytes(b"rotated sample rocket".ljust(80, b" ") + np.uint32(len(tris)).tobytes() + rec.tobytes())


def rot(axis, deg):
    a = np.radians(deg)
    c, s = np.cos(a), np.sin(a)
    i, j = [(1, 2), (2, 0), (0, 1)][axis]
    r = np.eye(3)
    r[i, i], r[i, j], r[j, i], r[j, j] = c, -s, s, c
    return r


tris = read_stl(EX / "sample_rocket.stl") * 0.001  # metres
R = rot(1, -38.0) @ rot(0, 24.0)
rt = tris @ R.T
write_stl(WORK / "sample_rotated.stl", rt)
cfg = config("sample_rotated", SAMPLE, [('"sample_motor.eng"', motor), ('stl = "sample_rocket.stl"', f'stl = "{WORK}/sample_rotated.stl"'), ("stl_scale = 0.001", "stl_scale = 1.0")])
gr, _ = geom(None, WORK / "out_sample_rotated", cfg=cfg)
area = 0.5 * np.linalg.norm(np.cross(rt[:, 1] - rt[:, 0], rt[:, 2] - rt[:, 0]), axis=1)
cen = (rt.sum(axis=1)) / 3.0
c0 = (cen * area[:, None]).sum(axis=0) / area.sum()
C = np.zeros((3, 3))
for k in range(3):
    d = rt[:, k] - c0
    C += (area[:, None, None] * d[:, :, None] * d[:, None, :]).sum(axis=0)
w, v = np.linalg.eigh(C)
e1 = v[:, 2]
aft_rust = np.array(gr["frame"]["aft_direction"])
aft_true = R @ np.array([0.0, 0.0, -1.0])
ang = lambda a, b: float(np.degrees(np.arccos(np.clip(abs(a @ b), -1, 1))))
fr0 = geo["sample_stl"]
fins = fr0["fins"]
prof = np.array([[p["x"], p["r"]] for p in fr0["profile"]])
length = fr0["length"]
orient = {
    "rotation": R.tolist(),
    "rotation_note": "rotation about x by 24 deg, then about y by -38 deg, applied to the sample STL (in metres)",
    "centroid_m": c0.tolist(),
    "eigenvalues_m2_relative": (w[::-1] / w.sum()).tolist(),
    "principal_axis": e1.tolist(),
    "angle_principal_axis_to_true_axis_deg": ang(e1, aft_true),
    "angle_program_axis_to_true_axis_deg": ang(aft_rust, aft_true),
    "program_aft_direction": aft_rust.tolist(),
    "true_aft_direction": aft_true.tolist(),
    "rotated_length_m": gr["length"],
    "rotated_ref_radius_m": gr["ref_radius"],
    "rotated_fins": gr["fins"],
    "fin_centre_fraction": (fins["x_le_root"] + 0.5 * fins["root_chord"]) / length,
    "end_radius_sum_nose_end_m": float(prof[:2, 1].sum()),
    "end_radius_sum_tail_end_m": float(prof[-2:, 1].sum()),
}
(DATA / "sample_axis.json").write_text(json.dumps(orient, indent=1))

# ---- slicing of the sample STL written again, compared with the program ---------------------------------------------
N_BINS, N_S = 72, 600
fr = fr0["frame"]
aft = np.array(fr["aft_direction"])
c = np.array(fr["axis_point"])
tris = read_stl(EX / "sample_rocket.stl") * 0.001


def section_frame(ex):
    helper = np.array([1.0, 0, 0]) if abs(ex[0]) < 0.9 else np.array([0, 1.0, 0])
    ey = np.cross(ex, helper)
    ey /= np.linalg.norm(ey)
    return ey, np.cross(ex, ey)


def slice_all(ex):
    """Cut segments per slice (y, z in the section frame) and body radius of the slice."""
    ey, ez = section_frame(ex)
    loc = np.stack([(tris - c) @ ex, (tris - c) @ ey, (tris - c) @ ez], axis=-1)
    xmin = loc[:, :, 0].min()
    length = loc[:, :, 0].max() - xmin
    dx = length / N_S
    segs = [[] for _ in range(N_S)]
    for t in loc:
        xs = t[:, 0] - xmin
        i0 = max(int(np.ceil(xs.min() / dx - 0.5)), 0)
        i1 = min(int(np.floor(xs.max() / dx - 0.5)), N_S - 1)
        for i in range(i0, i1 + 1):
            xi = (i + 0.5) * dx
            pts = []
            for a in range(3):
                b = (a + 1) % 3
                if (xs[a] >= xi) != (xs[b] >= xi) and len(pts) < 2:
                    s = (xi - xs[a]) / (xs[b] - xs[a])
                    p = t[a] + (t[b] - t[a]) * s
                    pts.append(p[1:])
            if len(pts) == 2:
                segs[i].append(pts)
    rb = []
    for sg in segs:
        bins = np.zeros(N_BINS)
        for p in np.array(sg).reshape(-1, 2) if sg else []:
            r, th = np.hypot(*p), np.arctan2(p[1], p[0])
            b = min(int((th + np.pi) / (2 * np.pi) * N_BINS), N_BINS - 1)
            bins[b] = max(bins[b], r)
        f = np.sort(bins[bins > 0])
        rb.append(f[len(f) // 2] if len(f) >= 3 else (f.max() if len(f) else 0.0))
    return segs, np.array(rb), length, dx, xmin


best = None
for sign in (1.0, -1.0):
    segs, rb, length_s, dx, xmin = slice_all(sign * aft)
    # with the axis along the aft direction the slices run from the nose; against it, from the tail
    ref = np.array([p["r"] for p in fr0["profile"]])
    cand = rb if sign > 0 else rb[::-1]
    dev = float(np.abs(cand - ref).max())
    if best is None or dev < best[0]:
        best = (dev, sign, segs, rb)
dev, sign, segs, rb = best
if sign < 0:
    segs, rb = segs[::-1], rb[::-1]
dxs = length_s / N_S
stations = []
fin_thr = 0.05 * fr0["ref_radius"]
sections = {}
for name, target in (("A", 0.12), ("B", 0.80), ("C", 1.33), ("D", 1.42)):
    i = int(round(target / dxs - 0.5))
    seg = np.array(segs[i]).reshape(-1, 2, 2)
    sections[name] = seg.astype(np.float32)
    stations.append({"name": name, "index": i, "x_m": (i + 0.5) * dxs, "r_body_numpy_m": float(rb[i]), "r_body_program_m": fr0["profile"][i]["r"],
                     "fin_span_program_m": fr0["profile"][i]["fin_span"], "segments": int(len(seg))})
np.savez_compressed(DATA / "sample_sections.npz", **sections)
prov["sections"] = {
    "source": "examples/sample_rocket.stl (scale 0.001), cut by the slicing of the extraction written in numpy (geometry_data.py)",
    "n_slices": N_S,
    "max_abs_difference_of_body_radius_to_program_all_slices_m": dev,
    "section_frame_axis_sign_matching_the_program": sign,
    "stations": stations,
    "fin_threshold_m": fin_thr,
}

# ---- provenance ------------------------------------------------------------------------------------------------------
prov["source"] = "examples/sample_rocket.stl and .step (the sample rocket), examples/basic_finner/basic_finner.stl and .step (the Basic Finner)"
prov["commands"] = [
    "ignisyeet geom <config> --out <dir>  (sample.toml with rocket.stl or rocket.cad = sample_rocket.step; basic_finner.toml with stl or cad = basic_finner.step)",
    "ignisyeet geom ... --set aero.n_slices=<n> / --set aero.fin_threshold=<t>",
    "ignisyeet aero <basic finner config> --out <dir>   (Barrowman, default method)",
    "python mesh_step.py (mode extract, divisions 250 / 400 / 1000) then ignisyeet geom on the STL with stl_scale = 1",
    "python scripts/geometry_gmsh.py examples/sample_rocket.step doc/data/geometry   (faces, fine tail tessellation, coarse tessellation)",
]
prov["cfd_prefix"] = "IGNISYEET_CFD_PREFIX (the sample.toml has no [aero.cfd] prefix; the Basic Finner configuration has)"
prov["date"] = subprocess.run(["date", "+%F"], capture_output=True, text=True).stdout.strip()
(DATA / "provenance.json").write_text(json.dumps(prov, indent=1))
run([A.gmsh_python, ROOT / "doc" / "scripts" / "geometry_gmsh.py", EX / "sample_rocket.step", DATA])
print("done; max body radius difference numpy - program:", dev)
