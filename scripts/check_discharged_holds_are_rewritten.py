#!/usr/bin/env python3
"""Find a planning row that announces a hold is discharged and still states it.

⛔⛔ THE DEFECT, THREE TIMES ON 2026-09-11 BY TWO DIFFERENT AGENTS, AND IT COST A
WHOLE PACKET BRIEF. A row carries a `**HOLD:**` sentence naming what must happen
before work starts. When the hold is discharged the habit is to add a banner at
the TOP of the row — `THE MAP THIS HOLD ASKS FOR IS DELIVERED`, `✅ … IS CLOSED`
— and leave the `**HOLD:**` line twenty lines below still stating the condition
as live:

    ## A7. Separate item custody/accounting from lifecycle orchestration
    **THE ENUMERATION THIS HOLD ASKS FOR IS DELIVERED:** …      <- line 410
    …
    **HOLD:** after A1, enumerate item occurrence, holder, …    <- line 432

A reader who scrolls to the hold — the sentence that DECIDES WHETHER TO START —
is told to wait for something the row's own first paragraph says arrived. On
2026-09-11 that sent an agent at an A7 job that had landed the previous day, and
the row it read had said `DELIVERED` twenty-two lines above the hold the whole
time. A4 was in the same state; a third banner was then added ABOVE A4's still-live
hold by the agent who had just been told about A7.

⇒ THE RULE THE CORPUS NOW STATES: a discharged hold is rewritten IN PLACE, naming
what discharged it and when. A banner is a SECOND COPY of that fact and the second
copy is the one that rots.

⭐ WHAT SEPARATES A LIVE HOLD FROM A QUOTED ONE, MEASURED RATHER THAN HOPED. A row
recording its own history quotes the old text in prose — `*"HOLD on extraction:
first map writers…"*` — so this matches only the BOLD MARKER AT LINE START, which
is the convention's form. Verified against today's corrected A4, which carries two
banners AND a quoted `HOLD on extraction` and is correctly NOT flagged.

⚠ WHAT IT CANNOT DECIDE, AND WHY THE ESCAPE EXISTS. A row may legitimately deliver
ONE half of a two-part hold and still be held on the other — A5's hold was
`"until A2's contact contract is established AND writer inventory is complete"`.
This check cannot tell that from a stale row, so a row in that state puts
`hold-ok` on the hold line and says which half remains.
"""

from __future__ import annotations

import argparse
import pathlib
import re
import sys

BANNER = re.compile(r"DELIVERED|SATISFIED|DISCHARGED|✅|✔")
# The convention's form: a BOLD marker at line start. Prose quoting an old hold
# writes `*"HOLD …"*` and is deliberately not matched.
#
# ⛔⛤ **`DO NOT START BEFORE` IS THE SAME CONVENTION UNDER A SECOND SPELLING, AND
# IT WAS OUTSIDE THE POPULATION.** `consolidation-plan.md` states every ranked
# row's gate that way -- ten of them -- and this check was scanning FOUR bold hold
# lines across 1072 rows. MEASURED 2026-09-16: three of those ten named blockers
# that had been discharged on 2026-09-15, and the check reported clean because
# their spelling was not the one it knew. A convention spelled twice is invisible
# to the guard that enforces it.
LIVE = re.compile(r"^\*\*(HOLD|BLOCKED|DO NOT START BEFORE)\b")
DISCHARGED = re.compile(r"DISCHARGED|SATISFIED|LIFTED|CLOSED|hold-ok|discharged")

# ⛔⛤ **THE SECOND RULE: A HOLD MAY NAME A ROW THIS SAME FILE MARKS COMPLETE.**
# Widening `LIVE` to reach `consolidation-plan.md` took the population from 4 to
# 12 and caught NOTHING -- MEASURED by restoring a real stale gate line, which
# stayed green. The banner rule needs the discharge announced INSIDE the row, and
# that file announces it in a priority TABLE at the top. So the gate line said
# "DO NOT START BEFORE: A10 complete" while the table two screens up already said
# COMPLETE, and no rule connected them.
#
# ⚠ A wider population is not a wider REACH. Poison the widening against the
# defect it was supposed to find before believing it.
ROW_ID = re.compile(r"\b([A-Z]\d{1,3})\b")
COMPLETE_HERE = re.compile(r"\b(COMPLETE|COMPLETED|CLOSED)\b")

# ⛔⛤ **THE THIRD RULE, AND IT IS A THIRD SPELLING OF THE SAME CONVENTION.**
# MEASURED 2026-09-16: `consolidation/architecture-census.md` carried the
# sentence *"The current A10 implementation should finish before another agent
# changes the room publication model."* A10 closed on 2026-09-15 and its
# demolition on 2026-09-16, so that line held a door shut with nobody behind it
# — and it was invisible to BOTH rules above, because it is neither a bold
# `**HOLD:**` marker nor inside a row that announces its own discharge. It is
# ordinary prose in a document with no status table at all.
#
# ⇒ So this rule crosses FILES: `queue.md` row headers are the authority on which
# campaigns are done, and a GATE SENTENCE anywhere in the corpus that names one
# of them is stale. ⚠ It needs a gate VERB near the id, because the corpus is
# full of legitimate history ("A10's room scope landed first") that names a
# closed row without gating anything on it.
QUEUE_ROW = re.compile(r"^### ([A-Za-z0-9/-]+)")
QUEUE_DONE = re.compile(r"DONE|CLOSED|✅")
#: A gate verb within ~40 characters of the row id, in either order.
GATE_VERBS = (
    r"should (?:finish|land|close|complete)|must (?:finish|land|wait)|"
    r"blocked on|waits? (?:on|for)|gated on|do not (?:start|begin)|"
    r"not until|depends on|pending"
)
GATE_NEAR_ID = re.compile(
    rf"(?:(?:{GATE_VERBS}).{{0,40}}?\b([A-Z][A-Z0-9-]*\d[A-Z0-9-]*)\b"
    rf"|\b([A-Z][A-Z0-9-]*\d[A-Z0-9-]*)\b.{{0,40}}?(?:{GATE_VERBS}))"
)


def rows_marked_done(queue: pathlib.Path) -> set[str]:
    """Which campaigns `queue.md` announces as finished, from its row headers."""
    out = set()
    for line in queue.read_text(encoding="utf-8").split("\n"):
        m = QUEUE_ROW.match(line)
        if m and QUEUE_DONE.search(line):
            out.add(m.group(1))
    return out


def stale_gates(text: str, done: set[str]) -> list[tuple[int, str, str]]:
    """(lineno, id, line) for gate sentences naming a finished campaign."""
    findings = []
    for n, line in enumerate(text.split("\n"), start=1):
        if DISCHARGED.search(line):
            continue
        # ⚠ A struck-through id is the REWRITE this check asks for.
        bare = re.sub(r"~~[^~]*~~", "", line)
        for m in GATE_NEAR_ID.finditer(bare):
            name = m.group(1) or m.group(2)
            if name in done:
                findings.append((n, name, line))
    return findings

# ⛔⛔ THE KNOWN-ANSWER CONTROL, RUN ON EVERY INVOCATION. A source-text guard goes
# blind when its pattern rots, and the symptom is a clean report — which is what a
# healthy corpus also produces. This is the exact A7 shape plus the exact quoted
# form that must NOT match; if either verdict flips, the check refuses to report a
# number rather than reporting a comforting one.
CONTROL_FLAGGED = [
    "## A7. Separate item custody",
    "**THE ENUMERATION THIS HOLD ASKS FOR IS DELIVERED:** page.md",
    "",
    "**HOLD:** after A1, enumerate item occurrence, holder and inventory writers.",
]
CONTROL_CLEAN = [
    "## A4. Co-locate accepted control",
    "**THE MAP THIS HOLD ASKS FOR IS DELIVERED:** page.md",
    "",
    '**HOLD DISCHARGED 2026-09-10** by the map above. ⚠ This line read *"HOLD on',
    'extraction: first map writers and select production fixtures"* until 2026-09-11.',
]


def offenders(body: list[str], base: int) -> tuple[list[str], list[tuple[int, str]]]:
    banners = [l for l in body if BANNER.search(l)]
    live = [
        (base + j + 1, l)
        for j, l in enumerate(body)
        if LIVE.match(l) and not DISCHARGED.search(l)
    ]
    return banners, live


def self_check() -> None:
    b, live = offenders(CONTROL_FLAGGED, 0)
    if not (b and live):
        raise SystemExit(
            "⛔⛔ THE CONTROL DID NOT FLAG THE KNOWN A7 SHAPE, so this check is blind "
            "and its clean report would mean nothing. Fix the pattern, not the control."
        )
    b, live = offenders(CONTROL_CLEAN, 0)
    if live:
        raise SystemExit(
            "⛔⛔ THE CONTROL FLAGGED A ROW WHOSE HOLD IS DISCHARGED AND ONLY QUOTED. "
            f"This check would fail correct rows: {live}"
        )


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("roots", nargs="*", default=["docs/planning"])
    parser.add_argument(
        "--report-only",
        action="store_true",
        help="print findings and exit 0 (default GATES, because the corpus is clean today)",
    )
    args = parser.parse_args()
    self_check()

    rows = holds = 0
    findings: list[str] = []
    for root in args.roots:
        for path in sorted(pathlib.Path(root).rglob("*.md")):
            lines = path.read_text(encoding="utf-8").split("\n")
            starts = [i for i, l in enumerate(lines) if l.startswith("## ")]
            for a, b in zip(starts, starts[1:] + [len(lines)]):
                rows += 1
                body = lines[a:b]
                holds += sum(1 for l in body if LIVE.match(l))
                # RULE 2: a hold naming an id this same file marks complete.
                done = {
                    m.group(1)
                    for l in lines
                    if COMPLETE_HERE.search(l)
                    for m in ROW_ID.finditer(l)
                }
                for n, l in enumerate(body, start=a + 1):
                    if not LIVE.match(l) or DISCHARGED.search(l):
                        continue
                    # ⚠ A struck-through id is the REWRITE this check asks for.
                    bare = re.sub(r"~~[^~]*~~", "", l)
                    stale = sorted(set(ROW_ID.findall(bare)) & done)
                    if stale:
                        findings.append(
                            f"  {path}:{n}\n"
                            f"     HOLD names {', '.join(stale)}, which this file "
                            f"marks COMPLETE\n"
                            f"     :{n} {l.strip()[:78]}"
                        )
                banners, live = offenders(body, a)
                if banners and live:
                    findings.append(
                        f"  {path}:{a + 1}  {lines[a][:70]}\n"
                        f"     banner : {banners[0].strip()[:78]}\n"
                        + "".join(
                            f"     LIVE   : :{n} {l.strip()[:78]}\n" for n, l in live
                        )
                    )

    # ⛔ ANTI-VACUITY. A corpus with no rows, or with no bold holds at all, makes
    # every verdict below trivially clean — and a scan root that a file move
    # silently emptied looks exactly like a repository with no defects.
    if rows < 100 or holds < 1:
        print(
            f"⛔⛔ THE POPULATION IS NOT THERE: {rows} row(s), {holds} bold hold "
            f"line(s) under {args.roots}. A clean verdict over this corpus would be "
            "a claim about the scan root, not about the rows."
        )
        return 1

    # RULE 3: a gate sentence anywhere in the corpus naming a FINISHED campaign.
    queue = pathlib.Path("docs/planning/queue.md")
    done_rows = rows_marked_done(queue) if queue.exists() else set()
    if done_rows:
        for root in args.roots:
            for path in sorted(pathlib.Path(root).rglob("*.md")):
                if path == queue:
                    continue  # the authority describes its own rows
                for n, name, line in stale_gates(
                    path.read_text(encoding="utf-8"), done_rows
                ):
                    findings.append(
                        f"  {path}:{n}\n"
                        f"     GATES work on {name}, which `queue.md` marks finished\n"
                        f"     :{n} {line.strip()[:78]}"
                    )

    if findings:
        print(
            f"⛔ {len(findings)} hold(s) state a condition this repository has "
            f"already discharged -- either announced in the row itself, or named as "
            f"a row this same file marks COMPLETE:\n"
        )
        print("\n".join(findings))
        print(
            "⇒ REWRITE THE GATING LINE ITSELF, naming what discharged it and when.\n"
            "  A banner above it, or a status table two screens up, is a SECOND COPY\n"
            "  of that fact and the second copy is the one that rots — the reader who\n"
            "  scrolls to the gate is the reader deciding whether to start.\n"
            "⚠ If the row genuinely delivers ONE half of a two-part hold and is still\n"
            "  held on the other, put `hold-ok` on the hold line and say which half.\n"
            "⚠ For a RULE 3 finding (a prose gate naming a campaign `queue.md` marks\n"
            "  finished): if the sentence is HISTORY rather than a gate, rewrite it in\n"
            "  the past tense. This check needs a gate VERB beside the id, so an\n"
            "  ordinary mention of a closed campaign is already invisible to it."
        )
        return 0 if args.report_only else 1

    print(
        f"No row announces a discharged hold while stating it "
        f"({rows} rows, {holds} bold hold line(s) scanned)."
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
