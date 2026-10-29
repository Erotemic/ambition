#!/usr/bin/env python3
"""Author the warp room of Mary-O into its LDtk world: a room of pipes, one to the start of each level.

    python3 game/ambition_demo_mary_o/tools/author_mary_o_warp_ldtk.py --preview
    python3 game/ambition_demo_mary_o/tools/author_mary_o_warp_ldtk.py --apply

`--apply` REPLACES the level in the LDtk file. See `mary_o_level.py` for the
tile language and the command line.

The room is a secret. The way in is the sky of 1-1: a `LoadingZone` on the top
line of that level brings a body here (Jon, 2026-10-12: "if I fly too high in
world 1-1 I trigger a special loading zone to a special level with warp
pipes"). The game puts a zone that is on the top line of a level some tiles
OVER the level (`lift_the_sky_zones`), so a body must go up out of the view to
be in it. `--apply` puts the zone in 1-1 first, one time: 1-1 has no script of
its own.

A new level gets a pipe here: add its id to `LEVELS`. The game writes the name
of the level over its pipe (`pipe_signs.rs`).
"""

from __future__ import annotations

import json
import tempfile
from pathlib import Path

from mary_o_level import TARGET, T, Level, cli, run_tool

AREA = "mary_o_warp"
# The levels a pipe goes to, in the sequence of the pipes.
LEVELS = (
    "mary_o_1_1",
    "mary_o_1_2",
    "mary_o_1_3",
    "mary_o_1_4",
    "mary_o_2_1",
    "mary_o_2_2",
    "mary_o_2_3",
    "mary_o_2_4",
)

WALL = 2  # columns of rock at each end
FIRST_PIPE = 9  # the column of the first pipe
PIPE_STEP = 7  # columns from one pipe to the next
WIDTH = FIRST_PIPE + PIPE_STEP * (len(LEVELS) - 1) + 3 + 6
HEIGHT = 15
FLOOR = 12

# The zone of this room where the trip from the sky of 1-1 ends.
ARRIVAL_ZONE = "mary_o_warp_from_the_sky"
# The zone at the top of 1-1, and the level it is in.
SKY_ZONE = "mary_o_1_1_sky"
SKY_LEVEL = "mary_o_1_1"
SKY_TALL = 16  # px


def build() -> Level:
    lv = Level(
        AREA,
        width=WIDTH,
        height=HEIGHT,
        floor=FLOOR,
        # 1-4, the last level of the file, is at y 3392..3872.
        world=(0, 4096),
        bg_color="#5c94fc",
        fields={"biome": "starlight", "parallax_theme": "starlight", "mode": "mary_o"},
        late_fields={"title": "WARP ZONE"},
        coin_stem="warp_coin",
    )
    lv.rock(0, FLOOR, WIDTH, HEIGHT - FLOOR)
    lv.rock(0, 0, WALL, HEIGHT)
    lv.rock(WIDTH - WALL, 0, WALL, HEIGHT)
    lv.start(4, "mary_o_warp_start")
    # She comes down out of the sky, to the left of the first pipe.
    lv.loading_zone(ARRIVAL_ZONE, "From the sky", 3, 1, 3, 3)
    for index, level in enumerate(LEVELS):
        lv.pipe(FIRST_PIPE + PIPE_STEP * index, level, "Warp")
    return lv


def sky_door() -> None:
    """Put the zone at the top of 1-1 that brings a body here, if 1-1 does not have it."""
    project = json.loads(TARGET.read_text())
    level = next(level for level in project["levels"] if level["identifier"] == SKY_LEVEL)
    for layer in level["layerInstances"]:
        for entity in layer.get("entityInstances", []):
            ids = [field["__value"] for field in entity["fieldInstances"] if field["__identifier"] == "id"]
            if entity["__identifier"] == "LoadingZone" and ids == [SKY_ZONE]:
                return
    zone = {
        "type": "LoadingZone",
        "px": [0, 0],
        "size": [level["pxWid"], SKY_TALL],
        "fields": {
            "id": SKY_ZONE,
            "name": "Sky",
            "activation": "Walk",
            "target_room": AREA,
            "target_zone": ARRIVAL_ZONE,
            "bidirectional": False,
        },
    }
    with tempfile.TemporaryDirectory() as tmp:
        spec = Path(tmp) / "sky.json"
        spec.write_text(json.dumps({"level_id": SKY_LEVEL, "entities": [zone]}, indent=2))
        # The file is not checked here: the zone names this room, and this
        # room goes into the file after it. `apply` checks the two together.
        run_tool("entity", "add", str(spec), "--ldtk", str(TARGET), "--in-place", "--no-repair")


if __name__ == "__main__":
    cli(build, __doc__, before_apply=sky_door)
