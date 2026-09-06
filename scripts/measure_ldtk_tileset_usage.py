#!/usr/bin/env python3
"""What do the LDtk worlds actually need the player spritesheet FOR?

`bevy_ecs_ldtk` loads every tileset a project declares. Five worlds declare
`sprite_player_robot_v3` with `relPath=../sprites/player_robot_v3_spritesheet.png`
— 3072x2468, 7.6 MP — and the runtime decodes it at boot on a road with no
demand stamp, then never draws it. The retarget to a cheaper tier is Jon's call
in the map submodule; this script establishes exactly what the retarget must
change, because it is NOT just the relPath.

WHAT IT ANSWERS
---------------

* Which tileset definitions name that path, per world.
* Whether any LEVEL LAYER uses the tileset (`layerInstances[].__tilesetDefUid`)
  — if one does, the art is drawn and a cheaper tier is a quality decision.
  If none does, it is an editor-preview-only asset and the tier is free.
* Which entity definitions reference it, and through which fields
  (`tilesetId`, `tileRect`, `uiTileRect`).

⛔⛔ AND WHY A BARE relPath SWAP IS WRONG. `tileRect` / `uiTileRect` are in
TILESET PIXEL coordinates, and the tileset def carries `pxWid`/`pxHei`/
`tileGridSize` describing the image it was authored against. Point the same
def at a quarter-size PNG and the 256x256 crop that framed one animation frame
now spans a third of the image. The tier PNGs are also NOT exact fractions
(0_25x of a 3072x2468 sheet is 832x653, not 768x617), so the rescale has no
clean integer factor and must be computed from the actual file headers.

ⓘ The def is ALREADY out of sync at full resolution: it declares pxHei 2484
against a 2468-pixel file. Whatever else the retarget does, it should fix that.

Usage:
    scripts/measure_ldtk_tileset_usage.py
    scripts/measure_ldtk_tileset_usage.py --json
"""

from __future__ import annotations

import argparse
import json
import struct
import subprocess
import sys
from pathlib import Path

REPO = Path(
    subprocess.run(
        ["git", "rev-parse", "--show-toplevel"], capture_output=True, text=True
    ).stdout.strip()
)
MAP_ASSETS = REPO / "game/ambition_map_assets"
ASSETS = REPO / "crates/ambition_platformer2d_actor_monolith/assets"
TARGET_REL = "../sprites/player_robot_v3_spritesheet.png"


def _shown(path: Path) -> str:
    """Repo-relative when it can be, absolute otherwise.

    ⛔ A bare `relative_to(REPO)` RAISES for anything outside the repo, and it
    was used in the REFUSAL MESSAGE — so the one path that reports "there are no
    worlds here" crashed with a ValueError instead of saying so, whenever the
    root it was told to look at lived elsewhere. A refusal that cannot be
    printed is not a refusal.
    """
    return str(path.relative_to(REPO) if path.is_relative_to(REPO) else path)


def png_size(path: Path) -> tuple[int, int] | None:
    """IHDR only — never decode; these are the images the finding is about."""
    try:
        header = path.open("rb").read(24)
    except OSError:
        return None
    if len(header) < 24 or header[:8] != b"\x89PNG\r\n\x1a\n":
        return None
    return struct.unpack(">II", header[16:24])


def worlds() -> list[Path]:
    if not MAP_ASSETS.is_dir():
        return []
    return sorted(MAP_ASSETS.rglob("*.ldtk"))


def inspect(world: Path) -> dict | None:
    """Every tileset in one world, with who uses it and how."""
    try:
        data = json.loads(world.read_text())
    except (OSError, json.JSONDecodeError):
        return None
    defs = data.get("defs", {})
    entities = defs.get("entities", [])
    # Layer USE is the question that decides whether the art is drawn. A level's
    # `layerInstances` may be inlined or held in a separate file per level
    # (`externalRelPath`), so absence here is only evidence when levels are
    # inlined — reported either way rather than assumed.
    levels = data.get("levels", [])
    external = [lvl for lvl in levels if lvl.get("externalRelPath")]
    layer_uids: set[int] = set()
    for level in levels:
        for layer in level.get("layerInstances") or []:
            uid = layer.get("__tilesetDefUid")
            if uid is not None:
                layer_uids.add(uid)

    rows = []
    for tileset in defs.get("tilesets", []):
        uid = tileset.get("uid")
        users = []
        for entity in entities:
            fields = [
                field
                for field in ("tilesetId", "tileRect", "uiTileRect")
                if (
                    entity.get(field) == uid
                    or (
                        isinstance(entity.get(field), dict)
                        and entity[field].get("tilesetUid") == uid
                    )
                )
            ]
            if fields:
                users.append(
                    {
                        "entity": entity.get("identifier"),
                        "fields": fields,
                        "tileRect": entity.get("tileRect"),
                    }
                )
        rows.append(
            {
                "uid": uid,
                "identifier": tileset.get("identifier"),
                "relPath": tileset.get("relPath"),
                "declared": [tileset.get("pxWid"), tileset.get("pxHei")],
                "tileGridSize": tileset.get("tileGridSize"),
                "used_by_a_layer": uid in layer_uids,
                "entity_users": users,
            }
        )
    return {
        # ⚠ BEST EFFORT, NOT `relative_to` OUTRIGHT. A bare `relative_to(REPO)`
        # raises `ValueError` for any world outside the repo, which made this
        # function impossible to exercise on a fixture — the test that would
        # have proved the layer-use answer could not call it at all.
        "world": _shown(world),
        "levels": len(levels),
        "levels_external": len(external),
        "tilesets": rows,
    }


def main(argv: list[str]) -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--json", action="store_true")
    args = ap.parse_args(argv)

    found = worlds()
    if not found:
        print(
            f"NO `.ldtk` WORLDS UNDER {_shown(MAP_ASSETS)}.\n"
            "⛔ That submodule may not be checked out — run\n"
            "   `git submodule update --init --recursive`. Absent is not zero."
        )
        return 2

    reports = [r for r in (inspect(w) for w in found) if r]
    if args.json:
        print(json.dumps(reports, indent=2))
        return 0

    print(f"{len(reports)} world(s) under {_shown(MAP_ASSETS)}\n")
    naming = 0
    for report in reports:
        interesting = [t for t in report["tilesets"] if t["relPath"] == TARGET_REL]
        mark = "  ← names the player sheet" if interesting else ""
        print(
            f"{report['world']:<62} {len(report['tilesets'])} tileset(s), "
            f"{report['levels']} level(s){mark}"
        )
        if report["levels_external"]:
            print(
                f"    ⚠ {report['levels_external']} level(s) store layers in a "
                "separate file; layer use below is only what this file shows"
            )
        for tileset in interesting:
            naming += 1
            print(
                f"    uid={tileset['uid']} {tileset['identifier']} "
                f"declared {tileset['declared'][0]}x{tileset['declared'][1]} "
                f"tileGridSize={tileset['tileGridSize']}"
            )
            print(
                f"    used by a level layer: "
                f"{'YES — the art is DRAWN' if tileset['used_by_a_layer'] else 'no'}"
            )
            for user in tileset["entity_users"]:
                rect = user["tileRect"] or {}
                print(
                    f"      entity {user['entity']}: {'+'.join(user['fields'])}"
                    f"  tileRect={rect.get('x')},{rect.get('y')},"
                    f"{rect.get('w')}x{rect.get('h')}"
                )

    actual = png_size(ASSETS / "sprites/player_robot_v3_spritesheet.png")
    print(f"\n{naming} tileset definition(s) name {TARGET_REL}")
    if actual:
        print(f"\nthe file they name is actually {actual[0]}x{actual[1]}")
        for tier in ["sprites_0_5x", "sprites_0_25x", "sprites_potato"]:
            size = png_size(ASSETS / tier / "player_robot_v3_spritesheet.png")
            if size:
                print(
                    f"  {tier:<16} {size[0]}x{size[1]}  "
                    f"(x{size[0] / actual[0]:.4f}, y{size[1] / actual[1]:.4f})"
                )
        print(
            "\n⛔ THE TIERS ARE NOT EXACT FRACTIONS and the x and y factors differ,\n"
            "   so a retarget cannot scale `tileRect` by a clean constant. Any patch\n"
            "   must recompute pxWid/pxHei/tileGridSize/tileRect/uiTileRect from the\n"
            "   real header of the file it points at."
        )
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
