"""The observer-effect table must name the biggest non-game cost in the capture.

⛔⛔ **THE DEFECT THIS PINS MADE THE TRUST SECTION THE UNTRUSTWORTHY ONE.**
A bundle's "Observer effect" table is what a reader consults to decide whether
the rest of the numbers stand. It bucketed threads on `cargo`/`rustc` and so
reported `build tooling: 4.0%` for `desktop-timeline-run-20260831T212248Z` —
whose own `perf-report-by-thread.txt` shows `ld.mold` at 9.11% plus twenty
`lto cgu.NN` / `opt cgu.NN` threads at 1.3-2.6% each. A compile was running
inside the capture and the table said it was noise.

A cargo build spawns one `rustc` per crate; the CYCLES are in the linker and the
LLVM codegen threads it forks, and none of those are named after cargo.

Every name below was taken from that bundle's real thread table, not invented.
"""

from __future__ import annotations

import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO / "scripts" / "lib"))

import profile_bundle_summary as summary  # noqa: E402

GAME = "the game itself"


def bucket(comm: str) -> str:
    """The classifier as the summary applies it — substring, case-insensitive."""
    for label, needles in summary.THREAD_BUCKETS:
        if any(needle.lower() in comm.lower() for needle in needles):
            return label
    return GAME


def test_the_linker_and_codegen_threads_are_the_compiler():
    for comm in ["ld.mold", "lto cgu.00", "opt cgu.15", "rustc", "clang"]:
        assert bucket(comm) == summary.COMPILER_THREADS, (
            f"{comm!r} is a codegen thread; counting it as the game hides a build "
            f"running inside the capture"
        )


def test_cargo_is_a_launcher_and_not_a_compile():
    """⛔⛔ The second half of the same repair, and the trust verdict turns on it.

    `desktop-perf-run-20260901T003332Z` is 88.0% game, 9.4% cargo and ZERO
    codegen threads — a clean capture whose launcher merely waited on a child
    with nothing to do. Folded into one `build tooling` bucket, that 9.4% is
    indistinguishable from a compile, and the verdict had to guess from a number
    that counts both. Only codegen dilutes a native profile.
    """
    for comm in ["cargo", "bash", "dirname"]:
        assert bucket(comm) == summary.LAUNCHER_THREADS, (
            f"{comm!r} spawns a compile; it does not do one"
        )
    assert summary.COMPILER_THREADS != summary.LAUNCHER_THREADS


def test_the_games_own_threads_are_still_the_game():
    """Premise guard: the needles above must not swallow the game.

    Without this, widening the compiler bucket until it matched everything would
    pass the test above. `sh` was a real needle here once, and would have.
    """
    for comm in ["ambition_game_b", "Compute Task Po", "IO Task Pool", "gilrs"]:
        assert bucket(comm) == GAME, f"{comm!r} is the game's own thread"


def test_tracy_and_audio_keep_their_own_buckets():
    # perf truncates COMM to 15 characters, so these are the truncated forms.
    assert bucket("Tracy Symbol Wo") == "profiler (Tracy)"
    assert bucket("pw-data-loop") == "audio", (
        "PipeWire's realtime thread never spells 'pipewire' in a truncated COMM"
    )
