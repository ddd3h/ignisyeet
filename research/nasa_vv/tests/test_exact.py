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
import math

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
