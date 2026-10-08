<div align="center">
<img src="doc/IgnisYeet-logo.svg" alt="IgnisYeet logo" width="40%" />

# IgnisYeet

**A rocket simulator that estimates aerodynamics from an STL file, flies the rocket in 6 degrees of freedom, and computes the landing dispersion**

[![Docs](https://github.com/ddd3h/ignisyeet/actions/workflows/docs.yml/badge.svg?branch=main)](https://github.com/ddd3h/ignisyeet/actions/workflows/docs.yml)
[![Documentation](https://img.shields.io/badge/docs-www.ddd3h.com%2Fignisyeet-1f6feb?logo=readthedocs&logoColor=white)](https://www.ddd3h.com/ignisyeet/)
[![License: GPL-3.0-or-later](https://img.shields.io/badge/license-GPL--3.0--or--later-blue.svg)](LICENSE)
[![Version](https://img.shields.io/badge/version-0.4.0-informational)](CITATION.cff)

[![Rust](https://img.shields.io/badge/Rust-2021_edition-000000?logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![Python](https://img.shields.io/badge/Python-3.10%2B-3776AB?logo=python&logoColor=white)](https://www.python.org/)
[![uv](https://img.shields.io/badge/uv-supported-DE5FE9?logo=uv&logoColor=white)](https://docs.astral.sh/uv/)
[![Sphinx](https://img.shields.io/badge/docs-Sphinx-0A507A?logo=sphinx&logoColor=white)](https://www.sphinx-doc.org/)
[![Platform](https://img.shields.io/badge/platform-Linux%20%7C%20macOS-FCC624?logo=linux&logoColor=black)](#installation)
[![Google Earth KML](https://img.shields.io/badge/output-KML-4285F4?logo=googleearth&logoColor=white)](#output-files)
[![Citation](https://img.shields.io/badge/cite-CITATION.cff-orange)](CITATION.cff)
</div>

Given a rocket **STL file**, mass properties, a motor thrust curve, and launch-site conditions, IgnisYeet runs the following pipeline:

1. Extracts the body radius distribution and the fin dimensions from the STL
2. Computes the aerodynamic coefficients once on a grid of Mach number and angle of attack (and sideslip for a six-component table), and stores them as a coefficient table
3. Flies the rocket in **6 degrees of freedom**, interpolating and extrapolating the coefficient table
4. Computes the **spread of landing points** with a grid of wind speeds and directions, or with Monte Carlo sampling
5. Plots the results and writes a KML file that can be viewed in Google Earth

The computationally heavy parts (mesh slicing, the aerodynamic coefficient table, the 6-DoF integration, and the parallel landing dispersion) are written in Rust. Plotting is written in Python (matplotlib).

日本語のドキュメント（式・図・検証）は [`doc/`](doc/) にあり、https://www.ddd3h.com/ignisyeet/ で公開されています。使い方は [Usage](https://www.ddd3h.com/ignisyeet/usage.html)、HTML/PDF のビルド方法は [Documentation](#documentation) を参照してください。
Detailed documentation with equations, figures, and verification is available in Japanese at <https://www.ddd3h.com/ignisyeet/> (sources in [`doc/`](doc/)). See [Usage](https://www.ddd3h.com/ignisyeet/usage.html) for usage and [Computation flow](https://www.ddd3h.com/ignisyeet/overview.html) for the computation flow. How to build the HTML and PDF versions is described in [Documentation](#documentation).

## Features

- **Geometry extraction from STL**: principal-axis detection, per-section radius distribution, and automatic estimation of the number and dimensions of fins (manual override is possible)
- **Three aerodynamic models**: Barrowman method (including Niskanen's supersonic and drag extensions), panel method (Morino method for subsonic, relaxed above 15 degrees to the Barrowman large-angle model with the fin crossflow, which it equals from 30 degrees; local-inclination method for supersonic; the nose moment is the complete r x F), and external coefficient tables (e.g. from CFD)
- **Six-component quasi-steady aerodynamics**:
  - axisymmetric table: CN, CA, center of pressure, damping sums, roll terms, apparent-mass lag sums, induced roll and Magnus force and moment slopes as a function of Mach number and total angle of attack (the lookup interpolates CN and the nose moment CN·xcp, and xcp is their ratio);
  - general table: CX, CY, CZ, Cl, Cm, Cn as a function of Mach number, angle of attack alpha and sideslip beta (from `aero.table_kind = "general"`, from CFD with `surface = "stl"` or `"step"` or sideslip cases, or as an external table);
  - roll from the fin cant forcing (`fin_cant_deg`, `fin_override.cant_deg`) and the roll damping Clp;
  - full 6-DoF rigid-body motion with quasi-steady aerodynamics (no flow-history states): diagonal rate damping (Clp, Cmq, Cnr), the rotation-induced normal force (CN_q), the slender-body apparent mass of the body and the fin sections (alpha-dot lag moment and convective rotation moment and force), a Magnus force and moment and induced roll (four fins); with the strip damping the sample's Cmq + Cmalphadot is within 7 % of the slender-body closed form, an internal consistency check of the rate and alpha-dot bookkeeping (both use the Barrowman normal-force slope, so it does not test the normalisation or the physical accuracy). The linear cross derivatives (Cmp, Cnq, Clq, Clr, ...) vanish for an axisymmetric body with cruciform fins and are not modelled. The Barrowman, panel and CFD tables carry the lag sums of the geometry but no Magnus or induced-roll values (zero unless an external table supplies them); `sim.quasi_steady = true` switches the rate and acceleration terms off for sensitivity checks;
  - a roll-resonance check: with a trim forcing (lateral CG offset, thrust misalignment, or an asymmetric general table; a symmetric fin cant only supplies the roll) a crossing of the roll rate and the resonance rate of the pitch oscillation during the ascent is reported in `summary.json` (`roll_resonance`) and as a warning
- **Static stability**: judged by the small-angle static margin (for an axisymmetric table the centre of pressure the interpolated table applies at small angles, taken over its first angle cell, unknown when the normal-force slope dCN/dalpha there is zero and unstable or unknown when it is negative; the separate pitch and yaw margins for a general table), with the time and Mach number where the smallest margin occurs; the angle of attack the rocket flies at only enters the `stability_cal` display column of `trajectory.csv`
- **CFD mode** (`aero.method = "cfd"`, optional; see [CFD](#cfd) and the [CFD chapter](https://www.ddd3h.com/ignisyeet/cfd.html)):
  - meshes the geometry with gmsh and solves a steady (Mach, angle of attack, optionally sideslip) matrix with SU2 (Euler by default, Roe/MUSCL, parallel MPI cases with warm starts, resume and an early stop on converged forces);
  - interpolates the solved points onto the same coefficient table, so the flight simulation is unchanged;
  - wall rebuilt from the extracted body and fins (`surface = "cad"`, alias `"panel_mesh"`), taken **directly from the STL** (`surface = "stl"`: repaired, united with Manifold, remeshed, with a fidelity report; close to, but not identical to, the input STL), or taken from the **CAD model** (`surface = "step"`: a STEP, IGES or BREP solid in `step_file` with every detail such as launch lugs and rail buttons, meshed on its exact surfaces and checked against `rocket.stl` before meshing: units, orientation and revision mismatches are reported);
  - half or quarter domains only after the mirror symmetry of the STL or CAD shape is verified; sideslip (`betas_deg`) and roll-damping (`roll_damping`) cases give a six-component table;
  - needs SU2 8.3.0, gmsh, Open MPI and, for `surface = "stl"`, manifold3d (check with `ignisyeet cfd-check <config>`); tens of minutes for a small matrix;
  - optional RANS (`model = "rans"`, Spalart-Allmaras) on a mesh with a prism boundary layer built on the wall, for the rebuilt geometry with a fin layout symmetric about the pitch plane and for the CAD model (RANS on a CAD model has been checked up to the mesh validity, not with SU2 solutions); verified for attached-flow friction and for the sample rocket at M = 0.8 and 2, alpha = ±4 deg, not for base flow or low subsonic speed (see the verification chapter before using its tables)
  - the CFD results are cross-checks against the other methods, not validation against measurements
- **Cached coefficient table**: not recomputed if the inputs are unchanged (the cache key includes the program version, a model version and the extracted geometry, and ignores settings that do not change the coefficients, such as unused method blocks and CFD resource and path settings); interpolated inside the range. Mach number is never extrapolated (a lookup beyond the Mach range is held at the table edge and makes the results unusable, exit status 3); angle of attack and sideslip beyond the table are extrapolated linearly (or clamped, `extrapolation = "clamp"`) with bounds that keep signs and ranges physical
- **6-DoF flight**: flat Earth (local ENU) and rotating Earth (ECEF, with Coriolis and centrifugal forces); constant, inverse-square, and J2 gravity; full inertia tensor (products of inertia), lateral CG offset, thrust misalignment and nozzle offset, and launch roll about the rail axis
- **Atmosphere and wind models**: US Standard Atmosphere 1976 (with optional temperature offset) and a constant atmosphere; constant, power-law, logarithmic, and altitude-table winds
- **Integrators**: fixed-step RK4 and adaptive-step RK45 (Dormand-Prince) and DOP853; attitude propagation by quaternion renormalization or a Lie-group (RKMK) update; the rail exit, the apogee (the zero of the interpolant's vertical velocity), the main deployment and the touchdown are located inside the step on the cubic Hermite interpolant with every integrator (free flight restarts at the rail exit; events within a step are handled in time order, and steps end exactly on thrust-curve knots, burnout, the deployment time and `sim.max_time`), and a diverged state (non-finite, above 8 km/s or 1000 km) stops the run with an error
- **Landing dispersion**:
  - a wind grid (speed x direction) and Monte Carlo sampling (1-sigma and 3-sigma error ellipses, P50/P90/P99 of the landing distance, and P99.9 from 1000 landed samples), in parallel, for both ballistic and parachute descent;
  - Monte Carlo perturbs thrust, burn time, dry mass, CG, CN, CA, the rail pointing (an isotropic tilt of the rail axis plus an azimuth setting error), wind and parachute, and optionally thrust misalignment, nozzle offset, fin cant, lateral CG, roll moment (Cl), roll damping (Clp), the lag sums, the Magnus slope and the pitch damping sums; the standard-normal draws are untruncated unless `dispersion.monte_carlo.truncate_sigma` clips them to ±k sigma (recorded in `dispersion_summary.json`);
  - cases and samples that do not land within `sim.max_time` count as failed: the statistics are flagged incomplete, the maximum landing distance is a lower bound, and the exit status is 3; statistics without enough landed samples are `null`;
  - statically unstable cases and samples are counted per descent mode and kept in the statistics (exit status 3);
  - ballistic runs that fall broadside (angle of attack 60 to 120 degrees for more than 10 s after apogee) are counted and warned about, because their landing depends on the unvalidated large-angle aerodynamics
- **KML output**: launch site, flight path, and landing-point distributions overlaid in Google Earth
- **Readable terminal output**: configuration summary, progress bars with ETA, and result panels in color (plain output is selected automatically when piped)
- **Sphinx documentation**: Japanese documentation (HTML and PDF)

## Screenshots

### Terminal output

At startup, the contents of the configuration file are shown grouped by item, so you can check units and wind directions before the run.

![Startup screen of sim (banner and configuration panel)](doc/images/readme/screen_sim_start.png)

When the flight computation finishes, a result panel and the list of output files are shown.

- The static margin row (`min margin`) gives the smaller of the free-flight minimum (which includes the rail-exit instant) and the rail-exit value, with the time and Mach number where it occurs. For the sample it is 1.99 cal at the rail exit, because the CG moves forward during the burn.
- The row carries a verdict mark: green at 1.5 cal or more, a warning below 1.5 cal, `low static margin` below 1.0 cal, `UNSTABLE` below 0 cal.
- With dual deploy, a `main deploy` row gives the height and time of the main parachute.
- In plain output, a `deploy` line gives the time, airspeed and dynamic pressure at the parachute opening (also on the `main deploy` line).
- When the flight ends before the end of the thrust curve, the burnout row reads `not reached`.

![Completion screen of sim (result panel and output files)](doc/images/readme/screen_sim_result.png)

Heavy computations show a progress bar, an ETA, and intermediate results.

![Screen during aero with the panel method](doc/images/readme/screen_aero_panel.png)

![Screen during dispersion with Monte Carlo](doc/images/readme/screen_dispersion.png)

### Result figures

All of the following are results for the bundled sample rocket (100 mm diameter, 1.5 m long, four fins), drawn by `python/plot.py`.

**Geometry extracted from the STL** (body outline, detected fins, center of pressure)

![Extracted geometry](doc/images/readme/geometry.png)

**Aerodynamic coefficients** (breakdown of zero-lift drag, slope of the normal-force coefficient, center of pressure, normal force versus angle of attack)

![Aerodynamic coefficients](doc/images/readme/aero.png)

**Flight time history** (altitude, speed, Mach number, angle of attack, CP–CG separation at the flown angle of attack during the ascent off the rail, dynamic pressure, thrust, mass, pitch angle, sideslip angle, roll rate). Only the altitude panel covers the whole flight; the other panels end at the parachute deployment (the thrust panel shows the burn), so the parachute descent appears only in the altitude panel

![Flight time history](doc/images/readme/trajectory.png)

**3-D flight path** (rail, free flight, and parachute descent are color-coded)

![3-D flight path](doc/images/readme/trajectory_3d.png)

**Landing dispersion on a wind grid** (landing points for each wind speed x direction; ballistic descent on the left, parachute descent on the right)

![Landing dispersion on a wind grid](doc/images/readme/dispersion_grid.png)

**Landing dispersion by Monte Carlo** (1000 samples, 1-sigma and 3-sigma error ellipses, RK45; 1000 landing points of ballistic descent on the left and of parachute descent on the right)

![Landing dispersion by Monte Carlo](doc/images/readme/dispersion_mc.png)

**Surface pressure coefficient Cp from the panel method** (angle of attack 4 degrees, lowest subsonic Mach number; developed body surface and both sides of the fins)

![Surface pressure coefficient from the panel method](doc/images/readme/panel_cp.png)

## CFD

With `aero.method = "cfd"`, IgnisYeet solves the flow around the rocket with SU2 (Euler equations by default) for a matrix of Mach number, angle of attack and, optionally, sideslip, and builds the coefficient table from the wall pressure. The flight simulation is unchanged. The wall geometry is rebuilt from the extracted body and fins (`surface = "cad"`), taken directly from the STL (`surface = "stl"`; the shells are repaired, united by a Manifold boolean union and remeshed, and a fidelity report states how far the CFD wall is from the input), or taken from a CAD model (`surface = "step"`, see below). Set up the tools (see [Requirements](#requirements)), check them, and run the CFD example:

```sh
./target/release/ignisyeet cfd-check examples/sample_cfd.toml --mesh   # tools, versions, case count, mesh quality
./target/release/ignisyeet aero      examples/sample_cfd.toml          # runs the SU2 cases (cached and resumable)
```

[`examples/sample_cfd.toml`](examples/sample_cfd.toml) is a reduced matrix (4 Mach x 3 angles of attack, Euler, 2 mm surface mesh). Euler meshes the fin faces at a quarter of the fin thickness by default (`fin_size = "auto"`, 1.4 M cells per case, 3.3 to 4.6 GB with 4 MPI ranks). In a measured run on a 20-thread desktop (16 cases with alpha = 2 degrees added, one case at a time on 4 ranks while other jobs shared the machine, 2.5 hours in total) the 12 cases of this matrix took about 1.9 hours. The commented options in that file cover sideslip (`betas_deg`), roll damping (`roll_damping`), mirror-symmetry handling (`shape_symmetry`), the Euler base treatment of an STL (`base_treatment`) and the early stop (`min_iterations`, `stable_window`).

[`examples/sample_step.toml`](examples/sample_step.toml) solves the CAD model [`examples/sample_rocket.step`](examples/sample_rocket.step) (the sample rocket with a launch lug and two rail buttons, in millimetres) with `surface = "step"`. Export the CAD model as one closed solid in the coordinate system of `rocket.stl` (same origin, nose direction and roll); STEP and IGES are read in the unit the file declares (`step_unit` overrides it, BREP is read as metres). Before meshing, the CAD model is registered against `rocket.stl`, from which the flight model geometry comes: the STL must lie on the CAD surface within max(2 `wall_size`, 1 % of the length), and parts of the CAD model that the STL lacks (lugs, buttons) are counted as details. A mismatch stops the run and names the likely cause (unit, orientation or position, a different revision). The lug and the buttons make the sample shape asymmetric, so it is solved on the full domain (about twice the cells of the half model) with the negative angles of attack. `cfd-check --mesh` prints the import, registration and domain summary; the Japanese documentation describes the export, the error messages and the internals.

The figures below are for the bundled sample rocket (Euler, 2 mm surface mesh, fins at 1 mm). **The labels inside these figures are in Japanese.**

**Mach number on the symmetry plane** (angle of attack 4 degrees; M = 0.8 on top, M = 2.0 below)

![Mach number field on the symmetry plane](doc/images/readme/cfd_mach_field.png)

**Wall pressure coefficient Cp** (angle of attack 4 degrees; M = 0.8 on top, M = 2.0 below; left: the nose, right: the fins and the tail with the Euler tail fairing behind the base)

![Wall pressure coefficient](doc/images/readme/cfd_cp_surface.png)

**Surface mesh** (near the nose tip and at the fin root)

![Surface mesh near the nose tip and fin root](doc/images/readme/cfd_mesh_surface.png)

**Direct-STL CFD compared with the rebuilt geometry and the Barrowman method** (CN, center of pressure, roll damping Clp)

![Direct-STL CFD versus rebuilt geometry versus Barrowman](doc/images/readme/cfd_stl_check.png)

What the CFD results are, and are not:

- **Cross-checks only.** The CFD in direct-STL mode and the CFD on the rebuilt geometry differ by 2 to 5 % in CN. For the roll damping at M = 0.8, Clp is -21.4 (STL CFD) and -20.9 (rebuilt CFD) against -29.9 from the Barrowman method. These are comparisons between methods, not validation against measurements; no comparison with a real rocket has been made.
- **Euler (inviscid).** Skin friction and base drag are not obtained from CFD and are added from the Barrowman model, as is the fin-edge drag for `surface = "cad"`; with `surface = "stl"` and `"step"` the fin faces are part of the CFD wall, so the fin-edge term is not added. The CFD wall is close to, but not identical to, the input STL; read the fidelity report before using a result. With the Euler solver an STL needs an explicit `base_treatment`: `"fairing"` adds a conical tail fairing behind the flat base (this changes the input shape), `"flat"` keeps the shape but may not converge (it did not for the sample STL).
- **RANS** (`model = "rans"`): the Spalart-Allmaras model (SA-neg) on a mesh whose prism boundary layer is built from the wall triangles (first layer for y+ = 1 by default, growing to the size of the tetrahedra outside), solved first order and then second order (Roe, MUSCL with the Van Albada limiter) and accepted on residual drops below the cold-start peak, flat coefficients and an extrapolated trend. Skin friction comes from CFD; the base drag from the Barrowman model (`rans_base = "model"`, default) or from CFD (`"cfd"`). Only for `surface = "cad"` with a fin layout symmetric about y = 0 and for `surface = "step"` (whose sharp CAD edges and narrow gaps the prism layer cannot always follow; RANS on a CAD model has been checked up to the mesh validity, not with SU2 solutions), and without `roll_damping`; a case needs much more memory than Euler (an estimated 9 GB at the RANS default `wall_size` of 5 mm and 22 GB at 3 mm for the sample half model). RANS is verified (against reference solutions, correlations and the Euler CFD, not against measurements) for attached-flow skin friction (flat plate within 0.8 % of the NASA TMR SA solution at M = 0.2 and 1.2 % of van Driest II at M = 2; cone and finless body within 2.5-7.4 %) and for the sample rocket at M = 0.8 and 2, alpha = ±4 deg (monotone mesh convergence, GCI below 2 %; CN within 5.0 % and xcp within 4 mm of the Euler CFD). Not verified: the base flow (`rans_base = "cfd"`), convergence at low subsonic speed (at M = 0.3 the alpha = 0 and negative-alpha cases end in limit cycles; 75 % of the sample table cases converged), and RANS on a CAD model. At subsonic speed a half model leaves a spurious CN at alpha = 0 from an asymmetric base wake (0.01-0.03 at M = 0.8), removed by `alpha_mode = "mirror"`. The RANS wall is smooth: the configured roughness enters as the Barrowman rough-minus-smooth friction increment, and on the rebuilt geometry (whose fin edges are blunted for the prism layer) the configured fin edges enter as the Barrowman fin-edge drag minus the 7.8 % share the blunted edges already carry. Details are in the [verification chapter](https://www.ddd3h.com/ignisyeet/verification_aero.html); do not treat RANS tables as verified beyond it.
- **Mesh convergence is an estimate.** With the fins at 1 mm, CN rises monotonically as `wall_size` goes from 10 to 1.8 mm; the default 2 mm is 0.1 % below the 1.8 mm result and an estimated 0.3 to 0.6 % below the Richardson-extrapolated value (the extrapolation itself moves by 0.3 % with the meshes used). Finer fin faces than 1 mm were not tried.
- **No Mach extrapolation.** CFD tables are held at the values of the highest (lowest) solved Mach number beyond the solved range; `cfd_report.json` records `mach_clamped_above` / `mach_clamped_below` and a warning is issued.
- **Quasi-steady.** Pitch and yaw damping are not obtained from the steady solutions: the damping sums of the Barrowman (or panel) model are rescaled to the CFD normal-force slope and centre of pressure. Clp comes from the rotating-frame runs when `roll_damping` is on (otherwise from the Barrowman model); cross derivatives are not modelled. Outside the solved (alpha, beta) range of a general table the values come from the model, not from CFD.
- Tested on Linux with conda-forge SU2 8.3.0, gmsh 4.15.2, Open MPI 5.0.11 and manifold3d 3.5.4; other environments have not been tested.

Details: [CFD usage](https://www.ddd3h.com/ignisyeet/cfd.html), [CFD internals](https://www.ddd3h.com/ignisyeet/cfd_internals.html) and [verification of the aerodynamic models](https://www.ddd3h.com/ignisyeet/verification_aero.html) (all in Japanese).

## Requirements

| Software | Purpose |
|---|---|
| Rust (stable) and `cargo` | Building the main program |
| [uv](https://docs.astral.sh/uv/) and Python 3.10 or later (`venv` and `pip` also work instead of uv) | Plotting (numpy, pandas, matplotlib) |
| SU2 8.3.0, gmsh (Python API), Open MPI, manifold3d | Only for `aero.method = "cfd"` (manifold3d only for `surface = "stl"`); see [CFD](#cfd) |
| LuaLaTeX (luatexja), dvisvgm | Only for building the TikZ figures of the documentation |
| upLaTeX, dvipdfmx, latexmk, inkscape | Only for building the PDF version of the documentation |

## Installation

One-line installer (Linux x86_64 / aarch64, macOS Intel / Apple Silicon; on Windows, use WSL and the Linux instructions; the CFD mode needs a Unix-like system (Linux, macOS, WSL), while the rest of the workspace also passes `cargo check` on Windows):

```sh
curl -fsSL https://raw.githubusercontent.com/ddd3h/ignisyeet/main/install.sh | bash
```

The installer asks whether you are a **user** or a **developer**, shows which tools it already found (cargo, uv, python3, git, conda/micromamba, an existing `ignisyeet-cfd` environment, SU2, gmsh, TeX, ...), and installs only what is missing. Nothing is reinstalled when it is already present, and re-running it is safe.

| Role | What is installed |
|---|---|
| user | The release binary for your platform (sha256 verified) in `~/.local/bin`, the examples and `python/plot.py` in `~/.local/share/ignisyeet/<version>` (with a `current` link), a plotting environment (uv, or `venv` + pip), and an `ignisyeet-plot` command. No documentation tools. |
| developer | A git clone (default `~/ignisyeet`, or the clone you run it from), Rust via rustup if `cargo` is missing, uv, `cargo build --release`, `cargo test --release`, the `python/` and `doc/` environments. Missing documentation tools (TeX Live, dvisvgm, mutool, inkscape, IPAex fonts) are listed with the exact `apt` / `brew` command; the installer never runs `sudo`. |

Both roles can additionally install the CFD tools (SU2, gmsh, Open MPI from conda-forge, about 3 GB, plus manifold3d 3.5.4 from pip for `surface = "stl"`) into a conda environment named `ignisyeet-cfd`. The versions are pinned on all platforms (SU2 8.3.0, gmsh 4.15.2, Open MPI 5, Python 3.10; micromamba 2.9.0 when it has to be installed) because the SU2 8.5.0 package crashes on CPUs without AVX-512. An existing micromamba/mamba/conda and an existing environment are reused. The plotting environment is installed from the locked versions (`uv.lock`, or `scripts/plot-constraints.txt` with pip). The installer prints the `[aero.cfd] prefix` value to put in your configuration (or `IGNISYEET_CFD_PREFIX`).

Options are given after `bash -s --`:

```sh
curl -fsSL https://raw.githubusercontent.com/ddd3h/ignisyeet/main/install.sh | bash -s -- --developer --cfd -y
```

| Option | Meaning |
|---|---|
| `--user` / `--developer` | Choose the role without asking |
| `--cfd` / `--no-cfd` | Install or skip the optional CFD tools |
| `--version vX.Y.Z` | Release to install (default: the latest GitHub release); developer: tag to check out. Only the form `vX.Y.Z` is accepted |
| `--prefix DIR` | Install prefix (default `~/.local`) |
| `--dir DIR` | Developer clone location |
| `--no-test` | Developer: skip `cargo test` |
| `--add-path` | Append the `PATH` line to your shell rc file (never done silently) |
| `--force` | Reinstall even if the same version is installed |
| `-y`, `--yes` | No questions; defaults (user, no CFD, no rc-file edit) |
| `--dry-run` | Show the plan and exit |
| `--uninstall` | Remove the binary, links and release data; asks before removing the plotting environment and the CFD environment (`--purge` removes both). A developer clone is deleted only after an explicit confirmation |
| `--no-color`, `--help` | Plain output; usage |

Every option also has an environment variable (`IGNISYEET_VERSION`, `IGNISYEET_PREFIX`, `IGNISYEET_ROLE`, `IGNISYEET_DIR`, `IGNISYEET_CFD=1|0`, `IGNISYEET_YES=1`, ...). Output follows `NO_COLOR`, falls back to ASCII on a non-UTF-8 locale and to plain lines when it is not a terminal. Everything is logged to `~/.local/share/ignisyeet/install.log`.

Releases are published on GitHub as `ignisyeet-<version>-<target>.tar.gz` with a `.sha256` file; the installer verifies the checksum before installing, and you can check it by hand with `sha256sum -c ignisyeet-<version>-<target>.tar.gz.sha256`. To read the script before running it, download it first: `curl -fsSLO https://raw.githubusercontent.com/ddd3h/ignisyeet/main/install.sh && bash install.sh`.

`--prefix` and `--dir` are made absolute at start-up (relative paths would give broken symlinks).

### Building from source

```sh
git clone https://github.com/ddd3h/ignisyeet.git
cd ignisyeet
cargo build --release          # produces target/release/ignisyeet
```

### Quick start

Run the whole pipeline (aerodynamics, flight, dispersion, plotting) on the bundled sample rocket.

```sh
make all                        # uses examples/sample.toml
make all CONFIG=my/rocket.toml  # to use your own configuration file
```

Notes on `make all`:

- It runs the targets `aero`, `sim`, `dispersion` and `plot` in order, from the top directory of the repository.
- The plots are read from the `output.dir` of the configuration file (a basic or literal TOML string; indentation, a quoted key and spaces inside the `[output]` header are accepted; without `dir` it is `out` next to the configuration file).
- When `sim` or `dispersion` exits with status 3 (outputs written but incomplete or not usable), it still makes the plots and then fails with `make: results are INCOMPLETE or NOT USABLE (exit status 3): see the lines above` (the 3 is the `ignisyeet` status; GNU make itself exits with status 2). Any other failure stops it at once.
- The targets run serially even under `make -j`; paths with spaces are handled; the binary is taken from `$CARGO_TARGET_DIR/release` when that variable is set.

The equivalent commands are as follows.

```sh
./target/release/ignisyeet aero       examples/sample.toml   # aero_table.csv/.json, aero_drag.csv
./target/release/ignisyeet sim        examples/sample.toml   # trajectory.csv, summary.json
./target/release/ignisyeet dispersion examples/sample.toml   # dispersion.csv
uv run --project python python/plot.py all examples/out      # plots/*.png
```

`make geom` extracts the geometry on its own (geometry.json, profile.csv), and `sim --descent ballistic` flies the ballistic descent.

Before the results are used to set a keep-out distance, read the [procedure for keep-out studies](https://www.ddd3h.com/ignisyeet/dispersion.html#sec-dispersion-safety) and the [limitations](https://www.ddd3h.com/ignisyeet/limitations.html) (Japanese). The results are model results; no comparison with a real flight has been made. In short:

- Every maximum landing distance is a maximum over the computed cases only (wind speeds, directions, samples).
- The radius is the largest landing distance (ballistic and parachute) over all of the following runs, plus a margin:
  - a wind grid up to the launch-limit wind, refined until the maximum stops changing;
  - Monte Carlo runs at the farthest grid wind and at the forecast wind;
  - for a dual-deploy rocket, the normal configuration in all of the above, and the failures (main at apogee, drogue only) as a grid and as a Monte Carlo run at the farthest wind of each failure;
  - all of it with both `aero.method = "barrowman"` and `"panel"`.
- A Monte Carlo maximum grows with the number of samples (the normal draws have unbounded tails): use 10^4 samples or more and report P99, P99.9 and the maximum. `dispersion.monte_carlo.truncate_sigma` clips the draws when the far tails are not physical; the clipping bound is then part of the assumptions.
- Wind-grid speeds are speeds at `wind.ref_height` for the `constant`, `power` and `log` models, and the profile mean speed for `profile`. For a profile, `wind_at_ref_height` in `summary.json` gives the table wind at `wind.ref_height`, so a launch-limit wind `V_lim` measured there corresponds to the grid speed `wind_speed * V_lim / wind_at_ref_height.speed`.
- A profile is moved to the farthest grid wind with `wind.profile_mean_speed` and `wind.profile_mean_direction_deg`; it ignores `wind.speed` and `wind.direction_deg`.
- Keep a wind profile table above the highest apogee you expect in the grid and Monte Carlo runs (for example the nominal apogee plus 3 sigma), not just above the nominal apogee: above its top the wind is held constant, and a warning counts the runs that go higher.
- Runs that fall broadside are kept in the maxima; their landing points depend on the unvalidated large-angle aerodynamics, so compare both aero methods.
- The procedure gives the complete bash command sequence (with `--set`, `--out` and `--aero-table`).
- Budget time and disk: the full procedure for the example wind profile (7 wind grids and 30 Monte Carlo runs of 10^4 samples, `rk45`) took about 8 minutes on a 20-thread desktop (Core i7-12700) and wrote about 610 MB, nearly half of it KML files. Limited to 4 threads on the same machine a Monte Carlo run took 2.5 times longer, so plan for tens of minutes to over an hour on a 4-core laptop; with the sample's `rk4` (`dt = 0.002`) each Monte Carlo run is about 13 times slower. Add `--set output.kml=false` for batch runs that do not need the KML files.

To plot with `pip` instead of uv, create a virtual environment first.

```sh
python3 -m venv .venv && . .venv/bin/activate
pip install -r python/requirements.txt -c scripts/plot-constraints.txt
python python/plot.py all examples/out
# via make: make plot PLOT="python python/plot.py"
```

`sim` and `dispersion` run `aero` automatically when there is no coefficient table, or when the STL or the aerodynamic settings have changed. The table is also rebuilt when the program version or the model version changes; `aero --force` rebuilds it unconditionally.

## Commands

```text
ignisyeet [-q|--quiet] [--no-progress] <command> ...
```

| Command | Description |
|---|---|
| `geom <config>` | Extracts the geometry from the STL and writes `geometry.json` and `profile.csv` |
| `aero <config> [--force]` | Builds the aerodynamic coefficient table. If the inputs are the same as last time, the stored table is used; `--force` always rebuilds it |
| `sim <config> [--descent ballistic\|parachute]` | Flies once with the configured wind and writes `trajectory.csv` and `summary.json` (see the notes below) |
| `dispersion <config>` | Computes the landing dispersion in parallel according to `dispersion.mode` (see the notes below) |
| `cfd-check <config> [--mesh]` | Checks that SU2, MPI and gmsh (and manifold3d for `surface = "stl"`) are installed (exit status 1 if a required tool is missing), reports their versions and the number of CFD cases; `--mesh` also exports the wall surface, builds the volume mesh and reports node/cell counts and quality (and the STL fidelity for `surface = "stl"`, the CAD import and registration for `surface = "step"`) |
| `sample-stl <path>` | Writes the STL of the bundled sample rocket (tangent ogive, four trapezoidal fins, in mm, nose pointing +z) |

Notes on `sim` and `dispersion`:

- `sim` removes the old `trajectory.csv` and `summary.json` at the start. A failed run writes `{"failed": true, "error": ...}` to `summary.json` and rewrites `ignisyeet.kml` without the flight layer.
- `sim` exits with status 3 when the rocket does not land within `sim.max_time`, is statically unstable, its static stability could not be evaluated, or the flight looked up the aero table beyond its angle or Mach range.
- `dispersion` removes the earlier results of the configured mode at the start (`dispersion.csv` and `dispersion_meta.json` for the wind grid, `dispersion_mc.csv` and `dispersion_summary.json` for Monte Carlo), so that a failed run leaves no results that look like its own. The results of the other mode are kept.
- `dispersion` exits with status 3 when cases or samples fail or do not land, the nominal ballistic flight is statically unstable or its stability could not be evaluated, any case or sample is statically unstable or of unknown stability, or the aero table was looked up beyond its angle or Mach range. It exits with status 1 when every wind-grid case stops with an error.

| Global option | Description |
|---|---|
| `-q`, `--quiet` | Prints only errors, warnings (on stderr) and the paths of the written files (for scripts) |
| `--no-progress` | Does not draw progress bars and spinners (the boxed summaries are still shown) |

| Option of `geom`, `aero`, `sim`, `dispersion` | Description |
|---|---|
| `--set KEY=VALUE` | Changes a configuration key for this run without editing the file, e.g. `--set wind.profile_mean_speed=15.6` or `--set dispersion.mode=monte_carlo`. `KEY` is the dotted TOML key path, `VALUE` a TOML value (a bare word that is not a TOML value is a string). Repeatable; checked like the file (unknown keys and bad values are errors) and part of the config hash. It cannot remove a key |
| `--out DIR` | Writes the outputs to `DIR` instead of `output.dir` (a relative `DIR` is resolved from the current directory). One run per output directory: a run holds a lock file `.ignisyeet.lock` there, and a second run on the same directory stops with `another ignisyeet run is using … (lock file …, pid …); wait for it or use a different --out (if no other run is active, delete …/.ignisyeet.lock)` (a lock left by a dead process is removed). Output files are written to a temporary file and renamed, so a reader never sees a half-written file |
| `--aero-table PATH` | (`aero`, `sim`, `dispersion`) Reuses an `aero_table.csv` (or a directory containing one) built in another output directory for the same rocket and aero settings (same cache key, otherwise an error); the table and its side files are copied to the output directory. Not for `aero.method = "table"` or with `--force` |

`--version` prints the version together with the copyright and license notice.

**Exit status.**

- 0: success.
- 1: error: invalid configuration, missing input, a failed step (including a diverged integration, also a speed above 8 km/s or a height above 1000 km), missing CFD tools, or a wind grid in which every case stopped with an error.
- 2: usage error, including a malformed `--set` (not `KEY=VALUE`, or a key with an empty part).
- 3: the run finished and wrote its outputs, but the results are incomplete or not usable. Do not use them for safety decisions.
  - Incomplete (a line `INCOMPLETE RESULTS` on stderr): a flight did not land within `sim.max_time`, or dispersion cases or samples failed or did not land (including wind-grid cases whose integration stopped with an error).
  - Aero table looked up beyond its angle range: `RESULTS NOT USABLE: … lookups beyond the aero table's angle range …: the descent uses extrapolated coefficients; build the table up to 90 deg`, and `angles_in_range` is false in `aero_table_range`.
  - Aero table looked up beyond its Mach range (Mach is held at the table edge, which changes the drag and stability of the ascent): `RESULTS NOT USABLE: 4426 of 53877 lookups beyond the aero table's Mach range (max Mach 1.53 > mach_max 1.20): coefficients are held at the edge; build the table up to at least Mach 1.6` (the sample with `aero.mach_max = 1.2`), and `machs_in_range` is false in `aero_table_range`.
  - Static stability could not be evaluated: `RESULTS NOT USABLE: static stability could not be evaluated (…)`; `stable` is null and the reason is in `stability_unknown` (counted per descent mode in `stability_unknown_runs` for `dispersion`).
  - Statically unstable, i.e. the smaller of the free-flight minimum small-angle static margin and the rail-exit value is below 0 cal (for `sim` whether or not it landed; for `dispersion` judged on the nominal ballistic flight): `RESULTS NOT USABLE: the rocket is statically UNSTABLE (min static margin -0.23 cal at t = 1.00 s, Mach 0.30)` (example values); `stable` and `static_margin_min_cal` in `summary.json`, `dispersion_meta.json` and `dispersion_summary.json` give the value, and `static_margin_min_t` and `static_margin_min_mach` in `summary.json` where it occurs.
  - Unstable wind-grid cases or Monte Carlo samples: `RESULTS NOT USABLE: N of M Monte Carlo samples (ballistic) are statically UNSTABLE (min static margin down to … cal): …`; the counts per descent mode are in `unstable` and `unstable_min_static_margin_cal`, and the CSV files have a `stable` column. These runs stay in the statistics.

**Validation and warnings.** Every numeric configuration key is range-checked, non-finite values (`nan`, `inf`) are rejected, and the message names the key (and the file and line for input files). The full list of checks is in the [documentation](https://www.ddd3h.com/ignisyeet/usage.html#sec-config-checks) (in Japanese).

- **Errors** (exit status 1):
  - unknown keys, and values outside their bounds (for example `sim.dt` above 0.05 s or below 1e-5 s, more than 1e8 `rk4` steps (`sim.max_time / sim.dt`), an integer `--set` value beyond the 64-bit TOML range, `sim.rtol` or `sim.atol` above 1e-3, `aero.mach_max` above 10, `aero.n_slices` outside 20 to 10000, `dispersion.directions` above 360, an empty `dispersion.wind_speeds`, zero or more than 1e6 Monte Carlo samples, a `dispersion.monte_carlo.truncate_sigma` that is not a finite number > 0, an aero table grid above about 1e6 points (Mach rows times the one-sided 0 to `alpha_max_deg` angle rows), invalid `[aero.panel]` options (checked for every method, the message names the `aero.panel.*` key; `transonic[1]` must not exceed `aero.mach_max` unless it is at most 1.2)); a `--set` value of the wrong type or an unknown `--set` key names the `--set` override (`--set rocket.dry_mass=abc: invalid type: string "abc", expected f64`);
  - `rocket.cg_dry` outside the body or a body length outside 0.1 to 20 m (the geometry warnings follow the message as `geometry warning: …` lines); a motor longer than the body (`the motor (… m) is longer than the body (… m): check motor.eng (the motor length in its header, in millimetres) and rocket.stl_scale`); `motor.aft_x` (default: the body length) placing the motor outside the body (it must lie between the motor length and the body length); `rocket.fin_override.thickness` of 0 or less with the panel method or the CAD-surface CFD, which mesh the fins as biconvex sections of that thickness;
  - `launch.elevation_deg` outside (0, 90];
  - wind: a non-finite or negative `wind.speed` or a non-finite `wind.direction_deg`; `wind.direction_deg` or a wind profile direction outside 0 to 360 degrees; a wind profile level below the launch site (negative altitude); a log wind profile with `2 * roughness_length >= ref_height` (the exponent, roughness length and blend height are checked only for the wind models that use them); `wind.profile_mean_speed` or `profile_mean_direction_deg` with a model other than `profile`, a negative mean speed or a direction outside 0 to 360 degrees, or a calm profile to be scaled or rotated; a calm wind profile (mean speed below 0.1 m/s) used for a wind grid or a nonzero wind sigma (the message for Monte Carlo names the wind perturbations);
  - `sim.output_interval` below 1e-4 s or `sim.max_time / sim.output_interval` above 1e7;
  - a truncated binary STL; an STL surface that is not closed: more than 1 % of the edges open, or a hole wider than 0.1 body diameters (`the STL surface is not closed: a hole bounded by … open edges is … m across (… % of the body diameter … m): …`; a missing end cap is caught even when it opens fewer than 1 % of the edges);
  - `recovery.cd_s` of 0 with recovery enabled; a parachute descent (`sim.descent = "parachute"` or `sim --descent parachute`) with recovery disabled; an incomplete dual-deploy pair;
  - an inertia tensor that is singular or ill-conditioned (largest to smallest principal moment above 1e8 at any time of the burn);
  - a missing or invalid input file (STL, `.eng`, external aero table, wind profile).
- **Warnings** (printed in every output mode, stderr with `-q`, and stored in `summary.json`, `dispersion_meta.json` and `dispersion_summary.json` as `warnings`; the configuration loader's warnings come first):
  - configuration: `wind.speed` or `wind.direction_deg` set for a `profile` wind, which ignores them; a `--set` key given more than once (`--set wind.speed given more than once; the last value (8) is used`);
  - geometry: a body diameter outside 0.005 to 3 m; the geometry findings in `geometry.json`, including a few open or non-manifold edges of the STL surface, fins whose trailing edge lies aft of the body end (`the fin trailing edge at … m lies aft of the body end at … m: …`) and a mesh whose aft end is wider than the body (`the aft end of the mesh is wider than the body …: the fins probably extend behind the body base; …`);
  - stability: a static margin below 1.5 cal (`marginal`), below 1.0 cal (`low static margin`) or below 0 cal (`UNSTABLE`), with the time and Mach number of the minimum; statically unstable wind-grid cases or Monte Carlo samples, or cases of unknown stability; a roll resonance (`roll resonance: the roll rate crosses the resonance rate of the pitch oscillation (… rad/s) at t = … s, Mach …; the lateral CG offset, thrust misalignment or asymmetric aero moment can amplify the angle of attack there`);
  - broadside descent: a ballistic descent that flew broadside (angle of attack 60 to 120 degrees) for more than 10 s after apogee (`the ballistic descent fell broadside (…): the landing depends on the large-angle aerodynamics, which are not validated`; for `dispersion` the count of such cases or samples);
  - wind: an apogee above 1000 m with the `power` or `log` wind, or above the top of a wind profile table (for `dispersion` also counted over the cases or samples: `N of M wind-grid cases reach above the top of the wind profile (… m), up to … m: …`); a wind profile scaled by a factor above 3 or below 1/3 (`wind profile scaled by a factor of …`, with enough decimals to show that it exceeds 3); a wind angle of attack at rail exit above 15 degrees (computed with the largest wind-grid speed, or the nominal speed plus 2 sigma for Monte Carlo; a wind profile is scaled to that mean speed as the dispersion cases fly it);
  - launch and recovery: a rail exit speed below 15 m/s; a parachute descent that landed without deploying or faster than 15 m/s; a parachute (drogue or main) opening above 50 m/s or 2 kPa (`the parachute opens at … m/s (q = … kPa): a canopy is unlikely to survive; …`); a dual-deploy `main_altitude` at or above the apogee; a deployment delayed to the end of the thrust because the motor still burns at apogee + `recovery.delay` (`the motor still burns at apogee + recovery.delay (t = …, thrust … N): the parachute deployment is delayed to burnout at … s`); a flight that does not land;
  - motor and aero table: a motor thrust curve that does not end at 0 N; a coarse aero table grid (Mach step above 0.1 or angle-of-attack step above 5 degrees, not for an external table); an external table without the lag and Magnus columns (`the external aero table has no lag/Magnus columns: the apparent-mass (lag) sums were filled from the geometry (component build-up); Magnus force and induced roll are 0. If its damping sums hold a measured or CFD Cmq + Cmalphadot ..., write lag_l0 and lag_l1 columns of 0 so they are not counted twice`, or without readable geometry `... the flight has no apparent-mass (lag) terms, Magnus force or induced roll`); a Monte Carlo `magnus_scale` or `lag_scale` with no such terms in the table; an induced roll `cl4` without `fin_count` in the sidecar (four fins assumed); all aero table lookups are counted in `aero_table_range` (lookups beyond the angle or Mach range make the results unusable, see the exit status);
  - runs: wind-grid cases that stopped with an error (counted apart from the cases that did not land; the aero table lookup statistics leave them out); fewer than 30 landed Monte Carlo samples in a descent mode; an SU2 version other than 8.3.x.
- **Exit status**: see above (3 means the outputs were written but are incomplete or not usable).
- Values compared with a threshold are printed with enough decimals to show on which side of the threshold they are (a margin of 1.4977 cal reads `1.498`, not `1.50`).

When stdout is redirected to a pipe or a file, the output is plain line-based text without colors or boxes. The environment variable `NO_COLOR` disables colors, and `TERM=dumb` disables colors and Unicode symbols.

```sh
./target/release/ignisyeet sim examples/sample.toml -q            # only errors, warnings and the written paths
./target/release/ignisyeet dispersion examples/sample.toml --no-progress
NO_COLOR=1 ./target/release/ignisyeet aero examples/sample.toml
```

## Configuration file (TOML)

Configuration is written in TOML. Relative paths are resolved against **the directory of the configuration file**. Lengths are in meters, and positions on the rocket are measured from the nose tip toward the tail. The quickest way to start is to copy [`examples/sample.toml`](examples/sample.toml), which lists every item with comments, and edit it. The default value and meaning of each item are summarized in [Usage](https://www.ddd3h.com/ignisyeet/usage.html) (in Japanese).

| Section | Main contents |
|---|---|
| `[output]` | Output directory `dir` (default `out`, next to the configuration file), `kml` switch for the Google Earth file |
| `[resources]` | Compute resources: `threads` (CPU thread cap, 0 = all available), `memory_gb` (budget for the panel matrix and CFD cases, 0 = no limit), `nice` (0 to 19, lowers the process priority) |
| `[rocket]` | STL path and unit `stl_scale`, `nose_direction`, dry mass, center of gravity, moments of inertia (`ixx_dry`, `iyy_dry`; optional `izz_dry`, products `ixy_dry`/`ixz_dry`/`iyz_dry`, lateral CG offset `cg_lateral_dry`), `fin_cant_deg`, roughness, fin leading- and trailing-edge shapes, `extra_cd`, `[rocket.fin_override]` (manual fin specification, including `cant_deg`) |
| `[motor]` | RASP `.eng` file, motor aft position `aft_x`, nozzle exit diameter, override of the propellant mass, thrust misalignment (`misalignment_deg`, `misalignment_azimuth_deg`, `misalignment_offset`) |
| `[launch]` | Latitude, longitude, altitude, rail length (the distance the rocket travels along the rail until it leaves, i.e. the effective guide length), elevation angle, azimuth, `roll_deg` (body roll about the rail axis) |
| `[recovery]` | Whether a parachute is used (`enabled`), `cd_s`, deployment delay `delay`; optional dual deploy: `main_cd_s` and `main_altitude` (both or neither; the main canopy replaces the drogue `cd_s` when the rocket descends through `main_altitude` above the launch site) |
| `[aero]` | `method` (`barrowman`, `panel`, `table`, `cfd`), external table `table`, Mach and angle-of-attack grids, number of sections `n_slices`, `fin_threshold`, `extrapolation` (`linear`, `clamp`), `table_kind` (`axisymmetric`, `general`) and `general_mach_step` |
| `[aero.panel]` | Panel-method mesh divisions (`body_axial`, `body_circ`, `fin_chord`, `fin_span`), `wake_length`, `tail_radii`, `subsonic_machs`, `transonic`, `fin_section` |
| `[aero.cfd]` | SU2 CFD (see [CFD](#cfd)): `prefix` of the tool environment, `model` (`euler`, `rans`), `machs`, `alphas_deg`, `betas_deg`, `roll_damping`, `roll_rate_hat`, `surface` (`cad`/`panel_mesh`, `stl`, `step`), `step_file`, `step_unit`, `symmetry`, `shape_symmetry`, `base_treatment`, `stl_remesh`, `max_geometry_error`, `wall_size`, `fin_size`, `farfield`, `iterations`, `min_iterations`, `stable_window`, `cfl`, `convergence`, `scheme`, `ranks_per_case`, `parallel_cases`, `workdir`, and more; the commented block in [`examples/sample_cfd.toml`](examples/sample_cfd.toml) lists every item |
| `[earth]` | `model` (`flat`, `ecef`), `gravity` (`constant`, `inverse_square`, `j2`; `j2` requires `ecef`) |
| `[atmosphere]` | `model` (`us1976`, `constant`), `temperature_offset`, constant-atmosphere `density`, `sound_speed`, `viscosity` |
| `[wind]` | `model` (`constant`, `power`, `log`, `profile`; for `profile` the representative wind is the mean speed over the table's altitude range and the direction of its mean wind vector, and a dispersion wind grid scales the whole table to the grid speed; `profile_mean_speed` and `profile_mean_direction_deg` scale and rotate the table to a given mean wind, and `speed` and `direction_deg` are not used), wind speed, wind direction (the direction the wind comes from), reference height, power-law exponent, roughness length, ground blend height `ground_blend_height` (cubic blend to zero near the ground for the `power` and `log` models), altitude table `profile` |
| `[sim]` | `integrator` (`rk4`, `rk45`, `dop853`), `attitude` (`normalize`, `lie_group`), `dt`, `rtol`, `atol`, `max_time` (default 3600 s; a run that has not landed by then is reported as a failure), `output_interval`, `descent`, `quasi_steady` (default false: fly without the apparent-mass, Magnus and CN_q terms) |
| `[dispersion]` | `mode` (`wind_grid`, `monte_carlo`), `wind_speeds`, `directions` |
| `[dispersion.monte_carlo]` | `samples`, `seed`, and 1-sigma values for thrust, burn time, dry mass, center of gravity, CN, CA, elevation (`elevation_deg` is the sigma of an isotropic rail pointing error: two independent tilts, in the vertical plane of the rail and perpendicular to it), azimuth (a compass setting error, a rotation of the rail about the vertical), wind, and parachute; optionally `thrust_misalignment_deg`, `thrust_offset`, `fin_cant_deg`, `cg_lateral`, `cl_scale`, `clp_scale`, `lag_scale`, `magnus_scale`, `cmq_scale` (drawn after all other perturbations, so they leave the other draws unchanged; for keep-out runs a `cmq_scale` of 0.1 (strip damping) and a `lag_scale` of 0.25 (apparent-mass terms, 16-29 % of the sample's pitch damping) are recommended), and `truncate_sigma` (clip every standard-normal draw to ±k sigma) |

## Output files

The output directory is `output.dir` (`examples/out/` in the sample).

| File | Contents |
|---|---|
| `geometry.json`, `profile.csv` | Extracted geometry |
| `aero_table.csv`, `aero_table.json` | Axisymmetric table (format version 4): CN, CA (powered and coasting), Xcp, CNalpha, pitch damping sums, fin cant forcing `cl_delta`, roll damping `clp`, lag sums `lag_l0` [m] and `lag_l1` [m²] (body and fin apparent mass), induced roll `cl4`, Magnus force slope `cnpa` and Magnus moment slope `cmpa` [1/rad] for each (Mach, angle of attack). General table (`kind = "general"`): CX (powered and coasting), CY, CZ, Cl, Cm, Cn, `cl_delta`, `clp`, the pitch and yaw damping sums, `lag_l0`, `lag_l1`, `cnpa` and `cmpa` for each (Mach, alpha, beta). The `.json` holds metadata (including `fin_roll_deg`, the fin the induced roll refers to, and `fin_count`) and a hash of the inputs. Older column sets load with the missing columns zero; an external table without the lag columns gets the lag sums of the STL geometry, with a warning |
| `aero_drag.csv` | Breakdown of zero-lift drag (total only for the panel method), and the Mach dependence of CNalpha and Xcp (not written for `method = "cfd"`) |
| `cfd_cases.csv`, `cfd_report.json` | `method = "cfd"` only: one row per solved case (status, iterations, residuals, coefficients including the six components) and the run summary (failed and unconverged cases, mesh size, timing, domain and symmetry used, roll-damping source, STL fidelity, CAD import and registration). Case data and the mesh are cached in `<output.dir>/<workdir>/` (default `cfd/`); for `surface = "stl"` the mesh directory also holds `repair_report.json`, `union_report.json` and `stl_fidelity.json`, for `surface = "step"` `step_report.json` and the prepared shape `step_prepared.brep` |
| `panel_cp.csv` | Surface panels and Cp (angle of attack 4 degrees) when `method = "panel"` |
| `trajectory.csv`, `summary.json` | Time history and main results of a single flight (see below) |
| `dispersion.csv`, `dispersion_meta.json` | Landing points of all wind-grid cases (ballistic and parachute descent) and their summary (see below) |
| `dispersion_mc.csv`, `dispersion_summary.json` | Perturbations and landing point of each Monte Carlo sample and the statistics per descent mode (see below) |
| `ignisyeet.kml` | A single KML file for Google Earth. Folders are `Launch site`, `Flight`, `Dispersion – wind grid`, and `Dispersion – Monte Carlo` (lines only). `sim` and `dispersion` update it while merging each other's results; only results whose `config_hash` matches the current configuration are merged (stale ones are left out with a warning), and flights or cases that did not land are left out (a `sim` that does not land rewrites the file without the flight layer; Monte Carlo ellipses need at least two landed samples) |
| `plots/*.png` | Figures drawn by `python/plot.py` |

`trajectory.csv` and `summary.json`:

- `trajectory.csv` has `x_cp` and `stability_cal` at the angle of attack flown (a display, not the stability verdict).
- `summary.json` holds the main results, plus `landed`, `max_time`, `burnout_time` (null when the flight ended before the end of the thrust curve), `config_hash`, `warnings` and `aero_table_range` (lookup counts, the largest Mach number and total angle of attack, `angles_in_range` and `machs_in_range`).
- Static stability: `static_margin_min_cal` (small-angle margin) with `static_margin_min_t` and `static_margin_min_mach`; `stable` (true, false, or null when the static stability could not be evaluated, with the reason in `stability_unknown`); `static_margin_pitch_min_cal` and `static_margin_yaw_min_cal` (evaluated at every step; for general six-component tables the stability verdict uses the smaller of the separate pitch and yaw margins).
- Recovery: `drogue_deploy_time`, `main_deploy_time` and `main_deploy_altitude` (dual deploy), `deploy_delayed` (`nominal_time` and `thrust`, only when the deployment waited for the end of the thrust), and the airspeed and dynamic pressure at the openings `drogue_deploy_speed`, `drogue_deploy_q`, `main_deploy_speed`, `main_deploy_q`.
- Roll resonance: `roll_resonance` (`t`, `mach`, `frequency`, `crossings`), written only when the roll rate crossed the resonance rate of the pitch oscillation (the natural frequency with the gyroscopic factor 1/sqrt(1 - Ixx/I)) during the ascent with a trim forcing present.
- Descent: `broadside_time` and `fell_broadside` (ballistic descent); `landing_speed` is the ground-relative speed at touchdown, including horizontal drift; the landing fields are `null` when the flight did not land.
- Environment: `earth_model`, `wind_model`, and for a profile wind `wind_scale` and `wind_at_ref_height` (the nominal profile wind at `wind.ref_height`: `height`, `speed`, `direction_deg`; `sim` and `dispersion` also print it as `profile wind at wind.ref_height = 2 m: 3.04 m/s from 10 deg (mean wind 11.89 m/s)`).
- After a failed run the file holds only `{"failed": true, "error": "..."}`, and `python/plot.py` then refuses to plot the trajectory.

Wind grid (`dispersion.csv`, `dispersion_meta.json`):

- `dispersion.csv` has a row per case with `landed`, `error` (the error text of a case whose integration stopped), `stable` (whether the case is statically stable) and `broadside_time` (the time flown broadside after apogee).
- `dispersion_meta.json` holds `config_hash`, `flight_config_hash`, `cases`, `failed`, `errored`, `warnings` and `aero_table_range`; per descent mode `unstable`, `unstable_min_static_margin_cal`, `stability_unknown_runs` and `broadside_runs`; `stable`, `static_margin_min_cal` and `stability_unknown` of the nominal flight; and the nominal wind `wind_model`, `wind_speed`, `wind_direction_deg`, and for a profile wind `wind_scale` and `wind_at_ref_height`.

Monte Carlo (`dispersion_mc.csv`, `dispersion_summary.json`):

- `dispersion_mc.csv` has a row per sample and descent mode with the perturbations (including `lag_scale`, `magnus_scale` and `cmq_scale`) and the landing point. `elevation_deg`, `azimuth_deg` and `roll_deg` are the changes of the launch attitude actually flown (the rail tilt turns the whole rocket); the column `stable` marks statically stable samples and the last column `broadside_time` gives the time flown broadside.
- `dispersion_summary.json` holds, for each descent mode, the mean, covariance, 1-sigma and 3-sigma error ellipses, P50/P90/P99/P99.9 of the landing distance (`landing_distance_p50`, `_p90`, `_p99`, `_p999`; `_p999` is null with fewer than 1000 landed samples), `failed`, `not_landed`, `complete` and `max_landing_distance_is_lower_bound`.
- It also holds `truncate_sigma` (null unless the draws are clipped), `config_hash`, `flight_config_hash`, `warnings`, `aero_table_range`; per descent mode `unstable`, `unstable_min_static_margin_cal`, `stability_unknown_runs` and `broadside_runs`; `stable`, `static_margin_min_cal` and `stability_unknown` of the nominal flight; and the nominal wind as in `dispersion_meta.json`.

## Plotting

```sh
uv run --project python python/plot.py all <out_dir>
uv run --project python python/plot.py {geometry,aero,panel,trajectory,dispersion} <out_dir>
uv run --project python python/plot.py aero <out_dir> --compare <another output directory>
```

| Subcommand | Output |
|---|---|
| `geometry` | `geometry.png` |
| `aero` | `aero.png`. With `--compare`, also draws `aero_compare.png`, which overlays CNalpha, Xcp, and CD0 of two coefficient tables |
| `panel` | `panel.png` (when `panel_cp.csv` exists) |
| `trajectory` | `trajectory.png`, `trajectory_3d.png` |
| `dispersion` | `dispersion.png` if `dispersion.csv` exists; `dispersion_mc.png` if `dispersion_mc.csv` and `dispersion_summary.json` exist |
| `all` | All of the above (those whose input files exist) |

- The trajectory plot takes the height above the launch site as `summary.json` measures the apogee: the integrated `up` on a flat Earth and the geodetic `alt` minus the launch altitude with `earth.model = "ecef"` (from `earth_model` in `summary.json`). Only the altitude panel covers the whole flight; the other panels end at the parachute deployment.
- A plot that is skipped removes its PNG of an earlier run from `plots/`.
- `plot.py` warns when `summary.json`, `dispersion_meta.json` and `dispersion_summary.json` were written for different flight configurations; the dispersion files are still compared with each other when `summary.json` is the failure marker of a failed `sim`.
- The trajectory and dispersion figures carry a banner when the run's JSON reports a problem: `RESULTS NOT USABLE` (statically unstable, unknown stability, aero table angle or Mach range exceeded), `INCOMPLETE` (runs that failed or did not land) or `CAUTION` (ballistic runs that fell broadside).
- In the dispersion figures, landing points of unstable runs are marked with a red x, of unknown stability with an amber triangle, and of broadside falls with a purple diamond. The wind-grid legend reads `profile mean speed` when `wind_model` in the dispersion files is `profile`.
- The Monte Carlo figure gives the maximum landing distance and P50/P90/P99/P99.9 in a box below the axes (P99.9 only with 1000 or more landed samples).
- For a general six-component table, the aero figure takes CNalpha and Xcp at beta = 0 from signed central differences of CZ and Cm around alpha = 0 (one-sided from 0 when the table has no negative angles; NaN where undefined), the pitch centre of pressure of the static margin.

## Documentation

The documentation (in Japanese) is published at <https://www.ddd3h.com/ignisyeet/>; the chapters are listed in [Overview of the methods](#overview-of-the-methods). It contains equations, figures, and verification, and can also be built from `doc/` (Sphinx).

```sh
cd doc && make html     # doc/build/html/index.html
cd doc && make pdf      # doc/build/latex/ignisyeet.pdf
```

The HTML version needs uv, lualatex (luatexja), and dvisvgm. The PDF version additionally needs uplatex, dvipdfmx, latexmk, and inkscape. Without uv, install the dependencies into a virtual environment and run with an empty `PY`.

```sh
python3 -m venv .venv && . .venv/bin/activate
pip install -r doc/requirements.txt
cd doc && make html PY=     # likewise make pdf PY= for the PDF
```

## Overview of the methods

Detailed equations and derivations are in the chapters of the documentation (in Japanese).

| Stage | Summary | Chapter (Japanese) |
|---|---|---|
| Usage | Installation, configuration items and defaults, commands, outputs | [Usage](https://www.ddd3h.com/ignisyeet/usage.html) |
| Overall | Processing flow, and why the coefficient table is built first | [Computation flow](https://www.ddd3h.com/ignisyeet/overview.html) |
| Geometry extraction | Principal axis by PCA; radius distribution and fins estimated from sections perpendicular to the axis | [Geometry extraction from STL](https://www.ddd3h.com/ignisyeet/geometry.html) |
| Aerodynamics (overview) | The coefficient table that connects the aerodynamic methods to the flight simulation; axisymmetric and general tables; choosing a method; table interpolation and external tables | [Overview of the aerodynamic models](https://www.ddd3h.com/ignisyeet/aerodynamics.html) |
| Aerodynamic coefficients | Six body-axis force and moment coefficients, CN, CA and the center of pressure, alpha, beta and the total angle of attack, damping and roll derivatives | [Aerodynamic coefficients and coordinate systems](https://www.ddd3h.com/ignisyeet/aero_coefficients.html) |
| Aerodynamics (Barrowman method) | Component build-up of normal force, center of pressure, drag, and pitch damping. Supersonic by Busemann's second-order theory, transonic joined by a cubic | [Aerodynamics by the Barrowman method](https://www.ddd3h.com/ignisyeet/barrowman.html) |
| Aerodynamics (panel method) | Morino method (Goethert transformation) for subsonic, local-inclination method for supersonic, cubic Hermite interpolation between them for transonic | [Aerodynamics by the panel method](https://www.ddd3h.com/ignisyeet/panel.html) |
| Aerodynamics (CFD) | Using SU2 through the CFD mode: wall geometry from the rebuilt body or the STL, the run, how to read the outputs, how far to trust them | [Using CFD](https://www.ddd3h.com/ignisyeet/cfd.html) |
| Aerodynamics (CFD internals) | Governing equations, SU2 settings, geometry preparation, meshing, case execution, post-processing | [CFD internals and numerical methods](https://www.ddd3h.com/ignisyeet/cfd_internals.html) |
| Flight | 6-DoF rigid body with full inertia tensor, thrust misalignment, launch roll, launch-rail constraint, changes in propellant mass, center of gravity, and inertia; six-component quasi-steady aerodynamics with rate damping, CN_q force, slender-body apparent mass of body and fins, Magnus force and moment, induced roll | [6-DoF equations of motion](https://www.ddd3h.com/ignisyeet/dynamics.html), [Thrust and mass properties](https://www.ddd3h.com/ignisyeet/propulsion.html) |
| Coordinates and environment | Flat ENU and ECEF, atmosphere, wind, gravity | [Coordinate systems and attitude](https://www.ddd3h.com/ignisyeet/coordinates.html), [Environment models](https://www.ddd3h.com/ignisyeet/environment.html) |
| Landing dispersion | Wind grid and Monte Carlo method, error ellipses | [Landing dispersion](https://www.ddd3h.com/ignisyeet/dispersion.html) |
| Verification of the aerodynamic models | Checks, cross-comparisons between the methods, and computation times | [Verification of the aerodynamic models](https://www.ddd3h.com/ignisyeet/verification_aero.html) |
| File formats | Columns and keys of the coefficient tables, the CFD reports, and the other output files | [File format reference](https://www.ddd3h.com/ignisyeet/file_formats.html) |
| Applicability | Limits of the methods | [Limitations and future work](https://www.ddd3h.com/ignisyeet/limitations.html) |
| References | Bibliography | [References](https://www.ddd3h.com/ignisyeet/references.html) |

Landing points are output as WGS84 latitude and longitude. For descent, you can choose between a parachute (point mass, with `cd_s` and a deployment delay, optionally dual deploy with a drogue and a main canopy; the inflation is instantaneous) and ballistic descent (kept in 6 DoF until landing). In a ballistic descent whose apogee is nearly vertical the rocket tumbles, so the landing point is sensitive to small changes. A tail-first flight uses the nose-first coefficients and centre of pressure at 180 degrees minus the angle of attack; when that centre of pressure lies ahead of the CG at 90 degrees, a broadside trim is stable and the rocket can fall broadside, slowly and far. Such falls are flagged (`fell_broadside`, `broadside_runs`, `broadside_time`): their landing points depend on the unvalidated large-angle aerodynamics. If the motor still burns at apogee + `recovery.delay`, the deployment waits for the end of the thrust.

## Verification

```sh
cargo test --release
```

The main things checked by the tests are as follows (details in [Verification](https://www.ddd3h.com/ignisyeet/verification.html) and [Verification of the aerodynamic models](https://www.ddd3h.com/ignisyeet/verification_aero.html), in Japanese).

- **Geometry extraction**: fin dimensions of a rotated and scaled sample rocket are recovered within 1 mm
- **Barrowman method**: against a hand calculation for the sample rocket (CNalpha = 11.617, center of pressure 1.1619 m), the values computed from the STL agree with CNalpha = 11.617 and 1.1620 m. For the body alone, CNalpha is within 0.02 of 2. Transonic continuity is also checked
- **Panel method**: comparison with analytical potential flows (sphere, spheroid, CNalpha of a slender body, Helmbold wing) and with the relations for supersonic shock waves, expansion waves, and cones. Refining the mesh changes CNalpha by 0.61 %, and the change of CN between adjacent Mach numbers (0.02 apart, Mach 0.6 to 1.6) is at most 6.0 % (largest just above the transonic band, where the fin leading-edge panels cross the wedge-detachment angle). Compared with the Barrowman method, the subsonic CNalpha of the panel method is about 27 % larger and the supersonic one (M = 2) is 11 % smaller
- **Flight**: analytical solution for vertical flight in vacuum, response to crosswind, agreement between ECEF and flat Earth, westward drift due to the Coriolis force, agreement between RK45 and RK4 (landing points within 1 m), the restart of free flight at the rail exit, events inside a step (touchdown before deployment, `sim.max_time`, thrust-curve knots)
- **Aerodynamics cross-checks**: the roll-damping and sideslip signs of the CFD agree with the expanded axisymmetric table, the subsonic CNalpha of the CFD and the panel method differ by 2.5 % (M = 0.8) to 9 % (M = 0.3), and the direct-STL CFD is compared with the rebuilt-geometry CFD (see [CFD](#cfd)). These are comparisons between methods, not validation against measurements
- **Rate and acceleration terms**: as an internal consistency check of the rate and alpha-dot terms (signs, convective factor, CG transfer, fin sections), the sample's pitch damping Cmq + Cmalphadot agrees with the slender-body closed form -2 CNalpha (l_e/d)^2 within 7 % (a finless body within 1.3 %; with the body-only apparent mass it would be 0.68-0.72; both sides use the Barrowman slope, so this is not an independent validation), free pitch oscillations agree with the linear theory of the implemented terms (frequency within 0.001 %, log decrement within 0.005 %), a spinning body with a Magnus force follows the exact roots of Murphy's epicyclic equation (frequency within 0.01 %), the Magnus force direction is checked against McCoy's equations, and switching the new terms off reproduces the quasi-steady trajectory bit for bit
- **Rigid-body details**: moment about the CG from a nozzle offset, thrust misalignment, products of inertia, and launch roll are covered by unit tests
- **Environment and coordinates**: literature values of the standard atmosphere, hydrostatic equilibrium, J2 gravity, round-trip conversion between geodetic coordinates and ECEF
- **Monte Carlo**: reproducibility of random numbers, agreement with the nominal flight at zero standard deviation, covariance and error ellipses, percentiles, incomplete statistics when samples do not land, the rail tilt (rotation of the axis, isotropic spread for a vertical rail, samples with an invalid elevation failing), unstable samples counted per descent mode
- **Dual deploy**: terminal speeds of the drogue and the main canopy, the main deployment at the set height, the cubic Hermite apogee
- **Configuration and outputs**: range and non-finite checks, integrator and table bounds, UTF-8 BOM input, the warnings, the cache key and the configuration hash, stale outputs left out of the KML
- **Aerodynamic table lookups**: Mach clamped at the table edge, bounded extrapolation in angle of attack, out-of-range lookups counted

## Directory layout

```text
Cargo.toml              Rust workspace
crates/geom/            STL I/O, axis detection, sectioning, fin extraction, sample geometry
crates/aero/            Atmosphere, Barrowman/Niskanen model, coefficient table
crates/panel/           Panel-method aerodynamics (Morino method for subsonic, local inclination for supersonic)
crates/cfd/             SU2 CFD driver: STL repair and union, gmsh meshing, case runner, six-component table
crates/sim/             Motor, 6-DoF flight, geodetic computations
crates/cli/             ignisyeet command and TOML configuration
python/plot.py          Plotting (python/ is a uv project)
examples/               sample.toml, sample_rocket.stl, sample_motor.eng (a fictional motor)
doc/                    Sphinx documentation in Japanese (TikZ and matplotlib figures)
doc/images/readme/      Images of this README
Makefile                Wrapper for cargo and uv
LICENSE                 GNU GPL version 3 (full text)
NOTICE                  Copyright, additional terms under GPLv3 section 7(b), and requests
CITATION.cff            Citation metadata
```

## Development

A release is made by pushing a tag equal to the workspace version (`git tag v0.4.0 && git push origin v0.4.0`); the `release` workflow builds the four platform archives (GNU tar, reproducible ordering and timestamps) and creates the GitHub Release. The release archives contain only the tracked files of `examples/`.

- `ci` workflow (every push and pull request): builds the workspace with the minimum supported Rust version (1.87, `rust-version` in `Cargo.toml`); builds and tests it with stable (`--locked`); runs `geom`, `aero`, `sim`, a wind-grid `dispersion`, a Monte Carlo `dispersion` and `python/plot.py` on the sample, and the `python/test_plot.py` unit tests; checks that an incomplete flight exits with status 3. Its clippy (`-D warnings`), rustfmt (`--check`; formatting follows `rustfmt.toml`, `max_width = 160`) and ShellCheck (`install.sh`, `scripts/*.sh`) steps fail the job on any finding; a ShellCheck false positive is suppressed with a `# shellcheck disable=SCxxxx` line that gives the reason.
- `docs` workflow: builds the HTML documentation with `-W` on pull requests and pushes to `main`, and publishes it to GitHub Pages only from `main`.

## License

IgnisYeet is free software, licensed under the **GNU General Public License, version 3 or (at your option) any later version** (`GPL-3.0-or-later`). The full text is in [`LICENSE`](LICENSE). It comes with NO WARRANTY.

Copyright (C) 2025-2026 西濱大将 (NISHIHAMA Daisuke)

In addition, the following **additional term under section 7(b) of the GPLv3** applies to this program and to works based on it:

- The author attribution "IgnisYeet by 西濱大将 (NISHIHAMA Daisuke)" and the copyright notice must be preserved in all copies and modified versions. This covers the source files, the documentation, and the Appropriate Legal Notices of the program (the startup banner and the `--version` output).
- Modified versions must be marked as changed (this is also required by section 5(a) of the GPLv3).

The exact wording of the terms is in [`NOTICE`](NOTICE).

## Citation and notification (requests, not conditions)

The following are **polite requests only. They are not conditions of the license** and do not restrict any right granted by the GPL. Not following them does not affect your rights.

- **Citation**: if you use IgnisYeet or its results in publications, reports, or presentations, please cite it. Citation metadata is in [`CITATION.cff`](CITATION.cff); GitHub shows a "Cite this repository" button from it.
- **Notification**: if you use IgnisYeet in a project, a publication, or a derived work, please let the author know by e-mail at `daisukenishihama63@gmail.com` or by opening an issue at <https://github.com/ddd3h/ignisyeet/issues>.

## Contact

For bug reports and feature requests, please open a GitHub issue or write to `daisukenishihama63@gmail.com`.
