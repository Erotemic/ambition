"""The C03 census guard must run in a lane, and must be able to FAIL.

⛔ A guard with no test beside it runs in no lane — that happened in this
repository on 2026-09-16 to a checker that was committed, correct, and executed
by nothing.
"""

from __future__ import annotations

import pathlib
import sys

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parents[1]))

import check_session_owner_census_matches_source as guard  # noqa: E402


def test_the_census_matches_source_today():
    assert guard.main() == 0


def test_the_marker_is_the_authority_the_guard_reads():
    """⚠ The prose can be reworded; the marker is what must not drift silently."""
    stated = guard.declared()
    assert set(stated) == {
        "SessionScopedResources",
        "SessionOwnedCheckpointState",
        "SessionMechanics",
    }, stated
    assert sum(stated.values()) >= 30, stated


def test_the_bundle_count_is_read_from_source_not_the_marker():
    """⛔ THE CONTROL, AND IT IS A DISAGREEMENT ARM ON PURPOSE.

    An equality assertion is satisfied by every reader that throws information
    away, the CONSTANT included: `bundle_members` returning a fixed 29 would
    satisfy `real == declared[...]` forever. MEASURED 2026-09-16 by collapsing it
    to exactly that — the arm below passed, and what caught the constant was the
    OTHER bundle disagreeing (`SessionOwnedCheckpointState` is 6).

    ⇒ So this arm now asserts the two bundles report DIFFERENT counts from the
    same reader. Two subjects through one function means a collapsing function
    must disagree with at least one of them.
    """
    scoped = guard.bundle_members(
        guard.BUNDLES["SessionScopedResources"], "SessionScopedResources"
    )
    checkpoint = guard.bundle_members(
        guard.BUNDLES["SessionOwnedCheckpointState"], "SessionOwnedCheckpointState"
    )
    assert scoped == guard.declared()["SessionScopedResources"]
    assert checkpoint == guard.declared()["SessionOwnedCheckpointState"]
    assert scoped != checkpoint, (
        f"both bundles read as {scoped}; a reader that returns the same number "
        "for two different structs is not reading them"
    )
    assert scoped > 3, "the ResMut field pattern stopped matching; the scan is broken"


def _live_bundle_count() -> int:
    """The bundle's size READ FROM SOURCE, for arms that need today's number.

    ⛔⛤ **THREE ARMS HARDCODED IT AND TWO OF THEM WENT RED THE DAY THE BUNDLE
    GAINED A MEMBER (2026-09-16, `GameplayElapsed`).** A test of a guard whose
    whole subject is *"a number restated away from its source goes stale"*
    restated the number away from its source. The arms are about the RULE, not
    about the value, so they ask the same reader the guard asks.
    """
    return len(
        guard.source_members(
            guard.BUNDLES["SessionScopedResources"], "SessionScopedResources"
        )
    )


def test_session_mechanics_is_counted_as_one_resource_not_six_fields():
    """⚠ THE MISTAKE THIS GUARD RECORDS. It is ONE resource with six fields, and

    counting fields reads the total as 41 instead of 36.
    """
    assert guard.declared()["SessionMechanics"] == 1


def test_a_stray_copy_of_the_bundle_count_is_found():
    """⛔ THE COPY NOBODY REMEMBERED. `architecture-census.md` held a third copy

    in a table cell and it was still 25 after the plan's two were corrected to
    29. A rule that checks one known copy cannot find that.
    """
    assert guard.stray_counts(_live_bundle_count()) == []
    found = guard.stray_counts(999)
    assert found, "no line states the bundle's count, so this arm witnesses nothing"


def test_the_total_across_all_three_groups_is_not_mistaken_for_the_bundles_own():
    """⚠ THE FALSE RED THIS RULE ALREADY PRODUCED ONCE.

    *"36 process/App resources are explicitly documented … (SessionScopedResources
    …)"* states the TOTAL and is correct. A false red is obeyed faster than a
    false green is questioned.
    """
    line = (
        "- **36** process/App resources are explicitly documented by source as "
        "session- or generation-owned (see SessionScopedResources for which four "
        "arrived);"
    )
    hits = [m for form in guard.COUNT_FORMS for m in form.finditer(line)]
    assert not hits, hits


def test_rule_3_accounts_for_every_raw_registration_occurrence():
    """⛔⛤ THE FLOOR THAT CAUGHT ITSELF, and it was a REASONED number failing.

    The first version asserted `len(registrations) >= 20` — a figure I picked,
    not measured — while reading only the monolith's `rollback_registration.rs`,
    which holds 5. It fired on a correct tree. The replacement is the TOOL'S OWN
    TOTAL: every raw `rollback_resource_clone_checksum::<` occurrence must be
    classified as either a call site or the trait's forwarding definition.

    ⚠ And the widened scan immediately found a second defect the floor could not
    have: the pattern accepted only an IDENTIFIER owner, so the four call sites
    spelling it as a string literal (`"test"`, `"ambition_demo_sanic"`) parsed as
    nothing — 17 of 23.
    """
    registrations, findings = guard.workspace_registrations()
    assert findings == [], findings
    # The checkpoint family is five of them; the rest belong to other domains.
    assert len(registrations) > 15, registrations


def test_rule_3_asks_about_every_registration_method_not_just_one():
    """⛔⛤ THE VERDICT IS A NEGATIVE, SO THE METHOD LIST IS ITS REAL SUBJECT.

    The first version scanned `rollback_resource_clone_checksum` alone and printed
    *"NO rollback registration"*. `RollbackRegistrar` declares TEN
    `rollback_resource_*` methods. The checkpoint family's answer happened not to
    change — all five use that one method — but `PendingLifecycleCommit`, one
    resource over, is documented as rollback-registered and does not appear under
    that name.

    ⇒ MEASURED by poison in the shipped file: swapping `AcceptedCheckpointRestore`
    from `clone_checksum` to `rollback_resource_canonical` keeps the guard GREEN,
    which is correct and is precisely what the narrow version got wrong.
    """
    methods = guard.registration_methods()
    assert len(methods) >= 10, methods
    assert "rollback_resource_clone_checksum" in methods
    assert "rollback_resource_canonical" in methods
    assert any(guard.MAP_METHOD in m for m in methods), methods


def test_rule_3_does_not_count_a_doc_comment_naming_a_registration():
    """⛔ REGION FIRST. `teardown.rs` explains a resource by naming its

    registration inside a doc comment, and that mention is a raw occurrence that
    is not a call. Recognising the prose is the rule backwards; the comment REGION
    is deleted before anything is classified.
    """
    body = (
        "/// its value is inside the state checksum because of\n"
        "/// `rollback_resource_canonical::<ProjectileSeqCounter>`), so its\n"
        "    registrar.rollback_resource_canonical::<Real>(OWNER, \"resource.real\");\n"
    )
    stripped = guard.code_only(body)
    assert stripped.count("rollback_resource_canonical::<") == 1, stripped


def test_rule_3_does_not_read_an_entity_map_registration_as_a_second_authority():
    """⚠ FOUR RESOURCES CARRY BOTH, and reading the second as a competing state

    registration produced four false reds the moment the method list widened.
    `resource.x` is the state; `map.resource.x` is entity remapping for the same
    value. Two registrations of different KINDS are not two authorities.
    """
    registrations, findings = guard.workspace_registrations()
    assert findings == [], findings
    for both in ("PossessionState", "EncounterRegistry", "ActiveConversation"):
        assert registrations.get(both) == f"resource.{_snake(both)}", (
            both,
            registrations.get(both),
        )


def _snake(name: str) -> str:
    out = []
    for i, ch in enumerate(name):
        if ch.isupper() and i:
            out.append("_")
        out.append(ch.lower())
    return "".join(out)


def test_rule_3_knows_which_checkpoint_members_are_rollback_state():
    """⭐ THE PARTITION IS 5 + 1 AND BOTH HALVES ARE MEASURED FROM SOURCE.

    Source's own doc block said *"Four of these are canonical ROLLBACK state"*.
    It was a count of the BULLETS, one of which pairs a registered value with an
    unregistered one, so a reader auditing the family got the mechanism's count
    from a prose grouping.
    """
    registrations, _ = guard.workspace_registrations()
    family = {
        "SessionCheckpointOperations": "resource.session_checkpoint_operations",
        "SessionCheckpointOutcomes": "resource.session_checkpoint_outcomes",
        "AcceptedCheckpointRestore": "resource.accepted_checkpoint_restore",
        "OutstandingCheckpointRequest": "resource.outstanding_checkpoint_request",
        "SessionStartupResume": "resource.session_startup_resume",
    }
    for member, key in family.items():
        assert registrations.get(member) == key, (member, registrations.get(member))
    assert "AbandonedCheckpointOperation" not in registrations, (
        "it is written from `Update`, which never rewinds, and carries a "
        "value-complete note so a rewound world DISCARDS it. If it is registered "
        "now, the doc block above `SessionOwnedCheckpointState` must be rewritten "
        "— not this assertion relaxed."
    )


def test_rule_3_can_fail_on_an_undeclared_unregistered_member():
    """⛔ THE POISON, and it must land in the SHIPPED text, not a fixture.

    MEASURED 2026-09-16 against the real files: deleting `SessionStartupResume`'s
    registration, moving the host-side declaration onto a registered member, and
    dropping one key from the doc block each turn the guard red, and all three
    restored by md5.

    This arm keeps the cheapest of the three runnable in a lane: it asks the
    partition function about a member the doc block does not mention at all.
    """
    text = guard.CHECKPOINT.read_text(encoding="utf-8")
    start = text.index("pub struct SessionOwnedCheckpointState")
    doc = text[max(0, start - 4000) : start]
    windows = guard._doc_windows(doc)
    declared = {
        line.strip()
        for line, window in windows
        if guard.HOST_SIDE_PHRASE in window and "AbandonedCheckpointOperation" in line
    }
    assert declared, (
        "no doc line names AbandonedCheckpointOperation beside "
        f"{guard.HOST_SIDE_PHRASE!r}; the exemption the guard honours is derived "
        "from source, so with no such line the guard is failing for a reason "
        "other than the one this arm means to witness"
    )
    assert not any(
        guard.HOST_SIDE_PHRASE in window and "SessionCheckpointOperations" in line
        for line, window in windows
    ), "a registered member is declared host-side; source contradicts itself"


def test_a_stray_count_is_found_through_a_closing_backtick():
    """⛔⛤ THE ONE CHARACTER THAT HID A WHOLE STALE SECTION.

    `architecture-census.md` §3 stated the old count with the bundle name in
    backticks — `` `SessionScopedResources` names **25** `` — while the same
    document's executive map had already been corrected to 36. The stray-count
    rule required the name and the verb to be ADJACENT, so it read past the
    closing backtick and certified a document that contradicted itself for a day.

    ⇒ A rule keyed on a phrase is keyed on its PUNCTUATION too.
    """
    backticked = "`SessionScopedResources` names **25** process resources"
    hits = [int(m.groups()[-1]) for f in guard.COUNT_FORMS for m in f.finditer(backticked)]
    assert 25 in hits, "the backticked form is invisible to every count pattern"
    # ⚠ THE CONTROL. The same sentence at the REAL count must not be a finding,
    # or the rule is just "this line mentions a number".
    real = "`SessionScopedResources` names **29** process resources"
    stray = [int(m.groups()[-1]) for f in guard.COUNT_FORMS for m in f.finditer(real)]
    assert stray == [29], stray


def test_rule_4_checks_the_member_list_not_only_its_length():
    """⛔⛤ A COUNT IS NOT A CHECK ON A LIST, and this document proved it.

    §3 carried a 25-name list beside a stale 25. Correcting the COUNT alone would
    have left a list four members short and passed every rule this guard had. And
    once the count is right, a RENAMED member keeps it at 29 forever.

    ⇒ POISONED in the shipped census, three ways, restored by md5: renaming one
    member (count unchanged) → red and names both sides; dropping one from the
    checkpoint list → red; un-fencing both lists so the pattern matches nothing →
    red on anti-vacuity rather than clean.
    """
    assert guard.member_lists() == []
    scoped = guard.source_members(
        guard.BUNDLES["SessionScopedResources"], "SessionScopedResources"
    )
    checkpoint = guard.source_members(
        guard.BUNDLES["SessionOwnedCheckpointState"], "SessionOwnedCheckpointState"
    )
    # ⚠ The census must carry the real names, not a count of them.
    census = guard.CENSUS.read_text(encoding="utf-8")
    for member in scoped + checkpoint:
        assert member in census, f"{member} is in source and not in the census"
    assert len(set(scoped)) == len(scoped), "a duplicated field would mask a gap"
    assert not set(scoped) & set(checkpoint), "the two bundles must be disjoint"


def test_the_stray_sweep_reaches_the_ledger_json_not_only_markdown():
    """⛔⛤ THE FOURTH RECORDER, AND THE GLOB COULD NOT SEE IT.

    `consolidation-ledger.json` held the stale bundle count in THREE fields —
    `current_truth`, `evidence[0].claim` and `representation` — on the same day
    the census prose did, invisible to a sweep whose glob was `*.md`. ⇒ Ask how
    many RECORDERS a fact has before choosing a rule's population. This one had
    four and was corrected in two.

    ⚠ `static_measurement_snapshot` in that file is deliberately NOT swept: it
    names its own `source_commit` and is a dated measurement, not a live claim.
    """
    assert "*.json" in guard.SWEPT_SUFFIXES
    ledger = guard.PLANNING / "consolidation/consolidation-ledger.json"
    assert ledger.exists(), ledger
    swept = {
        path
        for suffix in guard.SWEPT_SUFFIXES
        for path in guard.PLANNING.rglob(suffix)
    }
    assert ledger in swept, "the ledger is not in the swept population"
    assert guard.stray_counts(_live_bundle_count()) == []
