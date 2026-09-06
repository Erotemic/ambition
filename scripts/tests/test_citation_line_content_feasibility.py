"""Unit tests for `scripts/citation_line_content_feasibility.py`.

⚠ This script records a NEGATIVE result, so its tests guard the two things that
would quietly turn that negative into a fabrication: a resolver that silently finds
nothing, and an empty corpus reporting a clean table.
"""

from __future__ import annotations

import importlib.util
import pathlib

_SCRIPT = pathlib.Path(__file__).resolve().parents[1] / "citation_line_content_feasibility.py"
_spec = importlib.util.spec_from_file_location("citation_line_content_feasibility", _SCRIPT)
mod = importlib.util.module_from_spec(_spec)
assert _spec.loader is not None
_spec.loader.exec_module(mod)


def test_the_resolver_matches_by_suffix_not_exact_path():
    """⛔ The bug this replaced: an exact-path audit calls the repo's partial-path
    citations MISSING, which is a claim about the audit."""
    resolve = mod.resolver(["crates/a/src/semantic.rs", "crates/b/src/other.rs"])
    assert resolve("semantic.rs") == "crates/a/src/semantic.rs"
    assert resolve("crates/a/src/semantic.rs") == "crates/a/src/semantic.rs"


def test_an_ambiguous_suffix_resolves_to_nothing_rather_than_guessing():
    resolve = mod.resolver(["crates/a/src/arena.rs", "game/b/src/arena.rs"])
    assert resolve("arena.rs") is None


def test_near_finds_a_token_inside_the_window_and_not_outside_it(tmp_path):
    src = tmp_path / "s.rs"
    src.write_text("\n".join(f"line{i}" for i in range(1, 51)) + "\n", encoding="utf-8")
    p = str(src)
    assert mod.near(p, 10, {"line10"}, 0)
    assert mod.near(p, 10, {"line13"}, 3)
    assert not mod.near(p, 10, {"line40"}, 3)
    # ⚠ Anti-vacuity: a token that is in NO line must not match at any window,
    # or every measurement above would be an artifact of a permissive search.
    assert not mod.near(p, 10, {"absent_token"}, 30)


def test_the_regexes_pair_a_citation_with_the_tokens_beside_it():
    line = "the wrapper `prepare_the_match` now lives in (`ambition_match/src/prepared.rs:579`)"
    assert mod.CITE.findall(line) == [("ambition_match/src/prepared.rs", "579")]
    toks = {t.split("::")[-1] for t in mod.TOKEN.findall(line) if not t.endswith("rs")}
    assert "prepare_the_match" in toks


def test_a_line_naming_several_things_is_why_the_check_fails():
    """⭐ THE FINDING ITSELF, pinned: prose names SEVERAL things and cites ONE.

    Real example from `awaiting-maintainer-decision.md` — the citation points at
    `windbox`, while the same sentence contrasts it with `is_windbox`. Any checker
    pairing "the citation here" with "the identifiers here" must guess which, and
    guessing wrong is what makes the miss rate noise rather than signal.
    """
    line = "the volume accessor (`ambition_combat/src/strike.rs:94`) and `is_windbox` differ"
    toks = {t.split("::")[-1] for t in mod.TOKEN.findall(line) if not t.endswith("rs")}
    assert len(toks) >= 1
    assert "is_windbox" in toks, "the token that does NOT belong to the citation"
