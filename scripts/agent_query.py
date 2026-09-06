#!/usr/bin/env python3
"""Query Ambition's generated agent-navigation bundle.

The command is intentionally dependency-free and works inside source archives.
Run ``python scripts/agent_query.py build-catalog`` after regenerating the raw
indexes and ECS inventory. ``scripts/archive_agent_source.py`` does this
automatically for packaged agent archives.

Examples:
    python scripts/agent_query.py "room transition loading"
    python scripts/agent_query.py symbol GroundContactTransition
    python scripts/agent_query.py ecs "room transition" --crate ambition_app
    python scripts/agent_query.py crate ambition_platformer2d_runtime
    python scripts/agent_query.py deps ambition_audio
    python scripts/agent_query.py graph ambition_audio --format svg
    python scripts/agent_query.py build-catalog

Everything works with the standard library alone. The one exception is drawing:
``graph`` writes graphviz source unaided, and rendering that source to a picture
shells out to ``dot`` only when you ask for a picture format.
"""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
import textwrap
import tomllib
from collections import defaultdict
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Iterable, Mapping, Sequence

ROOT = Path(__file__).resolve().parents[1]
AGENT_DIR = ROOT / ".agent"
INDEX_DIR = AGENT_DIR / "index"
ECS_DIR = AGENT_DIR / "ecs_inventory"
CATALOG_PATH = INDEX_DIR / "catalog.json"
CRATE_INDEX_PATH = INDEX_DIR / "crates" / "index.json"
DECLARED_GRAPH_PATH = INDEX_DIR / "crates" / "graph-declared.json"
RESOLVED_GRAPH_PATH = INDEX_DIR / "crates" / "graph-resolved.json"
AGENT_README_PATH = AGENT_DIR / "README.md"
SCHEMA_VERSION = 1

KNOWN_COMMANDS = {
    "overview",
    "task",
    "symbol",
    "docs",
    "ecs",
    "tests",
    "crate",
    "deps",
    "graph",
    "path",
    "build-catalog",
}


@dataclass(frozen=True)
class DeclaredDep:
    """One edge as a manifest DECLARES it. See `build_declared_graph`."""

    name: str
    kind: str  # normal | dev | build
    optional: bool
    internal: bool
    target: str | None  # the `[target.'cfg(...)'.dependencies]` key, if any
    enabled_by: tuple[str, ...]  # features that turn an optional edge on

    def to_json(self) -> dict[str, Any]:
        row: dict[str, Any] = {
            "name": self.name,
            "kind": self.kind,
            "optional": self.optional,
            "internal": self.internal,
        }
        if self.target:
            row["target"] = self.target
        if self.enabled_by:
            row["enabled_by"] = list(self.enabled_by)
        return row


@dataclass(frozen=True)
class CrateInfo:
    name: str
    root: str
    manifest: str
    module_map: str | None
    declared_deps: tuple[DeclaredDep, ...] = ()


def load_json(path: Path, default: Any = None) -> Any:
    if not path.exists():
        return default
    return json.loads(path.read_text(encoding="utf-8"))


def write_json(path: Path, data: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(data, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def simple_manifest_value(key: str) -> str | None:
    path = AGENT_DIR / "manifest.yaml"
    if not path.exists():
        return None
    pattern = re.compile(rf"^{re.escape(key)}:\s*[\"']?([^\"'\n]+)", re.MULTILINE)
    match = pattern.search(path.read_text(encoding="utf-8", errors="replace"))
    return match.group(1).strip() if match else None


def generation_stamp() -> dict:
    """Machine-local provenance written by generate_agent_index.py."""
    path = ROOT / ".agent" / "index" / "generation_stamp.json"
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        return {}


def _missing_indexed_files(limit: int = 4000) -> tuple[int, int]:
    """(missing, checked) over the paths the index claims to describe.

    ⭐ **the freshness check for a reader with no git**, which is the archive
    case this whole bundle exists for. It is a `stat` per path and no reads, so
    it costs milliseconds; it detects deletions and renames — precisely the
    drift that produces a confidently-wrong owner — and deliberately does NOT
    detect edits or new files, which is why it reports rather than concludes.
    """
    files = load_json(INDEX_DIR / "file_summaries.json", {}) or {}
    rows = files.get("files")
    if not isinstance(rows, list) or not rows:
        return (0, 0)
    checked = 0
    missing = 0
    for row in rows[:limit]:
        path = row.get("path") if isinstance(row, dict) else None
        if not isinstance(path, str):
            continue
        checked += 1
        if not (ROOT / path).exists():
            missing += 1
    return (missing, checked)


def warn_if_index_stale() -> None:
    """Loudly flag a stale local index — silent staleness has produced
    confidently-wrong owners (deleted files ranked above live ones).

    ⚠ **two checks, because the first one can decline to answer.** The commit
    comparison is exact and is the right answer whenever git is present. Where
    it is not — a source archive published without history, which is the
    environment this bundle is built for — it returns nothing, and a check that
    returns nothing is indistinguishable from a check that passed. So absence of
    git falls through to a content check rather than to silence.
    """
    stamp = generation_stamp()
    stamp_commit = stamp.get("generated_from_commit")
    if not stamp_commit or stamp_commit == "unknown":
        print(
            "⚠ index has no generation stamp — run: python scripts/generate_agent_index.py",
            file=sys.stderr,
        )
        return
    if stamp.get("worktree_dirty"):
        print(
            f"⚠ index was generated from a DIRTY worktree at {stamp_commit} —"
            " the commit id is not the whole story",
            file=sys.stderr,
        )
    head = ""
    try:
        head = subprocess.run(
            ["git", "rev-parse", "--short=12", "HEAD"],
            cwd=ROOT,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.DEVNULL,
            check=False,
        ).stdout.strip()
    except OSError:
        head = ""

    if head and head != stamp_commit:
        count = ""
        try:
            count = subprocess.run(
                ["git", "rev-list", "--count", f"{stamp_commit}..HEAD"],
                cwd=ROOT,
                text=True,
                stdout=subprocess.PIPE,
                stderr=subprocess.DEVNULL,
                check=False,
            ).stdout.strip()
        except OSError:
            count = ""
        behind = f"{count} commits " if count else ""
        print(
            f"⚠ index generated at {stamp_commit}, {behind}behind HEAD {head}"
            " — run: python scripts/generate_agent_index.py",
            file=sys.stderr,
        )
        return
    if head:
        return

    # No git. Say so, and give the reader the check that does not need it.
    missing, checked = _missing_indexed_files()
    if missing:
        print(
            f"⚠ no git here, and {missing} of {checked} indexed files are absent from this"
            f" tree — the index (stamped {stamp_commit}) does not describe it",
            file=sys.stderr,
        )
    else:
        print(
            f"note: index stamped {stamp_commit}; freshness unverifiable without git."
            f" {checked} indexed files all still present.",
            file=sys.stderr,
        )


def source_commit() -> str:
    stamp_commit = generation_stamp().get("generated_from_commit")
    if stamp_commit and stamp_commit != "unknown":
        return stamp_commit
    manifest_commit = simple_manifest_value("generated_from_commit")
    if manifest_commit:
        return manifest_commit
    try:
        proc = subprocess.run(
            ["git", "rev-parse", "--short=12", "HEAD"],
            cwd=ROOT,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.DEVNULL,
            check=False,
        )
    except OSError:
        return "unknown"
    return proc.stdout.strip() if proc.returncode == 0 and proc.stdout.strip() else "unknown"


def read_manifest(manifest: Path) -> dict[str, Any] | None:
    try:
        return tomllib.loads(manifest.read_text(encoding="utf-8"))
    except (OSError, tomllib.TOMLDecodeError):
        return None


def package_name(manifest: Path) -> str | None:
    data = read_manifest(manifest)
    if data is None:
        return None
    package = data.get("package")
    if not isinstance(package, dict):
        return None
    value = package.get("name")
    return value if isinstance(value, str) else None


# `[features]` entries that switch an optional dependency on. Cargo spells this
# three ways and all three appear in this workspace, so all three are read:
#   "dep:ambition_x"            explicit, and does NOT imply a feature of x
#   "ambition_x/feat"           enables x AND its `feat` (unless `?/`)
#   "ambition_x?/feat"          enables x's `feat` only IF x is already on
_FEATURE_DEP_RE = re.compile(r"^(?:dep:)?([A-Za-z0-9_.-]+)(\?)?(?:/.*)?$")


def _features_that_enable(features: Any) -> dict[str, set[str]]:
    """dep name -> the feature names that turn it on.

    Only meaningful for OPTIONAL dependencies; a required dep is always on and
    its feature mentions say nothing about whether it is linked.
    """
    enabled_by: dict[str, set[str]] = defaultdict(set)
    if not isinstance(features, dict):
        return enabled_by
    for feature, entries in features.items():
        if not isinstance(entries, list):
            continue
        for entry in entries:
            if not isinstance(entry, str):
                continue
            match = _FEATURE_DEP_RE.match(entry.strip())
            if not match:
                continue
            dep, weak = match.group(1), match.group(2)
            # `x?/feat` is explicitly NOT an enabler — that is the whole point
            # of the `?` sigil, and reading it as one would report an optional
            # edge as reachable when nothing turns it on.
            if weak:
                continue
            enabled_by[dep].add(str(feature))
    return enabled_by


def _dependency_tables(data: dict[str, Any]) -> list[tuple[str, str | None, dict[str, Any]]]:
    """(kind, target, table) for every dependency table in one manifest."""
    kinds = {
        "dependencies": "normal",
        "dev-dependencies": "dev",
        "build-dependencies": "build",
    }
    out: list[tuple[str, str | None, dict[str, Any]]] = []
    for key, kind in kinds.items():
        table = data.get(key)
        if isinstance(table, dict):
            out.append((kind, None, table))
    # Platform-specific tables are real edges — a consumer on that platform
    # links them — and omitting them would under-report the graph on exactly
    # the platforms where it is hardest to check by hand.
    targets = data.get("target")
    if isinstance(targets, dict):
        for target, spec in targets.items():
            if not isinstance(spec, dict):
                continue
            for key, kind in kinds.items():
                table = spec.get(key)
                if isinstance(table, dict):
                    out.append((kind, str(target), table))
    return out


def declared_deps(manifest: Path, workspace_names: set[str]) -> tuple[DeclaredDep, ...]:
    """Every edge this manifest DECLARES, with the optional/feature facts.

    ⚠ declared is not linked. An `optional = true` edge that no enabled feature
    turns on costs a consumer nothing, and this workspace has such edges — so a
    reader that treats this as the link graph will overstate what a game pays
    for. `graph-resolved.json` is the other half; see `build_declared_graph`.
    """
    data = read_manifest(manifest)
    if data is None:
        return ()
    enablers = _features_that_enable(data.get("features"))
    out: list[DeclaredDep] = []
    for kind, target, table in _dependency_tables(data):
        for name, spec in table.items():
            # `package = "..."` renames: the edge is to the real package.
            real = name
            optional = False
            if isinstance(spec, dict):
                renamed = spec.get("package")
                if isinstance(renamed, str):
                    real = renamed
                optional = bool(spec.get("optional", False))
            out.append(
                DeclaredDep(
                    name=real,
                    kind=kind,
                    optional=optional,
                    internal=real in workspace_names,
                    target=target,
                    enabled_by=tuple(sorted(enablers.get(name, set()))) if optional else (),
                )
            )
    return tuple(sorted(out, key=lambda dep: (dep.name, dep.kind, dep.target or "")))


def discover_crates() -> list[CrateInfo]:
    entry_points = load_json(INDEX_DIR / "entry_points.json", {}) or {}
    module_by_root: dict[str, str] = {}
    for row in entry_points.get("module_maps", []):
        path = str(row.get("path", ""))
        if path.endswith("/MODULES.md"):
            module_by_root[path.removesuffix("/MODULES.md")] = path

    # Two passes: `internal` on an edge means "points at a workspace member",
    # which cannot be decided until every member is known.
    found: list[tuple[str, Path]] = []
    for parent in ("crates", "game", "tests"):
        base = ROOT / parent
        if not base.exists():
            continue
        for manifest in sorted(base.glob("*/Cargo.toml")):
            name = package_name(manifest)
            if name is not None:
                found.append((name, manifest))
    workspace_names = {name for name, _ in found}

    roots: list[CrateInfo] = []
    for name, manifest in found:
        root = manifest.parent.relative_to(ROOT).as_posix()
        roots.append(
            CrateInfo(
                name=name,
                root=root,
                manifest=manifest.relative_to(ROOT).as_posix(),
                module_map=module_by_root.get(root),
                declared_deps=declared_deps(manifest, workspace_names),
            )
        )
    return sorted(roots, key=lambda item: item.name)


# The two graphs answer two different questions, and conflating them has
# already cost this repo a measurement. Stated once, here, and repeated inside
# each generated file so a reader who opens only the JSON still gets it.
DECLARED_GRAPH_MEANING = (
    "What every workspace manifest DECLARES, parsed from Cargo.toml with the "
    "Python standard library alone. Includes optional edges and names the "
    "features that enable them. This is NOT what a consumer links: an "
    "`optional = true` edge that no enabled feature turns on costs nothing, "
    "and this workspace has such edges (ambition_ui_nav in the actor monolith "
    "is one). Use this to answer 'who names whom'."
)
RESOLVED_GRAPH_MEANING = (
    "What cargo RESOLVES for this workspace, from `cargo metadata` — feature "
    "unification applied, so every edge here is actually compiled. Requires a "
    "Rust toolchain at GENERATION time, not at read time. Use this to answer "
    "'what is actually linked'. ⚠ still workspace-wide: a specific consumer's "
    "closure is a different question, measured per-consumer with "
    "`cargo tree --edges normal` in that consumer's own workspace (see "
    "scripts/baselines/capability-footprint-baseline.json)."
)


def packet_prune_allowlist(crates: Sequence[CrateInfo]) -> set[str]:
    """Every file allowed to remain in `.agent/index/crates/`.

    ⛔ **`build_catalog` DELETES everything here it does not name**, which is
    right — a renamed crate used to leave its packet behind forever and an agent
    looking it up found a confident description of something gone.

    But the directory holds three files that are NOT per-crate packets, and one
    of them (`graph-resolved.json`) is written by a DIFFERENT generator that
    needs cargo. Forgetting it would delete it on every catalog build, and the
    only symptom would be a resolved graph that is mysteriously always missing.
    This is a named function so that invariant has somewhere to be tested.
    """
    return {f"{crate.name}.json" for crate in crates} | {
        CRATE_INDEX_PATH.name,
        "_repository.json",
        DECLARED_GRAPH_PATH.name,
        RESOLVED_GRAPH_PATH.name,
    }


def build_declared_graph(crates: Sequence[CrateInfo]) -> dict[str, Any]:
    """The manifest-declared edge graph, with reverse edges.

    Reverse edges are stored rather than derived because "who depends on X" is
    the question that actually gets asked, and making every reader write the
    inversion is how two readers end up disagreeing about it.
    """
    members = sorted(crate.name for crate in crates)
    edges: dict[str, list[dict[str, Any]]] = {}
    reverse: dict[str, set[str]] = {name: set() for name in members}
    for crate in crates:
        rows = [dep.to_json() for dep in crate.declared_deps]
        edges[crate.name] = rows
        for dep in crate.declared_deps:
            if dep.internal:
                reverse[dep.name].add(crate.name)
    internal_edges = sum(
        1 for crate in crates for dep in crate.declared_deps if dep.internal
    )
    return {
        "schema_version": SCHEMA_VERSION,
        "generator": "scripts/agent_query.py build-catalog",
        "graph": "declared",
        "means": DECLARED_GRAPH_MEANING,
        "requires_toolchain": False,
        "counts": {
            "members": len(members),
            "declared_edges": sum(len(rows) for rows in edges.values()),
            "internal_edges": internal_edges,
        },
        "members": members,
        "edges": edges,
        "reverse_edges": {name: sorted(callers) for name, callers in reverse.items()},
    }


def owner_for_path(path: str, crates: Sequence[CrateInfo]) -> CrateInfo | None:
    matches = [crate for crate in crates if path == crate.root or path.startswith(crate.root + "/")]
    if not matches:
        return None
    return max(matches, key=lambda item: len(item.root))


def record_count(data: dict[str, Any], key: str) -> int:
    value = data.get(key, [])
    return len(value) if isinstance(value, list) else 0


def build_catalog(*, quiet: bool = False) -> dict[str, Any]:
    files_data = load_json(INDEX_DIR / "file_summaries.json", {}) or {}
    symbols_data = load_json(INDEX_DIR / "symbol_index.json", {}) or {}
    tests_data = load_json(INDEX_DIR / "test_map.json", {}) or {}
    entry_points = load_json(INDEX_DIR / "entry_points.json", {}) or {}
    planning = load_json(INDEX_DIR / "planning_index.json", {}) or {}
    concepts = load_json(INDEX_DIR / "concept_index.json", {}) or {}
    adrs = load_json(INDEX_DIR / "adr_index.json", {}) or {}
    tools = load_json(INDEX_DIR / "tool_index.json", {}) or {}
    archive = load_json(INDEX_DIR / "archive_index.json", {}) or {}
    ecs_project = load_json(ECS_DIR / "project.json", {}) or {}

    required = {
        "file_summaries": files_data.get("files"),
        "symbol_index": symbols_data.get("symbols"),
        "test_map": tests_data.get("tests"),
    }
    missing = [name for name, value in required.items() if not isinstance(value, list)]
    if missing:
        raise SystemExit(
            "missing generated indexes: "
            + ", ".join(missing)
            + ". Run `python scripts/generate_agent_index.py` first."
        )

    crates = discover_crates()
    grouped_files: dict[str, list[dict[str, Any]]] = defaultdict(list)
    grouped_symbols: dict[str, list[dict[str, Any]]] = defaultdict(list)
    grouped_tests: dict[str, list[dict[str, Any]]] = defaultdict(list)

    for row in files_data["files"]:
        owner = owner_for_path(str(row.get("path", "")), crates)
        grouped_files[owner.name if owner else "_repository"].append(row)
    for row in symbols_data["symbols"]:
        owner = owner_for_path(str(row.get("path", "")), crates)
        grouped_symbols[owner.name if owner else "_repository"].append(row)
    for row in tests_data["tests"]:
        owner = owner_for_path(str(row.get("path", "")), crates)
        grouped_tests[owner.name if owner else "_repository"].append(row)

    ecs_by_name: dict[str, dict[str, Any]] = {}
    for row in ecs_project.get("crates", []):
        if isinstance(row, dict) and isinstance(row.get("crate_name"), str):
            ecs_by_name[row["crate_name"]] = row

    crate_rows: list[dict[str, Any]] = []
    crate_dir = INDEX_DIR / "crates"
    crate_dir.mkdir(parents=True, exist_ok=True)

    # **Prune packets for crates that no longer exist.**
    #
    # this is the regen invariant, not tidiness: regenerating in a FRESH CLONE
    # would not produce these files, so their presence made the index depend on
    # the history of the machine it was built on. `index.json` is the catalog
    # itself and is written below, not by this loop.
    # `index.json` is the catalog and `_repository.json` is the packet for files
    # no crate owns. Both are written further down in this same pass, so pruning
    # them would "work" — and would break the moment somebody splits this
    # function. Naming them is the difference between correct and lucky.
    live = packet_prune_allowlist(crates)
    for stale in sorted(crate_dir.glob("*.json")):
        if stale.name not in live:
            stale.unlink()

    for crate in crates:
        ecs_summary = ecs_by_name.get(crate.name, {})
        ecs_json = None
        ecs_markdown = None
        ecs_counts: dict[str, Any] = {}
        if ecs_summary:
            ecs_json = f".agent/ecs_inventory/{ecs_summary.get('json')}"
            ecs_markdown = f".agent/ecs_inventory/{ecs_summary.get('markdown')}"
            counts = ecs_summary.get("counts")
            if isinstance(counts, dict):
                ecs_counts = counts

        packet_rel = f".agent/index/crates/{crate.name}.json"
        packet = {
            "schema_version": SCHEMA_VERSION,
            "generator": "scripts/agent_query.py build-catalog",
            "crate_name": crate.name,
            "crate_root": crate.root,
            "manifest": crate.manifest,
            "module_map": crate.module_map,
            # Declared edges only — the resolved graph is a separate file
            # because it needs a toolchain. See DECLARED_GRAPH_MEANING.
            "declared_deps": [dep.to_json() for dep in crate.declared_deps],
            "summary": {
                "files": len(grouped_files[crate.name]),
                "symbols": len(grouped_symbols[crate.name]),
                "tests": len(grouped_tests[crate.name]),
                "ecs": ecs_counts,
            },
            "files": sorted(grouped_files[crate.name], key=lambda row: str(row.get("path", ""))),
            "symbols": sorted(
                grouped_symbols[crate.name],
                key=lambda row: (str(row.get("name", "")), str(row.get("path", "")), int(row.get("line", 0))),
            ),
            "tests": sorted(
                grouped_tests[crate.name],
                key=lambda row: (str(row.get("name", "")), str(row.get("path", "")), int(row.get("line", 0))),
            ),
            "ecs_inventory": {
                "markdown": ecs_markdown,
                "json": ecs_json,
            },
        }
        write_json(ROOT / packet_rel, packet)
        crate_rows.append(
            {
                "crate_name": crate.name,
                "crate_root": crate.root,
                "packet": packet_rel,
                "module_map": crate.module_map,
                "ecs_markdown": ecs_markdown,
                "counts": packet["summary"],
            }
        )

    repository_packet = {
        "schema_version": SCHEMA_VERSION,
        "generator": "scripts/agent_query.py build-catalog",
        "crate_name": "_repository",
        "crate_root": ".",
        "summary": {
            "files": len(grouped_files["_repository"]),
            "symbols": len(grouped_symbols["_repository"]),
            "tests": len(grouped_tests["_repository"]),
        },
        "files": sorted(grouped_files["_repository"], key=lambda row: str(row.get("path", ""))),
        "symbols": sorted(grouped_symbols["_repository"], key=lambda row: str(row.get("name", ""))),
        "tests": sorted(grouped_tests["_repository"], key=lambda row: str(row.get("name", ""))),
    }
    write_json(crate_dir / "_repository.json", repository_packet)

    write_json(DECLARED_GRAPH_PATH, build_declared_graph(crates))

    crate_index = {
        "schema_version": SCHEMA_VERSION,
        "generator": "scripts/agent_query.py build-catalog",
        "crates": crate_rows,
        "repository_packet": ".agent/index/crates/_repository.json",
        "declared_graph": ".agent/index/crates/graph-declared.json",
        "resolved_graph": ".agent/index/crates/graph-resolved.json",
    }
    write_json(CRATE_INDEX_PATH, crate_index)

    catalog = {
        "schema_version": SCHEMA_VERSION,
        "generator": "scripts/agent_query.py build-catalog",
        "generated_from_commit": source_commit(),
        "generated_at": generation_stamp().get("generated_at")
        or simple_manifest_value("generated_at"),
        "counts": {
            "files": record_count(files_data, "files"),
            "symbols": record_count(symbols_data, "symbols"),
            "tests": record_count(tests_data, "tests"),
            "planning_docs": record_count(planning, "planning_docs"),
            "concepts": record_count(concepts, "concepts"),
            "adrs": record_count(adrs, "adrs"),
            "tools": record_count(tools, "tools"),
            "archive_docs": record_count(archive, "archive_docs"),
            "crates": len(crate_rows),
            "ecs": ecs_project.get("counts", {}),
        },
        "entry_points": entry_points.get("start_here", []),
        "indexes": {
            "crate_index": ".agent/index/crates/index.json",
            "declared_dependency_graph": ".agent/index/crates/graph-declared.json",
            "resolved_dependency_graph": ".agent/index/crates/graph-resolved.json",
            "file_summaries": ".agent/index/file_summaries.json",
            "symbols": ".agent/index/symbol_index.json",
            "tests": ".agent/index/test_map.json",
            "planning": ".agent/index/planning_index.json",
            "concepts": ".agent/index/concept_index.json",
            "adrs": ".agent/index/adr_index.json",
            "tools": ".agent/index/tool_index.json",
            "archive": ".agent/index/archive_index.json",
            "ecs_project_markdown": ".agent/ecs_inventory/project.md",
            "ecs_project_json": ".agent/ecs_inventory/project.json",
        },
        "query_examples": [
            'python scripts/agent_query.py "room transition loading"',
            "python scripts/agent_query.py symbol GroundContactTransition",
            'python scripts/agent_query.py ecs "room transition" --crate ambition_app',
            "python scripts/agent_query.py crate ambition_platformer2d_runtime",
        ],
    }
    write_json(CATALOG_PATH, catalog)
    AGENT_README_PATH.write_text(render_agent_readme(catalog), encoding="utf-8")

    if not quiet:
        print(f"generated {CATALOG_PATH.relative_to(ROOT)}")
        print(f"generated {CRATE_INDEX_PATH.relative_to(ROOT)} and {len(crate_rows)} crate packets")
        print_output_location(CATALOG_PATH)
    return catalog


def render_agent_readme(catalog: dict[str, Any]) -> str:
    counts = catalog.get("counts", {})
    ecs = counts.get("ecs", {}) if isinstance(counts.get("ecs"), dict) else {}
    commit = catalog.get("generated_from_commit") or "unknown"
    generated_at = catalog.get("generated_at") or "unknown"
    return f"""# Generated agent navigation bundle

This directory describes the committed source snapshot packaged for an agent.
It is generated navigation data, not architectural authority.

- Source commit: `{commit}`
- Generated at: `{generated_at}`
- Query CLI: `python scripts/agent_query.py`
- Navigation recipe: `docs/recipes/fresh-agent-navigation.md`

# # Start here

```bash
python scripts/agent_query.py overview
python scripts/agent_query.py \"room transition loading\"
```

Then narrow only as needed:

```bash
python scripts/agent_query.py symbol GroundContactTransition
python scripts/agent_query.py docs \"transactional construction\"
python scripts/agent_query.py ecs \"room transition\" --crate ambition_app
python scripts/agent_query.py tests \"ground contact\"
python scripts/agent_query.py crate ambition_platformer2d_runtime
```

# # Available detail

| Corpus | Count | Best entry point |
|---|---:|---|
| Text files | {counts.get('files', 0)} | `.agent/index/file_summaries.json` |
| Rust symbols | {counts.get('symbols', 0)} | `.agent/index/symbol_index.json` |
| Tests | {counts.get('tests', 0)} | `.agent/index/test_map.json` |
| Workspace crates | {counts.get('crates', 0)} | `.agent/index/crates/index.json` |
| Registered ECS systems | {ecs.get('registered_systems', 0)} | `.agent/ecs_inventory/project.md` |
| ECS resources | {ecs.get('resources', 0)} | `.agent/ecs_inventory/project.md` |
| Message channels | {ecs.get('message_channels', 0)} | `.agent/ecs_inventory/project.md` |
| Spawn sites | {ecs.get('spawn_sites', 0)} | `.agent/ecs_inventory/project.md` |

Each `.agent/index/crates/<crate>.json` packet combines that package's files,
symbols, tests, module map, and links to its ECS inventory. Prefer those shards
over loading the full flat indexes into context.

# # Dependency edges: TWO graphs, and they disagree on purpose

`python scripts/agent_query.py deps <crate>` prints both. Which one answers your
question depends on the question:

| file | what it is | needs cargo |
|---|---|---|
| `.agent/index/crates/graph-declared.json` | what manifests **declare**, including optional edges and the features that enable them | no — parsed from `Cargo.toml` |
| `.agent/index/crates/graph-resolved.json` | what cargo **resolves**, feature-unified; every edge is compiled | at generation time only |

⚠ **the declared graph over-reports and the resolved graph is workspace-wide.**
An `optional = true` edge that no enabled feature turns on costs a consumer
nothing, so "X declares Y" is not "X links Y". Conversely the resolved graph
unifies features across the whole workspace, so it is not the closure of any one
consumer — that is a per-consumer measurement (`cargo tree --edges normal` in
that consumer's own workspace; see
`scripts/baselines/capability-footprint-baseline.json`).

`graph-resolved.json` is always present. When cargo was unavailable at
generation it carries `"available": false` and a reason, because a missing file
cannot be told apart from a workspace with no dependencies.

# ## Drawing one

    python scripts/agent_query.py graph                       # whole workspace, resolved
    python scripts/agent_query.py graph ambition_audio --depth 2
    python scripts/agent_query.py graph ambition_sfx --direction rdeps --depth 2
    python scripts/agent_query.py graph --graph declared --format svg

Graphviz is **opt-in and never required**: the `.dot` source is plain text this
script writes itself, and only `--format svg|png|pdf` shells out to `dot`. The
drawing states which graph it is in its title, dashes optional edges, and labels
them with the feature that enables them — a picture of declared edges and one of
resolved edges are otherwise indistinguishable and mean different things.

`./archive_agent_source.sh --dependency-graph` renders them into the archive.

# # Trust rule

Use generated data to locate likely owners and tests. Confirm the result in
source before editing. Current source wins for implementation fact; active
planning and ADRs win for intended direction. Historical docs and generated
summaries never override them.
"""


def tokenize(text: str) -> list[str]:
    return [token for token in re.findall(r"[A-Za-z0-9_]+", text.lower()) if len(token) > 1]


def score(query: str, fields: Iterable[str], *, primary: str = "") -> int:
    phrase = query.strip().lower()
    tokens = tokenize(query)
    hay = " \n".join(field.lower() for field in fields if field)
    primary_lower = primary.lower()
    value = 0
    if phrase and primary_lower == phrase:
        value += 200
    elif phrase and phrase in primary_lower:
        value += 110
    if phrase and phrase in hay:
        value += 70
    for token in tokens:
        if token == primary_lower:
            value += 60
        elif token in primary_lower:
            value += 30
        if re.search(rf"\b{re.escape(token)}\b", hay):
            value += 14
        elif token in hay:
            value += 5
    if tokens and all(token in hay for token in tokens):
        value += 35
    return value


def ranked(rows: Iterable[dict[str, Any]], query: str, field_names: Sequence[str], primary: str, limit: int) -> list[tuple[int, dict[str, Any]]]:
    scored = []
    for row in rows:
        fields = [str(row.get(name, "")) for name in field_names]
        row_score = score(query, fields, primary=str(row.get(primary, "")))
        if row_score > 0:
            scored.append((row_score, row))
    return sorted(scored, key=lambda item: (-item[0], str(item[1].get(primary, ""))))[:limit]


def line_location(row: dict[str, Any]) -> str:
    path = str(row.get("path") or row.get("file") or "")
    line = row.get("line")
    return f"{path}:{line}" if path and line else path


def print_section(title: str, rows: Sequence[str]) -> None:
    if not rows:
        return
    print(f"\n{title}")
    print("-" * len(title))
    for row in rows:
        print(row)


def all_doc_rows() -> list[dict[str, Any]]:
    rows: list[dict[str, Any]] = []
    specs = [
        ("planning", "planning_index.json", "planning_docs", "heading"),
        ("concept", "concept_index.json", "concepts", "title"),
        ("adr", "adr_index.json", "adrs", "title"),
        ("tool", "tool_index.json", "tools", "heading"),
        ("archive", "archive_index.json", "archive_docs", "heading"),
    ]
    for corpus, filename, key, title_key in specs:
        data = load_json(INDEX_DIR / filename, {}) or {}
        for item in data.get(key, []):
            row = dict(item)
            row["corpus"] = corpus
            row["title"] = row.get(title_key) or row.get("name") or row.get("path")
            rows.append(row)
    return rows


def ecs_rows(crate_filter: str | None = None) -> list[dict[str, Any]]:
    rows: list[dict[str, Any]] = []
    paths = sorted((ECS_DIR / "crates").glob("*.json")) if (ECS_DIR / "crates").exists() else []
    for path in paths:
        data = load_json(path, {}) or {}
        crate = str(data.get("crate_name", path.stem))
        if crate_filter and crate_filter.lower() not in crate.lower():
            continue
        for kind, key in [
            ("plugin", "plugins"),
            ("system", "system_like_functions"),
            ("ecs_item", "ecs_items"),
            ("architecture_item", "architecture_items"),
            ("spawn", "spawn_sites"),
            ("registration", "registrations"),
        ]:
            for item in data.get(key, []):
                if isinstance(item, str):
                    rows.append({"crate": crate, "kind": kind, "name": item})
                    continue
                if not isinstance(item, dict):
                    continue
                row = dict(item)
                row["crate"] = crate
                row["kind"] = kind
                row["name"] = row.get("name") or row.get("context") or row.get("expression") or ""
                rows.append(row)
        for name in data.get("registered_systems", []):
            rows.append({"crate": crate, "kind": "registered_system", "name": name})
        for name in (data.get("message_bus") or {}).keys():
            rows.append({"crate": crate, "kind": "message", "name": name})
        for name in (data.get("resource_access") or {}).keys():
            rows.append({"crate": crate, "kind": "resource", "name": name})
    return rows


def print_docs(query: str, limit: int) -> None:
    corpus_weight = {
        "planning": 45,
        "concept": 35,
        "adr": 30,
        "tool": 10,
        "archive": -25,
    }
    scored = []
    for row in all_doc_rows():
        row_score = score(
            query,
            [str(row.get(name, "")) for name in ["title", "path", "aliases", "status"]],
            primary=str(row.get("title", "")),
        )
        if row_score > 0:
            row_score += corpus_weight.get(str(row.get("corpus")), 0)
            scored.append((row_score, row))
    matches = sorted(scored, key=lambda item: (-item[0], str(item[1].get("title", ""))))[:limit]
    print_section(
        "Documents",
        [f"[{score_value:3}] {row.get('corpus')}: {row.get('title')} — {row.get('path')}" for score_value, row in matches],
    )


def print_symbols(query: str, limit: int) -> None:
    data = load_json(INDEX_DIR / "symbol_index.json", {}) or {}
    matches = ranked(data.get("symbols", []), query, ["name", "path", "kind", "visibility"], "name", limit)
    print_section(
        "Symbols",
        [f"[{score_value:3}] {row.get('kind')} {row.get('name')} — {line_location(row)} ({row.get('visibility')})" for score_value, row in matches],
    )


def print_tests(query: str, limit: int) -> None:
    data = load_json(INDEX_DIR / "test_map.json", {}) or {}
    matches = ranked(data.get("tests", []), query, ["name", "path"], "name", limit)
    print_section("Tests", [f"[{score_value:3}] {row.get('name')} — {line_location(row)}" for score_value, row in matches])


def print_files(query: str, limit: int) -> None:
    data = load_json(INDEX_DIR / "file_summaries.json", {}) or {}
    matches = ranked(data.get("files", []), query, ["path", "heading", "extension"], "path", limit)
    print_section(
        "Files",
        [f"[{score_value:3}] {row.get('path')} — {row.get('heading') or row.get('extension')} ({row.get('lines')} lines)" for score_value, row in matches],
    )


def print_ecs(query: str, limit: int, crate_filter: str | None) -> None:
    matches = ranked(
        ecs_rows(crate_filter),
        query,
        ["name", "crate", "kind", "file", "expression", "context", "identifiers", "resources_read", "resources_written", "messages_read", "messages_written"],
        "name",
        limit,
    )
    print_section(
        "ECS / Bevy inventory",
        [
            f"[{score_value:3}] {row.get('crate')} {row.get('kind')}: {str(row.get('name'))[:120]}"
            + (f" — {line_location(row)}" if line_location(row) else "")
            for score_value, row in matches
        ],
    )


def command_overview() -> None:
    catalog = load_json(CATALOG_PATH)
    if not catalog:
        raise SystemExit("missing .agent/index/catalog.json; run `python scripts/agent_query.py build-catalog`")
    counts = catalog.get("counts", {})
    print(f"source commit: {catalog.get('generated_from_commit', 'unknown')}")
    print(f"generated at: {catalog.get('generated_at') or 'unknown'}")
    for key in ["crates", "files", "symbols", "tests", "planning_docs", "concepts", "adrs"]:
        print(f"{key}: {counts.get(key, 0)}")
    ecs = counts.get("ecs", {})
    if isinstance(ecs, dict):
        print(f"registered systems: {ecs.get('registered_systems', 0)}")
        print(f"resources: {ecs.get('resources', 0)}")
        print(f"message channels: {ecs.get('message_channels', 0)}")
        print(f"spawn sites: {ecs.get('spawn_sites', 0)}")
    print("\nstart: python scripts/agent_query.py \"<task words>\"")
    print("guide: .agent/README.md")


def command_task(query: str, limit: int) -> None:
    print(f"Task packet: {query}")
    print_docs(query, limit)
    print_files(query, limit)
    print_symbols(query, limit)
    print_ecs(query, limit, None)
    print_tests(query, limit)


def command_crate(name: str) -> None:
    index = load_json(CRATE_INDEX_PATH, {}) or {}
    candidates = [row for row in index.get("crates", []) if name.lower() in str(row.get("crate_name", "")).lower()]
    if not candidates:
        raise SystemExit(f"no crate packet matching {name!r}; run build-catalog if indexes changed")
    exact = [row for row in candidates if str(row.get("crate_name", "")).lower() == name.lower()]
    row = exact[0] if exact else candidates[0]
    packet = load_json(ROOT / str(row["packet"]), {}) or {}
    summary = packet.get("summary", {})
    print(f"crate: {packet.get('crate_name')}")
    print(f"root: {packet.get('crate_root')}")
    print(f"manifest: {packet.get('manifest')}")
    print(f"module map: {packet.get('module_map') or '(none)'}")
    print(f"files: {summary.get('files', 0)}")
    print(f"symbols: {summary.get('symbols', 0)}")
    print(f"tests: {summary.get('tests', 0)}")
    ecs = packet.get("ecs_inventory", {})
    print(f"ecs inventory: {ecs.get('markdown') or '(none)'}")
    print(f"packet: {row['packet']}")
    public = [item for item in packet.get("symbols", []) if item.get("visibility") == "public"][:20]
    if public:
        print_section("First public symbols", [f"{item.get('kind')} {item.get('name')} — {line_location(item)}" for item in public])


def _resolve_member(name: str, members: Sequence[str]) -> str:
    exact = [member for member in members if member.lower() == name.lower()]
    if exact:
        return exact[0]
    partial = [member for member in members if name.lower() in member.lower()]
    if not partial:
        raise SystemExit(f"no workspace member matching {name!r}")
    if len(partial) > 1 and not any(member == name for member in partial):
        print(f"note: {name!r} matched {len(partial)}; using {partial[0]}", file=sys.stderr)
    return partial[0]


def command_deps(name: str, *, external: bool = False) -> None:
    """Both graphs for one crate, each labelled with what it is.

    Printing them together is the point: an agent asking "what does this crate
    pull in" is usually asking the RESOLVED question and would otherwise read
    the declared answer, which counts optional edges nothing enables.
    """
    declared = load_json(DECLARED_GRAPH_PATH, {}) or {}
    if not declared:
        raise SystemExit(
            "no declared dependency graph; run: python scripts/agent_query.py build-catalog"
        )
    members = declared.get("members", [])
    crate = _resolve_member(name, members)

    print(f"crate: {crate}")
    print()
    print("DECLARED (manifests; no toolchain needed)")
    rows = declared.get("edges", {}).get(crate, [])
    shown = [row for row in rows if external or row.get("internal")]
    for row in sorted(shown, key=lambda item: (not item.get("internal"), str(item.get("name")))):
        marks = []
        if row.get("kind") != "normal":
            marks.append(str(row.get("kind")))
        if row.get("optional"):
            enablers = row.get("enabled_by") or []
            marks.append("optional" + (f" via {','.join(enablers)}" if enablers else ", NOTHING enables it"))
        if row.get("target"):
            marks.append(f"target {row['target']}")
        if not row.get("internal"):
            marks.append("external")
        suffix = f"  [{'; '.join(marks)}]" if marks else ""
        print(f"  {row.get('name')}{suffix}")
    if not shown:
        print("  (none)")

    reverse = declared.get("reverse_edges", {}).get(crate, [])
    print()
    print(f"DECLARED BY ({len(reverse)} workspace members name this crate)")
    print("  " + (", ".join(reverse) if reverse else "(none)"))

    print()
    resolved = load_json(RESOLVED_GRAPH_PATH, {}) or {}
    if not resolved:
        print("RESOLVED  (absent — generated only where cargo is available)")
        print("  regenerate with a Rust toolchain: python scripts/generate_agent_index.py")
        return
    if not resolved.get("available", False):
        print("RESOLVED  (not generated)")
        print(f"  reason: {resolved.get('reason', 'unknown')}")
        return
    edges = resolved.get("edges", {}).get(crate)
    print("RESOLVED (cargo metadata; every edge is actually compiled)")
    if edges is None:
        print("  (crate absent from the resolve)")
    else:
        internal = [dep for dep in edges if dep in members]
        print("  " + (", ".join(internal) if internal else "(no workspace deps)"))
        if external:
            outside = [dep for dep in edges if dep not in members]
            print(f"  external ({len(outside)}): " + ", ".join(outside))
    features = resolved.get("features", {}).get(crate)
    if features:
        print(f"  features on: {', '.join(features)}")

    declared_internal = {row.get("name") for row in rows if row.get("internal")}
    if edges:
        only_declared = sorted(declared_internal - set(edges))
        if only_declared:
            print()
            print("  ⚠ declared but NOT resolved (optional, or dev-only):")
            print("    " + ", ".join(only_declared))


# ---------------------------------------------------------------------------
# Drawing. Graphviz is OPT-IN and is never required to produce the graph: `dot`
# source is plain text this script writes itself, and rendering it to a picture
# is a separate step that shells out. A machine with no graphviz still gets the
# `.dot` file and can render it elsewhere.
# ---------------------------------------------------------------------------

GRAPHVIZ_MISSING_HINT = (
    "graphviz is not installed — the `dot` binary is not on PATH.\n"
    "  The .dot source needs nothing; only rendering it does. Either install\n"
    "  graphviz, or write the source and render it elsewhere:\n"
    "    python scripts/agent_query.py graph --out deps.dot"
)


def normalized_graph_edges(
    graph: Mapping[str, Any], *, external: bool
) -> dict[str, list[dict[str, Any]]]:
    """Both graphs' edges in ONE shape, because they are stored differently.

    The declared graph stores rich rows (optional, `enabled_by`, kind); the
    resolved graph stores bare names, since by the time cargo has resolved it
    there is no optionality left to describe. Normalizing here keeps the drawing
    code from asking which graph it is holding — the only thing it needs to know
    is what to draw, and the difference shows up as edge STYLE, not as a branch.
    """
    members = set(graph.get("members", []))
    out: dict[str, list[dict[str, Any]]] = {}
    for source, rows in (graph.get("edges", {}) or {}).items():
        edges: list[dict[str, Any]] = []
        for row in rows:
            if isinstance(row, str):
                row = {"name": row, "internal": row in members, "kind": "normal"}
            if not external and not row.get("internal"):
                continue
            edges.append(dict(row))
        out[source] = sorted(edges, key=lambda item: str(item.get("name")))
    return out


def graph_neighbourhood(
    edges: Mapping[str, list[dict[str, Any]]],
    focus: str,
    depth: int,
    direction: str,
) -> set[str]:
    """Nodes within `depth` hops of `focus`, following the requested arrows.

    ⚠ `rdeps` is the question that actually gets asked ("what breaks if I change
    this"), and it is NOT answerable by reversing a `deps` drawing by eye once
    the graph is wider than a few nodes.
    """
    reverse: dict[str, set[str]] = {}
    for source, rows in edges.items():
        for row in rows:
            reverse.setdefault(str(row.get("name")), set()).add(source)

    seen = {focus}
    frontier = {focus}
    for _ in range(max(0, depth)):
        nxt: set[str] = set()
        for node in frontier:
            if direction in {"deps", "both"}:
                nxt |= {str(row.get("name")) for row in edges.get(node, [])}
            if direction in {"rdeps", "both"}:
                nxt |= reverse.get(node, set())
        nxt -= seen
        if not nxt:
            break
        seen |= nxt
        frontier = nxt
    return seen


def _dot_quote(text: str) -> str:
    return '"' + str(text).replace("\\", "\\\\").replace('"', '\\"') + '"'


def _dot_label(text: str) -> str:
    """A quoted label that keeps graphviz's own `\\n` line break working.

    ⚠ `_dot_quote` escapes backslashes, which is right for a crate name and
    wrong for a title — it turns the line break into a literal `\\n` printed in
    the picture. Separate helper rather than a flag, so the caller cannot pick
    the wrong one by omission.
    """
    return '"' + str(text).replace('"', '\\"') + '"'


def build_dot(
    graph: Mapping[str, Any],
    *,
    external: bool = False,
    focus: str | None = None,
    depth: int = 1,
    direction: str = "both",
) -> str:
    """Graphviz source for one dependency graph.

    ⭐ **the drawing says which graph it is**, in the title and in a comment
    header, for the same reason the JSON files do: a picture of declared edges
    and a picture of resolved edges look identical and mean different things,
    and a PNG in a chat window has lost every bit of context but its pixels.

    Declared-only distinctions are drawn, not dropped: an optional edge is
    dashed and labelled with the feature that enables it, and an optional edge
    that NOTHING enables — an edge that costs a consumer nothing — is dotted and
    said so. The resolved graph has no such edges by construction.
    """
    which = str(graph.get("graph", "unknown"))
    members = set(graph.get("members", []))
    edges = normalized_graph_edges(graph, external=external)

    keep: set[str] | None = None
    if focus:
        keep = graph_neighbourhood(edges, focus, depth, direction)

    lines: list[str] = [
        "// Generated by: python scripts/agent_query.py graph",
        f"// graph: {which}",
    ]
    lines += [f"// {chunk}" for chunk in textwrap.wrap(str(graph.get("means", "")), 76)]
    if which == "resolved" and not graph.get("available", True):
        lines.append(f"// ⚠ NOT AVAILABLE: {graph.get('reason', 'unknown')}")

    title = f"ambition dependencies — {which.upper()}"
    if focus:
        title += f" — {focus} ({direction}, depth {depth})"
    title += "\\nedges: " + (
        "what cargo resolves; every one is compiled"
        if which == "resolved"
        else "what manifests declare; dashed = optional, dotted = nothing enables it"
    )

    lines += [
        "digraph ambition_deps {",
        "  graph [rankdir=LR, splines=spline, overlap=false, fontname=Helvetica,",
        f"         labelloc=t, fontsize=14, label={_dot_label(title)}];",
        "  node [shape=box, style=rounded, fontname=Helvetica, fontsize=10];",
        "  edge [color=gray40];",
        "",
    ]

    drawn = sorted(
        {node for node in edges if keep is None or node in keep}
        | {
            str(row.get("name"))
            for source, rows in edges.items()
            if keep is None or source in keep
            for row in rows
            if keep is None or str(row.get("name")) in keep
        }
    )
    for node in drawn:
        attrs: list[str] = []
        if node == focus:
            attrs.append('style="rounded,filled", fillcolor="#ffe08a", penwidth=2')
        elif node not in members:
            attrs.append('shape=ellipse, color=gray60, fontcolor=gray40')
        suffix = f" [{', '.join(attrs)}]" if attrs else ""
        lines.append(f"  {_dot_quote(node)}{suffix};")
    lines.append("")

    for source in sorted(edges):
        if keep is not None and source not in keep:
            continue
        for row in edges[source]:
            target = str(row.get("name"))
            if keep is not None and target not in keep:
                continue
            attrs: list[str] = []
            if row.get("optional"):
                enablers = row.get("enabled_by") or []
                if enablers:
                    attrs.append("style=dashed")
                    attrs.append(f"label={_dot_quote(','.join(enablers))}")
                else:
                    attrs.append("style=dotted, color=gray70")
                    attrs.append('label="nothing enables"')
                attrs.append("fontsize=8, fontname=Helvetica")
            if row.get("kind") and row.get("kind") != "normal":
                attrs.append(f'xlabel={_dot_quote(row["kind"])}, fontsize=8')
            suffix = f" [{', '.join(attrs)}]" if attrs else ""
            lines.append(f"  {_dot_quote(source)} -> {_dot_quote(target)}{suffix};")

    lines.append("}")
    return "\n".join(lines) + "\n"


def render_dot(source: str, out: Path, fmt: str) -> None:
    """Run graphviz. Absence is an ERROR here, because you asked for a picture."""
    out.parent.mkdir(parents=True, exist_ok=True)
    try:
        proc = subprocess.run(
            ["dot", f"-T{fmt}", "-o", str(out)],
            input=source,
            text=True,
            capture_output=True,
            check=False,
            timeout=300,
        )
    except FileNotFoundError:
        raise SystemExit(GRAPHVIZ_MISSING_HINT) from None
    except (OSError, subprocess.SubprocessError) as exc:
        raise SystemExit(f"graphviz failed to run: {exc}") from None
    if proc.returncode != 0:
        detail = (proc.stderr or "").strip() or f"exit {proc.returncode}"
        raise SystemExit(f"graphviz failed: {detail}")


def command_graph(
    *,
    which: str,
    focus: str | None,
    depth: int,
    direction: str,
    external: bool,
    out: Path | None,
    fmt: str | None,
) -> None:
    """Write graphviz source for a dependency graph, and optionally render it."""
    path = RESOLVED_GRAPH_PATH if which == "resolved" else DECLARED_GRAPH_PATH
    graph = load_json(path, {}) or {}
    if not graph:
        raise SystemExit(
            f"no {which} dependency graph at {path}; "
            "run: python scripts/generate_agent_index.py"
        )
    if which == "resolved" and not graph.get("available", True):
        # Not a silent fall back to the declared graph: they disagree, and a
        # drawing that quietly answered the other question would be worse than
        # no drawing at all.
        raise SystemExit(
            f"the resolved graph was not generated: {graph.get('reason', 'unknown')}\n"
            "  regenerate it where cargo exists, or draw the other graph:\n"
            "    python scripts/agent_query.py graph --graph declared"
        )

    if focus:
        focus = _resolve_member(focus, graph.get("members", []))

    source = build_dot(
        graph, external=external, focus=focus, depth=depth, direction=direction
    )

    # Format is inferred from the output suffix when it is not given, because
    # `--out deps.svg --format dot` writing DOT into a .svg is a trap.
    if fmt is None:
        fmt = out.suffix.lstrip(".").lower() if out and out.suffix else "dot"
    if out is None:
        out = path.with_suffix(f".{fmt}")

    if fmt in {"dot", "gv"}:
        out.parent.mkdir(parents=True, exist_ok=True)
        out.write_text(source, encoding="utf-8")
    else:
        render_dot(source, out, fmt)
    print_output_location(out)


def command_path(path_text: str) -> None:
    normalized = Path(path_text).as_posix().removeprefix("./")
    files = load_json(INDEX_DIR / "file_summaries.json", {}) or {}
    row = next((item for item in files.get("files", []) if item.get("path") == normalized), None)
    if row:
        print(f"path: {normalized}")
        print(f"heading: {row.get('heading') or '(none)'}")
        print(f"lines: {row.get('lines')}")
    else:
        print(f"path not present in file summary: {normalized}")
    symbols = load_json(INDEX_DIR / "symbol_index.json", {}) or {}
    same_symbols = [item for item in symbols.get("symbols", []) if item.get("path") == normalized]
    tests = load_json(INDEX_DIR / "test_map.json", {}) or {}
    same_tests = [item for item in tests.get("tests", []) if item.get("path") == normalized]
    print_section("Symbols in file", [f"{item.get('kind')} {item.get('name')} — line {item.get('line')}" for item in same_symbols[:50]])
    print_section("Tests in file", [f"{item.get('name')} — line {item.get('line')}" for item in same_tests[:50]])


def file_uri(path: Path) -> str:
    return path.resolve().as_uri()


def print_output_location(path: Path) -> None:
    directory = path.resolve().parent
    try:
        from rich import print as rich_print
        from rich.markup import escape
    except ImportError:
        print(path.resolve())
        print(directory)
        return
    rich_print(f"[link={file_uri(path)}]{escape(str(path.resolve()))}[/link]")
    rich_print(f"[link={file_uri(directory)}]{escape(str(directory))}[/link]")


def parser() -> argparse.ArgumentParser:
    root = argparse.ArgumentParser(description=__doc__)
    root.add_argument("--limit", type=int, default=8, help="maximum results per section")
    sub = root.add_subparsers(dest="command")

    sub.add_parser("overview")
    task = sub.add_parser("task")
    task.add_argument("query", nargs="+")
    symbol = sub.add_parser("symbol")
    symbol.add_argument("query", nargs="+")
    docs = sub.add_parser("docs")
    docs.add_argument("query", nargs="+")
    ecs = sub.add_parser("ecs")
    ecs.add_argument("query", nargs="+")
    ecs.add_argument("--crate", dest="crate_filter")
    tests = sub.add_parser("tests")
    tests.add_argument("query", nargs="+")
    crate = sub.add_parser("crate")
    crate.add_argument("name")
    deps = sub.add_parser("deps")
    deps.add_argument("name")
    deps.add_argument(
        "--external",
        action="store_true",
        help="include non-workspace (crates.io) edges, which are hidden by default",
    )
    graph = sub.add_parser(
        "graph", help="write graphviz source for a dependency graph, optionally rendered"
    )
    graph.add_argument("focus", nargs="?", help="draw only around this crate")
    graph.add_argument(
        "--graph",
        dest="which",
        choices=["resolved", "declared"],
        default="resolved",
        help="which graph to draw; they disagree on purpose (default: resolved)",
    )
    graph.add_argument(
        "--depth", type=int, default=1, help="hops from the focus crate (default: 1)"
    )
    graph.add_argument(
        "--direction",
        choices=["deps", "rdeps", "both"],
        default="both",
        help="follow dependencies, dependents, or both from the focus (default: both)",
    )
    graph.add_argument(
        "--external",
        action="store_true",
        help="include non-workspace (crates.io) nodes, which are hidden by default",
    )
    graph.add_argument("--out", type=Path, help="output path; default sits beside the JSON graph")
    graph.add_argument(
        "--format",
        dest="fmt",
        help="dot (no graphviz needed) or any `dot -T` format: svg, png, pdf. "
        "Default: inferred from --out, else dot",
    )
    path = sub.add_parser("path")
    path.add_argument("path")
    build = sub.add_parser("build-catalog")
    build.add_argument("--quiet", action="store_true")
    return root


def normalize_argv(argv: Sequence[str]) -> list[str]:
    if not argv:
        return ["overview"]

    # Permit the global result limit before or after a subcommand/query.
    global_options: list[str] = []
    values: list[str] = []
    raw = list(argv)
    idx = 0
    while idx < len(raw):
        arg = raw[idx]
        if arg == "--limit" and idx + 1 < len(raw):
            global_options.extend([arg, raw[idx + 1]])
            idx += 2
            continue
        if arg.startswith("--limit="):
            global_options.append(arg)
            idx += 1
            continue
        values.append(arg)
        idx += 1

    first_non_option = next((i for i, arg in enumerate(values) if not arg.startswith("-")), None)
    if first_non_option is not None and values[first_non_option] not in KNOWN_COMMANDS:
        values.insert(first_non_option, "task")
    return [*global_options, *values]


def main(argv: Sequence[str] | None = None) -> int:
    args = parser().parse_args(normalize_argv(list(argv if argv is not None else sys.argv[1:])))
    limit = max(1, args.limit)
    warn_if_index_stale()
    if args.command == "overview":
        command_overview()
    elif args.command == "task":
        command_task(" ".join(args.query), limit)
    elif args.command == "symbol":
        print_symbols(" ".join(args.query), limit)
    elif args.command == "docs":
        print_docs(" ".join(args.query), limit)
    elif args.command == "ecs":
        print_ecs(" ".join(args.query), limit, args.crate_filter)
    elif args.command == "tests":
        print_tests(" ".join(args.query), limit)
    elif args.command == "crate":
        command_crate(args.name)
    elif args.command == "deps":
        command_deps(args.name, external=args.external)
    elif args.command == "graph":
        command_graph(
            which=args.which,
            focus=args.focus,
            depth=args.depth,
            direction=args.direction,
            external=args.external,
            out=args.out,
            fmt=args.fmt,
        )
    elif args.command == "path":
        command_path(args.path)
    elif args.command == "build-catalog":
        build_catalog(quiet=args.quiet)
    else:
        parser().print_help()
        return 2
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
