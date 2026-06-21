from __future__ import annotations

import json
from pathlib import Path

from ambition_ldtk_tools.edit.world_layout import Point, auto_layout, render_layout_svg


def field(identifier: str, value):
    return {"__identifier": identifier, "__value": value}


def loading_zone(iid: str, px: tuple[int, int], size: tuple[int, int], **fields):
    return {
        "__identifier": "LoadingZone",
        "iid": iid,
        "px": [px[0], px[1]],
        "width": size[0],
        "height": size[1],
        "__worldX": 0,
        "__worldY": 0,
        "fieldInstances": [field(k, v) for k, v in fields.items()],
    }


def level(
    identifier: str,
    active_area: str,
    world: tuple[int, int],
    size: tuple[int, int],
    entities: list[dict] | None = None,
):
    wx, wy = world
    out = {
        "identifier": identifier,
        "iid": f"iid-{identifier}",
        "uid": abs(hash(identifier)) % 10_000_000,
        "worldX": wx,
        "worldY": wy,
        "pxWid": size[0],
        "pxHei": size[1],
        "fieldInstances": [field("activeArea", active_area)],
        "layerInstances": [
            {
                "__identifier": "Entities",
                "__type": "Entities",
                "entityInstances": entities or [],
            }
        ],
    }
    for ent in out["layerInstances"][0]["entityInstances"]:
        ent["__worldX"] = wx + ent["px"][0]
        ent["__worldY"] = wy + ent["px"][1]
    return out


def mini_project():
    hub_basement_entities = [
        loading_zone(
            "lz-a",
            (100, 430),
            (48, 48),
            id="door_a",
            activation="Door",
            target_room="child_a",
            target_zone="entry",
        ),
        loading_zone(
            "lz-b",
            (700, 430),
            (48, 48),
            id="door_b",
            activation="Door",
            target_room="child_b",
            target_zone="entry",
        ),
    ]
    child_a_entities = [
        loading_zone(
            "entry-a",
            (0, 210),
            (48, 48),
            id="entry",
            activation="Door",
            target_room="hub_complex",
            target_zone="door_a",
        )
    ]
    child_b_entities = [
        loading_zone(
            "entry-b",
            (0, 210),
            (48, 48),
            id="entry",
            activation="Door",
            target_room="hub_complex",
            target_zone="door_b",
        )
    ]
    return {
        "worldLayout": "Free",
        "worldGridWidth": 64,
        "worldGridHeight": 64,
        "levels": [
            level("hub_main", "hub_complex", (1000, 500), (1000, 500)),
            level("hub_basement", "hub_complex", (1000, 1000), (1000, 500), hub_basement_entities),
            level("child_a", "child_a", (9000, 0), (400, 300), child_a_entities),
            level("child_b", "child_b", (-9000, 0), (400, 300), child_b_entities),
        ],
    }


def by_id(project: dict, identifier: str) -> dict:
    return next(level for level in project["levels"] if level["identifier"] == identifier)


def test_auto_layout_preserves_active_area_as_rigid_group() -> None:
    project = mini_project()
    result = auto_layout(project, start="hub_main", origin=Point(0, 0), grid=64, gap=128)

    hub_main = by_id(project, "hub_main")
    hub_basement = by_id(project, "hub_basement")
    assert (hub_main["worldX"], hub_main["worldY"]) == (0, 0)
    # Basement moved with hub_main, preserving the original 500px intra-group offset.
    assert (hub_basement["worldX"], hub_basement["worldY"]) == (0, 500)
    assert result.moved_levels == 4


def test_auto_layout_places_floor_row_targets_below_source_group() -> None:
    project = mini_project()
    auto_layout(project, start="hub_main", origin=Point(0, 0), grid=64, gap=128)

    child_a = by_id(project, "child_a")
    child_b = by_id(project, "child_b")
    assert child_a["worldY"] >= 1000
    assert child_b["worldY"] >= 1000
    # Door B is farther right than door A; the packed editor layout should keep
    # that coarse ordering when both targets are below the same source group.
    assert child_b["worldX"] > child_a["worldX"]


def test_auto_layout_updates_entity_world_coords() -> None:
    project = mini_project()
    auto_layout(project, start="hub_main", origin=Point(0, 0), grid=64, gap=128)
    child_a = by_id(project, "child_a")
    entry = child_a["layerInstances"][0]["entityInstances"][0]
    assert entry["__worldX"] == child_a["worldX"] + entry["px"][0]
    assert entry["__worldY"] == child_a["worldY"] + entry["px"][1]





def test_auto_layout_padding_is_reported_and_used() -> None:
    project = mini_project()
    result = auto_layout(project, start="hub_main", origin=Point(0, 0), grid=64, gap=128, padding=96)
    assert result.packing_padding == 96
    assert "padding=96" in result.report


def test_auto_layout_cli_lock_keeps_group_position() -> None:
    project = mini_project()
    before = by_id(project, "child_b")["worldX"], by_id(project, "child_b")["worldY"]
    result = auto_layout(
        project,
        start="hub_main",
        origin=Point(0, 0),
        grid=64,
        gap=128,
        lock=["child_b"],
    )
    child_b = by_id(project, "child_b")
    assert (child_b["worldX"], child_b["worldY"]) == before
    assert "child_b" in result.locked_groups
    assert "child_b" in result.report and "locked" in result.report


def test_auto_layout_level_field_lock_keeps_group_position() -> None:
    project = mini_project()
    by_id(project, "child_a")["fieldInstances"].append(field("layoutLocked", True))
    before = by_id(project, "child_a")["worldX"], by_id(project, "child_a")["worldY"]
    result = auto_layout(project, start="hub_main", origin=Point(0, 0), grid=64, gap=128)
    child_a = by_id(project, "child_a")
    assert (child_a["worldX"], child_a["worldY"]) == before
    assert "child_a" in result.locked_groups


def test_auto_layout_svg_preview_contains_groups_and_links() -> None:
    project = mini_project()
    result = auto_layout(project, start="hub_main", origin=Point(0, 0), grid=64, gap=128)
    svg = render_layout_svg(result)
    assert svg.startswith('<svg xmlns="http://www.w3.org/2000/svg"')
    assert "hub_complex" in svg
    assert "child_a" in svg
    assert "<line" in svg

def test_world_layout_cli_dry_run(tmp_path: Path, capsys) -> None:
    path = tmp_path / "mini.ldtk"
    original = mini_project()
    path.write_text(json.dumps(original))

    from ambition_ldtk_tools.edit import world_layout

    rc = world_layout.main(["auto-layout", str(path), "--start", "hub_main", "--dry-run"])
    captured = capsys.readouterr()
    assert rc == 0
    assert "LDtk world auto-layout report" in captured.out
    # Dry run must not mutate the file.
    assert json.loads(path.read_text())["levels"][0]["worldX"] == 1000



def test_world_layout_cli_dry_run_writes_svg_report(tmp_path: Path, capsys) -> None:
    path = tmp_path / "mini.ldtk"
    svg = tmp_path / "preview.svg"
    original = mini_project()
    path.write_text(json.dumps(original))

    from ambition_ldtk_tools.edit import world_layout

    rc = world_layout.main([
        "auto-layout",
        str(path),
        "--start",
        "hub_main",
        "--dry-run",
        "--svg-report",
        str(svg),
        "--padding",
        "96",
    ])
    captured = capsys.readouterr()
    assert rc == 0
    assert "wrote svg report" in captured.out
    assert svg.exists()
    assert "padding=96" in captured.out
    assert "hub_complex" in svg.read_text()
    # Dry run still must not mutate the file.
    assert json.loads(path.read_text())["levels"][0]["worldX"] == 1000
