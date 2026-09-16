#!/usr/bin/env python3
"""Who reads each `SessionScopedResources` member, besides the reset that owns it?

C03 asks which session-scoped App resources could become direct `SessionRoot`
ownership. The first question is which of them are RESET-ONLY — declared, reset
on activation, and read by nothing else. Those are the cheap ones; a resource
with many readers is a migration, not a move.

⛔⛤ **THE FIRST VERSION OF THIS SCRIPT REPORTED THREE RESET-ONLY TYPES AND ALL
THREE WERE WRONG.** It matched `Res<Short>`/`ResMut<Short>` after stripping each
type to its last path segment — but production writes the PATH:
`ResMut<ambition_cutscene::LastCutsceneRoom>`,
`Res<ambition_cutscene::CutsceneSkipHold>`. So every type whose readers spell it
with a module prefix read as having fewer readers than it has, and the three that
came back ZERO simply had no unqualified reader anywhere.

⇒ **A ZERO FROM THIS KIND OF SCAN IS A CLAIM ABOUT THE QUERY.** The pattern below
allows an optional path prefix and an optional lifetime, and the script prints
its own pattern so a reader can see what it would MISS rather than trusting the
column. ⚠ It still cannot see `world.resource::<T>()`, `SystemState`, or a type
reached through an alias, so this is a FLOOR on readers — never a zero.
"""

from __future__ import annotations

import pathlib
import re
import subprocess
import sys

REPO = pathlib.Path(__file__).resolve().parents[1]
OWNER = "crates/ambition_platformer2d_actor_monolith/src/session/teardown.rs"
BUNDLE = "SessionScopedResources"

FIELD = re.compile(r"^\s{4}(\w+):\s*ResMut<'w,\s*(.+?)>,\s*$", re.M)


def param_pattern(short: str) -> re.Pattern[str]:
    """`Res`/`ResMut` naming this type, with or without a module path."""
    return re.compile(
        rf"\b(?:Res|ResMut)<\s*(?:'\w+\s*,\s*)?(?:[\w:]+::)?{re.escape(short)}\s*>"
    )


#: A direct `world.resource` access, which a system-param scan cannot see.
def direct_pattern(short: str) -> re.Pattern[str]:
    return re.compile(rf"resource(?:_mut|_scope)?::<\s*(?:[\w:]+::)?{re.escape(short)}\s*>")


def main() -> int:
    text = (REPO / OWNER).read_text(encoding="utf-8")
    start = text.index(f"pub struct {BUNDLE}")
    fields = FIELD.findall(text[start : text.index("\n}\n", start)])
    if len(fields) < 3:
        print(f"⛔⛔ parsed {len(fields)} member(s) of {BUNDLE}; the scan is broken")
        return 1

    tracked = subprocess.run(
        ["git", "ls-files", "*.rs"], cwd=REPO, capture_output=True, text=True, check=True
    ).stdout.split()
    blob = {}
    for rel in tracked:
        try:
            blob[rel] = (REPO / rel).read_text(encoding="utf-8", errors="replace")
        except OSError:
            print(f"⛔⛔ could not read {rel}; the corpus is incomplete")
            return 1

    print(f"{BUNDLE} has {len(fields)} members. Readers are a FLOOR, not a count.")
    print("  param pattern : Res|ResMut< [lifetime,] [path::]Type >")
    print("  direct pattern: resource|resource_mut|resource_scope::<[path::]Type>")
    print(f"  {'type':40} {'params':>6} {'direct':>6}  first site outside the owner")
    rows = []
    for _, ty in fields:
        short = ty.rsplit("::", 1)[-1]
        params = [f for f, s in blob.items() if param_pattern(short).search(s) and OWNER not in f]
        direct = [f for f, s in blob.items() if direct_pattern(short).search(s) and OWNER not in f]
        rows.append((short, params, direct))

    for short, params, direct in sorted(rows, key=lambda r: len(r[1]) + len(r[2])):
        first = (params + direct or ["-- none found by EITHER pattern"])[0]
        print(f"  {short:40} {len(params):6} {len(direct):6}  {first}")

    lonely = [s for s, p, d in rows if not p and not d]
    print()
    if lonely:
        print(
            f"⚠ {len(lonely)} member(s) matched NEITHER pattern: {lonely}. That is a "
            "prompt to OPEN them, not a finding — this scan cannot see `SystemState`, "
            "an alias, or a reader in a macro."
        )
    else:
        print(
            "⭐ EVERY member has at least one reader outside its own reset, so there "
            "is no cheap reset-only subset to lift out of this bundle. C03's "
            "session-scoped half is a migration, not a move."
        )
    return 0


if __name__ == "__main__":
    sys.exit(main())
