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
# The run-state file another process should read to learn whether a suite is
# still going. See the module docstring: process-scanning for the answer is
# what hangs, because the scan matches the shell doing the scanning.
STATUS_NAME = "run_tests_status.json"
# the disk guard lives in `check_disk_headroom.py`, not here, and is IMPORTED rather than copied. A
# guard only one caller can run is a guard for one caller.
from check_disk_headroom import MIN_FREE_GB, free_gb_on_target, target_dir  # noqa: E402

# `scripts/lib/measurement_paths.py` imports nothing but `pathlib`, which is what makes it
# importable from the suite's own entry point — the telemetry ENVELOPE below still stays
# hand-copied, because that would mean importing a 1,500-line checker.
sys.path.insert(0, str(REPO / "scripts" / "lib"))
import measurement_paths  # noqa: E402

CARGO = os.path.expanduser("~/.cargo/bin/cargo")
if not os.path.exists(CARGO):
    CARGO = "cargo"

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
    # `profile` forwards `bevy/trace_tracy`, whose static initializer ABORTS the
    # test binary on a CPU without an invariant TSC -- before libtest lists a
    # single test, so `--list` and a filter matching nothing fail identically.
    # Nothing in the repo gates a test on it, so denying it loses no coverage.
    #
    # That set's own comment predicted this exact requirement: "if you remove this skip, expect to
    # deny `profile` in the same commit."
    "profile",
}
DENY_PREFIX = ("android", "web", "visible_web", "static_")

# Tests owned by self-contained maintainer tools do not belong to the
# repo-wide correctness gate: ordinary engine/game edits cannot invalidate
# them. They remain first-class tests and run explicitly through
# `./run_tests.sh --tool-tests` after the tool changes.
DETACHED_TOOL_MARKER = "detached_tool"
PYTEST_TIMING_ARGS = ["--durations=20", "--durations-min=0.05"]

# Big composition crates whose only non-default headless-safe features gate NO
# test code (verified: app's portal_ldtk/profile, actors' profile, menu,
# runtime). A feature job for them recompiles the entire Bevy/ambition graph in
# a fresh feature-variant -- tens of GB of target artifacts -- for zero added
# coverage (their real tests already run in the `--workspace` backbone). Skip
# them. Every other crate's feature job unlocks tests, so it stays.
#
# RULE: adding a `#[cfg(feature = ...)]` test to a skipped crate must remove
# the skip in the same commit -- a stale entry here silently un-runs tests.
# ambition_platformer2d_host left this set: its portal_render seam tests are
# feature-gated, and the portal feature now forwards ambition_platformer2d_runtime/portal
# so the composition is complete.
#
# `ambition_platformer2d` JOINED it on the reasoning that its 17 extra features
# are all FORWARDERS and "the facade's own tests gate on no feature".
#
# Real, and not the "tens of GB" that phrase suggests — that figure belonged to
# `ambition_platformer2d_actor_monolith`' `profile` feature, which is denied above.
#
# It appeared at all only because the facade gained its first `#[cfg(test)]`
# module that day — `crate_has_tests` is what admits a crate to this pass — and
# the job it created FAILED immediately, which is worth recording where the next
# person to remove this entry will read it:
#
#   Tracy Profiler initialization failure: CPU doesn't support invariant TSC.
#
# The `profile` feature forwards `bevy/trace_tracy`, whose static initializer ABORTS the test binary
# on a CPU without an invariant TSC — before libtest lists a single test, so `--list` and a filter
# matching nothing fail identically. Nothing in the repo gates a test on `profile`. Its `profile`
# feature is denied above rather than built, which is what the entry below always said would be
# needed.
#
# Removing it is what the rule requires; the cost is a feature-variant build of the runtime graph,
# paid once per suite.
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

    `executed_seconds` sums libtest's reported execution time. Cargo jobs with no
    test binaries and non-libtest jobs report 0.0, so the derived execution
    percentage applies to the Cargo/libtest portion of the suite.
    """
    name: str
    argv: list[str]
    ok: bool
    seconds: float
    executed_seconds: float = 0.0


def timing_report(results: list[JobResult]) -> str:
    """Per-job timings ranked slowest -> fastest.

    Success state rides along as a tag so a failed job's time is visible
    without being confused for a slow-but-green one; pass/fail accounting
    itself stays in the summary block, which lists failures separately.
    """
    total = sum(r.seconds for r in results) or 1.0
    executed = sum(r.executed_seconds for r in results)
    lines = ["  job timings (slowest first, `run` = libtest's own execution time):"]
    for r in sorted(results, key=lambda r: -r.seconds):
        tag = "ok  " if r.ok else "FAIL"
        lines.append(
            f"    {r.seconds:8.1f}s  {tag}  {r.name}"
            f"   (run {r.executed_seconds:.1f}s)"
        )
    lines.append(
        f"    ── {executed:.0f}s of {total:.0f}s executing tests "
        f"({executed / total * 100:.0f}%); the rest is the build graph."
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
            "executed_seconds": round(r.executed_seconds, 3),
        }
        for r in results
    ]


def selected_members(only: list[str]) -> list[Path]:
    """Workspace members with a `Cargo.toml`, validated against `only`.

    An unknown package name is a HARD error: silently planning zero jobs (the old
    behavior) makes a typo look like a green run.
    """
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
    """Whether `wasm32-unknown-unknown` is available to this toolchain.

    Asked rather than assumed: a machine without the target would otherwise turn
    the web check into a job that can never pass, and a check that cannot pass
    wedges whoever is waiting on a green suite.
    """
    rustup = os.path.expanduser("~/.cargo/bin/rustup")
    if not os.path.exists(rustup):
        rustup = "rustup"
    try:
        result = subprocess.run(
            [rustup, "target", "list", "--installed"],
            capture_output=True, text=True, check=False)
    except OSError:
        # No rustup at all (a distro toolchain, a container). Unknowable rather
        # than absent — say so by declining the job, which the caller reports.
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

    # Repo-coupled Python tests run first because they validate the cheap guards
    # that decide whether this checkout is internally coherent. Detached tool
    # tests live behind --tool-tests and periodic repository-hygiene audits live
    # behind --maintenance; neither ordinary engine/game edits nor a Rust test
    # failure need to wait for them.
    #
    # Everything after the pytest job is DEFERRED until the Rust backbone has
    # started. This is an ordering decision, not a coverage reduction: warning,
    # documentation, and compile-cost guards still contribute to the same final
    # default verdict, but `cargo test --workspace` begins as soon as the cheap
    # repo-coupled Python gate completes.
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
                "compile-cost ratchet (frozen weights, not a stopwatch)",
                [sys.executable, "scripts/compile_ratchet.py"],
            ),
        ])

    def libtest(extra: list[str] = ()) -> list[str]:
        tail = list(libtest_args) + list(extra)
        return (["--"] + tail) if tail else []

    # Default-feature jobs. Every SELECTED package gets its own `cargo test -p`,
    # so a package filter can NEVER plan zero jobs; with no filter the whole
    # workspace builds as one unified graph. `--fast` honors the same restriction
    # (it only drops the feature/heavy passes below).
    if only:
        for crate in members:
            if crate.name in only:
                jobs.append(Job(f"{crate.name} (default features)",
                                [CARGO, "test", "-p", crate.name, *libtest()]))
    else:
        jobs.append(Job("workspace (default features)",
                        [CARGO, "test", "--workspace", *libtest()]))

    # Exercise the visible render composition in the default plan. Headless
    # app-boot tests cover simulation only; `capture_scene` composes presentation
    # and exits non-zero on invalid parameters. Use a small frame and short
    # warmup because this checks composition, not visual fidelity.
    # binary's graph.
    # Whole-suite only: a `-p ambition_input` run has no business booting a
    # renderer, and the filter's contract is that it plans that package's tests.
    if not only:
        jobs.append(Job("acceptance: the render composition draws a frame",
                        [CARGO, "run", "-p", "ambition_app_tools",
                         "--bin", "capture_scene", "--",
                         "central_hub_complex", "player",
                         "target/composition_boot.png", "320x180",
                         "--warmup", "20"]))

    # Per-crate feature jobs: enable each crate's headless-safe extra features so
    # its #[cfg(feature = "...")] tests actually compile and run. Skipped under
    # --fast (backbone only). Big composition crates whose extra features gate no
    # test code are always skipped -- their default-feature job already runs every
    # test, and a feature variant would recompile the whole graph for nothing.
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
            # **FRONT 1: PROVE IT COMPILES HERE, RUN IT ONCE BELOW.**
            #
            # Only the first needs its own build graph — and a distinct feature set IS a distinct
            # build graph, which is why 23 of these cost 23 dependency builds at opt-level 3.
            #
            # `cargo check` keeps the compile guarantee without codegen or
            # linking; the union job below runs every gated test in ONE graph.
            check_jobs.append(Job(f"{name} [{','.join(extra)}] (compiles)",
                                  [CARGO, "check", "-p", name, "--all-targets",
                                   "--features", ",".join(extra)]))
            union_features.extend(f"{name}/{f}" for f in extra)

    # **AN OPTIONAL DEPENDENCY THAT CANNOT BE TURNED OFF IS A FICTION.**
    #
    # this guards the CONDITION, not a proxy for it: the boundary IS "the
    # crate builds without the feature", so the check is that build. A grep for
    # unconditional `bevy_ecs_ldtk` would pass the day someone reaches for a
    # different LDtk-shaped type through a re-export.
    #
    # `check`, not `build` — the claim is about the dependency graph resolving
    # and the code type-checking without LDtk, and neither needs codegen.
    if everything and (not only or "ambition_platformer2d_actor_monolith" in only):
        check_jobs.append(Job(
            "the monolith's LDtk feature is REALLY optional (--no-default-features)",
            [CARGO, "check", "-p", "ambition_platformer2d_actor_monolith",
             "--no-default-features"]))

    # **THE CAUSAL INSTRUMENT AGAINST THE REAL APP.** `ambition_app` is in
    # SKIP_FEATURE_JOB, and that set's own rule says adding a
    # `#[cfg(feature = ...)]` test to a skipped crate must remove the skip in the
    # same commit. This is the narrower form of that: one targeted job instead of
    # a full all-non-default-features variant of the biggest crate in the repo.
    #
    # A feature with no consumer is a feature that has quietly stopped working.
    #
    # this is a second feature variant of the app graph, so it is deliberately
    # non-fast. If it ever costs more than it catches, the honest alternative is
    # folding `causal` into `desktop_dev` — recording is policy-gated `Off` by
    # default, so the cost would be code size and a per-tick policy check, not
    # published facts.
    if not only and everything:
        # The compile proofs first: they are cheap, and a combination that does
        # not build should say so before an hour of test running.
        jobs.extend(check_jobs)

        # **AND THE ONE GRAPH THAT RUNS EVERY GATED TEST.** (Front 1)
        #
        # **the union SEES MORE than the per-crate jobs, which is the finding
        # that justifies it.** Compiling everything at once surfaced three
        # `causal` message channels that both rollback oracles had been green
        # over because the default job never compiled them.
        #
        # `--no-fail-fast` is load-bearing: cargo otherwise stops at the first
        # failing target, so one red crate hides every later one — measured, and
        # it is why this must not be a plain `cargo test`.
        #
        # feature unification means a crate here is compiled with features its
        # own job would not enable. That is exactly what the check lane above
        # exists to cover: each combination still gets its own resolution proof.
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
                        [CARGO, "test", "--workspace",
                         *libtest(["--include-ignored"])]))
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


def run_job_streaming(job: "Job", env: dict) -> tuple[int, float]:
    """Run one job, echoing its output live, and total libtest's own runtime.

    ⚠ **live output is not negotiable**, which is why this streams rather than
    capturing: somebody watching a suite needs to see the failure as it happens.
    stdout is piped only so the "finished in Xs" lines can be counted on the way
    past; stderr (where cargo writes progress) stays attached to the terminal.
    """
    executed = 0.0
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
        match = LIBTEST_DURATION.search(line)
        if match:
            executed += float(match.group(1))
    sys.stdout.flush()
    return proc.wait(), executed


def completed_rows(results: list[JobResult]) -> list[dict]:
    """The finished jobs, for a reader of the status file."""
    return [
        {"job": r.name, "ok": r.ok, "seconds": round(r.seconds, 1),
         "executed_seconds": round(r.executed_seconds, 1)}
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
        "executed_seconds": round(sum(r.executed_seconds for r in results), 1),
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

    # Append the persistent cost row; `--timings-json` is the per-run export.
    ledger = append_cost_ledger(
        results, exhaustive, filtered, rust_only, tool_tests_only, maintenance_only
    )
    if ledger:
        print(f"  cost appended to {ledger.relative_to(REPO) if ledger.is_relative_to(REPO) else ledger}")

    # **What this run COST in disk**, as a number rather than as a surprise.
    #
    # A figure printed every run is what lets somebody notice the trend BEFORE the volume is full —
    # and it is a measurement that moves when the job plan changes, which is the only kind worth
    # having.
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
                          "executed_seconds": round(
                              sum(r.executed_seconds for r in results), 1),
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
    # The NAME is the feature. An agent reading `--help` or a stale command
    # line should be talked out of it by the flag itself, because being the
    # default is exactly how the exhaustive plan became the reflex.
    ap.add_argument("--run-everything-you-probably-dont-need-this",
                    dest="run_everything", action="store_true",
                    help="the exhaustive plan: a cargo test -p per crate with "
                         "its feature-gated tests, the external-consumer "
                         "fixtures, the wasm check. ~33 jobs, ~25 MINUTES, "
                         "~17%% of it actually executing tests. There is no "
                         "CI and Jon "
                         "sweeps this periodically himself, so in a dev cycle "
                         "the default plan or a focused test is what you want.")
    # Kept so an existing command line or script does not break. `--fast` WAS
    # the backbone-only mode; the backbone is now the default, so it is a no-op
    # that says so rather than a silent one.
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
