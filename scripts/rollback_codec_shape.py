#!/usr/bin/env python3
"""Guard rollback codec shape against an unchanged wire-format version.

The checker hashes the ordered codec primitives used by rollback readers and
writers. Adding, removing, or changing the primitive type of an encoded field
changes the baseline while comment and local-name edits do not. A pure reorder
of adjacent fields with the same primitive type is not detectable by this
method.

When encoded bytes change, bump `GGRS_ROLLBACK_SCHEMA_VERSION` and record the new
baseline in the same change. Do not re-record merely to silence a real wire
change."""

from __future__ import annotations

import argparse
from functools import cache
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
#
# ⛔ `SnapshotCursor` WAS MISSING AND THAT LOST WHOLE FILES, found 2026-09-03.
# A file whose only codec impls are `impl SnapshotCursor for …` matched none of
# the three markers, so it was not in the population at all — its encoders could
# change shape and nothing would say so. Two such files existed:
# `ambition_sprite_sheet/src/snapshot_impls.rs` (long-standing) and
# `ambition_body_seed/src/snapshot_impls.rs`, which D33 cut 1 created by moving
# the `ActorMotionPath` cursor impl out of the monolith under the orphan rule.
# ⚠ The move is exactly how it surfaced: the monolith's file shrank and its hash
# moved, and the impl's new home was invisible — a codec can LEAVE the ledger by
# being carved into a crate the marker does not recognise.
CODEC_MARKER = re.compile(
    r'\bSnapshotState\b|\bSnapshotCursor\b|\bsnapshot_pod!|\bReader<'
)


@cache
def codec_files() -> list[Path]:
    found: list[Path] = []
    for root in SEARCH_ROOTS:
        for path in sorted((REPO_ROOT / root).rglob('*.rs')):
            if '/target/' in str(path) or '/.claude/' in str(path):
                continue
            text = path.read_text(encoding='utf8', errors='replace')
            # a codec marker is required, not just a `put_*` call.
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
    # AND AN ARRAY-DRIVEN CODEC HIDES THE SAME WAY A POD DID.
    #
    # this is the same hole `snapshot_pod!` was already patched for, in the
    # comment right above: *"otherwise a POD component could gain a field with no
    # primitive call anywhere and read as unchanged."* One construct had been
    # noticed and the other had not.
    #
    # the COUNT, never the names — a renamed field is not a wire change, and a
    # guard that fired on renames is the false positive this file's own docs say
    # gets a checker turned off.
    for match in re.finditer(r'\bfor\s+\w+\s+in\s*\[([^\[\]]*?)\]\s*\{', text, flags=re.S):
        body = match.group(1).strip().rstrip(',')
        if not body:
            continue
        tokens.append(f'arr[{len(body.split(",")) }]')
    for match in re.finditer(r'\[\s*(?:false|true|0u8|0u16|0u32|0u64|0i32|0f32|0\.0)\s*;\s*(\d+)\s*\]', text):
        tokens.append(f'fixed[{match.group(1)}]')
    # AND `snapshot_unit_enum!` IS THE THIRD CONSTRUCT TO HIDE THE SAME WAY.
    #
    # the file already documented the first two holes and this is the same
    # sentence a third time — a POD gaining a field with no primitive call, an
    # array-driven codec widening with no primitive call, and now a wire CODE
    # arriving with no primitive call.  when a construct decides bytes without
    # naming a `put_*`, it has to be folded in by hand; there is no general rule
    # here, only the list.
    #
    # the DISCRIMINANTS, SORTED — never the names and never the order. A
    # rename is not a wire change and neither is reordering the variant list,
    # because each variant keeps its own code; a checker that fires on either is
    # the false positive this file's own docs say gets it turned off. An ADDED,
    # REMOVED or RENUMBERED code is a wire change and moves the sorted set.
    for match in re.finditer(r'\bsnapshot_unit_enum!\s*\((.*?)\)\s*;', text, flags=re.S):
        codes = sorted(int(code) for code in re.findall(r'=\s*(\d+)', match.group(1)))
        tokens.append(f'unit_enum[{len(codes)}:{",".join(str(c) for c in codes)}]')
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
