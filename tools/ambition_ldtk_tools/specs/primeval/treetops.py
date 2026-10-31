#!/usr/bin/env python3
"""The treetops over the primeval run: a loop over the trail.

    python specs/primeval/treetops.py          # writes the .ron spec
    python specs/primeval/treetops.py --apply  # and makes the room, and
                                               # cuts the ways in

Run it from `tools/ambition_ldtk_tools`.

Jon (2026-10-30): "think of how hollow knight / silk song do it ... the
world is dense ... it mixes between them"; outdoors, "climbs, towers and
drops" (`docs/planning/game/world-composition.md`). The trail was a row of
rooms; this puts a road over it:

- **Up:** the great tree in the run (x 896..960): a rope ladder up its
  trunk, out of the run's top, into the treetops through a hatch.
- **Along:** the canopy's leaves are walked on; rope bridges between the
  crowns; the canopy research station (abandoned: the pterosaurs); a
  pterosaur's nest on the highest crown, one egg hatched (a chest there).
- **Down:** a gap in the leaves at the east end, over the rookery: drop
  through into it (no way back up: a shortcut on along the trail).

The run and the rookery are edited in place (their generator, `make_specs.py`,
makes doors that `side_exits.py` turns to edges): their ceilings are cut,
their ways in added. The art is `targets/props/primeval_treetops_props.py`.
"""
from __future__ import annotations

import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent))

from district_kit import WALL, WORLDS, add_entities, entity, ledge, make_rooms, paint, room, run, sign, solid, start  # noqa: E402
from district_scenery import body_box, piece  # noqa: E402

WORLD = WORLDS / "dinosaur.ldtk"
GENERATOR = "specs/primeval/treetops.py"
RUN = "primeval_run"
ROOKERY = "primeval_rookery"
TREETOPS = "primeval_treetops"
#: Where the run and the rookery are in the world, their floor (y 480) and
#: their ceiling (to y 16).
RUN_X, ROOKERY_X, TRAIL_FLOOR, TRAIL_CEILING = 21600, 24800, 480, 16
#: The hole in the run's ceiling over the great tree (x0, x1).
TREE = (896, 960)
#: The treetops: right over the run and the rookery, its foot on their tops.
W, H, G = 2880, 576, 512
AT = (RUN_X + TREE[0] - 96, -H)
HATCH = (TREE[0] + RUN_X - AT[0], TREE[1] + RUN_X - AT[0])
#: The gap down into the rookery (treetops x), and where it is in the
#: rookery.
GAP = (2720, 2800)
ROOKERY_GAP = (GAP[0] + AT[0] - ROOKERY_X, GAP[1] + AT[0] - ROOKERY_X)
#: The platforms (x0, x1, y), walked on; the art draws them.
PLATFORMS = ((280, 560, 400), (640, 960, 336), (1040, 1360, 400), (1440, 1760, 288), (1840, 2160, 384), (2240, 2560, 320))
BOX = {"stochastic_parrot": (36, 32), "owl": (36, 48), "vulture": (44, 66), "compsognathus": (24, 26),
       "european_swallow": (24, 20), "npc_raptor_stalker": (48, 80)}


def enemy(n, character, x, feet):
    w, h = BOX[character]
    return entity("EnemySpawn", x, feet - h, w, h, name=f"{TREETOPS}_{character}_{n}", brain="combatant",
                  character_id=character, respawn="OnRoomReenter")


def treetops():
    h0, h1 = HATCH
    g0, g1 = GAP
    nest = PLATFORMS[3]
    e = [
        piece("treetops_west", 16, 60, name="treetops_west"),
        piece("treetops_east", 1440, 60, name="treetops_east"),
        solid("leaves_west", 0, G, h0, H - G),
        solid("leaves", h1, G, g0 - h1, H - G),
        solid("leaves_east", g1, G, W - g1, H - G),
        solid("west_wall", 0, 0, WALL, G),
        solid("east_wall", W - WALL, 0, WALL, G),
        # The hatch at the great tree's top: come up through it and stand on
        # it; drop through it (down and jump) to go down.
        ledge("tree_hatch", h0, G, h1 - h0),
        entity("LoadingZone", h0, H - 16, h1 - h0, 16, id="treetops_trunk", name="down the great tree to the run",
               activation="EdgeExit", target_room=RUN, target_zone="run_great_tree", bidirectional=True),
        entity("LoadingZone", g0, H - 16, g1 - g0, 16, id="treetops_drop", name="down through the leaves to the rookery",
               activation="EdgeExit", target_room=ROOKERY, target_zone="rookery_treetops", bidirectional=True),
        start(TREETOPS, h1 + 140, G),
        *[ledge(f"platform_{i}", x0, y, x1 - x0) for i, (x0, x1, y) in enumerate(PLATFORMS)],
        entity("ChestSpawn", nest[0] + 180, nest[2] - 58, 76, 58, name="what the hatchling left", reward="currency:70"),
        sign("treetops_sign_station", 420, 200, "CANOPY RESEARCH STATION: unmanned since the pterosaurs", 440),
        sign("treetops_sign_nest", 1600, 150, "NEST: occupied (recently)", 300),
        sign("treetops_sign_gap", g0 - 60, 380, "MIND THE GAP (the penguins will)", 320),
        enemy(1, "stochastic_parrot", 760, 336 - 80),
        enemy(2, "stochastic_parrot", 1180, 400 - 90),
        enemy(1, "owl", 1960, 384 - 100),
        enemy(1, "vulture", 2380, 320 - 120),
        enemy(1, "compsognathus", 1500, G),
        enemy(2, "compsognathus", 1560, G),
        enemy(3, "compsognathus", 1620, G),
        enemy(1, "npc_raptor_stalker", 2280, G),
    ]
    return room(TREETOPS, e, at=AT, size=(W, H), biome="primeval", music="dinosaur_liberators", bg="#16200F",
                title="THE TREETOPS", generator=GENERATOR, dangerous=True)


def tree_yaml() -> str:
    """The run's great tree (its art), and the way up out of the run's top."""
    bx, by, bw, bh = body_box("run_great_tree")
    fx, fy = TREE[0] - 120, 0
    t0, t1 = TREE
    return f"""# The run's great tree, up to the treetops. Written by {GENERATOR}.
level_id: {RUN}
entities:
  - type: Prop
    px: [{round(fx + bx / 2)}, {round(fy + by / 2)}]
    size: [{max(1, round(bw / 2))}, {max(1, round(bh / 2))}]
    fields:
      name: run_great_tree
      kind: run_great_tree
  - type: LoadingZone
    px: [{t0}, 0]
    size: [{t1 - t0}, 16]
    fields:
      id: run_great_tree
      name: up the great tree to the treetops
      activation: EdgeExit
      target_room: {TREETOPS}
      target_zone: treetops_trunk
      bidirectional: true
  - type: DebugLabel
    px: [{t1 + 40}, 200]
    size: [300, 24]
    fields:
      name: run_sign_great_tree
      text: "THE GREAT TREE: rangers' ladder. Mind the parrots."
      category: Custom
"""


def gap_yaml() -> str:
    g0, g1 = ROOKERY_GAP
    return f"""# The rookery's way in from the treetops, through the leaves. Written by {GENERATOR}.
level_id: {ROOKERY}
entities:
  - type: LoadingZone
    px: [{g0}, 0]
    size: [{g1 - g0}, 16]
    fields:
      id: rookery_treetops
      name: up into the treetops
      activation: EdgeExit
      target_room: {TREETOPS}
      target_zone: treetops_drop
      bidirectional: true
"""


def main(argv):
    path = HERE / f"{TREETOPS}.ron"
    path.write_text(treetops())
    print(path.relative_to(HERE.parents[1]))
    if "--apply" not in argv:
        return 0
    # The treetops name the run's and the rookery's ways, which are added
    # after it: only the end state is checked (the last `validate`).
    make_rooms(WORLD, [path], keep_position="--relayout" not in argv)
    # The check after the first add refuses the half-joined world (the
    # rookery's way is the second): only the second add's code counts.
    add_entities(tree_yaml(), WORLD, HERE / ".tree.yaml", unless='"up the great tree to the treetops"')
    if code := add_entities(gap_yaml(), WORLD, HERE / ".gap.yaml", unless='"up into the treetops"'):
        return code
    t0, t1 = TREE
    r0, r1 = ROOKERY_GAP
    h0, _ = HATCH
    for args in ((RUN, "Collision", 0, t0, 0, t1 - t0, TRAIL_CEILING),
                 (ROOKERY, "Collision", 0, r0, 0, r1 - r0, TRAIL_CEILING),
                 # The rangers' ladder from the run's floor up out of its top,
                 # and up out of the hatch's hole in the treetops.
                 (RUN, "Climbable", 1, t0 + 16, 0, 32, TRAIL_FLOOR),
                 (TREETOPS, "Climbable", 1, h0 + 16, G - 80, 32, H - G + 80)):
        if code := paint(WORLD, *args):
            return code
    code, _ = run("validate", str(WORLD))
    return code


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
