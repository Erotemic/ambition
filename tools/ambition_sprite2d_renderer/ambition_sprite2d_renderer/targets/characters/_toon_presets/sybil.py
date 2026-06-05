"""Toon archetype preset: sybil.

One archetype per file so adding / editing a single character
doesn't require touching the shared preset module. The
``_toon_presets/__init__.py`` collects every ``PRESET`` exported
here into a single dict for the rig to consume.

See GOALS.md goal #1.
"""

from __future__ import annotations


PRESET = {
    "feminine_coded": True,
    "name": "Sybil",
    "role": "npc",
    "palette_name": "sybil",
    "body_plan": "soft",
    "outfit": "poncho",
    "hair_style": "many_braids",
    "prop": "mask_stack",
    "accessory": "shawl",
    "head_w": 24.5,
    "head_h": 27.0,
    "chin_h": 5.8,
    "neck_h": 3.4,
    # Narrower shoulders + hips wider than shoulders.
    # Was 30 / 24 (broad-shouldered). Now 22 / 25.
    "shoulder_w": 22.0,
    "torso_w": 19.0,
    "torso_h": 25.0,
    "hip_w": 25.0,
    "arm_upper": 12.0,
    "arm_lower": 11.5,
    "arm_radius": 2.8,
    "leg_upper": 12.5,
    "leg_lower": 11.5,
    "leg_radius": 2.9,
    "hand_r": 3.0,
    "foot_w": 11.5,
    "foot_h": 4.5,
    "coat_len": 14.0,
    "cape_len": 18.0,
    "hair_volume": 6.5,
    "nose_len": 2.8,
    "satchel_size": 0.0,
}
