from __future__ import annotations

"""Standalone generator for a heavy robot enemy with built-in variants.

This target is authored from scratch and only reuses ``build_sheet`` for
spritesheet + metadata emission.  The renderer supports multiple curated
variants so you can generate a small squad of related heavy robots that feel
like the same faction without reading as exact clones.
"""

import argparse
import math
from dataclasses import dataclass
from pathlib import Path
from typing import Dict, Iterable, List, Sequence, Tuple

from PIL import Image, ImageDraw

from ...authoring.tackon_sheet import build_sheet

ACTOR_METADATA = {'actor': {'character_id': 'npc_robot_heavy', 'display_name': 'Robot Heavy'},
 'body': {'body_plan': 'HumanoidBiped',
          'body_kind': 'Wide',
          'mass_class': 'Heavy',
          'traits': ['robot', 'enemy', 'heavy'],
          'locomotion_hint': 'Walk'},
 'capabilities': {'traversal': {'walk': True,
                                'jump': {'height_px': 24.0,
                                         'distance_px': 48.0,
                                         'source': 'explicit.profile.robot'},
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
 'brain': {'default_preset': 'melee_brute_brute'},
 'actions': {'default_preset': 'brute_lunge'},
 'visual': {'default_pose': 'idle'},
 'tags': ['robot', 'enemy', 'heavy'],
 'sockets': {'head': {'source': 'explicit.profile.robot', 'point': {'x': 64.0, 'y': 24.0}},
             'chest': {'source': 'explicit.profile.robot', 'point': {'x': 64.0, 'y': 54.0}},
             'hand_l': {'source': 'explicit.profile.robot', 'point': {'x': 48.0, 'y': 64.0}},
             'hand_r': {'source': 'explicit.profile.robot', 'point': {'x': 80.0, 'y': 64.0}},
             'muzzle': {'source': 'explicit.profile.robot', 'point': {'x': 90.0, 'y': 58.0}},
             'projectile_origin': {'source': 'explicit.profile.robot',
                                   'point': {'x': 90.0, 'y': 58.0}}},
 'animation_bindings': {'default': {'animation': 'idle', 'events': []},
                        'locomotion.walk': {'animation': 'walk', 'events': []},
                        'locomotion.run': {'animation': 'run', 'events': []},
                        'action.melee.primary': {'animation': 'slash',
                                                 'events': [{'t': 0.35,
                                                             'event': 'hitbox_active_start',
                                                             'source': 'explicit.profile.robot'},
                                                            {'t': 0.55,
                                                             'event': 'hitbox_active_end',
                                                             'source': 'explicit.profile.robot'}]},
                        'action.ranged.primary': {'animation': 'shoot',
                                                  'events': [{'t': 0.5,
                                                              'event': 'projectile_release',
                                                              'source': 'explicit.profile.robot'}]}}}


RGBA = Tuple[int, int, int, int]
Point = Tuple[float, float]

TARGET_BASENAME = "robot_heavy"
FRAME_SIZE = (240, 224)
WORK_FRAME_SIZE = (480, 448)
SUPER = 4
ROWS: List[Tuple[str, int, int]] = [
    ("idle", 6, 130),
    ("walk", 8, 95),
    ("smash", 7, 85),
    ("taunt", 6, 110),
    ("hurt", 4, 90),
    ("death", 8, 110),
]

OUTLINE = (18, 20, 26, 255)
GROUND_SHADOW = (0, 0, 0, 46)
SMOKE = (0, 0, 0, 38)
WHITE = (245, 246, 248, 255)
STEEL = (198, 204, 210, 255)
STEEL_DARK = (100, 107, 116, 255)


@dataclass(frozen=True)
class VariantSpec:
    key: str
    display_name: str
    shell_dark: RGBA
    shell_mid: RGBA
    shell_light: RGBA
    plate: RGBA
    plate_dark: RGBA
    accent: RGBA
    accent_hot: RGBA
    glow: RGBA
    glow_hot: RGBA
    cable: RGBA
    rust: RGBA
    body_scale: float
    shoulder_scale: float
    head_scale: float
    arm_scale: float
    leg_scale: float
    foot_scale: float
    weapon_scale: float
    backpack_style: str
    head_style: str
    weapon_style: str
    shoulder_pods: bool
    antenna: bool
    chest_window: bool
    belt_fins: bool

    @property
    def target_name(self) -> str:
        return f"{TARGET_BASENAME}_{self.key}"


VARIANTS: Dict[str, VariantSpec] = {
    "bastion": VariantSpec(
        key="bastion",
        display_name="Bastion Bruiser",
        shell_dark=(40, 46, 56, 255),
        shell_mid=(78, 88, 102, 255),
        shell_light=(118, 130, 146, 255),
        plate=(96, 104, 116, 255),
        plate_dark=(58, 62, 70, 255),
        accent=(171, 56, 53, 255),
        accent_hot=(231, 112, 90, 255),
        glow=(255, 90, 84, 255),
        glow_hot=(255, 186, 160, 255),
        cable=(160, 130, 84, 255),
        rust=(112, 74, 48, 255),
        body_scale=1.14,
        shoulder_scale=1.20,
        head_scale=1.04,
        arm_scale=1.14,
        leg_scale=1.08,
        foot_scale=1.12,
        weapon_scale=1.18,
        backpack_style="stacks",
        head_style="mono",
        weapon_style="maul",
        shoulder_pods=True,
        antenna=False,
        chest_window=True,
        belt_fins=False,
    ),
    "foundry": VariantSpec(
        key="foundry",
        display_name="Foundry Ram",
        shell_dark=(58, 44, 36, 255),
        shell_mid=(116, 86, 62, 255),
        shell_light=(154, 120, 90, 255),
        plate=(92, 84, 76, 255),
        plate_dark=(56, 50, 46, 255),
        accent=(214, 132, 40, 255),
        accent_hot=(255, 180, 62, 255),
        glow=(255, 158, 64, 255),
        glow_hot=(255, 220, 136, 255),
        cable=(148, 98, 50, 255),
        rust=(154, 78, 30, 255),
        body_scale=1.10,
        shoulder_scale=1.10,
        head_scale=1.00,
        arm_scale=1.16,
        leg_scale=1.04,
        foot_scale=1.08,
        weapon_scale=1.10,
        backpack_style="furnace",
        head_style="visor",
        weapon_style="pile",
        shoulder_pods=False,
        antenna=False,
        chest_window=True,
        belt_fins=True,
    ),
    "volt": VariantSpec(
        key="volt",
        display_name="Volt Crusher",
        shell_dark=(34, 38, 64, 255),
        shell_mid=(76, 84, 130, 255),
        shell_light=(120, 132, 186, 255),
        plate=(86, 96, 126, 255),
        plate_dark=(44, 50, 72, 255),
        accent=(70, 162, 232, 255),
        accent_hot=(122, 224, 255, 255),
        glow=(106, 232, 255, 255),
        glow_hot=(216, 252, 255, 255),
        cable=(126, 134, 188, 255),
        rust=(78, 84, 130, 255),
        body_scale=1.06,
        shoulder_scale=1.12,
        head_scale=1.08,
        arm_scale=1.10,
        leg_scale=1.10,
        foot_scale=1.02,
        weapon_scale=1.12,
        backpack_style="coil",
        head_style="dual",
        weapon_style="arc",
        shoulder_pods=True,
        antenna=True,
        chest_window=False,
        belt_fins=True,
    ),
}


@dataclass
class Pose:
    root_x: float = 0.0
    root_y: float = 0.0
    bob: float = 0.0
    lean: float = 0.0
    torso_tilt: float = 0.0
    head_tilt: float = 0.0
    front_arm: float = 0.0
    back_arm: float = 0.0
    front_leg: float = 0.0
    back_leg: float = 0.0
    front_foot_lift: float = 0.0
    back_foot_lift: float = 0.0
    weapon_angle: float = -62.0
    weapon_shift_x: float = 0.0
    weapon_shift_y: float = 0.0
    backpack_sway: float = 0.0
    vent_burst: float = 0.0
    mouth_open: float = 0.0
    blink: bool = False
    dead: bool = False
    x_eyes: bool = False
    impact: float = 0.0
    fade: float = 0.0

    def __init__(self, anim: str, frame_idx: int, nframes: int):
        t = frame_idx / max(1, nframes - 1)
        cyc = math.tau * frame_idx / max(1, nframes)
        s = math.sin(cyc)
        c = math.cos(cyc)

        self.root_x = 0.0
        self.root_y = 0.0
        self.bob = 0.0
        self.lean = 0.0
        self.torso_tilt = 0.0
        self.head_tilt = 0.0
        self.front_arm = 0.0
        self.back_arm = 0.0
        self.front_leg = 0.0
        self.back_leg = 0.0
        self.front_foot_lift = 0.0
        self.back_foot_lift = 0.0
        self.weapon_angle = -62.0
        self.weapon_shift_x = 0.0
        self.weapon_shift_y = 0.0
        self.backpack_sway = 0.0
        self.vent_burst = 0.0
        self.mouth_open = 0.0
        self.blink = False
        self.dead = False
        self.x_eyes = False
        self.impact = 0.0
        self.fade = 0.0

        if anim == "idle":
            self.bob = s * 1.6
            self.lean = s * 1.4
            self.torso_tilt = s * 1.2
            self.head_tilt = -s * 1.4
            self.front_arm = 4.0 + s * 3.0
            self.back_arm = -5.0 - s * 2.5
            self.front_leg = c * 1.0
            self.back_leg = -c * 1.0
            self.weapon_angle = -64.0 + s * 2.0
            self.backpack_sway = -s * 4.0
            self.vent_burst = max(0.0, s) * 0.4
            self.blink = frame_idx == nframes - 2
        elif anim == "walk":
            self.root_x = s * 2.2
            self.bob = abs(s) * 3.0 - 0.8
            self.lean = s * 3.0
            self.torso_tilt = s * 1.8
            self.head_tilt = -s * 2.0
            self.front_leg = 18.0 * s
            self.back_leg = -18.0 * s
            self.front_arm = -10.0 * s + 4.0
            self.back_arm = 10.0 * s - 4.0
            self.front_foot_lift = max(0.0, s) * 8.0
            self.back_foot_lift = max(0.0, -s) * 8.0
            self.weapon_angle = -70.0 - s * 6.0
            self.backpack_sway = -s * 9.0
            self.vent_burst = max(0.0, -s) * 0.55
        elif anim == "smash":
            tt = _ease(t)
            hit = math.sin(tt * math.pi)
            self.root_x = _lerp(-8.0, 10.0, tt)
            self.bob = -hit * 6.0
            self.lean = _lerp(-8.0, 18.0, tt)
            self.torso_tilt = _lerp(-4.0, 14.0, tt)
            self.head_tilt = _lerp(-8.0, 6.0, tt)
            self.front_arm = _lerp(-114.0, 40.0, tt)
            self.back_arm = _lerp(-40.0, 28.0, tt)
            self.front_leg = -12.0 - hit * 4.0
            self.back_leg = 14.0 + hit * 5.0
            self.weapon_angle = _lerp(-150.0, 16.0, tt)
            self.weapon_shift_x = _lerp(-8.0, 20.0, tt)
            self.weapon_shift_y = -hit * 12.0
            self.backpack_sway = _lerp(12.0, -16.0, tt)
            self.vent_burst = hit
            self.impact = hit
            self.mouth_open = 0.18 * hit
        elif anim == "taunt":
            self.bob = s * 1.0
            self.lean = -6.0 + s * 1.5
            self.torso_tilt = -5.0
            self.head_tilt = -4.0 + s * 1.5
            self.front_arm = -100.0 + s * 6.0
            self.back_arm = 20.0 + s * 5.0
            self.weapon_angle = -104.0 + s * 4.0
            self.backpack_sway = s * 6.0
            self.vent_burst = 0.25 + max(0.0, s) * 0.3
            self.mouth_open = 0.12
        elif anim == "hurt":
            hit = math.sin(t * math.pi)
            shake = math.sin(t * math.pi * 4.0) * (1.0 - t)
            self.root_x = shake * 5.0
            self.bob = -hit * 2.5
            self.lean = -14.0 * hit
            self.torso_tilt = -10.0 * hit
            self.head_tilt = 12.0 * hit
            self.front_arm = 18.0 * hit
            self.back_arm = 16.0 * hit
            self.front_leg = 8.0 * hit
            self.back_leg = -6.0 * hit
            self.weapon_angle = -56.0 + 12.0 * hit
            self.backpack_sway = -10.0 * hit
            self.vent_burst = 0.8 * hit
            self.mouth_open = 0.18 * hit
        elif anim == "death":
            tt = _ease(t)
            self.root_x = tt * 16.0
            self.root_y = tt * 4.0
            self.bob = -tt * 4.0
            self.lean = -84.0 * tt
            self.torso_tilt = -28.0 * tt
            self.head_tilt = 26.0 * tt
            self.front_arm = _lerp(-12.0, 66.0, tt)
            self.back_arm = _lerp(-12.0, -70.0, tt)
            self.front_leg = _lerp(0.0, 30.0, tt)
            self.back_leg = _lerp(0.0, -28.0, tt)
            self.weapon_angle = _lerp(-62.0, -124.0, tt)
            self.weapon_shift_x = tt * 14.0
            self.weapon_shift_y = tt * 6.0
            self.backpack_sway = tt * 12.0
            self.vent_burst = tt * 0.5
            self.mouth_open = 0.18 * tt
            self.dead = tt > 0.7
            self.x_eyes = tt > 0.55
            self.fade = tt * 0.05


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


def _rect_poly(center: Point, w: float, h: float, deg: float) -> List[Point]:
    hw = w * 0.5
    hh = h * 0.5
    return [
        (center[0] + dx, center[1] + dy)
        for dx, dy in (
            _rot_local(-hw, -hh, deg),
            _rot_local(hw, -hh, deg),
            _rot_local(hw, hh, deg),
            _rot_local(-hw, hh, deg),
        )
    ]


def _downsample(img: Image.Image) -> Image.Image:
    return img.resize(FRAME_SIZE, Image.Resampling.LANCZOS)


class RobotHeavyRenderer:
    def __init__(self, spec: VariantSpec):
        self.spec = spec

    def render_frame(self, anim: str, frame_idx: int, nframes: int) -> Image.Image:
        img = Image.new("RGBA", (WORK_FRAME_SIZE[0] * SUPER, WORK_FRAME_SIZE[1] * SUPER), (0, 0, 0, 0))
        draw = ImageDraw.Draw(img, "RGBA")
        pose = Pose(anim, frame_idx, nframes)
        spec = self.spec

        root = (
            WORK_FRAME_SIZE[0] * 0.47 + pose.root_x,
            WORK_FRAME_SIZE[1] * 0.76 + pose.root_y + pose.bob,
        )
        global_tilt = pose.lean

        def P(x: float, y: float) -> Point:
            rx, ry = _rot_local(x, y, global_tilt)
            return (root[0] + rx, root[1] + ry)

        # No baked ground drop shadow; the scene renderer owns contact shadows.
        self._draw_backpack(draw, P, pose)
        self._draw_back_leg(draw, P, pose)
        self._draw_back_arm(draw, P, pose)
        self._draw_torso(draw, P, pose)
        self._draw_front_leg(draw, P, pose)
        self._draw_head(draw, P, pose)
        self._draw_front_arm_and_weapon(draw, P, pose)

        if anim == "smash" and pose.impact > 0.18:
            self._draw_smash_fx(draw, P, pose)
        return _downsample(img)

    def _draw_shadow(self, draw: ImageDraw.ImageDraw, P, pose: Pose) -> None:
        c = P(0, 15)
        _ellipse(draw, c[0], c[1], 56 + abs(pose.lean) * 0.2, 12 - pose.bob * 0.08, GROUND_SHADOW, outline=(0, 0, 0, 0), width=0)

    def _draw_backpack(self, draw: ImageDraw.ImageDraw, P, pose: Pose) -> None:
        spec = self.spec
        sx = pose.backpack_sway
        if spec.backpack_style == "stacks":
            left = _rect_poly(P(-26 + sx * 0.2, -122), 12, 38, -6)
            right = _rect_poly(P(26 + sx * 0.2, -122), 12, 38, 6)
            _poly(draw, left, spec.plate_dark, OUTLINE, 1.1)
            _poly(draw, right, spec.plate_dark, OUTLINE, 1.1)
            for dx in (-26, 26):
                _line(draw, [P(dx, -136), P(dx, -154 - pose.vent_burst * 10)], spec.glow_hot, 1.0)
                smoke = P(dx + sx * 0.3, -158 - pose.vent_burst * 10)
                _ellipse(draw, smoke[0], smoke[1], 6 + pose.vent_burst * 8, 8 + pose.vent_burst * 10, SMOKE, outline=(0, 0, 0, 0), width=0)
        elif spec.backpack_style == "furnace":
            box = [P(-24, -134), P(24, -134), P(28, -92), P(-28, -92)]
            _poly(draw, box, spec.plate_dark, OUTLINE, 1.2)
            vent = [P(-12, -124), P(12, -124), P(10, -102), P(-10, -102)]
            _poly(draw, vent, spec.glow, OUTLINE, 1.0)
            _line(draw, [P(-18, -112), P(18, -112)], spec.glow_hot, 1.0)
            smoke = P(0, -148 - pose.vent_burst * 12)
            _ellipse(draw, smoke[0], smoke[1], 10 + pose.vent_burst * 9, 11 + pose.vent_burst * 12, SMOKE, outline=(0, 0, 0, 0), width=0)
        else:  # coil
            frame = [P(-28, -134), P(28, -134), P(28, -108), P(-28, -108)]
            _poly(draw, frame, spec.plate_dark, OUTLINE, 1.1)
            for dx in (-18, 0, 18):
                _line(draw, [P(dx, -132), P(dx, -110)], spec.glow, 1.2)
            arc_box = (_s(P(0, -121)[0] - 28), _s(P(0, -121)[1] - 18), _s(P(0, -121)[0] + 28), _s(P(0, -121)[1] + 18))
            draw.arc(arc_box, 200, 340, fill=(*spec.glow[:3], 150), width=_s(2.5))
            draw.arc(arc_box, 20, 160, fill=(*spec.glow[:3], 120), width=_s(2.0))

    def _draw_leg(self, draw: ImageDraw.ImageDraw, hip: Point, knee: Point, ankle: Point, foot_dir: int, front: bool) -> None:
        spec = self.spec
        line_w = (7.6 if front else 7.0) * spec.leg_scale
        plate_fill = spec.shell_mid if front else spec.shell_dark
        _line(draw, [hip, knee], spec.metal if hasattr(spec, 'metal') else STEEL_DARK, line_w)
        _line(draw, [knee, ankle], STEEL_DARK, line_w - 0.6)
        _line(draw, [hip, knee, ankle], OUTLINE, 2.0)
        _ellipse(draw, knee[0], knee[1], 8 * spec.leg_scale, 9 * spec.leg_scale, spec.plate, OUTLINE, 1.0)
        shin = _rect_poly(((knee[0] + ankle[0]) * 0.5, (knee[1] + ankle[1]) * 0.5), 16 * spec.leg_scale, 34 * spec.leg_scale, foot_dir * 4.0)
        _poly(draw, shin, plate_fill, OUTLINE, 1.1)
        foot = [
            (ankle[0] - 10 * spec.foot_scale, ankle[1] - 6),
            (ankle[0] + 16 * foot_dir * spec.foot_scale, ankle[1] - 7),
            (ankle[0] + 19 * foot_dir * spec.foot_scale, ankle[1] + 4),
            (ankle[0] - 9 * spec.foot_scale, ankle[1] + 6),
        ]
        _poly(draw, foot, spec.shell_dark if front else spec.plate_dark, OUTLINE, 1.2)
        _line(draw, [(ankle[0] - 4, ankle[1] - 4), (ankle[0] + 8 * foot_dir, ankle[1] - 4)], spec.accent, 0.8)

    def _draw_back_leg(self, draw: ImageDraw.ImageDraw, P, pose: Pose) -> None:
        hip = P(-22, -40)
        knee = P(-26 + pose.back_leg * 0.18, -6)
        ankle = P(-31 + pose.back_leg * 0.14, 12 - pose.back_foot_lift)
        self._draw_leg(draw, hip, knee, ankle, -1, front=False)

    def _draw_front_leg(self, draw: ImageDraw.ImageDraw, P, pose: Pose) -> None:
        hip = P(22, -38)
        knee = P(28 + pose.front_leg * 0.18, -4)
        ankle = P(34 + pose.front_leg * 0.14, 14 - pose.front_foot_lift)
        self._draw_leg(draw, hip, knee, ankle, 1, front=True)

    def _draw_torso(self, draw: ImageDraw.ImageDraw, P, pose: Pose) -> None:
        spec = self.spec
        sw = 52.0 * spec.shoulder_scale
        bw = 44.0 * spec.body_scale
        top_y = -122.0
        chest = [P(-sw, top_y), P(-42, -154), P(42, -154), P(sw, top_y), P(48, -68), P(0, -28), P(-48, -68)]
        _poly(draw, chest, spec.shell_dark, OUTLINE, 1.8)
        shoulder_bar = [P(-sw + 4, -128), P(sw - 4, -128), P(sw - 10, -108), P(-sw + 10, -108)]
        _poly(draw, shoulder_bar, spec.plate_dark, OUTLINE, 1.2)
        belly = [P(-bw, -118), P(-34, -142), P(34, -142), P(bw, -118), P(40, -70), P(0, -36), P(-40, -70)]
        _poly(draw, belly, spec.shell_mid, OUTLINE, 1.5)
        center = [P(-26, -118), P(-18, -134), P(18, -134), P(26, -118), P(22, -78), P(0, -54), P(-22, -78)]
        _poly(draw, center, spec.plate, OUTLINE, 1.2)
        _line(draw, [P(0, -134), P(0, -54)], spec.plate_dark, 1.0)
        _line(draw, [P(-18, -108), P(-6, -92)], spec.shell_light, 0.8)
        _line(draw, [P(18, -108), P(6, -92)], spec.shell_light, 0.8)
        if spec.chest_window:
            core = [P(-14, -103), P(14, -103), P(14, -82), P(-14, -82)]
            _poly(draw, core, spec.glow, OUTLINE, 1.0)
            _line(draw, [P(-12, -92), P(12, -92)], spec.glow_hot, 0.8)
        else:
            core = [P(-10, -100), P(0, -110), P(10, -100), P(0, -86)]
            _poly(draw, core, spec.accent, OUTLINE, 0.8)
        hips = [P(-50, -38), P(50, -38), P(44, -16), P(-44, -16)]
        _poly(draw, hips, spec.accent, OUTLINE, 1.2)
        if spec.belt_fins:
            _poly(draw, [P(-48, -38), P(-62, -24), P(-46, -20)], spec.plate_dark, OUTLINE, 0.8)
            _poly(draw, [P(48, -38), P(62, -24), P(46, -20)], spec.plate_dark, OUTLINE, 0.8)
        if spec.shoulder_pods:
            _poly(draw, [P(-sw - 6, -136), P(-sw + 12, -152), P(-sw + 24, -126), P(-sw + 4, -116)], spec.plate, OUTLINE, 1.0)
            _poly(draw, [P(sw - 24, -126), P(sw - 12, -152), P(sw + 6, -136), P(sw - 4, -116)], spec.plate, OUTLINE, 1.0)

    def _draw_arm(self, draw: ImageDraw.ImageDraw, shoulder: Point, elbow: Point, hand: Point, front: bool) -> None:
        spec = self.spec
        metal = STEEL if front else STEEL_DARK
        shell = spec.plate if front else spec.plate_dark
        arm_w = (9.5 if front else 8.5) * spec.arm_scale
        _line(draw, [shoulder, elbow], metal, arm_w)
        _line(draw, [elbow, hand], metal, arm_w - 0.8)
        _line(draw, [shoulder, elbow, hand], OUTLINE, 2.0)
        _ellipse(draw, elbow[0], elbow[1], 8.5 * spec.arm_scale, 9.0 * spec.arm_scale, shell, OUTLINE, 1.0)
        fore = _rect_poly(((elbow[0] + hand[0]) * 0.5, (elbow[1] + hand[1]) * 0.5), 17 * spec.arm_scale, 30 * spec.arm_scale, 6 if front else -8)
        _poly(draw, fore, spec.shell_mid if front else spec.shell_dark, OUTLINE, 1.0)
        _circle(draw, hand, 6.8 * spec.arm_scale, spec.rust, OUTLINE, 1.0)

    def _draw_back_arm(self, draw: ImageDraw.ImageDraw, P, pose: Pose) -> None:
        shoulder = P(-50 * self.spec.shoulder_scale, -118)
        elbow = P(-62 + pose.back_arm * 0.16, -90 + pose.back_arm * 0.16)
        hand = P(-44 + pose.back_arm * 0.18, -56 + pose.back_arm * 0.14)
        self._draw_arm(draw, shoulder, elbow, hand, front=False)

    def _draw_front_arm_and_weapon(self, draw: ImageDraw.ImageDraw, P, pose: Pose) -> None:
        shoulder = P(50 * self.spec.shoulder_scale, -116)
        elbow = P(60 + pose.front_arm * 0.10, -90 + pose.front_arm * 0.18 + pose.weapon_shift_y * 0.12)
        hand = P(32 + pose.front_arm * 0.23 + pose.weapon_shift_x, -104 + pose.weapon_shift_y)
        self._draw_arm(draw, shoulder, elbow, hand, front=True)
        self._draw_weapon(draw, hand, pose.weapon_angle + pose.lean)

    def _draw_weapon(self, draw: ImageDraw.ImageDraw, hand: Point, angle: float) -> None:
        spec = self.spec
        scale = spec.weapon_scale

        def tr(x: float, y: float) -> Point:
            rx, ry = _rot_local(x * scale, y * scale, angle)
            return (hand[0] + rx, hand[1] + ry)

        _line(draw, [tr(-8, 6), tr(62, 0), tr(96, 0)], spec.cable, 7.0 * scale)
        _line(draw, [tr(-8, 6), tr(62, 0), tr(96, 0)], OUTLINE, 2.0)
        _line(draw, [tr(2, 5), tr(26, 2)], spec.plate_dark, 1.0)
        _line(draw, [tr(16, 4), tr(42, 1)], spec.plate_dark, 1.0)
        if spec.weapon_style == "maul":
            head = _rect_poly(tr(110, 0), 54 * scale, 22 * scale, angle)
            _poly(draw, head, spec.plate_dark, OUTLINE, 1.3)
            inner = _rect_poly(tr(110, 0), 44 * scale, 14 * scale, angle)
            _poly(draw, inner, STEEL, OUTLINE, 0.9)
            for x in (88, 100, 112, 124):
                spike = [tr(x, -10), tr(x + 4, -22), tr(x + 8, -10)]
                _poly(draw, spike, STEEL, OUTLINE, 0.7)
                spike2 = [tr(x, 10), tr(x + 4, 22), tr(x + 8, 10)]
                _poly(draw, spike2, STEEL, OUTLINE, 0.7)
        elif spec.weapon_style == "pile":
            head = [tr(92, -14), tr(126, -14), tr(138, 0), tr(126, 14), tr(92, 14), tr(104, 0)]
            _poly(draw, head, spec.plate_dark, OUTLINE, 1.2)
            piston = [tr(82, -8), tr(102, -8), tr(112, 0), tr(102, 8), tr(82, 8)]
            _poly(draw, piston, spec.glow, OUTLINE, 1.0)
            tip = [tr(138, 0), tr(160, -5), tr(170, 0), tr(160, 5)]
            _poly(draw, tip, STEEL, OUTLINE, 0.8)
        else:  # arc
            head = _rect_poly(tr(108, 0), 42 * scale, 18 * scale, angle)
            _poly(draw, head, spec.plate_dark, OUTLINE, 1.2)
            coil_l = [tr(96, -10), tr(88, -28), tr(76, -10)]
            coil_r = [tr(96, 10), tr(88, 28), tr(76, 10)]
            _poly(draw, coil_l, spec.accent, OUTLINE, 0.8)
            _poly(draw, coil_r, spec.accent, OUTLINE, 0.8)
            arc_box = (_s(tr(118, 0)[0] - 18), _s(tr(118, 0)[1] - 22), _s(tr(118, 0)[0] + 18), _s(tr(118, 0)[1] + 22))
            draw.arc(arc_box, 250, 110, fill=(*spec.glow[:3], 180), width=_s(2.2))
            draw.arc(arc_box, 70, 290, fill=(*spec.glow_hot[:3], 120), width=_s(1.6))

    def _draw_head(self, draw: ImageDraw.ImageDraw, P, pose: Pose) -> None:
        spec = self.spec
        hx, hy = P(0, -172 + pose.head_tilt * 0.18)
        if spec.head_style == "mono":
            head = [(hx - 32, hy - 12), (hx - 22, hy - 34), (hx + 24, hy - 34), (hx + 34, hy - 8), (hx + 24, hy + 18), (hx - 24, hy + 18), (hx - 34, hy - 4)]
            _poly(draw, head, spec.shell_mid, OUTLINE, 1.4)
            brow = [(hx - 22, hy - 8), (hx + 24, hy - 8), (hx + 18, hy + 2), (hx - 18, hy + 2)]
            _poly(draw, brow, spec.plate_dark, OUTLINE, 1.0)
            if pose.x_eyes:
                _line(draw, [(hx - 5, hy - 3), (hx + 5, hy + 7)], spec.glow_hot, 1.2)
                _line(draw, [(hx - 5, hy + 7), (hx + 5, hy - 3)], spec.glow_hot, 1.2)
            elif pose.blink:
                _line(draw, [(hx - 9, hy + 1), (hx + 9, hy + 1)], spec.glow_hot, 1.2)
            else:
                _ellipse(draw, hx + 3, hy + 2, 9, 5, spec.glow, spec.glow_hot, 0.8)
        elif spec.head_style == "visor":
            head = [(hx - 30, hy - 10), (hx - 18, hy - 32), (hx + 20, hy - 32), (hx + 32, hy - 10), (hx + 22, hy + 20), (hx - 22, hy + 20)]
            _poly(draw, head, spec.shell_mid, OUTLINE, 1.4)
            visor = [(hx - 18, hy - 4), (hx + 18, hy - 4), (hx + 14, hy + 8), (hx - 14, hy + 8)]
            _poly(draw, visor, spec.glow if not pose.blink else spec.glow_hot, OUTLINE, 1.0)
            _line(draw, [(hx - 16, hy + 1), (hx + 16, hy + 1)], spec.glow_hot, 0.8)
        else:
            head = [(hx - 28, hy - 10), (hx - 18, hy - 34), (hx + 18, hy - 34), (hx + 28, hy - 10), (hx + 24, hy + 18), (hx - 24, hy + 18)]
            _poly(draw, head, spec.shell_mid, OUTLINE, 1.4)
            if pose.x_eyes:
                for ex in (-9, 9):
                    _line(draw, [(hx + ex - 4, hy - 2), (hx + ex + 4, hy + 6)], spec.glow_hot, 1.0)
                    _line(draw, [(hx + ex - 4, hy + 6), (hx + ex + 4, hy - 2)], spec.glow_hot, 1.0)
            else:
                if pose.blink:
                    _line(draw, [(hx - 15, hy + 1), (hx - 3, hy + 1)], spec.glow_hot, 1.0)
                    _line(draw, [(hx + 3, hy + 1), (hx + 15, hy + 1)], spec.glow_hot, 1.0)
                else:
                    _poly(draw, [(hx - 17, hy - 2), (hx - 2, hy - 2), (hx - 4, hy + 6), (hx - 16, hy + 5)], spec.glow, spec.glow_hot, 0.6)
                    _poly(draw, [(hx + 2, hy - 2), (hx + 17, hy - 2), (hx + 16, hy + 5), (hx + 4, hy + 6)], spec.glow, spec.glow_hot, 0.6)
        jaw = [(hx - 16, hy + 16), (hx + 16, hy + 16), (hx + 12, hy + 28), (hx - 12, hy + 28)]
        _poly(draw, jaw, spec.plate, OUTLINE, 1.0)
        if self.spec.antenna:
            _line(draw, [(hx + 12, hy - 28), (hx + 20, hy - 44)], STEEL, 1.2)
            _circle(draw, (hx + 21, hy - 46), 4.0, spec.glow, OUTLINE, 0.8)

    def _draw_smash_fx(self, draw: ImageDraw.ImageDraw, P, pose: Pose) -> None:
        spec = self.spec
        cx, cy = P(36, -84)
        box = (_s(cx - 102), _s(cy - 94), _s(cx + 96), _s(cy + 94))
        draw.arc(box, 204, 338, fill=(*spec.glow[:3], 130), width=_s(6.0 + pose.impact * 2.0))
        draw.arc(box, 220, 324, fill=(255, 255, 255, 98), width=_s(2.0))
        ground = P(58, 12)
        _ellipse(draw, ground[0], ground[1], 28 + pose.impact * 12, 6 + pose.impact * 2, (*spec.glow[:3], 70), outline=(0, 0, 0, 0), width=0)
        for dx in (-18, -4, 10, 22):
            spark = [P(52 + dx, 6), P(58 + dx, -4), P(66 + dx, 4)]
            _poly(draw, spark, spec.glow_hot, spec.glow, 0.5)


def render(out_dir: str | Path, variant: str = "bastion", **opts) -> List[Path]:
    out_dir = Path(out_dir)
    out_dir.mkdir(parents=True, exist_ok=True)
    if variant not in VARIANTS:
        raise ValueError(f"unknown variant {variant!r}; choices: {', '.join(sorted(VARIANTS))}")
    spec = VARIANTS[variant]
    renderer = RobotHeavyRenderer(spec)
    outputs = build_sheet(
        target=spec.target_name,
        rows=ROWS,
        render_fn=renderer.render_frame,
        out_dir=out_dir,
        frame_size=FRAME_SIZE,
        crop_margin=3,
        auto_crop=True,
    )
    return [
        outputs["spritesheet"], outputs["yaml"], outputs["ron"],
        outputs["preview"], outputs["canonical"], outputs["canonical_transparent"],
    ]


def render_many(out_dir: str | Path, variants: Iterable[str]) -> Dict[str, List[Path]]:
    results: Dict[str, List[Path]] = {}
    for variant in variants:
        results[variant] = render(out_dir, variant=variant)
    return results


def main(argv: List[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="Render one or more heavy robot sprite variants.")
    parser.add_argument("--variant", choices=sorted(list(VARIANTS.keys()) + ["all"]), default="bastion")
    parser.add_argument("--out-dir", type=Path, default=Path(__file__).resolve().parents[2] / "generated" / TARGET_BASENAME)
    args = parser.parse_args(argv)

    if args.variant == "all":
        results = render_many(args.out_dir, sorted(VARIANTS))
        for key in sorted(results):
            for path in results[key]:
                print(path)
    else:
        for path in render(args.out_dir, variant=args.variant):
            print(path)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
