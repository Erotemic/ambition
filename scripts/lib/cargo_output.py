"""One owner of *"read cargo's output as text"* — colour is not cosmetic here.

⛔⛤ **WHY THIS MODULE EXISTS: `CARGO_TERM_COLOR=always` TURNS A LINE-ANCHORED
WARNING PARSER INTO A SILENT ZERO, AND `scripts/run_tests.py` EXPORTS IT TO
EVERY CHILD JOB** (`env.setdefault("CARGO_TERM_COLOR", "always")`). Under it a
rustdoc warning arrives as `ESC[1m ESC[33m warning ESC[0m: unresolved link to
...` and cargo's `--message-format=short` diagnostic as
`src/lib.rs:1:18: ESC[1m ESC[33m warning ESC[0m: unused variable`, so a pattern
anchored on `^warning:` or requiring a literal `: warning: ` matches nothing.
Measured 2026-09-18: `check_doc_link_ratchet.py` scored 0 of 13 crates inside
`--maintenance` against a baseline of 141 named broken links, printed *"⭐ 42
repaired"* down the table, and advised `--update`, which would have retired the
ratchet. The same command from a plain shell scored 141.

⚠ **TWO HALVES, AND BOTH ARE NEEDED.** [`plain_env`] and [`COLOR_NEVER`] stop
cargo colouring in the first place; [`strip_ansi`] makes the reading survive a
colour source they do not reach — `RUSTDOCFLAGS=--color=always`, a wrapper that
allocates a pty, a future cargo that colours through a pipe. A parser that only
asks nicely fails closed the day something else answers.
"""

from __future__ import annotations

import os
import re

#: SGR ("select graphic rendition") is the only escape family cargo and rustc
#: emit; they do not move the cursor or clear the screen. Matching the whole
#: CSI space instead would also eat text that merely looks like one.
SGR = re.compile(r"\x1b\[[0-9;]*m")

#: Appended to a cargo argv. The FLAG beats the environment variable, so a
#: caller that passes both cannot be defeated by whichever cargo decides wins.
COLOR_NEVER = ("--color", "never")


def strip_ansi(text: str) -> str:
    """`text` with SGR colour escapes removed."""
    return SGR.sub("", text)


def plain_env(base: dict[str, str] | None = None) -> dict[str, str]:
    """`base` (default `os.environ`) with cargo's colour forced off.

    Returned as a plain dict rather than mutating the caller's environment: the
    child is the only process that must not colour, and a parent that edits its
    own `os.environ` changes every later subprocess too.
    """
    env = dict(os.environ if base is None else base)
    env["CARGO_TERM_COLOR"] = "never"
    return env
