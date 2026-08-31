"""The profiler's warm build must produce artifacts the profiled launch accepts.

⛔⛔ **THE DEFECT THIS PINS PUT THE COMPILER INSIDE THE PROFILE.**
`scripts/profile_desktop.sh` warm-builds before `perf record` so the capture is
about the game, then perf-records `run_game.sh`. But the warm build replayed the
plan's `build_arg` list with a bare `cargo build`, while `run_game.sh` exports
`CARGO_INCREMENTAL=0` for every release-level profile — deliberately, because
`.cargo/config.toml`'s `[build] incremental = true` overrides
`[profile.profiling] incremental = false`.

Two different cargo fingerprints. So the warm build warmed nothing, and the
launch rebuilt the whole graph *under* `perf record`. All three bundles of
2026-08-31 carry it: `rustc`, `ld.mold` and twenty `lto cgu.NN` threads in the
sample set, and in the worst one the game's first frame lands 275 seconds after
the capture started. Every "where did the time go" number in those bundles is
diluted by a compile nobody asked to measure.

The fix is that `--print-plan` publishes the build ENVIRONMENT alongside the
build COMMAND, and the profiler applies it. These arms guard both halves: a plan
that stops carrying it, and a profiler that stops using it.
"""

from __future__ import annotations

import subprocess
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]


def plan(*args: str) -> list[str]:
    out = subprocess.run(
        [str(REPO / "run_game.sh"), "--print-plan", *args],
        capture_output=True,
        text=True,
        check=True,
        cwd=REPO,
    )
    return out.stdout.splitlines()


def test_a_profiling_plan_carries_the_incremental_setting_the_launch_will_use():
    rows = plan("profiling", "--features", "profile")
    assert "build_profile=profiling" in rows, "premise: this must be a profiling plan"
    assert "build_env=CARGO_INCREMENTAL=0" in rows, (
        "run_game.sh exports CARGO_INCREMENTAL=0 for release-level profiles, so "
        "the plan has to say so; a caller that replays build_arg alone builds a "
        "DIFFERENT fingerprint and the real build lands inside the profile"
    )


def test_a_dev_plan_carries_no_incremental_override():
    """Premise guard: the row above is a real fact about profiling builds.

    Without this arm, a `--print-plan` that unconditionally printed
    `CARGO_INCREMENTAL=0` would pass the test above while breaking the dev edit
    loop, which incremental exists for.
    """
    rows = plan("dev")
    assert "build_profile=dev" in rows, "premise: this must be a dev plan"
    assert not [r for r in rows if r.startswith("build_env=CARGO_INCREMENTAL")], (
        "the dev profile keeps incremental on; see .cargo/config.toml"
    )


def test_the_profiler_applies_the_plans_build_environment():
    source = (REPO / "scripts" / "profile_desktop.sh").read_text(encoding="utf-8")
    assert "build_env) plan_build_env+=" in source, (
        "profile_desktop.sh must PARSE the build_env rows out of the plan"
    )
    assert "plan_env_prefix=(env " in source, (
        "and must APPLY them to the warm build; parsing them and dropping them "
        "is the same defect with a longer parser"
    )
