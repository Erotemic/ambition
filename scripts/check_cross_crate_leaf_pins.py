#!/usr/bin/env python3
"""How many ordering edges name another crate's FUNCTION instead of its SET?

`.after(other_crate::some_system)` is a consumer reaching past a crate's public
surface to grab an internal leaf and hold it still. It compiles, it is invisible
in every test, and it makes the named function un-renameable and un-splittable by
anyone who does not first grep the whole workspace. `.after(OtherCrate::SomeSet)`
says the same thing against a name the owning crate chose to expose, and the
owner stays free to add, split or reorder members behind it.

This counts the first kind and holds the number down.

## It reached zero, so it is now a ban

It began as a ratchet, and that was the right shape at 35: the edges are
legitimate — a consumer that needs to run after projectile stepping is not wrong
to say so — and what is wrong is only the *address*. The fix for any row is never
"delete the edge", it is "give the target a set and pin that", so a ban would
have been waived everywhere on day one.

The last one converted on 2026-08-02. The ceiling is 0 and every mechanism below
is unchanged: the guard still fails in both directions, and "below the ceiling"
is simply unreachable now. What it means in practice is that a NEW
`.after(other_crate::some_function)` fails immediately, with the three-step fix
in the message.

## ⛔ the drift this closes, which is the reason it exists

The conversion campaign was run off a hand-maintained list in a planning doc,
and BOTH the list and the attempt to check it by hand were wrong.

* the list recorded `tick_player_brains` as an ordinary row when it is the one
  case that already has a two-member set, i.e. the one row that needs a
  DECISION rather than an edit.
* checking the list with `git grep NAME | head -8` then reported
  `sync_local_player_input_frame` as a single intra-crate pin. It has two
  cross-crate pins. They were on lines 9 and 10 of the output. The truncation
  was read as an absence, and the correction was published before the count
  below refuted it.

⭐ that second one is the sharper lesson and it is why this file exists: a tally
kept by hand drifts from the tree SILENTLY, and the ad-hoc command reached for
to re-check it has its own silent limit. Only something that walks the whole
tree and prints a number can be wrong in a way anyone NOTICES.

⚠ **the honest limit: this is textual, like its siblings.** It reads qualified
paths out of `.before(...)`/`.after(...)` and classifies by the FIRST segment
(the crate) and the LAST (snake_case is a function, CamelCase is a set or type).
It therefore cannot see a pin written against a `use`-imported bare name, one
built by a helper, or one inside a macro. Those are stated rather than papered
over: the shape this catches is the shape the campaign has actually been
converting, and a cleverer instrument nobody trusts is worse.

⚠ **a bare or `crate::`-relative path is INTRA-crate and not counted.** That is
not a gap, it is the rule: `actors::sync_visuals` inside `ambition_render` is a
crate ordering its own systems, which is exactly what it should do. 17 such
edges live in one render file and none of them is a finding.

Usage:
    python3 scripts/check_cross_crate_leaf_pins.py
    python3 scripts/check_cross_crate_leaf_pins.py --list
"""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
SOURCE_ROOTS = ["crates", "game"]

# The ceiling — now 0. It may only ever be LOWERED, so this is terminal: there is
# no legitimate edit that raises it. If this number and the tree ever disagree,
# the tree wins and this is stale, which is the whole point of measuring here.
MAX_CROSS_CRATE_LEAF_PINS = 0

# `.after(a::b::c)` / `.before(a::b::c)` — one qualified path, no nesting.
#
# ⛔ the trailing comma and the newline are BOTH load-bearing. The first version
# required `)` immediately after the path, and rustfmt — run on a file this very
# campaign had just edited — wrapped one long pin onto its own line with a
# trailing comma, at which point the guard silently stopped counting it and the
# ceiling looked one lower than the tree. A formatter is not an adversary; it is
# the most likely thing to reshape these call sites, so the pattern has to
# survive what it does. `\s` spans newlines in Python, so the wrapping itself was
# never the problem — the comma was.
PIN = re.compile(
    r"\.(?:before|after)\(\s*([A-Za-z_][A-Za-z_0-9]*(?:::[A-Za-z_][A-Za-z_0-9]*)+)\s*,?\s*\)"
)


def _strip_comments(text: str) -> str:
    """Blank `//` comment bodies.

    A doc comment that NAMES a pin ("Runs after `foo::bar`") is prose about an
    edge, not an edge, and counting it would inflate the number with the very
    documentation this campaign asks people to write.
    """
    return "\n".join(line.split("//", 1)[0] for line in text.splitlines())


def _crate_of(path: Path, repo: Path = REPO) -> str | None:
    """The crate a source file belongs to, from its path."""
    rel = path.relative_to(repo).parts
    if len(rel) >= 2 and rel[0] in SOURCE_ROOTS:
        return rel[1]
    return None


def _is_test_path(path: Path) -> bool:
    parts = path.parts
    return "tests" in parts or path.name in {"tests.rs", "test.rs"}


def collect(repo: Path = REPO) -> list[tuple[str, int, str, str]]:
    """(file, line, target_crate, path) for every cross-crate leaf pin.

    `repo` is a parameter so the guard's own tests can walk a SYNTHETIC tree.
    ⛔ this campaign has twice mistaken a truncated command's output for an
    absence; a walk that can only ever be run against the real repo cannot be
    shown to find a planted pin, and "it printed nothing" would prove nothing.
    """
    found: list[tuple[str, int, str, str]] = []
    for root in SOURCE_ROOTS:
        if not (repo / root).is_dir():
            continue
        for src in sorted((repo / root).rglob("*.rs")):
            if _is_test_path(src):
                continue
            own = _crate_of(src, repo)
            if own is None:
                continue
            text = _strip_comments(src.read_text(encoding="utf-8", errors="replace"))
            for match in PIN.finditer(text):
                target = match.group(1)
                head, tail = target.split("::")[0], target.split("::")[-1]
                # Only a path rooted at a DIFFERENT ambition crate is a
                # cross-crate address. `crate::`, `self::` and bare module paths
                # are a crate ordering itself.
                if not head.startswith("ambition_") or head == own:
                    continue
                # CamelCase tail is a set or a type — the shape we WANT.
                if not tail[0].islower():
                    continue
                line = text[: match.start()].count("\n") + 1
                found.append((str(src.relative_to(repo)), line, head, target))
    return found


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--list", action="store_true", help="print every pin")
    args = parser.parse_args()

    found = collect()
    if args.list:
        by_crate: dict[str, list[tuple[str, int, str, str]]] = {}
        for row in found:
            by_crate.setdefault(row[2], []).append(row)
        for crate in sorted(by_crate):
            print(f"\n{crate}  ({len(by_crate[crate])})")
            for file, line, _, target in sorted(by_crate[crate]):
                print(f"  {file}:{line}  {target}")
        print()

    count = len(found)
    if count > MAX_CROSS_CRATE_LEAF_PINS:
        print(
            f"FAIL: {count} cross-crate leaf pins, ceiling is "
            f"{MAX_CROSS_CRATE_LEAF_PINS}.\n\n"
            "A new `.after(other_crate::some_function)` reaches past that "
            "crate's surface to hold an internal leaf still.\n\n"
            "1. FIRST check whether the target already has a set. One pin in "
            "this campaign looked like a missing boundary and was two demos "
            "disagreeing about an existing one — a second name for a single "
            "authority is worse than the leaf pin it replaces.\n"
            "2. Otherwise give the target a SystemSet beside its definition. If "
            "it already sits in a multi-member set, NEST a single-member one "
            "inside — that is exactly equivalent, and it is the only form that "
            "works when the consumer shares the parent set.\n"
            "3. Write at the definition WHY the set holds the members it holds. "
            "Every set here has had a different reason; the next reader can only "
            "widen one safely if the reason is written down.\n\n"
            "Run with --list to see every row.",
            file=sys.stderr,
        )
        return 1

    if count < MAX_CROSS_CRATE_LEAF_PINS:
        print(
            f"FAIL: {count} cross-crate leaf pins, but the ceiling still says "
            f"{MAX_CROSS_CRATE_LEAF_PINS}.\n\n"
            "You converted one — lower MAX_CROSS_CRATE_LEAF_PINS in this file "
            "to match, in the same commit. A ratchet that is not tightened is a "
            "ratchet that permits the next regression back up to the old bar.",
            file=sys.stderr,
        )
        return 1

    print(f"OK: {count} cross-crate leaf pins, at the ceiling.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
