"""Smoke tests for `generate_hall_of_characters` — the hall generator
that reads the catalog and emits the LDtk area spec with one
pedestal per character.

Tests focus on pure helpers (no LDtk file IO) so they stay fast and
deterministic."""
from __future__ import annotations

import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(REPO_ROOT / "tools" / "ambition_ldtk_tools"))

from ambition_ldtk_tools.generate_hall_of_characters import (  # noqa: E402
    BASEMENT_SLOTS_PER_ROW,
    BASEMENT_SLOT_HEIGHT_PX,
    BASEMENT_SLOT_WIDTH_PX,
    HALL_WIDTH_PX,
    MAIN_FLOORS,
    MAIN_SLOTS_PER_FLOOR,
    MAIN_SLOT_HEIGHT_PX,
    MAIN_SLOT_WIDTH_PX,
    derived_dims,
    make_entity,
)


def test_main_hall_capacity_matches_layout_constants():
    """Sanity-pin the layout dimensions so a future reflow doesn't
    silently change capacity. 6 floors × 16 slots = 96 main capacity."""
    assert MAIN_FLOORS * MAIN_SLOTS_PER_FLOOR == 96
    assert MAIN_SLOT_WIDTH_PX * MAIN_SLOTS_PER_FLOOR == HALL_WIDTH_PX


def test_basement_capacity_for_three_rows():
    """Basement slots are 512 wide × 4 per row = exactly HALL_WIDTH_PX.
    3 rows × 4 slots = 12 basement capacity (slack above current
    catalog's 10 Basement entries)."""
    assert BASEMENT_SLOT_WIDTH_PX * BASEMENT_SLOTS_PER_ROW == HALL_WIDTH_PX


def test_derived_dims_layout():
    """`derived_dims` returns (width, height) sized for the main
    section + 3 basement rows + chrome. Pin so a layout reflow that
    accidentally drops one of the sections trips the test."""
    width, height = derived_dims()
    assert width == HALL_WIDTH_PX
    # Height = ceiling + (6 floors × 192) + floor + (3 basement × 384) + floor
    expected = 16 + (MAIN_FLOORS * MAIN_SLOT_HEIGHT_PX) + 16 + (3 * BASEMENT_SLOT_HEIGHT_PX) + 16
    assert height == expected


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
