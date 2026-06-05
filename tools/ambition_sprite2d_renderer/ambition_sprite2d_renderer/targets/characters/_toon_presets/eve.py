"""Toon archetype preset: eve.

One archetype per file so adding / editing a single character
doesn't require touching the shared preset module. The
``_toon_presets/__init__.py`` collects every ``PRESET`` exported
here into a single dict for the rig to consume.

See GOALS.md goal #1.
"""

from __future__ import annotations


PRESET = {
    "name": "Eve",
    "role": "npc",
    "palette_name": "eve",
    "body_plan": "tall",
    "outfit": "eavesdrop_cloak",
    "hair_style": "hood",
    "prop": "listening_horn",
    "accessory": "satchel",
    "head_w": 23.5,
    "head_h": 28.0,
    "chin_h": 6.0,
    "neck_h": 3.8,
    "shoulder_w": 22.0,
    "torso_w": 18.0,
    "torso_h": 28.0,
    "hip_w": 17.0,
    "arm_upper": 14.0,
    "arm_lower": 13.6,
    "arm_radius": 2.5,
    "leg_upper": 18.5,
    "leg_lower": 17.0,
    "leg_radius": 2.7,
    "hand_r": 2.9,
    "foot_w": 11.0,
    "foot_h": 4.2,
    "coat_len": 20.0,
    "cape_len": 4.0,
    "hair_volume": 5.0,
    "nose_len": 3.2,
    "satchel_size": 6.0,
}
