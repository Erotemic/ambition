#!/usr/bin/env python3
"""
Build an agent-ready source archive for the Ambition repository.

This script is intentionally repo-local and opinionated. Configuration lives in
``CONFIG`` near the top of the file, so changing archive policy is a normal repo
change instead of an invocation-time mystery.

The archive layout is deterministic:

    <prefix>/
        .git/                         # optional shallow/full history
        .agent/README.md              # generated navigation front door
        .agent/index/...              # flat indexes + per-crate drill-down packets
        .agent/index/crates/graph-*.dot   # optional, --dependency-graph (graphviz)
        .agent/index/crates/graph-*.svg   # optional, --dependency-graph (graphviz)
        .agent/ecs_inventory/...       # per-crate ECS inventory shards
        .agent/reports/...             # optional full diagnostic reports
        .agent/dirstats-*.txt         # freshly generated from staged contents
        .agent/source_archive_manifest.yaml
        .agent/manifest.yaml              # tracked, byte-stable navigation policy
        .agent/live-disk-inventory-*.txt  # metadata only, from live checkout
        SOURCE_ARCHIVE_MANIFEST.txt
        GIT_WELL_ARCHIVE_INFO.txt
        ... tracked source files ...

Git-well stages committed source, history, branches, and submodules. Ambition
then enriches that staged tree through git-well's programmatic prepare hook and
validates the final payload through its validation hook before git-well writes
the requested archive format. The optional live-disk inventory records where
ignored/untracked assets exist in the user's checkout, but it does not copy
those files into the archive.
"""

from __future__ import annotations

import argparse
import fnmatch
import json
import os
import shutil
import subprocess
import sys
from collections import Counter
from dataclasses import dataclass, field
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Iterable, Sequence

try:  # rich is optional; the final artifact link degrades to plain print without it.
    from rich import print as rich_print
    from rich.markup import escape as rich_escape
except ImportError:
    rich_print = None

    def rich_escape(text: str) -> str:
        return text


CONFIG = {
    # Archive identity. ``prefix_template`` may use {repo}, {timestamp}, and
    # {short_sha}. The timestamp is UTC and filename-safe.
    'repo_name': 'ambition',
    'prefix_template': '{repo}-agent-source-{timestamp}-{short_sha}',

    # Git history policy. Use None or 'full' for full history, a positive int
    # for a shallow clone, or 0 for source-only git-archive mode.
    'include_git_history': True,
    'super_depth': 300,
    'submodule_depths': {
        # Keep this explicit so it is easy to tune as submodules grow.
        'tools/ambition_sfx_renderer': 50,
        '*': 25,
    },

    # Dirty worktree policy. The archive is built from committed source. If this
    # is True, local modifications cause an error instead of only a warning.
    'fail_if_dirty': False,

    # Agent index policy. This command is run inside the staged archive root,
    # after source/submodules are materialized and before archiving.
    'run_agent_index': True,
    'agent_index_command': [sys.executable, 'scripts/generate_agent_index.py'],
    'required_agent_index_paths': [
        '.agent/manifest.yaml',
        '.agent/index/generation_stamp.json',
        '.agent/index/entry_points.json',
        '.agent/index/planning_index.json',
        '.agent/index/file_summaries.json',
        '.agent/index/symbol_index.json',
        '.agent/index/test_map.json',
    ],

    # Progressive-disclosure navigation. This runs after the raw indexes and
    # ECS inventory so it can merge them into a small catalog, a generated
    # README, and per-crate packets without duplicating hand-maintained facts.
    'run_agent_navigation': True,
    'agent_navigation_command': [
        sys.executable,
        'scripts/agent_query.py',
        'build-catalog',
        '--quiet',
    ],
    'required_agent_navigation_paths': [
        '.agent/README.md',
        '.agent/index/catalog.json',
        '.agent/index/crates/index.json',
        # Both dependency graphs, required so a silently-missing one fails the
        # build instead of reaching a consumer as an absence they cannot
        # distinguish from "this workspace has no dependencies". The RESOLVED
        # graph is always written even when cargo is unavailable — it then says
        # so in `available`/`reason`, which is the fact worth shipping.
        '.agent/index/crates/graph-declared.json',
        '.agent/index/crates/graph-resolved.json',
    ],

    # Drawn dependency graphs. OPT-IN (`--dependency-graph`), because this is
    # the only payload needing a tool that is neither Rust nor Python: graphviz's
    # `dot`. The JSON graphs above already carry every edge, so the picture is a
    # convenience for a human reader, not a fact only it holds.
    #
    # the RESOLVED graph is drawn only when it was resolved. When cargo was
    # unavailable the JSON says so in `available`/`reason` and ships anyway, so
    # the missing .dot beside it is explained by a file in the same directory —
    # which is the only reason skipping it here is not a silent absence.
    'run_dependency_graph': False,
    'dependency_graph_formats': ['dot', 'svg'],
    'dependency_graph_command': [
        sys.executable,
        'scripts/agent_query.py',
        'graph',
    ],
    'required_dependency_graph_paths': [
        '.agent/index/crates/graph-declared.dot',
        '.agent/index/crates/graph-declared.svg',
    ],

    # Agent discovery reports. The ECS inventory is intentionally a neutral
    # inventory, not a migration planner. Full reports are opt-in because they
    # can be slower and require extra local cargo plugins.
    'run_ecs_inventory': True,
    'ecs_inventory_command': [
        # This script carries PEP 723 inline dependency metadata for the native
        # tree-sitter bindings. Always honor that environment contract instead
        # of inheriting whichever tree-sitter ABI happens to be installed in
        # the Python environment that launched the archiver.
        'uv',
        'run',
        '--script',
        'scripts/ecs_inventory.py',
        '--workspace',
        '--out-dir',
        '.agent/ecs_inventory',
    ],
    'required_ecs_inventory_paths': [
        '.agent/ecs_inventory/project.md',
        '.agent/ecs_inventory/project.json',
    ],
    'cargo_check_command': [
        'cargo',
        'check',
        '--workspace',
        '--lib',
        '--message-format=json',
    ],
    'cargo_check_warnings_output': '.agent/reports/cargo-check-warnings.md',
    'cargo_modules_reports': [
        {
            'output': '.agent/reports/module-tree-ambition_platformer2d_actor_monolith.md',
            'command': ['cargo', 'modules', 'structure', '--package', 'ambition_platformer2d_actor_monolith', '--lib'],
        },
        {
            'output': '.agent/reports/module-dependencies-ambition_platformer2d_actor_monolith.md',
            'command': ['cargo', 'modules', 'dependencies', '--package', 'ambition_platformer2d_actor_monolith', '--lib'],
        },
    ],

    # Dirstats prefer xdev's richer walker so text-like files get total_lines
    # and Rust/Python files get language-aware line breakdowns. If xdev is not
    # importable or fails, the built-in lightweight walker below is used as a
    # fallback. These describe the staged archive contents, not ignored build
    # products in the user's live checkout.
    'run_dirstats': True,
    'dirstats': [
        {
            'output': '.agent/dirstats-crates-summary.txt',
            'path': 'crates',
            'display_depth': 4,
        },
        {
            'output': '.agent/dirstats-crates-full.txt',
            'path': 'crates',
            'display_depth': None,
        },
        {
            'output': '.agent/dirstats-game-summary.txt',
            'path': 'game',
            'display_depth': 4,
        },
        {
            'output': '.agent/dirstats-game-full.txt',
            'path': 'game',
            'display_depth': None,
        },
        {
            'output': '.agent/dirstats-repo-summary.txt',
            'path': '.',
            'display_depth': 4,
        },
        {
            'output': '.agent/dirstats-repo-full.txt',
            'path': '.',
            'display_depth': None,
        },
    ],
    'dirstats_exclude_dnames': [
        '.git',
        '.agent',
        '.worktrees',
        'debug_traces',
        'target',
        '__pycache__',
    ],
    'dirstats_exclude_fnames': [
        '*.pyc',
        '*.pyo',
    ],
    'dirstats_max_lines': 20000,

    # Metadata-only inventory of the live checkout. This is intentionally not
    # used as source input for the archive. It gives agents hints about where
    # ignored/generated assets live on your disk without uploading the assets.
    'run_live_disk_inventory': True,
    'live_disk_inventory': [
        {
            'output': '.agent/live-disk-inventory-summary.txt',
            'path': '.',
            'display_depth': 4,
        },
        {
            'output': '.agent/live-disk-inventory-full.txt',
            'path': '.',
            'display_depth': None,
        },
    ],
    'live_disk_inventory_exclude_dnames': [
        '.git',
        '.agent',
        '.worktrees',
        'debug_traces',
        'target',
        '__pycache__',
        'node_modules',
        'dist',
        'build',
    ],
    'live_disk_inventory_exclude_fnames': [
        '*.pyc',
        '*.pyo',
        '*.tmp',
    ],
    'live_git_status_output': '.agent/live-git-status-ignored.txt',
    'live_git_status_max_lines': 5000,

    # Guardrails for content that should not be uploaded to agents. In fail
    # mode, any match in HEAD or in the included HEAD history aborts the build.
    # Use --allow-forbidden for one-off local debugging only.
    'forbidden_path_policy': 'fail',  # one of: fail, warn, ignore
    'forbidden_path_globs': [
        'docs/planning/plugin_refactor/snapshots/**',
    ],
    'scan_forbidden_history': True,

    # Final archive validation. Paths are relative to the staged archive root.
    # The base list is always required; the step-specific lists below are only
    # enforced when their step actually runs, so per-step skips stay valid.
    'required_archive_paths': [
        '.agent/source_archive_manifest.yaml',
        'SOURCE_ARCHIVE_MANIFEST.txt',
    ],
    'required_dirstats_paths': [
        '.agent/dirstats-repo-summary.txt',
    ],
    'required_live_disk_inventory_paths': [
        '.agent/live-disk-inventory-summary.txt',
        '.agent/live-git-status-ignored.txt',
    ],
}


@dataclass
class DirNode:
    path: Path
    rel: Path
    is_dir: bool
    size: int = 0
    files: int = 0
    dirs: int = 0
    direct_files: int = 0
    direct_dirs: int = 0
    ext_files: Counter[str] = field(default_factory=Counter)
    ext_sizes: Counter[str] = field(default_factory=Counter)
    children: list['DirNode'] = field(default_factory=list)
    error: str | None = None


class CommandError(RuntimeError):
    pass


class Log:
    def __init__(self, verbose: int = 1) -> None:
        self.verbose = verbose

    def __call__(self, message: str) -> None:
        if self.verbose:
            print(message, flush=True)


def run(
    args: Sequence[str | os.PathLike[str]],
    *,
    cwd: Path | None = None,
    check: bool = True,
    capture: bool = False,
    env: dict[str, str] | None = None,
) -> subprocess.CompletedProcess[str]:
    cmd = [os.fspath(a) for a in args]
    kwargs = {
        'cwd': os.fspath(cwd) if cwd is not None else None,
        'text': True,
        'env': env,
    }
    if capture:
        kwargs.update({'stdout': subprocess.PIPE, 'stderr': subprocess.PIPE})
    proc = subprocess.run(cmd, **kwargs)  # type: ignore[arg-type]
    if check and proc.returncode != 0:
        rendered = ' '.join(shell_quote(p) for p in cmd)
        detail = ''
        if capture:
            detail = f'\nstdout:\n{proc.stdout}\nstderr:\n{proc.stderr}'
        raise CommandError(f'command failed with code {proc.returncode}: {rendered}{detail}')
    return proc


def shell_quote(text: str) -> str:
    import shlex

    return shlex.quote(text)


def git(root: Path, *args: str, capture: bool = True, check: bool = True) -> str:
    proc = run(['git', '-C', root, *args], check=check, capture=capture)
    return proc.stdout.rstrip('\n') if proc.stdout is not None else ''


def coerce_repo_root(path: Path) -> Path:
    proc = run(
        ['git', '-C', path, 'rev-parse', '--show-toplevel'],
        capture=True,
        check=True,
    )
    return Path(proc.stdout.strip()).resolve()



def file_url(path: Path) -> str:
    # Path.as_uri handles spaces and other URL quoting for absolute paths.
    return path.resolve().as_uri()


def path_link(path: Path, label: str | None = None) -> str:
    """Rich clickable ``file://`` link markup for a path (repo convention)."""
    text = label if label is not None else os.fspath(path)
    return f'[link={file_url(path)}]{rich_escape(text)}[/link]'


def print_output_location(output: Path) -> None:
    """Announce the written archive + its directory as clickable links at the very
    end of stdout, so the artifact is one click away. Prints unconditionally (this
    is the result, not chatter) and degrades to plain paths when rich is absent."""
    directory = output.parent
    if rich_print is not None:
        rich_print(f'\n[bold green]✓ archive written[/bold green]  {path_link(output)}')
        rich_print(f'  [dim]open folder →[/dim]  {path_link(directory)}')
    else:
        print(f'\narchive written: {output}')
        print(f'open folder: {directory}')



def should_exclude(name: str, patterns: Iterable[str]) -> bool:
    return any(fnmatch.fnmatch(name, pattern) for pattern in patterns)


def scan_dir_tree(root: Path, *, base: Path, exclude_dnames: list[str], exclude_fnames: list[str]) -> DirNode:
    rel = root.relative_to(base) if root != base else Path('.')
    node = DirNode(path=root, rel=rel, is_dir=True)
    try:
        entries = list(os.scandir(root))
    except OSError as ex:
        node.error = f'{type(ex).__name__}: {ex}'
        return node

    dir_entries = []
    file_entries = []
    for entry in entries:
        try:
            if entry.is_dir(follow_symlinks=False):
                if not should_exclude(entry.name, exclude_dnames):
                    dir_entries.append(entry)
            else:
                if not should_exclude(entry.name, exclude_fnames):
                    file_entries.append(entry)
        except OSError:
            file_entries.append(entry)

    node.direct_dirs = len(dir_entries)
    node.direct_files = len(file_entries)
    node.dirs = len(dir_entries)

    for entry in sorted(file_entries, key=lambda e: e.name):
        fpath = Path(entry.path)
        try:
            st = entry.stat(follow_symlinks=False)
            size = st.st_size
        except OSError:
            size = 0
        ext = file_ext_label(fpath)
        node.files += 1
        node.size += size
        node.ext_files[ext] += 1
        node.ext_sizes[ext] += size

    children = []
    for entry in sorted(dir_entries, key=lambda e: e.name):
        child = scan_dir_tree(Path(entry.path), base=base, exclude_dnames=exclude_dnames, exclude_fnames=exclude_fnames)
        children.append(child)
        node.files += child.files
        node.dirs += child.dirs
        node.size += child.size
        node.ext_files.update(child.ext_files)
        node.ext_sizes.update(child.ext_sizes)

    children.sort(key=lambda c: (c.size, c.files, c.path.name), reverse=True)
    node.children = children
    return node


def file_ext_label(path: Path) -> str:
    if path.name in {'Cargo.lock', 'Makefile', 'Dockerfile'}:
        return path.name
    ext = path.suffix.lower()
    return ext if ext else '[no_ext]'


def byte_str(num: int) -> str:
    value = float(num)
    for unit in ['B', 'KB', 'MB', 'GB', 'TB']:
        if abs(value) < 1024.0 or unit == 'TB':
            if unit == 'B':
                return f'{int(value)} {unit}'
            return f'{value:.2f} {unit}'
        value /= 1024.0
    raise AssertionError('unreachable')


def compact_ext_summary(node: DirNode, limit: int = 5) -> str:
    if not node.ext_files:
        return ''
    parts = []
    for ext, count in node.ext_files.most_common(limit):
        parts.append(f'{ext}:{count}')
    extra = len(node.ext_files) - len(parts)
    if extra > 0:
        parts.append(f'+{extra} ext')
    return ', '.join(parts)


def render_tree(node: DirNode, *, max_depth: int | None, max_lines: int) -> list[str]:
    lines: list[str] = []
    truncated = False

    def rec(cur: DirNode, prefix: str, depth: int, is_last: bool) -> None:
        nonlocal truncated
        if len(lines) >= max_lines:
            truncated = True
            return
        branch = '' if depth == 0 else ('`-- ' if is_last else '|-- ')
        name = '.' if depth == 0 else cur.path.name + '/'
        summary = f'{cur.files} files, {cur.dirs} dirs, {byte_str(cur.size)}'
        ext_summary = compact_ext_summary(cur)
        suffix = f' [{ext_summary}]' if ext_summary else ''
        if cur.error:
            suffix += f' ERROR={cur.error}'
        lines.append(f'{prefix}{branch}{name}: {summary}{suffix}')
        if max_depth is not None and depth >= max_depth:
            return
        child_prefix = prefix if depth == 0 else prefix + ('    ' if is_last else '|   ')
        for idx, child in enumerate(cur.children):
            rec(child, child_prefix, depth + 1, idx == len(cur.children) - 1)

    rec(node, '', 0, True)
    if truncated:
        lines.append(f'... truncated after {max_lines} lines ...')
    return lines



def write_xdev_dirstats_report(archive_root: Path, spec: dict[str, object], generated_at: str) -> None:
    """Write dirstats with the xdev Python API, raising on failure."""
    output = archive_root / str(spec['output'])
    output.parent.mkdir(parents=True, exist_ok=True)

    try:
        from xdev.directory_walker import dirstats_report_text
    except Exception as ex:
        raise CommandError(f'xdev dirstats API is unavailable: {ex}') from ex

    rel_input = Path(str(spec['path']))
    display_depth = spec.get('display_depth')
    display_depth_int = None if display_depth is None else int(display_depth)
    max_rows = int(CONFIG['dirstats_max_lines'])

    try:
        report_text = dirstats_report_text(
            archive_root / rel_input,
            exclude_dnames=list(CONFIG['dirstats_exclude_dnames']),
            exclude_fnames=list(CONFIG['dirstats_exclude_fnames']),
            max_display_depth=display_depth_int,
            max_rows=max_rows,
            parse_content=True,
            python=True,
            rust=True,
            include_files=True,
            show_progress=False,
        )
    except Exception as ex:
        raise CommandError(f'xdev dirstats API failed: {ex}') from ex

    lines = [
        'Directory stats (xdev API)',
        '===========================',
        '',
        f'Generated at UTC: {generated_at}',
        f'Archive root: {archive_root.name}',
        f'Stats root: {rel_input.as_posix()}',
        f'Display depth: {display_depth_int if display_depth_int is not None else "full"}',
        f'Max rows: {max_rows}',
        f'Excluded directories: {", ".join(CONFIG["dirstats_exclude_dnames"])}',
        f'Excluded files: {", ".join(CONFIG["dirstats_exclude_fnames"])}',
        'API: xdev.directory_walker.dirstats_report_text(parse_content=True, python=True, rust=True)',
        '',
        report_text.rstrip(),
    ]
    output.write_text('\n'.join(lines).rstrip() + '\n', encoding='utf-8')


def write_dirstats_report(archive_root: Path, spec: dict[str, object], generated_at: str, log: Log) -> None:
    """Write a staged dirstats report, preferring xdev with a local fallback."""
    try:
        write_xdev_dirstats_report(archive_root, spec, generated_at)
    except Exception as ex:
        output = archive_root / str(spec['output'])
        log(
            '[archive-agent-source] warning: xdev dirstats failed for '
            f'{output.relative_to(archive_root)}; falling back to built-in walker: {ex}'
        )
        write_builtin_dirstats_report(archive_root, spec, generated_at)


def write_builtin_dirstats_report(archive_root: Path, spec: dict[str, object], generated_at: str) -> None:
    rel_input = Path(str(spec['path']))
    root = (archive_root / rel_input).resolve()
    output = archive_root / str(spec['output'])
    output.parent.mkdir(parents=True, exist_ok=True)

    exclude_dnames = list(CONFIG['dirstats_exclude_dnames'])
    exclude_fnames = list(CONFIG['dirstats_exclude_fnames'])
    max_lines = int(CONFIG['dirstats_max_lines'])
    display_depth = spec.get('display_depth')
    display_depth_int = None if display_depth is None else int(display_depth)

    lines: list[str] = []
    lines.append('Directory stats')
    lines.append('===============')
    lines.append('')
    lines.append(f'Generated at UTC: {generated_at}')
    lines.append(f'Archive root: {archive_root.name}')
    lines.append(f'Stats root: {rel_input.as_posix()}')
    lines.append(f'Display depth: {display_depth_int if display_depth_int is not None else "full"}')
    lines.append(f'Excluded directories: {", ".join(exclude_dnames)}')
    lines.append(f'Excluded files: {", ".join(exclude_fnames)}')
    lines.append('')

    if not root.exists():
        lines.append(f'MISSING: {rel_input.as_posix()}')
        output.write_text('\n'.join(lines).rstrip() + '\n')
        return

    node = scan_dir_tree(root, base=root, exclude_dnames=exclude_dnames, exclude_fnames=exclude_fnames)
    lines.append(f'Total files: {node.files}')
    lines.append(f'Total directories: {node.dirs}')
    lines.append(f'Total size: {byte_str(node.size)}')
    lines.append('')
    lines.append('Top extensions by size:')
    if node.ext_sizes:
        for ext, size in node.ext_sizes.most_common(30):
            lines.append(f'  {ext:>12} {byte_str(size):>12} {node.ext_files[ext]:>8} files')
    else:
        lines.append('  (none)')
    lines.append('')
    lines.append('Tree:')
    lines.extend(render_tree(node, max_depth=display_depth_int, max_lines=max_lines))
    output.write_text('\n'.join(lines).rstrip() + '\n')



def write_live_disk_inventory_report(
    repo_root: Path,
    archive_root: Path,
    spec: dict[str, object],
    generated_at: str,
    source_display_path: str,
) -> None:
    """Write metadata-only dirstats for the user's live checkout."""
    rel_input = Path(str(spec['path']))
    root = (repo_root / rel_input).resolve()
    output = archive_root / str(spec['output'])
    output.parent.mkdir(parents=True, exist_ok=True)

    exclude_dnames = list(CONFIG['live_disk_inventory_exclude_dnames'])
    exclude_fnames = list(CONFIG['live_disk_inventory_exclude_fnames'])
    max_lines = int(CONFIG['dirstats_max_lines'])
    display_depth = spec.get('display_depth')
    display_depth_int = None if display_depth is None else int(display_depth)

    lines: list[str] = []
    lines.append('Live disk inventory')
    lines.append('===================')
    lines.append('')
    lines.append(f'Generated at UTC: {generated_at}')
    lines.append(f'Live repo root: {source_display_path}')
    lines.append(f'Inventory root: {rel_input.as_posix()}')
    lines.append(f'Display depth: {display_depth_int if display_depth_int is not None else "full"}')
    lines.append('')
    lines.append('Policy: metadata only. Files described here are not copied into the archive.')
    lines.append(f'Excluded directories: {", ".join(exclude_dnames)}')
    lines.append(f'Excluded files: {", ".join(exclude_fnames)}')
    lines.append('')

    if not root.exists():
        lines.append(f'MISSING: {rel_input.as_posix()}')
        output.write_text('\n'.join(lines).rstrip() + '\n')
        return

    node = scan_dir_tree(root, base=root, exclude_dnames=exclude_dnames, exclude_fnames=exclude_fnames)
    lines.append(f'Total files visible to inventory: {node.files}')
    lines.append(f'Total directories visible to inventory: {node.dirs}')
    lines.append(f'Total visible byte size: {byte_str(node.size)}')
    lines.append('')
    lines.append('Top extensions by visible size:')
    if node.ext_sizes:
        for ext, size in node.ext_sizes.most_common(50):
            lines.append(f'  {ext:>12} {byte_str(size):>12} {node.ext_files[ext]:>8} files')
    else:
        lines.append('  (none)')
    lines.append('')
    lines.append('Tree:')
    lines.extend(render_tree(node, max_depth=display_depth_int, max_lines=max_lines))
    output.write_text('\n'.join(lines).rstrip() + '\n')


def write_live_git_status_report(
    repo_root: Path,
    archive_root: Path,
    generated_at: str,
    source_display_path: str,
) -> None:
    """Record ignored/untracked path hints without copying those files."""
    output = archive_root / str(CONFIG['live_git_status_output'])
    output.parent.mkdir(parents=True, exist_ok=True)
    max_lines = int(CONFIG['live_git_status_max_lines'])
    proc = run(
        ['git', '-C', repo_root, 'status', '--short', '--ignored'],
        capture=True,
        check=False,
    )
    status_lines = proc.stdout.splitlines() if proc.stdout else []
    truncated = len(status_lines) > max_lines
    shown_lines = status_lines[:max_lines]
    lines = [
        'Live git status with ignored paths',
        '==================================',
        '',
        f'Generated at UTC: {generated_at}',
        f'Live repo root: {source_display_path}',
        '',
        'Policy: metadata only. These paths are not copied into the archive.',
        'Legend follows git status --short --ignored output, e.g. ?? untracked and !! ignored.',
        '',
    ]
    if proc.returncode != 0:
        lines.append(f'git status failed with code {proc.returncode}')
        if proc.stderr:
            lines.append(proc.stderr.rstrip())
    elif shown_lines:
        lines.extend(shown_lines)
        if truncated:
            lines.append(f'... truncated after {max_lines} of {len(status_lines)} lines ...')
    else:
        lines.append('(no untracked or ignored paths reported)')
    output.write_text('\n'.join(lines).rstrip() + '\n')


def run_live_disk_inventory(
    repo_root: Path,
    archive_root: Path,
    generated_at: str,
    source_display_path: str,
    log: Log,
) -> None:
    if not CONFIG['run_live_disk_inventory']:
        log('[archive-agent-source] skipping live disk inventory')
        return
    for spec in CONFIG['live_disk_inventory']:
        log(f'[archive-agent-source] writing {spec["output"]}')
        write_live_disk_inventory_report(
            repo_root, archive_root, spec, generated_at, source_display_path
        )
    log(f'[archive-agent-source] writing {CONFIG["live_git_status_output"]}')
    write_live_git_status_report(
        repo_root, archive_root, generated_at, source_display_path
    )


def path_matches_any(relpath: str, patterns: Iterable[str]) -> bool:
    return any(fnmatch.fnmatch(relpath, pattern) for pattern in patterns)


def _staged_history_refs(context: Any) -> list[str]:
    if not context.all_branches or context.branch_refs is None:
        return ['HEAD']
    refs = [f'refs/heads/{name}' for name in context.branch_refs.local_branches]
    refs.extend(
        f'refs/remotes/{name}'
        for name in context.branch_refs.remote_tracking_branches
    )
    return refs or ['HEAD']


def find_forbidden_paths(context: Any) -> dict[str, list[str]]:
    """Inspect the exact staged source and history that will be archived."""
    patterns = [str(p) for p in CONFIG['forbidden_path_globs']]
    found: dict[str, list[str]] = {'head': [], 'history': []}
    if not patterns:
        return found

    if context.include_git_history:
        head_paths = git(
            context.archive_root, 'ls-tree', '-r', '--name-only', 'HEAD'
        ).splitlines()
    else:
        head_paths = [
            path.relative_to(context.archive_root).as_posix()
            for path in context.archive_root.rglob('*')
            if path.is_file() or path.is_symlink()
        ]
    found['head'] = sorted(
        {path for path in head_paths if path_matches_any(path, patterns)}
    )

    if CONFIG['scan_forbidden_history'] and context.include_git_history:
        args = ['log', '--format=', '--name-only', *_staged_history_refs(context)]
        hist_paths = git(context.archive_root, *args).splitlines()
        found['history'] = sorted(
            {path for path in hist_paths if path_matches_any(path, patterns)}
        )

    return found


def enforce_forbidden_path_policy(
    context: Any,
    allow_forbidden: bool,
    log: Log,
) -> None:
    if allow_forbidden:
        log('[archive-agent-source] forbidden path guard disabled by --allow-forbidden')
        return
    policy = str(CONFIG['forbidden_path_policy'])
    if policy == 'ignore':
        return
    if policy not in {'fail', 'warn'}:
        raise ValueError(
            'CONFIG["forbidden_path_policy"] must be one of: fail, warn, ignore'
        )
    found = find_forbidden_paths(context)
    if not found['head'] and not found['history']:
        return
    parts = ['forbidden paths matched archive guard:']
    if found['head']:
        parts.append('  present in staged HEAD:')
        parts.extend(f'    - {p}' for p in found['head'][:50])
        if len(found['head']) > 50:
            parts.append(f'    ... {len(found["head"]) - 50} more ...')
    if found['history']:
        scope = 'all archived branch history' if context.all_branches else 'archived HEAD history'
        parts.append(f'  present in {scope}:')
        parts.extend(f'    - {p}' for p in found['history'][:50])
        if len(found['history']) > 50:
            parts.append(f'    ... {len(found["history"]) - 50} more ...')
    message = '\n'.join(parts)
    if policy == 'fail':
        raise RuntimeError(
            message
            + '\nSet CONFIG["forbidden_path_policy"] to "warn" or run '
            '--allow-forbidden only if this is intentional.'
        )
    log(
        '[archive-agent-source] warning: '
        + message.replace('\n', '\n[archive-agent-source] warning: ')
    )

def yaml_scalar(value: object) -> str:
    text = str(value)
    if text == '' or any(ch in text for ch in ':#{}[]&,*?|<>!=%@`') or text.strip() != text:
        escaped = text.replace('\\', '\\\\').replace('"', '\\"')
        return f'"{escaped}"'
    return text


def refresh_generation_stamp(
    context: Any,
    generated_at: str,
    log: Log,
) -> None:
    """Record exact source provenance without modifying tracked manifest data."""
    stamp_path = context.archive_root / '.agent' / 'index' / 'generation_stamp.json'
    stamp_path.parent.mkdir(parents=True, exist_ok=True)
    try:
        payload = json.loads(stamp_path.read_text(encoding='utf-8'))
    except (FileNotFoundError, json.JSONDecodeError):
        payload = {}
    payload.update(
        {
            'generated_from_commit': context.short_sha,
            'source_commit_time': git(
                context.repo_root,
                'show',
                '-s',
                '--format=%cI',
                context.head_sha,
            ),
            'generated_at': generated_at,
        }
    )
    stamp_path.write_text(json.dumps(payload, indent=2) + '\n', encoding='utf-8')
    log(f'[archive-agent-source] refreshed {stamp_path.relative_to(context.archive_root)}')


def validate_generation_stamp(context: Any) -> None:
    stamp_path = context.archive_root / '.agent' / 'index' / 'generation_stamp.json'
    if not stamp_path.exists():
        raise RuntimeError('missing required .agent/index/generation_stamp.json')
    payload = json.loads(stamp_path.read_text(encoding='utf-8'))
    if payload.get('generated_from_commit') != context.short_sha:
        raise RuntimeError(
            'generation stamp does not match staged source: '
            f'expected {context.short_sha!r}, got '
            f'{payload.get("generated_from_commit")!r}'
        )


def _status_path(line: str) -> str:
    path = line[3:] if len(line) >= 4 else line
    if ' -> ' in path:
        path = path.split(' -> ', 1)[1]
    return path.strip()


def validate_staged_checkout_clean(context: Any) -> None:
    """Reject tracked mutations while allowing intentionally omitted submodules."""
    if not context.include_git_history:
        return
    status_lines = git(
        context.archive_root,
        'status',
        '--short',
        '--untracked-files=all',
    ).splitlines()
    omitted = {
        decision.info.path.rstrip('/')
        for decision in context.submodule_decisions
        if decision.omitted
    }
    unexpected = []
    for line in status_lines:
        path = _status_path(line)
        if any(path == item or path.startswith(item + '/') for item in omitted):
            continue
        unexpected.append(line)
    if unexpected:
        details = '\n'.join(f'  {line}' for line in unexpected)
        raise RuntimeError(
            'archive enrichment modified tracked source files:\n'
            f'{details}\n'
            'Generated payloads must remain untracked/ignored; keep volatile '
            'provenance in .agent/index/generation_stamp.json.'
        )

def write_manifests(
    *,
    context: Any,
    generated_at: str,
    dirty_status: str,
    full_reports: bool,
) -> None:
    archive_root = context.archive_root
    agent_dir = archive_root / '.agent'
    agent_dir.mkdir(exist_ok=True)
    manifest_yaml = agent_dir / 'source_archive_manifest.yaml'
    manifest_txt = archive_root / 'SOURCE_ARCHIVE_MANIFEST.txt'
    source_display_path = context.source_display_path()
    run_agent_index = bool(CONFIG['run_agent_index'])
    run_ecs_inventory = bool(CONFIG['run_ecs_inventory'])
    run_agent_navigation = bool(CONFIG['run_agent_navigation'])
    run_dirstats = bool(CONFIG['run_dirstats'])
    run_live_disk_inventory = bool(CONFIG['run_live_disk_inventory'])

    yaml_lines = [
        'schema_version: 3',
        f'generated_at_utc: {yaml_scalar(generated_at)}',
        f'generator: {yaml_scalar("scripts/archive_agent_source.py")}',
        f'repo_name: {yaml_scalar(CONFIG["repo_name"])}',
        f'source_repo_root: {yaml_scalar(source_display_path)}',
        f'archive_prefix: {yaml_scalar(context.archive_root_name)}',
        f'archive_format: {yaml_scalar(context.archive_format)}',
        'git:',
        f'  include_history: {str(context.include_git_history).lower()}',
        f'  super_depth: {yaml_scalar("full" if context.depth is None else context.depth)}',
        f'  all_branches: {str(context.all_branches).lower()}',
        f'  head_sha: {yaml_scalar(context.head_sha)}',
        f'  short_sha: {yaml_scalar(context.short_sha)}',
        f'  dirty_worktree_at_build_time: {str(bool(dirty_status.strip())).lower()}',
    ]
    if context.branch_refs is not None:
        yaml_lines.extend(
            [
                f'  local_branch_count: {len(context.branch_refs.local_branches)}',
                '  local_branches:',
                *[
                    f'    - {yaml_scalar(name)}'
                    for name in context.branch_refs.local_branches
                ],
                f'  remote_tracking_branch_count: {len(context.branch_refs.remote_tracking_branches)}',
                '  remote_tracking_branches:',
                *[
                    f'    - {yaml_scalar(name)}'
                    for name in context.branch_refs.remote_tracking_branches
                ],
            ]
        )

    if context.submodule_decisions:
        yaml_lines.append('submodules:')
        for decision in context.submodule_decisions:
            info = decision.info
            yaml_lines.extend(
                [
                    f'  - path: {yaml_scalar(info.path)}',
                    f'    sha: {yaml_scalar(info.sha)}',
                    f'    status: {yaml_scalar(info.status)}',
                    f'    omitted: {str(decision.omitted).lower()}',
                    f'    mode: {yaml_scalar(decision.mode)}',
                    f'    depth: {yaml_scalar("full" if decision.depth is None else decision.depth)}',
                    f'    reason: {yaml_scalar(decision.reason)}',
                ]
            )
    else:
        yaml_lines.append('submodules: []')

    yaml_lines.extend(
        [
            'generated_payloads:',
            '  agent_manifest: .agent/manifest.yaml',
            '  source_archive_manifest: .agent/source_archive_manifest.yaml',
            '  git_well_manifest: GIT_WELL_ARCHIVE_INFO.txt',
            f'  agent_index_enabled: {str(run_agent_index).lower()}',
            f'  ecs_inventory_enabled: {str(run_ecs_inventory).lower()}',
            f'  agent_navigation_enabled: {str(run_agent_navigation).lower()}',
            f'  full_reports_enabled: {str(bool(full_reports)).lower()}',
            f'  dirstats_enabled: {str(run_dirstats).lower()}',
            f'  live_disk_inventory_enabled: {str(run_live_disk_inventory).lower()}',
        ]
    )
    if run_agent_index:
        yaml_lines.append('  generation_stamp: .agent/index/generation_stamp.json')
        yaml_lines.append('  agent_index_command:')
        for item in CONFIG['agent_index_command']:
            yaml_lines.append(f'    - {yaml_scalar(item)}')
    if run_ecs_inventory:
        yaml_lines.append('  ecs_inventory_command:')
        for item in CONFIG['ecs_inventory_command']:
            yaml_lines.append(f'    - {yaml_scalar(item)}')
        yaml_lines.append(
            f'  ecs_inventory_project: '
            f'{yaml_scalar(".agent/ecs_inventory/project.md")}'
        )
    if run_agent_navigation:
        yaml_lines.append('  agent_navigation_command:')
        for item in CONFIG['agent_navigation_command']:
            yaml_lines.append(f'    - {yaml_scalar(item)}')
        yaml_lines.append(f'  agent_readme: {yaml_scalar(".agent/README.md")}')
        yaml_lines.append(
            f'  agent_catalog: {yaml_scalar(".agent/index/catalog.json")}'
        )
        yaml_lines.append(
            f'  agent_crate_index: '
            f'{yaml_scalar(".agent/index/crates/index.json")}'
        )
    if full_reports:
        yaml_lines.append('  full_report_outputs:')
        yaml_lines.append(
            f'    cargo_check_warnings: '
            f'{yaml_scalar(CONFIG["cargo_check_warnings_output"])}'
        )
        yaml_lines.append('    cargo_modules:')
        for spec in CONFIG['cargo_modules_reports']:
            yaml_lines.append(f'      - {yaml_scalar(spec["output"])}')
    if run_dirstats:
        yaml_lines.append('  dirstats:')
        for spec in CONFIG['dirstats']:
            yaml_lines.extend(
                [
                    f'    - output: {yaml_scalar(spec["output"])}',
                    f'      path: {yaml_scalar(spec["path"])}',
                    f'      display_depth: {yaml_scalar(spec["display_depth"] if spec["display_depth"] is not None else "full")}',
                ]
            )
    if run_live_disk_inventory:
        yaml_lines.append('  live_disk_inventory:')
        for spec in CONFIG['live_disk_inventory']:
            yaml_lines.extend(
                [
                    f'    - output: {yaml_scalar(spec["output"])}',
                    f'      path: {yaml_scalar(spec["path"])}',
                    f'      display_depth: {yaml_scalar(spec["display_depth"] if spec["display_depth"] is not None else "full")}',
                ]
            )
        yaml_lines.append(
            f'  live_git_status_output: '
            f'{yaml_scalar(CONFIG["live_git_status_output"])}'
        )
    if dirty_status.strip():
        yaml_lines.append('dirty_status: |')
        for line in dirty_status.splitlines():
            yaml_lines.append(f'  {line}')
    else:
        yaml_lines.append('dirty_status: ""')
    manifest_yaml.write_text('\n'.join(yaml_lines).rstrip() + '\n')

    txt = [
        'Agent source archive manifest',
        '=============================',
        '',
        f'Generated at UTC: {generated_at}',
        f'Repository: {CONFIG["repo_name"]}',
        f'Source repository root: {source_display_path}',
        f'Archive prefix: {context.archive_root_name}',
        f'Archive format: {context.archive_format}',
        f'Superproject HEAD: {context.head_sha}',
        f'Superproject short HEAD: {context.short_sha}',
        f'Git history included: {"yes" if context.include_git_history else "no"}',
        f'Superproject depth: {"full" if context.depth is None else context.depth}',
        f'All locally cached branches included: {"yes" if context.all_branches else "no"}',
        '',
        'Policy:',
        '- Git-well stages committed source, history, branch refs, and recursive submodules.',
        '- Ambition enriches the staged checkout through a programmatic prepare hook.',
        '- The tracked .agent/manifest.yaml remains byte-stable; volatile provenance lives in generation_stamp.json when the agent index is generated.',
        '- Local untracked files, ignored build products, and dirty worktree changes are excluded from source contents.',
        '',
        'Payload generation:',
        f'- Agent index: {"generated" if run_agent_index else "skipped"}',
        f'- ECS inventory: {"generated" if run_ecs_inventory else "skipped"}',
        f'- Agent navigation: {"generated" if run_agent_navigation else "skipped"}',
        f'- Full reports: {"generated" if full_reports else "skipped"}',
        f'- Dependency graph drawings: '
        f'{"rendered" if CONFIG["run_dependency_graph"] else "skipped"}',
        f'- Dirstats: {"generated" if run_dirstats else "skipped"}',
        f'- Live disk inventory: {"generated" if run_live_disk_inventory else "skipped"}',
        '',
        'Submodules:',
    ]
    if context.submodule_decisions:
        for decision in context.submodule_decisions:
            info = decision.info
            txt.append(
                f'- {info.path}: {info.sha} '
                f'depth={"full" if decision.depth is None else decision.depth} '
                f'mode={decision.mode} omitted={decision.omitted} '
                f'reason={decision.reason!r}'
            )
    else:
        txt.append('- (none)')
    txt.append('')
    txt.append('Dirty status at build time:')
    txt.append(dirty_status.rstrip() if dirty_status.strip() else '(clean)')
    manifest_txt.write_text('\n'.join(txt).rstrip() + '\n')

def validate_archive_root(archive_root: Path) -> None:
    required = list(CONFIG['required_archive_paths'])
    if CONFIG['run_agent_index']:
        required.extend(CONFIG['required_agent_index_paths'])
    if CONFIG['run_ecs_inventory']:
        required.extend(CONFIG['required_ecs_inventory_paths'])
    if CONFIG['run_agent_navigation']:
        required.extend(CONFIG['required_agent_navigation_paths'])
    if CONFIG['run_dependency_graph']:
        required.extend(CONFIG['required_dependency_graph_paths'])
    if CONFIG['run_dirstats']:
        required.extend(CONFIG['required_dirstats_paths'])
    if CONFIG['run_live_disk_inventory']:
        required.extend(CONFIG['required_live_disk_inventory_paths'])
    missing = [path for path in required if not (archive_root / path).exists()]
    if missing:
        details = '\n'.join(f'  - {m}' for m in missing)
        raise RuntimeError(f'archive validation failed; missing required paths:\n{details}')



def run_agent_index(archive_root: Path, log: Log) -> None:
    if not CONFIG['run_agent_index']:
        log('[archive-agent-source] skipping agent index generation')
        return
    cmd = [str(part) for part in CONFIG['agent_index_command']]
    log('[archive-agent-source] running agent index: ' + ' '.join(shell_quote(p) for p in cmd))
    env = os.environ.copy()
    env.setdefault('PYTHONUNBUFFERED', '1')
    run(cmd, cwd=archive_root, check=True, capture=False, env=env)



def run_ecs_inventory(archive_root: Path, log: Log) -> None:
    """Generate neutral ECS inventory shards inside the staged archive tree.

    ``ecs_inventory.py`` uses native tree-sitter bindings and declares their
    compatible versions through PEP 723 inline script metadata. Running it with
    ``sys.executable`` bypasses that metadata and can load an ABI-incompatible
    tree-sitter / grammar pair from the caller's environment. That failure mode
    is especially unpleasant because it may present as a native SIGSEGV rather
    than a Python import error.

    Keep the archive boundary deterministic: resolve ``uv`` explicitly and let
    ``uv run --script`` construct the environment declared by the staged script.
    """
    if not CONFIG['run_ecs_inventory']:
        log('[archive-agent-source] skipping ECS inventory generation')
        return
    cmd = [str(part) for part in CONFIG['ecs_inventory_command']]
    if cmd[:3] != ['uv', 'run', '--script']:
        raise CommandError(
            'ECS inventory command must use `uv run --script` so the native '
            'tree-sitter dependency versions declared by scripts/ecs_inventory.py '
            'are honored'
        )
    uv_exe = shutil.which('uv')
    if uv_exe is None:
        raise CommandError(
            'ECS inventory generation requires `uv` because '
            'scripts/ecs_inventory.py declares native tree-sitter dependencies '
            'through PEP 723 inline metadata. Install uv or rerun the archiver '
            'with --skip-ecs-inventory.'
        )
    cmd[0] = uv_exe
    log('[archive-agent-source] running ECS inventory: ' + ' '.join(shell_quote(p) for p in cmd))
    env = os.environ.copy()
    env.setdefault('PYTHONUNBUFFERED', '1')
    run(cmd, cwd=archive_root, check=True, capture=False, env=env)


def run_agent_navigation(archive_root: Path, log: Log) -> None:
    """Build the compact catalog and per-crate drill-down packets."""
    if not CONFIG['run_agent_navigation']:
        log('[archive-agent-source] skipping agent navigation catalog')
        return
    cmd = [str(part) for part in CONFIG['agent_navigation_command']]
    log('[archive-agent-source] building agent navigation: ' + ' '.join(shell_quote(p) for p in cmd))
    env = os.environ.copy()
    env.setdefault('PYTHONUNBUFFERED', '1')
    run(cmd, cwd=archive_root, check=True, capture=False, env=env)


def dependency_graphs_to_draw(archive_root: Path) -> list[str]:
    """Which graphs have something to draw, declared always and resolved if resolved.

    ⚠ read from the staged tree, not from the dev checkout: the generators ran
    in there, and a resolved graph that failed in the archive while succeeding
    locally is exactly the case this has to get right.
    """
    graphs = ['declared']
    resolved = archive_root / '.agent/index/crates/graph-resolved.json'
    try:
        payload = json.loads(resolved.read_text(encoding='utf-8'))
    except (OSError, json.JSONDecodeError):
        return graphs
    if payload.get('available'):
        graphs.append('resolved')
    return graphs


def run_dependency_graph(archive_root: Path, log: Log) -> None:
    """Draw the dependency graphs with graphviz. Opt-in; absence is an error.

    ⛔ **a missing `dot` fails the build here rather than degrading**, because
    this step only runs when it was explicitly asked for. Every other step is on
    by default and has to tolerate a thin machine; this one was requested.
    """
    if not CONFIG['run_dependency_graph']:
        log(
            '[archive-agent-source] skipping dependency graph drawings; pass '
            '--dependency-graph to render them (needs graphviz)'
        )
        return
    if shutil.which('dot') is None:
        raise RuntimeError(
            '--dependency-graph needs graphviz, and the `dot` binary is not on PATH. '
            'Install graphviz, or drop the flag — the JSON graphs carry every edge '
            'either way.'
        )
    base = [str(part) for part in CONFIG['dependency_graph_command']]
    env = os.environ.copy()
    env.setdefault('PYTHONUNBUFFERED', '1')
    for graph in dependency_graphs_to_draw(archive_root):
        for fmt in CONFIG['dependency_graph_formats']:
            cmd = [*base, '--graph', graph, '--format', fmt]
            log(
                '[archive-agent-source] drawing dependency graph: '
                + ' '.join(shell_quote(p) for p in cmd)
            )
            run(cmd, cwd=archive_root, check=True, capture=False, env=env)


def first_primary_span(message: dict[str, object]) -> dict[str, object] | None:
    spans = message.get('spans')
    if not isinstance(spans, list):
        return None
    for span in spans:
        if isinstance(span, dict) and span.get('is_primary'):
            return span
    for span in spans:
        if isinstance(span, dict):
            return span
    return None


def render_cargo_check_warnings_report(
    *,
    command: Sequence[str],
    proc: subprocess.CompletedProcess[str],
    generated_at: str,
    archive_root: Path,
) -> str:
    """Summarize cargo JSON diagnostics into a small markdown report."""
    diagnostics: list[dict[str, object]] = []
    malformed = 0
    for line in (proc.stdout or '').splitlines():
        line = line.strip()
        if not line:
            continue
        try:
            payload = json.loads(line)
        except json.JSONDecodeError:
            malformed += 1
            continue
        if payload.get('reason') != 'compiler-message':
            continue
        message = payload.get('message')
        if not isinstance(message, dict):
            continue
        level = str(message.get('level', '')).lower()
        if level in {'warning', 'error'}:
            diagnostics.append(message)

    by_level = Counter(str(item.get('level', 'unknown')) for item in diagnostics)
    lines: list[str] = []
    lines.append('Cargo check diagnostics')
    lines.append('=======================')
    lines.append('')
    lines.append(f'Generated at UTC: {generated_at}')
    lines.append('Command: `' + ' '.join(shell_quote(str(part)) for part in command) + '`')
    lines.append(f'Exit code: {proc.returncode}')
    lines.append(f'Diagnostics: {len(diagnostics)}')
    for level, count in sorted(by_level.items()):
        lines.append(f'- {level}: {count}')
    if malformed:
        lines.append(f'- malformed JSON lines ignored: {malformed}')
    lines.append('')

    if not diagnostics:
        lines.append('No warnings or errors found in cargo JSON output.')
    else:
        lines.append('## Warnings and errors')
        lines.append('')
        for index, message in enumerate(diagnostics, start=1):
            level = str(message.get('level', 'diagnostic'))
            text = str(message.get('message', '')).replace('\n', ' ')
            code = message.get('code')
            code_text = ''
            if isinstance(code, dict) and code.get('code'):
                code_text = f' `{code["code"]}`'
            span = first_primary_span(message)
            location = ''
            if span is not None:
                file_name = str(span.get('file_name', ''))
                line = span.get('line_start')
                column = span.get('column_start')
                if file_name:
                    location = f' — `{file_name}:{line}:{column}`'
            lines.append(f'{index}. **{level}**{code_text}{location}')
            lines.append(f'   {text}')
            lines.append('')

    stderr = (proc.stderr or '').strip()
    if stderr:
        lines.append('## Stderr')
        lines.append('')
        lines.append('```text')
        stderr_lines = stderr.splitlines()
        max_lines = 200
        lines.extend(stderr_lines[:max_lines])
        if len(stderr_lines) > max_lines:
            lines.append(f'... truncated after {max_lines} of {len(stderr_lines)} lines ...')
        lines.append('```')
        lines.append('')
    return '\n'.join(lines).rstrip() + '\n'


def run_cargo_check_report(archive_root: Path, generated_at: str, log: Log) -> None:
    command = [str(part) for part in CONFIG['cargo_check_command']]
    output = archive_root / str(CONFIG['cargo_check_warnings_output'])
    output.parent.mkdir(parents=True, exist_ok=True)
    log('[archive-agent-source] running cargo check report: ' + ' '.join(shell_quote(p) for p in command))
    env = os.environ.copy()
    # Keep build artifacts out of the archive tree. The tarball only packs
    # archive_root, not this sibling target directory.
    env['CARGO_TARGET_DIR'] = os.fspath(archive_root.parent / '.cargo-check-target')
    proc = run(command, cwd=archive_root, check=False, capture=True, env=env)
    output.write_text(
        render_cargo_check_warnings_report(
            command=command,
            proc=proc,
            generated_at=generated_at,
            archive_root=archive_root,
        ),
        encoding='utf-8',
    )
    if proc.returncode != 0:
        log(f'[archive-agent-source] warning: cargo check exited with {proc.returncode}; report still written to {output.relative_to(archive_root)}')


def run_cargo_modules_report(archive_root: Path, generated_at: str, spec: dict[str, object], log: Log) -> None:
    command = [str(part) for part in spec['command']]  # type: ignore[index]
    output = archive_root / str(spec['output'])
    output.parent.mkdir(parents=True, exist_ok=True)
    log('[archive-agent-source] running cargo modules report: ' + ' '.join(shell_quote(p) for p in command))
    proc = run(command, cwd=archive_root, check=False, capture=True)
    if proc.returncode == 0:
        text = proc.stdout or ''
        if not text.endswith('\n'):
            text += '\n'
        output.write_text(text, encoding='utf-8')
        return

    lines = [
        'Cargo modules report unavailable',
        '================================',
        '',
        f'Generated at UTC: {generated_at}',
        'Command: `' + ' '.join(shell_quote(p) for p in command) + '`',
        f'Exit code: {proc.returncode}',
        '',
        'This report is optional. Install or update `cargo-modules` if you want it populated.',
        '',
    ]
    if proc.stdout:
        lines.extend(['## Stdout', '', '```text', proc.stdout.rstrip(), '```', ''])
    if proc.stderr:
        lines.extend(['## Stderr', '', '```text', proc.stderr.rstrip(), '```', ''])
    output.write_text('\n'.join(lines).rstrip() + '\n', encoding='utf-8')
    log(f'[archive-agent-source] warning: cargo modules exited with {proc.returncode}; placeholder written to {output.relative_to(archive_root)}')


def run_full_agent_reports(archive_root: Path, generated_at: str, log: Log) -> None:
    run_cargo_check_report(archive_root, generated_at, log)
    for spec in CONFIG['cargo_modules_reports']:
        run_cargo_modules_report(archive_root, generated_at, spec, log)

def run_dirstats(archive_root: Path, generated_at: str, log: Log) -> None:
    if not CONFIG['run_dirstats']:
        log('[archive-agent-source] skipping dirstats generation')
        return
    for spec in CONFIG['dirstats']:
        log(f'[archive-agent-source] writing {spec["output"]}')
        write_dirstats_report(archive_root, spec, generated_at, log)


def _load_git_well_archive_api() -> tuple[Any, Any]:
    try:
        from git_well.git_archive_source import ArchiveSourceContext
        from git_well.git_archive_source import archive_source
    except ImportError as ex:
        raise RuntimeError(
            'Ambition source archiving requires git-well with the '
            'programmatic archive hook API. Install or update it with:\n'
            f'  {sys.executable} -m pip install -U "git_well>=0.3.4"'
        ) from ex
    return archive_source, ArchiveSourceContext


@dataclass
class AmbitionArchiveExtension:
    repo_root: Path
    dirty_status: str
    allow_forbidden: bool
    full_reports: bool
    log: Log
    generated_at: str = field(init=False, default='')

    def prepare(self, context: Any) -> None:
        """Add Ambition-specific agent payloads to git-well's staged tree."""
        enforce_forbidden_path_policy(context, self.allow_forbidden, self.log)
        context.add_generated_excludes(
            [
                '/.agent/',
                '/SOURCE_ARCHIVE_MANIFEST.txt',
            ]
        )
        self.generated_at = datetime.now(timezone.utc).strftime(
            '%Y-%m-%dT%H:%M:%SZ'
        )
        run_agent_index(context.archive_root, self.log)
        if CONFIG['run_agent_index']:
            refresh_generation_stamp(context, self.generated_at, self.log)
        run_ecs_inventory(context.archive_root, self.log)
        run_agent_navigation(context.archive_root, self.log)
        run_dependency_graph(context.archive_root, self.log)
        if self.full_reports:
            run_full_agent_reports(
                context.archive_root, self.generated_at, self.log
            )
        else:
            self.log(
                '[archive-agent-source] skipping full reports; pass --full '
                'to run cargo check and cargo-modules reports'
            )
        run_dirstats(context.archive_root, self.generated_at, self.log)
        run_live_disk_inventory(
            self.repo_root,
            context.archive_root,
            self.generated_at,
            context.source_display_path(),
            self.log,
        )
        write_manifests(
            context=context,
            generated_at=self.generated_at,
            dirty_status=self.dirty_status,
            full_reports=self.full_reports,
        )

    def validate(self, context: Any) -> None:
        """Validate the exact payload immediately before serialization."""
        if CONFIG['run_agent_index']:
            validate_generation_stamp(context)
        validate_archive_root(context.archive_root)
        validate_staged_checkout_clean(context)


def build_archive(
    repo_arg: Path,
    output_arg: Path | None,
    prefix_arg: str | None,
    keep_stage: bool,
    verbose: int,
    allow_forbidden: bool = False,
    full_reports: bool = False,
    depth: object | None = None,
    all_branches: bool = False,
    submodule_depth: object | None = None,
    exclude_submodule: list[str] | None = None,
    no_submodules: bool = False,
    archive_format: str = 'tar.gz',
    redact_local_paths: bool = False,
) -> Path:
    log = Log(verbose)
    repo_root = coerce_repo_root(repo_arg)
    repo_name = str(CONFIG['repo_name']) or repo_root.name
    head_sha = git(repo_root, 'rev-parse', 'HEAD')
    short_sha = git(repo_root, 'rev-parse', '--short=12', 'HEAD')
    dirty_status = git(repo_root, 'status', '--short')

    if dirty_status.strip() and CONFIG['fail_if_dirty']:
        raise RuntimeError(
            'worktree is dirty and CONFIG["fail_if_dirty"] is true:\n'
            + dirty_status
        )
    if dirty_status.strip():
        log(
            '[archive-agent-source] warning: source archive is committed-only; '
            'dirty changes are not included'
        )

    timestamp = datetime.now(timezone.utc).strftime('%Y%m%dT%H%M%SZ')
    prefix = prefix_arg or str(CONFIG['prefix_template']).format(
        repo=repo_name,
        timestamp=timestamp,
        short_sha=short_sha,
    )
    effective_depth = CONFIG['super_depth'] if depth is None else depth
    if not CONFIG['include_git_history']:
        effective_depth = 0
    effective_submodule_depth = (
        CONFIG['submodule_depths']
        if submodule_depth is None
        else submodule_depth
    )

    archive_source, _ = _load_git_well_archive_api()
    extension = AmbitionArchiveExtension(
        repo_root=repo_root,
        dirty_status=dirty_status,
        allow_forbidden=allow_forbidden,
        full_reports=full_reports,
        log=log,
    )
    return archive_source(
        repo_dpath=repo_root,
        output=output_arg,
        depth=effective_depth,
        all_branches=all_branches,
        submodule_depth=effective_submodule_depth,
        exclude_submodule=exclude_submodule,
        no_submodules=no_submodules,
        format=archive_format,
        redact_local_paths=redact_local_paths,
        verbose=verbose,
        prepare=extension.prepare,
        validate=extension.validate,
        archive_root_name=prefix,
        keep_stage=keep_stage,
    )


def main(argv: Sequence[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter
    )
    parser.add_argument(
        'repo',
        nargs='?',
        default='.',
        type=Path,
        help='repository path, default: current directory',
    )
    parser.add_argument(
        '-o',
        '--output',
        type=Path,
        default=None,
        help='exact archive path; relative paths are resolved under the repo root',
    )
    parser.add_argument(
        '--prefix',
        default=None,
        help='override the top-level directory name inside the archive',
    )
    parser.add_argument(
        '--depth',
        default=None,
        help='superproject history depth: 0, a positive integer, or full; default: CONFIG["super_depth"]',
    )
    parser.add_argument(
        '--all-branches',
        action='store_true',
        help='include every locally cached local and remote-tracking branch',
    )
    parser.add_argument(
        '--submodule-depth',
        default=None,
        help='git-well YAML depth scalar/mapping; default: CONFIG["submodule_depths"]',
    )
    parser.add_argument(
        '--exclude-submodule',
        action='append',
        default=None,
        metavar='PATH_OR_GLOB',
        help='omit a recursive submodule path or quoted fnmatch glob; repeatable',
    )
    parser.add_argument(
        '--no-submodules',
        action='store_true',
        help='omit all recursive submodule working trees',
    )
    parser.add_argument(
        '--format',
        dest='archive_format',
        default='tar.gz',
        choices=['auto', 'tar', 'tar.gz', 'tgz', 'zip', 'tar.bz2', 'tbz2', 'tar.xz', 'txz'],
        help='archive format; default: tar.gz',
    )
    parser.add_argument(
        '--redact-local-paths',
        action='store_true',
        help='redact local source/output paths and local clone origins',
    )
    parser.add_argument(
        '--skip-index',
        action='store_true',
        help='do not run CONFIG["agent_index_command"]',
    )
    parser.add_argument(
        '--skip-ecs-inventory',
        action='store_true',
        help='do not run CONFIG["ecs_inventory_command"]',
    )
    parser.add_argument(
        '--skip-agent-navigation',
        action='store_true',
        help='do not build .agent/README.md, catalog, or crate packets',
    )
    parser.add_argument(
        '--dependency-graph',
        action='store_true',
        help='render the crate dependency graphs with graphviz (needs the `dot` binary)',
    )
    parser.add_argument(
        '--skip-dirstats',
        action='store_true',
        help='do not generate the staged dirstats reports',
    )
    parser.add_argument(
        '--skip-live-inventory',
        action='store_true',
        help='do not generate live-disk inventory or live git-status reports',
    )
    parser.add_argument(
        '--full',
        action='store_true',
        help='also run cargo check and cargo-modules reports',
    )
    parser.add_argument(
        '-s',
        '--slim',
        '--quick',
        '--fast',
        action='count',
        default=0,
        help=(
            'repeatable slimness (-s, -ss, -sss, -ssss); incompatible with --full. '
            '1: stage source/history only, skipping every generated Ambition payload. '
            '2: also cap superproject depth at 100. '
            '3: also omit submodule working trees. '
            '4: also cut superproject depth to 10. '
            'An explicit --depth always wins over the level default.'
        ),
    )
    parser.add_argument(
        '--allow-forbidden',
        action='store_true',
        help='bypass CONFIG["forbidden_path_globs"] guardrails for this run',
    )
    parser.add_argument(
        '--keep-stage',
        action='store_true',
        help='retain git-well temporary staging directory for debugging',
    )
    parser.add_argument('-q', '--quiet', action='store_true', help='reduce logging')
    args = parser.parse_args(argv)

    if args.slim:
        if args.full:
            parser.error('--slim/--quick/--fast and --full are mutually exclusive')
        args.skip_index = True
        args.skip_ecs_inventory = True
        args.skip_agent_navigation = True
        args.skip_dirstats = True
        args.skip_live_inventory = True
        # An explicitly requested depth is the user's answer, not the level's.
        if args.depth is None:
            if args.slim >= 4:
                args.depth = 10
            elif args.slim >= 2:
                args.depth = 100
        if args.slim >= 3:
            args.no_submodules = True

    step_toggles = {
        'run_agent_index': not args.skip_index,
        'run_ecs_inventory': not args.skip_ecs_inventory,
        'run_agent_navigation': not args.skip_agent_navigation and not args.skip_index,
        # Nothing to draw without the graphs the navigation step writes, so the
        # flag cannot turn this on over a skipped prerequisite.
        'run_dependency_graph': (
            args.dependency_graph
            and not args.skip_agent_navigation
            and not args.skip_index
        ),
        'run_dirstats': not args.skip_dirstats,
        'run_live_disk_inventory': not args.skip_live_inventory,
    }
    saved = {key: CONFIG[key] for key in step_toggles}
    # Assign the toggle rather than only clearing it: `run_dependency_graph`
    # defaults to False and is turned ON by a flag, so a clear-only loop would
    # accept `--dependency-graph` and silently draw nothing.
    for key, keep in step_toggles.items():
        CONFIG[key] = keep
    try:
        output = build_archive(
            args.repo,
            args.output,
            args.prefix,
            args.keep_stage,
            verbose=0 if args.quiet else 1,
            allow_forbidden=args.allow_forbidden,
            full_reports=args.full,
            depth=args.depth,
            all_branches=args.all_branches,
            submodule_depth=args.submodule_depth,
            exclude_submodule=args.exclude_submodule,
            no_submodules=args.no_submodules,
            archive_format=args.archive_format,
            redact_local_paths=args.redact_local_paths,
        )
    finally:
        CONFIG.update(saved)
    print_output_location(output)
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
