#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
"""RANS verification runs through the ignisyeet CLI (`ignisyeet aero` with [aero] method = "cfd").

  setup DIR [options]   write DIR/case.toml: examples/sample_cfd.toml with the given [aero.cfd] overrides, the output
                        directory DIR/out and a [resources] section (threads, memory)
  run DIR               run `ignisyeet --no-progress aero --force DIR/case.toml` while sampling the resident set size
                        of every SU2_CFD process below DIR once per second (DIR/rss.json: per case directory the peak
                        of the largest rank and of the sum of its ranks) and record wall time, versions and the git
                        commit (DIR/run.json)
  collect DIR...        one CSV row per solved case (stdout): status, iterations, residual drops, y+ P95, wall
                        time, CN, CA with its wall / base split, xcp, peak RSS; mirror-combined rows for alpha > 0

The geometry: the sample STL (default), or --stl FILE --stl-scale S --cg C (e.g. the body or cone of
rans_geometry.py).
"""
import argparse
import csv
import json
import os
import re
import subprocess
import sys
import threading
import time

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import rans_replay  # noqa: E402

REPO = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
PREFIX = os.environ.get("IGNISYEET_CFD_PREFIX", os.path.expanduser("~/.local/share/micromamba/envs/ignisyeet-cfd"))


def toml_value(v):
    if isinstance(v, str):
        return json.dumps(v)
    if isinstance(v, bool):
        return "true" if v else "false"
    if isinstance(v, (list, tuple)):
        return "[" + ", ".join(toml_value(x) for x in v) + "]"
    return repr(v)


def setup(a):
    os.makedirs(a.dir, exist_ok=True)
    src = open(os.path.join(REPO, "examples", "sample_cfd.toml")).read()
    stl = os.path.abspath(a.stl) if a.stl else os.path.join(REPO, "examples", "sample_rocket.stl")
    sets = {
        ("output", "dir"): os.path.join(os.path.abspath(a.dir), "out"),
        ("output", "kml"): False,
        ("rocket", "stl"): stl,
        ("rocket", "stl_scale"): a.stl_scale,
        ("motor", "eng"): os.path.join(REPO, "examples", "sample_motor.eng"),
        ("aero.cfd", "model"): a.model,
        ("aero.cfd", "machs"): a.machs,
        ("aero.cfd", "alphas_deg"): a.alphas,
        ("aero.cfd", "wall_size"): a.wall,
        ("aero.cfd", "ranks_per_case"): a.ranks,
        ("aero.cfd", "parallel_cases"): a.parallel,
    }
    if a.cg is not None:
        sets[("rocket", "cg_dry")] = a.cg
    if a.nose:
        sets[("rocket", "nose_direction")] = a.nose
    for kv in a.set or []:
        k, v = kv.split("=", 1)
        sec, key = k.rsplit(".", 1)
        sets[(sec, key)] = json.loads(v)
    lines, section, seen = [], None, set()
    for line in src.splitlines():
        m = re.match(r"^\[([^\]]+)\]", line)
        if m:
            # keys of the previous section that the file does not contain (e.g. commented out)
            lines += [f"{k} = {toml_value(v)}" for (s, k), v in sets.items() if s == section and (s, k) not in seen]
            section = m.group(1)
        m = re.match(r"^\s*#?\s*([A-Za-z_]+)\s*=", line)
        if section == "aero.cfd" and re.match(r"^iterations\s*=", line) and ("aero.cfd", "iterations") not in sets:
            lines.append("# " + line + "  (left to the model default)")
            continue
        if m and (section, m.group(1)) in sets and (section, m.group(1)) not in seen:
            k = m.group(1)
            lines.append(f"{k} = {toml_value(sets[(section, k)])}")
            seen.add((section, k))
            continue
        lines.append(line)
    lines += [f"{k} = {toml_value(v)}" for (s, k), v in sets.items() if s == section and (s, k) not in seen]
    lines += ["", "[resources]", f"threads = {a.threads}", f"memory_gb = {a.memory_gb}"]
    open(os.path.join(a.dir, "case.toml"), "w").write("\n".join(lines) + "\n")
    print(os.path.join(a.dir, "case.toml"))


def rss_sampler(root, stop, peaks):
    """Per SU2 case directory below root: peak RSS of the largest rank and of the sum of ranks [MB]."""
    while not stop.is_set():
        now = {}
        for pid in os.listdir("/proc"):
            if not pid.isdigit():
                continue
            try:
                if open(f"/proc/{pid}/comm").read().strip() != "SU2_CFD":
                    continue
                cwd = os.readlink(f"/proc/{pid}/cwd")
                if not cwd.startswith(root):
                    continue
                rss = 0
                for line in open(f"/proc/{pid}/status"):
                    if line.startswith("VmRSS:"):
                        rss = int(line.split()[1]) / 1024.0
                now.setdefault(cwd, []).append(rss)
            except OSError:
                continue
        for cwd, v in now.items():
            p = peaks.setdefault(cwd, {"rank_max_mb": 0.0, "sum_max_mb": 0.0, "ranks": 0})
            p["rank_max_mb"] = max(p["rank_max_mb"], max(v))
            p["sum_max_mb"] = max(p["sum_max_mb"], sum(v))
            p["ranks"] = max(p["ranks"], len(v))
        stop.wait(1.0)


def run(a):
    d = os.path.abspath(a.dir)
    exe = a.exe
    peaks, stop = {}, threading.Event()
    th = threading.Thread(target=rss_sampler, args=(d, stop, peaks), daemon=True)
    th.start()
    t0 = time.time()
    env = dict(os.environ, IGNISYEET_CFD_PREFIX=PREFIX)
    with open(os.path.join(d, "aero.log"), "w") as log:
        rc = subprocess.run([exe, "--no-progress", "aero", "--force", os.path.join(d, "case.toml")], stdout=log,
                            stderr=subprocess.STDOUT, env=env).returncode
    wall = time.time() - t0
    stop.set()
    th.join()
    json.dump(peaks, open(os.path.join(d, "rss.json"), "w"), indent=1)
    commit = subprocess.run(["git", "-C", REPO, "rev-parse", "--short", "HEAD"], capture_output=True, text=True).stdout.strip()
    meta = {"exit_code": rc, "wall_s": wall, "git_commit": commit, "date": time.strftime("%Y-%m-%d"),
            "ignisyeet": subprocess.run([exe, "--version"], capture_output=True, text=True).stdout.strip()}
    json.dump(meta, open(os.path.join(d, "run.json"), "w"), indent=1)
    print(json.dumps(meta))


def direct(a):
    """Run the two SU2 stages of a pipeline case (its case_startup.cfg / case.cfg) in a new directory, with the
    Mach number replaced and the final stage capped at --iterations (for steady-force studies of cases that the
    acceptance would keep running to the iteration limit). RSS of the ranks is sampled as in `run`."""
    src, d = os.path.abspath(a.src), os.path.abspath(a.dir)
    os.makedirs(d, exist_ok=True)
    peaks, stop = {}, threading.Event()
    th = threading.Thread(target=rss_sampler, args=(d, stop, peaks), daemon=True)
    th.start()
    t0 = time.time()
    for stage, cfgname, log in (("startup", "case_startup.cfg", "su2_startup.log"), ("final", "case.cfg", "su2.log")):
        text = open(os.path.join(src, cfgname)).read()
        if a.mach is not None:
            text = re.sub(r"^MACH_NUMBER= .*$", f"MACH_NUMBER= {a.mach:.6f}", text, flags=re.M)
        if stage == "final":
            text = re.sub(r"^ITER= .*$", f"ITER= {a.iterations}", text, flags=re.M)
            for kv in a.sub or []:  # KEY=VALUE: replace (or append) a line of the final-stage config
                k, v = kv.split("=", 1)
                if re.search(rf"^{k}= ", text, flags=re.M):
                    text = re.sub(rf"^{k}= .*$", f"{k}= {v}", text, flags=re.M)
                else:
                    text += f"{k}= {v}\n"
        text = text.replace("MESH_FILENAME= ", "MESH_FILENAME= ", 1)
        open(os.path.join(d, cfgname), "w").write(text)
        cmd = [os.path.join(PREFIX, "bin", "mpirun"), "-np", str(a.ranks), "--bind-to", "none",
               os.path.join(PREFIX, "bin", "SU2_CFD"), cfgname]
        with open(os.path.join(d, log), "w") as f:
            subprocess.run(cmd, cwd=d, stdout=f, stderr=subprocess.STDOUT, env=dict(os.environ, OMP_NUM_THREADS="1"))
    stop.set()
    th.join()
    json.dump({"wall_s": time.time() - t0, "ranks": a.ranks, "rss": peaks.get(d), "src": src, "mach": a.mach,
               "iterations": a.iterations}, open(os.path.join(d, "direct.json"), "w"), indent=1)


def history(path):
    with open(path) as f:
        rows = list(csv.reader(f))
    head = [h.strip().strip('"') for h in rows[0]]
    return head, [[float(v) for v in r] for r in rows[1:] if r]


def collect(a):
    rows = []
    for d in a.dirs:
        d = os.path.abspath(d)
        out = os.path.join(d, "out")
        rep = json.load(open(os.path.join(out, "cfd_report.json")))
        rss = json.load(open(os.path.join(d, "rss.json"))) if os.path.exists(os.path.join(d, "rss.json")) else {}
        run_meta = json.load(open(os.path.join(d, "run.json"))) if os.path.exists(os.path.join(d, "run.json")) else {}
        cfg = open(os.path.join(d, "case.toml")).read()
        wall = float(re.search(r"^wall_size = ([0-9.e-]+)", cfg, re.M).group(1))
        cases = list(csv.DictReader(open(os.path.join(out, "cfd_cases.csv"))))
        work = os.path.join(out, "cfd")
        for c in cases:
            r = {"run": os.path.basename(d), "wall_size_m": wall, "mesh_nodes": rep.get("mesh_nodes"),
                 "mesh_cells": rep.get("mesh_cells"), "mach": float(c["mach"]), "alpha_deg": float(c["alpha_deg"]),
                 "status": c["status"]}
            if c["status"] == "combined":
                r.update({"cn": c["cn"], "xcp_m": c["xcp_m"], "ca": c["ca_cfd"]})
                rows.append(r)
                continue
            name = f"m{float(c['mach']):.3f}_a{'+' if float(c['alpha_deg']) >= 0 else '-'}{abs(float(c['alpha_deg'])):05.2f}"
            cdir = os.path.join(work, name)
            if not os.path.isdir(cdir):
                cands = [x for x in os.listdir(work) if x.startswith(f"m{float(c['mach']):.3f}_a")]
                cdir = next((os.path.join(work, x) for x in cands if abs(float(x.split("_a")[1].split("_")[0]) - float(c["alpha_deg"])) < 1e-9), cdir)
            done = json.load(open(os.path.join(cdir, "done.json"))) if os.path.exists(os.path.join(cdir, "done.json")) else {}
            ref = done.get("reference_peak") or {}
            drop_rho = drop_nu = None
            if os.path.exists(os.path.join(cdir, "history.csv")):
                h, data = history(os.path.join(cdir, "history.csv"))
                if ref and data:
                    rho_pk = max(ref["rho"], max(x[h.index("rms[Rho]")] for x in data))
                    nu_pk = max(ref["nu"], max(x[h.index("rms[nu]")] for x in data))
                    drop_rho = rho_pk - data[-1][h.index("rms[Rho]")]
                    drop_nu = nu_pk - data[-1][h.index("rms[nu]")]
            stop = {}
            if os.path.exists(os.path.join(cdir, "history.csv")) and os.path.exists(os.path.join(cdir, "history_startup.csv")):
                hs, ds = history(os.path.join(cdir, "history_startup.csv"))
                rr = ref.get("rho", max(x[hs.index("rms[Rho]")] for x in ds))
                rn = ref.get("nu", max(x[hs.index("rms[nu]")] for x in ds))
                h, data = history(os.path.join(cdir, "history.csv"))
                old, new = rans_replay.stops(h, data, rr, rn, a.length_ratio, window=a.window, min_iter=a.min_iter)
                last = data[-1]
                cz, cw, my = h.index("CFz"), h.index("CFx(wall)"), h.index("CMy")
                xcp = lambda row: -row[my] / row[cz] * a.ref_length if abs(row[cz]) > 1e-3 else float("nan")
                for tag, it in (("old", old), ("new", new)):
                    stop[f"{tag}_rule_stop_iter"] = it
                    if it:
                        row = data[it - 1]
                        stop[f"{tag}_dcn_raw"] = row[cz] - last[cz]
                        stop[f"{tag}_dca_wall_percent"] = 100 * (row[cw] / last[cw] - 1)
                        stop[f"{tag}_dxcp_m"] = xcp(row) - xcp(last)
                stop["final_cfz"], stop["final_cfx_wall"], stop["final_iter"] = last[cz], last[cw], len(data)
            ax = done.get("axial") or {}
            pk = rss.get(cdir, {})
            r.update({
                "iterations": c["iterations"], "rho_drop_orders": drop_rho, "nu_drop_orders": drop_nu,
                "yplus_p95": done.get("yplus_p95"), "case_wall_s": c["wall_seconds"],
                "cn_raw": c["cn_raw"], "xcp_raw_m": c["xcp_raw_m"], "ca": c["ca_cfd"],
                "ca_wall": ax.get("wall_cfx"), "ca_base": ax.get("base_cfx"),
                "rank_peak_rss_mb": pk.get("rank_max_mb"), "case_peak_rss_mb": pk.get("sum_max_mb"),
                "ranks": pk.get("ranks"), "warnings": " | ".join(done.get("warnings", [])),
            })
            r.update(stop)
            rows.append(r)
    keys = []
    for r in rows:
        keys += [k for k in r if k not in keys]
    w = csv.DictWriter(sys.stdout, fieldnames=keys)
    w.writeheader()
    for r in rows:
        w.writerow({k: (f"{v:.6g}" if isinstance(v, float) else v) for k, v in r.items()})


def main():
    p = argparse.ArgumentParser()
    sub = p.add_subparsers(dest="cmd", required=True)
    s = sub.add_parser("setup")
    s.add_argument("dir")
    s.add_argument("--model", default="rans")
    s.add_argument("--machs", type=float, nargs="+", default=[0.8, 2.0])
    s.add_argument("--alphas", type=float, nargs="+", default=[0.0, 4.0])
    s.add_argument("--wall", type=float, default=0.003)
    s.add_argument("--ranks", type=int, default=4)
    s.add_argument("--parallel", type=int, default=3)
    s.add_argument("--threads", type=int, default=12)
    s.add_argument("--memory-gb", type=float, default=20.0)
    s.add_argument("--stl")
    s.add_argument("--stl-scale", type=float, default=0.001)
    s.add_argument("--cg", type=float)
    s.add_argument("--nose")
    s.add_argument("--set", action="append", help='SECTION.KEY=JSON, e.g. aero.cfd.rans_base="cfd"')
    r = sub.add_parser("run")
    r.add_argument("dir")
    r.add_argument("--exe", default=os.path.join(REPO, "target", "release", "ignisyeet"))
    dr = sub.add_parser("direct")
    dr.add_argument("src")
    dr.add_argument("dir")
    dr.add_argument("--mach", type=float)
    dr.add_argument("--iterations", type=int, default=1500)
    dr.add_argument("--ranks", type=int, default=4)
    dr.add_argument("--sub", action="append", help="KEY=VALUE for the final-stage config, e.g. SLOPE_LIMITER_FLOW=VENKATAKRISHNAN_WANG")
    c = sub.add_parser("collect")
    c.add_argument("dirs", nargs="+")
    c.add_argument("--length-ratio", type=float, default=15.0, help="body length / reference length")
    c.add_argument("--ref-length", type=float, default=0.1)
    c.add_argument("--window", type=int, default=200)
    c.add_argument("--min-iter", type=int, default=400)
    a = p.parse_args()
    {"setup": setup, "run": run, "collect": collect, "direct": direct}[a.cmd](a)


if __name__ == "__main__":
    main()
