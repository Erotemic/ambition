from __future__ import annotations

"""Procedural anvil prop for the cut-rope boss arena.

The anvil is a visible Prop with kind ``cut_rope_anvil``. Runtime code
moves the LDtk prop down when the rope is cut; this sheet only supplies
its idle art so the entity no longer renders as a generic box.
"""

from pathlib import Path
from typing import List, Tuple

from PIL import Image, ImageColor, ImageDraw

from ...authoring.tackon_sheet import build_sheet, write_canonical

RGBA = Tuple[int, int, int, int]

TARGET_NAME = "cut_rope_anvil"
SHEET_FILES = [
    f"{TARGET_NAME}_spritesheet.png",
    f"{TARGET_NAME}_spritesheet.yaml",
    f"{TARGET_NAME}_spritesheet.ron",
    f"{TARGET_NAME}_actor.ron",
]

ACTOR_METADATA = {
    "actor": {
        "character_id": "prop_cut_rope_anvil",
        "display_name": "Cut-Rope Anvil",
    },
    "body": {
        "body_plan": "Prop",
        "body_kind": "Anvil",
        "mass_class": "Heavy",
        "locomotion_hint": "FallingTrap",
        "traits": ["prop", "trap", "anvil"],
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
            "point": {"x": 64.0, "y": 88.0},
        },
    },
    "tags": ["prop", "trap", "anvil", "boss-arena"],
}

ROWS: List[Tuple[str, int, int]] = [
    ("idle", 1, 1000),
]

FRAME_SIZE = (128, 96)
SUPER = 4
W, H = FRAME_SIZE[0] * SUPER, FRAME_SIZE[1] * SUPER

OUTLINE = ImageColor.getrgb("#111722") + (255,)
METAL_DARK = ImageColor.getrgb("#303A4A") + (255,)
METAL_MID = ImageColor.getrgb("#566373") + (255,)
METAL_LIGHT = ImageColor.getrgb("#9DA7B5") + (255,)
RIM = ImageColor.getrgb("#C8D0DC") + (255,)
STRAP = ImageColor.getrgb("#A66B2E") + (255,)


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

    # Static prop: runtime movement, not sprite animation, handles falling.
    del frame_idx, nframes
    y = 0.0

    # Hanging eye/strap at the top.
    draw.rectangle(
        _box(58, 2 + y, 70, 16 + y), fill=STRAP, outline=OUTLINE, width=_s(1.0)
    )
    draw.ellipse(_box(54, 3 + y, 74, 21 + y), outline=OUTLINE, width=_s(2.0))
    draw.ellipse(_box(59, 8 + y, 69, 18 + y), outline=METAL_LIGHT, width=_s(1.2))

    # Top face and horn. Broad classic blacksmith anvil silhouette.
    horn = _poly(
        [
            (13, 34 + y),
            (40, 23 + y),
            (90, 23 + y),
            (116, 34 + y),
            (93, 43 + y),
            (36, 43 + y),
        ]
    )
    draw.polygon(horn, fill=METAL_MID, outline=OUTLINE)
    draw.line(_poly([(39, 25 + y), (89, 25 + y)]), fill=RIM, width=_s(1.6))

    # Main body taper.
    body = _poly([(33, 43 + y), (96, 43 + y), (87, 67 + y), (43, 67 + y)])
    draw.polygon(body, fill=METAL_DARK, outline=OUTLINE)
    draw.rectangle(_box(42, 45 + y, 86, 52 + y), fill=METAL_LIGHT, outline=None)
    draw.rectangle(_box(43, 53 + y, 85, 64 + y), fill=METAL_MID, outline=None)

    # Hardy hole / face detail.
    draw.rectangle(_box(79, 31 + y, 88, 38 + y), fill=OUTLINE)
    draw.rectangle(_box(18, 33 + y, 39, 38 + y), fill=METAL_LIGHT)

    # Waist and base.
    draw.rectangle(
        _box(51, 67 + y, 78, 79 + y), fill=METAL_MID, outline=OUTLINE, width=_s(1.0)
    )
    base = _poly([(31, 78 + y), (98, 78 + y), (111, 89 + y), (18, 89 + y)])
    draw.polygon(base, fill=METAL_DARK, outline=OUTLINE)
    draw.rectangle(_box(29, 79 + y, 100, 84 + y), fill=METAL_LIGHT, outline=None)
    draw.line(_poly([(22, 88 + y), (106, 88 + y)]), fill=OUTLINE, width=_s(1.0))

    # White-edge chips on the corners.
    draw.line(
        _poly([(96, 31 + y), (105, 34 + y), (96, 38 + y)]), fill=RIM, width=_s(1.2)
    )
    draw.line(_poly([(37, 70 + y), (47, 70 + y)]), fill=RIM, width=_s(1.0))

    return img.resize(FRAME_SIZE, Image.Resampling.LANCZOS)


def _frame_meta(anim: str, frame_idx: int, nframes: int) -> dict:
    del frame_idx, nframes
    return {
        "anchors": {
            "hang": {"x": 64.0, "y": 8.0},
            "impact": {"x": 64.0, "y": 88.0},
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
