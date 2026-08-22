#!/usr/bin/env python3
"""Generate lightweight agent navigation indexes for Ambition.

The indexes are intentionally simple, reviewable JSON. They are navigation aids,
not source-of-truth replacements for code, ADRs, or concept pages.
"""

from __future__ import annotations

import json
import os
import re
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
INDEX_DIR = ROOT / ".agent" / "index"

SKIP_DIRS = {".git", ".agent", ".worktrees", "target", ".venv", "__pycache__"}
TEXT_EXTS = {".md", ".rs", ".toml", ".ron", ".yaml", ".yml", ".py", ".sh", ".json"}


def generated_meta() -> dict[str, str]:
    """Stable metadata for committed navigation indexes.

    Timestamps are the SOURCE COMMIT's committer time, not wall-clock time, so
    regenerating at the same commit is byte-stable. The commit stamp is what
    `agent_query.py` compares against HEAD to warn about a stale index.
    """
    meta = {"generator": "scripts/generate_agent_index.py"}
    commit = _git(["rev-parse", "--short=12", "HEAD"])
    if commit:
        meta["generated_from_commit"] = commit
    commit_time = _git(["show", "-s", "--format=%cI", "HEAD"])
    if commit_time:
        meta["generated_at"] = commit_time
    return meta


def _git(args: list[str]) -> str | None:
    try:
        out = subprocess.run(
            ["git", *args], cwd=ROOT, capture_output=True, text=True, check=True
        )
    except (OSError, subprocess.CalledProcessError):
        return None
    return out.stdout.strip() or None


def _git_file_universe() -> set[Path] | None:
    """The set of files the index should describe: tracked plus
    untracked-but-not-ignored. Respecting .gitignore is what keeps stale
    working-tree snapshots (`.tmp-*-stage/`), archive tarballs, and generated
    output out of the index — indexing those has produced confidently-wrong
    owners (phantom crates scored above live ones)."""
    listing = _git(["ls-files", "--cached", "--others", "--exclude-standard"])
    if listing is None:
        return None
    return {ROOT / line for line in listing.splitlines() if line}


def iter_files() -> list[Path]:
    # `os.walk` with in-place `dirnames` filtering so SKIP_DIRS actually
    # prevents descent. The old `rglob("*")` form filtered after the fact,
    # which meant the script still recursed into `target/` (millions of
    # files on Android/desktop builds) and exhausted file descriptors on
    # virtiofs hosts (EMFILE: Too many open files).
    universe = _git_file_universe()
    out: list[Path] = []
    for dirpath, dirnames, filenames in os.walk(ROOT):
        dirnames[:] = [d for d in dirnames if d not in SKIP_DIRS]
        for name in filenames:
            path = Path(dirpath) / name
            if path.suffix in TEXT_EXTS:
                if universe is not None and path not in universe:
                    continue
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


# Canonical entry documents, in suggested reading order.
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


RESOLVED_GRAPH_MEANING = (
    "What cargo RESOLVES for this workspace, from `cargo metadata` — feature "
    "unification applied, so every edge here is actually compiled. Requires a "
    "Rust toolchain at GENERATION time, not at read time. Use this to answer "
    "'what is actually linked'. ⚠ still workspace-wide: a specific "
    "consumer's closure is a different question, measured per-consumer with "
    "`cargo tree --edges normal` in that consumer's own workspace (see "
    "scripts/baselines/capability-footprint-baseline.json)."
)


def build_resolved_dependency_graph() -> dict[str, object]:
    """The cargo-resolved crate graph, or an explicit record that it is absent.

    ⭐ **written by the machine that HAS cargo, read by the one that does not.**
    That asymmetry is the whole point: `archive_agent_source.sh` runs on a dev
    box and ships the answer to an agent with no Rust toolchain, who otherwise
    has only the manifests — and manifests over-report, because an optional edge
    nothing enables looks identical to a real one.

    ⚠ **absence is recorded, not implied.** A missing file cannot be told apart
    from a generator that ran and found nothing, so a failed or skipped run
    writes `available: false` with the reason. Silence would read as "this
    workspace has no dependencies", which is the confidently-wrong answer.
    """
    base: dict[str, object] = {
        "schema_version": 1,
        "generator": "scripts/generate_agent_index.py",
        "graph": "resolved",
        "means": RESOLVED_GRAPH_MEANING,
        "requires_toolchain": True,
        "generated_from_commit": _git(["rev-parse", "--short=12", "HEAD"]) or "unknown",
    }
    try:
        proc = subprocess.run(
            ["cargo", "metadata", "--format-version", "1", "--locked"],
            cwd=ROOT,
            capture_output=True,
            text=True,
            check=False,
            timeout=180,
        )
    except (OSError, subprocess.SubprocessError) as exc:
        return {**base, "available": False, "reason": f"cargo not runnable: {exc}"}
    if proc.returncode != 0:
        detail = (proc.stderr or "").strip().splitlines()
        return {
            **base,
            "available": False,
            "reason": "cargo metadata failed: " + (detail[-1] if detail else "unknown"),
        }
    try:
        meta = json.loads(proc.stdout)
    except json.JSONDecodeError as exc:
        return {**base, "available": False, "reason": f"unparseable cargo metadata: {exc}"}

    name_by_id = {
        str(pkg.get("id")): str(pkg.get("name"))
        for pkg in meta.get("packages", [])
        if isinstance(pkg, dict)
    }
    member_ids = {str(pid) for pid in meta.get("workspace_members", [])}
    members = sorted(name_by_id[pid] for pid in member_ids if pid in name_by_id)

    # NORMAL edges only. A dev-dependency is not linked into the library a
    # consumer builds, and counting it would inflate every "what does this cost
    # me" answer with test-only crates.
    edges: dict[str, list[str]] = {}
    features: dict[str, list[str]] = {}
    all_edges: dict[str, list[str]] = {}
    for node in meta.get("resolve", {}).get("nodes", []):
        if not isinstance(node, dict):
            continue
        node_name = name_by_id.get(str(node.get("id")))
        if node_name is None:
            continue
        normal: list[str] = []
        for dep in node.get("deps", []):
            if not isinstance(dep, dict):
                continue
            kinds = dep.get("dep_kinds") or []
            # `kind: null` is cargo's spelling of a normal dependency.
            if not any(isinstance(k, dict) and k.get("kind") is None for k in kinds):
                continue
            dep_name = name_by_id.get(str(dep.get("pkg")))
            if dep_name:
                normal.append(dep_name)
        all_edges[node_name] = sorted(set(normal))
        if str(node.get("id")) in member_ids:
            edges[node_name] = sorted(set(normal))
            feats = node.get("features")
            if isinstance(feats, list) and feats:
                features[node_name] = sorted(str(f) for f in feats)

    # The transitive `ambition_*` closure per member — the capability-footprint
    # question, precomputed because a reader without cargo cannot walk it and a
    # reader with cargo should not have to.
    def closure(start: str) -> list[str]:
        seen: set[str] = set()
        stack = list(all_edges.get(start, []))
        while stack:
            current = stack.pop()
            if current in seen:
                continue
            seen.add(current)
            stack.extend(all_edges.get(current, []))
        return sorted(name for name in seen if name.startswith("ambition"))

    return {
        **base,
        "available": True,
        "counts": {
            "members": len(members),
            "packages_in_resolve": len(name_by_id),
        },
        "members": members,
        "edges": edges,
        "features": features,
        "ambition_closure": {name: closure(name) for name in members},
    }



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

def write_generation_stamp(indexed_files: int | None = None) -> None:
    """Machine-local generation stamp, next to the gitignored indexes.

    The tracked manifest must stay byte-stable across regens, so volatile
    provenance lives here instead: `agent_query.py` reads it to render
    "Generated at" and to warn when the local index is behind HEAD.

    ⭐ **two of these fields exist for the reader who has no git**, which is the
    environment the source archive is built for. A commit id is only a freshness
    signal to somebody who can ask what HEAD is; in an archive published without
    history, the commit comparison silently returns "no answer" and silence
    reads as "fresh". `indexed_files` gives that reader a check they can
    actually run — the index names its files, so counting how many are missing
    from the tree in front of them is a real answer with no toolchain at all.

    `worktree_dirty` records that the index was built over uncommitted edits, so
    a commit id that looks exact is not mistaken for one.
    """
    import datetime

    status = _git(["status", "--porcelain"])
    stamp = {
        "generated_from_commit": _git(["rev-parse", "--short=12", "HEAD"]) or "unknown",
        "source_commit_time": _git(["show", "-s", "--format=%cI", "HEAD"]) or "unknown",
        "generated_at": datetime.datetime.now(datetime.timezone.utc).isoformat(
            timespec="seconds"
        ),
        # None means "git could not be asked", which is not the same as clean.
        "worktree_dirty": None if status is None else bool(status),
    }
    if indexed_files is not None:
        stamp["indexed_files"] = indexed_files
    (INDEX_DIR / "generation_stamp.json").write_text(
        json.dumps(stamp, indent=2) + "\n", encoding="utf-8"
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
    write_generation_stamp(indexed_files=len(files))

    # The resolved graph before the catalog, so one run leaves a coherent set.
    # `build_catalog` names this file in its prune allowlist; if that ever stops
    # being true this file is deleted on every run and nothing says so.
    resolved = build_resolved_dependency_graph()
    write_json(INDEX_DIR / "crates" / "graph-resolved.json", resolved)
    if not resolved.get("available"):
        print(f"note: resolved dependency graph unavailable — {resolved.get('reason')}")

    # Build the progressive-disclosure catalog from the fresh flat indexes.
    # The archive builder reruns this after ECS inventory so archive packets
    # include current Bevy counts and per-crate inventory links.
    from agent_query import build_catalog

    build_catalog(quiet=True)
    print("generated .agent indexes, catalog, crate packets, and manifest")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
