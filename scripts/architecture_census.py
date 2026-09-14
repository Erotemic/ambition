#!/usr/bin/env python3
"""Produce repeatable static measurements for the architecture census.

This tool does not compile Rust. It parses manifests and source text. Results
from text patterns are discovery aids, not proof of Rust semantics.

Examples:
    python3 scripts/architecture_census.py
    python3 scripts/architecture_census.py --json
    python3 scripts/architecture_census.py --crate-table
"""

from __future__ import annotations

import argparse
import collections
import json
import pathlib
import re
import subprocess
import sys
import tomllib
from dataclasses import asdict, dataclass
from typing import Any, Iterable

REPO = pathlib.Path(__file__).resolve().parent.parent
LEDGER = REPO / "docs/planning/consolidation/consolidation-ledger.json"
SOURCE_ROOTS = ("crates", "game", "tests", "tools")
LARGE_MODULE_NONBLANK_LINES = 1000


@dataclass(frozen=True)
class CrateRow:
    package: str
    path: str
    rust_files: int
    raw_rust_loc: int
    nonblank_rust_loc: int
    test_rust_loc_heuristic: int
    direct_workspace_dependencies: tuple[str, ...]
    reverse_workspace_dependencies: tuple[str, ...]
    dev_workspace_dependencies: tuple[str, ...]
    direct_bevy_dependency: bool
    public_items_heuristic: int
    public_root_reexports_or_modules_heuristic: int
    responsibility_clue: str


def load_toml(path: pathlib.Path) -> dict[str, Any]:
    return tomllib.loads(path.read_text(encoding="utf-8"))


def workspace_members() -> list[pathlib.Path]:
    root = load_toml(REPO / "Cargo.toml")
    return [REPO / member for member in root["workspace"]["members"]]


def package_name(member: pathlib.Path) -> str:
    return str(load_toml(member / "Cargo.toml")["package"]["name"])


def dependency_tables(data: dict[str, Any], *, include_dev: bool) -> Iterable[dict[str, Any]]:
    for key in ("dependencies", "build-dependencies"):
        table = data.get(key)
        if isinstance(table, dict):
            yield table
    if include_dev:
        table = data.get("dev-dependencies")
        if isinstance(table, dict):
            yield table
    targets = data.get("target", {})
    if isinstance(targets, dict):
        for target in targets.values():
            if not isinstance(target, dict):
                continue
            for key in ("dependencies", "build-dependencies"):
                table = target.get(key)
                if isinstance(table, dict):
                    yield table
            if include_dev:
                table = target.get("dev-dependencies")
                if isinstance(table, dict):
                    yield table


def dependency_package_name(key: str, value: Any) -> str:
    if isinstance(value, dict) and isinstance(value.get("package"), str):
        return value["package"]
    return key


def responsibility_clue(member: pathlib.Path, data: dict[str, Any]) -> str:
    """Read a short responsibility clue from manifest or crate-root docs."""
    description = data.get("package", {}).get("description")
    if isinstance(description, str) and description.strip():
        return " ".join(description.split())

    path = member / "src/lib.rs"
    if not path.exists():
        return "No package description or src/lib.rs crate-doc clue."
    lines = path.read_text(encoding="utf-8", errors="replace").splitlines()
    docs: list[str] = []
    for line in lines:
        stripped = line.strip()
        if stripped.startswith("//!"):
            docs.append(stripped[3:].strip())
            continue
        if docs and not stripped:
            break
        if docs:
            break
    clue = " ".join(part for part in docs if part)
    return clue or "No package description or crate-doc clue."


def declared_dependencies(data: dict[str, Any], package_names: set[str], *, include_dev: bool) -> set[str]:
    out: set[str] = set()
    for table in dependency_tables(data, include_dev=include_dev):
        for key, value in table.items():
            name = dependency_package_name(key, value)
            if name in package_names:
                out.add(name)
    return out


def dev_dependencies(data: dict[str, Any], package_names: set[str]) -> set[str]:
    all_deps = declared_dependencies(data, package_names, include_dev=True)
    normal = declared_dependencies(data, package_names, include_dev=False)
    return all_deps - normal


def git_files(*prefixes: str) -> list[pathlib.Path]:
    cmd = ["git", "-c", f"safe.directory={REPO}", "ls-files"]
    if prefixes:
        cmd.extend(["--", *prefixes])
    out = subprocess.run(cmd, cwd=REPO, text=True, capture_output=True, check=True).stdout
    return [REPO / line for line in out.splitlines() if line]


def rust_files_under(path: pathlib.Path) -> list[pathlib.Path]:
    return sorted(p for p in path.rglob("*.rs") if ".agent" not in p.parts and "target" not in p.parts)


def line_counts(path: pathlib.Path) -> tuple[int, int]:
    text = path.read_text(encoding="utf-8", errors="replace")
    lines = text.splitlines()
    return len(lines), sum(bool(line.strip()) for line in lines)


def is_test_file(path: pathlib.Path) -> bool:
    rel = path.relative_to(REPO)
    return "tests" in rel.parts or path.name == "tests.rs" or path.name.endswith("_tests.rs")


PUBLIC_ITEM = re.compile(
    r"(?m)^\s*pub(?:\([^)]*\))?\s+(?:async\s+|unsafe\s+|const\s+)?"
    r"(?:fn|struct|enum|trait|type|const|static|mod)\s+[A-Za-z_][A-Za-z0-9_]*"
)
PUBLIC_ROOT_SURFACE = re.compile(r"(?m)^\s*pub(?:\([^)]*\))?\s+(?:use|mod)\b")


def public_surface_clues(member: pathlib.Path, files: list[pathlib.Path]) -> tuple[int, int]:
    """Return source-text clues for the public API surface.

    These counts do not model visibility through macros, cfg values, or Rust
    re-export resolution. They are navigation metrics only.
    """
    public_items = 0
    for path in files:
        text = strip_cfg_test_tail(path.read_text(encoding="utf-8", errors="replace"))
        public_items += len(PUBLIC_ITEM.findall(text))

    root_surface = 0
    for name in ("lib.rs", "main.rs"):
        path = member / "src" / name
        if not path.exists():
            continue
        text = strip_cfg_test_tail(path.read_text(encoding="utf-8", errors="replace"))
        root_surface += len(PUBLIC_ROOT_SURFACE.findall(text))
    return public_items, root_surface


def crate_rows() -> list[CrateRow]:
    members = workspace_members()
    names = {package_name(member) for member in members}
    metadata: dict[str, tuple[pathlib.Path, dict[str, Any]]] = {}
    normal_graph: dict[str, set[str]] = {}
    dev_graph: dict[str, set[str]] = {}
    for member in members:
        data = load_toml(member / "Cargo.toml")
        name = str(data["package"]["name"])
        metadata[name] = (member, data)
        normal_graph[name] = declared_dependencies(data, names, include_dev=False)
        dev_graph[name] = dev_dependencies(data, names)

    reverse: dict[str, set[str]] = {name: set() for name in names}
    for owner, deps in normal_graph.items():
        for dep in deps:
            reverse[dep].add(owner)

    rows: list[CrateRow] = []
    for name in sorted(names):
        member, data = metadata[name]
        files = rust_files_under(member)
        raw = nonblank = test_loc = 0
        for path in files:
            file_raw, file_nonblank = line_counts(path)
            raw += file_raw
            nonblank += file_nonblank
            if is_test_file(path):
                test_loc += file_nonblank
        all_declared_names = set()
        for table in dependency_tables(data, include_dev=True):
            all_declared_names.update(dependency_package_name(k, v) for k, v in table.items())
        public_items, public_root_surface = public_surface_clues(member, files)
        rows.append(
            CrateRow(
                package=name,
                path=str(member.relative_to(REPO)),
                rust_files=len(files),
                raw_rust_loc=raw,
                nonblank_rust_loc=nonblank,
                test_rust_loc_heuristic=test_loc,
                direct_workspace_dependencies=tuple(sorted(normal_graph[name])),
                reverse_workspace_dependencies=tuple(sorted(reverse[name])),
                dev_workspace_dependencies=tuple(sorted(dev_graph[name])),
                direct_bevy_dependency=any(dep == "bevy" or dep.startswith("bevy_") for dep in all_declared_names),
                public_items_heuristic=public_items,
                public_root_reexports_or_modules_heuristic=public_root_surface,
                responsibility_clue=responsibility_clue(member, data),
            )
        )
    return rows


def strip_cfg_test_tail(text: str) -> str:
    """Remove the common file tail that starts with `#[cfg(test)]`.

    This is a conservative heuristic. A file can contain non-test code after a
    test module. The report labels pattern counts as heuristic for this reason.
    """
    return text.split("#[cfg(test)]", 1)[0]


def production_rust_files() -> list[pathlib.Path]:
    out: list[pathlib.Path] = []
    for root in ("crates", "game", "tools"):
        base = REPO / root
        if not base.exists():
            continue
        for path in base.rglob("*.rs"):
            if is_test_file(path):
                continue
            out.append(path)
    return sorted(out)


OPTIONAL_RES = re.compile(
    r"Option\s*<\s*(?P<kind>(?:[A-Za-z_][A-Za-z0-9_]*::)*Res(?:Mut)?)\s*<\s*(?P<type>[A-Za-z_][A-Za-z0-9_:]*)"
)
DERIVE_RESOURCE = re.compile(r"#\s*\[\s*derive\s*\([^\]]*\bResource\b[^\]]*\)\s*\][\s\S]{0,360}?\b(?:struct|enum)\s+(?P<name>[A-Za-z_][A-Za-z0-9_]*)")
DERIVE_COMPONENT = re.compile(r"#\s*\[\s*derive\s*\([^\]]*\bComponent\b[^\]]*\)\s*\][\s\S]{0,360}?\b(?:struct|enum)\s+(?P<name>[A-Za-z_][A-Za-z0-9_]*)")
MECHANICAL_DOMAIN = re.compile(r"MechanicalDomain::of::<(?P<type>[A-Za-z_][A-Za-z0-9_:]*)>\(\s*\"(?P<label>[^\"]+)\"\s*,?\s*\)")


def source_pattern_metrics() -> dict[str, Any]:
    optional_occurrences: list[dict[str, str]] = []
    resources: dict[str, set[str]] = collections.defaultdict(set)
    components: dict[str, set[str]] = collections.defaultdict(set)
    mechanical: set[tuple[str, str]] = set()
    before_after = 0
    fallback_hits = collections.Counter()
    fallback_terms = ("fallback", "legacy", "compat", "bridge", "adapter", "deprecated", "temporary")

    for path in production_rust_files():
        rel = str(path.relative_to(REPO))
        text = strip_cfg_test_tail(path.read_text(encoding="utf-8", errors="replace"))
        for match in OPTIONAL_RES.finditer(text):
            optional_occurrences.append({"path": rel, "kind": match.group("kind"), "type": match.group("type")})
        for match in DERIVE_RESOURCE.finditer(text):
            resources[match.group("name")].add(rel)
        for match in DERIVE_COMPONENT.finditer(text):
            components[match.group("name")].add(rel)
        for match in MECHANICAL_DOMAIN.finditer(text):
            marker = match.group("type")
            if marker not in {"DomainA", "DomainB", "FirstPlugin", "SecondPlugin", "OnlyPlugin", "FixtureDomain"}:
                mechanical.add((marker, match.group("label")))
        before_after += text.count(".before(") + text.count(".after(")
        lowered = text.lower()
        for term in fallback_terms:
            fallback_hits[term] += lowered.count(term)

    optional_types = sorted({row["type"].split("::")[-1] for row in optional_occurrences})
    return {
        "heuristic_resource_declarations": len(resources),
        "heuristic_component_declarations": len(components),
        "optional_res_occurrences": len(optional_occurrences),
        "optional_res_unique_types": len(optional_types),
        "optional_res_types": optional_types,
        "mechanical_editor_domains": [
            {"marker": marker, "label": label} for marker, label in sorted(mechanical, key=lambda item: item[1])
        ],
        "raw_before_after_edges": before_after,
        "fallback_term_hits": dict(sorted(fallback_hits.items())),
    }


def large_modules() -> list[dict[str, Any]]:
    rows = []
    for path in git_files("crates", "game", "tests", "tools"):
        if path.suffix != ".rs" or not path.exists():
            continue
        raw, nonblank = line_counts(path)
        if nonblank >= LARGE_MODULE_NONBLANK_LINES:
            rows.append({"path": str(path.relative_to(REPO)), "raw_lines": raw, "nonblank_lines": nonblank})
    return sorted(rows, key=lambda row: (-row["nonblank_lines"], row["path"]))


def explicit_narrow_lifetime_resources() -> dict[str, Any]:
    """Read source-owned lists of resources with a narrower semantic lifetime."""
    specs = [
        (
            REPO / "crates/ambition_platformer2d_actor_monolith/src/session/teardown.rs",
            "SessionScopedResources",
        ),
        (
            REPO / "crates/ambition_platformer2d_actor_monolith/src/session/checkpoint.rs",
            "SessionOwnedCheckpointState",
        ),
    ]
    found: dict[str, list[str]] = {}
    resmut = re.compile(r"ResMut\s*<\s*'?[A-Za-z_][A-Za-z0-9_]*\s*,\s*(?P<type>[A-Za-z_][A-Za-z0-9_:]*)\s*>")
    for path, struct_name in specs:
        text = path.read_text(encoding="utf-8", errors="replace")
        start = re.search(rf"pub\s+struct\s+{re.escape(struct_name)}\b[^{{]*\{{", text)
        if not start:
            found[struct_name] = []
            continue
        depth = 1
        i = start.end()
        while i < len(text) and depth:
            depth += (text[i] == "{") - (text[i] == "}")
            i += 1
        body = text[start.end(): i - 1]
        found[struct_name] = [m.group("type").split("::")[-1] for m in resmut.finditer(body)]
    # SessionMechanics is also a process resource whose source states that one
    # activated content generation owns it. It is removed at session retirement.
    all_types = sorted({ty for rows in found.values() for ty in rows} | {"SessionMechanics"})
    return {"groups": found, "unique_types_including_session_mechanics": all_types, "count": len(all_types)}


def ledger_metrics() -> dict[str, Any]:
    if not LEDGER.exists():
        return {"available": False, "reason": f"missing {LEDGER.relative_to(REPO)}"}
    data = json.loads(LEDGER.read_text(encoding="utf-8"))
    items = data.get("items", [])
    tags: collections.Counter[str] = collections.Counter()
    categories: collections.Counter[str] = collections.Counter()
    statuses: collections.Counter[str] = collections.Counter()
    complexities: collections.Counter[str] = collections.Counter()
    for item in items:
        categories[item.get("category", "UNCLASSIFIED")] += 1
        statuses[item.get("status", "UNCLASSIFIED")] += 1
        complexities[item.get("complexity", "UNCLASSIFIED")] += 1
        for tag in item.get("metric_tags", []):
            tags[tag] += 1
    return {
        "available": True,
        "items": len(items),
        "metric_tags": dict(sorted(tags.items())),
        "categories": dict(sorted(categories.items())),
        "statuses": dict(sorted(statuses.items())),
        "complexities": dict(sorted(complexities.items())),
    }


def agent_metrics() -> dict[str, Any]:
    path = REPO / ".agent/index/catalog.json"
    if not path.exists():
        return {"available": False}
    data = json.loads(path.read_text(encoding="utf-8"))
    return {
        "available": True,
        "generated_at": data.get("generated_at"),
        "generated_from_commit": data.get("generated_from_commit"),
        "counts": data.get("counts", {}),
    }


def report() -> dict[str, Any]:
    rows = crate_rows()
    workspace_rust_files = sum(row.rust_files for row in rows)
    raw_loc = sum(row.raw_rust_loc for row in rows)
    nonblank_loc = sum(row.nonblank_rust_loc for row in rows)
    test_loc = sum(row.test_rust_loc_heuristic for row in rows)
    repo_rs = [path for path in git_files() if path.suffix == ".rs"]
    return {
        "schema_version": 1,
        "method": {
            "compiler_used": False,
            "workspace_member_rule": "root Cargo.toml [workspace].members",
            "workspace_rust_file_rule": "*.rs under workspace member directories",
            "loc_rule": "physical Rust source lines; nonblank removes whitespace-only lines",
            "test_loc_rule": "nonblank Rust lines in tests/ paths, tests.rs, or *_tests.rs; inline cfg(test) modules in other files are not counted",
            "large_module_rule": f"tracked Rust file with at least {LARGE_MODULE_NONBLANK_LINES} nonblank lines; discovery metric only",
            "direct_dependency_rule": "declared workspace dependencies from dependencies/build-dependencies and target-specific equivalents; dev-dependencies are reported separately",
            "optional_res_rule": "source-text pattern for Option<Res<T>> and Option<ResMut<T>> outside obvious test files/tails; heuristic",
            "resource_component_rule": "source-text derive pattern; heuristic",
            "public_api_rule": "source-text count of public item declarations plus pub use/pub mod declarations at crate roots; heuristic and not a resolved Rust API",
            "responsibility_clue_rule": "Cargo package.description when present; otherwise the first src/lib.rs crate-doc paragraph",
            "ledger_metric_rule": "manual semantic census entries carry metric_tags; each tagged ledger item counts once for that metric",
        },
        "workspace": {
            "crates": len(rows),
            "rust_files": workspace_rust_files,
            "raw_rust_loc": raw_loc,
            "nonblank_rust_loc": nonblank_loc,
            "test_rust_loc_heuristic": test_loc,
            "repository_tracked_rust_files": len(repo_rs),
            "large_modules": large_modules(),
        },
        "crates": [asdict(row) for row in rows],
        "source_patterns": source_pattern_metrics(),
        "explicit_narrow_lifetime_resources": explicit_narrow_lifetime_resources(),
        "ledger": ledger_metrics(),
        "agent_inventory": agent_metrics(),
    }


def print_summary(data: dict[str, Any]) -> None:
    workspace = data["workspace"]
    patterns = data["source_patterns"]
    narrow = data["explicit_narrow_lifetime_resources"]
    print("Architecture census static measurements")
    print("compiler used: no")
    print(f"workspace crates: {workspace['crates']}")
    print(f"workspace Rust files: {workspace['rust_files']}")
    print(f"workspace raw Rust LOC: {workspace['raw_rust_loc']}")
    print(f"workspace nonblank Rust LOC: {workspace['nonblank_rust_loc']}")
    print(f"workspace test LOC (heuristic): {workspace['test_rust_loc_heuristic']}")
    print(f"repository tracked Rust files: {workspace['repository_tracked_rust_files']}")
    print(f"large Rust modules >= {LARGE_MODULE_NONBLANK_LINES} nonblank lines: {len(workspace['large_modules'])}")
    print(f"optional Res/ResMut occurrences (heuristic): {patterns['optional_res_occurrences']}")
    print(f"optional Res/ResMut unique types (heuristic): {patterns['optional_res_unique_types']}")
    print(f"mechanical editor domains found in production source: {len(patterns['mechanical_editor_domains'])}")
    print(f"explicit process resources with session/generation semantics: {narrow['count']}")
    ledger = data["ledger"]
    if ledger.get("available"):
        print("manual ledger metric tags:")
        for tag, count in ledger["metric_tags"].items():
            print(f"  {tag}: {count}")


def print_crate_table(data: dict[str, Any]) -> None:
    print(
        "package\tnonblank_loc\trust_files\tdirect_workspace_deps\t"
        "reverse_workspace_deps\tbevy\tpublic_items_heuristic\t"
        "root_pub_use_or_mod_heuristic\tdirect_dependencies\treverse_dependencies\tresponsibility_clue"
    )
    for row in sorted(data["crates"], key=lambda r: (-r["nonblank_rust_loc"], r["package"])):
        direct = ",".join(row["direct_workspace_dependencies"])
        reverse = ",".join(row["reverse_workspace_dependencies"])
        clue = row["responsibility_clue"].replace("\t", " ").replace("\n", " ")
        print(
            f"{row['package']}\t{row['nonblank_rust_loc']}\t{row['rust_files']}\t"
            f"{len(row['direct_workspace_dependencies'])}\t{len(row['reverse_workspace_dependencies'])}\t"
            f"{'yes' if row['direct_bevy_dependency'] else 'no'}\t{row['public_items_heuristic']}\t"
            f"{row['public_root_reexports_or_modules_heuristic']}\t{direct}\t{reverse}\t{clue}"
        )


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--json", action="store_true", help="print the full report as JSON")
    parser.add_argument("--crate-table", action="store_true", help="print one TSV row per workspace crate")
    args = parser.parse_args(argv)
    data = report()
    if args.json:
        json.dump(data, sys.stdout, indent=2, sort_keys=True)
        print()
    elif args.crate_table:
        print_crate_table(data)
    else:
        print_summary(data)
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
