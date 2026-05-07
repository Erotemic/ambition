from gen3d_blender_lab.adapters import get_adapter
from gen3d_blender_lab.config import CharacterJob


def test_robot_spec_is_side_viewed():
    adapter = get_adapter("robot")
    job = CharacterJob(target="robot", seed=0, archetype="scout_bot")
    spec = adapter.spec_dict(adapter.sample_spec(job))
    assert spec["view"]["camera_y"] < 0
    assert spec["view"]["camera_x"] > 0
    assert spec["view"]["camera_style"] == "side_3q"
