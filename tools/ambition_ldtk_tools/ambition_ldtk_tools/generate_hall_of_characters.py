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

The hall GROWS to fit its roster: the number of main-hall floors and basement
rows is DERIVED from the character counts (`main_floors_for` /
`basement_rows_for`), so any number of characters is accommodated with no fixed
capacity cap. Adding characters just makes the hall taller.

  HALL_WIDTH_PX           = 2048
  MAIN_SLOT_WIDTH_PX      = 128
  MAIN_SLOT_HEIGHT_PX     = 192
  MAIN_SLOTS_PER_FLOOR    = 16    (floors = ceil(main_count / 16))
  BASEMENT_SLOT_WIDTH_PX  = 512
  BASEMENT_SLOT_HEIGHT_PX = 384
  BASEMENT_SLOTS_PER_ROW  = 4     (rows = ceil(basement_count / 4))

Provider-owned characters (Sanic, Mary-O, ...) are not in this catalog file;
they are exhibited by referencing their canonical ids in `PROVIDER_HALL_ENTRIES`
and resolved at runtime against the assembled catalog.

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
import json
import re
import sys
import tempfile
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
# The hall GROWS to fit its roster: the number of main-hall floors and basement
# rows is derived from the character counts (`main_floors_for` /
# `basement_rows_for`), so ANY number of characters is accommodated — there is no
# fixed capacity cap, and adding characters simply makes the hall taller.
HALL_WIDTH_PX = 2048
MAIN_SLOT_WIDTH_PX = 128
MAIN_SLOT_HEIGHT_PX = 192
MAIN_SLOTS_PER_FLOOR = 16
BASEMENT_SLOT_WIDTH_PX = 512
BASEMENT_SLOT_HEIGHT_PX = 384
BASEMENT_SLOTS_PER_ROW = 4
CEILING_PX = 16
FLOOR_THICKNESS_PX = 16
HALL_WORLD_X = 40000  # to the right of every existing level (rightmost is x=39024)
HALL_WORLD_Y = 0


def main_floors_for(main_count: int) -> int:
    """Main-hall floors needed to seat `main_count` pedestals — ceil-divide by
    the per-floor slot count. At least 1 so the hub-entry floor always exists."""
    return max(1, -(-main_count // MAIN_SLOTS_PER_FLOOR))


def basement_rows_for(basement_count: int) -> int:
    """Basement rows needed to seat `basement_count` pedestals. At least 1 so
    there is always a terminal basement floor even with no basement entries."""
    return max(1, -(-basement_count // BASEMENT_SLOTS_PER_ROW))


def derived_dims(main_count: int, basement_count: int) -> tuple[int, int]:
    """Return (pxWid, pxHei) sized to seat exactly `main_count` main-hall and
    `basement_count` basement pedestals. The hall grows to fit any roster; there
    is no fixed floor/row cap."""
    width = HALL_WIDTH_PX
    main_section = main_floors_for(main_count) * MAIN_SLOT_HEIGHT_PX
    basement_section = basement_rows_for(basement_count) * BASEMENT_SLOT_HEIGHT_PX
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

    This intentionally scans only the catalog's top-level character entries
    rather than importing the optional ``python-ron`` extension. The Hall
    generator needs just three stable fields (id, tier, Hall dialogue id), and
    all three are authored in a rigid top-level shape. Keeping this path
    dependency-light lets adding a character regenerate the Hall in any normal
    Python environment while the Rust runtime remains the authority for full
    catalog deserialization.
    """
    entry_pat = re.compile(
        r'^ {8}"(?P<id>[a-z0-9_]+)"\s*:\s*\(', re.MULTILINE
    )
    matches = list(entry_pat.finditer(catalog_text))
    ids = [match.group("id") for match in matches]
    tiers: dict[str, str] = {}
    hall_dialogue_ids: dict[str, str] = {}
    for idx, match in enumerate(matches):
        cid = match.group("id")
        end = matches[idx + 1].start() if idx + 1 < len(matches) else len(catalog_text)
        window = catalog_text[match.end() : end]
        tm = re.search(r"tier:\s*([A-Za-z_]+)", window)
        tiers[cid] = tm.group(1) if tm else "MainHall"
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
    # Floors/rows are SIZED to the roster (grows to fit any character count).
    main_floors = main_floors_for(len(main_ids))
    basement_rows = basement_rows_for(len(basement_ids))
    px_wid, px_hei = derived_dims(len(main_ids), len(basement_ids))

    # --- Compute slot positions ---
    # Floor 1 is the lowest main floor (hub entry); Floor N is the
    # top floor (newest / overflow). We lay out floors top to bottom
    # in screen-space (y=0 is top).
    main_section_top = CEILING_PX
    basement_section_top = (
        main_section_top + main_floors * MAIN_SLOT_HEIGHT_PX + FLOOR_THICKNESS_PX
    )

    # Floor index 0 is top (Floor N), floor index main_floors - 1 is bottom (Floor 1).
    # We pack main_ids starting at Floor 1 (bottom-most), left to right,
    # so the first catalog entry sits at the hub-entry floor.
    def main_slot_world_xy(slot_index: int) -> tuple[int, int, int, int]:
        """Return (px_x, px_y, px_w, px_h) for slot # slot_index."""
        floor_from_bottom = slot_index // MAIN_SLOTS_PER_FLOOR
        col_in_floor = slot_index % MAIN_SLOTS_PER_FLOOR
        floor_index_from_top = main_floors - 1 - floor_from_bottom
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
    # We have `main_floors` floors; that's main_floors - 1 platforms
    # *between* floors, plus the solid floor of Floor 1.
    for i in range(main_floors - 1):
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
    floor1_top = main_section_top + main_floors * MAIN_SLOT_HEIGHT_PX
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
    drop_hole_w = 96
    drop_hole_x = (px_wid - drop_hole_w) // 2
    for row in range(basement_rows):
        floor_top_y = basement_section_top + (row + 1) * BASEMENT_SLOT_HEIGHT_PX
        is_last_row = row == basement_rows - 1
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
    floor1_slot_y = main_section_top + (main_floors - 1) * MAIN_SLOT_HEIGHT_PX
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
    # No capacity cap: `main_floors` was sized to hold every entry, so the hall
    # grows to fit the roster rather than dropping trailing characters.
    for slot_index, cid in enumerate(main_ids):
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
                    # Every Hall pedestal is a REAL NPC instance frozen by an
                    # EXPLICIT stand-still brain override — not a Hall room flag or
                    # inferred placement behaviour. This is why peaceful wanderers
                    # (puppy slugs) hold still on their pedestal while keeping their
                    # full identity/body/dialogue and the ability to be switched
                    # back to their default brain later.
                    "brain_override": "stand_still",
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
                    # Explicit stand-still override (see the main-hall pedestal note).
                    "brain_override": "stand_still",
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


# Provider-owned characters to EXHIBIT in the Hall. These ids are authored by
# SEPARATE experience providers (ambition_demo_sanic, ambition_demo_mary_o, ...),
# NOT by ambition_content's catalog, so they never appear via the tier scan of
# the catalog file. We reference their CANONICAL ids here; at runtime each
# resolves against the App-local ASSEMBLED catalog (every provider fragment is
# merged before the Hall room loads). This is the authored inclusion list for
# cross-provider exhibits — the single place to add/remove a provider character
# from the Hall, never by editing the provider's own definition.
#
# (character_id, tier, hall_dialogue_id), where tier is "MainHall" or
# "Basement". Transformations
# (super_sanic, mary_o_tall) are listed EXPLICITLY, never auto-expanded.
PROVIDER_HALL_ENTRIES: list[tuple[str, str, str]] = [
    ("sanic", "MainHall", "hall_sanic"),
    ("super_sanic", "MainHall", "hall_super_sanic"),
    ("mary_o", "MainHall", "hall_mary_o"),
    ("mary_o_tall", "MainHall", "hall_mary_o_tall"),
]


def merge_provider_entries(
    main_ids: list[str],
    basement_ids: list[str],
    hall_dialogue_ids: dict[str, str],
    provider_entries: list[tuple[str, str, str]],
) -> tuple[list[str], list[str], dict[str, str]]:
    """Append authored provider-owned exhibits and their Hall dialogue binding.

    The provider rows live in separate Rust-embedded catalog fragments, so this
    cross-provider exhibit list is the Hall generator's only build-time view of
    them. Runtime integration tests compare each generated binding against the
    assembled catalog row, making any duplicate metadata drift fail loudly.

    Skips an id already present in either section, so a native catalog row and
    a provider reference never double up. Ordering is stable: catalog entries
    first (in file order), then provider entries in authored order.
    """
    seen = set(main_ids) | set(basement_ids)
    for cid, tier, dialogue_id in provider_entries:
        existing = hall_dialogue_ids.get(cid)
        if existing is not None and existing != dialogue_id:
            raise ValueError(
                f"provider Hall dialogue mismatch for {cid!r}: "
                f"catalog={existing!r}, exhibit={dialogue_id!r}"
            )
        hall_dialogue_ids[cid] = dialogue_id
        if cid in seen:
            continue
        seen.add(cid)
        if tier == "Basement":
            basement_ids.append(cid)
        else:
            main_ids.append(cid)
    return main_ids, basement_ids, hall_dialogue_ids


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
    main_ids, basement_ids, hall_dialogue_ids = merge_provider_entries(
        main_ids, basement_ids, hall_dialogue_ids, PROVIDER_HALL_ENTRIES
    )
    spec = build_spec(main_ids, basement_ids, hall_dialogue_ids)
    out_text = HEADER + ron_dumps(spec)
    args.out.write_text(out_text)

    applied = False
    if not args.spec_only:
        applied = _apply_to_dedicated_ldtk(args.out, args.ldtk, spec)

    if args.print_summary:
        px_wid, px_hei = derived_dims(len(main_ids), len(basement_ids))
        print(f"hall: {px_wid}x{px_hei} px")
        print(
            f"  main_hall entries: {len(main_ids)} "
            f"({main_floors_for(len(main_ids))} floors x {MAIN_SLOTS_PER_FLOOR} slots)"
        )
        print(
            f"  basement entries:  {len(basement_ids)} "
            f"({basement_rows_for(len(basement_ids))} rows x {BASEMENT_SLOTS_PER_ROW} slots)"
        )
        print(f"  spec written to:   {args.out}")
        if applied:
            print(f"  ldtk written to:   {args.ldtk}")
    return 0


def _apply_to_dedicated_ldtk(
    spec_path: Path, ldtk_path: Path, spec_data: dict[str, Any] | None = None
) -> bool:
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
    # Area authoring accepts JSON for generated specs.  Feed it the in-memory
    # spec through a temporary JSON file so this deterministic generator does
    # not require the optional native ``python-ron`` parser merely to read back
    # the RON it just wrote.  Hand-authored RON remains canonical on disk.
    temp_path: Path | None = None
    authoring_spec = spec_path
    if spec_data is not None:
        with tempfile.NamedTemporaryFile(
            mode="w", suffix=".json", prefix="hall_of_characters_", delete=False
        ) as file:
            json.dump(spec_data, file, indent=2)
            file.write("\n")
            temp_path = Path(file.name)
        authoring_spec = temp_path
    try:
        rc = area_authoring.main(
            [
                str(authoring_spec),
                "--ldtk",
                str(ldtk_path),
                "--replace-existing",
                "--no-repair",
            ]
        )
    finally:
        if temp_path is not None:
            temp_path.unlink(missing_ok=True)
    if rc != 0:
        print(f"error: area create into {ldtk_path} failed (rc={rc})", file=sys.stderr)
        return False
    return True


if __name__ == "__main__":
    sys.exit(main())
