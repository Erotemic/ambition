#!/usr/bin/env python3
"""Regression tests for the EdgeExit `touches_level_edge` check in
`ambition_ldtk_tools.validate`.

The original implementation shadowed `width`/`height` (level dims)
with the per-entity dims inside the entity loop, so the EdgeExit
edge check `touches_level_edge(entity, width, height)` compared the
entity rect against ITS OWN size — which is vacuously true for any
entity at px=[0, 0] and incorrectly approves every other
non-edge-touching zone too. These tests pin the corrected behavior:

- A mid-room EdgeExit (px clearly inside the level, not touching any
  edge) MUST produce a validation error.
- A genuine top-edge EdgeExit must validate clean.
- A genuine left-edge EdgeExit must still validate clean (kept as a
  guard against regressing back into "always passes regardless of
  position").

The tests build a synthetic LDtk project in-memory and call
`validate(...)` directly rather than invoking the CLI. The minimal
project ships the entity / layer / level field defs the validator
needs; everything else (tilesets, audio, etc.) is omitted.
"""
from __future__ import annotations

import copy
import json
import sys
import tempfile
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[3]
PKG_ROOT = REPO_ROOT / "tools" / "ambition_ldtk_tools"
sys.path.insert(0, str(PKG_ROOT))

from ambition_ldtk_tools.validate import validate  # noqa: E402


def _load_real_project() -> dict:
    """Use the real sandbox.ldtk defs (they have all the entity / field
    UIDs the validator looks up) so the synthetic project under test
    only differs in its levels."""
    src = REPO_ROOT / "crates" / "ambition_sandbox" / "assets" / "ambition" / "worlds" / "sandbox.ldtk"
    return json.loads(src.read_text())


def _strip_to_defs(project: dict) -> dict:
    """Return a copy of `project` with the level list cleared so the
    test can add only the level(s) it wants to exercise."""
    out = copy.deepcopy(project)
    out["levels"] = []
    return out


def _entity_def_uid(project: dict, identifier: str) -> int:
    for entry in project["defs"]["entities"]:
        if entry["identifier"] == identifier:
            return entry["uid"]
    raise KeyError(identifier)


def _field_def_uid(project: dict, entity_identifier: str, field_identifier: str) -> int:
    for entry in project["defs"]["entities"]:
        if entry["identifier"] != entity_identifier:
            continue
        for field in entry.get("fieldDefs") or []:
            if field["identifier"] == field_identifier:
                return field["uid"]
    raise KeyError((entity_identifier, field_identifier))


def _level_field_def_uid(project: dict, identifier: str) -> int:
    for field in project["defs"]["levelFields"]:
        if field["identifier"] == identifier:
            return field["uid"]
    raise KeyError(identifier)


def _make_loading_zone(
    project: dict,
    *,
    iid: str,
    zone_id: str,
    px: list[int],
    size: list[int],
    activation: str,
    target_room: str,
    target_zone: str,
    world_x: int,
    world_y: int,
) -> dict:
    def_uid = _entity_def_uid(project, "LoadingZone")
    id_uid = _field_def_uid(project, "LoadingZone", "id")
    name_uid = _field_def_uid(project, "LoadingZone", "name")
    activation_uid = _field_def_uid(project, "LoadingZone", "activation")
    target_room_uid = _field_def_uid(project, "LoadingZone", "target_room")
    target_zone_uid = _field_def_uid(project, "LoadingZone", "target_zone")
    bidir_uid = _field_def_uid(project, "LoadingZone", "bidirectional")
    return {
        "__identifier": "LoadingZone",
        "__grid": [px[0] // 16, px[1] // 16],
        "__pivot": [0, 0],
        "__tags": [],
        "__tile": None,
        "__smartColor": "#FFD166",
        "__worldX": world_x + px[0],
        "__worldY": world_y + px[1],
        "iid": iid,
        "width": size[0],
        "height": size[1],
        "defUid": def_uid,
        "px": list(px),
        "fieldInstances": [
            {"__identifier": "id", "__type": "String", "__value": zone_id,
             "__tile": None, "defUid": id_uid,
             "realEditorValues": [{"id": "V_String", "params": [zone_id]}]},
            {"__identifier": "name", "__type": "String", "__value": zone_id,
             "__tile": None, "defUid": name_uid,
             "realEditorValues": [{"id": "V_String", "params": [zone_id]}]},
            {"__identifier": "activation", "__type": "String", "__value": activation,
             "__tile": None, "defUid": activation_uid,
             "realEditorValues": [{"id": "V_String", "params": [activation]}]},
            {"__identifier": "target_room", "__type": "String", "__value": target_room,
             "__tile": None, "defUid": target_room_uid,
             "realEditorValues": [{"id": "V_String", "params": [target_room]}]},
            {"__identifier": "target_zone", "__type": "String", "__value": target_zone,
             "__tile": None, "defUid": target_zone_uid,
             "realEditorValues": [{"id": "V_String", "params": [target_zone]}]},
            {"__identifier": "bidirectional", "__type": "Bool", "__value": False,
             "__tile": None, "defUid": bidir_uid,
             "realEditorValues": [{"id": "V_Bool", "params": [False]}]},
        ],
    }


def _make_player_start(project: dict, *, iid: str, px: list[int], world_x: int, world_y: int, name: str) -> dict:
    def_uid = _entity_def_uid(project, "PlayerStart")
    name_uid = _field_def_uid(project, "PlayerStart", "name")
    return {
        "__identifier": "PlayerStart",
        "__grid": [px[0] // 16, px[1] // 16],
        "__pivot": [0, 0],
        "__tags": [],
        "__tile": None,
        "__smartColor": "#41D1FF",
        "__worldX": world_x + px[0],
        "__worldY": world_y + px[1],
        "iid": iid,
        "width": 28,
        "height": 46,
        "defUid": def_uid,
        "px": list(px),
        "fieldInstances": [
            {"__identifier": "name", "__type": "String", "__value": name,
             "__tile": None, "defUid": name_uid,
             "realEditorValues": [{"id": "V_String", "params": [name]}]},
        ],
    }


def _make_level(
    project: dict,
    *,
    identifier: str,
    uid: int,
    world_x: int,
    world_y: int,
    px_wid: int,
    px_hei: int,
    entities: list[dict],
    extra_levels: list[dict] | None = None,
) -> dict:
    active_area_uid = _level_field_def_uid(project, "activeArea")
    biome_uid = _level_field_def_uid(project, "biome")
    music_uid = _level_field_def_uid(project, "music_track")
    grid_w = px_wid // 16
    grid_h = px_hei // 16
    collision_csv = [0] * (grid_w * grid_h)
    return {
        "identifier": identifier,
        "iid": f"{identifier}-{uid}",
        "uid": uid,
        "worldX": world_x,
        "worldY": world_y,
        "worldDepth": 0,
        "pxWid": px_wid,
        "pxHei": px_hei,
        "__bgColor": "#202030",
        "bgColor": "#202030",
        "useAutoIdentifier": False,
        "bgRelPath": None,
        "bgPos": None,
        "bgPivotX": 0.5,
        "bgPivotY": 0.5,
        "__smartColor": "#FFFFFF",
        "__bgPos": None,
        "externalRelPath": None,
        "fieldInstances": [
            {"__identifier": "activeArea", "__type": "String", "__value": identifier,
             "__tile": None, "defUid": active_area_uid,
             "realEditorValues": [{"id": "V_String", "params": [identifier]}]},
            {"__identifier": "biome", "__type": "String", "__value": "lab",
             "__tile": None, "defUid": biome_uid,
             "realEditorValues": [{"id": "V_String", "params": ["lab"]}]},
            {"__identifier": "music_track", "__type": "String", "__value": "tech_bros_disruption",
             "__tile": None, "defUid": music_uid,
             "realEditorValues": [{"id": "V_String", "params": ["tech_bros_disruption"]}]},
        ],
        "layerInstances": [
            {
                "__identifier": "Collision",
                "__type": "IntGrid",
                "iid": f"Collision-{uid}",
                "layerDefUid": _collision_layer_uid(project),
                "intGridCsv": collision_csv,
                "__cWid": grid_w,
                "__cHei": grid_h,
                "__gridSize": 16,
                "__opacity": 1,
                "__pxTotalOffsetX": 0,
                "__pxTotalOffsetY": 0,
                "__tilesetDefUid": None,
                "__tilesetRelPath": None,
                "levelId": uid,
                "pxOffsetX": 0,
                "pxOffsetY": 0,
                "visible": True,
                "optionalRules": [],
                "autoLayerTiles": [],
                "seed": uid,
                "overrideTilesetUid": None,
                "gridTiles": [],
                "entityInstances": [],
            },
            {
                "__identifier": "Ambition",
                "__type": "Entities",
                "iid": f"Ambition-{uid}",
                "layerDefUid": _ambition_layer_uid(project),
                "__cWid": grid_w,
                "__cHei": grid_h,
                "__gridSize": 16,
                "__opacity": 1,
                "__pxTotalOffsetX": 0,
                "__pxTotalOffsetY": 0,
                "__tilesetDefUid": None,
                "__tilesetRelPath": None,
                "levelId": uid,
                "pxOffsetX": 0,
                "pxOffsetY": 0,
                "visible": True,
                "optionalRules": [],
                "autoLayerTiles": [],
                "seed": uid,
                "overrideTilesetUid": None,
                "gridTiles": [],
                "entityInstances": entities,
            },
        ],
        "__neighbours": [],
    }


def _collision_layer_uid(project: dict) -> int:
    for layer in project["defs"]["layers"]:
        if layer["identifier"] == "Collision":
            return layer["uid"]
    raise KeyError("Collision")


def _ambition_layer_uid(project: dict) -> int:
    for layer in project["defs"]["layers"]:
        if layer["identifier"] == "Ambition":
            return layer["uid"]
    raise KeyError("Ambition")


def _write_and_validate(project: dict) -> tuple[list[str], list[str]]:
    with tempfile.NamedTemporaryFile(
        mode="w", suffix=".ldtk", delete=False
    ) as fh:
        fh.write(json.dumps(project))
        path = Path(fh.name)
    try:
        return validate(path)
    finally:
        path.unlink(missing_ok=True)


def _scenario(*, source_zone_px, source_zone_size, target_zone_px, target_zone_size):
    """Build a two-level project with reciprocal EdgeExits whose
    placement is parameterized so each test can pick where the source
    zone sits."""
    project = _strip_to_defs(_load_real_project())
    src_entities = [
        _make_player_start(project, iid="PlayerStart-src", px=[64, 64],
                           world_x=0, world_y=0, name="src_spawn"),
        _make_loading_zone(
            project,
            iid="LoadingZone-src",
            zone_id="src_to_dst",
            px=source_zone_px,
            size=source_zone_size,
            activation="EdgeExit",
            target_room="dst_area",
            target_zone="dst_from_src",
            world_x=0,
            world_y=0,
        ),
    ]
    dst_entities = [
        _make_player_start(project, iid="PlayerStart-dst", px=[64, 64],
                           world_x=200, world_y=0, name="dst_spawn"),
        _make_loading_zone(
            project,
            iid="LoadingZone-dst",
            zone_id="dst_from_src",
            px=target_zone_px,
            size=target_zone_size,
            activation="EdgeExit",
            target_room="src_area",
            target_zone="src_to_dst",
            world_x=200,
            world_y=0,
        ),
    ]
    project["levels"] = [
        _make_level(project, identifier="src", uid=900_001,
                    world_x=0, world_y=0, px_wid=200, px_hei=200,
                    entities=src_entities),
        _make_level(project, identifier="dst", uid=900_002,
                    world_x=200, world_y=0, px_wid=200, px_hei=200,
                    entities=dst_entities),
    ]
    # The validator keys area lookups by `activeArea`, not `identifier`.
    # Use distinct active areas so each level is its own area.
    project["levels"][0]["fieldInstances"][0]["__value"] = "src_area"
    project["levels"][0]["fieldInstances"][0]["realEditorValues"] = [
        {"id": "V_String", "params": ["src_area"]}
    ]
    project["levels"][1]["fieldInstances"][0]["__value"] = "dst_area"
    project["levels"][1]["fieldInstances"][0]["realEditorValues"] = [
        {"id": "V_String", "params": ["dst_area"]}
    ]
    return project


def test_mid_room_edgeexit_fails_validation():
    """The bug case: a 96×16 EdgeExit zone placed in the *middle* of a
    200×200 level (px=[60, 80] so it's nowhere near any edge) MUST
    produce an error mentioning EdgeExit + level edge. Before the fix
    the shadowed `width`/`height` made this pass."""
    project = _scenario(
        source_zone_px=[60, 80],
        source_zone_size=[96, 16],
        # Target zone is fine (right edge). Only the source is mid-room.
        target_zone_px=[184, 80],
        target_zone_size=[16, 96],
    )
    errors, _warnings = _write_and_validate(project)
    edge_errors = [e for e in errors if "EdgeExit" in e and "level edge" in e]
    assert edge_errors, (
        "expected an EdgeExit-edge violation for the mid-room source zone, "
        f"got {len(errors)} other errors: {errors!r}"
    )


def test_top_edge_edgeexit_passes_validation():
    """An EdgeExit zone placed flush with the top edge (y=0) of a
    level — the vertical-seam case the runtime now supports — must
    NOT produce the touches-edge error."""
    project = _scenario(
        source_zone_px=[60, 0],  # top edge
        source_zone_size=[96, 16],
        target_zone_px=[60, 184],  # matching bottom edge in dst
        target_zone_size=[96, 16],
    )
    errors, _warnings = _write_and_validate(project)
    edge_errors = [e for e in errors if "EdgeExit" in e and "level edge" in e]
    assert not edge_errors, (
        f"unexpected EdgeExit-edge errors for an edge-flush zone: {edge_errors!r}"
    )


def test_left_edge_edgeexit_still_passes_validation():
    """Side-scroll EdgeExits (the historical pattern) must keep
    working — the fix isn't supposed to regress the left/right
    behavior into a stricter shape."""
    project = _scenario(
        source_zone_px=[0, 60],  # left edge
        source_zone_size=[16, 96],
        target_zone_px=[184, 60],  # right edge of dst
        target_zone_size=[16, 96],
    )
    errors, _warnings = _write_and_validate(project)
    edge_errors = [e for e in errors if "EdgeExit" in e and "level edge" in e]
    assert not edge_errors, (
        f"side-scroll left-edge EdgeExit regressed: {edge_errors!r}"
    )


if __name__ == "__main__":
    test_mid_room_edgeexit_fails_validation()
    test_top_edge_edgeexit_passes_validation()
    test_left_edge_edgeexit_still_passes_validation()
    print("EdgeExit validation regression tests: PASS")
