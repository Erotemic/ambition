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
        # ⭐ none of these three appear in the spec — they are the file's.
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
