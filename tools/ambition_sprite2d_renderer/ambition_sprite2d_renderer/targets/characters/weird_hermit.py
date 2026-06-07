from __future__ import annotations

"""Standalone generator for a weird hunched hermit enemy sprite.

Inspired by a loose sketch vibe: side-profile, droopy cap, long nose,
moustache, wrinkly face, long creepy fingers, awkward crouch, and shorts.
The rows support strange side-scroller attack patterns: finger jab, grab,
and curse sneeze.
"""

import argparse
import math
from dataclasses import dataclass
from pathlib import Path
from typing import List, Sequence, Tuple

from PIL import Image, ImageDraw

ACTOR_METADATA = {'actor': {'character_id': 'npc_weird_hermit', 'display_name': 'Weird Hermit'},
 'body': {'body_plan': 'HumanoidBiped',
          'body_kind': 'Standard',
          'mass_class': 'Medium',
          'traits': ['story', 'humanoid', 'story', 'hermit'],
          'locomotion_hint': 'Walk'},
 'capabilities': {'traversal': {'walk': True,
                                'jump': None,
                                'climb': None,
                                'fly': None,
                                'swim': None,
                                'crawl': None,
                                'use_lifts': True,
                                'door_access': ['public']},
                  'interactions': {'talk': True,
                                   'trade': None,
                                   'carry': None,
                                   'open_doors': ['public']}},
 'brain': {'default_preset': 'patrol_peaceful'},
 'actions': {'default_preset': 'peaceful'},
 'visual': {'default_pose': 'idle'},
 'tags': ['story', 'humanoid', 'story', 'hermit'],
 'sockets': {'head': {'source': 'explicit.profile.humanoid', 'point': {'x': 64.0, 'y': 24.0}},
             'chest': {'source': 'explicit.profile.humanoid', 'point': {'x': 64.0, 'y': 54.0}},
             'hand_l': {'source': 'explicit.profile.humanoid', 'point': {'x': 48.0, 'y': 64.0}},
             'hand_r': {'source': 'explicit.profile.humanoid', 'point': {'x': 80.0, 'y': 64.0}},
             'speech_bubble': {'source': 'explicit.profile.humanoid',
                               'point': {'x': 64.0, 'y': 8.0}}},
 'animation_bindings': {'default': {'animation': 'idle', 'events': []},
                        'locomotion.walk': {'animation': 'walk', 'events': []},
                        'interaction.talk': {'animation': 'talk', 'events': []},
                        'interaction.use': {'animation': 'interact', 'events': []}}}


RGBA = Tuple[int, int, int, int]
Point = Tuple[float, float]

TARGET_BASENAME = "weird_hermit"
# Files the tack-on installer copies into the sandbox sprites dir.
# Aligned with the rest of the project's `<target>_spritesheet.{ext}`
# convention as of 2026-05-24 — the previous bare-name output was a
# divergence from the runtime's manifest scanner expectations.
SHEET_FILES = [
    f"{TARGET_BASENAME}_spritesheet.png",
    f"{TARGET_BASENAME}_spritesheet.yaml",
    f"{TARGET_BASENAME}_spritesheet.ron",
]
FRAME_SIZE = (240, 224)
WORK_FRAME_SIZE = (480, 448)
SUPER = 4
ROWS: List[Tuple[str, int, int]] = [
    ("idle", 6, 130),
    ("creep", 8, 95),
    ("finger_jab", 7, 75),
    ("grab", 7, 85),
    ("curse_sneeze", 7, 90),
    ("hurt", 4, 90),
    ("death", 8, 110),
]

OUTLINE = (24, 20, 18, 255)
SKIN = (206, 170, 132, 255)
SKIN_SHADOW = (152, 112, 84, 255)
SKIN_LIGHT = (232, 204, 170, 255)
NOSE = (194, 136, 104, 255)
MOUSTACHE = (66, 52, 42, 255)
CLOTH = (224, 213, 190, 255)
CLOTH_SHADOW = (154, 144, 126, 255)
SHORTS = (76, 68, 70, 255)
SHORTS_HI = (118, 106, 108, 255)
EYE = (34, 30, 28, 255)
CURSE = (116, 220, 176, 145)
DUST = (205, 168, 125, 115)
SHADOW = (0, 0, 0, 40)


@dataclass
class Pose:
    root_x: float = 0.0
    root_y: float = 0.0
    bob: float = 0.0
    lean: float = 0.0
    torso_tilt: float = 0.0
    head_tilt: float = 0.0
    cap_swing: float = 0.0
    jaw: float = 0.0
    near_arm: float = 0.0
    far_arm: float = 0.0
    near_reach: float = 0.0
    far_reach: float = 0.0
    near_leg: float = 0.0
    far_leg: float = 0.0
    near_foot_lift: float = 0.0
    far_foot_lift: float = 0.0
    crouch: float = 0.0
    jab: float = 0.0
    grab: float = 0.0
    sneeze: float = 0.0
    hurt: float = 0.0
    x_eyes: bool = False
    blink: bool = False

    def __init__(self, anim: str, frame_idx: int, nframes: int) -> None:
        t = frame_idx / max(1, nframes - 1)
        cyc = math.tau * frame_idx / max(1, nframes)
        s = math.sin(cyc)
        c = math.cos(cyc)
        self.root_x = self.root_y = self.bob = self.lean = 0.0
        self.torso_tilt = self.head_tilt = self.cap_swing = self.jaw = 0.0
        self.near_arm = self.far_arm = self.near_reach = self.far_reach = 0.0
        self.near_leg = self.far_leg = self.near_foot_lift = self.far_foot_lift = 0.0
        self.crouch = self.jab = self.grab = self.sneeze = self.hurt = 0.0
        self.x_eyes = self.blink = False
        if anim == "idle":
            self.bob = s * 1.3
            self.lean = -2.0 + s * 1.3
            self.torso_tilt = -6.0 + s * 1.0
            self.head_tilt = -6.0 - s * 1.4
            self.cap_swing = s * 7.0
            self.near_arm = -8.0 + s * 4.0
            self.far_arm = -18.0 - s * 3.0
            self.near_leg = c * 1.2
            self.far_leg = -c * 1.2
            self.crouch = 4.0 + abs(s) * 1.0
            self.jaw = max(0.0, s) * 0.04
            self.blink = frame_idx == nframes - 2
        elif anim == "creep":
            self.root_x = s * 2.2
            self.bob = abs(s) * 2.8 - 0.8
            self.lean = -5.0 + s * 2.0
            self.torso_tilt = -8.0 + s * 2.0
            self.head_tilt = -8.0 - s * 2.0
            self.cap_swing = -s * 10.0
            self.near_arm = -16.0 * s - 6.0
            self.far_arm = 14.0 * s - 20.0
            self.near_reach = max(0.0, -s) * 8.0
            self.far_reach = max(0.0, s) * 7.0
            self.near_leg = 18.0 * s
            self.far_leg = -18.0 * s
            self.near_foot_lift = max(0.0, s) * 8.0
            self.far_foot_lift = max(0.0, -s) * 7.0
            self.crouch = 6.0
        elif anim == "finger_jab":
            tt = _ease(t)
            hit = math.sin(tt * math.pi)
            self.root_x = _lerp(-7.0, 11.0, tt)
            self.bob = -hit * 3.0
            self.lean = _lerp(-12.0, 10.0, tt)
            self.torso_tilt = _lerp(-10.0, 5.0, tt)
            self.head_tilt = _lerp(-12.0, 8.0, tt)
            self.cap_swing = _lerp(10.0, -8.0, tt)
            self.near_arm = _lerp(-60.0, 18.0, tt)
            self.near_reach = _lerp(0.0, 42.0, tt)
            self.far_arm = _lerp(-20.0, 0.0, tt)
            self.near_leg = -8.0 - hit * 3.0
            self.far_leg = 12.0 + hit * 2.0
            self.crouch = _lerp(10.0, 3.0, tt)
            self.jab = hit
            self.jaw = 0.10 * hit
        elif anim == "grab":
            tt = _ease(t)
            hit = math.sin(tt * math.pi)
            self.root_x = _lerp(-5.0, 8.0, tt)
            self.bob = -hit * 2.0
            self.lean = _lerp(-4.0, 16.0, tt)
            self.torso_tilt = _lerp(-8.0, 8.0, tt)
            self.head_tilt = _lerp(-8.0, 6.0, tt)
            self.near_arm = _lerp(-72.0, 28.0, tt)
            self.far_arm = _lerp(-56.0, 20.0, tt)
            self.near_reach = _lerp(10.0, 36.0, tt)
            self.far_reach = _lerp(0.0, 28.0, tt)
            self.near_leg = -10.0
            self.far_leg = 12.0
            self.crouch = 8.0 - hit * 2.0
            self.grab = hit
            self.jaw = 0.15 * hit
        elif anim == "curse_sneeze":
            tt = _ease(t)
            puff = math.sin(tt * math.pi)
            self.root_x = math.sin(t * math.pi * 5.0) * (1.0 - t) * 3.0
            self.bob = -puff * 2.5
            self.lean = _lerp(8.0, -20.0, tt)
            self.torso_tilt = _lerp(2.0, -18.0, tt)
            self.head_tilt = _lerp(12.0, -18.0, tt)
            self.cap_swing = _lerp(-14.0, 18.0, tt)
            self.near_arm = 26.0 * puff - 8.0
            self.far_arm = 18.0 * puff - 18.0
            self.near_leg = 8.0 * puff
            self.far_leg = -8.0 * puff
            self.crouch = 9.0 + puff * 2.0
            self.jaw = 0.35 * puff
            self.sneeze = puff
        elif anim == "hurt":
            hit = math.sin(t * math.pi)
            shake = math.sin(t * math.pi * 4.0) * (1.0 - t)
            self.root_x = shake * 4.0
            self.bob = -hit * 2.0
            self.lean = -18.0 * hit
            self.torso_tilt = -14.0 * hit
            self.head_tilt = 18.0 * hit
            self.near_arm = 32.0 * hit
            self.far_arm = 26.0 * hit
            self.near_leg = 10.0 * hit
            self.far_leg = -8.0 * hit
            self.jaw = 0.24 * hit
            self.hurt = hit
        elif anim == "death":
            tt = _ease(t)
            self.root_x = tt * 16.0
            self.root_y = tt * 7.0
            self.bob = -tt * 4.0
            self.lean = -82.0 * tt
            self.torso_tilt = -38.0 * tt
            self.head_tilt = 30.0 * tt
            self.cap_swing = 20.0 * tt
            self.near_arm = _lerp(-8.0, 58.0, tt)
            self.far_arm = _lerp(-18.0, -55.0, tt)
            self.near_leg = _lerp(0.0, 28.0, tt)
            self.far_leg = _lerp(0.0, -26.0, tt)
            self.near_foot_lift = tt * 8.0
            self.far_foot_lift = tt * 5.0
            self.crouch = tt * 5.0
            self.jaw = 0.25 * tt
            self.x_eyes = tt > 0.55


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


class WeirdHermitRenderer:
    def render_frame(self, anim: str, frame_idx: int, nframes: int) -> Image.Image:
        img = Image.new("RGBA", (WORK_FRAME_SIZE[0] * SUPER, WORK_FRAME_SIZE[1] * SUPER), (0, 0, 0, 0))
        draw = ImageDraw.Draw(img, "RGBA")
        pose = Pose(anim, frame_idx, nframes)
        root = (WORK_FRAME_SIZE[0] * 0.43 + pose.root_x, WORK_FRAME_SIZE[1] * 0.80 + pose.root_y + pose.bob)
        global_tilt = pose.lean

        def P(x: float, y: float) -> Point:
            rx, ry = _rot_local(x, y, global_tilt)
            return (root[0] + rx, root[1] + ry)

        # No baked ground drop shadow; the scene renderer owns contact shadows.
        self._draw_far_leg(draw, P, pose)
        self._draw_far_arm(draw, P, pose)
        self._draw_torso(draw, P, pose)
        self._draw_head(draw, P, pose)
        self._draw_near_leg(draw, P, pose)
        self._draw_near_arm(draw, P, pose)
        if pose.jab > 0.2:
            self._draw_jab_fx(draw, P, pose)
        if pose.grab > 0.2:
            self._draw_grab_fx(draw, P, pose)
        if pose.sneeze > 0.15:
            self._draw_sneeze_fx(draw, P, pose)
        return _downsample(img)

    def _draw_shadow(self, draw, P, pose):
        c = P(-2, 17)
        _ellipse(draw, c[0], c[1], 46, 10, SHADOW, outline=(0, 0, 0, 0), width=0)

    def _draw_torso(self, draw, P, pose):
        # Long hunched torso with sloped shoulders and pot belly.
        torso = [
            P(-30, -122 - pose.crouch), P(8, -150 - pose.crouch), P(38, -128 - pose.crouch),
            P(48, -82), P(38, -44), P(4, -32), P(-28, -48), P(-42, -88)
        ]
        _poly(draw, torso, SKIN, OUTLINE, 1.6)
        belly = [P(-6, -94), P(28, -90), P(40, -66), P(28, -48), P(0, -52), P(-12, -72)]
        _poly(draw, belly, SKIN_LIGHT, OUTLINE, 0.8)
        chest_mark = [P(2, -112), P(10, -116), P(15, -106), P(8, -100)]
        _poly(draw, chest_mark, SKIN_SHADOW, OUTLINE, 0.6)
        _ellipse(draw, *P(20, -98), 3.0, 4.0, SKIN_SHADOW, OUTLINE, 0.5)
        # Shorts / loincloth, angled around the crouch.
        shorts = [P(-24, -45), P(42, -44), P(52, -20), P(20, -7), P(-18, -16), P(-34, -32)]
        _poly(draw, shorts, SHORTS, OUTLINE, 1.2)
        _line(draw, [P(-18, -40), P(36, -40)], SHORTS_HI, 0.9)
        _line(draw, [P(14, -38), P(18, -10)], SHORTS_HI, 0.8)

    def _draw_head(self, draw, P, pose):
        hx, hy = P(12, -158 - pose.crouch * 0.35 + pose.head_tilt * 0.14)
        # Floppy cloth cap / head rag.
        cap = [(hx - 16, hy - 34), (hx + 2, hy - 48), (hx + 36, hy - 42), (hx + 55 + pose.cap_swing * 0.25, hy - 24), (hx + 42 + pose.cap_swing * 0.45, hy - 6), (hx + 18, hy - 22)]
        _poly(draw, cap, CLOTH, OUTLINE, 1.0)
        flap = [(hx + 34, hy - 34), (hx + 78 + pose.cap_swing * 0.35, hy - 40), (hx + 72 + pose.cap_swing * 0.5, hy - 10), (hx + 38, hy - 8)]
        _poly(draw, flap, CLOTH_SHADOW, OUTLINE, 1.0)
        head = [(hx - 22, hy - 28), (hx + 12, hy - 36), (hx + 38, hy - 18), (hx + 40, hy + 10), (hx + 18, hy + 32), (hx - 12, hy + 26), (hx - 30, hy + 0)]
        _poly(draw, head, SKIN, OUTLINE, 1.3)
        # Long nose and sagging face are the primary read.
        nose = [(hx + 18, hy - 8), (hx + 62, hy - 6), (hx + 74, hy + 2), (hx + 54, hy + 10), (hx + 20, hy + 6)]
        _poly(draw, nose, NOSE, OUTLINE, 1.0)
        _circle(draw, (hx + 61, hy + 2), 2.2, OUTLINE, OUTLINE, 0.4)
        chin = [(hx + 10, hy + 18), (hx + 38, hy + 20 + pose.jaw * 18), (hx + 26, hy + 38 + pose.jaw * 14), (hx + 4, hy + 30)]
        _poly(draw, chin, SKIN_SHADOW, OUTLINE, 0.8)
        # Moustache, brows, eye.
        _line(draw, [(hx + 20, hy + 8), (hx + 36, hy + 16), (hx + 50, hy + 12)], MOUSTACHE, 2.3)
        _line(draw, [(hx + 20, hy + 10), (hx + 31, hy + 23)], MOUSTACHE, 1.8)
        _line(draw, [(hx - 8, hy - 15), (hx + 10, hy - 18)], OUTLINE, 1.3)
        if pose.x_eyes:
            _line(draw, [(hx + 2, hy - 10), (hx + 12, hy + 0)], OUTLINE, 1.0)
            _line(draw, [(hx + 2, hy + 0), (hx + 12, hy - 10)], OUTLINE, 1.0)
        elif pose.blink:
            _line(draw, [(hx + 2, hy - 7), (hx + 14, hy - 7)], OUTLINE, 1.0)
        else:
            _ellipse(draw, hx + 8, hy - 8, 4.0, 2.8, (245, 236, 208, 255), OUTLINE, 0.6)
            _circle(draw, (hx + 9, hy - 8), 1.2, EYE, OUTLINE, 0.3)
        # Ear and wrinkles.
        _ellipse(draw, hx - 22, hy - 2, 6, 10, SKIN_SHADOW, OUTLINE, 0.8)
        _line(draw, [(hx + 0, hy + 6), (hx + 12, hy + 10)], SKIN_SHADOW, 0.8)
        _line(draw, [(hx - 2, hy + 14), (hx + 10, hy + 18)], SKIN_SHADOW, 0.7)

    def _draw_arm(self, draw, shoulder, elbow, hand, front: bool, reach: float):
        skin = SKIN if front else SKIN_SHADOW
        _line(draw, [shoulder, elbow], skin, 7.5 if front else 6.0)
        _line(draw, [elbow, hand], skin, 6.8 if front else 5.4)
        _line(draw, [shoulder, elbow, hand], OUTLINE, 1.8 if front else 1.3)
        _ellipse(draw, elbow[0], elbow[1], 6.5 if front else 5.0, 8.0 if front else 6.0, skin, OUTLINE, 0.8)
        _circle(draw, hand, 5.2 if front else 4.2, skin, OUTLINE, 0.8)
        # Four long creepy fingers, anchored at the hand.
        for idx, dy in enumerate((-6, -2, 2, 6)):
            length = (18 + idx * 2 + reach * 0.10) if front else (13 + idx)
            tip = (hand[0] + length, hand[1] + dy + idx * 1.0)
            base = (hand[0] + 3, hand[1] + dy * 0.4)
            _line(draw, [base, tip], skin, 1.5 if front else 1.0)
            _line(draw, [base, tip], OUTLINE, 0.55)

    def _draw_near_arm(self, draw, P, pose):
        shoulder = P(28, -112)
        elbow = P(42 + pose.near_arm * 0.10 + pose.near_reach * 0.22, -76 + pose.near_arm * 0.18)
        hand = P(54 + pose.near_arm * 0.20 + pose.near_reach, -47 + pose.near_arm * 0.14)
        self._draw_arm(draw, shoulder, elbow, hand, True, pose.near_reach)

    def _draw_far_arm(self, draw, P, pose):
        shoulder = P(4, -112)
        elbow = P(12 + pose.far_arm * 0.10 + pose.far_reach * 0.16, -78 + pose.far_arm * 0.14)
        hand = P(24 + pose.far_arm * 0.16 + pose.far_reach, -54 + pose.far_arm * 0.12)
        self._draw_arm(draw, shoulder, elbow, hand, False, pose.far_reach)

    def _draw_leg(self, draw, hip, knee, foot, front: bool):
        skin = SKIN if front else SKIN_SHADOW
        _line(draw, [hip, knee], skin, 9.0 if front else 7.2)
        _line(draw, [knee, foot], skin, 8.0 if front else 6.4)
        _line(draw, [hip, knee, foot], OUTLINE, 1.9 if front else 1.3)
        _ellipse(draw, knee[0], knee[1], 7.0 if front else 5.6, 9.0 if front else 7.0, skin, OUTLINE, 0.8)
        toes = [(foot[0] - 12, foot[1] + 3), (foot[0] + 22, foot[1] + 1), (foot[0] + 26, foot[1] + 7), (foot[0] - 10, foot[1] + 8)]
        _poly(draw, toes, skin, OUTLINE, 0.9)
        for dx in (8, 15, 22):
            _line(draw, [(foot[0] + dx, foot[1] + 3), (foot[0] + dx + 4, foot[1] + 7)], OUTLINE, 0.6)

    def _draw_near_leg(self, draw, P, pose):
        hip = P(30, -22)
        knee = P(44 + pose.near_leg * 0.16, 18 - pose.crouch * 0.35)
        foot = P(30 + pose.near_leg * 0.14, 28 - pose.near_foot_lift)
        self._draw_leg(draw, hip, knee, foot, True)

    def _draw_far_leg(self, draw, P, pose):
        hip = P(-18, -24)
        knee = P(-26 + pose.far_leg * 0.16, 12 - pose.crouch * 0.22)
        foot = P(-8 + pose.far_leg * 0.14, 29 - pose.far_foot_lift)
        self._draw_leg(draw, hip, knee, foot, False)

    def _draw_jab_fx(self, draw, P, pose):
        c = P(108 + pose.near_reach * 0.35, -48)
        _line(draw, [(c[0] - 14, c[1]), (c[0] + 18, c[1] - 2)], (255, 235, 180, 115), 2.0)
        _line(draw, [(c[0] - 6, c[1] - 8), (c[0] + 12, c[1] - 14)], (255, 235, 180, 80), 1.1)

    def _draw_grab_fx(self, draw, P, pose):
        c = P(104 + pose.near_reach * 0.25, -52)
        box = (_s(c[0] - 38), _s(c[1] - 28), _s(c[0] + 38), _s(c[1] + 28))
        draw.arc(box, 200, 340, fill=(255, 230, 160, 110), width=_s(3.0))

    def _draw_sneeze_fx(self, draw, P, pose):
        origin = P(74, -154)
        for i, (dx, dy, r) in enumerate([(24, -5, 10), (42, -10, 14), (61, -5, 18), (82, 4, 13)]):
            alpha = int(80 + pose.sneeze * 65 - i * 10)
            _ellipse(draw, origin[0] + dx, origin[1] + dy, r, r * 0.7, (*CURSE[:3], max(0, alpha)), outline=(0, 0, 0, 0), width=0)
        _line(draw, [(origin[0] + 12, origin[1] + 2), (origin[0] + 78, origin[1] - 4)], (*CURSE[:3], 130), 2.0)


def _write_yaml(path: Path) -> None:
    # Emit the SheetRow shape the rest of the project uses:
    # animation/row_index/frame_count/duration_ms/duration_secs/rects.
    fw, fh = FRAME_SIZE
    lines = [
        f"target: {TARGET_BASENAME}",
        f"image: {TARGET_BASENAME}_spritesheet.png",
        "label_width: 0",
        f"frame_width: {fw}",
        f"frame_height: {fh}",
        "rows:",
    ]
    for row_index, (name, frames, ms) in enumerate(ROWS):
        lines += [
            f"  - animation: {name}",
            f"    row_index: {row_index}",
            f"    frame_count: {frames}",
            f"    duration_ms: {ms}",
            f"    duration_secs: {ms / 1000.0}",
            "    rects:",
        ]
        for fi in range(frames):
            lines += [
                f"      - x: {fi * fw}",
                f"        y: {row_index * fh}",
                f"        w: {fw}",
                f"        h: {fh}",
            ]
    path.write_text("\n".join(lines) + "\n")


def _write_ron(path: Path) -> None:
    # Vec<SheetRecord>-shaped RON so the runtime + record_index can
    # consume the file without special-casing weird_hermit.
    fw, fh = FRAME_SIZE
    out_lines = ["[", "(", f'    target: "{TARGET_BASENAME}",',
                 f'    image: "{TARGET_BASENAME}_spritesheet.png",',
                 "    label_width: 0,",
                 f"    frame_width: {fw},",
                 f"    frame_height: {fh},",
                 "    rows: ["]
    for row_index, (name, frames, ms) in enumerate(ROWS):
        out_lines.append("        (")
        out_lines.append(f'            animation: "{name}",')
        out_lines.append(f"            row_index: {row_index},")
        out_lines.append(f"            frame_count: {frames},")
        out_lines.append(f"            duration_ms: {ms},")
        out_lines.append(f"            duration_secs: {ms / 1000.0},")
        out_lines.append("            rects: [")
        for fi in range(frames):
            out_lines.append(
                f"                (x: {fi * fw}, y: {row_index * fh}, "
                f"w: {fw}, h: {fh}, anchors: {{}}),"
            )
        out_lines.append("            ],")
        out_lines.append("        ),")
    out_lines += ["    ],", ")", "]"]
    path.write_text("\n".join(out_lines) + "\n")


def _render_sheet(renderer: WeirdHermitRenderer, out_dir: Path) -> List[Path]:
    fw, fh = FRAME_SIZE
    sheet_w = max(frames for _, frames, _ in ROWS) * fw
    sheet_h = len(ROWS) * fh
    sheet = Image.new("RGBA", (sheet_w, sheet_h), (0, 0, 0, 0))
    preview = Image.new("RGBA", (sheet_w + 128, sheet_h), (248, 246, 242, 255))
    pdraw = ImageDraw.Draw(preview)
    canonical = None
    for row_idx, (name, nframes, _ms) in enumerate(ROWS):
        pdraw.text((8, row_idx * fh + 8), name, fill=(36, 36, 36, 255))
        for frame_idx in range(nframes):
            frame = renderer.render_frame(name, frame_idx, nframes)
            x = frame_idx * fw
            y = row_idx * fh
            sheet.alpha_composite(frame, (x, y))
            preview.alpha_composite(frame, (x + 128, y))
            if canonical is None and name == "idle" and frame_idx == 0:
                canonical = frame
    if canonical is None:
        canonical = renderer.render_frame(ROWS[0][0], 0, ROWS[0][1])
    paths = [
        out_dir / f"{TARGET_BASENAME}_spritesheet.png",
        out_dir / f"{TARGET_BASENAME}_spritesheet.yaml",
        out_dir / f"{TARGET_BASENAME}_spritesheet.ron",
        out_dir / f"{TARGET_BASENAME}_preview_labeled.png",
        out_dir / f"{TARGET_BASENAME}_canonical.png",
    ]
    sheet.save(paths[0])
    _write_yaml(paths[1])
    _write_ron(paths[2])
    preview.save(paths[3])
    canonical.save(paths[4])
    return paths


def render(out_dir: str | Path, **opts) -> List[Path]:
    out_dir = Path(out_dir)
    out_dir.mkdir(parents=True, exist_ok=True)
    return _render_sheet(WeirdHermitRenderer(), out_dir)


def main(argv: List[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="Render a side-profile weird hermit enemy spritesheet.")
    parser.add_argument("--out-dir", type=Path, default=Path(__file__).resolve().parents[2] / "generated" / TARGET_BASENAME)
    args = parser.parse_args(argv)
    for path in render(args.out_dir):
        print(path)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
