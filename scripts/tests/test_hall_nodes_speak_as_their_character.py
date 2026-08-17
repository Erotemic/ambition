"""A pedestal's conversation has to be that character's, not somebody else's.

`test_hall_dialogue_ids_have_nodes` proves the node a pedestal names EXISTS.
This asks the next question: is it theirs? A hall node is written by copying a
neighbouring one and rewriting the lines, so the failure mode is a node that
resolves perfectly and opens with another character's name — and every check in
the repo passes, because the id paired, the art resolved, and a line was found.

⚠ **shared WORD, not an exact match, and the measurement decided that.** An
exact `display_name == speaker` rule reports 28 of the 124 pedestals, and every
one is a legitimate shortening a writer would defend: *Fretjaw, Cantina
Chieftain* speaks as `Fretjaw`, *Emmy Ethereal* as `Emmy`, *Pirate Admiral* as
`Admiral`, *Architect NPC* as `Architect`. A guard that cries wolf 28 times is
one nobody reads. Sharing one word of three-plus letters passes all 28 and still
catches a node that opens as an unrelated character, which is the actual mistake.
"""

from __future__ import annotations

import re
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
CATALOG = REPO / "game/ambition_content/assets/data/character_catalog.ron"
DIALOGUE = REPO / "game/ambition_content/assets/dialogue"

_ROW = re.compile(r'^        "([a-z_0-9]+)": \(', re.M)
_DISPLAY = re.compile(r'display_name:\s*"([^"]+)"')
_HALL_ID = re.compile(r'hall_dialogue_id:\s*Some\("([a-z_0-9]+)"\)')
_NODE = re.compile(r"^title:\s*([A-Za-z_0-9]+)\s*$\n---\n(.*?)^===", re.M | re.S)
#: `Speaker: line` at the start of a line, the Yarn convention this cast uses.
_SPEAKER = re.compile(r"^\s*([A-Za-z0-9 '\-\.\(\)]+?):", re.M)


def _pedestals() -> dict[str, tuple[str, str]]:
    """`character id -> (display name, hall node id)` for rows that name a node."""
    text = CATALOG.read_text(encoding="utf8")
    out: dict[str, tuple[str, str]] = {}
    for match in _ROW.finditer(text):
        start = match.end()
        following = text.find('\n        "', start)
        body = text[start : following if following > 0 else len(text)]
        display = _DISPLAY.search(body)
        hall = _HALL_ID.search(body)
        if display and hall:
            out[match.group(1)] = (display.group(1), hall.group(1))
    return out


def _nodes() -> dict[str, str]:
    nodes: dict[str, str] = {}
    for path in sorted(DIALOGUE.glob("**/*.yarn")):
        for match in _NODE.finditer(path.read_text(encoding="utf8")):
            nodes[match.group(1)] = match.group(2)
    return nodes


def _words(text: str) -> set[str]:
    """Name words worth matching on — three letters or more, punctuation off."""
    return {
        word
        for word in (part.strip(".,'()").lower() for part in text.split())
        if len(word) > 2
    }


def _strangers(
    pedestals: dict[str, tuple[str, str]], nodes: dict[str, str]
) -> list[str]:
    """Pedestals whose node never speaks a word of their own name."""
    strangers = []
    for character, (display, hall_id) in sorted(pedestals.items()):
        body = nodes.get(hall_id)
        if body is None:
            continue  # the pairing check owns that failure
        speakers = set(_SPEAKER.findall(body))
        if not any(_words(display) & _words(speaker) for speaker in speakers):
            strangers.append(
                f"{character} ({display}) -> {hall_id}, spoken by {sorted(speakers)}"
            )
    return strangers


def test_every_hall_conversation_is_spoken_by_its_own_character():
    pedestals = _pedestals()
    nodes = _nodes()
    assert len(pedestals) > 40, (
        f"only {len(pedestals)} pedestals name a hall node — the scan is broken "
        "and would report no strangers either"
    )
    strangers = _strangers(pedestals, nodes)
    assert not strangers, (
        "hall conversation(s) never speak a word of the character's own name, so "
        "the pedestal resolves and plays somebody else's prose:\n  "
        + "\n  ".join(strangers)
    )


def test_the_scan_would_notice_somebody_elses_prose():
    """The poison: a node that opens as an unrelated character is reported."""
    nodes = dict(_nodes())
    nodes["hall_npc_probe"] = "Somebody Entirely Different: I am not who you asked for.\n"
    planted = {"npc_probe": ("Probe Character", "hall_npc_probe")}
    assert _strangers(planted, nodes) == [
        "npc_probe (Probe Character) -> hall_npc_probe, "
        "spoken by ['Somebody Entirely Different']"
    ]
    # …and a shortened but genuine speaker is NOT reported, which is the case
    # that made an exact-match rule unusable.
    nodes["hall_npc_probe"] = "Fretjaw: I go by one name here.\n"
    planted = {"npc_probe": ("Fretjaw, Cantina Chieftain", "hall_npc_probe")}
    assert _strangers(planted, nodes) == []
