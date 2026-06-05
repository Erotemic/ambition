"""Pin tests for the pure helpers inside `publish_catalog_sprites` —
the catalog-driven renderer publish driver. The end-to-end flow
shells out to the renderer (full sprite generation), so this file
only covers what can be tested without an actual render."""

from __future__ import annotations

import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(REPO_ROOT / "tools" / "ambition_ldtk_tools"))

from ambition_ldtk_tools.publish_catalog_sprites import (  # noqa: E402
    renderer_target_for_catalog_entry,
)


def test_top_level_sprite_strips_prefix_and_suffix():
    """`sprites/architect_spritesheet.png` → renderer target `architect`."""
    assert (
        renderer_target_for_catalog_entry("sprites/architect_spritesheet.png")
        == "architect"
    )


def test_subdir_sprite_returns_target_name():
    """Multi-file subdir tack-on targets (gnu_ton_boss,
    mockingbird_boss) DO publish via the unified `publish <target>`
    path now — their `install()` function lays files into
    `<dest>/<target>/`. The driver returns the target name and
    `is_subdir_target` carries the layout hint."""
    from ambition_ldtk_tools.publish_catalog_sprites import is_subdir_target

    target = renderer_target_for_catalog_entry(
        "sprites/gnu_ton_boss/gnu_ton_boss_spritesheet.png"
    )
    assert target == "gnu_ton_boss"
    assert is_subdir_target(target, "sprites/gnu_ton_boss/gnu_ton_boss_spritesheet.png")

    target = renderer_target_for_catalog_entry(
        "sprites/mockingbird_boss/mockingbird_boss_spritesheet.png"
    )
    assert target == "mockingbird_boss"
    assert is_subdir_target(
        target, "sprites/mockingbird_boss/mockingbird_boss_spritesheet.png"
    )


def test_underscore_target_names_round_trip():
    """Multi-word target names keep their underscores; the loop
    strips only the trailing `_spritesheet.png` suffix."""
    assert (
        renderer_target_for_catalog_entry("sprites/player_robot_spritesheet.png")
        == "player_robot"
    )
    assert (
        renderer_target_for_catalog_entry(
            "sprites/pirate_heavy_iron_mary_spritesheet.png"
        )
        == "pirate_heavy_iron_mary"
    )


def test_missing_sprites_prefix_returns_none():
    """Defensively reject entries that aren't under `sprites/` —
    these would be configuration errors and the driver shouldn't
    silently mis-derive a target name."""
    assert renderer_target_for_catalog_entry("architect_spritesheet.png") is None
    assert (
        renderer_target_for_catalog_entry("assets/sprites/architect_spritesheet.png")
        is None
    )


def test_non_spritesheet_filename_returns_none():
    """Sentinel rejection for sheet paths that don't end in the
    canonical suffix. The catalog convention requires
    `<target>_spritesheet.png`."""
    assert renderer_target_for_catalog_entry("sprites/architect_canonical.png") is None
    assert renderer_target_for_catalog_entry("sprites/architect.png") is None


def test_expected_outputs_subdir_layout():
    """Subdir targets install into `<sprites_dir>/<target>/<files>`,
    so the post-publish coverage check must look in the subdir."""
    from ambition_ldtk_tools.publish_catalog_sprites import expected_outputs

    top = expected_outputs("architect", subdir=False)
    assert top == ["architect_spritesheet.png", "architect_spritesheet.ron"]

    sub = expected_outputs("mockingbird_boss", subdir=True)
    assert sub == [
        "mockingbird_boss/mockingbird_boss_spritesheet.png",
        "mockingbird_boss/mockingbird_boss_spritesheet.ron",
    ]
