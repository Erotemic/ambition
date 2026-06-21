from __future__ import annotations

"""Procedural apple sprite for GNU-ton's apple-rain attack.

This is a small, code-generated prop target rather than a full spritesheet.
The renderer emits a single transparent PNG that the game can use directly
for the projectile visual.
"""

import math
from pathlib import Path
from shutil import copy2
from typing import Iterable, List

from PIL import Image, ImageDraw, ImageFilter

from ...authoring.tackon_sheet import build_sheet

TARGET_NAME = "gnu_ton_apple"
SHEET_FILES = (
    f"{TARGET_NAME}.png",
    f"{TARGET_NAME}_spritesheet.yaml",
    f"{TARGET_NAME}_spritesheet.ron",
    f"{TARGET_NAME}_actor.ron",
)

WORK_SIZE = 512
FINAL_SIZE = 256
OUTPUT_NAME = f"{TARGET_NAME}.png"
CANONICAL_NAME = f"{TARGET_NAME}_canonical_transparent.png"
INSTALL_SUBDIR = "gnu_ton_boss"
ROWS = [("idle", 1, 100)]


def _clamp(v: float, lo: float = 0.0, hi: float = 1.0) -> float:
    return max(lo, min(hi, v))


def _rgba(r: int, g: int, b: int, a: int = 255) -> tuple[int, int, int, int]:
    return (r, g, b, a)


def _make_body_mask() -> Image.Image:
    mask = Image.new("L", (WORK_SIZE, WORK_SIZE), 0)
    draw = ImageDraw.Draw(mask)
    # Main apple silhouette: three overlapping lobes and a lower bulge
    # give the body a round but natural outline.
    draw.ellipse((102, 112, 410, 456), fill=255)
    draw.ellipse((56, 158, 292, 462), fill=255)
    draw.ellipse((220, 158, 456, 462), fill=255)
    draw.ellipse((150, 260, 362, 498), fill=255)
    # Carve the stem notch so it reads like an apple rather than a
    # generic red orb.
    draw.ellipse((206, 74, 306, 176), fill=0)
    return mask.filter(ImageFilter.GaussianBlur(radius=2.0))


def _body_image(mask: Image.Image) -> Image.Image:
    img = Image.new("RGBA", (WORK_SIZE, WORK_SIZE), (0, 0, 0, 0))
    pixels = img.load()
    mask_px = mask.load()
    for y in range(WORK_SIZE):
        v = y / max(1, WORK_SIZE - 1)
        for x in range(WORK_SIZE):
            if mask_px[x, y] <= 0:
                continue
            u = x / max(1, WORK_SIZE - 1)
            # Soft left/top highlight and right/bottom shadow produce a
            # polished painted look without any dependency on a photo source.
            highlight = _clamp(
                1.0 - (((u - 0.30) / 0.56) ** 2 + ((v - 0.28) / 0.62) ** 2) ** 0.5
            )
            shadow = _clamp(((u * 0.82) + (v * 0.68) - 0.36) / 0.68)
            stripe = 0.5 + 0.5 * math.sin(u * 22.0 + v * 5.0)
            sheen = _clamp(highlight * 1.2 - shadow * 0.15)
            r = int(round(132 + 88 * sheen + 24 * (1.0 - shadow) + 6 * stripe))
            g = int(round(10 + 44 * highlight - 12 * shadow + 5 * stripe))
            b = int(round(12 + 18 * highlight - 4 * shadow))
            pixels[x, y] = (min(255, r), min(255, g), min(255, b), mask_px[x, y])
    return img


def _make_leaf() -> Image.Image:
    layer = Image.new("RGBA", (WORK_SIZE, WORK_SIZE), (0, 0, 0, 0))
    mask = Image.new("L", (WORK_SIZE, WORK_SIZE), 0)
    draw = ImageDraw.Draw(mask)
    draw.polygon([(252, 122), (352, 88), (404, 140), (350, 196), (258, 178)], fill=255)
    draw.polygon([(246, 126), (176, 158), (216, 214), (282, 184)], fill=200)
    mask = mask.filter(ImageFilter.GaussianBlur(radius=1.3))
    px = layer.load()
    mpx = mask.load()
    for y in range(WORK_SIZE):
        for x in range(WORK_SIZE):
            alpha = mpx[x, y]
            if alpha <= 0:
                continue
            t = _clamp((x - 220) / 160.0)
            r = int(round(42 + 36 * (1.0 - t)))
            g = int(round(112 + 74 * (1.0 - t)))
            b = int(round(28 + 14 * (1.0 - t)))
            px[x, y] = (r, g, b, alpha)

    draw = ImageDraw.Draw(layer, "RGBA")
    # Central vein and some fiber highlights.
    draw.line((266, 160, 374, 126), fill=(208, 220, 98, 210), width=7)
    draw.line((288, 169, 337, 146), fill=(234, 242, 166, 110), width=3)
    draw.line((310, 156, 357, 162), fill=(234, 242, 166, 90), width=2)
    return layer.filter(ImageFilter.GaussianBlur(radius=0.35))


def _make_stem() -> Image.Image:
    layer = Image.new("RGBA", (WORK_SIZE, WORK_SIZE), (0, 0, 0, 0))
    draw = ImageDraw.Draw(layer, "RGBA")
    draw.line((286, 130, 266, 62), fill=(92, 55, 26, 255), width=18)
    draw.line((282, 126, 270, 70), fill=(138, 84, 42, 155), width=6)
    draw.line((281, 118, 272, 76), fill=(194, 141, 80, 110), width=3)
    draw.ellipse((256, 54, 280, 78), fill=(104, 63, 31, 255))
    return layer


def _render_apple() -> Image.Image:
    mask = _make_body_mask()
    canvas = Image.new("RGBA", (WORK_SIZE, WORK_SIZE), (0, 0, 0, 0))
    body = _body_image(mask)
    canvas.alpha_composite(body)

    shade = Image.new("RGBA", (WORK_SIZE, WORK_SIZE), (0, 0, 0, 0))
    sdraw = ImageDraw.Draw(shade, "RGBA")
    sdraw.ellipse((300, 164, 444, 404), fill=(72, 0, 0, 82))
    sdraw.ellipse((86, 154, 222, 396), fill=(255, 182, 132, 86))
    sdraw.ellipse((146, 92, 294, 176), fill=(255, 224, 180, 102))
    shade = shade.filter(ImageFilter.GaussianBlur(radius=20))
    canvas.alpha_composite(shade)

    highlight = Image.new("RGBA", (WORK_SIZE, WORK_SIZE), (0, 0, 0, 0))
    hdraw = ImageDraw.Draw(highlight, "RGBA")
    hdraw.ellipse((116, 138, 256, 304), fill=(255, 236, 210, 88))
    hdraw.ellipse((148, 156, 236, 226), fill=(255, 255, 255, 100))
    highlight = highlight.filter(ImageFilter.GaussianBlur(radius=17))
    canvas.alpha_composite(highlight)

    canvas.alpha_composite(_make_leaf())
    canvas.alpha_composite(_make_stem())

    # Small bloom around the top notch to soften the stem join.
    bloom = Image.new("RGBA", (WORK_SIZE, WORK_SIZE), (0, 0, 0, 0))
    bdraw = ImageDraw.Draw(bloom, "RGBA")
    bdraw.ellipse((210, 96, 306, 176), fill=(255, 204, 130, 36))
    bloom = bloom.filter(ImageFilter.GaussianBlur(radius=10))
    canvas.alpha_composite(bloom)

    return canvas.resize((FINAL_SIZE, FINAL_SIZE), Image.Resampling.LANCZOS)


def render_sheet(out_dir: str | Path, **opts) -> List[Path]:
    out_dir = Path(out_dir)
    out_dir.mkdir(parents=True, exist_ok=True)
    outputs = build_sheet(
        target=TARGET_NAME,
        rows=ROWS,
        render_fn=lambda _anim, _frame_idx, _nframes: _render_apple(),
        out_dir=out_dir,
        frame_size=(FINAL_SIZE, FINAL_SIZE),
        auto_crop=True,
        crop_margin=6,
    )
    # Keep the game-facing asset simple: one transparent apple PNG named
    # after the target, plus the standard YAML / RON metadata sidecars.
    copy2(outputs["canonical_transparent"], out_dir / OUTPUT_NAME)
    paths = [
        out_dir / OUTPUT_NAME,
        outputs["canonical_transparent"],
        outputs["spritesheet"],
        outputs["preview"],
        outputs["yaml"],
        outputs["ron"],
        outputs["actor"],
    ]
    return paths


def render_canonical(out_dir: str | Path, **opts) -> Path:
    del opts
    out_dir = Path(out_dir)
    out_dir.mkdir(parents=True, exist_ok=True)
    outputs = build_sheet(
        target=TARGET_NAME,
        rows=ROWS,
        render_fn=lambda _anim, _frame_idx, _nframes: _render_apple(),
        out_dir=out_dir,
        frame_size=(FINAL_SIZE, FINAL_SIZE),
        auto_crop=True,
        crop_margin=6,
    )
    return outputs["canonical_transparent"]


def install(render_dir: str | Path, dest_root: str | Path) -> Iterable[Path]:
    render_dir = Path(render_dir)
    dest_root = Path(dest_root)
    dest_dir = dest_root / INSTALL_SUBDIR
    dest_dir.mkdir(parents=True, exist_ok=True)
    installed: List[Path] = []
    for name in SHEET_FILES:
        src = render_dir / name
        if not src.exists():
            continue
        dst = dest_dir / name
        dst.write_bytes(src.read_bytes())
        installed.append(dst)
    return installed


def render(out_dir: str | Path, **opts) -> List[Path]:
    return render_sheet(out_dir, **opts)
