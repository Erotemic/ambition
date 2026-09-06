from pathlib import Path

from ambition_ldtk_tools.area.spec import load_spec
from ambition_ldtk_tools.edit.layout.model import GroupInfo, LayoutResult, Point, Rect
from ambition_ldtk_tools.ldtk.issues import Issue
from ambition_ldtk_tools.room_support.issues import room_issues
from ambition_ldtk_tools.validate import validate_issues


def test_validate_issues_wraps_parse_errors(tmp_path: Path):
    path = tmp_path / "broken.ldtk"
    path.write_text("not json")
    issues = validate_issues(path)
    assert len(issues) == 1
    assert issues[0].severity == "error"
    assert issues[0].code == "validate.json.parse"
    assert "failed to parse JSON" in issues[0].message


def test_room_issues_emit_shared_issue_shape():
    level = {"identifier": "symmetry_room", "pxWid": 128, "pxHei": 128}
    entities = [
        {
            "identifier": "CameraZone",
            "iid": "cam-1",
            "layer": "Ambition",
            "px": [0, 0],
            "size": [128, 128],
            "fields": {},
        }
    ]
    issues = room_issues(level, {"Collision": {"values": {"1": {"count": 1}}}}, entities)
    assert all(isinstance(issue, Issue) for issue in issues)
    wrong_layer = [issue for issue in issues if issue.code == "room.camera_zone.wrong_layer"]
    assert wrong_layer
    assert wrong_layer[0].fixable is True
    assert wrong_layer[0].as_dict()["entity"] == "CameraZone"


def test_layout_model_import_seam():
    group = GroupInfo(id="hub", rect=Rect(0, 0, 64, 32), anchor=Point(0, 0))
    result = LayoutResult(
        placements={"hub": Point(16, 32)},
        groups={"hub": group},
        edges=[],
        unresolved_edges=[],
        moved_levels=0,
        updated_entities=0,
        report="ok",
    )
    assert result.groups["hub"].rect.center == Point(32, 16)
    assert result.placements["hub"] + Point(1, 2) == Point(17, 34)


def test_area_spec_loader_rejects_yaml(tmp_path: Path):
    path = tmp_path / "area.yaml"
    path.write_text("id: demo\n")
    try:
        load_spec(path)
    except SystemExit as ex:
        assert "YAML area specs are no longer supported" in str(ex)
    else:
        raise AssertionError("expected yaml specs to be rejected")


def test_area_patch_plan_applies_named_ops():
    from ambition_ldtk_tools.area.plan import AreaPatchPlan, CallableAreaPatchOp

    project = {"levels": []}
    plan = AreaPatchPlan(area_id="demo", level_identifier="demo_level")
    plan.add_op(
        CallableAreaPatchOp(
            "append demo level",
            lambda p: p["levels"].append({"identifier": "demo_level"}) or ["added demo_level"],
        )
    )
    assert plan.apply(project) == ["added demo_level"]
    assert project["levels"][0]["identifier"] == "demo_level"


def test_authoring_hygiene_rule_module_emits_first_class_issue_codes():
    from ambition_ldtk_tools.validate_rules.authoring_hygiene import authoring_hygiene_issues

    project = {
        "levels": [
            {
                "identifier": "tiny_room",
                "pxWid": 64,
                "pxHei": 64,
                "layerInstances": [
                    {
                        "__identifier": "Ambition",
                        "entityInstances": [
                            {"__identifier": "DebugLabel", "iid": "label-a", "px": [0, 0], "width": 32, "height": 16},
                            {"__identifier": "DebugLabel", "iid": "label-b", "px": [8, 0], "width": 32, "height": 16},
                        ],
                    }
                ],
            }
        ]
    }

    codes = {issue.code for issue in authoring_hygiene_issues(project)}
    assert "validate.debug_label_overlap" in codes
    assert "validate.missing_level_wall" in codes


def test_area_create_plan_uses_first_class_ops():
    from ambition_ldtk_tools.area.plan import (
        AddReciprocalLoadingZonesOp,
        AppendGeneratedLevelOp,
        ReplaceExistingLevelOp,
    )
    from ambition_ldtk_tools.area_authoring import compile_area_create_plan

    project = {
        "jsonVersion": "1.5.3",
        "defaultGridSize": 16,
        "defaultLevelBgColor": "#000000",
        "nextUid": 1,
        "defs": {
            "layers": [
                {"identifier": "Collision", "uid": 10, "__type": "IntGrid", "gridSize": 16},
                {"identifier": "Ambition", "uid": 11, "__type": "Entities", "gridSize": 16},
            ],
            "entities": [],
            "levelFields": [
                {"identifier": "activeArea", "uid": 20, "__type": "String", "type": "F_String"},
            ],
        },
        "levels": [],
    }
    spec = {
        "id": "tiny_area",
        "level_id": "tiny_area",
        "world_x": 0,
        "world_y": 0,
        "px_wid": 64,
        "px_hei": 64,
        "entities": [],
    }

    plan, _level, _summaries = compile_area_create_plan(project, spec)
    assert any(isinstance(op, AppendGeneratedLevelOp) for op in plan.ops)
    assert not any(isinstance(op, ReplaceExistingLevelOp) for op in plan.ops)
    assert not any(isinstance(op, AddReciprocalLoadingZonesOp) for op in plan.ops)
