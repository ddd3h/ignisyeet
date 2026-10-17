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
    if delta_deg < 0.0:
        raise ValueError(
            f"oblique_shock got a negative deflection {delta_deg} deg; "
            "give the magnitude of the turn"
        )
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


_MAX_BRACKET_MACH = 1.0e12


def prandtl_meyer_expansion(
    mach: float, turn_deg: float, gamma: float = 1.4
) -> Expansion:
    """Isentropic turn through turn_deg; solves nu(M2) = nu(M1) + turn."""
    if mach < 1.0:
        raise ValueError("prandtl_meyer_expansion needs a supersonic free stream")
    nu_inf = 90.0 * (math.sqrt((gamma + 1.0) / (gamma - 1.0)) - 1.0)
    max_turn = nu_inf - prandtl_meyer_nu(mach, gamma)
    if turn_deg > max_turn:
        raise ValueError(
            f"a turn of {turn_deg} deg exceeds the maximum turn "
            f"{max_turn:.4f} deg available from M = {mach}"
        )
    target = prandtl_meyer_nu(mach, gamma) + turn_deg
    if target < 0.0:
        raise ValueError(
            f"a turn of {turn_deg} deg from M = {mach} would go below sonic "
            "(nu < 0)"
        )
    # Grow the upper bracket until it covers the target. nu(inf) - nu(M) falls
    # only as ~286/M deg, so turns close to the physical maximum need a large M2.
    lo, hi = 1.0, 100.0
    while prandtl_meyer_nu(hi, gamma) < target:
        if hi >= _MAX_BRACKET_MACH:
            raise ValueError(
                f"a turn of {turn_deg} deg from M = {mach} needs M2 beyond "
                f"{_MAX_BRACKET_MACH:g}, which this solver does not bracket"
            )
        hi *= 2.0
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


@dataclass
class Cone:
    cp: float
    beta_deg: float
    mach_surface: float
    p_pinf: float
    t_tinf: float
    rho_rhoinf: float


# Upper end of the first bisection bracket on the shock angle. A cone whose
# attached shock lies beyond it takes the detachment search below.
_CONE_BETA_BRACKET = math.radians(80.0)


def taylor_maccoll(mach: float, delta_deg: float, gamma: float = 1.4) -> Cone:
    """Sharp cone with an attached shock; shooting on the shock angle.

    Ported from doc/scripts/rans_cone.py:19 with the numerics unchanged (RK4
    step 1e-5 rad from the shock to the surface, linear interpolation to
    V_theta = 0, 80 bisection steps on the shock angle), then extended with the
    thermodynamic ratios at the cone surface.

    Raises ValueError for a subsonic free stream, a half angle outside
    (0, 90) degrees, gamma <= 1, and a half angle beyond the largest one that
    keeps the shock attached at this Mach number.
    """
    if not mach > 1.0:
        raise ValueError("taylor_maccoll needs a supersonic free stream")
    if not 0.0 < delta_deg < 90.0:
        raise ValueError(
            f"taylor_maccoll needs a cone half angle in (0, 90) degrees, got {delta_deg}"
        )
    if not gamma > 1.0:
        raise ValueError(f"taylor_maccoll needs gamma > 1, got {gamma}")
    g = gamma
    delta = math.radians(delta_deg)

    def cone_angle(beta: float) -> tuple[float, float, float]:
        """Returns (cone half angle, surface V_r / V_max, shock-normal Mach)."""
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
            k2 = f(th + h / 2, (y[0] + h / 2 * k1[0], y[1] + h / 2 * k1[1]))
            k3 = f(th + h / 2, (y[0] + h / 2 * k2[0], y[1] + h / 2 * k2[1]))
            k4 = f(th + h, (y[0] + h * k3[0], y[1] + h * k3[1]))
            yn = (
                y[0] + h / 6 * (k1[0] + 2 * k2[0] + 2 * k3[0] + k4[0]),
                y[1] + h / 6 * (k1[1] + 2 * k2[1] + 2 * k3[1] + k4[1]),
            )
            if yn[1] >= 0:
                w = -y[1] / (yn[1] - y[1])
                return th + w * h, y[0] + w * (yn[0] - y[0]), mn1
            th, y = th + h, yn
            if th <= 1e-6:   # reached the axis without V_theta = 0: no cone
                break
        return th, y[0], mn1

    lo = math.asin(1.0 / mach) + 1e-6
    hi = _CONE_BETA_BRACKET
    if cone_angle(hi)[0] < delta:
        # The cone angle rises with the shock angle up to a maximum (the
        # detachment angle) and falls again towards the normal shock. Locate
        # the maximum by golden-section search and bisect on the rising side.
        top = 0.5 * math.pi - 1e-6
        phi = 0.5 * (math.sqrt(5.0) - 1.0)
        a, b = lo, top
        c, d = b - phi * (b - a), a + phi * (b - a)
        fc, fd = cone_angle(c)[0], cone_angle(d)[0]
        for _ in range(40):
            if fc < fd:
                a, c, fc = c, d, fd
                d = a + phi * (b - a)
                fd = cone_angle(d)[0]
            else:
                b, d, fd = d, c, fc
                c = b - phi * (b - a)
                fc = cone_angle(c)[0]
        hi = 0.5 * (a + b)
        max_angle = cone_angle(hi)[0]
        if max_angle < delta:
            raise ValueError(
                f"the shock is detached: a cone half angle of {delta_deg} deg "
                f"exceeds the maximum {math.degrees(max_angle):.3f} deg at M = {mach}"
            )
    for _ in range(80):
        mid = 0.5 * (lo + hi)
        if cone_angle(mid)[0] < delta:
            lo = mid
        else:
            hi = mid
    beta = 0.5 * (lo + hi)
    _, vc, mn1 = cone_angle(beta)
    mc = math.sqrt(2.0 / (g - 1) * vc * vc / (1 - vc * vc))

    # pt is uniform behind the conical shock and the flow to the surface is
    # isentropic, so the surface state follows from M_surface and the shock loss.
    pt2_pt1 = (
        ((g + 1) * mn1**2 / ((g - 1) * mn1**2 + 2)) ** (g / (g - 1))
        * ((g + 1) / (2 * g * mn1**2 - (g - 1))) ** (1 / (g - 1))
    )
    p_pinf = pt2_pt1 * isentropic_pt_p(mach, g) / isentropic_pt_p(mc, g)
    t_tinf = isentropic_tt_t(mach, g) / isentropic_tt_t(mc, g)
    cp = 2 / (g * mach**2) * (p_pinf - 1)
    return Cone(cp, math.degrees(beta), mc, p_pinf, t_tinf, p_pinf / t_tinf)


# ------------------------------------------------------------------ Blasius flat plate

_BLASIUS_ETA_MAX = 10.0
_BLASIUS_STEP = 0.005


def blasius_cf(re_x: float) -> float:
    """Local skin-friction coefficient of the laminar incompressible flat plate, 0.664 / sqrt(Re_x).

    Cf = tau_w / (rho U^2 / 2), Re_x = U x / nu with the free-stream U and nu.
    """
    return 0.664 / math.sqrt(re_x)


def _blasius_rhs(y: tuple[float, float, float]) -> tuple[float, float, float]:
    """Blasius' equation f''' + f f'' / 2 = 0 as a first-order system in (f, f', f'')."""
    f, fp, fpp = y
    return fp, fpp, -0.5 * f * fpp


def _blasius_rk4(y: tuple[float, float, float], h: float) -> tuple[float, float, float]:
    k1 = _blasius_rhs(y)
    k2 = _blasius_rhs(tuple(a + 0.5 * h * b for a, b in zip(y, k1)))
    k3 = _blasius_rhs(tuple(a + 0.5 * h * b for a, b in zip(y, k2)))
    k4 = _blasius_rhs(tuple(a + h * b for a, b in zip(y, k3)))
    return tuple(a + h / 6.0 * (b + 2.0 * c + 2.0 * d + e) for a, b, c, d, e in zip(y, k1, k2, k3, k4))


def _blasius_integrate(fpp0: float, eta_max: float, n: int) -> list[tuple[float, float, float, float]]:
    """(eta, f, f', f'') at n + 1 equally spaced points, RK4 with substeps of at most _BLASIUS_STEP."""
    h_out = eta_max / n
    m = max(1, math.ceil(h_out / _BLASIUS_STEP - 1e-12))
    h = h_out / m
    y = (0.0, 0.0, fpp0)
    out = [(0.0, *y)]
    for k in range(n):
        for _ in range(m):
            y = _blasius_rk4(y, h)
        out.append(((k + 1) * h_out, *y))
    return out


_blasius_shot: dict[float, float] = {}


def _blasius_wall_curvature(eta_max: float) -> float:
    """f''(0) such that f'(eta_max) = 1, by bisection (f'(eta_max) increases with f''(0))."""
    if eta_max not in _blasius_shot:
        n = round(eta_max / _BLASIUS_STEP)
        lo, hi = 0.1, 0.6
        for _ in range(70):
            mid = 0.5 * (lo + hi)
            if _blasius_integrate(mid, eta_max, n)[-1][2] < 1.0:
                lo = mid
            else:
                hi = mid
        _blasius_shot[eta_max] = 0.5 * (lo + hi)
    return _blasius_shot[eta_max]


def blasius_profile(n: int = 200, eta_max: float = _BLASIUS_ETA_MAX) -> list[tuple[float, float]]:
    """The Blasius profile (eta, f'(eta)) = (y sqrt(U / nu x), u / U) at n + 1 equally spaced points of [0, eta_max].

    f''' + f f'' / 2 = 0, f(0) = f'(0) = 0, f'(eta_max) = 1, integrated with RK4 and
    shot on f''(0) (0.332057).
    """
    return [(e, fp) for e, _, fp, _ in _blasius_integrate(_blasius_wall_curvature(eta_max), eta_max, n)]


def blasius_thicknesses(eta_max: float = _BLASIUS_ETA_MAX) -> tuple[float, float]:
    """(delta*, theta) in the similarity coordinate eta: 1.7208 and 0.6641.

    delta* = int (1 - f') d eta = eta_max - f(eta_max); theta = int f' (1 - f') d eta, by the trapezoid rule on
    the integrated profile.
    """
    pts = _blasius_integrate(_blasius_wall_curvature(eta_max), eta_max, round(eta_max / _BLASIUS_STEP))
    h = eta_max / (len(pts) - 1)
    delta_star = eta_max - pts[-1][1]
    g = [fp * (1.0 - fp) for _, _, fp, _ in pts]
    theta = h * (0.5 * g[0] + sum(g[1:-1]) + 0.5 * g[-1])
    return delta_star, theta
