from __future__ import annotations

"""Distinct masked ninja / shadow-duelist target.

This target is intentionally semi-independent from the generic toon lane.  It
shares the adapter/YAML/sheet pipeline so the output has the same manifest
format as the rest of the runtime sprites, but the art is drawn by a bespoke
silhouette-first renderer: slate-gray cloth, red eye slits, angular armor
panels, a long katana, and scarf / sash tails that give it a readable outline.

The first-pass animation poses are deliberately conservative.  The canonical
idle frame is the art-review source of truth; the other rows provide enough
movement placeholders to exercise spritesheet + YAML generation until we author
proper action keys.
"""

import math
import random
from dataclasses import asdict, dataclass
from typing import Dict, Optional, Tuple

from PIL import Image, ImageColor, ImageDraw
from ambition_sprite2d_renderer.core.draw import rgba, with_alpha

from ...authoring.animation_vocab import CORE_CHARACTER_ANIMATION_ORDER, DEFAULT_CORE_TIMINGS, ordered_subset
from ...authoring.rig import add, clamp, vec
from ...authoring.common_draw import RESAMPLING, draw_capsule, draw_rotated_ellipse, draw_rotated_rounded_rect

Color = Tuple[int, int, int, int]
Point = Tuple[float, float]






def parse_background(value: str) -> Optional[Color]:
    return None if str(value).lower() == "transparent" else rgba(str(value))


def _mix(a: Color, b: Color, t: float) -> Color:
    t = clamp(t, 0.0, 1.0)
    return (
        int(a[0] + (b[0] - a[0]) * t),
        int(a[1] + (b[1] - a[1]) * t),
        int(a[2] + (b[2] - a[2]) * t),
        int(a[3] + (b[3] - a[3]) * t),
    )


@dataclass(frozen=True)
class NinjaSpec:
    target: str
    seed: int
    archetype: str
    name: str
    palette_name: str
    blade_style: str
    rank: str
    horn_len: float
    banner_len: float
    pauldron_scale: float
    skirt_len: float
    armor_bulk: float
    crest_scale: float
    sword_len: float
    head_w: float
    head_h: float
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
    scarf_len: float
    sash_len: float
    eye_glow: float

    def to_dict(self) -> Dict[str, object]:
        return asdict(self)


@dataclass
class NinjaPose:
    root_x: float = 0.0
    root_y: float = 0.0
    bob: float = 0.0
    lean: float = 0.0
    crouch: float = 0.0
    torso_tilt: float = 0.0
    head_tilt: float = 0.0
    sword_angle: float = -108.0
    sword_shift_x: float = 0.0
    sword_shift_y: float = 0.0
    near_arm_upper: float = 168.0
    near_arm_lower: float = 188.0
    far_arm_upper: float = 220.0
    far_arm_lower: float = 170.0
    near_leg_upper: float = 83.0
    near_leg_lower: float = 75.0
    far_leg_upper: float = 102.0
    far_leg_lower: float = 91.0
    scarf_swing: float = 0.0
    sash_swing: float = 0.0
    eye_squint: float = 0.0
    dash: float = 0.0
    slash: float = 0.0
    hit: float = 0.0
    fade: float = 0.0
    dead: bool = False


class NinjaSideGenerator:
    name = "ninja"

    ANIMATIONS: Dict[str, Dict[str, int]] = ordered_subset(
        {
            **DEFAULT_CORE_TIMINGS,
            # A faster slash helps sell this as a duelist even before the full
            # attack breakdown is authored.
            "slash": {"frames": 8, "duration_ms": 68},
            "dash": {"frames": 6, "duration_ms": 58},
        },
        CORE_CHARACTER_ANIMATION_ORDER,
    )

    PALETTES = {
        "moon_steel": {
            "outline": rgba("#11131A"),
            "cloth_dark": rgba("#272B31"),
            "cloth": rgba("#626970"),
            "cloth_mid": rgba("#777F86"),
            "cloth_light": rgba("#B8BEC2"),
            "armor": rgba("#434A52"),
            "armor_dark": rgba("#22262C"),
            "wrap": rgba("#151922"),
            "sash": rgba("#6D425E"),
            "sash_dark": rgba("#3D2638"),
            "eye": rgba("#E51424"),
            "eye_hot": rgba("#FF6B5A"),
            "blade": rgba("#EFF4F6"),
            "blade_shadow": rgba("#A8B0B5"),
            "blade_edge": rgba("#FFFFFF"),
            "brass": rgba("#B18C38"),
            "shadow": rgba("#000000", 54),
            "smoke": rgba("#465061", 58),
        }
    }

    PRESETS = {
        "shadow_duelist": {
            "name": "Shadow Duelist",
            "palette_name": "moon_steel",
            "blade_style": "long_katana",
            "rank": "duelist",
            "horn_len": 0.0,
            "banner_len": 0.0,
            "pauldron_scale": 1.0,
            "skirt_len": 0.0,
            "armor_bulk": 1.0,
            "crest_scale": 1.0,
            "sword_len": 88.0,
            "head_w": 27.0,
            "head_h": 27.5,
            "shoulder_w": 43.0,
            "torso_w": 34.0,
            "torso_h": 35.0,
            "hip_w": 27.0,
            "arm_upper": 18.5,
            "arm_lower": 17.5,
            "arm_radius": 4.7,
            "leg_upper": 25.0,
            "leg_lower": 24.0,
            "leg_radius": 5.2,
            "hand_r": 4.3,
            "foot_w": 16.5,
            "foot_h": 7.4,
            "scarf_len": 30.0,
            "sash_len": 34.0,
            "eye_glow": 1.0,
        },
        "shadow_oni_leader": {
            "name": "Shadow Oni Leader",
            "palette_name": "moon_steel",
            "blade_style": "commander_katana",
            "rank": "leader",
            "horn_len": 18.5,
            "banner_len": 45.0,
            "pauldron_scale": 1.55,
            "skirt_len": 31.0,
            "armor_bulk": 1.22,
            "crest_scale": 1.55,
            "sword_len": 78.0,
            "head_w": 29.0,
            "head_h": 30.0,
            "shoulder_w": 54.0,
            "torso_w": 38.0,
            "torso_h": 38.0,
            "hip_w": 34.0,
            "arm_upper": 19.5,
            "arm_lower": 19.0,
            "arm_radius": 5.4,
            "leg_upper": 25.5,
            "leg_lower": 24.5,
            "leg_radius": 5.7,
            "hand_r": 4.8,
            "foot_w": 17.8,
            "foot_h": 7.8,
            "scarf_len": 39.0,
            "sash_len": 42.0,
            "eye_glow": 1.16,
        },
    }

    def sample_spec(self, seed: int = 0, archetype: str = "shadow_duelist") -> NinjaSpec:
        preset = dict(self.PRESETS.get(archetype, self.PRESETS["shadow_duelist"]))
        rng = random.Random(seed)
        # Tiny deterministic variation only in secondary details.  The main
        # silhouette remains stable for animation authoring.
        preset["eye_glow"] = round(float(preset["eye_glow"]) * rng.uniform(0.92, 1.08), 3)
        preset["scarf_len"] = round(float(preset["scarf_len"]) + rng.uniform(-1.0, 1.2), 3)
        return NinjaSpec(target="ninja", seed=seed, archetype=archetype, **preset)

    def _palette(self, spec: NinjaSpec) -> Dict[str, Color]:
        return dict(self.PALETTES.get(spec.palette_name, self.PALETTES["moon_steel"]))

    def pose_for_animation(self, animation: str, frame_index: int, frame_count: int, _spec: NinjaSpec) -> NinjaPose:
        denom = max(1, frame_count)
        phase = math.sin((frame_index / denom) * math.tau)
        cphase = math.cos((frame_index / denom) * math.tau)
        p = NinjaPose()
        p.bob = phase * 1.0
        p.scarf_swing = phase * 3.5
        p.sash_swing = -phase * 3.2
        p.eye_squint = 0.15 + 0.05 * cphase

        if animation == "walk":
            p.root_x = phase * 1.6
            p.bob = abs(phase) * 1.6
            p.lean = 2.0
            p.near_leg_upper = 68.0 + phase * 18.0
            p.near_leg_lower = 73.0 - phase * 12.0
            p.far_leg_upper = 104.0 - phase * 16.0
            p.far_leg_lower = 91.0 + phase * 10.0
            p.near_arm_upper = 166.0 - phase * 16.0
            p.far_arm_upper = 218.0 + phase * 14.0
            p.scarf_swing = -phase * 5.0
        elif animation == "run":
            p.root_x = phase * 2.4
            p.bob = abs(phase) * 2.0
            p.lean = 5.0
            p.torso_tilt = -5.0
            p.near_leg_upper = 61.0 + phase * 24.0
            p.near_leg_lower = 70.0 - phase * 18.0
            p.far_leg_upper = 112.0 - phase * 20.0
            p.far_leg_lower = 91.0 + phase * 16.0
            p.sword_angle = -104.0 + phase * 2.0
            p.scarf_swing = -7.0 + -phase * 7.0
            p.sash_swing = -phase * 8.0
        elif animation == "jump":
            t = frame_index / max(1, frame_count - 1)
            arc = math.sin(t * math.pi)
            p.root_y = -20.0 * arc
            p.lean = 2.0
            p.crouch = 2.0 * (1.0 - arc)
            p.near_leg_upper = 68.0
            p.near_leg_lower = 48.0
            p.far_leg_upper = 118.0
            p.far_leg_lower = 134.0
            p.sash_swing = 6.0
            p.scarf_swing = -8.0
        elif animation == "fall":
            t = frame_index / max(1, frame_count - 1)
            p.root_y = -16.0 + 10.0 * t
            p.lean = -1.0
            p.sword_angle = -112.0
            p.near_leg_upper = 75.0
            p.near_leg_lower = 86.0
            p.far_leg_upper = 96.0
            p.far_leg_lower = 78.0
            p.scarf_swing = 9.0
        elif animation == "slash":
            t = frame_index / max(1, frame_count - 1)
            wind = max(0.0, 1.0 - t / 0.30)
            strike = max(0.0, min(1.0, (t - 0.18) / 0.42))
            recover = max(0.0, (t - 0.65) / 0.35)
            p.slash = strike
            p.root_x = -4.0 * wind + 8.0 * strike - 3.0 * recover
            p.lean = -6.0 * wind + 9.0 * strike
            p.torso_tilt = -11.0 * wind + 15.0 * strike
            p.sword_angle = -126.0 * wind + 25.0 * strike - 72.0 * recover
            p.sword_shift_x = -4.0 * wind + 5.0 * strike
            p.sword_shift_y = 3.0 * wind - 2.0 * strike
            p.near_arm_upper = 142.0 - 62.0 * strike
            p.near_arm_lower = 202.0 - 104.0 * strike
            p.far_arm_upper = 232.0 - 128.0 * strike
            p.far_arm_lower = 160.0 - 58.0 * strike
            p.scarf_swing = -12.0 * strike
            p.sash_swing = 11.0 * strike
            p.eye_squint = 0.75
        elif animation == "hit":
            t = frame_index / max(1, frame_count - 1)
            p.hit = 1.0 - t
            p.root_x = -5.0 * (1.0 - t)
            p.lean = -8.0 * (1.0 - t)
            p.torso_tilt = -9.0 * (1.0 - t)
            p.eye_squint = 0.8
        elif animation == "death":
            t = frame_index / max(1, frame_count - 1)
            p.dead = t > 0.45
            p.root_x = -8.0 * t
            p.root_y = 12.0 * t
            p.lean = -18.0 * t
            p.torso_tilt = -72.0 * t
            p.head_tilt = -45.0 * t
            p.sword_angle = -42.0 - 45.0 * t
            p.eye_squint = 1.0
            p.scarf_swing = 12.0 * t
            p.sash_swing = 10.0 * t
            p.fade = 0.25 * t
        elif animation == "blink_out":
            t = frame_index / max(1, frame_count - 1)
            p.dash = t
            p.fade = t * 0.75
            p.root_x = 18.0 * t
            p.root_y = -8.0 * math.sin(t * math.pi)
            p.lean = 12.0
            p.scarf_swing = -16.0
        elif animation == "blink_in":
            t = frame_index / max(1, frame_count - 1)
            inv = 1.0 - t
            p.dash = inv
            p.fade = inv * 0.75
            p.root_x = -18.0 * inv
            p.root_y = -8.0 * math.sin(t * math.pi)
            p.lean = 8.0
            p.scarf_swing = -16.0 * inv
        elif animation == "dash":
            t = frame_index / max(1, frame_count - 1)
            p.dash = 1.0
            p.root_x = 8.0 * math.sin(t * math.pi)
            p.lean = 18.0
            p.torso_tilt = -12.0
            p.sword_angle = -105.0
            p.scarf_swing = -22.0
            p.sash_swing = -14.0
            p.eye_squint = 0.65
        return p

    def _draw_smoke_crescent(self, draw: ImageDraw.ImageDraw, center: Point, w: float, h: float, color: Color) -> None:
        x, y = center
        draw.ellipse((x - w / 2.0, y - h / 2.0, x + w / 2.0, y + h / 2.0), fill=color)

    def _draw_scarf_tail(self, draw: ImageDraw.ImageDraw, points: Tuple[Point, Point, Point, Point], fill: Color, outline: Color) -> None:
        draw.line([points[0], points[1], points[2], points[3]], fill=outline, width=5, joint="curve")
        draw.line([points[0], points[1], points[2], points[3]], fill=fill, width=3, joint="curve")
        tip = points[-1]
        draw.polygon(
            [
                tip,
                (tip[0] - 5.0, tip[1] - 2.0),
                (tip[0] - 2.5, tip[1] + 4.0),
            ],
            fill=fill,
            outline=outline,
        )

    def _draw_blade(self, img: Image.Image, hilt: Point, angle: float, length: float, width: float, pal: Dict[str, Color], alpha: int = 255) -> None:
        d = ImageDraw.Draw(img, "RGBA")
        ux, uy = math.cos(math.radians(angle)), math.sin(math.radians(angle))
        nx, ny = -uy, ux
        tip = (hilt[0] + ux * length, hilt[1] + uy * length)
        base_l = (hilt[0] + nx * width * 0.55, hilt[1] + ny * width * 0.55)
        base_r = (hilt[0] - nx * width * 0.55, hilt[1] - ny * width * 0.55)
        mid_l = (hilt[0] + ux * length * 0.78 + nx * width * 0.33, hilt[1] + uy * length * 0.78 + ny * width * 0.33)
        mid_r = (hilt[0] + ux * length * 0.78 - nx * width * 0.33, hilt[1] + uy * length * 0.78 - ny * width * 0.33)
        outline = with_alpha(pal["outline"], alpha)
        blade = with_alpha(pal["blade"], alpha)
        shade = with_alpha(pal["blade_shadow"], alpha)
        edge = with_alpha(pal["blade_edge"], min(255, alpha + 25))
        d.polygon([base_l, mid_l, tip, mid_r, base_r], fill=outline)
        inset = max(1.2, width * 0.23)
        d.polygon(
            [
                (base_l[0] - nx * inset, base_l[1] - ny * inset),
                (mid_l[0] - nx * inset * 0.6, mid_l[1] - ny * inset * 0.6),
                tip,
                (mid_r[0] + nx * inset * 0.15, mid_r[1] + ny * inset * 0.15),
                (base_r[0] + nx * inset * 0.15, base_r[1] + ny * inset * 0.15),
            ],
            fill=blade,
        )
        d.polygon([base_r, mid_r, tip, (mid_r[0] + nx * 0.7, mid_r[1] + ny * 0.7)], fill=shade)
        d.line([base_l, tip], fill=edge, width=1)

    def _draw_ninja(self, img: Image.Image, spec: NinjaSpec, p: NinjaPose, scale: float) -> None:
        d = ImageDraw.Draw(img, "RGBA")
        pal = self._palette(spec)
        S = scale
        leader = spec.rank == "leader"

        def sp(pt: Point) -> Point:
            return (pt[0] * S, pt[1] * S)

        def sc(v: float) -> float:
            return v * S

        def col(name: str, alpha: Optional[int] = None) -> Color:
            c = pal[name]
            if alpha is not None:
                return with_alpha(c, alpha)
            if p.fade <= 0:
                return c
            return with_alpha(c, int(c[3] * (1.0 - p.fade)))

        root = (64.0 + p.root_x, 0.0 + p.root_y)
        ground_y = 115.0
        hip = (root[0] + p.lean * 0.38, 80.0 + p.root_y - p.bob + p.crouch)
        torso = (root[0] + p.lean, 55.0 + p.root_y - p.bob + p.crouch * 0.45)
        neck = (torso[0] + 0.5, torso[1] - 21.0)
        head = (torso[0] + 1.5, torso[1] - 34.0)

        # Ground shadow removed; dash smoke crescents (below) are
        # intentional VFX and stay.
        if p.dash > 0.0:
            for i, alpha in enumerate((72, 44, 25)):
                self._draw_smoke_crescent(
                    d,
                    sp((50.0 - i * 12.0, 75.0 + i * 5.0 + p.root_y * 0.3)),
                    sc(30.0 + i * 8.0),
                    sc(7.0),
                    with_alpha(pal["smoke"], int(alpha * (1.0 - p.fade * 0.5))),
                )

        # Back cloth tails first so the body cuts in front of them.
        scarf_anchor = (head[0] + 11.0, head[1] - 4.0)
        if leader:
            # The leader gets a ragged command banner before the smaller scarf tails.
            # At sprite scale this reads as a distinct right-side silhouette instead
            # of just another slim sword-user.
            top = (scarf_anchor[0] + 7.0, scarf_anchor[1] - 11.0 + p.scarf_swing * 0.10)
            banner_pts = [
                sp(top),
                sp((top[0] + spec.banner_len * 0.62, top[1] - 7.0 + p.scarf_swing * 0.08)),
                sp((top[0] + spec.banner_len, top[1] + 0.5 + p.scarf_swing * 0.22)),
                sp((top[0] + spec.banner_len * 0.78, top[1] + 8.5 + p.scarf_swing * 0.30)),
                sp((top[0] + spec.banner_len * 0.95, top[1] + 19.0 + p.scarf_swing * 0.35)),
                sp((top[0] + spec.banner_len * 0.58, top[1] + 15.5 + p.scarf_swing * 0.27)),
                sp((top[0] + spec.banner_len * 0.48, top[1] + 27.0 + p.scarf_swing * 0.32)),
                sp((top[0] + 3.0, top[1] + 17.0)),
            ]
            d.polygon(banner_pts, fill=col("cloth_dark"), outline=col("outline"))
            crest_center = sp((top[0] + spec.banner_len * 0.55, top[1] + 6.5 + p.scarf_swing * 0.20))
            crest_r = sc(6.5 * spec.crest_scale)
            crest_col = with_alpha(pal["eye"], int(86 * (1.0 - p.fade)))
            d.ellipse((crest_center[0] - crest_r, crest_center[1] - crest_r, crest_center[0] + crest_r, crest_center[1] + crest_r), outline=crest_col, width=max(1, int(sc(1.5))))
            d.line((crest_center[0], crest_center[1] - crest_r * 0.88, crest_center[0], crest_center[1] + crest_r * 0.88), fill=crest_col, width=max(1, int(sc(1.3))))
            d.line((crest_center[0] - crest_r * 0.58, crest_center[1] + crest_r * 0.15, crest_center[0] + crest_r * 0.58, crest_center[1] - crest_r * 0.18), fill=crest_col, width=max(1, int(sc(1.0))))
        self._draw_scarf_tail(
            d,
            (
                sp(scarf_anchor),
                sp((scarf_anchor[0] + 10.0, scarf_anchor[1] - 10.0 + p.scarf_swing * 0.15)),
                sp((scarf_anchor[0] + spec.scarf_len * 0.55, scarf_anchor[1] - 3.0 + p.scarf_swing)),
                sp((scarf_anchor[0] + spec.scarf_len, scarf_anchor[1] - 7.0 + p.scarf_swing * 1.1)),
            ),
            col("cloth_dark"),
            col("outline"),
        )
        self._draw_scarf_tail(
            d,
            (
                sp((scarf_anchor[0] + 1.0, scarf_anchor[1] + 4.0)),
                sp((scarf_anchor[0] + 12.0, scarf_anchor[1] + 8.0 + p.scarf_swing * 0.10)),
                sp((scarf_anchor[0] + spec.scarf_len * 0.50, scarf_anchor[1] + 13.0 + p.scarf_swing * 0.7)),
                sp((scarf_anchor[0] + spec.scarf_len * 0.92, scarf_anchor[1] + 12.0 + p.scarf_swing * 0.9)),
            ),
            col("sash_dark"),
            col("outline"),
        )

        # Sword / scabbard.  Duelists keep the long read-at-a-distance blade;
        # leaders default to a sheathed command-katana so the silhouette comes
        # from horns, banner, pauldrons, and skirt instead of the same sword pose.
        hilt = (torso[0] - (25.5 if leader else 21.5) + p.sword_shift_x, torso[1] + (30.5 if leader else 28.5) + p.sword_shift_y)
        leader_blade_active = (not leader) or p.slash > 0.08 or p.dash > 0.25
        if leader and not leader_blade_active:
            sheath_top = (hip[0] + 16.0, hip[1] - 6.5)
            sheath_bot = (hip[0] + 28.0, hip[1] + 30.0)
            d.line([sp(sheath_top), sp(sheath_bot)], fill=col("outline"), width=max(1, int(sc(6.5))))
            d.line([sp(sheath_top), sp(sheath_bot)], fill=col("armor_dark"), width=max(1, int(sc(4.0))))
            d.line([sp((sheath_top[0] - 1.8, sheath_top[1] + 2.0)), sp((sheath_top[0] + 3.0, sheath_top[1] - 1.5))], fill=col("brass"), width=max(1, int(sc(2.0))))
        else:
            self._draw_blade(img, sp(hilt), p.sword_angle, sc(spec.sword_len), sc(8.4 * spec.armor_bulk), pal, alpha=int(255 * (1.0 - p.fade)))
        if p.slash > 0.08:
            slash_layer = Image.new("RGBA", img.size, (0, 0, 0, 0))
            sd = ImageDraw.Draw(slash_layer, "RGBA")
            for off, alpha in ((0, 52), (7, 32), (14, 18)):
                arc_box = (
                    sc(29 + off + p.root_x),
                    sc(18 + p.root_y),
                    sc(111 + off + p.root_x),
                    sc(96 + p.root_y),
                )
                sd.arc(arc_box, 208, 332, fill=with_alpha(pal["blade_edge"], int(alpha * p.slash)), width=max(1, int(sc(3.2))))
            img.alpha_composite(slash_layer)

        # Legs.
        def limb(a: Point, b: Point, c: Point, radius: float, fill: Color, outline: Color) -> None:
            draw_capsule(d, sp(a), sp(b), sc(radius), fill, outline, outline_w=sc(1.25))
            draw_capsule(d, sp(b), sp(c), sc(radius * 0.95), fill, outline, outline_w=sc(1.25))

        def leg_points(is_near: bool) -> Tuple[Point, Point, Point]:
            sign = 1.0 if is_near else -1.0
            upper = p.near_leg_upper if is_near else p.far_leg_upper
            lower = p.near_leg_lower if is_near else p.far_leg_lower
            start = (hip[0] + sign * spec.hip_w * 0.28, hip[1] + 1.0)
            knee = add(start, vec(spec.leg_upper, upper + p.torso_tilt * 0.07))
            ankle = add(knee, vec(spec.leg_lower, lower + p.torso_tilt * 0.05))
            return start, knee, ankle

        far_hip, far_knee, far_ankle = leg_points(False)
        near_hip, near_knee, near_ankle = leg_points(True)
        limb(far_hip, far_knee, far_ankle, spec.leg_radius, col("cloth_dark"), col("outline"))
        limb(near_hip, near_knee, near_ankle, spec.leg_radius, col("cloth"), col("outline"))
        for ankle, sign, fill in ((far_ankle, -1.0, col("wrap")), (near_ankle, 1.0, col("armor_dark"))):
            draw_rotated_ellipse(
                img,
                sp((ankle[0] + sign * 4.3, ankle[1] + 2.5)),
                (sc(spec.foot_w), sc(spec.foot_h)),
                7.0 * sign,
                fill,
                col("outline"),
                sc(1.1),
            )

        # Torso / hip sash.
        body_outline = col("outline")
        shoulder_l = sp((torso[0] - spec.shoulder_w / 2.0, torso[1] - 13.0))
        shoulder_r = sp((torso[0] + spec.shoulder_w / 2.0, torso[1] - 11.0))
        waist_l = sp((hip[0] - spec.hip_w / 2.0, hip[1] - 4.0))
        waist_r = sp((hip[0] + spec.hip_w / 2.0, hip[1] - 5.0))
        d.polygon([shoulder_l, shoulder_r, waist_r, waist_l], fill=body_outline)
        d.polygon(
            [
                sp((torso[0] - spec.shoulder_w / 2.0 + 3.0, torso[1] - 10.5)),
                sp((torso[0] + spec.shoulder_w / 2.0 - 3.0, torso[1] - 9.0)),
                sp((hip[0] + spec.hip_w / 2.0 - 3.0, hip[1] - 7.0)),
                sp((hip[0] - spec.hip_w / 2.0 + 3.0, hip[1] - 6.0)),
            ],
            fill=col("cloth"),
        )
        # Chest armor: angular panels instead of generic shirt shapes.
        d.polygon(
            [sp((torso[0] - 15.0, torso[1] - 9.0)), sp((torso[0] - 1.0, torso[1] - 12.0)), sp((torso[0] - 3.0, torso[1] + 5.0)), sp((torso[0] - 14.0, torso[1] + 8.0))],
            fill=col("armor"),
        )
        d.polygon(
            [sp((torso[0] + 1.0, torso[1] - 11.0)), sp((torso[0] + 15.0, torso[1] - 8.0)), sp((torso[0] + 13.0, torso[1] + 7.0)), sp((torso[0] + 2.0, torso[1] + 4.0))],
            fill=col("armor_dark"),
        )
        d.line([sp((torso[0] - 11.0, torso[1] - 11.0)), sp((torso[0] - 2.0, torso[1] - 13.0))], fill=col("cloth_light"), width=max(1, int(sc(1.2))))
        d.line([sp((torso[0] + 4.0, torso[1] - 11.0)), sp((torso[0] + 15.0, torso[1] - 8.0))], fill=with_alpha(pal["cloth_light"], 150), width=max(1, int(sc(1.0))))
        # Little cyan-gray moon glyph for recognizability at review scale.
        d.arc((sp((torso[0] - 4.5, torso[1] - 2.0))[0], sp((torso[0] - 4.5, torso[1] - 2.0))[1], sp((torso[0] + 5.5, torso[1] + 8.0))[0], sp((torso[0] + 5.5, torso[1] + 8.0))[1]), 70, 275, fill=with_alpha(pal["blade_shadow"], 180), width=max(1, int(sc(1.2))))
        if leader:
            # Lamellar commander plates: broad, square, and red-riveted so the
            # leader does not collapse into the normal duelist chest shape.
            for yoff in (-4.0, 2.5, 8.5):
                d.line([sp((torso[0] - 15.5, torso[1] + yoff)), sp((torso[0] + 15.0, torso[1] + yoff + 1.0))], fill=with_alpha(pal["outline"], 190), width=max(1, int(sc(0.9))))
            for xoff in (-9.0, 0.0, 9.0):
                d.line([sp((torso[0] + xoff, torso[1] - 9.0)), sp((torso[0] + xoff * 0.75, torso[1] + 12.0))], fill=with_alpha(pal["outline"], 155), width=max(1, int(sc(0.8))))
            for xoff in (-12.5, -4.0, 5.0, 13.0):
                d.ellipse((sc(torso[0] + xoff - 1.0), sc(torso[1] + 2.0), sc(torso[0] + xoff + 1.0), sc(torso[1] + 4.0)), fill=with_alpha(pal["eye"], 120))

        # Belt / sash and trailing knot.
        if leader:
            skirt_y = hip[1] + spec.skirt_len
            # Three tattered armor-cloth panels form a skirted commander profile.
            panels = [
                [(hip[0] - 20.0, hip[1] - 2.0), (hip[0] - 7.0, hip[1] + 0.5), (hip[0] - 10.5, skirt_y - 4.0), (hip[0] - 25.0, skirt_y + 1.5)],
                [(hip[0] - 8.5, hip[1] - 1.0), (hip[0] + 8.5, hip[1] - 1.0), (hip[0] + 5.5, skirt_y + 4.0), (hip[0] - 2.0, skirt_y + 8.0), (hip[0] - 10.0, skirt_y + 2.0)],
                [(hip[0] + 7.0, hip[1] + 0.0), (hip[0] + 22.0, hip[1] - 2.0), (hip[0] + 27.0, skirt_y + 1.0), (hip[0] + 11.0, skirt_y - 3.0)],
            ]
            for idx, pts in enumerate(panels):
                d.polygon([sp(pt) for pt in pts], fill=col("outline"))
                inset_pts = [(x * 0.92 + hip[0] * 0.08, y * 0.96 + hip[1] * 0.04) for x, y in pts]
                d.polygon([sp(pt) for pt in inset_pts], fill=col("cloth_dark" if idx != 1 else "wrap"))
            crest_center = sp((hip[0] - 1.5, hip[1] + 19.0))
            crest_r = sc(4.6 * spec.crest_scale)
            crest_col = with_alpha(pal["eye"], int(132 * (1.0 - p.fade)))
            d.ellipse((crest_center[0] - crest_r, crest_center[1] - crest_r, crest_center[0] + crest_r, crest_center[1] + crest_r), outline=crest_col, width=max(1, int(sc(1.2))))
            d.line((crest_center[0], crest_center[1] - crest_r * 0.85, crest_center[0], crest_center[1] + crest_r * 0.85), fill=crest_col, width=max(1, int(sc(1.0))))
        draw_rotated_rounded_rect(img, sp((hip[0] + 0.5, hip[1] - 3.0)), (sc(33.0 * spec.armor_bulk), sc(8.0)), -2.0, sc(3.0), col("sash"), col("outline"), sc(1.2))
        d.rectangle((sc(hip[0] - 4.0), sc(hip[1] - 7.0), sc(hip[0] + 5.0), sc(hip[1] - 1.0)), fill=col("brass"))
        self._draw_scarf_tail(
            d,
            (
                sp((hip[0] + 14.0, hip[1] - 5.0)),
                sp((hip[0] + 24.0, hip[1] - 8.0 + p.sash_swing * 0.3)),
                sp((hip[0] + spec.sash_len * 0.72, hip[1] - 1.0 + p.sash_swing * 0.7)),
                sp((hip[0] + spec.sash_len, hip[1] - 3.0 + p.sash_swing)),
            ),
            col("sash"),
            col("outline"),
        )

        if leader:
            # Oversized sode/pauldrons give the leader a readable broad-shouldered
            # command silhouette even after the final sheet crop.
            for sign in (-1.0, 1.0):
                sx = torso[0] + sign * spec.shoulder_w * 0.45
                sy = torso[1] - 11.5
                outer = sx + sign * 13.5 * spec.pauldron_scale
                pts = [
                    (sx - sign * 3.0, sy - 5.5),
                    (outer, sy - 2.0),
                    (outer - sign * 3.5, sy + 13.5),
                    (sx - sign * 8.0, sy + 11.0),
                ]
                d.polygon([sp(pt) for pt in pts], fill=col("outline"))
                inner = [(x * 0.88 + sx * 0.12, y * 0.90 + sy * 0.10) for x, y in pts]
                d.polygon([sp(pt) for pt in inner], fill=col("armor_dark"))
                # Blade-like top spike, dark enough to keep silhouette clean.
                spike = [(sx + sign * 1.0, sy - 5.0), (outer + sign * 2.0, sy - 8.5), (sx + sign * 5.0, sy + 0.5)]
                d.polygon([sp(pt) for pt in spike], fill=col("outline"))
                d.line([sp((sx - sign * 1.0, sy + 2.0)), sp((outer - sign * 4.0, sy + 3.0))], fill=with_alpha(pal["cloth_light"], 120), width=max(1, int(sc(0.9))))

        # Arms and hands; far arm first.
        def arm_points(is_near: bool) -> Tuple[Point, Point, Point]:
            if is_near:
                shoulder = (torso[0] + spec.shoulder_w * (0.39 if leader else 0.34), torso[1] - 10.0)
                if leader and p.slash <= 0.08:
                    upper = 74.0 + p.torso_tilt * 0.06
                    lower = 92.0 + p.torso_tilt * 0.05
                else:
                    upper = p.near_arm_upper + p.torso_tilt * 0.10
                    lower = p.near_arm_lower + p.torso_tilt * 0.08
            else:
                shoulder = (torso[0] - spec.shoulder_w * (0.39 if leader else 0.34), torso[1] - 10.5)
                if leader and p.slash <= 0.08:
                    upper = 106.0 + p.torso_tilt * 0.06
                    lower = 84.0 + p.torso_tilt * 0.05
                else:
                    upper = p.far_arm_upper + p.torso_tilt * 0.08
                    lower = p.far_arm_lower + p.torso_tilt * 0.08
            elbow = add(shoulder, vec(spec.arm_upper, upper))
            hand = add(elbow, vec(spec.arm_lower, lower))
            if leader and p.slash <= 0.08:
                # A commanding idle: one hand rests near the sword, the other
                # drops naturally.  This avoids the double-grip arms that were
                # acceptable for the duelist but odd for a leader pose.
                if not is_near:
                    hand = _mix((hand[0], hand[1], 0, 255), (hilt[0] - 0.5, hilt[1] + 0.5, 0, 255), 0.24)
            else:
                # Pull both hands toward the sword hilt in the duelist / slash
                # poses; this avoids the disconnected accessory look common in
                # procedural rigs.
                hand = _mix((hand[0], hand[1], 0, 255), (hilt[0] + (2.5 if is_near else -2.0), hilt[1] + (2.5 if is_near else -2.5), 0, 255), 0.52)
            return shoulder, elbow, (hand[0], hand[1])

        far_sh, far_el, far_hand = arm_points(False)
        near_sh, near_el, near_hand = arm_points(True)
        limb(far_sh, far_el, far_hand, spec.arm_radius, col("cloth_dark"), col("outline"))
        limb(near_sh, near_el, near_hand, spec.arm_radius, col("cloth_mid"), col("outline"))
        if leader:
            # Repaint the hard pauldrons over the arm capsules so the leader
            # keeps angular shoulders instead of round robot-like joints.
            for sign in (-1.0, 1.0):
                sx = torso[0] + sign * spec.shoulder_w * 0.45
                sy = torso[1] - 11.5
                outer = sx + sign * 13.5 * spec.pauldron_scale
                pts = [
                    (sx - sign * 3.0, sy - 5.5),
                    (outer, sy - 2.0),
                    (outer - sign * 3.5, sy + 13.5),
                    (sx - sign * 8.0, sy + 11.0),
                ]
                d.polygon([sp(pt) for pt in pts], fill=col("outline"))
                inner = [(x * 0.88 + sx * 0.12, y * 0.90 + sy * 0.10) for x, y in pts]
                d.polygon([sp(pt) for pt in inner], fill=col("armor_dark"))
                spike = [(sx + sign * 1.0, sy - 5.0), (outer + sign * 2.0, sy - 8.5), (sx + sign * 5.0, sy + 0.5)]
                d.polygon([sp(pt) for pt in spike], fill=col("outline"))
                d.line([sp((sx - sign * 1.0, sy + 2.0)), sp((outer - sign * 4.0, sy + 3.0))], fill=with_alpha(pal["cloth_light"], 120), width=max(1, int(sc(0.9))))
        # Hand wraps / guards.
        for hand in (far_hand, near_hand):
            d.ellipse((sc(hand[0] - spec.hand_r), sc(hand[1] - spec.hand_r), sc(hand[0] + spec.hand_r), sc(hand[1] + spec.hand_r)), fill=col("wrap"), outline=col("outline"), width=max(1, int(sc(1.0))))
        # Hilt drawn after hands so the grip reads as held.
        ux, uy = math.cos(math.radians(p.sword_angle + 90.0)), math.sin(math.radians(p.sword_angle + 90.0))
        h0 = sp((hilt[0] - ux * 9.0, hilt[1] - uy * 9.0))
        h1 = sp((hilt[0] + ux * 9.0, hilt[1] + uy * 9.0))
        d.line([h0, h1], fill=col("outline"), width=max(1, int(sc(5.0))))
        d.line([h0, h1], fill=col("brass"), width=max(1, int(sc(2.7))))
        draw_rotated_rounded_rect(img, sp(hilt), (sc(14.0), sc(4.0)), p.sword_angle + 90.0, sc(2.0), col("armor_dark"), col("outline"), sc(1.0))

        # Neck, horns, and head.
        if spec.horn_len > 0.0:
            for sign in (-1.0, 1.0):
                # Wide, curved oni horns.  They intentionally sit outside the
                # helmet ellipse so the leader silhouette survives sprite scale.
                horn_base = (head[0] + sign * 7.0, head[1] - 12.0)
                horn_mid = (head[0] + sign * (12.5 + spec.horn_len * 0.20), head[1] - 19.0)
                horn_tip = (head[0] + sign * (16.0 + spec.horn_len * 0.20), max(1.5, head[1] - 20.0 - spec.horn_len * 0.45))
                horn_path = [sp(horn_base), sp(horn_mid), sp(horn_tip)]
                d.line(horn_path, fill=col("outline"), width=max(3, int(sc(5.4))), joint="curve")
                d.line(horn_path, fill=col("sash_dark"), width=max(2, int(sc(3.1))), joint="curve")
                tip = sp(horn_tip)
                d.polygon(
                    [tip, sp((horn_tip[0] - sign * 2.8, horn_tip[1] + 5.0)), sp((horn_tip[0] - sign * 0.2, horn_tip[1] + 1.0))],
                    fill=col("outline"),
                )
                d.line([sp(horn_base), sp(horn_tip)], fill=with_alpha(pal["eye_hot"], 92), width=max(1, int(sc(0.9))))
        draw_rotated_rounded_rect(img, sp(neck), (sc(9.0), sc(13.0)), p.head_tilt, sc(3.0), col("cloth_dark"), col("outline"), sc(1.0))
        draw_rotated_ellipse(img, sp(head), (sc(spec.head_w + 2.5), sc(spec.head_h + 1.5)), p.head_tilt, col("outline"), None, 0)
        draw_rotated_ellipse(img, sp((head[0] - 0.7, head[1] - 0.5)), (sc(spec.head_w), sc(spec.head_h)), p.head_tilt, col("cloth"), None, 0)
        # Hood top cap / brow wrap.
        draw_rotated_rounded_rect(img, sp((head[0] - 0.8, head[1] - 9.0)), (sc(spec.head_w * 0.90), sc(8.0)), p.head_tilt - 2.0, sc(4.0), col("cloth_mid"), None, 0)
        draw_rotated_rounded_rect(img, sp((head[0] - 0.3, head[1] - 1.5)), (sc(spec.head_w * 0.92), sc(7.3)), p.head_tilt, sc(3.0), col("wrap"), col("outline"), sc(0.9))
        # Face-mask lower half with a cheek highlight and nose plane.
        d.arc((sc(head[0] - 11.0), sc(head[1] + 1.0), sc(head[0] + 12.0), sc(head[1] + 15.0)), 10, 165, fill=col("outline"), width=max(1, int(sc(1.0))))
        d.line([sp((head[0] + 5.5, head[1] - 10.5)), sp((head[0] + 11.0, head[1] - 6.0))], fill=with_alpha(pal["cloth_light"], 145), width=max(1, int(sc(1.0))))
        d.line([sp((head[0] - 5.0, head[1] + 9.0)), sp((head[0] + 2.0, head[1] + 11.0))], fill=col("armor_dark"), width=max(1, int(sc(1.2))))
        if leader:
            tusk = with_alpha(pal["blade_shadow"], int(210 * (1.0 - p.fade)))
            d.polygon([sp((head[0] - 8.5, head[1] + 5.0)), sp((head[0] - 5.0, head[1] + 7.8)), sp((head[0] - 7.2, head[1] + 12.0))], fill=tusk, outline=col("outline"))
            d.polygon([sp((head[0] + 8.5, head[1] + 4.6)), sp((head[0] + 5.0, head[1] + 7.5)), sp((head[0] + 7.4, head[1] + 11.6))], fill=tusk, outline=col("outline"))

        # Red eye slits: the strongest reference-inspired feature, but drawn as
        # sharp triangular slashes rather than the source's exact eye shapes.
        eye_alpha = int(230 * spec.eye_glow * (1.0 - p.fade))
        eye_h = max(1.5, 3.3 - p.eye_squint * 1.4)
        left_eye = [sp((head[0] - 8.3, head[1] - 3.5)), sp((head[0] - 1.7, head[1] - 2.6)), sp((head[0] - 3.4, head[1] - 2.6 + eye_h))]
        right_eye = [sp((head[0] + 2.0, head[1] - 2.9)), sp((head[0] + 9.2, head[1] - 4.4)), sp((head[0] + 6.2, head[1] - 1.1 + eye_h))]
        d.polygon(left_eye, fill=with_alpha(pal["eye"], eye_alpha))
        d.polygon(right_eye, fill=with_alpha(pal["eye"], eye_alpha))
        d.line([left_eye[0], left_eye[1]], fill=with_alpha(pal["eye_hot"], min(255, eye_alpha + 35)), width=max(1, int(sc(0.8))))
        d.line([right_eye[0], right_eye[1]], fill=with_alpha(pal["eye_hot"], min(255, eye_alpha + 35)), width=max(1, int(sc(0.8))))

        if p.hit > 0:
            # Brief red rim on hit frames.
            d.arc((sc(28.0), sc(22.0), sc(101.0), sc(118.0)), 205, 305, fill=with_alpha(pal["eye"], int(110 * p.hit)), width=max(1, int(sc(2.0))))

    def render_animation_frame(
        self,
        spec: NinjaSpec,
        animation: str,
        frame_index: int,
        frame_count: int,
        size: Tuple[int, int] = (128, 128),
        *,
        background: Optional[Color] = None,
        supersample: int = 4,
        downsample: str = "lanczos",
    ) -> Image.Image:
        w, h = size
        ss = max(1, int(supersample))
        high = Image.new("RGBA", (w * ss, h * ss), background or (0, 0, 0, 0))
        scale = (w / 128.0) * ss
        pose = self.pose_for_animation(animation, frame_index, frame_count, spec)
        self._draw_ninja(high, spec, pose, scale)
        resample = RESAMPLING.NEAREST if downsample == "nearest" else RESAMPLING.LANCZOS
        return high.resize(size, resample)
