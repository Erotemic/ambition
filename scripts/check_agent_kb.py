#!/usr/bin/env python3
"""Audit Ambition's agent-readable repository knowledge base for consistency.

This is a periodic CI/maintainer hygiene tool, not a routine validation step for
ordinary code changes. Run it deliberately when auditing broad documentation or
generated-index consistency.
"""

from __future__ import annotations

import json
import os
import pathlib
import re
import subprocess
import sys
from collections import defaultdict
from pathlib import Path

#: ⭐ ONE definition of "this is a code span, not navigation", shared with
#: `check_doc_links.py`. Two copies of that rule is how the two checkers came to
#: disagree about one file in the first place.
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from check_doc_links import _blank_code_spans  # noqa: E402

ROOT = Path(__file__).resolve().parents[1]

REQUIRED_FILES = [
    "AGENTS.md",
    "CLAUDE.md",
    "README.md",
    "docs/README.md",
    "docs/concepts/engine-mental-model.md",
    "docs/concepts/content-and-provider-boundaries.md",
    "docs/concepts/index.md",
    "docs/concepts/bevy-native-data-driven-ecs.md",
    "docs/concepts/platform-targets.md",
    "docs/concepts/tools-and-generated-content.md",
    "docs/systems/index.md",
    "docs/systems/asset-manager.md",
    "docs/systems/collision-geometry-and-secondary-physics.md",
    "docs/recipes/index.md",
    "docs/recipes/generated-music-workflow.md",
    "docs/tools/index.md",
    "docs/tools/ldtk-tools.md",
    "docs/tools/generated-audio-tools.md",
    "docs/tools/generated-visual-tools.md",
    "docs/tools/optimization-and-reporting.md",
    "docs/tools/tool-authoring-policy.md",
    "docs/mechanics/index.md",
    "docs/mechanics/expressibility-checklist.md",
    "docs/vision/index.md",
    "docs/planning/README.md",
    "docs/planning/vision.md",
    "docs/planning/status.md",
    "docs/planning/tracks.md",
    "docs/adr/README.md",
    "docs/adr/0002-engine-must-be-bevy-native.md",
    "dev/README.md",
    "dev/SEARCH.md",
    "dev/journals/index.md",
    "dev/benchmark-candidates/index.md",
    ".agent/manifest.yaml",
    ".agent/README.md",
    ".agent/retrieval_evals.yaml",
    "scripts/generate_agent_index.py",
    "scripts/agent_query.py",
    "docs/recipes/fresh-agent-navigation.md",
    "docs/concepts/architecture-review-questions.md",
]

# ⭐ `reviewer-guide.md` IS A SECOND CANONICAL TOP-LEVEL DOC, not an oversight.
# The rule exists so `docs/` does not silt up with loose files, and moving a
# guide that other documents tell agents to open BY PATH -- to satisfy a rule
# about tidiness -- would break the reference to make the checker quiet. The
# allowlist is the honest place to say "these two are meant to be here".
ALLOWED_TOP_LEVEL_DOCS = {"README.md", "reviewer-guide.md"}
CONCEPT_REQUIRED_KEYS = {"id", "aliases", "last_verified"}
AGENTS_MAX_LINES = 180

GENERATE_AGENT_INDEX_COMMAND = "python scripts/generate_agent_index.py"
GENERATED_INDEX_FILES = {
    ".agent/index/file_summaries.json": "files",
    ".agent/index/symbol_index.json": "symbols",
    ".agent/index/test_map.json": "tests",
    ".agent/index/concept_index.json": "concepts",
    ".agent/index/adr_index.json": "adrs",
    ".agent/index/tool_index.json": "tools",
    ".agent/index/archive_index.json": "archive_docs",
    ".agent/index/doc_health.json": "doc_count",
    ".agent/index/catalog.json": "counts",
    ".agent/index/crates/index.json": "crates",
}

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
    "docs/systems/avian2d-physics-foundation.md",
    "docs/systems/parry2d-geometry.md",
    "docs/systems/enemy-collision.md",
    "docs/systems/moving-platforms.md",
    "docs/recipes/glam-migration.md",
    "docs/recipes/bevy-math-engine-refactor.md",
    "docs/recipes/events-refactor-plan.md",
    "docs/recipes/room-layout-refactor.md",
    "docs/recipes/mechanics-checklist.md",
    "docs/recipes/steam-deck-deploy.md",
    "docs/recipes/crate-split-plan.md",
    "docs/recipes/music-generation-balance-notes.md",
    "docs/recipes/music-generation-pipeline-notes.md",
    "docs/recipes/music-transition-lab.md",
    "docs/recipes/music-transition-notes.md",
    "docs/recipes/procedural-tune-authoring.md",
    "docs/vision/mechanics-expressibility-checklist.md",
    "docs/agent_states/gpt_5_5_20260430.md",
    "docs/agent_states/gpt_5_5_20260501-v1.md",
    "docs/agent_states/gpt_5_5_20260501-v2.md",
]

DUPLICATE_ARCHIVE_PATHS = [
    "docs/archive/superseded-migrations/bevy-math-engine-refactor.md",
    "docs/archive/superseded-migrations/events-refactor-plan.md",
    "docs/archive/superseded-migrations/glam-migration.md",
    "docs/archive/superseded-migrations/room-layout-refactor.md",
    "docs/archive/old-system-notes/room-graph-data-model.md",
    "docs/archive/old-system-notes/rooms-and-camera.md",
]

STALE_ACTIVE_PATTERNS = [
    (
        re.compile(
            r"docs/(AGENT_HANDOFF|CURRENT_STATE|GOAL_STATE|lessons_learned|redirects)\.md"
        ),
        "removed top-level doc stub",
    ),
    (re.compile(r"0002-engine-may-be-bevy-native\.md"), "superseded ADR path"),
    (
        re.compile(
            r"RON rooms|RON room authoring|RON-backed RoomSet|sandbox\.ron.*rooms",
            re.IGNORECASE,
        ),
        "stale RON-room phrasing",
    ),
]

STALE_RECIPE_OR_SYSTEM_PATTERNS = [
    (re.compile(r"\bthis patch introduces\b", re.IGNORECASE), "patch-era phrasing"),
    (re.compile(r"\bmigration plan\b", re.IGNORECASE), "migration-plan phrasing"),
    (re.compile(r"\blanded roadmap\b", re.IGNORECASE), "landed-roadmap phrasing"),
]


# A SOFT budget on the whole planning corpus, surfaced as a WARNING, never an error. The nudge
# it carries is real, though: when the corpus grows, archive the parts that are LONG DONE rather
# than trimming live plans. Size is NEVER a build-failing gate here (see the note on
# PLANNING_FILE_MAX_LINES); the only hard planning gate is content correctness — the
# stale-phrasing patterns below.
PLANNING_TOTAL_SOFT_BUDGET = 10_500
# Per-file SOFT ceilings on the always-live index docs — warnings, not gates. A hard
# line ceiling is a gameable proxy (pack everything onto one line and it passes), and
# blocking on plan SIZE just trains an agent to game the metric instead of writing a
# good plan. These numbers are generous headroom to notice a doc ballooning, nothing
# more; a human decides if a plan actually needs pruning.
PLANNING_FILE_MAX_LINES = {
    "docs/planning/README.md": 200,
    "docs/planning/status.md": 360,
    "docs/planning/tracks.md": 440,
    "docs/planning/roadmap.md": 360,
}

FORBIDDEN_PLANNING_PATTERNS = [
    (re.compile(r"8 errors, 1 warning", re.IGNORECASE), "stale boss-validator count"),
    (re.compile(r"1155/718(?:/471/1379)?"), "stale snapshot split count"),
    (re.compile(r"crates/ambition_platformer2d_actor_monolith/build\.rs"), "retired sprite embed owner"),
    (re.compile(r"Objective::Protect\b"), "nonexistent objective variant"),
    (
        re.compile(r"Status:\s*\*\*?IMPLEMENTED.*E0.?E7", re.IGNORECASE),
        "unsupported encounter implemented banner",
    ),
    (
        re.compile(r"ONE encounter-entity model"),
        "unsupported encounter-unification claim",
    ),
    (
        re.compile(
            r"Both boss and wave are entities now \(one snapshot representation\)",
            re.IGNORECASE,
        ),
        "unsupported encounter snapshot claim",
    ),
    (re.compile(r"OV1 blocks", re.IGNORECASE), "stale OV1 blocker claim"),
    (re.compile(r"the shell draws nothing", re.IGNORECASE), "stale renderer-gap claim"),
    (re.compile(r"44 workspace crates", re.IGNORECASE), "stale workspace count"),
    (re.compile(r"Q4 flagged below", re.IGNORECASE), "resolved Q4 presented as open"),
    (
        re.compile(r"may or may not split further", re.IGNORECASE),
        "resolved actor-carve uncertainty",
    ),
]


# Narrow code-comment documentation-link scanning: only repo-ANCHORED references
# (`docs/...` and `../docs/...`), backticked or not. Non-anchored `.md` shorthands
# (e.g. `engine/foo.md`, `brain/README.md`) are deliberately skipped — resolving
# them guesses a base directory and yields false positives (decision: reduce scope
# rather than over-interpret path-like strings).
DOC_REF_RE = re.compile(r"(?:\.\./)*docs/[A-Za-z0-9_./+-]+\.md")


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
            if path.suffix.lower() in {
                ".md",
                ".rs",
                ".toml",
                ".py",
                ".yaml",
                ".yml",
                ".sh",
            }:
                out.append(path)
    return sorted(set(out))


def check_required_files(errors: list[str]) -> None:
    for item in REQUIRED_FILES:
        if not (ROOT / item).exists():
            fail(errors, f"missing required KB file: {item}")


def check_forbidden_live_paths(errors: list[str]) -> None:
    for item in FORBIDDEN_LIVE_PATHS:
        if (ROOT / item).exists():
            fail(
                errors,
                f"forbidden live stale doc remains: {item}; archive or delete it",
            )
    for item in DUPLICATE_ARCHIVE_PATHS:
        if (ROOT / item).exists():
            fail(
                errors,
                f"duplicate archived copy remains: {item}; keep one canonical archive copy",
            )


def check_top_level_docs(errors: list[str]) -> None:
    docs = ROOT / "docs"
    for path in sorted(docs.glob("*.md")):
        if path.name not in ALLOWED_TOP_LEVEL_DOCS:
            fail(
                errors,
                f"unexpected top-level docs file: {rel(path)}; only "
                f"{sorted(ALLOWED_TOP_LEVEL_DOCS)} may live at docs/ top level",
            )


def check_stale_active_references(errors: list[str]) -> None:
    for path in active_text_files():
        text = path.read_text(encoding="utf-8", errors="replace")
        for pattern, label in STALE_ACTIVE_PATTERNS:
            if pattern.search(text):
                fail(errors, f"{rel(path)} contains {label}: {pattern.pattern}")


def check_agents_size(errors: list[str], warnings: list[str]) -> None:
    path = ROOT / "AGENTS.md"
    if not path.exists():
        return
    lines = path.read_text(encoding="utf-8", errors="replace").splitlines()
    if len(lines) <= AGENTS_MAX_LINES:
        return
    # **REPORT THE READING COST, not only the line count.** The budget is
    # LINE-based and this file's heaviest single item is one line of several
    # hundred words, so "you are N over" points at a number that can be satisfied
    # by moving the wrong things — join two short lines and the metric improves
    # while the file gets no shorter to read. Naming the heaviest lines turns an
    # unactionable overage into a decision somebody can actually make: route THIS
    # paragraph into a concept doc, or waive the budget for it.
    #
    # the GATE is still the line count, deliberately. Which quantity to budget
    # is a maintainer call, and a checker that quietly switched to
    # words would be making it. This only stops the message hiding where the
    # weight is.
    #
    # The overage is a warning rather than a gate because the right split is a
    # maintainer/document-routing decision, not something a line-count proxy can
    # decide. Other KB correctness checks remain independent and may still fail.
    heaviest = sorted(
        ((len(line.split()), i + 1, line) for i, line in enumerate(lines)),
        reverse=True,
    )[:3]
    words = sum(len(line.split()) for line in lines)
    detail = "; ".join(
        f"line {number} carries {count} words" for count, number, _ in heaviest if count > 40
    )
    warnings.append(
        f"AGENTS.md has {len(lines)} lines ({words} words); keep it <= "
        f"{AGENTS_MAX_LINES} and route to docs instead"
        + (f". Heaviest: {detail}" if detail else "")
        + " [WARNING, not an error: the overage is queue F6's open routing"
        " decision. Every other check in this file is fatal.]"
    )


def git_tracked_paths(pathspec: str) -> list[str]:
    try:
        proc = subprocess.run(
            ["git", "ls-files", "--", pathspec],
            cwd=ROOT,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.DEVNULL,
            check=False,
        )
    except OSError:
        return []
    if proc.returncode != 0:
        return []
    return [line.strip() for line in proc.stdout.splitlines() if line.strip()]


def check_generated_indexes(errors: list[str]) -> None:
    tracked = git_tracked_paths(".agent/index")
    if tracked:
        shown = ", ".join(tracked[:5])
        suffix = "" if len(tracked) <= 5 else f", ... and {len(tracked) - 5} more"
        fail(
            errors,
            ".agent/index/ is generated and must not be tracked by Git; "
            f"remove it with `git rm -r --cached .agent/index` ({shown}{suffix})",
        )

    missing = [
        relpath for relpath in GENERATED_INDEX_FILES if not (ROOT / relpath).exists()
    ]
    if missing:
        fail(
            errors,
            ".agent/index/ is generated, ignored by Git, and currently missing or incomplete. "
            f"Run `{GENERATE_AGENT_INDEX_COMMAND}` before using agent file/symbol/test lookup "
            "or before running this check. Missing: " + ", ".join(missing),
        )
        return

    for relpath, payload_key in GENERATED_INDEX_FILES.items():
        path = ROOT / relpath
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
        for key in [
            "implemented_by",
            "tested_by",
            "related_docs",
            "related_adrs",
            "related_memory",
        ]:
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
        # ⛔ A LINK INSIDE A CODE SPAN IS AN EXAMPLE, NOT NAVIGATION, and this
        # check called one broken for it: `yardrat-open-measurements.md` writes
        # `[text](other.md#anchor)` in backticks to NAME the class of link it is
        # discussing, and this exited 1 on it while `check_doc_links.py` --
        # scanning the same tree -- reported 927 links clean. Two link checkers
        # printing two answers about one file.
        # ⇒ Reuse that checker's blanking rather than write a second rule.
        for match in link_re.finditer(_blank_code_spans(text)):
            target = match.group(1).strip()
            if not target or target.startswith(("http://", "https://", "mailto:", "#")):
                continue
            if target.startswith("<") or (
                " " in target and not target.startswith("../")
            ):
                continue
            base_target = target.split("#", 1)[0]
            if not base_target:
                continue
            # ⛔⛔ NORMALISE, DO NOT `resolve()`. `Path.resolve()` follows
            # SYMLINKS, and an agent worktree seeds `.agent/README.md` as a
            # symlink into the primary tree — so every link to it resolved to
            # `<primary>/.agent/README.md`, landed outside this ROOT, and was
            # reported as "links outside repo". Two false errors in every
            # worktree, which is where nearly every agent runs this. The
            # question being asked is about the PATH the document names, not
            # about where the filesystem stores the bytes.
            joined = os.path.normpath(os.path.join(path.parent, base_target))
            resolved = pathlib.Path(joined)
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
        fail(
            errors,
            f".agent/retrieval_evals.yaml has only {eval_count} evals; expected at least 10",
        )
    for relpath in re.findall(r"(?:dev|docs|crates)/[^\s\]]+\.md", text):
        relpath = relpath.rstrip(",")
        if relpath.startswith("crates/"):
            continue
        if not (ROOT / relpath).exists():
            fail(errors, f"retrieval eval references missing path: {relpath}")


def check_adr_current_implications(errors: list[str]) -> None:
    for path in sorted((ROOT / "docs" / "adr").glob("*.md")):
        if path.name == "README.md":
            continue
        text = path.read_text(encoding="utf-8", errors="replace")
        if "## Status" not in text:
            fail(errors, f"{rel(path)} missing ## Status")
        if "## Current implications for agents" not in text:
            fail(errors, f"{rel(path)} missing ## Current implications for agents")


def check_active_doc_phrasing(errors: list[str]) -> None:
    for root in [ROOT / "docs" / "recipes", ROOT / "docs" / "systems"]:
        if not root.exists():
            continue
        for path in sorted(root.glob("*.md")):
            text = path.read_text(encoding="utf-8", errors="replace")
            for pattern, label in STALE_RECIPE_OR_SYSTEM_PATTERNS:
                if pattern.search(text):
                    fail(
                        errors,
                        f"{rel(path)} contains {label}; archive old patch/migration notes",
                    )


def planning_markdown_files() -> list[Path]:
    root = ROOT / "docs" / "planning"
    return sorted(path for path in root.rglob("*.md") if path.is_file())


def mask_rust_noncode(text: str) -> str:
    """Mask comments and literals while preserving code positions and newlines."""

    out = list(text)
    n = len(text)
    i = 0
    block_depth = 0
    state = "code"
    raw_hashes = 0

    def blank(index: int) -> None:
        if out[index] != "\n":
            out[index] = " "

    while i < n:
        if state == "line_comment":
            if text[i] == "\n":
                state = "code"
            else:
                blank(i)
            i += 1
            continue

        if state == "block_comment":
            if text.startswith("/*", i):
                blank(i)
                if i + 1 < n:
                    blank(i + 1)
                block_depth += 1
                i += 2
            elif text.startswith("*/", i):
                blank(i)
                if i + 1 < n:
                    blank(i + 1)
                block_depth -= 1
                i += 2
                if block_depth == 0:
                    state = "code"
            else:
                blank(i)
                i += 1
            continue

        if state == "string":
            if text[i] == "\\":
                blank(i)
                if i + 1 < n:
                    blank(i + 1)
                i += 2
            elif text[i] == '"':
                blank(i)
                state = "code"
                i += 1
            else:
                blank(i)
                i += 1
            continue

        if state == "char":
            if text[i] == "\\":
                blank(i)
                if i + 1 < n:
                    blank(i + 1)
                i += 2
            elif text[i] == "'":
                blank(i)
                state = "code"
                i += 1
            else:
                blank(i)
                i += 1
            continue

        if state == "raw":
            terminator = '"' + ('#' * raw_hashes)
            if text.startswith(terminator, i):
                for j in range(i, min(i + len(terminator), n)):
                    blank(j)
                i += len(terminator)
                state = "code"
            else:
                blank(i)
                i += 1
            continue

        if text.startswith("//", i):
            blank(i)
            blank(i + 1)
            state = "line_comment"
            i += 2
            continue
        if text.startswith("/*", i):
            blank(i)
            blank(i + 1)
            block_depth = 1
            state = "block_comment"
            i += 2
            continue

        raw = re.match(r"(?:br|r)(#+)?\"", text[i:])
        if raw:
            token = raw.group(0)
            raw_hashes = len(raw.group(1) or "")
            for j in range(i, i + len(token)):
                blank(j)
            i += len(token)
            state = "raw"
            continue

        if text[i] == '"':
            blank(i)
            state = "string"
            i += 1
            continue

        # Treat a quote as a char literal only when a closing quote is nearby;
        # this avoids masking Rust lifetimes such as `'a`.
        if text[i] == "'":
            close = i + 1
            escaped = False
            while close < min(i + 8, n) and text[close] != "\n":
                if text[close] == "'" and not escaped:
                    break
                escaped = text[close] == "\\" and not escaped
                if text[close] != "\\":
                    escaped = False
                close += 1
            if close < min(i + 8, n) and text[close] == "'":
                blank(i)
                state = "char"
                i += 1
                continue

        i += 1

    return "".join(out)


def check_planning_front_end(errors: list[str], warnings: list[str]) -> None:
    files = planning_markdown_files()
    total_lines = sum(
        len(path.read_text(encoding="utf-8", errors="replace").splitlines())
        for path in files
    )
    if total_lines > PLANNING_TOTAL_SOFT_BUDGET:
        # A nudge, not a gate: planning ahead is legitimate. Clean up long-done
        # history into docs/archive rather than trimming live plans.
        warnings.append(
            f"docs/planning has {total_lines} lines (soft budget {PLANNING_TOTAL_SOFT_BUDGET}); "
            "archive sections that are long done rather than trimming live plans",
        )

    for rpath, limit in PLANNING_FILE_MAX_LINES.items():
        path = ROOT / rpath
        if not path.exists():
            continue
        lines = len(path.read_text(encoding="utf-8", errors="replace").splitlines())
        if lines > limit:
            # A nudge, never a gate. A hard line ceiling is a gameable proxy (one
            # long line defeats it) and blocking on plan SIZE just trains the agent
            # to game the metric. Surface it so a ballooning index gets noticed;
            # let a human decide whether it actually needs pruning.
            warnings.append(f"{rpath} has {lines} lines (soft ceiling {limit}); consider pruning")

    for path in files:
        text = path.read_text(encoding="utf-8", errors="replace")
        for pattern, label in FORBIDDEN_PLANNING_PATTERNS:
            if pattern.search(text):
                fail(errors, f"{rel(path)} contains {label}: {pattern.pattern}")


def check_archive_duplicates(errors: list[str]) -> None:
    archive = ROOT / "docs" / "archive"
    if not archive.exists():
        return
    by_name: dict[str, list[str]] = defaultdict(list)
    for path in archive.rglob("*.md"):
        if path.name in {"README.md", "index.md"}:
            continue
        by_name[path.name].append(rel(path))
    for name, paths in sorted(by_name.items()):
        if len(paths) > 1:
            fail(errors, f"archive has duplicate basename {name}: {', '.join(paths)}")


def check_tool_docs(errors: list[str]) -> None:
    tools = ROOT / "tools"
    docs_text = (
        "\n".join(
            p.read_text(encoding="utf-8", errors="replace")
            for p in (ROOT / "docs" / "tools").glob("*.md")
        )
        if (ROOT / "docs" / "tools").exists()
        else ""
    )
    if not tools.exists():
        return
    for path in sorted(tools.iterdir()):
        if not path.is_dir() or path.name == "experimental":
            continue
        if (path / "README.md").exists() and path.name not in docs_text:
            fail(
                errors,
                f"tool with README missing from docs/tools coverage: tools/{path.name}",
            )


def rust_comment_text(text: str) -> str:
    """Return only Rust comment characters; code and string/char/raw literals are
    dropped (newlines preserved). Lets the doc-link scan look at comments rather
    than path-like string literals."""
    out: list[str] = []
    i, n = 0, len(text)
    while i < n:
        if text.startswith("//", i):
            j = text.find("\n", i)
            j = n if j == -1 else j
            out.append(text[i:j])
            i = j
            continue
        if text.startswith("/*", i):
            depth, j = 1, i + 2
            while j < n and depth:
                if text.startswith("/*", j):
                    depth += 1
                    j += 2
                elif text.startswith("*/", j):
                    depth -= 1
                    j += 2
                else:
                    j += 1
            out.append(text[i:j])
            i = j
            continue
        raw = re.match(r'(?:b?r)(#*)"', text[i:])
        if raw:
            term = '"' + "#" * len(raw.group(1))
            k = text.find(term, i + len(raw.group(0)))
            k = n if k == -1 else k + len(term)
            out.append("\n" * text.count("\n", i, k))
            i = k
            continue
        if text[i] == '"':
            j = i + 1
            while j < n:
                if text[j] == "\\":
                    j += 2
                    continue
                if text[j] == '"':
                    j += 1
                    break
                j += 1
            out.append("\n" * text.count("\n", i, j))
            i = j
            continue
        char = re.match(r"'(?:\\.|[^'\\])'", text[i:])
        if char:
            i += len(char.group(0))
            continue
        out.append("\n" if text[i] == "\n" else "")
        i += 1
    return "".join(out)


def doc_refs_in_comment_text(comment: str) -> set[str]:
    return set(DOC_REF_RE.findall(comment))


def check_code_comment_doc_links(errors: list[str]) -> None:
    """Verify repo-ANCHORED documentation references inside Rust comments exist.
    Deliberately narrow: only `docs/...` and `../docs/...` paths (backticked or
    not), in COMMENTS only. Non-anchored backticked `.md` shorthands (e.g.
    `engine/foo.md`) are NOT recognized — resolving them guesses a base directory.
    No arbitrary prose, external URLs, or string literals."""
    for base in [ROOT / "crates", ROOT / "game"]:
        if not base.exists():
            continue
        for path in sorted(base.rglob("*.rs")):
            text = path.read_text(encoding="utf-8", errors="replace")
            if "docs/" not in text:
                continue
            for ref in sorted(doc_refs_in_comment_text(rust_comment_text(text))):
                resolved = (
                    (ROOT / ref) if ref.startswith("docs/") else (path.parent / ref)
                ).resolve()
                try:
                    resolved.relative_to(ROOT)
                except ValueError:
                    continue
                if not resolved.exists():
                    fail(errors, f"{rel(path)} comment references missing doc: {ref}")


def check_code_comment_link_self_test(errors: list[str]) -> None:
    # A repo-anchored docs ref (backticked) is recognized.
    if "docs/planning/status.md" not in doc_refs_in_comment_text(
        rust_comment_text("//! see `docs/planning/status.md` for the current state\n")
    ):
        fail(errors, "code-comment link self-test: valid docs ref not recognized")
    # A non-anchored `.md` shorthand is deliberately skipped (no false positive).
    if doc_refs_in_comment_text(rust_comment_text("//! see `engine/architecture.md`\n")):
        fail(errors, "code-comment link self-test: non-anchored .md shorthand should be skipped")
    # A doc path inside a string literal is not a comment reference.
    if doc_refs_in_comment_text(rust_comment_text('fn f() { let _ = "docs/none.md"; }\n')):
        fail(errors, "code-comment link self-test: string literal treated as a comment ref")
    # A nonexistent anchored reference resolves as missing (the enforcement path).
    poison = doc_refs_in_comment_text(
        rust_comment_text("// see docs/planning/__poison_missing__.md\n")
    )
    if not poison or (ROOT / next(iter(poison))).exists():
        fail(errors, "code-comment link self-test: poison ref unexpectedly resolvable")


def main() -> int:
    errors: list[str] = []
    warnings: list[str] = []
    check_required_files(errors)
    check_forbidden_live_paths(errors)
    check_top_level_docs(errors)
    check_stale_active_references(errors)
    check_agents_size(errors, warnings)
    check_generated_indexes(errors)
    check_concepts(errors)
    check_markdown_links(errors)
    check_code_comment_link_self_test(errors)
    check_code_comment_doc_links(errors)
    check_retrieval_evals(errors)
    check_adr_current_implications(errors)
    check_active_doc_phrasing(errors)
    check_planning_front_end(errors, warnings)
    check_archive_duplicates(errors)
    check_tool_docs(errors)
    if warnings:
        print("Agent KB warnings (non-fatal):", file=sys.stderr)
        for msg in warnings:
            print(f"- {msg}", file=sys.stderr)
    if errors:
        print("Agent KB check failed:", file=sys.stderr)
        for msg in errors:
            print(f"- {msg}", file=sys.stderr)
        return 1
    print("Agent KB check passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
