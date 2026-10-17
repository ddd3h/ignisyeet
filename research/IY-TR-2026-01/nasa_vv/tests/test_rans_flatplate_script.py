# SPDX-License-Identifier: GPL-3.0-or-later
"""doc/scripts/rans_flatplate.py keeps a hand-written copy of the product's RANS settings. It is compared here with
what the Rust writer emitted (tests/data/golden_rans.cfg, the second-order stage, and golden_rans_startup.cfg, the
first-order stage, both at M = 0.3), and the test asserts exactly the set of differences that is documented.

The script is read only; if it changes, or if the writer does, this test fails and the set below is reviewed.
"""
import importlib.util
import pathlib


REPO = pathlib.Path(__file__).resolve().parents[4]
DATA = pathlib.Path(__file__).resolve().parent / "data"
MACH = 0.3


def _load():
    spec = importlib.util.spec_from_file_location("rans_flatplate_under_test", REPO / "doc" / "scripts" / "rans_flatplate.py")
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


SCRIPT = _load()


def parse(text):
    out = {}
    for ln in text.splitlines():
        ln = ln.strip()
        if ln and not ln.startswith("%") and "=" in ln:
            k, v = ln.split("=", 1)
            out[k.strip()] = v.strip()
    return out


def same(a, b):
    """Equal as text, or as numbers (5.0 and 5, -14.000 and -14 are formatting)."""
    if a == b:
        return True
    try:
        return a is not None and b is not None and float(a) == float(b)
    except ValueError:
        return False


def differences(script, golden):
    return {k for k in set(script) | set(golden) if not same(script.get(k), golden.get(k))}


# keys that differ because the script runs a flat plate and the golden a rocket (per case) or because of what is written out
PER_CASE = {"FREESTREAM_PRESSURE", "FREESTREAM_TEMPERATURE", "REF_LENGTH", "REF_AREA", "MARKER_FAR", "MARKER_HEATFLUX",
            "MARKER_MONITORING", "MARKER_PLOTTING", "MESH_FILENAME", "SCREEN_OUTPUT", "SCREEN_WRT_FREQ_INNER", "HISTORY_OUTPUT"}
PHYSICS = {"GAS_CONSTANT"}                   # 287.058 in the script, 287.05287 in the product
NUMERICS_FINAL = {"CONV_FIELD"}              # the script does not monitor RMS_NU_TILDE
NUMERICS_STARTUP = {"CFL_REDUCTION_TURB"}    # the product runs the SA equation at 0.2 of the flow CFL in the startup; the script at the full CFL
ITER_FINAL = {"ITER"}                        # 20000 in the script, 4000 in the product (a budget)


def test_the_final_stage_differs_from_the_golden_in_exactly_the_documented_keys():
    script = parse(SCRIPT.config(MACH, False, -14.0))
    golden = parse((DATA / "golden_rans.cfg").read_text())
    assert differences(script, golden) == PER_CASE | PHYSICS | NUMERICS_FINAL | ITER_FINAL
    assert script["CONV_FIELD"] == "RMS_DENSITY" and golden["CONV_FIELD"] == "( RMS_DENSITY, RMS_NU_TILDE )"
    assert script["GAS_CONSTANT"] == "287.05800" and golden["GAS_CONSTANT"] == "287.05287"
    assert script["ITER"] == "20000" and golden["ITER"] == "4000"


def test_the_startup_stage_differs_from_the_golden_in_exactly_the_documented_keys():
    script = parse(SCRIPT.config(MACH, True))
    golden = parse((DATA / "golden_rans_startup.cfg").read_text())
    assert differences(script, golden) == PER_CASE | PHYSICS | NUMERICS_STARTUP
    assert "CFL_REDUCTION_TURB" not in script and golden["CFL_REDUCTION_TURB"] == "0.2"


def test_every_numerics_key_of_the_product_except_the_documented_ones_agrees_with_the_script():
    from nasa_vv.su2cfg import RANS_NUMERICS_KEYS
    script = parse(SCRIPT.config(MACH, False, -14.0))
    golden = parse((DATA / "golden_rans.cfg").read_text())
    for key in RANS_NUMERICS_KEYS:
        assert same(script[key], golden[key]), key


def test_the_stop_rule_of_the_script_is_not_the_products():
    # the script stops the final stage FINAL_DROP orders below the startup peak, set per run; the product writes a floor of -14
    assert SCRIPT.FINAL_DROP == 8.0
    golden = parse((DATA / "golden_rans.cfg").read_text())
    assert golden["CONV_RESIDUAL_MINVAL"] == "-14"
    # the startup constants are the product's
    startup = parse((DATA / "golden_rans_startup.cfg").read_text())
    assert SCRIPT.STARTUP_MAX == int(startup["ITER"]) == 500
    assert SCRIPT.STARTUP_DROP == -float(startup["CONV_RESIDUAL_MINVAL"]) == 2.0


def test_the_script_writes_a_post_stage_the_product_has_not():
    post = parse(SCRIPT.config(MACH, False, post=True))
    assert post["ITER"] == "1" and post["CONV_RESIDUAL_MINVAL"] == "-30"
    assert post["SOLUTION_FILENAME"] == "restart.dat"
    golden = parse((DATA / "golden_rans.cfg").read_text())
    assert golden["SOLUTION_FILENAME"] == "restart_startup.dat" and golden["ITER"] != "1"


def test_the_script_docstring_names_a_limiter_its_configuration_does_not_write():
    assert "Venkatakrishnan-Wang limiter" in SCRIPT.__doc__
    assert parse(SCRIPT.config(MACH, False, -14.0))["SLOPE_LIMITER_FLOW"] == "VAN_ALBADA_EDGE"
