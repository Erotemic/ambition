from __future__ import annotations

"""Procedural rope prop for the cut-rope boss arena.

A narrow hanging rope authored as a Prop with kind ``cut_rope_rope``.
The sprite is intentionally just the visible rope above the anvil; the
LDtk hitbox is likewise authored above the anvil so players cannot cut
an invisible extension below it.
"""

from pathlib import Path
from typing import List, Tuple

from PIL import Image, ImageColor, ImageDraw

from ...authoring.tackon_sheet import build_sheet, write_canonical

RGBA = Tuple[int, int, int, int]

TARGET_NAME = "cut_rope_rope"
SHEET_FILES = [
    f"{TARGET_NAME}_spritesheet.png",
    f"{TARGET_NAME}_spritesheet.yaml",
    f"{TARGET_NAME}_spritesheet.ron",
    f"{TARGET_NAME}_actor.ron",
]

ACTOR_METADATA = {
    "actor": {
        "character_id": "prop_cut_rope_rope",
        "display_name": "Cut-Rope Rope",
    },
    "body": {
        "body_plan": "Prop",
        "body_kind": "HangingRope",
        "mass_class": "Light",
        "locomotion_hint": "Stationary",
        "traits": ["prop", "rope", "cuttable"],
    },
    "brain": {"default_preset": "stand_still"},
    "actions": {"default_preset": "peaceful"},
    "animation_bindings": {
        "default": {"animation": "idle", "events": []},
    },
    "sockets": {
        "top": {"source": f"{TARGET_NAME}.geometry", "point": {"x": 24.0, "y": 4.0}},
        "cut": {"source": f"{TARGET_NAME}.geometry", "point": {"x": 24.0, "y": 92.0}},
        "bottom": {
            "source": f"{TARGET_NAME}.geometry",
            "point": {"x": 24.0, "y": 188.0},
        },
    },
    "tags": ["prop", "rope", "cuttable", "boss-arena"],
}

ROWS: List[Tuple[str, int, int]] = [
    ("idle", 1, 1000),
]

FRAME_SIZE = (48, 192)
SUPER = 4
W, H = FRAME_SIZE[0] * SUPER, FRAME_SIZE[1] * SUPER

ROPE_DARK = ImageColor.getrgb("#5B3518") + (255,)
ROPE_MID = ImageColor.getrgb("#A66B2E") + (255,)
ROPE_LIGHT = ImageColor.getrgb("#E2B15E") + (255,)
ROPE_SHADOW = ImageColor.getrgb("#2B1A12") + (255,)
BINDING = ImageColor.getrgb("#D9C28F") + (255,)


def _s(v: float) -> int:
    return int(round(v * SUPER))


def _box(x1: float, y1: float, x2: float, y2: float) -> Tuple[int, int, int, int]:
    return (_s(x1), _s(y1), _s(x2), _s(y2))


def _rope_wave(y: float, strand: int) -> float:
    # Static braided silhouette. A deterministic triangular-ish wave reads as
    # rope twist without the DNA/dancing-flower motion the old idle row had.
    t = (y * 0.08 + strand * 0.33) % 1.0
    return (abs(t - 0.5) - 0.25) * 7.0


def _rope_band_wave(y: float) -> float:
    t = (y * 0.08) % 1.0
    return (abs(t - 0.5) - 0.25) * 5.0


def _draw_frame(anim: str, frame_idx: int, nframes: int) -> Image.Image:
    del frame_idx, nframes
    if anim != "idle":
        raise ValueError(f"unknown animation: {anim}")
    img = Image.new("RGBA", (W, H), (0, 0, 0, 0))
    draw = ImageDraw.Draw(img, "RGBA")

    cx = 24.0

    # Top tie/loop. No drop shadow: this is the visible rope only.
    draw.ellipse(_box(cx - 6, 2, cx + 6, 16), outline=ROPE_DARK, width=_s(3.0))
    draw.ellipse(_box(cx - 3.5, 5, cx + 3.5, 13), outline=ROPE_LIGHT, width=_s(1.1))
    draw.rectangle(_box(cx - 7, 15, cx + 7, 22), fill=ROPE_DARK)
    for x in (cx - 4.5, cx, cx + 4.5):
        draw.line((_s(x), _s(15), _s(x), _s(22)), fill=ROPE_LIGHT, width=_s(0.6))

    # Braided strands. Draw three sinusoidal strands and periodic bands
    # so the line reads as twisted rope at small scale.
    for strand, color, offset in [
        (0, ROPE_DARK, -3.4),
        (1, ROPE_MID, 0.0),
        (2, ROPE_LIGHT, 3.4),
    ]:
        pts = []
        for y in range(20, 188, 3):
            wave = _rope_wave(y, strand)
            pts.append((_s(cx + offset + wave), _s(y)))
        draw.line(pts, fill=color, width=_s(2.2))

    for y in range(26, 184, 12):
        wave = _rope_band_wave(y)
        draw.arc(
            _box(cx - 6 + wave, y - 4, cx + 6 + wave, y + 8),
            210,
            330,
            fill=ROPE_SHADOW,
            width=_s(1.0),
        )
        draw.arc(
            _box(cx - 6 - wave, y - 4, cx + 6 - wave, y + 8),
            30,
            150,
            fill=ROPE_LIGHT,
            width=_s(0.8),
        )

    # A faint cut marker near the intended strike height.
    draw.line((_s(cx - 7), _s(92), _s(cx + 7), _s(92)), fill=BINDING, width=_s(0.7))

    return img.resize(FRAME_SIZE, Image.Resampling.LANCZOS)


def _frame_meta(anim: str, frame_idx: int, nframes: int) -> dict:
    del frame_idx, nframes
    return {
        "anchors": {
            "top": {"x": 24.0, "y": 4.0},
            "cut": {"x": 24.0, "y": 92.0},
            "bottom": {"x": 24.0, "y": 188.0},
        },
        "prop": {"kind": TARGET_NAME, "animation": anim},
    }


def render(out_dir: str | Path, **opts) -> List[Path]:
    del opts
    out_dir = Path(out_dir)
    out_dir.mkdir(parents=True, exist_ok=True)
    outputs = build_sheet(
        target=TARGET_NAME,
        rows=ROWS,
        render_fn=_draw_frame,
        out_dir=out_dir,
        frame_size=FRAME_SIZE,
        label_width=108,
        frame_meta_fn=_frame_meta,
        auto_crop=False,
        actor_metadata=ACTOR_METADATA,
    )
    return [
        outputs["spritesheet"],
        outputs["yaml"],
        outputs["ron"],
        outputs["actor"],
        outputs["preview"],
        outputs["canonical"],
        outputs["canonical_transparent"],
    ]


def render_canonical(out_dir: str | Path, **opts) -> Path:
    del opts
    return write_canonical(
        TARGET_NAME,
        ROWS,
        _draw_frame,
        Path(out_dir),
        frame_size=FRAME_SIZE,
        label_width=108,
    )
