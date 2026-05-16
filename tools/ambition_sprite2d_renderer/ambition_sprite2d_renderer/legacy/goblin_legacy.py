#!/usr/bin/env python3
from __future__ import annotations

"""
Standalone procedural graphics generator.

Current target: goblin
Current outputs:
- single render
- contact sheet
- sprite sheet + JSON manifest

This version improves visual quality, adds more animations, and lets goblins
hold configurable items such as swords and guns.
"""

import argparse
import json
import math
import os
import random
from dataclasses import asdict, dataclass
from typing import Dict, List, Optional, Sequence, Tuple

from PIL import Image, ImageColor, ImageDraw, ImageFilter, ImageFont

from ambition_sprite2d_renderer.console import print_path

try:
    RESAMPLING = Image.Resampling
except AttributeError:  # pragma: no cover
    RESAMPLING = Image

Color = Tuple[int, int, int, int]
Point = Tuple[float, float]


def clamp(v: float, lo: float, hi: float) -> float:
    return max(lo, min(hi, v))


def lerp(a: float, b: float, t: float) -> float:
    return a + (b - a) * t


def smoothstep(t: float) -> float:
    t = clamp(t, 0.0, 1.0)
    return t * t * (3.0 - 2.0 * t)


def pingpong(t: float) -> float:
    return t * 2.0 if t <= 0.5 else (1.0 - t) * 2.0


def rgb(hex_color: str, alpha: int = 255) -> Color:
    r, g, b = ImageColor.getrgb(hex_color)
    return (r, g, b, alpha)


def lighten(color: Color, amount: float) -> Color:
    r, g, b, a = color
    return (
        int(clamp(lerp(r, 255, amount), 0, 255)),
        int(clamp(lerp(g, 255, amount), 0, 255)),
        int(clamp(lerp(b, 255, amount), 0, 255)),
        a,
    )


def darken(color: Color, amount: float) -> Color:
    r, g, b, a = color
    return (
        int(clamp(lerp(r, 0, amount), 0, 255)),
        int(clamp(lerp(g, 0, amount), 0, 255)),
        int(clamp(lerp(b, 0, amount), 0, 255)),
        a,
    )


def with_alpha(color: Color, alpha: int) -> Color:
    return (color[0], color[1], color[2], alpha)


def rounded(draw: ImageDraw.ImageDraw, box: Sequence[float], radius: float, fill: Color, outline: Optional[Color] = None, width: int = 1) -> None:
    draw.rounded_rectangle(box, radius=radius, fill=fill, outline=outline, width=width)


def ellipse(draw: ImageDraw.ImageDraw, box: Sequence[float], fill: Color, outline: Optional[Color] = None, width: int = 1) -> None:
    draw.ellipse(box, fill=fill, outline=outline, width=width)


def polygon(draw: ImageDraw.ImageDraw, pts: Sequence[Point], fill: Color, outline: Optional[Color] = None) -> None:
    draw.polygon(list(pts), fill=fill, outline=outline)


def line(draw: ImageDraw.ImageDraw, pts: Sequence[Point], fill: Color, width: int = 1) -> None:
    draw.line(list(pts), fill=fill, width=width, joint="curve")


def parse_csv(text: str) -> List[str]:
    return [part.strip() for part in text.split(",") if part.strip()]


@dataclass
class Palette:
    skin: Color
    skin_dark: Color
    skin_light: Color
    tunic: Color
    tunic_dark: Color
    accent: Color
    leather: Color
    eyes: Color
    teeth: Color
    outline: Color
    shadow: Color
    metal: Color
    metal_dark: Color
    muzzle: Color


@dataclass
class GoblinSpec:
    target: str
    seed: int
    archetype: str
    facing: int
    stance: str
    mood: str
    palette_name: str
    head_w: int
    head_h: int
    torso_w: int
    torso_h: int
    ear_len: int
    ear_drop: int
    nose_len: int
    arm_len: int
    leg_len: int
    foot_w: int
    eye_size: int
    brow_slant: int
    tooth_count: int
    has_hood: bool
    has_belt: bool
    has_pads: bool
    scar: bool
    underbite: bool
    held_item: str


@dataclass
class FramePose:
    body_bob: float = 0.0
    body_sway_x: float = 0.0
    crouch: float = 0.0
    torso_lean_x: float = 0.0
    shoulder_tilt: float = 0.0
    hip_tilt: float = 0.0
    squash_x: float = 0.0
    stretch_y: float = 0.0
    torso_twist: float = 0.0
    head_offset_y: float = 0.0
    head_lean_x: float = 0.0
    head_tilt: float = 0.0
    front_arm_dx: float = 0.0
    front_arm_dy: float = 0.0
    rear_arm_dx: float = 0.0
    rear_arm_dy: float = 0.0
    front_arm_bend: float = 0.0
    rear_arm_bend: float = 0.0
    front_leg_dx: float = 0.0
    rear_leg_dx: float = 0.0
    front_leg_lift: float = 0.0
    rear_leg_lift: float = 0.0
    front_knee_bend: float = 0.0
    rear_knee_bend: float = 0.0
    knee_bend: float = 0.0
    mouth_open: float = 0.0
    eye_squint: float = 0.0
    item_raise: float = 0.0
    item_lag: float = 0.0
    recoil: float = 0.0
    flash: bool = False
    dizzy: bool = False
    death_progress: float = 0.0
    in_air: bool = False
    falling: bool = False


@dataclass
class FaceLayout:
    near_eye_x: float
    far_eye_x: float
    eye_y: float
    near_eye_rx: float
    far_eye_rx: float
    eye_ry: float
    brow_near_a: Point
    brow_near_b: Point
    brow_far_a: Point
    brow_far_b: Point
    nose_pts: Tuple[Point, Point, Point]
    mouth_left: Point
    mouth_right: Point
    tooth_left: float
    tooth_right: float
    scar_a: Optional[Point]
    scar_b: Optional[Point]


class GoblinTarget:
    name = "goblin"

    ARCHETYPES = {
        "default": {
            "head": (44, 54),
            "torso": (42, 50),
            "ears": (20, 8),
            "arm": (28, 34),
            "leg": (22, 28),
            "moods": ["snarl", "smirk", "alert"],
            "items": ["sword", "club", "gun", "none"],
        },
        "small_skitter": {
            "head": (38, 48),
            "torso": (32, 38),
            "ears": (24, 10),
            "arm": (22, 28),
            "leg": (18, 24),
            "moods": ["sneaky", "snarl", "alert"],
            "items": ["knife", "bomb", "none", "gun"],
        },
        "medium_striker": {
            "head": (46, 56),
            "torso": (40, 52),
            "ears": (18, 8),
            "arm": (28, 34),
            "leg": (22, 28),
            "moods": ["snarl", "smirk", "focused"],
            "items": ["sword", "spear", "gun", "shield"],
        },
        "large_brute": {
            "head": (54, 62),
            "torso": (54, 64),
            "ears": (16, 6),
            "arm": (34, 42),
            "leg": (26, 34),
            "moods": ["glare", "snarl", "angry"],
            "items": ["club", "shield", "spear", "none"],
        },
        "gradient_seeker": {
            "head": (44, 54),
            "torso": (40, 48),
            "ears": (22, 8),
            "arm": (26, 32),
            "leg": (20, 28),
            "moods": ["alert", "focused", "eerily_calm"],
            "items": ["staff", "gun", "bomb", "none"],
        },
    }
    STANCES = ["idle", "lean", "crouch", "lunge"]
    HELD_ITEMS = ["none", "knife", "sword", "club", "spear", "staff", "gun", "shield", "bomb"]
    SPRITESHEET_ANIMATIONS = {
        "idle": {"frames": 4, "duration_ms": 140},
        "walk": {"frames": 6, "duration_ms": 100},
        "attack": {"frames": 6, "duration_ms": 80},
        "hurt": {"frames": 3, "duration_ms": 100},
        "jump": {"frames": 4, "duration_ms": 90},
        "fall": {"frames": 2, "duration_ms": 110},
        "stun": {"frames": 4, "duration_ms": 120},
        "death": {"frames": 6, "duration_ms": 110},
    }
    PALETTES = {
        "moss": Palette(
            rgb("#73A942"), rgb("#446A22"), rgb("#A8D673"), rgb("#6B4F3A"), rgb("#4A3425"),
            rgb("#C99835"), rgb("#8A5A34"), rgb("#F7E96E"), rgb("#F6F2D8"), rgb("#1B160F"),
            rgb("#0A0A12", 120), rgb("#B9C2CF"), rgb("#7A8596"), rgb("#FFD9A2")
        ),
        "bog": Palette(
            rgb("#6C8E3B"), rgb("#3D5B26"), rgb("#9EBF6B"), rgb("#5A4455"), rgb("#37263B"),
            rgb("#C56B2D"), rgb("#7A4E29"), rgb("#E9FF76"), rgb("#F3F0DB"), rgb("#171310"),
            rgb("#0A0A12", 120), rgb("#ADB6C2"), rgb("#73808F"), rgb("#FFD09B")
        ),
        "ember": Palette(
            rgb("#89B640"), rgb("#506C22"), rgb("#B7DB74"), rgb("#7A2E2B"), rgb("#4D1B19"),
            rgb("#E2933A"), rgb("#8A5C2F"), rgb("#FFD76A"), rgb("#F6F2D8"), rgb("#1A130E"),
            rgb("#0A0A12", 120), rgb("#D1D7E0"), rgb("#7D8694"), rgb("#FFC18C")
        ),
        "gradient": Palette(
            rgb("#76C96D"), rgb("#377945"), rgb("#B6F1B0"), rgb("#33415C"), rgb("#1C2438"),
            rgb("#57CC99"), rgb("#4B5E7A"), rgb("#86F7FF"), rgb("#F1FFFB"), rgb("#10131A"),
            rgb("#080A14", 120), rgb("#CAE8FF"), rgb("#6EAAC7"), rgb("#A9FBFF")
        ),
    }

    def sample_spec(self, seed: int, archetype: str = "default", held_item_override: Optional[str] = None) -> GoblinSpec:
        rng = random.Random(seed)
        arch = self.ARCHETYPES.get(archetype, self.ARCHETYPES["default"])
        palette_name = "gradient" if archetype == "gradient_seeker" else rng.choice(["moss", "bog", "ember"])
        if archetype == "default":
            palette_name = rng.choice(list(self.PALETTES.keys()))
        if held_item_override is not None:
            held_item = held_item_override
        else:
            held_item = rng.choice(arch["items"])
        return GoblinSpec(
            target="goblin",
            seed=seed,
            archetype=archetype,
            facing=rng.choice([-1, 1]),
            stance=rng.choice(self.STANCES),
            mood=rng.choice(arch["moods"]),
            palette_name=palette_name,
            head_w=rng.randint(arch["head"][0] - 3, arch["head"][1]),
            head_h=rng.randint(arch["head"][0], arch["head"][1] + 4),
            torso_w=rng.randint(arch["torso"][0] - 2, arch["torso"][1]),
            torso_h=rng.randint(arch["torso"][0], arch["torso"][1] + 4),
            ear_len=rng.randint(max(10, arch["ears"][0] - 4), arch["ears"][0] + 6),
            ear_drop=rng.randint(arch["ears"][1] - 3, arch["ears"][1] + 6),
            nose_len=rng.randint(8, 16),
            arm_len=rng.randint(arch["arm"][0] - 3, arch["arm"][1]),
            leg_len=rng.randint(arch["leg"][0] - 3, arch["leg"][1]),
            foot_w=rng.randint(12, 20),
            eye_size=rng.randint(4, 8 if archetype != "large_brute" else 7),
            brow_slant=rng.randint(-5, 6),
            tooth_count=rng.randint(1, 4 if archetype == "small_skitter" else 6),
            has_hood=rng.random() < (0.35 if archetype != "large_brute" else 0.15),
            has_belt=rng.random() < 0.75,
            has_pads=rng.random() < (0.55 if archetype in {"medium_striker", "large_brute"} else 0.25),
            scar=rng.random() < 0.28,
            underbite=rng.random() < 0.45,
            held_item=held_item,
        )

    def animation_pose(self, spec: GoblinSpec, animation: str, frame_index: int, frame_count: int) -> FramePose:
        phase = 0.0 if frame_count <= 1 else frame_index / float(max(1, frame_count - 1))
        wave = math.sin(phase * math.tau)
        wave_cos = math.cos(phase * math.tau)
        p = FramePose()
        if animation == "idle":
            p.body_bob = wave * 1.4
            p.body_sway_x = wave_cos * 1.1
            p.head_offset_y = -abs(wave) * 0.7
            p.head_tilt = wave * 1.2
            p.torso_twist = wave * 0.22
            p.shoulder_tilt = -wave * 2.0
            p.hip_tilt = wave * 1.2
            p.front_arm_dx = wave * 1.3
            p.rear_arm_dx = -wave * 1.1
            p.front_arm_bend = 1.5
            p.rear_arm_bend = -1.2
            p.front_leg_lift = max(0.0, wave) * 1.0
            p.rear_leg_lift = max(0.0, -wave) * 1.0
            p.front_knee_bend = 1.4
            p.rear_knee_bend = 1.0
            p.mouth_open = 0.08
            p.knee_bend = 1.0
        elif animation == "walk":
            stride = wave
            p.body_bob = abs(stride) * 2.5
            p.body_sway_x = wave_cos * 2.2
            p.torso_twist = stride * 0.42
            p.shoulder_tilt = -stride * 4.0
            p.hip_tilt = stride * 3.0
            p.squash_x = 0.03 * abs(stride)
            p.stretch_y = -0.04 * abs(stride)
            p.front_leg_dx = stride * 8.0
            p.rear_leg_dx = -stride * 8.0
            p.front_leg_lift = max(0.0, -stride) * 5.0
            p.rear_leg_lift = max(0.0, stride) * 5.0
            p.front_knee_bend = 4.5 + max(0.0, -stride) * 4.0
            p.rear_knee_bend = 3.5 + max(0.0, stride) * 3.5
            p.front_arm_dx = -stride * 9.0
            p.rear_arm_dx = stride * 9.0
            p.front_arm_dy = abs(stride) * 2.0
            p.rear_arm_dy = abs(stride) * 1.2
            p.front_arm_bend = -3.0 * stride
            p.rear_arm_bend = 2.5 * stride
            p.knee_bend = 2.0
            p.head_lean_x = stride * 1.8
            p.head_tilt = -stride * 1.2
            p.item_lag = -stride * 2.5
        elif animation == "attack":
            windup = 1.0 - smoothstep(clamp(phase / 0.35, 0.0, 1.0))
            strike = smoothstep(clamp((phase - 0.35) / 0.40, 0.0, 1.0))
            recover = smoothstep(clamp((phase - 0.75) / 0.25, 0.0, 1.0))
            p.torso_lean_x = -9.0 * windup + 12.0 * strike - 5.0 * recover
            p.body_sway_x = -2.0 * windup + 3.0 * strike
            p.torso_twist = -0.45 * windup + 0.65 * strike - 0.15 * recover
            p.shoulder_tilt = 5.0 * windup - 3.0 * strike
            p.hip_tilt = -4.0 * windup + 4.0 * strike
            p.recoil = 8.0 * windup - 5.0 * strike
            p.front_arm_dx = -11.0 * windup + 20.0 * strike - 5.0 * recover
            p.front_arm_dy = -7.0 * windup - 11.0 * strike + 5.0 * recover
            p.rear_arm_dx = -4.5 * windup + 5.0 * strike
            p.rear_arm_dy = 3.0 * windup - 2.0 * strike
            p.front_arm_bend = 8.0 * windup - 6.0 * strike
            p.rear_arm_bend = -4.0 * windup
            p.item_raise = 18.0 * windup - 12.0 * strike
            p.item_lag = 8.0 * windup - 6.0 * strike
            p.front_leg_dx = 8.0 * strike
            p.rear_leg_dx = -3.0 * strike
            p.front_leg_lift = 2.0 * windup
            p.front_knee_bend = 4.5
            p.rear_knee_bend = 2.0
            p.squash_x = 0.05 * windup
            p.stretch_y = -0.05 * windup
            p.mouth_open = 0.5
            p.eye_squint = 0.35
            p.head_tilt = -3.0 * windup + 1.0 * strike
        elif animation == "hurt":
            pulse = pingpong(phase)
            p.flash = frame_index in {0, 1}
            p.recoil = -10.0 * pulse
            p.torso_lean_x = -8.0 * pulse
            p.body_sway_x = -3.0 * pulse
            p.shoulder_tilt = 5.0 * pulse
            p.hip_tilt = -4.0 * pulse
            p.front_arm_dx = -8.0 * pulse
            p.rear_arm_dx = -4.0 * pulse
            p.front_arm_dy = 4.0 * pulse
            p.rear_arm_dy = 3.0 * pulse
            p.front_arm_bend = 5.0 * pulse
            p.rear_arm_bend = -2.0 * pulse
            p.crouch = 5.0 * pulse
            p.head_offset_y = 2.0 * pulse
            p.head_tilt = -4.0 * pulse
            p.squash_x = 0.05 * pulse
            p.stretch_y = -0.07 * pulse
            p.mouth_open = 0.35
            p.eye_squint = 0.55
        elif animation == "jump":
            launch = smoothstep(phase)
            p.body_bob = -12.0 * launch
            p.crouch = -4.0 * launch
            p.squash_x = 0.06 * (1.0 - launch)
            p.stretch_y = 0.10 * launch
            p.front_leg_lift = 7.0 * launch
            p.rear_leg_lift = 5.0 * launch
            p.front_knee_bend = 7.0
            p.rear_knee_bend = 6.0
            p.knee_bend = 3.0
            p.front_arm_dy = -5.0 * launch
            p.rear_arm_dy = -3.0 * launch
            p.front_arm_bend = 2.5
            p.rear_arm_bend = -1.5
            p.head_offset_y = -2.0 * launch
            p.head_tilt = wave * 0.6
            p.mouth_open = 0.15
            p.item_lag = -3.0 * launch
            p.in_air = True
        elif animation == "fall":
            p.body_bob = -6.0
            p.body_sway_x = wave * 0.8
            p.front_leg_lift = 2.0
            p.rear_leg_lift = 1.0
            p.front_knee_bend = 3.0
            p.rear_knee_bend = 2.0
            p.front_arm_dy = 2.0
            p.rear_arm_dy = 1.5
            p.front_arm_bend = -2.0
            p.rear_arm_bend = 1.0
            p.knee_bend = 1.0
            p.stretch_y = 0.03
            p.eye_squint = 0.20
            p.in_air = True
            p.falling = True
        elif animation == "stun":
            sway = math.sin(phase * math.tau)
            p.body_bob = abs(sway) * 1.0
            p.body_sway_x = sway * 2.0
            p.torso_lean_x = sway * 5.0
            p.head_lean_x = sway * 6.0
            p.head_tilt = sway * 5.0
            p.torso_twist = sway * 0.20
            p.shoulder_tilt = -sway * 4.0
            p.hip_tilt = sway * 2.5
            p.front_arm_dx = -sway * 3.0
            p.rear_arm_dx = sway * 2.0
            p.front_arm_bend = 3.0 * sway
            p.rear_arm_bend = -2.0 * sway
            p.front_leg_dx = sway * 1.5
            p.rear_leg_dx = -sway * 1.5
            p.front_knee_bend = 2.0 + abs(sway) * 1.5
            p.rear_knee_bend = 1.6 + abs(sway) * 1.0
            p.crouch = 2.0
            p.dizzy = True
            p.eye_squint = 0.30
            p.mouth_open = 0.10
            p.item_lag = -sway * 2.0
        elif animation == "death":
            d = smoothstep(phase)
            p.death_progress = d
            p.crouch = 10.0 * d
            p.body_bob = 3.0 * d
            p.body_sway_x = 8.0 * d
            p.torso_lean_x = 22.0 * d
            p.head_lean_x = 7.0 * d
            p.head_tilt = 8.0 * d
            p.shoulder_tilt = -10.0 * d
            p.hip_tilt = 8.0 * d
            p.head_offset_y = 4.0 * d
            p.torso_twist = 0.35 * d
            p.front_leg_dx = 10.0 * d
            p.rear_leg_dx = -8.0 * d
            p.front_leg_lift = 1.0 * (1.0 - d)
            p.rear_leg_lift = 3.0 * d
            p.front_knee_bend = 2.0 + 4.0 * d
            p.rear_knee_bend = 2.0 + 2.0 * d
            p.front_arm_dx = 10.0 * d
            p.front_arm_dy = 8.0 * d
            p.rear_arm_dx = -8.0 * d
            p.rear_arm_dy = 6.0 * d
            p.front_arm_bend = -6.0 * d
            p.rear_arm_bend = 5.0 * d
            p.item_raise = -8.0 * d
            p.item_lag = -5.0 * d
            p.eye_squint = d
            p.mouth_open = 0.1
        if spec.stance == "crouch":
            p.crouch += 4.0
            p.squash_x += 0.03
            p.stretch_y -= 0.04
        elif spec.stance == "lean":
            p.torso_lean_x += 2.0
            p.head_tilt += 1.0
        elif spec.stance == "lunge":
            p.front_leg_dx += 3.0
            p.front_arm_dx += 4.0
            p.torso_twist += 0.12
        return p

    def render(self, spec: GoblinSpec, size: Tuple[int, int], background: Optional[Color] = None) -> Image.Image:
        return self._render_core(spec, size, background, FramePose())

    def render_animation_frame(
        self,
        spec: GoblinSpec,
        animation: str,
        frame_index: int,
        frame_count: int,
        size: Tuple[int, int],
        background: Optional[Color] = None,
        supersample: int = 4,
        downsample: str = "nearest",
    ) -> Image.Image:
        pose = self.animation_pose(spec, animation, frame_index, frame_count)
        if supersample <= 1:
            return self._render_core(spec, size, background, pose)
        hi_img = self._render_core(spec, (size[0] * supersample, size[1] * supersample), background, pose)
        resample = RESAMPLING.NEAREST if downsample == "nearest" else RESAMPLING.LANCZOS
        return hi_img.resize(size, resample)

    def _render_core(self, spec: GoblinSpec, size: Tuple[int, int], background: Optional[Color], pose: FramePose) -> Image.Image:
        w, h = size
        img = Image.new("RGBA", (w, h), background if background is not None else (0, 0, 0, 0))
        draw = ImageDraw.Draw(img)
        pal = self.PALETTES[spec.palette_name]
        outline = lighten(pal.outline, 0.55) if pose.flash else pal.outline
        skin = lighten(pal.skin, 0.18) if pose.flash else pal.skin
        skin_dark = lighten(pal.skin_dark, 0.20) if pose.flash else pal.skin_dark
        skin_light = lighten(pal.skin_light, 0.16) if pose.flash else pal.skin_light
        tunic = lighten(pal.tunic, 0.18) if pose.flash else pal.tunic
        leather = lighten(pal.leather, 0.12) if pose.flash else pal.leather
        metal = lighten(pal.metal, 0.10) if pose.flash else pal.metal
        metal_dark = lighten(pal.metal_dark, 0.12) if pose.flash else pal.metal_dark
        eyes = lighten(pal.eyes, 0.12) if pose.flash else pal.eyes
        teeth = lighten(pal.teeth, 0.06) if pose.flash else pal.teeth
        scale = min(w, h) / 256.0
        stroke1 = max(1, int(1 * scale))
        stroke2 = max(1, int(2 * scale))
        cx = w / 2.0
        ground_y = h * 0.84
        facing = spec.facing

        base_head_w = spec.head_w * scale
        base_head_h = spec.head_h * scale
        base_torso_w = spec.torso_w * scale
        base_torso_h = spec.torso_h * scale
        ear_len = spec.ear_len * scale
        ear_drop = spec.ear_drop * scale
        arm_len = spec.arm_len * scale
        leg_len = spec.leg_len * scale
        nose_len = spec.nose_len * scale
        eye_size = max(2.0, spec.eye_size * scale)

        squash_scale = 1.0 + pose.squash_x
        stretch_scale = max(0.78, 1.0 + pose.stretch_y)
        torso_w = base_torso_w * squash_scale * (1.0 - abs(pose.torso_twist) * 0.04)
        torso_h = base_torso_h * stretch_scale
        head_w = base_head_w * (1.0 + pose.squash_x * 0.35) * (1.0 - abs(pose.torso_twist) * 0.05)
        head_h = base_head_h * (1.0 + pose.stretch_y * 0.18)

        body_bob = pose.body_bob * scale
        crouch = pose.crouch * scale
        lean_x = pose.torso_lean_x * scale * facing
        sway_x = pose.body_sway_x * scale * facing
        recoil = pose.recoil * scale * facing
        head_lean_x = pose.head_lean_x * scale * facing
        shoulder_tilt = pose.shoulder_tilt * scale
        hip_tilt = pose.hip_tilt * scale
        view_bias = 0.14
        twist_px = (pose.torso_twist + view_bias) * 8.0 * scale * facing

        leg_top_y = ground_y - leg_len - 12 * scale - body_bob + crouch
        torso_top_y = leg_top_y - torso_h + 10 * scale
        torso_bottom_y = torso_top_y + torso_h
        head_top_y = torso_top_y - head_h + 12 * scale + pose.head_offset_y * scale
        head_bottom_y = head_top_y + head_h
        torso_center_x = cx + lean_x + sway_x + recoil
        head_center_x = torso_center_x + head_lean_x + facing * 2.0 * scale
        torso_x0 = torso_center_x - torso_w / 2
        torso_x1 = torso_center_x + torso_w / 2
        head_x0 = head_center_x - head_w / 2
        head_x1 = head_center_x + head_w / 2

        rear_shoulder_x = torso_center_x - facing * max(torso_w * 0.16, torso_w * 0.24 - twist_px)
        rear_shoulder_y = torso_top_y + 13 * scale - shoulder_tilt * 0.45
        front_shoulder_x = torso_center_x + facing * (torso_w * 0.34 + twist_px)
        front_shoulder_y = torso_top_y + 12 * scale + shoulder_tilt * 0.45
        rear_hip_x = torso_center_x - facing * max(torso_w * 0.10, torso_w * 0.15 - twist_px * 0.35)
        rear_hip_y = leg_top_y - hip_tilt * 0.50
        front_hip_x = torso_center_x + facing * (torso_w * 0.20 + twist_px * 0.25)
        front_hip_y = leg_top_y + hip_tilt * 0.50

        rear_hand_x, rear_hand_y = self._draw_arm(
            draw, rear_shoulder_x, rear_shoulder_y, arm_len, facing, pose.rear_arm_dx, pose.rear_arm_dy,
            pose.rear_arm_bend, -1, skin_dark, scale, extra_raise=pose.item_raise * 0.18, hand_radius=4.0
        )

        self._draw_legs(draw, spec, skin_dark, leather, outline, front_hip_x, front_hip_y, rear_hip_x, rear_hip_y, ground_y, pose, scale)

        body_color = darken(tunic, 0.08) if spec.has_hood else tunic
        rounded(draw, (torso_x0, torso_top_y, torso_x1, torso_bottom_y), radius=11 * scale, fill=body_color, outline=outline, width=stroke2)
        chest_box = (torso_x0 + 6 * scale, torso_top_y + 6 * scale, torso_x1 - 6 * scale, torso_bottom_y - 14 * scale)
        rounded(draw, chest_box, radius=8 * scale, fill=lighten(body_color, 0.08))
        chest_mid_x = torso_center_x - facing * torso_w * 0.08 + twist_px * 0.2
        line(draw, [(chest_mid_x, chest_box[1] + 3 * scale), (chest_mid_x, chest_box[3] - 2 * scale)], fill=with_alpha(outline, 110), width=stroke1)
        hem_y = torso_bottom_y - 4 * scale
        for off in (-0.28, 0.0, 0.28):
            px = torso_center_x + torso_w * off
            polygon(draw, [(px - 7 * scale, hem_y), (px + 7 * scale, hem_y), (px, hem_y + 10 * scale)], fill=darken(body_color, 0.10), outline=outline)
        if spec.has_belt:
            belt_y = torso_top_y + torso_h * 0.60
            rounded(draw, (torso_x0 - 2 * scale, belt_y, torso_x1 + 2 * scale, belt_y + 8 * scale), radius=4 * scale, fill=leather)
            buckle_x = torso_center_x + twist_px * 0.35
            rounded(draw, (buckle_x - 5 * scale, belt_y + 1 * scale, buckle_x + 5 * scale, belt_y + 7 * scale), radius=2 * scale, fill=pal.accent)
        if spec.has_pads:
            for sgn in (-1, 1):
                local_x = torso_center_x + sgn * (torso_w * 0.29)
                py = torso_top_y + 6 * scale + (shoulder_tilt * 0.25 * sgn)
                ellipse(draw, (local_x - 9 * scale, py - 6 * scale, local_x + 9 * scale, py + 8 * scale), fill=darken(leather, 0.05), outline=outline, width=stroke2)

        ellipse(draw, (head_x0, head_top_y, head_x1, head_bottom_y), fill=skin, outline=outline, width=stroke2)
        jaw = [
            (head_center_x - head_w * 0.26, head_top_y + head_h * 0.68 + pose.head_tilt * 0.10 * scale),
            (head_center_x + head_w * 0.26, head_top_y + head_h * 0.68 - pose.head_tilt * 0.10 * scale),
            (head_center_x + head_w * 0.18, head_bottom_y + 3 * scale),
            (head_center_x - head_w * 0.18, head_bottom_y + 3 * scale),
        ]
        polygon(draw, jaw, fill=skin_dark, outline=outline)
        face_light_x0 = head_x0 + (10 - 4 * max(0, facing)) * scale
        face_light_x1 = head_x1 - (10 + 6 * max(0, -facing)) * scale
        ellipse(draw, (face_light_x0, head_top_y + 8 * scale, face_light_x1, head_bottom_y - 18 * scale), fill=skin_light)
        self._draw_ears(draw, outline, head_x0, head_x1, head_top_y + 18 * scale, ear_len, ear_drop, skin_dark, lighten(skin, 0.14), facing)
        if spec.has_hood:
            hood = darken(pal.tunic_dark, 0.05)
            rounded(draw, (head_x0 - 8 * scale, head_top_y - 4 * scale, head_x1 + 8 * scale, head_bottom_y - 2 * scale), radius=18 * scale, fill=with_alpha(hood, 220), outline=outline, width=stroke2)
            ellipse(draw, (head_x0 + 7 * scale, head_top_y + 3 * scale, head_x1 - 7 * scale, head_bottom_y - 2 * scale), fill=skin)
            self._draw_ears(draw, outline, head_x0, head_x1, head_top_y + 18 * scale, ear_len, ear_drop, skin_dark, lighten(skin, 0.14), facing)

        self._draw_face(draw, spec, outline, eyes, teeth, skin_dark, head_center_x, head_top_y + pose.head_tilt * 0.10 * scale, head_w, head_h, nose_len, eye_size, pose, scale)

        front_hand_x, front_hand_y = self._draw_arm(
            draw, front_shoulder_x, front_shoulder_y, arm_len, facing, pose.front_arm_dx, pose.front_arm_dy,
            pose.front_arm_bend, 1, skin, scale, extra_raise=pose.item_raise * 0.45, hand_radius=4.0
        )
        if spec.held_item == "none":
            self._draw_claws(draw, front_hand_x, front_hand_y, facing, teeth, scale)
        self._draw_item(draw, spec, outline, metal, metal_dark, leather, pal.accent, pal.muzzle, front_hand_x, front_hand_y + pose.item_lag * scale, facing, pose, scale)

        if pose.dizzy:
            self._draw_stun_effect(draw, outline, pal.accent, head_center_x, head_top_y - 10 * scale, scale)

        if spec.archetype == "gradient_seeker":
            glow = Image.new("RGBA", (w, h), (0, 0, 0, 0))
            gdraw = ImageDraw.Draw(glow)
            gx = head_center_x + facing * 4 * scale
            gy = head_top_y + head_h * 0.50
            ellipse(gdraw, (gx - 16 * scale, gy - 12 * scale, gx + 16 * scale, gy + 12 * scale), fill=with_alpha(eyes, 46))
            if spec.held_item in {"staff", "gun"}:
                ellipse(gdraw, (front_hand_x - 14 * scale, front_hand_y - 14 * scale, front_hand_x + 14 * scale, front_hand_y + 14 * scale), fill=with_alpha(pal.accent, 34))
            glow = glow.filter(ImageFilter.GaussianBlur(radius=max(2, int(8 * scale))))
            img.alpha_composite(glow)
        return img

    def _draw_ears(self, draw: ImageDraw.ImageDraw, outline: Color, head_x0: float, head_x1: float, ear_y: float, ear_len: float, ear_drop: float, ear_fill: Color, inner: Color, facing: int) -> None:
        for sgn in (-1, 1):
            base_x = head_x0 if sgn < 0 else head_x1
            near = 1.0 if sgn == facing else 0.72
            tip_x = base_x + sgn * ear_len * near
            tip_y = ear_y - ear_drop * (1.0 if sgn == facing else 0.82)
            low_y = ear_y + ear_drop * (0.70 if sgn == facing else 0.58)
            polygon(draw, [(base_x, ear_y - 6), (tip_x, tip_y), (base_x + sgn * 4 * near, low_y)], fill=ear_fill, outline=outline)
            polygon(draw, [(base_x + sgn * 2, ear_y - 3), (tip_x - sgn * 5 * near, tip_y + 4), (base_x + sgn * 3 * near, low_y - 2)], fill=inner)

    def _compute_face_layout(self, spec: GoblinSpec, cx: float, head_top_y: float, head_w: float, head_h: float, nose_len: float, eye_size: float, pose: FramePose, scale: float) -> FaceLayout:
        face_left = cx - head_w * 0.34
        face_right = cx + head_w * 0.34
        eye_y = head_top_y + head_h * 0.40
        eye_ry = max(3.2 * scale, eye_size * 0.82 * (1.0 - pose.eye_squint * 0.78))
        near_eye_rx = eye_size + 1.2 * scale
        far_eye_rx = max(2.2 * scale, near_eye_rx * 0.78)
        min_eye_gap = max(head_w * 0.22, near_eye_rx + far_eye_rx + 9.0 * scale)

        near_eye_x = cx + spec.facing * (head_w * 0.13)
        far_eye_x = cx - spec.facing * (head_w * 0.15)
        near_lo = face_left + near_eye_rx + 1.0 * scale
        near_hi = face_right - near_eye_rx - 1.0 * scale
        far_lo = face_left + far_eye_rx + 1.0 * scale
        far_hi = face_right - far_eye_rx - 1.0 * scale
        near_eye_x = clamp(near_eye_x, near_lo, near_hi)
        far_eye_x = clamp(far_eye_x, far_lo, far_hi)
        if spec.facing > 0 and near_eye_x - far_eye_x < min_eye_gap:
            midpoint = (near_eye_x + far_eye_x) * 0.5
            near_eye_x = clamp(midpoint + min_eye_gap * 0.5, near_lo, near_hi)
            far_eye_x = clamp(midpoint - min_eye_gap * 0.5, far_lo, far_hi)
        elif spec.facing < 0 and far_eye_x - near_eye_x < min_eye_gap:
            midpoint = (near_eye_x + far_eye_x) * 0.5
            near_eye_x = clamp(midpoint - min_eye_gap * 0.5, near_lo, near_hi)
            far_eye_x = clamp(midpoint + min_eye_gap * 0.5, far_lo, far_hi)

        brow_y = eye_y - max(5.0 * scale, eye_ry + 3.0 * scale)
        slant = spec.brow_slant * scale
        brow_far_a = (far_eye_x - far_eye_rx - 3.5 * scale, brow_y + slant * 0.45)
        brow_far_b = (far_eye_x + far_eye_rx * 0.55, brow_y + 1.0 * scale)
        brow_near_a = (near_eye_x - near_eye_rx * 0.60, brow_y + 0.5 * scale)
        brow_near_b = (near_eye_x + near_eye_rx + 3.8 * scale, brow_y - slant * 0.55)

        nose_y = head_top_y + head_h * 0.58
        nose_root_x = cx + spec.facing * (head_w * 0.10)
        nose_tip_x = nose_root_x + spec.facing * max(nose_len * 0.88, head_w * 0.13)
        nose_tip_x = clamp(nose_tip_x, face_left + 6.0 * scale, face_right - 4.0 * scale)
        nose_pts = (
            (nose_root_x - spec.facing * 2.0 * scale, nose_y - 5.0 * scale),
            (nose_tip_x, nose_y + 1.0 * scale),
            (nose_root_x + spec.facing * 1.5 * scale, nose_y + 8.5 * scale),
        )

        mouth_y = max(head_top_y + head_h * 0.76, nose_y + 12.0 * scale)
        mouth_center_x = cx + spec.facing * (head_w * 0.02)
        mouth_half = clamp(head_w * 0.15, 10.0 * scale, head_w * 0.19)
        mouth_left_x = clamp(mouth_center_x - mouth_half, cx - head_w * 0.18, cx + head_w * 0.08)
        mouth_right_x = clamp(mouth_center_x + mouth_half, cx - head_w * 0.08, cx + head_w * 0.18)
        if mouth_left_x > mouth_right_x:
            mouth_left_x, mouth_right_x = mouth_right_x, mouth_left_x
        tooth_left = mouth_left_x + 1.0 * scale
        tooth_right = mouth_right_x - 1.0 * scale

        scar_a = None
        scar_b = None
        if spec.scar:
            sx = near_eye_x - spec.facing * 2.0 * scale
            sy = eye_y - 2.0 * scale
            scar_a = (sx - 4.0 * scale, sy - 6.0 * scale)
            scar_b = (sx + 4.0 * scale, sy + 5.0 * scale)

        return FaceLayout(
            near_eye_x=near_eye_x,
            far_eye_x=far_eye_x,
            eye_y=eye_y,
            near_eye_rx=near_eye_rx,
            far_eye_rx=far_eye_rx,
            eye_ry=eye_ry,
            brow_near_a=brow_near_a,
            brow_near_b=brow_near_b,
            brow_far_a=brow_far_a,
            brow_far_b=brow_far_b,
            nose_pts=nose_pts,
            mouth_left=(mouth_left_x, mouth_y),
            mouth_right=(mouth_right_x, mouth_y),
            tooth_left=tooth_left,
            tooth_right=tooth_right,
            scar_a=scar_a,
            scar_b=scar_b,
        )

    def _draw_eye_feature(self, draw: ImageDraw.ImageDraw, cx: float, cy: float, rx: float, ry: float, outline: Color, iris: Color, scale: float, near: bool = False) -> None:
        sclera = rgb("#E6E2CC")
        pts = [
            (cx - rx, cy),
            (cx - rx * 0.62, cy - ry * 0.72),
            (cx - rx * 0.12, cy - ry * 0.98),
            (cx + rx * 0.46, cy - ry * 0.74),
            (cx + rx, cy),
            (cx + rx * 0.44, cy + ry * 0.62),
            (cx - rx * 0.10, cy + ry * 0.74),
            (cx - rx * 0.60, cy + ry * 0.56),
        ]
        polygon(draw, pts, fill=sclera, outline=outline)
        line(draw, [pts[0], pts[1], pts[2], pts[3], pts[4]], fill=outline, width=max(1, int(2 * scale)))
        iris_r = max(1.8 * scale, min(rx, ry) * (0.34 if near else 0.31))
        iris_x = cx + rx * (0.08 if near else 0.03)
        iris_y = cy + ry * 0.02
        ellipse(draw, (iris_x - iris_r, iris_y - iris_r, iris_x + iris_r, iris_y + iris_r), fill=iris, outline=outline)
        pupil_r = max(1.0 * scale, iris_r * 0.45)
        ellipse(draw, (iris_x - pupil_r, iris_y - pupil_r, iris_x + pupil_r, iris_y + pupil_r), fill=outline)
        gleam_r = max(0.9 * scale, iris_r * 0.22)
        ellipse(draw, (iris_x - gleam_r * 0.4, iris_y - gleam_r * 1.2, iris_x + gleam_r * 0.4, iris_y - gleam_r * 0.4), fill=rgb("#FFFDF7"))

    def _draw_face(self, draw: ImageDraw.ImageDraw, spec: GoblinSpec, outline: Color, eyes: Color, teeth: Color, nose_color: Color, cx: float, head_top_y: float, head_w: float, head_h: float, nose_len: float, eye_size: float, pose: FramePose, scale: float) -> None:
        layout = self._compute_face_layout(spec, cx, head_top_y, head_w, head_h, nose_len, eye_size, pose, scale)

        line(draw, [layout.brow_far_a, layout.brow_far_b], fill=outline, width=max(1, int(3 * scale)))
        line(draw, [layout.brow_near_a, layout.brow_near_b], fill=outline, width=max(1, int(3 * scale)))

        if pose.death_progress >= 0.65:
            self._draw_x_eye(draw, layout.far_eye_x, layout.eye_y, outline, scale)
            self._draw_x_eye(draw, layout.near_eye_x, layout.eye_y, outline, scale)
        elif pose.dizzy:
            self._draw_swirl_eye(draw, layout.far_eye_x, layout.eye_y, outline, eyes, scale)
            self._draw_swirl_eye(draw, layout.near_eye_x, layout.eye_y, outline, eyes, scale)
        else:
            self._draw_eye_feature(draw, layout.far_eye_x, layout.eye_y, layout.far_eye_rx, layout.eye_ry, outline, eyes, scale, near=False)
            self._draw_eye_feature(draw, layout.near_eye_x, layout.eye_y, layout.near_eye_rx, layout.eye_ry, outline, eyes, scale, near=True)

        polygon(draw, list(layout.nose_pts), fill=nose_color, outline=outline)

        mouth_drop = pose.mouth_open * 10.0 * scale
        mouth_left = layout.mouth_left
        mouth_right = layout.mouth_right
        if spec.mood in {"snarl", "angry", "glare"}:
            mouth_right = (mouth_right[0], mouth_right[1] + 2 * scale + mouth_drop * 0.25)
        elif spec.mood in {"smirk", "sneaky"}:
            mouth_right = (mouth_right[0], mouth_right[1] - 2 * scale + mouth_drop * 0.10)
            mouth_left = (mouth_left[0], mouth_left[1] + 1 * scale)
        else:
            mouth_right = (mouth_right[0], mouth_right[1] + 0.5 * scale)
        line(draw, [mouth_left, mouth_right], fill=outline, width=max(1, int((3 if spec.mood in {"snarl", "angry", "glare", "smirk", "sneaky"} else 2) * scale)))

        tooth_slots = min(4, max(2, spec.tooth_count)) if (spec.tooth_count > 0 or spec.underbite) else 0
        for i in range(tooth_slots):
            tx = lerp(layout.tooth_left, layout.tooth_right, i / max(1, tooth_slots - 1))
            th = (5 if i % 2 == 0 else 4) * scale + mouth_drop * 0.35
            polygon(draw, [(tx - 1.3 * scale, layout.mouth_left[1] + 2.0 * scale), (tx + 1.3 * scale, layout.mouth_left[1] + 2.0 * scale), (tx, layout.mouth_left[1] + 2.0 * scale + th)], fill=teeth, outline=outline)

        if layout.scar_a is not None and layout.scar_b is not None:
            line(draw, [layout.scar_a, layout.scar_b], fill=outline, width=max(1, int(2 * scale)))

    def _draw_arm(
        self,
        draw: ImageDraw.ImageDraw,
        shoulder_x: float,
        shoulder_y: float,
        arm_len: float,
        facing: int,
        arm_dx: float,
        arm_dy: float,
        bend: float,
        depth_sign: int,
        color: Color,
        scale: float,
        extra_raise: float = 0.0,
        hand_radius: float = 4.0,
    ) -> Tuple[float, float]:
        hand_x = shoulder_x + facing * (arm_len * 0.32) + arm_dx * scale * facing
        hand_y = shoulder_y + arm_len * 0.82 + arm_dy * scale - extra_raise * scale
        elbow_x = (shoulder_x + hand_x) * 0.5 + facing * bend * 0.55 * scale + depth_sign * 3.0 * scale
        elbow_y = (shoulder_y + hand_y) * 0.5 - abs(bend) * 0.18 * scale + arm_dy * 0.16 * scale
        line(draw, [(shoulder_x, shoulder_y), (elbow_x, elbow_y), (hand_x, hand_y)], fill=color, width=max(2, int((8 if depth_sign < 0 else 9) * scale)))
        ellipse(draw, (hand_x - hand_radius * scale, hand_y - hand_radius * scale, hand_x + hand_radius * scale, hand_y + hand_radius * scale), fill=color)
        return hand_x, hand_y

    def _draw_legs(self, draw: ImageDraw.ImageDraw, spec: GoblinSpec, leg_color: Color, foot_color: Color, outline: Color, front_hip_x: float, front_hip_y: float, rear_hip_x: float, rear_hip_y: float, ground_y: float, pose: FramePose, scale: float) -> None:
        foot_w = spec.foot_w * scale
        legs = [
            (rear_hip_x, rear_hip_y, pose.rear_leg_dx, pose.rear_leg_lift, pose.rear_knee_bend, -1),
            (front_hip_x, front_hip_y, pose.front_leg_dx, pose.front_leg_lift, pose.front_knee_bend, 1),
        ]
        for hip_x, hip_y, dx, lift, knee_bias, depth_sign in legs:
            local_dx = dx * scale * spec.facing + spec.facing * (1.5 + 0.8 * depth_sign) * scale
            ankle_x = hip_x + local_dx
            ankle_y = ground_y - 5 * scale - lift * scale
            bend_dir = spec.facing
            knee_x = (hip_x + ankle_x) * 0.5 + bend_dir * (3.2 + knee_bias * 0.30) * scale
            knee_y = (hip_y + ankle_y) * 0.5 + (5.5 + pose.knee_bend + knee_bias * 0.65) * scale
            width = max(2, int((9 if depth_sign < 0 else 10) * scale))
            line(draw, [(hip_x, hip_y), (knee_x, knee_y), (ankle_x, ankle_y)], fill=leg_color, width=width)

            heel_x = ankle_x - spec.facing * foot_w * 0.30
            toe_x = ankle_x + spec.facing * foot_w * 0.70
            shoe = [
                (heel_x, ankle_y - 3.5 * scale),
                (toe_x, ankle_y - 3.0 * scale),
                (toe_x + spec.facing * 2.0 * scale, ankle_y + 1.0 * scale),
                (toe_x, ankle_y + 5.5 * scale),
                (heel_x - spec.facing * 1.5 * scale, ankle_y + 5.0 * scale),
            ]
            polygon(draw, shoe, fill=foot_color, outline=outline)

    def _draw_claws(self, draw: ImageDraw.ImageDraw, hand_x: float, hand_y: float, facing: int, teeth: Color, scale: float) -> None:
        for delta in (-3, 0, 3):
            line(draw, [(hand_x + facing * 1 * scale, hand_y), (hand_x + facing * (4 + abs(delta)) * scale, hand_y + delta * scale)], fill=teeth, width=max(1, int(scale)))

    def _weapon_basis(self, facing: int, pose: FramePose) -> Tuple[Point, Point]:
        ang = math.radians(-16.0 - pose.item_raise * 0.25 - pose.front_arm_dy * 0.10 + pose.front_arm_bend * 0.08)
        fx = facing * math.cos(ang)
        fy = math.sin(ang)
        mag = math.hypot(fx, fy) or 1.0
        fx /= mag
        fy /= mag
        sx = -fy
        sy = fx
        return (fx, fy), (sx, sy)

    def _offset_point(self, origin: Point, forward: Point, side: Point, along: float, across: float) -> Point:
        return (
            origin[0] + forward[0] * along + side[0] * across,
            origin[1] + forward[1] * along + side[1] * across,
        )

    def _tapered_quad(self, start: Point, end: Point, side: Point, start_half: float, end_half: float) -> List[Point]:
        return [
            (start[0] + side[0] * start_half, start[1] + side[1] * start_half),
            (end[0] + side[0] * end_half, end[1] + side[1] * end_half),
            (end[0] - side[0] * end_half, end[1] - side[1] * end_half),
            (start[0] - side[0] * start_half, start[1] - side[1] * start_half),
        ]

    def _draw_socketed_sword(self, draw: ImageDraw.ImageDraw, outline: Color, metal: Color, leather: Color, accent: Color, hand_x: float, hand_y: float, facing: int, pose: FramePose, scale: float) -> None:
        forward, side = self._weapon_basis(facing, pose)
        hand = (hand_x, hand_y)
        grip_back = self._offset_point(hand, forward, side, -10.0 * scale, 0.0)
        pommel = self._offset_point(hand, forward, side, -13.0 * scale, 0.0)
        guard_c = self._offset_point(hand, forward, side, 6.0 * scale, 0.0)
        blade_start = self._offset_point(hand, forward, side, 8.0 * scale, 0.0)
        blade_end = self._offset_point(hand, forward, side, 34.0 * scale, 0.0)
        tip = self._offset_point(hand, forward, side, 41.0 * scale, 0.0)

        grip_poly = self._tapered_quad(grip_back, hand, side, 2.2 * scale, 2.6 * scale)
        polygon(draw, grip_poly, fill=leather, outline=outline)
        ellipse(draw, (pommel[0] - 2.5 * scale, pommel[1] - 2.5 * scale, pommel[0] + 2.5 * scale, pommel[1] + 2.5 * scale), fill=accent, outline=outline)

        cross_a = self._offset_point(guard_c, forward, side, 0.0, -6.5 * scale)
        cross_b = self._offset_point(guard_c, forward, side, 0.0, 6.5 * scale)
        line(draw, [cross_a, cross_b], fill=accent, width=max(1, int(3 * scale)))

        blade_poly = [
            self._offset_point(blade_start, forward, side, 0.0, 2.6 * scale),
            self._offset_point(blade_end, forward, side, 0.0, 1.3 * scale),
            tip,
            self._offset_point(blade_end, forward, side, 0.0, -1.3 * scale),
            self._offset_point(blade_start, forward, side, 0.0, -2.6 * scale),
        ]
        polygon(draw, blade_poly, fill=metal, outline=outline)
        fuller_a = self._offset_point(blade_start, forward, side, 2.0 * scale, 0.0)
        fuller_b = self._offset_point(blade_end, forward, side, -4.0 * scale, 0.0)
        line(draw, [fuller_a, fuller_b], fill=lighten(metal, 0.16), width=max(1, int(1.5 * scale)))

    def _draw_item(self, draw: ImageDraw.ImageDraw, spec: GoblinSpec, outline: Color, metal: Color, metal_dark: Color, leather: Color, accent: Color, muzzle: Color, hand_x: float, hand_y: float, facing: int, pose: FramePose, scale: float) -> None:
        item = spec.held_item
        if item == "none":
            return

        forward, side = self._weapon_basis(facing, pose)
        hand = (hand_x, hand_y)

        if item == "knife":
            grip_back = self._offset_point(hand, forward, side, -7.0 * scale, 0.0)
            blade_start = self._offset_point(hand, forward, side, 4.0 * scale, 0.0)
            tip = self._offset_point(hand, forward, side, 19.0 * scale, 0.0)
            polygon(draw, self._tapered_quad(grip_back, hand, side, 2.0 * scale, 2.4 * scale), fill=leather, outline=outline)
            blade = [
                self._offset_point(blade_start, forward, side, 0.0, 2.0 * scale),
                self._offset_point(tip, forward, side, -2.5 * scale, 0.9 * scale),
                tip,
                self._offset_point(tip, forward, side, -2.5 * scale, -0.9 * scale),
                self._offset_point(blade_start, forward, side, 0.0, -2.0 * scale),
            ]
            polygon(draw, blade, fill=metal, outline=outline)
        elif item == "sword":
            self._draw_socketed_sword(draw, outline, metal, leather, accent, hand_x, hand_y, facing, pose, scale)
        elif item == "club":
            grip_back = self._offset_point(hand, forward, side, -8.0 * scale, 0.0)
            head_c = self._offset_point(hand, forward, side, 21.0 * scale, 0.0)
            polygon(draw, self._tapered_quad(grip_back, head_c, side, 2.3 * scale, 3.0 * scale), fill=leather, outline=outline)
            ellipse(draw, (head_c[0] - 8.0 * scale, head_c[1] - 8.0 * scale, head_c[0] + 8.0 * scale, head_c[1] + 8.0 * scale), fill=darken(leather, 0.18), outline=outline, width=max(1, int(scale)))
        elif item == "spear":
            butt = self._offset_point(hand, forward, side, -10.0 * scale, 0.0)
            head_base = self._offset_point(hand, forward, side, 36.0 * scale, 0.0)
            tip = self._offset_point(hand, forward, side, 49.0 * scale, 0.0)
            polygon(draw, self._tapered_quad(butt, head_base, side, 1.5 * scale, 1.1 * scale), fill=leather, outline=outline)
            spear_head = [
                self._offset_point(head_base, forward, side, 0.0, 3.0 * scale),
                tip,
                self._offset_point(head_base, forward, side, 0.0, -3.0 * scale),
                self._offset_point(head_base, forward, side, -4.0 * scale, 0.0),
            ]
            polygon(draw, spear_head, fill=metal, outline=outline)
        elif item == "staff":
            butt = self._offset_point(hand, forward, side, -9.0 * scale, 0.0)
            head_c = self._offset_point(hand, forward, side, 25.0 * scale, 0.0)
            polygon(draw, self._tapered_quad(butt, head_c, side, 2.0 * scale, 2.0 * scale), fill=leather, outline=outline)
            orb = self._offset_point(head_c, forward, side, 6.0 * scale, 0.0)
            ellipse(draw, (orb[0] - 6 * scale, orb[1] - 6 * scale, orb[0] + 6 * scale, orb[1] + 6 * scale), fill=accent, outline=outline, width=max(1, int(scale)))
        elif item == "gun":
            body_a = self._offset_point(hand, forward, side, -1.0 * scale, 0.0)
            body_b = self._offset_point(hand, forward, side, 18.0 * scale, 0.0)
            slide = self._tapered_quad(body_a, body_b, side, 3.6 * scale, 3.0 * scale)
            polygon(draw, slide, fill=metal_dark, outline=outline)
            grip_top = self._offset_point(hand, forward, side, 4.0 * scale, -1.0 * scale)
            grip_bot = self._offset_point(hand, forward, side, 1.0 * scale, -8.0 * scale)
            grip_poly = [
                self._offset_point(grip_top, forward, side, 0.0, 2.0 * scale),
                self._offset_point(grip_top, forward, side, 0.0, -2.0 * scale),
                self._offset_point(grip_bot, forward, side, 0.0, -2.5 * scale),
                self._offset_point(grip_bot, forward, side, 0.0, 2.5 * scale),
            ]
            polygon(draw, grip_poly, fill=metal_dark, outline=outline)
            muzzle_pt = self._offset_point(body_b, forward, side, 2.0 * scale, 0.0)
            ellipse(draw, (muzzle_pt[0] - 2 * scale, muzzle_pt[1] - 2 * scale, muzzle_pt[0] + 2 * scale, muzzle_pt[1] + 2 * scale), fill=muzzle, outline=outline)
        elif item == "shield":
            center = self._offset_point(hand, forward, side, 8.0 * scale, 0.0)
            shield_poly = [
                self._offset_point(center, forward, side, -8.0 * scale, -7.0 * scale),
                self._offset_point(center, forward, side, 7.0 * scale, -8.0 * scale),
                self._offset_point(center, forward, side, 11.0 * scale, 0.0),
                self._offset_point(center, forward, side, 7.0 * scale, 8.0 * scale),
                self._offset_point(center, forward, side, -8.0 * scale, 7.0 * scale),
                self._offset_point(center, forward, side, -12.0 * scale, 0.0),
            ]
            polygon(draw, shield_poly, fill=metal_dark, outline=outline)
            boss = self._offset_point(center, forward, side, 0.0, 0.0)
            ellipse(draw, (boss[0] - 3.5 * scale, boss[1] - 3.5 * scale, boss[0] + 3.5 * scale, boss[1] + 3.5 * scale), fill=accent, outline=outline)
        elif item == "bomb":
            bomb_c = self._offset_point(hand, forward, side, 1.0 * scale, 0.0)
            ellipse(draw, (bomb_c[0] - 7 * scale, bomb_c[1] - 7 * scale, bomb_c[0] + 7 * scale, bomb_c[1] + 7 * scale), fill=metal_dark, outline=outline, width=max(1, int(scale)))
            fuse_a = self._offset_point(bomb_c, forward, side, 3.0 * scale, -1.0 * scale)
            fuse_b = self._offset_point(bomb_c, forward, side, 8.0 * scale, -8.0 * scale)
            line(draw, [fuse_a, fuse_b], fill=accent, width=max(1, int(2 * scale)))
            ellipse(draw, (fuse_b[0] - 2 * scale, fuse_b[1] - 2 * scale, fuse_b[0] + 2 * scale, fuse_b[1] + 2 * scale), fill=muzzle)

    def _draw_swirl_eye(self, draw: ImageDraw.ImageDraw, x: float, y: float, outline: Color, accent: Color, scale: float) -> None:
        ellipse(draw, (x - 6 * scale, y - 5 * scale, x + 6 * scale, y + 5 * scale), fill=outline)
        for radius in (4, 2):
            ellipse(draw, (x - radius * scale, y - radius * scale, x + radius * scale, y + radius * scale), fill=None, outline=accent, width=max(1, int(scale)))

    def _draw_x_eye(self, draw: ImageDraw.ImageDraw, x: float, y: float, outline: Color, scale: float) -> None:
        line(draw, [(x - 4 * scale, y - 4 * scale), (x + 4 * scale, y + 4 * scale)], fill=outline, width=max(1, int(2 * scale)))
        line(draw, [(x + 4 * scale, y - 4 * scale), (x - 4 * scale, y + 4 * scale)], fill=outline, width=max(1, int(2 * scale)))

    def _draw_stun_effect(self, draw: ImageDraw.ImageDraw, outline: Color, accent: Color, x: float, y: float, scale: float) -> None:
        for i, dx in enumerate((-12, 0, 12)):
            px = x + dx * scale
            py = y + (2 if i == 1 else -2) * scale
            polygon(draw, [(px - 3 * scale, py), (px, py - 5 * scale), (px + 3 * scale, py), (px, py + 5 * scale)], fill=accent, outline=outline)


TARGETS: Dict[str, GoblinTarget] = {"goblin": GoblinTarget()}


def parse_background(value: str) -> Optional[Color]:
    return None if value.lower() == "transparent" else rgb(value)


def ensure_parent(path: str) -> None:
    parent = os.path.dirname(os.path.abspath(path))
    if parent:
        os.makedirs(parent, exist_ok=True)


def get_target(name: str):
    return TARGETS[name]


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description="Standalone procedural graphics generator")
    sub = parser.add_subparsers(dest="command", required=True)

    common = argparse.ArgumentParser(add_help=False)
    common.add_argument("--target", default="goblin", choices=sorted(TARGETS.keys()))
    common.add_argument("--seed", type=int, default=0)
    common.add_argument("--archetype", default="default")
    common.add_argument("--held-item", default=None, choices=GoblinTarget.HELD_ITEMS)
    common.add_argument("--width", type=int, default=512)
    common.add_argument("--height", type=int, default=512)

    p_single = sub.add_parser("single", parents=[common])
    p_single.add_argument("--background", default="transparent")
    p_single.add_argument("--out", required=True)

    p_sheet = sub.add_parser("sheet")
    p_sheet.add_argument("--target", default="goblin", choices=sorted(TARGETS.keys()))
    p_sheet.add_argument("--archetype", default="default")
    p_sheet.add_argument("--held-item", default=None, choices=GoblinTarget.HELD_ITEMS)
    p_sheet.add_argument("--count", type=int, default=12)
    p_sheet.add_argument("--base-seed", type=int, default=0)
    p_sheet.add_argument("--columns", type=int, default=4)
    p_sheet.add_argument("--cell-width", type=int, default=196)
    p_sheet.add_argument("--cell-height", type=int, default=220)
    p_sheet.add_argument("--label-height", type=int, default=28)
    p_sheet.add_argument("--background", default="#12161F")
    p_sheet.add_argument("--cell-background", default="#1A2030")
    p_sheet.add_argument("--out", required=True)

    p_spec = sub.add_parser("spec", parents=[common])

    p_spritesheet = sub.add_parser("spritesheet")
    p_spritesheet.add_argument("--target", default="goblin", choices=sorted(TARGETS.keys()))
    p_spritesheet.add_argument("--seed", type=int, default=0)
    p_spritesheet.add_argument("--archetype", default="default")
    p_spritesheet.add_argument("--held-item", default=None, choices=GoblinTarget.HELD_ITEMS)
    p_spritesheet.add_argument("--animations", default="idle,walk,attack,hurt,jump,fall,stun,death")
    p_spritesheet.add_argument("--frame-width", type=int, default=192)
    p_spritesheet.add_argument("--frame-height", type=int, default=192)
    p_spritesheet.add_argument("--supersample", type=int, default=12)
    p_spritesheet.add_argument("--downsample", choices=["nearest", "lanczos"], default="nearest")
    p_spritesheet.add_argument("--background", default="transparent")
    p_spritesheet.add_argument("--sheet-background", default="transparent")
    p_spritesheet.add_argument("--border", type=int, default=0)
    p_spritesheet.add_argument("--out", required=True)
    p_spritesheet.add_argument("--manifest-out")
    p_spritesheet.add_argument("--label-width", type=int, default=96)

    return parser


def cmd_single(args: argparse.Namespace) -> None:
    target = get_target(args.target)
    spec = target.sample_spec(args.seed, args.archetype, args.held_item)
    img = target.render(spec, (args.width, args.height), parse_background(args.background))
    ensure_parent(args.out)
    img.save(args.out)
    print(json.dumps(asdict(spec), indent=2))
    print_path(args.out, prefix="Wrote ")


def cmd_sheet(args: argparse.Namespace) -> None:
    target = get_target(args.target)
    cols = max(1, args.columns)
    rows = math.ceil(args.count / cols)
    outer_pad = 16
    cell_pad = 10
    cell_w = args.cell_width
    cell_h = args.cell_height
    label_h = args.label_height
    font = ImageFont.load_default()
    bg = parse_background(args.background) or rgb("#12161F")
    cell_bg = parse_background(args.cell_background) or rgb("#1A2030")
    sheet = Image.new("RGBA", (outer_pad * 2 + cols * cell_w, outer_pad * 2 + rows * cell_h), bg)
    draw = ImageDraw.Draw(sheet)
    for idx in range(args.count):
        seed = args.base_seed + idx
        spec = target.sample_spec(seed, args.archetype, args.held_item)
        col = idx % cols
        row = idx // cols
        x0 = outer_pad + col * cell_w
        y0 = outer_pad + row * cell_h
        rounded(draw, (x0, y0, x0 + cell_w - 1, y0 + cell_h - 1), radius=12, fill=cell_bg, outline=rgb("#2A3248"), width=2)
        render_h = cell_h - label_h - 8
        img = target.render(spec, (cell_w - 2 * cell_pad, render_h - 2 * cell_pad), None)
        sheet.alpha_composite(img, (x0 + cell_pad, y0 + cell_pad))
        label = f"seed={seed} {spec.archetype} {spec.held_item}"
        draw.text((x0 + 8, y0 + cell_h - label_h + 6), label, fill=rgb("#E8EEF7"), font=font)
    ensure_parent(args.out)
    sheet.save(args.out)
    print_path(args.out, prefix="Wrote ")


def cmd_spec(args: argparse.Namespace) -> None:
    target = get_target(args.target)
    spec = target.sample_spec(args.seed, args.archetype, args.held_item)
    print(json.dumps(asdict(spec), indent=2))


def cmd_spritesheet(args: argparse.Namespace) -> None:
    target = get_target(args.target)
    anim_defs = target.SPRITESHEET_ANIMATIONS
    requested = parse_csv(args.animations)
    for name in requested:
        if name not in anim_defs:
            raise SystemExit(f"Unknown animation: {name}")
    spec = target.sample_spec(args.seed, args.archetype, args.held_item)
    fw, fh = args.frame_width, args.frame_height
    border = max(0, args.border)
    label_w = max(0, args.label_width)
    cols = max(anim_defs[name]["frames"] for name in requested)
    rows = len(requested)
    row_h = fh + border * 2
    sheet_bg = parse_background(args.sheet_background)
    frame_bg = parse_background(args.background)
    sheet = Image.new("RGBA", (label_w + cols * (fw + border * 2), rows * row_h), sheet_bg if sheet_bg is not None else (0, 0, 0, 0))
    draw = ImageDraw.Draw(sheet)
    font = ImageFont.load_default()
    manifest = {
        "meta": {
            "generator": "procedural_graphics_generator.py",
            "target": args.target,
            "seed": args.seed,
            "archetype": args.archetype,
            "held_item": spec.held_item,
            "frame_width": fw,
            "frame_height": fh,
            "border": border,
            "label_width": label_w,
            "animations": requested,
            "spec": asdict(spec),
        },
        "animations": {},
        "frames": [],
    }
    for row, anim in enumerate(requested):
        row_y = row * row_h
        if label_w > 0:
            rounded(draw, (4, row_y + 4, label_w - 4, row_y + row_h - 4), radius=8, fill=rgb("#151B27", 220), outline=rgb("#3A455A"), width=1)
            label_text = anim.upper()
            bbox = draw.textbbox((0, 0), label_text, font=font)
            tx = 8
            ty = row_y + max(0, (row_h - (bbox[3] - bbox[1])) // 2)
            draw.text((tx, ty), label_text, fill=rgb("#E8EEF7"), font=font)
        frame_count = anim_defs[anim]["frames"]
        duration_ms = anim_defs[anim]["duration_ms"]
        anim_names = []
        for idx in range(frame_count):
            frame = target.render_animation_frame(spec, anim, idx, frame_count, (fw, fh), frame_bg, args.supersample, args.downsample)
            x = label_w + idx * (fw + border * 2) + border
            y = row_y + border
            sheet.alpha_composite(frame, (x, y))
            name = f"{anim}_{idx}"
            manifest["frames"].append({"name": name, "animation": anim, "index": idx, "x": x, "y": y, "w": fw, "h": fh, "duration_ms": duration_ms})
            anim_names.append(name)
        manifest["animations"][anim] = {"frames": anim_names, "frame_count": frame_count, "duration_ms": duration_ms, "row": row}
    ensure_parent(args.out)
    sheet.save(args.out)
    print_path(args.out, prefix="Wrote ")
    if args.manifest_out:
        ensure_parent(args.manifest_out)
        with open(args.manifest_out, "w", encoding="utf-8") as file:
            json.dump(manifest, file, indent=2)
        print_path(args.manifest_out, prefix="Wrote ")


def main(argv=None) -> None:
    args = build_parser().parse_args(argv)
    if args.command == "single":
        cmd_single(args)
    elif args.command == "sheet":
        cmd_sheet(args)
    elif args.command == "spec":
        cmd_spec(args)
    elif args.command == "spritesheet":
        cmd_spritesheet(args)


if __name__ == "__main__":
    main()
