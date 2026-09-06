#!/usr/bin/env python3
"""Which SETS of messages are read together by more than one system?

⭐ WHY THIS IS AN ARCHITECTURE MEASUREMENT AND NOT A STYLE ONE. A set of messages
that several systems all read together is a FACT those systems each derive: "both
of these happened" is a question, and every reader answering it separately is the
same answer written N times. On 2026-09-05 exactly that shape held a live defect
-- four systems each combined `RoomLoaded` and `RoomReplayAdmitted`, three warned
in comments about a cursor rule, and the fourth had the bug the warnings
described. The union became one type (`FreshAttempt`) and the four now ask it.

⇒ This prints the candidates for the same treatment, most-shared first. It is a
REPORT, not a guard: a repeated pair is a question ("is this one fact?"), not a
verdict. Plenty of pairs are coincidence -- two systems that both happen to care
about damage and death are not a union.

⚠ IT DISAGREES WITH `room_replay_reader_slots.py` ON PURPOSE, and the difference
is the point rather than an inconsistency. That census counts BOTH roads to a
message -- direct `MessageReader<..>` and readers arriving through a
`SystemParam` -- because its question is "who answers a replay", and a bundled
reader still answers it. This scan counts only the HAND-WRITTEN road, because its
question is "who assembles this union themselves", and a bundled one no longer
does. ⇒ Same tree, two questions; a tool that counts spellings must say which
spellings it counts.

⚠ WHAT IT CANNOT SEE, stated so a zero is not misread. It matches
`MessageReader<...>` in a parameter list, so it misses a system that takes a
`SystemParam` bundling readers (which is the FIXED shape -- by construction the
successes disappear from this report), and it misses readers reached through a
type alias. A pair dropping off this list is therefore evidence of a fix, not of
a regression.
"""
from __future__ import annotations

import collections
import pathlib
import re
import subprocess
import sys

REPO = pathlib.Path(__file__).resolve().parent.parent
READER = re.compile(r"MessageReader<\s*(?:'[\w]+\s*,\s*)*([^,>]+)>")


def rust_files() -> list[pathlib.Path]:
    out = subprocess.run(
        ["git", "ls-files", "crates/", "game/"],
        cwd=REPO, capture_output=True, text=True, check=True,
    ).stdout.splitlines()
    return [
        REPO / f for f in out
        if f.endswith(".rs") and "/tests/" not in f and not f.endswith("tests.rs")
    ]


def systems(path: pathlib.Path):
    """Yield (fn name, [message type, ...]) for each fn taking >=1 MessageReader."""
    src = path.read_text(encoding="utf-8", errors="replace").splitlines()
    i = 0
    while i < len(src):
        if not re.match(r"\s*(pub(\([\w:]+\))? )?(async )?fn ", src[i]):
            i += 1
            continue
        name = re.search(r"fn (\w+)", src[i])
        # The parameter list ends at the first line closing it.
        end = i
        while end < min(i + 80, len(src)) and not src[end].rstrip().endswith(") {"):
            end += 1
        params = "\n".join(src[i:end + 1])
        found = [m.strip().split("::")[-1] for m in READER.findall(params)]
        if found:
            yield (name.group(1) if name else "<unnamed>", sorted(set(found)))
        i = end + 1


def main() -> int:
    pairs: dict[tuple[str, ...], list[str]] = collections.defaultdict(list)
    total_systems = 0
    for path in rust_files():
        rel = path.relative_to(REPO)
        for name, msgs in systems(path):
            total_systems += 1
            if len(msgs) < 2:
                continue
            # Every SUBSET of size 2 -- a shared pair inside two different larger
            # signatures is still a shared pair.
            for a in range(len(msgs)):
                for b in range(a + 1, len(msgs)):
                    pairs[(msgs[a], msgs[b])].append(f"{name}  ({rel})")

    if total_systems == 0:
        print("FAIL: no systems with a MessageReader found at all — the scan is "
              "broken, not the tree.", file=sys.stderr)
        return 1

    shared = {k: v for k, v in pairs.items() if len(v) > 1}
    print(f"{total_systems} system(s) take a `MessageReader`; "
          f"{len(pairs)} distinct message PAIR(s); "
          f"{len(shared)} pair(s) read together by more than one system.\n")
    for pair, sites in sorted(shared.items(), key=lambda kv: -len(kv[1])):
        print(f"  {len(sites):>2}x  {pair[0]} + {pair[1]}")
        for site in sites:
            print(f"        {site}")
    if not shared:
        print("  (none — every multi-message system reads a unique combination)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
