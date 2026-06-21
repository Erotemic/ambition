from __future__ import annotations

"""Standalone generator for a side-profile dinosaur enemy.

This target renders a stylized raptor-like dinosaur enemy built for
side-scrolling gameplay readability. It emphasizes a clear profile silhouette
and multiple attack rows that imply distinct combat patterns:
- bite / lunge
- tail sweep
- pounce
"""

import argparse
import math
from dataclasses import dataclass
from pathlib import Path
from typing import List, Sequence, Tuple

from PIL import Image, ImageDraw

RGBA = Tuple[int, int, int, int]
Point = Tuple[float, float]

TARGET_BASENAME = "raptor_stalker"
FRAME_SIZE = (240, 224)
WORK_FRAME_SIZE = (480, 448)
SUPER = 4
ROWS: List[Tuple[str, int, int]] = [
    ("idle", 6, 130),
    ("walk", 8, 95),
    ("bite", 7, 80),
    ("tail_sweep", 7, 85),
    ("pounce", 8, 85),
    ("hurt", 4, 90),
    ("death", 8, 110),
]

ACTOR_METADATA = {
    "actor": {
        "character_id": "npc_raptor_stalker",
        "display_name": "Raptor Stalker",
    },
    "body": {
        "body_plan": "BeastBiped",
        "body_kind": "Standard",
        "mass_class": "Medium",
        "locomotion_hint": "Run",
        "traits": ["enemy", "beast", "dinosaur", "stalker", "no_hands"],
    },
    "capabilities": {
        "traversal": {
            "walk": True,
            "jump": {"height_px": None, "distance_px": None, "source": "raptor_pounce_animation"},
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
    "brain": {"default_preset": "melee_brute_striker"},
    "actions": {"default_preset": "striker_swipe"},
    "animation_bindings": {
        "default": {"animation": "idle", "events": []},
        "locomotion.run": {"animation": "walk", "events": []},
        "action.melee.primary": {
            "animation": "bite",
            "events": [
                {"t": 0.34, "event": "hitbox_active_start", "source": "raptor_stalker.bite"},
                {"t": 0.56, "event": "hitbox_active_end", "source": "raptor_stalker.bite"},
            ],
        },
        "action.melee.tail_sweep": {
            "animation": "tail_sweep",
            "events": [
                {"t": 0.32, "event": "hitbox_active_start", "source": "raptor_stalker.tail_sweep"},
                {"t": 0.64, "event": "hitbox_active_end", "source": "raptor_stalker.tail_sweep"},
            ],
        },
        "action.special.pounce": {"animation": "pounce", "events": [{"t": 0.42, "event": "leap_commit", "source": "raptor_stalker.pounce"}]},
        "damage.hit": {"animation": "hurt", "events": []},
        "lifecycle.death": {"animation": "death", "events": []},
    },
    "sockets": {
        "head": {"source": "raptor_stalker.geometry", "point": {"x": 166.0, "y": 60.0}},
        "mouth": {"source": "raptor_stalker.geometry", "point": {"x": 194.0, "y": 76.0}},
        "tail_base": {"source": "raptor_stalker.geometry", "point": {"x": 72.0, "y": 118.0}},
        "tail_tip": {"source": "raptor_stalker.geometry", "point": {"x": 32.0, "y": 106.0}},
        "foreclaw": {"source": "raptor_stalker.geometry", "point": {"x": 164.0, "y": 118.0}},
        "pounce_origin": {"source": "raptor_stalker.geometry", "point": {"x": 116.0, "y": 172.0}},
    },
    "tags": ["enemy", "beast", "dinosaur"],
}

OUTLINE = (18, 22, 18, 255)
SCALE_DARK = (44, 74, 50, 255)
SCALE = (84, 138, 90, 255)
SCALE_LIGHT = (132, 190, 126, 255)
BELLY = (224, 214, 166, 255)
BELLY_SHADOW = (170, 156, 120, 255)
ACCENT = (204, 92, 54, 255)
ACCENT_LIGHT = (248, 172, 112, 255)
CLAW = (242, 236, 218, 255)
EYE = (252, 232, 116, 255)
EYE_HOT = (255, 254, 232, 255)
SHADOW = (0, 0, 0, 42)
DUST = (214, 166, 118, 120)


@dataclass
class Pose:
    root_x: float = 0.0
    root_y: float = 0.0
    bob: float = 0.0
    lean: float = 0.0
    body_tilt: float = 0.0
    head_tilt: float = 0.0
    tail_sway: float = 0.0
    tail_lift: float = 0.0
    neck_extend: float = 0.0
    jaw_open: float = 0.0
    crest_sway: float = 0.0
    front_leg: float = 0.0
    back_leg: float = 0.0
    front_foot_lift: float = 0.0
    back_foot_lift: float = 0.0
    front_arm: float = 0.0
    back_arm: float = 0.0
    crouch: float = 0.0
    pounce: float = 0.0
    bite_lunge: float = 0.0
    sweep_arc: float = 0.0
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
        self.body_tilt = 0.0
        self.head_tilt = 0.0
        self.tail_sway = 0.0
        self.tail_lift = 0.0
        self.neck_extend = 0.0
        self.jaw_open = 0.0
        self.crest_sway = 0.0
        self.front_leg = 0.0
        self.back_leg = 0.0
        self.front_foot_lift = 0.0
        self.back_foot_lift = 0.0
        self.front_arm = 0.0
        self.back_arm = 0.0
        self.crouch = 0.0
        self.pounce = 0.0
        self.bite_lunge = 0.0
        self.sweep_arc = 0.0
        self.blink = False
        self.x_eyes = False
        self.dead = False

        if anim == "idle":
            self.bob = s * 1.4
            self.lean = s * 1.2
            self.body_tilt = s * 1.2
            self.head_tilt = -s * 1.8
            self.tail_sway = -s * 10.0
            self.tail_lift = abs(s) * 3.0
            self.front_arm = 8.0 + s * 4.0
            self.back_arm = -10.0 - s * 4.0
            self.front_leg = c * 1.0
            self.back_leg = -c * 1.0
            self.crest_sway = s * 5.0
            self.blink = frame_idx == nframes - 2
        elif anim == "walk":
            self.root_x = s * 2.4
            self.bob = abs(s) * 2.8 - 0.8
            self.lean = s * 2.6
            self.body_tilt = s * 1.8
            self.head_tilt = -s * 1.6
            self.tail_sway = -s * 14.0
            self.tail_lift = abs(s) * 4.0
            self.front_leg = 20.0 * s
            self.back_leg = -20.0 * s
            self.front_foot_lift = max(0.0, s) * 10.0
            self.back_foot_lift = max(0.0, -s) * 10.0
            self.front_arm = -10.0 * s + 6.0
            self.back_arm = 10.0 * s - 10.0
            self.crest_sway = -s * 8.0
        elif anim == "bite":
            tt = _ease(t)
            hit = math.sin(tt * math.pi)
            self.root_x = _lerp(-8.0, 12.0, tt)
            self.bob = -hit * 4.0
            self.lean = _lerp(-10.0, 18.0, tt)
            self.body_tilt = _lerp(-6.0, 14.0, tt)
            self.head_tilt = _lerp(-8.0, 10.0, tt)
            self.neck_extend = _lerp(0.0, 34.0, tt)
            self.bite_lunge = _lerp(0.0, 32.0, tt)
            self.jaw_open = hit * 0.32
            self.tail_sway = _lerp(12.0, -14.0, tt)
            self.tail_lift = _lerp(6.0, -2.0, tt)
            self.front_leg = -10.0 - hit * 4.0
            self.back_leg = 14.0 + hit * 4.0
            self.front_arm = _lerp(-28.0, 18.0, tt)
            self.back_arm = _lerp(-12.0, 12.0, tt)
            self.crouch = _lerp(8.0, 0.0, tt)
            self.crest_sway = _lerp(10.0, -8.0, tt)
        elif anim == "tail_sweep":
            tt = _ease(t)
            hit = math.sin(tt * math.pi)
            self.root_x = _lerp(-4.0, 6.0, tt)
            self.bob = -hit * 2.8
            self.lean = _lerp(16.0, -20.0, tt)
            self.body_tilt = _lerp(14.0, -16.0, tt)
            self.head_tilt = _lerp(10.0, -12.0, tt)
            self.tail_sway = _lerp(-42.0, 54.0, tt)
            self.tail_lift = 4.0 + hit * 4.0
            self.front_leg = 8.0 - hit * 4.0
            self.back_leg = -8.0 + hit * 4.0
            self.front_arm = _lerp(18.0, -10.0, tt)
            self.back_arm = _lerp(10.0, -18.0, tt)
            self.sweep_arc = hit
            self.crest_sway = _lerp(-8.0, 12.0, tt)
            self.jaw_open = hit * 0.14
        elif anim == "pounce":
            tt = _ease(t)
            launch = math.sin(tt * math.pi)
            self.root_x = _lerp(-12.0, 22.0, tt)
            self.root_y = -launch * 20.0
            self.pounce = launch
            self.bob = -launch * 5.0
            self.lean = _lerp(-18.0, 22.0, tt)
            self.body_tilt = _lerp(-10.0, 16.0, tt)
            self.head_tilt = _lerp(-8.0, 12.0, tt)
            self.neck_extend = launch * 14.0
            self.jaw_open = launch * 0.16
            self.tail_sway = _lerp(18.0, -24.0, tt)
            self.tail_lift = _lerp(10.0, -4.0, tt)
            self.front_leg = _lerp(-24.0, 28.0, tt)
            self.back_leg = _lerp(-18.0, 24.0, tt)
            self.front_foot_lift = launch * 20.0
            self.back_foot_lift = launch * 18.0
            self.front_arm = _lerp(-16.0, 22.0, tt)
            self.back_arm = _lerp(-20.0, 18.0, tt)
            self.crouch = _lerp(12.0, 0.0, tt)
            self.crest_sway = _lerp(14.0, -14.0, tt)
        elif anim == "hurt":
            hit = math.sin(t * math.pi)
            shake = math.sin(t * math.pi * 4.0) * (1.0 - t)
            self.root_x = shake * 4.0
            self.bob = -hit * 2.2
            self.lean = -16.0 * hit
            self.body_tilt = -10.0 * hit
            self.head_tilt = 18.0 * hit
            self.jaw_open = 0.24 * hit
            self.tail_sway = 12.0 * hit
            self.tail_lift = 6.0 * hit
            self.front_leg = 8.0 * hit
            self.back_leg = -6.0 * hit
            self.front_arm = 12.0 * hit
            self.back_arm = 10.0 * hit
        elif anim == "death":
            tt = _ease(t)
            self.root_x = tt * 16.0
            self.root_y = tt * 6.0
            self.bob = -tt * 4.0
            self.lean = -86.0 * tt
            self.body_tilt = -32.0 * tt
            self.head_tilt = 26.0 * tt
            self.neck_extend = tt * 6.0
            self.jaw_open = 0.28 * tt
            self.tail_sway = -40.0 * tt
            self.tail_lift = 10.0 * tt
            self.front_leg = _lerp(0.0, 28.0, tt)
            self.back_leg = _lerp(0.0, -24.0, tt)
            self.front_foot_lift = tt * 10.0
            self.back_foot_lift = tt * 8.0
            self.front_arm = _lerp(8.0, 44.0, tt)
            self.back_arm = _lerp(-10.0, -40.0, tt)
            self.crest_sway = tt * 10.0
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


class RaptorStalkerRenderer:
    def render_frame(self, anim: str, frame_idx: int, nframes: int) -> Image.Image:
        img = Image.new("RGBA", (WORK_FRAME_SIZE[0] * SUPER, WORK_FRAME_SIZE[1] * SUPER), (0, 0, 0, 0))
        draw = ImageDraw.Draw(img, "RGBA")
        pose = Pose(anim, frame_idx, nframes)

        root = (
            WORK_FRAME_SIZE[0] * 0.39 + pose.root_x,
            WORK_FRAME_SIZE[1] * 0.79 + pose.root_y + pose.bob,
        )
        global_tilt = pose.lean

        def P(x: float, y: float) -> Point:
            rx, ry = _rot_local(x, y, global_tilt)
            return (root[0] + rx, root[1] + ry)

        # No baked ground drop shadow; the scene renderer owns contact shadows.
        self._draw_tail(draw, P, pose)
        self._draw_back_leg(draw, P, pose)
        self._draw_body(draw, P, pose)
        self._draw_back_arm(draw, P, pose)
        self._draw_head(draw, P, pose)
        self._draw_front_leg(draw, P, pose)
        self._draw_front_arm(draw, P, pose)
        if anim == "tail_sweep" and pose.sweep_arc > 0.2:
            self._draw_tail_fx(draw, P, pose)
        if anim == "pounce" and pose.pounce > 0.16:
            self._draw_pounce_fx(draw, P, pose)
        if anim == "bite" and pose.bite_lunge > 10:
            self._draw_bite_fx(draw, P, pose)
        return _downsample(img)

    def _draw_shadow(self, draw: ImageDraw.ImageDraw, P, pose: Pose) -> None:
        c = P(-6, 14)
        _ellipse(draw, c[0], c[1], 58 + pose.pounce * 8, 11 - pose.bob * 0.08, SHADOW, outline=(0, 0, 0, 0), width=0)

    def _draw_tail(self, draw: ImageDraw.ImageDraw, P, pose: Pose) -> None:
        a = P(-34, -74 + pose.tail_lift * 0.4)
        b = P(-84, -82 + pose.tail_lift * 0.9 + pose.tail_sway * 0.25)
        c = P(-126, -72 + pose.tail_lift * 1.1 + pose.tail_sway * 0.38)
        d = P(-154, -54 + pose.tail_lift * 0.8 + pose.tail_sway * 0.46)
        _line(draw, [a, b], SCALE_DARK, 12.0)
        _line(draw, [b, c], SCALE_DARK, 9.5)
        _line(draw, [c, d], SCALE_DARK, 7.5)
        _line(draw, [a, b, c, d], OUTLINE, 2.2)
        blade = [
            (d[0] - 6, d[1] - 6),
            (d[0] + 24, d[1] - 2),
            (d[0] + 8, d[1] + 8),
        ]
        _poly(draw, blade, ACCENT_LIGHT, OUTLINE, 0.8)
        fin1 = [P(-92, -84 + pose.tail_lift * 0.7), P(-104, -102 + pose.tail_lift * 0.8), P(-82, -92 + pose.tail_lift * 0.7)]
        fin2 = [P(-70, -78 + pose.tail_lift * 0.4), P(-82, -96 + pose.tail_lift * 0.5), P(-60, -84 + pose.tail_lift * 0.4)]
        _poly(draw, fin1, ACCENT, OUTLINE, 0.7)
        _poly(draw, fin2, ACCENT, OUTLINE, 0.7)

    def _draw_body(self, draw: ImageDraw.ImageDraw, P, pose: Pose) -> None:
        torso = [
            P(-42, -94 - pose.crouch), P(2, -122 - pose.crouch), P(48, -110 - pose.crouch),
            P(68, -80), P(50, -44), P(8, -30), P(-34, -42), P(-54, -68)
        ]
        _poly(draw, torso, SCALE, OUTLINE, 1.6)
        back_plate = [P(-28, -92 - pose.crouch), P(8, -110 - pose.crouch), P(36, -98 - pose.crouch), P(28, -70), P(-4, -60), P(-24, -72)]
        _poly(draw, back_plate, SCALE_LIGHT, OUTLINE, 1.0)
        belly = [P(-18, -74), P(20, -80), P(46, -68), P(34, -42), P(4, -34), P(-22, -46)]
        _poly(draw, belly, BELLY, OUTLINE, 0.9)
        chest = [P(10, -86), P(36, -88), P(52, -70), P(36, -54), P(12, -58)]
        _poly(draw, chest, BELLY_SHADOW, OUTLINE, 0.8)
        stripe1 = [P(-8, -96 - pose.crouch * 0.4), P(2, -108 - pose.crouch * 0.4), P(14, -90), P(4, -84)]
        stripe2 = [P(18, -100 - pose.crouch * 0.4), P(30, -110 - pose.crouch * 0.4), P(40, -92), P(28, -86)]
        _poly(draw, stripe1, ACCENT, OUTLINE, 0.6)
        _poly(draw, stripe2, ACCENT, OUTLINE, 0.6)
        _line(draw, [P(-16, -88), P(4, -38)], SCALE_DARK, 1.0)

    def _draw_head(self, draw: ImageDraw.ImageDraw, P, pose: Pose) -> None:
        hx, hy = P(64 + pose.neck_extend + pose.bite_lunge * 0.35, -98 - pose.crouch * 0.4 + pose.head_tilt * 0.15)
        skull = [
            (hx - 28, hy - 14), (hx - 8, hy - 24), (hx + 30, hy - 20), (hx + 60, hy - 8),
            (hx + 70, hy + 2), (hx + 54, hy + 10), (hx + 18, hy + 14), (hx - 18, hy + 10),
            (hx - 32, hy - 2),
        ]
        _poly(draw, skull, SCALE_LIGHT, OUTLINE, 1.2)
        crest = [
            (hx - 4, hy - 22), (hx + 14, hy - 36 + pose.crest_sway * 0.15), (hx + 30, hy - 20), (hx + 10, hy - 10)
        ]
        _poly(draw, crest, ACCENT, OUTLINE, 0.8)
        neck = [P(32, -100 - pose.crouch * 0.2), P(54 + pose.neck_extend * 0.35, -112 - pose.crouch * 0.3), P(66 + pose.neck_extend * 0.2, -90), P(40, -76)]
        _poly(draw, neck, SCALE, OUTLINE, 1.0)
        upper_jaw = [
            (hx + 12, hy - 2), (hx + 64, hy + 0), (hx + 76, hy + 4), (hx + 56, hy + 10), (hx + 18, hy + 8)
        ]
        _poly(draw, upper_jaw, BELLY, OUTLINE, 0.8)
        lj = pose.jaw_open * 22.0
        lower_jaw = [
            (hx + 12, hy + 8), (hx + 46, hy + 14 + lj), (hx + 68, hy + 12 + lj), (hx + 50, hy + 20 + lj), (hx + 18, hy + 16)
        ]
        _poly(draw, lower_jaw, BELLY_SHADOW, OUTLINE, 0.8)
        if pose.x_eyes:
            _line(draw, [(hx + 2, hy - 2), (hx + 12, hy + 8)], OUTLINE, 1.0)
            _line(draw, [(hx + 2, hy + 8), (hx + 12, hy - 2)], OUTLINE, 1.0)
        elif pose.blink:
            _line(draw, [(hx + 2, hy + 2), (hx + 14, hy + 2)], EYE_HOT, 1.0)
        else:
            _ellipse(draw, hx + 8, hy + 1, 6, 4, EYE, EYE_HOT, 0.8)
            _circle(draw, (hx + 10, hy + 1), 1.6, OUTLINE, OUTLINE, 0.5)
        nostril = [
            (hx + 42, hy - 2), (hx + 48, hy - 2), (hx + 46, hy + 2)
        ]
        _poly(draw, nostril, OUTLINE, OUTLINE, 0.3)
        for xoff in (24, 34, 46, 58, 68):
            _line(draw, [(hx + xoff, hy + 6), (hx + xoff - 4, hy + 12)], CLAW, 0.7)
            _line(draw, [(hx + xoff - 2, hy + 12 + lj * 0.5), (hx + xoff + 2, hy + 8 + lj * 0.3)], CLAW, 0.7)

    def _draw_leg(self, draw: ImageDraw.ImageDraw, hip: Point, knee: Point, ankle: Point, toe: Point, back_toe: Point, front: bool) -> None:
        base = SCALE if front else SCALE_DARK
        _line(draw, [hip, knee], base, 8.4 if front else 7.6)
        _line(draw, [knee, ankle], base, 7.2 if front else 6.6)
        _line(draw, [hip, knee, ankle], OUTLINE, 2.0)
        _ellipse(draw, knee[0], knee[1], 7.0, 8.5, SCALE_LIGHT if front else SCALE, OUTLINE, 1.0)
        shin = [
            (knee[0] - 4, knee[1]), (ankle[0] - 2, ankle[1] - 8), (ankle[0] + 7, ankle[1] + 2), (knee[0] + 5, knee[1] + 4)
        ]
        _poly(draw, shin, SCALE_LIGHT if front else SCALE, OUTLINE, 0.8)
        _line(draw, [ankle, toe], CLAW, 2.5)
        _line(draw, [ankle, toe], OUTLINE, 0.9)
        _line(draw, [ankle, back_toe], CLAW, 1.8)
        _line(draw, [ankle, back_toe], OUTLINE, 0.8)
        claw2 = (toe[0] + 8, toe[1] + 3)
        _line(draw, [ankle, claw2], CLAW, 2.0)
        _line(draw, [ankle, claw2], OUTLINE, 0.8)
        sickle = (toe[0] + 4, toe[1] - 10)
        _line(draw, [toe, sickle], CLAW, 2.0)
        _line(draw, [toe, sickle], OUTLINE, 0.8)

    def _draw_back_leg(self, draw: ImageDraw.ImageDraw, P, pose: Pose) -> None:
        hip = P(-24, -34)
        knee = P(-42 + pose.back_leg * 0.18, 2 - pose.crouch * 0.10)
        ankle = P(-52 + pose.back_leg * 0.16, 14 - pose.back_foot_lift)
        toe = P(-38 + pose.back_leg * 0.10, 18 - pose.back_foot_lift)
        back_toe = P(-58 + pose.back_leg * 0.10, 20 - pose.back_foot_lift)
        self._draw_leg(draw, hip, knee, ankle, toe, back_toe, front=False)

    def _draw_front_leg(self, draw: ImageDraw.ImageDraw, P, pose: Pose) -> None:
        hip = P(20, -30)
        knee = P(34 + pose.front_leg * 0.18, 0 - pose.crouch * 0.10)
        ankle = P(44 + pose.front_leg * 0.16, 16 - pose.front_foot_lift)
        toe = P(64 + pose.front_leg * 0.10, 18 - pose.front_foot_lift)
        back_toe = P(36 + pose.front_leg * 0.10, 20 - pose.front_foot_lift)
        self._draw_leg(draw, hip, knee, ankle, toe, back_toe, front=True)

    def _draw_arm(self, draw: ImageDraw.ImageDraw, shoulder: Point, elbow: Point, wrist: Point, front: bool) -> None:
        base = SCALE_LIGHT if front else SCALE
        _line(draw, [shoulder, elbow], base, 5.2 if front else 4.6)
        _line(draw, [elbow, wrist], base, 4.6 if front else 4.0)
        _line(draw, [shoulder, elbow, wrist], OUTLINE, 1.7)
        _ellipse(draw, elbow[0], elbow[1], 4.8, 6.0, SCALE_LIGHT, OUTLINE, 0.8)
        claw1 = (wrist[0] + 8, wrist[1] + 4)
        claw2 = (wrist[0] + 6, wrist[1] - 4)
        _line(draw, [wrist, claw1], CLAW, 1.6)
        _line(draw, [wrist, claw1], OUTLINE, 0.6)
        _line(draw, [wrist, claw2], CLAW, 1.6)
        _line(draw, [wrist, claw2], OUTLINE, 0.6)

    def _draw_back_arm(self, draw: ImageDraw.ImageDraw, P, pose: Pose) -> None:
        shoulder = P(6, -92)
        elbow = P(12 + pose.back_arm * 0.16, -74 + pose.back_arm * 0.12)
        wrist = P(18 + pose.back_arm * 0.20, -56 + pose.back_arm * 0.14)
        self._draw_arm(draw, shoulder, elbow, wrist, front=False)

    def _draw_front_arm(self, draw: ImageDraw.ImageDraw, P, pose: Pose) -> None:
        shoulder = P(20, -96)
        elbow = P(26 + pose.front_arm * 0.18, -76 + pose.front_arm * 0.14)
        wrist = P(36 + pose.front_arm * 0.20, -58 + pose.front_arm * 0.12)
        self._draw_arm(draw, shoulder, elbow, wrist, front=True)

    def _draw_tail_fx(self, draw: ImageDraw.ImageDraw, P, pose: Pose) -> None:
        cx, cy = P(-74, -70)
        box = (_s(cx - 102), _s(cy - 70), _s(cx + 102), _s(cy + 70))
        draw.arc(box, 18, 166, fill=(*ACCENT[:3], 132), width=_s(5.0 + pose.sweep_arc * 2.0))
        draw.arc(box, 28, 154, fill=(255, 244, 208, 104), width=_s(2.0))

    def _draw_pounce_fx(self, draw: ImageDraw.ImageDraw, P, pose: Pose) -> None:
        c = P(-12, 12)
        _ellipse(draw, c[0], c[1], 18 + pose.pounce * 8, 6 + pose.pounce * 2, DUST, outline=(0, 0, 0, 0), width=0)
        for dx in (-16, -4, 10):
            shard = [P(-12 + dx, 10), P(-4 + dx, 0), P(4 + dx, 10)]
            _poly(draw, shard, (*ACCENT_LIGHT[:3], 148), (*ACCENT[:3], 120), 0.5)

    def _draw_bite_fx(self, draw: ImageDraw.ImageDraw, P, pose: Pose) -> None:
        cx, cy = P(126, -84)
        box = (_s(cx - 34), _s(cy - 20), _s(cx + 34), _s(cy + 20))
        draw.arc(box, 180, 24, fill=(*ACCENT_LIGHT[:3], 180), width=_s(3.0))


def _render_sheet(renderer: RaptorStalkerRenderer, out_dir: Path):
    frame_w, frame_h = FRAME_SIZE
    sheet_w = max(frames for _, frames, _ in ROWS) * frame_w
    sheet_h = len(ROWS) * frame_h
    sheet = Image.new("RGBA", (sheet_w, sheet_h), (0, 0, 0, 0))
    preview = Image.new("RGBA", (sheet_w + 128, sheet_h), (248, 246, 242, 255))
    pdraw = ImageDraw.Draw(preview)
    canonical = None
    for row_idx, (name, nframes, _ms) in enumerate(ROWS):
        pdraw.text((8, row_idx * frame_h + 8), name, fill=(36, 36, 36, 255))
        for frame_idx in range(nframes):
            frame = renderer.render_frame(name, frame_idx, nframes)
            x = frame_idx * frame_w
            y = row_idx * frame_h
            sheet.alpha_composite(frame, (x, y))
            preview.alpha_composite(frame, (x + 128, y))
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
    """Render the raptor_stalker spritesheet bundle via the shared
    `tackon_sheet.build_sheet` pipeline (auto-cropped, with the
    runtime-compatible YAML+RON shape). See `bear_mauler.render` for
    the full rationale — same conversion."""
    from ...authoring.tackon_sheet import build_sheet
    out_dir = Path(out_dir)
    out_dir.mkdir(parents=True, exist_ok=True)
    renderer = RaptorStalkerRenderer()
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
    parser = argparse.ArgumentParser(description="Render a side-profile raptor stalker enemy spritesheet.")
    parser.add_argument("--out-dir", type=Path, default=Path(__file__).resolve().parents[2] / "generated" / TARGET_BASENAME)
    args = parser.parse_args(argv)
    for path in render(args.out_dir):
        print(path)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
