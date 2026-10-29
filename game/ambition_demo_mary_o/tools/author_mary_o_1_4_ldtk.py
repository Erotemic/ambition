#!/usr/bin/env python3
"""Author World 1-4 of Mary-O, the castle, into its LDtk world.

    python3 game/ambition_demo_mary_o/tools/author_mary_o_1_4_ldtk.py --preview
    python3 game/ambition_demo_mary_o/tools/author_mary_o_1_4_ldtk.py --apply

`--apply` REPLACES the level in the LDtk file. See `mary_o_level.py` for the
tile language and the command line.

The castle is a closed hall of bars of fire over pits of lava, and at its end a
bridge with the Void Lizard on it. The void has taken the castle: where a void
thing is, the hall has no colour. The quasar gives it back. A castle has no
pole: a switch is past the bridge. The bridge falls when the switch is taken,
the Void Lizard falls with it, and the robot that the void held thanks her.
"""

from __future__ import annotations

from mary_o_level import T, Level, cli

WIDTH = 170
TOP = 2  # the first row of the hall
FLOOR = 12  # the row of the top of the floor
SLAB = 3  # rows of rock in the floor
HEIGHT = FLOOR + SLAB
WALL = 2  # columns of rock at each end


def up(tiles: int) -> int:
    """The row whose top is `tiles` above the top of the floor."""
    return FLOOR - tiles


def build() -> Level:
    lv = Level(
        "mary_o_1_4",
        width=WIDTH,
        height=HEIGHT,
        floor=FLOOR,
        # 1-2 is at y 2560..3232.
        world=(0, 3392),
        bg_color="#14101c",
        fields={"biome": "void_castle", "parallax_theme": "void_castle", "music_track": "castle_crash_override", "mode": "mary_o"},
        late_fields={"next_room": "mary_o_2_1", "title": "WORLD 1-4"},
        coin_stem="castle_coin",
    )
    lv.rock(0, 0, WIDTH, TOP)
    lv.rock(0, FLOOR, WIDTH, SLAB)
    lv.rock(0, 0, WALL, HEIGHT)
    lv.rock(WIDTH - WALL, 0, WALL, HEIGHT)

    gate(lv)
    hall_of_bars(lv)
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
    """Columns 2 to 22. Steps down from a high ledge, a pit, and the first block."""
    lv.rock(2, up(4), 6, 4)
    lv.start(4, "mary_o_1_4_start", on=up(4))
    for index in range(3):
        lv.column(8 + index, 1, 3 - index)
    pit(lv, 14, 3)
    lv.coin_arc(14, up(3), (0, 0.5, 0))
    lv.block(20, up(4), "Question")


def hall_of_bars(lv: Level) -> None:
    """Columns 22 to 62. Bars of fire: one over a head, one over a pit, two that turn against each other."""
    # The first bar turns four tiles up: a small body walks under it at all
    # times, a tall one waits for its turn.
    lv.fire_bar(27, up(5), orbs=5, period=4.0)
    lv.coins(24, up(1), 7)
    # The second is over a pit of three tiles: the jump is with the bar away.
    pit(lv, 35, 3)
    lv.fire_bar(36, up(6), orbs=5, period=3.6, clockwise=False, turn=0.25)
    lv.enemy("star", 43)
    # Two that turn against each other, with a block between them.
    lv.fire_bar(48, up(4), orbs=4, period=4.0)
    lv.block(52, up(4), "Question", "Coins")
    lv.fire_bar(56, up(4), orbs=4, period=4.0, clockwise=False, turn=0.5)
    lv.enemy("snake", 60)


def low_hall(lv: Level) -> None:
    """Columns 62 to 92. Rock that hangs low, pits under it, and bars on the floor to jump."""
    lv.rock(64, TOP, 26, 4)
    pit(lv, 68, 3)
    # A bar that turns about a block on the floor: it comes over the floor.
    lv.fire_bar(75, up(1), orbs=2, period=3.2)
    pit(lv, 80, 4)
    lv.coin_arc(80, up(2), (0, 0.5, 0.5, 0))
    lv.fire_bar(87, up(1), orbs=2, period=3.2, clockwise=False, turn=0.5)


def the_ride(lv: Level) -> None:
    """Columns 92 to 116. A lift over a long pit, with a bar of fire in the way of the ride."""
    lv.block(93, up(4), "Question")
    pit(lv, 96, 16)
    lv.platform("mary_o_1_4_ferry", "Castle Ferry", 96, FLOOR, sweep_dx=float((16 - 3) * T), speed=70.0)
    # A body that stands on the lift is under the bar. A body that jumps for
    # the coins is in it.
    lv.fire_bar(104, up(5), orbs=4, period=3.4)
    lv.coins(98, up(2), 3)
    lv.coins(107, up(2), 3)


def garden(lv: Level) -> None:
    """Columns 116 to 136. Two plants with walkers between them, and the last block before the bridge."""
    # The quasar, in a brick that looks like a brick. The void of the garden
    # and of the bridge is in its time for a body that does not wait.
    lv.block(115, up(4), "Brick")
    lv.block(116, up(4), "Brick", "AlwaysQuasar")
    lv.block(117, up(4), "Brick")
    lv.plant(118)
    lv.enemy("star", 123)
    lv.enemy("slop", 125.5)
    lv.plant(128)
    lv.block(134, up(4), "Question")


def bridge(lv: Level) -> None:
    """Columns 138 to 167. The bridge of planks, the Void Lizard on it, the switch, the gate and the robot.

    A post of rock at each end of the bridge turns the Void Lizard, so it stays on
    the bridge. A post is three tiles high: its hop does not go over it, and a
    jump of the player does. Past the bridge is the switch, and a wall with a
    gate in it: there is no way on but the switch. A body that lands on the
    switch breaks the bridge, plank by plank from the switch, and the lava
    takes the Void Lizard. Then the gate opens, and she walks to the robot in
    the cell behind it."""
    lv.column(138, 1, 3)
    # The lava of this pit is lower than that of the others: the bridge is
    # read as a bridge by what shows under it.
    lv.pit(139, 12)
    lv.lava(139, 12, under=1.75)
    lv.planks(139, FLOOR, 12)
    lv.column(151, 1, 3)
    lv.enemy("void_lizard", 145)
    lv.coins(140, up(4), 4, step=3)
    lv.switch(153)
    # The wall is all the rock from the gate to the end of the level. The cell
    # of the robot is cut in it: five tiles wide and four high.
    gate, cell, cell_wide = 155, 156, 5
    lv.rock(gate, TOP, 1, FLOOR - 3 - TOP)
    lv.gate(gate, 3)
    lv.rock(cell, TOP, cell_wide, FLOOR - 4 - TOP)
    lv.rock(cell + cell_wide, TOP, WIDTH - WALL - cell - cell_wide, FLOOR - TOP)
    lv.npc("mary_o_robot", 158.25)
    # One view for the fight and its end: from the first plank to the end of
    # the cell. It starts ON the bridge, so the view stays with her until she
    # is over the first post (Jon, 2026-10-12: "the camera shifts to the bridge
    # too soon"). The fall of the bridge and of the boss is seen from the
    # switch (Jon, 2026-10-11: "you need to see the void boss swallowed by the
    # lava, and see the bridge break").
    lv.arena("mary_o_1_4_bridge_view", "Bridge", 139, cell + cell_wide - 139, zoom=1.25, down=2.0)


if __name__ == "__main__":
    cli(build, __doc__)
