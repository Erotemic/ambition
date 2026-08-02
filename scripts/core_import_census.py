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
CORE = "ambition_platformer2d_core"

# `use ambition_platformer2d_core::…;` and `use ae::…;` (the near-universal alias),
# plus bare path uses like `ambition_platformer2d_core::config::WORLD_Z_FX`.
USE_LINE = re.compile(
    r"use\s+(?:crate::)?(?P<root>" + CORE + r"|ae)\s*::\s*(?P<tail>[^;]+);",
    re.S,
)
BARE_PATH = re.compile(CORE + r"\s*::\s*(?P<tail>(?:[A-Za-z0-9_]+\s*::\s*)*[A-Za-z0-9_]+)")


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


def _depends_on_core(pkg: dict) -> bool:
    return any(dep["name"] == CORE for dep in pkg["dependencies"])


# `use ambition_platformer2d_core as ae;` — the near-universal idiom here.
SOURCE_ALIAS = re.compile(r"use\s+" + CORE + r"\s+as\s+(?P<alias>\w+)\s*;")


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
        aliases = {match["alias"] for match in SOURCE_ALIAS.finditer(source)}
        for match in re.finditer(
            r"use\s+(?:crate::)?(?P<root>[A-Za-z0-9_]+)\s*::\s*(?P<tail>[^;]+);",
            source,
            re.S,
        ):
            if match["root"] != CORE and match["root"] not in aliases:
                continue
            found.update(_expand(match["tail"]))
        for alias in aliases | {CORE}:
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


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--detail", action="store_true", help="every symbol, per crate")
    parser.add_argument("--kernel", action="store_true",
                        help="union of what the general-NAMED crates import")
    args = parser.parse_args()

    dependents = sorted(
        (pkg for pkg in _members() if _depends_on_core(pkg)),
        key=lambda pkg: pkg["name"],
    )
    print(f"{len(dependents)} workspace crates depend on {CORE}\n")

    by_module: dict[str, set[str]] = defaultdict(set)
    per_crate: dict[str, set[str]] = {}
    for pkg in dependents:
        items = _imports(pkg)
        per_crate[pkg["name"]] = items
        for item in items:
            by_module[_module_of(item)].add(pkg["name"])

    print("── core module → how many dependents touch it ──")
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
