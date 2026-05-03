from __future__ import annotations

from math import ceil
from pathlib import Path
from typing import Iterable, List, Sequence, Tuple

from PIL import Image, ImageDraw

from .adapters import BaseAdapter, get_adapter
from .config import CharacterJob, load_job


def render_canonical(adapter: BaseAdapter, job: CharacterJob) -> Tuple[Image.Image, dict]:
    spec = adapter.sample_spec(job)
    image = adapter.render_canonical(spec, job)
    manifest = {
        "name": f"{job.target}_{job.seed}",
        "target": job.target,
        "seed": job.seed,
        "archetype": job.archetype,
        "held_item": job.held_item,
        "canonical_animation": adapter.canonical_pose()[0],
        "canonical_frame": adapter.canonical_pose()[1],
        "spec": adapter.spec_dict(spec),
    }
    return image, manifest


def render_from_config(config_path: Path) -> Tuple[Image.Image, dict]:
    job = load_job(config_path)
    adapter = get_adapter(job.target)
    return render_canonical(adapter, job)


def write_canonical(adapter: BaseAdapter, job: CharacterJob, out_path: Path) -> Tuple[Path, dict]:
    out_path.parent.mkdir(parents=True, exist_ok=True)
    image, manifest = render_canonical(adapter, job)
    image.save(out_path)
    return out_path, manifest


def render_canonical_contact_sheet(entries: Sequence[Tuple[str, Image.Image]], cell_padding: int = 12, columns: int = 4) -> Image.Image:
    if not entries:
        raise ValueError("entries must not be empty")
    thumb_w = max(img.width for _, img in entries)
    thumb_h = max(img.height for _, img in entries)
    label_h = 20
    columns = max(1, columns)
    rows = ceil(len(entries) / columns)
    cell_w = thumb_w + cell_padding * 2
    cell_h = thumb_h + label_h + cell_padding * 2
    sheet = Image.new("RGBA", (columns * cell_w, rows * cell_h), (18, 18, 22, 255))
    draw = ImageDraw.Draw(sheet)
    for idx, (label, img) in enumerate(entries):
        row = idx // columns
        col = idx % columns
        x0 = col * cell_w
        y0 = row * cell_h
        frame = (x0 + 4, y0 + 4, x0 + cell_w - 4, y0 + cell_h - 4)
        draw.rounded_rectangle(frame, radius=12, fill=(28, 29, 35, 255), outline=(70, 74, 86, 255), width=1)
        paste_x = x0 + (cell_w - img.width) // 2
        paste_y = y0 + cell_padding
        sheet.alpha_composite(img, (paste_x, paste_y))
        draw.text((x0 + cell_padding, y0 + thumb_h + cell_padding + 2), label, fill=(230, 230, 235, 255))
    return sheet
