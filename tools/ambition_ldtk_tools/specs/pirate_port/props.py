"""The scenery of the pirate port's rooms: `Prop` entities (art, no
collision) that `make_specs.py` lists before each room's other entities, so
they draw behind.
"""
from __future__ import annotations


def scenery(room_id: str) -> list[str]:
    """The prop entity lines of `room_id`."""
    del room_id
    return []
