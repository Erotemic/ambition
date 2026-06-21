from __future__ import annotations

"""Throwing javelin — a balanced spear meant to be hurled.

A slim wooden shaft with a leaf-shaped steel head at the point end and
a short feathered binding at the butt for flight stability. Rendered
AXIS-ALIGNED with the point facing RIGHT (+X). It doubles as both the
held/cocked weapon and the in-flight projectile: the game pins it at
the ``grip`` (centre of balance) when carried and spins/translates it
along ``forward`` when thrown. See ``_held_prop_common`` for the shared
authoring contract.

Animations:

- ``idle``: 4-frame in-hand shimmer. The steel head catches a soft
  travelling glint; the shaft holds still. The same frames read fine
  spinning in flight because the silhouette is rotationally simple.
"""

import math
from pathlib import Path
from typing import List, Tuple

from PIL import Image, ImageDraw

from ...authoring.tackon_sheet import build_sheet
from . import _held_prop_common as hp

TARGET_NAME = "throwing_javelin"
SHEET_FILES = (
    f"{TARGET_NAME}_spritesheet.png",
    f"{TARGET_NAME}_spritesheet.yaml",
    f"{TARGET_NAME}_spritesheet.ron",
    f"{TARGET_NAME}_actor.ron",
)

FRAME_SIZE = (240, 40)
ROWS: List[Tuple[str, int, int]] = [("idle", 4, 130)]

CY = FRAME_SIZE[1] * 0.5
BUTT_X = 10.0
SHAFT_END_X = 198.0  # where shaft meets the head socket
TIP_X = 232.0  # spear point
GRIP_X = 132.0  # centre of balance (biased toward the head)
SHAFT_HALF = 2.2


def _draw_shaft(d: ImageDraw.ImageDraw) -> None:
    p = hp.px
    d.polygon(
        [
            (p(BUTT_X), p(CY - SHAFT_HALF)),
            (p(SHAFT_END_X), p(CY - SHAFT_HALF)),
            (p(SHAFT_END_X), p(CY + SHAFT_HALF)),
            (p(BUTT_X), p(CY + SHAFT_HALF)),
        ],
        fill=hp.WOOD,
        outline=hp.OUTLINE,
    )
    d.line(
        [
            (p(BUTT_X + 2), p(CY - SHAFT_HALF * 0.4)),
            (p(SHAFT_END_X - 2), p(CY - SHAFT_HALF * 0.4)),
        ],
        fill=hp.WOOD_HI,
        width=max(1, int(hp.px(0.7))),
    )


def _draw_fletching(d: ImageDraw.ImageDraw) -> None:
    p = hp.px
    # Two short feathered vanes near the butt for flight stability.
    for sign in (-1, 1):
        d.polygon(
            [
                (p(BUTT_X + 2), p(CY + sign * SHAFT_HALF)),
                (p(BUTT_X + 26), p(CY + sign * (SHAFT_HALF + 6.0))),
                (p(BUTT_X + 34), p(CY + sign * (SHAFT_HALF + 5.0))),
                (p(BUTT_X + 20), p(CY + sign * SHAFT_HALF)),
            ],
            fill=hp.FLETCH,
            outline=hp.OUTLINE,
        )
    # Binding cord wrap over the vane roots.
    for i in range(3):
        x = BUTT_X + 6 + i * 6
        d.line(
            [(p(x), p(CY - SHAFT_HALF - 1.4)), (p(x), p(CY + SHAFT_HALF + 1.4))],
            fill=hp.LEATHER_DARK,
            width=max(1, int(hp.px(0.8))),
        )


def _draw_grip(d: ImageDraw.ImageDraw) -> None:
    p = hp.px
    # Leather grip band at the balance point.
    d.polygon(
        [
            (p(GRIP_X - 10), p(CY - SHAFT_HALF - 0.8)),
            (p(GRIP_X + 10), p(CY - SHAFT_HALF - 0.8)),
            (p(GRIP_X + 10), p(CY + SHAFT_HALF + 0.8)),
            (p(GRIP_X - 10), p(CY + SHAFT_HALF + 0.8)),
        ],
        fill=hp.LEATHER,
        outline=hp.OUTLINE,
    )
    for i in range(4):
        x = GRIP_X - 7 + i * 4.6
        d.line(
            [(p(x - 1.2), p(CY - SHAFT_HALF)), (p(x + 1.2), p(CY + SHAFT_HALF))],
            fill=hp.LEATHER_DARK,
            width=max(1, int(hp.px(0.6))),
        )


def _draw_head(d: ImageDraw.ImageDraw, glint_t: float) -> None:
    p = hp.px
    # Socket collar.
    d.polygon(
        [
            (p(SHAFT_END_X - 4), p(CY - SHAFT_HALF - 2.4)),
            (p(SHAFT_END_X + 6), p(CY - SHAFT_HALF - 2.4)),
            (p(SHAFT_END_X + 6), p(CY + SHAFT_HALF + 2.4)),
            (p(SHAFT_END_X - 4), p(CY + SHAFT_HALF + 2.4)),
        ],
        fill=hp.IRON_DARK,
        outline=hp.OUTLINE,
    )
    # Leaf-shaped spear head.
    base_x = SHAFT_END_X + 4
    head = [
        (base_x, CY - 1.5),
        (base_x + 8, CY - 7.5),  # widest, top
        (base_x + 20, CY - 3.0),
        (TIP_X, CY),  # point
        (base_x + 20, CY + 3.0),
        (base_x + 8, CY + 7.5),  # widest, bottom
        (base_x, CY + 1.5),
    ]
    d.polygon([(p(x), p(y)) for (x, y) in head], fill=hp.IRON_HI, outline=hp.OUTLINE)
    # Central ridge.
    d.line(
        [(p(base_x + 2), p(CY)), (p(TIP_X - 2), p(CY))],
        fill=hp.IRON,
        width=max(1, int(hp.px(0.9))),
    )
    # Edge highlight + travelling glint on the upper edge.
    upper = [(base_x + 8, CY - 7.5), (base_x + 20, CY - 3.0), (TIP_X, CY)]
    d.line(
        [(p(x), p(y)) for (x, y) in upper],
        fill=hp.STEEL_EDGE,
        width=max(1, int(hp.px(1.1))),
    )
    gx = base_x + 8 + (TIP_X - (base_x + 8)) * glint_t
    gy = CY - 7.5 + (0 - (-7.5)) * glint_t * 0.6
    for r, a in ((2.2, 110), (1.1, 235)):
        d.ellipse((p(gx - r), p(gy - r), p(gx + r), p(gy + r)), fill=(255, 255, 255, a))


def _draw_frame(anim: str, frame_idx: int, nframes: int) -> Image.Image:
    if anim != "idle":
        raise ValueError(f"unknown animation: {anim}")
    canvas = hp.new_super(FRAME_SIZE)
    d = ImageDraw.Draw(canvas, "RGBA")
    _draw_shaft(d)
    _draw_fletching(d)
    _draw_grip(d)
    _draw_head(d, frame_idx / max(1, nframes - 1))
    return hp.finalize(canvas, FRAME_SIZE)


def _frame_meta(anim: str, frame_idx: int, nframes: int) -> dict:
    del anim, frame_idx, nframes
    return hp.anchor_meta(
        {
            "grip": (GRIP_X, CY),
            "butt": (BUTT_X, CY),
            "tip": (TIP_X, CY),
        }
    )


ACTOR_METADATA = {
    "actor": {
        "character_id": "prop_throwing_javelin",
        "display_name": "Throwing Javelin",
    },
    "body": {
        "body_plan": "Prop",
        "body_kind": "Projectile",
        "mass_class": "Light",
        "locomotion_hint": "Thrown",
        "traits": ["prop", "weapon", "javelin", "spear", "thrown", "projectile"],
    },
    "brain": {"default_preset": "stand_still"},
    "actions": {"default_preset": "peaceful"},
    "animation_bindings": {"default": {"animation": "idle", "events": []}},
    "sockets": {
        "grip": {"source": f"{TARGET_NAME}.geometry", "point": {"x": GRIP_X, "y": CY}},
        "tip": {"source": f"{TARGET_NAME}.geometry", "point": {"x": TIP_X, "y": CY}},
        "butt": {"source": f"{TARGET_NAME}.geometry", "point": {"x": BUTT_X, "y": CY}},
    },
    "tags": ["prop", "weapon", "javelin", "thrown"],
}


def render(out_dir: str | Path, **opts) -> List[Path]:
    out_dir = Path(out_dir)
    out_dir.mkdir(parents=True, exist_ok=True)
    outputs = build_sheet(
        target=TARGET_NAME,
        rows=ROWS,
        render_fn=_draw_frame,
        out_dir=out_dir,
        frame_size=opts.get("frame_size") or FRAME_SIZE,
        frame_meta_fn=_frame_meta,
        auto_crop=True,
        crop_margin=3,
        actor_metadata=ACTOR_METADATA,
    )
    return [
        outputs["canonical"],
        outputs["canonical_transparent"],
        outputs["spritesheet"],
        outputs["yaml"],
        outputs["ron"],
        outputs["actor"],
        outputs["preview"],
    ]
