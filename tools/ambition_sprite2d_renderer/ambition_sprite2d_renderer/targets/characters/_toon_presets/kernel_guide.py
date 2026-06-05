"""Toon archetype preset: kernel_guide.

One archetype per file so adding / editing a single character
doesn't require touching the shared preset module. The
``_toon_presets/__init__.py`` collects every ``PRESET`` exported
here into a single dict for the rig to consume.

See GOALS.md goal #1.
"""

from __future__ import annotations


PRESET = {
    "name": "Kernel Guide",
    "role": "npc",
    "palette_name": "guide",
    "body_plan": "soft",
    "outfit": "poncho",
    "hair_style": "hood",
    "prop": "tablet",
    "accessory": "shawl",
    "head_w": 26.0,
    "head_h": 28.0,
    "chin_h": 6.0,
    "neck_h": 3.0,
    "shoulder_w": 34.0,
    "torso_w": 28.0,
    "torso_h": 24.0,
    "hip_w": 26.5,
    "arm_upper": 11.5,
    "arm_lower": 11.0,
    "arm_radius": 2.7,
    "leg_upper": 10.8,
    "leg_lower": 9.8,
    "leg_radius": 2.9,
    "hand_r": 3.0,
    "foot_w": 11.5,
    "foot_h": 4.5,
    "coat_len": 12.0,
    "cape_len": 18.0,
    "hair_volume": 6.0,
    "nose_len": 2.6,
    "satchel_size": 7.5,
}
