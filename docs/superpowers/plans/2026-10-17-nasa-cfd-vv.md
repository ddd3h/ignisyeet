# NASA WIND V&V Study Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Put a numerical uncertainty band on IgnisYeet's aerodynamic coefficients by running the NASA WIND/NPARC verification and validation procedure, and publish the result as a Japanese report page and an English paper draft.

**Architecture:** A Python package under `research/nasa_vv/` holds exact solutions and a tested grid-convergence module. The grid-convergence code is lifted unchanged in substance from `doc/scripts/rans_mesh_study.py:43`, which already reproduces NASA's published example, and is given the tests, the constant-ratio path, and the asymptotic ratio it lacks. Three layers of cases feed it: SU2 alone on problems with exact solutions, the IgnisYeet pipeline on a cone-cylinder with the same exact solution, and the full tutorial applied to the sample rocket and the Basic Finner. Most of layer 0 and parts of layer 2 reuse results already committed in `doc/data/`.

**Tech Stack:** Python 3.10+ with numpy and pytest (new dev dependency), SU2 8.3.0 at `~/.local/share/ignisyeet-su2-native/bin/SU2_CFD`, gmsh through IgnisYeet's CFD environment, the `ignisyeet` release binary, PBS/Torque on the virgo cluster for the heavy runs.

**Spec:** `docs/superpowers/specs/2026-10-17-nasa-cfd-vv-design.md`

## Global Constraints

- Verification, cross-comparison and validation are three different words with three different meanings. Comparison against CFL3D or FUN3D is cross-comparison, never validation. Only comparison against measured data is validation. Spec §2.
- Do not rerun a calculation that already exists in `doc/data/`. Every task that produces numbers opens by reading the existing CSV or rst it extends. Spec §2.5.
- Do not write `Co-Authored-By`, `Generated with Claude Code`, or any other AI attribution trailer in a commit message. The user's memory rule forbids them.
- Do not `git push`. The user authorises each push separately.
- Representative grid size is `h = N**(-1/3)` with N the node count, and the flow domain extent is held fixed across a grid family. Only cell sizes scale. Spec §6.4.
- The factor of safety is 1.25 for three or more grids and 3.0 for two. Spec §7.1.
- The asymptotic range ratio is `r21**p * GCI21 / GCI32`, the direction NASA's `verify.f90:120` uses. Spec §7.2.
- Never use the NPARC `cone10` values for `p3/p1`, `T3/T1`, `rho3/rho1` or the shock angle `27.1843` as a test target; that table contradicts itself. Spec §4.0.
- A blank `probable_error_pct` means the error is unknown: draw no error bar. A value of `0` means below 0.5 per cent, not zero. Spec §6.7.
- Mesh files, SU2 solution files and raw solver output are never committed. Only small CSVs, figures and source are. Spec §8.
- Scratch space goes on disk, never in `/tmp`, which is RAM-backed here.
- Every command in this plan runs from `research/`, as `uv run --project nasa_vv ...`.
  The package is not installed (`package = false`), so `import nasa_vv` only resolves
  when `research/` is the working directory. Running from `research/nasa_vv` fails with
  `ModuleNotFoundError: No module named 'nasa_vv'`.
- No module may open a file by a path relative to the working directory. Resolve every
  input and output path from `pathlib.Path(__file__)`, so a module behaves the same
  wherever it is run from. Paths written on a command line are relative to `research/`.
- SU2 binary: `~/.local/share/ignisyeet-su2-native/bin/SU2_CFD`. The `ignisyeet` release binary: `target/release/ignisyeet`.

## Review Focus

These are the failure modes the spec implies but no single task's happy path exercises. Each line names the task that owns its test.

1. **The fixed-point iteration for the observed order does not converge, or converges to a negative or absurd `p`.** `rans_mesh_study.py:43` iterates 200 times with no convergence test and would return whatever it last held. A grid triplet from a shock-dominated supersonic case can do this. Expected: the module reports non-convergence rather than returning a number that looks valid. Test owned by Task 2.
2. **Oscillatory convergence is silently dropped.** The existing code returns `order=None, monotone=False` when `e32/e21 <= 0`, so a caller that only reads `order` records nothing and the report loses the case. Expected: the result is flagged, not discarded, and the report states it. Test owned by Task 2.
3. **A grid family that is not geometrically similar is fed to the module anyway.** The existing 6-level Euler series holds `fin_size` at 1 mm. Nothing in the data marks it. Expected: the case metadata records every size knob, and a family whose knobs did not scale together is refused. Test owned by Task 7.
4. **An unconverged SU2 case is read as if it were converged.** The Basic Finner RANS at M=2.375 reached its iteration limit in a limit cycle and still wrote a history file. Expected: a case whose residual drop is below the stated threshold is excluded from the GCI with its residual drop recorded. Test owned by Task 7.
5. **An error bar is drawn where the measurement error is unknown.** `make_plots.py:1610` does this today with `fillna(0.0)`. Expected: blank means no bar; `0` means a 0.5 per cent bound drawn distinctly. Test owned by Task 12.

---

## Task 1: Package skeleton and test harness

**Files:**
- Create: `research/README.md`
- Create: `research/nasa_vv/__init__.py`
- Create: `research/nasa_vv/pyproject.toml`
- Create: `research/.gitignore`
- Test: `research/nasa_vv/tests/test_smoke.py`

**Interfaces:**
- Consumes: nothing.
- Produces: an importable package `nasa_vv`, and `pytest` runnable from `research/` via `uv run --project nasa_vv --group dev python -m pytest nasa_vv/tests/`.

- [ ] **Step 1: Create the gitignore**

`research/.gitignore`:

```gitignore
# Meshes, solutions and raw solver output are never committed.
*.su2
*.cgns
*.vtk
*.vtu
*.dat
*.p2dfmt
*.fmt
*.stl
*.step
work/
runs/
__pycache__/
.pytest_cache/
```

- [ ] **Step 2: Create the package metadata**

`research/nasa_vv/pyproject.toml`:

```toml
[project]
name = "ignisyeet-nasa-vv"
version = "0.1.0"
description = "NASA WIND V&V study of the IgnisYeet aerodynamic methods"
requires-python = ">=3.10"
license = "GPL-3.0-or-later"
dependencies = ["numpy>=1.26", "matplotlib>=3.8"]

[dependency-groups]
dev = ["pytest>=8.0"]

[tool.uv]
package = false

[tool.pytest.ini_options]
testpaths = ["tests"]
```

- [ ] **Step 3: Create the package init**

`research/nasa_vv/__init__.py`:

```python
# SPDX-License-Identifier: GPL-3.0-or-later
"""NASA WIND V&V study of the IgnisYeet aerodynamic methods."""
```

- [ ] **Step 4: Write the smoke test**

`research/nasa_vv/tests/test_smoke.py`:

```python
# SPDX-License-Identifier: GPL-3.0-or-later
import nasa_vv


def test_package_imports():
    assert nasa_vv.__doc__
```

- [ ] **Step 5: Run the test to verify it passes**

```bash
cd research && uv run --project nasa_vv --group dev python -m pytest nasa_vv/tests/ -v
```

Expected: `test_package_imports PASSED`. If `nasa_vv` is not importable, add `pythonpath = [".."]` under `[tool.pytest.ini_options]` and rerun.

- [ ] **Step 6: Write the research index**

`research/README.md`:

```markdown
# IgnisYeet research

| Study | Status | Report |
|---|---|---|
| NASA WIND V&V of the aerodynamic methods | in progress | `nasa_vv/report/ja/` |

## nasa_vv

Applies the NASA Glenn WIND/NPARC verification and validation tutorial
(https://www.grc.nasa.gov/www/wind/valid/tutorial/tutorial.html, following
AIAA G-077-1998) to the IgnisYeet aerodynamic methods.

Design: `../docs/superpowers/specs/2026-10-17-nasa-cfd-vv-design.md`
Plan: `../docs/superpowers/plans/2026-10-17-nasa-cfd-vv.md`

Run the tests with:

    cd research && uv run --project nasa_vv --group dev python -m pytest nasa_vv/tests/ -v
```

- [ ] **Step 7: Commit**

```bash
git add research/README.md research/.gitignore research/nasa_vv/
git commit -m "research(nasa-vv): add the study package skeleton"
```

---

## Task 2: Grid convergence module

**Files:**
- Create: `research/nasa_vv/gci.py`
- Test: `research/nasa_vv/tests/test_gci.py`
- Test: `research/nasa_vv/tests/data/prD.out` (NASA VERIFY reference output, copied verbatim)
- Read for reference: `doc/scripts/rans_mesh_study.py:43-56`

**Interfaces:**
- Consumes: nothing.
- Produces:
  - `GridConvergence` dataclass with fields `order: float | None`, `extrapolated: float | None`, `gci_fine: float | None`, `gci_coarse: float | None`, `asymptotic_ratio: float | None`, `e_ext_fine: float | None`, `r21: float`, `r32: float`, `e21: float`, `e32: float`, `monotone: bool`, `converged: bool`, `note: str`
  - `grid_convergence(f1, f2, f3, h1, h2, h3, fs=1.25, max_iter=200, tol=1e-12) -> GridConvergence` — variable-ratio Celik (2008) form; `f1`/`h1` finest
  - `constant_ratio(f1, f2, f3, r, fs=1.25) -> GridConvergence` — the tutorial's closed form
  - `representative_h(nodes: int) -> float` — returns `nodes ** (-1/3)`

Note on sign conventions used by every later task: `e21 = f2 - f1` and `e32 = f3 - f2`; `gci_fine` and `gci_coarse` are **fractions**, not percentages; multiply by 100 when reporting.

- [ ] **Step 1: Save the NASA reference output as test data**

The file is already downloaded at
`/tmp/claude-1000/-home-ddd3h-ignisyeet/06700a50-1db7-42f5-aa2f-84e1466af7a3/scratchpad/nparc/prD.out`.
If it is gone, re-fetch it:

```bash
mkdir -p research/nasa_vv/tests/data
curl -s -o research/nasa_vv/tests/data/prD.out \
  https://www.grc.nasa.gov/www/wind/valid/tutorial/prD.out
```

Confirm it contains `Order of Convergence, p =  1.78618479`.

- [ ] **Step 2: Write the failing acceptance test**

`research/nasa_vv/tests/test_gci.py`:

```python
# SPDX-License-Identifier: GPL-3.0-or-later
"""The NASA VERIFY example of the WIND V&V tutorial, plus the degenerate cases.

Input and reference output: https://www.grc.nasa.gov/www/wind/valid/tutorial/
(prD.do, prD.out). verify.f90 declares REAL, so it is single precision and its
printed values differ from the double-precision ones in the fifth digit.
"""
import math
import pathlib

import pytest

from nasa_vv.gci import constant_ratio, grid_convergence, representative_h

# prD.do, finest first
PR = (0.97050, 0.96854, 0.96178)


def test_constant_ratio_matches_the_tutorial_double_precision():
    g = constant_ratio(*PR, r=2.0, fs=1.25)
    assert g.order == pytest.approx(1.786170, abs=5e-6)
    assert g.extrapolated == pytest.approx(0.97130, abs=5e-6)
    assert g.gci_fine * 100 == pytest.approx(0.103083, abs=5e-6)
    assert g.gci_coarse * 100 == pytest.approx(0.356249, abs=5e-6)
    assert g.asymptotic_ratio == pytest.approx(0.997984, abs=5e-6)
    assert g.monotone and g.converged


def test_constant_ratio_matches_the_single_precision_verify_output():
    """verify.f90 is single precision; allow 1e-4 relative."""
    g = constant_ratio(*PR, r=2.0, fs=1.25)
    assert g.order == pytest.approx(1.78618479, rel=1e-4)
    assert g.extrapolated == pytest.approx(0.971300304, rel=1e-4)
    assert g.gci_fine * 100 == pytest.approx(0.103080, rel=1e-4)
    assert g.gci_coarse * 100 == pytest.approx(0.356244, rel=1e-4)
    assert g.asymptotic_ratio == pytest.approx(0.997980, rel=1e-4)


def test_variable_ratio_reduces_to_the_constant_ratio_result():
    """h = 1, 2, 4 makes r21 = r32 = 2, so q(p) = 0 and the two paths agree."""
    v = grid_convergence(*PR, h1=1.0, h2=2.0, h3=4.0, fs=1.25)
    c = constant_ratio(*PR, r=2.0, fs=1.25)
    assert v.order == pytest.approx(c.order, abs=1e-9)
    assert v.extrapolated == pytest.approx(c.extrapolated, abs=1e-12)
    assert v.gci_fine == pytest.approx(c.gci_fine, abs=1e-12)


def test_oscillatory_convergence_is_flagged_not_dropped():
    """e32 / e21 < 0. The result must still carry the ratios and be labelled."""
    g = grid_convergence(1.00, 1.02, 1.00, h1=1.0, h2=1.4, h3=2.0)
    assert g.monotone is False
    assert "oscillatory" in g.note
    assert g.e21 == pytest.approx(0.02)
    assert g.e32 == pytest.approx(-0.02)
    assert g.order is None
    assert g.gci_fine is None


def test_zero_difference_between_the_two_finest_grids():
    g = grid_convergence(1.00, 1.00, 1.02, h1=1.0, h2=1.4, h3=2.0)
    assert g.order is None
    assert g.gci_fine is None
    assert "no change" in g.note


def test_non_convergent_fixed_point_is_reported():
    """r21 just above 1 makes ln(r21) tiny and the iteration wander."""
    g = grid_convergence(1.0, 1.0000001, 1.5, h1=1.0, h2=1.0000001, h3=2.0,
                         max_iter=5)
    assert g.converged is False
    assert g.order is None
    assert "did not converge" in g.note


def test_two_grid_factor_of_safety_is_three():
    g = constant_ratio(*PR, r=2.0, fs=3.0)
    c = constant_ratio(*PR, r=2.0, fs=1.25)
    assert g.gci_fine == pytest.approx(c.gci_fine * 3.0 / 1.25)


def test_representative_h():
    assert representative_h(1000) == pytest.approx(0.1)
    assert representative_h(8) / representative_h(1) == pytest.approx(0.5)


def test_verify_reference_file_is_the_one_we_pinned_to():
    text = (pathlib.Path(__file__).parent / "data" / "prD.out").read_text()
    assert "1.78618479" in text
    assert "0.971300304" in text
    assert "0.103080" in text and "0.356244" in text
    assert "0.997980" in text
```

- [ ] **Step 3: Run the test to verify it fails**

```bash
cd research && uv run --project nasa_vv --group dev python -m pytest nasa_vv/tests/test_gci.py -v
```

Expected: collection error, `ModuleNotFoundError: No module named 'nasa_vv.gci'`.

- [ ] **Step 4: Write the module**

`research/nasa_vv/gci.py`:

```python
# SPDX-License-Identifier: GPL-3.0-or-later
"""Richardson extrapolation and the Grid Convergence Index.

Constant refinement ratio: the NASA WIND V&V tutorial,
https://www.grc.nasa.gov/www/wind/valid/tutorial/spatconv.html

Variable refinement ratio: Celik, Ghia, Roache, Freitas, Coleman and Raad,
"Procedure for Estimation and Reporting of Uncertainty Due to Discretization
in CFD Applications", J. Fluids Eng. 130 (2008) 078001.

The asymptotic range ratio follows the direction of NASA's verify.f90 line 120,
ratio = r21**p * GCI21 / GCI32, so a value near 1 means the asymptotic range.
"""
from __future__ import annotations

import math
from dataclasses import dataclass


@dataclass
class GridConvergence:
    """One grid triplet. f1/h1 is the finest. GCI values are fractions."""

    r21: float
    r32: float
    e21: float
    e32: float
    order: float | None = None
    extrapolated: float | None = None
    gci_fine: float | None = None
    gci_coarse: float | None = None
    asymptotic_ratio: float | None = None
    e_ext_fine: float | None = None
    monotone: bool = False
    converged: bool = False
    note: str = ""


def representative_h(nodes: int) -> float:
    """Representative grid size from a node count.

    SU2 is vertex centred, so its control volumes correspond to the nodes. With
    the flow domain held fixed this gives the same ratios as (sum dV / N)**(1/3).
    """
    return float(nodes) ** (-1.0 / 3.0)


def _finish(g: GridConvergence, f1: float, f2: float, f3: float, fs: float) -> GridConvergence:
    """Fill the derived quantities once the order is known."""
    p = g.order
    assert p is not None
    g.extrapolated = (g.r21**p * f1 - f2) / (g.r21**p - 1.0)
    g.e_ext_fine = abs((g.extrapolated - f1) / g.extrapolated)
    g.gci_fine = fs * abs(g.e21 / f1) / (g.r21**p - 1.0)
    g.gci_coarse = fs * abs(g.e32 / f2) / (g.r32**p - 1.0)
    g.asymptotic_ratio = g.r21**p * g.gci_fine / g.gci_coarse
    return g


def grid_convergence(
    f1: float, f2: float, f3: float,
    h1: float, h2: float, h3: float,
    fs: float = 1.25, max_iter: int = 200, tol: float = 1e-12,
) -> GridConvergence:
    """Celik (2008), for a refinement ratio that need not be constant."""
    r21, r32 = h2 / h1, h3 / h2
    e21, e32 = f2 - f1, f3 - f2
    g = GridConvergence(r21=r21, r32=r32, e21=e21, e32=e32)

    if e21 == 0.0 or e32 == 0.0:
        g.note = "no change between two of the grids; the order is undefined"
        return g
    if e32 / e21 < 0.0:
        g.monotone = False
        g.note = "oscillatory convergence: the differences change sign"
        return g

    g.monotone = True
    s = math.copysign(1.0, e32 / e21)
    p = 2.0
    for _ in range(max_iter):
        q = math.log((r21**p - s) / (r32**p - s))
        nxt = abs(math.log(abs(e32 / e21)) + q) / math.log(r21)
        if not math.isfinite(nxt):
            g.note = "the fixed point iteration left the finite range"
            return g
        if abs(nxt - p) < tol:
            p = nxt
            g.converged = True
            break
        p = nxt
    if not g.converged:
        g.note = f"the fixed point iteration did not converge in {max_iter} steps"
        return g
    if p <= 0.0:
        g.note = f"the observed order is not positive ({p:.4g})"
        return g

    g.order = p
    return _finish(g, f1, f2, f3, fs)


def constant_ratio(
    f1: float, f2: float, f3: float, r: float, fs: float = 1.25
) -> GridConvergence:
    """The tutorial's closed form, valid when the refinement ratio is constant."""
    e21, e32 = f2 - f1, f3 - f2
    g = GridConvergence(r21=r, r32=r, e21=e21, e32=e32)

    if e21 == 0.0 or e32 == 0.0:
        g.note = "no change between two of the grids; the order is undefined"
        return g
    if e32 / e21 < 0.0:
        g.note = "oscillatory convergence: the differences change sign"
        return g

    g.monotone = True
    g.converged = True
    g.order = math.log(e32 / e21) / math.log(r)
    return _finish(g, f1, f2, f3, fs)
```

- [ ] **Step 5: Run the tests to verify they pass**

```bash
cd research && uv run --project nasa_vv --group dev python -m pytest nasa_vv/tests/test_gci.py -v
```

Expected: all nine tests PASS. If `test_non_convergent_fixed_point_is_reported` fails because the iteration happens to converge, widen the triplet until `ln(r21)` is small enough to make it wander, and keep the assertion on `converged is False`.

- [ ] **Step 6: Commit**

```bash
git add research/nasa_vv/gci.py research/nasa_vv/tests/test_gci.py research/nasa_vv/tests/data/prD.out
git commit -m "research(nasa-vv): add the grid convergence module, pinned to NASA's VERIFY example"
```

---

## Task 3: Celik variable-ratio test from the paper

**Files:**
- Modify: `research/nasa_vv/tests/test_gci.py`

**Interfaces:**
- Consumes: `grid_convergence` from Task 2.
- Produces: nothing new.

The NASA example has `r21 == r32`, so `q(p) == 0` and the variable-ratio branch is
never exercised against an independent reference. Celik et al. (2008) Table 1 gives
a worked example with a non-constant ratio.

- [ ] **Step 1: Obtain the Celik Table 1 numbers**

Do **not** write them from memory. Fetch the paper and read Table 1. It is widely
mirrored; the ASME page is
`https://asmedigitalcollection.asme.org/fluidsengineering/article/130/7/078001/444689`.
A commonly mirrored copy of the same procedure with the worked example is the
"Procedure for Estimation and Reporting of Uncertainty Due to Discretization in CFD
Applications" editorial PDF.

Record in the test docstring: which table, which column, and the URL used.

The example to look for is the turbulent flow over a backward facing step, with
`N1, N2, N3 = 18000, 8000, 4500`, `r21 = 1.5`, `r32 = 1.333`, and the reported
`p`, `phi_ext21`, `e_a21`, `e_ext21`, `GCI_fine21` for the reattachment length.

- [ ] **Step 2: Write the failing test**

Append to `research/nasa_vv/tests/test_gci.py`, filling the five `CELIK_*` constants
from what Step 1 actually read:

```python
# Celik et al. (2008) J. Fluids Eng. 130, 078001, Table 1.
# Source read: <URL>, Table 1, column <name>.
CELIK_N = (18000, 8000, 4500)
CELIK_F = (6.063, 5.972, 5.863)       # phi1, phi2, phi3
CELIK_P = ...                          # reported p
CELIK_EXT = ...                        # reported phi_ext21
CELIK_GCI = ...                        # reported GCI_fine21 in per cent


def test_variable_ratio_matches_the_celik_paper_example():
    h = [n ** (-1 / 2) for n in CELIK_N]   # the paper's 2-D case uses area
    g = grid_convergence(*CELIK_F, h1=h[0], h2=h[1], h3=h[2], fs=1.25)
    assert g.r21 == pytest.approx(1.5, abs=5e-3)
    assert g.r32 == pytest.approx(1.333, abs=5e-3)
    assert g.order == pytest.approx(CELIK_P, abs=5e-3)
    assert g.extrapolated == pytest.approx(CELIK_EXT, abs=5e-4)
    assert g.gci_fine * 100 == pytest.approx(CELIK_GCI, abs=5e-3)
```

- [ ] **Step 3: Run it and reconcile**

```bash
cd research && uv run --project nasa_vv --group dev python -m pytest nasa_vv/tests/test_gci.py::test_variable_ratio_matches_the_celik_paper_example -v
```

If it fails, the discrepancy is information, not a licence to bend the module.
Check in this order: whether the paper's `h` is the 2-D `(sum dA / N)**(1/2)` or
the 3-D form; whether the paper rounds `r32` to 1.333 from 1.3333; whether the
quantity is the one whose `p` is quoted. Only if the module itself is wrong may
`gci.py` change, and then every Task 2 test must still pass.

- [ ] **Step 4: Commit**

```bash
git add research/nasa_vv/tests/test_gci.py
git commit -m "research(nasa-vv): pin the variable-ratio path to the Celik paper example"
```

---

## Task 4: Exact solutions — oblique shock and Prandtl-Meyer

**Files:**
- Create: `research/nasa_vv/exact.py`
- Test: `research/nasa_vv/tests/test_exact.py`
- Read for reference: `crates/panel/src/supersonic.rs:37,50,71,77,88`

**Interfaces:**
- Consumes: nothing.
- Produces:
  - `oblique_shock(mach, delta_deg, gamma=1.4) -> ObliqueShock` with fields `beta_deg`, `mach2`, `p2_p1`, `rho2_rho1`, `t2_t1`, `pt2_pt1`
  - `prandtl_meyer_nu(mach, gamma=1.4) -> float` (degrees)
  - `prandtl_meyer_expansion(mach, turn_deg, gamma=1.4) -> Expansion` with fields `mach2`, `p2_p1`, `rho2_rho1`, `t2_t1`
  - `isentropic_pt_p(mach, gamma=1.4) -> float`
  - `isentropic_tt_t(mach, gamma=1.4) -> float`

- [ ] **Step 1: Write the failing test with the NPARC reference values**

`research/nasa_vv/tests/test_exact.py`:

```python
# SPDX-License-Identifier: GPL-3.0-or-later
"""Exact solutions against the NPARC validation archive's published values.

Wedge: https://www.grc.nasa.gov/www/wind/valid/wedge/wedge.html Table 2
Prandtl-Meyer: https://www.grc.nasa.gov/www/wind/valid/pm15/pm15.html Table 7

The cone10 archive page is deliberately NOT used as a reference here; its table
is internally inconsistent (spec section 4.0).
"""
import pytest

from nasa_vv.exact import (
    isentropic_pt_p,
    isentropic_tt_t,
    oblique_shock,
    prandtl_meyer_expansion,
    prandtl_meyer_nu,
)


def test_oblique_shock_matches_the_nparc_wedge_case():
    """15 degree wedge at M = 2.5, NPARC wedge Table 2."""
    s = oblique_shock(2.5, 15.0)
    assert s.beta_deg == pytest.approx(36.94490, abs=1e-4)
    assert s.mach2 == pytest.approx(1.873526, abs=1e-5)
    assert s.p2_p1 == pytest.approx(2.467500, abs=1e-5)
    assert s.rho2_rho1 == pytest.approx(1.866549, abs=1e-5)
    assert s.t2_t1 == pytest.approx(1.321958, abs=1e-5)


def test_prandtl_meyer_matches_the_nparc_pm15_case():
    """15 degree expansion from M = 2.5, NPARC pm15 Table 7."""
    e = prandtl_meyer_expansion(2.5, 15.0)
    assert e.mach2 == pytest.approx(3.2368, abs=5e-4)
    assert e.p2_p1 == pytest.approx(0.3274, abs=5e-4)
    assert e.t2_t1 == pytest.approx(0.7269, abs=5e-4)
    assert e.rho2_rho1 == pytest.approx(0.4505, abs=5e-4)


def test_prandtl_meyer_nu_known_values():
    """nu(1) = 0 and the textbook value at M = 2."""
    assert prandtl_meyer_nu(1.0) == pytest.approx(0.0, abs=1e-9)
    assert prandtl_meyer_nu(2.0) == pytest.approx(26.3798, abs=1e-3)


def test_zero_deflection_gives_the_mach_angle():
    """The weak solution degenerates to a Mach wave, asin(1/M) = 30 deg at M = 2."""
    s = oblique_shock(2.0, 1e-9)
    assert s.beta_deg == pytest.approx(30.0, abs=1e-3)
    assert s.p2_p1 == pytest.approx(1.0, abs=1e-5)


def test_oblique_shock_is_detached_above_the_maximum_deflection():
    """At M = 2 the maximum deflection is near 23 degrees."""
    with pytest.raises(ValueError, match="detached"):
        oblique_shock(2.0, 30.0)


def test_isentropic_relations_are_self_consistent():
    """p/pt and T/Tt must satisfy p/pt = (T/Tt)**(g/(g-1))."""
    g = 1.4
    for m in (0.2, 0.8, 1.0, 2.35, 3.2):
        assert isentropic_pt_p(m) == pytest.approx(
            isentropic_tt_t(m) ** (g / (g - 1)), rel=1e-12
        )


def test_shock_total_pressure_never_rises():
    """Deflections must stay below each Mach number's maximum attached value.

    At M = 1.05 that maximum is only 0.5581 deg, so 1/5/10 deg are all detached
    there and oblique_shock correctly raises instead of returning a shock.
    """
    cases = {1.05: (0.1, 0.3, 0.5), 1.5: (1.0, 5.0, 10.0),
             2.5: (1.0, 5.0, 10.0), 4.0: (1.0, 5.0, 10.0)}
    for m, deflections in cases.items():
        for d in deflections:
            assert oblique_shock(m, d).pt2_pt1 <= 1.0
```

- [ ] **Step 2: Run the test to verify it fails**

```bash
cd research && uv run --project nasa_vv --group dev python -m pytest nasa_vv/tests/test_exact.py -v
```

Expected: `ModuleNotFoundError: No module named 'nasa_vv.exact'`.

- [ ] **Step 3: Write the module**

`research/nasa_vv/exact.py`:

```python
# SPDX-License-Identifier: GPL-3.0-or-later
"""Exact compressible-flow solutions used as verification references.

These are independent of the product: nothing here imports or calls IgnisYeet.
Formulas: Anderson, Modern Compressible Flow, and NACA Report 1135.
"""
from __future__ import annotations

import math
from dataclasses import dataclass


@dataclass
class ObliqueShock:
    beta_deg: float
    mach2: float
    p2_p1: float
    rho2_rho1: float
    t2_t1: float
    pt2_pt1: float


@dataclass
class Expansion:
    mach2: float
    p2_p1: float
    rho2_rho1: float
    t2_t1: float


def isentropic_tt_t(mach: float, gamma: float = 1.4) -> float:
    """Tt / T."""
    return 1.0 + 0.5 * (gamma - 1.0) * mach * mach


def isentropic_pt_p(mach: float, gamma: float = 1.4) -> float:
    """pt / p."""
    return isentropic_tt_t(mach, gamma) ** (gamma / (gamma - 1.0))


def _deflection(mach: float, beta: float, gamma: float) -> float:
    """theta(beta) of the theta-beta-M relation, in radians."""
    mn1sq = (mach * math.sin(beta)) ** 2
    return math.atan(
        2.0 / math.tan(beta) * (mn1sq - 1.0)
        / (mach * mach * (gamma + math.cos(2.0 * beta)) + 2.0)
    )


def oblique_shock(mach: float, delta_deg: float, gamma: float = 1.4) -> ObliqueShock:
    """The weak solution of the theta-beta-M relation, by bisection on beta."""
    if mach <= 1.0:
        raise ValueError("oblique_shock needs a supersonic free stream")
    delta = math.radians(delta_deg)
    lo = math.asin(1.0 / mach)          # Mach angle: theta = 0
    # The maximum deflection lies between the Mach angle and 90 degrees.
    n = 2000
    best_beta, best_theta = lo, 0.0
    for i in range(1, n + 1):
        b = lo + (0.5 * math.pi - lo) * i / n
        th = _deflection(mach, b, gamma)
        if th > best_theta:
            best_theta, best_beta = th, b
    if delta > best_theta:
        raise ValueError(
            f"the shock is detached: delta {delta_deg} deg exceeds the maximum "
            f"{math.degrees(best_theta):.4f} deg at M = {mach}"
        )
    hi = best_beta
    for _ in range(200):
        mid = 0.5 * (lo + hi)
        if _deflection(mach, mid, gamma) < delta:
            lo = mid
        else:
            hi = mid
    beta = 0.5 * (lo + hi)

    mn1 = mach * math.sin(beta)
    mn1sq = mn1 * mn1
    p2_p1 = 1.0 + 2.0 * gamma / (gamma + 1.0) * (mn1sq - 1.0)
    rho2_rho1 = (gamma + 1.0) * mn1sq / ((gamma - 1.0) * mn1sq + 2.0)
    t2_t1 = p2_p1 / rho2_rho1
    mn2 = math.sqrt(
        (1.0 + 0.5 * (gamma - 1.0) * mn1sq) / (gamma * mn1sq - 0.5 * (gamma - 1.0))
    )
    theta = _deflection(mach, beta, gamma)
    mach2 = mn2 / math.sin(beta - theta) if beta > theta else mn2
    pt2_pt1 = (
        rho2_rho1 ** (gamma / (gamma - 1.0))
        * (1.0 / p2_p1) ** (1.0 / (gamma - 1.0))
    )
    return ObliqueShock(math.degrees(beta), mach2, p2_p1, rho2_rho1, t2_t1, pt2_pt1)


def prandtl_meyer_nu(mach: float, gamma: float = 1.4) -> float:
    """The Prandtl-Meyer function, in degrees."""
    if mach < 1.0:
        raise ValueError("the Prandtl-Meyer function needs M >= 1")
    if mach == 1.0:
        return 0.0
    b = math.sqrt((gamma + 1.0) / (gamma - 1.0))
    msq = mach * mach - 1.0
    return math.degrees(
        b * math.atan(math.sqrt(msq / (b * b))) - math.atan(math.sqrt(msq))
    )


def prandtl_meyer_expansion(
    mach: float, turn_deg: float, gamma: float = 1.4
) -> Expansion:
    """Isentropic turn through turn_deg; solves nu(M2) = nu(M1) + turn."""
    target = prandtl_meyer_nu(mach, gamma) + turn_deg
    lo, hi = 1.0, 100.0
    for _ in range(300):
        mid = 0.5 * (lo + hi)
        if prandtl_meyer_nu(mid, gamma) < target:
            lo = mid
        else:
            hi = mid
    m2 = 0.5 * (lo + hi)
    t2_t1 = isentropic_tt_t(mach, gamma) / isentropic_tt_t(m2, gamma)
    p2_p1 = isentropic_pt_p(mach, gamma) / isentropic_pt_p(m2, gamma)
    return Expansion(m2, p2_p1, p2_p1 / t2_t1, t2_t1)
```

- [ ] **Step 4: Run the tests to verify they pass**

```bash
cd research && uv run --project nasa_vv --group dev python -m pytest nasa_vv/tests/test_exact.py -v
```

Expected: all seven PASS. If `test_zero_deflection_gives_the_mach_angle` fails at the Mach-angle edge, the bisection's lower bound is exactly `asin(1/M)` where the deflection is zero; nudge the assertion tolerance rather than the bound.

- [ ] **Step 5: Commit**

```bash
git add research/nasa_vv/exact.py research/nasa_vv/tests/test_exact.py
git commit -m "research(nasa-vv): add the oblique shock and Prandtl-Meyer exact solutions"
```

---

## Task 5: Exact solutions — Taylor-Maccoll and the cone10 discrepancy

**Files:**
- Modify: `research/nasa_vv/exact.py`
- Modify: `research/nasa_vv/tests/test_exact.py`
- Create: `research/nasa_vv/results/cone10_reference_check.csv`
- Read for reference: `doc/scripts/rans_cone.py:19-68`

**Interfaces:**
- Consumes: `oblique_shock`, `isentropic_pt_p`, `isentropic_tt_t` from Task 4.
- Produces: `taylor_maccoll(mach, delta_deg, gamma=1.4) -> Cone` with fields `cp`, `beta_deg`, `mach_surface`, `p_pinf`, `t_tinf`, `rho_rhoinf`.

The existing `doc/scripts/rans_cone.py:19` already solves this and returns
`(cp, beta_deg, mach_surface)`. Port it, add the thermodynamic ratios, and do not
change its numerics; it must keep reproducing `M = 2 -> Cp 0.104471, beta 31.2061,
Mc 1.834028` and `M = 2.35 -> Cp 0.096731, beta 26.7367, Mc 2.146831`.

- [ ] **Step 1: Write the failing test**

Append to `research/nasa_vv/tests/test_exact.py`:

```python
from nasa_vv.exact import taylor_maccoll


def test_taylor_maccoll_reproduces_the_existing_repository_values():
    """doc/scripts/rans_cone.py must keep giving the same answer after the port."""
    c2 = taylor_maccoll(2.0, 10.0)
    assert c2.cp == pytest.approx(0.104471, abs=1e-5)
    assert c2.beta_deg == pytest.approx(31.2061, abs=1e-3)
    assert c2.mach_surface == pytest.approx(1.834028, abs=1e-5)

    c235 = taylor_maccoll(2.35, 10.0)
    assert c235.cp == pytest.approx(0.096731, abs=1e-5)
    assert c235.beta_deg == pytest.approx(26.7367, abs=1e-3)
    assert c235.mach_surface == pytest.approx(2.146831, abs=1e-5)


def test_taylor_maccoll_agrees_with_the_nparc_surface_mach():
    """NPARC cone10 states M3 = 2.1469; that one entry is consistent with us."""
    assert taylor_maccoll(2.35, 10.0).mach_surface == pytest.approx(2.1469, abs=5e-4)


def test_cone_surface_state_is_thermodynamically_consistent():
    """Behind a conical shock the flow is isentropic, so pt_surface = pt2 <= pt1.

    This is what the NPARC cone10 table fails (spec section 4.0).
    """
    c = taylor_maccoll(2.35, 10.0)
    # pt is uniform behind the conical shock and never rises across it, so
    # p_surface / p_inf is bounded by the loss-free value.
    bound = isentropic_pt_p(2.35) / isentropic_pt_p(c.mach_surface)
    assert c.p_pinf <= bound * (1.0 + 1e-9)
    assert c.t_tinf == pytest.approx(
        isentropic_tt_t(2.35) / isentropic_tt_t(c.mach_surface), rel=1e-9
    )
    assert c.rho_rhoinf == pytest.approx(c.p_pinf / c.t_tinf, rel=1e-9)


def test_the_nparc_cone10_table_is_internally_inconsistent():
    """Pin the discrepancy so the report's claim stays true if anything changes."""
    c = taylor_maccoll(2.35, 10.0)
    assert c.p_pinf == pytest.approx(1.3739, abs=1e-3)   # NPARC prints 1.4234
    assert c.t_tinf == pytest.approx(1.0951, abs=1e-3)   # NPARC prints 1.1063
    assert c.rho_rhoinf == pytest.approx(1.2546, abs=1e-3)  # NPARC prints 1.2867
    assert abs(c.beta_deg - 27.1843) > 0.4               # NPARC prints 27.1843
```

- [ ] **Step 2: Run the test to verify it fails**

```bash
cd research && uv run --project nasa_vv --group dev python -m pytest nasa_vv/tests/test_exact.py -k taylor -v
```

Expected: `ImportError: cannot import name 'taylor_maccoll'`.

- [ ] **Step 3: Port the solver**

Append to `research/nasa_vv/exact.py`:

```python
@dataclass
class Cone:
    cp: float
    beta_deg: float
    mach_surface: float
    p_pinf: float
    t_tinf: float
    rho_rhoinf: float


def taylor_maccoll(mach: float, delta_deg: float, gamma: float = 1.4) -> Cone:
    """Sharp cone with an attached shock; shooting on the shock angle.

    Ported from doc/scripts/rans_cone.py:19 with the numerics unchanged, then
    extended with the thermodynamic ratios at the cone surface.
    """
    g = gamma
    delta = math.radians(delta_deg)

    def cone_angle(beta: float) -> tuple[float, float]:
        """Returns (theta_cone, M_surface) for a given shock angle."""
        mn1 = mach * math.sin(beta)
        mn2 = math.sqrt(
            (1 + 0.5 * (g - 1) * mn1**2) / (g * mn1**2 - 0.5 * (g - 1))
        )
        theta = math.atan(
            2 / math.tan(beta) * (mn1**2 - 1)
            / (mach**2 * (g + math.cos(2 * beta)) + 2)
        )
        m2 = mn2 / math.sin(beta - theta)
        v = 1.0 / math.sqrt(2.0 / ((g - 1) * m2**2) + 1.0)   # V / V_max
        vr, vt = v * math.cos(beta - theta), -v * math.sin(beta - theta)

        def f(th, y):
            vr, vt = y
            a2 = 0.5 * (g - 1) * (1 - vr * vr - vt * vt)
            dvt = (vt * vt * vr - a2 * (2 * vr + vt / math.tan(th))) / (a2 - vt * vt)
            return (vt, dvt)

        th, y, h = beta, (vr, vt), -1e-5
        while y[1] < 0:
            k1 = f(th, y)
            k2 = f(th + 0.5 * h, (y[0] + 0.5 * h * k1[0], y[1] + 0.5 * h * k1[1]))
            k3 = f(th + 0.5 * h, (y[0] + 0.5 * h * k2[0], y[1] + 0.5 * h * k2[1]))
            k4 = f(th + h, (y[0] + h * k3[0], y[1] + h * k3[1]))
            y = (
                y[0] + h / 6 * (k1[0] + 2 * k2[0] + 2 * k3[0] + k4[0]),
                y[1] + h / 6 * (k1[1] + 2 * k2[1] + 2 * k3[1] + k4[1]),
            )
            th += h
            if th <= 1e-6:
                break
        vmag = math.hypot(y[0], y[1])
        a2 = 0.5 * (g - 1) * (1 - vmag * vmag)
        return th, vmag / math.sqrt(a2)

    lo, hi = math.asin(1.0 / mach) + 1e-6, 0.5 * math.pi - 1e-6
    for _ in range(200):
        mid = 0.5 * (lo + hi)
        if cone_angle(mid)[0] < delta:
            lo = mid
        else:
            hi = mid
    beta = 0.5 * (lo + hi)
    _, mc = cone_angle(beta)

    # pt is uniform behind the conical shock and the flow to the surface is
    # isentropic, so the surface state follows from M_surface and the shock loss.
    mn1 = mach * math.sin(beta)
    mn1sq = mn1 * mn1
    p2_p1 = 1.0 + 2.0 * g / (g + 1.0) * (mn1sq - 1.0)
    rho2_rho1 = (g + 1.0) * mn1sq / ((g - 1.0) * mn1sq + 2.0)
    pt2_pt1 = (
        rho2_rho1 ** (g / (g - 1.0)) * (1.0 / p2_p1) ** (1.0 / (g - 1.0))
    )
    p_pinf = pt2_pt1 * isentropic_pt_p(mach, g) / isentropic_pt_p(mc, g)
    t_tinf = isentropic_tt_t(mach, g) / isentropic_tt_t(mc, g)
    cp = (p_pinf - 1.0) / (0.5 * g * mach * mach)
    return Cone(cp, math.degrees(beta), mc, p_pinf, t_tinf, p_pinf / t_tinf)
```

- [ ] **Step 4: Run the tests to verify they pass**

```bash
cd research && uv run --project nasa_vv --group dev python -m pytest nasa_vv/tests/test_exact.py -v
```

Expected: all PASS, including the four new ones.

- [ ] **Step 5: Record the cone10 discrepancy as data**

Write `research/nasa_vv/results/cone10_reference_check.csv` by running:

```bash
cd research && uv run --project nasa_vv python - <<'PY'
import csv, pathlib
from nasa_vv.exact import taylor_maccoll
c = taylor_maccoll(2.35, 10.0)
nparc = {"beta_deg": 27.1843, "mach_surface": 2.1469, "p_pinf": 1.4234,
         "t_tinf": 1.1063, "rho_rhoinf": 1.2867}
rows = [{"quantity": k, "nparc_cone10": v, "taylor_maccoll": getattr(c, k),
         "difference": getattr(c, k) - v,
         "difference_pct": (getattr(c, k) - v) / v * 100} for k, v in nparc.items()]
p = pathlib.Path("nasa_vv/results"); p.mkdir(parents=True, exist_ok=True)
with open(p / "cone10_reference_check.csv", "w", newline="") as f:
    f.write("# NPARC cone10 (https://www.grc.nasa.gov/www/wind/valid/cone10/cone10.html)\n")
    f.write("# against nasa_vv.exact.taylor_maccoll at M = 2.35, 10 deg half angle, gamma = 1.4.\n")
    f.write("# Only mach_surface agrees. See the spec, section 4.0.\n")
    w = csv.DictWriter(f, fieldnames=list(rows[0])); w.writeheader(); w.writerows(rows)
print(open(p / "cone10_reference_check.csv").read())
PY
```

Check the printed table: `mach_surface` must differ by under 0.02 per cent and the
other four by more than 1 per cent.

- [ ] **Step 6: Commit**

```bash
git add research/nasa_vv/exact.py research/nasa_vv/tests/test_exact.py research/nasa_vv/results/cone10_reference_check.csv
git commit -m "research(nasa-vv): port Taylor-Maccoll and record the cone10 reference discrepancy"
```

---

## Task 6: Confirm the cone shock angle against an independent source

**Files:**
- Create: `research/nasa_vv/results/cone_shock_angle_sources.csv`
- Modify: `research/nasa_vv/tests/test_exact.py`

**Interfaces:**
- Consumes: `taylor_maccoll` from Task 5.
- Produces: nothing new in code; a committed CSV of the independent values.

The module and NPARC disagree on the shock angle by 0.45 degrees. One of them is
wrong, and the report cannot say which without a third source.

- [ ] **Step 1: Gather at least two independent values**

For a 10 degree half-angle cone at M = 2.35 and gamma = 1.4, obtain the shock angle
from sources that are not this module and not NPARC. Use at least two of:

- NACA Report 1135, the conical-flow charts (Chart 5 / Table II).
- The Rust implementation in the product: `crates/panel/src/supersonic.rs:164`,
  reached through `cone_cp`. Run it and print the internal beta. This is not fully
  independent (it is the same physics) but it is a separate implementation.
- A published Taylor-Maccoll table, for example Sims, NASA SP-3004.

Record the source, the value, and how it was read.

- [ ] **Step 2: Write the CSV**

`research/nasa_vv/results/cone_shock_angle_sources.csv`:

```csv
# Shock angle of a 10 degree half-angle cone at M = 2.35, gamma = 1.4.
# Collected to settle the disagreement between nasa_vv.exact.taylor_maccoll
# (26.7367 deg) and the NPARC cone10 page (27.1843 deg). See spec section 4.0.
source,reference,beta_deg,how_read,note
nasa_vv.exact,research/nasa_vv/exact.py,26.7367,computed,shooting on beta to theta_c = 10 deg
nparc_cone10,https://www.grc.nasa.gov/www/wind/valid/cone10/cone10.html,27.1843,table text,inconsistent with its own M3
...,...,...,...,...
```

- [ ] **Step 3: Pin the conclusion in a test**

Append to `research/nasa_vv/tests/test_exact.py`, with `INDEPENDENT_BETA` set to what
Step 1 actually found:

```python
# The shock angle from a source independent of this module and of NPARC.
# Source: <name>, <reference>, read as <how>.
INDEPENDENT_BETA = ...


def test_cone_shock_angle_agrees_with_an_independent_source():
    assert taylor_maccoll(2.35, 10.0).beta_deg == pytest.approx(
        INDEPENDENT_BETA, abs=0.05
    )
```

If the independent source instead supports 27.1843, this module is wrong. Stop,
report it, and do not proceed to Task 9; the Taylor-Maccoll port and the existing
`doc/scripts/rans_cone.py` would both need correcting, which is outside this plan.

- [ ] **Step 4: Run the tests**

```bash
cd research && uv run --project nasa_vv --group dev python -m pytest nasa_vv/tests/test_exact.py -v
```

- [ ] **Step 5: Commit**

```bash
git add research/nasa_vv/results/cone_shock_angle_sources.csv research/nasa_vv/tests/test_exact.py
git commit -m "research(nasa-vv): settle the cone shock angle against an independent source"
```

---

## Task 7: Grid family bookkeeping

**Files:**
- Create: `research/nasa_vv/family.py`
- Test: `research/nasa_vv/tests/test_family.py`

**Interfaces:**
- Consumes: `representative_h`, `grid_convergence`, `GridConvergence` from Task 2.
- Produces:
  - `GridLevel` dataclass: `name: str`, `nodes: int`, `knobs: dict[str, float]`, `residual_drop_orders: float | None`, `converged: bool`
  - `GridFamily` dataclass: `levels: list[GridLevel]` (finest first), with
    `check_similar(self, rtol=0.02) -> None` raising `ValueError` naming the knob that
    did not scale, and `h(self) -> list[float]`
  - `study(family: GridFamily, values: dict[str, list[float]], fs=1.25) -> dict[str, GridConvergence]`

This is where Review Focus items 3 and 4 are enforced.

- [ ] **Step 1: Write the failing test**

`research/nasa_vv/tests/test_family.py`:

```python
# SPDX-License-Identifier: GPL-3.0-or-later
import pytest

from nasa_vv.family import GridFamily, GridLevel, study


def _level(name, nodes, wall, fin, drop=6.0, ok=True):
    return GridLevel(name=name, nodes=nodes,
                     knobs={"wall_size": wall, "fin_size": fin},
                     residual_drop_orders=drop, converged=ok)


def test_a_similar_family_passes_the_check():
    f = GridFamily([_level("fine", 1_840_000, 3.0, 1.0),
                    _level("medium", 787_000, 4.8, 1.6),
                    _level("coarse", 348_000, 8.0, 2.667)])
    f.check_similar()


def test_a_family_that_held_one_knob_fixed_is_refused():
    """The existing six-level Euler series held fin_size at 1 mm."""
    f = GridFamily([_level("fine", 1_840_000, 1.8, 1.0),
                    _level("medium", 787_000, 3.0, 1.0),
                    _level("coarse", 348_000, 6.0, 1.0)])
    with pytest.raises(ValueError, match="fin_size"):
        f.check_similar()


def test_an_unconverged_level_is_refused():
    f = GridFamily([_level("fine", 1_840_000, 3.0, 1.0, drop=0.6, ok=False),
                    _level("medium", 787_000, 4.8, 1.6),
                    _level("coarse", 348_000, 8.0, 2.667)])
    with pytest.raises(ValueError, match="fine"):
        f.check_similar()


def test_levels_must_be_ordered_finest_first():
    f = GridFamily([_level("coarse", 348_000, 8.0, 2.667),
                    _level("medium", 787_000, 4.8, 1.6),
                    _level("fine", 1_840_000, 3.0, 1.0)])
    with pytest.raises(ValueError, match="finest first"):
        f.check_similar()


def test_study_returns_one_result_per_quantity():
    f = GridFamily([_level("fine", 1_840_000, 3.0, 1.0),
                    _level("medium", 787_000, 4.8, 1.6),
                    _level("coarse", 348_000, 8.0, 2.667)])
    out = study(f, {"cn": [1.2021, 1.2055, 1.2140], "ca": [0.5990, 0.6012, 0.6080]})
    assert set(out) == {"cn", "ca"}
    assert out["cn"].monotone
    assert out["cn"].order is not None
    assert out["cn"].r21 > 1.0
```

- [ ] **Step 2: Run the test to verify it fails**

```bash
cd research && uv run --project nasa_vv --group dev python -m pytest nasa_vv/tests/test_family.py -v
```

Expected: `ModuleNotFoundError: No module named 'nasa_vv.family'`.

- [ ] **Step 3: Write the module**

`research/nasa_vv/family.py`:

```python
# SPDX-License-Identifier: GPL-3.0-or-later
"""A family of grids for one grid convergence study, and its preconditions.

A GCI is only meaningful when the grids are geometrically similar, which means
every cell-size knob scaled by the same factor and the flow domain stayed fixed,
and when every level actually converged.
"""
from __future__ import annotations

from dataclasses import dataclass, field

from .gci import GridConvergence, grid_convergence, representative_h

MIN_RESIDUAL_DROP_ORDERS = 3.0
"""The NASA tutorial asks for three to four orders before calling a run converged."""


@dataclass
class GridLevel:
    name: str
    nodes: int
    knobs: dict[str, float] = field(default_factory=dict)
    residual_drop_orders: float | None = None
    converged: bool = True


@dataclass
class GridFamily:
    levels: list[GridLevel]
    """Finest first."""

    def h(self) -> list[float]:
        return [representative_h(lv.nodes) for lv in self.levels]

    def check_similar(self, rtol: float = 0.02) -> None:
        if len(self.levels) < 3:
            raise ValueError("a grid convergence study needs at least three levels")
        nodes = [lv.nodes for lv in self.levels]
        if nodes != sorted(nodes, reverse=True):
            raise ValueError(
                f"the levels must be ordered finest first; got node counts {nodes}"
            )
        for lv in self.levels:
            if not lv.converged:
                raise ValueError(
                    f"level {lv.name!r} did not converge "
                    f"(residual drop {lv.residual_drop_orders} orders); "
                    "it cannot enter a grid convergence study"
                )
            if (lv.residual_drop_orders is not None
                    and lv.residual_drop_orders < MIN_RESIDUAL_DROP_ORDERS):
                raise ValueError(
                    f"level {lv.name!r} dropped only {lv.residual_drop_orders} "
                    f"orders, below the {MIN_RESIDUAL_DROP_ORDERS} the tutorial asks for"
                )
        names = set(self.levels[0].knobs)
        for lv in self.levels[1:]:
            if set(lv.knobs) != names:
                raise ValueError(
                    f"level {lv.name!r} records knobs {sorted(lv.knobs)}, "
                    f"not {sorted(names)}"
                )
        # Every knob must scale by the same factor between consecutive levels.
        for a, b in zip(self.levels, self.levels[1:]):
            factors = {k: b.knobs[k] / a.knobs[k] for k in names}
            ref = next(iter(factors.values()))
            for k, v in factors.items():
                if abs(v - ref) > rtol * ref:
                    raise ValueError(
                        f"{k!r} scaled by {v:.4f} between {a.name!r} and {b.name!r} "
                        f"while the others scaled by about {ref:.4f}; "
                        "the grids are not geometrically similar"
                    )


def study(
    family: GridFamily, values: dict[str, list[float]], fs: float = 1.25
) -> dict[str, GridConvergence]:
    """One GridConvergence per quantity. Values are finest first, like the levels."""
    family.check_similar()
    h = family.h()
    if len(h) != 3:
        h = h[:3]
    out: dict[str, GridConvergence] = {}
    for name, v in values.items():
        if len(v) < 3:
            raise ValueError(f"quantity {name!r} needs three values, got {len(v)}")
        out[name] = grid_convergence(v[0], v[1], v[2], h[0], h[1], h[2], fs=fs)
    return out
```

- [ ] **Step 4: Run the tests to verify they pass**

```bash
cd research && uv run --project nasa_vv --group dev python -m pytest nasa_vv/tests/test_family.py -v
```

Expected: all five PASS.

- [ ] **Step 5: Commit**

```bash
git add research/nasa_vv/family.py research/nasa_vv/tests/test_family.py
git commit -m "research(nasa-vv): refuse grid families that are not similar or not converged"
```

---

## Task 8: Grid convergence from the data already committed

**Files:**
- Create: `research/nasa_vv/cases/existing_data.py`
- Create: `research/nasa_vv/results/gci_existing.csv`
- Test: `research/nasa_vv/tests/test_existing_data.py`
- Read: `doc/data/cfd/rans_mesh_study.csv`, `doc/data/cfd/rans_mesh_study.json`, `doc/data/cfd/rans_flatplate.csv`, `doc/data/validation/basic_finner/mesh_check.csv`

**Interfaces:**
- Consumes: `GridFamily`, `GridLevel`, `study` from Task 7.
- Produces: `main()` writing `results/gci_existing.csv`; readers
  `read_rans_mesh_study(path) -> dict[float, GridFamily]` keyed by Mach,
  `read_flatplate(path) -> GridFamily`, `read_basic_finner_mesh_check(path) -> dict`.

This task computes GCI for three studies **without running any new CFD**, from data
already in the repository. Spec §2.5 and §4.

- [ ] **Step 1: Look at the three source files before writing any code**

```bash
head -3 doc/data/cfd/rans_mesh_study.csv
head -3 doc/data/cfd/rans_flatplate.csv
head -3 doc/data/validation/basic_finner/mesh_check.csv
```

Record the exact column names in the module docstring. Do not guess them.

- [ ] **Step 2: Write the failing test**

`research/nasa_vv/tests/test_existing_data.py`:

```python
# SPDX-License-Identifier: GPL-3.0-or-later
"""Reads the committed CSVs. Paths are relative to the repository root."""
import pathlib

import pytest

from nasa_vv.cases.existing_data import (
    read_basic_finner_mesh_check,
    read_flatplate,
    read_rans_mesh_study,
)

ROOT = pathlib.Path(__file__).resolve().parents[3]


def test_rans_mesh_study_gives_three_levels_per_mach():
    fams = read_rans_mesh_study(ROOT / "doc/data/cfd/rans_mesh_study.csv")
    assert set(fams) == {0.8, 2.0}
    for m, f in fams.items():
        assert len(f.levels) == 3
        f.check_similar()
        assert f.levels[0].nodes > f.levels[-1].nodes


def test_flatplate_family_has_a_refinement_ratio_of_two():
    f = read_flatplate(ROOT / "doc/data/cfd/rans_flatplate.csv")
    h = f.h()
    assert h[1] / h[0] == pytest.approx(2.0, rel=0.05)
    assert h[2] / h[1] == pytest.approx(2.0, rel=0.05)


def test_basic_finner_has_four_euler_levels():
    d = read_basic_finner_mesh_check(
        ROOT / "doc/data/validation/basic_finner/mesh_check.csv"
    )
    assert len(d["levels"]) == 4
```

- [ ] **Step 3: Run the test to verify it fails**

```bash
cd research && uv run --project nasa_vv --group dev python -m pytest nasa_vv/tests/test_existing_data.py -v
```

Expected: `ModuleNotFoundError: No module named 'nasa_vv.cases'`.

- [ ] **Step 4: Write the readers and the driver**

Create `research/nasa_vv/cases/__init__.py` containing only the SPDX line, then
`research/nasa_vv/cases/existing_data.py`. Write the three readers against the column
names recorded in Step 1, and a `main()` that:

1. For `rans_mesh_study.csv`, builds a `GridFamily` per Mach from `wall_size_m` and the
   node count, and calls `study` for `cn`, `ca`, `xcp`.
2. For `rans_flatplate.csv`, builds a family from `h` and `cf_x097` at M = 0.2 and calls
   `constant_ratio` with `r = 2.0`, since the TMR grids double exactly.
3. For `mesh_check.csv`, builds **two** triplets, L1-L3 and L2-L4, and calls `study` on
   each. Agreement between their observed orders is itself a check; write both.
4. Writes one row per (study, condition, quantity, triplet) to
   `results/gci_existing.csv` with columns:
   `study,condition,quantity,triplet,n1,n2,n3,r21,r32,f1,f2,f3,order,extrapolated,gci_fine_pct,gci_coarse_pct,asymptotic_ratio,monotone,note`.
5. Writes a header comment naming every source file and the commit it read.

Each `main()` run must be reproducible: no timestamps in the CSV body.

- [ ] **Step 5: Run the tests to verify they pass**

```bash
cd research && uv run --project nasa_vv --group dev python -m pytest nasa_vv/tests/test_existing_data.py -v
```

- [ ] **Step 6: Produce the results and read them**

```bash
cd research && uv run --project nasa_vv python -m nasa_vv.cases.existing_data
column -s, -t results/gci_existing.csv | head -40
```

Check three things before committing:
- The RANS sample at M = 0.8 and 2.0 reproduces the GCI already published in
  `doc/source/verification.rst:947-983`. If it does not, the reader is wrong.
- The Basic Finner L1-L3 and L2-L4 orders are compared in the output.
- Any oscillatory or non-converged entry carries its `note` instead of a blank.

- [ ] **Step 7: Commit**

```bash
git add research/nasa_vv/cases/ research/nasa_vv/tests/test_existing_data.py research/nasa_vv/results/gci_existing.csv
git commit -m "research(nasa-vv): compute grid convergence from the committed results"
```

---

## Task 9: Layer 0 — SU2 configuration pinned to the product's numerics

**Files:**
- Create: `research/nasa_vv/su2cfg.py`
- Create: `research/nasa_vv/tests/data/golden_euler.cfg`
- Test: `research/nasa_vv/tests/test_su2cfg.py`
- Read: `crates/cfd/src/su2cfg.rs:183-203`, and its golden tests at `:414`

**Interfaces:**
- Consumes: nothing.
- Produces:
  - `NUMERICS_KEYS: tuple[str, ...]` — the keys whose values must match the product
  - `product_numerics(cfg_text: str) -> dict[str, str]`
  - `layer0_config(*, ndim: int, axisymmetric: bool, mach: float, markers: dict, viscous: bool, golden: str) -> str`

The claim "layer 0 used IgnisYeet's settings" is itself a claim under test. Spec §4.3.

- [ ] **Step 1: Capture a real product configuration as the golden file**

Run one short Euler case through the product and keep the `.cfg` it wrote:

```bash
cd /home/ddd3h/ignisyeet
cargo build --release
WORK=$(mktemp -d -p "$HOME/.cache")   # not /tmp: it is RAM-backed here
cp examples/sample_cfd.toml "$WORK/case.toml"
target/release/ignisyeet --no-progress aero "$WORK/case.toml" \
  --set aero.cfd.machs='[2.0]' --set aero.cfd.alphas_deg='[0.0, 4.0]' --force || true
find "$WORK" -name '*.cfg' | head
```

Copy the first `.cfg` to `research/nasa_vv/tests/data/golden_euler.cfg`. If the run is
too slow to finish, it only needs to get far enough to write the config; interrupt it
once the file exists.

- [ ] **Step 2: Write the failing test**

`research/nasa_vv/tests/test_su2cfg.py`:

```python
# SPDX-License-Identifier: GPL-3.0-or-later
"""The layer 0 configurations must reuse the product's numerics verbatim.

golden_euler.cfg was written by `ignisyeet aero` itself (see the plan, Task 9).
Only the markers and AXISYMMETRIC may differ. The dimension comes from the mesh file's `NDIME`;
SU2 8.3.0 has no `NDIM` option and aborts on one.
"""
import pathlib

import pytest

from nasa_vv.su2cfg import NUMERICS_KEYS, layer0_config, product_numerics

GOLDEN = (pathlib.Path(__file__).parent / "data" / "golden_euler.cfg").read_text()


def test_the_golden_file_is_a_product_configuration():
    assert "MATH_PROBLEM" in GOLDEN
    assert len(product_numerics(GOLDEN)) == len(NUMERICS_KEYS)


def test_layer0_reuses_every_numerics_key_verbatim():
    cfg = layer0_config(ndim=2, axisymmetric=False, mach=2.5,
                        markers={"wall": ["wedge"], "far": ["farfield"]},
                        viscous=False, golden=GOLDEN)
    got, want = product_numerics(cfg), product_numerics(GOLDEN)
    assert got == want


def test_layer0_may_differ_only_in_dimension_and_markers():
    cfg = layer0_config(ndim=2, axisymmetric=True, mach=2.35,
                        markers={"wall": ["cone"], "far": ["farfield"]},
                        viscous=False, golden=GOLDEN)
    assert not any(l.strip().startswith("NDIM") for l in cfg.splitlines())
    assert "AXISYMMETRIC= YES" in cfg
    assert "cone" in cfg


def test_a_tampered_numerics_key_is_caught():
    bad = GOLDEN.replace("CFL_NUMBER", "CFL_NUMBER_DISABLED", 1)
    with pytest.raises(KeyError):
        product_numerics(bad)
```

- [ ] **Step 3: Run the test to verify it fails**

```bash
cd research && uv run --project nasa_vv --group dev python -m pytest nasa_vv/tests/test_su2cfg.py -v
```

Expected: `ModuleNotFoundError: No module named 'nasa_vv.su2cfg'`.

- [ ] **Step 4: Write the module**

Read `golden_euler.cfg` and `crates/cfd/src/su2cfg.rs:183` together, then set
`NUMERICS_KEYS` to the keys that actually appear in the golden file and govern the
numerics. Start from this list and drop any the golden file does not contain:

```python
NUMERICS_KEYS = (
    "CONV_NUM_METHOD_FLOW", "MUSCL_FLOW", "SLOPE_LIMITER_FLOW",
    "VENKAT_LIMITER_COEFF", "NUM_METHOD_GRAD", "NUM_METHOD_GRAD_RECON",
    "CFL_NUMBER", "CFL_ADAPT", "CFL_ADAPT_PARAM",
    "LINEAR_SOLVER", "LINEAR_SOLVER_PREC", "LINEAR_SOLVER_ERROR",
    "LINEAR_SOLVER_ITER", "CONV_FIELD", "CONV_RESIDUAL_MINVAL",
    "TIME_DISCRE_FLOW", "ENTROPY_FIX_COEFF",
)
```

`product_numerics` parses `KEY= VALUE` lines and raises `KeyError` naming any key in
`NUMERICS_KEYS` that is absent. `layer0_config` copies every `NUMERICS_KEYS` line from
`golden` verbatim, then appends the case-specific block: `SOLVER`,
`AXISYMMETRIC`, `MACH_NUMBER`, the marker lines, the reference values, and the mesh
and output filenames.

- [ ] **Step 5: Run the tests to verify they pass**

```bash
cd research && uv run --project nasa_vv --group dev python -m pytest nasa_vv/tests/test_su2cfg.py -v
```

- [ ] **Step 6: Commit**

```bash
git add research/nasa_vv/su2cfg.py research/nasa_vv/tests/test_su2cfg.py research/nasa_vv/tests/data/golden_euler.cfg
git commit -m "research(nasa-vv): pin the layer 0 SU2 numerics to a real product configuration"
```

---

## Task 10: Layer 0 — meshes and the wedge, cone and expansion runs

**Files:**
- Create: `research/nasa_vv/mesh.py`
- Create: `research/nasa_vv/cases/layer0.py`
- Create: `research/nasa_vv/results/layer0.csv`
- Test: `research/nasa_vv/tests/test_mesh.py`
- Read for reference: `doc/scripts/rans_flatplate.py:103-145`, `doc/scripts/rans_plate_bl.py:39-100`

**Interfaces:**
- Consumes: `layer0_config` from Task 9; `oblique_shock`, `prandtl_meyer_expansion`, `taylor_maccoll` from Tasks 4 and 5; `constant_ratio` from Task 2.
- Produces:
  - `structured_quad_su2(path, xs, ys, markers) -> int` writing a 2-D `.su2` and returning the node count
  - `refine(xs, ys, factor=2) -> tuple[list, list]` producing an exactly nested grid
  - `cases/layer0.py` `main()` writing `results/layer0.csv`

- [ ] **Step 1: Write the failing mesh test**

`research/nasa_vv/tests/test_mesh.py`:

```python
# SPDX-License-Identifier: GPL-3.0-or-later
import pathlib

import pytest

from nasa_vv.mesh import refine, structured_quad_su2


def test_a_small_grid_writes_the_expected_counts(tmp_path):
    xs, ys = [0.0, 0.5, 1.0], [0.0, 0.5, 1.0]
    p = tmp_path / "g.su2"
    n = structured_quad_su2(p, xs, ys,
                            {"wall": "ymin", "far": ["ymax", "xmin", "xmax"]})
    text = p.read_text()
    assert n == 9
    assert "NDIME= 2" in text
    assert "NPOIN= 9" in text
    assert "NELEM= 4" in text
    assert "MARKER_TAG= wall" in text


def test_refinement_is_exactly_nested():
    """Every coarse point must survive in the fine grid, so r = 2 exactly."""
    xs, ys = [0.0, 0.25, 0.5, 1.0], [0.0, 0.5, 1.0]
    fx, fy = refine(xs, ys, factor=2)
    assert len(fx) == 2 * len(xs) - 1
    assert len(fy) == 2 * len(ys) - 1
    for v in xs:
        assert any(abs(v - f) < 1e-12 for f in fx)


def test_refining_twice_gives_a_ratio_of_four():
    xs, ys = [0.0, 1.0], [0.0, 1.0]
    once = refine(xs, ys)
    twice = refine(*once)
    assert len(twice[0]) == 5
```

- [ ] **Step 2: Run it to verify it fails**

```bash
cd research && uv run --project nasa_vv --group dev python -m pytest nasa_vv/tests/test_mesh.py -v
```

Expected: `ModuleNotFoundError: No module named 'nasa_vv.mesh'`.

- [ ] **Step 3: Write `research/nasa_vv/mesh.py`**

`structured_quad_su2` writes the SU2 ASCII format: `NDIME= 2`, an `NELEM` block of
quadrilaterals (VTK type `9`, one line per cell as `9 n0 n1 n2 n3 index`), an `NPOIN`
block of `x y index`, then one `MARKER_TAG` block per boundary with `MARKER_ELEMS` and
line elements (VTK type `3`). Index nodes as `j * len(xs) + i`.

`refine(xs, ys, factor=2)` inserts `factor - 1` evenly spaced points between every
consecutive pair, so the coarse points survive exactly.

- [ ] **Step 4: Run the tests to verify they pass**

```bash
cd research && uv run --project nasa_vv --group dev python -m pytest nasa_vv/tests/test_mesh.py -v
```

- [ ] **Step 5: Write the three layer 0 cases**

`research/nasa_vv/cases/layer0.py` with a `main()` that, for each of the three cases
below, builds three nested grids, writes the configs with `layer0_config`, runs
`SU2_CFD`, extracts the comparison quantity, and calls `constant_ratio` with `r = 2.0`.

| case | geometry | domain | M | quantity extracted | exact reference |
|---|---|---|---|---|---|
| `wedge` | 15 deg ramp from the origin | x in [-0.5, 1.0], y in [0, 1.0] | 2.5 | surface `p2/p1`, shock angle from the pressure jump locus | `oblique_shock(2.5, 15.0)` |
| `cone` | 10 deg ramp, `AXISYMMETRIC= YES` | x in [0, 1.0], r in [0, 1.0] | 2.35 | surface `p/pinf`, `Cp`, `M_surface` | `taylor_maccoll(2.35, 10.0)` |
| `pm15` | flat then 15 deg away from the flow | x in [-0.5, 1.0], y in [0, 1.0] | 2.5 | downstream `p2/p1`, `M2` | `prandtl_meyer_expansion(2.5, 15.0)` |

Base grid sizes: 65 x 49, refined to 129 x 97 and 257 x 193.

Run SU2 with:

```bash
~/.local/share/ignisyeet-su2-native/bin/SU2_CFD case.cfg
```

For the cone, compare only `M_surface` and `Cp` against the exact solution, never the
NPARC `p3/p1`, `T3/T1` or `beta` (Global Constraints).

Write `results/layer0.csv` with columns:
`case,mach,level,nodes,quantity,cfd,exact,difference,difference_pct,residual_drop_orders,iterations`
and a second file `results/layer0_gci.csv` with the `constant_ratio` output per case and
quantity, using the same column names as Task 8's `gci_existing.csv`.

- [ ] **Step 6: Run the cases**

```bash
cd research && uv run --project nasa_vv python -m nasa_vv.cases.layer0 --work "$HOME/.cache/nasa_vv_layer0"
column -s, -t results/layer0.csv
```

Check: every level reached at least 3 orders of residual drop; the finest-level
difference from the exact solution is under 2 per cent for the wedge and the expansion.
If a case will not converge, record its residual drop in the CSV and say so in the
report rather than tuning the numerics, which would break Task 9's guarantee.

- [ ] **Step 7: Commit**

```bash
git add research/nasa_vv/mesh.py research/nasa_vv/cases/layer0.py research/nasa_vv/tests/test_mesh.py research/nasa_vv/results/layer0.csv research/nasa_vv/results/layer0_gci.csv
git commit -m "research(nasa-vv): solve the wedge, cone and expansion against their exact solutions"
```

---

## Task 11: Layer 0 — the laminar flat plate

**Files:**
- Modify: `research/nasa_vv/exact.py`
- Modify: `research/nasa_vv/tests/test_exact.py`
- Create: `research/nasa_vv/cases/layer0_plate.py`
- Create: `research/nasa_vv/results/layer0_plate.csv`

**Interfaces:**
- Consumes: `structured_quad_su2`, `refine` from Task 10; `layer0_config` from Task 9.
- Produces: `blasius_cf(re_x) -> float`, `blasius_profile(n=200) -> list[tuple[float, float]]` returning `(eta, f')`.

Run at **M = 0.2**, not the NPARC `fplam` M = 0.1: the product's Roe scheme has no
low-Mach preconditioning, so at M = 0.1 the compressible discretisation error and the
difference from Blasius cannot be separated. Spec §4.

- [ ] **Step 1: Write the failing test**

Append to `research/nasa_vv/tests/test_exact.py`:

```python
from nasa_vv.exact import blasius_cf, blasius_profile


def test_blasius_skin_friction_formula():
    assert blasius_cf(1.0e5) == pytest.approx(0.664 / math.sqrt(1.0e5), rel=1e-12)


def test_blasius_profile_boundary_conditions():
    """f'(0) = 0 and f' -> 1 far from the wall."""
    p = blasius_profile()
    assert p[0][1] == pytest.approx(0.0, abs=1e-9)
    assert p[-1][1] == pytest.approx(1.0, abs=1e-3)


def test_blasius_wall_shear_constant():
    """f''(0) = 0.332057, the value the 0.664 in Cf comes from."""
    p = blasius_profile(n=4000)
    eta1, fp1 = p[1]
    assert fp1 / eta1 == pytest.approx(0.332057, rel=5e-3)
```

Add `import math` at the top of the test file if it is not already there.

- [ ] **Step 2: Run it to verify it fails, then implement**

```bash
cd research && uv run --project nasa_vv --group dev python -m pytest nasa_vv/tests/test_exact.py -k blasius -v
```

Then append to `research/nasa_vv/exact.py` a `blasius_cf` returning `0.664 / sqrt(re_x)`
and a `blasius_profile` that integrates `f''' + 0.5 f f'' = 0` with RK4, shooting on
`f''(0)` until `f'(eta_max) = 1`.

- [ ] **Step 3: Run the tests to verify they pass**

```bash
cd research && uv run --project nasa_vv --group dev python -m pytest nasa_vv/tests/test_exact.py -k blasius -v
```

- [ ] **Step 4: Write and run the case**

`research/nasa_vv/cases/layer0_plate.py`: a flat plate at M = 0.2, laminar
(`SOLVER= NAVIER_STOKES`, no turbulence model), `Re = 1e5` per metre so the plate stays
laminar, three nested grids, comparing `Cf(x)` at `x = 0.5` and the velocity profile at
the same station against `blasius_cf` and `blasius_profile`.

Write `results/layer0_plate.csv` with the same column names as `layer0.csv`, and add the
`constant_ratio` rows to `results/layer0_gci.csv`.

Note in the CSV header that Blasius is the incompressible solution, and record the
compressibility estimate at M = 0.2 so the report can state it.

- [ ] **Step 5: Commit**

```bash
git add research/nasa_vv/exact.py research/nasa_vv/tests/test_exact.py research/nasa_vv/cases/layer0_plate.py research/nasa_vv/results/layer0_plate.csv research/nasa_vv/results/layer0_gci.csv
git commit -m "research(nasa-vv): solve the laminar flat plate against the Blasius solution"
```

---

## Task 12: Validation figure with both uncertainty bands

**Files:**
- Create: `research/nasa_vv/plots.py`
- Create: `research/nasa_vv/figures/validation_bands.svg`
- Test: `research/nasa_vv/tests/test_plots.py`
- Read: `doc/data/validation/basic_finner/summary_vs_measured.csv`, `doc/scripts/make_plots.py:1598-1625`

**Interfaces:**
- Consumes: `results/gci_existing.csv` from Task 8.
- Produces: `error_bar_spec(probable_error_pct: str | float | None, value: float) -> ErrorBar | None` and `validation_figure(...) -> None`.

This is where Review Focus item 5 is enforced. The existing plot uses `fillna(0.0)`,
which draws a zero-length bar where the error is unknown. Spec §6.7.

- [ ] **Step 1: Write the failing test**

`research/nasa_vv/tests/test_plots.py`:

```python
# SPDX-License-Identifier: GPL-3.0-or-later
"""A blank probable error means unknown, and 0 means below 0.5 per cent.

doc/source/validation.rst:580 states the 0 convention. doc/scripts/make_plots.py:1610
uses fillna(0.0) and so draws a zero-length bar for an unknown error; the research
figures must not.
"""
import pytest

from nasa_vv.plots import error_bar_spec


def test_a_blank_probable_error_draws_no_bar():
    assert error_bar_spec("", 18.47) is None
    assert error_bar_spec(None, 18.47) is None


def test_a_numeric_probable_error_scales_with_the_value():
    b = error_bar_spec(2.0, 18.47)
    assert b.half_height == pytest.approx(18.47 * 0.02)
    assert b.bound is False


def test_zero_means_a_half_per_cent_bound_not_zero_error():
    b = error_bar_spec(0, 18.47)
    assert b.half_height == pytest.approx(18.47 * 0.005)
    assert b.bound is True


def test_a_negative_value_uses_its_magnitude():
    b = error_bar_spec(2.0, -52.648)
    assert b.half_height == pytest.approx(52.648 * 0.02)
```

- [ ] **Step 2: Run it to verify it fails**

```bash
cd research && uv run --project nasa_vv --group dev python -m pytest nasa_vv/tests/test_plots.py -v
```

- [ ] **Step 3: Write the module**

`research/nasa_vv/plots.py` with an `ErrorBar` dataclass (`half_height: float`,
`bound: bool`), `error_bar_spec` implementing the three rules above, and
`validation_figure` drawing, for CNa and CX0 against Mach:

- the measured points with their bars, where `bound=True` bars use a distinct cap style
- the CFD points with a GCI band taken from `results/gci_existing.csv`
- no band on a CFD point whose condition has no GCI, with a footnote saying so

- [ ] **Step 4: Run the tests and produce the figure**

```bash
cd research && uv run --project nasa_vv --group dev python -m pytest nasa_vv/tests/test_plots.py -v
cd research && uv run --project nasa_vv python -m nasa_vv.plots --out nasa_vv/figures/validation_bands.svg
```

Open the SVG and confirm by eye: no zero-length bars, the bound bars are visually
distinct, and the CFD bands are present only where a GCI exists.

- [ ] **Step 5: Commit**

```bash
git add research/nasa_vv/plots.py research/nasa_vv/tests/test_plots.py research/nasa_vv/figures/validation_bands.svg
git commit -m "research(nasa-vv): draw both uncertainty bands, and no bar where the error is unknown"
```

---

## Task 13: Consistency checks

**Files:**
- Create: `research/nasa_vv/cases/consistency.py`
- Create: `research/nasa_vv/results/consistency.csv`
- Test: `research/nasa_vv/tests/test_consistency.py`
- Read: `doc/data/cfd/sample_euler_cases.csv`, `doc/data/cfd/asymmetry_study.csv`

**Interfaces:**
- Consumes: nothing from earlier tasks except the repository CSVs.
- Produces: `antisymmetry(cn_plus, cn_minus) -> float` returning the even part
  `(cn_plus + cn_minus) / 2`, and `main()` writing `results/consistency.csv`.

Spec §6.3. Three of the four checks read data that already exists; only `CA(M=0.3)`
against grid refinement needs the new runs of Task 14.

- [ ] **Step 1: Write the failing test**

`research/nasa_vv/tests/test_consistency.py`:

```python
# SPDX-License-Identifier: GPL-3.0-or-later
import pytest

from nasa_vv.cases.consistency import antisymmetry


def test_a_perfectly_antisymmetric_pair_has_a_zero_even_part():
    assert antisymmetry(1.2021, -1.2021) == pytest.approx(0.0)


def test_the_even_part_is_half_the_sum():
    assert antisymmetry(1.20, -1.18) == pytest.approx(0.01)


def test_the_even_part_is_signed():
    assert antisymmetry(1.18, -1.20) == pytest.approx(-0.01)
```

- [ ] **Step 2: Run it to verify it fails, then implement**

```bash
cd research && uv run --project nasa_vv --group dev python -m pytest nasa_vv/tests/test_consistency.py -v
```

Write `research/nasa_vv/cases/consistency.py` with `antisymmetry` and a `main()` that
writes one row per check to `results/consistency.csv` with columns
`check,case,mach,alpha_deg,model,quantity,value,expected,tolerance,passed,source`:

| check | source |
|---|---|
| `cn_zero_alpha` | `doc/data/cfd/asymmetry_study.csv`, the z-symmetric rows |
| `cn_antisymmetry` | `doc/data/cfd/asymmetry_study.csv`, the +4 and -4 pairs |
| `ca_dalembert` | `doc/data/cfd/sample_euler_cases.csv`, the `0.3,0` row; expected 0 |
| `ca_dalembert_refined` | Task 14's output, once it exists; omit the rows until then |

The `source` column must name the file and the row, so every number in the report is
traceable. Spec §13.

- [ ] **Step 3: Run the tests and produce the results**

```bash
cd research && uv run --project nasa_vv --group dev python -m pytest nasa_vv/tests/test_consistency.py -v
cd research && uv run --project nasa_vv python -m nasa_vv.cases.consistency
column -s, -t results/consistency.csv
```

- [ ] **Step 4: Commit**

```bash
git add research/nasa_vv/cases/consistency.py research/nasa_vv/tests/test_consistency.py research/nasa_vv/results/consistency.csv
git commit -m "research(nasa-vv): collect the consistency checks from the committed results"
```

---

## Task 14: New Euler grid family for the sample rocket

**Files:**
- Create: `research/nasa_vv/cases/sample_euler.py`
- Create: `research/nasa_vv/results/sample_euler_levels.csv`
- Modify: `research/nasa_vv/results/gci_existing.csv` (append, or write `gci_sample_euler.csv`)
- Read for reference: `doc/scripts/rans_sample.py:44-200`

**Interfaces:**
- Consumes: `GridFamily`, `GridLevel`, `study` from Task 7.
- Produces: `main()` writing `results/sample_euler_levels.csv` and `results/gci_sample_euler.csv`.

This is the only genuinely new layer 2 compute. The existing six-level Euler series held
`fin_size` at 1 mm and so is not geometrically similar; Task 7's `check_similar` refuses
it by design. Spec §6.4.

- [ ] **Step 1: Size the run before committing to it**

Reuse `doc/scripts/rans_sample.py` through its CLI rather than rewriting the driver.
Scale `wall_size` and `fin_size` together by 1.4 per level:

| level | `wall_size` | `fin_size` |
|---|---|---|
| coarse | 3.92 mm | 1.96 mm |
| medium | 2.80 mm | 1.40 mm |
| fine | 2.00 mm | 1.00 mm |

Run the coarse level first and record its node count, peak memory and wall time:

```bash
cd /home/ddd3h/ignisyeet
WORK="$HOME/.cache/nasa_vv_sample"
mkdir -p "$WORK"
uv run --project doc python doc/scripts/rans_sample.py setup "$WORK/coarse" \
  --machs 0.3 0.8 2.0 --alphas 0 4 --wall 0.00392 --model euler \
  --set aero.cfd.fin_size=0.00196
uv run --project doc python doc/scripts/rans_sample.py run "$WORK/coarse"
```

Confirm `aero.cfd.fin_size` is the right key by checking `crates/cfd/src/config.rs`
before running; if the key differs, use the real one and record it.

- [ ] **Step 2: Decide where the fine level runs**

If the coarse level's peak memory times `1.4**6` exceeds 100 GB, send the medium and
fine levels to virgo:

```bash
/home/ddd3h/ignisyeet-wt/remote/virgo/virgo_run.sh -w -l \
  "$WORK/fine/case.toml" virgo01 64 aero --force
```

Otherwise run them locally. Record which machine each level ran on in the CSV.

- [ ] **Step 3: Solve alpha = -4 on the medium level only**

The antisymmetry check needs a directly solved negative incidence, not a mirrored one.
Spec §6.3.

```bash
uv run --project doc python doc/scripts/rans_sample.py setup "$WORK/medium_minus" \
  --machs 0.3 0.8 2.0 --alphas -4 --wall 0.0028 --model euler \
  --set aero.cfd.fin_size=0.0014 --set aero.cfd.alpha_mode='"direct"'
uv run --project doc python doc/scripts/rans_sample.py run "$WORK/medium_minus"
```

Check the real spelling of `alpha_mode` in `crates/cfd/src/config.rs` first.

- [ ] **Step 4: Write the collector**

`research/nasa_vv/cases/sample_euler.py` `main()`:

1. Reads each level's `out/cfd_cases.csv` and `out/cfd_report.json` the way
   `doc/scripts/rans_mesh_study.py:21` does.
2. Builds a `GridFamily` whose `knobs` carry **both** `wall_size` and `fin_size`, so
   `check_similar` can see they scaled together.
3. Records `residual_drop_orders` and `converged` per level from the history.
4. Computes CNa as `(CN(4) - CN(-4)) / 2 / radians(4)` where the negative case exists,
   and `CN(4) / radians(4)` otherwise, saying which in a column.
5. Calls `study` for `cna`, `ca_alpha0`, `xcp`.
6. Writes `results/sample_euler_levels.csv` and `results/gci_sample_euler.csv`.

- [ ] **Step 5: Run it and check**

```bash
cd research && uv run --project nasa_vv python -m nasa_vv.cases.sample_euler --work "$HOME/.cache/nasa_vv_sample"
column -s, -t results/gci_sample_euler.csv
```

Check: `check_similar` passes; `CA` at M = 0.3, alpha = 0 is reported at all three
levels so Task 13's `ca_dalembert_refined` rows can be filled; any oscillatory entry
carries its note.

- [ ] **Step 6: Fill the consistency rows and commit**

```bash
cd research && uv run --project nasa_vv python -m nasa_vv.cases.consistency
git add research/nasa_vv/cases/sample_euler.py research/nasa_vv/results/
git commit -m "research(nasa-vv): add a similar Euler grid family for the sample rocket"
```

---

## Task 15: Basic Finner RANS grid family at M = 1.832

**Files:**
- Create: `research/nasa_vv/cases/basic_finner_rans.py`
- Create: `research/nasa_vv/results/basic_finner_rans_levels.csv`
- Read: `doc/data/validation/basic_finner/cfd_rans.csv`, `doc/source/validation.rst:704-712`

**Interfaces:**
- Consumes: `GridFamily`, `GridLevel`, `study` from Task 7.
- Produces: `main()` writing the two CSVs.

**M = 1.832, alpha = 0 and 2.** Not M = 2.375: RANS does not converge there, reaching
the 3000-iteration limit with a 1.3 and 0.6 order residual drop in a limit cycle, and
M = 3.147 diverges. Spec §6.4.

- [ ] **Step 1: Run the coarse level first**

```bash
cd /home/ddd3h/ignisyeet
WORK="$HOME/.cache/nasa_vv_bf_rans"
mkdir -p "$WORK"
uv run --project doc python doc/scripts/rans_sample.py setup "$WORK/coarse" \
  --stl examples/basic_finner/basic_finner.stl --stl-scale 0.001 \
  --machs 1.832 --alphas 0 2 --wall 0.0021 --model rans \
  --set aero.cfd.fin_size=0.00047
```

Take the geometry, CG and reference values from `examples/basic_finner/basic_finner.toml`
rather than the sample defaults; read that file and pass the matching `--cg` and
`--nose` arguments.

- [ ] **Step 2: Check it converged before running the finer levels**

```bash
tail -3 "$WORK/coarse/out/cfd/m1.832_a+00.00/history.csv"
```

The residual must drop at least 3 orders. If it does not, stop: write
`results/basic_finner_rans_levels.csv` with the residual drop recorded and the note
`RANS GCI not obtained at M = 1.832`, skip Steps 3 and 4, and say so in the report.
Do not tune the solver; spec §14 excludes that.

- [ ] **Step 3: Run the medium and fine levels on virgo**

Scale both knobs by 1.4:

| level | `wall_size` | `fin_size` |
|---|---|---|
| coarse | 2.10 mm | 0.47 mm |
| medium | 1.50 mm | 0.336 mm |
| fine | 1.07 mm | 0.24 mm |

```bash
/home/ddd3h/ignisyeet-wt/remote/virgo/virgo_run.sh -w -l \
  "$WORK/medium/case.toml" virgo01 64 aero --force
/home/ddd3h/ignisyeet-wt/remote/virgo/virgo_run.sh -w -l \
  "$WORK/fine/case.toml" virgo02 96 aero --force
```

- [ ] **Step 4: Collect and compute**

`main()` reads the three levels, builds the family with both knobs, computes CNa as
`CN(2) / radians(2)` to match the existing Basic Finner convention, and calls `study`
for `cna`, `ca_alpha0`, `xcp`. Every level's `residual_drop_orders` goes in the CSV
whether or not it passed.

```bash
cd research && uv run --project nasa_vv python -m nasa_vv.cases.basic_finner_rans --work "$HOME/.cache/nasa_vv_bf_rans"
```

- [ ] **Step 5: Commit**

```bash
git add research/nasa_vv/cases/basic_finner_rans.py research/nasa_vv/results/basic_finner_rans_levels.csv
git commit -m "research(nasa-vv): add the Basic Finner RANS grid family at M = 1.832"
```

---

## Task 16: Iterative error against the grid uncertainty

**Files:**
- Create: `research/nasa_vv/cases/iterative.py`
- Create: `research/nasa_vv/results/iterative.csv`
- Read: `doc/data/cfd/convergence_depth.csv`, `doc/data/cfd/rans_flatplate.csv`, `doc/data/cfd/fields/history_*.csv`

**Interfaces:**
- Consumes: `results/gci_existing.csv` and `results/gci_sample_euler.csv`.
- Produces: `main()` writing `results/iterative.csv`.

The tutorial's point is not that the residual fell, but that what is left is small
compared with the grid uncertainty. Spec §6.2.

- [ ] **Step 1: Write the collector**

`research/nasa_vv/cases/iterative.py` `main()` writes one row per condition with columns
`case,mach,alpha_deg,model,quantity,value_at_stop,value_converged,iterative_error_pct,gci_fine_pct,ratio,source`
where `ratio = iterative_error_pct / gci_fine_pct`.

Sources:
- `doc/data/cfd/convergence_depth.csv` gives the Euler sample at M = 0.8 and 2: the
  values at different stopping depths against the deepest run.
- `doc/data/cfd/rans_flatplate.csv` has `cd_at_stop_vs_final_percent`.
- The GCI comes from the matching condition in the two GCI CSVs.

- [ ] **Step 2: Run it and judge**

```bash
cd research && uv run --project nasa_vv python -m nasa_vv.cases.iterative
column -s, -t results/iterative.csv
```

A `ratio` below about 0.1 means the iterative error does not disturb the grid study.
Any row above 1.0 invalidates the GCI for that condition and must be said plainly in
the report, not smoothed over.

- [ ] **Step 3: Commit**

```bash
git add research/nasa_vv/cases/iterative.py research/nasa_vv/results/iterative.csv
git commit -m "research(nasa-vv): compare the iterative error with the grid uncertainty"
```

---

## Task 17: Temporal convergence of the flight integrator

**Files:**
- Create: `research/nasa_vv/cases/temporal.py`
- Create: `research/nasa_vv/results/temporal.csv`
- Read: `doc/source/verification.rst:142-168`, `crates/sim/src/flight.rs:2120-2162`

**Interfaces:**
- Consumes: `constant_ratio` from Task 2.
- Produces: `main()` writing `results/temporal.csv`.

The CFD is steady, so the tutorial's temporal convergence **does not apply** to it, and
the report says exactly that. The time-dependent part of IgnisYeet is the 6-DOF
integrator, so the study is done there instead. Spec §6.5.

- [ ] **Step 1: Run the sweep**

Run `examples/sample.toml` with a fixed-step integrator at halving steps, so the
refinement ratio is exactly 2:

```bash
cd /home/ddd3h/ignisyeet
for dt in 0.008 0.004 0.002 0.001; do
  target/release/ignisyeet --no-progress sim examples/sample.toml \
    --set sim.integrator='"rk4"' --set sim.dt=$dt \
    --out "$HOME/.cache/nasa_vv_dt/dt$dt" --force
done
```

Check the real key names in `crates/sim/src/config.rs` or the config documentation
before running; `sim.integrator` and `sim.dt` are the likely spellings but must be
confirmed.

- [ ] **Step 2: Write the collector**

`main()` reads apogee, landing distance and flight time from each run's summary, calls
`constant_ratio` with `r = 2.0` on each quantity over the three finest steps, and writes
`results/temporal.csv` with columns
`quantity,dt1,dt2,dt3,f1,f2,f3,order,extrapolated,gci_fine_pct,asymptotic_ratio,note`.

The observed order should come out near 4 for rk4. If it does not, report what it is;
the deviation is the finding.

- [ ] **Step 3: Run it and commit**

```bash
cd research && uv run --project nasa_vv python -m nasa_vv.cases.temporal
column -s, -t results/temporal.csv
git add research/nasa_vv/cases/temporal.py research/nasa_vv/results/temporal.csv
git commit -m "research(nasa-vv): measure the temporal convergence of the flight integrator"
```

---

## Task 18: Layer 1 — the cone-cylinder through the IgnisYeet pipeline

**Files:**
- Create: `research/nasa_vv/cases/layer1_cone.py`
- Create: `research/nasa_vv/results/layer1_cone.csv`
- Create: `research/nasa_vv/results/layer0_vs_layer1.csv`
- Read for reference: `doc/scripts/rans_cone.py:71-105`, `doc/scripts/rans_geometry.py:36`

**Interfaces:**
- Consumes: `taylor_maccoll` from Task 5; `GridFamily`, `GridLevel`, `study` from Task 7; `results/layer0.csv` from Task 10.
- Produces: `main()` writing the two CSVs.

This is the centre of the study's design. Layer 0 solved the same cone with SU2 alone on
a structured axisymmetric grid; layer 1 solves it through the product's whole chain,
CAD to gmsh to SU2 to coefficients, on a 3-D unstructured half model. The difference
between the two, both measured against the same Taylor-Maccoll solution, is the
pipeline's own contribution. Spec §5.

Because the two layers differ in mesh type as well, the difference is reported as
"pipeline error, including 3-D unstructured meshing", never as meshing error alone.
Spec §4.

- [ ] **Step 1: Generate the geometry at the layer 0 conditions**

`doc/scripts/rans_geometry.py:36` already writes this body, but its CLI does not pass
the radius or the cone angle through. Check:

```bash
cd /home/ddd3h/ignisyeet && sed -n '90,101p' doc/scripts/rans_geometry.py
```

Call the `cone` function directly rather than editing that script:

```bash
cd /home/ddd3h/ignisyeet
WORK="$HOME/.cache/nasa_vv_layer1"; mkdir -p "$WORK"
uv run --project doc python -c "
import sys; sys.path.insert(0, 'doc/scripts')
import rans_geometry
rans_geometry.cone('$WORK/cone.stl', r=0.05, delta_deg=10.0, lb=0.3)
"
```

- [ ] **Step 2: Run three similar levels at M = 2.35, alpha = 0**

M = 2.35, not the M = 2 of the existing `rans_cone_cp.csv`, so that layer 0 and layer 1
share one exact solution. Euler, since layer 0 is inviscid.

| level | `wall_size` | `fin_size` |
|---|---|---|
| coarse | 3.92 mm | 1.96 mm |
| medium | 2.80 mm | 1.40 mm |
| fine | 2.00 mm | 1.00 mm |

```bash
for lv in coarse medium fine; do :; done   # see the table above for the sizes
uv run --project doc python doc/scripts/rans_sample.py setup "$WORK/coarse" \
  --stl "$WORK/cone.stl" --stl-scale 1 --machs 2.35 --alphas 0 \
  --wall 0.00392 --model euler --set aero.cfd.fin_size=0.00196
uv run --project doc python doc/scripts/rans_sample.py run "$WORK/coarse"
```

The body has no fins, so confirm whether `fin_size` has any effect here; if the product
ignores it for a finless body, record only `wall_size` in the family's `knobs` and say
so, otherwise `check_similar` would compare a knob that does nothing.

- [ ] **Step 3: Extract the cone surface pressure the way the existing script does**

`doc/scripts/rans_cone.py:71-105` already bins the wall nodes of `surface_flow.vtk`
along `x / L_cone` over 0.2 to 0.9 and keeps only nodes within 1 per cent of the cone
radius. Reuse it rather than rewriting the binning:

```bash
uv run --project doc python doc/scripts/rans_cone.py \
  "$WORK/coarse/out/cfd/m2.350_a+00.00" --out "$WORK/coarse_cp.csv"
```

Check `rans_cone.py --help` first: its `--mach` default is 2 and it must be told 2.35,
and its hard-coded `R = 0.05` and 10 degree angle match this body.

- [ ] **Step 4: Write the collector**

`research/nasa_vv/cases/layer1_cone.py` `main()`:

1. Reads the three levels' binned Cp.
2. Builds a `GridFamily` and calls `study` for `cp_mean` (the mean over the bins) and
   `cp_nearest_tip` (the bin at the smallest `x / L_cone`).
3. Compares each level's `cp_mean` with `taylor_maccoll(2.35, 10.0).cp`.
4. Writes `results/layer1_cone.csv` with columns
   `level,nodes,wall_size_m,x_over_l,cp_cfd,cp_exact,difference,difference_pct,residual_drop_orders`.
5. Writes `results/layer0_vs_layer1.csv` with one row per quantity:
   `quantity,exact,layer0_fine,layer0_diff_pct,layer0_gci_pct,layer1_fine,layer1_diff_pct,layer1_gci_pct,pipeline_contribution_pct`
   where `pipeline_contribution_pct = layer1_diff_pct - layer0_diff_pct`.

- [ ] **Step 5: Run it and read the result**

```bash
cd research && uv run --project nasa_vv python -m nasa_vv.cases.layer1_cone --work "$HOME/.cache/nasa_vv_layer1"
column -s, -t results/layer0_vs_layer1.csv
```

If `pipeline_contribution_pct` is smaller than either layer's GCI, the two layers are
indistinguishable at this resolution; say that rather than quoting a difference the
uncertainty cannot support.

- [ ] **Step 6: Commit**

```bash
git add research/nasa_vv/cases/layer1_cone.py research/nasa_vv/results/layer1_cone.csv research/nasa_vv/results/layer0_vs_layer1.csv
git commit -m "research(nasa-vv): separate the pipeline's error from the solver's on the cone"
```

---

## Task 19: Farfield mass-flux balance

**Files:**
- Create: `research/nasa_vv/cases/massflux.py`
- Create: `research/nasa_vv/results/massflux.csv`
- Test: `research/nasa_vv/tests/test_massflux.py`

**Interfaces:**
- Consumes: the SU2 output of Task 10's `wedge` and `cone` cases.
- Produces: `flux_imbalance(inflow_kg_s, outflow_kg_s) -> float` and `main()`.

Spec §6.3 lists this as the fourth consistency check. The tutorial's own example is a
duct whose mass flow must be equal at every station. The layer 0 cases have simple
rectangular domains, so the balance is a clean integral; the 3-D rocket half model is
not used for this check.

- [ ] **Step 1: Write the failing test**

`research/nasa_vv/tests/test_massflux.py`:

```python
# SPDX-License-Identifier: GPL-3.0-or-later
import pytest

from nasa_vv.cases.massflux import flux_imbalance


def test_a_balanced_domain_has_zero_imbalance():
    assert flux_imbalance(12.5, 12.5) == pytest.approx(0.0)


def test_the_imbalance_is_relative_to_the_inflow():
    assert flux_imbalance(100.0, 99.0) == pytest.approx(0.01)


def test_more_out_than_in_is_still_positive():
    assert flux_imbalance(100.0, 101.0) == pytest.approx(0.01)


def test_zero_inflow_is_an_error():
    with pytest.raises(ValueError):
        flux_imbalance(0.0, 1.0)
```

- [ ] **Step 2: Run it to verify it fails, then implement**

```bash
cd research && uv run --project nasa_vv --group dev python -m pytest nasa_vv/tests/test_massflux.py -v
```

`flux_imbalance` returns `abs(inflow - outflow) / abs(inflow)` and raises `ValueError`
on a zero inflow.

- [ ] **Step 3: Have SU2 report the flux**

SU2 can integrate the mass flow over a marker. Add to the layer 0 configs, through
`layer0_config`'s case-specific block:

```
MARKER_ANALYZE= ( inlet, outlet )
MARKER_ANALYZE_AVERAGE= MASSFLUX
```

Check the real keyword spelling against the SU2 8.3.0 the repository uses:

```bash
~/.local/share/ignisyeet-su2-native/bin/SU2_CFD 2>&1 | head -20
grep -rn "MARKER_ANALYZE" ~/.local/share/ignisyeet-su2-native/ 2>/dev/null | head
```

If the keyword differs, use the real one. If SU2 will not report it, integrate
`rho * u . n` over the boundary from the surface output instead, and say in the CSV
which method was used.

- [ ] **Step 4: Write the collector and run it**

`main()` reads each layer 0 case's flux output, computes the imbalance, and writes
`results/massflux.csv` with columns
`case,mach,level,inflow_kg_s,outflow_kg_s,imbalance_rel,method,source`.

```bash
cd research && uv run --project nasa_vv python -m nasa_vv.cases.massflux --work "$HOME/.cache/nasa_vv_layer0"
column -s, -t results/massflux.csv
```

An imbalance under about 1e-3 relative is consistent. A larger one is a finding to
report, not something to tune away.

- [ ] **Step 5: Commit**

```bash
git add research/nasa_vv/cases/massflux.py research/nasa_vv/tests/test_massflux.py research/nasa_vv/results/massflux.csv
git commit -m "research(nasa-vv): check the farfield mass-flux balance"
```

---

## Task 20: Model uncertainty — Euler against RANS

**Files:**
- Create: `research/nasa_vv/cases/model_uncertainty.py`
- Create: `research/nasa_vv/results/model_uncertainty.csv`
- Read: `doc/data/cfd/rans_vs_euler.csv`, `doc/data/validation/basic_finner/cfd_rans.csv`, `doc/data/validation/basic_finner/cfd_euler.csv`, `crates/cfd/src/su2cfg.rs:212`

**Interfaces:**
- Consumes: `results/gci_existing.csv`, `results/gci_sample_euler.csv`, `results/basic_finner_rans_levels.csv`.
- Produces: `main()` writing `results/model_uncertainty.csv`.

The tutorial's sixth step asks for several turbulence models. The product offers only
Euler and RANS with Spalart-Allmaras; `crates/cfd/src/su2cfg.rs:212` writes `SA` and
nothing else. So the step is answered in part, and the missing part is stated as a limit
of scope, not quietly dropped. Spec §6.6.

- [ ] **Step 1: Confirm the limit before writing about it**

```bash
cd /home/ddd3h/ignisyeet && grep -n "KIND_TURB_MODEL" crates/cfd/src/su2cfg.rs
grep -n "enum FlowModel" -A 10 crates/cfd/src/config.rs
```

Record what these actually say. If a second turbulence model has since been added, the
report must say so instead.

- [ ] **Step 2: Write the collector**

`research/nasa_vv/cases/model_uncertainty.py` `main()` writes one row per condition with
columns
`case,mach,alpha_deg,quantity,euler,rans,difference,difference_pct,euler_gci_pct,rans_gci_pct,separable,source`
where `separable` is true when `abs(difference_pct)` exceeds the sum of the two GCI
bands. A model difference smaller than the numerical uncertainty is not a measured model
difference, and the column records that judgement explicitly.

Conditions, all from committed data:
- sample rocket, M = 0.8 and 2.0, alpha = 4, from `rans_vs_euler.csv`
- Basic Finner, M = 1.832, alpha = 0 and 2, from `cfd_rans.csv` against `cfd_euler.csv`

- [ ] **Step 3: Run it and commit**

```bash
cd research && uv run --project nasa_vv python -m nasa_vv.cases.model_uncertainty
column -s, -t results/model_uncertainty.csv
git add research/nasa_vv/cases/model_uncertainty.py research/nasa_vv/results/model_uncertainty.csv
git commit -m "research(nasa-vv): compare Euler with RANS against the numerical uncertainty"
```

---

## Task 21: The Japanese report page

**Files:**
- Create: `research/nasa_vv/report/ja/report.md` (the source text)
- Create: `doc/source/research/nasa-vv/index.rst` (generated by `research/nasa_vv/report/ja/build_page.py`)
- Create: `doc/source/_static/research/nasa-vv/` (copies of the committed SVGs)
- Create: `doc/source/_static/research-report.css`
- Modify: `doc/source/research/index.rst`
- Modify: `doc/source/_templates/page.html`

**Interfaces:**
- Consumes: every CSV in `research/nasa_vv/results/` and every figure in `research/nasa_vv/figures/`.
- Produces: the published page at `/research/nasa-vv/`.

- [ ] **Step 1: Write the report source**

`research/nasa_vv/report/ja/report.md`, in Japanese, structured by the tutorial's six
steps, with these rules:

- Every number cites the CSV it came from.
- Verification, cross-comparison and validation are used precisely (Global Constraints).
- The cone10 discrepancy is reported as a finding, with the table from
  `results/cone10_reference_check.csv`.
- Temporal convergence states that it does not apply to the steady CFD, and gives the
  integrator study instead.
- The `make_plots.py:1610` zero-length error bar is reported as a finding, not fixed.
- Step 1 of the tutorial, the examination of the code itself, runs no new calculation.
  It cites the existing test corpus: `doc/source/testing.rst`, the integrator order
  tests at `crates/sim/src/dop853.rs:330` and `crates/sim/src/lie.rs:404`, and the
  grid-convergence tests this study adds. It states plainly that before this study
  there was no test of any grid-convergence code, which `doc/source/testing.rst:494`
  already recorded.
- Model uncertainty states that only Spalart-Allmaras is available, so the tutorial's
  request for several turbulence models is answered only in part.
- The layer 0 against layer 1 difference is called "pipeline error, including 3-D
  unstructured meshing", never meshing error alone.
- Where a RANS GCI was not obtained, the residual drop is given.
- No implementation narration: no script names, no function names, no run logs. The
  repository's documentation rule applies to the report as well.

- [ ] **Step 2: Build the page**

`doc/source/research/nasa-vv/index.rst`, following the structure and classes already in
`doc/source/research/index.rst`: a raw-HTML block with the `iy-*` wrappers, which gets the
shared top bar and footer from `_templates/page.html`, plus `_static/research-report.css`.
The generator copies the SVGs into `doc/source/_static/research/nasa-vv/`.

- [ ] **Step 3: Replace the placeholder on the research index**

In `doc/source/research/index.rst`, replace

```html
       <p class="iy-empty">掲載準備中</p>
```

with an `iy-entries` list whose first entry links to `nasa-vv/`, giving the title, the date
and a two-line summary.

- [ ] **Step 4: Build and look at it**

```bash
cd /home/ddd3h/ignisyeet/doc
uv run --project . sphinx-build -b html -W --keep-going source build/html
```

Then screenshot both pages at 1280 and 390 pixels wide, in light and dark, the way the
research page was checked earlier in this session, and look at them. Check: no broken
links, no horizontal scroll at 390, figures legible in dark.

- [ ] **Step 5: Commit**

```bash
git add research/nasa_vv/report/ doc/source/research/ doc/source/_static/ doc/source/_templates/
git commit -m "research(nasa-vv): publish the Japanese report at /research/nasa-vv/"
```

---

## Task 22: The English paper draft

**Files:**
- Create: `research/nasa_vv/report/en/main.tex`
- Create: `research/nasa_vv/report/en/references.bib`
- Create: `research/nasa_vv/report/en/Makefile`

**Interfaces:**
- Consumes: the same CSVs and figures as Task 18.
- Produces: `main.pdf` when built.

- [ ] **Step 1: Write the paper**

`main.tex` with the standard sections: Introduction, Method (the three layers),
Verification (layers 0 and 1), Application (layer 2), Validation, Discussion,
Conclusions. Cite AIAA G-077-1998, Roache, Celik et al. (2008), the NPARC archive pages,
DREV-TM-9703 and the NASA TMR in `references.bib`.

State the cone10 discrepancy in the Verification section, with the table.

- [ ] **Step 2: Write the Makefile**

```makefile
# SPDX-License-Identifier: GPL-3.0-or-later
main.pdf: main.tex references.bib
	latexmk -pdf -interaction=nonstopmode main.tex

clean:
	latexmk -C
.PHONY: clean
```

- [ ] **Step 3: Build it**

```bash
cd research/nasa_vv/report/en && make
```

If `latexmk` is missing, say so and leave the source committed without the PDF; the PDF
is not committed in either case.

- [ ] **Step 4: Add the PDF to the gitignore and commit**

Add `report/en/*.pdf` and the latex aux patterns to `research/.gitignore`, then:

```bash
git add research/nasa_vv/report/en/ research/.gitignore
git commit -m "research(nasa-vv): add the English paper draft"
```

---

## Task 23: Final review and the research index

**Files:**
- Modify: `research/README.md`
- Modify: `docs/superpowers/specs/2026-10-17-nasa-cfd-vv-design.md` (status only, if needed)

- [ ] **Step 1: Run every test**

```bash
cd research && uv run --project nasa_vv --group dev python -m pytest nasa_vv/tests/ -v
```

Expected: all pass. Record the count.

- [ ] **Step 2: Check every completion condition in spec §13**

Go through the seven bullets one by one and point at the file that satisfies each.
Any that is not satisfied is stated as not satisfied, in the report and here.

- [ ] **Step 3: Check that every reported number is traceable**

```bash
cd /home/ddd3h/ignisyeet && ls research/nasa_vv/results/
```

For each table in the report, confirm the CSV it cites exists and contains that number.

- [ ] **Step 4: Update the research index**

Set the study's status in `research/README.md` and list every results CSV with one line
saying what it holds.

- [ ] **Step 5: Commit**

```bash
git add research/README.md docs/superpowers/specs/
git commit -m "research(nasa-vv): record the study's results in the research index"
```
