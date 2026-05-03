"""Smoke tests for the basic per-target sheet + canonical render pipelines.

The previous generation of these tests imported a `render_spritesheet`
helper that has since been split into `build_spritesheet` (in-memory
result) and `write_spritesheet` (on-disk result). Updated to the current
API so the suite is green from a clean checkout.
"""
from pathlib import Path

from proc2d_character_lab.canonical import render_canonical
from proc2d_character_lab.config import CharacterJob
from proc2d_character_lab.sheet import write_spritesheet


def test_goblin_sheet_smoke(tmp_path: Path):
    job = CharacterJob.load(Path("proc2d_character_lab/configs/goblin.yaml"))
    job.render.frame_width = 64
    job.render.frame_height = 64
    job.render.supersample = 1
    # Single animation keeps the test under a second.
    job.animations = ["idle"]
    image_path, manifest_path = write_spritesheet(
        job, tmp_path / "goblin.png", tmp_path / "goblin.yaml"
    )
    assert image_path.exists()
    assert manifest_path.exists()


def test_robot_sheet_smoke(tmp_path: Path):
    job = CharacterJob.load(Path("proc2d_character_lab/configs/robot.yaml"))
    job.render.frame_width = 64
    job.render.frame_height = 64
    job.render.supersample = 1
    job.animations = ["idle"]
    image_path, _ = write_spritesheet(job, tmp_path / "robot.png", tmp_path / "robot.yaml")
    assert image_path.exists()


def test_canonical_smoke():
    job = CharacterJob.load(Path("proc2d_character_lab/configs/robot.yaml"))
    image = render_canonical(job)
    assert image.size == (job.render.single_width, job.render.single_height)
