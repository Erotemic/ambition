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

The search deliberately looks at SOURCE ONLY. The first version of this script
searched the whole repository, and every citation it looked for was one it had
just read out of the tracked roadmap — so `git grep` found the roadmap, and the
check reduced to "does this document still contain its own text?" It would have
passed a row whose implementation had been deleted entirely. That is the exact
failure this file exists to catch, committed inside the catcher.

What it deliberately does NOT do is judge whether the test proves the claim.
That is a human question and pretending otherwise would be the same
overclaiming this exists to catch. It only reports rows whose evidence has
VANISHED, plus rows that cite none at all.

Two document shapes are read, because the planning tree has two and forcing one
into the other would damage a document to fit a tool:

* the engine roadmap's `### Task N` sections with a `**Status … MET**` line, and
* `status.md`'s workstream TABLE, whose rows carry a bolded verdict
  (`**DONE**`, `**FIXED**`, `**CLOSED**`, `**LANDED**`) in their state cell.

A document with NEITHER is reported as a problem rather than as zero rows
checked. "Checked 0 rows, no problems" is the most misleading output a guard can
produce, and it is what pointing the task parser at `status.md` used to say.

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
# snake_case (a function or test) OR CamelCase (a type). Both are things a
# machine can look for, and restricting to the first meant two `status.md` rows
# citing only TYPE names read as "cites nothing checkable" — the guard demanding
# a particular vocabulary rather than any evidence at all.
IDENT_RE = re.compile(r"`([a-z_][a-z0-9_]{6,}|[A-Z][A-Za-z0-9]{6,})`")
PATH_RE = re.compile(r"`([\w./-]+\.(?:rs|md|py|ron))`")


# The SECOND shape (2026-07-27). `docs/planning/status.md` has no task headings
# and no `**Status**` lines at all: it is a table of WORKSTREAMS, each row a
# name, a bolded verdict, and what would close it. Pointing the task parser at it
# reports "checked 0 rows" and looks clean, which is the most misleading output a
# guard can produce.
#
# Giving that document the roadmap's row shape was the other option, and it is
# the wrong one: a table of workstreams and a list of tasks are different
# documents on purpose, and damaging one to fit a tool is how a tool starts
# deciding what may be written down.
TABLE_ROW_RE = re.compile(r"^\|(?!\s*-)(.+)\|\s*$", re.M)

# A verdict is the FIRST bolded run in the state cell — the row's headline. A
# row may go on to say a later slice landed while its headline is PARTIAL, and
# it is the headline that claims completion.
VERDICT_RE = re.compile(r"\*\*([A-Z][A-Z0-9 /+-]{2,})")

# Bolded verdicts that assert the work is finished. Everything else — PARTIAL,
# OPEN, BOUNDED, DESIGN CORRECTION REQUIRED — is not overclaiming and is skipped,
# exactly as a non-MET task row is.
COMPLETION_WORDS = ("DONE", "FIXED", "CLOSED", "LANDED", "PROVEN", "MET", "COMPLETE")

# A verdict that ALSO says part of the work is unfinished is not claiming the row
# is done, and must not be checked as though it were.
#
# Substring matching for the completion word was the whole classifier, so
# "third clause MET … the first two are partial" and "QUARANTINE DONE …;
# residual debt OPEN" both counted as completed rows (GPT 5.6, 2026-07-27). That
# is the wrong direction to be wrong in twice over: it demands evidence from a
# row that is honestly admitting it has none yet, and — because ANY surviving
# citation in the body passes — it lets the partial clauses' citations vouch for
# the completed one.
PARTIAL_WORDS = ("PARTIAL", "OPEN", "NOT MET", "UNMET", "PLAINLY FALSE")


def verdict_sentence(text: str) -> str:
    """The leading VERDICT sentence — everything up to the first full stop.

    Only this is classified. A row's body goes on to discuss residues, struck-out
    corrections and follow-ups, and scanning all of it for the word "partial"
    would silently stop checking rows that are genuinely complete — the same
    quiet-drop failure as reporting zero rows, one row at a time.
    """
    head = text.split(".", 1)[0]
    return head.upper()


def is_a_completion_claim(verdict: str) -> bool:
    """Does this verdict claim the WHOLE row is done, without qualification?"""
    head = verdict_sentence(verdict)
    if not any(word in head for word in COMPLETION_WORDS):
        return False
    return not any(word in head for word in PARTIAL_WORDS)


def task_sections(text: str) -> list[tuple[str, str, str]]:
    """(task number, title, body) for every task, body running to the next task."""
    out = []
    matches = list(TASK_RE.finditer(text))
    for i, m in enumerate(matches):
        end = matches[i + 1].start() if i + 1 < len(matches) else len(text)
        out.append((m.group(1), m.group(2), text[m.start() : end]))
    return out


def mixed_table_rows(text: str) -> list[tuple[str, str]]:
    """Rows whose verdict names a completion word AND qualifies it."""
    out = []
    for match in TABLE_ROW_RE.finditer(text):
        cells = [cell.strip() for cell in match.group(1).split("|")]
        if len(cells) < 2 or not cells[0] or cells[0].lower() == "workstream":
            continue
        head = verdict_sentence(cells[1])
        if any(word in head for word in COMPLETION_WORDS) and not is_a_completion_claim(cells[1]):
            out.append((cells[0], head.strip()[:60]))
    return out


def table_rows(text: str) -> list[tuple[str, str, str]]:
    """(row label, verdict, body) for every workstream table row claiming done."""
    out = []
    for match in TABLE_ROW_RE.finditer(text):
        cells = [cell.strip() for cell in match.group(1).split("|")]
        if len(cells) < 2 or not cells[0] or cells[0].lower() == "workstream":
            continue
        state = cells[1]
        verdict = VERDICT_RE.search(state)
        if not verdict:
            continue
        headline = verdict.group(1).strip()
        # The WHOLE state cell, not just the bolded run: a headline reading
        # "QUARANTINE DONE 2026-07-21; residual debt OPEN." qualifies itself
        # outside the bold, and reading only the bold calls it complete.
        if not is_a_completion_claim(state):
            continue
        out.append((cells[0], headline, match.group(0)))
    return out


class GitUnavailable(RuntimeError):
    """`git grep` could not run at all — which is not the same as "no match"."""


# Where IMPLEMENTATION evidence may live. Prose is excluded on purpose, and it
# is the whole correctness of this script: the first version grepped the entire
# repository, so every citation was found inside the roadmap it had just been
# parsed out of. It asked "does this document still contain the text I read from
# it", answered yes forever, and reported a clean bill of health for rows whose
# implementation had been deleted (GPT 5.6, 2026-07-27). A guard that cannot
# distinguish a live function from its own footnote is worse than no guard —
# it is a green light nobody double-checks.
SOURCE_PATHSPECS = [
    ":(exclude)docs/",
    ":(exclude)*.md",
]


def identifier_exists(name: str) -> bool:
    """Does this identifier appear in tracked SOURCE (never in prose)?"""
    result = subprocess.run(
        ["git", "grep", "-l", "-e", name, "--", *SOURCE_PATHSPECS],
        cwd=REPO_ROOT,
        capture_output=True,
        text=True,
    )
    # git grep: 0 = found, 1 = no match, >1 = it could not do the search (a bad
    # pathspec, a repository it refuses to open). Conflating the third case with
    # the second is how a broken checkout reports every row as rotten, which is
    # the same failure in the opposite direction — a guard that lies loudly
    # still lies.
    if result.returncode > 1:
        raise GitUnavailable(
            f"`git grep` failed ({result.returncode}): {result.stderr.strip()}"
        )
    return result.returncode == 0 and bool(result.stdout.strip())


def path_exists(candidate: str, document: Path) -> bool:
    """A cited path is checked AS a path — a file, not a string that occurs.

    Resolved against the DOCUMENT as well as the repo root: a planning page
    links to its siblings relatively (`engine/encounter-orchestration.md`), and
    resolving those only from the root reports a live document as deleted.
    """
    return (REPO_ROOT / candidate).exists() or (document.parent / candidate).exists()


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

    # Prove the search works before believing a single answer it gives. Without
    # this, a repository `git grep` refuses to open reports "every citation for
    # every row has vanished" — indistinguishable from a genuinely rotten
    # roadmap, and far more alarming.
    try:
        if not identifier_exists("identifier_exists"):
            print(
                "ERROR: the evidence search cannot find this script's own "
                "function, so it is not searching the source tree. Every "
                "answer below would be meaningless.",
                file=sys.stderr,
            )
            return 2
    except GitUnavailable as error:
        print(f"ERROR: {error}", file=sys.stderr)
        return 2

    claims: list[tuple[str, str]] = []
    # Rows deliberately NOT checked, REPORTED rather than dropped in silence. A
    # classifier that quietly stops looking at a row is the same failure as
    # reporting zero rows, one row at a time.
    partial: list[str] = []
    for number, title, body in task_sections(text):
        status = STATUS_RE.search(body)
        if not status:
            problems.append(f"Task {number} ({title}): no **Status** line at all")
            continue
        if not is_a_completion_claim(status.group(0)):
            # A row that says it is partial is not overclaiming; nothing to check.
            if any(word in status.group(0).upper() for word in COMPLETION_WORDS):
                partial.append(f"Task {number} ({title})")
            continue
        claims.append((f"Task {number} ({title})", body))
    for label, headline, body in table_rows(text):
        claims.append((f"{label} [{headline}]", body))
    for label, state in mixed_table_rows(text):
        partial.append(f"{label} [{state}]")

    if not claims and not problems:
        problems.append(
            "no completion claims found at all — neither a `### Task N` section "
            "claiming MET nor a table row with a bolded DONE/FIXED/CLOSED "
            "verdict. A guard reporting zero rows checked is not a clean bill of "
            "health; it means this document does not have a shape this script "
            "can read."
        )

    for label, body in claims:
        checked += 1

        # Evidence anywhere in the task body, not just the status line: the
        # convention here is a status sentence followed by paragraphs that cite.
        # A path is checked as a PATH and an identifier as source text; the two
        # kinds fail differently and a deleted file should say so.
        paths = set(PATH_RE.findall(body))
        idents = set(IDENT_RE.findall(body)) - paths
        cited = paths | idents
        if not cited:
            problems.append(
                f"{label}: claims done and cites NOTHING checkable. "
                "Name the test or the module that makes it true."
            )
            continue

        missing = sorted(
            [name for name in paths if not path_exists(name, Path(args.document))]
            + [name for name in idents if not identifier_exists(name)]
        )
        # One missing citation among many is usually prose (a word in backticks
        # that was never an identifier). ALL of them missing means the row is
        # pointing at a world that no longer exists.
        if missing and len(missing) == len(cited):
            problems.append(
                f"{label}: claims done and every citation is gone "
                f"from the source: {', '.join(missing)}"
            )

    print(f"checked {checked} row(s) claiming the work is done")
    for row in partial:
        print(f"  – not checked, verdict is qualified: {row}")
    for problem in problems:
        print(f"  ✗ {problem}")
    if not problems:
        print("  every completed row cites something that still exists")

    return 1 if (problems and args.check) else 0


if __name__ == "__main__":
    sys.exit(main())
