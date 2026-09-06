#!/usr/bin/env python3
"""Who reacts to an ADMITTED room replay, and in which scheduling slot?

⭐ THE QUESTION. `ContentRoomReplayResetSet` is a hand-kept list: a content
system that holds per-attempt state must remember to register there, and the host
anchors that slot BEFORE its generic replay consumer. A hand-kept list invites a
guard, and the obvious guard is "every content reader of `RoomReplayAdmitted`
must be in that set".

⛔⛔ THAT GUARD WOULD BE WRONG, WHICH IS WHY THIS SCRIPT EXISTS RATHER THAN THE
GUARD. Measured 2026-09-05: content readers legitimately live in FOUR different
slots, each chosen for a reason stated at the registration site --
`ContentRoomReplayResetSet` (per-attempt state the replay invalidates),
`ContentRoomResetSet` (room re-entry restore), a mechanic's own slot
(`PortalSet::RoomReset`), and ordinary simulation phases (a detector clearing its
own local state). "Not in the replay set" is therefore not evidence of anything.

⇒ This reports the POPULATION and its spread so the claim stays true, and does
NOT fail on membership. The number to watch is the total: a new reader is a
system somebody had to place by judgement, and the judgement is recorded only in
prose beside the registration.
"""
from __future__ import annotations

import pathlib
import re
import subprocess
import sys

REPO = pathlib.Path(__file__).resolve().parent.parent
MESSAGE = "RoomReplayAdmitted"
#: ⛔⛔ A READER MAY ARRIVE THROUGH A `SystemParam`, AND THIS CENSUS WENT BLIND TO
#: FOUR OF THEM. On 2026-09-05 the four systems that read `RoomLoaded` AND
#: `RoomReplayAdmitted` by hand were replaced by one
#: `ambition_combat::events::FreshAttempt` parameter that holds both readers. The
#: behaviour and the set membership did not change; this script's count fell from
#: 11 to 8, purely because it was looking for a spelling.
#: ⇒ A census of "who reads X" must count every ROAD to X, or a refactor that
#: bundles readers reads as a retraction.
#: ⚠ `scripts/message_reader_pairs.py` deliberately does the OPPOSITE and both are
#: right: its question is "who assembles this union by hand", so a union that
#: became a `SystemParam` SHOULD vanish from it. The question decides the roads,
#: not a house style.
UNION_PARAM = "FreshAttempt"
#: Slots a reader may legitimately sit in. Not a whitelist to enforce -- a list
#: to REPORT against, so an unfamiliar slot is visible rather than silently fine.
KNOWN_SLOTS = [
    "ContentRoomReplayResetSet",
    "ContentRoomResetSet",
    "PortalSet::RoomReset",
]


def git_grep(pattern: str, *paths: str) -> list[str]:
    """⚠ Flags BEFORE the pattern: `git grep <pat> -n` reads `-n` as a revision
    and dies with 'unable to resolve revision', which a `returncode not in (0,1)`
    guard then swallows into a confident zero."""
    proc = subprocess.run(
        ["git", "grep", "-n", pattern, "--", *paths],
        cwd=REPO, capture_output=True, text=True,
    )
    if proc.returncode not in (0, 1):
        raise SystemExit(f"git grep failed: {proc.stderr.strip()}")
    return proc.stdout.splitlines()


def main() -> int:
    def production(lines):
        return [
            line for line in lines
            if "/tests/" not in line and not line.split(":")[0].endswith("tests.rs")
        ]

    # ⚠ THE UNION'S DEFINITION IS NOT A REACTING SYSTEM. `FreshAttempt` holds a
    # `MessageReader<RoomReplayAdmitted>` in its struct, so it matches the direct
    # pattern -- but it reacts to nothing; its CALLERS do. Counting it inflates
    # the population by one and, worse, double-counts the same reader that the
    # union road already reports.
    direct = [
        line for line in production(
            git_grep(f"MessageReader<.*{MESSAGE}", "crates/", "game/")
        )
        if "ambition_combat/src/events.rs" not in line.split(":", 1)[0]
    ]
    # The indirect road: a parameter of the union type. Its DEFINITION holds a
    # `MessageReader<..>` and is already counted above, so exclude the file that
    # defines it to avoid double-counting the same reader.
    via_union = [
        line for line in production(git_grep(f": .*{UNION_PARAM}", "crates/", "game/"))
        if "ambition_combat/src/events.rs" not in line.split(":", 1)[0]
    ]
    readers = direct + via_union
    # ⛔⛔ EACH ROAD NEEDS ITS OWN FLOOR, and asking only whether the TOTAL is
    # non-empty does not give it one. The union road matches a PARAMETER TYPE and
    # says nothing about `MESSAGE`, so a renamed message left the total non-empty
    # and the vacuity check green -- the exact hole this check exists to close,
    # opened by teaching it a second road. The wrapper's poison caught it.
    if not direct:
        print(f"FAIL: no DIRECT `MessageReader<..{MESSAGE}>` anywhere -- that "
              "name has gone stale, and every count below is about a message "
              "nothing reads.", file=sys.stderr)
        return 1
    if not via_union:
        print(f"FAIL: no parameter of type `{UNION_PARAM}` anywhere -- the union "
              "was renamed or retired, so the indirect road silently contributes "
              "nothing and the census under-reports.", file=sys.stderr)
        return 1

    engine, content = [], []
    for line in readers:
        path = line.split(":", 1)[0]
        (content if path.startswith("game/") else engine).append(path)

    print(f"readers of {MESSAGE}: {len(readers)}  "
          f"({len(engine)} engine, {len(content)} content)")
    print(f"  by road: {len(direct)} direct `MessageReader<..>`, "
          f"{len(via_union)} through `{UNION_PARAM}`")

    members = [
        line for line in git_grep("ContentRoomReplayResetSet", "crates/", "game/")
        if ".in_set(" in line and "/tests/" not in line
    ]
    # ⚠ SITES, NOT SYSTEMS, and the difference is real: `mary_o` registers a
    # TUPLE of two systems at one `.in_set(..)`. Counting sites and calling them
    # systems undercounts the membership -- 2 sites carry 3 systems today.
    print(f"registration SITES into ContentRoomReplayResetSet: {len(members)} "
          "(a site may register a TUPLE of systems)")
    for line in members:
        print(f"  {line.split(':')[0]}:{line.split(':')[1]}")

    print("\nslots named anywhere in the tree:")
    for slot in KNOWN_SLOTS:
        hits = [l for l in git_grep(slot, "crates/", "game/") if ".in_set(" in l
                or "PortalSet::RoomReset" in slot]
        print(f"  {slot}: {len(hits)} mention(s)")

    print("\n⇒ Reported, not enforced: a reader outside the replay set is not a "
          "finding. See this file's docstring for why.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
