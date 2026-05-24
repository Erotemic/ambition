"""Toon archetype preset: fascist_enforcer.

One archetype per file so adding / editing a single character
doesn't require touching the shared preset module. The
``_toon_presets/__init__.py`` collects every ``PRESET`` exported
here into a single dict for the rig to consume.

See GOALS.md goal #1.
"""
from __future__ import annotations


PRESET = {
        "name": "Fascist Enforcer",
        "role": "enemy",
        "palette_name": "fascist",
        "body_plan": "rigid",
        "outfit": "storm_uniform",
        "hair_style": "officer_cap",
        "prop": "rifle",
        "accessory": "none",
        "head_w": 28.5,
        "head_h": 29.0,
        "chin_h": 7.2,
        "neck_h": 3.6,
        "shoulder_w": 34.5,
        "torso_w": 27.5,
        "torso_h": 31.5,
        "hip_w": 22.0,
        "arm_upper": 13.0,
        "arm_lower": 12.0,
        "arm_radius": 3.2,
        "leg_upper": 15.0,
        "leg_lower": 14.0,
        "leg_radius": 3.1,
        "hand_r": 3.2,
        "foot_w": 12.5,
        "foot_h": 4.9,
        "coat_len": 14.0,
        "cape_len": 0.0,
        "hair_volume": 2.6,
        "nose_len": 3.8,
        "satchel_size": 0.0,
    }
