#!/usr/bin/env python3
"""Check that the Noodly Temple can be finished, and that nothing in it
traps the player.

    python specs/noodly_temple/check_temple.py

Run it from `tools/ambition_ldtk_tools`, after `make_specs.py --apply`.

It reads the rooms from the map and walks them as a model of the game:

- A body is 32 by 48 and it flies: it goes to each place that it fits in.
  Solid cells stop it, and so does a `LockWall` that stands. A wall that
  breaks does not stop it.
- A `LockWall` stands until its `gated_by` is true. The model reads that
  line as the game does: a first word `not` turns the gate round, a first
  token with a dot is a condition (`world.switch_on`, `encounter.cleared`),
  and a bare value is a flag.
- A body that touches a key has its flag. A body that touches a lever has
  used it (`switch_<id>_used`) and has turned it (`world.switch_on`). A body
  that enters the strip of a fight is taken to have won it.
- A body that touches a door of a room goes to the door it names.

It fails when the monster can not be reached from the door of the central
hub, when a key or a room can not be reached, and when the player can be in
a place from which the door of the central hub can not be reached again (a
trap), with each set of keys he can have there.

It prints the order the keys are found in, and the least number of turns of
a wheel before the key of the cistern.

This is a model. `game/ambition_app/tests/the_noodly_temple.rs` asks the game
itself about a key, a door, a wheel and a fight.
"""
from __future__ import annotations

import json
import sys
from collections import deque
from pathlib import Path

HERE = Path(__file__).resolve().parent
TOOLS = HERE.parents[1]
if str(TOOLS) not in sys.path:
    sys.path.insert(0, str(TOOLS))

from ambition_ldtk_tools.ldtk.paths import default_worlds_dir  # noqa: E402

CELL = 16
#: The body, in cells: 32 by 48.
BODY_W, BODY_H = 2, 3
ROOMS = ("noodly_narthex", "noodly_nave", "noodly_scullery", "noodly_cistern", "noodly_belfry", "noodly_reliquary",
         "noodly_sanctum")
HUB = "central_hub_complex"
START = ("noodly_narthex", "noodly_narthex_to_hub")


class Room:
    def __init__(self, level):
        self.id = level["identifier"]
        self.w = level["pxWid"] // CELL
        self.h = level["pxHei"] // CELL
        self.solid = set()
        self.things = []
        for layer in level["layerInstances"]:
            if layer["__type"] == "IntGrid" and layer["__identifier"] == "Collision":
                for i, value in enumerate(layer["intGridCsv"]):
                    if value == 1:
                        self.solid.add((i % self.w, i // self.w))
            for entity in layer.get("entityInstances", []):
                fields = {f["__identifier"]: f["__value"] for f in entity["fieldInstances"]}
                x, y = entity["px"]
                self.things.append((entity["__identifier"], (x, y, x + entity["width"], y + entity["height"]), fields))

    def of(self, kind):
        return [(box, fields) for k, box, fields in self.things if k == kind]


def cells_of(box):
    x0, y0, x1, y1 = box
    return {(cx, cy) for cx in range(x0 // CELL, (x1 + CELL - 1) // CELL) for cy in range(y0 // CELL, (y1 + CELL - 1) // CELL)}


def gate_open(gated_by, facts, wheels_on):
    """Whether a wall that authors `gated_by` is open (the rule of
    `gated_lock_walls.rs`)."""
    words = gated_by.split()
    turned = words[0] == "not"
    if turned:
        words = words[1:]
    if "." in words[0]:
        if words[0] == "world.switch_on":
            answer = words[1] in wheels_on
        elif words[0] == "encounter.cleared":
            answer = f"cleared:{words[1]}" in facts
        else:
            raise SystemExit(f"the model does not know the condition `{gated_by}`")
    else:
        answer = words[0] in facts
    return answer != turned


def main():
    # The noodle dimension's world file.
    world = json.loads(Path(default_worlds_dir() / "noodle.ldtk").read_text())
    rooms = {level["identifier"]: Room(level) for level in world["levels"] if level["identifier"] in ROOMS}
    missing = [r for r in ROOMS if r not in rooms]
    if missing:
        raise SystemExit(f"the map has no room {missing}: run make_specs.py --apply")

    # The wheels: the levers that a wall asks with `world.switch_on`.
    asked = set()
    for room in rooms.values():
        for _, fields in room.of("LockWall"):
            words = (fields.get("gated_by") or "").split()
            if "world.switch_on" in words:
                asked.add(words[-1])
    wheels = sorted(asked)

    def blocked(room, facts, wheels_on):
        out = set(room.solid)
        for box, fields in room.of("LockWall"):
            line = fields.get("gated_by")
            if line and not gate_open(line, facts, wheels_on):
                out |= cells_of(box)
        return out

    def fits(cell, shut, room):
        cx, cy = cell
        if cx < 0 or cy < 0 or cx + BODY_W > room.w or cy + BODY_H > room.h:
            return False
        return all((cx + dx, cy + dy) not in shut for dx in range(BODY_W) for dy in range(BODY_H))

    def touches(cell, box):
        cx, cy = cell
        return cx * CELL < box[2] and (cx + BODY_W) * CELL > box[0] and cy * CELL < box[3] and (cy + BODY_H) * CELL > box[1]

    def arrival(room, zone_id, shut):
        """The cell a body is in when it comes in by the door `zone_id`."""
        for box, fields in room.of("LoadingZone"):
            if fields.get("id") == zone_id:
                cx = (box[0] + box[2]) // 2 // CELL - 1
                for cy in range(box[3] // CELL - BODY_H, box[1] // CELL - 1, -1):
                    if fits((cx, cy), shut, room):
                        return (cx, cy)
        raise SystemExit(f"`{room.id}` has no door `{zone_id}` that a body fits in")

    def walk(facts):
        """Each state (room, cell, wheels on) that a body with `facts` can be
        in, the edges between them, the new facts it finds, and the things
        it touches."""
        shut_of = {}

        def shut(room_id, on):
            key = (room_id, on)
            if key not in shut_of:
                shut_of[key] = blocked(rooms[room_id], facts, on)
            return shut_of[key]

        first = (START[0], arrival(rooms[START[0]], START[1], shut(START[0], frozenset())), frozenset())
        seen = {first: 0}
        # The state each state was last reached from, and the wheel turned on
        # that edge (`None` for a step): the turns of the cheapest road.
        parent = {first: None}
        edges = {}
        found = set()
        touched = set()
        exits = set()
        queue = deque([first])
        while queue:
            state = queue.popleft()
            room_id, cell, on = state
            room = rooms[room_id]
            nexts = []
            turns = seen[state]
            for dx, dy in ((1, 0), (-1, 0), (0, 1), (0, -1)):
                to = (cell[0] + dx, cell[1] + dy)
                if fits(to, shut(room_id, on), room):
                    nexts.append(((room_id, to, on), 0, None))
            for kind, box, fields in room.things:
                if not touches(cell, box):
                    continue
                if kind == "PickupSpawn" and str(fields.get("kind", "")).startswith("flag:"):
                    found.add(fields["kind"][5:])
                    touched.add(("key", fields["kind"][5:]))
                elif kind == "EncounterTrigger":
                    found.add(f"cleared:{fields['id']}")
                elif kind == "BossSpawn":
                    touched.add(("boss", room_id))
                elif kind == "Switch":
                    found.add(f"switch_{fields['id']}_used")
                    if fields["id"] in wheels:
                        flipped = on ^ frozenset([fields["id"]])
                        # A wheel that is turned while the body is in the way
                        # of a gate that shuts: the game pushes the body
                        # out. The model does not turn the wheel there.
                        if fits(cell, shut(room_id, flipped), room):
                            nexts.append(((room_id, cell, flipped), 1, fields["id"]))
                elif kind == "LoadingZone":
                    target = fields.get("target_room")
                    if target == HUB:
                        exits.add(state)
                    elif target in rooms:
                        there = arrival(rooms[target], fields["target_zone"], shut(target, on))
                        nexts.append(((target, there, on), 0, None))
                        touched.add(("room", target))
            edges[state] = [n for n, _, _ in nexts]
            # The least turns of a wheel to each state: a step is free and a
            # turn costs one, so a state found again by fewer turns is walked
            # again.
            for to, cost, wheel in nexts:
                if to not in seen or turns + cost < seen[to]:
                    seen[to] = turns + cost
                    parent[to] = (state, wheel)
                    (queue.append if cost else queue.appendleft)(to)
        return seen, parent, edges, found - facts, touched, exits

    problems = []
    facts = frozenset()
    order = []
    touched_all = set()
    turns_to_key = None
    wheel_road = []
    while True:
        seen, parent, edges, new, touched, exits = walk(facts)
        touched_all |= touched
        # A trap: a state from which no door of the hub can be reached.
        back = {}
        for state, tos in edges.items():
            for to in tos:
                back.setdefault(to, []).append(state)
        can_leave = set(exits)
        queue = deque(exits)
        while queue:
            for state in back.get(queue.popleft(), ()):
                if state not in can_leave:
                    can_leave.add(state)
                    queue.append(state)
        traps = [s for s in seen if s not in can_leave]
        if traps:
            room_id, cell, on = traps[0]
            problems.append(f"a trap with {sorted(facts)}: `{room_id}` at px {cell[0] * CELL},{cell[1] * CELL} with wheels on "
                            f"{sorted(on)} ({len(traps)} such states)")
        if not new:
            break
        keys = sorted(f for f in new if f.startswith("noodly_key_"))
        if keys:
            order.append(keys)
        if "noodly_key_pesto" in new:
            # The least turns of a wheel to a state that touches the key.
            for (room_id, cell, on), turns in seen.items():
                if room_id == "noodly_cistern":
                    for box, fields in rooms[room_id].of("PickupSpawn"):
                        if fields.get("kind") == "flag:noodly_key_pesto" and touches(cell, box):
                            if turns_to_key is None or turns < turns_to_key:
                                turns_to_key = turns
                                road, at = [], (room_id, cell, on)
                                while parent[at] is not None:
                                    at, wheel = parent[at]
                                    if wheel is not None:
                                        road.append(wheel)
                                wheel_road = road[::-1]
        facts = facts | new

    for room_id in ROOMS[1:]:
        if ("room", room_id) not in touched_all:
            problems.append(f"`{room_id}` can not be reached")
    for key in ("marinara", "pesto", "parmesan", "colander"):
        if ("key", f"noodly_key_{key}") not in touched_all:
            problems.append(f"the key `{key}` can not be reached")
    if ("boss", "noodly_sanctum") not in touched_all:
        problems.append("the monster can not be reached")

    print("keys, in the order they are found:", " -> ".join("+".join(k[11:] for k in keys) for keys in order))
    print("least turns of a wheel before the key of the cistern:", turns_to_key, "(" + ", ".join(wheel_road) + ")")
    if problems:
        print("\n".join(f"PROBLEM: {p}" for p in problems))
        return 1
    print("the temple can be finished, and no place in it is a trap")
    return 0


if __name__ == "__main__":
    sys.exit(main())
