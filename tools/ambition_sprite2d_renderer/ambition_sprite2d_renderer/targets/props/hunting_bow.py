from __future__ import annotations

"""Hunting bow — a recurved wooden bow that fires the ``bow_arrow`` prop.

Rendered in the canonical AIMING-RIGHT pose: the wooden limbs arc toward
the target on the RIGHT (+X), the bowstring is the chord on the
archer's side (LEFT), and the arrow rests at the grip pointing right.
The game pins the bow to the wielder's hand at the ``grip`` anchor and
rotates the sprite to the aim direction; it spawns / pins the separate
arrow sprite at the ``nock`` anchor. See ``_held_prop_common`` for the
shared authoring contract, and ``bow_arrow`` for the matching
projectile.

Animations:

- ``idle``: 1-frame braced-at-rest pose. String straight, no draw.
- ``draw``: 4-frame pull. The string midpoint (and the reported
  ``nock`` anchor) is hauled back toward the archer while the limbs
  flex, so the game can play the draw and read the live nock position
  for arrow placement / release.
"""

import math
from pathlib import Path
from typing import List, Tuple

from PIL import Image, ImageDraw

from ...authoring.tackon_sheet import build_sheet, ease_in_out
from . import _held_prop_common as hp

TARGET_NAME = "hunting_bow"
SHEET_FILES = (
    f"{TARGET_NAME}_spritesheet.png",
    f"{TARGET_NAME}_spritesheet.yaml",
    f"{TARGET_NAME}_spritesheet.ron",
    f"{TARGET_NAME}_actor.ron",
)

FRAME_SIZE = (128, 184)
ROWS: List[Tuple[str, int, int]] = [("idle", 1, 200), ("draw", 4, 90)]

STRING_X = FRAME_SIZE[0] * 0.40  # chord (string) line, archer side
TOP_Y = 16.0
BOT_Y = FRAME_SIZE[1] - 16.0
MID_Y = (TOP_Y + BOT_Y) * 0.5
BOW_DEPTH = 44.0  # how far the limbs bulge toward the target (+X)
MAX_PULL = 34.0  # string pull-back at full draw


def _limb_points(depth: float, n: int = 22) -> List[Tuple[float, float]]:
    """Sample the wooden bow arc, endpoints on the string line, bulging
    +X by ``depth`` at the midpoint."""
    pts: List[Tuple[float, float]] = []
    for i in range(n + 1):
        t = i / n
        y = TOP_Y + (BOT_Y - TOP_Y) * t
        bulge = math.sin(t * math.pi) * depth
        pts.append((STRING_X + bulge, y))
    return pts


def _draw_bow(d: ImageDraw.ImageDraw, depth: float) -> None:
    p = hp.px
    pts = [(p(x), p(y)) for (x, y) in _limb_points(depth)]
    # Wood body.
    d.line(pts, fill=hp.WOOD, width=max(2, int(hp.px(4.6))), joint="curve")
    # Dark back-edge + warm front highlight for a rounded limb read.
    d.line(pts, fill=hp.WOOD_DARK, width=max(1, int(hp.px(1.4))), joint="curve")
    hi = [(p(x - 1.4), p(y)) for (x, y) in _limb_points(depth)]
    d.line(hi, fill=hp.WOOD_HI, width=max(1, int(hp.px(1.2))), joint="curve")

    # Nock caps at the limb tips.
    for tx, ty in (_limb_points(depth)[0], _limb_points(depth)[-1]):
        d.ellipse(
            (p(tx - 2.4), p(ty - 2.4), p(tx + 2.4), p(ty + 2.4)),
            fill=hp.IRON,
            outline=hp.OUTLINE,
        )

    # Riser / grip: darker leather wrap at the belly (max bulge).
    grip_cx = STRING_X + depth
    d.polygon(
        [
            (p(grip_cx - 3.4), p(MID_Y - 16)),
            (p(grip_cx + 3.4), p(MID_Y - 16)),
            (p(grip_cx + 3.4), p(MID_Y + 16)),
            (p(grip_cx - 3.4), p(MID_Y + 16)),
        ],
        fill=hp.LEATHER,
        outline=hp.OUTLINE,
    )
    for i in range(5):
        y = MID_Y - 12 + i * 6
        d.line(
            [(p(grip_cx - 3.4), p(y)), (p(grip_cx + 3.4), p(y + 2))],
            fill=hp.LEATHER_DARK,
            width=max(1, int(hp.px(0.7))),
        )


def _draw_string(
    d: ImageDraw.ImageDraw, depth: float, pull: float
) -> Tuple[float, float]:
    """Draw the bowstring; return the nock (string-midpoint) position."""
    p = hp.px
    top = (STRING_X, TOP_Y)
    bot = (STRING_X, BOT_Y)
    nock = (STRING_X - pull, MID_Y)
    if pull < 0.5:
        d.line(
            [(p(top[0]), p(top[1])), (p(bot[0]), p(bot[1]))],
            fill=hp.IRON_HI,
            width=max(1, int(hp.px(0.9))),
        )
    else:
        d.line(
            [(p(top[0]), p(top[1])), (p(nock[0]), p(nock[1])), (p(bot[0]), p(bot[1]))],
            fill=hp.IRON_HI,
            width=max(1, int(hp.px(0.9))),
            joint="curve",
        )
        # Nocking point bead.
        d.ellipse(
            (p(nock[0] - 1.6), p(nock[1] - 1.6), p(nock[0] + 1.6), p(nock[1] + 1.6)),
            fill=hp.STEEL_EDGE,
        )
    return nock


def _params(anim: str, frame_idx: int, nframes: int) -> Tuple[float, float]:
    """Return (limb_depth, string_pull) for this frame."""
    if anim == "idle":
        return BOW_DEPTH, 0.0
    t = ease_in_out(frame_idx / max(1, nframes - 1))
    pull = MAX_PULL * t
    # Limbs bend a little deeper as the string stores energy.
    depth = BOW_DEPTH + 5.0 * t
    return depth, pull


def _draw_frame(anim: str, frame_idx: int, nframes: int) -> Image.Image:
    if anim not in ("idle", "draw"):
        raise ValueError(f"unknown animation: {anim}")
    depth, pull = _params(anim, frame_idx, nframes)
    canvas = hp.new_super(FRAME_SIZE)
    d = ImageDraw.Draw(canvas, "RGBA")
    _draw_string(d, depth, pull)
    _draw_bow(d, depth)
    return hp.finalize(canvas, FRAME_SIZE)


def _frame_meta(anim: str, frame_idx: int, nframes: int) -> dict:
    depth, pull = _params(anim, frame_idx, nframes)
    nock = (STRING_X - pull, MID_Y)
    return hp.anchor_meta(
        {
            "grip": (STRING_X + depth, MID_Y),
            "nock": (nock[0], nock[1]),
            "top_tip": (STRING_X, TOP_Y),
            "bottom_tip": (STRING_X, BOT_Y),
        }
    )


ACTOR_METADATA = {
    "actor": {"character_id": "prop_hunting_bow", "display_name": "Hunting Bow"},
    "body": {
        "body_plan": "Prop",
        "body_kind": "HeldWeapon",
        "mass_class": "Light",
        "locomotion_hint": "Stationary",
        "traits": ["prop", "weapon", "bow", "ranged", "held"],
    },
    "brain": {"default_preset": "stand_still"},
    "actions": {"default_preset": "peaceful"},
    "animation_bindings": {
        "default": {"animation": "idle", "events": []},
        "action.ranged.draw": {
            "animation": "draw",
            "events": [
                {
                    "t": 1.0,
                    "event": "projectile_release",
                    "source": f"{TARGET_NAME}.nock",
                }
            ],
        },
    },
    "sockets": {
        "grip": {
            "source": f"{TARGET_NAME}.geometry",
            "point": {"x": STRING_X + BOW_DEPTH, "y": MID_Y},
        },
        "nock": {
            "source": f"{TARGET_NAME}.geometry",
            "point": {"x": STRING_X, "y": MID_Y},
        },
    },
    "tags": ["prop", "weapon", "bow", "ranged", "held"],
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
