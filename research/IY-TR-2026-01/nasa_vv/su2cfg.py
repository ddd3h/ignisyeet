# SPDX-License-Identifier: GPL-3.0-or-later
"""SU2 configurations for the layer 0 cases, pinned to the product's numerics.

Layer 0 runs SU2 directly on problems with exact solutions (wedge, cone,
expansion corner). It verifies the solver *with IgnisYeet's numerical
settings* only if those settings really are the product's. The numerics block
is therefore never written here: it is copied line for line from a
configuration that `ignisyeet aero` itself wrote (tests/data/golden_euler.cfg,
Euler, written by crates/cfd/src/su2cfg.rs). Everything else is the case block
of the layer 0 problem.

NUMERICS_KEYS are the keys that occur in the golden file and govern the
numerics. The per-case keys (Mach number, angle of attack, freestream state,
reference values, markers, mesh and output names, the iteration cap, SOLVER)
are not among them. The golden file also lacks NUM_METHOD_GRAD_RECON (RANS
with Roe only), JST_SENSOR_COEFF (JST only) and ENTROPY_FIX_COEFF (never
written), so they are not listed.

Every active key of the golden file is classified: in NUMERICS_KEYS, or in
PER_CASE_KEYS. A golden file with a key in neither fails the completeness
test, so a new numerics key cannot enter the product unnoticed.

Viscous cases
-------------
IgnisYeet has no laminar mode: its flow model is Euler or RANS (Spalart-Allmaras). A laminar layer 0 case
(``viscous=True``) is therefore not a configuration the product writes. It runs SOLVER= NAVIER_STOKES with the flow
numerics and the gas model of a RANS configuration the product wrote (tests/data/golden_rans.cfg, the second-order
stage written by crates/cfd/src/su2cfg.rs), copied verbatim, and with every key that exists only because of the
turbulence model left out. It verifies the product's RANS discretisation with the turbulence model removed.

The RANS golden has its own classification with the same discipline (RANS_NUMERICS_KEYS, RANS_MONITOR_KEYS,
RANS_TURBULENCE_KEYS, RANS_PHYSICS_KEYS, RANS_PER_CASE_KEYS; every active key exactly once). One key does not split
cleanly: CONV_FIELD monitors ( RMS_DENSITY, RMS_NU_TILDE ). It is not copied and not omitted; the laminar value is
derived by removing the turbulence residual, and it must leave exactly RMS_DENSITY. The golden has no PRANDTL_LAM, so
SU2's default applies in the product and in the laminar case; the laminar case writes none either. The wall is the
product's wall condition: no slip, MARKER_HEATFLUX with the golden's own (adiabatic, 0.0) value.
"""
from __future__ import annotations

import pathlib
from typing import Iterable, Mapping, Sequence

GOLDEN_EULER = pathlib.Path(__file__).resolve().parent / "tests" / "data" / "golden_euler.cfg"

NUMERICS_KEYS: tuple[str, ...] = (
    # spatial scheme: Roe flux, MUSCL reconstruction, Venkatakrishnan limiter frozen at LIMITER_ITER
    "NUM_METHOD_GRAD",
    "CONV_NUM_METHOD_FLOW",
    "MUSCL_FLOW",
    "SLOPE_LIMITER_FLOW",
    "VENKAT_LIMITER_COEFF",
    "LIMITER_ITER",
    # time discretisation and CFL
    "TIME_DISCRE_FLOW",
    "CFL_NUMBER",
    "CFL_ADAPT",
    "CFL_ADAPT_PARAM",
    # linear solver
    "LINEAR_SOLVER",
    "LINEAR_SOLVER_PREC",
    "LINEAR_SOLVER_ERROR",
    "LINEAR_SOLVER_ITER",
    # convergence monitor
    "CONV_FIELD",
    "CONV_RESIDUAL_MINVAL",
    "CONV_STARTITER",
)

# Every other active key of the golden file. They legitimately differ per case, or are written by layer0_config
# itself (case block). tests/test_su2cfg.py requires NUMERICS_KEYS and PER_CASE_KEYS to partition the golden's keys.
PER_CASE_KEYS: frozenset[str] = frozenset({
    "SOLVER", "MATH_PROBLEM", "SYSTEM_MEASUREMENTS", "RESTART_SOL",
    "MACH_NUMBER", "AOA", "SIDESLIP_ANGLE",
    "FLUID_MODEL", "GAMMA_VALUE", "GAS_CONSTANT", "INIT_OPTION", "FREESTREAM_OPTION",
    "FREESTREAM_PRESSURE", "FREESTREAM_TEMPERATURE", "REF_DIMENSIONALIZATION",
    "REF_ORIGIN_MOMENT_X", "REF_ORIGIN_MOMENT_Y", "REF_ORIGIN_MOMENT_Z",
    "REF_LENGTH", "REF_AREA",
    "MARKER_EULER", "MARKER_MONITORING", "MARKER_FAR", "MARKER_SYM", "MARKER_PLOTTING",
    "ITER",
    "MESH_FILENAME", "MESH_FORMAT", "RESTART_FILENAME", "CONV_FILENAME", "SURFACE_FILENAME",
    "TABULAR_FORMAT", "OUTPUT_FILES", "HISTORY_OUTPUT", "SCREEN_OUTPUT", "VOLUME_OUTPUT",
    "SCREEN_WRT_FREQ_INNER", "HISTORY_WRT_FREQ_INNER", "OUTPUT_WRT_FREQ",
})

# The gas model and its initialisation: not numerics, but if they differed layer 0 would solve other physics than
# the product. layer0_config copies them from the golden file too.
PHYSICS_KEYS: tuple[str, ...] = (
    "FLUID_MODEL", "GAMMA_VALUE", "GAS_CONSTANT", "INIT_OPTION", "FREESTREAM_OPTION",
)

# marker kinds accepted by layer0_config
_MARKER_KINDS = ("wall", "far", "sym")

GOLDEN_RANS = pathlib.Path(__file__).resolve().parent / "tests" / "data" / "golden_rans.cfg"

# ---- the RANS golden (second-order stage of a cold RANS case): the same discipline, its own classification

# Flow numerics, copied verbatim into a laminar case.
RANS_NUMERICS_KEYS: tuple[str, ...] = (
    # spatial scheme: Green-Gauss gradients, weighted-least-squares reconstruction gradients, accurate Roe, MUSCL, Van Albada
    "NUM_METHOD_GRAD",
    "NUM_METHOD_GRAD_RECON",
    "CONV_NUM_METHOD_FLOW",
    "MUSCL_FLOW",
    "SLOPE_LIMITER_FLOW",
    "USE_ACCURATE_FLUX_JACOBIANS",
    # time discretisation and CFL
    "TIME_DISCRE_FLOW",
    "CFL_NUMBER",
    "CFL_ADAPT",
    "CFL_ADAPT_PARAM",
    # linear solver
    "LINEAR_SOLVER",
    "LINEAR_SOLVER_PREC",
    "LINEAR_SOLVER_ILU_FILL_IN",
    "LINEAR_SOLVER_ERROR",
    "LINEAR_SOLVER_ITER",
    # convergence monitor
    "CONV_RESIDUAL_MINVAL",
    "CONV_STARTITER",
)

# The one key that is neither copied nor omitted: its value names a turbulence residual (see the module docstring).
RANS_MONITOR_KEYS: tuple[str, ...] = ("CONV_FIELD",)

# Keys that exist only because of the turbulence model: never copied into a laminar case.
RANS_TURBULENCE_KEYS: tuple[str, ...] = (
    "KIND_TURB_MODEL",
    "SA_OPTIONS",
    "FREESTREAM_NU_FACTOR",
    "CONV_NUM_METHOD_TURB",
    "MUSCL_TURB",
    "TIME_DISCRE_TURB",
)

# Gas model, viscosity law and its initialisation, copied verbatim. (PRANDTL_LAM is not in the golden: SU2's default.)
RANS_PHYSICS_KEYS: tuple[str, ...] = (
    "FLUID_MODEL", "GAMMA_VALUE", "GAS_CONSTANT", "INIT_OPTION", "FREESTREAM_OPTION",
    "VISCOSITY_MODEL", "MU_REF", "MU_T_REF", "SUTHERLAND_CONSTANT",
)

# Every other active key of the RANS golden. MARKER_HEATFLUX carries the wall condition: layer0_config writes it
# for the case's wall markers with the golden's own value (rans_wall_heat_flux).
RANS_PER_CASE_KEYS: frozenset[str] = frozenset({
    "SOLVER", "MATH_PROBLEM", "SYSTEM_MEASUREMENTS", "RESTART_SOL", "SOLUTION_FILENAME",
    "MACH_NUMBER", "AOA", "SIDESLIP_ANGLE",
    "FREESTREAM_PRESSURE", "FREESTREAM_TEMPERATURE", "REF_DIMENSIONALIZATION",
    "REF_ORIGIN_MOMENT_X", "REF_ORIGIN_MOMENT_Y", "REF_ORIGIN_MOMENT_Z",
    "REF_LENGTH", "REF_AREA",
    "MARKER_HEATFLUX", "MARKER_MONITORING", "MARKER_FAR", "MARKER_SYM", "MARKER_PLOTTING",
    "ITER",
    "MESH_FILENAME", "MESH_FORMAT", "RESTART_FILENAME", "CONV_FILENAME", "SURFACE_FILENAME",
    "TABULAR_FORMAT", "OUTPUT_FILES", "HISTORY_OUTPUT", "SCREEN_OUTPUT", "VOLUME_OUTPUT",
    "SCREEN_WRT_FREQ_INNER", "HISTORY_WRT_FREQ_INNER", "OUTPUT_WRT_FREQ",
})

_TURBULENCE_RESIDUALS = ("RMS_NU_TILDE",)

GOLDEN_RANS_STARTUP = pathlib.Path(__file__).resolve().parent / "tests" / "data" / "golden_rans_startup.cfg"
"""The first-order startup stage `ignisyeet aero` wrote in the same run as golden_rans.cfg."""

# ---- a turbulent case under either turbulence model: the golden's own lines, the model the only difference

SST_KEYS: tuple[str, ...] = ("SST_OPTIONS", "FREESTREAM_TURBULENCEINTENSITY", "FREESTREAM_TURB2LAMVISCRATIO")
"""The keys SST needs and Spalart-Allmaras does not (SA_OPTIONS and FREESTREAM_NU_FACTOR take their place)."""

RANS_STOP_RULE_KEYS: tuple[str, ...] = ("CONV_FIELD", "CONV_RESIDUAL_MINVAL")
"""The two numerics keys a verification run may override: when it stops. They name no turbulence quantity once set to
RMS_DENSITY and a floor, so the stop rule is the same for both models."""


def _active_lines(text: str) -> Iterable[str]:
    """The non-empty lines that are not SU2 comments (a leading %)."""
    for raw in text.splitlines():
        line = raw.strip()
        if line and not line.startswith("%"):
            yield line


def _parse(text: str) -> dict[str, tuple[str, str]]:
    """key -> (verbatim line, value) of every KEY= VALUE line of a configuration."""
    out: dict[str, tuple[str, str]] = {}
    for line in _active_lines(text):
        if "=" not in line:
            continue
        key, value = line.split("=", 1)
        out[key.strip()] = (line, value.strip())
    return out


def product_numerics(cfg_text: str) -> dict[str, str]:
    """The values of NUMERICS_KEYS in a configuration.

    Raises KeyError naming the keys that are missing, so a configuration that
    no longer carries the product's numerics cannot pass unnoticed.
    """
    parsed = _parse(cfg_text)
    missing = [k for k in NUMERICS_KEYS if k not in parsed]
    if missing:
        raise KeyError(f"numerics keys missing from the configuration: {', '.join(missing)}")
    return {k: parsed[k][1] for k in NUMERICS_KEYS}


def _verbatim(golden: str, keys: Sequence[str]) -> list[str]:
    """The golden file's own lines of `keys`; KeyError naming the missing ones."""
    parsed = _parse(golden)
    missing = [k for k in keys if k not in parsed]
    if missing:
        raise KeyError(f"keys missing from the golden configuration: {', '.join(missing)}")
    return [parsed[k][0] for k in keys]


def _is_rans(golden: str) -> bool:
    return _parse(golden).get("SOLVER", ("", ""))[1] == "RANS"


def _list_items(value: str) -> list[str]:
    """The items of an SU2 list value, '( a, b )' or 'a'."""
    return [s.strip() for s in value.strip().strip("()").split(",") if s.strip()]


def _laminar_monitor(value: str) -> str:
    """CONV_FIELD of a laminar case: the golden's list without the turbulence residuals.

    Raises ValueError if what is left is not exactly RMS_DENSITY (the golden then monitors something this function
    has no right to reduce).
    """
    left = [f for f in _list_items(value) if f not in _TURBULENCE_RESIDUALS]
    if left != ["RMS_DENSITY"]:
        raise ValueError(f"CONV_FIELD= {value}: without the turbulence residuals {left} is left, expected RMS_DENSITY")
    return "RMS_DENSITY"


def rans_numerics(cfg_text: str) -> dict[str, str]:
    """The values of RANS_NUMERICS_KEYS and CONV_FIELD in a RANS or laminar configuration.

    CONV_FIELD is reduced to RMS_DENSITY (see _laminar_monitor), so the golden and a laminar case built from it
    compare equal. Raises KeyError naming the missing keys.
    """
    parsed = _parse(cfg_text)
    keys = RANS_NUMERICS_KEYS + RANS_MONITOR_KEYS
    missing = [k for k in keys if k not in parsed]
    if missing:
        raise KeyError(f"RANS numerics keys missing from the configuration: {', '.join(missing)}")
    out = {k: parsed[k][1] for k in RANS_NUMERICS_KEYS}
    out["CONV_FIELD"] = _laminar_monitor(parsed["CONV_FIELD"][1])
    return out


def _heat_flux_token(golden: str) -> str:
    parsed = _parse(golden)
    if "MARKER_HEATFLUX" not in parsed:
        raise KeyError("keys missing from the golden configuration: MARKER_HEATFLUX")
    items = _list_items(parsed["MARKER_HEATFLUX"][1])
    if len(items) < 2 or len(items) % 2:
        raise ValueError(f"MARKER_HEATFLUX= {parsed['MARKER_HEATFLUX'][1]} is not a list of (marker, value) pairs")
    values = items[1::2]
    if len({float(v) for v in values}) != 1:
        raise ValueError(f"the golden's wall markers have different heat fluxes {values}; a single wall condition is needed")
    return values[0]


def rans_wall_heat_flux(golden: str) -> float:
    """The heat flux of the golden's walls (0.0: adiabatic); ValueError if its markers differ, KeyError if absent."""
    return float(_heat_flux_token(golden))


def rans_gas_model(golden: str) -> dict[str, float]:
    """gamma, gas constant and the Sutherland law (mu_ref, t_ref, sutherland) of a RANS golden.

    ValueError unless the viscosity law is SUTHERLAND; KeyError for a missing key.
    """
    parsed = _parse(golden)
    need = ("GAMMA_VALUE", "GAS_CONSTANT", "VISCOSITY_MODEL", "MU_REF", "MU_T_REF", "SUTHERLAND_CONSTANT")
    missing = [k for k in need if k not in parsed]
    if missing:
        raise KeyError(f"keys missing from the golden configuration: {', '.join(missing)}")
    if parsed["VISCOSITY_MODEL"][1] != "SUTHERLAND":
        raise ValueError(f"viscosity model {parsed['VISCOSITY_MODEL'][1]}; only SUTHERLAND is handled")
    return {"gamma": float(parsed["GAMMA_VALUE"][1]), "gas_constant": float(parsed["GAS_CONSTANT"][1]),
            "mu_ref": float(parsed["MU_REF"][1]), "t_ref": float(parsed["MU_T_REF"][1]),
            "sutherland": float(parsed["SUTHERLAND_CONSTANT"][1])}


def _marker_list(names: Sequence[str]) -> str:
    return "( " + ", ".join(names) + " )"


def layer0_config(
    *,
    ndim: int,
    axisymmetric: bool,
    mach: float,
    markers: Mapping[str, Sequence[str]],
    viscous: bool,
    golden: str,
    analyze: Sequence[str] | None = None,
    mesh: str = "mesh.su2",
    aoa_deg: float = 0.0,
    pressure: float = 101325.0,
    temperature: float = 288.15,
    iterations: int = 3000,
) -> str:
    """SU2 configuration text of one layer 0 case.

    `ndim` (2 or 3) only validates `axisymmetric`; it is not written, because SU2
    takes the dimension from the mesh's NDIME and has no NDIM option.

    `markers` maps a kind to marker names: "wall" (slip wall, also monitored; with `viscous` a
    no-slip wall with the golden's heat flux),
    "far" (far field) and optionally "sym". `golden` is a configuration
    written by the product; its numerics lines are copied verbatim.

    `analyze` names the markers whose mass flux SU2 reports
    (MARKER_ANALYZE with MARKER_ANALYZE_AVERAGE= MASSFLUX; the history file
    then has Avg_Massflow(<marker>) columns). Default: the far-field markers.
    Name the inlet and the outlet separately: the flux through the walls is zero.
    """
    if ndim not in (2, 3):
        raise ValueError(f"ndim must be 2 or 3, got {ndim}")
    if axisymmetric and ndim != 2:
        raise ValueError("an axisymmetric case is two-dimensional")
    unknown = set(markers) - set(_MARKER_KINDS)
    if unknown:
        raise ValueError(f"unknown marker kinds {sorted(unknown)}; use {_MARKER_KINDS}")
    for kind in ("wall", "far"):
        if not markers.get(kind):
            raise ValueError(f"markers needs at least one '{kind}' marker")
    if viscous:
        if not _is_rans(golden):
            raise NotImplementedError(
                "the golden file is an Euler configuration; a viscous layer 0 case needs "
                "the numerics of a configuration the product wrote for RANS")
        return _laminar_config(axisymmetric=axisymmetric, mach=mach, markers=markers, golden=golden,
                               analyze=analyze, mesh=mesh, aoa_deg=aoa_deg, pressure=pressure,
                               temperature=temperature, iterations=iterations)
    if _is_rans(golden):
        raise ValueError("an inviscid layer 0 case needs the Euler golden configuration, this one is RANS")
    numerics = _verbatim(golden, NUMERICS_KEYS)
    physics = _verbatim(golden, PHYSICS_KEYS)
    if analyze is None:
        analyze = list(markers["far"])

    c: list[str] = []
    c.append("% layer 0 case. Numerics: copied from a configuration written by `ignisyeet aero`.")
    c.append("SOLVER= EULER")
    c.append("MATH_PROBLEM= DIRECT")
    c.append("SYSTEM_MEASUREMENTS= SI")
    c.append("RESTART_SOL= NO")
    c.append(f"AXISYMMETRIC= {'YES' if axisymmetric else 'NO'}")
    c.append("")
    c.append(f"MACH_NUMBER= {mach:.6f}")
    c.append(f"AOA= {aoa_deg:.6f}")
    c.append("SIDESLIP_ANGLE= 0.0")
    c.extend(physics)  # the golden's own gas-model lines
    c.append(f"FREESTREAM_PRESSURE= {pressure:.4f}")
    c.append(f"FREESTREAM_TEMPERATURE= {temperature:.4f}")
    c.append("REF_DIMENSIONALIZATION= DIMENSIONAL")
    c.append("")
    c.append("REF_ORIGIN_MOMENT_X= 0.0")
    c.append("REF_ORIGIN_MOMENT_Y= 0.0")
    c.append("REF_ORIGIN_MOMENT_Z= 0.0")
    c.append("REF_LENGTH= 1.0")
    c.append("REF_AREA= 1.0")
    c.append("")
    c.append(f"MARKER_EULER= {_marker_list(markers['wall'])}")
    c.append(f"MARKER_MONITORING= {_marker_list(markers['wall'])}")
    c.append(f"MARKER_FAR= {_marker_list(markers['far'])}")
    if markers.get("sym"):
        c.append(f"MARKER_SYM= {_marker_list(markers['sym'])}")
    c.append(f"MARKER_PLOTTING= {_marker_list(markers['wall'])}")
    history = ["ITER", "RMS_RES", "AERO_COEFF"]
    if analyze:
        c.append(f"MARKER_ANALYZE= {_marker_list(analyze)}")
        c.append("MARKER_ANALYZE_AVERAGE= MASSFLUX")
        history.append("SURFACE_MASSFLOW")
    c.append("")
    c.append("% numerics, verbatim from the golden configuration")
    c.extend(numerics)
    c.append("")
    c.append(f"ITER= {iterations}")
    c.append("")
    c.append(f"MESH_FILENAME= {mesh}")
    c.append("MESH_FORMAT= SU2")
    c.append("RESTART_FILENAME= restart.dat")
    c.append("CONV_FILENAME= history")
    c.append("SURFACE_FILENAME= surface_flow")
    c.append("TABULAR_FORMAT= CSV")
    c.append("OUTPUT_FILES= ( RESTART, SURFACE_CSV )")
    c.append(f"HISTORY_OUTPUT= ( {', '.join(history)} )")
    c.append("SCREEN_OUTPUT= ( INNER_ITER, RMS_DENSITY, LIFT, DRAG )")
    c.append("SCREEN_WRT_FREQ_INNER= 10")
    c.append("HISTORY_WRT_FREQ_INNER= 1")
    c.append("OUTPUT_WRT_FREQ= 50")
    return "\n".join(c) + "\n"


def _laminar_config(
    *,
    axisymmetric: bool,
    mach: float,
    markers: Mapping[str, Sequence[str]],
    golden: str,
    analyze: Sequence[str] | None,
    mesh: str,
    aoa_deg: float,
    pressure: float,
    temperature: float,
    iterations: int,
) -> str:
    """SU2 configuration of a laminar case: SOLVER= NAVIER_STOKES with the RANS golden's numerics and physics.

    RANS_NUMERICS_KEYS and RANS_PHYSICS_KEYS are the golden's own lines. CONV_FIELD is the golden's with the
    turbulence residual removed. The turbulence keys are not written. The wall is MARKER_HEATFLUX with the
    golden's heat flux.
    """
    numerics = _verbatim(golden, RANS_NUMERICS_KEYS)
    physics = _verbatim(golden, RANS_PHYSICS_KEYS)
    _verbatim(golden, RANS_MONITOR_KEYS)  # KeyError if CONV_FIELD is missing
    monitor = f"CONV_FIELD= {_laminar_monitor(_parse(golden)['CONV_FIELD'][1])}"
    flux = _heat_flux_token(golden)
    wall = list(markers["wall"])
    if analyze is None:
        analyze = list(markers["far"])

    c: list[str] = []
    c.append("% layer 0 laminar case: SOLVER= NAVIER_STOKES with the flow numerics and gas model of a RANS configuration")
    c.append("% written by `ignisyeet aero`, copied verbatim; the keys of the turbulence model are left out.")
    c.append("SOLVER= NAVIER_STOKES")
    c.append("MATH_PROBLEM= DIRECT")
    c.append("SYSTEM_MEASUREMENTS= SI")
    c.append("RESTART_SOL= NO")
    c.append(f"AXISYMMETRIC= {'YES' if axisymmetric else 'NO'}")
    c.append("")
    c.append(f"MACH_NUMBER= {mach:.6f}")
    c.append(f"AOA= {aoa_deg:.6f}")
    c.append("SIDESLIP_ANGLE= 0.0")
    c.extend(physics)  # the golden's own gas-model and viscosity lines
    c.append(f"FREESTREAM_PRESSURE= {pressure:.4f}")
    c.append(f"FREESTREAM_TEMPERATURE= {temperature:.4f}")
    c.append("REF_DIMENSIONALIZATION= DIMENSIONAL")
    c.append("")
    c.append("REF_ORIGIN_MOMENT_X= 0.0")
    c.append("REF_ORIGIN_MOMENT_Y= 0.0")
    c.append("REF_ORIGIN_MOMENT_Z= 0.0")
    c.append("REF_LENGTH= 1.0")
    c.append("REF_AREA= 1.0")
    c.append("")
    c.append("MARKER_HEATFLUX= ( " + ", ".join(f"{m}, {flux}" for m in wall) + " )")
    c.append(f"MARKER_MONITORING= {_marker_list(wall)}")
    c.append(f"MARKER_FAR= {_marker_list(markers['far'])}")
    if markers.get("sym"):
        c.append(f"MARKER_SYM= {_marker_list(markers['sym'])}")
    c.append(f"MARKER_PLOTTING= {_marker_list(wall)}")
    history = ["ITER", "RMS_RES", "AERO_COEFF"]
    if analyze:
        c.append(f"MARKER_ANALYZE= {_marker_list(analyze)}")
        c.append("MARKER_ANALYZE_AVERAGE= MASSFLUX")
        history.append("SURFACE_MASSFLOW")
    c.append("")
    c.append("% numerics, verbatim from the RANS golden configuration (CONV_FIELD without the turbulence residual)")
    c.extend(numerics)
    c.append(monitor)
    c.append("")
    c.append(f"ITER= {iterations}")
    c.append("")
    c.append(f"MESH_FILENAME= {mesh}")
    c.append("MESH_FORMAT= SU2")
    c.append("RESTART_FILENAME= restart.dat")
    c.append("CONV_FILENAME= history")
    c.append("SURFACE_FILENAME= surface_flow")
    c.append("TABULAR_FORMAT= CSV")
    c.append("OUTPUT_FILES= ( RESTART, SURFACE_CSV )")
    c.append(f"HISTORY_OUTPUT= ( {', '.join(history)} )")
    c.append("SCREEN_OUTPUT= ( INNER_ITER, RMS_DENSITY, LIFT, DRAG )")
    c.append("VOLUME_OUTPUT= ( COORDINATES, SOLUTION, PRIMITIVE )")
    c.append("SCREEN_WRT_FREQ_INNER= 10")
    c.append("HISTORY_WRT_FREQ_INNER= 1")
    c.append("OUTPUT_WRT_FREQ= 50")
    return "\n".join(c) + "\n"


def rans_stage_config(
    golden: str,
    *,
    model: str,
    case: Mapping[str, str],
    stop: Mapping[str, str] | None = None,
    sst: Mapping[str, str] | None = None,
) -> str:
    """One stage of a turbulent case under ``model`` ("SA" or "SST"), built from a stage the product wrote.

    Every active line of ``golden`` (golden_rans.cfg, the second-order stage, or golden_rans_startup.cfg) is copied
    verbatim, in order, except:

    - the keys in ``case``, which must be RANS_PER_CASE_KEYS present in the golden (KeyError if absent, ValueError if
      the key is a numerics, physics or turbulence key): the case's Mach number, freestream state, reference values,
      markers, iteration budget and file names;
    - the keys in ``stop``, which must be in RANS_STOP_RULE_KEYS: when the run stops;
    - for "SST", the turbulence model: ``KIND_TURB_MODEL= SST``, SA_OPTIONS replaced by SST_OPTIONS and
      FREESTREAM_NU_FACTOR by the two SST freestream keys (``sst`` must give exactly SST_KEYS). The convection scheme,
      time discretisation and CFL reduction of the turbulence equations are the golden's, unchanged.

    For "SA" the model lines are the golden's own and ``sst`` must be empty.
    """
    if model not in ("SA", "SST"):
        raise ValueError(f"model {model!r}: only SA and SST are handled")
    parsed = _parse(golden)
    if parsed.get("SOLVER", ("", ""))[1] != "RANS":
        raise ValueError("the golden file is not a RANS configuration")
    for key in case:
        if key not in parsed:
            raise KeyError(f"{key} is not a key of the golden configuration")
        if key not in RANS_PER_CASE_KEYS:
            raise ValueError(f"{key} is not a per-case key; the numerics, physics and turbulence keys are copied, not overridden")
    stop = dict(stop or {})
    for key in stop:
        if key not in RANS_STOP_RULE_KEYS:
            raise ValueError(f"{key} is not a stop-rule key {RANS_STOP_RULE_KEYS}")
        if key not in parsed:
            raise KeyError(f"{key} is not a key of the golden configuration")
    sst = dict(sst or {})
    if model == "SA" and sst:
        raise ValueError("SA takes no SST key")
    if model == "SST" and set(sst) != set(SST_KEYS):
        raise ValueError(f"SST needs exactly the keys {SST_KEYS}, got {sorted(sst)}")

    out: list[str] = []
    for line in _active_lines(golden):
        if "=" not in line:
            out.append(line)
            continue
        key = line.split("=", 1)[0].strip()
        if key in case:
            out.append(f"{key}= {case[key]}")
        elif key in stop:
            out.append(f"{key}= {stop[key]}")
        elif model == "SST" and key == "KIND_TURB_MODEL":
            out.append("KIND_TURB_MODEL= SST")
        elif model == "SST" and key == "SA_OPTIONS":
            out.append(f"SST_OPTIONS= {sst['SST_OPTIONS']}")
        elif model == "SST" and key == "FREESTREAM_NU_FACTOR":
            out.append(f"FREESTREAM_TURBULENCEINTENSITY= {sst['FREESTREAM_TURBULENCEINTENSITY']}")
            out.append(f"FREESTREAM_TURB2LAMVISCRATIO= {sst['FREESTREAM_TURB2LAMVISCRATIO']}")
        else:
            out.append(line)
    return "\n".join(out) + "\n"
