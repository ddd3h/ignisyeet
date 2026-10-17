# SPDX-License-Identifier: GPL-3.0-or-later
import pytest

from nasa_vv.family import GridFamily, GridLevel, study


def _level(name, nodes, wall, fin, drop=6.0, ok=True):
    return GridLevel(name=name, nodes=nodes,
                     knobs={"wall_size": wall, "fin_size": fin},
                     residual_drop_orders=drop, converged=ok)


def _similar():
    return GridFamily([_level("fine", 1_840_000, 3.0, 1.0),
                       _level("medium", 787_000, 4.8, 1.6),
                       _level("coarse", 348_000, 8.0, 2.667)])


def _knobs3(name, nodes, a, b, c):
    return GridLevel(name=name, nodes=nodes,
                     knobs={"wall_size": a, "fin_size": b, "far_size": c},
                     residual_drop_orders=6.0, converged=True)


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


def test_an_unconverged_level_is_refused():
    f = GridFamily([_level("fine", 1_840_000, 3.0, 1.0, drop=0.6, ok=False),
                    _level("medium", 787_000, 4.8, 1.6),
                    _level("coarse", 348_000, 8.0, 2.667)])
    with pytest.raises(ValueError, match="fine"):
        f.check_similar()


def test_a_level_flagged_converged_but_with_a_small_drop_is_refused():
    f = GridFamily([_level("fine", 1_840_000, 3.0, 1.0),
                    _level("medium", 787_000, 4.8, 1.6, drop=1.3),
                    _level("coarse", 348_000, 8.0, 2.667)])
    with pytest.raises(ValueError, match="medium"):
        f.check_similar()


def test_an_unknown_residual_drop_is_refused():
    f = GridFamily([_level("fine", 1_840_000, 3.0, 1.0),
                    _level("medium", 787_000, 4.8, 1.6),
                    _level("coarse", 348_000, 8.0, 2.667, drop=None)])
    with pytest.raises(ValueError, match="coarse.*unknown|unknown.*coarse"):
        f.check_similar()


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
                      residual_drop_orders=6.0, converged=True)
    with pytest.raises(ValueError, match="medium"):
        GridFamily(lv).check_similar()


def test_a_family_with_no_recorded_knobs_is_refused():
    lv = [GridLevel(n, k, {}, 6.0, True)
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
