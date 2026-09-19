"""The pairing guard must FIRE on the defect a review found, and stay quiet otherwise.

⛔ Every rule in that module reports by staying silent, so these arms pin the
rules from outside: a rotted regex, a comment read as code, and a leaf match
that is no longer sound all look identical to a healthy tree.
"""

from __future__ import annotations

import pathlib
import re
import sys

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parents[1]))

import check_narrative_writers_have_a_ledger as guard  # noqa: E402


def sources(pairs):
    return [(pathlib.Path(guard.REPO / p), body) for p, body in pairs]


def test_the_shipped_tree_pairs_every_writer():
    # ⭐ THE ARM THAT MAKES THIS A RATCHET: it runs against the real tree.
    assert guard.main() == 0


def test_a_writer_with_no_plugin_is_caught():
    """⛔⛤ THE SHIPPED DEFECT, EXACTLY. `cmd_watch_cut_rope_video` took a
    `NarrativeInputWriter<SetFlagRequested>` and nothing installed the plugin,
    so its `ResMut<NarrativeInputLedger<_>>` could not resolve and the authored
    Yarn command recorded nothing."""
    writers, plugins = guard.scan(
        sources([("a.rs", "fn f(w: NarrativeInputWriter<SetFlagRequested>) {}")])
    )
    assert "SetFlagRequested" in writers
    assert "SetFlagRequested" not in plugins


def test_the_message_channel_is_not_the_missing_half():
    """⚠ WHY IT HID. `add_message::<SetFlagRequested>()` is registered by the
    engine, so the obvious check answered yes. This guard must not be satisfied
    by a channel registration."""
    writers, plugins = guard.scan(
        sources([
            ("a.rs", "fn f(w: NarrativeInputWriter<Foo>) {}\napp.add_message::<Foo>();"),
        ])
    )
    assert "Foo" in writers and "Foo" not in plugins


def test_a_comment_naming_a_writer_is_not_a_use():
    """⛔⛤ THIS GUARD'S OWN FIRST POISON RUN FAILED HERE.

    The repair left a comment beside the fixed registration naming
    `NarrativeInputWriter<SetFlagRequested>` in prose, and the scan reported it
    as a second call site — so a comment explaining a defect would have become
    a requirement to fix it again.
    """
    body = guard.code_only("// see NarrativeInputWriter<GhostPayload> for why\n")
    writers, _ = guard.scan(sources([("a.rs", body)]))
    assert "GhostPayload" not in writers


def test_crate_prefixed_and_fully_qualified_spellings_are_one_type():
    """⛔ THE AMBIGUITY RULE REDDENED ON ITS AUTHOR WITHOUT THIS.

    `actor_monolith` spells its own payloads `crate::features::BrainCommand`;
    every other crate spells them
    `ambition_platformer2d_actor_monolith::features::BrainCommand`. Reading
    those as two module paths reported three collisions that do not exist.
    """
    resolved = guard.resolve_crate_prefix(
        "crate::features::BrainCommand",
        guard.REPO / "crates/ambition_platformer2d_actor_monolith/src/features/mod.rs",
    )
    assert resolved == "ambition_platformer2d_actor_monolith::features::BrainCommand"


def test_two_real_types_sharing_a_leaf_are_reported(monkeypatch, capsys):
    """⚠ THE COST OF MATCHING ON THE LEAF, PINNED. It has to be a leaf match —
    the writer imports the name and the plugin qualifies it — so the unsound
    case is reported rather than assumed away."""
    monkeypatch.setattr(
        guard,
        "production_sources",
        lambda: sources([
            ("a.rs", "fn f(w: NarrativeInputWriter<alpha::Clash>) {}"),
            ("b.rs", "app.add_plugins(NarrativeInputPlugin::<beta::Clash>::default());"),
            ("c.rs", "\n".join(
                f"fn f{i}(w: NarrativeInputWriter<p::T{i}>) {{}}"
                f"\napp.add_plugins(NarrativeInputPlugin::<p::T{i}>::default());"
                for i in range(guard.MIN_WRITERS)
            )),
        ]),
    )
    assert guard.main() == 1
    assert "more than one module path" in capsys.readouterr().out


def test_a_rotted_regex_refuses_instead_of_passing(monkeypatch, capsys):
    monkeypatch.setattr(guard, "WRITER", re.compile(r"NoSuchWriter<([^,>]+)>"))
    assert guard.main() == 1
    assert "claim about this scan" in capsys.readouterr().out
