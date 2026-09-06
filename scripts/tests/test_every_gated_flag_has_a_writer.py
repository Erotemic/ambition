"""A flag nothing can ever set is a door that never opens.

⛔⛔ THIS IS THE ONE HOLE THE OTHER AUTHORED-CONDITION GUARDS LEAVE. The
repository already checks that a `gated_by` PREPARES against the composed
catalog, that a `condition("id", …)` names a PUBLISHED condition, and that a
planning doc does not cite a fabricated id. All three are about the CONDITION.
None of them looks at the FACT.

`world.flag_set` takes an author-typed flag NAME, and a misspelt one is invisible
to every existing check: it parses, the condition is published, the evaluation
succeeds — and it answers NO for the rest of the game, because nothing will ever
set a flag by that name. ⇒ A wall gated on it never opens; a dialogue branch
behind it is unreachable. Both look like content that was never written rather
than content that is broken.

⭐ WHAT COUNTS AS A WRITER IS DELIBERATELY LOOSE. A flag is "writable" if its
name appears anywhere in a `.yarn` `set_flag` call or as a string literal in any
`.rs` file. That does not prove anything ever sets it at runtime — a flag can be
named by a system that never runs. ⚠ The looseness is in the SAFE direction: it
under-reports rather than over-reports, and the failure it exists for is a
TYPO, where the misspelling appears in exactly one place in the whole repository.

⭐ Measured 2026-09-04 before the rule was written: four authored reads across
three distinct flags, every one of them written.

    bob_field_survey_received   yarn set_flag + quest.rs        (also the one LDtk gate)
    kernel_guide_demo_flag      yarn set_flag, set and cleared
    p1_stabilizer_received      quest.rs

So this lands green as a ratchet rather than as a repair.

⛔⛔ AND THE FIRST POISON OF THIS GUARD PASSED, WHICH WAS A FACT ABOUT THE
POISON. Misspelling `p1_stabilizer_received` "in kernel.yarn" left the test
green, and for a moment that read as a hole in the rule. The flag is not in
`kernel.yarn` — it is in `intro.yarn` — so the edit changed nothing and the
green was correct. ⇒ **A poison that did not land proves nothing, and it looks
exactly like a guard that does not work.** Confirm the breakage is present
before reading the verdict; `grep -c` on the edited file costs one line.
Repeated against the right file, it fails with
`p1_stabilizer_recieved  (read in intro.yarn)`.

Poison-verified three ways: a misspelt flag in a dialogue read, a misspelt flag
in an LDtk `gated_by`, and the floor with the read spelling removed.
"""

from __future__ import annotations

import json
import re
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent.parent
DIALOGUE = REPO / "game/ambition_content/assets/dialogue"
WORLDS = REPO / "game/ambition_content/assets/worlds"

# `condition("world.flag_set", "<flag>")` — the dialogue road.
YARN_READ = re.compile(r'world\.flag_set"\s*,\s*"([A-Za-z0-9_]+)"')
# `<<set_flag "<flag>" true>>` — the dialogue road's writer.
YARN_WRITE = re.compile(r'(?:set_flag|clear_flag)"?\s+"([A-Za-z0-9_]+)"')


def _authored_reads() -> dict[str, list[str]]:
    """Every flag an authored gate or dialogue line asks about, and where."""
    reads: dict[str, list[str]] = {}
    for path in sorted(DIALOGUE.rglob("*.yarn")):
        text = path.read_text(encoding="utf-8", errors="replace")
        for flag in YARN_READ.findall(text):
            reads.setdefault(flag, []).append(path.name)
    for path in sorted(WORLDS.glob("*.ldtk")):
        try:
            world = json.loads(path.read_text(encoding="utf-8", errors="replace"))
        except json.JSONDecodeError as exc:  # a broken world is another test's finding
            raise AssertionError(f"{path.name} is not readable JSON: {exc}") from exc
        for level in world.get("levels", []):
            for layer in level.get("layerInstances") or []:
                for entity in layer.get("entityInstances") or []:
                    for field in entity.get("fieldInstances", []):
                        if field.get("__identifier") != "gated_by":
                            continue
                        value = field.get("__value")
                        # ⚠ A BARE VALUE IS STILL A FLAG. `gated_by` is a
                        # condition LINE now, so `body.fits 32` is also legal
                        # here — only the bare form names a flag.
                        if isinstance(value, str) and value and " " not in value.strip():
                            reads.setdefault(value.strip(), []).append(path.name)
    return reads


def _writable() -> set[str]:
    names: set[str] = set()
    for path in sorted(DIALOGUE.rglob("*.yarn")):
        names.update(YARN_WRITE.findall(path.read_text(encoding="utf-8", errors="replace")))
    for root in ("crates", "game", "tests"):
        for path in (REPO / root).rglob("*.rs"):
            text = path.read_text(encoding="utf-8", errors="replace")
            for literal in re.findall(r'"([a-z][a-z0-9_]{3,})"', text):
                names.add(literal)
    return names


def test_every_flag_an_authored_gate_reads_can_be_written() -> None:
    reads = _authored_reads()
    # ⛔ A FLOOR ABOVE THE LARGEST SINGLE SOURCE, not above zero: `kernel.yarn`
    # alone supplies two of the reads, so a floor of 1 would survive losing every
    # other file.
    assert len(reads) >= 3, (
        f"only {len(reads)} authored flag read(s) found across {DIALOGUE.name} and "
        f"{WORLDS.name} — the spellings this scans have changed, and an empty "
        f"walk cannot fail. Found: {sorted(reads)}"
    )
    writable = _writable()
    unwritable = sorted(flag for flag in reads if flag not in writable)
    assert not unwritable, (
        "authored content gates on flags that nothing in the repository can set, "
        "so those doors never open and those branches are unreachable — and a "
        "misspelt flag name looks exactly like this:\n  "
        + "\n  ".join(f"{flag}  (read in {', '.join(sorted(set(reads[flag])))})" for flag in unwritable)
    )
