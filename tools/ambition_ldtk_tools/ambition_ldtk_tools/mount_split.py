"""`mount split` — expand fused composite EnemySpawns into linked mount+rider pairs.

ADR 0020 authors a mount as **two linked LDtk entities** (the mount action
pre-applied): a rider `EnemySpawn` carrying a `mounted_on` EntityRef to a mount
`EnemySpawn`. This subcommand migrates the legacy fused rows — a single
`EnemySpawn` whose `brain` is `pirate_on_shark` / `pirate_heavy_on_shark` — into
that shape:

  * the original entity BECOMES the rider: its `brain` is rewritten to the
    matching rider archetype (`pirate_shark_rider` / `pirate_heavy_shark_rider`),
    its display name loses the `" on Shark"` suffix, and it gains a `mounted_on`
    EntityRef field;
  * a new mount `EnemySpawn` (`brain: burning_flying_shark`) is created at the
    same position, and the rider's `mounted_on` points at its `iid`.

Neither entity carries an explicit `id` field, so at load time each actor's
`FeatureId` equals its `iid` — which is exactly the target the EntityRef stores,
so `resolve_pending_mount_links` matches the pair by `FeatureId`.

The engine still resolves the pair identically; this only moves the "which two
actors, and that they are linked" fact from a fused archetype row into the world
file, per Jon's ADR-0020 decision.
"""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

from ambition_ldtk_tools.area_authoring import allocate_iid, find_entity_def, make_field_instance
from ambition_ldtk_tools.edit.postprocess import run_repair_and_validate
from ambition_ldtk_tools.ldtk.transaction import LdtkTransaction

REPO_ROOT = Path(__file__).resolve().parents[3]

# Fused composite brain -> the standalone rider archetype that carries its kit.
RIDER_ARCHETYPE = {
    "pirate_on_shark": "pirate_shark_rider",
    "pirate_heavy_on_shark": "pirate_heavy_shark_rider",
}
MOUNT_BRAIN = "burning_flying_shark"
MOUNT_NAME = "Burning Flying Shark"
RIDER_NAME_SUFFIX = " on Shark"
# Fallback rider display name when the fused entity was named after the mount
# (some light composites were authored as "Burning Flying Shark"). Matches the
# old composite `rider_fallback_name`. The rider's NAME drives its sprite bind,
# so it must be a rider name, never the mount's.
RIDER_DEFAULT_NAME = {
    "pirate_shark_rider": "Pirate Raider",
    "pirate_heavy_shark_rider": "Broadside Bess",
}
# The rider's authored spawn box, by rider archetype (matches the archetype
# default_size; the runtime derives real sizing from the archetype + the weld,
# so this is just the authored AABB).
RIDER_BOX = {
    "pirate_shark_rider": (44, 78),
    "pirate_heavy_shark_rider": (72, 110),
}


def _field(entity: dict, name: str) -> dict | None:
    for fi in entity.get("fieldInstances", []):
        if fi.get("__identifier") == name:
            return fi
    return None


def ensure_mounted_on_fielddef(
    project: dict,
    entity_identifier: str = "EnemySpawn",
    allowed_refs: str = "OnlySame",
) -> dict:
    """Ensure an entity def carries a `mounted_on` EntityRef field.

    ADR 0020 authors mount links as a rider entity's `mounted_on` EntityRef →
    a mount entity. The EnemySpawn rider (pirate/shark) references another
    EnemySpawn, so `allowed_refs="OnlySame"`; the BossSpawn rider (GNU-ton the
    scholar) references a `giant_gnu` EnemySpawn mount — a CROSS-type ref — so
    the caller passes `allowed_refs="Any"`. Idempotent: returns the existing
    field def untouched if it is already present.
    """
    es_def = find_entity_def(project, entity_identifier)
    for f in es_def.get("fieldDefs", []):
        if f["identifier"] == "mounted_on":
            return f
    _, uid = allocate_iid(project, entity_identifier)  # bumps nextUid; reuse the int
    field_def = {
        "identifier": "mounted_on",
        "doc": "ADR 0020: the mount EnemySpawn this rider is mounted on (the "
        "mount action pre-applied). Resolved into a RidingOn/MountSlot link.",
        "__type": "EntityRef",
        "uid": uid,
        "type": "F_EntityRef",
        "isArray": False,
        "canBeNull": True,
        "arrayMinLength": None,
        "arrayMaxLength": None,
        "editorDisplayMode": "RefLinkBetweenCenters",
        "editorDisplayScale": 1,
        "editorDisplayPos": "Above",
        "editorLinkStyle": "CurvedArrow",
        "editorDisplayColor": None,
        "editorAlwaysShow": False,
        "editorShowInWorld": True,
        "editorCutLongValues": True,
        "editorTextSuffix": None,
        "editorTextPrefix": None,
        "useForSmartColor": False,
        "exportToToc": False,
        "searchable": False,
        "min": None,
        "max": None,
        "regex": None,
        "acceptFileTypes": None,
        "defaultOverride": None,
        "textLanguageMode": None,
        # EntityRef targets: `allowed_refs` scopes what the rider may point at
        # ("OnlySame" for EnemySpawn→EnemySpawn, "Any" for BossSpawn→EnemySpawn).
        "symmetricalRef": False,
        "autoChainRef": True,
        "allowOutOfLevelRef": False,
        "allowedRefs": allowed_refs,
        "allowedRefsEntityUid": None,
        "allowedRefTags": [],
        "tilesetUid": None,
    }
    es_def.setdefault("fieldDefs", []).append(field_def)
    return field_def


def _set_string_field(field_inst: dict, value: str) -> None:
    field_inst["__value"] = value
    field_inst["realEditorValues"] = [{"id": "V_String", "params": [value]}]


def _build_mount_entity(project: dict, rider: dict, layer: dict) -> dict:
    """Create the mount EnemySpawn beside the rider, at the same position."""
    es_def = find_entity_def(project, "EnemySpawn")
    iid, _ = allocate_iid(project, "EnemySpawn")
    px = list(rider["px"])
    grid = list(rider.get("__grid", [px[0] // 16, px[1] // 16]))
    mount = {
        "__identifier": "EnemySpawn",
        "__grid": grid,
        "__pivot": rider.get("__pivot", [0, 0]),
        "__tags": [],
        "__tile": None,
        "__smartColor": es_def.get("color", "#FFFFFF"),
        "__worldX": rider.get("__worldX", px[0]),
        "__worldY": rider.get("__worldY", px[1]),
        "iid": iid,
        "width": rider.get("width", 108),
        "height": rider.get("height", 96),
        "defUid": es_def["uid"],
        "px": px,
        "fieldInstances": [],
    }
    # Mount fields: name + brain (no id field → FeatureId == iid).
    for fname, value in (("name", MOUNT_NAME), ("brain", MOUNT_BRAIN)):
        fdef = next(f for f in es_def["fieldDefs"] if f["identifier"] == fname)
        inst = make_field_instance(fdef, value)
        inst["realEditorValues"] = [{"id": "V_String", "params": [value]}]
        mount["fieldInstances"].append(inst)
    return mount


def split_composites(project: dict) -> list[str]:
    """Split every fused composite EnemySpawn into a linked mount+rider pair.

    Returns a list of human-readable change descriptions.
    """
    changes: list[str] = []
    ref_field_def = ensure_mounted_on_fielddef(project)
    world_iid = project.get("iid")
    for level in project.get("levels", []):
        level_iid = level.get("iid")
        for layer in level.get("layerInstances", []):
            layer_iid = layer.get("iid")
            new_mounts: list[dict] = []
            for rider in layer.get("entityInstances", []):
                if rider.get("__identifier") != "EnemySpawn":
                    continue
                brain_fi = _field(rider, "brain")
                if brain_fi is None:
                    continue
                composite = brain_fi.get("__value")
                if composite not in RIDER_ARCHETYPE:
                    continue
                rider_arch = RIDER_ARCHETYPE[composite]

                # 1. Create the mount entity at the rider's position.
                mount = _build_mount_entity(project, rider, layer)

                # 2. Rewrite the rider's brain to the rider archetype.
                _set_string_field(brain_fi, rider_arch)

                # 3. Give the rider a proper rider name (drives its sprite bind).
                # The fused entity's name was sometimes the rider's-with-suffix
                # ("Iron Mary on Shark" → "Iron Mary") and sometimes the mount's
                # ("Burning Flying Shark"); in the latter case fall back to the
                # archetype's default rider name so the rider never keeps the
                # mount's name (which would bind the shark sprite).
                name_fi = _field(rider, "name")
                if name_fi and isinstance(name_fi.get("__value"), str):
                    original = name_fi["__value"]
                    if original.endswith(RIDER_NAME_SUFFIX):
                        rider_name = original[: -len(RIDER_NAME_SUFFIX)]
                    elif original == MOUNT_NAME:
                        rider_name = RIDER_DEFAULT_NAME.get(rider_arch, original)
                    else:
                        rider_name = original
                    _set_string_field(name_fi, rider_name)

                # 4. Resize the rider's authored box to its archetype footprint.
                box = RIDER_BOX.get(rider_arch)
                if box:
                    rider["width"], rider["height"] = box

                # 5. Add the rider's mounted_on EntityRef → the mount's iid.
                ref_value = {
                    "entityIid": mount["iid"],
                    "layerIid": layer_iid,
                    "levelIid": level_iid,
                    "worldIid": world_iid,
                }
                existing = _field(rider, "mounted_on")
                if existing is not None:
                    existing["__value"] = ref_value
                else:
                    rider["fieldInstances"].append(
                        make_field_instance(ref_field_def, ref_value)
                    )

                new_mounts.append(mount)
                changes.append(
                    f"{level.get('identifier')}: {composite} '{rider.get('iid')}' "
                    f"→ rider {rider_arch} + mount {mount['iid']}"
                )
            layer.setdefault("entityInstances", []).extend(new_mounts)
    return changes


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("ldtk", type=Path, help="the .ldtk world file to migrate")
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

    tx = LdtkTransaction(
        args.ldtk,
        in_place=args.in_place,
        output=args.output,
        backup=args.backup,
    )
    changes = split_composites(tx.project)
    if changes:
        tx.note_changed(changes)
    target_path = tx.finish(
        noop_message="mount split: no fused composite EnemySpawns found",
        write_message="wrote {path}",
    )
    print(f"split {len(changes)} composite spawn(s):")
    for line in changes:
        print(f"  {line}")
    if target_path is None or args.no_repair:
        return 0
    return run_repair_and_validate(target_path, args.schema)


if __name__ == "__main__":
    raise SystemExit(main())
