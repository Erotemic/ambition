from __future__ import annotations

"""Standalone generator for a big classic green T-rex enemy.

Design goals:
- unmistakably *bipedal* silhouette: two powerful hind legs and two tiny arms
- clear side-view read for a 2D side scroller
- big classic T-rex proportions: oversized head, deep torso, long balancing tail
- fun attack tells: bite, roar, tail swipe, stomp, charge

Only ``build_sheet`` is reused for PNG / YAML / RON emission.
"""

import argparse
import math
from pathlib import Path
from typing import List, Sequence, Tuple

from PIL import Image, ImageDraw

from ...authoring.tackon_sheet import build_sheet

RGBA = Tuple[int, int, int, int]
Point = Tuple[float, float]

TARGET_NAME = "trex_enemy"
FRAME_SIZE = (416, 320)
WORK_FRAME_SIZE = (832, 640)
SUPER = 4
ROWS: List[Tuple[str, int, int]] = [
    ("idle", 6, 120),
    ("walk", 8, 90),
    ("charge", 8, 76),
    ("bite", 7, 78),
    ("roar", 6, 104),
    ("tail_swipe", 7, 82),
    ("stomp", 6, 92),
    ("hurt", 4, 90),
    ("death", 8, 110),
]

ACTOR_METADATA = {
    "actor": {
        "character_id": "npc_trex_enemy",
        "display_name": "T-Rex Enemy",
    },
    "body": {
        "body_plan": "BeastBiped",
        "body_kind": "Wide",
        "mass_class": "Heavy",
        "locomotion_hint": "HeavyWalk",
        "traits": ["enemy", "beast", "dinosaur", "heavy", "no_hands", "stomper"],
    },
    "capabilities": {
        "traversal": {
            "walk": True,
            "jump": None,
            "climb": None,
            "crawl": None,
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
    "brain": {"default_preset": "melee_brute_brute"},
    "actions": {"default_preset": "brute_lunge"},
    "animation_bindings": {
        "default": {"animation": "idle", "events": []},
        "locomotion.walk_heavy": {"animation": "walk", "events": []},
        "action.special.charge": {
            "animation": "charge",
            "events": [
                {"t": 0.20, "event": "charge_commit", "source": "trex_enemy.charge"}
            ],
        },
        "action.melee.primary": {
            "animation": "bite",
            "events": [
                {
                    "t": 0.32,
                    "event": "hitbox_active_start",
                    "source": "trex_enemy.bite",
                },
                {"t": 0.55, "event": "hitbox_active_end", "source": "trex_enemy.bite"},
            ],
        },
        "action.melee.tail_sweep": {
            "animation": "tail_swipe",
            "events": [
                {
                    "t": 0.36,
                    "event": "hitbox_active_start",
                    "source": "trex_enemy.tail_swipe",
                },
                {
                    "t": 0.66,
                    "event": "hitbox_active_end",
                    "source": "trex_enemy.tail_swipe",
                },
            ],
        },
        "action.melee.stomp": {
            "animation": "stomp",
            "events": [
                {"t": 0.48, "event": "ground_impact", "source": "trex_enemy.stomp"}
            ],
        },
        "interaction.roar": {
            "animation": "roar",
            "events": [{"t": 0.44, "event": "sfx_cue", "source": "trex_enemy.roar"}],
        },
        "damage.hit": {"animation": "hurt", "events": []},
        "lifecycle.death": {"animation": "death", "events": []},
    },
    "sockets": {
        "head": {"source": "trex_enemy.geometry", "point": {"x": 296.0, "y": 82.0}},
        "mouth": {"source": "trex_enemy.geometry", "point": {"x": 346.0, "y": 108.0}},
        "roar_origin": {
            "source": "trex_enemy.geometry",
            "point": {"x": 360.0, "y": 110.0},
        },
        "tail_base": {
            "source": "trex_enemy.geometry",
            "point": {"x": 130.0, "y": 158.0},
        },
        "tail_tip": {"source": "trex_enemy.geometry", "point": {"x": 48.0, "y": 164.0}},
        "foot_l": {"source": "trex_enemy.geometry", "point": {"x": 180.0, "y": 286.0}},
        "foot_r": {"source": "trex_enemy.geometry", "point": {"x": 244.0, "y": 286.0}},
    },
    "tags": ["enemy", "heavy", "dinosaur"],
}

OUTLINE = (22, 16, 12, 255)
GREEN_DARK = (54, 109, 44, 255)
GREEN = (74, 148, 58, 255)
GREEN_LIGHT = (110, 185, 88, 255)
BELLY = (188, 172, 112, 255)
BELLY_SHADE = (146, 128, 84, 255)
MOUTH = (110, 46, 52, 255)
TONGUE = (182, 98, 106, 255)
TOOTH = (247, 242, 224, 255)
CLAW = (234, 228, 208, 255)
EYE = (238, 216, 96, 255)
PUPIL = (34, 22, 17, 255)
DUST = (135, 112, 76, 170)
FX = (255, 235, 164, 170)
ROAR = (230, 244, 255, 118)
SCAR = (192, 115, 88, 255)


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
        self.body_tilt = 0.0
        self.neck = -6.0
        self.head = -6.0
        self.jaw = 8.0
        self.tail_base = -8.0
        self.tail_mid = -14.0
        self.tail_tip = -18.0
        self.near_leg = 0.0
        self.far_leg = 0.0
        self.near_knee = 0.0
        self.far_knee = 0.0
        self.near_lift = 0.0
        self.far_lift = 0.0
        self.near_arm = 0.0
        self.far_arm = 0.0
        self.near_reach = 0.0
        self.far_reach = 0.0
        self.bite_fx = 0.0
        self.roar = 0.0
        self.swipe = 0.0
        self.dust = 0.0
        self.dead_t = 0.0
        self.blink = False
        self.x_eye = False

        if anim == "idle":
            self.bob = s * 2.2
            self.body_tilt = s * 1.8
            self.neck = -6.0 + s * 1.8
            self.head = -5.0 + s * 1.5
            self.jaw = 7.0 + max(0.0, s) * 2.0
            self.near_leg = -4.0 + c * 2.0
            self.far_leg = 4.0 - c * 1.8
            self.tail_base = -10.0 + s * 4.0
            self.tail_mid = -16.0 + s * 6.0
            self.tail_tip = -20.0 + s * 8.0
            self.near_arm = -4.0 + s * 3.0
            self.far_arm = 3.0 - s * 2.5
            self.blink = frame_idx == nframes - 1
        elif anim == "walk":
            self.root_x = s * 2.6
            self.bob = abs(s) * 4.0 - 1.0
            self.body_tilt = s * 3.0
            self.neck = -7.0 - s * 1.2
            self.head = -6.0 - s * 1.8
            self.near_leg = -24.0 * s
            self.far_leg = 22.0 * s
            self.near_knee = 10.0 * max(0.0, -s)
            self.far_knee = 9.0 * max(0.0, s)
            self.near_lift = max(0.0, -s) * 10.0
            self.far_lift = max(0.0, s) * 8.0
            self.tail_base = -8.0 - s * 8.0
            self.tail_mid = -16.0 - s * 12.0
            self.tail_tip = -22.0 - s * 16.0
            self.near_arm = -8.0 * s
            self.far_arm = 6.0 * s
        elif anim == "charge":
            self.root_x = frame_idx * 3.0 + s * 1.0
            self.bob = abs(s) * 4.4 - 0.8
            self.body_tilt = -9.0 + s * 2.6
            self.neck = -16.0 - s * 3.5
            self.head = -13.0 - s * 4.0
            self.jaw = 8.0 + max(0.0, -s) * 5.0
            self.near_leg = -28.0 * s
            self.far_leg = 24.0 * s
            self.near_knee = 12.0 * max(0.0, -s)
            self.far_knee = 11.0 * max(0.0, s)
            self.near_lift = max(0.0, -s) * 10.0
            self.far_lift = max(0.0, s) * 8.0
            self.tail_base = -2.0 - s * 10.0
            self.tail_mid = -8.0 - s * 15.0
            self.tail_tip = -12.0 - s * 20.0
            self.near_arm = -12.0 * s
            self.far_arm = 10.0 * s
            self.dust = 0.45 + abs(s) * 0.55
        elif anim == "bite":
            tt = _ease(t)
            hit = math.sin(tt * math.pi)
            self.root_x = _lerp(-8.0, 24.0, tt)
            self.bob = -hit * 3.4
            self.body_tilt = _lerp(-4.0, 10.0, tt)
            self.neck = _lerp(-12.0, 20.0, tt)
            self.head = _lerp(-16.0, 24.0, tt)
            self.jaw = 8.0 + hit * 34.0
            self.near_leg = _lerp(-6.0, 12.0, tt)
            self.far_leg = _lerp(4.0, -8.0, tt)
            self.tail_base = _lerp(-6.0, -20.0, tt)
            self.tail_mid = _lerp(-12.0, -30.0, tt)
            self.tail_tip = _lerp(-18.0, -38.0, tt)
            self.near_arm = _lerp(0.0, 8.0, tt)
            self.far_arm = _lerp(0.0, 6.0, tt)
            self.bite_fx = hit
        elif anim == "roar":
            tt = math.sin(t * math.pi)
            self.bob = -tt * 1.5
            self.body_tilt = -2.0
            self.neck = -28.0 * tt
            self.head = -32.0 * tt
            self.jaw = 16.0 + tt * 34.0
            self.near_leg = -5.0 * tt
            self.far_leg = 4.0 * tt
            self.tail_base = -8.0 + tt * 5.0
            self.tail_mid = -14.0 + tt * 7.0
            self.tail_tip = -18.0 + tt * 9.0
            self.roar = tt
        elif anim == "tail_swipe":
            tt = _ease(t)
            sweep = math.sin(tt * math.pi)
            self.root_x = _lerp(8.0, -10.0, tt)
            self.bob = -sweep * 2.0
            self.body_tilt = _lerp(8.0, -14.0, tt)
            self.neck = _lerp(8.0, -16.0, tt)
            self.head = _lerp(4.0, -10.0, tt)
            self.near_leg = _lerp(5.0, -8.0, tt)
            self.far_leg = _lerp(-8.0, 12.0, tt)
            self.tail_base = _lerp(26.0, -34.0, tt)
            self.tail_mid = _lerp(44.0, -58.0, tt)
            self.tail_tip = _lerp(62.0, -84.0, tt)
            self.near_arm = _lerp(-4.0, 8.0, tt)
            self.swipe = sweep
        elif anim == "stomp":
            tt = _ease(t)
            slam = math.sin(tt * math.pi)
            self.root_x = _lerp(-4.0, 8.0, tt)
            self.bob = -slam * 7.0
            self.body_tilt = _lerp(-8.0, 14.0, tt)
            self.neck = _lerp(-10.0, 10.0, tt)
            self.head = _lerp(-8.0, 8.0, tt)
            self.jaw = 8.0 + slam * 8.0
            self.near_leg = _lerp(-24.0, 24.0, tt)
            self.far_leg = _lerp(10.0, -8.0, tt)
            self.near_knee = 16.0 * (1.0 - tt)
            self.near_lift = (1.0 - tt) * 24.0
            self.tail_base = _lerp(-12.0, -4.0, tt)
            self.tail_mid = _lerp(-18.0, -8.0, tt)
            self.tail_tip = _lerp(-24.0, -12.0, tt)
            self.dust = max(0.0, tt - 0.4) * 1.9
        elif anim == "hurt":
            hit = math.sin(t * math.pi)
            shake = math.sin(t * math.pi * 5.0) * (1.0 - t)
            self.root_x = -hit * 8.0 + shake * 3.0
            self.bob = -hit * 2.0
            self.body_tilt = -10.0 * hit
            self.neck = 12.0 * hit
            self.head = 18.0 * hit
            self.jaw = 14.0 * hit
            self.near_leg = -6.0 * hit
            self.far_leg = 5.0 * hit
            self.tail_base = 8.0 * hit
            self.tail_mid = 12.0 * hit
            self.tail_tip = 18.0 * hit
            self.near_arm = 10.0 * hit
        elif anim == "death":
            tt = _ease(t)
            self.dead_t = tt
            self.root_x = -tt * 28.0
            self.root_y = tt * 8.0
            self.bob = -tt * 7.0
            self.body_tilt = 84.0 * tt
            self.neck = -20.0 * tt
            self.head = -14.0 * tt
            self.jaw = 18.0 + tt * 10.0
            self.near_leg = -16.0 * tt
            self.far_leg = 20.0 * tt
            self.tail_base = -10.0 - 24.0 * tt
            self.tail_mid = -18.0 - 38.0 * tt
            self.tail_tip = -24.0 - 50.0 * tt
            self.near_arm = -8.0 * tt
            self.far_arm = 4.0 * tt
            self.x_eye = tt > 0.58


def _draw_hind_leg(
    draw: ImageDraw.ImageDraw,
    hip: Point,
    thigh_deg: float,
    knee_bend: float,
    foot_lift: float,
    *,
    scale: float,
    front: bool,
) -> Point:
    thigh_len = 68 * scale
    shin_len = 74 * scale
    foot_len = 42 * scale
    knee = (
        hip[0] + thigh_len * math.cos(math.radians(thigh_deg)),
        hip[1] + thigh_len * math.sin(math.radians(thigh_deg)),
    )
    shin_deg = thigh_deg + knee_bend
    ankle = (
        knee[0] + shin_len * math.cos(math.radians(shin_deg)),
        knee[1] + shin_len * math.sin(math.radians(shin_deg)) - foot_lift,
    )
    col = GREEN if front else GREEN_DARK
    w = 16 * scale if front else 13 * scale
    _line(draw, [hip, knee, ankle], col, w)
    _line(draw, [hip, knee, ankle], OUTLINE, 1.5)
    _circle(draw, knee, 7.0 * scale, GREEN_LIGHT if front else GREEN, OUTLINE, 0.8)
    foot = [
        (ankle[0] - 8 * scale, ankle[1] - 5 * scale),
        (ankle[0] + foot_len * 0.52, ankle[1] - 7 * scale),
        (ankle[0] + foot_len, ankle[1] + 3 * scale),
        (ankle[0] + foot_len * 0.62, ankle[1] + 10 * scale),
        (ankle[0] - 6 * scale, ankle[1] + 8 * scale),
    ]
    _poly(draw, foot, GREEN_LIGHT if front else GREEN, OUTLINE, 1.0)
    for frac in [0.52, 0.76, 0.96]:
        tip = (ankle[0] + foot_len * frac, ankle[1] + 5 * scale)
        _poly(
            draw,
            [
                tip,
                (tip[0] + 6 * scale, tip[1] - 2 * scale),
                (tip[0] + 3 * scale, tip[1] + 5 * scale),
            ],
            CLAW,
            OUTLINE,
            0.4,
        )
    return ankle


def _draw_tiny_arm(
    draw: ImageDraw.ImageDraw, shoulder: Point, ang: float, reach: float, *, front: bool
) -> Point:
    upper = 18 if front else 16
    lower = 16 + reach
    elbow = (
        shoulder[0] + upper * math.cos(math.radians(ang)),
        shoulder[1] + upper * math.sin(math.radians(ang)),
    )
    hand = (
        elbow[0] + lower * math.cos(math.radians(ang + 14)),
        elbow[1] + lower * math.sin(math.radians(ang + 14)),
    )
    col = GREEN_LIGHT if front else GREEN_DARK
    w = 6.8 if front else 5.2
    _line(draw, [shoulder, elbow, hand], col, w)
    _line(draw, [shoulder, elbow, hand], OUTLINE, 1.0)
    for i in range(2):
        claw = (hand[0] + 4 + i * 2.8, hand[1] + 2 + i * 1.5)
        _poly(
            draw,
            [hand, (claw[0] + 5, claw[1] - 1), (claw[0] + 3, claw[1] + 4)],
            CLAW,
            OUTLINE,
            0.35,
        )
    return hand


def _render_frame(anim: str, frame_idx: int, nframes: int) -> Image.Image:
    img = Image.new(
        "RGBA", (WORK_FRAME_SIZE[0] * SUPER, WORK_FRAME_SIZE[1] * SUPER), (0, 0, 0, 0)
    )
    draw = ImageDraw.Draw(img, "RGBA")
    pose = Pose(anim, frame_idx, nframes)

    root = (
        WORK_FRAME_SIZE[0] * 0.33 + pose.root_x,
        WORK_FRAME_SIZE[1] * 0.72 + pose.root_y + pose.bob,
    )
    body_angle = pose.body_tilt

    def P(x: float, y: float, extra: float = 0.0) -> Point:
        rx, ry = _rot(x, y, body_angle + extra)
        return (root[0] + rx, root[1] + ry)

    hip = P(0, 0)

    # Tail first, big and classic
    tail0 = P(-36, -40)
    tail1 = P(-112, -82, pose.tail_base)
    tail2 = P(-214, -92, pose.tail_mid)
    tail3 = P(-322, -70, pose.tail_tip)
    tail = [
        (tail0[0] - 8, tail0[1] - 18),
        (tail1[0] - 18, tail1[1] - 18),
        (tail2[0] - 12, tail2[1] - 14),
        (tail3[0] - 4, tail3[1] - 6),
        (tail3[0] + 7, tail3[1] + 8),
        (tail2[0] + 16, tail2[1] + 12),
        (tail1[0] + 24, tail1[1] + 18),
        (tail0[0] + 18, tail0[1] + 16),
    ]
    _poly(draw, tail, GREEN_DARK, OUTLINE, 1.4)
    _line(draw, [tail0, tail1, tail2, tail3], GREEN_LIGHT, 2.0)

    # Far hind leg (still only one of the two biped legs)
    far_hip = P(-10, -4)
    _draw_hind_leg(
        draw,
        far_hip,
        90 + pose.far_leg,
        34 + pose.far_knee,
        pose.far_lift,
        scale=0.92,
        front=False,
    )

    # Main torso
    torso = [
        P(-44, -96),
        P(34, -132),
        P(138, -128),
        P(210, -96),
        P(226, -44),
        P(182, -6),
        P(96, 14),
        P(6, 8),
        P(-40, -18),
    ]
    _poly(draw, torso, GREEN, OUTLINE, 1.8)
    belly = [
        P(6, -74),
        P(100, -76),
        P(180, -48),
        P(172, -4),
        P(84, 16),
        P(8, -4),
        P(-10, -30),
    ]
    _poly(draw, belly, BELLY, OUTLINE, 1.1)
    _line(draw, [P(-6, -82), P(64, -116), P(148, -112)], GREEN_LIGHT, 2.2)
    _line(draw, [P(12, -52), P(82, -42), P(162, -24)], BELLY_SHADE, 1.7)

    # Subtle back bumps, not fantasy spikes
    for bx, by, h in [
        (-12, -104, 12),
        (24, -120, 15),
        (70, -126, 16),
        (120, -122, 15),
        (168, -108, 11),
    ]:
        a = P(bx, by)
        _poly(
            draw,
            [(a[0] - 7, a[1] + 4), (a[0] + 4, a[1] - h), (a[0] + 12, a[1] + 2)],
            GREEN_LIGHT,
            OUTLINE,
            0.6,
        )

    # Battle wear
    _line(draw, [P(62, -84), P(78, -72), P(88, -92)], SCAR, 1.3)
    _line(draw, [P(104, -58), P(116, -46)], SCAR, 1.1)

    # Far tiny arm
    far_shoulder = P(126, -70)
    _draw_tiny_arm(draw, far_shoulder, 128 + pose.far_arm, pose.far_reach, front=False)

    # Neck
    neck_base = P(176, -88)
    neck_top = P(228, -134, pose.neck)
    neck = [
        P(154, -96),
        (neck_top[0] - 18, neck_top[1] - 10),
        (neck_top[0] + 10, neck_top[1] + 12),
        P(178, -38),
    ]
    _poly(draw, neck, GREEN, OUTLINE, 1.2)
    neck_belly = [
        P(174, -78),
        (neck_top[0] - 2, neck_top[1] + 8),
        (neck_top[0] + 18, neck_top[1] + 26),
        P(192, -34),
    ]
    _poly(draw, neck_belly, BELLY, OUTLINE, 0.9)

    head_pivot = (
        neck_top[0] + 28 * math.cos(math.radians(body_angle + pose.neck - 16)),
        neck_top[1] + 28 * math.sin(math.radians(body_angle + pose.neck - 16)),
    )
    head_ang = body_angle + pose.neck + pose.head - 6

    def H(x: float, y: float) -> Point:
        rx, ry = _rot(x, y, head_ang)
        return (head_pivot[0] + rx, head_pivot[1] + ry)

    # Big classic head
    skull = [H(-26, -22), H(-6, -40), H(34, -44), H(54, -30), H(22, -8), H(-18, -4)]
    upper_jaw = [
        H(-4, -26),
        H(40, -38),
        H(104, -34),
        H(154, -20),
        H(188, -3),
        H(176, 7),
        H(126, 4),
        H(58, 0),
        H(10, -6),
    ]
    lower_jaw = [
        H(2, 10),
        H(48, 18),
        H(118, 24 + pose.jaw * 0.20),
        H(172, 18 + pose.jaw * 0.18),
        H(188, 9 + pose.jaw * 0.14),
        H(134, 4 + pose.jaw * 0.18),
        H(62, 2),
        H(8, 4),
    ]
    _poly(draw, skull, GREEN_LIGHT, OUTLINE, 1.2)
    _poly(draw, upper_jaw, GREEN, OUTLINE, 1.3)
    _poly(draw, lower_jaw, GREEN_LIGHT, OUTLINE, 1.1)
    snout_belly = [
        H(24, -2),
        H(90, 2),
        H(164, 8),
        H(172, 14),
        H(114, 16),
        H(48, 12),
        H(12, 8),
    ]
    _poly(draw, snout_belly, BELLY, OUTLINE, 0.8)

    # Mouth interior and teeth
    _poly(
        draw,
        [
            H(18, 3),
            H(62, 8),
            H(132, 14 + pose.jaw * 0.10),
            H(162, 10 + pose.jaw * 0.10),
            H(118, 24 + pose.jaw * 0.11),
            H(56, 18),
        ],
        MOUTH,
        OUTLINE,
        0.6,
    )
    _poly(
        draw,
        [H(70, 16), H(116, 20 + pose.jaw * 0.10), H(92, 30 + pose.jaw * 0.11)],
        TONGUE,
        OUTLINE,
        0.5,
    )
    for tx in [32, 54, 78, 102, 126, 148, 166]:
        p = H(tx, 2)
        _poly(draw, [p, H(tx + 5, 10), H(tx + 10, 2)], TOOTH, OUTLINE, 0.4)
    for tx in [40, 72, 104, 138]:
        p = H(tx, 15 + pose.jaw * 0.09)
        _poly(
            draw,
            [p, H(tx + 5, 7 + pose.jaw * 0.06), H(tx + 10, 15 + pose.jaw * 0.09)],
            TOOTH,
            OUTLINE,
            0.4,
        )

    eye = H(40, -18)
    brow_a = H(28, -24)
    brow_b = H(46, -30)
    _line(draw, [brow_a, brow_b], OUTLINE, 1.1)
    if pose.x_eye:
        _line(draw, [H(34, -22), H(46, -14)], OUTLINE, 1.0)
        _line(draw, [H(34, -14), H(46, -22)], OUTLINE, 1.0)
    elif pose.blink:
        _line(draw, [H(34, -18), H(46, -18)], OUTLINE, 1.0)
    else:
        _ellipse(draw, eye[0], eye[1], 6.0, 5.0, EYE, OUTLINE, 0.7)
        _circle(draw, (eye[0] + 1.5, eye[1] + 0.5), 1.5, PUPIL, PUPIL, 0.1)
    nostril = H(128, -12)
    _line(
        draw,
        [(nostril[0] - _s(2), nostril[1]), (nostril[0] + _s(4), nostril[1] + _s(1))],
        OUTLINE,
        0.8,
    )

    # Near tiny arm
    near_shoulder = P(136, -62)
    _draw_tiny_arm(
        draw, near_shoulder, 124 + pose.near_arm, pose.near_reach, front=True
    )

    # Near hind leg (second and last leg)
    near_hip = P(34, -2)
    near_ankle = _draw_hind_leg(
        draw,
        near_hip,
        92 + pose.near_leg,
        38 + pose.near_knee,
        pose.near_lift,
        scale=1.0,
        front=True,
    )

    # Attack FX
    if anim == "bite" and pose.bite_fx > 0.15:
        cx, cy = H(162, 6)
        box = (_s(cx - 66), _s(cy - 40), _s(cx + 52), _s(cy + 48))
        draw.arc(box, 210, 350, fill=FX, width=_s(4.8 + pose.bite_fx))
    if anim == "roar" and pose.roar > 0.15:
        for i in range(3):
            rad = 28 + i * 22 + pose.roar * 8
            cx, cy = H(176, 10)
            box = (_s(cx - rad), _s(cy - rad * 0.66), _s(cx + rad), _s(cy + rad * 0.66))
            draw.arc(box, 300, 30, fill=ROAR, width=_s(2.0))
    if anim == "tail_swipe" and pose.swipe > 0.12:
        tx, ty = tail3
        box = (_s(tx - 76), _s(ty - 56), _s(tx + 46), _s(ty + 52))
        draw.arc(box, 100, 252, fill=FX, width=_s(4.2))
    if anim in {"charge", "stomp"} and pose.dust > 0.1:
        for i, dx in enumerate([-34, -10, 16, 42]):
            base = (near_ankle[0] + dx, near_ankle[1] + 10 + (i % 2) * 2)
            scale = pose.dust * (1.0 - i * 0.08)
            _poly(
                draw,
                [
                    (base[0] - 9 * scale, base[1]),
                    (base[0], base[1] - 12 * scale),
                    (base[0] + 10 * scale, base[1] - 1 * scale),
                    (base[0] + 3 * scale, base[1] + 7 * scale),
                ],
                DUST,
                (95, 78, 54, 110),
                0.4,
            )
    if anim == "stomp" and pose.dust > 0.1:
        sx, sy = near_ankle
        box = (_s(sx - 78), _s(sy - 14), _s(sx + 88), _s(sy + 32))
        draw.arc(box, 190, 350, fill=FX, width=_s(3.8))

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
        description="Render the standalone big classic T-rex enemy spritesheet."
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
