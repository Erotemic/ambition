"""The planning docs' own bookkeeping must cite something a machine can check.

⛔ **Four instances of ONE defect on 2026-08-08**, each a hand-written cache of a
machine-checkable fact, going stale inside the file whose job is to be current:

* `docs/planning/README.md` — heading *"Where the open work is"* — pointed at the
  2026-08-06 ledger as *"the live run ledger. This is the current work"*, two days
  after that run ended, sending every new reader to a retired file carrying 24
  open marks that were not work;
* the live ledger's standing-state block said the rollback schema was at **v17**
  while `registry.rs` had said **18** since that morning;
* the same block's suite baseline said **318** when the suite was 319;
* a navigation table was wrong within two hours of being written.

⭐ **the repo already has the rule and did not apply it to itself.**
`docs/planning/README.md` says: *write a status as a CITATION, not a SITUATION —
name something a reader confirms or refutes in one grep.* These are the docs'
bookkeeping failing their own standard.

⚠ **what this can and cannot check.** It checks facts with a machine-readable
source of truth. A suite count needs the suite; a claim about what a design MEANS
needs a reader. Those stay human, and this file does not pretend otherwise —
adding a check here for something unverifiable would be the
`a_check_that_cannot_fail` defect one layer up.
"""

from __future__ import annotations

import re
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
PLANNING = REPO / "docs" / "planning"
README = PLANNING / "README.md"


def _live_ledger_link() -> str:
    """The path README advertises as the live run ledger."""
    text = README.read_text(encoding="utf8")
    marker = text.index("Where the open work is")
    # the first markdown link after the heading is the live ledger by construction
    match = re.search(r"\[`([^`]+)`\]\(([^)]+)\)", text[marker:])
    assert match, "README's 'Where the open work is' section has no link at all"
    return match.group(2)


def test_the_readme_points_at_a_ledger_that_exists():
    """⛔ It pointed at a retired one for two days. Nothing noticed."""
    target = (PLANNING / _live_ledger_link()).resolve()
    assert target.is_file(), (
        f"docs/planning/README.md advertises `{_live_ledger_link()}` as the live "
        f"run ledger and that file does not exist. This pointer is DATED — update "
        f"it in the same commit that opens a new ledger."
    )


def test_the_live_ledger_is_not_an_archived_one():
    """A ledger moves to docs/archive/ only once nothing in it is open."""
    link = _live_ledger_link()
    assert "archive" not in link, (
        f"README advertises `{link}` as the LIVE run ledger, but it is in the "
        f"archive. A ledger is archived when nothing in it is open — so whatever "
        f"is live now needs to be named here instead."
    )


def test_an_archived_ledger_has_no_open_marks():
    """⛔ Archiving a ledger with real rows in it buries the work.

    Twice on 2026-08-08 a retired ledger looked archivable and held live rows —
    an unguarded room-scope fix, and a continuity row that had been open since
    the day it was filed. Both were carried forward before the move.
    """
    archive = REPO / "docs" / "archive"
    # ⚠ `*-closed-sections.md` are EXCLUDED, and the exclusion is their own
    # contract rather than a convenience: those files archive individual sections
    # that had zero open rows, and their header states **"Nothing here is
    # edited"** — verbatim and lossless, so a mark inside one may not be touched.
    # ⛔ two of them nonetheless carry rows that read as open (`AE6`, `S49`,
    # *"milestone 5 is NOT reached"*). That is a question about ORPHANED WORK, not
    # about formatting, and stripping the marks would erase the evidence for it.
    # Tracked separately; see the live ledger.
    offenders = {
        path.relative_to(REPO): path.read_text(encoding="utf8").count("▢")
        for path in sorted(archive.glob("queue-*.md"))
        if not path.name.endswith("-closed-sections.md")
        and "▢" in path.read_text(encoding="utf8")
    }
    assert not offenders, (
        "an archived ledger still carries open marks:\n"
        + "\n".join(f"  {p}: {n}" for p, n in offenders.items())
        + "\nEither the work is live (carry it into the current ledger first) or "
        "the mark is prose (the README bans that: a box means a thing to do and "
        "nothing else)."
    )


def test_the_ledgers_rollback_schema_version_matches_the_source():
    """⛔ The standing-state block said v17 while the source said 18.

    Every brief quotes that block, so a stale line there propagates into work
    nobody re-checks.
    """
    registry = (
        REPO
        / "crates"
        / "ambition_platformer2d_runtime"
        / "src"
        / "rollback"
        / "registry.rs"
    )
    source = re.search(
        r"GGRS_ROLLBACK_SCHEMA_VERSION\s*:\s*u32\s*=\s*(\d+)",
        registry.read_text(encoding="utf8"),
    )
    assert source, "registry.rs no longer declares GGRS_ROLLBACK_SCHEMA_VERSION"
    actual = source.group(1)

    ledger = PLANNING / _live_ledger_link()
    claimed = re.search(
        r"rollback schema is at \*\*v(\d+)\*\*", ledger.read_text(encoding="utf8")
    )
    if claimed is None:
        return  # a ledger need not state it; if it does, it must be right
    assert claimed.group(1) == actual, (
        f"{ledger.relative_to(REPO)} says the rollback schema is at "
        f"v{claimed.group(1)}; registry.rs says {actual}. A registration change "
        f"moves the version, the live baseline dump and the slice-evidence JSON "
        f"together — and this line too."
    )
