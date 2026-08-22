#!/usr/bin/env python3
"""**How many tests does `cargo test -p <crate>` NOT run?**

*"The suite is green"* is not a complete sentence. A crate that gates test code
behind a feature runs a SUBSET under a bare `cargo test -p <crate>` and reports
`ok` for it — `ambition_input` runs 55 of 84, `ambition_touch_input` 4 of 45, and
the sanic spike tests passed 4/4 for a day in the one configuration where input
does not exist (queue D57).

Queue D57 recorded a 23-crate table by hand and said what it could not do:

> ⚠ this is a STATIC scan and it UNDER-COUNTS by construction … treat a crate's
> absence from this table as "not proven complete", not as "runs everything."

⭐ **this makes that table regenerable and gives it the number it was missing.**
The hand table names WHICH features gate a crate; it never says HOW MANY tests
are behind them, which is the figure that decides whether a bare run is
worthless or merely incomplete.

# # What it counts, exactly

Every `#[test]` (and `#[tokio::test]`) in a crate's `src/` and `tests/`, split by
whether it is reachable without extra features:

* a test whose own attributes include `#[cfg(feature = "x")]`;
* a test inside a `mod` declared or opened under such a `cfg`;
* every test in a file whose inner attributes include `#![cfg(feature = "x")]`,
  which is how a whole integration-test file opts itself out.

⛔ **it does NOT resolve default features or feature unification.** A feature in
a crate's `default = [...]` list is ON for a bare `cargo test`, so a test gated
behind one is counted as gated here and is actually run. That over-counts, which
is the safe direction for this tool: the failure it exists to prevent is
believing a bare run covered everything.


⭐ **an estimate nobody checked against the real thing is the same species of
claim this tool exists to correct.** Measured 2026-08-10, scan vs
`cargo test -p <crate> -- --list`:

| crate | this scan | cargo |
|---|---|---|
| `ambition_touch_input` | 4 of 45 | **4 of 45** |
| `ambition_causal` | 21 of 22 | **21 of 22** |
| `ambition_input` | 54 of 115 | 55 of 117 |

⛔ **the first draft said 10 of 45 for `ambition_touch_input`** — it over-stated
bare coverage by six, which is the UNSAFE direction. Two causes, both fixed:
`#[cfg(feature)] mod x;` declarations were not followed into their files, and the
brace tracker never counted the brace that `mod x {` swallows, so a gate closed
on the first `}` and every test after the first in a gated block read as
ungated.

⚠ the residual on `ambition_input` is two tests the regex does not see as tests
at all, and it now errs toward reporting FEWER as bare — the safe direction. It
is still an estimate. `--verify` asks cargo and prints the exact pair; the survey
says which crate is worth spending two compiles on.
"""

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


def scan_file(path: Path) -> tuple[int, int, set[str]]:
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
            if feature and not token.startswith('#!['):
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


def gated_subtrees(crate: Path) -> dict[Path, str]:
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
    subtrees = gated_subtrees(crate)
    for sub in ('src', 'tests'):
        for path in sorted((crate / sub).rglob('*.rs')) if (crate / sub).is_dir() else []:
            t, g, f = scan_file(path)
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
        print('⚠ over-counts default-on features; under-counts transitively gated modules.')
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
