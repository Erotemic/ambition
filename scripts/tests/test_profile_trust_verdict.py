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


def verdict(build, game, profiler, headless=False):
    trust = summary.native_profile_trust(build, game, profiler)
    return trust, "\n".join(
        summary.native_profile_trust_lines(trust, build, game, profiler, headless)
    )


def test_a_heavy_compile_with_a_quiet_profiler_does_not_claim_the_numbers_stand():
    """The 20260831T210231Z state, exactly: 92.4% build, 7.5% game, 0.1% Tracy."""
    trust, text = verdict(build=92.4, game=7.5, profiler=0.1)
    assert trust == summary.TRUST_COMPILE
    assert STANDS not in text, (
        "a capture that is 92% compiler must not tell the reader its native "
        f"attribution stands; got:\n{text}"
    )
    assert "must not be quoted" in text, "and it must say so plainly"
    assert "compile ran inside this capture" in text, "naming which contaminant"


def test_a_heavy_profiler_with_no_compile_still_refuses_the_numbers():
    trust, text = verdict(build=1.0, game=60.0, profiler=30.0)
    assert trust == summary.TRUST_PROFILER
    assert STANDS not in text
    assert "Tracy cost 30%" in text
    assert "compile ran inside this capture" not in text, (
        "do not accuse a contaminant that is not there"
    )


def test_both_contaminants_are_both_named():
    trust, text = verdict(build=50.0, game=20.0, profiler=30.0)
    assert trust == summary.TRUST_BOTH
    assert STANDS not in text
    assert "compile ran inside this capture" in text
    assert "Tracy cost 30%" in text


def test_a_clean_capture_is_the_only_one_allowed_to_say_the_numbers_stand():
    """Premise guard: without this the phrase could simply have been deleted.

    A summary that never blesses a clean capture is not a fixed summary; it is a
    summary that stopped answering the question.
    """
    trust, text = verdict(build=2.0, game=90.0, profiler=1.0)
    assert trust == summary.TRUST_CLEAN
    assert STANDS in text
    assert "must not be quoted" not in text


def test_the_verdict_appears_once_and_only_once():
    """Two verdicts in one report is the defect, whatever they say."""
    for build, game, profiler in [(92.4, 7.5, 0.1), (1.0, 60.0, 30.0), (2.0, 90.0, 1.0)]:
        _, text = verdict(build, game, profiler)
        verdicts = text.count("must not be quoted") + text.count(STANDS)
        assert verdicts == 1, (
            f"exactly one trust conclusion, got {verdicts} for "
            f"build={build} game={game} profiler={profiler}:\n{text}"
        )
