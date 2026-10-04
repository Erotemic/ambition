"""The semantic LDtk diff separates authored changes from editor noise."""

from __future__ import annotations

import copy
import json
import subprocess
from pathlib import Path

import pytest

from ambition_ldtk_tools.edit.ldtk_canonical import canonical_project, dumps_canonical
from ambition_ldtk_tools.edit.semantic_diff import compare, main, report_json

W, H, GRID = 8, 6, 16


def field_def(identifier: str, uid: int, typ: str, default=None) -> dict:
    return {
        "identifier": identifier,
        "uid": uid,
        "__type": typ,
        "type": f"F_{typ}",
        "isArray": False,
        "canBeNull": True,
        "defaultOverride": None if default is None else {"id": f"V_{typ}", "params": [default]},
        "editorDisplayColor": None,
    }


def field(identifier: str, typ: str, value, def_uid: int) -> dict:
    return {"__identifier": identifier, "__type": typ, "__value": value, "__tile": None, "defUid": def_uid, "realEditorValues": []}


def entity(identifier: str, iid: str, px, fields: list[dict], def_uid: int) -> dict:
    return {
        "__identifier": identifier,
        "__grid": [px[0] // GRID, px[1] // GRID],
        "__pivot": [0, 0],
        "__tags": [],
        "__tile": None,
        "__smartColor": "#ffffff",
        "__worldX": px[0],
        "__worldY": px[1],
        "iid": iid,
        "width": 16,
        "height": 16,
        "defUid": def_uid,
        "px": list(px),
        "fieldInstances": fields,
    }


def level(identifier: str, uid: int, iid: str, world_x: int, walls: list[tuple[int, int]], ents: list[dict]) -> dict:
    csv = [0] * (W * H)
    for x, y in walls:
        csv[y * W + x] = 1
    return {
        "identifier": identifier,
        "iid": iid,
        "uid": uid,
        "worldX": world_x,
        "worldY": 0,
        "worldDepth": 0,
        "pxWid": W * GRID,
        "pxHei": H * GRID,
        "__bgColor": "#000000",
        "bgColor": None,
        "__neighbours": [],
        "fieldInstances": [field("music_track", "String", "calm", 900)],
        "layerInstances": [
            {
                "__identifier": "Entities",
                "__type": "Entities",
                "__cWid": W,
                "__cHei": H,
                "__gridSize": GRID,
                "iid": f"{iid}-ents",
                "levelId": uid,
                "layerDefUid": 100,
                "seed": 1,
                "visible": True,
                "pxOffsetX": 0,
                "pxOffsetY": 0,
                "intGridCsv": [],
                "gridTiles": [],
                "autoLayerTiles": [],
                "entityInstances": ents,
            },
            {
                "__identifier": "Collision",
                "__type": "IntGrid",
                "__cWid": W,
                "__cHei": H,
                "__gridSize": GRID,
                "iid": f"{iid}-col",
                "levelId": uid,
                "layerDefUid": 101,
                "seed": 2,
                "visible": True,
                "pxOffsetX": 0,
                "pxOffsetY": 0,
                "intGridCsv": csv,
                "gridTiles": [],
                "autoLayerTiles": [],
                "entityInstances": [],
            },
            {
                "__identifier": "Art",
                "__type": "Tiles",
                "__cWid": W,
                "__cHei": H,
                "__gridSize": GRID,
                "iid": f"{iid}-art",
                "levelId": uid,
                "layerDefUid": 102,
                "seed": 3,
                "visible": True,
                "pxOffsetX": 0,
                "pxOffsetY": 0,
                "intGridCsv": [],
                "gridTiles": [
                    {"px": [0, 80], "src": [0, 0], "f": 0, "t": 0, "d": [40], "a": 1},
                    {"px": [16, 80], "src": [16, 0], "f": 0, "t": 1, "d": [41], "a": 1},
                ],
                "autoLayerTiles": [],
                "entityInstances": [],
            },
        ],
    }


def project() -> dict:
    """Two rooms. Room A has a door with an id, two crates, and a switch that refers to a crate."""
    door = entity("Door", "door-iid", (32, 64), [field("id", "String", "door_a", 300), field("target", "String", "room_b", 301)], 30)
    crate1 = entity("Crate", "crate1-iid", (64, 64), [field("tint", "String", "red", 310)], 31)
    crate2 = entity("Crate", "crate2-iid", (96, 64), [field("tint", "String", "blue", 310)], 31)
    switch = entity("Switch", "switch-iid", (16, 32), [
        field("target", "EntityRef", {"entityIid": "crate2-iid", "layerIid": "a-ents", "levelIid": "a", "worldIid": "w"}, 320),
        field("mode", "String", None, 321),
    ], 32)
    return {
        "__header__": {"app": "LDtk", "appVersion": "1.5.3"},
        "iid": "project-iid",
        "jsonVersion": "1.5.3",
        "appBuildId": 473703,
        "nextUid": 1000,
        "worldLayout": "Free",
        "worldGridWidth": 256,
        "worldGridHeight": 256,
        "externalLevels": False,
        "toc": [],
        "worlds": [],
        "defs": {
            "layers": [
                {"identifier": "Entities", "uid": 100, "type": "Entities", "__type": "Entities", "gridSize": GRID, "uiColor": "#fff", "intGridValues": [], "autoRuleGroups": [], "tilesetDefUid": None},
                {"identifier": "Collision", "uid": 101, "type": "IntGrid", "__type": "IntGrid", "gridSize": GRID, "uiColor": "#fff",
                 "intGridValues": [{"value": 1, "identifier": "wall", "color": "#000"}], "autoRuleGroups": [], "tilesetDefUid": None},
                {"identifier": "Art", "uid": 102, "type": "Tiles", "__type": "Tiles", "gridSize": GRID, "uiColor": "#fff", "intGridValues": [], "autoRuleGroups": [], "tilesetDefUid": 200},
            ],
            "entities": [
                {"identifier": "Door", "uid": 30, "width": 16, "height": 16, "color": "#f00", "renderMode": "Rectangle", "tilesetId": None, "tileRect": None,
                 "fieldDefs": [field_def("id", 300, "String"), field_def("target", 301, "String")]},
                {"identifier": "Crate", "uid": 31, "width": 16, "height": 16, "color": "#0f0", "renderMode": "Rectangle", "tilesetId": None, "tileRect": None,
                 "fieldDefs": [field_def("tint", 310, "String", "red")]},
                {"identifier": "Switch", "uid": 32, "width": 16, "height": 16, "color": "#00f", "renderMode": "Rectangle", "tilesetId": None, "tileRect": None,
                 "fieldDefs": [field_def("target", 320, "EntityRef"), field_def("mode", 321, "String")]},
            ],
            "tilesets": [{"identifier": "Terrain", "uid": 200, "relPath": "terrain.png", "pxWid": 64, "pxHei": 64, "tileGridSize": 16, "cachedPixelData": None, "savedSelections": []}],
            "enums": [],
            "externalEnums": [],
            "levelFields": [field_def("music_track", 900, "String")],
        },
        "levels": [
            level("room_a", 1, "a", 0, [(0, 5), (1, 5), (2, 5)], [door, crate1, crate2, switch]),
            level("room_b", 2, "b", 256, [(3, 5)], []),
        ],
    }


def churn_ids(p: dict) -> dict:
    """Renumber every uid and iid the way an editor or a generator rerun does."""
    out = copy.deepcopy(p)

    # Uids: shift by 5000 wherever they appear as a uid value or uid reference.
    def walk(node):
        if isinstance(node, dict):
            for key, value in list(node.items()):
                if isinstance(value, int) and not isinstance(value, bool) and (key in {"uid", "defUid", "levelId", "nextUid", "tilesetId"} or key.endswith("Uid")):
                    node[key] = value + 5000
                elif isinstance(value, str) and (key in {"iid", "entityIid", "layerIid", "levelIid"}):
                    node[key] = "new-" + value
                else:
                    walk(value)
        elif isinstance(node, list):
            for value in node:
                walk(value)
    walk(out)
    out["appBuildId"] = 999999
    for lv in out["levels"]:
        for layer in lv["layerInstances"]:
            layer["seed"] += 7
            for tile in layer["gridTiles"]:
                tile["d"] = [tile["d"][0] + 1]
    return out


def kinds(report) -> set[str]:
    return {c.kind for c in report.changes}


# ---------------------------------------------------------------------------
# Noise only
# ---------------------------------------------------------------------------


def test_identical_projects_are_identical() -> None:
    report = compare(project(), project())
    assert report.verdict == "identical"
    assert not report.changes and not report.ambiguities


def test_pure_id_churn_is_not_a_semantic_change() -> None:
    report = compare(project(), churn_ids(project()))
    assert report.changes == [], [c.detail for c in report.changes]
    assert report.ambiguities == []
    assert report.verdict == "noise_only"
    assert report.noise.counts["uid_renumbered"] > 0
    assert report.noise.counts["iid_renumbered"] >= 4
    assert report.noise.counts["editor_metadata"] > 0


def test_reordered_entities_levels_and_tiles_are_not_a_semantic_change() -> None:
    after = project()
    ents = after["levels"][0]["layerInstances"][0]["entityInstances"]
    ents.reverse()
    ents[0]["fieldInstances"].reverse()
    after["levels"].reverse()
    after["levels"][1]["layerInstances"][2]["gridTiles"].reverse()
    after["defs"]["entities"].reverse()
    report = compare(project(), after)
    assert report.changes == []
    assert report.verdict == "noise_only"
    assert report.noise.counts["reordered"] >= 4


def test_null_field_and_absent_field_are_equal() -> None:
    after = project()
    switch = after["levels"][0]["layerInstances"][0]["entityInstances"][3]
    switch["fieldInstances"] = [f for f in switch["fieldInstances"] if f["__identifier"] != "mode"]
    report = compare(project(), after)
    assert report.changes == []
    assert report.noise.counts["null_field_omitted"] == 1


# ---------------------------------------------------------------------------
# Authored changes
# ---------------------------------------------------------------------------


def test_moved_wall_is_a_geometry_change_with_cells() -> None:
    after = churn_ids(project())
    csv = after["levels"][0]["layerInstances"][1]["intGridCsv"]
    csv[5 * W + 2] = 0
    csv[4 * W + 2] = 1
    report = compare(project(), after)
    assert kinds(report) == {"intgrid"}
    change = report.changes[0]
    assert change.level == "room_a"
    assert change.data["cells"] == [[2, 4, 0, 1], [2, 5, 1, 0]]
    assert change.data["transitions"] == {"0->1": 1, "1->0": 1}
    row = report_json(report)["levels"]["room_a"]
    assert row["geometry"] == "changed" and row["entities"] == "unchanged"


def test_moved_tile_is_a_tile_change() -> None:
    after = project()
    after["levels"][0]["layerInstances"][2]["gridTiles"][1]["px"] = [32, 80]
    report = compare(project(), after)
    assert kinds(report) == {"tiles"}
    assert report.changes[0].data["added"] == 1 and report.changes[0].data["removed"] == 1


def test_changed_entity_field_is_a_semantic_change() -> None:
    after = churn_ids(project())
    door = after["levels"][0]["layerInstances"][0]["entityInstances"][0]
    door["fieldInstances"][1]["__value"] = "room_c"
    report = compare(project(), after)
    assert kinds(report) == {"entity_field"}
    change = report.changes[0]
    assert (change.before, change.after) == ("room_b", "room_c")
    assert "paired by authored id" in change.detail


def test_field_written_with_its_default_is_reported() -> None:
    # The runtime does not read definition defaults, so absent and "red" can differ.
    before = project()
    crate = before["levels"][0]["layerInstances"][0]["entityInstances"][1]
    crate["fieldInstances"] = []
    report = compare(before, project())
    assert kinds(report) == {"entity_field"}
    assert "definition default" in report.changes[0].detail


def test_added_and_removed_room_are_semantic_changes() -> None:
    after = project()
    removed = after["levels"].pop(1)
    added = copy.deepcopy(removed)
    added["identifier"] = "room_c"
    added["layerInstances"][1]["intGridCsv"][0] = 1
    after["levels"].append(added)
    report = compare(project(), after)
    assert {"level_added", "level_removed"} <= kinds(report)
    assert report_json(report)["levels"]["room_c"]["status"] == "added"


def test_renamed_room_with_same_content_is_a_rename() -> None:
    after = churn_ids(project())
    after["levels"][1]["identifier"] = "room_renamed"
    report = compare(project(), after)
    assert kinds(report) == {"level_renamed"}


def test_moved_ref_target_does_not_change_the_ref() -> None:
    after = churn_ids(project())
    after["levels"][0]["layerInstances"][0]["entityInstances"][2]["px"] = [112, 64]
    report = compare(project(), after)
    assert kinds(report) == {"entity_moved"}


def test_ref_pointed_at_another_entity_is_a_change() -> None:
    after = project()
    switch = after["levels"][0]["layerInstances"][0]["entityInstances"][3]
    switch["fieldInstances"][0]["__value"]["entityIid"] = "crate1-iid"
    report = compare(project(), after)
    assert kinds(report) == {"entity_field"}
    assert report.changes[0].after == {"ref": "room_a/Entities/Crate@64,64"}


def test_definition_change_ignores_uid_and_display_noise() -> None:
    after = churn_ids(project())
    after["defs"]["entities"][1]["color"] = "#123456"
    after["defs"]["entities"][1]["width"] = 32
    report = compare(project(), after)
    assert kinds(report) == {"entity_def_changed"}
    assert report.changes[0].tier == "definition"
    assert report.noise.counts["editor_display"] >= 1


def test_unknown_key_is_reported_not_ignored() -> None:
    after = project()
    after["levels"][0]["layerInstances"][0]["entityInstances"][0]["newEditorThing"] = 3
    report = compare(project(), after)
    assert kinds(report) == {"unknown_key"}


# ---------------------------------------------------------------------------
# Ambiguity
# ---------------------------------------------------------------------------


def test_unpairable_entities_are_ambiguous_not_equal() -> None:
    """Two crates moved and retinted with churned iids: no stable key pairs them."""
    after = churn_ids(project())
    ents = after["levels"][0]["layerInstances"][0]["entityInstances"]
    ents[1]["px"], ents[1]["fieldInstances"][0]["__value"] = [16, 16], "green"
    ents[2]["px"], ents[2]["fieldInstances"][0]["__value"] = [48, 16], "green"
    report = compare(project(), after)
    assert report.verdict == "changed"
    assert [a.kind for a in report.ambiguities] == ["entity_unpaired"]
    amb = report.ambiguities[0]
    assert amb.before == ["room_a/Entities/Crate@64,64", "room_a/Entities/Crate@96,64"]
    assert amb.after == ["room_a/Entities/Crate@16,16", "room_a/Entities/Crate@48,16"]
    assert report_json(report)["levels"]["room_a"]["entities"] == "ambiguous"


def test_iid_pairs_only_when_iids_are_stable() -> None:
    """With stable iids the same edit pairs by iid and reports two moves."""
    after = project()
    ents = after["levels"][0]["layerInstances"][0]["entityInstances"]
    ents[1]["px"], ents[1]["fieldInstances"][0]["__value"] = [16, 16], "green"
    ents[2]["px"], ents[2]["fieldInstances"][0]["__value"] = [48, 16], "green"
    report = compare(project(), after)
    assert report.ambiguities == []
    assert {"entity_moved", "entity_field"} == kinds(report)


# ---------------------------------------------------------------------------
# Normalize, report stability, CLI
# ---------------------------------------------------------------------------


def test_canonical_form_ignores_id_churn_and_order() -> None:
    after = churn_ids(project())
    after["levels"].reverse()
    after["levels"][1]["layerInstances"][0]["entityInstances"].reverse()
    assert dumps_canonical(canonical_project(project())) == dumps_canonical(canonical_project(after))


def test_json_report_is_deterministic() -> None:
    after = churn_ids(project())
    after["levels"][0]["layerInstances"][1]["intGridCsv"][0] = 1
    one = json.dumps(report_json(compare(project(), after)), sort_keys=True)
    two = json.dumps(report_json(compare(project(), copy.deepcopy(after))), sort_keys=True)
    assert one == two


def test_cli_exit_codes_and_normalize_refuses_ldtk_output(tmp_path: Path, capsys) -> None:
    before = tmp_path / "before.ldtk"
    churned = tmp_path / "churned.ldtk"
    edited = tmp_path / "edited.ldtk"
    before.write_text(json.dumps(project()))
    churned.write_text(json.dumps(churn_ids(project()), indent=2))
    moved = churn_ids(project())
    moved["levels"][0]["worldX"] = 512
    edited.write_text(json.dumps(moved))
    assert main(["semantic", str(before), str(churned)]) == 0
    assert "NOISE ONLY" in capsys.readouterr().out
    assert main(["semantic", str(before), str(edited), "--format", "json"]) == 1
    data = json.loads(capsys.readouterr().out)
    assert data["verdict"] == "changed" and [c["kind"] for c in data["changes"]] == ["level_moved"]
    assert main(["normalize", str(before), "--out", str(tmp_path / "x.ldtk")]) == 2
    original = before.read_text()
    assert main(["normalize", str(before), "--out", str(tmp_path / "canon.json")]) == 0
    assert before.read_text() == original
    assert json.loads((tmp_path / "canon.json").read_text())["schema"] == "ambition-ldtk-canonical/1"


def test_git_revision_input(tmp_path: Path, capsys, monkeypatch) -> None:
    if subprocess.run(["git", "--version"], capture_output=True).returncode != 0:
        pytest.skip("git is not available")
    repo = tmp_path / "repo"
    repo.mkdir()
    world = repo / "world.ldtk"

    def git(*argv: str) -> None:
        subprocess.run(["git", "-C", str(repo), "-c", "user.name=t", "-c", "user.email=t@t", *argv], check=True, capture_output=True)

    git("init", "-q")
    world.write_text(json.dumps(project()))
    git("add", "world.ldtk")
    git("commit", "-q", "-m", "one")
    world.write_text(json.dumps(churn_ids(project())))
    git("commit", "-q", "-am", "churn")
    monkeypatch.chdir(repo)
    assert main(["semantic", "HEAD~1:world.ldtk", "world.ldtk"]) == 0
    assert main(["range", "HEAD~1..HEAD"]) == 0
    assert "NOISE ONLY" in capsys.readouterr().out
