# IgnisYeet research

| Study | Status | Report | Paper |
|---|---|---|---|
| NASA WIND V&V of the aerodynamic methods | written; what it did not obtain is listed below | `../doc/source/research/nasa-vv/` (Japanese) | `nasa_vv/report/en/` |

## nasa_vv

Applies the NASA Glenn WIND/NPARC verification and validation tutorial
(https://www.grc.nasa.gov/www/wind/valid/tutorial/tutorial.html, following
AIAA G-077-1998) to the IgnisYeet aerodynamic methods, in three layers:

- Layer 0: SU2 run directly with IgnisYeet's numerics on problems that have an exact
  solution (wedge shock, Taylor-Maccoll cone, Prandtl-Meyer expansion, laminar flat plate),
  and the turbulent flat plate of the product's documentation.
- Layer 1: the 10 degree cone-cylinder through the whole `ignisyeet aero` pipeline.
- Layer 2: the sample rocket and the Basic Finner, with the tutorial's six steps (program
  check, iterative convergence, consistency, spatial convergence, temporal convergence,
  model uncertainty and validation against the DREV-TM-9703 measurements).

Design: `../docs/superpowers/specs/2026-10-17-nasa-cfd-vv-design.md`
Plan: `../docs/superpowers/plans/2026-10-17-nasa-cfd-vv.md`

### Status

The Japanese report and the English paper draft are written. The report's numbers come
from the files in `nasa_vv/results/`, from the product's data under `../doc/data/`, or from
the product's documentation; the report names the source next to each table. The report
states what the study did not obtain (its section 5), and so does the paper:

- The grid convergence index (GCI) of the Basic Finner RANS case (Mach 1.832) was not
  obtained: the alpha = 2 degree solution does not converge on any of the three levels.
- The GCI of layer 1 was not obtained: the finest grid does not converge, so every row is
  refused, so the layer 1 wall pressure coefficient has no band. It is plotted against
  Taylor-Maccoll for each level (`nasa_vv/figures/layer1_cp.svg`), with the fine level marked
  as not accepted.
- The Basic Finner comparison with the measurements
  (`nasa_vv/figures/validation_bands.svg`) has a CFD band only as a grid-sensitivity box for
  the Euler CNalpha at Mach 2.375, not an established GCI; there is no band for RANS or for CX0.
- The product's grids are not geometrically similar, so a GCI on them depends on the
  definition of the grid size and is reported for two definitions.
- Temporal convergence does not apply to the steady CFD; the 6-DOF integrator is studied
  instead.

### Results files

All files are in `nasa_vv/results/`. The comment lines (`#`) at the top of each file state
its sources and conventions. A producer is run with
`cd research && uv run --project nasa_vv python -m nasa_vv.cases.<module>`; the producers
that read CFD output take `--work <directory>`.

| File | Holds | Producer |
|---|---|---|
| `layer0.csv` | Wedge, cone and expansion on three nested grids: values, difference from the exact solution, residual drop, wall entropy and other diagnostics | `layer0` |
| `layer0_gci.csv` | Layer 0 observed order, extrapolated value, GCI, true error and whether the GCI bounded it (laminar plate rows included) | `layer0`, `layer0_plate` |
| `layer0_plate.csv` | Laminar flat plate at Mach 0.2 against Blasius: Cf, displacement and momentum thickness, leading edge, domain sensitivity, residual drop | `layer0_plate` |
| `cone10_reference_check.csv` | NPARC cone10 table against Taylor-Maccoll, value by value | no producer module; the Taylor-Maccoll column is `nasa_vv.exact.taylor_maccoll` |
| `cone_shock_angle_sources.csv` | Independent sources for the shock angle of the 10 degree cone at Mach 2.35 | no producer module; compared in `tests/test_exact.py` |
| `massflux.csv` | Far-field mass-flux balance of the layer 0 cases | `massflux` |
| `layer1_cone.csv` | Cone-cylinder through the product pipeline: wall Cp per grid, difference from exact, solver verdict, state of the surface output files | `layer1_cone` |
| `layer1_gci.csv` | Layer 1 GCI rows (all refused) with the reason | `layer1_cone` |
| `layer0_vs_layer1.csv` | Layer 0 against layer 1 errors for the cone (the pipeline's contribution is not evaluated) | `layer1_cone` |
| `gci_existing.csv` | GCI of the studies already in `../doc/data`: turbulent flat plate (with its differences from the TMR value and the empirical correlations), sample rocket RANS, Basic Finner Euler | `existing_data` |
| `sample_euler_levels.csv` | Sample rocket Euler, three grid levels and the non-mirrored medium level: solutions, residual drops, verdicts | `sample_euler` |
| `gci_sample_euler.csv` | GCI of the sample rocket Euler grid family at Mach 0.3, 0.8 and 2 | `sample_euler` |
| `basic_finner_rans_levels.csv` | Basic Finner RANS at Mach 1.832, three grid levels: solutions, residual drops, the product's verdicts | `basic_finner_rans` |
| `gci_basic_finner_rans.csv` | Basic Finner RANS GCI rows (all refused) with the reason | `basic_finner_rans` |
| `iterative.csv` | Iterative error against the GCI, per quantity | `iterative` |
| `consistency.csv` | CN(0) = 0, antisymmetry in alpha and the d'Alembert checks | `consistency` |
| `temporal.csv` | Step-size convergence of the 6-DOF integrator and the control experiments | `temporal` |
| `model_uncertainty.csv` | Euler against RANS-SA for the same body and condition, set against both GCIs | `model_uncertainty` |
| `turbulence_models.csv` | Turbulent flat plate under Spalart-Allmaras and Menter SST on SU2, and the comparison with the TMR CFL3D and FUN3D values | `turbulence_models` |

`massflux` writes its check line (the extra analysed markers did not change the solve) only
when it is given the layer 0 runs to compare with:
`python -m nasa_vv.cases.massflux --reference ~/.cache/nasa_vv_layer0`.

The Basic Finner measurements and the product's comparison table are read from
`../doc/data/validation/basic_finner/measured.csv` and `summary_vs_measured.csv`.

### Build and run

Tests:

    cd research && uv run --project nasa_vv --group dev python -m pytest nasa_vv/tests/ -q

Japanese report page (the Sphinx page `doc/source/research/nasa-vv/index.rst`, generated from
`nasa_vv/report/ja/report.md`; `markdown-it-py` is in the `dev` group). The script also copies the two
figures into `doc/source/_static/research/nasa-vv/`, and the page's styles are in
`doc/source/_static/research-report.css`:

    cd research && uv run --project nasa_vv --group dev python nasa_vv/report/ja/build_page.py

English paper draft (`nasa_vv/report/en/main.pdf`; needs `latexmk` and `rsvg-convert` or
`inkscape`, which convert the SVG figures to PDF):

    cd research/nasa_vv/report/en && make

The two figures (`nasa_vv/figures/`; `build_page.py` copies them for the report page):

    cd research && uv run --project nasa_vv python -m nasa_vv.plots

SU2 output, meshes and the CFD work directories are not committed. The layer 0 producers
run SU2 under `~/.cache`; the layer 1 and layer 2 producers collect CFD that was run
separately (on the lab cluster for the three-dimensional cases).
