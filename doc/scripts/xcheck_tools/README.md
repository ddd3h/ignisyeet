# Cross-comparison with OpenRocket and RocketPy

These scripts fly the sample rocket (`examples/sample.toml`, `sample_rocket.stl`, `sample_motor.eng`) in
IgnisYeet, OpenRocket and RocketPy and collect the results in `doc/data/xcheck_tools/`. The figures and the
section "他のシミュレータとの相互比較" of `doc/source/validation.rst` are built from those files.
It is a comparison between programs, not between a program and a measurement.

## Versions used for the committed data

| tool | version |
| --- | --- |
| IgnisYeet | 0.4.0 (`target/release/ignisyeet`) |
| OpenRocket | 24.12 (`OpenRocket-24.12.jar`, SHA256 `4959b72f52f5f607941e9722abbb7b7f0c4a38ebbbf84204a329db9f31c4f897`), Java 25 (OpenJDK) |
| RocketPy | 1.13.0 (numpy 2.5.4, scipy 1.18.1, pandas 3.0.6, Python 3.12) |
| JPype | 1.7.1 (OpenRocket is driven headless through its Java API) |

`doc/data/xcheck_tools/provenance.json` records the exact versions, the download URL and the run date.

## Setup (once, needs the network)

```sh
cargo build --release                       # target/release/ignisyeet
doc/scripts/xcheck_tools/setup.sh ~/xcheck-tools
export OPENROCKET_JAR=~/xcheck-tools/jar/OpenRocket-24.12.jar PY=~/xcheck-tools/venv/bin/python
```

The jar and the virtual environment stay outside the repository. The Python package `orhelper` is not used:
it targets the Java package names of OpenRocket before 2022 (`net.sf.openrocket`); `or_session.py` starts the
JVM and the application directly with JPype.

## Run (no network)

```sh
doc/scripts/xcheck_tools/run_all.sh ~/xcheck-tools/work
```

This takes a few minutes and runs, in order:

| script | what it does |
| --- | --- |
| `run_ignisyeet.sh` | `ignisyeet geom` and `ignisyeet sim` for four cases: no wind / constant 4 m/s wind from 270 deg, each with `extra_cd = 0` and with the sample's `extra_cd = 0.02` |
| `run_openrocket.py` | builds the rocket (`build_openrocket_model.py`), writes `sample_rocket.ork`, computes the static coefficients (alpha = 0 and 1 deg, Mach 0.02 to 3) and flies the two cases to apogee for the fin sections `rounded`, `square`, `airfoil` (and `rounded` with a time step of 0.002 s) |
| `make_drag_inputs.py` | the drag curves that RocketPy is fed with: `drag_ign.csv` (IgnisYeet, power off and on) and `drag_or.csv` (OpenRocket's total drag coefficient, `rounded` fins) |
| `run_rocketpy.py` | builds the rocket, evaluates its Barrowman coefficients and flies both cases to apogee, once with each drag curve |
| `make_comparison.py` | `static_aero.csv`, `components_or.csv` (OpenRocket's nose, body-tube and fin contributions), `flight_nowind.csv`, `flight_wind.csv`, `alpha_wind.csv`, `summary.csv`, `sensitivity.csv` |
| `check_environment.py` | `environment.csv`: air density and gravity of the three programs |
| `run_basic_finner.py` | `basic_finner.csv`: static coefficients of the Basic Finner in OpenRocket and RocketPy next to the measured values and IgnisYeet's |
| `write_provenance.py` | `provenance.json` |

The figures are drawn by `doc/scripts/make_plots.py` from the committed files (no OpenRocket or RocketPy needed).

## What is shared and what is not

Shared by construction (read from the repository files): body diameter and length, the four trapezoidal fins
(root and tip chord, span, sweep, thickness, position, from `geometry.json`), the nose-cone shape (a tangent ogive of
0.300 m; its profile deviates from `profile.csv` by 0.003 mm rms), dry mass 8 kg, dry CG 0.85 m, `ixx_dry`, `iyy_dry`,
the thrust curve and propellant mass of `sample_motor.eng` (motor casing mass moved into the dry mass:
`sample_motor_nocase.eng`), the rail (5 m, 85 deg elevation, azimuth 270 deg), launch site (latitude, longitude,
altitude 20 m), the standard atmosphere, the constant 4 m/s wind from 270 deg, a flat Earth, ascent only.

Independent: the aerodynamic model (drag build-up, normal force, centre of pressure), the pitch damping, the
gravity model, the integrator, the rail-exit rule, the treatment of the thrust at the start and the end of the
burn. RocketPy has no drag model; it is fed with a curve (see `run_rocketpy.py`).

OpenRocket's nose-cone, mass and fin inputs: the nose cone, body tube and fins have a mass of 0; the rocket's mass,
CG and moments of inertia come from four point masses on the body tube (`build_openrocket_model.py`). `extra_cd`
(drag of launch lugs) has no counterpart in OpenRocket's model of this rocket and in RocketPy's, so the cases
with `extra_cd = 0` are the ones compared.

## Notes

- OpenRocket's results change in the sixth digit from run to run (apogee 4875.169 and 4875.172 m in two runs); the tables quote one decimal.
- `sample_rocket.ork` is for inspection. It refers to the motor by its digest, so OpenRocket's GUI shows the
  motor only after `sample_motor_nocase.eng` is imported. The scripts do not load it; they build the rocket in
  memory.
- OpenRocket's fin cross sections are `square`, `rounded` and `airfoil`; IgnisYeet's sample has a rounded leading
  edge and a square trailing edge, which OpenRocket cannot express. `rounded` is used as the main variant and the
  other two bracket it (`sensitivity.csv`).
- RocketPy's reported `max_acceleration` contains a spike at the burn-out step; `make_comparison.py` takes the
  maximum of the acceleration before burn-out of all three programs instead.
- The nose-cone fit: `numpy`-fit of a tangent ogive of length L and radius 0.05 m to `profile.csv` (`ignisyeet geom`);
  L = 0.300 m gives 0.003 mm rms, a parabola 0.22 mm, a von Karman nose 1.7 mm.
