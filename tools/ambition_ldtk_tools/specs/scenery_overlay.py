#!/usr/bin/env python3
"""Add scenery to rooms whose LDtk level is edited in place.

    python specs/scenery_overlay.py           # prints what it would add
    python specs/scenery_overlay.py --apply   # adds what is missing

Run it from `tools/ambition_ldtk_tools`.

Some rooms' generators are older than their levels: the level was edited in
the LDtk after (gate terminals, water, enemies tuned), so regenerating the
room would undo that (2026-10-30: `specs/primeval/make_specs.py --apply`
moved and dropped enemies, a gate terminal and the water). Their scenery is
ADDED instead: one `Prop` per piece of art, over the art's body box, each
added once (by its name).
"""
from __future__ import annotations

import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))

from district_kit import WORLDS, add_entities  # noqa: E402
from district_scenery import body_box  # noqa: E402

GENERATOR = "specs/scenery_overlay.py"

#: World file -> level -> [(kind, frame x, frame y)], furthest back first. The
#: numbers are the art's (its `PIECES` frames); change them with it.
OVERLAYS = {
    "dinosaur.ldtk": {
        "primeval_tyrant_hall": [("primeval_tyrant_hall_room", 0, 0)],
    },
    "pvn.ldtk": {
        "ninja_dojo": [("shadow_dojo_hall", 0, 0)],
    },
    "evil.ldtk": {
        "evil_gate_forest": [("evil_forest_roadside", 430, 260)],
    },
}


def overlay_yaml(level_id: str, pieces: list[tuple[str, int, int]], px: float = 2.0) -> str:
    rows = []
    for kind, fx, fy in pieces:
        bx, by, bw, bh = body_box(kind)
        rows.append(f"""  - type: Prop
    px: [{round(fx + bx / px)}, {round(fy + by / px)}]
    size: [{max(1, round(bw / px))}, {max(1, round(bh / px))}]
    fields:
      name: {kind}_overlay
      kind: {kind}""")
    return f"# Scenery of {level_id}. Written by {GENERATOR}.\nlevel_id: {level_id}\nentities:\n" + "\n".join(rows) + "\n"


def main(argv) -> int:
    for world, levels in OVERLAYS.items():
        ldtk = WORLDS / world
        for level_id, pieces in levels.items():
            text = overlay_yaml(level_id, pieces)
            first = f'"{pieces[0][0]}_overlay"'
            if "--apply" not in argv:
                print(f"{world} {level_id}: {'present' if first in ldtk.read_text() else 'would add'} {len(pieces)} piece(s)")
                continue
            if code := add_entities(text, ldtk, HERE / f".{level_id}_scenery.yaml", unless=first):
                return code
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
