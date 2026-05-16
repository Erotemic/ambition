from __future__ import annotations

"""Stylized right-facing humanoid character target.

This target is meant to be the "general character" lane for Ambition: a more
flexible 2D cartoon renderer that can produce memorable NPC silhouettes without
binding the output to one base rig plus accessory swaps.

Design goals:
- clean 2000s Flash / web-cartoon readability: bold outlines, integrated
  clothing shapes, and expressive silhouettes.
- deterministic YAML-driven specs: each character is defined by a preset plus
  optional numeric / categorical overrides, so later Rust integration can treat
  characters more like authored specs than ad-hoc random seeds.
- structural variation first: presets alter torso, limb, head, and costume
  proportions instead of only changing props and palettes.
"""

import math
import random
from dataclasses import dataclass
from typing import Dict, Optional, Tuple

from PIL import Image, ImageColor, ImageDraw

from .common_draw import RESAMPLING, draw_capsule, draw_rotated_ellipse, draw_rotated_rounded_rect
from ..rig import add, clamp, ease_in_out_sine, ease_out_cubic, smoothstep, vec

Color = Tuple[int, int, int, int]
Point = Tuple[float, float]


# --- generic helpers -----------------------------------------------------------

def rgba(value: str, alpha: int = 255) -> Color:
    r, g, b = ImageColor.getrgb(value)
    return (r, g, b, alpha)



def with_alpha(color: Color, alpha: int) -> Color:
    return (color[0], color[1], color[2], alpha)



def parse_background(value: str) -> Optional[Color]:
    return None if str(value).lower() == "transparent" else rgba(str(value))



def _bbox(center: Point, w: float, h: float) -> Tuple[float, float, float, float]:
    return (
        center[0] - w / 2.0,
        center[1] - h / 2.0,
        center[0] + w / 2.0,
        center[1] + h / 2.0,
    )



def _paste_rotated_local(base: Image.Image, layer: Image.Image, center: Point, angle: float) -> None:
    rotated = layer.rotate(angle, resample=RESAMPLING.BICUBIC, expand=True)
    base.alpha_composite(rotated, (int(center[0] - rotated.width / 2), int(center[1] - rotated.height / 2)))



def _scale_color(color: Color, factor: float) -> Color:
    return (
        int(clamp(color[0] * factor, 0, 255)),
        int(clamp(color[1] * factor, 0, 255)),
        int(clamp(color[2] * factor, 0, 255)),
        color[3],
    )


# --- dataclasses ---------------------------------------------------------------

@dataclass(frozen=True)
class ToonSpec:
    target: str
    seed: int
    archetype: str
    name: str
    role: str
    palette_name: str
    body_plan: str
    outfit: str
    hair_style: str
    prop: str
    accessory: str
    head_w: float
    head_h: float
    chin_h: float
    neck_h: float
    shoulder_w: float
    torso_w: float
    torso_h: float
    hip_w: float
    arm_upper: float
    arm_lower: float
    arm_radius: float
    leg_upper: float
    leg_lower: float
    leg_radius: float
    hand_r: float
    foot_w: float
    foot_h: float
    coat_len: float
    cape_len: float
    hair_volume: float
    nose_len: float
    satchel_size: float
    # General-hat local authored offsets.  YAML `spec` can use these to
    # tune the brim without touching drawing code. Negative Y moves the
    # brim upward in image space; positive Y lowers it.
    hat_brim_offset_x: float = 0.0
    hat_brim_offset_y: float = 0.0


@dataclass
class ToonPose:
    root_x: float = 0.0
    root_y: float = 0.0
    body_bob: float = 0.0
    torso_tilt: float = 0.0
    head_tilt: float = 0.0
    crouch: float = 0.0
    lean: float = 0.0
    far_arm_upper: float = 150.0
    far_arm_lower: float = 132.0
    near_arm_upper: float = 24.0
    near_arm_lower: float = 18.0
    far_leg_upper: float = 96.0
    far_leg_lower: float = 88.0
    near_leg_upper: float = 70.0
    near_leg_lower: float = 82.0
    blink: bool = False
    eye_squint: float = 0.0
    mouth_open: float = 0.0
    gesture: float = 0.0
    prop_swing: float = 0.0
    slash: float = 0.0
    dash: float = 0.0
    hit: float = 0.0
    collapse: float = 0.0
    dead: bool = False


class ToonSideGenerator:
    name = "toon"

    ANIMATIONS: Dict[str, Dict[str, int]] = {
        "idle": {"frames": 8, "duration_ms": 120},
        "walk": {"frames": 8, "duration_ms": 100},
        "run": {"frames": 8, "duration_ms": 78},
        "jump": {"frames": 6, "duration_ms": 90},
        "fall": {"frames": 6, "duration_ms": 90},
        "talk": {"frames": 6, "duration_ms": 100},
        "interact": {"frames": 6, "duration_ms": 95},
        "slash": {"frames": 7, "duration_ms": 72},
        "dash": {"frames": 6, "duration_ms": 64},
        "celebrate": {"frames": 6, "duration_ms": 90},
        "hit": {"frames": 5, "duration_ms": 90},
        "death": {"frames": 7, "duration_ms": 110},
    }

    PALETTES = {
        "hero": {
            "skin": rgba("#F1C7A4"),
            "skin_shadow": rgba("#D09C77"),
            "hair": rgba("#423137"),
            "hair_shine": rgba("#614850"),
            "outfit": rgba("#3C6FF4"),
            "outfit_dark": rgba("#2448A5"),
            "accent": rgba("#FFBA49"),
            "accent_dark": rgba("#D07C1F"),
            "shoe": rgba("#2D2738"),
            "outline": rgba("#1B1B22"),
            "shadow": rgba("#000000", 36),
            "white": rgba("#FFF6E8"),
        },
        "guide": {
            "skin": rgba("#D8C6B1"),
            "skin_shadow": rgba("#BAA189"),
            "hair": rgba("#32404B"),
            "hair_shine": rgba("#556774"),
            "outfit": rgba("#5D9BA3"),
            "outfit_dark": rgba("#396771"),
            "accent": rgba("#EAD27A"),
            "accent_dark": rgba("#C8AB44"),
            "shoe": rgba("#2F353A"),
            "outline": rgba("#1C1F22"),
            "shadow": rgba("#000000", 38),
            "white": rgba("#FFF6E8"),
        },
        "merchant": {
            "skin": rgba("#E2C09B"),
            "skin_shadow": rgba("#BB9471"),
            "hair": rgba("#5B3F2E"),
            "hair_shine": rgba("#7D5A45"),
            "outfit": rgba("#8A5D3C"),
            "outfit_dark": rgba("#5D3A23"),
            "accent": rgba("#D8B464"),
            "accent_dark": rgba("#A57A2D"),
            "shoe": rgba("#342A27"),
            "outline": rgba("#1E1B1A"),
            "shadow": rgba("#000000", 40),
            "white": rgba("#FFF6E8"),
        },
        "architect": {
            "skin": rgba("#CFBDB4"),
            "skin_shadow": rgba("#A48E85"),
            "hair": rgba("#24262B"),
            "hair_shine": rgba("#4A4F57"),
            "outfit": rgba("#6B4FD7"),
            "outfit_dark": rgba("#43318D"),
            "accent": rgba("#79D5E8"),
            "accent_dark": rgba("#3C98B0"),
            "shoe": rgba("#2A2732"),
            "outline": rgba("#17161F"),
            "shadow": rgba("#000000", 40),
            "white": rgba("#FFF6E8"),
        },
        "keeper": {
            "skin": rgba("#DDB69A"),
            "skin_shadow": rgba("#B88E71"),
            "hair": rgba("#F2E8D8"),
            "hair_shine": rgba("#FFF9EE"),
            "outfit": rgba("#6F303B"),
            "outfit_dark": rgba("#4A1E24"),
            "accent": rgba("#E1C66F"),
            "accent_dark": rgba("#BA9E45"),
            "shoe": rgba("#322730"),
            "outline": rgba("#1C171C"),
            "shadow": rgba("#000000", 42),
            "white": rgba("#FFF6E8"),
        },
        "absurd_general": {
            "skin": rgba("#E0A27F"),
            "skin_shadow": rgba("#B8725F"),
            "hair": rgba("#3A2E23"),
            "hair_shine": rgba("#67513C"),
            "outfit": rgba("#355F32"),
            "outfit_dark": rgba("#1D3421"),
            "accent": rgba("#FFD34F"),
            "accent_dark": rgba("#B77814"),
            "shoe": rgba("#201D17"),
            "outline": rgba("#181512"),
            "shadow": rgba("#000000", 46),
            "white": rgba("#FFF0DB"),
        },
        "fascist": {
            "skin": rgba("#D9C1AF"),
            "skin_shadow": rgba("#B99683"),
            "hair": rgba("#D6D0C2"),
            "hair_shine": rgba("#F1EBDD"),
            "outfit": rgba("#383B42"),
            "outfit_dark": rgba("#17191E"),
            "accent": rgba("#C02632"),
            "accent_dark": rgba("#78141D"),
            "shoe": rgba("#101114"),
            "outline": rgba("#0A0B0E"),
            "shadow": rgba("#000000", 52),
            "white": rgba("#F1ECE2"),
        },
    }

    PRESETS = {
        "general_hero": {
            "name": "General Hero",
            "role": "player",
            "palette_name": "hero",
            "body_plan": "hero",
            "outfit": "jacket",
            "hair_style": "swoop",
            "prop": "blade",
            "accessory": "scarf",
            "head_w": 27.0,
            "head_h": 30.0,
            "chin_h": 7.0,
            "neck_h": 4.5,
            "shoulder_w": 25.0,
            "torso_w": 22.0,
            "torso_h": 27.0,
            "hip_w": 18.5,
            "arm_upper": 13.5,
            "arm_lower": 13.0,
            "arm_radius": 2.8,
            "leg_upper": 17.0,
            "leg_lower": 16.0,
            "leg_radius": 3.0,
            "hand_r": 3.3,
            "foot_w": 12.5,
            "foot_h": 4.8,
            "coat_len": 8.0,
            "cape_len": 0.0,
            "hair_volume": 7.5,
            "nose_len": 3.0,
            "satchel_size": 0.0,
        },
        "absurd_general": {
            "name": "Absurd General",
            "role": "npc",
            "palette_name": "absurd_general",
            "body_plan": "broad",
            "outfit": "general_uniform",
            "hair_style": "general_hat",
            "prop": "baton",
            "accessory": "medals",
            "head_w": 31.0,
            "head_h": 29.0,
            "chin_h": 9.0,
            "neck_h": 3.5,
            "shoulder_w": 42.0,
            "torso_w": 34.0,
            "torso_h": 31.0,
            "hip_w": 25.0,
            "arm_upper": 12.0,
            "arm_lower": 11.0,
            "arm_radius": 3.5,
            "leg_upper": 12.5,
            "leg_lower": 11.5,
            "leg_radius": 3.4,
            "hand_r": 3.8,
            "foot_w": 13.5,
            "foot_h": 5.3,
            "coat_len": 12.0,
            "cape_len": 0.0,
            "hair_volume": 4.0,
            "nose_len": 4.4,
            "satchel_size": 0.0,
            "hat_brim_offset_x": 0.0,
            "hat_brim_offset_y": -2.0,
        },
        "fascist_enforcer": {
            "name": "Fascist Enforcer",
            "role": "enemy",
            "palette_name": "fascist",
            "body_plan": "rigid",
            "outfit": "storm_uniform",
            "hair_style": "officer_cap",
            "prop": "rifle",
            "accessory": "none",
            "head_w": 28.5,
            "head_h": 29.0,
            "chin_h": 7.2,
            "neck_h": 3.6,
            "shoulder_w": 34.5,
            "torso_w": 27.5,
            "torso_h": 31.5,
            "hip_w": 22.0,
            "arm_upper": 13.0,
            "arm_lower": 12.0,
            "arm_radius": 3.2,
            "leg_upper": 15.0,
            "leg_lower": 14.0,
            "leg_radius": 3.1,
            "hand_r": 3.2,
            "foot_w": 12.5,
            "foot_h": 4.9,
            "coat_len": 14.0,
            "cape_len": 0.0,
            "hair_volume": 2.6,
            "nose_len": 3.8,
            "satchel_size": 0.0,
        },
        "kernel_guide": {
            "name": "Kernel Guide",
            "role": "npc",
            "palette_name": "guide",
            "body_plan": "soft",
            "outfit": "poncho",
            "hair_style": "hood",
            "prop": "tablet",
            "accessory": "shawl",
            "head_w": 26.0,
            "head_h": 28.0,
            "chin_h": 6.0,
            "neck_h": 3.0,
            "shoulder_w": 34.0,
            "torso_w": 28.0,
            "torso_h": 24.0,
            "hip_w": 26.5,
            "arm_upper": 11.5,
            "arm_lower": 11.0,
            "arm_radius": 2.7,
            "leg_upper": 10.8,
            "leg_lower": 9.8,
            "leg_radius": 2.9,
            "hand_r": 3.0,
            "foot_w": 11.5,
            "foot_h": 4.5,
            "coat_len": 12.0,
            "cape_len": 18.0,
            "hair_volume": 6.0,
            "nose_len": 2.6,
            "satchel_size": 7.5,
        },
        "merchant_prototype": {
            "name": "Merchant Prototype",
            "role": "npc",
            "palette_name": "merchant",
            "body_plan": "round",
            "outfit": "apron",
            "hair_style": "cap",
            "prop": "coin_pouch",
            "accessory": "satchel",
            "head_w": 24.5,
            "head_h": 27.0,
            "chin_h": 5.5,
            "neck_h": 3.0,
            "shoulder_w": 25.5,
            "torso_w": 31.5,
            "torso_h": 28.0,
            "hip_w": 27.0,
            "arm_upper": 12.0,
            "arm_lower": 11.2,
            "arm_radius": 3.35,
            "leg_upper": 10.5,
            "leg_lower": 9.8,
            "leg_radius": 3.35,
            "hand_r": 3.2,
            "foot_w": 12.4,
            "foot_h": 4.7,
            "coat_len": 8.0,
            "cape_len": 0.0,
            "hair_volume": 5.0,
            "nose_len": 3.0,
            "satchel_size": 10.0,
        },
        "vault_keeper": {
            "name": "Vault Keeper",
            "role": "npc",
            "palette_name": "keeper",
            "body_plan": "broad",
            "outfit": "keeper_robe",
            "hair_style": "crest",
            "prop": "ledger",
            "accessory": "keys",
            "head_w": 26.5,
            "head_h": 30.0,
            "chin_h": 7.0,
            "neck_h": 4.2,
            "shoulder_w": 35.0,
            "torso_w": 29.5,
            "torso_h": 32.5,
            "hip_w": 26.5,
            "arm_upper": 14.0,
            "arm_lower": 13.0,
            "arm_radius": 3.4,
            "leg_upper": 15.0,
            "leg_lower": 14.0,
            "leg_radius": 3.3,
            "hand_r": 3.5,
            "foot_w": 13.0,
            "foot_h": 5.0,
            "coat_len": 22.0,
            "cape_len": 16.0,
            "hair_volume": 5.0,
            "nose_len": 3.2,
            "satchel_size": 0.0,
        },
        "architect": {
            "name": "Architect",
            "role": "npc",
            "palette_name": "architect",
            "body_plan": "tall",
            "outfit": "long_coat",
            "hair_style": "bob",
            "prop": "blueprint",
            "accessory": "sash",
            "head_w": 24.0,
            "head_h": 29.0,
            "chin_h": 6.4,
            "neck_h": 4.0,
            "shoulder_w": 22.5,
            "torso_w": 18.5,
            "torso_h": 29.0,
            "hip_w": 16.5,
            "arm_upper": 14.2,
            "arm_lower": 14.0,
            "arm_radius": 2.4,
            "leg_upper": 18.8,
            "leg_lower": 17.2,
            "leg_radius": 2.7,
            "hand_r": 2.9,
            "foot_w": 11.0,
            "foot_h": 4.2,
            "coat_len": 18.0,
            "cape_len": 0.0,
            "hair_volume": 6.6,
            "nose_len": 3.2,
            "satchel_size": 3.0,
        },
    }

    def sample_spec(self, seed: int, archetype: str = "general_hero") -> ToonSpec:
        try:
            preset = dict(self.PRESETS[archetype])
        except KeyError as ex:
            raise KeyError(f"unknown toon archetype {archetype!r}; available={sorted(self.PRESETS)}") from ex
        rng = random.Random(seed)
        # small hand-authored noise so repeats do not feel sterile while keeping
        # the structural silhouette locked to the preset.
        for key in [
            "head_w",
            "head_h",
            "torso_w",
            "torso_h",
            "hip_w",
            "shoulder_w",
            "leg_upper",
            "leg_lower",
            "arm_upper",
            "arm_lower",
            "foot_w",
        ]:
            preset[key] = float(preset[key]) + rng.uniform(-0.6, 0.6)
        preset["hair_volume"] = float(preset["hair_volume"]) + rng.uniform(-0.4, 0.4)
        preset["nose_len"] = float(preset["nose_len"]) + rng.uniform(-0.2, 0.2)
        return ToonSpec(target=self.name, seed=seed, archetype=archetype, **preset)

    def pose_for_animation(self, animation: str, frame_index: int, frame_count: int, spec: ToonSpec) -> ToonPose:
        p = ToonPose()
        t = 0.0 if frame_count <= 1 else frame_index / float(frame_count - 1)
        wave = math.sin(t * math.tau)
        plan = spec.body_plan
        run_scale = 1.0 if plan not in {"round", "soft"} else 0.82
        if animation == "idle":
            p.body_bob = abs(wave) * (1.1 if plan != "tall" else 0.7)
            p.torso_tilt = wave * (1.2 if plan != "broad" else 0.6)
            p.head_tilt = -wave * 0.8
            p.blink = frame_index == frame_count // 2
            p.eye_squint = 0.12 if frame_index in {1, frame_count - 2} else 0.0
        elif animation in {"walk", "run"}:
            stride = math.sin(t * math.tau)
            bounce = (1.0 - math.cos(t * math.tau * 2.0)) * 0.5
            leg_amp = (18.0 if animation == "walk" else 26.0) * run_scale
            arm_amp = (11.0 if animation == "walk" else 16.0) * (1.0 if plan != "round" else 0.85)
            p.root_x = stride * (1.0 if animation == "walk" else 1.8)
            p.body_bob = 0.5 + bounce * (1.8 if animation == "walk" else 2.5)
            p.torso_tilt = (-3.0 if animation == "walk" else -8.0) - stride * 3.5
            p.head_tilt = -bounce * 1.6
            p.far_arm_upper = 150.0 + stride * arm_amp
            p.far_arm_lower = 132.0 + stride * arm_amp * 0.6
            p.near_arm_upper = 24.0 - stride * arm_amp
            p.near_arm_lower = 18.0 - stride * arm_amp * 0.6
            p.far_leg_upper = 94.0 + stride * leg_amp
            p.far_leg_lower = 86.0 - max(0.0, stride) * 18.0 + max(0.0, -stride) * 8.0
            p.near_leg_upper = 70.0 - stride * leg_amp
            p.near_leg_lower = 82.0 - max(0.0, -stride) * 18.0 + max(0.0, stride) * 8.0
            p.eye_squint = 0.06 + bounce * 0.09
        elif animation == "jump":
            arc = math.sin(t * math.pi)
            lift = ease_in_out_sine(arc)
            p.root_y = -18.0 * lift
            p.root_x = 1.6 * t
            p.torso_tilt = -5.0 + 4.0 * t
            p.head_tilt = -2.0 - 2.0 * lift
            p.far_arm_upper = 166.0 - 10.0 * lift
            p.far_arm_lower = 142.0 - 6.0 * lift
            p.near_arm_upper = 4.0 + 18.0 * lift
            p.near_arm_lower = 10.0 + 16.0 * lift
            p.far_leg_upper = 125.0
            p.far_leg_lower = 92.0
            p.near_leg_upper = 80.0
            p.near_leg_lower = 94.0
        elif animation == "fall":
            fall = ease_out_cubic(t)
            p.root_y = -10.0 + 14.0 * fall
            p.torso_tilt = 5.0 + 2.0 * fall
            p.head_tilt = 2.0
            p.far_arm_upper = 198.0
            p.far_arm_lower = 172.0
            p.near_arm_upper = 48.0
            p.near_arm_lower = 36.0
            p.far_leg_upper = 100.0
            p.far_leg_lower = 72.0
            p.near_leg_upper = 76.0
            p.near_leg_lower = 68.0
        elif animation == "talk":
            bob = math.sin(t * math.tau)
            p.body_bob = abs(bob) * 0.6
            p.torso_tilt = -1.0 + bob * 1.6
            p.head_tilt = -2.0 + bob * 1.1
            p.near_arm_upper = -18.0 + max(0.0, bob) * 30.0
            p.near_arm_lower = -6.0 + max(0.0, bob) * 18.0
            p.far_arm_upper = 154.0 - max(0.0, -bob) * 16.0
            p.far_arm_lower = 136.0 - max(0.0, -bob) * 10.0
            p.mouth_open = 0.5 + 0.5 * abs(bob)
            p.gesture = max(0.0, bob)
            p.eye_squint = 0.08 * max(0.0, -bob)
        elif animation == "interact":
            reach = smoothstep(clamp(t / 0.85, 0.0, 1.0))
            p.root_x = 1.2 * reach
            p.torso_tilt = -6.0 * reach
            p.head_tilt = -3.0 * reach
            p.near_arm_upper = -12.0 - 14.0 * reach
            p.near_arm_lower = 10.0 + 8.0 * reach
            p.far_arm_upper = 150.0 + 8.0 * reach
            p.far_arm_lower = 132.0 + 4.0 * reach
            p.gesture = reach
        elif animation == "slash":
            wind = smoothstep(clamp(t / 0.34, 0.0, 1.0))
            swing = smoothstep(clamp((t - 0.26) / 0.48, 0.0, 1.0))
            p.root_x = -2.0 * wind + 4.0 * swing
            p.root_y = 2.0 * wind
            p.torso_tilt = -18.0 * wind + 11.0 * swing
            p.head_tilt = -8.0 * wind + 5.0 * swing
            p.far_arm_upper = 170.0 - 20.0 * wind
            p.far_arm_lower = 150.0 - 18.0 * wind
            p.near_arm_upper = -22.0 - 46.0 * wind + 120.0 * swing
            p.near_arm_lower = -18.0 - 12.0 * wind + 82.0 * swing
            p.far_leg_upper = 114.0 + 10.0 * wind
            p.far_leg_lower = 90.0
            p.near_leg_upper = 68.0 - 6.0 * wind
            p.near_leg_lower = 78.0
            p.slash = swing
            p.prop_swing = swing
        elif animation == "dash":
            burst = smoothstep(t)
            p.root_x = 8.0 * burst
            p.root_y = 1.2 * math.sin(t * math.pi)
            p.torso_tilt = -10.0
            p.head_tilt = -4.0
            p.far_arm_upper = 188.0
            p.far_arm_lower = 164.0
            p.near_arm_upper = 8.0
            p.near_arm_lower = 2.0
            p.far_leg_upper = 120.0
            p.far_leg_lower = 78.0
            p.near_leg_upper = 72.0
            p.near_leg_lower = 62.0
            p.dash = burst
        elif animation == "celebrate":
            pulse = math.sin(t * math.pi)
            p.body_bob = abs(wave) * 1.2
            p.torso_tilt = wave * 1.5
            p.head_tilt = -wave * 0.6
            p.far_arm_upper = 228.0 - 8.0 * pulse
            p.far_arm_lower = 210.0
            p.near_arm_upper = -50.0 + 8.0 * pulse
            p.near_arm_lower = -38.0
            p.mouth_open = 0.8
        elif animation == "hit":
            flinch = math.sin(t * math.pi)
            p.root_x = -4.0 * flinch
            p.root_y = 1.2 * flinch
            p.torso_tilt = 10.0 * flinch
            p.head_tilt = 8.0 * flinch
            p.far_arm_upper = 175.0
            p.far_arm_lower = 155.0
            p.near_arm_upper = 42.0
            p.near_arm_lower = 30.0
            p.hit = flinch
            p.eye_squint = 0.2 + 0.25 * flinch
        elif animation == "death":
            collapse = smoothstep(t)
            p.root_x = 6.0 * collapse
            p.root_y = 7.0 * collapse
            p.torso_tilt = 12.0 + 70.0 * collapse
            p.head_tilt = 8.0 + 56.0 * collapse
            p.far_arm_upper = 210.0 - 26.0 * collapse
            p.far_arm_lower = 188.0 - 20.0 * collapse
            p.near_arm_upper = 54.0 + 24.0 * collapse
            p.near_arm_lower = 36.0 + 18.0 * collapse
            p.far_leg_upper = 114.0 + 10.0 * collapse
            p.far_leg_lower = 76.0 + 8.0 * collapse
            p.near_leg_upper = 78.0 - 10.0 * collapse
            p.near_leg_lower = 72.0 - 8.0 * collapse
            p.collapse = collapse
            p.dead = collapse > 0.75
        if spec.archetype == "fascist_enforcer":
            if animation == "idle":
                p.body_bob *= 0.25
                p.torso_tilt -= 1.2
                p.head_tilt += 0.4
                p.eye_squint = max(p.eye_squint, 0.18)
            elif animation in {"walk", "run"}:
                p.torso_tilt -= 1.8
                p.head_tilt -= 0.6
                p.eye_squint = max(p.eye_squint, 0.16)
                p.far_arm_lower -= 3.0
                p.near_arm_lower += 3.0
            elif animation == "talk":
                p.torso_tilt -= 0.8
                p.head_tilt += 0.3
                p.eye_squint = max(p.eye_squint, 0.14)
                p.mouth_open = max(p.mouth_open, 0.25)
            elif animation == "interact":
                p.torso_tilt -= 2.0
                p.head_tilt -= 0.5
                p.gesture = max(p.gesture, 0.4)
            elif animation == "slash":
                p.root_x += 1.5
                p.torso_tilt -= 3.5
                p.head_tilt -= 1.5
                p.prop_swing = max(p.prop_swing, 0.75)
            elif animation == "dash":
                p.torso_tilt -= 2.0
                p.head_tilt -= 0.5
            elif animation == "hit":
                p.head_tilt += 1.4
            elif animation == "death":
                p.torso_tilt += 8.0 * p.collapse
                p.head_tilt += 6.0 * p.collapse
        return p

    # --- render helpers --------------------------------------------------------

    def _palette(self, spec: ToonSpec) -> Dict[str, Color]:
        return dict(self.PALETTES[spec.palette_name])

    def _body_plan_shift(self, spec: ToonSpec) -> Dict[str, float]:
        return {
            "hero": {"shoulder_y": -2.0, "hip_y": 1.0, "head_y": -1.0},
            "soft": {"shoulder_y": 0.0, "hip_y": 1.5, "head_y": -0.5},
            "round": {"shoulder_y": 1.0, "hip_y": 2.2, "head_y": 0.0},
            "broad": {"shoulder_y": -1.0, "hip_y": 1.0, "head_y": -1.2},
            "tall": {"shoulder_y": -2.8, "hip_y": -1.0, "head_y": -2.4},
            "rigid": {"shoulder_y": -1.6, "hip_y": 0.6, "head_y": -1.4},
        }.get(spec.body_plan, {"shoulder_y": 0.0, "hip_y": 0.0, "head_y": 0.0})

    def _draw_shadow(self, draw: ImageDraw.ImageDraw, center: Point, width: float, S: float, alpha: int) -> None:
        draw.ellipse(_bbox(center, width * S, 12.0 * S), fill=(0, 0, 0, alpha))

    def _draw_head(self, base: Image.Image, center: Point, spec: ToonSpec, pal: Dict[str, Color], S: float, pose: ToonPose) -> None:
        pad = int(max(spec.head_w, spec.head_h) * S * 1.7)
        layer = Image.new("RGBA", (pad * 2, pad * 2), (0, 0, 0, 0))
        d = ImageDraw.Draw(layer)
        c = (pad, pad)
        outline = pal["outline"]
        # Hood / back hair mass first.
        if spec.hair_style == "hood":
            d.ellipse(_bbox((c[0] - 2 * S, c[1] - 1 * S), (spec.head_w + 8.0) * S, (spec.head_h + 8.0) * S), fill=pal["outfit_dark"], outline=outline, width=max(1, int(1.2 * S)))
        elif spec.hair_style in {"bob", "crest", "swoop", "cap", "general_hat", "officer_cap"}:
            d.ellipse(_bbox((c[0] - 1.0 * S, c[1] - 4.0 * S), (spec.head_w + spec.hair_volume) * S, (spec.head_h * 0.78 + spec.hair_volume * 0.45) * S), fill=pal["hair"], outline=outline, width=max(1, int(1.1 * S)))
        # Face.
        d.ellipse(_bbox(c, spec.head_w * S, spec.head_h * S), fill=pal["skin"], outline=outline, width=max(1, int(1.2 * S)))
        d.ellipse(_bbox((c[0] + 1.0 * S, c[1] + spec.head_h * 0.18 * S), (spec.head_w * 0.70) * S, (spec.chin_h * 1.9) * S), fill=pal["skin_shadow"], outline=None)
        # Front hair / features.
        if spec.hair_style == "swoop":
            pts = [
                (c[0] - spec.head_w * 0.45 * S, c[1] - spec.head_h * 0.40 * S),
                (c[0] + spec.head_w * 0.12 * S, c[1] - spec.head_h * 0.62 * S),
                (c[0] + spec.head_w * 0.50 * S, c[1] - spec.head_h * 0.10 * S),
                (c[0] + spec.head_w * 0.10 * S, c[1] - spec.head_h * 0.04 * S),
            ]
            d.polygon(pts, fill=pal["hair"], outline=outline)
        elif spec.hair_style == "bob":
            d.pieslice(_bbox((c[0] - 1.0 * S, c[1] - 1.5 * S), (spec.head_w + spec.hair_volume * 0.8) * S, (spec.head_h + spec.hair_volume * 0.2) * S), start=195, end=18, fill=pal["hair"], outline=outline)
        elif spec.hair_style == "crest":
            crest = [
                (c[0] - 2 * S, c[1] - spec.head_h * 0.60 * S),
                (c[0] + 6 * S, c[1] - spec.head_h * 0.95 * S),
                (c[0] + 10 * S, c[1] - spec.head_h * 0.20 * S),
                (c[0] + 2 * S, c[1] - spec.head_h * 0.12 * S),
            ]
            d.polygon(crest, fill=pal["hair"], outline=outline)
        elif spec.hair_style == "cap":
            d.pieslice(_bbox((c[0] - 1.5 * S, c[1] - spec.head_h * 0.28 * S), (spec.head_w + 4.0) * S, (spec.head_h * 0.72) * S), start=180, end=15, fill=pal["outfit_dark"], outline=outline)
            d.polygon([(c[0] + 2 * S, c[1] - 1 * S), (c[0] + 12 * S, c[1] + 1 * S), (c[0] + 1 * S, c[1] + 4 * S)], fill=pal["outfit_dark"], outline=outline)
        elif spec.hair_style == "general_hat":
            # An intentionally over-loud peaked cap: huge crown, gold band,
            # forward brim, and a centered star so the silhouette reads as a
            # shouting cartoon general before any facial details are visible.
            crown = [
                (c[0] - 18.5 * S, c[1] - 21.0 * S),
                (c[0] - 12.0 * S, c[1] - 32.0 * S),
                (c[0] + 11.5 * S, c[1] - 33.5 * S),
                (c[0] + 19.0 * S, c[1] - 20.5 * S),
                (c[0] + 13.5 * S, c[1] - 14.5 * S),
                (c[0] - 14.0 * S, c[1] - 14.5 * S),
            ]
            d.polygon(crown, fill=pal["outfit"], outline=outline)
            d.rounded_rectangle((c[0] - 16.5 * S, c[1] - 19.4 * S, c[0] + 16.5 * S, c[1] - 12.6 * S), radius=2.0 * S, fill=pal["accent"], outline=outline, width=max(1, int(1.0 * S)))
            brim_dx = spec.hat_brim_offset_x * S
            brim_dy = spec.hat_brim_offset_y * S
            brim = [
                (c[0] - 18.2 * S + brim_dx, c[1] - 12.8 * S + brim_dy),
                (c[0] + 12.2 * S + brim_dx, c[1] - 11.4 * S + brim_dy),
                (c[0] + 21.0 * S + brim_dx, c[1] - 8.2 * S + brim_dy),
                (c[0] + 6.0 * S + brim_dx, c[1] - 5.0 * S + brim_dy),
                (c[0] - 15.8 * S + brim_dx, c[1] - 8.0 * S + brim_dy),
            ]
            d.polygon(brim, fill=pal["outfit_dark"], outline=outline)
            # A narrow highlight on the lower lip of the raised brim separates
            # it from the eyebrows / eyes when the sprite is downsampled.
            d.line(
                [
                    (c[0] - 12.0 * S + brim_dx, c[1] - 7.0 * S + brim_dy),
                    (c[0] + 6.2 * S + brim_dx, c[1] - 5.0 * S + brim_dy),
                ],
                fill=_scale_color(pal["outfit"], 1.18),
                width=max(1, int(0.8 * S)),
            )
            star_c = (c[0] + 0.5 * S, c[1] - 25.2 * S)
            star = []
            for i in range(10):
                r = (5.0 if i % 2 == 0 else 2.2) * S
                a = -math.pi / 2 + i * math.tau / 10
                star.append((star_c[0] + math.cos(a) * r, star_c[1] + math.sin(a) * r))
            d.polygon(star, fill=pal["accent"], outline=outline)
        elif spec.hair_style == "officer_cap":
            crown = [
                (c[0] - 16.5 * S, c[1] - 19.0 * S),
                (c[0] - 10.0 * S, c[1] - 28.5 * S),
                (c[0] + 10.8 * S, c[1] - 29.5 * S),
                (c[0] + 16.0 * S, c[1] - 18.5 * S),
                (c[0] + 10.5 * S, c[1] - 13.0 * S),
                (c[0] - 13.0 * S, c[1] - 13.5 * S),
            ]
            d.polygon(crown, fill=pal["outfit"], outline=outline)
            d.rounded_rectangle((c[0] - 15.0 * S, c[1] - 18.0 * S, c[0] + 14.0 * S, c[1] - 12.0 * S), radius=1.8 * S, fill=pal["accent_dark"], outline=outline, width=max(1, int(1.0 * S)))
            visor = [
                (c[0] - 13.0 * S, c[1] - 10.7 * S),
                (c[0] + 8.0 * S, c[1] - 9.8 * S),
                (c[0] + 17.5 * S, c[1] - 6.0 * S),
                (c[0] + 4.8 * S, c[1] - 2.8 * S),
                (c[0] - 10.8 * S, c[1] - 5.5 * S),
            ]
            d.polygon(visor, fill=pal["outfit_dark"], outline=outline)
            badge_c = (c[0] + 0.8 * S, c[1] - 23.2 * S)
            d.ellipse(_bbox(badge_c, 8.2 * S, 8.2 * S), fill=pal["white"], outline=outline, width=max(1, int(0.9 * S)))
            d.polygon([
                (badge_c[0] - 2.4 * S, badge_c[1] - 0.8 * S),
                (badge_c[0] + 2.2 * S, badge_c[1] - 0.8 * S),
                (badge_c[0] + 3.0 * S, badge_c[1] + 1.8 * S),
                (badge_c[0] - 3.0 * S, badge_c[1] + 1.8 * S),
            ], fill=outline, outline=outline)
            d.ellipse(_bbox((badge_c[0] - 1.6 * S, badge_c[1] - 2.0 * S), 2.0 * S, 2.0 * S), fill=outline)
            d.ellipse(_bbox((badge_c[0] + 1.6 * S, badge_c[1] - 2.0 * S), 2.0 * S, 2.0 * S), fill=outline)
        if spec.hair_style == "general_hat":
            # Draw the eyes clearly below the brim. Earlier versions put the
            # angry brow and brim on the same dark band, which read like a mask.
            eye_y = c[1] + 0.8 * S
            eye_x = c[0] + 4.6 * S
            eye_back = rgba("#FFF6E0")
            d.ellipse(_bbox((eye_x - 2.0 * S, eye_y), 4.4 * S, 2.7 * S), fill=eye_back, outline=outline, width=max(1, int(0.95 * S)))
            d.ellipse(_bbox((eye_x + 5.3 * S, eye_y - 0.1 * S), 3.2 * S, 2.2 * S), fill=eye_back, outline=outline, width=max(1, int(0.85 * S)))
            d.ellipse(_bbox((eye_x - 1.0 * S, eye_y + 0.2 * S), 1.4 * S, 1.8 * S), fill=outline)
            d.ellipse(_bbox((eye_x + 5.9 * S, eye_y + 0.2 * S), 1.1 * S, 1.5 * S), fill=outline)
            # Permanent angry brow shape; separate strokes, not a continuous visor.
            d.line([(eye_x - 6.0 * S, eye_y - 4.1 * S), (eye_x + 1.4 * S, eye_y - 1.6 * S)], fill=outline, width=max(1, int(1.45 * S)))
            d.line([(eye_x + 3.1 * S, eye_y - 1.5 * S), (eye_x + 8.2 * S, eye_y - 3.9 * S)], fill=outline, width=max(1, int(1.3 * S)))
        elif spec.hair_style == "officer_cap":
            eye_y = c[1] + 0.4 * S
            eye_x = c[0] + 4.5 * S
            eye_back = rgba("#EEE6D8")
            d.ellipse(_bbox((eye_x - 2.0 * S, eye_y), 4.0 * S, 2.2 * S), fill=eye_back, outline=outline, width=max(1, int(0.85 * S)))
            d.ellipse(_bbox((eye_x + 5.0 * S, eye_y + 0.1 * S), 3.2 * S, 2.0 * S), fill=eye_back, outline=outline, width=max(1, int(0.8 * S)))
            d.ellipse(_bbox((eye_x - 1.0 * S, eye_y + 0.2 * S), 1.3 * S, 1.4 * S), fill=outline)
            d.ellipse(_bbox((eye_x + 5.6 * S, eye_y + 0.3 * S), 1.0 * S, 1.2 * S), fill=outline)
            d.line([(eye_x - 5.5 * S, eye_y - 2.4 * S), (eye_x + 0.6 * S, eye_y - 0.8 * S)], fill=outline, width=max(1, int(1.2 * S)))
            d.line([(eye_x + 3.0 * S, eye_y - 0.9 * S), (eye_x + 7.2 * S, eye_y - 2.8 * S)], fill=outline, width=max(1, int(1.1 * S)))
        else:
            eye_y = c[1] - 1.8 * S
            eye_x = c[0] + 4.4 * S
            eyelid = max(1.2 * S, (1.2 + pose.eye_squint * 4.0) * S)
            if pose.blink or pose.dead:
                d.line([(eye_x - 2.2 * S, eye_y), (eye_x + 2.0 * S, eye_y)], fill=outline, width=max(1, int(1.3 * S)))
            else:
                d.ellipse(_bbox((eye_x, eye_y), 3.8 * S, eyelid), fill=pal["white"], outline=outline, width=max(1, int(1.0 * S)))
                pupil_y = eye_y + pose.eye_squint * 0.4 * S
                d.ellipse(_bbox((eye_x + 0.6 * S, pupil_y), 1.3 * S, 2.6 * S), fill=outline)
        nose = [
            (c[0] + 4.5 * S, c[1] + 1.8 * S),
            (c[0] + (4.5 + spec.nose_len) * S, c[1] + 3.0 * S),
            (c[0] + 4.4 * S, c[1] + 4.0 * S),
        ]
        d.line(nose, fill=_scale_color(pal["skin_shadow"], 0.85), width=max(1, int(1.0 * S)))
        mouth_y = c[1] + 7.0 * S
        if spec.hair_style == "general_hat":
            # More yell-hole than smile. The moustache is now split into two
            # clear chevrons so it reads as facial hair instead of a face mask.
            d.polygon([(c[0] - 1.8 * S, mouth_y - 3.0 * S), (c[0] + 2.0 * S, mouth_y - 4.4 * S), (c[0] + 4.0 * S, mouth_y - 2.0 * S), (c[0] + 0.5 * S, mouth_y - 0.6 * S)], fill=pal["hair"], outline=outline)
            d.polygon([(c[0] + 7.0 * S, mouth_y - 2.2 * S), (c[0] + 12.4 * S, mouth_y - 3.8 * S), (c[0] + 10.3 * S, mouth_y + 0.5 * S), (c[0] + 5.4 * S, mouth_y - 0.1 * S)], fill=pal["hair"], outline=outline)
            d.ellipse(_bbox((c[0] + 5.2 * S, mouth_y + 2.1 * S), 9.0 * S, 10.2 * S), fill=rgba("#2A1110"), outline=outline, width=max(1, int(1.1 * S)))
            d.rectangle((c[0] + 1.5 * S, mouth_y - 0.9 * S, c[0] + 8.4 * S, mouth_y + 1.0 * S), fill=pal["white"], outline=None)
            d.rectangle((c[0] + 3.0 * S, mouth_y + 5.0 * S, c[0] + 7.4 * S, mouth_y + 6.0 * S), fill=with_alpha(pal["white"], 205), outline=None)
        elif spec.hair_style == "officer_cap":
            d.line([(c[0] + 1.0 * S, mouth_y + 1.2 * S), (c[0] + 7.0 * S, mouth_y + 0.3 * S)], fill=outline, width=max(1, int(1.2 * S)))
            d.line([(c[0] + 2.6 * S, mouth_y - 1.4 * S), (c[0] + 5.8 * S, mouth_y - 1.8 * S)], fill=pal["hair"], width=max(1, int(1.1 * S)))
            if pose.mouth_open > 0.18:
                d.ellipse(_bbox((c[0] + 4.6 * S, mouth_y + 1.6 * S), 5.0 * S, 4.0 * S), fill=rgba("#30100F"), outline=outline, width=max(1, int(0.95 * S)))
        elif pose.mouth_open > 0.2:
            d.ellipse(_bbox((c[0] + 4.2 * S, mouth_y), 4.8 * S, (1.6 + pose.mouth_open * 1.8) * S), fill=_scale_color(outline, 0.9), outline=outline)
        else:
            d.arc((c[0] + 0.4 * S, mouth_y - 2 * S, c[0] + 8.2 * S, mouth_y + 2.5 * S), start=8, end=140, fill=outline, width=max(1, int(1.1 * S)))
        _paste_rotated_local(base, layer, center, pose.head_tilt)

    def _draw_torso(self, base: Image.Image, center: Point, spec: ToonSpec, pal: Dict[str, Color], S: float, pose: ToonPose) -> None:
        outline = pal["outline"]
        if spec.outfit == "jacket":
            pts = [
                (center[0] - spec.shoulder_w * 0.50 * S, center[1] - spec.torso_h * 0.46 * S),
                (center[0] + spec.shoulder_w * 0.32 * S, center[1] - spec.torso_h * 0.40 * S),
                (center[0] + spec.torso_w * 0.52 * S, center[1] + spec.torso_h * 0.06 * S),
                (center[0] + spec.hip_w * 0.32 * S, center[1] + spec.torso_h * 0.50 * S + spec.coat_len * 0.25 * S),
                (center[0] - spec.hip_w * 0.38 * S, center[1] + spec.torso_h * 0.50 * S),
            ]
            ImageDraw.Draw(base).polygon(pts, fill=pal["outfit"], outline=outline)
            d = ImageDraw.Draw(base)
            d.polygon([
                (center[0] - 4.8 * S, center[1] - spec.torso_h * 0.42 * S),
                (center[0] + 1.8 * S, center[1] - 2.0 * S),
                (center[0] - 2.0 * S, center[1] + spec.torso_h * 0.38 * S),
                (center[0] - 8.5 * S, center[1] + spec.torso_h * 0.38 * S),
            ], fill=pal["outfit_dark"], outline=outline)
            d.ellipse(_bbox((center[0] + 4.0 * S, center[1] - 2.0 * S), 5.8 * S, 6.0 * S), fill=pal["accent"], outline=outline, width=max(1, int(1.0 * S)))
        elif spec.outfit == "general_uniform":
            d = ImageDraw.Draw(base)
            jacket = [
                (center[0] - spec.shoulder_w * 0.62 * S, center[1] - spec.torso_h * 0.52 * S),
                (center[0] + spec.shoulder_w * 0.54 * S, center[1] - spec.torso_h * 0.48 * S),
                (center[0] + spec.torso_w * 0.56 * S, center[1] + spec.torso_h * 0.34 * S),
                (center[0] + spec.hip_w * 0.42 * S, center[1] + spec.torso_h * 0.52 * S + spec.coat_len * 0.18 * S),
                (center[0] - spec.hip_w * 0.55 * S, center[1] + spec.torso_h * 0.48 * S + spec.coat_len * 0.15 * S),
                (center[0] - spec.torso_w * 0.64 * S, center[1] + spec.torso_h * 0.28 * S),
            ]
            d.polygon(jacket, fill=pal["outfit"], outline=outline)
            # Giant epaulets integrated into the shoulders.
            for sign in (-1, 1):
                ep = (center[0] + sign * spec.shoulder_w * 0.44 * S, center[1] - spec.torso_h * 0.48 * S)
                d.rounded_rectangle((ep[0] - 8.5 * S, ep[1] - 3.2 * S, ep[0] + 8.5 * S, ep[1] + 4.8 * S), radius=3 * S, fill=pal["accent"], outline=outline, width=max(1, int(1.0 * S)))
                for k in range(3):
                    x = ep[0] + sign * (2.0 + k * 2.6) * S
                    d.line([(x, ep[1] + 4.0 * S), (x + sign * 2.4 * S, ep[1] + 9.0 * S)], fill=pal["accent_dark"], width=max(1, int(0.9 * S)))
            # Double-breasted panels and ribbon sash.
            d.polygon([
                (center[0] - 8.0 * S, center[1] - spec.torso_h * 0.46 * S),
                (center[0] + 4.0 * S, center[1] - spec.torso_h * 0.40 * S),
                (center[0] + 11.0 * S, center[1] + spec.torso_h * 0.48 * S),
                (center[0] - 2.0 * S, center[1] + spec.torso_h * 0.44 * S),
            ], fill=pal["outfit_dark"], outline=outline)
            d.polygon([
                (center[0] - 13.0 * S, center[1] - spec.torso_h * 0.43 * S),
                (center[0] - 6.0 * S, center[1] - spec.torso_h * 0.49 * S),
                (center[0] + 13.0 * S, center[1] + spec.torso_h * 0.36 * S),
                (center[0] + 6.0 * S, center[1] + spec.torso_h * 0.43 * S),
            ], fill=pal["accent"], outline=outline)
            for row in range(3):
                for col in range(2):
                    x = center[0] + (2.5 + col * 7.0) * S
                    y = center[1] - 5.0 * S + row * 6.0 * S
                    d.ellipse(_bbox((x, y), 3.4 * S, 3.4 * S), fill=pal["accent"], outline=outline, width=max(1, int(0.9 * S)))
            # One big chest star plus too many awards.
            star_c = (center[0] - 7.0 * S, center[1] - 1.0 * S)
            star = []
            for i in range(10):
                r = (4.7 if i % 2 == 0 else 2.0) * S
                a = -math.pi / 2 + i * math.tau / 10
                star.append((star_c[0] + math.cos(a) * r, star_c[1] + math.sin(a) * r))
            d.polygon(star, fill=pal["accent"], outline=outline)
            for i, color in enumerate([pal["accent"], pal["accent_dark"], pal["white"], pal["accent"]]):
                x = center[0] - 12.0 * S + i * 4.5 * S
                y = center[1] + 8.5 * S
                d.rectangle((x, y, x + 3.2 * S, y + 5.0 * S), fill=color, outline=outline, width=max(1, int(0.8 * S)))
                d.ellipse(_bbox((x + 1.6 * S, y + 6.3 * S), 3.2 * S, 3.2 * S), fill=pal["accent"], outline=outline, width=max(1, int(0.8 * S)))
        elif spec.outfit == "storm_uniform":
            d = ImageDraw.Draw(base)
            tunic = [
                (center[0] - spec.shoulder_w * 0.58 * S, center[1] - spec.torso_h * 0.48 * S),
                (center[0] + spec.shoulder_w * 0.48 * S, center[1] - spec.torso_h * 0.42 * S),
                (center[0] + spec.torso_w * 0.42 * S, center[1] + spec.torso_h * 0.08 * S),
                (center[0] + spec.hip_w * 0.38 * S, center[1] + spec.torso_h * 0.46 * S + spec.coat_len * 0.30 * S),
                (center[0] - spec.hip_w * 0.42 * S, center[1] + spec.torso_h * 0.42 * S + spec.coat_len * 0.24 * S),
                (center[0] - spec.torso_w * 0.42 * S, center[1] + spec.torso_h * 0.02 * S),
            ]
            d.polygon(tunic, fill=pal["outfit"], outline=outline)
            d.polygon([
                (center[0] - 5.5 * S, center[1] - spec.torso_h * 0.42 * S),
                (center[0] + 1.0 * S, center[1] - 1.0 * S),
                (center[0] - 3.5 * S, center[1] + spec.torso_h * 0.28 * S),
                (center[0] - 7.2 * S, center[1] + spec.torso_h * 0.24 * S),
            ], fill=pal["outfit_dark"], outline=outline)
            d.rounded_rectangle((center[0] - 13.0 * S, center[1] - 4.0 * S, center[0] + 10.0 * S, center[1] + 1.8 * S), radius=2.0 * S, fill=pal["outfit_dark"], outline=outline, width=max(1, int(0.9 * S)))
            d.rounded_rectangle((center[0] - 11.0 * S, center[1] - 2.8 * S, center[0] - 3.5 * S, center[1] + 8.8 * S), radius=2.0 * S, fill=_scale_color(pal["outfit"], 1.06), outline=outline, width=max(1, int(0.9 * S)))
            d.rounded_rectangle((center[0] + 1.0 * S, center[1] - 1.9 * S, center[0] + 8.0 * S, center[1] + 9.2 * S), radius=2.0 * S, fill=_scale_color(pal["outfit"], 1.06), outline=outline, width=max(1, int(0.9 * S)))
            d.line([(center[0] - 2.0 * S, center[1] - spec.torso_h * 0.46 * S), (center[0] - 2.0 * S, center[1] + spec.torso_h * 0.48 * S)], fill=with_alpha(pal["white"], 210), width=max(1, int(0.9 * S)))
            d.polygon([
                (center[0] - 11.5 * S, center[1] - spec.torso_h * 0.50 * S),
                (center[0] - 3.8 * S, center[1] - spec.torso_h * 0.38 * S),
                (center[0] - 8.3 * S, center[1] - spec.torso_h * 0.16 * S),
            ], fill=pal["outfit_dark"], outline=outline)
            d.polygon([
                (center[0] - 3.0 * S, center[1] - spec.torso_h * 0.42 * S),
                (center[0] + 5.8 * S, center[1] - spec.torso_h * 0.34 * S),
                (center[0] + 1.5 * S, center[1] - spec.torso_h * 0.14 * S),
            ], fill=pal["outfit_dark"], outline=outline)
            d.ellipse(_bbox((center[0] - 7.9 * S, center[1] - spec.torso_h * 0.30 * S), 3.0 * S, 3.0 * S), fill=pal["white"], outline=outline, width=max(1, int(0.8 * S)))
            d.ellipse(_bbox((center[0] + 0.9 * S, center[1] - spec.torso_h * 0.26 * S), 3.0 * S, 3.0 * S), fill=pal["white"], outline=outline, width=max(1, int(0.8 * S)))
            d.ellipse(_bbox((center[0] - 8.4 * S, center[1] - spec.torso_h * 0.31 * S), 0.8 * S, 0.8 * S), fill=outline)
            d.ellipse(_bbox((center[0] - 7.3 * S, center[1] - spec.torso_h * 0.31 * S), 0.8 * S, 0.8 * S), fill=outline)
            d.rectangle((center[0] - 8.8 * S, center[1] - spec.torso_h * 0.28 * S, center[0] - 6.8 * S, center[1] - spec.torso_h * 0.10 * S), fill=outline)
            d.ellipse(_bbox((center[0] + 0.4 * S, center[1] - spec.torso_h * 0.27 * S), 0.8 * S, 0.8 * S), fill=outline)
            d.ellipse(_bbox((center[0] + 1.5 * S, center[1] - spec.torso_h * 0.27 * S), 0.8 * S, 0.8 * S), fill=outline)
            d.rectangle((center[0] - 0.1 * S, center[1] - spec.torso_h * 0.24 * S, center[0] + 1.9 * S, center[1] - spec.torso_h * 0.06 * S), fill=outline)
        elif spec.outfit == "poncho":
            d = ImageDraw.Draw(base)
            shawl = [
                (center[0] - spec.shoulder_w * 0.70 * S, center[1] - spec.torso_h * 0.48 * S),
                (center[0] + spec.shoulder_w * 0.60 * S, center[1] - spec.torso_h * 0.22 * S),
                (center[0] + spec.hip_w * 0.40 * S, center[1] + spec.torso_h * 0.40 * S + spec.cape_len * 0.30 * S),
                (center[0] - spec.hip_w * 0.70 * S, center[1] + spec.torso_h * 0.30 * S + spec.cape_len * 0.52 * S),
            ]
            d.polygon(shawl, fill=pal["outfit"], outline=outline)
            d.polygon([
                (center[0] - 5.0 * S, center[1] - spec.torso_h * 0.46 * S),
                (center[0] + 12.0 * S, center[1] - spec.torso_h * 0.18 * S),
                (center[0] + 1.0 * S, center[1] + spec.torso_h * 0.52 * S),
            ], fill=pal["accent"], outline=outline)
            d.rounded_rectangle((center[0] - 6.0 * S, center[1] - 4.0 * S, center[0] + 4.0 * S, center[1] + 14.0 * S), radius=3 * S, fill=pal["outfit_dark"], outline=outline, width=max(1, int(1.0 * S)))
        elif spec.outfit == "apron":
            d = ImageDraw.Draw(base)
            d.ellipse(_bbox((center[0] - 1.0 * S, center[1] + 2.0 * S), spec.torso_w * 1.18 * S, spec.torso_h * 1.20 * S), fill=pal["outfit"], outline=outline, width=max(1, int(1.2 * S)))
            d.rounded_rectangle((center[0] - 5.0 * S, center[1] - 3.5 * S, center[0] + 9.0 * S, center[1] + spec.torso_h * 0.58 * S), radius=3 * S, fill=pal["accent"], outline=outline, width=max(1, int(1.0 * S)))
            d.line([(center[0] - 8.0 * S, center[1] - 6.0 * S), (center[0] + 8.0 * S, center[1] - 1.0 * S)], fill=outline, width=max(1, int(1.0 * S)))
        elif spec.outfit == "keeper_robe":
            d = ImageDraw.Draw(base)
            robe = [
                (center[0] - spec.shoulder_w * 0.72 * S, center[1] - spec.torso_h * 0.50 * S),
                (center[0] + spec.shoulder_w * 0.58 * S, center[1] - spec.torso_h * 0.42 * S),
                (center[0] + spec.hip_w * 0.46 * S, center[1] + spec.torso_h * 0.44 * S + spec.coat_len * 0.38 * S),
                (center[0] - spec.hip_w * 0.62 * S, center[1] + spec.torso_h * 0.44 * S + spec.coat_len * 0.32 * S),
            ]
            d.polygon(robe, fill=pal["outfit"], outline=outline)
            collar = [
                (center[0] - 8.0 * S, center[1] - spec.torso_h * 0.44 * S),
                (center[0] + 6.0 * S, center[1] - spec.torso_h * 0.38 * S),
                (center[0] + 12.5 * S, center[1] - 2.0 * S),
                (center[0] + 2.5 * S, center[1] + 5.0 * S),
                (center[0] - 10.0 * S, center[1] + 1.0 * S),
            ]
            d.polygon(collar, fill=pal["accent"], outline=outline)
        elif spec.outfit == "long_coat":
            d = ImageDraw.Draw(base)
            coat = [
                (center[0] - spec.shoulder_w * 0.46 * S, center[1] - spec.torso_h * 0.48 * S),
                (center[0] + spec.shoulder_w * 0.28 * S, center[1] - spec.torso_h * 0.44 * S),
                (center[0] + spec.torso_w * 0.42 * S, center[1] + spec.torso_h * 0.04 * S),
                (center[0] + spec.hip_w * 0.38 * S, center[1] + spec.torso_h * 0.48 * S + spec.coat_len * 0.45 * S),
                (center[0] + 1.0 * S, center[1] + spec.torso_h * 0.32 * S + spec.coat_len * 0.18 * S),
                (center[0] - spec.hip_w * 0.24 * S, center[1] + spec.torso_h * 0.48 * S + spec.coat_len * 0.52 * S),
                (center[0] - spec.hip_w * 0.40 * S, center[1] + spec.torso_h * 0.46 * S),
            ]
            d.polygon(coat, fill=pal["outfit"], outline=outline)
            d.polygon([
                (center[0] - 4.8 * S, center[1] - spec.torso_h * 0.42 * S),
                (center[0] + 2.4 * S, center[1] - 2.0 * S),
                (center[0] - 2.0 * S, center[1] + spec.torso_h * 0.50 * S),
                (center[0] - 7.4 * S, center[1] + spec.torso_h * 0.46 * S),
            ], fill=pal["outfit_dark"], outline=outline)
        # accessory overlays that belong to the silhouette, not random doodads.
        d = ImageDraw.Draw(base)
        if spec.accessory == "scarf":
            d.polygon([
                (center[0] - 5 * S, center[1] - spec.torso_h * 0.40 * S),
                (center[0] + 7 * S, center[1] - spec.torso_h * 0.34 * S),
                (center[0] + 5 * S, center[1] - spec.torso_h * 0.10 * S),
                (center[0] - 4 * S, center[1] - spec.torso_h * 0.16 * S),
            ], fill=pal["accent"], outline=outline)
            d.polygon([(center[0] + 1 * S, center[1] - 2 * S), (center[0] + 10 * S, center[1] + 11 * S), (center[0] + 5 * S, center[1] + 12 * S), (center[0] - 1 * S, center[1] + 4 * S)], fill=pal["accent_dark"], outline=outline)
        elif spec.accessory == "shawl":
            d.polygon([
                (center[0] - 2.5 * S, center[1] - spec.torso_h * 0.44 * S),
                (center[0] + 9.5 * S, center[1] - spec.torso_h * 0.22 * S),
                (center[0] + 2.0 * S, center[1] + 4.0 * S),
            ], fill=pal["accent"], outline=outline)
        elif spec.accessory == "satchel" and spec.satchel_size > 0:
            draw_rotated_rounded_rect(base, (center[0] - 8 * S, center[1] + 6 * S), (spec.satchel_size * 1.05 * S, spec.satchel_size * 0.9 * S), 8, 2.0 * S, pal["outfit_dark"], outline, 1.0 * S)
            d.line([(center[0] - 2 * S, center[1] - 7 * S), (center[0] - 8 * S, center[1] + 2 * S)], fill=outline, width=max(1, int(1.0 * S)))
        elif spec.accessory == "keys":
            d.line([(center[0] - 3 * S, center[1] + 10 * S), (center[0] + 5 * S, center[1] + 12 * S)], fill=outline, width=max(1, int(1.0 * S)))
            d.ellipse(_bbox((center[0] + 3.0 * S, center[1] + 12.0 * S), 3.2 * S, 3.2 * S), fill=pal["accent"], outline=outline, width=max(1, int(1.0 * S)))
        elif spec.accessory == "medals":
            # Small ordered rows; still excessive, but less visually muddy than
            # the first pass medal blob.
            for row in range(2):
                for col in range(3):
                    x = center[0] - 13.8 * S + col * 5.2 * S
                    y = center[1] + (2.0 + row * 5.8 + (col % 2) * 0.8) * S
                    ribbon = pal["accent_dark"] if (row + col) % 2 else pal["accent"]
                    d.rectangle((x - 1.1 * S, y - 4.0 * S, x + 1.1 * S, y), fill=ribbon, outline=outline, width=max(1, int(0.65 * S)))
                    d.ellipse(_bbox((x, y + 2.0 * S), 3.2 * S, 3.2 * S), fill=pal["accent"], outline=outline, width=max(1, int(0.75 * S)))

    def _draw_prop(self, base: Image.Image, hand: Point, spec: ToonSpec, pal: Dict[str, Color], S: float, angle: float) -> None:
        outline = pal["outline"]
        prop = spec.prop
        if prop == "blade":
            d = ImageDraw.Draw(base)
            tip = add(hand, vec(22.0 * S, angle - 8.0))
            guard = add(hand, vec(6.0 * S, angle + 90.0))
            guard2 = add(hand, vec(6.0 * S, angle - 90.0))
            d.line([hand, tip], fill=pal["white"], width=max(1, int(2.0 * S)))
            d.line([guard, guard2], fill=pal["accent"], width=max(1, int(2.0 * S)))
            d.line([hand, add(hand, vec(8.0 * S, angle + 180.0))], fill=pal["outfit_dark"], width=max(1, int(2.0 * S)))
        elif prop == "baton":
            d = ImageDraw.Draw(base)
            baton_angle = angle - 8.0
            tip = add(hand, vec(32.0 * S, baton_angle))
            base_pt = add(hand, vec(5.0 * S, baton_angle + 180.0))
            grip = add(hand, vec(2.0 * S, baton_angle + 180.0))
            d.line([base_pt, tip], fill=outline, width=max(1, int(4.0 * S)))
            d.line([base_pt, tip], fill=pal["accent_dark"], width=max(1, int(2.1 * S)))
            d.line([grip, add(grip, vec(7.0 * S, baton_angle))], fill=pal["hair"], width=max(1, int(2.6 * S)))
            d.ellipse(_bbox(tip, 5.2 * S, 5.2 * S), fill=pal["accent"], outline=outline, width=max(1, int(1.0 * S)))
            d.ellipse(_bbox(base_pt, 3.5 * S, 3.5 * S), fill=pal["accent"], outline=outline, width=max(1, int(0.9 * S)))
        elif prop == "rifle":
            d = ImageDraw.Draw(base)
            rifle_angle = angle - 6.0
            butt = add(hand, vec(5.0 * S, rifle_angle + 180.0))
            muzzle = add(hand, vec(34.0 * S, rifle_angle))
            d.line([butt, muzzle], fill=outline, width=max(1, int(4.4 * S)))
            d.line([butt, muzzle], fill=pal["outfit_dark"], width=max(1, int(2.6 * S)))
            stock_back = add(butt, vec(7.0 * S, rifle_angle + 150.0))
            stock_low = add(butt, vec(5.0 * S, rifle_angle + 218.0))
            d.polygon([stock_back, butt, stock_low], fill=pal["outfit"], outline=outline)
            mag_a = add(hand, vec(6.0 * S, rifle_angle + 90.0))
            mag_b = add(hand, vec(10.0 * S, rifle_angle + 90.0))
            mag_c = add(hand, vec(11.0 * S, rifle_angle + 148.0))
            mag_d = add(hand, vec(7.0 * S, rifle_angle + 148.0))
            d.polygon([mag_a, mag_b, mag_c, mag_d], fill=pal["accent_dark"], outline=outline)
            bayonet_base = add(muzzle, vec(0.0 * S, rifle_angle + 90.0))
            bayonet_tip = add(muzzle, vec(7.0 * S, rifle_angle - 4.0))
            bayonet_low = add(muzzle, vec(1.2 * S, rifle_angle - 74.0))
            d.polygon([bayonet_base, bayonet_tip, bayonet_low], fill=pal["white"], outline=outline)
        elif prop == "tablet":
            draw_rotated_rounded_rect(base, add(hand, vec(8.0 * S, angle - 10.0)), (10.0 * S, 14.0 * S), angle - 12.0, 2.0 * S, pal["outfit_dark"], outline, 1.0 * S)
            d = ImageDraw.Draw(base)
            d.line([add(hand, vec(4 * S, angle - 40)), add(hand, vec(10 * S, angle - 40))], fill=pal["accent"], width=max(1, int(1.0 * S)))
        elif prop == "coin_pouch":
            draw_rotated_ellipse(base, add(hand, vec(5.0 * S, angle - 10.0)), (9.0 * S, 11.0 * S), angle, pal["accent_dark"], outline, 1.0 * S)
            ImageDraw.Draw(base).line([add(hand, vec(4 * S, angle + 150)), add(hand, vec(7 * S, angle - 30))], fill=pal["accent"], width=max(1, int(1.0 * S)))
        elif prop == "ledger":
            draw_rotated_rounded_rect(base, add(hand, vec(7.0 * S, angle - 6.0)), (11.0 * S, 14.0 * S), angle - 8.0, 2.0 * S, pal["accent"], outline, 1.0 * S)
            d = ImageDraw.Draw(base)
            for i in range(3):
                yoff = -3 + i * 3
                d.line([add(hand, vec(2.0 * S, angle - 45)) , add(hand, vec(8.0 * S, angle - 45))], fill=pal["outfit_dark"], width=max(1, int(0.9 * S)))
        elif prop == "blueprint":
            draw_rotated_rounded_rect(base, add(hand, vec(10.0 * S, angle - 4.0)), (15.0 * S, 5.0 * S), angle - 4.0, 2.0 * S, pal["white"], outline, 1.0 * S)
            ImageDraw.Draw(base).line([add(hand, vec(4 * S, angle - 20)), add(hand, vec(12 * S, angle - 20))], fill=pal["accent_dark"], width=max(1, int(1.0 * S)))

    def render_animation_frame(
        self,
        spec: ToonSpec,
        animation: str,
        frame_index: int,
        frame_count: int,
        size: Tuple[int, int],
        *,
        background: Optional[Color] = None,
        supersample: int = 4,
        downsample: str = "lanczos",
    ) -> Image.Image:
        W, H = size
        ss = max(1, int(supersample))
        img = Image.new("RGBA", (W * ss, H * ss), background or (0, 0, 0, 0))
        d = ImageDraw.Draw(img)
        S = (W / 128.0) * ss
        pal = self._palette(spec)
        p = self.pose_for_animation(animation, frame_index, frame_count, spec)
        shift = self._body_plan_shift(spec)

        feet_base = (44.0 * S + p.root_x * S, 102.0 * S + p.root_y * S)
        hip_center = (44.0 * S + p.root_x * S + p.lean * S, 74.0 * S + p.root_y * S - p.body_bob * S + shift["hip_y"] * S)
        torso_center = (hip_center[0] + 0.5 * S, hip_center[1] - spec.torso_h * 0.52 * S + shift["shoulder_y"] * S)
        head_center = (torso_center[0] + 4.0 * S, torso_center[1] - spec.torso_h * 0.62 * S - spec.neck_h * S + shift["head_y"] * S)

        shadow_w = max(spec.shoulder_w * 1.8, spec.torso_w * 2.0)
        self._draw_shadow(d, (46 * S + p.root_x * S, 106 * S), shadow_w, S, 34 if animation != "dash" else 24)
        if p.dash > 0.0:
            trail = Image.new("RGBA", img.size, (0, 0, 0, 0))
            trail_d = ImageDraw.Draw(trail)
            for i, alpha in enumerate([55, 32, 18]):
                xoff = (i + 1) * 6.0 * S
                trail_d.rounded_rectangle((torso_center[0] - 18*S - xoff, torso_center[1] - 10*S, torso_center[0] + 12*S - xoff, torso_center[1] + 16*S), radius=6*S, fill=with_alpha(pal["accent"], alpha))
            img.alpha_composite(trail)

        def leg_points(is_near: bool):
            sign = 1.0 if is_near else -1.0
            upper = p.near_leg_upper if is_near else p.far_leg_upper
            lower = p.near_leg_lower if is_near else p.far_leg_lower
            hip_spread = (spec.hip_w * 0.26 if spec.outfit in {"general_uniform", "storm_uniform"} else 2.2) * S
            hip = (hip_center[0] + sign * hip_spread, hip_center[1] + 3.0 * S)
            knee = add(hip, vec(spec.leg_upper * S, upper + p.torso_tilt * 0.08))
            ankle = add(knee, vec(spec.leg_lower * S, lower + p.torso_tilt * 0.08))
            return hip, knee, ankle

        def arm_points(is_near: bool):
            sign = 1.0 if is_near else -1.0
            shoulder = (torso_center[0] + sign * (spec.shoulder_w * 0.32 * S), torso_center[1] - spec.torso_h * 0.22 * S)
            upper = p.near_arm_upper if is_near else p.far_arm_upper
            lower = p.near_arm_lower if is_near else p.far_arm_lower
            elbow = add(shoulder, vec(spec.arm_upper * S, upper + p.torso_tilt * 0.15))
            hand = add(elbow, vec(spec.arm_lower * S, lower + p.torso_tilt * 0.12))
            return shoulder, elbow, hand

        def draw_uniform_cuff(elbow: Point, hand: Point, *, scale: float = 1.0) -> None:
            """Draw a short yellow wrist band at the sleeve/hand boundary."""
            if spec.outfit != "general_uniform":
                return
            angle = math.degrees(math.atan2(hand[1] - elbow[1], hand[0] - elbow[0]))
            # Place the cuff just before the skin-toned hand so it reads as the
            # yellow trim at the end of the green sleeve, not as a bracelet.
            cuff_center = add(hand, vec((spec.hand_r * -0.58 * scale) * S, angle))
            draw_rotated_rounded_rect(
                img,
                cuff_center,
                (4.8 * scale * S, spec.arm_radius * 2.85 * scale * S),
                angle,
                2.0 * scale * S,
                pal["accent"],
                pal["outline"],
                0.9 * scale * S,
            )
            # A small darker trailing edge keeps the cuff from becoming a flat
            # yellow blob when the arm is anti-aliased down to runtime size.
            edge_center = add(cuff_center, vec(1.65 * scale * S, angle + 180.0))
            draw_rotated_rounded_rect(
                img,
                edge_center,
                (1.3 * scale * S, spec.arm_radius * 2.45 * scale * S),
                angle,
                0.8 * scale * S,
                pal["accent_dark"],
                None,
                0.0,
            )

        def draw_skin_hand(hand: Point, *, scale: float = 1.0, outline_width: float = 1.0) -> None:
            """Draw the terminal hand circle large enough to cover the sleeve cap."""
            diameter = spec.hand_r * scale * S
            if spec.outfit == "general_uniform":
                # For the general, hand_r behaves like a radius: the green
                # sleeve capsule already draws a rounded terminal cap, so the
                # skin hand must be a full ball on top of that cap rather than
                # a tiny dot at the wrist.
                diameter *= 2.0
            d.ellipse(
                _bbox(hand, diameter, diameter),
                fill=pal["skin"],
                outline=pal["outline"],
                width=max(1, int(outline_width * S)),
            )

        def draw_armband(shoulder: Point, elbow: Point, *, scale: float = 1.0, include_insignia: bool = True) -> None:
            if spec.outfit != "storm_uniform":
                return
            angle = math.degrees(math.atan2(elbow[1] - shoulder[1], elbow[0] - shoulder[0]))
            band_center = (
                shoulder[0] + (elbow[0] - shoulder[0]) * 0.42,
                shoulder[1] + (elbow[1] - shoulder[1]) * 0.42,
            )
            band_w = 7.8 * scale * S
            band_h = spec.arm_radius * 2.45 * scale * S
            draw_rotated_rounded_rect(
                img,
                band_center,
                (band_w, band_h),
                angle,
                1.2 * scale * S,
                pal["accent"],
                pal["outline"],
                0.9 * scale * S,
            )
            if not include_insignia:
                return
            disc_center = band_center
            draw_rotated_ellipse(
                img,
                disc_center,
                (4.0 * scale * S, 4.0 * scale * S),
                angle,
                pal["white"],
                pal["outline"],
                0.8 * scale * S,
            )
            layer_w = max(8, int(10.0 * scale * S))
            layer_h = max(8, int(10.0 * scale * S))
            layer = Image.new("RGBA", (layer_w, layer_h), (0, 0, 0, 0))
            ld = ImageDraw.Draw(layer)
            cx = layer_w / 2.0
            cy = layer_h / 2.0
            ld.polygon([
                (cx - 2.0 * scale * S, cy - 2.0 * scale * S),
                (cx + 0.3 * scale * S, cy - 0.2 * scale * S),
                (cx - 0.8 * scale * S, cy + 2.0 * scale * S),
                (cx - 2.8 * scale * S, cy + 0.4 * scale * S),
            ], fill=pal["outline"])
            ld.polygon([
                (cx + 0.2 * scale * S, cy - 2.0 * scale * S),
                (cx + 2.6 * scale * S, cy - 0.4 * scale * S),
                (cx + 0.9 * scale * S, cy + 2.2 * scale * S),
                (cx - 0.4 * scale * S, cy + 0.2 * scale * S),
            ], fill=pal["outline"])
            _paste_rotated_local(img, layer, disc_center, angle)

        # far limbs first
        far_hip, far_knee, far_ankle = leg_points(False)
        far_tint = _scale_color(pal["outfit_dark"], 0.93)
        draw_capsule(d, far_hip, far_knee, spec.leg_radius * 0.92 * S, far_tint, pal["outline"], 1.1 * S)
        draw_capsule(d, far_knee, far_ankle, spec.leg_radius * 0.88 * S, far_tint, pal["outline"], 1.1 * S)
        draw_rotated_rounded_rect(img, (far_ankle[0] + spec.foot_w * 0.25 * S, far_ankle[1] + 2.0 * S), (spec.foot_w * S, spec.foot_h * S), -2.0 + p.torso_tilt * 0.08, spec.foot_h * 0.48 * S, pal["shoe"], pal["outline"], 1.0 * S)
        far_shoulder, far_elbow, far_hand = arm_points(False)
        draw_capsule(d, far_shoulder, far_elbow, spec.arm_radius * 0.92 * S, far_tint, pal["outline"], 1.1 * S)
        draw_capsule(d, far_elbow, far_hand, spec.arm_radius * 0.88 * S, far_tint, pal["outline"], 1.1 * S)
        draw_armband(far_shoulder, far_elbow, scale=0.88, include_insignia=False)
        draw_uniform_cuff(far_elbow, far_hand, scale=0.88)
        # Keep sleeves uniform-colored, but hands skin-toned. The far hand is
        # drawn before the torso so it still sits behind the body volume.
        draw_skin_hand(far_hand, scale=0.90, outline_width=0.9)

        # torso/head core silhouette
        self._draw_torso(img, torso_center, spec, pal, S, p)
        self._draw_head(img, head_center, spec, pal, S, p)

        # near limbs and props
        near_hip, near_knee, near_ankle = leg_points(True)
        near_tint = pal["outfit"]
        draw_capsule(d, near_hip, near_knee, spec.leg_radius * S, near_tint, pal["outline"], 1.15 * S)
        draw_capsule(d, near_knee, near_ankle, spec.leg_radius * 0.96 * S, near_tint, pal["outline"], 1.15 * S)
        draw_rotated_rounded_rect(img, (near_ankle[0] + spec.foot_w * 0.28 * S, near_ankle[1] + 2.0 * S), (spec.foot_w * S, spec.foot_h * S), 2.0 + p.torso_tilt * 0.10, spec.foot_h * 0.48 * S, pal["shoe"], pal["outline"], 1.0 * S)
        near_shoulder, near_elbow, near_hand = arm_points(True)
        sleeve_fill = pal["outfit"] if spec.outfit in {"poncho", "keeper_robe", "long_coat", "general_uniform", "storm_uniform"} else pal["skin"]
        draw_capsule(d, near_shoulder, near_elbow, spec.arm_radius * S, sleeve_fill, pal["outline"], 1.1 * S)
        draw_capsule(d, near_elbow, near_hand, spec.arm_radius * 0.95 * S, sleeve_fill, pal["outline"], 1.1 * S)
        draw_armband(near_shoulder, near_elbow, scale=1.0, include_insignia=True)
        draw_uniform_cuff(near_elbow, near_hand, scale=1.0)

        prop_angle = p.near_arm_lower + p.torso_tilt * 0.10 + (14.0 if p.prop_swing > 0 else 0.0)
        self._draw_prop(img, near_hand, spec, pal, S, prop_angle)
        # Draw the near hand last so the baton/prop reads as being held by a
        # skin-toned hand instead of painting over it.
        draw_skin_hand(near_hand, scale=1.0, outline_width=1.0)
        if p.slash > 0.0:
            d.arc((near_hand[0] - 4 * S, near_hand[1] - 28 * S, near_hand[0] + 42 * S, near_hand[1] + 16 * S), start=-70, end=35, fill=with_alpha(pal["accent"], 160), width=max(1, int(2.5 * S)))
        if p.hit > 0.0:
            for off in [(-5, -10), (4, -14), (10, -6)]:
                d.line([(head_center[0] + off[0]*S, head_center[1] + off[1]*S), (head_center[0] + (off[0]+3)*S, head_center[1] + (off[1]-4)*S)], fill=with_alpha(pal["accent"], 180), width=max(1, int(1.2 * S)))
        if ss > 1:
            img = img.resize((W, H), RESAMPLING.LANCZOS if downsample == "lanczos" else RESAMPLING.BICUBIC)
        return img
