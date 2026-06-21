import pytest

# Full-resolution render path; opt in with --run-slow-render (see GOALS.md).
pytestmark = pytest.mark.slow_render

from pathlib import Path

from PIL import Image

from ambition_sprite2d_renderer.authoring.canonical import render_canonical
from ambition_sprite2d_renderer.registry import CharacterJob
from ambition_sprite2d_renderer.authoring.sheet import write_spritesheet

CONFIGS = (
    Path(__file__).resolve().parent.parent / "ambition_sprite2d_renderer" / "configs"
)


def test_raid_enforcer_canonical_smoke():
    job = CharacterJob.load(CONFIGS / "raid_enforcer.yaml")
    image = render_canonical(job)
    assert image.size == (job.render.single_width, job.render.single_height)


def test_raid_enforcer_sheet_smoke(tmp_path: Path):
    job = CharacterJob.load(CONFIGS / "raid_enforcer.yaml")
    job.render.frame_width = 64
    job.render.frame_height = 64
    job.render.supersample = 1
    job.animations = ["idle", "slash"]
    image_path, manifest_path = write_spritesheet(
        job, tmp_path / "raid_enforcer.png", tmp_path / "raid_enforcer.yaml"
    )
    assert image_path.exists()
    assert manifest_path.exists()

    image = Image.open(image_path).convert("RGBA")
    pixels = list(image.getdata())
    red_pixels = sum(
        1 for r, g, b, a in pixels if a > 0 and r >= 140 and g <= 90 and b <= 90
    )
    dark_pixels = sum(
        1 for r, g, b, a in pixels if a > 0 and r <= 70 and g <= 75 and b <= 85
    )
    assert red_pixels > 25
    assert dark_pixels > 300
