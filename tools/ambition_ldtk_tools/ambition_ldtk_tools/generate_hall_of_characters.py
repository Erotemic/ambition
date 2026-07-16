#!/usr/bin/env python3
"""Generate the Hall of Characters area spec from the character
catalog.

Phase 5 of the character-catalog refactor (see
`TODO-character-catalog-and-hall.md`). Reads
`game/ambition_content/assets/data/character_catalog.ron`, lays out
one pedestal per character, and emits
`tools/ambition_ldtk_tools/specs/hall_of_characters_area.ron`.

Each pedestal's name is drawn by the runtime actor nameplate (which reads
the character's own identity), so the hall authors no per-pedestal name
label of its own.

## Layout

   +-------------------------------------------------------+ y = 0
   | ▒ ceiling ▒                                            |
   |  🧍 🧍 🧍 ...  16 slots × 128 px                       |  Floor 6 (top / newest)
   | ─[ladder]──[ladder]── OneWayPlatform ──                |
   |  🧍 🧍 🧍 ...                                          |  Floor 5
   | ─[ladder]──[ladder]── OneWayPlatform ──                |
   |   ...                                                  |  Floors 4..2
   | ─[ladder]──[ladder]── OneWayPlatform ──                |
   |  🧍 🧍 🧍 ...                                          |  Floor 1 (hub entry)
   | ▒▒▒▒▒▒▒▒[drop hole]▒▒▒▒▒▒▒▒▒                          |
   |                                                        |
   |  🦖 🛸 🪲 🧙 🐻  ...                                   |  Basement row 1 (5 × 256)
   |  🦖 🛸 ...                                             |  Basement row 2 (overflow)
   | ▒▒▒▒▒▒▒▒[ladder back up]▒▒▒▒▒▒▒▒                      |
   +-------------------------------------------------------+

Constants (designed for the current 99-entry catalog with room to
grow to ~110 main-hall + ~16 basement):

  HALL_WIDTH_PX         = 2048
  MAIN_SLOT_WIDTH_PX    = 128
  MAIN_SLOT_HEIGHT_PX   = 192
  MAIN_FLOORS_DEFAULT   = 6     (extend by changing this)
  MAIN_SLOTS_PER_FLOOR  = 16
  BASEMENT_SLOT_WIDTH_PX  = 256
  BASEMENT_SLOT_HEIGHT_PX = 320
  BASEMENT_SLOTS_PER_ROW  = 8

## Usage

```bash
PYTHONPATH=tools/ambition_ldtk_tools \\
python -m ambition_ldtk_tools.generate_hall_of_characters
```

Re-running with no catalog changes produces a byte-identical spec
(idempotent — characters are emitted in catalog key order).
"""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path
from typing import Any

REPO_ROOT = Path(__file__).resolve().parents[3]
CATALOG_PATH = (
    REPO_ROOT
    / "game"
    / "ambition_content"
    / "assets"
    / "data"
    / "character_catalog.ron"
)
SPEC_PATH = (
    REPO_ROOT
    / "tools"
    / "ambition_ldtk_tools"
    / "specs"
    / "hall_of_characters_area.ron"
)
# The Hall lives in its OWN secondary-world `.ldtk` file (like the cut-rope
# arena) rather than inside the monolithic sandbox.ldtk, so each regen is a
# clean wholesale overwrite of one file instead of splicing a level into the
# monolith. The hub-side door is permanent hand-authored content in
# sandbox.ldtk (`hall_of_characters_door`); this file only carries the hall
# level + its own entry zone that cross-targets the hub.
HALL_LDTK_PATH = (
    REPO_ROOT
    / "game"
    / "ambition_content"
    / "assets"
    / "worlds"
    / "hall_of_characters.ldtk"
)
HALL_LDTK_IDENTIFIER = "ambition-hall-world"

# --- Layout dimensions ---
HALL_WIDTH_PX = 2048
MAIN_SLOT_WIDTH_PX = 128
MAIN_SLOT_HEIGHT_PX = 192
MAIN_FLOORS = 6
MAIN_SLOTS_PER_FLOOR = 16
BASEMENT_SLOT_WIDTH_PX = 512
BASEMENT_SLOT_HEIGHT_PX = 384
BASEMENT_SLOTS_PER_ROW = 4
CEILING_PX = 16
FLOOR_THICKNESS_PX = 16
HALL_WORLD_X = 40000  # to the right of every existing level (rightmost is x=39024)
HALL_WORLD_Y = 0


def derived_dims() -> tuple[int, int]:
    """Return (pxWid, pxHei) for the hall."""
    width = HALL_WIDTH_PX
    main_section = MAIN_FLOORS * MAIN_SLOT_HEIGHT_PX
    # Basement section sized to comfortably fit the current 10
    # Basement-tier entries plus headroom. With 4 slots/row, a 3-row
    # basement section yields 12 slot capacity.
    basement_rows = 3
    basement_section = basement_rows * BASEMENT_SLOT_HEIGHT_PX
    height = (
        CEILING_PX
        + main_section
        + FLOOR_THICKNESS_PX
        + basement_section
        + FLOOR_THICKNESS_PX
    )
    return width, height


def parse_catalog(
    catalog_text: str,
) -> tuple[list[str], list[str], dict[str, str]]:
    """Read the catalog file and return:

      (main_hall_ids_in_order, basement_ids_in_order, hall_dialogue_id_for_id)

    Display names are NOT read here: each pedestal's name is drawn by the
    actor nameplate system (which reads the character's own identity), so the
    hall no longer authors redundant `DebugLabel` overlays.

    The pyron upstream loses Rust enum discriminators on unit
    variants, so we regex `tier:` (and the optional `hall_dialogue_id:
    Some("...")`) directly out of each entry block.
    """
    from .ron_parse import load as ron_load

    data = ron_load(catalog_text)
    ids = list(data["characters"].keys())
    tiers: dict[str, str] = {}
    hall_dialogue_ids: dict[str, str] = {}
    for cid in ids:
        key_pat = re.compile(r'"' + re.escape(cid) + r'"\s*:\s*\(', re.MULTILINE)
        m = key_pat.search(catalog_text)
        if not m:
            tiers[cid] = "MainHall"
            continue
        # Bound the entry window at the next top-level character key (8-space
        # indent) so a long `barks` list never pushes a trailing field
        # (`hall_dialogue_id`) out of a fixed-size window.
        rest = catalog_text[m.end() :]
        next_key = re.search(r'\n {8}"[a-z0-9_]+"\s*:\s*\(', rest)
        window = rest[: next_key.start()] if next_key else rest[:2000]
        tm = re.search(r"tier:\s*([A-Za-z_]+)", window)
        tiers[cid] = tm.group(1) if tm else "MainHall"
        # Optional per-character Hall dialogue node. Regex the `Some("...")`
        # so pyron's unit-variant blind spot doesn't drop it.
        hm = re.search(r'hall_dialogue_id:\s*Some\(\s*"([^"]+)"\s*\)', window)
        if hm:
            hall_dialogue_ids[cid] = hm.group(1)

    main = [cid for cid in ids if tiers[cid] == "MainHall"]
    basement = [cid for cid in ids if tiers[cid] == "Basement"]
    return main, basement, hall_dialogue_ids


def make_entity(
    type_name: str, px: tuple[int, int], size: tuple[int, int], fields: dict[str, Any]
) -> dict:
    return {
        "type": type_name,
        "px": [px[0], px[1]],
        "size": [size[0], size[1]],
        "fields": fields,
    }


def build_spec(
    main_ids: list[str],
    basement_ids: list[str],
    hall_dialogue_ids: dict[str, str] | None = None,
) -> dict:
    hall_dialogue_ids = hall_dialogue_ids or {}
    px_wid, px_hei = derived_dims()

    # --- Compute slot positions ---
    # Floor 1 is the lowest main floor (hub entry); Floor N is the
    # top floor (newest / overflow). We lay out floors top to bottom
    # in screen-space (y=0 is top).
    main_section_top = CEILING_PX
    basement_section_top = (
        main_section_top + MAIN_FLOORS * MAIN_SLOT_HEIGHT_PX + FLOOR_THICKNESS_PX
    )

    # Floor index 0 is top (Floor N), floor index MAIN_FLOORS - 1 is bottom (Floor 1).
    # We pack main_ids starting at Floor 1 (bottom-most), left to right,
    # so the first catalog entry sits at the hub-entry floor.
    def main_slot_world_xy(slot_index: int) -> tuple[int, int, int, int]:
        """Return (px_x, px_y, px_w, px_h) for slot # slot_index."""
        floor_from_bottom = slot_index // MAIN_SLOTS_PER_FLOOR
        col_in_floor = slot_index % MAIN_SLOTS_PER_FLOOR
        floor_index_from_top = MAIN_FLOORS - 1 - floor_from_bottom
        slot_top_y = main_section_top + floor_index_from_top * MAIN_SLOT_HEIGHT_PX
        slot_left_x = col_in_floor * MAIN_SLOT_WIDTH_PX
        return (slot_left_x, slot_top_y, MAIN_SLOT_WIDTH_PX, MAIN_SLOT_HEIGHT_PX)

    def basement_slot_world_xy(slot_index: int) -> tuple[int, int, int, int]:
        row = slot_index // BASEMENT_SLOTS_PER_ROW
        col_in_row = slot_index % BASEMENT_SLOTS_PER_ROW
        slot_top_y = basement_section_top + row * BASEMENT_SLOT_HEIGHT_PX
        slot_left_x = col_in_row * BASEMENT_SLOT_WIDTH_PX
        return (
            slot_left_x,
            slot_top_y,
            BASEMENT_SLOT_WIDTH_PX,
            BASEMENT_SLOT_HEIGHT_PX,
        )

    entities: list[dict] = []

    # --- Outer geometry ---
    # Top ceiling.
    entities.append(
        make_entity(
            "Solid",
            (0, 0),
            (px_wid, CEILING_PX),
            {"name": "hall_ceiling"},
        )
    )
    # Bottom floor (under the basement).
    entities.append(
        make_entity(
            "Solid",
            (0, px_hei - FLOOR_THICKNESS_PX),
            (px_wid, FLOOR_THICKNESS_PX),
            {"name": "hall_floor"},
        )
    )
    # Left + right walls.
    entities.append(
        make_entity(
            "Solid",
            (0, 0),
            (16, px_hei),
            {"name": "hall_left_wall"},
        )
    )
    entities.append(
        make_entity(
            "Solid",
            (px_wid - 16, 0),
            (16, px_hei),
            {"name": "hall_right_wall"},
        )
    )

    # --- Main-hall floor surfaces (OneWayPlatform between floors) ---
    # Between Floor i (from top) and Floor i+1 sits a OneWayPlatform
    # with a center gap for ladder drop. The bottom-most floor
    # (Floor 1, the hub-entry floor) is a Solid so the player has
    # something to land on.
    # We have MAIN_FLOORS floors; that's MAIN_FLOORS - 1 platforms
    # *between* floors, plus the solid floor of Floor 1.
    for i in range(MAIN_FLOORS - 1):
        floor_top_index = i  # from top
        platform_y = main_section_top + (floor_top_index + 1) * MAIN_SLOT_HEIGHT_PX
        # Solid left half + solid right half with a gap in the middle
        # for the ladder column. The ladder is at x = 1024-32..1024+32.
        gap_w = 96
        gap_x = (px_wid - gap_w) // 2
        # Left segment.
        entities.append(
            make_entity(
                "OneWayPlatform",
                (16, platform_y),
                (gap_x - 16, 16),
                {"name": f"floor_platform_{i + 1}_left"},
            )
        )
        # Right segment.
        entities.append(
            make_entity(
                "OneWayPlatform",
                (gap_x + gap_w, platform_y),
                (px_wid - 16 - (gap_x + gap_w), 16),
                {"name": f"floor_platform_{i + 1}_right"},
            )
        )
    # Solid floor under Floor 1 (the bottom of the main section).
    floor1_top = main_section_top + MAIN_FLOORS * MAIN_SLOT_HEIGHT_PX
    drop_hole_w = 96
    drop_hole_x = (px_wid - drop_hole_w) // 2
    entities.append(
        make_entity(
            "Solid",
            (16, floor1_top),
            (drop_hole_x - 16, FLOOR_THICKNESS_PX),
            {"name": "floor_1_solid_left"},
        )
    )
    entities.append(
        make_entity(
            "Solid",
            (drop_hole_x + drop_hole_w, floor1_top),
            (px_wid - 16 - (drop_hole_x + drop_hole_w), FLOOR_THICKNESS_PX),
            {"name": "floor_1_solid_right"},
        )
    )

    # --- Basement floors: one platform per basement row so each
    # boss/large-character pedestal has something to stand on.
    # Without these the wide bosses (gnu_ton, trex_enemy) render
    # mid-air with the next row's sprite below them. Floor top is
    # placed at the slot's foot_y so the sprite's foot anchor lands
    # flush on the surface.
    #
    # Each floor is split into two Solid segments with a center
    # drop-through gap so the player can navigate down to lower
    # rows. The bottom-most basement row's floor stays whole as
    # the room's terminal floor.
    basement_floor_thickness = 16
    basement_rows_total = 3
    drop_hole_w = 96
    drop_hole_x = (px_wid - drop_hole_w) // 2
    for row in range(basement_rows_total):
        floor_top_y = basement_section_top + (row + 1) * BASEMENT_SLOT_HEIGHT_PX
        is_last_row = row == basement_rows_total - 1
        if is_last_row:
            # Terminal floor — no drop hole.
            entities.append(
                make_entity(
                    "Solid",
                    (16, floor_top_y),
                    (px_wid - 32, basement_floor_thickness),
                    {"name": f"basement_row_{row + 1}_floor"},
                )
            )
        else:
            # Two-segment floor with a center drop hole.
            entities.append(
                make_entity(
                    "Solid",
                    (16, floor_top_y),
                    (drop_hole_x - 16, basement_floor_thickness),
                    {"name": f"basement_row_{row + 1}_floor_left"},
                )
            )
            entities.append(
                make_entity(
                    "Solid",
                    (drop_hole_x + drop_hole_w, floor_top_y),
                    (
                        px_wid - 16 - (drop_hole_x + drop_hole_w),
                        basement_floor_thickness,
                    ),
                    {"name": f"basement_row_{row + 1}_floor_right"},
                )
            )

    # --- PlayerStart at the hub-entry floor (left side) ---
    floor1_slot_y = main_section_top + (MAIN_FLOORS - 1) * MAIN_SLOT_HEIGHT_PX
    entities.append(
        make_entity(
            "PlayerStart",
            (96, floor1_slot_y + MAIN_SLOT_HEIGHT_PX - 48),
            (28, 46),
            {"name": "hall_spawn"},
        )
    )

    # --- LoadingZone door back to the hub ---
    # Hall door targets the `central_hub_complex` active area (the
    # logical group covering main hub + basement). hall_of_bosses
    # uses the same target.
    entities.append(
        make_entity(
            "LoadingZone",
            (24, floor1_slot_y + MAIN_SLOT_HEIGHT_PX - 96),
            (48, 96),
            {
                "id": "hall_of_characters_entry",
                "name": "hall_of_characters_entry",
                "activation": "Door",
                "target_room": "central_hub_complex",
                "target_zone": "hall_of_characters_door",
                "bidirectional": True,
            },
        )
    )

    # --- NPC pedestals for each MainHall entry ---
    for slot_index, cid in enumerate(main_ids):
        if slot_index >= MAIN_FLOORS * MAIN_SLOTS_PER_FLOOR:
            # Out of capacity — log + skip. The headless test will
            # flag the overflow when it compares the catalog size to
            # the spawn count.
            sys.stderr.write(
                f"[warn] hall capacity ({MAIN_FLOORS}x{MAIN_SLOTS_PER_FLOOR}) "
                f"exhausted; skipping '{cid}' and subsequent main-hall entries.\n"
            )
            break
        x, y, w, h = main_slot_world_xy(slot_index)
        center_x = x + w // 2
        foot_y = y + h
        npc_w, npc_h = 32, 48
        entities.append(
            make_entity(
                "NpcSpawn",
                (center_x - npc_w // 2, foot_y - npc_h),
                (npc_w, npc_h),
                {
                    "character_id": cid,
                    "prompt": "Inspect",
                    "dialogue_id": hall_dialogue_ids.get(cid, ""),
                    "patrol_radius": 0,
                },
            )
        )

    # --- Basement pedestals ---
    for slot_index, cid in enumerate(basement_ids):
        x, y, w, h = basement_slot_world_xy(slot_index)
        center_x = x + w // 2
        foot_y = y + h
        npc_w, npc_h = 48, 80
        entities.append(
            make_entity(
                "NpcSpawn",
                (center_x - npc_w // 2, foot_y - npc_h),
                (npc_w, npc_h),
                {
                    "character_id": cid,
                    "prompt": "Inspect",
                    "dialogue_id": hall_dialogue_ids.get(cid, ""),
                    "patrol_radius": 0,
                },
            )
        )

    # --- Single full-room camera zone ---
    entities.append(
        make_entity(
            "CameraZone",
            (0, 0),
            (px_wid, px_hei),
            {
                "id": "hall_of_characters_camera",
                "name": "hall_of_characters_camera",
                "mode": "Default",
            },
        )
    )

    spec = {
        "id": "hall_of_characters",
        "level_id": "hall_of_characters",
        "world_x": HALL_WORLD_X,
        "world_y": HALL_WORLD_Y,
        "px_wid": px_wid,
        "px_hei": px_hei,
        "fill_collision": "empty",
        "bg_color": "#0F121A",
        "biome": "hall",
        "music_track": "pulse_drift_voyage",
        "ambient_profile": "hum",
        "visual_theme": "default",
        # The engine keys "Hall bark pool + gallery policy" off this generic
        # `gallery` flag, not a hardcoded room id (C1). The `gallery` levelField
        # def is registered in sandbox.ldtk (cloned into this file on a fresh
        # scaffold) via `ambition_ldtk_tools level add-field-def`.
        "gallery": True,
        # No `connect_to`: the hub-side door (`hall_of_characters_door` in
        # central_hub_main) is permanent hand-authored content in sandbox.ldtk.
        # The hall's own `hall_of_characters_entry` zone (authored above in
        # `entities`) cross-targets the hub; the runtime merge resolves it the
        # same way it resolves the cut-rope arena's door. Emitting connect_to
        # here would make `area create` try to insert a reciprocal into
        # central_hub_main, which does not live in this dedicated file.
        "entities": entities,
    }
    return spec


HEADER = """\
// Hall of Characters — auto-generated.
//
// One pedestal per character_catalog.ron entry. Stress-tests entity
// counts, verifies every catalog id resolves to a sprite, and serves
// as a visual sanity gallery: walking the hall confirms every
// character a future Ambition game might spawn is wired through the
// catalog.
//
// This file is GENERATED. Edit
// `tools/ambition_ldtk_tools/ambition_ldtk_tools/generate_hall_of_characters.py`
// or the catalog instead.

"""


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--catalog", type=Path, default=CATALOG_PATH)
    parser.add_argument("--out", type=Path, default=SPEC_PATH)
    parser.add_argument(
        "--ldtk",
        type=Path,
        default=HALL_LDTK_PATH,
        help="Dedicated secondary-world file to (re)build from the spec.",
    )
    parser.add_argument(
        "--spec-only",
        action="store_true",
        help="Write the area spec only; do not scaffold/apply the .ldtk file.",
    )
    parser.add_argument("--print-summary", action="store_true")
    args = parser.parse_args(argv)

    from .ron_parse import dumps as ron_dumps

    text = args.catalog.read_text()
    main_ids, basement_ids, hall_dialogue_ids = parse_catalog(text)
    spec = build_spec(main_ids, basement_ids, hall_dialogue_ids)
    out_text = HEADER + ron_dumps(spec)
    args.out.write_text(out_text)

    applied = False
    if not args.spec_only:
        applied = _apply_to_dedicated_ldtk(args.out, args.ldtk)

    if args.print_summary:
        px_wid, px_hei = derived_dims()
        print(f"hall: {px_wid}x{px_hei} px")
        print(
            f"  main_hall entries: {len(main_ids)} / capacity {MAIN_FLOORS * MAIN_SLOTS_PER_FLOOR}"
        )
        print(f"  basement entries:  {len(basement_ids)}")
        print(f"  spec written to:   {args.out}")
        if applied:
            print(f"  ldtk written to:   {args.ldtk}")
    return 0


def _apply_to_dedicated_ldtk(spec_path: Path, ldtk_path: Path) -> bool:
    """Scaffold the dedicated hall `.ldtk` (clone defs from sandbox.ldtk) if it
    does not exist yet, then (re)build the hall level inside it from `spec_path`.

    Idempotent: re-running overwrites the single hall level wholesale via
    `area create --replace-existing`, never touching sandbox.ldtk. Returns True
    on success.
    """
    from ambition_ldtk_tools import area_authoring, world_init

    if not ldtk_path.exists():
        print(f"scaffolding new secondary world: {ldtk_path}")
        rc = world_init.main([str(ldtk_path), "--identifier", HALL_LDTK_IDENTIFIER])
        if rc != 0:
            print(f"error: world init failed (rc={rc})", file=sys.stderr)
            return False

    # `--no-repair`: `write_project` already emits canonical editor-style JSON,
    # so no repair pass is needed. We skip the bundled validate because it runs
    # the file in isolation and would flag the hall's `hall_of_characters_entry`
    # zone (which legitimately cross-targets the hub in sandbox.ldtk) as an
    # "unknown room" false positive. The authoritative check is
    # `validate <hall> --secondary-world <sandbox>` (see the regen recipe).
    rc = area_authoring.main(
        [
            str(spec_path),
            "--ldtk",
            str(ldtk_path),
            "--replace-existing",
            "--no-repair",
        ]
    )
    if rc != 0:
        print(f"error: area create into {ldtk_path} failed (rc={rc})", file=sys.stderr)
        return False
    return True


if __name__ == "__main__":
    sys.exit(main())
