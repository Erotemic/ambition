#!/usr/bin/env python3
"""Can a checker verify that a `file:line` citation's LINE says what the prose claims?

⛔⛔ MEASURED ANSWER: NO, not by pairing a citation with the code tokens beside it.
This script exists to record that negative so the next person does not build the
checker. `docs/planning` asks for exactly this -- *"`file:line` is only as good as
the line, and nothing in this repo checks the line"* -- and the obvious
implementation does not work.

THE POPULATION IS FINE. Of 490 `file:line` citations across 103 planning docs,
210 (43%) sit on a line that also names a backticked code token, so there is
plenty to check.

THE PAIRING IS WHAT FAILS. Searching for the token within a window of the cited
line:

    window ± 0 lines:  83 confirmed, 111 not-found   miss 57%
    window ± 3 lines: 104 confirmed,  90 not-found   miss 46%
    window ±10 lines: 123 confirmed,  71 not-found   miss 37%
    window ±30 lines: 150 confirmed,  44 not-found   miss 23%

⇒ A 23% miss rate at ±30 lines is not a gate. And SAMPLING THE MISSES says they
are not rot -- the heuristic is wrong:

  * `combat-model.md` cites `lib.rs:29` (`pub mod capture;`) on a line that also
    says `CapturedBy`. The citation is about the MODULE; the token is a different
    thing named in the same sentence. Correct citation.
  * `awaiting-maintainer-decision.md` cites `strike.rs:94` (`pub fn windbox`) on a
    line that also says `is_windbox` -- a DIFFERENT function the sentence
    contrasts it with. Correct citation.
  * Table rows pair a citation in one column with tokens from other columns.

⭐ THE GENERAL SHAPE, and it is why no window size rescues it: **a prose line names
SEVERAL things and cites ONE.** Any checker that pairs "the citation on this line"
with "the identifiers on this line" is guessing which, and it guesses wrong often
enough to be noise. A real checker would need the citation to say what it is FOR --
which is a change to how citations are WRITTEN, not a check over how they are
written today.

⇒ WHAT DOES WORK, and is already routine: `check_planning_citations.py` (does the
path resolve, is it ambiguous), plus the human rule this measurement came from --
AFTER EDITING A FILE YOU HAVE CITED, RE-CHECK YOUR OWN CITATIONS INTO IT. Three
citations rotted that way in one session on 2026-09-06, all by the citer's own
later edit to the same file.

Usage:  python3 scripts/citation_line_content_feasibility.py [--windows 0,3,10,30]
        python3 scripts/citation_line_content_feasibility.py --sample 5
"""

from __future__ import annotations

import argparse
import pathlib
import random
import re
import subprocess
import sys

CITE = re.compile(r"`([\w./-]+\.rs):(\d+)`")
TOKEN = re.compile(r"`([A-Za-z_][A-Za-z0-9_]*(?:::[A-Za-z0-9_]+)*)(?:\(\))?`")
DOCS = pathlib.Path("docs/planning")


def tracked_rs() -> list[str]:
    out = subprocess.run(
        ["git", "ls-files", "*.rs"], capture_output=True, text=True, check=True
    ).stdout
    return out.split()


def resolver(tracked: list[str]):
    """Resolve by SUFFIX, the way `check_planning_citations.py` does.

    ⚠ NOT exact-path. An exact-path audit reports the repo's pre-existing
    partial-path citations (`semantic.rs:796`) as missing files, which is a claim
    about the audit rather than about the tree.
    """
    exact = set(tracked)

    def resolve(path: str) -> str | None:
        if path in exact:
            return path
        hits = [p for p in tracked if p.endswith("/" + path)]
        return hits[0] if len(hits) == 1 else None

    return resolve


def pairs(resolve):
    """Every (doc line, citation, tokens-on-that-line) triple worth checking."""
    for doc in sorted(DOCS.rglob("*.md")):
        for line in doc.read_text(encoding="utf-8", errors="replace").split("\n"):
            cites = CITE.findall(line)
            if not cites:
                continue
            toks = {t.split("::")[-1] for t in TOKEN.findall(line) if not t.endswith("rs")}
            if not toks:
                continue
            for path, ln in cites:
                resolved = resolve(path)
                if resolved is None:
                    continue
                yield doc, line, resolved, int(ln), toks


def near(resolved: str, line_no: int, toks: set[str], window: int) -> bool:
    src = pathlib.Path(resolved).read_text(encoding="utf-8", errors="replace").split("\n")
    lo = max(0, line_no - 1 - window)
    hi = min(len(src), line_no + window)
    blob = "\n".join(src[lo:hi])
    return any(t in blob for t in toks)


def main(argv: list[str]) -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--windows", default="0,3,10,30")
    ap.add_argument("--sample", type=int, default=0, help="print N misses at the widest window")
    ap.add_argument("--seed", type=int, default=7)
    args = ap.parse_args(argv)

    resolve = resolver(tracked_rs())
    rows = list(pairs(resolve))
    if not rows:
        # ⛔ An empty corpus would print a clean table and mean nothing.
        print("no checkable citations found — the instrument is broken, not the tree")
        return 2

    windows = [int(w) for w in args.windows.split(",")]
    for w in windows:
        hit = sum(1 for _, _, r, n, t in rows if near(r, n, t, w))
        miss = len(rows) - hit
        print(f"window ±{w:>2} lines: {hit:>3} confirmed, {miss:>3} not-found  -> miss {miss/len(rows):.0%}")

    if args.sample:
        widest = max(windows)
        misses = [row for row in rows if not near(row[2], row[3], row[4], widest)]
        random.seed(args.seed)
        for doc, line, resolved, n, toks in random.sample(misses, min(args.sample, len(misses))):
            src = pathlib.Path(resolved).read_text(errors="replace").split("\n")
            at = src[n - 1].strip() if n <= len(src) else "<past EOF>"
            print(f"\nDOC    {doc.name}\n  claim  {line.strip()[:100]}")
            print(f"  cites  {resolved}:{n}\n  tokens {sorted(toks)[:3]}\n  line   {at[:78]}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
