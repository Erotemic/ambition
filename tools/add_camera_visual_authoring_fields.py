#!/usr/bin/env python3
"""Add CameraZoneSpec and RoomVisualProfile authoring fields to sandbox.ldtk.

This script updates the LDtk project schema only. Runtime support for these
fields lives in `ldtk_world::conversion` and `RoomMetadata`.
"""
from __future__ import annotations

import json
from pathlib import Path

from ambition_ldtk_tools.edit.defs import field_def, write_project

ROOT = Path(__file__).resolve().parents[1]
DEFAULT_LDTK = ROOT / "crates/ambition_sandbox/assets/ambition/worlds/sandbox.ldtk"


def add_field(container: list[dict], project: dict, name: str, human_type: str, default, doc: str, *, show: bool = False) -> bool:
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


def main() -> None:
    import argparse

    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("path", nargs="?", type=Path, default=DEFAULT_LDTK)
    args = parser.parse_args()

    project = json.loads(args.path.read_text())
    defs = project["defs"]
    changed = False

    camera_zone = next(
        entity for entity in defs["entities"] if entity.get("identifier") == "CameraZone"
    )
    camera_zone["doc"] = (
        "Authored camera behavior zone. Runtime parses priority, zoom, offset, "
        "easing, lock, and clamp fields into CameraZoneSpec."
    )
    field_defs = camera_zone.setdefault("fieldDefs", [])
    changed |= add_field(field_defs, project, "priority", "Int", 0, "Higher-priority CameraZone wins when zones overlap.", show=True)
    changed |= add_field(field_defs, project, "zoom", "Float", None, "Optional camera zoom multiplier. Empty preserves the legacy 1.15 breath-out.", show=True)
    changed |= add_field(field_defs, project, "target_offset_x", "Float", 0, "World-space camera target offset X applied while active.")
    changed |= add_field(field_defs, project, "target_offset_y", "Float", 0, "World-space camera target offset Y applied while active.")
    changed |= add_field(field_defs, project, "easing_hz", "Float", None, "Optional camera target easing speed in hertz.")
    changed |= add_field(field_defs, project, "cinematic_lock", "Bool", False, "When true, target the zone center instead of the player.")
    changed |= add_field(field_defs, project, "clamp_mode", "String", "room_bounds", "room_bounds, zone_bounds, or none.")

    level_fields = defs.setdefault("levelFields", [])
    changed |= add_field(level_fields, project, "visual_profile", "String", None, "Stable RoomVisualProfile id, e.g. intro_wakeup_room.")
    changed |= add_field(level_fields, project, "parallax_theme", "String", None, "Explicit generated parallax/background theme.")
    changed |= add_field(level_fields, project, "palette", "String", None, "Optional renderer palette/color-grading hint.")
    changed |= add_field(level_fields, project, "lighting_hint", "String", None, "Optional lighting mood hint.")
    changed |= add_field(level_fields, project, "foreground_treatment", "String", None, "Optional foreground/atmosphere treatment hint.")

    if changed:
        write_project(args.path, project)
        print(f"updated {args.path}")
    else:
        print(f"{args.path} already has camera/visual authoring fields")


if __name__ == "__main__":
    main()
