from __future__ import annotations

from pathlib import Path
from typing import List, Tuple

from PIL import Image, ImageDraw, ImageFont

from .adapters import get_adapter
from .config import CharacterJob, load_jobs


def _font(size: int = 14):
    for name in ("DejaVuSans-Bold.ttf", "DejaVuSans.ttf"):
        try:
            return ImageFont.truetype(name, size=size)
        except OSError:
            pass
    return ImageFont.load_default()


def render_canonical(job: CharacterJob) -> Image.Image:
    adapter = get_adapter(job.target)
    spec = adapter.sample_spec(job)
    return adapter.render_canonical(spec, job)


def write_canonicals(config_dir: str | Path, out_dir: str | Path) -> List[Path]:
    out_dir = Path(out_dir)
    out_dir.mkdir(parents=True, exist_ok=True)
    jobs = load_jobs(config_dir)
    outputs: List[Path] = []
    tiles: List[Tuple[str, Image.Image]] = []
    for _path, job in jobs:
        img = render_canonical(job)
        out = out_dir / f"{job.target}_canonical.png"
        img.save(out)
        outputs.append(out)
        tiles.append((job.target, img))

    if tiles:
        tile_w = max(img.width for _, img in tiles)
        tile_h = max(img.height for _, img in tiles) + 24
        contact = Image.new("RGBA", (tile_w * len(tiles), tile_h), (0, 0, 0, 0))
        draw = ImageDraw.Draw(contact)
        font = _font(14)
        for idx, (name, img) in enumerate(tiles):
            x = idx * tile_w
            contact.alpha_composite(img, (x + (tile_w - img.width) // 2, 20))
            draw.text((x + 8, 3), name, fill=(255, 255, 255, 255), font=font)
        contact_out = out_dir / "canonicals_contact_sheet.png"
        contact.save(contact_out)
        outputs.append(contact_out)
    return outputs
