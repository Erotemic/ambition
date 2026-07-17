"""Smoke tests for `generate_hall_of_characters` — the hall generator
that reads the catalog and emits the LDtk area spec with one
pedestal per character.

Tests focus on pure helpers (no LDtk file IO) so they stay fast and
deterministic."""

from __future__ import annotations

import sys
from pathlib import Path

import pytest

REPO_ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(REPO_ROOT / "tools" / "ambition_ldtk_tools"))

from ambition_ldtk_tools.generate_hall_of_characters import (  # noqa: E402
    BASEMENT_SLOTS_PER_ROW,
    BASEMENT_SLOT_HEIGHT_PX,
    BASEMENT_SLOT_WIDTH_PX,
    HALL_WIDTH_PX,
    MAIN_SLOTS_PER_FLOOR,
    MAIN_SLOT_HEIGHT_PX,
    MAIN_SLOT_WIDTH_PX,
    basement_rows_for,
    build_spec,
    derived_dims,
    main_floors_for,
    make_entity,
    merge_provider_entries,
)


def test_slot_widths_tile_the_hall():
    """Main + basement slot widths tile the fixed hall width exactly, so a
    row/floor is always full-width with no gap or overhang."""
    assert MAIN_SLOT_WIDTH_PX * MAIN_SLOTS_PER_FLOOR == HALL_WIDTH_PX
    assert BASEMENT_SLOT_WIDTH_PX * BASEMENT_SLOTS_PER_ROW == HALL_WIDTH_PX


def test_floors_and_rows_grow_to_fit_any_count():
    """The hall sizes its floors/rows to the roster — ceil-divide, min 1 — so
    ANY number of characters is accommodated without a capacity cap."""
    # Exact multiples.
    assert main_floors_for(96) == 6
    assert main_floors_for(16) == 1
    assert basement_rows_for(12) == 3
    # Partial floors/rows round UP so trailing characters still get a slot.
    assert main_floors_for(97) == 7
    assert main_floors_for(106) == 7
    assert basement_rows_for(11) == 3
    assert basement_rows_for(13) == 4
    # Empty sections still yield one floor/row (the hub-entry / terminal floor).
    assert main_floors_for(0) == 1
    assert basement_rows_for(0) == 1


def test_derived_dims_sizes_to_the_roster():
    """`derived_dims(main, basement)` returns (width, height) sized to seat
    exactly the given counts + chrome — the hall grows taller as the roster
    grows, never capping."""
    main_count, basement_count = 106, 11
    width, height = derived_dims(main_count, basement_count)
    assert width == HALL_WIDTH_PX
    expected = (
        16
        + (main_floors_for(main_count) * MAIN_SLOT_HEIGHT_PX)
        + 16
        + (basement_rows_for(basement_count) * BASEMENT_SLOT_HEIGHT_PX)
        + 16
    )
    assert height == expected
    # Adding a character that spills to a new floor makes the hall strictly
    # taller — proof the size tracks the count rather than a fixed cap.
    _, taller = derived_dims(main_count + MAIN_SLOTS_PER_FLOOR, basement_count)
    assert taller > height


def test_merge_provider_entries_appends_and_dedupes():
    """Provider-owned exhibit ids append after the catalog entries, routed by
    tier, and an id already present in either section is skipped (no double
    pedestal)."""
    main = ["goblin", "robot"]
    basement = ["npc_trex_enemy"]
    merged_main, merged_basement, dialogue_ids = merge_provider_entries(
        main,
        basement,
        {"robot": "hall_robot"},
        [
            ("sanic", "MainHall", "hall_sanic"),
            ("mary_o", "MainHall", "hall_mary_o"),
            ("smirking_behemoth_boss", "Basement", "hall_behemoth"),
            ("robot", "MainHall", "hall_robot"),  # already present -> skipped
        ],
    )
    assert merged_main == ["goblin", "robot", "sanic", "mary_o"]
    assert merged_basement == ["npc_trex_enemy", "smirking_behemoth_boss"]
    assert dialogue_ids == {
        "robot": "hall_robot",
        "sanic": "hall_sanic",
        "mary_o": "hall_mary_o",
        "smirking_behemoth_boss": "hall_behemoth",
    }


def test_merge_provider_entries_rejects_dialogue_binding_drift():
    """A provider exhibit must not silently override a catalog-owned binding."""
    with pytest.raises(ValueError, match="provider Hall dialogue mismatch"):
        merge_provider_entries(
            ["sanic"],
            [],
            {"sanic": "hall_old_sanic"},
            [("sanic", "MainHall", "hall_sanic")],
        )


def test_every_pedestal_gets_the_explicit_stand_still_override():
    """Every generated Hall NpcSpawn (main-hall AND basement) carries an EXPLICIT
    `brain_override: "stand_still"`. That explicit preset — not a Hall room flag or
    inferred placement behaviour — is why a peaceful wanderer (a puppy slug) holds
    still on its pedestal. The Hall stays fully explicit."""
    spec = build_spec(["alpha", "beta", "gamma"], ["boss_one"], {})
    npc_entities = [e for e in spec["entities"] if e["type"] == "NpcSpawn"]
    # 3 main-hall + 1 basement pedestal.
    assert len(npc_entities) == 4
    for entity in npc_entities:
        assert entity["fields"]["brain_override"] == "stand_still", entity
        # The pedestal remains a real, complete NPC (identity + inspect prompt),
        # not a presentation-only exhibit.
        assert entity["fields"]["character_id"] in {"alpha", "beta", "gamma", "boss_one"}
        assert entity["fields"]["prompt"] == "Inspect"


def test_make_entity_shape():
    """The minimal LDtk entity helper produces the shape area_authoring
    expects."""
    entity = make_entity("Solid", (10, 20), (100, 16), {"name": "test_floor"})
    assert entity == {
        "type": "Solid",
        "px": [10, 20],
        "size": [100, 16],
        "fields": {"name": "test_floor"},
    }
