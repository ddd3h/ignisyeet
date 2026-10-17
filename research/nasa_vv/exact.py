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
