"""A Limit must never be spendable on the frame after a fighter respawns.

⛔⛔ THIS GUARD EXISTS BECAUSE DYING ONCE GRANTED THE LIMIT. A stock loss keeps
the body entity and calls `reset_body_clusters`. When the Limit lived in the
body's one generic meter, the reset handed back a FULL 100-point pool and a
system scheduled before `CombatSet::Trigger` had to empty it again before any
move was priced — a construction repair whose PLACEMENT was the defence.

⭐ THERE IS NO REPAIR TO PLACE ANY MORE. The Limit is the `smash.limit` slot of
a seat's `ActorResources` bank, declared EMPTY, and `reset_body_clusters`
returns every banked resource to its declared start — the same baseline the
seat was built from. So the property rests on two facts, and this guards both:

1. the Limit's declaration says `ResourceStart::Empty`;
2. the Rust test that proves the reset returns a bank to its declared start
   still exists (the behaviour itself is asserted there, not here).

⚠ TEXTUAL, AND THAT IS HONEST HERE: fact 1 IS a line of authored source, and
fact 2 is the existence of the arm that measures the behaviour.
"""

from __future__ import annotations

import pathlib
import re

REPO = pathlib.Path(__file__).resolve().parents[2]
LIMIT_SPEC = REPO / "crates/ambition_entity_catalog/src/smash_limit.rs"
BODY_CLUSTERS = REPO / "crates/ambition_platformer2d_core/src/body_clusters.rs"
RESET_ARM = "a_reset_returns_every_banked_resource_to_its_declared_start"


def _code(path: pathlib.Path) -> str:
    """Source with line comments stripped, so prose cannot satisfy a pattern."""
    return "\n".join(line.split("//", 1)[0] for line in path.read_text().splitlines())


def test_the_limit_is_declared_empty():
    code = _code(LIMIT_SPEC)
    match = re.search(
        r"pub fn declaration\(&self\)[^{]*\{(?P<body>.*?)\n    \}", code, re.S
    )
    assert match is not None, (
        "`LimitMeterFill::declaration` is gone from the Limit spec — this guard is "
        "checking a declaration that no longer exists. If the Limit's declaration "
        "moved, point this at its new home; do not delete it."
    )
    body = match.group("body")
    assert "LIMIT" in body and "ResourceStart::Empty" in body, (
        "the Limit is no longer declared EMPTY, so a seat is built — and every "
        "respawn resets it — with Limit it did not earn"
    )


def test_the_reset_to_the_declared_start_is_still_measured():
    code = _code(BODY_CLUSTERS)
    assert f"fn {RESET_ARM}(" in code, (
        f"`{RESET_ARM}` is gone, so nothing proves `reset_body_clusters` returns a "
        "bank to its declared start — which is the whole reason a respawned "
        "fighter's Limit is empty"
    )
    assert "reset_to_start()" in code, (
        "`reset_body_clusters` no longer calls `reset_to_start`; the arm above "
        "should be red"
    )
