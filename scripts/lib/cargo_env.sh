#!/usr/bin/env bash
# Put the rustup toolchain on PATH when the calling shell does not already have it.
#
# ⛔ THE README PROMISES TWO COMMANDS IN ONE SHELL, and rustup cannot keep that
# promise on its own:
#
#     ./run_developer_setup.sh
#     ./run_game.sh
#
# The installer appends to `~/.profile`/`~/.bashrc`, which the ALREADY-RUNNING
# shell has long since read, and setup sources `~/.cargo/env` only into its own
# process. So on the machine that just installed Rust — the fresh clone this
# sequence is written for, and the only one where it matters — the second
# command dies with `cargo: command not found` until a new terminal is opened.
#
# ⚠ It is not only the entry points. Tests under `scripts/tests` shell out to a
# BARE `cargo` (sub-workspace lockfiles, capability shipping), so an unsourced
# environment reports them as eight red tests rather than as a missing PATH.
#
# Never overrides a cargo the caller already has: a contributor on a pinned or
# non-rustup toolchain keeps it.
if ! command -v cargo >/dev/null 2>&1; then
    if [ -f "${CARGO_HOME:-$HOME/.cargo}/env" ]; then
        # shellcheck disable=SC1091
        . "${CARGO_HOME:-$HOME/.cargo}/env"
    fi
fi
