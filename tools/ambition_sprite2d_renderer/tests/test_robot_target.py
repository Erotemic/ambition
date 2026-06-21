"""Robot-side adapter contract tests.

The animation set grew to include core runtime rows plus richer review-only
player mechanics. Tests reflect the current adapter vocabulary rather than the
runtime row subset.
"""

from ambition_sprite2d_renderer.authoring.adapters import get_adapter
from ambition_sprite2d_renderer.authoring.animation_vocab import FULL_PLAYER_ANIMATION_ORDER


EXPECTED_ROBOT_ANIMS = list(FULL_PLAYER_ANIMATION_ORDER)


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
