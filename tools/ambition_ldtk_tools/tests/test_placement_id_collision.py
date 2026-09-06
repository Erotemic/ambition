"""Authored placement ids must be globally unique where they enter the global `SimId` namespace.

The test detects duplicate placement ids across rooms rather than validating each
room in isolation."""

from __future__ import annotations

import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(REPO_ROOT / "tools" / "ambition_ldtk_tools"))

from ambition_ldtk_tools.validate_rules.authoring_hygiene import (  # noqa: E402
    placement_id_collision_issues,
)


def entity(identifier: str, entity_id: str) -> dict:
    return {
        "__identifier": identifier,
        "iid": f"{identifier}-{entity_id}",
        "px": [0, 0],
        "width": 16,
        "height": 16,
        "fieldInstances": [
            {"__identifier": "id", "__type": "String", "__value": entity_id}
        ],
    }


def project(rooms: dict[str, list[dict]]) -> dict:
    return {
        "levels": [
            {
                "identifier": room,
                "layerInstances": [{"__identifier": "Ambition", "entityInstances": ents}],
            }
            for room, ents in rooms.items()
        ]
    }


def test_one_id_in_two_rooms_is_reported():
    issues = placement_id_collision_issues(
        project({"lab": [entity("Switch", "big_red")], "vault": [entity("Switch", "big_red")]})
    )
    assert len(issues) == 1, issues
    assert issues[0].code == "validate.placement_id_collision"
    assert "big_red" in issues[0].message
    assert "lab" in issues[0].message and "vault" in issues[0].message


def test_the_same_id_twice_in_ONE_room_is_not_a_collision():
    # Two switches in one room sharing an id is a different (and lesser)
    # problem; this check is about the GLOBAL namespace crossing rooms.
    issues = placement_id_collision_issues(
        project({"lab": [entity("Switch", "big_red"), entity("Switch", "big_red")]})
    )
    assert issues == []


def test_different_kinds_sharing_an_id_do_not_collide():
    # `SimId::placement` is minted from the id, but a Switch and a Portal are
    # distinct authored things; keying by kind keeps the message actionable.
    issues = placement_id_collision_issues(
        project({"lab": [entity("Switch", "shared")], "vault": [entity("Portal", "shared")]})
    )
    assert issues == []


def test_loading_zones_are_exempt_because_their_ids_are_room_scoped():
    # `return_door` names the way back in seven shipped rooms on purpose: a
    # zone's `target_zone` is resolved WITHIN its `target_room`.
    issues = placement_id_collision_issues(
        project(
            {
                "hazards": [entity("LoadingZone", "return_door")],
                "treasure": [entity("LoadingZone", "return_door")],
            }
        )
    )
    assert issues == []
