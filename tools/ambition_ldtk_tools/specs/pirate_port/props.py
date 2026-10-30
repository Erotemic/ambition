"""The scenery of the Pirate Port's rooms: `Prop` entities (art, no
collision) that `make_specs.py` lists before each room's other entities, so
they draw behind.

The art is the sprite tools' `targets/props/pirate_port_props.py`, drawn in
each room's own coordinates over the walk lines `make_specs.py` places (the
gallery, the roofs, the brig's yards, the gantry). Each piece names the top
left of its frame in its room: the numbers are the art's (its `PIECES`
frames); change them with it. A piece placed twice (a lamp post, the dock
clutter) names each place.
"""
from __future__ import annotations

from district_scenery import piece

#: Room -> [(kind, frame x, frame y)], furthest back first.
SCENERY = {
    "pirate_port_gate_plaza": [
        ("port_harbour_house", 990, 120),
        ("port_freight_booth", 520, 560),
        ("port_lamp_post", 104, 600),
    ],
    "pirate_port_waterfront": [
        ("port_waterfront_west", 0, 160),
        ("port_waterfront_middle", 540, 140),
        ("port_tavern_front", 1080, 160),
        ("port_waterfront_stair", 396, 560),
        ("port_chandler_stall", 750, 630),
        ("port_chandlery", 1660, 420),
        ("port_net_loft", 2076, 356),
        ("port_lamp_post", 1612, 664),
    ],
    "pirate_port_tavern": [
        ("port_tavern_room", 16, 32),
    ],
    # The Wreckers' Reef (`targets/props/pirate_reef_props.py`): each piece
    # at its frame.
    "pirate_wreckers_reef": [("reef_shore", 0, 520), ("reef_wreck", 860, 280)],
    # The Lighthouse Point (`targets/props/pirate_lighthouse_props.py`).
    "pirate_lighthouse_point": [("point_cottage", 290, 760), ("point_lighthouse", 420, 200)],
    "pirate_port_docks": [
        ("port_gantry", 0, 220),
        ("port_warehouse", 828, 440),
        ("port_brig", 1496, 196),
        ("port_dock_clutter", 250, 728),
        ("port_lamp_post", 2584, 664),
    ],
}


#: The pirate cove is not one of this generator's rooms (it predates it and is
#: edited in place), so its scenery is ADDED to it (`make_specs.py`,
#: `cove_scenery_yaml`), not listed in a room it writes.
COVE = "pirate_cove"
COVE_SCENERY = [
    ("cove_cave", 0, 0),
    ("cove_ledges", 300, 0),
    ("cove_doorways", 40, 438),
    ("cove_dressing", 120, 498),
]


def cove_scenery_yaml(generator: str) -> str:
    """The `entity add` YAML of the cove's scenery: one Prop per piece, over
    its art's body box."""
    from district_scenery import body_box

    rows = []
    for i, (kind, fx, fy) in enumerate(COVE_SCENERY):
        bx, by, bw, bh = body_box(kind)
        rows.append(f"""  - type: Prop
    px: [{round(fx + bx / 2)}, {round(fy + by / 2)}]
    size: [{max(1, round(bw / 2))}, {max(1, round(bh / 2))}]
    fields:
      name: {kind}_{i}
      kind: {kind}""")
    return f"# The pirate cove's scenery. Written by {generator}.\nlevel_id: {COVE}\nentities:\n" + "\n".join(rows) + "\n"


def scenery(room_id: str) -> list[str]:
    """The prop entity lines of `room_id`."""
    return [piece(kind, x, y, name=f"{kind}_{i}") for i, (kind, x, y) in enumerate(SCENERY.get(room_id, []))]
