#!/usr/bin/env python3
"""Check local Markdown links in active Ambition docs.

This intentionally scans active knowledge-base docs and root entrypoints, not
historical archives or patch files. Archives preserve stale paths on purpose.
"""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path
from urllib.parse import unquote, urlparse

LINK_RE = re.compile(r"(?<!!)\[[^\]]+\]\(([^)]+)\)")
REF_RE = re.compile(r"^\s*\[[^\]]+\]:\s*(\S+)", re.MULTILINE)

SKIP_DIR_PARTS = {
    ".git",
    ".agent",
    "target",
    "debug_traces",
    "docs/archive",
    "docs/patches",
}

ROOT_MARKDOWN = {
    "AGENTS.md",
    "CLAUDE.md",
    "FEATURES.md",
    "README.md",
    "TODO.md",
}

STALE_PATH_HINTS = [
    "docs/moving_platforms.md",
    "docs/progression_systems_2026-05-05.md",
    "docs/crate_split_plan.md",
    "tools/audio/",
    "tools/validate_ambition_ldtk.py",
    "tools/author_ldtk_area.py",
]


def should_skip(path: Path) -> bool:
    rel = path.as_posix()
    if path.name.startswith(".") and path.name not in ROOT_MARKDOWN:
        return True
    return any(rel == part or rel.startswith(part + "/") for part in SKIP_DIR_PARTS)


def iter_markdown(root: Path):
    for name in sorted(ROOT_MARKDOWN):
        p = root / name
        if p.exists():
            yield p
    docs = root / "docs"
    if docs.exists():
        for p in sorted(docs.rglob("*.md")):
            rel = p.relative_to(root)
            if not should_skip(rel):
                yield p


def strip_angle(target: str) -> str:
    target = target.strip()
    if target.startswith("<") and target.endswith(">"):
        target = target[1:-1]
    return target


def is_external(target: str) -> bool:
    parsed = urlparse(target)
    return bool(
        parsed.scheme and parsed.scheme not in {"", "file"}
    ) or target.startswith("mailto:")


def local_target_exists(root: Path, source: Path, target: str) -> bool:
    target = strip_angle(target).split()[0]
    if not target or target.startswith("#") or is_external(target):
        return True
    target = target.split("#", 1)[0]
    if not target:
        return True
    target = unquote(target)
    if target.startswith("/"):
        candidate = root / target.lstrip("/")
    else:
        candidate = source.parent / target
    return candidate.exists()


def collect_links(text: str):
    for match in LINK_RE.finditer(text):
        target = match.group(1)
        if target:
            yield match.start(1), target
    for match in REF_RE.finditer(text):
        yield match.start(1), match.group(1)


def line_for_offset(text: str, offset: int) -> int:
    return text.count("\n", 0, offset) + 1


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", default=".", help="repository root")
    args = parser.parse_args()

    root = Path(args.root).resolve()
    errors: list[str] = []

    for path in iter_markdown(root):
        text = path.read_text(encoding="utf8")
        rel = path.relative_to(root)
        for offset, target in collect_links(text):
            if not local_target_exists(root, path, target):
                errors.append(
                    f"{rel}:{line_for_offset(text, offset)} broken local link: {target}"
                )
        for stale in STALE_PATH_HINTS:
            if stale in text:
                errors.append(f"{rel}: stale path hint still present: {stale}")

    if errors:
        print("Documentation link check failed:", file=sys.stderr)
        for err in errors:
            print(f"  - {err}", file=sys.stderr)
        return 1

    print("Documentation link check passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
