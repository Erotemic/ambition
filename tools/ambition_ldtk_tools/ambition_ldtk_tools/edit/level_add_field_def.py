#!/usr/bin/env python3
"""Register a new levelField DEFINITION in an LDtk project.

Companion to `level set-field` (which sets a field's *instance* value on a
level). Before a spec or `set-field` can author a level field, the project's
`defs.levelFields` must DEFINE it. This adds that definition — a fresh `uid`
from `nextUid`, the LDtk-shaped def dict — so authored specs stop hitting
"the project has no matching levelField def".

The def dict is cloned from an existing levelField in the same project so every
LDtk-required key is present, then the identity/type keys are overridden. When
the project has no levelFields yet, a minimal-but-complete def is built.

Usage:

    ambition_ldtk_tools level add-field-def <name> --type Bool|String|Int|Float
        [--doc TEXT] [--default V] <ldtk> (--in-place | --output PATH)
        [--backup] [--no-repair]

Idempotent: re-adding a field whose identifier already exists is a no-op
(the type is checked; a type mismatch is an error, not a silent overwrite).
"""

from __future__ import annotations

import argparse
import copy
import sys
from pathlib import Path

# tools/ambition_ldtk_tools/ambition_ldtk_tools/edit/level_add_field_def.py -> repo root.
REPO_ROOT = Path(__file__).resolve().parents[4]

from ambition_ldtk_tools.edit.defs import alloc_uid
from ambition_ldtk_tools.edit.postprocess import run_repair_and_validate
from ambition_ldtk_tools.ldtk.transaction import LdtkTransaction

# LDtk's `__type` -> internal `type` tag.
_TYPE_TAGS = {
    "String": "F_String",
    "Bool": "F_Bool",
    "Int": "F_Int",
    "Float": "F_Float",
}


def _coerce_default(human_type: str, raw: str):
    if raw is None:
        return None
    if human_type == "Bool":
        return raw.lower() in {"true", "yes", "1"}
    if human_type == "Int":
        return int(raw)
    if human_type == "Float":
        return float(raw)
    return str(raw)


def _blank_field_template() -> dict:
    """A complete levelField def with LDtk's full key set (used when the
    project has no existing levelField to clone from)."""
    return {
        "identifier": "",
        "doc": None,
        "__type": "String",
        "uid": 0,
        "type": "F_String",
        "isArray": False,
        "canBeNull": True,
        "arrayMinLength": None,
        "arrayMaxLength": None,
        "editorDisplayMode": "NameAndValue",
        "editorDisplayScale": 1,
        "editorDisplayPos": "Above",
        "editorLinkStyle": "ZigZag",
        "editorDisplayColor": None,
        "editorAlwaysShow": False,
        "editorShowInWorld": False,
        "editorCutLongValues": True,
        "editorTextSuffix": None,
        "editorTextPrefix": None,
        "useForSmartColor": False,
        "exportToToc": False,
        "searchable": True,
        "min": None,
        "max": None,
        "regex": None,
        "acceptFileTypes": None,
        "defaultOverride": None,
        "textLanguageMode": None,
        "symmetricalRef": False,
        "autoChainRef": False,
        "allowOutOfLevelRef": False,
        "allowedRefs": "Any",
        "allowedRefsEntityUid": None,
        "allowedRefTags": [],
        "tilesetUid": None,
    }


def _add_level_field(project: dict, name: str, human_type: str, doc, default) -> str:
    defs = project.setdefault("defs", {})
    level_fields = defs.setdefault("levelFields", [])

    existing = next((f for f in level_fields if f.get("identifier") == name), None)
    if existing is not None:
        if existing.get("__type") != human_type:
            raise SystemExit(
                f"level field '{name}' already exists with type "
                f"{existing.get('__type')} != requested {human_type}"
            )
        return None

    template = copy.deepcopy(level_fields[0]) if level_fields else _blank_field_template()
    template["identifier"] = name
    template["__type"] = human_type
    template["type"] = _TYPE_TAGS[human_type]
    template["uid"] = alloc_uid(project)
    template["doc"] = doc
    template["isArray"] = False
    template["min"] = None
    template["max"] = None
    template["regex"] = None
    if default is None:
        template["canBeNull"] = True
        template["defaultOverride"] = None
    else:
        template["canBeNull"] = False
        template["defaultOverride"] = {"id": f"V_{human_type}", "params": [default]}

    level_fields.append(template)
    return f"  - added levelField '{name}' ({human_type}, uid={template['uid']})"


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("name", help="Level field identifier (e.g. `gallery`)")
    parser.add_argument("ldtk", type=Path, help="LDtk project file")
    parser.add_argument(
        "--type",
        dest="field_type",
        choices=sorted(_TYPE_TAGS),
        required=True,
    )
    parser.add_argument("--doc", default=None, help="Editor doc string")
    parser.add_argument("--default", default=None, help="Default value (typed by --type)")
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

    default = _coerce_default(args.field_type, args.default)
    tx = LdtkTransaction(
        args.ldtk,
        in_place=args.in_place,
        output=args.output,
        backup=args.backup,
    )
    summary = _add_level_field(tx.project, args.name, args.field_type, args.doc, default)
    if summary is None:
        print(f"  - levelField '{args.name}' already defined (no-op)")
    else:
        print(summary)
        tx.note_changed([summary])

    target_path = tx.finish(
        noop_message="level add-field-def: nothing changed",
        write_message="wrote {path}",
    )
    if target_path is None or args.no_repair:
        return 0
    return run_repair_and_validate(target_path, args.schema)


if __name__ == "__main__":
    raise SystemExit(main())
