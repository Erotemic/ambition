from __future__ import annotations

"""Standalone generator for an attacking Viking lady warrior sprite sheet.

Concept:
- a fierce shieldmaiden / viking lady warrior
- broad round shield, one-handed axe, fur mantle, braided hair
- attack-forward moveset suited to a side scroller
- readable silhouette with strong weapon and shield posing

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
        "character_id": "npc_viking_shieldmaiden",
        "display_name": "Viking Shieldmaiden",
    },
    "body": {
        "body_plan": "HumanoidBiped",
        "body_kind": "Standard",
        "mass_class": "Medium",
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
    "brain": {"default_preset": "melee_brute_striker"},
    "actions": {"default_preset": "striker_swipe"},
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

TARGET_NAME = "viking_shieldmaiden"
# Files the tack-on installer copies into the sandbox sprites dir.
# Names match what `build_sheet` writes (target_spritesheet.{png,yaml,ron}).
SHEET_FILES = [
    f"{TARGET_NAME}_spritesheet.png",
    f"{TARGET_NAME}_spritesheet.yaml",
    f"{TARGET_NAME}_spritesheet.ron",
]
FRAME_SIZE = (320, 320)
WORK_FRAME_SIZE = (640, 640)
SUPER = 4
ROWS: List[Tuple[str, int, int]] = [
    ("idle", 6, 128),
    ("walk", 8, 96),
    ("axe_swing", 7, 82),
    ("shield_bash", 6, 84),
    ("overhead_chop", 7, 80),
    ("battle_cry", 6, 108),
    ("hurt", 4, 92),
    ("death", 8, 112),
]

OUTLINE = (28, 22, 20, 255)
SKIN = (220, 178, 146, 255)
SKIN_SHADE = (182, 140, 114, 255)
HAIR = (194, 154, 78, 255)
HAIR_SHADE = (148, 112, 54, 255)
FUR = (214, 206, 188, 255)
FUR_SHADE = (170, 162, 146, 255)
TUNIC = (106, 74, 120, 255)
TUNIC_SHADE = (76, 52, 90, 255)
SKIRT = (56, 82, 126, 255)
SKIRT_SHADE = (40, 62, 98, 255)
LEATHER = (112, 76, 48, 255)
LEATHER_DARK = (82, 54, 34, 255)
STEEL = (188, 196, 206, 255)
STEEL_SHADE = (132, 142, 154, 255)
GOLD = (214, 176, 84, 255)
SHIELD_RED = (146, 56, 50, 255)
SHIELD_RED_DARK = (106, 38, 36, 255)
BOOT = (54, 40, 32, 255)
MOUTH = (108, 64, 68, 255)
TONGUE = (194, 98, 112, 255)
EYE = (242, 242, 238, 255)
PUPIL = (34, 34, 40, 255)
FX = (246, 230, 162, 150)
DUST = (130, 116, 92, 130)


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
    def __init__(self, anim: str, frame_idx: int, nframes: int) -> None:
        t = frame_idx / max(1, nframes - 1)
        cyc = math.tau * frame_idx / max(1, nframes)
        s = math.sin(cyc)
        c = math.cos(cyc)

        self.root_x = 0.0
        self.root_y = 0.0
        self.bob = 0.0
        self.lean = 0.0
        self.head = 0.0
        self.left_leg = 0.0
        self.right_leg = 0.0
        self.left_lift = 0.0
        self.right_lift = 0.0
        self.weapon_arm = 0.0
        self.shield_arm = 0.0
        self.weapon_raise = 0.0
        self.shield_push = 0.0
        self.hair = 0.0
        self.mouth = 0.0
        self.impact = 0.0
        self.dead_t = 0.0
        self.blink = False
        self.x_eye = False

        if anim == "idle":
            self.bob = s * 1.4
            self.lean = s * 1.5
            self.head = -2.0 + s * 1.2
            self.left_leg = -2.0 + c * 1.5
            self.right_leg = 2.0 - c * 1.4
            self.weapon_arm = 4.0 - s * 3.0
            self.shield_arm = -4.0 + s * 2.0
            self.hair = s * 3.0
            self.blink = frame_idx == nframes - 2
        elif anim == "walk":
            self.root_x = s * 2.2
            self.bob = abs(s) * 2.8 - 0.5
            self.lean = s * 2.0
            self.head = -2.0 - s * 1.0
            self.left_leg = -22.0 * s
            self.right_leg = 22.0 * s
            self.left_lift = max(0.0, -s) * 8.0
            self.right_lift = max(0.0, s) * 8.0
            self.weapon_arm = -14.0 * s + 2.0
            self.shield_arm = 14.0 * s - 2.0
            self.hair = -s * 8.0
        elif anim == "axe_swing":
            tt = _ease(t)
            hit = math.sin(tt * math.pi)
            self.root_x = _lerp(-6.0, 14.0, tt)
            self.bob = -hit * 2.0
            self.lean = _lerp(-10.0, 14.0, tt)
            self.head = _lerp(-6.0, 8.0, tt)
            self.left_leg = _lerp(-10.0, 10.0, tt)
            self.right_leg = _lerp(10.0, -4.0, tt)
            self.weapon_arm = _lerp(-58.0, 44.0, tt)
            self.shield_arm = _lerp(6.0, -14.0, tt)
            self.weapon_raise = hit
            self.hair = _lerp(10.0, -10.0, tt)
            self.mouth = 0.10 + hit * 0.06
            self.impact = hit
        elif anim == "shield_bash":
            tt = _ease(t)
            hit = math.sin(tt * math.pi)
            self.root_x = _lerp(-12.0, 18.0, tt)
            self.bob = -hit * 2.0
            self.lean = _lerp(-8.0, 18.0, tt)
            self.head = _lerp(-4.0, 6.0, tt)
            self.left_leg = _lerp(-14.0, 12.0, tt)
            self.right_leg = _lerp(10.0, -6.0, tt)
            self.weapon_arm = _lerp(6.0, -16.0, tt)
            self.shield_arm = _lerp(-36.0, 24.0, tt)
            self.shield_push = hit
            self.hair = _lerp(6.0, -8.0, tt)
            self.mouth = 0.12
            self.impact = hit
        elif anim == "overhead_chop":
            tt = _ease(t)
            hit = math.sin(tt * math.pi)
            self.root_x = _lerp(-6.0, 10.0, tt)
            self.root_y = _lerp(0.0, -3.0, tt)
            self.bob = -hit * 3.0
            self.lean = _lerp(-6.0, 12.0, tt)
            self.head = _lerp(-4.0, 10.0, tt)
            self.left_leg = _lerp(-6.0, 8.0, tt)
            self.right_leg = _lerp(8.0, -6.0, tt)
            self.weapon_arm = _lerp(-90.0, 70.0, tt)
            self.shield_arm = _lerp(-4.0, -18.0, tt)
            self.weapon_raise = hit
            self.hair = _lerp(12.0, -12.0, tt)
            self.mouth = 0.14
            self.impact = hit
        elif anim == "battle_cry":
            self.bob = s * 1.0
            self.lean = -2.0 + s * 2.0
            self.head = -4.0 + s * 2.0
            self.left_leg = -2.0
            self.right_leg = 4.0
            self.weapon_arm = -48.0 + s * 6.0
            self.shield_arm = -30.0 - s * 5.0
            self.weapon_raise = 1.0
            self.hair = s * 8.0
            self.mouth = 0.24 + max(0.0, s) * 0.10
        elif anim == "hurt":
            hit = math.sin(t * math.pi)
            shake = math.sin(t * math.pi * 5.0) * (1.0 - t)
            self.root_x = shake * 3.0 - hit * 3.0
            self.bob = -hit * 2.0
            self.lean = -12.0 * hit
            self.head = 8.0 * hit
            self.left_leg = -8.0 * hit
            self.right_leg = 8.0 * hit
            self.weapon_arm = 20.0 * hit
            self.shield_arm = 14.0 * hit
            self.hair = -12.0 * hit
            self.mouth = 0.10 * hit
        elif anim == "death":
            tt = _ease(t)
            self.dead_t = tt
            self.root_x = tt * 18.0
            self.root_y = tt * 10.0
            self.bob = -tt * 4.0
            self.lean = -82.0 * tt
            self.head = -18.0 * tt
            self.left_leg = _lerp(-2.0, 18.0, tt)
            self.right_leg = _lerp(2.0, -18.0, tt)
            self.weapon_arm = _lerp(4.0, 58.0, tt)
            self.shield_arm = _lerp(-4.0, -56.0, tt)
            self.hair = -22.0 * tt
            self.x_eye = tt > 0.58


def _draw_leg(
    draw: ImageDraw.ImageDraw, hip: Point, ang: float, lift: float, *, front: bool
) -> Point:
    thigh = 40
    shin = 38
    knee = (
        hip[0] + thigh * math.cos(math.radians(ang)),
        hip[1] + thigh * math.sin(math.radians(ang)),
    )
    ankle = (
        knee[0] + shin * math.cos(math.radians(ang + 8)),
        knee[1] + shin * math.sin(math.radians(ang + 8)) - lift,
    )
    col = LEATHER if front else LEATHER_DARK
    _line(draw, [hip, knee, ankle], col, 8.2 if front else 7.2)
    _line(draw, [hip, knee, ankle], OUTLINE, 1.1)
    boot = [
        (ankle[0] - 8, ankle[1] - 5),
        (ankle[0] + 10, ankle[1] - 5),
        (ankle[0] + 15, ankle[1] + 4),
        (ankle[0] + 6, ankle[1] + 10),
        (ankle[0] - 8, ankle[1] + 8),
    ]
    _poly(draw, boot, BOOT, OUTLINE, 0.8)
    return ankle


def _draw_shield(
    draw: ImageDraw.ImageDraw,
    center: Point,
    r: float,
    *,
    angle: float = 0.0,
    front: bool = True,
) -> None:
    rim = STEEL if front else STEEL_SHADE
    face = SHIELD_RED if front else SHIELD_RED_DARK
    _ellipse(draw, center[0], center[1], r + 3, r + 3, rim, OUTLINE, 0.8)
    _ellipse(draw, center[0], center[1], r, r, face, OUTLINE, 0.7)
    # quartered / ring detail
    _ellipse(draw, center[0], center[1], r * 0.58, r * 0.58, rim, OUTLINE, 0.4)
    _line(
        draw,
        [(center[0] - r * 0.74, center[1]), (center[0] + r * 0.74, center[1])],
        GOLD,
        0.9,
    )
    _line(
        draw,
        [(center[0], center[1] - r * 0.74), (center[0], center[1] + r * 0.74)],
        GOLD,
        0.9,
    )
    _circle(draw, center, r * 0.18, STEEL_SHADE, OUTLINE, 0.3)


def _draw_axe(
    draw: ImageDraw.ImageDraw, hand: Point, ang: float, length: float
) -> Point:
    tip = (
        hand[0] + length * math.cos(math.radians(ang)),
        hand[1] + length * math.sin(math.radians(ang)),
    )
    _line(draw, [hand, tip], LEATHER, 2.8)
    _line(draw, [hand, tip], OUTLINE, 0.5)
    hx, hy = tip
    head = [
        (hx - 4, hy - 6),
        (hx + 8, hy - 16),
        (hx + 22, hy - 6),
        (hx + 18, hy + 8),
        (hx + 6, hy + 14),
        (hx - 6, hy + 6),
    ]
    beard = [
        (hx + 2, hy - 2),
        (hx + 26, hy - 6),
        (hx + 36, hy + 4),
        (hx + 18, hy + 14),
    ]
    _poly(draw, head, STEEL, OUTLINE, 0.7)
    _poly(draw, beard, STEEL_SHADE, OUTLINE, 0.6)
    _line(draw, [(hx + 4, hy - 10), (hx + 18, hy + 8)], (224, 228, 236, 255), 0.8)
    return tip


def _render_frame(anim: str, frame_idx: int, nframes: int) -> Image.Image:
    img = Image.new(
        "RGBA", (WORK_FRAME_SIZE[0] * SUPER, WORK_FRAME_SIZE[1] * SUPER), (0, 0, 0, 0)
    )
    draw = ImageDraw.Draw(img, "RGBA")
    pose = Pose(anim, frame_idx, nframes)

    root = (
        WORK_FRAME_SIZE[0] * 0.48 + pose.root_x,
        WORK_FRAME_SIZE[1] * 0.78 + pose.root_y + pose.bob,
    )
    body_ang = pose.lean

    def P(x: float, y: float) -> Point:
        rx, ry = _rot(x, y, body_ang)
        return (root[0] + rx, root[1] + ry)

    # far leg and skirt back
    far_hip = P(10, -106)
    _draw_leg(draw, far_hip, 92 + pose.right_leg, pose.right_lift, front=False)

    skirt_back = [
        P(-22, -106),
        P(20, -106),
        P(32, -40),
        P(10, -10),
        P(-18, -14),
        P(-34, -48),
    ]
    _poly(draw, skirt_back, SKIRT_SHADE, OUTLINE, 1.0)

    # torso
    torso = [
        P(-34, -206),
        P(6, -214),
        P(34, -198),
        P(44, -150),
        P(36, -108),
        P(8, -88),
        P(-18, -94),
        P(-40, -144),
    ]
    _poly(draw, torso, TUNIC, OUTLINE, 1.2)
    vest = [
        P(-12, -192),
        P(10, -196),
        P(20, -120),
        P(0, -100),
        P(-18, -120),
        P(-22, -176),
    ]
    _poly(draw, vest, TUNIC_SHADE, OUTLINE, 0.7)

    # bust / upper torso coding
    breast_l = [P(-16, -168), P(-7, -180), P(0, -166), P(-4, -148), P(-14, -148)]
    breast_r = [P(0, -168), P(12, -178), P(18, -160), P(10, -146), P(0, -148)]
    _poly(draw, breast_l, TUNIC, OUTLINE, 0.4)
    _poly(draw, breast_r, TUNIC, OUTLINE, 0.4)

    # fur mantle
    mantle = [
        P(-30, -206),
        P(8, -216),
        P(34, -204),
        P(48, -174),
        P(28, -160),
        P(2, -168),
        P(-22, -158),
        P(-40, -176),
    ]
    _poly(draw, mantle, FUR, OUTLINE, 0.9)
    for x, y in [(-22, -188), (-6, -196), (10, -190), (24, -182)]:
        _line(draw, [P(x, y), P(x + 6, y + 8)], FUR_SHADE, 0.8)

    # far arm / shield arm behind body sometimes
    far_shoulder = P(28, -184)
    far_elbow = P(48 + pose.weapon_arm * 0.18, -146 + pose.weapon_arm * 0.12)
    far_hand = P(56 + pose.weapon_arm * 0.28, -100 + pose.weapon_arm * 0.14)
    _line(draw, [far_shoulder, far_elbow, far_hand], SKIN_SHADE, 6.4)
    _line(draw, [far_shoulder, far_elbow, far_hand], OUTLINE, 0.9)

    # head
    head_root = P(-2, -244)
    head_ang = body_ang + pose.head

    def H(x: float, y: float) -> Point:
        rx, ry = _rot(x, y, head_ang)
        return (head_root[0] + rx, head_root[1] + ry)

    hair_back = [
        H(-24, -8),
        H(-14, -34),
        H(10, -36),
        H(28, -18),
        H(28, 10),
        H(16, 26),
        H(-4, 20),
        H(-24, 8),
    ]
    _poly(draw, hair_back, HAIR_SHADE, OUTLINE, 0.9)
    # braid
    braid = [
        H(18, 8),
        H(26 + pose.hair * 0.15, 24),
        H(32 + pose.hair * 0.28, 42),
        H(26 + pose.hair * 0.22, 60),
    ]
    _line(draw, braid, HAIR, 5.6)
    _line(draw, braid, OUTLINE, 0.7)
    for frac in [0.2, 0.45, 0.7]:
        bx = _lerp(braid[0][0], braid[-1][0], frac)
        by = _lerp(braid[0][1], braid[-1][1], frac)
        _line(draw, [(bx - 4, by - 3), (bx + 4, by + 3)], GOLD, 0.5)

    helmet = [
        H(-20, -16),
        H(-6, -34),
        H(18, -30),
        H(30, -10),
        H(26, 4),
        H(8, 10),
        H(-12, 4),
    ]
    _poly(draw, helmet, STEEL, OUTLINE, 0.9)
    _poly(draw, [H(-6, -34), H(2, -46), H(12, -34)], STEEL_SHADE, OUTLINE, 0.5)
    head = [
        H(-18, -10),
        H(-12, -26),
        H(8, -30),
        H(22, -18),
        H(24, 6),
        H(12, 22),
        H(-8, 22),
        H(-22, 8),
    ]
    _poly(draw, head, SKIN, OUTLINE, 0.9)

    if pose.x_eye:
        _line(draw, [H(-8, -4), H(-1, 3)], OUTLINE, 0.8)
        _line(draw, [H(-8, 3), H(-1, -4)], OUTLINE, 0.8)
        _line(draw, [H(8, -5), H(15, 2)], OUTLINE, 0.8)
        _line(draw, [H(8, 2), H(15, -5)], OUTLINE, 0.8)
    elif pose.blink:
        _line(draw, [H(-10, -2), H(-2, -2)], OUTLINE, 0.7)
        _line(draw, [H(8, -3), H(16, -3)], OUTLINE, 0.7)
    else:
        _ellipse(draw, H(-6, -2)[0], H(-6, -2)[1], 3.8, 3.0, EYE, OUTLINE, 0.4)
        _ellipse(draw, H(12, -3)[0], H(12, -3)[1], 3.8, 3.0, EYE, OUTLINE, 0.4)
        _circle(draw, H(-5, -2), 1.0, PUPIL, PUPIL, 0.1)
        _circle(draw, H(13, -3), 1.0, PUPIL, PUPIL, 0.1)
    _line(draw, [H(-11, -9), H(-2, -10)], OUTLINE, 0.5)
    _line(draw, [H(8, -10), H(16, -11)], OUTLINE, 0.5)
    nose = [H(2, 0), H(6, 8), H(2, 10), H(-1, 5)]
    _poly(draw, nose, SKIN_SHADE, OUTLINE, 0.25)
    if pose.mouth > 0.03:
        _ellipse(
            draw,
            H(4, 14)[0],
            H(4, 14)[1],
            5.0,
            2.6 + pose.mouth * 10.0,
            MOUTH,
            OUTLINE,
            0.4,
        )
        if pose.mouth > 0.14:
            _poly(draw, [H(0, 14), H(4, 20), H(8, 14)], TONGUE, OUTLINE, 0.2)
    else:
        _line(draw, [H(-1, 14), H(5, 16), H(11, 14)], MOUTH, 0.7)

    # near leg
    near_hip = P(-12, -106)
    near_foot = _draw_leg(
        draw, near_hip, 92 + pose.left_leg, pose.left_lift, front=True
    )

    # front skirt/panel over legs
    skirt_front = [
        P(-28, -104),
        P(16, -104),
        P(24, -40),
        P(6, -8),
        P(-16, -10),
        P(-32, -44),
    ]
    _poly(draw, skirt_front, SKIRT, OUTLINE, 1.0)
    _line(draw, [P(-12, -98), P(-8, -18)], SKIRT_SHADE, 0.9)
    _line(draw, [P(2, -98), P(8, -14)], SKIRT_SHADE, 0.9)

    # near arm / shield on top
    near_shoulder = P(-30, -184)
    near_elbow = P(-46 + pose.shield_arm * 0.15, -148 + pose.shield_arm * 0.12)
    near_hand = P(
        -58 + pose.shield_arm * 0.26 + pose.shield_push * -8,
        -108 + pose.shield_arm * 0.10,
    )
    _line(draw, [near_shoulder, near_elbow, near_hand], SKIN, 6.8)
    _line(draw, [near_shoulder, near_elbow, near_hand], OUTLINE, 0.95)
    shield_center = (near_hand[0] - 12 - pose.shield_push * 12, near_hand[1] - 2)
    _draw_shield(draw, shield_center, 24, front=True)

    # weapon on top for readability
    axe_tip = _draw_axe(
        draw,
        far_hand,
        -32 + pose.weapon_arm * 1.15 - pose.weapon_raise * 28,
        54 + pose.weapon_raise * 10,
    )

    # belt and trim
    belt = [P(-26, -118), P(18, -118), P(18, -104), P(-26, -104)]
    _poly(draw, belt, LEATHER, OUTLINE, 0.6)
    _ellipse(draw, P(-4, -111)[0], P(-4, -111)[1], 5.0, 4.0, GOLD, OUTLINE, 0.3)
    _line(draw, [P(-24, -112), P(16, -112)], LEATHER_DARK, 0.6)

    # wrist bands
    _line(draw, [near_hand, (near_hand[0] + 10, near_hand[1] + 2)], GOLD, 0.6)
    _line(draw, [far_hand, (far_hand[0] - 8, far_hand[1] - 2)], GOLD, 0.6)

    # effects
    if anim in {"axe_swing", "overhead_chop"} and pose.impact > 0.18:
        cx, cy = axe_tip
        box = (_s(cx - 48), _s(cy - 24), _s(cx + 36), _s(cy + 42))
        draw.arc(box, 196, 344, fill=FX, width=_s(3.6))
    if anim == "shield_bash" and pose.impact > 0.18:
        cx, cy = shield_center
        box = (_s(cx - 34), _s(cy - 24), _s(cx + 46), _s(cy + 32))
        draw.arc(box, 210, 350, fill=FX, width=_s(3.2))
    if anim in {"walk", "shield_bash", "overhead_chop"} and (
        pose.left_lift > 0.5 or pose.right_lift > 0.5
    ):
        for dx in [-20, 0, 18]:
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
        outputs["spritesheet"],
        outputs["yaml"],
        outputs["ron"],
        outputs["preview"],
        outputs["canonical"],
        outputs["canonical_transparent"],
    ]


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        description="Render the standalone Viking Shieldmaiden sprite sheet."
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
