from __future__ import annotations

"""Shared geometry for the portal gun — a compact sci-fi emitter pistol.

A chunky light-alloy body with a finned heat-sink along the top, an
angled grip, and a circular lens emitter at the muzzle. Rendered
AXIS-ALIGNED with the muzzle facing RIGHT (+X). The emitter glow + a
thin energy stripe are drawn in the *mode accent* color so the two
variants (`portal_gun_blue` / `portal_gun_orange`) read as distinct
blue and orange guns while sharing one body. The game shows whichever
matches the active portal color.

This is a `_`-prefixed helper (no top-level ``render``) so the target
registry does not auto-register it; `portal_gun_blue` / `portal_gun_orange`
call :func:`build`.
"""

import math
from pathlib import Path
from typing import List, Tuple

from PIL import Image, ImageDraw

from ...authoring.tackon_sheet import build_sheet
from . import _held_prop_common as hp

RGBA = Tuple[int, int, int, int]

FRAME_SIZE = (180, 96)
ROWS: List[Tuple[str, int, int]] = [("idle", 4, 140)]

CY = FRAME_SIZE[1] * 0.5
GRIP_X = 64.0
MUZZLE_X = 158.0
LENS_R = 13.0

ALLOY_DARK = (120, 130, 146, 255)
ALLOY = (168, 178, 194, 255)
ALLOY_HI = (212, 220, 232, 255)


def _draw_body(d: ImageDraw.ImageDraw, accent: RGBA) -> None:
    p = hp.px
    d.rounded_rectangle(
        (p(48), p(CY - 13), p(146), p(CY + 13)),
        radius=p(8),
        fill=ALLOY,
        outline=hp.OUTLINE,
        width=max(1, int(p(0.8))),
    )
    d.rounded_rectangle(
        (p(50), p(CY - 11), p(144), p(CY + 11)), radius=p(7), fill=ALLOY
    )
    d.rounded_rectangle(
        (p(54), p(CY - 10), p(140), p(CY - 4)), radius=p(3), fill=ALLOY_HI
    )
    # Thin glowing energy stripe along the body in the mode accent color.
    d.line(
        [(p(58), p(CY + 6)), (p(138), p(CY + 6))],
        fill=accent,
        width=max(1, int(p(1.6))),
    )


def _draw_heatsink(d: ImageDraw.ImageDraw, glint_t: float) -> None:
    p = hp.px
    fin_x0, fin_x1 = 62.0, 132.0
    n = 7
    for i in range(n):
        x = fin_x0 + (fin_x1 - fin_x0) * (i / (n - 1))
        d.line(
            [(p(x), p(CY - 18)), (p(x), p(CY - 11))],
            fill=ALLOY_DARK,
            width=max(1, int(p(1.4))),
        )
    gx = fin_x0 + (fin_x1 - fin_x0) * glint_t
    d.line(
        [(p(gx), p(CY - 18)), (p(gx), p(CY - 11))],
        fill=hp.STEEL_EDGE,
        width=max(1, int(p(1.8))),
    )


def _draw_grip(d: ImageDraw.ImageDraw) -> None:
    p = hp.px
    grip = [
        (GRIP_X + 12, CY + 10),
        (GRIP_X - 12, CY + 10),
        (GRIP_X - 18, CY + 34),
        (GRIP_X + 4, CY + 34),
    ]
    d.polygon([(p(x), p(y)) for (x, y) in grip], fill=ALLOY_DARK, outline=hp.OUTLINE)
    for i in range(3):
        y = CY + 16 + i * 6
        d.line(
            [(p(GRIP_X - 14 - i * 1.4), p(y)), (p(GRIP_X + 6 - i * 0.6), p(y))],
            fill=hp.OUTLINE,
            width=max(1, int(p(0.7))),
        )
    d.arc(
        (p(GRIP_X + 4), p(CY + 8), p(GRIP_X + 26), p(CY + 30)),
        start=300,
        end=80,
        fill=hp.OUTLINE,
        width=max(1, int(p(1.0))),
    )


def _draw_emitter(d: ImageDraw.ImageDraw, pulse: float, glow: RGBA, core: RGBA) -> None:
    p = hp.px
    d.ellipse(
        (
            p(MUZZLE_X - LENS_R - 4),
            p(CY - LENS_R - 4),
            p(MUZZLE_X + LENS_R + 4),
            p(CY + LENS_R + 4),
        ),
        fill=ALLOY,
        outline=hp.OUTLINE,
        width=max(1, int(p(0.9))),
    )
    r_glow = LENS_R * (0.86 + 0.14 * pulse)
    d.ellipse(
        (p(MUZZLE_X - r_glow), p(CY - r_glow), p(MUZZLE_X + r_glow), p(CY + r_glow)),
        fill=glow,
    )
    r_core = LENS_R * (0.42 + 0.18 * pulse)
    d.ellipse(
        (p(MUZZLE_X - r_core), p(CY - r_core), p(MUZZLE_X + r_core), p(CY + r_core)),
        fill=core,
    )
    d.ellipse(
        (
            p(MUZZLE_X - 2.2),
            p(CY - LENS_R * 0.4 - 2.2),
            p(MUZZLE_X + 2.2),
            p(CY - LENS_R * 0.4 + 2.2),
        ),
        fill=(255, 255, 255, 235),
    )


def _frame_meta(anim: str, frame_idx: int, nframes: int) -> dict:
    del anim, frame_idx, nframes
    return hp.anchor_meta({"grip": (GRIP_X, CY + 18), "muzzle": (MUZZLE_X, CY)})


def build(
    out_dir: str | Path,
    target_name: str,
    glow: RGBA,
    core: RGBA,
    accent: RGBA,
    actor_id: str,
    display: str,
) -> List[Path]:
    out_dir = Path(out_dir)
    out_dir.mkdir(parents=True, exist_ok=True)

    def _draw_frame(anim: str, frame_idx: int, nframes: int) -> Image.Image:
        if anim != "idle":
            raise ValueError(f"unknown animation: {anim}")
        t = frame_idx / max(1, nframes - 1)
        pulse = 0.5 - 0.5 * math.cos(t * 2.0 * math.pi)
        canvas = hp.new_super(FRAME_SIZE)
        d = ImageDraw.Draw(canvas, "RGBA")
        _draw_grip(d)
        _draw_body(d, accent)
        _draw_heatsink(d, t)
        _draw_emitter(d, pulse, glow, core)
        return hp.finalize(canvas, FRAME_SIZE)

    actor_metadata = {
        "actor": {"character_id": actor_id, "display_name": display},
        "body": {
            "body_plan": "Prop",
            "body_kind": "Device",
            "mass_class": "Light",
            "locomotion_hint": "Held",
            "traits": ["prop", "weapon", "device", "portal_gun", "sci_fi", "emitter"],
        },
        "brain": {"default_preset": "stand_still"},
        "actions": {"default_preset": "peaceful"},
        "animation_bindings": {"default": {"animation": "idle", "events": []}},
        "sockets": {
            "grip": {
                "source": f"{target_name}.geometry",
                "point": {"x": GRIP_X, "y": CY + 18},
            },
            "muzzle": {
                "source": f"{target_name}.geometry",
                "point": {"x": MUZZLE_X, "y": CY},
            },
        },
        "tags": ["prop", "weapon", "device", "portal_gun"],
    }

    outputs = build_sheet(
        target=target_name,
        rows=ROWS,
        render_fn=_draw_frame,
        out_dir=out_dir,
        frame_size=FRAME_SIZE,
        frame_meta_fn=_frame_meta,
        auto_crop=True,
        crop_margin=3,
        actor_metadata=actor_metadata,
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
