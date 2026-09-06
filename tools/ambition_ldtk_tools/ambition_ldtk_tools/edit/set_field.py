#!/usr/bin/env python3
"""Set field instance values on existing LDtk entities.

Use this instead of hand-editing the LDtk JSON when you need to flip a
flag (e.g. `bidirectional`) or rename a target on an entity that's
already in the project. Mutating through the tool means the file goes
through the standard repair + validate pass on the way out, keeping
`__smartColor`, cached `__worldX`/`__worldY`, `realEditorValues`, and the field defs aligned.

Spec format (YAML or JSON):

    level_id: pirate_cove
    edits:
      - target:
          # Either select by `iid` (most precise; survives renames):
          iid: LoadingZone-4310
          # …or by entity identifier + a field/value match (for stable
          # surface keys like `id`):
          # identifier: LoadingZone
          # match:
          #   id: pirate_cove_to_arena
        fields:
          bidirectional: false
          target_zone: mockingbird_arena_locked

An `EntityRef` field (ADR 0020's `mounted_on`) names the TARGET, not the
four-key object LDtk stores:

    fields:
      mounted_on: EnemySpawn-6806        # the target's iid, and that is all

⛔ **the other three keys are DERIVED from the file being edited, never
written in a spec.** LDtk stores a ref as `{entityIid, layerIid, levelIid,
worldIid}`; a spec that carried its own `layerIid` would keep pointing at
whatever the container was called on the day it was typed, and a stale one
produces a ref that resolves to nothing while looking perfectly authored.
Naming the target and reading its containers out of the project is the only
version of this that cannot go stale — the same rule `build_level`'s spec-local
`ref:` handles already follow (`_resolve_entity_ref_handles`). Pass `null` to
clear a ref.

The tool errors out if:
  * the level doesn't exist;
  * no entity matches the target selector (or more than one does);
  * an `EntityRef` names an iid that is not in the project (a dangling mount
    link is the silent failure this whole path exists to prevent);
  * a field name isn't declared on the entity def (loud catch — silent
    write-through would leave the LDtk editor refusing to load the
    field next time).
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

# tools/ambition_ldtk_tools/ambition_ldtk_tools/edit/set_field.py -> repo root.
REPO_ROOT = Path(__file__).resolve().parents[4]

from ambition_ldtk_tools.edit.postprocess import run_repair_and_validate
from ambition_ldtk_tools.ldtk.transaction import LdtkTransaction

from ambition_ldtk_tools.area_authoring import (  # noqa: E402
    coerce_field_value,
    find_entity_def,
    load_project,
    make_field_instance,
    write_project,
)
from ambition_ldtk_tools.ldtk.paths import default_sandbox_ldtk  # noqa: E402


def load_spec(path: Path) -> dict:
    text = path.read_text()
    if path.suffix.lower() in {".yaml", ".yml"}:
        try:
            import yaml  # type: ignore
        except ImportError as ex:  # pragma: no cover
            raise SystemExit(f"YAML spec but pyyaml not installed: {ex}")
        return yaml.safe_load(text)
    return json.loads(text)


def find_level(project: dict, level_id: str) -> dict:
    for lev in project.get("levels", []):
        if lev.get("identifier") == level_id:
            return lev
    raise SystemExit(
        f"level '{level_id}' not found. Levels: "
        + ", ".join(l.get("identifier") for l in project.get("levels", []))
    )


def find_ambition_layer(level: dict) -> dict:
    for li in level.get("layerInstances", []):
        if li.get("__identifier") == "Ambition":
            return li
    raise SystemExit(f"level '{level['identifier']}' has no Ambition entity layer")


def _entity_field_value(entity: dict, field_name: str):
    for fi in entity.get("fieldInstances", []):
        if fi.get("__identifier") == field_name:
            return fi.get("__value")
    return None


def select_entities(layer: dict, target: dict) -> list[dict]:
    """Return the list of entity instances matching `target`. Raises a
    `SystemExit` when zero or more than one entity matches — set-field
    is intentionally strict so a stale spec doesn't quietly mutate the
    wrong door."""
    instances = layer.get("entityInstances", [])
    iid = target.get("iid")
    if iid is not None:
        matched = [e for e in instances if e.get("iid") == iid]
        if not matched:
            raise SystemExit(f"no entity with iid '{iid}' in level")
        return matched
    identifier = target.get("identifier")
    match = target.get("match") or {}
    if identifier is None:
        raise SystemExit(
            "target must include either `iid` or `identifier` (with optional `match`)"
        )
    candidates = [e for e in instances if e.get("__identifier") == identifier]
    for fname, fvalue in match.items():
        candidates = [e for e in candidates if _entity_field_value(e, fname) == fvalue]
    if not candidates:
        raise SystemExit(f"no entity '{identifier}' matched fields {match!r}")
    if len(candidates) > 1:
        ids = [c.get("iid", "<no-iid>") for c in candidates]
        raise SystemExit(
            f"target '{identifier}' / {match!r} is ambiguous, matched: {ids}. "
            f"Tighten the match selector or use iid."
        )
    return candidates


def _check_allowed_ref(
    field_def: dict | None, source: dict | None, target_entity: dict
) -> None:
    """Enforce the field def's own `allowedRefs` scope.

    ⭐ **the def already SAYS what the field may point at, and nothing read it.**
    LDtk's editor honours `allowedRefs` when a human draws the link; a ref written
    through this tool bypassed the editor entirely, so a `path_ref` at a
    `LoadingZone` or a rider `mounted_on` a `Prop` wrote cleanly and failed at
    load. This asks the declaration rather than re-deriving what each
    relationship means — every ref field is covered the moment its def exists.
    """
    if not field_def:
        return
    scope = field_def.get("allowedRefs")
    kind = target_entity.get("__identifier")
    if scope == "OnlySame" and source is not None:
        if kind != source.get("__identifier"):
            raise SystemExit(
                f"'{field_def['identifier']}' declares allowedRefs=OnlySame, so it "
                f"may only point at another '{source.get('__identifier')}'; "
                f"'{target_entity.get('iid')}' is a '{kind}'"
            )
    elif scope == "OnlySpecificEntity":
        wanted = field_def.get("allowedRefsEntityUid")
        if wanted is not None and target_entity.get("defUid") != wanted:
            raise SystemExit(
                f"'{field_def['identifier']}' may only point at entity def uid "
                f"{wanted}; '{target_entity.get('iid')}' is a '{kind}' "
                f"(defUid {target_entity.get('defUid')})"
            )


def resolve_entity_ref(
    project: dict, target, *, field_def: dict | None = None, source: dict | None = None
) -> dict | None:
    """Turn a spec's EntityRef TARGET into LDtk's canonical ref object.

    `target` is the referenced entity's `iid` (or `null` to clear the ref). The
    `layerIid` / `levelIid` / `worldIid` come from wherever that iid is found in
    THIS project, so they describe the file as it is now rather than as it was
    when somebody copied a ref out of an older one.

    `field_def` / `source` are the referring field's definition and the entity
    carrying it; when given, the def's declared `allowedRefs` scope is enforced.
    """
    if target is None or target == "":
        return None
    if isinstance(target, dict):
        raise SystemExit(
            "an EntityRef field takes the target entity's iid, not a prebuilt "
            f"{sorted(target)} object: the layer/level/world iids are read out of "
            "the project so they cannot go stale. Write `mounted_on: "
            "EnemySpawn-6806`."
        )
    target = str(target)
    found: list[tuple[dict, dict, dict]] = []
    for level in project.get("levels", []):
        for layer in level.get("layerInstances", []):
            for entity in layer.get("entityInstances", []):
                if entity.get("iid") == target:
                    found.append((level, layer, entity))
    if not found:
        raise SystemExit(
            f"EntityRef target '{target}' is not an entity in this project. A ref "
            f"to a missing iid loads as an unset ref and spawns the referrer alone."
        )
    if len(found) > 1:
        raise SystemExit(f"EntityRef target '{target}' is not unique in this project")
    level, layer, target_entity = found[0]
    _check_allowed_ref(field_def, source, target_entity)
    return {
        "entityIid": target,
        "layerIid": layer.get("iid"),
        "levelIid": level.get("iid"),
        "worldIid": project.get("iid"),
    }


def apply_field_edit(project: dict, entity: dict, field_name: str, new_value) -> None:
    """Set `entity[fieldInstances][field_name].__value` to `new_value`,
    coercing via the entity def's declared type so booleans /
    enumerations / numerics land in the canonical shape the LDtk editor
    expects. The repair pass (`ambition_ldtk_tools.repair`) keeps editor metadata aligned
    for common types, so we only need to write the parser-facing `__value`. Adds the field instance if
    it isn't already present."""
    ent_def = find_entity_def(project, entity.get("__identifier"))
    field_defs = {f["identifier"]: f for f in ent_def.get("fieldDefs", [])}
    if field_name not in field_defs:
        raise SystemExit(
            f"entity '{entity.get('__identifier')}' has no field '{field_name}'. "
            f"Known fields: {sorted(field_defs)}"
        )
    field_def = field_defs[field_name]
    type_str = field_def.get("__type") or field_def.get("type") or "String"
    if type_str == "EntityRef":
        coerced = resolve_entity_ref(
            project, new_value, field_def=field_def, source=entity
        )
    else:
        coerced = coerce_field_value(type_str, new_value)
    instance_payload = make_field_instance(field_def, coerced)
    for fi in entity.setdefault("fieldInstances", []):
        if fi.get("__identifier") == field_name:
            fi.clear()
            fi.update(instance_payload)
            return
    entity["fieldInstances"].append(instance_payload)


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("spec", type=Path)
    parser.add_argument(
        "--ldtk",
        type=Path,
        default=default_sandbox_ldtk(),
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
        parser.error("choose --in-place or --output <path>")

    spec = load_spec(args.spec)
    if not isinstance(spec, dict) or "level_id" not in spec or "edits" not in spec:
        return _fail("spec must be a mapping with `level_id` and `edits`")

    tx = LdtkTransaction(
        args.ldtk,
        in_place=args.in_place,
        output=args.output,
        backup=args.backup,
    )
    project = tx.project
    level = find_level(project, spec["level_id"])
    layer = find_ambition_layer(level)

    edits = []
    for edit in spec["edits"]:
        target = edit.get("target") or {}
        fields = edit.get("fields") or {}
        if not fields:
            return _fail("edit must include at least one field under `fields`")
        matched = select_entities(layer, target)
        for entity in matched:
            for fname, fvalue in fields.items():
                apply_field_edit(project, entity, fname, fvalue)
            edits.append(
                f"{entity.get('__identifier')} ({entity.get('iid')}): "
                + ", ".join(f"{k}={v!r}" for k, v in fields.items())
            )

    if edits:
        tx.note_changed(edits)
    target_path = tx.finish(
        noop_message="entity set-field: no matching edits were applied",
        write_message="wrote {path}",
    )
    print(f"applied {len(edits)} edit(s):")
    for line in edits:
        print(f"  {line}")
    if target_path is None or args.no_repair:
        return 0
    return run_repair_and_validate(target_path, args.schema)


def _fail(msg: str) -> int:
    print(f"error: {msg}", file=sys.stderr)
    return 2


if __name__ == "__main__":
    raise SystemExit(main())
