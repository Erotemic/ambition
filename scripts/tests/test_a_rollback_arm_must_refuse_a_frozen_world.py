"""The sync-test arm accounting, against corpora with known answers."""

from __future__ import annotations

import importlib.util
import pathlib
import sys

REPO = pathlib.Path(__file__).resolve().parents[2]


def _load():
    name = "a_rollback_arm_must_refuse_a_frozen_world"
    spec = importlib.util.spec_from_file_location(name, REPO / "scripts" / f"{name}.py")
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


CHECK = _load()


def test_the_population_is_found_by_the_fixture_call():
    arms = CHECK.sync_test_arms()
    assert arms, "no sync-test fixture found at all"
    # The one the original census could not see: it lives outside `crates/` and
    # `game/`, which is where that sweep looked.
    assert "examples/capability_demo/tests/rollback_round_trip.rs" in arms


def test_every_arm_is_checked_or_accounted_for():
    arms = CHECK.sync_test_arms()
    loose = sorted(
        rel
        for rel, health in arms.items()
        if not health and rel not in CHECK.ADJUDICATED and rel not in CHECK.NOT_AN_ARM
    )
    assert not loose, (
        "sync-test rollback arm(s) that neither read the session's health nor "
        f"carry a reading of what a frozen world breaks in them: {loose}"
    )


def test_no_adjudication_or_exemption_outlives_its_file():
    """⚠ A row for a file that no longer builds a sync-test session excuses nothing.

    Worse, it hides the next one: a reader seeing a long list assumes it was
    derived from the tree.
    """
    arms = set(CHECK.sync_test_arms())
    stale = sorted((set(CHECK.ADJUDICATED) | set(CHECK.NOT_AN_ARM)) - arms)
    assert not stale, f"rows naming files that no longer build one: {stale}"


def test_a_health_call_satisfies_the_accounting():
    assert CHECK.sync_test_arms(["scripts/tests/fixtures_do_not_exist.rs"]) == {}


def test_the_adjudications_all_name_a_mechanism():
    """⛔ A row whose reason is blank is a waiver wearing a decision's clothes."""
    for rel, reason in CHECK.ADJUDICATED.items():
        assert len(reason) > 20, f"{rel} carries no mechanism: {reason!r}"


def test_prose_naming_the_health_api_does_not_certify_an_arm():
    """⛔⛤ THE FALSE-GREEN HOLE THE GPT REVIEW OF 2026-09-16 NAMED.

    `HEALTH` is what CERTIFIES an arm as non-vacuous, and it ran against raw
    source — so a file whose only mention is a comment satisfied the guard while
    checking nothing. The sampled arms all held real calls, so this was a FUTURE
    hole rather than a present vacuity, which is the cheapest moment to close it.
    """
    module = CHECK
    prose = (
        "fn arm() {\n"
        "    // session_health should be checked here someday\n"
        '    let msg = "read rollback_health() before asserting";\n'
        "    /* outer /* session_health */ nested */\n"
        # ⛔⛤ THE SECOND REVIEW'S POISON, WHICH THE FIRST REPAIR'S REGEX LET
        # THROUGH: a raw string whose inner `"` ended the pattern's match, leaving
        # the call standing as apparent code. Every Rust literal form belongs
        # here, because the lesson was "do not extend the regex one form at a
        # time".
        '    let raw = r#"foo" rollback_health() "bar"#;\n'
        '    let bytes = br##"session_health"##;\n'
        "    let _ = with_sync_test_rollback_settings(4, 10);\n"
        "}\n"
    )
    assert module.HEALTH.search(prose), "the fixture must mention it at all"
    stripped = module.code_only(prose)
    assert not module.HEALTH.search(stripped), stripped
    # ⭐ AND THE POPULATION SURVIVES. A stripper that ate the file would take the
    # arm out of the census instead, which reads as "nothing to check".
    assert module.SYNC_TEST.search(stripped)
    assert prose.count("\n") == stripped.count("\n"), "line numbers must not move"


def test_a_real_health_call_still_reads_as_one():
    """The positive control: the strip must not be a way to fail everything."""
    module = CHECK
    real = "fn arm() { assert!(sim.rollback_health().is_ok()); }"
    assert module.HEALTH.search(module.code_only(real))
    other = "fn arm() { let h = session_health(world); }"
    assert module.HEALTH.search(module.code_only(other))


def test_the_bench_exemption_still_rests_on_the_mechanism_it_names():
    """⛔⛤ The reason was false and the conclusion was right, which is the worst pair.

    `hall_bench.rs` was exempted as *"it asserts nothing"* until 2026-09-18,
    while it asserts the active room at `hall_bench.rs:55`. The exemption
    survives on a different fact — `with_required_start_room` refuses to boot
    unless the room resolved, so `active_room` already equals the asserted value
    at tick 0 and a stopped clock satisfies it — and THAT is the fact worth
    pinning, because it is the one that could stop being true. If the bench
    stops requiring its room, or the option stops being fallible, the bench's
    one assertion may start falsifying a frozen world and the exemption needs
    re-reading rather than re-approving.
    """
    bench = (REPO / "game/ambition_app/examples/hall_bench.rs").read_text()
    assert "game/ambition_app/examples/hall_bench.rs" in CHECK.NOT_AN_ARM
    assert "with_required_start_room" in CHECK.code_only(bench)
    options = CHECK.code_only(
        (REPO / "crates/ambition_sim_harness/src/options.rs").read_text()
    )
    assert "start_room_must_resolve = true" in options
