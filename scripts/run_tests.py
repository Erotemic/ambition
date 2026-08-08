#!/usr/bin/env python3
"""Ambition test runner -- a pytest-like front door to the whole cargo suite.

`./run_tests.sh` (which execs this) runs the BACKBONE: the repo's own Python
suites, and `cargo test --workspace` — one cargo invocation, one build graph.
That is broad-good-enough coverage and it is what you want in a dev cycle.

⭐ **THE DEFAULT IS DELIBERATELY NOT EXHAUSTIVE, and that is Jon's call
(2026-08-02).** The exhaustive plan — a separate `cargo test -p` per crate with
its feature-gated tests enabled, the external-consumer fixtures, the wasm check —
lives behind `--run-everything-you-probably-dont-need-this`, and the name is the
warning. Measured on 2026-08-02: the exhaustive plan is 33 jobs and **63 minutes**
of which **~7% executed tests**; the backbone's cargo job is 607s of that. The
rest is cargo re-resolving features per invocation and rebuilding the same
dependencies — the actor monolith was compiled SIXTEEN times in one run.

⛔ **If you are an agent, you almost certainly want the default or narrower.**
There is no CI. Jon runs the exhaustive plan periodically himself and accepts a
day of drift; you spending an hour on it does not add safety, it duplicates a
sweep that is already scheduled. Run the focused test that answers your actual
question. The full plan's whole value is finding what the backbone cannot see,
and it is worth an hour roughly never in the middle of an edit.

What the exhaustive plan buys, so the trade is legible rather than folkloric:
cargo unifies features per build graph, and there is no safe workspace-wide
"--all-features" here (that would pull in android/web/wasm targets). So a crate's
`#[cfg(feature = "...")]` tests are compiled ONLY by a `cargo test -p` that
enables them. The safe set is computed from each Cargo.toml (own features minus a
platform/wasm/static-asset denylist), so it can't drift as features are added.
The backbone therefore does not run feature-gated tests at all, and says so out
loud at the end of every run rather than letting a partial pass read as a full one.

Usage:
  ./run_tests.sh                     # BACKBONE: python suites + cargo test --workspace
  ./run_tests.sh -p <crate>          # only that crate's job (repeatable)
  ./run_tests.sh -k <substr>         # only tests whose name contains <substr>
  ./run_tests.sh --list              # print the job plan, run nothing
  ./run_tests.sh -- --nocapture      # args after `--` go to libtest
  ./run_tests.sh --run-everything-you-probably-dont-need-this
                                     # the 33-job exhaustive plan (~25 min)
  ./run_tests.sh --heavy             # ALSO #[ignore]d tests + app acceptance;
                                     #   implies the exhaustive plan
An unknown -p package or an otherwise empty plan is a hard error.

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
# ⭐ the disk guard lives in `check_disk_headroom.py`, not here, and is IMPORTED
# rather than copied. It was local to this file until 2026-08-02, which is
# exactly how the disk filled a third time: the refusal below works, but a bare
# `cargo test --workspace` typed directly never reaches it. A guard only one
# caller can run is a guard for one caller.
from check_disk_headroom import MIN_FREE_GB, free_gb_on_target, target_dir  # noqa: E402

# ⭐ the cost ledger's PATH is imported for the same reason the disk guard is:
# it was inlined here, declared again in three other scripts, and the ledgers
# have since moved into the `dev/ambition_dev_measurements` submodule.
# ⚠ `scripts/lib/measurement_paths.py` imports nothing but `pathlib`, which is
# what makes it importable from the suite's own entry point — the telemetry
# ENVELOPE below still stays hand-copied, because that would mean importing a
# 1,500-line checker.
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
    # Added 2026-07-31 when `ambition_platformer2d_actor_monolith` left SKIP_FEATURE_JOB (it gained
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
# ambition_platformer2d_host left this set 2026-07-19: its portal_render seam tests are
# feature-gated, and the portal feature now forwards ambition_platformer2d_runtime/portal
# so the composition is complete.
#
# `ambition_platformer2d` JOINED it 2026-07-31 on the reasoning that its 17 extra features
# are all FORWARDERS and "the facade's own tests gate on no feature".
#
# ⛔ It LEFT again 2026-08-01, because that last clause stopped being true: the
# facade gained `causal` and a `#[cfg(feature = "causal")]` SDK test module, so
# the entry had become "silently un-runs 7 tests" — the exact failure this set's
# own rule names. The cost was MEASURED rather than assumed, because the comment
# above implied it would be ruinous: **6m59s and 2.1 GB** for the 17-feature
# variant. Real, and not the "tens of GB" that phrase suggests — that figure
# belonged to `ambition_platformer2d_actor_monolith`' `profile` feature, which is denied above.
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
# `ambition_platformer2d_actor_monolith` LEFT this set 2026-07-31 for the same reason and by the same
# rule: it gained `#[cfg(feature = "causal")]` tests (`actors::causal`, the
# movement-intent publisher). Its `profile` feature is denied above rather than
# built, which is what the entry below always said would be needed.
#
# `ambition_platformer2d_runtime` LEFT this set 2026-07-31, under the rule stated above: it
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
    """One executed job: what ran, whether it passed, and how long it took.

    `executed_seconds` is the part of `seconds` that was RUNNING TESTS rather
    than compiling — summed from libtest's own "finished in Xs" lines, which
    every test binary already prints. The rest is the build graph, and the whole
    campaign this measurement serves is about paying for that graph fewer times.
    A job with no test binaries (a bare `cargo check`) reports 0.0 and means it.

    ⚠ **it counts LIBTEST only, and the pytest jobs therefore read 0.0.** The
    repo-tooling and ldtk-tools jobs really do spend their whole wall clock
    executing, so the summary percentage is a statement about the CARGO jobs —
    which is where the campaign's cost is — and understates the whole-suite
    figure. Said here because a percentage nobody can source is how the last
    measurement went wrong.
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
               everything: bool = False) -> list[Job]:
    """Plan the run. `everything=False` (the DEFAULT) is the backbone.

    ⚠ The gate below reads `if everything:` and it used to read `if not fast:`.
    The inversion is the point, not a refactor: the exhaustive plan is opt-in as
    of 2026-08-02 because being the default made it the thing an agent reached
    for instead of the focused test that would have answered the question. See
    the module docstring and `docs/planning/test-iteration-cost-2026-08-02.md`.
    """
    jobs: list[Job] = []
    members = selected_members(only)
    # `--heavy` is the MORE-than-exhaustive pass; it cannot mean less.
    everything = everything or heavy

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
        # ⭐ the WARNING gate, which CI has had all along and no local command
        # did. `.github/workflows/test.yml` sets `RUSTFLAGS: -D warnings`, so a
        # warning is a red build there; locally `cargo check` says nothing and
        # five had accumulated in the tree by 2026-08-02.
        # ⚠ it does NOT set RUSTFLAGS itself -- that is part of cargo's
        # fingerprint and would rebuild the whole workspace, which is how this
        # target directory filled the volume three times. It reads the SAME
        # build's diagnostics instead. See the script's docstring.
        jobs.append(Job("no warnings (cargo check --all-targets)",
                        [sys.executable, "scripts/check_no_warnings.py"]))
        # ⭐ **THE THIRD CHECKER NOTHING RAN**, and the pattern is now a habit
        # worth naming: the two jobs above were both added for exactly this
        # reason. `check_agent_kb.py` owns the doc-navigation contracts -- dead
        # links between agent docs, stale planning evidence, ADR implications,
        # and the inline-test review markers that keep a 200+ line test module
        # from escaping review. Nothing invoked it, so all of that was advisory
        # in the sense of "true only if somebody remembered".
        # ⚠ it caught a live regression the day it was wired in: two inline test
        # modules had crossed the 200-line proxy that morning with no marker.
        # ⚠ its AGENTS.md SIZE finding is a warning rather than an error, because
        # that overage is queue F6's open maintainer decision and a suite
        # permanently red on a decision nobody has made teaches people to ignore
        # the suite. Every other check in that file is fatal.
        jobs.append(Job("agent KB (doc contracts + inline-test review)",
                        [sys.executable, "scripts/check_agent_kb.py"]))
        # ⭐ **AND THE FOURTH AND FIFTH, found by asking the question the third
        # one raised instead of waiting to trip over them.** Cross-referencing
        # `ls scripts/check_*.py` against this plan AND against `scripts/tests/`
        # (several checkers run through a pytest wrapper rather than directly, so
        # absence from this list is not proof) left exactly two orphans:
        #   * `check_doc_links.py` — 0.5s, dead links in the ACTIVE knowledge base
        #     (archives keep stale paths on purpose and are excluded);
        #   * `check_roadmap_evidence.py` — 10s, re-derives each roadmap
        #     `**Status …**` claim from SOURCE. Its own docstring records three
        #     stale claims found that way on 2026-07-27; prose does not rot loudly.
        # Both were already green, so wiring them in adds 10.7s and no red — the
        # value is the regression they now catch rather than anything they say today.
        jobs.append(Job("doc links (active KB)",
                        [sys.executable, "scripts/check_doc_links.py"]))
        # ⛔ `--check` IS LOAD-BEARING AND WAS MISSING (fixed 2026-08-07).
        # ⚠ and unlike `check_absence_contracts.py` — whose own `--check` is also
        # optional and is FINE, because `scripts/tests/test_absence_contracts.py`
        # asserts every contract against the live tree — this script has NO pytest
        # gate. Checked: 26 files in `scripts/tests/` and none of them is about
        # roadmap evidence. So this invocation was its only enforcement path, and
        # the flag was the whole of it. The
        # script prints its findings either way and returns
        # `1 if (problems and args.check) else 0`, so without the flag this job
        # could not go red — it ran for the whole period the comment above says
        # its value is "the regression they now catch", catching nothing. Proven
        # rather than reasoned: a fixture citing a deleted type exits 0 without
        # the flag and 1 with it. ⚠ this is the repo's recorded failure shape —
        # an optional `--check` that turns a guard green-by-construction — inside
        # the guard whose own docstring is about claims that do not rot loudly.
        jobs.append(Job("roadmap claims match source",
                        [sys.executable, "scripts/check_roadmap_evidence.py",
                         "--check"]))
        # ⭐ **the compile-cost ratchet** (Jon, 2026-08-08: *"I want to quantify
        # those compile wins as we do those. And to guard against compile time
        # regressions."*). Guards the DETERMINISTIC cause — blast radius of an
        # edit in measured SECONDS, largest recompilation unit, and the serial
        # chain length that parallelism cannot compress — against a frozen
        # baseline in `dev/`.
        # ⛔ **still not a wall-clock threshold**, which is what the renamed job
        # is saying. The seconds are per-crate WEIGHTS read from the committed
        # `dev/ambition_dev_measurements/compile_units.jsonl` and frozen into the baseline; nothing is
        # timed while the gate runs, so it cannot fail randomly on a busy
        # machine. A stale weight is a known number in a reviewable file, which
        # is the trade this instrument was built to make.
        # ⚠ **it costs ~1.6s and runs NO build.** `cargo metadata --offline` and
        # `cargo tree --offline` resolve manifests; neither compiles anything, so
        # this is safe beside any other cargo work and cannot be the job that
        # fills the disk.
        # ⚠ and it needs no `--check`: a violation exits 1 by default, because
        # an optional enforcement flag is how `check_roadmap_evidence.py` above
        # spent its whole life green.
        jobs.append(Job("compile-cost ratchet (frozen weights, not a stopwatch)",
                        [sys.executable, "scripts/compile_ratchet.py"]))
        # The LDtk AUTHORING toolchain, which is the path every room in the
        # game is built through and was the second Python suite nothing ran.
        # Found 2026-07-28 with 11 of 149 RED, all of them pointing at asset
        # paths that moved out of `crates/ambition_platformer2d_actor_monolith/assets` -- and the
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

    # ⭐ **the RENDER composition, which nothing else boots.** Every app-boot
    # job in this plan is `--headless`, and so is every app-boot test in the
    # workspace: the suite proves the SIMULATION composes and has never once
    # proved the presentation does.
    #
    # That hole is the exact shape of the defects. On 2026-08-04 four
    # param-validation panics sat in a row in `Update` systems of the render
    # composition — a `Res<ResolvedVisualQuality>` with no owner installed, a
    # `Res<ActiveShellSequence>` with no shell, a `MessageWriter` for an
    # unregistered message — and the whole suite was green through all of them,
    # because the shipped app installs everything and every acceptance cycle runs
    # headless. They were found by hand, by someone who happened to want a
    # picture (queue D16).
    #
    # `capture_scene` already builds that composition and already exits non-zero
    # when a param fails to validate, so this needs no new test infrastructure —
    # only for something to RUN it. It asks whether the app COMPOSES, not what it
    # looks like, hence 320x180 and 20 warmup ticks.
    #
    # ⚠ **in the DEFAULT plan, not behind `--heavy`.** Opt-in coverage is what
    # let `modules_md.py` sit with a check mode nothing ever called, and this
    # repo's conclusion from that was "a guard nobody executes is not a guard."
    # The class bit during ordinary work, so it runs during ordinary work; the
    # cost is RUNTIME only, because `--workspace` above already builds this
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
            # ⭐ **FRONT 1: PROVE IT COMPILES HERE, RUN IT ONCE BELOW.**
            #
            # This used to be `cargo test -p <crate> --features <extra>`, and
            # that job did two different things at once: proved the feature
            # combination COMPILES, and RAN the tests those features gate. Only
            # the first needs its own build graph — and a distinct feature set IS
            # a distinct build graph, which is why 23 of these cost 23 dependency
            # builds at opt-level 3 (measured 2026-08-02: 1858 compile events,
            # 400 of 454 crates built more than once, ~7% of the hour executing
            # tests).
            #
            # `cargo check` keeps the compile guarantee without codegen or
            # linking; the union job below runs every gated test in ONE graph.
            check_jobs.append(Job(f"{name} [{','.join(extra)}] (compiles)",
                                  [CARGO, "check", "-p", name, "--all-targets",
                                   "--features", ",".join(extra)]))
            union_features.extend(f"{name}/{f}" for f in extra)

    # ⛔ **THE CAUSAL INSTRUMENT AGAINST THE REAL APP.** `ambition_app` is in
    # SKIP_FEATURE_JOB, and that set's own rule says adding a
    # `#[cfg(feature = ...)]` test to a skipped crate must remove the skip in the
    # same commit. This is the narrower form of that: one targeted job instead of
    # a full all-non-default-features variant of the biggest crate in the repo.
    #
    # It exists because `causal` was enabled NOWHERE (measured 2026-08-01: false
    # for every crate in the default workspace resolve, and `ambition_app` had no
    # such feature at all), so every domain recorder was compiled out of every
    # build the app produces while the substrate's own per-crate tests stayed
    # green. A feature with no consumer is a feature that has quietly stopped
    # working.
    #
    # ⚠ this is a second feature variant of the app graph, so it is deliberately
    # non-fast. If it ever costs more than it catches, the honest alternative is
    # folding `causal` into `desktop_dev` — recording is policy-gated `Off` by
    # default, so the cost would be code size and a per-tick policy check, not
    # published facts.
    if not only and everything:
        # The compile proofs first: they are cheap, and a combination that does
        # not build should say so before an hour of test running.
        jobs.extend(check_jobs)

        # ⭐ **AND THE ONE GRAPH THAT RUNS EVERY GATED TEST.** (Front 1)
        #
        # The union of every feature job's extras, package-qualified. Measured
        # 2026-08-03 rather than assumed: `cargo check --workspace` with all 55
        # entries resolves, and the test run takes 5m54s of wall clock for the
        # whole workspace — against 23 separate graphs, each rebuilding shared
        # dependencies at opt-level 3.
        #
        # ⚠ **the union SEES MORE than the per-crate jobs, which is the finding
        # that justifies it.** Compiling everything at once surfaced three
        # `causal` message channels that both rollback oracles had been green
        # over because the default job never compiled them.
        #
        # ⚠ `--no-fail-fast` is load-bearing: cargo otherwise stops at the first
        # failing target, so one red crate hides every later one — measured, and
        # it is why this must not be a plain `cargo test`.
        #
        # ⚠ feature unification means a crate here is compiled with features its
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

    # The external-consumer fixture (Phase 6): its own [workspace], lockfile,
    # and target dir, driven through --manifest-path so its INDEPENDENT
    # dependency resolution is exactly what a third party gets from the
    # `ambition_platformer2d` umbrella. Whole-suite, non-fast only — an umbrella API break
    # can land while every in-repo job stays green (workspace feature
    # unification hides it), and this job is the only gate that can see it.
    if not only and everything:
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
        # so `ambition_platformer2d_runtime` compiled against the RELEASED `bevy_ggrs` and
        # failed on a missing `GgrsFrameTiming`. Invisible for as long as the
        # crate lived inside and inherited the patch for free.
        jobs.append(Job("external consumer: capability demo",
                        [CARGO, "test"],
                        cwd=str(REPO / "examples" / "capability_demo")))

    # The WEB build, as a compile CHECK rather than a test run: there is no wasm
    # runner here, and a check is what the failure mode needs anyway. The web
    # target sat broken for at least four days (see docs/archive/repair_wasm.md)
    # because nothing in the suite compiled it — every native job stayed green
    # while `--features web` had four errors in it.
    #
    # A cargo CHECK, so this costs a compile and not a link + run. Whole-suite
    # and non-fast, because it builds a second target's dependency graph.
    if not only and everything:
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
    """The columns every compile-telemetry row carries, whatever its grain.

    ⚠ **best-effort, like the ledger it feeds.** Nothing here is allowed to fail
    a suite: an unreadable manifest or a missing git binary yields `None` for
    that column rather than an exception. A missing value is a gap in a
    statistic; a raised exception is a red run for a bookkeeping reason.
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
        # ⚠ the suite forces this OFF for itself while the dev loop runs with it
        # ON (`.cargo/config.toml` vs `run()`'s env), and until now no row said
        # which of the two it was. That is the single largest confounder in this
        # ledger's own numbers.
        #
        # ⛔ **the default is "0", not the config file, and the first draft got
        # this backwards.** `run()` builds `env = dict(os.environ)` — a COPY —
        # and then `setdefault`s `CARGO_INCREMENTAL=0` on it, so the PARENT
        # process this function runs in has the variable UNSET on every ordinary
        # suite run. Reading it with no default reported `incremental: true` for
        # exactly the runs that are incremental-off: a column wrong precisely
        # when it matters, which is worse than no column. Mirror the runner's
        # own default, and if that `setdefault` ever changes, change this with it.
        "incremental": os.environ.get("CARGO_INCREMENTAL", "0") not in ("0", ""),
    }


def append_cost_ledger(results: list[JobResult], exhaustive: bool,
                       filtered: bool) -> Path | None:
    """Record what this run COST, on every run, so two runs can be compared.

    ⭐ **Front 0 of the test-iteration campaign** (Jon, 2026-08-02: *"testing
    iteration is wasting too much agent time, it is unacceptable"*). Every claim
    the campaign makes — that a change removed a compile, that parallelism helped
    — is judged against this file, and the campaign must not quote another
    hand-read log: its founding numbers were read off ONE run by hand, on a
    machine that had a second agent building in the same target directory.

    Appended, never rewritten: the value is the TREND, and one line per run is
    small enough to keep forever.
    """
    ledger = Path(os.environ.get("RUN_TESTS_COST_LEDGER",
                                 measurement_paths.JOBS_LEDGER))

    # ⛔ the ledger lives in a submodule, and on a clone without `--recursive`
    # its directory exists and is EMPTY — so `open("a")` would succeed, write a
    # real row into a file inside an uninitialised submodule mount, and lose it
    # to the next `git submodule update` without ever appearing in `git status`.
    # ⚠ this is the one writer that must NOT hard-fail: a cost record is worth
    # having and never worth failing a suite over (same rule as the `OSError`
    # below). It refuses to write and says why; it does not raise.
    unavailable = measurement_paths.unavailable_reason(ledger)
    if unavailable:
        print(f"  (cost NOT recorded: {unavailable}"
              f" — fix with `{measurement_paths.INIT_COMMAND}`)")
        return None

    row = {
        # The shared compile-telemetry envelope — `dev/compile_telemetry_schema.md`
        # §1. Added 2026-08-08 because the first 75 rows carry no commit, no
        # profile and no opt-level, so a year of them cannot answer "did that
        # change help" or "was this the incremental config". ⚠ these are the
        # columns that CANNOT be back-filled; `finished` stays for the rows that
        # already have it.
        #
        # ⛔ copied by hand rather than imported from `compile_ratchet.py`: this
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


def coverage_notice(exhaustive: bool, filtered: bool) -> str:
    """What this plan did NOT cover, said out loud.

    A green backbone must not be readable as a green everything. This is a
    printed line, not a guard: it states the gap and names the flag that closes
    it, and then gets out of the way.

    ⚠ It is deliberately NOT an argument that the gap must be closed. Jon,
    2026-08-02, on the wasm target having sat broken for four days: *"we let it
    sit for 4 days because we didn't care about it for 4 days."* Not caring was
    the correct call. The notice exists so the choice stays visible and
    deliberate, not so someone feels obliged to spend an hour closing it.
    """
    if exhaustive:
        return ""
    scope = "this package filter" if filtered else "the default BACKBONE plan"
    return (
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


def run(jobs: list[Job], list_only: bool, timings_json: str | None = None,
        status_json: str | None = None, exhaustive: bool = False,
        filtered: bool = False) -> int:
    if list_only:
        print(f"Planned {len(jobs)} job(s):\n")
        for j in jobs:
            print(f"  {j.name}")
            print(f"      {' '.join(j.argv)}")
        print(coverage_notice(exhaustive, filtered))
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
            # ⭐ **WHICH job, and since WHEN** — written BEFORE the job runs.
            #
            # The status file used to carry a count and nothing else, so an agent
            # waiting on a suite could see "7 of 33 done" and had no way to tell a
            # slow job from a wedged one. The two facts that answer it are the
            # name of what is running now and the time it started.
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
    notice = coverage_notice(exhaustive, filtered)
    if notice:
        print(notice)
    print("=" * 60)

    if timings_json:
        Path(timings_json).write_text(
            json.dumps(timings_payload(results), indent=2) + "\n")
        print(f"  timings written to {timings_json}")

    # ...and the ledger every run appends to, whether or not anybody asked.
    # `--timings-json` is for looking at ONE run; this is what makes the next
    # measurement a comparison instead of another hand-read log.
    ledger = append_cost_ledger(results, exhaustive, filtered)
    if ledger:
        print(f"  cost appended to {ledger.relative_to(REPO) if ledger.is_relative_to(REPO) else ledger}")

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
    # ⭐ The NAME is the feature. An agent reading `--help` or a stale command
    # line should be talked out of it by the flag itself, because being the
    # default is exactly how the exhaustive plan became the reflex.
    ap.add_argument("--run-everything-you-probably-dont-need-this",
                    dest="run_everything", action="store_true",
                    help="the exhaustive plan: a cargo test -p per crate with "
                         "its feature-gated tests, the external-consumer "
                         # ⚠ `%%`, not `%`. argparse runs every action help
                         # string through `%`-formatting, and it read "7% of"
                         # as the octal conversion `% o` — so `--help` itself
                         # died with `TypeError: %o format: an integer is
                         # required, not dict`. A runner whose --help crashes
                         # is a runner nobody asks, which is how the exhaustive
                         # sweep stays the reflex. (2026-08-03, C2.)
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

    if args.fast:
        print("run_tests: --fast is a no-op now — the backbone IS the default. "
              "The exhaustive plan is "
              "--run-everything-you-probably-dont-need-this.")

    jobs = build_jobs(args.package, args.heavy, libtest_args,
                      everything=args.run_everything)
    if args.run_everything or args.heavy:
        print("run_tests: EXHAUSTIVE plan requested. Measured 2026-08-03: "
              "~33 jobs, ~25 minutes, ~17% of it executing tests. If you are "
              "mid-edit, a focused test almost certainly answers your question "
              "faster and just as well.")
    return run(jobs, args.list, timings_json=args.timings_json,
               status_json=args.status_json,
               exhaustive=args.run_everything or args.heavy,
               filtered=bool(args.package))


if __name__ == "__main__":
    sys.exit(main())
