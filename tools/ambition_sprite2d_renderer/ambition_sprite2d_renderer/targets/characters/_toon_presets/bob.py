"""Toon archetype preset: bob.

One archetype per file so adding / editing a single character
doesn't require touching the shared preset module. The
``_toon_presets/__init__.py`` collects every ``PRESET`` exported
here into a single dict for the rig to consume.

See GOALS.md goal #1.
"""

from __future__ import annotations


PRESET = {
    "name": "Bob",
    "role": "npc",
    "palette_name": "bob",
    "body_plan": "broad",
    "outfit": "vest_over_shirt",
    "hair_style": "tousled_crop",
    "prop": "key_ring",
    "accessory": "satchel",
    "head_w": 26.5,
    "head_h": 28.0,
    "chin_h": 6.0,
    "neck_h": 3.6,
    "shoulder_w": 36.0,
    "torso_w": 30.0,
    "torso_h": 28.0,
    "hip_w": 25.0,
    "arm_upper": 13.0,
    "arm_lower": 12.5,
    "arm_radius": 3.3,
    "leg_upper": 14.0,
    "leg_lower": 13.0,
    "leg_radius": 3.2,
    "hand_r": 3.4,
    "foot_w": 13.0,
    "foot_h": 5.0,
    "coat_len": 6.0,
    "cape_len": 0.0,
    "hair_volume": 4.5,
    "nose_len": 3.0,
    "satchel_size": 7.5,
}
