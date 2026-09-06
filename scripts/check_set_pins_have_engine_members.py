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


def _is_test_path(path: Path) -> bool:
    return "tests" in path.parts or path.name in {"tests.rs", "test.rs"}


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
