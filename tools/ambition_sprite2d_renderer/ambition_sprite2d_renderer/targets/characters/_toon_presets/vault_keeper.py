"""Toon archetype preset: vault_keeper.

One archetype per file so adding / editing a single character
doesn't require touching the shared preset module. The
``_toon_presets/__init__.py`` collects every ``PRESET`` exported
here into a single dict for the rig to consume.

See GOALS.md goal #1.
"""

from __future__ import annotations


PRESET = {
    "name": "Vault Keeper",
    "role": "npc",
    "palette_name": "keeper",
    "body_plan": "broad",
    "outfit": "keeper_robe",
    "hair_style": "crest",
    "prop": "ledger",
    "accessory": "keys",
    "head_w": 26.5,
    "head_h": 30.0,
    "chin_h": 7.0,
    "neck_h": 4.2,
    "shoulder_w": 35.0,
    "torso_w": 29.5,
    "torso_h": 32.5,
    "hip_w": 26.5,
    "arm_upper": 14.0,
    "arm_lower": 13.0,
    "arm_radius": 3.4,
    "leg_upper": 15.0,
    "leg_lower": 14.0,
    "leg_radius": 3.3,
    "hand_r": 3.5,
    "foot_w": 13.0,
    "foot_h": 5.0,
    "coat_len": 22.0,
    "cape_len": 16.0,
    "hair_volume": 5.0,
    "nose_len": 3.2,
    "satchel_size": 0.0,
}
