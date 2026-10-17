# SPDX-License-Identifier: GPL-3.0-or-later
"""Exact solutions against the NPARC validation archive's published values.

Wedge: https://www.grc.nasa.gov/www/wind/valid/wedge/wedge.html Table 2
    15 degree wedge at M = 2.5: shock angle, downstream Mach number and the
    static pressure, density and temperature ratios across the shock.
Prandtl-Meyer: https://www.grc.nasa.gov/www/wind/valid/pm15/pm15.html Table 7
    15 degree expansion from M = 2.5: downstream Mach number and the static
    pressure, temperature and density ratios.

The cone10 archive page is deliberately NOT used as a reference here; its table
is internally inconsistent (spec section 4.0).
"""
import importlib.util
import math
from pathlib import Path

import pytest

from nasa_vv.exact import (
    isentropic_pt_p,
    isentropic_tt_t,
    oblique_shock,
    prandtl_meyer_expansion,
    prandtl_meyer_nu,
    taylor_maccoll,
)


def test_oblique_shock_matches_the_nparc_wedge_case():
    """15 degree wedge at M = 2.5, NPARC wedge Table 2."""
    # The page states its analytic values come from oblshk.f run to a
    # tolerance of 1.0E-06, so they carry about 1e-6 of slack of their own,
    # more than the half-unit of the last printed digit (5e-7). 2e-6 brackets
    # that solver tolerance without admitting a real error; do not tighten it.
    tol = 2e-6
    s = oblique_shock(2.5, 15.0)
    assert s.beta_deg == pytest.approx(36.94490, abs=tol)
    assert s.mach2 == pytest.approx(1.873526, abs=tol)
    assert s.p2_p1 == pytest.approx(2.467500, abs=tol)
    assert s.rho2_rho1 == pytest.approx(1.866549, abs=tol)
    assert s.t2_t1 == pytest.approx(1.321958, abs=tol)


def test_prandtl_meyer_matches_the_nparc_pm15_case():
    """15 degree expansion from M = 2.5, NPARC pm15 Table 7."""
    # Values are printed to 4 decimals: half a unit of the last digit is 5e-5.
    tol = 5e-5
    e = prandtl_meyer_expansion(2.5, 15.0)
    assert e.mach2 == pytest.approx(3.2368, abs=tol)
    assert e.p2_p1 == pytest.approx(0.3274, abs=tol)
    assert e.t2_t1 == pytest.approx(0.7269, abs=tol)
    assert e.rho2_rho1 == pytest.approx(0.4505, abs=tol)


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
    """Deflections are attached at each Mach number: at M = 1.05 the maximum
    deflection is only 0.558 deg, so 1, 5 and 10 deg would be detached there."""
    cases = {
        1.05: (0.1, 0.3, 0.5),
        1.5: (1.0, 5.0, 10.0),
        2.5: (1.0, 5.0, 10.0),
        4.0: (1.0, 5.0, 10.0),
    }
    for m, deflections in cases.items():
        for d in deflections:
            assert oblique_shock(m, d).pt2_pt1 < 1.0


def test_shock_total_pressure_ratio_matches_the_wedge_case():
    """Rankine-Hugoniot total-pressure ratio for the NPARC wedge case
    (15 degrees at M = 2.5)."""
    assert oblique_shock(2.5, 15.0).pt2_pt1 == pytest.approx(0.928955, abs=2e-6)


def test_oblique_shock_rejects_a_negative_deflection():
    with pytest.raises(ValueError, match="negative deflection -5"):
        oblique_shock(2.0, -5.0)


def test_prandtl_meyer_rejects_a_turn_beyond_the_maximum():
    """From M = 2.5 the most the flow can turn is nu(inf) - nu(2.5) = 91.33 deg."""
    with pytest.raises(ValueError, match="exceeds the maximum turn"):
        prandtl_meyer_expansion(2.5, 200.0)


def test_prandtl_meyer_rejects_a_turn_below_sonic():
    """A turn of -50 deg would need nu < 0, i.e. M < 1."""
    with pytest.raises(ValueError, match="below sonic"):
        prandtl_meyer_expansion(2.5, -50.0)


def test_prandtl_meyer_solves_turns_near_the_maximum_instead_of_clamping():
    """From M = 2.5 the turn is limited by nu(inf) - nu(2.5) = 91.33 deg. A
    solver bracketed at M = 100 would only reach 88.47 deg and return 100.0 as
    though it were a root for 89, 91 and 91.33 deg; each must be a true root."""
    nu1 = prandtl_meyer_nu(2.5)
    for turn in (89.0, 91.0, 91.33):
        e = prandtl_meyer_expansion(2.5, turn)
        assert e.mach2 > 100.0
        assert prandtl_meyer_nu(e.mach2) == pytest.approx(nu1 + turn, abs=1e-9)


def test_prandtl_meyer_refuses_the_unreachable_limit_itself():
    """A turn of exactly nu(inf) - nu(M1) needs M2 = infinity."""
    g = 1.4
    limit = 90.0 * (math.sqrt((g + 1.0) / (g - 1.0)) - 1.0) - prandtl_meyer_nu(2.5)
    with pytest.raises(ValueError, match="bracket"):
        prandtl_meyer_expansion(2.5, limit)


def test_prandtl_meyer_rejects_a_subsonic_free_stream():
    with pytest.raises(ValueError, match="prandtl_meyer_expansion needs a supersonic"):
        prandtl_meyer_expansion(0.8, 5.0)


def test_prandtl_meyer_maximum_turn_follows_gamma():
    """nu(inf) = 90*(sqrt((g+1)/(g-1)) - 1) deg, so the limit moves with gamma."""
    g = 1.67
    limit = 90.0 * (math.sqrt((g + 1.0) / (g - 1.0)) - 1.0) - prandtl_meyer_nu(2.5, g)
    nu1 = prandtl_meyer_nu(2.5, g)
    e = prandtl_meyer_expansion(2.5, limit - 1.0, g)
    assert e.mach2 > 100.0
    assert prandtl_meyer_nu(e.mach2, g) == pytest.approx(nu1 + limit - 1.0, abs=1e-9)
    with pytest.raises(ValueError, match="exceeds the maximum turn"):
        prandtl_meyer_expansion(2.5, limit + 1.0, g)


# --- Taylor-Maccoll cone ---------------------------------------------------

_RANS_CONE = Path(__file__).resolve().parents[3] / "doc" / "scripts" / "rans_cone.py"


def _load_rans_cone():
    spec = importlib.util.spec_from_file_location("_rans_cone_reference", _RANS_CONE)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def test_taylor_maccoll_reproduces_the_existing_repository_values():
    """doc/scripts/rans_cone.py must keep giving the same answer after the port.

    The targets are the values that script prints (Cp and Mc to six places,
    beta to four), so the tolerances are the half-unit of the last printed
    digit: 5e-7 for Cp and Mc, 5e-5 for beta.
    """
    c2 = taylor_maccoll(2.0, 10.0)
    assert c2.cp == pytest.approx(0.104471, abs=5e-7)
    assert c2.beta_deg == pytest.approx(31.2061, abs=5e-5)
    assert c2.mach_surface == pytest.approx(1.834028, abs=5e-7)

    c235 = taylor_maccoll(2.35, 10.0)
    assert c235.cp == pytest.approx(0.096731, abs=5e-7)
    assert c235.beta_deg == pytest.approx(26.7367, abs=5e-5)
    assert c235.mach_surface == pytest.approx(2.146831, abs=5e-7)


@pytest.mark.parametrize("mach,delta", [(2.0, 10.0), (3.0, 20.0)])
def test_taylor_maccoll_is_numerically_identical_to_the_repository_script(mach, delta):
    """The port changes no numerics: it matches doc/scripts/rans_cone.py to rounding."""
    cp, beta_deg, mc = _load_rans_cone().taylor_maccoll(mach, delta)
    c = taylor_maccoll(mach, delta)
    assert c.cp == pytest.approx(cp, rel=1e-12)
    assert c.beta_deg == pytest.approx(beta_deg, rel=1e-12)
    assert c.mach_surface == pytest.approx(mc, rel=1e-12)


def test_taylor_maccoll_agrees_with_the_nparc_surface_mach():
    """NPARC cone10 states M3 = 2.1469; that one entry is consistent with us.

    https://www.grc.nasa.gov/www/wind/valid/cone10/cone10.html
    The solver gives 2.146831, 6.9e-5 below the printed 2.1469, which is itself
    about 2e-5 outside the rounding interval of 2.1469, so the page's solver
    tolerance is visible here. 1e-4 is the smallest round bound that holds.
    """
    assert taylor_maccoll(2.35, 10.0).mach_surface == pytest.approx(2.1469, abs=1e-4)


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
    assert c.cp == pytest.approx((c.p_pinf - 1.0) / (0.5 * 1.4 * 2.35**2), rel=1e-12)


def _nparc_cone10_state_from_its_own_mach_and_beta():
    """Surface p, T, rho ratios implied by NPARC's printed beta and M3 alone."""
    g, m1, beta, m3 = 1.4, 2.35, math.radians(27.1843), 2.1469
    mn1sq = (m1 * math.sin(beta)) ** 2
    p2_p1 = 1.0 + 2.0 * g / (g + 1.0) * (mn1sq - 1.0)
    rho2_rho1 = (g + 1.0) * mn1sq / ((g - 1.0) * mn1sq + 2.0)
    pt2_pt1 = rho2_rho1 ** (g / (g - 1.0)) * (1.0 / p2_p1) ** (1.0 / (g - 1.0))
    p = pt2_pt1 * isentropic_pt_p(m1) / isentropic_pt_p(m3)
    t = isentropic_tt_t(m1) / isentropic_tt_t(m3)
    return p, t, p / t


def test_the_nparc_cone10_table_is_internally_inconsistent():
    """NPARC cone10 prints beta 27.1843, M3 2.1469, p3/p1 1.4234, T3/T1 1.1063,
    rho3/rho1 1.2867 for M = 2.35, 10 degrees, gamma = 1.4.

    Behind a conical shock the flow is isentropic, so the surface state follows
    from M3 and the shock loss. NPARC's own beta and M3 imply p, T, rho ratios
    well away from its own p3/p1, T3/T1 and rho3/rho1, and its beta is not the
    Taylor-Maccoll shock angle. Pin that so the finding stays true.
    """
    p, t, rho = _nparc_cone10_state_from_its_own_mach_and_beta()
    assert abs(1.4234 / p - 1.0) > 0.03
    assert abs(1.1063 / t - 1.0) > 0.01
    assert abs(1.2867 / rho - 1.0) > 0.02
    # T3/T1 depends on M3 alone: the M3 that reproduces 1.1063 is not 2.1469.
    m3_implied = math.sqrt((isentropic_tt_t(2.35) / 1.1063 - 1.0) / 0.2)
    assert abs(m3_implied - 2.1469) > 0.02

    c = taylor_maccoll(2.35, 10.0)
    assert c.p_pinf == pytest.approx(1.3739, abs=1e-3)      # NPARC prints 1.4234
    assert c.t_tinf == pytest.approx(1.0951, abs=1e-3)      # NPARC prints 1.1063
    assert c.rho_rhoinf == pytest.approx(1.2546, abs=1e-3)  # NPARC prints 1.2867
    assert abs(c.beta_deg - 27.1843) > 0.4                  # NPARC prints 27.1843


def test_taylor_maccoll_rejects_a_subsonic_free_stream():
    for mach in (0.5, 1.0, float("nan")):
        with pytest.raises(ValueError, match="supersonic"):
            taylor_maccoll(mach, 10.0)


def test_taylor_maccoll_rejects_a_non_positive_or_non_physical_half_angle():
    for delta in (0.0, -5.0, 90.0, 120.0, float("nan")):
        with pytest.raises(ValueError, match="half angle"):
            taylor_maccoll(2.0, delta)


def test_taylor_maccoll_rejects_a_half_angle_beyond_detachment():
    """At M = 2 the shock detaches from a cone above about 40.7 degrees."""
    with pytest.raises(ValueError, match="detached"):
        taylor_maccoll(2.0, 45.0)
    with pytest.raises(ValueError, match="detached"):
        taylor_maccoll(1.2, 40.0)


def test_taylor_maccoll_rejects_gamma_not_above_one():
    with pytest.raises(ValueError, match="gamma"):
        taylor_maccoll(2.0, 10.0, gamma=1.0)


def test_taylor_maccoll_solves_a_large_attached_cone():
    """A 38 degree cone at M = 2 is attached (the limit is about 40.7 degrees).

    Its shock angle, about 60 degrees, lies on the rising side of the
    cone-angle curve; the shock-angle bracket must not be fooled by the falling
    side beyond the detachment maximum.
    """
    c = taylor_maccoll(2.0, 38.0)
    assert 59.5 < c.beta_deg < 60.0
    assert 0.0 < c.mach_surface < 1.0
