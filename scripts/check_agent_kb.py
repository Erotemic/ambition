#!/usr/bin/env python3
"""Lightweight lint for Ambition's agent-readable repository knowledge base."""
from __future__ import annotations

import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]

REQUIRED_FILES = [
    "AGENTS.md",
    "CLAUDE.md",
    "README.md",
    "docs/README.md",
    "docs/current/state.md",
    "docs/current/risks.md",
    "docs/current/next.md",
    "docs/concepts/index.md",
    "docs/concepts/bevy-native-data-driven-ecs.md",
    "docs/concepts/platform-targets.md",
    "docs/concepts/tools-and-generated-content.md",
    "docs/systems/index.md",
    "docs/recipes/index.md",
    "docs/tools/index.md",
    "docs/mechanics/index.md",
    "docs/mechanics/expressibility-checklist.md",
    "docs/vision/index.md",
    "docs/planning/index.md",
    "docs/history/index.md",
    "docs/adr/README.md",
    "docs/adr/0002-engine-must-be-bevy-native.md",
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

ALLOWED_TOP_LEVEL_DOCS = {"README.md"}
CONCEPT_REQUIRED_KEYS = {"id", "aliases", "last_verified"}
AGENTS_MAX_LINES = 150

FORBIDDEN_LIVE_PATHS = [
    "docs/AGENT_HANDOFF.md",
    "docs/CURRENT_STATE.md",
    "docs/GOAL_STATE.md",
    "docs/lessons_learned.md",
    "docs/redirects.md",
    "docs/adr/0002-engine-may-be-bevy-native.md",
    "docs/systems/interaction-hazard-actor-skeleton.md",
    "docs/systems/data-driven-manifest.md",
    "docs/systems/rooms-and-camera.md",
    "docs/systems/room-graph-data-model.md",
    "docs/systems/ldtk-runtime-spine.md",
    "docs/recipes/glam-migration.md",
    "docs/recipes/bevy-math-engine-refactor.md",
    "docs/recipes/events-refactor-plan.md",
    "docs/recipes/room-layout-refactor.md",
    "docs/recipes/mechanics-checklist.md",
    "docs/recipes/steam-deck-deploy.md",
    "docs/agent_states/gpt_5_5_20260430.md",
    "docs/agent_states/gpt_5_5_20260501-v1.md",
    "docs/agent_states/gpt_5_5_20260501-v2.md",
]

STALE_ACTIVE_PATTERNS = [
    (re.compile(r"docs/(AGENT_HANDOFF|CURRENT_STATE|GOAL_STATE|lessons_learned|redirects)\.md"), "removed top-level doc stub"),
    (re.compile(r"0002-engine-may-be-bevy-native\.md"), "superseded ADR path"),
    (re.compile(r"RON rooms|RON room authoring|RON-backed RoomSet|sandbox\.ron.*rooms", re.IGNORECASE), "stale RON-room phrasing"),
]


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


def active_text_files() -> list[Path]:
    out: list[Path] = []
    for base in [ROOT / "AGENTS.md", ROOT / "README.md"]:
        if base.exists():
            out.append(base)
    for root in [ROOT / "docs", ROOT / "dev", ROOT / "crates", ROOT / "tools"]:
        if not root.exists():
            continue
        for path in root.rglob("*"):
            if not path.is_file():
                continue
            r = rel(path)
            if r.startswith(("docs/archive/", "tools/experimental/")):
                continue
            if path.suffix.lower() in {".md", ".rs", ".toml", ".py", ".yaml", ".yml", ".sh"}:
                out.append(path)
    return sorted(set(out))


def check_required_files(errors: list[str]) -> None:
    for item in REQUIRED_FILES:
        if not (ROOT / item).exists():
            fail(errors, f"missing required KB file: {item}")


def check_forbidden_live_paths(errors: list[str]) -> None:
    for item in FORBIDDEN_LIVE_PATHS:
        if (ROOT / item).exists():
            fail(errors, f"forbidden live stale doc remains: {item}; archive or delete it")


def check_top_level_docs(errors: list[str]) -> None:
    docs = ROOT / "docs"
    for path in sorted(docs.glob("*.md")):
        if path.name not in ALLOWED_TOP_LEVEL_DOCS:
            fail(errors, f"unexpected top-level docs file: {rel(path)}; only docs/README.md should remain")


def check_stale_active_references(errors: list[str]) -> None:
    for path in active_text_files():
        text = path.read_text(encoding="utf-8", errors="replace")
        for pattern, label in STALE_ACTIVE_PATTERNS:
            if pattern.search(text):
                fail(errors, f"{rel(path)} contains {label}: {pattern.pattern}")


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
        except Exception as ex:
            fail(errors, f"{relpath} is not valid JSON: {ex}")
            continue
        if payload_key not in data:
            fail(errors, f"{relpath} missing key {payload_key!r}")


def check_concepts(errors: list[str]) -> None:
    cdir = ROOT / "docs" / "concepts"
    if not cdir.exists():
        return
    for path in sorted(cdir.glob("*.md")):
        if path.name == "index.md":
            continue
        fm = parse_frontmatter(path)
        missing = sorted(CONCEPT_REQUIRED_KEYS - set(fm))
        if missing:
            fail(errors, f"{rel(path)} missing frontmatter keys: {', '.join(missing)}")
        for key in ["implemented_by", "tested_by", "related_docs", "related_adrs", "related_memory"]:
            values = fm.get(key)
            if not values:
                continue
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
    for root in [ROOT / "docs", ROOT / "dev"]:
        if root.exists():
            for item in sorted(root.rglob("*.md")):
                rel_item = rel(item)
                if rel_item.startswith("docs/archive/"):
                    continue
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
    if eval_count < 10:
        fail(errors, f".agent/retrieval_evals.yaml has only {eval_count} evals; expected at least 10")
    for relpath in re.findall(r"(?:dev|docs|crates)/[^\s\]]+\.md", text):
        relpath = relpath.rstrip(",")
        if relpath.startswith("crates/"):
            continue
        if not (ROOT / relpath).exists():
            fail(errors, f"retrieval eval references missing path: {relpath}")


def main() -> int:
    errors: list[str] = []
    check_required_files(errors)
    check_forbidden_live_paths(errors)
    check_top_level_docs(errors)
    check_stale_active_references(errors)
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
