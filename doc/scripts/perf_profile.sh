#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
# Profiles the ignisyeet stages on examples/sample.toml (doc/data/perf/profile_*.md).
#
# Usage: doc/scripts/perf_profile.sh SCRATCH_DIR [THREADS]
#
# Builds a throwaway copy of the CLI under SCRATCH_DIR/profcli (the sources of crates/cli with an
# in-process sampling profiler, pprof-rs at 997 Hz, around main and timestamps of the panel-method
# stages; the library crates are used unchanged by path), a phase harness for one subsonic panel solve
# (assembly / LU / triangular solves) and the supersonic primitives, then times and samples every stage.
# Nothing in the repository is modified; outputs go to SCRATCH_DIR/runs. Needs network access for
# the pprof crate on the first build. `perf` is not required (it is often blocked by
# kernel.perf_event_paranoid).
#
# PPROF_OUT=<prefix> makes the profiling binary write <prefix>.folded (folded stacks, one line per
# stack with its sample count); the summary at the end lists self and inclusive shares.
set -euo pipefail

REPO=$(cd "$(dirname "$0")/../.." && pwd)
SCR=$(mkdir -p "${1:?usage: perf_profile.sh SCRATCH_DIR [THREADS]}" && cd "$1" && pwd)
T=${2:-4}
PY=${PYTHON:-python3}
PLOT_PY=${PLOT_PYTHON:-$REPO/python/.venv/bin/python}
export RAYON_NUM_THREADS=$T

# ---------------------------------------------------------------- profiling copy of the CLI
P=$SCR/profcli
rm -rf "$P" && mkdir -p "$P/src/bin"
cp -r "$REPO/crates/cli/src/." "$P/src/"
cp "$REPO/Cargo.lock" "$P/"
cat > "$P/Cargo.toml" <<EOF
[package]
name = "profcli"
version = "0.0.0"
edition = "2021"
[[bin]]
name = "ignisyeet"
path = "src/main.rs"
[dependencies]
anyhow = "1"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
toml = "0.8"
clap = { version = "4", features = ["derive"] }
rayon = "1"
geom = { path = "$REPO/crates/geom" }
aero = { path = "$REPO/crates/aero" }
sim = { path = "$REPO/crates/sim" }
panel = { path = "$REPO/crates/panel" }
cfd = { path = "$REPO/crates/cfd" }
indicatif = "0.17"
console = "0.15"
libc = "0.2"
pprof = "0.15"
faer = { version = "0.24", default-features = false, features = ["std", "rayon"] }
[profile.release]
overflow-checks = true
opt-level = 3
debug = true
[workspace]
EOF
"$PY" - "$P/src/main.rs" <<'EOF'
import sys
p = sys.argv[1]
s = open(p).read()
def sub(old, new, s, start=0):
    i = s.index(old, start)
    return s[:i] + new + s[i + len(old):]
s = sub("fn main() -> Result<()> {\n    let r = run();", """fn main() -> Result<()> {
    let guard = std::env::var("PPROF_OUT").ok().map(|o| (o, pprof::ProfilerGuardBuilder::default().frequency(997).blocklist(&["libc", "libgcc", "pthread", "vdso"]).build().unwrap()));
    let t0 = std::time::Instant::now();
    let r = run();
    eprintln!("PROF wall {:.3} s", t0.elapsed().as_secs_f64());
    if let Some((o, g)) = guard {
        if let Ok(rep) = g.report().build() {
            use std::io::Write;
            let mut f = std::fs::File::create(format!("{o}.folded")).unwrap();
            for (frames, count) in rep.data.iter() {
                let mut names: Vec<String> = Vec::new();
                for fr in frames.frames.iter().rev() { for s in fr.iter().rev() { names.push(s.name().replace(';', ":")); } }
                writeln!(f, "{};{} {}", frames.thread_name.replace(';', ":"), names.join(";"), count).unwrap();
            }
        }
    }""", s)
# exit status 3 would skip the profile dump
s = s.replace("std::process::exit(EXIT_INCOMPLETE);", 'eprintln!("PROF (exit status {EXIT_INCOMPLETE} suppressed)"); return Ok(());')
i = s.index("fn panel_table(")
s = sub("    let progress = |st: St| match st {", """    let t_ph = std::time::Instant::now();
    let progress = |st: St| { if !matches!(st, St::Columns{..}) || matches!(st, St::Columns{done, total} if done == 1 || done == total) { eprintln!("PHASE {:9.3} {:?}", t_ph.elapsed().as_secs_f64(), st); } match st {""", s, i)
s = sub("        St::Done => pb.inc(1),\n    };", "        St::Done => pb.inc(1),\n    }};", s, i)
s = sub("    table.save(path)?;", '    eprintln!("PHASE {:9.3} lag terms filled", t_ph.elapsed().as_secs_f64());\n    table.save(path)?;\n    eprintln!("PHASE {:9.3} table saved", t_ph.elapsed().as_secs_f64());', s, i)
open(p, "w").write(s)
EOF
cat > "$P/src/bin/panel_lu.rs" <<'EOF'
// Phase split of one subsonic panel solve and the cost of the supersonic primitives.
use faer::dyn_stack::{MemBuffer, MemStack};
use faer::linalg::lu::partial_pivoting::factor::{lu_in_place, lu_in_place_scratch};
use faer::linalg::lu::partial_pivoting::solve::{solve_transpose_in_place_scratch, solve_transpose_in_place_with_conj};
use faer::{Conj, Mat, MatMut};
use panel::influence::{doublet, influence};
use rayon::prelude::*;
use std::time::Instant;

fn main() {
    let stl = std::env::args().nth(1).unwrap();
    let threads: usize = std::env::args().nth(2).map(|s| s.parse().unwrap()).unwrap_or(4);
    rayon::ThreadPoolBuilder::new().num_threads(threads).build_global().unwrap();
    panel::set_parallelism(threads);
    let tris: Vec<_> = geom::stl::read_stl(std::path::Path::new(&stl)).unwrap().into_iter().map(|t| t.map(|v| v * 0.001)).collect();
    let g = geom::extract(&tris, &geom::ExtractOptions::default()).unwrap();
    let t = Instant::now();
    let mesh = panel::mesh::Mesh::from_geometry(&g, &panel::PanelOptions::default()).unwrap();
    println!("threads {threads}; mesh {:.3} s, {} panels", t.elapsed().as_secs_f64(), mesh.panels.len());
    let n = mesh.panels.len();
    for rep in 0..2 {
        let t = Instant::now();
        let mut a = vec![0.0f64; n * n];
        let mut rhs = vec![[0.0f64; 3]; n];
        a.par_chunks_mut(n).zip(rhs.par_iter_mut()).enumerate().for_each(|(i, (row, rh))| {
            let xi = mesh.panels[i].centroid;
            let mut b = [0.0; 3];
            for (j, pj) in mesh.panels.iter().enumerate() {
                let (s, d) = influence(pj, xi);
                row[j] = if i == j { -0.5 } else { d };
                b[0] -= s * pj.normal.x;
                b[1] -= s * pj.normal.z;
                b[2] -= s * pj.normal.z * pj.centroid.x;
            }
            for w in &mesh.wake_cols {
                let wi: f64 = mesh.wake_panels[w.panels.clone()].iter().map(|wp| doublet(wp, xi)).sum();
                row[w.upper] += wi;
                if let Some(l) = w.lower {
                    row[l] -= wi;
                }
            }
            *rh = b;
        });
        let t_asm = t.elapsed().as_secs_f64();
        let t = Instant::now();
        let par = faer::get_global_parallelism();
        let (mut pf, mut pb) = (vec![0usize; n], vec![0usize; n]);
        let mut sol = Mat::<f64>::from_fn(n, 3, |i, k| rhs[i][k]);
        let mut b = MatMut::from_column_major_slice_mut(&mut a, n, n);
        let (_, perm) = lu_in_place(b.as_mut(), &mut pf, &mut pb, par, MemStack::new(&mut MemBuffer::new(lu_in_place_scratch::<usize, f64>(n, n, par, Default::default()))), Default::default());
        let t_lu = t.elapsed().as_secs_f64();
        let t = Instant::now();
        solve_transpose_in_place_with_conj(b.as_ref(), b.as_ref(), perm, Conj::No, sol.as_mut(), par, MemStack::new(&mut MemBuffer::new(solve_transpose_in_place_scratch::<usize, f64>(n, 3, par))));
        let t_sv = t.elapsed().as_secs_f64();
        println!("rep {rep}: assembly {t_asm:.3} s, LU {t_lu:.3} s ({:.1} GFLOP/s), triangular solves {t_sv:.3} s", 2.0 / 3.0 * (n as f64).powi(3) / t_lu / 1e9);
    }
    let t = Instant::now();
    let _ = panel::solver::solve_potential(&mesh).unwrap();
    println!("full solve_potential (incl. surface gradient) {:.3} s", t.elapsed().as_secs_f64());
    let tip = g.fins.as_ref().map(|f| (f.x_le_root + f.sweep, f.body_radius + f.span));
    let root = g.fins.as_ref().map(|f| (f.body_radius, f.x_le_root + f.root_chord));
    let sup = panel::supersonic::SupersonicPanels::new(&mesh, g.ref_area, tip, root);
    for &m in &[1.2, 2.0, 3.0] {
        let t = Instant::now();
        let s: f64 = (0..=90).map(|k| sup.forces(m, (k as f64).to_radians(), 0.0).fz).sum();
        println!("M {m}: 91 x SupersonicPanels::forces, one thread {:.3} s ({s:.3})", t.elapsed().as_secs_f64());
    }
    let n = 200_000;
    let d = |i: usize| 0.3 * (i as f64 + 1.0) / n as f64;
    let fs: [(&str, Box<dyn Fn(usize) -> f64>); 4] = [
        ("expansion_cp", Box::new(move |i| panel::supersonic::expansion_cp(2.0, d(i)))),
        ("wedge_cp", Box::new(move |i| panel::supersonic::wedge_cp(2.0, d(i)).unwrap_or(0.0))),
        ("cone_cp", Box::new(move |i| panel::supersonic::cone_cp(2.0, d(i)).unwrap_or(0.0))),
        ("prandtl_meyer", Box::new(move |i| panel::supersonic::prandtl_meyer(1.0 + 10.0 * d(i)))),
    ];
    for (name, f) in fs {
        let t = Instant::now();
        let s: f64 = (0..n).map(&f).sum();
        println!("{name}: {:.0} ns/call ({s:.3})", t.elapsed().as_secs_f64() / n as f64 * 1e9);
    }
}
EOF
(cd "$P" && CARGO_TARGET_DIR=$SCR/target cargo build --release -j "$T" --bins)
B=$SCR/target/release/ignisyeet
H=$SCR/target/release/panel_lu

# ---------------------------------------------------------------- folded-stack summary
summary() {
  "$PY" - "$1" "${2:-20}" <<'EOF'
import collections, re, sys
clean = lambda n: re.sub(r'::h[0-9a-f]{16}$', '', n)
own, incl, total = collections.Counter(), collections.Counter(), 0
for line in open(sys.argv[1]):
    stack, c = line.rstrip('\n').rsplit(' ', 1)
    c = int(c); total += c
    fr = [clean(f) for f in stack.split(';')[1:]]
    own[fr[-1] if fr else '(no frames: unwinding failed, e.g. assembly GEMM kernels)'] += c
    for f in set(fr):
        incl[f] += c
top = int(sys.argv[2])
print(f"  samples {total}; self:")
for f, c in own.most_common(top):
    print(f"    {100 * c / total:5.1f}%  {f[:140]}")
print("  inclusive (ignisyeet crates):")
mine = [(f, c) for f, c in incl.items() if re.search(r'(^|[<\s])(geom|aero|sim|panel|cfd|ignisyeet)::', f)]
for f, c in sorted(mine, key=lambda x: -x[1])[:top]:
    print(f"    {100 * c / total:5.1f}%  {f[:140]}")
EOF
}

R=$SCR/runs
mkdir -p "$R"
cd "$REPO"
C=examples/sample.toml
TS=(--set resources.threads="$T")
run() { # name, args...
  local name=$1; shift
  echo "=== $name"
  PPROF_OUT=$R/$name /usr/bin/time -f "  elapsed %e s, user %U s, max RSS %M kB" "$B" --no-progress "$@" 2>&1 | grep -E "PROF|PHASE|elapsed|Aero table|Dispersion:|Monte Carlo:|subsonic solves|apogee|landing" || true
  summary "$R/$name.folded" 15
}

run geom geom $C "${TS[@]}" --out "$R/o_bar"
run aero_barrowman aero $C --force "${TS[@]}" --out "$R/o_bar"
run aero_panel aero $C --force "${TS[@]}" --set aero.method=panel --out "$R/o_panel"
for ig in rk4 rk45 dop853; do
  run "sim_$ig" sim $C "${TS[@]}" --set sim.integrator=$ig --aero-table "$R/o_bar" --out "$R/o_sim_$ig"
done
for ig in rk4 rk45 dop853; do
  run "grid_$ig" dispersion $C "${TS[@]}" --set output.kml=false --set sim.integrator=$ig --aero-table "$R/o_bar" --out "$R/o_grid_$ig"
done
run mc_rk45 dispersion $C "${TS[@]}" --set output.kml=false --set sim.integrator=rk45 --set dispersion.mode=monte_carlo --aero-table "$R/o_bar" --out "$R/o_mc"

echo "=== thread scaling (no profiler)"
for t in 1 2 "$T"; do
  for mode in wind_grid monte_carlo; do
    ig=rk4; [ $mode = monte_carlo ] && ig=rk45
    /usr/bin/time -f "  $mode $ig threads $t: elapsed %e s, user %U s" "$B" --no-progress dispersion $C --set resources.threads=$t --set output.kml=false \
      --set sim.integrator=$ig --set dispersion.mode=$mode --aero-table "$R/o_bar" --out "$R/o_scale_${mode}_$t" 2>&1 | grep elapsed
  done
  "$B" --no-progress aero $C --force --set resources.threads=$t --set aero.method=panel --out "$R/o_panel_$t" 2>&1 | grep -E "PHASE .*(SubsonicDone \{ index: 5|Transonic|Columns \{ done: [0-9]+, total|table saved)" | sed "s/^/  panel threads $t: /"
done

echo "=== panel phase harness"
for t in 1 "$T"; do "$H" examples/sample_rocket.stl "$t"; done

echo "=== python/plot.py all"
if [ -x "$PLOT_PY" ]; then
  D=$R/plotdir
  rm -rf "$D" && cp -r "$R/o_sim_rk4" "$D" && cp "$R"/o_grid_rk4/dispersion* "$R"/o_mc/dispersion_mc.csv "$R"/o_mc/dispersion_summary.json "$R"/o_panel/panel_cp.csv "$D/"
  for w in geometry aero panel trajectory dispersion all; do
    /usr/bin/time -f "  plot $w: %e s" "$PLOT_PY" python/plot.py $w "$D" 2>&1 >/dev/null | tail -1
  done
else
  echo "  skipped: $PLOT_PY not found (set PLOT_PYTHON)"
fi
