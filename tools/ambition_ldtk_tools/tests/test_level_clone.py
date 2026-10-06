"""`level clone`: a copy of a room is its own room.

On the shipped sandbox world: the GNU-ton arena is the room whose boss rides
a mount by EntityRef (`mounted_on`), so it is the case where re-pointing an
in-level reference matters.
"""

from __future__ import annotations

import json

from ambition_ldtk_tools.edit.level_clone import clone_level
from ambition_ldtk_tools.ldtk.paths import default_sandbox_ldtk


def _entities(level):
    return [e for layer in level["layerInstances"] for e in layer.get("entityInstances", [])]


def test_a_cloned_room_has_fresh_ids_and_points_at_its_own_entities():
    project = json.loads(default_sandbox_ldtk().read_text())
    source = next(l for l in project["levels"] if l["identifier"] == "gnu_ton_arena")
    uid_before = project["nextUid"]

    copy = clone_level(project, "gnu_ton_arena", "gnu_ton_arena_copy", 900_000, 0)

    assert copy["identifier"] == "gnu_ton_arena_copy"
    assert (copy["worldX"], copy["worldY"]) == (900_000, 0)
    assert copy["uid"] >= uid_before and copy["iid"] != source["iid"]
    # every layer and entity has a new iid, none shared with the source
    source_iids = {e["iid"] for e in _entities(source)} | {l["iid"] for l in source["layerInstances"]}
    copy_iids = {e["iid"] for e in _entities(copy)} | {l["iid"] for l in copy["layerInstances"]}
    assert len(copy_iids) == len(source_iids)
    assert not copy_iids & source_iids
    assert all(layer["levelId"] == copy["uid"] for layer in copy["layerInstances"])
    # the same entities with the same authored fields, moved with the level
    assert sorted(e["__identifier"] for e in _entities(copy)) == sorted(e["__identifier"] for e in _entities(source))
    # the rider's mount reference points at the COPY's mount, not the source's
    rider = next(e for e in _entities(copy) if e["__identifier"] == "BossSpawn")
    mount_ref = next(f["__value"] for f in rider["fieldInstances"] if f["__identifier"] == "mounted_on")
    assert mount_ref is not None, "precondition: the GNU-ton rider is mounted by EntityRef"
    assert mount_ref["entityIid"] in {e["iid"] for e in _entities(copy)}
    assert mount_ref["levelIid"] == copy["iid"]
    # its own active area
    area = next(f["__value"] for f in copy["fieldInstances"] if f["__identifier"] == "activeArea")
    assert area == "gnu_ton_arena_copy"
    # the source is untouched
    assert next(l for l in project["levels"] if l["identifier"] == "gnu_ton_arena") is source
