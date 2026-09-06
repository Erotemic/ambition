"""The unread-message census has to be able to go wrong.

⭐ EVERY ONE OF THESE IS A BUG THE SWEEP ACTUALLY HAD while it was written, on a
HAND-BUILT corpus whose answer is known by counting. The live-tree test can only
tell you the number did not change; these say what the instrument requires.
"""

from __future__ import annotations

import importlib.util
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
SCRIPT = REPO / "scripts" / "messages_nothing_reads.py"


def _module():
    spec = importlib.util.spec_from_file_location("messages_nothing_reads", SCRIPT)
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


def test_it_is_green_against_the_tree(capsys) -> None:
    module = _module()
    assert module.main() == 0
    assert "declared with `add_message`" in capsys.readouterr().out


def test_an_empty_population_fails_rather_than_reporting_zero(monkeypatch) -> None:
    module = _module()
    monkeypatch.setattr(module, "rust_files", list)
    assert module.main() == 1


def test_a_reader_split_over_lines_with_a_trailing_comma_counts(tmp_path) -> None:
    """⛔⛔ THE BUG THAT MADE `RoomLoaded` REPORT ZERO READERS. `FreshAttempt`
    declares its reader across five lines and ends the list with a COMMA, so
    `MessageReader<..RoomLoaded>` and `..RoomLoaded\\s*>` both miss it."""
    module = _module()
    text = (
        "loads: bevy::ecs::message::MessageReader<\n"
        "    'w,\n"
        "    's,\n"
        "    ambition_platformer2d_world::rooms::RoomLoaded,\n"
        ">,\n"
    )
    inners = module.generic_inners(text, "MessageReader")
    assert [module.payload(inner) for inner in inners] == ["RoomLoaded"]


def test_a_turbofish_declaration_is_found(tmp_path) -> None:
    """⛔ `add_message::<T>()` has a `::` and `MessageReader<T>` does not. Missing
    it reported ZERO declarations — caught by the anti-vacuity floor, not by me."""
    module = _module()
    inners = module.generic_inners("app.add_message::<LoadEvent>();", "add_message")
    assert [module.payload(inner) for inner in inners] == ["LoadEvent"]


def test_a_comment_is_not_a_reader() -> None:
    """⛔ The sole hit for `MessageReader<LoadEvent>` was a doc comment SAYING
    there is no reader. A census that reads prose certifies the opposite of the
    truth."""
    module = _module()
    stripped = module.strip_line_comments(
        "/// mentions MessageReader<LoadEvent> in prose\nlet x = 1;\n"
    )
    assert module.generic_inners(stripped, "MessageReader") == []


def test_a_generic_parameter_is_not_a_message_type() -> None:
    """`app.add_message::<M>()` inside `fn register<M: Message>` names the
    CALLER's type, not one that exists."""
    module = _module()
    assert module.is_generic_parameter("M")
    assert not module.is_generic_parameter("LoadEvent")


def test_a_messages_handle_that_is_written_is_not_a_reader() -> None:
    """⛔⛔ THE OVER-CORRECTION. Counting every `Messages<T>` as a read road
    resolved two entries wrongly: `select_screen.rs:2520` takes
    `Messages<TouchInput>` to `.write(..)` it — the OPPOSITE of a read."""
    module = _module()
    written = "app.world_mut().resource_mut::<Messages<TouchInput>>().write(x);"
    drained = "world.get_resource_mut::<Messages<RunAuthoredCommand>>().drain();"
    assert module.messages_read_directly(written) == []
    assert [name for name, _ in module.messages_read_directly(drained)] == [
        "RunAuthoredCommand"
    ]


def test_a_read_inside_a_cfg_test_helper_is_not_a_production_reader() -> None:
    """`semantic.rs:926` drains `SemanticActionPressed` in a `#[cfg(test)]`
    helper. A test draining a channel does not make it a wired one."""
    module = _module()
    source = (
        "pub fn live() {}\n"
        "#[cfg(test)]\n"
        "mod tests {\n"
        "    fn drain(app: &mut App) {\n"
        "        app.world_mut().resource_mut::<Messages<Ping>>().drain();\n"
        "    }\n"
        "}\n"
    )
    kept, _unclosed = module.production_lines(source)
    body = "\n".join(line for _number, line in kept)
    assert module.messages_read_directly(body) == []


def test_an_over_matching_reader_detector_fails_rather_than_reporting_a_clean_zero() -> None:
    """⭐⭐ THE OPERAND-FLOOR DEFECT, named by the peer session: a guard whose
    finding is a set DIFFERENCE cannot be protected by flooring one operand.

    `MIN_DECLARED` floors the DECLARED side. The silent-failure direction is the
    other one — a reader matcher that accepts too much, after which every message
    has a "reader", `with NONE: 0` prints, and the run exits 0 looking immaculate.
    A positive control asks the detector about a name that cannot possibly have a
    reader.
    """
    module = _module()
    real = module.generic_inners
    module.generic_inners = lambda text, name: (
        real(text, "add_message") if name == "MessageReader" else real(text, name)
    )
    try:
        assert module.main() == 1
    finally:
        module.generic_inners = real


def test_a_read_through_a_binding_two_lines_down_counts() -> None:
    """⛔⛔ The production reader of `RunAuthoredCommand` binds the handle and
    drains it on the NEXT statement, with a `;` inside an `else` block between —
    so a window cut at the first `;` never saw the verb. It only looked resolved
    while a `tests.rs` elsewhere drained the same type in one statement."""
    module = _module()
    source = (
        "let Some(mut messages) = world.get_resource_mut::<Messages<Ping>>() else {\n"
        "    return;\n"
        "};\n"
        "let requests: Vec<Ping> = messages.drain().collect();\n"
    )
    assert [name for name, _ in module.messages_read_directly(source)] == ["Ping"]


def test_a_binding_that_is_only_written_is_still_not_a_reader() -> None:
    """The binding road must not become a blanket amnesty for `Messages<T>`."""
    module = _module()
    source = (
        "let mut msgs = app.world_mut().resource_mut::<Messages<Pong>>();\n"
        "msgs.write(Pong::default());\n"
    )
    assert module.messages_read_directly(source) == []


def test_a_whole_file_test_module_is_not_production() -> None:
    """`#[cfg(test)] mod tests;` puts the module in its own `tests.rs`, where
    NOTHING inside the file says it is test code — so the positional stripper
    cannot see it and the file name is the signal. An emission test of mine
    drained a message here and the census read it as a production reader."""
    module = _module()
    names = {p.name for p in module.rust_files()}
    assert "tests.rs" not in names


def test_a_fifth_unread_message_must_be_triaged_before_the_number_moves() -> None:
    """⭐⭐ THE DIRECTION THE FLOOR CANNOT SEE. `MIN_DECLARED` catches the sweep
    collapsing; it is blind to someone ADDING a message nothing reads — the count
    goes up, the report prints it, the run exits 0.

    ⚠ This does not make "unread" a finding: a published channel is legitimate and
    the docstring says so. It makes a NEW unread channel a DECISION, the same
    bargain `per_attempt_resource_census.py` strikes.
    """
    module = _module()
    real = module.generic_inners
    try:
        module.generic_inners = lambda text, name: (
            real(text, name) + ["ANewUnreadMessage"]
            if name == "add_message"
            else real(text, name)
        )
        assert module.main() == 1
    finally:
        module.generic_inners = real
