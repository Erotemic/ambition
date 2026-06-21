from __future__ import annotations

"""Procedural heal/save shrine prop sprites.

The shrine is a world prop, not an icon. This target renders a proper
spritesheet with two states:

- `idle`: a slow, living glow around the crystal and rune
- `activate`: a brighter burst with a stronger halo and flare

The runtime now consumes `sprites/shrine_spritesheet.png` directly. This
target still publishes a compatibility flat PNG for older callers that
expect `shrine.png`.
"""

import math
import shutil
from pathlib import Path
from typing import List, Tuple

from PIL import Image, ImageDraw, ImageFilter
from ambition_sprite2d_renderer.core.draw import with_alpha

from ...authoring.tackon_sheet import build_sheet

RGBA = Tuple[int, int, int, int]

TARGET_NAME = "shrine"
FRAME_SIZE = (88, 160)
SUPER = 4

ROWS: List[Tuple[str, int, int]] = [
    ("idle", 6, 150),
    ("activate", 8, 90),
]

SHEET_FILES = (
    f"{TARGET_NAME}_spritesheet.png",
    f"{TARGET_NAME}_spritesheet.yaml",
    f"{TARGET_NAME}_spritesheet.ron",
    f"{TARGET_NAME}_actor.ron",
    f"{TARGET_NAME}.png",
)

_STONE_DARK = (42, 49, 66, 255)
_STONE_MID = (60, 70, 92, 255)
_STONE_LIGHT = (88, 100, 124, 255)
_STONE_HI = (118, 132, 154, 255)
_SHRINE_ACCENT_IDLE = (140, 242, 217, 255)
_SHRINE_ACCENT_ACTIVE = (255, 118, 92, 255)
_SHRINE_CORE_IDLE = (246, 255, 250, 255)
_SHRINE_CORE_ACTIVE = (255, 241, 220, 255)
_SHRINE_OUTLINE = (5, 7, 13, 255)


def _s(v: float) -> float:
    return v * SUPER


def _box(x1: float, y1: float, x2: float, y2: float) -> Tuple[float, float, float, float]:
    return (_s(x1), _s(y1), _s(x2), _s(y2))


def _rgba(color: Tuple[int, int, int] | Tuple[int, int, int, int], alpha: int = 255) -> RGBA:
    return (color[0], color[1], color[2], alpha)


def _mix(a: RGBA, b: RGBA, t: float) -> RGBA:
    t = max(0.0, min(1.0, t))
    return (
        int(round(a[0] + (b[0] - a[0]) * t)),
        int(round(a[1] + (b[1] - a[1]) * t)),
        int(round(a[2] + (b[2] - a[2]) * t)),
        int(round(a[3] + (b[3] - a[3]) * t)),
    )


def _ease(t: float) -> float:
    t = max(0.0, min(1.0, t))
    return t * t * (3.0 - 2.0 * t)


def _downsample(img: Image.Image) -> Image.Image:
    return img.resize(FRAME_SIZE, Image.Resampling.LANCZOS)


def _frame_params(animation: str, frame_idx: int, n_frames: int) -> dict:
    if animation == "idle":
        phase = (frame_idx / max(1, n_frames)) * math.tau
        breath = 0.5 + 0.5 * math.sin(phase)
        return {
            "halo": 0.52 + 0.36 * breath,
            "rune": 0.72 + 0.20 * breath,
            "crystal": 0.72 + 0.28 * breath,
            "burst": 0.0,
            "spark": 0.34 + 0.28 * breath,
            "mode": "idle",
            "activation": 0.0,
            "accent_mix": 0.05 + 0.12 * breath,
            "core_mix": 0.10 + 0.18 * breath,
            "stone_mix": 0.04 + 0.10 * breath,
        }
    if animation == "activate":
        t = frame_idx / max(1, n_frames - 1)
        rise = _ease(t)
        flash = math.sin(math.pi * t)
        return {
            "halo": 0.92 + 0.86 * flash,
            "rune": 0.92 + 0.72 * rise,
            "crystal": 0.94 + 0.55 * rise,
            "burst": 0.48 + 1.45 * flash,
            "spark": 0.72 + 0.58 * flash,
            "mode": "activate",
            "activation": rise,
            "accent_mix": 0.40 + 0.60 * rise,
            "core_mix": 0.22 + 0.78 * rise,
            "stone_mix": 0.16 + 0.28 * rise,
        }
    raise ValueError(f"unknown animation: {animation}")


def _draw_base(img: Image.Image, draw: ImageDraw.ImageDraw, params: dict) -> None:
    activation = params["activation"]
    accent = _mix(
        _rgba(_SHRINE_ACCENT_IDLE),
        _rgba(_SHRINE_ACCENT_ACTIVE),
        params["accent_mix"],
    )
    core = _mix(
        _rgba(_SHRINE_CORE_IDLE),
        _rgba(_SHRINE_CORE_ACTIVE),
        params["core_mix"],
    )
    stone_hi = _mix(_rgba(_STONE_HI), _rgba((140, 90, 88, 255)), params["stone_mix"])
    shadow = (0, 0, 0, 72)
    draw.ellipse(_box(12, 149, 76, 159), fill=shadow)

    # Backlit aura. The activation animation broadens the glow and pushes it
    # brighter without changing the shrine's footprint.
    aura = Image.new("RGBA", (FRAME_SIZE[0] * SUPER, FRAME_SIZE[1] * SUPER), (0, 0, 0, 0))
    ad = ImageDraw.Draw(aura, "RGBA")
    aura_layers = [
        (48, 22, 18, 0.16),
        (38, 18, 28, 0.18),
        (26, 12, 40, 0.25),
    ]
    for rx, ry, cy, mul in aura_layers:
        alpha = int(255 * (params["halo"] * mul))
        ad.ellipse(_box(44 - rx / 2.0, cy - ry / 2.0, 44 + rx / 2.0, cy + ry / 2.0), fill=with_alpha(accent, alpha))
    if activation > 0.0:
        ring_r = 19 + 18 * activation
        ad.ellipse(
            _box(44 - ring_r, 74 - ring_r, 44 + ring_r, 74 + ring_r),
            outline=with_alpha(accent, int(140 * activation)),
            width=max(1, int(2.4 * SUPER)),
        )
    aura = aura.filter(ImageFilter.GaussianBlur(radius=6 * SUPER))
    img.alpha_composite(aura)

    # Stone steps.
    d = draw
    d.rectangle(_box(14, 142, 74, 157), fill=_STONE_DARK, outline=_SHRINE_OUTLINE, width=max(1, int(2 * SUPER)))
    d.rectangle(_box(20, 130, 68, 144), fill=_STONE_MID, outline=_SHRINE_OUTLINE, width=max(1, int(2 * SUPER)))
    d.rectangle(_box(26, 119, 62, 132), fill=_STONE_LIGHT, outline=_SHRINE_OUTLINE, width=max(1, int(2 * SUPER)))

    shaft_fill = _mix(_STONE_MID, stone_hi, 0.20 * activation)
    cap_fill = _mix(_STONE_LIGHT, stone_hi, 0.25 * activation)
    d.polygon(
        [(_s(33), _s(121)), (_s(55), _s(121)), (_s(50), _s(46)), (_s(38), _s(46))],
        fill=shaft_fill,
        outline=_SHRINE_OUTLINE,
    )
    d.line([(_s(38), _s(46)), (_s(33), _s(121))], fill=_rgba(_STONE_HI), width=max(1, int(2 * SUPER)))
    d.polygon(
        [(_s(38), _s(47)), (_s(50), _s(47)), (_s(44), _s(28))],
        fill=cap_fill,
        outline=_SHRINE_OUTLINE,
    )

    # Soft inner light running up the obelisk during activation.
    if activation > 0.0:
        glow = Image.new("RGBA", (FRAME_SIZE[0] * SUPER, FRAME_SIZE[1] * SUPER), (0, 0, 0, 0))
        gd = ImageDraw.Draw(glow, "RGBA")
        gd.polygon(
            [(_s(41), _s(118)), (_s(47), _s(118)), (_s(45), _s(48)), (_s(43), _s(48))],
            fill=with_alpha(accent, int(115 * activation)),
        )
        glow = glow.filter(ImageFilter.GaussianBlur(radius=4 * SUPER))
        img.alpha_composite(glow)

    # Rune cross.
    rx, ry = 44, 102
    rune_alpha = int(255 * params["rune"])
    d.line(
        [(_s(rx - 6), _s(ry)), (_s(rx + 6), _s(ry))],
        fill=with_alpha(accent, rune_alpha),
        width=max(1, int(2.5 * SUPER)),
    )
    d.line(
        [(_s(rx), _s(ry - 6)), (_s(rx), _s(ry + 6))],
        fill=with_alpha(accent, rune_alpha),
        width=max(1, int(2.5 * SUPER)),
    )

    # Crystal with halo + brighter core.
    crystal_alpha = int(220 * params["crystal"])
    for r, a in [(20, int(80 * params["crystal"])), (14, int(130 * params["crystal"]))]:
        d.ellipse(_box(44 - r, 74 - r, 44 + r, 74 + r), fill=with_alpha(accent, a))
    if activation > 0.0:
        for r, a in [(28, 90), (18, 130)]:
            d.ellipse(_box(44 - r, 74 - r, 44 + r, 74 + r), outline=with_alpha(accent, int(a * activation)), width=max(1, int(2 * SUPER)))
    d.polygon(
        [(_s(44), _s(60)), (_s(53), _s(74)), (_s(44), _s(90)), (_s(35), _s(74))],
        fill=with_alpha(accent, crystal_alpha),
        outline=_SHRINE_OUTLINE,
    )
    d.polygon(
        [(_s(44), _s(65)), (_s(49), _s(74)), (_s(44), _s(84)), (_s(39), _s(74))],
        fill=with_alpha(core, 220),
    )
    d.line(
        [(_s(44), _s(60)), (_s(44), _s(90))],
        fill=with_alpha(core, 160 + int(70 * params["crystal"])),
        width=max(1, int(1.4 * SUPER)),
    )

    # Small orbiting sparks on activation, more subdued in idle.
    spark_layer = Image.new("RGBA", (FRAME_SIZE[0] * SUPER, FRAME_SIZE[1] * SUPER), (0, 0, 0, 0))
    sd = ImageDraw.Draw(spark_layer, "RGBA")
    sparks = 3 if params["mode"] == "idle" else 6
    for i in range(sparks):
        phase = (i / max(1, sparks)) * math.tau + params["activation"] * math.pi * 1.5
        radius = (11.0 + 10.0 * params["activation"]) * SUPER
        sx = 44 * SUPER + math.cos(phase) * radius
        sy = 74 * SUPER + math.sin(phase) * radius * 0.55
        sr = (0.9 + 0.7 * params["spark"]) * SUPER
        sd.ellipse(
            (sx - sr, sy - sr, sx + sr, sy + sr),
            fill=with_alpha(accent, int(70 + 130 * params["spark"])),
        )
    spark_layer = spark_layer.filter(ImageFilter.GaussianBlur(radius=max(1, int(1.2 * SUPER))))
    img.alpha_composite(spark_layer)

    # Apex spark crowning the cap.
    apex_alpha = int(255 * params["spark"])
    for r, a in [(9, int(90 * params["spark"])), (4, int(180 * params["spark"]))]:
        d.ellipse(_box(44 - r, 25 - r, 44 + r, 25 + r), fill=with_alpha(accent, a))
    d.ellipse(_box(44 - 3, 25 - 3, 44 + 3, 25 + 3), fill=with_alpha(core, apex_alpha))

    # On activation the whole crystal gets a brief flare line to sell the
    # "switching on" moment.
    if activation > 0.45:
        flare = int(180 * (activation - 0.45) / 0.55)
        d.line(
            [(_s(44), _s(58)), (_s(44), _s(26))],
            fill=with_alpha(core, flare),
            width=max(1, int(1.8 * SUPER)),
        )




def render_frame(animation: str, frame_idx: int, n_frames: int) -> Image.Image:
    params = _frame_params(animation, frame_idx, n_frames)
    img = Image.new("RGBA", (FRAME_SIZE[0] * SUPER, FRAME_SIZE[1] * SUPER), (0, 0, 0, 0))
    draw = ImageDraw.Draw(img, "RGBA")
    _draw_base(img, draw, params)
    return _downsample(img)


def render(out_dir: str | Path, **opts) -> List[Path]:
    del opts
    out_dir = Path(out_dir)
    out_dir.mkdir(parents=True, exist_ok=True)
    outputs = build_sheet(
        target=TARGET_NAME,
        rows=ROWS,
        render_fn=render_frame,
        out_dir=out_dir,
        frame_size=FRAME_SIZE,
        label_width=112,
        auto_crop=False,
    )
    shrine_png = out_dir / f"{TARGET_NAME}.png"
    shutil.copy2(outputs["canonical_transparent"], shrine_png)
    return [
        outputs["canonical"],
        outputs["canonical_transparent"],
        outputs["spritesheet"],
        outputs["yaml"],
        outputs["ron"],
        outputs["actor"],
        outputs["preview"],
        shrine_png,
    ]


def write_shrine_prop(
    out_dir: str | Path, *, size: Tuple[int, int] = FRAME_SIZE
) -> Path:
    """Compatibility helper for older callers that still expect `shrine.png`."""
    out_dir = Path(out_dir)
    out_dir.mkdir(parents=True, exist_ok=True)
    render(out_dir)
    shrine_png = out_dir / f"{TARGET_NAME}.png"
    if size != FRAME_SIZE:
        with Image.open(out_dir / f"{TARGET_NAME}_canonical_transparent.png") as img:
            img.resize(size, Image.Resampling.LANCZOS).save(shrine_png)
    return shrine_png
