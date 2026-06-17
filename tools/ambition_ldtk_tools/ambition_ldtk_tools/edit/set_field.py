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

The tool errors out if:
  * the level doesn't exist;
  * no entity matches the target selector (or more than one does);
  * a field name isn't declared on the entity def (loud catch — silent
    write-through would leave the LDtk editor refusing to load the
    field next time).
"""

from __future__ import annotations

import argparse
import json
import shutil
import subprocess
import sys
from pathlib import Path

# tools/ambition_ldtk_tools/ambition_ldtk_tools/edit/set_field.py -> repo root.
REPO_ROOT = Path(__file__).resolve().parents[4]

from ambition_ldtk_tools.area_authoring import (  # noqa: E402
    coerce_field_value,
    find_entity_def,
    load_project,
    make_field_instance,
    write_project,
)


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
        default=REPO_ROOT
        / "crates"
        / "ambition_gameplay_core"
        / "assets"
        / "ambition"
        / "worlds"
        / "sandbox.ldtk",
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

    project = load_project(args.ldtk)
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

    target_path = args.output or args.ldtk
    if args.in_place and args.backup:
        backup = args.ldtk.with_suffix(args.ldtk.suffix + ".bak")
        shutil.copy2(args.ldtk, backup)
        print(f"wrote backup: {backup}")
    write_project(target_path, project)
    print(f"applied {len(edits)} edit(s):")
    for line in edits:
        print(f"  {line}")
    if args.no_repair:
        return 0

    cmd = [
        sys.executable,
        "-m",
        "ambition_ldtk_tools.repair",
        str(target_path),
        "--in-place",
    ]
    print("$ " + " ".join(cmd))
    if subprocess.run(cmd).returncode != 0:
        return 1
    cmd = [sys.executable, "-m", "ambition_ldtk_tools.validate", str(target_path)]
    if args.schema and args.schema.exists():
        cmd.extend(["--schema", str(args.schema), "--require-schema"])
    print("$ " + " ".join(cmd))
    return subprocess.run(cmd).returncode


def _fail(msg: str) -> int:
    print(f"error: {msg}", file=sys.stderr)
    return 2


if __name__ == "__main__":
    raise SystemExit(main())
