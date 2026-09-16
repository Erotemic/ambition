#!/usr/bin/env python3
"""Every live control-plane document still has its content.

⛔⛤ **THIS EXISTS BECAUSE I EMPTIED `status.md` TO ZERO BYTES AND PUSHED IT, AND
EVERY GATE STAYED GREEN.** MEASURED 2026-09-16: `run_tests.py --maintenance`
reported 7/7 over a control-plane document that no longer existed as content. It
is not a hole in those gates — a citation checker over a file with no citations
has nothing to report, and a hold checker over a file with no holds has nothing
to report. **A gate over a missing document does not fail; it has nothing to
check.** The absence of findings and the absence of a subject look identical.

⇒ So this asserts the SUBJECT, which no other check does: the named documents
exist, are non-trivial, and still carry the headings that make them the thing
they claim to be.

⛔⛔ **THE FLOORS ARE DELIBERATELY FAR BELOW TODAY'S SIZES.** This is a
catastrophe detector, not a ratchet. A row closing and being compressed to a
receipt is the queue contract WORKING — `queue.md` went 1063 → 908 lines the same
day this was written, and that must not red. What must red is a document losing
most of itself at once.

⚠ **IT CANNOT SEE A DOCUMENT THAT GOES WRONG WHILE STAYING BIG.** Stale claims,
a discharged hold still stated, a citation that no longer resolves — those have
their own checks, and a green run here says nothing about any of them.
"""

from __future__ import annotations

import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]

#: document → (minimum lines, headings that must survive)
#: Sizes on 2026-09-16 are in the comments; the floors sit far under them.
LIVE_CONTROL_PLANE: dict[str, tuple[int, tuple[str, ...]]] = {
    # 1061 lines. The executable queue.
    "docs/planning/queue.md": (300, ("# The queue", "## P0")),
    # 176 lines. The orientation snapshot — the one this check exists for.
    "docs/planning/status.md": (60, ("# Planning status", "## Current execution")),
    # 526 lines. Unresolved maintainer choices only.
    "docs/planning/awaiting-maintainer-decision.md": (150, ("# ",)),
    # 167 lines. Durable rulings.
    "docs/planning/maintainer-decisions.md": (60, ("# ",)),
    # 199 lines. How the planning tree is used.
    "docs/planning/README.md": (60, ("# ",)),
    # 281 lines. The standing reservoir.
    "docs/planning/tracks.md": (80, ("# ",)),
    # ⛔⛤ **THE CONSOLIDATION CONTROL PLANE WAS OUTSIDE THIS POPULATION UNTIL
    # 2026-09-16, and it is the half that gets EDITED.** `status.md` is what this
    # check was built for; these four are where the architecture campaigns' state
    # actually lives, they were rewritten repeatedly in one night, and every
    # argument for guarding `status.md` applies to them unchanged. A citation
    # checker over an emptied census has nothing to report.
    # 180 lines. The census's own reader's guide and the re-derivation table.
    "docs/planning/consolidation/README.md": (60, ("# ",)),
    # 671 lines. The ranked campaigns, their gates and their premises.
    "docs/planning/consolidation/consolidation-plan.md": (200, ("## Priority table",)),
    # 587 lines. The authority/lifetime map the campaigns are cut from.
    "docs/planning/consolidation/architecture-census.md": (200, ("# ",)),
    # 134 lines. The baseline snapshot and its later readings.
    "docs/planning/consolidation/campaign-metrics.md": (40, ("# ",)),
}


def main() -> int:
    findings: list[str] = []
    for rel, (floor, headings) in sorted(LIVE_CONTROL_PLANE.items()):
        path = REPO / rel
        if not path.exists():
            findings.append(f"  {rel}: GONE. The file does not exist.")
            continue
        text = path.read_text(encoding="utf-8")
        lines = text.count("\n")
        if lines < floor:
            findings.append(
                f"  {rel}: {lines} line(s), floor is {floor}. "
                "A live control-plane document does not shrink to this by being edited."
            )
            continue
        missing = [h for h in headings if h not in text]
        if missing:
            findings.append(
                f"  {rel}: {lines} line(s) but missing heading(s) {missing}. "
                "It is no longer the document it claims to be."
            )

    if findings:
        print(
            "⛔⛔ A LIVE CONTROL-PLANE DOCUMENT LOST ITS CONTENT:\n"
            + "\n".join(findings)
            + "\n⇒ Recover it with `git show <last-good-sha>:<path>`, reapply the "
            "intended edit, and DIFF against that blob to prove the restore is "
            "exactly the hunks you meant.\n"
            "⚠ Every other planning gate will stay GREEN over the damage — a "
            "citation checker over a file with no citations has nothing to report."
        )
        return 1

    total = sum(
        (REPO / rel).read_text(encoding="utf-8").count("\n") for rel in LIVE_CONTROL_PLANE
    )
    print(
        f"ok: {len(LIVE_CONTROL_PLANE)} live control-plane document(s) present, "
        f"{total} lines, every required heading intact"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
