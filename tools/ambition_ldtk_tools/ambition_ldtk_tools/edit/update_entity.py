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

It does NOT (today): remove fields, rename fields, change field
types, or update the validator / runtime identifier lists (those
are tied to entity identifier, not field identifier). Add those
as `--remove-field` / `--rename-field` flags when the use case
lands.
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
)


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
    if not args.add_field:
        return _fail("at least one --add-field is required")

    project = json.loads(args.ldtk.read_text())
    ent = find_entity_def(project, args.identifier)
    existing_field_ids = {f.get("identifier") for f in ent.get("fieldDefs", [])}

    added: list[str] = []
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
