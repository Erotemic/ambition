#!/usr/bin/env python3
"""Add a new field to an existing Ambition LDtk entity definition.

Companion to `def register-entity`: that tool creates new entity
defs; this one extends an existing def with one (or more) new
fields. Mirrors `register-entity`'s field-def synthesis so the
editor + validator round-trip stays clean.

The immediate motivation is ADR 0016 (Actor unification): the
`Actor` entity def needs `aggression`, `dialogue_id`, `brain`, and
`path_id` fields layered on top of the existing `name` baseline
without re-creating the entity from scratch and losing references.

## Usage

```bash
PYTHONPATH=tools/ambition_ldtk_tools \\
python -m ambition_ldtk_tools def update-entity Actor \\
    game/ambition_content/assets/worlds/sandbox.ldtk \\
    --add-field aggression:String:Peaceful \\
    --add-field dialogue_id:String: \\
    --add-field brain:String: \\
    --add-field path_id:String: \\
    --in-place
```

The format for `--add-field` is `name:type[:default]`. Supported
types match `register-entity`: `Int`, `Float`, `String`, `Bool`.
A trailing empty default (`name:Type:`) is treated as a null
default (LDtk renders the field as unset / `null`).

A native reference field has its own flag, because it is a
RELATIONSHIP rather than a value and carries no default:

```bash
python -m ambition_ldtk_tools def update-entity EnemySpawn \\
    game/ambition_content/assets/worlds/intro.ldtk \\
    --add-entity-ref-field path_ref --in-place
```

What `path_ref` points at and how the LDtk editor scopes the
targets it offers are declared once in
`ambition_ldtk_tools.ldtk.fields.ENTITY_REF_FIELDS`, not on this
command line — a reference field nobody documented is the string
convention native refs replaced. Adding one is idempotent, so a
sync script may run it forever.

The tool:

1. Refuses to add a duplicate field identifier (use a different
   name or remove the existing field by hand first).
2. Allocates a fresh `uid` for each new `fieldDef` from the
   project's `nextUid`.
3. Synthesizes the LDtk editor-roundtrip metadata so the result
   passes both Ambition validation and the official LDtk JSON
   schema.
4. Runs the standard `repair --in-place` + `validate
   --require-schema` post-pass (`--no-repair` skips).

It does NOT remove fields, rename fields, or change field types.
`def upsert-entity` does all three, driven by a whole manifest
rather than one flag at a time, and it refuses the two that
destroy authored values unless the operator names them — which is
the reason those never landed here as `--remove-field` /
`--rename-field`. Nor does either tool touch the validator /
runtime identifier lists from a field edit: those are tied to the
entity identifier, not the field identifier.
"""

from __future__ import annotations

import argparse
import json
import shutil
import subprocess
import sys
from pathlib import Path

# .../ambition_ldtk_tools/edit/update_entity.py -> repo root
REPO_ROOT = Path(__file__).resolve().parents[4]

# Reuse register-entity's field-def synthesis so the editor
# round-trip metadata stays in one place.
from ambition_ldtk_tools.edit.defs import (  # noqa: E402
    HUMAN_TO_INTERNAL,
    field_def as _new_field_def,
    repair_and_validate,
)
from ambition_ldtk_tools.ldtk.fields import ensure_entity_ref_fielddef  # noqa: E402


def parse_add_field(spec: str) -> tuple[str, str, object | None]:
    """`name:type[:default]` -> `(name, type, default-or-None)`.

    A trailing empty default (`name:String:`) is treated as None
    so the LDtk field reads as unset, matching `register-entity`
    semantics when `default: null` is in the YAML.
    """
    parts = spec.split(":", 2)
    if len(parts) < 2:
        raise SystemExit(f"--add-field expects 'name:type[:default]', got {spec!r}")
    name, human_type = parts[0], parts[1]
    default: object | None
    if len(parts) == 3:
        raw = parts[2]
        default = raw if raw != "" else None
    else:
        default = None
    if not name:
        raise SystemExit(f"--add-field name is empty in {spec!r}")
    if human_type not in HUMAN_TO_INTERNAL:
        raise SystemExit(
            f"--add-field unsupported type {human_type!r}; supported: "
            f"{sorted(HUMAN_TO_INTERNAL)}"
        )
    return name, human_type, default


def parse_add_enum_field(spec: str) -> dict:
    """`name:EnumName:v1|v2[:default]` -> a manifest-shaped `spec_field`.

    ⭐ **a separate flag rather than a type inside `--add-field`**, for the same
    reason `--add-entity-ref-field` is separate: an enum carries a NAME and a
    VALUE SET that a `name:type[:default]` triple has nowhere to put, and
    smuggling them through a fourth colon-delimited slot would make the common
    scalar case harder to read to serve the rare one.

    ⚠ **the enum def is shared, not owned by this field.** `ensure_enum_def`
    reuses an existing enum of the same name and rewrites its value list, so two
    fields may name one enum — and re-running with different values RETYPES it.
    That is the same reconciliation `def upsert-entity` performs; it is spelled
    here so an in-place edit does not have to go through a manifest to get a
    dropdown.
    """
    parts = spec.split(":", 3)
    if len(parts) < 3:
        raise SystemExit(
            "--add-enum-field expects 'name:EnumName:v1|v2[:default]', "
            f"got {spec!r}"
        )
    name, enum_identifier, raw_values = parts[0], parts[1], parts[2]
    default = parts[3] if len(parts) == 4 and parts[3] != "" else None
    values = [value for value in raw_values.split("|") if value]
    if not name:
        raise SystemExit(f"--add-enum-field name is empty in {spec!r}")
    if not enum_identifier:
        raise SystemExit(f"--add-enum-field names no enum in {spec!r}")
    if not values:
        raise SystemExit(
            f"--add-enum-field declares no values in {spec!r}; an enum with no "
            "values is a field nothing can be set to"
        )
    if default is not None and default not in values:
        raise SystemExit(
            f"--add-enum-field default {default!r} is not one of {values} in "
            f"{spec!r} — the editor would offer a value the enum cannot hold"
        )
    return {
        "name": name,
        "type": "Enum",
        "enum": enum_identifier,
        "values": values,
        "default": default,
    }


def find_entity_def(project: dict, identifier: str) -> dict:
    for ent in project.get("defs", {}).get("entities", []):
        if ent.get("identifier") == identifier:
            return ent
    raise SystemExit(
        f"entity '{identifier}' not found in project; use `def register-entity` "
        f"first or check the spelling."
    )


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument(
        "action",
        choices=["update-entity"],
        help="Subcommand action.",
    )
    parser.add_argument("identifier", help="Entity identifier to extend.")
    parser.add_argument("ldtk", type=Path, help="Target .ldtk file.")
    parser.add_argument(
        "--add-field",
        action="append",
        default=[],
        metavar="name:type[:default]",
        help=(
            "Add a new field to the entity def. Repeat to add several. "
            "type ∈ {Int, Float, String, Bool}. Empty default = null."
        ),
    )
    parser.add_argument(
        "--add-enum-field",
        action="append",
        default=[],
        metavar="name:EnumName:v1|v2[:default]",
        help=(
            "Add an enum-typed field, minting or reusing the LocalEnum. This is "
            "what gives the LDtk editor a DROPDOWN instead of free text, so a "
            "human authors the same closed value set an agent reads from the "
            "entity contract. Repeat to add several."
        ),
    )
    parser.add_argument(
        "--add-entity-ref-field",
        action="append",
        default=[],
        metavar="name",
        help=(
            "Add a native EntityRef field (a relationship, not a value). The "
            "name must be one `ldtk.fields.ENTITY_REF_FIELDS` documents — what "
            "it points at and how the editor scopes it live there, not on this "
            "command line. Repeat to add several."
        ),
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

    if args.action != "update-entity":
        return _fail(f"unknown def action '{args.action}'")
    if not args.in_place and args.output is None:
        return _fail("choose --in-place or --output <path>")
    if not args.ldtk.exists():
        return _fail(f"ldtk file not found: {args.ldtk}")
    if not args.add_field and not args.add_entity_ref_field and not args.add_enum_field:
        return _fail(
            "at least one --add-field, --add-enum-field or "
            "--add-entity-ref-field is required"
        )

    project = json.loads(args.ldtk.read_text())
    ent = find_entity_def(project, args.identifier)
    existing_field_ids = {f.get("identifier") for f in ent.get("fieldDefs", [])}

    added: list[str] = []
    # a REFERENCE field, not a value field: its ~30-key LDtk shape and the
    # editor scope of what it may point at belong to the relationship, so this
    # command names the relationship and `ensure_entity_ref_fielddef` writes it.
    # Idempotent by construction, which is what makes it safe in a sync script.
    for name in args.add_entity_ref_field:
        made = ensure_entity_ref_fielddef(project, args.identifier, name)
        if name in existing_field_ids:
            print(f"entity '{args.identifier}' already has EntityRef '{name}'; left as is")
            continue
        existing_field_ids.add(name)
        added.append(f"{name}:EntityRef->{made['allowedRefs']}")

    # enums first, so a `--add-field` that collides with one reports the
    # collision rather than winning the race and leaving a free-text field where
    # the author asked for a dropdown.
    for spec in args.add_enum_field:
        spec_field = parse_add_enum_field(spec)
        name = spec_field["name"]
        if name in existing_field_ids:
            return _fail(
                f"entity '{args.identifier}' already has a field '{name}'; use "
                f"a different name or remove the existing field first."
            )
        ent.setdefault("fieldDefs", []).append(
            _new_field_def(name, "Enum", spec_field["default"], project, spec_field)
        )
        existing_field_ids.add(name)
        added.append(
            f"{name}:LocalEnum.{spec_field['enum']}"
            f"{spec_field['values']}={spec_field['default']!r}"
        )

    for spec in args.add_field:
        name, human_type, default = parse_add_field(spec)
        if name in existing_field_ids:
            return _fail(
                f"entity '{args.identifier}' already has a field "
                f"'{name}'; use a different name or remove the "
                f"existing field first."
            )
        ent.setdefault("fieldDefs", []).append(
            _new_field_def(name, human_type, default, project)
        )
        existing_field_ids.add(name)
        added.append(f"{name}:{human_type}={default!r}")

    print(
        f"updated entity '{args.identifier}': added {len(added)} field(s): "
        + ", ".join(added)
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

    return repair_and_validate(target, args.schema)


def _fail(msg: str) -> int:
    print(f"error: {msg}", file=sys.stderr)
    return 2


if __name__ == "__main__":
    raise SystemExit(main())
