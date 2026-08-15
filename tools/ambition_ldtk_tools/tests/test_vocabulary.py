"""`vocabulary list` / `vocabulary check`.

The defect these were written for: `mary_o_1_3` was authored through
`area create` with six `EnemySpawn` entities carrying no `character_id`, and
`repair` + `validate` both reported green. The converter refuses that field's
absence, so the room would have panicked the game on load.

⭐ **the poison here is the real defect, and its complement is the guard.** The
same project appears twice — once with the field authored, once blank — because
a check that only ever sees the broken world cannot show it is not simply
shouting at everything.
"""

from __future__ import annotations

import copy

from ambition_ldtk_tools.vocabulary import build_listing, check_issues


def _entity(identifier: str, iid: str, **fields):
    return {
        "__identifier": identifier,
        "iid": iid,
        "px": [0, 0],
        "width": 32,
        "height": 32,
        "fieldInstances": [
            {"__identifier": k, "__value": v} for k, v in fields.items()
        ],
    }


def _project():
    return {
        "defs": {
            "enums": [
                {
                    "identifier": "Brain",
                    "values": [{"id": "slop"}, {"id": "snake"}],
                }
            ],
            "layers": [
                {
                    "identifier": "Ambition",
                    "__type": "Entities",
                    "requiredTags": [],
                    "excludedTags": ["Camera"],
                },
                {
                    "identifier": "AmbitionCameras",
                    "__type": "Entities",
                    "requiredTags": ["Camera"],
                    "excludedTags": [],
                },
            ],
            "entities": [
                {
                    "identifier": "EnemySpawn",
                    "width": 38,
                    "height": 66,
                    "color": "#ff0000",
                    "tags": [],
                    "fieldDefs": [
                        {"identifier": "brain", "__type": "LocalEnum.Brain"},
                        {"identifier": "character_id", "__type": "String"},
                        {"identifier": "mounted_on", "__type": "EntityRef"},
                    ],
                },
                {
                    "identifier": "CameraZone",
                    "width": 320,
                    "height": 180,
                    "color": "#00ff00",
                    "tags": ["Camera"],
                    "fieldDefs": [],
                },
            ],
        },
        "levels": [
            {
                "identifier": "room_one",
                "layerInstances": [
                    {
                        "__identifier": "Ambition",
                        "entityInstances": [
                            _entity(
                                "EnemySpawn", "a", brain="slop", character_id="ai_slop"
                            ),
                            _entity(
                                "EnemySpawn",
                                "b",
                                brain="snake",
                                character_id="solid_snake",
                            ),
                        ],
                    }
                ],
            },
            {
                "identifier": "room_two",
                "layerInstances": [
                    {
                        "__identifier": "Ambition",
                        "entityInstances": [
                            _entity(
                                "EnemySpawn", "c", brain="slop", character_id="ai_slop"
                            )
                        ],
                    }
                ],
            },
        ],
    }


def test_check_is_silent_when_every_placement_agrees() -> None:
    assert check_issues(_project(), None) == []


def test_check_names_the_placement_that_omits_what_its_siblings_author() -> None:
    project = _project()
    # The exact `mary_o_1_3` defect: a new room's spawn carries no character_id.
    project["levels"][1]["layerInstances"][0]["entityInstances"][0]["fieldInstances"][
        1
    ]["__value"] = ""

    issues = check_issues(project, None)
    assert [issue.code for issue in issues] == ["vocabulary.field_omitted_here"]
    issue = issues[0]
    assert issue.level == "room_two"
    assert issue.entity_iid == "c"
    assert issue.field == "character_id"
    # The fix hint is the census, so the author is told what to write rather
    # than only that something is missing.
    assert "ai_slop" in (issue.fix_hint or "")
    assert "solid_snake" in (issue.fix_hint or "")

    # ...and scoping to the healthy level must not report the other one's defect.
    assert check_issues(project, "room_one") == []


def test_a_field_only_some_placements_want_is_never_flagged() -> None:
    """`mounted_on` is blank on all three, and optional fields must stay quiet.

    Without the 100% rule this check would fire on every optional field in the
    project and be turned off within a day.
    """
    project = _project()
    for level in project["levels"]:
        for layer in level["layerInstances"]:
            for entity in layer["entityInstances"]:
                entity["fieldInstances"].append(
                    {"__identifier": "mounted_on", "__value": None}
                )
    assert check_issues(project, None) == []


def test_an_enum_value_the_enum_cannot_spell_is_an_error() -> None:
    project = _project()
    project["levels"][0]["layerInstances"][0]["entityInstances"][0]["fieldInstances"][0][
        "__value"
    ] = "snakes_on_a_plane"

    issues = check_issues(project, None)
    codes = [issue.code for issue in issues]
    assert "vocabulary.value_outside_enum" in codes
    bad = next(i for i in issues if i.code == "vocabulary.value_outside_enum")
    assert bad.severity == "error"
    assert "slop | snake" in (bad.fix_hint or "")


def test_listing_reports_placeable_layers_and_the_value_census(tmp_path) -> None:
    rows = build_listing(_project(), tmp_path / "world.ldtk", None)
    by_id = {row["identifier"]: row for row in rows}

    # A tag-filtered layer answers "where may I put one", not "where is one".
    assert by_id["EnemySpawn"]["layers"] == ["Ambition"]
    assert by_id["CameraZone"]["layers"] == ["AmbitionCameras"]

    brain = next(f for f in by_id["EnemySpawn"]["fields"] if f["name"] == "brain")
    assert brain["enum"] == ["slop", "snake"]
    # The census is the discoverable source of truth for a `String` field whose
    # legal values live in a Rust parser.
    character_id = next(
        f for f in by_id["EnemySpawn"]["fields"] if f["name"] == "character_id"
    )
    assert character_id["authored_by"] == 3
    assert character_id["observed"] == {"ai_slop": 2, "solid_snake": 1}


def test_listing_does_not_mutate_the_project(tmp_path) -> None:
    project = _project()
    before = copy.deepcopy(project)
    build_listing(project, tmp_path / "world.ldtk", None)
    check_issues(project, None)
    assert project == before
