"""The scenery of Eureka's rooms: `Prop` entities (art, no collision) that
`make_specs.py` lists before each room's other entities, so they draw behind.

The art is the sprite tools' `targets/props/eureka_props.py`, drawn in each
room's own coordinates over the walk lines `make_specs.py` places (the
counter, the card cabinet, the archive gallery, the observatory's stair and
platform). Each piece names the top left of its frame in its room: the
numbers are the art's (its `PIECES` frames); change them with it.
"""
from __future__ import annotations

from district_scenery import piece

#: Room -> [(kind, frame x, frame y)], furthest back first.
SCENERY = {
    "eureka_quad": [
        # The quad between them (`targets/props/eureka_quad_props.py`).
        ("quad_raised_walk", 16, 536),
        ("quad_lecture_hall", 1430, 536),
        ("quad_lab_front", 2330, 300),
        ("eureka_patent_front", 662, 272),
        ("eureka_observatory_front", 2052, 208),
        ("quad_cafe", 990, 480),
        ("quad_fountain", 1250, 600),
    ],
    "eureka_patent_office": [
        ("eureka_patent_room", 16, 32),
    ],
    "eureka_observatory": [
        ("eureka_observatory_room", 0, 0),
    ],
    "eureka_coil_lab": [
        ("eureka_lab_room", 16, 32),
    ],
    "eureka_test_range": [
        ("eureka_range_field", 200, 560),
        ("eureka_range_tower", 1960, 270),
        ("eureka_range_rocket", 660, 140),
        ("eureka_range_bunker", 1670, 560),
    ],
    "eureka_elevator_station": [
        ("eureka_tether_hall", 0, 0),
    ],
}


def scenery(room_id: str) -> list[str]:
    """The prop entity lines of `room_id`."""
    return [piece(kind, x, y, name=f"{kind}_{i}") for i, (kind, x, y) in enumerate(SCENERY.get(room_id, []))]
