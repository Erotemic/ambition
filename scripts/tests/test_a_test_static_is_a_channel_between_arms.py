"""The test-side static census, against corpora with known answers.

⛔ Every arm that can build its own corpus does. The repository arm is the
ratchet and is separate, so a new shared `static` fails exactly one test and the
message names it.
"""

from __future__ import annotations

import importlib.util
import pathlib
import sys

REPO = pathlib.Path(__file__).resolve().parents[2]


def _load():
    name = "a_test_static_is_a_channel_between_arms"
    spec = importlib.util.spec_from_file_location(name, REPO / "scripts" / f"{name}.py")
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


CHECK = _load()


def _statics(text: str) -> list[str]:
    """⚠ Through `statics_in`, the SAME function the repository scan calls.

    An earlier version re-spelled the scan here and a poison on the production
    call site left every arm below green.
    """
    return [name for name, _ in CHECK.statics_in(text)]


def test_a_shared_counter_is_reported():
    """The exact shape that broke two arms: a cadence phase in a `static`."""
    assert _statics(
        "fn cadence() -> AgentAction {\n"
        "    static N: AtomicUsize = AtomicUsize::new(0);\n"
        "    let n = N.fetch_add(1, Ordering::SeqCst);\n"
        "}"
    ) == ["N"]


def test_a_thread_local_cell_is_the_remedy_and_is_not_reported():
    """⛔ `thread_local!` DECLARES ITS CELLS WITH `static`, one per line.

    The first version of the checker reported two of them as channels, because
    the macro opens a block and the `static` lines sit inside it.
    """
    assert (
        _statics(
            "thread_local! {\n"
            "    static SABOTAGE: std::cell::Cell<Sabotage> = const { Cell::new(None) };\n"
            "    static OTHER: std::cell::Cell<u8> = const { Cell::new(0) };\n"
            "}"
        )
        == []
    )


def test_a_paren_form_thread_local_is_also_excluded():
    assert _statics("thread_local!(static UNIQUE: Cell<u64> = const { Cell::new(0) });") == []


def test_an_immutable_static_is_not_a_channel():
    assert _statics('static ROOM: &str = "central_hub_complex";\nstatic STEPS: usize = 120;') == []


def test_a_lock_and_a_sequence_are_still_reported_for_adjudication():
    """They are legitimate, but the DECISION is what makes them so."""
    assert _statics(
        "static TEST_DIR_LOCK: Mutex<()> = Mutex::new(());\n"
        "static FAILURE_DUMP_SEQ: AtomicU64 = AtomicU64::new(0);"
    ) == ["TEST_DIR_LOCK", "FAILURE_DUMP_SEQ"]


def test_the_repository_has_no_unadjudicated_test_side_static():
    statics = CHECK.test_side_statics()
    # ⛔ THE FLOOR. Every filter this census inherits removes files.
    assert statics, "the test corpus yielded no static at all"
    loose = [row for row in statics if (row[0], row[1]) not in CHECK.ADJUDICATED]
    assert not loose, f"unadjudicated test-side static(s): {loose}"


def test_every_adjudicated_row_still_exists():
    """⚠ A row whose static was deleted excuses nothing and hides the next one."""
    present = {(row[0], row[1]) for row in CHECK.test_side_statics()}
    stale = sorted(set(CHECK.ADJUDICATED) - present)
    assert not stale, f"adjudicated static(s) that no longer exist: {stale}"
