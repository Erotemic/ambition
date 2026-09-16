#!/usr/bin/env python3
"""Report engine ordering edges that target sets with no engine-owned members.

A Bevy ordering edge against an empty set is vacuous in that schedule. This check
compares engine set pins with engine-installed systems so host/game-only extension
sets are not mistaken for active ordering constraints."""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
ENGINE_ROOT = "crates"
GAME_ROOT = "game"
SOURCE_ROOTS = [ENGINE_ROOT, GAME_ROOT]

PIN = re.compile(
    r"\.(?:before|after)\(\s*([A-Za-z_][A-Za-z_0-9]*(?:::[A-Za-z_][A-Za-z_0-9]*)*)\s*,?\s*\)"
)
IN_SET = re.compile(
    r"\.in_set\(\s*([A-Za-z_][A-Za-z_0-9]*(?:::[A-Za-z_][A-Za-z_0-9]*)*)\s*,?\s*\)"
)
# A `#[derive(..., SystemSet, ...)]` immediately above a struct/enum item.
SET_DEF = re.compile(
    r"#\[derive\([^)]*\bSystemSet\b[^)]*\)\][^;{]*?\b(?:struct|enum)\s+([A-Za-z_][A-Za-z_0-9]*)",
    re.S,
)

# ── The waivers ──
#
# name → why an engine pin against this set may be vacuous outside a game. Every
# entry is a DECISION that the empty case is correct behaviour; deleting one is
# how you say "actually every composition should get this ordering".
WAIVERS: dict[str, str] = {
    "ContentRoomResetSet": (
        "an EXTENSION POINT by construction — the engine defines it so a content "
        "layer can join, and the name says so. A composition with no content has "
        "nothing to order around, which is the correct empty case rather than a "
        "lost edge."
    ),
    "PresentationSetupSet": (
        "same shape: the engine names the setup slot, each app fills it with its "
        "own presentation. A headless or sim-only composition legitimately has no "
        "presentation setup to sequence."
    ),
    "MenuNavConsume": (
        "the members are the two INVENTORY BACKENDS' directional nav, and both "
        "are app-local by design (a demo does not get the app's inventory). A "
        "composition with no backend has no nav to land before, so the touch "
        "gesture fold's `.before` is correctly vacuous there."
    ),
}


def _strip_comments(text: str) -> str:
    return "\n".join(line.split("//", 1)[0] for line in text.splitlines())


#: ⭐ MOVED to `scripts/lib/test_paths.py` 2026-09-16 and re-exported here. One
#: of FIVE drifted copies; this one missed `*_tests.rs`, and like all five it
#: missed a file whose inner `#![cfg(test)]` compiles it out entirely.
#: ⚠ The union sees MORE test files, so this check sees FEWER sources — the
#: green direction, which `POPULATION_FLOOR` above exists to make reviewable.
sys.path.insert(0, str(Path(__file__).resolve().parent / "lib"))
from test_paths import is_test_path  # noqa: E402


def _is_test_path(path: Path) -> bool:
    return is_test_path(path)


def _sources(repo: Path):
    for root in SOURCE_ROOTS:
        if not (repo / root).is_dir():
            continue
        for src in sorted((repo / root).rglob("*.rs")):
            if _is_test_path(src):
                continue
            rel = src.relative_to(repo).parts
            if len(rel) < 2:
                continue
            yield src, rel[0], rel[1]


#: What this scan must still be able to SEE. ⛔ THE FAILURE MODE OF A
#: SOURCE-READING GUARD IS A CLEAN REPORT: a pattern that stops matching, a
#: source root that moves, or a test-path rule that widens all produce FEWER
#: findings, and fewer findings reads as good news.
#:
#: ⛔⛤ MEASURED ELSEWHERE IN THIS REPOSITORY, THREE TIMES, ALWAYS IN THE GREEN
#: DIRECTION: `check_rollback_mutators_run_in_sim.py` saw 1 type of 113 for
#: months, then hid six behind `SessionWorldMut<T>`, then thirteen behind a
#: `#[derive(SystemParam)]` bundle. Each time the report got shorter and nobody
#: could tell. ⇒ Raise a floor when the tree genuinely grows; a DROP is the
#: signature of the next silent hole.
#:
#: ⚠ AND THIS FLOOR HAS TO LIVE HERE RATHER THAN IN A SWEEP. An external probe
#: cannot redirect `collect(repo: Path = REPO)` by patching the module global —
#: the default argument bound at import — so it re-scans the real tree and
#: passes. Measured 2026-09-16 by writing that probe and getting 14 false
#: positives out of 29 scripts.
#:
#: ⛔⛤ **LOWERED 2026-09-16 FROM 1300, DELIBERATELY, AND THE FLOOR IS WHY ANYONE
#: KNOWS.** Repointing `_is_test_path` at the single owner in
#: `scripts/lib/test_paths.py` took `sources scanned` 1367 -> 1293 and turned
#: this check RED. The drop is CORRECT: this copy had never matched
#: `*_tests.rs`, and all 74 newly excluded files are that pattern plus the four
#: whose inner `#![cfg(test)]` compiles them out entirely. `sets pinned` fell
#: 225 -> 222, which is three pins that lived in test files and were never
#: production pins.
#: ⇒ That is the consolidation this row asked for arriving VISIBLY rather than
#: as a shorter report nobody could question.
POPULATION_FLOOR = {
    "sources scanned": 1280,
    "sets defined": 120,
    "sets pinned": 210,
}


def population_sizes(repo: Path = REPO) -> dict[str, int]:
    defined: set[str] = set()
    pinned: set[str] = set()
    scanned = 0
    for src, _root, _crate in _sources(repo):
        scanned += 1
        text = _strip_comments(src.read_text(encoding="utf-8", errors="replace"))
        defined.update(match.group(1) for match in SET_DEF.finditer(text))
        pinned.update(match.group(1).split("::")[-1] for match in PIN.finditer(text))
    return {
        "sources scanned": scanned,
        "sets defined": len(defined),
        "sets pinned": len(pinned),
    }


def population_shortfalls(repo: Path = REPO) -> list[str]:
    sizes = population_sizes(repo)
    return [
        f"{label}: {sizes[label]} visible, floor is {floor}"
        for label, floor in POPULATION_FLOOR.items()
        if sizes[label] < floor
    ]


def collect(repo: Path = REPO) -> list[tuple[str, str, list[str], list[str]]]:
    """(set, defining_crate, engine crates pinning it, crates registering members)."""
    game_root = repo / GAME_ROOT
    # Derive this per call because tests run the collector against synthetic
    # trees before querying the real repository.
    game_crates = {p.name for p in game_root.iterdir()} if game_root.is_dir() else set()

    defined: dict[str, str] = {}
    members: dict[str, set[str]] = {}
    pins: dict[str, set[str]] = {}
    pin_site: dict[str, str] = {}

    for src, root, crate in _sources(repo):
        text = _strip_comments(src.read_text(encoding="utf-8", errors="replace"))
        for match in SET_DEF.finditer(text):
            defined.setdefault(match.group(1), crate)
        for match in IN_SET.finditer(text):
            members.setdefault(match.group(1).split("::")[-1], set()).add(crate)
        for match in PIN.finditer(text):
            leaf = match.group(1).split("::")[-1]
            if root != ENGINE_ROOT:
                continue
            pins.setdefault(leaf, set()).add(crate)
            pin_site.setdefault(leaf, f"{src.relative_to(repo)}")

    findings = []
    for name, pinning in sorted(pins.items()):
        if name not in defined:
            # Not ours — Bevy's or a dependency's. See the module note.
            continue
        registering = members.get(name, set())
        if registering and any(crate not in game_crates for crate in registering):
            continue
        findings.append(
            (name, defined[name], sorted(pinning), sorted(registering))
        )
    return findings


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--list", action="store_true", help="print every row, waived included")
    args = parser.parse_args()

    findings = collect()

    # ⛔ BEFORE THE FINDINGS. "No offenders" over a collapsed population is the
    # one thing this guard cannot otherwise report, and every clean line below
    # would be describing a scan that had stopped working.
    shortfalls = population_shortfalls()
    if shortfalls:
        print(
            "the scan lost reach — it can no longer see part of its own "
            "population:\n\n  " + "\n  ".join(shortfalls) + "\n\n"
            "A source-reading guard fails by reporting LESS, so find the "
            "pattern or path rule that stopped matching before trusting any "
            "verdict here. If the drop is legitimate, lower POPULATION_FLOOR in "
            "the same commit that causes it.",
            file=sys.stderr,
        )
        return 1
    if args.list:
        for name, owner, pinning, registering in findings:
            mark = "WAIVED" if name in WAIVERS else "OPEN  "
            where = ", ".join(registering) or "NOBODY"
            print(f"{mark} {name:24s} defined in {owner}\n"
                  f"         pinned by {', '.join(pinning)}\n"
                  f"         members registered by {where}")
        print()

    unwaived = [f for f in findings if f[0] not in WAIVERS]
    if unwaived:
        lines = []
        for name, owner, pinning, registering in unwaived:
            where = ", ".join(registering) or "NOBODY — the set has no members at all"
            lines.append(
                f"  {name} (defined in {owner})\n"
                f"    pinned by engine crate(s): {', '.join(pinning)}\n"
                f"    members registered only by: {where}"
            )
        print(
            "an engine ordering edge points at a set only a game fills, so it is "
            "VACUOUS in every other composition:\n\n"
            + "\n".join(lines)
            + "\n\nBevy does not warn and no test fails — the edge reads exactly "
            "like one that works. Either move the member registration into an "
            "engine plugin so every composition gets it, or add a WAIVER in this "
            "file saying why the empty case is correct.",
            file=sys.stderr,
        )
        return 1

    print(f"OK: {len(findings)} app-filled set(s), all waived with a reason.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
