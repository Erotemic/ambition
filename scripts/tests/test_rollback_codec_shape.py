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


def test_a_planted_wire_code_moves_the_shape(tmp_path):
    """⛔⛔ **the THIRD construct that decides bytes without naming a `put_*`.**

    `snapshot_unit_enum!(Ty { A = 0, B = 1 })` expands to one `put_u8` of a
    discriminant, and that call lives in the MACRO — so an invocation site
    contributes no primitive at all and a new wire CODE was invisible. Measured
    2026-08-20 on a branch adding `MovementOp::Footstool = 34`: the file stayed
    byte-identical at `3355eb410d168f88` while the enum went 34 variants to 35.

    ⚠ **the COUNT does not move, the DIGEST does** — one token per invocation,
    carrying the sorted codes. Same as `snapshot_pod!` and the array fold, and
    the same trap for a reader skimming counts.
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
