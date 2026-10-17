# SPDX-License-Identifier: GPL-3.0-or-later
"""The laminar layer 0 configuration must reuse the numerics and physics of the product's RANS configuration.

golden_rans.cfg was written by `ignisyeet aero` for a RANS case (second-order stage; see its header). IgnisYeet has no
laminar mode, so the laminar plate runs SOLVER= NAVIER_STOKES with the RANS flow numerics and gas model, copied
verbatim, and with every key that exists only because of the turbulence model left out.
"""
import pathlib

import pytest

from nasa_vv.su2cfg import (GOLDEN_RANS, PER_CASE_KEYS, RANS_MONITOR_KEYS, RANS_NUMERICS_KEYS, RANS_PER_CASE_KEYS,
                            RANS_PHYSICS_KEYS, RANS_TURBULENCE_KEYS, layer0_config, rans_gas_model,
                            rans_numerics, rans_wall_heat_flux)

DATA = pathlib.Path(__file__).parent / "data"
GOLDEN = (DATA / "golden_rans.cfg").read_text()
EULER = (DATA / "golden_euler.cfg").read_text()

PLATE = {"wall": ["plate"], "far": ["inlet", "top", "outlet"], "sym": ["upstream"]}


def active_lines(text):
    return [ln.strip() for ln in text.splitlines() if ln.strip() and not ln.lstrip().startswith("%")]


def keys_of(text):
    return [ln.split("=", 1)[0].strip() for ln in active_lines(text) if "=" in ln]


def laminar(golden=GOLDEN, **kw):
    args = dict(ndim=2, axisymmetric=False, mach=0.2, markers=PLATE, viscous=True, golden=golden)
    args.update(kw)
    return layer0_config(**args)


def test_the_rans_golden_is_a_product_rans_configuration():
    assert GOLDEN_RANS == DATA / "golden_rans.cfg"
    lines = active_lines(GOLDEN)
    assert "SOLVER= RANS" in lines and "KIND_TURB_MODEL= SA" in lines
    assert any(ln.startswith("VISCOSITY_MODEL= SUTHERLAND") for ln in lines)
    assert len(rans_numerics(GOLDEN)) == len(RANS_NUMERICS_KEYS) + len(RANS_MONITOR_KEYS)


# ---- the classification: every active key of the RANS golden, exactly once

PINNED_RANS_NUMERICS_KEYS = (
    # spatial scheme: Green-Gauss gradients, weighted-least-squares reconstruction gradients, accurate Roe, MUSCL, Van Albada
    "NUM_METHOD_GRAD", "NUM_METHOD_GRAD_RECON", "CONV_NUM_METHOD_FLOW", "MUSCL_FLOW", "SLOPE_LIMITER_FLOW",
    "USE_ACCURATE_FLUX_JACOBIANS",
    # time discretisation and CFL
    "TIME_DISCRE_FLOW", "CFL_NUMBER", "CFL_ADAPT", "CFL_ADAPT_PARAM",
    # linear solver
    "LINEAR_SOLVER", "LINEAR_SOLVER_PREC", "LINEAR_SOLVER_ILU_FILL_IN", "LINEAR_SOLVER_ERROR", "LINEAR_SOLVER_ITER",
    # convergence monitor
    "CONV_RESIDUAL_MINVAL", "CONV_STARTITER",
)
PINNED_RANS_MONITOR_KEYS = ("CONV_FIELD",)
PINNED_RANS_TURBULENCE_KEYS = (
    "KIND_TURB_MODEL", "SA_OPTIONS", "FREESTREAM_NU_FACTOR",
    "CONV_NUM_METHOD_TURB", "MUSCL_TURB", "TIME_DISCRE_TURB",
)
PINNED_RANS_PHYSICS_KEYS = (
    "FLUID_MODEL", "GAMMA_VALUE", "GAS_CONSTANT", "INIT_OPTION", "FREESTREAM_OPTION",
    "VISCOSITY_MODEL", "MU_REF", "MU_T_REF", "SUTHERLAND_CONSTANT",
)


def test_the_rans_classification_is_pinned():
    # dropping (or adding) a key must be a deliberate change to this test as well
    assert len(PINNED_RANS_NUMERICS_KEYS) == 17
    assert RANS_NUMERICS_KEYS == PINNED_RANS_NUMERICS_KEYS
    assert RANS_MONITOR_KEYS == PINNED_RANS_MONITOR_KEYS
    assert RANS_TURBULENCE_KEYS == PINNED_RANS_TURBULENCE_KEYS
    assert RANS_PHYSICS_KEYS == PINNED_RANS_PHYSICS_KEYS


def classes():
    return {"numerics": set(RANS_NUMERICS_KEYS), "monitor": set(RANS_MONITOR_KEYS),
            "turbulence": set(RANS_TURBULENCE_KEYS), "physics": set(RANS_PHYSICS_KEYS),
            "per case": set(RANS_PER_CASE_KEYS)}


def test_every_active_rans_golden_key_is_classified_exactly_once():
    golden_keys = keys_of(GOLDEN)
    assert len(golden_keys) == len(set(golden_keys)), "a key occurs twice in the golden file"
    cls = classes()
    names = list(cls)
    for i, a in enumerate(names):
        for b in names[i + 1:]:
            assert not cls[a] & cls[b], f"in both {a} and {b}: {sorted(cls[a] & cls[b])}"
    union = set().union(*cls.values())
    assert not set(golden_keys) - union, f"unclassified golden keys: {sorted(set(golden_keys) - union)}"
    assert union == set(golden_keys), f"classified but not in the golden file: {sorted(union - set(golden_keys))}"


def test_a_new_rans_golden_key_is_unclassified_until_someone_classifies_it():
    newer = GOLDEN + "PRANDTL_LAM= 0.72\n"
    union = set().union(*classes().values())
    assert set(keys_of(newer)) - union == {"PRANDTL_LAM"}


def test_the_euler_classification_is_untouched_and_disjoint_from_the_rans_flow_keys():
    # per-case keys shared by both goldens are the same words; the Euler pin test lives in test_su2cfg.py
    assert "MARKER_EULER" in PER_CASE_KEYS and "MARKER_EULER" not in RANS_PER_CASE_KEYS
    assert "MARKER_HEATFLUX" in RANS_PER_CASE_KEYS and "MARKER_HEATFLUX" not in PER_CASE_KEYS


def test_the_turbulence_keys_name_the_turbulence_model_only():
    assert {"KIND_TURB_MODEL", "SA_OPTIONS", "FREESTREAM_NU_FACTOR"} <= set(RANS_TURBULENCE_KEYS)
    assert all(("TURB" in k) or k in ("SA_OPTIONS", "FREESTREAM_NU_FACTOR") for k in RANS_TURBULENCE_KEYS)
    assert not set(RANS_TURBULENCE_KEYS) & set(RANS_NUMERICS_KEYS)


# ---- the laminar configuration

def test_laminar_copies_every_rans_numerics_line_verbatim():
    cfg = laminar()
    golden_lines = active_lines(GOLDEN)
    for key in RANS_NUMERICS_KEYS:
        line = next(ln for ln in golden_lines if ln.startswith(key + "="))
        assert active_lines(cfg).count(line) == 1, line
    assert rans_numerics(cfg)["CFL_ADAPT_PARAM"] == rans_numerics(GOLDEN)["CFL_ADAPT_PARAM"]


def test_laminar_copies_the_gas_model_and_viscosity_law_verbatim():
    cfg = laminar()
    golden_lines = active_lines(GOLDEN)
    for key in RANS_PHYSICS_KEYS:
        line = next(ln for ln in golden_lines if ln.startswith(key + "="))
        assert active_lines(cfg).count(line) == 1, line
    assert "VISCOSITY_MODEL= SUTHERLAND" in active_lines(cfg)


def test_laminar_has_no_turbulence_key_and_no_turbulence_residual():
    cfg = laminar()
    assert "SOLVER= NAVIER_STOKES" in active_lines(cfg)
    assert "SOLVER= RANS" not in cfg
    for key in RANS_TURBULENCE_KEYS:
        assert key not in keys_of(cfg), key
    assert "NU_TILDE" not in cfg
    assert not any("TURB" in k for k in keys_of(cfg))
    assert "CONV_FIELD= RMS_DENSITY" in active_lines(cfg)


def test_laminar_writes_each_golden_derived_key_once():
    keys = keys_of(laminar())
    for key in RANS_NUMERICS_KEYS + RANS_MONITOR_KEYS + RANS_PHYSICS_KEYS:
        assert keys.count(key) == 1, key
    assert len(keys) == len(set(keys))


def test_laminar_writes_no_prandtl_number_because_the_product_does_not():
    # neither golden sets PRANDTL_LAM; SU2's default applies in the product and here
    assert not any(k.startswith("PRANDTL") for k in keys_of(GOLDEN))
    assert not any(k.startswith("PRANDTL") for k in keys_of(laminar()))


def test_the_plate_is_a_no_slip_wall_with_the_goldens_heat_flux():
    cfg = laminar()
    assert rans_wall_heat_flux(GOLDEN) == 0.0                   # adiabatic
    assert "MARKER_HEATFLUX= ( plate, 0.0 )" in active_lines(cfg)
    assert "MARKER_EULER" not in cfg and "MARKER_WALL" not in cfg
    assert "MARKER_MONITORING= ( plate )" in active_lines(cfg)
    assert "MARKER_SYM= ( upstream )" in active_lines(cfg)
    assert "MARKER_FAR= ( inlet, top, outlet )" in active_lines(cfg)


def test_a_changed_heat_flux_of_the_golden_reaches_the_plate_and_mixed_values_are_refused():
    other = GOLDEN.replace("MARKER_HEATFLUX= ( wall, 0.0, base, 0.0 )", "MARKER_HEATFLUX= ( wall, 2.5, base, 2.5 )")
    assert other != GOLDEN
    assert "MARKER_HEATFLUX= ( plate, 2.5 )" in laminar(other)
    mixed = GOLDEN.replace("MARKER_HEATFLUX= ( wall, 0.0, base, 0.0 )", "MARKER_HEATFLUX= ( wall, 0.0, base, 3.0 )")
    with pytest.raises(ValueError):
        laminar(mixed)
    with pytest.raises(KeyError):
        laminar(GOLDEN.replace("MARKER_HEATFLUX", "MARKER_ISOTHERMAL"))


def test_laminar_does_not_copy_the_goldens_per_case_keys():
    cfg = laminar(mesh="plate.su2")
    golden_lines = {ln.split("=", 1)[0].strip(): ln for ln in active_lines(GOLDEN)}
    for key in ("MACH_NUMBER", "MESH_FILENAME", "MARKER_HEATFLUX", "MARKER_MONITORING", "MARKER_FAR", "MARKER_SYM",
                "MARKER_PLOTTING", "FREESTREAM_PRESSURE", "FREESTREAM_TEMPERATURE", "REF_LENGTH", "REF_AREA",
                "ITER", "RESTART_SOL", "SOLUTION_FILENAME", "SOLVER"):
        assert golden_lines[key] not in active_lines(cfg), f"golden line leaked: {golden_lines[key]}"
    assert "MACH_NUMBER= 0.200000" in cfg and "MESH_FILENAME= plate.su2" in cfg
    assert "RESTART_SOL= NO" in cfg and "SOLUTION_FILENAME" not in cfg
    assert "symmetry" not in cfg and "base" not in cfg           # the rocket's markers
    assert "101084.9684" not in cfg                              # the rocket's freestream pressure


def test_the_iteration_budget_and_the_freestream_are_per_case():
    cfg = laminar(iterations=12345, pressure=2200.0, temperature=300.0)
    assert "ITER= 12345" in cfg
    assert "FREESTREAM_PRESSURE= 2200.0000" in cfg and "FREESTREAM_TEMPERATURE= 300.0000" in cfg


def test_a_tampered_rans_numerics_key_is_caught():
    for key in ("USE_ACCURATE_FLUX_JACOBIANS", "SLOPE_LIMITER_FLOW", "CFL_ADAPT_PARAM"):
        bad = GOLDEN.replace(key, key + "_DISABLED", 1)
        with pytest.raises(KeyError, match=key):
            rans_numerics(bad)
        with pytest.raises(KeyError, match=key):
            laminar(bad)


def test_a_changed_golden_viscosity_law_reaches_the_laminar_config():
    other = GOLDEN.replace("SUTHERLAND_CONSTANT= 110.4", "SUTHERLAND_CONSTANT= 120.0")
    assert other != GOLDEN
    assert "SUTHERLAND_CONSTANT= 120.0" in laminar(other)
    with pytest.raises(KeyError):
        laminar(GOLDEN.replace("MU_REF", "MU_REF_X"))


def test_the_monitor_field_is_the_goldens_with_the_turbulence_residual_removed():
    assert "CONV_FIELD= ( RMS_DENSITY, RMS_NU_TILDE )" in active_lines(GOLDEN)
    assert rans_numerics(GOLDEN)["CONV_FIELD"] == "RMS_DENSITY"
    # a golden that monitors something else is not silently reduced to the density
    other = GOLDEN.replace("CONV_FIELD= ( RMS_DENSITY, RMS_NU_TILDE )", "CONV_FIELD= ( RMS_ENERGY, RMS_NU_TILDE )")
    with pytest.raises(ValueError):
        laminar(other)
    with pytest.raises(ValueError):
        rans_numerics(other)


def test_viscous_layer0_still_refuses_the_euler_golden_and_inviscid_the_rans_golden():
    with pytest.raises(NotImplementedError):
        layer0_config(ndim=2, axisymmetric=False, mach=0.2, markers=PLATE, viscous=True, golden=EULER)
    with pytest.raises(ValueError):
        layer0_config(ndim=2, axisymmetric=False, mach=2.0, markers={"wall": ["w"], "far": ["f"]},
                      viscous=False, golden=GOLDEN)


def test_the_euler_path_is_unchanged_by_the_rans_golden_existing():
    cfg = layer0_config(ndim=2, axisymmetric=False, mach=2.5, markers={"wall": ["wedge"], "far": ["farfield"]},
                        viscous=False, golden=EULER)
    assert "SOLVER= EULER" in cfg and "MARKER_EULER= ( wedge )" in cfg
    assert "MARKER_HEATFLUX" not in cfg and "VISCOSITY_MODEL" not in cfg


def test_the_gas_model_is_read_from_the_golden():
    g = rans_gas_model(GOLDEN)
    assert g == {"gamma": 1.4, "gas_constant": 287.05287, "mu_ref": 1.716e-5, "t_ref": 273.15, "sutherland": 110.4}
    assert rans_gas_model(GOLDEN.replace("GAMMA_VALUE= 1.4", "GAMMA_VALUE= 1.3"))["gamma"] == 1.3
    with pytest.raises(KeyError):
        rans_gas_model(GOLDEN.replace("SUTHERLAND_CONSTANT", "SC"))
    with pytest.raises(ValueError):
        rans_gas_model(GOLDEN.replace("VISCOSITY_MODEL= SUTHERLAND", "VISCOSITY_MODEL= CONSTANT_VISCOSITY"))


def test_inconsistent_laminar_requests_are_refused():
    with pytest.raises(ValueError):
        laminar(markers={"wall": ["w"]})
    with pytest.raises(ValueError):
        laminar(ndim=3, axisymmetric=True)


# ---- the turbulent configuration: the golden's own lines, the turbulence model the only difference

from nasa_vv.su2cfg import (GOLDEN_RANS_STARTUP, RANS_STOP_RULE_KEYS, SST_KEYS, rans_stage_config)  # noqa: E402

STARTUP = (DATA / "golden_rans_startup.cfg").read_text()
SST = {"SST_OPTIONS": "V1994m", "FREESTREAM_TURBULENCEINTENSITY": "0.00038729", "FREESTREAM_TURB2LAMVISCRATIO": "0.009"}
CASE = {
    "MACH_NUMBER": "0.200000", "FREESTREAM_PRESSURE": "1234.5000", "FREESTREAM_TEMPERATURE": "300.0000",
    "REF_LENGTH": "1.0", "REF_AREA": "2.0", "MARKER_HEATFLUX": "( wall, 0.0 )", "MARKER_MONITORING": "( wall )",
    "MARKER_FAR": "( inlet, top, outlet )", "MARKER_SYM": "( symmetry )", "MARKER_PLOTTING": "( wall )",
    "ITER": "30000", "MESH_FILENAME": "mesh.su2", "SCREEN_OUTPUT": "( INNER_ITER, RMS_DENSITY, DRAG )",
}
STOP = {"CONV_FIELD": "RMS_DENSITY", "CONV_RESIDUAL_MINVAL": "-9.5"}


def stage(model, golden=GOLDEN, **kw):
    kw.setdefault("case", CASE)
    if model == "SST":
        kw.setdefault("sst", SST)
    return rans_stage_config(golden, model=model, **kw)


def as_dict(text):
    return {ln.split("=", 1)[0].strip(): ln.split("=", 1)[1].strip() for ln in active_lines(text) if "=" in ln}


def test_the_stage_config_of_sa_changes_only_the_case_keys():
    sa = as_dict(stage("SA"))
    gold = as_dict(GOLDEN)
    assert set(sa) == set(gold)
    changed = {k for k in gold if sa[k] != gold[k]}
    assert changed == set(CASE) - {k for k in CASE if CASE[k] == gold[k]}
    assert sa["KIND_TURB_MODEL"] == "SA" and sa["SA_OPTIONS"] == "( NEGATIVE, WITHFT2 )" and sa["FREESTREAM_NU_FACTOR"] == "3.0"


def test_every_numerics_physics_and_turbulence_numerics_line_is_verbatim_for_both_models():
    for model in ("SA", "SST"):
        lines = active_lines(stage(model))
        for key in RANS_NUMERICS_KEYS + RANS_PHYSICS_KEYS + ("CONV_NUM_METHOD_TURB", "MUSCL_TURB", "TIME_DISCRE_TURB"):
            line = next(ln for ln in active_lines(GOLDEN) if ln.startswith(key + "="))
            assert lines.count(line) == 1, (model, line)


def test_sa_and_sst_differ_in_exactly_the_turbulence_model_keys():
    sa, sst = as_dict(stage("SA", stop=STOP)), as_dict(stage("SST", stop=STOP))
    only_sa = {k for k in sa if k not in sst}
    only_sst = {k for k in sst if k not in sa}
    assert only_sa == {"SA_OPTIONS", "FREESTREAM_NU_FACTOR"}
    assert only_sst == set(SST_KEYS)
    assert {k for k in sa if k in sst and sa[k] != sst[k]} == {"KIND_TURB_MODEL"}
    assert sst["KIND_TURB_MODEL"] == "SST"


def test_the_sst_keys_sit_where_the_sa_keys_were():
    lines = [ln.split("=")[0] for ln in active_lines(stage("SST"))]
    assert lines.index("SST_OPTIONS") == lines.index("KIND_TURB_MODEL") + 1
    gold = [ln.split("=")[0] for ln in active_lines(GOLDEN)]
    assert lines.index("FREESTREAM_TURBULENCEINTENSITY") == gold.index("FREESTREAM_NU_FACTOR")
    assert lines.index("FREESTREAM_TURB2LAMVISCRATIO") == lines.index("FREESTREAM_TURBULENCEINTENSITY") + 1


def test_the_stop_rule_is_the_same_for_both_models_and_is_the_only_numerics_override():
    assert set(RANS_STOP_RULE_KEYS) == {"CONV_FIELD", "CONV_RESIDUAL_MINVAL"}
    for model in ("SA", "SST"):
        d = as_dict(stage(model, stop=STOP))
        assert d["CONV_FIELD"] == "RMS_DENSITY" and d["CONV_RESIDUAL_MINVAL"] == "-9.5"


def test_the_startup_stage_is_built_from_the_startup_golden_and_keeps_its_reduced_turbulence_cfl():
    assert GOLDEN_RANS_STARTUP == DATA / "golden_rans_startup.cfg"
    for model in ("SA", "SST"):
        d = as_dict(stage(model, golden=STARTUP))
        assert d["CFL_REDUCTION_TURB"] == "0.2" and d["MUSCL_FLOW"] == "NO" and d["RESTART_SOL"] == "NO"


def test_an_override_of_a_numerics_key_is_refused():
    for key in ("CFL_NUMBER", "MUSCL_FLOW", "CONV_NUM_METHOD_FLOW", "LINEAR_SOLVER_ERROR", "GAS_CONSTANT", "SA_OPTIONS"):
        with pytest.raises(ValueError, match=key):
            rans_stage_config(GOLDEN, model="SA", case={key: "1"})
    with pytest.raises(ValueError, match="CFL_NUMBER"):
        rans_stage_config(GOLDEN, model="SA", case=CASE, stop={"CFL_NUMBER": "1"})


def test_an_override_of_a_key_the_golden_does_not_have_is_refused():
    with pytest.raises(KeyError, match="MARKER_EULER"):
        rans_stage_config(GOLDEN, model="SA", case={"MARKER_EULER": "( wall )"})


def test_the_sst_keys_must_all_be_given_for_sst_and_none_for_sa():
    with pytest.raises(ValueError, match="SST"):
        rans_stage_config(GOLDEN, model="SST", case=CASE)
    with pytest.raises(ValueError, match="SST"):
        rans_stage_config(GOLDEN, model="SST", case=CASE, sst={"SST_OPTIONS": "V1994m"})
    with pytest.raises(ValueError, match="SA"):
        rans_stage_config(GOLDEN, model="SA", case=CASE, sst=SST)
    with pytest.raises(ValueError, match="model"):
        rans_stage_config(GOLDEN, model="KE", case=CASE)


def test_the_euler_golden_is_refused():
    with pytest.raises(ValueError, match="RANS"):
        rans_stage_config(EULER, model="SA", case={})
