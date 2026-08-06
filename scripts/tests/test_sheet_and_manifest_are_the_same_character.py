"""A row's spritesheet and its manifest have to describe the same character.

Every catalog row names two files by hand:

    spritesheet: "sprites/alice_spritesheet.png",
    manifest:    "sprites/alice_spritesheet.ron",

and nothing required the stems to agree. A row pointing at one character's PNG
and another's RON resolves both files — so `declared_art_resolves` passes — and
then draws one animal with the other's frame rects, body box and feet anchor.
The result is a character whose picture and whose collision describe different
creatures, which is the exact complaint Jon raised about the snake and which took
a bespoke measurement to pin down.

⚠ **this is the fourth hand-made PAIRING in the hall's content, and the last one
unguarded.** The other three landed the same day: a pedestal's dialogue id to its
Yarn node, that node's speaker to the character's name, and a character row to
the map it lives in. Each was written after the pairing had already gone wrong
once.
"""

from __future__ import annotations

import re
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
CATALOG = REPO / "game/ambition_content/assets/data/character_catalog.ron"

_ROW = re.compile(r'^        "([a-z_0-9]+)": \(', re.M)
_SHEET = re.compile(r'spritesheet:\s*"([^"]+)"')
_MANIFEST = re.compile(r'manifest:\s*"([^"]+)"')


def _stem(path: str, suffix: str) -> str:
    return path.rsplit("/", 1)[-1].removesuffix(suffix)


def _disagreements(text: str) -> tuple[list[str], int]:
    """`(rows whose two files disagree, rows that named both)`."""
    disagreements: list[str] = []
    paired = 0
    for match in _ROW.finditer(text):
        start = match.end()
        following = text.find('\n        "', start)
        body = text[start : following if following > 0 else len(text)]
        sheet = _SHEET.search(body)
        manifest = _MANIFEST.search(body)
        if not (sheet and manifest):
            continue
        paired += 1
        drawn = _stem(sheet.group(1), ".png")
        described = _stem(manifest.group(1), ".ron")
        if drawn != described:
            disagreements.append(
                f"{match.group(1)} draws `{drawn}` and measures `{described}`"
            )
    return disagreements, paired


def test_every_row_draws_and_measures_the_same_character():
    disagreements, paired = _disagreements(CATALOG.read_text(encoding="utf8"))
    assert paired > 80, (
        f"only {paired} rows name both a spritesheet and a manifest — the scan "
        "is broken and would report no disagreements either"
    )
    assert not disagreements, (
        "catalog row(s) name a spritesheet and a manifest belonging to DIFFERENT "
        "characters. Both files exist, so every art check passes, and the "
        "character is drawn as one creature with another's frames and body "
        "box:\n  " + "\n  ".join(disagreements)
    )


def test_the_scan_would_notice_a_mismatched_pair():
    """The poison: a planted row whose two files disagree must be reported."""
    planted = (
        '        "npc_probe": (\n'
        '            display_name: "Probe",\n'
        '            spritesheet: "sprites/alice_spritesheet.png",\n'
        '            manifest: "sprites/bob_spritesheet.ron",\n'
        "        ),\n"
    )
    text = CATALOG.read_text(encoding="utf8")
    disagreements, _ = _disagreements(text + planted)
    assert disagreements == ["npc_probe draws `alice_spritesheet` and measures `bob_spritesheet`"], (
        f"a mismatched pair was not reported; got {disagreements}"
    )
