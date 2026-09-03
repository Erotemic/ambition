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

from check_disk_headroom import free_gb_on_target, target_dir  # noqa: E402

# A `cargo check` of this workspace, not a suite. The suite floor is 40 GB; this
# needs enough for whatever is stale.
MIN_FREE_GB = 3.0

CARGO = os.path.expanduser("~/.cargo/bin/cargo")
if not os.path.exists(CARGO):
    CARGO = "cargo"

# In `--message-format=short`, real diagnostics carry a `path:line:col:` prefix.
# Column-zero `warning:` lines are Cargo summaries and would double-count.
_WARNING = re.compile(r"^(?P<where>\S+?:\d+:\d+): warning: (?P<what>.*)$", re.M)


def warnings_from(stderr: str) -> list[str]:
    """Real diagnostics only — never cargo's per-crate summary lines."""
    return [f"{m.group('where')}: {m.group('what').strip()}" for m in _WARNING.finditer(stderr)]


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

    argv = [CARGO, "check", "--all-targets", "--message-format=short"]
    if args.package:
        for name in args.package:
            argv += ["-p", name]
    else:
        argv.append("--workspace")
    if args.fresh:
        # Touching every crate root is cheaper than `clean` and does not throw
        # away the dependency graph — only OUR crates recompile.
        for manifest in REPO.rglob("src/lib.rs"):
            manifest.touch()

    done = subprocess.run(argv, cwd=REPO, capture_output=True, text=True)
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
