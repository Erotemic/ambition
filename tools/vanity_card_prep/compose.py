#!/usr/bin/env python3
"""
Stage 2 — Pose detection & final panel composition.

1. Detects individual poses from each pose sheet and writes them to:
     assets/vanity_card/poses/{name}/pose_{N:02d}.png
     assets/vanity_card/poses/{name}/index_map.png   ← open this to find pose indices!

2. Composes the 4 final animation panels per config.yaml and writes them to:
     assets/vanity_card/final/panel_{beat}.png

Interactive controls (no re-running needed):
  • After running, open poses/{name}/index_map.png to see numbered poses, then edit
    pose_index in config.yaml and re-run to update the final panels.
  • Drop a custom PNG at assets/vanity_card/final/panel_{beat}_override.png to bypass
    the auto-compose for that beat entirely.

Run:  python3 compose.py
"""

import os
import sys
import numpy as np
from PIL import Image, ImageDraw, ImageFont

from utils import (
    load_config,
    src_path,
    out_path,
    chroma_key,
    remove_flat_bg,
    find_content_spans,
    tight_crop,
    has_transparency,
    save,
)

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.dirname(os.path.dirname(SCRIPT_DIR))


# ── Pose sheet loading ────────────────────────────────────────────────────────


def load_pose_sheet(path: str) -> Image.Image:
    """Load a pose sheet, removing background if needed."""
    img = Image.open(path).convert("RGBA")
    if has_transparency(img):
        return img
    # White/flat background — flood-fill remove it
    return remove_flat_bg(img, tolerance=30)


def detect_sprites(
    img: Image.Image,
    min_gap: int = 18,
    min_size: int = 55,
    content_col_frac: float = 0.05,
    content_row_frac: float = 0.03,
    margin: int = 10,
) -> list:
    """
    Find all distinct sprites in a sheet image.

    Returns a list of RGBA Images, sorted top-to-bottom then left-to-right.

    min_gap:          px of transparent gap needed between sprites before splitting
    min_size:         minimum sprite dimension (px) — filters tiny labels/noise
    content_col_frac: fraction of non-transparent pixels a column must have to
                      count as "content" (higher values ignore faint label text)
    content_row_frac: same threshold applied to rows
    """
    arr = np.array(img)
    alpha = arr[:, :, 3]

    # Use fraction of non-transparent pixels rather than binary max-presence.
    # This lets a high content_col_frac filter out number-label columns that
    # sit between sprite columns (they have low density, not zero).
    row_has = (alpha > 15).mean(axis=1) > content_row_frac
    col_has = (alpha > 15).mean(axis=0) > content_col_frac

    row_spans = find_content_spans(row_has, min_gap=min_gap, min_size=min_size)
    col_spans = find_content_spans(col_has, min_gap=min_gap, min_size=min_size)

    sprites = []
    for ry0, ry1 in row_spans:
        for cx0, cx1 in col_spans:
            cell_alpha = alpha[ry0:ry1, cx0:cx1]
            # Skip cells that are almost empty (labels, stray pixels, decoration)
            if cell_alpha.sum() < 3000 or (cell_alpha > 15).mean() < 0.015:
                continue
            cell = img.crop((cx0, ry0, cx1, ry1))
            bbox = cell.getbbox()
            if bbox:
                l, t, r, b = bbox
                l, t = max(0, l - margin), max(0, t - margin)
                r, b = min(cell.width, r + margin), min(cell.height, b + margin)
                cell = cell.crop((l, t, r, b))
            sprites.append(((ry0, cx0), cell))  # keep position for sorting

    sprites.sort(key=lambda x: (x[0][0] // 80, x[0][1]))  # row-major order
    return [s for _, s in sprites]


def save_index_map(sprites: list, output_path: str, cols: int = 5) -> None:
    """Save a grid image showing every sprite with its index number."""
    if not sprites:
        return

    thumb_w, thumb_h = 200, 250
    label_h = 24
    pad = 6
    rows = (len(sprites) + cols - 1) // cols

    grid_w = cols * (thumb_w + pad) + pad
    grid_h = rows * (thumb_h + label_h + pad) + pad
    grid = Image.new("RGBA", (grid_w, grid_h), (230, 230, 230, 255))
    draw = ImageDraw.Draw(grid)

    try:
        font = ImageFont.truetype(
            "/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf", 18
        )
    except Exception:
        font = ImageFont.load_default()

    for i, sprite in enumerate(sprites):
        col_idx = i % cols
        row_idx = i // cols
        x = col_idx * (thumb_w + pad) + pad
        y = row_idx * (thumb_h + label_h + pad) + pad

        # White cell background
        draw.rectangle(
            [x, y, x + thumb_w - 1, y + thumb_h - 1],
            fill=(255, 255, 255, 255),
            outline=(160, 160, 160),
            width=1,
        )

        # Thumbnail: fit in cell, align bottom
        thumb = sprite.copy()
        thumb.thumbnail((thumb_w - 4, thumb_h - 4), Image.LANCZOS)
        tx = x + (thumb_w - thumb.width) // 2
        ty = y + thumb_h - thumb.height - 2
        grid.paste(thumb, (tx, ty), thumb)

        # Index label
        label = f"#{i}"
        draw.text(
            (x + thumb_w // 2, y + thumb_h + 2),
            label,
            fill=(20, 20, 20),
            anchor="mt",
            font=font,
        )

    grid = grid.convert("RGB")
    os.makedirs(os.path.dirname(output_path), exist_ok=True)
    grid.save(output_path)
    print(f"  index map → {os.path.relpath(output_path, REPO)}")


# ── Panel canvas composition ──────────────────────────────────────────────────


def fit_to_canvas(
    src: Image.Image, canvas_w: int, canvas_h: int, ground_y: float, margin: int = 24
) -> Image.Image:
    """
    Fit *src* (transparent PNG) onto a transparent canvas of (canvas_w, canvas_h).
    The bottom of the content lands at ground_y * canvas_h.
    """
    canvas = Image.new("RGBA", (canvas_w, canvas_h), (0, 0, 0, 0))

    content = src
    bbox = src.getbbox()
    if bbox:
        content = src.crop(bbox)

    cw, ch = content.size
    avail_w = canvas_w - 2 * margin
    avail_h = int(canvas_h * ground_y) - margin

    scale = min(avail_w / max(cw, 1), avail_h / max(ch, 1), 1.0)
    new_w = max(1, int(cw * scale))
    new_h = max(1, int(ch * scale))
    scaled = content.resize((new_w, new_h), Image.LANCZOS)

    paste_x = (canvas_w - new_w) // 2
    paste_y = int(canvas_h * ground_y) - new_h
    canvas.paste(scaled, (paste_x, paste_y), scaled)
    return canvas


def compose_panel(
    panel_cfg: dict, cfg: dict, pose_sets: dict, panels_dir: str
) -> Image.Image:
    """Build one final panel image from its config entry."""
    source = panel_cfg["source"]
    pw, ph = cfg["panel_size"]
    gy = cfg["ground_y"]

    if source == "scene":
        col = panel_cfg["scene_col"]
        row = panel_cfg["scene_row"]
        src = Image.open(os.path.join(panels_dir, f"panel_{col}_{row}.png"))
    elif source == "robot_pose":
        idx = panel_cfg["pose_index"]
        poses = pose_sets.get("robot", [])
        if idx >= len(poses):
            print(
                f"  WARNING: robot pose_index {idx} out of range "
                f"({len(poses)} detected). Using last available."
            )
            idx = max(0, len(poses) - 1)
        src = poses[idx]
    elif source == "human_pose":
        idx = panel_cfg["pose_index"]
        poses = pose_sets.get("human", [])
        if idx >= len(poses):
            print(
                f"  WARNING: human pose_index {idx} out of range "
                f"({len(poses)} detected). Using last available."
            )
            idx = max(0, len(poses) - 1)
        src = poses[idx]
    else:
        raise ValueError(f"Unknown source type: {source!r}")

    return fit_to_canvas(src, pw, ph, gy)


# ── Main ──────────────────────────────────────────────────────────────────────


def main():
    cfg = load_config()

    # ── Step 1: Extract poses from each pose sheet ─────────────────────────
    print("\n--- Detecting poses ---")
    pose_sets = {}
    for name, ps_cfg in cfg["pose_sources"].items():
        path = src_path(cfg, ps_cfg["file"])
        if not os.path.exists(path):
            print(f"  SKIP {name}: file not found: {path}")
            pose_sets[name] = []
            continue

        img = load_pose_sheet(path)
        det = ps_cfg.get("detect", {})
        sprites = detect_sprites(
            img,
            min_gap=det.get("min_gap", 18),
            min_size=det.get("min_size", 55),
            content_col_frac=det.get("content_col_frac", 0.05),
            content_row_frac=det.get("content_row_frac", 0.03),
        )
        print(f"  {name}: {len(sprites)} poses detected")

        pose_dir = out_path(cfg, "poses", name)
        os.makedirs(pose_dir, exist_ok=True)
        for i, sp in enumerate(sprites):
            sp.save(os.path.join(pose_dir, f"pose_{i:02d}.png"))

        save_index_map(sprites, os.path.join(pose_dir, "index_map.png"))
        pose_sets[name] = sprites

    # ── Step 2: Compose final panels ───────────────────────────────────────
    print("\n--- Composing final panels ---")
    panels_dir = out_path(cfg, "panels")
    final_dir = out_path(cfg, "final")
    os.makedirs(final_dir, exist_ok=True)

    for panel_cfg in cfg["panels"]:
        beat = panel_cfg["beat"]
        label = panel_cfg["label"]

        override_path = os.path.join(final_dir, f"panel_{beat}_override.png")
        if os.path.exists(override_path):
            print(f"  beat {beat} ({label}): using override file")
            continue  # override already in place, don't overwrite it

        img = compose_panel(panel_cfg, cfg, pose_sets, panels_dir)
        dest = os.path.join(final_dir, f"panel_{beat}.png")
        save(img, dest)

    print(
        "\nDone.\n"
        "  • Open assets/vanity_card/poses/*/index_map.png to find pose indices.\n"
        "  • Edit config.yaml → panels[*].pose_index, then re-run compose.py.\n"
        "  • Drop panel_{beat}_override.png in assets/vanity_card/final/ to use a custom image.\n"
        "  • Run python3 demo.py to preview the animation."
    )


if __name__ == "__main__":
    main()
