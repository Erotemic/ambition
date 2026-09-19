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
    # ⚠ body AND raw are the same text for a synthetic fixture: nothing here is
    # stripped, so the line map is the identity and the arms stay about rules.
    return [(pathlib.Path(guard.REPO / p), body, body) for p, body in pairs]


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


def test_reported_lines_address_the_real_file_not_the_stripped_body():
    """⛔⛤ THE CITATIONS WERE OFF BY HUNDREDS OF LINES (found 2026-09-19).

    The five actor-monolith registrations were reported at
    `features/mod.rs:956-960`; they are at `:1401-1405`. `strip_test_modules`
    deletes 482 lines from that file and the line was counted in the stripped
    text, so every citation a failure printed pointed at whatever now sits at
    the shifted line. This arm resolves each reported citation against the file
    on disk and requires the payload to actually be there.
    """
    writers, plugins = guard.scan(guard.production_sources())
    checked = 0
    for name, sites in list(writers.items()) + list(plugins.items()):
        for site in sites:
            path, _, line = site.rpartition(":")
            text = (guard.REPO / path).read_text(encoding="utf-8").split("\n")
            # The generic wraps in the shipped spellings, so the leaf may be on
            # the line after the one the match starts on.
            window = "\n".join(text[int(line) - 1 : int(line) + 2])
            assert name in window, (
                f"{site} does not name `{name}` in the file on disk; the citation "
                f"was computed on a body with test modules cut out of it"
            )
            checked += 1
    assert checked >= 20, f"only {checked} citations resolved, population collapsed"


def test_the_line_map_survives_a_stripped_module_in_the_middle():
    """⛔⛤ THE ARM THE REAL TREE COULD NOT PROVIDE.

    `test_reported_lines_address_the_real_file_not_the_stripped_body` passed
    against a greedy character walk that was wrong — the shipped citations
    happened to land, and a poisoned declaration at line 733 reported as 595.
    So this builds the case directly: a test module in the MIDDLE, and a
    subject after it whose true line is known by construction.

    ⚠ The decoy inside the stripped module matters. A greedy walk drifts by
    finding the characters it needs inside the removed region, so a region that
    contains a plausible match is what separates the two implementations.
    """
    import sys as _sys

    _sys.path.insert(0, str(pathlib.Path(guard.REPO) / "scripts" / "lib"))
    from test_paths import strip_test_modules

    raw = "\n".join(
        ["// line 1", "pub struct Before;"]
        + ["#[cfg(test)]", "mod tests {", "    pub struct Decoy;"]
        + [f"    // filler {i}" for i in range(20)]
        + ["}", "", "pub struct After;", ""]
    )
    true_line = raw.split("\n").index("pub struct After;") + 1
    assert true_line == 28, f"fixture drifted: {true_line}"

    stripped = strip_test_modules(raw)
    offset = stripped.index("pub struct After;")
    assert "Decoy" not in stripped, "the fixture's module was not stripped at all"

    mapped = guard.raw_line_numbers(raw, [offset])
    assert mapped[offset] == true_line, (
        f"the map put `After` on line {mapped[offset]}, and it is on {true_line}. "
        "A greedy subsequence walk lands inside the stripped module"
    )
