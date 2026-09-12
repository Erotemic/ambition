#!/usr/bin/env python3
"""Refuse an LDtk world whose `nextUid` counter has run ahead of its contents.

⛔⛤ **THE CHURN THIS EXISTS FOR: FOUR AUTHORED WORLDS SAT DIRTY IN A SHARED TREE
WITH A ONE-LINE DIFF EACH, `nextUid` BUMPED BY EXACTLY ONE AND NOTHING ELSE
CHANGED.** An LDtk project counts uids monotonically, and `alloc_uid` /
`allocate_iid` bump the counter the moment they are CALLED. A command that
allocates and then discovers the thing it was building already exists — an
`ensure_*` that checks after it allocates rather than before — spends a uid on a
no-op and rewrites an authored file to say so.

⭐⭐ **AND THE INVARIANT IS ALREADY TRUE, WHICH IS WHY IT IS WORTH ENFORCING.**
MEASURED 2026-09-12 across every tracked `.ldtk`: the two worlds nobody had run a
tool over sat at `nextUid == max_in_use + 1` exactly, and the four dirty ones sat
one above it. The leak is not a tolerated condition to ratchet down; it is a
deviation from what the corpus already satisfies.

⛔ **WHY THIS AND NOT AN AUDIT OF THE ALLOCATORS.** There are twenty-odd
`alloc_uid` / `allocate_iid` call sites. A previous fix (`54d99e7fb`, "make LDtk
sprite regeneration idempotent") corrected the two that were then known, by
reusing existing uids instead of spending new ones — and the churn came back from
somewhere else. A hand-enumerated list of allocation sites is a POPULATION, and a
population rots the moment somebody adds a twenty-first. This checks the
PROPERTY, so it cannot miss a site it has never heard of.

⚠ **A DELETION LEGITIMATELY RAISES THE COUNTER ABOVE THE CONTENTS, AND THAT IS
WHY `--fix` EXISTS RATHER THAN A BARE REFUSAL.** Removing the highest-uid entity
leaves `nextUid` pointing past the new maximum, which is not a leaked allocation
and not a defect. The repair is the same either way — bring the counter back to
`max_in_use + 1` — and it is only safe when the reclaimed values appear NOWHERE
ELSE, which `--fix` verifies against the whole tracked tree before touching a
file. A uid that was allocated and discarded was never written, so nothing can
remember it; a uid freed by a deletion may still be named by a sibling file, and
this refuses to reclaim that one.
"""

from __future__ import annotations

import argparse
import json
import pathlib
import subprocess
import sys

# ⛔ THE INVARIANT IS DEFINED ONCE, IN THE LIBRARY THAT WRITES THESE FILES.
# `ldtk.io.warn_if_uid_leaked` says so at the moment a leak is created and this
# says so at the gate; two spellings of "which integers has this project spent"
# would drift, and the one that drifted would be the one nobody ran.
sys.path.insert(0, str(pathlib.Path(__file__).resolve().parents[1] / "tools" / "ambition_ldtk_tools"))
from ambition_ldtk_tools.ldtk.io import uids_in_use  # noqa: E402


def worlds(roots: list[pathlib.Path]) -> list[pathlib.Path]:
    out: list[pathlib.Path] = []
    for root in roots:
        out.extend(sorted(root.rglob("*.ldtk")) if root.is_dir() else [root])
    return out


def leak_of(path: pathlib.Path) -> tuple[int, int, int]:
    """`(nextUid, max_in_use, leak)` for one world."""
    text = path.read_text()
    project = json.loads(text)
    counter = int(project.get("nextUid", 1))
    highest = max(uids_in_use(project, text), default=0)
    return counter, highest, counter - highest - 1


#: A leak wider than this is not an allocate-and-discard — it is a counter
#: somebody MOVED. `world_init.py` offsets a new world's `nextUid` by 100,000 so
#: iids cannot collide on merge, and winding that back would undo it on purpose.
#: Refusing is the right answer, not a slower scan.
MAX_RECLAIMABLE = 512


def unreferenced(values: range) -> list[int]:
    """Which of `values` no tracked file mentions at all.

    ⛔⛤ **ONE `git grep` FOR THE WHOLE RANGE, BECAUSE ONE PER VALUE DOES NOT
    TERMINATE.** The first version looped `git grep` per candidate — fine for the
    leak this was written for (one uid) and non-terminating for ~100,000. Found
    by POISONING `--fix` with a counter moved into another world's range, which
    is exactly the `world_init.py` offset shape and therefore not hypothetical.
    ⇒ The poison that was meant to exercise the refusal branch found a defect in
    the branch it was aiming at.
    """
    if not values or len(values) > MAX_RECLAIMABLE:
        return []
    pattern = "|".join(str(v) for v in values)
    found = subprocess.run(
        ["git", "grep", "-hoE", pattern], capture_output=True, text=True
    )
    seen = {int(line) for line in found.stdout.split("\n") if line.strip().isdigit()}
    return [v for v in values if v not in seen]


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument(
        "roots",
        nargs="*",
        type=pathlib.Path,
        default=[pathlib.Path("game/ambition_map_assets")],
        help="Directories or .ldtk files to check (default: the map submodule).",
    )
    parser.add_argument(
        "--fix",
        action="store_true",
        help=(
            "Reclaim the leaked range by lowering `nextUid` to max_in_use + 1, "
            "but ONLY for values no tracked file mentions."
        ),
    )
    args = parser.parse_args(argv)

    files = worlds(args.roots)
    if not files:
        print(f"no .ldtk world under {[str(r) for r in args.roots]}", file=sys.stderr)
        # ⛔ AN EMPTY CORPUS IS A FAILURE, NOT A PASS. This check's whole subject
        # is a set of files; finding none means it was aimed wrongly, and
        # printing `ok` would be the most common way a guard in this repository
        # certifies nothing.
        return 2

    leaks = []
    for path in files:
        counter, highest, leak = leak_of(path)
        status = "ok" if leak == 0 else f"LEAK {leak}"
        print(f"  {path}: nextUid={counter} max_in_use={highest} {status}")
        if leak > 0:
            leaks.append((path, counter, highest, leak))

    if not leaks:
        print(f"{len(files)} world(s) checked; every counter sits exactly one above its contents.")
        return 0

    if not args.fix:
        print()
        for path, counter, highest, leak in leaks:
            print(
                f"⛔ {path}: `nextUid` is {counter} but nothing in the project uses "
                f"more than {highest} — {leak} uid(s) were allocated and never "
                f"written. A command that allocates BEFORE it checks whether the "
                f"thing already exists rewrites an authored world to record a "
                f"no-op. Re-run with --fix to reclaim them."
            )
        return 1

    for path, counter, highest, leak in leaks:
        candidates = range(highest + 1, counter)
        safe = unreferenced(candidates)
        if len(candidates) > MAX_RECLAIMABLE:
            print(
                f"⚠ {path}: refusing to reclaim {len(candidates)} uid(s) — a leak "
                f"that wide is a counter somebody MOVED, not one an allocation "
                f"leaked. `world_init.py` offsets a new world's `nextUid` by "
                f"100,000 so iids cannot collide on merge, and winding that back "
                f"would undo it. Repair by hand if this is genuinely a leak."
            )
            continue
        if len(safe) != len(candidates):
            held = sorted(set(candidates) - set(safe))[:8]
            print(
                f"⚠ {path}: refusing to reclaim {held} — a tracked file still "
                f"names them, so these were FREED BY A DELETION rather than "
                f"leaked by an allocation, and lowering the counter could mint a "
                f"name something still remembers."
            )
            continue
        project = json.loads(path.read_text())
        project["nextUid"] = highest + 1
        from ambition_ldtk_tools.ldtk.io import write_project

        write_project(path, project)
        print(f"✔ {path}: nextUid {counter} → {highest + 1} ({leak} reclaimed)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
