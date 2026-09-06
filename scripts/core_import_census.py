#!/usr/bin/env python3
"""Which crates depend on `ambition_platformer2d_core`, and for WHAT.

The rename made a claim visible: 19 crates with deliberately unqualified names
(`ambition_input`, `ambition_time`, `ambition_characters`, …) declare a
dependency on a crate named for one game genre. Either those crates are not as
general as their names say, or the dependency is on a KERNEL that has no
business being platformer-named.

This measures which. For every dependent, it reports every item imported from
the core crate, grouped by the core module the item comes from — so "how much of
the 28 is kernel-only" stops being a guess.

⚠ this is a MEASUREMENT, not a verdict. It reports paths and counts; whether
`Aabb` is kernel and `LedgeGrab` is platformer is a judgement made by reading the
output, not by this script. A script that classified them would be encoding the
answer it was written to find.

    python scripts/core_import_census.py            # summary
    python scripts/core_import_census.py --detail   # every symbol, per crate
"""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
from collections import defaultdict
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
DEFAULT_TARGET = "ambition_platformer2d_core"

# The crate under measurement. `--of` rewrites it; `_imports` and the alias
# pattern are the only things that read it.
#
# two module-level regexes (`USE_LINE`, `BARE_PATH`) stood here and were NEVER CALLED —
# `_imports` builds its own inline.
TARGET = DEFAULT_TARGET


def _members() -> list[dict]:
    out = subprocess.run(
        ["cargo", "metadata", "--no-deps", "--format-version", "1"],
        cwd=REPO,
        capture_output=True,
        text=True,
        check=True,
    ).stdout
    meta = json.loads(out)
    ids = set(meta["workspace_members"])
    return [pkg for pkg in meta["packages"] if pkg["id"] in ids]


def _depends_on_target(pkg: dict) -> bool:
    return any(dep["name"] == TARGET for dep in pkg["dependencies"])


def _source_alias() -> re.Pattern:
    """`use ambition_platformer2d_core as ae;` — the near-universal idiom here.

    ⚠ built per-target rather than at import time. The alias is declared in RUST,
    not in Cargo.toml, and reading it from the manifest `rename` field is the bug
    that once made two crates look like they imported NOTHING from a crate they
    use on every line.
    """
    return re.compile(r"use\s+" + re.escape(TARGET) + r"\s+as\s+(?P<alias>\w+)\s*;")


def _expand(tail: str) -> list[str]:
    """Flatten a use-tree tail into leaf paths.

    `config::{world_to_bevy, WORLD_Z_FX}` -> ['config::world_to_bevy', 'config::WORLD_Z_FX']
    """
    tail = " ".join(tail.split())
    if "{" not in tail:
        return [tail.replace(" ", "")]
    prefix, _, rest = tail.partition("{")
    prefix = prefix.strip().rstrip(":").rstrip(":")
    depth, current, parts = 0, "", []
    for ch in rest:
        if ch == "{":
            depth += 1
        elif ch == "}":
            if depth == 0:
                break
            depth -= 1
        if ch == "," and depth == 0:
            parts.append(current)
            current = ""
        else:
            current += ch
    parts.append(current)
    out: list[str] = []
    for part in parts:
        part = part.strip()
        if not part:
            continue
        for leaf in _expand(part) if "{" in part else [part.replace(" ", "")]:
            out.append(f"{prefix}::{leaf}" if prefix else leaf)
    return out


def _imports(pkg: dict) -> set[str]:
    """Every core item this crate names, resolved PER FILE.

    ⚠ the alias is declared in RUST, not in Cargo.toml. The first draft looked
    for a `rename` on the dependency and found none, so every `ae::Vec2` in the
    repo was invisible and two crates reported importing NOTHING from a crate
    they use on every other line. Whatever a file calls it, that file says so.
    """
    root = Path(pkg["manifest_path"]).parent
    found: set[str] = set()
    for rust in root.rglob("*.rs"):
        if "/target/" in str(rust):
            continue
        source = rust.read_text(encoding="utf-8", errors="ignore")
        aliases = {match["alias"] for match in _source_alias().finditer(source)}
        for match in re.finditer(
            r"use\s+(?:crate::)?(?P<root>[A-Za-z0-9_]+)\s*::\s*(?P<tail>[^;]+);",
            source,
            re.S,
        ):
            if match["root"] != TARGET and match["root"] not in aliases:
                continue
            found.update(_expand(match["tail"]))
        for alias in aliases | {TARGET}:
            for match in re.finditer(
                re.escape(alias)
                + r"\s*::\s*(?P<tail>(?:[A-Za-z0-9_]+\s*::\s*)*[A-Za-z0-9_]+)",
                source,
            ):
                found.add(match["tail"].replace(" ", ""))
    # `as X` renames carry no information about what was imported.
    cleaned = {re.sub(r"\s*as\s*\w+$", "", item) for item in found}
    # `self as ae` imports the CRATE, not an item in it.
    return {item for item in cleaned if item and item != "*" and item != "self"}


def _module_of(path: str) -> str:
    """The core MODULE a path reaches into, or `<crate root>`.

    ⚠ a single lowercase segment is NOT a module. `step_kinematic`,
    `snapshot_pod!` and `aabb_from_min_size` are a function, a macro and a
    function re-exported at the crate root; the first draft read all three as
    modules and invented nine module names that do not exist. Only a path with a
    second segment names a module.
    """
    parts = [part for part in path.split("::") if part]
    if len(parts) < 2:
        return "<crate root>"
    head = parts[0]
    return head if head[0].islower() else "<crate root>"


def _transitive_closure(name: str) -> set[str]:
    """Every workspace crate in `name`'s normal-edge build closure.

    ⚠ `--edges normal` deliberately: dev-dependencies do not ship, and the
    footprint ratchet reads the same edge set, so the two instruments agree about
    what "depends on" means.
    """
    out = subprocess.run(
        ["cargo", "tree", "-p", name, "--locked", "--edges", "normal", "--prefix", "none"],
        cwd=REPO, capture_output=True, text=True, check=False,
    ).stdout
    return {line.split()[0] for line in out.splitlines() if line.strip()}


def _workspace_edges() -> dict[str, set[str]]:
    """Workspace-crate → the workspace crates it declares as a normal dependency.

    ⚠ from `cargo metadata`, so it is the DECLARED graph. That is the right graph
    for this question: a shortest path here is a list of manifests somebody would
    have to change, which is exactly what `--paths` is asked to report.
    """
    out = subprocess.run(
        ["cargo", "metadata", "--no-deps", "--format-version", "1"],
        cwd=REPO, capture_output=True, text=True, check=True,
    ).stdout
    meta = json.loads(out)
    ids = set(meta["workspace_members"])
    names = {pkg["name"] for pkg in meta["packages"] if pkg["id"] in ids}
    edges: dict[str, set[str]] = {}
    for pkg in meta["packages"]:
        if pkg["id"] not in ids:
            continue
        edges[pkg["name"]] = {
            dep["name"] for dep in pkg["dependencies"]
            if dep["name"] in names
            and dep.get("kind") in (None, "null")
            # OPTIONAL edges excluded, and this is not cosmetic. The closure
            # count above comes from `cargo tree --edges normal`, which resolves
            # DEFAULT features — so counting an off-by-default optional edge here
            # would make the two halves of this one instrument disagree about the
            # same graph. `ambition_touch_input` is the case that showed it: it
            # declares monolith / render / sim_view optionally, and including them
            # made it look like it had seven paths to core when its default build
            # has far fewer.
            and not dep.get("optional", False)
        }
    return edges


def _shortest_path(start: str, goal: str, edges: dict[str, set[str]]) -> list[str] | None:
    """BFS, so the reported path is the FEWEST manifests that must change.

    ⚠ shortest is not the same as easiest — the one-hop path may be the immovable
    one (see the orphan-rule note in `ambition_projectile_spec`). This reports
    distance, and a human still decides which edge is actually cuttable.
    """
    if start == goal:
        return [start]
    seen = {start}
    queue = [[start]]
    while queue:
        path = queue.pop(0)
        for nxt in sorted(edges.get(path[-1], ())):
            if nxt in seen:
                continue
            if nxt == goal:
                return path + [nxt]
            seen.add(nxt)
            queue.append(path + [nxt])
    return None


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--detail", action="store_true", help="every symbol, per crate")
    parser.add_argument("--kernel", action="store_true",
                        help="union of what the general-NAMED crates import")
    parser.add_argument("--of", default=DEFAULT_TARGET, metavar="CRATE",
                        help="measure imports of CRATE instead of the core crate")
    parser.add_argument("--cuts", action="store_true",
                        help="rank direct dependents by how many crates would leave the "
                             "target's closure if that ONE edge were cut")
    parser.add_argument("--paths", action="store_true",
                        help="for every crate still carrying the target, the SHORTEST "
                             "dependency path to it — i.e. what would have to be cut")
    args = parser.parse_args()

    global TARGET
    TARGET = args.of
    known = {pkg["name"] for pkg in _members()}
    if TARGET not in known:
        print(f"no workspace crate named {TARGET!r}", file=sys.stderr)
        return 2

    dependents = sorted(
        (pkg for pkg in _members() if _depends_on_target(pkg)),
        key=lambda pkg: pkg["name"],
    )
    print(f"{len(dependents)} workspace crates DECLARE a dependency on {TARGET}")
    # Dropping a declared edge makes a manifest honest; it does not remove core from anybody's
    # build.
    freed = [pkg["name"] for pkg in _members()
             if not _depends_on_target(pkg) and pkg["name"] != TARGET
             and TARGET not in _transitive_closure(pkg["name"])]
    print(f"{len(freed)} of {len(_members()) - 1} build WITHOUT it in their closure\n")

    if args.cuts:
        # the actual planning question, and the one a shortest-path list
        # answers WRONGLY. Reading the `--paths` column, seven crates looked like
        # they reached core only through `ambition_input`; simulating the cut says
        # THREE. The others have additional core-carrying dependencies that a
        # shortest path hides by construction. Rank by the simulation, never by
        # the column.
        edges = _workspace_edges()
        carrying = {n for n in edges if n != TARGET and _shortest_path(n, TARGET, edges)}
        print(f"── if ONE crate dropped its direct {TARGET} edge, who leaves the closure ──")
        rows = []
        for name in sorted(edges):
            if TARGET not in edges.get(name, ()):
                continue
            trial = {k: set(v) for k, v in edges.items()}
            trial[name].discard(TARGET)
            still = {n for n in trial if n != TARGET and _shortest_path(n, TARGET, trial)}
            rows.append((len(carrying - still), name, sorted(carrying - still)))
        for count, name, freed in sorted(rows, reverse=True):
            names = ", ".join(f.replace("ambition_", "") for f in freed)
            print(f"  {count:2} crate(s)  {name:<44} {names}")
        print(f"\n  {len(carrying)} crates carry {TARGET} today.")
        print(
            "  ⚠ this ranks by VALUE, not by FEASIBILITY: it says what a cut would\n"
            "    buy, never whether the cut can be made. Measured 2026-08-02, the\n"
            "    #2 entry `ambition_characters` was not a carve at all — 20 of its\n"
            "    imports are genuinely platformer (AxisJumpLaw, AIR_ACCEL,\n"
            "    DASH_SPEED, MovementTuning), so its dependency is real and the\n"
            "    question there is renaming, not carving. Always read a row's\n"
            "    surface with --detail before planning against it."
        )
        print()

    if args.paths:
        # the question the carve-out plan kept needing a manual `cargo tree -i`
        # to answer: for a crate that still carries the target, WHAT is carrying
        # it. A one-hop path is a direct edge the crate could drop itself; a
        # longer one means the work is in somebody else's manifest first.
        edges = _workspace_edges()
        print(f"── shortest path to {TARGET}, for crates that still carry it ──")
        rows = []
        for pkg in _members():
            name = pkg["name"]
            if name == TARGET or name in freed:
                continue
            path = _shortest_path(name, TARGET, edges)
            if path:
                rows.append((len(path), name, path))
        for hops, name, path in sorted(rows):
            arrow = " → ".join(p.replace("ambition_", "") for p in path[1:])
            print(f"  {hops - 1} hop(s)  {name:<44} {arrow}")
        print()

    by_module: dict[str, set[str]] = defaultdict(set)
    per_crate: dict[str, set[str]] = {}
    for pkg in dependents:
        items = _imports(pkg)
        per_crate[pkg["name"]] = items
        for item in items:
            by_module[_module_of(item)].add(pkg["name"])

    print(f"── {TARGET} module → how many dependents touch it ──")
    for module, crates in sorted(by_module.items(), key=lambda kv: (-len(kv[1]), kv[0])):
        print(f"  {len(crates):3}  {module}")

    print("\n── dependents by surface size ──")
    for name, items in sorted(per_crate.items(), key=lambda kv: (len(kv[1]), kv[0])):
        modules = sorted({_module_of(item) for item in items})
        print(f"  {len(items):4} item(s)  {name:46} {','.join(modules) or '(none found)'}")

    if args.kernel:
        # The crates whose NAMES claim to be general services. If the kernel
        # story is right, their combined surface is small and contains nothing
        # platformer-specific — and that union IS the carve-out's contents.
        general = {
            name: items
            for name, items in per_crate.items()
            if "platformer2d" not in name and "portal2d" not in name
        }
        union: set[str] = set()
        for items in general.values():
            union |= items
        print(f"\n── the {len(general)} general-named dependents want {len(union)} distinct items ──")
        for item in sorted(union):
            wanters = sorted(n for n, items in general.items() if item in items)
            print(f"  {item:44} {len(wanters):2}  {', '.join(wanters)}")

    if args.detail:
        print("\n── every symbol, per crate ──")
        for name, items in sorted(per_crate.items()):
            print(f"\n{name}:")
            for item in sorted(items):
                print(f"    {item}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
