from __future__ import annotations

"""AI Slop Zeta: a bespoke scary side-scroller boss.

Unlike the robot / goblin targets, this is not a player-like humanoid.  It is a
floating horror made of a rigid skull-mask head, ragged cloak mass, many eye
cores, and tendril-arms.  The animation set is boss-specific and named to
match the Rust `BossAttackKind` verbs: Rest, FloorSlam, SideSweep, SpikeHalo,
and DashEcho, with hit / death feedback rows for runtime presentation.

Implementation notes:
- pure PIL / procedural rendering
- fixed frame canvas and stable anchor behavior
- rigid head rendering via a local layer, then rotated/composited as one unit
- right-facing side-view; far appendages first, near appendages later
"""

import math
import random
from dataclasses import asdict, dataclass
from typing import Dict, Optional, Tuple

from PIL import Image, ImageColor, ImageDraw
from ambition_sprite2d_renderer.core.draw import rgba, with_alpha, bbox_from_center as _bbox

from ...authoring.common_draw import (
    RESAMPLING,
    draw_capsule,
    draw_rotated_ellipse,
    draw_rotated_rounded_rect,
)
from ...authoring.rig import add, clamp, ease_in_out_sine, ease_out_cubic, lerp, smoothstep, vec

Color = Tuple[int, int, int, int]
Point = Tuple[float, float]






def parse_background(value: str) -> Optional[Color]:
    return None if str(value).lower() == "transparent" else rgba(str(value))




def _paste_rotated_local(
    base: Image.Image, layer: Image.Image, center: Point, angle: float
) -> None:
    rotated = layer.rotate(angle, resample=RESAMPLING.BICUBIC, expand=True)
    base.alpha_composite(
        rotated,
        (int(center[0] - rotated.width / 2), int(center[1] - rotated.height / 2)),
    )


@dataclass(frozen=True)
class ZetaSpec:
    target: str
    seed: int
    archetype: str
    palette_name: str
    head_w: float
    head_h: float
    hood_w: float
    hood_h: float
    eye_r: float
    horn_len: float
    tendril_upper: float
    tendril_lower: float
    claw_r: float
    cloak_w: float
    cloak_h: float
    tail_len: float


@dataclass
class ZetaPose:
    root_x: float = 0.0
    root_y: float = 0.0
    hover: float = 0.0
    body_tilt: float = 0.0
    head_tilt: float = 0.0
    far_arm_upper: float = 160.0
    far_arm_lower: float = 145.0
    near_arm_upper: float = 18.0
    near_arm_lower: float = 20.0
    jaw_open: float = 0.0
    eye_glow: float = 0.2
    cloak_flare: float = 0.0
    summon: float = 0.0
    side_sweep: float = 0.0
    spike_halo: float = 0.0
    dash_echo: float = 0.0
    spit: float = 0.0
    beam_charge: float = 0.0
    beam_fire: float = 0.0
    slam: float = 0.0
    teleport: float = 0.0
    collapse: float = 0.0
    dead: bool = False


class AISlopZetaGenerator:
    name = "boss"

    ANIMATIONS: Dict[str, Dict[str, int]] = {
        # Rows named after ambition_engine::BossAttackKind where applicable.
        "rest": {"frames": 8, "duration_ms": 120},
        "floor_slam": {"frames": 7, "duration_ms": 82},
        "side_sweep": {"frames": 7, "duration_ms": 72},
        "spike_halo": {"frames": 8, "duration_ms": 92},
        "dash_echo": {"frames": 7, "duration_ms": 62},
        # Presentation feedback rows for sandbox hit/death signals.
        "hit": {"frames": 5, "duration_ms": 90},
        "death": {"frames": 8, "duration_ms": 110},
    }

    PALETTE = {
        "outline": rgba("#100A16"),
        "cloak": rgba("#18131F"),
        "cloak_mid": rgba("#2A2036"),
        "cloak_hi": rgba("#47305D"),
        "skin": rgba("#B9B0C6"),
        "skin_shadow": rgba("#756B85"),
        "eye": rgba("#FF4BDD"),
        "eye_soft": rgba("#FF9DF1"),
        "mouth": rgba("#100A16"),
        "tooth": rgba("#F3E7D9"),
        "energy": rgba("#B447FF"),
        "energy2": rgba("#FF47CF"),
        "energy3": rgba("#70E0FF"),
        "shadow": rgba("#000000", 42),
    }

    def sample_spec(self, seed: int, archetype: str = "ai_slop_zeta") -> ZetaSpec:
        rng = random.Random(seed)
        return ZetaSpec(
            target=self.name,
            seed=seed,
            archetype=archetype,
            palette_name="ai_slop_zeta",
            head_w=rng.uniform(28.0, 31.0),
            head_h=rng.uniform(33.0, 36.0),
            hood_w=rng.uniform(40.0, 46.0),
            hood_h=rng.uniform(39.0, 44.0),
            eye_r=rng.uniform(4.0, 5.0),
            horn_len=rng.uniform(12.0, 15.0),
            tendril_upper=rng.uniform(16.0, 18.5),
            tendril_lower=rng.uniform(17.0, 20.0),
            claw_r=rng.uniform(4.0, 5.0),
            cloak_w=rng.uniform(52.0, 58.0),
            cloak_h=rng.uniform(52.0, 58.0),
            tail_len=rng.uniform(22.0, 26.0),
        )

    def spec_dict(self, spec: ZetaSpec) -> Dict[str, object]:
        return asdict(spec)

    def pose_for_animation(
        self, animation: str, frame_index: int, frame_count: int
    ) -> ZetaPose:
        p = ZetaPose()
        t = 0.0 if frame_count <= 1 else frame_index / float(frame_count - 1)
        wave = math.sin(t * math.tau)
        pulse = math.sin(t * math.pi)

        if animation in {"rest", "idle"}:
            p.hover = 1.8 * abs(wave)
            p.body_tilt = wave * 2.4
            p.head_tilt = -wave * 2.0
            p.cloak_flare = 0.12 + abs(wave) * 0.12
            p.eye_glow = 0.30 + abs(wave) * 0.20
        elif animation == "hover":
            p.hover = 4.0 * math.sin(t * math.tau)
            p.root_x = 1.6 * math.sin(t * math.tau)
            p.body_tilt = 5.0 * math.sin(t * math.tau)
            p.head_tilt = -4.0 * math.sin(t * math.tau)
            p.cloak_flare = 0.18 + abs(wave) * 0.16
            p.eye_glow = 0.40 + abs(wave) * 0.22
            p.far_arm_upper += wave * 10
            p.near_arm_upper -= wave * 12
        elif animation == "spike_halo":
            charge = smoothstep(t)
            p.hover = 1.6 + pulse * 1.0
            p.body_tilt = -7.0 * charge
            p.head_tilt = -6.0 * charge
            p.cloak_flare = 0.25 + charge * 0.35
            p.summon = charge
            p.spike_halo = charge
            p.eye_glow = 0.45 + charge * 0.35
            p.far_arm_upper = 196.0 - 34.0 * charge
            p.far_arm_lower = 182.0 - 25.0 * charge
            p.near_arm_upper = -16.0 + 18.0 * charge
            p.near_arm_lower = -8.0 + 22.0 * charge
            p.jaw_open = 0.16 + charge * 0.24
        elif animation == "side_sweep":
            wind = 1.0 - smoothstep(clamp(t / 0.36, 0.0, 1.0))
            sweep = smoothstep(clamp((t - 0.20) / 0.48, 0.0, 1.0))
            recover = smoothstep(clamp((t - 0.68) / 0.32, 0.0, 1.0))
            p.root_x = -3.0 * wind + 2.0 * sweep - 1.0 * recover
            p.body_tilt = -16.0 * wind + 18.0 * sweep - 6.0 * recover
            p.head_tilt = -10.0 * wind + 10.0 * sweep
            p.jaw_open = 0.20 + 0.20 * sweep
            p.side_sweep = sweep * (1.0 - 0.35 * recover)
            p.eye_glow = 0.50 + p.side_sweep * 0.42
            p.far_arm_upper = 186.0 - 36.0 * sweep
            p.far_arm_lower = 168.0 - 48.0 * sweep
            p.near_arm_upper = -34.0 + 92.0 * sweep
            p.near_arm_lower = -18.0 + 86.0 * sweep
            p.cloak_flare = 0.22 + 0.20 * p.side_sweep
        elif animation == "beam_charge":
            charge = smoothstep(t)
            p.hover = 1.0 + pulse * 1.0
            p.root_x = -1.0 * charge
            p.body_tilt = -14.0 * charge
            p.head_tilt = -18.0 * charge
            p.jaw_open = 0.24 + charge * 0.22
            p.beam_charge = charge
            p.eye_glow = 0.55 + charge * 0.45
            p.cloak_flare = 0.18 + charge * 0.22
            p.far_arm_upper = 172.0 + 12.0 * charge
            p.near_arm_upper = 6.0 - 14.0 * charge
        elif animation == "beam_fire":
            fire = smoothstep(t)
            p.body_tilt = -18.0
            p.head_tilt = -20.0
            p.jaw_open = 0.52
            p.beam_fire = fire
            p.eye_glow = 1.0
            p.cloak_flare = 0.24
            p.far_arm_upper = 176.0
            p.near_arm_upper = -8.0
        elif animation == "floor_slam":
            wind = 1.0 - smoothstep(clamp(t / 0.36, 0.0, 1.0))
            hit = smoothstep(clamp((t - 0.34) / 0.34, 0.0, 1.0))
            settle = smoothstep(clamp((t - 0.66) / 0.34, 0.0, 1.0))
            p.root_y = -8.0 * wind + 4.0 * hit
            p.hover = 1.0 - 2.0 * hit
            p.body_tilt = -10.0 * wind + 18.0 * hit - 4.0 * settle
            p.head_tilt = -8.0 * wind + 14.0 * hit
            p.slam = hit
            p.cloak_flare = 0.16 + wind * 0.26 + hit * 0.28
            p.far_arm_upper = 210.0 - 58.0 * hit
            p.far_arm_lower = 194.0 - 70.0 * hit
            p.near_arm_upper = -32.0 + 56.0 * hit
            p.near_arm_lower = -24.0 + 64.0 * hit
            p.eye_glow = 0.50 + hit * 0.30
        elif animation == "dash_echo":
            charge = smoothstep(clamp(t / 0.34, 0.0, 1.0))
            dash = smoothstep(clamp((t - 0.22) / 0.42, 0.0, 1.0))
            recover = smoothstep(clamp((t - 0.66) / 0.34, 0.0, 1.0))
            p.root_x = -7.0 * charge + 13.0 * dash - 4.0 * recover
            p.root_y = -1.5 * charge + 1.4 * recover
            p.body_tilt = -20.0 * charge + 30.0 * dash - 10.0 * recover
            p.head_tilt = -12.0 * charge + 16.0 * dash
            p.dash_echo = dash
            p.cloak_flare = 0.20 + charge * 0.20 + dash * 0.34
            p.eye_glow = 0.55 + dash * 0.38
            p.far_arm_upper = 174.0 + 22.0 * charge - 24.0 * dash
            p.far_arm_lower = 160.0 + 16.0 * charge - 20.0 * dash
            p.near_arm_upper = 6.0 - 32.0 * charge + 44.0 * dash
            p.near_arm_lower = 4.0 - 22.0 * charge + 34.0 * dash
        elif animation == "teleport_in_legacy":
            appear = smoothstep(clamp(t / 0.62, 0.0, 1.0))
            recoil = 1.0 - ease_out_cubic(appear)
            p.root_x = 5.0 * recoil
            p.root_y = 2.0 * recoil
            p.body_tilt = 18.0 * recoil - 4.0 * appear
            p.head_tilt = 12.0 * recoil - 3.0 * appear
            p.teleport = 1.0 - appear
            p.cloak_flare = 0.42 - appear * 0.18
            p.eye_glow = 0.88 - appear * 0.24
            p.far_arm_upper = 188.0 - 28.0 * appear
            p.near_arm_upper = 40.0 - 24.0 * appear
        elif animation == "hit":
            j = abs(math.sin(t * math.pi * 2.0))
            p.root_x = -5.0 * j
            p.root_y = 2.0 * j
            p.body_tilt = -16.0 * j
            p.head_tilt = -14.0 * j
            p.cloak_flare = 0.22 + j * 0.12
            p.eye_glow = 0.80
            p.far_arm_upper = 192.0
            p.near_arm_upper = 54.0
            p.jaw_open = 0.30
        elif animation == "death":
            fall = ease_out_cubic(t)
            p.root_x = lerp(0.0, -7.0, fall)
            p.root_y = lerp(0.0, 12.0, fall)
            p.body_tilt = lerp(0.0, 74.0, fall)
            p.head_tilt = lerp(0.0, 58.0, fall)
            p.cloak_flare = 0.30 - fall * 0.16
            p.collapse = fall
            p.dead = t > 0.60
            p.eye_glow = 0.90 - fall * 0.85
            p.jaw_open = 0.18 + fall * 0.32
            p.far_arm_upper = lerp(160.0, 130.0, fall)
            p.far_arm_lower = lerp(145.0, 104.0, fall)
            p.near_arm_upper = lerp(18.0, 70.0, fall)
            p.near_arm_lower = lerp(20.0, 96.0, fall)
        return p

    def _leg_chain(
        self, hip: Point, upper: float, lower: float, a1: float, a2: float
    ) -> Tuple[Point, Point]:
        knee = add(hip, vec(upper, a1))
        ankle = add(knee, vec(lower, a2))
        return knee, ankle

    def _draw_shadow(
        self,
        img: Image.Image,
        ground_y: float,
        center_x: float,
        radius_x: float,
        alpha: int,
    ) -> None:
        draw_rotated_ellipse(
            img,
            (center_x, ground_y + 2),
            (radius_x, 8),
            0.0,
            with_alpha(self.PALETTE["shadow"], alpha),
            None,
            0,
        )

    def _draw_dash_echo_legacy_fx(
        self,
        img: Image.Image,
        root_x: float,
        ground_y: float,
        S: float,
        frame_index: int,
        frame_count: int,
    ) -> None:
        d = ImageDraw.Draw(img)
        t = 0.0 if frame_count <= 1 else frame_index / float(frame_count - 1)
        ring = 18 + 24 * smoothstep(t)
        for i, color in enumerate(
            [self.PALETTE["energy2"], self.PALETTE["energy"], self.PALETTE["energy3"]]
        ):
            alpha = int(140 * (1.0 - 0.16 * i) * (1.0 - 0.15 * t))
            d.arc(
                (
                    root_x - (ring + i * 8) * S,
                    ground_y - (48 + i * 4) * S,
                    root_x + (ring + i * 8) * S,
                    ground_y + (12 + i * 4) * S,
                ),
                -40,
                55,
                fill=with_alpha(color, alpha),
                width=max(1, int((2.2 - i * 0.3) * S)),
            )
        for j in range(9):
            frac = j / 8.0
            ang = -100 + frac * 130
            start = (
                root_x + math.cos(math.radians(ang)) * 12 * S,
                ground_y - 34 * S + math.sin(math.radians(ang)) * 10 * S,
            )
            end = (
                root_x + math.cos(math.radians(ang)) * (18 + 20 * t) * S,
                ground_y - 34 * S + math.sin(math.radians(ang)) * (16 + 24 * t) * S,
            )
            d.line(
                [start, end],
                fill=with_alpha(self.PALETTE["energy2"], int(140 * (1.0 - 0.4 * t))),
                width=max(1, int(1.6 * S)),
            )

    def _draw_teleport_in_legacy_fx(
        self,
        img: Image.Image,
        root_x: float,
        ground_y: float,
        S: float,
        frame_index: int,
        frame_count: int,
    ) -> None:
        d = ImageDraw.Draw(img)
        t = 0.0 if frame_count <= 1 else frame_index / float(frame_count - 1)
        p = 1.0 - smoothstep(t)
        ring = 18 + 22 * p
        for i, color in enumerate(
            [self.PALETTE["energy3"], self.PALETTE["energy"], self.PALETTE["energy2"]]
        ):
            alpha = int(160 * (1.0 - 0.22 * i) * p)
            d.arc(
                (
                    root_x - (ring + i * 8) * S,
                    ground_y - (46 + i * 4) * S,
                    root_x + (ring + i * 8) * S,
                    ground_y + (10 + i * 4) * S,
                ),
                -50,
                65,
                fill=with_alpha(color, alpha),
                width=max(1, int((2.2 - i * 0.25) * S)),
            )
        for j in range(10):
            frac = j / 9.0
            x = root_x - 16 * S + frac * 34 * S
            d.line(
                [(x, ground_y - (10 + 34 * p) * S), (x, ground_y - 4 * S)],
                fill=with_alpha(self.PALETTE["energy3"], int(150 * p)),
                width=max(1, int(1.6 * S)),
            )

    def _composite_teleport_actor(
        self,
        base: Image.Image,
        actor: Image.Image,
        mode: str,
        frame_index: int,
        frame_count: int,
        S: float,
    ) -> None:
        bbox = actor.getchannel("A").getbbox()
        if bbox is None:
            return
        x1, y1, x2, y2 = bbox
        slice_w = max(2, int(3 * S))
        t = 0.0 if frame_count <= 1 else frame_index / float(frame_count - 1)
        if mode == "dash_echo_legacy":
            progress = smoothstep(t)
            for i, x in enumerate(range(x1, x2, slice_w)):
                strip = actor.crop((x, y1, min(x + slice_w, x2), y2))
                if strip.getchannel("A").getbbox() is None:
                    continue
                frac = (
                    0.5
                    if x2 == x1
                    else ((x + slice_w * 0.5) - x1) / float(max(1, x2 - x1))
                )
                dx = (frac - 0.5) * (26.0 * S * progress)
                dy = -(4.0 + abs(frac - 0.5) * 18.0) * S * progress
                alpha_scale = max(0.0, 1.0 - progress * (0.72 + 0.20 * abs(frac - 0.5)))
                if progress > 0.42 and (i + frame_index) % 4 == 0:
                    alpha_scale *= 0.45
                a = strip.getchannel("A").point(
                    lambda v, s=alpha_scale: max(0, min(255, int(v * s)))
                )
                strip.putalpha(a)
                base.alpha_composite(strip, (int(x + dx), int(y1 + dy)))
        else:
            progress = smoothstep(t)
            for i, x in enumerate(range(x1, x2, slice_w)):
                strip = actor.crop((x, y1, min(x + slice_w, x2), y2))
                if strip.getchannel("A").getbbox() is None:
                    continue
                frac = (
                    0.5
                    if x2 == x1
                    else ((x + slice_w * 0.5) - x1) / float(max(1, x2 - x1))
                )
                dx = (frac - 0.5) * (28.0 * S * (1.0 - progress))
                dy = -(4.0 + abs(frac - 0.5) * 18.0) * S * (1.0 - progress)
                alpha_scale = min(1.0, 0.18 + 0.94 * progress)
                if progress < 0.45 and (i + frame_index) % 4 == 0:
                    alpha_scale *= 0.55
                a = strip.getchannel("A").point(
                    lambda v, s=alpha_scale: max(0, min(255, int(v * s)))
                )
                strip.putalpha(a)
                base.alpha_composite(strip, (int(x + dx), int(y1 + dy)))
            full_alpha = smoothstep(clamp((progress - 0.34) / 0.66, 0.0, 1.0))
            if full_alpha > 0:
                resolved = actor.copy()
                a = resolved.getchannel("A").point(
                    lambda v, s=full_alpha: max(0, min(255, int(v * s)))
                )
                resolved.putalpha(a)
                base.alpha_composite(resolved)

    def _draw_head(
        self,
        img: Image.Image,
        center: Point,
        spec: ZetaSpec,
        pal: Dict[str, Color],
        S: float,
        angle: float,
        jaw_open: float,
        eye_glow: float,
        dead: bool,
    ) -> None:
        pad = int(math.ceil(42 * S))
        layer = Image.new("RGBA", (pad * 2, pad * 2), (0, 0, 0, 0))
        d = ImageDraw.Draw(layer)
        cx, cy = float(pad), float(pad)
        outline = max(1, int(round(1.7 * S)))

        # horns / crown spikes
        for sign in (-1, 1):
            tip = (
                cx - sign * spec.head_w * 0.18,
                cy - spec.head_h * 0.78 - spec.horn_len * 0.55,
            )
            mid = (cx - sign * spec.head_w * 0.34, cy - spec.head_h * 0.50)
            base = (cx - sign * spec.head_w * 0.10, cy - spec.head_h * 0.34)
            d.polygon([tip, mid, base], fill=pal["cloak_hi"], outline=pal["outline"])

        hood_outer = _bbox((cx + 2 * S, cy + 2 * S), spec.hood_w * S, spec.hood_h * S)
        d.ellipse(hood_outer, fill=pal["outline"])
        hood_inner = _bbox(
            (cx + 2 * S, cy + 1 * S), (spec.hood_w - 4) * S, (spec.hood_h - 4) * S
        )
        d.ellipse(hood_inner, fill=pal["cloak_mid"])

        skull = _bbox((cx, cy - 1 * S), spec.head_w * S, spec.head_h * S)
        d.ellipse(skull, fill=pal["skin"], outline=pal["outline"], width=outline)
        d.ellipse(
            (skull[0] + 4 * S, skull[1] + 3 * S, skull[2] - 5 * S, cy - 2 * S),
            fill=with_alpha((255, 255, 255, 255), 48),
        )

        # multiple glowing eyes
        eye_y = cy - 4.5 * S
        centers = [
            (cx - 6.0 * S, eye_y),
            (cx + 1.0 * S, eye_y - 1.0 * S),
            (cx + 8.0 * S, eye_y + 1.0 * S),
        ]
        for ex, ey in centers:
            glow_r = spec.eye_r * S * (1.0 + eye_glow * 0.6)
            d.ellipse(
                _bbox((ex, ey), glow_r * 2.8, glow_r * 2.0),
                fill=with_alpha(pal["eye_soft"], int(42 + 90 * eye_glow)),
            )
            d.ellipse(
                _bbox((ex, ey), glow_r * 1.3, glow_r * 1.6),
                fill=pal["eye"],
                outline=pal["outline"],
            )
        if dead:
            d.line(
                [(cx - 9 * S, eye_y - 5 * S), (cx + 11 * S, eye_y + 5 * S)],
                fill=pal["energy3"],
                width=max(1, int(1.4 * S)),
            )
            d.line(
                [(cx - 9 * S, eye_y + 5 * S), (cx + 11 * S, eye_y - 5 * S)],
                fill=pal["energy3"],
                width=max(1, int(1.4 * S)),
            )

        # mouth / jaw
        mouth_c = (cx + 2 * S, cy + 8.5 * S)
        mh = (5.0 + jaw_open * 10.0) * S
        d.rounded_rectangle(
            (
                mouth_c[0] - 10 * S,
                mouth_c[1] - mh * 0.45,
                mouth_c[0] + 11 * S,
                mouth_c[1] + mh * 0.55,
            ),
            radius=5 * S,
            fill=pal["mouth"],
            outline=pal["outline"],
            width=max(1, int(1.0 * S)),
        )
        for tx in (-7.0, -1.0, 5.0):
            d.polygon(
                [
                    (mouth_c[0] + tx * S, mouth_c[1] - mh * 0.42),
                    (mouth_c[0] + (tx + 2.5) * S, mouth_c[1] - mh * 0.05),
                    (mouth_c[0] + (tx + 5.0) * S, mouth_c[1] - mh * 0.42),
                ],
                fill=pal["tooth"],
                outline=pal["outline"],
            )

        _paste_rotated_local(img, layer, center, angle)

    def _draw_tendril(
        self,
        img: Image.Image,
        d: ImageDraw.ImageDraw,
        shoulder: Point,
        a1: float,
        a2: float,
        tint: Color,
        spec: ZetaSpec,
        pal: Dict[str, Color],
        S: float,
        outline: float,
    ) -> Point:
        elbow = add(shoulder, vec(spec.tendril_upper * S, a1))
        hand = add(elbow, vec(spec.tendril_lower * S, a2))
        draw_capsule(d, shoulder, elbow, 3.6 * S, tint, pal["outline"], outline * 0.70)
        draw_capsule(d, elbow, hand, 2.8 * S, tint, pal["outline"], outline * 0.70)
        d.ellipse(
            (
                hand[0] - spec.claw_r * S,
                hand[1] - spec.claw_r * S,
                hand[0] + spec.claw_r * S,
                hand[1] + spec.claw_r * S,
            ),
            fill=pal["energy2"],
            outline=pal["outline"],
            width=max(1, int(outline * 0.65)),
        )
        return hand

    def _draw_cloak(
        self,
        img: Image.Image,
        center: Point,
        spec: ZetaSpec,
        pal: Dict[str, Color],
        S: float,
        angle: float,
        flare: float,
        collapse: float,
    ) -> None:
        pad = int(math.ceil(58 * S))
        layer = Image.new("RGBA", (pad * 2, pad * 2), (0, 0, 0, 0))
        d = ImageDraw.Draw(layer)
        cx, cy = float(pad), float(pad)
        w = spec.cloak_w * S * (1.0 + flare * 0.22 - collapse * 0.10)
        h = spec.cloak_h * S * (1.0 - collapse * 0.08)
        pts = [
            (cx - w * 0.18, cy - h * 0.48),
            (cx + w * 0.18, cy - h * 0.50),
            (cx + w * 0.40, cy - h * 0.15),
            (cx + w * 0.48, cy + h * 0.22),
            (cx + w * 0.30, cy + h * 0.48),
            (cx + w * 0.10, cy + h * 0.40),
            (cx, cy + h * 0.56),
            (cx - w * 0.12, cy + h * 0.42),
            (cx - w * 0.34, cy + h * 0.54),
            (cx - w * 0.50, cy + h * 0.18),
            (cx - w * 0.40, cy - h * 0.18),
        ]
        d.polygon(pts, fill=pal["outline"])
        inner = [(x * 0.96 + cx * 0.04, y * 0.96 + cy * 0.04) for x, y in pts]
        d.polygon(inner, fill=pal["cloak"])
        d.polygon(
            [
                (cx - w * 0.10, cy - h * 0.18),
                (cx + w * 0.18, cy - h * 0.22),
                (cx + w * 0.26, cy + h * 0.28),
                (cx - w * 0.08, cy + h * 0.18),
            ],
            fill=with_alpha(pal["cloak_hi"], 120),
        )
        tail = [
            (cx - w * 0.10, cy + h * 0.35),
            (cx - w * 0.24, cy + h * 0.68),
            (cx - w * 0.06, cy + h * 0.48),
        ]
        d.polygon(tail, fill=pal["cloak_mid"], outline=pal["outline"])
        _paste_rotated_local(img, layer, center, angle)

    def _draw_spit_fx(
        self,
        img: Image.Image,
        mouth: Point,
        direction: float,
        strength: float,
        S: float,
    ) -> None:
        d = ImageDraw.Draw(img)
        if strength <= 0:
            return
        tip = add(mouth, vec((14 + 18 * strength) * S, direction))
        d.line(
            [mouth, tip],
            fill=with_alpha(self.PALETTE["energy2"], 190),
            width=max(1, int(2.2 * S)),
        )
        d.ellipse(
            _bbox(tip, 7 * S, 7 * S),
            fill=with_alpha(self.PALETTE["energy"], 220),
            outline=self.PALETTE["outline"],
        )
        d.ellipse(
            _bbox((tip[0] + 4 * S, tip[1]), 4 * S, 4 * S),
            fill=with_alpha(self.PALETTE["energy3"], 180),
        )

    def _draw_beam_fx(
        self, img: Image.Image, mouth: Point, charge: float, fire: float, S: float
    ) -> None:
        d = ImageDraw.Draw(img)
        if charge > 0:
            for r, a in [(10, 70), (16, 60), (22, 48)]:
                d.ellipse(
                    _bbox(mouth, (r + charge * 8) * S, (r + charge * 8) * S),
                    outline=with_alpha(self.PALETTE["energy2"], int(a + charge * 70)),
                    width=max(1, int(1.6 * S)),
                )
        if fire > 0:
            x2 = mouth[0] + (48 + 40 * fire) * S
            d.line(
                [mouth, (x2, mouth[1] - 2 * S)],
                fill=with_alpha(self.PALETTE["energy3"], 110),
                width=max(1, int(11 * S)),
            )
            d.line(
                [mouth, (x2, mouth[1] - 2 * S)],
                fill=with_alpha(self.PALETTE["energy2"], 160),
                width=max(1, int(7 * S)),
            )
            d.line(
                [mouth, (x2, mouth[1] - 2 * S)],
                fill=with_alpha(self.PALETTE["eye_soft"], 230),
                width=max(1, int(3 * S)),
            )

    def _draw_slam_fx(
        self, img: Image.Image, root_x: float, ground_y: float, amount: float, S: float
    ) -> None:
        if amount <= 0:
            return
        d = ImageDraw.Draw(img)
        rad = (16 + 28 * amount) * S
        d.arc(
            (root_x - rad, ground_y - 16 * S, root_x + rad, ground_y + 16 * S),
            180,
            360,
            fill=with_alpha(self.PALETTE["energy3"], 180),
            width=max(1, int(2.5 * S)),
        )
        for sign in (-1, 1):
            d.line(
                [
                    (root_x + sign * 10 * S, ground_y - 2 * S),
                    (
                        root_x + sign * (18 + 18 * amount) * S,
                        ground_y - (8 + 10 * amount) * S,
                    ),
                ],
                fill=with_alpha(self.PALETTE["energy2"], 150),
                width=max(1, int(1.8 * S)),
            )

    def _draw_summon_fx(
        self, img: Image.Image, root_x: float, center_y: float, amount: float, S: float
    ) -> None:
        if amount <= 0:
            return
        d = ImageDraw.Draw(img)
        for sign in (-1, 1):
            orb = (
                root_x + sign * (18 + 8 * amount) * S,
                center_y + (8 - 18 * amount) * S,
            )
            d.ellipse(
                _bbox(orb, (9 + 8 * amount) * S, (9 + 8 * amount) * S),
                fill=with_alpha(self.PALETTE["energy"], 140),
                outline=self.PALETTE["outline"],
            )
            for i in range(3):
                d.arc(
                    (
                        orb[0] - (13 + i * 6) * S,
                        orb[1] - (13 + i * 6) * S,
                        orb[0] + (13 + i * 6) * S,
                        orb[1] + (13 + i * 6) * S,
                    ),
                    -40,
                    70,
                    fill=with_alpha(self.PALETTE["energy3"], int(90 - i * 18)),
                    width=max(1, int(1.4 * S)),
                )

    def _draw_side_sweep_fx(
        self, img: Image.Image, hand: Point, root_x: float, amount: float, S: float
    ) -> None:
        if amount <= 0:
            return
        d = ImageDraw.Draw(img)
        alpha = int(70 + 120 * amount)
        # Wide horizontal claw arc matching BossAttackKind::SideSweep.
        box = (
            root_x - 4 * S,
            hand[1] - (28 + 10 * amount) * S,
            root_x + (78 + 22 * amount) * S,
            hand[1] + (24 + 8 * amount) * S,
        )
        d.arc(
            box,
            start=-34,
            end=42,
            fill=with_alpha(self.PALETTE["energy2"], alpha),
            width=max(1, int(3.8 * S)),
        )
        d.arc(
            (box[0] + 5 * S, box[1] + 6 * S, box[2] - 5 * S, box[3] - 3 * S),
            start=-28,
            end=36,
            fill=with_alpha(self.PALETTE["energy3"], int(alpha * 0.72)),
            width=max(1, int(1.8 * S)),
        )
        for i in range(3):
            x = hand[0] + (10 + i * 14) * S
            y = hand[1] - (4 - i * 3) * S
            d.line(
                [(x, y), (x + (18 + 4 * i) * S, y - (8 - 2 * i) * S)],
                fill=with_alpha(self.PALETTE["eye_soft"], int(alpha * 0.58)),
                width=max(1, int(1.2 * S)),
            )

    def _draw_spike_halo_fx(
        self, img: Image.Image, root_x: float, center_y: float, amount: float, S: float
    ) -> None:
        if amount <= 0:
            return
        d = ImageDraw.Draw(img)
        radius = (20 + 18 * amount) * S
        cx, cy = root_x + 2 * S, center_y - 10 * S
        for i in range(12):
            theta = (math.tau * i / 12.0) + amount * 0.8
            inner = (
                cx + math.cos(theta) * radius * 0.55,
                cy + math.sin(theta) * radius * 0.55,
            )
            outer = (cx + math.cos(theta) * radius, cy + math.sin(theta) * radius)
            side1 = (
                cx + math.cos(theta + 0.12) * radius * 0.73,
                cy + math.sin(theta + 0.12) * radius * 0.73,
            )
            side2 = (
                cx + math.cos(theta - 0.12) * radius * 0.73,
                cy + math.sin(theta - 0.12) * radius * 0.73,
            )
            d.polygon(
                [inner, side1, outer, side2],
                fill=with_alpha(self.PALETTE["energy2"], int(72 + 96 * amount)),
                outline=with_alpha(self.PALETTE["outline"], 180),
            )
        for rscale, alpha in [(0.9, 120), (1.16, 78), (1.42, 42)]:
            rr = radius * rscale
            d.ellipse(
                (cx - rr, cy - rr, cx + rr, cy + rr),
                outline=with_alpha(self.PALETTE["energy3"], int(alpha * amount)),
                width=max(1, int(1.3 * S)),
            )

    def _draw_dash_echo_fx(
        self, img: Image.Image, root_x: float, ground_y: float, amount: float, S: float
    ) -> None:
        if amount <= 0:
            return
        d = ImageDraw.Draw(img)
        # Horizontal echo streaks: the body itself draws later, these imply a fast dash clone.
        for i in range(5):
            y = ground_y - (25 + i * 9) * S
            x1 = root_x - (42 + i * 4) * S * amount
            x2 = root_x + (4 + i * 3) * S
            alpha = int((112 - i * 14) * amount)
            d.line(
                [(x1, y), (x2, y - 2 * S)],
                fill=with_alpha(self.PALETTE["energy3"], alpha),
                width=max(1, int((2.4 - i * 0.22) * S)),
            )
            d.line(
                [(x1 + 12 * S, y + 3 * S), (x2 + 18 * S, y + 1 * S)],
                fill=with_alpha(self.PALETTE["energy2"], max(0, alpha - 28)),
                width=max(1, int(1.2 * S)),
            )

    def _render_highres(
        self,
        spec: ZetaSpec,
        animation: str,
        frame_index: int,
        frame_count: int,
        size: Tuple[int, int],
        background: Optional[Color],
        scale: int,
    ) -> Image.Image:
        W, H = size[0] * scale, size[1] * scale
        bg = (0, 0, 0, 0) if background is None else background
        img = Image.new("RGBA", (W, H), bg)
        # Scale the 128-base character to the requested frame width so a
        # render_scale>1 canvas draws the SAME character with more native
        # pixels (matches the toon generator's S=(W/128)*ss). Identical at the
        # 128 default; only render_scale>1 changes it.
        S = float(scale) * (size[0] / 128.0)
        pal = self.PALETTE
        p = self.pose_for_animation(animation, frame_index, frame_count)
        ground_y = (103.0 + p.root_y) * S
        root_x = (60.0 + p.root_x) * S
        outline = 1.8 * S

        # Ground shadow removed; the in-game renderer composites bosses
        # over floor geometry that already provides ground contact.
        if animation == "dash_echo":
            self._draw_dash_echo_fx(img, root_x, ground_y, p.dash_echo, S)

        character_img = Image.new("RGBA", (W, H), (0, 0, 0, 0))
        character_draw = ImageDraw.Draw(character_img)

        collapse = p.collapse
        body_center = (
            root_x + lerp(0, 14 * S, collapse),
            ground_y - lerp(54 * S, 22 * S, collapse) - p.hover * S,
        )
        head_center = (
            body_center[0] + lerp(5 * S, 20 * S, collapse),
            body_center[1] - lerp(22 * S, 8 * S, collapse),
        )
        shoulder_far = (body_center[0] - 15 * S, body_center[1] - 8 * S)
        shoulder_near = (body_center[0] + 15 * S, body_center[1] - 7 * S)
        body_angle = p.body_tilt
        head_angle = p.head_tilt

        # far tendril first
        self._draw_tendril(
            character_img,
            character_draw,
            shoulder_far,
            p.far_arm_upper,
            p.far_arm_lower,
            pal["cloak_mid"],
            spec,
            pal,
            S,
            outline,
        )

        # cloak body then rigid head
        self._draw_cloak(
            character_img,
            body_center,
            spec,
            pal,
            S,
            body_angle,
            p.cloak_flare,
            p.collapse,
        )
        # core / chest eye
        core_center = (body_center[0] + 2 * S, body_center[1] - 1 * S)
        draw_rotated_ellipse(
            character_img,
            core_center,
            (15 * S, 11 * S),
            body_angle,
            with_alpha(pal["eye_soft"], int(60 + p.eye_glow * 90)),
            None,
            0,
        )
        draw_rotated_ellipse(
            character_img,
            core_center,
            (8 * S, 9 * S),
            body_angle,
            pal["eye"],
            pal["outline"],
            max(1, int(1.0 * S)),
        )
        self._draw_head(
            character_img,
            head_center,
            spec,
            pal,
            S,
            head_angle,
            p.jaw_open,
            p.eye_glow,
            p.dead,
        )

        # near tendril on top
        near_hand = self._draw_tendril(
            character_img,
            character_draw,
            shoulder_near,
            p.near_arm_upper,
            p.near_arm_lower,
            pal["cloak_hi"],
            spec,
            pal,
            S,
            outline,
        )

        mouth = (head_center[0] + 11 * S, head_center[1] + 6 * S)
        self._draw_side_sweep_fx(character_img, near_hand, root_x, p.side_sweep, S)
        self._draw_beam_fx(character_img, mouth, p.beam_charge, p.beam_fire, S)
        self._draw_slam_fx(character_img, root_x, ground_y, p.slam, S)
        self._draw_spike_halo_fx(character_img, root_x, body_center[1], p.spike_halo, S)

        # eerie hand trails during hover / summon
        if p.summon > 0 or animation == "hover":
            for hand, alpha in [
                (near_hand, 110),
                ((shoulder_far[0] - 8 * S, shoulder_far[1] + 14 * S), 80),
            ]:
                ImageDraw.Draw(character_img).arc(
                    (
                        hand[0] - 12 * S,
                        hand[1] - 12 * S,
                        hand[0] + 12 * S,
                        hand[1] + 12 * S,
                    ),
                    -30,
                    80,
                    fill=with_alpha(pal["energy3"], alpha),
                    width=max(1, int(1.3 * S)),
                )

        img.alpha_composite(character_img)
        return img

    def render_animation_frame(
        self,
        spec: ZetaSpec,
        animation: str,
        frame_index: int,
        frame_count: int,
        size: Tuple[int, int] = (128, 128),
        background: Optional[Color] = None,
        supersample: int = 4,
        downsample: str = "lanczos",
    ) -> Image.Image:
        high = self._render_highres(
            spec,
            animation,
            frame_index,
            frame_count,
            size,
            background,
            max(1, int(supersample)),
        )
        resample = RESAMPLING.NEAREST if downsample == "nearest" else RESAMPLING.LANCZOS
        return high.resize(size, resample)
