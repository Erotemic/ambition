#!/usr/bin/env python3
"""Clean, low-edge, hand-authored PCA polygons (the template rebuild).

The optimizer/soup approach is refinement-only; the *structure* — especially the
face/head, which becomes a template for new bodies — is authored by hand as a
small number of low-edge named polygons, then eyeballed against the reference.

Each part is ``{name, color, pts}`` in absolute sheet coords; ``PARTS[pose]`` is
drawn back-to-front.  ``--overlay`` renders the parts at 60% over the reference
crop so placement can be corrected by eye; ``--solid`` renders parts alone.
"""
from __future__ import annotations

import argparse
import json
from pathlib import Path

import numpy as np
from PIL import Image, ImageDraw

HERE = Path(__file__).resolve().parent
REF = Path("/home/joncrall/code/ambition/assets/concept_art/"
           "prefect-cellular-automaton-reference-image.png")

PALETTE = {
    "black": (16, 18, 19),      # charcoal helmet/bodysuit (matches reference)
    "dark_green": (54, 88, 49),
    "green": (137, 188, 49),
    "lime": (164, 218, 42),
    "purple": (114, 78, 130),
    "cream": (240, 235, 180),
    "outline": (0, 0, 0),
}

# Regions of interest (absolute sheet coords).
HEAD_BOX = (110, 25, 235, 215)
FRONT_BOX = (55, 25, 290, 555)

# Front head: a handful of clean polygons.  Authored from grid_front_head.png.
PARTS = {
    "top_front_head": [
        # horns (green blades)
        {"name": "horn_l", "color": "green",
         "pts": [[139, 28], [150, 84], [120, 90]]},
        {"name": "horn_r", "color": "green",
         "pts": [[226, 28], [212, 84], [198, 88]]},
        # helmet dome (charcoal) — low-edge hexagon framing the face
        {"name": "helmet", "color": "black",
         "pts": [[140, 80], [205, 80], [214, 110], [205, 150],
                 [140, 150], [131, 110]]},
        # forehead automaton cells (4)
        {"name": "cell_top", "color": "green",
         "pts": [[153, 98], [173, 98], [173, 128], [153, 128]]},
        {"name": "cell_mid", "color": "lime",
         "pts": [[153, 113], [176, 113], [176, 140], [153, 140]]},
        {"name": "cell_l", "color": "dark_green",
         "pts": [[136, 112], [158, 112], [158, 138], [136, 138]]},
        {"name": "cell_r", "color": "dark_green",
         "pts": [[180, 112], [204, 112], [204, 138], [180, 138]]},
        # cream face mask (octagon)
        {"name": "face", "color": "cream",
         "pts": [[120, 134], [218, 138], [206, 168], [188, 182],
                 [172, 200], [152, 182], [130, 165], [121, 150]]},
        # eyes (dark slits)
        {"name": "eye_l", "color": "black",
         "pts": [[150, 148], [161, 150], [160, 168], [151, 166]]},
        {"name": "eye_r", "color": "black",
         "pts": [[183, 150], [193, 148], [192, 168], [184, 170]]},
    ],
}

# Front body (drawn back-to-front), authored from grid_front_full.png.
_FRONT_BODY = [
    # dark bodysuit core (behind the plates)
    {"name": "torso_core", "color": "black",
     "pts": [[140, 188], [200, 188], [202, 300], [170, 322], [138, 300]]},
    {"name": "pelvis", "color": "black",
     "pts": [[150, 305], [192, 305], [190, 358], [152, 358]]},
    # legs
    {"name": "thigh_l", "color": "purple",
     "pts": [[143, 352], [171, 352], [168, 432], [144, 432]]},
    {"name": "thigh_r", "color": "purple",
     "pts": [[171, 352], [199, 352], [198, 432], [174, 432]]},
    {"name": "knee_l", "color": "green",
     "pts": [[140, 424], [172, 424], [170, 466], [138, 466]]},
    {"name": "knee_r", "color": "green",
     "pts": [[172, 424], [206, 424], [206, 466], [174, 466]]},
    {"name": "shin_l", "color": "green",
     "pts": [[140, 462], [170, 462], [166, 522], [140, 522]]},
    {"name": "shin_r", "color": "green",
     "pts": [[174, 462], [206, 462], [206, 522], [178, 522]]},
    {"name": "foot_l", "color": "cream",
     "pts": [[120, 518], [168, 518], [168, 544], [122, 544]]},
    {"name": "foot_r", "color": "cream",
     "pts": [[176, 518], [222, 518], [220, 544], [176, 544]]},
    # arms -- shoulders are big rounded pauldrons that overlap the torso (z-order
    # bridges the gap to the body); drawn before the cream chest so the chest
    # edge sits on top.
    {"name": "shoulder_l", "color": "green",
     "pts": [[74, 184], [132, 180], [134, 214], [128, 238], [86, 240], [78, 214]]},
    {"name": "shoulder_r", "color": "green",
     "pts": [[212, 180], [270, 184], [266, 214], [258, 240], [216, 238], [210, 214]]},
    {"name": "uparm_l", "color": "green",
     "pts": [[90, 224], [126, 224], [122, 290], [92, 290]]},
    {"name": "uparm_r", "color": "green",
     "pts": [[218, 224], [254, 224], [252, 290], [222, 290]]},
    {"name": "forearm_l", "color": "purple",
     "pts": [[88, 288], [122, 288], [118, 344], [90, 344]]},
    {"name": "forearm_r", "color": "purple",
     "pts": [[222, 288], [256, 288], [254, 344], [226, 344]]},
    {"name": "hand_l", "color": "cream",
     "pts": [[86, 340], [118, 340], [112, 378], [88, 374]]},
    {"name": "hand_r", "color": "cream",
     "pts": [[226, 340], [258, 340], [256, 374], [232, 378]]},
    # chest plate (cream shield) over the torso core -- wider, with the green
    # shell shoulders flanking it
    {"name": "chest", "color": "cream",
     "pts": [[134, 184], [170, 178], [206, 184], [200, 230], [170, 250], [140, 230]]},
    # abdomen automaton grid (cream panel + cells)
    {"name": "belly_panel", "color": "cream",
     "pts": [[150, 240], [192, 240], [190, 300], [152, 300]]},
]

# 3x3 belly cells (green/lime checker) on the panel.
_BELLY_CELLS = []
for _r, _y in enumerate(range(246, 294, 16)):
    for _c, _x in enumerate(range(154, 190, 12)):
        _col = "lime" if (_r + _c) % 2 == 0 else "dark_green"
        _BELLY_CELLS.append({"name": f"bcell_{_r}_{_c}", "color": _col,
                             "pts": [[_x, _y], [_x + 10, _y], [_x + 10, _y + 13], [_x, _y + 13]]})

PARTS["top_front"] = _FRONT_BODY + _BELLY_CELLS + PARTS["top_front_head"]


def draw_parts(d, parts, outline=True):
    for p in parts:
        pts = [(float(x), float(y)) for x, y in p["pts"]]
        if len(pts) < 3:
            continue
        d.polygon(pts, fill=PALETTE[p["color"]] + (255,),
                  outline=(PALETTE["outline"] + (255,)) if outline else None)


def render_solid(key, box, scale=5):
    x0, y0, x1, y1 = box
    img = Image.new("RGBA", (x1 - x0, y1 - y0), (24, 26, 28, 255))
    d = ImageDraw.Draw(img, "RGBA")
    # translate parts into crop space
    shifted = [{**p, "pts": [[x - x0, y - y0] for x, y in p["pts"]]} for p in PARTS[key]]
    draw_parts(d, shifted)
    return img.resize(((x1 - x0) * scale, (y1 - y0) * scale), Image.NEAREST)


def render_overlay(key, box, scale=5, alpha=0.55):
    x0, y0, x1, y1 = box
    ref = Image.open(REF).convert("RGBA").crop(box)
    layer = Image.new("RGBA", ref.size, (0, 0, 0, 0))
    d = ImageDraw.Draw(layer, "RGBA")
    shifted = [{**p, "pts": [[x - x0, y - y0] for x, y in p["pts"]]} for p in PARTS[key]]
    for p in shifted:
        pts = [(float(x), float(y)) for x, y in p["pts"]]
        if len(pts) < 3:
            continue
        d.polygon(pts, fill=PALETTE[p["color"]] + (int(255 * alpha),),
                  outline=(255, 0, 255, 255))
    out = Image.alpha_composite(ref, layer)
    return out.resize((ref.width * scale, ref.height * scale), Image.NEAREST)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--key", default="top_front_head")
    ap.add_argument("--out-solid", type=Path, default=None)
    ap.add_argument("--out-overlay", type=Path, default=None)
    args = ap.parse_args()
    box = HEAD_BOX
    if args.out_solid:
        render_solid(args.key, box).save(args.out_solid)
        print("wrote", args.out_solid)
    if args.out_overlay:
        render_overlay(args.key, box).save(args.out_overlay)
        print("wrote", args.out_overlay)


if __name__ == "__main__":
    main()
