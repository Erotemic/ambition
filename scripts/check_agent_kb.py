#!/usr/bin/env python3
"""Lightweight lint for Ambition's agent-readable repository knowledge base.

This intentionally uses only the Python standard library so it can run in a
minimal checkout and inside GitHub Actions.
"""
from __future__ import annotations

import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]

REQUIRED_FILES = [
    "AGENTS.md",
    "README.md",
    "docs/README.md",
    "docs/redirects.md",
    "docs/current/state.md",
    "docs/current/risks.md",
    "docs/current/next.md",
    "docs/concepts/index.md",
    "docs/systems/index.md",
    "docs/recipes/index.md",
    "docs/vision/index.md",
    "docs/planning/index.md",
    "docs/history/index.md",
    "dev/README.md",
    "dev/SEARCH.md",
    "dev/journals/index.md",
    "dev/benchmark-candidates/index.md",
    ".agent/manifest.yaml",
    ".agent/retrieval_evals.yaml",
    ".agent/index/file_summaries.json",
    ".agent/index/symbol_index.json",
    ".agent/index/test_map.json",
    ".agent/index/concept_index.json",
    "scripts/generate_agent_index.py",
]

ALLOWED_TOP_LEVEL_DOCS = {
    "AGENT_HANDOFF.md",
    "CURRENT_STATE.md",
    "GOAL_STATE.md",
    "README.md",
    "lessons_learned.md",
    "redirects.md",
}

CONCEPT_REQUIRED_KEYS = {"id", "aliases", "last_verified"}
AGENTS_MAX_LINES = 170


def rel(path: Path) -> str:
    return path.relative_to(ROOT).as_posix()


def fail(errors: list[str], msg: str) -> None:
    errors.append(msg)


def parse_frontmatter(path: Path) -> dict[str, object]:
    lines = path.read_text(encoding="utf-8", errors="replace").splitlines()
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
            value = line.strip()[2:].strip()
            if not isinstance(data.get(current_key), list):
                data[current_key] = []
            data[current_key].append(value)  # type: ignore[union-attr]
    return data


def check_required_files(errors: list[str]) -> None:
    for item in REQUIRED_FILES:
        if not (ROOT / item).exists():
            fail(errors, f"missing required KB file: {item}")


def check_top_level_docs(errors: list[str]) -> None:
    docs = ROOT / "docs"
    for path in sorted(docs.glob("*.md")):
        if path.name not in ALLOWED_TOP_LEVEL_DOCS:
            fail(errors, f"unexpected top-level docs stub or doc: {rel(path)}; move it under a routed folder or add it to docs/redirects.md")


def check_agents_size(errors: list[str]) -> None:
    path = ROOT / "AGENTS.md"
    if not path.exists():
        return
    lines = path.read_text(encoding="utf-8", errors="replace").splitlines()
    if len(lines) > AGENTS_MAX_LINES:
        fail(errors, f"AGENTS.md has {len(lines)} lines; keep it <= {AGENTS_MAX_LINES} and route to docs instead")


def check_json_indexes(errors: list[str]) -> None:
    required_keys = {
        ".agent/index/file_summaries.json": "files",
        ".agent/index/symbol_index.json": "symbols",
        ".agent/index/test_map.json": "tests",
        ".agent/index/concept_index.json": "concepts",
    }
    for relpath, payload_key in required_keys.items():
        path = ROOT / relpath
        if not path.exists():
            continue
        try:
            data = json.loads(path.read_text(encoding="utf-8"))
        except Exception as ex:  # pragma: no cover - lint output path
            fail(errors, f"invalid JSON in {relpath}: {ex}")
            continue
        if "generated_from_commit" not in data or "generated_at" not in data:
            fail(errors, f"{relpath} missing generated_from_commit/generated_at provenance")
        if payload_key not in data:
            fail(errors, f"{relpath} missing payload key: {payload_key}")
        elif not isinstance(data[payload_key], list):
            fail(errors, f"{relpath} payload key is not a list: {payload_key}")


def check_concepts(errors: list[str]) -> None:
    concept_dir = ROOT / "docs/concepts"
    if not concept_dir.exists():
        return
    for path in sorted(concept_dir.glob("*.md")):
        if path.name == "index.md":
            continue
        data = parse_frontmatter(path)
        missing = sorted(CONCEPT_REQUIRED_KEYS - set(data))
        if missing:
            fail(errors, f"{rel(path)} missing frontmatter keys: {', '.join(missing)}")
        for key in ["implemented_by", "tested_by", "related_docs", "related_memory", "related_adrs"]:
            values = data.get(key, [])
            if isinstance(values, str):
                values = [values]
            for raw in values:  # type: ignore[assignment]
                target = str(raw).split("::", 1)[0].strip()
                if not target or target.startswith(("http://", "https://")):
                    continue
                if not (ROOT / target).exists():
                    fail(errors, f"{rel(path)} references missing {key} path: {target}")


def check_markdown_links(errors: list[str]) -> None:
    candidates = [ROOT / "AGENTS.md", ROOT / "README.md"]
    docs_base = ROOT / "docs"
    if docs_base.exists():
        for item in sorted(docs_base.rglob("*.md")):
            rel_item = rel(item)
            if rel_item.startswith("docs/archive/"):
                continue
            candidates.append(item)
    for item in [
        ROOT / "dev/README.md",
        ROOT / "dev/SEARCH.md",
        ROOT / "dev/journals/index.md",
        ROOT / "dev/benchmark-candidates/index.md",
        ROOT / "dev/benchmark-candidates/README.md",
    ]:
        if item.exists():
            candidates.append(item)
    link_re = re.compile(r"\[[^\]]+\]\(([^)]+)\)")
    for path in candidates:
        if not path.exists():
            continue
        text = path.read_text(encoding="utf-8", errors="replace")
        for match in link_re.finditer(text):
            target = match.group(1).strip()
            if not target or target.startswith(("http://", "https://", "mailto:", "#")):
                continue
            if target.startswith("<") or (" " in target and not target.startswith("../")):
                continue
            base_target = target.split("#", 1)[0]
            if not base_target:
                continue
            resolved = (path.parent / base_target).resolve()
            try:
                resolved.relative_to(ROOT)
            except ValueError:
                fail(errors, f"{rel(path)} links outside repo: {target}")
                continue
            if not resolved.exists():
                fail(errors, f"{rel(path)} has broken local link: {target}")


def check_retrieval_evals(errors: list[str]) -> None:
    path = ROOT / ".agent/retrieval_evals.yaml"
    if not path.exists():
        return
    text = path.read_text(encoding="utf-8", errors="replace")
    eval_count = len(re.findall(r"^\s*- id:\s*", text, flags=re.MULTILINE))
    if eval_count < 8:
        fail(errors, f".agent/retrieval_evals.yaml has only {eval_count} evals; expected at least 8")
    for relpath in re.findall(r"(?:dev|docs|crates)/[^\s\]]+\.md", text):
        relpath = relpath.rstrip(",")
        if relpath.startswith("crates/"):
            continue
        if not (ROOT / relpath).exists():
            fail(errors, f"retrieval eval references missing path: {relpath}")


def main() -> int:
    errors: list[str] = []
    check_required_files(errors)
    check_top_level_docs(errors)
    check_agents_size(errors)
    check_json_indexes(errors)
    check_concepts(errors)
    check_markdown_links(errors)
    check_retrieval_evals(errors)
    if errors:
        print("Agent KB check failed:", file=sys.stderr)
        for msg in errors:
            print(f"- {msg}", file=sys.stderr)
        return 1
    print("Agent KB check passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
