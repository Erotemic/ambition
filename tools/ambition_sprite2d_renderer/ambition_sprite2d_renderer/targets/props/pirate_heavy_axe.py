from __future__ import annotations

"""Boarding axe held by the Pirate Heavy.

A heavy two-hand boarding axe: a thick wooden haft wrapped in leather,
a broad bearded steel blade flaring forward, and a stubby reverse spike
on the poll. Rendered AXIS-ALIGNED with the haft horizontal and the
cutting edge pointing RIGHT (+X). The game pins it to the Heavy's hand
at the ``grip`` anchor and rotates the whole sprite to the swing
direction at runtime — see ``_held_prop_common`` for the shared
authoring contract.

Animations:

- ``idle``: 6-frame resting hum. A bright gleam travels along the steel
  edge so the blade reads as live metal catching the light rather than
  a flat painted shape; everything else holds still.
"""

import math
from pathlib import Path
from typing import List, Tuple

from PIL import Image, ImageDraw

from ...authoring.tackon_sheet import build_sheet
from . import _held_prop_common as hp

TARGET_NAME = "pirate_heavy_axe"
SHEET_FILES = (
    f"{TARGET_NAME}_spritesheet.png",
    f"{TARGET_NAME}_spritesheet.yaml",
    f"{TARGET_NAME}_spritesheet.ron",
    f"{TARGET_NAME}_actor.ron",
)

# Generous box; auto-crop hugs the silhouette afterwards.
FRAME_SIZE = (176, 104)
ROWS: List[Tuple[str, int, int]] = [("idle", 6, 120)]

# Haft runs horizontally through the vertical centre. Butt at the left,
# axe head at the right. Coordinates are in final-frame design pixels.
CY = FRAME_SIZE[1] * 0.55
BUTT_X = 14.0
HEAD_X = 150.0
GRIP_X = 52.0  # primary (lower) hand
HAFT_HALF = 4.4  # haft half-thickness


def _draw_haft(d: ImageDraw.ImageDraw) -> None:
    p = hp.px
    # Wooden shaft.
    d.polygon(
        [
            (p(BUTT_X), p(CY - HAFT_HALF)),
            (p(HEAD_X), p(CY - HAFT_HALF)),
            (p(HEAD_X), p(CY + HAFT_HALF)),
            (p(BUTT_X), p(CY + HAFT_HALF)),
        ],
        fill=hp.WOOD,
        outline=hp.OUTLINE,
    )
    # Grain highlight along the top of the shaft.
    d.line(
        [
            (p(BUTT_X + 2), p(CY - HAFT_HALF * 0.5)),
            (p(HEAD_X - 4), p(CY - HAFT_HALF * 0.5)),
        ],
        fill=hp.WOOD_HI,
        width=max(1, int(hp.px(0.9))),
    )
    d.line(
        [
            (p(BUTT_X + 2), p(CY + HAFT_HALF * 0.55)),
            (p(HEAD_X - 4), p(CY + HAFT_HALF * 0.55)),
        ],
        fill=hp.WOOD_DARK,
        width=max(1, int(hp.px(0.8))),
    )
    # Steel butt-cap on the pommel end.
    d.polygon(
        [
            (p(BUTT_X - 4), p(CY - HAFT_HALF - 1.2)),
            (p(BUTT_X + 4), p(CY - HAFT_HALF - 1.2)),
            (p(BUTT_X + 4), p(CY + HAFT_HALF + 1.2)),
            (p(BUTT_X - 4), p(CY + HAFT_HALF + 1.2)),
        ],
        fill=hp.IRON,
        outline=hp.OUTLINE,
    )


def _draw_grip_wrap(d: ImageDraw.ImageDraw) -> None:
    p = hp.px
    # Leather grip wrap around the lower hand: diagonal cross-binding.
    wrap_x0, wrap_x1 = GRIP_X - 12.0, GRIP_X + 12.0
    d.polygon(
        [
            (p(wrap_x0), p(CY - HAFT_HALF - 0.6)),
            (p(wrap_x1), p(CY - HAFT_HALF - 0.6)),
            (p(wrap_x1), p(CY + HAFT_HALF + 0.6)),
            (p(wrap_x0), p(CY + HAFT_HALF + 0.6)),
        ],
        fill=hp.LEATHER,
        outline=hp.OUTLINE,
    )
    for i in range(6):
        x = wrap_x0 + (wrap_x1 - wrap_x0) * (i + 0.5) / 6.0
        d.line(
            [(p(x - 1.6), p(CY - HAFT_HALF)), (p(x + 1.6), p(CY + HAFT_HALF))],
            fill=hp.LEATHER_DARK,
            width=max(1, int(hp.px(0.7))),
        )


def _draw_head(d: ImageDraw.ImageDraw, gleam_t: float) -> None:
    p = hp.px
    # Socket / collar where the head seats on the haft.
    socket_x = HEAD_X - 26.0
    d.polygon(
        [
            (p(socket_x), p(CY - HAFT_HALF - 4.0)),
            (p(HEAD_X - 6), p(CY - HAFT_HALF - 4.0)),
            (p(HEAD_X - 6), p(CY + HAFT_HALF + 4.0)),
            (p(socket_x), p(CY + HAFT_HALF + 4.0)),
        ],
        fill=hp.IRON_DARK,
        outline=hp.OUTLINE,
    )
    d.ellipse(
        (p(socket_x + 2), p(CY - 2.2), p(socket_x + 6), p(CY + 2.2)),
        fill=hp.BRASS,
        outline=hp.OUTLINE,
    )

    # Reverse spike on the poll (top-back of the head).
    d.polygon(
        [
            (p(socket_x + 4), p(CY - HAFT_HALF - 4.0)),
            (p(socket_x + 14), p(CY - HAFT_HALF - 4.0)),
            (p(socket_x + 8), p(CY - HAFT_HALF - 20.0)),
        ],
        fill=hp.IRON,
        outline=hp.OUTLINE,
    )

    # Bearded blade: broad crescent flaring up and forward, with a long
    # "beard" hooking down past the haft on the cutting side.
    blade = [
        (HEAD_X - 8, CY - HAFT_HALF - 6.0),  # back-top, against the socket
        (HEAD_X + 8, CY - 30.0),  # top horn
        (HEAD_X + 22, CY - 12.0),  # leading top
        (HEAD_X + 24, CY + 6.0),  # edge mid (forward point)
        (HEAD_X + 14, CY + 26.0),  # leading bottom
        (HEAD_X - 4, CY + 30.0),  # beard tip (hooks down)
        (HEAD_X - 8, CY + HAFT_HALF + 6.0),  # back-bottom, against the socket
    ]
    d.polygon([(p(x), p(y)) for (x, y) in blade], fill=hp.IRON_HI, outline=hp.OUTLINE)

    # Bevelled inner face (darker), inset from the cutting edge.
    inner = [
        (HEAD_X - 6, CY - HAFT_HALF - 4.0),
        (HEAD_X + 5, CY - 24.0),
        (HEAD_X + 15, CY - 9.0),
        (HEAD_X + 16, CY + 6.0),
        (HEAD_X + 8, CY + 21.0),
        (HEAD_X - 4, CY + 24.0),
        (HEAD_X - 6, CY + HAFT_HALF + 4.0),
    ]
    d.polygon([(p(x), p(y)) for (x, y) in inner], fill=hp.IRON)

    # Bright cutting edge along the forward arc.
    edge = [
        (HEAD_X + 8, CY - 30.0),
        (HEAD_X + 22, CY - 12.0),
        (HEAD_X + 24, CY + 6.0),
        (HEAD_X + 14, CY + 26.0),
        (HEAD_X - 4, CY + 30.0),
    ]
    d.line(
        [(p(x), p(y)) for (x, y) in edge],
        fill=hp.STEEL_EDGE,
        width=max(1, int(hp.px(1.4))),
    )

    # Travelling gleam: a short bright segment sliding along the edge.
    n = len(edge)
    fpos = gleam_t * (n - 1)
    i0 = int(fpos)
    frac = fpos - i0
    if i0 < n - 1:
        ax, ay = edge[i0]
        bx, by = edge[i0 + 1]
        gx = ax + (bx - ax) * frac
        gy = ay + (by - ay) * frac
        for r, a in ((3.0, 90), (1.7, 200), (0.9, 255)):
            d.ellipse(
                (p(gx - r), p(gy - r), p(gx + r), p(gy + r)),
                fill=(255, 255, 255, a),
            )


def _draw_frame(anim: str, frame_idx: int, nframes: int) -> Image.Image:
    if anim != "idle":
        raise ValueError(f"unknown animation: {anim}")
    canvas = hp.new_super(FRAME_SIZE)
    d = ImageDraw.Draw(canvas, "RGBA")
    _draw_haft(d)
    _draw_grip_wrap(d)
    gleam_t = frame_idx / max(1, nframes - 1)
    _draw_head(d, gleam_t)
    return hp.finalize(canvas, FRAME_SIZE)


def _frame_meta(anim: str, frame_idx: int, nframes: int) -> dict:
    del anim, frame_idx, nframes
    return hp.anchor_meta(
        {
            "grip": (GRIP_X, CY),
            "fore_grip": (HEAD_X - 34.0, CY),
            "butt": (BUTT_X, CY),
            "head": (HEAD_X, CY),
            "edge": (HEAD_X + 24.0, CY + 6.0),
        }
    )


ACTOR_METADATA = {
    "actor": {"character_id": "prop_pirate_heavy_axe", "display_name": "Boarding Axe"},
    "body": {
        "body_plan": "Prop",
        "body_kind": "HeldWeapon",
        "mass_class": "Heavy",
        "locomotion_hint": "Stationary",
        "traits": ["prop", "weapon", "axe", "melee", "held", "pirate"],
    },
    "brain": {"default_preset": "stand_still"},
    "actions": {"default_preset": "peaceful"},
    "animation_bindings": {"default": {"animation": "idle", "events": []}},
    "sockets": {
        "grip": {"source": f"{TARGET_NAME}.geometry", "point": {"x": GRIP_X, "y": CY}},
        "head": {"source": f"{TARGET_NAME}.geometry", "point": {"x": HEAD_X, "y": CY}},
        "edge": {
            "source": f"{TARGET_NAME}.geometry",
            "point": {"x": HEAD_X + 24.0, "y": CY + 6.0},
        },
    },
    "tags": ["prop", "weapon", "axe", "held"],
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
        crop_margin=4,
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
