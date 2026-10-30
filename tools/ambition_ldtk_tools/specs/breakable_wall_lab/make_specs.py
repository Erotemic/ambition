#!/usr/bin/env python3
"""Write the breakable-wall test bed in the sandbox: a corridor shut at its
east end by a wall that breaks under hits and stays broken in the save
(`district_kit.breakable_wall`), and the small room behind it.

    python specs/breakable_wall_lab/make_specs.py           # writes the specs
    python specs/breakable_wall_lab/make_specs.py --apply   # and makes the rooms

Run it from `tools/ambition_ldtk_tools`. The app test
`game/ambition_app/tests/a_broken_wall_stays_broken.rs` plays it.

The corridor is 160 high, the height of the wall and of its look
(`castle_cracked_wall`, 64 x 160), so no jump goes over it: the way east is
through the wall.
"""
from __future__ import annotations

import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent))

from district_kit import WALL, WORLDS, breakable_wall, edge, make_rooms, room, sign, solid, start  # noqa: E402

WORLD = WORLDS / "sandbox.ldtk"
GENERATOR = "specs/breakable_wall_lab/make_specs.py"
LAB = "breakable_wall_lab"
SECRET = "breakable_wall_lab_secret"
WALL_ID = "breakable_wall_lab_wall"
W, SW, H, FLOOR, CEILING = 960, 640, 288, 224, 64
#: The wall: its left edge, and its size (the look's).
WALL_X, WALL_W = 640, 64
#: East of every other sandbox room, the secret room's west edge on the
#: lab's east edge.
AT = (86016, 0)


def shell(width, west_open: bool, east_open: bool):
    """Floor, ceiling and the closed ends of a corridor room."""
    parts = [
        solid("floor", 0, FLOOR, width, H - FLOOR),
        solid("ceiling", 0, 0, width, CEILING),
    ]
    if not west_open:
        parts.append(solid("west wall", 0, CEILING, WALL, FLOOR - CEILING))
    if not east_open:
        parts.append(solid("east wall", width - WALL, CEILING, WALL, FLOOR - CEILING))
    return parts


def specs() -> dict[str, str]:
    lab = shell(W, west_open=False, east_open=True) + [
        start(LAB, 120, FLOOR),
        sign("lab sign", 320, CEILING + 16, "Hit the cracked wall until it breaks"),
        breakable_wall(WALL_ID, WALL_X, CEILING, WALL_W, FLOOR - CEILING, look="castle_cracked_wall", hp=3,
                       name="the cracked wall"),
        edge("lab_east", "to the room behind the wall", W - 16, FLOOR, SECRET, "secret_west"),
    ]
    secret = shell(SW, west_open=True, east_open=False) + [
        start(SECRET, SW - 120, FLOOR),
        edge("secret_west", "back to the lab", 0, FLOOR, LAB, "lab_east"),
    ]
    common = dict(biome="lab", music="tech_bros_disruption", bg="#1c1a22", generator=GENERATOR)
    return {
        LAB: room(LAB, lab, at=AT, size=(W, H), title="Breakable wall lab", **common),
        SECRET: room(SECRET, secret, at=(AT[0] + W, AT[1]), size=(SW, H), **common),
    }


def main(argv: list[str]) -> int:
    paths = []
    for rid, text in specs().items():
        path = HERE / f"{rid}.ron"
        path.write_text(text)
        paths.append(path)
        print(f"wrote {path}")
    if "--apply" in argv:
        return make_rooms(WORLD, paths)
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
