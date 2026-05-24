#!/usr/bin/env python3
"""Auto-space overlapping DebugLabel entities in an LDtk level.

When two DebugLabels' rendered bboxes overlap, the debug overlay
renders them as a wall of garbled text. The `validate` pass already
flags overlapping pairs as warnings; this tool fixes them in place by
shifting the second label of each overlapping pair downward until the
overlap clears.

Per [[feedback-ldtk-tools-only]]: never hand-edit `sandbox.ldtk` or
`intro.ldtk` JSON directly — add a tool subcommand. This is that.

## Usage

```bash
PYTHONPATH=tools/ambition_ldtk_tools \\
python -m ambition_ldtk_tools.edit.space_debug_labels \\
    crates/ambition_sandbox/assets/ambition/worlds/intro.ldtk \\
    --in-place
```

The tool:
  1. Walks every level's DebugLabel entities.
  2. For each pair whose rects overlap (with a small padding), the
     SECOND of the pair (later in iteration order) is shifted down
     by `(other_height + PADDING)` px. Cached `__worldX/Y`/`__grid`
     fields are updated to stay consistent.
  3. Logs every shift so the caller can review.
  4. Always runs the `repair` post-pass to canonicalize the file.

Idempotent — running on an already-spaced file is a no-op.
"""
from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

PADDING_PX = 8


def rects_overlap(
    a: tuple[int, int, int, int],
    b: tuple[int, int, int, int],
) -> bool:
    ax, ay, aw, ah = a
    bx, by, bw, bh = b
    return ax < bx + bw and bx < ax + aw and ay < by + bh and by < ay + ah


def shift_overlapping_labels_in_level(level: dict) -> list[tuple[str, str, tuple[int, int], tuple[int, int]]]:
    """Mutate the level in place. Returns a list of (level_id,
    label_text, old_px, new_px) tuples documenting the shifts."""
    moved: list[tuple[str, str, tuple[int, int], tuple[int, int]]] = []
    level_id = level.get("identifier", "<unknown>")
    world_x = int(level.get("worldX", 0))
    world_y = int(level.get("worldY", 0))
    px_hei = int(level.get("pxHei", 4096))

    # Collect references so we can both inspect and mutate.
    label_refs: list[dict] = []
    for layer in level.get("layerInstances") or []:
        for inst in layer.get("entityInstances") or []:
            if inst.get("__identifier") == "DebugLabel":
                label_refs.append(inst)

    # Pair-wise check; shift the second of each colliding pair down.
    def current_rect(inst: dict) -> tuple[int, int, int, int]:
        px = inst.get("px", [0, 0])
        return (int(px[0]), int(px[1]), int(inst.get("width", 0)), int(inst.get("height", 0)))

    for i in range(len(label_refs)):
        for j in range(i + 1, len(label_refs)):
            ra = current_rect(label_refs[i])
            rb = current_rect(label_refs[j])
            if not rects_overlap(ra, rb):
                continue
            # Place j's top below ra's bottom + padding so the two
            # rects no longer overlap vertically. Computing from
            # ra's position (rather than incrementally from rb's
            # current y) is necessary when rb starts overlapping ra
            # by only a few pixels — incrementing by ra.height +
            # padding from rb's y over-shoots when rb already
            # starts mid-ra.
            old_px = (rb[0], rb[1])
            new_y = ra[1] + ra[3] + PADDING_PX
            if new_y + rb[3] > px_hei:
                # Would push out the bottom of the level — try
                # shifting up instead.
                new_y = max(0, ra[1] - rb[3] - PADDING_PX)
                if new_y == 0 and new_y + rb[3] > ra[1]:
                    # Still overlaps — skip rather than push out of
                    # bounds. The author can re-author manually.
                    continue
            inst = label_refs[j]
            inst["px"][1] = new_y
            inst["__worldX"] = world_x + int(inst["px"][0])
            inst["__worldY"] = world_y + int(inst["px"][1])
            grid_size = 16
            inst["__grid"] = [int(inst["px"][0]) // grid_size, int(inst["px"][1]) // grid_size]
            fields = {f["__identifier"]: f.get("__value")
                      for f in inst.get("fieldInstances", []) or []}
            text = (fields.get("text") or "")[:30]
            moved.append((level_id, text, old_px, (rb[0], new_y)))
    return moved


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("ldtk_file", type=Path)
    parser.add_argument("--in-place", action="store_true",
                        help="Write the result back to the input file (else stdout).")
    parser.add_argument("--no-repair", action="store_true",
                        help="Skip the post-pass `ambition_ldtk_tools repair`.")
    args = parser.parse_args(argv)

    data = json.loads(args.ldtk_file.read_text())
    all_moved: list = []
    for level in data.get("levels", []):
        all_moved.extend(shift_overlapping_labels_in_level(level))

    if all_moved:
        for level_id, text, old_px, new_px in all_moved:
            print(f"  shifted {level_id!r} label {text!r}: {old_px} -> {new_px}")
        print(f"# moved {len(all_moved)} label(s)")
    else:
        print("# no overlapping DebugLabels found — file unchanged")

    if args.in_place:
        args.ldtk_file.write_text(
            json.dumps(data, indent="\t", ensure_ascii=False) + "\n"
        )
        if not args.no_repair and all_moved:
            import subprocess
            cmd = [
                sys.executable, "-m", "ambition_ldtk_tools.repair",
                str(args.ldtk_file), "--in-place",
            ]
            subprocess.run(cmd)
    else:
        print(json.dumps(data, indent="\t", ensure_ascii=False))
    return 0


if __name__ == "__main__":
    sys.exit(main())
