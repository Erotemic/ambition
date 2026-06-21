from __future__ import annotations

"""Standalone generator for a crouched skulking ghoul enemy.

Visual inspiration:
- hunched, creeping humanoid silhouette
- long hooked nose and moustache tendrils
- simple cap / hood flap
- loincloth and bony limbs
- clawing hands and bent stalking legs

This is intentionally *not* a pirate. It is a creepy dark-lord-adjacent minion /
wretch enemy with a sneaky, crouched stance suitable for a side scroller or
arena enemy roster.

Only ``build_sheet`` is reused for spritesheet / YAML / RON emission.
"""

import argparse
import math
from pathlib import Path
from typing import List, Sequence, Tuple

from PIL import Image, ImageDraw

from ...authoring.tackon_sheet import build_sheet

RGBA = Tuple[int, int, int, int]
Point = Tuple[float, float]

TARGET_NAME = "ghoul_skulker"
# Files the tack-on installer copies into the sandbox sprites dir.
# Names match what `build_sheet` writes (target_spritesheet.{png,yaml,ron}).
SHEET_FILES = [
    f"{TARGET_NAME}_spritesheet.png",
    f"{TARGET_NAME}_spritesheet.yaml",
    f"{TARGET_NAME}_spritesheet.ron",
    f"{TARGET_NAME}_actor.ron",
]
FRAME_SIZE = (320, 320)
WORK_FRAME_SIZE = (640, 640)
SUPER = 4
ROWS: List[Tuple[str, int, int]] = [
    ("idle", 6, 130),
    ("skulk", 8, 95),
    ("claw", 7, 82),
    ("pounce", 6, 78),
    ("cackle", 6, 108),
    ("hurt", 4, 90),
    ("death", 8, 112),
]

ACTOR_METADATA = {
    "actor": {
        "character_id": "npc_ghoul_skulker",
        "display_name": "Ghoul Skulker",
    },
    "body": {
        "body_plan": "HumanoidBiped",
        "body_kind": "LowProfile",
        "mass_class": "Light",
        "locomotion_hint": "Skulk",
        "traits": ["enemy", "undead", "skulker", "clawed", "low_profile"],
    },
    "capabilities": {
        "traversal": {
            "walk": True,
            "jump": {
                "height_px": None,
                "distance_px": None,
                "source": "ghoul_pounce_animation",
            },
            "climb": None,
            "crawl": True,
            "fly": None,
            "swim": None,
            "use_lifts": None,
            "door_access": [],
        },
        "interactions": {
            "talk": None,
            "trade": None,
            "carry": None,
            "open_doors": [],
        },
    },
    "brain": {"default_preset": "melee_brute_striker"},
    "actions": {"default_preset": "striker_swipe"},
    "animation_bindings": {
        "default": {"animation": "idle", "events": []},
        "locomotion.skulk": {"animation": "skulk", "events": []},
        "action.melee.primary": {
            "animation": "claw",
            "events": [
                {
                    "t": 0.34,
                    "event": "hitbox_active_start",
                    "source": "ghoul_skulker.claw",
                },
                {
                    "t": 0.58,
                    "event": "hitbox_active_end",
                    "source": "ghoul_skulker.claw",
                },
            ],
        },
        "action.special.pounce": {
            "animation": "pounce",
            "events": [
                {"t": 0.25, "event": "leap_commit", "source": "ghoul_skulker.pounce"},
                {
                    "t": 0.54,
                    "event": "hitbox_active_start",
                    "source": "ghoul_skulker.pounce",
                },
                {
                    "t": 0.70,
                    "event": "hitbox_active_end",
                    "source": "ghoul_skulker.pounce",
                },
            ],
        },
        "interaction.cackle": {"animation": "cackle", "events": []},
        "damage.hit": {"animation": "hurt", "events": []},
        "lifecycle.death": {"animation": "death", "events": []},
    },
    "sockets": {
        "head": {"source": "ghoul_skulker.geometry", "point": {"x": 166.0, "y": 82.0}},
        "mouth": {
            "source": "ghoul_skulker.geometry",
            "point": {"x": 188.0, "y": 102.0},
        },
        "hand_l": {
            "source": "ghoul_skulker.geometry",
            "point": {"x": 90.0, "y": 198.0},
        },
        "hand_r": {
            "source": "ghoul_skulker.geometry",
            "point": {"x": 238.0, "y": 194.0},
        },
        "claw_tip": {
            "source": "ghoul_skulker.geometry",
            "point": {"x": 254.0, "y": 202.0},
        },
        "pounce_origin": {
            "source": "ghoul_skulker.geometry",
            "point": {"x": 170.0, "y": 245.0},
        },
    },
    "tags": ["enemy", "undead", "skulker"],
}

OUTLINE = (24, 20, 19, 255)
SKIN = (214, 205, 192, 255)
SKIN_SHADE = (164, 149, 139, 255)
SKIN_DARK = (118, 104, 95, 255)
LIP = (110, 70, 72, 255)
MOUTH = (70, 40, 46, 255)
TEETH = (246, 242, 230, 255)
EYE = (238, 234, 220, 255)
PUPIL = (24, 21, 22, 255)
CAP = (146, 112, 92, 255)
CAP_SHADE = (103, 78, 64, 255)
CLOTH = (122, 110, 96, 255)
CLOTH_HI = (160, 148, 132, 255)
NAIL = (212, 205, 188, 255)
DUST = (124, 112, 96, 150)
FX = (245, 236, 160, 150)


def _s(v: float) -> int:
    return int(round(v * SUPER))


def _pt(p: Point) -> Tuple[int, int]:
    return (_s(p[0]), _s(p[1]))


def _box(cx: float, cy: float, rx: float, ry: float) -> Tuple[int, int, int, int]:
    return (_s(cx - rx), _s(cy - ry), _s(cx + rx), _s(cy + ry))


def _lerp(a: float, b: float, t: float) -> float:
    return a + (b - a) * t


def _ease(t: float) -> float:
    t = max(0.0, min(1.0, t))
    return 0.5 - 0.5 * math.cos(math.pi * t)


def _rot(x: float, y: float, deg: float) -> Point:
    rad = math.radians(deg)
    c = math.cos(rad)
    s = math.sin(rad)
    return (x * c - y * s, x * s + y * c)


def _poly(
    draw: ImageDraw.ImageDraw,
    pts: Sequence[Point],
    fill: RGBA,
    outline: RGBA = OUTLINE,
    width: float = 1.0,
) -> None:
    ipts = [_pt(p) for p in pts]
    draw.polygon(ipts, fill=fill)
    if outline and width > 0:
        draw.line(
            ipts + [ipts[0]], fill=outline, width=max(1, _s(width)), joint="curve"
        )


def _line(
    draw: ImageDraw.ImageDraw, pts: Sequence[Point], fill: RGBA, width: float = 1.0
) -> None:
    draw.line([_pt(p) for p in pts], fill=fill, width=max(1, _s(width)), joint="curve")


def _circle(
    draw: ImageDraw.ImageDraw,
    p: Point,
    r: float,
    fill: RGBA,
    outline: RGBA = OUTLINE,
    width: float = 1.0,
) -> None:
    draw.ellipse(
        (_s(p[0] - r), _s(p[1] - r), _s(p[0] + r), _s(p[1] + r)),
        fill=fill,
        outline=outline,
        width=max(1, _s(width)),
    )


def _ellipse(
    draw: ImageDraw.ImageDraw,
    cx: float,
    cy: float,
    rx: float,
    ry: float,
    fill: RGBA,
    outline: RGBA = OUTLINE,
    width: float = 1.0,
) -> None:
    draw.ellipse(
        _box(cx, cy, rx, ry), fill=fill, outline=outline, width=max(1, _s(width))
    )


def _downsample(img: Image.Image) -> Image.Image:
    return img.resize(FRAME_SIZE, Image.Resampling.LANCZOS)


class Pose:
    def __init__(self, anim: str, frame_idx: int, nframes: int) -> None:
        t = frame_idx / max(1, nframes - 1)
        cyc = math.tau * frame_idx / max(1, nframes)
        s = math.sin(cyc)
        c = math.cos(cyc)

        self.root_x = 0.0
        self.root_y = 0.0
        self.bob = 0.0
        self.lean = 0.0
        self.head_tilt = 0.0
        self.left_arm = 0.0
        self.right_arm = 0.0
        self.left_leg = 0.0
        self.right_leg = 0.0
        self.left_lift = 0.0
        self.right_lift = 0.0
        self.hand_spread = 0.0
        self.nose_pitch = 0.0
        self.cap_swing = 0.0
        self.mouth = 0.0
        self.dead_t = 0.0
        self.impact = 0.0
        self.blink = False
        self.x_eye = False

        if anim == "idle":
            self.root_x = s * 1.2
            self.bob = s * 1.8
            self.lean = -6.0 + s * 2.0
            self.head_tilt = -6.0 + s * 1.8
            self.left_arm = -6.0 + s * 5.0
            self.right_arm = 8.0 - s * 4.0
            self.left_leg = -3.0 + c * 2.0
            self.right_leg = 4.0 - c * 1.5
            self.hand_spread = 1.0 + max(0.0, s) * 2.0
            self.cap_swing = -s * 6.0
            self.mouth = max(0.0, s) * 0.06
            self.blink = frame_idx == nframes - 2
        elif anim == "skulk":
            self.root_x = s * 2.2
            self.bob = abs(s) * 2.8 - 0.6
            self.lean = -10.0 + s * 3.0
            self.head_tilt = -10.0 - s * 3.0
            self.left_leg = -24.0 * s
            self.right_leg = 22.0 * s
            self.left_lift = max(0.0, -s) * 10.0
            self.right_lift = max(0.0, s) * 9.0
            self.left_arm = 18.0 * s - 10.0
            self.right_arm = -16.0 * s + 7.0
            self.hand_spread = 2.0 + abs(s) * 2.0
            self.cap_swing = -s * 9.0
            self.nose_pitch = -s * 4.0
        elif anim == "claw":
            tt = _ease(t)
            hit = math.sin(tt * math.pi)
            self.root_x = _lerp(-10.0, 12.0, tt)
            self.bob = -hit * 4.0
            self.lean = _lerp(-18.0, 18.0, tt)
            self.head_tilt = _lerp(-16.0, 12.0, tt)
            self.left_arm = _lerp(-34.0, 44.0, tt)
            self.right_arm = _lerp(10.0, -28.0, tt)
            self.left_leg = _lerp(-8.0, 12.0, tt)
            self.right_leg = _lerp(10.0, -4.0, tt)
            self.hand_spread = 3.0 + hit * 6.0
            self.cap_swing = _lerp(12.0, -16.0, tt)
            self.nose_pitch = _lerp(-8.0, 8.0, tt)
            self.mouth = 0.18 + hit * 0.14
            self.impact = hit
        elif anim == "pounce":
            tt = _ease(t)
            self.root_x = _lerp(-16.0, 18.0, tt)
            self.root_y = _lerp(4.0, -14.0, tt)
            self.bob = -math.sin(tt * math.pi) * 3.0
            self.lean = _lerp(-18.0, 24.0, tt)
            self.head_tilt = _lerp(-14.0, 16.0, tt)
            self.left_arm = _lerp(-20.0, 36.0, tt)
            self.right_arm = _lerp(-10.0, 30.0, tt)
            self.left_leg = _lerp(-18.0, 20.0, tt)
            self.right_leg = _lerp(-6.0, 16.0, tt)
            self.left_lift = _lerp(0.0, 10.0, tt)
            self.right_lift = _lerp(0.0, 6.0, tt)
            self.hand_spread = 3.0 + tt * 5.0
            self.cap_swing = _lerp(10.0, -12.0, tt)
            self.nose_pitch = _lerp(-6.0, 6.0, tt)
            self.mouth = 0.20 + tt * 0.12
            self.impact = math.sin(tt * math.pi)
        elif anim == "cackle":
            self.root_x = s * 1.0
            self.bob = s * 2.0
            self.lean = -8.0 + s * 6.0
            self.head_tilt = -6.0 - s * 5.0
            self.left_arm = -28.0 + s * 10.0
            self.right_arm = -34.0 - s * 9.0
            self.left_leg = -2.0
            self.right_leg = 6.0
            self.hand_spread = 4.0 + max(0.0, s) * 5.0
            self.cap_swing = s * 8.0
            self.nose_pitch = s * 4.0
            self.mouth = 0.30 + max(0.0, s) * 0.18
        elif anim == "hurt":
            hit = math.sin(t * math.pi)
            shake = math.sin(t * math.pi * 5.0) * (1.0 - t)
            self.root_x = shake * 4.0 - hit * 4.0
            self.bob = -hit * 2.5
            self.lean = -18.0 * hit
            self.head_tilt = 10.0 * hit
            self.left_arm = 20.0 * hit
            self.right_arm = 18.0 * hit
            self.left_leg = -10.0 * hit
            self.right_leg = 8.0 * hit
            self.hand_spread = 2.0 + hit * 3.0
            self.cap_swing = -14.0 * hit
            self.mouth = 0.16 * hit
        elif anim == "death":
            tt = _ease(t)
            self.dead_t = tt
            self.root_x = tt * 20.0
            self.root_y = tt * 8.0
            self.bob = -tt * 5.0
            self.lean = -82.0 * tt
            self.head_tilt = -28.0 * tt
            self.left_arm = _lerp(-4.0, 56.0, tt)
            self.right_arm = _lerp(6.0, -66.0, tt)
            self.left_leg = _lerp(-4.0, 26.0, tt)
            self.right_leg = _lerp(6.0, -22.0, tt)
            self.hand_spread = 2.0 + tt * 4.0
            self.cap_swing = -22.0 * tt
            self.mouth = 0.26 * tt
            self.x_eye = tt > 0.55


def _draw_hand(
    draw: ImageDraw.ImageDraw,
    hand: Point,
    ang: float,
    spread: float,
    *,
    front: bool = True,
) -> None:
    palm_r = 6.0 if front else 5.0
    _circle(draw, hand, palm_r, SKIN if front else SKIN_SHADE, OUTLINE, 0.8)
    finger_len = 12.0 if front else 10.0
    for i, base_ang in enumerate([-34, -10, 14, 34]):
        a = ang + base_ang + (i - 1.5) * spread * 0.7
        base = (
            hand[0] + math.cos(math.radians(a - 10)) * 4.0,
            hand[1] + math.sin(math.radians(a - 10)) * 4.0,
        )
        tip = (
            base[0] + math.cos(math.radians(a)) * finger_len,
            base[1] + math.sin(math.radians(a)) * finger_len,
        )
        _line(
            draw,
            [hand, base, tip],
            SKIN if front else SKIN_SHADE,
            2.8 if front else 2.3,
        )
        _line(draw, [hand, base, tip], OUTLINE, 0.6)
        _poly(
            draw,
            [
                tip,
                (
                    tip[0] + 4 * math.cos(math.radians(a - 18)),
                    tip[1] + 4 * math.sin(math.radians(a - 18)),
                ),
                (
                    tip[0] + 3 * math.cos(math.radians(a + 20)),
                    tip[1] + 3 * math.sin(math.radians(a + 20)),
                ),
            ],
            NAIL,
            OUTLINE,
            0.35,
        )


def _draw_foot(draw: ImageDraw.ImageDraw, foot: Point, facing: float) -> None:
    sole = [
        (foot[0] - 8, foot[1] - 3),
        (foot[0] + 10 + facing * 4, foot[1] - 4),
        (foot[0] + 16 + facing * 5, foot[1] + 3),
        (foot[0] + 8, foot[1] + 7),
        (foot[0] - 7, foot[1] + 5),
    ]
    _poly(draw, sole, SKIN_SHADE, OUTLINE, 0.8)
    for frac in [0.65, 0.82, 0.98]:
        toe = (foot[0] + (12 + facing * 4) * frac, foot[1] + 2)
        _poly(
            draw,
            [toe, (toe[0] + 4, toe[1] - 1), (toe[0] + 2, toe[1] + 3)],
            NAIL,
            OUTLINE,
            0.3,
        )


def _render_frame(anim: str, frame_idx: int, nframes: int) -> Image.Image:
    img = Image.new(
        "RGBA", (WORK_FRAME_SIZE[0] * SUPER, WORK_FRAME_SIZE[1] * SUPER), (0, 0, 0, 0)
    )
    draw = ImageDraw.Draw(img, "RGBA")
    pose = Pose(anim, frame_idx, nframes)

    root = (
        WORK_FRAME_SIZE[0] * 0.47 + pose.root_x + pose.dead_t * 8.0,
        WORK_FRAME_SIZE[1] * 0.75 + pose.root_y + pose.bob,
    )
    tilt = pose.lean

    def P(x: float, y: float) -> Point:
        rx, ry = _rot(x, y, tilt)
        return (root[0] + rx, root[1] + ry)

    # legs behind / base crouch
    left_hip = P(-14, -56)
    right_hip = P(20, -52)
    left_knee = P(-30 + pose.left_leg * 0.32, -12)
    right_knee = P(18 + pose.right_leg * 0.26, -6)
    left_foot = P(-42 + pose.left_leg * 0.22, 26 - pose.left_lift)
    right_foot = P(42 + pose.right_leg * 0.18, 28 - pose.right_lift)

    # far leg first
    _line(draw, [right_hip, right_knee, right_foot], SKIN_SHADE, 7.5)
    _line(draw, [right_hip, right_knee, right_foot], OUTLINE, 1.2)
    _circle(draw, right_knee, 5.2, SKIN_SHADE, OUTLINE, 0.6)
    _draw_foot(draw, right_foot, 1)

    # pelvis and torso
    pelvis = [P(-18, -66), P(14, -66), P(28, -44), P(12, -24), P(-12, -26), P(-26, -46)]
    _poly(draw, pelvis, SKIN_SHADE, OUTLINE, 1.0)
    loincloth = [
        P(-10, -60),
        P(16, -60),
        P(18, -25),
        P(4, -6),
        P(-10, -12),
        P(-14, -34),
    ]
    _poly(draw, loincloth, CLOTH, OUTLINE, 0.8)
    _line(draw, [P(-4, -54), P(4, -10)], CLOTH_HI, 0.8)
    torso = [
        P(-18, -116),
        P(18, -124),
        P(34, -102),
        P(28, -58),
        P(12, -40),
        P(-16, -46),
        P(-30, -84),
    ]
    _poly(draw, torso, SKIN, OUTLINE, 1.2)
    chest_l = [P(-14, -94), P(-6, -102), P(0, -90), P(-4, -78), P(-12, -78)]
    chest_r = [P(2, -94), P(12, -100), P(16, -86), P(12, -76), P(2, -78)]
    _poly(draw, chest_l, SKIN_SHADE, OUTLINE, 0.5)
    _poly(draw, chest_r, SKIN_SHADE, OUTLINE, 0.5)
    _circle(draw, P(-7, -86), 1.8, LIP, OUTLINE, 0.2)
    _circle(draw, P(8, -84), 1.8, LIP, OUTLINE, 0.2)
    _line(draw, [P(-5, -70), P(2, -66), P(10, -64)], SKIN_SHADE, 0.8)

    # far arm first
    right_shoulder = P(28, -104)
    right_elbow = P(48 + pose.right_arm * 0.10, -78 + pose.right_arm * 0.16)
    right_hand = P(58 + pose.right_arm * 0.28, -46 + pose.right_arm * 0.22)
    _line(draw, [right_shoulder, right_elbow], SKIN_SHADE, 6.2)
    _line(draw, [right_elbow, right_hand], SKIN_SHADE, 5.2)
    _line(draw, [right_shoulder, right_elbow, right_hand], OUTLINE, 1.0)
    _circle(draw, right_elbow, 4.8, SKIN_SHADE, OUTLINE, 0.5)
    _draw_hand(
        draw, right_hand, 12 + pose.right_arm * 0.3, pose.hand_spread, front=False
    )

    # head and cap
    head_root = P(0, -128)
    head_tilt = tilt + pose.head_tilt

    def H(x: float, y: float) -> Point:
        rx, ry = _rot(x, y, head_tilt)
        return (head_root[0] + rx, head_root[1] + ry)

    cap = [
        H(-16, -22),
        H(0, -40),
        H(22, -38),
        H(36 + pose.cap_swing * 0.35, -18 + pose.cap_swing * 0.12),
        H(26 + pose.cap_swing * 0.45, -4),
        H(8, -8),
        H(-10, -4),
    ]
    _poly(draw, cap, CAP, OUTLINE, 1.0)
    tail = [
        H(18, -28),
        H(44 + pose.cap_swing * 0.35, -42),
        H(66 + pose.cap_swing * 0.5, -26),
        H(34 + pose.cap_swing * 0.25, -10),
    ]
    _poly(draw, tail, CAP_SHADE, OUTLINE, 0.8)
    head = [
        H(-16, -14),
        H(-10, -26),
        H(6, -30),
        H(18, -20),
        H(20, -4),
        H(12, 10),
        H(-6, 14),
        H(-18, 2),
    ]
    _poly(draw, head, SKIN, OUTLINE, 1.1)

    nose = [
        H(10, -10),
        H(28, -12 + pose.nose_pitch * 0.2),
        H(42, -4 + pose.nose_pitch * 0.25),
        H(22, 0),
        H(14, 2),
    ]
    _poly(draw, nose, SKIN_SHADE, OUTLINE, 0.8)
    moust_l = [H(8, 0), H(-2, 4), H(-12, 2), H(-18, -4), H(-14, -8), H(-4, -5)]
    moust_r = [H(14, 0), H(22, 2), H(34, 0), H(40, -7), H(36, -12), H(24, -8)]
    _poly(draw, moust_l, SKIN_DARK, OUTLINE, 0.5)
    _poly(draw, moust_r, SKIN_DARK, OUTLINE, 0.5)

    if pose.x_eye:
        _line(draw, [H(-5, -10), H(3, -2)], OUTLINE, 0.9)
        _line(draw, [H(-5, -2), H(3, -10)], OUTLINE, 0.9)
    elif pose.blink:
        _line(draw, [H(-6, -7), H(2, -7)], OUTLINE, 0.9)
    else:
        _ellipse(draw, H(-2, -6)[0], H(-2, -6)[1], 3.8, 3.0, EYE, OUTLINE, 0.5)
        _circle(draw, H(-1, -6), 1.0, PUPIL, PUPIL, 0.1)
        _line(draw, [H(-8, -11), H(2, -12)], OUTLINE, 0.8)
    _line(draw, [H(15, -4), H(18, 2)], SKIN_DARK, 0.6)
    if pose.mouth > 0.16:
        _ellipse(
            draw,
            H(4, 8)[0],
            H(4, 8)[1],
            6.0,
            3.6 + pose.mouth * 3.0,
            MOUTH,
            OUTLINE,
            0.6,
        )
        _poly(draw, [H(0, 8), H(4, 13), H(8, 8)], TEETH, OUTLINE, 0.25)
    else:
        _line(draw, [H(0, 8), H(6, 10), H(12, 8)], LIP, 0.8)

    # front leg on top
    _line(draw, [left_hip, left_knee, left_foot], SKIN, 8.6)
    _line(draw, [left_hip, left_knee, left_foot], OUTLINE, 1.4)
    _circle(draw, left_knee, 5.8, SKIN, OUTLINE, 0.6)
    _draw_foot(draw, left_foot, 1)

    # front arm on top
    left_shoulder = P(-18, -106)
    left_elbow = P(-52 + pose.left_arm * 0.12, -74 + pose.left_arm * 0.18)
    left_hand = P(-70 + pose.left_arm * 0.34, -42 + pose.left_arm * 0.28)
    _line(draw, [left_shoulder, left_elbow], SKIN, 7.0)
    _line(draw, [left_elbow, left_hand], SKIN, 5.8)
    _line(draw, [left_shoulder, left_elbow, left_hand], OUTLINE, 1.1)
    _circle(draw, left_elbow, 5.0, SKIN, OUTLINE, 0.5)
    _draw_hand(draw, left_hand, 186 - pose.left_arm * 0.3, pose.hand_spread, front=True)

    if anim in {"claw", "pounce"} and pose.impact > 0.2:
        hx, hy = left_hand
        box = (_s(hx - 44), _s(hy - 26), _s(hx + 40), _s(hy + 36))
        draw.arc(box, 168, 308, fill=FX, width=_s(3.8))
    if anim in {"skulk", "pounce"} and (pose.left_lift > 1.0 or pose.right_lift > 1.0):
        for i, (dx, dy) in enumerate([(-26, 0), (-10, 4), (12, 1), (30, 5)]):
            c = P(dx, 34 + dy)
            _poly(
                draw,
                [
                    (c[0] - 2, c[1]),
                    (c[0], c[1] - 4),
                    (c[0] + 3, c[1] - 1),
                    (c[0] + 1, c[1] + 2),
                ],
                DUST,
                (88, 80, 70, 100),
                0.25,
            )

    return _downsample(img)


def render(out_dir: str | Path, **opts) -> List[Path]:
    out_dir = Path(out_dir)
    out_dir.mkdir(parents=True, exist_ok=True)
    outputs = build_sheet(
        target=TARGET_NAME,
        rows=ROWS,
        render_fn=lambda anim, frame_idx, nframes: _render_frame(
            anim, frame_idx, nframes
        ),
        out_dir=out_dir,
        frame_size=opts.get("frame_size", FRAME_SIZE),
        crop_margin=10,
        auto_crop=True,
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


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        description="Render the standalone Ghoul Skulker sprite sheet."
    )
    parser.add_argument(
        "--out-dir",
        type=Path,
        default=Path(__file__).resolve().parents[2] / "generated" / TARGET_NAME,
    )
    args = parser.parse_args(argv)
    for path in render(args.out_dir):
        print(path)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
