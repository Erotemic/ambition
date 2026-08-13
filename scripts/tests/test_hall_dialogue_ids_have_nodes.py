"""A pedestal that names a conversation nobody wrote stands there silent.

Every hall character pairs a catalog `hall_dialogue_id: Some("hall_npc_x")` with
a `title: hall_npc_x` node in a Yarn file, and the pairing is made BY HAND — six
times on 2026-08-05 alone, four for newly cast characters and two for the
snakes-on-planes pair.

⛔ **nothing checked it.** `every_character_says_something` asserts a character
has a LINE available, which a `barks.hall` entry satisfies on its own — so a row
whose `hall_dialogue_id` points at a node that was never written, or at one
deleted with a removed character, is silent when a player walks up to it and
green in the suite. Twenty-one characters were removed that same day and their
nodes deleted with them; getting one id wrong in either direction is a typo away.

⚠ **both directions.** A node nobody names is dead prose that survives every
cleanup, and it is how the file grows. It is reported separately because it is a
weaker fault than a mute pedestal.
"""

from __future__ import annotations

import re
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
CATALOG = REPO / "game/ambition_content/assets/data/character_catalog.ron"
DIALOGUE = REPO / "game/ambition_content/assets/dialogue"

#: `hall_dialogue_id: Some("hall_npc_whoever"),`
_DECLARED = re.compile(r'hall_dialogue_id:\s*Some\("([a-z_0-9]+)"\)')

#: `title: hall_npc_whoever`
_NODE = re.compile(r"^title:\s*([A-Za-z_0-9]+)\s*$", re.M)


def _declared_ids() -> set[str]:
    """Every pedestal id, from EVERY provider that stages hall characters.

    ⛔⛔ **this read the RON catalog alone, and the hall stopped being one
    provider's** (2026-08-14). AC6's content half moved the two plane swarms to
    Mary-O, which declares its rows in RUST — `hall_dialogue_id:
    Some("hall_npc_snakes_on_a_paper_plane")` in `game/ambition_demo_mary_o/src`
    — so their conversations read as prose nobody names and the check called two
    live pedestals dead. It was the weaker half of the test reporting a fault
    that did not exist, which is worse than not reporting one.

    ⭐ the same regex matches both forms: a RON row and a Rust literal spell the
    field identically, so widening the scan is a wider glob, not a second parser.
    """
    text = [CATALOG.read_text(encoding="utf8")]
    for path in sorted(REPO.glob("game/*/src/**/*.rs")):
        text.append(path.read_text(encoding="utf8"))
    return set(_DECLARED.findall("\n".join(text)))


def _authored_nodes() -> set[str]:
    nodes: set[str] = set()
    for path in sorted(DIALOGUE.glob("**/*.yarn")):
        nodes.update(_NODE.findall(path.read_text(encoding="utf8")))
    return nodes


def _mute_pedestals(declared: set[str], nodes: set[str]) -> list[str]:
    """Ids a pedestal names that no Yarn file writes.

    ⚠ **takes the two sets**, so the poison below can plant a broken pairing
    without editing the tracked catalog or a Yarn file to prove itself.
    """
    return sorted(declared - nodes)


def test_every_pedestal_names_a_conversation_that_exists():
    declared = _declared_ids()
    nodes = _authored_nodes()
    assert len(declared) > 40, (
        f"only {len(declared)} pedestals declare a dialogue id — the scan is "
        "broken and would report no mute pedestals either"
    )
    assert len(nodes) > 40, (
        f"only {len(nodes)} yarn nodes found under {DIALOGUE} — the scan is "
        "reading the wrong tree"
    )

    mute = _mute_pedestals(declared, nodes)
    assert not mute, (
        "hall pedestal(s) name a conversation no Yarn file writes, so they are "
        "silent when a player walks up and green in every existing test:\n  "
        + "\n  ".join(mute)
    )


def test_no_hall_conversation_is_written_for_nobody():
    """The other direction: prose that survives every cleanup.

    ⚠ **a WAIVED list rather than an assertion of emptiness**, because a hall
    node may legitimately exist for a character a provider registers in Rust
    rather than in this catalog. Each entry is a claim, and the test fails when
    one stops being needed as well as when a new orphan appears.
    """
    provider_owned: set[str] = set()

    orphans = {
        node
        for node in _authored_nodes()
        if node.startswith("hall_npc_") and node not in _declared_ids()
    }
    stale = sorted(provider_owned - orphans)
    assert not stale, (
        f"waived hall node(s) are no longer orphaned, so their excuse is dead: {stale}"
    )
    assert not sorted(orphans - provider_owned), (
        "hall conversation(s) exist for characters no catalog row names — dead "
        "prose that every cleanup will read past:\n  "
        + "\n  ".join(sorted(orphans - provider_owned))
    )


def test_the_scan_would_notice_a_pedestal_with_no_conversation():
    """The poison: a declared id with no node has to be reported."""
    nodes = _authored_nodes()
    planted = _declared_ids() | {"hall_npc_nobody_wrote_this"}
    assert _mute_pedestals(planted, nodes) == ["hall_npc_nobody_wrote_this"], (
        "a pedestal naming a conversation nobody wrote was not reported — the "
        "check cannot see the mistake it exists for"
    )
    # …and a real pairing is not reported, so the matcher is not always-true.
    assert not _mute_pedestals({"hall_npc_alice"}, nodes) or "hall_npc_alice" not in nodes
