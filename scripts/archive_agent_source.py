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
        .agent/ecs_inventory/...       # per-crate ECS inventory shards
        .agent/reports/...             # optional full diagnostic reports
        .agent/dirstats-*.txt         # freshly generated from staged contents
        .agent/source_archive_manifest.yaml
        .agent/manifest.yaml              # refreshed to match staged HEAD
        .agent/live-disk-inventory-*.txt  # metadata only, from live checkout
        SOURCE_ARCHIVE_MANIFEST.txt
        ... tracked source files ...

Only committed/tracked source is copied from the superproject and initialized
submodules. Generated agent metadata is then added to the staged tree before the
tarball is written, so it lands under the same archive prefix as the source.
The optional live-disk inventory records where ignored/untracked assets exist in
the user's checkout, but it does not copy those files into the archive.
"""

from __future__ import annotations

import argparse
import fnmatch
import json
import os
import shutil
import subprocess
import sys
import tarfile
import tempfile
import textwrap
from collections import Counter
from dataclasses import dataclass, field
from datetime import datetime, timezone
from pathlib import Path
from typing import Iterable, Sequence

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
    'output_template': '{prefix}.tar.gz',

    # Git history policy. Use None or 'full' for full history, a positive int
    # for a shallow clone, or 0 for source-only git-archive mode.
    'include_git_history': True,
    'super_depth': 100,
    'submodule_depths': {
        # Keep this explicit so it is easy to tune as submodules grow.
        'tools/ambition_sfx_renderer': 50,
        '*': 25,
    },
    'strip_remotes': True,

    # Dirty worktree policy. The archive is built from committed source. If this
    # is True, local modifications cause an error instead of only a warning.
    'fail_if_dirty': False,

    # Agent index policy. This command is run inside the staged archive root,
    # after source/submodules are materialized and before archiving.
    'run_agent_index': True,
    'agent_index_command': [sys.executable, 'scripts/generate_agent_index.py'],
    'required_agent_index_paths': [
        '.agent/manifest.yaml',
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
    ],

    # Agent discovery reports. The ECS inventory is intentionally a neutral
    # inventory, not a migration planner. Full reports are opt-in because they
    # can be slower and require extra local cargo plugins.
    'run_ecs_inventory': True,
    'ecs_inventory_command': [
        'python',
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
            'output': '.agent/reports/module-tree-ambition_actors.md',
            'command': ['cargo', 'modules', 'structure', '--package', 'ambition_actors', '--lib'],
        },
        {
            'output': '.agent/reports/module-dependencies-ambition_actors.md',
            'command': ['cargo', 'modules', 'dependencies', '--package', 'ambition_actors', '--lib'],
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


@dataclass(frozen=True)
class SubmoduleInfo:
    status: str
    sha: str
    path: str
    line: str


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


def normalize_depth(value: object) -> int | None:
    if value is None or value == '' or value == 'full':
        return None
    if isinstance(value, bool):
        raise ValueError('depth must be None, "full", 0, or a positive integer')
    ivalue = int(value)  # type: ignore[arg-type]
    if ivalue < 0:
        raise ValueError('depth cannot be negative')
    return ivalue


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


def parse_submodule_status(repo_root: Path) -> list[SubmoduleInfo]:
    proc = run(
        ['git', '-C', repo_root, 'submodule', 'status', '--recursive'],
        capture=True,
        check=False,
    )
    if proc.returncode != 0 or not proc.stdout.strip():
        return []

    infos: list[SubmoduleInfo] = []
    for line in proc.stdout.splitlines():
        if not line.strip():
            continue
        parts = line.split()
        if len(parts) < 2:
            raise RuntimeError(f'could not parse submodule status line: {line!r}')
        status = line[0]
        first = parts[0]
        sha = first if status == ' ' else first[1:]
        path = parts[1]
        infos.append(SubmoduleInfo(status=status, sha=sha, path=path, line=line))
    return infos


def resolve_submodule_depth(path: str) -> int | None:
    table = CONFIG['submodule_depths']
    assert isinstance(table, dict)
    if path in table:
        return normalize_depth(table[path])
    for pattern, value in table.items():
        if pattern != '*' and fnmatch.fnmatch(path, pattern):
            return normalize_depth(value)
    return normalize_depth(table.get('*', CONFIG['super_depth']))


def clone_committed_checkout(
    *,
    src: Path,
    dst: Path,
    commit: str,
    depth: int | None,
    label: str,
    log: Log,
) -> None:
    if dst.exists() or dst.is_symlink():
        if dst.is_dir() and not dst.is_symlink():
            shutil.rmtree(dst)
        else:
            dst.unlink()
    dst.parent.mkdir(parents=True, exist_ok=True)

    clone_args = [
        'git',
        'clone',
        '--quiet',
        '--no-local',
        '--single-branch',
        '--no-checkout',
    ]
    if depth is not None:
        clone_args += ['--depth', str(depth)]
    clone_args += [file_url(src), os.fspath(dst)]

    log(f'[archive-agent-source] clone {label}: {src} -> {dst}')
    run(clone_args, check=True)

    try:
        git(dst, 'checkout', '-q', '--detach', commit)
    except CommandError:
        # A submodule commit can be outside the shallow default ref. Fetch the
        # exact object, retaining the requested depth where possible.
        fetch_args = ['fetch', '--quiet']
        if depth is not None:
            fetch_args += ['--depth', str(depth)]
        fetch_args += ['origin', commit]
        try:
            git(dst, *fetch_args)
        except CommandError:
            git(dst, 'fetch', '--quiet', 'origin', commit)
        git(dst, 'checkout', '-q', '--detach', commit)

    if CONFIG['strip_remotes']:
        git(dst, 'remote', 'remove', 'origin', check=False)

    git(dst, 'reflog', 'expire', '--expire=now', '--expire-unreachable=now', '--all', check=False)
    git(dst, 'gc', '--prune=now', '--quiet', check=False)


def export_git_archive(src: Path, dst_parent: Path, prefix: str, treeish: str = 'HEAD') -> None:
    import io

    proc = subprocess.run(
        ['git', '-C', src, 'archive', '--format=tar', f'--prefix={prefix.rstrip("/")}/', treeish],
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=False,
    )
    if proc.returncode != 0:
        raise CommandError(proc.stderr.decode('utf8', errors='replace'))
    with tarfile.open(fileobj=io.BytesIO(proc.stdout), mode='r:') as tar:
        safe_extractall(tar, dst_parent)


def safe_extractall(tar: tarfile.TarFile, dst: Path) -> None:
    dst = dst.resolve()
    for member in tar.getmembers():
        target = (dst / member.name).resolve()
        try:
            target.relative_to(dst)
        except ValueError as ex:
            raise RuntimeError(f'unsafe tar member path: {member.name!r}') from ex
    try:
        tar.extractall(path=dst, filter='fully_trusted')
    except TypeError:
        tar.extractall(path=dst)


def append_archive_excludes(repo_path: Path) -> None:
    info = repo_path / '.git' / 'info'
    if not info.exists():
        return
    exclude = info / 'exclude'
    with exclude.open('a') as file:
        file.write(
            textwrap.dedent(
                '''

                # Added by scripts/archive_agent_source.py so generated archive
                # metadata does not dirty the unpacked inspection checkout.
                /.agent/
                /SOURCE_ARCHIVE_MANIFEST.txt
                '''
            ).lstrip('\n')
        )


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



def write_live_disk_inventory_report(repo_root: Path, archive_root: Path, spec: dict[str, object], generated_at: str) -> None:
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
    lines.append(f'Live repo root: {repo_root}')
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


def write_live_git_status_report(repo_root: Path, archive_root: Path, generated_at: str) -> None:
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
        f'Live repo root: {repo_root}',
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


def run_live_disk_inventory(repo_root: Path, archive_root: Path, generated_at: str, log: Log) -> None:
    if not CONFIG['run_live_disk_inventory']:
        log('[archive-agent-source] skipping live disk inventory')
        return
    for spec in CONFIG['live_disk_inventory']:
        log(f'[archive-agent-source] writing {spec["output"]}')
        write_live_disk_inventory_report(repo_root, archive_root, spec, generated_at)
    log(f'[archive-agent-source] writing {CONFIG["live_git_status_output"]}')
    write_live_git_status_report(repo_root, archive_root, generated_at)


def path_matches_any(relpath: str, patterns: Iterable[str]) -> bool:
    return any(fnmatch.fnmatch(relpath, pattern) for pattern in patterns)


def history_max_count(depth: int | None) -> str | None:
    if depth is None:
        return None
    if depth <= 0:
        return '1'
    return str(depth)


def find_forbidden_paths(repo_root: Path, super_depth: int | None) -> dict[str, list[str]]:
    patterns = [str(p) for p in CONFIG['forbidden_path_globs']]
    found: dict[str, list[str]] = {'head': [], 'history': []}
    if not patterns:
        return found

    head_paths = git(repo_root, 'ls-tree', '-r', '--name-only', 'HEAD').splitlines()
    found['head'] = sorted({p for p in head_paths if path_matches_any(p, patterns)})

    if CONFIG['scan_forbidden_history']:
        args = ['log', '--format=', '--name-only']
        max_count = history_max_count(super_depth)
        if max_count is not None:
            args.append(f'--max-count={max_count}')
        args.append('HEAD')
        hist_paths = git(repo_root, *args).splitlines()
        found['history'] = sorted({p for p in hist_paths if path_matches_any(p, patterns)})

    return found


def enforce_forbidden_path_policy(repo_root: Path, super_depth: int | None, allow_forbidden: bool, log: Log) -> None:
    if allow_forbidden:
        log('[archive-agent-source] forbidden path guard disabled by --allow-forbidden')
        return
    policy = str(CONFIG['forbidden_path_policy'])
    if policy == 'ignore':
        return
    if policy not in {'fail', 'warn'}:
        raise ValueError('CONFIG["forbidden_path_policy"] must be one of: fail, warn, ignore')
    found = find_forbidden_paths(repo_root, super_depth)
    if not found['head'] and not found['history']:
        return
    parts = ['forbidden paths matched archive guard:']
    if found['head']:
        parts.append('  present in HEAD:')
        parts.extend(f'    - {p}' for p in found['head'][:50])
        if len(found['head']) > 50:
            parts.append(f'    ... {len(found["head"]) - 50} more ...')
    if found['history']:
        parts.append('  present in included HEAD history:')
        parts.extend(f'    - {p}' for p in found['history'][:50])
        if len(found['history']) > 50:
            parts.append(f'    ... {len(found["history"]) - 50} more ...')
    message = '\n'.join(parts)
    if policy == 'fail':
        raise RuntimeError(message + '\nSet CONFIG["forbidden_path_policy"] to "warn" or run --allow-forbidden only if this is intentional.')
    log('[archive-agent-source] warning: ' + message.replace('\n', '\n[archive-agent-source] warning: '))

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


def refresh_agent_manifest(archive_root: Path, *, generated_at: str, short_sha: str, log: Log) -> None:
    """Refresh ``.agent/manifest.yaml`` metadata after the indexer runs.

    The repo's indexer historically regenerated ``.agent/index/*.json`` but did
    not rewrite the manifest itself. Because the archive builder stages a fresh
    checkout, this function makes the staged manifest match the staged HEAD and
    fails early if the indexer did not leave a manifest behind.
    """
    manifest = archive_root / '.agent' / 'manifest.yaml'
    if not manifest.exists():
        raise RuntimeError(
            'agent indexer did not produce required .agent/manifest.yaml; '
            'cannot build a self-describing agent archive'
        )
    text = manifest.read_text(encoding='utf-8')
    updates = {
        'generated_from_commit': short_sha,
        'generated_at': generated_at,
        'generator': 'scripts/generate_agent_index.py',
    }
    patched = patch_yaml_top_level_scalars(text, updates, after_key='schema_version')
    manifest.write_text(patched, encoding='utf-8')
    log(f'[archive-agent-source] refreshed {manifest.relative_to(archive_root)}')


def validate_agent_manifest_metadata(archive_root: Path, *, short_sha: str) -> None:
    """Ensure the packaged agent manifest is not stale."""
    manifest = archive_root / '.agent' / 'manifest.yaml'
    if not manifest.exists():
        raise RuntimeError('missing required .agent/manifest.yaml')
    text = manifest.read_text(encoding='utf-8')
    expected = f'generated_from_commit: {short_sha}'
    quoted_expected = f'generated_from_commit: "{short_sha}"'
    if expected not in text and quoted_expected not in text:
        raise RuntimeError(
            '.agent/manifest.yaml was not refreshed for this archive: '
            f'expected generated_from_commit={short_sha!r}'
        )
    if 'generated_from_commit: unknown' in text:
        raise RuntimeError('.agent/manifest.yaml still says generated_from_commit: unknown')


def write_manifests(
    *,
    archive_root: Path,
    repo_root: Path,
    prefix: str,
    generated_at: str,
    head_sha: str,
    short_sha: str,
    dirty_status: str,
    submodules: list[SubmoduleInfo],
    include_git: bool,
    super_depth: int | None,
    full_reports: bool,
) -> None:
    agent_dir = archive_root / '.agent'
    agent_dir.mkdir(exist_ok=True)
    manifest_yaml = agent_dir / 'source_archive_manifest.yaml'
    manifest_txt = archive_root / 'SOURCE_ARCHIVE_MANIFEST.txt'

    submodule_rows = []
    for info in submodules:
        submodule_rows.append((info, resolve_submodule_depth(info.path)))

    yaml_lines = [
        'schema_version: 1',
        f'generated_at_utc: {yaml_scalar(generated_at)}',
        f'generator: {yaml_scalar("scripts/archive_agent_source.py")}',
        f'repo_name: {yaml_scalar(CONFIG["repo_name"])}',
        f'source_repo_root: {yaml_scalar(repo_root)}',
        f'archive_prefix: {yaml_scalar(prefix)}',
        'git:',
        f'  include_history: {str(include_git).lower()}',
        f'  super_depth: {yaml_scalar("full" if super_depth is None else super_depth)}',
        f'  head_sha: {yaml_scalar(head_sha)}',
        f'  short_sha: {yaml_scalar(short_sha)}',
        f'  dirty_worktree_at_build_time: {str(bool(dirty_status.strip())).lower()}',
    ]
    if submodule_rows:
        yaml_lines.append('submodules:')
        for info, depth in submodule_rows:
            yaml_lines.extend([
                f'  - path: {yaml_scalar(info.path)}',
                f'    sha: {yaml_scalar(info.sha)}',
                f'    status: {yaml_scalar(info.status)}',
                f'    depth: {yaml_scalar("full" if depth is None else depth)}',
            ])
    else:
        yaml_lines.append('submodules: []')

    yaml_lines.extend([
        'generated_payloads:',
        '  agent_manifest: .agent/manifest.yaml',
        '  source_archive_manifest: .agent/source_archive_manifest.yaml',
        '  agent_index_command:',
    ])
    for item in CONFIG['agent_index_command']:
        yaml_lines.append(f'    - {yaml_scalar(item)}')
    yaml_lines.append('  ecs_inventory_command:')
    for item in CONFIG['ecs_inventory_command']:
        yaml_lines.append(f'    - {yaml_scalar(item)}')
    yaml_lines.append('  agent_navigation_command:')
    for item in CONFIG['agent_navigation_command']:
        yaml_lines.append(f'    - {yaml_scalar(item)}')
    yaml_lines.append(f'  agent_readme: {yaml_scalar(".agent/README.md")}')
    yaml_lines.append(f'  agent_catalog: {yaml_scalar(".agent/index/catalog.json")}')
    yaml_lines.append(f'  agent_crate_index: {yaml_scalar(".agent/index/crates/index.json")}')
    yaml_lines.append(f'  ecs_inventory_project: {yaml_scalar(".agent/ecs_inventory/project.md")}')
    yaml_lines.append(f'  full_reports_enabled: {str(bool(full_reports)).lower()}')
    yaml_lines.append('  full_report_outputs:')
    yaml_lines.append(f'    cargo_check_warnings: {yaml_scalar(CONFIG["cargo_check_warnings_output"])}')
    yaml_lines.append('    cargo_modules:')
    for spec in CONFIG['cargo_modules_reports']:
        yaml_lines.append(f'      - {yaml_scalar(spec["output"])}')
    yaml_lines.append('  dirstats:')
    for spec in CONFIG['dirstats']:
        yaml_lines.extend([
            f'    - output: {yaml_scalar(spec["output"])}',
            f'      path: {yaml_scalar(spec["path"])}',
            f'      display_depth: {yaml_scalar(spec["display_depth"] if spec["display_depth"] is not None else "full")}',
        ])
    yaml_lines.append('  live_disk_inventory:')
    for spec in CONFIG['live_disk_inventory']:
        yaml_lines.extend([
            f'    - output: {yaml_scalar(spec["output"])}',
            f'      path: {yaml_scalar(spec["path"])}',
            f'      display_depth: {yaml_scalar(spec["display_depth"] if spec["display_depth"] is not None else "full")}',
        ])
    yaml_lines.append(f'  live_git_status_output: {yaml_scalar(CONFIG["live_git_status_output"])}')
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
        f'Source repository root: {repo_root}',
        f'Archive prefix: {prefix}',
        f'Superproject HEAD: {head_sha}',
        f'Superproject short HEAD: {short_sha}',
        f'Git history included: {"yes" if include_git else "no"}',
        f'Superproject depth: {"full" if super_depth is None else super_depth}',
        '',
        'Policy:',
        '- Archives committed/tracked source from the superproject and initialized recursive submodules.',
        '- Rebuilds .agent index metadata inside the staged archive tree before writing the tarball.',
        '- Generates ECS inventory shards inside the staged archive tree.',
        '- Generates a compact .agent/README.md, catalog, and per-crate drill-down packets.',
        '- Generates optional full reports when --full is used.',
        '- Generates dirstats inside the staged archive tree so paths match what agents unpack.',
        '- Writes metadata-only live disk inventory so agents can see where ignored/generated assets live.',
        '- Excludes local untracked files, ignored build products, and dirty worktree changes from source contents.',
        '',
        'Submodules:',
    ]
    if submodule_rows:
        for info, depth in submodule_rows:
            txt.append(f'- {info.path}: {info.sha} depth={"full" if depth is None else depth} status={info.status!r}')
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
    if CONFIG['run_dirstats']:
        required.extend(CONFIG['required_dirstats_paths'])
    if CONFIG['run_live_disk_inventory']:
        required.extend(CONFIG['required_live_disk_inventory_paths'])
    missing = [path for path in required if not (archive_root / path).exists()]
    if missing:
        details = '\n'.join(f'  - {m}' for m in missing)
        raise RuntimeError(f'archive validation failed; missing required paths:\n{details}')


def write_tar_gz(stage: Path, prefix: str, output: Path) -> None:
    root = stage / prefix
    output.parent.mkdir(parents=True, exist_ok=True)
    tmp_output = output.with_suffix(output.suffix + '.tmp')
    if tmp_output.exists():
        tmp_output.unlink()
    with tarfile.open(tmp_output, mode='w:gz', compresslevel=6) as tar:
        tar.add(root, arcname=prefix, recursive=True)
    tmp_output.replace(output)


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
    """Generate neutral ECS inventory shards inside the staged archive tree."""
    if not CONFIG['run_ecs_inventory']:
        log('[archive-agent-source] skipping ECS inventory generation')
        return
    cmd = [str(part) for part in CONFIG['ecs_inventory_command']]
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


def build_archive(
    repo_arg: Path,
    output_arg: Path | None,
    prefix_arg: str | None,
    keep_stage: bool,
    verbose: int,
    allow_forbidden: bool = False,
    full_reports: bool = False,
) -> Path:
    log = Log(verbose)
    repo_root = coerce_repo_root(repo_arg)
    repo_name = str(CONFIG['repo_name']) or repo_root.name
    head_sha = git(repo_root, 'rev-parse', 'HEAD')
    short_sha = git(repo_root, 'rev-parse', '--short=12', 'HEAD')
    dirty_status = git(repo_root, 'status', '--short')

    if dirty_status.strip() and CONFIG['fail_if_dirty']:
        raise RuntimeError('worktree is dirty and CONFIG["fail_if_dirty"] is true:\n' + dirty_status)
    if dirty_status.strip():
        log('[archive-agent-source] warning: source archive is committed-only; dirty changes are not included')

    timestamp = datetime.now(timezone.utc).strftime('%Y%m%dT%H%M%SZ')
    prefix = prefix_arg or str(CONFIG['prefix_template']).format(
        repo=repo_name,
        timestamp=timestamp,
        short_sha=short_sha,
    )
    output = output_arg
    if output is None:
        output_name = str(CONFIG['output_template']).format(prefix=prefix, repo=repo_name, timestamp=timestamp, short_sha=short_sha)
        output = repo_root / output_name
    elif not output.is_absolute():
        output = repo_root / output
    output = output.resolve()

    include_git = bool(CONFIG['include_git_history']) and normalize_depth(CONFIG['super_depth']) != 0
    super_depth = normalize_depth(CONFIG['super_depth'])
    if not include_git:
        super_depth = 0

    enforce_forbidden_path_policy(repo_root, super_depth, allow_forbidden, log)

    submodules = parse_submodule_status(repo_root)
    submodules.sort(key=lambda info: (info.path.count('/'), info.path))

    log(f'[archive-agent-source] repo: {repo_root}')
    log(f'[archive-agent-source] prefix: {prefix}')
    log(f'[archive-agent-source] output: {output}')
    log(f'[archive-agent-source] superproject HEAD: {short_sha}')
    log(f'[archive-agent-source] git history: {"included" if include_git else "omitted"}')
    log(f'[archive-agent-source] superproject depth: {"full" if super_depth is None else super_depth}')

    tmp_obj = tempfile.TemporaryDirectory(prefix=f'{repo_name}-agent-archive.')
    tmp = Path(tmp_obj.name)
    stage = tmp / 'stage'
    archive_root = stage / prefix
    stage.mkdir(parents=True, exist_ok=True)

    try:
        if include_git:
            clone_committed_checkout(
                src=repo_root,
                dst=archive_root,
                commit=head_sha,
                depth=super_depth,
                label='superproject',
                log=log,
            )
        else:
            log('[archive-agent-source] exporting superproject source-only')
            export_git_archive(repo_root, stage, prefix, 'HEAD')

        for info in submodules:
            if info.status == '-':
                raise RuntimeError(
                    f'submodule is not initialized: {info.path!r}; run git submodule update --init --recursive'
                )
            sub_src = repo_root / info.path
            if not sub_src.exists():
                raise RuntimeError(f'submodule path is missing: {info.path!r}')
            sub_depth = resolve_submodule_depth(info.path)
            log(f'[archive-agent-source] submodule {info.path}: depth={"full" if sub_depth is None else sub_depth}')
            if include_git and sub_depth != 0:
                clone_committed_checkout(
                    src=sub_src,
                    dst=archive_root / info.path,
                    commit=info.sha,
                    depth=sub_depth,
                    label=f'submodule {info.path}',
                    log=log,
                )
            else:
                target = archive_root / info.path
                if target.exists() or target.is_symlink():
                    if target.is_dir() and not target.is_symlink():
                        shutil.rmtree(target)
                    else:
                        target.unlink()
                export_git_archive(sub_src, stage, f'{prefix}/{info.path}', info.sha)

        # Make generated metadata invisible to git status in the unpacked clone.
        if include_git:
            append_archive_excludes(archive_root)
            for info in submodules:
                append_archive_excludes(archive_root / info.path)

        run_agent_index(archive_root, log)
        generated_at = datetime.now(timezone.utc).strftime('%Y-%m-%dT%H:%M:%SZ')
        refresh_agent_manifest(archive_root, generated_at=generated_at, short_sha=short_sha, log=log)
        run_ecs_inventory(archive_root, log)
        run_agent_navigation(archive_root, log)
        if full_reports:
            run_full_agent_reports(archive_root, generated_at, log)
        else:
            log('[archive-agent-source] skipping full reports; pass --full to run cargo check and cargo-modules reports')
        run_dirstats(archive_root, generated_at, log)
        run_live_disk_inventory(repo_root, archive_root, generated_at, log)
        write_manifests(
            archive_root=archive_root,
            repo_root=repo_root,
            prefix=prefix,
            generated_at=generated_at,
            head_sha=head_sha,
            short_sha=short_sha,
            dirty_status=dirty_status,
            submodules=submodules,
            include_git=include_git,
            super_depth=super_depth,
            full_reports=full_reports,
        )

        if include_git:
            append_archive_excludes(archive_root)
            for info in submodules:
                append_archive_excludes(archive_root / info.path)

        validate_agent_manifest_metadata(archive_root, short_sha=short_sha)
        validate_archive_root(archive_root)
        write_tar_gz(stage, prefix, output)
        log(f'[archive-agent-source] wrote: {output}')
        log(f'[archive-agent-source] inspect: tar -tzf {shell_quote(os.fspath(output))} | less')
        if keep_stage:
            kept = repo_root / f'.tmp-{prefix}-stage'
            if kept.exists():
                shutil.rmtree(kept)
            shutil.copytree(archive_root, kept, symlinks=True)
            log(f'[archive-agent-source] kept staged tree: {kept}')
    finally:
        tmp_obj.cleanup()

    return output


def main(argv: Sequence[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument('repo', nargs='?', default='.', type=Path, help='repository path, default: current directory')
    parser.add_argument('-o', '--output', type=Path, default=None, help='archive path to write, relative paths are under the repo root')
    parser.add_argument('--prefix', default=None, help='override the top-level directory name inside the archive')
    parser.add_argument('--skip-index', action='store_true', help='do not run CONFIG["agent_index_command"]')
    parser.add_argument('--skip-ecs-inventory', action='store_true', help='do not run CONFIG["ecs_inventory_command"]')
    parser.add_argument('--skip-agent-navigation', action='store_true', help='do not build .agent/README.md, catalog, or crate packets')
    parser.add_argument('--skip-dirstats', action='store_true', help='do not generate the staged dirstats reports')
    parser.add_argument('--skip-live-inventory', action='store_true', help='do not generate the live-disk inventory or live git-status reports')
    parser.add_argument('--full', action='store_true', help='also run slower cargo reports: cargo check --workspace --lib and cargo-modules')
    parser.add_argument('--quick', '--fast', action='store_true', help='only stage the shallow clones; skip every heavy step (agent index, navigation catalog, ECS inventory, dirstats, live-disk inventory). Implies all --skip-* flags and is incompatible with --full')
    parser.add_argument('--allow-forbidden', action='store_true', help='bypass CONFIG["forbidden_path_globs"] guardrails for this run')
    parser.add_argument('--keep-stage', action='store_true', help='copy the staged archive root to .tmp-<prefix>-stage for debugging')
    parser.add_argument('-q', '--quiet', action='store_true', help='reduce logging')
    args = parser.parse_args(argv)

    if args.quick:
        if args.full:
            parser.error('--quick and --full are mutually exclusive')
        args.skip_index = True
        args.skip_ecs_inventory = True
        args.skip_agent_navigation = True
        args.skip_dirstats = True
        args.skip_live_inventory = True

    # Each heavy step is a CONFIG toggle; flip the requested ones off for this
    # run and restore the originals afterward so CONFIG stays a static default.
    step_toggles = {
        'run_agent_index': not args.skip_index,
        'run_ecs_inventory': not args.skip_ecs_inventory,
        'run_agent_navigation': not args.skip_agent_navigation and not args.skip_index,
        'run_dirstats': not args.skip_dirstats,
        'run_live_disk_inventory': not args.skip_live_inventory,
    }
    saved = {key: CONFIG[key] for key in step_toggles}
    for key, keep in step_toggles.items():
        if not keep:
            CONFIG[key] = False
    try:
        output = build_archive(
            args.repo,
            args.output,
            args.prefix,
            args.keep_stage,
            verbose=0 if args.quiet else 1,
            allow_forbidden=args.allow_forbidden,
            full_reports=args.full,
        )
    finally:
        CONFIG.update(saved)
    print_output_location(output)
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
