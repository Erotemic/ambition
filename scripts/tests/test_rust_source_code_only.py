"""`rust_source.code_only`: comments and literals blanked, code kept."""

from __future__ import annotations

import pathlib
import re
import sys

REPO = pathlib.Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO / "scripts" / "lib"))

from rust_source import code_only  # noqa: E402

CALL = re.compile(r"\brollback_health\s*\(\s*\)|\bsession_health\b")


def test_prose_naming_a_call_does_not_read_as_the_call():
    """⛔⛤ The false-green hole the GPT review of 2026-09-16 named, and its poison.

    A pattern run against raw source is satisfied by a comment or a string that
    names the call. The first repair was a regex, and a raw string whose inner
    `"` ended its match left the call standing as apparent code. Every Rust
    literal form belongs here: the lesson was "do not extend the regex one form
    at a time".
    """
    prose = (
        "fn arm() {\n"
        "    // session_health should be checked here someday\n"
        '    let msg = "read rollback_health() before asserting";\n'
        "    /* outer /* session_health */ nested */\n"
        '    let raw = r#"foo" rollback_health() "bar"#;\n'
        '    let bytes = br##"session_health"##;\n'
        "    let _ = with_sync_test_rollback_settings(4, 10);\n"
        "}\n"
    )
    assert CALL.search(prose), "the fixture must mention it at all"
    stripped = code_only(prose)
    assert not CALL.search(stripped), stripped
    # ⭐ AND THE CODE SURVIVES. A scanner that ate the file would read as "nothing here".
    assert "with_sync_test_rollback_settings" in stripped
    assert prose.count("\n") == stripped.count("\n"), "line numbers must not move"
    assert len(prose) == len(stripped), "offsets must not move"


def test_a_real_call_still_reads_as_one():
    """The positive control: the strip must not be a way to fail everything."""
    assert CALL.search(code_only("fn arm() { assert!(sim.rollback_health().is_ok()); }"))
    assert CALL.search(code_only("fn arm() { let h = session_health(world); }"))


def test_quote_characters_keep_later_code_and_lifetimes_visible():
    """A quote character must not hide later Rust code."""
    source = r"""fn split<'a>(piece: &'a str) {
        let quote = '\"';
        let brace = '{';
        let escaped = '\u{7d}';
        session_health(world);
    }
    """
    stripped = code_only(source)
    assert CALL.search(stripped)
    assert "split<'a>(piece: &'a str)" in stripped
    assert stripped.count("{") == stripped.count("}") == 1
    assert len(stripped) == len(source)
    assert stripped.count("\n") == source.count("\n")
