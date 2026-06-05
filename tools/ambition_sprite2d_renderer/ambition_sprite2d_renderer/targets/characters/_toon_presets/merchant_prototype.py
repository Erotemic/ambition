"""Toon archetype preset: merchant_prototype.

One archetype per file so adding / editing a single character
doesn't require touching the shared preset module. The
``_toon_presets/__init__.py`` collects every ``PRESET`` exported
here into a single dict for the rig to consume.

See GOALS.md goal #1.
"""

from __future__ import annotations


PRESET = {
    "name": "Merchant Prototype",
    "role": "npc",
    "palette_name": "merchant",
    "body_plan": "round",
    "outfit": "apron",
    "hair_style": "cap",
    "prop": "coin_pouch",
    "accessory": "satchel",
    "head_w": 24.5,
    "head_h": 27.0,
    "chin_h": 5.5,
    "neck_h": 3.0,
    "shoulder_w": 25.5,
    "torso_w": 31.5,
    "torso_h": 28.0,
    "hip_w": 27.0,
    "arm_upper": 12.0,
    "arm_lower": 11.2,
    "arm_radius": 3.35,
    "leg_upper": 10.5,
    "leg_lower": 9.8,
    "leg_radius": 3.35,
    "hand_r": 3.2,
    "foot_w": 12.4,
    "foot_h": 4.7,
    "coat_len": 8.0,
    "cape_len": 0.0,
    "hair_volume": 5.0,
    "nose_len": 3.0,
    "satchel_size": 10.0,
}
