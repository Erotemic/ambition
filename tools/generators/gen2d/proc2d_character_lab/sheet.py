from __future__ import annotations

from pathlib import Path
from typing import Any, Dict, Optional

import yaml
from PIL import Image, ImageDraw

from .adapters import BaseAdapter
from .config import CharacterJob
from .rendering import load_font, parse_color, rounded_rect


def render_spritesheet(adapter: BaseAdapter, job: CharacterJob, out: str | Path, manifest_out: Optional[str | Path] = None) -> Dict[str, Any]:
    spec = adapter.sample_spec(job)
    anim_defs = adapter.animations()
    animations = job.animations or adapter.default_animations()
    for name in animations:
        if name not in anim_defs:
            raise ValueError(f"unknown animation {name!r} for {adapter.target}; available={sorted(anim_defs)}")

    r = job.render
    fw = r.frame_width
    fh = r.frame_height
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
            "generator": "proc2d-character-lab",
            "target": adapter.target,
            "seed": job.seed,
            "archetype": job.archetype,
            "held_item": job.held_item,
            "frame_width": fw,
            "frame_height": fh,
            "border": border,
            "label_width": label_w,
            "animations": animations,
            "spec": adapter.spec_dict(spec),
        },
        "animations": {},
        "frames": [],
    }

    for row, anim in enumerate(animations):
        row_y = row * row_h
        if label_w:
            label = anim.replace("_", " ").title()
            bbox = draw.textbbox((0, 0), label, font=font)
            tw = bbox[2] - bbox[0]
            th = bbox[3] - bbox[1]
            chip_pad_x = max(8, int(fw * 0.05))
            chip_pad_y = max(4, int(fh * 0.035))
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
            frame = adapter.render_frame(spec, anim, idx, (fw, fh), job)
            x = label_w + idx * (fw + border * 2) + border
            y = row_y + border
            sheet.alpha_composite(frame, (x, y))
            name = f"{anim}_{idx}"
            manifest["frames"].append({"name": name, "animation": anim, "index": idx, "x": x, "y": y, "w": fw, "h": fh, "duration_ms": duration_ms})
            names.append(name)
        manifest["animations"][anim] = {"frames": names, "frame_count": frame_count, "duration_ms": duration_ms, "row": row, "label": anim.replace("_", " ").title()}

    Path(out).parent.mkdir(parents=True, exist_ok=True)
    sheet.save(out)
    if manifest_out is not None:
        Path(manifest_out).parent.mkdir(parents=True, exist_ok=True)
        Path(manifest_out).write_text(yaml.safe_dump(manifest, sort_keys=False), encoding="utf-8")
    return manifest
