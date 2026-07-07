#!/usr/bin/env python3
"""gates audit: list a level's gating / destructible elements — read-only.

One view of the elements that lock, toggle, or block a region — switches
(and what they target), runtime lock walls, encounter triggers, and
breakable platforms / pogo orbs — so a boss/encounter author doesn't have
to grep the JSON to answer "what gates this area, and what flips it?".
Switches print their action + target so the control relationship is
explicit; lock walls / triggers are listed with ids + px for visual
correlation (the lock wall is runtime-inserted when its encounter goes
Active, keyed by encounter id).

Examples:
  python -m ambition_ldtk_tools gates audit --level goblin_encounter
  python -m ambition_ldtk_tools gates audit          # every level
"""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[4]

from ambition_ldtk_tools.area_authoring import load_project  # noqa: E402
from ambition_ldtk_tools.edit.query import collect  # noqa: E402

DEFAULT_LDTK = (
    REPO_ROOT
    / "crates"
    / "ambition_actors"
    / "assets"
    / "ambition"
    / "worlds"
    / "sandbox.ldtk"
)

SWITCH_TYPES = {"Switch"}
LOCK_TYPES = {"LockWall"}
TRIGGER_TYPES = {"EncounterTrigger"}
BREAKABLE_TYPES = {"BreakablePlatform", "BreakablePogoOrb"}
GATE_TYPES = SWITCH_TYPES | LOCK_TYPES | TRIGGER_TYPES | BREAKABLE_TYPES


def _field(row: dict, name: str, default: str = "?") -> str:
    val = row["fields"].get(name, default)
    return "?" if val is None else str(val)


def main(argv=None) -> int:
    if argv is None:
        argv = sys.argv[1:]
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--ldtk", type=Path, default=DEFAULT_LDTK)
    ap.add_argument("--level", help="restrict to one level id")
    args = ap.parse_args(argv)

    project = load_project(args.ldtk)
    rows = [
        r
        for r in collect(project, args.level, None, [], None)
        if r["identifier"] in GATE_TYPES
    ]
    if not rows:
        print(
            "no gating elements found (switches / lock walls / triggers / breakables)"
        )
        return 0

    by_level: dict[str, list[dict]] = {}
    for r in rows:
        by_level.setdefault(r["level"], []).append(r)

    for level in sorted(by_level):
        lr = by_level[level]
        triggers = [r for r in lr if r["identifier"] in TRIGGER_TYPES]
        locks = [r for r in lr if r["identifier"] in LOCK_TYPES]
        switches = [r for r in lr if r["identifier"] in SWITCH_TYPES]
        breakables = [r for r in lr if r["identifier"] in BREAKABLE_TYPES]

        print(f"# {level}")
        if triggers:
            print("  encounters (triggers):")
            for r in triggers:
                print(f"    - {_field(r, 'id')}  px={r['px']}")
        if locks:
            print("  lock walls (inserted while the keyed encounter is Active):")
            for r in locks:
                print(f"    - {_field(r, 'id')}  px={r['px']} size={r['size']}")
        if switches:
            print("  switches (id: action -> target):")
            for r in switches:
                target = r["fields"].get("target_encounter") or None
                # A switch with no target_encounter doesn't gate an encounter
                # — it piggybacks on the switch-activation bus and is consumed
                # by id elsewhere (e.g. falling_sand spouts). Surface its prompt
                # so the real purpose is visible instead of an empty arrow.
                target_str = target if target else "(none — consumed by id)"
                prompt = r["fields"].get("prompt")
                prompt_str = f'  "{prompt}"' if prompt else ""
                print(
                    f"    - {_field(r, 'id')}: {_field(r, 'action')}"
                    f" -> {target_str}{prompt_str}"
                )
        if breakables:
            kinds = ", ".join(sorted({r["identifier"] for r in breakables}))
            print(f"  breakables: {len(breakables)} ({kinds})")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
