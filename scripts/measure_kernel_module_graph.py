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


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--edges", type=int, default=0, help="also print the top-N edges")
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
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
