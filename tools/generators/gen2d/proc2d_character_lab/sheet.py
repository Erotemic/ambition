from __future__ import annotations

from pathlib import Path
from typing import Any, Dict, Iterable, List, Tuple

import yaml
from PIL import Image, ImageColor, ImageDraw, ImageFont

from .adapters import get_adapter
from .config import CharacterJob


def _parse_bg(value: str):
    if str(value).lower() == "transparent":
        return (0, 0, 0, 0)
    r, g, b = ImageColor.getrgb(value)
    return (r, g, b, 255)


def _font(size: int = 12):
    for name in ("DejaVuSans-Bold.ttf", "DejaVuSans.ttf"):
        try:
            return ImageFont.truetype(name, size=size)
        except OSError:
            pass
    return ImageFont.load_default()


def build_spritesheet(job: CharacterJob) -> Tuple[Image.Image, Dict[str, Any]]:
    adapter = get_adapter(job.target)
    spec = adapter.sample_spec(job)
    animations = adapter.animations()
    selected = [a for a in job.animations if a in animations]
    missing = [a for a in job.animations if a not in animations]
    if missing:
        raise KeyError(f"unsupported animations for {job.target}: {missing}; available={sorted(animations)}")
    fw, fh = job.render.frame_width, job.render.frame_height
    label_w = max(0, job.render.label_width)
    border = max(0, job.render.border)
    max_frames = max(animations[a]["frames"] for a in selected)
    sheet_w = label_w + max_frames * (fw + border) + border
    sheet_h = len(selected) * (fh + border) + border
    sheet = Image.new("RGBA", (sheet_w, sheet_h), _parse_bg(job.render.sheet_background))
    draw = ImageDraw.Draw(sheet)
    font = _font(12)
    manifest: Dict[str, Any] = {
        "target": job.target,
        "seed": job.seed,
        "archetype": job.archetype,
        "held_item": job.held_item,
        "frame_width": fw,
        "frame_height": fh,
        "label_width": label_w,
        "border": border,
        "spec": adapter.spec_dict(spec),
        "animations": {},
    }
    for row, animation in enumerate(selected):
        info = animations[animation]
        y = border + row * (fh + border)
        if label_w:
            draw.text((8, y + 8), animation, fill=(255, 255, 255, 255), font=font)
            draw.text((8, y + 23), f"{info['frames']}f/{info['duration_ms']}ms", fill=(190, 190, 190, 255), font=_font(10))
        frames: List[Dict[str, Any]] = []
        for frame_index in range(info["frames"]):
            x = label_w + border + frame_index * (fw + border)
            frame = adapter.render_frame(spec, animation, frame_index, (fw, fh), job)
            sheet.alpha_composite(frame, (x, y))
            frames.append({
                "index": frame_index,
                "x": x,
                "y": y,
                "w": fw,
                "h": fh,
                "duration_ms": info["duration_ms"],
            })
        manifest["animations"][animation] = {"frames": frames, "duration_ms": info["duration_ms"]}
    return sheet, manifest


def write_spritesheet(job: CharacterJob, image_out: str | Path, manifest_out: str | Path | None = None) -> Tuple[Path, Path]:
    image_out = Path(image_out)
    if manifest_out is None:
        manifest_out = image_out.with_suffix(".yaml")
    manifest_out = Path(manifest_out)
    image_out.parent.mkdir(parents=True, exist_ok=True)
    manifest_out.parent.mkdir(parents=True, exist_ok=True)
    sheet, manifest = build_spritesheet(job)
    sheet.save(image_out)
    with open(manifest_out, "w", encoding="utf8") as file:
        yaml.safe_dump(manifest, file, sort_keys=False)
    return image_out, manifest_out
