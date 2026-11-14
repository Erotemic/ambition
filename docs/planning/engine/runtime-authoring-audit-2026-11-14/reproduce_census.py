#!/usr/bin/env python3
"""Reproduce the bounded source and manifest census for the static audit.

Run with Python 3.11 or later. This tool reads files only and writes the selected
JSON report. It does not run Cargo, compile, test, or alter engine source.

Raw source line counts include comments, blank lines, and inline test modules.
The manifest graphs are not feature-resolved consumer build graphs. See the
method field in the JSON output for the exact graph limits.
"""

from __future__ import annotations

import argparse
import collections
import json
from pathlib import Path
import re
import tomllib


def reachable(graph: dict[str, set[str]], start: str) -> set[str]:
    seen: set[str] = set()
    pending = list(graph.get(start, ()))
    while pending:
        name = pending.pop()
        if name in seen:
            continue
        seen.add(name)
        pending.extend(graph.get(name, ()))
    return seen


def census(root: Path, snapshot: str | None) -> dict:
    workspace = tomllib.loads((root / "Cargo.toml").read_text())
    packages = {}
    for member in workspace["workspace"]["members"]:
        manifest_path = root / member / "Cargo.toml"
        data = tomllib.loads(manifest_path.read_text())
        name = data["package"]["name"]
        files = sorted((manifest_path.parent / "src").rglob("*.rs"))
        rows = []
        for path in files:
            lines = path.read_text().splitlines()
            rows.append({
                "path": str(path.relative_to(root)),
                "lines": len(lines),
                "nonblank_lines": sum(bool(line.strip()) for line in lines),
            })
        tests = sorted((manifest_path.parent / "tests").rglob("*.rs"))
        packages[name] = {
            "path": member,
            "manifest": data,
            "source_files": len(files),
            "source_lines": sum(row["lines"] for row in rows),
            "source_nonblank_lines": sum(row["nonblank_lines"] for row in rows),
            "integration_test_files": len(tests),
            "integration_test_lines": sum(len(path.read_text().splitlines()) for path in tests),
            "files": rows,
        }
    path_names = {(root / data["path"]).resolve(): name for name, data in packages.items()}
    edges = []
    kinds = ("dependencies", "build-dependencies", "dev-dependencies")
    for name, data in packages.items():
        manifest = data["manifest"]
        sections = [(None, kind, manifest.get(kind, {})) for kind in kinds]
        for target, target_manifest in manifest.get("target", {}).items():
            sections.extend((target, kind, target_manifest.get(kind, {})) for kind in kinds)
        for target, kind, dependencies in sections:
            for alias, spec in dependencies.items():
                if isinstance(spec, str):
                    spec = {"version": spec}
                dep_name = spec.get("package", alias)
                if "path" in spec:
                    path = (root / data["path"] / spec["path"]).resolve()
                    dep_name = path_names.get(path, dep_name)
                edges.append({
                    "from": name,
                    "to": dep_name,
                    "kind": kind,
                    "target": target,
                    "optional": bool(spec.get("optional", False)),
                    "internal": dep_name in packages,
                    "features": spec.get("features", []),
                    "default_features": spec.get("default-features", True),
                })
    mandatory: dict[str, set[str]] = collections.defaultdict(set)
    declared: dict[str, set[str]] = collections.defaultdict(set)
    for edge in edges:
        if edge["internal"] and edge["kind"] == "dependencies":
            declared[edge["from"]].add(edge["to"])
            if edge["target"] is None and not edge["optional"]:
                mandatory[edge["from"]].add(edge["to"])
    reverse: dict[str, set[str]] = collections.defaultdict(set)
    for owner, dependencies in declared.items():
        for dependency in dependencies:
            reverse[dependency].add(owner)
    summaries = []
    for name, data in packages.items():
        summaries.append({
            "name": name,
            "path": data["path"],
            "source_files": data["source_files"],
            "source_lines": data["source_lines"],
            "source_nonblank_lines": data["source_nonblank_lines"],
            "integration_test_files": data["integration_test_files"],
            "integration_test_lines": data["integration_test_lines"],
            "declared_normal_internal_dependencies": len(declared[name]),
            "mandatory_target_independent_normal_closure_size_including_self": len(reachable(mandatory, name) | {name}),
            "declared_normal_internal_reverse_reach": len(reachable(reverse, name) - {name}),
        })
    declared_index_path = root / ".agent/index/crates/graph-declared.json"
    resolved_index_path = root / ".agent/index/crates/graph-resolved.json"
    declared_index = json.loads(declared_index_path.read_text()) if declared_index_path.exists() else None
    resolved_index = json.loads(resolved_index_path.read_text()) if resolved_index_path.exists() else None
    if snapshot is None and resolved_index:
        snapshot = resolved_index.get("generated_from_commit")
    if snapshot is None and (root / ".agent/README.md").exists():
        match = re.search(r"Source commit: `([^`]+)`", (root / ".agent/README.md").read_text())
        snapshot = match.group(1) if match else None
    selected = [
        "ambition_platformer2d", "ambition_platformer2d_actor_monolith",
        "ambition_platformer2d_runtime", "ambition_engine_schemas",
        "ambition_content_cli", "ambition_content_pack", "ambition_extension_sdk",
        "ambition_content_modules",
    ]
    return {
        "snapshot": snapshot,
        "method": (
            "Parse root workspace member Cargo.toml with Python tomllib. Count raw .rs source "
            "under each member/src, including inline test modules, comments and blanks. Separate "
            "integration tests under member/tests. Graph is declared normal internal edges, not "
            "a Cargo feature-resolved consumer closure; optional edges and all target tables "
            "retained. Mandatory closure traverses only nonoptional target-independent normal "
            "edges and ignores default features, so it is an incompleteness proof/lower bound, "
            "not a full resolved closure. No builds or cargo execution."
        ),
        "workspace_member_count": len(packages),
        "source_files": sum(row["source_files"] for row in summaries),
        "source_lines": sum(row["source_lines"] for row in summaries),
        "source_nonblank_lines": sum(row["source_nonblank_lines"] for row in summaries),
        "integration_test_files": sum(row["integration_test_files"] for row in summaries),
        "integration_test_lines": sum(row["integration_test_lines"] for row in summaries),
        "missing_from_generated_declared_index": sorted(set(packages) - set(declared_index["members"])) if declared_index else None,
        "generated_declared_counts": declared_index["counts"] if declared_index else None,
        "generated_resolved_metadata": {key: resolved_index[key] for key in ("available", "generated_from_commit", "counts") if key in resolved_index} if resolved_index else None,
        "edge_counts": dict(collections.Counter(edge["kind"] + ":" + ("internal" if edge["internal"] else "external") for edge in edges)),
        "packages": sorted(summaries, key=lambda row: (-row["source_lines"], row["name"])),
        "largest_source_files": sorted([row for data in packages.values() for row in data["files"]], key=lambda row: (-row["lines"], row["path"]))[:50],
        "selected_mandatory_closures": {name: sorted(reachable(mandatory, name) | {name}) for name in selected if name in packages},
        "manifest_edges": edges,
    }


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, required=True, help="Ambition source root with Cargo.toml")
    parser.add_argument("--output", type=Path, required=True, help="Destination JSON file")
    parser.add_argument("--snapshot", help="Optional source revision; otherwise use generated index metadata")
    args = parser.parse_args()
    result = census(args.root.resolve(), args.snapshot)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, indent=2) + "\n")
    print(json.dumps({
        "output": str(args.output.resolve()),
        "workspace_member_count": result["workspace_member_count"],
        "source_files": result["source_files"],
        "source_lines": result["source_lines"],
        "integration_test_files": result["integration_test_files"],
        "integration_test_lines": result["integration_test_lines"],
    }, indent=2))


if __name__ == "__main__":
    main()
