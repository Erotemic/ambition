"""A character-shaped row that is not in the `characters` map is not a character.

⛔ **This happened on 2026-08-05 and nothing caught it.** Four new rows were
inserted into `character_catalog.ron` by finding the first id that sorted after
each one — and the file has several id-keyed maps, so all four landed inside
`action_set_presets`. Every art check passed, because the sheets and portraits
they named really did exist. The characters simply were not in the game.

The only thing that noticed was a guard added an hour earlier for an unrelated
reason (`rendered_identities_are_registered`), and it noticed by accident: it
asks whether a renderer's declared id is registered, and these four were not.

So this asks the question directly, and from the FILE rather than the runtime:
a row carrying `display_name` and `spritesheet` is a character row, and a
character row belongs in exactly one place.

⚠ **it reads structure, not names.** A list of expected ids would have to be
maintained and would go stale the first time somebody adds a character; the
SHAPE of a character row is the durable fact.
"""

from __future__ import annotations

import re
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
CATALOG = REPO / "game/ambition_content/assets/data/character_catalog.ron"

#: Maps at the top level of the catalog, in the order they appear.
_MAP_HEADER = re.compile(r"^    ([a-z_]+):\s*\{", re.M)

#: `        "some_id": (` — a row at character-row indentation.
_ROW = re.compile(r'^        "([a-z_0-9]+)":\s*\(', re.M)


def _map_spans(text: str) -> list[tuple[str, int, int]]:
    """`(map name, start offset, end offset)` for each top-level map."""
    headers = [(m.group(1), m.start()) for m in _MAP_HEADER.finditer(text)]
    spans = []
    for index, (name, start) in enumerate(headers):
        end = headers[index + 1][1] if index + 1 < len(headers) else len(text)
        spans.append((name, start, end))
    return spans


def _row_body(text: str, start: int) -> str:
    """The text of one row, from its `(` to the matching `)`."""
    depth = 0
    inside_string = False
    escaped = False
    for offset in range(start, len(text)):
        char = text[offset]
        if escaped:
            escaped = False
            continue
        if char == "\\":
            escaped = True
            continue
        if char == '"':
            inside_string = not inside_string
            continue
        if inside_string:
            continue
        if char == "(":
            depth += 1
        elif char == ")":
            depth -= 1
            if depth == 0:
                return text[start : offset + 1]
    return text[start:]


def _misplaced_rows(text: str) -> tuple[list[str], int]:
    """`(descriptions of misplaced rows, count of well-placed ones)`.

    ⚠ **takes the TEXT**, so the poison below can feed it a planted row without
    writing to a tracked source file. A test that edits the repo to prove itself
    leaves residue the first time it fails partway.
    """
    spans = _map_spans(text)
    misplaced: list[str] = []
    characters_seen = 0
    for match in _ROW.finditer(text):
        body = _row_body(text, match.end() - 1)
        # The two fields that make a row a CHARACTER rather than a preset.
        if "display_name:" not in body or "spritesheet:" not in body:
            continue
        home = next(
            (name for name, start, end in spans if start <= match.start() < end),
            "<outside every map>",
        )
        if home == "characters":
            characters_seen += 1
        else:
            misplaced.append(f"{match.group(1)} is inside `{home}`")

    return misplaced, characters_seen


def test_every_character_shaped_row_lives_in_the_characters_map():
    text = CATALOG.read_text(encoding="utf8")
    assert any(name == "characters" for name, _, _ in _map_spans(text)), (
        "the catalog has no `characters:` map — this scan is reading the wrong "
        "file and is about to pass over nothing"
    )
    misplaced, characters_seen = _misplaced_rows(text)
    assert characters_seen > 80, (
        f"only {characters_seen} character rows found inside `characters:` — the "
        "scan is broken, and a broken scan reports no misplaced rows either"
    )
    assert not misplaced, (
        "character-shaped row(s) outside the `characters:` map, so the game does "
        "not have them even though their art resolves:\n  "
        + "\n  ".join(misplaced)
    )


def test_the_scan_would_notice_a_row_in_the_wrong_map():
    """The poison: the same row, planted in a preset map, has to be reported."""
    text = CATALOG.read_text(encoding="utf8")
    spans = _map_spans(text)
    presets = next((name for name, _, _ in spans if name != "characters"), None)
    assert presets, "the catalog has only one map, so misplacement is unsayable"

    opening = next(start for name, start, _ in spans if name == presets)
    cursor = opening + text[opening:].index("{") + 2
    planted = (
        '        "npc_probe": (\n'
        '            display_name: "Probe",\n'
        '            spritesheet: "sprites/probe_spritesheet.png",\n'
        "        ),\n"
    )
    misplaced, _ = _misplaced_rows(text[:cursor] + planted + text[cursor:])
    assert any("npc_probe" in row for row in misplaced), (
        "a character row planted in a preset map was not reported — the check "
        f"cannot see the mistake it exists for; it found {misplaced}"
    )
