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
fn live_axes(sim: &Platformer2dSimHarness) -> u32 {
    sim.world().resource::<OwnedItems>().count(Item::Axe)
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

    // ── THE WRITE, in the shape the menu makes it: `OwnedItems` mutated from
    // outside the rewinding schedule, mid-window.
    {
        let world = sim.world_mut();
        let mut owned = world.resource_mut::<OwnedItems>();
        owned.grant(Item::Axe, 1);
    }
    let granted = live_axes(&sim);
    assert!(granted > 0, "the grant did not reach the live bag at all");

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
        if live_lost_at.is_none() && live_axes(&sim) < granted {
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
        world.resource_mut::<OwnedItems>().grant(Item::Axe, 1);
    }
    let granted = live_axes(&sim);
    assert!(granted > 0, "the grant did not reach the live bag at all");

    for frame in 0..240 {
        sim.step(AgentAction::default());
        assert!(
            live_axes(&sim) >= granted,
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
