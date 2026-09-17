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
    real = census.writers
    subject = next(t for t, n in guard.BASELINE.items() if n == 2)

    def with_one_writer(files):
        found = real(files)
        found[subject] = {sorted(found[subject])[0]}
        return found

    monkeypatch.setattr(census, "writers", with_one_writer)
    assert guard.main() == 1
    assert "no longer written from more than one file" in capsys.readouterr().out
