"""Read character dates from catalog history and generation receipts."""
from __future__ import annotations

import datetime as dt
import json
from pathlib import Path
import re
import subprocess


_TOKEN = re.compile(r'//[^\n]*|/\*.*?\*/|"(?:\\.|[^"\\])*"|[A-Za-z_][A-Za-z_0-9]*|[^\s]', re.S)


def catalog_ids(text: str) -> set[str]:
    """Read keys in the characters map. Ignore comments and nested maps."""
    tokens = [m.group() for m in _TOKEN.finditer(text) if not m.group().startswith(('//', '/*'))]
    found = set()
    for at in range(len(tokens) - 2):
        if tokens[at:at + 3] != ['characters', ':', '{']:
            continue
        depth = 1
        for index in range(at + 3, len(tokens)):
            token = tokens[index]
            if depth == 1 and token.startswith('"') and tokens[index + 1:index + 2] == [':']:
                found.add(json.loads(token))
            if token in ('{', '(', '['):
                depth += 1
            elif token in ('}', ')', ']'):
                depth -= 1
            if not depth:
                break
    return found


def first_added(path: Path) -> dict[str, str]:
    """Use the oldest catalog occurrence in available Git history, across renames."""
    try:
        log = subprocess.run(
            ['git', '-C', str(path.parent), 'log', '--follow', '--format=%x1e%H%x00%aI',
             '--name-only', '--no-ext-diff', '--no-textconv', '--', path.name],
            capture_output=True, text=True, timeout=30, check=True)
        revisions = []
        for record in log.stdout.split('\x1e')[1:]:
            header, *names = record.strip().splitlines()
            commit, date = header.split('\0')
            names = [name for name in names if name]
            if names:
                revisions.append((commit + ':' + names[0], date))
        if not revisions:
            return {}
        blobs = subprocess.run(
            ['git', '-C', str(path.parent), 'cat-file', '--batch'],
            input=('\n'.join(rev for rev, _ in revisions) + '\n').encode(),
            capture_output=True, timeout=30, check=True).stdout
    except (OSError, ValueError, subprocess.SubprocessError):
        return {}
    dates = {}
    offset = 0
    for _, date in revisions:
        end = blobs.index(b'\n', offset)
        header = blobs[offset:end].split()
        offset = end + 1
        if len(header) != 3 or header[1] != b'blob':
            continue
        size = int(header[2])
        text = blobs[offset:offset + size].decode('utf-8', errors='replace')
        offset += size + 1
        for key in catalog_ids(text):
            dates[key] = date
    return dates


def timestamp(value) -> float | None:
    try:
        date = dt.datetime.fromisoformat(value)
        return date.timestamp() if date.tzinfo else None
    except (ValueError, TypeError):
        return None


def attach(report: dict, prepared: dict, repo: Path) -> None:
    """Attach dates for the browser. Status queries do not read Git history."""
    added = {}
    pack = Path(prepared['pack_root'])
    for source in prepared.get('sources', []):
        if source.get('schema') == 'character_catalog':
            for key, date in first_added(pack / source['path']).items():
                if key not in added or timestamp(date) < timestamp(added[key]):
                    added[key] = date
    for actor in report['identities'].values():
        receipts = [(report['units'][uid].get('receipt') or {}) for uid in actor['units']]
        dates = [r['written_at'] for r in receipts if r.get('basis') == 'rendered' and timestamp(r.get('written_at')) is not None]
        actor['added_at'] = added.get(actor['id'])
        actor['generated_at'] = max(dates, key=timestamp) if dates else None
        actor['generated_at_evidence'] = 'render-receipt' if dates else 'unknown'
        if not dates:
            modified = []
            for uid in actor['units']:
                for relative in report['units'][uid].get('required_paths', {}).values():
                    path = Path(relative)
                    if path.suffix.lower() not in ('.png', '.webp', '.svg', '.jpg', '.jpeg'):
                        continue
                    try:
                        modified.append((repo / path).stat().st_mtime)
                    except OSError:
                        continue
            if modified:
                actor['generated_at'] = dt.datetime.fromtimestamp(max(modified), dt.timezone.utc).isoformat()
                actor['generated_at_evidence'] = 'file-mtime'
