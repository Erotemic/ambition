#!/usr/bin/env python3
"""Author World 2-3 of Mary-O, the mesas of the desert, into its LDtk world.

    python3 game/ambition_demo_mary_o/tools/author_mary_o_2_3_ldtk.py --preview
    python3 game/ambition_demo_mary_o/tools/author_mary_o_2_3_ldtk.py --apply

`--apply` REPLACES the level in the LDtk file. See `mary_o_level.py` for the
tile language and the command line.

The level has no floor from end to end: it is mesas, shelves and bridges of
brick over a fall. `GROUND` is the row of the top of the first mesa, and each
height is given from it.
"""

from __future__ import annotations

from mary_o_level import T, Level, cli

WIDTH = 190
HEIGHT = 24
GROUND = 16  # the row of the top of the first mesa


def up(tiles: int) -> int:
    """The row whose top is `tiles` above the top of the first mesa."""
    return GROUND - tiles


def build() -> Level:
    lv = Level(
        "mary_o_2_3",
        width=WIDTH,
        height=HEIGHT,
        floor=GROUND,
        # 2-2 is at y 5760..6240.
        world=(0, 6400),
        bg_color="#68b2d6",
        fields={"biome": "stardune", "parallax_theme": "stardune", "music_track": "sparse_theme", "mode": "mary_o"},
        late_fields={"next_room": "mary_o_2_4", "title": "WORLD 2-3"},
        coin_stem="mesa_coin",
    )
    first_mesa(lv)
    stepping_mesas(lv)
    shelves(lv)
    first_ferry(lv)
    lifts(lv)
    brick_bridges(lv)
    twin_ferries(lv)
    goal(lv)
    return lv


def mesa(lv: Level, col: int, wide: int, tall: int = 0) -> None:
    """Rock from `tall` tiles above the first mesa down to the bottom of the level."""
    lv.rock(col, up(tall), wide, HEIGHT - up(tall))


def first_mesa(lv: Level) -> None:
    """Columns 0 to 15. The blocks and a snake, on ground with no pit in it."""
    mesa(lv, 0, 16)
    lv.start(2, "mary_o_2_3_start")
    lv.block(6, up(4), "Question")
    lv.block(9, up(4), "Brick")
    lv.block(10, up(4), "Brick", "Coins5")
    lv.block(11, up(4), "Brick")
    lv.enemy("snake", 12)


def stepping_mesas(lv: Level) -> None:
    """Columns 16 to 46. Mesas of a few tiles at heights that go up and come down, under a swallow."""
    for col, wide, tall in ((19, 3, 1), (25, 3, 3), (31, 3, 5), (37, 3, 3), (43, 4, 1)):
        mesa(lv, col, wide, tall)
    lv.coin_arc(16, up(3), (0, 0.5, 0))
    lv.coin_arc(22, up(5), (0, 0.5, 0))
    lv.coin_arc(28, up(7), (0, 0.5, 0))
    lv.enemy("slop", 26, up(3))
    lv.enemy("swallow", 32, up(11))
    lv.enemy("slop", 44.5, up(1))


def shelves(lv: Level) -> None:
    """Columns 47 to 79. Shelves that go up to a high mesa, a bridge of bricks down from it, and a plane."""
    lv.one_way(49, up(3), 3)
    lv.one_way(54, up(5), 3)
    lv.one_way(59, up(7), 3)
    lv.coins(59, up(8), 3)
    mesa(lv, 64, 5, 7)
    lv.block(66, up(11), "Question")
    lv.bricks(71, up(5), 5)
    lv.enemy("paper", 72, up(9))
    mesa(lv, 77, 3, 3)


def first_ferry(lv: Level) -> None:
    """Columns 80 to 101. A ferry over a fall that no jump crosses, and a mesa with a plant on it."""
    lv.platform("mary_o_2_3_ferry_a", "Mesa Ferry", 80, up(3), sweep_dx=float((16 - 3) * T), speed=85.0)
    lv.coins(83, up(5), 5, step=2)
    lv.enemy("cartesian", 88, up(8))
    mesa(lv, 96, 6, 3)
    lv.plant(98, on=up(3))


def lifts(lv: Level) -> None:
    """Columns 102 to 122. Two lines of lifts over the fall: one that goes down, then one that goes up to a high mesa."""
    lv.lift_shaft("mary_o_2_3_lift_down", "Mesa Lift Down", 104, down=True, speed=60.0, cars=4)
    lv.rock(110, up(1), 3, 1)
    lv.coins(110, up(2), 3)
    lv.lift_shaft("mary_o_2_3_lift_up", "Mesa Lift Up", 115, down=False, speed=60.0, cars=4)
    mesa(lv, 120, 4, 6)


def brick_bridges(lv: Level) -> None:
    """Columns 124 to 151. Two bridges of bricks with a hole between them, a snake and walkers on them, and a Void Star."""
    lv.bricks(125, up(4), 9)
    lv.bricks(138, up(4), 8)
    lv.enemy("snake", 127, up(4))
    lv.enemy("slop", 131, up(4))
    lv.block(141, up(8), "Question", "Coins")
    lv.block(142, up(8), "Question")
    lv.coin_arc(134, up(6), (0, 0.5, 0.5, 0))
    lv.enemy("star", 143, up(4))
    lv.enemy("swallow", 136, up(11))
    mesa(lv, 148, 4, 3)


def twin_ferries(lv: Level) -> None:
    """Columns 152 to 172. Two ferries, the second higher than the first, with planes in their way."""
    lv.platform("mary_o_2_3_ferry_b", "Low Ferry", 152, up(3), sweep_dx=float((9 - 3) * T), speed=80.0)
    lv.coins(154, up(5), 3, step=2)
    lv.enemy("paper", 157, up(7))
    lv.platform("mary_o_2_3_ferry_c", "High Ferry", 162, up(5), sweep_dx=float((10 - 3) * T), speed=95.0)
    lv.coins(164, up(7), 4, step=2)
    lv.enemy("cartesian", 168, up(9))


def goal(lv: Level) -> None:
    """Columns 172 to 189. The last mesa: steps of rock down to the pole."""
    mesa(lv, 172, 18, 0)
    for index in range(4):
        mesa(lv, 172 + index, 1, 4 - index)
    lv.enemy("snake", 178)
    lv.pole(184, 8)


if __name__ == "__main__":
    cli(build, __doc__)
