"""`world move-levels`: a room moves to its dimension's world file whole, or
not at all (Jon, 2026-10-29: the world files broken up by dimension)."""

from __future__ import annotations

import copy

from ambition_ldtk_tools.edit.world_move import move, plan


def _entity(iid: str, ref: str | None = None, world: str = "src-world") -> dict:
    fields = []
    if ref is not None:
        fields.append({"__identifier": "mounted_on", "__type": "EntityRef", "defUid": 9,
                       "__value": {"entityIid": ref, "layerIid": "L", "levelIid": "V", "worldIid": world}})
    return {"iid": iid, "__identifier": "EnemySpawn", "defUid": 2, "fieldInstances": fields}


def _level(name: str, uid: int, entities: list[dict]) -> dict:
    return {"identifier": name, "uid": uid, "worldX": 0, "worldY": 0, "__neighbours": [{"levelIid": "x", "dir": "e"}],
            "layerInstances": [{"__identifier": "Entities", "layerDefUid": 1, "levelId": uid, "entityInstances": entities}]}


DEFS = {"layers": [{"uid": 1, "identifier": "Entities"}], "entities": [{"uid": 2, "identifier": "EnemySpawn", "fieldDefs": []}],
        "tilesets": []}


def _worlds():
    source = {"iid": "src-world", "nextUid": 10, "defs": copy.deepcopy(DEFS), "levels": [
        _level("trail", 3, [_entity("rider", "mount"), _entity("mount")]),
        _level("hub", 4, [_entity("door")]),
    ]}
    target = {"iid": "dino-world", "nextUid": 500, "defs": copy.deepcopy(DEFS), "levels": []}
    return source, target


def test_a_room_moves_whole_with_its_references_renamed_to_its_new_file():
    source, target = _worlds()
    assert plan(source, target, ["trail"]) == []
    move(source, target, ["trail"])
    assert [level["identifier"] for level in source["levels"]] == ["hub"]
    (trail,) = target["levels"]
    assert trail["uid"] == 500 and target["nextUid"] == 501, "its uid is from the new file's counter"
    assert trail["layerInstances"][0]["levelId"] == 500
    ref = trail["layerInstances"][0]["entityInstances"][0]["fieldInstances"][0]["__value"]
    assert ref == {"entityIid": "mount", "layerIid": "L", "levelIid": "V", "worldIid": "dino-world"}
    assert trail["__neighbours"] == [], "its neighbours are its new file's"


def test_a_reference_across_the_two_files_refuses_the_move():
    source, target = _worlds()
    source["levels"][1]["layerInstances"][0]["entityInstances"].append(_entity("hub_rider", "mount"))
    problems = plan(source, target, ["trail"])
    assert any("hub (stays)" in p and "across the two files" in p for p in problems), problems
    # The control: moving both rooms keeps the reference in one file.
    assert plan(source, target, ["trail", "hub"]) == []


def test_a_def_that_means_something_else_in_the_target_refuses_the_move():
    source, target = _worlds()
    target["defs"]["entities"][0]["fieldDefs"] = [{"uid": 7, "identifier": "respawn"}]
    problems = plan(source, target, ["trail"])
    assert any("entity def 2 (EnemySpawn) is not the source's" in p for p in problems), problems
    target["defs"]["entities"] = []
    assert any("no entity def 2" in p for p in plan(source, target, ["trail"]))


def test_an_unknown_or_present_level_refuses_the_move():
    source, target = _worlds()
    assert plan(source, target, ["nowhere"]) == ["level 'nowhere' is not in the source"]
    target["levels"].append({"identifier": "trail"})
    assert "level 'trail' is in the target already" in plan(source, target, ["trail"])
