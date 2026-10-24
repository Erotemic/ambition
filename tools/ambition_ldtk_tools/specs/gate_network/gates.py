"""The entities of a dialed gate: a ring, its portal, its loading zone and
its terminal. One place writes them, so each room generator that holds a ring
(`specs/math_town`, `specs/primeval`, the hub's `make_gates.py`, ...) states
the same shape.

A dialed ring's zone names no target: the gate network's live link decides
where it leads (`game/ambition_content/assets/data/gate_network.ron`). Its
portal is commanded by the network (`gate_portals.ron`, `Dialed`). The
terminal is a `GateTerminal` switch on the face of a dialer pedestal (the
`gate_dialer` prop): a press opens the terminal panel.

Sizes, in world units: the ring and its portal are 192 x 192 and stand on the
floor; the zone is the middle 96 x 192 of the ring. The dialer is 64 x 56;
its switch is 24 x 32 on its face, low enough that a standing body touches
it.
"""
from __future__ import annotations

import json

RING = 192
ZONE_W = 96
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


def ring(label: str, zone: str, x: int, floor: int, name: str) -> list[tuple]:
    """A dialed ring standing on `floor` with its left edge at `x`: the ring
    prop, the portal prop and the zone. `label` names the props, which each
    portal owns alone ("<label> Ring", "<label> Portal")."""
    top = floor - RING
    return [
        ("Prop", x, top, RING, RING, dict(name=f"{label} Ring", kind="gate_ring")),
        ("Prop", x, top, RING, RING, dict(name=f"{label} Portal", kind="gate_portal")),
        ("LoadingZone", x + (RING - ZONE_W) // 2, top, ZONE_W, RING,
         dict(id=zone, name=name, activation="Door", target_room="", target_zone="", bidirectional=False)),
    ]


def terminal(label: str, terminal_id: str, x: int, floor: int) -> list[tuple]:
    """A dialer pedestal on `floor` with its left edge at `x`, and the
    terminal switch on its face."""
    return [
        ("Prop", x, floor - DIALER_H, DIALER_W, DIALER_H, dict(name=f"{label} Dialer", kind="gate_dialer")),
        ("Switch", x + (DIALER_W - SWITCH_W) // 2, floor - SWITCH_H - 12, SWITCH_W, SWITCH_H,
         dict(id=terminal_id, name=f"{label} terminal", prompt="Dial the gate", target_encounter="", action="GateTerminal")),
    ]


def as_ron(entities: list[tuple]) -> list[str]:
    return [ron_entity(t, x, y, w, h, **f) for t, x, y, w, h, f in entities]


def as_yaml(level_id: str, entities: list[tuple], comment: str = "") -> str:
    head = "".join(f"# {line}\n" for line in comment.splitlines())
    body = "\n".join(yaml_entity(t, x, y, w, h, **f) for t, x, y, w, h, f in entities)
    return f"{head}level_id: {level_id}\nentities:\n{body}\n"
