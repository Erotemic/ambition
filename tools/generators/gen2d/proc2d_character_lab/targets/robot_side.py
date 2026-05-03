from __future__ import annotations

"""Cute right-facing side-scroller robot target.

The renderer keeps a fixed canvas, fixed ground anchor, and stable part sizes for
every animation.  The ``blink`` row is the Ambition teleport / precision-blink
ability, not an eyelid blink.  Eyelid blinks remain as incidental idle acting.

The robot head is drawn as a rigid local layer and rotated as one unit so the
visor, antenna, face, and shell keep their spatial relationship.  For a
right-facing model, the far arm is drawn behind the body and the near arm / blade
is drawn in front.
"""

import math
import random
from typing import Dict, Optional, Tuple

from PIL import Image, ImageDraw

from .common_draw import RESAMPLING, draw_capsule, draw_rotated_rounded_rect
from .robot25d import BotSpec, Pose, parse_background
from ..rig import add, clamp, ease_in_out_sine, ease_out_cubic, lerp, smoothstep, vec

Color = Tuple[int, int, int, int]
Point = Tuple[float, float]


def _rgba(hex_color: str, alpha: int = 255) -> Color:
    from PIL import ImageColor

    r, g, b = ImageColor.getrgb(hex_color)
    return (r, g, b, alpha)


def _with_alpha(color: Color, alpha: int) -> Color:
    return (color[0], color[1], color[2], alpha)


def _bbox(center: Point, w: float, h: float) -> Tuple[float, float, float, float]:
    return (center[0] - w / 2.0, center[1] - h / 2.0, center[0] + w / 2.0, center[1] + h / 2.0)


def _paste_rotated_local(base: Image.Image, layer: Image.Image, center: Point, angle: float) -> None:
    rotated = layer.rotate(angle, resample=RESAMPLING.BICUBIC, expand=True)
    base.alpha_composite(rotated, (int(center[0] - rotated.width / 2), int(center[1] - rotated.height / 2)))


class SideRobotGenerator:
    name = "robot"

    ANIMATIONS: Dict[str, Dict[str, int]] = {
        "idle": {"frames": 8, "duration_ms": 120},
        "walk": {"frames": 8, "duration_ms": 95},
        "run": {"frames": 8, "duration_ms": 75},
        "jump": {"frames": 6, "duration_ms": 95},
        "fall": {"frames": 6, "duration_ms": 95},
        "slash": {"frames": 8, "duration_ms": 75},
        "hit": {"frames": 5, "duration_ms": 90},
        "death": {"frames": 8, "duration_ms": 110},
        # Ambition blink ability: short teleport / precision-blink visual, not eyelids.
        "blink": {"frames": 8, "duration_ms": 62},
        "dash": {"frames": 6, "duration_ms": 65},
    }

    PALETTE = {
        "shell": _rgba("#FDFDFB"),
        "shell_top": _rgba("#FFFFFF"),
        "shell_side": _rgba("#E8E2DB"),
        "outline": _rgba("#17191F"),
        "joint": _rgba("#5D646D"),
        "joint_dark": _rgba("#333941"),
        "visor": _rgba("#0B111C"),
        "visor_glow": _rgba("#0CEBFF"),
        "accent": _rgba("#C58AFF"),
        "accent_dark": _rgba("#8E56D8"),
        "metal": _rgba("#B4BAC2"),
        "shadow": _rgba("#000000", 38),
    }

    def sample_spec(self, seed: int, archetype: str = "cute_scout") -> BotSpec:
        rng = random.Random(seed)
        scale = 1.0
        if archetype == "guardian":
            scale = 1.07
        elif archetype == "runner":
            scale = 0.97
        return BotSpec(
            target=self.name,
            seed=seed,
            archetype=archetype,
            palette_name="classic",
            head_w=(41 + rng.uniform(-1.0, 1.0)) * scale,
            head_h=(34 + rng.uniform(-1.0, 1.0)) * scale,
            body_w=(26 + rng.uniform(-0.8, 0.8)) * scale,
            body_h=(25 + rng.uniform(-0.8, 0.8)) * scale,
            arm_upper=(13.6 + rng.uniform(-0.4, 0.7)) * scale,
            arm_lower=(11.5 + rng.uniform(-0.4, 0.5)) * scale,
            leg_upper=(13.4 + rng.uniform(-0.4, 0.6)) * scale,
            leg_lower=(12.0 + rng.uniform(-0.4, 0.6)) * scale,
            visor_w=(23.5 + rng.uniform(-0.6, 0.6)) * scale,
            visor_h=(12.0 + rng.uniform(-0.4, 0.4)) * scale,
            antenna_h=(12.0 + rng.uniform(-0.8, 0.8)) * scale,
            blade_len=(30.0 + rng.uniform(-1.0, 2.0)) * scale,
        )

    def pose_for_animation(self, animation: str, frame_index: int, frame_count: int) -> Pose:
        p = Pose()
        t = 0.0 if frame_count <= 1 else frame_index / float(frame_count - 1)
        wave = math.sin(t * math.tau)

        if animation == "idle":
            p.body_bob = abs(wave) * 1.0
            p.body_tilt = wave * 1.2
            p.head_tilt = -wave * 0.8
            p.blink = frame_index == frame_count // 2
            p.eye_squint = 0.08 if frame_index in {1, frame_count - 2} else 0.0
        elif animation == "blink":
            # Teleport/precision-blink: gather, vanish vector, arrival.  The
            # character remains same-scale and anchored; the game-space movement
            # is expressed with FX and a small local recoil, not by rescaling.
            charge = 1.0 - smoothstep(clamp(t / 0.34, 0.0, 1.0))
            arrive = smoothstep(clamp((t - 0.38) / 0.42, 0.0, 1.0))
            pulse = math.sin(t * math.pi)
            p.root_x = lerp(-3.0, 4.0, arrive) - charge * 2.0
            p.root_y = -pulse * 1.3
            p.body_bob = 0.2 * pulse
            p.body_tilt = -12.0 * charge + 7.0 * arrive
            p.head_tilt = -5.0 * charge + 2.0 * arrive
            p.far_arm_upper = lerp(152.0, 178.0, arrive) + charge * 10.0
            p.far_arm_lower = lerp(132.0, 160.0, arrive)
            p.near_arm_upper = lerp(22.0, 10.0, charge) + arrive * 18.0
            p.near_arm_lower = lerp(18.0, 5.0, charge) + arrive * 12.0
            p.far_leg_upper = lerp(105.0, 130.0, pulse)
            p.far_leg_lower = lerp(96.0, 112.0, pulse)
            p.near_leg_upper = lerp(72.0, 94.0, pulse)
            p.near_leg_lower = lerp(85.0, 104.0, pulse)
            p.eye_squint = 0.22 + 0.20 * pulse
        elif animation in {"walk", "run"}:
            stride = math.sin(t * math.tau)
            bounce = (1.0 - math.cos(t * math.tau * 2.0)) * 0.5
            amp = 18.0 if animation == "walk" else 27.0
            arm = 10.0 if animation == "walk" else 15.0
            lean = -1.5 if animation == "walk" else -7.0
            p.root_x = stride * (1.0 if animation == "walk" else 1.7)
            p.body_bob = 0.5 + bounce * (1.8 if animation == "walk" else 2.5)
            p.body_tilt = lean - stride * 3.5
            p.head_tilt = -bounce * 1.5
            p.far_arm_upper = 145 + stride * arm
            p.far_arm_lower = 122 + stride * arm * 0.55
            p.near_arm_upper = 32 - stride * arm
            p.near_arm_lower = 20 - stride * arm * 0.55
            p.far_leg_upper = 105 + stride * amp
            p.far_leg_lower = 97 - max(0.0, stride) * 22.0 + max(0.0, -stride) * 8.0
            p.near_leg_upper = 72 - stride * amp
            p.near_leg_lower = 85 - max(0.0, -stride) * 22.0 + max(0.0, stride) * 8.0
            p.eye_squint = 0.08 + bounce * 0.12
        elif animation == "jump":
            arc = math.sin(t * math.pi)
            lift = ease_in_out_sine(arc)
            p.root_y = -18.0 * lift
            p.root_x = 2.0 * t
            p.body_tilt = -4.0 + 4.0 * t
            p.head_tilt = -2.0 - 2.0 * lift
            p.far_arm_upper = 165 - 18 * lift
            p.far_arm_lower = 142 - 10 * lift
            p.near_arm_upper = 8 + 18 * lift
            p.near_arm_lower = 8 + 12 * lift
            p.far_leg_upper = 128
            p.far_leg_lower = 78
            p.near_leg_upper = 88
            p.near_leg_lower = 62
            p.eye_squint = 0.08
        elif animation == "fall":
            p.root_y = -10.0 + 10.0 * t
            p.body_tilt = 5.0 + 5.0 * t
            p.head_tilt = 2.0 * t
            p.far_arm_upper = 176 - 10 * t
            p.far_arm_lower = 160 - 8 * t
            p.near_arm_upper = 4 + 12 * t
            p.near_arm_lower = 14 + 8 * t
            p.far_leg_upper = 132 - 8 * t
            p.far_leg_lower = 130 - 18 * t
            p.near_leg_upper = 94 - 6 * t
            p.near_leg_lower = 112 - 16 * t
            p.eye_squint = 0.16
        elif animation == "slash":
            wind = 1.0 - smoothstep(clamp(t / 0.28, 0.0, 1.0))
            strike = smoothstep(clamp((t - 0.22) / 0.36, 0.0, 1.0))
            recover = smoothstep(clamp((t - 0.68) / 0.32, 0.0, 1.0))
            p.root_x = -2.0 * wind + 4.0 * strike - 1.0 * recover
            p.body_tilt = -8.0 * wind + 12.0 * strike - 3.0 * recover
            p.head_tilt = -2.0 * wind + 4.0 * strike
            p.far_arm_upper = 156
            p.far_arm_lower = 145
            p.near_arm_upper = -22 - 20 * wind + 52 * strike - 14 * recover
            p.near_arm_lower = -12 - 18 * wind + 48 * strike - 16 * recover
            p.far_leg_upper = 108 + 10 * strike
            p.near_leg_upper = 62 - 8 * wind
            p.slash = max(0.2, wind, strike)
            p.slash_arc = strike
            p.eye_squint = 0.22 + strike * 0.20
        elif animation == "hit":
            j = abs(math.sin(t * math.pi * 2.0))
            p.root_x = -4.0 * j
            p.root_y = 1.8 * j
            p.body_tilt = -15.0 * j
            p.head_tilt = -12.0 * j
            p.far_arm_upper = 176
            p.far_arm_lower = 162
            p.near_arm_upper = 48
            p.near_arm_lower = 58
            p.far_leg_upper = 116
            p.far_leg_lower = 108
            p.near_leg_upper = 88
            p.near_leg_lower = 96
            p.eye_squint = 0.5
        elif animation == "dash":
            surge = ease_in_out_sine(t)
            pulse = math.sin(t * math.pi)
            p.root_x = 6.0 + surge * 3.0
            p.root_y = -1.0 + pulse * 0.4
            p.body_tilt = -18.0 + wave * 1.4
            p.head_tilt = -4.0
            p.far_arm_upper = 170 + wave * 2.0
            p.far_arm_lower = 170 + wave * 2.0
            p.near_arm_upper = 158 + wave * 2.0
            p.near_arm_lower = 165 + wave * 2.0
            p.far_leg_upper = 142 + wave * 2.0
            p.far_leg_lower = 145 + wave * 3.0
            p.near_leg_upper = 128 + wave * 2.0
            p.near_leg_lower = 135 + wave * 3.0
            p.dash = 1.0
            p.eye_squint = 0.30
        elif animation == "death":
            fall = ease_out_cubic(t)
            p.root_x = lerp(0.0, -4.0, fall)
            p.root_y = lerp(0.0, 4.0, fall)
            p.body_tilt = lerp(0.0, 73.0, fall)
            p.head_tilt = lerp(0.0, 66.0, fall)
            p.far_arm_upper = lerp(145.0, 196.0, fall)
            p.far_arm_lower = lerp(122.0, 218.0, fall)
            p.near_arm_upper = lerp(32.0, 96.0, fall)
            p.near_arm_lower = lerp(20.0, 118.0, fall)
            p.far_leg_upper = lerp(105.0, 156.0, fall)
            p.far_leg_lower = lerp(97.0, 172.0, fall)
            p.near_leg_upper = lerp(72.0, 118.0, fall)
            p.near_leg_lower = lerp(85.0, 144.0, fall)
            p.collapse = fall
            p.dead = True
            p.eye_squint = 0.55
        return p

    def _leg_chain(self, hip: Point, upper_len: float, lower_len: float, a1: float, a2: float) -> Tuple[Point, Point]:
        knee = add(hip, vec(upper_len, a1))
        ankle = add(knee, vec(lower_len, a2))
        return knee, ankle

    def _draw_shadow(self, img: Image.Image, ground_y: float, x: float, width: float, alpha: int) -> None:
        d = ImageDraw.Draw(img)
        d.ellipse((x - width / 2, ground_y - 5, x + width / 2, ground_y + 6), fill=(0, 0, 0, alpha))

    def _draw_blink_fx(self, img: Image.Image, root_x: float, ground_y: float, S: float, frame_index: int, frame_count: int) -> None:
        d = ImageDraw.Draw(img)
        t = 0.0 if frame_count <= 1 else frame_index / float(frame_count - 1)
        charge = 1.0 - smoothstep(clamp(t / 0.35, 0.0, 1.0))
        transit = math.sin(clamp((t - 0.12) / 0.72, 0.0, 1.0) * math.pi)
        arrive = smoothstep(clamp((t - 0.45) / 0.40, 0.0, 1.0))
        energy = self.PALETTE["visor_glow"]
        accent = self.PALETTE["accent"]
        mid_y = ground_y - 52 * S
        source_x = root_x - 24 * S
        dest_x = root_x + 30 * S

        # Precision-blink aim line and destination ring.
        alpha_line = int(40 + 115 * max(charge, transit))
        d.line([(source_x, mid_y), (dest_x, mid_y - 5 * S)], fill=_with_alpha(energy, alpha_line), width=max(1, int(1.2 * S)))
        for rscale, alpha in [(1.0 + 0.5 * arrive, 150), (0.56 + 0.25 * charge, 105)]:
            rx, ry = 7.0 * S * rscale, 13.0 * S * rscale
            box = (dest_x - rx, mid_y - ry - 5 * S, dest_x + rx, mid_y + ry - 5 * S)
            d.ellipse(box, outline=_with_alpha(energy, int(alpha * max(0.25, charge + arrive))), width=max(1, int(1.3 * S)))

        # Departure/arrival slivers read like teleport afterimages without
        # turning the character itself translucent.
        for i, frac in enumerate((0.0, 0.33, 0.66, 1.0)):
            x = lerp(source_x, dest_x, frac)
            h = (31.0 - i * 3.4 + transit * 5.0) * S
            a = int((85 - i * 14) * transit)
            if a > 0:
                d.line([(x, mid_y - h / 2), (x + 7 * S, mid_y + h / 2)], fill=_with_alpha(accent, a), width=max(1, int(1.8 * S)))
                d.line([(x + 3 * S, mid_y - h / 2), (x - 4 * S, mid_y + h / 2)], fill=_with_alpha(energy, max(15, a - 20)), width=max(1, int(0.9 * S)))

    def _draw_rigid_head(self, img: Image.Image, center: Point, spec: BotSpec, pal: Dict[str, Color], S: float, angle: float, blink_closed: bool, squint: float, dead: bool) -> None:
        # Draw in head-local coordinates, then rotate/paste the full layer.  This
        # preserves the older in-repo rigid 2.5D-head idea while remaining pure 2D.
        pad = int(math.ceil(48 * S))
        layer = Image.new("RGBA", (pad * 2, pad * 2), (0, 0, 0, 0))
        d = ImageDraw.Draw(layer)
        cx, cy = float(pad), float(pad)
        outline = max(1, int(round(1.8 * S)))
        head_w = spec.head_w * S
        head_h = spec.head_h * S

        # Antenna is part of the rigid head layer.
        ant_base = (cx - 8 * S, cy - head_h * 0.50)
        ant_tip = (cx - 12 * S, cy - head_h * 0.50 - spec.antenna_h * S)
        d.line([ant_base, ant_tip], fill=pal["outline"], width=max(1, int(1.7 * S)))
        d.ellipse(_bbox(ant_tip, 6.4 * S, 6.4 * S), fill=pal["accent"], outline=pal["outline"], width=max(1, int(1.0 * S)))

        outer = _bbox((cx, cy), head_w + 2 * outline, head_h + 2 * outline)
        inner = _bbox((cx, cy), head_w, head_h)
        d.rounded_rectangle(outer, radius=9 * S + outline, fill=pal["outline"])
        d.rounded_rectangle(inner, radius=9 * S, fill=pal["shell_top"])
        d.rounded_rectangle((inner[0] + 4 * S, inner[1] + 3 * S, inner[2] - 5 * S, cy - 1 * S), radius=7 * S, fill=_with_alpha((255, 255, 255, 255), 205))
        d.rounded_rectangle((inner[0] + 8 * S, cy + 1 * S, inner[2] - 2 * S, inner[3] - 3 * S), radius=7 * S, fill=_with_alpha(pal["shell_side"], 190))

        visor_center = (cx + 7.0 * S, cy - 1.0 * S)
        visor_h = spec.visor_h * S
        if blink_closed:
            visor_h = max(2.0 * S, visor_h * 0.22)
        else:
            visor_h *= max(0.35, 1.0 - squint * 0.50)
        vouter = _bbox(visor_center, spec.visor_w * S + outline * 0.6, visor_h + outline * 0.6)
        vinner = _bbox(visor_center, spec.visor_w * S, visor_h)
        d.rounded_rectangle(vouter, radius=4 * S + outline * 0.25, fill=pal["outline"])
        d.rounded_rectangle(vinner, radius=4 * S, fill=pal["visor"])
        if dead:
            x, y = visor_center
            r = 4.0 * S
            d.line([(x - r, y - r), (x + r, y + r)], fill=pal["visor_glow"], width=max(1, int(1.3 * S)))
            d.line([(x - r, y + r), (x + r, y - r)], fill=pal["visor_glow"], width=max(1, int(1.3 * S)))
        elif not blink_closed:
            for ex in (-4.0, 4.0):
                d.ellipse(_bbox((visor_center[0] + ex * S, visor_center[1]), 3.0 * S, 6.0 * S), fill=pal["visor_glow"])

        _paste_rotated_local(img, layer, center, angle)

    def _draw_robot_arm(self, img: Image.Image, d: ImageDraw.ImageDraw, shoulder: Point, a1: float, a2: float, tint: Color, spec: BotSpec, pal: Dict[str, Color], S: float, outline: float, slash: float = 0.0, slash_arc: float = 0.0) -> Point:
        elbow = add(shoulder, vec(spec.arm_upper * S, a1))
        hand = add(elbow, vec(spec.arm_lower * S, a2))
        draw_capsule(d, shoulder, elbow, 2.7 * S, tint, pal["outline"], outline * 0.65)
        draw_capsule(d, elbow, hand, 2.5 * S, tint, pal["outline"], outline * 0.65)
        d.ellipse((hand[0] - 4 * S, hand[1] - 4 * S, hand[0] + 4 * S, hand[1] + 4 * S), fill=tint, outline=pal["outline"], width=max(1, int(outline * 0.65)))
        if slash:
            blade_angle = -18 + slash_arc * 52
            tip = add(hand, vec(spec.blade_len * S, blade_angle))
            d.line([hand, tip], fill=pal["outline"], width=max(1, int(4.0 * S)))
            d.line([hand, tip], fill=pal["accent"], width=max(1, int(2.1 * S)))
            if slash_arc > 0.18:
                arc_box = (hand[0] - 5 * S, hand[1] - 34 * S, hand[0] + 42 * S, hand[1] + 20 * S)
                d.arc(arc_box, start=-70, end=42, fill=(12, 235, 255, 170), width=max(1, int(2.4 * S)))
        return hand

    def _render_highres(self, spec: BotSpec, animation: str, frame_index: int, frame_count: int, size: Tuple[int, int], background: Optional[Color], scale: int) -> Image.Image:
        W, H = size[0] * scale, size[1] * scale
        bg = (0, 0, 0, 0) if background is None else background
        img = Image.new("RGBA", (W, H), bg)
        S = float(scale)
        pal = self.PALETTE
        p = self.pose_for_animation(animation, frame_index, frame_count)
        ground_y = (101.0 + p.root_y) * S
        root_x = (62.0 + p.root_x) * S
        outline = 1.8 * S

        self._draw_shadow(img, ground_y, root_x + 3 * S, (55 + 18 * p.collapse) * S, int(32 * (1 - 0.35 * p.collapse)))
        d = ImageDraw.Draw(img)

        if animation == "blink":
            self._draw_blink_fx(img, root_x, ground_y, S, frame_index, frame_count)

        if p.dash:
            for i in range(4):
                y = (49 + i * 12 + math.sin(frame_index + i) * 2) * S
                d.line([(14 * S, y), ((43 - i * 3) * S, y - 2 * S)], fill=(12, 235, 255, 90), width=max(1, int(1.6 * S)))

        # Stable body reference. Death moves to a lying pose without scaling.
        collapse = p.collapse
        body_center = (root_x + lerp(0, 12 * S, collapse), ground_y - lerp(39 * S, 11 * S, collapse) + p.body_bob * S)
        head_center = (root_x + lerp(12 * S, 34 * S, collapse), ground_y - lerp(68 * S, 15 * S, collapse) + p.body_bob * S * 0.4)
        body_angle = p.body_tilt
        head_angle = p.head_tilt

        hip_far = (body_center[0] - 6 * S, body_center[1] + 11 * S)
        hip_near = (body_center[0] + 8 * S, body_center[1] + 10 * S)
        shoulder_far = (body_center[0] - 8 * S, body_center[1] - 8 * S)
        shoulder_near = (body_center[0] + 9 * S, body_center[1] - 8 * S)

        # Legs sit below the torso. Far/near tints preserve side-view depth.
        for hip, a1, a2, tint, foot_shift in [
            (hip_far, p.far_leg_upper, p.far_leg_lower, pal["shell_side"], -2.0),
            (hip_near, p.near_leg_upper, p.near_leg_lower, pal["shell"], 3.0),
        ]:
            knee, ankle = self._leg_chain(hip, spec.leg_upper * S, spec.leg_lower * S, a1, a2)
            draw_capsule(d, hip, knee, 2.9 * S, tint, pal["outline"], outline * 0.65)
            draw_capsule(d, knee, ankle, 2.7 * S, tint, pal["outline"], outline * 0.65)
            foot_w = 12 * S
            foot_h = 6 * S
            foot_center = (ankle[0] + (foot_w * 0.34) + foot_shift * S, min(ground_y - 2 * S, ankle[1] + 2 * S))
            draw_rotated_rounded_rect(img, foot_center, (foot_w, foot_h), -4 + body_angle * 0.10, 3 * S, tint, pal["outline"], outline * 0.7)

        # Far/back arm first so it disappears correctly behind the body.
        self._draw_robot_arm(img, d, shoulder_far, p.far_arm_upper, p.far_arm_lower, pal["shell_side"], spec, pal, S, outline)

        # Body and rigid head.
        draw_rotated_rounded_rect(img, body_center, (spec.body_w * S, spec.body_h * S), body_angle, 7 * S, pal["shell"], pal["outline"], outline)
        draw_rotated_rounded_rect(img, (body_center[0] + 3 * S, body_center[1] - 1 * S), (10 * S, 9 * S), body_angle, 2.5 * S, pal["accent"], pal["outline"], outline * 0.45)
        self._draw_rigid_head(img, head_center, spec, pal, S, head_angle, p.blink, p.eye_squint, p.dead)

        # Near/front arm and weapon after the torso/head.
        self._draw_robot_arm(img, d, shoulder_near, p.near_arm_upper, p.near_arm_lower, pal["shell"], spec, pal, S, outline, p.slash, p.slash_arc)

        return img

    def render_animation_frame(
        self,
        spec: BotSpec,
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
