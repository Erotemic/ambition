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

⭐ **[`code_only`] DOES KNOW ONE.** It blanks comments AND string literals,
length and newlines preserved, for a checker whose pattern a string could
satisfy (a call name quoted in a message). It came here from the sync-test
frozen-world census, which was deleted on 2026-10-02 when the harness began
to refuse an unhealthy session.
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


def code_only(text: str) -> str:
    """`text` with comments and string literals blanked out, newlines preserved.

    ⛔⛤ **THE FROZEN-WORLD CENSUS RAN ITS PATTERNS AGAINST RAW SOURCE, A
    FALSE-GREEN HOLE IN THE UNSAFE DIRECTION — NAMED BY THE GPT ARCHITECTURE
    REVIEW OF 2026-09-16.** Its health pattern CERTIFIED an arm as non-vacuous,
    so a file containing only

    ```text
    // session_health should be checked here someday
    ```

    satisfied the guard while checking nothing.

    ⛔⛤ **AND THE FIRST REPAIR WAS A REGULAR EXPRESSION, WHICH THE SAME REVIEW
    POISONED THROUGH THE NEXT DAY.** It handled `//`, `/* */` and `"..."`, and
    Rust has more literal forms than that:

    ```rust
    let explanation = r#"foo" rollback_health() "bar"#;
    ```

    is a single raw string whose inner `"` ends the pattern's match, leaving
    `rollback_health()` standing as apparent code. ⇒ **Do not extend the regex one
    literal form at a time.** This is a scanner, and it blanks `//` comments,
    NESTED `/* */` comments (Rust allows them), and every string form: plain,
    byte, and raw with an arbitrary `#` count, byte-raw included.

    ⭐ **BOTH PATTERNS ARE STRIPPED, AND THE TWO DIRECTIONS ARE NOT SYMMETRIC.** A
    comment naming `HEALTH` certifies an arm that checks nothing, which is
    silent. A comment naming `SYNC_TEST` only pulls a non-arm INTO the population,
    where it has to be adjudicated by hand — loud, and safe. Both are stripped
    anyway, because a population found by prose is not the population, and `main`
    floors the count so a scanner that ate the file cannot read as "no arms".

    ⚠ Character literals are deliberately NOT handled: no health call fits in
    one, and `'` is also a lifetime, so recognising them costs more than it buys.
    """
    out = []
    i = 0
    n = len(text)

    def blank(chunk: str) -> str:
        return "".join("\n" if ch == "\n" else " " for ch in chunk)

    while i < n:
        ch = text[i]
        if text.startswith("//", i):
            j = text.find("\n", i)
            j = n if j == -1 else j
            out.append(blank(text[i:j]))
            i = j
            continue
        if text.startswith("/*", i):
            depth = 0
            j = i
            while j < n:
                if text.startswith("/*", j):
                    depth += 1
                    j += 2
                elif text.startswith("*/", j):
                    depth -= 1
                    j += 2
                    if depth == 0:
                        break
                else:
                    j += 1
            out.append(blank(text[i:j]))
            i = j
            continue
        # A raw string: an optional `b`, then `r`, then any number of `#`, then `"`.
        m = _RAW_OPEN.match(text, i)
        if m:
            hashes = m.group("hashes")
            close = '"' + hashes
            j = text.find(close, m.end())
            j = n if j == -1 else j + len(close)
            out.append(blank(text[i:j]))
            i = j
            continue
        # A plain or byte string, where a backslash escapes the next character.
        if ch == '"' or (ch == "b" and text.startswith('b"', i)):
            j = i + (2 if ch == "b" else 1)
            while j < n:
                if text[j] == "\\":
                    j += 2
                    continue
                if text[j] == '"':
                    j += 1
                    break
                j += 1
            out.append(blank(text[i:j]))
            i = j
            continue
        out.append(ch)
        i += 1
    return "".join(out)


#: The opening of a raw string: `r"`, `r#"`, `br##"` and so on. The `#` run has
#: to be captured because the CLOSER must match its length — that is the whole
#: reason a regex over the literal cannot do this job.
_RAW_OPEN = re.compile(r'b?r(?P<hashes>#*)"')
