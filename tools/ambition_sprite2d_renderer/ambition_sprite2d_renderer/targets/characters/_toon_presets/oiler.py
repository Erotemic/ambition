"""Toon archetype preset: oiler.

One archetype per file so adding / editing a single character
doesn't require touching the shared preset module. The
``_toon_presets/__init__.py`` collects every ``PRESET`` exported
here into a single dict for the rig to consume.

See GOALS.md goal #1.
"""

from __future__ import annotations


PRESET = {
    "name": "Oiler",
    "role": "npc",
    "palette_name": "oiler",
    "body_plan": "broad",
    "outfit": "banyan",
    "hair_style": "savant_cap",
    "prop": "wrench",
    "accessory": "satchel",
    "head_w": 26.0,
    "head_h": 28.5,
    "chin_h": 6.5,
    "neck_h": 3.4,
    "shoulder_w": 32.0,
    "torso_w": 31.0,
    "torso_h": 28.5,
    "hip_w": 26.0,
    "arm_upper": 12.5,
    "arm_lower": 12.0,
    "arm_radius": 3.2,
    "leg_upper": 12.0,
    "leg_lower": 11.0,
    "leg_radius": 3.2,
    "hand_r": 3.3,
    "foot_w": 12.8,
    "foot_h": 5.0,
    "coat_len": 14.0,
    "cape_len": 0.0,
    "hair_volume": 4.0,
    "nose_len": 3.1,
    "satchel_size": 9.0,
}
