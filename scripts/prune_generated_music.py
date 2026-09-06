#!/usr/bin/env python3
"""
Delete superseded renders from the music submodule.

``tools/ambition_music_renderer/generated/<track>/`` keeps every render ever
made under ``.versioned/<hash>/``, with a ``latest`` symlink pointing at the
current one. Nothing prunes the rest, and a single render can be ~400 MB.

The rule is one line: **if ``latest`` does not point at it and it is older than
``--days``, it goes.** Matching ``agent/<track>_<hash>_bundle*`` files go with
it. Everything here is gitignored, and re-rendering a score reproduces it.

Dry run by default; pass ``--apply`` to delete.
"""

from __future__ import annotations

import argparse
import os
import re
import shutil
import time
from pathlib import Path

RENDERER = Path('tools/ambition_music_renderer')
DEFAULT_DAYS = 14


def dir_size(path: Path) -> int:
    if not path.is_dir() or path.is_symlink():
        return path.lstat().st_size
    total = 0
    for root, _dirs, files in os.walk(path, onerror=lambda _e: None):
        for name in files:
            try:
                total += os.lstat(os.path.join(root, name)).st_size
            except OSError:
                pass
    return total


def byte_str(num: int) -> str:
    value = float(num)
    for unit in ('B', 'KB', 'MB', 'GB', 'TB'):
        if abs(value) < 1024.0 or unit == 'TB':
            return f'{int(value)} B' if unit == 'B' else f'{value:.1f} {unit}'
        value /= 1024.0
    raise AssertionError('unreachable')


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter
    )
    parser.add_argument('--days', type=float, default=DEFAULT_DAYS,
                        help=f'only delete renders older than this (default: {DEFAULT_DAYS})')
    parser.add_argument('--apply', action='store_true',
                        help='actually delete; without it this only reports')
    parser.add_argument('--repo', type=Path, default=Path(__file__).resolve().parent.parent)
    args = parser.parse_args(argv)

    root = args.repo.resolve() / RENDERER
    generated = root / 'generated'
    if not generated.is_dir():
        raise SystemExit(f'no renders under {generated}; is the submodule initialized?')

    now = time.time()
    keep: set[str] = set()
    stale: list[Path] = []

    for track in sorted(p for p in generated.glob('*') if p.is_dir()):
        link = track / 'latest'
        current = Path(os.readlink(link)).name if link.is_symlink() else None
        if current:
            keep.add(current)
        else:
            # No `latest`: half-rendered or interrupted. Do not guess which
            # version is live — skip the whole track.
            continue
        for version in sorted(p for p in (track / '.versioned').glob('*') if p.is_dir()):
            if version.name != current and (now - version.lstat().st_mtime) / 86400.0 >= args.days:
                stale.append(version)

    # Bundles are `<track>_<hash>_bundle[.zip|_report.zip]`. `keep` holds every
    # track's live hash, so a bundle is only dropped when no track still points
    # at its render.
    agent = root / 'agent'
    if agent.is_dir():
        for entry in sorted(agent.glob('*')):
            match = re.match(r'^.+_(?P<hash>[0-9a-f]{8,})_bundle', entry.name)
            if (match and match.group('hash') not in keep
                    and (now - entry.lstat().st_mtime) / 86400.0 >= args.days):
                stale.append(entry)

    if not stale:
        print(f'nothing older than {args.days:g}d to prune')
        return 0

    sized = sorted(((p, dir_size(p)) for p in stale), key=lambda pair: pair[1], reverse=True)
    total = sum(size for _p, size in sized)
    print(f'{"removing" if args.apply else "would remove"} {len(sized)} item(s), {byte_str(total)}:\n')
    for path, size in sized:
        print(f'  {byte_str(size):>9}  {path.relative_to(args.repo.resolve())}')

    if not args.apply:
        print(f'\nDRY RUN — re-run with --apply to free {byte_str(total)}.')
        return 0

    freed = 0
    for path, size in sized:
        try:
            shutil.rmtree(path) if path.is_dir() and not path.is_symlink() else path.unlink()
            freed += size
        except OSError as ex:
            print(f'  failed: {path}: {ex}')
    print(f'\nfreed {byte_str(freed)}')
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
