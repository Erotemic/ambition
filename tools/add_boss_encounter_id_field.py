#!/usr/bin/env python3
"""Add the authored `BossSpawn.encounter_id` field, and name the Mockingbird.

⭐⭐ WHY THIS FIELD EXISTS — Jon's ruling on decision 57 (2026-09-05):

    Boss progress is keyed only by stable authored encounter/placement IDs.
    `boss.cleared(id)` means "has this specific authored boss encounter been
    cleared?"

⛔⛔ THE PLACEMENT ID USED TO BE THE LDtk IID, AND THAT MADE EVERY AUTHORED CALL
FALSE. `boss_encounter/src/systems.rs` writes the durable record under the
placement id and `boss.cleared` looks it up exactly — but an author writing Yarn
can only type a name they can know, so `cove.yarn` and `kernel.yarn` asked
`boss_cleared("mockingbird")` (the BEHAVIOUR id) against a save keyed
`BossSpawn-4308`. Three executable branches that could never open.

`convert_boss_spawn` now uses `encounter_id` as the placement's id when authored
and the iid otherwise, so the save key, `FeatureId`, the duplicate-id check and
mount links all read ONE id. Nothing here is a second progress table — Jon
rejected that explicitly, along with archetype save keys, "some boss of this
type", and runtime ECS translation.

⚠ The id names the narrative owner (`cove.mockingbird`), NOT the level
(`mockingbird_arena`): a durable save key that tracks a level identifier changes
when the level is renamed, which is the property an authored id exists to avoid.

Dry-run is the default, matching `add_path_motion_authoring_fields.py`, because
`.ldtk` files are large and schema edits should be reviewed as focused diffs.

    PYTHONPATH=tools/ambition_ldtk_tools python tools/add_boss_encounter_id_field.py --dry-run
    PYTHONPATH=tools/ambition_ldtk_tools python tools/add_boss_encounter_id_field.py <path> --in-place
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
DEFAULT_LDTK = ROOT / "game/ambition_content/assets/worlds/sandbox.ldtk"
DEFAULT_SCHEMA = ROOT / "tools/ambition_ldtk_tools/schemas/ldtk/JSON_SCHEMA.json"

FIELD = "encounter_id"
DOC = (
    "Stable AUTHORED encounter id — the key this boss's cleared state persists "
    "under and the id `boss_cleared(...)` takes. Empty means fall back to the "
    "LDtk iid, which no author can type; name it before gating dialogue on it."
)
# The one placement authored so far, keyed by the LDtk iid it currently carries.
NAMED_PLACEMENTS = {"BossSpawn-4308": "cove.mockingbird"}


def entity_def(project: dict, identifier: str) -> dict | None:
    return next(
        (
            entity
            for entity in project["defs"].get("entities", [])
            if entity.get("identifier") == identifier
        ),
        None,
    )


def apply(project: dict) -> list[str]:
    changed: list[str] = []
    boss = entity_def(project, "BossSpawn")
    if boss is None:
        return changed

    fields = boss.setdefault("fieldDefs", [])
    if not any(f.get("identifier") == FIELD for f in fields):
        field = field_def(FIELD, "String", None, project)
        field["doc"] = DOC
        field["editorDisplayMode"] = "NameAndValue"
        field["editorAlwaysShow"] = True
        field["editorShowInWorld"] = False
        field["useForSmartColor"] = False
        fields.append(field)
        changed.append(f"added BossSpawn.{FIELD}")

    definition = next(f for f in fields if f.get("identifier") == FIELD)
    for level in project.get("levels", []):
        for layer in level.get("layerInstances") or []:
            for inst in layer.get("entityInstances", []):
                if inst.get("__identifier") != "BossSpawn":
                    continue
                authored = NAMED_PLACEMENTS.get(inst.get("iid"))
                if authored is None:
                    continue
                instances = inst.setdefault("fieldInstances", [])
                existing = next(
                    (f for f in instances if f.get("__identifier") == FIELD), None
                )
                if existing is not None:
                    if existing.get("__value") != authored:
                        existing["__value"] = authored
                        existing["realEditorValues"] = [
                            {"id": "V_String", "params": [authored]}
                        ]
                        changed.append(f"set {inst['iid']}.{FIELD} = {authored}")
                    continue
                instances.append(
                    {
                        "__identifier": FIELD,
                        "__type": "String",
                        "__value": authored,
                        "__tile": None,
                        "defUid": definition["uid"],
                        "realEditorValues": [
                            {"id": "V_String", "params": [authored]}
                        ],
                    }
                )
                changed.append(f"set {inst['iid']}.{FIELD} = {authored}")

    normalize_project_for_editor(project)
    return changed


def display_path(path: Path) -> str:
    try:
        return str(path.resolve().relative_to(ROOT))
    except ValueError:
        return str(path)


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("path", nargs="?", type=Path, default=DEFAULT_LDTK)
    parser.add_argument("--in-place", action="store_true")
    parser.add_argument("--dry-run", action="store_true")
    parser.add_argument("--schema", type=Path, default=DEFAULT_SCHEMA)
    parser.add_argument("--require-schema", action="store_true")
    args = parser.parse_args(argv)
    if not args.in_place:
        args.dry_run = True

    project = json.loads(args.path.read_text())
    changed = apply(project)
    if not changed:
        print(f"{args.path}: nothing to do")
        return 0
    print("planned LDtk changes:")
    for item in changed:
        print(f"  - {item}")
    if args.dry_run:
        print("dry-run only; no file written")
        return 0
    write_project(args.path, project)
    errors, warnings = validate(args.path, args.schema, args.require_schema)
    for warning in warnings:
        print(f"warning: {warning}")
    for error in errors:
        print(f"error: {error}")
    if errors:
        return 1
    print(f"updated {display_path(args.path)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
