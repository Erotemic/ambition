#!/usr/bin/env python3
"""Author World 2-1 of Mary-O, the first level of the desert, into its LDtk world.

    python3 game/ambition_demo_mary_o/tools/author_mary_o_2_1_ldtk.py --preview
    python3 game/ambition_demo_mary_o/tools/author_mary_o_2_1_ldtk.py --apply

`--apply` REPLACES the level in the LDtk file. See `mary_o_level.py` for the
tile language and the command line.

The level is ground from end to end, with pits in it: dunes in steps, an oasis
of plants under swallows, a pyramid to go over, and a pit that only a ferry
crosses.
"""

from __future__ import annotations

from mary_o_level import T, Level, cli

WIDTH = 200
HEIGHT = 24
FLOOR = 20  # the row of the top of the floor


def up(tiles: int) -> int:
    """The row whose top is `tiles` above the top of the floor."""
    return FLOOR - tiles


def build() -> Level:
    lv = Level(
        "mary_o_2_1",
        width=WIDTH,
        height=HEIGHT,
        floor=FLOOR,
        # The warp room, the last level of the file before world 2, is at y 4096..4576.
        world=(0, 4800),
        bg_color="#68b2d6",
        fields={"biome": "stardune", "parallax_theme": "stardune", "music_track": "sparse_theme", "mode": "mary_o"},
        late_fields={"next_room": "mary_o_2_2", "title": "WORLD 2-1"},
        coin_stem="dune_coin",
    )
    lv.rock(0, FLOOR, WIDTH, HEIGHT - FLOOR)
    arrival(lv)
    dunes(lv)
    oasis(lv)
    pyramid(lv)
    crossing(lv)
    goal(lv)
    return lv


def arrival(lv: Level) -> None:
    """Columns 0 to 25. A row of blocks and two walkers, on ground with no pit in it."""
    lv.start(3, "mary_o_2_1_start")
    lv.block(10, up(4), "Question")
    lv.block(14, up(4), "Brick")
    lv.block(15, up(4), "Question", "Coins")
    lv.block(16, up(4), "Brick")
    lv.block(15, up(8), "Question", "Coins")
    lv.enemy("slop", 19)
    lv.enemy("slop", 23)


def dunes(lv: Level) -> None:
    """Columns 26 to 58. Two dunes in steps: one to go over, one that ends at a pit."""
    for col, wide, tall in ((27, 3, 1), (30, 3, 2), (33, 4, 3), (37, 3, 2), (40, 3, 1)):
        lv.column(col, wide, tall)
    lv.enemy("slop", 34.5, up(3))
    lv.coins(33, up(5), 4)
    lv.enemy("snake", 45)
    for index in range(4):
        lv.column(48 + 2 * index, 2, index + 1)
    lv.pit(56, 3)
    lv.coin_arc(56, up(5), (0, 0.5, 0))


def oasis(lv: Level) -> None:
    """Columns 59 to 92. Plants in pipes, a pit, a row of bricks, and swallows over all of it."""
    lv.plant(62)
    lv.enemy("swallow", 68, up(8))
    lv.pit(72, 4)
    lv.coin_arc(72, up(3), (0, 0.5, 0.5, 0))
    lv.block(80, up(4), "Question")
    lv.block(81, up(4), "Brick", "Coins5")
    lv.bricks(78, up(4), 6)
    lv.enemy("slop", 80)
    lv.plant(86)
    lv.enemy("swallow", 88, up(9))
    lv.enemy("slop", 91)


def pyramid(lv: Level) -> None:
    """Columns 93 to 125. A pyramid in steps, six tiles high, with a quasar over its top."""
    for index in range(6):
        lv.column(94 + 2 * index, 2, index + 1)
    lv.column(106, 6, 6)
    for index in range(6):
        lv.column(112 + 2 * index, 2, 6 - index)
    lv.block(107, up(10), "Brick")
    lv.block(108, up(10), "Brick", "AlwaysQuasar")
    lv.block(109, up(10), "Brick")
    lv.coins(96, up(4), 2)
    lv.coins(100, up(6), 2)
    # A Void Star on the top: the top is six tiles wide, with room to jump it.
    lv.enemy("star", 110, up(6))
    lv.enemy("snake", 116.5, up(4))


def crossing(lv: Level) -> None:
    """Columns 126 to 160. A pit that only the ferry crosses, with a swallow and a plane over the ride."""
    lv.pit(128, 18)
    lv.platform("mary_o_2_1_ferry", "Dune Ferry", 128, FLOOR, sweep_dx=float((18 - 3) * T), speed=75.0)
    lv.coins(131, up(2), 6, step=2)
    lv.enemy("swallow", 136, up(8))
    lv.enemy("paper", 142, up(6))
    lv.block(150, up(4), "Question", "AlwaysWand")
    lv.plant(154)
    lv.enemy("slop", 157)
    lv.enemy("slop", 158.5)


def goal(lv: Level) -> None:
    """Columns 160 to 199. A snake under bricks, steps of rock, and the pole."""
    lv.bricks(166, up(4), 4)
    lv.enemy("snake", 165)
    lv.enemy("slop", 172)
    for index in range(5):
        lv.column(178 + index, 1, index + 1)
    lv.pole(190, 9)


if __name__ == "__main__":
    cli(build, __doc__)
