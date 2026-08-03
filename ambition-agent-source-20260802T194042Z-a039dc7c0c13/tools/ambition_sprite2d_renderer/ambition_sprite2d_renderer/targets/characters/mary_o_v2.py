"""Mary-O v2: an ambitious second-draft procedural character family.

This additive module keeps a parallel v2 Mary family on the same unified
surface as other modern character sheets:

- a single drawing core with form specs + palette swaps
- module-level ``TARGETS`` so small / tall / fire forms stay colocated
- ``build_sheet`` for all spritesheet / YAML / RON / actor sidecars

It preserves the accepted animation and transition vocabulary while redrafting
the visual components: clearer cap emblem, integrated bib and skirt, puff
sleeves, stronger boots, intentional wing motifs, and a more ornate fire form.
"""

from __future__ import annotations

import copy
import math
from dataclasses import dataclass, replace
from pathlib import Path
from typing import Dict, List, Tuple

from PIL import Image

from ...authoring.sheet_build import build_sheet
from ..super_mary_o_common import (
    OUTLINE,
    WHITE,
    MaryPalette,
    bottom_center_canvas,
    rasterize_logical,
)

TARGET_BASE = "mary_o_v2"
FRAME_SIZE = (80, 96)
LOGICAL_SIZE = (24, 32)
SCALE = 3
LABEL_WIDTH = 122

MARY_NORMAL = MaryPalette(
    cap=(188, 48, 92, 255),
    shirt=(223, 83, 76, 255),
    overalls=(38, 135, 160, 255),
    buttons=(255, 220, 91, 255),
    gloves=(248, 245, 239, 255),
    hair=(94, 54, 36, 255),
    skin=(251, 194, 148, 255),
    shoes=(96, 61, 42, 255),
    accent=(255, 155, 189, 255),
)

MARY_FIRE = MaryPalette(
    cap=(236, 88, 58, 255),
    shirt=(242, 112, 56, 255),
    overalls=(246, 242, 232, 255),
    buttons=(255, 190, 75, 255),
    gloves=(255, 251, 246, 255),
    hair=(98, 55, 35, 255),
    skin=(252, 198, 152, 255),
    shoes=(103, 65, 43, 255),
    accent=(255, 219, 108, 255),
)

MARY_FIRE_FLASH = MaryPalette(
    cap=(255, 176, 120, 255),
    shirt=(255, 237, 162, 255),
    overalls=(255, 252, 248, 255),
    buttons=(255, 232, 152, 255),
    gloves=(255, 255, 250, 255),
    hair=MARY_NORMAL.hair,
    skin=MARY_NORMAL.skin,
    shoes=(168, 116, 76, 255),
    accent=(255, 242, 178, 255),
)

MARY_FIRE_BLAST = MaryPalette(
    cap=(255, 246, 208, 255),
    shirt=(255, 255, 252, 255),
    overalls=(255, 255, 255, 255),
    buttons=(255, 240, 174, 255),
    gloves=(255, 255, 255, 255),
    hair=MARY_NORMAL.hair,
    skin=MARY_NORMAL.skin,
    shoes=(255, 226, 160, 255),
    accent=(255, 228, 148, 255),
)

RIBBON_PINK = (231, 120, 170, 255)
BROOCH_GOLD = (255, 208, 84, 255)
BROOCH_LIGHT = (255, 244, 205, 255)
EMBER_ORANGE = (255, 159, 76, 255)
EMBER_CORE = (255, 240, 190, 255)
BLUSH = (244, 157, 146, 255)
LIP = (178, 89, 91, 255)
WING_PEARL = (255, 246, 235, 255)
AURA_PINK = (244, 162, 202, 255)
AURA_GOLD = (255, 213, 118, 255)

# A form-transition clip is authored on the sheet of the form it ARRIVES AT, so
# the runtime plays it from the identity it has already switched to and nothing
# has to defer a swap to show it. Read the three lists together and each sheet
# answers "how did I get here":
#
#   short:  shrink (from tall), big_shrink (from fire)
#   tall:   grow   (from short), shrink     (from fire)
#   fire:   transform (from tall)
#
# The frames themselves draw whatever silhouettes the transition needs — the
# short sheet's `shrink` opens on the TALL body — so hosting is about who OWNS
# the clip, not about which forms appear in it.
SHORT_ROWS: List[Tuple[str, int, int]] = [
    ("idle", 1, 160),
    ("death", 1, 120),
    ("walk", 3, 95),
    ("jump", 1, 120),
    ("skid", 1, 110),
    ("climb", 2, 120),
    ("swim", 4, 100),
    ("shrink", 4, 85),
    ("big_shrink", 8, 85),
]

TALL_ROWS: List[Tuple[str, int, int]] = [
    ("idle", 1, 160),
    ("death", 1, 120),
    ("walk", 3, 95),
    ("jump", 1, 120),
    ("skid", 1, 110),
    ("crouch", 1, 120),
    ("climb", 2, 120),
    ("swim", 6, 100),
    ("grow", 4, 70),
    ("shrink", 6, 85),
]

FIRE_ROWS: List[Tuple[str, int, int]] = [
    ("idle", 1, 160),
    ("death", 1, 120),
    ("walk", 3, 95),
    ("jump", 1, 120),
    ("skid", 1, 110),
    ("crouch", 1, 120),
    ("climb", 2, 120),
    ("swim", 6, 100),
    ("fireball", 1, 120),
    ("transform", 11, 80),
]


@dataclass(frozen=True)
class Pose:
    bob: float = 0.0
    body_lean: float = 0.0
    head_dx: float = 0.0
    head_dy: float = 0.0
    arm_front_dx: float = 0.0
    arm_front_dy: float = 0.0
    arm_back_dx: float = 0.0
    arm_back_dy: float = 0.0
    leg_front_dx: float = 0.0
    leg_front_dy: float = 0.0
    leg_back_dx: float = 0.0
    leg_back_dy: float = 0.0
    arm_front_angle: float | None = None
    arm_back_angle: float | None = None
    leg_front_angle: float | None = None
    leg_back_angle: float | None = None
    crouch: float = 0.0
    mode: str = "side"


@dataclass(frozen=True)
class FormSpec:
    target_name: str
    display_name: str
    body_height: float
    leg_height: float
    body_width: float
    palette: MaryPalette
    power: str
    tall: bool
    magic_stage: int
    rows: List[Tuple[str, int, int]]


SHORT_FORM = FormSpec(
    target_name=TARGET_BASE,
    display_name="Mary-O v2",
    body_height=4.8,
    leg_height=4.8,
    body_width=8.5,
    palette=MARY_NORMAL,
    power="short",
    tall=False,
    magic_stage=0,
    rows=SHORT_ROWS,
)

TALL_FORM = FormSpec(
    target_name=f"{TARGET_BASE}_tall",
    display_name="Mary-O v2 Tall",
    body_height=9.5,
    leg_height=8.6,
    body_width=9.4,
    palette=MARY_NORMAL,
    power="tall",
    tall=True,
    magic_stage=1,
    rows=TALL_ROWS,
)

FIRE_FORM = FormSpec(
    target_name=f"{TARGET_BASE}_fire",
    display_name="Mary-O v2 Fire",
    body_height=9.7,
    leg_height=8.5,
    body_width=9.6,
    palette=MARY_FIRE,
    power="fire",
    tall=True,
    magic_stage=2,
    rows=FIRE_ROWS,
)


def _lerp_rgba(a: tuple[int, int, int, int], b: tuple[int, int, int, int], t: float) -> tuple[int, int, int, int]:
    t = max(0.0, min(1.0, t))
    return tuple(int(round(x + (y - x) * t)) for x, y in zip(a, b))


def _mix_outfit_palette(base: MaryPalette, target: MaryPalette, t: float) -> MaryPalette:
    return MaryPalette(
        cap=_lerp_rgba(base.cap, target.cap, t),
        shirt=_lerp_rgba(base.shirt, target.shirt, t),
        overalls=_lerp_rgba(base.overalls, target.overalls, t),
        buttons=_lerp_rgba(base.buttons, target.buttons, t),
        gloves=_lerp_rgba(base.gloves, target.gloves, t),
        hair=base.hair,
        skin=base.skin,
        shoes=_lerp_rgba(base.shoes, target.shoes, t),
        accent=_lerp_rgba(base.accent, target.accent, t),
    )


def _form_with_palette(form: FormSpec, palette: MaryPalette) -> FormSpec:
    return replace(form, palette=palette)


def _transition_form(form: FormSpec, palette: MaryPalette, *, stage: float | None = None, power: str | None = None) -> FormSpec:
    updates = {"palette": palette}
    if stage is not None:
        updates["magic_stage"] = stage
    if power is not None:
        updates["power"] = power
    return replace(form, **updates)


def _magic_stage_value(form: FormSpec) -> float:
    return float(form.magic_stage)


def _fire_transition_t(form: FormSpec) -> float:
    return max(0.0, min(1.0, _magic_stage_value(form) - 1.0))


def _fire_accessory_t(form: FormSpec) -> float:
    return max(0.0, min(1.0, (_magic_stage_value(form) - 1.35) / 0.65))


SHORT_POSES: Dict[str, List[Pose]] = {
    "idle": [Pose()],
    "death": [Pose(mode="dead", bob=-4.2)],
    "walk": [
        Pose(
            body_lean=0.5,
            arm_front_dx=1.2,
            arm_front_dy=-1.0,
            arm_back_dx=-0.9,
            arm_back_dy=1.0,
            leg_front_dx=1.3,
            leg_back_dx=-0.9,
            leg_back_dy=1.0,
        ),
        Pose(
            bob=0.4,
            arm_front_dy=0.6,
            arm_back_dy=0.2,
            leg_front_dx=0.2,
            leg_back_dx=-0.2,
        ),
        Pose(
            body_lean=-0.4,
            arm_front_dx=-0.9,
            arm_front_dy=1.0,
            arm_back_dx=1.1,
            arm_back_dy=-1.1,
            leg_front_dx=-0.8,
            leg_front_dy=1.0,
            leg_back_dx=1.4,
        ),
    ],
    "jump": [
        Pose(
            bob=-1.8,
            arm_front_dx=0.6,
            arm_front_dy=-0.4,
            arm_back_dx=-0.5,
            arm_back_dy=0.3,
            arm_front_angle=145,
            arm_back_angle=-18,
            leg_front_angle=42,
            leg_back_angle=-30,
        ),
    ],
    "skid": [
        Pose(
            mode="lookback",
            body_lean=-1.6,
            head_dx=-1.1,
            arm_front_dx=0.5,
            arm_front_dy=-0.5,
            arm_back_dx=0.8,
            arm_back_dy=1.0,
            leg_front_angle=-36,
            leg_back_angle=-58,
            leg_front_dy=0.5,
            leg_back_dy=1.0,
        ),
    ],
    "climb": [
        Pose(mode="climb", bob=-0.2, arm_front_angle=88, arm_back_angle=82, leg_front_angle=92, leg_back_angle=86),
        Pose(mode="climb", bob=0.2, arm_front_angle=126, arm_back_angle=112, leg_front_angle=54, leg_back_angle=68),
    ],
    "swim": [
        Pose(mode="swim", bob=-0.7, arm_front_angle=125, arm_back_angle=45, leg_front_angle=25, leg_back_angle=-12),
        Pose(mode="swim", bob=-0.9, arm_front_angle=92, arm_back_angle=12, leg_front_angle=5, leg_back_angle=18),
        Pose(mode="swim", bob=-0.5, arm_front_angle=48, arm_back_angle=-25, leg_front_angle=-18, leg_back_angle=28),
        Pose(mode="swim", bob=-0.8, body_lean=-0.2, arm_front_angle=8, arm_back_angle=78, leg_front_angle=16, leg_back_angle=-22),
    ],
}

TALL_LIKE_POSES: Dict[str, List[Pose]] = {
    "idle": [Pose()],
    "death": [Pose(mode="dead", bob=-4.4)],
    "walk": [
        Pose(
            body_lean=0.5,
            arm_front_dx=1.4,
            arm_front_dy=-1.1,
            arm_back_dx=-1.0,
            arm_back_dy=1.1,
            leg_front_dx=1.4,
            leg_back_dx=-1.0,
            leg_back_dy=1.2,
        ),
        Pose(
            bob=0.4,
            arm_front_dy=0.7,
            arm_back_dy=0.2,
            leg_front_dx=0.3,
            leg_back_dx=-0.2,
        ),
        Pose(
            body_lean=-0.5,
            arm_front_dx=-1.0,
            arm_front_dy=1.1,
            arm_back_dx=1.2,
            arm_back_dy=-1.2,
            leg_front_dx=-0.8,
            leg_front_dy=1.1,
            leg_back_dx=1.5,
        ),
    ],
    "jump": [
        Pose(
            bob=-2.0,
            arm_front_dx=0.8,
            arm_front_dy=-0.5,
            arm_back_dx=-0.6,
            arm_back_dy=0.4,
            arm_front_angle=148,
            arm_back_angle=-22,
            leg_front_angle=45,
            leg_back_angle=-32,
        ),
    ],
    "skid": [
        Pose(
            mode="lookback",
            body_lean=-1.8,
            head_dx=-1.5,
            arm_front_dx=0.7,
            arm_front_dy=-0.5,
            arm_back_dx=1.0,
            arm_back_dy=1.1,
            leg_front_angle=-38,
            leg_back_angle=-62,
            leg_front_dy=0.6,
            leg_back_dy=1.2,
        ),
    ],
    "crouch": [
        Pose(
            mode="crouch",
            crouch=2.4,
            head_dx=0.6,
            arm_front_dx=0.8,
            arm_back_dx=-0.4,
            leg_front_dx=0.3,
            leg_back_dx=-0.2,
        )
    ],
    "climb": [
        Pose(mode="climb", bob=-0.2, arm_front_angle=88, arm_back_angle=82, leg_front_angle=92, leg_back_angle=86),
        Pose(mode="climb", bob=0.2, arm_front_angle=126, arm_back_angle=112, leg_front_angle=54, leg_back_angle=68),
    ],
    "swim": [
        Pose(mode="swim", bob=-0.6, arm_front_angle=132, arm_back_angle=52, leg_front_angle=30, leg_back_angle=-10),
        Pose(mode="swim", bob=-0.8, arm_front_angle=108, arm_back_angle=25, leg_front_angle=15, leg_back_angle=6),
        Pose(mode="swim", bob=-1.0, arm_front_angle=82, arm_back_angle=-8, leg_front_angle=-2, leg_back_angle=18),
        Pose(mode="swim", bob=-0.8, arm_front_angle=48, arm_back_angle=-35, leg_front_angle=-20, leg_back_angle=26),
        Pose(mode="swim", bob=-0.6, arm_front_angle=18, arm_back_angle=8, leg_front_angle=6, leg_back_angle=-16),
        Pose(mode="swim", bob=-0.7, body_lean=-0.2, arm_front_angle=2, arm_back_angle=88, leg_front_angle=22, leg_back_angle=-24),
    ],
    "fireball": [
        Pose(
            mode="fireball",
            body_lean=0.3,
            arm_front_angle=92,
            arm_back_angle=-12,
            leg_front_dx=0.8,
        )
    ],
}

ACTOR_METADATA_BASE = {
    "body": {
        "body_plan": "HumanoidBiped",
        "mass_class": "Light",
        "locomotion_hint": "Walk",
    },
    "capabilities": {
        "traversal": {
            "walk": True,
            "jump": {"height_px": 48, "distance_px": 80, "source": "super_mary_o"},
            "climb": None,
            "crawl": None,
            "fly": None,
            "swim": None,
            "use_lifts": True,
            "door_access": [],
        },
        "interactions": {"talk": None, "trade": None, "carry": True, "open_doors": []},
    },
    "brain": {"default_preset": "wanderer_puppy_slug"},
    "actions": {"default_preset": "peaceful_float"},
    "animation_bindings": {
        "default": {"animation": "idle", "events": []},
        "locomotion.walk": {"animation": "walk", "events": []},
        "locomotion.run": {"animation": "walk", "events": []},
        "locomotion.jump": {"animation": "jump", "events": []},
        "locomotion.fall": {"animation": "jump", "events": []},
        "locomotion.skid": {"animation": "skid", "events": []},
        "locomotion.climb": {"animation": "climb", "events": []},
        "locomotion.swim": {"animation": "swim", "events": []},
        "state.dead": {"animation": "death", "events": []},
    },
    "tags": ["hero", "platformer", "mary_o", "retro"],
}


def _outlined_rect(px, x1, y1, x2, y2, *, fill, inset: float = 0.5) -> None:
    px.rect(x1, y1, x2, y2, fill=OUTLINE)
    ix1, iy1 = x1 + inset, y1 + inset
    ix2, iy2 = x2 - inset, y2 - inset
    if ix2 <= ix1 or iy2 <= iy1:
        px.rect(x1, y1, x2, y2, fill=fill)
        return
    px.rect(ix1, iy1, ix2, iy2, fill=fill)


def _segment_quad(x1: float, y1: float, x2: float, y2: float, half_w: float) -> List[Tuple[float, float]]:
    dx = x2 - x1
    dy = y2 - y1
    dist = math.hypot(dx, dy) or 1.0
    ox = -dy / dist * half_w
    oy = dx / dist * half_w
    return [
        (x1 + ox, y1 + oy),
        (x2 + ox, y2 + oy),
        (x2 - ox, y2 - oy),
        (x1 - ox, y1 - oy),
    ]


def _draw_segment(px, x1: float, y1: float, x2: float, y2: float, *, half_w: float, fill) -> None:
    px.polygon(_segment_quad(x1, y1, x2, y2, half_w), fill=fill, outline=OUTLINE, width=0.55)


def _rotated_endpoint(pivot_x: float, pivot_y: float, angle_deg: float, length: float) -> Tuple[float, float]:
    radians = math.radians(angle_deg)
    return (
        pivot_x + math.sin(radians) * length,
        pivot_y + math.cos(radians) * length,
    )


def _draw_star(px, cx: float, cy: float, *, outer: float, inner: float, fill, outline=OUTLINE, width: float = 0.45) -> None:
    pts: List[Tuple[float, float]] = []
    for idx in range(10):
        angle = math.radians(-90 + idx * 36)
        radius = outer if idx % 2 == 0 else inner
        pts.append((cx + math.cos(angle) * radius, cy + math.sin(angle) * radius))
    px.polygon(pts, fill=fill, outline=outline, width=width)


def _draw_ribbon_tail(px, x: float, y: float, *, flip: bool, fill, long: bool = False) -> None:
    sign = -1.0 if flip else 1.0
    loop_dx = 1.5 * sign
    px.polygon(
        [(x, y), (x + loop_dx, y - 1.0), (x + loop_dx * 1.2, y + 0.9)],
        fill=fill,
        outline=OUTLINE,
        width=0.45,
    )
    px.polygon(
        [(x, y), (x + loop_dx, y + 1.0), (x + loop_dx * 1.1, y + 2.1)],
        fill=fill,
        outline=OUTLINE,
        width=0.45,
    )
    tail_len = 4.2 if long else 3.0
    px.polygon(
        [(x, y + 0.4), (x + sign * 0.9, y + 2.0), (x + sign * 0.4, y + tail_len), (x - sign * 0.3, y + 2.6)],
        fill=fill,
        outline=OUTLINE,
        width=0.45,
    )


def _draw_rotated_arm(
    px,
    shoulder_x: float,
    shoulder_y: float,
    *,
    front: bool,
    form: FormSpec,
    angle_deg: float,
    length: float = 4.4,
) -> None:
    pal = form.palette
    hand_fill = pal.gloves if form.power != "normal" else pal.skin
    end_x, end_y = _rotated_endpoint(shoulder_x, shoulder_y, angle_deg, length)
    _draw_segment(px, shoulder_x, shoulder_y, end_x, end_y, half_w=0.8, fill=pal.shirt)
    if form.magic_stage >= 1:
        cuff_fill = pal.accent if form.magic_stage == 1 else pal.buttons
        cuff_x, cuff_y = _rotated_endpoint(shoulder_x, shoulder_y, angle_deg, max(0.0, length - 0.9))
        _draw_segment(px, cuff_x, cuff_y, end_x, end_y, half_w=0.9, fill=cuff_fill)
    _outlined_rect(px, end_x - 1.0, end_y - 0.9, end_x + 1.0, end_y + 0.9, fill=hand_fill, inset=0.15)
    _draw_hand_outline(px, end_x - 1.05, end_y - 0.95, end_x + 1.05, end_y + 0.95)


def _draw_rotated_leg(
    px,
    hip_x: float,
    hip_y: float,
    *,
    form: FormSpec,
    angle_deg: float,
    length: float = 5.4,
    front: bool = False,
) -> None:
    pal = form.palette
    end_x, end_y = _rotated_endpoint(hip_x, hip_y, angle_deg, length)
    _draw_segment(px, hip_x, hip_y, end_x, end_y, half_w=0.95, fill=pal.overalls)
    shoe_dir = 1.0 if math.sin(math.radians(angle_deg)) >= 0 else -1.0
    x1 = end_x - 0.5 if shoe_dir > 0 else end_x - 2.7
    x2 = end_x + 2.3 if shoe_dir > 0 else end_x + 0.5
    if form.magic_stage >= 1:
        cuff_fill = pal.accent if form.magic_stage == 1 else pal.buttons
        _outlined_rect(px, x1 + 0.2, end_y - 1.3, x2 - 0.2, end_y - 0.1, fill=cuff_fill, inset=0.15)
    _outlined_rect(px, x1, end_y - 0.4, x2, end_y + 1.0, fill=pal.shoes, inset=0.15)


def _draw_head_side(px, form: FormSpec, x: float, y: float, *, lookback: bool = False) -> None:
    pal = form.palette
    if lookback:
        px.polygon(
            [
                (x + 8.6, y + 3.2),
                (x + 12.8, y + 8.2),
                (x + 11.5, y + 13.8),
                (x + 8.1, y + 11.8),
            ],
            fill=pal.hair,
            outline=OUTLINE,
            width=0.75,
        )
        px.polygon(
            [
                (x + 1.9, y + 2.9),
                (x + 10.0, y + 3.2),
                (x + 9.1, y + 11.2),
                (x + 2.5, y + 10.7),
            ],
            fill=pal.hair,
            outline=OUTLINE,
            width=0.75,
        )
        if form.magic_stage >= 1:
            _draw_ribbon_tail(px, x + 10.7, y + 4.3, flip=False, fill=RIBBON_PINK, long=form.magic_stage >= 2)
            if form.magic_stage >= 2:
                px.polygon(
                    [(x + 11.0, y + 1.4), (x + 13.2, y + 2.6), (x + 11.8, y + 4.1)],
                    fill=pal.buttons,
                    outline=OUTLINE,
                    width=0.4,
                )
        px.ellipse(x + 1.0, y + 0.1, x + 10.6, y + 5.0, fill=pal.cap, outline=OUTLINE, width=0.7)
        _outlined_rect(px, x + 0.8, y + 3.2, x + 10.2, y + 4.8, fill=pal.accent, inset=0.25)
        px.polygon(
            [(x + 10.9, y + 3.6), (x + 12.8, y + 5.7), (x + 10.6, y + 5.9)],
            fill=pal.accent,
            outline=OUTLINE,
            width=0.5,
        )
        if form.magic_stage >= 1:
            _draw_star(px, x + 5.2, y + 2.4, outer=1.4 if form.magic_stage >= 2 else 1.1, inner=0.55, fill=BROOCH_GOLD)
        _outlined_rect(px, x + 2.1, y + 4.9, x + 9.1, y + 11.1, fill=pal.skin)
        px.polygon(
            [(x + 6.3, y + 4.8), (x + 9.0, y + 4.8), (x + 8.1, y + 7.2)],
            fill=pal.hair,
            outline=OUTLINE,
            width=0.35,
        )
        eye_x = x + 3.3
        _outlined_rect(px, eye_x, y + 6.2, eye_x + 1.3, y + 7.3, fill=WHITE, inset=0.2)
        _outlined_rect(px, eye_x + 0.2, y + 6.5, eye_x + 0.6, y + 7.0, fill=OUTLINE, inset=0.0)
        px.line([(x + 4.5, y + 6.0), (x + 3.6, y + 5.7)], fill=OUTLINE, width=0.35)
        px.rect(x + 3.4, y + 8.6, x + 4.8, y + 9.3, fill=LIP)
        px.rect(x + 2.8, y + 7.7, x + 3.8, y + 8.4, fill=BLUSH)
        if form.magic_stage >= 2:
            _draw_star(px, x + 8.7, y + 6.0, outer=0.7, inner=0.3, fill=BROOCH_LIGHT, width=0.25)
        return

    px.polygon(
        [
            (x + 1.0, y + 3.2),
            (x - 3.3, y + 8.3),
            (x - 2.1, y + 13.8),
            (x + 1.6, y + 11.9),
        ],
        fill=pal.hair,
        outline=OUTLINE,
        width=0.75,
    )
    px.polygon(
        [
            (x + 2.0, y + 2.9),
            (x + 10.1, y + 3.2),
            (x + 9.0, y + 11.2),
            (x + 1.5, y + 10.6),
        ],
        fill=pal.hair,
        outline=OUTLINE,
        width=0.75,
    )
    if form.magic_stage >= 1:
        _draw_ribbon_tail(px, x + 1.2, y + 4.2, flip=True, fill=RIBBON_PINK, long=form.magic_stage >= 2)
        if form.magic_stage >= 2:
            px.polygon(
                [(x - 0.8, y + 1.5), (x - 2.8, y + 2.7), (x - 1.5, y + 4.3)],
                fill=pal.buttons,
                outline=OUTLINE,
                width=0.4,
            )
    px.ellipse(x + 1.0, y + 0.1, x + 10.6, y + 5.0, fill=pal.cap, outline=OUTLINE, width=0.7)
    _outlined_rect(px, x + 1.4, y + 3.2, x + 10.8, y + 4.8, fill=pal.accent, inset=0.25)
    px.polygon(
        [(x + 0.8, y + 3.6), (x - 1.3, y + 5.7), (x + 1.1, y + 5.9)],
        fill=pal.accent,
        outline=OUTLINE,
        width=0.5,
    )
    if form.magic_stage >= 1:
        _draw_star(px, x + 6.3, y + 2.4, outer=1.4 if form.magic_stage >= 2 else 1.1, inner=0.55, fill=BROOCH_GOLD)
    _outlined_rect(px, x + 2.5, y + 4.9, x + 9.5, y + 11.1, fill=pal.skin)
    px.polygon(
        [(x + 2.4, y + 4.8), (x + 5.1, y + 4.8), (x + 3.2, y + 7.2)],
        fill=pal.hair,
        outline=OUTLINE,
        width=0.35,
    )
    eye_x = x + 6.1
    _outlined_rect(px, eye_x, y + 6.2, eye_x + 1.3, y + 7.3, fill=WHITE, inset=0.2)
    _outlined_rect(px, eye_x + 0.8, y + 6.5, eye_x + 1.2, y + 7.0, fill=OUTLINE, inset=0.0)
    px.line([(x + 7.4, y + 6.1), (x + 8.2, y + 5.7)], fill=OUTLINE, width=0.35)
    px.rect(x + 7.4, y + 8.6, x + 8.8, y + 9.3, fill=LIP)
    px.rect(x + 8.0, y + 7.7, x + 9.0, y + 8.4, fill=BLUSH)
    if form.magic_stage >= 2:
        _draw_star(px, x + 2.8, y + 6.0, outer=0.7, inner=0.3, fill=BROOCH_LIGHT, width=0.25)


def _draw_head_front(px, form: FormSpec, x: float, y: float) -> None:
    pal = form.palette
    px.polygon(
        [(x + 1.5, y + 3.0), (x - 1.5, y + 9.5), (x + 1.0, y + 14.0), (x + 4.5, y + 11.2)],
        fill=pal.hair,
        outline=OUTLINE,
        width=0.75,
    )
    px.polygon(
        [(x + 8.5, y + 3.0), (x + 11.5, y + 9.5), (x + 9.0, y + 14.0), (x + 5.5, y + 11.2)],
        fill=pal.hair,
        outline=OUTLINE,
        width=0.75,
    )
    if form.magic_stage >= 1:
        _draw_ribbon_tail(px, x + 1.3, y + 4.4, flip=True, fill=RIBBON_PINK, long=form.magic_stage >= 2)
        _draw_ribbon_tail(px, x + 9.7, y + 4.4, flip=False, fill=RIBBON_PINK, long=form.magic_stage >= 2)
    px.ellipse(x + 0.6, y + 0.2, x + 10.4, y + 5.0, fill=pal.cap, outline=OUTLINE, width=0.7)
    _outlined_rect(px, x + 1.0, y + 3.3, x + 10.0, y + 4.9, fill=pal.accent, inset=0.25)
    if form.magic_stage >= 1:
        _draw_star(px, x + 5.4, y + 2.4, outer=1.5 if form.magic_stage >= 2 else 1.2, inner=0.6, fill=BROOCH_GOLD)
        if form.magic_stage >= 2:
            px.polygon(
                [(x + 0.8, y + 2.5), (x - 1.2, y + 3.3), (x + 0.1, y + 5.0)],
                fill=pal.buttons,
                outline=OUTLINE,
                width=0.35,
            )
            px.polygon(
                [(x + 10.2, y + 2.5), (x + 12.2, y + 3.3), (x + 10.9, y + 5.0)],
                fill=pal.buttons,
                outline=OUTLINE,
                width=0.35,
            )
    _outlined_rect(px, x + 2.0, y + 4.8, x + 9.0, y + 11.1, fill=pal.skin)
    px.polygon(
        [(x + 2.2, y + 4.6), (x + 8.8, y + 4.6), (x + 7.6, y + 6.2), (x + 3.4, y + 6.2)],
        fill=pal.hair,
        outline=OUTLINE,
        width=0.35,
    )
    _outlined_rect(px, x + 3.4, y + 6.5, x + 4.8, y + 7.6, fill=WHITE, inset=0.2)
    _outlined_rect(px, x + 6.2, y + 6.5, x + 7.6, y + 7.6, fill=WHITE, inset=0.2)
    _outlined_rect(px, x + 4.0, y + 6.8, x + 4.4, y + 7.3, fill=OUTLINE, inset=0.0)
    _outlined_rect(px, x + 6.8, y + 6.8, x + 7.2, y + 7.3, fill=OUTLINE, inset=0.0)
    px.line([(x + 5.4, y + 7.2), (x + 5.1, y + 8.6), (x + 5.8, y + 8.8)], fill=OUTLINE, width=0.35)
    px.rect(x + 4.2, y + 9.2, x + 6.8, y + 9.9, fill=LIP)
    px.rect(x + 2.6, y + 7.7, x + 3.6, y + 8.4, fill=BLUSH)
    px.rect(x + 7.4, y + 7.7, x + 8.4, y + 8.4, fill=BLUSH)


def _draw_body_side(px, form: FormSpec, x: float, y: float, crouch: float) -> None:
    pal = form.palette
    body_h = form.body_height - 0.55 * crouch
    body_w = form.body_width + 0.4 * min(crouch, 1.4)
    waist = y + body_h * 0.63
    if form.magic_stage >= 1:
        skirt_fill = pal.accent if form.magic_stage == 1 else pal.shirt
        hem_fill = pal.buttons if form.magic_stage == 1 else BROOCH_LIGHT
        px.polygon(
            [
                (x + 1.0, waist - 0.1),
                (x + 1.0 + body_w - 0.6, waist + 0.1),
                (x + 1.0 + body_w + 1.2, y + body_h + 1.9),
                (x + 0.5, y + body_h + 1.7),
            ],
            fill=skirt_fill,
            outline=OUTLINE,
            width=0.55,
        )
        px.line([(x + 1.5, y + body_h + 1.2), (x + 1.0 + body_w + 0.6, y + body_h + 1.2)], fill=hem_fill, width=0.6)
        px.polygon(
            [(x + 0.6, waist + 0.2), (x - 1.0, waist - 0.6), (x - 0.2, waist + 1.0)],
            fill=RIBBON_PINK if form.magic_stage == 1 else pal.buttons,
            outline=OUTLINE,
            width=0.35,
        )
        _outlined_rect(px, x + 4.1, y + body_h + 0.2, x + 5.0, y + body_h + 1.1, fill=pal.buttons, inset=0.18)
        _outlined_rect(px, x + 7.0, y + body_h + 0.2, x + 7.9, y + body_h + 1.1, fill=pal.buttons, inset=0.18)
    _outlined_rect(px, x + 1.0, y + 0.0, x + 1.0 + body_w, y + body_h, fill=pal.shirt)
    px.polygon(
        [
            (x + 2.0, y + 1.5),
            (x + 1.0 + body_w - 0.8, y + 1.5),
            (x + 1.0 + body_w, y + body_h + 0.9),
            (x + 1.0, y + body_h + 0.9),
        ],
        fill=pal.overalls,
        outline=OUTLINE,
        width=0.75,
    )
    px.line([(x + 2.3, y + 0.4), (x + 4.5, waist)], fill=pal.overalls, width=1.2)
    px.line([(x + 1.0 + body_w - 1.3, y + 0.4), (x + 6.3, waist)], fill=pal.overalls, width=1.2)
    px.line([(x + 2.0, waist), (x + 1.0 + body_w - 0.9, waist)], fill=OUTLINE, width=0.45)
    _outlined_rect(px, x + 3.5, y + 3.0, x + 4.5, y + 4.1, fill=pal.buttons, inset=0.2)
    _outlined_rect(px, x + 6.5, y + 3.0, x + 7.5, y + 4.1, fill=pal.buttons, inset=0.2)
    if form.magic_stage >= 1:
        _draw_star(px, x + 5.7, y + 2.3, outer=1.0 if form.magic_stage == 1 else 1.3, inner=0.45, fill=BROOCH_GOLD, width=0.35)
        px.polygon(
            [(x + 5.7, y + 2.9), (x + 4.6, y + 4.1), (x + 6.8, y + 4.1)],
            fill=RIBBON_PINK if form.magic_stage == 1 else pal.accent,
            outline=OUTLINE,
            width=0.3,
        )
        if form.magic_stage >= 2:
            px.polygon(
                [(x + 1.2, y + 1.2), (x - 0.9, y + 2.3), (x + 0.4, y + 5.4)],
                fill=pal.buttons,
                outline=OUTLINE,
                width=0.35,
            )
            px.polygon(
                [(x + 10.2, y + 1.0), (x + 12.0, y + 2.2), (x + 9.8, y + 5.6)],
                fill=pal.buttons,
                outline=OUTLINE,
                width=0.35,
            )
        _draw_suspender_fasteners_side(px, x, y, form)


def _draw_body_front(px, form: FormSpec, x: float, y: float, *, crouch: float = 0.0) -> None:
    pal = form.palette
    body_h = form.body_height - 0.55 * crouch
    body_w = form.body_width + 0.4 * min(crouch, 1.4)
    waist = y + body_h * 0.63
    if form.magic_stage >= 1:
        skirt_fill = pal.accent if form.magic_stage == 1 else pal.shirt
        hem_fill = pal.buttons if form.magic_stage == 1 else BROOCH_LIGHT
        px.polygon(
            [
                (x + 1.4, waist),
                (x + 1.2 + body_w - 0.2, waist),
                (x + 1.2 + body_w + 0.8, y + body_h + 1.9),
                (x + 0.4, y + body_h + 1.9),
            ],
            fill=skirt_fill,
            outline=OUTLINE,
            width=0.55,
        )
        px.line([(x + 1.0, y + body_h + 1.2), (x + 1.2 + body_w, y + body_h + 1.2)], fill=hem_fill, width=0.6)
        px.polygon(
            [(x + 1.2, waist + 0.2), (x - 0.9, waist - 0.6), (x - 0.1, waist + 1.2)],
            fill=RIBBON_PINK if form.magic_stage == 1 else pal.buttons,
            outline=OUTLINE,
            width=0.35,
        )
        px.polygon(
            [(x + 1.2 + body_w, waist + 0.2), (x + 3.3 + body_w, waist - 0.6), (x + 2.5 + body_w, waist + 1.2)],
            fill=RIBBON_PINK if form.magic_stage == 1 else pal.buttons,
            outline=OUTLINE,
            width=0.35,
        )
    _outlined_rect(px, x + 1.2, y + 0.0, x + 1.2 + body_w, y + body_h, fill=pal.shirt)
    px.polygon(
        [
            (x + 2.0, y + 1.4),
            (x + 1.2 + body_w - 0.8, y + 1.4),
            (x + 1.2 + body_w - 1.4, y + body_h + 0.8),
            (x + 2.8, y + body_h + 0.8),
        ],
        fill=pal.overalls,
        outline=OUTLINE,
        width=0.75,
    )
    px.line([(x + 3.2, y + 0.6), (x + 4.8, y + 4.6)], fill=pal.overalls, width=1.2)
    px.line([(x + 8.8, y + 0.6), (x + 7.2, y + 4.6)], fill=pal.overalls, width=1.2)
    _outlined_rect(px, x + 4.0, y + 2.8, x + 5.0, y + 4.0, fill=pal.buttons, inset=0.2)
    _outlined_rect(px, x + 7.0, y + 2.8, x + 8.0, y + 4.0, fill=pal.buttons, inset=0.2)
    if form.magic_stage >= 1:
        _draw_star(px, x + 5.9, y + 2.1, outer=1.0 if form.magic_stage == 1 else 1.35, inner=0.45, fill=BROOCH_GOLD, width=0.35)
        px.polygon(
            [(x + 5.9, y + 2.8), (x + 4.7, y + 4.1), (x + 7.1, y + 4.1)],
            fill=RIBBON_PINK if form.magic_stage == 1 else pal.accent,
            outline=OUTLINE,
            width=0.3,
        )
        if form.magic_stage >= 2:
            px.polygon(
                [(x + 1.4, y + 1.0), (x - 1.0, y + 2.0), (x + 1.0, y + 5.4)],
                fill=pal.buttons,
                outline=OUTLINE,
                width=0.35,
            )
            px.polygon(
                [(x + 10.8, y + 1.0), (x + 13.2, y + 2.0), (x + 11.2, y + 5.4)],
                fill=pal.buttons,
                outline=OUTLINE,
                width=0.35,
            )


def _draw_arm(px, x: float, y: float, *, front: bool, form: FormSpec, length: float = 4.2, glove_down: bool = True) -> None:
    pal = form.palette
    glove_fill = pal.gloves if form.power != "normal" else pal.skin
    _outlined_rect(px, x, y, x + 1.6, y + length, fill=pal.shirt)
    glove_y = y + (length - 0.5 if glove_down else -1.2)
    if form.magic_stage >= 1:
        cuff_fill = pal.accent if form.magic_stage == 1 else pal.buttons
        _outlined_rect(px, x - 0.1, glove_y - 0.8, x + 1.7, glove_y + 0.1, fill=cuff_fill, inset=0.15)
    _outlined_rect(px, x - 0.2, glove_y, x + 1.8, glove_y + 1.7, fill=glove_fill, inset=0.15)


def _draw_leg(px, x: float, y: float, *, form: FormSpec, length: float = 5.2, front: bool = False) -> None:
    pal = form.palette
    _outlined_rect(px, x + 0.2, y, x + 2.0, y + length, fill=pal.overalls)
    if form.magic_stage >= 1:
        cuff_fill = pal.accent if form.magic_stage == 1 else pal.buttons
        _outlined_rect(px, x, y + length - 1.1, x + 2.2, y + length - 0.2, fill=cuff_fill, inset=0.15)
    _outlined_rect(px, x - 0.4, y + length - 0.4, x + 2.8, y + length + 1.2, fill=pal.shoes)


def _draw_fire_orb(px, x: float, y: float) -> None:
    px.ellipse(x - 2.0, y - 2.0, x + 2.0, y + 2.0, fill=EMBER_ORANGE, outline=OUTLINE, width=0.45)
    px.ellipse(x - 1.0, y - 1.0, x + 1.0, y + 1.0, fill=EMBER_CORE, outline=OUTLINE, width=0.3)
    _draw_star(px, x + 2.3, y - 1.4, outer=0.8, inner=0.35, fill=BROOCH_LIGHT, width=0.25)


def _draw_suspender_fasteners_front(px, x: float, y: float, form: FormSpec) -> None:
    # Keep the classic overall-button read from the base Mary-O sprite.
    for cx in (x + 4.5, x + 7.5):
        px.ellipse(cx - 0.9, y + 2.65, cx + 0.9, y + 4.15, fill=form.palette.buttons, outline=OUTLINE, width=0.34)
        px.ellipse(cx - 0.28, y + 2.95, cx + 0.28, y + 3.50, fill=BROOCH_LIGHT, outline=None)


def _draw_suspender_fasteners_side(px, x: float, y: float, form: FormSpec) -> None:
    # Side views still keep two readable gold fasteners so the silhouette maps
    # back to the corresponding detail in the short/base form.
    for cx in (x + 4.15, x + 7.05):
        px.ellipse(cx - 0.84, y + 2.75, cx + 0.84, y + 4.18, fill=form.palette.buttons, outline=OUTLINE, width=0.34)
        px.ellipse(cx - 0.24, y + 3.02, cx + 0.24, y + 3.56, fill=BROOCH_LIGHT, outline=None)


def _draw_transform_outfit_stars(px, body_x: float, body_top: float, *, phase: int, form: FormSpec) -> None:
    star_fill = AURA_GOLD if form.magic_stage >= 2 else BROOCH_GOLD
    positions = [
        (body_x + 8.6, body_top + 2.2, 0.9),
        (body_x + 6.0, body_top + 6.3, 0.8),
        (body_x + 3.5, body_top + 9.2, 0.72),
    ]
    for sx, sy, outer in positions[: max(0, min(phase, len(positions)))]:
        _draw_star(px, sx, sy, outer=outer, inner=outer * 0.42, fill=star_fill, width=0.22)


def _draw_sleeve_wing_side(px, anchor_x: float, anchor_y: float, *, form: FormSpec, strength: float = 1.0, facing: float = 1.0) -> None:
    stage = _magic_stage_value(form)
    fire_t = _fire_transition_t(form)
    if strength <= 0.0 or stage < 1:
        return
    outer = _lerp_rgba(form.palette.accent, form.palette.buttons, fire_t)
    inner = _lerp_rgba(BROOCH_LIGHT, WING_PEARL, fire_t)
    span = 1.3 + 0.8 * strength + 0.45 * _fire_accessory_t(form)
    lift = 0.8 + 0.35 * strength
    px.polygon(
        [
            (anchor_x, anchor_y),
            (anchor_x + facing * span, anchor_y - lift),
            (anchor_x + facing * 0.2, anchor_y + 0.9),
        ],
        fill=outer,
        outline=OUTLINE,
        width=0.3,
    )
    px.polygon(
        [
            (anchor_x + facing * 0.1, anchor_y + 0.35),
            (anchor_x + facing * (span + 0.5), anchor_y + 0.1),
            (anchor_x + facing * 0.25, anchor_y + 1.2),
        ],
        fill=inner,
        outline=OUTLINE,
        width=0.28,
    )
    if fire_t >= 0.65 or strength > 0.8:
        px.polygon(
            [
                (anchor_x + facing * 0.15, anchor_y + 0.8),
                (anchor_x + facing * (span * 0.9), anchor_y + 1.35),
                (anchor_x + facing * 0.2, anchor_y + 1.55),
            ],
            fill=outer,
            outline=OUTLINE,
            width=0.28,
        )


def _draw_wing_side(px, anchor_x: float, anchor_y: float, *, form: FormSpec, spread: float = 0.0) -> None:
    stage = _magic_stage_value(form)
    if stage < 1:
        return
    pal = form.palette
    fire_t = _fire_transition_t(form)
    phase = stage + spread
    outer = _lerp_rgba(pal.accent, pal.buttons, fire_t)
    inner = _lerp_rgba(BROOCH_LIGHT, WING_PEARL, fire_t)
    fire_bonus = 0.9 * _fire_accessory_t(form)
    depth = 2.4 + 0.8 * phase + fire_bonus
    height = 1.4 + 0.45 * phase + 0.35 * fire_bonus
    lift = 0.5 * spread + 0.2 * fire_bonus
    px.polygon(
        [
            (anchor_x, anchor_y + 0.4),
            (anchor_x - depth, anchor_y - height - lift),
            (anchor_x - 0.4, anchor_y - 0.3),
        ],
        fill=outer,
        outline=OUTLINE,
        width=0.35,
    )
    px.polygon(
        [
            (anchor_x, anchor_y + 0.8),
            (anchor_x - depth - 0.8, anchor_y + 0.6),
            (anchor_x - 0.4, anchor_y + 1.1),
        ],
        fill=inner,
        outline=OUTLINE,
        width=0.35,
    )
    if fire_t >= 0.55 or spread >= 0.45:
        px.polygon(
            [
                (anchor_x + 0.2, anchor_y + 1.0),
                (anchor_x - depth * 0.9, anchor_y + height + 1.3 + lift),
                (anchor_x - 0.2, anchor_y + 1.8),
            ],
            fill=outer,
            outline=OUTLINE,
            width=0.35,
        )
        if form.magic_stage >= 2:
            px.polygon(
                [
                    (anchor_x + 0.35, anchor_y + 0.1),
                    (anchor_x - depth * 0.72, anchor_y - height * 0.2),
                    (anchor_x + 0.15, anchor_y + 0.95),
                ],
                fill=inner,
                outline=OUTLINE,
                width=0.28,
            )
        _draw_star(px, anchor_x - depth * 0.7, anchor_y - height - 0.4, outer=0.7, inner=0.3, fill=AURA_GOLD, width=0.25)


def _draw_wings_front(px, center_x: float, shoulder_y: float, *, form: FormSpec, spread: float = 0.0) -> None:
    stage = _magic_stage_value(form)
    if stage < 1:
        return
    pal = form.palette
    fire_t = _fire_transition_t(form)
    outer = _lerp_rgba(pal.accent, pal.buttons, fire_t)
    inner = _lerp_rgba(BROOCH_LIGHT, WING_PEARL, fire_t)
    fire_bonus = 0.8 * _fire_accessory_t(form)
    wing_h = 2.6 + 0.7 * (stage + spread) + 0.5 * fire_bonus
    wing_w = 3.8 + 0.9 * (stage + spread) + 0.9 * fire_bonus
    for sign in (-1, 1):
        px.polygon(
            [
                (center_x + sign * 1.4, shoulder_y + 0.6),
                (center_x + sign * wing_w, shoulder_y - wing_h),
                (center_x + sign * 2.2, shoulder_y + 0.4),
            ],
            fill=outer,
            outline=OUTLINE,
            width=0.35,
        )
        px.polygon(
            [
                (center_x + sign * 1.6, shoulder_y + 1.2),
                (center_x + sign * (wing_w + 0.3), shoulder_y + 0.8),
                (center_x + sign * 2.0, shoulder_y + 2.0),
            ],
            fill=inner,
            outline=OUTLINE,
            width=0.35,
        )
    if fire_t >= 0.55 or spread >= 0.5:
        for sign in (-1, 1):
            px.polygon(
                [
                    (center_x + sign * 1.5, shoulder_y + 1.7),
                    (center_x + sign * (wing_w + 0.5), shoulder_y + 2.2),
                    (center_x + sign * 2.0, shoulder_y + 2.8),
                ],
                fill=outer,
                outline=OUTLINE,
                width=0.28,
            )
        _draw_star(px, center_x - wing_w - 0.6, shoulder_y - wing_h + 0.2, outer=0.7, inner=0.3, fill=AURA_GOLD, width=0.25)
        _draw_star(px, center_x + wing_w + 0.6, shoulder_y - wing_h + 0.2, outer=0.7, inner=0.3, fill=AURA_GOLD, width=0.25)


def _draw_transform_aura(px, frame_idx: int) -> None:
    blast = min(frame_idx, 5)
    radii = [4.2, 5.2, 6.4, 7.6, 8.8, 8.0, 7.0, 6.0]
    rx = radii[frame_idx % len(radii)]
    ry = rx * 1.18
    cx, cy = 12.0, 12.8
    px.ellipse(cx - rx, cy - ry, cx + rx, cy + ry, fill=AURA_PINK, outline=None)
    px.ellipse(cx - rx * 0.78, cy - ry * 0.78, cx + rx * 0.78, cy + ry * 0.78, fill=AURA_GOLD, outline=None)
    if blast >= 3:
        px.ellipse(cx - rx * 0.48, cy - ry * 0.48, cx + rx * 0.48, cy + ry * 0.48, fill=(255, 248, 220, 255), outline=None)
    burst_sets = [
        [(2.6, 8.0, 1.0), (21.2, 8.0, 1.0), (12.0, 2.8, 0.9), (12.0, 21.0, 0.8)],
        [(2.0, 7.2, 1.15), (21.8, 7.2, 1.15), (4.5, 16.8, 0.8), (19.5, 16.8, 0.8), (12.0, 2.2, 1.0)],
        [(1.6, 6.2, 1.28), (22.2, 6.2, 1.28), (3.5, 15.8, 0.95), (20.3, 15.8, 0.95), (12.0, 1.8, 1.1), (12.0, 22.1, 0.95)],
        [(1.2, 5.5, 1.45), (22.8, 5.5, 1.45), (2.8, 14.8, 1.1), (21.0, 14.8, 1.1), (12.0, 1.2, 1.22), (12.0, 22.6, 1.0)],
        [(1.5, 5.8, 1.25), (22.5, 5.8, 1.25), (3.2, 14.5, 0.95), (20.8, 14.5, 0.95), (12.0, 1.5, 1.05)],
        [(2.5, 6.6, 1.0), (21.5, 6.6, 1.0), (4.2, 15.3, 0.75), (19.2, 15.3, 0.75)],
        [(3.6, 7.4, 0.8), (20.4, 7.4, 0.8), (5.6, 16.0, 0.65), (18.2, 16.0, 0.65)],
        [(4.2, 7.8, 0.65), (19.8, 7.8, 0.65), (6.0, 16.2, 0.55), (17.8, 16.2, 0.55)],
    ]
    for x, y, outer in burst_sets[frame_idx % len(burst_sets)]:
        fill = (255, 248, 220, 255) if outer >= 1.2 else (AURA_GOLD if outer >= 0.85 else AURA_PINK)
        _draw_star(px, x, y, outer=outer, inner=outer * 0.42, fill=fill, width=0.22)


def _draw_power_loss_sparkles(px, frame_idx: int, *, fire: bool = False) -> None:
    sparkle_sets = [
        [(6.0, 8.4, 0.7), (17.8, 9.2, 0.6), (11.8, 18.4, 0.5)],
        [(7.0, 10.1, 0.65), (18.0, 11.0, 0.55), (12.0, 19.6, 0.45)],
        [(8.4, 12.0, 0.55), (17.0, 13.0, 0.45)],
        [(9.4, 14.0, 0.5), (15.8, 14.8, 0.4)],
        [(10.3, 15.2, 0.42)],
        [],
    ]
    for x, y, outer in sparkle_sets[min(frame_idx, len(sparkle_sets) - 1)]:
        fill = AURA_GOLD if fire and outer >= 0.55 else AURA_PINK
        _draw_star(px, x, y, outer=outer, inner=max(0.2, outer * 0.42), fill=fill, width=0.22)
    if fire and frame_idx <= 3:
        # a few embers trail downward as the power drains away
        ember_sets = [
            [(18.8, 13.5), (20.3, 15.2)],
            [(18.1, 14.7), (19.4, 16.4)],
            [(17.2, 16.0)],
            [(16.4, 17.2)],
        ]
        for ex, ey in ember_sets[min(frame_idx, len(ember_sets) - 1)]:
            px.ellipse(ex - 0.55, ey - 0.55, ex + 0.55, ey + 0.55, fill=EMBER_ORANGE, outline=OUTLINE, width=0.2)


def _draw_dead_front(px, form: FormSpec, pose: Pose, *, wing_boost: float = 0.0) -> None:
    body_x = 6.0
    foot_y = 28.8 + pose.bob
    torso_bottom = foot_y - form.leg_height
    body_top = torso_bottom - form.body_height
    head_top = body_top - 10.2

    left_hip_x = body_x + 4.9
    right_hip_x = body_x + 7.3
    hip_y = torso_bottom + 0.2
    _draw_rotated_leg(
        px,
        left_hip_x,
        hip_y,
        form=form,
        angle_deg=-14.0,
        length=form.leg_height - 0.4,
        front=True,
    )
    _draw_rotated_leg(
        px,
        right_hip_x,
        hip_y,
        form=form,
        angle_deg=14.0,
        length=form.leg_height - 0.4,
        front=True,
    )

    _draw_wings_front(px, body_x + 6.0, body_top + 2.2, form=form, spread=wing_boost)
    _draw_body_front(px, form, body_x, body_top)
    head_x = body_x + 0.3
    _draw_head_front(px, form, head_x, head_top)
    if pose.mode == "dead":
        if form.magic_stage >= 1:
            cover_fill = form.palette.overalls if form.magic_stage <= 1 else V2_IVORY
            px.polygon([(body_x + 2.25, body_top + 3.55), (body_x + 9.75, body_top + 3.55), (body_x + 8.95, body_top + 10.8), (body_x + 3.05, body_top + 10.8)], fill=cover_fill, outline=None)
            px.line([(body_x + 2.45, body_top + 4.15), (body_x + 9.2, body_top + 4.15)], fill=OUTLINE, width=0.34)
        _draw_dead_mouth_front(px, head_x, head_top)

    shoulder_y = body_top + 0.7
    _draw_rotated_arm(
        px,
        body_x + 3.0,
        shoulder_y,
        front=True,
        form=form,
        angle_deg=-135.0,
        length=5.3,
    )
    _draw_rotated_arm(
        px,
        body_x + 9.0,
        shoulder_y,
        front=True,
        form=form,
        angle_deg=135.0,
        length=5.3,
    )


def _draw_side_pose(px, form: FormSpec, pose: Pose, *, animation: str = "idle", wing_boost: float = 0.0, sleeve_wing_boost: float = 0.0, extra_star_phase: int = 0) -> None:
    stage = _magic_stage_value(form)
    fire_accessory_t = _fire_accessory_t(form)
    foot_y = 30.2 + pose.bob
    torso_bottom = foot_y - form.leg_height + 0.4 * pose.crouch
    body_top = torso_bottom - form.body_height + 0.6 * pose.crouch
    head_top = body_top - 10.0 + 0.8 * pose.crouch + pose.head_dy
    body_x = 7.0 + pose.body_lean

    if pose.mode == "swim":
        body_x = 6.3 + pose.body_lean
        head_top -= 0.6
    elif pose.mode == "crouch":
        body_x = 6.8 + pose.body_lean
    elif pose.mode == "climb":
        body_x = 6.4 + pose.body_lean

    body_w = form.body_width + 0.4 * min(pose.crouch, 1.4)
    back_shoulder = (body_x + 1.8 + pose.arm_back_dx, body_top + 1.4 + pose.arm_back_dy)
    front_shoulder = (body_x + body_w - 0.2 + pose.arm_front_dx, body_top + 1.2 + pose.arm_front_dy)
    back_hip = (body_x + 3.0 + pose.leg_back_dx, torso_bottom + pose.leg_back_dy)
    front_hip = (body_x + 6.3 + pose.leg_front_dx, torso_bottom + pose.leg_front_dy)

    if pose.arm_back_angle is not None:
        _draw_rotated_arm(
            px,
            back_shoulder[0],
            back_shoulder[1],
            front=False,
            form=form,
            angle_deg=pose.arm_back_angle,
            length=4.4 if pose.mode != "climb" else 4.8,
        )
    else:
        _draw_arm(
            px,
            body_x - 1.4 + pose.arm_back_dx,
            body_top + 1.1 + pose.arm_back_dy,
            front=False,
            form=form,
            length=4.0,
        )

    if pose.leg_back_angle is not None:
        _draw_rotated_leg(
            px,
            back_hip[0],
            back_hip[1],
            form=form,
            angle_deg=pose.leg_back_angle,
            length=form.leg_height - 0.5 * pose.crouch,
            front=False,
        )
    else:
        _draw_leg(
            px,
            body_x + 2.1 + pose.leg_back_dx,
            torso_bottom + pose.leg_back_dy,
            form=form,
            length=form.leg_height - 0.6 * pose.crouch,
        )

    side_wing_boost = wing_boost + 0.45 * fire_accessory_t + (0.25 if animation == "fireball" else 0.0)
    sleeve_boost = sleeve_wing_boost + 0.85 * fire_accessory_t
    _draw_wing_side(px, body_x + 1.6, body_top + 3.4, form=form, spread=side_wing_boost)
    if stage >= 1.7:
        _draw_wing_side(px, body_x + 2.6, body_top + 5.1, form=form, spread=max(0.0, side_wing_boost - 0.15))
    if sleeve_boost > 0.0:
        _draw_sleeve_wing_side(px, back_shoulder[0] - 0.3, back_shoulder[1] + 1.1, form=form, strength=max(0.45, sleeve_boost * 0.8), facing=-1.0)

    # Keep the front leg tucked behind the dress / skirt silhouette in side view.
    if pose.leg_front_angle is not None:
        _draw_rotated_leg(
            px,
            front_hip[0],
            front_hip[1],
            form=form,
            angle_deg=pose.leg_front_angle,
            length=form.leg_height - 0.5 * pose.crouch,
            front=True,
        )
    else:
        _draw_leg(
            px,
            body_x + 5.1 + pose.leg_front_dx,
            torso_bottom + pose.leg_front_dy,
            form=form,
            length=form.leg_height - 0.6 * pose.crouch,
            front=True,
        )

    _draw_body_side(px, form, body_x, body_top, pose.crouch)
    if extra_star_phase > 0:
        _draw_transform_outfit_stars(px, body_x, body_top, phase=extra_star_phase, form=form)
    head_x = body_x - 0.4 + pose.head_dx
    lookback = pose.mode == "lookback"
    _draw_head_side(px, form, head_x, head_top, lookback=lookback)
    if pose.mode == "dead":
        # Restore the classic "oh no!" read and keep the ponytail behind the torso.
        _draw_dead_mouth_side(px, head_x, head_top, lookback=lookback)
        cover_fill = form.palette.overalls if form.magic_stage <= 1 else V2_IVORY
        px.polygon([(body_x + 0.6, body_top + 3.2), (body_x + 6.6, body_top + 3.5), (body_x + 5.9, body_top + 11.0), (body_x + 1.2, body_top + 10.8)], fill=cover_fill, outline=None)
        px.line([(body_x + 1.2, body_top + 4.0), (body_x + 5.8, body_top + 4.25)], fill=OUTLINE, width=0.34)

    if sleeve_boost > 0.0:
        _draw_sleeve_wing_side(px, front_shoulder[0] + 0.2, front_shoulder[1] + 1.0, form=form, strength=sleeve_boost, facing=1.0)
    if pose.arm_front_angle is not None:
        _draw_rotated_arm(
            px,
            front_shoulder[0],
            front_shoulder[1],
            front=True,
            form=form,
            angle_deg=pose.arm_front_angle,
            length=5.2 if pose.mode == "fireball" else (4.8 if pose.mode in {"swim", "climb"} else 4.4),
        )
    else:
        _draw_arm(
            px,
            body_x + 8.3 + pose.arm_front_dx,
            body_top + 0.8 + pose.arm_front_dy,
            front=True,
            form=form,
            length=4.0,
        )

    if form.power == "fire" and animation == "fireball":
        orb_x = front_shoulder[0] + 5.0
        orb_y = front_shoulder[1] + 0.8
        _draw_fire_orb(px, orb_x, orb_y)



# ---------------------------------------------------------------------------
# Mary-O v2 visual redraft
# ---------------------------------------------------------------------------
# Preserve the accepted movement and transition choreography above, but replace
# the component art with a more deliberate second-draft costume system. These
# late definitions are intentional: Python resolves the drawing helpers by
# name when a frame is rendered, so all existing poses and transition clips use
# the v2 components below without copying or weakening their animation logic.

_v1_draw_head_side = _draw_head_side
_v1_draw_head_front = _draw_head_front
_v1_draw_body_side = _draw_body_side
_v1_draw_body_front = _draw_body_front
_v1_draw_side_pose = _draw_side_pose

V2_TEAL_DARK = (17, 91, 117, 255)
V2_TEAL_LIGHT = (40, 144, 172, 255)
V2_PINK_DARK = (184, 72, 121, 255)
V2_PINK_LIGHT = (229, 132, 180, 255)
V2_GOLD_DARK = (217, 126, 28, 255)
V2_GOLD = (255, 201, 64, 255)
V2_IVORY = (255, 250, 239, 255)
V2_ORANGE = (255, 116, 38, 255)
V2_ORANGE_DARK = (211, 70, 25, 255)


def _draw_v2_cap_badge(px, form: FormSpec, x: float, y: float, *, lookback: bool) -> None:
    cx = x + (4.5 if lookback else 6.2)
    cy = y + 2.35
    ring = form.palette.buttons
    center = form.palette.cap if form.magic_stage < 2 else V2_ORANGE
    px.ellipse(cx - 1.15, cy - 1.05, cx + 1.15, cy + 1.05, fill=ring, outline=OUTLINE, width=0.35)
    _draw_star(px, cx, cy, outer=0.72, inner=0.31, fill=center, outline=OUTLINE, width=0.22)


def _draw_v2_hat_wing(px, form: FormSpec, x: float, y: float, *, lookback: bool) -> None:
    accessory_t = _fire_accessory_t(form)
    if accessory_t <= 0.0:
        return
    sign = 1.0 if lookback else -1.0
    anchor_x = x + (10.4 if lookback else 1.0)
    anchor_y = y + 2.2
    outer = _lerp_rgba(V2_PINK_LIGHT, V2_GOLD, accessory_t)
    inner = _lerp_rgba(BROOCH_LIGHT, V2_IVORY, accessory_t)
    spans = (2.3 + 0.6 * accessory_t, 1.75 + 0.55 * accessory_t)
    if accessory_t >= 0.62:
        spans += (1.15 + 0.65 * accessory_t,)
    for i, span in enumerate(spans):
        dy = i * 0.75
        px.polygon(
            [
                (anchor_x, anchor_y + dy),
                (anchor_x + sign * span, anchor_y - 0.9 + dy),
                (anchor_x + sign * 0.45, anchor_y + 0.7 + dy),
            ],
            fill=outer if i % 2 == 0 else inner,
            outline=OUTLINE,
            width=0.3,
        )
    if accessory_t >= 0.55:
        lift = 1.5 + 1.0 * accessory_t
        px.polygon(
            [
                (anchor_x + sign * 0.1, anchor_y - 0.1),
                (anchor_x + sign * (0.4 + 0.5 * accessory_t), anchor_y - lift),
                (anchor_x + sign * (0.8 + 0.75 * accessory_t), anchor_y - 0.4),
            ],
            fill=V2_ORANGE,
            outline=OUTLINE,
            width=0.3,
        )


def _draw_v2_ear_star(px, form: FormSpec, x: float, y: float, *, lookback: bool) -> None:
    if _magic_stage_value(form) < 1:
        return
    cx = x + (8.7 if lookback else 2.6)
    cy = y + 8.0
    outer = 0.72 + 0.16 * _fire_transition_t(form)
    _draw_star(px, cx, cy, outer=outer, inner=0.34, fill=form.palette.buttons, outline=OUTLINE, width=0.24)


def _add_side_blush(px, x: float, y: float, *, lookback: bool = False) -> None:
    if lookback:
        px.rect(x + 5.2, y + 7.85, x + 6.2, y + 8.55, fill=BLUSH)
    else:
        px.rect(x + 4.75, y + 7.85, x + 5.75, y + 8.55, fill=BLUSH)


def _nose_tone(form: FormSpec) -> tuple[int, int, int, int]:
    r, g, b, a = form.palette.skin
    return (max(0, r - 28), max(0, g - 30), max(0, b - 24), a)


def _draw_side_nose(px, form: FormSpec, x: float, y: float, *, lookback: bool = False) -> None:
    """Stamp the exact same rounded profile in every side-facing frame.

    We now use one tiny physical-pixel stencil derived from the death-pose nose
    read and only translate / mirror it. The cheek-side seam stays open (no ink)
    so the profile reads as skin extending out from the face, not a pasted-on
    outlined square.
    """
    nose = _nose_tone(form)
    scale = px.scale

    # Canonical rounded profile shared by all side poses. Column zero is the
    # face seam and intentionally receives no outline.
    fill_rows = (
        (1, 2),
        (0, 1, 2, 3),
        (0, 1, 2, 3),
        (0, 1, 2),
        (1, 2),
    )
    outline_pixels = {
        (1, 0), (2, 0),
        (3, 1),
        (3, 2),
        (2, 3),
        (1, 4), (2, 4),
    }

    if lookback:
        # Exact horizontal mirror of the east-facing stamp.
        anchor_x = int(round((x + 3.18) * scale))
        x_sign = -1
    else:
        anchor_x = int(round((x + 8.22) * scale))
        x_sign = 1
    anchor_y = int(round((y + 7.32) * scale))

    for row, columns in enumerate(fill_rows):
        for column in columns:
            px.draw.point(
                (anchor_x + x_sign * column, anchor_y + row),
                fill=nose,
            )
    for column, row in outline_pixels:
        px.draw.point(
            (anchor_x + x_sign * column, anchor_y + row),
            fill=OUTLINE,
        )


def _repair_face_side(px, form: FormSpec, x: float, y: float, *, lookback: bool = False) -> None:
    pal = form.palette
    # Repaint the lower side face as a shaped cheek patch instead of a square
    # overpaint. This gives the canonical nose a clean skin-tone bed without
    # introducing rectangular artifacts behind the profile.
    if lookback:
        cheek_fill = [
            (x + 2.70, y + 7.10),
            (x + 4.95, y + 7.10),
            (x + 5.20, y + 7.90),
            (x + 5.00, y + 9.65),
            (x + 2.92, y + 9.78),
            (x + 2.56, y + 8.34),
        ]
        mouth_pts = [(x + 3.46, y + 9.25), (x + 4.18, y + 9.25)]
    else:
        cheek_fill = [
            (x + 6.90, y + 7.10),
            (x + 9.05, y + 7.10),
            (x + 9.42, y + 7.92),
            (x + 9.10, y + 9.68),
            (x + 7.02, y + 9.78),
            (x + 6.76, y + 8.36),
        ]
        mouth_pts = [(x + 7.72, y + 9.25), (x + 8.44, y + 9.25)]
    px.polygon(cheek_fill, fill=pal.skin, outline=None)
    _draw_side_nose(px, form, x, y, lookback=lookback)
    _add_side_blush(px, x, y, lookback=lookback)
    px.line(mouth_pts, fill=LIP, width=0.28)


def _repair_face_front(px, form: FormSpec, x: float, y: float) -> None:
    pal = form.palette
    px.rect(x + 4.0, y + 8.95, x + 7.0, y + 10.15, fill=pal.skin)
    px.rect(x + 2.4, y + 7.5, x + 3.95, y + 8.5, fill=BLUSH)
    px.rect(x + 7.05, y + 7.5, x + 8.6, y + 8.5, fill=BLUSH)
    px.line([(x + 5.38, y + 7.26), (x + 5.08, y + 8.55), (x + 5.82, y + 8.82)], fill=OUTLINE, width=0.28)
    px.line([(x + 4.78, y + 9.55), (x + 6.2, y + 9.55)], fill=LIP, width=0.28)


def _draw_dead_mouth_side(px, x: float, y: float, *, lookback: bool = False) -> None:
    if lookback:
        px.ellipse(x + 3.42, y + 8.93, x + 4.62, y + 9.93, fill=LIP, outline=OUTLINE, width=0.22)
        px.ellipse(x + 3.78, y + 9.18, x + 4.26, y + 9.72, fill=(56, 20, 34, 255), outline=None)
    else:
        px.ellipse(x + 7.35, y + 8.93, x + 8.55, y + 9.93, fill=LIP, outline=OUTLINE, width=0.22)
        px.ellipse(x + 7.71, y + 9.18, x + 8.19, y + 9.72, fill=(56, 20, 34, 255), outline=None)


def _draw_dead_mouth_front(px, x: float, y: float) -> None:
    px.ellipse(x + 4.72, y + 9.03, x + 6.38, y + 10.23, fill=LIP, outline=OUTLINE, width=0.22)
    px.ellipse(x + 5.10, y + 9.30, x + 6.00, y + 9.96, fill=(56, 20, 34, 255), outline=None)


def _draw_head_side(px, form: FormSpec, x: float, y: float, *, lookback: bool = False) -> None:
    # Preserve the clearer accepted eyes / hat / star language, then repaint
    # the lower face so every side-facing pose reuses one audited nose model.
    _v1_draw_head_side(px, form, x, y, lookback=lookback)
    _repair_face_side(px, form, x, y, lookback=lookback)
    if _magic_stage_value(form) <= 0:
        return
    _draw_v2_ear_star(px, form, x, y, lookback=lookback)
    _draw_v2_hat_wing(px, form, x, y, lookback=lookback)


def _draw_head_front(px, form: FormSpec, x: float, y: float) -> None:
    _v1_draw_head_front(px, form, x, y)
    if _magic_stage_value(form) >= 1:
        _repair_face_front(px, form, x, y)
    accessory_t = _fire_accessory_t(form)
    if accessory_t > 0.0:
        for sign in (-1, 1):
            anchor = x + 5.4 + sign * 5.1
            span = 1.15 + 0.65 * accessory_t
            px.polygon(
                [(anchor, y + 2.5), (anchor + sign * span, y + 1.9 - 0.35 * accessory_t), (anchor + sign * 0.45, y + 3.45)],
                fill=_lerp_rgba(V2_PINK_LIGHT, V2_GOLD, accessory_t),
                outline=OUTLINE,
                width=0.28,
            )


def _draw_v2_button(px, cx: float, cy: float, fill) -> None:
    px.ellipse(cx - 0.78, cy - 0.72, cx + 0.78, cy + 0.72, fill=fill, outline=OUTLINE, width=0.32)
    px.ellipse(cx - 0.21, cy - 0.20, cx + 0.21, cy + 0.20, fill=BROOCH_LIGHT, outline=None)


def _draw_hand_outline(px, x1: float, y1: float, x2: float, y2: float) -> None:
    px.line([(x1, y1), (x2, y1)], fill=OUTLINE, width=0.34)
    px.line([(x1, y2), (x2, y2)], fill=OUTLINE, width=0.34)
    px.line([(x1, y1), (x1, y2)], fill=OUTLINE, width=0.34)
    px.line([(x2, y1), (x2, y2)], fill=OUTLINE, width=0.34)


def _draw_body_side(px, form: FormSpec, x: float, y: float, crouch: float) -> None:
    pal = form.palette
    stage = _magic_stage_value(form)
    fire_t = _fire_transition_t(form)
    accessory_t = _fire_accessory_t(form)
    body_h = form.body_height - 0.55 * crouch
    body_w = form.body_width + 0.4 * min(crouch, 1.4)
    waist = y + body_h * (0.58 if stage else 0.62)
    bottom = y + body_h

    # Shirt / blouse base.
    _outlined_rect(px, x + 1.0, y, x + 1.0 + body_w, waist + 0.7, fill=pal.shirt, inset=0.42)

    if stage == 0:
        return _v1_draw_body_side(px, form, x, y, crouch)

    # Powered forms: integrated bodice, star centerpiece, and flared skirt.
    bodice_fill = pal.overalls
    px.polygon(
        [(x + 2.0, y + 1.1), (x + body_w + 0.05, y + 1.1), (x + body_w - 0.35, waist + 0.7), (x + 1.65, waist + 0.7)],
        fill=bodice_fill,
        outline=OUTLINE,
        width=0.7,
    )
    strap = _lerp_rgba(V2_TEAL_DARK, V2_GOLD_DARK, fire_t)
    px.line([(x + 2.5, y + 0.25), (x + 4.05, y + 3.0)], fill=strap, width=1.1)
    px.line([(x + body_w - 0.65, y + 0.25), (x + 7.05, y + 3.0)], fill=strap, width=1.1)
    _draw_v2_button(px, x + 4.05, y + 3.1, pal.buttons)
    _draw_v2_button(px, x + 7.05, y + 3.1, pal.buttons)

    flare = 1.45 + 0.55 * fire_t
    skirt_fill = _lerp_rgba(pal.overalls, V2_IVORY, fire_t)
    px.polygon(
        [
            (x + 1.7, waist + 0.4),
            (x + body_w + 0.25, waist + 0.4),
            (x + body_w + flare, bottom + 2.0),
            (x + 0.75 - flare * 0.25, bottom + 1.9),
        ],
        fill=skirt_fill,
        outline=OUTLINE,
        width=0.72,
    )
    hem = _lerp_rgba(V2_GOLD, V2_ORANGE, fire_t)
    px.line([(x + 1.0, bottom + 1.25), (x + body_w + flare - 0.4, bottom + 1.25)], fill=hem, width=0.82)
    if fire_t >= 0.72:
        # Keep the tall form closer to classic SMB1; only the late fire phase
        # gets the extra vertical pleat rhythm.
        pleat = _lerp_rgba(V2_TEAL_DARK, V2_GOLD_DARK, fire_t)
        px.line([(x + 4.0, waist + 0.8), (x + 3.65, bottom + 1.0)], fill=pleat, width=0.28)
        px.line([(x + 7.0, waist + 0.8), (x + 7.45, bottom + 1.0)], fill=pleat, width=0.28)

    _draw_star(px, x + 5.65, y + 4.25, outer=1.45 + 0.27 * fire_t, inner=0.68, fill=pal.buttons, width=0.34)
    # A small bow at the back echoes the generated second-draft concept.
    bow = _lerp_rgba(V2_PINK_LIGHT, V2_GOLD, fire_t)
    px.polygon([(x + 1.0, waist + 0.4), (x - 1.15, waist - 0.6), (x - 0.25, waist + 1.05)], fill=bow, outline=OUTLINE, width=0.32)
    px.polygon([(x + 0.9, waist + 0.55), (x - 0.35, waist + 2.0), (x + 1.2, waist + 1.25)], fill=bow, outline=OUTLINE, width=0.32)

    if accessory_t >= 0.5:
        # Flame-feather epaulettes ramp in during the late transform instead of
        # snapping on in a single frame.
        span = 1.2 + 0.8 * accessory_t
        for ax in (x + 0.9, x + body_w + 0.95):
            sign = -1.0 if ax < x + body_w / 2 else 1.0
            px.polygon([(ax, y + 1.0), (ax + sign * span, y + 0.6 - 0.5 * accessory_t), (ax + sign * 0.6, y + 2.6)], fill=V2_GOLD, outline=OUTLINE, width=0.32)
            px.polygon([(ax, y + 1.5), (ax + sign * (span + 0.4), y + 1.7), (ax + sign * 0.5, y + 3.0)], fill=V2_ORANGE, outline=OUTLINE, width=0.3)


def _draw_body_front(px, form: FormSpec, x: float, y: float, *, crouch: float = 0.0) -> None:
    pal = form.palette
    stage = _magic_stage_value(form)
    fire_t = _fire_transition_t(form)
    body_h = form.body_height - 0.55 * crouch
    body_w = form.body_width + 0.4 * min(crouch, 1.4)
    waist = y + body_h * (0.58 if stage else 0.62)
    bottom = y + body_h
    _outlined_rect(px, x + 1.2, y, x + 1.2 + body_w, waist + 0.7, fill=pal.shirt, inset=0.42)

    if stage == 0:
        return _v1_draw_body_front(px, form, x, y, crouch=crouch)

    px.polygon([(x + 2.1, y + 1.0), (x + body_w + 0.3, y + 1.0), (x + body_w - 0.25, waist + 0.7), (x + 1.8, waist + 0.7)], fill=pal.overalls, outline=OUTLINE, width=0.7)
    strap = _lerp_rgba(V2_TEAL_DARK, V2_GOLD_DARK, fire_t)
    px.line([(x + 3.1, y + 0.25), (x + 4.7, y + 3.0)], fill=strap, width=1.1)
    px.line([(x + 9.0, y + 0.25), (x + 7.35, y + 3.0)], fill=strap, width=1.1)
    _draw_v2_button(px, x + 4.7, y + 3.1, pal.buttons)
    _draw_v2_button(px, x + 7.35, y + 3.1, pal.buttons)
    flare = 1.55 + 0.60 * fire_t
    skirt_fill = _lerp_rgba(pal.overalls, V2_IVORY, fire_t)
    px.polygon([(x + 1.8, waist + 0.35), (x + body_w + 0.55, waist + 0.35), (x + body_w + flare, bottom + 2.0), (x + 0.6 - flare * 0.2, bottom + 2.0)], fill=skirt_fill, outline=OUTLINE, width=0.72)
    px.line([(x + 0.9, bottom + 1.25), (x + body_w + flare - 0.2, bottom + 1.25)], fill=_lerp_rgba(V2_GOLD, V2_ORANGE, fire_t), width=0.82)
    _draw_star(px, x + 6.0, y + 4.15, outer=1.5 + 0.25 * fire_t, inner=0.7, fill=pal.buttons, width=0.34)


def _draw_arm(px, x: float, y: float, *, front: bool, form: FormSpec, length: float = 4.2, glove_down: bool = True) -> None:
    pal = form.palette
    glove_fill = pal.skin if form.magic_stage == 0 else pal.gloves
    sleeve_fill = pal.shirt
    if form.magic_stage >= 1:
        puff = RIBBON_PINK if form.magic_stage == 1 else V2_IVORY
        px.ellipse(x - 0.65, y - 0.4, x + 2.35, y + 2.5, fill=puff, outline=OUTLINE, width=0.42)
        if form.magic_stage >= 2:
            px.polygon([(x + 0.1, y - 0.2), (x + 0.8, y - 1.5), (x + 1.5, y - 0.1)], fill=V2_GOLD, outline=OUTLINE, width=0.28)
    _outlined_rect(px, x, y + (1.0 if form.magic_stage >= 1 else 0.0), x + 1.6, y + length, fill=sleeve_fill, inset=0.35)
    glove_y = y + (length - 0.5 if glove_down else -1.2)
    if form.magic_stage >= 1:
        cuff_fill = V2_GOLD if form.magic_stage >= 2 else form.palette.cap
        _outlined_rect(px, x - 0.2, glove_y - 0.9, x + 1.8, glove_y + 0.1, fill=cuff_fill, inset=0.14)
    _outlined_rect(px, x - 0.25, glove_y, x + 1.85, glove_y + 1.75, fill=glove_fill, inset=0.15)
    _draw_hand_outline(px, x - 0.3, glove_y - 0.02, x + 1.9, glove_y + 1.8)


def _draw_rotated_arm(px, shoulder_x: float, shoulder_y: float, *, front: bool, form: FormSpec, angle_deg: float, length: float = 4.4) -> None:
    pal = form.palette
    hand_fill = pal.skin if form.magic_stage == 0 else pal.gloves
    end_x, end_y = _rotated_endpoint(shoulder_x, shoulder_y, angle_deg, length)
    if form.magic_stage >= 1:
        puff = RIBBON_PINK if form.magic_stage == 1 else V2_IVORY
        px.ellipse(shoulder_x - 1.35, shoulder_y - 1.25, shoulder_x + 1.35, shoulder_y + 1.35, fill=puff, outline=OUTLINE, width=0.42)
        if form.magic_stage >= 2:
            px.polygon([(shoulder_x - 0.7, shoulder_y - 0.8), (shoulder_x, shoulder_y - 2.0), (shoulder_x + 0.7, shoulder_y - 0.8)], fill=V2_GOLD, outline=OUTLINE, width=0.28)
    _draw_segment(px, shoulder_x, shoulder_y, end_x, end_y, half_w=0.74, fill=pal.shirt)
    if form.magic_stage >= 1:
        cuff_x, cuff_y = _rotated_endpoint(shoulder_x, shoulder_y, angle_deg, max(0.0, length - 0.95))
        _draw_segment(px, cuff_x, cuff_y, end_x, end_y, half_w=0.92, fill=V2_GOLD if form.magic_stage >= 2 else form.palette.cap)
    _outlined_rect(px, end_x - 1.0, end_y - 0.9, end_x + 1.0, end_y + 0.9, fill=hand_fill, inset=0.15)
    _draw_hand_outline(px, end_x - 1.05, end_y - 0.95, end_x + 1.05, end_y + 0.95)


def _draw_leg(px, x: float, y: float, *, form: FormSpec, length: float = 5.2, front: bool = False) -> None:
    pal = form.palette
    leg_fill = V2_TEAL_DARK if form.magic_stage == 1 else pal.overalls
    _outlined_rect(px, x + 0.2, y, x + 2.0, y + length, fill=leg_fill, inset=0.34)
    if form.magic_stage >= 1:
        _outlined_rect(px, x - 0.05, y + length - 1.25, x + 2.25, y + length - 0.15, fill=V2_GOLD if form.magic_stage >= 2 else V2_PINK_LIGHT, inset=0.14)
    _outlined_rect(px, x - 0.55, y + length - 0.45, x + 3.05, y + length + 1.3, fill=pal.shoes, inset=0.2)
    px.line([(x + 1.55, y + length + 0.15), (x + 2.75, y + length + 0.15)], fill=BROOCH_LIGHT if form.magic_stage >= 1 else (139, 91, 55, 255), width=0.34)


def _draw_rotated_leg(px, hip_x: float, hip_y: float, *, form: FormSpec, angle_deg: float, length: float = 5.4, front: bool = False) -> None:
    pal = form.palette
    leg_fill = V2_TEAL_DARK if form.magic_stage == 1 else pal.overalls
    end_x, end_y = _rotated_endpoint(hip_x, hip_y, angle_deg, length)
    _draw_segment(px, hip_x, hip_y, end_x, end_y, half_w=0.88, fill=leg_fill)
    shoe_dir = 1.0 if math.sin(math.radians(angle_deg)) >= 0 else -1.0
    x1 = end_x - 0.5 if shoe_dir > 0 else end_x - 2.9
    x2 = end_x + 2.5 if shoe_dir > 0 else end_x + 0.5
    if form.magic_stage >= 1:
        _outlined_rect(px, x1 + 0.1, end_y - 1.35, x2 - 0.1, end_y - 0.1, fill=V2_GOLD if form.magic_stage >= 2 else V2_PINK_LIGHT, inset=0.14)
    _outlined_rect(px, x1, end_y - 0.4, x2, end_y + 1.05, fill=pal.shoes, inset=0.16)


def _draw_wing_side(px, anchor_x: float, anchor_y: float, *, form: FormSpec, spread: float = 0.0) -> None:
    if form.magic_stage < 1:
        return
    if form.magic_stage == 1:
        # A controlled three-feather shoulder wing: more designed, less noisy.
        lengths = (3.3 + spread, 2.8 + spread * 0.7, 2.2 + spread * 0.5)
        for i, length in enumerate(lengths):
            dy = i * 0.85
            px.polygon(
                [(anchor_x, anchor_y + dy), (anchor_x - length, anchor_y - 1.1 + dy), (anchor_x - 0.35, anchor_y + 0.8 + dy)],
                fill=V2_IVORY if i == 1 else V2_PINK_LIGHT,
                outline=OUTLINE,
                width=0.32,
            )
        return

    # Fire form: layered gold-and-ivory pinions with a flame-shaped lower tail.
    lengths = (5.1 + spread, 4.4 + spread * 0.85, 3.7 + spread * 0.7, 3.0 + spread * 0.55)
    fills = (V2_GOLD, V2_IVORY, V2_GOLD, V2_IVORY)
    for i, (length, fill) in enumerate(zip(lengths, fills)):
        dy = i * 0.9
        px.polygon(
            [(anchor_x + 0.2, anchor_y + dy), (anchor_x - length, anchor_y - 1.7 + dy), (anchor_x - 0.4, anchor_y + 0.85 + dy)],
            fill=fill,
            outline=OUTLINE,
            width=0.34,
        )
    px.polygon(
        [(anchor_x, anchor_y + 2.6), (anchor_x - 3.8, anchor_y + 5.6), (anchor_x - 1.1, anchor_y + 4.7), (anchor_x - 2.0, anchor_y + 7.2), (anchor_x + 0.5, anchor_y + 4.4)],
        fill=V2_ORANGE,
        outline=OUTLINE,
        width=0.34,
    )
    # The main pinion also curls forward around the silhouette. This is what
    # gives the fire form the broad, heraldic wing read from the concept art
    # instead of letting every feather disappear behind the ponytail.
    if spread >= 0.4:
        for i, (length, fill) in enumerate(((4.2, V2_GOLD), (3.4, V2_IVORY), (2.7, V2_ORANGE))):
            dy = i * 0.9
            px.polygon(
                [(anchor_x + 0.7, anchor_y + 0.4 + dy), (anchor_x + length, anchor_y - 1.4 + dy), (anchor_x + 1.1, anchor_y + 1.2 + dy)],
                fill=fill,
                outline=OUTLINE,
                width=0.32,
            )
    _draw_star(px, anchor_x - 4.2, anchor_y - 1.2, outer=0.75, inner=0.31, fill=BROOCH_LIGHT, width=0.22)


def _draw_wings_front(px, center_x: float, shoulder_y: float, *, form: FormSpec, spread: float = 0.0) -> None:
    if form.magic_stage < 1:
        return
    for sign in (-1, 1):
        if form.magic_stage == 1:
            for i, length in enumerate((3.5 + spread, 2.8 + spread * 0.7, 2.2 + spread * 0.5)):
                px.polygon(
                    [(center_x + sign * 1.2, shoulder_y + 0.4 + i * 0.75), (center_x + sign * (1.2 + length), shoulder_y - 1.2 + i * 0.75), (center_x + sign * 1.8, shoulder_y + 1.2 + i * 0.75)],
                    fill=V2_PINK_LIGHT if i != 1 else V2_IVORY,
                    outline=OUTLINE,
                    width=0.3,
                )
        else:
            for i, length in enumerate((5.1 + spread, 4.2 + spread * 0.8, 3.4 + spread * 0.6, 2.7 + spread * 0.45)):
                px.polygon(
                    [(center_x + sign * 1.3, shoulder_y + 0.3 + i * 0.8), (center_x + sign * (1.3 + length), shoulder_y - 1.7 + i * 0.8), (center_x + sign * 1.9, shoulder_y + 1.2 + i * 0.8)],
                    fill=V2_GOLD if i % 2 == 0 else V2_IVORY,
                    outline=OUTLINE,
                    width=0.32,
                )


def _draw_sleeve_wing_side(px, anchor_x: float, anchor_y: float, *, form: FormSpec, strength: float = 1.0, facing: float = 1.0) -> None:
    if strength <= 0.0 or form.magic_stage < 1:
        return
    count = 2 if form.magic_stage == 1 else 3
    for i in range(count):
        span = 1.5 + strength * 0.9 - i * 0.2
        px.polygon(
            [(anchor_x, anchor_y + i * 0.55), (anchor_x + facing * span, anchor_y - 0.65 + i * 0.55), (anchor_x + facing * 0.2, anchor_y + 0.75 + i * 0.55)],
            fill=(V2_PINK_LIGHT if form.magic_stage == 1 else (V2_GOLD if i % 2 == 0 else V2_IVORY)),
            outline=OUTLINE,
            width=0.28,
        )


def _draw_fire_orb(px, x: float, y: float) -> None:
    # More substantial than the first draft: a bright core, hot ring, and
    # asymmetric flame crown that still reads as a compact gameplay projectile.
    px.ellipse(x - 2.45, y - 2.35, x + 2.45, y + 2.35, fill=V2_ORANGE_DARK, outline=OUTLINE, width=0.48)
    px.ellipse(x - 1.75, y - 1.75, x + 1.75, y + 1.75, fill=V2_ORANGE, outline=V2_GOLD_DARK, width=0.34)
    px.ellipse(x - 0.9, y - 0.9, x + 0.9, y + 0.9, fill=V2_IVORY, outline=V2_GOLD, width=0.26)
    px.polygon([(x - 1.3, y - 1.7), (x - 0.7, y - 4.2), (x + 0.2, y - 2.0)], fill=V2_GOLD, outline=OUTLINE, width=0.3)
    px.polygon([(x + 0.1, y - 1.8), (x + 1.2, y - 4.8), (x + 1.6, y - 1.5)], fill=V2_ORANGE, outline=OUTLINE, width=0.3)
    px.polygon([(x + 1.4, y - 0.9), (x + 3.8, y - 2.2), (x + 2.3, y + 0.2)], fill=V2_GOLD, outline=OUTLINE, width=0.3)
    _draw_star(px, x + 3.4, y + 1.9, outer=0.72, inner=0.3, fill=BROOCH_LIGHT, width=0.22)


def _draw_side_pose(px, form: FormSpec, pose: Pose, *, animation: str = "idle", wing_boost: float = 0.0, sleeve_wing_boost: float = 0.0, extra_star_phase: int = 0) -> None:
    _v1_draw_side_pose(
        px,
        form,
        pose,
        animation=animation,
        wing_boost=wing_boost,
        sleeve_wing_boost=sleeve_wing_boost,
        extra_star_phase=extra_star_phase,
    )

    # Re-stamp the canonical nose after all foreground limbs and accessories.
    # This prevents a cuff / wing outline from shaving a pixel off the nose in
    # isolated poses (notably fire crouch), so every side frame is identical.
    foot_y = 30.2 + pose.bob
    torso_bottom = foot_y - form.leg_height + 0.4 * pose.crouch
    body_top = torso_bottom - form.body_height + 0.6 * pose.crouch
    head_top = body_top - 10.0 + 0.8 * pose.crouch + pose.head_dy
    body_x = 7.0 + pose.body_lean
    if pose.mode == "swim":
        body_x = 6.2 + pose.body_lean
        head_top -= 0.6
    elif pose.mode == "crouch":
        body_x = 6.8 + pose.body_lean
    elif pose.mode == "climb":
        body_x = 6.4 + pose.body_lean
    head_x = body_x - 0.4 + pose.head_dx
    _draw_side_nose(px, form, head_x, head_top, lookback=pose.mode == "lookback")

def _poses_for(form: FormSpec) -> Dict[str, List[Pose]]:
    if form.tall:
        return TALL_LIKE_POSES
    return SHORT_POSES


def _draw_form(form: FormSpec, animation: str, frame_idx: int, nframes: int) -> Image.Image:
    if animation == "grow":
        # Hosted by the TALL sheet (the form arrived at). Named explicitly
        # rather than taken from `form` so the clip keeps meaning "small becomes
        # tall" wherever it is hosted, and ends on the form it arrives at.
        alt_form = SHORT_FORM if frame_idx % 2 == 0 else TALL_FORM
        return _draw_form(alt_form, "idle", 0, 1)

    if animation == "transform":
        fire_flash_1 = _mix_outfit_palette(MARY_NORMAL, MARY_FIRE_FLASH, 0.45)
        fire_flash_2 = _mix_outfit_palette(MARY_NORMAL, MARY_FIRE_FLASH, 0.88)
        fire_flash_3 = _mix_outfit_palette(MARY_FIRE_FLASH, MARY_FIRE_BLAST, 0.42)
        fire_flash_4 = _mix_outfit_palette(MARY_FIRE_FLASH, MARY_FIRE_BLAST, 0.82)
        fire_blast = MARY_FIRE_BLAST
        fire_reveal_1 = _mix_outfit_palette(MARY_FIRE_BLAST, MARY_FIRE, 0.18)
        fire_reveal_2 = _mix_outfit_palette(MARY_FIRE_BLAST, MARY_FIRE, 0.42)
        fire_reveal_3 = _mix_outfit_palette(MARY_FIRE_BLAST, MARY_FIRE, 0.72)
        transform_seq = [
            (_transition_form(TALL_FORM, MARY_NORMAL, stage=1.00), Pose(), 0.00, 0.00, 0, False),
            (_transition_form(TALL_FORM, MARY_NORMAL, stage=1.00), Pose(bob=-0.35, arm_front_angle=118, arm_back_angle=42, leg_front_angle=8, leg_back_angle=-8), 0.10, 0.00, 1, False),
            (_transition_form(FIRE_FORM, fire_flash_1, stage=1.16, power="tall"), Pose(bob=-0.75, body_lean=0.04, arm_front_angle=96, arm_back_angle=30, leg_front_angle=12, leg_back_angle=-9), 0.40, 0.18, 2, False),
            (_transition_form(FIRE_FORM, fire_flash_2, stage=1.36, power="tall"), Pose(bob=-1.0, body_lean=0.08, arm_front_angle=90, arm_back_angle=20, leg_front_angle=15, leg_back_angle=-11), 0.88, 0.56, 3, False),
            (_transition_form(FIRE_FORM, fire_flash_3, stage=1.62, power="fire"), Pose(bob=-1.18, body_lean=0.11, arm_front_angle=98, arm_back_angle=18, leg_front_angle=17, leg_back_angle=-12), 1.22, 0.92, 3, False),
            (_transition_form(FIRE_FORM, fire_flash_4, stage=1.86, power="fire"), Pose(bob=-1.32, body_lean=0.13, arm_front_angle=106, arm_back_angle=20, leg_front_angle=18, leg_back_angle=-13), 1.48, 1.18, 3, False),
            (_transition_form(FIRE_FORM, fire_blast, stage=2.00, power="fire"), Pose(bob=-1.40, body_lean=0.14, arm_front_angle=112, arm_back_angle=22, leg_front_angle=20, leg_back_angle=-15), 1.62, 1.34, 3, False),
            (_transition_form(FIRE_FORM, fire_reveal_1, stage=1.94, power="fire"), Pose(bob=-1.08, body_lean=0.14, arm_front_angle=108, arm_back_angle=18, leg_front_angle=18, leg_back_angle=-12), 1.38, 1.18, 3, False),
            (_transition_form(FIRE_FORM, fire_reveal_2, stage=1.98, power="fire"), Pose(bob=-0.72, body_lean=0.12, arm_front_angle=86, arm_back_angle=6, leg_front_angle=12, leg_back_angle=-8), 1.08, 1.02, 3, False),
            (_transition_form(FIRE_FORM, fire_reveal_3, stage=2.00, power="fire"), Pose(bob=-0.45, body_lean=0.10, arm_front_angle=70, arm_back_angle=-4, leg_front_angle=10, leg_back_angle=-6), 0.94, 0.96, 3, True),
            (FIRE_FORM, TALL_LIKE_POSES["fireball"][0], 0.90, 1.0, 3, True),
        ]
        active_form, pose, wing_boost, sleeve_wing_boost, extra_star_phase, show_orb = transform_seq[frame_idx % len(transform_seq)]

        def painter(px) -> None:
            _draw_transform_aura(px, frame_idx)
            _draw_side_pose(
                px,
                active_form,
                pose,
                animation="transform",
                wing_boost=wing_boost,
                sleeve_wing_boost=sleeve_wing_boost,
                extra_star_phase=extra_star_phase,
            )
            if show_orb:
                _draw_fire_orb(px, 19.4, 13.2 + 0.3 * math.sin(frame_idx))

        sprite = rasterize_logical(LOGICAL_SIZE, SCALE, painter)
        return bottom_center_canvas(sprite, FRAME_SIZE)

    if animation == "shrink":
        # Two hosts, two clips: the TALL sheet's shrink is "fire became tall"
        # and the SHORT sheet's is "tall became small". Both end on the sheet's
        # own form, which is what makes the arriving-sheet rule hold.
        if form.power == "tall":
            fire_dull_1 = _mix_outfit_palette(MARY_FIRE, MARY_NORMAL, 0.22)
            fire_dull_2 = _mix_outfit_palette(MARY_FIRE, MARY_NORMAL, 0.46)
            fire_dull_3 = _mix_outfit_palette(MARY_FIRE, MARY_NORMAL, 0.72)
            hurt_seq = [
                (FIRE_FORM, Pose(mode="fireball", bob=0.1, arm_front_angle=35, arm_back_angle=-18, leg_front_angle=-12, leg_back_angle=22), 0.85, 0.95, 2),
                (_transition_form(FIRE_FORM, fire_dull_1, stage=1.82, power="fire"), Pose(bob=0.35, body_lean=-0.1, arm_front_angle=24, arm_back_angle=-36, leg_front_angle=-8, leg_back_angle=18), 0.55, 0.70, 1),
                (_transition_form(FIRE_FORM, fire_dull_2, stage=1.56, power="fire"), Pose(bob=0.7, body_lean=-0.18, arm_front_angle=10, arm_back_angle=-58, leg_front_angle=5, leg_back_angle=10), 0.20, 0.35, 1),
                (_transition_form(FIRE_FORM, fire_dull_3, stage=1.28, power="tall"), Pose(bob=1.0, body_lean=-0.08, arm_front_angle=88, arm_back_angle=-80, leg_front_angle=14, leg_back_angle=4), 0.0, 0.08, 0),
                (_transition_form(TALL_FORM, _mix_outfit_palette(MARY_NORMAL, MARY_FIRE, 0.18), stage=1.06), Pose(bob=0.75, body_lean=0.02, arm_front_angle=118, arm_back_angle=-48, leg_front_angle=10, leg_back_angle=-2), 0.0, 0.0, 0),
                (TALL_FORM, Pose(bob=0.3, body_lean=0.0, arm_front_angle=52, arm_back_angle=-12, leg_front_angle=0, leg_back_angle=0), 0.0, 0.0, 0),
            ]
            active_form, pose, wing_boost, sleeve_wing_boost, extra_star_phase = hurt_seq[frame_idx % len(hurt_seq)]

            def painter(px) -> None:
                _draw_power_loss_sparkles(px, frame_idx, fire=True)
                _draw_side_pose(
                    px,
                    active_form,
                    pose,
                    animation="shrink",
                    wing_boost=wing_boost,
                    sleeve_wing_boost=sleeve_wing_boost,
                    extra_star_phase=extra_star_phase,
                )

            sprite = rasterize_logical(LOGICAL_SIZE, SCALE, painter)
            return bottom_center_canvas(sprite, FRAME_SIZE)
        else:
            tall_dull = _mix_outfit_palette(MARY_NORMAL, MARY_FIRE_FLASH, 0.06)
            hurt_seq = [
                (TALL_FORM, Pose(bob=0.2, body_lean=-0.06, arm_front_angle=24, arm_back_angle=-18, leg_front_angle=-10, leg_back_angle=18), 1),
                (SHORT_FORM, Pose(bob=0.55, body_lean=-0.02, arm_front_angle=40, arm_back_angle=-18, leg_front_angle=-4, leg_back_angle=10), 0),
                (_form_with_palette(TALL_FORM, tall_dull), Pose(bob=0.85, body_lean=-0.10, arm_front_angle=88, arm_back_angle=-54, leg_front_angle=8, leg_back_angle=6), 0),
                (SHORT_FORM, Pose(bob=0.35, body_lean=0.0, arm_front_angle=46, arm_back_angle=-10, leg_front_angle=0, leg_back_angle=0), 0),
            ]
            active_form, pose, extra_star_phase = hurt_seq[frame_idx % len(hurt_seq)]

            def painter(px) -> None:
                _draw_power_loss_sparkles(px, frame_idx, fire=False)
                _draw_side_pose(px, active_form, pose, animation="shrink", extra_star_phase=extra_star_phase)

            sprite = rasterize_logical(LOGICAL_SIZE, SCALE, painter)
            return bottom_center_canvas(sprite, FRAME_SIZE)

    if animation == "big_shrink":
        # Hosted by the SHORT sheet: fire loses two tiers at once and arrives
        # small. No power guard — the sheet that owns the clip is the one it
        # ends on, and only the short sheet lists this row.
        fire_dull_1 = _mix_outfit_palette(MARY_FIRE, MARY_NORMAL, 0.24)
        fire_dull_2 = _mix_outfit_palette(MARY_FIRE, MARY_NORMAL, 0.50)
        fire_dull_3 = _mix_outfit_palette(MARY_FIRE, MARY_NORMAL, 0.78)
        big_shrink_seq = [
            (FIRE_FORM, Pose(mode="fireball", bob=0.1, arm_front_angle=35, arm_back_angle=-18, leg_front_angle=-12, leg_back_angle=22), 0.95, 1.05, 2),
            (_transition_form(FIRE_FORM, fire_dull_1, stage=1.82, power="fire"), Pose(bob=0.35, body_lean=-0.1, arm_front_angle=24, arm_back_angle=-36, leg_front_angle=-8, leg_back_angle=18), 0.60, 0.72, 1),
            (_transition_form(FIRE_FORM, fire_dull_2, stage=1.48, power="fire"), Pose(bob=0.7, body_lean=-0.16, arm_front_angle=12, arm_back_angle=-58, leg_front_angle=4, leg_back_angle=12), 0.18, 0.32, 0),
            (_transition_form(FIRE_FORM, fire_dull_3, stage=1.16, power="tall"), Pose(bob=0.95, body_lean=-0.08, arm_front_angle=84, arm_back_angle=-76, leg_front_angle=12, leg_back_angle=4), 0.0, 0.0, 0),
            (TALL_FORM, Pose(bob=0.55, body_lean=-0.02, arm_front_angle=58, arm_back_angle=-22, leg_front_angle=-2, leg_back_angle=8), 0.0, 0.0, 0),
            (SHORT_FORM, Pose(bob=0.78, body_lean=-0.02, arm_front_angle=36, arm_back_angle=-18, leg_front_angle=-2, leg_back_angle=8), 0.0, 0.0, 0),
            (TALL_FORM, Pose(bob=0.48, body_lean=0.0, arm_front_angle=72, arm_back_angle=-28, leg_front_angle=4, leg_back_angle=2), 0.0, 0.0, 0),
            (SHORT_FORM, Pose(bob=0.25, body_lean=0.0, arm_front_angle=46, arm_back_angle=-10, leg_front_angle=0, leg_back_angle=0), 0.0, 0.0, 0),
        ]
        active_form, pose, wing_boost, sleeve_wing_boost, extra_star_phase = big_shrink_seq[frame_idx % len(big_shrink_seq)]

        def painter(px) -> None:
            _draw_power_loss_sparkles(px, frame_idx, fire=True)
            _draw_side_pose(
                px,
                active_form,
                pose,
                animation="big_shrink",
                wing_boost=wing_boost,
                sleeve_wing_boost=sleeve_wing_boost,
                extra_star_phase=extra_star_phase,
            )

        sprite = rasterize_logical(LOGICAL_SIZE, SCALE, painter)
        return bottom_center_canvas(sprite, FRAME_SIZE)

    pose_seq = _poses_for(form).get(animation) or SHORT_POSES["idle"]
    pose = pose_seq[frame_idx % len(pose_seq)]

    def painter(px) -> None:
        if pose.mode == "dead":
            _draw_dead_front(px, form, pose)
        else:
            _draw_side_pose(px, form, pose, animation=animation)

    sprite = rasterize_logical(LOGICAL_SIZE, SCALE, painter)
    return bottom_center_canvas(sprite, FRAME_SIZE)


def _actor_metadata(form: FormSpec) -> dict:
    metadata = copy.deepcopy(ACTOR_METADATA_BASE)
    metadata.update(
        {
            "actor": {
                "character_id": f"pc_{form.target_name}",
                "display_name": form.display_name,
            },
            "body": {
                **ACTOR_METADATA_BASE["body"],
                "body_kind": "Tall" if form.tall else "Compact",
                "traits": ["hero", "retro", "platformer", form.power],
            },
            "sockets": {
                "head": {"source": f"{form.target_name}.geometry", "point": {"x": 39.0, "y": 16.0 if form.tall else 20.0}},
                "hand_r": {"source": f"{form.target_name}.geometry", "point": {"x": 58.0, "y": 54.0}},
                "hand_l": {"source": f"{form.target_name}.geometry", "point": {"x": 23.0, "y": 54.0}},
                "foot_r": {"source": f"{form.target_name}.geometry", "point": {"x": 49.0, "y": 88.0}},
                "foot_l": {"source": f"{form.target_name}.geometry", "point": {"x": 35.0, "y": 88.0}},
            },
            "tags": [*ACTOR_METADATA_BASE["tags"], form.power],
            "authoring_description": (
                "Mary-O v2 is an additive second-draft reinterpretation of the accepted "
                "Super Mary-O family. It preserves the platformer movement vocabulary and "
                "form progression while redrafting the silhouette, cap, bodice, skirt, "
                "boots, wing language, and fire-form ornamentation as a coherent costume."
            ),
            "gameplay_description": (
                f"Use the {form.display_name} sheet as a responsive retro-platform hero "
                f"in her {form.power} state. Games may opt into running, jumping, skidding, "
                "climbing, swimming, growth, or fireball actions according to the form's "
                "published animation set."
            ),
            "dialogue_hints": {
                "barks": [
                    "A clear jump is a kind of argument.",
                    "The level can keep its royal road. I brought running shoes.",
                    "One more platform.",
                ]
            },
        }
    )
    bindings = metadata["animation_bindings"]
    if form.tall:
        bindings["locomotion.crouch"] = {"animation": "crouch", "events": []}
    # Each sheet publishes the transitions that ARRIVE at it (see the row
    # tables): the short form knows how it was shrunk into, the tall form knows
    # how it was grown or dropped into, and the fire form knows how it was
    # transformed into.
    if form.power == "short":
        bindings["power.shrink"] = {"animation": "shrink", "events": []}
        bindings["power.big_shrink"] = {"animation": "big_shrink", "events": []}
    if form.power == "tall":
        bindings["power.grow"] = {"animation": "grow", "events": []}
        bindings["power.shrink"] = {"animation": "shrink", "events": []}
    if form.power == "fire":
        bindings["ability.fireball"] = {"animation": "fireball", "events": []}
        bindings["power.transform"] = {"animation": "transform", "events": []}
    return metadata


def _render_form(form: FormSpec, out_dir: str | Path) -> List[Path]:
    out_dir = Path(out_dir)
    out_dir.mkdir(parents=True, exist_ok=True)

    def render_frame(animation: str, frame_idx: int, nframes: int) -> Image.Image:
        return _draw_form(form, animation, frame_idx, nframes)

    outputs = build_sheet(
        target=form.target_name,
        rows=form.rows,
        render_fn=render_frame,
        out_dir=out_dir,
        frame_size=FRAME_SIZE,
        label_width=LABEL_WIDTH,
        auto_crop=False,
        actor_metadata=_actor_metadata(form),
        trim=False,
    )
    return [
        outputs[k]
        for k in (
            "canonical",
            "canonical_transparent",
            "spritesheet",
            "yaml",
            "ron",
            "actor",
            "preview",
        )
    ]


def render_mary_o_v2(out_dir: str | Path, **opts) -> List[Path]:
    return _render_form(SHORT_FORM, out_dir)


def render_mary_o_v2_tall(out_dir: str | Path, **opts) -> List[Path]:
    return _render_form(TALL_FORM, out_dir)


def render_mary_o_v2_fire(out_dir: str | Path, **opts) -> List[Path]:
    return _render_form(FIRE_FORM, out_dir)


TARGETS = {
    SHORT_FORM.target_name: {"render": render_mary_o_v2, "actor_metadata": _actor_metadata(SHORT_FORM)},
    TALL_FORM.target_name: {"render": render_mary_o_v2_tall, "actor_metadata": _actor_metadata(TALL_FORM)},
    FIRE_FORM.target_name: {"render": render_mary_o_v2_fire, "actor_metadata": _actor_metadata(FIRE_FORM)},
}


def render(out_dir: str | Path, **opts) -> List[Path]:
    return render_mary_o_v2(out_dir, **opts)
