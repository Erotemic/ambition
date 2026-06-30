#!/usr/bin/env python3
"""Generate optional visual-quality asset variants.

This is intentionally a post-publish helper: full-resolution sprite/background
generation remains unchanged, then this mirrors the installed folders into
`*_0_5x` / `*_0_25x` siblings. PNGs are resized deterministically; RON/YAML
sidecars are copied beside them so loaders can keep page filenames local to the
selected variant folder.
"""

from __future__ import annotations

import argparse
import shutil
from pathlib import Path

from PIL import Image


SCALES: tuple[tuple[str, float], ...] = (("0_5x", 0.5), ("0_25x", 0.25))
SIDECAR_SUFFIXES = {".ron", ".yaml", ".yml", ".json"}


def resize_png(src: Path, dst: Path, scale: float) -> None:
    with Image.open(src) as image:
        width = max(1, round(image.width * scale))
        height = max(1, round(image.height * scale))
        resized = image.resize((width, height), Image.Resampling.LANCZOS)
        dst.parent.mkdir(parents=True, exist_ok=True)
        resized.save(dst)


def mirror_tree(src_root: Path, dst_root: Path, scale: float) -> tuple[int, int]:
    png_count = 0
    sidecar_count = 0
    for src in sorted(src_root.rglob("*")):
        if src.is_dir():
            continue
        rel = src.relative_to(src_root)
        dst = dst_root / rel
        if src.suffix.lower() == ".png":
            resize_png(src, dst, scale)
            png_count += 1
        elif src.suffix.lower() in SIDECAR_SUFFIXES:
            # TODO(quality): RON/YAML atlas sidecars should be emitted by the
            # source renderers at the target render_scale. This post-resize
            # scaffold copies metadata so folder-local manifests exist, but it
            # does not rewrite packed frame rects.
            dst.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(src, dst)
            sidecar_count += 1
    return png_count, sidecar_count


def generate_sprite_variants(asset_root: Path) -> None:
    src = asset_root / "sprites"
    if not src.exists():
        print(f"skip missing sprite root: {src}")
        return
    for suffix, scale in SCALES:
        dst = asset_root / f"sprites_{suffix}"
        pngs, sidecars = mirror_tree(src, dst, scale)
        print(f"sprites {suffix}: {pngs} png, {sidecars} sidecars -> {dst}")


def generate_parallax_variants(asset_root: Path) -> None:
    src = asset_root / "backgrounds" / "parallax_layers"
    if not src.exists():
        print(f"skip missing parallax root: {src}")
        return
    for suffix, scale in SCALES:
        dst = asset_root / "backgrounds" / f"parallax_layers_{suffix}"
        pngs, sidecars = mirror_tree(src, dst, scale)
        print(f"parallax {suffix}: {pngs} png, {sidecars} sidecars -> {dst}")


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--asset-root",
        type=Path,
        default=Path("crates/ambition_gameplay_core/assets"),
        help="gameplay-core asset root containing sprites/ and backgrounds/",
    )
    parser.add_argument("--sprites-only", action="store_true")
    parser.add_argument("--backgrounds-only", action="store_true")
    args = parser.parse_args()

    asset_root = args.asset_root.resolve()
    if not args.backgrounds_only:
        generate_sprite_variants(asset_root)
    if not args.sprites_only:
        generate_parallax_variants(asset_root)


if __name__ == "__main__":
    main()
