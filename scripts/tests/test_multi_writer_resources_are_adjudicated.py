"""The multi-writer ratchet must match the tree it ships against, and be able to fail."""

from __future__ import annotations

import pathlib
import re
import sys
from unittest import mock

import pytest

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
    assert all(len(files) >= 2 for files in guard.BASELINE.values())
    # ⭐ AND THE RECORDED FILES ARE PATHS, NOT A COUNT. The table held bare ints
    # until 2026-09-18, which made a same-cardinality writer swap invisible; an
    # int here again would restore that blindness with every other arm green.
    assert all(
        isinstance(f, str) and f.endswith(".rs")
        for files in guard.BASELINE.values()
        for f in files
    )


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
    # ⚠ NOT "`ResMut<T>` type", which is what this read until 2026-09-17. The
    # census now counts `world.resource_mut::<T>()` too, so a message naming only
    # the parameter shape would send a reader to the wrong half of the regex.
    assert "mutably-reached resource type" in capsys.readouterr().out


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
    subject = next(t for t, files in guard.BASELINE.items() if len(files) > 2)

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
    # `test_a_verdict_whose_duplication_was_repaired_is_refused` below.
    #
    # ⛔⛤ AND THE SUBJECT IS SYNTHETIC BECAUSE THIS ARM USED TO BORROW A REAL
    # ONE, WHICH MADE IT DEPEND ON THE CENSUS BEING INCOMPLETE. It picked
    # `next(t for t, n in BASELINE.items() if n == 2 and t not in ADJUDICATED)`.
    # That is a live query against the adjudication backlog: every verdict
    # landed shrinks the candidate set, and on 2026-09-18 the last 2-writer
    # unadjudicated type was adjudicated and this arm died with `StopIteration`
    # — a guard's own test broken by the guard's subject matter being finished,
    # which is the one outcome the campaign is FOR. A synthetic subject cannot
    # be adjudicated away.
    real = census.writers
    subject = "ASyntheticTypeNoProductionFileWrites"
    # A two-file recorded set, matching the table's type. An int here would
    # have worked only by luck: the swap check never evaluates it for a type
    # that dropped out of `multi`.
    monkeypatch.setitem(guard.BASELINE, subject, ("a.rs", "b.rs"))

    def with_one_writer(files):
        found = real(files)
        # Present in the baseline at two writers, found at one: exactly the
        # "population moved" shape, with no dependence on a real backlog row.
        found[subject] = {"crates/ambition_geometry/src/lib.rs"}
        return found

    monkeypatch.setattr(census, "writers", with_one_writer)
    assert guard.main() == 1
    assert "no longer written from more than one file" in capsys.readouterr().out


def test_a_writer_file_swapped_at_constant_count_is_reported(monkeypatch, capsys):
    """⛔⛤ THE AXIS THE COUNT-ONLY BASELINE COULD NOT SEE, and the reason the
    table stores paths.

    `test_a_type_that_gained_a_writer_is_reported` and
    `test_a_type_that_lost_a_writer_is_reported` both move the COUNT
    (add-without-remove, remove-without-add). Neither can witness one writer
    file being REPLACED by another: the cardinality is identical, so a ratchet
    comparing `len()` against a recorded int stays green forever.

    DEMONSTRATED on the real tree before this was fixed (CalculexAmbition,
    2026-09-18): swapping one of `AmbitionGameSave`'s 18 writer files for a
    fabricated path left the guard printing its ordinary `ok` line and
    `UNADJUDICATED: 0`. It matters because every entry in `ADJUDICATED` argues
    from the SPECIFIC files it names, so a silent substitution rots the
    citations with nothing to flag it.
    """
    real = census.writers
    subject = next(t for t, files in guard.BASELINE.items() if len(files) > 2)

    def with_one_swapped(files):
        found = real(files)
        kept = set(found[subject])
        kept.discard(sorted(kept)[0])
        kept.add("crates/a_fabricated_writer_nobody_declared.rs")
        found[subject] = kept
        return found

    monkeypatch.setattr(census, "writers", with_one_swapped)
    assert guard.main() == 1
    out = capsys.readouterr().out
    assert "CHANGED WHICH ONES" in out, out
    # The message must NAME both sides: "something changed" sends the reader
    # back to re-derive what this run already knew.
    assert "LEFT    " in out and "ARRIVED " in out, out
    assert "crates/a_fabricated_writer_nobody_declared.rs" in out, out


def test_a_session_world_writer_swapped_at_constant_count_is_reported(
    monkeypatch, capsys
):
    """⛔ THE SAME GAP LIVED IN `SESSION_WORLD_BASELINE`, because it was the same
    shape: `dict[str, int]` compared by count. Fixed in the same commit, and
    pinned here so the two tables cannot drift apart again."""
    real = census.session_world_writers
    subject = next(
        t for t, files in guard.SESSION_WORLD_BASELINE.items() if len(files) > 2
    )

    def with_one_swapped(files):
        found = real(files)
        kept = set(found[subject])
        kept.discard(sorted(kept)[0])
        kept.add("crates/another_fabricated_writer.rs")
        found[subject] = kept
        return found

    monkeypatch.setattr(census, "session_world_writers", with_one_swapped)
    assert guard.main() == 1
    out = capsys.readouterr().out
    assert "CHANGED WHICH ONES" in out, out
    assert "crates/another_fabricated_writer.rs" in out, out


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
    subject = next(
        t for t in guard.ADJUDICATED if len(guard.BASELINE.get(t, ())) == 2
    )

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


def test_a_new_multi_writer_session_world_component_is_reported(monkeypatch, capsys):
    """⛔ THE SECOND POPULATION MUST RATCHET TOO, or adding it was decoration.

    It is kept separate from `BASELINE` on purpose: a resource's lifetime is the
    App's and a session world component's is the SESSION's, so "two writers"
    answers a different question and a session boundary reclaims the second.
    """
    real = census.session_world_writers

    def with_a_newcomer(files):
        found = real(files)
        found["APoisonedSessionComponent"] = {"crates/a.rs", "crates/b.rs"}
        return found

    monkeypatch.setattr(census, "session_world_writers", with_a_newcomer)
    assert guard.main() == 1
    out = capsys.readouterr().out
    assert "NEW multi-writer SESSION WORLD component: APoisonedSessionComponent" in out
    assert "crates/b.rs" in out


def test_a_session_world_component_that_gained_a_writer_is_reported(monkeypatch, capsys):
    real = census.session_world_writers
    subject = next(iter(guard.SESSION_WORLD_BASELINE))

    def with_one_more(files):
        found = real(files)
        found[subject] = set(found[subject]) | {"crates/a_new_writer.rs"}
        return found

    monkeypatch.setattr(census, "session_world_writers", with_one_more)
    assert guard.main() == 1
    assert f"{subject} moved:" in capsys.readouterr().out


def test_a_collapsed_session_world_scan_refuses_a_verdict(monkeypatch, capsys):
    """⛔⛔ THE FLOOR, AND THIS POPULATION NEEDS ONE MORE THAN THE OTHER DOES.
    Eight types carry the accessor; a regex that stopped matching would report
    "no multi-writer session components" and read as good news."""
    monkeypatch.setattr(census, "session_world_writers", lambda files: {"Solo": {"a.rs"}})
    assert guard.main() == 1
    assert "`SessionWorldMut<T>`" in capsys.readouterr().out


def test_a_sole_owner_claim_that_gained_a_second_system_is_refused(monkeypatch, capsys):
    """⭐⛤ THE RULE THAT TURNS A PARAGRAPH INTO A CHECK, AND WHY IT EXISTS.

    Eleven verdicts say *"exactly one production system writes this inside a
    session"*. That sentence is the whole argument for calling them CORRECT, and
    until 2026-09-18 nothing could notice a second system arriving — the census
    is FILE-granular, so a new writer in the same file changes no count at all.
    ⛔ And it was not hypothetical: of the thirteen types the green line counted
    as *"one other file"*, two (`ActiveCutscene`, `ProjectileSeqCounter`) already
    had more than one writing function there.
    """
    real = census.write_sites
    subject = next(iter(guard.SOLE_IN_SESSION_OWNER))

    def with_a_second_system(ty, files):
        found = real(ty, files)
        if ty == subject:
            for f in found:
                if f != guard.SESSION_SCOPE_RESET:
                    found[f] = [*found[f], "a_poisoned_second_writer"]
        return found

    monkeypatch.setattr(census, "write_sites", with_a_second_system)
    assert guard.main() == 1
    out = capsys.readouterr().out
    assert "a_poisoned_second_writer" in out
    assert subject in out


def test_a_sole_owner_claim_whose_named_system_was_renamed_is_refused(
    monkeypatch, capsys
):
    """⚠ THE OTHER DIRECTION, AND THE LIKELIER ONE. Renaming the owning system
    leaves a verdict naming a function that no longer exists — the same rot the
    `phantom` rule catches for TYPES, one level down."""
    real = census.write_sites
    subject = next(iter(guard.SOLE_IN_SESSION_OWNER))

    def with_a_renamed_owner(ty, files):
        found = real(ty, files)
        if ty == subject:
            for f in found:
                if f != guard.SESSION_SCOPE_RESET:
                    found[f] = ["the_owner_under_its_new_name"]
        return found

    monkeypatch.setattr(census, "write_sites", with_a_renamed_owner)
    assert guard.main() == 1
    assert "the_owner_under_its_new_name" in capsys.readouterr().out


def test_a_sole_owner_claim_that_lost_the_reset_road_is_refused(monkeypatch, capsys):
    """⛔ THE SECOND HALF OF THE VERDICT IS ALSO A CLAIM. *"The other writer is
    the session boundary"* stops being true if the reset drops the resource, and
    then two in-session files are two authorities with no verdict at all."""
    real = census.writers
    subject = next(iter(guard.SOLE_IN_SESSION_OWNER))

    def without_the_reset(files):
        found = real(files)
        found[subject] = {
            f for f in found[subject] if f != guard.SESSION_SCOPE_RESET
        } | {"crates/somewhere_else.rs"}
        return found

    monkeypatch.setattr(census, "writers", without_the_reset)
    assert guard.main() == 1
    assert "no longer includes `SessionScopedResources::reset`" in capsys.readouterr().out


def test_every_sole_owner_claim_carries_a_verdict():
    # ⚠ The two tables must agree in BOTH directions, or the check above can be
    # satisfied by a type nobody adjudicated.
    for ty in guard.SOLE_IN_SESSION_OWNER:
        assert ty in guard.ADJUDICATED, ty


#: Backticked snake_case tokens that appear in verdicts and are NOT declarations
#: in this tree. Every one needs a reason on the line, because the arm below
#: exists precisely so that an unresolvable name cannot be left behind.
NOT_TREE_NAMES: dict[str, str] = {
    "ambition_conversation": "a crate, not a system",
    "ambition_platformer2d_runtime": "a crate, not a system",
    "leafwing_input_manager": "an EXTERNAL crate, not a system — and the distinction the verdict rests on: the four writers in `virtual_device.rs` are its trait impls",
    "app_it": "the app's integration-test TARGET, not a function",
    "configure_sets": "bevy's `App::configure_sets`, declared outside this tree",
    "in_set": "bevy's `IntoScheduleConfigs::in_set`, same",
    "init_resource": "bevy's `App::init_resource`, declared outside this tree",
    "resource_mut": "bevy's `World::resource_mut`, same",
    "write_sites": "a python function in `multi_writer_resource_census.py`",
    "this_tick": "prose shorthand for `materialize_projectiles_for_this_tick`",
    "get_resource_mut": "bevy's `World::get_resource_mut`, same",
    "run_if": "bevy's `IntoScheduleConfigs::run_if`, same",
    # ⚠ ID LITERALS, not identifiers. Backticks are right for a literal and
    # wrong for this arm, so they are named here rather than un-quoted: the
    # allowlist is where a token says which kind of thing it is.
    "respawn_platform_": "the value of `RESPAWN_PLATFORM_PREFIX`, an id prefix",
    "respawn_platform_0": "an example id built from that prefix",
}

_DECLARED = re.compile(
    # A function, or a struct/enum FIELD declaration. A field counts because a
    # verdict legitimately names the one field a clear skips.
    r"\bfn\s+([a-z_][a-z0-9_]*)"
    r"|^\s*(?:pub(?:\([^)]*\))?\s+)?([a-z_][a-z0-9_]*)\s*:\s*[A-Za-z&<\[(]",
    re.MULTILINE,
)
#: `snake_case` inside backticks — the shape a verdict uses to name a system.
#: One underscore minimum, so `slots` and `sim` are not candidates.
_CITED = re.compile(r"`([a-z][a-z0-9_]*_[a-z0-9_]*)`")


def test_every_system_a_verdict_names_exists():
    # ⛔⛤ THIS ARM CAUGHT A FABRICATED SYSTEM NAME IN A VERDICT THE DAY IT WAS
    # WRITTEN. `ActiveConversation` cited
    # `stamp_conversation_end_when_the_box_closes` as one of the three roads that
    # end a conversation. No such identifier exists: it is the DOC SENTENCE above
    # `publish_the_narrative_end`, promoted to an identifier and then cited twice
    # — here and in `walking_into_a_loading_zone.rs` — as if it had been read.
    #
    # ⇒ A paraphrase in backticks is indistinguishable from a citation to every
    # reader, including the one who wrote it. The chain it described was real and
    # the verdict's conclusion survived; only the name was invented, which is the
    # failure mode that cannot be caught by re-reading the argument.
    # ⚠ NOT `production_files()`. A verdict's strongest citation is the TEST ARM
    # that witnesses it, and those live in exactly the files that predicate cuts.
    # Aimed at production only, this arm reported nine real arm names as
    # fabricated — a corpus narrower than the claim it checks.
    declared: set[str] = set()
    for path in census.rust_files(("crates", "game", "fixtures", "examples", "tools")):
        for match in _DECLARED.finditer(pathlib.Path(path).read_text(errors="replace")):
            declared.add(match.group(1) or match.group(2))
    for path in pathlib.Path(__file__).resolve().parents[1].rglob("*.py"):
        for match in re.finditer(r"^def\s+([a-z_][a-z0-9_]*)", path.read_text(errors="replace"), re.M):
            declared.add(match.group(1))

    unresolved: dict[str, list[str]] = {}
    # ⚠ BOTH TABLES. The session-world verdicts arrived after this arm and would
    # have been exempt from it by omission, which is the shape of exemption this
    # repository keeps paying for.
    verdicts = {**guard.ADJUDICATED, **guard.SESSION_WORLD_ADJUDICATED}
    for name, reason in verdicts.items():
        for match in _CITED.finditer(reason):
            token = match.group(1)
            if token in declared or token in NOT_TREE_NAMES:
                continue
            unresolved.setdefault(token, []).append(name)
    assert not unresolved, (
        "a verdict names something this tree does not declare — either the name "
        "is wrong or it belongs in NOT_TREE_NAMES with its reason: "
        f"{ {k: sorted(set(v)) for k, v in sorted(unresolved.items())} }"
    )


def test_the_name_check_can_fail(monkeypatch):
    # ⚠ The arm above reads a 1,300-file corpus, so "it passed" is the answer it
    # gives when its regex matches nothing at all. Poison the verdict, not the
    # corpus: a name shaped like a system and spelled like nothing in the tree.
    poisoned = dict(guard.ADJUDICATED)
    poisoned["ControlFrame"] = "CORRECT — see `a_system_that_was_never_written`."
    monkeypatch.setattr(guard, "ADJUDICATED", poisoned)
    with pytest.raises(AssertionError, match="a_system_that_was_never_written"):
        test_every_system_a_verdict_names_exists()


def test_a_declaration_the_corpus_hides_is_not_accepted(monkeypatch):
    # ⚠ And the corpus itself is a premise. If `production_files()` returned
    # nothing the arm would report every token as unresolved rather than pass, so
    # an empty scan is loud — the opposite failure from the one above.
    monkeypatch.setattr(census, "production_files", lambda *a, **k: [])
    monkeypatch.setattr(census, "rust_files", lambda *a, **k: [])
    with pytest.raises(AssertionError, match="does not declare"):
        test_every_system_a_verdict_names_exists()


def test_the_printed_shortlist_is_not_empty_by_accident():
    # ⚠ THE LINE THIS PINS IS A SUBTRACTION, so "nothing owed" and "the parse
    # broke" print the same reassuring thing. A turbofish regex that matched
    # nothing would report every registered type as unregistered and the
    # shortlist as CLEAR — the most comfortable possible failure.
    files = census.production_files()
    multi = {t: sorted(fs) for t, fs in census.writers(files).items() if len(fs) > 1}
    registered, owed = guard.rollback_registered_shortlist(multi)
    assert registered > 20, (
        "the rollback-registration parse found almost nothing, so the printed "
        f"shortlist is a fact about the regex: {registered} intersecting types"
    )
    assert set(owed).isdisjoint(guard.ADJUDICATED), owed
    assert set(owed) <= set(multi), owed


def test_every_session_world_adjudication_cites_something():
    # Same rule as the resource table: a verdict is a CITATION, not an opinion.
    for name, reason in guard.SESSION_WORLD_ADJUDICATED.items():
        assert name in guard.SESSION_WORLD_BASELINE, name
        assert len(reason) > 60, name
        assert "`" in reason, f"{name}: no system or field cited"


def test_a_name_collision_claim_that_stopped_being_local_is_refused(tmp_path, monkeypatch, capsys):
    """⛔ A VERDICT THAT SAYS "THESE ARE NOT THE SAME TYPE" IS A CLAIM ABOUT
    SCOPE, and scope is exactly what a later refactor changes.

    `FixedStepsTaken` is declared INSIDE a function in two files, so the census's
    name-keyed row is a phantom. Promote either declaration to module scope and
    the two writers might really share a type — at which point the row is a real
    question and the verdict must not still be saying it is not.
    """
    module = tmp_path / "moved.rs"
    module.write_text("struct FixedStepsTaken(u32);\n")
    monkeypatch.setattr(
        guard,
        "NAME_COLLISION_NOT_ONE_TYPE",
        {"FixedStepsTaken": (str(module.relative_to(tmp_path)),)},
    )
    monkeypatch.chdir(tmp_path)
    problems = guard.name_collision_shortfalls(
        {"FixedStepsTaken": [str(module.relative_to(tmp_path))]},
        [str(module.relative_to(tmp_path))],
    )
    # The file exists relative to the REPO, not to tmp_path, so this arm asserts
    # the refusal a MISSING declaration produces — the per-tabled-entry check is
    # anchored to THIS SCRIPT's own location, not to `files` or the caller's
    # cwd, precisely so a claim about the repository stays one no matter what
    # corpus the unlisted-collision scan below it is given.
    assert problems, "a claim whose declaration cannot be found must be refused"
    assert "no declaration" in problems[0]


def test_the_name_collision_claim_holds_on_the_real_tree():
    """⭐ AND THE CONTROL, because the arm above would pass if the checker simply
    always refused. Run it against the tree as it is."""
    files = census.production_files()
    multi = {t: sorted(fs) for t, fs in census.writers(files).items() if len(fs) > 1}
    assert guard.name_collision_shortfalls(multi, files) == []


def test_an_unlisted_name_collision_is_refused():
    """⭐ THE OTHER DIRECTION: a multi-writer type NOT in the table whose name
    still matches more than one struct/enum declaration in the tree.

    Warmup's collision used to live in ADJUDICATED prose alone, with ZERO
    automated re-check — a fourth `struct Warmup`, or the three declarations
    collapsing into a shared type without the writer-file set changing, would
    have passed everything this file checked before this row existed. The
    positive control here is what makes the empty result on the real tree mean
    something: emptying the table must recover EXACTLY the two known cases and
    nothing else, not merely produce SOME problems.
    """
    files = census.production_files()
    multi = {t: sorted(fs) for t, fs in census.writers(files).items() if len(fs) > 1}
    with mock.patch.object(guard, "NAME_COLLISION_NOT_ONE_TYPE", {}):
        problems = guard.name_collision_shortfalls(multi, files)
    flagged = sorted(p.split()[0] for p in problems if "matches" in p)
    assert flagged == ["FixedStepsTaken", "Warmup"], flagged


def test_a_vacuous_declaration_scan_is_refused_not_silently_clean():
    """⛔ ANTI-VACUITY. A regex that stopped matching declarations would find
    zero types with 2+ declarations and report a clean tree — the same failure
    `MIN_FILES`/`MIN_TYPES` exist to catch on the file/type side of this
    module, now covered on the declaration-count side too."""
    files = census.production_files()
    multi = {t: sorted(fs) for t, fs in census.writers(files).items() if len(fs) > 1}
    with mock.patch.object(guard, "MIN_DECLARATIONS", 10**9):
        problems = guard.name_collision_shortfalls(multi, files)
    assert problems and "declaration" in problems[0], problems


def test_a_session_world_verdict_whose_duplication_was_repaired_is_refused(
    monkeypatch, capsys
):
    # ⛔ The debt line for this population is a subtraction too. A verdict left
    # behind after its subject became single-writer reads as one more thing
    # settled while shrinking the unread half.
    monkeypatch.setattr(
        census,
        "session_world_writers",
        lambda files: {
            **{t: {"a.rs", "b.rs"} for t in guard.SESSION_WORLD_BASELINE},
            "LdtkRuntimeIndex": {"only.rs"},
            "Spare": {"x.rs"},
            "Spare2": {"y.rs"},
            "Spare3": {"z.rs"},
        },
    )
    assert guard.main() == 1
    out = capsys.readouterr().out
    assert "LdtkRuntimeIndex" in out
    assert "a repair is not an amnesty" in out
