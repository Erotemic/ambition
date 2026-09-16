//! DURABLE-HORIZON-CHECKSUM — a save mirror written from `Update`, driven across
//! a rollback window while the value it mirrors actually CHANGES.
//!
//! `persist_inventory_to_save` runs in top-level `Update`, which does not rewind,
//! and writes `AmbitionGameSave` — a `rollback_resource_clone_checksum`
//! registration, so it FEEDS THE PEER CHECKSUM. Its plugin says the placement is
//! deliberate and argues it from file side effects: writing a file twice is not a
//! desync. That argument does not reach the hashed half.
//!
//! ⛔ AND THE EXISTING SYNC TESTS CANNOT ASK THIS. `rollback_full_reset.rs` and
//! `rollback_lifecycle_reset.rs` both drive this mirror for 180 and 240 frames
//! and are green — but the mirror is VALUE-COMPARED, so in a world where the bag
//! never changes it writes once and early-returns forever after. A green window
//! over a mirror that is not writing says nothing about a mirror that is.
//!
//! ⇒ So this arm changes `OwnedItems` in the MIDDLE of the window, which is the
//! shape the menu makes: `dispatch_menu_action` equips from `Update`, outside the
//! rewind, exactly as this does.

#![cfg(feature = "rl_sim")]

use ambition_app::rl_sim::{
    AgentAction, AmbitionSim, Platformer2dSimHarness, Platformer2dSimHarnessOptions, TimestepMode,
};

type SaveRestored = ambition_platformer2d::actors::session::durable_horizon::SaveRestored;
type AmbitionGameSave = ambition_platformer2d::persistence::save::AmbitionGameSave;
type OwnedItems = ambition_platformer2d::item::OwnedItems;
type Item = ambition_platformer2d::item::Item;
type ItemGrantRequested = ambition_platformer2d::item::ItemGrantRequested;

/// The room the other rollback arms use, so a failure here is about the mirror
/// rather than about an unusual world.
const ROOM: &str = "combat_calibration_lab";

fn repro_sim() -> Platformer2dSimHarness {
    Platformer2dSimHarness::new_with_options(
        Platformer2dSimHarnessOptions::default()
            .with_timestep(TimestepMode::fixed_60hz())
            .with_required_start_room(ROOM)
            .with_sync_test_rollback_settings(4, 10),
    )
    .expect("Ambition GGRS sync-test harness builds in the calibration lab")
}

/// The SAME world with no rollback session at all. ⛔ WITHOUT THIS THE REPRO
/// PROVES NOTHING: "the bag lost an item" and "the REWIND took the item back"
/// look identical from inside one harness, and plenty of other things clear a
/// bag.
fn control_sim() -> Platformer2dSimHarness {
    Platformer2dSimHarness::new_with_options(
        Platformer2dSimHarnessOptions::default()
            .with_timestep(TimestepMode::fixed_60hz())
            .with_required_start_room(ROOM),
    )
    .expect("the same world builds without a rollback session")
}

/// What the autosave would commit — the mirrored value, not the live bag.
fn mirrored_items(sim: &Platformer2dSimHarness) -> Vec<String> {
    sim.world()
        .resource::<AmbitionGameSave>()
        .data()
        .items()
        .iter()
        .map(|item| format!("{item:?}"))
        .collect()
}

fn health(sim: &Platformer2dSimHarness) -> Result<(), String> {
    ambition_platformer2d::rollback::session_health(sim.world())
}

/// A grant that happens INSIDE the rewinding schedule, installed through the
/// `compose` callback so it is present before the harness's first update rather
/// than bolted on after its GGRS session has started.
/// ⛔ THE CONTROL FOR EVERY OTHER ARM IN THIS FILE, AND IT WAS MISSING.
/// The three arms above all run on `with_sync_test_rollback_settings`, and none
/// of them asks how many SIM TICKS their window actually contains. A window that
/// holds six ticks is not the window the arm's name claims, and an arm that
/// steps 240 times over a stopped schedule agrees with itself forever.
///
/// ⇒ This prints the tick trajectory of both harnesses over the same 240 steps.
/// The rollback one is the question; the no-rollback one is the reference that
/// says whether the room itself can tick at all.
#[test]
#[ignore = "PROBE, print-only: how many sim ticks each harness gives per 240 steps"]
fn probe_how_far_each_harness_ticks_over_the_same_window() {
    for (name, mut sim) in [
        ("rollback sync-test", repro_sim()),
        ("no rollback session", control_sim()),
        ("compose, empty system", sim_composed_with(nothing_each_tick)),
        ("compose, grant each tick", sim_composed_with(grant_each_tick)),
        (
            "compose, touch the bag, grant ZERO",
            sim_composed_with(touch_the_bag_each_tick),
        ),
        (
            "compose, grant ONCE at tick 20",
            sim_composed_with(grant_once_at_tick_20),
        ),
        (
            "compose, request a grant each tick",
            sim_composed_with(request_a_grant_each_tick),
        ),
    ] {
        // ⛔ THE BAG TRAVELS WITH THE TICK, BECAUSE A ROW WHOSE SYSTEM NEVER
        // FIRED IS CLEAN FOR THE WRONG REASON. "Granting once at tick 20 does not
        // desync" and "the grant never happened" are the same trajectory, and
        // only this column tells them apart.
        let mut trajectory: Vec<(usize, u64, u32)> =
            vec![(0, sim_tick(&sim), live_cells(&sim))];
        for frame in 1..=240 {
            sim.step(AgentAction::default());
            if frame % 40 == 0 {
                trajectory.push((frame, sim_tick(&sim), live_cells(&sim)));
            }
        }
        // ⚠ A STOPPED CLOCK HAS A REASON AND `session_health` HOLDS IT. Printing
        // the trajectory without it reports the symptom and hides the cause.
        println!(
            "[tick] {name}: (step, tick, bag)={trajectory:?} health={:?}",
            health(&sim)
        );
    }
}

/// ⛔ THE ROW'S ACTUAL QUESTION, MEASURED FRAME BY FRAME RATHER THAN ARGUED.
/// `persist_inventory_to_save` derives the checksummed `AmbitionGameSave` once
/// per FRAME in `Update`; the sim advances and is compared once per TICK. This
/// prints both clocks beside both values for the first frames of the window,
/// which is where the session is still alive.
///
/// ⚠ It runs the SANCTIONED road — `ItemGrantRequested`, which
/// `apply_item_grants` owns and which is `clear_message_on_rollback` — so a
/// mismatch here cannot be dismissed as writing rollback state from the wrong
/// place.
#[test]
#[ignore = "PROBE, print-only: the save mirror's clock against the sim's clock"]
fn probe_the_mirror_and_the_sim_do_not_share_a_clock() {
    for (name, mut sim) in [
        ("requested grant (sanctioned road)", sim_composed_with(request_a_grant_each_tick)),
        ("direct write", sim_composed_with(grant_each_tick)),
    ] {
        println!("── {name}");
        for step in 0..14 {
            // ⛔ THE RAW LIST, NOT A SUBSTRING MATCH ON IT. Filtering these for
            // "HealthCell" returned 0 for the whole window and I read that as
            // "the save is not changing" — a false negative from guessing the
            // encoding. `census_all` then showed `AmbitionGameSave` as the one
            // entry of 364 that moves with the bag.
            let mirrored = mirrored_items(&sim).join(",");
            println!(
                "   step={step:>2} tick={:>3} live={:>3} mirrored=[{mirrored}] health={:?}",
                sim_tick(&sim),
                live_cells(&sim),
                health(&sim).err().map(|error| error.chars().take(46).collect::<String>()),
            );
            sim.step(AgentAction::default());
        }
    }
}

/// ⛔ WHICH HASHED ENTRY MOVES WHEN THE BAG MOVES — asked of the registry
/// instead of guessed. Four hypotheses were each cheap and each wrong; the
/// registry already knows every entry that feeds the peer checksum, and
/// `RollbackChecksumProbes::census_all` will read them all.
///
/// The method is a DIFFERENCE BETWEEN TWO RUNS AT THE SAME TICK. ⚠ THIS DOC
/// CLAIMED A DIFFERENCE ACROSS A REWIND "CANNOT BE OBSERVED FROM OUTSIDE" AND
/// THAT IS FALSE — `RollbackRestoreAudit` censuses every save and compares when
/// GGRS saves the SAME frame twice, which is what a resimulation is, and it
/// shipped before this probe. YardratAmbition used it to read the replay
/// directly: the replay xor is CONSTANT across frames 2, 3, 4 while the
/// first-pass xor moves every frame. ⇒ Two runs at one tick prove the entry
/// FOLLOWS the bag; only the audit shows the replay stops updating it. Both
/// harnesses take the same `ResMut<OwnedItems>` at the same schedule position and
/// fire the same change detection; one grants 1 per tick and one grants 0. Any
/// hashed entry whose census differs between them is a value that derives from
/// the bag, which is exactly the population this row is missing.
///
/// ⚠ Read inside frames 0..=5. The granting run invalidates at frame 6 and
/// everything after that is a frozen world agreeing with itself.
#[test]
#[ignore = "PROBE, print-only: which hashed rollback entries follow the bag"]
fn probe_which_hashed_entries_follow_the_bag() {
    use std::collections::BTreeMap;

    fn census_at(
        sim: &mut Platformer2dSimHarness,
        steps: usize,
    ) -> BTreeMap<&'static str, (usize, u64)> {
        for _ in 0..steps {
            sim.step(AgentAction::default());
        }
        let probes = sim
            .world()
            .resource::<ambition_platformer2d::rollback::RollbackChecksumProbes>()
            .clone();
        probes
            .census_all(sim.world_mut())
            .into_iter()
            .map(|(name, census)| (name, (census.count, census.xor)))
            .collect()
    }

    // ⚠ FIVE STEPS, WHICH IS INSIDE THE LIVE WINDOW. At six the granting run is
    // already invalidated and its census describes a stopped world.
    const STEPS: usize = 5;
    let mut granting = sim_composed_with(grant_each_tick);
    let mut still = sim_composed_with(touch_the_bag_each_tick);
    println!(
        "   granting health={:?} / still health={:?}",
        health(&granting),
        health(&still)
    );
    let moved = census_at(&mut granting, STEPS);
    let held = census_at(&mut still, STEPS);
    println!(
        "   after {STEPS} steps: granting bag={} tick={}, still bag={} tick={}",
        live_cells(&granting),
        sim_tick(&granting),
        live_cells(&still),
        sim_tick(&still)
    );

    let mut differ = 0usize;
    for (name, (count, xor)) in &moved {
        match held.get(name) {
            Some((held_count, held_xor)) if held_count == count && held_xor == xor => {}
            Some((held_count, held_xor)) => {
                differ += 1;
                println!(
                    "   ≠ {name}: granting=({count}, {xor:#x}) still=({held_count}, {held_xor:#x})"
                );
            }
            None => {
                differ += 1;
                println!("   ≠ {name}: present only in the granting run");
            }
        }
    }
    // ⛔ ANTI-VACUITY: zero probes censused reads exactly like zero differences.
    println!("   {differ} of {} probed entries differ", moved.len());
    assert!(
        !moved.is_empty(),
        "no rollback checksum probes were censused at all, so this probe compared          nothing and its clean output means nothing"
    );
}

/// ⛔ WHETHER "ONE CHANGE IS CLEAN" IS A RESULT OR AN ARTEFACT.
///
/// `probe_how_far_each_harness_ticks_over_the_same_window` reports that a grant
/// firing ONCE at tick 20 runs 240 steps with `session_health` clean, and the
/// bag column proves the grant fired. ⚠ THAT IS STILL NOT ENOUGH. The sync test
/// compares a frame against its own resimulation, and the save's value is what
/// two passes disagree about — so if the save had already settled by the frames
/// GGRS actually compared, the comparison had nothing to disagree about and the
/// clean run says nothing. A floor must be raised on the quantity the claim is
/// about, not on the instrument's own activity. (YardratAmbition's lesson, from
/// an audit that would have called a stationary ground item reproducible.)
///
/// ⇒ So this prints the SAVE'S OWN CENSUS per step across the grant, next to the
/// bag. `check_distance` is 4, so the frames compared around tick 20 are the
/// handful either side of it. If the xor moves at the grant and the neighbouring
/// ticks are inside that distance, the comparison did straddle a moving value
/// and "one change is clean" is a real result.
#[test]
#[ignore = "PROBE, print-only: was the save still moving at the frames GGRS compared"]
fn probe_whether_the_single_grant_lands_where_the_comparison_looks() {
    let mut sim = sim_composed_with(grant_once_at_tick_20);
    for _ in 0..14 {
        sim.step(AgentAction::default());
    }
    for _ in 0..14 {
        let tick = sim_tick(&sim);
        let bag = live_cells(&sim);
        let probes = sim
            .world()
            .resource::<ambition_platformer2d::rollback::RollbackChecksumProbes>()
            .clone();
        let save = probes
            .census_all(sim.world_mut())
            .into_iter()
            .find(|(name, _)| name.contains("AmbitionGameSave"))
            .map(|(_, census)| census.xor);
        println!(
            "   tick={tick:>3} bag={bag:>3} save_xor={:?} health={:?}",
            save.map(|xor| format!("{xor:#018x}")),
            health(&sim).err().map(|e| e.chars().take(40).collect::<String>())
        );
        sim.step(AgentAction::default());
    }
}

/// ⛔ THE THRESHOLD BETWEEN "ONE CHANGE IS CLEAN" AND "EVERY TICK DESYNCS".
/// Both are measured; the gap between them is the whole open question, and it is
/// a PARAMETER rather than a mystery. `N` consecutive granting ticks starting at
/// 20, for a spread of `N`. The smallest `N` that reports a mismatch is the
/// answer, and a spread that is clean at every `N` says the cadence is not what
/// matters after all.
///
/// ⚠ Read the bag column, not just the health: an `N` whose grants never fired
/// is clean for the wrong reason, and the bag is what says they did.
#[test]
#[ignore = "PROBE, print-only: how many consecutive changed ticks it takes to desync"]
fn probe_how_many_consecutive_changed_ticks_desync() {
    fn run<const N: u64>() -> (u32, Option<String>) {
        fn grant<const N: u64>(
            tick: bevy::prelude::Res<ambition_platformer2d::time::SimTick>,
            mut owned: bevy::prelude::ResMut<OwnedItems>,
        ) {
            if (20..20 + N).contains(&tick.0) {
                owned.grant(Item::HealthCell, 1);
            }
        }
        let mut sim = sim_composed_with(grant::<N>);
        for _ in 0..120 {
            sim.step(AgentAction::default());
        }
        (
            live_cells(&sim),
            health(&sim)
                .err()
                .map(|error| error.chars().take(44).collect::<String>()),
        )
    }

    // ⛔ The starter bag is 3, so `bag == 3 + N` is the premise that the grants
    // fired; anything else and that row measures nothing.
    for (n, (bag, err)) in [
        (1, run::<1>()),
        (2, run::<2>()),
        (3, run::<3>()),
        (4, run::<4>()),
        (5, run::<5>()),
        (8, run::<8>()),
        // ⛔ UNBOUNDED FROM TICK 20. `N` up to 8 is clean and granting EVERY tick
        // from tick 1 dies at frames [2, 3, 4], so the question is whether the
        // cadence needs longer than 8 or whether the defect is about EARLY
        // frames. This row is the discriminator: same every-tick cadence, late
        // start. (Its premise is `bag > 3 + 8`, not `3 + N`.)
        (0, run::<10_000>()),
    ] {
        let expected_floor = if n == 0 { 3 + 8 } else { 3 + n };
        let fired = if bag >= expected_floor { "grants fired" } else { "⛔ PREMISE" };
        let label = if n == 0 { "every tick from 20".to_string() } else { format!("N={n}") };
        println!("   {label:>20} bag={bag:>3} ({fired}) health={err:?}");
    }
}

/// ⛔ NOT CADENCE — START TICK. Granting EVERY tick from tick 1 desyncs at frames
/// [2, 3, 4]; granting every tick from tick 20 is clean over 120 steps with the
/// bag reaching 104. So the cadence sweep's answer was that cadence is not the
/// variable. This sweeps the START TICK instead, granting unconditionally from
/// `FROM` onward, and the smallest `FROM` that stays clean is the edge of the
/// window where the defect lives.
#[test]
#[ignore = "PROBE, print-only: the start tick at which an every-tick grant stops desyncing"]
fn probe_which_start_tick_stops_desyncing() {
    fn run<const FROM: u64>() -> (u64, u32, Option<String>) {
        fn grant<const FROM: u64>(
            tick: bevy::prelude::Res<ambition_platformer2d::time::SimTick>,
            mut owned: bevy::prelude::ResMut<OwnedItems>,
        ) {
            if tick.0 >= FROM {
                owned.grant(Item::HealthCell, 1);
            }
        }
        let mut sim = sim_composed_with(grant::<FROM>);
        for _ in 0..120 {
            sim.step(AgentAction::default());
        }
        (
            sim_tick(&sim),
            live_cells(&sim),
            health(&sim)
                .err()
                .map(|error| error.chars().take(44).collect::<String>()),
        )
    }

    for (from, (tick, bag, err)) in [
        (1, run::<1>()),
        (2, run::<2>()),
        (4, run::<4>()),
        (6, run::<6>()),
        (8, run::<8>()),
        (12, run::<12>()),
        (16, run::<16>()),
        (20, run::<20>()),
    ] {
        // ⛔ The premise: a clean row must ALSO have kept ticking, or it is clean
        // because the session died quietly rather than because nothing diverged.
        let premise = if bag > 3 { "granted" } else { "⛔ NEVER GRANTED" };
        println!(
            "   from tick {from:>3}: end tick={tick:>4} bag={bag:>4} ({premise}) health={err:?}"
        );
    }
}

/// ⛔ ACCUMULATION OR AN UPDATE-CADENCE LAG — two candidates with OPPOSITE
/// predictions, separated by one write. YardratAmbition's design; a mechanism
/// that explains the number is not evidence for it, so this asks the tree.
///
/// **A — ACCUMULATION.** `grant` makes the bag `3 + (times the system has RUN)`,
/// which is not a function of the FRAME, because a resimulation re-executes
/// steps. Predicts: a value that changes every tick but is a pure function of
/// the tick runs clean.
///
/// **B — THE ONE-UPDATE LAG.** The mirror runs in `Update`, once per
/// `app.update()`, while GGRS snapshots during the sim schedule — so a frame's
/// snapshot holds the save as of the PREVIOUS update, and how many sim steps sat
/// in that update differs between passes. Predicts: a pure function of the tick
/// still desyncs, because the lag is about WHEN the mirror ran.
///
/// ⚠ The second row is the control that keeps either answer from being about the
/// harness: the same write fired ONCE must stay clean, reproducing `b0b7280dc`.
#[test]
#[ignore = "PROBE, print-only: accumulation versus an Update-cadence lag"]
fn probe_whether_a_pure_function_of_the_tick_also_desyncs() {
    fn set_from_tick(
        tick: bevy::prelude::Res<ambition_platformer2d::time::SimTick>,
        mut owned: bevy::prelude::ResMut<OwnedItems>,
    ) {
        // ⛔ A PURE FUNCTION OF THE TICK, which is the whole point: `take`
        // everything first so the result cannot depend on how many times this ran.
        owned.take(Item::HealthCell, u32::MAX);
        owned.grant(Item::HealthCell, (tick.0 % 5) as u32);
    }
    fn set_from_tick_once(
        tick: bevy::prelude::Res<ambition_platformer2d::time::SimTick>,
        mut owned: bevy::prelude::ResMut<OwnedItems>,
    ) {
        if tick.0 == 20 {
            owned.take(Item::HealthCell, u32::MAX);
            owned.grant(Item::HealthCell, 2);
        }
    }

    // ⛔ THE ROW THAT STOPS THIS REPEATING THE LAST MISTAKE. `set_from_tick`
    // writes from tick 1, and the accumulating reproduction also writes from tick
    // 1 while the one-shot control fires at 20 — so without this row the
    // experiment varies START TICK alongside the arithmetic, which is exactly the
    // confound that made "sustained versus single" wrong. An accumulating grant
    // from tick 4 is clean; a pure function from tick 4 must be compared against
    // THAT, not against the once-at-20 control.
    fn set_from_tick_after_3(
        tick: bevy::prelude::Res<ambition_platformer2d::time::SimTick>,
        mut owned: bevy::prelude::ResMut<OwnedItems>,
    ) {
        if tick.0 >= 4 {
            owned.take(Item::HealthCell, u32::MAX);
            owned.grant(Item::HealthCell, (tick.0 % 5) as u32);
        }
    }

    for (name, mut sim) in [
        ("pure f(tick), EVERY tick", sim_composed_with(set_from_tick)),
        (
            "pure f(tick), every tick FROM 4",
            sim_composed_with(set_from_tick_after_3),
        ),
        ("pure f(tick), ONCE at 20", sim_composed_with(set_from_tick_once)),
        ("accumulating grant, EVERY tick", sim_composed_with(grant_each_tick)),
    ] {
        // ⛔ THE PREMISE, AND `bag=0` IS WHY IT IS HERE. `tick % 5` is zero once
        // every five ticks, so the FINAL bag reads 0 both when the write ran and
        // when it never ran at all. The high-water mark separates them: a write
        // that ran reaches 4, and one that never ran never leaves the starter 3.
        let mut peak = live_cells(&sim);
        for _ in 0..120 {
            sim.step(AgentAction::default());
            peak = peak.max(live_cells(&sim));
        }
        println!(
            "   {name:>32}: end tick={:>4} bag={:>4} peak={peak:>4} health={:?}",
            sim_tick(&sim),
            live_cells(&sim),
            health(&sim)
                .err()
                .map(|error| error.chars().take(40).collect::<String>())
        );
    }
}

/// ⛔ WHY THE FIRST THREE TICKS ARE DIFFERENT — the one candidate left standing
/// after accumulation and the update-lag were both refuted, and it is offered as
/// a candidate rather than asserted.
///
/// `complete_durable_restore` sets `SaveRestored` ONCE, from `Update`, on the
/// first frame a primary player body exists, and that latch gates whether the
/// three save mirrors write at all. It is `rollback_resource_clone`, so a rewind
/// RESTORES it — and a rewind into a frame where it was still false lets the next
/// `Update` run `complete_durable_restore` a second time, which also writes
/// `ResetToCheckpoint`. Once it has settled, restoring `true` over `true` is a
/// no-op. ⇒ That shape would make ticks 1..=3 special and everything after
/// boring, which is the shape the measurements have.
///
/// This prints the latch beside the tick and the save's census for the first
/// steps of both a desyncing run and a clean one.
#[test]
#[ignore = "PROBE, print-only: does the SaveRestored latch move during the first ticks"]
fn probe_whether_the_restore_latch_settles_before_the_window() {
    fn latch(sim: &Platformer2dSimHarness) -> Option<bool> {
        sim.world().get_resource::<SaveRestored>().map(|l| l.0)
    }
    fn save_xor(sim: &mut Platformer2dSimHarness) -> Option<u64> {
        let probes = sim
            .world()
            .resource::<ambition_platformer2d::rollback::RollbackChecksumProbes>()
            .clone();
        probes
            .census_all(sim.world_mut())
            .into_iter()
            .find(|(name, _)| name.contains("AmbitionGameSave"))
            .map(|(_, census)| census.xor)
    }

    for (name, mut sim) in [
        ("grants from tick 1 (desyncs)", sim_composed_with(grant_each_tick)),
        ("no writer at all (clean)", sim_composed_with(nothing_each_tick)),
    ] {
        println!("── {name}");
        for step in 0..10 {
            let (tick, bag, l) = (sim_tick(&sim), live_cells(&sim), latch(&sim));
            let xor = save_xor(&mut sim);
            println!(
                "   step={step:>2} tick={tick:>3} bag={bag:>3} SaveRestored={l:?}                  save_xor={:?} health={:?}",
                xor.map(|x| format!("{x:#018x}")),
                health(&sim).err().map(|e| e.chars().take(34).collect::<String>())
            );
            sim.step(AgentAction::default());
        }
    }
}

/// ⛔ DOES THE WORLD STOP CHANGING WHERE THE DESYNC STOPS? YardratAmbition's
/// candidate for why ticks 1..=3 are special: the frame-1 lifecycle trace shows
/// roots admitted, a candidate session published and entities promoted, so the
/// early ticks are the only ones at which the ENTITY POPULATION is still moving.
/// That is a structural property of the window rather than of anything a test
/// writes, and it would explain why the START TICK is the only variable that
/// predicts the outcome.
///
/// ⚠ It has a constraint to satisfy, from a measurement already taken: a system
/// granting ZERO every tick FROM TICK 1 is clean. So the window alone is not
/// sufficient — the property has to be a conjunction, a CHANGED hashed value
/// during a window that is still settling.
///
/// This prints the roster size per tick, with no writer at all, so the answer is
/// about the world rather than about a probe's writes.
#[test]
#[ignore = "PROBE, print-only: when the entity population stops changing"]
fn probe_when_the_world_stops_settling() {
    let mut sim = sim_composed_with(nothing_each_tick);
    let mut previous = feature_roster(&mut sim).len();
    println!("   tick={:>3} roster={previous:>4} (before any step)", sim_tick(&sim));
    for _ in 0..12 {
        sim.step(AgentAction::default());
        let now = feature_roster(&mut sim).len();
        let moved = if now == previous { "" } else { "  ← CHANGED" };
        println!("   tick={:>3} roster={now:>4}{moved}", sim_tick(&sim));
        previous = now;
    }
}

/// The schedule's own step count. This file never writes it, which is the
/// point: it is the control column for a frozen bag.
fn sim_tick(sim: &Platformer2dSimHarness) -> u64 {
    sim.world()
        .get_resource::<ambition_platformer2d::time::SimTick>()
        .map(|tick| tick.0)
        .unwrap_or(u64::MAX)
}

fn grant_each_tick(mut owned: bevy::prelude::ResMut<OwnedItems>) {
    owned.grant(Item::HealthCell, 1);
}

/// The other half of the bisect: same road into the schedule, no writes at all.
fn nothing_each_tick() {}

/// ⛔ ONE CHANGE, NOT ONE PER TICK — which is what decides whether Q129 is
/// urgent. A pickup during play changes the bag ONCE; this reproduction changes
/// it every tick, which is the loudest possible version of the defect.
///
/// ⚠ The gate is `SimTick`, and that is the whole reason this is a valid
/// one-shot. A `Local` flag would NOT be restored by a rewind, so the replay
/// would skip the grant the original pass performed and manufacture a divergence
/// of its own. `SimTick` is rollback state, so the replay re-enters the same
/// branch on the same tick.
fn grant_once_at_tick_20(
    tick: bevy::prelude::Res<ambition_platformer2d::time::SimTick>,
    mut owned: bevy::prelude::ResMut<OwnedItems>,
) {
    if tick.0 == 20 {
        owned.grant(Item::HealthCell, 1);
    }
}

/// ⛔ THE CONTROL THAT SEPARATES THE VALUE FROM THE SYSTEM. This takes the same
/// `ResMut<OwnedItems>` at the same ambiguous schedule position and fires the
/// same change detection, but grants ZERO — the bag's value never moves. If this
/// desyncs, the trigger is a system's PRESENCE in the schedule and nothing in
/// this file is about items at all.
fn touch_the_bag_each_tick(mut owned: bevy::prelude::ResMut<OwnedItems>) {
    owned.grant(Item::HealthCell, 0);
}

/// The SANCTIONED road, which is the whole difference from `grant_each_tick`:
/// ask for the grant instead of performing it. `apply_item_grants` is already in
/// the sim schedule and owns the write, and `ItemGrantRequested` is
/// `clear_message_on_rollback`, so a rewind drops the request and the replay
/// re-issues it.
fn request_a_grant_each_tick(
    mut requests: bevy::prelude::MessageWriter<ItemGrantRequested>,
) {
    requests.write(ItemGrantRequested {
        item: Item::HealthCell,
        count: 1,
    });
}

fn sim_that_grants_inside_the_tick() -> Platformer2dSimHarness {
    sim_composed_with(grant_each_tick)
}

fn sim_composed_with<M>(
    system: impl bevy::prelude::IntoScheduleConfigs<bevy::ecs::system::ScheduleSystem, M>
    + Clone
    + Send
    + Sync
    + 'static,
) -> Platformer2dSimHarness {
    use ambition_platformer2d::sim::SimScheduleExt;
    Platformer2dSimHarness::build(
        Platformer2dSimHarnessOptions::default()
            .with_timestep(TimestepMode::fixed_60hz())
            .with_required_start_room(ROOM)
            .with_sync_test_rollback_settings(4, 10),
        |app, options| {
            ambition_app::rl_sim::ambition_sim_composition(app, options)?;
            let label = app.sim_schedule();
            app.add_systems(label, system.clone());
            Ok(())
        },
    )
    .expect("the sync-test harness builds with a grant inside the tick")
}

/// The room-scoped roster. A full sandbox reset despawns this whole set and
/// respawns it, and despawn bumps the generation, so a reconstruction that
/// really ran leaves NO original `Entity` value behind. Borrowed from
/// `rollback_full_reset.rs`, which uses it for the same discrimination.
fn feature_roster(sim: &mut Platformer2dSimHarness) -> std::collections::HashSet<bevy::prelude::Entity> {
    use bevy::prelude::With;
    let world = sim.world_mut();
    let mut q = world.query_filtered::<bevy::prelude::Entity, With<
        ambition_platformer2d::platformer::lifecycle::FeatureSimEntity,
    >>();
    q.iter(world).collect()
}

/// The LIVE bag, which is what separates the two ways this arm can fail: a
/// mirror that did not write, and a write the rewind took back before the
/// mirror ever saw it.
///
/// ⚠ A CONSUMABLE ON PURPOSE. `OwnedItems::grant` clamps every non-`Consumable`
/// category to 1, so a held tool cannot be granted twice and cannot be granted
/// at all if the starter bag already holds one — which would make these arms
/// report "the rewind no longer takes it back", i.e. read as FIXED.
fn live_cells(sim: &Platformer2dSimHarness) -> u32 {
    sim.world().resource::<OwnedItems>().count(Item::HealthCell)
}

#[test]
fn a_bag_changed_from_update_is_silently_taken_back_by_the_rewind() {
    let mut sim = repro_sim();

    // ── PREMISE 1: the latch. `persist_inventory_to_save` early-returns on
    // `!restored.0` forever, so an arm that never flips it measures nothing.
    // ⚠ The latch asks for a primary player BODY, not for a save file.
    let mut latched = false;
    for _ in 0..240 {
        sim.step(AgentAction::default());
        if sim
            .world()
            .get_resource::<SaveRestored>()
            .is_some_and(|restored| restored.0)
        {
            latched = true;
            break;
        }
    }
    assert!(
        latched,
        "`SaveRestored` never became true, so all three save mirrors early-return \
         forever and a green window below would say nothing about rollback"
    );

    // Let the first mirror write settle, so what follows is a CHANGE rather than
    // the initial population of an empty save.
    for _ in 0..30 {
        sim.step(AgentAction::default());
    }
    let before = mirrored_items(&sim);
    let baseline = live_cells(&sim);

    // ── THE WRITE, in the shape the menu makes it: `OwnedItems` mutated from
    // outside the rewinding schedule, mid-window.
    {
        let world = sim.world_mut();
        let mut owned = world.resource_mut::<OwnedItems>();
        owned.grant(Item::HealthCell, 1);
    }
    let granted = live_cells(&sim);
    assert_eq!(
        granted,
        baseline + 1,
        "the grant did not reach the live bag, so nothing below is measuring a change"
    );

    // ── PREMISE 2, MID-ARM: the mirror has to actually write, and it has to have
    // written by the time the window has run. A value that is right at frame 0
    // and right at frame N can have been lost and re-established in between, so
    // this is sampled rather than only compared at the ends.
    let mut wrote_at: Option<usize> = None;
    let mut lost_after_writing: Vec<usize> = Vec::new();
    let mut live_lost_at: Option<usize> = None;

    for frame in 0..240 {
        sim.step(AgentAction::default());
        health(&sim).unwrap_or_else(|error| {
            panic!(
                "frame {frame} desynced after the bag changed from `Update`: {error}\n\
                 mirrored items now: {:?}",
                mirrored_items(&sim)
            )
        });
        if live_lost_at.is_none() && live_cells(&sim) < granted {
            live_lost_at = Some(frame);
        }
        let now = mirrored_items(&sim);
        if wrote_at.is_none() && now != before {
            wrote_at = Some(frame);
        } else if wrote_at.is_some() && now == before {
            lost_after_writing.push(frame);
        }
    }

    // ⛔⛤ THIS ARM ASSERTS A DEFECT, DELIBERATELY. Everything above is the
    // reproduction; what follows records what the engine DOES today so that the
    // day it changes, somebody is told. ⇒ WHEN THIS ARM GOES RED THE DEFECT IS
    // FIXED: delete it and close `MENU-RESET-MIDSESSION` in docs/planning/queue.md.
    //
    // ⚠ It is paired with `the_control_keeps_the_same_grant_when_nothing_rewinds`,
    // which runs the SAME grant in the SAME world with no rollback session and
    // keeps it for 240 frames. Without that control "the bag lost an item" and
    // "the REWIND took the item back" are indistinguishable.
    // ⚠ `is_some`, not a frame number. MEASURED at frame 0 on 2026-09-16 — the
    // sync-test rewinds every step, so the loss is immediate — but pinning the
    // index would redden this shared lane for a scheduling change that does not
    // alter the claim. The claim is THAT the rewind takes it back.
    assert!(
        live_lost_at.is_some(),
        "the rewind no longer takes back a bag changed from `Update` — that is \
         the FIX this arm is waiting for, not a regression. Delete this arm and \
         close MENU-RESET-MIDSESSION."
    );
    assert!(
        wrote_at.is_none(),
        "the save mirror wrote at {wrote_at:?} after a grant the rewind took \
         back, which is a DIFFERENT state from the one measured on 2026-09-16 \
         (the value was restored before `persist_inventory_to_save` ever saw a \
         difference, so it never wrote at all). Re-read the row before changing \
         this."
    );
    assert_eq!(
        before,
        mirrored_items(&sim),
        "the mirrored bag moved even though the live grant was taken back"
    );
    assert!(
        lost_after_writing.is_empty(),
        "unreachable while the mirror never writes; kept so the sampling above \
         is not dead code if the behaviour changes"
    );
}

/// ⛔ THE CONTROL, AND THE ARM ABOVE IS WORTHLESS WITHOUT IT. "The bag lost an
/// item" and "the REWIND took the item back" are indistinguishable from inside
/// one harness, and plenty of things clear a bag. This runs the SAME grant in
/// the SAME world with no rollback session and keeps it for 240 frames.
#[test]
fn the_control_keeps_the_same_grant_when_nothing_rewinds() {
    let mut sim = control_sim();
    for _ in 0..120 {
        sim.step(AgentAction::default());
        if sim
            .world()
            .get_resource::<SaveRestored>()
            .is_some_and(|restored| restored.0)
        {
            break;
        }
    }
    for _ in 0..30 {
        sim.step(AgentAction::default());
    }
    {
        let world = sim.world_mut();
        world.resource_mut::<OwnedItems>().grant(Item::HealthCell, 1);
    }
    let granted = live_cells(&sim);
    assert!(granted > 0, "the grant did not reach the live bag at all");

    for frame in 0..240 {
        sim.step(AgentAction::default());
        assert!(
            live_cells(&sim) >= granted,
            "the control lost the grant at frame {frame} with NO rollback session \
             running, so the repro arm's loss is not evidence about the rewind — \
             find what else clears `OwnedItems` before reading it that way"
        );
    }
}

/// The OTHER half of `MENU-RESET-MIDSESSION`, and it is a different failure.
///
/// `NewGameResetRequested` is `rollback_resource_canonical`, which
/// `RollbackEntryKind::feeds_peer_checksum` reports as hashed — so unlike the
/// bag above, a local-only write to it is something the peers can DISAGREE
/// about rather than something that vanishes quietly.
///
/// ⛔ `rollback_full_reset.rs` sets this same flag and then calls
/// `rebase_rollback_history()`, folding it into the baseline ON PURPOSE, so the
/// reconstruction runs on the baseline frame and every re-simulation of it. That
/// is the safe shape by construction. This arm is the same write WITHOUT the
/// rebase, which is what the menu actually does.
#[test]
fn a_reset_requested_from_update_mid_window() {
    type NewGameResetRequested =
        ambition_platformer2d::actors::session::reset::NewGameResetRequested;

    let mut sim = repro_sim();
    for _ in 0..60 {
        sim.step(AgentAction::default());
    }
    health(&sim).expect("the window is clean before the flag is set");

    let roster_before = feature_roster(&mut sim);
    assert!(!roster_before.is_empty(), "the room has a roster before the request");

    // The menu's shape: set from outside the rewinding schedule, no rebase.
    {
        let world = sim.world_mut();
        world.resource_mut::<NewGameResetRequested>().request = true;
    }

    let mut still_requested_at: Vec<usize> = Vec::new();
    let mut desync: Option<(usize, String)> = None;
    for frame in 0..180 {
        sim.step(AgentAction::default());
        if sim
            .world()
            .get_resource::<NewGameResetRequested>()
            .is_some_and(|flag| flag.request)
        {
            still_requested_at.push(frame);
        }
        if desync.is_none() {
            if let Err(error) = health(&sim) {
                desync = Some((frame, error));
            }
        }
    }

    // ⚠ THE PREMISE, AND IT HAS THREE POSSIBLE ANSWERS RATHER THAN TWO. A clean
    // window here is only interesting if the flag was actually LIVE for part of
    // it. If the rewind took the flag back the way it took the bag back, then
    // `still_requested_at` is empty and this arm has reproduced the SILENT
    // failure again rather than shown the hashed one to be safe.
    assert!(
        desync.is_none(),
        "a reset requested from `Update` mid-window DESYNCED at {desync:?} — that \
         is the hashed-half failure MENU-RESET-MIDSESSION predicted, and it is a \
         stronger result than the silent one the bag arm found. \
         flag still set on frames: {still_requested_at:?}"
    );
    // ⭐ THE DISCRIMINATOR. The flag was never observed set after the write, and
    // that has two causes with opposite meanings: the sim CONSUMED it (a reset
    // ran, and the room was rebuilt) or the rewind TOOK IT BACK (nothing
    // happened, the same silent loss the bag arm found). A full sandbox reset
    // despawns and respawns the room-scoped roster, so a reconstruction that
    // really ran shares no `Entity` with the roster before it.
    let roster_after = feature_roster(&mut sim);
    let reconstructed = roster_before.is_disjoint(&roster_after);
    assert!(
        still_requested_at.is_empty(),
        "the flag survived the window on frames {still_requested_at:?}, which is a \
         THIRD outcome this arm has not seen and the row is not written for"
    );
    // ⛔✦ ASSERTS THE DEFECT, like its sibling. MEASURED 2026-09-16: the room was
    // NOT rebuilt — 7 of 7 roster entities survived — so a reset requested from
    // `Update` is taken back by the rewind exactly as the bag is. RED here means
    // the request now survives, which is the FIX; delete the arm and close the
    // row.
    //
    // ⚠⚠ WHAT THIS ARM CANNOT SHOW, and the row must not claim: the sync-test
    // harness is ONE peer replaying itself, so a write that is erased identically
    // on every replay produces no mismatch to detect. `NewGameResetRequested`
    // feeds the peer checksum, and whether TWO peers would disagree before the
    // erase is a question no single-peer harness can answer. The local loss is
    // measured; the cross-peer divergence is not.
    assert!(
        !reconstructed,
        "the reset request now survives the rewind and the room WAS rebuilt — \
         that is the fix this arm is waiting for, not a regression"
    );
    assert_eq!(
        roster_before.intersection(&roster_after).count(),
        roster_before.len(),
        "the roster partially changed, which is neither outcome this arm knows \
         how to read — re-measure before trusting either assertion above"
    );
}

/// DURABLE-HORIZON-CHECKSUM's own question, which is the OPPOSITE of the two
/// arms above: what happens when the mirrored value changes LEGITIMATELY, inside
/// the rewinding schedule, so the `Update` mirror has a real change to carry?
///
/// `persist_inventory_to_save` derives `AmbitionGameSave` from simulation state
/// once per FRAME, while the value is snapshotted and compared once per TICK. A
/// peer that rewound and re-simulated three ticks ran the sim three extra times
/// and `Update` zero extra times.
///
/// ⚠ The grant is unconditional rather than one-shot ON PURPOSE. A "fire once"
/// flag would itself have to be rollback state for the one-shot to replay
/// correctly, and a flag that survives the rewind would suppress the replayed
/// grant and manufacture the divergence this arm is looking for.
/// ⛔ INCONCLUSIVE ABOUT THE MIRROR, AND IT FAILED IN A MORE USEFUL DIRECTION.
/// Kept as a probe because the scaffolding is the expensive part and because the
/// measurement below is the thing worth starting from.
///
/// MEASURED 2026-09-16, `SimTick` and the bag over the same 240 `sim.step()`
/// calls, bisected against `probe_how_far_each_harness_ticks_over_the_same_window`:
///
/// | harness | tick at 0/40/…/240 |
/// |---|---|
/// | `new_with_options` sync-test | 1, 41, 81, 121, 161, 201, 241 |
/// | no rollback session | 0, 40, 80, …, 240 |
/// | `build` + compose, EMPTY system | 1, 41, 81, 121, 161, 201, 241 |
/// | `build` + compose, THIS grant | 1, 6, 6, 6, 6, 6, 6 |
///
/// ⛔ THE CLOCK STOPS BECAUSE THE SESSION DIED, AND NOTHING IN THE STEP LOOP
/// SAYS SO. `session_health` reads `GGRS sync-test checksum mismatch at frames
/// [2, 3, 4, …]` on that last row, repeating forever. An invalidated session
/// keeps accepting `sim.step()` and returns an observation every time; it simply
/// stops advancing `SimTick`. Every assertion after the invalidation runs over a
/// frozen world and agrees with itself.
///
/// ⚠ SO MY FIRST READING WAS WRONG IN THE WAY THAT MATTERS: I wrote that "the
/// simulation stops advancing", which is true of this harness and of NO OTHER
/// one here. The compose road is innocent — an empty system through the same
/// callback ticks 1:1. What stalls the session is THIS SYSTEM writing
/// `OwnedItems` from a bare sim system, and `OwnedItems` is
/// `rollback_resource_clone` (`ambition_items/src/rollback_registration.rs:11`).
/// ⇒ A rollback-registered resource has one sanctioned writer road
/// (`ItemGrantRequested` → `apply_item_grants`), and writing it directly from an
/// unordered system desyncs the sync test rather than being ignored.
///
/// ⇒ WHAT IS STILL OWED for the mirror question is unchanged and now has a
/// shape: a window in which the mirrored value changes tick over tick WITHOUT
/// desyncing — which means going through `ItemGrantRequested`, not around it.
#[test]
#[ignore = "PROBE, print-only: the in-sim grant desyncs the sync test, which freezes the clock"]
fn probe_a_bag_changed_inside_the_sim_is_mirrored_across_the_window() {
    let mut sim = sim_that_grants_inside_the_tick();

    // ⚠ THE PREMISE IS "MY SYSTEM RAN", AND `count > 0` DOES NOT SAY THAT — the
    // starter bag may already hold cells. Two samples, and the SECOND must exceed
    // the first.
    let at_start = live_cells(&sim);
    for _ in 0..60 {
        sim.step(AgentAction::default());
    }
    let settled = live_cells(&sim);
    assert!(
        settled > at_start,
        "the in-sim grant did not accumulate over 60 steps ({at_start} -> \
         {settled}), so the system was never reached and nothing below measures \
         the mirror"
    );

    let mut desync: Option<(usize, String)> = None;
    let mut mirror_moved = false;
    // ⚠ THE TICK IS IN THIS TUPLE BECAUSE "THE SIM STOPPED" AND "MY WRITER
    // STOPPED" LOOK IDENTICAL FROM THE BAG ALONE. Only a column that the
    // schedule owns, and that this file does not write, tells them apart.
    let mut trajectory: Vec<(usize, u64, u32)> = Vec::new();
    let before = mirrored_items(&sim);
    for frame in 0..240 {
        sim.step(AgentAction::default());
        if desync.is_none() {
            if let Err(error) = health(&sim) {
                desync = Some((frame, error));
            }
        }
        if mirrored_items(&sim) != before {
            mirror_moved = true;
        }
        if frame % 40 == 0 {
            trajectory.push((frame, sim_tick(&sim), live_cells(&sim)));
        }
    }
    trajectory.push((240, sim_tick(&sim), live_cells(&sim)));

    assert!(
        live_cells(&sim) > settled,
        "the in-sim grant stopped accumulating (settled={settled}, \
         trajectory={trajectory:?} as (step, tick, cells), health={:?}), so the \
         window ran over a value that was not changing and a clean result says \
         nothing. ⇒ If the tick column is also frozen, read the health: a \
         session that invalidated keeps accepting steps and stops advancing",
        health(&sim)
    );
    assert!(
        mirror_moved,
        "`persist_inventory_to_save` never mirrored the changing bag, so this \
         arm measured a mirror that was not writing"
    );
    assert!(
        desync.is_none(),
        "⛔ mirroring a value that changes INSIDE the sim from a system that runs \
         once per FRAME desynced at {desync:?} — that is DURABLE-HORIZON-CHECKSUM's \
         per-frame-versus-per-tick question answered in the affirmative"
    );
}

// ---------------------------------------------------------------------------
// The one-shot pair against GGRS start
// ---------------------------------------------------------------------------

/// One frame's answer to the three questions the ordering turns on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct DurableRestoreFrame {
    session_world: bool,
    primary_bodies: usize,
    ggrs_live: bool,
    restored: bool,
    ggrs_frame: i32,
}

#[derive(bevy::prelude::Resource, Default)]
struct DurableRestoreLog(Vec<DurableRestoreFrame>);

fn record_durable_restore_order(world: &mut bevy::prelude::World) {
    let session_world =
        ambition_platformer2d::platformer::lifecycle::session_world_entity(world).is_some();
    let primary_bodies = world
        .query_filtered::<bevy::prelude::Entity, ambition_platformer2d::platformer::markers::PrimaryPlayerOnly>()
        .iter(world)
        .count();
    let ggrs_live = world.contains_resource::<ambition_platformer2d::rollback::AmbitionGgrsSession>();
    let restored = world
        .get_resource::<SaveRestored>()
        .is_some_and(|latch| latch.0);
    let ggrs_frame = world
        .get_resource::<ambition_platformer2d::rollback::RollbackFrameCount>()
        .map(|count| count.0)
        .unwrap_or(-1);
    world
        .resource_mut::<DurableRestoreLog>()
        .0
        .push(DurableRestoreFrame {
            session_world,
            primary_bodies,
            ggrs_live,
            restored,
            ggrs_frame,
        });
}

fn sim_recording_the_restore_order() -> Platformer2dSimHarness {
    Platformer2dSimHarness::build(
        Platformer2dSimHarnessOptions::default()
            .with_timestep(TimestepMode::fixed_60hz())
            .with_required_start_room(ROOM)
            .with_sync_test_rollback_settings(4, 10),
        |app, options| {
            ambition_app::rl_sim::ambition_sim_composition(app, options)?;
            app.init_resource::<DurableRestoreLog>();
            app.init_resource::<WithinFrameOrder>();
            // `Last`, so the frame is read after every schedule that could have
            // moved any of the four facts.
            app.add_systems(bevy::prelude::Last, record_durable_restore_order);
            // The probe that resolves WITHIN the frame both facts flip on: it
            // takes an edge to the latch and samples the OTHER subject.
            use bevy::prelude::IntoScheduleConfigs as _;
            app.add_systems(
                bevy::prelude::Update,
                sample_ggrs_after_the_latch.after(
                    ambition_platformer2d::actors::session::durable_horizon::complete_durable_restore,
                ),
            );
            Ok(())
        },
    )
    .expect("the sync-test harness builds with the restore-order recorder")
}

/// When does the durable-restore chain run, relative to the frame GGRS starts?
///
/// ⛔ THE ANSWER IS "AFTER", WHICH IS THE UNFAVOURABLE ONE. The chain —
/// `adopt_occurrence_checkpoint_from_save`, `restore_inventory_from_save`,
/// `complete_durable_restore` — sits in top-level `Update` and writes
/// rollback-registered state. The session-scope waivers excuse a write like that
/// when it PRECEDES the timeline. Measured here, the timeline precedes the write:
/// the session goes live on frame 1 or 2 and the latch flips on frame 2, and the
/// within-frame probe — a sampler with an explicit `.after(complete_durable_restore)`
/// edge — finds the GGRS session ALREADY LIVE at the instant the latch has just
/// been set.
///
/// ⛔⛤ AND IT IS INSIDE THE REWIND WINDOW, which is what makes it a defect rather
/// than an ordering curiosity. `RollbackFrameCount` reads **1** at that instant —
/// timeline frame one, not "before frame zero" — and the sync-test settings give
/// a check distance of four, so a resimulation reaches back past it. The three
/// restored resources are `rollback_resource_clone_checksum` registrations, so a
/// rewind across frame 1 restores them to their pre-write snapshot and `Update`
/// does not re-run.
///
/// ⚠ AND THE GAP IS NOT STABLE AGAINST UNRELATED COMPOSITION CHANGES, which is
/// the more useful half. `maintain_local_session` runs in `Update` in
/// `LocalSessionSet::Maintain`, ordered only `.after(InputSet::Collect)`; the
/// restore chain is in top-level `Update` with no edge to it at all. Adding ONE
/// exclusive system to `Update` — this probe's own within-frame sampler — moved
/// the session start from frame 2 to frame 1 and shortened the boot by a frame:
///
///     without the within-frame sampler   ggrs@2 restored@2, 35 frames, 3/3 runs
///     with it                            ggrs@1 restored@2, 34 frames, 6/6 runs
///
/// ⇒ So this probe PERTURBS ITS OWN SUBJECT, and that is the finding rather than
/// a caveat: each configuration is perfectly repeatable and they disagree, so the
/// order these two land in is a property of the whole `Update` set and not of
/// either system. Do not read the exact frame numbers as the fact. The fact is
/// that nothing orders them.
#[test]
#[ignore = "PROBE, print-only: reports the frame each of the four durable-restore facts first becomes true"]
fn probe_when_the_durable_restore_latch_flips_against_ggrs_start() {
    let mut sim = sim_recording_the_restore_order();
    for _ in 0..30 {
        sim.step(AgentAction::default());
    }
    let log = sim.world().resource::<DurableRestoreLog>().0.clone();
    let first = |pred: fn(&DurableRestoreFrame) -> bool| {
        log.iter().position(pred).map(|i| i as i64).unwrap_or(-1)
    };
    eprintln!(
        "PROBE frames={} session_world@{} body@{} ggrs@{} restored@{}",
        log.len(),
        first(|f| f.session_world),
        first(|f| f.primary_bodies > 0),
        first(|f| f.ggrs_live),
        first(|f| f.restored),
    );
    for (i, f) in log.iter().enumerate().take(6) {
        eprintln!("  frame {i}: {f:?}");
    }
    eprintln!(
        "PROBE within-frame {:?}",
        sim.world().resource::<WithinFrameOrder>()
    );
}

#[derive(bevy::prelude::Resource, Default, Debug)]
struct WithinFrameOrder {
    ggrs_live_just_after_the_latch: Option<bool>,
    ggrs_frame_when_the_latch_was_set: Option<i32>,
}

fn sample_ggrs_after_the_latch(world: &mut bevy::prelude::World) {
    let latched = world
        .get_resource::<SaveRestored>()
        .is_some_and(|latch| latch.0);
    if !latched {
        return;
    }
    let live = world.contains_resource::<ambition_platformer2d::rollback::AmbitionGgrsSession>();
    let frame = world
        .get_resource::<ambition_platformer2d::rollback::RollbackFrameCount>()
        .map(|count| count.0)
        .unwrap_or(-1);
    let mut order = world.resource_mut::<WithinFrameOrder>();
    if order.ggrs_live_just_after_the_latch.is_none() {
        order.ggrs_live_just_after_the_latch = Some(live);
        order.ggrs_frame_when_the_latch_was_set = Some(frame);
    }
}

/// A MID-SESSION LOAD, with the rollback timeline already live and settled.
///
/// The boot ordering probe above shows the durable-restore chain landing at GGRS
/// frame 1. This asks the sharper version of the same question: re-arm the latch
/// long after the timeline has settled, hand the chain a save that is NOT empty,
/// and see what the restore audit says about where the writes happened.
///
/// ⛔ `restore_inventory_from_save` carries a waiver FOR THE ACTIVATION CASE
/// ONLY, and `durable_horizon.rs` supports a mid-session load — so this is the
/// case the waiver excluded and nobody had driven.
///
/// ⚠ WHY AN EMPTY SAVE PROVES NOTHING HERE, and it is why the lane's green on
/// `no_registered_type_is_written_outside_the_rewinding_schedule` must not be
/// quoted against this: `adopt_the_ledger` writes what it read, so against the
/// harness's empty save it writes the same empty value the baselines already
/// hold, and a value-compared audit sees two identical censuses. The seeded
/// occurrence below is the whole point.
/// ⛔⛤ **MEASURED 2026-09-16 AND IT DESYNCS.** With the load staged at tick 40:
///
///     written_outside_the_rewinding_schedule()  ["...continuity::OccurrenceBaseline"]
///     session_health()                          Err("checksum mismatch at frames [38, 39, 40]")
///
/// ⇒ This is the POSITIVE CONTROL this class has owed all day, and it names the
/// culprit rather than the file: the staging system writes `AmbitionGameSave` and
/// `SaveRestored` from INSIDE the schedule and neither appears in the outside set.
/// What appears is `OccurrenceBaseline`, whose only writer here is
/// `adopt_occurrence_checkpoint_from_save`, in `Update`.
///
/// ⚠ CONTROL, with its confound stated: the same staging system with the latch
/// left ALONE — so the restore chain never fires — reports `Ok(())` and an empty
/// outside set. The confound is that leaving the latch true also lets the
/// in-schedule mirror re-derive the save on the next tick, so the control differs
/// in two ways rather than one. It is enough to attribute the desync to the chain
/// and not to this system's presence in the schedule; it is not enough to say a
/// save write is harmless on its own.
#[test]
#[ignore = "PROBE, print-only: reports what a mid-session load writes outside the rewinding schedule"]
fn probe_what_a_mid_session_load_writes_outside_the_rewinding_schedule() {
    let mut sim = Platformer2dSimHarness::build(
        Platformer2dSimHarnessOptions::default()
            .with_timestep(TimestepMode::fixed_60hz())
            .with_required_start_room(ROOM)
            .with_sync_test_rollback_settings(4, 10),
        |app, options| {
            use ambition_platformer2d::sim::SimScheduleExt;
            ambition_app::rl_sim::ambition_sim_composition(app, options)?;
            app.init_resource::<PeakBaselineRows>();
            app.add_systems(bevy::prelude::Last, record_peak_baseline_rows);
            let label = app.sim_schedule();
            app.add_systems(label, stage_a_mid_session_load_at_tick_40);
            Ok(())
        },
    )
    .expect("the sync-test harness builds with the baseline recorder");
    sim.world_mut()
        .insert_resource(ambition_platformer2d::rollback::RollbackRestoreAudit::enabled());
    for _ in 0..30 {
        sim.step(AgentAction::default());
    }
    let peak_before = sim.world().resource::<PeakBaselineRows>().0;
    let audit_before = {
        let audit = sim
            .world()
            .resource::<ambition_platformer2d::rollback::RollbackRestoreAudit>();
        (
            audit.live_comparisons,
            audit
                .written_outside_the_rewinding_schedule()
                .iter()
                .map(|name| (*name).to_string())
                .collect::<Vec<String>>(),
        )
    };

    // ⛔ THE STAGING CANNOT HAPPEN FROM OUT HERE, and finding that out is half
    // the result. `AmbitionGameSave` is `rollback_resource_clone_checksum` and
    // `SaveRestored` is `rollback_resource_clone`, so a write between two
    // `step()` calls is restored away by the next rollback: measured, the latch
    // never went false and the save's occurrence count never left zero. The
    // staging system above is inside the rewinding schedule, where a
    // resimulation re-applies it — which is exactly the property the writer
    // under investigation lacks.
    for _ in 0..90 {
        sim.step(AgentAction::default());
    }

    let latched = sim.world().resource::<SaveRestored>().0;
    let rows = sim
        .world()
        .resource::<ambition_platformer2d::platformer::lifecycle::OccurrenceBaseline>()
        .remembered()
        .rows()
        .count();
    // ⭐ AND THE OUTSIDE SET HAS A THIRD MEMBER SINCE SCHEMA v195:
    // `AuthoredOccurrences`. It is not a new defect — it is the same `Update`
    // write it always was, newly VISIBLE, because this detector's population is
    // REGISTERED types and a `declare_rollback_derived_*` type was never in it.
    // ⇒ A false derived declaration removes its own subject from the guard named
    // after the problem.
    // ⭐ THE SECOND BASELINE, UNDER THE SAME PRESSURE FOR THE FIRST TIME.
    let custody_rows = sim
        .world()
        .resource::<ambition_platformer2d::platformer::lifecycle::CustodyBaseline>()
        .rows()
        .count();
    let save_rows = sim
        .world()
        .resource::<AmbitionGameSave>()
        .data()
        .occurrences()
        .len();
    let audit = sim
        .world()
        .resource::<ambition_platformer2d::rollback::RollbackRestoreAudit>();
    let moved = audit.types_whose_census_moved_across_compared_frames();
    eprintln!(
        "PROBE mid-session-load latched={latched} baseline_rows={rows} \
         custody_rows={custody_rows} save_rows={save_rows} comparisons {} -> {} \
         outside_before={:?} outside_after={:?} moved={} health={:?}",
        audit_before.0,
        audit.live_comparisons,
        audit_before.1,
        audit.written_outside_the_rewinding_schedule(),
        moved.len(),
        health(&sim),
    );
    let peak_after = sim.world().resource::<PeakBaselineRows>().0;
    eprintln!("PROBE peak baseline rows before={peak_before} after={peak_after}");
    let trace = sim.world().resource::<PeakBaselineRows>().1.clone();
    eprintln!(
        "PROBE (latched, save_rows, baseline_rows, authored_rows) frames 28..42: {:?}",
        &trace[28.min(trace.len())..42.min(trace.len())]
    );
    eprintln!("PROBE moved types: {moved:?}");

    // ⛔ THE PREMISE, ASSERTED, BECAUSE ASSUMING IT IS WHAT PRODUCED THE WRONG
    // ANSWER LAST TIME. A staging system that loads nothing reports an empty
    // outside set and looks exactly like a clean subject. Both halves of the
    // durable horizon must actually arrive before the reading below means
    // anything.
    assert_eq!(
        (rows, custody_rows),
        (1, 1),
        "the staged load must put a row in BOTH baselines, or this probe is \
         reporting on a subject that was never captured (occurrence={rows}, \
         custody={custody_rows})"
    );

    // The defect, stated so that FIXING it reds this arm instead of leaving a
    // stale `#[ignore]`d probe agreeing with whatever the code does. Q135 is
    // the ruling; when it lands, invert this.
    let outside = audit.written_outside_the_rewinding_schedule();
    for owed in [
        "ambition_platformer2d_shared_tangle::lifecycle::continuity::OccurrenceBaseline",
        "ambition_platformer2d_shared_tangle::lifecycle::custody_horizon::CustodyBaseline",
    ] {
        assert!(
            outside.contains(&owed),
            "{owed} is no longer written outside the rewinding schedule \
             (outside={outside:?}) — if that is a FIX, invert this arm and \
             close Q135; if it is a weaker fixture, the probe lost its subject"
        );
    }
}

/// The most rows `OccurrenceBaseline` ever held, sampled every frame — because
/// the adoption is a ONE-FRAME write and the room republishes over it.
#[derive(bevy::prelude::Resource, Default)]
struct PeakBaselineRows(usize, Vec<(bool, usize, usize, usize)>);

fn record_peak_baseline_rows(
    baseline: bevy::prelude::Res<
        ambition_platformer2d::platformer::lifecycle::OccurrenceBaseline,
    >,
    authored: Option<
        bevy::prelude::Res<ambition_platformer2d::platformer::lifecycle::AuthoredOccurrences>,
    >,
    save: bevy::prelude::Res<AmbitionGameSave>,
    latch: bevy::prelude::Res<SaveRestored>,
    mut peak: bevy::prelude::ResMut<PeakBaselineRows>,
) {
    let rows = baseline.remembered().rows().count();
    if rows > peak.0 {
        peak.0 = rows;
    }
    peak.1.push((
        latch.0,
        save.data().occurrences().len(),
        rows,
        authored.map(|a| a.rows().count()).unwrap_or(usize::MAX),
    ));
}

/// Stage the mid-session load from INSIDE the rewinding schedule, so a rollback
/// re-applies it instead of undoing it.
fn stage_a_mid_session_load_at_tick_40(
    tick: bevy::prelude::Res<ambition_platformer2d::time::SimTick>,
    mut save: bevy::prelude::ResMut<AmbitionGameSave>,
    mut restored: bevy::prelude::ResMut<SaveRestored>,
) {
    use ambition_platformer2d::persistence::save_data::{
        PersistedCustody, PersistedOccurrence, PersistedWhereabouts,
    };
    if tick.0 != 40 {
        return;
    }
    // ⛔⛤ **THE CUSTODY LIST USED TO BE `Vec::new()`, AND THAT IS WHY
    // `CustodyBaseline` READ CLEAN.** `adopt_occurrence_checkpoint_from_save`
    // hands BOTH baselines to `adopt_the_ledger`, so an empty custody list makes
    // it write back the same value it read — no census change, and the audit
    // cannot see a write that does not move the value. The page recorded
    // `CustodyBaseline` as clean on exactly that evidence, which was a statement
    // about the PRESSURE and not about the resource. ⇒ Both halves are seeded
    // now, so the two are under the same pressure and a difference between them
    // means something.
    save.data_mut().set_durable_horizon(
        vec![PersistedOccurrence::new(
            "probe:seeded_occurrence",
            PersistedWhereabouts::Placed {
                room: ROOM.to_string(),
                x: 64,
                y: 64,
            },
        )],
        vec![PersistedCustody::new(
            "probe:seeded_occurrence",
            "probe:seeded_custodian",
        )],
    );
    restored.0 = false;
}

/// The dialogue-visit id the two arms below share. Namespaced so a content
/// author's node can never collide with it.
const VISIT_NODE: &str = "probe:visit_counted_from_update";

fn visit_count(sim: &Platformer2dSimHarness) -> u32 {
    sim.world()
        .resource::<AmbitionGameSave>()
        .data()
        .dialog_visit_count(VISIT_NODE)
}

/// Count a visit the way `dispatch_pending_dialog_requests` does — from outside
/// the rewinding schedule, into the hashed save.
fn count_a_visit_from_update(sim: &mut Platformer2dSimHarness) {
    sim.world_mut()
        .resource_mut::<AmbitionGameSave>()
        .data_mut()
        .increment_dialog_visit(VISIT_NODE);
}

/// ⛔⛤ A DIALOGUE VISIT COUNTED FROM `Update` IS TAKEN BACK BY THE REWIND, AND
/// THE PEER CHECKSUM IS NOT THE MECHANISM.
///
/// `dispatch_pending_dialog_requests` (`crates/ambition_dialog/src/bridge.rs`)
/// calls `save.data_mut().increment_dialog_visit(&dialogue_id)` in top-level
/// `Update`, and consumes the request that caused it with
/// `state.pending_start.take()` from a `DialogState` that is on no rollback road.
/// ⇒ A rewind restores the save to its pre-increment value, the request does not
/// come back, and nothing re-runs the dispatcher. **The visit is LOST**, and it
/// is the one save field no tick can re-derive: an increment is neither
/// idempotent nor a function of simulation state.
///
/// ⛔ THIS ARM EXISTS TO NARROW [Q134] TO ONE WAY. That question offers
/// *"stop the checksum covering fields no tick derives"* as a repair. It is not
/// one. `rollback_resource_clone_checksum` installs
/// `rollback_resource_with_clone` and `checksum_resource` INDEPENDENTLY
/// (`crates/ambition_platformer2d_rollback_ggrs/src/registration.rs`, inside
/// `install_resource_clone_checksum`), so narrowing the checksum changes only
/// what two peers compare and leaves the snapshot/restore untouched — and the
/// restore is what loses the visit.
///
/// ⭐ AND THAT IS MEASURED RATHER THAN READ, BY AN ARM THAT HAS BEEN GREEN IN
/// THIS FILE ALL ALONG: `a_bag_changed_from_update_is_silently_taken_back_by_the_rewind`
/// takes back an `Update` write to `OwnedItems`, which is
/// `rollback_resource_clone` — snapshotted and restored, in NO peer checksum at
/// all. A field outside the checksum still loses its `Update` write.
///
/// ⇒ WHEN THIS ARM GOES RED THE DEFECT IS FIXED. Delete it, and close Q134 with
/// whatever made the visit replayable.
///
/// [Q134]: ../../../docs/planning/awaiting-maintainer-decision.md
#[test]
fn a_dialogue_visit_counted_from_update_is_taken_back_by_the_rewind() {
    let mut sim = repro_sim();

    // PREMISE: the latch, for the same reason the bag arm needs it — a world
    // where the save mirrors never run is not the world the dispatcher writes
    // into.
    let mut latched = false;
    for _ in 0..240 {
        sim.step(AgentAction::default());
        if sim
            .world()
            .get_resource::<SaveRestored>()
            .is_some_and(|restored| restored.0)
        {
            latched = true;
            break;
        }
    }
    assert!(
        latched,
        "`SaveRestored` never became true, so the save mirrors early-return \
         forever and nothing below is measuring the dispatcher's world"
    );
    for _ in 0..30 {
        sim.step(AgentAction::default());
    }

    assert_eq!(
        visit_count(&sim),
        0,
        "the probe's node was already visited, so the increment below would not \
         be the first and the loss could be read as a miscount"
    );
    count_a_visit_from_update(&mut sim);
    assert_eq!(
        visit_count(&sim),
        1,
        "the increment did not reach the live save, so nothing below is \
         measuring a change"
    );

    // Sampled every frame rather than compared at the ends, because a value that
    // is right at frame 0 and right at frame N can have been lost in between.
    let mut lost_at: Option<usize> = None;
    for frame in 0..240 {
        sim.step(AgentAction::default());
        if lost_at.is_none() && visit_count(&sim) < 1 {
            lost_at = Some(frame);
        }
    }

    assert!(
        lost_at.is_some(),
        "the rewind no longer takes back a dialogue visit counted from \
         `Update` — that is the FIX this arm is waiting for, not a regression. \
         Delete this arm and close Q134."
    );
    assert_eq!(
        visit_count(&sim),
        0,
        "the visit came back by the end of the window, so the loss is a \
         transient rather than the restore, and the claim above is wrong"
    );
}

/// ⛔ THE CONTROL, AND THE ARM ABOVE IS WORTHLESS WITHOUT IT. "The save lost a
/// visit" and "the REWIND took the visit back" are indistinguishable from inside
/// one harness — the durable restore rewrites this save, and a reset clears it.
/// This runs the SAME increment in the SAME world with no rollback session.
#[test]
fn the_control_keeps_the_same_visit_when_nothing_rewinds() {
    let mut sim = control_sim();
    let mut latched = false;
    for _ in 0..240 {
        sim.step(AgentAction::default());
        if sim
            .world()
            .get_resource::<SaveRestored>()
            .is_some_and(|restored| restored.0)
        {
            latched = true;
            break;
        }
    }
    assert!(
        latched,
        "the control never latched, so it is not the same world as the repro"
    );
    for _ in 0..30 {
        sim.step(AgentAction::default());
    }

    count_a_visit_from_update(&mut sim);
    for frame in 0..240 {
        sim.step(AgentAction::default());
        assert_eq!(
            visit_count(&sim),
            1,
            "frame {frame}: the visit left the save with NO rollback session \
             running, so something other than the rewind clears it and the \
             repro above is not measuring the rewind"
        );
    }
}

/// The ticks the in-schedule counter fires on. Five, not one, so the arm below
/// distinguishes "the restore made it idempotent" from "tick 90 was simulated
/// once".
const VISIT_TICKS: [u64; 5] = [90, 91, 95, 120, 121];

/// Count the visit from inside the rewinding schedule, on five known ticks.
fn count_a_visit_inside_the_tick(
    tick: bevy::prelude::Res<ambition_platformer2d::time::SimTick>,
    mut save: bevy::prelude::ResMut<AmbitionGameSave>,
) {
    if VISIT_TICKS.contains(&tick.0) {
        save.data_mut().increment_dialog_visit(VISIT_NODE);
    }
}

/// ✔⛤ AN INCREMENT INSIDE THE REWINDING SCHEDULE IS IDEMPOTENT, BECAUSE THE
/// RESTORE MAKES IT SO — MEASURED, AND IT IS THE OPPOSITE OF WHAT Q134 ARGUES.
///
/// Q134 reasons that *"an increment is neither idempotent nor derivable"* and
/// prices its option 2 accordingly. **The first half is false where it matters.**
/// A resimulated tick does not add to the value the previous run left: the
/// snapshot restores `AmbitionGameSave` to its state BEFORE the tick, so every
/// replay adds one to the same base and reaches the same total. Non-idempotence
/// only bites a write the snapshot cannot reach — which is precisely where
/// `dispatch_pending_dialog_requests` puts it.
///
/// ⇒ So the repair is the ordinary one and not a research project: move the
/// increment into the rewinding schedule, driven by a fact a replay reproduces.
/// `ambition_conversation::ActiveConversation` is already rollback state
/// (`rollback_resource_clone_entity_set_probed` + `rollback_resource_map_entities`)
/// and already carries a deterministic `ConversationInstanceId` with an
/// `opened_at` tick, so the edge "this instance became live" is replayable.
/// `ambition_dialog` needs no rollback vocabulary; it keeps Yarn.
///
/// ⚠ FIVE TICKS, NOT ONE, and that is the whole strength of this arm. A single
/// tick reaching 1 is also what "the tick ran once, no replay happened" looks
/// like. Five separate ticks reaching exactly 5 says each one's increment landed
/// exactly once across every replay of it.
#[test]
fn an_increment_inside_the_tick_is_made_idempotent_by_the_restore() {
    let mut sim = sim_composed_with(count_a_visit_inside_the_tick);
    // ⛔ THE REWIND IS A PREMISE, NOT AN ASSUMPTION. Five ticks reaching five is
    // ALSO what a harness with no rollback session reports — the control below
    // measures exactly that. So this arm has to witness that frames were
    // actually compared, or its subject was never present.
    sim.world_mut()
        .insert_resource(ambition_platformer2d::rollback::RollbackRestoreAudit::enabled());
    for _ in 0..200 {
        sim.step(AgentAction::default());
    }
    let compared = sim
        .world()
        .resource::<ambition_platformer2d::rollback::RollbackRestoreAudit>()
        .live_comparisons;
    assert!(
        compared > 0,
        "no frame was compared across a restore, so this harness did not rewind          and the count below is the control's measurement wearing the repro's          name"
    );
    assert!(
        sim_tick(&sim) > *VISIT_TICKS.last().expect("five ticks"),
        "the window ended before the last counted tick, so a low count below \
         would mean 'not yet' rather than 'idempotent'"
    );
    assert_eq!(
        visit_count(&sim),
        VISIT_TICKS.len() as u32,
        "an increment inside the rewinding schedule no longer lands exactly once \
         per tick. HIGHER means replays are accumulating and the restore is not \
         reaching this resource — re-read Q134, whose cost argument depends on \
         this. LOWER means a tick in VISIT_TICKS never ran."
    );
}

/// ⛔ THE CONTROL FOR THE ARM ABOVE, and it is the ABSENCE of the rewind rather
/// than a different instance of it. Without a rollback session the same five
/// increments must also reach five — otherwise "the restore made it idempotent"
/// and "these five ticks each fired once, rewind or no rewind" are the same
/// measurement, and the arm above says nothing about the restore.
///
/// ⚠ This is the weaker direction on purpose: agreement here does not prove the
/// restore did anything, it removes the reading under which the restore was
/// never involved. What proves the restore matters is the pair of arms that
/// bracket it — the `Update` write LOSES its increment and the in-schedule write
/// KEEPS exactly one per tick, in the same world.
#[test]
fn the_control_counts_the_same_five_visits_with_no_rewind() {
    use ambition_platformer2d::sim::SimScheduleExt;
    let mut sim = Platformer2dSimHarness::build(
        Platformer2dSimHarnessOptions::default()
            .with_timestep(TimestepMode::fixed_60hz())
            .with_required_start_room(ROOM),
        |app, options| {
            ambition_app::rl_sim::ambition_sim_composition(app, options)?;
            let label = app.sim_schedule();
            app.add_systems(label, count_a_visit_inside_the_tick);
            Ok(())
        },
    )
    .expect("the same world builds without a rollback session");
    for _ in 0..200 {
        sim.step(AgentAction::default());
    }
    assert_eq!(
        visit_count(&sim),
        VISIT_TICKS.len() as u32,
        "the counter did not land five times in a world with NO rewind, so the \
         arm above is measuring the fixture and not the restore"
    );
}

/// The two ticks the acceptance arm opens a conversation on, and the tick it
/// closes the first one. TWO openings, because an arm that expects 1 cannot tell
/// "counted on the opening edge" from "counted once ever".
const OPENS_AT: [u64; 2] = [90, 150];
const CLOSES_AT: u64 = 120;

/// Open and close a conversation on known ticks, from inside the rewinding
/// schedule — standing in for `interact_ecs_actors_and_switches`, which needs a
/// body in reach of an NPC with a compiled Yarn node.
///
/// ⚠ It mints the instance the way production does, at the CURRENT tick, rather
/// than through `LiveConversation::for_test` — that hatch opens at tick zero on
/// purpose, and the opening tick is the entire subject here.
fn open_a_conversation_on_known_ticks(
    tick: bevy::prelude::Res<ambition_platformer2d::time::SimTick>,
    mut conversation: bevy::prelude::ResMut<
        ambition_platformer2d::conversation::ActiveConversation,
    >,
) {
    use ambition_platformer2d::conversation::{
        ActiveConversation, ConversationInputOwner, ConversationInstanceId, LiveConversation,
    };
    let _: &ActiveConversation = &conversation;
    if tick.0 == CLOSES_AT {
        conversation.close();
        return;
    }
    if !OPENS_AT.contains(&tick.0) {
        return;
    }
    conversation.open(LiveConversation {
        instance: ConversationInstanceId::mint(
            tick.0,
            VISIT_NODE,
            None,
            None,
            &ambition_platformer2d::dialog::DialogueContext::scripted(),
        ),
        initiator: None,
        talker: None,
        input_owner: ConversationInputOwner::Primary,
        speaker_name: String::new(),
    });
}

/// ✔⛤ THE ACCEPTANCE FOR THE REPAIR: A CONVERSATION'S VISIT IS COUNTED ONCE PER
/// OPENING, ACROSS A REWOUND WINDOW.
///
/// `count_the_dialogue_visit_when_a_conversation_opens` runs in the sim schedule
/// and fires on `ActiveConversation`'s `opened_at == SimTick`, which is a pure
/// function of rollback state. Two openings of the same node must reach exactly
/// two, with every tick of the window resimulated.
///
/// ⛔ THE COUNT IS THE WHOLE ASSERTION, IN BOTH DIRECTIONS. **0** is the old
/// defect's signature — nothing counted the visit, or it was counted and taken
/// back. **More than 2** is the failure mode the edge exists to prevent: a
/// level rule (`opened_at <= now`) or change detection would fire on every tick
/// the conversation stays live, and a restore marks a rollback-registered
/// resource changed, so `is_changed` fires every frame under GGRS.
#[test]
fn a_conversation_opening_counts_exactly_one_visit_across_a_rewound_window() {
    use ambition_platformer2d::sim::SimScheduleExt;
    let mut sim = Platformer2dSimHarness::build(
        Platformer2dSimHarnessOptions::default()
            .with_timestep(TimestepMode::fixed_60hz())
            .with_required_start_room(ROOM)
            .with_sync_test_rollback_settings(4, 10),
        |app, options| {
            use bevy::prelude::IntoScheduleConfigs as _;
            ambition_app::rl_sim::ambition_sim_composition(app, options)?;
            let label = app.sim_schedule();
            // ⛔⛤ THE SET IS LOAD-BEARING AND COST AN HOUR. Registered anywhere
            // else in this schedule, the stand-in opens the conversation AFTER
            // the counter has already run for that tick — so the counter sees
            // the instance first at `opened_at + 1`, the edge is gone, and the
            // visit is never counted. Measured: the same fixture reports 0
            // visits unset and 2 with this line, with the counter's own
            // `opened_at == now` never once true in the first case.
            // ⇒ `FeatureInteractionSet::Actuate` is where
            // `interact_ecs_actors_and_switches` — the real opener — sits, and
            // the counter's `.after(interact_ecs_actors_and_switches)` edge is
            // the production form of this line.
            app.add_systems(
                label,
                open_a_conversation_on_known_ticks.in_set(
                    ambition_platformer2d::platformer::schedule::FeatureInteractionSet::Actuate,
                ),
            );
            Ok(())
        },
    )
    .expect("the sync-test harness builds with a conversation opener");
    sim.world_mut()
        .insert_resource(ambition_platformer2d::rollback::RollbackRestoreAudit::enabled());
    for _ in 0..240 {
        sim.step(AgentAction::default());
    }
    let compared = sim
        .world()
        .resource::<ambition_platformer2d::rollback::RollbackRestoreAudit>()
        .live_comparisons;
    assert!(
        compared > 0,
        "no frame was compared across a restore, so this arm ran without the \
         rewind it claims to survive"
    );
    assert!(
        sim_tick(&sim) > *OPENS_AT.last().expect("two openings"),
        "the window ended before the second opening, so a count of 1 would mean \
         'not yet' rather than 'counted once'"
    );
    assert!(
        sim.world()
            .get_resource::<SaveRestored>()
            .is_some_and(|restored| restored.0),
        "the latch never flipped, so the counter early-returned for the whole \
         window and a count of 0 would say nothing about the edge"
    );
    assert_eq!(
        visit_count(&sim),
        OPENS_AT.len() as u32,
        "two conversation openings did not produce two visits. 0 means the \
         counter never fired or its write was taken back — the defect this \
         repair closes. MORE than 2 means the edge is not an edge and the \
         counter is firing while the conversation merely stays live."
    );
}


/// Every per-type census this world reported, keyed by the tick it was taken on.
///
/// ⛔ THE POINT IS THE PASS, NOT THE TICK. Under the sync test each frame is
/// simulated several times, so one tick has several entries here. A type whose
/// entries for ONE tick disagree is a type two passes of the same frame computed
/// differently — which is what a checksum mismatch at that frame IS, expressed
/// as a type instead of a frame number.
#[derive(bevy::prelude::Resource, Default)]
struct CensusByPass(
    std::collections::BTreeMap<
        u64,
        Vec<std::collections::BTreeMap<&'static str, (usize, u64)>>,
    >,
    /// `(AuthoredOccurrences rows, save occurrence rows)` per pass.
    std::collections::BTreeMap<u64, Vec<(usize, usize)>>,
);

fn record_the_census_of_every_pass(world: &mut bevy::prelude::World) {
    let Some(tick) = world
        .get_resource::<ambition_platformer2d::time::SimTick>()
        .map(|tick| tick.0)
    else {
        return;
    };
    let probes = world
        .remove_resource::<ambition_platformer2d::rollback::RollbackChecksumProbes>();
    let Some(probes) = probes else {
        return;
    };
    let census: std::collections::BTreeMap<&'static str, (usize, u64)> = probes
        .census_all(world)
        .into_iter()
        .map(|(name, census)| (name, (census.count, census.xor)))
        .collect();
    world.insert_resource(probes);
    // ⭐ AND THE ROWS, RECORDED BESIDE THE CENSUS. These two are different KINDS
    // of evidence about the same resource — a COUNT here, a digest there — so a
    // disagreement between them is informative rather than redundant. (Until the
    // probe was strengthened this read was the only witness, because a
    // presence-only probe on a singleton resource reports `count: 1, xor: 0`
    // however many rows it holds.)
    let authored = world
        .get_resource::<ambition_platformer2d::platformer::lifecycle::AuthoredOccurrences>()
        .map_or(usize::MAX, |authored| authored.rows().count());
    let saved = world
        .get_resource::<AmbitionGameSave>()
        .map_or(usize::MAX, |save| save.data().occurrences().len());
    let mut by_pass = world.remove_resource::<CensusByPass>().unwrap_or_default();
    by_pass.0.entry(tick).or_default().push(census);
    by_pass.1.entry(tick).or_default().push((authored, saved));
    world.insert_resource(by_pass);
}

/// ✔⛤ THE MID-SESSION LOAD NO LONGER REACHES BACKWARDS ACROSS THE REWIND — AND
/// THE LOAD'S LEDGER WRITE IS NOW DETERMINISTICALLY LOST INSTEAD, WHICH IS THE
/// REMAINING HALF OF [Q135].
///
/// ⚠ THIS ARM WAS INVERTED WHEN THE DEFECT CLOSED, and the history matters
/// because the numbers below used to be the other way round. What follows is what
/// it measured while `AuthoredOccurrences` was declared `Derived`:
///
/// Of 364 probed entries, exactly one disagrees between two passes of the same
/// frame outside world construction: `AmbitionGameSave`, at frames 38 and 39 —
/// the frames the sync test names. Neither baseline disagrees, so the
/// `OccurrenceBaseline` path this was first attributed to is not the road.
///
/// ⭐ THE ROAD IS `AuthoredOccurrences`, which is
/// `declare_rollback_derived_resource` — carried in no snapshot, restored by no
/// rewind — on the stated grounds that it is *"republished from live state while
/// its room is loaded"*. `adopt_the_ledger` fills it from the SAVE instead, and
/// nothing republishes it during a rewind, so it keeps the adopted row while the
/// timeline re-simulates frames from BEFORE the load. Then
/// `persist_occurrence_horizon_to_save` mirrors that row into the save's
/// occurrence slice, which is hashed. ⇒ A hashed value derived, inside the
/// rewinding schedule, from a value that does not rewind.
///
/// Measured `(AuthoredOccurrences rows, save occurrence rows)` per pass, load
/// staged at tick 40:
///
/// ```text
/// tick 37   (0,0) (0,0) (0,0) (0,0) (1,0)
/// tick 38   (0,0) (0,0) (0,0) (1,1)
/// tick 39   (0,0) (0,0) (1,1)
/// tick 40   (0,1) (1,1)
/// ```
///
/// ⚠ AND THE LEAK REACHES ONE FRAME FURTHER BACK THAN THE SYNC TEST REPORTS:
/// tick 37's last pass already holds the row. The checksum only notices once the
/// mirror has copied it into the save.
///
/// ⛔⛤ AND THE INSTRUMENT'S BLIND SPOT IS THE SUBJECT ITSELF — measured, after
/// a first reading that got the reason wrong. `AuthoredOccurrences` IS probed
/// and IS one of the 364 entries; its probe is **presence-only**, and a presence
/// probe on a RESOURCE reports `count: 1, xor: 0` however many rows the resource
/// holds. So the census can see the type and can never see this defect. That is
/// the weakness `declare_rollback_derived_component`'s own doc names — *"for a
/// singleton derived resource 'present' is nearly a constant"* — which is why
/// the rows are read directly here, beside the census.
///
/// ⛔⛤ AND THE DECLARED REASON IS FALSE, WHICH THE REGISTRATION DOC PREDICTED IN
/// THESE WORDS: *"a derived declaration that lies is worse than no declaration,
/// because it satisfies the coverage sweep."* It records one such lie already
/// (`ProjectileOwner`, a day of bisection). This is a second:
/// `AuthoredOccurrences` declares *"republished from live state while its room
/// is loaded"*, and `adopt_the_ledger` fills it from a SAVE with no republish to
/// correct it across a rewind. ⚠ `every_presence_only_probe_is_named_with_its_reason`
/// deliberately does not list derived registrations, on the grounds that their
/// reason is declared at the registration site — so the promise is checked for
/// EXISTENCE and never for TRUTH.
///
/// ✅ **AND THAT IS CLOSED.** `AuthoredOccurrences` is
/// `rollback_resource_clone_checksum` now, so the snapshot restores its rows and
/// the leak has nowhere to come from. Measured after: **no entry of the 364
/// disagrees between two passes of one frame outside world construction**, and
/// ticks 37-39 read `(0,0)` on every pass where they used to read `(1,1)` on the
/// later ones.
///
/// ⛔⛤ **WHAT REPLACED IT IS A LOST WRITE, NOT A CLEAN LOAD.**
/// `adopt_occurrence_checkpoint_from_save` runs in top-level `Update`, so now
/// that the ledger rewinds, its write is restored away exactly like any other
/// `Update` write to rollback state: the save holds the row from tick 40 on
/// (`saved = 1`, consistently, in every pass) and the ledger holds none
/// (`authored = 0`, in every pass). ⇒ **A divergence became a deterministic
/// loss.** That is the honest trade and it is an improvement — two peers now
/// agree — but the durable restore does not reach the ledger under a rollback
/// host, and that is [Q135]'s lifecycle half rather than its checksum half.
///
/// ⚠ NO BEHAVIOUR THAT WORKED WAS TAKEN AWAY. This road previously desynced the
/// sync test at the frames of the load, so there was no run in which it worked.
/// A fixed-tick host has no restore and is unaffected.
///
/// ⇒ WHEN THE `authored = 0` HALF CHANGES the lifecycle repair has landed:
/// re-read [Q135] rather than editing the number.
///
/// [Q135]: ../../../docs/planning/awaiting-maintainer-decision.md
#[test]
fn a_mid_session_load_does_not_reach_back_across_the_rewind() {
    use ambition_platformer2d::sim::SimScheduleExt;
    let mut sim = Platformer2dSimHarness::build(
        Platformer2dSimHarnessOptions::default()
            .with_timestep(TimestepMode::fixed_60hz())
            .with_required_start_room(ROOM)
            .with_sync_test_rollback_settings(4, 10),
        |app, options| {
            use bevy::prelude::IntoScheduleConfigs as _;
            ambition_app::rl_sim::ambition_sim_composition(app, options)?;
            let label = app.sim_schedule();
            app.init_resource::<CensusByPass>();
            app.add_systems(
                label,
                (
                    stage_a_mid_session_load_at_tick_40,
                    record_the_census_of_every_pass,
                )
                    .chain(),
            );
            Ok(())
        },
    )
    .expect("the sync-test harness builds with a per-pass census");
    for _ in 0..120 {
        sim.step(AgentAction::default());
    }

    let by_pass = sim.world().resource::<CensusByPass>();
    // A type is a SUSPECT when one tick's passes disagree about it. Report how
    // many ticks each suspect disagreed on, so a per-tick-changing type (which
    // disagrees everywhere) is distinguishable from one that only disagrees at
    // the load.
    let mut disagreed: std::collections::BTreeMap<&'static str, Vec<u64>> =
        std::collections::BTreeMap::new();
    for (tick, passes) in &by_pass.0 {
        if passes.len() < 2 {
            continue;
        }
        for (name, first) in &passes[0] {
            if passes[1..]
                .iter()
                .any(|later| later.get(name) != Some(first))
            {
                disagreed.entry(name).or_default().push(*tick);
            }
        }
    }
    let ticks_with_passes = by_pass.0.values().filter(|p| p.len() >= 2).count();
    let entries = by_pass.0.values().next().map_or(0, |p| p[0].len());
    eprintln!(
        "PASSES ticks={} ticks-with-2+-passes={ticks_with_passes} entries={entries}",
        by_pass.0.len(),
    );
    for (name, ticks) in &disagreed {
        eprintln!(
            "PASSES disagreed on {} tick(s): {name} at {:?}",
            ticks.len(),
            &ticks[..ticks.len().min(6)]
        );
    }
    for tick in [37u64, 38, 39, 40, 41] {
        eprintln!(
            "PASSES tick {tick}: (authored, saved) per pass = {:?}",
            by_pass.1.get(&tick)
        );
    }

    // ⛔ THE ARM'S OWN PREMISE ABOUT ITS INSTRUMENT, ASSERTED. Reading the rows
    // directly is only justified while the census cannot see them; if the probe
    // is ever strengthened, the census becomes the better witness and the
    // reasoning above needs rewriting rather than re-running.
    {
        const SUBJECT: &str =
            "ambition_platformer2d_shared_tangle::lifecycle::continuity::AuthoredOccurrences";
        let probes = sim
            .world()
            .resource::<ambition_platformer2d::rollback::RollbackChecksumProbes>();
        assert!(
            probes.type_names().contains(&SUBJECT),
            "the subject left the probe set entirely, so 'the census cannot see \
             this defect' is now true for a different reason than the one stated"
        );
        assert!(
            !probes.presence_only_type_names().contains(&SUBJECT),
            "the subject's probe went back to presence-only, which on a \
             singleton resource reports `count: 1, xor: 0` however many rows it \
             holds — so the census below can no longer see the cause and this \
             arm is reporting on its own blindness"
        );
    }

    // ── PREMISES, because every number below is worthless without them.
    assert!(
        ticks_with_passes >= 20,
        "only {ticks_with_passes} tick(s) were simulated more than once, so \
         there is barely a replay here to disagree with"
    );
    assert!(
        entries > 300,
        "the probe set collapsed to {entries} entries; a small population \
         reports few suspects for a reason that is not agreement"
    );

    // ── PREMISE: the load actually happened. `saved` must reach the row, or
    // the agreement below is agreement about a load that never ran.
    let load_landed = by_pass
        .1
        .iter()
        .any(|(tick, passes)| *tick >= 40 && passes.iter().any(|(_, saved)| *saved > 0));
    assert!(
        load_landed,
        "the staged load never reached the save, so this arm is reporting on a \
         world where nothing loaded"
    );

    // ── SUBJECT ONE: no frame's passes disagree about the ledger any more.
    let carried_back: Vec<u64> = [37u64, 38, 39]
        .into_iter()
        .filter(|tick| {
            by_pass.1.get(tick).is_some_and(|passes| {
                passes.iter().any(|(authored, _)| *authored == 0)
                    && passes.iter().any(|(authored, _)| *authored > 0)
            })
        })
        .collect();
    assert!(
        carried_back.is_empty(),
        "`AuthoredOccurrences` disagrees between passes of ticks \
         {carried_back:?}, which are frames from BEFORE the load — so the row is \
         reaching backwards across the rewind again. The registration is what \
         stops that: `rollback_resource_clone_checksum`, not \
         `declare_rollback_derived_*`."
    );

    // ── SUBJECT TWO: and the load's ledger write is LOST, which is the state
    // this arm now documents rather than the divergence.
    let ledger_after_the_load: Vec<usize> = by_pass
        .1
        .iter()
        .filter(|(tick, _)| **tick >= 40)
        .flat_map(|(_, passes)| passes.iter().map(|(authored, _)| *authored))
        .collect();
    assert!(
        ledger_after_the_load.iter().all(|rows| *rows == 0),
        "the ledger now holds rows after the load ({}). If \
         `adopt_occurrence_checkpoint_from_save` moved inside the rewinding \
         schedule, or session admission now hydrates before GGRS starts, that is \
         Q135's lifecycle half LANDING — re-read it and invert this half rather \
         than editing the number.",
        ledger_after_the_load
            .iter()
            .filter(|rows| **rows > 0)
            .count()
    );

    // ── SUBJECT THREE, AND THE WIDEST: NOTHING at all disagrees between two
    // passes of one frame, over the whole probed set, outside world
    // construction. This is the assertion the repair earns; while the ledger was
    // `Derived` the set was exactly `{AmbitionGameSave, AuthoredOccurrences}`,
    // with the ledger's first disagreement one frame AHEAD of the save's because
    // the chain is ledger -> mirror -> hashed save.
    //
    // ⚠ TICK 0 IS EXCLUDED because the world is being built during its passes,
    // which is not a replay disagreement about simulation.
    let outside_construction: Vec<(&&str, &Vec<u64>)> = disagreed
        .iter()
        .filter(|(_, ticks)| ticks.iter().any(|tick| *tick > 0))
        .map(|(name, ticks)| (name, ticks))
        .collect();
    assert!(
        outside_construction.is_empty(),
        "{} probed entr(ies) disagree between two passes of one frame: {:?}. A \
         member here is a value two passes of the same simulated frame computed \
         differently, which is what a peer checksum mismatch IS.",
        outside_construction.len(),
        &outside_construction[..outside_construction.len().min(6)]
    );
    // ── AND THE SYNC TEST ITSELF, which is a different KIND of evidence than
    // the per-pass census: GGRS's own comparison rather than this arm's.
    health(&sim).unwrap_or_else(|error| {
        panic!(
            "the per-pass census found no disagreement and the sync test \
             disagrees anyway, so something diverges that no probe covers. The \
             probed set is a FLOOR on what is compared, not the whole of \
             it.\n{error}"
        )
    });
}


const LEDGER_ROW_AT: u64 = 5;
const LEDGER_OCCURRENCE: &str = "probe:an_object_the_world_remembers";

/// Write ONE row into the occurrence ledger, once, from inside the rewinding
/// schedule — the minimal shape of what New Game already does.
///
/// ⚠ ONCE, NOT EVERY TICK. A system that rewrites the row each tick IS the live
/// producer `AuthoredOccurrences`'s derived declaration describes, and it would
/// re-establish the row on every replay and hide the defect.
fn leave_one_row_in_the_occurrence_ledger(
    tick: bevy::prelude::Res<ambition_platformer2d::time::SimTick>,
    mut occurrences: bevy::prelude::ResMut<
        ambition_platformer2d::platformer::lifecycle::AuthoredOccurrences,
    >,
) {
    use ambition_platformer2d::platformer::lifecycle::OccurrenceWhereabouts;
    if tick.0 != LEDGER_ROW_AT {
        return;
    }
    let mut rows = std::collections::BTreeMap::new();
    rows.insert(
        ambition_platformer2d::platformer::sim_id::SimId::from_snapshot(
            LEDGER_OCCURRENCE.to_string(),
        ),
        OccurrenceWhereabouts::Placed {
            room: ROOM.to_string(),
            at: bevy::prelude::Vec2::new(32.0, 48.0),
        },
    );
    occurrences.adopt_rows(rows);
}

/// ⛔⛤ ONE WRITE TO THE OCCURRENCE LEDGER FROM INSIDE THE SIMULATION DESYNCS THE
/// SYNC TEST WITHIN TWO TICKS. `AuthoredOccurrences` IS AUTHORITATIVE MUTABLE
/// SIMULATION STATE AND ITS `Derived` DECLARATION IS FALSE.
///
/// ⭐ THIS NEEDS NO SAVE FILE, NO LOAD AND NO NEW GAME, which is what makes it
/// the right witness. The mid-session-load arm above reaches the same defect
/// through `adopt_occurrence_checkpoint_from_save`, a road whose production
/// customer is not established, and that invited the reading that the defect is
/// a property of an unsupported feature. It is not: a single `adopt_rows` call
/// in the rewinding schedule is enough.
///
/// ⛔ AND THE SHIPPED INSTANCE OF EXACTLY THIS WRITE IS NEW GAME.
/// `process_new_game_reset_request` is registered in this same schedule
/// (`Platformer2dSimulationPhaseMonolith::ResetProcessing`,
/// `crates/ambition_platformer2d_actor_monolith/src/session/reset/mod.rs`) and
/// calls `occurrences.forget_everything()`. A rewind across that clear cannot
/// restore what it cleared, and for a row naming an unloaded room there is no
/// live producer that could republish it — so the *"republished from live state
/// while its room is loaded"* justification has nothing to offer.
///
/// ⚠ MEASURED BEHAVIOUR OF THE DEFECT, 2026-09-16: with the row seeded the
/// timeline stalls at `SimTick` 7 with a checksum mismatch; with the identical
/// fixture and no row it reaches 31. **A row in this ledger is not inert.**
///
/// ⚠ THIS ARM ASSERTS THE REPAIRED BEHAVIOUR AND FAILS UNTIL
/// `AuthoredOccurrences` BECOMES REAL ROLLBACK STATE. The defect already has a
/// witness; what this owes is the acceptance.
#[test]
fn one_write_to_the_occurrence_ledger_does_not_desync_the_sync_test() {
    use ambition_platformer2d::sim::SimScheduleExt;
    let mut sim = Platformer2dSimHarness::build(
        Platformer2dSimHarnessOptions::default()
            .with_timestep(TimestepMode::fixed_60hz())
            .with_required_start_room(ROOM)
            .with_sync_test_rollback_settings(4, 10),
        |app, options| {
            ambition_app::rl_sim::ambition_sim_composition(app, options)?;
            let label = app.sim_schedule();
            app.add_systems(label, leave_one_row_in_the_occurrence_ledger);
            Ok(())
        },
    )
    .expect("the sync-test harness builds with one ledger write");
    for _ in 0..140 {
        sim.step(AgentAction::default());
    }

    let rows = sim
        .world()
        .resource::<ambition_platformer2d::platformer::lifecycle::AuthoredOccurrences>()
        .rows()
        .count();
    let reached = sim_tick(&sim);
    eprintln!("LEDGER rows={rows} SimTick={reached} health={:?}", health(&sim));

    // ── PREMISE: the write happened. Without it a clean window says nothing.
    assert_eq!(
        rows, 1,
        "the seeded row is not in the ledger, so this arm is measuring a world \
         where nothing wrote to it"
    );

    // ── THE SUBJECT.
    health(&sim).unwrap_or_else(|error| {
        panic!(
            "one `adopt_rows` call inside the rewinding schedule desynced the \
             sync test. `AuthoredOccurrences` is `declare_rollback_derived_*`, so \
             no snapshot restores it: the row survives a rewind into frames from \
             before the write, `persist_occurrence_horizon_to_save` mirrors it \
             into the hashed save, and two passes of one frame disagree. New \
             Game does this same write on a shipped road via \
             `forget_everything()`.\n\
             SimTick reached {reached} (an unseeded run of this fixture reaches \
             31)\n{error}"
        )
    });
    assert!(
        reached > 25,
        "the timeline stalled at tick {reached}; an unseeded run of this fixture \
         reaches 31, so the write is holding the session back even if the \
         checksum happens to agree"
    );
}

const NEW_GAME_AT: u64 = 30;

/// Every value `SaveRestored` held, sampled once per `Update`.
///
/// ⛔⛤ THIS EXISTS BECAUSE A BETWEEN-STEP READ CANNOT SEE THE TRANSITION.
/// `reset_inventory_on_new_game` lowered the latch and the load chain raised it
/// again in the SAME frame, so `sim.step()`-boundary sampling reported it true
/// throughout — measured: the first version of the arm below passed with the
/// defect fully present, which is a vacuous guard wearing a green tick.
#[derive(bevy::prelude::Resource, Default)]
struct LatchHistory(Vec<bool>);

fn record_the_latch_every_frame(
    restored: bevy::prelude::Res<SaveRestored>,
    mut history: bevy::prelude::ResMut<LatchHistory>,
) {
    history.0.push(restored.0);
}

/// Put a row in each occurrence baseline, then commit New Game — all from
/// inside the rewinding schedule, which is where the shipped reset lives.
fn seed_both_baselines_then_start_a_new_game(
    tick: bevy::prelude::Res<ambition_platformer2d::time::SimTick>,
    mut occurrence_baseline: bevy::prelude::ResMut<
        ambition_platformer2d::platformer::lifecycle::OccurrenceBaseline,
    >,
    mut custody_baseline: bevy::prelude::ResMut<
        ambition_platformer2d::platformer::lifecycle::CustodyBaseline,
    >,
    mut reset: bevy::prelude::ResMut<
        ambition_platformer2d::actors::session::reset::NewGameResetRequested,
    >,
) {
    use ambition_platformer2d::platformer::lifecycle::{
        AuthoredOccurrences, OccurrenceWhereabouts,
    };
    use ambition_platformer2d::platformer::sim_id::SimId;
    if tick.0 == 5 {
        let subject = SimId::from_snapshot("probe:a_thing_the_checkpoint_remembers".to_string());
        let mut rows = std::collections::BTreeMap::new();
        rows.insert(subject.clone(), OccurrenceWhereabouts::InCustody);
        let mut remembered = AuthoredOccurrences::default();
        remembered.adopt_rows(rows);
        occurrence_baseline.adopt(remembered);
        let mut held = std::collections::BTreeMap::new();
        held.insert(
            subject,
            SimId::from_snapshot("probe:a_custodian".to_string()),
        );
        custody_baseline.adopt(held);
    } else if tick.0 == NEW_GAME_AT {
        reset.request = true;
    }
}

/// ✔⛤ NEW GAME CLEARS THE OCCURRENCE BASELINES ITSELF, AND NEVER LOWERS THE
/// DURABLE-RESTORE LATCH TO DO IT.
///
/// `reset_inventory_on_new_game` used to end `restored.0 = false`, which sent
/// the fresh run back through the generic load chain so that
/// `adopt_occurrence_checkpoint_from_save` would re-adopt these two baselines
/// from the wiped file. ⛔ **That line was the ONLY place in the codebase that
/// lowered `SaveRestored`** — measured by census — so it was single-handedly
/// creating a "replace the save while a GGRS session is live" road that no
/// product feature asked for, and running a chain of `Update` systems over
/// rollback-owned state to do it.
///
/// ⇒ Every other fresh-run durable fact was already reset in that same function
/// (bag, wallet, minted-item baseline, owned-items baseline); these two were the
/// whole reason to re-enter the load road. They are reset directly now.
///
/// ⚠ `AuthoredOccurrences` is not checked here: `process_new_game_reset_request`
/// clears it with `forget_everything()` one system earlier in the same committed
/// transaction, and `one_write_to_the_occurrence_ledger_does_not_desync_the_sync_test`
/// owns that road.
#[test]
fn a_new_game_clears_the_occurrence_baselines_without_lowering_the_latch() {
    use ambition_platformer2d::sim::SimScheduleExt;
    let mut sim = Platformer2dSimHarness::build(
        Platformer2dSimHarnessOptions::default()
            .with_timestep(TimestepMode::fixed_60hz())
            .with_required_start_room(ROOM)
            .with_sync_test_rollback_settings(4, 10),
        |app, options| {
            use bevy::prelude::IntoScheduleConfigs as _;
            ambition_app::rl_sim::ambition_sim_composition(app, options)?;
            let label = app.sim_schedule();
            app.add_systems(label, seed_both_baselines_then_start_a_new_game);
            // ⚠ `Update`, not the sim schedule: the chain that lowers and raises
            // the latch lives in `Update`, and that is the resolution the claim
            // needs.
            app.init_resource::<LatchHistory>();
            // ⛔⛤ AND `.before` THE RESTORE CHAIN, WHICH IS THE THIRD TIME THIS
            // ARM'S SHUTTER OPENED AT THE WRONG MOMENT. Unordered in `Update`
            // the recorder lands AFTER `complete_durable_restore` has raised the
            // latch again, so it reports `true` on every frame with the defect
            // fully present. The window where the latch is false is: the sim
            // schedule lowers it (from `PreUpdate`, under the rollback host) and
            // this same frame's `Update` chain raises it. Sampling has to sit at
            // the head of that chain.
            app.add_systems(
                bevy::prelude::Update,
                record_the_latch_every_frame.before(
                    ambition_platformer2d::actors::session::durable_horizon::adopt_occurrence_checkpoint_from_save,
                ),
            );
            Ok(())
        },
    )
    .expect("the sync-test harness builds with a New Game on a known tick");

    // Sampled EVERY step, because the claim is that the latch never goes false —
    // and a value that is true at the start and true at the end can have been
    // false in between. That is exactly how the old road worked.
    let mut latch_lowered_at: Vec<usize> = Vec::new();
    let mut baselines_seeded = false;
    // ⛔ AND THE ROSTER IS WHAT PROVES THE RESET COMMITTED, not `SimTick`. A
    // committed reset writes a `ClockResetRequest`, so the tick counter is not a
    // measure of window progress across one: measured, a 140-step run and a
    // 280-step run both report tick 32 against a reset staged at 30. The reset
    // despawns and respawns the room's roster, and despawn bumps the entity
    // generation, so NO original `Entity` value survives a rebuild that really
    // ran — the discriminator `a_reset_requested_from_update_mid_window` uses.
    let roster_before = feature_roster(&mut sim);
    for frame in 0..200 {
        sim.step(AgentAction::default());
        if !sim
            .world()
            .get_resource::<SaveRestored>()
            .is_some_and(|restored| restored.0)
        {
            // Kept, but it is the WEAK reading — see `LatchHistory`.
            latch_lowered_at.push(frame);
        }
        if !baselines_seeded
            && sim
                .world()
                .resource::<ambition_platformer2d::platformer::lifecycle::OccurrenceBaseline>()
                .remembered()
                .rows()
                .count()
                > 0
        {
            baselines_seeded = true;
        }
    }

    let occurrence_rows = sim
        .world()
        .resource::<ambition_platformer2d::platformer::lifecycle::OccurrenceBaseline>()
        .remembered()
        .rows()
        .count();
    let custody_rows = sim
        .world()
        .resource::<ambition_platformer2d::platformer::lifecycle::CustodyBaseline>()
        .rows()
        .count();
    let roster_after = feature_roster(&mut sim);
    let survivors = roster_before.intersection(&roster_after).count();
    eprintln!(
        "NEWGAME tick={} seeded={baselines_seeded} occurrence_rows={occurrence_rows} \
         custody_rows={custody_rows} latch_lowered_at={latch_lowered_at:?} \
         roster {} -> {} survivors={survivors}",
        sim_tick(&sim),
        roster_before.len(),
        roster_after.len(),
    );

    // ── PREMISES. The window has to reach the reset, and the baselines have to
    // have held something, or "they are empty" is a statement about a seed that
    // never landed.
    assert!(
        !roster_before.is_empty(),
        "the room had no roster before the reset, so 'the roster was rebuilt' \
         below is a claim about an empty set"
    );
    assert_eq!(
        survivors, 0,
        "{survivors} of {} roster entities survived, so the New Game reset did \
         not rebuild the world and nothing below is about a committed reset",
        roster_before.len()
    );
    assert!(
        baselines_seeded,
        "the occurrence baseline never held a row, so 'New Game cleared it' is a \
         claim about an empty subject"
    );

    // ── THE SUBJECT, both halves.
    assert_eq!(
        (occurrence_rows, custody_rows),
        (0, 0),
        "New Game left rows in the occurrence baselines, so the fresh run \
         inherits what the finished run remembered at its last checkpoint"
    );
    // ── AND THE LATCH, AT THE RESOLUTION THE CLAIM NEEDS.
    let history = &sim.world().resource::<LatchHistory>().0;
    let first_true = history.iter().position(|held| *held);
    assert!(
        first_true.is_some(),
        "the latch never became true in {} sampled frames, so this arm never \
         entered the state it is about",
        history.len()
    );
    let lowered_after: Vec<usize> = history
        .iter()
        .enumerate()
        .skip(first_true.expect("asserted just above"))
        .filter(|(_, held)| !**held)
        .map(|(index, _)| index)
        .collect();
    assert!(
        lowered_after.is_empty(),
        "`SaveRestored` went false again at sampled frames {:?} of {} after \
         first becoming true. That is the mid-session save-replacement road: it \
         re-runs a chain of `Update` systems over rollback-owned state while a \
         GGRS session is already live. New Game must RESET its durable domains, \
         not RELOAD them.\n\
         ⚠ The between-step reading says {latch_lowered_at:?}, and it is blind \
         to a lower-and-raise inside one frame — which is exactly what the old \
         road did.",
        &lowered_after[..lowered_after.len().min(8)],
        history.len()
    );
}

/// Every `(SimTick, SaveRestored)` pair this world simulated, in execution
/// order, one row per PASS.
///
/// ⛔ READ FROM INSIDE THE SIMULATION SCHEDULE, which is the whole point.
/// `probe_when_the_durable_restore_latch_flips_against_ggrs_start` samples
/// between `sim.step()` calls and therefore cannot see a latch that is false
/// only WHILE the schedule runs — the error that made the New Game window look
/// two frames wide and closed.
#[derive(bevy::prelude::Resource, Default)]
struct LatchInsideTheSchedule(Vec<(u64, u64, bool)>);

/// The host frame this world is on, counted OUTSIDE the simulation schedule.
///
/// ⭐ IT IS THE JOIN BETWEEN THE TWO INSTRUMENTS. A between-frame probe reports
/// "frame 1: ggrs live, restored false"; an in-schedule probe reports "tick 0:
/// restored true". Neither row can refute the other without a shared column,
/// and this is it.
#[derive(bevy::prelude::Resource, Default)]
struct HostFrame(u64);

fn count_the_host_frame(mut frame: bevy::prelude::ResMut<HostFrame>) {
    frame.0 += 1;
}

fn record_the_latch_from_inside_the_schedule(
    tick: bevy::prelude::Res<ambition_platformer2d::time::SimTick>,
    frame: bevy::prelude::Res<HostFrame>,
    restored: Option<bevy::prelude::Res<SaveRestored>>,
    mut log: bevy::prelude::ResMut<LatchInsideTheSchedule>,
) {
    log.0
        .push((frame.0, tick.0, restored.is_some_and(|restored| restored.0)));
}

/// Build the in-schedule latch recorder, optionally alongside the within-frame
/// sampler whose mere presence
/// [`probe_when_the_durable_restore_latch_flips_against_ggrs_start`] measured as
/// moving the GGRS start by a frame.
///
/// ⭐ THE SAMPLER IS THE VARIABLE, and it is installed or not installed in
/// worlds that are otherwise identical — including the recorder. The earlier
/// probe compared a world WITH the sampler against a world with NOTHING added,
/// so "the sampler moved it" and "adding any Update system moves it" were the
/// same measurement.
fn sim_recording_the_latch_inside_the_schedule(
    with_the_within_frame_sampler: bool,
) -> Platformer2dSimHarness {
    use ambition_platformer2d::sim::SimScheduleExt;
    Platformer2dSimHarness::build(
        Platformer2dSimHarnessOptions::default()
            .with_timestep(TimestepMode::fixed_60hz())
            .with_required_start_room(ROOM)
            .with_sync_test_rollback_settings(4, 10),
        |app, options| {
            use bevy::prelude::IntoScheduleConfigs as _;
            ambition_app::rl_sim::ambition_sim_composition(app, options)?;
            let label = app.sim_schedule();
            app.init_resource::<LatchInsideTheSchedule>();
            app.init_resource::<HostFrame>();
            app.add_systems(bevy::prelude::Last, count_the_host_frame);
            app.add_systems(
                label,
                record_the_latch_from_inside_the_schedule.in_set(
                    ambition_platformer2d::platformer::schedule::FeatureInteractionSet::Actuate,
                ),
            );
            if with_the_within_frame_sampler {
                app.init_resource::<WithinFrameOrder>();
                app.add_systems(
                    bevy::prelude::Update,
                    sample_ggrs_after_the_latch.after(
                        ambition_platformer2d::actors::session::durable_horizon::complete_durable_restore,
                    ),
                );
            }
            Ok(())
        },
    )
    .expect("the sync-test harness builds with an in-schedule latch recorder")
}

/// Open a conversation on the FIRST tick the simulation ever runs, and never
/// again.
///
/// ⚠ TICK 0 IS NOT AN ARBITRARY EARLY TICK, it is the one the measurement
/// named: with the perturbing sampler installed, tick 0 is the only tick ever
/// simulated while `SaveRestored` is false. A conversation cannot open earlier
/// than the first tick, so this is the earliest openable moment of a session —
/// unreachable by a human hand on the pad, reachable by a scripted or
/// autostarted conversation, and reachable by any content that opens dialogue
/// on room entry.
fn open_a_conversation_on_the_very_first_tick(
    tick: bevy::prelude::Res<ambition_platformer2d::time::SimTick>,
    mut conversation: bevy::prelude::ResMut<
        ambition_platformer2d::conversation::ActiveConversation,
    >,
) {
    use ambition_platformer2d::conversation::{
        ConversationInputOwner, ConversationInstanceId, LiveConversation,
    };
    if tick.0 != 0 {
        return;
    }
    conversation.open(LiveConversation {
        instance: ConversationInstanceId::mint(
            tick.0,
            VISIT_NODE,
            None,
            None,
            &ambition_platformer2d::dialog::DialogueContext::scripted(),
        ),
        initiator: None,
        talker: None,
        input_owner: ConversationInputOwner::Primary,
        speaker_name: String::new(),
    });
}

fn sim_opening_a_conversation_on_the_first_tick(
    with_the_within_frame_sampler: bool,
) -> Platformer2dSimHarness {
    use ambition_platformer2d::sim::SimScheduleExt;
    Platformer2dSimHarness::build(
        Platformer2dSimHarnessOptions::default()
            .with_timestep(TimestepMode::fixed_60hz())
            .with_required_start_room(ROOM)
            .with_sync_test_rollback_settings(4, 10),
        |app, options| {
            use bevy::prelude::IntoScheduleConfigs as _;
            ambition_app::rl_sim::ambition_sim_composition(app, options)?;
            let label = app.sim_schedule();
            app.init_resource::<LatchInsideTheSchedule>();
            app.init_resource::<HostFrame>();
            app.add_systems(bevy::prelude::Last, count_the_host_frame);
            app.add_systems(
                label,
                (
                    record_the_latch_from_inside_the_schedule,
                    open_a_conversation_on_the_very_first_tick,
                )
                    .in_set(
                        ambition_platformer2d::platformer::schedule::FeatureInteractionSet::Actuate,
                    ),
            );
            if with_the_within_frame_sampler {
                app.init_resource::<WithinFrameOrder>();
                app.add_systems(
                    bevy::prelude::Update,
                    sample_ggrs_after_the_latch.after(
                        ambition_platformer2d::actors::session::durable_horizon::complete_durable_restore,
                    ),
                );
            }
            Ok(())
        },
    )
    .expect("the sync-test harness builds with a first-tick conversation opener")
}

/// The ticks this world simulated before the durable-restore latch rose.
///
/// ⚠ TICKS, NOT HOST FRAMES. Which host frame the boot lands on is exactly what
/// [`probe_when_the_durable_restore_latch_flips_against_ggrs_start`] measured as
/// moving under unrelated composition changes, so asserting on it would make
/// this arm fail for the reason it is trying to REPORT. The tick number is the
/// stable fact: tick 0 is the first tick whatever frame it runs on.
fn ticks_simulated_unrestored(sim: &Platformer2dSimHarness) -> Vec<u64> {
    sim.world()
        .resource::<LatchInsideTheSchedule>()
        .0
        .iter()
        .filter(|(_, _, restored)| !restored)
        .map(|(_, tick, _)| *tick)
        .collect()
}

/// ✔⛤ **A CONVERSATION OPENED ON THE FIRST TICK OF A SESSION IS COUNTED ONCE,
/// AND NO LONGER DEPENDS ON UNRELATED `Update` MEMBERSHIP.**
///
/// The two arms are the same world with the same opener, the same in-schedule
/// recorder and the same tick. The only difference is whether one unrelated
/// `Update` system — this file's own within-frame sampler — is installed. That
/// is the variable on purpose: before the repair it DECIDED the answer.
///
/// ```text
///                   before the gate                     after
/// sampler absent    tick 0 simulated, latch TRUE  -> 1   unchanged -> 1
/// sampler present   tick 0 simulated, latch FALSE -> 0   no unrestored tick -> 1
/// ```
///
/// `maintain_local_session` now refuses to CREATE a GGRS session over a world
/// whose durable state is still loading, so no tick is simulated before
/// `SaveRestored` rises in either composition and
/// `count_the_dialogue_visit_when_a_conversation_opens` never has to drop a
/// write it has nowhere to put. The counter's `if !restored.0` guard is
/// untouched and stays correct; what changed is that it is no longer reachable
/// with a live conversation behind it.
///
/// ⭐ THE CONTROL IS THE ABSENCE OF THE PERTURBATION, not a different
/// conversation. An arm reporting a number alone could mean the stand-in never
/// opened anything; the pair rules that out with the same code, and the
/// `unrestored` assertions below rule out the arms having become the same
/// condition measured twice.
///
/// ⚠ **THIS ARM IS THE GATE'S ONLY WITNESS, AND ITS POWER IS IN THE SECOND
/// ASSERTION.** Deleting the gate makes `ticks_simulated_unrestored(&with)`
/// read `[0]` and that arm's visit count read 0 — measured, both. Do not
/// "repair" a future failure here by relaxing the counter's edge to
/// `opened_at <= tick`: that turns an edge into a level and over-counts every
/// tick a conversation stays live, which
/// `a_dialogue_visit_counted_from_update_is_taken_back_by_the_rewind`'s
/// more-than-two case pins.
#[test]
fn a_conversation_on_the_first_tick_of_a_session_is_counted_exactly_once() {
    let mut without = sim_opening_a_conversation_on_the_first_tick(false);
    for _ in 0..12 {
        without.step(AgentAction::default());
    }
    let mut with = sim_opening_a_conversation_on_the_first_tick(true);
    for _ in 0..12 {
        with.step(AgentAction::default());
    }

    for (name, sim) in [("without", &without), ("with", &with)] {
        // PREMISE: the world simulated tick 0 at all. A visit count from a world
        // whose schedule never ran says nothing about the counter — and it is
        // the failure this repair could plausibly CAUSE, since the gate can
        // refuse to start a session.
        assert!(
            sim.world()
                .resource::<LatchInsideTheSchedule>()
                .0
                .iter()
                .any(|(_, tick, _)| *tick == 0),
            "the {name}-sampler world never simulated tick 0. If the durable \
             hydration gate is now refusing to start a session at all, this is \
             where that shows up — a world that never simulates is not a world \
             that never loses a visit"
        );
        assert_eq!(
            ticks_simulated_unrestored(sim),
            Vec::<u64>::new(),
            "the {name}-sampler world simulated ticks before the durable restore \
             completed. That is the defect the session-start gate in \
             `maintain_local_session` closes, and a non-empty list here means \
             the timeline is starting over an unhydrated world again"
        );
        assert_eq!(
            visit_count(sim),
            1,
            "a conversation opened on tick 0 of the {name}-sampler world was not \
             counted exactly once. 0 is the pre-repair signature: the visit \
             reached a counter that was still gated on the latch. More than 1 \
             means the opening edge has become a level"
        );
    }
}

/// Is any simulation tick ever SIMULATED while the durable-restore latch is
/// still false?
///
/// ⛔ THE BETWEEN-FRAME PROBE CANNOT ANSWER THIS, and the distinction is the
/// finding. It reports the frame GGRS becomes LIVE against the frame the latch
/// rises; liveness is not advance. A session can exist for a frame without
/// stepping the timeline, and a frame of that shape carries no tick for a
/// conversation to open on.
#[test]
#[ignore = "probe: prints the latch as the simulation itself sees it, with and without the perturbing sampler"]
fn probe_the_latch_as_the_opener_sees_it() {
    for with_sampler in [false, true] {
        let mut sim = sim_recording_the_latch_inside_the_schedule(with_sampler);
        for _ in 0..12 {
            sim.step(AgentAction::default());
        }
        let log = &sim.world().resource::<LatchInsideTheSchedule>().0;
        eprintln!(
            "PROBE with_within_frame_sampler={with_sampler} passes={} first={:?} \
             unrestored_ticks={:?}",
            log.len(),
            log.first(),
            log.iter()
                .filter(|(_, _, restored)| !restored)
                .map(|(frame, tick, _)| (*frame, *tick))
                .collect::<Vec<_>>(),
        );
        for (index, (frame, tick, restored)) in log.iter().take(8).enumerate() {
            eprintln!("    pass {index:>3}: host frame {frame:>3}  tick {tick:>3}  restored {restored}");
        }
    }
}

/// Every commit of a New Game this world performed, by the `SimTick` it
/// committed on, one entry per PASS.
#[derive(bevy::prelude::Resource, Default)]
struct ResetCommits(Vec<u64>);

fn record_every_new_game_commit(
    mut committed: bevy::prelude::MessageReader<
        ambition_platformer2d::actors::session::reset::NewGameResetCommitted,
    >,
    tick: bevy::prelude::Res<ambition_platformer2d::time::SimTick>,
    mut commits: bevy::prelude::ResMut<ResetCommits>,
) {
    for _ in committed.read() {
        commits.0.push(tick.0);
    }
}

/// The tick the in-sim control asks for its New Game on — comfortably after
/// durable hydration, so neither arm is racing the latch.
const IN_SIM_NEW_GAME_AT: u64 = 30;

fn request_a_new_game_from_inside_the_sim(
    tick: bevy::prelude::Res<ambition_platformer2d::time::SimTick>,
    mut reset: bevy::prelude::ResMut<
        ambition_platformer2d::actors::session::reset::NewGameResetRequested,
    >,
) {
    if tick.0 == IN_SIM_NEW_GAME_AT {
        reset.request = true;
    }
}

fn sim_recording_new_game_commits(with_an_in_sim_requester: bool) -> Platformer2dSimHarness {
    use ambition_platformer2d::sim::SimScheduleExt;
    Platformer2dSimHarness::build(
        Platformer2dSimHarnessOptions::default()
            .with_timestep(TimestepMode::fixed_60hz())
            .with_required_start_room(ROOM)
            .with_sync_test_rollback_settings(4, 10),
        |app, options| {
            ambition_app::rl_sim::ambition_sim_composition(app, options)?;
            let label = app.sim_schedule();
            app.init_resource::<ResetCommits>();
            app.add_systems(label, record_every_new_game_commit);
            if with_an_in_sim_requester {
                app.add_systems(label, request_a_new_game_from_inside_the_sim);
            }
            Ok(())
        },
    )
    .expect("the sync-test harness builds with a reset-commit recorder")
}

/// Step until the durable latch rises, then a little further.
fn step_past_hydration(sim: &mut Platformer2dSimHarness) {
    let mut latched = false;
    for _ in 0..240 {
        sim.step(AgentAction::default());
        if sim
            .world()
            .get_resource::<SaveRestored>()
            .is_some_and(|restored| restored.0)
        {
            latched = true;
            break;
        }
    }
    assert!(
        latched,
        "the durable latch never rose, so this world never reached the state a \
         menu press would be made in"
    );
}

/// ⛔⛤ **A NEW GAME ASKED FOR FROM OUTSIDE THE SIMULATION IS SILENTLY SWALLOWED,
/// AND THE SHIPPED MENU IS OUTSIDE THE SIMULATION — MEASURED 2026-09-16.**
///
/// `NewGameResetRequested` is `resource-canonical` in
/// `rollback_schema_baseline.txt`: snapshotted, restored, and in the peer
/// checksum. `dispatch_menu_action` sets it through `ResMut` from top-level
/// `Update` (`request_reset`, `game/ambition_app/src/menu/kaleidoscope_app.rs`),
/// and `process_new_game_reset_request` consumes it from INSIDE the rewinding
/// schedule. So the flag is restored to its snapshot value before the consumer
/// ever sees it:
///
/// ```text
/// asked from INSIDE the sim schedule    1 commit
/// asked from OUTSIDE it (the menu)      0 commits, and the flag reads false again
/// ```
///
/// ⇒ **Pressing New Game does nothing under a rollback host, and leaves no
/// trace.** `false` afterwards is what makes it silent: the flag looks consumed
/// whether the reset ran or the restore ate the request.
///
/// ⭐ THE IN-SIM ARM IS THE CONTROL AND IT IS WHY THIS IS NOT A FIXTURE FAULT.
/// `process_new_game_reset_request` has several *"DECLINE, do not die"* roads and
/// clears the flag BEFORE them, so "0 commits, flag false" is exactly what a
/// declining reset would also print. The two arms differ only in WHERE the
/// request is written, so a decline would take both to zero.
///
/// ⚠ **THE REPAIR IS A SEMANTIC REQUEST, NOT A LOUDER FLAG.** The menu should
/// write a message the simulation consumes — the road `ItemGrantRequested` and
/// `ShopTransactionRequested` already take — so the press survives as an intent
/// rather than as rollback-owned state written from outside the timeline. Do not
/// "fix" it by removing `NewGameResetRequested` from the peer checksum: the
/// restore is installed independently of the checksum
/// (`install_resource_clone_checksum`), so narrowing what peers compare leaves
/// the swallow untouched.
#[test]
fn a_new_game_asked_for_from_outside_the_simulation_is_swallowed() {
    // CONTROL: the same request, written from inside the timeline.
    let mut inside = sim_recording_new_game_commits(true);
    step_past_hydration(&mut inside);
    for _ in 0..200 {
        inside.step(AgentAction::default());
    }
    let inside_commits = inside.world().resource::<ResetCommits>().0.clone();
    assert_eq!(
        inside_commits.len(),
        1,
        "the in-sim control did not commit exactly one New Game (ticks {inside_commits:?}). \
         0 means this fixture cannot perform a reset at all — every number below \
         would then be measuring the fixture, not the road. More than 1 means the \
         consumption itself is being replayed, which is a different defect in the \
         same place"
    );

    // THE MENU'S ROAD: a `ResMut` write from outside the simulation schedule.
    let mut outside = sim_recording_new_game_commits(false);
    step_past_hydration(&mut outside);
    for _ in 0..20 {
        outside.step(AgentAction::default());
    }
    outside
        .world_mut()
        .resource_mut::<ambition_platformer2d::actors::session::reset::NewGameResetRequested>()
        .request();
    assert!(
        outside
            .world()
            .resource::<ambition_platformer2d::actors::session::reset::NewGameResetRequested>()
            .request,
        "the request did not even land in the resource, so the arm below is not \
         measuring what happens to a request"
    );
    for _ in 0..200 {
        outside.step(AgentAction::default());
    }
    let outside_commits = outside.world().resource::<ResetCommits>().0.clone();

    assert_eq!(
        outside_commits.len(),
        0,
        "a New Game asked for from outside the simulation now commits \
         ({outside_commits:?}) — that is the FIX this arm is waiting for, not a \
         regression. Invert it: both arms should read 1, and the menu should be \
         writing a semantic request the simulation consumes"
    );
    assert!(
        !outside
            .world()
            .resource::<ambition_platformer2d::actors::session::reset::NewGameResetRequested>()
            .request,
        "the request is still standing, so it was not swallowed but merely \
         delayed — which would be a different (and much less bad) defect than \
         the one this arm records"
    );
}

/// The tick the in-sim control writes its grant request on.
const IN_SIM_GRANT_AT: u64 = 30;

fn request_a_grant_from_inside_the_sim(
    tick: bevy::prelude::Res<ambition_platformer2d::time::SimTick>,
    mut requests: bevy::prelude::MessageWriter<ambition_platformer2d::item::ItemGrantRequested>,
) {
    if tick.0 == IN_SIM_GRANT_AT {
        requests.write(ambition_platformer2d::item::ItemGrantRequested {
            item: ambition_platformer2d::item::Item::HealthCell,
            count: 1,
        });
    }
}

fn sim_granting_an_item(with_an_in_sim_producer: bool) -> Platformer2dSimHarness {
    use ambition_platformer2d::sim::SimScheduleExt;
    Platformer2dSimHarness::build(
        Platformer2dSimHarnessOptions::default()
            .with_timestep(TimestepMode::fixed_60hz())
            .with_required_start_room(ROOM)
            .with_sync_test_rollback_settings(4, 10),
        |app, options| {
            ambition_app::rl_sim::ambition_sim_composition(app, options)?;
            let label = app.sim_schedule();
            if with_an_in_sim_producer {
                app.add_systems(label, request_a_grant_from_inside_the_sim);
            }
            Ok(())
        },
    )
    .expect("the sync-test harness builds with an item-grant producer")
}

fn health_cells(sim: &Platformer2dSimHarness) -> u32 {
    sim.world()
        .get_resource::<ambition_platformer2d::item::OwnedItems>()
        .map_or(u32::MAX, |owned| {
            owned.count(ambition_platformer2d::item::Item::HealthCell)
        })
}

/// ⛔⛤ **THE REPAIR BOTH REVIEWS PROPOSED FOR [MENU-RESET-MIDSESSION] DOES NOT
/// WORK FROM OUTSIDE THE SIMULATION, AND THAT IS WHY THIS ARM EXISTS BEFORE THE
/// REPAIR DOES — MEASURED 2026-09-16.**
///
/// Both reviews named `ItemGrantRequested`/`ShopTransactionRequested` as the road
/// a menu action should take *instead of* writing rollback state. Their shipped
/// producer is a conversation node, which runs INSIDE the sim schedule, and the
/// messages are `clear_message_on_rollback`
/// (`crates/ambition_items/src/rollback_registration.rs`) — so **the queue is
/// rollback state too.** Measured, one grant of one `HealthCell`:
///
/// ```text
/// written from INSIDE the sim schedule    bag 3 -> 4
/// written from OUTSIDE it (the menu)      bag 3 -> 3, no transient change at all
/// ```
///
/// ⇒ **Re-spelling the menu's write as one of these messages would change
/// nothing.** A message cleared on rollback, produced outside the timeline, is
/// exactly as loseable as the resource write it would replace: the rewind clears
/// the queue and restores the bag, and nothing re-produces the request.
///
/// ⭐ THE IN-SIM ARM IS THE CONTROL, AND WITHOUT IT THE ZERO IS UNREADABLE — a
/// composition that never installs `apply_item_grants` prints the same `3 -> 3`.
/// The two arms differ only in WHERE the message is written.
///
/// ⚠ **SO THE OPEN QUESTION IS NOT "WHICH MESSAGE" BUT "HOW DOES A LOCAL INTENT
/// ENTER A SYNCHRONISED TIMELINE AT ALL".** A menu press is a local input event,
/// and in rollback netcode local inputs reach the timeline through the INPUT
/// payload GGRS carries — not through a resource or a rewinding queue. That is a
/// peer-visible design decision (the input wire format), which is why it belongs
/// in a maintainer question rather than in a quiet refactor. Recorded in
/// [MENU-RESET-MIDSESSION].
///
/// [MENU-RESET-MIDSESSION]: ../../../docs/planning/queue.md
#[test]
fn a_rollback_cleared_message_written_from_outside_the_simulation_is_also_lost() {
    // CONTROL: the same message, written from inside the timeline.
    let mut inside = sim_granting_an_item(true);
    step_past_hydration(&mut inside);
    let inside_before = health_cells(&inside);
    for _ in 0..200 {
        inside.step(AgentAction::default());
    }
    assert_eq!(
        health_cells(&inside),
        inside_before + 1,
        "the in-sim control did not grant its item, so `apply_item_grants` is \
         either absent from this composition or never reached — every number \
         below would then be measuring the fixture rather than the road"
    );

    // THE MENU'S POSITION: a producer outside the rewinding schedule.
    let mut outside = sim_granting_an_item(false);
    step_past_hydration(&mut outside);
    for _ in 0..20 {
        outside.step(AgentAction::default());
    }
    let before = health_cells(&outside);
    outside
        .world_mut()
        .write_message(ambition_platformer2d::item::ItemGrantRequested {
            item: ambition_platformer2d::item::Item::HealthCell,
            count: 1,
        });

    // Sampled every frame: a value that is right at both ends can still have
    // been granted and taken back in between, and "never landed" is a different
    // finding from "landed and was reverted".
    let mut ever_rose = false;
    for _ in 0..200 {
        outside.step(AgentAction::default());
        if health_cells(&outside) > before {
            ever_rose = true;
        }
    }
    assert_eq!(
        health_cells(&outside),
        before,
        "a grant requested from outside the simulation now survives — that is \
         the FIX this arm is waiting for, not a regression. Invert it, and note \
         in MENU-RESET-MIDSESSION what made a local intent reach the timeline"
    );
    assert!(
        !ever_rose,
        "the grant DID land and was then taken back, which is a different defect \
         from the one recorded here: this arm says the request never reaches \
         `apply_item_grants` at all. Update the row to whichever it is"
    );
}


const CUTSCENE_ID: &str = "cutscene_lab_intro";
const CUTSCENE_TRIGGER_AT: u64 = 30;

fn trigger_a_cutscene_from_inside_the_sim(
    tick: bevy::prelude::Res<ambition_platformer2d::time::SimTick>,
    mut queue: bevy::prelude::ResMut<ambition_platformer2d::cutscene::CutsceneTriggerQueue>,
) {
    if tick.0 == CUTSCENE_TRIGGER_AT {
        queue.request(CUTSCENE_ID);
    }
}

fn cutscene_beat(sim: &Platformer2dSimHarness) -> Option<(usize, bool)> {
    sim.world()
        .get_resource::<ambition_platformer2d::cutscene::ActiveCutscene>()
        .and_then(|active| active.runtime.as_ref())
        .map(|runtime| (runtime.beat_index, runtime.finished))
}

/// The tick the in-sim control issues its dismiss edge on — well after the
/// cutscene has reached its `Dialogue` beat and stalled there.
const DISMISS_AT: u64 = 100;

fn dismiss_the_dialogue_from_inside_the_sim(
    tick: bevy::prelude::Res<ambition_platformer2d::time::SimTick>,
    mut request: bevy::prelude::ResMut<ambition_platformer2d::cutscene::CutsceneAdvanceRequest>,
) {
    // ⚠ ONE TICK ONLY. A producer that holds the flag high would advance the
    // cutscene every tick, and then "the beat moved" would say nothing about
    // whether an EDGE survives.
    if tick.0 == DISMISS_AT {
        request.dismiss_dialogue = true;
    }
}

fn sim_playing_a_cutscene(with_an_in_sim_dismiss: bool) -> Platformer2dSimHarness {
    use ambition_platformer2d::sim::SimScheduleExt;
    Platformer2dSimHarness::build(
        Platformer2dSimHarnessOptions::default()
            .with_timestep(TimestepMode::fixed_60hz())
            .with_required_start_room(ROOM)
            .with_sync_test_rollback_settings(4, 10),
        |app, options| {
            use bevy::prelude::IntoScheduleConfigs as _;
            ambition_app::rl_sim::ambition_sim_composition(app, options)?;
            let label = app.sim_schedule();
            app.add_systems(label, trigger_a_cutscene_from_inside_the_sim);
            if with_an_in_sim_dismiss {
                // ⛔ THE `.before` IS LOAD-BEARING. `tick_active_cutscene`
                // consumes the request with `mem::take`, so a producer ordered
                // after it writes a flag that is never read in that tick and the
                // control arm would report the defect's own number.
                app.add_systems(
                    label,
                    dismiss_the_dialogue_from_inside_the_sim
                        .before(ambition_platformer2d::actors::cutscene::tick_active_cutscene),
                );
            }
            Ok(())
        },
    )
    .expect("the sync-test harness builds with a cutscene fixture")
}

/// Step until the cutscene reaches the beat that waits for a dismiss.
fn step_until_the_cutscene_waits(sim: &mut Platformer2dSimHarness) -> usize {
    for _ in 0..200 {
        sim.step(AgentAction::default());
        if let Some((index, _)) = cutscene_beat(sim) {
            if index >= 1 {
                return index;
            }
        }
    }
    panic!(
        "the cutscene never reached its `Dialogue` beat, so nothing below is \
         measuring a dismiss edge. `{CUTSCENE_ID}` must be in `CutsceneLibrary` \
         and `drain_cutscene_triggers` must be installed"
    );
}

/// ⛔⛤ **A CUTSCENE DISMISS RAISED OUTSIDE THE SIMULATION IS LOST, AND THE
/// SHIPPED PRODUCER IS OUTSIDE THE SIMULATION — MEASURED 2026-09-16.**
///
/// `CutsceneAdvanceRequest` is in NO row of `rollback_schema_baseline.txt`: it
/// carries no rollback decision at all. `apply_menu_frame_to_cutscene_request`
/// writes it from the HOST's input chain in `Update`
/// (`crates/ambition_platformer2d_host/src/lib.rs`, whose own comment notes that
/// set is *"LOAD-BEARING ONLY under the `RenderFrame` host, where the sim
/// schedule IS `Update`"*), while `tick_active_cutscene` consumes it with
/// `mem::take` from inside the sim schedule's `Cutscene` phase.
///
/// ```text
/// dismissed from INSIDE the sim schedule    beat 1 -> advances
/// dismissed from OUTSIDE it (the host)      beat 1 -> stays at 1
/// ```
///
/// ⇒ The first pass takes the edge and advances; the rewind restores
/// `ActiveCutscene` (which IS `resource-canonical`) but not the request, which is
/// unregistered and already `false`. Nothing re-produces it, so the
/// authoritative replay never advances. **A dismiss press does nothing.** Same
/// shape as [Q136]'s menu findings: an out-of-sim producer of a request consumed
/// inside the rewinding schedule.
///
/// ⭐ THE IN-SIM ARM IS THE CONTROL. A fixture where the cutscene simply cannot
/// advance — a missing library entry, an uninstalled `drain_cutscene_triggers` —
/// prints the same stalled `1`, and `step_until_the_cutscene_waits` panics rather
/// than letting that read as the defect.
///
/// ⚠ AND IT IS A LOST EDGE, NOT A DOUBLED ONE. The `Dialogue` beat advances only
/// on the edge, so this arm would equally catch the opposite failure: an
/// unregistered request that is NOT taken back would advance the beat more than
/// once per press, and the asserted beat index below pins the count rather than
/// the direction.
///
/// [Q136]: ../../../docs/planning/awaiting-maintainer-decision.md
#[test]
fn a_cutscene_dismiss_raised_outside_the_simulation_is_lost() {
    // CONTROL: the same edge, raised inside the timeline.
    let mut inside = sim_playing_a_cutscene(true);
    let inside_waiting = step_until_the_cutscene_waits(&mut inside);
    for _ in 0..200 {
        inside.step(AgentAction::default());
    }
    let inside_after = cutscene_beat(&inside);
    assert!(
        inside_after.is_none_or(|(index, _)| index > inside_waiting),
        "the in-sim control did not advance past beat {inside_waiting} \
         (now {inside_after:?}), so this fixture cannot consume a dismiss at all \
         and the arm below would be measuring the fixture. `None` here is a PASS: \
         the cutscene ran to completion and dropped its runtime"
    );

    // THE SHIPPED PRODUCER'S POSITION: outside the rewinding schedule.
    let mut outside = sim_playing_a_cutscene(false);
    let waiting = step_until_the_cutscene_waits(&mut outside);
    outside
        .world_mut()
        .resource_mut::<ambition_platformer2d::cutscene::CutsceneAdvanceRequest>()
        .dismiss_dialogue = true;
    assert!(
        outside
            .world()
            .resource::<ambition_platformer2d::cutscene::CutsceneAdvanceRequest>()
            .dismiss_dialogue,
        "the dismiss did not even land in the resource"
    );
    for _ in 0..200 {
        outside.step(AgentAction::default());
    }
    assert_eq!(
        cutscene_beat(&outside),
        Some((waiting, false)),
        "a dismiss raised from outside the simulation now advances the cutscene \
         — that is the FIX this arm is waiting for, not a regression. Invert it, \
         and record in CUTSCENE-ROLLBACK-DECISION what made the edge survive"
    );
}

