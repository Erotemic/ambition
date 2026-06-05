"""Toon archetype preset: judy.

One archetype per file so adding / editing a single character
doesn't require touching the shared preset module. The
``_toon_presets/__init__.py`` collects every ``PRESET`` exported
here into a single dict for the rig to consume.

See GOALS.md goal #1.
"""

from __future__ import annotations


PRESET = {
    "name": "Judy",
    "role": "npc",
    "palette_name": "judy",
    "body_plan": "broad",
    "outfit": "judicial_robe",
    "hair_style": "barrister_wig",
    "prop": "gavel",
    "accessory": "jabot_collar",
    "head_w": 25.0,
    "head_h": 27.5,
    "chin_h": 6.4,
    "neck_h": 3.4,
    "shoulder_w": 33.0,
    "torso_w": 27.0,
    "torso_h": 30.0,
    "hip_w": 25.0,
    "arm_upper": 13.0,
    "arm_lower": 12.5,
    "arm_radius": 3.2,
    "leg_upper": 13.5,
    "leg_lower": 12.5,
    "leg_radius": 3.2,
    "hand_r": 3.3,
    "foot_w": 12.5,
    "foot_h": 4.8,
    "coat_len": 22.0,
    "cape_len": 0.0,
    "hair_volume": 7.5,
    "nose_len": 3.0,
    "satchel_size": 0.0,
}
