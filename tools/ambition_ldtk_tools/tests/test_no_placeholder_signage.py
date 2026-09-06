"""Player-visible DebugLabel signage must not ship with placeholder field defaults.

These labels carry authored instructional text in game worlds. The test rejects a
small known set of placeholder values while allowing ordinary authored signage."""

from __future__ import annotations

import json

from ambition_ldtk_tools.ldtk.paths import default_worlds_dir

WORLDS = default_worlds_dir()

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
