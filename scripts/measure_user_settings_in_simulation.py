#!/usr/bin/env python3
"""Which `UserSettings` fields does DETERMINISTIC SIMULATION read?

The 2026-09-13 architecture review's priority 1: `UserSettings` is waived in
`rollback_coverage.rs` as *"user settings, forward-only"* — the category `Q119`
already ruled is not one — while the settings menu mutates it at runtime and
simulation systems read it. This inventories the readers so the split can be
decided from evidence rather than from the type name.

⛔⛤ **THE KEY IS A TYPE, NOT A FIELD NAME, AND THAT IS THE WHOLE DIFFERENCE FROM
`scripts/measure_identity_field_consumers.py`.** That script tried to classify
`Q122`'s fields by grepping `.field` and answered *"50 of 50 mechanical"*, because
`.id` and `.body` are spelled the same way in a hundred structs. `UserSettings`
is one named type, so `Res<…UserSettings>` finds its readers and nothing else —
and each hit is reported with its file so the attribution can be checked rather
than trusted.

⚠ **SCHEDULE ATTRIBUTION IS THE HALF THAT CAN BE WRONG, AND IT SAYS SO.** A
reader matters only if it runs in the SIMULATION schedule — under the rollback
host that is `GgrsSchedule`, where a resimulation of confirmed frames reads it.
This script decides that by looking for the reader's name in an `add_systems`
call whose schedule argument is `sim` / `sim_schedule()`. ⇒ A system registered
through an intermediate or a set this scan cannot follow is reported as
`UNATTRIBUTED`, never as safe.

    python3 scripts/measure_user_settings_in_simulation.py
"""

from __future__ import annotations

import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

# The reader signature, in every qualification it is written in.
READER = re.compile(r"Res<\s*(?:[A-Za-z0-9_]+::)*UserSettings\s*>")
# `fn name(` at any indentation, for attributing a hit to its enclosing function.
FN = re.compile(r"^\s*(?:pub(?:\([^)]*\))?\s+)?(?:async\s+)?fn\s+([a-z_][a-z0-9_]*)\s*[(<]")


def enclosing_fn(lines: list[str], index: int) -> str | None:
    """The `fn` a line belongs to.

    ⛔ SCANS UPWARD AND ACCEPTS `pub(crate) fn`. A previous census in this
    repository credited two systems to helper functions defined earlier in the
    file because its regex was `(pub )?fn`, and the wrong answers were plausible
    NAMES so nothing looked broken.
    """
    for i in range(index, -1, -1):
        match = FN.match(lines[i])
        if match:
            return match.group(1)
    return None


def readers() -> dict[str, set[str]]:
    """Function name -> the files it is declared in."""
    out: dict[str, set[str]] = {}
    listing = subprocess.run(
        ["git", "grep", "-lE", r"Res<\s*([A-Za-z0-9_]+::)*UserSettings\s*>"],
        cwd=ROOT,
        capture_output=True,
        text=True,
    ).stdout.split()
    for path in listing:
        if not path.endswith(".rs"):
            continue
        if "/tests/" in path or path.endswith("tests.rs") or path.endswith("_tests.rs"):
            continue
        lines = (ROOT / path).read_text().splitlines()
        for i, line in enumerate(lines):
            if not READER.search(line):
                continue
            name = enclosing_fn(lines, i)
            if name:
                out.setdefault(name, set()).add(path)
    return out


def sim_registered() -> set[str]:
    """Every system name registered into a SIMULATION schedule.

    ⚠ Matches `add_systems(sim, …)` and `add_systems(app.sim_schedule(), …)`
    blocks and collects the identifiers inside. Deliberately generous: a name
    that appears in such a block is reported as simulation-scheduled, and a
    FALSE POSITIVE here is visible (the reader is named and can be checked)
    while a false negative would be silence.
    """
    names: set[str] = set()
    listing = subprocess.run(
        ["git", "grep", "-l", "add_systems"],
        cwd=ROOT,
        capture_output=True,
        text=True,
    ).stdout.split()
    for path in listing:
        if not path.endswith(".rs"):
            continue
        text = (ROOT / path).read_text()
        for match in re.finditer(
            r"add_systems\(\s*(sim|sim_schedule|app\.sim_schedule\(\)|"
            r"[A-Za-z_:]*GgrsSchedule)\s*,",
            text,
        ):
            # ⚠ THE WINDOW IS THE WHOLE CALL, not a fixed byte count. A truncating
            # window silently drops the tail of a long chain — and a long chain is
            # exactly where the systems this census is about live. MEASURED: at
            # 2000 bytes it missed `apply_feature_hit_events`, which the review
            # names by hand.
            tail = text[match.end() :]
            depth = 0
            chunk = []
            for ch in tail:
                if ch == "(":
                    depth += 1
                elif ch == ")":
                    if depth == 0:
                        break
                    depth -= 1
                chunk.append(ch)
            names.update(re.findall(r"\b([a-z_][a-z0-9_]{4,})\b", "".join(chunk)))
    return names


def main() -> int:
    found = readers()
    if not found:
        print("⛔ NO `Res<UserSettings>` READER FOUND AT ALL.")
        print("   That is an INSTRUMENT failure, not a clean bill of health — the")
        print("   type was renamed or re-exported and this scan is about nothing.")
        return 1
    sim = sim_registered()
    if not sim:
        print("⛔ NO SIMULATION-SCHEDULED SYSTEM FOUND AT ALL — the attribution half")
        print("   of this instrument is broken, so every row below would read SAFE.")
        return 1

    in_sim = sorted(name for name in found if name in sim)
    elsewhere = sorted(name for name in found if name not in sim)

    print(f"{len(found)} production function(s) take `Res<UserSettings>`.\n")
    print(f"⛔ READ INSIDE THE SIMULATION SCHEDULE ({len(in_sim)}) — under the rollback")
    print("   host this is `GgrsSchedule`, so a replay of frame N reads whatever the")
    print("   settings menu says NOW:")
    for name in in_sim:
        for path in sorted(found[name]):
            print(f"     {name:<44} {path}")
    print(f"\n⚠ UNATTRIBUTED TO A SIMULATION SCHEDULE ({len(elsewhere)}) — this scan did")
    print("   not find them in an `add_systems(sim, …)` block. That is NOT a clearance:")
    print("   a system registered through an intermediate, a helper, or a set this scan")
    print("   cannot follow lands here too. Check each before treating it as safe:")
    for name in elsewhere:
        for path in sorted(found[name]):
            print(f"     {name:<44} {path}")
    print(
        "\n⇒ The classification each SIMULATION reader needs (review, 2026-09-13):\n"
        "   · LOCAL INPUT INTERPRETATION (movement/aim/camera frame) — resolve at the\n"
        "     INPUT-CAPTURE boundary so deterministic simulation consumes semantic\n"
        "     intent; peers must not have to share accessibility settings.\n"
        "   · GAME-MECHANICAL POLICY (difficulty, assist, damage scaling) — needs a\n"
        "     deterministic session/match projection. Whether that is match-wide or\n"
        "     per-participant is Jon's product call; neither answer permits a read of\n"
        "     an App-local persisted resource during historical simulation.\n"
        "   · PRESENTATION — stays ordinary mutable `UserSettings`."
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
