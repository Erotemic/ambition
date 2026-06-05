"""Toon archetype preset: general_hero.

One archetype per file so adding / editing a single character
doesn't require touching the shared preset module. The
``_toon_presets/__init__.py`` collects every ``PRESET`` exported
here into a single dict for the rig to consume.

See GOALS.md goal #1.
"""

from __future__ import annotations


PRESET = {
    "name": "General Hero",
    "role": "player",
    "palette_name": "hero",
    "body_plan": "hero",
    "outfit": "jacket",
    "hair_style": "swoop",
    "prop": "blade",
    "accessory": "scarf",
    "head_w": 27.0,
    "head_h": 30.0,
    "chin_h": 7.0,
    "neck_h": 4.5,
    "shoulder_w": 25.0,
    "torso_w": 22.0,
    "torso_h": 27.0,
    "hip_w": 18.5,
    "arm_upper": 13.5,
    "arm_lower": 13.0,
    "arm_radius": 2.8,
    "leg_upper": 17.0,
    "leg_lower": 16.0,
    "leg_radius": 3.0,
    "hand_r": 3.3,
    "foot_w": 12.5,
    "foot_h": 4.8,
    "coat_len": 8.0,
    "cape_len": 0.0,
    "hair_volume": 7.5,
    "nose_len": 3.0,
    "satchel_size": 0.0,
}
