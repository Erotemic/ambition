#!/usr/bin/env python3
"""The union "a fresh attempt begins" is spelled ONE way: `FreshAttempt`.

⛔⛔ THE BUG THIS EXISTS FOR, found 2026-09-05. Sanic's `SpentMonitors` was
rearmed on `RoomLoaded` only. Sanic declares `DeathRules::replay_level_after(0.0)`,
so a pit death REPLAYS the room in place -- and an in-place replay never emits
`RoomLoaded`, which is written from exactly one place, an actual room load. ⇒ A
monitor broken before the death stayed broken after the respawn and its grant was
unreachable for the rest of the run.

⭐⭐ THE FIX IS NOW STRUCTURAL, AND THIS GUARD CHANGED SUBJECT TO MATCH. Until
2026-09-05 this file policed a HAND-KEPT PAIRING: four systems each declared two
`MessageReader`s and each re-solved the same cursor rule in its own words. That
is the union of a fact derived in four places, so it moved into one type --
`ambition_combat::events::FreshAttempt` -- and the four call sites now ask it a
question instead of assembling the answer.
⇒ What is left to check is not "did you remember both messages" (the type holds
both) but "did anyone go back to assembling it by hand".

⛔⛔ THE CURSOR RULE THAT MAKES IT A TYPE. Both readers must be drained EVERY
frame, unconditionally. The natural spelling short-circuits:

    let crossed = replays.read().count() > 0 || loads.read().count() > 0;  # ⛔

and a `MessageReader` whose cursor does not advance re-reads that message on a
later frame. Three of the four call sites carried a comment warning about exactly
this and the fourth (`void_pending_player_hits_at_lifecycle_boundaries`) was
written with the short-circuit anyway -- prose could not stop it, so the shape
does: `FreshAttempt` binds both counts to locals before the `||`.

⚠ NOT every replay reader wants the union. Of the eleven `RoomReplayAdmitted`
readers, seven deliberately do NOT answer a load -- retracting a boss defeat or a
gravity override on an ordinary room entry would undo progress the player kept.
This guard is about systems that want BOTH, and says nothing about the other
seven.

⭐ THE ANTI-VACUITY FLOOR IS A PRESENCE. The old floor counted the population
being policed, which this change legitimately emptied; counting it now would fail
forever. The floor here counts ADOPTERS of `FreshAttempt`, so deleting the
abstraction (or renaming it out from under this file) reddens instead of quietly
certifying nothing.
"""
from __future__ import annotations

import pathlib
import re
import subprocess
import sys

REPO = pathlib.Path(__file__).resolve().parent.parent
LOAD = "RoomLoaded"
REPLAY = "RoomReplayAdmitted"
UNION = "FreshAttempt"
ESCAPE = "ALLOW_LOAD_ONLY"
#: ⭐ Four call sites adopted the type on 2026-09-05 (bricks, power blocks,
#: monitors, pending player hits). A drop below this means the abstraction is
#: being abandoned, which is the thing worth hearing about.
MIN_ADOPTERS = 4


def git_grep(pattern: str, *paths: str) -> list[str]:
    # ⚠ Flags BEFORE the pattern: `git grep <pat> -n` reads `-n` as a revision.
    proc = subprocess.run(
        ["git", "grep", "-n", pattern, "--", *paths],
        cwd=REPO, capture_output=True, text=True,
    )
    if proc.returncode not in (0, 1):
        raise SystemExit(f"git grep failed: {proc.stderr.strip()}")
    return proc.stdout.splitlines()


def production(lines: list[str]) -> list[str]:
    return [
        line for line in lines
        if "/tests/" not in line and not line.split(":")[0].endswith("tests.rs")
    ]


def enclosing_system(path: pathlib.Path, lineno: int) -> tuple[str, str, str] | None:
    """(name, doc block, PARAMETER list) of the fn whose parameters hold `lineno`.

    ⛔⛔ THE DOC AND THE PARAMETERS ARE RETURNED SEPARATELY, AND THAT IS THE
    POINT. The first cut returned them concatenated, so `REPLAY in sig` was true
    when the message was merely NAMED IN A COMMENT -- a system with the exact
    defect shape passed because its doc block mentioned the message it failed to
    read. Found by poisoning: a planted load-only system was reported by the
    wrong arm, because the doc above it belonged to its neighbour.
    ⇒ A message check asks the PARAMETERS. Only the escape hatch reads prose,
    which is what it is for.
    """
    src = path.read_text(encoding="utf-8").splitlines()
    start = next(
        (i for i in range(lineno - 1, max(lineno - 60, -1), -1)
         if re.match(r"\s*(pub )?fn ", src[i])),
        None,
    )
    if start is None:
        return None
    end = next(
        (i for i in range(start, min(start + 80, len(src)))
         if src[i].rstrip().endswith(") {")),
        min(start + 60, len(src) - 1),
    )
    # Include the doc block above the signature so the escape hatch can be
    # written where a reader will actually see it.
    doc = start
    while doc > 0 and src[doc - 1].lstrip().startswith(("///", "//", "#[")):
        doc -= 1
    name = re.search(r"fn (\w+)", src[start])
    return (
        name.group(1) if name else "<unnamed>",
        "\n".join(src[doc:start]),      # prose only
        "\n".join(src[start:end + 1]),  # the signature and its parameters
    )


def main() -> int:
    # ⭐ THE FLOOR FIRST: everything below is a claim about a corpus, and an
    # empty corpus would make all of it trivially true.
    adopters = production(git_grep(f"{UNION}", "crates/", "game/"))
    # The definition itself lives in ambition_combat; call sites are the rest.
    # ⚠ A COMMENT MENTIONING THE TYPE IS NOT AN ADOPTER. The first cut of this
    # floor counted raw grep hits and read 5 for 4 systems, because one call site
    # explains itself in a comment that names the type -- so a real adopter could
    # have been removed with the floor still green. Count PARAMETER declarations.
    call_sites = [
        line for line in adopters
        if "ambition_combat/src/events.rs" not in line.split(":")[0]
        and not line.split(":", 2)[2].lstrip().startswith("//")
        and re.search(rf":\s*[\w:]*{UNION}\s*[,<]", line.split(":", 2)[2])
    ]
    if len(call_sites) < MIN_ADOPTERS:
        print(
            f"FAIL: only {len(call_sites)} call site(s) of `{UNION}`, expected at "
            f"least {MIN_ADOPTERS}.\n"
            "  Either the abstraction is being abandoned, or it was renamed and "
            "this guard now\n  certifies an empty set. Both are worth a look.",
            file=sys.stderr,
        )
        return 1

    # (1) Nobody assembles the union by hand any more.
    by_hand = []
    for line in production(git_grep(f"MessageReader<.*{LOAD}", "crates/", "game/")):
        path_s, lineno_s = line.split(":")[0], line.split(":")[1]
        found = enclosing_system(REPO / path_s, int(lineno_s))
        if found is None:
            continue
        name, _doc, params = found
        if REPLAY in params:
            by_hand.append(f"{name}  ({path_s}:{lineno_s})")

    # (2) The ORIGINAL defect shape, still reachable by a freshly written system:
    #     clears state on a load and never answers a replay.
    checked, violations, exempt = 0, [], []
    for line in production(git_grep(f"MessageReader<.*{LOAD}", "crates/", "game/")):
        path_s, lineno_s = line.split(":")[0], line.split(":")[1]
        found = enclosing_system(REPO / path_s, int(lineno_s))
        if found is None:
            continue
        name, doc, params = found
        if "ResMut<" not in params:
            continue  # reads the load but changes nothing; nothing to retract.
        checked += 1
        if ESCAPE in doc or ESCAPE in params:
            exempt.append(f"{name}  ({path_s})")
        elif REPLAY not in params and UNION not in params:
            violations.append(f"{name}  ({path_s}:{lineno_s})")

    for name in exempt:
        print(f"  exempt ({ESCAPE}): {name}")

    if by_hand:
        print(
            f"FAIL: {len(by_hand)} system(s) declare BOTH `{LOAD}` and `{REPLAY}` "
            f"readers by hand:", file=sys.stderr,
        )
        for name in by_hand:
            print(f"  {name}", file=sys.stderr)
        print(
            f"\n  That union has a type: `ambition_combat::events::{UNION}`. Take "
            f"it as a\n  parameter and ask `began()` (or `began_in(room)`). Doing "
            "it by hand means\n  re-solving the cursor rule, which is how the "
            "short-circuit bug got written.",
            file=sys.stderr,
        )
        return 1

    if violations:
        print(
            f"FAIL: {len(violations)} system(s) clear state on a room LOAD but do "
            f"not answer an admitted room REPLAY:", file=sys.stderr,
        )
        for name in violations:
            print(f"  {name}", file=sys.stderr)
        print(
            f"\n  An in-place replay (a death, a retry) never emits `{LOAD}`, so "
            "per-attempt\n  state cleared only on load survives the respawn. Take "
            f"`{UNION}` instead, or\n  mark the system `{ESCAPE}` with a reason if "
            "its state really is load-scoped\n  rather than attempt-scoped.",
            file=sys.stderr,
        )
        return 1

    print(
        f"ok: {len(call_sites)} `{UNION}` call site(s); no system assembles the "
        f"load/replay union by hand; {checked} remaining `{LOAD}`+`ResMut` "
        f"system(s) all answer a replay."
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
