from __future__ import annotations

"""Cute side-scroller robot target.

This target intentionally keeps the old 2.5D head / shell rendering from the
robot25d prototype, but re-stages the proportions and animation set for a
side-scrolling game. The character should read as a left-facing profile rather
than a front-facing over-the-shoulder look.
"""

import math
import random
from typing import Dict, Tuple

from .robot25d import BotSpec, Pose, Robot25DGenerator, clamp, ease_in_out_sine


class SideRobotGenerator(Robot25DGenerator):
    name = "robot"

    ARCHETYPES = {
        "default": {},
        "cute_scout": {"head_w": 2, "head_h": 2, "body_h": -1, "body_w": -1, "char_yaw": -16},
        "runner": {"leg_upper": 2, "leg_lower": 1, "arm_upper": 1, "char_yaw": -18},
        "guardian": {"body_w": 2, "body_h": 1, "char_yaw": -15},
    }

    ANIMATIONS: Dict[str, Dict[str, int]] = {
        "idle": {"frames": 8, "duration_ms": 120},
        "walk": {"frames": 8, "duration_ms": 95},
        "run": {"frames": 8, "duration_ms": 75},
        "jump": {"frames": 6, "duration_ms": 95},
        "fall": {"frames": 6, "duration_ms": 95},
        "slash": {"frames": 8, "duration_ms": 75},
        "hit": {"frames": 5, "duration_ms": 90},
        "death": {"frames": 8, "duration_ms": 110},
    }

    def sample_spec(self, seed: int, archetype: str = "default") -> BotSpec:
        rng = random.Random(seed)
        tweaks = self.ARCHETYPES.get(archetype, self.ARCHETYPES["default"])

        def tweak(name: str, default: float = 0.0) -> float:
            return float(tweaks.get(name, default))

        # Keep the familiar robot head from the old 2.5D prototype, but make the
        # whole silhouette cuter and more side-scroller friendly.
        return BotSpec(
            target=self.name,
            seed=seed,
            archetype=archetype,
            palette_name="classic",
            head_w=rng.randint(40, 43) + tweak("head_w"),
            head_h=rng.randint(33, 36) + tweak("head_h"),
            head_d=rng.randint(18, 20),
            head_round=rng.randint(8, 10),
            body_w=rng.randint(23, 26) + tweak("body_w"),
            body_h=rng.randint(22, 24) + tweak("body_h"),
            body_d=rng.randint(14, 16),
            body_round=rng.randint(6, 8),
            shoulder_span=rng.randint(21, 24),
            hip_span=rng.randint(12, 14),
            arm_upper=rng.randint(12, 14) + tweak("arm_upper"),
            arm_lower=rng.randint(10, 12),
            leg_upper=rng.randint(11, 13) + tweak("leg_upper"),
            leg_lower=rng.randint(10, 11) + tweak("leg_lower"),
            arm_z_back=-1.0,
            arm_z_front=3.8,
            leg_z_back=-2.2,
            leg_z_front=3.1,
            arm_width=rng.uniform(4.0, 4.6),
            leg_width=rng.uniform(4.5, 5.0),
            hand_r=rng.uniform(4.2, 4.9),
            joint_r=rng.uniform(3.2, 3.8),
            foot_w=rng.uniform(10.2, 12.4),
            foot_h=rng.uniform(5.8, 6.7),
            foot_d=rng.uniform(8.4, 9.5),
            visor_w=rng.uniform(22.0, 24.0),
            visor_h=rng.uniform(12.2, 13.8),
            visor_round=rng.uniform(4.2, 5.0),
            visor_inset_y=rng.uniform(0.0, 1.0),
            eye_w=rng.uniform(4.5, 5.4),
            eye_h=rng.uniform(8.6, 9.8),
            eye_gap=rng.uniform(7.2, 8.2),
            antenna_h=rng.uniform(13.0, 15.0),
            antenna_ball=rng.uniform(3.0, 3.7),
            chest_w=rng.uniform(7.5, 9.0),
            chest_h=rng.uniform(7.2, 9.0),
            chest_style="screen",
            mood="happy",
            blade_len=rng.uniform(27.0, 32.0),
            panel_w=rng.uniform(18.0, 22.0),
            panel_h=rng.uniform(18.0, 22.0),
            # Negative yaw = show the left side and face to the left.
            char_yaw=tweak("char_yaw", -16.0),
            char_pitch=-0.6,
            head_yaw=rng.uniform(-8.0, -5.0),
            head_pitch=rng.uniform(-5.2, -3.2),
        )

    def pose_for_animation(self, animation: str, frame_index: int, frame_count: int) -> Pose:
        if animation in {"idle", "walk", "slash", "death"}:
            return super().pose_for_animation(animation, frame_index, frame_count)
        if animation == "hit":
            return super().pose_for_animation("hurt", frame_index, frame_count)

        p = Pose()
        t = 0.0 if frame_count <= 1 else frame_index / float(frame_count - 1)

        if animation == "run":
            stride = math.sin(t * math.tau)
            bounce = (1.0 - math.cos(t * math.tau * 2.0)) * 0.5
            left_forward = -stride
            right_forward = stride
            left_lift = max(0.0, left_forward)
            right_lift = max(0.0, right_forward)
            left_push = max(0.0, -left_forward)
            right_push = max(0.0, -right_forward)

            p.root_x = stride * 1.6
            p.body_bob = 0.8 + bounce * 2.0
            p.root_tilt = -8.0 - stride * 3.5
            p.head_pitch_delta = -1.8 - bounce * 1.4
            p.head_yaw_delta = -1.2 + stride * 1.0

            p.left_arm_upper = 116 + stride * 20
            p.left_arm_lower = 102 + stride * 11
            p.right_arm_upper = 60 - stride * 20
            p.right_arm_lower = 74 - stride * 11

            p.left_leg_upper = 116 - left_forward * 30 + left_lift * 6
            p.left_leg_lower = 102 - left_lift * 28 + left_push * 16
            p.right_leg_upper = 68 - right_forward * 30 + right_lift * 6
            p.right_leg_lower = 84 - right_lift * 28 + right_push * 16
            p.eye_squint = 0.12 + bounce * 0.14
        elif animation == "jump":
            arc = math.sin(t * math.pi)
            lift = ease_in_out_sine(arc)
            p.root_y = -18.0 * lift
            p.root_x = 2.0 * t
            p.body_bob = 0.5 * lift
            p.root_tilt = -5.0 + 2.0 * t
            p.head_pitch_delta = -2.0 - 2.0 * lift
            p.left_arm_upper = 130 - 14 * lift
            p.left_arm_lower = 98 - 8 * lift
            p.right_arm_upper = 56 + 18 * lift
            p.right_arm_lower = 74 + 16 * lift
            p.left_leg_upper = 128 + 10 * lift
            p.left_leg_lower = 92 - 24 * lift
            p.right_leg_upper = 82 + 22 * lift
            p.right_leg_lower = 70 - 18 * lift
            p.eye_squint = 0.08
        elif animation == "fall":
            fall = t
            p.root_y = -12.0 + fall * 10.0
            p.root_x = 1.0 + 2.0 * t
            p.root_tilt = 4.0 + 5.0 * fall
            p.head_pitch_delta = 2.0 * fall
            p.left_arm_upper = 162 - 10 * fall
            p.left_arm_lower = 152 - 8 * fall
            p.right_arm_upper = 24 + 12 * fall
            p.right_arm_lower = 34 + 10 * fall
            p.left_leg_upper = 150 - 6 * fall
            p.left_leg_lower = 140 - 8 * fall
            p.right_leg_upper = 102 - 4 * fall
            p.right_leg_lower = 118 - 10 * fall
            p.eye_squint = 0.16
        else:
            return super().pose_for_animation("idle", frame_index, frame_count)

        return p
