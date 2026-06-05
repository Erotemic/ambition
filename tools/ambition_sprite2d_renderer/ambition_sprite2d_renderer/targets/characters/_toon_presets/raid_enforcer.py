"""Toon archetype preset: raid_enforcer.

Antagonist preset. Includes a `pose_override` callable that adjusts
the base pose per-animation to read as "rigid, stiff military
posture" — the rig doesn't have any archetype-specific branches;
all per-archetype touch-ups land here.

See GOALS.md goal #1.
"""

from __future__ import annotations

from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from ..toon_side import ToonPose


def _override_pose(p: "ToonPose", animation: str) -> None:
    """Stiffer, more upright pose for the raid_enforcer.

    Called from `ToonSideGenerator.pose_for_animation` AFTER the base
    pose is built. Mutates in place — return value ignored. Each
    `elif` is a per-animation tweak; default (no clause) leaves the
    pose unchanged.
    """
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


PRESET = {
    "name": "Raid Enforcer",
    "role": "enemy",
    "palette_name": "raid_enforcer",
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
    "pose_override": _override_pose,
}
