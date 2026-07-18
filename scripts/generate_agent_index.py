#!/usr/bin/env python3
"""Generate lightweight agent navigation indexes for Ambition.

The indexes are intentionally simple, reviewable JSON. They are navigation aids,
not source-of-truth replacements for code, ADRs, or concept pages.
"""

from __future__ import annotations

import json
import os
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
INDEX_DIR = ROOT / ".agent" / "index"

SKIP_DIRS = {".git", ".agent", ".worktrees", "target", ".venv", "__pycache__"}
TEXT_EXTS = {".md", ".rs", ".toml", ".ron", ".yaml", ".yml", ".py", ".sh", ".json"}


def generated_meta() -> dict[str, str]:
    """Stable metadata for committed navigation indexes.

    The tracked indexes intentionally omit HEAD and wall-clock time. Embedding
    either makes generation self-invalidating: committing the generated file
    changes HEAD, and rerunning at a later second changes the timestamp. Source
    content is already represented by the generated index bodies themselves.
    """
    return {"generator": "scripts/generate_agent_index.py"}


def iter_files() -> list[Path]:
    # `os.walk` with in-place `dirnames` filtering so SKIP_DIRS actually
    # prevents descent. The old `rglob("*")` form filtered after the fact,
    # which meant the script still recursed into `target/` (millions of
    # files on Android/desktop builds) and exhausted file descriptors on
    # virtiofs hosts (EMFILE: Too many open files).
    out: list[Path] = []
    for dirpath, dirnames, filenames in os.walk(ROOT):
        dirnames[:] = [d for d in dirnames if d not in SKIP_DIRS]
        for name in filenames:
            path = Path(dirpath) / name
            if path.suffix in TEXT_EXTS:
                out.append(path)
    return sorted(out)


# Workspace source roots that hold Rust crate source. `crates/` is the engine;
# `game/` holds the app + content + demo crates (re-homed by decomposition E7);
# `tests/` holds the workspace-policy package. The symbol/test indexes sweep all
# three so a chat agent can find e.g. Smb1RulesPlugin / level_1_1 in game/.
SOURCE_ROOTS = ("crates", "game", "tests")


def iter_source_rs() -> list[Path]:
    """Every `.rs` under the workspace source roots (crates/, game/, tests/)."""
    out: list[Path] = []
    for root_name in SOURCE_ROOTS:
        root = ROOT / root_name
        if not root.is_dir():
            continue
        for path in root.rglob("*.rs"):
            if any(part in SKIP_DIRS for part in path.parts):
                continue
            out.append(path)
    return sorted(out)


def rel(path: Path) -> str:
    return path.relative_to(ROOT).as_posix()


def first_heading(text: str) -> str | None:
    for line in text.splitlines():
        if line.startswith("#"):
            return line.lstrip("#").strip()
    return None


def parse_frontmatter(text: str) -> dict[str, object]:
    lines = text.splitlines()
    if not lines or lines[0].strip() != "---":
        return {}
    data: dict[str, object] = {}
    current_key: str | None = None
    current_list: list[str] | None = None
    for line in lines[1:]:
        if line.strip() == "---":
            break
        if re.match(r"^[A-Za-z_][A-Za-z0-9_-]*:\s*", line):
            key, value = line.split(":", 1)
            key = key.strip()
            value = value.strip()
            current_key = key
            if value:
                data[key] = value
                current_list = None
            else:
                current_list = []
                data[key] = current_list
        elif current_list is not None and line.strip().startswith("- "):
            current_list.append(line.strip()[2:].strip())
        elif current_key and line.startswith("  - "):
            if not isinstance(data.get(current_key), list):
                data[current_key] = []
            data[current_key].append(line.strip()[2:].strip())  # type: ignore[union-attr]
    return data


def build_file_summaries(files: list[Path]) -> dict[str, object]:
    entries = []
    for path in files:
        text = path.read_text(encoding="utf-8", errors="replace")
        entries.append(
            {
                "path": rel(path),
                "extension": path.suffix,
                "lines": text.count("\n") + (1 if text else 0),
                "heading": first_heading(text),
            }
        )
    return {**generated_meta(), "files": entries}


SYMBOL_RE = re.compile(
    r"^\s*(?P<vis>pub(?:\([^)]*\))?\s+)?(?P<kind>struct|enum|trait|type|fn|const|static|mod)\s+(?P<name>[A-Za-z_][A-Za-z0-9_]*)",
    re.MULTILINE,
)


def build_symbol_index() -> dict[str, object]:
    symbols = []
    for path in iter_source_rs():
        text = path.read_text(encoding="utf-8", errors="replace")
        for m in SYMBOL_RE.finditer(text):
            symbols.append(
                {
                    "name": m.group("name"),
                    "kind": m.group("kind"),
                    "visibility": "public" if m.group("vis") else "private",
                    "path": rel(path),
                    "line": text.count("\n", 0, m.start()) + 1,
                }
            )
    return {**generated_meta(), "symbols": symbols}


def build_test_map() -> dict[str, object]:
    tests = []
    for path in iter_source_rs():
        text = path.read_text(encoding="utf-8", errors="replace")
        lines = text.splitlines()
        pending_attr_line: int | None = None
        for idx, line in enumerate(lines, start=1):
            stripped = line.strip()
            if (
                stripped.startswith("#[test]")
                or stripped.startswith("#[rstest")
                or stripped.startswith("#[tokio::test")
            ):
                pending_attr_line = idx
                continue
            if pending_attr_line is not None:
                m = re.search(r"\bfn\s+([A-Za-z_][A-Za-z0-9_]*)", line)
                if m:
                    tests.append(
                        {
                            "name": m.group(1),
                            "path": rel(path),
                            "line": idx,
                            "attr_line": pending_attr_line,
                        }
                    )
                    pending_attr_line = None
            m = re.search(r"\bfn\s+(test_[A-Za-z0-9_]+|[A-Za-z0-9_]+_test)\b", line)
            if m:
                tests.append(
                    {
                        "name": m.group(1),
                        "path": rel(path),
                        "line": idx,
                        "attr_line": None,
                    }
                )
        r = rel(path)
        if "/tests/" in r and not any(t["path"] == r for t in tests):
            tests.append(
                {"name": Path(r).stem, "path": r, "line": 1, "attr_line": None}
            )
    return {**generated_meta(), "tests": tests}


def build_concept_index() -> dict[str, object]:
    concepts = []
    cdir = ROOT / "docs" / "concepts"
    for path in sorted(cdir.glob("*.md")) if cdir.exists() else []:
        if path.name == "index.md":
            continue
        text = path.read_text(encoding="utf-8", errors="replace")
        fm = parse_frontmatter(text)
        concepts.append(
            {
                "id": fm.get("id", path.stem),
                "path": rel(path),
                "title": first_heading(text) or path.stem,
                "aliases": fm.get("aliases", []),
                "implemented_by": fm.get("implemented_by", []),
                "tested_by": fm.get("tested_by", []),
                "related_docs": fm.get("related_docs", []),
                "related_memory": fm.get("related_memory", []),
                "last_verified": fm.get("last_verified"),
            }
        )
    return {**generated_meta(), "concepts": concepts}


def build_adr_index() -> dict[str, object]:
    adrs = []
    for path in sorted((ROOT / "docs" / "adr").glob("*.md")):
        if path.name == "README.md":
            continue
        text = path.read_text(encoding="utf-8", errors="replace")
        status = None
        m = re.search(r"## Status\s+\n\s*([^\n]+)", text)
        if m:
            status = m.group(1).strip()
        adrs.append({"path": rel(path), "title": first_heading(text), "status": status})
    return {**generated_meta(), "adrs": adrs}


def build_tool_index() -> dict[str, object]:
    tools = []
    tools_dir = ROOT / "tools"
    if tools_dir.exists():
        for path in sorted(tools_dir.iterdir()):
            if path.name == "experimental" or not path.is_dir():
                continue
            readme = path / "README.md"
            pyproject = path / "pyproject.toml"
            tools.append(
                {
                    "name": path.name,
                    "path": rel(path),
                    "has_readme": readme.exists(),
                    "has_pyproject": pyproject.exists(),
                    "heading": first_heading(
                        readme.read_text(encoding="utf-8", errors="replace")
                    )
                    if readme.exists()
                    else None,
                }
            )
    return {**generated_meta(), "tools": tools}


def build_archive_index() -> dict[str, object]:
    entries = []
    archive = ROOT / "docs" / "archive"
    if archive.exists():
        for path in sorted(archive.rglob("*.md")):
            text = path.read_text(encoding="utf-8", errors="replace")
            entries.append(
                {
                    "path": rel(path),
                    "basename": path.name,
                    "heading": first_heading(text),
                    "lines": text.count("\n") + 1,
                }
            )
    return {**generated_meta(), "archive_docs": entries}


def build_doc_health(files: list[Path]) -> dict[str, object]:
    md = [p for p in files if p.suffix == ".md"]
    longest = sorted(
        (
            {
                "path": rel(p),
                "lines": p.read_text(encoding="utf-8", errors="replace").count("\n")
                + 1,
            }
            for p in md
        ),
        key=lambda x: x["lines"],
        reverse=True,
    )[:25]
    return {**generated_meta(), "doc_count": len(md), "longest_markdown": longest}


# The canonical "read me first" docs. AGENTS.md is the root instruction file
# (CLAUDE.md defers to it); docs/planning/ is the consolidated single source of
# truth (vision -> roadmap -> the live tracks queue). Order = suggested read order.
ENTRY_DOC_CANDIDATES = (
    "AGENTS.md",
    "CLAUDE.md",
    "README.md",
    "docs/planning/README.md",
    "docs/planning/vision.md",
    "docs/planning/roadmap.md",
    "docs/planning/tracks.md",
    "docs/planning/decision-principles.md",
    "docs/recipes/fresh-agent-navigation.md",
    "docs/concepts/architecture-review-questions.md",
    "MODULES.md",
)


def build_entry_points() -> dict[str, object]:
    """A curated 'start here' index so an uploaded tarball is self-orienting."""
    start_here = []
    for relpath in ENTRY_DOC_CANDIDATES:
        p = ROOT / relpath
        if not p.is_file():
            continue
        text = p.read_text(encoding="utf-8", errors="replace")
        start_here.append(
            {
                "path": relpath,
                "heading": first_heading(text),
                "lines": text.count("\n") + (1 if text else 0),
            }
        )
    # Every MODULES.md concern-map across the source roots (per-crate navigation).
    module_maps = []
    for root_name in SOURCE_ROOTS:
        root = ROOT / root_name
        if not root.is_dir():
            continue
        for p in sorted(root.rglob("MODULES.md")):
            if any(part in SKIP_DIRS for part in p.parts):
                continue
            module_maps.append(
                {
                    "path": rel(p),
                    "heading": first_heading(
                        p.read_text(encoding="utf-8", errors="replace")
                    ),
                }
            )
    return {**generated_meta(), "start_here": start_here, "module_maps": module_maps}


def build_planning_index() -> dict[str, object]:
    """The docs/planning master-plan tree (THE single source of truth)."""
    docs = []
    pdir = ROOT / "docs" / "planning"
    if pdir.is_dir():
        for p in sorted(pdir.rglob("*.md")):
            text = p.read_text(encoding="utf-8", errors="replace")
            docs.append(
                {
                    "path": rel(p),
                    "heading": first_heading(text),
                    "lines": text.count("\n") + 1,
                }
            )
    return {**generated_meta(), "planning_docs": docs}


def write_json(path: Path, data: dict[str, object]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(data, indent=2, sort_keys=True) + "\n", encoding="utf-8")



def yaml_scalar(value: object) -> str:
    text = str(value)
    if text == '' or any(ch in text for ch in ':#{}[]&,*?|<>!=%@`') or text.strip() != text:
        escaped = text.replace('\\', '\\\\').replace('"', '\\"')
        return f'"{escaped}"'
    return text


def patch_yaml_top_level_scalars(text: str, updates: dict[str, object], *, after_key: str | None = None) -> str:
    """Patch simple top-level YAML scalar keys while preserving the file body."""
    lines = text.splitlines()
    found: set[str] = set()
    out: list[str] = []

    for line in lines:
        key_match = None
        for key in updates:
            if line.startswith(f'{key}:'):
                key_match = key
                break
        if key_match is None:
            out.append(line)
        else:
            out.append(f'{key_match}: {yaml_scalar(updates[key_match])}')
            found.add(key_match)

    missing = [(key, value) for key, value in updates.items() if key not in found]
    if missing:
        insert_at = None
        if after_key is not None:
            for idx, line in enumerate(out):
                if line.startswith(f'{after_key}:'):
                    insert_at = idx + 1
                    break
        new_lines = [f'{key}: {yaml_scalar(value)}' for key, value in missing]
        if insert_at is None:
            if out and out[-1].strip():
                out.append('')
            out.extend(new_lines)
        else:
            out[insert_at:insert_at] = new_lines

    return '\n'.join(out).rstrip() + '\n'


def update_agent_manifest(meta: dict[str, str]) -> None:
    """Refresh stable manifest metadata alongside the JSON indexes.

    Remove legacy volatile generation keys if present. The manifest must be
    byte-identical after repeated generation when source content is unchanged.
    """
    manifest = ROOT / ".agent" / "manifest.yaml"
    manifest.parent.mkdir(parents=True, exist_ok=True)
    if manifest.exists():
        lines = manifest.read_text(encoding="utf-8").splitlines()
        lines = [
            line
            for line in lines
            if not line.startswith("generated_from_commit:")
            and not line.startswith("generated_at:")
        ]
        text = "\n".join(lines).rstrip() + "\n"
    else:
        text = "schema_version: 4\n"
    updates = {"generator": meta["generator"]}
    manifest.write_text(
        patch_yaml_top_level_scalars(text, updates, after_key="schema_version"),
        encoding="utf-8",
    )

def main() -> int:
    files = iter_files()
    INDEX_DIR.mkdir(parents=True, exist_ok=True)
    write_json(INDEX_DIR / "entry_points.json", build_entry_points())
    write_json(INDEX_DIR / "planning_index.json", build_planning_index())
    write_json(INDEX_DIR / "file_summaries.json", build_file_summaries(files))
    write_json(INDEX_DIR / "symbol_index.json", build_symbol_index())
    write_json(INDEX_DIR / "test_map.json", build_test_map())
    write_json(INDEX_DIR / "concept_index.json", build_concept_index())
    write_json(INDEX_DIR / "adr_index.json", build_adr_index())
    write_json(INDEX_DIR / "tool_index.json", build_tool_index())
    write_json(INDEX_DIR / "archive_index.json", build_archive_index())
    write_json(INDEX_DIR / "doc_health.json", build_doc_health(files))
    update_agent_manifest(generated_meta())

    # Build the progressive-disclosure catalog from the fresh flat indexes.
    # The archive builder reruns this after ECS inventory so archive packets
    # include current Bevy counts and per-crate inventory links.
    from agent_query import build_catalog

    build_catalog(quiet=True)
    print("generated .agent indexes, catalog, crate packets, and manifest")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
