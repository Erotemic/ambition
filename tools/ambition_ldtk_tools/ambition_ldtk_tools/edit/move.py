#!/usr/bin/env python3
"""Move an existing entity instance to a new pixel position.

Companion to `entity add` / `entity set-field`. Use this when an
authored entity needs to nudge — most often after `door snap`
returns a different y from the one originally fed to `area create`.

Spec format (YAML or JSON):

    level_id: central_hub_main
    moves:
      - target:
          # Either iid (preferred, survives renames):
          iid: LoadingZone-4310
          # …or identifier + match (matches set_field.py semantics):
          # identifier: LoadingZone
          # match:
          #   id: ninja_dojo_door
        px: [376, 880]   # new top-left in level-local pixels
        # `size` is optional — if present it also rewrites width/height.
        # size: [48, 96]

The `__grid` array is recomputed from `px` using the level's grid
size so LDtk's editor view stays consistent. The repair + validate
pass runs on the way out, identical to `entity set-field`.
"""
from __future__ import annotations

import argparse
import json
import shutil
import subprocess
import sys
from pathlib import Path

# tools/ambition_ldtk_tools/ambition_ldtk_tools/edit/move.py -> repo root.
REPO_ROOT = Path(__file__).resolve().parents[4]

from ambition_ldtk_tools.area_authoring import (  # noqa: E402
    load_project,
    write_project,
)
from ambition_ldtk_tools.edit.set_field import (  # noqa: E402
    find_ambition_layer,
    find_level,
    load_spec,
    select_entities,
)


def _grid_size(project: dict, level: dict) -> int:
    # Each layer instance carries its own gridSize; the Ambition layer
    # is what we mutate so use that one. Fall back to project default.
    for li in level.get("layerInstances", []):
        if li.get("__identifier") == "Ambition" and li.get("__gridSize"):
            return int(li["__gridSize"])
    return int(project.get("defaultGridSize", 16))


def apply_move(entity: dict, px, size, grid_size: int) -> None:
    if not (isinstance(px, (list, tuple)) and len(px) == 2):
        raise SystemExit(f"`px` must be [x, y]; got {px!r}")
    nx, ny = int(px[0]), int(px[1])
    entity["px"] = [nx, ny]
    entity["__grid"] = [nx // grid_size, ny // grid_size]
    if size is not None:
        if not (isinstance(size, (list, tuple)) and len(size) == 2):
            raise SystemExit(f"`size` must be [w, h]; got {size!r}")
        entity["width"] = int(size[0])
        entity["height"] = int(size[1])


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("spec", type=Path)
    parser.add_argument(
        "--ldtk",
        type=Path,
        default=REPO_ROOT
        / "crates"
        / "ambition_sandbox"
        / "assets"
        / "ambition"
        / "worlds"
        / "sandbox.ldtk",
    )
    parser.add_argument("--in-place", action="store_true")
    parser.add_argument("--output", type=Path, default=None)
    parser.add_argument("--backup", action="store_true")
    parser.add_argument("--no-repair", action="store_true")
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
    if not args.in_place and args.output is None:
        parser.error("choose --in-place or --output <path>")

    spec = load_spec(args.spec)
    if not isinstance(spec, dict) or "level_id" not in spec or "moves" not in spec:
        return _fail("spec must be a mapping with `level_id` and `moves`")

    project = load_project(args.ldtk)
    level = find_level(project, spec["level_id"])
    layer = find_ambition_layer(level)
    grid_size = _grid_size(project, level)

    moves: list[str] = []
    for move in spec["moves"]:
        target = move.get("target") or {}
        px = move.get("px")
        size = move.get("size")
        if px is None:
            return _fail("each move must include `px: [x, y]`")
        matched = select_entities(layer, target)
        for entity in matched:
            old_px = list(entity.get("px") or [])
            apply_move(entity, px, size, grid_size)
            moves.append(
                f"{entity.get('__identifier')} ({entity.get('iid')}): "
                f"px {old_px} -> {entity['px']}"
                + (f", size -> {entity['width']}x{entity['height']}" if size else "")
            )

    target_path = args.output or args.ldtk
    if args.in_place and args.backup:
        backup = args.ldtk.with_suffix(args.ldtk.suffix + ".bak")
        shutil.copy2(args.ldtk, backup)
        print(f"wrote backup: {backup}")
    write_project(target_path, project)
    print(f"applied {len(moves)} move(s):")
    for line in moves:
        print(f"  {line}")
    if args.no_repair:
        return 0

    cmd = [sys.executable, "-m", "ambition_ldtk_tools.repair", str(target_path), "--in-place"]
    print("$ " + " ".join(cmd))
    if subprocess.run(cmd).returncode != 0:
        return 1
    cmd = [sys.executable, "-m", "ambition_ldtk_tools.validate", str(target_path)]
    if args.schema and args.schema.exists():
        cmd.extend(["--schema", str(args.schema), "--require-schema"])
    print("$ " + " ".join(cmd))
    return subprocess.run(cmd).returncode


def _fail(msg: str) -> int:
    print(f"error: {msg}", file=sys.stderr)
    return 2


if __name__ == "__main__":
    raise SystemExit(main())
