#!/usr/bin/env python3
"""Warnings a crate emits when built ALONE, which the workspace gate cannot see.

⛔⛔ `check_no_warnings.py` MAKES A WORKSPACE-UNIFIED CLAIM. It builds
`--workspace --all-targets`, where cargo unions every crate's features, so a
crate whose callers sit behind a feature another crate turns on compiles clean.
Built by itself — which is what a downstream consumer does, and what
`cargo test -p <crate>` does, the command this project reaches for constantly —
the same crate can warn.

⭐ That script already prints the OTHER half of this caveat: *"Code behind a
NON-DEFAULT `#[cfg(feature = ...)]` is not compiled by this run and is not
covered by that OK."* This is its mirror, and nothing was measuring it.

Measured 2026-09-04 across the workspace crates that declare a non-default
feature (only those can differ) — ⚠ the counts below are DATED, not live; run
the script rather than quoting them, which is the mistake `authored_route_gates.py`
made three times in one day:

    4  ambition_dialog
    3  ambition_sim_view
    3  ambition_render
    3  ambition_conversation
    2  ambition_platformer2d_actor_monolith
    1  ambition_game_shell
    1  ambition_content
    ────
   17  warning OCCURRENCES across 7 of 43 crates; 36 clean
    6  DISTINCT sites — see below

⛔⛔ AND THE PER-CRATE COUNT DOUBLE-COUNTS A DEPENDENCY'S WARNINGS, which is
worth knowing before quoting the total. `ambition_conversation`'s three are not
its own — they are `ambition_dialog`'s, recompiled by every dependent that
builds it without `input`. By SITE the 17 is six:

    crates/ambition_dialog/src/runtime.rs:363, :409, :512, :566   (the `input` four)
    crates/ambition_game_shell/src/session.rs:147                 stub_live
    game/ambition_content/src/portal/input_adapter.rs:114         presented_subject

⇒ A per-crate count of a warning in a DEPENDENCY counts DEPENDENTS, not
defects. This script reports per crate because that is what a consumer
experiences; group by the `-->` line when you want the work.

⚠ NOT ALL OF THESE ARE DEFECTS, and `ambition_dialog` is the worked example:
its four are `pub(crate)` view-model methods whose only callers are behind the
`input` feature, and the manifest says of `input` that *"without it the input
systems compile as no-op stubs"*. A stub that does not call them is the feature
doing what it says. ⇒ Gating the methods to match would break the crate's own
tests, which call them under default features. **A decision, not a deletion.**

⇒ So read this as a CENSUS. The number that matters is the delta: a crate that
starts warning alone has usually had a caller move behind a feature, and that is
worth knowing before a consumer discovers it.
"""

from __future__ import annotations

import re
import subprocess
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
FEATURE_LINE = re.compile(r"^([a-z_][a-z0-9_-]*)\s*=", re.M)


def crates_with_a_non_default_feature() -> list[str]:
    found: list[str] = []
    for manifest in sorted(REPO.glob("crates/*/Cargo.toml")) + sorted(
        REPO.glob("game/*/Cargo.toml")
    ):
        text = manifest.read_text(encoding="utf-8", errors="replace")
        block = text.split("[features]", 1)
        if len(block) < 2:
            continue
        section = block[1].split("\n[", 1)[0]
        names = [n for n in FEATURE_LINE.findall(section) if n != "default"]
        if names:
            found.append(manifest.parent.name)
    return found


def warnings_for(crate: str) -> int:
    done = subprocess.run(
        ["cargo", "check", "-q", "-p", crate, "--all-targets"],
        cwd=REPO,
        capture_output=True,
        text=True,
    )
    return sum(1 for line in done.stderr.splitlines() if line.startswith("warning:"))


def main() -> int:
    crates = crates_with_a_non_default_feature()
    rows = [(warnings_for(c), c) for c in crates]
    noisy = sorted((n, c) for n, c in rows if n)
    for count, crate in reversed(noisy):
        print(f"  {count:>3}  {crate}")
    total = sum(n for n, _ in rows)
    print(
        f"\n{total} warning(s) across {len(noisy)} of {len(rows)} crate(s) that "
        f"declare a non-default feature; {len(rows) - len(noisy)} clean."
    )
    print(
        "⚠ A CENSUS, NOT A DEFECT LIST — the workspace gate cannot see any of "
        "these, and some are a feature doing what it says. Read the DELTA."
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
