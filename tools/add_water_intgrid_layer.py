#!/usr/bin/env python3
"""Add a `Water` IntGrid layer to the Ambition LDtk project.

Values:
  1 = ClearWater  (#3DB8E0)
  2 = MurkyWater  (#264A3A)

Why a separate layer (not extra values on `Collision`):
- Single-cell IntGrid layers can only carry one value per cell. Water
  must coexist with collision (e.g. a one-way bottom + water above) so
  it lives on its own layer.
- Layer ordering also lets us draw the water tint in front of the
  collision tile so the floor stays readable.

The script:
  - Inserts the `Water` layer def (uid allocated from `nextUid`).
  - Adds an empty `Water` `layerInstances` entry to every existing
    level so the schema stays consistent and the LDtk editor opens
    cleanly.
"""
from __future__ import annotations

import json
from pathlib import Path

LDTK_PATH = Path(__file__).resolve().parent.parent / "crates/ambition_sandbox/assets/ambition/worlds/sandbox.ldtk"
LAYER_IDENT = "Water"


def main() -> None:
    data = json.loads(LDTK_PATH.read_text())

    # --- Skip if already present.
    layers = data["defs"]["layers"]
    if any(layer["identifier"] == LAYER_IDENT for layer in layers):
        print(f"layer '{LAYER_IDENT}' already exists; nothing to do")
        return

    next_uid = int(data.get("nextUid", 1))
    layer_uid = next_uid
    data["nextUid"] = next_uid + 1

    layer_def = {
        "__type": "IntGrid",
        "identifier": LAYER_IDENT,
        "type": "IntGrid",
        "uid": layer_uid,
        "doc": "Water material: 1=ClearWater (transparent pool), 2=MurkyWater (opaque pool).",
        "uiColor": None,
        "gridSize": 16,
        "guideGridWid": 0,
        "guideGridHei": 0,
        "displayOpacity": 0.6,
        "inactiveOpacity": 0.4,
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
        "intGridValues": [
            {
                "value": 1,
                "identifier": "ClearWater",
                "color": "#3DB8E0",
                "tile": None,
                "groupUid": 0,
            },
            {
                "value": 2,
                "identifier": "MurkyWater",
                "color": "#264A3A",
                "tile": None,
                "groupUid": 0,
            },
        ],
        "intGridValuesGroups": [],
        "autoRuleGroups": [],
        "autoSourceLayerDefUid": None,
        "tilesetDefUid": None,
        "tilePivotX": 0,
        "tilePivotY": 0,
        "biomeFieldUid": None,
    }
    layers.append(layer_def)

    # --- Add an empty Water layer instance to every level. Mirror the
    # Collision layer's IntGrid shape (cWid/cHei from the level, all
    # zeros). Allocate a fresh layer-instance iid per level.
    levels_updated = 0
    for level in data["levels"]:
        instances = level["layerInstances"]
        if any(inst["__identifier"] == LAYER_IDENT for inst in instances):
            continue
        # Pick cWid/cHei from the existing Collision layer instance so
        # the grid sizes stay aligned even if the project default
        # changes.
        collision = next((i for i in instances if i["__identifier"] == "Collision"), None)
        if collision is None:
            raise SystemExit(f"level {level['identifier']!r} has no Collision layer; can't size Water")
        cw, ch = collision["__cWid"], collision["__cHei"]
        next_uid = int(data.get("nextUid", 1))
        data["nextUid"] = next_uid + 1
        iid = f"{LAYER_IDENT}-{next_uid:04d}"

        water_inst = {
            "__identifier": LAYER_IDENT,
            "__type": "IntGrid",
            "__cWid": cw,
            "__cHei": ch,
            "__gridSize": 16,
            "__opacity": 0.6,
            "__pxTotalOffsetX": 0,
            "__pxTotalOffsetY": 0,
            "__tilesetDefUid": None,
            "__tilesetRelPath": None,
            "iid": iid,
            "levelId": level["uid"],
            "layerDefUid": layer_uid,
            "pxOffsetX": 0,
            "pxOffsetY": 0,
            "visible": True,
            "optionalRules": [],
            "intGridCsv": [0] * (cw * ch),
            "autoLayerTiles": [],
            "seed": 0,
            "overrideTilesetUid": None,
            "gridTiles": [],
            "entityInstances": [],
        }
        # Insert RIGHT AFTER the Collision layer so editor draw order
        # stays predictable.
        idx = next(i for i, inst in enumerate(instances) if inst is collision)
        instances.insert(idx + 1, water_inst)
        levels_updated += 1

    LDTK_PATH.write_text(json.dumps(data, indent=2))
    print(f"added layer 'Water' (uid={layer_uid}); seeded {levels_updated} level(s) with empty Water grid")


if __name__ == "__main__":
    main()
