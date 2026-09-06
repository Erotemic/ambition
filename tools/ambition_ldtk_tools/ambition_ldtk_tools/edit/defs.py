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

# # Usage

```bash
PYTHONPATH=tools/ambition_ldtk_tools \
python -m ambition_ldtk_tools def register-entity spec.yaml --in-place
```

# # Spec format (YAML or JSON)

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
1. Refuses an entity identifier the project already has. That refusal is the
   command's meaning — *I believe this noun is new; tell me if I am wrong* —
   and it is deliberately not overridable here. To make an EXISTING definition
   match a spec, use `def upsert-entity`, which preserves the uids every
   placement in every level references; deleting the def and registering it
   again parses fine and empties the level.
2. Allocates a fresh `uid` from the project's `nextUid` for both the
   entity def and each field def.
3. Synthesizes the LDtk editor-roundtrip metadata (`__type`, `type`
   internal constructor, `allowedRefs`, `realEditorValues`, etc.) so
   the resulting file passes both Ambition validation and the
   official LDtk JSON schema.
4. Declares the identifier in the LDtk AUTHORING CONTRACT
   (`crates/ambition_platformer2d_ldtk/ldtk_entity_contract.json`) so the
   validator stops calling it "unsupported". Field RULES are not written
   there — the converter does not exist yet, and the Rust prover would
   catch any guess.
5. Runs `ambition_ldtk_tools repair --in-place` and
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
from ambition_ldtk_tools.ldtk.paths import (  # noqa: E402
    default_entity_contract,
    default_sandbox_ldtk,
)

# tools/ambition_ldtk_tools/ambition_ldtk_tools/edit/defs.py -> repo root
REPO_ROOT = Path(__file__).resolve().parents[4]
PKG_DIR = Path(__file__).resolve().parents[1]
SANDBOX_LDTK = default_sandbox_ldtk()
VALIDATOR = PKG_DIR / "validate.py"
# The LDtk authoring contract — the single table the validator reads and the Rust
# `contract::prover` proves. `default_entity_contract` owns the path so this file
# does not become the next stale hardcoded consumer (see the note below).
ENTITY_CONTRACT = default_entity_contract()
# A half-succeeded tool is worse than a failed one.
#
# a hardcoded consumer path in a tool is the same class as the renderer
# submodules that hardcode the consumer crate name: nothing compiles it, so a
# rename cannot break it until someone runs the command.
RUNTIME_RS = (
    REPO_ROOT
    / "crates"
    / "ambition_platformer2d_ldtk"
    / "src"
    / "bevy_runtime"
    / "plugin.rs"
)

HUMAN_TO_INTERNAL = {
    "Int": "F_Int",
    "Float": "F_Float",
    "String": "F_String",
    "Bool": "F_Bool",
    # **`Enum` is the type that turns a field into a DROPDOWN.** A manifest
    # declaring one also declares its values (`"enum"` + `"values"`), because a
    # closed vocabulary is the point: `ldtk_vocabulary.rs` parses `kind`
    # case-insensitively and says why — *"an author typing into a free-text
    # field is a real possibility until the enum def lands in the project."*
    # This is that enum def landing.
    #
    # the INTERNAL type is `F_Enum(<uid>)`, so it cannot be a constant here;
    # `enum_field_type` resolves it against the project, creating the enum def
    # when the project has not seen it yet.
    "Enum": None,
}


def ensure_enum_def(project: dict, identifier: str, values: list[str]) -> dict:
    """Find or create a local enum def, preserving uids the levels point at.

    ⭐ **the value SET is owned here, the value's PICTURE is not.** A manifest
    says which words are sayable; `asset editor-art` fills in each value's
    `tileRect` afterwards, and re-running either must not undo the other. So
    this reconciles the list of ids and leaves every other key on a value it
    already knows alone.
    """
    enums = project.setdefault("defs", {}).setdefault("enums", [])
    existing = next((e for e in enums if e.get("identifier") == identifier), None)
    if existing is None:
        existing = {
            "identifier": identifier,
            "uid": alloc_uid(project),
            "values": [],
            "iconTilesetUid": None,
            "externalRelPath": None,
            "externalFileChecksum": None,
            "tags": [],
        }
        enums.append(existing)
    by_id = {value.get("id"): value for value in existing.get("values") or []}
    existing["values"] = [
        by_id.get(value, {"id": value, "tileRect": None, "color": 0})
        for value in values
    ]
    return existing


def enum_field_type(project: dict, spec_field: dict) -> tuple[str, str]:
    """`(__type, type)` for an `Enum` field, creating its enum def if needed."""
    identifier = spec_field.get("enum")
    if not identifier:
        raise SystemExit(
            f"field {spec_field.get('name')!r} is an Enum but names no `enum`"
        )
    values = [str(value) for value in spec_field.get("values") or []]
    if not values:
        raise SystemExit(f"enum {identifier!r} declares no `values`")
    enum = ensure_enum_def(project, str(identifier), values)
    return f"LocalEnum.{identifier}", f"F_Enum({enum['uid']})"


def resolve_field_type(project: dict, spec_field: dict) -> tuple[str, str]:
    """`(__type, type)` for any manifest field, enum or not."""
    human = spec_field.get("type")
    if human == "Enum":
        return enum_field_type(project, spec_field)
    if human not in HUMAN_TO_INTERNAL:
        raise SystemExit(
            f"unsupported field type {human!r}; supported: {sorted(HUMAN_TO_INTERNAL)}"
        )
    return human, HUMAN_TO_INTERNAL[human]


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


def field_def(
    name: str, human_type: str, default, project: dict, spec_field: dict | None = None
) -> dict:
    """Build a `fieldDefs[]` entry with the editor-roundtrip metadata
    `ambition_ldtk_tools repair` would otherwise have to fill in.

    `spec_field` is the manifest entry this came from, which an `Enum` needs
    (its `enum` identifier and `values` live there and nowhere else).
    """
    human_type, internal = resolve_field_type(
        project, spec_field or {"name": name, "type": human_type}
    )
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


def repair_and_validate(target: Path, schema: Path | None) -> int:
    """The post-pass every definition edit owes the file it just wrote.

    Editing `defs` desynchronizes the instances that reference them —
    `realEditorValues` records go stale the moment a default moves, and a
    freshly-minted field def has no matching `defUid` anywhere. `repair` is the
    tool that already knows how to derive all of that, so a definition editor's
    job ends at the definitions and this hands the rest to it.

    Three commands owed this and each had written its own copy; the third one
    (`upsert-entity`) is what made a shared seam cheaper than a fourth.
    """
    cmd = [sys.executable, "-m", "ambition_ldtk_tools.repair", str(target), "--in-place"]
    print("$ " + " ".join(cmd))
    rc = subprocess.run(cmd).returncode
    if rc != 0:
        return rc
    cmd = [sys.executable, "-m", "ambition_ldtk_tools.validate", str(target)]
    if schema and schema.exists():
        cmd.extend(["--schema", str(schema), "--require-schema"])
    print("$ " + " ".join(cmd))
    return subprocess.run(cmd).returncode


def _default_override(human_type: str, value):
    """LDtk's `defaultOverride` shape."""
    if value is None:
        return None
    wrapper = {
        "String": "V_String",
        "Bool": "V_Bool",
        "Int": "V_Int",
        "Float": "V_Float",
        # An enum default is stored as the value's id — the same `V_String`
        # wrapper LDtk writes for `BreakablePlatform.collision`.
        "Enum": "V_String",
    }.get(human_type if human_type in {"String", "Bool", "Int", "Float"} else "Enum")
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
        field_def(f["name"], f["type"], f.get("default"), project, f) for f in fields
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
    """Declare `identifiers` in the LDtk AUTHORING CONTRACT.

    ⛔ **this used to regex a literal `KNOWN_ENTITIES` set inside `validate.py`,
    and that set was the second authority.** It had already drifted — `SurfaceRamp`
    was registered in `standard_converters()` and absent there, so the validator
    called the engine's own fillet unsupported. The validator now reads
    `crates/ambition_platformer2d_ldtk/ldtk_entity_contract.json`, which the Rust
    `contract::prover` pins against the converter registry in both directions, so
    this writes there instead.

    ⚠ **a bare entry declares the identifier and NO field rules**, which is the
    truthful thing to write: `register-entity` runs before any converter exists,
    and the tool cannot know what the parser will refuse. Fill the `fields` list in
    when the converter lands — the prover will tell you the moment a claim there is
    not true.

    Returns the names that were actually added (sorted) so the caller can report.
    """
    document = json.loads(ENTITY_CONTRACT.read_text())
    existing = {entity["identifier"] for entity in document["entities"]}
    additions = sorted({name for name in identifiers if name not in existing})
    if not additions:
        return []
    for name in additions:
        document["entities"].append(
            {
                "identifier": name,
                "probe_size": [16, 16],
                "note": (
                    "declared by `def register-entity`; its field rules are not "
                    "written yet — add them as the converter grows refusals"
                ),
                "fields": [],
            }
        )
    document["entities"].sort(key=lambda entity: entity["identifier"])
    ENTITY_CONTRACT.write_text(json.dumps(document, indent=2) + "\n")
    return additions


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument(
        "spec", type=Path, help="YAML or JSON spec with `entities` list"
    )
    parser.add_argument("--ldtk", type=Path, default=SANDBOX_LDTK)
    parser.add_argument("--in-place", action="store_true", help="write to --ldtk")
    parser.add_argument(
        "--game-owned",
        action="store_true",
        help=(
            "this entity belongs to a GAME, not the engine: register it in the "
            "project only, and leave the engine's vocabulary lists alone"
        ),
    )
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
        # **A GAME'S NOUN MUST NOT JOIN THE ENGINE'S VOCABULARY.**
        # The authoring contract is checked two ways — every project is expected
        # to DEFINE all of it, and no instance may fall outside it — so adding
        # `MaryOBlock` there would push a block only Mary-O has onto every other
        # world. It is also the list the Rust prover pins against
        # `standard_converters()`, so a game entity in it makes the engine claim a
        # vocabulary it cannot convert.
        # ⭐ THERE IS NO LONGER A MARKER LIST TO PATCH. This used to write the
        # identifier into `AMBITION_LDTK_ENTITY_IDENTIFIERS` as a third place the
        # same noun had to be spelled, and the comment here claimed "a test pins
        # them equal" -- which was never true and is how the list drifted by two
        # entries. The plugin now DERIVES its registrations from the converter
        # vocabulary, so an engine entity gets its marker by existing.
        # A game registers its own through `install_ldtk_entity_converters`; the
        # project's `defs` is where tooling sees it, and the validator accepts an
        # identifier the project defines.
        added_validator = [] if args.game_owned else patch_validator_known_entities(new_identifiers)

    if args.no_repair:
        return 0

    return repair_and_validate(target, args.schema)


def _fail(msg: str) -> int:
    print(f"error: {msg}", file=sys.stderr)
    return 2


if __name__ == "__main__":
    raise SystemExit(main())
