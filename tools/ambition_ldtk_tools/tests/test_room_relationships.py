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

from unittest import mock

from ambition_ldtk_tools.room_support import relationships as rel
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
                        # NOT `mounted_on`, and not in any table the module
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


# ---------------------------------------------------------------------------
# A prefix convention is reported with ITS OWN resolver owner.
#
# The report site spelled `KinematicPathSpec::matches_id` for every prefix row, so
# `BossSpawn.brain = "PhaseScript:<id>"` — a boss phase script, resolved by `parse_boss_brain`,
# with no relationship to kinematic paths whatsoever — was attributed to the resolver that owns
# `EnemySpawn`'s `Patrol:` references. The entire value of this half of the diagnostic is
# telling an author WHICH authority to go read, so naming the wrong one is worse than saying
# nothing. ---------------------------------------------------------------------------


def _entity_with_field(kind: str, iid: str, field: str, value: str) -> dict:
    return {
        "defs": {"entities": []},
        "levels": [
            {
                "identifier": "fixture_room",
                "layerInstances": [
                    {
                        "__type": "Entities",
                        "entityInstances": [
                            {
                                "iid": iid,
                                "__identifier": kind,
                                "fieldInstances": [
                                    {"__identifier": field, "__value": value}
                                ],
                            }
                        ],
                    }
                ],
            }
        ],
    }


def test_every_prefix_convention_reports_the_owner_its_own_row_declares():
    """The invariant: ownership travels with the row, never with the report site.

    Driven off the shipped table rather than a copy of it, so a row that leaves
    (a relationship migrating to `EntityRef`) takes its assertion with it and a
    row that arrives is covered the day it lands.
    """
    assert rel.PREFIX_CONVENTIONS, "the table under test is empty"
    for kind, field, prefix, _points_at, owner in rel.PREFIX_CONVENTIONS:
        report = relationship_report(
            _entity_with_field(kind, f"{kind}-1", field, f"{prefix}some_target")
        )
        rows = [r for r in report["conventional"] if r["source_kind"] == kind]
        assert len(rows) == 1, f"{kind}.{field} authored `{prefix}` was not reported"
        assert rows[0]["resolution_owned_by"] == owner, (
            f"{kind}.{field} `{prefix}` is resolved by `{owner}`, and the report "
            f"named `{rows[0]['resolution_owned_by']}`"
        )


def test_a_phase_script_reference_is_not_attributed_to_the_patrol_resolver():
    """The exact bug, named: the string that was wrong must not come back."""
    report = relationship_report(
        _entity_with_field("BossSpawn", "BossSpawn-1", "brain", "PhaseScript:act_two")
    )
    row = report["conventional"][0]
    assert row["spelling"] == "act_two", "precondition: the prefix row matched"
    assert "KinematicPathSpec" not in row["resolution_owned_by"], (
        "a boss phase script has nothing to do with kinematic paths; this is the "
        "resolver that owns EnemySpawn's `Patrol:` references"
    )
    assert "boss" in row["resolution_owned_by"].lower()


def test_two_conventions_do_not_share_one_resolver_name():
    """The poison: a single owner spelled at the report site cannot pass.

    The shipped table is allowed to shrink to one row (that is the migration
    working), and a one-row sweep cannot distinguish "read off the row" from
    "hardcoded, and the hardcode happens to match". Two synthetic rows with
    distinct owners can, and they exercise the production code path unchanged.
    """
    synthetic = (
        ("AlphaSpawn", "brain", "Alpha:", "an alpha target", "alpha_resolver"),
        ("BetaSpawn", "brain", "Beta:", "a beta target", "beta_resolver"),
    )
    project = _entity_with_field("AlphaSpawn", "AlphaSpawn-1", "brain", "Alpha:one")
    project["levels"][0]["layerInstances"][0]["entityInstances"].append(
        {
            "iid": "BetaSpawn-1",
            "__identifier": "BetaSpawn",
            "fieldInstances": [{"__identifier": "brain", "__value": "Beta:two"}],
        }
    )
    with mock.patch.object(rel, "PREFIX_CONVENTIONS", synthetic):
        report = relationship_report(project)
    owners = {
        r["source_kind"]: r["resolution_owned_by"] for r in report["conventional"]
    }
    assert owners == {"AlphaSpawn": "alpha_resolver", "BetaSpawn": "beta_resolver"}
