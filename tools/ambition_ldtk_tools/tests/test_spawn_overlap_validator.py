"""Tests that authoring validation warns on overlapping spawn rectangles."""

from __future__ import annotations

import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(REPO_ROOT / "tools" / "ambition_ldtk_tools"))

from ambition_ldtk_tools.validate import _check_intro_authoring_hygiene  # noqa: E402


def make_spawn(iid: str, identifier: str, px: tuple[int, int], size: tuple[int, int]):
    return {
        "__identifier": identifier,
        "iid": iid,
        "px": list(px),
        "width": size[0],
        "height": size[1],
        "fieldInstances": [],
    }


def make_level(level_id: str, entities: list[dict]):
    return {
        "identifier": level_id,
        "pxWid": 4096,
        "pxHei": 2048,
        "fieldInstances": [],
        "layerInstances": [
            {
                "__identifier": "Ambition",
                "entityInstances": entities,
            },
            {
                "__identifier": "Collision",
                "__cWid": 256,
                "__cHei": 128,
                "__gridSize": 16,
                "intGridCsv": [0] * (256 * 128),
            },
        ],
    }


def test_spawn_overlap_warning_fires_on_overlapping_npc_spawns():
    project = {
        "levels": [
            make_level(
                "overlap_test_room",
                [
                    make_spawn("a", "NpcSpawn", (100, 100), (48, 80)),
                    # Overlaps the first by ~24 px.
                    make_spawn("b", "NpcSpawn", (124, 110), (48, 80)),
                ],
            )
        ]
    }
    warnings: list[str] = []
    _check_intro_authoring_hygiene(project, warnings)
    overlap_warnings = [
        w for w in warnings if "overlap" in w.lower() and "NpcSpawn" in w
    ]
    assert len(overlap_warnings) == 1, (
        f"expected one spawn-overlap warning; got {len(overlap_warnings)}: {overlap_warnings}"
    )


def test_spawn_overlap_warning_skips_far_apart_spawns():
    project = {
        "levels": [
            make_level(
                "spread_room",
                [
                    make_spawn("a", "NpcSpawn", (0, 0), (48, 80)),
                    make_spawn("b", "NpcSpawn", (2000, 1000), (48, 80)),
                ],
            )
        ]
    }
    warnings: list[str] = []
    _check_intro_authoring_hygiene(project, warnings)
    overlap_warnings = [
        w for w in warnings if "overlap" in w.lower() and "NpcSpawn" in w
    ]
    assert overlap_warnings == [], (
        f"spawn-overlap check fired on far-apart spawns: {overlap_warnings}"
    )


def test_spawn_overlap_warning_covers_mixed_spawn_kinds():
    # An NpcSpawn near an EnemySpawn should still flag — the validator
    # treats every spawn-identifier as belonging to the same overlap
    # pool.
    project = {
        "levels": [
            make_level(
                "mixed_room",
                [
                    make_spawn("a", "NpcSpawn", (100, 100), (48, 80)),
                    make_spawn("b", "EnemySpawn", (124, 110), (48, 80)),
                ],
            )
        ]
    }
    warnings: list[str] = []
    _check_intro_authoring_hygiene(project, warnings)
    overlap_warnings = [
        w
        for w in warnings
        if "overlap" in w.lower() and "NpcSpawn" in w and "EnemySpawn" in w
    ]
    assert len(overlap_warnings) == 1, (
        f"mixed spawn-kind overlap not flagged; got {overlap_warnings}"
    )


def make_mount_pair(rider_iid: str, mount_iid: str, px: tuple[int, int]):
    """A rider and its mount, authored at the SAME pixel — which is what a mount is."""
    rider = make_spawn(rider_iid, "EnemySpawn", px, (48, 80))
    rider["fieldInstances"] = [
        {
            "__identifier": "mounted_on",
            "__type": "EntityRef",
            "__value": {
                "entityIid": mount_iid,
                "layerIid": "Ambition-1",
                "levelIid": "mount_room-1",
                "worldIid": "test-world",
            },
        }
    ]
    return [rider, make_spawn(mount_iid, "EnemySpawn", px, (48, 80))]


def test_spawn_overlap_exempts_a_rider_sitting_on_its_mount():
    # The sky levels author every shark-riding pirate this way. Before the
    # exemption this fired on all seven of them, and that warning is what
    # talked a session into reaching for `entity delete` on the very content
    project = {
        "levels": [make_level("mount_room", make_mount_pair("rider", "shark", (192, 240)))]
    }
    warnings: list[str] = []
    _check_intro_authoring_hygiene(project, warnings)
    overlap_warnings = [w for w in warnings if "overlap" in w.lower()]
    assert overlap_warnings == [], (
        f"a rider on its mount was reported as an overlap: {overlap_warnings}"
    )


def test_spawn_overlap_still_fires_when_the_pair_names_nobody():
    # The exemption must key on the REFERENCE, not on the coincidence of two
    # spawns sharing a pixel — otherwise it silences the real duplicate it
    # exists to catch.
    project = {
        "levels": [
            make_level(
                "stacked_room",
                [
                    make_spawn("a", "EnemySpawn", (192, 240), (48, 80)),
                    make_spawn("b", "EnemySpawn", (192, 240), (48, 80)),
                ],
            )
        ]
    }
    warnings: list[str] = []
    _check_intro_authoring_hygiene(project, warnings)
    overlap_warnings = [w for w in warnings if "overlap" in w.lower()]
    assert len(overlap_warnings) == 1, (
        f"two unrelated spawns at one pixel should still warn; got {overlap_warnings}"
    )
