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
    after = [
        s["ms"]
        for s in tells["spikes"]
        if s["stamp"] is not None and s["stamp"] > tells["transitions"][-1]["at"]
    ]
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


# ── Robustness I asserted before I had checked it ─────────────────────────
#
# ⛔⛔ THIS PARSER WAS WRITTEN FROM THE EMITTER SOURCE AND VALIDATED ONLY ON
# FIXTURES I AUTHORED FROM THE SAME SOURCE. That is exactly how
# `measure_first_room_manifest.py`'s first parser came to read NOTHING from a
# real capture: it required a column the capture predated and took the demand
# road from the wrong position. Both parsers read the same game's stderr.
#
# These pin the two properties this one depends on and had never been tested:
# a real run's lines come through `tracing`, which wraps its prefix in ANSI
# escapes, and a raw `2>` capture carries no `[  N.NNNs]` stamp at all.

TRACING_PREFIX = (
    "\x1b[2m2026-09-02T18:40:23.341075Z\x1b[0m \x1b[32m INFO\x1b[0m "
    "\x1b[2mambition_platformer2d::room_transition::performance\x1b[0m\x1b[2m:\x1b[0m "
)
REAL_TRANSITION = (
    "room transition 1 hub -> hall_of_characters: construction_preflight_ms=Some(1.0) "
    "asset_manifest_ms=Some(2.0) asset_wait_ms=Some(3.0) ready_ms=Some(9.0) "
    "cover_present_ms=Some(4.0) commit_enqueue_ms=Some(1.0) "
    "commit_to_first_frame_ms=Some(2.0) loading_visible_ms=0.000 covered=true "
    "prefetch_hit=false loading_visible=false"
)


def test_ansi_coloured_tracing_output_still_parses():
    """A real run's `info!`/`warn!` lines are coloured by the tracing
    subscriber. The stamper strips ANSI; a raw capture does not."""
    plain = summary.room_reveal_tells(REAL_TRANSITION + "\n")
    coloured = summary.room_reveal_tells(TRACING_PREFIX + REAL_TRANSITION + "\n")
    assert len(plain["transitions"]) == 1, "premise: the bare message parses"
    assert len(coloured["transitions"]) == 1, (
        "an ANSI-coloured line must parse too — the escapes wrap the prefix, "
        "and the fields are searched rather than matched from line start"
    )
    assert coloured["transitions"][0]["asset_wait_ms"] == 3.0


def test_an_ansi_coloured_warning_is_still_counted():
    warn = (
        "\x1b[33m WARN\x1b[0m ambition_platformer2d::sprites: actor 'npc_x' resolved "
        "no sprite and is drawing the placeholder rectangle: declared as 'g' but "
        "never materialized — no realization of it has ever been resident"
    )
    tells = summary.room_reveal_tells(warn + "\n")
    assert tells["placeholders"]["never materialized"] == 1


def test_an_unstamped_capture_refuses_to_place_spikes_rather_than_guessing():
    """⛔ Without the `[  N.NNNs]` stamp there is no time on the transition
    line, so 'after the reveal' is unanswerable. Counting every spike in the
    run would blame the transition for the whole boot — the report says so and
    omits the number instead."""
    unstamped = (
        "INFO room transition 1 hub -> hall: asset_wait_ms=Some(3.0) covered=true\n"
        "[frame-spike]   11.000s   210.0ms\n"
    )
    text = "\n".join(summary.room_reveal_lines(unstamped))
    assert "no `[  N.NNNs]` stamps" in text, text
    assert "Frames over" not in text, (
        "an unplaceable spike count must be omitted, not printed as if the "
        "boundary were known"
    )


# ── Lines taken verbatim from the first real host bundle ──────────────────
#
# `desktop-timeline-run-20260902T215256Z`, the first host capture carrying the
# `f NNN` frame stamp and the first-room lines. Every fixture above this point
# was authored from the emitter source; these are copied out of a capture.
#
# ⛔⛔ AND THEY CARRY THE DEFECT THAT ONLY REAL DATA EXPOSED: a `[frame-spike]`
# line holds TWO clocks — the stamper's wall time in the `[   N.NNNs]` prefix
# and the GAME's own elapsed time in the body — and on this bundle they are
# 1.3 s apart. The transition line is a `tracing` record with no game clock, so
# the stamp is the only quantity both share. Ordering a spike's GAME time
# against a transition's STAMP compares different origins.
HOST_LINES = """\
[    2.386s] [frame-spike]    1.071s   125.3ms
[    2.589s] [frame-spike]    1.274s   203.3ms
[    5.334s] [frame-spike]    4.018s   122.8ms
[    5.174s] [first-room-art] room 'central_hub_complex' ready after 29 updates (2 of them waiting only on GPU uploads): 12 assets, 4 characters
[    7.767s] [world-event]    6.452s f   1864 room-transition begin seq=1 central_hub_complex -> hall_of_characters covered=true
[    8.066s] [world-event]    6.750s f   1919 room-loaded hall_of_characters
[    8.321s] 2026-09-02T21:53:06.229701Z  INFO ambition_platformer2d::room_transition::performance: room transition 1 central_hub_complex -> hall_of_characters: construction_preflight_ms=Some(2.333752) asset_manifest_ms=Some(1.357278) asset_wait_ms=Some(292.033299) ready_ms=Some(295.7) cover_present_ms=Some(4.0) commit_enqueue_ms=Some(1.0) commit_to_first_frame_ms=Some(2.0) loading_visible_ms=0.000 covered=true prefetch_hit=false loading_visible=false
"""


def test_the_real_host_bundle_lines_parse_to_what_the_raw_log_says():
    """Hand-checked against the capture: 0 placeholder warnings, one
    transition with asset_wait_ms=292.03 and covered=true, three spikes."""
    tells = summary.room_reveal_tells(HOST_LINES)
    assert sum(tells["placeholders"].values()) == 0
    assert len(tells["transitions"]) == 1
    move = tells["transitions"][0]
    assert (move["source"], move["target"]) == ("central_hub_complex", "hall_of_characters")
    assert move["asset_wait_ms"] == 292.033299
    assert move["covered"] is True
    assert move["at"] == 8.321, (
        "the transition's time is the COMPLETION line's stamp, not the "
        "`room-transition begin` marker three lines earlier — reading the "
        "begin line gives 7.767 and dates the reveal too early"
    )
    assert len(tells["spikes"]) == 3


def test_spikes_are_ordered_on_the_stamp_not_the_games_own_clock():
    """⛔ THE DEFECT REAL DATA FOUND. Both clocks put every spike before the
    reveal on THIS bundle, so the verdict was right by luck; a run with a spike
    between the two clocks' 1.3 s offset would have been misreported."""
    tells = summary.room_reveal_tells(HOST_LINES)
    stamps = sorted(s["stamp"] for s in tells["spikes"])
    games = sorted(s["game"] for s in tells["spikes"])
    assert stamps == [2.386, 2.589, 5.334]
    assert games == [1.071, 1.274, 4.018]
    assert stamps != games, (
        "premise: the two clocks really do differ on real output, which is what "
        "makes ordering on the wrong one a defect rather than a style choice"
    )

    text = "\n".join(summary.room_reveal_lines(HOST_LINES))
    assert "AFTER the last transition was logged (t=8.321s): **0**" in text, text


def test_a_spike_the_two_clocks_disagree_about_is_placed_by_the_stamp():
    """⛔⛔ THE ARM THAT ACTUALLY PINS THE FIX, and the first version of this
    file did not have it. On the real bundle both clocks put every spike before
    the reveal, so ordering on the wrong one gave the right answer and poisoning
    the fix left the tests GREEN. This spike is stamped AFTER the transition
    (9.000 > 8.321) while its game clock reads BEFORE it (7.700 < 8.321) — the
    1.3 s offset the capture actually shows. Only the stamp can place it.
    """
    lines = HOST_LINES + "[    9.000s] [frame-spike]    7.700s   150.0ms\n"
    tells = summary.room_reveal_tells(lines)
    late = [s for s in tells["spikes"] if s["ms"] == 150.0][0]
    assert late["stamp"] > 8.321 and late["game"] < 8.321, (
        "premise: this spike is after the reveal by the stamp and before it by "
        "the game clock — if that stops being true the test proves nothing"
    )

    text = "\n".join(summary.room_reveal_lines(lines))
    assert "**1**, worst 150.0 ms" in text, (
        "a spike after the reveal must be COUNTED; ordering on the game clock "
        "hides it, which is exactly the failure this tell exists to catch\n" + text
    )


def test_the_host_capture_meets_all_three_reveal_tells():
    """⭐ THE CAMPAIGN'S OWN ACCEPTANCE, on the first host run that could answer
    it: zero placeholder warnings (was 111), a cover that waited (292 ms, was
    3), and no >33.4 ms frame after the reveal (was nine, 89-355 ms)."""
    tells = summary.room_reveal_tells(HOST_LINES)
    text = "\n".join(summary.room_reveal_lines(HOST_LINES))
    assert sum(tells["placeholders"].values()) == 0
    assert tells["transitions"][0]["asset_wait_ms"] > 100
    assert "**0**" in text
