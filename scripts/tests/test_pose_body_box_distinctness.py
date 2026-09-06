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


def test_an_unreadable_sheet_raises_rather_than_reporting_zero(tmp_path):
    """⛔ A swallowed read error would report itself as 'no per-pose boxes'."""
    missing = tmp_path / "gone_spritesheet.ron"
    try:
        mod.read_sheet(missing)
    except OSError:
        return
    raise AssertionError("a missing sheet must raise, not parse to an empty result")
