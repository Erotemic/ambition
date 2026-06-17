#!/usr/bin/env python3
"""Add a Tiles layer definition + empty per-level instances to an LDtk project.

Companion to `tileset add` (which registers a tileset def from a PNG).
This tool wires that tileset into the level-authoring pipeline by:

1. Adding a Tiles layer def to `defs.layers[]` whose
   `tilesetDefUid` points at the registered tileset.
2. Adding an empty Tiles layer instance to every existing level
   so the layer schema stays consistent across the project. An
   empty layer has `gridTiles: []` and renders nothing until
   tiles are painted.

The layer's `gridSize` matches the tileset's `tileGridSize` so
each painted cell maps 1:1 to a tile in the source PNG. Layers
are visual-only (ADR 0015 §Tiles = visuals); collision still
flows from the existing Collision IntGrid layer.

Companion subcommand `tileset paint` actually populates
`gridTiles[]` from the active Collision IntGrid (or a manual
recipe).

## Usage

```bash
PYTHONPATH=tools/ambition_ldtk_tools \\
python -m ambition_ldtk_tools tileset add-layer \\
    crates/ambition_gameplay_core/assets/ambition/worlds/intro.ldtk \\
    intro_lab \\
    --layer-identifier IntroLabTiles \\
    --in-place
```

`tileset_identifier` is the registered tileset's `identifier`
(matches what `tileset add` printed). `--layer-identifier`
defaults to `<TilesetIdentifier>Tiles` (PascalCased).

The tool refuses to overwrite an existing layer identifier;
pass a different `--layer-identifier` to disambiguate.

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

# Reuse the iid allocator that already lives in area_authoring so
# every layer instance picks up a non-colliding `<Identifier>-NNNN`
# iid that bumps the project's `nextUid`.
from ambition_ldtk_tools.area_authoring import allocate_iid  # noqa: E402


def find_tileset_def(project: dict, identifier: str) -> dict:
    for ts in project.get("defs", {}).get("tilesets", []):
        if ts.get("identifier") == identifier:
            return ts
    raise SystemExit(
        f"tileset '{identifier}' not found in project; use `tileset add` "
        f"first or check the spelling."
    )


def find_layer_def(project: dict, identifier: str) -> dict | None:
    for layer in project.get("defs", {}).get("layers", []):
        if layer.get("identifier") == identifier:
            return layer
    return None


def alloc_uid(project: dict) -> int:
    next_uid = int(project.get("nextUid", 1))
    project["nextUid"] = next_uid + 1
    return next_uid


def build_tiles_layer_def(
    project: dict,
    identifier: str,
    tileset_def: dict,
    display_opacity: float,
) -> dict:
    """Mirrors `area_authoring::ensure_climbable_layer_def`'s shape
    but as a Tiles layer (vs IntGrid) with `tilesetDefUid` set.
    """
    grid_size = int(tileset_def["tileGridSize"])
    return {
        "__type": "Tiles",
        "identifier": identifier,
        "type": "Tiles",
        "uid": alloc_uid(project),
        "doc": f"Tiles layer backed by tileset '{tileset_def['identifier']}'.",
        "uiColor": None,
        "gridSize": grid_size,
        "guideGridWid": 0,
        "guideGridHei": 0,
        "displayOpacity": float(display_opacity),
        "inactiveOpacity": max(0.0, display_opacity - 0.3),
        "hideInList": False,
        "hideFieldsWhenInactive": True,
        "canSelectWhenInactive": True,
        "renderInWorldView": True,
        "pxOffsetX": 0,
        "pxOffsetY": 0,
        "parallaxFactorX": 0,
        "parallaxFactorY": 0,
        "parallaxScaling": True,
        "requiredTags": [],
        "excludedTags": [],
        "autoTilesKilledByOtherLayerUid": None,
        "uiFilterTags": [],
        "useAsyncRender": False,
        "intGridValues": [],
        "intGridValuesGroups": [],
        "autoRuleGroups": [],
        "autoSourceLayerDefUid": None,
        "tilesetDefUid": int(tileset_def["uid"]),
        "tilePivotX": 0,
        "tilePivotY": 0,
        "biomeFieldUid": None,
    }


def add_empty_layer_instance_to_levels(
    project: dict,
    layer_def: dict,
    tileset_def: dict,
) -> int:
    """Add an empty Tiles layer instance to every level that doesn't
    already have one with this identifier. Returns the count of
    levels mutated.
    """
    grid_size = int(layer_def["gridSize"])
    identifier = layer_def["identifier"]
    mutated = 0
    for level in project.get("levels", []):
        if any(
            lyr.get("__identifier") == identifier
            for lyr in level.get("layerInstances", [])
        ):
            continue
        c_wid = level["pxWid"] // grid_size
        c_hei = level["pxHei"] // grid_size
        layer_iid, _ = allocate_iid(project, identifier)
        empty_layer = {
            "__identifier": identifier,
            "__type": "Tiles",
            "iid": layer_iid,
            "layerDefUid": int(layer_def["uid"]),
            "__cWid": c_wid,
            "__cHei": c_hei,
            "__gridSize": grid_size,
            "__opacity": 1,
            "__pxTotalOffsetX": 0,
            "__pxTotalOffsetY": 0,
            "__tilesetDefUid": int(tileset_def["uid"]),
            "__tilesetRelPath": tileset_def.get("relPath"),
            "levelId": level["uid"],
            "pxOffsetX": 0,
            "pxOffsetY": 0,
            "visible": True,
            "optionalRules": [],
            "intGridCsv": [],
            "autoLayerTiles": [],
            "seed": level["uid"],
            "overrideTilesetUid": None,
            "gridTiles": [],
            "entityInstances": [],
        }
        level.setdefault("layerInstances", []).append(empty_layer)
        mutated += 1
    return mutated


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument(
        "action",
        choices=["add-layer"],
        help="Subcommand action.",
    )
    parser.add_argument("ldtk", type=Path, help="Target .ldtk file to modify.")
    parser.add_argument(
        "tileset_identifier",
        help="Registered tileset identifier (matches `tileset add` output).",
    )
    parser.add_argument(
        "--layer-identifier",
        type=str,
        default=None,
        help="Identifier for the new layer (default: <Tileset>Tiles).",
    )
    parser.add_argument(
        "--display-opacity",
        type=float,
        default=1.0,
        help="Editor display opacity (0.0..1.0). Default 1.0.",
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

    if args.action != "add-layer":
        return _fail(f"unknown tileset action '{args.action}'")
    if not args.in_place and args.output is None:
        return _fail("choose --in-place or --output <path>")
    if not args.ldtk.exists():
        return _fail(f"ldtk file not found: {args.ldtk}")

    project = json.loads(args.ldtk.read_text())
    tileset_def = find_tileset_def(project, args.tileset_identifier)

    # Default layer identifier: <Tileset>Tiles, PascalCased. Convert
    # snake_case to PascalCase so `intro_lab` -> `IntroLabTiles`.
    layer_identifier = args.layer_identifier
    if layer_identifier is None:
        parts = args.tileset_identifier.replace("-", "_").split("_")
        layer_identifier = "".join(p.capitalize() for p in parts) + "Tiles"

    existing = find_layer_def(project, layer_identifier)
    if existing is not None:
        # Backfill mode: the layer def is already in `defs.layers`, but
        # one or more levels may be missing the matching layer instance
        # (this happens when `area create --replace-existing` rewrites a
        # level without re-emitting Tiles layers). Re-run the
        # idempotent per-level adder so missing instances get an empty
        # `gridTiles: []` entry; levels that already have one are
        # skipped.
        layer_def = existing
        mutated = add_empty_layer_instance_to_levels(project, layer_def, tileset_def)
        print(
            f"Tiles layer '{layer_identifier}' already declared "
            f"(uid={layer_def['uid']}); backfilled {mutated} missing level instance(s)"
        )
        if mutated == 0:
            print("nothing to do; exiting without writing")
            return 0
    else:
        layer_def = build_tiles_layer_def(
            project=project,
            identifier=layer_identifier,
            tileset_def=tileset_def,
            display_opacity=args.display_opacity,
        )
        project["defs"].setdefault("layers", []).append(layer_def)
        mutated = add_empty_layer_instance_to_levels(project, layer_def, tileset_def)
        print(
            f"added Tiles layer def: {layer_identifier} (uid={layer_def['uid']}, "
            f"gridSize={layer_def['gridSize']}, tileset={tileset_def['identifier']})"
        )
        print(f"added empty Tiles layer instance to {mutated} level(s)")

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
