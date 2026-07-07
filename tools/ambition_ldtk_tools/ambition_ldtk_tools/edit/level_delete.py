#!/usr/bin/env python3
"""Delete a whole level (room) from an LDtk file.

Companion to `entity delete` (which removes entity *instances* inside a
level). Use this when a level is being **relocated to another `.ldtk`
file** — e.g. moving the generated Hall of Characters out of the
monolithic `sandbox.ldtk` into its own secondary world — or when a room
is genuinely retired.

Cross-file `LoadingZone`s that point *into* the deleted level (a hub
door targeting it) are intentionally left untouched: once the level lives
in a secondary world, the runtime merge resolves the target across files,
exactly like the cut-rope arena. Validate the pair afterwards with
`validate <primary> --secondary-world <secondary>`.

Usage:

    ambition_ldtk_tools level delete <level_id> [--ldtk PATH]
        (--in-place | --output PATH) [--backup] [--no-repair]

`<level_id>` is the level identifier (e.g. `hall_of_characters`). The
repair + validate post-pass runs on the way out (skip with --no-repair);
it recomputes Free-layout neighbours after the removal.
"""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

# tools/ambition_ldtk_tools/ambition_ldtk_tools/edit/level_delete.py -> repo root.
REPO_ROOT = Path(__file__).resolve().parents[4]

from ambition_ldtk_tools.edit.postprocess import run_repair_and_validate
from ambition_ldtk_tools.ldtk.transaction import LdtkTransaction


def _remove_level(project: dict, level_id: str) -> dict:
    """Remove the level with `identifier == level_id` from every place LDtk
    stores levels (top-level `levels`, and each `worlds[].levels` for
    multi-world files). Returns the removed level dict. Errors if absent."""
    removed: dict | None = None

    def drop(levels: list) -> list:
        nonlocal removed
        kept = []
        for lev in levels:
            if lev.get("identifier") == level_id:
                removed = lev
            else:
                kept.append(lev)
        return kept

    project["levels"] = drop(project.get("levels", []))
    for world in project.get("worlds", []) or []:
        if "levels" in world:
            world["levels"] = drop(world["levels"])

    if removed is None:
        present = ", ".join(
            l.get("identifier", "?") for l in project.get("levels", [])
        )
        raise SystemExit(f"level '{level_id}' not found. Levels: {present}")
    return removed


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("level_id", help="Identifier of the level to delete")
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
        print("error: choose --in-place or --output <path>", file=sys.stderr)
        return 2

    tx = LdtkTransaction(
        args.ldtk,
        in_place=args.in_place,
        output=args.output,
        backup=args.backup,
    )
    removed = _remove_level(tx.project, args.level_id)
    summary = (
        f"  - deleted level '{args.level_id}' "
        f"({removed.get('pxWid')}x{removed.get('pxHei')} at "
        f"{removed.get('worldX')},{removed.get('worldY')})"
    )
    print(summary)
    print(f"deleted level '{args.level_id}'")
    tx.note_changed([summary])

    target_path = tx.finish(
        noop_message="level delete: nothing changed",
        write_message="wrote {path}",
    )
    if target_path is None or args.no_repair:
        return 0
    return run_repair_and_validate(target_path, args.schema)


if __name__ == "__main__":
    raise SystemExit(main())
