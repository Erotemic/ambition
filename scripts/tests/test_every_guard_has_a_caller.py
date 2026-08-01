"""A `check_*.py` that nothing runs is indistinguishable from one that passes.

Four of them were found this way on 2026-08-01: `modules_md.py` had a check mode
nobody called, the goal invoked `check_absence_contracts.py` WITHOUT `--check`
(the flag that lets it fail), and `check_doc_links.py` and
`check_roadmap_evidence.py` appeared in no runner at all.

⚠ **`check_doc_links.py` is why this is a test and not a note.** It was green for
the whole run *because a person was running it by hand after every doc edit*. The
greenness was evidence about the person, not about the repository, and it would
have stayed green right up until they stopped.

## Why the `check_` prefix is the rule

The first draft looked for "any script with a non-zero exit path" and had to
carve out `ecs_inventory.py`, `regen_music_registry.py`, `render_line_profiles.py`
and friends — GENERATORS, which legitimately exit non-zero and legitimately have
no caller in a test suite. An allowlist of those would need maintaining and would
be waived the first time it got in the way.

The naming convention already draws the line the allowlist was trying to draw, so
this uses it. A guard is called `check_*`; if a new one is not, it is not covered,
and that is a naming problem with an obvious fix rather than a silent hole.
"""

from __future__ import annotations

from pathlib import Path

REPO = Path(__file__).resolve().parents[2]

# Anywhere a guard can legitimately be invoked from.
CALLER_PATHS = (
    REPO / "scripts" / "run_tests.py",
    REPO / "scripts" / "tests",
    REPO / ".github" / "workflows",
    REPO / ".goal" / "active.json",
)


def _caller_text() -> str:
    chunks: list[str] = []
    for path in CALLER_PATHS:
        if path.is_dir():
            for child in sorted(path.rglob("*")):
                if child.is_file():
                    try:
                        chunks.append(child.read_text(encoding="utf-8"))
                    except (UnicodeDecodeError, OSError):
                        continue
        elif path.is_file():
            try:
                chunks.append(path.read_text(encoding="utf-8"))
            except (UnicodeDecodeError, OSError):
                pass
    return "\n".join(chunks)


def test_every_check_script_is_invoked_by_something():
    guards = sorted(p.stem for p in (REPO / "scripts").glob("check_*.py"))
    assert guards, "the glob found no guards at all, which means it is broken"

    callers = _caller_text()
    orphaned = [name for name in guards if name not in callers]
    assert not orphaned, (
        "these guards are never invoked, so they cannot fail and cannot be "
        f"trusted: {orphaned}. Add a `scripts/tests` case that runs the guard "
        "against the live tree — that suite runs first and cheaply in the "
        "backbone — or a `run_tests.py` job, or a CI step. A check with no "
        "caller is indistinguishable from one that passes."
    )
