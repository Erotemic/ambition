#!/usr/bin/env python3
"""Ratchet player-visible loading-zone names that still look like authoring ids.

A zone `name` is presentation text, so underscore-shaped identifiers should be
replaced by authored prose rather than prettified mechanically or hidden. Counts
are tracked per world file and symlinked worlds are deduplicated by real path so
one world's improvement cannot hide another's regression.

Usage::

    python3 scripts/check_zone_name_ratchet.py
    python3 scripts/check_zone_name_ratchet.py --check
    python3 scripts/check_zone_name_ratchet.py --update"""

from __future__ import annotations

import argparse
import json
import os
import re
import sys

REPO = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
BASELINE = os.path.join(REPO, "dev", "zone_name_ratchet_baseline.json")

#: A name that is an authoring id: lowercase words joined by underscores.
#: Deliberately narrow — it must not fire on prose, and prose is what we want.
ID_SHAPED = re.compile(r"^[a-z0-9]+(_[a-z0-9]+)+$")

WORLD_ROOT = os.path.join(REPO, "game")


def world_files() -> list[str]:
    """Every distinct `.ldtk`, deduped by real path (they are symlinked)."""

    seen: dict[str, str] = {}
    for root, _dirs, files in os.walk(WORLD_ROOT):
        if os.sep + ".git" in root:
            continue
        for name in files:
            if not name.endswith(".ldtk"):
                continue
            real = os.path.realpath(os.path.join(root, name))
            seen.setdefault(real, real)
    return sorted(seen)


def zone_names(path: str) -> list[str]:
    with open(path, encoding="utf-8") as handle:
        project = json.load(handle)
    names: list[str] = []
    for level in project.get("levels") or []:
        for layer in level.get("layerInstances") or []:
            for entity in layer.get("entityInstances") or []:
                if entity.get("__identifier") != "LoadingZone":
                    continue
                for field in entity.get("fieldInstances") or []:
                    if field.get("__identifier") == "name" and field.get("__value"):
                        names.append(str(field["__value"]))
    return names


def measure() -> tuple[dict[str, int], int, int]:
    """Per-world id counts, plus the totals the report prints."""

    counts: dict[str, int] = {}
    total = 0
    id_shaped = 0
    for path in world_files():
        names = zone_names(path)
        total += len(names)
        bad = sum(1 for name in names if ID_SHAPED.match(name))
        id_shaped += bad
        if bad:
            counts[os.path.relpath(path, REPO)] = bad
    return counts, total, id_shaped


def load_baseline() -> dict[str, int] | None:
    """The recorded per-world counts, or `None` when no baseline exists.

    ⛔ **`None` and `{}` are different and the difference only shows up on
    SUCCESS.** An empty dict is the state where every zone has an authored name
    — the goal — and returning `{}` for "no file" made that indistinguishable
    from "never recorded", so the check would have started failing at the exact
    moment it was satisfied.
    """

    if not os.path.exists(BASELINE):
        return None
    with open(BASELINE, encoding="utf-8") as handle:
        return json.load(handle).get("worlds", {})


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="exit 1 when a count rises")
    parser.add_argument("--update", action="store_true", help="rewrite the baseline")
    args = parser.parse_args()

    counts, total, id_shaped = measure()

    # A CHECK THAT OBSERVED NOTHING IS NOT A PERFECT SCORE. A moved
    # directory, a renamed entity, or a parse error all produce zero rows, which
    # would otherwise read as "every zone is authored" and pass forever.
    if total == 0:
        print("FAIL: no LoadingZone names were observed at all — the check is broken, not the content")
        return 1

    print(f"loading zones carrying a name   {total}")
    print(f"  name is an authoring id       {id_shaped}  ({100 * id_shaped / total:.0f}%)")
    for world, count in sorted(counts.items(), key=lambda kv: -kv[1]):
        print(f"    {count:>4}  {world}")

    if args.update:
        os.makedirs(os.path.dirname(BASELINE), exist_ok=True)
        with open(BASELINE, "w", encoding="utf-8") as handle:
            json.dump({"worlds": counts}, handle, indent=2, sort_keys=True)
            handle.write("\n")
        print(f"\nbaseline written: {os.path.relpath(BASELINE, REPO)}")
        return 0

    if not args.check:
        return 0

    baseline = load_baseline()
    if baseline is None:
        print(f"\nFAIL: no baseline at {os.path.relpath(BASELINE, REPO)}; run --update")
        return 1

    risen = [
        (world, baseline.get(world, 0), count)
        for world, count in counts.items()
        if count > baseline.get(world, 0)
    ]
    if risen:
        print("\nFAIL: a world gained loading-zone names that are authoring ids:")
        for world, was, now in sorted(risen):
            print(f"    {world}: {was} -> {now}")
        print(
            "\n⇒ author the zone's `name` as the prose a player should read "
            '(`water_world` says "to basement hub"), or lower another world in '
            "the same commit and rerun with --update."
        )
        return 1

    fell = [
        (world, baseline[world], counts.get(world, 0))
        for world in baseline
        if counts.get(world, 0) < baseline[world]
    ]
    if fell:
        print("\n✔ improved — rerun with --update to bank it:")
        for world, was, now in sorted(fell):
            print(f"    {world}: {was} -> {now}")
    else:
        print("\n✔ no world gained an id-shaped zone name")
    return 0


if __name__ == "__main__":
    sys.exit(main())
