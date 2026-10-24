"""`entity move` with `to_level` carries an entity into another level.

The LDtk editor cannot cut an entity out of one level and paste it into
another (Jon, 2026-10-24); this is the tool's version of that move.
"""

from __future__ import annotations

from ambition_ldtk_tools.edit.move import apply_move, move_to_level


def _level(iid: str, ident: str, world_x: int, entities: list[dict]) -> dict:
    return {
        "iid": iid,
        "identifier": ident,
        "worldX": world_x,
        "worldY": 0,
        "layerInstances": [
            {"__identifier": "Ambition", "iid": f"Ambition-{iid}", "__gridSize": 16, "entityInstances": entities}
        ],
    }


def test_an_entity_moves_into_another_level_and_its_refs_follow() -> None:
    door = {"__identifier": "LoadingZone", "iid": "door-1", "px": [64, 368], "width": 48, "height": 96, "fieldInstances": []}
    pointer = {
        "__identifier": "Sign",
        "iid": "sign-1",
        "px": [0, 0],
        "fieldInstances": [
            {"__identifier": "points_at", "__value": {"entityIid": "door-1", "layerIid": "Ambition-wing", "levelIid": "wing", "worldIid": "w"}},
        ],
    }
    wing = _level("wing", "dev_wing", 1904, [door])
    hub = _level("hub", "hub_main", 0, [pointer])
    project = {"levels": [hub, wing]}

    apply_move(door, [1133, 624], None, 16, hub["worldX"], hub["worldY"])
    move_to_level(project, wing["layerInstances"][0], door, hub)

    assert door not in wing["layerInstances"][0]["entityInstances"]
    assert door in hub["layerInstances"][0]["entityInstances"]
    assert door["px"] == [1133, 624] and door["__worldX"] == 1133 and door["__grid"] == [70, 39]
    ref = pointer["fieldInstances"][0]["__value"]
    assert (ref["levelIid"], ref["layerIid"]) == ("hub", "Ambition-hub")
