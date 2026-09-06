"""⛔⛔ A TEST THAT CALLS `run()` IS A LEDGER WRITER, AND THE DEFENCE WAS A HAND-KEPT LIST.

`append_cost_ledger` appends one row per suite run to a JSONL corpus inside the
`dev/ambition_dev_measurements` submodule — a corpus planning pages cite for
suite cost. Nothing about that file says it is machine-local: it is tracked, it
is cited, and it looks exactly like shared history.

⚠ THE DEFECT THIS FILE PINS (found 2026-09-03, while committing real rows):
four FIXTURE rows were sitting in the ledger — 0 and 2 jobs at 0.1 s — written
by tests that called `run()` directly. They had to be pruned by hand before the
real rows could be committed, and pruning an append-only file by hand is how two
older rows were silently deleted on the first attempt.

⭐ The defence lived in the THREE tests that remember
`monkeypatch.setattr(run_tests, "append_cost_ledger", ...)`. That is a hand-kept
list guarding a shared corpus: a new test calling `run()` is silently a writer,
and it writes into the developer's real submodule, where the row survives long
enough for someone reading `git status` months later to commit it in good faith.

⇒ So the refusal belongs in the WRITER, where every caller gets it. Same shape
as the two refusals around it: say why, return None, never fail the suite over a
cost record.
"""

from __future__ import annotations

import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO / "scripts"))

import run_tests  # noqa: E402


def _one_result() -> list:
    return [run_tests.JobResult("a job", ["true"], True, 0.1)]


def test_the_writer_refuses_when_pytest_is_running(tmp_path, monkeypatch, capsys):
    """The gap, not the fix: this test does NOT stub the writer, and must still
    leave no row. That is precisely the caller the hand-kept list did not cover."""
    ledger = tmp_path / "run_tests_cost.jsonl"
    monkeypatch.setenv("RUN_TESTS_COST_LEDGER", str(ledger))
    monkeypatch.delenv(run_tests.COST_LEDGER_TEST_WRITES_ENV, raising=False)

    assert run_tests.append_cost_ledger(_one_result(), False, False) is None
    assert not ledger.exists(), "a test wrote a row into the cost corpus"
    assert "cost NOT recorded" in capsys.readouterr().out


def test_the_refusal_names_the_escape_hatch(tmp_path, monkeypatch, capsys):
    """⭐ A refusal a reader cannot act on is how the four fixture rows sat
    unnoticed. It must name the variable that turns writing back on."""
    monkeypatch.setenv("RUN_TESTS_COST_LEDGER", str(tmp_path / "ledger.jsonl"))
    monkeypatch.delenv(run_tests.COST_LEDGER_TEST_WRITES_ENV, raising=False)

    run_tests.append_cost_ledger(_one_result(), False, False)
    assert run_tests.COST_LEDGER_TEST_WRITES_ENV in capsys.readouterr().out


def test_a_test_that_MEANS_to_exercise_the_writer_still_can(tmp_path, monkeypatch):
    """The guard must not make the writer untestable — that would trade one
    unmeasurable thing for another."""
    ledger = tmp_path / "run_tests_cost.jsonl"
    monkeypatch.setenv("RUN_TESTS_COST_LEDGER", str(ledger))
    monkeypatch.setenv(run_tests.COST_LEDGER_TEST_WRITES_ENV, "1")

    assert run_tests.append_cost_ledger(_one_result(), False, False) == ledger
    assert ledger.exists() and ledger.read_text().count("\n") == 1


def test_the_escape_hatch_is_not_the_path_override(tmp_path, monkeypatch):
    """⛔ Redirecting the ledger PATH is what a test does to write somewhere
    safe. If that also re-enabled writing, every future test that redirects the
    path to keep itself clean would silently become a writer again — the same
    hand-kept-list failure, one level down."""
    ledger = tmp_path / "run_tests_cost.jsonl"
    monkeypatch.setenv("RUN_TESTS_COST_LEDGER", str(ledger))
    monkeypatch.delenv(run_tests.COST_LEDGER_TEST_WRITES_ENV, raising=False)

    assert run_tests.append_cost_ledger(_one_result(), False, False) is None
    assert not ledger.exists()
