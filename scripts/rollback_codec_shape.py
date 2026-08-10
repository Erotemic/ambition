#!/usr/bin/env python3
"""**Did a rollback codec change shape without the wire format saying so?**

`GGRS_ROLLBACK_SCHEMA_VERSION` exists so two peers that encode different bytes
cannot believe they agree. Bumping it is a HUMAN step, and on 2026-08-10 four
codecs changed under an unchanged version in a single day — `BodyCombat` gained
`landing_lag_timer`, `AxisManeuverState` gained `jump_squat_timer`,
`AxisLocomotion` gained `jump_squat_time` and `max_air_speed`, and the deferred
transition intent gained `zone_sfx`.

⛔ **nothing caught it, and the near-miss is the interesting part.** Two guards
already watch this area and neither can see a field:

* `rollback-wire-format-is-frozen` counts stable NAMES. All 348 were unchanged —
  no registration was added, removed or renamed.
* `rollback_schema_baseline.txt` records `name / kind / type / description` per
  registration. A field inside a codec changes none of those four columns.

Both answer *"is the SET of registered things the same"*. Neither answers *"does
each thing still encode the same shape"*, which is the question a peer actually
depends on.

## What this measures

The ordered sequence of codec PRIMITIVES per file — `put_f32`, `put_bool`,
`r.u8()`, … — hashed. Adding, removing or retyping an encoded field changes the
sequence; editing a comment, renaming a local, or rewrapping a line does not. So
the guard is quiet during ordinary work and loud on exactly the edit that needs
a version bump.

⚠ **the one blind spot, stated rather than hidden**: swapping two fields of the
SAME primitive type leaves the sequence identical, so this cannot see a pure
reorder of two `f32`s. That is a real wire change and this will not catch it. It
is the narrow residue of a check that is otherwise free of false positives, and
a checker that cried wolf on every rename would be turned off within a week.

## What to do when it goes red

1. Did the BYTES a peer encodes change? Then bump `GGRS_ROLLBACK_SCHEMA_VERSION`
   and describe the change in its doc log, the way every version there does.
2. Re-record this baseline (`--record`) in the same commit.

⛔ do not re-record without step 1 unless the edit provably changed no bytes —
re-recording alone silences the guard and restores the exact hole it exists to
close.
"""

from __future__ import annotations

import argparse
import hashlib
import re
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
BASELINE = Path(__file__).resolve().parent / 'tests' / 'rollback_codec_shape.txt'

# Where the wire format is written. Anything matching the primitive patterns in
# these trees participates; a new codec file is picked up with no edit here,
# which is the property that keeps the guard from quietly narrowing.
SEARCH_ROOTS = ('crates', 'game')

# A codec primitive is either a writer call (`put_f32(`) or a reader call
# (`r.u8()`), and BOTH sides are hashed: an encode that gains a field whose
# decode does not is a corruption, not merely a version question.
WRITER = re.compile(r'\bput_[a-z0-9_]+\s*\(')
READER = re.compile(r'\br\s*\.\s*([a-z0-9_]+)\s*\(\s*\)')
SNAPSHOT_POD = re.compile(r'\bsnapshot_pod!\s*\(')
# What makes a file a CODEC rather than a file that happens to call something
# named `put_…`. See the note in `codec_files`.
CODEC_MARKER = re.compile(r'\bSnapshotState\b|\bsnapshot_pod!|\bReader<')


def codec_files() -> list[Path]:
    found: list[Path] = []
    for root in SEARCH_ROOTS:
        for path in sorted((REPO_ROOT / root).rglob('*.rs')):
            if '/target/' in str(path) or '/.claude/' in str(path):
                continue
            text = path.read_text(encoding='utf8', errors='replace')
            # ⚠ **a codec marker is required, not just a `put_*` call.**
            # `put_[a-z0-9_]+(` alone matched `put_pixel` in a room-geometry
            # RENDERER example — an unrelated file whose every edit would have
            # raised a wire-format alarm. A guard with a false positive in its
            # first run is a guard that gets ignored.
            if not CODEC_MARKER.search(text):
                continue
            if WRITER.search(text) or SNAPSHOT_POD.search(text):
                found.append(path)
    return found


def shape_of(path: Path) -> tuple[int, str]:
    """The ordered primitive sequence for one file, as (count, sha1[:16]).

    ⚠ comments are stripped FIRST. A `put_f32` named in prose — and this repo's
    codecs are heavily commented — would otherwise make a doc edit look like a
    wire change, which is the false positive that gets a checker ignored.
    """
    text = path.read_text(encoding='utf8', errors='replace')
    text = re.sub(r'//[^\n]*', '', text)
    text = re.sub(r'/\*.*?\*/', '', text, flags=re.S)
    tokens: list[str] = []
    for match in re.finditer(
        r'\bput_[a-z0-9_]+\s*\(|\br\s*\.\s*[a-z0-9_]+\s*\(\s*\)|\bsnapshot_pod!\s*\(', text
    ):
        tokens.append(re.sub(r'\s+', '', match.group(0)))
    # `snapshot_pod!` lists its fields as bare idents rather than calls, so the
    # macro body is folded in by name — otherwise a POD component could gain a
    # field with no primitive call anywhere and read as unchanged.
    for match in re.finditer(r'\bsnapshot_pod!\s*\((.*?)\)\s*;', text, flags=re.S):
        body = re.sub(r'\s+', '', match.group(1))
        tokens.append(f'pod[{body}]')
    digest = hashlib.sha1('\n'.join(tokens).encode('utf8')).hexdigest()[:16]
    return len(tokens), digest


def current() -> list[str]:
    rows = []
    for path in codec_files():
        count, digest = shape_of(path)
        rows.append(f'{path.relative_to(REPO_ROOT)}\t{count}\t{digest}')
    return rows


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--record', action='store_true', help='rewrite the baseline')
    args = parser.parse_args()

    rows = current()
    if args.record:
        BASELINE.parent.mkdir(parents=True, exist_ok=True)
        BASELINE.write_text('\n'.join(rows) + '\n', encoding='utf8')
        print(f'recorded {len(rows)} codec files → {BASELINE.relative_to(REPO_ROOT)}')
        return 0

    if not BASELINE.exists():
        print(f'no baseline at {BASELINE}; run with --record', file=sys.stderr)
        return 1
    expected = [line for line in BASELINE.read_text(encoding='utf8').splitlines() if line.strip()]
    if rows == expected:
        print(f'{len(rows)} rollback codec files unchanged in shape')
        return 0

    before = {line.split('\t')[0]: line for line in expected}
    after = {line.split('\t')[0]: line for line in rows}
    print('⛔ a rollback codec changed SHAPE:', file=sys.stderr)
    for name in sorted(set(before) | set(after)):
        if before.get(name) != after.get(name):
            print(f'  - {before.get(name, "(absent)")}', file=sys.stderr)
            print(f'  + {after.get(name, "(absent)")}', file=sys.stderr)
    print(
        '\nIf the bytes a peer encodes changed, bump GGRS_ROLLBACK_SCHEMA_VERSION\n'
        'and describe it in the version log, then re-record with:\n'
        '  python3 scripts/rollback_codec_shape.py --record',
        file=sys.stderr,
    )
    return 1


if __name__ == '__main__':
    raise SystemExit(main())
