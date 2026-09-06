"""A profile summary must reach ONE conclusion about whether to trust it.

⛔⛔ **THE DEFECT THIS PINS SHIPPED IN THREE REPORTS ON ONE DAY.** The observer
effect section asked two questions — "did a compile pollute this capture" and
"did Tracy" — and answered them in two `if` branches that could not see each
other. `desktop-timeline-run-20260831T210231Z` was 92.4% compiler and 0.1%
Tracy, so it printed both:

    The native symbol ranking and the DSO split below are diluted by it
    and must not be quoted.
        ... eleven lines later ...
    The profiler cost 0% of sampled cycles. Low enough that the
    measurements below stand on their own.

Only the first was right. The second was only ever about TRACY and was worded as
a conclusion about every measurement below it. A reader skimming for the verdict
could take away either one.

The exact combination that produced it — heavy compile, negligible profiler — is
the arm this file exists for. The verdict is a single value now, so a
contradiction has to be written deliberately into one function rather than
falling out of two that never met.
"""

from __future__ import annotations

import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO / "scripts" / "lib"))

import profile_bundle_summary as summary  # noqa: E402

# The phrase that must never appear over a contaminated capture.
STANDS = "stand on their own"


# The claim that is TRUE of a compile and FALSE of a profiler.
GAME_TIME_SURVIVES = "GAME TIME is unaffected"


def verdict(compiler, game, profiler, headless=False, launcher=0.0):
    trust = summary.native_profile_trust(compiler, game, profiler)
    return trust, "\n".join(
        summary.native_profile_trust_lines(
            trust, compiler, game, profiler, headless, launcher
        )
    )


def test_a_heavy_compile_with_a_quiet_profiler_does_not_claim_the_numbers_stand():
    """The 20260831T210231Z state, exactly: 92.4% build, 7.5% game, 0.1% Tracy."""
    trust, text = verdict(compiler=92.4, game=7.5, profiler=0.1)
    assert trust == summary.TRUST_COMPILE
    assert STANDS not in text, (
        "a capture that is 92% compiler must not tell the reader its native "
        f"attribution stands; got:\n{text}"
    )
    assert "must not be quoted" in text, "and it must say so plainly"
    assert "compile ran inside this capture" in text, "naming which contaminant"


def test_a_heavy_profiler_with_no_compile_still_refuses_the_numbers():
    trust, text = verdict(compiler=0.0, game=60.0, profiler=30.0)
    assert trust == summary.TRUST_PROFILER
    assert STANDS not in text
    assert "Tracy cost 30%" in text
    assert "compile ran inside this capture" not in text, (
        "do not accuse a contaminant that is not there"
    )


def test_both_contaminants_are_both_named():
    trust, text = verdict(compiler=50.0, game=20.0, profiler=30.0)
    assert trust == summary.TRUST_BOTH
    assert STANDS not in text
    assert "compile ran inside this capture" in text
    assert "Tracy cost 30%" in text


def test_a_clean_capture_is_the_only_one_allowed_to_say_the_numbers_stand():
    """Premise guard: without this the phrase could simply have been deleted.

    A summary that never blesses a clean capture is not a fixed summary; it is a
    summary that stopped answering the question.
    """
    trust, text = verdict(compiler=0.0, game=90.0, profiler=1.0)
    assert trust == summary.TRUST_CLEAN
    assert STANDS in text
    assert "must not be quoted" not in text


def test_the_verdict_appears_once_and_only_once():
    """Two verdicts in one report is the defect, whatever they say."""
    for compiler, game, profiler in [(92.4, 7.5, 0.1), (0.0, 60.0, 30.0), (0.0, 90.0, 1.0)]:
        _, text = verdict(compiler, game, profiler)
        verdicts = text.count("must not be quoted") + text.count(STANDS)
        assert verdicts == 1, (
            f"exactly one trust conclusion, got {verdicts} for "
            f"compiler={compiler} game={game} profiler={profiler}:\n{text}"
        )


def test_a_profiler_contaminated_report_never_says_game_time_survives():
    """⛔⛔ **THE SECOND CONTRADICTION, IN THE FUNCTION THAT FIXED THE FIRST.**

    Making the verdict one value stopped the report reaching two conclusions.
    It did not stop ONE conclusion from containing two incompatible sentences,
    and it did: every contaminated state printed

        ⭐ Everything keyed to GAME TIME is unaffected ...
            ... a few lines later ...
        every frame time, zone duration and plugin-build number here is inflated too.

    Both, for the same capture. The paragraph was only ever true of a COMPILE,
    which runs beside the game and mostly before `exec`; Tracy runs INSIDE the
    process the census is recorded by.
    """
    for compiler, game, profiler in [(0.0, 60.0, 30.0), (50.0, 20.0, 30.0)]:
        _, text = verdict(compiler, game, profiler)
        assert GAME_TIME_SURVIVES not in text, (
            "Tracy inflates the frames the game's own census records; a report "
            f"must not also promise that census is untouched:\n{text}"
        )
        assert "the same inflation the native" in text, "say what actually survives"


def test_a_compile_contaminated_report_still_says_the_census_survives():
    """Premise guard: the paragraph must be CONDITIONAL, not deleted.

    A compile really does leave `frame_times.csv` alone, and that is the most
    useful sentence in a contaminated report — it tells the reader which half of
    the bundle they can still use.
    """
    _, text = verdict(compiler=92.4, game=7.5, profiler=0.1)
    assert GAME_TIME_SURVIVES in text


def test_an_idle_cargo_launcher_is_not_a_compile():
    """⛔⛔ The 20260901T003332Z state: 88% game, 9.4% cargo, ZERO codegen.

    The old rule asked whether a combined `build tooling` bucket out-cost the
    game. It is wrong in both directions, and this is the direction that
    mislabels a GOOD capture — the one written to validate the warm-build fix.
    """
    trust, text = verdict(compiler=0.0, game=88.0, profiler=2.5, launcher=9.4)
    assert trust == summary.TRUST_CLEAN
    assert "compile ran inside this capture" not in text
    assert "9.4%" in text, "the launcher is still reported, just not as a compile"


def test_a_compile_smaller_than_the_game_is_still_a_compile():
    """The other direction the old rule got wrong: 70% game, 20% rustc.

    `build_share > game_share` calls that clean. Twenty percent of the cycles
    went somewhere other than the game, so every native percentage below is
    diluted by a fifth.
    """
    trust, _ = verdict(compiler=20.0, game=70.0, profiler=2.0)
    assert trust == summary.TRUST_COMPILE


def test_a_capture_the_game_never_appears_in_is_not_clean():
    """⛔⛔ **THE HOLE THE COMPILER-BUCKET REPAIR OPENED**, found by re-running
    every bundle in `dev/ambition_dev_measurements/profiles/` against the new
    rule rather than by reasoning about it.

    `desktop-timeline-run-20260829T020516Z` is 94.4% `cargo` and 5.6% `bash`:
    the game contributed ZERO samples. The OLD rule caught it by accident
    (`build_share > game_share`); judged on the codegen bucket alone it is 0.0%
    compiler and reads CLEAN — a verdict blessing a capture with no game in it.

    A named contaminant is more useful than this verdict, so this is the
    fallback, not an override. See the premise guard below.
    """
    trust, text = verdict(compiler=0.0, game=0.0, profiler=0.0, launcher=100.0)
    assert trust == summary.TRUST_NO_GAME
    assert STANDS not in text
    assert "nothing to rank" in text


def test_a_named_contaminant_still_wins_over_the_game_share_floor():
    """Premise guard: the floor must not swallow the actionable verdicts.

    A 92.4%-codegen capture ALSO fails the game-share floor. Reporting it as
    "not a profile of the game" is true and useless — the reader needs to be
    sent to `warm-build.status`, which only the compile verdict does.
    """
    trust, text = verdict(compiler=92.4, game=7.5, profiler=0.1)
    assert trust == summary.TRUST_COMPILE
    assert "warm-build.status" in text
