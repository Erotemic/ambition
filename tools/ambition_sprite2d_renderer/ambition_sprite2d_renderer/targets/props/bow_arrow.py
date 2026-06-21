from __future__ import annotations

"""Arrow projectile fired by the ``hunting_bow`` prop.

A thin wooden shaft with a steel broadhead at the point end and three
feathered fletches at the nock end. Rendered AXIS-ALIGNED with the
point facing RIGHT (+X). The game spawns it at the bow's ``nock``
anchor and translates it along ``forward``, rotating the sprite to the
flight direction — so no spin frames are baked in. See
``_held_prop_common`` for the shared authoring contract, and
``hunting_bow`` for the launcher.

Animations:

- ``idle``: 1-frame straight arrow. The flight projectile and the
  nocked-on-the-string arrow are the same image.
"""

from pathlib import Path
from typing import List, Tuple

from PIL import Image, ImageDraw

from ...authoring.tackon_sheet import build_sheet
from . import _held_prop_common as hp

TARGET_NAME = "bow_arrow"
SHEET_FILES = (
    f"{TARGET_NAME}_spritesheet.png",
    f"{TARGET_NAME}_spritesheet.yaml",
    f"{TARGET_NAME}_spritesheet.ron",
    f"{TARGET_NAME}_actor.ron",
)

FRAME_SIZE = (176, 32)
ROWS: List[Tuple[str, int, int]] = [("idle", 1, 200)]

CY = FRAME_SIZE[1] * 0.5
NOCK_X = 12.0  # back end (string contact)
SHAFT_END_X = 150.0  # where shaft meets the head
TIP_X = 166.0  # broadhead point
SHAFT_HALF = 1.6


def _draw_arrow(d: ImageDraw.ImageDraw) -> None:
    p = hp.px
    # Shaft.
    d.polygon(
        [
            (p(NOCK_X), p(CY - SHAFT_HALF)),
            (p(SHAFT_END_X), p(CY - SHAFT_HALF)),
            (p(SHAFT_END_X), p(CY + SHAFT_HALF)),
            (p(NOCK_X), p(CY + SHAFT_HALF)),
        ],
        fill=hp.WOOD,
        outline=hp.OUTLINE,
    )
    d.line(
        [
            (p(NOCK_X + 1), p(CY - SHAFT_HALF * 0.3)),
            (p(SHAFT_END_X - 1), p(CY - SHAFT_HALF * 0.3)),
        ],
        fill=hp.WOOD_HI,
        width=max(1, int(hp.px(0.5))),
    )

    # Nock notch + three fletches at the back.
    d.line(
        [(p(NOCK_X), p(CY - 2.4)), (p(NOCK_X), p(CY + 2.4))],
        fill=hp.OUTLINE,
        width=max(1, int(hp.px(0.8))),
    )
    for sign, vane_h in ((-1, 7.0), (1, 7.0), (0, 0.0)):
        if sign == 0:
            continue
        d.polygon(
            [
                (p(NOCK_X + 2), p(CY + sign * SHAFT_HALF)),
                (p(NOCK_X + 24), p(CY + sign * vane_h)),
                (p(NOCK_X + 30), p(CY + sign * (vane_h - 1.0))),
                (p(NOCK_X + 18), p(CY + sign * SHAFT_HALF)),
            ],
            fill=hp.FLETCH,
            outline=hp.OUTLINE,
        )
        d.line(
            [
                (p(NOCK_X + 6), p(CY + sign * (SHAFT_HALF + 1.0))),
                (p(NOCK_X + 26), p(CY + sign * (vane_h - 1.5))),
            ],
            fill=hp.FLETCH_HI,
            width=max(1, int(hp.px(0.5))),
        )

    # Steel broadhead.
    head = [
        (SHAFT_END_X, CY - 1.4),
        (SHAFT_END_X + 4, CY - 6.0),
        (TIP_X, CY),
        (SHAFT_END_X + 4, CY + 6.0),
        (SHAFT_END_X, CY + 1.4),
    ]
    d.polygon([(p(x), p(y)) for (x, y) in head], fill=hp.IRON_HI, outline=hp.OUTLINE)
    d.line(
        [(p(SHAFT_END_X + 4), p(CY - 6.0)), (p(TIP_X), p(CY))],
        fill=hp.STEEL_EDGE,
        width=max(1, int(hp.px(0.8))),
    )


def _draw_frame(anim: str, frame_idx: int, nframes: int) -> Image.Image:
    del frame_idx, nframes
    if anim != "idle":
        raise ValueError(f"unknown animation: {anim}")
    canvas = hp.new_super(FRAME_SIZE)
    d = ImageDraw.Draw(canvas, "RGBA")
    _draw_arrow(d)
    return hp.finalize(canvas, FRAME_SIZE)


def _frame_meta(anim: str, frame_idx: int, nframes: int) -> dict:
    del anim, frame_idx, nframes
    return hp.anchor_meta(
        {
            "nock": (NOCK_X, CY),
            "tip": (TIP_X, CY),
        }
    )


ACTOR_METADATA = {
    "actor": {"character_id": "prop_bow_arrow", "display_name": "Arrow"},
    "body": {
        "body_plan": "Prop",
        "body_kind": "Projectile",
        "mass_class": "Light",
        "locomotion_hint": "Flying",
        "traits": ["prop", "weapon", "arrow", "ammo", "projectile"],
    },
    "brain": {"default_preset": "stand_still"},
    "actions": {"default_preset": "peaceful"},
    "animation_bindings": {"default": {"animation": "idle", "events": []}},
    "sockets": {
        "nock": {"source": f"{TARGET_NAME}.geometry", "point": {"x": NOCK_X, "y": CY}},
        "tip": {"source": f"{TARGET_NAME}.geometry", "point": {"x": TIP_X, "y": CY}},
    },
    "tags": ["prop", "weapon", "arrow", "projectile"],
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
