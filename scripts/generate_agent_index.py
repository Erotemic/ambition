#!/usr/bin/env python3
"""Generate lightweight agent navigation indexes for Ambition.

The indexes are intentionally simple, reviewable JSON. They are navigation aids,
not source-of-truth replacements for code, ADRs, or concept pages.
"""
from __future__ import annotations

import json
import re
import subprocess
from datetime import datetime, timezone
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
INDEX_DIR = ROOT / ".agent" / "index"

SKIP_DIRS = {".git", "target", ".venv", "__pycache__"}
TEXT_EXTS = {".md", ".rs", ".toml", ".ron", ".yaml", ".yml", ".py", ".sh", ".json"}


def git_commit() -> str:
    try:
        return subprocess.check_output(["git", "rev-parse", "--short", "HEAD"], cwd=ROOT, text=True).strip()
    except Exception:
        return "unknown"


def generated_meta() -> dict[str, str]:
    return {
        "generated_from_commit": git_commit(),
        "generated_at": datetime.now(timezone.utc).replace(microsecond=0).isoformat().replace("+00:00", "Z"),
        "generator": "scripts/generate_agent_index.py",
    }


def iter_files() -> list[Path]:
    out: list[Path] = []
    for path in ROOT.rglob("*"):
        if not path.is_file():
            continue
        rel_parts = path.relative_to(ROOT).parts
        if any(part in SKIP_DIRS for part in rel_parts):
            continue
        if path.suffix in TEXT_EXTS:
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
        entries.append({
            "path": rel(path),
            "extension": path.suffix,
            "lines": text.count("\n") + (1 if text else 0),
            "heading": first_heading(text),
        })
    return {**generated_meta(), "files": entries}


SYMBOL_RE = re.compile(
    r"^\s*(?P<vis>pub(?:\([^)]*\))?\s+)?(?P<kind>struct|enum|trait|type|fn|const|static|mod)\s+(?P<name>[A-Za-z_][A-Za-z0-9_]*)",
    re.MULTILINE,
)


def build_symbol_index() -> dict[str, object]:
    symbols = []
    crates_dir = ROOT / "crates"
    if not crates_dir.exists():
        return {**generated_meta(), "symbols": symbols}
    for path in sorted(crates_dir.rglob("*.rs")):
        text = path.read_text(encoding="utf-8", errors="replace")
        for m in SYMBOL_RE.finditer(text):
            symbols.append({
                "name": m.group("name"),
                "kind": m.group("kind"),
                "visibility": "public" if m.group("vis") else "private",
                "path": rel(path),
                "line": text.count("\n", 0, m.start()) + 1,
            })
    return {**generated_meta(), "symbols": symbols}


def build_test_map() -> dict[str, object]:
    tests = []
    crates_dir = ROOT / "crates"
    if not crates_dir.exists():
        return {**generated_meta(), "tests": tests}
    for path in sorted(crates_dir.rglob("*.rs")):
        text = path.read_text(encoding="utf-8", errors="replace")
        lines = text.splitlines()
        pending_attr_line: int | None = None
        for idx, line in enumerate(lines, start=1):
            stripped = line.strip()
            if stripped.startswith("#[test]") or stripped.startswith("#[rstest") or stripped.startswith("#[tokio::test"):
                pending_attr_line = idx
                continue
            if pending_attr_line is not None:
                m = re.search(r"\bfn\s+([A-Za-z_][A-Za-z0-9_]*)", line)
                if m:
                    tests.append({"name": m.group(1), "path": rel(path), "line": idx, "attr_line": pending_attr_line})
                    pending_attr_line = None
            m = re.search(r"\bfn\s+(test_[A-Za-z0-9_]+|[A-Za-z0-9_]+_test)\b", line)
            if m:
                tests.append({"name": m.group(1), "path": rel(path), "line": idx, "attr_line": None})
        r = rel(path)
        if "/tests/" in r and not any(t["path"] == r for t in tests):
            tests.append({"name": Path(r).stem, "path": r, "line": 1, "attr_line": None})
    return {**generated_meta(), "tests": tests}


def build_concept_index() -> dict[str, object]:
    concepts = []
    cdir = ROOT / "docs" / "concepts"
    for path in sorted(cdir.glob("*.md")) if cdir.exists() else []:
        if path.name == "index.md":
            continue
        text = path.read_text(encoding="utf-8", errors="replace")
        fm = parse_frontmatter(text)
        concepts.append({
            "id": fm.get("id", path.stem),
            "path": rel(path),
            "title": first_heading(text) or path.stem,
            "aliases": fm.get("aliases", []),
            "implemented_by": fm.get("implemented_by", []),
            "tested_by": fm.get("tested_by", []),
            "related_docs": fm.get("related_docs", []),
            "related_memory": fm.get("related_memory", []),
            "last_verified": fm.get("last_verified"),
        })
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
            tools.append({
                "name": path.name,
                "path": rel(path),
                "has_readme": readme.exists(),
                "has_pyproject": pyproject.exists(),
                "heading": first_heading(readme.read_text(encoding="utf-8", errors="replace")) if readme.exists() else None,
            })
    return {**generated_meta(), "tools": tools}


def build_archive_index() -> dict[str, object]:
    entries = []
    archive = ROOT / "docs" / "archive"
    if archive.exists():
        for path in sorted(archive.rglob("*.md")):
            text = path.read_text(encoding="utf-8", errors="replace")
            entries.append({"path": rel(path), "basename": path.name, "heading": first_heading(text), "lines": text.count("\n") + 1})
    return {**generated_meta(), "archive_docs": entries}


def build_doc_health(files: list[Path]) -> dict[str, object]:
    md = [p for p in files if p.suffix == ".md"]
    longest = sorted(
        ({"path": rel(p), "lines": p.read_text(encoding="utf-8", errors="replace").count("\n") + 1} for p in md),
        key=lambda x: x["lines"],
        reverse=True,
    )[:25]
    return {**generated_meta(), "doc_count": len(md), "longest_markdown": longest}


def write_json(path: Path, data: dict[str, object]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(data, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def main() -> int:
    files = iter_files()
    INDEX_DIR.mkdir(parents=True, exist_ok=True)
    write_json(INDEX_DIR / "file_summaries.json", build_file_summaries(files))
    write_json(INDEX_DIR / "symbol_index.json", build_symbol_index())
    write_json(INDEX_DIR / "test_map.json", build_test_map())
    write_json(INDEX_DIR / "concept_index.json", build_concept_index())
    write_json(INDEX_DIR / "adr_index.json", build_adr_index())
    write_json(INDEX_DIR / "tool_index.json", build_tool_index())
    write_json(INDEX_DIR / "archive_index.json", build_archive_index())
    write_json(INDEX_DIR / "doc_health.json", build_doc_health(files))
    print("generated .agent indexes")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
