#!/usr/bin/env python3
"""List and shape-check ⌛ STALE marks in the planning docs.

Convention: `docs/planning/STALENESS.md`. Jon, 2026-08-08 — *"mark which items in
planning documents are actually stale … they can record the evidence that they
are stale and then we can make a sweep up later."*

⛔ **This validates SHAPE, never truth.** It cannot know whether a claim is
actually stale — only whether the mark carries a date and some evidence. A
checker that pretended to judge staleness would be exactly the
`a_check_that_cannot_fail` defect: green because it never looked.

So the failure modes it CAN catch are the ones that make the eventual sweep
unreviewable:

* a mark with no date — nobody can tell whether the suspicion predates the fix;
* a mark with no evidence — the sweep must either trust it blindly or redo it;
* a malformed glyph line that the sweep's own grep will silently miss.
"""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
PLANNING = REPO / "docs" / "planning"

#: `⌛ **STALE 2026-08-08** — …` or `⌛ **STALE 2026-08-08 — …**`
MARK = re.compile(r"⌛\s*\*\*STALE\s*(?P<date>\d{4}-\d{2}-\d{2})?(?P<tail>[^*]*)\*\*(?P<rest>.*)")

#: A mark needs a reason a human can act on. This is a floor, not a judgement.
MIN_EVIDENCE_CHARS = 40


class Mark:
    def __init__(self, path: Path, line_no: int, date: str | None, body: str):
        self.path = path
        self.line_no = line_no
        self.date = date
        self.body = body.strip()

    @property
    def rel(self) -> str:
        return str(self.path.relative_to(REPO))

    def problems(self) -> list[str]:
        out = []
        if not self.date:
            out.append("no ISO date — the sweep cannot tell if this predates a fix")
        if len(self.body) < MIN_EVIDENCE_CHARS:
            out.append(
                f"no evidence ({len(self.body)} chars) — say what you SAW: a "
                f"command, a file, a commit, or a person"
            )
        return out


#: The convention document itself. Its marks are SPECIMENS, not claims — and a
#: checker that trips on the file defining its own format is a checker nobody
#: runs twice. Found by running it: the first version reported all three examples.
SPEC_DOC = "STALENESS.md"


def collect(root: Path) -> list[Mark]:
    marks: list[Mark] = []
    for md in sorted(root.rglob("*.md")):
        if md.name == SPEC_DOC:
            continue
        try:
            lines = md.read_text(encoding="utf8").splitlines()
        except OSError:
            continue
        fenced = False
        for i, line in enumerate(lines, 1):
            if line.lstrip().startswith("```"):
                fenced = not fenced
                continue
            # ⛔ a mark QUOTED inside a code fence is an example, not a claim.
            if fenced:
                continue
            m = MARK.search(line)
            if not m:
                continue
            # evidence may continue onto the following quoted/indented lines
            body = (m.group("tail") or "") + (m.group("rest") or "")
            for follow in lines[i : i + 4]:
                stripped = follow.lstrip("> ").rstrip()
                if not stripped or stripped.startswith("#"):
                    break
                body += " " + stripped
            marks.append(Mark(md, i, m.group("date"), body))
    return marks


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--check", action="store_true",
                    help="exit 1 if any mark is malformed (shape only)")
    args = ap.parse_args(argv)

    marks = collect(PLANNING)
    if not marks:
        print("no ⌛ STALE marks in docs/planning — see docs/planning/STALENESS.md")
        return 0

    marks.sort(key=lambda m: (m.date or "", m.rel), reverse=True)
    bad = 0
    for m in marks:
        problems = m.problems()
        flag = "⛔" if problems else "  "
        print(f"{flag} {m.date or '????-??-??'}  {m.rel}:{m.line_no}")
        print(f"      {m.body[:150]}")
        for p in problems:
            bad += 1
            print(f"      ⛔ {p}")

    print(f"\n{len(marks)} mark(s), {bad} problem(s)")
    if args.check and bad:
        print("⛔ a malformed mark makes the eventual sweep unreviewable "
              "— see docs/planning/STALENESS.md")
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
