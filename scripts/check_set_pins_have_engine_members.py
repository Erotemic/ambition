#!/usr/bin/env python3
"""Which engine ordering edges point at a set only a GAME ever fills?

⚠ **the name used to be `check_set_pins_are_not_vacuous`, and it promised more
than it delivers.** A pin is vacuous for more than one reason, and this checks
exactly ONE of them: that the set's members are all registered by game crates.
A pin whose member is registered in a DIFFERENT SCHEDULE is equally vacuous —
`Update`'s `.before(CoreSimulation)` orders nothing against a member registered
in `GgrsSchedule` — and this checker cannot see that, because it does not model
schedules and modelling them textually is a checker that would lie in a new way.
That hazard is real and was found independently (07-31 ledger, S39). It is
addressed where it belongs: `check_rollback_mutators_run_in_sim.py` for the
schedule question, and the compiler-side sets themselves.

A guard named for a property it does not have is worse than a narrower one, so
this one is named for the question it actually asks. (GPT review of
5cc4337..47d7de3, finding 10, whose own remedy — teach this checker schedule
identity — is refused: its closing paragraph forbids another general-purpose
checker, and that applies to itself.)


`.before(SomeSet)` against a set with no members is **vacuously satisfied**.
Bevy does not warn, no test fails, and the edge reads in the source exactly like
one that works. So when an ENGINE crate orders against a set whose members are
all registered by an APP, every composition that is not that app silently loses
the ordering — the same failure shape as
`check_engine_systems_are_engine_installed.py`, one level up: there the SYSTEM is
app-only, here the SET's membership is.

This became worth checking on 2026-08-02. A campaign converted 51 cross-crate
leaf pins into 32 named sets, which is a strict improvement — a set can be
widened behind its name, a function cannot — but it also multiplied the number of
places this hazard can hide. A leaf pin at least names something that exists.

## What it checks

For every `.before(X)` / `.after(X)` in an ENGINE crate where `X` is a SystemSet
DEFINED in this workspace: find every `.in_set(X)`. If all of them are in game
crates, the pin is vacuous outside those games, and the row must be fixed or
WAIVED here with a reason.

⚠ **a waiver is the normal outcome.** An extension point that a game fills is a
legitimate design — `ContentRoomResetSet` exists precisely so content can join
it, and "no content installed, so nothing to order around" is correct behaviour,
not a bug. The point is that the choice is *made* and written down, rather than
being whichever composition happened to wire it.

## What it deliberately is not

⚠ **it cannot see Bevy's own sets, and does not try.** `UiSystem::Layout`,
`TransformSystem::Propagate`, `InputSystem::Unify` and friends have no `.in_set`
anywhere in this workspace because Bevy registers their members. Eight such
names showed up in the first run as false findings; restricting to sets DEFINED
here removes all eight. A set defined in a third-party crate is out of scope for
the same reason.

⚠ and it is textual, like its siblings: comments stripped, production source
only, `.in_set(Path::To::Set)` matched by its LAST segment. A membership added
through a helper or a macro is invisible.

Usage:
    python3 scripts/check_set_pins_have_engine_members.py
    python3 scripts/check_set_pins_have_engine_members.py --list
"""

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
    # derived per call, never cached in a module global. The first version
    # cached it, and the guard's own test suite walks SYNTHETIC trees before
    # asserting against the real one — the real-tree assertion would have run
    # with a fake crate list and quietly meant nothing.
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
