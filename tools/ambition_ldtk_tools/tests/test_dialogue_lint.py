"""Tests for the Yarn markup lint (`ambition_ldtk_tools dialogue lint`).

The lint is the fast pre-flight twin of the authoritative Rust guard
`ambition_actors::dialog_lint::no_malformed_yarn_markup_tags`; both
encode yarnspinner's open/self-close markup grammar so a bracketed stage
direction (`[MULTIPLE VOICES]`) is caught before it panics the running game.
"""

from __future__ import annotations

import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(REPO_ROOT / "tools" / "ambition_ldtk_tools"))

from ambition_ldtk_tools.dialogue_lint import (  # noqa: E402
    lint_text,
    main,
    markup_inner_well_formed,
)


def test_well_formed_classifier_accepts_real_markup():
    # Tags the codebase actually authors.
    for ok in ("shout", "/shout", "b", "/b", "/", "wave speed=10", "select 1=a 2=b", "x/"):
        assert markup_inner_well_formed(ok), f"[{ok}] should be well-formed"


def test_well_formed_classifier_flags_bare_words():
    # The reported crash and relatives: a bracketed token without `=`.
    for bad in ("MULTIPLE VOICES", "STAGE DIRECTION", "a b c", ""):
        assert not markup_inner_well_formed(bad), f"[{bad}] should be flagged"


def test_lint_text_reports_line_and_ignores_escaped_and_valid():
    text = "\n".join(
        [
            "title: t",
            "---",
            "Agent Swarm: [MULTIPLE VOICES] hello",  # line 3: bad
            "Guide: [shout]LOUD[/shout]",  # valid markup
            r"Note: \[escaped] is literal",  # escaped — not markup
            "===",
        ]
    )
    violations = lint_text("x.yarn", text)
    assert len(violations) == 1
    assert "x.yarn:3:" in violations[0]
    assert "[MULTIPLE VOICES]" in violations[0]


def test_repo_dialogue_passes(capsys):
    # The real dialogue tree must be clean (regression guard).
    assert main([]) == 0
    assert "no malformed markup" in capsys.readouterr().out
