# SPDX-License-Identifier: GPL-3.0-or-later
"""A family of grids for one grid convergence study, and its preconditions.

A GCI is only meaningful when the grids are geometrically similar, which means
every cell-size knob scaled by the same factor and the flow domain stayed fixed,
and when every level actually converged. ``check_similar`` refuses a family that
fails either condition, and ``study`` calls it before computing anything.
"""
from __future__ import annotations

import math
from dataclasses import dataclass, field

from .gci import GridConvergence, grid_convergence, representative_h

MIN_RESIDUAL_DROP_ORDERS = 3.0
"""The NASA tutorial asks for three to four orders before calling a run converged."""


@dataclass
class GridLevel:
    """One grid and the evidence that its solution converged.

    There is no default that asserts convergence. Two kinds of evidence exist
    and a level needs at least one:

    - ``solver_converged``: the solver's own acceptance verdict as recorded, for
      runs judged against an absolute target. The caller maps any status string
      to True or False; this module does not parse strings. None means no
      verdict was recorded.
    - ``residual_drop_orders``: orders of magnitude the residual fell. None (or
      NaN) means it was not recorded.
    """

    name: str
    nodes: int
    knobs: dict[str, float] = field(default_factory=dict)
    """Cell-size knobs, as lengths: a coarser grid has larger values."""
    residual_drop_orders: float | None = None
    solver_converged: bool | None = None

    @property
    def basis(self) -> str:
        """What justified this level, derived from the evidence it carries."""
        if self.solver_converged is not None:
            return "solver status"
        if self.residual_drop_orders is not None and not math.isnan(
            self.residual_drop_orders
        ):
            return "residual"
        return "none"

    def check_converged(self) -> None:
        """Raise ValueError unless the recorded evidence shows convergence.

        A solver verdict decides on its own: its acceptance is against an
        absolute target, so the residual-drop floor is not applied on top. Only
        without a verdict does the drop have to reach MIN_RESIDUAL_DROP_ORDERS.
        """
        if self.solver_converged is False:
            raise ValueError(
                f"level {self.name!r} was recorded as not converged by the "
                "solver; it cannot enter a grid convergence study"
            )
        if self.solver_converged:
            return
        drop = self.residual_drop_orders
        if drop is None or math.isnan(drop):
            raise ValueError(
                f"level {self.name!r} carries no convergence evidence: no "
                "solver verdict and no residual drop"
            )
        if drop < MIN_RESIDUAL_DROP_ORDERS:
            raise ValueError(
                f"level {self.name!r} dropped only {drop} orders and has no "
                f"solver verdict, below the {MIN_RESIDUAL_DROP_ORDERS} the "
                "tutorial asks for"
            )


def _odd_knobs(factors: dict[str, float], rtol: float) -> list[str]:
    """The knobs whose scaling factor is out of line with the others.

    The reference is the largest group of knobs that agree within rtol. When no
    group is strictly the largest (two knobs that disagree, or a 2-2 split)
    nothing singles one knob out, so every knob is returned.
    """
    groups = {
        frozenset(k for k, w in factors.items() if abs(w - v) <= rtol * v)
        for v in factors.values()
    }
    size = max(len(g) for g in groups)
    largest = [g for g in groups if len(g) == size]
    if len(largest) != 1:
        return list(factors)
    return [k for k in factors if k not in largest[0]]


@dataclass
class GridFamily:
    levels: list[GridLevel]
    """Finest first."""
    ndim: int = 3
    """Spatial dimension of the grids; a 2-D grid has h = N**(-1/2)."""

    def __post_init__(self) -> None:
        if self.ndim not in (2, 3):
            raise ValueError(f"ndim must be 2 or 3, got {self.ndim}")

    def h(self) -> list[float]:
        if self.ndim == 3:
            return [representative_h(lv.nodes) for lv in self.levels]
        return [float(lv.nodes) ** -0.5 for lv in self.levels]

    def check_similar(self, rtol: float = 0.02) -> None:
        """Raise ValueError unless this is a usable grid family.

        Three or more levels are accepted, so a longer series can be validated
        as a whole before triplets are drawn from it.
        """
        if len(self.levels) < 3:
            raise ValueError("a grid convergence study needs at least three levels")
        nodes = [lv.nodes for lv in self.levels]
        if any(a <= b for a, b in zip(nodes, nodes[1:])):
            raise ValueError(
                "the levels must be ordered finest first, with strictly fewer "
                f"nodes at each coarser level; got node counts {nodes}"
            )
        for lv in self.levels:
            lv.check_converged()
        names = set(self.levels[0].knobs)
        if not names:
            raise ValueError(
                "no cell-size knobs are recorded, so geometric similarity "
                "cannot be checked"
            )
        for lv in self.levels:
            if set(lv.knobs) != names:
                raise ValueError(
                    f"level {lv.name!r} records knobs {sorted(lv.knobs)}, "
                    f"not {sorted(names)}"
                )
            for k, v in lv.knobs.items():
                if not (math.isfinite(v) and v > 0.0):
                    raise ValueError(
                        f"knob {k!r} of level {lv.name!r} is {v}; "
                        "cell sizes must be positive"
                    )
        # Every knob must scale by the same factor between consecutive levels.
        for a, b in zip(self.levels, self.levels[1:]):
            factors = {k: b.knobs[k] / a.knobs[k] for k in a.knobs}
            for k, v in factors.items():
                if v <= 1.0:
                    raise ValueError(
                        f"knob {k!r} went from {a.knobs[k]} to {b.knobs[k]} "
                        f"between {a.name!r} and {b.name!r}; the coarser grid "
                        "must have the larger cell size"
                    )
            odd = _odd_knobs(factors, rtol)
            if odd:
                detail = ", ".join(f"{k!r} x{factors[k]:.4f}" for k in odd)
                if len(odd) == len(factors):
                    raise ValueError(
                        f"the knobs scaled by different factors between "
                        f"{a.name!r} and {b.name!r} ({detail}) and no knob "
                        "agrees with a majority; "
                        "the grids are not geometrically similar"
                    )
                ref = next(v for k, v in factors.items() if k not in odd)
                raise ValueError(
                    f"{detail} between {a.name!r} and {b.name!r} while the "
                    f"others scaled by about {ref:.4f}; "
                    "the grids are not geometrically similar"
                )


def study(
    family: GridFamily, values: dict[str, list[float]], fs: float = 1.25
) -> dict[str, GridConvergence]:
    """One GridConvergence per quantity. Values are finest first, like the levels.

    The family must hold exactly three levels. A longer series is not trimmed
    here, because a silent choice of which three to use would decide the answer;
    the caller builds each triplet explicitly. A result whose order is None is a
    legitimate outcome (the triplet does not converge monotonically), and its
    note says why.
    """
    if len(family.levels) != 3:
        raise ValueError(
            "a grid convergence study takes exactly three levels, got "
            f"{len(family.levels)}; build the triplet explicitly"
        )
    family.check_similar()
    h = family.h()
    out: dict[str, GridConvergence] = {}
    for name, v in values.items():
        if len(v) != 3:
            raise ValueError(f"quantity {name!r} needs exactly three values, got {len(v)}")
        out[name] = grid_convergence(v[0], v[1], v[2], h[0], h[1], h[2], fs=fs)
    return out
