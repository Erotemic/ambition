"""`room relationships` finds NATIVE links without being told they exist.

⭐ **this is the whole argument for migrating a relationship to `EntityRef`,
expressed as a test.** A native reference is declared in the project schema, so
the inspector discovers it, names its target, and can say the target is missing —
all without knowing what the relationship MEANS. A string-convention reference is
indistinguishable from a label, so the same inspector can only report the ones it
carries a hand-written row for.

The fixture therefore uses an `EntityRef` field the tool has never heard of. If
someone "helpfully" replaces schema discovery with a list of known ref fields,
this fails — and the list would be the very rot the migration is meant to end.
"""

from __future__ import annotations

from ambition_ldtk_tools.room_support.relationships import relationship_report


def _project(target_iid: str) -> dict:
    """A rider pointing at a mount through a field named nowhere in the tool."""
    return {
        "defs": {
            "entities": [
                {
                    "identifier": "Gizmo",
                    "fieldDefs": [
                        {"identifier": "label", "__type": "String"},
                        # ⚠ NOT `mounted_on`, and not in any table the module
                        # ships: discovery has to come from the schema.
                        {"identifier": "tethered_to", "__type": "EntityRef"},
                    ],
                }
            ]
        },
        "levels": [
            {
                "identifier": "fixture_room",
                "layerInstances": [
                    {
                        "__type": "Entities",
                        "entityInstances": [
                            {
                                "iid": "Gizmo-1",
                                "__identifier": "Gizmo",
                                "fieldInstances": [
                                    {
                                        "__identifier": "tethered_to",
                                        "__value": {"entityIid": target_iid},
                                    }
                                ],
                            },
                            {
                                "iid": "Gizmo-2",
                                "__identifier": "Gizmo",
                                "fieldInstances": [],
                            },
                        ],
                    }
                ],
            }
        ],
    }


def test_a_native_reference_is_discovered_from_the_schema_alone():
    report = relationship_report(_project("Gizmo-2"))

    assert report["native_entity_ref_fields"] == {"Gizmo": ["tethered_to"]}
    assert len(report["native"]) == 1
    link = report["native"][0]
    assert link["source"] == "Gizmo-1"
    assert link["field"] == "tethered_to"
    assert link["target_iid"] == "Gizmo-2"
    assert link["target_kind"] == "Gizmo"
    assert not link["broken"]

    # ...and the poison: a plain String field beside it is NOT a relationship.
    # If `label` shows up, the tool started guessing, and a guess that calls a
    # label a reference will call a healthy room broken.
    assert all(row["field"] != "label" for row in report["native"])


def test_a_dangling_native_reference_names_the_offending_entity():
    report = relationship_report(_project("Gizmo-does-not-exist"))

    link = report["native"][0]
    assert link["broken"], "a ref to an entity that is not in the project is broken"
    # The bar K5 sets: the diagnostic names WHICH entity is wrong, not just that
    # something is. A count is not something an author can act on.
    assert link["source"] == "Gizmo-1"
    assert link["level"] == "fixture_room"
