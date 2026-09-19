#!/usr/bin/env python3
"""A type deleted to collapse a duplicate authority does not come back as code.

⛔⛤ **THE CONSOLIDATION CENSUS CALLS THREE FAMILIES `RESOLVED`, AND NOTHING
RE-DERIVED THAT AGAINST SOURCE.** `check_consolidation_ledger_states_are_live.py`
holds the ledger and the page in agreement — a real job, and a different one:
it checks that two DOCUMENTS say the same thing. What each row actually claims
is about the TREE, and for the collapses below the claim reduces to a symbol
being gone. A resolved duplicate authority that quietly regrows its second
owner would keep printing `RESOLVED` from both documents.

⚠ **AND THE OBVIOUS SPELLING OF THIS CHECK IS FALSE ON DAY ONE.** Every one of
these names still appears in the tree, four times for `GameplaySessionLinks`
alone — in the COMMENTS that record the deletion and why it mattered. A
crate-wide string count would have failed immediately, which is the same trap
`Q134`'s *"`ambition_dialog` contains the string `rollback` zero times"* fell
into: writing the finding down falsifies the evidence for it. ⇒ This strips
comments and string literals first and asks only about CODE.

⭐ The stripper is [`a_rollback_arm_must_refuse_a_frozen_world.code_only`],
imported rather than rewritten: it is the one in this tree that handles nested
`/* */` and every Rust string form including raw strings with an arbitrary `#`
count, and it got that way by being poisoned through by a review.

⚠ **WHAT THIS CANNOT DO, STATED SO A GREEN IS NOT READ AS MORE:** a second
owner that returns under a DIFFERENT NAME is invisible here. This is a ratchet
against the specific regression — the deleted abstraction being reinstated —
not a proof that the family stayed collapsed. The semantic half is the census's
review, and it is dated.
"""

from __future__ import annotations

import re
import subprocess
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO / "scripts"))

import a_rollback_arm_must_refuse_a_frozen_world as frozen  # noqa: E402

#: symbol → (census family, what its return would re-open).
#:
#: ⛔ A ROW IS A READING OF A COLLAPSE, NOT A BAN LIST. Each says which
#: duplicate authority the deletion closed, so a maintainer who wants the
#: abstraction back knows which row they are re-opening rather than which
#: check they are fighting.
COLLAPSED: dict[str, tuple[str, str]] = {
    "GameplaySessionLinks": (
        "DUP-SESSION-CURRENT",
        "a one-entry copy of a pair the live session instance already carried, "
        "with no production reader for its query. Deleting it is what turned "
        "the rest of that family from a claim into a layer split",
    ),
    "CandidateState": (
        "DUP-CONTENT-CANDIDATE",
        "*\"the one abstraction that would have been a second road\"*, deleted "
        "unused. A10's scene candidate is one hidden candidate and one "
        "admission; a parallel candidate state machine is the thing that "
        "separation is defined against",
    ),
    "SpawnPlayerCloneRequest": (
        "ROAD-DYNAMIC-SPAWN",
        "the developer clone's request seam. The feature was removed 2026-09-18 "
        "after it shipped on a key two presets bound to `taunt`; the request "
        "type coming back means the road did",
    ),
    "AdmittedCheckpointRestore": (
        "ORDER-CHECKPOINT",
        "a one-frame token *\"that every reducer had to remember to read\"*. "
        "A1c/3b replaced it with a schedule only the commit executor runs, so "
        "the ordering is structural instead of a fact each reader re-checks",
    ),
}

#: ⛔ The scan has to look at the whole tree or it cannot answer. A shrinking
#: corpus is how this check would go quiet without failing. MEASURED 2026-09-18:
#: 1,917 tracked `.rs` files. ⚠ The first draft of this line said 2000 because I
#: guessed a round number, and the floor reddened on its own first run — which
#: is the floor working, on its author.
FLOOR = 1700


def _tracked_rust() -> list[str]:
    return subprocess.run(
        ["git", "-C", str(REPO), "ls-files", "*.rs"],
        capture_output=True,
        text=True,
        check=True,
    ).stdout.split()


def code_occurrences(repo: Path = REPO) -> dict[str, list[tuple[str, int]]]:
    """`{symbol: [(file, line), ..]}` for each name, comments and strings blanked."""
    patterns = {name: re.compile(rf"\b{re.escape(name)}\b") for name in COLLAPSED}
    found: dict[str, list[tuple[str, int]]] = {name: [] for name in COLLAPSED}
    for rel in _tracked_rust():
        if "/target/" in rel or rel.startswith(".worktrees"):
            continue
        try:
            text = (repo / rel).read_text(errors="replace")
        except OSError:
            continue
        # Cheap reject before the scanner, which is the expensive part.
        if not any(name in text for name in COLLAPSED):
            continue
        code = frozen.code_only(text)
        for name, pattern in patterns.items():
            for match in pattern.finditer(code):
                found[name].append((rel, code.count("\n", 0, match.start()) + 1))
    return found


def scanned_files(repo: Path = REPO) -> int:
    return len([r for r in _tracked_rust() if "/target/" not in r])


def main() -> int:
    size = scanned_files()
    if size < FLOOR:
        print(
            f"⛔ scanned {size} Rust file(s), below the recorded floor of {FLOOR}. "
            "This is looking at less than it was built against, so a clean result "
            "here means nothing."
        )
        return 1

    found = code_occurrences()
    back = {name: hits for name, hits in found.items() if hits}
    print(
        f"{len(COLLAPSED)} collapsed authorit(ies) checked across {size} Rust "
        f"file(s), comments and string literals blanked"
    )
    for name, (family, why) in sorted(COLLAPSED.items()):
        mark = "⛔ BACK" if found[name] else "✔ gone"
        print(f"  {mark}  {name:<28} {family}")
    if back:
        for name, hits in sorted(back.items()):
            family, why = COLLAPSED[name]
            print(f"\n⛔ `{name}` is CODE again, in {len(hits)} place(s):")
            for rel, line in hits[:10]:
                print(f"     {rel}:{line}")
            print(f"   It was deleted to collapse `{family}` — {why}.")
        print(
            "\n⇒ Either the census row is no longer RESOLVED and should say so, or "
            "this name is being reused for something unrelated and the row below it "
            "should say THAT. Silence is the one wrong answer."
        )
        return 1
    print("ok: every collapse is still a collapse in code.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
