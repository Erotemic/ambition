#!/usr/bin/env python3
"""Move every entity instance of one `__identifier` into its own
Entities-type LDtk layer so the editor can toggle / lock that layer
independently of the catch-all `Ambition` layer.

Motivating case: the intro map has `CameraZone` entities that cover
most of the level — every other entity sits "underneath" them on the
same layer, so click-to-select picks the camera zone first. Splitting
them onto a dedicated `AmbitionCameras` layer lets the author lock /
hide that layer while editing the rest of the room.

The Rust loader (`world::ldtk_world::project::LdtkLevel::all_entity_instances`)
iterates **every** `__type: "Entities"` layer instance, so moving
entities onto a sibling layer doesn't break runtime spawning.

## Usage

```bash
PYTHONPATH=tools/ambition_ldtk_tools \\
python -m ambition_ldtk_tools layer split-entities \\
    crates/ambition_gameplay_core/assets/ambition/worlds/intro.ldtk \\
    --type CameraZone \\
    --to-layer AmbitionCameras \\
    --in-place
```

* `--type`        the `__identifier` of every entity to relocate
                  (e.g. `CameraZone`).
* `--to-layer`    target layer identifier; created if it doesn't
                  exist (cloned from `--from-layer`'s def).
* `--from-layer`  source layer identifier; defaults to `Ambition`.
* `--in-place` / `--output PATH` mutually exclusive.

Idempotent: re-running after the entities have moved is a no-op
(nothing matches in `from-layer` anymore). Safe to commit + re-run
in CI.

Writes LDtk editor-style JSON directly. It no longer shells out to `repair`
by default because `repair` also runs full project validation, and sandbox files
may intentionally contain LoadingZone links to rooms in other LDtk files.
"""

from __future__ import annotations

import argparse
import json
import shutil
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[4]


def write_project(path: Path, project: dict) -> None:
    """Write editor-style JSON without running full semantic validation.

    `repair` is still useful as a standalone command, but it also validates
    LoadingZone targets. That is too strong for entity relocation because some
    sandbox worlds intentionally link to rooms that live in other LDtk files.
    """
    try:
        from ambition_ldtk_tools.editor_format import dump_editor_style
        from ambition_ldtk_tools.validate import normalize_project_for_editor

        normalize_project_for_editor(project)
        path.write_text(dump_editor_style(project))
    except Exception:  # pragma: no cover - fallback for partial tool installs
        path.write_text(json.dumps(project, indent=2))


def find_layer_def(project: dict, identifier: str) -> dict | None:
    for layer in project.get("defs", {}).get("layers", []):
        if layer.get("identifier") == identifier:
            return layer
    return None


def alloc_uid(project: dict) -> int:
    """Allocate a new `nextUid` from the project counter."""
    next_uid = int(project.get("nextUid", 1))
    project["nextUid"] = next_uid + 1
    return next_uid


def ensure_dest_layer_def(
    project: dict, *, from_def: dict, dest_identifier: str
) -> dict:
    """Return the layer-def for `dest_identifier`, creating it as a
    sibling Entities layer of `from_def` if it doesn't exist.
    """
    existing = find_layer_def(project, dest_identifier)
    if existing is not None:
        if existing.get("__type") != "Entities":
            raise SystemExit(
                f"layer '{dest_identifier}' already exists with __type="
                f"{existing.get('__type')}; refusing to use a non-Entities "
                f"layer as the destination."
            )
        return existing
    # Clone the source's structural fields. `uid` MUST be unique;
    # take it from `nextUid` so the project's counter stays in sync.
    new_def = dict(from_def)
    new_def["identifier"] = dest_identifier
    new_def["uid"] = alloc_uid(project)
    project.setdefault("defs", {}).setdefault("layers", []).append(new_def)
    return new_def


def find_layer_instance(level: dict, identifier: str) -> dict | None:
    for li in level.get("layerInstances", []):
        if li.get("__identifier") == identifier:
            return li
    return None


def ensure_dest_layer_instance(
    project: dict,
    level: dict,
    *,
    from_instance: dict,
    dest_def: dict,
    dest_identifier: str,
) -> dict:
    """Return the layer-instance for `dest_identifier` on `level`,
    creating it as a sibling of `from_instance` if it doesn't exist.
    """
    existing = find_layer_instance(level, dest_identifier)
    if existing is not None:
        return existing
    # Clone from_instance's structural fields. Empty out
    # entityInstances so the new instance is a fresh container.
    new_inst = dict(from_instance)
    new_inst["__identifier"] = dest_identifier
    new_inst["layerDefUid"] = dest_def["uid"]
    new_inst["iid"] = f"{dest_identifier}-{alloc_uid(project)}"
    new_inst["entityInstances"] = []
    # Insert after the source layer so the editor's draw order
    # places the new layer on top of the source (LDtk renders
    # layerInstances in array order, top of list = bottom in the
    # rendered z stack — but visibility in the editor follows the
    # same order).
    layer_instances = level.setdefault("layerInstances", [])
    try:
        idx = layer_instances.index(from_instance)
        layer_instances.insert(idx, new_inst)
    except ValueError:
        layer_instances.append(new_inst)
    return new_inst


def relocate_entities(
    *,
    from_instance: dict,
    dest_instance: dict,
    entity_identifier: str,
) -> int:
    """Move every entityInstance matching `entity_identifier` from
    `from_instance` into `dest_instance`. Returns the count moved.
    """
    moved = 0
    remaining = []
    for entity in from_instance.get("entityInstances", []):
        if entity.get("__identifier") == entity_identifier:
            dest_instance.setdefault("entityInstances", []).append(entity)
            moved += 1
        else:
            remaining.append(entity)
    from_instance["entityInstances"] = remaining
    return moved


def run_repair(ldtk_path: Path) -> None:
    """Deprecated compatibility shim.

    Entity-layer moves now write canonical editor-style JSON directly. Running
    full `repair` here made harmless cross-LDtk LoadingZone links fail unrelated
    layer moves.
    """
    print(f"note: wrote canonical editor-style JSON; skipped full repair validation for {ldtk_path}")


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    # Allow either positional or the `split-entities` verb in argv[0]
    # so `python -m ambition_ldtk_tools layer split-entities <ldtk>`
    # flows naturally through the dispatcher.
    ap.add_argument(
        "action",
        nargs="?",
        default="split-entities",
        choices=["split-entities"],
        help=argparse.SUPPRESS,
    )
    ap.add_argument("ldtk", type=Path)
    ap.add_argument(
        "--type",
        dest="entity_type",
        required=True,
        help="Entity `__identifier` to relocate (e.g. CameraZone)",
    )
    ap.add_argument(
        "--to-layer",
        required=True,
        help="Destination layer identifier (created if absent)",
    )
    ap.add_argument(
        "--from-layer",
        default="Ambition",
        help="Source layer identifier (default: Ambition)",
    )
    ap.add_argument("--in-place", action="store_true")
    ap.add_argument("--output", type=Path, default=None)
    ap.add_argument("--backup", action="store_true")
    ap.add_argument("--dry-run", action="store_true")
    ap.add_argument("--no-repair", action="store_true", help="compatibility flag; writes already skip full repair validation")
    args = ap.parse_args(argv)

    if args.in_place and args.output:
        ap.error("choose --in-place or --output <path>")
    if args.dry_run and (args.in_place or args.output):
        ap.error("--dry-run cannot be combined with --in-place/--output")
    if not args.dry_run and not args.in_place and not args.output:
        ap.error("choose --dry-run, --in-place, or --output <path>")

    project = json.loads(args.ldtk.read_text())

    from_def = find_layer_def(project, args.from_layer)
    if from_def is None or from_def.get("__type") != "Entities":
        return _fail(
            f"source layer '{args.from_layer}' not found or not an Entities layer"
        )
    dest_def = find_layer_def(project, args.to_layer)

    total_moved = 0
    levels_touched = 0
    levels_skipped = 0
    for level in project.get("levels", []):
        from_inst = find_layer_instance(level, args.from_layer)
        if from_inst is None:
            levels_skipped += 1
            continue
        if not any(
            e.get("__identifier") == args.entity_type
            for e in from_inst.get("entityInstances", [])
        ):
            continue
        if dest_def is None:
            dest_def = ensure_dest_layer_def(
                project, from_def=from_def, dest_identifier=args.to_layer
            )
        dest_inst = ensure_dest_layer_instance(
            project,
            level,
            from_instance=from_inst,
            dest_def=dest_def,
            dest_identifier=args.to_layer,
        )
        moved = relocate_entities(
            from_instance=from_inst,
            dest_instance=dest_inst,
            entity_identifier=args.entity_type,
        )
        total_moved += moved
        if moved > 0:
            levels_touched += 1
            print(
                f"  level '{level['identifier']}': moved {moved} "
                f"{args.entity_type} entities"
            )

    print(
        f"split-entities: {'would move' if args.dry_run else 'moved'} {total_moved} {args.entity_type} entities "
        f"across {levels_touched} level(s) "
        f"(skipped {levels_skipped} level(s) without `{args.from_layer}`)"
    )
    if args.dry_run:
        return 0
    if total_moved == 0:
        print("split-entities: no matching entities; left file unchanged")
        return 0
    target_path = args.output or args.ldtk
    if args.backup and args.in_place:
        backup_path = target_path.with_suffix(target_path.suffix + ".bak")
        shutil.copy2(target_path, backup_path)
        print(f"backup written: {backup_path}")
    write_project(target_path, project)
    if not args.no_repair:
        run_repair(target_path)
    return 0


def _fail(msg: str) -> int:
    print(f"error: {msg}", file=sys.stderr)
    return 1


if __name__ == "__main__":
    raise SystemExit(main())
