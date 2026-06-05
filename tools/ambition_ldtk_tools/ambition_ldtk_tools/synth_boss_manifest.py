#!/usr/bin/env python3
"""Synthesize a `<boss>_spritesheet.ron` SheetRecord file from the
boss's existing `<boss>_spritesheet_manifest.json`.

The current boss spritesheet generators (`mockingbird_boss`,
`gnu_ton_boss`) emit a JSON manifest but no RON SheetRecord; the
runtime instead drives them through the `EntitySprite::BossCore`
path. After Phase 6 of the character-catalog refactor, the
sandbox's catalog wants these bosses too — but the catalog's
manifest-driven loader expects a SheetRecord-shaped `.ron` file.

This one-shot generator reads the JSON, writes a minimal RON
SheetRecord (frame_width/frame_height/rows + grid-derived rects),
and lets the catalog pull the boss into the Hall of Characters
without the BossCore special-case.

Output is intentionally minimal — no `body_metrics`, no per-row
duration_secs (derived from `duration_ms`). The catalog falls back
to its default tuning (`collision_scale = 1.5`, `frame_sample_inset
= 1`), which sizes the rendered sprite to whatever AABB the
NpcSpawn declares — fine for a gallery pedestal.

## Usage

```bash
PYTHONPATH=tools/ambition_ldtk_tools \\
python -m ambition_ldtk_tools.synth_boss_manifest \\
    crates/ambition_sandbox/assets/sprites/gnu_ton_boss/gnu_ton_boss_spritesheet_manifest.json
```

Writes `<target>_spritesheet.ron` next to the JSON.
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any


def normalize_frame_size(v: Any) -> tuple[int, int]:
    if isinstance(v, dict):
        return int(v["w"]), int(v["h"])
    if isinstance(v, (list, tuple)) and len(v) >= 2:
        return int(v[0]), int(v[1])
    raise ValueError(f"unexpected frame_size: {v!r}")


def normalize_row(row: dict, frame_w: int, frame_h: int, is_first_row: bool) -> dict:
    """Return a SheetRow-shaped dict with rects filled in.

    When the source animation name isn't one the runtime's
    `CharacterAnim::from_name` maps to `Idle` (including its aliases:
    `idle`, `opening`, `rest`, `front_idle`, `side_idle`) AND this is
    the first row of the sheet, rename the row to `rest` so the
    Hall-of-Characters pedestal has a static pose to play. Real
    runtime consumers (boss encounter driver, etc.) still drive the
    sheet via their own row indices, not by name.
    """
    animation = row.get("animation") or row.get("name")
    idle_aliases = {"idle", "opening", "rest", "front_idle", "side_idle"}
    if is_first_row and animation not in idle_aliases:
        animation = "rest"
    row_index = int(row.get("row_index", row.get("row", 0)))
    frame_count = int(row.get("frames", row.get("frame_count", 1)))
    duration_ms = int(row.get("duration_ms", 100))
    rects_in = row.get("rects")
    if rects_in:
        rects = [
            {
                "x": int(r["x"]),
                "y": int(r["y"]),
                "w": int(r["w"]),
                "h": int(r["h"]),
                "anchors": {},
            }
            for r in rects_in
        ]
    else:
        rects = [
            {
                "x": i * frame_w,
                "y": row_index * frame_h,
                "w": frame_w,
                "h": frame_h,
                "anchors": {},
            }
            for i in range(frame_count)
        ]
    return {
        "animation": animation,
        "row_index": row_index,
        "frame_count": frame_count,
        "duration_ms": duration_ms,
        "duration_secs": duration_ms / 1000.0,
        "rects": rects,
    }


def synthesize(json_path: Path) -> Path:
    data = json.loads(json_path.read_text())
    target = data["target"]
    frame_w, frame_h = normalize_frame_size(data["frame_size"])
    rows_in = data.get("rows", [])
    rows_out = [
        normalize_row(r, frame_w, frame_h, is_first_row=(i == 0))
        for i, r in enumerate(rows_in)
    ]
    record = {
        "target": target,
        "image": f"{target}_spritesheet.png",
        "label_width": 0,
        "frame_width": frame_w,
        "frame_height": frame_h,
        # body_metrics omitted — the catalog's default tuning
        # handles it. Bosses that need a real anchor can graduate
        # to a hardcoded `*_SHEET` const + bespoke
        # `feet_anchor_y_override` later.
        "rows": rows_out,
    }

    from .ron_parse import dumps as ron_dumps

    out_path = json_path.parent / f"{target}_spritesheet.ron"
    # SheetRecord files are a Vec<SheetRecord> in Rust, so wrap our
    # one record in a list.
    out_text = ron_dumps([record])
    out_path.write_text(out_text)
    return out_path


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("manifest_json", nargs="+", type=Path)
    args = parser.parse_args(argv)
    for path in args.manifest_json:
        if not path.exists():
            print(f"[error] {path} does not exist", file=sys.stderr)
            return 1
        out = synthesize(path)
        print(f"  wrote {out}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
