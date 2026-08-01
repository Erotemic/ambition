#!/usr/bin/env python3
"""Ambition test runner -- a pytest-like front door to the whole cargo suite.

`./run_tests.sh` (which execs this) runs *everything that can run headlessly*:
the default `cargo test --workspace`, PLUS one job per crate that has extra
feature-gated tests, with those features turned on. Heavy/diagnostic tests are
marked `#[ignore]` in Rust (the "skip marker") and are opt-in via `--heavy`.

Why per-crate feature jobs: cargo unifies features per build graph, and there is
no safe workspace-wide "--all-features" here (that would pull in android/web/wasm
targets). So to actually COMPILE AND RUN a crate's `#[cfg(feature = "...")]`
tests, we enable that crate's headless-safe features in its own `cargo test -p`
invocation. The safe set is computed from each Cargo.toml (own features minus a
platform/wasm/static-asset denylist), so it can't drift as features are added.

Usage:
  ./run_tests.sh                     # full headless suite (excludes #[ignore])
  ./run_tests.sh --heavy             # ALSO run #[ignore]d tests + app acceptance
  ./run_tests.sh --list              # print the job plan, run nothing
  ./run_tests.sh -k <substr>         # only tests whose name contains <substr>
  ./run_tests.sh -p <crate>          # only that crate's job (repeatable)
  ./run_tests.sh --fast              # backbone only (default features, no
                                     #   feature jobs); honors -p if given
An unknown -p package or an otherwise empty plan is a hard error.
  ./run_tests.sh -- --nocapture      # args after `--` go to libtest

Exit code is nonzero if any job fails. A pytest-style summary is printed last.

WAITING FOR A RUN (read this before you write a polling loop)
------------------------------------------------------------
Every run rewrites a small status file -- `target/run_tests_status.json` by
default, `--status-json PATH` to move it. It holds `{"state": "running" |
"done" | "crashed", ...}` plus the pass/fail tally once finished. Ask *that*
whether the suite is still going:

    python3 -c 'import json,sys; \
      print(json.load(open("target/run_tests_status.json"))["state"])'

Ctrl-C and any exception land on `"crashed"`. A SIGKILL (the OOM killer) runs
nothing, so it can strand a `"running"` -- that is what the recorded `pid` is
for: no such process means the file is stale, whatever it says.

Do NOT poll with `pgrep -f run_tests.py`. The polling shell's own command line
contains the string `run_tests.py`, so pgrep matches the waiter itself, the
condition is permanently true, and the loop never exits. On 2026-07-31 seven
such shells were found still sleeping hours after the suite they were waiting
on had finished -- each one kept the next one alive. The bug is silent: the
loop looks like it is waiting for work that is simply slow.
"""
from __future__ import annotations

import argparse
import json
import os
import shutil
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
# Measured 2026-07-31: 28 jobs consumed ~295G of `target/debug/deps`. This is a
# floor with a little room, not a precise budget -- the point is to refuse
# BEFORE a job dies of ENOSPC and reports it as a compile error.
MIN_FREE_GB = 40.0


def target_dir() -> Path:
    """Where cargo actually writes — which is NOT always under the repo.

    ⚠ **this function exists because the first version of the disk guard read
    the wrong filesystem.** It measured the REPO's volume, and this checkout has
    the repo on a 1.8 TB disk while `.cargo/config.toml` points `target-dir` at
    `/home/joncrall/ambition-target` on a 387 GB one. The guard would have
    reported 380 GB free while the volume that actually fills had two — a green
    instrument answering a question nobody asked.
    """
    if env := os.environ.get("CARGO_TARGET_DIR"):
        return Path(env)
    config = REPO / ".cargo" / "config.toml"
    if config.is_file():
        for line in config.read_text().splitlines():
            line = line.strip()
            if line.startswith("target-dir"):
                _, _, value = line.partition("=")
                value = value.strip().strip('"').strip("'")
                if value:
                    return Path(value)
    return REPO / "target"


def free_gb_on_target() -> float:
    """Free space on the volume cargo writes to. Falls back to the repo's own
    volume only when the target directory's parent does not exist yet."""
    path = target_dir()
    while not path.exists() and path != path.parent:
        path = path.parent
    return shutil.disk_usage(path).free / 1024**3
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
    # Added 2026-07-31 when `ambition_actors` left SKIP_FEATURE_JOB (it gained
    # `#[cfg(feature = "causal")]` tests). That set's own comment predicted this
    # exact requirement: "if you remove this skip, expect to deny `profile` in
    # the same commit."
    "profile",
}
DENY_PREFIX = ("android", "web", "visible_web", "static_")

# Big composition crates whose only non-default headless-safe features gate NO
# test code (verified: app's portal_ldtk/profile, actors' profile, menu,
# runtime). A feature job for them recompiles the entire Bevy/ambition graph in
# a fresh feature-variant -- tens of GB of target artifacts -- for zero added
# coverage (their real tests already run in the `--workspace` backbone). Skip
# them. Every other crate's feature job unlocks tests, so it stays.
#
# RULE: adding a `#[cfg(feature = ...)]` test to a skipped crate must remove
# the skip in the same commit -- a stale entry here silently un-runs tests.
# ambition_host left this set 2026-07-19: its portal_render seam tests are
# feature-gated, and the portal feature now forwards ambition_runtime/portal
# so the composition is complete.
#
# `ambition` JOINED it 2026-07-31 on the reasoning that its 17 extra features
# are all FORWARDERS and "the facade's own tests gate on no feature".
#
# ⛔ It LEFT again 2026-08-01, because that last clause stopped being true: the
# facade gained `causal` and a `#[cfg(feature = "causal")]` SDK test module, so
# the entry had become "silently un-runs 7 tests" — the exact failure this set's
# own rule names. The cost was MEASURED rather than assumed, because the comment
# above implied it would be ruinous: **6m59s and 2.1 GB** for the 17-feature
# variant. Real, and not the "tens of GB" that phrase suggests — that figure
# belonged to `ambition_actors`' `profile` feature, which is denied above.
#
# It appeared at all only because the facade gained its first `#[cfg(test)]`
# module that day — `crate_has_tests` is what admits a crate to this pass — and
# the job it created FAILED immediately, which is worth recording where the next
# person to remove this entry will read it:
#
#   Tracy Profiler initialization failure: CPU doesn't support invariant TSC.
#
# The `profile` feature forwards `bevy/trace_tracy`, whose static initializer
# ABORTS the test binary on a CPU without an invariant TSC — before libtest lists
# a single test, so `--list` and a filter matching nothing fail identically.
# Nothing in the repo gates a test on `profile`. If you remove this skip, expect
# to deny `profile` (or set `TRACY_NO_INVARIANT_CHECK=1`) in the same commit —
# and to pay a full-graph feature-variant rebuild, measured at 488s.
# `ambition_actors` LEFT this set 2026-07-31 for the same reason and by the same
# rule: it gained `#[cfg(feature = "causal")]` tests (`actors::causal`, the
# movement-intent publisher). Its `profile` feature is denied above rather than
# built, which is what the entry below always said would be needed.
#
# `ambition_runtime` LEFT this set 2026-07-31, under the rule stated above: it
# gained `#[cfg(feature = "causal")]` tests (`runtime::causal`, the ECS side of
# the causal recorder), so the entry stopped being "no added coverage" and
# became "silently un-runs three tests". Removing it is what the rule requires;
# the cost is a feature-variant build of the runtime graph, paid once per suite.
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
    """One executed job: what ran, whether it passed, and how long it took."""
    name: str
    argv: list[str]
    ok: bool
    seconds: float


def timing_report(results: list[JobResult]) -> str:
    """Per-job timings ranked slowest -> fastest.

    Success state rides along as a tag so a failed job's time is visible
    without being confused for a slow-but-green one; pass/fail accounting
    itself stays in the summary block, which lists failures separately.
    """
    lines = ["  job timings (slowest first):"]
    for r in sorted(results, key=lambda r: -r.seconds):
        tag = "ok  " if r.ok else "FAIL"
        lines.append(f"    {r.seconds:8.1f}s  {tag}  {r.name}")
    return "\n".join(lines)


def timings_payload(results: list[JobResult]) -> list[dict]:
    """Machine-readable timing rows (for --timings-json / RUN_TESTS_TIMINGS_JSON)."""
    return [
        {
            "job": r.name,
            "command": " ".join(r.argv),
            "ok": r.ok,
            "seconds": round(r.seconds, 3),
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
               fast: bool = False) -> list[Job]:
    jobs: list[Job] = []
    members = selected_members(only)

    # The repo's OWN tooling, which is Python and was therefore invisible to a
    # cargo-only runner. `scripts/tests/` guards the goal guard, the test runner,
    # the package-asset guard and the architectural absence contracts -- and on
    # 2026-07-28 one of them had been RED for a day because a deliberate
    # behaviour change (SessionStart must never run the checks) left a stale
    # assertion behind and nothing ran it. A guard nobody executes is not a
    # guard. Cheap (~3s) and dependency-free, so it runs in the backbone too,
    # and FIRST: if the thing that decides whether the suite is honest is
    # broken, that is the answer, not the 40 minutes of cargo behind it.
    if not only:
        jobs.append(Job("repo tooling (scripts/tests)",
                        [sys.executable, "-m", "pytest", "scripts/tests", "-q"]))
        # The LDtk AUTHORING toolchain, which is the path every room in the
        # game is built through and was the second Python suite nothing ran.
        # Found 2026-07-28 with 11 of 149 RED, all of them pointing at asset
        # paths that moved out of `crates/ambition_actors/assets` -- and the
        # tools themselves defaulted to the same dead path, so a bare
        # `ldtk level set-field` opened a file that had not existed for weeks.
        # The lesson is the same one the job above records: a suite nobody
        # executes stops being a suite and becomes a document about the past.
        # ⚠ Its OWN interpreter, not `sys.executable`. `run_developer_setup.sh`
        # installs each tool project into a venv beside it (`install_tool_project`
        # → `uv pip install -e .`), so the LDtk package and its dependencies —
        # `pyron` among them — live in `tools/ambition_ldtk_tools/.venv` and are
        # NOT importable from the repo-root `.venv` this runner happens to be
        # executing under. Running the suite with the root interpreter collapsed
        # all 149 tests into `ModuleNotFoundError: pyron` at collection time.
        #
        # That was invisible for as long as the root venv had no `pytest` either:
        # the job died one step EARLIER, for a different reason, and fixing the
        # first exposed the second (2026-07-30). Falls back to this interpreter
        # when the tool venv is absent, so a partially set-up clone reports the
        # real import error rather than a missing-file error about python.
        ldtk = REPO / "tools" / "ambition_ldtk_tools"
        ldtk_python = ldtk / ".venv" / "bin" / "python"
        jobs.append(Job("ldtk authoring tools (tools/ambition_ldtk_tools)",
                        [str(ldtk_python) if ldtk_python.exists() else sys.executable,
                         "-m", "pytest", "tests", "-q"],
                        cwd=str(ldtk)))

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

    # Per-crate feature jobs: enable each crate's headless-safe extra features so
    # its #[cfg(feature = "...")] tests actually compile and run. Skipped under
    # --fast (backbone only). Big composition crates whose extra features gate no
    # test code are always skipped -- their default-feature job already runs every
    # test, and a feature variant would recompile the whole graph for nothing.
    if not fast:
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
            jobs.append(Job(f"{name} [{','.join(extra)}]",
                            [CARGO, "test", "-p", name,
                             "--features", ",".join(extra), *libtest()]))

    # The external-consumer fixture (Phase 6): its own [workspace], lockfile,
    # and target dir, driven through --manifest-path so its INDEPENDENT
    # dependency resolution is exactly what a third party gets from the
    # `ambition` umbrella. Whole-suite, non-fast only — an umbrella API break
    # can land while every in-repo job stays green (workspace feature
    # unification hides it), and this job is the only gate that can see it.
    if not only and not fast:
        jobs.append(Job("external consumer: outlander",
                        [CARGO, "test"],
                        cwd=str(REPO / "fixtures" / "external_consumer")))

        # ⚠ ADDED 2026-07-30. `minimal_game` is the campaign's SECOND sentinel
        # consumer and it had never run here — for the same structural reason
        # Outlander needs its own job (own workspace, own lockfile, outside
        # `cargo test --workspace`), minus the job.
        #
        # Two consumer-matrix rows rest on its 16 tests:
        # `movement-only-minimal-game` and `noncombat-actor`. Both were recorded
        # as PROVEN, and the proofs were never executed by the suite — a row
        # naming a test nobody runs is the same defect as a row naming no test,
        # and the matrix cannot tell them apart.
        #
        # It was ratcheted the whole time (`minimal-game-names-only-the-public-sdk`,
        # baseline zero), which is what made the gap easy to miss: a green
        # contract about the consumer's IMPORTS says nothing about whether the
        # consumer still boots.
        jobs.append(Job("external consumer: minimal game",
                        [CARGO, "test"],
                        cwd=str(REPO / "fixtures" / "minimal_game")))

        # ⚠ ADDED 2026-08-01, in the SAME COMMIT that moved this crate out of
        # `crates/` to `examples/capability_demo`. Leaving the workspace drops a
        # crate from `cargo test --workspace` silently, and its 19 tests are the
        # only proof that a capability can contribute a schema, an action,
        # rollback state and causal facts without editing anything central.
        #
        # ⛔ moving it also broke it, which is the argument for the job: an
        # outside workspace does not inherit the engine's `[patch.crates-io]`,
        # so `ambition_runtime` compiled against the RELEASED `bevy_ggrs` and
        # failed on a missing `GgrsFrameTiming`. Invisible for as long as the
        # crate lived inside and inherited the patch for free.
        jobs.append(Job("external consumer: capability demo",
                        [CARGO, "test"],
                        cwd=str(REPO / "examples" / "capability_demo")))

    # The WEB build, as a compile CHECK rather than a test run: there is no wasm
    # runner here, and a check is what the failure mode needs anyway. The web
    # target sat broken for at least four days (see docs/planning/repair_wasm.md)
    # because nothing in the suite compiled it — every native job stayed green
    # while `--features web` had four errors in it.
    #
    # A cargo CHECK, so this costs a compile and not a link + run. Whole-suite
    # and non-fast, because it builds a second target's dependency graph.
    if not only and not fast:
        if wasm_target_installed():
            for persona in ("web", "web_served_assets"):
                jobs.append(Job(
                    f"web build check [{persona}]",
                    [CARGO, "check", "-p", "ambition_app", "--lib",
                     "--target", "wasm32-unknown-unknown",
                     "--no-default-features", "--features", persona]))
        else:
            # LOUD, not silent. A skipped coverage that says nothing reads
            # exactly like coverage that passed, which is the failure this job
            # exists to end.
            print("run_tests: SKIPPING the web build check — the "
                  "wasm32-unknown-unknown target is not installed "
                  "(`rustup target add wasm32-unknown-unknown`). "
                  "The web build is UNCHECKED in this run.")

    # Heavy pass: rerun including #[ignore]d tests, plus the shipping-entrypoint
    # acceptance cycles (full app boot). Whole-suite, non-fast only.
    if heavy and not only and not fast:
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


def run(jobs: list[Job], list_only: bool, timings_json: str | None = None,
        status_json: str | None = None) -> int:
    if list_only:
        print(f"Planned {len(jobs)} job(s):\n")
        for j in jobs:
            print(f"  {j.name}")
            print(f"      {' '.join(j.argv)}")
        return 0

    env = dict(os.environ)
    env.setdefault("RUST_BACKTRACE", "1")
    env.setdefault("CARGO_TERM_COLOR", "always")
    # ⚠ **No incremental cache for suite runs, and the number is why.** On
    # 2026-07-31 a long autonomous run filled the disk — 387G, 100%, builds
    # failing with ENOSPC mid-suite — and `target/debug/incremental` alone was
    # **110G**. This runner is the main producer: every feature job is its own
    # variant, each variant keeps its own incremental tree, and a suite that runs
    # a dozen times a day never reuses most of them.
    #
    # Incremental buys nothing here either. A job either recompiles from a
    # feature set nothing else shares (no cache to hit) or is already fresh (no
    # compile at all). It stays ON for a developer's own `cargo run`, which is
    # where the edit-rebuild loop actually lives — `setdefault`, so anyone who
    # wants it back exports `CARGO_INCREMENTAL=1`.
    env.setdefault("CARGO_INCREMENTAL", "0")

    # ⛔ **REFUSE ON A FULL DISK RATHER THAN DYING HALFWAY THROUGH IT.**
    #
    # `CARGO_INCREMENTAL=0` above fixed the 110G half of the 2026-07-31 disk
    # fill. It did NOT fix the other half: every feature job is a full variant of
    # the graph and cargo never garbage-collects the previous ones, so a suite
    # run costs ~10G/job in `target/debug/deps` and 28 jobs refilled a 387G
    # volume the same day the incremental fix landed.
    #
    # The failure mode is what makes this worth a check rather than a note: a
    # suite that runs out of space mid-way reports a wall of unrelated compile
    # errors from whichever job was unlucky, and the actual cause (ENOSPC) is
    # nowhere in the output. One refusal up front, naming the remedy, is worth
    # more than any amount of diagnosing that.
    free_gb = free_gb_on_target()
    if free_gb < MIN_FREE_GB:
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
            "free_gb_at_start": round(free_gb, 1)}
    write_status(status, {**base, "state": "running", "finished_jobs": 0})
    # Not a happy-path write: a suite that dies mid-run (Ctrl-C, an unhandled
    # exception) must not leave `"running"` behind, or a future reader waits on
    # a run that no longer exists. SIGKILL still can, hence the `pid` above.
    try:
        for j in jobs:
            print(f"\n\033[1m==> {j.name}\033[0m")
            print("    " + " ".join(j.argv))
            start = time.monotonic()
            rc = subprocess.run(j.argv, cwd=j.cwd or REPO, env=env).returncode
            results.append(
                JobResult(j.name, j.argv, rc == 0, time.monotonic() - start))
            if rc != 0:
                print(f"\033[31m    FAILED ({j.name})\033[0m")
            write_status(status, {**base, "state": "running",
                                  "finished_jobs": len(results)})
    except BaseException:
        write_status(status, {**base, "state": "crashed",
                              "finished_jobs": len(results),
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
    print("=" * 60)

    if timings_json:
        Path(timings_json).write_text(
            json.dumps(timings_payload(results), indent=2) + "\n")
        print(f"  timings written to {timings_json}")

    # **What this run COST in disk**, as a number rather than as a surprise.
    #
    # The suite is the repo's largest disk consumer and its cost is invisible
    # until it is fatal: every feature job builds its own variant of the graph,
    # cargo never prunes the previous one, and the failure mode is a mid-run
    # ENOSPC that surfaces as unrelated compile errors. A figure printed every
    # run is what lets somebody notice the trend BEFORE the volume is full — and
    # it is a measurement that moves when the job plan changes, which is the only
    # kind worth having.
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
                    help="also run #[ignore]d tests and app acceptance cycles")
    ap.add_argument("--fast", action="store_true",
                    help="backbone only: cargo test --workspace")
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

    libtest_args = list(args.cargo_extra)
    if args.k:
        libtest_args.insert(0, args.k)

    jobs = build_jobs(args.package, args.heavy, libtest_args, fast=args.fast)
    return run(jobs, args.list, timings_json=args.timings_json,
               status_json=args.status_json)


if __name__ == "__main__":
    sys.exit(main())
