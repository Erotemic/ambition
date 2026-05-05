#!/usr/bin/env python3
"""Add a single entity instance to an existing LDtk level.

Companion to `author_ldtk_area.py` (which authors whole levels) and
`register_ldtk_entity_def.py` (which registers new entity types). Use
this when you need to surgically attach one entity — most often a
`LoadingZone` connecting an existing level to a freshly-authored
area.

The tool refuses to run when:
- the level does not exist,
- the entity identifier is not registered in `defs.entities`,
- a field referenced in the spec is not declared on the entity def.

Spec format (YAML or JSON):

    level_id: central_hub_basement
    entities:
      - type: LoadingZone
        px: [1820, 800]      # level-local pixel coords
        size: [60, 132]
        fields:
          id: lab_door
          name: lab_door
          activation: walk
          target_room: mob_lab
          target_zone: lab_entry
          bidirectional: true
"""
from __future__ import annotations

import argparse
import json
import shutil
import subprocess
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
TOOLS_DIR = REPO_ROOT / "tools"
sys.path.insert(0, str(TOOLS_DIR))

# Reuse the bigger authoring tool's helpers so we never drift on the
# entity-instance shape (defUid sync, __smartColor, fieldInstance
# coercion, iid allocation).
from author_ldtk_area import (  # noqa: E402
    build_entity_instance,
    coerce_field_value,
    find_entity_def,
    load_project,
    write_project,
)

REPAIR = TOOLS_DIR / "repair_ambition_ldtk.py"
VALIDATE = TOOLS_DIR / "validate_ambition_ldtk.py"


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
    raise SystemExit(
        f"level '{level['identifier']}' has no Ambition entity layer"
    )


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("spec", type=Path)
    parser.add_argument(
        "--ldtk",
        type=Path,
        default=REPO_ROOT
        / "crates"
        / "ambition_sandbox"
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
        default=REPO_ROOT / "tools" / "schemas" / "ldtk" / "JSON_SCHEMA.json",
    )
    args = parser.parse_args(argv)
    if not args.in_place and args.output is None:
        parser.error("choose --in-place or --output <path>")

    spec = load_spec(args.spec)
    if not isinstance(spec, dict) or "level_id" not in spec or "entities" not in spec:
        return _fail("spec must be a mapping with `level_id` and `entities`")

    project = load_project(args.ldtk)
    level = find_level(project, spec["level_id"])
    layer = find_ambition_layer(level)
    grid_size = int(project.get("defaultGridSize", 16))

    added = []
    for ent_spec in spec["entities"]:
        # Validate that referenced fields exist on the entity def so we
        # fail loudly rather than silently emitting an unconsumable field.
        ent_def = find_entity_def(project, ent_spec["type"])
        valid_fields = {f["identifier"] for f in ent_def.get("fieldDefs", [])}
        for fname in (ent_spec.get("fields") or {}):
            if fname not in valid_fields:
                return _fail(
                    f"entity '{ent_spec['type']}' has no field '{fname}' "
                    f"(known: {sorted(valid_fields)})"
                )
        instance = build_entity_instance(project, ent_spec, grid_size)
        layer.setdefault("entityInstances", []).append(instance)
        added.append(f"{ent_spec['type']} ({instance['iid']})")

    target = args.output or args.ldtk
    if args.in_place and args.backup:
        backup = args.ldtk.with_suffix(args.ldtk.suffix + ".bak")
        shutil.copy2(args.ldtk, backup)
        print(f"wrote backup: {backup}")
    write_project(target, project)
    print(f"added {len(added)} entity instance(s) to '{spec['level_id']}': {', '.join(added)}")
    if args.no_repair:
        return 0

    cmd = [sys.executable, str(REPAIR), str(target), "--in-place"]
    print("$ " + " ".join(cmd))
    if subprocess.run(cmd).returncode != 0:
        return 1
    cmd = [sys.executable, str(VALIDATE), str(target)]
    if args.schema and args.schema.exists():
        cmd.extend(["--schema", str(args.schema), "--require-schema"])
    print("$ " + " ".join(cmd))
    return subprocess.run(cmd).returncode


def _fail(msg: str) -> int:
    print(f"error: {msg}", file=sys.stderr)
    return 2


# Keep the import-only side-effect-free so the smoke test for the
# parent author_ldtk_area module can also import this without running
# the CLI parser.
if __name__ == "__main__":
    raise SystemExit(main())
_ = coerce_field_value  # silence unused import warning if linters check
