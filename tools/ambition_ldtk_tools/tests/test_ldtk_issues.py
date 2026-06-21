from __future__ import annotations

import json

from ambition_ldtk_tools.ldtk import Issue, format_issue_lines, has_errors


def test_issue_location_and_json_shape_are_stable() -> None:
    issue = Issue(
        "error",
        "entity_wrong_layer",
        "is on Ambition, expected AmbitionCameras",
        level="symmetry_room",
        layer="Ambition",
        entity="CameraZone",
        entity_iid="CameraZone-1",
        fixable=True,
        fix_hint="move CameraZone to AmbitionCameras",
        data={"expected_layer": "AmbitionCameras"},
    )

    assert issue.location == "symmetry_room / Ambition / CameraZone CameraZone-1"
    payload = issue.as_dict()
    assert payload["code"] == "entity_wrong_layer"
    assert payload["level"] == "symmetry_room"
    assert payload["fixable"] is True
    assert "fix_hint" in payload
    assert json.loads(json.dumps(payload))["data"]["expected_layer"] == "AmbitionCameras"


def test_issue_text_format_and_error_detection() -> None:
    warnings = [Issue("warning", "camera_missing", "no CameraZone found", level="room_a")]
    assert not has_errors(warnings)
    text = format_issue_lines(warnings, title="Issues:", empty="none")
    assert "warning: camera_missing: room_a: no CameraZone found" in text

    assert has_errors([Issue("error", "bad", "problem")])
    assert format_issue_lines([], title="Issues:", empty="none") == "none\n"
