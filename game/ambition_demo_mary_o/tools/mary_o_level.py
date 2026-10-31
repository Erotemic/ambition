"""The tile language that the Mary-O level scripts are written in.

A level script makes a `Level`, calls its methods to put rock, blocks, coins,
enemies, pipes and platforms into it, and hands it to `cli`. The `Level` is
written through `ambition_ldtk_tools`: `area create` for the room, `entity add`
for the named blocks, `level set-field` for the level fields that an area spec
cannot state.

All positions are in tiles of 32 px. Row 0 is the top. The floor row of a level
is the row of the top of its floor: a body that stands on the floor has its
feet at the top of that row.

The numbers a level is sized with (the `classic` tuning,
`assets/data/character_catalog.ron`): a full jump from a stand rises 5.4 tiles,
a full jump at a run rises 6.4 tiles and goes 11 tiles to the same height. A
gap that only a platform crosses is wider than 11 tiles.
"""

from __future__ import annotations

import argparse
import json
import math
import subprocess
import sys
import tempfile
from pathlib import Path
from typing import Callable

REPO = Path(__file__).resolve().parents[3]
TOOLS = REPO / "tools" / "ambition_ldtk_tools"
TARGET = REPO / "game" / "ambition_map_assets" / "ambition_demo_mary_o" / "worlds" / "mary_o.ldtk"
# The same file by the path that has Mary-O's editor-art overlay beside it
# (`mary_o.editor_art.json`). `asset editor-art` reads the overlay that is
# beside the path it is given.
EDITOR_ART_VIEW = REPO / "game" / "ambition_demo_mary_o" / "authoring" / "worlds" / "mary_o.ldtk"

T = 32

# The clockwork lift (`scenery.rs` registers its look).
PLATFORM_VISUAL = "super_mary_o_lift"

# Fields of the engine's `MovingPlatform` that a level here uses and an older
# copy of the entity definition does not have.
PLATFORM_FIELDS = ("visual:String:", "one_way:Bool:false")

ENEMIES = {
    "slop": ("AI Slop", "mary_o_ai_slop", "ai_slop", (28, 28), "s"),
    "snake": ("Solid Snake", "mary_o_snake", "solid_snake", (28, 32), "k"),
    "paper": ("Snakes on a Paper Plane", "mary_o_snakes_on_a_paper_plane", "npc_snakes_on_a_paper_plane", (43, 21), "p"),
    "cartesian": ("Snakes on a Cartesian Plane", "mary_o_snakes_on_a_cartesian_plane", "npc_snakes_on_a_cartesian_plane", (43, 18), "c"),
    # It stays over the player and lets coconuts go (`swallow.rs`).
    "swallow": ("Laden Swallow", "mary_o_laden_swallow", "laden_swallow", (40, 28), "w"),
    # No stomp hurts it: a touch from any side is a hit (`void_star.rs`).
    "star": ("Void Star", "mary_o_void_star", "void_star", (28, 28), "x"),
    # The boss of the castle (`void_lizard.rs`).
    "void_lizard": ("Void Lizard", "mary_o_void_lizard", "void_lizard", (60, 60), "W"),
    # The plant in a pipe (`plant.rs`). Use `Level.plant`, which gives it its pipe.
    "plant": ("Snake Plant", "mary_o_snake_plant", "snake_plant", (28, 46), "f"),
}  # fmt: skip


class Level:
    """A grid of tiles for the static collision, and lists of the entities.

    `world` is where the level is in the world of the LDtk file. The layout is
    free: no level can overlap another. `fields` are level fields of an area
    spec (`biome`, `music_track`); `late_fields` are those that only
    `level set-field` can state (`next_room`, `title`). A coin is named
    `<coin_stem>_<n>` unless its call gives a stem."""

    def __init__(
        self,
        area: str,
        *,
        width: int,
        height: int,
        floor: int,
        world: tuple[int, int],
        bg_color: str,
        fields: dict[str, str],
        late_fields: dict[str, str],
        coin_stem: str = "coin",
    ) -> None:
        self.area = area
        self.width, self.height, self.floor = width, height, floor
        self.world = world
        self.bg_color = bg_color
        self.fields = fields
        self.late_fields = late_fields
        self.coin_stem = coin_stem
        self.grid = [["."] * width for _ in range(height)]
        self.entities: list[dict] = []
        self.named: list[dict] = []
        self.marks: list[tuple[float, float, str]] = []
        self.counts: dict[str, int] = {}

    def up(self, tiles: int) -> int:
        """The row whose top is `tiles` above the top of the floor."""
        return self.floor - tiles

    # ── static collision ────────────────────────────────────────────────
    def fill(self, col: int, row: int, wide: int, tall: int, tile: str) -> None:
        for r in range(row, row + tall):
            for c in range(col, col + wide):
                if 0 <= r < self.height and 0 <= c < self.width:
                    self.grid[r][c] = tile

    def rock(self, col: int, row: int, wide: int, tall: int) -> None:
        self.fill(col, row, wide, tall, "#")

    def carve(self, col: int, row: int, wide: int, tall: int) -> None:
        self.fill(col, row, wide, tall, ".")

    def one_way(self, col: int, row: int, wide: int) -> None:
        self.fill(col, row, wide, 1, "=")

    def column(self, col: int, wide: int, tall: int) -> None:
        """Rock that stands `tall` tiles on the floor."""
        self.rock(col, self.floor - tall, wide, tall)

    def pit(self, col: int, wide: int) -> None:
        """A hole in the floor that is open to the bottom of the level."""
        self.carve(col, self.floor, wide, self.height - self.floor)

    def lava(self, col: int, wide: int, *, under: float = 1.25) -> None:
        """Lava in a pit: its top is `under` tiles below the top of the floor, and it goes to the bottom of
        the level. It is a damage volume the game draws as lava (its name starts with `lava`)."""
        top = int(round((self.floor + under) * T))
        self._entity("DamageVolume", (col * T, top), (wide * T, self.height * T - top), name=self._number("lava"))
        for index in range(wide):
            self.marks.append((col + index, self.floor + 1, "="))

    def pillar(self, col: int, wide: int, tall: int) -> None:
        """Rock that comes up out of a pit to `tall` tiles above the floor."""
        self.rock(col, self.floor - tall, wide, self.height - (self.floor - tall))

    # ── entities ────────────────────────────────────────────────────────
    def _entity(self, etype: str, px: tuple[float, float], size: tuple[int, int], /, **fields) -> None:
        entry: dict = {"type": etype, "px": [int(round(px[0])), int(round(px[1]))], "size": list(size)}
        if fields:
            entry["fields"] = fields
        self.entities.append(entry)

    def _number(self, stem: str) -> str:
        index = self.counts.get(stem, 0)
        self.counts[stem] = index + 1
        return f"{stem}_{index}"

    def start(self, col: float, name: str, *, on: int | None = None) -> None:
        """Where the player starts: over the floor, or over the surface whose top is the top of row `on`."""
        surface = self.floor if on is None else on
        self._entity("PlayerStart", (col * T, (surface - 2) * T), (28, 46), name=name)
        self.marks.append((col, surface - 1, "S"))

    def block(self, col: int, row: int, kind: str, contents: str | None = None) -> None:
        """A block that reacts to a head: `kind` is Question, Brick, Hidden or Plank.

        A block that holds an item needs open air over it: the item comes out
        of its top."""
        fields = {"kind": kind}
        if contents:
            fields["contents"] = contents
        self._entity("MaryOBlock", (col * T, row * T), (T, T), **fields)
        letter = {"Question": "?", "Brick": "B", "Hidden": "h", "Plank": "_"}[kind]
        if contents and kind == "Brick":
            letter = "b"
        assert self.grid[row][col] == ".", f"a block at {col},{row} is in other collision"
        self.grid[row][col] = letter

    def bricks(self, col: int, row: int, wide: int, tall: int = 1) -> None:
        for r in range(row, row + tall):
            for c in range(col, col + wide):
                if self.grid[r][c] == ".":
                    self.block(c, r, "Brick")

    def planks(self, col: int, row: int, wide: int) -> None:
        """A bridge: all the planks of a level fall when its goal is taken."""
        for c in range(col, col + wide):
            self.block(c, row, "Plank")

    def coin(self, col: float, row: float, stem: str | None = None) -> None:
        """A coin in the tile at `col`, `row`, low in the tile as the first level has it."""
        self._entity("PickupSpawn", (col * T + 4, row * T + 8), (24, 24), name=self._number(stem or self.coin_stem), kind="currency:1")
        self.marks.append((col, row, "o"))

    def coins(self, col: float, row: float, count: int, step: float = 1.0, stem: str | None = None) -> None:
        for index in range(count):
            self.coin(col + index * step, row, stem)

    def coin_arc(self, col: float, row: float, rise: tuple[float, ...], stem: str | None = None) -> None:
        """Coins in the line of a jump: one per column, each `rise` tiles above `row`."""
        for index, lift in enumerate(rise):
            self.coin(col + index, row - lift, stem)

    def enemy(self, kind: str, col: float, row: float | None = None) -> None:
        """An enemy of `ENEMIES`. `row` is the row of the top of what its feet are on; the floor if not given."""
        name, brain, character, size, letter = ENEMIES[kind]
        feet = self.floor if row is None else row
        self._entity(
            "EnemySpawn",
            (col * T + 2, feet * T - size[1]),
            size,
            name=name,
            brain=brain,
            character_id=character,
            respawn="OnRoomReenter",
            facing="Left",
        )
        self.marks.append((col, feet - 1, letter))

    def pipe(self, col: int, link: str, role: str, *, on: int | None = None, under: int | None = None) -> None:
        """A pipe, 3 tiles wide and 2 tall. `role` is Entrance or Exit, and two pipes of one `link` are one
        tube; or it is Sealed, and the pipe goes nowhere; or it is Warp, and `link` is the level it goes to.

        `on` is the row of the top of the surface it stands on: its mouth is up.
        `under` is the first row below the ceiling it hangs from: its mouth is down."""
        if under is not None:
            px, mouth = (col * T, under * T), "Down"
        else:
            px, mouth = (col * T, ((self.floor if on is None else on) - 2) * T), "Up"
        self._entity("MaryOPipe", px, (3 * T, 2 * T), link=link, mouth=mouth, role=role)
        for index in range(3):
            for row in range(2):
                self.marks.append((col + index, px[1] / T + row, "P"))

    def plant(self, col: int, *, on: int | None = None) -> None:
        """A Snake Plant in a pipe of its own, which goes nowhere. The pipe stands on the surface at `on`."""
        surface = self.floor if on is None else on
        # A link of its own: the tiles of a pipe are named for its link.
        self.pipe(col, f"pot_{col}", "Sealed", on=surface)
        name, brain, character, size, letter = ENEMIES["plant"]
        top = (surface - 2) * T
        self._entity(
            "EnemySpawn",
            (col * T + 1.5 * T - size[0] / 2, top - size[1]),
            size,
            name=name,
            brain=brain,
            character_id=character,
            respawn="OnRoomReenter",
            facing="Left",
        )
        self.marks.append((col + 1, surface - 3, letter))

    def fire_bar(
        self, col: int, row: int, *, orbs: int = 5, period: float = 4.0, clockwise: bool = True, turn: float = 0.0
    ) -> None:
        """A bar of fire that turns about the tile at `col`, `row`, which becomes rock.

        Each orb is a damage volume on a path of its own about the pivot: a
        ring of straight parts, which is the motion a path has. Each ring takes
        `period` seconds, so the orbs stay in a line. `turn` is where the bar
        starts, as a part of a full turn (0 is to the right).

        The game draws a damage volume of a Mary-O room as a ball of fire
        (`scenery::dress_fire`)."""
        self.rock(col, row, 1, 1)
        cx, cy = (col + 0.5) * T, (row + 0.5) * T
        sides, size = 24, 18
        for orb in range(1, orbs + 1):
            radius = orb * 16.0
            ring = []
            for side in range(sides):
                angle = math.tau * (turn + (side / sides if clockwise else -side / sides))
                ring.append((cx + radius * math.cos(angle), cy + radius * math.sin(angle)))
            edge = 2.0 * radius * math.sin(math.pi / sides)
            self._entity(
                "DamageVolume",
                (ring[0][0] - size / 2, ring[0][1] - size / 2),
                (size, size),
                name=self._number("fire_bar_orb"),
                path_points=";".join(f"{x:.2f},{y:.2f}" for x, y in ring),
                path_speed=round(edge * sides / period, 3),
                path_mode="Loop",
            )
        reach = math.ceil(orbs / 2)
        for step in range(1, reach + 1):
            self.marks.append((col + step * (1 if math.cos(math.tau * turn) >= 0 else -1), row, "*"))

    def platform(self, ident: str, name: str, col: float, row: float, *, wide: int = 3, **motion: float) -> None:
        """A moving platform whose top is at the top of `row`. `motion` is its fields (`sweep_dx`, `speed`)."""
        self._entity(
            "MovingPlatform",
            (col * T, row * T),
            (wide * T, T // 2),
            id=ident,
            name=name,
            visual=PLATFORM_VISUAL,
            **motion,
        )
        for index in range(wide):
            self.marks.append((col + index, row, "~"))

    def lift_shaft(
        self, stem: str, name: str, col: int, *, down: bool, speed: float, cars: int, chimney: int = 0
    ) -> None:
        """Platforms that go one way in a line and come back in at its other end.

        The line is from over the level to under it, so a car comes in and goes
        out where no view can show it (`every_lift_wraps_outside_the_room`).
        `chimney` rows of rock under the top row are opened to let the cars
        through; the top row of the level stays closed. A car is one-way: the
        rock that stops a rider does not crush it."""
        self.carve(col, 1, 3, chimney)
        top, bottom = -T, (self.height + 1) * T
        span = bottom - top
        for index in range(cars):
            self._entity(
                "MovingPlatform",
                # Each car starts in the level: a placement outside it is refused.
                (col * T, top + span * (index + 0.5) / cars),
                (3 * T, T // 2),
                id=f"{stem}_{'abcd'[index]}",
                name=f"{name} {'ABCD'[index]}",
                visual=PLATFORM_VISUAL,
                one_way=True,
                speed=speed,
                loop_dy=float(span if down else -span),
                loop_min_y=float(top),
            )
        for row in range(1, self.height):
            self.marks.append((col + 1, row, "v" if down else "^"))

    def camera_zone(self, ident: str, name: str, row: int, tall: int) -> None:
        """The camera stays in these rows while the player is in them.

        A body that stands on the top line of the zone is not in it (measured
        on the roof of 1-2: her feet are on the line at each tick, and a box
        that touches a zone is not in it)."""
        self._entity(
            "CameraZone",
            (0, row * T),
            (self.width * T, tall * T),
            id=ident,
            name=name,
            clamp_mode="zone_bounds",
            zoom=1.0,
        )

    def arena(self, ident: str, name: str, col: int, wide: int, *, zoom: float = 1.0, down: float = 0.0) -> None:
        """A view that does not move: while the player is in these columns the
        camera looks at their middle, `down` tiles under the middle of the
        height of the level. `zoom` is how much MORE of the level the view
        shows: 1.2 is a view a fifth wider (the engine does not go under 1)."""
        self._entity(
            "CameraZone",
            (col * T, 0),
            (wide * T, self.height * T),
            id=ident,
            name=name,
            priority=10,
            zoom=zoom,
            target_offset_y=down * T,
            cinematic_lock=True,
            easing_hz=2.5,
        )

    def pole(self, col: int, tall: int) -> None:
        """The goal: a shaft, its knob and its banner, as the first level has them.

        They are named blocks. `area create` drops the name of a static block,
        so `apply` adds them after it."""
        x, top = col * T, (self.floor - tall) * T
        for name, y, high in (("goal_pole", top, tall * T), ("goal_pole_knob", top - 16, 16), ("goal_pole_banner", top + 16, 32)):
            self.named.append({"type": "OneWayPlatform", "px": [x, y], "size": [16, high], "fields": {"name": name}})
        for row in range(self.floor - tall, self.floor):
            self.marks.append((col, row, "|"))

    def switch(self, col: int) -> None:
        """The goal of a castle: a button on the floor (`castle_end.rs`). A body presses it when it lands on it.

        It is a named block, as the parts of a pole are. It is solid, and it
        has the size of its art: 36 px wide, and 44 px to the top of the button."""
        wide, tall = 36, 44
        x = col * T + (T - wide) // 2
        self.named.append({"type": "Solid", "px": [x, self.floor * T - tall], "size": [wide, tall], "fields": {"name": "goal_switch"}})
        self.marks.append((col, self.floor - 1, "!"))

    def gate(self, col: int, tall: int = 3) -> None:
        """The gate of a castle: a wall of one tile in a doorway `tall` tiles high. It opens when the boss
        of the castle is dead (`castle_end.rs`). It is a named block, as the switch is."""
        self.named.append({"type": "Solid", "px": [col * T, (self.floor - tall) * T], "size": [T, tall * T], "fields": {"name": "goal_gate"}})
        for row in range(self.floor - tall, self.floor):
            self.marks.append((col, row, "H"))

    def loading_zone(
        self, ident: str, name: str, col: float, row: float, wide: float, tall: float, *, to: tuple[str, str] | None = None
    ) -> None:
        """A zone of the engine that takes a body to another level when the body is in it.

        `to` is the level and the zone of that level where the body arrives.
        With no `to` the zone is where a trip ends, and it takes a body
        nowhere: the body arrives in the middle of it, 26 px over its bottom."""
        fields = {"id": ident, "name": name, "activation": "Walk", "bidirectional": False}
        if to is not None:
            fields |= {"target_room": to[0], "target_zone": to[1]}
        self._entity("LoadingZone", (col * T, row * T), (int(wide * T), int(tall * T)), **fields)

    def npc(self, character: str, col: float, size: tuple[int, int] = (28, 44)) -> None:
        """A character of the catalog that stands on the floor and is not an enemy."""
        self._entity("NpcSpawn", (col * T, self.floor * T - size[1]), size, character_id=character)
        self.marks.append((int(col), self.floor - 1, "R"))

    # ── output ──────────────────────────────────────────────────────────
    def picture(self) -> list[str]:
        rows = [row[:] for row in self.grid]
        for col, row, letter in self.marks:
            r, c = int(row), int(col)
            if 0 <= r < self.height and 0 <= c < self.width and rows[r][c] == ".":
                rows[r][c] = letter
        return ["".join(row) for row in rows]

    def collision_rects(self) -> list[dict]:
        """The rock and the one-way tiles as rectangles, one per run in a row."""
        rects = []
        for kind, tile in (("Solid", "#"), ("OneWayPlatform", "=")):
            for r, row in enumerate(self.grid):
                c = 0
                while c < self.width:
                    if row[c] != tile:
                        c += 1
                        continue
                    start = c
                    while c < self.width and row[c] == tile:
                        c += 1
                    high = T if kind == "Solid" else T // 2
                    rects.append({"type": kind, "px": [start * T, r * T], "size": [(c - start) * T, high]})
        return rects

    def area_spec(self) -> dict:
        return {
            "id": self.area,
            "level_id": self.area,
            "world_x": self.world[0],
            "world_y": self.world[1],
            "px_wid": self.width * T,
            "px_hei": self.height * T,
            "grid_size": 16,
            "fill_collision": "empty",
            "bg_color": self.bg_color,
            **self.fields,
            "entities": self.collision_rects() + self.entities,
        }


def run_tool(*args: str) -> None:
    print("::", " ".join(str(a) for a in args))
    result = subprocess.run(
        [sys.executable, "-m", "ambition_ldtk_tools", *args],
        cwd=REPO,
        env={"PYTHONPATH": str(TOOLS), "PATH": "/usr/bin:/bin"},
    )
    if result.returncode != 0:
        sys.exit(f"tool step failed: {' '.join(str(a) for a in args)}")


def ensure_platform_fields() -> None:
    project = json.loads(TARGET.read_text())
    definition = next(d for d in project["defs"]["entities"] if d["identifier"] == "MovingPlatform")
    have = {field["identifier"] for field in definition["fieldDefs"]}
    missing = [spec for spec in PLATFORM_FIELDS if spec.split(":")[0] not in have]
    if missing:
        args = [arg for spec in missing for arg in ("--add-field", spec)]
        run_tool("def", "update-entity", "MovingPlatform", str(TARGET), *args, "--in-place", "--no-repair")


def apply(lv: Level) -> None:
    """Replace the level in the LDtk file. The file is checked only at the end: it is whole only then."""
    ensure_platform_fields()
    with tempfile.TemporaryDirectory() as tmp:
        area = Path(tmp) / f"{lv.area}_area.json"
        area.write_text(json.dumps(lv.area_spec(), indent=2))
        run_tool("area", "create", str(area), "--ldtk", str(TARGET), "--replace-existing", "--no-repair")
        named = Path(tmp) / f"{lv.area}_named.json"
        named.write_text(json.dumps({"level_id": lv.area, "entities": lv.named}, indent=2))
        run_tool("entity", "add", str(named), "--ldtk", str(TARGET), "--in-place", "--no-repair")
    sets = [arg for key, value in lv.late_fields.items() for arg in ("--set", f"{key}={value}")]
    run_tool("level", "set-field", "--ldtk", str(TARGET), "--in-place", "--level", lv.area, *sets)
    run_tool("repair", str(TARGET), "--in-place")
    run_tool("validate", str(TARGET))
    run_tool("vocabulary", "check", "--ldtk", str(TARGET), "--level", lv.area)
    # A level that `area create` made has no editor-art layers: in the LDtk
    # editor it is bare collision until this runs.
    run_tool("asset", "editor-art", str(EDITOR_ART_VIEW), "--in-place")


# What a body stands on, in the letters of `Level.picture`.
STANDS_ON = set("#=Bb?_PH!")
# A jump that each level can ask for, in tiles. They are under what the
# `classic` tuning gives (see the top of this file), so a level that passes
# has room for a player who is not exact.
JUMP_UP, JUMP_OVER, JUMP_OVER_HIGH, FALL_OVER = 4, 6, 4, 7


def way_through(lv: Level) -> tuple[bool, str]:
    """Is there a way from the start to the goal by jumps of a safe size?

    The places a body stands are the tiles over a thing of `STANDS_ON`, the
    line a ferry goes along, and each height of a line of lifts. A jump goes
    up `JUMP_UP` tiles at most and over `JUMP_OVER` (`JUMP_OVER_HIGH` when it
    goes up more than two); a fall goes over `FALL_OVER`. A column between
    the two places that is solid for the full height of a jump stops it. An
    enemy is not seen, and a roof is not seen: this finds a pit that is too
    wide and a wall or a ledge that is too high, and no more than that."""
    rows = lv.picture()
    spots: set[tuple[int, int]] = set()
    for r in range(1, lv.height):
        for c in range(lv.width):
            if rows[r][c] in STANDS_ON and rows[r - 1][c] not in STANDS_ON:
                spots.add((c, r - 1))
    start = goal = None
    for entity in lv.entities + lv.named:
        col, row = entity["px"][0] / T, entity["px"][1] / T
        fields = entity.get("fields", {})
        if entity["type"] == "PlayerStart":
            start = int(col)
        elif fields.get("name") in ("goal_pole", "goal_switch"):
            goal = int(col)
        elif entity["type"] == "MovingPlatform":
            wide = entity["size"][0] // T
            if "loop_dy" in fields:
                spots.update((int(col) + i, r) for i in range(wide) for r in range(1, lv.height - 1))
            else:
                reach = int(abs(fields.get("sweep_dx", 0.0)) // T)
                spots.update((int(col) + i, int(row) - 1) for i in range(wide + reach))
    if start is None:
        return False, "the level has no start"
    if goal is None:
        # A room of pipes: each way out is a pipe.
        return True, "the level has no pole and no switch: there is no goal to get to"
    def walled(c1: int, r1: int, c2: int) -> bool:
        low = max(0, r1 - JUMP_UP)
        return any(all(rows[r][c] in STANDS_ON for r in range(low, r1 + 1)) for c in range(min(c1, c2) + 1, max(c1, c2)))

    seen = {spot for spot in spots if spot[0] == start}
    edge = list(seen)
    while edge:
        c1, r1 = edge.pop()
        for spot in spots:
            if spot in seen:
                continue
            over, up = abs(spot[0] - c1), r1 - spot[1]
            if up > JUMP_UP or over > (FALL_OVER if up < 0 else JUMP_OVER if up <= 2 else JUMP_OVER_HIGH):
                continue
            if walled(c1, r1, spot[0]):
                continue
            seen.add(spot)
            edge.append(spot)
    if any(abs(c - goal) <= 2 for c, _ in seen):
        return True, f"a way of safe jumps goes from column {start} to the goal at column {goal}"
    return False, f"no way of safe jumps goes past column {max(c for c, _ in seen)}: the goal is at column {goal}"


def draw(lv: Level, path: Path, scale: int = 8) -> None:
    from PIL import Image, ImageDraw

    colors = {
        "#": (86, 70, 150), "=": (220, 120, 200), "B": (170, 90, 60), "b": (210, 130, 60),
        "?": (250, 200, 60), "h": (120, 120, 120), "o": (255, 235, 120), "s": (230, 80, 80),
        "k": (80, 200, 110), "p": (240, 240, 240), "c": (120, 220, 240), "P": (200, 120, 60),
        "~": (90, 190, 255), "v": (40, 70, 110), "^": (40, 110, 70), "|": (240, 240, 220),
        "S": (255, 120, 220), "w": (60, 90, 200), "f": (40, 160, 70), "*": (255, 150, 40), "_": (150, 110, 80), "W": (235, 235, 245), "x": (200, 200, 215),
    }  # fmt: skip
    image = Image.new("RGB", (lv.width * scale, lv.height * scale), (20, 16, 28))
    pen = ImageDraw.Draw(image)
    for r, row in enumerate(lv.picture()):
        for c, tile in enumerate(row):
            if tile in colors:
                pen.rectangle([c * scale, r * scale, (c + 1) * scale - 1, (r + 1) * scale - 1], fill=colors[tile])
    for c in range(0, lv.width, 10):
        pen.line([c * scale, 0, c * scale, 3], fill=(255, 255, 255))
    image.save(path)


def cli(build: Callable[[], Level], doc: str, *, before_apply: Callable[[], None] | None = None) -> None:
    """The command line of a level script.

    `--preview` prints the level as text and changes nothing. `--png FILE`
    draws the same picture. `--check` looks for a way from the start to the
    goal (`way_through`); `--apply` does it first and stops if there is none. `--apply` REPLACES the level in the LDtk file: an
    edit made in the LDtk editor after the last `--apply` is lost. Make such an
    edit in the script also, or stop using the script for the level.

    `before_apply` runs before the level goes into the file: a script uses it
    for an entity that it puts in a level of another script. `apply` checks
    the file at its end, so the two levels are checked together."""
    parser = argparse.ArgumentParser(description=doc, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--preview", action="store_true", help="print the level as text")
    parser.add_argument("--png", type=Path, help="draw the level to this file")
    parser.add_argument("--check", action="store_true", help="look for a way from the start to the goal by safe jumps")
    parser.add_argument("--apply", action="store_true", help="replace the level in the LDtk file")
    args = parser.parse_args()
    lv = build()
    if args.check or args.apply:
        ok, words = way_through(lv)
        print(("OK: " if ok else "error: ") + words)
        if not ok:
            sys.exit(1)
    if args.preview or not (args.png or args.apply or args.check):
        ruler = "".join(str((c // 10) % 10) if c % 10 == 0 else " " for c in range(lv.width))
        print("   " + ruler)
        for r, row in enumerate(lv.picture()):
            print(f"{r:2d} {row}")
    if args.png:
        draw(lv, args.png)
    if args.apply:
        if before_apply is not None:
            before_apply()
        apply(lv)
