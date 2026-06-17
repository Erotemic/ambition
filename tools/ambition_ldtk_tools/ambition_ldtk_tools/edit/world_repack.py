#!/usr/bin/env python3
"""Re-pack LDtk levels into a left-to-right adjacent chain.

`world init` defaults to a Free layout with arbitrary world coords.
Once a project graduates to GridVania, levels need to snap to the
world grid and (ideally) sit edge-adjacent so the editor view shows
a continuous map instead of disconnected rectangles.

This tool re-packs a project's levels horizontally:

1. Walk levels in their current order.
2. Place the first level at `(start_x, start_y)`.
3. Each subsequent level is placed with its left edge at the
   previous level's right edge — no gaps, no overlap.
4. Update every entity's `__worldX` / `__worldY` inside the
   re-positioned levels to match (LDtk pre-computes these from
   `level.worldX + entity.px[i]`).

The tool refuses to act if any level dimension isn't a multiple
of the project's `worldGridWidth` (would land off-grid). Pass
`--allow-off-grid` to ignore.

## Usage

```bash
PYTHONPATH=tools/ambition_ldtk_tools \\
python -m ambition_ldtk_tools world repack \\
    crates/ambition_gameplay_core/assets/ambition/worlds/intro.ldtk \\
    --start-x 0 --start-y 0 \\
    --in-place
```

`--order LEVEL1,LEVEL2,...` overrides the placement order; otherwise
levels keep their existing array order.

The standard `repair --in-place` + `validate --require-schema`
post-pass runs unless `--no-repair` is passed.
"""

from __future__ import annotations

import argparse
import json
import shutil
import subprocess
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[4]


def find_level_by_identifier(levels: list[dict], identifier: str) -> dict:
    for level in levels:
        if level.get("identifier") == identifier:
            return level
    raise SystemExit(f"level '{identifier}' not in project")


def update_entity_world_coords(level: dict) -> int:
    """For every entity in every layer of `level`, re-derive its
    `__worldX` / `__worldY` from `level.worldX + entity.px[0]` etc.
    Returns the count of entities updated.
    """
    world_x = int(level["worldX"])
    world_y = int(level["worldY"])
    count = 0
    for layer in level.get("layerInstances", []):
        for ent in layer.get("entityInstances", []):
            px = ent.get("px") or [0, 0]
            ent["__worldX"] = world_x + int(px[0])
            ent["__worldY"] = world_y + int(px[1])
            count += 1
    return count


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("action", choices=["repack"], help="Subcommand action.")
    parser.add_argument("ldtk", type=Path, help="Target .ldtk file to modify.")
    parser.add_argument(
        "--start-x",
        type=int,
        default=0,
        help="x coordinate of the first level (default 0).",
    )
    parser.add_argument(
        "--start-y",
        type=int,
        default=0,
        help="y coordinate of every level (default 0). Re-packs along x.",
    )
    parser.add_argument(
        "--order",
        type=str,
        default=None,
        metavar="L1,L2,...",
        help="Comma-separated level identifiers giving placement order.",
    )
    parser.add_argument(
        "--allow-off-grid",
        action="store_true",
        help="Don't enforce that level dimensions match worldGridWidth.",
    )
    parser.add_argument(
        "--in-place",
        action="store_true",
        help="Write back to the input .ldtk path.",
    )
    parser.add_argument(
        "--output",
        type=Path,
        default=None,
        help="Output path (alternative to --in-place).",
    )
    parser.add_argument(
        "--backup",
        action="store_true",
        help="When using --in-place, copy the original to <ldtk>.bak first.",
    )
    parser.add_argument(
        "--no-repair",
        action="store_true",
        help="Skip the repair + validate post-pass.",
    )
    parser.add_argument(
        "--schema",
        type=Path,
        default=REPO_ROOT
        / "tools"
        / "ambition_ldtk_tools"
        / "schemas"
        / "ldtk"
        / "JSON_SCHEMA.json",
    )
    args = parser.parse_args(argv)

    if args.action != "repack":
        return _fail(f"unknown world action '{args.action}'")
    if not args.in_place and args.output is None:
        return _fail("choose --in-place or --output <path>")
    if not args.ldtk.exists():
        return _fail(f"ldtk file not found: {args.ldtk}")

    project = json.loads(args.ldtk.read_text())
    levels = list(project.get("levels", []))
    if not levels:
        return _fail("project has no levels to repack")

    grid_w = int(project.get("worldGridWidth", 16))
    grid_h = int(project.get("worldGridHeight", 16))

    if args.order:
        order_ids = [s.strip() for s in args.order.split(",") if s.strip()]
        levels = [find_level_by_identifier(levels, ident) for ident in order_ids]
    else:
        order_ids = [lvl["identifier"] for lvl in levels]

    if not args.allow_off_grid:
        for level in levels:
            for dim, divisor, name in (
                (int(level["pxWid"]), grid_w, "pxWid / worldGridWidth"),
                (int(level["pxHei"]), grid_h, "pxHei / worldGridHeight"),
            ):
                if dim % divisor != 0:
                    return _fail(
                        f"level '{level['identifier']}' {name} mismatch: "
                        f"{dim} is not a multiple of {divisor}. Pass "
                        f"--allow-off-grid to bypass."
                    )

    cursor_x = int(args.start_x)
    start_y = int(args.start_y)
    moved = 0
    total_entities = 0
    for level in levels:
        prev_x = int(level.get("worldX", 0))
        prev_y = int(level.get("worldY", 0))
        new_x = cursor_x
        new_y = start_y
        if prev_x != new_x or prev_y != new_y:
            moved += 1
        level["worldX"] = new_x
        level["worldY"] = new_y
        total_entities += update_entity_world_coords(level)
        print(
            f"placed {level['identifier']:24s} at ({new_x:>6}, {new_y:>4}) "
            f"size {level['pxWid']:>4}x{level['pxHei']:<4} "
            f"(was ({prev_x}, {prev_y}))"
        )
        cursor_x += int(level["pxWid"])

    print(
        f"re-packed {len(levels)} level(s); moved {moved}; "
        f"updated __worldX/__worldY on {total_entities} entit(y/ies); "
        f"final span = {cursor_x - int(args.start_x)}px wide."
    )

    target = args.output or args.ldtk
    if args.in_place and args.backup:
        backup = args.ldtk.with_suffix(args.ldtk.suffix + ".bak")
        shutil.copy2(args.ldtk, backup)
        print(f"wrote backup: {backup}")

    from ambition_ldtk_tools.editor_format import dump_editor_style

    target.write_text(dump_editor_style(project))
    print(f"wrote {target}")

    if args.no_repair:
        return 0

    cmd = [
        sys.executable,
        "-m",
        "ambition_ldtk_tools.repair",
        str(target),
        "--in-place",
    ]
    print("$ " + " ".join(cmd))
    rc = subprocess.run(cmd).returncode
    if rc != 0:
        return rc
    cmd = [sys.executable, "-m", "ambition_ldtk_tools.validate", str(target)]
    if args.schema and args.schema.exists():
        cmd.extend(["--schema", str(args.schema), "--require-schema"])
    print("$ " + " ".join(cmd))
    return subprocess.run(cmd).returncode


def _fail(msg: str) -> int:
    print(f"error: {msg}", file=sys.stderr)
    return 2


if __name__ == "__main__":
    raise SystemExit(main())
