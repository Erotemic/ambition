"""Toon archetype preset: architect.

One archetype per file so adding / editing a single character
doesn't require touching the shared preset module. The
``_toon_presets/__init__.py`` collects every ``PRESET`` exported
here into a single dict for the rig to consume.

See GOALS.md goal #1.
"""

from __future__ import annotations


PRESET = {
    "name": "Architect",
    "role": "npc",
    "palette_name": "architect",
    "body_plan": "tall",
    "outfit": "long_coat",
    "hair_style": "bob",
    "prop": "blueprint",
    "accessory": "sash",
    "head_w": 24.0,
    "head_h": 29.0,
    "chin_h": 6.4,
    "neck_h": 4.0,
    "shoulder_w": 22.5,
    "torso_w": 18.5,
    "torso_h": 29.0,
    "hip_w": 16.5,
    "arm_upper": 14.2,
    "arm_lower": 14.0,
    "arm_radius": 2.4,
    "leg_upper": 18.8,
    "leg_lower": 17.2,
    "leg_radius": 2.7,
    "hand_r": 2.9,
    "foot_w": 11.0,
    "foot_h": 4.2,
    "coat_len": 18.0,
    "cape_len": 0.0,
    "hair_volume": 6.6,
    "nose_len": 3.2,
    "satchel_size": 3.0,
}
