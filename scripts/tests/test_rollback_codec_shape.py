"""Tests that the rollback codec-shape guard can observe encoded fields.

The scan must find codec files, and adding a representative encoded field must
change the recorded shape. A guard that scans nothing or cannot respond to a
field change must not report success."""

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


def test_a_planted_wire_code_moves_the_shape(tmp_path):
    """Unit-enum discriminant codes must contribute to the codec-shape digest.

    The macro contains the `put_u8`, so invocation sites do not change primitive
    counts when a new code is added. The digest therefore includes sorted enum
    codes explicitly.
    """
    path = tmp_path / 'codec.rs'
    path.write_text(
        'snapshot_unit_enum!(crate::movement::MovementOp {\n'
        '    Jump = 1,\n'
        '    Land = 2,\n'
        '});\n',
        encoding='utf8',
    )
    bare = shape.shape_of(path)

    path.write_text(
        'snapshot_unit_enum!(crate::movement::MovementOp {\n'
        '    Jump = 1,\n'
        '    Land = 2,\n'
        '    Footstool = 3,\n'
        '});\n',
        encoding='utf8',
    )
    assert shape.shape_of(path)[1] != bare[1], (
        'a new wire code did not move the digest — a peer can now decode a byte '
        'the baseline says nothing about'
    )

    path.write_text(
        'snapshot_unit_enum!(crate::movement::MovementOp {\n'
        '    Jump = 1,\n'
        '    Land = 9,\n'
        '});\n',
        encoding='utf8',
    )
    assert shape.shape_of(path)[1] != bare[1], 'a RENUMBERED code did not move the digest'


def test_renaming_or_reordering_a_wire_code_does_not_move_the_shape(tmp_path):
    """The poison for the guard above. Each variant keeps its own discriminant,
    so neither a rename nor a reorder changes a byte — and a checker that fires
    on either is the false positive this file's docs say gets it turned off."""
    path = tmp_path / 'codec.rs'
    path.write_text(
        'snapshot_unit_enum!(crate::movement::MovementOp {\n'
        '    Jump = 1,\n'
        '    Land = 2,\n'
        '});\n',
        encoding='utf8',
    )
    bare = shape.shape_of(path)

    path.write_text(
        'snapshot_unit_enum!(crate::movement::MovementOp {\n'
        '    Hop = 1,\n'
        '    Land = 2,\n'
        '});\n',
        encoding='utf8',
    )
    assert shape.shape_of(path) == bare, 'a RENAMED variant moved the shape'

    path.write_text(
        'snapshot_unit_enum!(crate::movement::MovementOp {\n'
        '    Land = 2,\n'
        '    Jump = 1,\n'
        '});\n',
        encoding='utf8',
    )
    assert shape.shape_of(path) == bare, 'a REORDERED variant list moved the shape'


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
