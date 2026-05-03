from proc2d_character_lab.adapters import get_adapter


def test_robot_animation_contract():
    adapter = get_adapter("robot")
    animations = adapter.animations()
    assert list(animations) == ["idle", "walk", "run", "jump", "fall", "slash", "hit", "death"]


def test_robot_run_pose_is_side_scroller_friendly():
    adapter = get_adapter("robot")
    gen = adapter.generator
    pose = gen.pose_for_animation("run", 2, gen.ANIMATIONS["run"]["frames"])
    # Running leans into the facing direction with a strong stride.
    assert pose.root_tilt < 0
    assert pose.left_leg_upper != pose.right_leg_upper
