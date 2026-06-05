"""Smoke tests for `codegen_character_catalog` — the one-shot script
that synthesizes catalog entries from the renderer's `list-targets`
output (used during Phase 3 of the character-catalog refactor).

These tests focus on pure helper functions (no subprocess execution
of the renderer) so they're fast and self-contained."""

from __future__ import annotations

import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(REPO_ROOT / "tools" / "ambition_ldtk_tools"))

from ambition_ldtk_tools.codegen_character_catalog import (  # noqa: E402
    character_id_for,
    display_name_of,
    render_entry,
)


def test_character_id_aliases_player_robot_to_player():
    """`player_robot` is the dedicated player sheet; the catalog id
    stays as `player` (not `npc_player_robot`)."""
    assert character_id_for("player_robot") == "player"


def test_character_id_aliases_absurd_general_to_npc_general():
    """The renderer's `absurd_general` target maps to the catalog's
    `npc_general` entry (legacy alias preserved during Phase 3
    migration)."""
    assert character_id_for("absurd_general") == "npc_general"


def test_character_id_aliases_puppy_slug_to_npc_puppy_slug():
    """puppy_slug is renderer-side bare-name but the catalog uses
    `npc_puppy_slug` for symmetry with other NPCs."""
    assert character_id_for("puppy_slug") == "npc_puppy_slug"


def test_character_id_preserves_base_characters():
    """goblin / robot / sandbag stay bare (they're base entries)."""
    assert character_id_for("goblin") == "goblin"
    assert character_id_for("robot") == "robot"
    assert character_id_for("sandbag") == "sandbag"


def test_character_id_prefixes_npc_for_arbitrary_target():
    """Anything else gets the `npc_` prefix."""
    assert character_id_for("agent_swarm") == "npc_agent_swarm"
    assert character_id_for("trex_enemy") == "npc_trex_enemy"
    assert (
        character_id_for("flying_spaghetti_monster_boss")
        == "npc_flying_spaghetti_monster_boss"
    )


def test_display_name_camelizes_underscores():
    """snake_case target → Title Case display."""
    assert display_name_of("agent_swarm") == "Agent Swarm"
    assert display_name_of("robot_guardian") == "Robot Guardian"
    assert (
        display_name_of("flying_spaghetti_monster_boss")
        == "Flying Spaghetti Monster Boss"
    )


def test_render_entry_produces_well_formed_ron():
    """End-to-end: render_entry emits a RON-formatted entry that
    fits inside the existing `characters: { ... }` map.

    The exact shape needs to round-trip through pyron without
    syntax errors and contain the expected keys."""
    out = render_entry("npc_agent_swarm", "agent_swarm", "enemy_swipe")
    # Should contain canonical fields.
    assert '"npc_agent_swarm":' in out
    assert 'display_name: "Agent Swarm"' in out
    assert 'spritesheet: "sprites/agent_swarm_spritesheet.png"' in out
    assert 'manifest: "sprites/agent_swarm_spritesheet.ron"' in out
    assert "tier: MainHall" in out
    assert "body_kind: Standard" in out
    assert 'default_brain: "melee_brute_striker"' in out
    assert 'default_action_set: "striker_swipe"' in out


def test_render_entry_picks_basement_tier_for_boss_category():
    """Boss category entries get the Basement tier + Wide body."""
    out = render_entry("npc_gnu_ton_boss", "gnu_ton_boss", "boss")
    assert "tier: Basement" in out
    assert "body_kind: Wide" in out
