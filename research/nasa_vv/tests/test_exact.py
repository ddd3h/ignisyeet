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
            assert oblique_shock(m, d).pt2_pt1 <= 1.0
