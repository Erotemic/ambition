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


def inserted_at_build() -> dict[str, list[tuple[str, str]]]:
    found: dict[str, list[tuple[str, str]]] = {}
    for demo in sorted(REPO.glob("game/ambition_demo_*/src")):
        crate = demo.parent.name
        for path in sorted(demo.rglob("*.rs")):
            src = path.read_text(encoding="utf-8", errors="replace")
            for match in BUILD.finditer(src):
                for hit in INSERT.finditer(body_of(src, match.end() - 1)):
                    found.setdefault(crate, []).append(
                        (path.name, hit.group(1).split("::")[-1])
                    )
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
