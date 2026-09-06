#!/usr/bin/env python3
"""Reject retired crate or type names in live repository content.

The checker uses an explicit retired-name map and a plain substring search so
escaped regex/text representations are not missed. Historical records are
excluded from the live-content scan because their old names are evidence rather
than active references. Add a row when a crate or tracked type is renamed."""

from __future__ import annotations

import subprocess
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]

# name-that-is-gone -> what it is now. Add a row the moment a crate is renamed.
RETIRED_CRATE_NAMES = {
    "ambition_engine_core": "ambition_platformer2d_core",
    "ambition_platformer_primitives": "ambition_platformer2d_shared_tangle",
    "ambition_world": "ambition_platformer2d_world",
    "ambition_ldtk_map": "ambition_platformer2d_ldtk",
    "ambition_portal_presentation": "ambition_portal2d_presentation",
    "ambition_platformer_provider": "ambition_platformer2d_provider",
    "ambition_runtime": "ambition_platformer2d_runtime",
    "ambition_host": "ambition_platformer2d_host",
    "ambition_actors": "ambition_platformer2d_actor_monolith",
    # `ambition_portal` is deliberately ABSENT: it is a prefix of the live
    # `ambition_portal2d`, so a substring search for it matches every correct
    # use. The prefix relation is what makes it safe to omit — a stale
    # `ambition_portal` on its own cannot survive `cargo check`, because unlike
    # an owner string in a test fixture it is always a real crate path.
    "ambition_engine": "ambition_platformer2d_core",
    "ambition_pulse": "examples/capability_demo",
    # ── retired TYPE names ──
    #
    # The row that mattered told a reader to migrate `InputMap<SandboxAction>` — a grep for which
    # returns nothing, so the honest conclusion from that row was "this is already done". A stale
    # crate name breaks a build; a stale TYPE name in a planning doc quietly retires a piece of
    # work.
    "SandboxAction": "Platformer2dInputActionMonolith",
    # the filter that separates them is whether the mention sits under an OPEN row: only there
    # does a dead name send somebody after work.
    #
    # These two failed that filter.
    "SandboxResetCommitted": "NewGameResetCommitted",
    "process_sandbox_reset_request": "process_new_game_reset_request",
}

# Records of what happened. Never rewritten.
HISTORICAL_PREFIXES = (
    "docs/archive/",
    "docs/brainstorms/",
    "docs/reviews/",
    "dev/journals/",
    "dev/benchmark-candidates/",
    ".agent/",
    ".llm_resource_tally/",
    # They are not a record of the past — they are the LIVE WORKLIST this repository is driven from,
    # so a dead name in one does not misdescribe history, it hands the next reader a task they
    # cannot find.
)

# the guard's own test file is skipped because its FIXTURES are retired names —
# that is what it tests. It went unnoticed at first because `git ls-files` did not
# yet list the file: the live-tree ratchet passed while its own counter-example was
# untracked, which is the "green at minute zero" trap one level in.
SKIP_NAMES = (
    "Cargo.lock",
    "check_retired_crate_names.py",
    "test_retired_crate_names.py",
)

# Whole files whose CONTENT is a quotation of old source. Listed individually,
# with the reason, rather than pattern-matched.
WAIVED_FILES = {
    "tools/optimization_report/apply_headless_hygiene_patch.py":
        "a frozen one-off migration; its payload is a literal snippet of the "
        "source it used to patch, so the retired name IS the data",
}

# A line may keep a retired name when it is explicitly recording history.
HISTORICAL_MARKERS = (
    "# was:",
    "was: ",
    "libambition_",
    "former",
    "formerly",
    "no longer",
    "was collapsed",
    "was renamed",
    "renamed to",
    "is gone",
    "deletion of",
    "was deleted",
    "collapsed long ago",
    "named here before",
    "when this was authored",
    "then called",
    "Earlier notes",
    "retired",
)


# UNTRACKED files the guard must still read.
#
# `.goal/active.json` holds the goal harness's own check COMMANDS, and it is not in git.
#
# `done-*.json` are archived goals — records, skipped like any other history.
#
# `root` is a parameter so the RULE can be tested without a goal being armed on the machine running
# the test.
def extra_paths(root: Path | None = None) -> list[str]:
    goal = (root or REPO) / ".goal"
    if not goal.is_dir():
        return []
    return [
        f".goal/{p.name}"
        for p in sorted(goal.glob("*.json"))
        if not p.name.startswith("done-")
    ]


def tracked_files() -> list[str]:
    raw = subprocess.run(
        ["git", "ls-files", "-z"], cwd=REPO, capture_output=True, text=True, check=True
    ).stdout
    return [f for f in raw.split("\0") if f] + extra_paths()


def offences(files: list[str]) -> list[tuple[str, int, str, str]]:
    found: list[tuple[str, int, str, str]] = []
    for f in files:
        if f in WAIVED_FILES:
            continue
        if f.startswith(HISTORICAL_PREFIXES) or Path(f).name in SKIP_NAMES:
            continue
        path = REPO / f
        if not path.is_file():
            continue
        try:
            text = path.read_text(encoding="utf-8")
        except (UnicodeDecodeError, OSError):
            continue
        if not any(old in text for old in RETIRED_CRATE_NAMES):
            continue
        for number, line in enumerate(text.splitlines(), 1):
            for old, new in retired_names_in_line(line):
                found.append((f, number, old, new))
    return found


def retired_names_in_line(line: str) -> list[tuple[str, str]]:
    """Every retired crate name this line still uses as a NAME.

    Split out from the file walk so it can be tested directly — the escape cases
    this exists for are one-liners, and a test that had to build a file tree to
    reach them would not get written.
    """
    if any(marker in line for marker in HISTORICAL_MARKERS):
        return []
    hits: list[tuple[str, str]] = []
    for old, new in RETIRED_CRATE_NAMES.items():
        # Plain substring, then a TRAILING boundary only.
        start = 0
        while (at := line.find(old, start)) != -1:
            start = at + 1
            tail = line[at + len(old):at + len(old) + 1]
            if tail and (tail.isalnum() or tail == "_"):
                continue
            hits.append((old, new))
            break
    return hits


def main() -> int:
    found = offences(tracked_files())
    if found:
        print("Retired crate names are still live:\n")
        for f, number, old, new in found:
            print(f"  {f}:{number}: `{old}` was renamed to `{new}`")
        print(
            "\nIf the line is a RECORD of something that happened (a linker error,\n"
            "a dated review, a `# was:` note), move it under a historical path or\n"
            "mark the line, rather than rewriting history."
        )
        return 1
    print(f"No retired crate name is live ({len(RETIRED_CRATE_NAMES)} tracked).")
    return 0


if __name__ == "__main__":
    sys.exit(main())
