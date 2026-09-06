"""Every authored encounter mob names a character the game can build a body for.

⛔⛔ THE CRASH THIS EXISTS FOR, reported by Jon 2026-09-06 in the goblin fight:

    encounter wave mob `encounter:goblin_encounter:w1:5` is of kind
    `Custom("npc_goblin_brute")` and names no character that can build a body

⇒ TWO CORRECT CHANGES COLLIDED, MADE APART. The wave's `character: None` was
DELIBERATE and said so in a comment — "No catalog row is the goblin lab's heavy…
the brute keeps drawing the placeholder until somebody makes it. Visible debt beats
a body borrowing someone else's art." That reasoning depended on a placeholder
FALLBACK. AC6 later deleted the fallback because it "WAS A LIE", turning the
authored debt into a panic. Neither change was wrong; nothing checked the pair.
⚠ And by then the note's premise had also expired: `npc_goblin_brute` HAD become a
catalog row with its own art.

⭐ SO THE GUARD IS ON THE AUTHORED DATA, not on the spawn path. The panic is
correct and should stay: it refuses rather than substituting a body. What must not
happen is authoring a wave that reaches it — a defect that only appears when a
player walks into that specific fight, several rooms into the game.

⚠ `character: None` IS A DISTINCT SPELLING FROM AN ABSENT FIELD and both must
fail. The original bug was the absent form; a reader "fixing" it by writing the
explicit `None` would satisfy a naive presence check and crash identically.
"""

from __future__ import annotations

import pathlib
import re

REPO = pathlib.Path(__file__).resolve().parents[2]
ENCOUNTERS = REPO / "game/ambition_content/assets/data/encounters"
CATALOG = REPO / "game/ambition_content/assets/data/character_catalog.ron"

_MOB = re.compile(r"^\s*\(kind:\s*\"(?P<kind>[^\"]+)\"(?P<rest>.*)$")
_CHARACTER = re.compile(r"character:\s*Some\(\"(?P<id>[^\"]+)\"\)")
_CATALOG_ROW = re.compile(r'^\s*"(?P<id>[a-z0-9_]+)":\s*\(', re.M)


def _mob_rows() -> list[tuple[str, int, str, str]]:
    rows = []
    for f in sorted(ENCOUNTERS.glob("*.ron")):
        for lineno, line in enumerate(f.read_text(encoding="utf-8").split("\n"), 1):
            m = _MOB.match(line)
            if m:
                rows.append((f.name, lineno, m.group("kind"), m.group("rest")))
    return rows


def test_every_encounter_mob_names_a_character():
    """⛔ Absent AND explicit-`None` both fail — they crash identically."""
    rows = _mob_rows()
    # ⚠ ANTI-VACUITY, and it must clear the real corpus rather than zero: a
    # renamed directory or a changed row spelling would otherwise report a clean
    # bill of health over nothing at all.
    assert len(rows) >= 7, (
        f"only {len(rows)} mob row(s) parsed from {ENCOUNTERS}; the corpus or the "
        f"row spelling moved and this guard is reading almost nothing"
    )
    nameless = [
        f"{f}:{lineno} kind={kind}"
        for f, lineno, kind, rest in rows
        if not _CHARACTER.search(rest)
    ]
    assert not nameless, (
        "these encounter mobs name no character, so spawning one PANICS the game "
        "the moment a player reaches that wave — `kind` is a brain and never "
        f"substitutes a body: {nameless}"
    )


def test_every_named_character_is_a_catalog_row():
    """⚠ A typo'd id fails the same way as no id, one room further in."""
    catalog = set(_CATALOG_ROW.findall(CATALOG.read_text(encoding="utf-8")))
    assert len(catalog) > 20, (
        f"only {len(catalog)} catalog rows parsed; the catalog's shape moved and "
        f"this guard would pass every id"
    )
    unknown = []
    for f, lineno, kind, rest in _mob_rows():
        m = _CHARACTER.search(rest)
        if m and m.group("id") not in catalog:
            unknown.append(f"{f}:{lineno} character={m.group('id')}")
    assert not unknown, (
        f"these mobs name a character with no catalog row: {unknown}"
    )
