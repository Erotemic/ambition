#!/usr/bin/env python3
"""Author World 1-2 of Mary-O, the level under the ground, into its LDtk world.

    python3 game/ambition_demo_mary_o/tools/author_mary_o_1_2_ldtk.py --preview
    python3 game/ambition_demo_mary_o/tools/author_mary_o_1_2_ldtk.py --apply

`--apply` REPLACES the level in the LDtk file. See `mary_o_level.py` for the
tile language and the command line.
"""

from __future__ import annotations

from mary_o_level import T, Level, cli

WIDTH = 240
# Rows, from the top: the rock over the top road, the top road, the roof of
# the corridor (one row of bricks: it is the floor of the top road), the
# corridor, the floor.
ATTIC = 2  # the first row of the top road
ATTIC_FLOOR = 8  # the row of the roof: its top is the floor of the top road
TOP = 9  # the first row of the corridor
FLOOR = 19  # the row of the top of the floor
SLAB = 2  # rows of rock in the floor
HEIGHT = FLOOR + SLAB
WALL = 2  # columns of rock at each end

# The `id` of the ferry, which `level_1_2.rs` and the app tests read.
FERRY_ID = "mary_o_1_2_ferry"


def up(tiles: int) -> int:
    """The row whose top is `tiles` above the top of the floor."""
    return FLOOR - tiles


def build() -> Level:
    lv = Level(
        "mary_o_1_2",
        width=WIDTH,
        height=HEIGHT,
        floor=FLOOR,
        # 1-1 is at y 0..768 and 1-3 at y 1600..2368.
        world=(0, 2560),
        bg_color="#14101c",
        fields={"biome": "starless", "parallax_theme": "starless", "music_track": "embedded_theme", "mode": "mary_o"},
        late_fields={"next_room": "mary_o_1_3", "title": "WORLD 1-2"},
        coin_stem="cavern_coin",
    )

    # The shell: rock from the top of the level down to the corridor, a floor,
    # and a wall at each end. The roof has no hole in its top row.
    lv.rock(0, 0, WIDTH, TOP)
    # The top road: open from end to end over the roof (Jon, 2026-10-11: "the
    # ceiling should be made of breakable blocks so you can run on top of the
    # stage if you can get up there").
    lv.carve(WALL, ATTIC, WIDTH - 2 * WALL, ATTIC_FLOOR - ATTIC)
    lv.rock(0, FLOOR, WIDTH, SLAB)
    lv.rock(0, 0, WALL, HEIGHT)
    lv.rock(WIDTH - WALL, 0, WALL, HEIGHT)
    # The camera stays on the corridor: a room this high would show the top
    # road over each part of it.
    lv.camera_zone("mary_o_1_2_corridor", "Corridor", TOP - 1, HEIGHT - (TOP - 1))

    arrival(lv)
    shell_alley(lv)
    brick_maze(lv)
    teeth_and_pits(lv)
    lifts(lv)
    attic(lv)
    clock_gallery(lv)
    ferry(lv)
    goal(lv)
    roof(lv)
    return lv


def roof(lv: Level) -> None:
    """The roof of the corridor: one row of bricks from end to end.

    A body that can break a brick and can get its head to the roof goes up
    through it and runs on the top road. Rock stays under and over a pipe. The
    lines of the lifts stay open. Over the steps of the goal the roof has a
    hole: the way down from the top road."""
    rock = set(range(175, 178)) | set(range(188, 191))
    hole = {227, 228}
    for col in range(WALL, WIDTH - WALL):
        if lv.grid[ATTIC_FLOOR][col] != "#" or col in rock:
            continue
        lv.carve(col, ATTIC_FLOOR, 1, 1)
        if col not in hole:
            lv.block(col, ATTIC_FLOOR, "Brick")


def arrival(lv: Level) -> None:
    """Columns 2 to 32. A row of blocks, two walkers, and steps of rock with a walker in a hollow."""
    lv.start(4, "mary_o_1_2_start")
    lv.rock(13, TOP, 4, 1)
    lv.block(9, up(4), "Question")
    lv.block(10, up(4), "Question", "Coins")
    lv.block(11, up(4), "Question", "Coins")
    lv.block(12, up(4), "Brick", "Coins5")
    lv.block(13, up(4), "Question", "Coins")
    lv.enemy("slop", 16)
    lv.enemy("slop", 18.5)

    lv.column(20, 2, 1)
    lv.column(22, 2, 2)
    lv.column(24, 2, 3)
    lv.enemy("slop", 27)  # in the hollow, columns 26 to 28
    lv.coin_arc(26, up(5), (0, 1, 0))
    lv.column(29, 2, 3)
    lv.column(31, 2, 4)
    lv.rock(24, TOP, 9, 1)


def shell_alley(lv: Level) -> None:
    """Columns 33 to 58. A snake in front of a line of walkers, under a band of bricks.

    A shell that is kicked goes down the alley and comes back off the step at
    its end. The top of the band is a second road with coins on it: the way
    onto it is the last step of the arrival."""
    lv.bricks(36, up(5), 17)
    lv.coins(38, up(6), 13)
    lv.enemy("snake", 37)
    for col in (41.5, 44.5, 47.5, 50.5):
        lv.enemy("slop", col)
    lv.column(57, 2, 2)


def brick_maze(lv: Level) -> None:
    """Columns 59 to 84. A mass of bricks with a tunnel through it.

    Two ways through: under the mass with the enemies, or through the tunnel
    with the coins. The floor of the tunnel starts two columns before the mass:
    the way onto that porch is the step before it. A hole in the floor of the
    tunnel joins the two ways."""
    for row in (TOP, TOP + 1):
        lv.bricks(61, row, 18)
    lv.block(64, up(4), "Question", "Coins")
    lv.block(75, up(4), "Question", "Coins")
    for row in (up(5), up(4)):
        lv.bricks(59, row, 10)
        lv.bricks(71, row, 8)
    lv.coins(63, up(6), 7, step=2)
    lv.enemy("slop", 66)
    lv.enemy("snake", 72)
    lv.enemy("slop", 77)
    lv.block(82, up(4), "Question")


def teeth_and_pits(lv: Level) -> None:
    """Columns 85 to 123. Three pits under rock that hangs low, with planes in the air.

    The low rock makes each jump a short one. The third pit has a pillar in it."""
    lv.rock(90, TOP, 7, 3)
    lv.pit(92, 3)
    lv.coin_arc(92, up(3), (0, 0.5, 0))
    lv.enemy("paper", 98, up(6))
    lv.rock(100, TOP, 6, 4)
    lv.pit(101, 4)
    lv.coin_arc(101, up(3), (0, 0.5, 0.5, 0))
    lv.enemy("snake", 107)
    lv.enemy("cartesian", 108, up(7))
    lv.pit(110, 7)
    lv.pillar(113, 1, 2)
    lv.coin(113, up(4))
    lv.rock(111, TOP, 5, 2)
    lv.enemy("slop", 119)
    lv.block(121, up(4), "Hidden", "AlwaysQuasar")


def lifts(lv: Level) -> None:
    """Columns 124 to 149. Two lines of lifts over a pit, one down and one up, with an island between.

    The far side is a ledge six tiles up: the way onto it is the lift that goes
    up. That lift goes on through a hole in the roof, into the attic."""
    lv.pit(124, 20)
    lv.lift_shaft("mary_o_1_2_lift_down", "Lift Down", 126, down=True, speed=60.0, cars=4, chimney=TOP - 1)
    lv.rock(132, up(4), 3, 1)
    lv.block(133, up(8), "Question", "Coins")
    lv.lift_shaft("mary_o_1_2_lift_up", "Lift Up", 138, down=False, speed=60.0, cars=4, chimney=TOP - 1)
    lv.rock(144, up(6), 6, 6)


def attic(lv: Level) -> None:
    """The part of the top road over the clock gallery, columns 136 to 179. Coins, and a pipe that comes out before the ferry.

    The lift that goes up comes to it, and it is a way past the clock gallery."""
    lv.coins(144, ATTIC_FLOOR - 1, 28, stem="attic_coin")
    for col in (146, 154, 162):
        lv.coin_arc(col, ATTIC_FLOOR - 3, (0, 0.5, 0.5, 0))
    lv.block(151, ATTIC_FLOOR - 4, "Question")
    lv.block(159, ATTIC_FLOOR - 4, "Brick", "Coins5")
    lv.block(167, ATTIC_FLOOR - 4, "Question", "Coins")
    lv.pipe(175, "attic_descent", "Entrance", on=ATTIC_FLOOR)
    lv.pipe(188, "attic_descent", "Exit", under=TOP)


def clock_gallery(lv: Level) -> None:
    """Columns 150 to 193. A high road of ledges and one-way shelves over a low road.

    A fall from the high road is a walk on the low road, not a death."""
    lv.one_way(151, up(6), 4)
    lv.coins(151, up(7), 4)
    lv.rock(157, up(5), 5, 1)
    lv.enemy("slop", 160, up(5))
    lv.bricks(164, up(6), 2)
    lv.block(166, up(6), "Brick", "Coins5")
    lv.bricks(167, up(6), 2)
    lv.enemy("paper", 166, up(8))
    lv.rock(171, up(5), 6, 1)
    lv.enemy("snake", 174, up(5))
    lv.one_way(179, up(6), 4)
    lv.coins(179, up(7), 4)
    lv.enemy("cartesian", 181, up(3))
    # The low road. Its blocks are between the ledges, with open air over them.
    for col in (156, 164, 170, 180):
        lv.enemy("slop", col)
    lv.block(162, up(4), "Question", "Coins")
    lv.block(178, up(4), "Question")


def ferry(lv: Level) -> None:
    """Columns 194 to 215. The pit that only the ferry crosses, with an island of bricks over its middle."""
    lv.pit(194, 22)
    lv.platform(FERRY_ID, "Underground Ferry", 194, FLOOR, sweep_dx=float((22 - 3) * T), speed=90.0)
    lv.bricks(203, up(6), 1)
    lv.block(204, up(6), "Question", "AlwaysWand")
    lv.bricks(205, up(6), 1)
    lv.coin_arc(197, up(4), (0, 0.5, 0.5, 0))
    lv.coin_arc(209, up(4), (0, 0.5, 0.5, 0))
    lv.enemy("paper", 200, up(7))
    lv.enemy("cartesian", 207, up(4))
    lv.enemy("paper", 213, up(7))
    lv.rock(198, TOP, 3, 2)
    lv.rock(210, TOP, 3, 2)


def goal(lv: Level) -> None:
    """Columns 216 to 237. A snake, steps of rock, and the pole."""
    lv.enemy("snake", 220)
    for index in range(4):
        lv.column(224 + index, 1, index + 1)
    lv.pole(232, 7)


if __name__ == "__main__":
    cli(build, __doc__)
