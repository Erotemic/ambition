"""The replay-reader census has to be able to go wrong.

It reports rather than enforces, on purpose -- the guard it looks like it should
be ("every content reader is in `ContentRoomReplayResetSet`") is FALSE, because
readers legitimately sit in four different slots. So what is left to protect is
the measurement itself:

* an empty corpus must FAIL, not report a calm zero; and
* it must count registration SITES as sites. One `.in_set(..)` can carry a tuple,
  and calling sites "systems" undercounts the membership -- 2 sites, 3 systems.
"""

from __future__ import annotations

import importlib.util
import subprocess
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
SCRIPT = REPO / "scripts" / "room_replay_reader_slots.py"


def _module():
    spec = importlib.util.spec_from_file_location("room_replay_reader_slots", SCRIPT)
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


def test_an_absent_message_type_fails_rather_than_reporting_zero(monkeypatch) -> None:
    """⛔⛔ If the message is ever renamed, a grep-based census reports zero
    readers and every claim built on it becomes trivially true."""
    module = _module()
    monkeypatch.setattr(module, "MESSAGE", "NoSuchMessageTypeExists")
    assert module.main() == 1


def test_it_reports_the_live_population(capsys) -> None:
    """⛔ THE FIRST VERSION ASSERTED THAT STRINGS APPEARED, which is false only if
    somebody deletes a `print`. It could not tell a correct census from a broken
    one. The peer session's rule names the fix: say what edit would make the
    assertion FALSE, and if the answer is "delete the print", the arm is
    decoration.
    """
    module = _module()
    assert module.main() == 0
    out = capsys.readouterr().out

    readers = int(out.split("readers of RoomReplayAdmitted:")[1].split()[0])
    sites = int(out.split("registration SITES into ContentRoomReplayResetSet:")[1].split()[0])

    # ⭐ The claim this census exists to support: the retraction list is NOT one
    # system. A stale note said it was, and counting SITES is how that survived --
    # so the arm fails if the population collapses back to something a reader
    # could mistake for "just cut-rope".
    assert readers >= 6, (
        f"only {readers} readers of the admitted replay — the census has stopped "
        "seeing the population it reports on"
    )
    assert sites >= 2, (
        f"{sites} registration site(s): the row this census corrected said the "
        "set held ONE system, and fewer than two sites cannot demonstrate "
        "otherwise"
    )
    # ⚠ And the distinction itself, because a site can carry a TUPLE: the printed
    # line must keep saying which of the two it counted.
    assert "SITES" in out and "TUPLE" in out


def test_git_grep_flags_precede_the_pattern() -> None:
    """⚠ `git grep <pattern> -n` reads `-n` as a REVISION and dies; a
    `returncode not in (0,1)` guard then swallows that into a confident zero.
    This pins the call shape rather than trusting it stays right."""
    module = _module()
    assert module.git_grep("RoomReplayAdmitted", "crates/"), "census found nothing"


def test_the_script_runs_clean_end_to_end() -> None:
    proc = subprocess.run([sys.executable, str(SCRIPT)], capture_output=True, text=True)
    assert proc.returncode == 0, proc.stderr
