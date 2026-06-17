#!/usr/bin/env python3
"""Add CameraZoneSpec and RoomVisualProfile authoring fields to sandbox.ldtk.

This updates the LDtk project schema only. Runtime support for these fields
lives in `ldtk_world::conversion` and `RoomMetadata`.

The script is intentionally dry-run by default because `.ldtk` files are large
and any schema edit should be reviewed as a focused diff.

Usage:

    PYTHONPATH=tools/ambition_ldtk_tools \
    python tools/add_camera_visual_authoring_fields.py --dry-run

    PYTHONPATH=tools/ambition_ldtk_tools \
    python tools/add_camera_visual_authoring_fields.py --in-place

After `--in-place`, inspect and validate:

    git diff -- crates/ambition_gameplay_core/assets/ambition/worlds/sandbox.ldtk
    PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools doctor \
      crates/ambition_gameplay_core/assets/ambition/worlds/sandbox.ldtk
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
DEFAULT_LDTK = ROOT / "crates/ambition_gameplay_core/assets/ambition/worlds/sandbox.ldtk"
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


def apply_authoring_fields(project: dict) -> list[str]:
    changed: list[str] = []
    defs = project["defs"]
    camera_zone = next(
        entity
        for entity in defs["entities"]
        if entity.get("identifier") == "CameraZone"
    )
    new_doc = (
        "Authored camera behavior zone. Runtime parses priority, zoom, offset, "
        "easing, lock, and clamp fields into CameraZoneSpec."
    )
    if camera_zone.get("doc") != new_doc:
        camera_zone["doc"] = new_doc
        changed.append("updated CameraZone doc")
    field_defs = camera_zone.setdefault("fieldDefs", [])
    specs = [
        (
            field_defs,
            "priority",
            "Int",
            0,
            "Higher-priority CameraZone wins when zones overlap.",
            True,
        ),
        (
            field_defs,
            "zoom",
            "Float",
            None,
            "Optional camera zoom multiplier. Empty preserves the legacy 1.15 breath-out.",
            True,
        ),
        (
            field_defs,
            "target_offset_x",
            "Float",
            0,
            "World-space camera target offset X applied while active.",
            False,
        ),
        (
            field_defs,
            "target_offset_y",
            "Float",
            0,
            "World-space camera target offset Y applied while active.",
            False,
        ),
        (
            field_defs,
            "easing_hz",
            "Float",
            None,
            "Optional camera target easing speed in hertz.",
            False,
        ),
        (
            field_defs,
            "cinematic_lock",
            "Bool",
            False,
            "When true, target the zone center instead of the player.",
            False,
        ),
        (
            field_defs,
            "clamp_mode",
            "String",
            "room_bounds",
            "room_bounds, zone_bounds, or none.",
            False,
        ),
    ]
    level_fields = defs.setdefault("levelFields", [])
    specs.extend(
        [
            (
                level_fields,
                "visual_profile",
                "String",
                None,
                "Stable RoomVisualProfile id, e.g. intro_wakeup_room.",
                False,
            ),
            (
                level_fields,
                "parallax_theme",
                "String",
                None,
                "Explicit generated parallax/background theme.",
                False,
            ),
            (
                level_fields,
                "palette",
                "String",
                None,
                "Optional renderer palette/color-grading hint.",
                False,
            ),
            (
                level_fields,
                "lighting_hint",
                "String",
                None,
                "Optional lighting mood hint.",
                False,
            ),
            (
                level_fields,
                "foreground_treatment",
                "String",
                None,
                "Optional foreground/atmosphere treatment hint.",
                False,
            ),
        ]
    )
    for container, name, human_type, default, doc, show in specs:
        if add_field(container, project, name, human_type, default, doc, show=show):
            changed.append(f"added field {name}")
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
    parser.add_argument(
        "--output", type=Path, default=None, help="Write to a separate LDtk path"
    )
    parser.add_argument(
        "--dry-run", action="store_true", help="Print planned changes without writing"
    )
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
        print(f"{args.path} already has camera/visual authoring fields")
        return 0

    print("planned LDtk schema changes:")
    for item in changed:
        print(f"  - {item}")

    if args.dry_run:
        print("dry-run only; no file written")
        print("to apply:")
        print(
            f"  PYTHONPATH=tools/ambition_ldtk_tools python tools/add_camera_visual_authoring_fields.py {shlex.quote(display_path(args.path))} --in-place"
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
