"""The scenery of the Ninja Village's rooms: `Prop` entities (art, no
collision) that `make_specs.py` lists before each room's other entities, so
they draw behind.

The art is the sprite tools' `targets/props/ninja_village_props.py`, drawn
in each room's own coordinates over the walk lines `make_specs.py` places
(the shrine's stair and terrace, the roofs' ridges, the cedars' boughs, the
dojo's beam). Each piece names the top left of its frame in its room: the
numbers are the art's (its `PIECES` frames); change them with it.
"""
from __future__ import annotations

from district_scenery import piece

BRANCH_STEP, FOREST_FLOOR = 384, 832

#: Room -> [(kind, frame x, frame y)], furthest back first.
SCENERY = {
    "ninja_village_gate_shrine": [
        ("village_shrine_stair", 950, 440),
        ("village_torii", 1180, 280),
        ("village_shrine_hall", 1330, 280),
        ("village_shrine_small_things", 530, 540),
    ],
    "ninja_village_street": [
        ("village_house_baker", 0, 470),
        ("village_house_weaver", 520, 440),
        ("village_tea_house", 1040, 500),
        ("village_dojo_front", 1520, 404),
        ("village_storehouse", 2000, 376),
        ("village_notice_board", 2186, 660),
        ("village_watch_house", 2440, 310),
    ],
    # The Bamboo Pass (`targets/props/ninja_pass_props.py`, drawn on the
    # room's numbers): each piece at its frame.
    "ninja_bamboo_pass": [("pass_bamboo", 0, 300), ("pass_bridge", 870, 770), ("pass_lookout", 1530, 500), ("pass_falls", 1880, 390)],
    "ninja_village_forest": [
        # Cedar i stands at the room's branch i (x i*384, +16 for the first;
        # y 384 + 64 i): its frame is 40 west of the branch and 300 over it.
        *[(f"village_cedar_{i}", i * BRANCH_STEP + (16 if i == 0 else 0) - 40, 448 + 64 * i - 300) for i in range(6)],
        ("village_clan_gatehouse", 2590, 590),
    ],
    "ninja_village_dojo": [
        ("village_dojo_entry", 16, 32),
        ("village_dojo_exam", 528, 32),
    ],
}


def scenery(room_id: str) -> list[str]:
    """The prop entity lines of `room_id`."""
    return [piece(kind, x, y, name=f"{kind}_{i}") for i, (kind, x, y) in enumerate(SCENERY.get(room_id, []))]
