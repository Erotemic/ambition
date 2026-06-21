from __future__ import annotations

"""Standalone generator for a heavy Viking shieldmaiden sprite sheet.

Redesigned from scratch with a distinct, opera-singer inspired silhouette:
- huge horned steel helmet
- giant gold chestplate / breastplate discs
- big beefy arms, broad body, dress-like lower silhouette
- round saw-like shield and long spear
- exaggerated bellow / diva-warrior energy

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
        "character_id": "npc_viking_heavy_shieldmaiden",
        "display_name": "Viking Heavy Shieldmaiden",
    },
    "body": {
        "body_plan": "HumanoidBiped",
        "body_kind": "Wide",
        "mass_class": "Heavy",
        "traits": ["story", "humanoid", "enemy", "combatant", "viking", "shield"],
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
    "tags": ["story", "humanoid", "enemy", "combatant", "viking", "shield"],
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
        "shield_center": {
            "source": "explicit.profile.viking",
            "point": {"x": 46.0, "y": 62.0},
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
        "action.defend.block": {"animation": "block", "events": []},
    },
}


RGBA = Tuple[int, int, int, int]
Point = Tuple[float, float]

TARGET_NAME = "viking_heavy_shieldmaiden"
FRAME_SIZE = (320, 320)
WORK_FRAME_SIZE = (760, 760)
SUPER = 4
ROWS: List[Tuple[str, int, int]] = [
    ("idle", 6, 132),
    ("march", 8, 98),
    ("shield_barge", 7, 84),
    ("spear_jab", 7, 82),
    ("diva_bellow", 6, 104),
    ("hurt", 4, 92),
    ("death", 8, 112),
]

OUTLINE = (26, 20, 18, 255)
SKIN = (232, 194, 164, 255)
SKIN_SHADE = (194, 156, 126, 255)
BLONDE = (247, 194, 56, 255)
BLONDE_SHADE = (210, 156, 36, 255)
STEEL = (188, 198, 210, 255)
STEEL_SHADE = (132, 142, 154, 255)
HORN = (235, 228, 176, 255)
HORN_SHADE = (198, 188, 132, 255)
GOLD = (236, 192, 68, 255)
GOLD_SHADE = (188, 138, 38, 255)
BROWN = (138, 90, 54, 255)
BROWN_DARK = (92, 60, 38, 255)
DRESS = (120, 162, 212, 255)
DRESS_SHADE = (80, 120, 174, 255)
CLOTH = (118, 96, 92, 255)
CLOTH_SHADE = (88, 70, 68, 255)
SHIELD = (168, 172, 182, 255)
SHIELD_SHADE = (118, 122, 132, 255)
SANDAL = (82, 58, 34, 255)
EYE = (248, 246, 238, 255)
PUPIL = (40, 38, 42, 255)
LIP = (190, 58, 70, 255)
MOUTH = (108, 48, 52, 255)
TONGUE = (212, 100, 116, 255)
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
        self.shield_arm = 0.0
        self.weapon_arm = 0.0
        self.weapon_pitch = 0.0
        self.shield_push = 0.0
        self.mouth = 0.0
        self.braid = 0.0
        self.blink = False
        self.x_eye = False
        self.impact = 0.0

        if anim == "idle":
            self.bob = s * 1.2
            self.lean = s * 1.4
            self.head = -1.0 + s * 1.0
            self.shield_arm = -2.0 + s * 1.5
            self.weapon_arm = 2.0 - s * 1.5
            self.weapon_pitch = -2.0
            self.braid = s * 4.0
            self.blink = idx == n - 2
        elif anim == "march":
            self.root_x = s * 2.4
            self.bob = abs(s) * 3.6 - 0.6
            self.lean = s * 2.2
            self.head = -2.0 - s * 1.2
            self.left_leg = -20.0 * s
            self.right_leg = 20.0 * s
            self.left_lift = max(0.0, -s) * 8.0
            self.right_lift = max(0.0, s) * 8.0
            self.shield_arm = 14.0 * s - 4.0
            self.weapon_arm = -10.0 * s + 4.0
            self.weapon_pitch = -10.0 * s
            self.braid = -s * 10.0
        elif anim == "shield_barge":
            tt = _ease(t)
            hit = math.sin(tt * math.pi)
            self.root_x = _lerp(-14.0, 24.0, tt)
            self.bob = -hit * 2.4
            self.lean = _lerp(-12.0, 22.0, tt)
            self.head = _lerp(-4.0, 8.0, tt)
            self.left_leg = _lerp(-12.0, 14.0, tt)
            self.right_leg = _lerp(10.0, -10.0, tt)
            self.shield_arm = _lerp(-42.0, 28.0, tt)
            self.weapon_arm = _lerp(-12.0, -2.0, tt)
            self.weapon_pitch = _lerp(16.0, 32.0, tt)
            self.shield_push = hit
            self.mouth = 0.10
            self.braid = _lerp(8.0, -8.0, tt)
            self.impact = hit
        elif anim == "spear_jab":
            tt = _ease(t)
            hit = math.sin(tt * math.pi)
            self.root_x = _lerp(-12.0, 22.0, tt)
            self.bob = -hit * 2.0
            self.lean = _lerp(-10.0, 18.0, tt)
            self.head = _lerp(-5.0, 8.0, tt)
            self.left_leg = _lerp(-8.0, 10.0, tt)
            self.right_leg = _lerp(8.0, -8.0, tt)
            self.shield_arm = _lerp(-8.0, -12.0, tt)
            self.weapon_arm = _lerp(-34.0, 38.0, tt)
            self.weapon_pitch = _lerp(-46.0, 8.0, tt)
            self.mouth = 0.12
            self.braid = _lerp(12.0, -12.0, tt)
            self.impact = hit
        elif anim == "diva_bellow":
            self.bob = s * 1.2
            self.lean = -2.0 + s * 2.0
            self.head = -4.0 + s * 2.0
            self.left_leg = -2.0
            self.right_leg = 2.0
            self.shield_arm = -18.0 - s * 3.0
            self.weapon_arm = -46.0 + s * 4.0
            self.weapon_pitch = -42.0 + s * 4.0
            self.braid = s * 8.0
            self.mouth = 0.28 + max(0.0, s) * 0.08
        elif anim == "hurt":
            hit = math.sin(t * math.pi)
            shake = math.sin(t * math.pi * 5.0) * (1.0 - t)
            self.root_x = shake * 3.0 - hit * 5.0
            self.bob = -hit * 2.2
            self.lean = -12.0 * hit
            self.head = 10.0 * hit
            self.left_leg = -8.0 * hit
            self.right_leg = 8.0 * hit
            self.shield_arm = 16.0 * hit
            self.weapon_arm = 20.0 * hit
            self.weapon_pitch = 18.0 * hit
            self.braid = -14.0 * hit
            self.mouth = 0.12 * hit
        elif anim == "death":
            tt = _ease(t)
            self.root_x = tt * 16.0
            self.root_y = tt * 10.0
            self.lean = -84.0 * tt
            self.head = -18.0 * tt
            self.left_leg = _lerp(-2.0, 18.0, tt)
            self.right_leg = _lerp(2.0, -18.0, tt)
            self.shield_arm = _lerp(0.0, -42.0, tt)
            self.weapon_arm = _lerp(0.0, 34.0, tt)
            self.weapon_pitch = _lerp(-2.0, 40.0, tt)
            self.braid = -18.0 * tt
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
    col = SKIN if front else SKIN_SHADE
    _line(draw, [hip, knee, ankle], col, 6.6 if front else 6.0)
    _line(draw, [hip, knee, ankle], OUTLINE, 1.0)
    sandal = [
        (ankle[0] - 10, ankle[1] - 4),
        (ankle[0] + 10, ankle[1] - 4),
        (ankle[0] + 14, ankle[1] + 4),
        (ankle[0] + 6, ankle[1] + 10),
        (ankle[0] - 10, ankle[1] + 8),
    ]
    _poly(draw, sandal, SANDAL, OUTLINE, 0.8)
    for off in [-4, 1, 6]:
        _line(
            draw,
            [(ankle[0] + off, ankle[1] - 3), (ankle[0] + off, ankle[1] + 7)],
            GOLD_SHADE,
            0.35,
        )
    return ankle


def _draw_beefy_arm(
    draw: ImageDraw.ImageDraw, shoulder: Point, elbow: Point, hand: Point, skin: RGBA
) -> None:
    _line(draw, [shoulder, elbow, hand], skin, 12.2)
    _line(draw, [shoulder, elbow, hand], OUTLINE, 1.2)
    upper_mid = ((shoulder[0] + elbow[0]) / 2.0, (shoulder[1] + elbow[1]) / 2.0)
    fore_mid = ((elbow[0] + hand[0]) / 2.0, (elbow[1] + hand[1]) / 2.0)
    _ellipse(draw, shoulder[0], shoulder[1], 6.0, 6.0, skin, OUTLINE, 0.35)
    _ellipse(draw, upper_mid[0], upper_mid[1], 12.2, 9.8, skin, OUTLINE, 0.5)
    _ellipse(draw, elbow[0], elbow[1], 7.2, 6.2, skin, OUTLINE, 0.35)
    _ellipse(draw, fore_mid[0], fore_mid[1], 10.0, 8.0, skin, OUTLINE, 0.45)
    _line(
        draw,
        [(upper_mid[0] - 8, upper_mid[1] - 4), (upper_mid[0] + 8, upper_mid[1] + 4)],
        GOLD,
        1.0,
    )


def _draw_saw_shield(draw: ImageDraw.ImageDraw, center: Point, r: float) -> None:
    teeth: List[Point] = []
    for i in range(18):
        ang = math.tau * i / 18.0
        rr = r + (6 if i % 2 == 0 else 1)
        teeth.append((center[0] + rr * math.cos(ang), center[1] + rr * math.sin(ang)))
    _poly(draw, teeth, SHIELD, OUTLINE, 0.8)
    _circle(draw, center, r * 0.76, SHIELD_SHADE, OUTLINE, 0.45)
    _circle(draw, center, r * 0.28, STEEL, OUTLINE, 0.3)
    for ang in [0.0, math.pi / 2, math.pi / 4, -math.pi / 4]:
        p1 = (
            center[0] + math.cos(ang) * r * 0.18,
            center[1] + math.sin(ang) * r * 0.18,
        )
        p2 = (
            center[0] + math.cos(ang) * r * 0.56,
            center[1] + math.sin(ang) * r * 0.56,
        )
        _line(draw, [p1, p2], STEEL, 0.7)


def _draw_spear(
    draw: ImageDraw.ImageDraw, hand: Point, angle: float, length: float
) -> Point:
    butt = (
        hand[0] - length * 0.25 * math.cos(math.radians(angle)),
        hand[1] - length * 0.25 * math.sin(math.radians(angle)),
    )
    tip_base = (
        hand[0] + length * 0.75 * math.cos(math.radians(angle)),
        hand[1] + length * 0.75 * math.sin(math.radians(angle)),
    )
    _line(draw, [butt, tip_base], BROWN, 6.2)
    _line(draw, [butt, tip_base], OUTLINE, 0.9)
    tip = (
        tip_base[0] + 24 * math.cos(math.radians(angle)),
        tip_base[1] + 24 * math.sin(math.radians(angle)),
    )
    side = (
        tip_base[0] + 14 * math.cos(math.radians(angle + 90)),
        tip_base[1] + 14 * math.sin(math.radians(angle + 90)),
    )
    side2 = (
        tip_base[0] + 14 * math.cos(math.radians(angle - 90)),
        tip_base[1] + 14 * math.sin(math.radians(angle - 90)),
    )
    _poly(draw, [tip, side, side2], STEEL, OUTLINE, 0.8)
    ferrule_a = (
        tip_base[0] - 8 * math.cos(math.radians(angle)),
        tip_base[1] - 8 * math.sin(math.radians(angle)),
    )
    ferrule_b = (
        tip_base[0] - 16 * math.cos(math.radians(angle)),
        tip_base[1] - 16 * math.sin(math.radians(angle)),
    )
    _line(draw, [ferrule_b, ferrule_a], GOLD_SHADE, 1.8)
    cap = (
        butt[0] - 12 * math.cos(math.radians(angle)),
        butt[1] - 12 * math.sin(math.radians(angle)),
    )
    _poly(
        draw,
        [cap, (butt[0] + 5, butt[1] + 6), (butt[0] - 5, butt[1] - 6)],
        STEEL_SHADE,
        OUTLINE,
        0.35,
    )
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

    # Back leg / body mass.
    far_hip = P(20, -118)
    _draw_leg(draw, far_hip, 94 + pose.right_leg, pose.right_lift, front=False)
    dress_back = [
        P(-44, -164),
        P(42, -164),
        P(58, -46),
        P(24, -4),
        P(-18, -6),
        P(-56, -56),
    ]
    _poly(draw, dress_back, DRESS_SHADE, OUTLINE, 1.0)

    torso = [
        P(-56, -270),
        P(16, -274),
        P(60, -240),
        P(72, -168),
        P(48, -118),
        P(-10, -106),
        P(-56, -138),
        P(-72, -206),
    ]
    _poly(draw, torso, CLOTH, OUTLINE, 1.2)

    chestplate = [
        P(-42, -244),
        P(22, -246),
        P(48, -226),
        P(46, -182),
        P(28, -136),
        P(-16, -128),
        P(-46, -156),
        P(-52, -206),
    ]
    _poly(draw, chestplate, GOLD, OUTLINE, 0.9)
    left_breast = P(-18, -206)
    right_breast = P(22, -206)
    _circle(draw, left_breast, 26, GOLD_SHADE, OUTLINE, 0.75)
    _circle(draw, right_breast, 26, GOLD_SHADE, OUTLINE, 0.75)
    _line(draw, [P(0, -230), P(0, -134)], GOLD_SHADE, 0.8)
    for cc in [left_breast, right_breast]:
        for rr in [5, 10, 15, 20]:
            _circle(draw, cc, rr, None, GOLD, 0.3)
    collar = [
        P(-34, -246),
        P(-10, -258),
        P(14, -258),
        P(38, -246),
        P(26, -234),
        P(-10, -236),
    ]
    _poly(draw, collar, STEEL, OUTLINE, 0.7)

    # Back arm (weapon).
    far_shoulder = P(42, -226)
    far_elbow = P(68 + pose.weapon_arm * 0.22, -180 + pose.weapon_arm * 0.18)
    far_hand = P(84 + pose.weapon_arm * 0.35, -122 + pose.weapon_arm * 0.24)
    _draw_beefy_arm(draw, far_shoulder, far_elbow, far_hand, SKIN_SHADE)

    head_root = P(-4, -270)
    head_ang = body_ang + pose.head

    def H(x: float, y: float) -> Point:
        rx, ry = _rot(x, y, head_ang)
        return (head_root[0] + rx, head_root[1] + ry)

    # Horns behind helmet, shaped more like curved rib bones with pointed tips.
    left_horn_base = [H(-16, -8), H(-24, -18), H(-32, -20), H(-28, -10), H(-18, -4)]
    left_horn_mid1 = [H(-24, -18), H(-34, -28), H(-42, -32), H(-40, -22), H(-30, -16)]
    left_horn_mid2 = [H(-34, -28), H(-46, -34), H(-56, -32), H(-50, -24), H(-40, -22)]
    left_horn_tip = [H(-46, -34), H(-60, -30), H(-68, -20), H(-56, -18), H(-50, -24)]
    right_horn_base = [H(16, -10), H(24, -20), H(32, -22), H(28, -12), H(18, -6)]
    right_horn_mid1 = [H(24, -20), H(34, -30), H(42, -34), H(40, -24), H(30, -18)]
    right_horn_mid2 = [H(34, -30), H(46, -36), H(56, -34), H(50, -26), H(40, -24)]
    right_horn_tip = [H(46, -36), H(60, -32), H(68, -22), H(56, -20), H(50, -26)]
    for pts in [
        left_horn_base,
        left_horn_mid1,
        left_horn_mid2,
        left_horn_tip,
        right_horn_base,
        right_horn_mid1,
        right_horn_mid2,
        right_horn_tip,
    ]:
        _poly(draw, pts, HORN, OUTLINE, 0.42)
    _line(
        draw,
        [H(-22, -14), H(-30, -20), H(-40, -26), H(-52, -27), H(-61, -24)],
        HORN_SHADE,
        0.38,
    )
    _line(
        draw,
        [H(22, -16), H(30, -22), H(40, -28), H(52, -29), H(61, -26)],
        HORN_SHADE,
        0.38,
    )

    braid_l = [
        H(-28, 6),
        H(-42 + pose.braid * 0.16, 28),
        H(-44 + pose.braid * 0.22, 52),
        H(-34 + pose.braid * 0.20, 76),
    ]
    braid_r = [
        H(26, 6),
        H(40 + pose.braid * 0.12, 30),
        H(40 + pose.braid * 0.20, 54),
        H(30 + pose.braid * 0.18, 76),
    ]
    _line(draw, braid_l, BLONDE, 6.5)
    _line(draw, braid_r, BLONDE, 6.5)
    _line(draw, braid_l, OUTLINE, 0.7)
    _line(draw, braid_r, OUTLINE, 0.7)
    for frac in [0.25, 0.5, 0.75]:
        for braid in [braid_l, braid_r]:
            bx = _lerp(braid[0][0], braid[-1][0], frac)
            by = _lerp(braid[0][1], braid[-1][1], frac)
            _line(draw, [(bx - 4, by - 3), (bx + 4, by + 3)], BLONDE_SHADE, 0.45)

    helmet = [
        H(-24, -8),
        H(-18, -34),
        H(18, -34),
        H(26, -8),
        H(24, 14),
        H(10, 24),
        H(-12, 24),
        H(-26, 10),
    ]
    _poly(draw, helmet, STEEL, OUTLINE, 0.9)
    face = [
        H(-22, -2),
        H(-18, -22),
        H(12, -22),
        H(24, -4),
        H(22, 16),
        H(10, 30),
        H(-6, 30),
        H(-20, 16),
    ]
    _poly(draw, face, SKIN, OUTLINE, 0.8)
    _ellipse(draw, H(-9, 14)[0], H(-9, 14)[1], 4.8, 3.2, (232, 142, 150, 130), None, 0)
    _ellipse(draw, H(13, 14)[0], H(13, 14)[1], 4.8, 3.2, (232, 142, 150, 130), None, 0)

    if pose.x_eye:
        _line(draw, [H(-10, 0), H(-3, 7)], OUTLINE, 0.8)
        _line(draw, [H(-10, 7), H(-3, 0)], OUTLINE, 0.8)
        _line(draw, [H(6, 0), H(13, 7)], OUTLINE, 0.8)
        _line(draw, [H(6, 7), H(13, 0)], OUTLINE, 0.8)
    elif pose.blink:
        _line(draw, [H(-12, 2), H(-4, 2)], OUTLINE, 0.7)
        _line(draw, [H(6, 2), H(14, 2)], OUTLINE, 0.7)
    else:
        _ellipse(draw, H(-8, 3)[0], H(-8, 3)[1], 4.0, 3.2, EYE, OUTLINE, 0.35)
        _ellipse(draw, H(10, 3)[0], H(10, 3)[1], 4.0, 3.2, EYE, OUTLINE, 0.35)
        _circle(draw, H(-7, 3), 1.1, PUPIL, PUPIL, 0.1)
        _circle(draw, H(11, 3), 1.1, PUPIL, PUPIL, 0.1)
    _line(draw, [H(-14, -6), H(-4, -8)], OUTLINE, 0.45)
    _line(draw, [H(6, -8), H(16, -6)], OUTLINE, 0.45)
    _line(draw, [H(1, 2), H(4, 12)], SKIN_SHADE, 0.35)
    if pose.mouth > 0.03:
        _ellipse(
            draw,
            H(2, 22)[0],
            H(2, 22)[1],
            6.0,
            3.2 + pose.mouth * 14.0,
            MOUTH,
            OUTLINE,
            0.35,
        )
        if pose.mouth > 0.14:
            _poly(draw, [H(-2, 22), H(2, 30), H(6, 22)], TONGUE, OUTLINE, 0.2)
    else:
        _ellipse(draw, H(2, 22)[0], H(2, 22)[1], 5.2, 2.0, LIP, OUTLINE, 0.25)

    # Front leg / arm.
    near_hip = P(-18, -118)
    near_foot = _draw_leg(
        draw, near_hip, 94 + pose.left_leg, pose.left_lift, front=True
    )
    dress_front = [
        P(-48, -164),
        P(40, -164),
        P(50, -42),
        P(16, 0),
        P(-28, -2),
        P(-60, -54),
    ]
    _poly(draw, dress_front, DRESS, OUTLINE, 1.0)
    for x in [-24, -4, 14, 30]:
        _line(draw, [P(x, -154), P(x + 4, -8)], DRESS_SHADE, 0.9)
    hem_cover = [P(-34, -86), P(8, -80), P(18, -12), P(-22, -8), P(-42, -42)]
    _poly(draw, hem_cover, DRESS, OUTLINE, 0.6)
    near_shoulder = P(-44, -224)
    near_elbow = P(-70 + pose.shield_arm * 0.18, -176 + pose.shield_arm * 0.16)
    near_hand = P(
        -88 + pose.shield_arm * 0.32 - pose.shield_push * 10,
        -120 + pose.shield_arm * 0.12,
    )
    _draw_beefy_arm(draw, near_shoulder, near_elbow, near_hand, SKIN)
    shield_center = near_hand
    _draw_saw_shield(draw, shield_center, 46)
    spear_tip = _draw_spear(draw, far_hand, -78 + pose.weapon_pitch, 146)

    # Feet dust / impact FX.
    if anim in {"march", "shield_barge"} and (
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
    if anim == "shield_barge" and pose.impact > 0.18:
        cx, cy = shield_center
        box = (_s(cx - 36), _s(cy - 28), _s(cx + 48), _s(cy + 34))
        draw.arc(box, 210, 350, fill=FX, width=_s(3.2))
    if anim == "spear_jab" and pose.impact > 0.18:
        cx, cy = spear_tip
        box = (_s(cx - 40), _s(cy - 22), _s(cx + 56), _s(cy + 26))
        draw.arc(box, 195, 345, fill=FX, width=_s(3.0))
    if anim == "diva_bellow" and pose.mouth > 0.2:
        cx, cy = H(6, 22)
        for expand in [0, 14]:
            box = (
                _s(cx - 16 - expand),
                _s(cy - 18 - expand * 0.6),
                _s(cx + 42 + expand),
                _s(cy + 18 + expand * 0.6),
            )
            draw.arc(box, 330, 30, fill=FX, width=_s(2.0))

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
        description="Render the standalone opera-style heavy Viking shieldmaiden sprite sheet."
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
