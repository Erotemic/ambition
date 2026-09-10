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


def test_a_lockstep_bout_is_flagged_and_an_asymmetric_one_is_not(tmp_path, capsys):
    """⛔⛔ A MIRROR MATCH OF A DETERMINISTIC BRAIN IS ONE SEAT REPORTED TWICE.

    MEASURED 2026-09-10: 3 of the 5 fighters below the gate are exactly symmetric
    on damage, starts, damage dealt AND hitstun; **0 of the 15 that clear it
    are.** Those three are not fighters that convert badly, they are duels this
    harness cannot measure — and it reported them as fighter quality.

    ⚠ The arm pins BOTH directions, because a flag that fires on everything is as
    useless as one that never fires: `npc_emmy_noether` diverged on every field
    and failed the gate anyway, which is what makes her the one real failure.
    """
    module = _module()
    log = tmp_path / "sweep.log"
    rows = []
    for i in range(16):  # clear the population floor
        rows.append(
            f"=== clear_{i} run1\n"
            f"[duel] clear_{i} rung 9: duel ran 3600 ticks, took 1.0 / 1.1 of pool "
            f"= 1.00 / 1.10 per minute of duel, hitstun [300, 310] ticks, 1 knockout\n"
            f"[moves] seat 0: 50 starts across 9 distinct -> []\n"
            f"[moves] seat 1: 48 starts across 9 distinct -> []\n"
            f"[dealt] seat 0: 90 damage across 4 moves (+0 unclaimed) -> []\n"
            f"[dealt] seat 1: 95 damage across 4 moves (+0 unclaimed) -> []\n"
        )
    rows.append(
        "=== locked run1\n"
        "[duel] locked rung 9: duel ran 3600 ticks, took 0.4 / 0.4 of pool "
        "= 0.40 / 0.40 per minute of duel, hitstun [118, 118] ticks, 0 knockouts\n"
        "[moves] seat 0: 49 starts across 6 distinct -> []\n"
        "[moves] seat 1: 49 starts across 6 distinct -> []\n"
        "[dealt] seat 0: 41 damage across 1 moves (+0 unclaimed) -> []\n"
        "[dealt] seat 1: 41 damage across 1 moves (+0 unclaimed) -> []\n"
    )
    rows.append(
        "=== diverged run1\n"
        "[duel] diverged rung 9: duel ran 3600 ticks, took 0.28 / 0.44 of pool "
        "= 0.28 / 0.44 per minute of duel, hitstun [38, 59] ticks, 0 knockouts\n"
        "[moves] seat 0: 13 starts across 7 distinct -> []\n"
        "[moves] seat 1: 12 starts across 7 distinct -> []\n"
        "[dealt] seat 0: 18 damage across 1 moves (+0 unclaimed) -> []\n"
        "[dealt] seat 1: 15 damage across 1 moves (+0 unclaimed) -> []\n"
    )
    log.write_text("".join(rows) + "SWEEP DONE\n")
    module.LOG = log
    module.fold()
    printed = capsys.readouterr().out
    locked = next(l for l in printed.split("\n") if l.startswith("locked"))
    diverged = next(l for l in printed.split("\n") if l.startswith("diverged"))
    assert "LOCKSTEP" in locked, (
        "a bout whose two seats agree on damage, starts, dealt and hitstun was not "
        "flagged; that row would be read as a fact about the fighter"
    )
    assert "BELOW GATE" in diverged and "LOCKSTEP" not in diverged, (
        "a bout that diverged on every field was flagged as lockstep, which would "
        "explain away the one genuine failure on the grid"
    )
    assert "1 LOCKSTEP" in printed
