from __future__ import annotations

"""Cute right-facing side-scroller robot target.

The renderer keeps a fixed canvas, fixed ground anchor, and stable part sizes for
every animation.  The ``blink_out`` and ``blink_in`` rows are the Ambition
teleport / precision-blink ability split into source and destination phases, not
an eyelid blink.  Eyelid blinks remain as incidental idle acting.

The robot head is drawn as a rigid local layer and rotated as one unit so the
visor, antenna, face, and shell keep their spatial relationship.  For a
right-facing model, the far arm is drawn behind the body and the near arm / blade
is drawn in front.
"""

import math
import random
from typing import Dict, Optional, Tuple

from PIL import Image, ImageDraw
from ambition_sprite2d_renderer.core.draw import bbox_from_center as _bbox

from ...authoring.common_draw import RESAMPLING, draw_capsule, draw_rotated_rounded_rect
from .robot25d import BotSpec, Pose, parse_background
from ...authoring.animation_vocab import (
    DEFAULT_ADVANCED_TIMINGS,
    DEFAULT_DIRECTIONAL_ATTACK_TIMINGS,
    DEFAULT_EXTENDED_TIMINGS,
    DEFAULT_TRAVERSAL_POLISH_TIMINGS,
)
from ...authoring.rig import add, clamp, ease_in_out_sine, ease_out_cubic, lerp, smoothstep, vec

Color = Tuple[int, int, int, int]
Point = Tuple[float, float]


def _rgba(hex_color: str, alpha: int = 255) -> Color:
    from PIL import ImageColor

    r, g, b = ImageColor.getrgb(hex_color)
    return (r, g, b, alpha)


def _with_alpha(color: Color, alpha: int) -> Color:
    return (color[0], color[1], color[2], alpha)




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
        # Ambition blink ability split into source/departure and destination/arrival.
        "blink_out": {"frames": 6, "duration_ms": 62},
        "blink_in": {"frames": 6, "duration_ms": 62},
        "dash": {"frames": 6, "duration_ms": 65},
        **DEFAULT_EXTENDED_TIMINGS,
        **DEFAULT_ADVANCED_TIMINGS,
        **DEFAULT_TRAVERSAL_POLISH_TIMINGS,
        **DEFAULT_DIRECTIONAL_ATTACK_TIMINGS,
        # Player primary melee. A 3-frame CONTINUOUS SWEEP: the body holds the
        # committed lunge and the blade arcs through its hitbox across all three
        # frames (slash_arc = t), so the slash + hitbox are live the whole swing
        # (active_frames=[0,1,2]) with no windup/recover beat. Authored feel —
        # verify with `python -m ambition_sprite2d_renderer debug-hitboxes
        # player_robot`. Enemies use `slash` (8f) via pick_enemy_anim, so this
        # only changes the player's read.
        "attack_side": {"frames": 3, "duration_ms": 60},
        # The rest of the directional/aerial attacks share the 3-frame sweep:
        # the committed pose is held and the blade sweeps (slash_arc = t) with
        # the hitbox live across all frames.
        "attack_up": {"frames": 3, "duration_ms": 60},
        "attack_down": {"frames": 3, "duration_ms": 60},
        "air_neutral": {"frames": 3, "duration_ms": 60},
        "air_forward": {"frames": 3, "duration_ms": 60},
        "air_back": {"frames": 3, "duration_ms": 60},
        "air_down": {"frames": 3, "duration_ms": 60},
        "air_up": {"frames": 3, "duration_ms": 60},
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
        key = str(archetype or "").lower()
        if archetype in {"guardian", "heavy_guardian"} or any(token in key for token in ["marshal", "iron", "warden"]):
            scale = 1.08
        elif archetype in {"runner", "scout_runner"} or any(token in key for token in ["pulse", "captain"]):
            scale = 0.96
        elif archetype in {"diver", "swimmer"}:
            scale = 0.99
        elif archetype in {"caster", "radio_mage"}:
            scale = 1.01
        elif archetype in {"engineer", "field_mechanic"}:
            scale = 1.02
        # Player-specific compact silhouette. Disproportionately shrinks
        # limbs + lowers body anchors so the rendered character matches
        # the gameplay collider (30×48 standing). Keep this archetype
        # exclusive to the player sheet — other characters share the
        # `cute_scout` proportions on the runtime robot sheet.
        player_compact = archetype == "player_compact" or "player_compact" in key
        limb_scale = 1.0
        leg_scale = 1.0
        vertical_scale = 1.0
        if player_compact:
            scale = 0.88
            limb_scale = 0.82
            leg_scale = 0.72
            vertical_scale = 0.78
        return BotSpec(
            target=self.name,
            seed=seed,
            archetype=archetype,
            palette_name=archetype,
            head_w=(41 + rng.uniform(-1.0, 1.0)) * scale,
            head_h=(34 + rng.uniform(-1.0, 1.0)) * scale,
            body_w=(26 + rng.uniform(-0.8, 0.8)) * scale,
            body_h=(25 + rng.uniform(-0.8, 0.8)) * scale * (0.90 if player_compact else 1.0),
            arm_upper=(13.6 + rng.uniform(-0.4, 0.7)) * scale * limb_scale,
            arm_lower=(11.5 + rng.uniform(-0.4, 0.5)) * scale * limb_scale,
            leg_upper=(13.4 + rng.uniform(-0.4, 0.6)) * scale * leg_scale,
            leg_lower=(12.0 + rng.uniform(-0.4, 0.6)) * scale * leg_scale,
            visor_w=(23.5 + rng.uniform(-0.6, 0.6)) * scale,
            visor_h=(12.0 + rng.uniform(-0.4, 0.4)) * scale,
            antenna_h=(12.0 + rng.uniform(-0.8, 0.8)) * scale,
            blade_len=(30.0 + rng.uniform(-1.0, 2.0)) * scale * (1.12 if (archetype in {"guardian", "heavy_guardian"} or any(token in key for token in ["marshal", "iron", "warden"])) else 0.94 if (archetype in {"runner", "scout_runner"} or any(token in key for token in ["pulse", "captain"])) else 1.0),
            vertical_scale=vertical_scale,
        )

    def _palette_for_spec(self, spec: BotSpec) -> Dict[str, Color]:
        pal = dict(self.PALETTE)
        name = str(spec.palette_name or spec.archetype or "").lower()
        if any(token in name for token in ["drift", "dj", "lofi", "radio"]):
            pal["accent"] = _rgba("#FF86D7")
            pal["accent_dark"] = _rgba("#9542B8")
            pal["visor_glow"] = _rgba("#FFE36E")
            pal["shell_side"] = _rgba("#E9E1F1")
        elif any(token in name for token in ["pulse", "captain", "voyage"]):
            pal["accent"] = _rgba("#58D6FF")
            pal["accent_dark"] = _rgba("#1A78B3")
            pal["visor_glow"] = _rgba("#B6FFF5")
            pal["shell_side"] = _rgba("#E0EEF5")
        elif any(token in name for token in ["tech", "disrupt"]):
            pal["accent"] = _rgba("#9FE66A")
            pal["accent_dark"] = _rgba("#4D9D43")
            pal["visor_glow"] = _rgba("#86FF7A")
            pal["shell_side"] = _rgba("#E7F0DD")
        elif any(token in name for token in ["dino", "saur", "liberator"]):
            pal["accent"] = _rgba("#A8F05E")
            pal["accent_dark"] = _rgba("#5B8F31")
            pal["visor_glow"] = _rgba("#FFF18A")
            pal["shell_side"] = _rgba("#E9F1D7")
        elif any(token in name for token in ["env", "advocate", "solace"]):
            pal["accent"] = _rgba("#38E983")
            pal["accent_dark"] = _rgba("#1C9C60")
            pal["visor_glow"] = _rgba("#D8FFE5")
            pal["shell_side"] = _rgba("#DFF0E4")
        elif any(token in name for token in ["iron", "marshal", "military"]):
            pal["accent"] = _rgba("#FF7059")
            pal["accent_dark"] = _rgba("#983729")
            pal["visor_glow"] = _rgba("#FFD0C7")
            pal["metal"] = _rgba("#CBD0D7")
        elif any(token in name for token in ["moonlit", "canal", "noct"]):
            pal["accent"] = _rgba("#8D7CFF")
            pal["accent_dark"] = _rgba("#3D367F")
            pal["visor_glow"] = _rgba("#B6FFF5")
            pal["shell_side"] = _rgba("#DDE6F8")
        elif any(token in name for token in ["glass", "warden", "canopy"]):
            pal["accent"] = _rgba("#83BDFF")
            pal["accent_dark"] = _rgba("#4567B0")
            pal["visor_glow"] = _rgba("#E7FFFF")
            pal["shell_side"] = _rgba("#EAF4FF")
        elif name in {"runner", "scout_runner"}:
            pal["accent"] = _rgba("#FFB15E")
            pal["accent_dark"] = _rgba("#D66A2D")
            pal["visor_glow"] = _rgba("#86FF7A")
            pal["shell_side"] = _rgba("#DED8CF")
        elif name in {"guardian", "heavy_guardian"}:
            pal["accent"] = _rgba("#7EA7FF")
            pal["accent_dark"] = _rgba("#4D69C9")
            pal["visor_glow"] = _rgba("#7DE7FF")
            pal["metal"] = _rgba("#C0C6D1")
        elif name in {"diver", "swimmer"}:
            pal["accent"] = _rgba("#58D6FF")
            pal["accent_dark"] = _rgba("#2387B8")
            pal["visor_glow"] = _rgba("#B6FFF5")
            pal["shell_side"] = _rgba("#DDEBF0")
        elif name in {"caster", "radio_mage"}:
            pal["accent"] = _rgba("#FF86D7")
            pal["accent_dark"] = _rgba("#B957A7")
            pal["visor_glow"] = _rgba("#FFE36E")
        elif name in {"engineer", "field_mechanic"}:
            pal["accent"] = _rgba("#9FE66A")
            pal["accent_dark"] = _rgba("#5EA83B")
            pal["visor_glow"] = _rgba("#56F1B7")
        elif name in {"medic", "field_medic"}:
            pal["accent"] = _rgba("#38E983")
            pal["accent_dark"] = _rgba("#1D9C59")
            pal["visor_glow"] = _rgba("#D9FFF0")
            pal["shell_side"] = _rgba("#E7F1E7")
        elif name in {"miner", "cavern_miner"}:
            pal["accent"] = _rgba("#FFD65A")
            pal["accent_dark"] = _rgba("#B88922")
            pal["visor_glow"] = _rgba("#FFEFA7")
            pal["shell_side"] = _rgba("#D8D0C1")
        elif name in {"archivist", "map_keeper"}:
            pal["accent"] = _rgba("#83BDFF")
            pal["accent_dark"] = _rgba("#4567B0")
            pal["visor_glow"] = _rgba("#D9EFFF")
            pal["shell_side"] = _rgba("#E7E7F3")
        return pal

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
        elif animation == "blink_out":
            # Source/departure phase: compress, brace, then shear into the portal.
            charge = smoothstep(clamp(t / 0.46, 0.0, 1.0))
            burst = smoothstep(clamp((t - 0.30) / 0.48, 0.0, 1.0))
            pulse = math.sin(t * math.pi)
            p.root_x = -2.4 * charge - 2.0 * burst
            p.root_y = 1.6 * charge - 2.2 * burst
            p.body_bob = -1.3 * charge + 0.25 * pulse
            p.body_tilt = -16.0 * charge - 12.0 * burst
            p.head_tilt = -9.0 * charge - 4.0 * burst
            p.far_arm_upper = 168.0 + 18.0 * charge
            p.far_arm_lower = 148.0 + 30.0 * burst
            p.near_arm_upper = 8.0 - 22.0 * charge
            p.near_arm_lower = 4.0 - 20.0 * burst
            p.far_leg_upper = 124.0 + 20.0 * charge
            p.far_leg_lower = 72.0 + 20.0 * charge
            p.near_leg_upper = 102.0 + 18.0 * charge
            p.near_leg_lower = 64.0 + 14.0 * charge
            p.eye_squint = 0.20 + 0.16 * pulse + 0.14 * burst
        elif animation == "blink_in":
            # Destination/arrival phase: assemble out of the portal into a low landing.
            appear = smoothstep(clamp(t / 0.60, 0.0, 1.0))
            settle = ease_out_cubic(appear)
            recoil = 1.0 - settle
            pulse = math.sin(t * math.pi)
            p.root_x = 5.4 * recoil
            p.root_y = 2.2 * recoil - 1.8 * pulse * recoil
            p.body_bob = -1.0 * recoil + 0.2 * pulse
            p.body_tilt = 18.0 * recoil - 2.0 * settle
            p.head_tilt = 9.0 * recoil - 1.0 * settle
            p.far_arm_upper = 188.0 - 34.0 * settle
            p.far_arm_lower = 170.0 - 34.0 * settle
            p.near_arm_upper = 48.0 - 30.0 * settle
            p.near_arm_lower = 34.0 - 28.0 * settle
            p.far_leg_upper = 136.0 - 32.0 * settle
            p.far_leg_lower = 86.0 + 8.0 * recoil
            p.near_leg_upper = 110.0 - 26.0 * settle
            p.near_leg_lower = 80.0 + 10.0 * recoil
            p.eye_squint = 0.28 + 0.20 * recoil
        elif animation in {"walk", "run"}:
            # Stride amplitude bumped 2026-05-29 so the legs read
            # clearly across the 8-frame cycle. The previous amp=18/27
            # gave a ~36° hip swing; against the supersample-3 → 128px
            # downsample the alternating-foot read came across as a
            # shuffle. The new amp=34/52 lifts each foot well past the
            # standing pose. `lift_phase` adds a clear knee-bend on the
            # forward foot (lower-leg tucks up under the body) so the
            # silhouette has a visible "step" shape, not just a hip
            # sway. Body_bob + arm swing scale together.
            stride = math.sin(t * math.tau)
            bounce = (1.0 - math.cos(t * math.tau * 2.0)) * 0.5
            is_run = animation == "run"
            amp = 34.0 if not is_run else 52.0
            arm = 18.0 if not is_run else 28.0
            lean = -2.0 if not is_run else -9.0
            # The forward-foot knee tuck. `clamp(...)` so only the
            # rising-back leg lifts, not the planted one.
            forward_lift = max(0.0, stride)
            back_lift = max(0.0, -stride)
            knee_tuck = 30.0 if not is_run else 44.0
            heel_kick = 14.0 if not is_run else 22.0
            p.root_x = stride * (1.4 if not is_run else 2.4)
            p.body_bob = 0.5 + bounce * (2.2 if not is_run else 3.2)
            p.body_tilt = lean - stride * 4.5
            p.head_tilt = -bounce * 1.8 - stride * 0.6
            p.far_arm_upper = 145 + stride * arm
            p.far_arm_lower = 122 + stride * arm * 0.55
            p.near_arm_upper = 32 - stride * arm
            p.near_arm_lower = 20 - stride * arm * 0.55
            # Far leg = leg behind on positive stride, leg lifting on
            # negative stride; the lower-leg knee tuck is keyed off the
            # *lifting* phase so the bent knee always faces forward.
            p.far_leg_upper = 105 + stride * amp
            p.far_leg_lower = 97 - forward_lift * knee_tuck + back_lift * heel_kick
            p.near_leg_upper = 72 - stride * amp
            p.near_leg_lower = 85 - back_lift * knee_tuck + forward_lift * heel_kick
            p.eye_squint = 0.08 + bounce * 0.12
        elif animation == "jump":
            # Quick squat-to-spring jump. The old version started airborne,
            # which made the action read as a float. This keeps an obvious
            # crouch on the first frames, a push-off beat in the middle, then
            # a compact airborne silhouette.
            preload = 1.0 - smoothstep(clamp(t / 0.24, 0.0, 1.0))
            launch = smoothstep(clamp((t - 0.14) / 0.24, 0.0, 1.0))
            air = smoothstep(clamp((t - 0.34) / 0.32, 0.0, 1.0))
            tuck = smoothstep(clamp((t - 0.46) / 0.18, 0.0, 1.0)) * (1.0 - 0.60 * smoothstep(clamp((t - 0.82) / 0.18, 0.0, 1.0)))
            p.root_y = 5.0 * preload - 19.0 * air
            p.root_x = 0.6 + 2.2 * air
            p.body_tilt = -12.0 * preload - 3.0 * launch + 4.0 * air
            p.head_tilt = -5.0 * preload - 2.0 * air
            p.far_arm_upper = 164.0 - 14.0 * air
            p.far_arm_lower = 150.0 - 10.0 * air
            p.near_arm_upper = 14.0 + 12.0 * air
            p.near_arm_lower = 10.0 + 10.0 * air
            p.far_leg_upper = 88.0 - 12.0 * tuck
            p.far_leg_lower = 118.0 + 30.0 * tuck
            p.near_leg_upper = 74.0 - 10.0 * tuck
            p.near_leg_lower = 104.0 + 24.0 * tuck
            p.eye_squint = 0.08 + 0.05 * air
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
        elif animation == "crouch":
            crouch = smoothstep(clamp(math.sin(t * math.pi), 0.0, 1.0))
            p.root_y = 5.0 * crouch
            p.body_bob = 3.0 * crouch
            p.body_tilt = -7.0 * crouch
            p.head_tilt = -4.0 * crouch
            p.far_arm_upper = 158.0
            p.far_arm_lower = 144.0
            p.near_arm_upper = 20.0
            p.near_arm_lower = 8.0
            p.far_leg_upper = 132.0
            p.far_leg_lower = 52.0
            p.near_leg_upper = 96.0
            p.near_leg_lower = 48.0
            p.eye_squint = 0.18
        elif animation == "wall_slide":
            jitter = math.sin(t * math.tau * 2.0)
            p.root_x = -7.5
            p.root_y = 3.0 + t * 4.0
            p.body_tilt = 12.0 + jitter * 1.2
            p.head_tilt = 6.0
            p.far_arm_upper = 200.0
            p.far_arm_lower = 186.0
            p.near_arm_upper = -18.0
            p.near_arm_lower = -26.0
            p.far_leg_upper = 118.0
            p.far_leg_lower = 122.0
            p.near_leg_upper = 72.0
            p.near_leg_lower = 118.0
            p.eye_squint = 0.24
        elif animation == "wall_jump":
            kick = smoothstep(clamp(t / 0.55, 0.0, 1.0))
            arc = math.sin(t * math.pi)
            p.root_x = -8.0 + 16.0 * kick
            p.root_y = -18.0 * arc
            p.body_tilt = -18.0 + 34.0 * kick
            p.head_tilt = -8.0 + 12.0 * kick
            p.far_arm_upper = 178.0 - 18.0 * kick
            p.far_arm_lower = 160.0 - 14.0 * kick
            p.near_arm_upper = -12.0 + 32.0 * kick
            p.near_arm_lower = -20.0 + 28.0 * kick
            p.far_leg_upper = 145.0 - 42.0 * kick
            p.far_leg_lower = 72.0 + 28.0 * kick
            p.near_leg_upper = 100.0 - 20.0 * kick
            p.near_leg_lower = 54.0 + 36.0 * kick
            p.eye_squint = 0.22
        elif animation == "ledge_grab":
            # Held hang loop. The runtime snaps instantly to the
            # climb target on Up+Jump, so this animation is a
            # steady-state dangle, not a pull-up transition. Both
            # arms reach overhead gripping the ledge; the body
            # slumps below with a subtle weight bob + leg sway.
            grip = math.sin(t * math.tau)
            p.root_x = -2.0
            p.root_y = -8.0 + grip * 1.4
            p.body_tilt = -9.0 + grip * 1.5
            p.head_tilt = -5.0 + grip * 1.0
            p.far_arm_upper = 200.0
            p.far_arm_lower = 208.0
            p.near_arm_upper = -52.0 - grip * 2.0
            p.near_arm_lower = -56.0 - grip * 2.0
            p.far_leg_upper = 122.0 + grip * 4.0
            p.far_leg_lower = 92.0 - grip * 3.0
            p.near_leg_upper = 92.0 - grip * 4.0
            p.near_leg_lower = 76.0 + grip * 3.0
            p.eye_squint = 0.22
        elif animation == "climb":
            stride = math.sin(t * math.tau)
            p.root_y = -2.0 + stride * 1.2
            p.body_tilt = stride * 5.0
            p.head_tilt = -stride * 2.0
            p.far_arm_upper = 194.0 + stride * 18.0
            p.far_arm_lower = 180.0 + stride * 18.0
            p.near_arm_upper = -24.0 - stride * 18.0
            p.near_arm_lower = -8.0 - stride * 16.0
            p.far_leg_upper = 118.0 - stride * 28.0
            p.far_leg_lower = 72.0 - stride * 20.0
            p.near_leg_upper = 78.0 + stride * 28.0
            p.near_leg_lower = 64.0 + stride * 20.0
            p.eye_squint = 0.10
        elif animation == "swim":
            stroke = math.sin(t * math.tau)
            p.root_y = -10.0 + math.sin(t * math.tau * 2.0) * 1.4
            p.body_tilt = -27.0 + stroke * 3.0
            p.head_tilt = -7.0 + stroke * 2.0
            p.far_arm_upper = 190.0 + stroke * 24.0
            p.far_arm_lower = 175.0 + stroke * 20.0
            p.near_arm_upper = 8.0 - stroke * 30.0
            p.near_arm_lower = -12.0 - stroke * 24.0
            p.far_leg_upper = 148.0 - stroke * 14.0
            p.far_leg_lower = 150.0 + stroke * 18.0
            p.near_leg_upper = 124.0 + stroke * 14.0
            p.near_leg_lower = 132.0 - stroke * 18.0
            p.eye_squint = 0.16
        elif animation == "interact":
            reach = smoothstep(clamp(math.sin(t * math.pi), 0.0, 1.0))
            p.root_x = 1.8 * reach
            p.body_tilt = -2.0 + 3.0 * reach
            p.head_tilt = 2.0 * reach
            p.far_arm_upper = 148.0
            p.far_arm_lower = 132.0
            p.near_arm_upper = 0.0 + 12.0 * reach
            p.near_arm_lower = -8.0 + 8.0 * reach
            p.near_leg_upper = 76.0
            p.near_leg_lower = 82.0
            p.eye_squint = 0.08
        elif animation == "talk":
            mouth_like = 0.5 + 0.5 * math.sin(t * math.tau * 2.0)
            p.body_bob = abs(wave) * 0.8
            p.body_tilt = wave * 1.0
            p.head_tilt = 2.5 * wave
            p.far_arm_upper = 142.0 + wave * 5.0
            p.near_arm_upper = 28.0 - wave * 5.0
            p.eye_squint = 0.06 + 0.12 * mouth_like
        elif animation == "block":
            brace = smoothstep(clamp(t / 0.30, 0.0, 1.0)) * (1.0 - 0.25 * smoothstep(clamp((t - 0.65) / 0.35, 0.0, 1.0)))
            p.root_x = -2.0 * brace
            p.body_tilt = -10.0 * brace
            p.head_tilt = -3.0 * brace
            p.far_arm_upper = 168.0
            p.far_arm_lower = 150.0
            p.near_arm_upper = -30.0 + 8.0 * wave
            p.near_arm_lower = -38.0 + 6.0 * wave
            p.far_leg_upper = 122.0
            p.far_leg_lower = 86.0
            p.near_leg_upper = 82.0
            p.near_leg_lower = 72.0
            p.eye_squint = 0.26
        elif animation == "land":
            impact = 1.0 - smoothstep(clamp(t / 0.72, 0.0, 1.0))
            rebound = math.sin(t * math.pi)
            p.root_y = 7.0 * impact - 2.0 * rebound
            p.body_bob = 4.0 * impact
            p.body_tilt = -6.0 * impact
            p.head_tilt = -4.0 * impact
            p.far_arm_upper = 166.0 - 12.0 * rebound
            p.far_arm_lower = 150.0 - 8.0 * rebound
            p.near_arm_upper = 16.0 + 16.0 * rebound
            p.near_arm_lower = 8.0 + 12.0 * rebound
            p.far_leg_upper = 134.0
            p.far_leg_lower = 54.0
            p.near_leg_upper = 94.0
            p.near_leg_lower = 48.0
            p.eye_squint = 0.22 * impact
        elif animation == "roll":
            crouch = smoothstep(clamp(t / 0.14, 0.0, 1.0))
            recover = smoothstep(clamp((t - 0.78) / 0.22, 0.0, 1.0))
            spin = smoothstep(clamp((t - 0.08) / 0.80, 0.0, 1.0))
            tuck = smoothstep(clamp((t - 0.08) / 0.18, 0.0, 1.0)) * (1.0 - 0.35 * recover)
            p.root_x = -7.0 + 20.0 * smoothstep(t)
            p.root_y = 5.0 + 3.0 * math.sin(t * math.pi) - 2.0 * tuck + 1.6 * recover
            p.body_tilt = -16.0 * crouch + 8.0 * recover
            p.head_tilt = -8.0 * crouch + 4.0 * recover
            p.far_arm_upper = 168.0 + 24.0 * tuck - 10.0 * recover
            p.far_arm_lower = 152.0 + 28.0 * tuck - 14.0 * recover
            p.near_arm_upper = 36.0 - 30.0 * tuck - 4.0 * recover
            p.near_arm_lower = 14.0 - 24.0 * tuck - 6.0 * recover
            p.far_leg_upper = 98.0 - 14.0 * tuck + 8.0 * recover
            p.far_leg_lower = 180.0 - 48.0 * tuck + 10.0 * recover
            p.near_leg_upper = 72.0 - 10.0 * tuck + 8.0 * recover
            p.near_leg_lower = 20.0 - 6.0 * tuck + 14.0 * recover
            p.whole_body_rotation = -360.0 * spin
            p.eye_squint = 0.22 + 0.10 * tuck
        elif animation == "slide":
            skid = smoothstep(clamp(t / 0.35, 0.0, 1.0))
            p.root_x = 4.0 + 10.0 * t
            p.root_y = 8.0
            p.body_tilt = -24.0 + 4.0 * wave
            p.head_tilt = -10.0
            p.far_arm_upper = 176.0
            p.far_arm_lower = 164.0
            p.near_arm_upper = 150.0 - 14.0 * skid
            p.near_arm_lower = 162.0 - 10.0 * skid
            p.far_leg_upper = 152.0
            p.far_leg_lower = 152.0
            p.near_leg_upper = 126.0
            p.near_leg_lower = 142.0
            p.eye_squint = 0.28
        elif animation == "crouch_walk":
            stride = math.sin(t * math.tau)
            bounce = (1.0 - math.cos(t * math.tau * 2.0)) * 0.5
            p.root_y = 5.0 + bounce * 0.9
            p.root_x = stride * 0.8
            p.body_bob = 3.0
            p.body_tilt = -10.0 - stride * 2.5
            p.head_tilt = -5.0
            p.far_arm_upper = 150.0 + stride * 7.0
            p.far_arm_lower = 140.0 + stride * 5.0
            p.near_arm_upper = 24.0 - stride * 8.0
            p.near_arm_lower = 12.0 - stride * 5.0
            p.far_leg_upper = 133.0 + stride * 14.0
            p.far_leg_lower = 48.0 - max(0.0, stride) * 9.0
            p.near_leg_upper = 94.0 - stride * 14.0
            p.near_leg_lower = 46.0 - max(0.0, -stride) * 9.0
            p.eye_squint = 0.16
        elif animation == "pickup":
            bend = smoothstep(clamp(t / 0.55, 0.0, 1.0)) * (1.0 - 0.35 * smoothstep(clamp((t - 0.60) / 0.40, 0.0, 1.0)))
            lift = smoothstep(clamp((t - 0.42) / 0.45, 0.0, 1.0))
            p.root_y = 4.0 * bend - 3.0 * lift
            p.body_tilt = -18.0 * bend + 6.0 * lift
            p.head_tilt = -9.0 * bend + 3.0 * lift
            p.near_arm_upper = 26.0 + 50.0 * bend - 48.0 * lift
            p.near_arm_lower = 44.0 + 34.0 * bend - 46.0 * lift
            p.far_arm_upper = 160.0
            p.far_arm_lower = 150.0
            p.far_leg_upper = 132.0
            p.far_leg_lower = 56.0
            p.near_leg_upper = 96.0
            p.near_leg_lower = 52.0
            p.eye_squint = 0.12
        elif animation == "throw":
            wind = 1.0 - smoothstep(clamp(t / 0.35, 0.0, 1.0))
            release = smoothstep(clamp((t - 0.28) / 0.32, 0.0, 1.0))
            p.root_x = -3.0 * wind + 6.0 * release
            p.body_tilt = -18.0 * wind + 18.0 * release
            p.head_tilt = -6.0 * wind + 4.0 * release
            p.near_arm_upper = -50.0 * wind + 30.0 * release
            p.near_arm_lower = -68.0 * wind + 24.0 * release
            p.far_arm_upper = 162.0
            p.far_arm_lower = 146.0
            p.far_leg_upper = 118.0 + 8.0 * release
            p.near_leg_upper = 66.0 - 8.0 * wind
            p.eye_squint = 0.26
        elif animation == "aim":
            settle = smoothstep(clamp(t / 0.36, 0.0, 1.0))
            p.root_x = -1.0 * settle
            p.body_tilt = -6.0 * settle
            p.head_tilt = -2.0 * settle
            p.far_arm_upper = 160.0
            p.far_arm_lower = 148.0
            p.near_arm_upper = -2.0 + 2.0 * wave
            p.near_arm_lower = -8.0 + 2.0 * wave
            p.far_leg_upper = 120.0
            p.near_leg_upper = 72.0
            p.eye_squint = 0.18
        elif animation == "shoot":
            recoil = 1.0 - smoothstep(clamp(t / 0.50, 0.0, 1.0))
            p.root_x = -3.0 * recoil
            p.body_tilt = -10.0 - 7.0 * recoil
            p.head_tilt = -3.0 - 3.0 * recoil
            p.near_arm_upper = -5.0 - 12.0 * recoil
            p.near_arm_lower = -10.0 - 12.0 * recoil
            p.far_arm_upper = 160.0
            p.far_arm_lower = 148.0
            p.far_leg_upper = 122.0
            p.near_leg_upper = 70.0
            p.eye_squint = 0.32
        elif animation == "charge":
            charge = smoothstep(t)
            pulse = 0.5 + 0.5 * math.sin(t * math.tau * 3.0)
            p.root_y = -2.0 * pulse
            p.body_tilt = -6.0 + pulse * 4.0
            p.head_tilt = -4.0 + pulse * 3.0
            p.far_arm_upper = 188.0 - 28.0 * charge
            p.far_arm_lower = 176.0 - 22.0 * charge
            p.near_arm_upper = -34.0 + 16.0 * pulse
            p.near_arm_lower = -34.0 + 18.0 * pulse
            p.far_leg_upper = 126.0
            p.near_leg_upper = 76.0
            p.eye_squint = 0.18 + 0.22 * pulse
        elif animation == "cast":
            cast = smoothstep(clamp(t / 0.70, 0.0, 1.0))
            p.root_y = -3.0 * math.sin(t * math.pi)
            p.body_tilt = -7.0 + 14.0 * cast
            p.head_tilt = -4.0 + 7.0 * cast
            p.far_arm_upper = 180.0 - 20.0 * cast
            p.far_arm_lower = 162.0 - 12.0 * cast
            p.near_arm_upper = -50.0 + 46.0 * cast
            p.near_arm_lower = -58.0 + 42.0 * cast
            p.far_leg_upper = 124.0
            p.near_leg_upper = 74.0
            p.eye_squint = 0.20
        elif animation == "celebrate":
            hop = abs(math.sin(t * math.tau))
            p.root_y = -8.0 * hop
            p.body_tilt = wave * 8.0
            p.head_tilt = -wave * 6.0
            p.far_arm_upper = 214.0 + wave * 12.0
            p.far_arm_lower = 218.0 + wave * 10.0
            p.near_arm_upper = -74.0 - wave * 12.0
            p.near_arm_lower = -82.0 - wave * 10.0
            p.far_leg_upper = 118.0 + wave * 10.0
            p.near_leg_upper = 78.0 - wave * 10.0
            p.eye_squint = 0.06
        elif animation == "sit":
            settle = smoothstep(clamp(t / 0.55, 0.0, 1.0))
            p.root_y = 13.0 * settle
            p.body_tilt = -6.0 * settle
            p.head_tilt = -2.0 * settle
            p.far_arm_upper = 158.0
            p.far_arm_lower = 142.0
            p.near_arm_upper = 20.0
            p.near_arm_lower = 4.0
            p.far_leg_upper = 158.0
            p.far_leg_lower = 18.0
            p.near_leg_upper = 38.0
            p.near_leg_lower = 18.0
            p.eye_squint = 0.10
        elif animation == "sleep":
            breathe = 0.5 + 0.5 * math.sin(t * math.tau)
            p.root_y = 14.0
            p.body_tilt = -8.0
            p.head_tilt = -7.0
            p.body_bob = breathe * 1.0
            p.far_arm_upper = 160.0
            p.far_arm_lower = 150.0
            p.near_arm_upper = 20.0
            p.near_arm_lower = 16.0
            p.far_leg_upper = 158.0
            p.far_leg_lower = 24.0
            p.near_leg_upper = 42.0
            p.near_leg_lower = 24.0
            p.eye_squint = 0.55
            p.blink = True
        elif animation == "hover":
            hover = math.sin(t * math.tau)
            # Modest lift only. The original -12 pushed the antenna
            # tip above the canvas top (clipped 6-8px in downsample),
            # which the eye reads as the antenna blipping in and out
            # across the bob oscillation. -2 keeps the silhouette
            # entirely in-frame; the FLAMES carry the "lifted off the
            # ground" read, not the body translation.
            p.root_y = -2.0 + hover * 2.0
            p.body_tilt = -6.0 + hover * 3.0
            p.head_tilt = -2.0 + hover * 1.5
            p.far_arm_upper = 168.0 + hover * 8.0
            p.far_arm_lower = 150.0 + hover * 6.0
            p.near_arm_upper = 12.0 - hover * 8.0
            p.near_arm_lower = 2.0 - hover * 6.0
            p.far_leg_upper = 142.0
            p.far_leg_lower = 138.0
            p.near_leg_upper = 116.0
            p.near_leg_lower = 128.0
            p.eye_squint = 0.13
        elif animation == "stomp":
            wind = 1.0 - smoothstep(clamp(t / 0.46, 0.0, 1.0))
            impact = smoothstep(clamp((t - 0.42) / 0.22, 0.0, 1.0))
            p.root_y = -12.0 * wind + 7.0 * impact
            p.body_tilt = -8.0 * wind + 10.0 * impact
            p.head_tilt = -6.0 * wind + 4.0 * impact
            p.far_arm_upper = 188.0 - 28.0 * impact
            p.near_arm_upper = -34.0 + 40.0 * impact
            p.far_leg_upper = 156.0 - 42.0 * impact
            p.far_leg_lower = 24.0 + 24.0 * impact
            p.near_leg_upper = 78.0
            p.near_leg_lower = 58.0
            p.eye_squint = 0.30
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
        elif animation == "dash_startup":
            # Very short animation-only pre-dash: torso winds back, far leg
            # plants, near leg cocks. No actual horizontal travel here.
            wind = smoothstep(clamp(t / 0.85, 0.0, 1.0))
            p.root_x = -3.0 * wind
            p.root_y = 1.5 * wind
            p.body_tilt = -18.0 * wind
            p.head_tilt = -8.0 * wind
            p.far_arm_upper = 170.0 + 4.0 * wind
            p.far_arm_lower = 156.0 + 4.0 * wind
            p.near_arm_upper = 28.0 - 20.0 * wind
            p.near_arm_lower = 18.0 - 16.0 * wind
            # Keep both knees reading forward for a right-facing crouch-start.
            p.far_leg_upper = 108.0 + 10.0 * wind
            p.far_leg_lower = 54.0 + 6.0 * wind
            p.near_leg_upper = 78.0 - 8.0 * wind
            p.near_leg_lower = 118.0 - 18.0 * wind
            p.eye_squint = 0.18 + 0.18 * wind
        elif animation == "land_hard":
            # Heavier counterpart to "land". Big squash on impact, slow rebound,
            # arms thrown forward to brace, knees pancake.
            impact = 1.0 - smoothstep(clamp(t / 0.42, 0.0, 1.0))
            dust = math.sin(t * math.pi)
            p.root_y = 11.0 * impact - 1.0 * dust
            p.body_bob = 6.0 * impact
            p.body_tilt = -14.0 * impact
            p.head_tilt = -9.0 * impact
            p.far_arm_upper = 150.0 - 14.0 * impact
            p.far_arm_lower = 136.0 - 12.0 * impact
            p.near_arm_upper = 18.0 + 32.0 * impact
            p.near_arm_lower = 10.0 + 24.0 * impact
            p.far_leg_upper = 112.0 + 6.0 * impact
            p.far_leg_lower = 30.0 + 4.0 * impact
            p.near_leg_upper = 82.0 + 6.0 * impact
            p.near_leg_lower = 22.0 + 6.0 * impact
            p.eye_squint = 0.42 * impact + 0.10
        elif animation == "land_recovery":
            # Rise back to neutral after a (hard) landing: legs unfold, torso
            # straightens, arms drop. t=0 is "still crouched", t=1 is idle-ish.
            rise = smoothstep(t)
            p.root_y = 6.0 * (1.0 - rise)
            p.body_bob = 3.0 * (1.0 - rise)
            p.body_tilt = -10.0 * (1.0 - rise)
            p.head_tilt = -5.0 * (1.0 - rise)
            p.far_arm_upper = 148.0 + 10.0 * rise
            p.far_arm_lower = 134.0 + 8.0 * rise
            p.near_arm_upper = 32.0 - 14.0 * rise
            p.near_arm_lower = 18.0 - 10.0 * rise
            p.far_leg_upper = 114.0 - 10.0 * rise
            p.far_leg_lower = 34.0 + 58.0 * rise
            p.near_leg_upper = 84.0 - 12.0 * rise
            p.near_leg_lower = 26.0 + 58.0 * rise
            p.eye_squint = 0.20 * (1.0 - rise)
        elif animation == "wall_grab":
            # Pinned-against-wall hold. Both arms locked straight out
            # forward against the wall surface, body leans hard into
            # the wall, knees drawn up so the silhouette reads "hanging
            # on by the hands" rather than just "standing leaning."
            # 2026-05-29: rewrote the pose for a much clearer read.
            # Old pose had arms at near-shoulder angles + extended
            # legs, which looked almost identical to idle. New pose:
            # arms aim FORWARD (clamped horizontally toward the wall),
            # body tilts ~22° into the wall, near leg pulled up high
            # so a bent knee crosses the body silhouette.
            breathe = math.sin(t * math.tau)
            p.root_x = 10.2 + breathe * 0.5
            p.root_y = -11.0 + breathe * 0.8
            p.body_tilt = 12.0 + breathe * 1.0
            p.head_tilt = -8.0 + breathe * 0.6
            p.head_look = -0.85
            p.head_dx = -3.8
            p.head_dy = -1.5
            # Both hands now actually grip the wall ahead of the player.
            p.far_arm_upper = -18.0 + breathe * 1.4
            p.far_arm_lower = 10.0 + breathe * 1.0
            p.near_arm_upper = -30.0 - breathe * 1.4
            p.near_arm_lower = 4.0 - breathe * 1.0
            # Player-right leg braces on the wall; the back leg stays folded
            # under the body with a forward knee rather than reversing.
            p.far_leg_upper = 52.0 + breathe * 1.8
            p.far_leg_lower = 16.0 + breathe * 1.2
            p.near_leg_upper = 102.0 + breathe * 1.5
            p.near_leg_lower = 176.0 - breathe * 1.0
            p.eye_squint = 0.18
        elif animation == "ledge_climb":
            # Slow, deliberate haul-up against an overhead grip. Arms remain
            # locked overhead; the body pulls upward in two pump phases while
            # the legs scuff at the wall below for traction.
            pump = math.sin(t * math.tau)
            haul = smoothstep(clamp(t / 0.82, 0.0, 1.0))
            p.root_x = 6.0 + 0.8 * haul - 0.5 * pump
            p.root_y = -12.0 + 8.0 * haul + 0.8 * pump
            p.body_tilt = -14.0 + 12.0 * haul + pump * 1.5
            p.head_tilt = -8.0 + 6.0 * haul + pump * 0.8
            # Both arms reach toward the ledge on the player's front/right.
            p.far_arm_upper = -30.0 + 18.0 * haul
            p.far_arm_lower = -10.0 + 26.0 * haul
            p.near_arm_upper = -44.0 + 16.0 * haul
            p.near_arm_lower = -18.0 + 24.0 * haul
            p.far_leg_upper = 98.0 + pump * 5.0 - 6.0 * haul
            p.far_leg_lower = 176.0 - pump * 6.0 - 12.0 * haul
            p.near_leg_upper = 72.0 - pump * 4.0 + 4.0 * haul
            p.near_leg_lower = 18.0 + pump * 4.0 + 8.0 * haul
            p.eye_squint = 0.30
        elif animation == "float_glide":
            # Sustained aerial float (Peach/Kirby-style). Body stays close to
            # upright, arms held out for balance, gentle vertical bob and the
            # legs hang relaxed below. Distinct from hover (which has rocket
            # jets reading) and swim (water stroke loop).
            bob = math.sin(t * math.tau)
            drift = math.sin(t * math.tau + math.pi / 2)
            p.root_x = drift * 0.8
            p.root_y = -8.0 + bob * 1.8
            p.body_bob = bob * 0.6
            p.body_tilt = -3.0 + bob * 1.5
            p.head_tilt = -2.0 + drift * 1.0
            # Arms held outward like balance wings.
            p.far_arm_upper = 218.0 + bob * 4.0
            p.far_arm_lower = 214.0 + bob * 4.0
            p.near_arm_upper = -38.0 - bob * 4.0
            p.near_arm_lower = -42.0 - bob * 4.0
            # Legs dangle softly; toes drift with the bob.
            p.far_leg_upper = 102.0 + bob * 4.0
            p.far_leg_lower = 96.0 - bob * 3.0
            p.near_leg_upper = 78.0 - bob * 4.0
            p.near_leg_lower = 86.0 + bob * 3.0
            p.eye_squint = 0.10
        elif animation == "ledge_getup":
            # Mantling pop-up: arms transition from overhead grip (early) to
            # planted push-off (mid) to standing (end). The body rises from
            # under-the-ledge crouch to upright over the duration.
            mantle = smoothstep(t)
            pop = smoothstep(clamp((t - 0.45) / 0.55, 0.0, 1.0))
            p.root_x = 5.0 + 7.0 * mantle
            p.root_y = -10.0 + 10.0 * mantle - 2.0 * pop
            p.body_tilt = -14.0 + 20.0 * mantle - 8.0 * pop
            p.head_tilt = -8.0 + 10.0 * mantle - 4.0 * pop
            p.far_arm_upper = lerp(-28.0, 150.0, mantle)
            p.far_arm_lower = lerp(-8.0, 132.0, mantle)
            p.near_arm_upper = lerp(-42.0, 18.0, mantle)
            p.near_arm_lower = lerp(-18.0, 8.0, mantle)
            p.far_leg_upper = lerp(102.0, 106.0, mantle)
            p.far_leg_lower = lerp(176.0, 94.0, mantle)
            p.near_leg_upper = lerp(72.0, 70.0, mantle)
            p.near_leg_lower = lerp(18.0, 86.0, mantle)
            p.eye_squint = 0.26 - 0.10 * mantle
        elif animation == "ledge_roll":
            # Smash-Bros style ledge roll: tumble onto the platform.
            # Phase map (t = 0..1):
            #   0.00 – 0.18 : hang, legs swinging forward to build momentum
            #   0.18 – 0.55 : tuck into a tight ball mid-roll (limbs in)
            #   0.55 – 0.85 : exit the tuck, body extending forward
            #   0.85 – 1.00 : stand up low, ready to act
            #
            # The runtime arms `Player::dodge_roll_timer` for the
            # whole transition + a 100ms tail, so this is also the
            # invuln pose — the tuck keeps the silhouette small and
            # round to read "rolling = not a target."
            swing = smoothstep(clamp(t / 0.18, 0.0, 1.0))
            tuck = smoothstep(clamp((t - 0.14) / 0.24, 0.0, 1.0))
            stand = smoothstep(clamp((t - 0.82) / 0.18, 0.0, 1.0))
            roll_spin = smoothstep(clamp((t - 0.08) / 0.76, 0.0, 1.0))
            tight = tuck * (1.0 - 0.55 * stand)
            p.root_x = 4.0 + 26.0 * smoothstep(t)
            p.root_y = -8.0 + 4.0 * swing - 6.0 * tight + 6.0 * stand
            p.body_tilt = -16.0 * (1.0 - stand)
            p.head_tilt = -8.0 * (1.0 - stand)
            p.far_arm_upper = lerp(-30.0, 164.0, tight) + 10.0 * stand
            p.far_arm_lower = lerp(-8.0, 148.0, tight) + 12.0 * stand
            p.near_arm_upper = lerp(-42.0, 26.0, swing) - 26.0 * tight - 10.0 * stand
            p.near_arm_lower = lerp(-18.0, 12.0, swing) - 22.0 * tight - 8.0 * stand
            p.far_leg_upper = lerp(102.0, 104.0, stand) - 18.0 * tight
            p.far_leg_lower = lerp(176.0, 94.0, stand) - 48.0 * tight
            p.near_leg_upper = lerp(72.0, 70.0, stand) - 12.0 * tight
            p.near_leg_lower = lerp(18.0, 84.0, stand) - 8.0 * tight
            p.whole_body_rotation = -360.0 * roll_spin
            p.eye_squint = 0.28 + 0.16 * tight - 0.10 * stand
        elif animation == "ledge_getup_attack":
            # Swing-onto-platform attack: the player vaults up while
            # slashing in a single continuous motion. Engine fires
            # `MovementOp::Slash` at frame 0; the visual swing peaks
            # mid-animation so the hitbox and the sprite agree on
            # "this is the slash beat."
            #
            # Phase map:
            #   0.00 – 0.18 : windup, arm cocks back while body lifts
            #   0.18 – 0.55 : swing peak (active hitbox window in
            #                  the existing slash-anim timer)
            #   0.55 – 0.80 : follow-through and body planting
            #   0.80 – 1.00 : recovery into combat-ready standing
            mantle = smoothstep(t)
            wind = 1.0 - smoothstep(clamp(t / 0.18, 0.0, 1.0))
            strike = smoothstep(clamp((t - 0.16) / 0.39, 0.0, 1.0))
            recover = smoothstep(clamp((t - 0.78) / 0.22, 0.0, 1.0))
            # Body rises from below-the-lip to standing, same arc as
            # the regular getup but with a forward thrust at the
            # strike peak.
            p.root_x = 5.0 + 7.0 * mantle + 3.0 * strike - 1.5 * recover
            p.root_y = -10.0 + 10.0 * mantle - 2.5 * strike
            p.body_tilt = -14.0 + 18.0 * mantle - 8.0 * strike + 2.0 * recover
            p.head_tilt = -7.0 + 9.0 * mantle - 4.0 * strike
            # Far arm starts on the front/right ledge, then drops into the
            # balance/back-swing role as the slash comes through.
            p.far_arm_upper = lerp(-28.0, 152.0, mantle) + 8.0 * recover
            p.far_arm_lower = lerp(-8.0, 136.0, mantle) + 6.0 * recover
            # Near (sword) arm: cocks high-back during wind, whips
            # forward over the lip during strike, settles into a
            # combat-ready guard during recovery.
            p.near_arm_upper = lerp(-46.0, 18.0, mantle) - 30.0 * wind + 86.0 * strike - 28.0 * recover
            p.near_arm_lower = lerp(-18.0, 4.0, mantle) - 24.0 * wind + 78.0 * strike - 22.0 * recover
            # Legs mantle up; near leg drives forward at the strike
            # peak (committed weight transfer).
            p.far_leg_upper = lerp(102.0, 106.0, mantle) + 10.0 * strike
            p.far_leg_lower = lerp(176.0, 94.0, mantle) + 6.0 * strike
            p.near_leg_upper = lerp(72.0, 68.0, mantle) - 10.0 * wind + 8.0 * strike
            p.near_leg_lower = lerp(18.0, 84.0, mantle) - 8.0 * wind + 6.0 * strike
            # Slash overlay: peaks mid-strike (frame 4-5), trails into
            # recovery. The "side" direction matches the engine's
            # `AttackIntent::Forward` projection used for ledge-getup
            # attack hitbox geometry.
            p.slash = max(0.25, wind, strike)
            p.slash_arc = strike
            p.slash_dir = "side"
            p.eye_squint = 0.24 + 0.22 * strike - 0.08 * recover
        elif animation == "attack_side":
            # 3-frame continuous sweep (blade arcs across all frames, hitbox live throughout) forehand (see ANIMATIONS["attack_side"]).
            # NO windup: frame 0 is the full strike — blade extended,
            # body committed forward — so the hit reads on the very
            # first frame. Frame 1 is the lone wind-down, settling back
            # toward a combat-ready guard. `strike`/`recover` are linear
            # in t so they land cleanly on the two frames (t=0 / t=1) but
            # still read if the row is ever re-timed to more frames.
            strike = 1.0  # 1.0 @ frame 0, 0.0 @ frame 1
            recover = 0.0       # 0.0 @ frame 0, 1.0 @ frame 1
            p.root_x = 6.0 * strike - 1.6 * recover
            p.body_tilt = 16.0 * strike - 4.0 * recover
            p.head_tilt = 6.0 * strike
            p.far_arm_upper = 156.0
            p.far_arm_lower = 145.0
            p.near_arm_upper = -30.0 + 64.0 * strike - 18.0 * recover
            p.near_arm_lower = -18.0 + 58.0 * strike - 20.0 * recover
            p.far_leg_upper = 106.0 + 12.0 * strike
            p.far_leg_lower = 92.0
            p.near_leg_upper = 60.0 - 6.0 * recover
            p.near_leg_lower = 80.0
            p.slash = max(0.25, strike)
            p.slash_arc = t
            p.slash_dir = "side"
            p.eye_squint = 0.22 + strike * 0.22
        elif animation == "attack_up":
            # Up-tilt: blade arcs from low forward up over the head to back-up.
            # 3-frame continuous sweep (blade arcs across all frames, hitbox live throughout) (see ANIMATIONS): f0 = full overhead strike,
            # f1 = wind-down. No windup; pose body unchanged.
            wind = 0.0
            strike = 1.0
            recover = 0.0
            p.root_y = -3.0 * strike + 1.0 * recover
            p.body_tilt = 4.0 * wind - 8.0 * strike + 2.0 * recover
            p.head_tilt = 4.0 * wind - 10.0 * strike + 2.0 * recover
            p.far_arm_upper = 158.0 - 14.0 * strike
            p.far_arm_lower = 146.0 - 12.0 * strike
            # Near arm whips from cocked-back-low overhead to back-high.
            p.near_arm_upper = 60.0 * wind - 70.0 * strike - 20.0 * recover
            p.near_arm_lower = 70.0 * wind - 80.0 * strike - 24.0 * recover
            p.far_leg_upper = 110.0 + 6.0 * strike
            p.near_leg_upper = 72.0 - 6.0 * wind
            p.slash = max(0.25, wind, strike)
            p.slash_arc = t
            p.slash_dir = "up"
            p.eye_squint = 0.20 + strike * 0.18
        elif animation == "attack_down":
            # Grounded down tilt — kneeling forward poke (Marth/Lucina
            # down-tilt). Body sinks into a deep crouch, far leg folds
            # under as the kneeling support, near leg plants forward
            # bent; near arm extends low and horizontal with the blade
            # for a short thrust. Distinct from the aerial Down spike.
            # 3-frame continuous sweep (blade arcs across all frames, hitbox live throughout): f0 = committed kneeling poke, f1 = rise/recover.
            crouch = 1.0
            thrust = 1.0
            recover = 0.0
            sink = max(0.65 * crouch, thrust, 0.55 * (1.0 - recover) * crouch)
            p.root_y = 9.0 * sink
            p.body_bob = 4.0 * sink
            p.body_tilt = -8.0 * crouch + 4.0 * thrust
            p.head_tilt = -4.0 * crouch + 2.0 * thrust
            # Far arm tucked back for balance during the kneel.
            p.far_arm_upper = 178.0 + 6.0 * crouch
            p.far_arm_lower = 168.0 + 6.0 * crouch
            # Near arm whips from windup to a horizontal forward poke.
            # 0° = blade tip pointing forward (right) when the hand is
            # at the elbow's right; values approach 0 as the arm
            # extends along +x.
            p.near_arm_upper = lerp(38.0, 4.0, thrust) - 10.0 * recover
            p.near_arm_lower = lerp(46.0, -4.0, thrust) - 12.0 * recover
            # Kneeling support: far leg folds high knee + heel tight.
            p.far_leg_upper = 98.0 + 4.0 * crouch
            p.far_leg_lower = 184.0 - 10.0 * recover
            # Front leg planted forward, bent.
            p.near_leg_upper = 74.0 + 6.0 * crouch
            p.near_leg_lower = 18.0 + 8.0 * crouch + 14.0 * recover
            p.slash = max(0.25, crouch, thrust)
            p.slash_arc = t
            p.slash_dir = "low_poke"
            p.eye_squint = 0.18 + thrust * 0.18
        elif animation == "air_neutral":
            # Aerial neutral: short spin-slash with the blade making a near
            # full revolution around the body. Body floats; legs tuck.
            # 3-frame continuous sweep (blade arcs across all frames, hitbox live throughout): f0 = mid-spin slash, f1 = settle.
            spin = 1.0
            float_t = 1.0
            p.root_y = -6.0 + float_t * 2.0
            p.body_tilt = -6.0 + spin * 24.0
            p.head_tilt = -2.0 + spin * 12.0
            p.far_arm_upper = 178.0
            p.far_arm_lower = 168.0
            p.near_arm_upper = -16.0 + spin * 18.0
            p.near_arm_lower = -22.0 + spin * 18.0
            p.far_leg_upper = 132.0 - 14.0 * float_t
            p.far_leg_lower = 70.0 + 18.0 * float_t
            p.near_leg_upper = 92.0 - 10.0 * float_t
            p.near_leg_lower = 56.0 + 18.0 * float_t
            p.slash = 0.85
            p.slash_arc = t
            p.slash_dir = "air_neutral"
            p.eye_squint = 0.24
        elif animation == "air_forward":
            # Fair: forward-down committed swing. Body leans into the swing,
            # near arm whips forward, far leg trails behind for balance.
            # 3-frame continuous sweep (blade arcs across all frames, hitbox live throughout): f0 = committed aerial swing, f1 = settle.
            wind = 0.0
            strike = 1.0
            float_t = 1.0
            p.root_x = -2.0 * wind + 5.0 * strike
            p.root_y = -7.0 + float_t * 1.6
            p.body_tilt = -6.0 * wind + 18.0 * strike
            p.head_tilt = -2.0 * wind + 6.0 * strike
            p.far_arm_upper = 188.0 - 12.0 * strike
            p.far_arm_lower = 176.0 - 10.0 * strike
            p.near_arm_upper = -42.0 * wind + 48.0 * strike
            p.near_arm_lower = -48.0 * wind + 52.0 * strike
            p.far_leg_upper = 140.0 - 12.0 * strike
            p.far_leg_lower = 78.0 + 12.0 * strike
            p.near_leg_upper = 102.0 - 10.0 * strike
            p.near_leg_lower = 60.0 + 14.0 * strike
            p.slash = max(0.25, wind, strike)
            p.slash_arc = t
            p.slash_dir = "air_forward"
            p.eye_squint = 0.24 + strike * 0.18
        elif animation == "air_back":
            # Bair: backward sword swing. Body turns/leans backward, near arm
            # whips behind, blade traces back-up to back-down arc. Front shows
            # a clear shoulder check / over-shoulder cut read.
            # 3-frame continuous sweep (blade arcs across all frames, hitbox live throughout): f0 = committed aerial swing, f1 = settle.
            wind = 0.0
            strike = 1.0
            float_t = 1.0
            p.root_x = 2.0 * wind - 4.0 * strike
            p.root_y = -7.0 + float_t * 1.6
            p.body_tilt = 6.0 * wind - 14.0 * strike
            p.head_tilt = 3.0 * wind - 6.0 * strike
            p.far_arm_upper = 168.0 + 12.0 * strike
            p.far_arm_lower = 156.0 + 10.0 * strike
            # Near arm starts low-forward, whips to back-high.
            p.near_arm_upper = 40.0 * wind + 180.0 * strike
            p.near_arm_lower = 30.0 * wind + 188.0 * strike
            p.far_leg_upper = 132.0 - 4.0 * strike
            p.far_leg_lower = 96.0 - 8.0 * strike
            p.near_leg_upper = 92.0 + 6.0 * strike
            p.near_leg_lower = 86.0 + 10.0 * strike
            p.slash = max(0.25, wind, strike)
            p.slash_arc = t
            p.slash_dir = "air_back"
            p.eye_squint = 0.24 + strike * 0.18
        elif animation == "air_down":
            # Dair: straight-down stab/spike. Body inverts slightly head-down,
            # blade points down with a brief downward thrust pulse.
            # 3-frame continuous sweep (blade arcs across all frames, hitbox live throughout): f0 = downward spike, f1 = settle.
            wind = 0.0
            thrust = 1.0
            p.root_y = -8.0 + 4.0 * thrust
            p.body_tilt = 4.0 * wind + 12.0 * thrust
            p.head_tilt = 2.0 * wind + 8.0 * thrust
            p.far_arm_upper = 168.0 + 14.0 * thrust
            p.far_arm_lower = 156.0 + 12.0 * thrust
            p.near_arm_upper = 60.0 - 16.0 * wind + 18.0 * thrust
            p.near_arm_lower = 78.0 - 18.0 * wind + 20.0 * thrust
            p.far_leg_upper = 118.0 + 8.0 * thrust
            p.far_leg_lower = 92.0
            p.near_leg_upper = 84.0
            p.near_leg_lower = 80.0
            p.slash = max(0.3, thrust, wind)
            p.slash_arc = t
            p.slash_dir = "air_down"
            p.eye_squint = 0.26 + thrust * 0.18
        elif animation == "air_up":
            # Uair: straight-up thrust. Body straightens, near arm reaches
            # overhead, blade points up with a quick upward thrust pulse.
            # 3-frame continuous sweep (blade arcs across all frames, hitbox live throughout): f0 = overhead thrust, f1 = settle.
            wind = 0.0
            thrust = 1.0
            p.root_y = -10.0 - 2.0 * thrust
            p.body_tilt = -4.0 * wind - 10.0 * thrust
            p.head_tilt = -3.0 * wind - 8.0 * thrust
            p.far_arm_upper = 178.0 - 8.0 * thrust
            p.far_arm_lower = 166.0 - 8.0 * thrust
            # Near arm rises overhead.
            p.near_arm_upper = -40.0 * wind - 92.0 * thrust
            p.near_arm_lower = -34.0 * wind - 96.0 * thrust
            p.far_leg_upper = 138.0 - 6.0 * thrust
            p.far_leg_lower = 110.0 - 8.0 * thrust
            p.near_leg_upper = 100.0 - 4.0 * thrust
            p.near_leg_lower = 96.0 - 6.0 * thrust
            p.slash = max(0.3, thrust, wind)
            p.slash_arc = t
            p.slash_dir = "air_up"
            p.eye_squint = 0.24 + thrust * 0.18
        return p


    def _draw_archetype_accessories(self, img: Image.Image, d: ImageDraw.ImageDraw, spec: BotSpec, pal: Dict[str, Color], S: float, root_x: float, ground_y: float, body_center: Point, head_center: Point) -> None:
        """Draw small silhouette-level NPC variant reads after the base robot.

        These are intentionally additive and keyed only by archetype so review
        configs can create distinct NPCs without a larger rig schema yet.
        """
        name = (spec.archetype or spec.palette_name or "").lower()
        outline = pal["outline"]
        accent = pal["accent"]
        glow = pal["visor_glow"]
        if any(token in name for token in ["drift", "dj", "lofi", "radio"]):
            # DJ headphones, transmitter antenna, and a tiny waveform panel.
            d.arc((head_center[0] - 27*S, head_center[1] - 21*S, head_center[0] + 29*S, head_center[1] + 22*S), start=190, end=350, fill=accent, width=max(1, int(2.0*S)))
            d.ellipse((head_center[0] - 30*S, head_center[1] - 4*S, head_center[0] - 20*S, head_center[1] + 10*S), fill=pal["metal"], outline=outline, width=max(1, int(1*S)))
            d.ellipse((head_center[0] + 24*S, head_center[1] - 4*S, head_center[0] + 34*S, head_center[1] + 10*S), fill=pal["metal"], outline=outline, width=max(1, int(1*S)))
            d.rounded_rectangle((body_center[0] - 30*S, body_center[1] - 6*S, body_center[0] - 18*S, body_center[1] + 18*S), radius=3*S, fill=_with_alpha(accent, 130), outline=outline, width=max(1, int(1*S)))
            for i, h in enumerate([4, 9, 6, 12]):
                x = body_center[0] - (26 - i*2.8)*S
                d.line([(x, body_center[1] + 11*S), (x, body_center[1] + (11-h)*S)], fill=glow, width=max(1, int(1.2*S)))
        elif any(token in name for token in ["pulse", "captain", "voyage"]):
            # Officer cap and comet-tail speed pennants.
            d.rounded_rectangle((head_center[0] - 18*S, head_center[1] - 29*S, head_center[0] + 18*S, head_center[1] - 21*S), radius=4*S, fill=accent, outline=outline, width=max(1, int(1*S)))
            d.polygon([(head_center[0] + 2*S, head_center[1] - 31*S), (head_center[0] + 12*S, head_center[1] - 42*S), (head_center[0] + 21*S, head_center[1] - 28*S)], fill=glow, outline=outline)
            for y in (ground_y - 22*S, ground_y - 14*S, ground_y - 6*S):
                d.line([(root_x - 23*S, y), (root_x - 44*S, y - 3*S)], fill=_with_alpha(accent, 120), width=max(1, int(1.5*S)))
        elif any(token in name for token in ["tech", "disrupt"]):
            # Oversized smart-glasses + lanyard badge + laptop slab.
            d.rounded_rectangle((head_center[0] - 18*S, head_center[1] - 9*S, head_center[0] + 24*S, head_center[1] + 4*S), radius=3*S, fill=_with_alpha(glow, 210), outline=outline, width=max(1, int(1*S)))
            d.line([(body_center[0] + 2*S, body_center[1] - 7*S), (body_center[0] + 8*S, body_center[1] + 13*S)], fill=accent, width=max(1, int(2*S)))
            d.rounded_rectangle((body_center[0] + 3*S, body_center[1] + 10*S, body_center[0] + 15*S, body_center[1] + 20*S), radius=2*S, fill=pal["metal"], outline=outline, width=max(1, int(0.8*S)))
            d.rounded_rectangle((body_center[0] - 31*S, body_center[1] - 4*S, body_center[0] - 19*S, body_center[1] + 18*S), radius=2*S, fill=_rgba("#2B3348"), outline=outline, width=max(1, int(1*S)))
        elif any(token in name for token in ["dino", "saur", "liberator"]):
            # Dinosaur crest, tail flag, and fossil-bone badge.
            for dx in (-10, 1, 12):
                d.polygon([(head_center[0] + dx*S, head_center[1] - 28*S), (head_center[0] + (dx+6)*S, head_center[1] - 42*S), (head_center[0] + (dx+12)*S, head_center[1] - 28*S)], fill=accent, outline=outline)
            d.arc((root_x - 43*S, ground_y - 40*S, root_x - 9*S, ground_y - 2*S), start=215, end=338, fill=accent, width=max(1, int(3*S)))
            d.ellipse((body_center[0] + 8*S, body_center[1] - 1*S, body_center[0] + 18*S, body_center[1] + 9*S), fill=_rgba("#F3E8C8"), outline=outline, width=max(1, int(0.8*S)))
        elif any(token in name for token in ["env", "advocate", "solace"]):
            # Leaf collar and seed-pod satchel.
            for dx, rot in [(-18, -1), (-5, 1), (8, -1), (20, 1)]:
                d.ellipse((body_center[0] + dx*S - 5*S, body_center[1] - 17*S, body_center[0] + dx*S + 7*S, body_center[1] - 5*S), fill=_with_alpha(accent, 170), outline=outline, width=max(1, int(0.7*S)))
            d.rounded_rectangle((body_center[0] - 30*S, body_center[1] - 2*S, body_center[0] - 18*S, body_center[1] + 18*S), radius=5*S, fill=_with_alpha(glow, 150), outline=outline, width=max(1, int(1*S)))
        elif any(token in name for token in ["iron", "marshal", "military"]):
            # Red cap, epaulettes, and command sash.
            d.rounded_rectangle((head_center[0] - 19*S, head_center[1] - 29*S, head_center[0] + 20*S, head_center[1] - 21*S), radius=3*S, fill=accent, outline=outline, width=max(1, int(1*S)))
            d.rectangle((head_center[0] - 8*S, head_center[1] - 34*S, head_center[0] + 8*S, head_center[1] - 29*S), fill=pal["metal"], outline=outline, width=max(1, int(0.8*S)))
            for sx in (-1, 1):
                d.rounded_rectangle((body_center[0] + sx*14*S - 8*S, body_center[1] - 14*S, body_center[0] + sx*14*S + 8*S, body_center[1] - 5*S), radius=3*S, fill=pal["metal"], outline=outline, width=max(1, int(1*S)))
            d.line([(body_center[0] - 16*S, body_center[1] - 12*S), (body_center[0] + 17*S, body_center[1] + 16*S)], fill=accent, width=max(1, int(3*S)))
        elif any(token in name for token in ["moonlit", "canal", "noct"]):
            # Crescent antenna, dock lantern, and watery half-cloak.
            d.arc((head_center[0] - 4*S, head_center[1] - 40*S, head_center[0] + 25*S, head_center[1] - 15*S), start=80, end=270, fill=glow, width=max(1, int(2*S)))
            d.rounded_rectangle((body_center[0] - 31*S, body_center[1] - 6*S, body_center[0] - 19*S, body_center[1] + 16*S), radius=4*S, fill=_rgba("#24304F"), outline=outline, width=max(1, int(1*S)))
            d.ellipse((body_center[0] - 28*S, body_center[1] - 2*S, body_center[0] - 22*S, body_center[1] + 5*S), fill=glow)
            for yy in (ground_y - 10*S, ground_y - 4*S):
                d.arc((root_x - 30*S, yy - 5*S, root_x + 26*S, yy + 8*S), start=190, end=350, fill=_with_alpha(accent, 120), width=max(1, int(1*S)))
        elif any(token in name for token in ["glass", "warden", "canopy"]):
            # Glass antler branches and translucent cloak triangle.
            for sx in (-1, 1):
                base = (head_center[0] + sx*12*S, head_center[1] - 23*S)
                d.line([base, (base[0] + sx*13*S, base[1] - 17*S)], fill=glow, width=max(1, int(2*S)))
                d.line([(base[0] + sx*7*S, base[1] - 9*S), (base[0] + sx*18*S, base[1] - 13*S)], fill=glow, width=max(1, int(1.3*S)))
            d.polygon([(body_center[0] - 25*S, body_center[1] - 7*S), (body_center[0] + 25*S, body_center[1] - 7*S), (body_center[0] + 2*S, body_center[1] + 28*S)], fill=_with_alpha(glow, 64), outline=_with_alpha(accent, 150))
        elif name in {"guardian", "heavy_guardian"}:
            # Broad shoulder guard + hip plate reads as a defensive NPC even in idle.
            d.rounded_rectangle((body_center[0] - 24*S, body_center[1] - 14*S, body_center[0] - 7*S, body_center[1] - 3*S), radius=4*S, fill=pal["metal"], outline=outline, width=max(1, int(1.0*S)))
            d.rounded_rectangle((body_center[0] + 11*S, body_center[1] - 14*S, body_center[0] + 28*S, body_center[1] - 3*S), radius=4*S, fill=pal["metal"], outline=outline, width=max(1, int(1.0*S)))
            d.rounded_rectangle((body_center[0] - 23*S, body_center[1] + 3*S, body_center[0] + 24*S, body_center[1] + 12*S), radius=3*S, fill=_with_alpha(accent, 185), outline=outline, width=max(1, int(0.9*S)))
        elif name in {"runner", "scout_runner"}:
            # Thin antenna fin and ankle streamers.
            d.polygon([(head_center[0] - 13*S, head_center[1] - 29*S), (head_center[0] + 3*S, head_center[1] - 42*S), (head_center[0] - 1*S, head_center[1] - 27*S)], fill=accent, outline=outline)
            for y in (ground_y - 14*S, ground_y - 7*S):
                d.line([(root_x - 24*S, y), (root_x - 42*S, y + 3*S)], fill=_with_alpha(accent, 145), width=max(1, int(1.6*S)))
        elif name in {"diver", "swimmer"}:
            # Bubble helmet ring and fin-like boots.
            d.ellipse((head_center[0] - 27*S, head_center[1] - 24*S, head_center[0] + 30*S, head_center[1] + 22*S), outline=_with_alpha(glow, 120), width=max(1, int(1.6*S)))
            for x in (root_x - 12*S, root_x + 11*S):
                d.polygon([(x, ground_y - 2*S), (x + 16*S, ground_y + 5*S), (x - 2*S, ground_y + 7*S)], fill=_with_alpha(accent, 160), outline=outline)
        elif name in {"caster", "radio_mage"}:
            # Floating tuning halo and spell mote.
            d.arc((head_center[0] - 23*S, head_center[1] - 34*S, head_center[0] + 25*S, head_center[1] - 9*S), start=190, end=350, fill=_with_alpha(accent, 180), width=max(1, int(1.6*S)))
            d.ellipse((head_center[0] + 25*S, head_center[1] - 26*S, head_center[0] + 33*S, head_center[1] - 18*S), fill=_with_alpha(glow, 210), outline=outline)
        elif name in {"engineer", "field_mechanic"}:
            # Backpack battery and little wrench badge.
            d.rounded_rectangle((body_center[0] - 29*S, body_center[1] - 7*S, body_center[0] - 19*S, body_center[1] + 19*S), radius=3*S, fill=pal["metal"], outline=outline, width=max(1, int(1.0*S)))
            d.line([(body_center[0] + 13*S, body_center[1] - 4*S), (body_center[0] + 26*S, body_center[1] + 8*S)], fill=accent, width=max(1, int(2*S)))
            d.ellipse((body_center[0] + 23*S, body_center[1] + 5*S, body_center[0] + 29*S, body_center[1] + 11*S), outline=outline, width=max(1, int(1*S)))
        elif name in {"medic", "field_medic"}:
            # Cross badge and soft backpack pack.
            d.rounded_rectangle((body_center[0] - 28*S, body_center[1] - 6*S, body_center[0] - 18*S, body_center[1] + 17*S), radius=3*S, fill=_with_alpha(accent, 130), outline=outline, width=max(1, int(1.0*S)))
            bx, by = body_center[0] + 8*S, body_center[1] + 1*S
            d.rounded_rectangle((bx - 8*S, by - 2*S, bx + 8*S, by + 3*S), radius=1.5*S, fill=accent)
            d.rounded_rectangle((bx - 2*S, by - 8*S, bx + 3*S, by + 8*S), radius=1.5*S, fill=accent)
        elif name in {"miner", "cavern_miner"}:
            # Headlamp + tool roll.
            d.rounded_rectangle((head_center[0] - 18*S, head_center[1] - 25*S, head_center[0] + 16*S, head_center[1] - 18*S), radius=3*S, fill=pal["metal"], outline=outline, width=max(1, int(1.0*S)))
            d.ellipse((head_center[0] + 5*S, head_center[1] - 29*S, head_center[0] + 15*S, head_center[1] - 19*S), fill=glow, outline=outline, width=max(1, int(0.8*S)))
            d.polygon([(head_center[0] + 14*S, head_center[1] - 26*S), (head_center[0] + 45*S, head_center[1] - 34*S), (head_center[0] + 45*S, head_center[1] - 16*S)], fill=_with_alpha(glow, 42))
        elif name in {"archivist", "map_keeper"}:
            # Scroll satchel and paper tab.
            d.rounded_rectangle((body_center[0] - 30*S, body_center[1] - 4*S, body_center[0] - 17*S, body_center[1] + 17*S), radius=3*S, fill=_with_alpha(accent, 160), outline=outline, width=max(1, int(1.0*S)))
            d.rectangle((body_center[0] - 28*S, body_center[1] - 2*S, body_center[0] - 18*S, body_center[1] + 4*S), fill=_rgba("#F3E8C8"), outline=outline, width=max(1, int(0.7*S)))

    def _leg_chain(self, hip: Point, upper_len: float, lower_len: float, a1: float, a2: float) -> Tuple[Point, Point]:
        knee = add(hip, vec(upper_len, a1))
        ankle = add(knee, vec(lower_len, a2))
        return knee, ankle

    def _solve_leg_ik(self, hip: Point, ankle: Point, upper_len: float, lower_len: float, bend_sign: float = 1.0) -> Tuple[Point, float, float]:
        """Solve a simple two-bone leg toward an ankle target.

        `bend_sign=+1` keeps the knee pitched toward screen-right / facing-forward
        for the right-facing robot silhouette.
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

    def _sample_cycle(self, samples: Tuple[float, ...], phase: float) -> float:
        count = len(samples)
        if count == 0:
            return 0.0
        phase = (phase % 1.0) * count
        i0 = int(math.floor(phase)) % count
        i1 = (i0 + 1) % count
        frac = phase - math.floor(phase)
        return lerp(samples[i0], samples[i1], frac)

    def _player_compact_stride_layout(self, animation: str, phase: float, body_center: Point, ground_y: float, S: float) -> Dict[str, Tuple[Point, Point, float, float, Color, float]]:
        """Hand-tuned crossover gait for the compact player robot.

        The smart-house walk works because the feet pass through the middle of
        the stride instead of staying locked to wide lanes. The compact robot
        keeps knees, so we only borrow that crossover timing and use explicit
        foot targets to avoid the previous out-of-whack foot placements.
        """
        is_run = animation == "run"
        if is_run:
            far_x = (-9.6, -5.2, 0.8, 6.2, 8.8, 5.0, -0.8, -6.8)
            far_lift = (0.0, 0.0, 1.6, 0.5, 0.0, 4.6, 2.6, 0.7)
            near_x = (10.8, 6.1, 0.2, -5.8, -7.8, -4.0, 1.8, 7.6)
            near_lift = (0.0, 3.4, 5.5, 0.7, 0.0, 0.0, 1.2, 0.4)
            pelvis_w = 12.0
            pelvis_h = 7.5
            foot_tilt = -7.0
        else:
            far_x = (-8.0, -4.5, 0.4, 4.6, 6.0, 3.6, -1.0, -5.8)
            far_lift = (0.0, 0.0, 0.9, 0.3, 0.0, 3.1, 1.3, 0.3)
            near_x = (9.2, 4.9, 0.2, -4.2, -5.4, -2.9, 1.4, 6.2)
            near_lift = (0.0, 2.0, 3.2, 0.3, 0.0, 0.0, 0.8, 0.2)
            pelvis_w = 11.0
            pelvis_h = 7.0
            foot_tilt = -5.0

        foot_w = 12.0 * S
        foot_h = 6.0 * S
        lane_far = -1.8
        lane_near = 2.0
        phase = phase % 1.0
        far_fc = (body_center[0] + self._sample_cycle(far_x, phase) * S, ground_y - 2.0 * S - self._sample_cycle(far_lift, phase) * S)
        near_fc = (body_center[0] + self._sample_cycle(near_x, phase) * S, ground_y - 2.0 * S - self._sample_cycle(near_lift, phase) * S)
        far_ankle = (far_fc[0] - foot_w * 0.34 - lane_far * S, far_fc[1] - 2.0 * S)
        near_ankle = (near_fc[0] - foot_w * 0.34 - lane_near * S, near_fc[1] - 2.0 * S)
        pelvis_center = (body_center[0] + 0.8 * S, body_center[1] + 10.0 * S)
        return {
            "far": (far_ankle, far_fc, foot_w, foot_h, foot_tilt, lane_far),
            "near": (near_ankle, near_fc, foot_w, foot_h, foot_tilt, lane_near),
            "pelvis": (pelvis_center, (pelvis_w * S, pelvis_h * S)),
        }

    def _draw_leg_from_ik(self, img: Image.Image, d: ImageDraw.ImageDraw, hip: Point, ankle: Point, foot_center: Point, foot_size: Tuple[float, float], foot_angle: float, tint: Color, outline_color: Color, outline: float, upper_len: float, lower_len: float, pixel_scale: float, bend_sign: float = 1.0) -> None:
        # Keep a small persistent bend so the compact robot never reads as if
        # the shin telescopes longer in the crossover frames.
        dx = ankle[0] - hip[0]
        dy = ankle[1] - hip[1]
        dist = math.hypot(dx, dy)
        max_reach = max(1.0, upper_len + lower_len - 1.8 * pixel_scale)
        if dist > max_reach and dist > 1e-5:
            scale = max_reach / dist
            clamped_ankle = (hip[0] + dx * scale, hip[1] + dy * scale)
            shift = (clamped_ankle[0] - ankle[0], clamped_ankle[1] - ankle[1])
            ankle = clamped_ankle
            foot_center = (foot_center[0] + shift[0], foot_center[1] + shift[1])
        knee, _a1, _a2 = self._solve_leg_ik(hip, ankle, upper_len, lower_len, bend_sign)
        draw_capsule(d, hip, knee, 2.9 * pixel_scale, tint, outline_color, outline * 0.65)
        draw_capsule(d, knee, ankle, 2.7 * pixel_scale, tint, outline_color, outline * 0.65)
        draw_rotated_rounded_rect(img, foot_center, foot_size, foot_angle, 3.0 * pixel_scale, tint, outline_color, outline * 0.7)

    def _draw_shadow(self, img: Image.Image, ground_y: float, x: float, width: float, alpha: int) -> None:
        d = ImageDraw.Draw(img)
        d.ellipse((x - width / 2, ground_y - 5, x + width / 2, ground_y + 6), fill=(0, 0, 0, alpha))

    def _draw_blink_out_fx(self, img: Image.Image, root_x: float, ground_y: float, S: float, frame_index: int, frame_count: int) -> None:
        d = ImageDraw.Draw(img)
        t = 0.0 if frame_count <= 1 else frame_index / float(frame_count - 1)
        charge = smoothstep(clamp(t / 0.56, 0.0, 1.0))
        burst = smoothstep(clamp((t - 0.30) / 0.50, 0.0, 1.0))
        energy = self.PALETTE["visor_glow"]
        accent = self.PALETTE["accent"]
        source_x = root_x + 8 * S
        mid_y = ground_y - 50 * S

        # Expanding portal rings at the departure point.
        for rscale, alpha in [
            (0.62 + 0.55 * charge, int(145 * max(charge, 0.15))),
            (0.40 + 0.85 * burst, int(118 * max(burst, 0.12))),
        ]:
            rx, ry = 8.0 * S * rscale, 14.0 * S * rscale
            box = (source_x - rx, mid_y - ry - 4 * S, source_x + rx, mid_y + ry - 4 * S)
            d.ellipse(box, outline=_with_alpha(energy, alpha), width=max(1, int(1.3 * S)))

        # Vertical slivers and shard sparks make the disappearance read like teleportation.
        for i, dx in enumerate((-10, -4, 3, 10)):
            height = (28.0 - i * 2.4 + 9.0 * burst) * S
            alpha = int((90 - i * 14) * max(charge, burst))
            if alpha > 0:
                x = source_x + dx * S
                d.line([(x, mid_y - height / 2), (x + 6 * S, mid_y + height / 2)], fill=_with_alpha(accent, alpha), width=max(1, int(1.7 * S)))
                d.line([(x + 2 * S, mid_y - height / 2), (x - 4 * S, mid_y + height / 2)], fill=_with_alpha(energy, max(20, alpha - 24)), width=max(1, int(0.9 * S)))

        for i in range(4):
            frac = i / 3.0 if 3 else 0.0
            sx = source_x - 8 * S + frac * 18 * S
            sy = mid_y - 12 * S - frac * 7 * S
            ex = sx + (6 + i * 2) * S
            ey = sy - (8 + i * 2) * S
            d.line([(sx, sy), (ex, ey)], fill=_with_alpha(energy, int(65 * max(charge, burst))), width=max(1, int(1.0 * S)))

        ripple_alpha = int(80 * max(charge, burst))
        if ripple_alpha > 0:
            d.ellipse((source_x - 18 * S, ground_y - 7 * S, source_x + 16 * S, ground_y + 1 * S), outline=_with_alpha(accent, ripple_alpha), width=max(1, int(1.0 * S)))

    def _draw_blink_in_fx(self, img: Image.Image, root_x: float, ground_y: float, S: float, frame_index: int, frame_count: int) -> None:
        d = ImageDraw.Draw(img)
        t = 0.0 if frame_count <= 1 else frame_index / float(frame_count - 1)
        appear = smoothstep(clamp(t / 0.60, 0.0, 1.0))
        settle = ease_out_cubic(appear)
        energy = self.PALETTE["visor_glow"]
        accent = self.PALETTE["accent"]
        dest_x = root_x + 8 * S
        mid_y = ground_y - 50 * S

        for rscale, alpha in [
            (1.25 - 0.45 * settle, int(155 * max(0.18, 1.0 - t * 0.55))),
            (0.52 + 0.30 * appear, int(120 * max(0.20, 1.0 - t * 0.35))),
        ]:
            rx, ry = 8.5 * S * rscale, 14.0 * S * rscale
            box = (dest_x - rx, mid_y - ry - 4 * S, dest_x + rx, mid_y + ry - 4 * S)
            d.ellipse(box, outline=_with_alpha(energy, alpha), width=max(1, int(1.3 * S)))

        for i, dx in enumerate((-14, -7, 0, 8, 14)):
            height = (30.0 - i * 2.6 + 7.0 * (1.0 - settle)) * S
            alpha = int((95 - i * 12) * max(0.18, 1.0 - t * 0.42))
            x = dest_x + dx * S
            d.line([(x, mid_y - height / 2), (dest_x, mid_y)], fill=_with_alpha(accent, alpha), width=max(1, int(1.6 * S)))
            d.line([(x, mid_y + height / 2), (dest_x + 2 * S, mid_y - 2 * S)], fill=_with_alpha(energy, max(15, alpha - 18)), width=max(1, int(0.9 * S)))

        ripple_alpha = int(78 * max(0.18, 1.0 - t * 0.35))
        d.ellipse((dest_x - 18 * S, ground_y - 7 * S, dest_x + 16 * S, ground_y + 1 * S), outline=_with_alpha(energy, ripple_alpha), width=max(1, int(1.0 * S)))

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
                dx = (frac - 0.5) * (22.0 * S * progress) + math.sin(frac * math.pi * 7.0 + progress * 7.0) * 1.8 * S * progress
                dy = -(5.0 + abs(frac - 0.5) * 18.0) * S * progress
                alpha_scale = max(0.06, 1.0 - 0.88 * progress)
                if progress > 0.35 and (i + int(progress * 10)) % 3 == 0:
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
                dx = (frac - 0.5) * (24.0 * S * (1.0 - progress))
                dy = -(3.0 + abs(frac - 0.5) * 16.0) * S * (1.0 - progress)
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

    def _draw_rigid_head(self, img: Image.Image, center: Point, spec: BotSpec, pal: Dict[str, Color], S: float, angle: float, blink_closed: bool, squint: float, dead: bool, look: float = 1.0) -> None:
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
        look = clamp(float(look), -1.0, 1.0)
        ant_base = (cx - 8 * S * look, cy - head_h * 0.50)
        ant_tip = (cx - 12 * S * look, cy - head_h * 0.50 - spec.antenna_h * S)
        d.line([ant_base, ant_tip], fill=pal["outline"], width=max(1, int(1.7 * S)))
        d.ellipse(_bbox(ant_tip, 6.4 * S, 6.4 * S), fill=pal["accent"], outline=pal["outline"], width=max(1, int(1.0 * S)))

        outer = _bbox((cx, cy), head_w + 2 * outline, head_h + 2 * outline)
        inner = _bbox((cx, cy), head_w, head_h)
        d.rounded_rectangle(outer, radius=9 * S + outline, fill=pal["outline"])
        d.rounded_rectangle(inner, radius=9 * S, fill=pal["shell_top"])

        # Semi-transparent head highlights/shadows must be alpha-composited
        # over the opaque head base.  Drawing translucent fills directly onto
        # the layer replaces the destination alpha and leaves the mouth/shadow
        # area partially transparent after rotation/compositing.
        detail = Image.new("RGBA", layer.size, (0, 0, 0, 0))
        hd = ImageDraw.Draw(detail)
        hd.rounded_rectangle((inner[0] + 4 * S, inner[1] + 3 * S, inner[2] - 5 * S, cy - 1 * S), radius=7 * S, fill=_with_alpha((255, 255, 255, 255), 205))
        hd.rounded_rectangle((inner[0] + 8 * S, cy + 1 * S, inner[2] - 2 * S, inner[3] - 3 * S), radius=7 * S, fill=_with_alpha(pal["shell_side"], 190))
        layer.alpha_composite(detail)

        visor_center = (cx + 7.0 * S * look, cy - 1.0 * S)
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

    # Per-direction blade and arc-visual tuning for directional slashes.
    # Each entry: (blade_base_deg, blade_sweep_deg, (arc_box_dx0, dy0, dx1, dy1)*S, arc_start, arc_end).
    # blade_base + slash_arc*blade_sweep is fed to vec() for the blade tip;
    # the arc box is relative to the hand position.
    _SLASH_DIR_TABLE: Dict[str, Tuple[float, float, Tuple[float, float, float, float], float, float]] = {
        "side":         (-18.0,  52.0, ( -5.0, -34.0,  42.0,  20.0),  -70.0,   42.0),
        "up":           ( 80.0, -200.0, (-32.0, -52.0,  32.0,   8.0),  200.0,  350.0),
        "down":         (-110.0, 200.0, (-22.0, -10.0,  42.0,  52.0),  -30.0,  130.0),
        "back":         (162.0,  -52.0, (-42.0, -34.0,   5.0,  20.0),  138.0,  250.0),
        "air_neutral":  ( 30.0,  340.0, (-32.0, -32.0,  32.0,  32.0),    0.0,  360.0),
        "air_forward":  (-55.0,  130.0, ( -8.0, -30.0,  48.0,  28.0),  -90.0,   70.0),
        "air_back":     (235.0, -130.0, (-48.0, -30.0,   8.0,  28.0),  110.0,  270.0),
        "air_down":     ( 75.0,   30.0, (-16.0,  -4.0,  16.0,  42.0),   60.0,  120.0),
        "air_up":       (-105.0, -30.0, (-16.0, -42.0,  16.0,   4.0), -120.0,  -60.0),
        # Kneeling forward poke (Marth/Lucina down-tilt). Blade stays
        # nearly horizontal, tips up slightly through the thrust as the
        # body drops. Short, low arc visual ahead of the hand.
        "low_poke":     ( -4.0, -16.0, ( -2.0,  -8.0,  36.0,   8.0),  -28.0,   18.0),
    }

    def _draw_robot_arm(self, img: Image.Image, d: ImageDraw.ImageDraw, shoulder: Point, a1: float, a2: float, tint: Color, spec: BotSpec, pal: Dict[str, Color], S: float, outline: float, slash: float = 0.0, slash_arc: float = 0.0, slash_dir: str = "side") -> Point:
        elbow = add(shoulder, vec(spec.arm_upper * S, a1))
        hand = add(elbow, vec(spec.arm_lower * S, a2))
        draw_capsule(d, shoulder, elbow, 2.7 * S, tint, pal["outline"], outline * 0.65)
        draw_capsule(d, elbow, hand, 2.5 * S, tint, pal["outline"], outline * 0.65)
        d.ellipse((hand[0] - 4 * S, hand[1] - 4 * S, hand[0] + 4 * S, hand[1] + 4 * S), fill=tint, outline=pal["outline"], width=max(1, int(outline * 0.65)))
        if slash:
            blade_base, blade_sweep, arc_rel, arc_start, arc_end = self._SLASH_DIR_TABLE.get(slash_dir, self._SLASH_DIR_TABLE["side"])
            blade_angle = blade_base + slash_arc * blade_sweep
            tip = add(hand, vec(spec.blade_len * S, blade_angle))
            d.line([hand, tip], fill=pal["outline"], width=max(1, int(4.0 * S)))
            d.line([hand, tip], fill=pal["accent"], width=max(1, int(2.1 * S)))
            if slash_arc > 0.18:
                arc_box = (
                    hand[0] + arc_rel[0] * S,
                    hand[1] + arc_rel[1] * S,
                    hand[0] + arc_rel[2] * S,
                    hand[1] + arc_rel[3] * S,
                )
                d.arc(arc_box, start=arc_start, end=arc_end, fill=(12, 235, 255, 170), width=max(1, int(2.4 * S)))
        return hand

    def _render_highres(self, spec: BotSpec, animation: str, frame_index: int, frame_count: int, size: Tuple[int, int], background: Optional[Color], scale: int) -> Image.Image:
        W, H = size[0] * scale, size[1] * scale
        bg = (0, 0, 0, 0) if background is None else background
        img = Image.new("RGBA", (W, H), bg)
        # Scale the 128-base character to the requested frame width so a
        # render_scale>1 canvas draws the SAME character with more native
        # pixels (matches the toon generator's S=(W/128)*ss). Identical at the
        # 128 default; only render_scale>1 changes it.
        S = float(scale) * (size[0] / 128.0)
        pal = self._palette_for_spec(spec)
        p = self.pose_for_animation(animation, frame_index, frame_count)
        ground_y = (101.0 + p.root_y) * S
        root_x = (62.0 + p.root_x) * S
        outline = 1.8 * S

        # Ground shadow removed; the in-game renderer composites the
        # robot over floor geometry that already provides contact.
        d = ImageDraw.Draw(img)

        if animation == "blink_out":
            self._draw_blink_out_fx(img, root_x, ground_y, S, frame_index, frame_count)
        elif animation == "blink_in":
            self._draw_blink_in_fx(img, root_x, ground_y, S, frame_index, frame_count)

        if p.dash:
            for i in range(4):
                y = (49 + i * 12 + math.sin(frame_index + i) * 2) * S
                d.line([(14 * S, y), ((43 - i * 3) * S, y - 2 * S)], fill=(12, 235, 255, 90), width=max(1, int(1.6 * S)))
        if animation == "swim":
            for i in range(4):
                x = (24 + i * 18 + math.sin(frame_index + i) * 2) * S
                y = (83 + i % 2 * 6) * S
                d.arc((x, y, x + 18 * S, y + 9 * S), start=180, end=358, fill=(89, 210, 255, 92), width=max(1, int(1.1 * S)))
        if animation == "interact":
            pulse = math.sin((0.0 if frame_count <= 1 else frame_index / float(frame_count - 1)) * math.pi)
            if pulse > 0.05:
                d.line([(94 * S, 49 * S), (107 * S, 42 * S)], fill=(255, 241, 150, int(140 * pulse)), width=max(1, int(1.5 * S)))
                d.line([(96 * S, 61 * S), (112 * S, 61 * S)], fill=(255, 241, 150, int(140 * pulse)), width=max(1, int(1.5 * S)))
        if animation == "block":
            d.rounded_rectangle((31 * S, 43 * S, 43 * S, 85 * S), radius=4 * S, fill=(197, 205, 232, 165), outline=pal["outline"], width=max(1, int(1.0 * S)))

        # Review-only action FX. These are intentionally simple read-at-a-glance
        # effects that make the generated sheet useful before Rust selects rows.
        if animation in {"land", "stomp"}:
            impact_t = 0.0 if frame_count <= 1 else frame_index / float(frame_count - 1)
            impact = 1.0 - min(1.0, abs(impact_t - 0.52) / 0.52)
            for i in range(3):
                rx = (18 + i * 12 + 20 * impact) * S
                ry = (2.2 + i * 0.7) * S
                alpha = int(86 * impact * (1.0 - i * 0.18))
                if alpha > 0:
                    d.arc((root_x - rx, ground_y - ry, root_x + rx, ground_y + ry), start=190, end=350, fill=_with_alpha(pal["accent"], alpha), width=max(1, int(1.1 * S)))
        if animation in {"slide", "roll"}:
            for i in range(4):
                alpha = 70 - i * 13
                x0 = (28 - i * 8) * S
                y = (96 + i * 3) * S
                d.line([(x0, y), (x0 - (16 + i * 3) * S, y + 4 * S)], fill=(210, 206, 190, alpha), width=max(1, int(1.2 * S)))
        if animation == "pickup":
            lift_t = 0.0 if frame_count <= 1 else frame_index / float(frame_count - 1)
            obj_y = (93 - 36 * smoothstep(clamp((lift_t - 0.30) / 0.55, 0.0, 1.0))) * S
            obj_x = (91 - 8 * smoothstep(clamp(lift_t / 0.55, 0.0, 1.0))) * S
            d.rounded_rectangle((obj_x - 5 * S, obj_y - 5 * S, obj_x + 5 * S, obj_y + 5 * S), radius=2 * S, fill=_with_alpha(pal["accent"], 200), outline=pal["outline"], width=max(1, int(0.8 * S)))
        if animation == "throw":
            throw_t = 0.0 if frame_count <= 1 else frame_index / float(frame_count - 1)
            if throw_t > 0.18:
                arc_t = smoothstep(clamp((throw_t - 0.18) / 0.74, 0.0, 1.0))
                obj_x = (73 + 45 * arc_t) * S
                obj_y = (52 - 18 * math.sin(arc_t * math.pi) + 18 * arc_t) * S
                d.ellipse((obj_x - 4 * S, obj_y - 4 * S, obj_x + 4 * S, obj_y + 4 * S), fill=_with_alpha(pal["accent"], 210), outline=pal["outline"], width=max(1, int(0.8 * S)))
                d.arc((66 * S, 34 * S, 126 * S, 85 * S), start=205, end=314, fill=_with_alpha(pal["accent"], 78), width=max(1, int(1.0 * S)))
        if animation in {"aim", "shoot"}:
            tx, ty = 111 * S, 51 * S
            a = 110 if animation == "aim" else 170
            d.ellipse((tx - 6 * S, ty - 6 * S, tx + 6 * S, ty + 6 * S), outline=_with_alpha(pal["visor_glow"], a), width=max(1, int(1.0 * S)))
            d.line([(tx - 9 * S, ty), (tx - 3 * S, ty)], fill=_with_alpha(pal["visor_glow"], a), width=max(1, int(0.9 * S)))
            d.line([(tx + 3 * S, ty), (tx + 9 * S, ty)], fill=_with_alpha(pal["visor_glow"], a), width=max(1, int(0.9 * S)))
        if animation == "shoot":
            flash = 1.0 - smoothstep(clamp((0.0 if frame_count <= 1 else frame_index / float(frame_count - 1)) / 0.50, 0.0, 1.0))
            if flash > 0.05:
                mx, my = 96 * S, 54 * S
                d.polygon([(mx, my), (mx + 22 * S * flash, my - 7 * S), (mx + 18 * S * flash, my + 6 * S)], fill=(255, 238, 126, int(205 * flash)))
        if animation in {"charge", "cast"}:
            spell_t = 0.0 if frame_count <= 1 else frame_index / float(frame_count - 1)
            radius = (11 + 13 * smoothstep(spell_t)) * S
            cx = (92 if animation == "charge" else 98) * S
            cy = (57 if animation == "charge" else 45) * S
            alpha = 90 + int(70 * (0.5 + 0.5 * math.sin(spell_t * math.tau * 3.0)))
            d.ellipse((cx - radius, cy - radius, cx + radius, cy + radius), outline=_with_alpha(pal["accent"], alpha), width=max(1, int(1.5 * S)))
            d.ellipse((cx - 3 * S, cy - 3 * S, cx + 3 * S, cy + 3 * S), fill=_with_alpha(pal["visor_glow"], alpha))
        if animation == "celebrate":
            for i, (dx, dy) in enumerate([(-20, -35), (0, -41), (21, -34), (33, -20), (-30, -18)]):
                phase = (frame_index + i) % max(1, frame_count)
                alpha = 80 + int(90 * (phase / max(1, frame_count - 1)))
                x = root_x + dx * S
                y = ground_y + dy * S + phase * 1.4 * S
                color = pal["accent"] if i % 2 else pal["visor_glow"]
                d.rectangle((x - 2 * S, y - 2 * S, x + 2 * S, y + 2 * S), fill=_with_alpha(color, min(210, alpha)))
        if animation == "sleep":
            for i in range(3):
                zt = ((frame_index + i * 2) % max(1, frame_count)) / max(1, frame_count - 1)
                x = (83 + i * 8) * S
                y = (42 - zt * 24) * S
                d.text((x, y), "Z", fill=_with_alpha(pal["visor_glow"], int(150 * (1.0 - zt * 0.45))))
        if animation == "hover":
            # 2026-05-29: doubled the jet plume length and added a
            # bright white-hot inner core so the fly pose reads as
            # genuine thrust rather than "small flicker under feet."
            # Two flicker phases (offset 90°) so the two jets pulse
            # slightly out of sync — visually busier and obviously
            # "alive" instead of mirrored.
            flame_l = 0.65 + 0.35 * math.sin(frame_index * 1.7)
            flame_r = 0.65 + 0.35 * math.sin(frame_index * 1.7 + math.pi / 2)
            # Anchor each jet at the actual foot position, mirroring
            # the leg_chain / foot_center math used by the body draw
            # below. The previous fixed-canvas flames floated ~10px
            # below the lifted hover body, looking detached. Tracking
            # the feet keeps the jets glued on through the bob.
            hover_body_x = root_x + lerp(0.0, 12 * S, p.collapse)
            hover_body_y = ground_y - lerp(39 * S, 11 * S, p.collapse) + p.body_bob * S
            jet_hips = (
                (hover_body_x - 6 * S, hover_body_y + 11 * S, p.far_leg_upper, p.far_leg_lower, -2.0, flame_l),
                (hover_body_x + 8 * S, hover_body_y + 10 * S, p.near_leg_upper, p.near_leg_lower, 3.0, flame_r),
            )
            for hx, hy, a1, a2, foot_shift, flame in jet_hips:
                _, ankle = self._leg_chain((hx, hy), spec.leg_upper * S, spec.leg_lower * S, a1, a2)
                foot_w = 12 * S
                foot_cx = ankle[0] + (foot_w * 0.34) + foot_shift * S
                foot_cy = min(ground_y - 2 * S, ankle[1] + 2 * S)
                # Three-layer plume so the jets read clearly even
                # downsampled. Width-then-length so the silhouette is
                # a tall, slightly-tapered teardrop.
                top = (foot_cx, foot_cy + 1 * S)
                # Outer glow halo (faint cyan, widest, longest).
                halo_w = (7 + 3 * flame) * S
                halo_base = foot_cy + (22 + 14 * flame) * S
                d.polygon(
                    [
                        (foot_cx, foot_cy - 1 * S),
                        (foot_cx - halo_w, halo_base),
                        (foot_cx + halo_w, halo_base),
                    ],
                    fill=_with_alpha(pal["visor_glow"], int(70 * flame)),
                )
                # Mid cyan plume — the main flame body.
                outer_base = foot_cy + (18 + 12 * flame) * S
                d.polygon(
                    [
                        top,
                        (foot_cx - 5 * S, outer_base),
                        (foot_cx + 5 * S, outer_base),
                    ],
                    fill=_with_alpha(pal["visor_glow"], int(180 * flame)),
                )
                # Inner yellow flame.
                inner_base = foot_cy + (12 + 8 * flame) * S
                d.polygon(
                    [
                        (foot_cx, foot_cy + 2 * S),
                        (foot_cx - 3 * S, inner_base),
                        (foot_cx + 3 * S, inner_base),
                    ],
                    fill=(255, 235, 130, int(210 * flame)),
                )
                # White-hot core (narrowest, brightest, anchored at
                # the nozzle so the eye locks on to "this is a jet").
                core_base = foot_cy + (7 + 5 * flame) * S
                d.polygon(
                    [
                        (foot_cx, foot_cy + 2 * S),
                        (foot_cx - 1.6 * S, core_base),
                        (foot_cx + 1.6 * S, core_base),
                    ],
                    fill=(255, 255, 240, int(240 * flame)),
                )

        # Draw the actor on its own transparent layer first.  Some
        # animations apply a whole-body layer rotation (roll / ledge_roll);
        # drawing directly onto the base canvas and then rotating caused the
        # unrotated body to remain behind the rotated copy, which read as a
        # duplicate torso.  Keeping the actor isolated also avoids
        # alpha-compositing the canvas onto itself for non-teleport rows.
        character_img = Image.new("RGBA", (W, H), (0, 0, 0, 0))
        character_draw = ImageDraw.Draw(character_img)

        # Stable body reference. Death moves to a lying pose without scaling.
        collapse = p.collapse
        # `vertical_scale` collapses the rendered silhouette toward the
        # ground anchor for compact archetypes (player_compact). It
        # multiplies every hard-coded vertical offset that places the
        # body/head/hip/shoulder above the ground; leg/arm lengths are
        # already squashed via the BotSpec fields so the legs still
        # reach the ground line.
        vscale = max(0.3, float(spec.vertical_scale))
        body_y_offset = lerp(39 * S * vscale, 11 * S, collapse)
        head_y_offset = lerp(68 * S * vscale, 15 * S, collapse)
        body_center = (root_x + lerp(0, 12 * S, collapse), ground_y - body_y_offset + p.body_bob * S)
        head_center = (
            root_x + lerp(12 * S, 34 * S, collapse) + p.head_dx * S,
            ground_y - head_y_offset + p.body_bob * S * 0.4 + p.head_dy * S,
        )
        body_angle = p.body_tilt
        head_angle = p.head_tilt

        player_compact_walk = "player_compact" in str(spec.archetype or "").lower() and animation in {"walk", "run"}
        if player_compact_walk:
            # Keep the torso/hips steadier for the compact player.  The walk
            # reads better when the knees carry the motion rather than an
            # extra bounce layered on top.
            body_center = (root_x + lerp(0, 12 * S, collapse), ground_y - body_y_offset)
            head_center = (
                root_x + lerp(12 * S, 34 * S, collapse) + p.head_dx * S,
                ground_y - head_y_offset + p.head_dy * S,
            )
            hip_far = (body_center[0] - 7 * S, body_center[1] + 10 * S * vscale)
            hip_near = (body_center[0] + 8 * S, body_center[1] + 10 * S * vscale)
        else:
            hip_far = (body_center[0] - 6 * S, body_center[1] + 11 * S * vscale)
            hip_near = (body_center[0] + 8 * S, body_center[1] + 10 * S * vscale)
        shoulder_far = (body_center[0] - 8 * S, body_center[1] - 8 * S * vscale)
        shoulder_near = (body_center[0] + 9 * S, body_center[1] - 8 * S * vscale)

        player_right_leg = None
        player_left_leg = None
        pelvis_draw = None
        if player_compact_walk:
            # Classical 8-pose contact/down/passing/up cycle for the compact
            # player.  We keep the authored ankle targets exactly as-is here;
            # this pass only makes the semantic limb labels and z-order explicit.
            idx = frame_index % 8
            if animation == "run":
                far_ax = (-12.0, -9.2, -4.2, 0.2, 1.6, -1.8, -6.6, -10.5)
                far_ay = (23.0, 23.4, 22.2, 19.4, 22.4, 23.5, 23.1, 22.7)
                near_ax = (10.5, 7.6, 3.2, 0.0, -0.8, 2.4, 6.9, 10.8)
                near_ay = (22.6, 23.5, 23.2, 22.7, 23.0, 23.4, 22.0, 19.2)
                far_shift = (-2.1, -1.7, -0.8, 0.5, 1.5, 1.0, 0.1, -1.0)
                near_shift = (1.9, 1.2, 0.2, -1.2, -1.8, -1.3, -0.4, 0.9)
                foot_tilt = (-8, -5, -2, 3, 8, 5, 2, -3)
                pelvis_w, pelvis_h = 13.0 * S, 7.5 * S
            else:
                far_ax = (-11.4, -8.8, -4.0, 0.0, 1.2, -1.8, -6.2, -10.0)
                far_ay = (23.0, 23.2, 22.0, 19.8, 22.5, 23.3, 22.9, 22.6)
                near_ax = (9.8, 7.0, 3.0, 0.0, -0.6, 2.2, 6.4, 10.0)
                near_ay = (22.6, 23.3, 22.9, 22.6, 23.0, 23.2, 21.8, 19.7)
                far_shift = (-1.8, -1.4, -0.6, 0.4, 1.3, 0.9, 0.0, -0.8)
                near_shift = (1.7, 1.0, 0.1, -1.0, -1.5, -1.0, -0.3, 0.8)
                foot_tilt = (-6, -4, -2, 2, 6, 4, 2, -2)
                pelvis_w, pelvis_h = 12.0 * S, 7.0 * S

            # Semantic mapping for a right-facing, slightly camera-tilted player:
            # - player-right arm = lighter arm on screen-left
            # - player-left arm  = darker arm on screen-right
            # - player-right leg = darker leg on screen-left
            # - player-left leg  = lighter leg on screen-right
            ankle = (body_center[0] + far_ax[idx] * S, body_center[1] + far_ay[idx] * S)
            knee, _a1, _a2 = self._solve_leg_ik(hip_far, ankle, spec.leg_upper * S, spec.leg_lower * S, bend_sign=1.0)
            foot_w = 12.0 * S
            foot_h = 6.0 * S
            foot_center = (ankle[0] + foot_w * 0.34 + far_shift[idx] * S, ankle[1] + 2.0 * S)
            player_right_leg = (hip_far, knee, ankle, pal["shell_side"], foot_center, foot_w, foot_h, foot_tilt[idx] + body_angle * 0.10)

            ankle = (body_center[0] + near_ax[idx] * S, body_center[1] + near_ay[idx] * S)
            knee, _a1, _a2 = self._solve_leg_ik(hip_near, ankle, spec.leg_upper * S, spec.leg_lower * S, bend_sign=1.0)
            foot_center = (ankle[0] + foot_w * 0.34 + near_shift[idx] * S, ankle[1] + 2.0 * S)
            player_left_leg = (hip_near, knee, ankle, pal["shell"], foot_center, foot_w, foot_h, -foot_tilt[(idx + 4) % 8] + body_angle * 0.10)

            pelvis_center = (body_center[0] + 0.8 * S, body_center[1] + 10.0 * S)
            pelvis_draw = (pelvis_center, pelvis_w, pelvis_h)
        elif animation in {"walk", "run"}:
            # Apply the same authored ankle-target baseline more broadly to the
            # side-view robot lane, but scale it from this rig's own leg length
            # instead of copying the compact-player coordinates.  That keeps
            # non-player robot legs visually the same length while improving
            # the contact/down/passing/up read.
            idx = frame_index % 8
            foot_w = 12 * S
            foot_h = 6 * S
            leg_len = (spec.leg_upper + spec.leg_lower) * S
            stride = leg_len * (0.42 if animation == "run" else 0.36)
            base_drop = leg_len * (0.86 if animation == "run" else 0.88)
            far_x = (-1.00, -0.76, -0.36, -0.06, 0.12, -0.18, -0.58, -0.90)
            near_x = (0.92, 0.68, 0.30, 0.04, -0.08, 0.20, 0.62, 0.94)
            far_lift = (0.00, 0.00, 0.05, 0.14, 0.00, 0.02, 0.08, 0.02)
            near_lift = (0.00, 0.02, 0.08, 0.02, 0.00, 0.00, 0.05, 0.14)
            far_shift = (-2.0, -1.5, -0.7, 0.4, 1.4, 1.0, 0.1, -0.9)
            near_shift = (1.8, 1.1, 0.1, -1.1, -1.6, -1.1, -0.3, 0.9)
            foot_tilt = (-8, -5, -2, 3, 8, 5, 2, -3) if animation == "run" else (-6, -4, -2, 2, 6, 4, 2, -2)

            ankle = (body_center[0] + far_x[idx] * stride, hip_far[1] + base_drop - far_lift[idx] * leg_len)
            knee, _a1, _a2 = self._solve_leg_ik(hip_far, ankle, spec.leg_upper * S, spec.leg_lower * S, bend_sign=1.0)
            foot_center = (ankle[0] + (foot_w * 0.34) + far_shift[idx] * S, ankle[1] + 2.0 * S)
            player_right_leg = (hip_far, knee, ankle, pal["shell_side"], foot_center, foot_w, foot_h, foot_tilt[idx] + body_angle * 0.10)

            ankle = (body_center[0] + near_x[idx] * stride, hip_near[1] + base_drop - near_lift[idx] * leg_len)
            knee, _a1, _a2 = self._solve_leg_ik(hip_near, ankle, spec.leg_upper * S, spec.leg_lower * S, bend_sign=1.0)
            foot_center = (ankle[0] + (foot_w * 0.34) + near_shift[idx] * S, ankle[1] + 2.0 * S)
            player_left_leg = (hip_near, knee, ankle, pal["shell"], foot_center, foot_w, foot_h, -foot_tilt[(idx + 4) % 8] + body_angle * 0.10)
        elif animation in {"attack_down", "dash_startup", "jump", "land_hard"}:
            # These poses read best when they inherit the same forward-knee
            # baseline as the improved walk/run cycle.  Use explicit ankle
            # targets so the player-right leg never flips its knee backward.
            foot_w = 12 * S
            foot_h = 6 * S
            if animation == "attack_down":
                idx = frame_index % 8
                far_ax = (-6.2, -6.5, -6.8, -7.2, -7.1, -6.8, -6.5, -6.2)
                far_ay = (23.5, 23.8, 24.0, 24.2, 24.1, 23.9, 23.7, 23.5)
                near_ax = (5.2, 5.8, 6.5, 7.2, 7.4, 6.9, 6.0, 5.4)
                near_ay = (23.0, 23.1, 23.3, 23.5, 23.7, 23.6, 23.3, 23.1)
                far_shift = (-1.7, -1.6, -1.4, -1.2, -1.1, -1.2, -1.4, -1.6)
                near_shift = (1.1, 1.2, 1.2, 1.3, 1.4, 1.3, 1.2, 1.1)
                foot_tilt_far = (-5, -5, -4, -3, -3, -4, -5, -5)
                foot_tilt_near = (-3, -2, -1, 0, 1, 0, -1, -2)
            elif animation == "dash_startup":
                idx = frame_index % 4
                far_ax = (-7.8, -8.2, -8.8, -9.4)
                far_ay = (23.2, 23.6, 23.9, 24.2)
                near_ax = (7.0, 6.4, 5.8, 5.1)
                near_ay = (22.9, 23.1, 23.4, 23.7)
                far_shift = (-1.6, -1.5, -1.3, -1.1)
                near_shift = (1.5, 1.2, 0.8, 0.5)
                foot_tilt_far = (-5, -5, -4, -4)
                foot_tilt_near = (-4, -4, -3, -3)
            elif animation == "jump":
                idx = frame_index % 6
                far_ax = (-7.0, -6.2, -7.8, -3.4, -1.8, -2.6)
                far_ay = (24.0, 24.6, 22.8, 18.2, 17.2, 18.4)
                near_ax = (6.0, 5.2, 7.2, 3.6, 4.6, 3.2)
                near_ay = (23.5, 24.1, 22.2, 17.8, 18.0, 18.6)
                far_shift = (-1.5, -1.4, -1.0, -0.4, -0.2, -0.6)
                near_shift = (1.2, 1.0, 1.4, 0.8, 1.0, 0.8)
                foot_tilt_far = (-5, -6, -4, -2, -1, -2)
                foot_tilt_near = (-4, -5, -3, -1, -1, -1)
            else:  # land_hard
                idx = frame_index % 8
                far_ax = (-6.0, -5.7, -5.5, -5.6, -5.9, -6.3, -6.7, -6.9)
                far_ay = (23.0, 23.6, 24.1, 23.9, 23.7, 23.5, 23.3, 23.1)
                near_ax = (5.0, 5.4, 5.8, 5.6, 5.2, 5.0, 5.2, 5.5)
                near_ay = (22.8, 23.3, 23.8, 23.6, 23.4, 23.2, 23.0, 22.8)
                far_shift = (-1.5, -1.3, -1.2, -1.2, -1.3, -1.4, -1.5, -1.6)
                near_shift = (1.0, 1.1, 1.2, 1.2, 1.1, 1.0, 1.0, 1.1)
                foot_tilt_far = (-5, -4, -3, -3, -4, -4, -5, -5)
                foot_tilt_near = (-4, -3, -2, -2, -3, -3, -4, -4)

            ankle = (body_center[0] + far_ax[idx] * S, body_center[1] + far_ay[idx] * S)
            knee, _a1, _a2 = self._solve_leg_ik(hip_far, ankle, spec.leg_upper * S, spec.leg_lower * S, bend_sign=1.0)
            foot_center = (ankle[0] + (foot_w * 0.34) + far_shift[idx] * S, ankle[1] + 2.0 * S)
            player_right_leg = (hip_far, knee, ankle, pal["shell_side"], foot_center, foot_w, foot_h, foot_tilt_far[idx] + body_angle * 0.10)

            ankle = (body_center[0] + near_ax[idx] * S, body_center[1] + near_ay[idx] * S)
            knee, _a1, _a2 = self._solve_leg_ik(hip_near, ankle, spec.leg_upper * S, spec.leg_lower * S, bend_sign=1.0)
            foot_center = (ankle[0] + (foot_w * 0.34) + near_shift[idx] * S, ankle[1] + 2.0 * S)
            player_left_leg = (hip_near, knee, ankle, pal["shell"], foot_center, foot_w, foot_h, foot_tilt_near[idx] + body_angle * 0.10)
        else:
            # Keep the non-walk geometry the same, but assign explicit semantic
            # labels so the draw order is unambiguous.
            knee, ankle = self._leg_chain(hip_far, spec.leg_upper * S, spec.leg_lower * S, p.far_leg_upper, p.far_leg_lower)
            foot_w = 12 * S
            foot_h = 6 * S
            foot_center = (ankle[0] + (foot_w * 0.34) - 2.0 * S, min(ground_y - 2 * S, ankle[1] + 2 * S))
            player_right_leg = (hip_far, knee, ankle, pal["shell_side"], foot_center, foot_w, foot_h, -4 + body_angle * 0.10)

            knee, ankle = self._leg_chain(hip_near, spec.leg_upper * S, spec.leg_lower * S, p.near_leg_upper, p.near_leg_lower)
            foot_center = (ankle[0] + (foot_w * 0.34) + 3.0 * S, min(ground_y - 2 * S, ankle[1] + 2 * S))
            player_left_leg = (hip_near, knee, ankle, pal["shell"], foot_center, foot_w, foot_h, -4 + body_angle * 0.10)

        # Explicit semantic limb mapping for the right-facing player view.
        player_left_arm = (shoulder_near, p.near_arm_upper, p.near_arm_lower, pal["shell_side"])
        player_right_arm = (shoulder_far, p.far_arm_upper, p.far_arm_lower, pal["shell"])

        # Desired z-order, from back to front:
        #   player-left-arm, player-left-leg, pelvis, player-right-leg,
        #   torso, head, player-right-arm.
        self._draw_robot_arm(character_img, character_draw, player_left_arm[0], player_left_arm[1], player_left_arm[2], player_left_arm[3], spec, pal, S, outline, p.slash, p.slash_arc, p.slash_dir)

        if player_left_leg is not None:
            hip, knee, ankle, tint, foot_center, foot_w, foot_h, foot_angle = player_left_leg
            draw_capsule(character_draw, hip, knee, 2.9 * S, tint, pal["outline"], outline * 0.65)
            draw_capsule(character_draw, knee, ankle, 2.7 * S, tint, pal["outline"], outline * 0.65)
            draw_rotated_rounded_rect(character_img, foot_center, (foot_w, foot_h), foot_angle, 3.0 * S, tint, pal["outline"], outline * 0.7)

        if pelvis_draw is not None:
            pelvis_center, pelvis_w, pelvis_h = pelvis_draw
            draw_rotated_rounded_rect(character_img, pelvis_center, (pelvis_w, pelvis_h), body_angle * 0.25, 3.0 * S, pal["shell_side"], pal["outline"], outline * 0.7)

        if player_right_leg is not None:
            hip, knee, ankle, tint, foot_center, foot_w, foot_h, foot_angle = player_right_leg
            draw_capsule(character_draw, hip, knee, 2.9 * S, tint, pal["outline"], outline * 0.65)
            draw_capsule(character_draw, knee, ankle, 2.7 * S, tint, pal["outline"], outline * 0.65)
            draw_rotated_rounded_rect(character_img, foot_center, (foot_w, foot_h), foot_angle, 3.0 * S, tint, pal["outline"], outline * 0.7)

        draw_rotated_rounded_rect(character_img, body_center, (spec.body_w * S, spec.body_h * S), body_angle, 7 * S, pal["shell"], pal["outline"], outline)
        draw_rotated_rounded_rect(character_img, (body_center[0] + 3 * S, body_center[1] - 1 * S), (10 * S, 9 * S), body_angle, 2.5 * S, pal["accent"], pal["outline"], outline * 0.45)

        # Archetype accessories sit over the base body but under the front arm/head.
        self._draw_archetype_accessories(character_img, character_draw, spec, pal, S, root_x, ground_y, body_center, head_center)

        self._draw_rigid_head(character_img, head_center, spec, pal, S, head_angle, p.blink, p.eye_squint, p.dead, p.head_look)
        self._draw_robot_arm(character_img, character_draw, player_right_arm[0], player_right_arm[1], player_right_arm[2], player_right_arm[3], spec, pal, S, outline)

        if abs(p.whole_body_rotation) > 1e-4:
            roll_center = (body_center[0] + 2.0 * S, body_center[1] + 4.0 * S)
            character_img = character_img.rotate(p.whole_body_rotation, resample=RESAMPLING.BICUBIC, center=roll_center)

        if animation in {"blink_out", "blink_in"}:
            self._composite_teleport_actor(img, character_img, animation, frame_index, frame_count, S)
        else:
            img.alpha_composite(character_img)

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
