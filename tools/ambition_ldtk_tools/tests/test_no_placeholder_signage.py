"""**No room ships a sign with the field default still in it.** (2026-07-29)

`drain_alley` had a `DebugLabel` reading `Label` — the LDtk field's default,
placed and never filled in. It rendered, in the shipped intro world, in the room
between the under-town grate and the System Boss shortcut.

# Why this is signage and not debug output

The name says debug; the contents do not. There are 134 of these across the
authored worlds and they carry the game's instructional voice —
*"Ratchet Climb: hop straight UP through each plate; they catch you on top."*,
*"Wall Run: walk right into the field — down rotates, walk UP the right wall."*
So they render for players by design, and a placeholder among them is shipped
text, not a leftover console line.

# Why a test rather than one more careful read

It was found by looking at a capture, and looking does not scale to 134 labels
across five worlds — the other 133 were fine, so a sweep by eye had a 1-in-134
chance of catching this per glance. The set of values worth refusing is small and
knowable, which is exactly when a guard beats attention.
"""

from __future__ import annotations

import json
from pathlib import Path

REPO = Path(__file__).resolve().parents[3]
WORLDS = REPO / "game" / "ambition_content" / "assets" / "worlds"

#: Values that mean "nobody wrote this yet". `Label` and `Debug Label` are the
#: LDtk entity definition's own defaults for `text` and `name`; the rest are the
#: usual stand-ins somebody types intending to come back.
PLACEHOLDERS = {
    "",
    "label",
    "debug label",
    "text",
    "todo",
    "tbd",
    "fixme",
    "placeholder",
    "lorem ipsum",
    "xxx",
    "asdf",
}


def _labels():
    """Every `DebugLabel` in every authored world, with where it is."""
    for path in sorted(WORLDS.glob("*.ldtk")):
        data = json.loads(path.read_text(encoding="utf-8"))
        levels = data.get("levels") or [
            level for world in data.get("worlds", []) for level in world.get("levels", [])
        ]
        for level in levels:
            for layer in level.get("layerInstances") or []:
                for entity in layer.get("entityInstances") or []:
                    if entity.get("__identifier") != "DebugLabel":
                        continue
                    fields = {
                        f["__identifier"]: f["__value"]
                        for f in entity.get("fieldInstances") or []
                    }
                    yield path.name, level["identifier"], entity.get("px"), fields


def test_no_authored_label_still_holds_its_field_default():
    offenders = [
        f"{world} / {room} at {px}: text={fields.get('text')!r}"
        for world, room, px, fields in _labels()
        if str(fields.get("text") or "").strip().lower() in PLACEHOLDERS
    ]
    assert not offenders, (
        "authored room signage still holds a placeholder — these RENDER, in the "
        "same voice as every tutorial sign:\n  " + "\n  ".join(offenders)
    )


def test_the_sweep_actually_has_labels_to_sweep():
    """The population floor.

    A sweep that silently finds nothing passes forever, and this one reads a glob
    of asset paths — the exact shape that goes quietly empty when a directory
    moves.
    """
    count = sum(1 for _ in _labels())
    assert count > 100, (
        f"only {count} DebugLabel(s) found across {WORLDS}; the authored worlds "
        "carry well over a hundred, so this sweep is looking in the wrong place "
        "and would pass no matter what the content said"
    )
