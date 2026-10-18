#!/usr/bin/env python3
"""Attribute `perf script -F time,ip,sym` stacks of the main thread to Bevy systems.

    system_shares.py STACKS            each system's inclusive share of the samples
    system_shares.py STACKS SUBSTRING  that system's immediate callees and leaves

A sample belongs to the innermost system frame on its stack: the frame inside
`System::run_without_applying_deferred` (the system's body is inlined into its
`run_unsafe`, so the next frame in is its own function). Samples whose inner
frame is the executor itself, or `World::resource_scope` (a schedule run by a
system), are the per-system cost of running systems at all: parameter fetch,
archetype checks, deferred application. `headless_system_shares.sh` makes the
input.
"""

from __future__ import annotations

import collections
import re
import sys

SYSTEM_FRAME = (
    "bevy_ecs::system::system::System::run_without_applying_deferred",
    "<bevy_ecs::system::function_system::FunctionSystem",
    "bevy_ecs::schedule::executor::__rust_begin_short_backtrace::run",
)
GLUE = ("core::", "alloc::", "<core::", "<alloc::")
HASH = re.compile(r"::h[0-9a-f]{16}$")


def samples(path: str):
    stack: list[str] = []
    for line in open(path, errors="replace"):
        if not line.strip():
            if stack:
                yield stack
                stack = []
            continue
        if line[0] in " \t":
            parts = line.strip().split(" ", 1)
            stack.append(HASH.sub("", parts[1] if len(parts) > 1 else "?"))
    if stack:
        yield stack


def owner(stack: list[str]) -> tuple[str, int]:
    """The system that owns a sample (leaf first), and the index of its frame."""
    for i, frame in enumerate(stack):
        if frame.startswith(SYSTEM_FRAME):
            j = i - 1
            while j >= 0 and stack[j].startswith(GLUE):
                j -= 1
            return (stack[j] if j >= 0 else "(executor)"), j
    return "(no system)", -1


def main() -> None:
    path = sys.argv[1]
    focus = sys.argv[2] if len(sys.argv) > 2 else None
    total = 0
    systems: collections.Counter[str] = collections.Counter()
    children: collections.Counter[str] = collections.Counter()
    leaves: collections.Counter[str] = collections.Counter()
    for stack in samples(path):
        total += 1
        name, j = owner(stack)
        systems[name[:160]] += 1
        if focus and focus in name:
            children[(stack[j - 1] if j >= 1 else "(self)")[:150]] += 1
            leaves[stack[0][:150]] += 1
    if not total:
        sys.exit("no samples")
    if not focus:
        for name, n in systems.most_common(50):
            print(f"{100 * n / total:6.2f}%  {name}")
        print(f"samples: {total}")
        return
    mine = sum(children.values())
    print(f"{focus}: {mine} samples, {100 * mine / total:.2f}% of the main thread")
    for name, n in children.most_common(25):
        print(f"  callee {100 * n / mine:5.1f}%  {name}")
    for name, n in leaves.most_common(20):
        print(f"  leaf   {100 * n / mine:5.1f}%  {name}")


if __name__ == "__main__":
    main()
