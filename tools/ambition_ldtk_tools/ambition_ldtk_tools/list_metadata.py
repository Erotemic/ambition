#!/usr/bin/env python3
"""Print every LDtk level's biome metadata as a quick diagnostic.

Reads the LDtk project, groups levels by `activeArea`, and prints
`biome` / `music_track` / `ambient_profile` / `visual_theme` for
each. Helpful for agents that want to see "what does the runtime
read?" without booting the sandbox.

Usage:
    python3 tools/list_ldtk_metadata.py
    python3 tools/list_ldtk_metadata.py --ldtk path/to/sandbox.ldtk

Output groups levels under their activeArea so the merge semantics
(first non-empty value wins per area) are visible. A column showing
each level's individual metadata appears alongside the merged value
the runtime would resolve.
"""

from __future__ import annotations

import argparse
import json
import sys
from collections import defaultdict
from pathlib import Path

OPTIONAL_FIELDS = ("biome", "music_track", "ambient_profile", "visual_theme")


def field_value(fields, name):
    for f in fields:
        if f.get("__identifier") == name:
            v = f.get("__value")
            if isinstance(v, str):
                v = v.strip()
                if v:
                    return v
            return None
    return None


def merge_metadata(levels):
    """Mirror `RoomMetadata::merge` in Rust: first non-empty wins."""
    merged = {f: None for f in OPTIONAL_FIELDS}
    for level in levels:
        for field in OPTIONAL_FIELDS:
            if merged[field] is None:
                merged[field] = field_value(level.get("fieldInstances") or [], field)
    return merged


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument(
        "--ldtk",
        type=Path,
        default=Path("game/ambition_content/assets/worlds/sandbox.ldtk"),
    )
    args = parser.parse_args(argv)

    if not args.ldtk.exists():
        print(f"error: {args.ldtk} does not exist", file=sys.stderr)
        return 1
    proj = json.loads(args.ldtk.read_text())

    by_area = defaultdict(list)
    for level in proj.get("levels", []):
        area = (
            field_value(level.get("fieldInstances") or [], "activeArea")
            or level["identifier"]
        )
        by_area[area].append(level)

    print(f"# LDtk metadata report ({args.ldtk})")
    print(f"# {len(proj.get('levels', []))} levels in {len(by_area)} active areas")
    print()
    for area in sorted(by_area):
        levels = by_area[area]
        merged = merge_metadata(levels)
        bits = [f"{f}={v}" for f, v in merged.items() if v]
        merged_text = ", ".join(bits) if bits else "(empty)"
        print(f"area '{area}' [{len(levels)} level(s)] -> runtime: {merged_text}")
        for level in levels:
            per = {
                f: field_value(level.get("fieldInstances") or [], f)
                for f in OPTIONAL_FIELDS
            }
            level_bits = [f"{f}={v}" for f, v in per.items() if v]
            level_text = ", ".join(level_bits) if level_bits else "(none)"
            print(f"    level '{level['identifier']}': {level_text}")
        print()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
