from __future__ import annotations

"""Standalone generator for an attacking Viking man warrior sprite sheet.

Concept:
- broad, bearded Viking man warrior / raider
- heavy two-handed dane axe for a distinct silhouette
- fur mantle, leather boots, tunic, bracers
- aggressive attack-forward poses suited to a side scroller

Generator only. No registration or GUI wiring.
"""

import argparse
import math
from pathlib import Path
from typing import List, Sequence, Tuple

from PIL import Image, ImageDraw

from ...authoring.tackon_sheet import build_sheet

ACTOR_METADATA = {
    "actor": {"character_id": "npc_viking_warrior", "display_name": "Viking Warrior"},
    "body": {
        "body_plan": "HumanoidBiped",
        "body_kind": "Standard",
        "mass_class": "Medium",
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
    "brain": {"default_preset": "melee_brute_striker"},
    "actions": {"default_preset": "striker_swipe"},
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

TARGET_NAME = "viking_warrior"
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
    ("cleave", 7, 80),
    ("charge", 7, 80),
    ("leap_chop", 7, 82),
    ("roar", 6, 106),
    ("hurt", 4, 90),
    ("death", 8, 112),
]

OUTLINE = (28, 22, 18, 255)
SKIN = (214, 166, 132, 255)
SKIN_SHADE = (172, 126, 98, 255)
HAIR = (170, 112, 58, 255)
HAIR_SHADE = (126, 80, 42, 255)
BEARD = (136, 86, 48, 255)
FUR = (208, 200, 184, 255)
FUR_SHADE = (164, 154, 138, 255)
TUNIC = (76, 92, 124, 255)
TUNIC_SHADE = (56, 68, 92, 255)
PANTS = (92, 66, 48, 255)
PANTS_SHADE = (70, 50, 36, 255)
LEATHER = (112, 76, 46, 255)
LEATHER_DARK = (82, 56, 34, 255)
STEEL = (190, 198, 208, 255)
STEEL_SHADE = (130, 140, 152, 255)
WOOD = (118, 82, 52, 255)
GOLD = (212, 174, 82, 255)
BOOT = (50, 38, 30, 255)
EYE = (242, 240, 236, 255)
PUPIL = (34, 34, 40, 255)
MOUTH = (102, 64, 66, 255)
TONGUE = (194, 94, 108, 255)
FX = (245, 232, 164, 150)
DUST = (134, 116, 92, 130)


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
        self.left_arm = 0.0
        self.right_arm = 0.0
        self.weapon_angle = 0.0
        self.weapon_len = 0.0
        self.hair = 0.0
        self.mouth = 0.0
        self.impact = 0.0
        self.dead_t = 0.0
        self.blink = False
        self.x_eye = False

        if anim == "idle":
            self.bob = s * 1.2
            self.lean = s * 1.3
            self.head = -2.0 + s * 1.2
            self.left_leg = -2.0 + c * 1.4
            self.right_leg = 2.0 - c * 1.4
            self.left_arm = -2.0 + s * 2.0
            self.right_arm = 2.0 - s * 2.0
            self.weapon_angle = -44.0 + s * 4.0
            self.weapon_len = 0.0
            self.hair = s * 2.0
            self.blink = frame_idx == nframes - 2
        elif anim == "walk":
            self.root_x = s * 2.0
            self.bob = abs(s) * 2.8 - 0.5
            self.lean = s * 2.0
            self.head = -2.0 - s * 1.0
            self.left_leg = -22.0 * s
            self.right_leg = 22.0 * s
            self.left_lift = max(0.0, -s) * 8.0
            self.right_lift = max(0.0, s) * 8.0
            self.left_arm = 12.0 * s - 4.0
            self.right_arm = -12.0 * s + 4.0
            self.weapon_angle = -48.0 - s * 10.0
            self.hair = -s * 6.0
        elif anim == "cleave":
            tt = _ease(t)
            hit = math.sin(tt * math.pi)
            self.root_x = _lerp(-10.0, 16.0, tt)
            self.bob = -hit * 2.0
            self.lean = _lerp(-12.0, 16.0, tt)
            self.head = _lerp(-8.0, 10.0, tt)
            self.left_leg = _lerp(-12.0, 12.0, tt)
            self.right_leg = _lerp(10.0, -6.0, tt)
            self.left_arm = _lerp(-46.0, 26.0, tt)
            self.right_arm = _lerp(-18.0, 34.0, tt)
            self.weapon_angle = _lerp(-120.0, 24.0, tt)
            self.weapon_len = hit * 10.0
            self.hair = _lerp(10.0, -10.0, tt)
            self.mouth = 0.12 + hit * 0.06
            self.impact = hit
        elif anim == "charge":
            tt = _ease(t)
            hit = math.sin(tt * math.pi)
            self.root_x = _lerp(-14.0, 26.0, tt)
            self.bob = -hit * 2.0
            self.lean = _lerp(-10.0, 20.0, tt)
            self.head = _lerp(-6.0, 8.0, tt)
            self.left_leg = _lerp(-18.0, 16.0, tt)
            self.right_leg = _lerp(12.0, -10.0, tt)
            self.left_lift = _lerp(0.0, 5.0, tt)
            self.right_lift = _lerp(0.0, 2.0, tt)
            self.left_arm = _lerp(-14.0, 8.0, tt)
            self.right_arm = _lerp(-10.0, 14.0, tt)
            self.weapon_angle = _lerp(-54.0, -6.0, tt)
            self.weapon_len = hit * 6.0
            self.hair = _lerp(6.0, -6.0, tt)
            self.mouth = 0.10
            self.impact = hit
        elif anim == "leap_chop":
            tt = _ease(t)
            hit = math.sin(tt * math.pi)
            self.root_x = _lerp(-8.0, 14.0, tt)
            self.root_y = _lerp(0.0, -10.0, hit)
            self.bob = -hit * 4.0
            self.lean = _lerp(-8.0, 14.0, tt)
            self.head = _lerp(-6.0, 10.0, tt)
            self.left_leg = _lerp(-8.0, 18.0, tt)
            self.right_leg = _lerp(10.0, -12.0, tt)
            self.left_lift = hit * 6.0
            self.right_lift = hit * 9.0
            self.left_arm = _lerp(-58.0, 32.0, tt)
            self.right_arm = _lerp(-26.0, 36.0, tt)
            self.weapon_angle = _lerp(-132.0, 36.0, tt)
            self.weapon_len = hit * 12.0
            self.hair = _lerp(12.0, -12.0, tt)
            self.mouth = 0.14
            self.impact = hit
        elif anim == "roar":
            self.bob = s * 0.8
            self.lean = -2.0 + s * 2.0
            self.head = -4.0 + s * 2.0
            self.left_leg = -2.0
            self.right_leg = 3.0
            self.left_arm = -12.0 + s * 4.0
            self.right_arm = -18.0 - s * 3.0
            self.weapon_angle = -96.0 + s * 6.0
            self.weapon_len = 10.0
            self.hair = s * 8.0
            self.mouth = 0.26 + max(0.0, s) * 0.08
        elif anim == "hurt":
            hit = math.sin(t * math.pi)
            shake = math.sin(t * math.pi * 5.0) * (1.0 - t)
            self.root_x = shake * 3.0 - hit * 3.0
            self.bob = -hit * 2.0
            self.lean = -12.0 * hit
            self.head = 8.0 * hit
            self.left_leg = -8.0 * hit
            self.right_leg = 8.0 * hit
            self.left_arm = 20.0 * hit
            self.right_arm = 12.0 * hit
            self.weapon_angle = -40.0 + hit * 12.0
            self.hair = -12.0 * hit
            self.mouth = 0.10 * hit
        elif anim == "death":
            tt = _ease(t)
            self.dead_t = tt
            self.root_x = tt * 18.0
            self.root_y = tt * 10.0
            self.bob = -tt * 4.0
            self.lean = -80.0 * tt
            self.head = -18.0 * tt
            self.left_leg = _lerp(-2.0, 18.0, tt)
            self.right_leg = _lerp(2.0, -18.0, tt)
            self.left_arm = _lerp(-4.0, 52.0, tt)
            self.right_arm = _lerp(4.0, -44.0, tt)
            self.weapon_angle = _lerp(-46.0, -10.0, tt)
            self.weapon_len = tt * 8.0
            self.hair = -18.0 * tt
            self.x_eye = tt > 0.56


def _draw_leg(
    draw: ImageDraw.ImageDraw, hip: Point, ang: float, lift: float, *, front: bool
) -> Point:
    thigh = 42
    shin = 40
    knee = (
        hip[0] + thigh * math.cos(math.radians(ang)),
        hip[1] + thigh * math.sin(math.radians(ang)),
    )
    ankle = (
        knee[0] + shin * math.cos(math.radians(ang + 8)),
        knee[1] + shin * math.sin(math.radians(ang + 8)) - lift,
    )
    col = PANTS if front else PANTS_SHADE
    _line(draw, [hip, knee, ankle], col, 8.6 if front else 7.6)
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


def _draw_axe(
    draw: ImageDraw.ImageDraw, hand_a: Point, hand_b: Point, angle: float, length: float
) -> Tuple[Point, Point]:
    mid = ((hand_a[0] + hand_b[0]) / 2.0, (hand_a[1] + hand_b[1]) / 2.0)
    tail = (
        mid[0] - (length * 0.45) * math.cos(math.radians(angle)),
        mid[1] - (length * 0.45) * math.sin(math.radians(angle)),
    )
    tip = (
        mid[0] + (length * 0.55) * math.cos(math.radians(angle)),
        mid[1] + (length * 0.55) * math.sin(math.radians(angle)),
    )
    _line(draw, [tail, tip], WOOD, 3.2)
    _line(draw, [tail, tip], OUTLINE, 0.6)
    hx, hy = tip
    head_main = [
        (hx - 4, hy - 6),
        (hx + 8, hy - 18),
        (hx + 26, hy - 10),
        (hx + 28, hy + 6),
        (hx + 10, hy + 16),
        (hx - 8, hy + 8),
    ]
    beard = [
        (hx + 2, hy - 2),
        (hx + 30, hy - 10),
        (hx + 42, hy + 0),
        (hx + 18, hy + 16),
    ]
    spike = [
        (tail[0] - 2, tail[1] - 2),
        (tail[0] - 16, tail[1] - 6),
        (tail[0] - 24, tail[1] + 2),
        (tail[0] - 12, tail[1] + 8),
    ]
    _poly(draw, head_main, STEEL, OUTLINE, 0.7)
    _poly(draw, beard, STEEL_SHADE, OUTLINE, 0.6)
    _poly(draw, spike, STEEL_SHADE, OUTLINE, 0.5)
    _line(draw, [(hx + 4, hy - 12), (hx + 18, hy + 8)], (226, 230, 236, 255), 0.8)
    return tip, tail


def _render_frame(anim: str, frame_idx: int, nframes: int) -> Image.Image:
    img = Image.new(
        "RGBA", (WORK_FRAME_SIZE[0] * SUPER, WORK_FRAME_SIZE[1] * SUPER), (0, 0, 0, 0)
    )
    draw = ImageDraw.Draw(img, "RGBA")
    pose = Pose(anim, frame_idx, nframes)

    root = (
        WORK_FRAME_SIZE[0] * 0.47 + pose.root_x,
        WORK_FRAME_SIZE[1] * 0.79 + pose.root_y + pose.bob,
    )
    body_ang = pose.lean

    def P(x: float, y: float) -> Point:
        rx, ry = _rot(x, y, body_ang)
        return (root[0] + rx, root[1] + ry)

    far_hip = P(12, -108)
    _draw_leg(draw, far_hip, 92 + pose.right_leg, pose.right_lift, front=False)

    # hips / tunic back
    kilt_back = [
        P(-24, -114),
        P(22, -114),
        P(28, -54),
        P(8, -16),
        P(-18, -20),
        P(-30, -60),
    ]
    _poly(draw, kilt_back, TUNIC_SHADE, OUTLINE, 1.0)

    torso = [
        P(-38, -214),
        P(8, -222),
        P(40, -200),
        P(50, -146),
        P(42, -106),
        P(12, -86),
        P(-22, -90),
        P(-44, -146),
    ]
    _poly(draw, torso, TUNIC, OUTLINE, 1.2)
    chest_panel = [
        P(-12, -198),
        P(18, -198),
        P(22, -122),
        P(2, -98),
        P(-18, -122),
        P(-22, -182),
    ]
    _poly(draw, chest_panel, TUNIC_SHADE, OUTLINE, 0.7)

    fur = [
        P(-34, -214),
        P(10, -224),
        P(40, -206),
        P(52, -176),
        P(30, -162),
        P(2, -170),
        P(-24, -158),
        P(-44, -176),
    ]
    _poly(draw, fur, FUR, OUTLINE, 1.0)
    for x, y in [(-26, -192), (-10, -200), (8, -194), (24, -186)]:
        _line(draw, [P(x, y), P(x + 6, y + 8)], FUR_SHADE, 0.8)

    # far arm
    far_shoulder = P(30, -186)
    far_elbow = P(44 + pose.right_arm * 0.22, -148 + pose.right_arm * 0.16)
    far_hand = P(52 + pose.right_arm * 0.34, -102 + pose.right_arm * 0.18)
    _line(draw, [far_shoulder, far_elbow, far_hand], SKIN_SHADE, 7.0)
    _line(draw, [far_shoulder, far_elbow, far_hand], OUTLINE, 0.95)

    # head
    head_root = P(-2, -248)
    head_ang = body_ang + pose.head

    def H(x: float, y: float) -> Point:
        rx, ry = _rot(x, y, head_ang)
        return (head_root[0] + rx, head_root[1] + ry)

    hair_back = [
        H(-24, -10),
        H(-14, -36),
        H(10, -38),
        H(28, -20),
        H(30, 8),
        H(16, 24),
        H(-4, 20),
        H(-24, 8),
    ]
    _poly(draw, hair_back, HAIR_SHADE, OUTLINE, 0.8)
    hair_tail = [
        H(16, 4),
        H(26 + pose.hair * 0.18, 18),
        H(34 + pose.hair * 0.28, 34),
        H(30 + pose.hair * 0.22, 54),
    ]
    _line(draw, hair_tail, HAIR, 6.0)
    _line(draw, hair_tail, OUTLINE, 0.7)

    helmet = [
        H(-18, -18),
        H(-4, -32),
        H(18, -30),
        H(30, -12),
        H(26, 2),
        H(8, 8),
        H(-12, 2),
    ]
    _poly(draw, helmet, STEEL, OUTLINE, 0.8)
    nose_guard = [H(4, -18), H(10, -6), H(6, 14), H(0, 8)]
    _poly(draw, nose_guard, STEEL_SHADE, OUTLINE, 0.35)

    face = [
        H(-18, -10),
        H(-12, -26),
        H(10, -28),
        H(24, -16),
        H(24, 6),
        H(12, 18),
        H(-6, 18),
        H(-20, 6),
    ]
    _poly(draw, face, SKIN, OUTLINE, 0.9)
    beard = [
        H(-12, 2),
        H(0, 10),
        H(12, 8),
        H(22, 2),
        H(18, 28),
        H(6, 42),
        H(-6, 38),
        H(-16, 20),
    ]
    _poly(draw, beard, BEARD, OUTLINE, 0.8)
    moustache_l = [H(-8, 6), H(-2, 10), H(2, 8), H(-2, 2)]
    moustache_r = [H(2, 8), H(10, 10), H(14, 6), H(6, 2)]
    _poly(draw, moustache_l, HAIR, OUTLINE, 0.25)
    _poly(draw, moustache_r, HAIR, OUTLINE, 0.25)

    if pose.x_eye:
        _line(draw, [H(-8, -2), H(-1, 5)], OUTLINE, 0.8)
        _line(draw, [H(-8, 5), H(-1, -2)], OUTLINE, 0.8)
        _line(draw, [H(8, -3), H(15, 4)], OUTLINE, 0.8)
        _line(draw, [H(8, 4), H(15, -3)], OUTLINE, 0.8)
    elif pose.blink:
        _line(draw, [H(-10, 0), H(-2, 0)], OUTLINE, 0.7)
        _line(draw, [H(8, -1), H(16, -1)], OUTLINE, 0.7)
    else:
        _ellipse(draw, H(-6, 0)[0], H(-6, 0)[1], 3.8, 2.8, EYE, OUTLINE, 0.4)
        _ellipse(draw, H(12, -1)[0], H(12, -1)[1], 3.8, 2.8, EYE, OUTLINE, 0.4)
        _circle(draw, H(-5, 0), 1.0, PUPIL, PUPIL, 0.1)
        _circle(draw, H(13, -1), 1.0, PUPIL, PUPIL, 0.1)
    _line(draw, [H(-11, -8), H(-2, -10)], OUTLINE, 0.5)
    _line(draw, [H(8, -9), H(16, -10)], OUTLINE, 0.5)

    if pose.mouth > 0.03:
        _ellipse(
            draw,
            H(4, 12)[0],
            H(4, 12)[1],
            5.0,
            2.6 + pose.mouth * 10.0,
            MOUTH,
            OUTLINE,
            0.4,
        )
        if pose.mouth > 0.15:
            _poly(draw, [H(0, 12), H(4, 18), H(8, 12)], TONGUE, OUTLINE, 0.2)
    else:
        _line(draw, [H(-1, 12), H(5, 14), H(11, 12)], MOUTH, 0.7)

    near_hip = P(-12, -108)
    near_foot = _draw_leg(
        draw, near_hip, 92 + pose.left_leg, pose.left_lift, front=True
    )

    kilt_front = [
        P(-28, -112),
        P(20, -112),
        P(26, -50),
        P(10, -12),
        P(-14, -14),
        P(-30, -56),
    ]
    _poly(draw, kilt_front, TUNIC, OUTLINE, 1.0)
    _line(draw, [P(-10, -104), P(-8, -20)], TUNIC_SHADE, 0.8)
    _line(draw, [P(6, -104), P(10, -18)], TUNIC_SHADE, 0.8)

    belt = [P(-28, -124), P(20, -124), P(20, -110), P(-28, -110)]
    _poly(draw, belt, LEATHER, OUTLINE, 0.7)
    _ellipse(draw, P(-4, -117)[0], P(-4, -117)[1], 5.0, 4.0, GOLD, OUTLINE, 0.3)

    # near arm / both hands grip axe shaft
    near_shoulder = P(-32, -186)
    near_elbow = P(-44 + pose.left_arm * 0.20, -148 + pose.left_arm * 0.18)
    near_hand = P(-52 + pose.left_arm * 0.34, -102 + pose.left_arm * 0.22)
    _line(draw, [near_shoulder, near_elbow, near_hand], SKIN, 7.2)
    _line(draw, [near_shoulder, near_elbow, near_hand], OUTLINE, 1.0)

    axe_tip, axe_tail = _draw_axe(
        draw,
        near_hand,
        far_hand,
        pose.weapon_angle,
        88 + pose.weapon_len,
    )

    # hand grips over shaft
    for hand in [near_hand, far_hand]:
        _circle(
            draw, hand, 4.5, SKIN if hand == near_hand else SKIN_SHADE, OUTLINE, 0.35
        )
        _line(
            draw,
            [(hand[0] - 5, hand[1] - 2), (hand[0] + 5, hand[1] + 2)],
            LEATHER_DARK,
            0.5,
        )

    # arm bands
    _line(
        draw,
        [(near_hand[0] - 8, near_hand[1] - 2), (near_hand[0] + 5, near_hand[1] + 2)],
        GOLD,
        0.6,
    )
    _line(
        draw,
        [(far_hand[0] - 7, far_hand[1] - 2), (far_hand[0] + 4, far_hand[1] + 1)],
        GOLD,
        0.6,
    )

    if anim in {"cleave", "leap_chop"} and pose.impact > 0.18:
        cx, cy = axe_tip
        box = (_s(cx - 48), _s(cy - 26), _s(cx + 40), _s(cy + 44))
        draw.arc(box, 195, 344, fill=FX, width=_s(3.6))
    if anim == "charge" and pose.impact > 0.18:
        cx, cy = axe_tip
        box = (_s(cx - 24), _s(cy - 20), _s(cx + 64), _s(cy + 28))
        draw.arc(box, 215, 350, fill=FX, width=_s(3.2))
    if anim in {"walk", "charge", "leap_chop"} and (
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
        description="Render the standalone Viking Warrior sprite sheet."
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
