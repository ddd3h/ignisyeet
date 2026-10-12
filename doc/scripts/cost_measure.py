# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2025-2026 西濱大将 (NISHIHAMA Daisuke)
"""Measure the run time, CPU time and peak memory of the ignisyeet stages (doc/data/cost/*.csv).

Usage: python doc/scripts/cost_measure.py SCRATCH_DIR EXPERIMENT [EXPERIMENT ...]
       python doc/scripts/cost_measure.py SCRATCH_DIR all

Runs the release binary (target/release/ignisyeet) on examples/sample.toml. Every timing is repeated
REPEATS times (3) and the median wall time is written, together with the median user+system CPU time,
the largest resident set size and the load average read before each run. Outputs go below SCRATCH_DIR
and are never written into the repository. Experiments:

  stages    each stage once, 4 threads (geom, aero barrowman / panel, sim, dispersion, report, cached aero)
  threads   panel table, wind grid and Monte Carlo at 1, 2, 4, 8 and 20 threads
  panel     panel table time and memory against the surface panel count (4 threads)
  grid      Barrowman table time and file size against the Mach x alpha grid (4 threads)
  disp      wind-grid and Monte Carlo time against the number of flights (4 threads)
  sim       one flight against integrator, dt and output_interval; file sizes
  example   panel table + 36 x 6 wind grid + 10^4 Monte Carlo samples in a row (the chapter's worked estimate)
  plots     python/plot.py on the result files of one run
  cad       CAD geometry with a cold and a cached tessellation; CFD mesh generation
"""

import csv
import json
import os
import re
import shutil
import statistics
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent.parent
BIN = Path(os.environ.get("COST_BIN", ROOT / "target" / "release" / "ignisyeet"))
SAMPLE = Path(os.environ.get("COST_SAMPLE", ROOT / "examples" / "sample.toml"))
DATA = Path(os.environ.get("COST_DATA", ROOT / "doc" / "data" / "cost"))
REPEATS = int(os.environ.get("COST_REPEATS", "3"))
# Thread counts of the `threads` experiment and a tag for its file name (threads_<tag>.csv), e.g. for another machine.
THREADS = [int(v) for v in os.environ.get("COST_THREADS", "1,2,4,8,20").split(",")]
TAG = os.environ.get("COST_TAG", "")


def load1() -> float:
    return float(Path("/proc/loadavg").read_text().split()[0])


def run(args, scratch: Path, sets=(), cwd=ROOT):
    """One run: returns dict(wall, cpu, rss_mb, load, text)."""
    cmd = ["/usr/bin/time", "-f", "%e %U %S %M", "-o", str(scratch / "time.txt"), str(BIN), "--no-progress"]
    cmd += [str(a) for a in args]
    for s in sets:
        cmd += ["--set", s]
    ld = load1()
    p = subprocess.run(cmd, cwd=cwd, capture_output=True, text=True)
    tl = (scratch / "time.txt").read_text().strip().splitlines()[-1].split()
    wall, user, sys_, rss = float(tl[0]), float(tl[1]), float(tl[2]), float(tl[3])
    return dict(wall=wall, cpu=user + sys_, rss_mb=rss / 1024.0, load=ld, code=p.returncode, text=p.stdout + p.stderr)


def median_run(args, scratch: Path, sets=(), repeats=REPEATS, before=None, grab=None):
    rs = []
    for _ in range(repeats):
        if before:
            before()
        r = run(args, scratch, sets)
        if r["code"] not in (0, 3):
            raise SystemExit(f"failed ({r['code']}): {args} {sets}\n{r['text'][-800:]}")
        rs.append(r)
    return dict(
        wall=statistics.median(r["wall"] for r in rs),
        wall_min=min(r["wall"] for r in rs),
        wall_max=max(r["wall"] for r in rs),
        cpu=statistics.median(r["cpu"] for r in rs),
        rss_mb=max(r["rss_mb"] for r in rs),
        load=statistics.median(r["load"] for r in rs),
        text=rs[-1]["text"],
        grab=statistics.median(float(re.search(grab, r["text"]).group(1)) for r in rs) if grab else None,
    )


def header(f, title):
    blocks = Path("/proc/cpuinfo").read_text().split("\n\n")
    cpu = [l.split(":", 1)[1].strip() for l in blocks[0].splitlines() if l.startswith("model name")][0]
    cores = set()
    for b in blocks:
        kv = dict((k.strip(), v.strip()) for k, _, v in (l.partition(":") for l in b.splitlines()))
        if "core id" in kv:
            cores.add((kv.get("physical id"), kv["core id"]))
    commit = os.environ.get("COST_COMMIT") or subprocess.run(["git", "rev-parse", "--short", "HEAD"], cwd=ROOT, capture_output=True, text=True).stdout.strip()
    f.write(f"# {title}\n")
    f.write(f"# {cpu} ({os.cpu_count()} threads, {len(cores)} cores), release binary of commit {commit}, examples/sample.toml, {time.strftime('%Y-%m-%d')}.\n")
    f.write(f"# Median of {REPEATS} runs (wall_min/wall_max: range); cpu = user+system time of the process; rss_mb = largest peak resident memory;\n")
    f.write("# load = 1-minute load average before the runs (median over the runs). Written by doc/scripts/cost_measure.py.\n")


def write(name, title, cols, rows):
    DATA.mkdir(parents=True, exist_ok=True)
    with open(DATA / name, "w", newline="") as f:
        header(f, title)
        w = csv.writer(f, lineterminator="\n")
        w.writerow(cols)
        for r in rows:
            w.writerow([format(v, ".6g") if isinstance(v, float) else v for v in r])
    print("wrote", DATA / name, flush=True)


def fmt(m):
    return [m["wall"], m["wall_min"], m["wall_max"], m["cpu"], m["rss_mb"], m["load"]]


COLS = ["wall_s", "wall_min_s", "wall_max_s", "cpu_s", "rss_mb", "load"]


def fresh(p: Path):
    shutil.rmtree(p, ignore_errors=True)
    p.mkdir(parents=True)
    return p


def stages(S: Path):
    T = ["resources.threads=4"]
    bar, pan = S / "o_bar", S / "o_pan"
    rows = []
    m = median_run(["geom", SAMPLE, "--out", bar], S, T)
    rows.append(["geom", "-"] + fmt(m))
    m = median_run(["aero", SAMPLE, "--force", "--out", bar], S, T)
    rows.append(["aero", "barrowman"] + fmt(m))
    m = median_run(["aero", SAMPLE, "--out", bar], S, T)
    rows.append(["aero", "barrowman, cached table"] + fmt(m))
    m = median_run(["aero", SAMPLE, "--force", "--out", pan], S, T + ["aero.method=panel"])
    rows.append(["aero", "panel"] + fmt(m))
    m = median_run(["aero", SAMPLE, "--out", pan], S, T + ["aero.method=panel"])
    rows.append(["aero", "panel, cached table"] + fmt(m))
    for ig in ("rk4", "rk45", "dop853"):
        m = median_run(["sim", SAMPLE, "--aero-table", bar, "--out", S / f"o_sim_{ig}"], S, T + [f"sim.integrator={ig}"])
        rows.append(["sim", ig] + fmt(m))
    m = median_run(["dispersion", SAMPLE, "--aero-table", bar, "--out", S / "o_grid"], S, T + ["output.kml=false"])
    rows.append(["dispersion", "wind_grid 112 flights, rk4"] + fmt(m))
    m = median_run(["dispersion", SAMPLE, "--aero-table", bar, "--out", S / "o_grid45"], S, T + ["output.kml=false", "sim.integrator=rk45"])
    rows.append(["dispersion", "wind_grid 112 flights, rk45"] + fmt(m))
    m = median_run(["dispersion", SAMPLE, "--aero-table", bar, "--out", S / "o_mc"], S, T + ["output.kml=false", "sim.integrator=rk45", "dispersion.mode=monte_carlo"])
    rows.append(["dispersion", "monte_carlo 1000 x 2, rk45"] + fmt(m))
    full = S / "o_full"
    run(["aero", SAMPLE, "--force", "--out", full], S, T)
    run(["sim", SAMPLE, "--aero-table", full, "--out", full], S, T)
    run(["dispersion", SAMPLE, "--aero-table", full, "--out", full], S, T + ["output.kml=false", "sim.integrator=rk45"])
    run(["dispersion", SAMPLE, "--aero-table", full, "--out", full], S, T + ["output.kml=false", "sim.integrator=rk45", "dispersion.mode=monte_carlo"])
    m = median_run(["report", SAMPLE, "--out", full], S, [])
    rows.append(["report", "-"] + fmt(m))
    write("stages.csv", "Run time of each stage on the sample rocket, 4 threads", ["stage", "variant"] + COLS, rows)


def threads(S: Path):
    bar = S / "o_bar"
    if not (bar / "aero_table.csv").exists():
        run(["aero", SAMPLE, "--force", "--out", bar], S, ["resources.threads=4"])
    rows = []
    for t in THREADS:
        T = [f"resources.threads={t}"]
        m = median_run(["aero", SAMPLE, "--force", "--out", S / "o_pan_t"], S, T + ["aero.method=panel"])
        rows.append(["panel_table", t] + fmt(m))
        m = median_run(["dispersion", SAMPLE, "--aero-table", bar, "--out", S / "o_gt"], S, T + ["output.kml=false", "dispersion.directions=16"])
        rows.append(["wind_grid_rk4_224", t] + fmt(m))
        m = median_run(["dispersion", SAMPLE, "--aero-table", bar, "--out", S / "o_mt"], S, T + ["output.kml=false", "sim.integrator=rk45", "dispersion.mode=monte_carlo", "dispersion.monte_carlo.samples=2000"])
        rows.append(["monte_carlo_rk45_4000", t] + fmt(m))
        write(f"threads_{TAG}.csv" if TAG else "threads.csv", "Run time against the thread count (resources.threads)", ["task", "threads"] + COLS, rows)


PANEL_COLS = ["body_axial", "body_circ", "fin_chord", "fin_span", "panels", "wake_panels", "subsonic_s"] + COLS
PANEL_TITLE = "Panel table time against the panel mesh (4 threads); subsonic_s = the six subsonic solves"


def panel(S: Path):
    """Meshes already in panel_mesh.csv are kept; only the missing ones are measured (the sweep takes about 40 minutes)."""
    path = DATA / "panel_mesh.csv"
    rows = []
    if path.exists():
        old = list(csv.reader(l for l in path.read_text().splitlines() if not l.startswith("#")))
        rows = [[float(v) if "." in v else int(v) for v in r] for r in old[1:]]
    have = {tuple(r[:4]) for r in rows}
    # (body_axial, body_circ, fin_chord, fin_span); (60, 32, 16, 10), (80, 48, 16, 10) and (120, 64, 16, 10) are the meshes of the panel chapter
    for ba, bc, fc, fs in ((40, 24, 8, 5), (60, 36, 12, 7), (60, 32, 16, 10), (80, 32, 16, 10), (80, 48, 16, 10), (100, 60, 20, 12), (110, 72, 24, 14), (120, 64, 16, 10)):
        if (ba, bc, fc, fs) in have:
            continue
        sets = ["resources.threads=4", "aero.method=panel", f"aero.panel.body_axial={ba}", f"aero.panel.body_circ={bc}", f"aero.panel.fin_chord={fc}", f"aero.panel.fin_span={fs}"]
        out = S / "o_panel_n"
        m = median_run(["aero", SAMPLE, "--force", "--out", out], S, sets, grab=r"subsonic solves ([\d.]+) s")
        mm = re.search(r"Panel method: (\d+) panels \+ (\d+) wake panels", m["text"])
        rows.append([ba, bc, fc, fs, int(mm.group(1)), int(mm.group(2)), m["grab"]] + fmt(m))
        rows.sort(key=lambda r: r[4])
        write("panel_mesh.csv", PANEL_TITLE, PANEL_COLS, rows)


def grid(S: Path):
    rows = []
    for ms, a_s in ((0.02, 1.0), (0.01, 1.0), (0.01, 0.5), (0.005, 0.5), (0.005, 0.25), (0.0025, 0.25)):
        out = S / "o_grid_t"
        sets = ["resources.threads=4", f"aero.mach_step={ms}", f"aero.alpha_step_deg={a_s}"]
        m = median_run(["aero", SAMPLE, "--force", "--out", out], S, sets)
        nm = round(3.0 / ms) + 1
        na = round(90.0 / a_s) + 1
        rows.append([ms, a_s, nm, na, nm * na, (out / "aero_table.csv").stat().st_size / 1e6] + fmt(m))
        write("table_grid.csv", "Barrowman table time and file size against the Mach x alpha grid (4 threads)", ["mach_step", "alpha_step_deg", "n_mach", "n_alpha", "points", "csv_mb"] + COLS, rows)
    out = S / "o_grid_gen"
    m = median_run(["aero", SAMPLE, "--force", "--out", out], S, ["resources.threads=4", "aero.table_kind=\"general\""])
    rows.append(["general", "-", "-", "-", "-", (out / "aero_table.csv").stat().st_size / 1e6] + fmt(m))
    write("table_grid.csv", "Barrowman table time and file size against the Mach x alpha grid (4 threads)", ["mach_step", "alpha_step_deg", "n_mach", "n_alpha", "points", "csv_mb"] + COLS, rows)


def disp(S: Path):
    bar = S / "o_bar"
    if not (bar / "aero_table.csv").exists():
        run(["aero", SAMPLE, "--force", "--out", bar], S, ["resources.threads=4"])
    rows = []
    T = ["resources.threads=4", "output.kml=false", "sim.integrator=rk45"]
    for d in (8, 16, 32, 64, 128, 360):
        out = S / "o_gd"
        m = median_run(["dispersion", SAMPLE, "--aero-table", bar, "--out", out], S, T + [f"dispersion.directions={d}"])
        rows.append(["wind_grid", 7 * d * 2] + fmt(m) + [sum(f.stat().st_size for f in out.glob("dispersion*")) / 1e6])
        write(f"dispersion_scaling_{TAG}.csv" if TAG else "dispersion_scaling.csv", "Dispersion time against the number of flights (rk45, 4 threads)", ["mode", "flights"] + COLS + ["out_mb"], rows)
    for n in (250, 500, 1000, 2000, 4000, 10000, 20000):
        out = S / "o_mc2"
        m = median_run(["dispersion", SAMPLE, "--aero-table", bar, "--out", out], S, T + ["dispersion.mode=monte_carlo", f"dispersion.monte_carlo.samples={n}"])
        rows.append(["monte_carlo", n * 2] + fmt(m) + [sum(f.stat().st_size for f in out.glob("dispersion*")) / 1e6])
        write(f"dispersion_scaling_{TAG}.csv" if TAG else "dispersion_scaling.csv", "Dispersion time against the number of flights (rk45, 4 threads)", ["mode", "flights"] + COLS + ["out_mb"], rows)
    out = S / "o_gd4"
    for ig in ("rk4", "dop853"):
        m = median_run(["dispersion", SAMPLE, "--aero-table", bar, "--out", out], S, ["resources.threads=4", "output.kml=false", f"sim.integrator={ig}"])
        rows.append([f"wind_grid_{ig}", 112] + fmt(m) + [sum(f.stat().st_size for f in out.glob("dispersion*")) / 1e6])
    write(f"dispersion_scaling_{TAG}.csv" if TAG else "dispersion_scaling.csv", "Dispersion time against the number of flights (rk45, 4 threads)", ["mode", "flights"] + COLS + ["out_mb"], rows)


def sim(S: Path):
    bar = S / "o_bar"
    if not (bar / "aero_table.csv").exists():
        run(["aero", SAMPLE, "--force", "--out", bar], S, ["resources.threads=4"])
    rows = []
    cases = [("rk4", f"sim.dt={dt}", "sim.output_interval=0.05") for dt in (0.001, 0.002, 0.005, 0.01)]
    cases += [("rk45", "sim.dt=0.002", "sim.output_interval=0.05"), ("rk45", "sim.dt=0.002", "sim.output_interval=1.0"), ("dop853", "sim.dt=0.002", "sim.output_interval=0.05")]
    for ig, dt, oi in cases:
        out = S / "o_sim_x"
        m = median_run(["sim", SAMPLE, "--aero-table", bar, "--out", out], S, ["resources.threads=4", f"sim.integrator={ig}", dt, oi])
        s = json.loads((out / "summary.json").read_text())
        rows.append([ig, dt.split("=")[1], oi.split("=")[1], (out / "trajectory.csv").stat().st_size / 1e6, s["apogee"]] + fmt(m))
        write("sim_cost.csv", "One flight against the integrator, dt and output_interval (4 threads)", ["integrator", "dt", "output_interval", "trajectory_mb", "apogee_m"] + COLS, rows)


def example(S: Path):
    """The estimate of the chapter against a run: panel table, a 36 x 6 wind grid and 10^4 Monte Carlo samples (rk45, 4 threads)."""
    T = ["resources.threads=4", "output.kml=false", "sim.integrator=rk45"]
    pan = S / "o_ex_pan"
    rows = []
    m = median_run(["aero", SAMPLE, "--force", "--out", pan], S, ["resources.threads=4", "aero.method=panel"])
    rows.append(["panel_table", 0] + fmt(m))
    m = median_run(["dispersion", SAMPLE, "--aero-table", pan, "--out", S / "o_ex_g"], S, T + ["aero.method=panel", "dispersion.directions=36", "dispersion.wind_speeds=[1.0, 2.0, 3.0, 4.0, 5.0, 6.0]"])
    rows.append(["wind_grid_36x6", 36 * 6 * 2] + fmt(m))
    m = median_run(["dispersion", SAMPLE, "--aero-table", pan, "--out", S / "o_ex_m"], S, T + ["aero.method=panel", "dispersion.mode=monte_carlo", "dispersion.monte_carlo.samples=10000"])
    rows.append(["monte_carlo_10000", 10000 * 2] + fmt(m))
    write("example_run.csv", "Panel table, 36 x 6 wind grid and 10^4 Monte Carlo samples, one after the other (rk45, 4 threads)", ["step", "flights"] + COLS, rows)


def cad(S: Path):
    """CAD input (geometry from the STEP file, cold and cached tessellation) and CFD mesh generation, one run each.

    Works on a copy of the example files below SCRATCH_DIR/cadcfg, so that the repository's examples/ stays untouched."""
    ex = ROOT / "examples"
    cfg = fresh(S / "cadcfg")
    for n in ("sample_rocket.step", "sample_rocket.stl", "sample_motor.eng", "sample_cfd.toml", "sample_step.toml", "wind_profile_example.csv"):
        shutil.copy(ex / n, cfg / n)
    text = (ex / "sample.toml").read_text()
    text = text.replace('stl = "sample_rocket.stl"', 'cad = "sample_rocket.step"').replace("stl_scale = 0.001", "")
    (cfg / "sample_cad.toml").write_text(text)
    rows = []
    for label in ("geom_from_cad_cold", "geom_from_cad_cached"):
        r = run(["geom", cfg / "sample_cad.toml", "--out", S / "o_cad"], S, ["resources.threads=4"])
        rows.append([label, r["wall"], r["cpu"], r["rss_mb"], r["load"]])
    for label, name in (("cfd_mesh_euler_sample_cfd_toml", "sample_cfd.toml"), ("cfd_mesh_step_sample_step_toml", "sample_step.toml")):
        r = run(["cfd-check", cfg / name, "--mesh"], S, [])
        rows.append([label, r["wall"], r["cpu"], r["rss_mb"], r["load"]])
        print(r["text"][-700:], flush=True)
    write("cad_mesh.csv", "CAD geometry (cold and cached tessellation) and CFD mesh generation (gmsh, one thread), one run each", ["step", "wall_s", "cpu_s", "rss_mb", "load"], rows)


def plots(S: Path):
    """python/plot.py all on the result files of one run (needs matplotlib and pandas; PLOT_PYTHON selects the interpreter)."""
    full = S / "o_full"
    py = os.environ.get("PLOT_PYTHON", str(ROOT / "doc" / ".venv" / "bin" / "python"))
    if not (full / "dispersion_mc.csv").exists():
        stages_prepare(S)
    rows = []
    for sub in ("trajectory", "dispersion", "aero", "geometry", "all"):
        ws, cs = [], []
        for _ in range(REPEATS):
            cmd = ["/usr/bin/time", "-f", "%e %U %S %M", "-o", str(S / "time.txt"), py, str(ROOT / "python" / "plot.py"), sub, str(full)]
            ld = load1()
            p = subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True)
            tl = (S / "time.txt").read_text().strip().splitlines()[-1].split()
            ws.append(float(tl[0]))
            cs.append(float(tl[1]) + float(tl[2]))
            if p.returncode != 0:
                raise SystemExit(f"plot {sub} failed: {p.stderr[-500:]}")
        rows.append([sub, statistics.median(ws), min(ws), max(ws), statistics.median(cs), ld])
    write("plots.csv", "python/plot.py on the result files of one run", ["subcommand", "wall_s", "wall_min_s", "wall_max_s", "cpu_s", "load"], rows)


def stages_prepare(S: Path):
    T = ["resources.threads=4"]
    full = S / "o_full"
    run(["aero", SAMPLE, "--force", "--out", full], S, T)
    run(["sim", SAMPLE, "--aero-table", full, "--out", full], S, T)
    run(["dispersion", SAMPLE, "--aero-table", full, "--out", full], S, T + ["output.kml=false", "sim.integrator=rk45"])
    run(["dispersion", SAMPLE, "--aero-table", full, "--out", full], S, T + ["output.kml=false", "sim.integrator=rk45", "dispersion.mode=monte_carlo"])


EXP = dict(stages=stages, threads=threads, panel=panel, grid=grid, disp=disp, sim=sim, example=example, cad=cad, plots=plots)

if __name__ == "__main__":
    scratch = Path(sys.argv[1]).resolve()
    scratch.mkdir(parents=True, exist_ok=True)
    names = list(EXP) if sys.argv[2:] == ["all"] else sys.argv[2:]
    for n in names:
        EXP[n](scratch)
