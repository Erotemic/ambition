"""Toon archetype preset: mallory.

One archetype per file so adding / editing a single character
doesn't require touching the shared preset module. The
``_toon_presets/__init__.py`` collects every ``PRESET`` exported
here into a single dict for the rig to consume.

See GOALS.md goal #1.
"""

from __future__ import annotations


PRESET = {
    "feminine_coded": True,
    "name": "Mallory",
    "role": "npc",
    "palette_name": "mallory",
    # `hero` instead of `rigid` so the male tactical-hulk
    # read disappears.
    "body_plan": "hero",
    # `cinched_field_jacket` adds a visible belt at the
    # waist that the plain field_jacket doesn't have.
    "outfit": "cinched_field_jacket",
    # `forward_braid` drops the braid OVER the front
    # shoulder where the camera sees it.
    "hair_style": "forward_braid",
    "prop": "tablet",
    "accessory": "medals",
    "head_w": 23.0,
    "head_h": 27.0,
    "chin_h": 5.6,
    "neck_h": 3.4,
    # Narrow tactical-athlete shoulders, defined waist.
    "shoulder_w": 22.0,
    "torso_w": 18.0,
    "torso_h": 27.0,
    "hip_w": 20.0,
    "arm_upper": 13.5,
    "arm_lower": 13.0,
    "arm_radius": 2.6,
    "leg_upper": 17.0,
    "leg_lower": 16.0,
    "leg_radius": 2.8,
    "hand_r": 2.9,
    "foot_w": 11.0,
    "foot_h": 4.4,
    "coat_len": 8.0,
    "cape_len": 0.0,
    "hair_volume": 6.5,
    "nose_len": 2.6,
    "satchel_size": 0.0,
}
