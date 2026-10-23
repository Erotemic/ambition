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

from ambition_ldtk_tools.generate_hall_of_characters import (
    DOOR_RESERVED_SLOTS,  # noqa: E402
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
    provider_hall_entries,
    parse_catalog,
)


def test_slot_widths_tile_the_hall():
    """Main + basement slot widths tile the fixed hall width exactly, so a
    row/floor is always full-width with no gap or overhang."""
    assert MAIN_SLOT_WIDTH_PX * MAIN_SLOTS_PER_FLOOR == HALL_WIDTH_PX
    assert BASEMENT_SLOT_WIDTH_PX * BASEMENT_SLOTS_PER_ROW == HALL_WIDTH_PX


def test_floors_and_rows_grow_to_fit_any_count():
    """The hall sizes its floors/rows to the roster — ceil-divide, min 1 — so
    ANY number of characters is accommodated without a capacity cap.

    ⚠ **the door's reserved columns count against the main hall.** They occupy
    real slots on the entry floor, so the main-hall arithmetic is
    `ceil((count + DOOR_RESERVED_SLOTS) / MAIN_SLOTS_PER_FLOOR)` and a roster
    that used to exactly fill its last floor now needs one more. This test
    asserted the pre-door numbers (96 → 6) and went red when the door landed;
    it is written against the CONSTANTS now, so the next change to either one
    moves the expectation with it instead of leaving a stale literal behind.
    The basement has no door and keeps the plain ceil-divide.
    """
    # A main-hall count that exactly fills N floors, reserved slots included.
    exact = 6 * MAIN_SLOTS_PER_FLOOR - DOOR_RESERVED_SLOTS
    assert main_floors_for(exact) == 6
    assert main_floors_for(MAIN_SLOTS_PER_FLOOR - DOOR_RESERVED_SLOTS) == 1
    assert basement_rows_for(3 * BASEMENT_SLOTS_PER_ROW) == 3
    # One past a full floor/row rounds UP so trailing characters still get a slot.
    assert main_floors_for(exact + 1) == 7
    assert basement_rows_for(3 * BASEMENT_SLOTS_PER_ROW - 1) == 3
    assert basement_rows_for(3 * BASEMENT_SLOTS_PER_ROW + 1) == 4
    # the reserved slots are the whole reason this is not a plain ceil-divide,
    # so assert the boundary they move rather than trusting the arithmetic above
    # to have exercised it.
    assert main_floors_for(exact + DOOR_RESERVED_SLOTS) == 7, (
        "a roster that exactly fills the floors WITHOUT the door still fits — "
        "the reserved columns are not being counted"
    )
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


def test_parse_catalog_is_dependency_light_and_preserves_order():
    catalog = r'''
(
    brain_presets: {
        "stand_still": StandStill,
    },
    action_set_presets: {
        "peaceful": (
            move_style: Walk,
            melee: None,
            ranged: None,
            special: None,
        ),
    },
    characters: {
        "alpha": (
            tier: MainHall,
            barks: (hall: ["hello {world}"]),
            hall_dialogue_id: Some("hall_alpha"),
        ),
        // A comment containing a brace must not terminate the map: }
        "beta": (
            tier: Basement,
            hall_dialogue_id: Some("hall_beta"),
        ),
        "gamma": (
            tier: MainHall,
        ),
    },
)
'''
    main, basement, giants, dialogue = parse_catalog(catalog)
    assert giants == []
    assert main == ["alpha", "gamma"]
    assert "peaceful" not in main
    assert "stand_still" not in main
    assert basement == ["beta"]
    assert dialogue == {"alpha": "hall_alpha", "beta": "hall_beta"}


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


def test_a_giant_stands_in_its_own_row_below_the_basement():
    """The giant gnu's horns are 760 px above the floor and it draws ~700 px
    wide. On a 128x192 main-hall pedestal it covered eight neighbours and
    several floors; a basement slot is too short for it. A `Giant` gets a row
    of its own at the very bottom, as tall as its horns."""
    spec = build_spec(["alpha"], ["boss_one"], {}, ["npc_giant_gnu"])
    npcs = {e["fields"]["character_id"]: e for e in spec["entities"] if e["type"] == "NpcSpawn"}
    assert set(npcs) == {"alpha", "boss_one", "npc_giant_gnu"}

    def feet(cid):
        e = npcs[cid]
        return e["px"][1] + e["size"][1]

    assert feet("npc_giant_gnu") > feet("boss_one"), "below the basement"
    floors = {e["fields"]["name"]: e for e in spec["entities"] if e["type"] == "Solid"}
    assert feet("npc_giant_gnu") == floors["giant_row_1_floor"]["px"][1], "on the giant row's floor"
    # Clear height above the giant's feet: up to the basement floor over its row.
    over = floors["basement_row_1_floor_left"]
    clear = feet("npc_giant_gnu") - (over["px"][1] + over["size"][1])
    assert clear >= 780, clear
    # The basement's last row now drops through to the giants.
    assert "basement_row_1_floor" not in floors
    assert spec["px_hei"] == derived_dims(1, 1, 1)[1]


def test_an_unknown_tier_is_refused_not_dropped():
    catalog = """
(
    characters: {
        "alpha": (
            tier: Colossal,
        ),
    },
)
"""
    with pytest.raises(ValueError, match="Colossal"):
        parse_catalog(catalog)


def _catalog(rows: dict[str, str]) -> str:
    """A catalog whose `characters` map has each id with the manifest given."""
    body = "".join(
        f'        "{cid}": (\n            manifest: "{manifest}",\n            tier: MainHall,\n        ),\n'
        for cid, manifest in rows.items()
    )
    return "(\n    characters: {\n" + body + "    },\n)\n"


def test_a_provider_stand_in_is_not_a_second_stand():
    """A provider row that wears a sheet the Hall already shows is that
    character; a row with its own sheet is exhibited, bound to its Hall
    conversation only when it names one."""
    content = _catalog({"robot": "sprites/robot_spritesheet.ron"})
    provider = _catalog(
        {
            "duelist": "sprites/robot_spritesheet.ron",
            "snake": "sprites/snake_spritesheet.ron",
            "snake_again": "sprites/snake_spritesheet.ron",
        }
    )
    assert provider_hall_entries(content, [provider]) == [("snake", "MainHall", None)]


def test_the_committed_hall_is_the_one_the_catalogs_generate():
    """Every hub character has a stand: the committed spec is what the
    catalogs generate now. A character added without regenerating the Hall
    (`python -m ambition_ldtk_tools.generate_hall_of_characters`) fails here."""
    from ambition_ldtk_tools.generate_hall_of_characters import (
        CATALOG_PATH,
        HEADER,
        ORDER_PATH,
        PROVIDER_CATALOGS,
        SPEC_PATH,
        assemble_hall,
    )
    from ambition_ldtk_tools.ron_parse import dumps as ron_dumps

    main, basement, giants, dialogue_ids, unnamed = assemble_hall(
        CATALOG_PATH.read_text(), [p.read_text() for p in PROVIDER_CATALOGS], ORDER_PATH.read_text()
    )
    assert not unnamed, f"give these exhibits a wing in {ORDER_PATH.name}: {unnamed}"
    expected = HEADER + ron_dumps(build_spec(main, basement, dialogue_ids, giants))
    assert SPEC_PATH.read_text() == expected, "the Hall is stale: regenerate it"


def test_the_hall_seats_its_wings_in_order_and_the_unnamed_last():
    """The order file seats each floor wing by wing; an exhibit no wing names
    is seated after them in catalog order, and is reported."""
    from ambition_ldtk_tools.generate_hall_of_characters import order_by_wings, parse_wings

    wings = parse_wings("# a comment\n[Robots]\nrobot_b\nrobot_a  # trailing\n\n[Dogs]\ndog\ngone\n")
    assert wings == [("Robots", ["robot_b", "robot_a"]), ("Dogs", ["dog", "gone"])]
    ordered, unnamed = order_by_wings(["new_one", "dog", "robot_a", "robot_b", "newer"], wings)
    assert ordered == ["robot_b", "robot_a", "dog", "new_one", "newer"]
    assert unnamed == ["new_one", "newer"]


def test_an_exhibit_named_twice_in_the_order_is_refused():
    from ambition_ldtk_tools.generate_hall_of_characters import parse_wings
    with pytest.raises(ValueError, match="named twice"):
        parse_wings("[A]\nrobot\n[B]\nrobot\n")
