"""Robot-side adapter contract tests.

The animation set grew to include `blink_out`, `blink_in`, and `dash`
since the original 8-row layout. Tests reflect the current adapter.
"""
from proc2d_character_lab.adapters import get_adapter


EXPECTED_ROBOT_ANIMS = [
    "idle",
    "walk",
    "run",
    "jump",
    "fall",
    "slash",
    "hit",
    "death",
    "blink_out",
    "blink_in",
    "dash",
]


def test_robot_animation_contract():
    adapter = get_adapter("robot")
    animations = adapter.animations()
    assert list(animations) == EXPECTED_ROBOT_ANIMS


def test_robot_run_pose_is_side_scroller_friendly():
    adapter = get_adapter("robot")
    gen = adapter.generator
    pose = gen.pose_for_animation("run", 2, gen.ANIMATIONS["run"]["frames"])
    # Running leans into the facing direction with a strong stride.
    # `root_tilt` was renamed to `body_tilt` in a later refactor.
    assert pose.body_tilt < 0
    # `left_leg_upper`/`right_leg_upper` were renamed to side-relative
    # names (`near_leg_upper`/`far_leg_upper`) in a later refactor.
    assert pose.near_leg_upper != pose.far_leg_upper
