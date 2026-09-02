#!/usr/bin/env python3
"""Ambition test runner and front door to the Cargo/Python suite.

The default backbone runs repository-coupled Python checks plus one
`cargo test --workspace` build graph. Detached maintainer-tool tests and periodic
repository-maintenance checks are opt-in. The exhaustive feature-gated plan is
available explicitly but is not the default development loop.

Useful forms::

    ./run_tests.sh
    ./run_tests.sh --rust
    ./run_tests.sh --tool-tests
    ./run_tests.sh --maintenance
    ./run_tests.sh -p <crate>
    ./run_tests.sh -k <substring>
    ./run_tests.sh --list
    ./run_tests.sh --run-everything-you-probably-dont-need-this
    ./run_tests.sh --heavy

Every run writes `target/run_tests_status.json` (or `--status-json PATH`) with a
state of `running`, `done`, or `crashed`, plus its pid and final tally. External
waiters should read that status file rather than process-scan for `run_tests.py`.
An empty job plan or unknown package is an error; the process exits nonzero when
any selected job fails."""
from __future__ import annotations

import argparse
import json
import os
import re
import subprocess
import sys
import time
import tomllib
from dataclasses import dataclass, field
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
# External waiters read this state file; process-name polling can match the waiter itself.
STATUS_NAME = "run_tests_status.json"
# Share the repository disk-headroom policy rather than duplicating it in this runner.
from check_disk_headroom import MIN_FREE_GB, free_gb_on_target, target_dir  # noqa: E402

# Keep shared measurement paths in the small dependency-free helper.
sys.path.insert(0, str(REPO / "scripts" / "lib"))
import measurement_paths  # noqa: E402

CARGO = os.path.expanduser("~/.cargo/bin/cargo")
if not os.path.exists(CARGO):
    CARGO = "cargo"

# ⭐⭐ RUN THE COMPILED TESTS UNDER `cargo nextest` WHEN IT IS INSTALLED, and
# what that buys is DIAGNOSIS rather than raw speed. Measured 2026-08-27 on the
# app suite: 265s under libtest and 265s under nextest, because ONE test spends
# 164 of them and both runners are bounded by it. libtest reports a total, so
# that number had never been attributed to a test; nextest names it while it
# runs.
#
# ⛔ IT DOES NOT RUN DOCTESTS. Nothing goes red when a runner silently stops
# covering a class of test, which is exactly what happened here in the other
# direction: the suite ran NO doctests at all, and
# `ambition_sim_harness`'s had been failing to compile the whole time. The
# doctest job below is not optional garnish.
#
# ⛔ OPTIONAL, NOT REQUIRED. A contributor without nextest still gets the same
# verdict from libtest; a tool the suite REFUSES to run without is a tool
# everybody has to install to read one number.
NEXTEST = os.path.exists(os.path.expanduser("~/.cargo/bin/cargo-nextest"))


def cargo_test(args: list[str], libtest: list[str]) -> list[str]:
    """One `cargo test`-shaped invocation, routed through nextest when present.

    `args` is everything between the subcommand and the `--`; `libtest` is what
    would follow it. nextest takes a bare substring as a filter and spells the
    ignored lane `--run-ignored`, so the two libtest flags this suite actually
    passes are translated rather than forwarded.
    """
    if not NEXTEST:
        return [CARGO, "test", *args, *(["--"] + libtest if libtest else [])]
    translated: list[str] = []
    filters: list[str] = []
    for flag in libtest:
        if flag in ("--include-ignored",):
            translated += ["--run-ignored", "all"]
        elif flag == "--ignored":
            translated += ["--run-ignored", "only"]
        elif flag == "--nocapture":
            translated += ["--no-capture"]
        elif flag.startswith("-"):
            # An unrecognised libtest flag is a reason to use libtest, not to
            # guess: forwarding it to a runner that does not know it turns a
            # filter into a hard error somebody has to decode.
            return [CARGO, "test", *args, "--", *libtest]
        else:
            filters.append(flag)
    return [CARGO, "nextest", "run", *args, *translated, *filters]

# Features that cannot run on a headless desktop test host: other-platform
# selectors, wasm/web, and static-asset embedding (needs generated assets).
# Everything else (visible, input, ui, audio, kira, portal*, dev_tools,
# basic_presentation, falling_sand, mobile_touch, ...) is headless-safe here --
# the suite already exercises them via default features.
DENY_EXACT = {
    "default",
    "android", "android_dev", "android_platform",
    "web", "web_platform", "web_served", "web_served_assets", "web_audio",
    "visible_web", "visible_web_base", "visible_web_served",
    "static_map", "static_core_assets", "static_sfx_bank",
    "dev_hot_reload",
    # `headless` swaps in a windowless render path; the default `visible` graph
    # already runs headlessly in tests, and enabling both double-registers
    # render setup. No test gates on `headless`, so denying it loses nothing.
    "headless",
    # `profile` forwards Tracy, whose initializer can abort before libtest on CPUs without an
    # invariant TSC. No test is gated on this feature, so exclude it from headless feature jobs.
    "profile",
}
DENY_PREFIX = ("android", "web", "visible_web", "static_")

# Tests owned by self-contained maintainer tools do not belong to the
# repo-wide correctness gate: ordinary engine/game edits cannot invalidate
# them. They remain first-class tests and run explicitly through
# `./run_tests.sh --tool-tests` after the tool changes.
DETACHED_TOOL_MARKER = "detached_tool"
PYTEST_TIMING_ARGS = ["--durations=20", "--durations-min=0.05"]

# Composition crates listed here have no tests gated by their extra headless-safe features, so a
# separate feature variant adds build cost without coverage. If one gains a feature-gated test,
# remove it from this set in the same change.
SKIP_FEATURE_JOB = {
    "ambition_app",
    "ambition_menu",
}


def is_denied(feat: str) -> bool:
    return feat in DENY_EXACT or feat.startswith(DENY_PREFIX)


def workspace_members() -> list[Path]:
    text = (REPO / "Cargo.toml").read_text()
    body = re.search(r"members\s*=\s*\[(.*?)\]", text, re.S).group(1)
    out = []
    for line in body.splitlines():
        line = line.strip().strip(",").strip()
        if line and not line.startswith("#"):
            out.append(REPO / line.strip('"'))
    return out


def expand_default(features: dict[str, list[str]]) -> set[str]:
    """Feature names transitively pulled in by `default` (same-crate only)."""
    seen: set[str] = set()
    stack = list(features.get("default", []))
    while stack:
        f = stack.pop().split("/")[0]
        if f in features and f not in seen:
            seen.add(f)
            stack.extend(features[f])
    return seen


def crate_has_tests(crate: Path) -> bool:
    if (crate / "tests").is_dir():
        return True
    src = crate / "src"
    if not src.is_dir():
        return False
    for rs in src.rglob("*.rs"):
        t = rs.read_text(errors="replace")
        if "#[test]" in t or "#[cfg(test)]" in t:
            return True
    return False


@dataclass
class Job:
    name: str
    argv: list[str]
    # Working directory for the job; None = the repo root. Cargo discovers
    # `.cargo/config.toml` from the CWD upward (NOT from --manifest-path), so a
    # job that must honor an out-of-tree crate's own config — the external
    # consumer fixture's isolated target-dir — has to run from that directory.
    cwd: str | None = None


@dataclass
class JobResult:
    """One executed job and its wall-clock/test-execution timings.

    `executed_seconds` sums the runner's own reported execution time — libtest's
    per-binary `finished in`, or nextest's per-run `Summary`.
    """
    name: str
    argv: list[str]
    ok: bool
    seconds: float
    # ⛔⛔ `None` MEANS UNMEASURED AND `0.0` MEANS ZERO. A nextest job cannot
    # report its duration at all (see `timing_report`), and a zero there is a
    # MEASUREMENT — "this job spent no time running tests" — which hands its
    # whole wall clock to the derived build column. `wall_time_split` is the only
    # thing entitled to aggregate this; do not sum it with `or 0.0`.
    executed_seconds: float | None = None


def wall_time_split(results: list[JobResult]) -> dict:
    """Wall time divided into what was MEASURED and what could not be.

    ⛔⛔ THREE NUMBERS, BECAUSE TWO CANNOT SAY THIS. `seconds - executed` is only
    "the build graph" for a job whose runner REPORTED its execution time; for one
    that did not, the same subtraction hands its entire wall clock to the build
    column and states that as a measurement.

    ⇒ `unclassified_seconds` is the wall time of jobs that reported nothing, and
    `build_seconds` is derived ONLY from the jobs that did — so a reader gets the
    denominator it is entitled to (`executed + build`) and can see how much of
    the run that denominator does not cover.
    """
    measured = [r for r in results if r.executed_seconds is not None]
    unmeasured = [r for r in results if r.executed_seconds is None]
    executed = sum(r.executed_seconds or 0.0 for r in measured)
    measured_wall = sum(r.seconds for r in measured)
    return {
        "executed_seconds": round(executed, 1),
        # Never negative: a runner can report a duration slightly longer than
        # the wall clock this process measured around it.
        "build_seconds": round(max(0.0, measured_wall - executed), 1),
        "unclassified_seconds": round(sum(r.seconds for r in unmeasured), 1),
        "unclassified_jobs": len(unmeasured),
    }


def timing_report(results: list[JobResult]) -> str:
    """Per-job timings ranked slowest -> fastest.

    Success state rides along as a tag so a failed job's time is visible
    without being confused for a slow-but-green one; pass/fail accounting
    itself stays in the summary block, which lists failures separately.
    """
    total = sum(r.seconds for r in results) or 1.0
    split = wall_time_split(results)
    executed = split["executed_seconds"]
    lines = ["  job timings (slowest first, `run` = the runner's own execution time):"]
    for r in sorted(results, key=lambda r: -r.seconds):
        tag = "ok  " if r.ok else "FAIL"
        run = (
            f"{r.executed_seconds:.1f}s"
            if r.executed_seconds is not None
            else "not reported"
        )
        lines.append(f"    {r.seconds:8.1f}s  {tag}  {r.name}   (run {run})")
    # ⛔⛔ AN INSTRUMENT THAT CANNOT MEASURE SAYS SO. libtest prints
    # `finished in Xs` on STDOUT, which this runner pipes; nextest prints its
    # `Summary [ Xs ]` on STDERR, which it deliberately does NOT pipe so cargo's
    # progress bar keeps rendering. So under nextest the number is unavailable —
    # and printing "0s of 1734s executing tests (0%)" said, in the voice of a
    # measurement, that the suite spent no time running tests.
    #
    # ⛔ NOT FIXED BY MERGING STDERR INTO THE PIPE. Cargo's progress bar has no
    # newlines, so a line-buffered reader would stall on it — which is the whole
    # reason stderr stays attached.
    if executed:
        classified = executed + split["build_seconds"]
        lines.append(
            f"    ── {executed:.0f}s of {classified:.0f}s executing tests "
            f"({executed / max(classified, 1.0) * 100:.0f}%); the rest is the "
            f"build graph."
        )
        if split["unclassified_jobs"]:
            lines.append(
                f"    ── and {split['unclassified_seconds']:.0f}s across "
                f"{split['unclassified_jobs']} job(s) is NEITHER: their runner "
                "reported no execution time, so that wall clock is unsplit."
            )
    else:
        lines.append(
            f"    ── {total:.0f}s total. Test-execution time is NOT REPORTED: "
            "nextest prints it on stderr, which is left attached so cargo's "
            "progress renders. Read its own `Summary [ Xs ]` line above."
        )
    return "\n".join(lines)


def timings_payload(results: list[JobResult]) -> list[dict]:
    """Machine-readable timing rows (for --timings-json / RUN_TESTS_TIMINGS_JSON)."""
    return [
        {
            "job": r.name,
            "command": " ".join(r.argv),
            "ok": r.ok,
            "seconds": round(r.seconds, 3),
            # `null`, not `0`, for a job whose runner did not report — see
            # `JobResult.executed_seconds`.
            "executed_seconds": (
                round(r.executed_seconds, 3)
                if r.executed_seconds is not None
                else None
            ),
        }
        for r in results
    ]


def selected_members(only: list[str]) -> list[Path]:
    """Workspace members with a `Cargo.toml`, rejecting unknown requested packages."""
    members = [c for c in workspace_members() if (c / "Cargo.toml").exists()]
    if only:
        known = {c.name for c in members}
        unknown = [p for p in only if p not in known]
        if unknown:
            raise SystemExit(
                "run_tests: unknown package(s): " + ", ".join(sorted(unknown))
                + "\n  known packages: "
                + ", ".join(sorted(c.name for c in members)))
    return members


def wasm_target_installed() -> bool:
    """Whether `wasm32-unknown-unknown` is available to this toolchain."""
    rustup = os.path.expanduser("~/.cargo/bin/rustup")
    if not os.path.exists(rustup):
        rustup = "rustup"
    try:
        result = subprocess.run(
            [rustup, "target", "list", "--installed"],
            capture_output=True, text=True, check=False)
    except OSError:
        # Without rustup, this runner cannot prove the wasm target is installed.
        return False
    return result.returncode == 0 and "wasm32-unknown-unknown" in result.stdout


def build_jobs(only: list[str], heavy: bool, libtest_args: list[str],
               everything: bool = False,
               include_python_tooling: bool = True) -> list[Job]:
    """Plan the run; the default is the focused backbone.

    The exhaustive plan is opt-in. See `docs/recipes/cheapest-sufficient-check.md`.
    """
    jobs: list[Job] = []
    members = selected_members(only)
    # `--heavy` is the MORE-than-exhaustive pass; it cannot mean less.
    everything = everything or heavy

    # Run the cheap repo-coupled Python gate first, then start the Rust backbone before deferred
    # documentation/warning/compile-cost checks. Detached tools and maintenance audits are opt-in.
    post_rust_repo_jobs: list[Job] = []
    if not only and include_python_tooling:
        jobs.append(Job(
            "repo tooling (scripts/tests; repo-coupled)",
            [
                sys.executable, "-m", "pytest", "scripts/tests", "-q",
                "-m", f"not {DETACHED_TOOL_MARKER}", *PYTEST_TIMING_ARGS,
            ],
        ))
        post_rust_repo_jobs.extend([
            Job(
                "no warnings (cargo check --all-targets)",
                [sys.executable, "scripts/check_no_warnings.py"],
            ),
            Job(
                "doc links (active KB)",
                [sys.executable, "scripts/check_doc_links.py"],
            ),
            Job(
                # ⚠ NON-STRICT ON PURPOSE: it reports, it does not fail. This is
                # a linter for PROSE, so a false positive is a matter of a name
                # it cannot know about (a macro-declared const, an upstream type)
                # rather than a defect. Failing the lane on one would train
                # everybody to pass --no-verify. `--strict` exists for a
                # deliberate sweep.
                "planning citations (reports, does not gate)",
                [sys.executable, "scripts/check_planning_citations.py"],
            ),
            Job(
                "compile-cost ratchet (frozen weights, not a stopwatch)",
                [sys.executable, "scripts/compile_ratchet.py"],
            ),
        ])

    def libtest(extra: list[str] = ()) -> list[str]:
        tail = list(libtest_args) + list(extra)
        return (["--"] + tail) if tail else []

    # Package filters run that package directly; the unfiltered backbone uses one workspace graph.
    # ⛔ THE CLASS NEXTEST CANNOT SEE, and the suite could not either: it ran no
    # doctests until 2026-08-27, and one had been failing to compile for as long
    # as it had existed (`ambition_sim_harness`, `use crate::` in a block that
    # compiles as its own crate).
    #
    # ⛔⛔ IT BELONGS ON BOTH BRANCHES AND ONLY UNDER NEXTEST, which the first
    # version got wrong in both directions. `-p` had no doctest job at all, so
    # `./run_tests.sh -p ambition_sim_harness` covered LESS than the plain
    # `cargo test -p ambition_sim_harness` it replaced — on the very crate whose
    # broken doctest started this. And the workspace job ran unconditionally,
    # which under plain cargo is a second doctest pass, because `cargo test`
    # already runs them.
    if only:
        for crate in members:
            if crate.name in only:
                jobs.append(Job(f"{crate.name} (default features)",
                                cargo_test(["-p", crate.name], list(libtest_args))))
                if NEXTEST:
                    jobs.append(Job(f"{crate.name} doctests",
                                    [CARGO, "test", "-p", crate.name, "--doc"]))
    else:
        jobs.append(Job("workspace (default features)",
                        cargo_test(["--workspace"], list(libtest_args))))
        if NEXTEST:
            jobs.append(Job("workspace doctests",
                            [CARGO, "test", "--workspace", "--doc"]))

    # The unfiltered default plan also boots visible presentation through a small `capture_scene`;
    # package-filtered runs stay scoped to the requested crate.
    if not only:
        jobs.append(Job("acceptance: the render composition draws a frame",
                        [CARGO, "run", "-p", "ambition_app_tools",
                         "--bin", "capture_scene", "--",
                         "central_hub_complex", "player",
                         "target/composition_boot.png", "320x180",
                         "--warmup", "20"]))

    # Exhaustive mode proves each crate's headless-safe feature combination compiles, then runs all
    # gated tests together in one union graph.
    check_jobs: list[Job] = []
    union_features: list[str] = []
    if everything:
        for crate in members:
            name = crate.name
            if only and name not in only:
                continue
            if name in SKIP_FEATURE_JOB:
                continue
            data = tomllib.loads((crate / "Cargo.toml").read_text())
            features = data.get("features", {})
            default = expand_default(features)
            extra = sorted(f for f in features
                           if not is_denied(f) and f not in default)
            if not extra or not crate_has_tests(crate):
                continue
            # Prove this feature combination with `cargo check`; the union job below executes the
            # gated tests without linking a separate test graph per crate.
            check_jobs.append(Job(f"{name} [{','.join(extra)}] (compiles)",
                                  [CARGO, "check", "-p", name, "--all-targets",
                                   "--features", ",".join(extra)]))
            union_features.extend(f"{name}/{f}" for f in extra)

    # The LDtk dependency is optional only if the monolith type-checks with default features off;
    # test that boundary directly rather than scanning source text for one dependency name.
    if everything and (not only or "ambition_platformer2d_actor_monolith" in only):
        check_jobs.append(Job(
            "the monolith's LDtk feature is REALLY optional (--no-default-features)",
            [CARGO, "check", "-p", "ambition_platformer2d_actor_monolith",
             "--no-default-features"]))

    # Exhaustive mode also exercises the causal feature against the real app composition rather
    # than assuming that a leaf-crate feature compile proves the assembled consumer.
    if not only and everything:
        # Fail feature-combination compile proofs before the more expensive test graph.
        jobs.extend(check_jobs)

        # Run every feature-gated test in one graph. `--no-fail-fast` preserves failures from later
        # targets; the per-crate checks above separately prove each feature combination resolves.
        if union_features:
            jobs.append(Job(
                "workspace [every headless-safe feature] — one graph, every gated test",
                [CARGO, "test", "--workspace", "--no-fail-fast",
                 "--features", ",".join(sorted(set(union_features))), *libtest()],
            ))
        else:
            # The causal instrument against the real composition, kept for the
            # filtered case where no union was accumulated.
            jobs.append(Job(
                "ambition_app [causal] — the instrument against the real composition",
                [CARGO, "test", "-p", "ambition_app", "--features", "rl_sim causal",
                 "--test", "app_it", *libtest(["causal_explains_the_real_app"])],
            ))

    # The external-consumer fixture (Phase 6): its own [workspace], lockfile, and target dir,
    # driven through --manifest-path so its INDEPENDENT dependency resolution is exactly what a
    # third party gets from the `ambition_platformer2d` umbrella. TWO public-API regressions,
    # invisible.
    #
    # ⇒ `cargo check` is seconds and catches exactly that class, so it is
    # unconditional; the full suite stays whole-suite-only because its VALUE is
    # the behaviour, not the surface, and that is worth the minutes only once.
    jobs.append(Job("external consumer: outlander COMPILES against the umbrella",
                    [CARGO, "check", "--all-targets"],
                    cwd=str(REPO / "fixtures" / "external_consumer")))

    # These checks are repo-coupled and remain part of the default verdict, but
    # they do not gate STARTING the Rust tests. Running them after the workspace,
    # render-composition, and external-consumer Rust jobs minimizes time to the
    # feedback that dominates ordinary engine work. The no-warnings check also
    # benefits from Cargo artifacts the Rust lane has just materialized.
    jobs.extend(post_rust_repo_jobs)

    if not only and everything:
        jobs.append(Job("external consumer: outlander",
                        [CARGO, "test"],
                        cwd=str(REPO / "fixtures" / "external_consumer")))

        # `minimal_game` is its own workspace, so `cargo test --workspace` does
        # not execute it. Run it explicitly to cover the minimal consumer's boot
        # behavior in addition to its import ratchet.
        jobs.append(Job("external consumer: minimal game",
                        [CARGO, "test"],
                        cwd=str(REPO / "fixtures" / "minimal_game")))

        # Leaving the workspace drops a crate from `cargo test --workspace` silently, and its 19
        # tests are the only proof that a capability can contribute a schema, an action, rollback
        # state and causal facts without editing anything central.
        #
        # moving it also broke it, which is the argument for the job: an
        # outside workspace does not inherit the engine's `[patch.crates-io]`,
        # so the rollback backend compiled against the RELEASED `bevy_ggrs` and
        # failed on a missing `GgrsFrameTiming`. Invisible for as long as the
        # crate lived inside and inherited the patch for free.
        jobs.append(Job("external consumer: capability demo",
                        [CARGO, "test"],
                        cwd=str(REPO / "examples" / "capability_demo")))

    # The WEB build. The web target sat broken for at least four days (see
    # docs/archive/repair_wasm.md) because nothing in the suite compiled it —
    # every native job stayed green while `--features web` had four errors in it.
    #
    # **ONE PERSONA LINKS, AND THAT IS THE POINT.** This was two `cargo
    # check`s, which never invoke `rust-lld` — so a whole class of browser
    # failure was structurally invisible: a dependency whose wasm feature is
    # missing compiles perfectly and then dies at the link with `undefined
    # symbol`. `ggrs` reaches `instant`, whose `now()` on wasm32 has no body
    # unless `instant/wasm-bindgen` is on, and the `web_platform` feature chain
    # stopped one crate short of the rollback backend that owns `bevy_ggrs`.
    # A check cannot see that. A build can.
    #
    # One persona because a linked wasm artifact is expensive; the served persona differs in ASSET
    # TRANSPORT, not in its dependency graph, so it stays a check — if that ever stops being true,
    # this is the line to change.
    if not only and everything:
        if wasm_target_installed():
            jobs.append(Job(
                "web build LINK [web, release]",
                [CARGO, "build", "-p", "ambition_app", "--lib", "--release",
                 "--target", "wasm32-unknown-unknown",
                 "--no-default-features", "--features", "web"]))
            jobs.append(Job(
                "web build check [web_served_assets]",
                [CARGO, "check", "-p", "ambition_app", "--lib",
                 "--target", "wasm32-unknown-unknown",
                 "--no-default-features", "--features", "web_served_assets"]))
        else:
            print("run_tests: SKIPPING the web build check — the "
                  "wasm32-unknown-unknown target is not installed "
                  "(`rustup target add wasm32-unknown-unknown`). "
                  "The web build is UNCHECKED in this run.")

    # Compile/link checks do not prove the web persona can boot. Step the web
    # composition natively under `visible_web_base`; `web_served_assets` also
    # enables wasm-only platform entry points that do not belong in this run.
    if not only and everything:
        jobs.append(Job(
            "web persona BOOTS [visible_web_base, native]",
            [CARGO, "run", "-p", "ambition_app", "--no-default-features",
             "--features", "visible_web_base", "--example", "web_persona_boot"]))

    # Heavy pass: rerun including #[ignore]d tests, plus the shipping-entrypoint
    # acceptance cycles (full app boot). Whole-suite, non-fast only.
    if heavy and not only:
        jobs.append(Job("workspace (+ ignored)",
                        cargo_test(["--workspace"],
                                   list(libtest_args) + ["--include-ignored"])))
        jobs.append(Job("acceptance: headless cycle",
                        ["./run_game.sh", "--", "--headless-acceptance-cycle"]))
        jobs.append(Job("acceptance: headless 120 ticks",
                        ["./run_game.sh", "--", "--headless", "--headless-ticks", "120"]))

    if not jobs:
        raise SystemExit("run_tests: empty job plan (nothing to run)")
    return jobs


def build_detached_tool_jobs(pytest_filter: str | None = None) -> list[Job]:
    """Plan tests owned by self-contained developer tools, and nothing else.

    Detached does not mean ignored: these tests run explicitly when their tool
    changes. Keeping root-tool ownership as a marker expression makes it visible
    at each test module and lets a tool migrate out of this repository without
    leaving runner-specific filename exceptions behind.

    The LDtk authoring package is already physically self-contained and owns its
    own virtualenv, so its full pytest suite belongs here too. A focused `-k`
    selects only the root detached tests; that avoids making an unrelated LDtk
    suite fail with pytest's "no tests selected" exit code when the filter names
    (for example) a goal-guard test.
    """
    argv = [
        sys.executable, "-m", "pytest", "scripts/tests", "-q",
        "-m", DETACHED_TOOL_MARKER, *PYTEST_TIMING_ARGS,
    ]
    if pytest_filter:
        argv.extend(["-k", pytest_filter])
        return [Job("detached repo developer tools", argv)]

    ldtk = REPO / "tools" / "ambition_ldtk_tools"
    ldtk_python = ldtk / ".venv" / "bin" / "python"
    return [
        Job("detached repo developer tools", argv),
        Job(
            "ldtk authoring tool tests",
            [
                str(ldtk_python) if ldtk_python.exists() else sys.executable,
                "-m", "pytest", "tests", "-q", *PYTEST_TIMING_ARGS,
            ],
            cwd=str(ldtk),
        ),
    ]


def build_maintenance_jobs() -> list[Job]:
    """Plan periodic repository-hygiene audits, and nothing else.

    `check_agent_kb.py` describes itself as periodic maintainer hygiene rather
    than routine code validation. Keeping that ownership explicit prevents an
    ungenerated local `.agent/index` or planning-corpus housekeeping issue from
    delaying every Rust edit while retaining a one-command audit lane.
    """
    return [
        Job(
            "agent KB (periodic doc/index hygiene)",
            [sys.executable, "scripts/check_agent_kb.py"],
        )
    ]


# libtest ends every binary with a line naming how long IT ran, and that number
# is the only part of a job's wall clock that is not the build graph.
LIBTEST_DURATION = re.compile(r"finished in ([0-9]+\.[0-9]+)s")
# ⛔⛔ AND NEXTEST DOES NOT SAY THAT SENTENCE, NOR ON THIS STREAM. It ends a run
# with one `Summary [ 265.439s] …` line, and it writes it to STDERR — which this
# runner deliberately leaves attached so cargo's progress bar keeps rendering. So
# the pattern is here and matches nothing today; what stops the metric LYING is
# the "not reported" arm in `format_timings`, not this regex.
NEXTEST_DURATION = re.compile(r"Summary \[\s*([0-9]+\.[0-9]+)s\]")


def run_job_streaming(job: "Job", env: dict) -> tuple[int, float | None]:
    """Run one job, echoing its output live, and total libtest's own runtime.

    ⚠ **live output is not negotiable**, which is why this streams rather than
    capturing: somebody watching a suite needs to see the failure as it happens.
    stdout is piped only so the duration lines can be counted on the way past;
    stderr (where cargo writes progress) stays attached to the terminal.

    ⛔ BOTH RUNNERS ARE READ, and a job never mixes them: libtest prints one
    `finished in Xs` per binary and nextest prints one `Summary [ Xs ]` per run.
    Summing whichever appears keeps the number meaning "time spent running
    tests" under either.
    """
    executed: float | None = None
    proc = subprocess.Popen(
        job.argv,
        cwd=job.cwd or REPO,
        env=env,
        stdout=subprocess.PIPE,
        text=True,
        bufsize=1,
    )
    assert proc.stdout is not None
    for line in proc.stdout:
        sys.stdout.write(line)
        match = LIBTEST_DURATION.search(line) or NEXTEST_DURATION.search(line)
        if match:
            executed = (executed or 0.0) + float(match.group(1))
    sys.stdout.flush()
    return proc.wait(), executed


def completed_rows(results: list[JobResult]) -> list[dict]:
    """The finished jobs, for a reader of the status file."""
    return [
        {"job": r.name, "ok": r.ok, "seconds": round(r.seconds, 1),
         "executed_seconds": (
             round(r.executed_seconds, 1) if r.executed_seconds is not None else None
         )}
        for r in results
    ]


def telemetry_envelope() -> dict:
    """Return best-effort metadata shared by compile-telemetry rows.

    Missing metadata yields `None`; telemetry collection must not fail the suite.
    """
    def git(*args: str) -> str | None:
        try:
            out = subprocess.run(["git", *args], cwd=REPO, capture_output=True,
                                 text=True, check=False)
            return out.stdout.strip() or None
        except OSError:
            return None

    opt_level = None
    try:
        manifest = tomllib.loads((REPO / "Cargo.toml").read_text(encoding="utf-8"))
        opt_level = str(manifest["profile"]["dev"].get("opt-level", 0))
    except (OSError, KeyError, tomllib.TOMLDecodeError):
        pass

    return {
        "schema": 1,
        "kind": "job",
        "recorded_at": time.strftime("%Y-%m-%dT%H:%M:%S%z"),
        "commit": git("rev-parse", "--short=12", "HEAD") or "unknown",
        "dirty": bool(git("status", "--porcelain")),
        "run_id": os.urandom(6).hex(),
        "label": os.environ.get("RUN_TESTS_LABEL", ""),
        "profile": "test",
        "opt_level": opt_level,
        # `run()` defaults child Cargo jobs to `CARGO_INCREMENTAL=0` without
        # mutating the parent environment, so mirror that default here.
        # own default, and if that `setdefault` ever changes, change this with it.
        "incremental": os.environ.get("CARGO_INCREMENTAL", "0") not in ("0", ""),
    }


def append_cost_ledger(results: list[JobResult], exhaustive: bool,
                       filtered: bool, rust_only: bool = False,
                       tool_tests_only: bool = False,
                       maintenance_only: bool = False) -> Path | None:
    """Append this run's cost so test-iteration trends can be compared.

    The ledger is append-only; individual runs remain available for comparison.
    """
    ledger = Path(os.environ.get("RUN_TESTS_COST_LEDGER",
                                 measurement_paths.JOBS_LEDGER))

    # the ledger lives in a submodule, and on a clone without `--recursive`
    # its directory exists and is EMPTY — so `open("a")` would succeed, write a
    # real row into a file inside an uninitialised submodule mount, and lose it
    # to the next `git submodule update` without ever appearing in `git status`.
    # this is the one writer that must NOT hard-fail: a cost record is worth
    # having and never worth failing a suite over (same rule as the `OSError`
    # below). It refuses to write and says why; it does not raise.
    unavailable = measurement_paths.unavailable_reason(ledger)
    if unavailable:
        print(f"  (cost NOT recorded: {unavailable}"
              f" — fix with `{measurement_paths.INIT_COMMAND}`)")
        return None

    row = {
        # The shared compile-telemetry envelope — `dev/compile_telemetry_schema.md` §1.
        #
        # copied by hand rather than imported from `compile_ratchet.py`: this
        # is the suite's own entry point and must not gain an import of a module
        # that itself imports a 1,500-line checker.
        **telemetry_envelope(),
        "finished": time.time(),
        "jobs": len(results),
        "seconds": round(sum(r.seconds for r in results), 1),
        **wall_time_split(results),
        "passed": sum(1 for r in results if r.ok),
        "exhaustive": exhaustive,
        "filtered": filtered,
        "rust_only": rust_only,
        "tool_tests_only": tool_tests_only,
        "maintenance_only": maintenance_only,
        "per_job": timings_payload(results),
    }
    try:
        ledger.parent.mkdir(parents=True, exist_ok=True)
        with ledger.open("a") as handle:
            handle.write(json.dumps(row) + "\n")
    except OSError as exc:
        # A cost record is worth having and never worth failing a suite over.
        print(f"  (could not append the cost ledger: {exc})")
        return None
    return ledger


def write_status(path: Path, payload: dict) -> None:
    """Rewrite the run-state file atomically.

    Atomically because this file exists to be read by another process while
    this one is running: a reader that catches a half-written file gets a
    JSON error, and the natural way to "handle" that is to retry forever.
    """
    path.parent.mkdir(parents=True, exist_ok=True)
    tmp = path.with_suffix(path.suffix + ".tmp")
    tmp.write_text(json.dumps(payload, indent=2) + "\n")
    tmp.replace(path)


def coverage_notice(
    exhaustive: bool,
    filtered: bool,
    rust_only: bool = False,
    tool_tests_only: bool = False,
    maintenance_only: bool = False,
) -> str:
    """State the intentionally omitted validation lanes out loud."""
    if tool_tests_only:
        return (
            "\n  ⚠ --tool-tests ran detached developer-tool tests only. "
            "Repo-coupled validation, Rust/Cargo lanes, and periodic maintenance "
            "audits were NOT run."
        )
    if maintenance_only:
        return (
            "\n  ⚠ --maintenance ran periodic repository-hygiene audits only. "
            "Repo-coupled tests, Rust/Cargo lanes, and detached tool tests were NOT run."
        )

    notices: list[str] = [
        "\n  · detached developer-tool tests were omitted from repo-wide "
        "validation; run `./run_tests.sh --tool-tests` after editing those tools.",
        "\n  · periodic repository-hygiene audits were omitted; run "
        "`./run_tests.sh --maintenance` when auditing agent indexes, planning, "
        "or knowledge-base structure.",
    ]
    if rust_only:
        notices.append(
            "\n  ⚠ --rust ran the Rust/Cargo lane only. Python repo checkers and "
            "authoring-tool pytest suites were NOT run. Run without --rust "
            "for the full repo backbone."
        )
    if not exhaustive:
        scope = "this package filter" if filtered else "the default BACKBONE plan"
        notices.append(
            f"\n  ⚠ this was {scope}, which does NOT cover:\n"
            "      - tests behind #[cfg(feature = \"...\")] (only a per-crate\n"
            "        `cargo test -p <crate> --features ...` compiles them)\n"
            "      - the external-consumer fixtures (own workspace + lockfile, so\n"
            "        an umbrella API break stays invisible to a workspace build)\n"
            "      - the wasm/web build check\n"
            "    All three: --run-everything-you-probably-dont-need-this (~25 min).\n"
            "    That is the right trade for a dev cycle and the wrong one before a\n"
            "    release or after touching features, an SDK surface, or the web path."
        )
    return "\n".join(notices)


def run(jobs: list[Job], list_only: bool, timings_json: str | None = None,
        status_json: str | None = None, exhaustive: bool = False,
        filtered: bool = False, rust_only: bool = False,
        tool_tests_only: bool = False,
        maintenance_only: bool = False) -> int:
    if list_only:
        print(f"Planned {len(jobs)} job(s):\n")
        for j in jobs:
            print(f"  {j.name}")
            print(f"      {' '.join(j.argv)}")
        print(coverage_notice(
            exhaustive, filtered, rust_only, tool_tests_only, maintenance_only
        ))
        return 0

    env = dict(os.environ)
    env.setdefault("RUST_BACKTRACE", "1")
    env.setdefault("CARGO_TERM_COLOR", "always")
    # This runner is the main producer: every feature job is its own variant, each variant keeps its
    # own incremental tree, and a suite that runs a dozen times a day never reuses most of them.
    #
    # Incremental buys nothing here either. A job either recompiles from a
    # feature set nothing else shares (no cache to hit) or is already fresh (no
    # compile at all). It stays ON for a developer's own `cargo run`, which is
    # where the edit-rebuild loop actually lives — `setdefault`, so anyone who
    # wants it back exports `CARGO_INCREMENTAL=1`.
    env.setdefault("CARGO_INCREMENTAL", "0")

    # **REFUSE ON A FULL DISK RATHER THAN DYING HALFWAY THROUGH IT.**
    #
    # One refusal up front, naming the remedy, is worth more than any amount of diagnosing that.
    free_gb = free_gb_on_target()
    if not (tool_tests_only or maintenance_only) and free_gb < MIN_FREE_GB:
        print(
            f"REFUSING: {free_gb:.1f} GB free on {target_dir()}, and a full suite "
            f"needs about "
            f"{MIN_FREE_GB:.0f}.\n"
            f"  Every feature job builds its own variant of the graph and cargo "
            f"never prunes the last one.\n"
            f"  Free it:  cargo clean            (then expect one full rebuild)\n"
            f"  Or run a subset:  ./run_tests.sh -p <crate>",
            file=sys.stderr,
        )
        return 1

    status = Path(status_json) if status_json else REPO / "target" / STATUS_NAME
    results: list[JobResult] = []
    # `free_gb` travels in the status file so a long autonomous run can WATCH
    # the headroom fall instead of discovering it at zero.
    base = {"pid": os.getpid(), "started": time.time(), "jobs": len(jobs),
            "free_gb_at_start": round(free_gb, 1), "rust_only": rust_only,
            "tool_tests_only": tool_tests_only,
            "maintenance_only": maintenance_only}
    write_status(status, {**base, "state": "running", "finished_jobs": 0})
    # Not a happy-path write: a suite that dies mid-run (Ctrl-C, an unhandled
    # exception) must not leave `"running"` behind, or a future reader waits on
    # a run that no longer exists. SIGKILL still can, hence the `pid` above.
    try:
        for j in jobs:
            print(f"\n\033[1m==> {j.name}\033[0m")
            print("    " + " ".join(j.argv))
            start = time.monotonic()
            # **WHICH job, and since WHEN** — written BEFORE the job runs.
            write_status(status, {**base, "state": "running",
                                  "finished_jobs": len(results),
                                  "current_job": j.name,
                                  "current_started": time.time(),
                                  "completed": completed_rows(results)})
            rc, executed = run_job_streaming(j, env)
            results.append(
                JobResult(j.name, j.argv, rc == 0, time.monotonic() - start,
                          executed))
            if rc != 0:
                print(f"\033[31m    FAILED ({j.name})\033[0m")
            write_status(status, {**base, "state": "running",
                                  "finished_jobs": len(results),
                                  "current_job": None,
                                  "completed": completed_rows(results)})
    except BaseException:
        write_status(status, {**base, "state": "crashed",
                              "finished_jobs": len(results),
                              "completed": completed_rows(results),
                              "failed": [r.name for r in results if not r.ok]})
        raise

    passed = sum(1 for r in results if r.ok)
    failed = [r.name for r in results if not r.ok]
    total = sum(r.seconds for r in results)
    print("\n" + "=" * 60)
    print(f"  {passed}/{len(results)} jobs passed in {total:.0f}s")
    if failed:
        print("  FAILED jobs:")
        for n in failed:
            print(f"    - {n}")
    print(timing_report(results))
    notice = coverage_notice(
        exhaustive, filtered, rust_only, tool_tests_only, maintenance_only
    )
    if notice:
        print(notice)
    print("=" * 60)

    if timings_json:
        Path(timings_json).write_text(
            json.dumps(timings_payload(results), indent=2) + "\n")
        print(f"  timings written to {timings_json}")

    # Keep the persistent suite-cost ledger separate from the optional per-run timing export.
    ledger = append_cost_ledger(
        results, exhaustive, filtered, rust_only, tool_tests_only, maintenance_only
    )
    if ledger:
        print(f"  cost appended to {ledger.relative_to(REPO) if ledger.is_relative_to(REPO) else ledger}")

    # Report disk delta because the selected build graph can materially change target usage.
    free_after = free_gb_on_target()
    spent = free_gb - free_after
    print(f"  disk: {free_after:.0f} GB free "
          f"({spent:+.0f} GB this run, {free_gb:.0f} GB before)")
    if free_after < MIN_FREE_GB:
        print(f"  ⚠ below the {MIN_FREE_GB:.0f} GB floor — the NEXT suite run will "
              f"refuse. `cargo clean` frees it, at the cost of one full rebuild.")

    write_status(status, {**base, "state": "done", "finished_jobs": len(results),
                          "passed": passed, "failed": failed,
                          "seconds": round(total, 1),
                          **wall_time_split(results),
                          "completed": completed_rows(results),
                          "current_job": None,
                          "free_gb_at_end": round(free_after, 1),
                          "disk_gb_spent": round(spent, 1),
                          "exit_code": 1 if failed else 0})
    print(f"  status written to {status}")
    return 1 if failed else 0


def main() -> int:
    ap = argparse.ArgumentParser(
        description="Ambition full test suite runner (pytest-like).",
        formatter_class=argparse.RawDescriptionHelpFormatter, epilog=__doc__)
    ap.add_argument("--heavy", action="store_true",
                    help="also run #[ignore]d tests and app acceptance cycles "
                         "(implies the exhaustive plan)")
    # Keep the exhaustive flag deliberately explicit; it is not the edit-loop default.
    ap.add_argument("--run-everything-you-probably-dont-need-this",
                    dest="run_everything", action="store_true",
                    help="the exhaustive plan: a cargo test -p per crate with "
                         "its feature-gated tests, the external-consumer "
                         "fixtures, the wasm check. ~33 jobs, ~25 MINUTES, "
                         "~17%% of it actually executing tests. There is no "
                         "CI and Jon "
                         "sweeps this periodically himself, so in a dev cycle "
                         "the default plan or a focused test is what you want.")
    # Compatibility flag: the backbone is now the default, so `--fast` is a reported no-op.
    ap.add_argument("--fast", action="store_true",
                    help="DEPRECATED no-op: the backbone is the default now")
    ap.add_argument("--rust", action="store_true",
                    help="run the Rust/Cargo lane only; skip all Python checker "
                         "and authoring-tool pytest jobs")
    ap.add_argument("--tool-tests", action="store_true",
                    help="run detached developer-tool tests only; these are "
                         "excluded from repo-wide validation because ordinary "
                         "engine/game edits cannot invalidate them")
    ap.add_argument("--maintenance", action="store_true",
                    help="run periodic repository-hygiene audits only; these "
                         "maintainer checks do not block ordinary code validation")
    ap.add_argument("--list", action="store_true", help="print job plan, run nothing")
    ap.add_argument("-k", metavar="SUBSTR", default=None,
                    help="only tests whose name contains SUBSTR (libtest filter)")
    ap.add_argument("-p", "--package", action="append", default=[],
                    help="restrict to this crate's job (repeatable)")
    ap.add_argument("--timings-json", metavar="PATH",
                    default=os.environ.get("RUN_TESTS_TIMINGS_JSON"),
                    help="also write per-job timings as JSON to PATH "
                         "(or set RUN_TESTS_TIMINGS_JSON)")
    ap.add_argument("--status-json", metavar="PATH",
                    default=os.environ.get("RUN_TESTS_STATUS_JSON"),
                    help="run-state file to rewrite as the suite progresses "
                         f"(default target/{STATUS_NAME}); read this to wait "
                         "for a run instead of scanning for the process")
    ap.add_argument("cargo_extra", nargs="*",
                    help="args after `--` forwarded to libtest")
    args = ap.parse_args()

    scope_flags = [args.rust, args.tool_tests, args.maintenance]
    if sum(bool(flag) for flag in scope_flags) > 1:
        ap.error("--rust, --tool-tests, and --maintenance are mutually exclusive scopes")
    if (args.tool_tests or args.maintenance) and args.package:
        ap.error("--tool-tests/--maintenance do not accept -p/--package")
    if (args.tool_tests or args.maintenance) and (args.heavy or args.run_everything):
        ap.error("focused tool/maintenance lanes cannot be combined with exhaustive modes")
    if (args.tool_tests or args.maintenance) and args.cargo_extra:
        ap.error("arguments after -- are libtest arguments and do not apply to tool/maintenance lanes")
    if args.maintenance and args.k:
        ap.error("--maintenance does not accept -k; run the maintenance checker directly to focus it")

    if args.tool_tests:
        jobs = build_detached_tool_jobs(args.k)
        print("run_tests: DETACHED TOOL lane requested; repo-coupled Python and "
              "Rust/Cargo jobs are omitted.")
        return run(
            jobs, args.list, timings_json=args.timings_json,
            status_json=args.status_json, exhaustive=False, filtered=False,
            tool_tests_only=True,
        )

    if args.maintenance:
        jobs = build_maintenance_jobs()
        print("run_tests: MAINTENANCE lane requested; routine repo tests, detached "
              "tool suites, and Rust/Cargo jobs are omitted.")
        return run(
            jobs, args.list, timings_json=args.timings_json,
            status_json=args.status_json, exhaustive=False, filtered=False,
            maintenance_only=True,
        )

    libtest_args = list(args.cargo_extra)
    if args.k:
        libtest_args.insert(0, args.k)

    if args.fast:
        print("run_tests: --fast is a no-op now — the backbone IS the default. "
              "The exhaustive plan is "
              "--run-everything-you-probably-dont-need-this.")

    jobs = build_jobs(args.package, args.heavy, libtest_args,
                      everything=args.run_everything,
                      include_python_tooling=not args.rust)
    if args.rust:
        print("run_tests: RUST/CARGO lane requested; Python checker and "
              "authoring-tool jobs are omitted.")
    if args.run_everything or args.heavy:
        print("run_tests: EXHAUSTIVE plan requested. Measured 2026-08-03: "
              "~33 jobs, ~25 minutes, ~17% of it executing tests. If you are "
              "mid-edit, a focused test almost certainly answers your question "
              "faster and just as well.")
    return run(jobs, args.list, timings_json=args.timings_json,
               status_json=args.status_json,
               exhaustive=args.run_everything or args.heavy,
               filtered=bool(args.package), rust_only=args.rust)


if __name__ == "__main__":
    sys.exit(main())
