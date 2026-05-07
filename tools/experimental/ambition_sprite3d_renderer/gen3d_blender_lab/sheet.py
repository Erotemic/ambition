from __future__ import annotations

from pathlib import Path
from tempfile import TemporaryDirectory
from typing import Any, Dict, List, Tuple

import yaml
from PIL import Image, ImageColor, ImageDraw, ImageFont

from .adapters import BaseAdapter
from .blender_backend.driver import render_requests
from .config import CharacterJob
from . import __version__


def parse_color(value: str):
    return (0, 0, 0, 0) if value.lower() == "transparent" else ImageColor.getcolor(value, "RGBA")


def load_font(size: int):
    for name in ("DejaVuSans-Bold.ttf", "DejaVuSans.ttf"):
        try:
            return ImageFont.truetype(name, size)
        except OSError:
            pass
    return ImageFont.load_default()


def rounded_rect(draw: ImageDraw.ImageDraw, box, radius: int, fill, outline, width: int = 1):
    try:
        draw.rounded_rectangle(box, radius=radius, fill=fill, outline=outline, width=width)
    except AttributeError:
        draw.rectangle(box, fill=fill, outline=outline, width=width)



def stamp_version(img: Image.Image, text: str | None = None) -> None:
    text = text or f"v{__version__}"
    draw = ImageDraw.Draw(img)
    font = load_font(16)
    bbox = draw.textbbox((0, 0), text, font=font)
    tw = bbox[2] - bbox[0]
    th = bbox[3] - bbox[1]
    pad_x = 8
    pad_y = 5
    x1 = img.width - 12
    y0 = 10
    x0 = x1 - tw - pad_x * 2
    y1 = y0 + th + pad_y * 2
    rounded_rect(draw, (x0, y0, x1, y1), radius=8, fill=(232, 235, 241, 230), outline=(70, 76, 88, 220), width=1)
    draw.text((x0 + pad_x, y0 + pad_y - 1), text, font=font, fill=(20, 22, 28, 255))

def render_spritesheet(adapter: BaseAdapter, job: CharacterJob, out: str | Path, manifest_out: str | Path | None = None) -> Dict[str, Any]:
    animations = job.animations or adapter.default_animations()
    anim_defs = adapter.animations()
    r = job.render
    fw, fh = r.frame_width, r.frame_height
    border = max(0, r.border)
    label_w = max(0, r.label_width)
    cols = max(anim_defs[name]["frames"] for name in animations)
    row_h = fh + border * 2
    sheet_w = label_w + cols * (fw + border * 2)
    sheet_h = len(animations) * row_h
    sheet = Image.new("RGBA", (sheet_w, sheet_h), parse_color(r.sheet_background))
    draw = ImageDraw.Draw(sheet)
    font = load_font(max(12, int(fh * 0.16)))

    manifest: Dict[str, Any] = {
        "meta": {
            "generator": "ambition-gen3d-blender-lab",
            "backend": "blender",
            "target": adapter.target,
            "seed": job.seed,
            "archetype": job.archetype,
            "held_item": job.held_item,
            "frame_width": fw,
            "frame_height": fh,
            "border": border,
            "label_width": label_w,
            "animations": animations,
            "spec": adapter.spec_dict(adapter.sample_spec(job)),
            "version": __version__,
        },
        "animations": {},
        "frames": [],
    }

    with TemporaryDirectory(prefix="gen3d_blender_lab_frames_") as d:
        dpath = Path(d)
        requests: List[Dict[str, Any]] = []
        temp_paths: Dict[Tuple[str, int], Path] = {}
        for anim in animations:
            frame_count = anim_defs[anim]["frames"]
            for idx in range(frame_count):
                temp_path = dpath / f"{anim}_{idx:03d}.png"
                temp_paths[(anim, idx)] = temp_path
                requests.append({
                    "animation": anim,
                    "frame_index": idx,
                    "frame_count": frame_count,
                    "width": fw,
                    "height": fh,
                    "out_path": str(temp_path),
                })
        render_requests(adapter, job, requests, mode="spritesheet")

        for row, anim in enumerate(animations):
            row_y = row * row_h
            if label_w:
                label = anim.replace("_", " ").upper()
                bbox = draw.textbbox((0, 0), label, font=font)
                tw = bbox[2] - bbox[0]
                th = bbox[3] - bbox[1]
                chip_pad_x = max(8, int(fw * 0.05))
                chip_pad_y = max(4, int(fh * 0.03))
                chip_w = min(label_w - 12, tw + chip_pad_x * 2)
                chip_h = th + chip_pad_y * 2
                chip_x0 = max(6, int((label_w - chip_w) / 2))
                chip_y0 = int(row_y + (row_h - chip_h) / 2)
                rounded_rect(draw, (chip_x0, chip_y0, chip_x0 + chip_w, chip_y0 + chip_h), radius=max(6, int(chip_h * 0.25)), fill=(232, 235, 241, 220), outline=(70, 76, 88, 230), width=1)
                draw.text((chip_x0 + (chip_w - tw) / 2 - bbox[0], chip_y0 + (chip_h - th) / 2 - bbox[1]), label, font=font, fill=(20, 22, 28, 255))
                draw.line([(label_w - 1, row_y + 6), (label_w - 1, row_y + row_h - 6)], fill=(128, 132, 144, 210), width=max(1, int(fh * 0.01)))
            frame_count = anim_defs[anim]["frames"]
            duration_ms = anim_defs[anim]["duration_ms"]
            names = []
            for idx in range(frame_count):
                frame_path = temp_paths[(anim, idx)]
                frame = Image.open(frame_path).convert("RGBA")
                x = label_w + idx * (fw + border * 2) + border
                y = row_y + border
                sheet.alpha_composite(frame, (x, y))
                name = f"{anim}_{idx}"
                manifest["frames"].append({"name": name, "animation": anim, "index": idx, "x": x, "y": y, "w": fw, "h": fh, "duration_ms": duration_ms})
                names.append(name)
            manifest["animations"][anim] = {"frames": names, "frame_count": frame_count, "duration_ms": duration_ms, "row": row, "label": anim.replace("_", " ").title()}

    stamp_version(sheet)
    out = Path(out)
    out.parent.mkdir(parents=True, exist_ok=True)
    sheet.save(out)
    if manifest_out is not None:
        manifest_out = Path(manifest_out)
        manifest_out.parent.mkdir(parents=True, exist_ok=True)
        manifest_out.write_text(yaml.safe_dump(manifest, sort_keys=False), encoding="utf-8")
    return manifest
