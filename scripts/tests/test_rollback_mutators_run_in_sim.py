"""The rollback-mutator guard, and the three lies its cheap version told.

S35 recorded that a naive version of this check returns 87 rows of which ~80 are
artifacts. Each of those three causes is planted here as its own test, because a
guard that is re-derived later will be re-derived the cheap way unless the
expensive lesson is executable.
"""

from __future__ import annotations

import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO / "scripts"))

import check_rollback_mutators_run_in_sim as guard  # noqa: E402

ROLLBACK_REGISTRY_REL = "crates/ambition_platformer2d_runtime/src/rollback/mod.rs"


def _tree(tmp_path: Path, files: dict[str, str]) -> Path:
    registry = tmp_path / ROLLBACK_REGISTRY_REL
    registry.parent.mkdir(parents=True, exist_ok=True)
    registry.write_text(
        'fn r(app: &mut App) { app.rollback_component_canonical::<bc::BodyMana>('
        'ENGINE, "body.mana"); }',
        encoding="utf-8",
    )
    for rel, body in files.items():
        p = tmp_path / rel
        p.parent.mkdir(parents=True, exist_ok=True)
        p.write_text(body, encoding="utf-8")
    return tmp_path


def test_the_registry_is_the_source_of_truth_for_what_is_rollback_state(tmp_path):
    root = _tree(tmp_path, {})
    assert guard.rollback_types(root) == {"BodyMana"}


def test_a_rollback_mutator_registered_into_Update_is_found(tmp_path):
    root = _tree(tmp_path, {
        "crates/ambition_x/src/lib.rs": "\n".join([
            "pub fn regen(mut m: Query<&mut BodyMana>) {}",
            "fn build(app: &mut App) { app.add_systems(Update, regen); }",
        ]),
    })
    found = guard.collect(root)
    assert len(found) == 1, f"the planted mutator was not found: {found}"
    name, _file, schedule, hits = found[0]
    assert (name, schedule, hits) == ("regen", "Update", ["BodyMana"])


def test_a_qualified_rollback_type_is_still_recognised(tmp_path):
    """The fast extractor must preserve the old regex's qualified-path case."""
    root = _tree(tmp_path, {
        "crates/ambition_x/src/lib.rs": "\n".join([
            "pub fn regen(mut m: ResMut<bc::BodyMana>) {}",
            "fn build(app: &mut App) { app.add_systems(Update, regen); }",
        ]),
    })
    found = guard.collect(root)
    assert len(found) == 1
    assert found[0][3] == ["BodyMana"]


def test_going_through_sim_schedule_is_the_point_and_is_not_flagged(tmp_path):
    root = _tree(tmp_path, {
        "crates/ambition_x/src/lib.rs": "\n".join([
            "pub fn regen(mut m: Query<&mut BodyMana>) {}",
            "fn build(app: &mut App) {",
            "    let sim = app.sim_schedule();",
            "    app.add_systems(sim, regen);",
            "}",
        ]),
    })
    assert guard.collect(root) == [], (
        "`sim_schedule()` is exactly the thing this guard asks for — flagging it "
        "would make the fix fail the check"
    )


def test_STARTUP_is_initialisation_not_a_defect(tmp_path):
    """⚠ my own category error, caught by reading the first output.

    `Startup` runs once, before any rewind exists, so writing rollback state
    there is what the first snapshot is taken OF. Including it reported
    `setup_simulation_system` seeding `MovingPlatformSet`, which is correct.
    """
    root = _tree(tmp_path, {
        "crates/ambition_x/src/lib.rs": "\n".join([
            "pub fn seed(mut m: ResMut<BodyMana>) {}",
            "fn build(app: &mut App) { app.add_systems(Startup, seed); }",
        ]),
    })
    assert guard.collect(root) == []


def test_a_system_named_update_does_not_collide_with_app_dot_update(tmp_path):
    """⛔ artifact 1: ~80 of the cheap version's 87 rows.

    Paren balance kills it by construction — `app.update();` is not inside the
    parentheses of an `add_systems(` call — so this pins the property rather
    than a workaround.
    """
    root = _tree(tmp_path, {
        "crates/ambition_x/src/lib.rs": "\n".join([
            "pub fn update(mut m: ResMut<BodyMana>) {}",
            "fn build(app: &mut App) {",
            "    let sim = app.sim_schedule();",
            "    app.add_systems(sim, update);",
            "}",
            "fn helper(app: &mut App) { app.add_systems(Update, unrelated); app.update(); }",
        ]),
    })
    assert guard.collect(root) == [], (
        "`app.update()` after an unrelated `add_systems(Update, ...)` was read "
        "as registering the system named `update`"
    )


def test_a_later_add_systems_call_is_not_attributed_to_an_earlier_schedule(tmp_path):
    """⛔ artifact 2: a fixed-size window runs past the end of one call."""
    root = _tree(tmp_path, {
        "crates/ambition_x/src/lib.rs": "\n".join([
            "pub fn regen(mut m: ResMut<BodyMana>) {}",
            "fn build(app: &mut App) {",
            "    app.add_systems(Update, something_else);",
            "    let sim = app.sim_schedule();",
            "    app.add_systems(sim, regen);",
            "}",
        ]),
    })
    assert guard.collect(root) == [], (
        "`regen` belongs to the sim registration that follows, not to the "
        "`Update` one that precedes it"
    )


def test_an_inline_cfg_test_module_is_not_production(tmp_path):
    """⛔ artifact 3: `#[cfg(test)] mod` INSIDE a production file.

    A test app has no GGRS schedule to reach, so registering a rollback mutator
    into `Update` there is correct — and looks exactly like the defect. Path
    filtering never sees it because the file is production.
    """
    root = _tree(tmp_path, {
        "crates/ambition_x/src/vortex.rs": "\n".join([
            "pub fn fire(mut m: ResMut<BodyMana>) {}",
            "#[cfg(test)]",
            "mod tests {",
            "    fn test_app() -> App {",
            "        let mut app = App::new();",
            "        app.add_systems(Update, fire);",
            "        app",
            "    }",
            "}",
        ]),
    })
    assert guard.collect(root) == []


def test_an_ordering_edge_is_not_a_registration(tmp_path):
    root = _tree(tmp_path, {
        "crates/ambition_x/src/lib.rs": "\n".join([
            "pub fn regen(mut m: ResMut<BodyMana>) {}",
            "fn build(app: &mut App) { app.add_systems(Update, hud.after(regen)); }",
        ]),
    })
    assert guard.collect(root) == [], (
        "`.after(regen)` names a system somebody ELSE registered — the sibling "
        "guard learned this the expensive way and its stripper is imported"
    )


def test_the_real_tree_is_clean_or_waived():
    assert guard.collect() == [], (
        "rollback state is mutated outside the rewinding schedule: "
        f"{[f[0] for f in guard.collect()]}"
    )


def test_every_waiver_cites_the_code_that_makes_it_true():
    """A waiver here claims a value may drift across a rewind. That is a strong
    claim, so it has to point at something checkable rather than assert itself."""
    thin = [
        name
        for name, why in guard.WAIVERS.items()
        if "⛔" not in why or len(why) < 150
    ]
    assert not thin, f"these waivers assert rather than cite: {thin}"

# ── the live tree ─────────────────────────────────────────────────────────
#
# ⛔⛔ EVERY TEST ABOVE RUNS ON A `tmp_path` FIXTURE. They pin the scanner's
# behaviour — which schedules it recognises, which it forgives — and none of
# them asks the question the check exists to answer: does THIS repository
# register a rollback mutator into a schedule that does not rewind? Audited
# 2026-09-02 across all 16 `check_*.py`: this was the one guard whose live-tree
# answer nothing asserted.
#
# ⭐ ADDING IT EXPOSED THE REAL DEFECT. `rollback_types()` read ONE file, so the
# guard was green over ONE type of the workspace's 113 — under 1% of the surface
# its own docstring called its source of truth. Widened to scan the same
# production sources the mutator scan uses: 113 types, 318 mutating systems,
# six offenders, all six triaged into `WAIVERS` with reasons.
#
# ⚠ A desync from a mutator in a non-rewinding schedule is silent — the run
# diverges, nothing throws. A scanner provably correct on fixtures and never
# pointed at the tree is not a guard against it.


def test_the_real_tree_registers_no_unwaived_rollback_mutator_outside_the_rewind():
    """⚠ Against the REAL crates, not a fixture — `collect()` defaults to REPO.

    Green means zero UNWAIVED. Six systems are waived with reasons; a seventh
    turns this red, which is the property the widening bought.
    """
    offenders = guard.collect()
    assert not offenders, (
        "a system that mutates rollback state is registered into a schedule that "
        "does not rewind, so its writes survive a rollback and desync the run: "
        f"{[(row[0], row[3]) for row in offenders]}. Register it through "
        "`app.sim_schedule()`, or waive it in WAIVERS with the reason its drift "
        "across a rewind does not matter."
    )


def test_the_scan_covers_the_whole_registration_surface_not_one_file():
    """⛔⛔ THE REGRESSION THAT MADE THIS GUARD ORNAMENTAL, PINNED BY ITS SIZE.

    Registration is DISTRIBUTED — each crate owns a `rollback_registration.rs` —
    so a `rollback_types()` that reads any single file sees a snapshot of one
    crate. It read exactly one and found one type; the workspace has 113 across
    ten files. The number below is a floor, not the measurement: it exists to
    fail if the scan narrows again, and 20 is far under today's 113 so ordinary
    churn cannot trip it.
    """
    types = guard.rollback_types()
    assert len(types) > 20, (
        f"the canonical rollback scan sees only {len(types)} type(s). It once "
        "read a single file and saw 1 of 113, which made every green above "
        "meaningless — check that `rollback_types` still scans all production "
        "sources rather than one registry"
    )


def test_no_waiver_names_a_system_that_no_longer_mutates_rollback_state():
    """⛔ A WAIVER THAT OUTLIVES ITS SYSTEM IS A HOLE NOBODY IS WATCHING.

    Each entry excuses a named system from the guard. If the system is gone or
    no longer touches rollback state, the entry silently pre-authorises the next
    thing to take that name.
    """
    mutators = set(guard.mutating_systems())
    stale = sorted(name for name in guard.WAIVERS if name not in mutators)
    assert not stale, (
        f"{stale} are waived but no longer seen mutating rollback state — remove "
        "them, so the waiver cannot cover a future system that reuses the name"
    )


def test_the_live_scan_reasoned_about_something_at_all():
    """⭐ POSITIVE CONTROL. `collect()` returning nothing is the pass condition —
    and it also returns nothing if the registry moved, the parse broke, or the
    source glob went empty."""
    assert guard.rollback_types(), (
        "the canonical rollback scan named NO types at all; the parse broke, and "
        "the offender assertion above is certifying an empty scan"
    )
    assert guard.mutating_systems(), (
        "no system anywhere was seen mutating rollback state; the source scan is "
        "empty and the guard above cannot fail"
    )
