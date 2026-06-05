"""Toon archetype preset: peggy.

One archetype per file so adding / editing a single character
doesn't require touching the shared preset module. The
``_toon_presets/__init__.py`` collects every ``PRESET`` exported
here into a single dict for the rig to consume.

See GOALS.md goal #1.
"""

from __future__ import annotations


PRESET = {
    "feminine_coded": True,
    "name": "Peggy",
    "role": "npc",
    "palette_name": "peggy",
    "body_plan": "hero",
    "outfit": "jacket",
    "hair_style": "ponytail",
    "prop": "long_pointer",
    "accessory": "scarf",
    "head_w": 24.0,
    "head_h": 28.0,
    "chin_h": 5.8,
    "neck_h": 3.6,
    # Athletic but feminine — shoulders narrower than hips.
    # Was 24 / 21; now 20 / 22.
    "shoulder_w": 20.0,
    "torso_w": 17.0,
    "torso_h": 27.0,
    "hip_w": 22.0,
    "arm_upper": 13.5,
    "arm_lower": 13.0,
    "arm_radius": 2.5,
    "leg_upper": 17.0,
    "leg_lower": 16.0,
    "leg_radius": 2.7,
    "hand_r": 2.9,
    "foot_w": 11.0,
    "foot_h": 4.4,
    "coat_len": 8.0,
    "cape_len": 0.0,
    "hair_volume": 6.5,
    "nose_len": 2.6,
    "satchel_size": 0.0,
}
