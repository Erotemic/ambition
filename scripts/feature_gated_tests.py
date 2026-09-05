#!/usr/bin/env python3
"""Measure tests hidden behind Cargo feature gates.

The script compares default and feature-enabled test discovery so `cargo test -p
<crate>` cannot appear comprehensive while gated tests are absent from the run."""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
CRATE_ROOTS = ('crates', 'game')

TEST_ATTR = re.compile(r'#\[\s*(?:tokio::)?test\s*[\]\(]')
CFG_FEATURE = re.compile(r'#!?\[\s*cfg\s*\([^)]*feature\s*=\s*"([a-zA-Z0-9_\-]+)"')
MOD_DECL = re.compile(r'\bmod\s+[a-zA-Z0-9_]+\s*[;{]')


def crate_dirs() -> list[Path]:
    found = []
    for root in CRATE_ROOTS:
        for cargo in sorted((REPO_ROOT / root).rglob('Cargo.toml')):
            if '/target/' in str(cargo) or '/.claude/' in str(cargo):
                continue
            found.append(cargo.parent)
    return found


def default_features(crate: Path) -> set[str]:
    """Every feature reachable from this crate's `default`, transitively.

    ⛔⛔ WITHOUT THIS THE SURVEY IS WRONG IN THE DIRECTION THAT MATTERS. It
    counted any `#[cfg(feature = "x")]` as hidden, never asking whether `x` is ON
    BY DEFAULT. `ambition_platformer2d_ldtk` was reported as *51 of 53 run bare
    (ldtk_runtime, portal_ldtk)* while `--verify` -- which asks cargo -- answers
    **53 of 53**, because both of those features are in `default`.

    ⚠ The consumer is what makes it costly: the ledger test tells whoever moves
    the number that their test "does NOT run in the default gate plan", and for a
    default-on feature that sentence is false. MEASURED 2026-09-05 two ways: the
    two tests that moved the count run under `cargo test --workspace`, and cargo
    lists them in a bare `cargo test -p`.

    ⭐ "Behind a cfg(feature)" and "absent from a default run" are DIFFERENT
    CLAIMS. This function is what lets the survey make the second one.

    `dep:` entries and `crate/feature` entries are not this crate's own features
    and are skipped; a missing or unparseable manifest yields an empty set, which
    keeps the OLD, over-reporting behaviour rather than inventing a clean bill.
    """
    try:
        import tomllib
    except ModuleNotFoundError:  # pragma: no cover - Python < 3.11
        return set()
    manifest = crate / 'Cargo.toml'
    try:
        table = tomllib.loads(manifest.read_text(encoding='utf8')).get('features', {})
    except Exception:
        return set()
    closure: set[str] = set()
    stack = list(table.get('default', []))
    while stack:
        name = stack.pop()
        if '/' in name or name.startswith('dep:') or name in closure:
            continue
        closure.add(name)
        stack.extend(table.get(name, []))
    return closure


def scan_file(path: Path, on_by_default: set[str] = frozenset()) -> tuple[int, int, set[str]]:
    """`(tests, gated_tests, features)` for one file.

    Brace-tracked rather than line-matched: a `#[cfg(feature)] mod x { … }`
    guards everything to its closing brace, and a scanner that only looked at
    the attribute line would call the tests inside it ungated — reporting the
    exact "runs everything" that D57 warns against.
    """
    text = path.read_text(encoding='utf8', errors='replace')
    text = re.sub(r'//[^\n]*', '', text)

    features: set[str] = set()
    file_level = [m for m in CFG_FEATURE.finditer(text) if m.group(0).startswith('#![')]
    # A gate whose feature is ON BY DEFAULT hides nothing from a default run.
    file_level = [m for m in file_level if m.group(1) not in on_by_default]
    file_gated = bool(file_level)
    features.update(m.group(1) for m in file_level)

    total = 0
    gated = 0
    # Depth at which a feature gate opened, so it can be closed again.
    gate_depth: list[int] = []
    depth = 0
    pending_gate: str | None = None

    for match in re.finditer(r'#!?\[[^\]]*\]|\bmod\s+[a-zA-Z0-9_]+\s*[;{]|[{}]', text):
        token = match.group(0)
        if token.startswith('#'):
            feature = CFG_FEATURE.search(token)
            if feature and not token.startswith('#![') and feature.group(1) not in on_by_default:
                pending_gate = feature.group(1)
                features.add(feature.group(1))
            elif TEST_ATTR.match(token):
                total += 1
                if file_gated or gate_depth or pending_gate:
                    gated += 1
                pending_gate = None
            continue
        if token.startswith('mod'):
            if token.rstrip().endswith('{'):
                # the token regex SWALLOWS this brace, so it must be counted
                # here or depth never rises: the first `}` inside the module
                # then popped the gate and every test after the first read as
                # ungated. Caught by the brace-tracking test, which is why that
                # test plants TWO tests in the block rather than one.
                if pending_gate:
                    gate_depth.append(depth)
                depth += 1
                pending_gate = None
                continue
            if pending_gate:
                # `#[cfg(feature)] mod x;` — the whole child FILE is gated, and
                # this scanner cannot follow it. Counted in `features` so the
                # crate is flagged, never silently dropped.
                pass
            pending_gate = None
            continue
        if token == '{':
            depth += 1
            if pending_gate:
                gate_depth.append(depth - 1)
                pending_gate = None
        elif token == '}':
            depth -= 1
            while gate_depth and gate_depth[-1] >= depth:
                gate_depth.pop()
    return total, gated, features


GATED_MOD_DECL = re.compile(
    r'#\[\s*cfg\s*\([^)]*feature\s*=\s*"([a-zA-Z0-9_\-]+)"[^)]*\)\s*\]\s*'
    r'(?:pub(?:\([^)]*\))?\s+)?mod\s+([a-zA-Z0-9_]+)\s*;'
)


def gated_subtrees(crate: Path, on_by_default: set[str] = frozenset()) -> dict[Path, str]:
    """Files reached only through a `#[cfg(feature)] mod x;` DECLARATION.

    ⛔ **this is the difference between a useful number and a confidently wrong
    one.** Without it the scan reported `ambition_touch_input` as *39 of 45 run
    bare* while cargo measures **4 of 45** (queue D57): the crate gates
    `bevy_plugin` as a bare declaration, so every test in that module lives in
    another file the attribute never touches. A static scanner that stops at the
    declaration line reads the whole module as ungated and understates the hole
    by an order of magnitude.
    """
    gated: dict[Path, str] = {}
    for sub in ('src', 'tests'):
        base = crate / sub
        if not base.is_dir():
            continue
        for path in sorted(base.rglob('*.rs')):
            text = re.sub(r'//[^\n]*', '', path.read_text(encoding='utf8', errors='replace'))
            for feature, name in GATED_MOD_DECL.findall(text):
                if feature in on_by_default:
                    continue
                parent = path.parent
                for candidate in (parent / f'{name}.rs', parent / name / 'mod.rs'):
                    if candidate.exists():
                        gated[candidate] = feature
                folder = parent / name
                if folder.is_dir():
                    for child in folder.rglob('*.rs'):
                        gated.setdefault(child, feature)
    return gated


def scan_crate(crate: Path) -> tuple[int, int, set[str]]:
    total = gated = 0
    features: set[str] = set()
    on_by_default = default_features(crate)
    subtrees = gated_subtrees(crate, on_by_default)
    for sub in ('src', 'tests'):
        for path in sorted((crate / sub).rglob('*.rs')) if (crate / sub).is_dir() else []:
            t, g, f = scan_file(path, on_by_default)
            total += t
            if path in subtrees:
                # Reached only through a gated declaration: EVERY test in it is
                # behind that feature, whatever the file itself says.
                gated += t
                features.add(subtrees[path])
            else:
                gated += g
            features |= f
    return total, gated, features


def verify(crate: str) -> tuple[int, int]:
    """`(bare, all_features)` test counts from cargo itself — the exact answer.

    Costs two compiles of one crate. That is the price of a number you can quote,
    and the reason the survey above exists to tell you which crate to spend it
    on.
    """
    import subprocess

    def count(extra: list[str]) -> int:
        result = subprocess.run(
            ['cargo', 'test', '-p', crate, *extra, '--quiet', '--', '--list'],
            cwd=REPO_ROOT,
            capture_output=True,
            text=True,
            check=False,
        )
        return sum(1 for line in result.stdout.splitlines() if line.endswith(': test'))

    return count([]), count(['--all-features'])


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--markdown', action='store_true', help='emit the ledger table')
    parser.add_argument('--verify', metavar='CRATE', help='ask cargo for the exact pair')
    parser.add_argument(
        '--min-gated', type=int, default=1, help='only report crates hiding at least this many'
    )
    args = parser.parse_args()

    if args.verify:
        bare, everything = verify(args.verify)
        hidden = everything - bare
        print(f'{args.verify}: {bare} of {everything} tests run under a bare `cargo test -p`')
        if hidden:
            print(f'⛔ {hidden} tests ({hidden * 100 // max(everything, 1)}%) need extra features')
        else:
            print('✅ a bare run is the whole suite for this crate')
        return 0

    rows = []
    for crate in crate_dirs():
        total, gated, features = scan_crate(crate)
        if gated >= args.min_gated:
            rows.append((crate.name, total, gated, sorted(features)))
    rows.sort(key=lambda r: (-r[2], r[0]))

    if args.markdown:
        print('| crate | bare run | gated | features that gate its tests |')
        print('|---|---|---|---|')
        for name, total, gated, features in rows:
            print(f'| `{name}` | {total - gated} of {total} | {gated} | {", ".join(features)} |')
    else:
        for name, total, gated, features in rows:
            print(f'{name:44} {total - gated:4} of {total:4} run bare   ({", ".join(features)})')
        hidden = sum(r[2] for r in rows)
        print(f'\n{len(rows)} crates hide {hidden} tests behind features.')
        print(
            '⚠ counts only OPT-IN features: a gate whose feature is reachable from\n'
            '  `default` hides nothing from a default run and is not counted here.\n'
            '  Still under-counts modules gated transitively through several hops.'
        )
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
