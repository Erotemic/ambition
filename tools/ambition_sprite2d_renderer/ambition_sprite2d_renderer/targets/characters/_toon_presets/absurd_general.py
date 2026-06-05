"""Toon archetype preset: absurd_general.

One archetype per file so adding / editing a single character
doesn't require touching the shared preset module. The
``_toon_presets/__init__.py`` collects every ``PRESET`` exported
here into a single dict for the rig to consume.

See GOALS.md goal #1.
"""

from __future__ import annotations


PRESET = {
    "name": "Absurd General",
    "role": "npc",
    "palette_name": "absurd_general",
    "body_plan": "broad",
    "outfit": "general_uniform",
    "hair_style": "general_hat",
    "prop": "baton",
    "accessory": "medals",
    "head_w": 31.0,
    "head_h": 29.0,
    "chin_h": 9.0,
    "neck_h": 3.5,
    "shoulder_w": 42.0,
    "torso_w": 34.0,
    "torso_h": 31.0,
    "hip_w": 25.0,
    "arm_upper": 12.0,
    "arm_lower": 11.0,
    "arm_radius": 3.5,
    "leg_upper": 12.5,
    "leg_lower": 11.5,
    "leg_radius": 3.4,
    "hand_r": 3.8,
    "foot_w": 13.5,
    "foot_h": 5.3,
    "coat_len": 12.0,
    "cape_len": 0.0,
    "hair_volume": 4.0,
    "nose_len": 4.4,
    "satchel_size": 0.0,
    "hat_brim_offset_x": 0.0,
    "hat_brim_offset_y": -2.0,
}
