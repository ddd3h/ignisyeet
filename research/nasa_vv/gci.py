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
        try:
            q = math.log((r21**p - s) / (r32**p - s))
            nxt = abs(math.log(abs(e32 / e21)) + q) / math.log(r21)
        except (OverflowError, ValueError, ZeroDivisionError):
            g.note = "the fixed point iteration did not converge: it left the finite range"
            return g
        if not math.isfinite(nxt):
            g.note = "the fixed point iteration did not converge: it left the finite range"
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
