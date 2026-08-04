#!/usr/bin/env python3
"""Author game/ambition_demo_mary_o/assets/worlds/mary_o.ldtk.

Jon, 2026-08-04: *"I would like to make maryo an ldtk level so I can manually
play with it and lay it out."* This is the file he lays out.

Generated through the sanctioned `ambition_ldtk_tools` pipeline (world init ->
area create -> entity add -> repair -> validate); the .ldtk JSON is never
hand-edited. Re-run from the repo root:

    python3 game/ambition_demo_mary_o/tools/author_mary_o_ldtk.py

⛔ **THE CONSTANTS BELOW ARE A ONE-TIME MIRROR OF `lib.rs`, NOT A SECOND
AUTHORITY.** They exist to reproduce today's 1-1 exactly, so the migration can be
checked (`the_ldtk_room_is_the_room_the_constants_built`). Once that probe is
green and Jon starts editing, **the .ldtk file is the level** and these numbers
are history — do not "keep them in sync", delete the ones the Rust no longer
needs. A generator that stays authoritative after the author takes over is just
the old hardcoded level wearing a different extension.

## Two kinds of block, and why the split is not arbitrary

`area create` LOWERS `Solid` / `OneWayPlatform` entities into the Collision
IntGrid — which is what terrain wants, because an IntGrid-derived block carries
`GeoSource::TileLayer` and that is what selects the TILED art path. An
entity-authored surface takes the stretched 128px entity texture, which down a
104-tile ground run is exactly the smear that path exists to avoid.

But lowering to IntGrid **eats the name**, and an IntGrid cell's id is a
row-major merge ordinal that renumbers the moment you paint a neighbouring cell.
So anything the runtime has to RECOGNISE is added afterwards with `entity add`
and keeps its name — the same rule Sanic's generator states for its monitor
boxes, for the same reason.

    terrain        -> `area create`, lowered to IntGrid, tiled art, no identity
    special blocks -> `entity add`, stays an entity, keeps its NAME

## The authored vocabulary (what Jon types in the GUI)

A block's name IS its meaning — the engine has no typed channel for a game's own
nouns, so this is the convention (see
`docs/planning/proposal-authored-vocabulary-2026-08-04.md` §4):

    MaryOBlock (kind: Power|Quasar|Brick)   the reactive blocks — a FIELD, not
                                            a name; see `ldtk_vocabulary.rs`
    warp_pipe_<link>_up      a pipe whose mouth points UP (you press DOWN on it)
    warp_pipe_<link>_down    a pipe whose mouth points DOWN (you fall out of it)
    goal_pole{,_knob,_banner}   the flag; touching the shaft ends the level
    vault_wall_<n> / vault_floor   the secret chamber's masonry

⚠ two pipes with the same `<link>` are a PAIR, and a link that is not exactly two
members is refused at load. That check is the mitigation for pairing-by-name; a
typo pairs nothing and has to be loud.
"""

from __future__ import annotations

import json
import subprocess
import sys
import tempfile
from pathlib import Path

REPO = Path(__file__).resolve().parents[3]
TOOLS = REPO / "tools" / "ambition_ldtk_tools"
TARGET = REPO / "game" / "ambition_demo_mary_o" / "assets" / "worlds" / "mary_o.ldtk"

# ── Mirrored from game/ambition_demo_mary_o/src/lib.rs (see the header) ─────
T = 32
GROUND_TILES = 2
LEVEL_W = 104 * T
SURFACE_H = 15 * T
VAULT_DEPTH_TILES = 9
LEVEL_H = SURFACE_H + VAULT_DEPTH_TILES * T
GROUND_TOP = SURFACE_H - GROUND_TILES * T

# Ground runs, as [from_tile, to_tile). The gaps between them are the pits.
GROUND_RUNS = [(0, 20), (22, 42), (45, 60), (65, 104)]

POWER_BLOCK_COLUMNS = [6, 30, 60]
QUASAR_BLOCK_COLUMNS = [10, 50, 70]
BRICK_COLUMNS = [48, 49, 50]
BONUS_ROW = 4  # tiles above the ground top, shared by all three families

STAIR_FIRST_COLUMN = 74
STAIR_STEPS = 4
STAIR_GAP_TILES = 6

POLE_COLUMN = 98
POLE_WIDTH = T // 2

PIPE_COLUMN = 26
EXIT_PIPE_COLUMN = 35
PIPE_WIDTH_TILES = 3
PIPE_HEIGHT_TILES = 2

VAULT_LEFT = 23 * T
VAULT_WIDTH = 18 * T
VAULT_CEILING = SURFACE_H
VAULT_FLOOR = SURFACE_H + (VAULT_DEPTH_TILES - 2) * T
VAULT_COINS = 8

# ⛔ **MEASURED, not derived, and it cannot be otherwise.** A vault pipe hangs
# from the chamber ceiling and its lip must sit where a TALL Mary-O standing
# under it can touch — `powerups::tall_body_size().y + 0.25 * T`. That body size
# comes from her SPRITE SHEET at runtime (`form_body_size(TALL_SHEET_TARGET)`),
# so a Python generator has no way to compute it.
#
# ⚠ I guessed 48 the first time and the equivalence probe caught it — 24 cells of
# pipe hanging too low. The real number is `Vec2(25.6, 67.2)`, printed by the
# `#[ignore]`d `print_the_sprite_derived_numbers_the_generator_has_to_mirror`.
# **Re-run that after any change to her tall sheet.**
#
# ⭐ and this is exactly the kind of thing the file should stop deriving: once the
# pipe is authored, its length is Jon's to draw, and what protects it is the
# load-time check that the lip is REACHABLE — not a number copied between two
# languages. That check is owed.
TALL_BODY_H = 67.2
VAULT_PIPE_CLEARANCE = TALL_BODY_H + T / 4


def rect(etype: str, px: tuple[int, int], size: tuple[int, int], **fields) -> dict:
    entry: dict = {"type": etype, "px": list(px), "size": list(size)}
    if fields:
        entry["fields"] = fields
    return entry


def stair_steps() -> list[tuple[int, int]]:
    """Every step as `(tile column, height in tiles)` — both halves interleaved,
    exactly as `lib.rs::stair_steps` builds them."""
    far = STAIR_FIRST_COLUMN + STAIR_STEPS + STAIR_GAP_TILES
    out: list[tuple[int, int]] = []
    for step in range(1, STAIR_STEPS + 1):
        out.append((STAIR_FIRST_COLUMN + step - 1, step))
        out.append((far + STAIR_STEPS - step, STAIR_STEPS + 1 - step))
    return out


def terrain() -> list[dict]:
    """Everything that lowers to IntGrid: no identity, tiled art, paintable."""
    out = [
        rect("PlayerStart", (2 * T, GROUND_TOP - 2 * T), (28, 46), name="mary_o_start"),
    ]
    for i, (start, end) in enumerate(GROUND_RUNS):
        out.append(
            rect(
                "Solid",
                (start * T, GROUND_TOP),
                ((end - start) * T, GROUND_TILES * T),
                name=f"ground_{i}",
            )
        )
    # The teach platform, over SAFE ground at jump height, and the widest pit's
    # stepping stone — the same jump, now load-bearing.
    out.append(
        rect("OneWayPlatform", (12 * T, GROUND_TOP - 4 * T), (3 * T, T // 2), name="teach_platform")
    )
    out.append(
        rect(
            "OneWayPlatform",
            (62 * T, GROUND_TOP - 3 * T),
            (T, T // 2),
            name="pit_c_stepping_stone",
        )
    )
    for step, (column, height) in enumerate(stair_steps()):
        out.append(
            rect(
                "Solid",
                (column * T, GROUND_TOP - height * T),
                (T, height * T),
                name=f"stair_{step}",
            )
        )
    return out


def vault_masonry() -> list[dict]:
    """The chamber's floor and side walls. NAMED entities rather than IntGrid:
    they wear their own stone colour, which the demo attaches by name, and an
    IntGrid cell has no name to attach it to."""
    wall = T
    return [
        rect(
            "Solid",
            (VAULT_LEFT - wall, VAULT_FLOOR),
            (VAULT_WIDTH + wall * 2, wall),
            name="vault_floor",
        ),
        rect(
            "Solid",
            (VAULT_LEFT - wall, VAULT_CEILING),
            (wall, VAULT_FLOOR - VAULT_CEILING),
            name="vault_wall_0",
        ),
        rect(
            "Solid",
            (VAULT_LEFT + VAULT_WIDTH, VAULT_CEILING),
            (wall, VAULT_FLOOR - VAULT_CEILING),
            name="vault_wall_1",
        ),
    ]


def vault_coins() -> list[dict]:
    """The vault's reward, on the shared economy's `currency` channel — the same
    one Sanic's rings take. No demo collection code, and they land in the HUD's
    COINS readout for free."""
    size = int(0.75 * T)
    coin_y = int(VAULT_FLOOR - 1.5 * T)
    out = []
    for i in range(VAULT_COINS):
        cx = VAULT_LEFT + (1 + i * 2) * T
        out.append(
            rect(
                "PickupSpawn",
                (cx, coin_y),
                (size, size),
                name=f"vault_coin_{i}",
                kind="currency:1",
            )
        )
    return out


def bonus_blocks() -> list[dict]:
    """The three reactive-block families, as `MaryOBlock` entities.

    ⭐ **authored by a FIELD, not by a name.** These used to be `Solid` entities
    called `power_block_0` and friends — the index mattered and the spelling
    mattered, and adding a fourth meant knowing the convention. A `MaryOBlock`
    carries `kind`, which is a dropdown in the editor, and the converter derives
    the runtime name from it.

    ⚠ they MUST stay entities either way: `area create`'s IntGrid lowering would
    eat the identity the runtime recognises them by."""
    y = GROUND_TOP - BONUS_ROW * T
    out = []
    for column in POWER_BLOCK_COLUMNS:
        out.append(rect("MaryOBlock", (column * T, y), (T, T), kind="Power"))
    for column in QUASAR_BLOCK_COLUMNS:
        out.append(rect("MaryOBlock", (column * T, y), (T, T), kind="Quasar"))
    for column in BRICK_COLUMNS:
        out.append(rect("MaryOBlock", (column * T, y), (T, T), kind="Brick"))
    return out


def warp_pipes() -> list[dict]:
    """Two tubes through the ground slab, each authored as two halves at the same
    column: a SURFACE half standing on the ground with its mouth UP, and a VAULT
    half hanging from the chamber ceiling with its mouth DOWN.

    The `<link>` in the name is the pairing. `descent` is the one you press DOWN
    on; `ascent` is the one you press UP under."""
    surface_h = PIPE_HEIGHT_TILES * T
    vault_h = int(VAULT_FLOOR - VAULT_CEILING - VAULT_PIPE_CLEARANCE)
    width = PIPE_WIDTH_TILES * T
    return [
        rect(
            "Solid",
            (PIPE_COLUMN * T, GROUND_TOP - surface_h),
            (width, surface_h),
            name="warp_pipe_descent_up",
        ),
        rect(
            "Solid",
            (PIPE_COLUMN * T, VAULT_CEILING),
            (width, vault_h),
            name="warp_pipe_descent_down",
        ),
        rect(
            "Solid",
            (EXIT_PIPE_COLUMN * T, VAULT_CEILING),
            (width, vault_h),
            name="warp_pipe_ascent_down",
        ),
        rect(
            "Solid",
            (EXIT_PIPE_COLUMN * T, GROUND_TOP - surface_h),
            (width, surface_h),
            name="warp_pipe_ascent_up",
        ),
    ]


def goal() -> list[dict]:
    """Three pieces so the goal READS as a flagpole rather than a bar: a shaft, a
    finial capping it, and a banner off the top. All three are the same width and
    column, so none of them changes what is reachable or where the grab band is.

    ONE-WAY, not solid: a solid pole stops the body a half-body-width short of its
    own centre, so the grab could only ever fire from above the top."""
    pole_x = POLE_COLUMN * T
    pole_top = GROUND_TOP - 9 * T
    return [
        rect("OneWayPlatform", (pole_x, pole_top), (POLE_WIDTH, 9 * T), name="goal_pole"),
        rect(
            "OneWayPlatform",
            (pole_x, pole_top - POLE_WIDTH),
            (POLE_WIDTH, POLE_WIDTH),
            name="goal_pole_knob",
        ),
        rect(
            "OneWayPlatform",
            (pole_x, pole_top + POLE_WIDTH),
            (POLE_WIDTH, POLE_WIDTH * 2),
            name="goal_pole_banner",
        ),
    ]


def area_spec() -> dict:
    return {
        "id": "mary_o_1_1",
        "level_id": "mary_o_1_1",
        "world_x": 0,
        "world_y": 0,
        "px_wid": LEVEL_W,
        "px_hei": LEVEL_H,
        "fill_collision": "empty",
        "bg_color": "#5c94fc",
        "entities": terrain() + vault_coins(),
    }


def named_blocks() -> dict:
    """Everything added AFTER `area create`, so its name survives."""
    return {
        "level_id": "mary_o_1_1",
        "entities": bonus_blocks() + warp_pipes() + goal() + vault_masonry(),
    }


def run_tool(*args: str) -> None:
    cmd = [sys.executable, "-m", "ambition_ldtk_tools", *args]
    print("::", " ".join(str(a) for a in args))
    env = {"PYTHONPATH": str(TOOLS), "PATH": "/usr/bin:/bin"}
    result = subprocess.run(cmd, cwd=REPO, env=env)
    if result.returncode != 0:
        sys.exit(f"tool step failed: {' '.join(args)}")


def main() -> None:
    # ⛔⛔ **THIS SCRIPT DESTROYS THE LEVEL, so it refuses to run twice.**
    #
    # It deletes `mary_o.ldtk` and rebuilds it from the mirrored constants at the
    # top of this file. That is exactly right ONCE, to bootstrap the migration,
    # and it is catastrophic the moment Jon has opened the project and moved
    # anything: every block he dragged, every enemy he placed, every level he
    # added is gone, and the generator cheerfully reports success.
    #
    # ⚠ **the danger is that re-running a generator FEELS safe** — it is what you
    # do after editing the layout constants, and those constants are still sitting
    # right there at the top of this file looking authoritative. They are history.
    # The .ldtk is the level.
    #
    # So: the file existing is a REFUSAL, and overwriting it has to be typed out
    # in full. `--regenerate` is deliberately not `-f`.
    if TARGET.exists() and "--regenerate" not in sys.argv:
        sys.exit(
            f"REFUSED: {TARGET.relative_to(REPO)} already exists.\n"
            "\n"
            "This script REBUILDS the level from the Rust constants at the top of "
            "it and would discard every edit made in the LDtk editor. The .ldtk "
            "file is the level now; these constants are how it was bootstrapped.\n"
            "\n"
            "  • to change the layout: edit it in LDtk, not here\n"
            "  • to genuinely start over and lose the authored file:\n"
            f"      python3 {Path(__file__).relative_to(REPO)} --regenerate\n"
            "\n"
            "⚠ commit first. `git checkout` cannot bring back what was never "
            "committed."
        )
    TARGET.parent.mkdir(parents=True, exist_ok=True)
    if TARGET.exists():
        print(f"!! REGENERATING {TARGET.relative_to(REPO)} — discarding authored edits")
        TARGET.unlink()
    run_tool("world", "init", str(TARGET), "--identifier", "ambition-mary-o-world")
    with tempfile.TemporaryDirectory() as tmp:
        area = Path(tmp) / "mary_o_1_1_area.json"
        area.write_text(json.dumps(area_spec(), indent=2))
        run_tool("area", "create", str(area), "--ldtk", str(TARGET))
        # ⛔ **DEFS AFTER THE FIRST LEVEL, BEFORE THE ENTITIES THAT NEED THEM.**
        # `world init` clones the sandbox's definitions, which do not include
        # Mary-O's own nouns — but `def register-entity` VALIDATES, and an
        # empty project fails validation ("project has no levels"). So the
        # window is exactly here: after `area create` has made a level out of
        # standard entities, and before `entity add` places the `MaryOBlock`s
        # that need the definition to exist.
        run_tool(
            "def",
            "register-entity",
            str(Path(__file__).resolve().parent / "mary_o_entities.json"),
            "--ldtk",
            str(TARGET),
            "--in-place",
            # Mary-O's noun, not the engine's — see the flag's help.
            "--game-owned",
        )
        named = Path(tmp) / "mary_o_1_1_named.json"
        named.write_text(json.dumps(named_blocks(), indent=2))
        run_tool("entity", "add", str(named), "--ldtk", str(TARGET), "--in-place")
    run_tool("repair", str(TARGET), "--in-place")
    run_tool("validate", str(TARGET))
    print(f"authored {TARGET.relative_to(REPO)}")


if __name__ == "__main__":
    main()
