#!/usr/bin/env python3
"""
Delete old music renders from the music submodule.

``tools/ambition_music_renderer/generated/<cue>/`` keeps every render ever made
under ``.versioned/<hash>/``, with a ``latest`` symlink that points at the
current one. ``agent/`` keeps a full copy of each render that was bundled for a
reviewer. Nothing else prunes them, and one render can be ~800 MB.

An item goes when it is older than ``--days`` and it is one of these:

``superseded``
    A render that ``latest`` does not point at.
``bundle``
    ``agent/<cue>_<hash>_bundle`` and its zip files. A bundle is a copy of a
    render. The current render of each cue stays in ``generated/``, thus the
    copy has no more use.
``stems``
    The raw group stems (``scratch_stems/``) in a render of a cue that has no
    score. They are most of the size of a render. The mastered audio stays,
    because a cue with no score cannot be rendered again.

Two options delete more, and are off by default:

``--stems``
    Also delete the raw group stems of the current render of each cue that has
    a score. The stem lab and the stem audits read them. To get them back,
    render the cue again.
``--orphans``
    Delete all renders of a cue that has no score, the mastered audio included.
    This cannot be undone.

Everything here is gitignored. Do not use ``--days 0`` while a render runs.

Dry run by default; pass ``--apply`` to delete.
"""

from __future__ import annotations

import argparse
import fnmatch
import os
import re
import shutil
import time
from pathlib import Path

RENDERER = Path('tools/ambition_music_renderer')
DEFAULT_DAYS = 2
SCORE_SUFFIXES = ('.music.yaml', '.yaml', '.yml')
BUNDLE_NAME = re.compile(r'^(?P<cue>.+)_(?P<hash>[0-9a-f]{8,})_bundle')


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


def scored_cues(root: Path) -> set[str]:
    """The cue ids that have a score, by the file name rule of the renderer."""
    cues: set[str] = set()
    for path in (root / 'scores').rglob('*'):
        for suffix in SCORE_SUFFIXES:
            if path.name.endswith(suffix):
                cues.add(path.name[:-len(suffix)])
                break
    return cues


def pointed_version(track: Path, name: str) -> str | None:
    link = track / name
    if link.is_symlink():
        return Path(os.readlink(link)).name
    pointer = track / f'{name}.path'
    if pointer.is_file():
        return Path(pointer.read_text(encoding='utf8').strip()).name
    return None


def plan(root: Path, *, days: float = DEFAULT_DAYS, now: float | None = None,
         cues: list[str] | None = None, stems: bool = False,
         orphans: bool = False) -> list[tuple[str, Path]]:
    """The ``(kind, path)`` items to delete under the renderer ``root``."""
    now = time.time() if now is None else now

    def old(path: Path) -> bool:
        return (now - path.lstat().st_mtime) / 86400.0 >= days

    def wanted(cue: str) -> bool:
        return not cues or any(fnmatch.fnmatchcase(cue, pattern) for pattern in cues)

    scored = scored_cues(root)
    stale: list[tuple[str, Path]] = []
    # The hash of a current render that is not on disk. A bundle of it is the
    # only copy of that render.
    lost: set[str] = set()

    generated = root / 'generated'
    for track in sorted(p for p in generated.glob('*') if p.is_dir()):
        if not wanted(track.name):
            continue
        has_score = track.name in scored
        current = pointed_version(track, 'latest')
        if current and not (track / '.versioned' / current).is_dir():
            lost.add(current)
        if orphans and not has_score:
            # The cue directory changes each time `latest` moves. The renders
            # in it give its age.
            runs = list(track.glob('.versioned/*')) or [
                p for p in track.iterdir() if not p.is_symlink()
            ]
            if all(old(p) for p in runs):
                stale.append(('orphan', track))
            continue
        versions = sorted(p for p in (track / '.versioned').glob('*') if p.is_dir())
        if versions and current is None:
            # No `latest`: half-rendered or interrupted. Do not guess which
            # version is live — keep them all.
            versions = []
        keep = {current, pointed_version(track, 'building')}
        doomed: list[Path] = []
        for version in versions:
            if version.name not in keep and old(version):
                doomed.append(version)
                stale.append(('superseded', version))
        if has_score and not stems:
            continue
        if has_score:
            banks = [track / '.versioned' / current / 'scratch_stems'] if current else []
        else:
            banks = [p for p in track.rglob('scratch_stems')
                     if not any(d in p.parents for d in doomed)]
        stale.extend(('stems', p) for p in sorted(banks) if p.is_dir() and old(p.parent))

    agent = root / 'agent'
    if agent.is_dir():
        for entry in sorted(agent.glob('*')):
            match = BUNDLE_NAME.match(entry.name)
            if (match and wanted(match.group('cue')) and match.group('hash') not in lost
                    and old(entry)):
                stale.append(('bundle', entry))
    return stale


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter
    )
    parser.add_argument('--days', type=float, default=DEFAULT_DAYS,
                        help=f'only delete items older than this (default: {DEFAULT_DAYS})')
    parser.add_argument('--cue', action='append', default=[], metavar='GLOB',
                        help='only look at cues whose id matches; give it more than once for more cues')
    parser.add_argument('--stems', action='store_true',
                        help='also delete the raw stems of the current render of each cue that has a score')
    parser.add_argument('--orphans', action='store_true',
                        help='delete all renders of a cue that has no score; this cannot be undone')
    parser.add_argument('--apply', action='store_true',
                        help='actually delete; without it this only reports')
    parser.add_argument('--repo', type=Path, default=Path(__file__).resolve().parent.parent)
    args = parser.parse_args(argv)

    repo = args.repo.resolve()
    root = repo / RENDERER
    if not (root / 'generated').is_dir():
        raise SystemExit(f'no renders under {root / "generated"}; is the submodule initialized?')

    stale = plan(root, days=args.days, cues=args.cue, stems=args.stems, orphans=args.orphans)
    if not stale:
        print(f'nothing older than {args.days:g}d to prune')
        return 0

    sized = sorted(((kind, p, dir_size(p)) for kind, p in stale), key=lambda row: row[2], reverse=True)
    total = sum(size for _kind, _p, size in sized)
    print(f'{"removing" if args.apply else "would remove"} {len(sized)} item(s), {byte_str(total)}:\n')
    for kind, path, size in sized:
        print(f'  {byte_str(size):>9}  {kind:<10}  {path.relative_to(repo)}')
    print()
    for kind in sorted({kind for kind, _p, _size in sized}):
        rows = [size for row_kind, _p, size in sized if row_kind == kind]
        print(f'  {byte_str(sum(rows)):>9}  {kind}: {len(rows)} item(s)')

    if not args.apply:
        print(f'\nDRY RUN — re-run with --apply to free {byte_str(total)}.')
        return 0

    freed = 0
    for _kind, path, size in sized:
        try:
            shutil.rmtree(path) if path.is_dir() and not path.is_symlink() else path.unlink()
            freed += size
        except OSError as ex:
            print(f'  failed: {path}: {ex}')
    print(f'\nfreed {byte_str(freed)}')
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
