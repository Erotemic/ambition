#!/usr/bin/env python3
"""How much of the actor kernel's apparent INTERNAL coupling is a re-export.

⭐ THE QUESTION. Every sizing of an actor-monolith carve counts references of
the form `crate::features::X` as coupling to the kernel's `features` module. But
`features/mod.rs` is partly a FACADE: it re-exports types that are defined in
crates BELOW the monolith. A file naming `crate::features::HeldItem` is not
coupled to `features` at all — `HeldItem` is `ambition_combat`'s, and the same
file could name it directly and lose the edge.

⛔ WHY THIS MATTERS RATHER THAN BEING TIDINESS. Carve sizings are chosen from
these counts. `docs/planning/queue.md` sized the `items/` carve as "multi-day"
partly on references that resolve out of the crate entirely, and this script was
written after a per-file count of `items/conditions.rs` reported seven kernel
references, six of which were one re-exported `ambition_combat` type.

WHAT IT REPORTS, and the distinction is the whole point:
  * RE-EXPORT   the name is defined in another crate  -> not kernel coupling
  * LOCAL       the name is defined inside the monolith -> real kernel coupling
  * UNRESOLVED  no definition found by this script      -> counted separately,
                never silently folded into either bucket

⚠ AND IT REPORTS VISIBILITY, because RE-EXPORT alone does not mean "defect". A
`pub` re-export of another crate's name is a road a consumer can learn and take.
A `pub(crate)` one — especially under `#[cfg(test)]` — is a local alias nobody
outside can reach, and re-pointing it is pure churn. This script flagged one of
those as a finding before anybody read its visibility.

⚠ IT IS A TEXT SCAN, NOT A RESOLVER. It finds `pub struct/enum/type/fn/trait`
definitions by regex, so a macro-generated or heavily-cfg'd name lands in
UNRESOLVED rather than being guessed at. Read UNRESOLVED as "ask rustc", not as
"none".

Usage:
    python3 scripts/measure_facade_reexport_coupling.py
    python3 scripts/measure_facade_reexport_coupling.py --by-file
"""

from __future__ import annotations

import argparse
import re
import subprocess
from collections import Counter, defaultdict
from pathlib import Path

KERNEL = Path("crates/ambition_platformer2d_actor_monolith")
FACADE = KERNEL / "src/features/mod.rs"

# `pub use ecs::{ ... };` style blocks in the facade.
#
# ⛔ VISIBILITY IS CAPTURED, and it is the distinction that decides whether a
# re-export is a DEFECT or a convenience. A `pub` re-export of another crate's
# name is a road a consumer can learn and take, which is what made the
# `crate::features::X` cases worth 45 edits. A `pub(crate)` one — especially
# under `#[cfg(test)]` — is a local alias nobody outside can reach, and
# re-pointing it is churn. This script reported one of those as a finding on
# 2026-09-02 before anyone read its visibility.
REEXPORT_BLOCK = re.compile(
    r"(pub(?:\(crate\))?)\s+use\s+[^;{]*\{([^}]*)\};", re.S
)
DEF = re.compile(
    r"^\s*pub(?:\((?:crate|super)\))?\s+"
    r"(?:struct|enum|trait|type|const|fn)\s+([A-Za-z_][A-Za-z0-9_]*)",
    re.M,
)


def facade_names() -> dict[str, str]:
    """Every name the facade re-exports -> the visibility it is exported at.

    ⚠ Returns a dict, not a set: a caller that ignores the value is asking the
    wrong question. `pub` is reachable from outside the crate; `pub(crate)` is
    not, and cannot be a wrong path anybody learns.
    """
    text = FACADE.read_text()
    names: dict[str, str] = {}
    for vis, block in REEXPORT_BLOCK.findall(text):
        for raw in block.split(","):
            name = raw.strip().split(" as ")[0].strip()
            if name and not name.startswith("//"):
                # `pub` wins if a name is exported twice at different visibility.
                if names.get(name) != "pub":
                    names[name] = vis.strip()
    return names


def definitions(root: Path) -> dict[str, set[str]]:
    """name -> set of crates that define it, over `crates/` and `game/`."""
    out: dict[str, set[str]] = defaultdict(set)
    for base in ("crates", "game"):
        for path in (root / base).rglob("*.rs"):
            if "/target/" in str(path):
                continue
            # crate name = the directory two levels under crates/ or game/
            try:
                crate = path.relative_to(root / base).parts[0]
            except ValueError:
                continue
            for name in DEF.findall(path.read_text(errors="ignore")):
                out[name].add(crate)
    return out


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--by-file", action="store_true", help="list the top files")
    args = ap.parse_args()

    root = Path(__file__).resolve().parent.parent
    names = facade_names()
    defs = definitions(root)

    kernel_crate = KERNEL.name
    classified: dict[str, str] = {}
    home: dict[str, str] = {}
    for name in names:
        where = defs.get(name, set())
        outside = {c for c in where if c != kernel_crate}
        if not where:
            classified[name] = "UNRESOLVED"
        elif kernel_crate in where:
            # Defined in the kernel too: a local definition wins, because the
            # re-export cannot be the only road to it.
            classified[name] = "LOCAL"
            home[name] = kernel_crate
        else:
            classified[name] = "RE-EXPORT"
            home[name] = sorted(outside)[0]

    # Count `crate::features::NAME` uses across the kernel's own source.
    hits = subprocess.run(
        ["grep", "-rno", r"crate::features::[A-Za-z_][A-Za-z0-9_]*",
         str(root / KERNEL / "src")],
        capture_output=True, text=True, check=False,
    ).stdout.splitlines()

    per_kind: Counter[str] = Counter()
    per_crate: Counter[str] = Counter()
    per_file: Counter[str] = Counter()
    for line in hits:
        path, _, rest = line.partition(":")
        name = rest.split("crate::features::")[-1]
        if name not in classified:
            continue
        kind = classified[name]
        per_kind[kind] += 1
        if kind == "RE-EXPORT":
            per_crate[home[name]] += 1
            per_file[Path(path).relative_to(root).as_posix()] += 1

    total = sum(per_kind.values())
    externally_reachable = sum(1 for v in names.values() if v == "pub")
    print(
        f"facade re-exports {len(names)} names "
        f"({externally_reachable} `pub`, {len(names) - externally_reachable} crate-local)"
    )
    print("⚠ only the `pub` ones are roads a consumer can learn; the rest are aliases")
    for kind in ("RE-EXPORT", "LOCAL", "UNRESOLVED"):
        n = sum(1 for k in classified.values() if k == kind)
        print(f"  {kind:<11} {n:>4} names")
    print()
    print(f"`crate::features::X` uses inside the kernel: {total}")
    for kind in ("RE-EXPORT", "LOCAL", "UNRESOLVED"):
        share = 100.0 * per_kind[kind] / total if total else 0.0
        print(f"  {kind:<11} {per_kind[kind]:>4}  ({share:.0f}%)")
    print()
    print("RE-EXPORT uses by the crate that really owns the name:")
    for crate, n in per_crate.most_common():
        print(f"  {n:>4}  {crate}")
    if args.by_file:
        print()
        print("Files naming the most re-exported names (each is a lost edge):")
        for path, n in per_file.most_common(15):
            print(f"  {n:>4}  {path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
