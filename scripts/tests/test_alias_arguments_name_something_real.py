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

⭐ MEASURED 2026-09-04, and the boss side WAS broken: the authored
`boss_cleared` calls passed `"mockingbird"`, the BEHAVIOR id, while
`systems.rs:259` writes the save under `feature.config.id` — the PLACEMENT. They
could never match. Filed as awaiting-maintainer-decision #57, because the repair
was a design choice and not a spelling fix.

⚠ TWO CORRECTIONS TO THE SENTENCE ABOVE, both 2026-09-05 and both kept visible
because each was a claim this file made and then had to withdraw.
⛔ **"All FIVE authored calls" was never true — there are THREE.** Two of the
five raw matches are the Kernel Guide SAYING the call in spoken prose, which four
whole-file scanners counted as code until `executable_regions` landed. This
module now reads executable Yarn only.
✔ **And #57 is RULED and implemented**, so those three calls resolve today. The
paragraph is left standing rather than deleted because the guard's shape is still
explained by it.

⚠ THIS GUARD STILL ACCEPTS EITHER SPELLING, and that is now a deliberate
LEFTOVER rather than a deferral. Its question is *"does this name a real boss at
all"*, which catches the typo class; the id question is settled and the
narrowing — making the behaviour id a red — is owned by the Rust guard
`every_authored_boss_cleared_call_names_a_real_boss_placement`, which resolves
against real placements. ⇒ Nothing here should be tightened without checking
that arm first, or two guards end up asserting the same property.
"""

from __future__ import annotations

import json
import re
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent.parent
import sys  # noqa: E402
sys.path.insert(0, str(REPO / "scripts"))
from lib.yarn_source import executable_source  # noqa: E402
DIALOGUE = REPO / "game/ambition_content/assets/dialogue"
WORLDS = REPO / "game/ambition_content/assets/worlds"
QUESTS = REPO / "game/ambition_content/src/quest.rs"

BOSS_CALL = re.compile(r'boss_cleared\(\s*"([^"]+)"')
QUEST_CALL = re.compile(r'quest_active\(\s*"([^"]+)"')


def _authored(pattern: re.Pattern[str]) -> dict[str, list[str]]:
    """Arguments passed in EXECUTABLE Yarn only.

    ⛔⛔ THIS SCANNED WHOLE FILES AND THAT IS A GUARD THAT FAILS ON PROSE. A
    `.yarn` file is mostly spoken lines; only `<<…>>` is evaluated. `kernel.yarn`
    contains `Kernel Guide: boss_cleared("mockingbird") returned TRUE.` — a
    character EXPLAINING the call — and a whole-file regex reads it as one. ⇒ A
    character saying a misspelling would have reddened CI over text the
    interpreter never evaluates: a guard reporting a defect in the WRITING.
    ⭐ `scripts/lib/yarn_source.py` is the one definition, mirroring
    `dialogue_lint.rs::extract_command_calls`, which had it right all along.
    """
    found: dict[str, list[str]] = {}
    for path in sorted(DIALOGUE.rglob("*.yarn")):
        source = executable_source(path.read_text(encoding="utf-8", errors="replace"))
        for name in pattern.findall(source):
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
    assert calls >= 3, (
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
    assert calls >= 1, (
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
