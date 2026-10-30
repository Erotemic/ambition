#!/usr/bin/env python3
"""Write the rooms of Eureka, the science dimension's institute town, and the
space elevator from it up to the orbital station.

    python specs/eureka/make_specs.py          # writes the specs
    python specs/eureka/make_specs.py --apply  # and makes the rooms

Run it from `tools/ambition_ldtk_tools`.

Jon (2026-10-29): the science dimension's first build is an inhabited town
("we are world building not quest building": no quest), and "the science
dimension should have a space elevator that brings you up to the orbital
station". Eureka is on the planet under the station, in the dimension's own
world file (`science.ldtk`): the station and GNU-ton's gravity works are
overhead, and the tether joins them by real geography. Gate address SCIENCE
arrives in its gate court.

- `eureka_gate_court`: the gate court. Travellers arrive at the ring (the
  gate network's `gate_interface.yaml` places it on the clear floor at the
  west end).
- `eureka_quad`: the campus street, with the lecture hall's terrace over it.
  Doors to the patent office, the coil laboratory and the observatory.
- `eureka_patent_office`: the Patent Clerk's office.
- `eureka_coil_lab`: Tesseract Laboratories, Nikobolt Tesseract's coil hall.
- `eureka_observatory`: Carl Stargan's dome; ledges up to the telescope.
- `eureka_elevator_station`: the tether's ground station. Its car rests in the
  floor at the foot of the shaft; stand on it and it carries you up through
  the roof.
- `eureka_elevator_climb`: the climb, a tall room of sky. You arrive on the
  second car (a body that comes up through a floor edge stands 69 px over
  it, `rooms::spawn::edge_arrival`), and it carries you out of the top.
- `orbital_station_tether_terminus`: the station end, east of the docking
  bay: you arrive standing on its floor. A door takes you back down to the
  ground station; one to the docking bay.

The elevator is the engine's lift (`MovingPlatform` with `lift_dy`): it waits
for a rider, carries it, and goes back down to rest. Jump off on the climb and
you fall back to the ground station.
"""
from __future__ import annotations

import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent))

from district_kit import (  # noqa: E402
    WALL, WORLDS, add_entities, door, edge, entity, ledge, make_rooms, pad, resident, room, run, sign, solid, start,
)

import props  # noqa: E402  (this directory's scenery)

WORLD_LDTK = WORLDS / "science.ldtk"
GENERATOR = "specs/eureka/make_specs.py"

COURT = "eureka_gate_court"
QUAD = "eureka_quad"
PATENT = "eureka_patent_office"
LAB = "eureka_coil_lab"
DOME = "eureka_observatory"
STATION = "eureka_elevator_station"
CLIMB = "eureka_elevator_climb"
TERMINUS = "orbital_station_tether_terminus"
BAY = "orbital_station_docking_bay"

#: The contract with the gate network: the arrival point a dialled SCIENCE
#: names (its ring is `gate_interface.yaml`, written by
#: `gate_network/make_gates.py`).
ARRIVAL = "eureka_gate_arrival"
#: The docking bay's door to the terminus (the bay is `orbital_station`'s
#: room; its generator has the same door).
BAY_DOOR = "bay_tether_door"

BIOME = "eureka"
HEIGHTS = "eureka_heights"
STATION_THEME = "orbital"
BG = "#A9D3F0"
TOWN_MUSIC = "a_possible_morning"
PATENT_MUSIC = "graphite_and_rain"
LAB_MUSIC = "scaling_laws"
DOME_MUSIC = "billions_and_billions"
TETHER_MUSIC = "salt_steel_and_starlight"
STATION_MUSIC = "between_objectives"

#: Each room: (world_x, world_y), (width, height). The town is on the ground
#: (y 6880), under the station's rooms (y 3168 and up): the climb and the
#: terminus stand over the ground station's shaft.
ROOMS_AT = {
    COURT: ((0, 6880), (1600, 832)),
    QUAD: ((1856, 6880), (2816, 896)),
    PATENT: ((1856, 8064), (1024, 640)),
    LAB: ((3072, 8064), (1280, 576)),
    DOME: ((2560, 5600), (1280, 1024)),
    STATION: ((4928, 6880), (1600, 1408)),
    CLIMB: ((5312, 3424), (1280, 3200)),
    TERMINUS: ((5312, 2144), (1280, 1024)),
}

#: The elevator: the cars' size and speed; the foot of the ground station's
#: shaft (room x), and the climb's and terminus's shaft (room x).
CAR_W, CAR_H = 128, 22
CAR_SPEED = 300.0
CLIMB_SPEED = 340.0
STATION_SHAFT_X = 960
SHAFT_X = 576
#: A body that comes up through a floor edge stands this far over it.
EDGE_FEET = 68


def build(rid, entities, title, music=TOWN_MUSIC, biome=BIOME, **kw):
    at, size = ROOMS_AT[rid]
    # The room's scenery (`props.py`) is listed first: it draws behind.
    entities = [*props.scenery(rid), *entities]
    return rid, room(rid, entities, at=at, size=size, biome=biome, music=music, bg=BG, title=title, generator=GENERATOR, **kw)


def vertical_edge(zone, name, x, y, w, to_room, to_zone, bidirectional=True):
    """An edge exit across the top or the bottom of a room."""
    return entity("LoadingZone", x, y, w, 16, id=zone, name=name, activation="EdgeExit", target_room=to_room,
                  target_zone=to_zone, bidirectional=bidirectional)


def car(cid, name, x, top, dy, speed):
    """A car of the elevator: it rests with its top at `top` and carries a
    rider `dy` (up is negative)."""
    return entity("MovingPlatform", x, top, CAR_W, CAR_H, id=cid, name=name, speed=speed, lift_dy=float(dy), one_way=True)


def court():
    """The gate court. Floor 704. The ring stands on the clear floor of the
    west end (x 112..512); a plinth and a bench by it, the institute's sign,
    and the way east to the quad on the floor and along a raised walk."""
    rid = COURT
    _, (w, h) = ROOMS_AT[rid]
    floor, walk = 704, 512
    e = [
        solid("ground", 0, floor, w, h - floor),
        solid("west_wall", 0, 0, WALL, floor),
        solid("east_wall_high", w - WALL, 0, WALL, walk - 96),
        solid("east_wall_mid", w - WALL, walk + 16, WALL, floor - 96 - walk - 16),
        start(rid, 336, floor),
        ledge("founders_plinth", 640, floor - 64, 128),
        ledge("court_bench", 860, floor - 32, 160),
        sign("court_sign_gate", 336, floor - 300, "SCIENCE: arrivals for the Eureka Institute", 420),
        sign("court_sign_tether", 900, floor - 260, "The tether: ground station east of the quad. Cars on demand.", 520),
        # Steps up to the raised walk that runs on east over the quad.
        ledge("walk_step_1", 1080, floor - 64, 96),
        ledge("walk_step_2", 1160, floor - 128, 96),
        ledge("raised_walk", 1232, walk, w - 1232),
        edge("court_east_low", "to the quad", w - WALL, floor, QUAD, "quad_west_low"),
        edge("court_east_high", "along the raised walk", w - WALL, walk, QUAD, "quad_west_high"),
    ]
    return build(rid, e, "EUREKA: THE GATE COURT")


def quad():
    """The quad. The street (y 768) runs the room; the lecture hall's terrace
    (y 576) over its middle, the raised walk (y 512) at the west end. Doors on
    the street to the patent office and the coil laboratory, and on the
    terrace to the observatory. The cafe's awning is a step."""
    rid = QUAD
    _, (w, h) = ROOMS_AT[rid]
    street, terrace, walk = 768, 576, 512
    e = [
        solid("street", 0, street, w, h - street),
        solid("west_wall_high", 0, 0, WALL, walk - 96),
        solid("west_wall_mid", 0, walk + 16, WALL, street - 96 - walk - 16),
        solid("east_wall", w - WALL, 0, WALL, street - 96),
        edge("quad_west_low", "to the gate court", 0, street, COURT, "court_east_low"),
        edge("quad_west_high", "along the raised walk", 0, walk, COURT, "court_east_high"),
        start(rid, 120, street),
        ledge("raised_walk", WALL, walk, 560),
        ledge("walk_stair_1", 560, street - 64, 96),
        ledge("walk_stair_2", 600, street - 128, 64),
        door("quad_patent_door", "into the patent office", 760, street, PATENT, "patent_door"),
        sign("patent_sign", 784, street - 150, "PATENT OFFICE (THIRD CLASS)", 300),
        ledge("cafe_awning", 1000, street - 112, 200),
        resident("npc_eureka_mina", "eureka_mina", 1100, street),
        sign("cafe_sign", 1100, street - 180, "THE UNCERTAIN CUP: coffee, probably", 340),
        resident("npc_eureka_priya", "eureka_priya", 1320, street),
        # The lecture hall: its steps up to the terrace, and the observatory's door on it.
        ledge("hall_step_1", 1440, street - 64, 96),
        ledge("hall_step_2", 1520, street - 128, 96),
        ledge("hall_terrace", 1600, terrace, 720),
        resident("npc_eureka_halvorsen", "eureka_halvorsen", 1800, terrace),
        door("quad_dome_door", "up to the observatory", 2180, terrace, DOME, "dome_door"),
        sign("dome_sign", 2204, terrace - 140, "OBSERVATORY: visiting hours after dark", 340),
        sign("hall_sign", 1960, terrace - 260, "THE LECTURE HALL OF THE EUREKA INSTITUTE", 460),
        door("quad_lab_door", "into Tesseract Laboratories", 2440, street, LAB, "lab_door"),
        sign("lab_sign", 2464, street - 150, "TESSERACT LABORATORIES: mind the coils", 360),
        resident("npc_eureka_wilbur", "eureka_wilbur", 2620, street),
        edge("quad_east", "to the tether's ground station", w - WALL, street, STATION, "station_west"),
    ]
    return build(rid, e, "THE QUAD")


def patent_office():
    """The patent office: the clerk behind a long counter, the card cabinet,
    and the archive gallery over it (one jump up from the cabinet's top)."""
    rid = PATENT
    _, (w, h) = ROOMS_AT[rid]
    floor, ceiling = h - 64, 32
    e = [
        solid("floor", 0, floor, w, 64),
        solid("ceiling", 0, 0, w, ceiling),
        solid("west_wall", 0, ceiling, WALL, floor - ceiling),
        solid("east_wall", w - WALL, ceiling, WALL, floor - ceiling),
        door("patent_door", "out to the quad", 40, floor, QUAD, "quad_patent_door"),
        start(rid, 140, floor),
        ledge("counter", 480, floor - 40, 240),
        ledge("file_cabinets", 820, floor - 96, 160),
        ledge("archive_gallery", 760, floor - 176, w - WALL - 760),
        resident("special_patent_clerk", "eureka_patent_clerk", 640, floor),
        sign("patent_sign_rules", 520, ceiling + 40, "All applications are processed relative to the applicant.", 520),
    ]
    return build(rid, e, None, music=PATENT_MUSIC)


def coil_lab():
    """Tesseract Laboratories: a high hall of coils. Gantries over the floor;
    the director's office door at the east end."""
    rid = LAB
    _, (w, h) = ROOMS_AT[rid]
    floor, ceiling = h - 64, 32
    e = [
        solid("floor", 0, floor, w, 64),
        solid("ceiling", 0, 0, w, ceiling),
        solid("west_wall", 0, ceiling, WALL, floor - ceiling),
        solid("east_wall", w - WALL, ceiling, WALL, floor - ceiling),
        door("lab_door", "out to the quad", 40, floor, QUAD, "quad_lab_door"),
        start(rid, 140, floor),
        ledge("crate", 200, floor - 80, 96),
        ledge("coil_gantry_west", 320, floor - 160, 240),
        ledge("coil_gantry_east", 720, floor - 224, 240),
        ledge("bench", 1000, floor - 48, 160),
        resident("npc_eureka_dmitri", "eureka_dmitri", 560, floor),
        sign("lab_sign_rules", 640, ceiling + 40, "DO NOT TOUCH THE COILS. The coils will touch you.", 520),
        sign("lab_sign_director", 1100, floor - 240, "N. TESSERACT, DIRECTOR: back soon", 320),
    ]
    return build(rid, e, None, music=LAB_MUSIC)


def observatory():
    """The observatory: a dome. Ledges spiral up the wall to the telescope's
    platform under the slit; Carl Stargan is at the eyepiece."""
    rid = DOME
    _, (w, h) = ROOMS_AT[rid]
    floor = h - 64
    platform = 320
    e = [
        solid("floor", 0, floor, w, 64),
        solid("dome", 0, 0, w, 32),
        solid("west_wall", 0, 32, WALL, floor - 32),
        solid("east_wall", w - WALL, 32, WALL, floor - 32),
        door("dome_door", "down to the quad", 40, floor, QUAD, "quad_dome_door"),
        start(rid, 140, floor),
        # Up the wall in steps of 80 (one jump), east then back west.
        *[ledge(f"stair_{k + 1}", x, floor - 80 * (k + 1), 160) for k, x in enumerate((240, 480, 720, 960, 720, 480, 240))],
        ledge("telescope_platform", 320, platform, 480),
        resident("npc_carl_stargan", "eureka_stargan", 560, platform),
        sign("dome_sign_slit", 640, 72, "The station: a pale dot, directly overhead", 440),
    ]
    return build(rid, e, None, music=DOME_MUSIC)


def elevator_station():
    """The ground station: a tall hall round the foot of the tether. The car
    rests in the floor (its top level with it) under the open roof of the
    shaft; the operator by it. The down car from the terminus lets you off
    on its platform here."""
    rid = STATION
    _, (w, h) = ROOMS_AT[rid]
    floor = h - 64
    sx, sw = STATION_SHAFT_X, CAR_W
    e = [
        solid("floor_west", 0, floor, sx, 64),
        solid("floor_east", sx + sw, floor, w - sx - sw, 64),
        solid("pit_floor", sx, floor + CAR_H + 8, sw, 64 - CAR_H - 8),
        solid("roof_west", 0, 0, sx, 48),
        solid("roof_east", sx + sw, 0, w - sx - sw, 48),
        solid("west_wall_high", 0, 48, WALL, floor - 96 - 48),
        solid("east_wall", w - WALL, 48, WALL, floor - 48),
        edge("station_west", "to the quad", 0, floor, QUAD, "quad_east"),
        start(rid, 200, floor),
        car("tether_car_ground", "the tether's car", sx, floor, -(floor + 60), CAR_SPEED),
        vertical_edge("station_to_climb", "up the tether", sx, 0, sw, CLIMB, "climb_from_station"),
        resident("npc_eureka_teodora", "eureka_teodora", 720, floor),
        # Where the down car lets you off (one way: up is the ride).
        pad("station_down_arrival", "the down car's platform", 360, floor),
        sign("station_sign", 760, 120, "EUREKA TETHER: GROUND STATION", 460),
        sign("station_sign_car", sx + sw // 2, floor - 200, "STAND ON THE CAR TO ASCEND", 320),
        sign("station_sign_altitude", sx + sw // 2, floor - 260, "Orbit: 35,786 km. Allow for the view.", 400),
        # Galleries round the hall.
        ledge("gallery_west", WALL, floor - 320, 480),
        ledge("gallery_east", w - 400, floor - 320, 400 - WALL),
    ]
    return build(rid, e, "THE TETHER: GROUND STATION")


#: The signs on the climb, by how far up the room they are (0 bottom, 1 top).
CLIMB_SIGNS = [
    (0.08, "12 km: the weather, now under you"),
    (0.30, "100 km: the Karman line. Space, technically"),
    (0.55, "2,000 km: low orbit, and its traffic"),
    (0.80, "20,000 km: the planet fits in the window"),
]


def elevator_climb():
    """The climb: a tall room of sky, the tether up its middle. The second car
    rests where a body coming up through the floor edge stands, and carries
    it out of the top."""
    rid = CLIMB
    _, (w, h) = ROOMS_AT[rid]
    top = h - EDGE_FEET
    e = [
        solid("west_wall", 0, 0, WALL, h),
        solid("east_wall", w - WALL, 0, WALL, h),
        vertical_edge("climb_from_station", "down to the ground station", SHAFT_X, h - 16, CAR_W, STATION, "station_to_climb"),
        car("tether_car_climb", "the tether's car", SHAFT_X, top, -(top + 60), CLIMB_SPEED),
        vertical_edge("climb_to_terminus", "up to the station", SHAFT_X, 0, CAR_W, TERMINUS, "terminus_from_climb"),
        start(rid, SHAFT_X + CAR_W // 2, top),
    ]
    for frac, text in CLIMB_SIGNS:
        e.append(sign(f"climb_sign_{int(frac * 100)}", SHAFT_X + CAR_W // 2 + 260, round(h * (1.0 - frac)), text, 440))
    return build(rid, e, None, music=TETHER_MUSIC, biome=HEIGHTS)


def terminus():
    """The station end of the tether: you come up through the floor and stand
    on it. A door to the docking bay, and the down car's door to the ground."""
    rid = TERMINUS
    _, (w, h) = ROOMS_AT[rid]
    floor = h - EDGE_FEET
    e = [
        solid("floor", 0, floor, w, h - floor),
        solid("vault", 0, 0, w, 48),
        solid("west_wall", 0, 48, WALL, floor - 48),
        solid("east_wall", w - WALL, 48, WALL, floor - 48),
        vertical_edge("terminus_from_climb", "down the tether", SHAFT_X, h - 16, CAR_W, CLIMB, "climb_to_terminus"),
        start(rid, SHAFT_X + CAR_W // 2, floor),
        door("terminus_down_door", "the down car to the ground station", 200, floor, STATION, "station_down_arrival",
             bidirectional=False),
        door("terminus_bay_door", "to the docking bay", w - 120, floor, BAY, BAY_DOOR),
        sign("terminus_sign", w // 2, 120, "TETHER TERMINUS: the planet is 35,786 km down", 520),
    ]
    return build(rid, e, "THE TETHER TERMINUS", music=STATION_MUSIC, biome=STATION_THEME)


ROOMS = (court, quad, patent_office, coil_lab, observatory, elevator_station, elevator_climb, terminus)

#: The docking bay's door to the terminus, on its floor west of the cargo
#: lift (the bay is the station generator's; it lists the same door).
BAY_DOOR_YAML = f"""# The docking bay's door to the tether terminus. Written by {GENERATOR}.
level_id: {BAY}
entities:
  - type: LoadingZone
    px: [2300, 864]
    size: [48, 96]
    fields:
      id: {BAY_DOOR}
      name: to the tether terminus
      activation: Door
      target_room: {TERMINUS}
      target_zone: terminus_bay_door
      bidirectional: true
"""

#: The gate network's ring and terminal in the gate court, if it has written
#: them: a regenerate of the rooms puts them back.
GATE_INTERFACE = HERE / "gate_interface.yaml"


def bay_has_door() -> bool:
    """The docking bay has its door to the terminus already (the zone's id
    is also the terminus door's target, so look in the bay's own entities)."""
    import json

    world = json.loads(WORLD_LDTK.read_text())
    bay = next((level for level in world["levels"] if level["identifier"] == BAY), None)
    zones = (e for layer in (bay or {}).get("layerInstances") or [] for e in layer["entityInstances"] if e["__identifier"] == "LoadingZone")
    return any(f["__identifier"] == "id" and f["__value"] == BAY_DOOR for e in zones for f in e["fieldInstances"])


def main(argv):
    made = []
    for make in ROOMS:
        rid, text = make()
        path = HERE / f"{rid}.ron"
        path.write_text(text)
        made.append(path)
        print(path.relative_to(HERE.parents[1]))
    if "--apply" not in argv:
        return 0
    if code := make_rooms(WORLD_LDTK, made):
        return code
    if not bay_has_door():
        if code := add_entities(BAY_DOOR_YAML, WORLD_LDTK, HERE / "bay_door.yaml"):
            return code
    if GATE_INTERFACE.exists():
        code, _ = run("entity", "add", str(GATE_INTERFACE), "--ldtk", str(WORLD_LDTK), "--in-place")
        if code:
            return code
    code, _ = run("validate", str(WORLD_LDTK))
    return code


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
