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
    python scripts/agent_query.py crate ambition_runtime
    python scripts/agent_query.py build-catalog
"""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
import tomllib
from collections import defaultdict
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Iterable, Sequence

ROOT = Path(__file__).resolve().parents[1]
AGENT_DIR = ROOT / ".agent"
INDEX_DIR = AGENT_DIR / "index"
ECS_DIR = AGENT_DIR / "ecs_inventory"
CATALOG_PATH = INDEX_DIR / "catalog.json"
CRATE_INDEX_PATH = INDEX_DIR / "crates" / "index.json"
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
    "path",
    "build-catalog",
}


@dataclass(frozen=True)
class CrateInfo:
    name: str
    root: str
    manifest: str
    module_map: str | None


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


def source_commit() -> str:
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


def package_name(manifest: Path) -> str | None:
    try:
        data = tomllib.loads(manifest.read_text(encoding="utf-8"))
    except (OSError, tomllib.TOMLDecodeError):
        return None
    package = data.get("package")
    if not isinstance(package, dict):
        return None
    value = package.get("name")
    return value if isinstance(value, str) else None


def discover_crates() -> list[CrateInfo]:
    entry_points = load_json(INDEX_DIR / "entry_points.json", {}) or {}
    module_by_root: dict[str, str] = {}
    for row in entry_points.get("module_maps", []):
        path = str(row.get("path", ""))
        if path.endswith("/MODULES.md"):
            module_by_root[path.removesuffix("/MODULES.md")] = path

    roots: list[CrateInfo] = []
    for parent in ("crates", "game", "tests"):
        base = ROOT / parent
        if not base.exists():
            continue
        for manifest in sorted(base.glob("*/Cargo.toml")):
            name = package_name(manifest)
            if name is None:
                continue
            root = manifest.parent.relative_to(ROOT).as_posix()
            roots.append(
                CrateInfo(
                    name=name,
                    root=root,
                    manifest=manifest.relative_to(ROOT).as_posix(),
                    module_map=module_by_root.get(root),
                )
            )
    return sorted(roots, key=lambda item: item.name)


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

    crate_index = {
        "schema_version": SCHEMA_VERSION,
        "generator": "scripts/agent_query.py build-catalog",
        "crates": crate_rows,
        "repository_packet": ".agent/index/crates/_repository.json",
    }
    write_json(CRATE_INDEX_PATH, crate_index)

    catalog = {
        "schema_version": SCHEMA_VERSION,
        "generator": "scripts/agent_query.py build-catalog",
        "generated_from_commit": source_commit(),
        "generated_at": simple_manifest_value("generated_at"),
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
            "python scripts/agent_query.py crate ambition_runtime",
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

## Start here

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
python scripts/agent_query.py crate ambition_runtime
```

## Available detail

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

## Trust rule

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
