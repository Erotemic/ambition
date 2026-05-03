from __future__ import annotations

from pathlib import Path
from tempfile import TemporaryDirectory
from typing import Any, Dict, List

from PIL import Image, ImageColor, ImageDraw, ImageFont

from .adapters import BaseAdapter
from .blender_backend.driver import render_requests
from .config import CharacterJob


def _load_font(size: int):
    for name in ("DejaVuSans-Bold.ttf", "DejaVuSans.ttf"):
        try:
            return ImageFont.truetype(name, size)
        except OSError:
            pass
    return ImageFont.load_default()


def render_canonical(adapter: BaseAdapter, job: CharacterJob, out: str | Path) -> Dict[str, Any]:
    animation, frame_index = adapter.canonical_pose()
    width = job.render.single_width
    height = job.render.single_height
    with TemporaryDirectory(prefix="gen3d_canonical_") as d:
        raw = Path(d) / f"{adapter.target}_canonical_raw.png"
        req = [{
            "animation": animation,
            "frame_index": frame_index,
            "frame_count": adapter.animations()[animation]["frames"],
            "width": width,
            "height": height,
            "out_path": str(raw),
        }]
        render_requests(adapter, job, req, mode="canonical")
        img = Image.open(raw).convert("RGBA")
        background = (0, 0, 0, 0) if job.render.background.lower() == "transparent" else ImageColor.getcolor(job.render.background, "RGBA")
        canvas = Image.new("RGBA", (width, height), background)
        canvas.alpha_composite(img, (0, 0))
        out = Path(out)
        out.parent.mkdir(parents=True, exist_ok=True)
        canvas.save(out)
    return {
        "target": adapter.target,
        "animation": animation,
        "frame_index": frame_index,
        "out": str(out),
    }


def render_canonical_contact_sheet(items: List[Dict[str, Any]], out: str | Path, card_width: int = 512, card_height: int = 512) -> None:
    if not items:
        raise ValueError("No canonical items to render")
    cols = min(3, max(1, len(items)))
    rows = (len(items) + cols - 1) // cols
    margin = 24
    label_h = 52
    sheet = Image.new("RGBA", (cols * (card_width + margin) + margin, rows * (card_height + label_h + margin) + margin), (245, 247, 250, 255))
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
        draw.text((x + 4, y + card_height + 10), label, font=font, fill=(22, 24, 32, 255))
    out = Path(out)
    out.parent.mkdir(parents=True, exist_ok=True)
    sheet.save(out)
