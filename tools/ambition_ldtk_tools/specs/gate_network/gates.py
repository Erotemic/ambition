"""The entities of a dialed gate: a ring, its portal, its loading zone and
its terminal. One place writes them, so each room generator that holds a ring
(`specs/math_town`, the hub's `make_gates.py`, ...) states
the same shape.

A dialed ring's zone names no target: the gate network's live link decides
where it leads (`game/ambition_content/assets/data/gate_network.ron`). Its
portal is commanded by the network (`gate_portals.ron`, `Dialed`). The
terminal is a `GateTerminal` switch on the face of a dialer pedestal (the
`gate_dialer_triangle` prop beside a triangle ring, `gate_dialer` beside a
round one): a press opens the terminal panel.

Sizes, in world units: the ring and its portal are 288 x 288 and stand on the
floor; the zone is the middle 144 x 288 of the ring. An arch (a travel portal
drawn at a door's size: `jungle_arch`, `starlit_arch`) is 144 or 132 x 174
and its zone the 48 x 146 of its opening (`SHAPE_BOXES`). The dialer is
64 x 56;
its switch is 24 x 32 on its face, low enough that a standing body touches
it.
"""
from __future__ import annotations

import json

RING = 288
#: A ring's shape: its ring and portal prop kinds. The triangle is the gate
#: network's main look; the round ring is the Jungle's (Jon, 2026-10-31:
#: "Let's make the triangle the main variant though. Put the circle as the
#: gate for the jungle dimension.").
RING_SHAPES = {
    "triangle": ("triangle_gate_ring", "triangle_gate_portal"),
    "round": ("gate_ring", "gate_portal"),
    # Travel portals at a door's size (WORLD-LAYERS JL-2, MG-2): the
    # Jungle's mossy stone, Magical's starlit stone.
    "jungle_arch": ("jungle_arch_ring", "jungle_arch_portal"),
    "starlit_arch": ("starlit_arch_ring", "starlit_arch_portal"),
}
#: A ring's shape: its prop box (width, height) and its zone in that box
#: (x, y, width, height). An arch's box is its art's body at 1.45 px a
#: unit (the Jungle's fern makes its body wider); its zone is its opening,
#: down to the floor.
SHAPE_BOXES = {
    "triangle": ((288, 288), (72, 0, 144, 288)),
    "round": ((288, 288), (72, 0, 144, 288)),
    "jungle_arch": ((144, 174), (42, 28, 48, 146)),
    "starlit_arch": ((132, 174), (42, 28, 48, 146)),
}
#: A ring's shape: the prop kind of its dialer, whose dial has the ring's
#: shape (Jon, 2026-11-11: "the dial devices need a corresponding version
#: based on the shape of the gate").
DIALER_KINDS = {
    "triangle": "gate_dialer_triangle",
    "round": "gate_dialer",
    # An arch's keystone carries the network's Δ.
    "jungle_arch": "gate_dialer_triangle",
    "starlit_arch": "gate_dialer_triangle",
}
ZONE_W = 144
DIALER_W, DIALER_H = 64, 56
SWITCH_W, SWITCH_H = 24, 32


def _value(v) -> str:
    if isinstance(v, bool):
        return "true" if v else "false"
    if isinstance(v, (int, float)):
        return str(v)
    return json.dumps(v)


def ron_entity(etype: str, x: int, y: int, w: int, h: int, **fields) -> str:
    """One entity line of an `area create` spec."""
    text = ", ".join(f"{k}: {_value(v)}" for k, v in fields.items())
    return f'( type: "{etype}", px: [{x}, {y}], size: [{w}, {h}], fields: ( {text} ) ),'


def yaml_entity(etype: str, x: int, y: int, w: int, h: int, **fields) -> str:
    """One entity of an `entity add` YAML spec."""
    lines = [f"  - type: {etype}", f"    px: [{x}, {y}]", f"    size: [{w}, {h}]", "    fields:"]
    for k, v in fields.items():
        lines.append(f"      {k}: {json.dumps(v) if isinstance(v, str) else _value(v)}")
    return "\n".join(lines)


def ring(label: str, zone: str, x: int, floor: int, name: str, shape: str = "triangle") -> list[tuple]:
    """A dialed ring standing on `floor` with its left edge at `x`: the ring
    prop, the portal prop and the zone. `label` names the props, which each
    portal owns alone ("<label> Ring", "<label> Portal"); `shape` is a key of
    `RING_SHAPES`. The ring's props are its picture, so the zone draws no
    door marker over them (`presentation: SceneArt`)."""
    (w, h), (zx, zy, zw, zh) = SHAPE_BOXES[shape]
    top = floor - h
    ring_kind, portal_kind = RING_SHAPES[shape]
    return [
        ("Prop", x, top, w, h, dict(name=f"{label} Ring", kind=ring_kind)),
        ("Prop", x, top, w, h, dict(name=f"{label} Portal", kind=portal_kind)),
        ("LoadingZone", x + zx, top + zy, zw, zh,
         dict(id=zone, name=name, activation="Door", target_room="", target_zone="", bidirectional=False,
              presentation="SceneArt")),
    ]


def terminal(label: str, terminal_id: str, x: int, floor: int, shape: str = "triangle") -> list[tuple]:
    """A dialer pedestal on `floor` with its left edge at `x`, and the
    terminal switch on its face. `shape` is the shape of its ring, a key of
    `DIALER_KINDS`."""
    return [
        ("Prop", x, floor - DIALER_H, DIALER_W, DIALER_H, dict(name=f"{label} Dialer", kind=DIALER_KINDS[shape])),
        ("Switch", x + (DIALER_W - SWITCH_W) // 2, floor - SWITCH_H - 12, SWITCH_W, SWITCH_H,
         dict(id=terminal_id, name=f"{label} terminal", prompt="Dial the gate", target_encounter="", action="GateTerminal")),
    ]


def as_ron(entities: list[tuple]) -> list[str]:
    return [ron_entity(t, x, y, w, h, **f) for t, x, y, w, h, f in entities]


def as_yaml(level_id: str, entities: list[tuple], comment: str = "") -> str:
    head = "".join(f"# {line}\n" for line in comment.splitlines())
    body = "\n".join(yaml_entity(t, x, y, w, h, **f) for t, x, y, w, h, f in entities)
    return f"{head}level_id: {level_id}\nentities:\n{body}\n"
