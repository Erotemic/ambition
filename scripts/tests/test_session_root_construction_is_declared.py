"""The `Q132` root-construction ratchet must FIRE, and must not count a declaration.

⛔ This guard reports by staying silent, so these arms pin it from outside: a
new constructor, a declared one vanishing, and the tuple-struct declaration
that looks exactly like a spawn to a naive scan.
"""

from __future__ import annotations

import pathlib
import sys

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parents[1]))

import check_session_root_construction_is_declared as guard  # noqa: E402


def test_the_shipped_tree_declares_every_constructor():
    # ⭐ THE RATCHET: it runs against the real tree.
    assert guard.main() == 0


def test_the_tuple_struct_DECLARATION_is_not_a_construction():
    """⚠ `pub struct SessionRoot(pub SessionScopeId);` matches `SessionRoot(`
    exactly as a spawn does. The first scan counted it and read FIVE sites where
    there are four."""
    sources = [("a.rs", "#[derive(Component)]\npub struct SessionRoot(pub SessionScopeId);\n")]
    assert guard.construction_sites(sources) == {}


def test_a_real_construction_is_found():
    sources = [("b.rs", "world.spawn((Name::new(\"x\"), SessionRoot(owner)));\n")]
    assert guard.construction_sites(sources) == {"b.rs": [1]}


def test_a_qualified_construction_is_found():
    """A host in another crate spells the path in full, and that is the case
    this guard most needs to see — it is how a new composition mints a root."""
    sources = [
        ("c.rs", "world.spawn(shared_tangle::lifecycle::SessionRoot(SessionScopeId(7)));\n")
    ]
    assert guard.construction_sites(sources) == {"c.rs": [1]}


def test_the_declared_set_matches_the_tree_exactly():
    found = set(guard.construction_sites(guard.production_sources()))
    assert found == set(guard.DECLARED), (
        f"undeclared: {sorted(found - set(guard.DECLARED))}; "
        f"declared but gone: {sorted(set(guard.DECLARED) - found)}"
    )


def test_the_floor_is_measured_on_this_corpus():
    assert len(guard.production_sources()) >= guard.FLOOR
