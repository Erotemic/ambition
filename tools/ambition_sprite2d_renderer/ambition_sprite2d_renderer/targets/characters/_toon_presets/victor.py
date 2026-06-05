"""Toon archetype preset: victor.

One archetype per file so adding / editing a single character
doesn't require touching the shared preset module. The
``_toon_presets/__init__.py`` collects every ``PRESET`` exported
here into a single dict for the rig to consume.

See GOALS.md goal #1.
"""

from __future__ import annotations


PRESET = {
    "name": "Victor",
    "role": "npc",
    "palette_name": "victor",
    "body_plan": "rigid",
    "outfit": "jacket",
    "hair_style": "square_fringe",
    "prop": "magnifier",
    "accessory": "scarf",
    "head_w": 24.5,
    "head_h": 28.0,
    "chin_h": 6.4,
    "neck_h": 3.6,
    "shoulder_w": 28.0,
    "torso_w": 22.0,
    "torso_h": 28.0,
    "hip_w": 20.0,
    "arm_upper": 13.5,
    "arm_lower": 13.0,
    "arm_radius": 2.8,
    "leg_upper": 15.5,
    "leg_lower": 14.5,
    "leg_radius": 2.9,
    "hand_r": 3.0,
    "foot_w": 11.5,
    "foot_h": 4.6,
    "coat_len": 8.0,
    "cape_len": 0.0,
    "hair_volume": 4.5,
    "nose_len": 3.2,
    "satchel_size": 0.0,
}
