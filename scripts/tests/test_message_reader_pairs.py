"""The shared-pair scanner must DETECT a union, or its zero means nothing.

⛔⛔ THE REPORT IS GREEN AND EMPTY, which is the most dangerous shape a
measurement can have: `0 pairs read together by more than one system` is exactly
what a BROKEN parser prints. These tests are the positive control.

⭐ The fixture is the real pre-fix shape, reduced: four systems that each combined
`RoomLoaded` and `RoomReplayAdmitted` by hand (2026-09-05, extracted into
`ambition_combat::events::FreshAttempt`). If the scanner cannot see this, its
silence about the live tree is worthless.
"""

from __future__ import annotations

import importlib.util
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
SCRIPT = REPO / "scripts" / "message_reader_pairs.py"

HAND_WRITTEN_UNION = """
pub fn rearm_bricks_for_a_fresh_attempt(
    mut rooms: MessageReader<RoomLoaded>,
    mut replays: MessageReader<ambition_combat::events::RoomReplayAdmitted>,
    mut broken: ResMut<BrokenBricks>,
) {
}
"""

SINGLE_READER = """
pub fn only_one_message(
    mut rooms: MessageReader<RoomLoaded>,
    mut broken: ResMut<BrokenBricks>,
) {
}
"""

BUNDLED_PARAM = """
pub fn rearm_bricks_for_a_fresh_attempt(
    mut attempt: ambition_combat::events::FreshAttempt,
    mut broken: ResMut<BrokenBricks>,
) {
}
"""


def _module():
    spec = importlib.util.spec_from_file_location("message_reader_pairs", SCRIPT)
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


def _parse(tmp_path: Path, source: str):
    path = tmp_path / "sample.rs"
    path.write_text(source, encoding="utf-8")
    return list(_module().systems(path))


def test_it_sees_both_messages_of_a_hand_written_union(tmp_path) -> None:
    """⭐ THE POSITIVE CONTROL. Verified against the real pre-fix files too: the
    parser reports the 4-way `RoomLoaded + RoomReplayAdmitted` pair on the four
    systems as they stood at 84f463cfd~1."""
    (name, msgs), = _parse(tmp_path, HAND_WRITTEN_UNION)
    assert name == "rearm_bricks_for_a_fresh_attempt"
    assert msgs == ["RoomLoaded", "RoomReplayAdmitted"], (
        "the scanner must strip the path and see BOTH messages, or a shared pair "
        "is invisible to it"
    )


def test_a_single_reader_is_not_a_pair(tmp_path) -> None:
    """One message cannot be a union; counting it would inflate every pair."""
    (_, msgs), = _parse(tmp_path, SINGLE_READER)
    assert msgs == ["RoomLoaded"]


def test_a_bundled_param_is_invisible_by_design(tmp_path) -> None:
    """⚠ THE FIXED SHAPE DISAPPEARS FROM THE REPORT, and that is intended: a
    union that became a `SystemParam` is no longer assembled by hand. So a pair
    dropping off this list is evidence of a FIX, not of a regression — which is
    why the report must never be read as a guard."""
    assert _parse(tmp_path, BUNDLED_PARAM) == []


def test_the_live_scan_runs_and_finds_systems() -> None:
    """The anti-vacuity floor: the tree really does have systems taking messages,
    so `0 shared pairs` is a fact about the code and not about the parser."""
    import io
    import contextlib

    module = _module()
    out = io.StringIO()
    with contextlib.redirect_stdout(out):
        assert module.main() == 0
    text = out.getvalue()
    count = int(text.split()[0])
    assert count > 100, f"only {count} systems take a MessageReader; scan looks broken"
