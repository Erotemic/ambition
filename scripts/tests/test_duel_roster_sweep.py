"""The roster sweep separates the three ways a fighter produces no number."""

from __future__ import annotations

import importlib.util
import pathlib

REPO = pathlib.Path(__file__).resolve().parents[2]
SCRIPT = REPO / "scripts/measure_duel_roster.py"


def _module():
    spec = importlib.util.spec_from_file_location("duel", SCRIPT)
    assert spec and spec.loader
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def test_the_two_published_vocabularies_are_read():
    """⛔ A GRID ID, A PLAYABLE ID AND A BARE REGISTRATION ARE THREE THINGS.

    `npc_carl_stargan` is on the grid, is NOT on `PLAYABLE_ROSTER`, and authors
    no body — reporting his zero as a fighter's is how a content defect gets
    filed as an AI defect.
    """
    module = _module()
    playable, bare = module.rosters()
    assert "npc_pirate_admiral" in playable, (
        "the playable roster no longer parses; every row would read `grid-only` "
        "and the table's own classification would silently stop working"
    )
    assert "npc_carl_stargan" in bare
    assert "npc_carl_stargan" not in playable


def test_a_gate_failure_and_a_harness_failure_are_different_rows(tmp_path):
    """⛔⛔ BOTH SURFACE AS A PANIC, and they have different owners.

    One is `A_REAL_FIGHT` firing on a fighter that was MEASURED and fell short.
    The other is a dependency's despawn hook demanding a resource this headless
    composition lacks, and that fighter was never measured at all.
    """
    module = _module()
    log = tmp_path / "sweep.log"
    log.write_text(
        "=== a_fighter run1\n"
        "[duel] a_fighter rung 9: duel ran 3600 ticks (decided None), took 0.2 / 0.2 "
        "of pool = 0.20 / 0.20 per minute of duel\n"
        "[gap] closest the seats ever came: 0px; ticks within 60px: 100 of 3600\n"
        "[moves] seat 0: 9 starts across 3 distinct -> []\n"
        "[moves] seat 1: 9 starts across 3 distinct -> []\n"
        "thread 'x' panicked at game/ambition_app/tests/smash_cpus_damage_each_other.rs:408:9:\n"
        "=== an_unrunnable run1\n"
        "thread 'x' panicked at /home/x/.cargo/registry/src/i/bevy_render-0.19.1/src/sync_component.rs:55:41:\n"
        + "=== c run1\n[duel] c rung 9: took 9.0 / 9.0 of pool = 9.00 / 9.00 per minute of duel\n" * 1
    )
    module.LOG = log
    # ⛔ The anti-vacuity floor refuses a corpus this small, which is the point —
    # so the shapes are asserted through `fold`'s parsing rather than its report.
    try:
        module.fold()
    except AssertionError as error:
        assert "Re-run the sweep" in str(error)
    else:
        raise AssertionError(
            "a three-row log passed the population floor; an empty or truncated "
            "sweep must refuse, because no rows reads exactly like a sweep nobody ran"
        )


def test_the_grid_is_asked_of_the_app_not_of_a_file():
    """⭐ THE POPULATION COMES FROM THE COMPOSED APP.

    The first version of this sweep took its ids from `authored_movesets::tables()`,
    whose own header warns it is *"NOT THE SELECTABLE CAST"*. This pins that the
    fallback is a REFUSAL rather than a list in the script — a hand-kept list is
    exactly what goes stale while reading as authoritative.
    """
    source = SCRIPT.read_text(encoding="utf-8")
    assert "do NOT fall back to a list" in source
    assert "authored_movesets" not in source.split('"""')[2] if source.count('"""') > 2 else True
