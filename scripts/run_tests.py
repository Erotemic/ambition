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
SKIP_FEATURE_JOB = {
    "ambition_app", "ambition_actors",
    "ambition_menu", "ambition_runtime",
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


def build_jobs(only: list[str], heavy: bool, libtest_args: list[str],
               fast: bool = False) -> list[Job]:
    jobs: list[Job] = []
    members = selected_members(only)

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


def run(jobs: list[Job], list_only: bool, timings_json: str | None = None) -> int:
    if list_only:
        print(f"Planned {len(jobs)} job(s):\n")
        for j in jobs:
            print(f"  {j.name}")
            print(f"      {' '.join(j.argv)}")
        return 0

    env = dict(os.environ)
    env.setdefault("RUST_BACKTRACE", "1")
    env.setdefault("CARGO_TERM_COLOR", "always")
    results: list[JobResult] = []
    for j in jobs:
        print(f"\n\033[1m==> {j.name}\033[0m")
        print("    " + " ".join(j.argv))
        start = time.monotonic()
        rc = subprocess.run(j.argv, cwd=j.cwd or REPO, env=env).returncode
        results.append(JobResult(j.name, j.argv, rc == 0, time.monotonic() - start))
        if rc != 0:
            print(f"\033[31m    FAILED ({j.name})\033[0m")

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
    ap.add_argument("cargo_extra", nargs="*",
                    help="args after `--` forwarded to libtest")
    args = ap.parse_args()

    libtest_args = list(args.cargo_extra)
    if args.k:
        libtest_args.insert(0, args.k)

    jobs = build_jobs(args.package, args.heavy, libtest_args, fast=args.fast)
    return run(jobs, args.list, timings_json=args.timings_json)


if __name__ == "__main__":
    sys.exit(main())
