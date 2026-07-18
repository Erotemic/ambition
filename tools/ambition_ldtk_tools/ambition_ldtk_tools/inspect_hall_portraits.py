#!/usr/bin/env python3
"""Validate default dialog portraits for Hall-of-Characters catalog rows.

Portrait paths normally derive from the gameplay spritesheet path:
``sprites/<stem>_spritesheet.png`` becomes
``sprites/<stem>_portraits.{png,ron}``. An explicit ``portrait`` block may
replace that convention for exceptional products.
"""

from __future__ import annotations

import argparse
import re
import struct
import sys
from pathlib import Path
from typing import Any

from .ldtk import default_character_catalog, default_sprite_assets_dir

CATALOG_PATH = default_character_catalog()
SPRITES_DIR = default_sprite_assets_dir()
PNG_SIGNATURE = b"\x89PNG\r\n\x1a\n"


def png_dimensions(path: Path) -> tuple[int, int]:
    """Read and validate the fixed PNG signature and IHDR dimensions."""

    header = path.read_bytes()[:24]
    if len(header) < 24 or header[:8] != PNG_SIGNATURE:
        raise ValueError("not a PNG file")
    ihdr_length = struct.unpack(">I", header[8:12])[0]
    if ihdr_length != 13 or header[12:16] != b"IHDR":
        raise ValueError("missing canonical PNG IHDR header")
    width, height = struct.unpack(">II", header[16:24])
    if width <= 0 or height <= 0:
        raise ValueError(f"invalid PNG dimensions {width}x{height}")
    return width, height


def portrait_paths(entry: dict[str, Any], sprites_dir: Path) -> tuple[Path, Path] | None:
    explicit = entry.get("portrait")
    if isinstance(explicit, dict):
        image = explicit.get("image")
        manifest = explicit.get("manifest")
        if isinstance(image, str) and isinstance(manifest, str):
            return (
                sprites_dir / image.removeprefix("sprites/"),
                sprites_dir / manifest.removeprefix("sprites/"),
            )
    spritesheet = entry.get("spritesheet")
    if not isinstance(spritesheet, str) or not spritesheet.endswith("_spritesheet.png"):
        return None
    base = spritesheet[: -len("_spritesheet.png")]
    return (
        sprites_dir / f"{base.removeprefix('sprites/')}_portraits.png",
        sprites_dir / f"{base.removeprefix('sprites/')}_portraits.ron",
    )


def classify(entry: dict[str, Any], sprites_dir: Path) -> tuple[str, str]:
    paths = portrait_paths(entry, sprites_dir)
    if paths is None:
        return ("no_contract", "spritesheet path cannot derive a portrait product")
    image_path, manifest_path = paths
    if not image_path.exists():
        return ("no_png", f"missing {image_path}")
    try:
        image_width, image_height = png_dimensions(image_path)
    except (OSError, ValueError) as ex:
        return ("bad_png", f"{image_path.name}: {ex}")
    if not manifest_path.exists():
        return ("no_manifest", f"missing {manifest_path}")

    text = manifest_path.read_text(encoding="utf8")
    default_match = re.search(r'\bdefault_clip:\s*"([^"\n]+)"', text)
    if default_match is None:
        return ("bad_manifest", f"{manifest_path.name} has no default_clip")
    default_clip = default_match.group(1)
    marker = re.search(
        rf'"{re.escape(default_clip)}"\s*:\s*\((?P<body>.*?)\n\s*\),',
        text,
        flags=re.S,
    )
    if marker is None:
        return (
            "bad_manifest",
            f"{manifest_path.name} does not define clip {default_clip!r}",
        )
    if re.search(r'\bframes:\s*\[\s*\(', marker.group("body")) is None:
        return ("bad_manifest", f"{manifest_path.name} default clip has no frames")
    frame_match = re.search(
        r"\(x:\s*(\d+),\s*y:\s*(\d+),\s*w:\s*(\d+),\s*h:\s*(\d+)\)",
        marker.group("body"),
    )
    if frame_match is None:
        return ("bad_manifest", f"{manifest_path.name} default frame is malformed")
    x, y, width, height = (int(value) for value in frame_match.groups())
    if width <= 0 or height <= 0:
        return ("bad_manifest", f"{manifest_path.name} default frame is empty")
    if x + width > image_width or y + height > image_height:
        return (
            "bad_manifest",
            f"{manifest_path.name} default frame exceeds "
            f"{image_path.name} bounds {image_width}x{image_height}",
        )
    return ("ok", "")


def catalog_hall_rows(catalog: dict[str, Any]) -> list[tuple[str, dict[str, Any]]]:
    characters = catalog.get("characters") or {}
    return [
        (str(character_id), entry)
        for character_id, entry in characters.items()
        if isinstance(entry, dict) and entry.get("hall_dialogue_id") is not None
    ]


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--catalog", type=Path, default=CATALOG_PATH)
    parser.add_argument("--sprites-dir", type=Path, default=SPRITES_DIR)
    parser.add_argument("--only-issues", action="store_true")
    args = parser.parse_args(argv)

    from .ron_parse import load as ron_load

    catalog = ron_load(args.catalog.read_text(encoding="utf8"))
    rows = sorted(catalog_hall_rows(catalog), key=lambda item: item[0])
    counts: dict[str, int] = {}
    for character_id, entry in rows:
        status, detail = classify(entry, args.sprites_dir)
        counts[status] = counts.get(status, 0) + 1
        if args.only_issues and status == "ok":
            continue
        line = f"  [{status:<12}] {character_id:42s}"
        if detail:
            line += f"  {detail}"
        print(line)

    print(f"# {len(rows)} Hall catalog rows; " + ", ".join(
        f"{key}={value}" for key, value in sorted(counts.items())
    ))
    return 0 if counts.get("ok", 0) == len(rows) else 1


if __name__ == "__main__":
    sys.exit(main())
