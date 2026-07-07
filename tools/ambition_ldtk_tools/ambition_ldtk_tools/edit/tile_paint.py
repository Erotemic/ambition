#!/usr/bin/env python3
"""Paint tiles into a Tiles layer instance from an IntGrid source.

Companion to `tileset add-layer`: that tool wires up an empty
Tiles layer; this one populates `gridTiles[]` from the active
Collision IntGrid layer (or any IntGrid layer in the same level)
using a per-value tile-id mapping.

The mapping is per-IntGrid-value:

    --map 1=0     # IntGrid value 1 (Solid) -> tile id 0 (wall_plain)
    --map 2=28    # IntGrid value 2 (OneWayUp) -> tile id 28 (platform_single)

For every cell where the source IntGrid is the named value, the
tool emits a `Tile` entry at the same grid coordinate referencing
the mapped tile id. Cells where IntGrid is 0 (or not in the map)
are skipped — `gridTiles` is sparse.

This is the lowest-fidelity "the tileset renders" path: every
solid cell gets the same wall tile, every one-way cell gets the
same platform tile. Visual variety + auto-tile rules are a follow-
on. The point of this step is to ship visible, source-driven
tiles in-game.

## Usage

```bash
PYTHONPATH=tools/ambition_ldtk_tools \\
python -m ambition_ldtk_tools tileset paint \\
    game/ambition_content/assets/worlds/intro.ldtk \\
    intro_wake_room \\
    IntroLabTiles \\
    --from-intgrid Collision \\
    --map 1=0 \\
    --map 2=28 \\
    --in-place
```

Repeat the call per level. `--all-levels` paints every level in
one pass using the same map.

The tool refuses to paint over an existing `gridTiles[]` unless
`--overwrite` is passed (the default protects manual editor work
that might not be in the IntGrid).

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


def find_tileset_def_by_uid(project: dict, uid: int) -> dict:
    for ts in project.get("defs", {}).get("tilesets", []):
        if int(ts.get("uid", -1)) == int(uid):
            return ts
    raise SystemExit(f"tileset uid {uid} not found in project")


def find_layer_def(project: dict, identifier: str) -> dict:
    for layer in project.get("defs", {}).get("layers", []):
        if layer.get("identifier") == identifier:
            return layer
    raise SystemExit(
        f"layer def '{identifier}' not found in project; use `tileset add-layer` first"
    )


def find_levels(project: dict, level_identifier: str | None) -> list[dict]:
    if level_identifier is None:
        return list(project.get("levels", []))
    matches = [
        lvl
        for lvl in project.get("levels", [])
        if lvl.get("identifier") == level_identifier
    ]
    if not matches:
        raise SystemExit(f"level '{level_identifier}' not found in project")
    return matches


def find_layer_instance(level: dict, identifier: str) -> dict:
    for layer in level.get("layerInstances", []):
        if layer.get("__identifier") == identifier:
            return layer
    raise SystemExit(
        f"layer instance '{identifier}' missing on level '{level.get('identifier')}'; "
        f"use `tileset add-layer` first."
    )


def parse_map(args: list[str]) -> dict[int, int]:
    """`--map 1=0 --map 2=28` -> `{1: 0, 2: 28}`."""
    out: dict[int, int] = {}
    for raw in args:
        if "=" not in raw:
            raise SystemExit(f"--map expects 'VALUE=TILE_ID', got {raw!r}")
        value_s, tile_s = raw.split("=", 1)
        try:
            value, tile = int(value_s), int(tile_s)
        except ValueError as ex:
            raise SystemExit(f"--map values must be integers: {raw!r} ({ex})")
        if value <= 0:
            raise SystemExit(f"--map value must be > 0 (0 = empty); got {value}")
        if tile < 0:
            raise SystemExit(f"--map tile id must be >= 0; got {tile}")
        if value in out:
            raise SystemExit(f"--map value {value} specified twice")
        out[value] = tile
    if not out:
        raise SystemExit("at least one --map VALUE=TILE_ID is required")
    return out


def tile_src_px(tile_id: int, tile_grid_size: int, atlas_cols: int) -> tuple[int, int]:
    col = tile_id % atlas_cols
    row = tile_id // atlas_cols
    return col * tile_grid_size, row * tile_grid_size


def paint_layer(
    layer_instance: dict,
    source_csv: list[int],
    source_c_wid: int,
    source_c_hei: int,
    source_grid_size: int,
    c_wid: int,
    c_hei: int,
    grid_size: int,
    tile_grid_size: int,
    atlas_cols: int,
    value_to_tile: dict[int, int],
) -> int:
    """Emit one Tile per matching IntGrid cell. Returns count painted.

    Handles a target grid that's a multiple of the source grid:
    e.g. Collision IntGrid at 16px painting into a Tiles layer at
    32px (one tile per 2x2 IntGrid block). The aggregator picks
    the highest-priority value present in the block, where higher
    priority = the value listed earlier in `value_to_tile` (insertion
    order; Python 3.7+ dicts preserve it).
    """
    if grid_size % source_grid_size != 0:
        raise SystemExit(
            f"target grid {grid_size} must be a multiple of source grid "
            f"{source_grid_size}; got ratio {grid_size / source_grid_size}"
        )
    ratio = grid_size // source_grid_size

    # Build a priority list: value -> priority. Lower number = higher
    # priority (wins ties). Matches insertion order of value_to_tile.
    priority = {v: rank for rank, v in enumerate(value_to_tile.keys())}

    grid_tiles: list[dict] = []
    painted = 0
    for cy in range(c_hei):
        for cx in range(c_wid):
            # Collect source values in the rxratio block at (cx*ratio, cy*ratio).
            best_value: int | None = None
            best_priority = 1_000_000
            for dy in range(ratio):
                src_y = cy * ratio + dy
                if src_y >= source_c_hei:
                    continue
                row_off = src_y * source_c_wid
                for dx in range(ratio):
                    src_x = cx * ratio + dx
                    if src_x >= source_c_wid:
                        continue
                    v = int(source_csv[row_off + src_x])
                    if v in priority and priority[v] < best_priority:
                        best_priority = priority[v]
                        best_value = v
            if best_value is None:
                continue
            tile_id = value_to_tile[best_value]
            atlas_x, atlas_y = tile_src_px(tile_id, tile_grid_size, atlas_cols)
            grid_tiles.append(
                {
                    "a": 1,
                    "f": 0,
                    "px": [cx * grid_size, cy * grid_size],
                    "src": [atlas_x, atlas_y],
                    "t": tile_id,
                    "d": [cy * c_wid + cx],
                }
            )
            painted += 1
    layer_instance["gridTiles"] = grid_tiles
    return painted


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument(
        "action",
        choices=["paint"],
        help="Subcommand action.",
    )
    parser.add_argument("ldtk", type=Path, help="Target .ldtk file to modify.")
    parser.add_argument(
        "level",
        help="Level identifier (e.g. intro_wake_room). Use '*' or --all-levels for every level.",
    )
    parser.add_argument(
        "layer",
        help="Tiles layer identifier to paint into (e.g. IntroLabTiles).",
    )
    parser.add_argument(
        "--from-intgrid",
        type=str,
        required=True,
        metavar="LAYER",
        help="Source IntGrid layer to read cell values from (e.g. Collision).",
    )
    parser.add_argument(
        "--map",
        action="append",
        default=[],
        metavar="VALUE=TILE",
        help="Per-IntGrid-value tile mapping. Repeat for multiple values.",
    )
    parser.add_argument(
        "--all-levels",
        action="store_true",
        help="Paint every level in the project (overrides positional 'level').",
    )
    parser.add_argument(
        "--overwrite",
        action="store_true",
        help="Overwrite an existing non-empty gridTiles list. Default refuses.",
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

    if args.action != "paint":
        return _fail(f"unknown tileset action '{args.action}'")
    if not args.in_place and args.output is None:
        return _fail("choose --in-place or --output <path>")
    if not args.ldtk.exists():
        return _fail(f"ldtk file not found: {args.ldtk}")

    project = json.loads(args.ldtk.read_text())
    layer_def = find_layer_def(project, args.layer)
    if layer_def.get("__type") != "Tiles":
        return _fail(
            f"layer '{args.layer}' has type {layer_def.get('__type')!r}; "
            f"paint targets a Tiles layer."
        )
    tileset_uid = int(layer_def["tilesetDefUid"])
    tileset_def = find_tileset_def_by_uid(project, tileset_uid)
    tile_grid_size = int(tileset_def["tileGridSize"])
    atlas_cols = int(tileset_def["__cWid"])

    value_to_tile = parse_map(args.map)

    if args.all_levels or args.level == "*":
        levels = find_levels(project, None)
    else:
        levels = find_levels(project, args.level)

    total_painted = 0
    levels_painted = 0
    for level in levels:
        layer_instance = find_layer_instance(level, args.layer)
        source_layer = find_layer_instance(level, args.from_intgrid)
        if source_layer.get("__type") != "IntGrid":
            return _fail(
                f"--from-intgrid '{args.from_intgrid}' on level "
                f"'{level['identifier']}' is type "
                f"{source_layer.get('__type')!r}; need IntGrid."
            )
        if layer_instance.get("gridTiles") and not args.overwrite:
            print(
                f"skip {level['identifier']}/{args.layer}: "
                f"{len(layer_instance['gridTiles'])} existing tiles; pass --overwrite to replace",
                file=sys.stderr,
            )
            continue
        c_wid = int(layer_instance["__cWid"])
        c_hei = int(layer_instance["__cHei"])
        source_c_wid = int(source_layer["__cWid"])
        source_c_hei = int(source_layer["__cHei"])
        grid_size = int(layer_instance["__gridSize"])
        source_grid_size = int(source_layer["__gridSize"])
        # Accept identical grids or a target-is-multiple-of-source
        # downsample (e.g. 16px Collision -> 32px Tiles, 2x2 block).
        if grid_size % source_grid_size != 0:
            return _fail(
                f"grid mismatch on {level['identifier']}: target {args.layer} "
                f"grid {grid_size}px is not a multiple of source "
                f"{args.from_intgrid} grid {source_grid_size}px."
            )
        ratio = grid_size // source_grid_size
        # Allow odd source dimensions; the downsampler clamps inside
        # the source loop.
        expected_c_wid = (source_c_wid + ratio - 1) // ratio
        expected_c_hei = (source_c_hei + ratio - 1) // ratio
        if c_wid != expected_c_wid or c_hei != expected_c_hei:
            return _fail(
                f"grid size mismatch on {level['identifier']}: target "
                f"{args.layer} {c_wid}x{c_hei} at {grid_size}px doesn't "
                f"line up with source {args.from_intgrid} {source_c_wid}x"
                f"{source_c_hei} at {source_grid_size}px (expected "
                f"{expected_c_wid}x{expected_c_hei})."
            )
        painted = paint_layer(
            layer_instance,
            source_layer.get("intGridCsv", []),
            source_c_wid,
            source_c_hei,
            source_grid_size,
            c_wid,
            c_hei,
            grid_size,
            tile_grid_size,
            atlas_cols,
            value_to_tile,
        )
        print(f"painted {painted} tiles on {level['identifier']}/{args.layer}")
        total_painted += painted
        levels_painted += 1

    print(f"painted {total_painted} tiles across {levels_painted} level(s)")

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
