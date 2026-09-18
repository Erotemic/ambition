"""Tests for the warning parser against representative Cargo short-format output.

The parser must return the actual diagnostics rather than Cargo's per-crate
warning summary lines, and planted warning output must both fail the guard and
name the underlying diagnostic."""

from __future__ import annotations

import subprocess
import sys
import tempfile
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO / "scripts"))

from check_no_warnings import (  # noqa: E402
    REPO as CHECKED_REPO,
    VacuousFreshRun,
    fresh_touch_targets,
    warnings_from,
)

# Captured from `cargo check -p ambition_geometry --all-targets
# --message-format=short` with one unused import planted.
REAL_STDERR = """\
    Checking ambition_geometry v0.1.0 (/home/joncrall/code/ambition/crates/ambition_geometry)
crates/ambition_geometry/src/lib.rs:53:5: warning: unused import: `std::collections::BTreeSet as _ProbeUnused`
warning: `ambition_geometry` (lib) generated 1 warning (run `cargo fix --lib -p ambition_geometry` to apply 1 suggestion)
warning: `ambition_geometry` (lib test) generated 1 warning (1 duplicate)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.53s
"""


def test_the_real_diagnostic_is_reported_once_with_its_location():
    found = warnings_from(REAL_STDERR)
    assert len(found) == 1, f"expected exactly the one diagnostic, got {found}"
    assert "crates/ambition_geometry/src/lib.rs:53:5" in found[0]
    assert "unused import" in found[0]


def test_a_coloured_build_is_still_counted():
    """⛔⛤ **THE ANCHOR THIS GATE MATCHES ON CAN BE PUSHED OFF THE FRONT OF THE
    LINE, AND THE ONLY LANE THAT RUNS THIS GATE IS WHAT PUSHES IT.**
    `scripts/run_tests.py` exports `CARGO_TERM_COLOR=always` to every child job,
    and `_WARNING` requires a literal `: warning: ` after `path:line:col`. Under
    colour cargo writes `: ESC[1m ESC[33m warning ESC[0m: ...` instead, the
    pattern matches nothing, and a workspace full of warnings reports clean.

    ⚠ The coloured bytes are COPIED FROM A REAL `cargo check --all-targets
    --message-format=short` (2026-09-18, a one-file probe crate), not composed —
    a hand-written escape is a guess about which codes rustc picks and where the
    reset lands. The sibling guard this was found in is
    `check_doc_link_ratchet.py`, which scored 0 of 13 crates against a baseline
    of 141 for exactly this reason.

    The assertion is EQUALITY WITH THE PLAIN READING: a parser that noticed the
    coloured diagnostic but kept an escape inside the reported location would
    print an identity nothing can grep for.
    """
    plain = (
        "src/lib.rs:1:18: warning: unused variable: `x`: help: if this is "
        "intentional, prefix it with an underscore: `_x`\n"
        "warning: `warnprobe` (lib) generated 1 warning\n"
    )
    coloured = (
        "src/lib.rs:1:18: \x1b[1m\x1b[33mwarning\x1b[0m: unused variable: `x`: "
        "help: if this is intentional, prefix it with an underscore: `_x`\n"
        "\x1b[1m\x1b[33mwarning\x1b[0m: `warnprobe` (lib) generated 1 warning\n"
    )
    assert warnings_from(plain) == warnings_from(coloured)
    assert len(warnings_from(coloured)) == 1, warnings_from(coloured)


def test_cargos_per_crate_SUMMARY_lines_are_not_warnings():
    """⛔ the inverted-parser regression, pinned.

    Both summary lines describe the SAME diagnostic — one of them says
    "(1 duplicate)" outright — so counting them reports two problems where there
    is one, and neither row says where to look.
    """
    for line in REAL_STDERR.splitlines():
        if line.startswith("warning: "):
            assert warnings_from(line) == [], f"summary counted as a warning: {line}"


def test_a_clean_build_reports_nothing():
    clean = (
        "    Checking ambition_geometry v0.1.0 (/repo/crates/ambition_geometry)\n"
        "    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.61s\n"
    )
    assert warnings_from(clean) == []


def test_several_diagnostics_are_each_reported():
    stderr = (
        "a/src/lib.rs:1:5: warning: unused import: `Foo`\n"
        "b/src/lib.rs:9:1: warning: method `bar` is never used\n"
        "warning: `a` (lib) generated 1 warning\n"
    )
    found = warnings_from(stderr)
    assert len(found) == 2
    assert any("unused import" in f for f in found)
    assert any("never used" in f for f in found)


def test_fresh_touches_only_files_tracked_in_this_checkout():
    """⛔ THE DEFECT IS INVISIBLE UNTIL A `.worktrees/` EXISTS, which is why it
    needs an arm rather than a reading. `--fresh` does not READ what it finds, it
    TOUCHES it, so a `rglob` that walks another session's checkout forces a
    rebuild in somebody else's tree and nothing here says so.
    """
    targets = fresh_touch_targets(CHECKED_REPO)
    # ⛔ ANTI-VACUITY: an empty list satisfies every assertion below.
    assert len(targets) > 50, (
        f"only {len(targets)} crate roots resolved, so this arm is asserting "
        "about almost nothing — check that `git ls-files` still reaches them"
    )
    assert all(path.exists() for path in targets)
    # ⚠ This one cannot fire in a tree with no `.worktrees/`, so it is vigilance
    # rather than safety — the arm below is the one that bites today.
    assert not any(".worktrees" in path.parts for path in targets)


def test_an_untracked_worktree_checkout_is_not_touched():
    """⛔ THE ARM THAT ACTUALLY FIRES. The tree this runs in has no
    `.worktrees/`, so asserting its absence proves nothing. This BUILDS one — a
    second checkout's `src/lib.rs` that git does not track — and requires the
    enumeration to leave it alone. `rglob` returns it; `git ls-files` does not.
    """
    with tempfile.TemporaryDirectory() as root:
        repo = Path(root)
        subprocess.run(["git", "init", "-q", str(repo)], check=True)
        mine = repo / "crates" / "a" / "src"
        mine.mkdir(parents=True)
        (mine / "lib.rs").write_text("// mine\n")
        theirs = repo / ".worktrees" / "peer" / "crates" / "b" / "src"
        theirs.mkdir(parents=True)
        (theirs / "lib.rs").write_text("// somebody else's session\n")
        subprocess.run(
            ["git", "-C", str(repo), "add", "crates/a/src/lib.rs"], check=True
        )

        targets = fresh_touch_targets(repo)

        # The premise: the peer's file is really there, so a scan COULD find it.
        assert (theirs / "lib.rs").exists()
        assert targets == [mine / "lib.rs"], (
            f"expected only this checkout's crate root, got {targets}. A "
            "`--fresh` run does not read what it finds, it TOUCHES it, so "
            "reaching into `.worktrees/` forces a rebuild in another session"
        )


def test_a_fresh_run_that_would_touch_nothing_is_a_refusal():
    """The other half: a rebuild of nothing reports no warnings for the wrong
    reason, and that reads exactly like a clean tree."""
    with tempfile.TemporaryDirectory() as empty:
        repo = Path(empty)
        subprocess.run(["git", "init", "-q", str(repo)], check=True)
        try:
            fresh_touch_targets(repo)
        except VacuousFreshRun:
            return
        raise AssertionError(
            "a checkout with no tracked `src/lib.rs` was accepted, so `--fresh` "
            "would rebuild nothing and the lane would report clean"
        )
