"""The hall-entry campaign's three host tells, checked instead of read.

`asset-preparation-and-residency.md` names them and says they are "still owed":
zero placeholder warnings at the reveal, `asset_wait_ms` in the seconds, and no
frames over 33.4 ms after the cover lifts. Nothing parsed them, so a host
capture had to be read by eye and judged — which is how the 2026-09-01 capture's
111 warnings became "the main evidence about a cause nobody checked".

The BEFORE fixture is that capture's shape: 111 placeholder warnings,
`asset_wait_ms=3` (the cover waited on one already-cached character), and nine
spikes of 89-355 ms AFTER the transition line. The AFTER fixture is what the
fix at `2c8f27b32` claims. A checker that cannot tell these two apart is not
worth running.

⛔ THE DIAGNOSIS SPLIT IN THREE and the campaign's phrasing predates it. A
RETIRED sheet was demanded AND decoded, so counting it beside "never
materialized" is the conflation `retired_tier` exists to end.
"""

from __future__ import annotations

import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO / "scripts" / "lib"))

import profile_bundle_summary as summary  # noqa: E402

PLACEHOLDER = (
    "actor 'npc_{i}' resolved no sprite and is drawing the placeholder "
    "rectangle: declared as 'goblin' but never materialized — no realization of "
    "it has ever been resident, so nothing has decoded its sheet"
)
RETIRED = (
    "actor 'npc_r' resolved no sprite and is drawing the placeholder rectangle: "
    "declared as 'goblin' and RETIRED from Quarter — it was decoded and then "
    "dropped by a quality transition, so this is a re-realization that has not "
    "happened yet, not art nobody asked for"
)
TRANSITION = (
    "room transition 1 hub -> hall_of_characters: construction_preflight_ms="
    "Some(1.0) asset_manifest_ms=Some(2.0) asset_wait_ms={wait} ready_ms="
    "Some(9.0) cover_present_ms=Some(4.0) commit_enqueue_ms=Some(1.0) "
    "commit_to_first_frame_ms=Some(2.0) loading_visible_ms=0.000 covered=true "
    "prefetch_hit=false loading_visible=false"
)


def stamped(at: float, body: str) -> str:
    return f"[{at:9.3f}s] {body}"


def before_log() -> str:
    """The 2026-09-01 capture: art arrived after the cover lifted."""
    lines = [stamped(10.0 + i * 0.001, PLACEHOLDER.format(i=i)) for i in range(111)]
    lines.append(stamped(10.5, TRANSITION.format(wait="Some(3.0)")))
    for n, ms in enumerate([89.0, 120.0, 355.0, 210.0, 99.0, 140.0, 175.0, 91.0, 260.0]):
        lines.append(stamped(11.0 + n * 0.1, f"[frame-spike] {11.0 + n * 0.1:8.3f}s {ms:7.1f}ms"))
    return "\n".join(lines) + "\n"


def after_log() -> str:
    """What the reveal-barrier fix claims: the hitches moved under the cover."""
    lines = [
        stamped(9.0, "[frame-spike]    9.000s   210.0ms"),  # under the cover
        stamped(10.5, TRANSITION.format(wait="Some(2841.0)")),
    ]
    return "\n".join(lines) + "\n"


def test_the_before_capture_is_reported_as_the_failure_it_was():
    # ⛔ ASSERT ON THE PARSE, NOT THE PROSE. The first version of this test
    # checked `"111" in text` — and the section's own explanatory sentence
    # carried the digits 111 and 355, so it passed on static text and would have
    # passed with the parser returning nothing. The prose no longer spells the
    # numbers, and these assertions go through the parser.
    tells = summary.room_reveal_tells(before_log())
    assert tells["placeholders"]["never materialized"] == 111
    assert tells["transitions"][-1]["asset_wait_ms"] == 3.0, "the cover did not wait"
    after = [ms for at, ms in tells["spikes"] if at > tells["transitions"][-1]["at"]]
    assert len(after) == 9, "nine hitches after the cover lifted"
    assert max(after) == 355.0

    text = "\n".join(summary.room_reveal_lines(before_log()))
    assert "total                 111" in text, f"the count reaches the reader\n{text}"
    assert "**9**" in text and "worst 355.0 ms" in text, text


def test_the_after_capture_clears_all_three_tells():
    text = "\n".join(summary.room_reveal_lines(after_log()))
    assert "total                   0" in text, f"no placeholder rectangles\n{text}"
    assert "**0**" in text, "no spikes after the reveal"
    assert "2841" in text, "the cover held for seconds while art arrived"
    tells = summary.room_reveal_tells(after_log())
    assert tells["transitions"][-1]["asset_wait_ms"] == 2841.0
    assert sum(tells["placeholders"].values()) == 0


def test_a_spike_under_the_cover_is_not_counted_against_the_reveal():
    """The whole point of a cover: hitches before the reveal are cover time."""
    text = "\n".join(summary.room_reveal_lines(after_log()))
    assert "**0**" in text, (
        "the 210 ms spike at t=9.0 precedes the transition at t=10.5 and must "
        "not be blamed on the reveal — counting every spike in the run would "
        "make the fix unmeasurable"
    )


def test_a_retired_sheet_is_not_counted_as_never_materialized():
    log = stamped(1.0, RETIRED) + "\n" + stamped(2.0, TRANSITION.format(wait="Some(5.0)")) + "\n"
    tells = summary.room_reveal_tells(log)
    assert tells["placeholders"]["retired"] == 1
    assert tells["placeholders"]["never materialized"] == 0, (
        "a retired sheet WAS demanded and WAS decoded; counting it as 'nothing "
        "decoded its sheet' is the conflation retired_tier was added to end"
    )


def test_the_spike_cap_is_reported_as_a_floor():
    """⛔ 60 logged spikes is a cap. A count that hides it reads as a total."""
    log = (
        stamped(10.0, TRANSITION.format(wait="Some(5.0)"))
        + "\n"
        + "\n".join(
            stamped(11.0 + n * 0.01, f"[frame-spike] {11.0 + n * 0.01:8.3f}s   50.0ms")
            for n in range(60)
        )
        + "\n"
        + stamped(
            11.6,
            "[frame-spike]   11.600s reached 60 logged spikes; further per-frame "
            "lines suppressed (percentile summaries continue)",
        )
        + "\n"
    )
    text = "\n".join(summary.room_reveal_lines(log))
    assert "FLOOR" in text, f"a capped count must say it is a floor\n{text}"
    assert "**60**" in text, "and still report what it saw"
    tells = summary.room_reveal_tells(log)
    assert len(tells["spikes"]) == 60, "the cap NOTICE is not itself a frame"

    # ⛔⛔ I GAVE THE REASON WRONG TWICE, WHICH IS WHY THIS ASSERTS THE OUTCOME.
    # First I said the explicit `continue` excludes the cap notice — poisoning it
    # away left the test green. Then I said the trailing `ms` was load-bearing —
    # dropping THAT left it green too. What actually excludes the notice is that
    # a WORD ("reached") follows the timestamp where the pattern needs a number.
    # Each poison I could think of was survivable by a mechanism I had not
    # noticed, so the guard pins what the reader gets, not how it is achieved.
    assert not summary.FRAME_SPIKE.search(
        "[frame-spike]   11.600s reached 60 logged spikes; further per-frame "
        "lines suppressed (percentile summaries continue)"
    ), "a line that is not a frame measurement must not parse as one"


def test_a_run_with_no_transition_and_no_warnings_prints_nothing():
    assert summary.room_reveal_lines("[    1.000s] nothing to see\n") == [], (
        "a capture that never changed rooms must not grow an empty section"
    )
