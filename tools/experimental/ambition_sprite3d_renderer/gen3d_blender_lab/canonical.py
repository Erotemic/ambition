from __future__ import annotations

from pathlib import Path
from tempfile import TemporaryDirectory
from typing import Any, Dict, List

from PIL import Image, ImageColor, ImageDraw, ImageFont

from .adapters import BaseAdapter
from .blender_backend.driver import render_requests
from .config import CharacterJob
from . import __version__


def _load_font(size: int):
    for name in ("DejaVuSans-Bold.ttf", "DejaVuSans.ttf"):
        try:
            return ImageFont.truetype(name, size)
        except OSError:
            pass
    return ImageFont.load_default()


def _stamp_version(img: Image.Image, text: str | None = None) -> None:
    text = text or f"v{__version__}"
    draw = ImageDraw.Draw(img)
    font = _load_font(16)
    bbox = draw.textbbox((0, 0), text, font=font)
    tw = bbox[2] - bbox[0]
    th = bbox[3] - bbox[1]
    pad_x = 8
    pad_y = 5
    x1 = img.width - 12
    y0 = 10
    x0 = x1 - tw - pad_x * 2
    y1 = y0 + th + pad_y * 2
    try:
        draw.rounded_rectangle(
            (x0, y0, x1, y1),
            radius=8,
            fill=(250, 252, 255, 220),
            outline=(70, 76, 88, 220),
            width=1,
        )
    except AttributeError:
        draw.rectangle(
            (x0, y0, x1, y1),
            fill=(250, 252, 255, 220),
            outline=(70, 76, 88, 220),
            width=1,
        )
    draw.text((x0 + pad_x, y0 + pad_y - 1), text, font=font, fill=(30, 34, 42, 255))


def render_canonical(
    adapter: BaseAdapter, job: CharacterJob, out: str | Path
) -> Dict[str, Any]:
    animation, frame_index = adapter.canonical_pose()
    width = job.render.single_width
    height = job.render.single_height
    with TemporaryDirectory(prefix="gen3d_canonical_") as d:
        construction_raw = Path(d) / f"{adapter.target}_construction_raw.png"
        side_raw = Path(d) / f"{adapter.target}_side_raw.png"
        req = [
            {
                "animation": "idle",
                "frame_index": 0,
                "frame_count": adapter.animations()["idle"]["frames"],
                "width": width,
                "height": height,
                "out_path": str(construction_raw),
                "render_variant": "construction",
            },
            {
                "animation": animation,
                "frame_index": frame_index,
                "frame_count": adapter.animations()[animation]["frames"],
                "width": width,
                "height": height,
                "out_path": str(side_raw),
                "render_variant": "side_pose",
            },
        ]
        render_requests(adapter, job, req, mode="canonical")
        construction_img = Image.open(construction_raw).convert("RGBA")
        side_img = Image.open(side_raw).convert("RGBA")
        background = (
            (0, 0, 0, 0)
            if job.render.background.lower() == "transparent"
            else ImageColor.getcolor(job.render.background, "RGBA")
        )
        label_h = 42
        gutter = 18
        canvas = Image.new("RGBA", (width * 2 + gutter, height + label_h), background)
        canvas.alpha_composite(construction_img, (0, label_h))
        canvas.alpha_composite(side_img, (width + gutter, label_h))
        draw = ImageDraw.Draw(canvas)
        font = _load_font(22)
        fill = (28, 30, 38, 255)
        draw.text((16, 10), "Construction View", font=font, fill=fill)
        draw.text((width + gutter + 16, 10), "Side Pose", font=font, fill=fill)
        _stamp_version(canvas)
        out = Path(out)
        out.parent.mkdir(parents=True, exist_ok=True)
        canvas.save(out)
    return {
        "target": adapter.target,
        "animation": animation,
        "frame_index": frame_index,
        "out": str(out),
        "width": width * 2 + gutter,
        "height": height + label_h,
    }


def render_canonical_contact_sheet(
    items: List[Dict[str, Any]],
    out: str | Path,
    card_width: int | None = None,
    card_height: int | None = None,
) -> None:
    if not items:
        raise ValueError("No canonical items to render")
    if card_width is None or card_height is None:
        sample = Image.open(items[0]["out"]).convert("RGBA")
        inferred_w, inferred_h = sample.size
        card_width = card_width or inferred_w
        card_height = card_height or inferred_h
    cols = min(2, max(1, len(items)))
    rows = (len(items) + cols - 1) // cols
    margin = 24
    label_h = 52
    sheet = Image.new(
        "RGBA",
        (
            cols * (card_width + margin) + margin,
            rows * (card_height + label_h + margin) + margin,
        ),
        (245, 247, 250, 255),
    )
    font = _load_font(28)
    draw = ImageDraw.Draw(sheet)
    for idx, item in enumerate(items):
        row = idx // cols
        col = idx % cols
        x = margin + col * (card_width + margin)
        y = margin + row * (card_height + label_h + margin)
        img = Image.open(item["out"]).convert("RGBA")
        sheet.alpha_composite(img, (x, y))
        label = item["target"].replace("_", " ").title()
        draw.text(
            (x + 4, y + card_height + 10), label, font=font, fill=(22, 24, 32, 255)
        )
    _stamp_version(sheet)
    out = Path(out)
    out.parent.mkdir(parents=True, exist_ok=True)
    sheet.save(out)
