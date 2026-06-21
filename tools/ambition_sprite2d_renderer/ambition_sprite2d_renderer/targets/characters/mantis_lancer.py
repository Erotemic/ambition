from __future__ import annotations

"""Standalone generator for a side-profile mantis lancer enemy.

Designed specifically for side-scrolling readability. The silhouette is a
forward-facing insectoid lancer with exaggerated blade arms and hind legs, and
it includes multiple attack animations that suggest distinct gameplay patterns:
long stab, sweeping slash, and pouncing leap.
"""

import argparse
import math
from dataclasses import dataclass
from pathlib import Path
from typing import List, Sequence, Tuple

from PIL import Image, ImageDraw


RGBA = Tuple[int, int, int, int]
Point = Tuple[float, float]

TARGET_BASENAME = "mantis_lancer"
FRAME_SIZE = (240, 224)
WORK_FRAME_SIZE = (480, 448)
SUPER = 4
ROWS: List[Tuple[str, int, int]] = [
    ("idle", 6, 130),
    ("walk", 8, 95),
    ("stab", 7, 80),
    ("slash", 7, 80),
    ("pounce", 8, 85),
    ("hurt", 4, 90),
    ("death", 8, 110),
]

ACTOR_METADATA = {
    "actor": {
        "character_id": "npc_mantis_lancer",
        "display_name": "Mantis Lancer",
    },
    "body": {
        "body_plan": "InsectoidBiped",
        "body_kind": "Standard",
        "mass_class": "Medium",
        "locomotion_hint": "Walk",
        "traits": ["enemy", "insectoid", "blade_arms", "lancer"],
    },
    "capabilities": {
        "traversal": {
            "walk": True,
            "jump": {"height_px": None, "distance_px": None, "source": "mantis_pounce_animation"},
            "climb": True,
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
    "brain": {"default_preset": "melee_brute_striker"},
    "actions": {"default_preset": "brute_lunge"},
    "animation_bindings": {
        "default": {"animation": "idle", "events": []},
        "locomotion.walk": {"animation": "walk", "events": []},
        "action.melee.primary": {
            "animation": "stab",
            "events": [
                {"t": 0.36, "event": "hitbox_active_start", "source": "mantis_lancer.stab"},
                {"t": 0.60, "event": "hitbox_active_end", "source": "mantis_lancer.stab"},
            ],
        },
        "action.melee.sweep": {
            "animation": "slash",
            "events": [
                {"t": 0.30, "event": "hitbox_active_start", "source": "mantis_lancer.slash"},
                {"t": 0.62, "event": "hitbox_active_end", "source": "mantis_lancer.slash"},
            ],
        },
        "action.special.pounce": {"animation": "pounce", "events": [{"t": 0.40, "event": "leap_commit", "source": "mantis_lancer.pounce"}]},
        "damage.hit": {"animation": "hurt", "events": []},
        "lifecycle.death": {"animation": "death", "events": []},
    },
    "sockets": {
        "head": {"source": "mantis_lancer.geometry", "point": {"x": 144.0, "y": 58.0}},
        "thorax": {"source": "mantis_lancer.geometry", "point": {"x": 112.0, "y": 96.0}},
        "blade_l": {"source": "mantis_lancer.geometry", "point": {"x": 95.0, "y": 102.0}},
        "blade_r": {"source": "mantis_lancer.geometry", "point": {"x": 172.0, "y": 100.0}},
        "blade_tip": {"source": "mantis_lancer.geometry", "point": {"x": 210.0, "y": 88.0}},
        "pounce_origin": {"source": "mantis_lancer.geometry", "point": {"x": 124.0, "y": 170.0}},
    },
    "tags": ["enemy", "insectoid", "lancer"],
}

OUTLINE = (16, 18, 18, 255)
CHITIN_DARK = (38, 56, 34, 255)
CHITIN = (72, 108, 62, 255)
CHITIN_LIGHT = (122, 164, 94, 255)
ACCENT = (196, 80, 44, 255)
ACCENT_LIGHT = (242, 156, 88, 255)
BONE = (218, 224, 194, 255)
SHADOW = (0, 0, 0, 42)
EYE = (255, 224, 118, 255)
EYE_HOT = (255, 250, 210, 255)


@dataclass
class Pose:
    root_x: float = 0.0
    root_y: float = 0.0
    bob: float = 0.0
    lean: float = 0.0
    thorax_tilt: float = 0.0
    head_tilt: float = 0.0
    abdomen_lift: float = 0.0
    crest_sway: float = 0.0
    front_blade: float = 0.0
    back_blade: float = 0.0
    front_leg: float = 0.0
    back_leg: float = 0.0
    front_foot_lift: float = 0.0
    back_foot_lift: float = 0.0
    stab_extension: float = 0.0
    crouch: float = 0.0
    airborne: float = 0.0
    slash_arc: float = 0.0
    mouth_open: float = 0.0
    blink: bool = False
    x_eyes: bool = False
    dead: bool = False

    def __init__(self, anim: str, frame_idx: int, nframes: int):
        t = frame_idx / max(1, nframes - 1)
        cyc = math.tau * frame_idx / max(1, nframes)
        s = math.sin(cyc)
        c = math.cos(cyc)

        self.root_x = 0.0
        self.root_y = 0.0
        self.bob = 0.0
        self.lean = 0.0
        self.thorax_tilt = 0.0
        self.head_tilt = 0.0
        self.abdomen_lift = 0.0
        self.crest_sway = 0.0
        self.front_blade = 0.0
        self.back_blade = 0.0
        self.front_leg = 0.0
        self.back_leg = 0.0
        self.front_foot_lift = 0.0
        self.back_foot_lift = 0.0
        self.stab_extension = 0.0
        self.crouch = 0.0
        self.airborne = 0.0
        self.slash_arc = 0.0
        self.mouth_open = 0.0
        self.blink = False
        self.x_eyes = False
        self.dead = False

        if anim == "idle":
            self.bob = s * 1.5
            self.lean = s * 1.4
            self.thorax_tilt = s * 1.2
            self.head_tilt = -s * 1.6
            self.abdomen_lift = abs(s) * 1.8
            self.front_blade = 12.0 + s * 4.0
            self.back_blade = -18.0 - s * 4.0
            self.front_leg = c * 1.2
            self.back_leg = -c * 1.2
            self.crest_sway = s * 6.0
            self.blink = frame_idx == nframes - 2
        elif anim == "walk":
            self.root_x = s * 2.4
            self.bob = abs(s) * 3.0 - 0.8
            self.lean = s * 3.2
            self.thorax_tilt = s * 2.2
            self.head_tilt = -s * 2.0
            self.abdomen_lift = abs(s) * 3.0
            self.front_leg = 22.0 * s
            self.back_leg = -20.0 * s
            self.front_blade = -8.0 * s + 10.0
            self.back_blade = 10.0 * s - 18.0
            self.front_foot_lift = max(0.0, s) * 9.0
            self.back_foot_lift = max(0.0, -s) * 9.0
            self.crest_sway = -s * 10.0
        elif anim == "stab":
            tt = _ease(t)
            hit = math.sin(tt * math.pi)
            self.root_x = _lerp(-8.0, 12.0, tt)
            self.bob = -hit * 4.0
            self.lean = _lerp(-12.0, 16.0, tt)
            self.thorax_tilt = _lerp(-8.0, 14.0, tt)
            self.head_tilt = _lerp(-10.0, 6.0, tt)
            self.abdomen_lift = _lerp(4.0, -2.0, tt)
            self.front_blade = _lerp(-126.0, 16.0, tt)
            self.back_blade = _lerp(-26.0, 18.0, tt)
            self.front_leg = -8.0 - hit * 4.0
            self.back_leg = 14.0 + hit * 3.0
            self.stab_extension = _lerp(0.0, 44.0, tt)
            self.crouch = _lerp(6.0, 0.0, tt)
            self.mouth_open = 0.22 * hit
            self.crest_sway = _lerp(14.0, -10.0, tt)
        elif anim == "slash":
            tt = _ease(t)
            hit = math.sin(tt * math.pi)
            self.root_x = _lerp(-4.0, 6.0, tt)
            self.bob = -hit * 3.0
            self.lean = _lerp(10.0, -18.0, tt)
            self.thorax_tilt = _lerp(8.0, -22.0, tt)
            self.head_tilt = _lerp(8.0, -14.0, tt)
            self.abdomen_lift = 2.0 + hit * 2.0
            self.front_blade = _lerp(54.0, -108.0, tt)
            self.back_blade = _lerp(18.0, -32.0, tt)
            self.front_leg = 10.0 - hit * 6.0
            self.back_leg = -10.0 + hit * 4.0
            self.slash_arc = hit
            self.crouch = 2.0
            self.mouth_open = 0.16 * hit
            self.crest_sway = _lerp(-12.0, 16.0, tt)
        elif anim == "pounce":
            tt = _ease(t)
            launch = math.sin(tt * math.pi)
            self.root_x = _lerp(-12.0, 20.0, tt)
            self.root_y = -launch * 20.0
            self.airborne = launch
            self.bob = -launch * 6.0
            self.lean = _lerp(-18.0, 22.0, tt)
            self.thorax_tilt = _lerp(-12.0, 18.0, tt)
            self.head_tilt = _lerp(-10.0, 10.0, tt)
            self.abdomen_lift = _lerp(8.0, -4.0, tt)
            self.front_blade = _lerp(-72.0, 30.0, tt)
            self.back_blade = _lerp(-48.0, 26.0, tt)
            self.front_leg = _lerp(-26.0, 22.0, tt)
            self.back_leg = _lerp(-18.0, 28.0, tt)
            self.front_foot_lift = launch * 20.0
            self.back_foot_lift = launch * 18.0
            self.stab_extension = launch * 16.0
            self.crouch = _lerp(10.0, 0.0, tt)
            self.crest_sway = _lerp(18.0, -18.0, tt)
            self.mouth_open = 0.2 * launch
        elif anim == "hurt":
            hit = math.sin(t * math.pi)
            shake = math.sin(t * math.pi * 4.0) * (1.0 - t)
            self.root_x = shake * 4.0
            self.bob = -hit * 2.5
            self.lean = -18.0 * hit
            self.thorax_tilt = -12.0 * hit
            self.head_tilt = 18.0 * hit
            self.front_blade = 22.0 * hit
            self.back_blade = 18.0 * hit
            self.front_leg = 10.0 * hit
            self.back_leg = -8.0 * hit
            self.abdomen_lift = 8.0 * hit
            self.mouth_open = 0.24 * hit
        elif anim == "death":
            tt = _ease(t)
            self.root_x = tt * 16.0
            self.root_y = tt * 6.0
            self.bob = -tt * 4.0
            self.lean = -88.0 * tt
            self.thorax_tilt = -36.0 * tt
            self.head_tilt = 28.0 * tt
            self.front_blade = _lerp(10.0, 72.0, tt)
            self.back_blade = _lerp(-16.0, -88.0, tt)
            self.front_leg = _lerp(0.0, 30.0, tt)
            self.back_leg = _lerp(0.0, -28.0, tt)
            self.front_foot_lift = tt * 8.0
            self.back_foot_lift = tt * 10.0
            self.abdomen_lift = _lerp(0.0, 12.0, tt)
            self.crouch = tt * 6.0
            self.crest_sway = tt * 12.0
            self.mouth_open = 0.22 * tt
            self.x_eyes = tt > 0.55
            self.dead = tt > 0.7


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


def _rot_local(x: float, y: float, deg: float) -> Point:
    rad = math.radians(deg)
    c = math.cos(rad)
    s = math.sin(rad)
    return (x * c - y * s, x * s + y * c)


def _poly(draw: ImageDraw.ImageDraw, pts: Sequence[Point], fill: RGBA, outline: RGBA = OUTLINE, width: float = 1.0) -> None:
    ipts = [_pt(p) for p in pts]
    draw.polygon(ipts, fill=fill)
    if outline and width > 0:
        draw.line(ipts + [ipts[0]], fill=outline, width=max(1, _s(width)), joint="curve")


def _line(draw: ImageDraw.ImageDraw, pts: Sequence[Point], fill: RGBA, width: float = 1.0) -> None:
    draw.line([_pt(p) for p in pts], fill=fill, width=max(1, _s(width)), joint="curve")


def _ellipse(draw: ImageDraw.ImageDraw, cx: float, cy: float, rx: float, ry: float, fill: RGBA, outline: RGBA = OUTLINE, width: float = 1.0) -> None:
    draw.ellipse(_box(cx, cy, rx, ry), fill=fill, outline=outline, width=max(1, _s(width)))


def _circle(draw: ImageDraw.ImageDraw, c: Point, r: float, fill: RGBA, outline: RGBA = OUTLINE, width: float = 1.0) -> None:
    _ellipse(draw, c[0], c[1], r, r, fill, outline, width)


def _downsample(img: Image.Image) -> Image.Image:
    return img.resize(FRAME_SIZE, Image.Resampling.LANCZOS)


class MantisLancerRenderer:
    def render_frame(self, anim: str, frame_idx: int, nframes: int) -> Image.Image:
        img = Image.new("RGBA", (WORK_FRAME_SIZE[0] * SUPER, WORK_FRAME_SIZE[1] * SUPER), (0, 0, 0, 0))
        draw = ImageDraw.Draw(img, "RGBA")
        pose = Pose(anim, frame_idx, nframes)

        root = (
            WORK_FRAME_SIZE[0] * 0.40 + pose.root_x,
            WORK_FRAME_SIZE[1] * 0.78 + pose.root_y + pose.bob,
        )
        global_tilt = pose.lean

        def P(x: float, y: float) -> Point:
            rx, ry = _rot_local(x, y, global_tilt)
            return (root[0] + rx, root[1] + ry)

        # No baked ground drop shadow; the scene renderer owns contact shadows.
        self._draw_back_leg(draw, P, pose)
        self._draw_abdomen(draw, P, pose)
        self._draw_body(draw, P, pose)
        self._draw_back_blade(draw, P, pose)
        self._draw_head(draw, P, pose)
        self._draw_front_leg(draw, P, pose)
        self._draw_front_blade(draw, P, pose)
        if anim == "slash" and pose.slash_arc > 0.2:
            self._draw_slash_fx(draw, P, pose)
        if anim == "pounce" and pose.airborne > 0.15:
            self._draw_pounce_fx(draw, P, pose)
        return _downsample(img)

    def _draw_shadow(self, draw: ImageDraw.ImageDraw, P, pose: Pose) -> None:
        c = P(-2, 14)
        _ellipse(draw, c[0], c[1], 54 + pose.airborne * 6, 11 - pose.bob * 0.08, SHADOW, outline=(0, 0, 0, 0), width=0)

    def _draw_abdomen(self, draw: ImageDraw.ImageDraw, P, pose: Pose) -> None:
        belly = [
            P(-78, -86 + pose.abdomen_lift), P(-104, -74 + pose.abdomen_lift), P(-114, -46 + pose.abdomen_lift),
            P(-100, -20), P(-72, -8), P(-42, -20), P(-34, -50), P(-48, -76 + pose.abdomen_lift)
        ]
        _poly(draw, belly, CHITIN_DARK, OUTLINE, 1.5)
        highlight = [P(-90, -66 + pose.abdomen_lift), P(-106, -58 + pose.abdomen_lift), P(-102, -40 + pose.abdomen_lift), P(-82, -34)]
        _poly(draw, highlight, CHITIN, OUTLINE, 0.8)
        for off in (-94, -82, -70, -58):
            _line(draw, [P(off, -68 + pose.abdomen_lift * 0.6), P(off + 4, -22)], CHITIN_LIGHT, 0.7)
        stinger = [P(-112, -52 + pose.abdomen_lift), P(-136, -60 + pose.abdomen_lift), P(-126, -40 + pose.abdomen_lift)]
        _poly(draw, stinger, BONE, OUTLINE, 0.8)

    def _draw_body(self, draw: ImageDraw.ImageDraw, P, pose: Pose) -> None:
        thorax = [
            P(-36, -104 - pose.crouch), P(18, -128 - pose.crouch), P(52, -106 - pose.crouch),
            P(58, -72), P(30, -42), P(-12, -38), P(-44, -62)
        ]
        _poly(draw, thorax, CHITIN, OUTLINE, 1.7)
        plate = [P(-22, -102 - pose.crouch), P(14, -118 - pose.crouch), P(40, -102 - pose.crouch), P(36, -72), P(8, -54), P(-20, -68)]
        _poly(draw, plate, CHITIN_LIGHT, OUTLINE, 1.1)
        _line(draw, [P(-8, -106 - pose.crouch), P(6, -54)], CHITIN_DARK, 1.0)
        flank = [P(-18, -72), P(8, -78), P(22, -58), P(4, -40), P(-18, -44)]
        _poly(draw, flank, ACCENT, OUTLINE, 0.9)

    def _draw_head(self, draw: ImageDraw.ImageDraw, P, pose: Pose) -> None:
        hx, hy = P(62 + pose.stab_extension * 0.35, -108 - pose.crouch + pose.head_tilt * 0.18)
        head = [
            (hx - 18, hy - 12), (hx + 8, hy - 20), (hx + 34, hy - 10), (hx + 42, hy + 4),
            (hx + 26, hy + 16), (hx - 2, hy + 18), (hx - 18, hy + 8)
        ]
        _poly(draw, head, CHITIN_LIGHT, OUTLINE, 1.2)
        crest = [
            (hx - 6, hy - 18), (hx + 16, hy - 36 + pose.crest_sway * 0.2), (hx + 34, hy - 22),
            (hx + 14, hy - 8)
        ]
        _poly(draw, crest, ACCENT, OUTLINE, 0.9)
        if pose.x_eyes:
            _line(draw, [(hx + 5, hy - 3), (hx + 15, hy + 5)], OUTLINE, 1.0)
            _line(draw, [(hx + 5, hy + 5), (hx + 15, hy - 3)], OUTLINE, 1.0)
        elif pose.blink:
            _line(draw, [(hx + 4, hy + 1), (hx + 16, hy + 1)], EYE_HOT, 1.0)
        else:
            _ellipse(draw, hx + 10, hy + 0, 6, 4, EYE, EYE_HOT, 0.8)
            _line(draw, [(hx + 2, hy - 6), (hx + 16, hy - 2)], OUTLINE, 0.8)
        upper_jaw = [(hx + 16, hy + 6), (hx + 38, hy + 8), (hx + 48, hy + 2), (hx + 34, hy + 14)]
        lower_jaw = [(hx + 16, hy + 10), (hx + 34, hy + 18 + pose.mouth_open * 10), (hx + 44, hy + 14), (hx + 28, hy + 20 + pose.mouth_open * 8)]
        _poly(draw, upper_jaw, BONE, OUTLINE, 0.8)
        _poly(draw, lower_jaw, BONE, OUTLINE, 0.8)
        for dy in (-6, 4):
            _line(draw, [(hx + 10, hy + dy), (hx + 2, hy + dy - 12)], BONE, 0.7)

    def _draw_leg(self, draw: ImageDraw.ImageDraw, hip: Point, knee: Point, ankle: Point, toe: Point, front: bool) -> None:
        base = CHITIN if front else CHITIN_DARK
        _line(draw, [hip, knee], base, 7.2 if front else 6.8)
        _line(draw, [knee, ankle], base, 6.4 if front else 6.0)
        _line(draw, [hip, knee, ankle], OUTLINE, 2.0)
        _ellipse(draw, knee[0], knee[1], 6.5, 8.0, CHITIN_LIGHT, OUTLINE, 1.0)
        shin_plate = [(knee[0] - 5, knee[1]), (ankle[0] - 3, ankle[1] - 6), (ankle[0] + 6, ankle[1] + 2), (knee[0] + 4, knee[1] + 4)]
        _poly(draw, shin_plate, CHITIN_LIGHT if front else CHITIN, OUTLINE, 0.8)
        _line(draw, [ankle, toe], BONE, 2.4)
        _line(draw, [ankle, toe], OUTLINE, 1.0)
        claw2 = (toe[0] + (7 if front else 5), toe[1] + 3)
        _line(draw, [ankle, claw2], BONE, 2.0)
        _line(draw, [ankle, claw2], OUTLINE, 0.9)

    def _draw_back_leg(self, draw: ImageDraw.ImageDraw, P, pose: Pose) -> None:
        hip = P(-40, -34)
        knee = P(-66 + pose.back_leg * 0.18, -2 - pose.crouch * 0.15)
        ankle = P(-82 + pose.back_leg * 0.14, 11 - pose.back_foot_lift)
        toe = P(-94 + pose.back_leg * 0.12, 16 - pose.back_foot_lift)
        self._draw_leg(draw, hip, knee, ankle, toe, front=False)

    def _draw_front_leg(self, draw: ImageDraw.ImageDraw, P, pose: Pose) -> None:
        hip = P(6, -28)
        knee = P(18 + pose.front_leg * 0.16, -4 - pose.crouch * 0.10)
        ankle = P(26 + pose.front_leg * 0.14, 12 - pose.front_foot_lift)
        toe = P(40 + pose.front_leg * 0.12, 16 - pose.front_foot_lift)
        self._draw_leg(draw, hip, knee, ankle, toe, front=True)

    def _draw_blade_arm(self, draw: ImageDraw.ImageDraw, shoulder: Point, elbow: Point, wrist: Point, blade_tip: Point, front: bool) -> None:
        limb = CHITIN_LIGHT if front else CHITIN
        _line(draw, [shoulder, elbow], limb, 7.2 if front else 6.4)
        _line(draw, [elbow, wrist], limb, 6.6 if front else 5.8)
        _line(draw, [shoulder, elbow, wrist], OUTLINE, 2.0)
        _ellipse(draw, elbow[0], elbow[1], 6.5, 8.5, CHITIN_LIGHT, OUTLINE, 1.0)
        blade = [
            wrist,
            (blade_tip[0] - 22, blade_tip[1] - 10),
            blade_tip,
            (blade_tip[0] - 14, blade_tip[1] + 14),
            (wrist[0] - 6, wrist[1] + 6),
        ]
        _poly(draw, blade, BONE, OUTLINE, 1.0)
        inner = [
            (wrist[0] + 2, wrist[1] - 2),
            (blade_tip[0] - 18, blade_tip[1] - 2),
            (blade_tip[0] - 10, blade_tip[1] + 7),
            (wrist[0], wrist[1] + 2),
        ]
        _poly(draw, inner, ACCENT_LIGHT if front else ACCENT, OUTLINE, 0.7)

    def _draw_back_blade(self, draw: ImageDraw.ImageDraw, P, pose: Pose) -> None:
        shoulder = P(2, -102 - pose.crouch * 0.3)
        elbow = P(12 + pose.back_blade * 0.10, -70 + pose.back_blade * 0.16)
        wrist = P(30 + pose.back_blade * 0.18, -48 + pose.back_blade * 0.18)
        tip = P(70 + pose.back_blade * 0.40 + pose.stab_extension * 0.18, -64 + pose.back_blade * 0.08)
        self._draw_blade_arm(draw, shoulder, elbow, wrist, tip, front=False)

    def _draw_front_blade(self, draw: ImageDraw.ImageDraw, P, pose: Pose) -> None:
        shoulder = P(24, -112 - pose.crouch * 0.3)
        elbow = P(36 + pose.front_blade * 0.10 + pose.stab_extension * 0.12, -80 + pose.front_blade * 0.18)
        wrist = P(56 + pose.front_blade * 0.22 + pose.stab_extension * 0.30, -64 + pose.front_blade * 0.16)
        tip = P(112 + pose.front_blade * 0.44 + pose.stab_extension * 0.70, -72 + pose.front_blade * 0.10)
        self._draw_blade_arm(draw, shoulder, elbow, wrist, tip, front=True)

    def _draw_slash_fx(self, draw: ImageDraw.ImageDraw, P, pose: Pose) -> None:
        cx, cy = P(84, -76)
        box = (_s(cx - 92), _s(cy - 72), _s(cx + 92), _s(cy + 72))
        draw.arc(box, 182, 332, fill=(*ACCENT[:3], 136), width=_s(5.0 + pose.slash_arc * 2.0))
        draw.arc(box, 196, 320, fill=(255, 240, 210, 110), width=_s(2.0))

    def _draw_pounce_fx(self, draw: ImageDraw.ImageDraw, P, pose: Pose) -> None:
        c = P(-24, 9)
        _ellipse(draw, c[0], c[1], 18 + pose.airborne * 8, 6 + pose.airborne * 2, (*ACCENT[:3], 80), outline=(0, 0, 0, 0), width=0)
        for dx in (-18, -6, 10):
            shard = [P(-18 + dx, 8), P(-10 + dx, 0), P(-2 + dx, 8)]
            _poly(draw, shard, (*ACCENT_LIGHT[:3], 160), (*ACCENT[:3], 140), 0.5)


def _write_yaml(path: Path) -> None:
    lines = [
        f"target: {TARGET_BASENAME}",
        f"frame_width: {FRAME_SIZE[0]}",
        f"frame_height: {FRAME_SIZE[1]}",
        "rows:",
    ]
    for name, frames, ms in ROWS:
        lines.extend([
            f"  - name: {name}",
            f"    frames: {frames}",
            f"    frame_ms: {ms}",
        ])
    path.write_text("\n".join(lines) + "\n")


def _write_ron(path: Path) -> None:
    row_lines = []
    for name, frames, ms in ROWS:
        row_lines.append(f'        (name: "{name}", frames: {frames}, frame_ms: {ms}),')
    ron = [
        "(",
        f'    target: "{TARGET_BASENAME}",',
        f'    frame_width: {FRAME_SIZE[0]},',
        f'    frame_height: {FRAME_SIZE[1]},',
        "    rows: [",
        *row_lines,
        "    ],",
        ")",
    ]
    path.write_text("\n".join(ron) + "\n")


def _render_sheet(renderer: MantisLancerRenderer, out_dir: Path):
    frame_w, frame_h = FRAME_SIZE
    sheet_w = max(frames for _, frames, _ in ROWS) * frame_w
    sheet_h = len(ROWS) * frame_h
    sheet = Image.new("RGBA", (sheet_w, sheet_h), (0, 0, 0, 0))
    preview = Image.new("RGBA", (sheet_w + 112, sheet_h), (250, 248, 244, 255))
    pdraw = ImageDraw.Draw(preview)
    canonical = None
    for row_idx, (name, nframes, _ms) in enumerate(ROWS):
        pdraw.text((8, row_idx * frame_h + 8), name, fill=(32, 32, 32, 255))
        for frame_idx in range(nframes):
            frame = renderer.render_frame(name, frame_idx, nframes)
            x = frame_idx * frame_w
            y = row_idx * frame_h
            sheet.alpha_composite(frame, (x, y))
            preview.alpha_composite(frame, (x + 112, y))
            if canonical is None and name == "idle" and frame_idx == 0:
                canonical = frame
    if canonical is None:
        canonical = renderer.render_frame(ROWS[0][0], 0, ROWS[0][1])
    spritesheet_path = out_dir / f"{TARGET_BASENAME}.png"
    yaml_path = out_dir / f"{TARGET_BASENAME}.yaml"
    ron_path = out_dir / f"{TARGET_BASENAME}.ron"
    preview_path = out_dir / f"{TARGET_BASENAME}_preview_labeled.png"
    canonical_path = out_dir / f"{TARGET_BASENAME}_canonical.png"
    sheet.save(spritesheet_path)
    preview.save(preview_path)
    canonical.save(canonical_path)
    _write_yaml(yaml_path)
    _write_ron(ron_path)
    return [spritesheet_path, yaml_path, ron_path, preview_path, canonical_path]


def render(out_dir: str | Path, **opts):
    """Render the mantis_lancer spritesheet bundle via the shared
    `tackon_sheet.build_sheet` pipeline (auto-cropped, with the
    runtime-compatible YAML+RON shape). See `bear_mauler.render` for
    the full rationale — same conversion."""
    from ...authoring.tackon_sheet import build_sheet
    out_dir = Path(out_dir)
    out_dir.mkdir(parents=True, exist_ok=True)
    renderer = MantisLancerRenderer()
    outputs = build_sheet(
        target=TARGET_BASENAME,
        rows=ROWS,
        render_fn=renderer.render_frame,
        out_dir=out_dir,
        frame_size=FRAME_SIZE,
        auto_crop=True,
        actor_metadata=ACTOR_METADATA,
    )
    return [
        outputs["spritesheet"], outputs["yaml"], outputs["ron"],
        outputs["actor"], outputs["preview"], outputs["canonical"], outputs["canonical_transparent"],
    ]


def main(argv: List[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="Render a side-profile mantis lancer enemy spritesheet.")
    parser.add_argument("--out-dir", type=Path, default=Path(__file__).resolve().parents[2] / "generated" / TARGET_BASENAME)
    args = parser.parse_args(argv)
    for path in render(args.out_dir):
        print(path)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
