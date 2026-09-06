#!/usr/bin/env python3
"""The residual actor kernel's module-to-module reference graph.

For every top-level module of `ambition_platformer2d_actor_monolith` (a
directory or a single file under `src/`), count production lines and the
`crate::<module>` references it makes to each other top-level module. Test
files (`tests.rs`, `*_tests.rs`, anything under a `tests/` directory) are
counted in the `all` column and excluded from the edges, because a test
reaching across modules is a fixture, not a dependency.

This is the input the decomposition doc's "central kernel split" asks for:
"once outer domains are gone, the remaining dependency graph will show whether
body state, movement, decision integration and construction still need one
crate or have another stable seam." It is a textual count of qualified paths,
so it undercounts what a `use` brings in by glob and overcounts a name in a
string; read it as a shape, not a bill.

    python3 scripts/measure_kernel_module_graph.py            # table
    python3 scripts/measure_kernel_module_graph.py --edges N  # top-N edges
"""

from __future__ import annotations

import argparse
import collections
import os
import re
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
KERNEL = REPO / "crates" / "ambition_platformer2d_actor_monolith" / "src"
PATH_REF = re.compile(r"\bcrate::([a-z_][a-z0-9_]*)")


def is_test_file(path: Path) -> bool:
    name = path.name
    return name == "tests.rs" or name.endswith("_tests.rs") or "tests" in path.parent.parts[-1:]


def modules() -> dict[str, list[Path]]:
    found: dict[str, list[Path]] = {}
    for entry in sorted(KERNEL.iterdir()):
        if entry.is_dir():
            found[entry.name] = sorted(entry.rglob("*.rs"))
        elif entry.suffix == ".rs" and entry.name != "lib.rs":
            found[entry.stem] = [entry]
    return found


def count_lines(path: Path) -> int:
    with path.open(errors="replace") as handle:
        return sum(1 for _ in handle)


def measure() -> tuple[dict[str, int], dict[str, int], dict[str, collections.Counter]]:
    mods = modules()
    prod = {m: sum(count_lines(f) for f in fs if not is_test_file(f)) for m, fs in mods.items()}
    total = {m: sum(count_lines(f) for f in fs) for m, fs in mods.items()}
    edges: dict[str, collections.Counter] = collections.defaultdict(collections.Counter)
    for m, fs in mods.items():
        for f in fs:
            if is_test_file(f):
                continue
            with f.open(errors="replace") as handle:
                for line in handle:
                    if line.lstrip().startswith("//"):
                        continue
                    for target in PATH_REF.findall(line):
                        if target in mods and target != m:
                            edges[m][target] += 1
    return prod, total, edges



def strongly_connected(edges: dict[str, "collections.Counter[str]"]) -> list[list[str]]:
    """Tarjan's SCCs over the module graph.

    ⭐⭐ THE QUESTION THE EDGE TABLE CANNOT ANSWER. An edge list says which
    modules reference which; it does not say which modules CANNOT BE SEPARATED.
    A component of size N means no member of it can leave for its own crate
    without the other N-1, however clean any individual edge looks — which is
    the only question that matters before a capability-composition push.
    """
    adjacency = {src: set(dsts) for src, dsts in edges.items()}
    nodes = sorted(set(adjacency) | {d for dsts in adjacency.values() for d in dsts})
    index: dict[str, int] = {}
    low: dict[str, int] = {}
    on_stack: set[str] = set()
    stack: list[str] = []
    components: list[list[str]] = []
    counter = [0]

    def visit(root: str) -> None:
        work = [(root, 0)]
        while work:
            node, child = work[-1]
            if child == 0:
                index[node] = low[node] = counter[0]
                counter[0] += 1
                stack.append(node)
                on_stack.add(node)
            recursed = False
            successors = sorted(adjacency.get(node, ()))
            for i in range(child, len(successors)):
                nxt = successors[i]
                if nxt not in index:
                    work[-1] = (node, i + 1)
                    work.append((nxt, 0))
                    recursed = True
                    break
                if nxt in on_stack:
                    low[node] = min(low[node], index[nxt])
            if recursed:
                continue
            if low[node] == index[node]:
                component = []
                while True:
                    popped = stack.pop()
                    on_stack.discard(popped)
                    component.append(popped)
                    if popped == node:
                        break
                components.append(sorted(component))
            work.pop()
            if work:
                parent = work[-1][0]
                low[parent] = min(low[parent], low[node])

    for node in nodes:
        if node not in index:
            visit(node)
    return sorted((c for c in components if len(c) > 1), key=len, reverse=True)


def shrinking_cuts(edges) -> list[tuple[int, int, str, str]]:
    """Single edges whose removal makes the largest component smaller.

    ⛔ A DENSE KNOT USUALLY HAS NONE, WHICH IS WHY THE ONES IT HAS ARE WORTH
    FINDING. Measured 2026-09-06: `assets -> session` was ONE reference — a
    single `use` of a two-field `Deserialize` struct — and removing it took the
    kernel's component from 15 modules to 13. The cheapest cut and the biggest
    payoff were the same edge, and nothing in the edge table said so.
    """
    biggest = len(strongly_connected(edges)[0]) if strongly_connected(edges) else 0
    rows = []
    for src in list(edges):
        for dst in list(edges[src]):
            trimmed = {a: collections.Counter(b) for a, b in edges.items()}
            del trimmed[src][dst]
            after = strongly_connected(trimmed)
            size = len(after[0]) if after else 0
            if size < biggest:
                rows.append((size, edges[src][dst], src, dst))
    return sorted(rows, key=lambda r: (r[0], r[1]))


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--edges", type=int, default=0, help="also print the top-N edges")
    parser.add_argument(
        "--scc",
        action="store_true",
        help="print the cyclic components — which modules cannot be separated",
    )
    parser.add_argument(
        "--cuts",
        action="store_true",
        help="single edges whose removal shrinks the largest component",
    )
    args = parser.parse_args()
    prod, total, edges = measure()
    print(f"{'module':22} {'prod':>6} {'all':>6}  out-edges (module:refs)")
    for m in sorted(prod, key=lambda x: -prod[x]):
        out = " ".join(f"{t}:{c}" for t, c in edges[m].most_common(8))
        print(f"{m:22} {prod[m]:6} {total[m]:6}  {out}")
    if args.edges:
        flat = sorted(
            ((c, m, t) for m, counter in edges.items() for t, c in counter.items()),
            reverse=True,
        )
        print()
        print("top edges (refs  from -> to):")
        for c, m, t in flat[: args.edges]:
            back = edges[t][m]
            print(f"  {c:4}  {m} -> {t}" + (f"   (and {back} back)" if back else ""))
    if args.scc:
        components = strongly_connected(edges)
        print("\ncyclic components (modules that cannot be separated):")
        if not components:
            print("  none — the module graph is a DAG")
        for component in components:
            print(f"  {len(component)}: {', '.join(component)}")
    if args.cuts:
        print("\nsingle edges whose removal shrinks the largest component:")
        rows = shrinking_cuts(edges)
        if not rows:
            print("  none — no single edge splits it; the knot needs a coordinated cut")
        for after, refs, src, dst in rows:
            print(f"  -> {after:2}   {refs:3} refs   {src} -> {dst}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
