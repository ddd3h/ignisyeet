# SPDX-License-Identifier: GPL-3.0-or-later
"""The NASA VERIFY example of the WIND V&V tutorial, plus the degenerate cases.

Input and reference output: https://www.grc.nasa.gov/www/wind/valid/tutorial/
(prD.do, prD.out). verify.f90 declares REAL, so it is single precision and its
printed values differ from the double-precision ones in the fifth digit.
"""
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


def test_divergent_triplet_is_refused_by_the_constant_ratio_form():
    """The finer pair changes more than the coarser pair: p would be -1."""
    g = constant_ratio(1.0, 1.1, 1.15, r=2.0)
    assert g.order is None
    assert g.gci_fine is None and g.gci_coarse is None
    assert g.converged is False
    assert "not converging under refinement" in g.note
    assert g.e21 == pytest.approx(0.1) and g.e32 == pytest.approx(0.05)


def test_divergent_triplet_is_refused_by_the_variable_ratio_form():
    """Celik's abs() would turn p = -1 into +1; the screen keeps it out."""
    g = grid_convergence(1.0, 1.1, 1.15, h1=1.0, h2=2.0, h3=4.0)
    assert g.order is None
    assert g.gci_fine is None and g.gci_coarse is None
    assert g.converged is False
    assert "not converging under refinement" in g.note


def test_equal_differences_are_refused():
    g = constant_ratio(1.0, 1.1, 1.2, r=2.0)
    assert g.order is None
    assert g.extrapolated is None
    assert g.gci_fine is None
    assert g.converged is False
    assert "not converging under refinement" in g.note


def test_grid_sizes_must_increase_from_finest_to_coarsest():
    g = grid_convergence(1.0, 1.1, 1.3, h1=1.0, h2=1.0, h3=2.0)
    assert g.order is None and g.gci_fine is None
    assert g.converged is False
    assert "increase from finest to coarsest" in g.note
    r = grid_convergence(1.0, 1.1, 1.3, h1=2.0, h2=1.0, h3=0.5)
    assert r.order is None and "increase from finest to coarsest" in r.note


def test_constant_ratio_of_one_is_refused():
    g = constant_ratio(1.0, 1.1, 1.3, r=1.0)
    assert g.order is None and g.gci_fine is None
    assert g.converged is False
    assert "greater than 1" in g.note


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


# Celik, Ghia, Roache, Freitas, Coleman and Raad, "Procedure for Estimation and
# Reporting of Uncertainty Due to Discretization in CFD Applications",
# J. Fluids Eng. 130 (2008) 078001, Table 1 ("Sample calculations of discretization
# error", page 7 of the policy statement).
#
# Source read: https://qa12.asme.org/wwwasmeorg/media/ResourceFiles/Shop/Journals/JFENumAccuracy.pdf
# (ASME's own copy of the policy statement, 15 pages, sha256
# 3e22d96c3c0ac7c27e7daabcca92ca6e8b09e30d7ae7d8f24ffd273c01fa2397). The values below
# were read off the rendered table, not computed.
#
# Unlike the NASA tutorial case, r21 != r32 here, so the q(p) term in the
# fixed-point iteration is non-zero and is what these tests pin. The paper prints
# p to 2 decimals, phi_ext to 4, GCI to 1 decimal in per cent, so the tolerances are
# half a unit in the last printed digit.

# Column 1: phi = dimensionless reattachment length (monotonic convergence).
CELIK_N = (18000, 8000, 4500)
CELIK_F = (6.063, 5.972, 5.863)       # phi1, phi2, phi3
CELIK_P = 1.53                        # printed p
CELIK_EXT = 6.1685                    # printed phi_ext21
CELIK_GCI = 2.2                       # printed GCI_fine21, per cent

# Column 2: phi = axial velocity at x/H=8, y=0.0526 (p < 1).
CELIK_N_V = (18000, 4500, 980)
CELIK_F_V = (10.7880, 10.7250, 10.6050)
CELIK_P_V = 0.75
CELIK_EXT_V = 10.8801
CELIK_GCI_V = 1.1


def _celik(ns, fs):
    h = [n ** (-1 / 2) for n in ns]   # the paper's cases are 2-D: h = (A/N)**(1/2)
    return grid_convergence(*fs, h1=h[0], h2=h[1], h3=h[2], fs=1.25)


def test_variable_ratio_matches_the_celik_paper_example():
    g = _celik(CELIK_N, CELIK_F)
    assert g.converged and g.note == ""
    assert g.r21 == pytest.approx(1.5, abs=5e-3)
    assert g.r32 == pytest.approx(1.333, abs=5e-3)
    assert g.order == pytest.approx(CELIK_P, abs=5e-3)
    assert g.extrapolated == pytest.approx(CELIK_EXT, abs=5e-5)
    assert g.gci_fine * 100 == pytest.approx(CELIK_GCI, abs=0.05)
    # printed e_a21 = 1.5 %, e_ext21 = 1.7 %
    assert abs(g.e21 / CELIK_F[0]) * 100 == pytest.approx(1.5, abs=0.05)
    assert g.e_ext_fine * 100 == pytest.approx(1.7, abs=0.05)


def test_variable_ratio_matches_the_celik_paper_example_with_p_below_one():
    g = _celik(CELIK_N_V, CELIK_F_V)
    assert g.converged and g.note == ""
    assert g.r21 == pytest.approx(2.0, abs=5e-3)
    assert g.r32 == pytest.approx(2.143, abs=5e-3)
    assert g.order == pytest.approx(CELIK_P_V, abs=5e-3)
    assert g.extrapolated == pytest.approx(CELIK_EXT_V, abs=5e-5)
    assert g.gci_fine * 100 == pytest.approx(CELIK_GCI_V, abs=0.05)
    assert abs(g.e21 / CELIK_F_V[0]) * 100 == pytest.approx(0.6, abs=0.05)
    # The paper prints e_ext21 = 0.9 % here, but its own printed phi_ext21 and phi1
    # give |10.8801 - 10.7880| / 10.8801 = 0.85 %, so that cell is not asserted.
    assert g.e_ext_fine == pytest.approx(
        abs(CELIK_EXT_V - CELIK_F_V[0]) / CELIK_EXT_V, abs=1e-6)


def test_celik_oscillatory_column_is_flagged():
    # Table 1 column 3: phi = 6.0042, 5.9624, 6.0909 (oscillatory convergence).
    # The paper still prints p = 1.51 and phi_ext21 = 6.0269 there, using s = -1 in
    # the fixed-point iteration. The module refuses oscillatory triplets rather than
    # apply s = -1, and says so; this test pins that refusal, not the paper's number.
    g = _celik(CELIK_N_V, (6.0042, 5.9624, 6.0909))
    assert g.monotone is False
    assert g.order is None
    assert "oscillatory" in g.note
