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
