"""Generate the `noether` rig document (Emmy No-Ether, the symmetry guide).

Reuses the proven `player_robot_fable` skeleton + two-bone-IK legs + idle/walk
clips (so feet plant and the bob/sway read correctly) and replaces only the
palette and the part painters so the silhouette reads as a scholar in an
academic robe: hair bun, round glasses, a long teal coat with a gold lapel and
a white rotational-symmetry sigil. Optional accessories (glasses, sigil, and a
bobbing hairpin) are tagged as toggleable `feature`s; the hairpin is OFF by
default — it read as an antenna sticking off her head. Output is committed
JSON; the PNG sheet is regenerated.

    cd tools/ambition_sprite2d_renderer
    uv run --python 3.12 python gen_noether_rig.py
"""

from __future__ import annotations

import json
from pathlib import Path

HERE = Path(__file__).resolve().parent
PKG = HERE / "ambition_sprite2d_renderer"
TEMPLATE = PKG / "data" / "rig_templates" / "player_robot_fable.rig.json"
OUT = PKG / "targets" / "characters" / "rigged" / "noether.rig.json"

robot = json.loads(TEMPLATE.read_text(encoding="utf8"))

# Reuse the boots verbatim (geometry must match the IK foot bones); recolour.
foot_parts = {p["name"]: p for p in robot["parts"] if p["name"] in ("far_foot", "near_foot")}
far_foot = dict(foot_parts["far_foot"]); far_foot["fill"] = "boot_dark"
near_foot = dict(foot_parts["near_foot"]); near_foot["fill"] = "boot"

PALETTE = {
    "skin": "#E7B391",
    "skin_hi": "#F4D2B6",
    "skin_shade": "#3A1F1366",
    "hair": "#3A2B26",
    "hair_hi": "#5A4038",
    "robe": "#2F6168",
    "robe_top": "#3E838B",
    "robe_dark": "#214A50",
    "robe_shade": "#11343A99",
    "gold": "#D7A33C",
    "gold_dark": "#A87A26",
    "glass": "#BCE6E8AA",
    "chalk": "#F4F1E8",
    "boot": "#42352D",
    "boot_dark": "#2C231D",
    "outline": "#1A1410",
}


def cap(name, bone, z, r, fill, ow=1.1):
    return {"name": name, "bone": bone, "z": z, "kind": "capsule",
            "a": [0.0, 0.0], "b": None, "radius": r, "fill": fill,
            "outline": "outline", "outline_w": ow}


def circ(name, bone, z, center, r, fill, outline="outline", ow=0.5):
    p = {"name": name, "bone": bone, "z": z, "kind": "circle",
         "center": center, "radius": r, "fill": fill}
    if outline:
        p["outline"] = outline; p["outline_w"] = ow
    return p


def poly(name, bone, z, points, radius, fill, outline="outline", ow=1.1):
    p = {"name": name, "bone": bone, "z": z, "kind": "polygon",
         "points": points, "radius": radius, "fill": fill}
    if outline:
        p["outline"] = outline; p["outline_w"] = ow
    return p


def feature(part, name):
    """Tag a part as an optional accessory toggled by the doc's `features` map."""
    part["feature"] = name
    return part


parts = [
    # ---- Far (back) arm + leg: behind the body. ----
    cap("far_arm_upper", "far_arm_u", 10, 2.3, "robe_dark"),
    cap("far_arm_lower", "far_arm_l", 11, 2.0, "robe_dark"),
    circ("far_hand", "far_arm_l", 13, [8.0, 0.0], 2.6, "skin_shade", outline="outline", ow=0.5),
    cap("far_leg_upper", "far_leg_u", 20, 2.5, "robe_dark"),
    cap("far_leg_lower", "far_leg_l", 21, 2.3, "robe_dark"),
    far_foot,
    # ---- Robe skirt (pelvis): a wide flare so the coat reads floor-length. ----
    poly("skirt", "pelvis", 30,
         [[-11.0, -3.5], [11.0, -3.5], [16.0, 9.5], [-15.0, 9.5]], 3.0, "robe"),
    poly("skirt_shade", "pelvis", 31,
         [[-11.0, -3.0], [-4.0, -3.0], [-7.0, 9.0], [-15.0, 9.0]], 2.0, "robe_shade", outline=None),
    # ---- Coat (torso). ----
    poly("coat", "torso", 40,
         [[-10.0, -13.5], [10.5, -13.5], [11.5, -3.0], [12.5, 2.0], [-12.0, 2.0]], 3.4, "robe"),
    poly("coat_shade", "torso", 41,
         [[-10.0, -12.6], [-5.0, -12.8], [-6.0, 1.5], [-10.5, 1.5]], 2.2, "robe_shade", outline=None),
    poly("coat_hi", "torso", 42,
         [[5.0, -12.6], [10.0, -12.6], [11.0, -4.0], [9.5, 1.0], [5.5, 1.0]], 2.2, "robe_top", outline=None),
    # Gold lapel running down the coat front (+x side faces the viewer).
    poly("lapel", "torso", 43,
         [[3.4, -12.5], [7.0, -12.5], [5.2, 1.6], [3.2, 1.6]], 1.0, "gold", ow=0.5),
    # White rotational-symmetry sigil (a ring) — Noether's signature.
    feature(circ("sigil_ring", "torso", 44, [5.0, -6.5], 2.6, "robe_top", outline="chalk", ow=0.9), "sigil"),
    feature(circ("sigil_dot", "torso", 45, [5.0, -6.5], 0.7, "chalk", outline=None), "sigil"),
    # ---- Near (front) leg. ----
    cap("near_leg_upper", "near_leg_u", 50, 2.5, "robe_dark"),
    cap("near_leg_lower", "near_leg_l", 51, 2.3, "robe_dark"),
    near_foot,
    # ---- Head group. ----
    # Hair framing behind the face (back-left since she faces +x).
    poly("hair_back", "head", 58,
         [[-14.0, -15.0], [9.0, -15.0], [11.0, -2.0], [6.0, 13.0], [-14.0, 12.0]], 8.0, "hair"),
    # Skin of the face/head.
    poly("head", "head", 60,
         [[-12.0, -13.0], [12.5, -13.0], [12.5, 8.0], [8.0, 13.5], [-11.0, 12.0]], 7.5, "skin"),
    poly("face_shade", "head", 60.5,
         [[-11.0, -12.0], [-5.0, -12.0], [-5.0, 11.0], [-10.5, 10.5]], 4.0, "skin_shade", outline=None),
    # Brow/fringe sweeping across the forehead.
    poly("fringe", "head", 61,
         [[-12.0, -13.5], [12.5, -13.5], [12.5, -7.0], [4.0, -4.5], [-7.0, -5.5], [-12.0, -7.5]], 3.0, "hair"),
    # Eyes (behind the glass).
    circ("eye_far", "head", 62, [3.0, -1.0], 1.3, "outline", outline=None),
    circ("eye_near", "head", 62, [9.6, -1.0], 1.4, "outline", outline=None),
    # Round glasses: gold rims with translucent lenses, joined by a bridge.
    feature(circ("lens_far", "head", 63, [3.0, -1.0], 3.0, "glass", outline="gold", ow=0.9), "glasses"),
    feature(circ("lens_near", "head", 63, [9.6, -1.0], 3.1, "glass", outline="gold", ow=0.9), "glasses"),
    feature(poly("bridge", "head", 63,
                 [[5.8, -1.6], [7.0, -1.6], [7.0, -0.6], [5.8, -0.6]], 0.3, "gold", outline=None), "glasses"),
    # Hair bun on top, with a soft highlight.
    circ("bun", "head", 64, [-2.0, -15.5], 6.6, "hair", outline="outline", ow=1.0),
    circ("bun_hi", "head", 65, [-4.0, -17.0], 2.2, "hair_hi", outline=None),
    # ---- Hairpin: a rigid decorative pin set INTO the bun. Bound to `head`
    # (NOT the bobbing `antenna` channel) so it stays put instead of waving
    # like an antenna; a short gold shaft tucked diagonally through the bun
    # with a small pearl ornament at the exposed end. Optional `hairpin`
    # feature, on by default.
    feature({"name": "pin_shaft", "bone": "head", "z": 66, "kind": "capsule",
             "a": [-4.5, -13.5], "b": [2.5, -20.5], "radius": 0.65,
             "fill": "gold", "outline": "outline", "outline_w": 0.4}, "hairpin"),
    feature(circ("pin_bead", "head", 67, [3.2, -21.2], 1.5, "chalk", outline="gold", ow=0.5), "hairpin"),
    # ---- Near (front) arm: drawn last so it reads in front of the coat. ----
    cap("near_arm_upper", "near_arm_u", 70, 2.3, "robe_top"),
    cap("near_arm_lower", "near_arm_l", 71, 2.0, "robe_top"),
    circ("near_hand", "near_arm_l", 73, [8.0, 0.0], 2.8, "skin", outline="outline", ow=0.5),
]

# Emmy reads as a TALL scholar — a deliberate contrast to the short robot
# player. In-game height is `collision * collision_scale`, so a rigged NPC's
# stature is set by its sheet tuning, not its pixel art: bump collision_scale
# above the 1.5 fallback (the short robot's neighbourhood) so she stands taller.
# render_scale=2 doubles the rendered pixel resolution (geometry stays in
# base-frame units) so she isn't upscaled-and-pixelated in game — the display
# SIZE is unchanged (set by collision_scale), only the texture is crisper.
frame = dict(robot["frame"])
frame["render_scale"] = 2

doc = {
    "name": "noether",
    "frame": frame,
    "palette": PALETTE,
    # In-game sheet tuning emitted to the RON. collision_scale 2.0 (vs the 1.5
    # fallback) makes Emmy noticeably taller than the short robot player; nudge
    # it if she reads too tall/short against the player in game.
    "sprite_tuning": {"collision_scale": 2.0, "frame_sample_inset": 1},
    # Optional-accessory toggles. Flip any to re-customize Emmy without
    # touching the parts list. The hairpin now reads as a rigid pin set into
    # the bun (was an antenna), so it's on.
    "features": {"hairpin": True, "glasses": True, "sigil": True},
    "bones": robot["bones"],          # reuse skeleton verbatim (IK depends on it)
    "parts": parts,
    "ik_legs": robot["ik_legs"],
    # Idle + walk only (drop the robot's sword `slash` row — an NPC never swings).
    "clips": {"idle": robot["clips"]["idle"], "walk": robot["clips"]["walk"]},
}

OUT.write_text(json.dumps(doc, indent=1), encoding="utf8")
print(f"wrote {OUT} ({len(parts)} parts, clips={list(doc['clips'])})")
