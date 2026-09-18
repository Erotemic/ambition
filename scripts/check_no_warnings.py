#!/usr/bin/env python3
"""Run the normal Cargo check and fail if it emits Rust warnings.

⛔ DEFAULT FEATURES ONLY, and the OK line says so because it was read as more
than it is. Code behind a non-default `#[cfg(feature = ...)]` is not compiled
here at all, so it cannot warn here: on 2026-09-03 this printed clean while the
union build emitted three warnings (two unused imports in the monolith's
`causal.rs`, an unused doc comment in `ladder_probe.rs`), all in gated code.
Extending this run to the feature union is NOT the fix -- that is a full
workspace rebuild of ~25 minutes, which is the wrong price on every dev cycle.
Stating the bound is.

The checker parses diagnostics instead of setting `RUSTFLAGS=-D warnings`, so it
reuses the normal build fingerprint and cache. Cached crates do not re-emit old
warnings; `--fresh` requests the stronger cold-check behavior when needed.

Usage::

    python3 scripts/check_no_warnings.py
    python3 scripts/check_no_warnings.py --fresh
    python3 scripts/check_no_warnings.py -p ambition_app"""

from __future__ import annotations

import argparse
import os
import re
import subprocess
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO / "scripts"))
sys.path.insert(0, str(REPO / "scripts" / "lib"))

from cargo_output import COLOR_NEVER, plain_env, strip_ansi  # noqa: E402
from check_disk_headroom import free_gb_on_target, target_dir  # noqa: E402

# A `cargo check` of this workspace, not a suite. The suite floor is 40 GB; this
# needs enough for whatever is stale.
MIN_FREE_GB = 3.0

CARGO = os.path.expanduser("~/.cargo/bin/cargo")
if not os.path.exists(CARGO):
    CARGO = "cargo"

# In `--message-format=short`, a real diagnostic carries a `path:line:col:` prefix.
_SHORT = re.compile(r"^(?P<where>\S+?:\d+:\d+): warning: (?P<what>.*)$")
# The DEFAULT rendering puts the message at column zero and the location on a
# following line. So does cargo's own per-crate summary, which is why the
# location is what separates them — see [`warnings_from`].
_FULL = re.compile(r"^warning: (?P<what>.*)$")
_LOCATION = re.compile(r"^\s*--> (?P<where>\S+?:\d+:\d+)\s*$")


def warnings_from(stderr: str) -> list[str]:
    """Real diagnostics only — never cargo's per-crate summary lines.

    ⛔⛤ **TWO RENDERINGS, AND THIS GATE DOES NOT CONTROL WHICH ONE ARRIVES.**
    It asks for `--message-format=short`, and for a unit cargo actually
    compiles it gets it. But a FRESH unit REPLAYS ITS CACHED `rendered` STRING,
    which was produced under whatever format built it — so an ordinary
    `cargo check` at the terminal (default format) followed by this gate hands
    the short-format parser the FULL rendering, and the old single pattern saw
    ZERO. Measured 2026-09-18 on `ambition_app`, one dead-code warning, the
    three runs one minute apart:

        cold --message-format=short   `tests/versus_stage.rs:2793:5: warning: …`  seen
        warm --message-format=short   the same line                               seen
        cold default, warm short      `warning: …` + `  --> tests/…:2793:5`       MISSED

    ⇒ The location is what separates a diagnostic from cargo's
    ``warning: `crate` (lib) generated 1 warning`` summary, in BOTH renderings —
    so that is what this reads, instead of a prefix only one of them has.

    ⛔⛤ **AND THE STRIP IS LOAD-BEARING FOR THE SAME REASON.**
    `scripts/run_tests.py` — the ONLY lane that invokes this checker — exports
    `CARGO_TERM_COLOR=always`, which makes cargo emit
    `src/lib.rs:1:18: ESC[1m ESC[33m warning ESC[0m: unused variable`. Measured
    the same day on a one-file probe crate: the pattern matched the plain form
    and not the coloured one. See `scripts/lib/cargo_output.py`; the sibling
    that was CAUGHT doing this is `check_doc_link_ratchet.py`.
    """
    found: list[str] = []
    lines = strip_ansi(stderr).splitlines()
    for index, line in enumerate(lines):
        short = _SHORT.match(line)
        if short:
            found.append(f"{short.group('where')}: {short.group('what').strip()}")
            continue
        full = _FULL.match(line)
        if not full:
            continue
        # ⚠ A SUMMARY HAS NO LOCATION, and that is the whole test. Look only a
        # few lines ahead: rustc puts the `-->` immediately after the message,
        # and a wider window would let one diagnostic's location adopt the
        # summary above it.
        for ahead in lines[index + 1 : index + 3]:
            located = _LOCATION.match(ahead)
            if located:
                found.append(f"{located.group('where')}: {full.group('what').strip()}")
                break
    return found


class VacuousFreshRun(RuntimeError):
    """`--fresh` found nothing to touch, so it would rebuild nothing."""


def fresh_touch_targets(repo: Path) -> list[Path]:
    """Every crate root `--fresh` should touch, in THIS checkout.

    Touching every crate root is cheaper than `clean` and does not throw away the
    dependency graph — only OUR crates recompile.

    ⛔⛤ TRACKED FILES ONLY. `repo.rglob` walks `.worktrees/`, which holds full
    checkouts belonging to OTHER SESSIONS, and this does not read them — it
    TOUCHES them, so a `--fresh` run here would silently force a rebuild in
    somebody else's tree. `git ls-files` answers about this checkout. (The same
    glob defect was found in `check_headless_arms_can_fail.py`, where it only
    mis-reported.)
    """
    listed = subprocess.run(
        ["git", "-C", str(repo), "ls-files", "*/src/lib.rs", "src/lib.rs"],
        capture_output=True,
        text=True,
        check=True,
    ).stdout.split()
    # ⛔ ANTI-VACUITY: touching nothing makes `--fresh` a silent no-op, and a lane
    # that rebuilt nothing reports no warnings for the wrong reason.
    if not listed:
        raise VacuousFreshRun(
            "⛔ --fresh found no tracked `src/lib.rs` to touch, so it would "
            "rebuild nothing and report clean for that reason"
        )
    return [repo / name for name in listed]


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("-p", "--package", action="append", default=[])
    parser.add_argument(
        "--min-gb",
        type=float,
        default=MIN_FREE_GB,
        help=f"free GB required before building (default {MIN_FREE_GB})",
    )
    parser.add_argument(
        "--fresh",
        action="store_true",
        help="rebuild so cached-clean crates re-emit their diagnostics",
    )
    args = parser.parse_args()

    free = free_gb_on_target()
    if free < args.min_gb:
        print(
            f"SKIPPED: {free:.1f} GB free on {target_dir()}, need {args.min_gb:.0f}. "
            "A build started here dies of ENOSPC and reports it as unrelated "
            "compile errors — see scripts/check_disk_headroom.py.",
            file=sys.stderr,
        )
        return 1

    argv = [CARGO, "check", "--all-targets", "--message-format=short", *COLOR_NEVER]
    if args.package:
        for name in args.package:
            argv += ["-p", name]
    else:
        argv.append("--workspace")
    if args.fresh:
        try:
            targets = fresh_touch_targets(REPO)
        except VacuousFreshRun as refusal:
            print(refusal, file=sys.stderr)
            return 1
        for manifest in targets:
            manifest.touch()

    done = subprocess.run(argv, cwd=REPO, capture_output=True, text=True, env=plain_env())
    if done.returncode != 0 and "error" in done.stderr:
        print(done.stderr[-4000:], file=sys.stderr)
        return done.returncode

    found = warnings_from(done.stderr)
    if found:
        print(
            f"{len(found)} warning(s) — CI sets `RUSTFLAGS: -D warnings`, so this "
            "is a RED build there:\n\n  "
            + "\n  ".join(found)
            + "\n\nFix them, or silence one AT THE ITEM with the reason. "
            "⚠ do not reach for `RUSTFLAGS` to enforce this: it is part of "
            "cargo's fingerprint and would rebuild the whole workspace, which is "
            "how this target directory filled the disk three times.",
            file=sys.stderr,
        )
        return 1

    scope = ", ".join(args.package) if args.package else "workspace"
    print(f"OK: {scope} --all-targets compiled with no warnings, under DEFAULT features.")
    print(
        "   \u26a0 Code behind a NON-DEFAULT `#[cfg(feature = ...)]` is not compiled by "
        "this run and is not covered by that OK.\n"
        "     Only the union build sees it: the command `run_tests.py --list "
        "--run-everything-you-probably-dont-need-this` prints under\n"
        "     'one graph, every gated test'. Three warnings were living there on "
        "2026-09-03 while this line read clean (`170d4293d`)."
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
