"""Smoke tests for the basic per-target sheet + canonical render pipelines.

These tests deliberately render at minimal config (``supersample=1``,
single animation) so they're already fast — no ``slow_render`` marker
needed. They form the bottom rung of the regression net: prove the
core adapter pipelines (robot/goblin/ninja/canonical) can render end
to end from a clean checkout.

The previous generation of these tests imported a `render_spritesheet`
helper that has since been split into `build_spritesheet` (in-memory
result) and `write_spritesheet` (on-disk result). Updated to the current
API so the suite is green from a clean checkout.
"""

from pathlib import Path

from ambition_sprite2d_renderer.authoring.canonical import render_canonical
from ambition_sprite2d_renderer.registry import CharacterJob
from ambition_sprite2d_renderer.authoring.sheet import write_spritesheet


# Resolve configs relative to the package, not the cwd.
CONFIGS = (
    Path(__file__).resolve().parent.parent / "ambition_sprite2d_renderer" / "configs"
)


def test_goblin_sheet_smoke(tmp_path: Path):
    job = CharacterJob.load(CONFIGS / "goblin.yaml")
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
    job = CharacterJob.load(CONFIGS / "robot.yaml")
    job.render.frame_width = 64
    job.render.frame_height = 64
    job.render.supersample = 1
    job.animations = ["idle"]
    image_path, _ = write_spritesheet(
        job, tmp_path / "robot.png", tmp_path / "robot.yaml"
    )
    assert image_path.exists()


def test_canonical_smoke():
    job = CharacterJob.load(CONFIGS / "robot.yaml")
    image = render_canonical(job)
    assert image.size == (job.render.single_width, job.render.single_height)


def test_ninja_canonical_smoke():
    job = CharacterJob.load(CONFIGS / "ninja.yaml")
    image = render_canonical(job)
    assert image.size == (job.render.single_width, job.render.single_height)


def test_ninja_sheet_smoke(tmp_path: Path):
    job = CharacterJob.load(CONFIGS / "ninja.yaml")
    job.render.frame_width = 64
    job.render.frame_height = 64
    job.render.supersample = 1
    job.animations = ["idle"]
    image_path, manifest_path = write_spritesheet(
        job, tmp_path / "ninja.png", tmp_path / "ninja.yaml"
    )
    assert image_path.exists()
    assert manifest_path.exists()


def test_ninja_leader_canonical_smoke():
    job = CharacterJob.load(CONFIGS / "ninja_leader.yaml")
    image = render_canonical(job)
    assert image.size == (job.render.single_width, job.render.single_height)


def test_ninja_leader_sheet_smoke(tmp_path: Path):
    job = CharacterJob.load(CONFIGS / "ninja_leader.yaml")
    job.render.frame_width = 64
    job.render.frame_height = 64
    job.render.supersample = 1
    job.animations = ["idle"]
    image_path, manifest_path = write_spritesheet(
        job, tmp_path / "ninja_leader.png", tmp_path / "ninja_leader.yaml"
    )
    assert image_path.exists()
    assert manifest_path.exists()
