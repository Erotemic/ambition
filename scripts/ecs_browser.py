#!/usr/bin/env python3
"""Create an offline HTML view of Ambition's generated Bevy ECS inventory.

The source of truth is the existing .agent/ecs_inventory index. This tool only
adds derived navigation links and statistics; it does not inspect runtime ECS.
"""

from __future__ import annotations

import argparse
import collections
import json
import re
import subprocess
import sys
import webbrowser
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
INVENTORY = ROOT / '.agent' / 'ecs_inventory'
INDEX = ROOT / '.agent' / 'index'
WEB = ROOT / 'tools' / 'ecs_architecture_browser' / 'web'
DEFAULT_OUTPUT = ROOT / '.agent' / 'ecs_browser' / 'index.html'

REFERENCE_RE = re.compile(r"&\s*(?:'[A-Za-z_][\w]*\s+)?(?:(mut)\s+)?([A-Za-z_]\w*(?:::[A-Za-z_]\w*)*)")
FILTER_RE = re.compile(r"\b(With|Without|Has|Added|Changed)\s*<\s*([A-Za-z_]\w*(?:::[A-Za-z_]\w*)*)")
SKIP_TOKENS = {'Entity', 'T', 'B', 'Self', 'dyn', 'str', 'u8', 'u16', 'u32', 'usize', 'f32', 'f64', 'bool'}
TYPE_KINDS = {'Component', 'Bundle', 'Resource', 'Message'}


def _load_json(path: Path) -> dict:
    try:
        return json.loads(path.read_text(encoding='utf-8'))
    except FileNotFoundError as exc:
        raise ValueError(f'Missing generated inventory: {path.relative_to(ROOT)}. '
                         'Run: python scripts/generate_agent_index.py') from exc


def _repo_file(path: str) -> Path | None:
    """Do not allow a generated index to read outside the repository."""
    candidate = (ROOT / path).resolve()
    if candidate.is_relative_to(ROOT.resolve()) and candidate.is_file():
        return candidate
    return None


def _excerpt(path: str, line: int, cache: dict[str, list[str]]) -> dict:
    file = _repo_file(path)
    if file is None or not isinstance(line, int) or line < 1:
        return {'start': 0, 'text': ''}
    if path not in cache:
        cache[path] = file.read_text(encoding='utf-8', errors='replace').splitlines()
    lines = cache[path]
    start = max(1, line - 2)
    end = min(len(lines), line + 7)
    return {'start': start, 'text': '\n'.join(lines[start - 1:end])[:1600]}


def _ref(crate: str, item: dict, prefix: str, seq: int = 0) -> str:
    return f'{prefix}:{crate}:{item.get("file", "")}:{item.get("line", 0)}:{item.get("name", seq)}'


def _base_name(raw: str) -> str:
    return raw.rsplit('::', 1)[-1].strip()


def _query_mentions(queries: list[str]) -> list[dict]:
    """Extract query references and filter roles; only a heuristic.

    Never use this parser to prove a Bevy type's semantic role. We only join to
    Component derives that the inventory already identified.
    """
    seen: set[tuple[str, str]] = set()
    result: list[dict] = []
    for query in queries:
        for match in REFERENCE_RE.finditer(query):
            raw = match.group(2)
            if _base_name(raw) in SKIP_TOKENS:
                continue
            role = 'write' if match.group(1) else 'read'
            key = (raw, role)
            if key not in seen:
                seen.add(key)
                result.append({'name': _base_name(raw), 'qualified': raw, 'role': role})
        for match in FILTER_RE.finditer(query):
            raw = match.group(2)
            if _base_name(raw) in SKIP_TOKENS:
                continue
            role = match.group(1).lower()
            key = (raw, role)
            if key not in seen:
                seen.add(key)
                result.append({'name': _base_name(raw), 'qualified': raw, 'role': role})
    return result


def _resolve_type(name: str, crate: str, by_name: dict[str, list[dict]], kind: str) -> list[str]:
    # Multiple definitions with one base name are not resolved by guesswork.
    options = [r for r in by_name.get(_base_name(name), []) if r['kind'] == kind]
    parts = name.split('::')
    local = [r for r in options if r['crate'] == crate]
    if len(parts) > 1:
        if parts[0] in {'crate', 'self', 'super'}:
            return [local[0]['id']] if len(local) == 1 else []
        # A full crate path is reliable; an import alias is not.
        if parts[0] in {r['crate'] for r in options}:
            qualified = [r for r in options if r['crate'] == parts[0]]
            return [qualified[0]['id']] if len(qualified) == 1 else []
        return []
    if len(local) == 1:
        return [local[0]['id']]
    if len(options) == 1:
        return [options[0]['id']]
    return []


def _git_head() -> str | None:
    try:
        value = subprocess.run(['git', 'rev-parse', '--short=12', 'HEAD'],
                               cwd=ROOT, capture_output=True, text=True, timeout=2, check=True)
        return value.stdout.strip() or None
    except (OSError, subprocess.CalledProcessError, subprocess.TimeoutExpired):
        return None


def create_model(*, allow_stale: bool = False) -> dict[str, Any]:
    project = _load_json(INVENTORY / 'project.json')
    if project.get('schema_version') != 4:
        raise ValueError(f'Unsupported ECS inventory schema {project.get("schema_version")}; expected 4')
    stamp = _load_json(INDEX / 'generation_stamp.json')
    head = _git_head()
    indexed = stamp.get('generated_from_commit', '')
    if not allow_stale and head and indexed and not head.startswith(indexed) and not indexed.startswith(head):
        raise ValueError(f'ECS index describes commit {indexed}, but HEAD is {head}. '
                         'Regenerate .agent indexes before browsing the current code. '
                         'Use --allow-stale to inspect the existing snapshot.')
    output: dict[str, Any] = {
        'meta': {
            'commit': indexed,
            'head': head,
            'stale': bool(head and indexed and head != indexed),
            'generated': stamp.get('generated_at', ''),
            'schema': project['schema_version'],
            'coverage': 'Static source inventory; spawn sites are not live entities.',
            'resolvedDeps': False,
        },
        'sections': {section: [] for section in
                     ['systems', 'components', 'resources', 'messages', 'channels', 'bundles',
                      'spawns', 'crates', 'plugins', 'registrations']},
    }
    records = output['sections']
    cache: dict[str, list[str]] = {}
    shards: list[dict] = []
    for crate_entry in project['crates']:
        shard_path = INVENTORY / crate_entry['json']
        shard = _load_json(shard_path)
        if shard.get('schema_version') != 4 or shard.get('crate_name') != crate_entry['crate_name']:
            raise ValueError(f'Inconsistent ECS shard: {shard_path}')
        shards.append(shard)

    name_index: dict[str, list[dict]] = collections.defaultdict(list)
    for shard in shards:
        crate = shard['crate_name']
        for item in shard.get('ecs_items', []):
            for kind in item.get('derives', []):
                if kind not in TYPE_KINDS:
                    continue
                section = {'Component': 'components', 'Resource': 'resources',
                           'Message': 'messages', 'Bundle': 'bundles'}[kind]
                row = dict(id=_ref(crate, item, 't') + ':' + kind, name=item['name'],
                           crate=crate, kind=kind, file=item['file'], line=item['line'],
                           visibility=item.get('visibility', ''), derives=item.get('derives', []),
                           excerpt=_excerpt(item['file'], item['line'], cache),
                           systems=[], spawnIds=[], reads=0, writes=0, filters=0)
                records[section].append(row)
                name_index[item['name']].append(row)

    sys_by_crate_name: dict[tuple[str, str], list[dict]] = collections.defaultdict(list)
    for shard in shards:
        crate = shard['crate_name']
        for seq, item in enumerate(shard.get('system_like_functions', [])):
            mentions = _query_mentions(item.get('queries', []))
            # Only count known components in the component-use ranking.
            for mention in mentions:
                mention['targets'] = _resolve_type(mention['qualified'], crate, name_index, 'Component')
            row = dict(id=_ref(crate, item, 's', seq), kind='System', name=item['name'], crate=crate,
                       file=item['file'], line=item['line'], public=item.get('public', False),
                       params=item.get('params', ''), queries=item.get('queries', []),
                       queryCount=len(item.get('queries', [])), mentions=mentions,
                       componentCount=len({target for mention in mentions for target in mention['targets']}),
                       resourceReads=item.get('resources_read', []),
                       resourceWrites=item.get('resources_written', []),
                       messageReads=item.get('messages_read', []),
                       messageWrites=item.get('messages_written', []),
                       commands=item.get('commands', False), locals=item.get('locals', []),
                       registrations=[], excerpt=_excerpt(item['file'], item['line'], cache))
            records['systems'].append(row)
            sys_by_crate_name[(crate, item['name'])].append(row)

    for shard in shards:
        crate = shard['crate_name']
        for name, channel in sorted(shard.get('message_bus', {}).items()):
            records['channels'].append(dict(
                id=f'm:{crate}:{name}', kind='Message channel', name=name, crate=crate,
                file='', line=0, readers=channel.get('read_by', []),
                writers=channel.get('written_by', []),
                registeredAt=channel.get('registered_at', []), systems=[],
                excerpts=[]))
        for seq, item in enumerate(shard.get('registrations', [])):
            row = dict(id=_ref(crate, item, 'r', seq), kind='Registration', crate=crate,
                       name=item.get('kind', '') + ' / ' + item.get('context', ''),
                       regKind=item.get('kind', ''), context=item.get('context', ''),
                       file=item.get('file', ''), line=item.get('line', 0),
                       expression=item.get('expression', ''), schedule=item.get('schedule_or_arg', ''),
                       systemNames=item.get('systems', []), systems=[],
                       sets=item.get('sets', []), ordering=item.get('ordering', []),
                       runConditions=item.get('run_conditions', []),
                       excerpt=_excerpt(item['file'], item['line'], cache))
            for raw in row['systemNames']:
                options = sys_by_crate_name.get((crate, _base_name(raw)), [])
                # A source function name can be overloaded by modules in a crate.
                # The link remains navigational rather than proof of registration.
                for sys in options:
                    row['systems'].append(sys['id'])
                    sys['registrations'].append(row['id'])
            row['systems'] = sorted(set(row['systems']))
            records['registrations'].append(row)

        for seq, item in enumerate(shard.get('spawn_sites', [])):
            labels = [s.strip('"') for s in item.get('name_labels', [])]
            matched = item.get('matched_ecs_items', [])
            name = labels[0] if labels else (', '.join(matched[:2]) if matched else 'Entity spawn')
            row = dict(id=_ref(crate, item, 'e', seq), kind='Spawn evidence',
                       crate=crate, name=name, labels=labels, matched=matched,
                       file=item['file'], line=item['line'], expression=item.get('expression', ''),
                       identifiers=item.get('identifiers', []), components=[],
                       excerpt=_excerpt(item['file'], item['line'], cache))
            for raw in matched:
                target_ids = _resolve_type(raw, crate, name_index, 'Component')
                row['components'].extend(target_ids)
                for tid in target_ids:
                    next(t for t in records['components'] if t['id'] == tid)['spawnIds'].append(row['id'])
            row['components'] = sorted(set(row['components']))
            records['spawns'].append(row)

        for item in shard.get('plugins', []):
            records['plugins'].append(dict(id=_ref(crate, item, 'p'), kind='Plugin',
                                           name=item['name'], crate=crate, file=item['file'],
                                           line=item['line'], registrations=[],
                                           excerpt=_excerpt(item['file'], item['line'], cache)))
        records['crates'].append(dict(id='c:' + crate, kind='Crate', name=crate, crate=crate,
                                     file=shard['crate_root'] + '/Cargo.toml', line=1,
                                     counts=shard['counts'], root=shard['crate_root'],
                                     modules=shard.get('module_summaries', []),
                                     deps=[], rdeps=[], systems=[], components=[], bundles=[],
                                     resources=[], messages=[], channels=[], plugins=[], spawns=[]))

    id_lookup = {row['id']: row for rows in records.values() for row in rows}
    for sys in records['systems']:
        for mention in sys['mentions']:
            for target in mention['targets']:
                comp = id_lookup[target]
                comp['systems'].append(sys['id'])
                if mention['role'] == 'read':
                    comp['reads'] += 1
                elif mention['role'] == 'write':
                    comp['writes'] += 1
                else:
                    comp['filters'] += 1
        for raw, attr, kind in [
            (sys['resourceReads'], 'reads', 'Resource'),
            (sys['resourceWrites'], 'writes', 'Resource'),
            (sys['messageReads'], 'reads', 'Message'),
            (sys['messageWrites'], 'writes', 'Message'),
        ]:
            for name in set(raw):
                target_ids = _resolve_type(name, sys['crate'], name_index, kind)
                for target in target_ids:
                    t = id_lookup[target]
                    t['systems'].append(sys['id'])
                    t[attr] += 1
        sys['registeredHere'] = bool(sys['registrations'])

    for section in ['components', 'resources', 'messages', 'bundles']:
        for item in records[section]:
            item['systems'] = sorted(set(item['systems']))
            item['usageCount'] = len(item['systems'])
            item['spawnIds'] = sorted(set(item['spawnIds']))

    # Message-bus evidence can point to a function without a Message derive.
    evidence_re = re.compile(r'^(.*?) \((.*?):(\d+)\)$')
    by_position = {(s['file'], s['line']): s['id'] for s in records['systems']}
    for channel in records['channels']:
        linked = []
        for raw in channel['readers'] + channel['writers']:
            match = evidence_re.match(raw)
            if match:
                target = by_position.get((match.group(2), int(match.group(3))))
                if target:
                    linked.append(target)
        channel['systems'] = sorted(set(linked))

    # Source registrations reference plugin build contexts by name.
    for plugin in records['plugins']:
        for reg in records['registrations']:
            if plugin['crate'] == reg['crate'] and reg['context'].split('::')[0] == plugin['name']:
                plugin['registrations'].append(reg['id'])

    # Declared Cargo graph; labels must not be misread as compiled dependencies.
    declared_path = INDEX / 'crates' / 'graph-declared.json'
    if declared_path.exists():
        graph = _load_json(declared_path)
        for crate in records['crates']:
            for edge in graph.get('edges', {}).get(crate['name'], []):
                if edge.get('internal'):
                    crate['deps'].append({'crate': edge['name'],
                                          'optional': bool(edge.get('optional')),
                                          'kind': edge.get('kind', '')})
                    target = id_lookup.get('c:' + edge['name'])
                    if target:
                        target['rdeps'].append(crate['name'])

    for section in ['systems', 'components', 'resources', 'messages', 'channels', 'bundles', 'spawns', 'plugins']:
        for item in records[section]:
            id_lookup['c:' + item['crate']][section].append(item['id'])

    for section in records:
        records[section].sort(key=lambda item: (item['crate'], item.get('file', ''),
                                                 item.get('line', 0), item['name']))

    output['stats'] = {
        'counts': project.get('counts', {}),
        'crateCount': project['crate_count'],
        'registeredMatched': sum(bool(s['registrations']) for s in records['systems']),
        'queryParams': sum(s['queryCount'] for s in records['systems']),
        'componentRefs': sum(c['reads'] + c['writes'] + c['filters'] for c in records['components']),
        'systemsWithQueries': sum(s['queryCount'] > 0 for s in records['systems']),
    }
    return output


def write_browser(output: Path, model: dict) -> None:
    template = (WEB / 'index.html').read_text(encoding='utf-8')
    css = (WEB / 'style.css').read_text(encoding='utf-8')
    js = (WEB / 'app.js').read_text(encoding='utf-8')
    data = json.dumps(model, ensure_ascii=False, separators=(',', ':'), sort_keys=True)
    data = data.replace('&', '\\u0026').replace('<', '\\u003c').replace('>', '\\u003e')
    data = data.replace('\u2028', '\\u2028').replace('\u2029', '\\u2029')
    for token, replacement in {'__CSS__': css, '__DATA__': data, '__JS__': js}.items():
        if token not in template:
            raise ValueError(f'Missing browser template marker {token}')
        template = template.replace(token, replacement)
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(template, encoding='utf-8')


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path, default=DEFAULT_OUTPUT,
                        help='HTML path (default: .agent/ecs_browser/index.html)')
    parser.add_argument('--open', action='store_true', help='Open the generated HTML in a browser')
    parser.add_argument('--allow-stale', action='store_true', help='Build from stale index with warning')
    args = parser.parse_args(argv)
    try:
        model = create_model(allow_stale=args.allow_stale)
        write_browser(args.output, model)
    except (ValueError, OSError, json.JSONDecodeError) as exc:
        parser.exit(2, f'error: {exc}\n')
    print(f'Wrote {args.output} ({args.output.stat().st_size:,} bytes)')
    print('Snapshot:', model['meta']['commit'], '/', len(model['sections']['systems']), 'system candidates,',
          len(model['sections']['components']), 'components,',
          len(model['sections']['spawns']), 'spawn sites')
    if args.open:
        webbrowser.open(args.output.resolve().as_uri())
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
