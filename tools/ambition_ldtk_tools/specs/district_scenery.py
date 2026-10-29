"""Place a district's scenery props where the sprite tools drew them.

The district props (`targets/props/pirate_port_props.py`,
`ninja_village_props.py` in the sprite renderer) are drawn in their room's
own coordinates: each sheet's frame is a window of the room. A room places a
prop over the art's body box in that frame, so the art lands on the room's
walk lines. `piece(kind, frame_x, frame_y)` is that placement: the frame's
top left in the room, and the body box the published sheet records.
"""
from __future__ import annotations

import re
from pathlib import Path

from district_kit import prop

#: The published sheets, whose body boxes say where each prop's art is in its frame.
SPRITES = Path(__file__).resolve().parents[3] / "game/ambition_content_library/generated/runtime/sprites"


def body_box(kind: str) -> tuple[int, int, int, int]:
    """The body box of `kind`'s published sheet in frame pixels (x, y, w, h)."""
    text = (SPRITES / f"{kind}_spritesheet.ron").read_text()
    m = re.search(r"body_pixel_bbox: Some\(\(x: (\d+), y: (\d+), w: (\d+), h: (\d+)\)\)", text)
    if not m:
        raise SystemExit(f"{kind}: no body_pixel_bbox in its published RON; publish the sprite first")
    return tuple(int(v) for v in m.groups())


def piece(kind: str, frame_x: float, frame_y: float, px: float = 2.0, name: str | None = None) -> str:
    """Prop `kind` whose frame's top left is at (frame_x, frame_y) in the
    room; `px` is the sheet's pixels to one world unit."""
    bx, by, bw, bh = body_box(kind)
    return prop(name or kind, kind, frame_x + bx / px, frame_y + by / px, bw / px, bh / px)
