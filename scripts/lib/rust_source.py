#!/usr/bin/env python3
"""THE one answer to *"which text in this .rs file is code?"* for `scripts/`.

⛔⛔ **A CHECKER THAT GREPS SOURCE READS THE PROSE ABOUT THE CODE AS IF IT WERE
THE CODE**, and this repository has paid for that in both directions. Three
guards in `check_absence_contracts.py` went red on a doc comment explaining a
REMOVAL; `check_capability_ships.py` read its own documentation as evidence that
a resource shipped; and the multi-writer resource census counted a paragraph
saying *"this used to take `ResMut<PortalTuning>`"* as a second WRITER of it.
MEASURED 2026-09-17 on that last one: of 85 multi-writer resources, **3 were
entirely an artefact of prose** (`AcceptedCheckpointRestore`, `PortalTuning`,
`PortalViewer` — their only second writer was a sentence about them) and 9 more
carried a phantom writer file. The population also held a type called `R` and one
called `_`, from `Res<R>/ResMut<R>` in a doc comment and `Option<ResMut<_>>` in a
line comment: a census parsing prose does not fail, it invents.

⛔⛤ **AND THE RULE ALREADY EXISTED SIX TIMES.** `rules/source_reference.rs` has
stripped comments since it was written — *"Comment lines and trailing `//`
comments are always stripped first, so prose…"* — and the Python guards did not
inherit it. Each of them then grew its own: `audio_levels._strip_comments`,
`check_capability_ships._without_comments`,
`check_set_pins_have_engine_members._strip_comments`,
`check_engine_systems_are_engine_installed.strip_comments`,
`check_absence_contracts.strip_comments_for` and the inverse reader
`check_agent_kb.rust_comment_text`. They do not agree: two of them never strip a
`/* */` block at all. This module is the owner the census now imports; the
others are routed as each consumer's counts are measured, which is the same
staging [`test_paths`] was collapsed under.

⚠ **WHAT THIS DELIBERATELY DOES NOT DO: KNOW A STRING LITERAL.** `let u =
"http://x";` has a `//` in it, and stripping to end-of-line would eat real code
after it. That was measured before this became the shared answer rather than
assumed away: across the 1,294 production Rust files there are **zero** lines
where a quote-opened `//` precedes a `ResMut<` on the same line. A consumer whose
question can be answered inside a string literal — a URL census, an asset-path
audit — must not use this, and should say so where it declines to.
"""

from __future__ import annotations

import re

#: `/* … */`, non-greedy and across lines. Replaced with a SPACE rather than
#: nothing: `foo/*c*/(bar)` must not become `foo(bar)` for a caller counting
#: call sites, and no Rust token is split by adding whitespace.
_BLOCK_COMMENT = re.compile(r"/\*.*?\*/", re.S)

#: `//` to end of line, which covers `///` and `//!` without naming them.
_LINE_COMMENT = re.compile(r"//[^\n]*")


def strip_comments(source: str) -> str:
    """`source` with every Rust comment removed, line count preserved.

    Blocks go first, so a `//` inside a `/* */` cannot swallow the block's
    closing `*/` and everything after it on that line.
    """
    return _LINE_COMMENT.sub("", _BLOCK_COMMENT.sub(" ", source))
