from pathlib import Path

from proc2d_character_lab.adapters import get_adapter
from proc2d_character_lab.canonical import render_canonical
from proc2d_character_lab.config import CharacterJob
from proc2d_character_lab.sheet import render_spritesheet


def test_goblin_sheet_smoke(tmp_path: Path):
    job = CharacterJob(target="goblin", seed=1, archetype="default", held_item="sword", animations=["idle"])
    job.render.frame_width = 64
    job.render.frame_height = 64
    job.render.supersample = 2
    adapter = get_adapter("goblin")
    out = tmp_path / "goblin.png"
    manifest = render_spritesheet(adapter, job, out, tmp_path / "goblin.yaml")
    assert out.exists()
    assert len(manifest["frames"]) == adapter.animations()["idle"]["frames"]


def test_robot_sheet_smoke(tmp_path: Path):
    job = CharacterJob(target="robot", seed=7, archetype="cute_scout", animations=["idle"])
    job.render.frame_width = 64
    job.render.frame_height = 64
    job.render.supersample = 2
    job.render.background = "transparent"
    job.render.sheet_background = "transparent"
    adapter = get_adapter("robot")
    out = tmp_path / "robot.png"
    manifest = render_spritesheet(adapter, job, out, tmp_path / "robot.yaml")
    assert out.exists()
    assert len(manifest["frames"]) == adapter.animations()["idle"]["frames"]


def test_canonical_smoke():
    adapter = get_adapter("robot")
    job = CharacterJob(target="robot", seed=3, archetype="cute_scout")
    image, manifest = render_canonical(adapter, job)
    assert image.size == (job.render.single_width, job.render.single_height)
    assert manifest["target"] == "robot"
