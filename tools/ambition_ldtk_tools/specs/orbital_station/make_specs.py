#!/usr/bin/env python3
"""Write the rooms of the orbital station of the Science dimension.

    python specs/orbital_station/make_specs.py          # writes the specs
    python specs/orbital_station/make_specs.py --apply  # and makes the rooms

Run it from `tools/ambition_ldtk_tools`.

The station is in the Science dimension's world file (`science.ldtk`), with
the gravity works where GNU-ton waits (`specs/gnu_ton_gravity/`): one world
file is one dimension (Jon, 2026-10-24). A long spoke joins the two on foot
and by tram, so they are two places of one geography; each has its own gate
(`GALACTIC` at the gate dock, `ASCENT` at the orchard).

The rooms, and the way each joins the next:

    gate dock --(stairs)-- habitation --- observation deck
        |                     |                  |
    (freight lane,       service deck --(lift)-- docking bay
     shut by customs)     |       |                  |
        +-----------------|-------|------ (freight lane) --+
                          |    (hatch, by pass)
                        spoke  damaged spur
                          |
                    the orchard (gravity works)

- `orbital_station_gate_dock`: the gate's arrival (`orbital_station_gate_arrival`)
  and a clear floor for the ring and its terminal (Agent GATE places them),
  the freight track from the ring, the customs and quarantine desk, the
  manifest board, and the quarantine pen. The pen's gate shuts the freight
  lane to the docking bay until customs clears consignment 0447.
- `orbital_station_docking_bay`: the hauler *Remainder* in its berth, a
  gantry up to its hatch, a shuttle on its cradle, freight on the floor. Up
  the gantry, the way to the observation deck; on the floor, the cargo lift
  to the service deck and the freight lane back to the gate dock.
- `orbital_station_observation_deck`: the viewpoint, open on the planet.
- `orbital_station_habitation`: bunks, the galley, the hydroponic racks.
- `orbital_station_service_deck`: the plant, the hatch of the damaged spur
  (opened by a pass customs gives), and the spoke to the gravity works.
- `orbital_station_damaged_spur`: the optional, dangerous branch. Broken
  deck over live cables, maintenance crawlers that do not stop, a breaker at
  the end and what the crew left there.
- `orbital_station_spoke`: the long walk to the gravity works, with a tram
  over the gap where the spoke is open to space, and the lower end of the
  works' lift (its gate is let go from inside).

Cycles: habitation, service deck, docking bay and observation deck are one
loop; the freight lane, once customs clears the crate, is a short way from
the gate dock to the bay. The lift from the GNU-ton approach to the spoke is
the trail's shortcut.

The arrival is the zone of the dock's ring (GALACTIC, a ring of the gate
network) and its terminal stands beside it.

The numbers (the robot, measured): a jump rises 83 px, a jump and a double
jump 139 px, a run is 270 px/s, the body is 30 x 48. Every step up is 64 px
or less.
"""
from __future__ import annotations

import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
TOOLS = HERE.parents[1]
if str(TOOLS) not in sys.path:
    sys.path.insert(0, str(TOOLS))

from ambition_ldtk_tools.ldtk.paths import default_worlds_dir  # noqa: E402

sys.path.insert(0, str(HERE.parent / "gate_network"))
import gates  # noqa: E402

WORLD_LDTK = default_worlds_dir() / "science.ldtk"

DOCK = "orbital_station_gate_dock"
BAY = "orbital_station_docking_bay"
VIEW = "orbital_station_observation_deck"
HAB = "orbital_station_habitation"
SERVICE = "orbital_station_service_deck"
SPUR = "orbital_station_damaged_spur"
SPOKE = "orbital_station_spoke"

#: The gate's arrival in the dock (the address catalog names it).
ARRIVAL = "orbital_station_gate_arrival"

#: The trail's rooms the station joins (`specs/gnu_ton_gravity/make_specs.py`).
ORCHARD, ORCHARD_SPOKE_DOOR = "gnu_ton_orchard", "orchard_spoke_door"
APPROACH, APPROACH_LIFT_DOOR = "gnu_ton_works_approach", "approach_lift_door"
#: The lift's lower end, in the spoke; the approach's lift door names it.
SPOKE_LIFT_DOOR = "spoke_lift_door"

#: The flags of the errand (`orbital.yarn`, `quests.ron`).
CLEARED = "orbital_consignment_cleared"
SPUR_PASS = "orbital_spur_pass"

DECK_THEME = "orbital"
OPEN_THEME = "orbital_truss"
MUSIC = "between_objectives"
SPUR_MUSIC = "vault_in_the_dark"
BG = "#0E1424"
WALL = 16

#: Each room: (world_x, world_y, width, height). The station is below the
#: gravity works in the world's layout.
ROOMS_AT = {
    DOCK: (0, 2400, 2240, 768),
    BAY: (2496, 2144, 2560, 1024),
    VIEW: (2496, 1280, 1600, 768),
    HAB: (0, 1408, 2400, 768),
    SERVICE: (-2496, 1280, 2240, 896),
    SPUR: (-5312, 1280, 2560, 896),
    SPOKE: (-3712, 192, 3200, 640),
}


# ---------------------------------------------------------------------------
# Entities
# ---------------------------------------------------------------------------


def _value(v) -> str:
    if isinstance(v, bool):
        return "true" if v else "false"
    if isinstance(v, float):
        return f"{v:.1f}" if v == int(v) else repr(v)
    if isinstance(v, int):
        return str(v)
    return f'"{v}"'


def entity(etype, x, y, w, h, **fields) -> str:
    field_text = ", ".join(f"{k}: {_value(v)}" for k, v in fields.items())
    return f'( type: "{etype}", px: [{x}, {y}], size: [{w}, {h}], fields: ( {field_text} ) ),'


def solid(name, x, y, w, h):
    return entity("Solid", x, y, w, h, name=name)


def ledge(name, x, y, w):
    return entity("OneWayPlatform", x, y, w, 16, name=name)


def hazard(name, x, y, w, h=16):
    return entity("HazardBlock", x, y, w, h, name=name)


def door(zone, name, x, floor, to_room, to_zone, bidirectional=True):
    return entity("LoadingZone", x, floor - 96, 48, 96, id=zone, name=name, activation="Door",
                  target_room=to_room, target_zone=to_zone, bidirectional=bidirectional)


def landing(zone, name, x, floor):
    """A door that only receives: an arrival with no way out of its own."""
    return entity("LoadingZone", x, floor - 96, 48, 96, id=zone, name=name, activation="Door")


def start(rid, x, floor):
    return entity("PlayerStart", x, floor - 46, 28, 46, name=f"{rid}_spawn")


def prop(name, kind, x, bottom, w, h):
    """A prop whose foot is at `bottom`."""
    return entity("Prop", x, bottom - h, w, h, name=name, kind=kind)


def resident(character, dialogue, x, floor, h=64, roam=True):
    fields = dict(character_id=character, dialogue_id=dialogue)
    if roam:
        fields["brain_override"] = "townsfolk_roam"
    return entity("NpcSpawn", x, floor - h, 32, h, **fields)


def lock(lid, name, x, y, w, h, gated_by, look=None):
    fields = dict(id=lid, name=name, gated_by=gated_by)
    if look:
        fields["look"] = look
    return entity("LockWall", x, y, w, h, **fields)


def switch(sid, x, floor, prompt):
    return entity("Switch", x, floor - 32, 24, 32, id=sid, name=sid.replace("_", " "), prompt=prompt,
                  target_encounter="", action="ToggleFlag")


def foe(rid, n, character, x, floor, w, h):
    return entity("EnemySpawn", x, floor - h, w, h, name=f"{rid}_{character}_{n}", brain="combatant",
                  character_id=character, respawn="OnRoomReenter")


def coin(name, x, y, amount=10):
    return entity("PickupSpawn", x, y, 24, 24, name=name, kind=f"currency:{amount}", respawn="Never")


def label(name, x, y, text, w=640):
    return entity("DebugLabel", x, y, w, 24, name=name, text=text, category="Custom")


def camera(rid, w, h):
    return entity("CameraZone", 0, 0, w, h, id=f"{rid}_camera", name=f"{rid}_camera", mode="Default")


def shell(w, h, floor, vault=48):
    """The floor, the vault and the two end walls of a deck."""
    return [
        solid("floor", 0, floor, w, h - floor),
        solid("vault", 0, 0, w, vault),
        solid("west_wall", 0, vault, WALL, floor - vault),
        solid("east_wall", w - WALL, vault, WALL, floor - vault),
    ]


def room(rid, entities, title, theme=DECK_THEME, music=MUSIC, title_y=64):
    world_x, world_y, w, h = ROOMS_AT[rid]
    body = "\n        ".join(entities + [camera(rid, w, h), label(f"{rid}_title", 64, title_y, title, min(900, w - 128))])
    return f"""// Written by specs/orbital_station/make_specs.py. Change the script, not this file.
(
    id: "{rid}",
    level_id: "{rid}",
    world_x: {world_x},
    world_y: {world_y},
    px_wid: {w},
    px_hei: {h},
    fill_collision: "empty",
    bg_color: "{BG}",
    biome: "{theme}",
    parallax_theme: "{theme}",
    music_track: "{music}",
    entities: [
        {body}
    ],
    connect_to: [],
)
"""


# ---------------------------------------------------------------------------
# The rooms
# ---------------------------------------------------------------------------

#: The freight lane's pen, in the dock and in the bay: its gate is a lock
#: wall that stands until customs clears the crate.
PEN_GATE_W, PEN_GATE_H = 96, 144


def gate_dock():
    """The gate deck. From the west: the temporary link home, the arrival
    and the clear floor where the ring and its terminal stand; the freight
    track from the ring, the customs desk under the manifest board; the
    stairs up to the habitation ring; the quarantine pen, which holds the
    crate and shuts the freight lane to the docking bay."""
    rid = DOCK
    _, _, w, h = ROOMS_AT[rid]
    floor = 704
    pen_x = 1712
    e = shell(w, h, floor) + [
        # The arrival, with the floor clear from x 128 to 640 for the ring
        # (192 wide) and its terminal, to the vault (656 high). Until the
        # ring's return terminal exists, the arrival is also the temporary
        # link home (one way, to the hub): the world check refuses a landing
        # that no zone names, and nothing names this one until the gate
        # network's catalog does.
        # The ring (GALACTIC, `data/gate_network.ron`): its zone is the
        # arrival. Its terminal stands beside it, and LEGAL is written on it.
        *gates.as_ron(gates.ring("Orbital Gate (Dock)", ARRIVAL, 112, floor, "the gate")),
        *gates.as_ron(gates.terminal("Orbital Gate (Dock)", "orbital_station_gate_terminal", 412, floor)),
        start(rid, 460, floor),
        label("dock_ring_floor", 160, 400, "THE RING: KEEP THIS DECK CLEAR", 440),
        # Freight: the track from the ring, a stack waiting for the hauler.
        prop("dock_conveyor", "orbital_conveyor", 640, floor, 480, 48),
        prop("dock_freight", "orbital_freight_stack", 1128, floor, 180, 120),
        # Customs: the desk, its officer, the board over it.
        prop("dock_customs_desk", "orbital_customs_desk", 1320, floor, 200, 110),
        resident("npc_station_okafor", "station_okafor", 1420, floor, roam=False),
        prop("dock_manifest_board", "orbital_manifest_board", 1060, 400, 300, 150),
        door("dock_habitation_door", "up to the habitation ring", 1600, floor, HAB, "hab_dock_door"),
        # The quarantine pen: a roof over the east end, its gate, the crate,
        # and the freight lane's door to the bay behind it.
        solid("pen_roof", pen_x, floor - PEN_GATE_H - 16, w - pen_x - WALL, 16),
        lock("dock_quarantine_gate", "the gate of the quarantine pen", pen_x, floor - PEN_GATE_H, PEN_GATE_W, PEN_GATE_H,
             CLEARED, look="orbital_quarantine_gate"),
        prop("dock_consignment", "orbital_consignment", 1880, floor, 72, 52),
        door("dock_freight_door", "the freight lane to the docking bay", 2128, floor, BAY, "bay_freight_door"),
    ]
    return rid, room(rid, e, "THE GATE DECK: EVERYTHING THE RING BRINGS STOPS AT CUSTOMS")


def docking_bay():
    """The docking bay, open on space behind its field. The hauler hangs in
    its berth over the floor; a gantry climbs from the floor to a walk under
    its spine and on to the observation deck's door, high in the east."""
    rid = BAY
    _, _, w, h = ROOMS_AT[rid]
    floor = 960
    walk = 576
    e = shell(w, h, floor) + [
        # The freight lane from the gate dock, behind its own pen gate.
        door("bay_freight_door", "the freight lane to the gate deck", 48, floor, DOCK, "dock_freight_door"),
        solid("pen_roof", WALL, floor - PEN_GATE_H - 16, 224 - WALL, 16),
        lock("bay_quarantine_gate", "the bay's gate of the freight lane", 128, floor - PEN_GATE_H, PEN_GATE_W, PEN_GATE_H,
             CLEARED, look="orbital_quarantine_gate"),
        start(rid, 400, floor),
        # The hauler in its berth, and freight under it.
        prop("bay_hauler", "orbital_hauler", 720, walk - 40, 900, 300),
        prop("bay_freight_west", "orbital_freight_stack", 320, floor, 180, 120),
        prop("bay_freight_east", "orbital_freight_stack", 1500, floor, 180, 120),
        prop("bay_shuttle", "orbital_shuttle", 1880, floor, 300, 140),
        resident("npc_station_lindqvist", "station_lindqvist", 1240, floor),
        # The gantry: steps of 64 from the floor up to the walk.
        ledge("gantry_1", 560, 896, 128),
        ledge("gantry_2", 640, 832, 128),
        ledge("gantry_3", 560, 768, 128),
        ledge("gantry_4", 640, 704, 128),
        ledge("gantry_5", 560, 640, 128),
        ledge("hauler_walk", 640, walk, 1760),
        resident("npc_station_reyes", "station_reyes", 1560, walk, roam=False),
        coin("bay_spine_coin", 1100, walk - 40),
        # The landing of the observation deck's door, high in the east.
        solid("east_landing", 2400, walk, w - 2400 - WALL, 32),
        door("bay_observation_door", "up to the observation deck", 2448, walk, VIEW, "view_bay_door"),
        door("bay_lift_door", "the cargo lift to the service deck", 2448, floor, SERVICE, "service_lift_door"),
        # The space elevator's terminus, east of the bay (`specs/eureka`).
        door("bay_tether_door", "to the tether terminus", 2300, floor, "orbital_station_tether_terminus", "terminus_bay_door"),
    ]
    return rid, room(rid, e, "THE DOCKING BAY: THE HAULER REMAINDER IN ITS BERTH", theme=OPEN_THEME)


def observation_deck():
    """The viewpoint: a long window on the planet, benches before it."""
    rid = VIEW
    _, _, w, h = ROOMS_AT[rid]
    floor = 704
    e = shell(w, h, floor) + [
        door("view_bay_door", "down to the docking bay", 48, floor, BAY, "bay_observation_door"),
        start(rid, 160, floor),
        prop("view_bench_west", "orbital_bench", 360, floor, 160, 40),
        prop("view_bench_east", "orbital_bench", 960, floor, 160, 40),
        resident("npc_neil_ongras_turfson", "station_neil", 600, floor, h=72, roam=False),
        resident("npc_station_wen", "station_wen", 1100, floor, roam=False),
        door("view_habitation_door", "to the habitation ring", w - 112, floor, HAB, "hab_view_door"),
    ]
    return rid, room(rid, e, "THE OBSERVATION DECK", theme=OPEN_THEME)


def habitation():
    """The habitation ring: bunks, the galley, the green racks."""
    rid = HAB
    _, _, w, h = ROOMS_AT[rid]
    floor = 704
    e = shell(w, h, floor) + [
        door("hab_view_door", "to the observation deck", 48, floor, VIEW, "view_habitation_door"),
        start(rid, 160, floor),
        prop("hab_bunks", "orbital_bunks", 240, floor, 220, 160),
        prop("hab_galley", "orbital_galley", 620, floor, 260, 120),
        resident("npc_station_pell", "station_pell", 760, floor, roam=False),
        door("hab_dock_door", "down to the gate deck", 1100, floor, DOCK, "dock_habitation_door"),
        prop("hab_hydroponics", "orbital_hydroponics", 1400, floor, 240, 140),
        prop("hab_bunks_east", "orbital_bunks", 1760, floor, 220, 160),
        door("hab_service_door", "to the service deck", w - 112, floor, SERVICE, "service_hab_door"),
    ]
    return rid, room(rid, e, "THE HABITATION RING")


def service_deck():
    """The service deck: the plant, the cargo lift down to the bay, the
    hatch of the damaged spur, and the spoke to the gravity works."""
    rid = SERVICE
    _, _, w, h = ROOMS_AT[rid]
    floor = 832
    # The spur's end of the deck, shut in: the west wall, a roof, and the
    # hatch under the roof's east end. The spur's door is behind it.
    hatch_x = 208
    e = shell(w, h, floor) + [
        solid("hatch_roof", WALL, floor - 144, hatch_x + 64 - WALL, 16),
        lock("service_spur_hatch", "the hatch of the damaged spur", hatch_x, floor - 128, 64, 128, SPUR_PASS,
             look="orbital_spur_hatch"),
        door("service_spur_door", "into the damaged spur", 64, floor, SPUR, "spur_service_door"),
        door("service_spoke_door", "the spoke to the gravity works", 400, floor, SPOKE, "spoke_station_door"),
        start(rid, 520, floor),
        prop("service_plant", "orbital_machinery", 600, floor, 300, 200),
        resident("npc_station_rahimi", "station_rahimi", 980, floor),
        door("service_lift_door", "the cargo lift to the docking bay", 1200, floor, BAY, "bay_lift_door"),
        door("service_hab_door", "to the habitation ring", w - 112, floor, HAB, "hab_service_door"),
    ]
    return rid, room(rid, e, "THE SERVICE DECK")


def damaged_spur():
    """The damaged spur: dark, the deck broken over live cables, and the
    maintenance crawlers that do not stop. A fall on the cables starts the
    spur again at its door. The breaker and the crew's cache are at the end."""
    rid = SPUR
    _, _, w, h = ROOMS_AT[rid]
    floor = 832
    pit = h - 16
    # The deck: (x0, x1) of each piece that stands; the gaps between have
    # live cable at their foot. Each gap is 128 or 160: a running jump.
    decks = [(0, 560), (688, 1200), (1360, 1760), (1888, w)]
    e = [
        solid("vault", 0, 0, w, 48),
        solid("west_wall", 0, 48, WALL, floor - 48),
        solid("east_wall", w - WALL, 48, WALL, floor - 48),
    ]
    for i, (x0, x1) in enumerate(decks):
        e.append(solid(f"deck_{i}", x0, floor, x1 - x0, h - floor))
    for i, ((_, a), (b, _)) in enumerate(zip(decks, decks[1:])):
        e.append(solid(f"gap_{i}_bed", a, pit, b - a, 16))
        e.append(hazard(f"gap_{i}_cables", a, pit - 16, b - a))
    e += [
        door("spur_service_door", "back to the service deck", 48, floor, SERVICE, "service_spur_door"),
        start(rid, 160, floor),
        # Fallen plate overhead, low enough to duck a jump into.
        solid("fallen_plate", 900, 48, 160, 560),
        foe(rid, 1, "npc_searchlight_crawler", 1000, floor, 48, 34),
        foe(rid, 2, "npc_searchlight_crawler", 1600, floor, 48, 34),
        foe(rid, 3, "npc_exploding_mite", 2100, floor, 32, 28),
        foe(rid, 4, "npc_exploding_mite", 2200, floor, 32, 28),
        # A ledge over the last deck with a coin, out of the crawlers' way.
        ledge("spur_shelf", 1960, floor - 144, 192),
        coin("spur_shelf_coin", 2040, floor - 184, 25),
        switch("orbital_spur_breaker", 2400, floor, "Reset the spur's breaker"),
        entity("ChestSpawn", 2280, floor - 58, 76, 58, name="the crew's cache", reward="health:2"),
    ]
    return rid, room(rid, e, "THE DAMAGED SPUR: LIVE CABLES, NO AIR, NO CREW", music=SPUR_MUSIC)


def spoke():
    """The spoke from the station to the gravity works, under glass. In its
    middle the deck is gone where the spoke is open to space: a tram crosses
    the gap. At the works' end, the lower end of the works' lift, behind a
    gate that is let go from inside."""
    rid = SPOKE
    _, _, w, h = ROOMS_AT[rid]
    floor = 576
    gap_x0, gap_x1 = 1200, 1840
    tram_w = 192
    alcove_x = 2848
    e = [
        solid("vault", 0, 0, w, 48),
        solid("west_wall", 0, 48, WALL, floor - 48),
        solid("east_wall", w - WALL, 48, WALL, floor - 48),
        solid("deck_station", 0, floor, gap_x0, h - floor),
        solid("deck_works", gap_x1, floor, w - gap_x1, h - floor),
        solid("gap_bed", gap_x0, h - 16, gap_x1 - gap_x0, 16),
        hazard("gap_vacuum", gap_x0, h - 32, gap_x1 - gap_x0),
        door("spoke_station_door", "back to the service deck", 48, floor, SERVICE, "service_spoke_door"),
        start(rid, 160, floor),
        # The tram: a car that runs to and fro over the gap, at the deck's height.
        entity("MovingPlatform", gap_x0, floor, tram_w, 16, id="spoke_tram", name="the spoke tram",
               speed=110.0, sweep_dx=float(gap_x1 - gap_x0 - tram_w), visual="orbital_tram"),
        label("spoke_tram_sign", gap_x0 - 220, floor - 160, "TRAM: STAND ON THE CAR", 260),
        door("spoke_works_door", "on to the gravity works", 2560, floor, ORCHARD, ORCHARD_SPOKE_DOOR),
        # The lift's lower end, at the end of the spoke: an alcove shut in by
        # the east wall and a roof, behind a gate that is let go from inside.
        solid("alcove_roof", alcove_x, floor - 160, w - WALL - alcove_x, 16),
        lock("spoke_lift_gate", "the gate of the works' lift", alcove_x, floor - 144, 32, 144,
             "switch_spoke_lift_release_used"),
        switch("spoke_lift_release", alcove_x + 72, floor, "Let go of the lift gate"),
        door(SPOKE_LIFT_DOOR, "the lift up to the approach", alcove_x + 192, floor, APPROACH, APPROACH_LIFT_DOOR),
    ]
    return rid, room(rid, e, "THE SPOKE: TO THE GRAVITY WORKS", theme=OPEN_THEME)


ROOMS = (gate_dock, habitation, observation_deck, docking_bay, service_deck, damaged_spur, spoke)


def duplicate_rooms() -> list[str]:
    """Level ids that more than one world file holds: each file validates on
    its own, but the game joins all of them into one room set."""
    import json
    seen: dict[str, int] = {}
    for world in sorted(default_worlds_dir().glob("*.ldtk")):
        for level in json.loads(world.read_text())["levels"]:
            seen[level["identifier"]] = seen.get(level["identifier"], 0) + 1
    return sorted(rid for rid, n in seen.items() if n > 1)


def run(*args) -> tuple[int, str]:
    done = subprocess.run([sys.executable, "-m", "ambition_ldtk_tools", *args], cwd=TOOLS, capture_output=True, text=True)
    print(done.stdout + done.stderr)
    return done.returncode, done.stdout + done.stderr


def main(argv):
    made = []
    for build in ROOMS:
        rid, text = build()
        path = HERE / f"{rid}.ron"
        path.write_text(text)
        made.append(path)
        print(path.relative_to(TOOLS))
    if "--apply" not in argv:
        return 0
    world = WORLD_LDTK.read_text()
    for path in made:
        if f'"identifier": "{path.stem}"' in world:
            code, _ = run("level", "delete", path.stem, "--ldtk", str(WORLD_LDTK), "--in-place", "--no-repair")
            if code != 0:
                return code
    # Each room's doors name the others: the world is whole only after the
    # last room, so until then it is enough that each was written.
    for path in made:
        code, out = run("area", "create", str(path), "--ldtk", str(WORLD_LDTK), "--replace-existing")
        if code != 0 and "wrote new level" not in out and "replaced level" not in out:
            return code
    code, _ = run("validate", str(WORLD_LDTK))
    twice = duplicate_rooms()
    if twice:
        print(f"error: rooms that two world files hold: {twice}")
        return 1
    return code


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
