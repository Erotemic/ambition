#!/usr/bin/env python3
"""A `LoadingZone` is either an EXIT or a LANDING PAD, and the validator used to
allow only the first.

`ambition_ldtk_tools.validate` demanded `target_room` + `target_zone` on every
zone. The runtime has always had both shapes — `collect_room_links` skips a zone
that names no target, and `transition_from_zone` only fires on a zone with an
outgoing edge — so the arrival end of every one-way trip was unauthorable, and
Mary-O's two 1-1 zones had to stay in Rust because of it.

⚠ **a landing pad that names a target is a BOUNCE, not a harmless extra.** The
body arrives standing inside the target zone (`door_arrival` = zone centre, 26px
off its floor), so the moment the 0.16s transition cooldown lapses the zone it
landed on fires and sends it straight back. Measured against
`RoomSet::transition_for_player`, not reasoned about.

What the rule still catches is the typo it was written for: a zone with no
target that nothing arrives through is dead geometry, and an exit whose fields
were never filled in looks exactly like one.
"""

from __future__ import annotations

from test_edgeexit_validation import (  # noqa: E402
    _make_loading_zone,
    _make_level,
    _make_player_start,
    _load_real_project,
    _strip_to_defs,
    _write_and_validate,
)


def _unset_target(zone: dict) -> dict:
    """The shape LDtk writes for a String field the author left empty."""
    for field in zone["fieldInstances"]:
        if field["__identifier"] in {"target_room", "target_zone"}:
            field["__value"] = None
            field["realEditorValues"] = [None]
    return zone


def _one_way_project(*, pad_is_targeted: bool) -> dict:
    """`src_area` has an exit; `dst_area` has a landing pad that names nothing.

    When `pad_is_targeted` is False the exit points at a DIFFERENT zone id, so
    the pad is a zone nobody can ever arrive through.
    """
    project = _strip_to_defs(_load_real_project())
    src_entities = [
        _make_player_start(
            project,
            iid="PlayerStart-src",
            px=[64, 64],
            world_x=0,
            world_y=0,
            name="src_spawn",
        ),
        _make_loading_zone(
            project,
            iid="LoadingZone-src",
            zone_id="descent",
            px=[60, 80],
            size=[32, 48],
            activation="Walk",
            target_room="dst_area",
            target_zone="arrival" if pad_is_targeted else "some_other_zone",
            world_x=0,
            world_y=0,
        ),
    ]
    dst_entities = [
        _make_player_start(
            project,
            iid="PlayerStart-dst",
            px=[64, 64],
            world_x=200,
            world_y=0,
            name="dst_spawn",
        ),
        _unset_target(
            _make_loading_zone(
                project,
                iid="LoadingZone-dst",
                zone_id="arrival",
                px=[60, 80],
                size=[32, 48],
                activation="Walk",
                target_room="",
                target_zone="",
                world_x=200,
                world_y=0,
            )
        ),
    ]
    project["levels"] = [
        _make_level(
            project,
            identifier="src",
            uid=910_001,
            world_x=0,
            world_y=0,
            px_wid=200,
            px_hei=200,
            entities=src_entities,
        ),
        _make_level(
            project,
            identifier="dst",
            uid=910_002,
            world_x=200,
            world_y=0,
            px_wid=200,
            px_hei=200,
            entities=dst_entities,
        ),
    ]
    for level, area in zip(project["levels"], ("src_area", "dst_area")):
        level["fieldInstances"][0]["__value"] = area
        level["fieldInstances"][0]["realEditorValues"] = [
            {"id": "V_String", "params": [area]}
        ]
    return project


def test_a_landing_pad_that_names_no_target_validates():
    errors, _warnings = _write_and_validate(_one_way_project(pad_is_targeted=True))
    zone_errors = [e for e in errors if "LoadingZone" in e]
    assert not zone_errors, (
        "the arrival end of a one-way trip names no target on purpose; "
        f"validation rejected it: {zone_errors!r}"
    )


def test_a_landing_pad_nothing_arrives_through_is_still_refused():
    errors, _warnings = _write_and_validate(_one_way_project(pad_is_targeted=False))
    assert any(
        "nothing arrives through it" in e for e in errors
    ), f"a dead zone must still be an error, got {errors!r}"


def test_half_a_target_is_refused():
    project = _one_way_project(pad_is_targeted=True)
    # Fill in only `target_room` on the pad: the half-authored exit.
    pad = project["levels"][1]["layerInstances"][1]["entityInstances"][1]
    for field in pad["fieldInstances"]:
        if field["__identifier"] == "target_room":
            field["__value"] = "src_area"
            field["realEditorValues"] = [{"id": "V_String", "params": ["src_area"]}]
    errors, _warnings = _write_and_validate(project)
    assert any(
        "names half a target" in e for e in errors
    ), f"half an exit must not read as a landing pad, got {errors!r}"


if __name__ == "__main__":
    test_a_landing_pad_that_names_no_target_validates()
    test_a_landing_pad_nothing_arrives_through_is_still_refused()
    test_half_a_target_is_refused()
    print("LoadingZone landing-pad tests: PASS")
