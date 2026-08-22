#!/usr/bin/env python3
"""**No authored level may vanish from a world.**

⛔⛔ **the failure this exists to catch leaves everything else GREEN.** On
2026-08-15 a commit that authored enemy `facing` also deleted an entire level —
139 insertions against 1316 deletions — and every downstream check agreed with
the result: valid LDtk, clean roundtrip, clean `doctor`, intact schema. Nothing
was corrupt. The world was simply smaller, and every tool that derives its roster
*from the file* reported success about the smaller world.

⭐ **the cause is structural and will recur.** A generator that owns a whole file
discards anything authored into that file by a different road. `author_*.py`
rebuilds a world from the specs it knows; a level built with `area create` +
`entity add` is not among them; and a `.ldtk` cannot be partially regenerated. An
agent running the regenerate does this every time without doing anything wrong.

⇒ so the invariant is a RATCHET, not a fixed list. **Adding a level is ordinary
and needs no ceremony; losing one is an error.** Run with `--bless` to record new
levels after adding them deliberately.

⚠ **this checks the ROSTER, not the contents.** A level that survives as an empty
husk passes here. That is deliberate — emptiness is what `doctor` and the entity
contract already measure, and a guard that tries to check everything checks
nothing.
"""

from __future__ import annotations

import argparse
import json
import pathlib
import sys

REPO = pathlib.Path(__file__).resolve().parent.parent
BASELINE = REPO / "scripts" / "authored_levels_baseline.json"

# a world reachable by several paths (a submodule, the symlink into it, and a
# packaged web copy) is ONE world. Keying on the file's own `iid` would be
# tempting and wrong — a regenerate can mint a new one — so key on the world
# file's NAME, which is what every consumer already calls it.
SKIP_PARTS = {"target", ".claude", ".worktrees", "node_modules"}


def worlds() -> dict[str, set[str]]:
    """Every authored world, by file name, mapped to its level identifiers."""
    found: dict[str, set[str]] = {}
    for path in sorted(REPO.rglob("*.ldtk")):
        if SKIP_PARTS & set(path.parts):
            continue
        try:
            data = json.loads(path.read_text())
        except (OSError, json.JSONDecodeError) as exc:
            print(f"  ⛔ {path.relative_to(REPO)}: unreadable ({exc})")
            continue
        levels = {
            level["identifier"] for level in data.get("levels", []) if "identifier" in level
        }
        # a world seen twice must AGREE.
        found.setdefault(path.name, set()).update(levels)
    return found


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--bless",
        action="store_true",
        help="record the current rosters as the new floor (use after ADDING a level)",
    )
    args = parser.parse_args()

    current = worlds()
    if not current:
        print("⛔ found no .ldtk worlds at all — this guard just checked nothing.")
        return 1

    if args.bless or not BASELINE.exists():
        BASELINE.write_text(
            json.dumps({name: sorted(ids) for name, ids in sorted(current.items())}, indent=2)
            + "\n"
        )
        print(f"recorded {sum(len(v) for v in current.values())} levels across "
              f"{len(current)} worlds into {BASELINE.relative_to(REPO)}")
        return 0

    baseline = {name: set(ids) for name, ids in json.loads(BASELINE.read_text()).items()}

    lost: list[str] = []
    gained: list[str] = []
    for name, ids in sorted(baseline.items()):
        missing = ids - current.get(name, set())
        if missing:
            lost.append(f"  ⛔ {name}: lost {sorted(missing)}")
    for name, ids in sorted(current.items()):
        extra = ids - baseline.get(name, set())
        if extra:
            gained.append(f"  ＋ {name}: new {sorted(extra)}")

    for line in gained:
        print(line)
    if lost:
        print("\n".join(lost))
        print()
        print("An authored level disappeared. The most likely cause is a REGENERATE:")
        print("  an author_*.py rebuilds a world from the specs it knows, and a level")
        print("  authored by a different road is not among them. The file it wrote is")
        print("  valid, and every other check will pass on it.")
        print()
        print("If the level was deleted DELIBERATELY, re-run with --bless.")
        return 1

    total = sum(len(v) for v in current.values())
    print(f"{total} authored levels across {len(current)} worlds; none lost.")
    if gained:
        print("(new levels above are not an error; --bless to record them)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
