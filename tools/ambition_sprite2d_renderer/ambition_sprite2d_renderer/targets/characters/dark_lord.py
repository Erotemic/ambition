from __future__ import annotations

"""Standalone generator for a dark-lord armored boss sprite.

This is *not* a pirate.  It is a heavy dark-fantasy boss / elite enemy:
horned helmet, silver-black plate armor, glowing red visor, spiked pauldrons,
tattered cape, clawed gauntlets, and a rune halberd.

The animation rows are chosen to be useful for a side-scrolling action enemy:
- ``idle``: looming cape / breathing pose
- ``walk``: heavy armored advance
- ``slash``: halberd sweep
- ``cast``: red rune projectile / curse tell
- ``summon``: ground-spike / area denial tell
- ``guard``: armored block / parry stance
- ``hurt`` and ``death``

Only ``build_sheet`` is reused for the Ambition-compatible PNG/YAML/RON sheet
layout.  There is no GUI / registry wiring in this target.
"""

import argparse
import math
from pathlib import Path
from typing import List, Sequence, Tuple

from PIL import Image, ImageDraw

from ...authoring.tackon_sheet import build_sheet

ACTOR_METADATA = {
    "actor": {"character_id": "npc_dark_lord", "display_name": "Dark Lord"},
    "body": {
        "body_plan": "HumanoidBiped",
        "body_kind": "Wide",
        "mass_class": "Heavy",
        "traits": [
            "story",
            "humanoid",
            "enemy",
            "combatant",
            "story",
            "enemy",
            "boss_candidate",
        ],
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
    "actions": {"default_preset": "boss_bolt"},
    "visual": {"default_pose": "idle"},
    "tags": [
        "story",
        "humanoid",
        "enemy",
        "combatant",
        "story",
        "enemy",
        "boss_candidate",
    ],
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
        "projectile_origin": {
            "source": "explicit.profile.dark_lord",
            "point": {"x": 84.0, "y": 44.0},
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

TARGET_NAME = "dark_lord"
FRAME_SIZE = (320, 288)
WORK_FRAME_SIZE = (640, 576)
SUPER = 3
ROWS: List[Tuple[str, int, int]] = [
    ("idle", 6, 130),
    ("walk", 8, 100),
    ("slash", 8, 78),
    ("cast", 7, 90),
    ("summon", 7, 95),
    ("guard", 5, 105),
    ("hurt", 4, 90),
    ("death", 8, 115),
]

OUTLINE = (10, 9, 11, 255)
BLACK = (15, 15, 18, 255)
VOID = (3, 4, 6, 255)
CAPE = (12, 11, 14, 255)
CAPE_DARK = (4, 4, 6, 255)
CAPE_RED = (70, 13, 17, 230)
METAL = (154, 158, 160, 255)
METAL_HI = (222, 226, 226, 255)
METAL_DARK = (66, 69, 74, 255)
METAL_MID = (112, 118, 124, 255)
RUNE = (214, 22, 30, 255)
RUNE_HI = (255, 88, 66, 220)
RUNE_GLOW = (255, 28, 24, 120)
SHADOW_RED = (88, 16, 23, 220)
POLE = (38, 34, 34, 255)
POLE_HI = (88, 76, 70, 255)
DUST = (94, 70, 55, 135)


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


def _rect(center: Point, w: float, h: float, deg: float) -> List[Point]:
    hw = w * 0.5
    hh = h * 0.5
    pts = []
    for x, y in [(-hw, -hh), (hw, -hh), (hw, hh), (-hw, hh)]:
        rx, ry = _rot(x, y, deg)
        pts.append((center[0] + rx, center[1] + ry))
    return pts


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
        self.head_tilt = 0.0
        self.cape_sway = 0.0
        self.near_arm = 0.0
        self.far_arm = 0.0
        self.weapon = -58.0
        self.weapon_lift = 0.0
        self.near_leg = 0.0
        self.far_leg = 0.0
        self.near_lift = 0.0
        self.far_lift = 0.0
        self.eye = 1.0
        self.slash = 0.0
        self.cast = 0.0
        self.summon = 0.0
        self.guard = 0.0
        self.dead_t = 0.0
        self.hurt = 0.0

        if anim == "idle":
            self.bob = s * 1.4
            self.tilt = s * 1.5
            self.head_tilt = -2.0 + s * 1.5
            self.cape_sway = s * 7.0
            self.near_arm = 2.0 + s * 3.0
            self.far_arm = -4.0 - s * 2.5
            self.weapon = -58.0 + s * 3.0
            self.near_leg = -2.0 + c * 1.2
            self.far_leg = 3.0 - c * 1.0
            self.eye = 0.85 + max(0.0, s) * 0.2
        elif anim == "walk":
            self.root_x = s * 2.3
            self.bob = abs(s) * 2.8 - 0.6
            self.tilt = s * 3.0
            self.head_tilt = -s * 1.5
            self.cape_sway = -s * 10.0
            self.near_arm = -12.0 * s
            self.far_arm = 10.0 * s
            self.weapon = -62.0 - s * 6.0
            self.near_leg = -19.0 * s
            self.far_leg = 18.0 * s
            self.near_lift = max(0.0, -s) * 8.0
            self.far_lift = max(0.0, s) * 6.0
        elif anim == "slash":
            tt = _ease(t)
            hit = math.sin(tt * math.pi)
            self.root_x = _lerp(-7.0, 10.0, tt)
            self.bob = -hit * 3.0
            self.tilt = _lerp(-12.0, 18.0, tt)
            self.head_tilt = _lerp(-8.0, 5.0, tt)
            self.cape_sway = _lerp(12.0, -14.0, tt)
            self.near_arm = _lerp(-72.0, 42.0, tt)
            self.far_arm = _lerp(22.0, -18.0, tt)
            self.weapon = _lerp(-142.0, 24.0, tt)
            self.weapon_lift = -hit * 12.0 + tt * 10.0
            self.near_leg = _lerp(-8.0, 12.0, tt)
            self.far_leg = _lerp(8.0, -8.0, tt)
            self.eye = 1.2 + hit * 0.35
            self.slash = hit
        elif anim == "cast":
            tt = _ease(t)
            pulse = math.sin(tt * math.pi)
            self.root_x = -pulse * 2.0
            self.bob = -pulse * 1.0
            self.tilt = -5.0 + pulse * 2.0
            self.head_tilt = -8.0
            self.cape_sway = -8.0 + pulse * 6.0
            self.near_arm = _lerp(-8.0, -92.0, tt)
            self.far_arm = _lerp(-4.0, -44.0, tt)
            self.weapon = -74.0 + pulse * 4.0
            self.weapon_lift = -pulse * 4.0
            self.eye = 1.1 + pulse * 0.45
            self.cast = pulse
        elif anim == "summon":
            tt = _ease(t)
            pulse = math.sin(tt * math.pi)
            self.root_x = -pulse * 1.5
            self.bob = -pulse * 1.4
            self.tilt = _lerp(-4.0, 9.0, tt)
            self.head_tilt = -7.0 + pulse * 4.0
            self.cape_sway = _lerp(-12.0, 14.0, tt)
            self.near_arm = _lerp(-18.0, 54.0, tt)
            self.far_arm = _lerp(-8.0, 24.0, tt)
            self.weapon = _lerp(-70.0, -112.0, tt)
            self.weapon_lift = pulse * 10.0
            self.eye = 1.0 + pulse * 0.4
            self.summon = pulse
        elif anim == "guard":
            pulse = math.sin(t * math.pi)
            self.root_x = -pulse * 2.5
            self.bob = -pulse * 1.0
            self.tilt = -8.0
            self.head_tilt = -5.0
            self.cape_sway = -6.0
            self.near_arm = -36.0 + pulse * 7.0
            self.far_arm = -22.0
            self.weapon = -36.0
            self.weapon_lift = -12.0
            self.near_leg = -5.0
            self.far_leg = 7.0
            self.guard = 0.6 + pulse * 0.4
            self.eye = 1.15
        elif anim == "hurt":
            hit = math.sin(t * math.pi)
            shake = math.sin(t * math.pi * 5.0) * (1.0 - t)
            self.root_x = -hit * 6.0 + shake * 3.0
            self.bob = -hit * 2.0
            self.tilt = -16.0 * hit
            self.head_tilt = 14.0 * hit
            self.cape_sway = -14.0 * hit
            self.near_arm = 26.0 * hit
            self.far_arm = -18.0 * hit
            self.weapon = -48.0 + hit * 20.0
            self.near_leg = 8.0 * hit
            self.far_leg = -6.0 * hit
            self.hurt = hit
        elif anim == "death":
            tt = _ease(t)
            self.dead_t = tt
            self.root_x = tt * 14.0
            self.root_y = tt * 8.0
            self.bob = -tt * 4.0
            self.tilt = 82.0 * tt
            self.head_tilt = 18.0 * tt
            self.cape_sway = -20.0 * tt
            self.near_arm = _lerp(2.0, 64.0, tt)
            self.far_arm = _lerp(-4.0, -48.0, tt)
            self.weapon = _lerp(-58.0, -128.0, tt)
            self.weapon_lift = tt * 16.0
            self.near_leg = _lerp(-2.0, 28.0, tt)
            self.far_leg = _lerp(3.0, -24.0, tt)
            self.eye = max(0.0, 1.0 - tt * 1.5)


def _draw_halberd(
    draw: ImageDraw.ImageDraw, hand: Point, angle: float, front: bool = True
) -> None:
    def T(x: float, y: float) -> Point:
        rx, ry = _rot(x, y, angle)
        return (hand[0] + rx, hand[1] + ry)

    _line(draw, [T(-18, 8), T(50, 2), T(132, -4)], OUTLINE, 7.0)
    _line(draw, [T(-18, 8), T(50, 2), T(132, -4)], POLE, 4.3)
    _line(draw, [T(0, 6), T(112, -3)], POLE_HI, 1.0)
    for x in [4, 26, 48, 70, 92]:
        _line(draw, [T(x, -6), T(x + 6, 8)], METAL_DARK, 0.8)

    # grip and pommel
    _poly(draw, _rect(T(-4, 7), 16, 8, angle - 6), METAL_DARK, OUTLINE, 0.9)
    _circle(draw, T(-22, 8), 4.0, RUNE, OUTLINE, 0.7)

    # axe / spear head
    blade = [
        T(88, -35),
        T(118, -45),
        T(146, -30),
        T(156, -10),
        T(134, -8),
        T(160, 18),
        T(146, 42),
        T(116, 34),
        T(100, 15),
        T(76, 24),
        T(90, 2),
        T(74, -16),
    ]
    _poly(draw, blade, METAL, OUTLINE, 1.5)
    _poly(draw, [T(122, -42), T(134, -72), T(146, -40)], METAL_HI, OUTLINE, 0.8)
    _poly(draw, [T(134, 36), T(146, 66), T(156, 32)], METAL_HI, OUTLINE, 0.8)
    _line(draw, [T(86, -18), T(145, 28)], METAL_HI, 1.1)
    _line(draw, [T(94, 1), T(136, 0)], METAL_DARK, 1.0)
    if front:
        core = T(122, 3)
        _circle(draw, core, 5.0, RUNE, OUTLINE, 0.7)
        _line(draw, [T(112, 3), T(132, 3)], RUNE_HI, 1.0)
        _line(draw, [T(122, -7), T(122, 13)], RUNE_HI, 1.0)


def _draw_boot(
    draw: ImageDraw.ImageDraw, p: Point, toe: float, scale: float = 1.0
) -> None:
    greave = [
        (p[0] - 9 * scale, p[1] - 34 * scale),
        (p[0] + 10 * scale, p[1] - 33 * scale),
        (p[0] + 12 * scale, p[1] - 6 * scale),
        (p[0] - 10 * scale, p[1] - 4 * scale),
    ]
    _poly(draw, greave, METAL_DARK, OUTLINE, 1.0)
    _poly(
        draw,
        [
            (p[0] - 7 * scale, p[1] - 28 * scale),
            (p[0] + 8 * scale, p[1] - 29 * scale),
            (p[0] + 5 * scale, p[1] - 13 * scale),
            (p[0] - 6 * scale, p[1] - 11 * scale),
        ],
        METAL,
        OUTLINE,
        0.7,
    )
    foot = [
        (p[0] - 12 * scale, p[1] - 6 * scale),
        (p[0] + 12 * scale + toe * 2, p[1] - 8 * scale),
        (p[0] + 28 * scale + toe * 3, p[1] + 4 * scale),
        (p[0] + 6 * scale, p[1] + 9 * scale),
        (p[0] - 13 * scale, p[1] + 4 * scale),
    ]
    _poly(draw, foot, METAL, OUTLINE, 1.0)
    _poly(
        draw,
        [
            (p[0] + 14 * scale + toe * 2, p[1] - 6 * scale),
            (p[0] + 34 * scale + toe * 3, p[1] + 2 * scale),
            (p[0] + 15 * scale + toe, p[1] + 7 * scale),
        ],
        METAL_HI,
        OUTLINE,
        0.7,
    )


def _draw_helmet(draw: ImageDraw.ImageDraw, P, pose: Pose) -> None:
    # back horns
    _poly(draw, [P(-22, -156), P(-38, -206), P(-18, -170)], METAL_DARK, OUTLINE, 1.0)
    _poly(draw, [P(22, -156), P(42, -208), P(22, -168)], METAL_DARK, OUTLINE, 1.0)
    _poly(draw, [P(-10, -164), P(-8, -225), P(6, -166)], BLACK, OUTLINE, 1.0)
    _poly(draw, [P(9, -164), P(18, -224), P(22, -166)], BLACK, OUTLINE, 1.0)

    helm = [
        P(-28, -154),
        P(-14, -174),
        P(12, -176),
        P(30, -158),
        P(28, -132),
        P(16, -115),
        P(-8, -113),
        P(-27, -128),
    ]
    _poly(draw, helm, METAL, OUTLINE, 1.4)
    _poly(
        draw,
        [P(-22, -150), P(-2, -164), P(18, -156), P(24, -139), P(4, -144), P(-20, -136)],
        METAL_HI,
        OUTLINE,
        0.8,
    )
    _poly(
        draw, [P(-18, -138), P(21, -141), P(18, -127), P(-16, -125)], VOID, OUTLINE, 0.7
    )

    # red visor
    glow = max(0.0, pose.eye)
    _line(
        draw, [P(-15, -135), P(-3, -132), P(11, -134), P(20, -138)], RUNE_HI, 1.5 + glow
    )
    _line(draw, [P(-14, -134), P(18, -137)], RUNE, 2.0)

    faceplate = [P(-16, -124), P(16, -126), P(11, -106), P(-12, -105)]
    _poly(draw, faceplate, METAL_MID, OUTLINE, 0.9)
    for x in [-8, -3, 2, 7]:
        _line(draw, [P(x, -121), P(x - 1, -110)], METAL_DARK, 0.7)
    _poly(draw, [P(-20, -150), P(-34, -159), P(-26, -136)], METAL_DARK, OUTLINE, 0.8)
    _poly(draw, [P(20, -151), P(36, -162), P(27, -136)], METAL_DARK, OUTLINE, 0.8)


def _draw_torso(draw: ImageDraw.ImageDraw, P, pose: Pose) -> None:
    # cape behind shoulders
    cape = [
        P(-46, -118),
        P(-86 - pose.cape_sway * 0.4, -92),
        P(-104 - pose.cape_sway * 0.6, -26),
        P(-75 - pose.cape_sway * 0.7, 60),
        P(-42 - pose.cape_sway * 0.3, 28),
        P(-22, -44),
        P(0, -50),
        P(32, -42),
        P(68 + pose.cape_sway * 0.3, 72),
        P(90 + pose.cape_sway * 0.7, 40),
        P(84 + pose.cape_sway * 0.5, -22),
        P(54, -108),
    ]
    _poly(draw, cape, CAPE, OUTLINE, 1.2)
    inner = [
        P(-20, -58),
        P(0, -48),
        P(30, -45),
        P(58 + pose.cape_sway * 0.4, 42),
        P(22, 54),
        P(-5, -3),
        P(-44 - pose.cape_sway * 0.2, 42),
    ]
    _poly(draw, inner, CAPE_RED, None, 0)
    # tears
    for x, y, h in [(-67, 35, 28), (-20, 50, 24), (40, 54, 34), (73, 30, 24)]:
        _poly(draw, [P(x, y), P(x + 9, y + h), P(x + 18, y)], CAPE_DARK, None, 0)

    # pauldrons and spikes
    left = [P(-56, -112), P(-34, -132), P(-6, -125), P(-10, -98), P(-45, -88)]
    right = [P(8, -127), P(40, -135), P(62, -114), P(53, -90), P(17, -98)]
    _poly(draw, left, METAL_DARK, OUTLINE, 1.3)
    _poly(draw, right, METAL_DARK, OUTLINE, 1.3)
    _poly(draw, [P(-50, -123), P(-72, -170), P(-36, -132)], METAL_HI, OUTLINE, 0.8)
    _poly(draw, [P(-28, -130), P(-32, -181), P(-14, -128)], METAL_HI, OUTLINE, 0.8)
    _poly(draw, [P(34, -132), P(62, -182), P(47, -126)], METAL_HI, OUTLINE, 0.8)
    _poly(draw, [P(55, -116), P(86, -152), P(62, -101)], METAL_HI, OUTLINE, 0.8)
    _circle(draw, P(-31, -105), 8.0, METAL, OUTLINE, 0.8)
    _circle(draw, P(34, -107), 8.0, METAL, OUTLINE, 0.8)
    _line(draw, [P(-36, -105), P(-26, -105)], RUNE, 0.9)
    _line(draw, [P(29, -107), P(39, -107)], RUNE, 0.9)

    # chest armor
    chest = [
        P(-30, -108),
        P(-12, -126),
        P(15, -126),
        P(34, -105),
        P(28, -66),
        P(0, -48),
        P(-28, -65),
    ]
    _poly(draw, chest, METAL, OUTLINE, 1.4)
    _poly(
        draw,
        [P(-23, -101), P(-4, -116), P(-2, -58), P(-26, -67)],
        METAL_HI,
        OUTLINE,
        0.8,
    )
    _poly(
        draw, [P(6, -116), P(27, -101), P(25, -68), P(2, -58)], METAL_DARK, OUTLINE, 0.8
    )
    _line(draw, [P(0, -119), P(0, -51)], OUTLINE, 0.8)

    # rune sigil on chest
    _circle(draw, P(0, -86), 11.0, (50, 12, 16, 220), OUTLINE, 0.7)
    _line(draw, [P(0, -99), P(0, -73)], RUNE_HI, 1.2)
    _line(draw, [P(-10, -86), P(10, -86)], RUNE_HI, 1.0)
    _line(draw, [P(-6, -94), P(0, -86), P(7, -96)], RUNE, 0.9)
    _line(draw, [P(-7, -76), P(0, -86), P(8, -76)], RUNE, 0.9)

    # waist / tabard
    belt = [P(-31, -62), P(31, -62), P(26, -48), P(-28, -48)]
    _poly(draw, belt, METAL_DARK, OUTLINE, 1.0)
    _poly(draw, [P(-8, -66), P(8, -66), P(8, -46), P(-8, -46)], METAL_HI, OUTLINE, 0.8)
    tabard = [P(-15, -48), P(17, -48), P(20, 42), P(2, 64), P(-17, 42)]
    _poly(draw, tabard, BLACK, OUTLINE, 1.0)
    _line(draw, [P(0, -42), P(0, 52)], RUNE, 0.8)
    _line(draw, [P(-8, 8), P(0, 28), P(9, 8)], SHADOW_RED, 0.8)


def _draw_limbs(draw: ImageDraw.ImageDraw, P, pose: Pose) -> Tuple[Point, Point]:
    # legs behind tabard
    far_hip = P(-16, -45)
    near_hip = P(18, -45)
    far_knee = P(-18 + pose.far_leg * 0.22, -7)
    near_knee = P(20 + pose.near_leg * 0.22, -5)
    far_foot = P(-23 + pose.far_leg * 0.20, 54 - pose.far_lift)
    near_foot = P(24 + pose.near_leg * 0.20, 55 - pose.near_lift)
    _line(draw, [far_hip, far_knee, far_foot], METAL_DARK, 10.5)
    _line(draw, [far_hip, far_knee, far_foot], OUTLINE, 1.3)
    _draw_boot(draw, far_foot, -1, 0.92)
    _line(draw, [near_hip, near_knee, near_foot], METAL_MID, 12.5)
    _line(draw, [near_hip, near_knee, near_foot], OUTLINE, 1.5)
    _draw_boot(draw, near_foot, 1, 1.05)

    # arms
    far_shoulder = P(-35, -99)
    far_elbow = P(-49 + pose.far_arm * 0.09, -70 + pose.far_arm * 0.16)
    far_hand = P(-35 + pose.far_arm * 0.18, -36 + pose.far_arm * 0.21)
    _line(draw, [far_shoulder, far_elbow, far_hand], METAL_DARK, 10.0)
    _line(draw, [far_shoulder, far_elbow, far_hand], OUTLINE, 1.4)
    _poly(draw, _rect(far_elbow, 16, 20, pose.tilt * 0.2), METAL, OUTLINE, 0.8)

    near_shoulder = P(38, -99)
    near_elbow = P(
        54 + pose.near_arm * 0.08, -68 + pose.near_arm * 0.16 + pose.weapon_lift * 0.15
    )
    near_hand = P(
        39 + pose.near_arm * 0.19, -34 + pose.near_arm * 0.23 + pose.weapon_lift
    )
    _line(draw, [near_shoulder, near_elbow, near_hand], METAL_MID, 11.0)
    _line(draw, [near_shoulder, near_elbow, near_hand], OUTLINE, 1.4)
    _poly(draw, _rect(near_elbow, 18, 22, pose.tilt * 0.2), METAL_HI, OUTLINE, 0.8)

    for hand, front in [(far_hand, False), (near_hand, True)]:
        _circle(draw, hand, 5.2 if front else 4.6, VOID, OUTLINE, 0.7)
        for dx in [-4, 0, 4]:
            _poly(
                draw,
                [
                    (hand[0] + dx, hand[1]),
                    (hand[0] + dx + 4, hand[1] + 9),
                    (hand[0] + dx - 2, hand[1] + 6),
                ],
                METAL_HI if front else METAL,
                OUTLINE,
                0.35,
            )

    return far_hand, near_hand


def _render_frame(anim: str, frame_idx: int, nframes: int) -> Image.Image:
    img = Image.new(
        "RGBA", (WORK_FRAME_SIZE[0] * SUPER, WORK_FRAME_SIZE[1] * SUPER), (0, 0, 0, 0)
    )
    draw = ImageDraw.Draw(img, "RGBA")
    pose = Pose(anim, frame_idx, nframes)

    root = (
        WORK_FRAME_SIZE[0] * 0.47 + pose.root_x + pose.dead_t * 7.0,
        WORK_FRAME_SIZE[1] * 0.70 + pose.root_y + pose.bob,
    )
    tilt = pose.tilt

    def P(x: float, y: float) -> Point:
        rx, ry = _rot(x, y, tilt)
        return (root[0] + rx, root[1] + ry)

    weapon_front = anim in {"slash", "guard"}
    if not weapon_front:
        back_hand = P(37 + pose.near_arm * 0.13, -34 + pose.weapon_lift)
        _draw_halberd(draw, back_hand, pose.weapon + tilt, front=False)

    # offensive FX behind body
    if anim == "slash" and pose.slash > 0.12:
        cx, cy = P(45, -68)
        box = (_s(cx - 110), _s(cy - 100), _s(cx + 130), _s(cy + 90))
        draw.arc(
            box, 202, 342, fill=(255, 46, 38, 145), width=_s(6.5 + pose.slash * 2.0)
        )
        draw.arc(box, 214, 330, fill=(255, 184, 140, 110), width=_s(2.5))

    _draw_torso(draw, P, pose)
    far_hand, near_hand = _draw_limbs(draw, P, pose)
    _poly(
        draw,
        [P(-9, -128), P(11, -128), P(9, -112), P(-8, -112)],
        METAL_DARK,
        OUTLINE,
        0.8,
    )
    _draw_helmet(draw, P, pose)

    if weapon_front:
        hand = P(
            39 + pose.near_arm * 0.19, -34 + pose.near_arm * 0.23 + pose.weapon_lift
        )
        _draw_halberd(draw, hand, pose.weapon + tilt, front=True)

    # magic / guard / summon effects in front
    if anim == "cast" and pose.cast > 0.12:
        palm = far_hand
        r = 9 + 10 * pose.cast
        _circle(draw, palm, r, RUNE_GLOW, None, 0)
        _circle(draw, palm, r * 0.45, RUNE, OUTLINE, 0.5)
        for ang in [0, 72, 144, 216, 288]:
            a = math.radians(ang + frame_idx * 12)
            p1 = (palm[0] + math.cos(a) * r * 0.7, palm[1] + math.sin(a) * r * 0.7)
            p2 = (palm[0] + math.cos(a) * r * 1.2, palm[1] + math.sin(a) * r * 1.2)
            _line(draw, [p1, p2], RUNE_HI, 0.8)
    if anim == "guard" and pose.guard > 0.1:
        cx, cy = P(42, -80)
        shield = [
            (cx - 30, cy - 52),
            (cx + 36, cy - 34),
            (cx + 26, cy + 42),
            (cx - 34, cy + 50),
            (cx - 48, cy - 4),
        ]
        _poly(
            draw,
            shield,
            (190, 210, 220, int(56 + 50 * pose.guard)),
            (245, 80, 70, int(120 + 50 * pose.guard)),
            0.8,
        )
        _line(draw, [(cx - 26, cy), (cx + 26, cy)], RUNE_HI, 1.0)
        _line(draw, [(cx, cy - 34), (cx, cy + 34)], RUNE_HI, 1.0)
    if anim == "summon" and pose.summon > 0.12:
        ground_y = P(0, 64)[1]
        for i, xoff in enumerate([-70, -42, -14, 18, 48, 76]):
            height = (16 + (i % 3) * 8) * pose.summon
            base_x = P(xoff, 60)[0]
            _poly(
                draw,
                [
                    (base_x - 7, ground_y + 5),
                    (base_x + 3, ground_y - height),
                    (base_x + 12, ground_y + 6),
                ],
                METAL_DARK,
                OUTLINE,
                0.5,
            )
            _line(
                draw,
                [(base_x + 1, ground_y - height * 0.8), (base_x + 6, ground_y + 2)],
                RUNE,
                0.6,
            )
    if anim in {"slash", "summon", "walk"}:
        # grounded dust / sparks
        if pose.slash > 0.4 or pose.summon > 0.3 or anim == "walk":
            amount = max(pose.slash, pose.summon, 0.2 if anim == "walk" else 0.0)
            for dx in [-34, -12, 18, 44]:
                c = P(dx, 62)
                _poly(
                    draw,
                    [
                        (c[0] - 3 * amount, c[1]),
                        (c[0] + 4 * amount, c[1] - 5 * amount),
                        (c[0] + 8 * amount, c[1] + 1),
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
        description="Render the standalone dark-lord armored boss spritesheet."
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
