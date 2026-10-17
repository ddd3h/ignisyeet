# SPDX-License-Identifier: GPL-3.0-or-later
"""The layer 0 configurations must reuse the product's numerics verbatim.

golden_euler.cfg was written by `ignisyeet aero` itself (see its header for the
command and the product commit). Only the case block may differ: the Mach
number, the markers, the mesh and output names and the dimension.
"""
import pathlib

import pytest

from nasa_vv.su2cfg import (NUMERICS_KEYS, PER_CASE_KEYS, PHYSICS_KEYS, layer0_config,
                            product_numerics)

GOLDEN = (pathlib.Path(__file__).parent / "data" / "golden_euler.cfg").read_text()

WEDGE = {"wall": ["wedge"], "far": ["farfield"]}


def active_lines(text):
    return [ln.strip() for ln in text.splitlines()
            if ln.strip() and not ln.lstrip().startswith("%")]


def keys_of(text):
    return [ln.split("=", 1)[0].strip() for ln in active_lines(text) if "=" in ln]


def test_the_golden_file_is_a_product_configuration():
    assert "MATH_PROBLEM" in GOLDEN
    assert len(product_numerics(GOLDEN)) == len(NUMERICS_KEYS)


def test_layer0_reuses_every_numerics_key_verbatim():
    cfg = layer0_config(ndim=2, axisymmetric=False, mach=2.5, markers=WEDGE,
                        viscous=False, golden=GOLDEN)
    got, want = product_numerics(cfg), product_numerics(GOLDEN)
    assert got == want
    # the lines themselves, not only the parsed values
    golden_lines = set(active_lines(GOLDEN))
    for key in NUMERICS_KEYS:
        line = next(ln for ln in active_lines(GOLDEN) if ln.startswith(key + "="))
        assert line in golden_lines
        assert line in active_lines(cfg)


def test_layer0_may_differ_only_in_dimension_and_markers():
    cfg = layer0_config(ndim=2, axisymmetric=True, mach=2.35,
                        markers={"wall": ["cone"], "far": ["farfield"]},
                        viscous=False, golden=GOLDEN)
    assert "AXISYMMETRIC= YES" in cfg
    assert "cone" in cfg
    # SU2 8.3.0 rejects NDIM as an option name: the dimension is the mesh's NDIME
    assert "NDIM" not in keys_of(cfg)


def test_a_tampered_numerics_key_is_caught():
    bad = GOLDEN.replace("CFL_NUMBER", "CFL_NUMBER_DISABLED", 1)
    with pytest.raises(KeyError):
        product_numerics(bad)
    with pytest.raises(KeyError):
        layer0_config(ndim=2, axisymmetric=False, mach=2.0, markers=WEDGE,
                      viscous=False, golden=bad)


def test_numerics_keys_are_a_subset_of_the_golden_file_and_exclude_case_keys():
    assert set(NUMERICS_KEYS) <= set(keys_of(GOLDEN))
    per_case = {"MACH_NUMBER", "AOA", "SIDESLIP_ANGLE", "SOLVER", "NDIM", "AXISYMMETRIC",
                "MESH_FILENAME", "RESTART_FILENAME", "FREESTREAM_PRESSURE",
                "FREESTREAM_TEMPERATURE", "REF_LENGTH", "REF_AREA", "ITER",
                "MARKER_EULER", "MARKER_FAR", "MARKER_SYM", "MARKER_MONITORING"}
    assert not per_case & set(NUMERICS_KEYS)
    # the keys that make the product's Euler scheme what it is
    assert {"CONV_NUM_METHOD_FLOW", "SLOPE_LIMITER_FLOW", "CFL_NUMBER",
            "LINEAR_SOLVER", "CONV_FIELD"} <= set(NUMERICS_KEYS)


def test_layer0_does_not_copy_the_goldens_per_case_keys():
    cfg = layer0_config(ndim=2, axisymmetric=False, mach=2.5, markers=WEDGE,
                        viscous=False, golden=GOLDEN, mesh="wedge.su2")
    # the rocket-specific lines of the golden file: none of them is carried over
    rocket_keys = {"MACH_NUMBER", "MESH_FILENAME", "MARKER_EULER", "MARKER_MONITORING",
                   "MARKER_SYM", "MARKER_PLOTTING", "FREESTREAM_PRESSURE",
                   "FREESTREAM_TEMPERATURE", "REF_LENGTH", "REF_AREA"}
    golden_lines = {ln.split("=", 1)[0].strip(): ln for ln in active_lines(GOLDEN)}
    assert rocket_keys <= set(golden_lines)
    for key in rocket_keys:
        assert golden_lines[key] not in active_lines(cfg), f"golden line leaked: {golden_lines[key]}"
    assert "MACH_NUMBER= 2.500000" in cfg
    assert "MACH_NUMBER= 2.000000" not in cfg
    assert "MESH_FILENAME= wedge.su2" in cfg
    assert "MARKER_EULER= ( wedge )" in cfg
    assert "MARKER_FAR= ( farfield )" in cfg
    assert "base" not in cfg and "symmetry" not in cfg  # the rocket's markers
    assert "101084.9684" not in cfg                     # the rocket's freestream pressure


def test_layer0_emits_the_mass_flux_analysis():
    cfg = layer0_config(ndim=2, axisymmetric=False, mach=2.0,
                        markers={"wall": ["wall"], "far": ["farfield"]},
                        viscous=False, golden=GOLDEN, analyze=("inlet", "outlet"))
    lines = active_lines(cfg)
    assert "MARKER_ANALYZE= ( inlet, outlet )" in lines
    assert "MARKER_ANALYZE_AVERAGE= MASSFLUX" in lines
    hist = next(ln for ln in lines if ln.startswith("HISTORY_OUTPUT="))
    assert "SURFACE_MASSFLOW" in hist  # SU2 writes Avg_Massflow(<marker>) to the history


def test_mass_flux_defaults_to_the_far_field_markers():
    cfg = layer0_config(ndim=2, axisymmetric=False, mach=2.0,
                        markers={"wall": ["wedge"], "far": ["inlet", "top", "outlet"]},
                        viscous=False, golden=GOLDEN)
    assert "MARKER_ANALYZE= ( inlet, top, outlet )" in cfg
    assert "MARKER_ANALYZE_AVERAGE= MASSFLUX" in cfg


def test_the_mass_flux_analysis_adds_no_numerics_key():
    cfg = layer0_config(ndim=2, axisymmetric=False, mach=2.0, markers=WEDGE,
                        viscous=False, golden=GOLDEN, analyze=("farfield",))
    assert product_numerics(cfg) == product_numerics(GOLDEN)


def test_a_numerics_key_is_written_once_and_only_in_the_copied_block():
    cfg = layer0_config(ndim=2, axisymmetric=True, mach=2.0, markers=WEDGE,
                        viscous=False, golden=GOLDEN)
    keys = keys_of(cfg)
    for key in NUMERICS_KEYS:
        assert keys.count(key) == 1


def test_inconsistent_requests_are_refused():
    with pytest.raises(ValueError):
        layer0_config(ndim=3, axisymmetric=True, mach=2.0, markers=WEDGE,
                      viscous=False, golden=GOLDEN)
    with pytest.raises(ValueError):
        layer0_config(ndim=2, axisymmetric=False, mach=2.0,
                      markers={"wall": ["w"]}, viscous=False, golden=GOLDEN)
    with pytest.raises(ValueError):
        layer0_config(ndim=2, axisymmetric=False, mach=2.0,
                      markers={"wall": ["w"], "far": ["f"], "bogus": ["x"]},
                      viscous=False, golden=GOLDEN)


def test_viscous_layer0_needs_a_viscous_golden_file():
    # the golden file is an Euler configuration; its numerics are not the product's RANS numerics
    with pytest.raises(NotImplementedError):
        layer0_config(ndim=2, axisymmetric=False, mach=2.0, markers=WEDGE,
                      viscous=True, golden=GOLDEN)


PINNED_NUMERICS_KEYS = (
    "NUM_METHOD_GRAD", "CONV_NUM_METHOD_FLOW", "MUSCL_FLOW", "SLOPE_LIMITER_FLOW",
    "VENKAT_LIMITER_COEFF", "LIMITER_ITER", "TIME_DISCRE_FLOW", "CFL_NUMBER", "CFL_ADAPT",
    "CFL_ADAPT_PARAM", "LINEAR_SOLVER", "LINEAR_SOLVER_PREC", "LINEAR_SOLVER_ERROR",
    "LINEAR_SOLVER_ITER", "CONV_FIELD", "CONV_RESIDUAL_MINVAL", "CONV_STARTITER",
)


def test_the_numerics_key_list_is_pinned():
    # dropping (or adding) a key must be a deliberate change to this test as well
    assert len(PINNED_NUMERICS_KEYS) == 17
    assert NUMERICS_KEYS == PINNED_NUMERICS_KEYS


def test_every_active_golden_key_is_classified_exactly_once():
    golden_keys = set(keys_of(GOLDEN))
    numerics, per_case = set(NUMERICS_KEYS), set(PER_CASE_KEYS)
    assert not numerics & per_case, f"in both sets: {sorted(numerics & per_case)}"
    unclassified = golden_keys - numerics - per_case
    assert not unclassified, f"golden keys that are neither numerics nor per case: {sorted(unclassified)}"
    # no stale entries either: every classified key occurs in the golden file
    assert numerics | per_case == golden_keys


def test_a_new_golden_key_is_unclassified_until_someone_classifies_it():
    newer = GOLDEN + "ENTROPY_FIX_COEFF= 0.001\n"
    assert set(keys_of(newer)) - set(NUMERICS_KEYS) - set(PER_CASE_KEYS) == {"ENTROPY_FIX_COEFF"}


def test_the_physics_keys_come_from_the_golden_file():
    cfg = layer0_config(ndim=2, axisymmetric=False, mach=2.0, markers=WEDGE,
                        viscous=False, golden=GOLDEN)
    golden_lines = {ln.split("=", 1)[0].strip(): ln for ln in active_lines(GOLDEN)}
    assert set(PHYSICS_KEYS) == {"FLUID_MODEL", "GAMMA_VALUE", "GAS_CONSTANT",
                                 "INIT_OPTION", "FREESTREAM_OPTION"}
    for key in PHYSICS_KEYS:
        assert golden_lines[key] in active_lines(cfg)
        assert keys_of(cfg).count(key) == 1


def test_a_changed_golden_gas_model_reaches_layer0():
    other = GOLDEN.replace("GAMMA_VALUE= 1.4", "GAMMA_VALUE= 1.3")
    assert other != GOLDEN
    cfg = layer0_config(ndim=2, axisymmetric=False, mach=2.0, markers=WEDGE,
                        viscous=False, golden=other)
    assert "GAMMA_VALUE= 1.3" in cfg
    assert "GAMMA_VALUE= 1.4" not in cfg
    with pytest.raises(KeyError):
        layer0_config(ndim=2, axisymmetric=False, mach=2.0, markers=WEDGE, viscous=False,
                      golden=GOLDEN.replace("GAS_CONSTANT", "GAS_CONSTANT_X"))
