#!/usr/bin/env python3
# PYTHON_ARGCOMPLETE_OK
"""
Inspect and remove Git history bloat.

This script intentionally lives in-repo for now, but it is written as a small
scriptconfig-based modal CLI so it can be promoted into git-well later.

Examples:
    ./git-debloat stats
    ./git-debloat search --limit=30 --min-size=1MB
    ./git-debloat purge docs/planning/plugin_refactor/snapshots
    ./git-debloat purge docs/planning/plugin_refactor/snapshots --execute
    ./git-debloat gc --execute
    ./git-debloat repack --execute --aggressive
"""

from __future__ import annotations

import collections
import dataclasses
import fnmatch
import json
import os
import re
import shlex
import shutil
import subprocess
import textwrap
import time
import urllib.parse
from pathlib import Path
from typing import Any, Dict, Iterable, List, Mapping, Optional, Sequence, Tuple, Union

import scriptconfig as scfg
from rich import print as rprint
from rich.markup import escape as rich_escape
from rich.panel import Panel


PathLike = Union[str, os.PathLike]


@dataclasses.dataclass
class GitBlobRecord:
    oid: str
    path: str
    size: int
    disk_size: Optional[int]


@dataclasses.dataclass
class TargetReport:
    target: str
    match_mode: str
    filter_repo_path: str
    head_state: str
    head_type: Optional[str]
    head_entries: int
    found_in_head: bool
    found_in_history: bool
    rewrite_needed: bool
    num_blob_path_entries: int
    num_unique_blobs: int
    history_raw_bytes: int
    history_disk_bytes: Optional[int]
    head_raw_bytes: int
    direct_touching_commits: int
    descendant_impacted_commits: int
    selected_refs_commits: int
    first_touch: Optional[Dict[str, str]]
    last_touch: Optional[Dict[str, str]]
    largest_blobs: List[Dict[str, Any]]

    def asdict(self) -> Dict[str, Any]:
        return dataclasses.asdict(self)



class CommonRepoMixin(scfg.DataConfig):
    repo = scfg.Value(
        '.',
        short_alias=['C'],
        help='Git repository to inspect. Defaults to the current directory.',
    )
    verbose = scfg.Value(1, help='Verbosity level.')


class StatsCLI(CommonRepoMixin):
    """
    Show current Git object/pack statistics.
    """
    __command__ = 'stats'

    json = scfg.Value(False, isflag=True, help='Emit JSON instead of text.')

    @classmethod
    def main(cls, argv=1, **kwargs: Any) -> Dict[str, Any]:
        argv = kwargs.pop('cmdline', argv)
        config = cls.cli(argv=argv, data=kwargs, strict=True)
        repo_root = coerce_repo_root(config.repo)
        result = repo_storage_stats(repo_root)
        if config.json:
            rprint(rich_escape(json_dumps(result)))
        else:
            print_git_stats(result)
        return result


class SearchCLI(CommonRepoMixin):
    """
    Search for the largest blobs and path prefixes in reachable Git history.
    """
    __command__ = 'search'

    limit = scfg.Value(40, short_alias=['n'], help='Number of largest blobs to show.')
    min_size = scfg.Value('1MB', help='Only show blobs at least this large. Accepts B, KB, MB, GB, KiB, MiB, GiB.')
    group_depth = scfg.Value(4, help='Depth for path-prefix bloat grouping. Use 0 to disable grouped output.')
    group_limit = scfg.Value(30, help='Number of path-prefix groups to show.')
    refs = scfg.Value(None, nargs='+', help='Refs/revisions to inspect. Defaults to --all.')
    include = scfg.Value(None, nargs='+', help='Optional fnmatch globs for paths to include.')
    exclude = scfg.Value(None, nargs='+', help='Optional fnmatch globs for paths to exclude.')
    json = scfg.Value(False, isflag=True, help='Emit JSON instead of text.')

    @classmethod
    def main(cls, argv=1, **kwargs: Any) -> Dict[str, Any]:
        argv = kwargs.pop('cmdline', argv)
        config = cls.cli(argv=argv, data=kwargs, strict=True)
        repo_root = coerce_repo_root(config.repo)
        min_size = parse_size(config.min_size)
        records = list_history_blobs(repo_root, refs=config.refs)
        records = filter_records(records, include=config.include, exclude=config.exclude)

        oid_to_paths: Dict[str, List[str]] = collections.defaultdict(list)
        oid_to_record: Dict[str, GitBlobRecord] = {}
        for rec in records:
            oid_to_paths[rec.oid].append(rec.path)
            old = oid_to_record.get(rec.oid)
            if old is None or rec.size > old.size:
                oid_to_record[rec.oid] = rec

        unique_blobs = list(oid_to_record.values())
        candidate_blobs = [rec for rec in unique_blobs if rec.size >= min_size]
        candidate_blobs.sort(key=lambda r: (r.size, r.disk_size or -1, r.path), reverse=True)

        top_blobs = []
        for rec in candidate_blobs[: int(config.limit)]:
            paths = sorted(set(oid_to_paths[rec.oid]))
            top_blobs.append({
                'oid': rec.oid,
                'raw_size': rec.size,
                'raw_size_human': byte_str(rec.size),
                'disk_size': rec.disk_size,
                'disk_size_human': None if rec.disk_size is None else byte_str(rec.disk_size),
                'path_count': len(paths),
                'paths': paths[:10],
                'paths_truncated': len(paths) > 10,
            })

        grouped = []
        if int(config.group_depth) > 0:
            grouped = summarize_path_groups(
                oid_to_record=oid_to_record,
                oid_to_paths=oid_to_paths,
                depth=int(config.group_depth),
                limit=int(config.group_limit),
                min_size=min_size,
            )

        result = {
            'repo_root': os.fspath(repo_root),
            'refs': config.refs or ['--all'],
            'num_blob_path_entries': len(records),
            'num_unique_blobs': len(unique_blobs),
            'min_size': min_size,
            'min_size_human': byte_str(min_size),
            'top_blobs': top_blobs,
            'top_path_groups': grouped,
        }
        if config.json:
            rprint(rich_escape(json_dumps(result)))
        else:
            print_search_report(result)
        return result


class PurgeCLI(CommonRepoMixin):
    """
    Report on, and optionally remove, paths from reachable Git history.

    Dry-run is the default. The destructive rewrite only happens with
    --execute. This command uses git-filter-repo for execution because it is the
    safest modern tool for path-based history rewrites.
    """
    __command__ = 'purge'

    targets = scfg.Value(
        None,
        position=1,
        nargs='+',
        help='File or directory paths to remove from history. Paths are relative to the repo root.',
    )
    execute = scfg.Value(False, isflag=True, help='Actually rewrite history. Default is report-only dry-run.')
    glob = scfg.Value(False, isflag=True, help='Interpret targets as git-filter-repo path globs / fnmatch-style report globs.')
    refs = scfg.Value(None, nargs='+', help='Optional refs/revisions for reporting and git-filter-repo --refs. Defaults to all refs.')
    report_json = scfg.Value(None, help='Optional path to write the dry-run/report JSON.')
    allow_dirty = scfg.Value(False, isflag=True, help='Allow executing with a dirty worktree. Default refuses dirty worktrees.')
    no_backup_bundle = scfg.Value(False, isflag=True, help='Do not create a .git/git-debloat-backups/*.bundle before executing.')
    no_force_filter_repo = scfg.Value(False, isflag=True, help='Do not pass --force to git-filter-repo when executing.')
    no_restore_remotes = scfg.Value(False, isflag=True, help='Do not restore remote configuration after git-filter-repo removes it. Default restores remotes.')
    no_fetch_restored_remotes = scfg.Value(False, isflag=True, help='Do not fetch restored remotes after rewriting. Default fetches so branch divergence is visible again.')
    max_largest_blobs = scfg.Value(20, help='Number of largest matching blobs to include per target.')
    yes = scfg.Value(False, short_alias=['y'], isflag=True, help='Alias for --execute; useful for scripts.')
    json = scfg.Value(False, isflag=True, help='Emit JSON instead of text.')

    @classmethod
    def main(cls, argv=1, **kwargs: Any) -> Dict[str, Any]:
        argv = kwargs.pop('cmdline', argv)
        config = cls.cli(argv=argv, data=kwargs, strict=True)
        repo_root = coerce_repo_root(config.repo)
        if not config.targets:
            raise SystemExit('purge requires at least one target path')
        execute = bool(config.execute or config.yes)
        targets = [normalize_repo_path(t) for t in config.targets]

        before_stats = repo_storage_stats(repo_root)
        reports = build_purge_reports(
            repo_root=repo_root,
            targets=targets,
            refs=config.refs,
            glob=bool(config.glob),
            max_largest_blobs=int(config.max_largest_blobs),
        )
        rewrite_reports = [r for r in reports if r.rewrite_needed]
        filter_cmd = build_filter_repo_command(
            repo_root=repo_root,
            reports=rewrite_reports,
            refs=config.refs,
            glob=bool(config.glob),
            force=not bool(config.no_force_filter_repo),
        )
        result: Dict[str, Any] = {
            'repo_root': os.fspath(repo_root),
            'execute': execute,
            'refs': config.refs or ['--all'],
            'before_stats': before_stats,
            'targets': [r.asdict() for r in reports],
            'rewrite_needed': bool(rewrite_reports),
            'filter_repo_command': filter_cmd,
            'notes': [
                'direct_touching_commits counts commits whose diff touches the target path.',
                'descendant_impacted_commits estimates commits whose object IDs would change because they descend from direct touching commits.',
                'history_raw_bytes is the sum of unique reachable blob sizes matching the target path; Git pack compression may make actual pack savings smaller.',
            ],
        }

        if config.report_json:
            out_fpath = Path(config.report_json).expanduser()
            if not out_fpath.is_absolute():
                out_fpath = repo_root / out_fpath
            out_fpath.parent.mkdir(parents=True, exist_ok=True)
            out_fpath.write_text(json_dumps(result) + '\n')
            result['report_json'] = os.fspath(out_fpath)

        if config.json:
            rprint(rich_escape(json_dumps(result)))
        else:
            print_purge_report(result)

        if not result['rewrite_needed']:
            rprint('\n[yellow]Nothing to purge:[/yellow] no target matched files in the selected reachable history.')
            return result

        if not execute:
            rprint('\n[dim]Dry-run only.[/dim] Re-run with [bold]--execute[/bold] to rewrite history.')
            return result

        if not config.allow_dirty:
            assert_clean_worktree(repo_root)
        warn_untracked_worktree(repo_root)
        ensure_filter_repo_available()

        remote_snapshot = None
        if not config.no_restore_remotes:
            remote_snapshot = snapshot_remote_config(repo_root)
            result['remote_snapshot'] = remote_snapshot
            snapshot_fpath = write_remote_snapshot(repo_root, remote_snapshot)
            result['remote_snapshot_path'] = os.fspath(snapshot_fpath)

        print_rewrite_danger_banner(
            repo_root=repo_root,
            reports=rewrite_reports,
            filter_cmd=filter_cmd,
            remote_snapshot=remote_snapshot,
            will_restore_remotes=not bool(config.no_restore_remotes),
            will_fetch_restored_remotes=not bool(config.no_fetch_restored_remotes),
        )

        if not config.no_backup_bundle:
            bundle_fpath = create_backup_bundle(repo_root)
            result['backup_bundle'] = os.fspath(bundle_fpath)
            rprint(f'Created backup bundle: {format_path(bundle_fpath)}')

        rprint('\n[bold red]Executing history rewrite:[/bold red]')
        rprint(format_command(filter_cmd))
        subprocess.run(filter_cmd, check=True)

        if remote_snapshot is not None:
            restore_remote_config(repo_root, remote_snapshot)
            rprint('\n[bold green]Restored Git remote configuration removed by git-filter-repo.[/bold green]')
            print_remote_summary(repo_root)
            if not config.no_fetch_restored_remotes:
                fetch_results = fetch_restored_remotes(repo_root, remote_snapshot)
                result['fetch_restored_remotes'] = fetch_results
                print_fetch_results(fetch_results)

        after_stats = repo_storage_stats(repo_root)
        result['after_filter_repo_stats'] = after_stats
        rprint('\n[bold green]Rewrite complete.[/bold green] Run [bold]./git-debloat gc --execute[/bold] to expire reflogs and prune unreachable objects locally.')
        rprint('[yellow]Remember:[/yellow] collaborators must reclone or hard-reset to the rewritten history, and publication usually requires [bold]git push --force-with-lease[/bold].')
        return result


class GcCLI(CommonRepoMixin):
    """
    Expire reflogs and prune unreachable objects from the local checkout.
    """
    __command__ = 'gc'

    execute = scfg.Value(False, isflag=True, help='Actually run cleanup. Default is dry-run.')
    prune = scfg.Value('now', help='Prune date passed to git gc --prune=<value>.')
    aggressive = scfg.Value(False, isflag=True, help='Pass --aggressive to git gc.')
    yes = scfg.Value(False, short_alias=['y'], isflag=True, help='Alias for --execute.')
    json = scfg.Value(False, isflag=True, help='Emit JSON instead of text.')

    @classmethod
    def main(cls, argv=1, **kwargs: Any) -> Dict[str, Any]:
        argv = kwargs.pop('cmdline', argv)
        config = cls.cli(argv=argv, data=kwargs, strict=True)
        repo_root = coerce_repo_root(config.repo)
        execute = bool(config.execute or config.yes)
        before = repo_storage_stats(repo_root)
        commands = [
            ['git', '-C', os.fspath(repo_root), 'reflog', 'expire', '--expire=now', '--expire-unreachable=now', '--all'],
            ['git', '-C', os.fspath(repo_root), 'gc', f'--prune={config.prune}'],
        ]
        if config.aggressive:
            commands[-1].append('--aggressive')
        result: Dict[str, Any] = {
            'repo_root': os.fspath(repo_root),
            'execute': execute,
            'before_stats': before,
            'commands': commands,
        }
        if not config.json:
            rprint('[bold]Git GC plan:[/bold]')
            for cmd in commands:
                rprint('  ' + format_command(cmd))
        if execute:
            for cmd in commands:
                subprocess.run(cmd, check=True)
            result['after_stats'] = repo_storage_stats(repo_root)
        if config.json:
            rprint(rich_escape(json_dumps(result)))
        elif execute:
            rprint('\n[bold]Before:[/bold]')
            print_git_stats(result['before_stats'], indent='  ')
            rprint('\n[bold]After:[/bold]')
            print_git_stats(result['after_stats'], indent='  ')
        else:
            rprint('\n[dim]Dry-run only.[/dim] Re-run with [bold]--execute[/bold] to run GC.')
        return result


class RepackCLI(CommonRepoMixin):
    """
    Repack Git objects for better local compression.
    """
    __command__ = 'repack'

    execute = scfg.Value(False, isflag=True, help='Actually repack. Default is dry-run.')
    aggressive = scfg.Value(False, isflag=True, help='Use a slower deeper repack: -a -d -f --depth --window.')
    depth = scfg.Value(250, help='Depth for --aggressive repack mode.')
    window = scfg.Value(250, help='Window for --aggressive repack mode.')
    no_commit_graph = scfg.Value(False, isflag=True, help='Do not write commit-graph after repack.')
    yes = scfg.Value(False, short_alias=['y'], isflag=True, help='Alias for --execute.')
    json = scfg.Value(False, isflag=True, help='Emit JSON instead of text.')

    @classmethod
    def main(cls, argv=1, **kwargs: Any) -> Dict[str, Any]:
        argv = kwargs.pop('cmdline', argv)
        config = cls.cli(argv=argv, data=kwargs, strict=True)
        repo_root = coerce_repo_root(config.repo)
        execute = bool(config.execute or config.yes)
        before = repo_storage_stats(repo_root)
        if config.aggressive:
            commands = [[
                'git', '-C', os.fspath(repo_root), 'repack', '-a', '-d', '-f',
                f'--depth={int(config.depth)}', f'--window={int(config.window)}',
            ]]
        else:
            commands = [[
                'git', '-C', os.fspath(repo_root), 'repack', '-A', '-d',
            ]]
        commands.append(['git', '-C', os.fspath(repo_root), 'prune-packed'])
        if not config.no_commit_graph:
            commands.append(['git', '-C', os.fspath(repo_root), 'commit-graph', 'write', '--reachable', '--changed-paths'])
        result: Dict[str, Any] = {
            'repo_root': os.fspath(repo_root),
            'execute': execute,
            'before_stats': before,
            'commands': commands,
        }
        if not config.json:
            rprint('[bold]Git repack plan:[/bold]')
            for cmd in commands:
                rprint('  ' + format_command(cmd))
        if execute:
            for cmd in commands:
                subprocess.run(cmd, check=True)
            result['after_stats'] = repo_storage_stats(repo_root)
        if config.json:
            rprint(rich_escape(json_dumps(result)))
        elif execute:
            rprint('\n[bold]Before:[/bold]')
            print_git_stats(result['before_stats'], indent='  ')
            rprint('\n[bold]After:[/bold]')
            print_git_stats(result['after_stats'], indent='  ')
        else:
            rprint('\n[dim]Dry-run only.[/dim] Re-run with [bold]--execute[/bold] to repack.')
        return result


class GitDebloatModalCLI(scfg.ModalCLI):
    """Inspect and remove Git history bloat.

    The subcommands are exposed by assigning DataConfig classes as ModalCLI
    attributes, matching the pattern used by git-well's top-level modal CLI.
    """
    __command__ = 'git-debloat'

    stats = StatsCLI
    search = SearchCLI
    purge = PurgeCLI
    gc = GcCLI
    repack = RepackCLI


__cli__ = GitDebloatModalCLI


def main() -> None:
    """Entry point for ``git-debloat``."""
    modal = GitDebloatModalCLI(version='0.0.0')
    modal.main()


def coerce_repo_root(repo: PathLike) -> Path:
    path = Path(repo).expanduser()
    proc = subprocess.run(
        ['git', '-C', os.fspath(path), 'rev-parse', '--show-toplevel'],
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=False,
    )
    if proc.returncode != 0:
        raise RuntimeError(f'not inside a Git repository: {path}\n{proc.stderr.strip()}')
    return Path(proc.stdout.strip()).resolve()


def normalize_repo_path(path: str) -> str:
    text = path.strip()
    if not text:
        raise ValueError('empty path is not valid')
    text = text.replace('\\', '/')
    while text.startswith('./'):
        text = text[2:]
    text = text.strip('/')
    if text in {'', '.', '/'}:
        raise ValueError('refusing to target the repository root')
    if text.startswith('../') or '..' in text.split('/'):
        # The split check above is intentionally conservative and the startswith
        # catches the common case. Keep the message simple.
        raise ValueError(f'path must stay inside the repository: {path!r}')
    return text


def git_lines(repo_root: Path, args: Sequence[str], *, input_text: Optional[str] = None) -> List[str]:
    proc = subprocess.run(
        ['git', '-C', os.fspath(repo_root), *args],
        input=input_text,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=False,
    )
    if proc.returncode != 0:
        raise RuntimeError(
            'git command failed:\n  {}\n{}'.format(
                shell_join(['git', '-C', os.fspath(repo_root), *args]),
                proc.stderr.strip(),
            )
        )
    return proc.stdout.splitlines()


def git_output(repo_root: Path, args: Sequence[str], *, input_text: Optional[str] = None) -> str:
    return '\n'.join(git_lines(repo_root, args, input_text=input_text))


def list_revisions_args(refs: Optional[Sequence[str]]) -> List[str]:
    if refs:
        return list(refs)
    return ['--all']


def list_history_blobs(repo_root: Path, refs: Optional[Sequence[str]] = None) -> List[GitBlobRecord]:
    rev_args = list_revisions_args(refs)
    object_lines = git_lines(repo_root, ['rev-list', '--objects', *rev_args])
    if not object_lines:
        return []
    input_text = '\n'.join(object_lines) + '\n'
    # objectsize:disk is available in modern Git. Fall back if needed.
    fmt = '%(objectname) %(objecttype) %(objectsize) %(objectsize:disk) %(rest)'
    try:
        check_lines = git_lines(repo_root, ['cat-file', f'--batch-check={fmt}'], input_text=input_text)
        has_disk_size = True
    except RuntimeError:
        fmt = '%(objectname) %(objecttype) %(objectsize) %(rest)'
        check_lines = git_lines(repo_root, ['cat-file', f'--batch-check={fmt}'], input_text=input_text)
        has_disk_size = False

    records: List[GitBlobRecord] = []
    for line in check_lines:
        if has_disk_size:
            parts = line.split(' ', 4)
            if len(parts) < 5:
                continue
            oid, objtype, size_text, disk_text, rest = parts
            disk_size = int(disk_text) if disk_text.isdigit() else None
        else:
            parts = line.split(' ', 3)
            if len(parts) < 4:
                continue
            oid, objtype, size_text, rest = parts
            disk_size = None
        if objtype != 'blob':
            continue
        path = rest.strip()
        if not path:
            continue
        try:
            size = int(size_text)
        except ValueError:
            continue
        records.append(GitBlobRecord(oid=oid, path=path, size=size, disk_size=disk_size))
    return records


def filter_records(
    records: Iterable[GitBlobRecord],
    *,
    include: Optional[Sequence[str]] = None,
    exclude: Optional[Sequence[str]] = None,
) -> List[GitBlobRecord]:
    result = []
    for rec in records:
        if include and not any(fnmatch.fnmatch(rec.path, pat) for pat in include):
            continue
        if exclude and any(fnmatch.fnmatch(rec.path, pat) for pat in exclude):
            continue
        result.append(rec)
    return result


def path_matches_target(path: str, target: str, *, glob: bool = False) -> bool:
    if glob:
        return fnmatch.fnmatch(path, target)
    norm = target.rstrip('/')
    return path == norm or path.startswith(norm + '/')


def infer_filter_repo_path(target: str, matching_paths: Sequence[str], *, glob: bool) -> str:
    if glob:
        return target
    norm = target.rstrip('/')
    if any(p.startswith(norm + '/') for p in matching_paths):
        return norm + '/'
    return norm


def head_tracking_info(repo_root: Path, target: str, *, glob: bool = False) -> Dict[str, Any]:
    """Report whether a target is currently tracked by HEAD."""
    if glob:
        lines = git_lines(repo_root, ['ls-tree', '-r', '--name-only', 'HEAD'])
        matches = [p for p in lines if path_matches_target(p, target, glob=True)]
        return {
            'state': 'tracked' if matches else 'not_tracked',
            'type': 'glob-matches' if matches else None,
            'entries': len(matches),
            'found': bool(matches),
        }

    norm = target.rstrip('/')
    proc = subprocess.run(
        ['git', '-C', os.fspath(repo_root), 'cat-file', '-t', f'HEAD:{norm}'],
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=False,
    )
    if proc.returncode != 0:
        return {
            'state': 'not_tracked',
            'type': None,
            'entries': 0,
            'found': False,
        }
    objtype = proc.stdout.strip()
    entries = 1
    if objtype == 'tree':
        entries = len(git_lines(repo_root, ['ls-tree', '-r', '--name-only', 'HEAD', '--', norm]))
    return {
        'state': 'tracked',
        'type': objtype,
        'entries': entries,
        'found': True,
    }


def head_size_for_target(repo_root: Path, target: str, *, glob: bool = False) -> int:
    if glob:
        # Git's pathspec glob behavior is not exactly fnmatch. For consistency,
        # inspect HEAD's tree and apply the same matcher used for history report.
        lines = git_lines(repo_root, ['ls-tree', '-r', '-l', 'HEAD'])
        total = 0
        for line in lines:
            # 100644 blob <oid> <size>\t<path>
            before_tab, _, path = line.partition('\t')
            if not path_matches_target(path, target, glob=True):
                continue
            fields = before_tab.split()
            if len(fields) >= 4 and fields[3].isdigit():
                total += int(fields[3])
        return total
    lines = git_lines(repo_root, ['ls-tree', '-r', '-l', 'HEAD', '--', target])
    total = 0
    for line in lines:
        before_tab, _, _path = line.partition('\t')
        fields = before_tab.split()
        if len(fields) >= 4 and fields[3].isdigit():
            total += int(fields[3])
    return total


def touching_commits(repo_root: Path, target: str, refs: Optional[Sequence[str]], *, glob: bool = False) -> List[str]:
    if glob:
        records = list_history_blobs(repo_root, refs=refs)
        paths = sorted({r.path for r in records if path_matches_target(r.path, target, glob=True)})
        commits: set[str] = set()
        # Limit to a path set; if a glob matches thousands of paths, this is slow,
        # but purge reports are intentionally conservative and explicit.
        for path in paths:
            commits.update(git_lines(repo_root, ['rev-list', *list_revisions_args(refs), '--', path]))
        return sorted(commits)
    return git_lines(repo_root, ['rev-list', *list_revisions_args(refs), '--', target])


def commit_descendant_closure(repo_root: Path, seeds: Iterable[str], refs: Optional[Sequence[str]]) -> set[str]:
    seed_set = set(seeds)
    if not seed_set:
        return set()
    lines = git_lines(repo_root, ['rev-list', '--parents', *list_revisions_args(refs)])
    children: Dict[str, List[str]] = collections.defaultdict(list)
    all_commits: set[str] = set()
    for line in lines:
        parts = line.split()
        if not parts:
            continue
        commit = parts[0]
        all_commits.add(commit)
        for parent in parts[1:]:
            children[parent].append(commit)
    impacted = set(seed_set & all_commits)
    queue = collections.deque(impacted)
    while queue:
        current = queue.popleft()
        for child in children.get(current, []):
            if child not in impacted:
                impacted.add(child)
                queue.append(child)
    return impacted


def selected_commit_count(repo_root: Path, refs: Optional[Sequence[str]]) -> int:
    return len(git_lines(repo_root, ['rev-list', *list_revisions_args(refs)]))


def touch_summary(repo_root: Path, target: str, refs: Optional[Sequence[str]], *, glob: bool = False) -> Tuple[Optional[Dict[str, str]], Optional[Dict[str, str]]]:
    if glob:
        return None, None
    fmt = '%H%x09%cI%x09%s'
    lines = git_lines(repo_root, ['log', *list_revisions_args(refs), f'--format={fmt}', '--', target])
    entries = []
    for line in lines:
        parts = line.split('\t', 2)
        if len(parts) == 3:
            entries.append({'commit': parts[0], 'date': parts[1], 'subject': parts[2]})
    if not entries:
        return None, None
    # git log is newest first.
    return entries[-1], entries[0]


def build_purge_reports(
    repo_root: Path,
    targets: Sequence[str],
    refs: Optional[Sequence[str]],
    glob: bool,
    max_largest_blobs: int,
) -> List[TargetReport]:
    records = list_history_blobs(repo_root, refs=refs)
    selected_count = selected_commit_count(repo_root, refs)
    reports = []
    for target in targets:
        matching = [r for r in records if path_matches_target(r.path, target, glob=glob)]
        matching_paths = sorted({r.path for r in matching})
        oid_to_record: Dict[str, GitBlobRecord] = {}
        for rec in matching:
            old = oid_to_record.get(rec.oid)
            if old is None or rec.size > old.size:
                oid_to_record[rec.oid] = rec
        unique = list(oid_to_record.values())
        raw_bytes = sum(r.size for r in unique)
        disk_sizes = [r.disk_size for r in unique]
        disk_bytes = None if any(v is None for v in disk_sizes) else sum(int(v) for v in disk_sizes)
        touches = touching_commits(repo_root, target, refs, glob=glob)
        impacted = commit_descendant_closure(repo_root, touches, refs)
        first_touch, last_touch = touch_summary(repo_root, target, refs, glob=glob)
        largest = sorted(unique, key=lambda r: (r.size, r.path), reverse=True)[:max_largest_blobs]
        largest_rows = [
            {
                'oid': r.oid,
                'path': r.path,
                'raw_size': r.size,
                'raw_size_human': byte_str(r.size),
                'disk_size': r.disk_size,
                'disk_size_human': None if r.disk_size is None else byte_str(r.disk_size),
            }
            for r in largest
        ]
        head_info = head_tracking_info(repo_root, target, glob=glob)
        found_in_history = bool(matching)
        reports.append(TargetReport(
            target=target,
            match_mode='glob' if glob else 'path-prefix',
            filter_repo_path=infer_filter_repo_path(target, matching_paths, glob=glob),
            head_state=head_info['state'],
            head_type=head_info['type'],
            head_entries=int(head_info['entries']),
            found_in_head=bool(head_info['found']),
            found_in_history=found_in_history,
            rewrite_needed=found_in_history,
            num_blob_path_entries=len(matching),
            num_unique_blobs=len(unique),
            history_raw_bytes=raw_bytes,
            history_disk_bytes=disk_bytes,
            head_raw_bytes=head_size_for_target(repo_root, target, glob=glob),
            direct_touching_commits=len(set(touches)),
            descendant_impacted_commits=len(impacted),
            selected_refs_commits=selected_count,
            first_touch=first_touch,
            last_touch=last_touch,
            largest_blobs=largest_rows,
        ))
    return reports


def build_filter_repo_command(
    repo_root: Path,
    reports: Sequence[TargetReport],
    refs: Optional[Sequence[str]],
    glob: bool,
    force: bool,
) -> List[str]:
    if not reports:
        return []
    cmd = ['git', '-C', os.fspath(repo_root), 'filter-repo']
    for report in reports:
        if glob:
            cmd.extend(['--path-glob', report.filter_repo_path])
        else:
            cmd.extend(['--path', report.filter_repo_path])
    cmd.append('--invert-paths')
    if refs:
        cmd.append('--refs')
        cmd.extend(refs)
    if force:
        cmd.append('--force')
    return cmd


def summarize_path_groups(
    oid_to_record: Mapping[str, GitBlobRecord],
    oid_to_paths: Mapping[str, Sequence[str]],
    depth: int,
    limit: int,
    min_size: int,
) -> List[Dict[str, Any]]:
    # Group every unique blob under every path prefix it has been seen at. This
    # can intentionally over-count renamed duplicate paths, but is useful for
    # discovering candidate locations to purge.
    groups: Dict[str, Dict[str, Any]] = collections.defaultdict(lambda: {
        'raw_size': 0,
        'disk_size': 0,
        'disk_size_known': True,
        'unique_blobs': set(),
        'path_occurrences': 0,
    })
    for oid, rec in oid_to_record.items():
        if rec.size < min_size:
            continue
        for path in sorted(set(oid_to_paths.get(oid, []))):
            prefix = path_prefix(path, depth)
            group = groups[prefix]
            if oid not in group['unique_blobs']:
                group['unique_blobs'].add(oid)
                group['raw_size'] += rec.size
                if rec.disk_size is None:
                    group['disk_size_known'] = False
                else:
                    group['disk_size'] += rec.disk_size
            group['path_occurrences'] += 1
    rows = []
    for prefix, group in groups.items():
        disk_size = group['disk_size'] if group['disk_size_known'] else None
        rows.append({
            'path_prefix': prefix,
            'raw_size': group['raw_size'],
            'raw_size_human': byte_str(group['raw_size']),
            'disk_size': disk_size,
            'disk_size_human': None if disk_size is None else byte_str(disk_size),
            'unique_blobs': len(group['unique_blobs']),
            'path_occurrences': group['path_occurrences'],
        })
    rows.sort(key=lambda r: (r['raw_size'], r['path_prefix']), reverse=True)
    return rows[:limit]


def path_prefix(path: str, depth: int) -> str:
    parts = path.split('/')
    if len(parts) <= depth:
        return path
    return '/'.join(parts[:depth]) + '/...'


def repo_storage_stats(repo_root: Path) -> Dict[str, str]:
    lines = git_lines(repo_root, ['count-objects', '-vH'])
    data: Dict[str, str] = {}
    for line in lines:
        key, _, value = line.partition(':')
        data[key.strip()] = value.strip()
    try:
        data['head'] = git_output(repo_root, ['rev-parse', 'HEAD']).strip()
    except RuntimeError:
        pass
    return data


def assert_clean_worktree(repo_root: Path) -> None:
    # Only tracked modifications block a history rewrite. Large repos often have
    # intentionally untracked/generated files, and git-filter-repo can safely
    # leave those alone. They are reported separately as a warning.
    status = git_output(repo_root, ['status', '--porcelain', '--untracked-files=no'])
    if status.strip():
        rprint('[bold red]Refusing to rewrite history with modified tracked files.[/bold red]')
        rprint('[dim]Commit/stash first, or pass --allow-dirty if you know what you are doing.[/dim]')
        for line in status.splitlines():
            rprint('  ' + format_status_line(line, repo_root))
        raise RuntimeError('modified tracked files are present')


def warn_untracked_worktree(repo_root: Path) -> None:
    status = git_output(repo_root, ['status', '--porcelain', '--untracked-files=all'])
    untracked = [line for line in status.splitlines() if line.startswith('?? ')]
    if untracked:
        shown = untracked[:20]
        rprint('\n[yellow]Note:[/yellow] untracked files are present but will not block the rewrite:')
        for line in shown:
            rprint('  ' + format_status_line(line, repo_root))
        if len(untracked) > len(shown):
            rprint(f'  [dim]... {len(untracked) - len(shown)} more untracked entries[/dim]')


def ensure_filter_repo_available() -> None:
    if shutil.which('git-filter-repo'):
        return
    proc = subprocess.run(
        ['git', 'filter-repo', '--help'],
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
        check=False,
    )
    if proc.returncode == 0:
        return
    raise RuntimeError(
        'git-filter-repo is required for purge --execute. Install it, then retry. '
        'For example: python -m pip install git-filter-repo'
    )


def create_backup_bundle(repo_root: Path) -> Path:
    backup_dpath = repo_root / '.git' / 'git-debloat-backups'
    backup_dpath.mkdir(parents=True, exist_ok=True)
    timestamp = time.strftime('%Y%m%dT%H%M%S')
    short = git_output(repo_root, ['rev-parse', '--short=12', 'HEAD']).strip()
    bundle_fpath = backup_dpath / f'before-debloat-{timestamp}-{short}.bundle'
    subprocess.run(
        ['git', '-C', os.fspath(repo_root), 'bundle', 'create', os.fspath(bundle_fpath), '--all'],
        check=True,
    )
    return bundle_fpath



def snapshot_remote_config(repo_root: Path) -> Dict[str, Any]:
    """Capture remote/upstream config before git-filter-repo removes it."""
    entries = []
    for pattern in ['^remote\\.', '^branch\\..*\\.remote$', '^branch\\..*\\.merge$']:
        proc = subprocess.run(
            ['git', '-C', os.fspath(repo_root), 'config', '--local', '--get-regexp', pattern],
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            check=False,
        )
        if proc.returncode not in {0, 1}:
            raise RuntimeError(f'failed to inspect git config pattern {pattern!r}: {proc.stderr.strip()}')
        for line in proc.stdout.splitlines():
            key, sep, value = line.partition(' ')
            if sep:
                entries.append({'key': key, 'value': value})

    remote_names = git_lines_allow_error(repo_root, ['remote'])
    remote_verbose = git_lines_allow_error(repo_root, ['remote', '-v'])
    current_branch = git_output_allow_error(repo_root, ['branch', '--show-current']).strip()
    upstream = git_output_allow_error(repo_root, ['rev-parse', '--abbrev-ref', '--symbolic-full-name', '@{u}']).strip()
    return {
        'entries': entries,
        'remote_names': remote_names,
        'remote_verbose': remote_verbose,
        'current_branch': current_branch or None,
        'upstream': upstream or None,
    }


def write_remote_snapshot(repo_root: Path, snapshot: Mapping[str, Any]) -> Path:
    backup_dpath = repo_root / '.git' / 'git-debloat-backups'
    backup_dpath.mkdir(parents=True, exist_ok=True)
    timestamp = time.strftime('%Y%m%dT%H%M%S')
    short = git_output_allow_error(repo_root, ['rev-parse', '--short=12', 'HEAD']).strip() or 'unknown'
    fpath = backup_dpath / f'remotes-before-debloat-{timestamp}-{short}.json'
    fpath.write_text(json_dumps(snapshot) + '\n')
    rprint(f'[dim]Saved remote configuration snapshot:[/dim] {format_path(fpath)}')
    return fpath


def restore_remote_config(repo_root: Path, snapshot: Mapping[str, Any]) -> None:
    """Restore remote/upstream config entries captured before filter-repo."""
    entries = list(snapshot.get('entries') or [])
    grouped: Dict[str, List[str]] = collections.defaultdict(list)
    for entry in entries:
        key = str(entry['key'])
        value = str(entry['value'])
        grouped[key].append(value)

    for key, values in sorted(grouped.items()):
        subprocess.run(
            ['git', '-C', os.fspath(repo_root), 'config', '--local', '--unset-all', key],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
            check=False,
        )
        for value in values:
            subprocess.run(
                ['git', '-C', os.fspath(repo_root), 'config', '--local', '--add', key, value],
                check=True,
            )


def fetch_restored_remotes(repo_root: Path, snapshot: Mapping[str, Any]) -> List[Dict[str, Any]]:
    """Fetch remotes after restoring config, but do not fail the rewrite if fetch fails."""
    results = []
    remote_names = list(snapshot.get('remote_names') or [])
    for remote in remote_names:
        proc = subprocess.run(
            ['git', '-C', os.fspath(repo_root), 'fetch', remote, '--prune'],
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            check=False,
        )
        results.append({
            'remote': remote,
            'returncode': proc.returncode,
            'stdout': proc.stdout,
            'stderr': proc.stderr,
        })
    return results


def print_fetch_results(results: Sequence[Mapping[str, Any]]) -> None:
    if not results:
        rprint('[yellow]No restored remotes to fetch.[/yellow]')
        return
    rprint('\n[bold]Fetch restored remotes:[/bold]')
    for row in results:
        remote = rich_escape(str(row.get('remote')))
        rc = int(row.get('returncode') or 0)
        if rc == 0:
            rprint(f'  [green]ok[/green] {remote}')
        else:
            rprint(f'  [yellow]failed[/yellow] {remote} [dim](exit {rc})[/dim]')
            stderr = str(row.get('stderr') or '').strip()
            if stderr:
                rprint('    [dim]' + rich_escape(stderr.splitlines()[-1]) + '[/dim]')


def print_remote_summary(repo_root: Path) -> None:
    lines = git_lines_allow_error(repo_root, ['remote', '-v'])
    if not lines:
        rprint('[yellow]No remotes are configured after restore.[/yellow]')
        return
    rprint('[bold]Restored remotes:[/bold]')
    for line in lines:
        rprint('  ' + rich_escape(line))


def git_lines_allow_error(repo_root: Path, args: Sequence[str]) -> List[str]:
    proc = subprocess.run(
        ['git', '-C', os.fspath(repo_root), *args],
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=False,
    )
    if proc.returncode != 0:
        return []
    return proc.stdout.splitlines()


def git_output_allow_error(repo_root: Path, args: Sequence[str]) -> str:
    return '\n'.join(git_lines_allow_error(repo_root, args))


def print_rewrite_danger_banner(
    *,
    repo_root: Path,
    reports: Sequence[TargetReport],
    filter_cmd: Sequence[str],
    remote_snapshot: Optional[Mapping[str, Any]],
    will_restore_remotes: bool,
    will_fetch_restored_remotes: bool,
) -> None:
    impacted = sum(r.descendant_impacted_commits for r in reports)
    raw_bytes = sum(r.history_raw_bytes for r in reports)
    targets = ', '.join(r.target for r in reports)
    remote_lines = []
    if remote_snapshot is not None:
        names = list(remote_snapshot.get('remote_names') or [])
        upstream = remote_snapshot.get('upstream')
        remote_lines.append(f'Remotes snapshotted: {", ".join(names) if names else "(none)"}')
        remote_lines.append(f'Current upstream: {upstream or "(none)"}')
        if will_restore_remotes:
            remote_lines.append('After rewrite: remote config will be restored automatically.')
            if will_fetch_restored_remotes:
                remote_lines.append('After restore: restored remotes will be fetched so divergence is visible.')
            else:
                remote_lines.append('After restore: fetch is disabled by --no-fetch-restored-remotes.')
    else:
        remote_lines.append('Remote restore is disabled; git-filter-repo may remove all remotes.')

    remote_text = '\n'.join('    ' + line for line in remote_lines)
    text = (
        'DANGEROUS HISTORY REWRITE\n\n'
        f'Repository: {repo_root}\n'
        f'Targets: {targets}\n'
        f'Matching history payload: {byte_str(raw_bytes)} raw\n'
        f'Descendant commits estimated to receive new IDs: {impacted}\n\n'
        'This rewrites commit IDs. Any pushed branches that contain these commits will\n'
        'diverge from their remote copies. Collaborators must reclone or hard-reset.\n'
        'Publishing the result usually requires: git push --force-with-lease\n\n'
        f'{remote_text}\n\n'
        'Command:\n'
        f'  {shell_join(filter_cmd)}'
    )
    rprint('\n' + Panel(rich_escape(text), title='[bold red]git-debloat purge --execute[/bold red]', border_style='red'))

def parse_size(text: Any) -> int:
    if isinstance(text, int):
        return text
    raw = str(text).strip()
    match = re.match(r'^([0-9]+(?:\.[0-9]+)?)\s*([A-Za-z]*)$', raw)
    if not match:
        raise ValueError(f'cannot parse size: {text!r}')
    number = float(match.group(1))
    unit = match.group(2).lower()
    factors = {
        '': 1,
        'b': 1,
        'k': 1000,
        'kb': 1000,
        'm': 1000 ** 2,
        'mb': 1000 ** 2,
        'g': 1000 ** 3,
        'gb': 1000 ** 3,
        't': 1000 ** 4,
        'tb': 1000 ** 4,
        'kib': 1024,
        'ki': 1024,
        'mib': 1024 ** 2,
        'mi': 1024 ** 2,
        'gib': 1024 ** 3,
        'gi': 1024 ** 3,
        'tib': 1024 ** 4,
        'ti': 1024 ** 4,
    }
    if unit not in factors:
        raise ValueError(f'unknown size unit in {text!r}')
    return int(number * factors[unit])


def byte_str(num: Optional[int], precision: int = 2) -> str:
    if num is None:
        return 'unknown'
    value = float(num)
    units = ['B', 'KB', 'MB', 'GB', 'TB']
    unit = units[0]
    for unit in units:
        if abs(value) < 1024 or unit == units[-1]:
            break
        value /= 1024.0
    return f'{value:.{precision}f} {unit}'



def file_uri(path: PathLike) -> str:
    """Return a file:// URI suitable for Rich terminal links."""
    return 'file://' + urllib.parse.quote(os.fspath(Path(path).expanduser().resolve()))


def format_path(path: PathLike, *, label: Optional[str] = None) -> str:
    """Format a filesystem path as a Rich clickable link."""
    p = Path(path).expanduser()
    text = os.fspath(label if label is not None else p)
    try:
        uri = file_uri(p)
    except Exception:
        return rich_escape(text)
    return f'[link={uri}]{rich_escape(text)}[/link]'


def format_repo_path(repo_root: Path, relpath: str, *, suffix: str = '') -> str:
    """Format a repo-relative path with a Rich clickable file link."""
    rel = relpath.rstrip('/')
    link_rel = rel
    if rel.endswith('/...'):
        link_rel = rel[:-4].rstrip('/')
    abs_path = repo_root / link_rel if link_rel else repo_root
    display = relpath + suffix
    try:
        uri = file_uri(abs_path)
    except Exception:
        return rich_escape(display)
    return f'[link={uri}]{rich_escape(display)}[/link]'


def format_status_line(line: str, repo_root: Path) -> str:
    """Pretty-print a porcelain status line with linked paths when possible."""
    if len(line) >= 4 and line[2] == ' ':
        code = rich_escape(line[:2])
        path_text = line[3:]
        return f'{code} {format_repo_path(repo_root, path_text)}'
    return rich_escape(line)


def format_command(cmd: Sequence[str]) -> str:
    """Format a shell command in a subdued style for Rich output."""
    return '[dim]' + rich_escape(shell_join(cmd)) + '[/dim]'


def format_commit(commit: str) -> str:
    return f'[cyan]{rich_escape(commit[:12])}[/cyan]'


def print_git_stats(stats: Mapping[str, Any], indent: str = '') -> None:
    for key in sorted(stats):
        value = stats[key]
        if key == 'head':
            value_text = format_commit(str(value))
        else:
            value_text = rich_escape(str(value))
        rprint(f'{indent}[bold]{rich_escape(str(key))}:[/bold] {value_text}')


def print_search_report(result: Mapping[str, Any]) -> None:
    repo_root = Path(os.fspath(result['repo_root']))
    rprint(f"[bold]Repo:[/bold] {format_path(repo_root)}")
    rprint(f"[bold]Reachable blob path entries:[/bold] {result['num_blob_path_entries']}")
    rprint(f"[bold]Unique reachable blobs:[/bold] {result['num_unique_blobs']}")
    rprint(f"[bold]Minimum shown size:[/bold] {rich_escape(str(result['min_size_human']))}")
    groups = result.get('top_path_groups') or []
    if groups:
        rprint('\n[bold]Largest path-prefix groups[/bold] [dim](unique blobs above min-size):[/dim]')
        for row in groups:
            rprint(
                f"  [green]{row['raw_size_human']:>10}[/green] raw  "
                f"[blue]{row['disk_size_human'] or 'unknown':>10}[/blue] disk  "
                f"{row['unique_blobs']:>5} blobs  "
                f"{format_repo_path(repo_root, row['path_prefix'])}"
            )
    rprint('\n[bold]Largest unique blobs:[/bold]')
    for row in result['top_blobs']:
        path_links = [format_repo_path(repo_root, p) for p in row['paths']]
        paths = ', '.join(path_links)
        if row['paths_truncated']:
            paths += ', [dim]...[/dim]'
        rprint(
            f"  [green]{row['raw_size_human']:>10}[/green] raw  "
            f"[blue]{row['disk_size_human'] or 'unknown':>10}[/blue] disk  "
            f"{format_commit(row['oid'])}  {paths}"
        )


def print_purge_report(result: Mapping[str, Any]) -> None:
    repo_root = Path(os.fspath(result['repo_root']))
    rprint(f"[bold]Repo:[/bold] {format_path(repo_root)}")
    rprint('\n[bold]Current object stats:[/bold]')
    print_git_stats(result['before_stats'], indent='  ')
    for target in result['targets']:
        disk = target['history_disk_bytes']
        target_label = target['target']
        rprint(f"\n[bold]Target:[/bold] {format_repo_path(repo_root, target_label)}")
        rprint(f"  [bold]match mode:[/bold] {rich_escape(str(target['match_mode']))}")
        rprint(f"  [bold]HEAD tracking:[/bold] {format_head_tracking(target)}")
        if not target['found_in_history']:
            if target['found_in_head']:
                rprint('  [bold]history status:[/bold] [yellow]no matching blobs in selected refs; check --refs if this is unexpected[/yellow]')
            else:
                rprint('  [bold]history status:[/bold] [yellow]not found in HEAD or selected reachable history[/yellow]')
        else:
            if not target['found_in_head']:
                rprint('  [bold]history status:[/bold] [yellow]found in history, but not currently tracked in HEAD[/yellow]')
            else:
                rprint('  [bold]history status:[/bold] [green]found in selected reachable history[/green]')
        if target['rewrite_needed']:
            rprint(f"  [bold]filter-repo path:[/bold] {rich_escape(str(target['filter_repo_path']))}")
        else:
            rprint('  [bold]filter-repo path:[/bold] [dim](none; target would be a no-op)[/dim]')
        rprint(f"  [bold]current HEAD size:[/bold] {byte_str(target['head_raw_bytes'])}")
        rprint(f"  [bold]history unique blob size:[/bold] {byte_str(target['history_raw_bytes'])} raw")
        rprint(f"  [bold]history unique blob disk size:[/bold] {byte_str(disk) if disk is not None else 'unknown'}")
        rprint(f"  [bold]matching blob path entries:[/bold] {target['num_blob_path_entries']}")
        rprint(f"  [bold]matching unique blobs:[/bold] {target['num_unique_blobs']}")
        rprint(f"  [bold]direct touching commits:[/bold] {target['direct_touching_commits']}")
        rprint(f"  [bold]descendant impacted commits estimate:[/bold] {target['descendant_impacted_commits']} / {target['selected_refs_commits']}")
        if target.get('first_touch'):
            first = target['first_touch']
            rprint(f"  [bold]first touch:[/bold] {format_commit(first['commit'])} {rich_escape(first['date'])} {rich_escape(first['subject'])}")
        if target.get('last_touch'):
            last = target['last_touch']
            rprint(f"  [bold]last touch:[/bold]  {format_commit(last['commit'])} {rich_escape(last['date'])} {rich_escape(last['subject'])}")
        if target['largest_blobs']:
            rprint('  [bold]largest matching blobs:[/bold]')
            for blob in target['largest_blobs']:
                rprint(
                    f"    [green]{blob['raw_size_human']:>10}[/green] raw  "
                    f"[blue]{blob['disk_size_human'] or 'unknown':>10}[/blue] disk  "
                    f"{format_commit(blob['oid'])}  {format_repo_path(repo_root, blob['path'])}"
                )
    if result.get('rewrite_needed') and result.get('filter_repo_command'):
        rprint('\n[bold]Rewrite command:[/bold]')
        rprint('  ' + format_command(result['filter_repo_command']))
    else:
        rprint('\n[bold]Rewrite command:[/bold] [dim](none; all requested targets are absent from selected history)[/dim]')


def format_head_tracking(target: Mapping[str, Any]) -> str:
    if not target.get('found_in_head'):
        return 'not tracked in HEAD'
    typ = target.get('head_type') or 'unknown'
    entries = int(target.get('head_entries') or 0)
    if typ == 'tree':
        return f'tracked directory ({entries} file entries in HEAD)'
    if typ == 'blob':
        return 'tracked file in HEAD'
    if typ == 'glob-matches':
        return f'tracked by glob ({entries} matching file entries in HEAD)'
    return f'tracked in HEAD ({typ})'


def shell_join(cmd: Sequence[str]) -> str:
    return ' '.join(shlex.quote(os.fspath(c)) for c in cmd)


def json_dumps(data: Any) -> str:
    return json.dumps(data, indent=2, sort_keys=True)


if __name__ == '__main__':
    try:
        main()
    except RuntimeError as ex:
        rprint(f'[bold red]ERROR:[/bold red] {rich_escape(str(ex))}')
        raise SystemExit(1) from None
