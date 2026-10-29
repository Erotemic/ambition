#!/usr/bin/env python3
"""Author World 2-4 of Mary-O, the castle of the desert, into its LDtk world.

    python3 game/ambition_demo_mary_o/tools/author_mary_o_2_4_ldtk.py --preview
    python3 game/ambition_demo_mary_o/tools/author_mary_o_2_4_ldtk.py --apply

`--apply` REPLACES the level in the LDtk file. See `mary_o_level.py` for the
tile language and the command line.

The castle is one hall, as high as a view, with lava in each pit: lifts over
lava, bars of fire in pairs, a low hall of Void Stars, a long ride, and the
bridge of the Void Lizard. Its goal is the switch past the bridge
(`castle_end.rs`), as in the castle of the first world.
"""

from __future__ import annotations

from mary_o_level import T, Level, cli

WIDTH = 176
HEIGHT = 15
TOP = 2  # the first row under the rock of the roof
FLOOR = 12  # the row of the top of the floor
WALL = 2  # columns of rock at each end


def up(tiles: int) -> int:
    """The row whose top is `tiles` above the top of the floor."""
    return FLOOR - tiles


def build() -> Level:
    lv = Level(
        "mary_o_2_4",
        width=WIDTH,
        height=HEIGHT,
        floor=FLOOR,
        # 2-3 is at y 6400..7168.
        world=(0, 7360),
        bg_color="#14101c",
        fields={"biome": "void_castle", "parallax_theme": "void_castle", "music_track": "castle_crash_override", "mode": "mary_o"},
        late_fields={"next_room": "mary_o_1_1", "title": "WORLD 2-4"},
        coin_stem="keep_coin",
    )
    lv.rock(0, 0, WIDTH, TOP)
    lv.rock(0, FLOOR, WIDTH, HEIGHT - FLOOR)
    lv.rock(0, 0, WALL, HEIGHT)
    lv.rock(WIDTH - WALL, 0, WALL, HEIGHT)
    gate(lv)
    lifts(lv)
    hall_of_pairs(lv)
    low_hall(lv)
    the_ride(lv)
    garden(lv)
    bridge(lv)
    return lv


def pit(lv: Level, col: int, wide: int) -> None:
    """A pit of the castle: lava is in it."""
    lv.pit(col, wide)
    lv.lava(col, wide)


def gate(lv: Level) -> None:
    """Columns 2 to 20. A high ledge, steps down, a bar of fire, and the first block."""
    lv.rock(2, up(4), 6, 4)
    lv.start(4, "mary_o_2_4_start", on=up(4))
    for index in range(3):
        lv.column(8 + index, 1, 3 - index)
    lv.fire_bar(15, up(5), orbs=5, period=4.0)
    lv.block(19, up(4), "Question")


def lifts(lv: Level) -> None:
    """Columns 21 to 49. Two lines of lifts over lava, one down and one up, with an island between.

    The far side is a ledge six tiles up: the way onto it is the lift that goes up."""
    pit(lv, 22, 20)
    lv.lift_shaft("mary_o_2_4_lift_down", "Keep Lift Down", 24, down=True, speed=65.0, cars=3, chimney=TOP - 1)
    lv.rock(30, up(4), 3, 1)
    lv.coins(30, up(5), 3)
    lv.lift_shaft("mary_o_2_4_lift_up", "Keep Lift Up", 36, down=False, speed=65.0, cars=3, chimney=TOP - 1)
    lv.rock(42, up(6), 5, 6)
    lv.column(47, 1, 3)
    lv.column(48, 1, 1)


def hall_of_pairs(lv: Level) -> None:
    """Columns 50 to 76. Bars of fire in pairs that turn against each other, a pit between the pairs, and a snake."""
    lv.fire_bar(52, up(4), orbs=4, period=3.8)
    lv.block(56, up(4), "Question", "Coins")
    lv.fire_bar(60, up(4), orbs=4, period=3.8, clockwise=False, turn=0.5)
    pit(lv, 64, 3)
    lv.coin_arc(64, up(3), (0, 0.5, 0))
    lv.fire_bar(69, up(5), orbs=5, period=3.4)
    lv.fire_bar(74, up(5), orbs=5, period=3.4, clockwise=False, turn=0.25)
    lv.enemy("snake", 71.5)


def low_hall(lv: Level) -> None:
    """Columns 77 to 104. Rock that hangs low, pits under it, and Void Stars on the floor between the pits."""
    lv.rock(78, TOP, 26, 4)
    pit(lv, 81, 3)
    lv.enemy("star", 86)
    pit(lv, 89, 4)
    lv.coin_arc(89, up(2), (0, 0.5, 0.5, 0))
    lv.enemy("star", 95.5)
    lv.fire_bar(100, up(1), orbs=2, period=3.0)


def the_ride(lv: Level) -> None:
    """Columns 105 to 128. A lift over a long pit, with two bars of fire in the way of the ride."""
    lv.block(106, up(4), "Question")
    pit(lv, 109, 18)
    lv.platform("mary_o_2_4_ferry", "Keep Ferry", 109, FLOOR, sweep_dx=float((18 - 3) * T), speed=75.0)
    lv.fire_bar(115, up(5), orbs=4, period=3.4)
    lv.fire_bar(122, up(5), orbs=4, period=3.4, clockwise=False, turn=0.5)
    lv.coins(111, up(2), 3)
    lv.coins(118, up(2), 3)


def garden(lv: Level) -> None:
    """Columns 128 to 142. Two plants with a Void Star between them, and the last blocks before the bridge."""
    # The quasar, in a brick that looks like a brick.
    lv.block(129, up(4), "Brick")
    lv.block(130, up(4), "Brick", "AlwaysQuasar")
    lv.block(131, up(4), "Brick")
    lv.plant(132)
    lv.enemy("star", 136.5)
    lv.plant(138)


def bridge(lv: Level) -> None:
    """Columns 143 to 173. The bridge of planks, the Void Lizard on it, the switch, the gate and the robot.

    It is the bridge of the castle of the first world (see `bridge` of
    `author_mary_o_1_4_ldtk.py` for the reason of each part), two planks
    longer. The view is wider by the same part."""
    lv.column(143, 1, 3)
    lv.pit(144, 14)
    lv.lava(144, 14, under=1.75)
    lv.planks(144, FLOOR, 14)
    lv.column(158, 1, 3)
    lv.enemy("void_lizard", 151)
    lv.coins(145, up(4), 4, step=4)
    lv.switch(160)
    gate, cell, cell_wide = 162, 163, 5
    lv.rock(gate, TOP, 1, FLOOR - 3 - TOP)
    lv.gate(gate, 3)
    lv.rock(cell, TOP, cell_wide, FLOOR - 4 - TOP)
    lv.rock(cell + cell_wide, TOP, WIDTH - WALL - cell - cell_wide, FLOOR - TOP)
    lv.npc("mary_o_robot", 165.25)
    lv.arena("mary_o_2_4_bridge_view", "Bridge", 144, cell + cell_wide - 144, zoom=1.36, down=2.0)


if __name__ == "__main__":
    cli(build, __doc__)
