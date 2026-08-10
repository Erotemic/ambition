"""The wire format's shape guard must stay able to see a field.

⛔ **the hole this closes was found by falling into it.** On 2026-08-10 four
rollback codecs gained fields under an unchanged `GGRS_ROLLBACK_SCHEMA_VERSION`,
and both existing guards were structurally blind: one counts stable registration
NAMES, the other records `name / kind / type / description` per registration.
Neither can see inside a codec, so a peer could encode different bytes for the
same 348 names and believe it agreed.

Only two things are pinned here, and each is a way this guard could report a
confident pass that is false:

* it still MATCHES the codec files (a scan that finds nothing is a silent pass);
* a planted field CHANGES the recorded shape (a hash that cannot move is a
  silent pass too).
"""

from __future__ import annotations

import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent.parent
sys.path.insert(0, str(REPO_ROOT / 'scripts'))

import rollback_codec_shape as shape  # noqa: E402


def test_the_scan_still_finds_the_codecs():
    """A scan that matches nothing passes forever. The known codec homes must be
    in it — `motion_codec.rs` above all, since it is one of the files whose
    silent change prompted this guard."""
    files = {str(p.relative_to(REPO_ROOT)) for p in shape.codec_files()}
    assert len(files) >= 15, f'the codec scan collapsed to {len(files)} files: {sorted(files)}'
    assert 'crates/ambition_platformer2d_core/src/motion_codec.rs' in files
    assert any(f.endswith('snapshot_impls.rs') for f in files), 'no snapshot_impls in the scan'


def test_the_baseline_matches_the_tree():
    """The guard itself, run as a test so the suite carries it."""
    assert shape.main.__module__  # keep the import meaningful under -O
    rows = shape.current()
    expected = [
        line
        for line in shape.BASELINE.read_text(encoding='utf8').splitlines()
        if line.strip()
    ]
    assert rows == expected, (
        'a rollback codec changed shape. If the bytes a peer encodes changed, bump '
        'GGRS_ROLLBACK_SCHEMA_VERSION and describe it in the version log, then '
        're-record: python3 scripts/rollback_codec_shape.py --record'
    )


def test_a_planted_field_moves_the_shape(tmp_path):
    """⛔ **the fire path.** A hash that cannot move is the same failure as a
    scan that finds nothing: the report says unchanged and everybody believes
    it. One added `put_f32` must change both the count and the digest."""
    before = tmp_path / 'codec.rs'
    before.write_text(
        'impl SnapshotState for X {\n'
        '  fn encode(&self, out: &mut Vec<u8>) {\n'
        '    put_f32(out, self.a);\n'
        '    put_bool(out, self.b);\n'
        '  }\n'
        '}\n',
        encoding='utf8',
    )
    count_a, digest_a = shape.shape_of(before)

    before.write_text(
        'impl SnapshotState for X {\n'
        '  fn encode(&self, out: &mut Vec<u8>) {\n'
        '    put_f32(out, self.a);\n'
        '    put_f32(out, self.planted);\n'
        '    put_bool(out, self.b);\n'
        '  }\n'
        '}\n',
        encoding='utf8',
    )
    count_b, digest_b = shape.shape_of(before)
    assert count_b == count_a + 1, 'the planted field did not change the primitive count'
    assert digest_b != digest_a, 'the planted field did not change the digest'


def test_a_comment_edit_does_not_move_the_shape(tmp_path):
    """The poison. A guard that fires on prose gets switched off, and these
    codecs are among the most heavily commented files in the repo — the word
    `put_f32` appears in their doc blocks."""
    path = tmp_path / 'codec.rs'
    body = (
        'impl SnapshotState for X {\n'
        '  fn encode(&self, out: &mut Vec<u8>) {\n'
        '{comment}'
        '    put_f32(out, self.a);\n'
        '  }\n'
        '}\n'
    )
    path.write_text(body.replace('{comment}', ''), encoding='utf8')
    bare = shape.shape_of(path)
    path.write_text(
        body.replace('{comment}', '    // a put_f32 named in prose, and a rewrap.\n'),
        encoding='utf8',
    )
    assert shape.shape_of(path) == bare, 'a comment mentioning put_f32 moved the shape'
