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
LIVE = re.compile(r"^\*\*(HOLD|BLOCKED)\b")
DISCHARGED = re.compile(r"DISCHARGED|SATISFIED|LIFTED|CLOSED|hold-ok")

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

    if findings:
        print(
            f"⛔ {len(findings)} row(s) announce a hold is discharged and still state "
            f"it as live:\n"
        )
        print("\n".join(findings))
        print(
            "⇒ REWRITE THE `**HOLD:**` LINE ITSELF, naming what discharged it and\n"
            "  when. The banner above it is a second copy of that fact and the second\n"
            "  copy is the one that rots — a reader scrolling to the hold is the\n"
            "  reader deciding whether to start.\n"
            "⚠ If the row genuinely delivers ONE half of a two-part hold and is still\n"
            "  held on the other, put `hold-ok` on the hold line and say which half."
        )
        return 0 if args.report_only else 1

    print(
        f"No row announces a discharged hold while stating it "
        f"({rows} rows, {holds} bold hold line(s) scanned)."
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
