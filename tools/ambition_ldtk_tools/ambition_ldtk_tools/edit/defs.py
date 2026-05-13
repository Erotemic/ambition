#!/usr/bin/env python3
"""Register a new Ambition LDtk entity definition into `sandbox.ldtk`.

Adding a new entity type the agent (or LDtk editor) can place requires
a `defs.entities[]` entry with a fresh `uid`, a properly-shaped
`fieldDefs` list (each with the right `defUid`, internal `type`
constructor, and editor-roundtrip metadata), and a matching color so
the LDtk editor can render the entity. Doing that by hand is the same
class of editor-roundtrip pain that `ambition_ldtk_tools area create` solved for
levels — this tool solves it for entity definitions.

Pairs with `python -m ambition_ldtk_tools area create`: register the def first, then
author levels that place the new entity.

## Usage

```bash
PYTHONPATH=tools/ambition_ldtk_tools \
python -m ambition_ldtk_tools def register-entity spec.yaml --in-place
```

## Spec format (YAML or JSON)

```yaml
entities:
  - identifier: EncounterTrigger
    color: "#FF7AB6"        # required: editor color
    width: 64               # default authored size
    height: 64
    docs: "Activates the parent active area's encounter when the player enters."
    fields:
      - { name: id,         type: String, default: "" }
      - { name: name,       type: String, default: "" }
      - { name: camera_zoom, type: Float, default: 1.0 }

  - identifier: Switch
    color: "#FFC857"
    width: 24
    height: 32
    docs: "Latched player switch; persists in the save."
    fields:
      - { name: id,        type: String, default: "" }
      - { name: name,      type: String, default: "" }
      - { name: prompt,    type: String, default: "Activate" }
      - { name: target_encounter, type: String, default: "" }
      - { name: action,    type: String, default: "ResetEncounter" }
```

The tool:
1. Refuses to overwrite an existing entity identifier (use a different
   identifier or delete the def by hand first).
2. Allocates a fresh `uid` from the project's `nextUid` for both the
   entity def and each field def.
3. Synthesizes the LDtk editor-roundtrip metadata (`__type`, `type`
   internal constructor, `allowedRefs`, `realEditorValues`, etc.) so
   the resulting file passes both Ambition validation and the
   official LDtk JSON schema.
4. Adds the identifier to `ambition_ldtk_tools validate`'s
   `KNOWN_ENTITIES` set on disk so the validator stops complaining
   about an "unsupported" identifier.
5. Adds the identifier to `bevy_runtime.rs`'s
   `AMBITION_LDTK_ENTITY_IDENTIFIERS` so `bevy_ecs_ldtk` registers a
   marker bundle for the new entity.
6. Runs `ambition_ldtk_tools repair --in-place` and
   `ambition_ldtk_tools validate --schema ... --require-schema`.
"""
from __future__ import annotations

import argparse
import json
import re
import shutil
import subprocess
import sys
from pathlib import Path

# tools/ambition_ldtk_tools/ambition_ldtk_tools/edit/defs.py -> repo root
REPO_ROOT = Path(__file__).resolve().parents[4]
PKG_DIR = Path(__file__).resolve().parents[1]
SANDBOX_LDTK = (
    REPO_ROOT
    / "crates"
    / "ambition_sandbox"
    / "assets"
    / "ambition"
    / "worlds"
    / "sandbox.ldtk"
)
VALIDATOR = PKG_DIR / "validate.py"
RUNTIME_RS = (
    REPO_ROOT
    / "crates"
    / "ambition_sandbox"
    / "src"
    / "ldtk_world"
    / "bevy_runtime.rs"
)

HUMAN_TO_INTERNAL = {
    "Int": "F_Int",
    "Float": "F_Float",
    "String": "F_String",
    "Bool": "F_Bool",
}


def load_spec(path: Path) -> dict:
    text = path.read_text()
    if path.suffix.lower() in {".yaml", ".yml"}:
        try:
            import yaml  # type: ignore
        except ImportError as ex:  # pragma: no cover
            raise SystemExit(f"YAML spec but pyyaml not installed: {ex}")
        return yaml.safe_load(text)
    return json.loads(text)


def load_project(path: Path) -> dict:
    return json.loads(path.read_text())


def write_project(path: Path, project: dict) -> None:
    from ambition_ldtk_tools.editor_format import dump_editor_style

    path.write_text(dump_editor_style(project))


def alloc_uid(project: dict) -> int:
    next_uid = int(project.get("nextUid", 1))
    project["nextUid"] = next_uid + 1
    return next_uid


def field_def(name: str, human_type: str, default, project: dict) -> dict:
    """Build a `fieldDefs[]` entry with the editor-roundtrip metadata
    `ambition_ldtk_tools repair` would otherwise have to fill in."""
    if human_type not in HUMAN_TO_INTERNAL:
        raise SystemExit(
            f"unsupported field type {human_type!r}; supported: {sorted(HUMAN_TO_INTERNAL)}"
        )
    internal = HUMAN_TO_INTERNAL[human_type]
    uid = alloc_uid(project)
    return {
        "identifier": name,
        "doc": None,
        "__type": human_type,
        "uid": uid,
        "type": internal,
        "isArray": False,
        "canBeNull": True,
        "arrayMinLength": None,
        "arrayMaxLength": None,
        "editorDisplayMode": "RefLinkBetweenCenters",
        "editorDisplayPos": "Above",
        "editorDisplayScale": 1.0,
        "editorDisplayColor": None,
        "editorAlwaysShow": False,
        "editorCutLongValues": True,
        "editorShowInWorld": True,
        "editorTextSuffix": None,
        "editorTextPrefix": None,
        "editorLinkStyle": "CurvedArrow",
        "useForSmartColor": False,
        "min": None,
        "max": None,
        "regex": None,
        "acceptFileTypes": None,
        "tilesetUid": None,
        "defaultOverride": _default_override(human_type, default),
        "textLanguageMode": None,
        "symmetricalRef": False,
        "autoChainRef": True,
        "allowOutOfLevelRef": True,
        "allowedRefs": "Any",
        "allowedRefsEntityUid": None,
        "allowedRefTags": [],
        "exportToToc": False,
        "searchable": True,
    }


def _default_override(human_type: str, value):
    """LDtk's `defaultOverride` shape."""
    if value is None:
        return None
    wrapper = {"String": "V_String", "Bool": "V_Bool", "Int": "V_Int", "Float": "V_Float"}[
        human_type
    ]
    if human_type == "Bool":
        params = [bool(value)]
    elif human_type == "Int":
        params = [int(value)]
    elif human_type == "Float":
        params = [float(value)]
    else:
        params = [str(value)]
    return {"id": wrapper, "params": params}


def build_entity_def(spec: dict, project: dict) -> dict:
    identifier = spec["identifier"]
    fields = spec.get("fields", [])
    color = spec.get("color", "#FFFFFF")
    width = int(spec.get("width", 16))
    height = int(spec.get("height", 16))
    docs = spec.get("docs")
    field_defs = [
        field_def(f["name"], f["type"], f.get("default"), project) for f in fields
    ]
    return {
        "identifier": identifier,
        "uid": alloc_uid(project),
        "tags": [],
        "exportToToc": False,
        "allowOutOfBounds": True,
        "doc": docs,
        "tileOpacity": 1.0,
        "lineOpacity": 1.0,
        "fillOpacity": 0.08,
        "hollow": False,
        "color": color,
        "renderMode": "Rectangle",
        "showName": True,
        "tilesetId": None,
        "tileRenderMode": "FitInside",
        "tileRect": None,
        "uiTileRect": None,
        "nineSliceBorders": [],
        "maxCount": 0,
        "limitScope": "PerLevel",
        "limitBehavior": "MoveLastOne",
        "pivotX": 0,
        "pivotY": 0,
        "tileId": None,
        "width": width,
        "height": height,
        "resizableX": True,
        "resizableY": True,
        "minWidth": None,
        "maxWidth": None,
        "minHeight": None,
        "maxHeight": None,
        "keepAspectRatio": False,
        "fieldDefs": field_defs,
    }


def patch_validator_known_entities(identifiers: list[str]) -> list[str]:
    """Add `identifiers` to `ambition_ldtk_tools validate`'s `KNOWN_ENTITIES` set.

    Returns the names that were actually added (sorted) so the caller
    can report.
    """
    text = VALIDATOR.read_text()
    match = re.search(r"KNOWN_ENTITIES = \{\s*([^}]+?)\}", text, flags=re.DOTALL)
    if not match:
        raise SystemExit(
            "could not find KNOWN_ENTITIES in ambition_ldtk_tools validate"
        )
    block = match.group(1)
    existing = set(re.findall(r'"([^"]+)"', block))
    additions = [name for name in identifiers if name not in existing]
    if not additions:
        return []
    new_set = sorted(existing | set(additions))
    rendered = "    " + ",\n    ".join(f'"{name}"' for name in new_set) + ",\n"
    new_text = (
        text[: match.start()] + "KNOWN_ENTITIES = {\n" + rendered + "}" + text[match.end() :]
    )
    VALIDATOR.write_text(new_text)
    return additions


def patch_runtime_identifiers(identifiers: list[str]) -> list[str]:
    """Add `identifiers` to `bevy_runtime.rs::AMBITION_LDTK_ENTITY_IDENTIFIERS`."""
    text = RUNTIME_RS.read_text()
    match = re.search(
        r"pub const AMBITION_LDTK_ENTITY_IDENTIFIERS: &\[&str\] = &\[\s*([^]]+?)\];",
        text,
        flags=re.DOTALL,
    )
    if not match:
        raise SystemExit(
            "could not find AMBITION_LDTK_ENTITY_IDENTIFIERS in bevy_runtime.rs"
        )
    block = match.group(1)
    existing = re.findall(r'"([^"]+)"', block)
    additions = [name for name in identifiers if name not in existing]
    if not additions:
        return []
    new_list = existing + additions
    rendered = "    " + ",\n    ".join(f'"{name}"' for name in new_list) + ",\n"
    new_text = (
        text[: match.start()]
        + "pub const AMBITION_LDTK_ENTITY_IDENTIFIERS: &[&str] = &[\n"
        + rendered
        + "];"
        + text[match.end() :]
    )
    RUNTIME_RS.write_text(new_text)
    return additions


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("spec", type=Path, help="YAML or JSON spec with `entities` list")
    parser.add_argument("--ldtk", type=Path, default=SANDBOX_LDTK)
    parser.add_argument("--in-place", action="store_true", help="write to --ldtk")
    parser.add_argument("--output", type=Path, default=None)
    parser.add_argument("--backup", action="store_true")
    parser.add_argument(
        "--no-repair", action="store_true", help="skip repair + validate post-pass"
    )
    parser.add_argument(
        "--no-source-patch",
        action="store_true",
        help="skip patching validator + runtime source files",
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

    if not args.in_place and args.output is None:
        parser.error("choose --in-place or --output <path>")

    spec = load_spec(args.spec)
    if not isinstance(spec, dict) or "entities" not in spec:
        return _fail("spec must be a mapping with an `entities` list")

    project = load_project(args.ldtk)
    existing = {e["identifier"] for e in project["defs"]["entities"]}
    new_identifiers: list[str] = []
    for ent_spec in spec["entities"]:
        identifier = ent_spec["identifier"]
        if identifier in existing:
            return _fail(
                f"entity identifier '{identifier}' already exists in the project"
            )
        ent_def = build_entity_def(ent_spec, project)
        project["defs"]["entities"].append(ent_def)
        new_identifiers.append(identifier)
        print(f"added entity def: {identifier} (uid={ent_def['uid']})")

    target = args.output or args.ldtk
    if args.in_place and args.backup:
        backup = args.ldtk.with_suffix(args.ldtk.suffix + ".bak")
        shutil.copy2(args.ldtk, backup)
        print(f"wrote backup: {backup}")
    write_project(target, project)
    print(f"wrote {target} with {len(new_identifiers)} new entity def(s)")

    if not args.no_source_patch:
        added_validator = patch_validator_known_entities(new_identifiers)
        if added_validator:
            print(f"validator KNOWN_ENTITIES += {added_validator}")
        added_runtime = patch_runtime_identifiers(new_identifiers)
        if added_runtime:
            print(f"bevy_runtime AMBITION_LDTK_ENTITY_IDENTIFIERS += {added_runtime}")

    if args.no_repair:
        return 0

    cmd = [sys.executable, "-m", "ambition_ldtk_tools.repair", str(target), "--in-place"]
    print("$ " + " ".join(cmd))
    r = subprocess.run(cmd)
    if r.returncode != 0:
        return r.returncode
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
