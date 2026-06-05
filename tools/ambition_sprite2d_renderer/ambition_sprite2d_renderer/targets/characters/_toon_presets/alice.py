"""Toon archetype preset: alice.

One archetype per file so adding / editing a single character
doesn't require touching the shared preset module. The
``_toon_presets/__init__.py`` collects every ``PRESET`` exported
here into a single dict for the rig to consume.

See GOALS.md goal #1.
"""

from __future__ import annotations


PRESET = {
    "feminine_coded": True,
    "name": "Alice",
    "role": "npc",
    "palette_name": "alice",
    "body_plan": "hero",
    # `cinched_tabard` adds a visible waist seam + lower
    # hip flare that the plain `tabard` doesn't have.
    "outfit": "cinched_tabard",
    # `long_side_braid` is hair that visibly hangs OVER the
    # front of her shoulder, not a tiny bun at the back.
    "hair_style": "long_side_braid",
    "prop": "cipher_scroll",
    "accessory": "scarf",
    # Head a touch smaller + cheekbones from chin_h shift.
    "head_w": 23.5,
    "head_h": 27.5,
    "chin_h": 5.6,
    "neck_h": 3.6,
    # Narrow shoulders + slightly wider hips than the male
    # hero. Torso slightly shorter so the cinched waist sits
    # high. Arms longer + thinner.
    "shoulder_w": 19.0,
    "torso_w": 16.5,
    "torso_h": 26.0,
    "hip_w": 19.5,
    "arm_upper": 14.0,
    "arm_lower": 13.6,
    "arm_radius": 2.2,
    "leg_upper": 18.0,
    "leg_lower": 17.0,
    "leg_radius": 2.6,
    "hand_r": 2.8,
    "foot_w": 10.5,
    "foot_h": 4.2,
    "coat_len": 14.0,
    "cape_len": 0.0,
    "hair_volume": 7.5,
    "nose_len": 2.4,
    "satchel_size": 0.0,
}
