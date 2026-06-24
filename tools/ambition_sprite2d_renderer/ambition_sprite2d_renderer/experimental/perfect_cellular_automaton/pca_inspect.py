#!/usr/bin/env python3
"""Build a target-vs-candidate inspection grid for every pose ROI.

Renders, for each ROI, [reference | candidate | overlay] stacked into one image
so a human (or a vision-capable agent) can eyeball alignment and spot missing
structure / noise.  Overlay: magenta = reference-only foreground,
cyan = candidate-only, white = both.
"""
from __future__ import annotations

import argparse
import json
from pathlib import Path

import numpy as np
from PIL import Image, ImageDraw, ImageFont

import pca_seg

HERE = Path(__file__).resolve().parent


def _font(size):
    try:
        return ImageFont.truetype("/usr/share/fonts/truetype/dejavu/DejaVuSansMono-Bold.ttf", size)
    except Exception:
        return ImageFont.load_default()


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--ref", type=Path, required=True)
    ap.add_argument("--cand", type=Path, required=True)
    ap.add_argument("--specs", type=Path, default=HERE / "pca_roi_specs_v14.json")
    ap.add_argument("--out", type=Path, required=True)
    ap.add_argument("--scale", type=float, default=1.6)
    args = ap.parse_args()

    ref = Image.open(args.ref).convert("RGB")
    cand = Image.open(args.cand).convert("RGB")
    if cand.size != ref.size:
        cand = cand.resize(ref.size, Image.NEAREST)
    specs = json.loads(args.specs.read_text())["rois"]
    ref_fg = pca_seg.foreground_mask(ref, tol=22.0)
    cbg = pca_seg.estimate_bg(cand)
    cand_fg_full = np.sqrt(((np.asarray(cand).astype(np.float32) - cbg) ** 2).sum(axis=2)) > 18.0

    panels = []
    for name, meta in specs.items():
        x0, y0, x1, y1 = meta["roi"]
        w, h = x1 - x0, y1 - y0
        t = ref.crop((x0, y0, x1, y1))
        c = cand.crop((x0, y0, x1, y1))
        tf = ref_fg[y0:y1, x0:x1]
        cf = cand_fg_full[y0:y1, x0:x1]
        ov = np.full((h, w, 3), 18, np.uint8)
        ov[tf & cf] = (235, 235, 235)
        ov[tf & ~cf] = (255, 0, 255)
        ov[~tf & cf] = (0, 220, 235)
        ovimg = Image.fromarray(ov)
        row = Image.new("RGB", (w * 3 + 24, h + 20), (12, 12, 14))
        d = ImageDraw.Draw(row)
        d.text((4, 2), name, font=_font(13), fill=(240, 240, 240))
        for i, im in enumerate([t, c, ovimg]):
            row.paste(im, (i * (w + 8), 18))
        panels.append(row.resize((int((w * 3 + 24) * args.scale), int((h + 20) * args.scale)), Image.NEAREST))

    width = max(p.width for p in panels)
    height = sum(p.height + 6 for p in panels)
    sheet = Image.new("RGB", (width, height), (6, 6, 8))
    y = 0
    for p in panels:
        sheet.paste(p, (0, y))
        y += p.height + 6
    sheet.save(args.out)
    print(f"wrote {args.out}  ({width}x{height})")


if __name__ == "__main__":
    main()
