"""Toon archetype preset: olivia.

One archetype per file so adding / editing a single character
doesn't require touching the shared preset module. The
``_toon_presets/__init__.py`` collects every ``PRESET`` exported
here into a single dict for the rig to consume.

See GOALS.md goal #1.
"""

from __future__ import annotations


PRESET = {
    "feminine_coded": True,
    "name": "Olivia",
    "role": "npc",
    "palette_name": "olivia",
    "body_plan": "tall",
    "outfit": "keeper_robe",
    "hair_style": "veiled",
    "prop": "none",
    "accessory": "shawl",
    "head_w": 23.0,
    "head_h": 28.5,
    "chin_h": 5.8,
    "neck_h": 4.0,
    # Slender feminine frame — narrow shoulders, slightly
    # wider hips (was 21 / 16.5, which read as upside-down
    # triangle / masculine).
    "shoulder_w": 17.0,
    "torso_w": 14.0,
    "torso_h": 28.0,
    "hip_w": 18.0,
    "arm_upper": 14.0,
    "arm_lower": 13.5,
    "arm_radius": 2.3,
    "leg_upper": 18.5,
    "leg_lower": 17.0,
    "leg_radius": 2.6,
    "hand_r": 2.8,
    "foot_w": 10.5,
    "foot_h": 4.2,
    "coat_len": 22.0,
    "cape_len": 12.0,
    "hair_volume": 6.0,
    "nose_len": 2.4,
    "satchel_size": 0.0,
}
