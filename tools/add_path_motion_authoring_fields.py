#!/usr/bin/env python3
"""Add KinematicPath consumer fields to sandbox.ldtk.

This updates the LDtk project schema only. Runtime support lives in
`ldtk_world::conversion`, `RoomSpec::kinematic_paths`, `MovingPlatformState`,
`NpcRuntime`, and `EnemyRuntime`.

Dry-run is the default because `.ldtk` files are large and schema edits should
be reviewed as focused diffs.

Usage:

    PYTHONPATH=tools/ambition_ldtk_tools \
    python tools/add_path_motion_authoring_fields.py --dry-run

    PYTHONPATH=tools/ambition_ldtk_tools \
    python tools/add_path_motion_authoring_fields.py --in-place

After `--in-place`, inspect and validate:

    git diff -- crates/ambition_sandbox/assets/ambition/worlds/sandbox.ldtk
    PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools doctor \
      crates/ambition_sandbox/assets/ambition/worlds/sandbox.ldtk
"""
from __future__ import annotations

import argparse
import json
import shlex
from pathlib import Path

from ambition_ldtk_tools.edit.defs import field_def
from ambition_ldtk_tools.repair import write_project
from ambition_ldtk_tools.validate import normalize_project_for_editor, validate

ROOT = Path(__file__).resolve().parents[1]
DEFAULT_LDTK = ROOT / "crates/ambition_sandbox/assets/ambition/worlds/sandbox.ldtk"
DEFAULT_SCHEMA = ROOT / "tools/ambition_ldtk_tools/schemas/ldtk/JSON_SCHEMA.json"


def add_field(
    container: list[dict],
    project: dict,
    name: str,
    human_type: str,
    default,
    doc: str,
    *,
    show: bool = False,
) -> bool:
    if any(field.get("identifier") == name for field in container):
        return False
    field = field_def(name, human_type, default, project)
    field["doc"] = doc
    field["editorDisplayMode"] = "NameAndValue"
    field["editorAlwaysShow"] = show
    field["editorShowInWorld"] = False
    field["useForSmartColor"] = False
    container.append(field)
    return True


def entity_def(project: dict, identifier: str) -> dict:
    try:
        return next(
            entity
            for entity in project["defs"].get("entities", [])
            if entity.get("identifier") == identifier
        )
    except StopIteration as ex:
        raise SystemExit(f"LDtk entity definition '{identifier}' not found") from ex


def apply_authoring_fields(project: dict) -> list[str]:
    changed: list[str] = []
    defs = project["defs"]

    kin = entity_def(project, "KinematicPath")
    kin_fields = kin.setdefault("fieldDefs", [])
    if add_field(
        kin_fields,
        project,
        "id",
        "String",
        None,
        "Stable path lookup id. If empty, runtime derives an id from name/iid.",
        show=True,
    ):
        changed.append("added KinematicPath.id")

    moving = entity_def(project, "MovingPlatform")
    moving_doc = (
        "LDtk-authored moving platform. If path_id is set, the platform follows "
        "that KinematicPath; otherwise it uses sweep_dx/speed as a simple "
        "horizontal ping-pong."
    )
    if moving.get("doc") != moving_doc:
        moving["doc"] = moving_doc
        changed.append("updated MovingPlatform doc")
    if add_field(
        moving.setdefault("fieldDefs", []),
        project,
        "path_id",
        "String",
        None,
        "Optional KinematicPath id/name. When set, this overrides sweep_dx motion.",
        show=True,
    ):
        changed.append("added MovingPlatform.path_id")

    npc = entity_def(project, "NpcSpawn")
    if add_field(
        npc.setdefault("fieldDefs", []),
        project,
        "path_id",
        "String",
        None,
        "Optional KinematicPath id/name for NPC patrol. Overrides patrol_radius pacing.",
        show=False,
    ):
        changed.append("added NpcSpawn.path_id")

    enemy = entity_def(project, "EnemySpawn")
    if add_field(
        enemy.setdefault("fieldDefs", []),
        project,
        "path_id",
        "String",
        None,
        "Optional KinematicPath id/name. Equivalent to brain=Patrol:<id> for path patrols.",
        show=False,
    ):
        changed.append("added EnemySpawn.path_id")

    normalize_project_for_editor(project)
    return changed


def display_path(path: Path) -> str:
    try:
        return str(path.resolve().relative_to(ROOT))
    except ValueError:
        return str(path)


def cli_command(subcommand: str, path: Path, *extra: str) -> str:
    parts = [
        "PYTHONPATH=tools/ambition_ldtk_tools",
        "python",
        "-m",
        "ambition_ldtk_tools",
        subcommand,
        display_path(path),
        *extra,
    ]
    return " ".join(shlex.quote(part) for part in parts)


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("path", nargs="?", type=Path, default=DEFAULT_LDTK)
    parser.add_argument("--in-place", action="store_true", help="Rewrite the LDtk file")
    parser.add_argument("--output", type=Path, default=None, help="Write to a separate LDtk path")
    parser.add_argument("--dry-run", action="store_true", help="Print planned changes without writing")
    parser.add_argument("--schema", type=Path, default=DEFAULT_SCHEMA)
    parser.add_argument("--require-schema", action="store_true")
    args = parser.parse_args(argv)

    if args.in_place and args.output:
        parser.error("choose only one of --in-place or --output")
    if not args.in_place and args.output is None:
        args.dry_run = True

    project = json.loads(args.path.read_text())
    changed = apply_authoring_fields(project)
    if not changed:
        print(f"{args.path} already has path-motion authoring fields")
        return 0

    print("planned LDtk schema changes:")
    for item in changed:
        print(f"  - {item}")

    if args.dry_run:
        print("dry-run only; no file written")
        print("to apply:")
        print(
            "  PYTHONPATH=tools/ambition_ldtk_tools python "
            f"tools/add_path_motion_authoring_fields.py {shlex.quote(display_path(args.path))} --in-place"
        )
        return 0

    target = args.output or args.path
    write_project(target, project)
    errors, warnings = validate(target, args.schema, args.require_schema)
    for warning in warnings:
        print(f"warning: {warning}")
    for error in errors:
        print(f"error: {error}")
    if errors:
        return 1
    print(f"updated {target}")
    print("diagnostics:")
    print(f"  git diff -- {shlex.quote(display_path(target))}")
    print(f"  {cli_command('doctor', target)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
