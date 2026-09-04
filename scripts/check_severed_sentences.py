#!/usr/bin/env python3
"""Find planning prose whose sentence was cut in half by a later insertion.

⛔⛔ THE DEFECT, FOUND THREE TIMES IN `docs/planning/queue.md` ON 2026-09-04 AND
INVISIBLE TO EVERY OTHER GATE. An edit inserts a new block *between* a line that
ends mid-clause and the line that finishes it. The reader meets a clause with no
ending; its continuation is stranded tens of lines below, after unrelated text,
where it reads as a new paragraph:

    ... and `6 vs 5` / `9 vs 6` are within        <- head, no ending
    ⛔⛔ **AND THE SIGNIFICANCE HALF ...**         <- 45 lines inserted here
    ...
    spread.** ⚠ So it is **one bad rung ...**     <- orphaned continuation

Nothing catches it. Both halves are valid Markdown, the link and citation gates
pass, `git diff` shows a clean insertion, and neither fragment is malformed on
its own. ⭐ It is the prose form of the stolen `#[test]` recorded in
`dev/benchmark-candidates/` — **an insertion between an opening and its
continuation captures the space between them** — and the defence is the same:
anchor an insertion on a whole paragraph, never on a line that ends mid-clause.

TWO DETECTORS, and they are deliberately not the same strength.

`stranded continuation` GATES. A prose line starting lowercase whose previous
line ends a sentence is a wrap that cannot be one: in this corpus's wrapped
style, a genuine continuation follows a line that does NOT terminate.

`severed head` REPORTS ONLY. A line ending mid-clause followed by a new marker
block is the other side of the same cut, but the corpus writes marker glyphs
mid-sentence on purpose (*"... `cutscene` 16, and ⛔ NONE of those crates"*), so
failing on it would train everyone to pass `--no-verify`. Same reasoning, and
the same precedent, as `check_planning_citations.py` being non-strict by default.
"""

from __future__ import annotations

import argparse
import pathlib
import re
import sys

MARKERS = "⛔⭐⚠✔▢ⓘ⏳◐⊙"
# A block opener: a bare marker glyph. NOT a list bullet -- `* ✔ ...` is how
# JONS_OBSERVATIONS_BUGS_AND_ISSUES.md formats every agent reply under a
# maintainer bullet that legitimately ends without punctuation.
BLOCK_OPENER = re.compile(r"^\s*[" + MARKERS + r"]")
STARTS_LOWER = re.compile(r"^\s+[a-z]")
# Three or more numeric tokens means a data row pasted into prose, not a clause.
NUMERIC_ROW = re.compile(r"(?:(?<=\s)|^)\d[\d.%:/-]*(?=\s|$)")

SENTENCE_END = (".", "!", "?")
ABBREVIATION = ("etc.", "e.g.", "i.e.", "vs.", "cf.", "approx.")
# Endings that are mid-line by design and say so: punctuation the author chose.
CONTINUES = (",", ":", ";", "—", "-", "/", "**", "*", "`", '"', "]", ")", "%", "|",
             "-->", "</details>", "~~")


def terminates(line: str) -> bool:
    t = line.rstrip()
    return t.endswith(SENTENCE_END) and not t.endswith(ABBREVIATION)


def prose(line: str) -> bool:
    t = line.lstrip()
    return bool(t) and not t.startswith(("|", "#", ">", "-", "*", "<", "```", "!["))


def scan(path: pathlib.Path) -> tuple[list, list]:
    lines = path.read_text(encoding="utf-8").split("\n")
    stranded, severed = [], []
    fence = False
    for i, line in enumerate(lines):
        if line.lstrip().startswith("```"):
            fence = not fence
            continue
        if fence or i == 0:
            continue
        prev = lines[i - 1]
        if not line.strip() or not prev.strip():
            continue

        # A continuation that begins a line while the line above ended a sentence.
        if (prose(line) and STARTS_LOWER.match(line) and terminates(prev)
                and len(NUMERIC_ROW.findall(line)) < 3):
            stranded.append((i + 1, prev.strip()[-50:], line.strip()[:60]))

        # The other half: a clause with no ending, then a new block.
        if (prose(prev) and not terminates(prev)
                and not prev.rstrip().endswith(CONTINUES)
                and BLOCK_OPENER.match(line)):
            severed.append((i, prev.strip()[-50:], line.strip()[:60]))
    return stranded, severed


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("roots", nargs="*", default=["docs/planning"])
    ap.add_argument("--strict", action="store_true",
                    help="fail on the reporting detector too")
    args = ap.parse_args()

    all_stranded, all_severed, files = [], [], 0
    for root in args.roots or ["docs/planning"]:
        for path in sorted(pathlib.Path(root).rglob("*.md")):
            files += 1
            stranded, severed = scan(path)
            all_stranded += [(path, *h) for h in stranded]
            all_severed += [(path, *h) for h in severed]

    for path, line, prev, cur in all_stranded:
        print(f"⛔ {path}:{line} — a continuation stranded from its sentence")
        print(f"     the line above ENDS a sentence: ...{prev}")
        print(f"     and this one continues one:     {cur}")
    if all_severed:
        print(f"\nⓘ {len(all_severed)} clause(s) ending with no terminator before a "
              f"new block (reported, does not gate — the corpus uses marker glyphs "
              f"mid-sentence on purpose):")
        for path, line, prev, cur in all_severed:
            print(f"   {path}:{line}  ...{prev}\n       >>> {cur}")

    if all_stranded or (args.strict and all_severed):
        print(f"\n⇒ Rejoin the halves. Recover the original with "
              f"`git log -S '<the orphan text>' -- <file>` rather than inventing "
              f"an ending — the continuation is usually still in the file, "
              f"displaced, not deleted.")
        return 1
    print(f"No severed sentences ({files} planning documents scanned).")
    return 0


if __name__ == "__main__":
    sys.exit(main())
