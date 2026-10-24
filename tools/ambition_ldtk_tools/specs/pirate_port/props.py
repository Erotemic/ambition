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
    "pirate_port_docks": [
        ("port_gantry", 0, 220),
        ("port_warehouse", 828, 440),
        ("port_brig", 1496, 196),
        ("port_dock_clutter", 250, 728),
        ("port_lamp_post", 2584, 664),
    ],
}


def scenery(room_id: str) -> list[str]:
    """The prop entity lines of `room_id`."""
    return [piece(kind, x, y, name=f"{kind}_{i}") for i, (kind, x, y) in enumerate(SCENERY.get(room_id, []))]
