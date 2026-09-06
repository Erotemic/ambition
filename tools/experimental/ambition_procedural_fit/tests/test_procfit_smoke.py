from __future__ import annotations

from pathlib import Path

import torch
from PIL import Image, ImageDraw

from ambition_procedural_fit.fit import fit_template, render_template
from ambition_procedural_fit.init_template import init_template_from_image
from ambition_procedural_fit.schema import (
    CanvasSpec,
    FitSpec,
    PrimitiveSpec,
    write_spec,
)
from ambition_procedural_fit.soft_render import ProceduralScene, RenderOptions


def test_soft_render_smoke() -> None:
    spec = FitSpec(
        canvas=CanvasSpec(width=32, height=32, background=[0.0, 0.0, 0.0, 1.0]),
        primitives=[
            PrimitiveSpec(
                kind="rect",
                name="box",
                params={
                    "xy": [0.5, 0.5],
                    "wh": [0.4, 0.3],
                    "angle": 0.1,
                    "color": [1.0, 0.8, 0.2, 0.9],
                },
                train=["xy", "wh", "angle", "color"],
            ),
            PrimitiveSpec(
                kind="superellipse",
                name="squircle",
                params={
                    "xy": [0.25, 0.22],
                    "wh": [0.22, 0.18],
                    "angle": 0.0,
                    "exponent": 5.0,
                    "color": [0.8, 0.3, 0.9, 0.6],
                },
                train=["xy", "wh", "angle", "exponent", "color"],
            ),
            PrimitiveSpec(
                kind="segment",
                name="line",
                params={
                    "p0": [0.1, 0.1],
                    "p1": [0.9, 0.8],
                    "width": 0.04,
                    "color": [0.1, 0.7, 1.0, 0.7],
                },
                train=["p0", "p1", "width", "color"],
            ),
        ],
    )
    scene = ProceduralScene(spec)
    img = scene(RenderOptions(width=32, height=32, supersample=1))
    assert tuple(img.shape) == (32, 32, 3)
    assert torch.isfinite(img).all()
    img_det = img.detach()
    assert 0.0 <= float(img_det.min()) <= float(img_det.max()) <= 1.0


def test_init_render_and_tiny_fit(tmp_path: Path) -> None:
    target_path = tmp_path / "target.png"
    img = Image.new("RGB", (64, 64), (16, 18, 24))
    draw = ImageDraw.Draw(img)
    draw.rectangle([18, 20, 46, 48], fill=(150, 120, 80))
    draw.ellipse([22, 8, 42, 28], fill=(220, 190, 90))
    draw.line([8, 52, 56, 10], fill=(210, 230, 240), width=2)
    img.save(target_path)

    template_path = tmp_path / "seed.yaml"
    init_template_from_image(
        target_path,
        template_path,
        size=48,
        rects=8,
        ellipses=2,
        superellipses=2,
        segments=2,
    )
    assert template_path.exists()

    render_path = tmp_path / "render.png"
    render_template(
        template_path, render_path, width=48, height=48, device="cpu", supersample=1
    )
    assert render_path.exists()

    out_dir = tmp_path / "fit"
    result = fit_template(
        template_path,
        target_path,
        out_dir,
        steps=2,
        lr=0.02,
        size=48,
        device="cpu",
        supersample=1,
        restarts=1,
        log_every=0,
    )
    assert result.template_path.exists()
    assert result.render_path.exists()
    assert result.comparison_path.exists()
    assert result.metrics_path.exists()
    assert result.best_loss >= 0
