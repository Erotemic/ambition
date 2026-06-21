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
from typing import Callable, Dict, Optional, Tuple

from PIL import Image, ImageColor, ImageDraw
from ambition_sprite2d_renderer.core.draw import rgba, with_alpha, bbox_from_center as _bbox

from ...authoring.common_draw import RESAMPLING, draw_capsule, draw_rotated_ellipse, draw_rotated_rounded_rect
from ...authoring.rig import add, clamp, ease_in_out_sine, ease_out_cubic, smoothstep, vec

Color = Tuple[int, int, int, int]
Point = Tuple[float, float]


# --- generic helpers -----------------------------------------------------------







def parse_background(value: str) -> Optional[Color]:
    return None if str(value).lower() == "transparent" else rgba(str(value))






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
    # ---- Per-archetype rig flags ---------------------------------------
    # Set on the preset rather than checked against a hardcoded set of
    # archetype names in the rig. New "feminine-coded" toons just set
    # `feminine_coded: True` in their preset; no rig edit required.
    feminine_coded: bool = False
    # Optional callable run AFTER `pose_for_animation` builds the base
    # pose, signature `(pose: ToonPose, animation: str) -> None` that
    # mutates the pose for archetype-specific touch-ups (e.g.
    # raid_enforcer's stiffer posture). Avoids `if archetype == "X":`
    # blocks inside the rig.
    pose_override: Optional[Callable[["ToonPose", str], None]] = None


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


from ._toon_palettes import PALETTES as _TOON_PALETTES
from ._toon_presets import PRESETS as _TOON_PRESETS


class ToonSideGenerator:
    PALETTES = _TOON_PALETTES
    PRESETS = _TOON_PRESETS

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

    def _clamp_leg_target(self, hip: Point, ankle: Point, upper_len: float, lower_len: float, pad: float = 0.8) -> Point:
        dx = ankle[0] - hip[0]
        dy = ankle[1] - hip[1]
        dist = math.hypot(dx, dy)
        min_reach = abs(upper_len - lower_len) + 0.001
        max_reach = max(min_reach + 0.001, upper_len + lower_len - pad)
        if dist == 0.0:
            return (hip[0] + min_reach, hip[1])
        if dist < min_reach:
            scale = min_reach / dist
            return (hip[0] + dx * scale, hip[1] + dy * scale)
        if dist > max_reach:
            scale = max_reach / dist
            return (hip[0] + dx * scale, hip[1] + dy * scale)
        return ankle

    def _solve_leg_ik(self, hip: Point, ankle: Point, upper_len: float, lower_len: float, bend_sign: float = 1.0) -> Tuple[Point, float, float]:
        """Solve a two-bone side-view leg toward a reachable ankle target."""
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

    def pose_for_animation(self, animation: str, frame_index: int, frame_count: int, spec: ToonSpec) -> ToonPose:
        p = ToonPose()
        t = 0.0 if frame_count <= 1 else frame_index / float(frame_count - 1)
        wave = math.sin(t * math.tau)
        plan = spec.body_plan
        run_scale = 1.0 if plan not in {"round", "soft"} else 0.82
        if animation == "idle":
            # Two-cycle breath layered over a slower body sway so the
            # NPC reads as alive rather than mechanically swinging
            # back-and-forth. The slower sway uses the loop's full
            # period (`wave`), the breath uses half-period for visible
            # chest rise without forcing the silhouette off-balance.
            breath = math.sin(t * math.tau * 2.0)
            p.body_bob = (0.55 + abs(breath) * 0.55) * (1.1 if plan != "tall" else 0.7)
            p.torso_tilt = wave * (1.2 if plan != "broad" else 0.6)
            p.head_tilt = -wave * 0.8 + breath * 0.25
            # Subtle near-arm drift and an occasional small gesture
            # part-way through the loop so the figure doesn't look
            # frozen between blinks. Use small numbers — anything
            # larger reads as a fidget instead of "idle".
            gesture_t = clamp((t - 0.55) / 0.3, 0.0, 1.0)
            gesture_pulse = gesture_t * (1.0 - gesture_t) * 4.0
            p.near_arm_upper = 24.0 + breath * 1.4 - gesture_pulse * 3.5
            p.near_arm_lower = 18.0 + breath * 1.0 - gesture_pulse * 2.5
            p.far_arm_upper = 150.0 - breath * 1.1
            p.far_arm_lower = 132.0 - breath * 0.7
            p.gesture = gesture_pulse * 0.35
            # Blink-and-glance pattern: blink near the middle, a brief
            # eye-squint near the start and end, plus a subtle pupil
            # squint during the gesture so the eye direction feels
            # connected to the small arm move.
            p.blink = frame_index == frame_count // 2
            p.eye_squint = 0.12 if frame_index in {1, frame_count - 2} else 0.0
            if gesture_pulse > 0.1:
                p.eye_squint = max(p.eye_squint, 0.05 + gesture_pulse * 0.08)
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
        # Archetype-specific pose touch-ups (e.g. raid_enforcer's
        # stiffer posture) live on the preset as a callable so the rig
        # stays archetype-agnostic. See `_toon_presets/raid_enforcer.py`.
        if spec.pose_override is not None:
            spec.pose_override(p, animation)
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
        elif spec.hair_style == "savant_cap":
            # Soft cloth turban / nightcap (Emanuel Handmann 1753 portrait
            # of Euler). The cap is a rounded mass over the skull, slightly
            # peaked on one side, with a visible cap band where it meets
            # the forehead. We deliberately do NOT draw side hair or a
            # tail — period portraits of Euler in this style show only the
            # cap silhouette over a clean face, which is the simplest way
            # to make the character read as the mathematician at first
            # glance. `pal["cap"]` is the cap fabric color (fall back to
            # outfit if a palette pre-dates the field).
            cap_fill = pal.get("cap", pal["outfit"])
            cap_band = pal.get("cap_band", pal["outfit_dark"])
            # Main cap dome: wider than the head, taller above the brow.
            cap_w = (spec.head_w + spec.hair_volume * 1.6 + 6.0) * S
            cap_h = (spec.head_h * 0.85 + spec.hair_volume * 0.55) * S
            cap_cx = c[0] - 0.5 * S
            cap_cy = c[1] - spec.head_h * 0.30 * S
            d.ellipse(_bbox((cap_cx, cap_cy), cap_w, cap_h), fill=cap_fill, outline=outline, width=max(1, int(1.2 * S)))
            # Soft fold/peak leaning forward (camera-right) so the cap
            # silhouette has a recognizable Handmann-style nipple shape
            # instead of a perfect dome.
            peak = [
                (cap_cx - 1.0 * S, cap_cy - cap_h * 0.45),
                (cap_cx + cap_w * 0.18, cap_cy - cap_h * 0.78),
                (cap_cx + cap_w * 0.32, cap_cy - cap_h * 0.42),
                (cap_cx + cap_w * 0.06, cap_cy - cap_h * 0.30),
            ]
            d.polygon(peak, fill=cap_fill, outline=outline)
            # Cap band along the forehead — a darker strip with two short
            # tabs that read as the fabric's hem.
            band_top = c[1] - spec.head_h * 0.18 * S
            band_bot = c[1] - spec.head_h * 0.04 * S
            d.rounded_rectangle(
                (c[0] - spec.head_w * 0.50 * S, band_top, c[0] + spec.head_w * 0.50 * S, band_bot),
                radius=2.0 * S,
                fill=cap_band,
                outline=outline,
                width=max(1, int(1.0 * S)),
            )
            # Highlight curl on top of the dome so it doesn't read flat.
            d.ellipse(
                _bbox((cap_cx + cap_w * 0.05, cap_cy - cap_h * 0.18), cap_w * 0.42, cap_h * 0.28),
                fill=_scale_color(cap_fill, 1.18),
                outline=None,
            )
            # A small triangular shadow under the peak fold lifts it off
            # the dome and gives the cap a believable crease.
            d.polygon([
                (cap_cx + cap_w * 0.08, cap_cy - cap_h * 0.32),
                (cap_cx + cap_w * 0.26, cap_cy - cap_h * 0.20),
                (cap_cx + cap_w * 0.10, cap_cy - cap_h * 0.10),
            ], fill=_scale_color(cap_fill, 0.78), outline=None)
        elif spec.hair_style == "combed_back_balding":
            # Gray, receding, combed-back hair with a few wilder strands
            # at the back (Erdős). The hairline starts well behind the
            # forehead so a clear band of skin shows up front, then the
            # hair mass sweeps up and back over the crown. A handful of
            # short stragglers behind the ear keep it from reading
            # corporate.
            # Back-of-skull cap, but pulled BACK from the brow so the
            # forehead skin is exposed.
            d.ellipse(
                _bbox(
                    (c[0] + 1.0 * S, c[1] - spec.head_h * 0.28 * S),
                    (spec.head_w * 0.92 + spec.hair_volume * 0.7) * S,
                    (spec.head_h * 0.55 + spec.hair_volume * 0.35) * S,
                ),
                fill=pal["hair"],
                outline=outline,
                width=max(1, int(1.0 * S)),
            )
            # Combed-back streaks: a few thin parallel lines following the
            # crown curve, drawn in the lighter hair_shine to suggest a
            # combed texture rather than a solid block.
            for i in range(4):
                t_streak = i / 3.0
                start = (c[0] - spec.head_w * 0.30 * S + t_streak * spec.head_w * 0.55 * S, c[1] - spec.head_h * 0.55 * S + t_streak * 1.6 * S)
                end = (c[0] + spec.head_w * 0.42 * S, c[1] - spec.head_h * 0.34 * S + t_streak * 4.0 * S)
                d.line([start, end], fill=pal["hair_shine"], width=max(1, int(0.9 * S)))
            # Stray wisps poking out at the back (camera-left) so the
            # silhouette has a slightly wild edge without losing the
            # tidy "combed back" read.
            for dy, dx in [(-2.4, -4.6), (1.0, -5.4), (5.0, -4.0)]:
                wisp_tip = (c[0] - spec.head_w * 0.52 * S + dx * S, c[1] - spec.head_h * 0.10 * S + dy * S)
                wisp_root = (c[0] - spec.head_w * 0.38 * S, c[1] - spec.head_h * 0.18 * S + dy * S * 0.6)
                d.line([wisp_root, wisp_tip], fill=pal["hair"], width=max(1, int(1.5 * S)))
            # Slight forehead temple recession line so the receding
            # hairline is visible even at downsample: two short skin
            # strokes carving into the front of the cap.
            for sign in (-1, 1):
                temple_tip = (c[0] + sign * spec.head_w * 0.30 * S, c[1] - spec.head_h * 0.30 * S)
                temple_base = (c[0] + sign * spec.head_w * 0.16 * S, c[1] - spec.head_h * 0.20 * S)
                d.polygon([
                    temple_tip,
                    temple_base,
                    (temple_base[0], temple_base[1] + 3.0 * S),
                    (temple_tip[0], temple_tip[1] + 3.0 * S),
                ], fill=pal["skin"], outline=None)
        elif spec.hair_style == "barrister_wig":
            # Full-bottom barrister/judge wig — three tiers of side
            # curls cascading past the cheek and a tightly-curled
            # crown that hugs the skull. Visually distinct from
            # `savant_cap` (Newton): horizontal-banded curls instead
            # of a smooth turban, and the curl tiers project sideways
            # past the head rather than tucking back.
            wig = pal["hair"]
            wig_shine = pal["hair_shine"]
            # Crown: a wider-than-the-head cap of tight curl texture.
            crown_w = (spec.head_w + spec.hair_volume * 1.6 + 6.0) * S
            crown_h = (spec.head_h * 0.86 + spec.hair_volume * 0.50) * S
            crown_cx = c[0] - 1.0 * S
            crown_cy = c[1] - spec.head_h * 0.18 * S
            d.ellipse(_bbox((crown_cx, crown_cy), crown_w, crown_h), fill=wig, outline=outline, width=max(1, int(1.1 * S)))
            # Three stacked curl tiers on each side, getting slightly
            # wider as they go down. Each tier is a row of small
            # ellipses for the "tight ringlet" texture.
            for tier_idx, dy in enumerate((-2.0, 5.0, 12.0)):
                tier_w = (spec.head_w * 0.46 + tier_idx * 3.0) * S
                tier_y = c[1] + dy * S
                for sign in (-1, 1):
                    base_x = c[0] + sign * (spec.head_w * 0.42 + tier_idx * 1.2) * S
                    d.ellipse(_bbox((base_x, tier_y), 5.6 * S, 4.2 * S), fill=wig, outline=outline, width=max(1, int(0.9 * S)))
                    d.ellipse(_bbox((base_x - sign * 1.4 * S, tier_y - 1.0 * S), 2.2 * S, 1.4 * S), fill=wig_shine, outline=None)
                # subtle horizontal band across the back of the crown
                # so the curls read as stacked rows even at downsample.
                d.line(
                    [(crown_cx - crown_w * 0.40, tier_y - 1.0 * S), (crown_cx + crown_w * 0.18, tier_y - 1.0 * S)],
                    fill=_scale_color(wig, 0.85),
                    width=max(1, int(0.7 * S)),
                )
            # Highlight curl on the crown so the dome doesn't read flat.
            d.ellipse(_bbox((crown_cx + 2.0 * S, crown_cy - crown_h * 0.32), crown_w * 0.38, crown_h * 0.22), fill=wig_shine, outline=None)
        elif spec.hair_style == "clean_bald":
            # Trent — bald top with a tidy fringe of gray hair only
            # behind the ears. No top mass at all so the face dome
            # reads as bare skin; the fringe runs as a low horizontal
            # band that traces just above the ear line.
            fringe_y = c[1] + spec.head_h * 0.04 * S
            fringe_w = (spec.head_w + 2.0) * S
            fringe_h = (spec.head_h * 0.40) * S
            # Side fringe: a low arc behind both temples, drawn as a
            # pieslice so the front-facing portion stays clear.
            d.pieslice(
                _bbox((c[0] - 1.0 * S, fringe_y), fringe_w, fringe_h),
                start=180,
                end=360,
                fill=pal["hair"],
                outline=outline,
                width=max(1, int(1.0 * S)),
            )
            # A short tuft just behind the ear (camera-right) so the
            # silhouette has a small notch instead of a smooth dome.
            d.ellipse(
                _bbox((c[0] + spec.head_w * 0.40 * S, fringe_y + 2.0 * S), 3.5 * S, 3.0 * S),
                fill=pal["hair"],
                outline=outline,
                width=max(1, int(0.8 * S)),
            )
        elif spec.hair_style == "long_side_braid":
            # Alice — full-head hair mass that drops a LONG single
            # braid OVER the camera-side shoulder, with full forehead
            # bangs framing the face. Different silhouette from any
            # male-default toon: the hair has bangs + side curtains
            # + a 9-segment forward braid that falls past her ribs.
            # Back-of-skull mass: wider than the small bob crown so
            # the head reads as having full hair.
            d.ellipse(
                _bbox((c[0] - 1.0 * S, c[1] - 5.0 * S), (spec.head_w + spec.hair_volume + 2.0) * S, (spec.head_h * 0.92 + spec.hair_volume * 0.60) * S),
                fill=pal["hair"],
                outline=outline,
                width=max(1, int(1.1 * S)),
            )
            # Side hair drops past the jaw on both sides of the face
            # (curtain that frames the cheeks — doesn't cover the
            # face, hangs in front of the ears).
            for sign in (-1, 1):
                curtain = [
                    (c[0] + sign * spec.head_w * 0.50 * S, c[1] - spec.head_h * 0.30 * S),
                    (c[0] + sign * spec.head_w * 0.60 * S, c[1] + spec.head_h * 0.10 * S),
                    (c[0] + sign * spec.head_w * 0.42 * S, c[1] + spec.head_h * 0.34 * S),
                    (c[0] + sign * spec.head_w * 0.32 * S, c[1] + spec.head_h * 0.10 * S),
                ]
                d.polygon(curtain, fill=pal["hair"], outline=outline)
            # Forehead bangs — a soft fringe that crosses the brow,
            # broken into two clumps that meet at a central part so
            # the eyes stay visible. Reads as "she has bangs" even
            # at the runtime downsample.
            left_bang = [
                (c[0] - spec.head_w * 0.50 * S, c[1] - spec.head_h * 0.42 * S),
                (c[0] - spec.head_w * 0.06 * S, c[1] - spec.head_h * 0.48 * S),
                (c[0] - spec.head_w * 0.02 * S, c[1] - spec.head_h * 0.14 * S),
                (c[0] - spec.head_w * 0.38 * S, c[1] - spec.head_h * 0.06 * S),
            ]
            right_bang = [
                (c[0] + spec.head_w * 0.04 * S, c[1] - spec.head_h * 0.48 * S),
                (c[0] + spec.head_w * 0.48 * S, c[1] - spec.head_h * 0.40 * S),
                (c[0] + spec.head_w * 0.40 * S, c[1] - spec.head_h * 0.04 * S),
                (c[0] + spec.head_w * 0.06 * S, c[1] - spec.head_h * 0.16 * S),
            ]
            d.polygon(left_bang, fill=pal["hair"], outline=outline)
            d.polygon(right_bang, fill=pal["hair"], outline=outline)
            # Small skin-colored part-line so the bangs don't read
            # as a solid mop. Subtle but it breaks the silhouette.
            d.line([
                (c[0] - spec.head_w * 0.02 * S, c[1] - spec.head_h * 0.46 * S),
                (c[0] + spec.head_w * 0.02 * S, c[1] - spec.head_h * 0.18 * S),
            ], fill=pal["skin"], width=max(1, int(0.9 * S)))
            # The braid: a LONG stack of 9 ellipses falling forward
            # over the camera-side (+x) shoulder, past the chest with
            # a ribbon-tied tip. Was 6 segments; bumped to 9 +
            # tapered widths so the braid extends visibly past the
            # ribcage instead of stopping at the shoulder.
            braid_anchor_x = c[0] + spec.head_w * 0.32 * S
            braid_anchor_y = c[1] + spec.head_h * 0.34 * S
            braid_segs = (
                (2.0, 4.8), (8.0, 4.6), (14.0, 4.2), (20.0, 3.9),
                (26.0, 3.6), (32.0, 3.3), (38.0, 3.0), (44.0, 2.6),
                (50.0, 2.1),
            )
            for i, (dy, w) in enumerate(braid_segs):
                seg_c = (braid_anchor_x - i * 0.5 * S, braid_anchor_y + dy * S)
                d.ellipse(_bbox(seg_c, w * S, 3.4 * S), fill=pal["hair"], outline=outline, width=max(1, int(0.9 * S)))
                # Braid weave shine on the upper-left of each segment.
                d.ellipse(_bbox((seg_c[0] - 0.8 * S, seg_c[1] - 1.0 * S), 1.6 * S, 1.0 * S), fill=pal["hair_shine"], outline=None)
            # Ribbon-tied tip at the bottom of the braid.
            last_dy, last_w = braid_segs[-1]
            tip_y = braid_anchor_y + (last_dy + 4.0) * S
            tip_x = braid_anchor_x - (len(braid_segs) - 1) * 0.5 * S
            d.rounded_rectangle(
                (tip_x - 4.0 * S, tip_y - 2.0 * S, tip_x + 0.0 * S, tip_y + 1.0 * S),
                radius=1.0 * S,
                fill=pal["accent"],
                outline=outline,
                width=max(1, int(0.7 * S)),
            )
        elif spec.hair_style == "forward_braid":
            # Mallory — undercut on one side, long braid forward over
            # the camera-side shoulder. The undercut + visible braid
            # silhouette is unmistakable feminine + tactical.
            # Top hair mass: tighter than long_side_braid, swept up.
            d.ellipse(
                _bbox((c[0] - 1.0 * S, c[1] - 6.0 * S), (spec.head_w + spec.hair_volume) * S, (spec.head_h * 0.74 + spec.hair_volume * 0.45) * S),
                fill=pal["hair"],
                outline=outline,
                width=max(1, int(1.1 * S)),
            )
            # Undercut band along the camera-far side: a thin strip
            # of skin exposed where the hair is shaved.
            for sign in (-1,):
                temple_top = (c[0] + sign * spec.head_w * 0.46 * S, c[1] - spec.head_h * 0.16 * S)
                temple_bot = (c[0] + sign * spec.head_w * 0.42 * S, c[1] + spec.head_h * 0.18 * S)
                d.polygon([
                    temple_top,
                    (temple_top[0] + sign * -2.0 * S, temple_top[1]),
                    (temple_bot[0] + sign * -2.0 * S, temple_bot[1]),
                    temple_bot,
                ], fill=pal["skin"], outline=None)
            # Bangs swept forward to one side (camera-right of the
            # face) so the kept-long side reads at a glance.
            d.polygon([
                (c[0] - 2.0 * S, c[1] - spec.head_h * 0.50 * S),
                (c[0] + 10.0 * S, c[1] - spec.head_h * 0.42 * S),
                (c[0] + 12.0 * S, c[1] - spec.head_h * 0.12 * S),
                (c[0] + 4.0 * S, c[1] - spec.head_h * 0.22 * S),
                (c[0] - 4.0 * S, c[1] - spec.head_h * 0.28 * S),
            ], fill=pal["hair"], outline=outline)
            # The forward braid: lays over the camera-side shoulder
            # in front of the face, hanging down past the chest.
            braid_anchor_x = c[0] + spec.head_w * 0.34 * S
            braid_anchor_y = c[1] + spec.head_h * 0.28 * S
            for i, (dy, w) in enumerate(((2.0, 4.4), (8.0, 4.2), (14.0, 3.8), (20.0, 3.4), (26.0, 3.0), (32.0, 2.6))):
                seg_c = (braid_anchor_x - i * 0.6 * S, braid_anchor_y + dy * S)
                d.ellipse(_bbox(seg_c, w * S, 3.2 * S), fill=pal["hair"], outline=outline, width=max(1, int(0.9 * S)))
                # Shine accent uses the brighter hair_shine so red-on-
                # red braids still read as woven.
                d.ellipse(_bbox((seg_c[0] - 0.8 * S, seg_c[1] - 1.0 * S), 1.6 * S, 1.0 * S), fill=pal["hair_shine"], outline=None)
            # Black ribbon at the braid tip (mallory's outfit color).
            tip_y = braid_anchor_y + 34.0 * S
            d.rounded_rectangle(
                (braid_anchor_x - 4.5 * S, tip_y - 2.0 * S, braid_anchor_x + 0.5 * S, tip_y + 1.0 * S),
                radius=1.0 * S,
                fill=pal["outfit"],
                outline=outline,
                width=max(1, int(0.7 * S)),
            )
        elif spec.hair_style == "many_braids":
            # Sybil — head full of small tight braids. Back mass is
            # rounded + textured with small dot accents in the
            # shine color to suggest individual braid tips.
            d.ellipse(_bbox((c[0] - 1.0 * S, c[1] - 4.0 * S), (spec.head_w + spec.hair_volume + 2.0) * S, (spec.head_h * 0.82 + spec.hair_volume * 0.5) * S), fill=pal["hair"], outline=outline, width=max(1, int(1.1 * S)))
            for bx, by in ((-7, -8), (-4, -10), (0, -11), (4, -10), (7, -8), (-9, -4), (9, -4), (-10, 0), (10, 0)):
                d.ellipse(_bbox((c[0] + bx * S, c[1] + by * S), 1.6 * S, 1.6 * S), fill=pal["hair_shine"], outline=None)
        elif spec.hair_style == "veiled":
            # Olivia — long veil draped from the crown. The veil
            # color is the OUTFIT shade (lavender) — not hair. The
            # actual hair beneath barely shows; the veil is the
            # silhouette.
            veil_color = pal.get("accent", pal["outfit"])
            d.ellipse(_bbox((c[0] - 0.5 * S, c[1] - 4.0 * S), (spec.head_w + spec.hair_volume + 4.0) * S, (spec.head_h * 0.84 + spec.hair_volume * 0.6) * S), fill=veil_color, outline=outline, width=max(1, int(1.1 * S)))
            # Long veil panels drop past the chin on both sides.
            for sign in (-1, 1):
                drop = [
                    (c[0] + sign * (spec.head_w * 0.46) * S, c[1] - spec.head_h * 0.10 * S),
                    (c[0] + sign * (spec.head_w * 0.56) * S, c[1] + spec.head_h * 0.32 * S),
                    (c[0] + sign * (spec.head_w * 0.38) * S, c[1] + spec.head_h * 0.62 * S),
                    (c[0] + sign * (spec.head_w * 0.20) * S, c[1] + spec.head_h * 0.30 * S),
                ]
                d.polygon(drop, fill=veil_color, outline=outline)
        elif spec.hair_style == "ponytail":
            # Peggy — short crown of hair plus a high ponytail
            # streaming out the back. The back mass is shorter +
            # the ponytail is a separate teardrop shape.
            d.ellipse(_bbox((c[0] - 0.5 * S, c[1] - 4.0 * S), (spec.head_w + spec.hair_volume) * S, (spec.head_h * 0.74 + spec.hair_volume * 0.45) * S), fill=pal["hair"], outline=outline, width=max(1, int(1.1 * S)))
            # Ponytail extending out the back (camera-left).
            tail = [
                (c[0] - spec.head_w * 0.32 * S, c[1] - spec.head_h * 0.22 * S),
                (c[0] - spec.head_w * 0.62 * S, c[1] - spec.head_h * 0.10 * S),
                (c[0] - spec.head_w * 0.72 * S, c[1] + spec.head_h * 0.18 * S),
                (c[0] - spec.head_w * 0.52 * S, c[1] + spec.head_h * 0.28 * S),
                (c[0] - spec.head_w * 0.28 * S, c[1] + spec.head_h * 0.06 * S),
            ]
            d.polygon(tail, fill=pal["hair"], outline=outline)
        elif spec.hair_style == "square_fringe":
            # Victor — precise blocky bangs across the forehead,
            # cropped close on the sides. The back mass is
            # geometric (a flat-bottomed ellipse).
            d.ellipse(_bbox((c[0] - 0.5 * S, c[1] - 5.0 * S), (spec.head_w + spec.hair_volume) * S, (spec.head_h * 0.74 + spec.hair_volume * 0.4) * S), fill=pal["hair"], outline=outline, width=max(1, int(1.1 * S)))
        elif spec.hair_style == "wide_brim_hat":
            # Craig — a wide-brimmed straw / felt hat. Brim is a
            # broad ellipse, crown is a shorter rounded rectangle.
            brim_w = (spec.head_w + spec.hair_volume + 16.0) * S
            brim_h = 5.0 * S
            brim_y = c[1] - spec.head_h * 0.32 * S
            d.ellipse(_bbox((c[0], brim_y), brim_w, brim_h), fill=pal["accent_dark"], outline=outline, width=max(1, int(1.0 * S)))
            d.ellipse(_bbox((c[0], brim_y + 1.0 * S), brim_w * 0.92, brim_h * 0.7), fill=pal["accent"], outline=None)
            # Crown.
            crown_top = brim_y - 7.0 * S
            d.rounded_rectangle(
                (c[0] - (spec.head_w * 0.40) * S, crown_top, c[0] + (spec.head_w * 0.40) * S, brim_y - 0.0 * S),
                radius=2.5 * S, fill=pal["accent_dark"], outline=outline, width=max(1, int(1.0 * S)),
            )
            d.rectangle(
                (c[0] - (spec.head_w * 0.42) * S, brim_y - 1.5 * S, c[0] + (spec.head_w * 0.42) * S, brim_y + 0.5 * S),
                fill=pal["accent"], outline=outline, width=max(1, int(0.7 * S)),
            )
        elif spec.hair_style == "tricorn_hat":
            # Walter — three-cornered hat with brass trim. A
            # triangle silhouette pointed forward + sideways.
            tri = [
                (c[0] - spec.head_w * 0.62 * S, c[1] - spec.head_h * 0.28 * S),
                (c[0] - spec.head_w * 0.08 * S, c[1] - spec.head_h * 0.62 * S),
                (c[0] + spec.head_w * 0.50 * S, c[1] - spec.head_h * 0.20 * S),
            ]
            d.polygon(tri, fill=pal["outfit"], outline=outline)
            # Brim band under the points.
            d.line(
                [(c[0] - spec.head_w * 0.62 * S, c[1] - spec.head_h * 0.28 * S),
                 (c[0] + spec.head_w * 0.50 * S, c[1] - spec.head_h * 0.20 * S)],
                fill=pal["accent"], width=max(1, int(1.4 * S)),
            )
            # Silver side fringe of hair visible below the hat.
            d.pieslice(
                _bbox((c[0] - 0.5 * S, c[1] - spec.head_h * 0.08 * S), (spec.head_w + 2.0) * S, (spec.head_h * 0.36) * S),
                start=180, end=360,
                fill=pal["hair"], outline=outline, width=max(1, int(0.9 * S)),
            )
        elif spec.hair_style in {"bob", "crest", "swoop", "cap", "general_hat", "officer_cap", "tousled_crop", "chignon", "undercut_braid"}:
            d.ellipse(_bbox((c[0] - 1.0 * S, c[1] - 4.0 * S), (spec.head_w + spec.hair_volume) * S, (spec.head_h * 0.78 + spec.hair_volume * 0.45) * S), fill=pal["hair"], outline=outline, width=max(1, int(1.1 * S)))
        # Face.
        d.ellipse(_bbox(c, spec.head_w * S, spec.head_h * S), fill=pal["skin"], outline=outline, width=max(1, int(1.2 * S)))
        # Chin/jaw shadow. Suppressed for `feminine_coded` archetypes
        # because the skin_shadow ellipse against the lighter face
        # reads as a beard/goatee at the runtime downsample. Hair
        # length is the main feminine cue; this avoids fighting it.
        if not spec.feminine_coded:
            d.ellipse(_bbox((c[0] + 1.0 * S, c[1] + spec.head_h * 0.18 * S), (spec.head_w * 0.70) * S, (spec.chin_h * 1.9) * S), fill=pal["skin_shadow"], outline=None)
        # Front hair / features.
        if spec.hair_style == "swoop":
            # Tousled front fringe broken into two clumps so it doesn't
            # read as a single hard-edged side-sweep. A short forehead
            # highlight keeps a sliver of bare skin visible between the
            # clumps even when the sprite is downsampled.
            left_clump = [
                (c[0] - spec.head_w * 0.46 * S, c[1] - spec.head_h * 0.42 * S),
                (c[0] - spec.head_w * 0.08 * S, c[1] - spec.head_h * 0.60 * S),
                (c[0] - spec.head_w * 0.02 * S, c[1] - spec.head_h * 0.18 * S),
                (c[0] - spec.head_w * 0.34 * S, c[1] - spec.head_h * 0.10 * S),
            ]
            right_clump = [
                (c[0] + spec.head_w * 0.06 * S, c[1] - spec.head_h * 0.56 * S),
                (c[0] + spec.head_w * 0.42 * S, c[1] - spec.head_h * 0.34 * S),
                (c[0] + spec.head_w * 0.34 * S, c[1] - spec.head_h * 0.04 * S),
                (c[0] + spec.head_w * 0.06 * S, c[1] - spec.head_h * 0.08 * S),
            ]
            d.polygon(left_clump, fill=pal["hair"], outline=outline)
            d.polygon(right_clump, fill=pal["hair"], outline=outline)
            d.line([
                (c[0] - spec.head_w * 0.06 * S, c[1] - spec.head_h * 0.30 * S),
                (c[0] + spec.head_w * 0.02 * S, c[1] - spec.head_h * 0.18 * S),
            ], fill=pal["skin"], width=max(1, int(1.0 * S)))
        elif spec.hair_style == "bob":
            # Short close-cropped professional cut. Earlier passes had
            # this as a chin-length bob with two curtain polygons that
            # hung past the jaw; the user read those as "two lobes over
            # his face," so we drop the side curtains entirely. The hair
            # now hugs the top of the skull, has a soft side-part forehead
            # sweep, and stops cleanly above the ear so the face silhouette
            # stays open. The back-of-head ellipse drawn above this
            # branch is still doing the bulk of the silhouette work.
            d.pieslice(
                _bbox(
                    (c[0] - 0.5 * S, c[1] - spec.head_h * 0.18 * S),
                    (spec.head_w * 0.94 + spec.hair_volume * 0.40) * S,
                    (spec.head_h * 0.46 + spec.hair_volume * 0.18) * S,
                ),
                start=200,
                end=345,
                fill=pal["hair"],
                outline=outline,
                width=max(1, int(1.0 * S)),
            )
            # Soft side-part forehead sweep — a single small wedge angled
            # toward the camera-right brow. Kept short (does not reach
            # past the brow line) so it can't be misread as a side sweep.
            d.polygon([
                (c[0] - spec.head_w * 0.18 * S, c[1] - spec.head_h * 0.28 * S),
                (c[0] + spec.head_w * 0.22 * S, c[1] - spec.head_h * 0.36 * S),
                (c[0] + spec.head_w * 0.10 * S, c[1] - spec.head_h * 0.18 * S),
                (c[0] - spec.head_w * 0.04 * S, c[1] - spec.head_h * 0.16 * S),
            ], fill=pal["hair"], outline=outline)
            # Visible side-part line in the lighter shine color so the
            # cut reads as styled hair rather than a helmet.
            d.line([
                (c[0] - spec.head_w * 0.04 * S, c[1] - spec.head_h * 0.46 * S),
                (c[0] + spec.head_w * 0.14 * S, c[1] - spec.head_h * 0.26 * S),
            ], fill=pal["hair_shine"], width=max(1, int(1.1 * S)))
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
        elif spec.hair_style == "tousled_crop":
            # Bob — short messy cut. A handful of small wedge tufts
            # sitting on top of the back-mass ellipse, plus two
            # forehead bangs that flop forward without covering the
            # eyes. Reads as "doesn't think about hair, but it
            # looks fine."
            for sign, dx in zip((-1, -1, 1, 1), (-7, -2, 3, 8)):
                tuft = [
                    (c[0] + dx * S, c[1] - spec.head_h * 0.48 * S),
                    (c[0] + (dx + 2.0) * S, c[1] - spec.head_h * 0.62 * S),
                    (c[0] + (dx + 4.0) * S, c[1] - spec.head_h * 0.40 * S),
                ]
                d.polygon(tuft, fill=pal["hair"], outline=outline)
            # Forehead bangs — two short triangles dipping toward the brow.
            for dx in (-3.0, 3.0):
                d.polygon([
                    (c[0] + dx * S, c[1] - spec.head_h * 0.42 * S),
                    (c[0] + (dx + 2.5) * S, c[1] - spec.head_h * 0.18 * S),
                    (c[0] + (dx + 5.0) * S, c[1] - spec.head_h * 0.34 * S),
                ], fill=pal["hair"], outline=outline)
        elif spec.hair_style == "chignon":
            # Alice — hair pulled back into a tidy bun on top with a
            # decorative stick poking out. Forehead stays clear so
            # the face reads sharp and focused.
            bun_c = (c[0] - 3.0 * S, c[1] - spec.head_h * 0.62 * S)
            d.ellipse(_bbox(bun_c, 6.5 * S, 5.0 * S), fill=pal["hair"], outline=outline, width=max(1, int(1.0 * S)))
            # Hair-stick: a thin diagonal line crossing the bun.
            stick_a = (bun_c[0] - 6.0 * S, bun_c[1] + 2.0 * S)
            stick_b = (bun_c[0] + 6.0 * S, bun_c[1] - 2.0 * S)
            d.line([stick_a, stick_b], fill=pal["accent"], width=max(1, int(1.4 * S)))
            d.ellipse(_bbox(stick_b, 1.4 * S, 1.4 * S), fill=pal["accent"], outline=outline, width=max(1, int(0.7 * S)))
            # Small wisp at the nape so the back of the head doesn't
            # read as a bald patch behind the bun.
            d.polygon([
                (c[0] - spec.head_w * 0.34 * S, c[1] + 2.0 * S),
                (c[0] - spec.head_w * 0.46 * S, c[1] - 4.0 * S),
                (c[0] - spec.head_w * 0.20 * S, c[1] - 6.0 * S),
            ], fill=pal["hair"], outline=outline)
        elif spec.hair_style == "undercut_braid":
            # Mallory — sides shaved short, long braid down the back
            # falling past the shoulder. The shave is implied by a
            # thin skin band exposed along the temple; the braid is
            # three stacked ovals tapering to a tip.
            # Temple shave band.
            for sign in (-1, 1):
                temple_top = (c[0] + sign * spec.head_w * 0.34 * S, c[1] - spec.head_h * 0.36 * S)
                temple_bot = (c[0] + sign * spec.head_w * 0.32 * S, c[1] - spec.head_h * 0.08 * S)
                d.line([temple_top, temple_bot], fill=pal["skin"], width=max(1, int(2.0 * S)))
            # Braid hanging down the back (camera-left of the head).
            braid_x = c[0] - spec.head_w * 0.46 * S
            for i, (dy, w) in enumerate(((-2.0, 4.4), (4.0, 4.0), (10.0, 3.5), (16.0, 2.8))):
                seg_c = (braid_x, c[1] + dy * S)
                d.ellipse(_bbox(seg_c, w * S, 3.2 * S), fill=pal["hair"], outline=outline, width=max(1, int(0.9 * S)))
                # Braid-weave shine: a small lighter dot near the top
                # of each segment.
                d.ellipse(_bbox((seg_c[0] - 0.6 * S, seg_c[1] - 1.0 * S), 1.4 * S, 0.9 * S), fill=pal["hair_shine"], outline=None)
            # Braid-tie ribbon at the bottom in the accent color.
            d.rounded_rectangle(
                (braid_x - 3.0 * S, c[1] + 18.0 * S, braid_x + 3.0 * S, c[1] + 21.0 * S),
                radius=1.4 * S, fill=pal["accent"], outline=outline, width=max(1, int(0.7 * S)),
            )
            # Forehead bang on the kept-long side (camera-right).
            d.polygon([
                (c[0] + 2.0 * S, c[1] - spec.head_h * 0.46 * S),
                (c[0] + 8.0 * S, c[1] - spec.head_h * 0.42 * S),
                (c[0] + 5.0 * S, c[1] - spec.head_h * 0.16 * S),
                (c[0] + 1.0 * S, c[1] - spec.head_h * 0.24 * S),
            ], fill=pal["hair"], outline=outline)
        elif spec.hair_style == "clean_bald":
            # Trent — front of the head is bare skin, but he gets a
            # subtle gray brow line plus a small white mustache so
            # the bald silhouette doesn't read as expressionless.
            # Brow line (subtle): a faint dark stroke above the eyes.
            d.line(
                [(c[0] - 3.0 * S, c[1] - spec.head_h * 0.06 * S), (c[0] + 9.0 * S, c[1] - spec.head_h * 0.10 * S)],
                fill=pal["hair_shine"],
                width=max(1, int(0.9 * S)),
            )
        elif spec.hair_style == "square_fringe":
            # Victor — precise blocky bangs across the forehead.
            # Flat-bottomed rectangle with sharp corners (vs the
            # rounded tousled / soft bangs of other characters).
            d.rectangle(
                (c[0] - spec.head_w * 0.40 * S, c[1] - spec.head_h * 0.50 * S,
                 c[0] + spec.head_w * 0.40 * S, c[1] - spec.head_h * 0.22 * S),
                fill=pal["hair"], outline=outline, width=max(1, int(0.9 * S)),
            )
        elif spec.hair_style == "ponytail":
            # Peggy — small forehead bang sweep on the camera-side.
            d.polygon([
                (c[0] - spec.head_w * 0.30 * S, c[1] - spec.head_h * 0.46 * S),
                (c[0] + spec.head_w * 0.30 * S, c[1] - spec.head_h * 0.40 * S),
                (c[0] + spec.head_w * 0.22 * S, c[1] - spec.head_h * 0.18 * S),
                (c[0] - spec.head_w * 0.18 * S, c[1] - spec.head_h * 0.22 * S),
            ], fill=pal["hair"], outline=outline)
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
            # 3/4 view: draw both eyes flanking the nose so the face
            # doesn't read as a cyclops. Matches the general_hat /
            # officer_cap layout the user already approved: a larger
            # near eye on the camera-left of the face and a smaller
            # far eye on the camera-right, both with pupils tracking
            # forward. Spacing is 7.0×S center-to-center, roughly the
            # same fraction of head width as the General sheet.
            eye_y = c[1] - 1.8 * S
            near_eye_x = c[0] + 1.0 * S
            far_eye_x = c[0] + 8.0 * S
            near_w, near_h = 3.6 * S, max(1.2 * S, (1.2 + pose.eye_squint * 4.0) * S)
            far_w, far_h = 3.0 * S, max(1.0 * S, (1.0 + pose.eye_squint * 3.4) * S)
            if pose.blink or pose.dead:
                d.line([(near_eye_x - 2.2 * S, eye_y), (near_eye_x + 2.0 * S, eye_y)], fill=outline, width=max(1, int(1.3 * S)))
                d.line([(far_eye_x - 1.6 * S, eye_y), (far_eye_x + 1.4 * S, eye_y)], fill=outline, width=max(1, int(1.1 * S)))
            else:
                pupil_y = eye_y + pose.eye_squint * 0.4 * S
                # Near eye (camera-left, larger).
                d.ellipse(_bbox((near_eye_x, eye_y), near_w, near_h), fill=pal["white"], outline=outline, width=max(1, int(1.0 * S)))
                d.ellipse(_bbox((near_eye_x + 0.55 * S, pupil_y), 1.3 * S, 2.4 * S), fill=outline)
                # Far eye (camera-right, smaller, slightly higher to
                # suggest the head tilt from the 3/4 angle).
                d.ellipse(_bbox((far_eye_x, eye_y - 0.1 * S), far_w, far_h), fill=pal["white"], outline=outline, width=max(1, int(0.9 * S)))
                d.ellipse(_bbox((far_eye_x + 0.45 * S, pupil_y - 0.05 * S), 1.05 * S, 2.0 * S), fill=outline)
                # Eyelash cue for `feminine_coded` archetypes. One short
                # outer-corner tick per eye — read as a hint of lash
                # at the runtime downsample without sliding into
                # "make-up trope" territory. Hair length is the
                # primary feminine cue; this is the subtle finish.
                if spec.feminine_coded:
                    # Single short stroke at the outer corner of the
                    # near eye only.
                    lash_root = (near_eye_x + 2.0 * S, eye_y - near_h)
                    lash_tip = (lash_root[0] + 0.4 * S, lash_root[1] - 1.0 * S)
                    d.line([lash_root, lash_tip], fill=outline, width=max(1, int(0.6 * S)))
                    # One even shorter stroke on the far eye outer corner.
                    lash_root = (far_eye_x + 2.0 * S, eye_y - far_h - 0.1 * S)
                    lash_tip = (lash_root[0] + 0.3 * S, lash_root[1] - 0.8 * S)
                    d.line([lash_root, lash_tip], fill=outline, width=max(1, int(0.5 * S)))
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
        elif spec.outfit == "banyan":
            # Loose 18th-century silk banyan / dressing gown (Handmann's
            # 1753 Euler portrait). A wide-shouldered draped robe that
            # falls open at the front to reveal a paler shirt underneath,
            # tied with a sash at the waist. We render: (1) the outer
            # robe silhouette, (2) the visible shirt panel down the
            # center, (3) the broad lapel/shawl collar, (4) a darker sash
            # at the waist, and (5) a sparse paisley-dot pattern in the
            # accent color so the silk reads as patterned rather than
            # a flat block.
            d = ImageDraw.Draw(base)
            shirt = pal.get("white", rgba("#FFF6E0"))
            sash = pal.get("accent_dark", pal["outfit_dark"])
            silk_dot = pal.get("accent", pal["outfit_dark"])
            # Outer robe (drapes wider than the torso, falls long).
            robe = [
                (center[0] - spec.shoulder_w * 0.74 * S, center[1] - spec.torso_h * 0.50 * S),
                (center[0] + spec.shoulder_w * 0.60 * S, center[1] - spec.torso_h * 0.44 * S),
                (center[0] + (spec.torso_w * 0.62 + 2.0) * S, center[1] + spec.torso_h * 0.10 * S),
                (center[0] + (spec.hip_w * 0.58 + 1.0) * S, center[1] + spec.torso_h * 0.48 * S + spec.coat_len * 0.55 * S),
                (center[0] - (spec.hip_w * 0.74 + 1.0) * S, center[1] + spec.torso_h * 0.46 * S + spec.coat_len * 0.50 * S),
                (center[0] - (spec.torso_w * 0.64 + 2.0) * S, center[1] + spec.torso_h * 0.08 * S),
            ]
            d.polygon(robe, fill=pal["outfit"], outline=outline)
            # Sparse paisley dots on the silk. Only a handful so they
            # read as pattern at the runtime downsample, not as
            # accidental dirt.
            for px, py in [
                (-9.0, -8.0), (4.0, -10.0), (-12.0, 2.0),
                (8.0, -1.0), (-6.0, 8.0), (10.0, 10.0),
                (-10.0, 14.0),
            ]:
                d.ellipse(
                    _bbox((center[0] + px * S, center[1] + py * S), 2.2 * S, 2.2 * S),
                    fill=silk_dot,
                    outline=None,
                )
            # Visible shirt panel running down the open front of the robe.
            shirt_panel = [
                (center[0] - 6.0 * S, center[1] - spec.torso_h * 0.40 * S),
                (center[0] + 6.0 * S, center[1] - spec.torso_h * 0.36 * S),
                (center[0] + 4.0 * S, center[1] + spec.torso_h * 0.48 * S),
                (center[0] - 4.0 * S, center[1] + spec.torso_h * 0.46 * S),
            ]
            d.polygon(shirt_panel, fill=shirt, outline=outline)
            # Cravat / ruffled neckline at the top of the shirt panel.
            d.polygon([
                (center[0] - 7.0 * S, center[1] - spec.torso_h * 0.42 * S),
                (center[0] + 7.5 * S, center[1] - spec.torso_h * 0.38 * S),
                (center[0] + 5.0 * S, center[1] - spec.torso_h * 0.26 * S),
                (center[0] - 4.5 * S, center[1] - spec.torso_h * 0.28 * S),
            ], fill=shirt, outline=outline)
            # Broad shawl-collar lapels of the banyan, framing the
            # shirt panel.
            lapel_left = [
                (center[0] - spec.shoulder_w * 0.62 * S, center[1] - spec.torso_h * 0.48 * S),
                (center[0] - 4.0 * S, center[1] - spec.torso_h * 0.38 * S),
                (center[0] - 2.0 * S, center[1] + spec.torso_h * 0.10 * S),
                (center[0] - spec.shoulder_w * 0.30 * S, center[1] + spec.torso_h * 0.20 * S),
            ]
            lapel_right = [
                (center[0] + spec.shoulder_w * 0.50 * S, center[1] - spec.torso_h * 0.42 * S),
                (center[0] + 5.0 * S, center[1] - spec.torso_h * 0.34 * S),
                (center[0] + 4.0 * S, center[1] + spec.torso_h * 0.18 * S),
                (center[0] + spec.shoulder_w * 0.26 * S, center[1] + spec.torso_h * 0.28 * S),
            ]
            d.polygon(lapel_left, fill=pal["outfit_dark"], outline=outline)
            d.polygon(lapel_right, fill=pal["outfit_dark"], outline=outline)
            # Waist sash tied to one side.
            sash_y = center[1] + spec.torso_h * 0.30 * S
            d.rounded_rectangle(
                (center[0] - spec.hip_w * 0.62 * S, sash_y - 3.5 * S, center[0] + spec.hip_w * 0.52 * S, sash_y + 3.5 * S),
                radius=2.0 * S,
                fill=sash,
                outline=outline,
                width=max(1, int(1.0 * S)),
            )
            # Sash tassel drop on camera-right.
            d.polygon([
                (center[0] + spec.hip_w * 0.50 * S, sash_y - 1.5 * S),
                (center[0] + spec.hip_w * 0.58 * S + 4.0 * S, sash_y + 0.0 * S),
                (center[0] + spec.hip_w * 0.46 * S + 1.0 * S, sash_y + 10.0 * S),
                (center[0] + spec.hip_w * 0.34 * S, sash_y + 2.0 * S),
            ], fill=sash, outline=outline)
        elif spec.outfit == "vest_over_shirt":
            # Bob — high-vis workshop vest layered over a tee. Two
            # diagonal reflective stripes across the chest in the
            # accent color make the silhouette pop and signal
            # "practical engineer" instantly.
            d = ImageDraw.Draw(base)
            # Tee underneath (visible at sleeves and collar).
            tee = [
                (center[0] - spec.shoulder_w * 0.50 * S, center[1] - spec.torso_h * 0.46 * S),
                (center[0] + spec.shoulder_w * 0.36 * S, center[1] - spec.torso_h * 0.42 * S),
                (center[0] + spec.torso_w * 0.50 * S, center[1] + spec.torso_h * 0.06 * S),
                (center[0] + spec.hip_w * 0.34 * S, center[1] + spec.torso_h * 0.50 * S),
                (center[0] - spec.hip_w * 0.38 * S, center[1] + spec.torso_h * 0.50 * S),
            ]
            d.polygon(tee, fill=pal.get("white", rgba("#EEE6D2")), outline=outline)
            # Vest on top, open at the front.
            vest_left = [
                (center[0] - spec.shoulder_w * 0.48 * S, center[1] - spec.torso_h * 0.42 * S),
                (center[0] - 3.0 * S, center[1] - spec.torso_h * 0.40 * S),
                (center[0] - 4.0 * S, center[1] + spec.torso_h * 0.48 * S),
                (center[0] - spec.hip_w * 0.36 * S, center[1] + spec.torso_h * 0.48 * S),
            ]
            vest_right = [
                (center[0] + 2.0 * S, center[1] - spec.torso_h * 0.40 * S),
                (center[0] + spec.shoulder_w * 0.34 * S, center[1] - spec.torso_h * 0.42 * S),
                (center[0] + spec.hip_w * 0.32 * S, center[1] + spec.torso_h * 0.48 * S),
                (center[0] + 2.0 * S, center[1] + spec.torso_h * 0.48 * S),
            ]
            d.polygon(vest_left, fill=pal["outfit"], outline=outline)
            d.polygon(vest_right, fill=pal["outfit"], outline=outline)
            # Reflective accent stripes across both vest panels.
            for sign in (-1, 1):
                stripe_y = center[1] + 2.0 * S
                d.rectangle(
                    (center[0] + sign * 11.0 * S - 5.0 * S, stripe_y, center[0] + sign * 11.0 * S + 5.0 * S, stripe_y + 2.4 * S),
                    fill=pal["accent"],
                    outline=outline,
                    width=max(1, int(0.7 * S)),
                )
            # Chest patch pocket in the darker outfit color on the
            # camera-right panel.
            d.rounded_rectangle(
                (center[0] + 4.0 * S, center[1] - 6.0 * S, center[0] + 10.5 * S, center[1] + 0.5 * S),
                radius=1.4 * S,
                fill=pal["outfit_dark"],
                outline=outline,
                width=max(1, int(0.7 * S)),
            )
        elif spec.outfit == "cinched_tabard":
            # Alice — variant of `tabard` with a clearly cinched
            # waist and a flared hip hem so the silhouette reads
            # feminine without leaning on body-type stereotype. Same
            # one-time-pad checker survives so the cipher signature
            # stays.
            d = ImageDraw.Draw(base)
            # Under-jacket — slim through the waist.
            waist_y_top = center[1] + spec.torso_h * 0.06 * S
            waist_y_bot = center[1] + spec.torso_h * 0.20 * S
            jacket = [
                (center[0] - spec.shoulder_w * 0.48 * S, center[1] - spec.torso_h * 0.46 * S),
                (center[0] + spec.shoulder_w * 0.34 * S, center[1] - spec.torso_h * 0.42 * S),
                (center[0] + spec.torso_w * 0.46 * S, waist_y_top),
                # Pinch at the waist.
                (center[0] + spec.torso_w * 0.34 * S, waist_y_bot),
                (center[0] + spec.hip_w * 0.42 * S, center[1] + spec.torso_h * 0.42 * S),
                # Flared hem below the waist.
                (center[0] + spec.hip_w * 0.56 * S, center[1] + spec.torso_h * 0.56 * S),
                (center[0] - spec.hip_w * 0.62 * S, center[1] + spec.torso_h * 0.56 * S),
                (center[0] - spec.hip_w * 0.46 * S, center[1] + spec.torso_h * 0.42 * S),
                (center[0] - spec.torso_w * 0.38 * S, waist_y_bot),
                (center[0] - spec.torso_w * 0.46 * S, waist_y_top),
            ]
            d.polygon(jacket, fill=pal["outfit_dark"], outline=outline)
            # Front tabard panel — slightly tapered at the waist,
            # widens again at the hem.
            tabard = [
                (center[0] - 7.0 * S, center[1] - spec.torso_h * 0.40 * S),
                (center[0] + 7.0 * S, center[1] - spec.torso_h * 0.36 * S),
                (center[0] + 5.5 * S, waist_y_bot),
                (center[0] + 7.5 * S, center[1] + spec.torso_h * 0.60 * S),
                (center[0] - 7.5 * S, center[1] + spec.torso_h * 0.60 * S),
                (center[0] - 5.5 * S, waist_y_bot),
            ]
            d.polygon(tabard, fill=pal["outfit"], outline=outline)
            # OTP checker pattern (same as tabard, condensed slightly
            # because the panel is narrower at the waist).
            cols, rows = 4, 6
            cell_w = 13.0 * S / cols
            cell_h = (spec.torso_h * 0.90 * S) / rows
            x0 = center[0] - 6.5 * S
            y0 = center[1] - spec.torso_h * 0.34 * S
            for r in range(rows):
                for cc in range(cols):
                    if (r + cc) % 2 == 0:
                        continue
                    cx = x0 + cc * cell_w
                    cy = y0 + r * cell_h
                    d.rectangle(
                        (cx, cy, cx + cell_w - 0.6 * S, cy + cell_h - 0.6 * S),
                        fill=pal["accent_dark"],
                        outline=None,
                    )
            # Visible waist sash (wider + brighter than the plain
            # tabard's belt) — the key feminine silhouette cue.
            d.rounded_rectangle(
                (center[0] - 9.0 * S, waist_y_top, center[0] + 9.0 * S, waist_y_bot),
                radius=2.0 * S,
                fill=pal["accent"],
                outline=outline,
                width=max(1, int(0.9 * S)),
            )
            # Sash tie + tail on camera-right.
            d.polygon([
                (center[0] + 8.0 * S, waist_y_top),
                (center[0] + 13.0 * S, waist_y_top + 1.0 * S),
                (center[0] + 11.0 * S, waist_y_bot + 6.0 * S),
                (center[0] + 7.0 * S, waist_y_bot),
            ], fill=pal["accent"], outline=outline)
        elif spec.outfit == "cinched_field_jacket":
            # Mallory — field_jacket with a visible cinched belt at
            # the waist and slightly tapered torso. The chest strap
            # + zip + pockets all carry over from the original; the
            # belt is the silhouette cue.
            d = ImageDraw.Draw(base)
            waist_y_top = center[1] + spec.torso_h * 0.10 * S
            waist_y_bot = center[1] + spec.torso_h * 0.22 * S
            jacket = [
                (center[0] - spec.shoulder_w * 0.50 * S, center[1] - spec.torso_h * 0.46 * S),
                (center[0] + spec.shoulder_w * 0.38 * S, center[1] - spec.torso_h * 0.42 * S),
                (center[0] + spec.torso_w * 0.50 * S, waist_y_top),
                (center[0] + spec.torso_w * 0.36 * S, waist_y_bot),
                (center[0] + spec.hip_w * 0.36 * S, center[1] + spec.torso_h * 0.52 * S),
                (center[0] - spec.hip_w * 0.42 * S, center[1] + spec.torso_h * 0.52 * S),
                (center[0] - spec.torso_w * 0.40 * S, waist_y_bot),
                (center[0] - spec.torso_w * 0.52 * S, waist_y_top),
            ]
            d.polygon(jacket, fill=pal["outfit"], outline=outline)
            # Central zip.
            d.line(
                [(center[0] - 1.0 * S, center[1] - spec.torso_h * 0.42 * S), (center[0] - 2.0 * S, waist_y_top - 1.0 * S)],
                fill=pal["white"],
                width=max(1, int(0.9 * S)),
            )
            d.rectangle(
                (center[0] - 2.4 * S, center[1] - spec.torso_h * 0.18 * S, center[0] + 0.4 * S, center[1] - spec.torso_h * 0.10 * S),
                fill=pal["white"],
                outline=outline,
                width=max(1, int(0.6 * S)),
            )
            # Two square chest pockets in the darker outfit color.
            for sign in (-1, 1):
                px = center[0] + sign * 7.0 * S
                d.rectangle(
                    (px - 3.5 * S, center[1] - 6.0 * S, px + 3.5 * S, center[1] + 1.0 * S),
                    fill=pal["outfit_dark"],
                    outline=outline,
                    width=max(1, int(0.7 * S)),
                )
                d.line(
                    [(px - 2.6 * S, center[1] - 5.0 * S), (px + 2.6 * S, center[1] - 5.0 * S)],
                    fill=pal["white"],
                    width=max(1, int(0.6 * S)),
                )
            # Diagonal chest strap in the accent color (oxblood).
            d.polygon([
                (center[0] - spec.shoulder_w * 0.46 * S, center[1] - spec.torso_h * 0.34 * S),
                (center[0] + spec.shoulder_w * 0.10 * S, center[1] - spec.torso_h * 0.20 * S),
                (center[0] + spec.shoulder_w * 0.06 * S, center[1] - spec.torso_h * 0.10 * S),
                (center[0] - spec.shoulder_w * 0.48 * S, center[1] - spec.torso_h * 0.24 * S),
            ], fill=pal["accent"], outline=outline)
            # The waist belt — wide accent_dark strap with a chrome
            # buckle on center.
            d.rounded_rectangle(
                (center[0] - spec.torso_w * 0.46 * S, waist_y_top, center[0] + spec.torso_w * 0.42 * S, waist_y_bot),
                radius=1.6 * S,
                fill=pal["accent_dark"],
                outline=outline,
                width=max(1, int(0.9 * S)),
            )
            d.rectangle(
                (center[0] - 2.4 * S, waist_y_top + 0.2 * S, center[0] + 2.4 * S, waist_y_bot - 0.2 * S),
                fill=pal["white"],
                outline=outline,
                width=max(1, int(0.7 * S)),
            )
        elif spec.outfit == "tabard":
            # Alice — a fitted under-jacket plus a long front tabard
            # patterned like a one-time pad. The checker pattern is
            # the locking visual: 4×8 grid of small squares in white
            # + accent_dark covers the chest panel down to the hem.
            d = ImageDraw.Draw(base)
            # Jacket underneath (close-fitting).
            jacket = [
                (center[0] - spec.shoulder_w * 0.48 * S, center[1] - spec.torso_h * 0.46 * S),
                (center[0] + spec.shoulder_w * 0.34 * S, center[1] - spec.torso_h * 0.42 * S),
                (center[0] + spec.torso_w * 0.46 * S, center[1] + spec.torso_h * 0.06 * S),
                (center[0] + spec.hip_w * 0.30 * S, center[1] + spec.torso_h * 0.52 * S),
                (center[0] - spec.hip_w * 0.36 * S, center[1] + spec.torso_h * 0.52 * S),
            ]
            d.polygon(jacket, fill=pal["outfit_dark"], outline=outline)
            # Long tabard front panel in the lighter outfit color.
            tabard = [
                (center[0] - 7.5 * S, center[1] - spec.torso_h * 0.40 * S),
                (center[0] + 7.5 * S, center[1] - spec.torso_h * 0.36 * S),
                (center[0] + 6.5 * S, center[1] + spec.torso_h * 0.60 * S),
                (center[0] - 6.5 * S, center[1] + spec.torso_h * 0.60 * S),
            ]
            d.polygon(tabard, fill=pal["outfit"], outline=outline)
            # One-time-pad checker pattern — 4 columns × 6 rows,
            # alternating accent (cream) and accent_dark.
            cols, rows = 4, 6
            cell_w = 14.0 * S / cols
            cell_h = (spec.torso_h * 0.90 * S) / rows
            x0 = center[0] - 7.0 * S
            y0 = center[1] - spec.torso_h * 0.34 * S
            for r in range(rows):
                for cc in range(cols):
                    if (r + cc) % 2 == 0:
                        continue
                    cx = x0 + cc * cell_w
                    cy = y0 + r * cell_h
                    d.rectangle(
                        (cx, cy, cx + cell_w - 0.6 * S, cy + cell_h - 0.6 * S),
                        fill=pal["accent_dark"],
                        outline=None,
                    )
            # Belt at the waist holding the tabard down.
            d.rounded_rectangle(
                (center[0] - 8.0 * S, center[1] + spec.torso_h * 0.20 * S, center[0] + 8.0 * S, center[1] + spec.torso_h * 0.30 * S),
                radius=1.6 * S,
                fill=pal["outfit_dark"],
                outline=outline,
                width=max(1, int(0.8 * S)),
            )
            d.rectangle(
                (center[0] - 1.5 * S, center[1] + spec.torso_h * 0.20 * S, center[0] + 1.5 * S, center[1] + spec.torso_h * 0.30 * S),
                fill=pal["accent"],
                outline=outline,
                width=max(1, int(0.6 * S)),
            )
        elif spec.outfit == "eavesdrop_cloak":
            # Eve — a long straight-fall cloak that drapes to ankles
            # with a side-clasp at the throat. Quieter than the
            # poncho's diagonal shawl; reads as someone trying not
            # to be noticed.
            d = ImageDraw.Draw(base)
            cloak = [
                (center[0] - spec.shoulder_w * 0.55 * S, center[1] - spec.torso_h * 0.50 * S),
                (center[0] + spec.shoulder_w * 0.42 * S, center[1] - spec.torso_h * 0.44 * S),
                (center[0] + spec.hip_w * 0.50 * S, center[1] + spec.torso_h * 0.52 * S + spec.coat_len * 0.50 * S),
                (center[0] - spec.hip_w * 0.62 * S, center[1] + spec.torso_h * 0.50 * S + spec.coat_len * 0.55 * S),
            ]
            d.polygon(cloak, fill=pal["outfit"], outline=outline)
            # Vertical seam down the center to give the long drop
            # some structure at the runtime downsample.
            d.line(
                [
                    (center[0] - 1.0 * S, center[1] - spec.torso_h * 0.46 * S),
                    (center[0] - 2.0 * S, center[1] + spec.torso_h * 0.52 * S + spec.coat_len * 0.40 * S),
                ],
                fill=pal["outfit_dark"],
                width=max(1, int(1.0 * S)),
            )
            # Throat clasp — a small round brooch in the accent color.
            d.ellipse(
                _bbox((center[0] - 4.0 * S, center[1] - spec.torso_h * 0.46 * S), 2.6 * S, 2.6 * S),
                fill=pal["accent"],
                outline=outline,
                width=max(1, int(0.7 * S)),
            )
            # A subtle vertical pleat highlight on the camera-right
            # half so the cloak doesn't read as a single flat block.
            d.line(
                [
                    (center[0] + 6.0 * S, center[1] - spec.torso_h * 0.36 * S),
                    (center[0] + 10.0 * S, center[1] + spec.torso_h * 0.46 * S + spec.coat_len * 0.30 * S),
                ],
                fill=_scale_color(pal["outfit"], 1.18),
                width=max(1, int(0.8 * S)),
            )
        elif spec.outfit == "field_jacket":
            # Mallory — close-fitting tactical jacket with chest
            # straps and a zip down the middle. Multiple visible
            # rectangular pockets read as "kit ready to go" without
            # any military-cap accessories.
            d = ImageDraw.Draw(base)
            jacket = [
                (center[0] - spec.shoulder_w * 0.48 * S, center[1] - spec.torso_h * 0.46 * S),
                (center[0] + spec.shoulder_w * 0.36 * S, center[1] - spec.torso_h * 0.42 * S),
                (center[0] + spec.torso_w * 0.50 * S, center[1] + spec.torso_h * 0.06 * S),
                (center[0] + spec.hip_w * 0.34 * S, center[1] + spec.torso_h * 0.52 * S),
                (center[0] - spec.hip_w * 0.40 * S, center[1] + spec.torso_h * 0.52 * S),
            ]
            d.polygon(jacket, fill=pal["outfit"], outline=outline)
            # Central zip in chrome (white) with a tab.
            d.line(
                [(center[0] - 1.0 * S, center[1] - spec.torso_h * 0.42 * S), (center[0] - 2.0 * S, center[1] + spec.torso_h * 0.46 * S)],
                fill=pal["white"],
                width=max(1, int(0.9 * S)),
            )
            d.rectangle(
                (center[0] - 2.4 * S, center[1] - spec.torso_h * 0.18 * S, center[0] + 0.4 * S, center[1] - spec.torso_h * 0.10 * S),
                fill=pal["white"],
                outline=outline,
                width=max(1, int(0.6 * S)),
            )
            # Two square chest pockets in the darker outfit color.
            for sign in (-1, 1):
                px = center[0] + sign * 8.0 * S
                d.rectangle(
                    (px - 4.0 * S, center[1] - 7.0 * S, px + 4.0 * S, center[1] + 1.0 * S),
                    fill=pal["outfit_dark"],
                    outline=outline,
                    width=max(1, int(0.7 * S)),
                )
                d.line(
                    [(px - 3.0 * S, center[1] - 6.0 * S), (px + 3.0 * S, center[1] - 6.0 * S)],
                    fill=pal["white"],
                    width=max(1, int(0.6 * S)),
                )
            # Diagonal chest strap in the accent color (oxblood).
            d.polygon([
                (center[0] - spec.shoulder_w * 0.46 * S, center[1] - spec.torso_h * 0.34 * S),
                (center[0] + spec.shoulder_w * 0.10 * S, center[1] - spec.torso_h * 0.20 * S),
                (center[0] + spec.shoulder_w * 0.06 * S, center[1] - spec.torso_h * 0.10 * S),
                (center[0] - spec.shoulder_w * 0.48 * S, center[1] - spec.torso_h * 0.24 * S),
            ], fill=pal["accent"], outline=outline)
        elif spec.outfit == "formal_robe":
            # Trent — long secular council robe with a square placket
            # down the front (lighter than keeper_robe's narrow
            # accent collar). Reads as "civic, not religious."
            d = ImageDraw.Draw(base)
            robe = [
                (center[0] - spec.shoulder_w * 0.66 * S, center[1] - spec.torso_h * 0.50 * S),
                (center[0] + spec.shoulder_w * 0.56 * S, center[1] - spec.torso_h * 0.44 * S),
                (center[0] + spec.hip_w * 0.50 * S, center[1] + spec.torso_h * 0.50 * S + spec.coat_len * 0.42 * S),
                (center[0] - spec.hip_w * 0.62 * S, center[1] + spec.torso_h * 0.50 * S + spec.coat_len * 0.36 * S),
            ]
            d.polygon(robe, fill=pal["outfit"], outline=outline)
            # Wide square placket front (lighter), running the full
            # length so the silhouette reads vertical and dignified.
            placket = [
                (center[0] - 6.0 * S, center[1] - spec.torso_h * 0.42 * S),
                (center[0] + 6.0 * S, center[1] - spec.torso_h * 0.40 * S),
                (center[0] + 5.0 * S, center[1] + spec.torso_h * 0.48 * S + spec.coat_len * 0.30 * S),
                (center[0] - 5.0 * S, center[1] + spec.torso_h * 0.48 * S + spec.coat_len * 0.30 * S),
            ]
            d.polygon(placket, fill=pal["accent"], outline=outline)
            # Three brass buttons down the placket.
            for i, ty in enumerate((-spec.torso_h * 0.20, -spec.torso_h * 0.02, spec.torso_h * 0.16)):
                d.ellipse(
                    _bbox((center[0] - 0.5 * S, center[1] + ty * S), 1.8 * S, 1.8 * S),
                    fill=pal["accent_dark"],
                    outline=outline,
                    width=max(1, int(0.6 * S)),
                )
            # Wide upturned collar in the darker outfit color.
            d.polygon([
                (center[0] - 9.0 * S, center[1] - spec.torso_h * 0.46 * S),
                (center[0] + 8.0 * S, center[1] - spec.torso_h * 0.42 * S),
                (center[0] + 6.0 * S, center[1] - spec.torso_h * 0.28 * S),
                (center[0] - 7.0 * S, center[1] - spec.torso_h * 0.32 * S),
            ], fill=pal["outfit_dark"], outline=outline)
        elif spec.outfit == "judicial_robe":
            # Judy — black judicial robe with crimson front placket
            # and white cuffs. Same broad silhouette as formal_robe
            # but the crimson and the wider sleeves separate Judy
            # from Trent at a glance.
            d = ImageDraw.Draw(base)
            robe = [
                (center[0] - spec.shoulder_w * 0.68 * S, center[1] - spec.torso_h * 0.50 * S),
                (center[0] + spec.shoulder_w * 0.58 * S, center[1] - spec.torso_h * 0.44 * S),
                (center[0] + spec.hip_w * 0.52 * S, center[1] + spec.torso_h * 0.50 * S + spec.coat_len * 0.44 * S),
                (center[0] - spec.hip_w * 0.66 * S, center[1] + spec.torso_h * 0.50 * S + spec.coat_len * 0.40 * S),
            ]
            d.polygon(robe, fill=pal["outfit"], outline=outline)
            # Narrow crimson placket strip down the front.
            d.polygon([
                (center[0] - 2.5 * S, center[1] - spec.torso_h * 0.42 * S),
                (center[0] + 2.5 * S, center[1] - spec.torso_h * 0.40 * S),
                (center[0] + 2.0 * S, center[1] + spec.torso_h * 0.46 * S + spec.coat_len * 0.30 * S),
                (center[0] - 2.0 * S, center[1] + spec.torso_h * 0.46 * S + spec.coat_len * 0.30 * S),
            ], fill=pal["accent"], outline=outline)
            # White starched cuff strip along the shoulders.
            d.rounded_rectangle(
                (center[0] - spec.shoulder_w * 0.60 * S, center[1] - spec.torso_h * 0.50 * S, center[0] - spec.shoulder_w * 0.26 * S, center[1] - spec.torso_h * 0.36 * S),
                radius=1.4 * S,
                fill=pal["white"],
                outline=outline,
                width=max(1, int(0.7 * S)),
            )
            # Subtle vertical fold lines so the robe drapes.
            for fx in (-7.0, 7.0):
                d.line(
                    [
                        (center[0] + fx * S, center[1] - spec.torso_h * 0.30 * S),
                        (center[0] + fx * S, center[1] + spec.torso_h * 0.46 * S + spec.coat_len * 0.25 * S),
                    ],
                    fill=pal["outfit_dark"],
                    width=max(1, int(0.7 * S)),
                )
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
        elif spec.accessory == "jabot_collar":
            # Judy — starched white frilled collar (jabot) hanging
            # from the throat. Three stacked layers of ruffled fabric
            # drop ~12 design-pixels down the front of the robe.
            white = pal.get("white", rgba("#F8F2E2"))
            base_y = center[1] - spec.torso_h * 0.42 * S
            for tier, (w, dy) in enumerate(((6.0, 0.0), (5.2, 4.5), (4.2, 8.5))):
                tier_w = w * S
                tier_y = base_y + dy * S
                # Scalloped tier edge using three small overlapping
                # ellipses so the ruffle reads at the runtime scale.
                for sign, dx in zip((-1, 0, 1), (-tier_w, 0.0, tier_w)):
                    d.ellipse(
                        _bbox((center[0] + dx * 0.5, tier_y + 1.0 * S), 3.0 * S, 2.0 * S),
                        fill=white,
                        outline=outline,
                        width=max(1, int(0.7 * S)),
                    )
                d.rectangle(
                    (center[0] - tier_w, tier_y - 1.0 * S, center[0] + tier_w, tier_y + 1.0 * S),
                    fill=white,
                    outline=outline,
                    width=max(1, int(0.7 * S)),
                )

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
        elif prop == "key_ring":
            # Bob — a carabiner ring with three pendant keys hanging
            # below it. Reads as "key custodian" at a glance.
            d = ImageDraw.Draw(base)
            ring_c = add(hand, vec(6.0 * S, angle - 20.0))
            d.ellipse(_bbox(ring_c, 4.0 * S, 4.0 * S), outline=outline, width=max(1, int(1.4 * S)))
            # Three keys hanging from the bottom of the ring.
            for i, ddx in enumerate((-3.0, 0.0, 3.0)):
                key_top = (ring_c[0] + ddx * S, ring_c[1] + 4.0 * S)
                key_tip = (ring_c[0] + ddx * S, ring_c[1] + 12.0 * S)
                d.line([key_top, key_tip], fill=pal["accent"], width=max(1, int(1.4 * S)))
                # Two short teeth on the lower half of each key.
                for ty in (8.0, 11.0):
                    d.line(
                        [(ring_c[0] + ddx * S, ring_c[1] + ty * S), (ring_c[0] + (ddx + 1.4) * S, ring_c[1] + ty * S)],
                        fill=pal["accent_dark"],
                        width=max(1, int(1.0 * S)),
                    )
                # Bow (head) of the key.
                d.ellipse(
                    _bbox((ring_c[0] + ddx * S, ring_c[1] + 4.8 * S), 1.2 * S, 1.4 * S),
                    fill=pal["accent_dark"],
                    outline=outline,
                    width=max(1, int(0.5 * S)),
                )
        elif prop == "cipher_scroll":
            # Alice — a tightly-rolled scroll about half the size of
            # a tablet, with a ribbon tied around it and a sliver of
            # cipher text visible on the exposed edge.
            d = ImageDraw.Draw(base)
            scroll_c = add(hand, vec(6.0 * S, angle - 12.0))
            # Body of the scroll.
            draw_rotated_rounded_rect(base, scroll_c, (12.0 * S, 4.0 * S), angle - 14.0, 1.6 * S, pal["white"], outline, 0.9 * S)
            # End caps darker so the rolled-up shape reads.
            for sign in (-1, 1):
                end_c = add(scroll_c, vec(sign * 6.0 * S, angle - 14.0))
                d.ellipse(
                    _bbox(end_c, 1.6 * S, 2.4 * S),
                    fill=pal["accent_dark"],
                    outline=outline,
                    width=max(1, int(0.6 * S)),
                )
            # Ribbon tied around the middle.
            ribbon_a = add(scroll_c, vec(2.0 * S, angle + 80.0))
            ribbon_b = add(scroll_c, vec(2.0 * S, angle - 100.0))
            d.line([ribbon_a, ribbon_b], fill=pal["outfit"], width=max(1, int(1.4 * S)))
            # Small ciphertext sliver — three short ticks.
            for tx in (-3.0, 0.0, 3.0):
                tick_top = add(scroll_c, vec(tx * S, angle - 14.0))
                tick_top = (tick_top[0], tick_top[1] - 1.0 * S)
                d.line([tick_top, (tick_top[0], tick_top[1] + 2.2 * S)], fill=pal["outfit_dark"], width=max(1, int(0.5 * S)))
        elif prop == "listening_horn":
            # Eve — a brass ear-trumpet (Victorian listening horn).
            # Held near the body in the hand; orientation suggests
            # she's cupping it forward to overhear something. The
            # cone widens away from the grip so the silhouette reads
            # as a horn even at small scale.
            d = ImageDraw.Draw(base)
            grip = hand
            mid = add(grip, vec(9.0 * S, angle - 25.0))
            wide = add(grip, vec(15.0 * S, angle - 25.0))
            # Cone polygon — wide end farther from the body.
            cone = [
                grip,
                add(grip, vec(2.0 * S, angle + 90.0)),
                add(mid, vec(4.0 * S, angle - 115.0)),
                add(wide, vec(7.0 * S, angle - 90.0)),
                add(wide, vec(7.0 * S, angle + 90.0)),
                add(mid, vec(4.0 * S, angle + 65.0)),
                add(grip, vec(2.0 * S, angle - 90.0)),
            ]
            d.polygon(cone, fill=pal["accent"], outline=outline)
            # Brass ring at the wide rim.
            d.ellipse(_bbox(wide, 4.0 * S, 5.5 * S), outline=outline, width=max(1, int(1.2 * S)))
            d.ellipse(_bbox(wide, 2.6 * S, 4.0 * S), fill=pal["accent_dark"], outline=None)
            # Highlight along the upper cone edge so the brass reads.
            d.line(
                [add(grip, vec(2.0 * S, angle + 90.0)), add(wide, vec(7.0 * S, angle + 90.0))],
                fill=_scale_color(pal["accent"], 1.25),
                width=max(1, int(0.9 * S)),
            )
        elif prop == "balance_scales":
            # Trent — a small handheld balance: a central pillar
            # rising from the fist, two side arms, two shallow pans
            # at the arm tips. Universal "fair arbitration" read.
            d = ImageDraw.Draw(base)
            pillar_base = hand
            pillar_top = add(hand, vec(14.0 * S, angle - 75.0))
            d.line([pillar_base, pillar_top], fill=pal["accent_dark"], width=max(1, int(1.6 * S)))
            # Crossbeam.
            beam_a = add(pillar_top, vec(7.0 * S, angle + 0.0))
            beam_b = add(pillar_top, vec(7.0 * S, angle + 180.0))
            d.line([beam_a, beam_b], fill=pal["accent"], width=max(1, int(1.6 * S)))
            # Pans (shallow arcs hanging from the beam tips).
            for pan_c in (beam_a, beam_b):
                # Chain from beam to pan.
                pan_anchor = (pan_c[0], pan_c[1] + 3.0 * S)
                d.line([pan_c, pan_anchor], fill=pal["accent_dark"], width=max(1, int(0.7 * S)))
                # Pan: a flat arc.
                d.arc(
                    (pan_c[0] - 3.4 * S, pan_anchor[1] - 0.5 * S, pan_c[0] + 3.4 * S, pan_anchor[1] + 4.5 * S),
                    start=0,
                    end=180,
                    fill=outline,
                    width=max(1, int(1.0 * S)),
                )
                d.line(
                    [(pan_c[0] - 3.4 * S, pan_anchor[1] + 0.5 * S), (pan_c[0] + 3.4 * S, pan_anchor[1] + 0.5 * S)],
                    fill=pal["accent_dark"],
                    width=max(1, int(0.8 * S)),
                )
            # Apex finial on the pillar.
            d.ellipse(_bbox(pillar_top, 1.8 * S, 1.8 * S), fill=pal["accent"], outline=outline, width=max(1, int(0.6 * S)))
        elif prop == "gavel":
            # Judy — judicial gavel: a cylindrical wooden head with a
            # darker handle. Held with the head out so the silhouette
            # reads "gavel" even at small render sizes.
            d = ImageDraw.Draw(base)
            handle_a = hand
            handle_b = add(hand, vec(11.0 * S, angle - 8.0))
            head_c = add(handle_b, vec(3.0 * S, angle - 8.0))
            # Handle.
            d.line([handle_a, handle_b], fill=pal["outfit_dark"], width=max(1, int(2.8 * S)))
            d.line([handle_a, handle_b], fill=pal["accent_dark"], width=max(1, int(1.4 * S)))
            # Cylindrical head — rotated rectangle perpendicular to
            # the handle so the gavel reads as a perpendicular cap.
            draw_rotated_rounded_rect(base, head_c, (6.0 * S, 11.0 * S), angle - 8.0, 2.0 * S, pal["accent_dark"], outline, 1.0 * S)
            # Wood-grain accent stripe across the head.
            d.line(
                [add(head_c, vec(5.0 * S, angle + 82.0)), add(head_c, vec(5.0 * S, angle - 98.0))],
                fill=_scale_color(pal["accent_dark"], 1.30),
                width=max(1, int(0.9 * S)),
            )
        elif prop == "lockpick":
            # Trudy — a slim L-shaped lockpick + tension wrench.
            d = ImageDraw.Draw(base)
            tip = add(hand, vec(14.0 * S, angle - 30.0))
            d.line([hand, tip], fill=pal["accent_dark"], width=max(1, int(1.4 * S)))
            d.line([hand, tip], fill=_scale_color(pal["accent"], 1.05), width=max(1, int(0.7 * S)))
            # L-bend at the tip.
            tip_l = add(tip, vec(3.5 * S, angle - 100.0))
            d.line([tip, tip_l], fill=pal["accent_dark"], width=max(1, int(1.4 * S)))
            # Tension wrench (smaller, perpendicular).
            wrench_a = add(hand, vec(8.0 * S, angle - 70.0))
            wrench_b = add(wrench_a, vec(6.0 * S, angle - 70.0))
            d.line([wrench_a, wrench_b], fill=pal["accent_dark"], width=max(1, int(1.2 * S)))
        elif prop == "stethoscope":
            # Craig — safe-cracker's stethoscope. Drum at the end +
            # tubing curving back to the ear pieces.
            d = ImageDraw.Draw(base)
            drum_c = add(hand, vec(10.0 * S, angle - 20.0))
            d.ellipse(_bbox(drum_c, 4.6 * S, 4.6 * S), fill=pal["accent"], outline=outline, width=max(1, int(1.0 * S)))
            d.ellipse(_bbox(drum_c, 2.8 * S, 2.8 * S), fill=pal["accent_dark"], outline=None)
            # Tubing.
            mid = add(hand, vec(4.0 * S, angle - 70.0))
            d.line([drum_c, mid, hand], fill=pal["outfit_dark"], width=max(1, int(1.4 * S)))
        elif prop == "mask_stack":
            # Sybil — a stack of three small masks held by the
            # strings. Each mask in a different palette tone so the
            # "many identities" signal reads.
            d = ImageDraw.Draw(base)
            colors = [pal["accent"], pal["outfit"], pal["accent_dark"]]
            for i, fill in enumerate(colors):
                mask_c = add(hand, vec((6.0 + i * 1.4) * S, angle - 18.0 - i * 4.0))
                d.ellipse(_bbox(mask_c, 4.0 * S, 3.0 * S), fill=fill, outline=outline, width=max(1, int(0.7 * S)))
                # Eye holes (two tiny dots).
                d.ellipse(_bbox((mask_c[0] - 1.2 * S, mask_c[1] - 0.4 * S), 0.6 * S, 0.6 * S), fill=outline)
                d.ellipse(_bbox((mask_c[0] + 1.2 * S, mask_c[1] - 0.4 * S), 0.6 * S, 0.6 * S), fill=outline)
        elif prop == "magnifier":
            # Victor — small lens-on-handle magnifier.
            d = ImageDraw.Draw(base)
            handle_tip = add(hand, vec(6.0 * S, angle - 30.0))
            lens_c = add(handle_tip, vec(4.5 * S, angle - 30.0))
            d.line([hand, handle_tip], fill=pal["accent_dark"], width=max(1, int(1.8 * S)))
            d.ellipse(_bbox(lens_c, 5.0 * S, 5.0 * S), fill=None, outline=outline, width=max(1, int(1.4 * S)))
            d.ellipse(_bbox(lens_c, 4.0 * S, 4.0 * S), fill=_scale_color(pal["white"], 0.95), outline=None)
        elif prop == "long_pointer":
            # Peggy — a long pointer / wand ~28 design pixels long.
            d = ImageDraw.Draw(base)
            tip = add(hand, vec(28.0 * S, angle - 12.0))
            d.line([hand, tip], fill=pal["accent_dark"], width=max(1, int(1.6 * S)))
            d.ellipse(_bbox(tip, 1.8 * S, 1.8 * S), fill=pal["accent"], outline=outline, width=max(1, int(0.6 * S)))
            d.line([hand, add(hand, vec(3.0 * S, angle + 168.0))], fill=pal["accent"], width=max(1, int(1.8 * S)))
        elif prop == "lantern":
            # Walter — handheld brass lantern with a glowing
            # interior + a small wire handle.
            d = ImageDraw.Draw(base)
            lantern_c = add(hand, vec(6.0 * S, angle - 30.0))
            # Body — vertical rectangle with rounded top.
            body = (lantern_c[0] - 3.6 * S, lantern_c[1] - 5.0 * S,
                    lantern_c[0] + 3.6 * S, lantern_c[1] + 5.0 * S)
            d.rounded_rectangle(body, radius=1.4 * S, fill=pal["accent_dark"], outline=outline, width=max(1, int(0.9 * S)))
            # Glass inset (lighter glow).
            d.rectangle((lantern_c[0] - 2.6 * S, lantern_c[1] - 3.0 * S,
                         lantern_c[0] + 2.6 * S, lantern_c[1] + 3.0 * S), fill=pal["accent"], outline=None)
            # Wire handle arc above.
            d.arc((lantern_c[0] - 3.6 * S, lantern_c[1] - 9.0 * S,
                   lantern_c[0] + 3.6 * S, lantern_c[1] - 5.0 * S), start=180, end=360, fill=outline, width=max(1, int(1.0 * S)))
        elif prop == "none":
            # Sentinel — characters with no held prop (Olivia, etc.)
            # explicitly set prop: "none" so the dispatch is a no-op
            # rather than silently falling through.
            pass

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

        # Drop-shadow removed — the in-game renderer composites
        # characters over scene geometry that already provides ground
        # contact, and the baked-in shadow ellipse fought camera
        # angles and transparent backgrounds.
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
            hip_spread = (spec.hip_w * 0.26 if spec.outfit in {"general_uniform", "storm_uniform"} else max(3.8, spec.hip_w * 0.18)) * S
            hip = (hip_center[0] + sign * hip_spread, hip_center[1] + 3.0 * S)
            knee = add(hip, vec(spec.leg_upper * S, upper + p.torso_tilt * 0.08))
            ankle = add(knee, vec(spec.leg_lower * S, lower + p.torso_tilt * 0.08))
            return hip, knee, ankle

        def walk_leg_pose(is_near: bool):
            idx = frame_index % 8
            leg_len = (spec.leg_upper + spec.leg_lower) * S
            stride = leg_len * (0.42 if animation == "run" else 0.36)
            base_drop = leg_len * (0.86 if animation == "run" else 0.88)
            far_x = (-1.00, -0.76, -0.36, -0.06, 0.12, -0.18, -0.58, -0.90)
            near_x = (0.92, 0.68, 0.30, 0.04, -0.08, 0.20, 0.62, 0.94)
            far_lift = (0.00, 0.00, 0.05, 0.14, 0.00, 0.02, 0.08, 0.02)
            near_lift = (0.00, 0.02, 0.08, 0.02, 0.00, 0.00, 0.05, 0.14)
            far_shift = (-1.4, -1.0, -0.4, 0.4, 1.0, 0.7, 0.1, -0.6)
            near_shift = (1.3, 0.9, 0.1, -0.8, -1.2, -0.8, -0.2, 0.7)
            foot_tilt = (-7, -5, -2, 3, 7, 5, 2, -3) if animation == "run" else (-6, -4, -1, 2, 6, 4, 1, -2)

            hip_spread = (spec.hip_w * 0.26 if spec.outfit in {"general_uniform", "storm_uniform"} else max(3.8, spec.hip_w * 0.18)) * S
            hip = (hip_center[0] + (hip_spread if is_near else -hip_spread), hip_center[1] + 3.0 * S)
            if is_near:
                ankle = (hip_center[0] + near_x[idx] * stride, hip[1] + base_drop - near_lift[idx] * leg_len)
                ankle = self._clamp_leg_target(hip, ankle, spec.leg_upper * S, spec.leg_lower * S)
                knee, _a1, _a2 = self._solve_leg_ik(hip, ankle, spec.leg_upper * S, spec.leg_lower * S, bend_sign=1.0)
                foot_center = (ankle[0] + spec.foot_w * 0.28 * S + near_shift[idx] * S, ankle[1] + 2.0 * S)
                foot_angle = -foot_tilt[(idx + 4) % 8] + p.torso_tilt * 0.10
            else:
                ankle = (hip_center[0] + far_x[idx] * stride, hip[1] + base_drop - far_lift[idx] * leg_len)
                ankle = self._clamp_leg_target(hip, ankle, spec.leg_upper * S, spec.leg_lower * S)
                knee, _a1, _a2 = self._solve_leg_ik(hip, ankle, spec.leg_upper * S, spec.leg_lower * S, bend_sign=1.0)
                foot_center = (ankle[0] + spec.foot_w * 0.25 * S + far_shift[idx] * S, ankle[1] + 2.0 * S)
                foot_angle = foot_tilt[idx] + p.torso_tilt * 0.08
            return hip, knee, ankle, foot_center, foot_angle

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

        # Side-view depth semantics for right-facing toon rigs:
        # screen-right limb = player-left/back limb (darker, behind),
        # screen-left limb  = player-right/front limb (lighter, in front).
        if animation in {"walk", "run"}:
            front_hip, front_knee, front_ankle, front_foot_center, front_foot_angle = walk_leg_pose(False)
            back_hip, back_knee, back_ankle, back_foot_center, back_foot_angle = walk_leg_pose(True)
        else:
            front_hip, front_knee, front_ankle = leg_points(False)
            back_hip, back_knee, back_ankle = leg_points(True)
            front_foot_center = (front_ankle[0] + spec.foot_w * 0.25 * S, front_ankle[1] + 2.0 * S)
            front_foot_angle = -2.0 + p.torso_tilt * 0.08
            back_foot_center = (back_ankle[0] + spec.foot_w * 0.28 * S, back_ankle[1] + 2.0 * S)
            back_foot_angle = 2.0 + p.torso_tilt * 0.10

        back_tint = _scale_color(pal["outfit_dark"], 0.93)
        front_tint = pal["outfit"]
        draw_capsule(d, back_hip, back_knee, spec.leg_radius * 0.92 * S, back_tint, pal["outline"], 1.1 * S)
        draw_capsule(d, back_knee, back_ankle, spec.leg_radius * 0.88 * S, back_tint, pal["outline"], 1.1 * S)
        draw_rotated_rounded_rect(img, back_foot_center, (spec.foot_w * S, spec.foot_h * S), back_foot_angle, spec.foot_h * 0.48 * S, pal["shoe"], pal["outline"], 1.0 * S)
        back_shoulder, back_elbow, back_hand = arm_points(True)
        draw_capsule(d, back_shoulder, back_elbow, spec.arm_radius * 0.92 * S, back_tint, pal["outline"], 1.1 * S)
        draw_capsule(d, back_elbow, back_hand, spec.arm_radius * 0.88 * S, back_tint, pal["outline"], 1.1 * S)
        draw_armband(back_shoulder, back_elbow, scale=0.88, include_insignia=False)
        draw_uniform_cuff(back_elbow, back_hand, scale=0.88)
        draw_skin_hand(back_hand, scale=0.90, outline_width=0.9)

        # torso/head core silhouette
        self._draw_torso(img, torso_center, spec, pal, S, p)
        self._draw_head(img, head_center, spec, pal, S, p)

        # front limbs and props
        draw_capsule(d, front_hip, front_knee, spec.leg_radius * S, front_tint, pal["outline"], 1.15 * S)
        draw_capsule(d, front_knee, front_ankle, spec.leg_radius * 0.96 * S, front_tint, pal["outline"], 1.15 * S)
        draw_rotated_rounded_rect(img, front_foot_center, (spec.foot_w * S, spec.foot_h * S), front_foot_angle, spec.foot_h * 0.48 * S, pal["shoe"], pal["outline"], 1.0 * S)
        front_shoulder, front_elbow, front_hand = arm_points(False)
        sleeve_fill = pal["outfit"] if spec.outfit in {"poncho", "keeper_robe", "long_coat", "general_uniform", "storm_uniform", "banyan", "eavesdrop_cloak", "field_jacket", "cinched_field_jacket", "formal_robe", "judicial_robe", "vest_over_shirt", "tabard", "cinched_tabard"} else pal["skin"]
        draw_capsule(d, front_shoulder, front_elbow, spec.arm_radius * S, sleeve_fill, pal["outline"], 1.1 * S)
        draw_capsule(d, front_elbow, front_hand, spec.arm_radius * 0.95 * S, sleeve_fill, pal["outline"], 1.1 * S)
        draw_armband(front_shoulder, front_elbow, scale=1.0, include_insignia=True)
        draw_uniform_cuff(front_elbow, front_hand, scale=1.0)

        prop_angle = p.far_arm_lower + p.torso_tilt * 0.10 + (14.0 if p.prop_swing > 0 else 0.0)
        self._draw_prop(img, front_hand, spec, pal, S, prop_angle)
        draw_skin_hand(front_hand, scale=1.0, outline_width=1.0)
        if p.slash > 0.0:
            d.arc((front_hand[0] - 4 * S, front_hand[1] - 28 * S, front_hand[0] + 42 * S, front_hand[1] + 16 * S), start=-70, end=35, fill=with_alpha(pal["accent"], 160), width=max(1, int(2.5 * S)))
        if p.hit > 0.0:
            for off in [(-5, -10), (4, -14), (10, -6)]:
                d.line([(head_center[0] + off[0]*S, head_center[1] + off[1]*S), (head_center[0] + (off[0]+3)*S, head_center[1] + (off[1]-4)*S)], fill=with_alpha(pal["accent"], 180), width=max(1, int(1.2 * S)))
        if ss > 1:
            img = img.resize((W, H), RESAMPLING.LANCZOS if downsample == "lanczos" else RESAMPLING.BICUBIC)
        return img
