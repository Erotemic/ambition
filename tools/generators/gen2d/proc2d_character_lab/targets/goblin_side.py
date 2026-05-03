from __future__ import annotations

"""Cute side-view goblin target for side-scrolling games."""

import math
import random
from dataclasses import asdict, dataclass
from typing import Dict, List, Optional, Sequence, Tuple

from PIL import Image, ImageChops, ImageColor, ImageDraw, ImageFilter

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


def ease_in_out_sine(t: float) -> float:
    t = clamp(t, 0.0, 1.0)
    return -(math.cos(math.pi * t) - 1.0) / 2.0


def ease_out_cubic(t: float) -> float:
    t = clamp(t, 0.0, 1.0)
    return 1.0 - (1.0 - t) ** 3


def rgba(value: str, alpha: int = 255) -> Color:
    r, g, b = ImageColor.getrgb(value)
    return (r, g, b, alpha)


def lighten(color: Color, amount: float) -> Color:
    r, g, b, a = color
    return (
        int(lerp(r, 255, amount)),
        int(lerp(g, 255, amount)),
        int(lerp(b, 255, amount)),
        a,
    )


def darken(color: Color, amount: float) -> Color:
    r, g, b, a = color
    return (int(r * (1 - amount)), int(g * (1 - amount)), int(b * (1 - amount)), a)


def with_alpha(color: Color, alpha: int) -> Color:
    return (color[0], color[1], color[2], alpha)


def parse_background(value: str) -> Optional[Color]:
    return None if value.lower() == "transparent" else rgba(value)


def add(a: Point, b: Point) -> Point:
    return (a[0] + b[0], a[1] + b[1])


def vec(length: float, degrees: float) -> Point:
    a = math.radians(degrees)
    return (math.cos(a) * length, math.sin(a) * length)


def interp(a: Point, b: Point, t: float) -> Point:
    return (lerp(a[0], b[0], t), lerp(a[1], b[1], t))


def bbox_from_center(center: Point, w: float, h: float) -> Tuple[float, float, float, float]:
    return (center[0] - w / 2.0, center[1] - h / 2.0, center[0] + w / 2.0, center[1] + h / 2.0)


def solidify_layer_alpha(layer: Image.Image, opaque_mask: Image.Image) -> None:
    layer.putalpha(ImageChops.lighter(layer.getchannel("A"), opaque_mask))


@dataclass
class GoblinPalette:
    skin: Color
    skin_top: Color
    skin_shadow: Color
    belly: Color
    belly_shadow: Color
    cloth: Color
    cloth_dark: Color
    eye: Color
    eye_glow: Color
    outline: Color
    mouth: Color
    tooth: Color
    weapon: Color
    weapon_dark: Color
    metal: Color
    shadow: Color


@dataclass
class GoblinSpec:
    target: str
    seed: int
    archetype: str
    held_item: str
    palette_name: str
    head_w: float
    head_h: float
    snout_len: float
    ear_w: float
    ear_h: float
    body_w: float
    body_h: float
    arm_upper: float
    arm_lower: float
    leg_upper: float
    leg_lower: float
    hand_r: float
    foot_w: float
    foot_h: float
    eye_w: float
    eye_h: float
    tooth_size: float


@dataclass
class GoblinPose:
    root_x: float = 0.0
    root_y: float = 0.0
    body_bob: float = 0.0
    body_tilt: float = 0.0
    head_tilt: float = 0.0
    crouch: float = 0.0
    far_arm_upper: float = 125.0
    far_arm_lower: float = 145.0
    near_arm_upper: float = 30.0
    near_arm_lower: float = 18.0
    far_leg_upper: float = 88.0
    far_leg_lower: float = 98.0
    near_leg_upper: float = 60.0
    near_leg_lower: float = 82.0
    blink: bool = False
    eye_squint: float = 0.0
    slash: float = 0.0
    slash_arc: float = 0.0
    recoil: float = 0.0
    collapse: float = 0.0
    dead: bool = False


class SideGoblinGenerator:
    name = "goblin"

    SPRITESHEET_ANIMATIONS: Dict[str, Dict[str, int]] = {
        "idle": {"frames": 8, "duration_ms": 120},
        "walk": {"frames": 8, "duration_ms": 95},
        "run": {"frames": 8, "duration_ms": 75},
        "jump": {"frames": 6, "duration_ms": 95},
        "fall": {"frames": 6, "duration_ms": 95},
        "slash": {"frames": 7, "duration_ms": 75},
        "hit": {"frames": 5, "duration_ms": 90},
        "death": {"frames": 8, "duration_ms": 110},
    }

    PALETTES: Dict[str, GoblinPalette] = {
        "classic": GoblinPalette(
            skin=rgba("#6FAE4A"),
            skin_top=rgba("#A6D872"),
            skin_shadow=rgba("#3E6B29"),
            belly=rgba("#8BC061"),
            belly_shadow=rgba("#5B8C3E"),
            cloth=rgba("#6B2B9E"),
            cloth_dark=rgba("#4A1D6F"),
            eye=rgba("#F04DFF"),
            eye_glow=rgba("#FFAEFF"),
            outline=rgba("#141518"),
            mouth=rgba("#2A1C14"),
            tooth=rgba("#F4ECD8"),
            weapon=rgba("#A963F8"),
            weapon_dark=rgba("#6A2CC1"),
            metal=rgba("#E2E3EC"),
            shadow=rgba("#000000", 34),
        ),
        "forest": GoblinPalette(
            skin=rgba("#5B8E49"),
            skin_top=rgba("#8FC06B"),
            skin_shadow=rgba("#35542A"),
            belly=rgba("#72A55A"),
            belly_shadow=rgba("#4E7640"),
            cloth=rgba("#6B2B9E"),
            cloth_dark=rgba("#4A1D6F"),
            eye=rgba("#EF4CFF"),
            eye_glow=rgba("#F39BFF"),
            outline=rgba("#141518"),
            mouth=rgba("#221C23"),
            tooth=rgba("#F2ECDD"),
            weapon=rgba("#B069FF"),
            weapon_dark=rgba("#6A2CC1"),
            metal=rgba("#DDD7EA"),
            shadow=rgba("#000000", 34),
        ),
        "void": GoblinPalette(
            skin=rgba("#45404D"),
            skin_top=rgba("#686074"),
            skin_shadow=rgba("#2E2B35"),
            belly=rgba("#544E61"),
            belly_shadow=rgba("#3F3B48"),
            cloth=rgba("#7D2BA6"),
            cloth_dark=rgba("#5D1E7D"),
            eye=rgba("#EC42FF"),
            eye_glow=rgba("#F078FF"),
            outline=rgba("#16141A"),
            mouth=rgba("#251B28"),
            tooth=rgba("#F5F0E8"),
            weapon=rgba("#A55AF7"),
            weapon_dark=rgba("#6D2AB5"),
            metal=rgba("#DAD8E5"),
            shadow=rgba("#000000", 34),
        ),
    }

    def sample_spec(self, seed: int, archetype: str = "default", held_item: Optional[str] = None) -> GoblinSpec:
        rng = random.Random(seed)
        palette_name = "classic" if archetype == "default" else rng.choice(list(self.PALETTES.keys()))
        if held_item is None:
            held_item = rng.choice(["dagger", "spear", "sword"])
        return GoblinSpec(
            target=self.name,
            seed=seed,
            archetype=archetype,
            held_item=held_item,
            palette_name=palette_name,
            head_w=rng.uniform(28.0, 32.0),
            head_h=rng.uniform(23.0, 26.0),
            snout_len=rng.uniform(6.5, 8.0),
            ear_w=rng.uniform(14.0, 17.0),
            ear_h=rng.uniform(10.0, 13.0),
            body_w=rng.uniform(20.0, 23.0),
            body_h=rng.uniform(17.0, 20.0),
            arm_upper=rng.uniform(11.0, 13.0),
            arm_lower=rng.uniform(10.0, 12.0),
            leg_upper=rng.uniform(11.0, 13.0),
            leg_lower=rng.uniform(10.0, 12.0),
            hand_r=rng.uniform(3.0, 3.8),
            foot_w=rng.uniform(10.0, 12.0),
            foot_h=rng.uniform(4.6, 5.6),
            eye_w=rng.uniform(4.4, 5.4),
            eye_h=rng.uniform(7.0, 8.8),
            tooth_size=rng.uniform(2.4, 3.2),
        )

    def pose_for_animation(self, animation: str, frame_index: int, frame_count: int) -> GoblinPose:
        p = GoblinPose()
        t = 0.0 if frame_count <= 1 else frame_index / float(frame_count - 1)
        wave = math.sin(t * math.tau)

        if animation == "idle":
            bob = abs(wave)
            p.body_bob = bob * 1.2
            p.body_tilt = -2.0 + wave * 1.2
            p.head_tilt = -3.0 + bob * 1.0
            p.far_arm_upper = 136 + wave * 3
            p.far_arm_lower = 152 + wave * 2
            p.near_arm_upper = 28 - wave * 3
            p.near_arm_lower = 18 - wave * 2
            p.blink = frame_index == frame_count // 2
            p.eye_squint = 0.10 if frame_index in {1, frame_count - 2} else 0.0
        elif animation in {"walk", "run"}:
            stride = math.sin(t * math.tau)
            bounce = (1.0 - math.cos(t * math.tau * 2.0)) * 0.5
            amp = 18 if animation == "walk" else 26
            arm_amp = 10 if animation == "walk" else 16
            p.root_x = stride * (1.0 if animation == "walk" else 1.6)
            p.body_bob = 0.6 + bounce * (1.8 if animation == "walk" else 2.5)
            p.body_tilt = -6.0 - (2.0 if animation == "run" else 0.0) - stride * 4.0
            p.head_tilt = -4.0 - bounce * 2.0
            p.far_arm_upper = 140 + stride * arm_amp
            p.far_arm_lower = 152 + stride * (arm_amp * 0.6)
            p.near_arm_upper = 24 - stride * arm_amp
            p.near_arm_lower = 18 - stride * (arm_amp * 0.6)
            p.far_leg_upper = 88 + stride * amp
            p.far_leg_lower = 96 - max(0.0, stride) * 18 + max(0.0, -stride) * 8
            p.near_leg_upper = 60 - stride * amp
            p.near_leg_lower = 82 - max(0.0, -stride) * 18 + max(0.0, stride) * 8
            p.eye_squint = 0.08 + bounce * 0.10
        elif animation == "jump":
            arc = math.sin(t * math.pi)
            lift = ease_in_out_sine(arc)
            p.root_y = -18 * lift
            p.body_tilt = -5.0 + lift * 3.0
            p.head_tilt = -6.0
            p.crouch = 0.4 * (1.0 - lift)
            p.far_arm_upper = 160 - 18 * lift
            p.far_arm_lower = 142 - 12 * lift
            p.near_arm_upper = 12 + 18 * lift
            p.near_arm_lower = 6 + 12 * lift
            p.far_leg_upper = 118
            p.far_leg_lower = 70
            p.near_leg_upper = 86
            p.near_leg_lower = 58
            p.eye_squint = 0.08
        elif animation == "fall":
            p.root_y = -10 + t * 8
            p.body_tilt = 4.0 + 8.0 * t
            p.head_tilt = 2.0
            p.far_arm_upper = 175 - 10 * t
            p.far_arm_lower = 162 - 12 * t
            p.near_arm_upper = 6 + 8 * t
            p.near_arm_lower = 10 + 6 * t
            p.far_leg_upper = 124 - 6 * t
            p.far_leg_lower = 126 - 18 * t
            p.near_leg_upper = 88 - 4 * t
            p.near_leg_lower = 110 - 14 * t
            p.eye_squint = 0.14
        elif animation == "slash":
            wind = 1.0 - smoothstep(clamp(t / 0.32, 0.0, 1.0))
            strike = smoothstep(clamp((t - 0.28) / 0.36, 0.0, 1.0))
            p.root_x = -1.0 * wind + 3.0 * strike
            p.body_tilt = -10.0 * wind + 16.0 * strike
            p.head_tilt = -4.0 + 6.0 * strike
            p.far_arm_upper = 150
            p.far_arm_lower = 164
            p.near_arm_upper = -24 - 18 * wind + 42 * strike
            p.near_arm_lower = -14 - 16 * wind + 30 * strike
            p.far_leg_upper = 96 + 10 * strike
            p.far_leg_lower = 96
            p.near_leg_upper = 54 - 8 * wind
            p.near_leg_lower = 82
            p.slash = max(0.2, wind, strike)
            p.slash_arc = strike
            p.eye_squint = 0.24 + strike * 0.20
        elif animation == "hit":
            j = abs(math.sin(t * math.pi * 2.0))
            p.root_x = -4.0 * j
            p.root_y = 2.0 * j
            p.body_tilt = -16.0 * j
            p.head_tilt = -18.0 * j
            p.far_arm_upper = 175
            p.far_arm_lower = 165
            p.near_arm_upper = 40
            p.near_arm_lower = 55
            p.far_leg_upper = 112
            p.far_leg_lower = 110
            p.near_leg_upper = 86
            p.near_leg_lower = 96
            p.recoil = j
            p.eye_squint = 0.45
        elif animation == "death":
            fall = ease_out_cubic(t)
            p.root_x = lerp(0.0, -10.0, fall)
            p.root_y = lerp(0.0, 14.0, fall)
            p.body_tilt = lerp(0.0, 64.0, fall)
            p.head_tilt = lerp(0.0, 42.0, fall)
            p.far_arm_upper = lerp(145.0, 210.0, fall)
            p.far_arm_lower = lerp(156.0, 236.0, fall)
            p.near_arm_upper = lerp(28.0, 84.0, fall)
            p.near_arm_lower = lerp(18.0, 112.0, fall)
            p.far_leg_upper = lerp(88.0, 146.0, fall)
            p.far_leg_lower = lerp(98.0, 162.0, fall)
            p.near_leg_upper = lerp(60.0, 108.0, fall)
            p.near_leg_lower = lerp(82.0, 136.0, fall)
            p.collapse = fall
            p.dead = True
            p.eye_squint = 0.6
        return p

    def _draw_capsule(self, draw: ImageDraw.ImageDraw, a: Point, b: Point, radius: float, fill: Color, outline: Color, outline_w: int) -> None:
        draw.line([a, b], fill=outline, width=max(1, int(radius * 2 + outline_w * 2)))
        draw.line([a, b], fill=fill, width=max(1, int(radius * 2)))
        for c in [a, b]:
            box = bbox_from_center(c, radius * 2 + outline_w * 2, radius * 2 + outline_w * 2)
            draw.ellipse(box, fill=outline)
            box = bbox_from_center(c, radius * 2, radius * 2)
            draw.ellipse(box, fill=fill)

    def _draw_foot(self, img: Image.Image, ankle: Point, w: float, h: float, fill: Color, outline: Color, outline_w: int) -> None:
        layer = Image.new("RGBA", img.size, (0, 0, 0, 0))
        draw = ImageDraw.Draw(layer)
        opaque_mask = Image.new("L", img.size, 0)
        mdraw = ImageDraw.Draw(opaque_mask)
        center = (ankle[0] + w * 0.30, ankle[1] + h * 0.15)
        outer_box = bbox_from_center(center, w + outline_w * 2, h + outline_w * 2)
        inner_box = bbox_from_center(center, w, h)
        draw.rounded_rectangle(outer_box, radius=h * 0.6, fill=outline)
        draw.rounded_rectangle(inner_box, radius=h * 0.6, fill=fill)
        mdraw.rounded_rectangle(outer_box, radius=h * 0.6, fill=255)
        shine = bbox_from_center((center[0] - w * 0.12, center[1] - h * 0.10), w * 0.45, h * 0.38)
        draw.ellipse(shine, fill=with_alpha(lighten(fill, 0.15), 150))
        solidify_layer_alpha(layer, opaque_mask)
        img.alpha_composite(layer)

    def _draw_body(self, img: Image.Image, center: Point, spec: GoblinSpec, pal: GoblinPalette, px: float) -> None:
        layer = Image.new("RGBA", img.size, (0, 0, 0, 0))
        draw = ImageDraw.Draw(layer)
        opaque_mask = Image.new("L", img.size, 0)
        mdraw = ImageDraw.Draw(opaque_mask)
        outline_w = max(1, int(px * 0.20))
        body_box = bbox_from_center(center, spec.body_w * px, spec.body_h * px)
        outer_body_box = (body_box[0] - outline_w, body_box[1] - outline_w, body_box[2] + outline_w, body_box[3] + outline_w)
        outer_radius = spec.body_h * px * 0.38
        inner_radius = spec.body_h * px * 0.38
        draw.rounded_rectangle(outer_body_box, radius=outer_radius, fill=pal.outline)
        draw.rounded_rectangle(body_box, radius=inner_radius, fill=pal.skin)
        mdraw.rounded_rectangle(outer_body_box, radius=outer_radius, fill=255)
        belly_box = (body_box[0] + spec.body_w * px * 0.08, body_box[1] + spec.body_h * px * 0.22, body_box[2] - spec.body_w * px * 0.22, body_box[3] - spec.body_h * px * 0.10)
        draw.rounded_rectangle(belly_box, radius=spec.body_h * px * 0.26, fill=pal.belly)
        highlight = (body_box[0] + spec.body_w * px * 0.02, body_box[1] + spec.body_h * px * 0.02, body_box[2] - spec.body_w * px * 0.14, body_box[1] + spec.body_h * px * 0.42)
        draw.rounded_rectangle(highlight, radius=spec.body_h * px * 0.20, fill=with_alpha(pal.skin_top, 170))
        shadow = (body_box[0] + spec.body_w * px * 0.16, body_box[1] + spec.body_h * px * 0.48, body_box[2] - spec.body_w * px * 0.04, body_box[3] - spec.body_h * px * 0.04)
        draw.rounded_rectangle(shadow, radius=spec.body_h * px * 0.18, fill=with_alpha(pal.belly_shadow, 155))
        cloth = [
            (center[0] - spec.body_w * px * 0.16, body_box[3] - spec.body_h * px * 0.02),
            (center[0] + spec.body_w * px * 0.06, body_box[3] - spec.body_h * px * 0.02),
            (center[0] + spec.body_w * px * 0.20, body_box[3] + spec.body_h * px * 0.28),
            (center[0] - spec.body_w * px * 0.06, body_box[3] + spec.body_h * px * 0.18),
        ]
        draw.polygon(cloth, fill=pal.cloth, outline=pal.outline)
        draw.line([cloth[0], cloth[2]], fill=pal.cloth_dark, width=max(1, int(px * 0.40)))
        mdraw.polygon(cloth, fill=255)
        solidify_layer_alpha(layer, opaque_mask)
        img.alpha_composite(layer)

    def _draw_head(self, img: Image.Image, center: Point, spec: GoblinSpec, pal: GoblinPalette, px: float, blink: bool, squint: float, dead: bool) -> Point:
        layer = Image.new("RGBA", img.size, (0, 0, 0, 0))
        draw = ImageDraw.Draw(layer)
        opaque_mask = Image.new("L", img.size, 0)
        mdraw = ImageDraw.Draw(opaque_mask)
        outline_w = max(1, int(px * 0.20))
        head_box = bbox_from_center(center, spec.head_w * px, spec.head_h * px)
        outer_head_box = (head_box[0] - outline_w, head_box[1] - outline_w, head_box[2] + outline_w, head_box[3] + outline_w)
        # far ear
        far_ear = [
            (center[0] - spec.head_w * px * 0.18, center[1] - spec.head_h * px * 0.18),
            (center[0] - spec.head_w * px * 0.62, center[1] - spec.ear_h * px * 0.25),
            (center[0] - spec.head_w * px * 0.22, center[1] + spec.ear_h * px * 0.02),
        ]
        draw.polygon(far_ear, fill=darken(pal.skin_shadow, 0.08), outline=pal.outline)
        mdraw.polygon(far_ear, fill=255)
        # head
        draw.ellipse(outer_head_box, fill=pal.outline)
        draw.ellipse(head_box, fill=pal.skin)
        mdraw.ellipse(outer_head_box, fill=255)
        head_hi = (head_box[0] + spec.head_w * px * 0.04, head_box[1] + spec.head_h * px * 0.02, head_box[2] - spec.head_w * px * 0.18, head_box[1] + spec.head_h * px * 0.46)
        draw.ellipse(head_hi, fill=with_alpha(pal.skin_top, 180))
        head_shadow = (head_box[0] + spec.head_w * px * 0.10, head_box[1] + spec.head_h * px * 0.42, head_box[2] - spec.head_w * px * 0.02, head_box[3] - spec.head_h * px * 0.04)
        draw.ellipse(head_shadow, fill=with_alpha(pal.skin_shadow, 150))
        # snout / muzzle
        snout_center = (center[0] + spec.head_w * px * 0.44, center[1] + spec.head_h * px * 0.10)
        snout_box = bbox_from_center(snout_center, spec.snout_len * px * 1.5, spec.head_h * px * 0.34)
        outer_snout_box = (snout_box[0] - outline_w, snout_box[1] - outline_w, snout_box[2] + outline_w, snout_box[3] + outline_w)
        draw.ellipse(outer_snout_box, fill=pal.outline)
        draw.ellipse(snout_box, fill=darken(pal.skin, 0.08))
        mdraw.ellipse(outer_snout_box, fill=255)
        # near ear
        near_ear = [
            (center[0] - spec.head_w * px * 0.05, center[1] - spec.head_h * px * 0.20),
            (center[0] - spec.head_w * px * 0.80, center[1] - spec.ear_h * px * 0.34),
            (center[0] - spec.head_w * px * 0.14, center[1] + spec.ear_h * px * 0.02),
        ]
        inner_ear = [interp(near_ear[0], near_ear[1], 0.56), interp(near_ear[1], near_ear[2], 0.40), interp(near_ear[2], near_ear[0], 0.34)]
        draw.polygon(near_ear, fill=pal.skin_shadow, outline=pal.outline)
        draw.polygon(inner_ear, fill=with_alpha(pal.cloth, 110), outline=with_alpha(pal.outline, 0))
        mdraw.polygon(near_ear, fill=255)
        # eye
        eye_center = (center[0] + spec.head_w * px * 0.18, center[1] - spec.head_h * px * 0.02)
        eye_h = spec.eye_h * px * (0.20 if blink else 1.0 - 0.5 * squint)
        eye_box = bbox_from_center(eye_center, spec.eye_w * px, max(px * 0.6, eye_h))
        if dead:
            draw.line([(eye_box[0], eye_box[1]), (eye_box[2], eye_box[3])], fill=pal.eye, width=max(1, int(px * 0.5)))
            draw.line([(eye_box[0], eye_box[3]), (eye_box[2], eye_box[1])], fill=pal.eye, width=max(1, int(px * 0.5)))
        elif blink:
            draw.line([(eye_box[0], eye_center[1]), (eye_box[2], eye_center[1])], fill=pal.eye, width=max(1, int(px * 0.52)))
        else:
            draw.ellipse(eye_box, fill=pal.eye)
            dot = bbox_from_center((eye_center[0] - px * 0.25, eye_center[1] - px * 0.40), px * 0.55, px * 0.55)
            draw.ellipse(dot, fill=with_alpha(pal.eye_glow, 230))
        # mouth and teeth
        mouth_a = (snout_center[0] - spec.snout_len * px * 0.18, snout_center[1] + spec.head_h * px * 0.10)
        mouth_b = (snout_center[0] + spec.snout_len * px * 0.32, snout_center[1] + spec.head_h * px * 0.12)
        draw.line([mouth_a, mouth_b], fill=pal.mouth, width=max(1, int(px * 0.45)))
        tooth1 = [(mouth_a[0] + px * 0.6, mouth_a[1]), (mouth_a[0] + px * 1.6, mouth_a[1]), (mouth_a[0] + px * 1.1, mouth_a[1] + spec.tooth_size * px)]
        tooth2 = [(mouth_a[0] + px * 2.0, mouth_a[1] + px * 0.1), (mouth_a[0] + px * 3.0, mouth_a[1] + px * 0.1), (mouth_a[0] + px * 2.5, mouth_a[1] + spec.tooth_size * px * 0.8)]
        draw.polygon(tooth1, fill=pal.tooth, outline=pal.outline)
        draw.polygon(tooth2, fill=pal.tooth, outline=pal.outline)
        solidify_layer_alpha(layer, opaque_mask)
        img.alpha_composite(layer)
        return (snout_center[0] + spec.snout_len * px * 0.60, snout_center[1] - spec.head_h * px * 0.04)

    def _draw_weapon(self, draw: ImageDraw.ImageDraw, hand: Point, spec: GoblinSpec, pal: GoblinPalette, px: float, slash: float, slash_arc: float) -> None:
        handle_len = 6.0 * px
        handle_tip = add(hand, vec(handle_len, -18))
        draw.line([hand, handle_tip], fill=pal.outline, width=max(1, int(px * 0.66)))
        draw.line([hand, handle_tip], fill=darken(pal.weapon_dark, 0.2), width=max(1, int(px * 0.38)))
        item = spec.held_item.lower()
        if item == "spear":
            spear_tip = add(handle_tip, vec(18.0 * px, -4.0))
            draw.line([handle_tip, spear_tip], fill=pal.outline, width=max(1, int(px * 0.60)))
            draw.line([handle_tip, spear_tip], fill=pal.weapon, width=max(1, int(px * 0.34)))
            tip_poly = [
                spear_tip,
                add(spear_tip, (-4.0 * px, -2.0 * px)),
                add(spear_tip, (-3.2 * px, 2.0 * px)),
            ]
            draw.polygon(tip_poly, fill=pal.metal, outline=pal.outline)
        elif item == "sword":
            blade_root = add(handle_tip, vec(5.5 * px, -8.0))
            blade_tip = add(blade_root, vec(14.0 * px, -8.0))
            blade_poly = [
                add(blade_root, (-1.8 * px, -1.2 * px)),
                blade_tip,
                add(blade_root, (1.8 * px, 2.0 * px)),
            ]
            draw.polygon(blade_poly, fill=pal.metal, outline=pal.outline)
            guard = bbox_from_center(blade_root, 3.0 * px, 1.5 * px)
            draw.rounded_rectangle(guard, radius=px * 0.4, fill=pal.weapon, outline=pal.outline)
        else:
            dagger_root = add(handle_tip, vec(2.0 * px, -8.0))
            dagger_tip = add(dagger_root, vec(8.0 * px, -6.0))
            dagger_poly = [
                add(dagger_root, (-1.5 * px, -1.2 * px)),
                dagger_tip,
                add(dagger_root, (1.2 * px, 1.8 * px)),
            ]
            draw.polygon(dagger_poly, fill=pal.metal, outline=pal.outline)
        if slash_arc > 0.02:
            bbox = (hand[0] - 12 * px, hand[1] - 26 * px, hand[0] + 28 * px, hand[1] + 20 * px)
            draw.arc(bbox, start=-64, end=110, fill=with_alpha(pal.weapon, int(140 * slash_arc)), width=max(1, int(px * 0.48)))
            draw.arc((bbox[0] - 3 * px, bbox[1] - 2 * px, bbox[2] + 1 * px, bbox[3] + 1 * px), start=-58, end=98, fill=with_alpha(pal.eye_glow, int(120 * slash_arc)), width=max(1, int(px * 0.28)))

    def render_animation_frame(
        self,
        spec: GoblinSpec,
        animation: str,
        frame_index: int,
        frame_count: int,
        size: Tuple[int, int],
        background: Optional[Color] = None,
        supersample: int = 4,
        downsample: str = "lanczos",
    ) -> Image.Image:
        pose = self.pose_for_animation(animation, frame_index, frame_count)
        resample = RESAMPLING.LANCZOS if downsample == "lanczos" else RESAMPLING.NEAREST
        base_size = size if supersample <= 1 else (size[0] * supersample, size[1] * supersample)
        img = self._render_core(spec, pose, base_size, background)
        if supersample <= 1:
            return img
        return img.resize(size, resample)

    def _render_core(self, spec: GoblinSpec, pose: GoblinPose, size: Tuple[int, int], background: Optional[Color]) -> Image.Image:
        w, h = size
        img = Image.new("RGBA", size, background if background is not None else (0, 0, 0, 0))
        draw = ImageDraw.Draw(img)
        pal = self.PALETTES[spec.palette_name]
        px = min(w, h) / 120.0
        outline_w = max(1, int(px * 0.24))
        ground_y = h * 0.84 + pose.root_y * px
        root = (w * 0.45 + pose.root_x * px, ground_y)
        hip = add(root, (0.0, -13.5 * px - pose.crouch * 4.0 * px - pose.body_bob * px))
        torso = add(root, (2.0 * px, -28.0 * px - pose.crouch * 7.0 * px - pose.body_bob * px))
        head = add(torso, (14.5 * px, -14.0 * px))
        shoulder = add(torso, (3.0 * px, -3.0 * px))

        far_shoulder = add(shoulder, (-1.5 * px, -1.0 * px))
        near_shoulder = add(shoulder, (2.0 * px, 1.2 * px))
        far_hip = add(hip, (-1.0 * px, -0.5 * px))
        near_hip = add(hip, (1.6 * px, 1.2 * px))

        # far limbs first
        far_elbow = add(far_shoulder, vec(spec.arm_upper * px, pose.far_arm_upper))
        far_hand = add(far_elbow, vec(spec.arm_lower * px, pose.far_arm_lower))
        self._draw_capsule(draw, far_shoulder, far_elbow, 2.7 * px, darken(pal.skin_shadow, 0.05), pal.outline, outline_w)
        self._draw_capsule(draw, far_elbow, far_hand, 2.5 * px, darken(pal.skin_shadow, 0.05), pal.outline, outline_w)
        draw.ellipse(bbox_from_center(far_hand, spec.hand_r * px * 1.8, spec.hand_r * px * 1.8), fill=pal.outline)
        draw.ellipse(bbox_from_center(far_hand, spec.hand_r * px * 1.45, spec.hand_r * px * 1.45), fill=pal.skin_shadow)

        far_knee = add(far_hip, vec(spec.leg_upper * px, pose.far_leg_upper))
        far_ankle = add(far_knee, vec(spec.leg_lower * px, pose.far_leg_lower))
        self._draw_capsule(draw, far_hip, far_knee, 3.0 * px, darken(pal.skin_shadow, 0.02), pal.outline, outline_w)
        self._draw_capsule(draw, far_knee, far_ankle, 2.8 * px, darken(pal.skin_shadow, 0.02), pal.outline, outline_w)
        self._draw_foot(img, far_ankle, spec.foot_w * px, spec.foot_h * px, pal.skin_shadow, pal.outline, outline_w)

        # torso / head
        self._draw_body(img, torso, spec, pal, px)
        muzzle_tip = self._draw_head(img, head, spec, pal, px, pose.blink, pose.eye_squint, pose.dead)

        # near limbs
        near_elbow = add(near_shoulder, vec(spec.arm_upper * px, pose.near_arm_upper))
        near_hand = add(near_elbow, vec(spec.arm_lower * px, pose.near_arm_lower))
        self._draw_capsule(draw, near_shoulder, near_elbow, 2.9 * px, pal.skin, pal.outline, outline_w)
        self._draw_capsule(draw, near_elbow, near_hand, 2.7 * px, pal.skin, pal.outline, outline_w)
        draw.ellipse(bbox_from_center(near_hand, spec.hand_r * px * 1.85, spec.hand_r * px * 1.85), fill=pal.outline)
        draw.ellipse(bbox_from_center(near_hand, spec.hand_r * px * 1.48, spec.hand_r * px * 1.48), fill=pal.skin)
        self._draw_weapon(draw, near_hand, spec, pal, px, pose.slash, pose.slash_arc)

        near_knee = add(near_hip, vec(spec.leg_upper * px, pose.near_leg_upper))
        near_ankle = add(near_knee, vec(spec.leg_lower * px, pose.near_leg_lower))
        self._draw_capsule(draw, near_hip, near_knee, 3.2 * px, pal.skin, pal.outline, outline_w)
        self._draw_capsule(draw, near_knee, near_ankle, 3.0 * px, pal.skin, pal.outline, outline_w)
        self._draw_foot(img, near_ankle, spec.foot_w * px, spec.foot_h * px, pal.skin, pal.outline, outline_w)

        # nose highlight and eye glow bloom
        glow = Image.new("RGBA", size, (0, 0, 0, 0))
        gdraw = ImageDraw.Draw(glow)
        gdraw.ellipse(bbox_from_center((muzzle_tip[0] - 5.0 * px, muzzle_tip[1] - 5.0 * px), 2.4 * px, 2.4 * px), fill=with_alpha(pal.eye_glow, 90))
        if pose.recoil > 0.0:
            count = int(10 + pose.recoil * 28)
            rng = random.Random(spec.seed + int(pose.recoil * 100))
            for _ in range(count):
                x = torso[0] + rng.uniform(-15.0, 18.0) * px
                y = torso[1] + rng.uniform(-20.0, 18.0) * px
                s = rng.uniform(0.4, 1.2) * px
                gdraw.rectangle((x - s, y - s, x + s, y + s), fill=with_alpha(pal.eye_glow, rng.randint(50, 110)))
        if pose.dead:
            for i in range(8):
                x = torso[0] + (i - 3.5) * 5.0 * px
                y = torso[1] + 20.0 * px + abs(math.sin(i)) * 4.0 * px
                s = 0.8 * px
                gdraw.rectangle((x - s, y - s, x + s, y + s), fill=with_alpha(pal.eye_glow, 60))
        glow = glow.filter(ImageFilter.GaussianBlur(radius=max(1, int(px * 0.8))))
        img.alpha_composite(glow)
        return img


TARGETS: Dict[str, SideGoblinGenerator] = {SideGoblinGenerator.name: SideGoblinGenerator()}
