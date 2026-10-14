#!/usr/bin/env python3
"""Who makes Bevy match query states against archetypes, from a desktop perf
profile: no Android needed.

Record a game with DWARF call stacks (perf needs root on a host whose
`perf_event_paranoid` is above 1; attaching to a running process keeps the
game itself a normal process):

    AMBITION_PROFILE_CENSUS=1 target/debug/capture_scene --route mary_o_gameplay out.png 1280x720 --warmup 3000 &
    sudo perf record -F 299 --call-graph dwarf,32768 -p "$(pgrep -n -x capture_scene)" -o perf.data -- sleep 15
    sudo chown "$USER" perf.data
    python3 scripts/attribute_archetype_matching.py perf.data target/debug/capture_scene

It prints the share of samples in archetype matching (`update_archetypes`,
`new_archetype`) and, for each, the first frame of the game binary below the
ECS plumbing that made the call. A Bevy executor frame there means the
per-run check of the systems that executor runs; a system's own name means
that system makes a query state on each run (`World::query`, `QueryState::new`).

Read the `[census] ecs` and `[census] render_ecs` rows of the same run beside
it: matching that is not new archetypes being made is per-run work.
"""

from __future__ import annotations

import collections
import re
import subprocess
import sys

PLUMBING = (
    "update_archetypes", "new_archetype", "matches_component_set", "fixedbitset", "FixedBitSet",
    "QueryState", "alloc::", "mi_", "core::", "hashbrown", "SparseArray", "SparseSet", "Archetype",
    "ComponentId", "Access", "init_state", "get_param", "SystemParam", "run_unsafe",
    "run_without_applying_deferred", "System>::", "SystemState", "Command>::apply", "validate_param",
    "bevy_ecs::query", "bevy_ecs::world", "bevy_ecs::storage",
)


def stacks(perf_data: str):
    """Each sample's frames as (offset in its file, symbol, file), leaf first."""
    proc = subprocess.Popen(
        ["perf", "script", "-i", perf_data, "-F", "ip,sym,dsoff", "--no-inline"],
        stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, text=True,
    )
    frame = re.compile(r"^\s*[0-9a-f]+ (.*) \((.*)\+(0x[0-9a-f]+)\)$")
    stack: list[tuple[int, str, str]] = []
    for line in proc.stdout:
        if not line.strip():
            if stack:
                yield stack
            stack = []
            continue
        m = frame.match(line.rstrip("\n"))
        if m:
            stack.append((int(m.group(3), 16), m.group(1), m.group(2)))
    if stack:
        yield stack


def main() -> int:
    if len(sys.argv) != 3:
        print(__doc__)
        return 2
    perf_data, binary = sys.argv[1], sys.argv[2]
    name = binary.split("/")[-1]
    hits: collections.Counter[str] = collections.Counter()
    total = matched = 0
    for stack in stacks(perf_data):
        total += 1
        if not any("update_archetypes" in s or "new_archetype" in s for _, s, _ in stack):
            continue
        matched += 1
        ours = [sym for _, sym, dso in stack if dso.endswith(name)]
        pick = next((sym for sym in ours if not any(k in sym for k in PLUMBING)), None)
        if pick is None:
            pick = next((sym for sym in ours if "run_unsafe" in sym or "get_param" in sym), "?")
        hits[pick] += 1
    print(f"samples {total}; in archetype matching {matched} ({100 * matched / max(1, total):.2f}%)")
    for sym, count in hits.most_common(15):
        print(f"{count:6d} {100 * count / max(1, matched):5.1f}%  {sym[:200]}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
