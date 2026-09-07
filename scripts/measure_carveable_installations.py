#!/usr/bin/env python3
"""Split "foreign system installations" into REDUCIBLE and IRREDUCIBLE.

⭐ WHY THIS EXISTS. `measure_foreign_system_ordering.py` reports one number for
"INSTALLING a foreign system (the broader question)". Read alone it invites the
conclusion that every one of them is a carve waiting to happen. It is not, and the
distinction is structural rather than a matter of effort:

  REDUCIBLE   the block names systems/sets from exactly ONE capability (plus shared
              vocabulary every crate already depends on). That capability can install
              itself, and the composition names one function instead of N paths.

  IRREDUCIBLE the block names TWO capabilities that do not depend on each other. The
              composition is the ONLY place that can name both, so the naming is its
              job rather than a leak. Measured example: `projectile_visuals` orders
              `ambition_render` systems `.after` a `ambition_platformer2d_runtime` set,
              and render does not depend on runtime — that block cannot move anywhere.

⛔⛔ AND THE OBVIOUS IMPLEMENTATION IS WRONG, which is why this is a script and not a
grep. Classifying a block by the `crate::path::` prefixes written in it MISSES every
foreign system imported with `use` and named bare — a scanner doing that called two
blocks "pure input" while they contained the host's own
`sync_primary_recipe_from_settings` and `declare_gameplay_input_context`. This resolves
the file's `use` statements first and attributes bare names through them.

⚠ IT IS A REPORT, NOT A GATE. The reducible count is an upper bound on easy carves, not
a promise: a block can be single-capability and still be entangled by a `.chain()` that
crosses a lane boundary (measured: `CombatSet::Playback` chains eleven
`ambition_combat` systems with one `actor_monolith` system). Those show as REDUCIBLE
here and are not.
"""
import argparse
import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parents[1]
#: Vocabulary every capability already depends on; naming it is not a leak.
SHARED = {"ambition_platformer2d_shared_tangle"}


def use_map(text: str) -> dict[str, str]:
    """Bare name -> owning crate, from the file's `use ambition_x::{a, b}` statements."""
    owner: dict[str, str] = {}
    for m in re.finditer(r"use\s+(ambition_[a-z_0-9]+)::([^;]+);", text, re.S):
        crate, tail = m.group(1), m.group(2)
        for name in re.findall(r"\b([a-z_][a-z_0-9]*)\b", tail):
            owner.setdefault(name, crate)
        for name in re.findall(r"\b([A-Z][A-Za-z0-9]*)\b", tail):
            owner.setdefault(name, crate)
    return owner


def blocks(lines: list[str]) -> list[tuple[int, str]]:
    out = []
    for i, line in enumerate(lines):
        if "add_systems(" not in line and "configure_sets(" not in line:
            continue
        depth, end = 0, i
        for j in range(i, len(lines)):
            depth += lines[j].count("(") - lines[j].count(")")
            if depth <= 0 and j > i:
                end = j
                break
        out.append((i + 1, "\n".join(lines[i : end + 1])))
    return out


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("files", nargs="*", help="host files to classify")
    args = ap.parse_args()
    files = args.files or [
        "crates/ambition_platformer2d_runtime/src/combat_schedule.rs",
        "crates/ambition_platformer2d_runtime/src/player_schedule.rs",
        "crates/ambition_platformer2d_runtime/src/progression_schedule.rs",
        "crates/ambition_platformer2d_runtime/src/sim_core_resources.rs",
        "crates/ambition_platformer2d_runtime/src/world_gating.rs",
        "crates/ambition_platformer2d_host/src/lib.rs",
    ]
    total_r = total_i = 0
    for rel in files:
        path = ROOT / rel
        if not path.exists():
            print(f"  (absent: {rel})")
            continue
        text = path.read_text()
        owners = use_map(text)
        own_crate = re.search(r"crates/([a-z_0-9]+)/", rel)
        own_crate = own_crate.group(1) if own_crate else ""
        red, irr = [], []
        for line, body in blocks(text.splitlines()):
            named = set(re.findall(r"\b(ambition_[a-z_0-9]+)::", body))
            for bare in re.findall(r"\b([a-z_][a-z_0-9]*)\b", body):
                if bare in owners:
                    named.add(owners[bare])
            named -= SHARED | {own_crate}
            if len(named) == 1:
                red.append((line, next(iter(named))))
            elif len(named) > 1:
                irr.append((line, sorted(named)))
        total_r += len(red)
        total_i += len(irr)
        if red or irr:
            print(f"\n{rel}")
            print(f"   REDUCIBLE   {len(red):>3}  (one capability + shared vocabulary)")
            for line, crate in red:
                print(f"       line {line:>4}  {crate}")
            print(f"   IRREDUCIBLE {len(irr):>3}  (two capabilities that do not depend on each other)")
    print(f"\n  REDUCIBLE   {total_r}   — a capability could install these itself")
    print(f"  IRREDUCIBLE {total_i}   — the composition is the only place that can name both")
    print("\n⚠ Reducible is an UPPER BOUND: a single-capability block can still be "
          "entangled by a `.chain()` that crosses a lane boundary.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
