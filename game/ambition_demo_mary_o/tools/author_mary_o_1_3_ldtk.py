#!/usr/bin/env python3
"""Author World 1-3 of Mary-O, the level in the sky, into its LDtk world.

    python3 game/ambition_demo_mary_o/tools/author_mary_o_1_3_ldtk.py --preview
    python3 game/ambition_demo_mary_o/tools/author_mary_o_1_3_ldtk.py --apply

`--apply` REPLACES the level in the LDtk file. See `mary_o_level.py` for the
tile language and the command line.

The level has no floor from end to end: it is islands, pillars and shelves over
a fall. `GROUND` is the row of the top of the first island, and each height is
given from it.
"""

from __future__ import annotations

from mary_o_level import T, Level, cli

WIDTH = 208
HEIGHT = 24
GROUND = 16  # the row of the top of the first island

# The `id` of the long ferry.
FERRY_ID = "mary_o_1_3_ferry"


def up(tiles: int) -> int:
    """The row whose top is `tiles` above the top of the first island."""
    return GROUND - tiles


def build() -> Level:
    lv = Level(
        "mary_o_1_3",
        width=WIDTH,
        height=HEIGHT,
        floor=GROUND,
        # 1-1 is at y 0..768 and 1-2 at y 2560..3232.
        world=(0, 1600),
        bg_color="#5c94fc",
        fields={"biome": "starlight", "parallax_theme": "starlight", "mode": "mary_o"},
        late_fields={"next_room": "mary_o_1_4", "title": "WORLD 1-3"},
        coin_stem="sky_coin",
    )
    first_island(lv)
    stepping_pillars(lv)
    first_ferry(lv)
    the_climb(lv)
    brick_bridge(lv)
    twin_ferries(lv)
    goal(lv)
    return lv


def island(lv: Level, col: int, wide: int, tall: int = 0) -> None:
    """Ground from `tall` tiles above the first island down to the bottom of the level."""
    lv.rock(col, up(tall), wide, HEIGHT - up(tall))


def first_island(lv: Level) -> None:
    """Columns 0 to 17. The blocks, a walker and a snake, on ground with no pit in it."""
    island(lv, 0, 18)
    lv.start(2, "mary_o_1_3_start")
    lv.block(7, up(4), "Question")
    lv.block(9, up(4), "Brick")
    lv.block(10, up(4), "Brick", "Coins5")
    lv.block(11, up(4), "Brick")
    lv.enemy("slop", 9)
    # The first plant of the game, on ground with room to go round it by a jump.
    lv.plant(13)


def stepping_pillars(lv: Level) -> None:
    """Columns 18 to 43. Pillars of a few tiles at heights that go up, with coins in the line of each jump."""
    for col, wide, tall in ((21, 3, 1), (27, 2, 2), (32, 3, 3), (38, 2, 2)):
        island(lv, col, wide, tall)
    lv.coin_arc(18, up(3), (0, 0.5, 0))
    lv.coin_arc(24, up(4), (0, 0.5, 0))
    lv.coin_arc(29, up(5), (0, 0.5, 0))
    lv.enemy("slop", 33, up(3))
    lv.enemy("paper", 30, up(8))
    lv.one_way(35, up(6), 3)
    lv.coins(35, up(7), 3)
    island(lv, 43, 4, 1)


def first_ferry(lv: Level) -> None:
    """Columns 47 to 66. A ferry over a fall that no jump crosses, with a plane in its way."""
    lv.platform("mary_o_1_3_ferry_a", "Star Ferry", 47, up(1), sweep_dx=float((16 - 3) * T), speed=80.0)
    lv.coins(50, up(3), 5, step=2)
    # The first swallow, over a ride: the ferry does not stop for a coconut.
    lv.enemy("swallow", 56, up(7))
    island(lv, 63, 6, 1)
    lv.block(65, up(5), "Question")


def the_climb(lv: Level) -> None:
    """Columns 69 to 104. Shelves that go up to a high road, and a low road of pillars under it.

    The high road has the coins and a quasar; the low road has the enemies. A
    fall from the high road is onto the low road where a pillar is, and a death
    where one is not."""
    island(lv, 69, 5, 2)
    lv.one_way(75, up(4), 3)
    lv.one_way(79, up(6), 3)
    lv.one_way(83, up(8), 4)
    lv.coins(83, up(9), 4)
    island(lv, 88, 6, 8)
    lv.block(88, up(12), "Brick", "AlwaysQuasar")
    lv.plant(90, on=up(8))
    lv.one_way(95, up(7), 3)
    lv.coins(95, up(8), 3)
    lv.one_way(99, up(5), 3)
    lv.coins(99, up(6), 3)
    # The low road.
    island(lv, 78, 4, 1)
    lv.enemy("slop", 79, up(1))
    island(lv, 96, 5, 0)
    lv.enemy("slop", 98)
    island(lv, 103, 5, 2)


def brick_bridge(lv: Level) -> None:
    """Columns 108 to 139. A bridge of bricks with holes in it, a snake and a line of walkers on it.

    A shell that is kicked goes down the bridge and into a hole."""
    lv.bricks(108, up(2), 10)
    lv.bricks(121, up(2), 8)
    lv.bricks(132, up(2), 8)
    lv.enemy("snake", 110, up(2))
    for col in (113.5, 123, 126):
        lv.enemy("slop", col, up(2))
    lv.block(125, up(6), "Question", "Coins")
    lv.block(126, up(6), "Question")
    lv.coin_arc(118, up(4), (0, 0.5, 0))
    lv.coin_arc(129, up(4), (0, 0.5, 0))
    lv.enemy("paper", 119, up(7))
    # The first Void Star, on a bridge with room to jump it.
    lv.enemy("star", 136, up(2))


def twin_ferries(lv: Level) -> None:
    """Columns 140 to 181. Two ferries, the second higher than the first, with an island between them."""
    lv.platform(FERRY_ID, "Cinder Ferry", 141, up(2), sweep_dx=float((15 - 3) * T), speed=80.0)
    lv.coins(144, up(4), 5, step=2)
    lv.enemy("cartesian", 149, up(6))
    island(lv, 156, 5, 3)
    lv.enemy("star", 158, up(3))
    lv.platform("mary_o_1_3_ferry_c", "High Ferry", 162, up(4), sweep_dx=float((16 - 3) * T), speed=95.0)
    lv.coins(165, up(6), 6, step=2)
    lv.enemy("swallow", 168, up(9))
    lv.enemy("paper", 174, up(7))


def goal(lv: Level) -> None:
    """Columns 178 to 207. The last island: steps of rock down to the pole."""
    island(lv, 178, 30, 0)
    for index in range(4):
        island(lv, 178 + index, 1, 4 - index)
    lv.plant(185)
    lv.enemy("snake", 190)
    lv.block(191, up(4), "Question", "AlwaysWand")
    for index in range(4):
        lv.column(194 + index, 1, index + 1)
    lv.pole(201, 8)


if __name__ == "__main__":
    cli(build, __doc__)
