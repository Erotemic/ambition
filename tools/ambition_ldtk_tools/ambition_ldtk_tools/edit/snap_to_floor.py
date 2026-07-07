#!/usr/bin/env python3
"""Snap an existing entity straight down onto the nearest floor surface.

Companion to `entity move` (absolute reposition) and `entity measure`
(read-only placement probe). Where `move` needs you to know the exact y,
this reads the level's Collision IntGrid and drops the entity so its bottom
edge rests flush on the first Solid or OneWayUp surface beneath its current
x-span — the same floor-finding doors use via `connect_to ... snap_to_surface`,
exposed for any entity (switches, props, pickups, NPCs).

Why it exists: `measure` only probes for Solid(1) cells, so a one-way
platform floor (OneWayUp=2) reads as `down=edge` and hand-picked y values
end up floating. `snap_entity_to_surface` treats both as floor, so the
result is always grounded.

Typical use — place a switch in a door-free gap, then ground it:

    entity snap-to-floor --level central_hub_main --iid Switch-5939 \\
        --x 873 --in-place

Targeting mirrors `entity set-field`: `--iid` (preferred, survives renames)
or `--identifier ID` plus repeatable `--match key=value`. `--x` optionally
repositions horizontally before snapping; `--prefer-y` biases toward a
particular surface row; `--dry-run` reports the landing without writing.
"""

from __future__ import annotations

import argparse
import shutil
import subprocess
import sys
from pathlib import Path

# tools/ambition_ldtk_tools/ambition_ldtk_tools/edit/snap_to_floor.py -> repo root.
REPO_ROOT = Path(__file__).resolve().parents[4]

from ambition_ldtk_tools.area_authoring import (  # noqa: E402
    load_project,
    snap_entity_to_surface,
    write_project,
)
from ambition_ldtk_tools.edit.set_field import (  # noqa: E402
    find_ambition_layer,
    find_level,
    select_entities,
)


def _grid_size(project: dict, level: dict) -> int:
    for li in level.get("layerInstances", []):
        if li.get("__identifier") == "Ambition" and li.get("__gridSize"):
            return int(li["__gridSize"])
    return int(project.get("defaultGridSize", 16))


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument(
        "--ldtk",
        type=Path,
        default=REPO_ROOT
        / "crates"
        / "ambition_actors"
        / "assets"
        / "ambition"
        / "worlds"
        / "sandbox.ldtk",
    )
    parser.add_argument("--level", required=True, help="Level (room) identifier")
    parser.add_argument("--iid", help="Target entity iid (preferred)")
    parser.add_argument("--identifier", help="Target entity type (with --match)")
    parser.add_argument(
        "--match",
        action="append",
        default=[],
        metavar="KEY=VALUE",
        help="Field filter for --identifier (repeatable), e.g. --match id=hub_gravity_switch",
    )
    parser.add_argument(
        "--x",
        type=int,
        default=None,
        help="Reposition the entity to this level-local x before snapping",
    )
    parser.add_argument(
        "--prefer-y",
        type=int,
        default=None,
        help="Bias toward the surface whose snapped y is closest to this",
    )
    parser.add_argument("--in-place", action="store_true")
    parser.add_argument("--output", type=Path, default=None)
    parser.add_argument("--backup", action="store_true")
    parser.add_argument("--no-repair", action="store_true")
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="Report the landing y without modifying the file",
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
    if not args.iid and not args.identifier:
        parser.error("choose --iid or --identifier")
    if not args.dry_run and not args.in_place and args.output is None:
        parser.error("choose --in-place, --output <path>, or --dry-run")

    target: dict = {}
    if args.iid:
        target["iid"] = args.iid
    if args.identifier:
        target["identifier"] = args.identifier
        match: dict = {}
        for kv in args.match:
            if "=" not in kv:
                return _fail(f"--match expects KEY=VALUE, got {kv!r}")
            k, v = kv.split("=", 1)
            match[k] = v
        if match:
            target["match"] = match

    project = load_project(args.ldtk)
    level = find_level(project, args.level)
    layer = find_ambition_layer(level)
    grid_size = _grid_size(project, level)
    world_x = int(level.get("worldX", 0))
    world_y = int(level.get("worldY", 0))

    matched = select_entities(layer, target)
    if not matched:
        return _fail(f"no entity in '{args.level}' matched {target}")

    results: list[str] = []
    for entity in matched:
        old_px = list(entity.get("px") or [0, 0])
        x = args.x if args.x is not None else int(old_px[0])
        width = int(entity.get("width", grid_size))
        height = int(entity.get("height", grid_size))
        snapped_x, snapped_y, kind = snap_entity_to_surface(
            project, args.level, x, width, height, prefer_y=args.prefer_y
        )
        label = f"{entity.get('__identifier')} ({entity.get('iid')})"
        moved_x = "" if snapped_x == old_px[0] else f"x {old_px[0]}->{snapped_x}, "
        results.append(
            f"{label}: {moved_x}y {old_px[1]}->{snapped_y} (rests on {kind})"
        )
        if not args.dry_run:
            entity["px"] = [snapped_x, snapped_y]
            entity["__grid"] = [snapped_x // grid_size, snapped_y // grid_size]
            entity["__worldX"] = world_x + snapped_x
            entity["__worldY"] = world_y + snapped_y

    if args.dry_run:
        print(f"dry-run: would snap {len(results)} entity(ies):")
        for line in results:
            print(f"  {line}")
        return 0

    target_path = args.output or args.ldtk
    if args.in_place and args.backup:
        backup = args.ldtk.with_suffix(args.ldtk.suffix + ".bak")
        shutil.copy2(args.ldtk, backup)
        print(f"wrote backup: {backup}")
    write_project(target_path, project)
    print(f"snapped {len(results)} entity(ies):")
    for line in results:
        print(f"  {line}")
    if args.no_repair:
        return 0

    cmd = [
        sys.executable,
        "-m",
        "ambition_ldtk_tools.repair",
        str(target_path),
        "--in-place",
    ]
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
