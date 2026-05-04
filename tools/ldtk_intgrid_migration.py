#!/usr/bin/env python3
"""One-shot migration: introduce an IntGrid `Collision` layer to sandbox.ldtk
and rewrite static collision entities (Solid / OneWayPlatform / BlinkWall)
in the central_hub_main level into IntGrid cells.

Per `docs/path_forward.md` step E. Idempotent — running twice is a no-op
(detects the existing Collision layer and skips). LDtk 1.5.3 round-trip
must still work after this script runs (verified by repair script).

Layer values:
    1 = Solid (matches existing Solid entity collision)
    2 = OneWayUp (matches OneWayPlatform)
    3 = BlinkSoft (matches BlinkWall { tier: Soft })
    4 = BlinkHard (matches BlinkWall { tier: Hard })

Cells are 16x16 pixels (matches the existing AMBITION_LAYER grid).
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

LDTK_PATH = Path("crates/ambition_sandbox/assets/ambition/worlds/sandbox.ldtk")
GRID = 16

VALUE_SOLID = 1
VALUE_ONE_WAY = 2
VALUE_BLINK_SOFT = 3
VALUE_BLINK_HARD = 4

INT_GRID_VALUES = [
    {"value": VALUE_SOLID, "identifier": "Solid", "color": "#6B7280", "tile": None, "groupUid": 0},
    {"value": VALUE_ONE_WAY, "identifier": "OneWayUp", "color": "#92C9F5", "tile": None, "groupUid": 0},
    {"value": VALUE_BLINK_SOFT, "identifier": "BlinkSoft", "color": "#A78BFA", "tile": None, "groupUid": 0},
    {"value": VALUE_BLINK_HARD, "identifier": "BlinkHard", "color": "#7C3AED", "tile": None, "groupUid": 0},
]

# Levels to migrate. Start with the central hub; extend as we validate the path.
LEVELS_TO_MIGRATE = ["central_hub_main"]

# Entities that lower into IntGrid cells. The mapping captures the value the
# cell should take, plus how to read the wall tier for BlinkWall.
def entity_to_value(entity: dict) -> int | None:
    ident = entity["__identifier"]
    if ident == "Solid":
        return VALUE_SOLID
    if ident == "OneWayPlatform":
        return VALUE_ONE_WAY
    if ident == "BlinkWall":
        tier = _field(entity, "tier") or "Soft"
        return VALUE_BLINK_HARD if tier == "Hard" else VALUE_BLINK_SOFT
    return None


def _field(entity: dict, name: str):
    for f in entity.get("fieldInstances", []):
        if f["__identifier"] == name:
            return f.get("__value")
    return None


def collision_layer_def(layers: list[dict]) -> dict | None:
    return next((l for l in layers if l["identifier"] == "Collision"), None)


def next_uid(doc: dict) -> int:
    """Compute the next free UID. LDtk uses monotonic uids across defs/levels/layers."""
    seen = {0}
    def walk(node):
        if isinstance(node, dict):
            for k, v in node.items():
                if k.lower().endswith("uid") and isinstance(v, int):
                    seen.add(v)
                walk(v)
        elif isinstance(node, list):
            for item in node:
                walk(item)
    walk(doc)
    return max(seen) + 1


def make_collision_layer_def(uid: int) -> dict:
    """Create the IntGrid layer definition. Uses defaults that LDtk 1.5.3
    emits for an unconfigured IntGrid layer."""
    return {
        "__type": "IntGrid",
        "identifier": "Collision",
        "type": "IntGrid",
        "uid": uid,
        "doc": "Static collision: 1=Solid, 2=OneWayUp, 3=BlinkSoft, 4=BlinkHard.",
        "uiColor": None,
        "gridSize": GRID,
        "guideGridWid": 0,
        "guideGridHei": 0,
        "displayOpacity": 1,
        "inactiveOpacity": 0.6,
        "hideInList": False,
        "hideFieldsWhenInactive": True,
        "canSelectWhenInactive": True,
        "renderInWorldView": True,
        "pxOffsetX": 0,
        "pxOffsetY": 0,
        "parallaxFactorX": 0,
        "parallaxFactorY": 0,
        "parallaxScaling": True,
        "requiredTags": [],
        "excludedTags": [],
        "autoTilesKilledByOtherLayerUid": None,
        "uiFilterTags": [],
        "useAsyncRender": False,
        "intGridValues": INT_GRID_VALUES,
        "intGridValuesGroups": [],
        "autoRuleGroups": [],
        "autoSourceLayerDefUid": None,
        "tilesetDefUid": None,
        "tilePivotX": 0,
        "tilePivotY": 0,
        "biomeFieldUid": None,
    }


def cells_for_size(px: int) -> int:
    """LDtk computes layer cWid/cHei as `ceil(level_px / gridSize)` and the
    intGridCsv array length is `cWid * cHei`. Earlier versions of this
    script used floor division, which left the array one cell short on
    any axis whose level dimension wasn't a multiple of `GRID`. LDtk
    silently re-strode the array on load (stride 119 vs 118 for a 1900px
    level) and the cells smeared into a one-cell-per-row staircase.
    Always use ceil here."""
    return (px + GRID - 1) // GRID


def make_collision_layer_instance(level: dict, layer_def_uid: int, uid: int, intgrid_csv: list[int]) -> dict:
    return {
        "__identifier": "Collision",
        "__type": "IntGrid",
        "__cWid": cells_for_size(level["pxWid"]),
        "__cHei": cells_for_size(level["pxHei"]),
        "__gridSize": GRID,
        "__opacity": 1,
        "__pxTotalOffsetX": 0,
        "__pxTotalOffsetY": 0,
        "__tilesetDefUid": None,
        "__tilesetRelPath": None,
        "iid": f"Collision-{uid}",
        "levelId": level["uid"],
        "layerDefUid": layer_def_uid,
        "pxOffsetX": 0,
        "pxOffsetY": 0,
        "visible": True,
        "optionalRules": [],
        "intGridCsv": intgrid_csv,
        "autoLayerTiles": [],
        "seed": 0,
        "overrideTilesetUid": None,
        "gridTiles": [],
        "entityInstances": [],
    }


def fill_cells(intgrid: list[int], cw: int, px: int, py: int, w: int, h: int, value: int) -> int:
    """Paint `value` into every cell that intersects the rect [px,py,px+w,py+h).
    Returns count of cells painted."""
    cx0 = px // GRID
    cy0 = py // GRID
    cx1 = (px + w + GRID - 1) // GRID
    cy1 = (py + h + GRID - 1) // GRID
    count = 0
    for cy in range(cy0, cy1):
        for cx in range(cx0, cx1):
            idx = cy * cw + cx
            if 0 <= idx < len(intgrid):
                intgrid[idx] = value
                count += 1
    return count


def migrate_level(doc: dict, level: dict, layer_def_uid: int, instance_uid: int) -> tuple[int, int]:
    """Returns (cells_painted, entities_removed)."""
    cw = cells_for_size(level["pxWid"])
    ch = cells_for_size(level["pxHei"])
    intgrid = [0] * (cw * ch)

    # Pull the Ambition entity layer + walk its entityInstances.
    ambition_layer = next(l for l in level["layerInstances"] if l["__identifier"] == "Ambition")
    survivors = []
    cells_painted = 0
    entities_removed = 0
    for ent in ambition_layer["entityInstances"]:
        value = entity_to_value(ent)
        if value is None:
            survivors.append(ent)
            continue
        px, py = ent["px"]
        w, h = ent["width"], ent["height"]
        cells_painted += fill_cells(intgrid, cw, px, py, w, h, value)
        entities_removed += 1
    ambition_layer["entityInstances"] = survivors

    # Insert the Collision layer instance. LDtk lists layer instances in the
    # order their defs appear in defs.layers; we'll fix the order at the end.
    coll_inst = make_collision_layer_instance(level, layer_def_uid, instance_uid, intgrid)
    level["layerInstances"].append(coll_inst)
    return cells_painted, entities_removed


def main() -> int:
    if not LDTK_PATH.exists():
        print(f"missing: {LDTK_PATH}", file=sys.stderr)
        return 1
    with LDTK_PATH.open() as f:
        doc = json.load(f)

    layers = doc["defs"]["layers"]
    existing = collision_layer_def(layers)
    if existing is not None:
        print(f"Collision layer already exists (uid={existing['uid']}); nothing to do.")
        return 0

    # 1) Add the layer def. New defs go in front of Entities so the IntGrid
    # renders behind entities in the editor / runtime. LDtk renders earlier
    # entries on top, so prepend.
    layer_def_uid = next_uid(doc)
    coll_def = make_collision_layer_def(layer_def_uid)
    layers.insert(0, coll_def)

    # 2) Walk levels: every level gets a Collision layerInstance (empty by
    # default); the targeted levels also have their static-collision
    # entities lowered into cells.
    total_cells = 0
    total_removed = 0
    levels_touched = 0
    instance_uid_counter = layer_def_uid + 1
    for level in doc["levels"]:
        # All levels need an instance for the new layer def, otherwise LDtk
        # complains about missing layer data.
        if level["identifier"] in LEVELS_TO_MIGRATE:
            cells, removed = migrate_level(doc, level, layer_def_uid, instance_uid_counter)
            total_cells += cells
            total_removed += removed
            levels_touched += 1
        else:
            cw = cells_for_size(level["pxWid"])
            ch = cells_for_size(level["pxHei"])
            empty = [0] * (cw * ch)
            level["layerInstances"].append(
                make_collision_layer_instance(level, layer_def_uid, instance_uid_counter, empty)
            )
        # Reorder so Collision precedes Ambition (matches the def order).
        level["layerInstances"].sort(
            key=lambda li: 0 if li["__identifier"] == "Collision" else 1
        )
        instance_uid_counter += 1

    # Custom serializer: collapse `intGridCsv` (and similarly large numeric
    # arrays) onto a single line. Default `json.dump(indent=2)` expands every
    # int onto its own line, which inflates a typical 50000-cell layer to
    # ~100k file lines and turns a 200kB asset into 1.7MB. LDtk 1.5.3's GUI
    # emits these arrays compactly; matching that keeps diffs reviewable.
    text = _dump_with_compact_int_arrays(doc)
    LDTK_PATH.write_text(text + "\n")

    print(
        f"Added Collision IntGrid layer (uid={layer_def_uid}). "
        f"Migrated {levels_touched} level(s): {total_cells} cells painted, "
        f"{total_removed} entities removed."
    )
    return 0


def _dump_with_compact_int_arrays(doc: dict) -> str:
    """Pretty-print like `json.dump(indent=2)`, but inline arrays whose
    elements are all simple numbers / bools / null. That collapses
    `intGridCsv` and similar bulk-numeric arrays onto a single line
    while leaving structured arrays (`entityInstances`, `layerInstances`)
    expanded for readability."""

    def is_simple_array(value) -> bool:
        return isinstance(value, list) and all(
            isinstance(v, (int, float, bool)) or v is None for v in value
        )

    def encode(node, indent: int) -> str:
        pad = " " * indent
        if is_simple_array(node):
            return "[" + ", ".join(json.dumps(v) for v in node) + "]"
        if isinstance(node, list):
            if not node:
                return "[]"
            inner = ",\n".join(pad + "  " + encode(v, indent + 2) for v in node)
            return "[\n" + inner + "\n" + pad + "]"
        if isinstance(node, dict):
            if not node:
                return "{}"
            parts = []
            for k, v in node.items():
                parts.append(f'{pad}  {json.dumps(k)}: {encode(v, indent + 2)}')
            return "{\n" + ",\n".join(parts) + "\n" + pad + "}"
        return json.dumps(node)

    return encode(doc, 0)


if __name__ == "__main__":
    sys.exit(main())
