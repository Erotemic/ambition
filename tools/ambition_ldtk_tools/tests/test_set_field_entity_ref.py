"""`entity set-field` on an EntityRef names the TARGET and derives the rest.

⛔ **the three container iids are the part a paste gets wrong.** LDtk stores a
ref as `{entityIid, layerIid, levelIid, worldIid}`; only the first is a fact
about the link, and the other three describe wherever the target happens to live
in the file being edited today. A spec carrying its own `layerIid` is a spec
that keeps asserting a container name after the container has been renamed, and
the ref it writes resolves to nothing while looking perfectly authored.

Written for D49, where four `mounted_on` refs in `pirate_sky_lookout` had to be
restored from a month-old blob: the `entityIid`s came from history and these
tests are why nothing else did.
"""

from __future__ import annotations

import pytest

from ambition_ldtk_tools.edit.set_field import apply_field_edit, resolve_entity_ref
from ambition_ldtk_tools.ldtk.fields import ensure_entity_ref_fielddef


def _project() -> dict:
    """Two entities in one level: a rider and the mount it should point at."""

    def spawn(iid: str, px: list[int], with_ref: bool) -> dict:
        fields = [
            {
                "__identifier": "brain",
                "__type": "String",
                "__value": "pirate_shark_rider",
                "__tile": None,
                "defUid": 3019,
                "realEditorValues": [],
            }
        ]
        if with_ref:
            fields.append(
                {
                    "__identifier": "mounted_on",
                    "__type": "EntityRef",
                    "__value": None,
                    "__tile": None,
                    "defUid": 6805,
                    "realEditorValues": [],
                }
            )
        return {
            "__identifier": "EnemySpawn",
            "iid": iid,
            "px": px,
            "width": 44,
            "height": 78,
            "defUid": 2008,
            "fieldInstances": fields,
        }

    return {
        "iid": "test-world",
        "defs": {
            "entities": [
                {
                    "identifier": "EnemySpawn",
                    "uid": 2008,
                    "fieldDefs": [
                        {"identifier": "brain", "uid": 3019, "__type": "String"},
                        {"identifier": "mounted_on", "uid": 6805, "__type": "EntityRef"},
                    ],
                }
            ]
        },
        "levels": [
            {
                "iid": "lookout-1",
                "identifier": "pirate_sky_lookout",
                "layerInstances": [
                    {
                        "__identifier": "Ambition",
                        "iid": "Ambition-1",
                        "entityInstances": [
                            spawn("rider-1", [192, 240], True),
                            spawn("mount-1", [192, 240], False),
                        ],
                    }
                ],
            }
        ],
    }


def test_a_target_iid_becomes_a_full_ref_read_out_of_the_project():
    project = _project()
    rider = project["levels"][0]["layerInstances"][0]["entityInstances"][0]

    apply_field_edit(project, rider, "mounted_on", "mount-1")

    ref = next(
        f["__value"] for f in rider["fieldInstances"] if f["__identifier"] == "mounted_on"
    )
    assert ref == {
        "entityIid": "mount-1",
        # none of these three appear in the spec — they are the file's.
        "layerIid": "Ambition-1",
        "levelIid": "lookout-1",
        "worldIid": "test-world",
    }


def test_a_ref_to_a_missing_iid_is_refused():
    """⛔ the failure this whole path exists to prevent, and it is SILENT.

    A `mounted_on` naming an iid nobody minted reads back as an unset ref, so
    the rider spawns alone and the game says nothing. It has to die here.
    """
    project = _project()
    with pytest.raises(SystemExit) as ex:
        resolve_entity_ref(project, "EnemySpawn-does-not-exist")
    assert "not an entity in this project" in str(ex.value)


def test_a_prebuilt_ref_object_is_refused_with_the_reason():
    project = _project()
    with pytest.raises(SystemExit) as ex:
        resolve_entity_ref(
            project,
            {
                "entityIid": "mount-1",
                "layerIid": "Ambition-STALE",
                "levelIid": "lookout-1",
                "worldIid": "test-world",
            },
        )
    assert "cannot go stale" in str(ex.value)


def test_null_clears_the_ref():
    assert resolve_entity_ref(_project(), None) is None


# ---------------------------------------------------------------------------
# `path_ref` — the SECOND native reference relationship.
#
# two adopters is what earns a shared synthesizer.


def _project_with_paths() -> dict:
    """The rider/mount fixture plus a `KinematicPath` for a patrol to point at."""
    project = _project()
    project["nextUid"] = 9000
    project["defs"]["entities"].append(
        {
            "identifier": "KinematicPath",
            "uid": 2012,
            "fieldDefs": [{"identifier": "id", "uid": 4341, "__type": "String"}],
        }
    )
    project["levels"][0]["layerInstances"][0]["entityInstances"].append(
        {
            "__identifier": "KinematicPath",
            "iid": "path-1",
            "px": [128, 240],
            "width": 360,
            "height": 12,
            "defUid": 2012,
            "fieldInstances": [],
        }
    )
    return project


def test_a_path_ref_field_def_is_synthesized_once_and_scoped_to_kinematic_paths():
    project = _project_with_paths()

    made = ensure_entity_ref_fielddef(project, "EnemySpawn", "path_ref")

    assert made["__type"] == "EntityRef" and made["type"] == "F_EntityRef"
    # the editor itself will only offer KinematicPath targets, so the
    # wrong-kind mistake cannot be made by hand at all.
    assert made["allowedRefs"] == "OnlySpecificEntity"
    assert made["allowedRefsEntityUid"] == 2012, "must name the KinematicPath def"
    # Idempotent: re-running an authoring pass must not mint a second field def
    # (a duplicate identifier makes LDtk refuse the file outright).
    again = ensure_entity_ref_fielddef(project, "EnemySpawn", "path_ref")
    assert again is made
    es_def = next(
        e for e in project["defs"]["entities"] if e["identifier"] == "EnemySpawn"
    )
    assert [f["identifier"] for f in es_def["fieldDefs"]].count("path_ref") == 1


def test_a_path_ref_resolves_to_the_path_and_refuses_anything_else():
    project = _project_with_paths()
    ensure_entity_ref_fielddef(project, "EnemySpawn", "path_ref")
    spawn = project["levels"][0]["layerInstances"][0]["entityInstances"][0]

    apply_field_edit(project, spawn, "path_ref", "path-1")
    ref = next(
        f["__value"] for f in spawn["fieldInstances"] if f["__identifier"] == "path_ref"
    )
    assert ref["entityIid"] == "path-1"
    assert ref["layerIid"] == "Ambition-1", "the containers come from the file"

    # THE POISON, and it is the whole reason the def declares a scope: a ref at
    # the wrong KIND of entity writes cleanly and fails at load. The def already
    # said what this field may point at; now something reads it.
    with pytest.raises(SystemExit) as ex:
        apply_field_edit(project, spawn, "path_ref", "mount-1")
    assert "mount-1" in str(ex.value)
    assert "EnemySpawn" in str(ex.value)


def test_an_undocumented_ref_field_is_refused_rather_than_invented():
    """A reference field nobody documented is the string convention this replaced."""
    project = _project_with_paths()
    with pytest.raises(SystemExit) as ex:
        ensure_entity_ref_fielddef(project, "EnemySpawn", "points_at_something")
    assert "ENTITY_REF_FIELDS" in str(ex.value)
