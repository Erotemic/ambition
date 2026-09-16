#!/usr/bin/env python3
"""A headless arm that steps must be able to observe that it stepped.

`add_headless_foundation` installs `MinimalPlugins`, which leaves
`TimeUpdateStrategy::Automatic` in force. Under `Automatic`,
`run_fixed_main_schedule` executes zero or more fixed steps depending on how
much WALL CLOCK passed since the previous update — so a fast `app.update()`
crosses no 1/60 s boundary and runs no fixed step at all. MEASURED and recorded
in `docs/planning/queue.md`: one such probe passed in 0.47 s at 13 MB having run
ZERO fixed steps.

⛔ THIS IS A COVERAGE GUARD, NOT A PERFORMANCE ONE. An arm that neither pins a
timestep nor asserts anything is green whatever the engine does, and its green
is indistinguishable from a broken engine's.

⛔⛤ AND THAT IS NOT HYPOTHETICAL. `b9f2ece18` put `drive_boss_animators` in
`WorldPrep` while it also ran `.after` a system in `CombatSet::Playback` — one
system ordered both before and after Playback, a schedule cycle that retried
forever inside `RunFixedMainLoop` and allocated until the box died. It shipped
on `cargo check` alone, reasoned safe because the arm it touched "does not
step". A schedule cycle is invisible to `cargo check` AND to every arm that
never advances a fixed step. The arms below are where that class hides.

An arm satisfies this guard by EITHER:
  * pinning `TimeUpdateStrategy::ManualDuration`, so its steps are real; or
  * asserting something (`assert*!`, `panic!`, or `#[should_panic]`), so a
    wrong answer can fail it.

Pinning without asserting is allowed here deliberately: a pinned arm that only
builds and steps still proves the schedule can advance, which is exactly the
property `b9f2ece18` broke.
"""
from __future__ import annotations

import re
import sys
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]

FOUNDATION = re.compile(r"add_headless_foundation")
STEP = re.compile(r"\.update\(\)|\.step\(|run_frames|advance_frames")
PIN = re.compile(r"TimeUpdateStrategy::ManualDuration")
ASSERT = re.compile(r"\bassert\w*!|\bpanic!\(")
TEST_ATTR = re.compile(r"#\[(?:tokio::)?test\]")
FN_DEF = re.compile(r"^\s*(?:pub\s+)?(?:async\s+)?fn\s+(\w+)", re.M)

#: Arms that do not satisfy the guard yet. ⛔ THIS LIST MAY ONLY SHRINK.
#: Each entry is a real gap, verified by reading the arm — not a parser
#: artifact. Fix one by pinning the timestep or asserting what it produced.
KNOWN_GAPS: set[tuple[str, str]] = set()


def braced_body(text: str, brace: int) -> str:
    depth, i = 0, brace
    while i < len(text):
        if text[i] == "{":
            depth += 1
        elif text[i] == "}":
            depth -= 1
            if depth == 0:
                return text[brace:i + 1]
        i += 1
    return text[brace:]


def test_arms(text: str):
    """(name, body, should_panic) per real `#[test]` fn.

    ⛔ Anchored on the ATTRIBUTE and scanned FORWARD to the fn it decorates. A
    first pass searched backwards from each `fn` instead and reported helpers
    NESTED INSIDE a test as arms of their own, because the enclosing test's
    `#[test]` was in the window behind them.
    """
    for match in TEST_ATTR.finditer(text):
        rest = text[match.end():]
        sig = FN_DEF.search(rest) or re.search(r"\bfn\s+(\w+)", rest)
        if not sig:
            continue
        between = rest[:sig.start()]
        if re.sub(r"#\[[^\]]*\]|//[^\n]*|/\*.*?\*/|\s", "", between, flags=re.S):
            continue  # something other than attributes sits in between
        if "#[ignore" in between:
            continue  # a declared print-only probe does not run at all
        brace = rest.find("{", sig.end())
        if brace < 0:
            continue
        yield sig.group(1), braced_body(rest, brace), "#[should_panic" in between


def reachable(text: str, body: str) -> str:
    """`body` plus every same-file fn it names.

    ⚠ SAME FILE ONLY. An arm reaching a pin or an assertion through a helper in
    ANOTHER crate reads here as if it had neither, so this guard is scoped to
    `add_headless_foundation` callers — where the composition is built inline —
    rather than to every stepping arm in the workspace.
    """
    out = body
    for helper in FN_DEF.finditer(text):
        name = helper.group(1)
        if re.search(rf"\b{re.escape(name)}\s*(?:::<[^>]*>)?\s*\(", body):
            brace = text.find("{", helper.end())
            if brace >= 0:
                out += braced_body(text, brace)
    return out


def tracked_rust_files() -> list[str]:
    """Every `.rs` file THIS checkout tracks, repo-relative.

    ⛔⛤ **ONE KEEPER, BECAUSE THE TEST BESIDE THIS GUARD SPELLED THE
    ENUMERATION A SECOND TIME.** Its anti-vacuity floor counted files with its
    own `ROOT.glob("**/*.rs")`, which sweeps `.worktrees/` — so the floor was
    inflated by other agents' checkouts and would have stayed green while the
    real scan lost reach. A guard and its own vacuity test disagreeing about the
    corpus is the vacuity test measuring something else.
    """
    return subprocess.run(
        ["git", "-C", str(ROOT), "ls-files", "*.rs"],
        capture_output=True,
        text=True,
        check=True,
    ).stdout.split()


def main() -> int:
    offenders, checked = [], 0
    # ⛔⛤ **TRACKED FILES ONLY — A `**/*.rs` GLOB SWEEPS OTHER AGENTS'
    # CHECKOUTS.** `.worktrees/` holds full copies of this repository at whatever
    # commit another session left them at, and they are not `/target/` so the
    # exclusions below never saw them. MEASURED 2026-09-16: this check reported
    # EIGHT offenders and every one was a stale worktree — arms already fixed in
    # the live tree, which nobody could act on and which would train a reader to
    # ignore the whole report. `git ls-files` answers about THIS checkout.
    tracked = tracked_rust_files()
    # ⛔ ANTI-VACUITY. An empty listing would make every verdict below trivially
    # clean, and a scan root a move silently emptied looks exactly like a
    # repository with no defects.
    if len(tracked) < 500:
        print(
            f"⛔⛔ only {len(tracked)} tracked .rs file(s) found; a clean verdict "
            "over this corpus would be a claim about the scan, not about the tree"
        )
        return 1
    for path in sorted(ROOT / rel for rel in tracked):
        if "/target/" in str(path):
            continue
        try:
            text = path.read_text(errors="replace")
        except OSError:
            continue
        if not FOUNDATION.search(text):
            continue
        relative = str(path.relative_to(ROOT))
        for name, body, should_panic in test_arms(text):
            reach = reachable(text, body)
            if not (FOUNDATION.search(reach) and STEP.search(reach)):
                continue
            checked += 1
            if PIN.search(reach) or ASSERT.search(reach) or should_panic:
                continue
            offenders.append((relative, name))

    if not checked:
        print("⛔ no headless stepping arm was found at all — this guard reads "
              "source, so an empty population means the SCAN broke, not that "
              "the tree is clean.")
        return 1

    found = set(offenders)
    fixed = KNOWN_GAPS - found
    new = found - KNOWN_GAPS

    for where, name in sorted(new):
        print(f"⛔ {where}::{name}")
        print("   steps a headless app but neither pins ManualDuration nor "
              "asserts anything, so it is green whatever the engine does.")
    for where, name in sorted(fixed):
        print(f"✔ FIXED, remove from KNOWN_GAPS: {where}::{name}")

    print(f"\nheadless stepping arms checked: {checked}; "
          f"gaps: {len(found)} (allowed {len(KNOWN_GAPS)})")
    if new:
        print("⛔ a NEW arm that cannot fail. Pin the timestep or assert the result.")
    if fixed:
        print("⛔ the allowlist names an arm that no longer needs it. It may "
              "only shrink; delete those entries.")
    return 1 if (new or fixed) else 0


if __name__ == "__main__":
    sys.exit(main())
