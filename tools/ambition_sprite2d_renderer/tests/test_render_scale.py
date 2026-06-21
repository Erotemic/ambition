"""The fleet sprite-resolution knob (RenderConfig.render_scale).

render_scale multiplies a published sheet's native pixel count without
changing how big the sprite appears in game (display size is collision-driven
and takes only aspect from the frame), fixing upscaled/pixelated sprites. It
must work uniformly across generator families — both the toon "scale to frame
width" generators and the robot/goblin/boss "128-base" generators.
"""

from __future__ import annotations

import tempfile
from pathlib import Path

import yaml

from ambition_sprite2d_renderer.authoring.sheet import write_spritesheet
from ambition_sprite2d_renderer.cli.commands import (
    DEFAULT_CONFIG_DIR,
    DEFAULT_REVIEW_CONFIG_DIR,
)
from ambition_sprite2d_renderer.registry import load_jobs
from ambition_sprite2d_renderer.registry.config import RenderConfig


def test_default_render_scale_is_hi_res():
    """The fleet default renders at 2x so sheets aren't soft when upscaled."""
    assert RenderConfig().render_scale == 2


def _jobs():
    out = {}
    for cdir in (DEFAULT_CONFIG_DIR, DEFAULT_REVIEW_CONFIG_DIR):
        for path, job in load_jobs(Path(cdir)):
            out.setdefault(path.stem, job)
    return out


def _published_frame(job, scale):
    job.render.render_scale = scale
    job.render.supersample = 1  # speed: this test asserts dimensions, not AA
    job.animations = job.animations[:1]  # one row keeps it quick
    d = Path(tempfile.mkdtemp())
    write_spritesheet(job, d / "s_spritesheet.png", d / "s_spritesheet.yaml")
    m = yaml.safe_load((d / "s_spritesheet.yaml").read_text())
    return m["frame_width"], m["frame_height"]


def test_render_scale_doubles_resolution_across_generator_families():
    jobs = _jobs()
    # One representative per generator family: toon (scale-to-width) +
    # robot/goblin/boss (128-base, fixed to scale-proportionally).
    for name in ("absurd_general", "player_robot", "goblin", "boss"):
        job = jobs.get(name)
        if job is None:
            continue
        w1, h1 = _published_frame(job, 1)
        w2, h2 = _published_frame(job, 2)  # same job; helper only flips scale
        ratio = w2 / w1
        assert 1.7 < ratio < 2.3, f"{name}: render_scale=2 should ~2x ({w1}->{w2})"
        # Aspect is preserved up to crop-bbox pixel rounding (looser here only
        # because the test forces supersample=1 for speed; at the real ss=4 the
        # crop edges are smooth and aspect matches tightly).
        assert abs((w1 / h1) - (w2 / h2)) < 0.12, f"{name}: aspect must hold ({w1}x{h1} vs {w2}x{h2})"
