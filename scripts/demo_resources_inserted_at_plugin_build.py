#!/usr/bin/env python3
"""Which resources does a DEMO plugin insert into whatever app composes it?

⭐ THE QUESTION, and it was asked three times in one day on 2026-09-06 by three
different people finding three different bugs:

    99ab15e32  Smash was deleting another plugin's resources on the way out
    03e6f6638  a stock loss wipes the Limit
    0d6ace5a7  dying granted the Limit for a frame

Each was smash-plugin state reaching `ambition_app`, the aggregate game. ⇒ the
class is worth a census even though no single instance needed one.

⛔⛔ AND `run_if` CANNOT COVER THIS, WHICH IS THE POINT. A demo gates its SYSTEMS
by mode — smash binds `in_mode(SMASH_MODE)` to a `gate` and reuses it, mary_o
spells it inline eleven times — but `app.insert_resource(..)` inside
`Plugin::build` runs when the plugin is ADDED, and there is no run condition on a
plugin's build. So a resource inserted there exists in every composition that
includes the demo, for the whole process, whatever mode the player is in.

⭐⭐ AND THE OBVIOUS NARROWING RULE IS NOT THE RIGHT ONE, which the fighter lane
caught before it was written. "Inserted at build AND defined OUTSIDE the demo"
catches `PlayerManaRegen` and misses `SmashLimitFill` — smash defines that type
itself, and it caused two of the three bugs. ⇒ the property those two share is
that their READERS query a component the demo does not own (`BodyMana`), not that
the resource's type is foreign. That is a reachability question, which is exactly
the half this census does not answer; it is named here so the next person does not
implement the tidy rule and believe it covers the class.

⭐⭐ AND THE OBVIOUS NARROWING RULE IS NOT THE RIGHT ONE, which the fighter lane
caught before it was written. "Inserted at build AND defined OUTSIDE the demo"
catches `PlayerManaRegen` and MISSES `SmashLimitFill` -- smash defines that type
itself, and it caused two of the three bugs. ⇒ the property those two share is
that their READERS query a component the demo does not own (`BodyMana`), not that
the resource's own type is foreign. That is a reachability question, which is
exactly the half this census does not answer; it is named here so the next person
does not implement the tidy rule and believe it covers the class.

⚠ THIS REPORTS AND DOES NOT FAIL. Inserting a resource at build is often exactly
right: a demo's own tuning, its own select-screen state, and the engine's
`FeatureEcsWorldOverlay` (which mary_o and sanic both insert because a composition
without the engine's own plugin still needs the overlay to exist). What matters is
the SHARED ones — a type the demo does not define, that another system in the
aggregate app also reads.

⭐ THE NUMBER TO WATCH IS THE PER-DEMO COUNT, and the spread is the finding:
measured 2026-09-06, smash 14, mary_o 5, sanic 3, twintrack 2. Nearly three times
the next demo, and the three bugs above all came from that column.

⛔ A MEASUREMENT NOTE, because this file's own matcher was wrong twice. `fn build`
is written both `&mut App` and `&mut bevy::prelude::App` in this repo, and the
short spelling alone reported smash as ZERO — the demo with the most insertions
and every known instance of the bug. A census that under-matches reports the
cleanest possible answer about the dirtiest crate.
"""
from __future__ import annotations

import pathlib
import re
import sys

REPO = pathlib.Path(__file__).resolve().parent.parent
#: ⚠ Both spellings. See the docstring: the short one alone reported smash as 0.
BUILD = re.compile(r"fn build\(\s*&self,\s*app:\s*&mut\s*[\w:]*App\s*\)\s*\{")
INSERT = re.compile(r"(?:init_resource::<|insert_resource\()\s*([A-Za-z_][\w:]*)")
MIN_DEMOS = 3


def body_of(src: str, start: int) -> str:
    """The `{ .. }` block opening at `start`, brace-balanced."""
    depth, i = 0, start
    while i < len(src):
        if src[i] == "{":
            depth += 1
        elif src[i] == "}":
            depth -= 1
            if depth == 0:
                return src[start:i]
        i += 1
    return src[start:]


def own_types(demo: pathlib.Path) -> set[str]:
    #: Type names this demo defines, so a FOREIGN one can be told apart.
    names: set[str] = set()
    for path in demo.rglob("*.rs"):
        names |= set(
            re.findall(
                r"(?:pub )?struct (\w+)",
                path.read_text(encoding="utf-8", errors="replace"),
            )
        )
    return names


def inserted_at_build() -> dict[str, list[tuple[str, str]]]:
    found: dict[str, list[tuple[str, str]]] = {}
    for demo in sorted(REPO.glob("game/ambition_demo_*/src")):
        crate = demo.parent.name
        mine = own_types(demo)
        for path in sorted(demo.rglob("*.rs")):
            src = path.read_text(encoding="utf-8", errors="replace")
            for match in BUILD.finditer(src):
                body = body_of(src, match.end() - 1)
                for hit in INSERT.finditer(body):
                    name = hit.group(1).split("::")[-1]
                    # ⭐ `insert_resource` OVERWRITES; `init_resource` is a no-op
                    # when the resource is already present. On a type the demo
                    # does NOT define, that difference is Jon's `99ab15e32`
                    # ("Smash was deleting another plugin's resources on the way
                    # out") arriving from the other direction.
                    # ⚠ Lowercase names are VARIABLES, not types
                    # (`insert_resource(goal_pole)`) — counting them as foreign
                    # types was this arm's first false positive.
                    overwrites = (
                        hit.group(0).startswith("insert_resource")
                        and name not in mine
                        and name[:1].isupper()
                    )
                    label = name + ("  ⚠ OVERWRITING insert of a foreign type" if overwrites else "")
                    found.setdefault(crate, []).append((path.name, label))
    return found


def main() -> int:
    found = inserted_at_build()
    if len(found) < MIN_DEMOS:
        print(
            f"FAIL: only {len(found)} demo(s) matched — the sweep is broken, not "
            f"the tree.\n  Expected at least {MIN_DEMOS}. Check the `fn build` "
            "spellings this repo uses.",
            file=sys.stderr,
        )
        return 1
    total = sum(len(v) for v in found.values())
    print(f"resources inserted at demo PLUGIN BUILD: {total} across {len(found)} demos")
    for crate, hits in sorted(found.items(), key=lambda kv: -len(kv[1])):
        print(f"  {crate}: {len(hits)}")
        for name, ty in hits:
            print(f"      {name}: {ty}")
    print(
        "\n⇒ Reported, not enforced. `run_if` cannot gate a plugin's BUILD, so each of\n"
        "  these exists in every composition that includes the demo, for the whole\n"
        "  process. That is often right; the ones to look at are types the demo does\n"
        "  not define, which another system in the aggregate app also reads."
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
