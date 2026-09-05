"""Jon's explicitly authored moves must stay identifiable in the code that implements them.

⭐⭐ WHY THIS GUARD EXISTS, in Jon's own words (2026-09-05): *"The entire point is
that this game is demoing the capabilities of an LLM to make a game, and every
decision or explicit authoring choice I make takes away from that claim."*

⇒ That makes agent-decided moves a FEATURE and maintainer-decided ones a fact
worth protecting. A polish pass that rewrites a move Jon asked for, without
knowing he asked, destroys evidence that cannot be reconstructed from the code —
and the code is the only place that evidence currently lives, as prose beside the
move it explains.

⛔ IT GUARDS THE FLOOR, NOT THE TEXT. Asserting an exact quote would break on any
reword; asserting a COUNT per file lets an agent improve the prose freely and
fails only when an attribution disappears. ⇒ The failure a reader gets is "this
file used to record a maintainer decision and no longer does", which is the
question worth asking.
"""

from __future__ import annotations

import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]

# ⚠ SEEDED FROM WHAT JON NAMED, then reconciled against the code the same day.
# The counts are the floor found on 2026-09-05, not a target: raising one is a
# decision, watching one fall silently is the failure.
FLOOR: dict[str, int] = {
    "game/ambition_content/src/pirate_admiral_moveset.rs": 3,
    "game/ambition_content/src/performer_moveset.rs": 9,
    "game/ambition_content/src/author_moveset.rs": 1,
    "game/ambition_content/src/officer_moveset.rs": 1,
    "game/ambition_content/src/projectile_polygon_moveset.rs": 2,
    "game/ambition_content/src/player_robot_moveset.rs": 1,
    "game/ambition_content/src/alice_moveset.rs": 2,
    # ⭐ ADDED 2026-09-05: "PCA needs to shoot a glider" was SATISFIED and
    # UNRECORDED, so nothing stopped a later pass from removing the ranged
    # glider without learning the maintainer had asked for it by name.
    "game/ambition_content/src/authored/perfect_cellular_automaton.rs": 1,
}

ATTRIBUTION = re.compile(r"JON'S DESIGN|Jon, \d{4}-\d{2}-\d{2}|Jon:|Jon, verbatim")


def _count(path: Path) -> int:
    return len(ATTRIBUTION.findall(path.read_text(encoding="utf-8", errors="replace")))


def test_maintainer_authored_moves_keep_their_attribution() -> None:
    lost: list[str] = []
    for rel, floor in FLOOR.items():
        path = ROOT / rel
        if not path.exists():
            lost.append(f"{rel}: the file is gone, and with it {floor} maintainer attribution(s)")
            continue
        found = _count(path)
        if found < floor:
            lost.append(f"{rel}: {found} attribution(s), was {floor}")
    assert not lost, (
        "maintainer attributions disappeared. Jon's explicitly authored moves are "
        "the one part of this roster an agent must not quietly rewrite, because "
        "the code comment is the only record that he asked:\n  " + "\n  ".join(lost)
    )


def test_the_attribution_pattern_still_matches_something() -> None:
    """⛔ ANTI-VACUITY. A regex that stopped matching would make every floor above
    read as zero-or-more and the guard would pass forever."""
    total = sum(_count(ROOT / rel) for rel in FLOOR if (ROOT / rel).exists())
    assert total >= 20, (
        f"only {total} maintainer attribution(s) found across every recorded file — "
        "the pattern has stopped matching, so this guard is checking nothing"
    )
