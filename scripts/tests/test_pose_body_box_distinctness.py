"""Unit tests for `scripts/pose_body_box_distinctness.py` -- on a HAND-BUILT corpus.

⛔ The instrument's whole job is to tell "one box repeated" apart from "no box at
all", because those two want different fixes and the original report conflated
them. So the corpus contains one of each, plus a genuinely varied sheet, and the
tests assert the three are reported differently.
"""

from __future__ import annotations

import importlib.util
import pathlib

_SCRIPT = pathlib.Path(__file__).resolve().parents[1] / "pose_body_box_distinctness.py"
_spec = importlib.util.spec_from_file_location("pose_body_box_distinctness", _SCRIPT)
mod = importlib.util.module_from_spec(_spec)
assert _spec.loader is not None
_spec.loader.exec_module(mod)


def _sheet(body_box: str, poses: dict[str, str | None]) -> str:
    """Enough of the real `.ron` shape for the parser, in the real field order."""
    entries = []
    for name, bbox in poses.items():
        if bbox is None:
            entries.append(f'"{name}": (hurtbox: None)')
        else:
            entries.append(
                f'"{name}": (hurtbox: Some((parts: [(name: "body", {bbox})], '
                f"bbox: Some(({bbox})))))"
            )
    return (
        f"(target: \"t\", body_metrics: Some((body_pixel_bbox: Some(({body_box})), "
        "animations: {" + ", ".join(entries) + "}, authored_body: true)))"
    )


def _write(tmp_path, name: str, text: str) -> pathlib.Path:
    p = tmp_path / f"{name}_spritesheet.ron"
    p.write_text(text, encoding="utf-8")
    return p


def test_one_box_repeated_is_reported_as_flat_and_named_as_such(tmp_path):
    box = "x: 12, y: 26, w: 14, h: 21"
    p = _write(tmp_path, "flat", _sheet(box, {"idle": box, "walk": box, "jump": box}))
    row = mod.read_sheet(p)
    assert row["distinct"] == 1
    assert len(row["poses"]) == 3
    line = mod.report(row)
    assert "ONE BOX FOR EVERY POSE" in line
    # It also says the repeated box IS the sheet-level one, which is what makes
    # the fallback and the flat-generator readings look alike from outside.
    assert "identical to the sheet-level" in line


def test_absent_metrics_are_NOT_reported_as_flat(tmp_path):
    """⭐ The distinction the original report lost. These are different defects."""
    box = "x: 1, y: 2, w: 3, h: 4"
    p = _write(tmp_path, "absent", _sheet(box, {"idle": None, "walk": None}))
    row = mod.read_sheet(p)
    assert row["poses"] == {}
    line = mod.report(row)
    assert "NO per-pose hurtbox bbox at all" in line
    assert "ONE BOX FOR EVERY POSE" not in line


def test_a_varied_sheet_counts_its_distinct_boxes(tmp_path):
    p = _write(
        tmp_path,
        "varied",
        _sheet(
            "x: 0, y: 0, w: 9, h: 9",
            {
                "idle": "x: 1, y: 1, w: 4, h: 8",
                "walk": "x: 2, y: 1, w: 4, h: 8",
                "crouch": "x: 1, y: 5, w: 4, h: 4",
            },
        ),
    )
    row = mod.read_sheet(p)
    assert row["distinct"] == 3
    assert "ONE BOX FOR EVERY POSE" not in mod.report(row)


def test_a_single_pose_sheet_is_not_called_flat(tmp_path):
    """⚠ Anti-false-positive: one pose trivially has one box. That is not the defect."""
    box = "x: 1, y: 1, w: 2, h: 2"
    p = _write(tmp_path, "single", _sheet(box, {"idle": box}))
    assert "ONE BOX FOR EVERY POSE" not in mod.report(mod.read_sheet(p))


def test_a_sheet_that_barely_moves_is_flagged_even_though_it_is_not_flat(tmp_path):
    """⭐ THE CASE A `distinct == 1` FLAG MISSES, and it is a selectable fighter.

    `perfect_cellular_automaton` publishes 7 distinct boxes across 136 poses. A
    flag that fires only at 1 reads that as healthy; it is the same defect as a
    flat sheet, two orders of magnitude of poses later. The count is the tail, the
    RATIO is the defect.
    """
    poses = {f"pose_{i}": "x: 1, y: 1, w: 4, h: 8" for i in range(20)}
    poses["odd"] = "x: 2, y: 2, w: 4, h: 8"
    p = _write(tmp_path, "barely", _sheet("x: 0, y: 0, w: 9, h: 9", poses))
    row = mod.read_sheet(p)
    assert row["distinct"] == 2 and len(row["poses"]) == 21
    line = mod.report(row)
    assert "BARELY MOVES" in line
    # ⚠ And it must NOT claim to be flat: those want different conversations.
    assert "ONE BOX FOR EVERY POSE" not in line


def test_the_flag_moves_with_the_ratio_argument(tmp_path):
    """⚠ The threshold is a knob a reader can move, which is what makes the claim
    'the answer does not depend on where in the gap it sits' checkable rather than
    asserted."""
    poses = {f"pose_{i}": f"x: {i}, y: 1, w: 4, h: 8" for i in range(10)}
    poses.update({f"same_{i}": "x: 99, y: 1, w: 4, h: 8" for i in range(10)})
    p = _write(tmp_path, "mid", _sheet("x: 0, y: 0, w: 9, h: 9", poses))
    row = mod.read_sheet(p)
    assert row["distinct"] == 11 and len(row["poses"]) == 20  # ratio 0.55
    assert "⛔" not in mod.report(row, 0.5)
    assert "BARELY MOVES" in mod.report(row, 0.6)


def test_a_parts_only_pose_is_read_as_the_union_of_its_parts(tmp_path):
    """⛔⛔ THE BUG THIS SCRIPT SHIPPED WITH: requiring a literal `bbox:`.

    A disjoint-piece character publishes `parts: [...]` and NO `bbox` -- the box is
    their union, which is what the runtime derives. Requiring the field made the
    census report `noether` (123 poses, seven authored body parts each, the
    richest body data in the tree) as "NO per-pose hurtbox bbox at all". A negative
    result was a claim about the instrument.
    """
    text = (
        '(target: "t", body_metrics: Some((body_pixel_bbox: Some((x: 0, y: 0, w: 9, h: 9)), '
        'animations: {'
        '"idle": (hurtbox: Some((parts: [(name: "head", x: 10, y: 4, w: 6, h: 6), '
        '(name: "legs", x: 12, y: 20, w: 4, h: 10)]))), '
        '"walk": (hurtbox: Some((parts: [(name: "head", x: 30, y: 4, w: 6, h: 6), '
        '(name: "legs", x: 32, y: 20, w: 4, h: 10)])))'
        '}, authored_body: true)))'
    )
    p = tmp_path / "parts_spritesheet.ron"
    p.write_text(text, encoding="utf-8")
    row = mod.read_sheet(p)
    assert len(row["poses"]) == 2, "a parts-only pose must still be a pose"
    # head spans x 10..16, legs x 12..16, y 4..30 -> union x:10 y:4 w:6 h:26
    assert row["poses"]["idle"] == "x: 10, y: 4, w: 6, h: 26"
    assert row["distinct"] == 2, "two poses in different places are two boxes"


def test_an_authored_bbox_wins_over_the_parts_union(tmp_path):
    """⚠ When a sheet publishes both, the authored box is the answer -- the union
    is the FALLBACK road, not a second authority competing with it."""
    text = (
        '(target: "t", body_metrics: Some((body_pixel_bbox: Some((x: 0, y: 0, w: 9, h: 9)), '
        'animations: {'
        '"idle": (hurtbox: Some((parts: [(name: "head", x: 10, y: 4, w: 6, h: 6)], '
        "bbox: Some((x: 1, y: 2, w: 3, h: 4)))))"
        '}, authored_body: true)))'
    )
    p = tmp_path / "both_spritesheet.ron"
    p.write_text(text, encoding="utf-8")
    assert mod.read_sheet(p)["poses"]["idle"] == "x: 1, y: 2, w: 3, h: 4"


def test_the_art_witness_is_a_different_kind_of_evidence(tmp_path):
    """⭐ art/body counts DRAWN positions per body box -- the one figure here that
    is not another reading of distinctness, and so the only one that can witness
    for the others."""
    box = "x: 1, y: 1, w: 4, h: 8"
    text = (
        '(target: "t", body_metrics: Some((body_pixel_bbox: Some((x: 0, y: 0, w: 9, h: 9)), '
        'animations: {"idle": (hurtbox: Some((parts: [(name: "b", ' + box + ")], "
        "bbox: Some((" + box + ')))), "walk": (hurtbox: Some((parts: [(name: "b", '
        + box + ")], bbox: Some((" + box + ")))))}, authored_body: true)), "
        "rows: [(animation: \"idle\", rects: [(x: 0, y: 0, w: 1, h: 1, page: 0, off: (1, 1)), "
        "(x: 0, y: 0, w: 1, h: 1, page: 0, off: (2, 2)), "
        "(x: 0, y: 0, w: 1, h: 1, page: 0, off: (3, 3))])])"
    )
    p = tmp_path / "witness_spritesheet.ron"
    p.write_text(text, encoding="utf-8")
    row = mod.read_sheet(p)
    assert row["distinct"] == 1 and row["art_offsets"] == 3
    assert mod.art_vs_body(row) == 3.0, "three drawn positions over one body box"
    assert "art/body 3.0x" in mod.report(row)


def test_an_unreadable_sheet_raises_rather_than_reporting_zero(tmp_path):
    """⛔ A swallowed read error would report itself as 'no per-pose boxes'."""
    missing = tmp_path / "gone_spritesheet.ron"
    try:
        mod.read_sheet(missing)
    except OSError:
        return
    raise AssertionError("a missing sheet must raise, not parse to an empty result")
