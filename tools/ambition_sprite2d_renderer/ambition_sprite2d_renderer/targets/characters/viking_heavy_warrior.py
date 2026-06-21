from __future__ import annotations

"""Standalone generator for a heavy Viking warrior sprite sheet.

Redesigned from scratch with a broad opera / saga silhouette:
- giant horned steel helmet
- thick beard and barrel torso
- big beefy arms and short strong legs
- heavy double axe and exaggerated bellow poses
- clearly distinct from slimmer Viking targets

Generator only. No registration or GUI wiring.
"""

import argparse
import math
from pathlib import Path
from typing import List, Sequence, Tuple

from PIL import Image, ImageDraw

from ...authoring.tackon_sheet import build_sheet

ACTOR_METADATA = {
    "actor": {
        "character_id": "npc_viking_heavy_warrior",
        "display_name": "Viking Heavy Warrior",
    },
    "body": {
        "body_plan": "HumanoidBiped",
        "body_kind": "Wide",
        "mass_class": "Heavy",
        "traits": ["story", "humanoid", "enemy", "combatant", "viking"],
        "locomotion_hint": "Walk",
    },
    "capabilities": {
        "traversal": {
            "walk": True,
            "jump": None,
            "climb": None,
            "fly": None,
            "swim": None,
            "crawl": None,
            "use_lifts": True,
            "door_access": ["public"],
        },
        "interactions": {
            "talk": True,
            "trade": None,
            "carry": None,
            "open_doors": ["public"],
        },
    },
    "brain": {"default_preset": "melee_brute_brute"},
    "actions": {"default_preset": "brute_lunge"},
    "visual": {"default_pose": "idle"},
    "tags": ["story", "humanoid", "enemy", "combatant", "viking"],
    "sockets": {
        "head": {
            "source": "explicit.profile.humanoid",
            "point": {"x": 64.0, "y": 24.0},
        },
        "chest": {
            "source": "explicit.profile.humanoid",
            "point": {"x": 64.0, "y": 54.0},
        },
        "hand_l": {
            "source": "explicit.profile.humanoid",
            "point": {"x": 48.0, "y": 64.0},
        },
        "hand_r": {
            "source": "explicit.profile.humanoid",
            "point": {"x": 80.0, "y": 64.0},
        },
        "speech_bubble": {
            "source": "explicit.profile.humanoid",
            "point": {"x": 64.0, "y": 8.0},
        },
        "weapon_grip": {
            "source": "explicit.profile.combat_humanoid",
            "point": {"x": 80.0, "y": 64.0},
        },
        "weapon_tip": {
            "source": "explicit.profile.combat_humanoid",
            "point": {"x": 104.0, "y": 60.0},
        },
    },
    "animation_bindings": {
        "default": {"animation": "idle", "events": []},
        "locomotion.walk": {"animation": "walk", "events": []},
        "interaction.talk": {"animation": "talk", "events": []},
        "interaction.use": {"animation": "interact", "events": []},
        "action.melee.primary": {
            "animation": "slash",
            "events": [
                {
                    "t": 0.34,
                    "event": "hitbox_active_start",
                    "source": "explicit.profile.combat_humanoid",
                },
                {
                    "t": 0.58,
                    "event": "hitbox_active_end",
                    "source": "explicit.profile.combat_humanoid",
                },
            ],
        },
    },
}


RGBA = Tuple[int, int, int, int]
Point = Tuple[float, float]

TARGET_NAME = "viking_heavy_warrior"
FRAME_SIZE = (320, 320)
WORK_FRAME_SIZE = (760, 760)
SUPER = 4
ROWS: List[Tuple[str, int, int]] = [
    ("idle", 6, 132),
    ("march", 8, 98),
    ("axe_cleave", 7, 82),
    ("helm_bash", 7, 84),
    ("bass_bellow", 6, 106),
    ("hurt", 4, 92),
    ("death", 8, 112),
]

OUTLINE = (26, 20, 18, 255)
SKIN = (222, 176, 136, 255)
SKIN_SHADE = (184, 136, 100, 255)
HAIR = (184, 104, 52, 255)
HAIR_SHADE = (132, 72, 38, 255)
BEARD = (146, 80, 38, 255)
BEARD_SHADE = (112, 60, 30, 255)
STEEL = (186, 196, 208, 255)
STEEL_SHADE = (126, 138, 150, 255)
HORN = (235, 228, 176, 255)
HORN_SHADE = (198, 188, 132, 255)
FUR = (224, 212, 196, 255)
FUR_SHADE = (182, 166, 150, 255)
TUNIC = (96, 128, 184, 255)
TUNIC_SHADE = (70, 98, 150, 255)
LEATHER = (130, 88, 54, 255)
LEATHER_DARK = (92, 60, 38, 255)
GOLD = (228, 186, 64, 255)
PANTS = (110, 96, 80, 255)
PANTS_SHADE = (84, 72, 58, 255)
SANDAL = (76, 54, 34, 255)
WOOD = (124, 88, 54, 255)
EYE = (248, 246, 238, 255)
PUPIL = (40, 38, 42, 255)
MOUTH = (102, 44, 48, 255)
TONGUE = (210, 104, 118, 255)
FX = (248, 238, 188, 148)
DUST = (136, 118, 92, 132)


def _s(v: float) -> int:
    return int(round(v * SUPER))


def _pt(p: Point) -> Tuple[int, int]:
    return (_s(p[0]), _s(p[1]))


def _box(cx: float, cy: float, rx: float, ry: float) -> Tuple[int, int, int, int]:
    return (_s(cx - rx), _s(cy - ry), _s(cx + rx), _s(cy + ry))


def _rot(x: float, y: float, deg: float) -> Point:
    rad = math.radians(deg)
    c = math.cos(rad)
    s = math.sin(rad)
    return (x * c - y * s, x * s + y * c)


def _lerp(a: float, b: float, t: float) -> float:
    return a + (b - a) * t


def _ease(t: float) -> float:
    t = max(0.0, min(1.0, t))
    return 0.5 - 0.5 * math.cos(math.pi * t)


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


def _circle(
    draw: ImageDraw.ImageDraw,
    p: Point,
    r: float,
    fill: RGBA,
    outline: RGBA = OUTLINE,
    width: float = 1.0,
) -> None:
    _ellipse(draw, p[0], p[1], r, r, fill, outline, width)


def _downsample(img: Image.Image) -> Image.Image:
    return img.resize(FRAME_SIZE, Image.Resampling.LANCZOS)


class Pose:
    def __init__(self, anim: str, idx: int, n: int) -> None:
        t = idx / max(1, n - 1)
        cyc = math.tau * idx / max(1, n)
        s = math.sin(cyc)

        self.root_x = 0.0
        self.root_y = 0.0
        self.bob = 0.0
        self.lean = 0.0
        self.head = 0.0
        self.left_leg = 0.0
        self.right_leg = 0.0
        self.left_lift = 0.0
        self.right_lift = 0.0
        self.left_arm = 0.0
        self.right_arm = 0.0
        self.weapon_angle = 0.0
        self.weapon_len = 0.0
        self.beard = 0.0
        self.mouth = 0.0
        self.impact = 0.0
        self.blink = False
        self.x_eye = False

        if anim == "idle":
            self.bob = s * 1.0
            self.lean = s * 1.2
            self.head = -1.0 + s * 1.0
            self.left_arm = -2.0 + s * 1.2
            self.right_arm = 2.0 - s * 1.2
            self.weapon_angle = -12.0 + s * 4.0
            self.beard = s * 3.0
            self.blink = idx == n - 2
        elif anim == "march":
            self.root_x = s * 2.2
            self.bob = abs(s) * 3.4 - 0.5
            self.lean = s * 2.0
            self.head = -2.0 - s * 1.2
            self.left_leg = -20.0 * s
            self.right_leg = 20.0 * s
            self.left_lift = max(0.0, -s) * 8.0
            self.right_lift = max(0.0, s) * 8.0
            self.left_arm = 14.0 * s - 4.0
            self.right_arm = -12.0 * s + 4.0
            self.weapon_angle = -22.0 - s * 8.0
            self.beard = -s * 8.0
        elif anim == "axe_cleave":
            tt = _ease(t)
            hit = math.sin(tt * math.pi)
            self.root_x = _lerp(-14.0, 24.0, tt)
            self.bob = -hit * 2.6
            self.lean = _lerp(-14.0, 18.0, tt)
            self.head = _lerp(-6.0, 8.0, tt)
            self.left_leg = _lerp(-10.0, 10.0, tt)
            self.right_leg = _lerp(10.0, -8.0, tt)
            self.left_arm = _lerp(-58.0, 22.0, tt)
            self.right_arm = _lerp(-24.0, 30.0, tt)
            self.weapon_angle = _lerp(-118.0, 34.0, tt)
            self.weapon_len = hit * 12.0
            self.beard = _lerp(10.0, -10.0, tt)
            self.mouth = 0.12
            self.impact = hit
        elif anim == "helm_bash":
            tt = _ease(t)
            hit = math.sin(tt * math.pi)
            self.root_x = _lerp(-12.0, 26.0, tt)
            self.bob = -hit * 2.0
            self.lean = _lerp(-12.0, 24.0, tt)
            self.head = _lerp(-4.0, 12.0, tt)
            self.left_leg = _lerp(-12.0, 16.0, tt)
            self.right_leg = _lerp(12.0, -10.0, tt)
            self.left_arm = _lerp(10.0, 16.0, tt)
            self.right_arm = _lerp(-16.0, 18.0, tt)
            self.weapon_angle = _lerp(-34.0, 8.0, tt)
            self.weapon_len = hit * 8.0
            self.beard = _lerp(8.0, -8.0, tt)
            self.mouth = 0.10
            self.impact = hit
        elif anim == "bass_bellow":
            self.bob = s * 1.0
            self.lean = -2.0 + s * 2.0
            self.head = -4.0 + s * 2.0
            self.left_arm = -20.0 + s * 3.0
            self.right_arm = -28.0 - s * 4.0
            self.weapon_angle = -76.0 + s * 6.0
            self.beard = s * 8.0
            self.mouth = 0.30 + max(0.0, s) * 0.08
        elif anim == "hurt":
            hit = math.sin(t * math.pi)
            shake = math.sin(t * math.pi * 5.0) * (1.0 - t)
            self.root_x = shake * 3.0 - hit * 5.0
            self.bob = -hit * 2.2
            self.lean = -14.0 * hit
            self.head = 10.0 * hit
            self.left_leg = -8.0 * hit
            self.right_leg = 8.0 * hit
            self.left_arm = 20.0 * hit
            self.right_arm = 14.0 * hit
            self.weapon_angle = -18.0 + hit * 20.0
            self.beard = -14.0 * hit
            self.mouth = 0.10 * hit
        elif anim == "death":
            tt = _ease(t)
            self.root_x = tt * 18.0
            self.root_y = tt * 12.0
            self.lean = -84.0 * tt
            self.head = -18.0 * tt
            self.left_leg = _lerp(-2.0, 18.0, tt)
            self.right_leg = _lerp(2.0, -18.0, tt)
            self.left_arm = _lerp(0.0, 50.0, tt)
            self.right_arm = _lerp(0.0, -44.0, tt)
            self.weapon_angle = _lerp(-18.0, 22.0, tt)
            self.weapon_len = tt * 10.0
            self.beard = -20.0 * tt
            self.x_eye = tt > 0.58


def _draw_leg(
    draw: ImageDraw.ImageDraw, hip: Point, ang: float, lift: float, *, front: bool
) -> Point:
    thigh, shin = 36, 34
    knee = (
        hip[0] + thigh * math.cos(math.radians(ang)),
        hip[1] + thigh * math.sin(math.radians(ang)),
    )
    ankle = (
        knee[0] + shin * math.cos(math.radians(ang + 8)),
        knee[1] + shin * math.sin(math.radians(ang + 8)) - lift,
    )
    col = PANTS if front else PANTS_SHADE
    _line(draw, [hip, knee, ankle], col, 7.2 if front else 6.6)
    _line(draw, [hip, knee, ankle], OUTLINE, 1.0)
    sandal = [
        (ankle[0] - 10, ankle[1] - 4),
        (ankle[0] + 10, ankle[1] - 4),
        (ankle[0] + 14, ankle[1] + 4),
        (ankle[0] + 6, ankle[1] + 10),
        (ankle[0] - 10, ankle[1] + 8),
    ]
    _poly(draw, sandal, SANDAL, OUTLINE, 0.8)
    return ankle


def _draw_beefy_arm(
    draw: ImageDraw.ImageDraw, shoulder: Point, elbow: Point, hand: Point, skin: RGBA
) -> None:
    _line(draw, [shoulder, elbow, hand], skin, 10.8)
    _line(draw, [shoulder, elbow, hand], OUTLINE, 1.15)
    upper_mid = ((shoulder[0] + elbow[0]) / 2.0, (shoulder[1] + elbow[1]) / 2.0)
    fore_mid = ((elbow[0] + hand[0]) / 2.0, (elbow[1] + hand[1]) / 2.0)
    _ellipse(draw, upper_mid[0], upper_mid[1], 10.6, 8.8, skin, OUTLINE, 0.45)
    _ellipse(draw, fore_mid[0], fore_mid[1], 9.0, 7.4, skin, OUTLINE, 0.4)
    _line(
        draw,
        [(upper_mid[0] - 7, upper_mid[1] - 4), (upper_mid[0] + 7, upper_mid[1] + 4)],
        GOLD,
        0.9,
    )


def _draw_double_axe(
    draw: ImageDraw.ImageDraw, hand_a: Point, hand_b: Point, angle: float, length: float
) -> Point:
    mid = ((hand_a[0] + hand_b[0]) / 2.0, (hand_a[1] + hand_b[1]) / 2.0)
    tail = (
        mid[0] - length * 0.46 * math.cos(math.radians(angle)),
        mid[1] - length * 0.46 * math.sin(math.radians(angle)),
    )
    tip = (
        mid[0] + length * 0.56 * math.cos(math.radians(angle)),
        mid[1] + length * 0.56 * math.sin(math.radians(angle)),
    )
    _line(draw, [tail, tip], WOOD, 3.1)
    _line(draw, [tail, tip], OUTLINE, 0.55)
    for anchor, sign in [(tip, 1), (tail, -1)]:
        hx, hy = anchor
        forward = angle if sign > 0 else angle + 180
        p1 = (
            hx + 6 * math.cos(math.radians(forward)),
            hy + 6 * math.sin(math.radians(forward)),
        )
        left = (
            hx + 10 * math.cos(math.radians(forward + 90)),
            hy + 10 * math.sin(math.radians(forward + 90)),
        )
        right = (
            hx + 10 * math.cos(math.radians(forward - 90)),
            hy + 10 * math.sin(math.radians(forward - 90)),
        )
        outer1 = (
            hx + 36 * math.cos(math.radians(forward + 34)),
            hy + 36 * math.sin(math.radians(forward + 34)),
        )
        outer2 = (
            hx + 36 * math.cos(math.radians(forward - 34)),
            hy + 36 * math.sin(math.radians(forward - 34)),
        )
        _poly(draw, [p1, outer1, left], STEEL, OUTLINE, 0.55)
        _poly(draw, [p1, outer2, right], STEEL_SHADE, OUTLINE, 0.55)
    return tip


def _render_frame(anim: str, idx: int, n: int) -> Image.Image:
    img = Image.new(
        "RGBA", (WORK_FRAME_SIZE[0] * SUPER, WORK_FRAME_SIZE[1] * SUPER), (0, 0, 0, 0)
    )
    draw = ImageDraw.Draw(img, "RGBA")
    pose = Pose(anim, idx, n)

    root = (
        WORK_FRAME_SIZE[0] * 0.48 + pose.root_x,
        WORK_FRAME_SIZE[1] * 0.80 + pose.root_y + pose.bob,
    )
    body_ang = pose.lean

    def P(x: float, y: float) -> Point:
        rx, ry = _rot(x, y, body_ang)
        return (root[0] + rx, root[1] + ry)

    far_hip = P(18, -120)
    _draw_leg(draw, far_hip, 94 + pose.right_leg, pose.right_lift, front=False)
    tunic_back = [
        P(-48, -180),
        P(44, -180),
        P(56, -52),
        P(16, -4),
        P(-26, -4),
        P(-60, -58),
    ]
    _poly(draw, tunic_back, TUNIC_SHADE, OUTLINE, 1.0)

    torso = [
        P(-60, -274),
        P(14, -282),
        P(62, -246),
        P(74, -180),
        P(58, -124),
        P(2, -102),
        P(-50, -124),
        P(-74, -190),
    ]
    _poly(draw, torso, TUNIC, OUTLINE, 1.2)
    belly = [
        P(-38, -212),
        P(28, -214),
        P(50, -176),
        P(40, -130),
        P(0, -108),
        P(-40, -126),
        P(-50, -174),
    ]
    _poly(draw, belly, FUR, OUTLINE, 0.8)
    fur = [
        P(-50, -258),
        P(14, -276),
        P(58, -250),
        P(74, -220),
        P(46, -202),
        P(6, -212),
        P(-36, -198),
        P(-64, -222),
    ]
    _poly(draw, fur, FUR, OUTLINE, 0.9)
    for x, y in [(-34, -230), (-14, -238), (8, -232), (28, -222)]:
        _line(draw, [P(x, y), P(x + 6, y + 9)], FUR_SHADE, 0.9)
    belt = [P(-42, -132), P(34, -132), P(34, -114), P(-42, -114)]
    _poly(draw, belt, LEATHER, OUTLINE, 0.7)
    _ellipse(draw, P(-4, -123)[0], P(-4, -123)[1], 7, 5, GOLD, OUTLINE, 0.35)

    far_shoulder = P(44, -224)
    far_elbow = P(70 + pose.right_arm * 0.18, -180 + pose.right_arm * 0.16)
    far_hand = P(84 + pose.right_arm * 0.28, -126 + pose.right_arm * 0.22)
    _draw_beefy_arm(draw, far_shoulder, far_elbow, far_hand, SKIN_SHADE)

    head_root = P(-4, -304)
    head_ang = body_ang + pose.head

    def H(x: float, y: float) -> Point:
        rx, ry = _rot(x, y, head_ang)
        return (head_root[0] + rx, head_root[1] + ry)

    left_horn = [H(-20, -14), H(-46, -34), H(-58, -18), H(-46, 2), H(-24, -2)]
    right_horn = [H(18, -16), H(44, -36), H(56, -20), H(46, 2), H(22, -4)]
    _poly(draw, left_horn, HORN, OUTLINE, 0.55)
    _poly(draw, right_horn, HORN, OUTLINE, 0.55)
    _line(draw, [H(-36, -22), H(-48, -20)], HORN_SHADE, 0.45)
    _line(draw, [H(34, -24), H(46, -22)], HORN_SHADE, 0.45)

    hair_back = [
        H(-28, -8),
        H(-24, -28),
        H(18, -28),
        H(28, -10),
        H(28, 18),
        H(12, 30),
        H(-12, 24),
        H(-28, 12),
    ]
    _poly(draw, hair_back, HAIR_SHADE, OUTLINE, 0.7)
    helmet = [
        H(-26, -8),
        H(-20, -34),
        H(20, -34),
        H(28, -10),
        H(24, 18),
        H(10, 26),
        H(-12, 26),
        H(-28, 10),
    ]
    _poly(draw, helmet, STEEL, OUTLINE, 0.9)
    nose_guard = [H(2, -10), H(10, -2), H(6, 18), H(-2, 12)]
    _poly(draw, nose_guard, STEEL_SHADE, OUTLINE, 0.35)
    face = [
        H(-22, -2),
        H(-16, -20),
        H(12, -20),
        H(22, -2),
        H(18, 16),
        H(8, 24),
        H(-8, 24),
        H(-22, 12),
    ]
    _poly(draw, face, SKIN, OUTLINE, 0.8)
    moustache_l = [H(-10, 10), H(-2, 14), H(0, 10), H(-4, 6)]
    moustache_r = [H(2, 10), H(10, 14), H(14, 10), H(6, 6)]
    _poly(draw, moustache_l, HAIR, OUTLINE, 0.2)
    _poly(draw, moustache_r, HAIR, OUTLINE, 0.2)
    beard = [
        H(-18, 14),
        H(-2, 28 + pose.beard * 0.1),
        H(16, 22 + pose.beard * 0.08),
        H(22, 44 + pose.beard * 0.1),
        H(6, 64 + pose.beard * 0.1),
        H(-10, 58 + pose.beard * 0.1),
        H(-22, 34),
    ]
    _poly(draw, beard, BEARD, OUTLINE, 0.8)
    for frac in [0.3, 0.6]:
        bx = _lerp(beard[0][0], beard[-2][0], frac)
        by = _lerp(beard[0][1], beard[-2][1], frac)
        _line(draw, [(bx - 3, by - 2), (bx + 3, by + 4)], BEARD_SHADE, 0.35)

    if pose.x_eye:
        _line(draw, [H(-10, 0), H(-3, 7)], OUTLINE, 0.8)
        _line(draw, [H(-10, 7), H(-3, 0)], OUTLINE, 0.8)
        _line(draw, [H(6, 0), H(13, 7)], OUTLINE, 0.8)
        _line(draw, [H(6, 7), H(13, 0)], OUTLINE, 0.8)
    elif pose.blink:
        _line(draw, [H(-12, 1), H(-4, 1)], OUTLINE, 0.7)
        _line(draw, [H(6, 1), H(14, 1)], OUTLINE, 0.7)
    else:
        _ellipse(draw, H(-8, 1)[0], H(-8, 1)[1], 3.8, 3.0, EYE, OUTLINE, 0.35)
        _ellipse(draw, H(10, 1)[0], H(10, 1)[1], 3.8, 3.0, EYE, OUTLINE, 0.35)
        _circle(draw, H(-7, 1), 1.0, PUPIL, PUPIL, 0.1)
        _circle(draw, H(11, 1), 1.0, PUPIL, PUPIL, 0.1)
    _line(draw, [H(-14, -6), H(-4, -8)], OUTLINE, 0.45)
    _line(draw, [H(6, -8), H(16, -6)], OUTLINE, 0.45)
    if pose.mouth > 0.03:
        _ellipse(
            draw,
            H(2, 20)[0],
            H(2, 20)[1],
            6.0,
            3.2 + pose.mouth * 12.0,
            MOUTH,
            OUTLINE,
            0.35,
        )
        if pose.mouth > 0.14:
            _poly(draw, [H(-2, 20), H(2, 28), H(6, 20)], TONGUE, OUTLINE, 0.2)
    else:
        _line(draw, [H(-2, 18), H(4, 20), H(10, 18)], MOUTH, 0.7)

    near_hip = P(-20, -120)
    near_foot = _draw_leg(
        draw, near_hip, 94 + pose.left_leg, pose.left_lift, front=True
    )
    tunic_front = [
        P(-52, -176),
        P(46, -176),
        P(56, -48),
        P(18, 0),
        P(-28, -2),
        P(-62, -54),
    ]
    _poly(draw, tunic_front, TUNIC, OUTLINE, 1.0)
    for x in [-24, -4, 16, 34]:
        _line(draw, [P(x, -166), P(x + 6, -8)], TUNIC_SHADE, 0.9)

    near_shoulder = P(-48, -224)
    near_elbow = P(-74 + pose.left_arm * 0.18, -180 + pose.left_arm * 0.16)
    near_hand = P(-90 + pose.left_arm * 0.32, -124 + pose.left_arm * 0.22)
    _draw_beefy_arm(draw, near_shoulder, near_elbow, near_hand, SKIN)

    axe_tip = _draw_double_axe(
        draw, near_hand, far_hand, pose.weapon_angle, 122 + pose.weapon_len
    )
    for hand in [near_hand, far_hand]:
        _circle(
            draw, hand, 4.6, SKIN if hand == near_hand else SKIN_SHADE, OUTLINE, 0.3
        )
        _line(
            draw,
            [(hand[0] - 5, hand[1] - 2), (hand[0] + 5, hand[1] + 2)],
            LEATHER_DARK,
            0.45,
        )

    if anim in {"march", "helm_bash"} and (
        pose.left_lift > 0.5 or pose.right_lift > 0.5
    ):
        for dx in [-18, 0, 14]:
            c = (near_foot[0] + dx, near_foot[1] + 8)
            _poly(
                draw,
                [
                    (c[0] - 3, c[1]),
                    (c[0], c[1] - 4),
                    (c[0] + 4, c[1] - 1),
                    (c[0] + 1, c[1] + 3),
                ],
                DUST,
                None,
                0,
            )
    if anim == "axe_cleave" and pose.impact > 0.18:
        cx, cy = axe_tip
        box = (_s(cx - 54), _s(cy - 30), _s(cx + 44), _s(cy + 50))
        draw.arc(box, 194, 344, fill=FX, width=_s(3.4))
    if anim == "helm_bash" and pose.impact > 0.18:
        cx, cy = H(0, -2)
        box = (_s(cx - 34), _s(cy - 24), _s(cx + 54), _s(cy + 32))
        draw.arc(box, 210, 350, fill=FX, width=_s(3.2))
    if anim == "bass_bellow" and pose.mouth > 0.2:
        cx, cy = H(6, 20)
        for expand in [0, 14]:
            box = (
                _s(cx - 18 - expand),
                _s(cy - 20 - expand * 0.6),
                _s(cx + 42 + expand),
                _s(cy + 20 + expand * 0.6),
            )
            draw.arc(box, 330, 30, fill=FX, width=_s(2.1))

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
    )
    return [
        outputs[k]
        for k in [
            "spritesheet",
            "yaml",
            "ron",
            "preview",
            "canonical",
            "canonical_transparent",
        ]
    ]


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        description="Render the standalone opera-style heavy Viking warrior sprite sheet."
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
