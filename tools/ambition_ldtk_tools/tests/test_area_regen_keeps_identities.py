"""A regenerated level keeps the identities of the level it replaces.

`area create --replace-existing` rebuilds a level wholesale. It allocated
every iid and uid again from `nextUid`, so each regen of an unchanged spec
renumbered every entity (LDTK-SEMANTIC-DIFF: 145 iids per run of the Hall
generator). The runtime uses an entity's iid as its id when it has no `id`
field, so saved state lost its subject.
"""

import copy
import json

from ambition_ldtk_tools.area_authoring import compile_area_create_plan


def _project():
    return {
        "jsonVersion": "1.5.3",
        "defaultGridSize": 16,
        "defaultLevelBgColor": "#000000",
        "nextUid": 100,
        "defs": {
            "layers": [
                {"identifier": "Collision", "uid": 10, "__type": "IntGrid", "gridSize": 16},
                {"identifier": "Ambition", "uid": 11, "__type": "Entities", "gridSize": 16},
            ],
            "entities": [
                {"identifier": "Marker", "uid": 30, "width": 16, "height": 16, "color": "#FF0000", "fieldDefs": []},
            ],
            "levelFields": [
                {"identifier": "activeArea", "uid": 20, "__type": "String", "type": "F_String"},
            ],
        },
        "levels": [],
    }


def _spec(*positions):
    return {
        "id": "tiny_area",
        "level_id": "tiny_area",
        "world_x": 0,
        "world_y": 0,
        "px_wid": 128,
        "px_hei": 64,
        "entities": [{"type": "Marker", "px": list(px)} for px in positions],
    }


def _create(project, spec, replace):
    plan, _level, _ = compile_area_create_plan(project, spec, replace_existing=replace)
    plan.apply(project)
    return project


def _entity_iids(project):
    (level,) = project["levels"]
    return {
        tuple(entity["px"]): entity["iid"]
        for layer in level["layerInstances"]
        for entity in layer.get("entityInstances") or []
    }


def test_a_second_regen_of_an_unchanged_spec_writes_the_same_project():
    spec = _spec((16, 16), (48, 16))
    first = _create(_project(), spec, replace=False)
    once = copy.deepcopy(_create(first, spec, replace=True))
    twice = _create(copy.deepcopy(once), spec, replace=True)
    assert json.dumps(once, sort_keys=True) == json.dumps(twice, sort_keys=True)


def test_a_regen_keeps_each_unmoved_entity_and_names_a_new_one_anew():
    project = _create(_project(), _spec((16, 16), (48, 16)), replace=False)
    before = _entity_iids(project)
    level_before = (project["levels"][0]["iid"], project["levels"][0]["uid"])
    _create(project, _spec((16, 16), (80, 16)), replace=True)
    after = _entity_iids(project)
    assert after[(16, 16)] == before[(16, 16)], "the unmoved marker keeps its iid"
    assert after[(80, 16)] not in before.values(), "the marker at a new place is a new entity"
    assert (project["levels"][0]["iid"], project["levels"][0]["uid"]) == level_before


def test_a_built_camera_zone_is_on_its_policy_layer_and_keeps_its_iid():
    project = _project()
    project["defs"]["layers"].append(
        {"identifier": "AmbitionCameras", "uid": 12, "__type": "Entities", "gridSize": 16}
    )
    project["defs"]["entities"].append(
        {"identifier": "CameraZone", "uid": 31, "width": 16, "height": 16, "color": "#00FF00", "fieldDefs": []}
    )
    spec = _spec((16, 16))
    spec["entities"].append({"type": "CameraZone", "px": [0, 0]})
    _create(project, spec, replace=False)

    def camera_layers():
        (level,) = project["levels"]
        return {
            layer["__identifier"]: entity["iid"]
            for layer in level["layerInstances"]
            for entity in layer.get("entityInstances") or []
            if entity["__identifier"] == "CameraZone"
        }

    built = camera_layers()
    assert list(built) == ["AmbitionCameras"], "the policy layer of a CameraZone"
    _create(project, spec, replace=True)
    assert camera_layers() == built, "a regen keeps the camera's layer and iid"
