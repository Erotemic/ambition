#!/usr/bin/env python3
"""Does the workspace compile without a single warning?

`.github/workflows/test.yml` sets `RUSTFLAGS: -D warnings`, so any warning is a
red build there. **Nothing local applies that flag** — `cargo check`, `cargo
check --workspace --all-targets`, and the goal's own `cargo check -p
ambition_app` all pass happily with warnings present. On 2026-08-02 five had
accumulated in the tree, two of them from earlier the same session, and the only
thing that would ever have said so was a push.

This closes the asymmetry from the local side.

## why it does NOT set `RUSTFLAGS=-D warnings`, which is the obvious fix

`RUSTFLAGS` is part of cargo's fingerprint. Setting it — in
`.cargo/config.toml`, or per-invocation here — invalidates every artifact built
without it and forces a full rebuild of the workspace. This target directory has
carried ~300 GB of `debug/deps` and has filled the volume to 100% three times
(S14, S14-again, S33). The obvious fix for a warnings gap is a good way to
reproduce the disk outage.

So this runs the SAME `cargo check` everyone else runs — same flags, same
fingerprint, same cache — and reads its diagnostics instead. Warnings are on
stderr whether or not they are fatal; the only thing `-D warnings` adds is the
exit code, and an exit code is something a script can supply for free.

⚠ **the consequence, stated rather than hidden: cargo does not re-emit warnings
for a crate it did not rebuild.** A cached-clean workspace prints nothing and
this check passes, which is correct for "did the last build warn" and is NOT the
same claim as "a cold build would be silent". CI still owns that claim, and
should. `--fresh` forces the stronger version when you have the disk for it.

Usage:
    python3 scripts/check_no_warnings.py
    python3 scripts/check_no_warnings.py --fresh      # touch the tree first
    python3 scripts/check_no_warnings.py -p ambition_app
"""

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

# the FIRST version of this parser had it exactly backwards, and only a probe said so. In
# `--message-format=short` a real diagnostic carries a `path:line:col:` prefix, and a line that
# STARTS with `warning:` at column zero is cargo's per-crate SUMMARY ("`x` (lib) generated 1
# warning", "1 warning emitted"). Matching `^warning:` therefore reported the summaries — which are
# noise, and double-count — while dropping every actual warning.
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
    print(f"OK: {scope} --all-targets compiled with no warnings.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
