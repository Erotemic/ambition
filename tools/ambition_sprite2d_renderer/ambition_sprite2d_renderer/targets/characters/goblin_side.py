from __future__ import annotations

"""Opaque right-facing green goblin target for side-scrolling games.

The ``blink_out`` and ``blink_in`` rows are Ambition's short-range teleport /
precision-blink ability split into source and destination phases, not an eyelid
blink.  The goblin remains fully opaque inside the character
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
from ambition_sprite2d_renderer.core.draw import rgba, with_alpha, bbox_from_center as _bbox

from ...authoring.common_draw import RESAMPLING, draw_capsule, draw_rotated_ellipse, draw_rotated_rounded_rect
from ...authoring.rig import add, clamp, ease_in_out_sine, ease_out_cubic, lerp, smoothstep, vec

Color = Tuple[int, int, int, int]
Point = Tuple[float, float]






def parse_background(value: str) -> Optional[Color]:
    return None if str(value).lower() == "transparent" else rgba(str(value))




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
        # Ambition blink ability split into source/departure and destination/arrival.
        "blink_out": {"frames": 6, "duration_ms": 62},
        "blink_in": {"frames": 6, "duration_ms": 62},
        "dash": {"frames": 6, "duration_ms": 65},
        "talk": {"frames": 8, "duration_ms": 110},
        "interact": {"frames": 6, "duration_ms": 90},
        "celebrate": {"frames": 8, "duration_ms": 92},
        "block": {"frames": 6, "duration_ms": 85},
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
        "cave": {
            "skin": rgba("#5F6F68"), "skin_top": rgba("#8EA29A"), "skin_shadow": rgba("#35423E"), "belly": rgba("#778A82"),
            "cloth": rgba("#394BA0"), "cloth_dark": rgba("#242C69"), "eye": rgba("#6BE9FF"), "eye_glow": rgba("#D9FFFF"),
            "outline": rgba("#15171B"), "mouth": rgba("#251D1D"), "tooth": rgba("#F4EBD5"), "weapon": rgba("#75B8FF"),
            "weapon_dark": rgba("#2D5E98"), "metal": rgba("#CDD3DA"), "shadow": rgba("#000000", 36),
        },
        "desert": {
            "skin": rgba("#A7A34B"), "skin_top": rgba("#D4C76B"), "skin_shadow": rgba("#67632B"), "belly": rgba("#BDB65D"),
            "cloth": rgba("#B06B2A"), "cloth_dark": rgba("#72451D"), "eye": rgba("#FF7059"), "eye_glow": rgba("#FFD0C7"),
            "outline": rgba("#15171B"), "mouth": rgba("#2A1B18"), "tooth": rgba("#F4EBD5"), "weapon": rgba("#FFB15E"),
            "weapon_dark": rgba("#9A5A25"), "metal": rgba("#E7DDC4"), "shadow": rgba("#000000", 34),
        },
        "frost": {
            "skin": rgba("#6EA0A3"), "skin_top": rgba("#A8DDE0"), "skin_shadow": rgba("#3B686B"), "belly": rgba("#85BFC0"),
            "cloth": rgba("#5063B8"), "cloth_dark": rgba("#2B3677"), "eye": rgba("#FFFFFF"), "eye_glow": rgba("#B6FFF5"),
            "outline": rgba("#15171B"), "mouth": rgba("#1E2527"), "tooth": rgba("#F4EBD5"), "weapon": rgba("#9FEAFF"),
            "weapon_dark": rgba("#37798C"), "metal": rgba("#E2F6FA"), "shadow": rgba("#000000", 34),
        },
        "brute": {
            "skin": rgba("#7B8F35"), "skin_top": rgba("#A8C24D"), "skin_shadow": rgba("#4D5C23"), "belly": rgba("#8DA544"),
            "cloth": rgba("#8A3A2B"), "cloth_dark": rgba("#5C241B"), "eye": rgba("#FF7059"), "eye_glow": rgba("#FFD0C7"),
            "outline": rgba("#15171B"), "mouth": rgba("#2A1B18"), "tooth": rgba("#F4EBD5"), "weapon": rgba("#FF8A5E"),
            "weapon_dark": rgba("#A0432D"), "metal": rgba("#E2D3C0"), "shadow": rgba("#000000", 38),
        },
        "shaman": {
            "skin": rgba("#4C8A5B"), "skin_top": rgba("#78C987"), "skin_shadow": rgba("#2E5D37"), "belly": rgba("#6BAB73"),
            "cloth": rgba("#7E4FCC"), "cloth_dark": rgba("#4C2C87"), "eye": rgba("#FFE36E"), "eye_glow": rgba("#FFF6B0"),
            "outline": rgba("#15171B"), "mouth": rgba("#261D22"), "tooth": rgba("#F4EBD5"), "weapon": rgba("#B98CFF"),
            "weapon_dark": rgba("#6A2CC1"), "metal": rgba("#DADCE4"), "shadow": rgba("#000000", 34),
        },
        "chieftain": {
            "skin": rgba("#6E9238"), "skin_top": rgba("#B4CF55"), "skin_shadow": rgba("#3F5C25"), "belly": rgba("#93AA47"),
            "cloth": rgba("#C07039"), "cloth_dark": rgba("#6B2F1F"), "eye": rgba("#FFDA66"), "eye_glow": rgba("#FFF0A8"),
            "outline": rgba("#15171B"), "mouth": rgba("#291815"), "tooth": rgba("#F4EBD5"), "weapon": rgba("#FFB15E"),
            "weapon_dark": rgba("#934A25"), "metal": rgba("#E7D2A8"), "shadow": rgba("#000000", 38),
        },
        "bard": {
            "skin": rgba("#5B9A56"), "skin_top": rgba("#8FD36F"), "skin_shadow": rgba("#376333"), "belly": rgba("#78B763"),
            "cloth": rgba("#D85EA5"), "cloth_dark": rgba("#842E69"), "eye": rgba("#FFE36E"), "eye_glow": rgba("#FFF6B0"),
            "outline": rgba("#15171B"), "mouth": rgba("#261D22"), "tooth": rgba("#F4EBD5"), "weapon": rgba("#FFD65A"),
            "weapon_dark": rgba("#A96B22"), "metal": rgba("#DADCE4"), "shadow": rgba("#000000", 34),
        },
    }

    def sample_spec(self, seed: int, archetype: str = "default", held_item: Optional[str] = None) -> GoblinSpec:
        rng = random.Random(seed)
        archetype_key = str(archetype or "default").lower()
        palette_name = "classic"
        for token, palette in [
            ("chieftain", "chieftain"),
            ("chief", "chieftain"),
            ("bard", "bard"),
            ("drummer", "bard"),
            ("brute", "brute"),
            ("shaman", "shaman"),
            ("cave", "cave"),
            ("desert", "desert"),
            ("frost", "frost"),
            ("scout", "forest"),
            ("forest", "forest"),
        ]:
            if token in archetype_key:
                palette_name = palette
                break
        if archetype != "default" and palette_name == "classic":
            palette_name = "forest"
        if held_item is None:
            if any(token in archetype_key for token in ["chieftain", "chief", "brute"]):
                held_item = "hammer"
            elif any(token in archetype_key for token in ["shaman", "bard", "drummer"]):
                held_item = "staff"
            elif "scout" in archetype_key:
                held_item = "bow"
            else:
                held_item = rng.choice(["dagger", "spear", "sword"])
        scale = 1.0
        if any(token in archetype_key for token in ["chieftain", "chief", "brute"]):
            scale = 1.14
        elif "scout" in archetype_key:
            scale = 0.93
        elif any(token in archetype_key for token in ["shaman", "bard", "drummer"]):
            scale = 1.02
        return GoblinSpec(
            target=self.name,
            seed=seed,
            archetype=archetype,
            held_item=held_item,
            palette_name=palette_name,
            head_w=rng.uniform(29.0, 32.0) * scale,
            head_h=rng.uniform(23.5, 26.0) * scale,
            snout_len=rng.uniform(7.0, 8.5) * scale,
            ear_w=rng.uniform(15.0, 17.0) * scale,
            ear_h=rng.uniform(11.0, 13.0) * scale,
            body_w=rng.uniform(21.0, 23.0) * scale,
            body_h=rng.uniform(18.0, 20.0) * scale,
            arm_upper=rng.uniform(11.5, 13.0) * scale,
            arm_lower=rng.uniform(10.5, 12.0) * scale,
            leg_upper=rng.uniform(11.0, 13.0) * scale,
            leg_lower=rng.uniform(10.5, 12.0) * scale,
            hand_r=rng.uniform(3.2, 3.8) * scale,
            foot_w=rng.uniform(10.5, 12.0) * scale,
            foot_h=rng.uniform(4.8, 5.6) * scale,
            eye_w=rng.uniform(4.5, 5.4) * scale,
            eye_h=rng.uniform(7.2, 8.8) * scale,
            tooth_size=rng.uniform(2.4, 3.2) * scale,
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
        elif animation == "talk":
            bob = abs(wave)
            p.body_bob = bob * 0.9
            p.head_tilt = -4.0 + wave * 3.0
            p.near_arm_upper = 24.0 + wave * 10.0
            p.near_arm_lower = -18.0 + wave * 8.0
            p.far_arm_upper = 178.0 - wave * 7.0
            p.far_arm_lower = 146.0
            p.eye_squint = 0.06 + 0.06 * bob
        elif animation == "interact":
            reach = smoothstep(clamp(t / 0.55, 0.0, 1.0))
            p.body_tilt = -4.0 * reach
            p.head_tilt = -3.0 * reach
            p.near_arm_upper = lerp(18.0, -20.0, reach)
            p.near_arm_lower = lerp(8.0, -32.0, reach)
        elif animation == "celebrate":
            lift = math.sin(t * math.pi)
            p.body_bob = -2.0 * lift
            p.body_tilt = wave * 6.0
            p.head_tilt = -wave * 5.0
            p.near_arm_upper = -62.0 + wave * 8.0
            p.near_arm_lower = -46.0
            p.far_arm_upper = 214.0 - wave * 8.0
            p.far_arm_lower = 210.0
            p.eye_squint = 0.02
        elif animation == "block":
            p.body_tilt = -10.0
            p.head_tilt = -8.0
            p.near_arm_upper = -24.0
            p.near_arm_lower = -52.0
            p.far_arm_upper = 156.0
            p.far_arm_lower = 188.0
            p.eye_squint = 0.18
        elif animation == "blink_out":
            charge = smoothstep(clamp(t / 0.46, 0.0, 1.0))
            burst = smoothstep(clamp((t - 0.30) / 0.48, 0.0, 1.0))
            pulse = math.sin(t * math.pi)
            p.root_x = -2.0 * charge - 2.0 * burst
            p.root_y = 1.3 * charge - 1.8 * burst
            p.body_bob = -1.0 * charge + 0.18 * pulse
            p.body_tilt = -15.0 * charge - 11.0 * burst
            p.head_tilt = -11.0 * charge - 3.0 * burst
            p.far_arm_upper = 166.0 + 20.0 * charge
            p.far_arm_lower = 170.0 + 18.0 * burst
            p.near_arm_upper = -2.0 - 14.0 * charge
            p.near_arm_lower = -4.0 - 16.0 * burst
            p.far_leg_upper = 122.0 + 18.0 * charge
            p.far_leg_lower = 76.0 + 15.0 * charge
            p.near_leg_upper = 92.0 + 16.0 * charge
            p.near_leg_lower = 68.0 + 12.0 * charge
            p.eye_squint = 0.22 + 0.14 * pulse + 0.14 * burst
        elif animation == "blink_in":
            appear = smoothstep(clamp(t / 0.60, 0.0, 1.0))
            settle = ease_out_cubic(appear)
            recoil = 1.0 - settle
            pulse = math.sin(t * math.pi)
            p.root_x = 4.8 * recoil
            p.root_y = 1.8 * recoil - 1.6 * pulse * recoil
            p.body_bob = -0.9 * recoil + 0.16 * pulse
            p.body_tilt = 16.0 * recoil - 4.0 * settle
            p.head_tilt = 10.0 * recoil - 2.0 * settle
            p.far_arm_upper = 184.0 - 28.0 * settle
            p.far_arm_lower = 176.0 - 18.0 * settle
            p.near_arm_upper = 34.0 - 20.0 * settle
            p.near_arm_lower = 26.0 - 18.0 * settle
            p.far_leg_upper = 128.0 - 26.0 * settle
            p.far_leg_lower = 84.0 + 10.0 * recoil
            p.near_leg_upper = 104.0 - 22.0 * settle
            p.near_leg_lower = 76.0 + 12.0 * recoil
            p.eye_squint = 0.28 + 0.18 * recoil
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

    def _solve_leg_ik(self, hip: Point, ankle: Point, upper_len: float, lower_len: float, bend_sign: float = 1.0) -> Tuple[Point, float, float]:
        """Solve a two-segment side-view leg toward a reachable ankle target.

        This is the same baseline used by the compact player walk: fixed upper
        and lower segment lengths, authored ankle targets, and forward-bending
        knees for a right-facing side profile.
        """
        dx = ankle[0] - hip[0]
        dy = ankle[1] - hip[1]
        dist = math.hypot(dx, dy)
        min_reach = abs(upper_len - lower_len) + 0.001
        max_reach = max(min_reach + 0.001, upper_len + lower_len - 0.001)
        dist = clamp(dist, min_reach, max_reach)
        base = math.degrees(math.atan2(dy, dx))
        cos_off = clamp((upper_len * upper_len + dist * dist - lower_len * lower_len) / (2.0 * upper_len * dist), -1.0, 1.0)
        off = math.degrees(math.acos(cos_off))
        a1 = base - bend_sign * off
        knee = add(hip, vec(upper_len, a1))
        a2 = math.degrees(math.atan2(ankle[1] - knee[1], ankle[0] - knee[0]))
        return knee, a1, a2

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
        elif item == "hammer":
            tip = add(handle, vec(16 * S, angle - 4))
            d.line([handle, tip], fill=pal["outline"], width=max(1, int(4.0 * S)))
            d.line([handle, tip], fill=pal["weapon_dark"], width=max(1, int(2.0 * S)))
            head = add(tip, vec(4 * S, angle - 5))
            d.rounded_rectangle((head[0] - 5*S, head[1] - 5*S, head[0] + 8*S, head[1] + 5*S), radius=2*S, fill=pal["metal"], outline=pal["outline"], width=max(1, int(1.0*S)))
        elif item == "staff":
            tip = add(handle, vec(22 * S, angle + 1))
            d.line([handle, tip], fill=pal["outline"], width=max(1, int(3.0 * S)))
            d.line([handle, tip], fill=pal["weapon_dark"], width=max(1, int(1.4 * S)))
            d.ellipse((tip[0] - 5*S, tip[1] - 5*S, tip[0] + 5*S, tip[1] + 5*S), fill=pal["eye"], outline=pal["outline"], width=max(1, int(0.9*S)))
        elif item == "bow":
            tip = add(handle, vec(16 * S, angle - 4))
            d.arc((tip[0] - 4*S, tip[1] - 17*S, tip[0] + 14*S, tip[1] + 17*S), start=250, end=105, fill=pal["weapon"], width=max(1, int(2.0*S)))
            d.line([(tip[0] + 6*S, tip[1] - 12*S), (tip[0] + 6*S, tip[1] + 12*S)], fill=pal["metal"], width=max(1, int(0.8*S)))
            d.line([handle, (tip[0] + 19*S, tip[1])], fill=pal["metal"], width=max(1, int(1.1*S)))
        else:
            tip = add(handle, vec(12 * S, angle - 10))
            d.line([handle, tip], fill=pal["outline"], width=max(1, int(3.4 * S)))
            d.line([handle, tip], fill=pal["metal"], width=max(1, int(1.7 * S)))

    def _draw_blink_out_fx(self, img: Image.Image, root_x: float, ground_y: float, S: float, frame_index: int, frame_count: int, pal: Dict[str, Color]) -> None:
        d = ImageDraw.Draw(img)
        t = 0.0 if frame_count <= 1 else frame_index / float(frame_count - 1)
        charge = smoothstep(clamp(t / 0.56, 0.0, 1.0))
        burst = smoothstep(clamp((t - 0.32) / 0.48, 0.0, 1.0))
        source_x = root_x + 9 * S
        mid_y = ground_y - 48 * S

        for rscale, alpha in [
            (0.62 + 0.55 * charge, int(145 * max(charge, 0.15))),
            (0.44 + 0.70 * burst, int(110 * max(burst, 0.12))),
        ]:
            rx, ry = 8.0 * S * rscale, 13.0 * S * rscale
            box = (source_x - rx, mid_y - ry - 4 * S, source_x + rx, mid_y + ry - 4 * S)
            d.ellipse(box, outline=with_alpha(pal["eye"], alpha), width=max(1, int(1.3 * S)))

        for i, dx in enumerate((-10, -4, 3, 10)):
            height = (27.0 - i * 2.2 + 8.0 * burst) * S
            alpha = int((88 - i * 14) * max(charge, burst))
            if alpha > 0:
                x = source_x + dx * S
                d.line([(x, mid_y - height / 2), (x + 6 * S, mid_y + height / 2)], fill=with_alpha(pal["cloth"], alpha), width=max(1, int(1.6 * S)))
                d.line([(x + 3 * S, mid_y - height / 2), (x - 4 * S, mid_y + height / 2)], fill=with_alpha(pal["eye"], max(18, alpha - 20)), width=max(1, int(0.9 * S)))

        for i in range(4):
            frac = i / 3.0 if 3 else 0.0
            sx = source_x - 9 * S + frac * 18 * S
            sy = mid_y - 11 * S - frac * 6 * S
            ex = sx + (6 + i * 2) * S
            ey = sy - (8 + i * 2) * S
            d.line([(sx, sy), (ex, ey)], fill=with_alpha(pal["eye"], int(62 * max(charge, burst))), width=max(1, int(1.0 * S)))

        ripple_alpha = int(76 * max(charge, burst))
        if ripple_alpha > 0:
            d.ellipse((source_x - 18 * S, ground_y - 7 * S, source_x + 15 * S, ground_y + 1 * S), outline=with_alpha(pal["cloth"], ripple_alpha), width=max(1, int(1.0 * S)))

    def _draw_blink_in_fx(self, img: Image.Image, root_x: float, ground_y: float, S: float, frame_index: int, frame_count: int, pal: Dict[str, Color]) -> None:
        d = ImageDraw.Draw(img)
        t = 0.0 if frame_count <= 1 else frame_index / float(frame_count - 1)
        appear = smoothstep(clamp(t / 0.60, 0.0, 1.0))
        settle = ease_out_cubic(appear)
        dest_x = root_x + 9 * S
        mid_y = ground_y - 48 * S

        for rscale, alpha in [
            (1.25 - 0.45 * settle, int(150 * max(0.18, 1.0 - t * 0.55))),
            (0.52 + 0.30 * appear, int(116 * max(0.20, 1.0 - t * 0.35))),
        ]:
            rx, ry = 8.2 * S * rscale, 13.0 * S * rscale
            box = (dest_x - rx, mid_y - ry - 4 * S, dest_x + rx, mid_y + ry - 4 * S)
            d.ellipse(box, outline=with_alpha(pal["eye"], alpha), width=max(1, int(1.3 * S)))

        for i, dx in enumerate((-14, -7, 0, 8, 14)):
            height = (28.0 - i * 2.5 + 7.0 * (1.0 - settle)) * S
            alpha = int((92 - i * 12) * max(0.18, 1.0 - t * 0.42))
            x = dest_x + dx * S
            d.line([(x, mid_y - height / 2), (dest_x, mid_y)], fill=with_alpha(pal["cloth"], alpha), width=max(1, int(1.5 * S)))
            d.line([(x, mid_y + height / 2), (dest_x + 2 * S, mid_y - 2 * S)], fill=with_alpha(pal["eye"], max(16, alpha - 18)), width=max(1, int(0.9 * S)))

        ripple_alpha = int(76 * max(0.18, 1.0 - t * 0.35))
        d.ellipse((dest_x - 18 * S, ground_y - 7 * S, dest_x + 15 * S, ground_y + 1 * S), outline=with_alpha(pal["eye"], ripple_alpha), width=max(1, int(1.0 * S)))

    def _composite_teleport_actor(self, base: Image.Image, actor: Image.Image, animation: str, frame_index: int, frame_count: int, S: float) -> None:
        alpha_bbox = actor.getchannel("A").getbbox()
        if alpha_bbox is None:
            return
        x1, y1, x2, y2 = alpha_bbox
        t = 0.0 if frame_count <= 1 else frame_index / float(frame_count - 1)
        slice_w = max(1, int(5 * S))
        if animation == "blink_out":
            progress = smoothstep(clamp((t - 0.02) / 0.98, 0.0, 1.0))
            for i, x in enumerate(range(x1, x2, slice_w)):
                strip = actor.crop((x, y1, min(x + slice_w, x2), y2))
                if strip.getchannel("A").getbbox() is None:
                    continue
                frac = 0.5 if x2 == x1 else ((x + slice_w * 0.5) - x1) / float(max(1, x2 - x1))
                dx = (frac - 0.5) * (21.0 * S * progress) + math.sin(frac * math.pi * 7.0 + progress * 6.0) * 1.6 * S * progress
                dy = -(5.0 + abs(frac - 0.5) * 17.0) * S * progress
                alpha_scale = max(0.06, 1.0 - 0.88 * progress)
                if progress > 0.35 and (i + int(progress * 9)) % 3 == 0:
                    alpha_scale *= 0.35
                a = strip.getchannel("A").point(lambda v, s=alpha_scale: max(0, min(255, int(v * s))))
                strip.putalpha(a)
                base.alpha_composite(strip, (int(x + dx), int(y1 + dy)))
        else:
            progress = smoothstep(clamp(t / 1.0, 0.0, 1.0))
            for i, x in enumerate(range(x1, x2, slice_w)):
                strip = actor.crop((x, y1, min(x + slice_w, x2), y2))
                if strip.getchannel("A").getbbox() is None:
                    continue
                frac = 0.5 if x2 == x1 else ((x + slice_w * 0.5) - x1) / float(max(1, x2 - x1))
                dx = (frac - 0.5) * (22.0 * S * (1.0 - progress))
                dy = -(3.0 + abs(frac - 0.5) * 15.0) * S * (1.0 - progress)
                alpha_scale = min(1.0, 0.18 + 0.94 * progress)
                if progress < 0.45 and (i + frame_index) % 4 == 0:
                    alpha_scale *= 0.55
                a = strip.getchannel("A").point(lambda v, s=alpha_scale: max(0, min(255, int(v * s))))
                strip.putalpha(a)
                base.alpha_composite(strip, (int(x + dx), int(y1 + dy)))
            full_alpha = smoothstep(clamp((progress - 0.34) / 0.66, 0.0, 1.0))
            if full_alpha > 0:
                resolved = actor.copy()
                a = resolved.getchannel("A").point(lambda v, s=full_alpha: max(0, min(255, int(v * s))))
                resolved.putalpha(a)
                base.alpha_composite(resolved)


    def _draw_variant_accessories(self, d: ImageDraw.ImageDraw, spec: GoblinSpec, pal: Dict[str, Color], S: float, root_x: float, ground_y: float, body_center: Point, head_center: Point) -> None:
        name = (spec.archetype or "").lower()
        outline = pal["outline"]
        if any(token in name for token in ["chieftain", "chief"]):
            # Crude crown, boss pauldrons, and rally banner silhouette.
            for dx in (-12, 0, 12):
                d.polygon([(head_center[0] + dx*S - 7*S, head_center[1] - 27*S), (head_center[0] + dx*S, head_center[1] - 42*S), (head_center[0] + dx*S + 7*S, head_center[1] - 27*S)], fill=pal["weapon"], outline=outline)
            d.rounded_rectangle((body_center[0] - 22*S, body_center[1] - 14*S, body_center[0] - 5*S, body_center[1] - 3*S), radius=4*S, fill=pal["metal"], outline=outline, width=max(1, int(1*S)))
            d.rounded_rectangle((body_center[0] + 8*S, body_center[1] - 14*S, body_center[0] + 25*S, body_center[1] - 3*S), radius=4*S, fill=pal["metal"], outline=outline, width=max(1, int(1*S)))
            d.line([(body_center[0] - 24*S, body_center[1] + 15*S), (body_center[0] - 24*S, body_center[1] - 26*S)], fill=outline, width=max(1, int(2*S)))
            d.polygon([(body_center[0] - 24*S, body_center[1] - 26*S), (body_center[0] - 5*S, body_center[1] - 19*S), (body_center[0] - 24*S, body_center[1] - 12*S)], fill=pal["cloth"], outline=outline)
        elif any(token in name for token in ["bard", "drummer"]):
            # Ear tassels and little drum/sound charm for the music faction tie-in.
            d.arc((head_center[0] - 24*S, head_center[1] - 34*S, head_center[0] + 24*S, head_center[1] - 8*S), start=205, end=335, fill=pal["weapon"], width=max(1, int(1.8*S)))
            for dx in (-22, 24):
                d.line([(head_center[0] + dx*S, head_center[1] - 13*S), (head_center[0] + dx*S, head_center[1] + 5*S)], fill=pal["cloth"], width=max(1, int(2*S)))
                d.ellipse((head_center[0] + dx*S - 3*S, head_center[1] + 4*S, head_center[0] + dx*S + 3*S, head_center[1] + 10*S), fill=pal["weapon"], outline=outline)
            d.ellipse((body_center[0] - 24*S, body_center[1] + 2*S, body_center[0] - 6*S, body_center[1] + 20*S), fill=pal["cloth_dark"], outline=outline, width=max(1, int(1*S)))
            d.line([(body_center[0] - 22*S, body_center[1] + 10*S), (body_center[0] - 8*S, body_center[1] + 9*S)], fill=pal["weapon"], width=max(1, int(1*S)))
        elif "brute" in name:
            d.rounded_rectangle((body_center[0] - 19*S, body_center[1] - 13*S, body_center[0] - 4*S, body_center[1] - 3*S), radius=4*S, fill=pal["metal"], outline=outline, width=max(1, int(1*S)))
            d.rounded_rectangle((body_center[0] + 7*S, body_center[1] - 13*S, body_center[0] + 23*S, body_center[1] - 3*S), radius=4*S, fill=pal["metal"], outline=outline, width=max(1, int(1*S)))
            for dx in (-8, 2, 12):
                d.polygon([(head_center[0] + dx*S, head_center[1] - 28*S), (head_center[0] + (dx+5)*S, head_center[1] - 40*S), (head_center[0] + (dx+10)*S, head_center[1] - 28*S)], fill=pal["tooth"], outline=outline)
        elif "shaman" in name:
            d.arc((head_center[0] - 22*S, head_center[1] - 35*S, head_center[0] + 23*S, head_center[1] - 10*S), start=195, end=345, fill=pal["eye"], width=max(1, int(1.8*S)))
            for dx, dy in [(-18, -34), (3, -39), (23, -31)]:
                d.ellipse((head_center[0] + dx*S - 2*S, head_center[1] + dy*S - 2*S, head_center[0] + dx*S + 2*S, head_center[1] + dy*S + 2*S), fill=pal["eye_glow"])
        elif "scout" in name:
            d.polygon([(head_center[0] - 24*S, head_center[1] - 12*S), (head_center[0] + 11*S, head_center[1] - 25*S), (head_center[0] + 26*S, head_center[1] - 8*S), (head_center[0] + 6*S, head_center[1] - 13*S)], fill=pal["cloth"], outline=outline)
            d.line([(body_center[0] - 16*S, body_center[1] - 3*S), (body_center[0] + 18*S, body_center[1] + 13*S)], fill=pal["cloth_dark"], width=max(1, int(2*S)))
        elif "frost" in name:
            d.rounded_rectangle((head_center[0] - 8*S, head_center[1] + 12*S, head_center[0] + 23*S, head_center[1] + 21*S), radius=3*S, fill=pal["cloth"], outline=outline, width=max(1, int(1*S)))
            for dx in (-10, 6, 20):
                d.line([(head_center[0] + dx*S, head_center[1] + 20*S), (head_center[0] + (dx-4)*S, head_center[1] + 29*S)], fill=pal["cloth"], width=max(1, int(2*S)))
        elif "desert" in name:
            d.rounded_rectangle((head_center[0] - 17*S, head_center[1] - 20*S, head_center[0] + 24*S, head_center[1] - 12*S), radius=4*S, fill=pal["cloth"], outline=outline, width=max(1, int(1*S)))
            d.polygon([(head_center[0] + 17*S, head_center[1] - 18*S), (head_center[0] + 33*S, head_center[1] - 23*S), (head_center[0] + 24*S, head_center[1] - 11*S)], fill=pal["cloth"], outline=outline)
        elif "cave" in name:
            d.rounded_rectangle((body_center[0] - 20*S, body_center[1] - 3*S, body_center[0] - 9*S, body_center[1] + 15*S), radius=3*S, fill=pal["metal"], outline=outline, width=max(1, int(1*S)))
            d.ellipse((head_center[0] + 12*S, head_center[1] - 25*S, head_center[0] + 19*S, head_center[1] - 18*S), fill=pal["eye_glow"], outline=outline, width=max(1, int(0.8*S)))

    def _render_highres(self, spec: GoblinSpec, animation: str, frame_index: int, frame_count: int, size: Tuple[int, int], background: Optional[Color], scale: int) -> Image.Image:
        W, H = size[0] * scale, size[1] * scale
        bg = (0, 0, 0, 0) if background is None else background
        img = Image.new("RGBA", (W, H), bg)
        # Scale the 128-base character to the requested frame width so a
        # render_scale>1 canvas draws the SAME character with more native
        # pixels (matches the toon generator's S=(W/128)*ss). Identical at the
        # 128 default; only render_scale>1 changes it.
        S = float(scale) * (size[0] / 128.0)
        pal = self.PALETTES.get(spec.palette_name, self.PALETTES["classic"])
        p = self.pose_for_animation(animation, frame_index, frame_count)
        ground_y = (101.0 + p.root_y) * S
        root_x = (60.0 + p.root_x) * S
        d = ImageDraw.Draw(img)
        # No baked ground drop shadow; the scene renderer owns contact shadows.

        if animation == "blink_out":
            self._draw_blink_out_fx(img, root_x, ground_y, S, frame_index, frame_count, pal)
        elif animation == "blink_in":
            self._draw_blink_in_fx(img, root_x, ground_y, S, frame_index, frame_count, pal)

        if p.dash:
            for i in range(4):
                y = (50 + i * 10 + math.sin(frame_index + i) * 2) * S
                d.line([(14 * S, y), ((40 - i * 3) * S, y - 2 * S)], fill=(150, 212, 105, 90), width=max(1, int(1.5 * S)))

        character_img = img if animation not in {"blink_out", "blink_in"} else Image.new("RGBA", (W, H), (0, 0, 0, 0))
        character_draw = ImageDraw.Draw(character_img)

        collapse = p.collapse
        body_center = (root_x + lerp(0, 12 * S, collapse), ground_y - lerp(37 * S, 11 * S, collapse) + p.body_bob * S)
        head_center = (root_x + lerp(16 * S, 37 * S, collapse), ground_y - lerp(62 * S, 15 * S, collapse) + p.body_bob * 0.45 * S)

        hip_far = (body_center[0] - 5 * S, body_center[1] + 9 * S)
        hip_near = (body_center[0] + 7 * S, body_center[1] + 9 * S)
        shoulder_far = (body_center[0] - 8 * S, body_center[1] - 7 * S)
        shoulder_near = (body_center[0] + 8 * S, body_center[1] - 7 * S)

        # Legs.  Walk/run uses the documented side-view baseline: authored
        # contact/down/passing/up ankle targets, fixed two-bone lengths, and
        # forward-bending knees.  This keeps goblin walks from degenerating
        # into opposed sticks while preserving rigid shin/thigh lengths.
        if animation in {"walk", "run"}:
            idx = frame_index % 8
            leg_len = (spec.leg_upper + spec.leg_lower) * S
            stride = leg_len * (0.42 if animation == "run" else 0.36)
            base_drop = leg_len * (0.86 if animation == "run" else 0.88)
            far_x = (-1.00, -0.76, -0.36, -0.06, 0.12, -0.18, -0.58, -0.90)
            near_x = (0.92, 0.68, 0.30, 0.04, -0.08, 0.20, 0.62, 0.94)
            far_lift = (0.00, 0.00, 0.05, 0.14, 0.00, 0.02, 0.08, 0.02)
            near_lift = (0.00, 0.02, 0.08, 0.02, 0.00, 0.00, 0.05, 0.14)
            far_shift = (-1.5, -1.2, -0.5, 0.3, 1.0, 0.7, 0.0, -0.7)
            near_shift = (1.4, 0.9, 0.1, -0.7, -1.1, -0.8, -0.2, 0.7)
            foot_tilt = (-7, -4, -2, 3, 7, 4, 2, -3) if animation == "run" else (-5, -3, -1, 2, 5, 3, 1, -2)

            leg_draws = []
            for name, hip, xnorm, lift, tint, foot_shift, foot_angle in [
                ("front_dark", hip_far, far_x[idx], far_lift[idx], pal["skin_shadow"], far_shift[idx], foot_tilt[idx]),
                ("back_light", hip_near, near_x[idx], near_lift[idx], pal["skin"], near_shift[idx], -foot_tilt[(idx + 4) % 8]),
            ]:
                ankle = (body_center[0] + xnorm * stride, hip[1] + base_drop - lift * leg_len)
                knee, _a1, _a2 = self._solve_leg_ik(hip, ankle, spec.leg_upper * S, spec.leg_lower * S, bend_sign=1.0)
                foot_center = (ankle[0] + spec.foot_w * 0.32 * S + foot_shift * S, ankle[1] + 2.0 * S)
                leg_draws.append((0 if name == "back_light" else 1, hip, knee, ankle, tint, foot_center, foot_angle))
            for _z, hip, knee, ankle, tint, foot_center, foot_angle in sorted(leg_draws):
                draw_capsule(character_draw, hip, knee, 2.5 * S, tint, pal["outline"], 1.2 * S)
                draw_capsule(character_draw, knee, ankle, 2.3 * S, tint, pal["outline"], 1.2 * S)
                draw_rotated_rounded_rect(character_img, foot_center, (spec.foot_w * S, spec.foot_h * S), foot_angle + p.body_tilt * 0.08, spec.foot_h * 0.5 * S, tint, pal["outline"], 1.1 * S)
        else:
            for hip, a1, a2, tint, foot_shift in [
                (hip_far, p.far_leg_upper, p.far_leg_lower, pal["skin_shadow"], -1.5),
                (hip_near, p.near_leg_upper, p.near_leg_lower, pal["skin"], 3.0),
            ]:
                knee, ankle = self._limb_chain(hip, spec.leg_upper * S, spec.leg_lower * S, a1, a2)
                draw_capsule(character_draw, hip, knee, 2.5 * S, tint, pal["outline"], 1.2 * S)
                draw_capsule(character_draw, knee, ankle, 2.3 * S, tint, pal["outline"], 1.2 * S)
                foot_center = (ankle[0] + spec.foot_w * 0.32 * S + foot_shift * S, min(ground_y - 2 * S, ankle[1] + 2 * S))
                draw_rotated_rounded_rect(character_img, foot_center, (spec.foot_w * S, spec.foot_h * S), -5 + p.body_tilt * 0.08, spec.foot_h * 0.5 * S, tint, pal["outline"], 1.1 * S)

        # Far arm behind body.
        elbow, hand = self._limb_chain(shoulder_far, spec.arm_upper * S, spec.arm_lower * S, p.far_arm_upper, p.far_arm_lower)
        draw_capsule(character_draw, shoulder_far, elbow, 2.2 * S, pal["skin_shadow"], pal["outline"], 1.1 * S)
        draw_capsule(character_draw, elbow, hand, 2.1 * S, pal["skin_shadow"], pal["outline"], 1.1 * S)

        self._draw_body(character_img, body_center, spec, pal, S, p.body_tilt)
        self._draw_rigid_head(character_img, head_center, spec, pal, S, p.head_tilt, p.blink, p.eye_squint, p.dead)
        self._draw_variant_accessories(character_draw, spec, pal, S, root_x, ground_y, body_center, head_center)

        # Near arm and weapon on top.
        elbow, hand = self._limb_chain(shoulder_near, spec.arm_upper * S, spec.arm_lower * S, p.near_arm_upper, p.near_arm_lower)
        draw_capsule(character_draw, shoulder_near, elbow, 2.3 * S, pal["skin"], pal["outline"], 1.1 * S)
        draw_capsule(character_draw, elbow, hand, 2.2 * S, pal["skin"], pal["outline"], 1.1 * S)
        character_draw.ellipse((hand[0] - spec.hand_r * S, hand[1] - spec.hand_r * S, hand[0] + spec.hand_r * S, hand[1] + spec.hand_r * S), fill=pal["skin"], outline=pal["outline"], width=max(1, int(1.0 * S)))
        if animation in {"slash", "idle", "walk", "run", "dash", "blink_out", "blink_in"}:
            self._draw_weapon(character_draw, hand, spec, pal, S, p.slash_arc)
        if p.slash_arc > 0.18:
            character_draw.arc((hand[0] - 6 * S, hand[1] - 30 * S, hand[0] + 38 * S, hand[1] + 19 * S), start=-70, end=45, fill=(242, 77, 255, 155), width=max(1, int(2.2 * S)))

        if animation in {"blink_out", "blink_in"}:
            self._composite_teleport_actor(img, character_img, animation, frame_index, frame_count, S)
        else:
            img.alpha_composite(character_img)
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
