from __future__ import annotations

"""Procedural upright piano prop for the cut-rope boss arena.

The piano is an alternate visible heavy object for the LDtk-authored
``cut_rope_anvil`` trap slot. Runtime code keeps the authored collision / fall
path stable and swaps only the displayed prop kind according to the replay
cycle.
"""

from pathlib import Path
from typing import List, Tuple

from PIL import Image, ImageColor, ImageDraw

from ...authoring.tackon_sheet import build_sheet, write_canonical

RGBA = Tuple[int, int, int, int]

TARGET_NAME = "cut_rope_piano"
SHEET_FILES = [
    f"{TARGET_NAME}_spritesheet.png",
    f"{TARGET_NAME}_spritesheet.yaml",
    f"{TARGET_NAME}_spritesheet.ron",
    f"{TARGET_NAME}_actor.ron",
]

ACTOR_METADATA = {
    "actor": {
        "character_id": "prop_cut_rope_piano",
        "display_name": "Cut-Rope Piano",
    },
    "body": {
        "body_plan": "Prop",
        "body_kind": "Piano",
        "mass_class": "Heavy",
        "locomotion_hint": "FallingTrap",
        "traits": ["prop", "trap", "piano"],
    },
    "brain": {"default_preset": "stand_still"},
    "actions": {"default_preset": "peaceful"},
    "animation_bindings": {
        "default": {"animation": "idle", "events": []},
    },
    "sockets": {
        "hang": {"source": f"{TARGET_NAME}.geometry", "point": {"x": 64.0, "y": 8.0}},
        "impact": {
            "source": f"{TARGET_NAME}.geometry",
            "point": {"x": 64.0, "y": 90.0},
        },
    },
    "tags": ["prop", "trap", "piano", "boss-arena"],
}

ROWS: List[Tuple[str, int, int]] = [
    ("idle", 1, 1000),
]

FRAME_SIZE = (128, 96)
SUPER = 4
W, H = FRAME_SIZE[0] * SUPER, FRAME_SIZE[1] * SUPER

OUTLINE = ImageColor.getrgb("#171016") + (255,)
WOOD_DARK = ImageColor.getrgb("#332018") + (255,)
WOOD_MID = ImageColor.getrgb("#5A3524") + (255,)
WOOD_LIGHT = ImageColor.getrgb("#9A6040") + (255,)
BRASS = ImageColor.getrgb("#D6A64B") + (255,)
KEY_WHITE = ImageColor.getrgb("#F4EBD9") + (255,)
KEY_BLACK = ImageColor.getrgb("#171820") + (255,)
HIGHLIGHT = ImageColor.getrgb("#F0B56A") + (255,)


def _s(v: float) -> int:
    return int(round(v * SUPER))


def _box(x1: float, y1: float, x2: float, y2: float) -> Tuple[int, int, int, int]:
    return (_s(x1), _s(y1), _s(x2), _s(y2))


def _poly(points: List[Tuple[float, float]]) -> List[Tuple[int, int]]:
    return [(_s(x), _s(y)) for x, y in points]


def _draw_frame(anim: str, frame_idx: int, nframes: int) -> Image.Image:
    if anim != "idle":
        raise ValueError(f"unknown animation: {anim}")
    img = Image.new("RGBA", (W, H), (0, 0, 0, 0))
    draw = ImageDraw.Draw(img, "RGBA")

    del frame_idx, nframes
    y = 0.0

    # Hanging hook / strap. Match the anvil's hang socket so the trap reads
    # as the same authored prop mechanically, just with a sillier payload.
    draw.rectangle(
        _box(59, 2 + y, 69, 18 + y), fill=BRASS, outline=OUTLINE, width=_s(1.0)
    )
    draw.ellipse(_box(54, 4 + y, 74, 24 + y), outline=OUTLINE, width=_s(2.0))
    draw.ellipse(_box(59, 9 + y, 69, 19 + y), outline=BRASS, width=_s(1.4))

    # Upright piano cabinet, slightly cartooned but with a readable heavy mass.
    draw.rounded_rectangle(
        _box(25, 26 + y, 103, 88 + y),
        radius=_s(5.0),
        fill=WOOD_DARK,
        outline=OUTLINE,
        width=_s(2.0),
    )
    draw.rounded_rectangle(
        _box(31, 31 + y, 97, 51 + y),
        radius=_s(3.0),
        fill=WOOD_MID,
        outline=OUTLINE,
        width=_s(1.2),
    )
    draw.rectangle(
        _box(33, 53 + y, 95, 70 + y), fill=WOOD_LIGHT, outline=OUTLINE, width=_s(1.2)
    )
    draw.rectangle(_box(34, 56 + y, 94, 68 + y), fill=KEY_WHITE, outline=None)

    # Individual white and black keys.
    for x in range(36, 94, 8):
        draw.line(_poly([(x, 56 + y), (x, 68 + y)]), fill=OUTLINE, width=_s(0.7))
    for x in (40, 48, 64, 72, 80):
        draw.rectangle(_box(x, 56 + y, x + 4, 63 + y), fill=KEY_BLACK, outline=None)

    # Lid/side accents and front panel.
    draw.line(_poly([(34, 34 + y), (94, 34 + y)]), fill=HIGHLIGHT, width=_s(1.5))
    draw.arc(
        _box(41, 32 + y, 87, 51 + y), start=190, end=350, fill=HIGHLIGHT, width=_s(1.2)
    )
    draw.rectangle(
        _box(36, 72 + y, 92, 82 + y), fill=WOOD_MID, outline=OUTLINE, width=_s(1.0)
    )
    draw.line(_poly([(40, 77 + y), (88, 77 + y)]), fill=WOOD_LIGHT, width=_s(1.2))

    # Little feet / casters.
    draw.rectangle(
        _box(32, 86 + y, 42, 92 + y), fill=WOOD_DARK, outline=OUTLINE, width=_s(1.0)
    )
    draw.rectangle(
        _box(86, 86 + y, 96, 92 + y), fill=WOOD_DARK, outline=OUTLINE, width=_s(1.0)
    )
    draw.ellipse(_box(33, 90 + y, 41, 95 + y), fill=BRASS, outline=OUTLINE)
    draw.ellipse(_box(87, 90 + y, 95, 95 + y), fill=BRASS, outline=OUTLINE)

    # Impact-facing outline/chips so it still reads under motion blur / explosion.
    draw.line(_poly([(26, 87 + y), (102, 87 + y)]), fill=OUTLINE, width=_s(1.1))
    draw.line(
        _poly([(97, 35 + y), (101, 41 + y), (97, 46 + y)]),
        fill=HIGHLIGHT,
        width=_s(1.1),
    )

    return img.resize(FRAME_SIZE, Image.Resampling.LANCZOS)


def _frame_meta(anim: str, frame_idx: int, nframes: int) -> dict:
    del frame_idx, nframes
    return {
        "anchors": {
            "hang": {"x": 64.0, "y": 8.0},
            "impact": {"x": 64.0, "y": 90.0},
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
        label_width=124,
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
        label_width=124,
    )
