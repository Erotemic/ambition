"""Toon archetype preset: trudy.

One archetype per file so adding / editing a single character
doesn't require touching the shared preset module. The
``_toon_presets/__init__.py`` collects every ``PRESET`` exported
here into a single dict for the rig to consume.

See GOALS.md goal #1.
"""

from __future__ import annotations


PRESET = {
    "feminine_coded": True,
    "name": "Trudy",
    "role": "npc",
    "palette_name": "trudy",
    "body_plan": "hero",
    "outfit": "field_jacket",
    # Ponytail (not tousled crop) — visible long hair is the
    # primary feminine silhouette cue. Field jacket + lockpick
    # carry over.
    "hair_style": "ponytail",
    "prop": "lockpick",
    "accessory": "satchel",
    "head_w": 23.0,
    "head_h": 26.5,
    "chin_h": 5.6,
    "neck_h": 3.4,
    # Narrower shoulders + slightly wider hips so the
    # silhouette reads feminine. Was 23 / 19; now 19 / 20.
    "shoulder_w": 19.0,
    "torso_w": 16.5,
    "torso_h": 26.0,
    "hip_w": 20.0,
    "arm_upper": 13.0,
    "arm_lower": 12.6,
    "arm_radius": 2.4,
    "leg_upper": 16.5,
    "leg_lower": 15.5,
    "leg_radius": 2.8,
    "hand_r": 2.8,
    "foot_w": 10.8,
    "foot_h": 4.4,
    "coat_len": 6.0,
    "cape_len": 0.0,
    "hair_volume": 6.0,
    "nose_len": 2.4,
    "satchel_size": 6.5,
}
