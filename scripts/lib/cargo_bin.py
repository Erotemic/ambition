#!/usr/bin/env python3
"""`cargo`, found even when PATH does not have it.

⛔⛔ A CHECK THAT CAN ONLY REPORT "command not found" CAN NEVER PASS. This repo
has paid for that lesson three times. The goal-guard hook's PATH has no cargo,
and neither does an agent harness that runs a non-login shell — but rustup's
`~/.cargo/bin/cargo` is right there. A guard invoked as bare `cargo` in that
environment dies with `FileNotFoundError` and reports a failure that is about
the machine, not the code.

⭐ THE LESSON WAS ALREADY WRITTEN DOWN AND ONLY HALF APPLIED. `check_no_warnings.py`,
`run_tests.py`, `test_suite_cost.py` and `check_absence_contracts.py` each
resolved the rustup path — four copies of the same three lines — while
`check_capability_ships.py` and
`scripts/tests/test_sub_workspace_lockfiles_are_current.py` called bare `cargo`
and crashed. Measured 2026-09-02: on this machine `cargo` is not on PATH,
`~/.cargo/bin/cargo` exists, `check_no_warnings.py` passes and
`check_capability_ships.py` exits 1 with a traceback. Same repo, same tool, two
answers to "is cargo available".

⇒ One definition, so the next call site inherits the fix instead of the bug.

⚠ IT DOES NOT VERIFY THAT CARGO WORKS. A returned path is a path; a machine
with no Rust toolchain at all still fails, and that failure is real and should
be reported rather than hidden.
"""

from __future__ import annotations

from pathlib import Path

__all__ = ["cargo_binary"]


def cargo_binary() -> str:
    """The best available `cargo`: rustup's if it exists, else bare `cargo`.

    Falling back to the bare name rather than raising keeps a machine that puts
    cargo somewhere else entirely (a distro package, a nix profile) working
    through PATH, which is the case the rustup path would otherwise break.
    """
    rustup = Path.home() / ".cargo" / "bin" / "cargo"
    return str(rustup) if rustup.exists() else "cargo"
