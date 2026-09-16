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
