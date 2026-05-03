from __future__ import annotations

"""Opaque right-facing green goblin target for side-scrolling games.

The ``blink`` row is Ambition's short-range teleport / precision-blink ability,
not an eyelid blink.  The goblin remains fully opaque inside the character
silhouette; translucent pixels are reserved for outer antialiasing and FX.

For this right-facing target, the far arm is drawn behind the body and the near
weapon arm is drawn in front.  The head is drawn as a rigid local layer and then
rotated as one unit, so ears, snout, eye, mouth, and teeth do not shear apart.
"""

import math
import random
from dataclasses import asdict, dataclass
from typing import Dict, Optional, Tuple

from PIL import Image, ImageColor, ImageDraw

from .common_draw import RESAMPLING, draw_capsule, draw_rotated_ellipse, draw_rotated_rounded_rect
from ..rig import add, clamp, ease_in_out_sine, ease_out_cubic, lerp, smoothstep, vec

Color = Tuple[int, int, int, int]
Point = Tuple[float, float]


def rgba(value: str, alpha: int = 255) -> Color:
    r, g, b = ImageColor.getrgb(value)
    return (r, g, b, alpha)


def with_alpha(color: Color, alpha: int) -> Color:
    return (color[0], color[1], color[2], alpha)


def parse_background(value: str) -> Optional[Color]:
    return None if str(value).lower() == "transparent" else rgba(str(value))


def _bbox(center: Point, w: float, h: float) -> Tuple[float, float, float, float]:
    return (center[0] - w / 2.0, center[1] - h / 2.0, center[0] + w / 2.0, center[1] + h / 2.0)


def _paste_rotated_local(base: Image.Image, layer: Image.Image, center: Point, angle: float) -> None:
    rotated = layer.rotate(angle, resample=RESAMPLING.BICUBIC, expand=True)
    base.alpha_composite(rotated, (int(center[0] - rotated.width / 2), int(center[1] - rotated.height / 2)))


@dataclass(frozen=True)
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
    far_arm_upper: float = 136.0
    far_arm_lower: float = 152.0
    near_arm_upper: float = 28.0
    near_arm_lower: float = 18.0
    far_leg_upper: float = 92.0
    far_leg_lower: float = 98.0
    near_leg_upper: float = 62.0
    near_leg_lower: float = 82.0
    blink: bool = False
    eye_squint: float = 0.0
    slash: float = 0.0
    slash_arc: float = 0.0
    recoil: float = 0.0
    dash: float = 0.0
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
        # Ambition blink ability: teleport/precision-blink effect, not eyelids.
        "blink": {"frames": 8, "duration_ms": 62},
        "dash": {"frames": 6, "duration_ms": 65},
    }

    PALETTES = {
        "classic": {
            "skin": rgba("#67A84B"),
            "skin_top": rgba("#96D46B"),
            "skin_shadow": rgba("#3D6C2B"),
            "belly": rgba("#83BD5D"),
            "cloth": rgba("#6D2BA0"),
            "cloth_dark": rgba("#4B1E72"),
            "eye": rgba("#F24DFF"),
            "eye_glow": rgba("#FFD0FF"),
            "outline": rgba("#15171B"),
            "mouth": rgba("#2A1B18"),
            "tooth": rgba("#F4EBD5"),
            "weapon": rgba("#A963F8"),
            "weapon_dark": rgba("#6A2CC1"),
            "metal": rgba("#E2E4EA"),
            "shadow": rgba("#000000", 34),
        },
        "forest": {
            "skin": rgba("#5C9248"),
            "skin_top": rgba("#8CC66B"),
            "skin_shadow": rgba("#345B2A"),
            "belly": rgba("#74AA58"),
            "cloth": rgba("#6D2BA0"),
            "cloth_dark": rgba("#4B1E72"),
            "eye": rgba("#EF52FF"),
            "eye_glow": rgba("#F6BCFF"),
            "outline": rgba("#15171B"),
            "mouth": rgba("#261D22"),
            "tooth": rgba("#F4EBD5"),
            "weapon": rgba("#B169FF"),
            "weapon_dark": rgba("#6A2CC1"),
            "metal": rgba("#DADCE4"),
            "shadow": rgba("#000000", 34),
        },
    }

    def sample_spec(self, seed: int, archetype: str = "default", held_item: Optional[str] = None) -> GoblinSpec:
        rng = random.Random(seed)
        palette_name = "classic" if archetype == "default" else "forest"
        if held_item is None:
            held_item = rng.choice(["dagger", "spear", "sword"])
        return GoblinSpec(
            target=self.name,
            seed=seed,
            archetype=archetype,
            held_item=held_item,
            palette_name=palette_name,
            head_w=rng.uniform(29.0, 32.0),
            head_h=rng.uniform(23.5, 26.0),
            snout_len=rng.uniform(7.0, 8.5),
            ear_w=rng.uniform(15.0, 17.0),
            ear_h=rng.uniform(11.0, 13.0),
            body_w=rng.uniform(21.0, 23.0),
            body_h=rng.uniform(18.0, 20.0),
            arm_upper=rng.uniform(11.5, 13.0),
            arm_lower=rng.uniform(10.5, 12.0),
            leg_upper=rng.uniform(11.0, 13.0),
            leg_lower=rng.uniform(10.5, 12.0),
            hand_r=rng.uniform(3.2, 3.8),
            foot_w=rng.uniform(10.5, 12.0),
            foot_h=rng.uniform(4.8, 5.6),
            eye_w=rng.uniform(4.5, 5.4),
            eye_h=rng.uniform(7.2, 8.8),
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
            p.blink = frame_index == frame_count // 2
            p.eye_squint = 0.10 if frame_index in {1, frame_count - 2} else 0.0
        elif animation == "blink":
            charge = 1.0 - smoothstep(clamp(t / 0.34, 0.0, 1.0))
            arrive = smoothstep(clamp((t - 0.38) / 0.42, 0.0, 1.0))
            pulse = math.sin(t * math.pi)
            p.root_x = lerp(-3.5, 4.0, arrive) - 2.0 * charge
            p.root_y = -1.0 * pulse
            p.body_bob = 0.2 * pulse
            p.body_tilt = -13.0 * charge + 7.0 * arrive
            p.head_tilt = -8.0 * charge + 3.0 * arrive
            p.far_arm_upper = 150 + 20 * charge + 10 * arrive
            p.far_arm_lower = 158 + 18 * charge
            p.near_arm_upper = 12 - 8 * charge + 24 * arrive
            p.near_arm_lower = 8 - 6 * charge + 15 * arrive
            p.far_leg_upper = 96 + 28 * pulse
            p.far_leg_lower = 98 + 15 * pulse
            p.near_leg_upper = 60 + 22 * pulse
            p.near_leg_lower = 82 + 13 * pulse
            p.eye_squint = 0.24 + 0.18 * pulse
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
            p.far_leg_upper = 90 + stride * amp
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
        elif animation == "dash":
            surge = ease_in_out_sine(t)
            p.root_x = 5.5 + surge * 3.0
            p.body_tilt = -17.0 + wave * 1.0
            p.head_tilt = -8.0
            p.far_arm_upper = 166 + wave * 2
            p.far_arm_lower = 160 + wave * 2
            p.near_arm_upper = 152 + wave * 2
            p.near_arm_lower = 148 + wave * 2
            p.far_leg_upper = 144 + wave * 2
            p.far_leg_lower = 148 + wave * 2
            p.near_leg_upper = 126 + wave * 2
            p.near_leg_lower = 132 + wave * 2
            p.dash = 1.0
            p.eye_squint = 0.32
        elif animation == "death":
            fall = ease_out_cubic(t)
            p.root_x = lerp(0.0, -5.0, fall)
            p.root_y = lerp(0.0, 4.0, fall)
            p.body_tilt = lerp(0.0, 74.0, fall)
            p.head_tilt = lerp(0.0, 58.0, fall)
            p.far_arm_upper = lerp(140.0, 206.0, fall)
            p.far_arm_lower = lerp(152.0, 232.0, fall)
            p.near_arm_upper = lerp(28.0, 92.0, fall)
            p.near_arm_lower = lerp(18.0, 118.0, fall)
            p.far_leg_upper = lerp(90.0, 150.0, fall)
            p.far_leg_lower = lerp(96.0, 166.0, fall)
            p.near_leg_upper = lerp(60.0, 110.0, fall)
            p.near_leg_lower = lerp(82.0, 142.0, fall)
            p.collapse = fall
            p.dead = True
            p.eye_squint = 0.60
        return p

    def _draw_body(self, img: Image.Image, center: Point, spec: GoblinSpec, pal: Dict[str, Color], S: float, angle: float) -> None:
        outline = pal["outline"]
        draw_rotated_ellipse(img, center, (spec.body_w * S, spec.body_h * S), angle, pal["skin"], outline, 1.7 * S)
        draw_rotated_ellipse(img, (center[0] + 2 * S, center[1] + 2 * S), (spec.body_w * 0.58 * S, spec.body_h * 0.60 * S), angle, pal["belly"], None, 0)
        # Opaque cloth silhouette over body.
        layer = Image.new("RGBA", img.size, (0, 0, 0, 0))
        d = ImageDraw.Draw(layer)
        x, y = center
        cloth = [(x - 8 * S, y + 7 * S), (x + 8 * S, y + 7 * S), (x + 11 * S, y + 15 * S), (x - 6 * S, y + 13 * S)]
        d.polygon(cloth, fill=pal["cloth"], outline=outline)
        d.line([cloth[0], cloth[2]], fill=pal["cloth_dark"], width=max(1, int(1.2 * S)))
        img.alpha_composite(layer)

    def _draw_rigid_head(self, img: Image.Image, center: Point, spec: GoblinSpec, pal: Dict[str, Color], S: float, angle: float, blink: bool, squint: float, dead: bool) -> Point:
        pad = int(math.ceil(54 * S))
        layer = Image.new("RGBA", (pad * 2, pad * 2), (0, 0, 0, 0))
        d = ImageDraw.Draw(layer)
        cx, cy = float(pad), float(pad)
        outline = pal["outline"]
        ow = 1.8 * S
        # Ears point backward (left), while snout/eye face right.
        far_ear = [(cx - 9 * S, cy - 5 * S), (cx - 29 * S, cy - 10 * S), (cx - 12 * S, cy + 4 * S)]
        near_ear = [(cx - 3 * S, cy - 7 * S), (cx - 31 * S, cy - 13 * S), (cx - 10 * S, cy + 5 * S)]
        d.polygon(far_ear, fill=pal["skin_shadow"], outline=outline)
        # Head ellipse with opaque fill.
        head_outer = _bbox((cx, cy), spec.head_w * S + 2 * ow, spec.head_h * S + 2 * ow)
        head_inner = _bbox((cx, cy), spec.head_w * S, spec.head_h * S)
        d.ellipse(head_outer, fill=outline)
        d.ellipse(head_inner, fill=pal["skin"])
        d.polygon(near_ear, fill=pal["skin"], outline=outline)
        d.polygon([(cx - 15 * S, cy - 9 * S), (cx - 25 * S, cy - 10 * S), (cx - 13 * S, cy + 1 * S)], fill=pal["cloth"])
        # Snout.
        snout_center = (cx + spec.head_w * 0.42 * S, cy + 2.5 * S)
        snout_outer = _bbox(snout_center, spec.snout_len * 1.65 * S + ow, spec.head_h * 0.38 * S + ow)
        snout_inner = _bbox(snout_center, spec.snout_len * 1.65 * S, spec.head_h * 0.38 * S)
        d.ellipse(snout_outer, fill=outline)
        d.ellipse(snout_inner, fill=pal["skin_shadow"])
        # Semi-transparent highlight composited over opaque base, preserving alpha.
        detail = Image.new("RGBA", layer.size, (0, 0, 0, 0))
        hd = ImageDraw.Draw(detail)
        hd.ellipse((cx - 8 * S, cy - 10 * S, cx + 12 * S, cy + 1 * S), fill=with_alpha(pal["skin_top"], 125))
        layer.alpha_composite(detail)
        # Eye.
        eye_center = (cx + 7.5 * S, cy - 2.0 * S)
        eye_h = spec.eye_h * S * (0.20 if blink else max(0.30, 1.0 - 0.5 * squint))
        if dead:
            r = 3.0 * S
            d.line([(eye_center[0] - r, eye_center[1] - r), (eye_center[0] + r, eye_center[1] + r)], fill=pal["eye"], width=max(1, int(1.2 * S)))
            d.line([(eye_center[0] - r, eye_center[1] + r), (eye_center[0] + r, eye_center[1] - r)], fill=pal["eye"], width=max(1, int(1.2 * S)))
        elif blink:
            d.line([(eye_center[0] - 3 * S, eye_center[1]), (eye_center[0] + 3 * S, eye_center[1])], fill=pal["eye"], width=max(1, int(1.2 * S)))
        else:
            d.ellipse((eye_center[0] - spec.eye_w * S / 2, eye_center[1] - eye_h / 2, eye_center[0] + spec.eye_w * S / 2, eye_center[1] + eye_h / 2), fill=pal["eye"])
            d.ellipse((eye_center[0] - 0.8 * S, eye_center[1] - 2.5 * S, eye_center[0] + 0.6 * S, eye_center[1] - 1.1 * S), fill=pal["eye_glow"])
        # Mouth and teeth.
        mouth_a = (snout_center[0] - 3 * S, snout_center[1] + 3 * S)
        mouth_b = (snout_center[0] + 5 * S, snout_center[1] + 3.5 * S)
        d.line([mouth_a, mouth_b], fill=pal["mouth"], width=max(1, int(1.1 * S)))
        d.polygon([(mouth_a[0] + 1 * S, mouth_a[1]), (mouth_a[0] + 2.7 * S, mouth_a[1]), (mouth_a[0] + 1.9 * S, mouth_a[1] + spec.tooth_size * S)], fill=pal["tooth"], outline=outline)

        _paste_rotated_local(img, layer, center, angle)
        return (center[0] + spec.head_w * 0.42 * S + 6 * S, center[1] + 0.5 * S)

    def _limb_chain(self, root: Point, upper: float, lower: float, a1: float, a2: float) -> Tuple[Point, Point]:
        mid = add(root, vec(upper, a1))
        end = add(mid, vec(lower, a2))
        return mid, end

    def _draw_weapon(self, d: ImageDraw.ImageDraw, hand: Point, spec: GoblinSpec, pal: Dict[str, Color], S: float, slash_arc: float) -> None:
        angle = -18 + slash_arc * 36
        handle = add(hand, vec(7 * S, angle))
        d.line([hand, handle], fill=pal["outline"], width=max(1, int(2.0 * S)))
        d.line([hand, handle], fill=pal["weapon_dark"], width=max(1, int(1.0 * S)))
        item = spec.held_item.lower()
        if item == "spear":
            tip = add(handle, vec(20 * S, angle + 2))
            d.line([handle, tip], fill=pal["outline"], width=max(1, int(1.8 * S)))
            d.line([handle, tip], fill=pal["weapon"], width=max(1, int(0.9 * S)))
            d.polygon([tip, add(tip, (-5 * S, -3 * S)), add(tip, (-4 * S, 3 * S))], fill=pal["metal"], outline=pal["outline"])
        elif item == "sword":
            tip = add(handle, vec(18 * S, angle - 6))
            d.line([handle, tip], fill=pal["outline"], width=max(1, int(4.0 * S)))
            d.line([handle, tip], fill=pal["metal"], width=max(1, int(2.0 * S)))
        else:
            tip = add(handle, vec(12 * S, angle - 10))
            d.line([handle, tip], fill=pal["outline"], width=max(1, int(3.4 * S)))
            d.line([handle, tip], fill=pal["metal"], width=max(1, int(1.7 * S)))

    def _draw_blink_fx(self, img: Image.Image, root_x: float, ground_y: float, S: float, frame_index: int, frame_count: int, pal: Dict[str, Color]) -> None:
        d = ImageDraw.Draw(img)
        t = 0.0 if frame_count <= 1 else frame_index / float(frame_count - 1)
        charge = 1.0 - smoothstep(clamp(t / 0.35, 0.0, 1.0))
        transit = math.sin(clamp((t - 0.12) / 0.72, 0.0, 1.0) * math.pi)
        arrive = smoothstep(clamp((t - 0.45) / 0.40, 0.0, 1.0))
        mid_y = ground_y - 51 * S
        source_x = root_x - 24 * S
        dest_x = root_x + 31 * S
        d.line([(source_x, mid_y), (dest_x, mid_y - 5 * S)], fill=with_alpha(pal["eye"], int(45 + 105 * max(charge, transit))), width=max(1, int(1.1 * S)))
        for rscale, alpha in [(1.0 + 0.55 * arrive, 150), (0.55 + 0.25 * charge, 95)]:
            rx, ry = 7.5 * S * rscale, 13.0 * S * rscale
            box = (dest_x - rx, mid_y - ry - 4 * S, dest_x + rx, mid_y + ry - 4 * S)
            d.ellipse(box, outline=with_alpha(pal["eye"], int(alpha * max(0.25, charge + arrive))), width=max(1, int(1.3 * S)))
        for i, frac in enumerate((0.0, 0.33, 0.66, 1.0)):
            x = lerp(source_x, dest_x, frac)
            h = (29.0 - i * 3.0 + transit * 5.0) * S
            a = int((78 - i * 12) * transit)
            if a > 0:
                d.line([(x, mid_y - h / 2), (x + 7 * S, mid_y + h / 2)], fill=with_alpha(pal["cloth"], a), width=max(1, int(1.6 * S)))
                d.line([(x + 3 * S, mid_y - h / 2), (x - 4 * S, mid_y + h / 2)], fill=with_alpha(pal["eye"], max(15, a - 18)), width=max(1, int(0.85 * S)))

    def _render_highres(self, spec: GoblinSpec, animation: str, frame_index: int, frame_count: int, size: Tuple[int, int], background: Optional[Color], scale: int) -> Image.Image:
        W, H = size[0] * scale, size[1] * scale
        bg = (0, 0, 0, 0) if background is None else background
        img = Image.new("RGBA", (W, H), bg)
        S = float(scale)
        pal = self.PALETTES.get(spec.palette_name, self.PALETTES["classic"])
        p = self.pose_for_animation(animation, frame_index, frame_count)
        ground_y = (101.0 + p.root_y) * S
        root_x = (60.0 + p.root_x) * S
        d = ImageDraw.Draw(img)
        d.ellipse((root_x - 26 * S, ground_y - 5 * S, root_x + (31 + 13 * p.collapse) * S, ground_y + 6 * S), fill=(0, 0, 0, int(30 * (1 - 0.28 * p.collapse))))

        if animation == "blink":
            self._draw_blink_fx(img, root_x, ground_y, S, frame_index, frame_count, pal)

        if p.dash:
            for i in range(4):
                y = (50 + i * 10 + math.sin(frame_index + i) * 2) * S
                d.line([(14 * S, y), ((40 - i * 3) * S, y - 2 * S)], fill=(150, 212, 105, 90), width=max(1, int(1.5 * S)))

        collapse = p.collapse
        body_center = (root_x + lerp(0, 12 * S, collapse), ground_y - lerp(37 * S, 11 * S, collapse) + p.body_bob * S)
        head_center = (root_x + lerp(16 * S, 37 * S, collapse), ground_y - lerp(62 * S, 15 * S, collapse) + p.body_bob * 0.45 * S)

        hip_far = (body_center[0] - 5 * S, body_center[1] + 9 * S)
        hip_near = (body_center[0] + 7 * S, body_center[1] + 9 * S)
        shoulder_far = (body_center[0] - 8 * S, body_center[1] - 7 * S)
        shoulder_near = (body_center[0] + 8 * S, body_center[1] - 7 * S)

        # Legs.
        for hip, a1, a2, tint, foot_shift in [
            (hip_far, p.far_leg_upper, p.far_leg_lower, pal["skin_shadow"], -1.5),
            (hip_near, p.near_leg_upper, p.near_leg_lower, pal["skin"], 3.0),
        ]:
            knee, ankle = self._limb_chain(hip, spec.leg_upper * S, spec.leg_lower * S, a1, a2)
            draw_capsule(d, hip, knee, 2.5 * S, tint, pal["outline"], 1.2 * S)
            draw_capsule(d, knee, ankle, 2.3 * S, tint, pal["outline"], 1.2 * S)
            foot_center = (ankle[0] + spec.foot_w * 0.32 * S + foot_shift * S, min(ground_y - 2 * S, ankle[1] + 2 * S))
            draw_rotated_rounded_rect(img, foot_center, (spec.foot_w * S, spec.foot_h * S), -5 + p.body_tilt * 0.08, spec.foot_h * 0.5 * S, tint, pal["outline"], 1.1 * S)

        # Far arm behind body.
        elbow, hand = self._limb_chain(shoulder_far, spec.arm_upper * S, spec.arm_lower * S, p.far_arm_upper, p.far_arm_lower)
        draw_capsule(d, shoulder_far, elbow, 2.2 * S, pal["skin_shadow"], pal["outline"], 1.1 * S)
        draw_capsule(d, elbow, hand, 2.1 * S, pal["skin_shadow"], pal["outline"], 1.1 * S)

        self._draw_body(img, body_center, spec, pal, S, p.body_tilt)
        self._draw_rigid_head(img, head_center, spec, pal, S, p.head_tilt, p.blink, p.eye_squint, p.dead)

        # Near arm and weapon on top.
        elbow, hand = self._limb_chain(shoulder_near, spec.arm_upper * S, spec.arm_lower * S, p.near_arm_upper, p.near_arm_lower)
        draw_capsule(d, shoulder_near, elbow, 2.3 * S, pal["skin"], pal["outline"], 1.1 * S)
        draw_capsule(d, elbow, hand, 2.2 * S, pal["skin"], pal["outline"], 1.1 * S)
        d.ellipse((hand[0] - spec.hand_r * S, hand[1] - spec.hand_r * S, hand[0] + spec.hand_r * S, hand[1] + spec.hand_r * S), fill=pal["skin"], outline=pal["outline"], width=max(1, int(1.0 * S)))
        if animation in {"slash", "idle", "walk", "run", "dash", "blink"}:
            self._draw_weapon(d, hand, spec, pal, S, p.slash_arc)
        if p.slash_arc > 0.18:
            d.arc((hand[0] - 6 * S, hand[1] - 30 * S, hand[0] + 38 * S, hand[1] + 19 * S), start=-70, end=45, fill=(242, 77, 255, 155), width=max(1, int(2.2 * S)))
        return img

    def render_animation_frame(
        self,
        spec: GoblinSpec,
        animation: str,
        frame_index: int,
        frame_count: int,
        size: Tuple[int, int] = (128, 128),
        background: Optional[Color] = None,
        supersample: int = 4,
        downsample: str = "lanczos",
    ) -> Image.Image:
        high = self._render_highres(spec, animation, frame_index, frame_count, size, background, max(1, int(supersample)))
        resample = RESAMPLING.NEAREST if downsample == "nearest" else RESAMPLING.LANCZOS
        return high.resize(size, resample)
