"""Toon archetype preset: craig.

One archetype per file so adding / editing a single character
doesn't require touching the shared preset module. The
``_toon_presets/__init__.py`` collects every ``PRESET`` exported
here into a single dict for the rig to consume.

See GOALS.md goal #1.
"""

from __future__ import annotations


PRESET = {
    "name": "Craig",
    "role": "npc",
    "palette_name": "craig",
    "body_plan": "tall",
    "outfit": "apron",
    "hair_style": "wide_brim_hat",
    "prop": "stethoscope",
    "accessory": "satchel",
    "head_w": 23.5,
    "head_h": 28.0,
    "chin_h": 7.0,
    "neck_h": 4.4,
    "shoulder_w": 22.0,
    "torso_w": 17.5,
    "torso_h": 29.0,
    "hip_w": 16.0,
    "arm_upper": 15.0,
    "arm_lower": 14.5,
    "arm_radius": 2.2,
    "leg_upper": 19.5,
    "leg_lower": 18.0,
    "leg_radius": 2.5,
    "hand_r": 2.7,
    "foot_w": 10.5,
    "foot_h": 4.2,
    "coat_len": 6.0,
    "cape_len": 0.0,
    "hair_volume": 4.0,
    "nose_len": 3.4,
    "satchel_size": 7.5,
}
