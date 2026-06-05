#!/usr/bin/env python3
"""
Stage 1 — Chroma-key extraction.

Reads config.yaml for chroma-key parameters and scene-grid coordinates.

Outputs (under assets/vanity_card/):
  panels/panel_{col}_{row}.png   6 keyed & cropped scene panels
  parts/robot_parts_3d.png       keyed robot parts kit (3-D style)
  parts/robot_parts_alt.png      keyed robot parts kit (elongated style)
  parts/human_parts.png          keyed human parts kit
  ui/bubbles.png                 full speech-bubble sheet (bg removed)
  ui/speech_bubbles.png          top-strip of bubbles only

Run:  python3 extract_panels.py
"""

import os
import numpy as np
from PIL import Image

from utils import (
    load_config,
    src_path,
    out_path,
    chroma_key,
    cleanup_green_residue,
    remove_flat_bg,
    tight_crop,
    save,
)

# ── Source files ──────────────────────────────────────────────────────────────


def _scene_sheet(cfg):
    return src_path(cfg, "cutouts", "ChatGPT Image May 13, 2026, 09_35_29 PM.png")


def _robot_gs_3d(cfg):
    return src_path(
        cfg, "cutouts", "greenscreen", "ChatGPT Image May 13, 2026, 09_15_45 PM.png"
    )


def _robot_gs_alt(cfg):
    return src_path(
        cfg, "cutouts", "greenscreen", "ChatGPT Image May 13, 2026, 09_20_55 PM.png"
    )


def _human_gs(cfg):
    return src_path(
        cfg, "cutouts", "greenscreen", "ChatGPT Image May 13, 2026, 09_14_02 PM.png"
    )


def _bubble_sheet(cfg):
    return src_path(cfg, "ChatGPT Image May 13, 2026, 12_20_36 AM (3).png")


# Scene-sheet grid — measured from pixel analysis.
# To adjust: change config.yaml → chroma_key params, then re-run this script.
SCENE_COL_SPANS = [(22, 478), (512, 947), (975, 1427)]
SCENE_ROW_SPANS = [(136, 482), (582, 928)]


# ── Stage 1a: Scene panels ────────────────────────────────────────────────────


def extract_scene_panels(cfg):
    print("\n--- Scene panels ---")
    ck = cfg["chroma_key"]
    src = Image.open(_scene_sheet(cfg))

    for ci, (cx0, cx1) in enumerate(SCENE_COL_SPANS):
        for ri, (ry0, ry1) in enumerate(SCENE_ROW_SPANS):
            cell = src.crop((cx0, ry0, cx1 + 1, ry1 + 1))
            keyed = chroma_key(
                cell, ck["inner_radius"], ck["outer_radius"], ck["spill_reduction"]
            )
            keyed = cleanup_green_residue(keyed)
            cropped = tight_crop(keyed, margin=8)
            save(cropped, out_path(cfg, "panels", f"panel_{ci}_{ri}.png"))


# ── Stage 1b: Parts kits ──────────────────────────────────────────────────────


def extract_parts(cfg):
    print("\n--- Parts kits ---")
    ck = cfg["chroma_key"]
    kits = [
        (_robot_gs_3d(cfg), out_path(cfg, "parts", "robot_parts_3d.png")),
        (_robot_gs_alt(cfg), out_path(cfg, "parts", "robot_parts_alt.png")),
        (_human_gs(cfg), out_path(cfg, "parts", "human_parts.png")),
    ]
    for src_file, dst in kits:
        img = Image.open(src_file)
        keyed = chroma_key(
            img, ck["inner_radius"], ck["outer_radius"], ck["spill_reduction"]
        )
        save(keyed, dst)


# ── Stage 1c: Speech-bubble sheet ─────────────────────────────────────────────


def extract_ui(cfg):
    print("\n--- Speech bubbles ---")
    src = Image.open(_bubble_sheet(cfg))
    keyed = remove_flat_bg(src, tolerance=25)
    save(keyed, out_path(cfg, "ui", "bubbles.png"))

    h = keyed.height
    strip = keyed.crop((0, 0, keyed.width, int(h * 0.35)))
    save(tight_crop(strip, margin=4), out_path(cfg, "ui", "speech_bubbles.png"))


# ── Main ──────────────────────────────────────────────────────────────────────


def main():
    cfg = load_config()
    extract_scene_panels(cfg)
    extract_parts(cfg)
    extract_ui(cfg)
    print("\nDone.  Next: python3 compose.py")


if __name__ == "__main__":
    main()
