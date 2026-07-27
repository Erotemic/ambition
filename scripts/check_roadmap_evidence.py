#!/usr/bin/env python3
"""Check that every roadmap task claiming MET cites evidence that still exists.

A ``**Status …**`` line is prose, and prose does not rot loudly. Three lines in
``competitive-2d-platformer-engine-roadmap.md`` were stale on 2026-07-27 and all
three were found by reading the SOURCE rather than the document:

* Task 4 said "the CC3 GATE is diagnostic, not enforcing" for a day after the
  gate started enforcing.
* Task 5 described a plan/validate/commit boundary as absent when it exists —
  the expensive direction, because it would have sent somebody to build a
  staging world for a problem that path does not have.
* Task 11 said MET while the feature only worked when the session happened to
  open in the right room.

The fix is not more diligence. It is making the claim CHECKABLE: a status that
names a test can be verified mechanically; a status that describes a situation
cannot. So this asks one question of every task that claims MET — *does it name
anything a machine can look for, and does that thing still exist?*

What it deliberately does NOT do is judge whether the test proves the claim.
That is a human question and pretending otherwise would be the same
overclaiming this exists to catch. It only reports rows whose evidence has
VANISHED, plus rows that cite none at all.

Usage:
    python3 scripts/check_roadmap_evidence.py             # report
    python3 scripts/check_roadmap_evidence.py --check     # exit 1 on a problem
    python3 scripts/check_roadmap_evidence.py OTHER.md    # any planning doc
"""

from __future__ import annotations

import argparse
import re
import subprocess
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
ROADMAP = (
    REPO_ROOT / "docs" / "planning" / "engine" / "competitive-2d-platformer-engine-roadmap.md"
)

# A task heading and the status line that follows it.
TASK_RE = re.compile(r"^### Task (\d+) — (.+)$", re.M)
STATUS_RE = re.compile(r"^\*\*Status[^*]*\*\*", re.M)

# Evidence a machine can look for: a Rust test/function name in backticks, or a
# path. Deliberately loose — the question is "did the author point at anything",
# not "did they point at it in an approved format".
IDENT_RE = re.compile(r"`([a-z_][a-z0-9_]{6,})`")
PATH_RE = re.compile(r"`([\w./-]+\.(?:rs|md|py|ron))`")


def task_sections(text: str) -> list[tuple[str, str, str]]:
    """(task number, title, body) for every task, body running to the next task."""
    out = []
    matches = list(TASK_RE.finditer(text))
    for i, m in enumerate(matches):
        end = matches[i + 1].start() if i + 1 < len(matches) else len(text)
        out.append((m.group(1), m.group(2), text[m.start() : end]))
    return out


def identifier_exists(name: str) -> bool:
    """Does this identifier appear anywhere in the tracked source?"""
    result = subprocess.run(
        ["git", "grep", "-l", "--", name],
        cwd=REPO_ROOT,
        capture_output=True,
        text=True,
    )
    return result.returncode == 0 and bool(result.stdout.strip())


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "document",
        nargs="?",
        default=str(ROADMAP),
        help="the planning document to check (defaults to the engine roadmap)",
    )
    parser.add_argument(
        "--check",
        action="store_true",
        help="exit non-zero when a MET row cites no evidence or cites something gone",
    )
    args = parser.parse_args()

    text = Path(args.document).read_text(encoding="utf8")
    problems: list[str] = []
    checked = 0

    for number, title, body in task_sections(text):
        status = STATUS_RE.search(body)
        if not status:
            problems.append(f"Task {number} ({title}): no **Status** line at all")
            continue
        status_line = status.group(0)
        if "MET" not in status_line:
            # A row that says it is partial is not overclaiming; nothing to check.
            continue
        checked += 1

        # Evidence anywhere in the task body, not just the status line: the
        # convention here is a status sentence followed by paragraphs that cite.
        cited = set(IDENT_RE.findall(body)) | set(PATH_RE.findall(body))
        if not cited:
            problems.append(
                f"Task {number} ({title}): claims MET and cites NOTHING checkable. "
                "Name the test or the module that makes it true."
            )
            continue

        missing = sorted(name for name in cited if not identifier_exists(name))
        # One missing citation among many is usually prose (a word in backticks
        # that was never an identifier). ALL of them missing means the row is
        # pointing at a world that no longer exists.
        if missing and len(missing) == len(cited):
            problems.append(
                f"Task {number} ({title}): claims MET and every citation is gone "
                f"from the source: {', '.join(missing)}"
            )

    print(f"checked {checked} task row(s) claiming MET")
    for problem in problems:
        print(f"  ✗ {problem}")
    if not problems:
        print("  every MET row cites something that still exists")

    return 1 if (problems and args.check) else 0


if __name__ == "__main__":
    sys.exit(main())
