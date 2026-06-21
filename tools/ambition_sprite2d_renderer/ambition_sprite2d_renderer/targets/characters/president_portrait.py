from __future__ import annotations

"""Standalone generator for a portrait-inspired president character.

Visual goal:
- inspired by formal 18th-century presidential oil portrait aesthetics
- powdered white wig with side rolls
- stern presidential expression, pale skin, dark formal coat, white cravat
- restrained, dignified posture with more explicitly presidential command / oath animations
- still readable as a side-scroller unit with a few active moves

This character is not a pirate; it is a formal president / commander figure
that leans into classic portrait styling.
"""

import argparse
import math
from pathlib import Path
from typing import List, Sequence, Tuple

from PIL import Image, ImageDraw

from ...authoring.tackon_sheet import build_sheet

RGBA = Tuple[int, int, int, int]
Point = Tuple[float, float]

TARGET_NAME = "president_portrait"
# Files the tack-on installer copies into the sandbox sprites dir.
# Names match what `build_sheet` writes (target_spritesheet.{png,yaml,ron}).
SHEET_FILES = [
    f"{TARGET_NAME}_spritesheet.png",
    f"{TARGET_NAME}_spritesheet.yaml",
    f"{TARGET_NAME}_spritesheet.ron",
    f"{TARGET_NAME}_actor.ron",
]

ACTOR_METADATA = {
    "actor": {
        "character_id": "npc_president_portrait",
        "display_name": "President Portrait",
    },
    "body": {
        "body_plan": "HumanoidBiped",
        "body_kind": "Standard",
        "mass_class": "Medium",
        "locomotion_hint": "Walk",
        "traits": ["hub", "story", "statesman", "speaker"],
    },
    "capabilities": {
        "traversal": {
            "walk": True,
            "jump": None,
            "climb": None,
            "fly": None,
            "swim": None,
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
    "brain": {"default_preset": "stand_still"},
    "actions": {"default_preset": "peaceful"},
    "animation_bindings": {
        "default": {"animation": "idle", "events": []},
        "locomotion.walk": {"animation": "walk", "events": []},
        "interaction.address": {"animation": "address", "events": []},
        "interaction.decree": {"animation": "decree", "events": []},
        "interaction.oath": {"animation": "oath", "events": []},
        "damage.hit": {"animation": "hurt", "events": []},
        "lifecycle.death": {"animation": "death", "events": []},
    },
    "sockets": {
        "head": {
            "source": "president_portrait.geometry",
            "point": {"x": 160.0, "y": 78.0},
        },
        "chest": {
            "source": "president_portrait.geometry",
            "point": {"x": 160.0, "y": 170.0},
        },
        "hand_l": {
            "source": "president_portrait.geometry",
            "point": {"x": 112.0, "y": 198.0},
        },
        "hand_r": {
            "source": "president_portrait.geometry",
            "point": {"x": 210.0, "y": 198.0},
        },
        "decree_origin": {
            "source": "president_portrait.geometry",
            "point": {"x": 208.0, "y": 178.0},
        },
        "speech_bubble": {
            "source": "president_portrait.geometry",
            "point": {"x": 160.0, "y": 48.0},
        },
    },
    "tags": ["hub", "static", "story", "statesman"],
}
FRAME_SIZE = (320, 352)
WORK_FRAME_SIZE = (640, 704)
SUPER = 4
ROWS: List[Tuple[str, int, int]] = [
    ("idle", 6, 130),
    ("walk", 8, 96),
    ("address", 6, 112),
    ("decree", 7, 82),
    ("oath", 6, 96),
    ("hurt", 4, 92),
    ("death", 8, 112),
]

OUTLINE = (28, 22, 20, 255)
SKIN = (232, 205, 177, 255)
SKIN_SHADE = (198, 166, 142, 255)
BLUSH = (208, 151, 132, 255)
WIG = (236, 236, 228, 255)
WIG_SHADE = (206, 205, 197, 255)
COAT = (24, 34, 70, 255)
COAT_HI = (52, 72, 126, 255)
VEST = (244, 242, 236, 255)
CRAVAT = (252, 251, 247, 255)
BREECH = (228, 226, 219, 255)
BOOT = (30, 28, 30, 255)
GOLD = (209, 176, 87, 255)
PRES_BLUE = (82, 108, 186, 255)
PRES_RED = (168, 52, 54, 255)
PRES_WHITE = (245, 244, 238, 255)
BOOK = (106, 54, 46, 255)
BOOK_EDGE = (226, 214, 188, 255)
FX = (240, 220, 150, 160)
PARCHMENT = (230, 214, 172, 255)
PARCHMENT_SHADE = (188, 162, 112, 255)
SEAL = (148, 28, 40, 255)
INK = (48, 42, 38, 255)
SHADOW = (90, 70, 54, 80)


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
        self.tilt = 0.0
        self.head = 0.0
        self.left_arm = 0.0
        self.right_arm = 0.0
        self.left_leg = 0.0
        self.right_leg = 0.0
        self.left_lift = 0.0
        self.right_lift = 0.0
        self.coat_sway = 0.0
        self.cravat = 0.0
        self.rapier = 0.0
        self.oath = 0.0
        self.flash = 0.0
        self.open_mouth = 0.0
        self.dead_t = 0.0
        self.blink = False
        self.x_eye = False

        if anim == "idle":
            self.bob = s * 1.5
            self.tilt = s * 1.3
            self.head = -2.0 + s * 1.0
            self.left_arm = -4.0 + s * 2.0
            self.right_arm = 2.0 - s * 1.5
            self.left_leg = -2.0 + c * 1.2
            self.right_leg = 2.0 - c * 1.0
            self.coat_sway = s * 2.0
            self.cravat = max(0.0, s) * 2.0
            self.blink = frame_idx == nframes - 2
        elif anim == "walk":
            self.root_x = s * 2.0
            self.bob = abs(s) * 2.6 - 0.4
            self.tilt = s * 2.2
            self.head = -2.0 - s * 0.8
            self.left_leg = -22.0 * s
            self.right_leg = 20.0 * s
            self.left_lift = max(0.0, -s) * 8.0
            self.right_lift = max(0.0, s) * 7.0
            self.left_arm = 12.0 * s - 4.0
            self.right_arm = -10.0 * s + 4.0
            self.coat_sway = -s * 6.0
        elif anim == "address":
            self.bob = s * 1.0
            self.tilt = -1.5 + s * 0.8
            self.head = -1.5 + s * 1.2
            self.left_arm = _lerp(-6.0, 30.0, math.sin(t * math.pi))
            self.right_arm = -4.0
            self.left_leg = -1.0
            self.right_leg = 2.0
            self.open_mouth = 0.08 + max(0.0, s) * 0.06
            self.coat_sway = s * 1.4
            self.cravat = 1.0 + max(0.0, s) * 2.0
        elif anim == "decree":
            tt = _ease(t)
            hit = math.sin(tt * math.pi)
            self.root_x = _lerp(-6.0, 18.0, tt)
            self.bob = -hit * 2.0
            self.tilt = _lerp(-6.0, 10.0, tt)
            self.head = _lerp(-4.0, 8.0, tt)
            self.left_arm = _lerp(-18.0, 56.0, tt)
            self.right_arm = _lerp(6.0, -24.0, tt)
            self.left_leg = _lerp(-10.0, 16.0, tt)
            self.right_leg = _lerp(8.0, -6.0, tt)
            self.left_lift = _lerp(0.0, 6.0, tt)
            self.coat_sway = _lerp(6.0, -14.0, tt)
            self.rapier = hit
        elif anim == "oath":
            tt = _ease(t)
            self.root_x = _lerp(-2.0, 6.0, tt)
            self.bob = -math.sin(tt * math.pi) * 1.2
            self.tilt = _lerp(-1.0, 2.5, tt)
            self.head = _lerp(-1.0, 2.0, tt)
            self.left_arm = _lerp(-6.0, 18.0, tt)
            self.right_arm = _lerp(-18.0, -78.0, tt)
            self.left_leg = -2.0
            self.right_leg = 3.0
            self.coat_sway = _lerp(2.0, -4.0, tt)
            self.open_mouth = 0.04 + tt * 0.04
            self.oath = tt
        elif anim == "hurt":
            hit = math.sin(t * math.pi)
            shake = math.sin(t * math.pi * 5.0) * (1.0 - t)
            self.root_x = shake * 3.0 - hit * 3.5
            self.bob = -hit * 2.0
            self.tilt = -9.0 * hit
            self.head = 6.0 * hit
            self.left_arm = 12.0 * hit
            self.right_arm = 16.0 * hit
            self.left_leg = -8.0 * hit
            self.right_leg = 7.0 * hit
            self.coat_sway = -8.0 * hit
            self.open_mouth = 0.10 * hit
        elif anim == "death":
            tt = _ease(t)
            self.dead_t = tt
            self.root_x = tt * 14.0
            self.root_y = tt * 8.0
            self.bob = -tt * 4.0
            self.tilt = -78.0 * tt
            self.head = -16.0 * tt
            self.left_arm = _lerp(-4.0, 48.0, tt)
            self.right_arm = _lerp(4.0, -52.0, tt)
            self.left_leg = _lerp(-2.0, 18.0, tt)
            self.right_leg = _lerp(2.0, -18.0, tt)
            self.coat_sway = -20.0 * tt
            self.x_eye = tt > 0.58


def _draw_leg(
    draw: ImageDraw.ImageDraw, hip: Point, thigh_ang: float, lift: float, front: bool
) -> Point:
    thigh_len = 46
    shin_len = 44
    knee = (
        hip[0] + thigh_len * math.cos(math.radians(thigh_ang)),
        hip[1] + thigh_len * math.sin(math.radians(thigh_ang)),
    )
    ankle = (
        knee[0] + shin_len * math.cos(math.radians(thigh_ang + 10)),
        knee[1] + shin_len * math.sin(math.radians(thigh_ang + 10)) - lift,
    )
    col = BREECH if front else (212, 210, 205, 255)
    _line(draw, [hip, knee, ankle], col, 8.0 if front else 7.0)
    _line(draw, [hip, knee, ankle], OUTLINE, 1.1)
    _ellipse(draw, knee[0], knee[1], 5.2, 5.6, col, OUTLINE, 0.5)
    boot = [
        (ankle[0] - 7, ankle[1] - 6),
        (ankle[0] + 10, ankle[1] - 6),
        (ankle[0] + 16, ankle[1] + 4),
        (ankle[0] + 6, ankle[1] + 10),
        (ankle[0] - 8, ankle[1] + 8),
    ]
    _poly(draw, boot, BOOT, OUTLINE, 0.8)
    return ankle


def _draw_hand(draw: ImageDraw.ImageDraw, p: Point, r: float = 4.4) -> None:
    _ellipse(draw, p[0], p[1], r, r * 0.88, SKIN, OUTLINE, 0.5)


def _draw_breeches_overlay(draw: ImageDraw.ImageDraw, P) -> None:
    """Draw a clean breeches layer over the upper legs.

    This sits on top of the leg strokes so the trousers read as an outer garment
    and hide any distracting upper-leg construction lines.
    """
    waist = [
        P(-24, -114),
        P(20, -114),
        P(28, -98),
        P(10, -82),
        P(-16, -84),
        P(-30, -100),
    ]
    _poly(draw, waist, COAT, OUTLINE, 0.9)
    left_breech = [
        P(-24, -110),
        P(-2, -112),
        P(6, -96),
        P(8, -60),
        P(-4, -48),
        P(-24, -54),
        P(-34, -74),
        P(-34, -96),
    ]
    right_breech = [
        P(0, -112),
        P(22, -110),
        P(32, -94),
        P(34, -70),
        P(24, -50),
        P(8, -48),
        P(-2, -66),
        P(-2, -94),
    ]
    _poly(draw, left_breech, BREECH, OUTLINE, 0.9)
    _poly(draw, right_breech, BREECH, OUTLINE, 0.9)
    # leg openings and gentle folds
    _line(draw, [P(-28, -74), P(-6, -72)], OUTLINE, 0.55)
    _line(draw, [P(2, -72), P(28, -72)], OUTLINE, 0.55)
    _line(draw, [P(-12, -104), P(-10, -58)], (206, 203, 196, 255), 0.85)
    _line(draw, [P(10, -104), P(14, -60)], (206, 203, 196, 255), 0.85)


def _render_frame(anim: str, frame_idx: int, nframes: int) -> Image.Image:
    img = Image.new(
        "RGBA", (WORK_FRAME_SIZE[0] * SUPER, WORK_FRAME_SIZE[1] * SUPER), (0, 0, 0, 0)
    )
    draw = ImageDraw.Draw(img, "RGBA")
    pose = Pose(anim, frame_idx, nframes)

    root = (
        WORK_FRAME_SIZE[0] * 0.47 + pose.root_x,
        WORK_FRAME_SIZE[1] * 0.77 + pose.root_y + pose.bob,
    )
    tilt = pose.tilt

    def P(x: float, y: float) -> Point:
        rx, ry = _rot(x, y, tilt)
        return (root[0] + rx, root[1] + ry)

    # far leg first
    far_hip = P(10, -104)
    _draw_leg(draw, far_hip, 92 + pose.right_leg, pose.right_lift, False)

    # coat tails behind body
    tail_l = [
        P(-18, -102),
        P(-6, -38),
        P(-20 + pose.coat_sway * 0.4, 22),
        P(-2, 18),
        P(10, -24),
        P(2, -102),
    ]
    tail_r = [
        P(8, -102),
        P(12, -34),
        P(28 + pose.coat_sway * 0.55, 18),
        P(44, 12),
        P(32, -40),
        P(26, -102),
    ]
    _poly(draw, tail_l, COAT, OUTLINE, 1.0)
    _poly(draw, tail_r, COAT, OUTLINE, 1.0)

    # torso/coat
    torso = [
        P(-34, -200),
        P(6, -214),
        P(36, -198),
        P(48, -148),
        P(44, -100),
        P(22, -76),
        P(-10, -72),
        P(-36, -94),
        P(-42, -148),
    ]
    _poly(draw, torso, COAT, OUTLINE, 1.2)
    lapel_l = [P(-12, -188), P(0, -194), P(-4, -116), P(-18, -100), P(-26, -124)]
    lapel_r = [P(10, -190), P(22, -188), P(34, -124), P(18, -98), P(6, -118)]
    _poly(draw, lapel_l, COAT_HI, OUTLINE, 0.6)
    _poly(draw, lapel_r, COAT_HI, OUTLINE, 0.6)
    vest = [
        P(-8, -196),
        P(14, -194),
        P(18, -104),
        P(-2, -92),
        P(-18, -108),
        P(-18, -176),
    ]
    _poly(draw, vest, VEST, OUTLINE, 0.8)
    cravat = [
        P(-2, -202),
        P(10, -202),
        P(14, -174 + pose.cravat * 0.2),
        P(6, -144),
        P(-4, -170),
        P(-10, -184),
    ]
    _poly(draw, cravat, CRAVAT, OUTLINE, 0.7)
    folds = [P(0, -182), P(8, -166), P(2, -152), P(12, -138)]
    _line(draw, folds, (214, 214, 210, 255), 0.8)
    # patriotic presidential sash and medallion for clearer role read
    sash = [P(-22, -186), P(-8, -194), P(34, -112), P(24, -98), P(-18, -176)]
    _poly(draw, sash, PRES_BLUE, OUTLINE, 0.55)
    sash_trim = [P(-18, -182), P(-8, -188), P(28, -114), P(20, -104), P(-14, -172)]
    _line(draw, sash_trim, PRES_WHITE, 1.2)
    _line(draw, [P(-16, -180), P(4, -140), P(22, -106)], PRES_RED, 0.8)
    _ellipse(draw, P(18, -112)[0], P(18, -112)[1], 5.0, 5.0, GOLD, OUTLINE, 0.45)

    # far arm
    far_shoulder = P(28, -182)
    far_elbow = P(40 + pose.right_arm * 0.18, -138 + pose.right_arm * 0.10)
    far_hand = P(46 + pose.right_arm * 0.28, -92 + pose.right_arm * 0.12)
    _line(draw, [far_shoulder, far_elbow, far_hand], COAT, 7.2)
    _line(draw, [far_shoulder, far_elbow, far_hand], OUTLINE, 1.0)
    _draw_hand(draw, far_hand, 4.2)

    # head + wig
    head_root = P(-2, -246)
    head_ang = tilt + pose.head

    def H(x: float, y: float) -> Point:
        rx, ry = _rot(x, y, head_ang)
        return (head_root[0] + rx, head_root[1] + ry)

    wig_back = [
        H(-28, -10),
        H(-22, -40),
        H(0, -54),
        H(20, -46),
        H(32, -20),
        H(30, 12),
        H(22, 26),
        H(12, 24),
        H(8, 0),
        H(-20, 8),
    ]
    _poly(draw, wig_back, WIG_SHADE, OUTLINE, 1.0)
    side_left = [H(-34, -4), H(-48, 10), H(-50, 28), H(-38, 42), H(-18, 34), H(-20, 12)]
    side_right = [H(30, -4), H(46, 6), H(50, 24), H(42, 42), H(24, 36), H(22, 8)]
    _poly(draw, side_left, WIG, OUTLINE, 0.8)
    _poly(draw, side_right, WIG, OUTLINE, 0.8)
    queue = [H(14, 18), H(20, 34), H(16, 54), H(8, 62), H(0, 54), H(4, 30)]
    _poly(draw, queue, WIG_SHADE, OUTLINE, 0.6)
    _poly(draw, [H(2, 42), H(22, 42), H(20, 48), H(4, 48)], PRES_BLUE, OUTLINE, 0.3)
    head = [
        H(-20, -18),
        H(-10, -36),
        H(12, -40),
        H(28, -26),
        H(30, 0),
        H(16, 20),
        H(-6, 24),
        H(-24, 10),
    ]
    _poly(draw, head, SKIN, OUTLINE, 1.0)
    _ellipse(draw, H(8, -2)[0], H(8, -2)[1], 9.0, 7.2, BLUSH, None, 0)
    _ellipse(draw, H(-8, 0)[0], H(-8, 0)[1], 8.5, 6.8, BLUSH, None, 0)

    # facial features
    if pose.x_eye:
        _line(draw, [H(-6, -6), H(0, 0)], OUTLINE, 0.8)
        _line(draw, [H(-6, 0), H(0, -6)], OUTLINE, 0.8)
        _line(draw, [H(12, -6), H(18, 0)], OUTLINE, 0.8)
        _line(draw, [H(12, 0), H(18, -6)], OUTLINE, 0.8)
    elif pose.blink:
        _line(draw, [H(-8, -3), H(0, -3)], OUTLINE, 0.8)
        _line(draw, [H(10, -4), H(18, -4)], OUTLINE, 0.8)
    else:
        _ellipse(
            draw,
            H(-4, -3)[0],
            H(-4, -3)[1],
            3.5,
            2.8,
            (239, 241, 240, 255),
            OUTLINE,
            0.4,
        )
        _ellipse(
            draw,
            H(14, -4)[0],
            H(14, -4)[1],
            3.5,
            2.8,
            (239, 241, 240, 255),
            OUTLINE,
            0.4,
        )
        _circle(draw, H(-3, -3), 0.9, (36, 44, 54, 255), (36, 44, 54, 255), 0.1)
        _circle(draw, H(15, -4), 0.9, (36, 44, 54, 255), (36, 44, 54, 255), 0.1)
        _line(draw, [H(-9, -8), H(-1, -10)], OUTLINE, 0.5)
        _line(draw, [H(10, -9), H(18, -10)], OUTLINE, 0.5)
    nose = [H(6, -2), H(10, 6), H(4, 10), H(2, 4)]
    _poly(draw, nose, SKIN_SHADE, OUTLINE, 0.3)
    if pose.open_mouth > 0.02:
        _ellipse(
            draw,
            H(7, 14)[0],
            H(7, 14)[1],
            4.6,
            2.4 + pose.open_mouth * 10.0,
            (102, 62, 66, 255),
            OUTLINE,
            0.4,
        )
    else:
        _line(draw, [H(2, 14), H(8, 15), H(14, 14)], (114, 76, 72, 255), 0.7)

    # near leg
    near_hip = P(-12, -104)
    _draw_leg(draw, near_hip, 92 + pose.left_leg, pose.left_lift, True)

    # Render breeches / coat-skirt over the tops of the legs so the pants sit
    # visually over the limbs instead of under them.
    _draw_breeches_overlay(draw, P)

    # near arm with weapon / gesturing
    near_shoulder = P(-28, -182)
    near_elbow = P(-40 + pose.left_arm * 0.20, -140 + pose.left_arm * 0.10)
    near_hand = P(-46 + pose.left_arm * 0.36, -96 + pose.left_arm * 0.14)
    _line(draw, [near_shoulder, near_elbow, near_hand], COAT, 7.6)
    _line(draw, [near_shoulder, near_elbow, near_hand], OUTLINE, 1.0)
    _draw_hand(draw, near_hand, 4.4)

    if anim == "decree":
        # The president presents / signs an executive parchment instead of
        # fighting like a generic duelist. This makes the role read clearly.
        paper_c = (near_hand[0] + 22 + pose.rapier * 10, near_hand[1] - 8)
        paper = [
            (paper_c[0] - 14, paper_c[1] - 11),
            (paper_c[0] + 18, paper_c[1] - 9),
            (paper_c[0] + 16, paper_c[1] + 13),
            (paper_c[0] - 16, paper_c[1] + 11),
        ]
        _poly(draw, paper, PARCHMENT, OUTLINE, 0.5)
        _line(
            draw,
            [(paper_c[0] - 9, paper_c[1] - 4), (paper_c[0] + 8, paper_c[1] - 3)],
            INK,
            0.5,
        )
        _line(
            draw,
            [(paper_c[0] - 8, paper_c[1] + 2), (paper_c[0] + 10, paper_c[1] + 3)],
            INK,
            0.5,
        )
        _circle(draw, (paper_c[0] + 9, paper_c[1] + 8), 2.6, SEAL, OUTLINE, 0.25)
        # quill / signature flourish
        quill_base = (near_hand[0] + 2, near_hand[1] - 3)
        quill_tip = (paper_c[0] - 10 + pose.rapier * 8, paper_c[1] + 7)
        _line(draw, [quill_base, quill_tip], INK, 0.8)
        _poly(
            draw,
            [
                (quill_base[0] - 2, quill_base[1] - 8),
                (quill_base[0] + 9, quill_base[1] - 2),
                (quill_base[0] + 1, quill_base[1] + 1),
            ],
            CRAVAT,
            OUTLINE,
            0.25,
        )
        if pose.rapier > 0.2:
            cx, cy = paper_c
            box = (_s(cx - 46), _s(cy - 28), _s(cx + 42), _s(cy + 32))
            draw.arc(box, 205, 340, fill=FX, width=_s(2.6))
    elif anim == "oath":
        # Raised right hand + constitution-style book = unmistakably presidential oath-taking.
        book_c = (near_hand[0] + 20 + pose.oath * 4, near_hand[1] - 6)
        book = [
            (book_c[0] - 14, book_c[1] - 11),
            (book_c[0] + 10, book_c[1] - 9),
            (book_c[0] + 14, book_c[1] + 8),
            (book_c[0] - 10, book_c[1] + 11),
        ]
        _poly(draw, book, BOOK, OUTLINE, 0.5)
        _line(
            draw,
            [(book_c[0] - 8, book_c[1] - 4), (book_c[0] + 8, book_c[1] - 2)],
            BOOK_EDGE,
            0.7,
        )
        _line(
            draw,
            [(book_c[0] - 6, book_c[1] + 2), (book_c[0] + 10, book_c[1] + 3)],
            BOOK_EDGE,
            0.6,
        )
        _line(
            draw,
            [(book_c[0] - 10, book_c[1] - 8), (book_c[0] - 10, book_c[1] + 8)],
            GOLD,
            0.5,
        )
        # oath energy lines above raised hand
        hand_c = far_hand
        for rr in [0, 1, 2]:
            cx, cy = hand_c[0] + 14, hand_c[1] - 22
            box = (
                _s(cx - 12 - rr * 7),
                _s(cy - 10 - rr * 6),
                _s(cx + 12 + rr * 7),
                _s(cy + 10 + rr * 6),
            )
            draw.arc(box, 210, 330, fill=FX, width=_s(1.6))

    # buttons and cuff details
    for y in [-180, -160, -140, -120]:
        _ellipse(draw, P(4, y)[0], P(4, y)[1], 2.0, 2.0, GOLD, OUTLINE, 0.3)
    _ellipse(draw, P(-10, -150)[0], P(-10, -150)[1], 4.0, 4.0, PRES_RED, OUTLINE, 0.35)
    _ellipse(draw, P(-10, -150)[0], P(-10, -150)[1], 2.0, 2.0, PRES_WHITE, OUTLINE, 0.2)
    _line(draw, [P(-36, -126), P(-28, -126)], CRAVAT, 0.8)
    _line(draw, [P(42, -124), P(50, -124)], CRAVAT, 0.8)

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
        outputs["preview"],
        outputs["canonical"],
        outputs["canonical_transparent"],
        outputs["actor"],
    ]


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        description="Render the more explicitly presidential portrait-style President sprite sheet."
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
