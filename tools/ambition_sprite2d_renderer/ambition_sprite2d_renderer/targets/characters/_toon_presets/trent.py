"""Toon archetype preset: trent.

One archetype per file so adding / editing a single character
doesn't require touching the shared preset module. The
``_toon_presets/__init__.py`` collects every ``PRESET`` exported
here into a single dict for the rig to consume.

See GOALS.md goal #1.
"""

from __future__ import annotations


PRESET = {
    "name": "Trent",
    "role": "npc",
    "palette_name": "trent",
    "body_plan": "broad",
    "outfit": "formal_robe",
    "hair_style": "clean_bald",
    "prop": "balance_scales",
    "accessory": "medals",
    "head_w": 26.0,
    "head_h": 28.5,
    "chin_h": 7.0,
    "neck_h": 3.6,
    "shoulder_w": 33.0,
    "torso_w": 28.0,
    "torso_h": 30.0,
    "hip_w": 25.0,
    "arm_upper": 13.0,
    "arm_lower": 12.5,
    "arm_radius": 3.2,
    "leg_upper": 13.0,
    "leg_lower": 12.0,
    "leg_radius": 3.2,
    "hand_r": 3.3,
    "foot_w": 12.5,
    "foot_h": 4.8,
    "coat_len": 22.0,
    "cape_len": 0.0,
    "hair_volume": 3.0,
    "nose_len": 3.4,
    "satchel_size": 0.0,
}
