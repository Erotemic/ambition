"""Toon archetype preset: walter.

One archetype per file so adding / editing a single character
doesn't require touching the shared preset module. The
``_toon_presets/__init__.py`` collects every ``PRESET`` exported
here into a single dict for the rig to consume.

See GOALS.md goal #1.
"""

from __future__ import annotations


PRESET = {
    "name": "Walter",
    "role": "npc",
    "palette_name": "walter",
    "body_plan": "rigid",
    "outfit": "long_coat",
    "hair_style": "tricorn_hat",
    "prop": "lantern",
    "accessory": "keys",
    "head_w": 26.0,
    "head_h": 28.5,
    "chin_h": 6.6,
    "neck_h": 3.8,
    "shoulder_w": 31.0,
    "torso_w": 24.5,
    "torso_h": 30.0,
    "hip_w": 22.0,
    "arm_upper": 13.5,
    "arm_lower": 13.0,
    "arm_radius": 3.0,
    "leg_upper": 15.5,
    "leg_lower": 14.5,
    "leg_radius": 3.0,
    "hand_r": 3.1,
    "foot_w": 12.0,
    "foot_h": 4.8,
    "coat_len": 16.0,
    "cape_len": 0.0,
    "hair_volume": 4.5,
    "nose_len": 3.5,
    "satchel_size": 0.0,
}
