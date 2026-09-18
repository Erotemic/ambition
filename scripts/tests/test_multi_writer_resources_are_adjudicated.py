"""The multi-writer ratchet must match the tree it ships against, and be able to fail."""

from __future__ import annotations

import pathlib
import sys

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parents[1]))

import check_multi_writer_resources_are_adjudicated as guard  # noqa: E402
import multi_writer_resource_census as census  # noqa: E402


def test_the_baseline_describes_the_tree_today():
    # ⭐ THE RATCHET ARM. It is the whole point: this reddens the moment a
    # resource gains a writer, which is how a duplicated authority arrives.
    assert guard.main() == 0


def test_the_baseline_is_a_population_and_not_a_placeholder():
    # ⛔ A baseline of three would pass the arm above and ratchet nothing.
    assert len(guard.BASELINE) > 50
    assert all(count >= 2 for count in guard.BASELINE.values())


def test_every_adjudication_cites_something():
    # ⚠ An entry in ADJUDICATED is a CITATION, not an opinion. A reason that
    # names no row and no contract is how an amnesty list grows.
    for name, reason in guard.ADJUDICATED.items():
        assert name in guard.BASELINE, name
        assert len(reason) > 60, name
        assert "`" in reason, f"{name}: no row or symbol cited"


def test_a_corpus_that_collapsed_refuses_a_verdict(monkeypatch, capsys):
    """⛔⛔ EVERY FINDING HERE IS A SET DIFFERENCE, AND TWO EMPTY SETS AGREE.

    Without the floors, a `git ls-files` that returned nothing — a moved repo
    root, a changed path argument — would print a clean run over no corpus at
    all.
    """
    monkeypatch.setattr(census, "production_files", lambda *a, **k: ["crates/x.rs"])
    assert guard.main() == 1
    assert "production Rust file" in capsys.readouterr().out


def test_a_type_population_that_collapsed_refuses_a_verdict(monkeypatch, capsys):
    monkeypatch.setattr(census, "writers", lambda files: {"Solo": {"a.rs", "b.rs"}})
    assert guard.main() == 1
    assert "`ResMut<T>` type" in capsys.readouterr().out


def test_a_new_multi_writer_type_is_reported(monkeypatch, capsys):
    real = census.writers

    def with_a_newcomer(files):
        found = real(files)
        found["APoisonedResource"] = {"crates/a.rs", "crates/b.rs"}
        return found

    monkeypatch.setattr(census, "writers", with_a_newcomer)
    assert guard.main() == 1
    out = capsys.readouterr().out
    assert "NEW multi-writer authority: APoisonedResource" in out
    # ⚠ The files are printed, because "which second file" is the first question
    # a reader has and re-running the census by hand is the cost this avoids.
    assert "crates/b.rs" in out


def test_a_type_that_gained_a_writer_is_reported(monkeypatch, capsys):
    real = census.writers
    subject = next(iter(guard.BASELINE))

    def with_one_more(files):
        found = real(files)
        found[subject] = set(found[subject]) | {"crates/a_new_writer.rs"}
        return found

    monkeypatch.setattr(census, "writers", with_one_more)
    assert guard.main() == 1
    assert "gained a writer" in capsys.readouterr().out


def test_a_type_that_lost_a_writer_is_reported(monkeypatch, capsys):
    """⛔ A DROP FAILS TOO, ON PURPOSE. A baseline nobody has to lower stops
    describing the tree, and then its silence means nothing."""
    real = census.writers
    subject = next(t for t, n in guard.BASELINE.items() if n > 2)

    def with_one_fewer(files):
        found = real(files)
        found[subject] = set(sorted(found[subject])[:-1])
        return found

    monkeypatch.setattr(census, "writers", with_one_fewer)
    assert guard.main() == 1
    assert "lost a writer" in capsys.readouterr().out


def test_a_type_that_left_the_population_must_be_removed(monkeypatch, capsys):
    # ⚠ AN UNADJUDICATED SUBJECT, DELIBERATELY. A verdict-carrying type takes a
    # different road — the phantom rule refuses it first, with a message that
    # says to remove the REASON and not just the baseline row — and that road is
    # `test_a_verdict_whose_duplication_was_repaired_is_refused` below. Picking
    # whichever 2-writer type came first in the baseline made this arm start
    # measuring that other rule the moment the baseline was re-derived.
    real = census.writers
    subject = next(
        t for t, n in guard.BASELINE.items() if n == 2 and t not in guard.ADJUDICATED
    )

    def with_one_writer(files):
        found = real(files)
        found[subject] = {sorted(found[subject])[0]}
        return found

    monkeypatch.setattr(census, "writers", with_one_writer)
    assert guard.main() == 1
    assert "no longer written from more than one file" in capsys.readouterr().out


def test_an_adjudication_of_a_type_that_is_not_multi_writer_is_refused(
    monkeypatch, capsys
):
    """⛔⛤ THE RULE MOVED INTO THE GUARD 2026-09-17, AND WHY IT HAD TO.

    `test_every_adjudication_cites_something` already asserts `name in BASELINE`
    — but `pytest scripts/tests` is NOT in `--maintenance`, so the lane that runs
    this guard could not see a verdict whose subject does not exist. That matters
    because the green line prints `len(multi) - len(ADJUDICATED)` as the unread
    debt, and a phantom verdict makes that subtraction understate the debt while
    reading as one more thing settled.
    """
    monkeypatch.setitem(guard.ADJUDICATED, "AResourceNobodyWrites", "a citation `X`")
    assert guard.main() == 1
    out = capsys.readouterr().out
    assert "AResourceNobodyWrites is adjudicated but has 0 production writer" in out


def test_a_verdict_whose_duplication_was_repaired_is_refused(monkeypatch, capsys):
    """⭐ THE OTHER HALF, AND IT IS THE LIKELIER ONE: somebody collapses the
    second writer and leaves the verdict describing a tree that no longer has the
    shape. The type is still written — just from one file — so this is distinct
    from the misspelling above."""
    real = census.writers
    subject = next(t for t in guard.ADJUDICATED if guard.BASELINE.get(t) == 2)

    def with_one_writer(files):
        found = real(files)
        found[subject] = {sorted(found[subject])[0]}
        return found

    monkeypatch.setattr(census, "writers", with_one_writer)
    assert guard.main() == 1
    out = capsys.readouterr().out
    assert f"{subject} is adjudicated but has 1 production writer file(s)" in out
    # ⚠ AND IT MUST BEAT THE `left` RULE TO THE VERDICT, because "remove it from
    # the baseline" alone would leave the stale reason behind.
    assert "no longer written from more than one file" not in out
