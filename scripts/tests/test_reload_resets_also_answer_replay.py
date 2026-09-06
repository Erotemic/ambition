"""The reload-vs-replay guard must fire, must be escapable, and must not go quiet.

It is green against the tree, so a test that only ran it would prove nothing.
These do what the live run cannot:

* reproduce the ACTUAL defect it was written for -- Sanic's monitors rearmed on
  `RoomLoaded` only, so a pit death (which replays in place and emits no
  `RoomLoaded`) left them broken for the rest of the run;
* prove the escape hatch works, because a rule with no exception either becomes a
  lie or gets deleted; and
* prove an emptied corpus FAILS. This is a grep, and a renamed message turns
  every claim it makes into a vacuous pass.
"""

from __future__ import annotations

import importlib.util
import subprocess
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
SCRIPT = REPO / "scripts" / "check_reload_resets_also_answer_replay.py"

LOAD_ONLY = """
/// Rearm the widgets when the room loads.
pub fn rearm_widgets(
    mut rooms: MessageReader<RoomLoaded>,
    mut spent: ResMut<SpentWidgets>,
) {
}
"""

BOTH = """
pub fn rearm_widgets(
    mut rooms: MessageReader<RoomLoaded>,
    mut replays: MessageReader<RoomReplayAdmitted>,
    mut spent: ResMut<SpentWidgets>,
) {
}
"""

EXEMPT = """
/// ALLOW_LOAD_ONLY: an asset cache keyed to a room BUILD, not to an attempt.
pub fn prime_room_cache(
    mut rooms: MessageReader<RoomLoaded>,
    mut cache: ResMut<RoomAssetCache>,
) {
}
"""

READ_ONLY = """
pub fn report_room_loads(
    mut rooms: MessageReader<RoomLoaded>,
    counter: Res<Counter>,
) {
}
"""


def _module():
    spec = importlib.util.spec_from_file_location("check_reload_replay", SCRIPT)
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


def _classify(tmp_path: Path, source: str):
    """Run the signature reader over one synthetic system."""
    module = _module()
    path = tmp_path / "widgets.rs"
    path.write_text(source, encoding="utf-8")
    lineno = next(
        i + 1 for i, line in enumerate(source.splitlines())
        if "MessageReader<RoomLoaded" in line
    )
    return module, module.enclosing_system(path, lineno)


#: A system with the EXACT defect shape whose DOC happens to name the message it
#: fails to read. Until 2026-09-05 this passed: the reader returned the doc and
#: the signature concatenated, so a mention in PROSE satisfied the check.
PROSE_ONLY = """
/// Rearm the widgets. Unlike `RoomReplayAdmitted`, a load means a fresh build.
pub fn rearm_widgets(
    mut rooms: MessageReader<RoomLoaded>,
    mut spent: ResMut<SpentWidgets>,
) {
}
"""


def test_a_load_only_reset_is_a_violation(tmp_path) -> None:
    """The Sanic defect, reproduced: reads the load, mutates state, ignores the
    replay."""
    module, (name, _doc, params) = _classify(tmp_path, LOAD_ONLY)
    assert name == "rearm_widgets"
    assert "ResMut<" in params
    assert module.REPLAY not in params, "this is the shape the guard must reject"


def test_answering_both_messages_passes(tmp_path) -> None:
    module, (_, _doc, params) = _classify(tmp_path, BOTH)
    assert module.REPLAY in params


def test_the_escape_hatch_is_visible_in_the_signature_block(tmp_path) -> None:
    """⚠ Written in the DOC above the signature, which is where a reviewer reads
    it -- so the reader must include that block, not only the parameter list."""
    module, (_, doc, _params) = _classify(tmp_path, EXEMPT)
    assert module.ESCAPE in doc, "the escape hatch must be found in the doc block"


def test_a_reader_that_mutates_nothing_is_not_a_subject(tmp_path) -> None:
    """Nothing to retract means nothing to check; counting it would inflate the
    corpus with systems that can never violate the rule."""
    _, (_, _doc, params) = _classify(tmp_path, READ_ONLY)
    assert "ResMut<" not in params


def test_a_doc_mentioning_the_message_does_not_satisfy_the_check(tmp_path) -> None:
    """⛔⛔ THE HOLE THIS CLOSED, found by poisoning on 2026-09-05. A planted
    load-only system was reported by the WRONG arm, because the reader handed
    back the doc block and the parameters as one string: naming
    `RoomReplayAdmitted` in a COMMENT read as answering it. A system can now
    only satisfy the rule by taking a parameter."""
    module, (_, doc, params) = _classify(tmp_path, PROSE_ONLY)
    assert module.REPLAY in doc, "the fixture must mention it in prose"
    assert module.REPLAY not in params, "prose must not count as answering"


def test_an_emptied_corpus_fails(monkeypatch) -> None:
    """⛔⛔ A CHECK THAT CANNOT FAIL.

    ⭐ THE FLOOR MOVED WITH THE FIX. It used to count `RoomLoaded` readers, and
    the conversion to `FreshAttempt` legitimately emptied that population -- zero
    non-test systems read the message directly now, so counting it would fail
    forever. The live floor counts ADOPTERS of the union type.
    ⚠ And a rename of `RoomLoaded` itself is no longer this script's to catch:
    `FreshAttempt` holds a `MessageReader<RoomLoaded>`, so renaming the message
    without updating it does not compile. That check got stronger, not weaker.
    """
    module = _module()
    monkeypatch.setattr(module, "UNION", "NoSuchTypeExists")
    assert module.main() == 1


def test_it_is_green_against_the_tree() -> None:
    proc = subprocess.run([sys.executable, str(SCRIPT)], capture_output=True, text=True)
    assert proc.returncode == 0, proc.stdout + proc.stderr
