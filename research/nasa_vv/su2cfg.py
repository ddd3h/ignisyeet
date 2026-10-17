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

# marker kinds accepted by layer0_config
_MARKER_KINDS = ("wall", "far", "sym")


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


def _numerics_block(golden: str) -> list[str]:
    product_numerics(golden)  # KeyError when a key is missing
    parsed = _parse(golden)
    return [parsed[k][0] for k in NUMERICS_KEYS]


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

    `markers` maps a kind to marker names: "wall" (slip wall, also monitored),
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
        raise NotImplementedError(
            "the golden file is an Euler configuration; a viscous layer 0 case needs "
            "the numerics of a configuration the product wrote for RANS")
    numerics = _numerics_block(golden)
    if analyze is None:
        analyze = list(markers["far"])

    c: list[str] = []
    c.append("% layer 0 case. Numerics: copied from a configuration written by `ignisyeet aero`.")
    c.append(f"% NDIM= {ndim} (SU2 takes the dimension from the mesh's NDIME; NDIM is not an SU2 option)")
    c.append("SOLVER= EULER")
    c.append("MATH_PROBLEM= DIRECT")
    c.append("SYSTEM_MEASUREMENTS= SI")
    c.append("RESTART_SOL= NO")
    c.append(f"AXISYMMETRIC= {'YES' if axisymmetric else 'NO'}")
    c.append("")
    c.append(f"MACH_NUMBER= {mach:.6f}")
    c.append(f"AOA= {aoa_deg:.6f}")
    c.append("SIDESLIP_ANGLE= 0.0")
    c.append("FLUID_MODEL= IDEAL_GAS")
    c.append("GAMMA_VALUE= 1.4")
    c.append("GAS_CONSTANT= 287.05287")
    c.append("INIT_OPTION= TD_CONDITIONS")
    c.append("FREESTREAM_OPTION= TEMPERATURE_FS")
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
