#!/usr/bin/env python3
"""Audit top-level Rust module reference cycles across a Cargo workspace.

This is a *source-path* audit, not a Rust compiler dependency graph. Edges
are explicit `crate::module` paths, `super::module` paths that reach the crate
root, and non-wildcard root `pub use` aliases. Qualified paths occurring in
production code count, including direct function/type references (not only
`use` declarations). Nested imports such as `use crate::{a, b}` are expanded. The scan also
reports SCCs among nested sibling modules within directories.

Test-only modules/files/items, comments, and string literals are omitted.
Workspace crates are discovered from root Cargo.toml without requiring Cargo.
No external Python dependencies are required.

    python3 scripts/audit_internal_module_cycles.py --top 30
    python3 scripts/audit_internal_module_cycles.py --crate ambition_combat --details
    python3 scripts/audit_internal_module_cycles.py --json /tmp/module-cycles.json

CAVEATS: in-source inline `mod` namespaces, glob root reexports, macro token
expansion, feature-conditional compilation and custom `#[path]` modules are not
fully modeled. Refs from unqualified paths are conservatively excluded.
"""
from __future__ import annotations

import argparse
from collections import Counter, defaultdict
from dataclasses import dataclass
import json
from pathlib import Path
import re
import tomllib

IDENT = r'[A-Za-z_][A-Za-z_0-9]*'
PATH_RE = re.compile(r'(?<![A-Za-z_0-9])((?:(?:crate|super)\s*::\s*)+)(%s)' % IDENT)
ROOT_ALIAS_RE = re.compile(r'\bpub(?:\s*\([^)]*\))?\s+use\s+([^;]+);', re.S)
USE_RE = re.compile(r'\buse\s+([^;]+);', re.S)
MOD_DECL_RE = re.compile(r'\bmod\s+(%s)\s*([;{])' % IDENT)
CFG_TEST_RE = re.compile(r'#\s*\[\s*cfg\s*\(\s*test\s*\)\s*\]')
RAW_STRING_RE = re.compile(r'\b(?:br|rb|r)(?P<hash>#{0,20})"')


def mask_span(buf: list[str], a: int, b: int) -> None:
    for i in range(a, b):
        if buf[i] != '\n':
            buf[i] = ' '


def strip_comments_and_literals(source: str) -> str:
    """Mask comments and literals while preserving exact source offsets/lines."""
    out = list(source)
    size = len(source)
    i = 0
    while i < size:
        if source.startswith('//', i):
            j = source.find('\n', i)
            if j == -1:
                j = size
            mask_span(out, i, j)
            i = j
        elif source.startswith('/*', i):
            start = i
            depth = 1
            i += 2
            while i < size and depth:
                if source.startswith('/*', i):
                    depth += 1
                    i += 2
                elif source.startswith('*/', i):
                    depth -= 1
                    i += 2
                else:
                    i += 1
            mask_span(out, start, i)
        else:
            raw = RAW_STRING_RE.match(source, i)
            if raw:
                end_pat = '"' + raw.group('hash')
                j = source.find(end_pat, raw.end())
                j = size if j < 0 else j + len(end_pat)
                mask_span(out, i, j)
                i = j
            elif source[i] == '"':
                start = i
                i += 1
                while i < size:
                    if source[i] == '\\':
                        i += 2
                    elif source[i] == '"':
                        i += 1
                        break
                    else:
                        i += 1
                mask_span(out, start, min(i, size))
            else:
                i += 1
    return ''.join(out)


def skip_balanced(text: str, start: int, open_c='{', close_c='}') -> int:
    """Return index just after the matching closing delimiter."""
    assert text[start] == open_c
    d = 0
    for j in range(start, len(text)):
        if text[j] == open_c:
            d += 1
        elif text[j] == close_c:
            d -= 1
            if not d:
                return j + 1
    return len(text)


def strip_test_items(clean: str) -> str:
    """Mask `#[cfg(test)]` items, including inline modules and functions."""
    out = list(clean)
    cursor = 0
    while (match := CFG_TEST_RE.search(clean, cursor)) is not None:
        j = match.end()
        # Interleaved outer attributes and whitespace; any comments were masked.
        while True:
            j = re.match(r'\s*', clean[j:]).end() + j
            if clean.startswith('#[', j):
                k = clean.find(']', j)
                if k < 0:
                    break
                j = k + 1
            else:
                break
        # Find the start of the item (pub/use, fn, mod, impl, const, etc).
        brace = clean.find('{', j)
        semicolon = clean.find(';', j)
        if semicolon != -1 and (brace == -1 or semicolon < brace):
            end = semicolon + 1
        elif brace != -1:
            end = skip_balanced(clean, brace)
        else:
            end = len(clean)
        mask_span(out, match.start(), end)
        cursor = end
    return ''.join(out)


def split_commas(body: str) -> list[str]:
    chunks = []
    begin = 0
    nest = 0
    for i, ch in enumerate(body):
        if ch == '{':
            nest += 1
        elif ch == '}':
            nest -= 1
        elif ch == ',' and nest == 0:
            chunks.append(body[begin:i].strip())
            begin = i + 1
    chunks.append(body[begin:].strip())
    return [x for x in chunks if x]


def expand_use_tree(expr: str, prefix: str = '') -> list[tuple[str, str]]:
    """Return (imported path, exposed alias) pairs from Rust use tree syntax."""
    expr = expr.strip().lstrip(':').strip()
    brace = expr.find('{')
    if brace >= 0 and expr.endswith('}'):
        head = expr[:brace].strip().rstrip(':')
        pfx = prefix + ('::' if prefix and head else '') + head
        return [pair for part in split_commas(expr[brace + 1:-1])
                for pair in expand_use_tree(part, pfx)]
    rename = re.search(r'\s+as\s+(%s)\s*$' % IDENT, expr)
    alias = rename.group(1) if rename else expr.split('::')[-1].strip()
    path = expr[:rename.start()].strip() if rename else expr
    full = prefix + ('::' if prefix and path else '') + path
    # self in grouped `foo::{self, bar}` means `foo`.
    if full.endswith('::self'):
        full = full[:-6]
    return [(full, alias)]


def module_path(source_dir: Path, file: Path) -> tuple[str, ...]:
    rel = file.relative_to(source_dir)
    parts = list(rel.parts)
    if parts[-1] in ('mod.rs', 'lib.rs', 'main.rs'):
        parts.pop()
    else:
        parts[-1] = parts[-1][:-3]
    return tuple(parts)


def is_test_file(src_dir: Path, path: Path, clean: str) -> bool:
    rel = path.relative_to(src_dir)
    if rel.name == 'tests.rs' or rel.name.endswith('_tests.rs'):
        return True
    if 'tests' in rel.parts[:-1]:
        return True
    if re.search(r'^\s*#!\s*\[\s*cfg\s*\(\s*test\s*\)\s*\]', clean, re.M):
        return True
    return False


def strongly_connected(nodes: set[str], edges: dict[str, Counter]) -> list[list[str]]:
    """Tarjan SCC; only return cyclic components with >1 node."""
    idx = 0
    stack = []
    active = set()
    indices = {}
    low = {}
    comps = []

    def visit(v):
        nonlocal idx
        indices[v] = low[v] = idx
        idx += 1
        stack.append(v)
        active.add(v)
        for w in edges.get(v, ()):
            if w not in indices:
                visit(w)
                low[v] = min(low[v], low[w])
            elif w in active:
                low[v] = min(low[v], indices[w])
        if low[v] == indices[v]:
            comp = []
            while True:
                w = stack.pop()
                active.remove(w)
                comp.append(w)
                if w == v:
                    break
            if len(comp) > 1:
                comps.append(sorted(comp))

    for v in sorted(nodes):
        if v not in indices:
            visit(v)
    return sorted(comps, key=lambda c: (-len(c), c))


def root_reexports(src_root: Path, modules: set[str]) -> dict[str, str]:
    rootfile = src_root / 'lib.rs'
    if not rootfile.exists():
        rootfile = src_root / 'main.rs'
    if not rootfile.exists():
        return {}
    text = strip_test_items(strip_comments_and_literals(rootfile.read_text(errors='replace')))
    aliases = defaultdict(set)
    for match in ROOT_ALIAS_RE.finditer(text):
        for path, alias in expand_use_tree(match.group(1)):
            path = path.removeprefix('crate::')
            first = path.split('::', 1)[0]
            if first in modules and alias != '*' and alias not in modules:
                aliases[alias].add(first)
    return {name: next(iter(owners)) for name, owners in aliases.items() if len(owners) == 1}


@dataclass
class CrateScan:
    path: str
    modules: dict[str, int]
    edges: dict[str, Counter]
    sites: dict[tuple[str, str], list[tuple[str, int, str]]]
    cycles: list[list[str]]
    alias_count: int
    files_scanned: int
    has_globs: bool


def reachable_module_files(src: Path, include_tests=False) -> tuple[list[Path], set[str], int]:
    """Follow Rust `mod child;` declarations from the lib/main source root.

    Ignore the physically present but unreferenced Rust files and test-gated
    modules. `#[path]` overrides and generated mods are not modeled here.
    """
    rootfile = src / 'lib.rs'
    if not rootfile.is_file():
        rootfile = src / 'main.rs'
    if not rootfile.is_file():
        return [], set(), 0
    pending = [rootfile]
    visited = set()
    root_names = set()
    unresolved = 0
    while pending:
        path = pending.pop()
        if path in visited:
            continue
        visited.add(path)
        content = path.read_text(encoding='utf-8', errors='replace')
        clean = strip_comments_and_literals(content)
        if not include_tests:
            clean = strip_test_items(clean)
        rel = path.relative_to(src)
        top = module_path(src, path)
        for match in MOD_DECL_RE.finditer(clean):
            name, delim = match.groups()
            if not top:
                root_names.add(name)
            if delim == '{':
                # Inline modules are not file modules. Their item bodies may
                # have child declarations; this audit records the limitation.
                continue
            if rel.name in ('mod.rs', 'lib.rs', 'main.rs'):
                parent = path.parent
            else:
                parent = path.parent / path.stem
            target = parent / (name + '.rs')
            if not target.is_file():
                target = parent / name / 'mod.rs'
            if target.is_file():
                pending.append(target)
            else:
                unresolved += 1
    return sorted(visited), root_names, unresolved


def scan_crate(root: Path, member: str, include_tests=False) -> CrateScan:
    src = root / member / 'src'
    all_rs, root_names, unresolved = reachable_module_files(src, include_tests)
    modules = {}
    file_texts = []
    for path in all_rs:
        mod = module_path(src, path)
        if not mod:
            continue  # lib.rs is crate-root facade
        if mod[0] not in root_names:
            continue
        text = path.read_text(encoding='utf-8', errors='replace')
        code = strip_comments_and_literals(text)
        if not include_tests:
            if is_test_file(src, path, code):
                continue
            code = strip_test_items(code)
        modules.setdefault(mod[0], 0)
        modules[mod[0]] += text.count('\n') + int(bool(text) and not text.endswith('\n'))
        file_texts.append((path, mod, text, code))
    known = set(modules)
    aliases = root_reexports(src, known)
    edges: dict[str, Counter] = defaultdict(Counter)
    sites: dict[tuple[str, str], list[tuple[str, int, str]]] = defaultdict(list)
    has_globs = False

    def register(srcmod, target, path, position, origin_text):
        owner = aliases.get(target, target)
        if owner not in known or owner == srcmod:
            return
        edges[srcmod][owner] += 1
        line = origin_text.count('\n', 0, position) + 1
        excerpt = origin_text.splitlines()[line - 1].strip()
        site = (str(path.relative_to(root)), line, excerpt[:180])
        if site not in sites[(srcmod, owner)]:
            sites[(srcmod, owner)].append(site)

    for path, mod, source, code in file_texts:
        for match in PATH_RE.finditer(code):
            qualifiers = re.findall(r'crate|super', match.group(1))
            target = match.group(2)
            if qualifiers[0] == 'crate':
                if len(qualifiers) == 1:
                    register(mod[0], target, path, match.start(), source)
            elif all(q == 'super' for q in qualifiers) and len(qualifiers) >= len(mod):
                register(mod[0], target, path, match.start(), source)
        for match in USE_RE.finditer(code):
            clause = match.group(1)
            if '*' in clause:
                has_globs = True
            if not ('{' in clause and '}' in clause):
                continue
            for full, _alias in expand_use_tree(clause):
                full = re.sub(r'\s+', '', full)
                bits = full.split('::')
                target = None
                if len(bits) >= 2 and bits[0] == 'crate':
                    target = bits[1]
                elif bits[0] == 'super' and len(bits) > len(mod) and all(
                        bit == 'super' for bit in bits[:len(mod)]):
                    target = bits[len(mod)]
                if target:
                    register(mod[0], target, path, match.start(), source)

    cycles = strongly_connected(known, edges)
    return CrateScan(member, modules, dict(edges), dict(sites), cycles,
                     len(aliases), len(file_texts), has_globs)



# This second pass checks sibling modules *inside* top-level folders.
FULL_PATH_RE = re.compile(r'(?<![A-Za-z_0-9])(?:crate|super|self)(?:\s*::\s*%s)+' % IDENT)


def scan_nested_sibling_cycles(root: Path, member: str) -> list[dict]:
    """Find SCCs among siblings at each nested module path.

    A top-level cycle is not the only obstacle to splitting a crate. For
    example, `foo::left <-> foo::right` is invisible to a top-level SCC
    audit because both references belong to the `foo` owner. We resolve the
    deepest referenced file module, then aggregate edges at each siblings'
    nearest common parent. Parent-child uses are not treated as sibling edges.
    """
    src = root / member / 'src'
    files, root_names, _unresolved = reachable_module_files(src)
    modules: set[tuple[str, ...]] = set()
    module_loc: dict[tuple[str, ...], int] = {}
    sources: list[tuple[tuple[str, ...], str, Path, str]] = []
    for path in files:
        mod = module_path(src, path)
        if not mod or mod[0] not in root_names:
            continue
        text = path.read_text(encoding='utf-8', errors='replace')
        clean = strip_comments_and_literals(text)
        if is_test_file(src, path, clean):
            continue
        clean = strip_test_items(clean)
        modules.add(mod)
        module_loc[mod] = len(text.splitlines())
        sources.append((mod, clean, path, text))
    aliases = root_reexports(src, {m[0] for m in modules})

    def resolve(bits: list[str], cur: tuple[str, ...]) -> tuple[str, ...] | None:
        if not bits:
            return None
        if bits[0] == 'crate':
            tokens = bits[1:]
        elif bits[0] == 'self':
            tokens = [*cur, *bits[1:]]
        elif bits[0] == 'super':
            tokens = list(cur)
            i = 0
            while i < len(bits) and bits[i] == 'super':
                if tokens:
                    tokens.pop()
                i += 1
            tokens.extend(bits[i:])
        else:
            return None
        if not tokens:
            return None
        tokens[0] = aliases.get(tokens[0], tokens[0])
        for n in range(len(tokens), 0, -1):
            cand = tuple(tokens[:n])
            if cand in modules:
                return cand
        return None

    edges: dict[tuple[str, ...], Counter] = defaultdict(Counter)
    sites: dict[tuple[tuple[str, ...], tuple[str, ...]], list] = defaultdict(list)

    def add_ref(source: tuple[str, ...], bits: list[str], path: Path, pos: int, text: str):
        target = resolve(bits, source)
        if target is None or target == source:
            return
        edges[source][target] += 1
        line = text.count('\n', 0, pos) + 1
        site = (str(path.relative_to(root)), line)
        if site not in sites[(source, target)]:
            sites[(source, target)].append(site)

    for mod, code, file, source in sources:
        for match in FULL_PATH_RE.finditer(code):
            tokens = re.sub(r'\s+', '', match.group(0)).split('::')
            add_ref(mod, tokens, file, match.start(), source)
        for match in USE_RE.finditer(code):
            clause = match.group(1)
            if '{' in clause and '}' in clause:
                for qualified, _alias in expand_use_tree(clause):
                    bits = re.sub(r'\s+', '', qualified).split('::')
                    add_ref(mod, bits, file, match.start(), source)

    children: dict[tuple[str, ...], set[str]] = defaultdict(set)
    for mod in modules:
        for depth in range(len(mod)):
            children[mod[:depth]].add(mod[depth])

    by_parent: dict[tuple[str, ...], dict[str, Counter]] = defaultdict(
        lambda: defaultdict(Counter)
    )
    by_parent_sites: dict[tuple[tuple[str, ...], str, str], list] = defaultdict(list)
    for source, targets in edges.items():
        for target, weight in targets.items():
            length = 0
            while length < min(len(source), len(target)) and source[length] == target[length]:
                length += 1
            if len(source) > length and len(target) > length:
                parent = source[:length]
                a, b = source[length], target[length]
                by_parent[parent][a][b] += weight
                for site in sites[source, target]:
                    if site not in by_parent_sites[(parent, a, b)]:
                        by_parent_sites[(parent, a, b)].append(site)

    result = []
    for parent, names in sorted(children.items()):
        if not parent:
            continue  # top-level cycles are already reported by scan_crate
        comps = strongly_connected(names, by_parent[parent])
        for component in comps:
            parent_edges = by_parent[parent]
            connected = []
            for a in component:
                for b, weight in parent_edges.get(a, {}).items():
                    if b in component:
                        connected.append({
                            'source': a, 'target': b, 'references': weight,
                            'sites': by_parent_sites[(parent, a, b)][:6],
                        })
            involved = set(component)
            loc = sum(v for mod, v in module_loc.items()
                      if len(mod) > len(parent) and mod[:len(parent)] == parent
                      and mod[len(parent)] in involved)
            result.append({
                'parent': '::'.join(parent), 'modules': component,
                'lines': loc, 'edges': sorted(connected, key=lambda e: (e['source'], e['target'])),
            })
    return sorted(result, key=lambda x: (-len(x['modules']), x['parent']))

def single_edge_cuts(scan: CrateScan, component: list[str]) -> list[dict]:
    """Edges that, individually, shrink this component's largest cyclic SCC."""
    old_size = len(component)
    nodes = set(component)
    cuts = []
    for source in component:
        for target, count in scan.edges.get(source, {}).items():
            if target not in nodes:
                continue
            modified = {k: v.copy() for k, v in scan.edges.items()}
            del modified[source][target]
            after = strongly_connected(nodes, modified)
            max_after = max((len(c) for c in after), default=1)
            if max_after < old_size:
                cuts.append({'from': source, 'to': target,
                             'references': count, 'largest_after': max_after,
                             'sites': scan.sites.get((source, target), [])[:5]})
    cuts.sort(key=lambda x: (x['largest_after'], x['references'], x['from'], x['to']))
    return cuts


def scan_workspace(root: Path, filters: list[str] | None = None) -> list[CrateScan]:
    workspace = tomllib.loads((root / 'Cargo.toml').read_text(encoding='utf-8'))
    members = workspace['workspace']['members']
    if filters:
        members = [m for m in members if any(m == f or Path(m).name == f for f in filters)]
    return [scan_crate(root, m) for m in members if (root / m / 'src').exists()]


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--root', type=Path, default=Path(__file__).resolve().parent.parent)
    p.add_argument('--crate', action='append', help='limit to crate name or workspace path')
    p.add_argument('--top', type=int, default=0, help='display at most N crates with cycles')
    p.add_argument('--details', action='store_true', help='show SCCs, edge cuts, and example source sites')
    p.add_argument('--json', type=Path, help='write full machine-readable report')
    args = p.parse_args()
    scans = scan_workspace(args.root.resolve(), args.crate)
    nested = {s.path: scan_nested_sibling_cycles(args.root.resolve(), s.path) for s in scans}
    with_cycles = [s for s in scans if s.cycles]
    # Nested-only cycles matter too. In particular, a top-level DAG may still
    # contain tightly coupled siblings that block a smaller future carve.
    with_any_cycles = [s for s in scans if s.cycles or nested[s.path]]

    def largest_component(scan: CrateScan) -> int:
        return max([*(len(comp) for comp in scan.cycles),
                    *(len(comp['modules']) for comp in nested[scan.path])], default=0)

    with_any_cycles.sort(key=lambda s: (-largest_component(s),
                                        -sum(s.modules.values()), s.path))
    print(f'Workspace members audited: {len(scans)}')
    print(f'Crates with cyclic top-level modules: {len(with_cycles)}')
    print(f'Crates with any detected module cycle: {len(with_any_cycles)}')
    print(f'Cyclic strongly-connected components: {sum(len(s.cycles) for s in scans)}')
    print(f'Modules scanned: {sum(len(s.modules) for s in scans)}')
    print(f'Production files scanned: {sum(s.files_scanned for s in scans)}')
    print(f'Nested sibling SCCs: {sum(len(items) for items in nested.values())} across {sum(bool(x) for x in nested.values())} members')
    for scan in with_any_cycles[:args.top or len(with_any_cycles)]:
        print(f'  {scan.path:<56} max={largest_component(scan):>2} top_SCCs={len(scan.cycles):>2} nested_SCCs={len(nested[scan.path]):>2} LOC={sum(scan.modules.values()):>6}')
        if args.details:
            for comp in scan.cycles:
                print('    SCC:', ', '.join(comp))
                cuts = single_edge_cuts(scan, comp)
                if cuts:
                    for c in cuts[:5]:
                        print(f"      CUT {c['from']} -> {c['to']} refs={c['references']} max-after={c['largest_after']}")
                        for file, line, excerpt in c['sites'][:1]:
                            print(f'        {file}:{line}: {excerpt}')
                else:
                    print('      no single edge shrinks this SCC')
            for comp in nested[scan.path]:
                print(f"    NESTED {comp['parent']} ({len(comp['modules'])}): {', '.join(comp['modules'])}")
    if args.json:
        args.json.parent.mkdir(exist_ok=True, parents=True)
        report = {'workspace_root': str(args.root.resolve()),
                  'total_crates': len(scans),
                  'crates_with_cycles': len(with_cycles),
                  'total_sccs': sum(len(s.cycles) for s in scans),
                  'total_nested_sibling_sccs': sum(len(n) for n in nested.values()),
                  'crates': []}
        for scan in scans:
            report['crates'].append({
                'nested_sibling_components': nested[scan.path],
                'path': scan.path,
                'production_lines': sum(scan.modules.values()),
                'file_count': scan.files_scanned,
                'modules': scan.modules,
                'root_alias_count': scan.alias_count,
                'has_glob_use': scan.has_globs,
                'cyclic_components': [
                    {'modules': c,
                     'lines': sum(scan.modules[n] for n in c),
                     'single_edge_cuts': single_edge_cuts(scan, c)}
                    for c in scan.cycles],
                'edges': [
                    {'source': a, 'target': b, 'references': cnt,
                     'sites': scan.sites.get((a, b), [])}
                    for a, outs in sorted(scan.edges.items())
                    for b, cnt in sorted(outs.items())],
            })
        args.json.write_text(json.dumps(report, indent=2) + '\n')
        print(f'JSON: {args.json}')


if __name__ == '__main__':
    main()
