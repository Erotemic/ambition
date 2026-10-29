#!/usr/bin/env python3
"""Author World 2-2 of Mary-O, the tomb under the desert, into its LDtk world.

    python3 game/ambition_demo_mary_o/tools/author_mary_o_2_2_ldtk.py --preview
    python3 game/ambition_demo_mary_o/tools/author_mary_o_2_2_ldtk.py --apply

`--apply` REPLACES the level in the LDtk file. See `mary_o_level.py` for the
tile language and the command line.

The level is one corridor under rock, as high as a view: a low passage, two
lines of lifts over a pit, a hall of fire bars, a hall of bricks, and a ferry.
"""

from __future__ import annotations

from mary_o_level import T, Level, cli

WIDTH = 170
HEIGHT = 15
TOP = 2  # the first row under the rock of the roof
FLOOR = 12  # the row of the top of the floor
WALL = 2  # columns of rock at each end


def up(tiles: int) -> int:
    """The row whose top is `tiles` above the top of the floor."""
    return FLOOR - tiles


def build() -> Level:
    lv = Level(
        "mary_o_2_2",
        width=WIDTH,
        height=HEIGHT,
        floor=FLOOR,
        # 2-1 is at y 4800..5568.
        world=(0, 5760),
        bg_color="#14101c",
        fields={"biome": "starless", "parallax_theme": "starless", "music_track": "embedded_theme", "mode": "mary_o"},
        late_fields={"next_room": "mary_o_2_3", "title": "WORLD 2-2"},
        coin_stem="tomb_coin",
    )
    lv.rock(0, 0, WIDTH, TOP)
    lv.rock(0, FLOOR, WIDTH, HEIGHT - FLOOR)
    lv.rock(0, 0, WALL, HEIGHT)
    lv.rock(WIDTH - WALL, 0, WALL, HEIGHT)
    arrival(lv)
    low_passage(lv)
    lifts(lv)
    hall_of_bars(lv)
    hall_of_bricks(lv)
    ferry(lv)
    goal(lv)
    return lv


def arrival(lv: Level) -> None:
    """Columns 2 to 20. A row of blocks and two walkers."""
    lv.start(4, "mary_o_2_2_start")
    lv.block(9, up(4), "Question")
    lv.block(10, up(4), "Brick")
    lv.block(11, up(4), "Question", "Coins")
    lv.enemy("slop", 14)
    lv.enemy("slop", 17)


def low_passage(lv: Level) -> None:
    """Columns 20 to 45. Rock that hangs low, two pits under it, and a Void Star between them."""
    lv.rock(22, TOP, 22, 5)
    lv.pit(27, 3)
    lv.coin_arc(27, up(2), (0, 0.5, 0))
    lv.enemy("star", 33)
    lv.pit(37, 3)
    lv.coin_arc(37, up(2), (0, 0.5, 0))


def lifts(lv: Level) -> None:
    """Columns 47 to 73. Two lines of lifts over a pit, one down and one up, with an island between.

    The far side is a ledge six tiles up: the way onto it is the lift that goes up."""
    lv.pit(48, 20)
    lv.lift_shaft("mary_o_2_2_lift_down", "Tomb Lift Down", 50, down=True, speed=60.0, cars=3, chimney=TOP - 1)
    lv.rock(56, up(4), 3, 1)
    lv.block(57, up(8), "Question", "Coins")
    lv.lift_shaft("mary_o_2_2_lift_up", "Tomb Lift Up", 62, down=False, speed=60.0, cars=3, chimney=TOP - 1)
    lv.rock(68, up(6), 6, 6)
    lv.coins(69, up(7), 4)
    # Steps down from the ledge.
    for col, tall in ((74, 4), (75, 2), (76, 1)):
        lv.column(col, 1, tall)


def hall_of_bars(lv: Level) -> None:
    """Columns 78 to 104. Three bars of fire, a snake, and a block."""
    lv.fire_bar(82, up(5), orbs=5, period=4.2)
    lv.coins(79, up(1), 6)
    lv.enemy("snake", 88)
    lv.fire_bar(93, up(4), orbs=4, period=3.8, clockwise=False, turn=0.25)
    lv.block(97, up(4), "Question")
    lv.fire_bar(101, up(4), orbs=4, period=3.8, turn=0.5)


def hall_of_bricks(lv: Level) -> None:
    """Columns 105 to 131. A roof of bricks with coins over it, two plants, and a Void Star."""
    lv.block(110, up(4), "Brick", "Coins5")
    lv.bricks(107, up(4), 10)
    lv.coins(107, up(5), 10)
    lv.plant(118)
    lv.enemy("star", 122)
    lv.enemy("slop", 124.5)
    lv.plant(127)


def ferry(lv: Level) -> None:
    """Columns 132 to 151. The pit that only the ferry crosses, with a bar of fire in the way of the ride."""
    lv.pit(133, 16)
    lv.platform("mary_o_2_2_ferry", "Tomb Ferry", 133, FLOOR, sweep_dx=float((16 - 3) * T), speed=80.0)
    lv.fire_bar(141, up(5), orbs=4, period=3.4)
    lv.coins(135, up(2), 3)
    lv.coins(144, up(2), 3)
    lv.enemy("paper", 146, up(6))


def goal(lv: Level) -> None:
    """Columns 151 to 167. A block, steps of rock, and the pole."""
    lv.block(152, up(4), "Question", "AlwaysWand")
    for index in range(4):
        lv.column(155 + index, 1, index + 1)
    lv.pole(163, 7)


if __name__ == "__main__":
    cli(build, __doc__)
