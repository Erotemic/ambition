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


def test_the_real_tree_has_no_mutator_outside_the_rewind_that_is_not_banked():
    """⛔ NOT "clean" — twelve offenders are real, owed and NOT waived.

    A `WAIVERS` entry says the drift does not matter; an `ACKNOWLEDGED` entry
    says it does and names the row it is owed to. What this refuses is a
    THIRTEENTH, because while the guard could not go green one more could not
    change its verdict.
    """
    new_offenders = [f[0] for f in guard.collect() if f[0] not in guard.ACKNOWLEDGED]
    assert new_offenders == [], (
        "rollback state is mutated outside the rewinding schedule by a system "
        f"that is neither waived nor acknowledged: {new_offenders}"
    )


def test_every_banked_offender_is_still_one():
    """⛔⛤ THE LIST HAS TO BE EXACT OR IT BECOMES A SECOND WAIVER TABLE.

    A banked name the scan no longer reports was either fixed — delete it — or
    lost to a blind spot, and a stale entry silently absorbs the next system to
    take its place.
    """
    reported = {f[0] for f in guard.collect()}
    stale = sorted(set(guard.ACKNOWLEDGED) - reported)
    assert stale == [], (
        f"ACKNOWLEDGED names systems the scan no longer reports: {stale}. "
        "Check WHICH of the two happened before deleting the entry."
    )


def test_the_two_tables_never_name_the_same_system():
    """They make OPPOSITE claims, so an overlap is one of them being false."""
    both = sorted(set(guard.ACKNOWLEDGED) & set(guard.WAIVERS))
    assert both == [], (
        f"{both} are both waived (the drift does not matter) and acknowledged "
        "(the drift is real and owed) — those cannot both be true"
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

    Green means no offender that is neither WAIVED nor ACKNOWLEDGED. The twelve
    banked ones are real and owed; a THIRTEENTH turns this red, which is the
    property that was lost while the guard could not go green.
    """
    offenders = [row for row in guard.collect() if row[0] not in guard.ACKNOWLEDGED]
    assert not offenders, (
        "a system that mutates rollback state is registered into a schedule that "
        "does not rewind, so its writes survive a rollback and desync the run: "
        f"{[(row[0], row[3]) for row in offenders]}. Register it through "
        "`app.sim_schedule()`, or waive it in WAIVERS with the reason its drift "
        "across a rewind does not matter \u2014 or, if it is real and owed to an "
        "open row, bank it in ACKNOWLEDGED, which claims the opposite."
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
    for name, (relative, _why) in sorted(guard.BLIND_SPOTS.items()):
        source = guard.REPO / relative
        assert source.exists() and f"fn {name}" in source.read_text(errors="replace"), (
            f"{name} is recorded as a scanner blind spot but is gone from "
            f"{relative}; drop it from guard.BLIND_SPOTS and from WAIVERS"
        )
    assert stale == sorted(guard.BLIND_SPOTS), (
        f"{stale} are waived but no longer seen mutating rollback state — remove "
        "them, so the waiver cannot cover a future system that reuses the name. "
        "If the scanner is what stopped seeing it, say so in "
        "guard.BLIND_SPOTS rather than deleting a live waiver"
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


# ── `#[derive(SystemParam)]` resolution (added 2026-09-16) ──────────────────
#
# The fourth spelling of a mutable write, and the one that hid the most: a
# bundle is ONE identifier in a signature and can hold any number of `ResMut`
# fields. MEASURED when the hole was found: `SessionScopedResources` holds 25,
# of which 13 are rollback-registered — `MovingPlatformSet` among them, the type
# this guard's own docstring records as its entire population before 2026-09-02.


def test_a_bundle_field_is_a_mutable_write(tmp_path, monkeypatch) -> None:
    """A system naming a bundle mutates what the BUNDLE borrows mutably."""
    crate = tmp_path / "crates" / "demo" / "src"
    crate.mkdir(parents=True)
    (crate / "lib.rs").write_text(
        """
        registrar.rollback_resource_canonical::<MovingPlatformSet>(OWNER, "x");

        #[derive(SystemParam)]
        pub struct SessionScoped<'w> {
            platforms: ResMut<'w, MovingPlatformSet>,
        }

        pub fn reset_on_activation(scoped: SessionScoped) {}

        app.add_systems(Update, (reset_on_activation,));
        """
    )
    monkeypatch.setattr(guard, "REPO", tmp_path)
    for cached in (guard.rollback_types, guard.mutating_systems, guard.system_param_mutables,
                   guard._production_sources):
        cached.cache_clear()

    assert guard.system_param_mutables(tmp_path)["SessionScoped"] == frozenset({"MovingPlatformSet"})
    assert guard.mutating_systems(tmp_path)["reset_on_activation"] == ["MovingPlatformSet"]


def test_the_lifetime_in_a_bundle_field_does_not_hide_it(tmp_path) -> None:
    """⛔ A SIGNATURE ELIDES THE LIFETIME AND A STRUCT FIELD CANNOT.
    `ResMut<T>` matched everywhere and `ResMut<'w, T>` matched nowhere, so the
    pattern was perfect on functions and blind on exactly the bodies where the
    13 registered types were."""
    assert guard._MUTABLE_PARAM_TYPE.findall("ResMut<'w, MovingPlatformSet>") == ["MovingPlatformSet"]
    assert guard._MUTABLE_PARAM_TYPE.findall("ResMut<MovingPlatformSet>") == ["MovingPlatformSet"]
    assert guard._MUTABLE_PARAM_TYPE.findall("SessionWorldMut<'w, world::RoomSet>") == ["RoomSet"]


def test_a_bundle_inside_a_bundle_still_reaches_the_write(tmp_path, monkeypatch) -> None:
    """Bundles nest, so resolution is transitive or it is another silent floor."""
    crate = tmp_path / "crates" / "demo" / "src"
    crate.mkdir(parents=True)
    (crate / "lib.rs").write_text(
        """
        registrar.rollback_resource_canonical::<BaseGravity>(OWNER, "x");

        #[derive(SystemParam)]
        pub struct Inner<'w> { gravity: ResMut<'w, BaseGravity> }

        #[derive(SystemParam)]
        pub struct Outer<'w, 's> { inner: Inner<'w, 's> }

        pub fn outer_writer(bundle: Outer) {}
        """
    )
    monkeypatch.setattr(guard, "REPO", tmp_path)
    for cached in (guard.rollback_types, guard.mutating_systems, guard.system_param_mutables,
                   guard._production_sources):
        cached.cache_clear()
    assert guard.mutating_systems(tmp_path)["outer_writer"] == ["BaseGravity"]


def test_a_shrinking_population_is_a_failure_not_a_clean_report() -> None:
    """⛔ EVERY HOLE THIS GUARD HAS HAD REPORTED FEWER OFFENDERS, NOT MORE.

    `&mut T` alone saw 1 type of 113; `SessionWorldMut<T>` hid six; a
    `SystemParam` bundle hid thirteen. Each made the report shorter and cleaner.
    So the population size is part of the verdict: a count that FALLS below the
    floor must fail, because the next hole cannot announce itself any other way.
    """
    sizes = guard.population_sizes()
    assert not guard.population_shortfalls(), (
        f"the scan lost reach: {sizes} against {guard.POPULATION_FLOOR}"
    )
    # and the floor must actually be capable of failing
    original = dict(guard.POPULATION_FLOOR)
    try:
        guard.POPULATION_FLOOR["rollback types"] = sizes["rollback types"] + 1
        assert guard.population_shortfalls(), "the floor cannot fail, so it guards nothing"
    finally:
        guard.POPULATION_FLOOR.clear()
        guard.POPULATION_FLOOR.update(original)


def test_a_whole_file_compiled_out_is_not_production(tmp_path, monkeypatch) -> None:
    """⛔ THE NAME CONVENTIONS ARE A PROXY; `#![cfg(test)]` IS THE FACT.

    `features/ecs/fighter_harness.rs` is a test harness beside the code it
    exercises: declared `mod fighter_harness;` with no `#[cfg(test)]` on the
    DECLARATION and the attribute inside the file instead. Every name rule
    passed it, so its `Update` registrations sat in the production corpus.
    MEASURED: 4 files carry that attribute and all four were missed, so the
    proxy had no overlap with the fact at all.
    """
    crate = tmp_path / "crates" / "demo" / "src"
    crate.mkdir(parents=True)
    (crate / "harness.rs").write_text(
        """
        #![cfg(test)]
        registrar.rollback_resource_canonical::<MovingPlatformSet>(OWNER, "x");
        pub fn fixture_writer(platforms: ResMut<MovingPlatformSet>) {}
        app.add_systems(Update, (fixture_writer,));
        """
    )
    (crate / "real.rs").write_text(
        """
        registrar.rollback_resource_canonical::<BaseGravity>(OWNER, "y");
        pub fn real_writer(gravity: ResMut<BaseGravity>) {}
        app.add_systems(Update, (real_writer,));
        """
    )
    monkeypatch.setattr(guard, "REPO", tmp_path)
    for cached in (guard.rollback_types, guard.mutating_systems, guard.system_param_mutables,
                   guard._production_sources):
        cached.cache_clear()

    names = {name for name, _file, _sched, _hits in guard.collect(tmp_path)}
    assert "real_writer" in names, "a production Update mutator must still be found"
    assert "fixture_writer" not in names, (
        "a file compiled out by `#![cfg(test)]` cannot hold a production registration"
    )


def test_an_exclusive_world_write_is_not_hidden_by_an_empty_signature(
    tmp_path, monkeypatch
) -> None:
    """⛔⛤ THE FIFTH SPELLING. An exclusive-world system's signature is
    `fn f(world: &mut World)`: `_MUTABLE_PARAM_TYPE` matches it and extracts
    `World`, which is not registered, so the scan moved on and every write in
    the BODY was invisible. Measured 2026-09-18 on the real tree: nine functions
    reach a registered type this way, seven of them unseen — including
    `MovingPlatformSet`, which this guard's own docstring records as the ENTIRE
    population it could see before 2026-09-02 and which the `SystemParam` hole
    had already hidden once.

    ⚠ The `World` test in the scanner is a NARROWING, not the subject: without
    it every helper's body would be read, and a `&mut T` parameter is already
    covered by the signature pass.
    """
    crate = tmp_path / "crates" / "demo" / "src"
    crate.mkdir(parents=True)
    (crate / "lib.rs").write_text(
        """
        registrar.rollback_resource_canonical::<MovingPlatformSet>(OWNER, "x");

        pub fn commit_it(world: &mut World) {
            if let Some(mut set) = world.get_resource_mut::<MovingPlatformSet>() {
                set.0.clear();
            }
        }

        app.add_systems(PreUpdate, commit_it);
        """
    )
    monkeypatch.setattr(guard, "REPO", tmp_path)
    for cached in (
        guard.rollback_types,
        guard.mutating_systems,
        guard.system_param_mutables,
        guard._production_sources,
    ):
        cached.cache_clear()

    assert guard.mutating_systems(tmp_path)["commit_it"] == ["MovingPlatformSet"]
    assert [(n, s, t) for n, _f, s, t in guard.collect(tmp_path)] == [
        ("commit_it", "PreUpdate", ["MovingPlatformSet"])
    ]


def test_the_exclusive_world_spelling_would_be_missed_without_the_body_read(
    tmp_path, monkeypatch
) -> None:
    """⭐ THE POISON, and it is pointed at the SPELLING rather than at the count.

    `POPULATION_FLOOR` catches a collapse; it cannot catch this, and saying so
    is the point. The exclusive-world widening moved the real tree by SEVEN
    systems out of 523 — well inside any floor anybody would set — so the floor
    is not what holds this spelling. This arm is. Break
    `_EXCLUSIVE_WORLD_WRITE` and the subject below goes quiet while every other
    number in this file stays exactly where it was.
    """
    crate = tmp_path / "crates" / "demo" / "src"
    crate.mkdir(parents=True)
    (crate / "lib.rs").write_text(
        """
        registrar.rollback_resource_canonical::<MovingPlatformSet>(OWNER, "x");

        pub fn commit_it(world: &mut World) {
            world.resource_mut::<MovingPlatformSet>().0.clear();
        }

        app.add_systems(PreUpdate, commit_it);
        """
    )
    monkeypatch.setattr(guard, "REPO", tmp_path)
    import re as _re

    monkeypatch.setattr(
        guard, "_EXCLUSIVE_WORLD_WRITE", _re.compile(r"(a)(b)(?!x)(?<!y)\bzzz_no_such_write\b")
    )
    for cached in (
        guard.rollback_types,
        guard.mutating_systems,
        guard.system_param_mutables,
        guard._production_sources,
    ):
        cached.cache_clear()

    assert "commit_it" not in guard.mutating_systems(tmp_path), (
        "the body read is not what finds this write, so the arm above proves "
        "nothing about the fifth spelling"
    )


def test_the_session_world_helper_spellings_are_both_read() -> None:
    """Both session-world roads, and the `_at` one is not optional.

    `session_world_component_mut` asks *"which root is LIVE"*;
    `session_world_component_mut_at` takes the root the caller resolved, and the
    room publication moved onto it in September 2026. A pattern that read only
    the first would have gone blind on the authoritative writer the week it was
    introduced — which is exactly what happened to the multi-writer census.
    """
    matches = guard._EXCLUSIVE_WORLD_WRITE.findall(
        "session_world_component_mut::<RoomSet>(world); "
        "session_world_component_mut_at::<world::rooms::RoomGeometry>(world, root); "
        "world.get_mut::<LdtkRuntimeIndex>(root);"
    )
    found = {a or b for a, b in matches}
    assert found == {"RoomSet", "RoomGeometry", "LdtkRuntimeIndex"}, found


# ── the SIXTH spelling: the schedule LABEL, 2026-09-18 ──────────────────────
#
# The five spellings above are all ways a WRITE can be written. This one was in
# the test itself: `collect` compared the schedule label against a tuple of four
# BARE names, and the workspace spells a non-rewinding schedule in QUALIFIED form
# 39 times. Every such registration was skipped — the guard reported FEWER
# offenders, which is the direction that reads as green.


def test_a_qualified_schedule_label_is_the_same_schedule(tmp_path):
    """⛔⛤ THE DEFECT ITSELF. `bevy::prelude::Update` IS `Update`.

    Measured on the day this was found: the tree spells them qualified 39 times
    and two live offenders were invisible — `publish_player_stats_edits`
    (`bevy::app::PreUpdate`) and `reset_checkpoint_coordinator_on_activation`
    (`bevy::prelude::Update`). Both now carry waivers; neither could be ASKED
    about while the label did not match.
    """
    root = _tree(tmp_path, {
        "crates/ambition_x/src/lib.rs": "\n".join([
            "pub fn regen(mut m: Query<&mut BodyMana>) {}",
            "fn build(app: &mut App) {",
            "    app.add_systems(bevy::prelude::Update, regen);",
            "}",
        ]),
    })
    found = guard.collect(root)
    assert len(found) == 1, f"a qualified label hid the mutator: {found}"
    name, _file, schedule, hits = found[0]
    assert (name, schedule, hits) == ("regen", "Update", ["BodyMana"]), (
        "the reported schedule is NORMALIZED, so two spellings of one schedule "
        "do not read as two different findings"
    )


def test_every_qualified_spelling_in_the_tree_normalizes(tmp_path):
    """Not one prefix — the two the workspace actually uses, plus a third.

    `bevy::app::` and `bevy::prelude::` both appear at real registration sites;
    a guard keyed on either one alone would still be half blind.
    """
    for prefix in ("bevy::app::", "bevy::prelude::", "some::deep::path::"):
        root = _tree(tmp_path / prefix.replace(":", "_"), {
            "crates/ambition_x/src/lib.rs": "\n".join([
                "pub fn regen(mut m: Query<&mut BodyMana>) {}",
                "fn build(app: &mut App) {",
                f"    app.add_systems({prefix}PreUpdate, regen);",
                "}",
            ]),
        })
        found = guard.collect(root)
        assert [(r[0], r[2]) for r in found] == [("regen", "PreUpdate")], (
            f"{prefix}PreUpdate did not normalize: {found}"
        )


def test_a_qualified_rewinding_label_is_still_not_a_finding(tmp_path):
    """The inversion must not swing the other way and flag the sim schedule.

    ⚠ this is the control for the test above: normalization that only ever adds
    rows would pass that one while making the guard useless.
    """
    root = _tree(tmp_path, {
        "crates/ambition_x/src/lib.rs": "\n".join([
            "pub fn regen(mut m: Query<&mut BodyMana>) {}",
            "fn build(app: &mut App) {",
            "    app.add_systems(bevy_ggrs::GgrsSchedule, regen);",
            "    app.add_systems(bevy::app::Startup, regen);",
            "}",
        ]),
    })
    assert guard.collect(root) == []


def test_an_unanticipated_schedule_arrives_as_a_finding(tmp_path):
    """⭐ WHY THE TEST IS INVERTED RATHER THAN THE LIST WIDENED.

    The set of schedules that do NOT rewind is open — it grows with every crate
    and every third-party plugin, and an omission is silent. The set that DOES
    rewind is closed. A label nobody anticipated must therefore land in the
    population, where somebody is forced to adjudicate it.
    """
    root = _tree(tmp_path, {
        "crates/ambition_x/src/lib.rs": "\n".join([
            "pub fn regen(mut m: Query<&mut BodyMana>) {}",
            "fn build(app: &mut App) {",
            "    app.add_systems(SomeThirdPartyPlugin::RenderPass, regen);",
            "}",
        ]),
    })
    found = guard.collect(root)
    assert [(r[0], r[2]) for r in found] == [("regen", "RenderPass")]


def test_a_variable_schedule_label_is_skipped_and_that_is_the_residual(tmp_path):
    """⛔ THE RESIDUAL, EXECUTABLE SO IT CANNOT BE FORGOTTEN.

    `app.add_systems(sim, …)` is the sanctioned idiom and 258 calls use it, but
    a source scan cannot tell `sim` from `load_schedule` or any other binding.
    A variable label is SKIPPED — so a mutator behind a variable that is NOT the
    sim schedule is invisible here. This test asserts the blind spot rather than
    a capability, and it fails the day dataflow makes it resolvable.
    """
    root = _tree(tmp_path, {
        "crates/ambition_x/src/lib.rs": "\n".join([
            "pub fn regen(mut m: Query<&mut BodyMana>) {}",
            "fn build(app: &mut App) {",
            "    let not_the_sim = PostUpdate;",
            "    app.add_systems(not_the_sim, regen);",
            "}",
        ]),
    })
    assert guard.collect(root) == [], (
        "if this now reports, the residual documented on `is_schedule_variable` "
        "has been closed — delete the blind-spot note with this test"
    )


def test_every_schedule_exemption_says_why(tmp_path):
    """An exemption is a claim, so it carries an argument, like a waiver."""
    for label, why in guard.REWINDING_OR_NOT_A_TIMELINE.items():
        assert len(why) > 20, f"{label}'s exemption is a placeholder: {why!r}"


def test_the_load_bearing_schedule_exemptions_are_measured():
    """⚠ WHICH EXEMPTIONS DO WORK TODAY, AND WHICH ARE ANTICIPATORY.

    Measured 2026-09-18 by dropping each entry and re-running the real tree:
    three change the verdict — `SaveWorld` (`update`), `Startup`
    (`load_save_at_startup`) and `CheckpointDomainApply` (the commit executor's
    three domain reducers). The other six carry no current population and are
    therefore claims about the future, not measured exemptions.

    This test pins the three that are load-bearing. It does NOT require the
    other six to stay empty — it requires nobody to delete a working one while
    believing it was decorative.
    """
    load_bearing = ("SaveWorld", "Startup", "CheckpointDomainApply")
    base = {r[0] for r in guard.collect()}
    for label in load_bearing:
        saved = guard.REWINDING_OR_NOT_A_TIMELINE.pop(label)
        try:
            exposed = {r[0] for r in guard.collect()} - base
        finally:
            guard.REWINDING_OR_NOT_A_TIMELINE[label] = saved
        assert exposed, (
            f"{label} was measured load-bearing on 2026-09-18 and now exempts "
            "nothing — either its systems moved, in which case say so here, or "
            "the scan stopped seeing them"
        )
