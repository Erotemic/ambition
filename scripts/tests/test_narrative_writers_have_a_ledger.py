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


def _filler(n):
    """`n` distinct payloads, each paired and each declared exactly once."""
    return "\n".join(
        f"struct T{i};"
        f"\nfn f{i}(w: NarrativeInputWriter<p::T{i}>) {{}}"
        f"\napp.add_plugins(NarrativeInputPlugin::<p::T{i}>::default());"
        for i in range(n)
    )


def test_two_declarations_of_one_leaf_are_reported(monkeypatch, capsys):
    """⚠ THE COST OF MATCHING ON THE LEAF, PINNED. It has to be a leaf match —
    the writer imports the name and the plugin qualifies it — so the unsound
    case is reported rather than assumed away."""
    monkeypatch.setattr(
        guard,
        "production_sources",
        lambda: sources([
            ("a.rs", "struct Clash;\nfn f(w: NarrativeInputWriter<alpha::Clash>) {}"),
            ("b.rs", "enum Clash { X }\n"
                     "app.add_plugins(NarrativeInputPlugin::<beta::Clash>::default());"),
            ("c.rs", _filler(guard.MIN_WRITERS)),
        ]),
    )
    assert guard.main() == 1
    assert "declared 2 times" in capsys.readouterr().out


def test_a_re_export_is_not_a_second_declaration(monkeypatch, capsys):
    """⛔⛤ THE FALSE POSITIVE THAT REPLACED THE OLD RULE, HELD OPEN.

    This is `SpawnActorRequest`'s shape: the writer reaches the type through
    the facade and the plugin names the owning crate, so the two module paths
    differ while exactly one `struct` exists. A path comparison reds here; a
    declaration count must not.
    """
    monkeypatch.setattr(
        guard,
        "production_sources",
        lambda: sources([
            ("owner.rs", "struct SpawnX;"),
            ("facade.rs", "pub use owner::SpawnX;\n"
                          "fn f(w: NarrativeInputWriter<facade::actor::SpawnX>) {}"),
            ("host.rs", "app.add_plugins(NarrativeInputPlugin::<owner::SpawnX>::default());"),
            ("c.rs", _filler(guard.MIN_WRITERS)),
        ]),
    )
    assert guard.main() == 0
    assert "SpawnX" not in capsys.readouterr().out


def test_a_paired_leaf_with_no_declaration_is_reported(monkeypatch, capsys):
    """A leaf nothing declares is the shape an EXTERNAL type would arrive in,
    and the premise cannot be checked for it — so it is reported, not passed."""
    monkeypatch.setattr(
        guard,
        "production_sources",
        lambda: sources([
            ("a.rs", "fn f(w: NarrativeInputWriter<far::Away>) {}"
                     "\napp.add_plugins(NarrativeInputPlugin::<far::Away>::default());"),
            ("c.rs", _filler(guard.MIN_WRITERS)),
        ]),
    )
    assert guard.main() == 1
    assert "premise of that pairing cannot be checked" in capsys.readouterr().out


def test_a_rotted_regex_refuses_instead_of_passing(monkeypatch, capsys):
    monkeypatch.setattr(guard, "WRITER", re.compile(r"NoSuchWriter<([^,>]+)>"))
    assert guard.main() == 1
    assert "claim about this scan" in capsys.readouterr().out
