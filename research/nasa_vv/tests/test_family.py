# SPDX-License-Identifier: GPL-3.0-or-later
import pytest

from nasa_vv.family import GridFamily, GridLevel, study


def _level(name, nodes, wall, fin, drop=6.0, solver=None):
    return GridLevel(name=name, nodes=nodes,
                     knobs={"wall_size": wall, "fin_size": fin},
                     residual_drop_orders=drop, solver_converged=solver)


def _similar():
    return GridFamily([_level("fine", 1_840_000, 3.0, 1.0),
                       _level("medium", 787_000, 4.8, 1.6),
                       _level("coarse", 348_000, 8.0, 2.667)])


def _knobs3(name, nodes, a, b, c):
    return GridLevel(name=name, nodes=nodes,
                     knobs={"wall_size": a, "fin_size": b, "far_size": c},
                     residual_drop_orders=6.0)


def test_a_similar_family_passes_the_check():
    _similar().check_similar()


def test_a_family_that_held_one_knob_fixed_is_refused():
    """The existing six-level Euler series held fin_size at 1 mm."""
    f = GridFamily([_level("fine", 1_840_000, 1.8, 1.0),
                    _level("medium", 787_000, 3.0, 1.0),
                    _level("coarse", 348_000, 6.0, 1.0)])
    with pytest.raises(ValueError, match="fin_size"):
        f.check_similar()


def test_the_message_names_the_odd_knob_not_the_agreeing_ones():
    f = GridFamily([_knobs3("fine", 1_840_000, 3.0, 1.0, 20.0),
                    _knobs3("medium", 787_000, 4.8, 1.6, 32.0),
                    _knobs3("coarse", 348_000, 8.0, 2.0, 53.3)])
    with pytest.raises(ValueError) as e:
        f.check_similar()
    msg = str(e.value)
    assert "fin_size" in msg
    assert "wall_size" not in msg and "far_size" not in msg


def test_the_odd_knob_is_named_whichever_knob_comes_first():
    f = GridFamily([_knobs3("fine", 1_840_000, 1.0, 3.0, 20.0),
                    _knobs3("medium", 787_000, 1.0, 4.8, 32.0),
                    _knobs3("coarse", 348_000, 1.0, 8.0, 53.3)])
    with pytest.raises(ValueError) as e:
        f.check_similar()
    msg = str(e.value)
    assert "wall_size" in msg
    assert "fin_size" not in msg and "far_size" not in msg


def _with(level):
    """A similar family whose coarsest level is replaced by the given one."""
    lv = _similar().levels[:2] + [level]
    return GridFamily(lv)


def test_a_level_the_solver_recorded_as_unconverged_is_refused():
    f = GridFamily([_level("fine", 1_840_000, 3.0, 1.0, drop=0.6, solver=False),
                    _level("medium", 787_000, 4.8, 1.6),
                    _level("coarse", 348_000, 8.0, 2.667)])
    with pytest.raises(ValueError, match=r"'fine'.*not converged by the solver"):
        f.check_similar()


def test_a_solver_verdict_of_false_wins_over_a_large_drop():
    f = _with(_level("coarse", 348_000, 8.0, 2.667, drop=9.0, solver=False))
    with pytest.raises(ValueError, match=r"'coarse'.*not converged by the solver"):
        f.check_similar()


def test_a_level_with_no_evidence_at_all_is_refused():
    f = _with(_level("coarse", 348_000, 8.0, 2.667, drop=None, solver=None))
    with pytest.raises(ValueError, match=r"'coarse'.*no convergence evidence"):
        f.check_similar()


def test_a_nan_drop_counts_as_no_evidence():
    f = _with(_level("coarse", 348_000, 8.0, 2.667, drop=float("nan")))
    with pytest.raises(ValueError, match=r"'coarse'.*no convergence evidence"):
        f.check_similar()


def test_a_solver_verdict_alone_is_enough():
    _with(_level("coarse", 348_000, 8.0, 2.667, drop=None, solver=True)).check_similar()


def test_a_nan_drop_does_not_hurt_a_level_the_solver_accepted():
    _with(_level("coarse", 348_000, 8.0, 2.667,
                 drop=float("nan"), solver=True)).check_similar()


def test_the_drop_floor_is_not_applied_on_top_of_a_solver_verdict():
    """IgnisYeet accepts RANS against an absolute target, taking 2 orders on nu."""
    _with(_level("coarse", 348_000, 8.0, 2.667, drop=2.0, solver=True)).check_similar()


def test_without_a_verdict_a_small_drop_is_refused():
    f = _with(_level("medium2", 348_000, 8.0, 2.667, drop=1.3))
    with pytest.raises(ValueError, match=r"'medium2'.*dropped only 1.3.*no solver verdict"):
        f.check_similar()


def test_without_a_verdict_a_drop_of_exactly_three_passes():
    _with(_level("coarse", 348_000, 8.0, 2.667, drop=3.0)).check_similar()


def test_the_basis_reports_which_kind_of_evidence_justified_the_level():
    assert _level("a", 1, 1.0, 1.0, drop=6.0).basis == "residual"
    assert _level("a", 1, 1.0, 1.0, drop=None, solver=True).basis == "solver status"
    assert _level("a", 1, 1.0, 1.0, drop=6.0, solver=True).basis == "solver status"
    assert _level("a", 1, 1.0, 1.0, drop=None).basis == "none"
    assert _level("a", 1, 1.0, 1.0, drop=float("nan")).basis == "none"


def test_no_default_asserts_convergence():
    with pytest.raises(ValueError, match="no convergence evidence"):
        GridLevel("x", 1000, {"s": 1.0}).check_converged()


def test_levels_must_be_ordered_finest_first():
    f = GridFamily([_level("coarse", 348_000, 8.0, 2.667),
                    _level("medium", 787_000, 4.8, 1.6),
                    _level("fine", 1_840_000, 3.0, 1.0)])
    with pytest.raises(ValueError, match="finest first"):
        f.check_similar()


def test_two_levels_are_not_enough():
    f = GridFamily(_similar().levels[:2])
    with pytest.raises(ValueError, match="three"):
        f.check_similar()


def test_a_four_level_family_can_be_checked_as_a_whole():
    f = GridFamily([_level("L1", 1_840_000, 3.0, 1.0),
                    _level("L2", 787_000, 4.8, 1.6),
                    _level("L3", 348_000, 8.0, 2.667),
                    _level("L4", 150_000, 12.8, 4.267)])
    f.check_similar()


def test_a_knob_set_that_differs_between_levels_is_refused():
    lv = _similar().levels
    lv[1] = GridLevel("medium", 787_000, {"wall_size": 4.8},
                      residual_drop_orders=6.0)
    with pytest.raises(ValueError, match="medium"):
        GridFamily(lv).check_similar()


def test_a_family_with_no_recorded_knobs_is_refused():
    lv = [GridLevel(n, k, {}, 6.0)
          for n, k in [("a", 3_000_000), ("b", 2_000_000), ("c", 1_000_000)]]
    with pytest.raises(ValueError, match="knob"):
        GridFamily(lv).check_similar()


def test_knobs_that_do_not_grow_toward_the_coarse_grid_are_refused():
    f = GridFamily([_level("fine", 1_840_000, 8.0, 2.667),
                    _level("medium", 787_000, 4.8, 1.6),
                    _level("coarse", 348_000, 3.0, 1.0)])
    with pytest.raises(ValueError, match="coarser"):
        f.check_similar()


def test_study_returns_one_result_per_quantity():
    out = study(_similar(), {"cn": [1.2021, 1.2055, 1.2140],
                             "ca": [0.5990, 0.6012, 0.6080]})
    assert set(out) == {"cn", "ca"}
    assert out["cn"].monotone
    assert out["cn"].order is not None
    assert out["cn"].r21 > 1.0


def test_study_hands_back_a_non_converging_triplet_with_its_note():
    out = study(_similar(), {"cn": [1.2021, 1.2140, 1.2150]})
    assert out["cn"].order is None
    assert out["cn"].gci_fine is None
    assert out["cn"].note


def test_study_refuses_a_family_that_fails_the_check():
    f = GridFamily([_level("fine", 1_840_000, 1.8, 1.0),
                    _level("medium", 787_000, 3.0, 1.0),
                    _level("coarse", 348_000, 6.0, 1.0)])
    with pytest.raises(ValueError, match="fin_size"):
        study(f, {"cn": [1.2021, 1.2055, 1.2140]})


def test_study_refuses_a_four_level_family_rather_than_dropping_a_level():
    f = GridFamily([_level("L1", 1_840_000, 3.0, 1.0),
                    _level("L2", 787_000, 4.8, 1.6),
                    _level("L3", 348_000, 8.0, 2.667),
                    _level("L4", 150_000, 12.8, 4.267)])
    with pytest.raises(ValueError, match="exactly three levels"):
        study(f, {"cn": [1.2021, 1.2055, 1.2140]})


def test_study_refuses_a_value_list_of_the_wrong_length():
    with pytest.raises(ValueError, match="cn"):
        study(_similar(), {"cn": [1.2021, 1.2055]})
    with pytest.raises(ValueError, match="cn"):
        study(_similar(), {"cn": [1.2021, 1.2055, 1.2140, 1.23]})


def _flatplate(ndim=2):
    """Node counts of the three flat-plate grids: 273x193, 137x97, 69x49."""
    return GridFamily([GridLevel("fine", 273 * 193, {"h": 1.0}, 6.0),
                       GridLevel("medium", 137 * 97, {"h": 2.0}, 6.0),
                       GridLevel("coarse", 69 * 49, {"h": 4.0}, 6.0)], ndim=ndim)


def test_a_2d_family_has_a_refinement_ratio_of_two():
    f = _flatplate()
    f.check_similar()
    h = f.h()
    assert h[1] / h[0] == pytest.approx(2.0, rel=0.05)
    assert h[2] / h[1] == pytest.approx(2.0, rel=0.05)


def test_the_same_nodes_read_as_3d_give_the_wrong_ratio():
    h = _flatplate(ndim=3).h()
    assert h[1] / h[0] == pytest.approx(1.58, abs=0.01)


def test_ndim_must_be_two_or_three():
    with pytest.raises(ValueError, match="ndim"):
        GridFamily(_similar().levels, ndim=1)
