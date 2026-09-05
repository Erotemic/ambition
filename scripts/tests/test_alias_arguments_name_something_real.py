"""An author-typed argument to a named Yarn alias must name something real.

⛔⛔ THIS IS THE HOLE THE OTHER FOUR AUTHORED-INTEGRITY GUARDS LEAVE, and unlike
them it landed on a defect that was already live. The repository checks that a
`gated_by` prepares, that a `condition("id", …)` names a published condition,
that a planning doc cites no fabricated id, and that a gated flag has a writer.
All four are about the CONDITION or the FACT. **None looks at the ARGUMENT a
named alias is called with.**

`boss_cleared("mockingbird")` and `quest_active("pirate_treasure")` pass an
author-typed string straight through to a save lookup. A misspelt one is
invisible: it parses, the alias is registered, the condition is published, the
evaluation succeeds — and it answers NO for the rest of the game.

⭐ MEASURED 2026-09-04, and the boss side was ALREADY BROKEN: all five authored
`boss_cleared` calls pass `"mockingbird"`, which is the BEHAVIOR id, while
`systems.rs:259` writes the save under `feature.config.id` — the PLACEMENT
(`BossSpawn-4308`). They can never match. Filed as awaiting-maintainer-decision
#57, because the repair is a design choice and not a spelling fix.

⚠ SO THIS GUARD DELIBERATELY ACCEPTS EITHER SPELLING. Its question is *"does
this name a real boss at all"*, which is answerable today and catches the typo
class. **Which of the two ids the API should take is #57's to answer**, and a
guard that pre-empted it would either fail on shipped content or bake in an
answer nobody has given. ⇒ When #57 lands, narrow this to the surviving id and
the other spelling becomes a red.
"""

from __future__ import annotations

import json
import re
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent.parent
DIALOGUE = REPO / "game/ambition_content/assets/dialogue"
WORLDS = REPO / "game/ambition_content/assets/worlds"
QUESTS = REPO / "game/ambition_content/src/quest.rs"

BOSS_CALL = re.compile(r'boss_cleared\(\s*"([^"]+)"')
QUEST_CALL = re.compile(r'quest_active\(\s*"([^"]+)"')


def _authored(pattern: re.Pattern[str]) -> dict[str, list[str]]:
    found: dict[str, list[str]] = {}
    for path in sorted(DIALOGUE.rglob("*.yarn")):
        for name in pattern.findall(path.read_text(encoding="utf-8", errors="replace")):
            found.setdefault(name, []).append(path.name)
    return found


def _real_bosses() -> set[str]:
    """Every id a shipped world gives a boss — placement AND behavior."""
    names: set[str] = set()
    for path in sorted(WORLDS.glob("*.ldtk")):
        world = json.loads(path.read_text(encoding="utf-8", errors="replace"))
        for level in world.get("levels", []):
            for layer in level.get("layerInstances") or []:
                for entity in layer.get("entityInstances") or []:
                    if entity.get("__identifier") != "BossSpawn":
                        continue
                    names.add(entity.get("iid", ""))
                    for field in entity.get("fieldInstances", []):
                        value = field.get("__value")
                        if not isinstance(value, str):
                            continue
                        # `PhaseScript:mockingbird` — the behavior id is the tail.
                        names.add(value.rsplit(":", 1)[-1])
                        names.add(value)
    return {n for n in names if n}


def _real_quests() -> set[str]:
    text = QUESTS.read_text(encoding="utf-8", errors="replace")
    return set(re.findall(r'QuestSpec::new\(\s*"([^"]+)"', text)) | set(
        re.findall(r'^\s*"([a-z0-9_]+)",\s*$', text, re.M)
    )


def test_every_boss_cleared_argument_names_a_real_boss() -> None:
    asked = _authored(BOSS_CALL)
    # ⛔ A FLOOR ABOVE THE LARGEST SINGLE FILE, not above zero. `boss_cleared`
    # is 3 calls in `kernel.yarn` and 2 in `cove.yarn`, so a non-empty check
    # survives losing either file entirely — which is not hypothetical: a poison
    # aimed at `kernel.yarn` alone left this green and read, for a minute, as a
    # floor that did not work.
    calls = sum(len(v) for v in asked.values())
    assert calls >= 4, (
        f"only {calls} `boss_cleared(\"…\")` call(s) across "
        f"{len({f for v in asked.values() for f in v})} file(s) — the call spelling "
        "this scans has changed, or a whole file stopped being read"
    )
    real = _real_bosses()
    assert len(real) >= 10, f"only {len(real)} boss id(s) found in shipped worlds"
    unknown = sorted(name for name in asked if name not in real)
    assert not unknown, (
        "authored dialogue asks about bosses no shipped world authors, so those "
        "branches never open:\n  "
        + "\n  ".join(f"{n}  (in {', '.join(sorted(set(asked[n])))})" for n in unknown)
    )


def test_every_quest_active_argument_names_a_real_quest() -> None:
    asked = _authored(QUEST_CALL)
    calls = sum(len(v) for v in asked.values())
    assert calls >= 2, (
        f"only {calls} `quest_active(\"…\")` call(s) in authored dialogue — the "
        "call spelling this scans has changed"
    )
    real = _real_quests()
    assert len(real) >= 5, f"only {len(real)} quest id(s) parsed from quest.rs"
    unknown = sorted(name for name in asked if name not in real)
    assert not unknown, (
        "authored dialogue asks about quests the roster does not define:\n  "
        + "\n  ".join(f"{n}  (in {', '.join(sorted(set(asked[n])))})" for n in unknown)
    )
