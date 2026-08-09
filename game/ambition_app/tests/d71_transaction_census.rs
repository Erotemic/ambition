//! **PROBE (D71): does a room change on the PLAYER's route open a transition
//! transaction?**
//!
//! Counts, over one authored `Door` crossing driven by a held interact press:
//! * how many times the ACTIVE ROOM changed,
//! * how many times a `RoomTransitionLoadState` transaction was open, and
//! * how many times a deferred `PendingLifecycleCommit` intent was recorded.
//!
//! Run under both hosts. The rollback host is the one the shipped desktop binary
//! composes (`cli.rs` sets `SimulationHost::Ggrs` unconditionally and
//! `runtime::rollback::local_session` autostarts a `LocalSyncTest` session), so
//! its numbers are the ones Jon's play produced.
//!
//! # ⭐ THE ANSWER, measured 2026-08-09 (D71 is REPRODUCIBLE, and NAMED)
//!
//! ```text
//! fixed-tick host (ConfirmedFrameBoundary absent):  11 room changes, 11 transactions,  0 deferred intents
//! ROLLBACK host   (ConfirmedFrameBoundary present): 24 room changes,  0 transactions, 24 deferred intents
//! shipped app: ConfirmedFrameBoundary present=true, LocalSyncTest / LocalMaintainer
//! ```
//!
//! ⭐ **1:1 both times, and never the same one.** Every room change is accounted
//! for by exactly one route, and which route it is turns entirely on whether a
//! rollback host is composed. The shipped desktop binary composes one.
//!
//! **The bypass has a name.**
//! [`detect_room_transition_system`](ambition_platformer2d::actors::rooms::detect_room_transition_system)
//! forks on `Option<Res<ConfirmedFrameBoundary>>`: with a rollback host present
//! it records a `PendingLifecycleCommit` and **returns before writing
//! `RoomTransitionRequested`**. So `begin_room_transition_load_system` never
//! runs, no `ambition_load` barrier opens, `GameMode::RoomTransition` is never
//! requested, and no cover is ever presented. The room is instead rebuilt by
//! `runtime::lifecycle_commit::commit_confirmed_lifecycle` in `PreUpdate`,
//! outside `GgrsSchedule` — a second, transaction-free room-change route.
//!
//! Two independent signals agree, neither of them the sampled resource above:
//! * the unconditional `[world-event] room-transition begin seq=N` line prints
//!   11 times under the fixed-tick host and **0 times** under the rollback host,
//!   while `room-loaded` prints on both;
//! * temporary markers at all five room-construction call sites showed the
//!   rollback host's 24 loads came from **none** of the four transactional
//!   sites — they came through `RoomConstructionPlan::apply_to_world`, which
//!   only `commit_confirmed_lifecycle` calls.
//!
//! ⚠ the `transactions=0` figure alone would have been weak evidence: this probe
//! samples `RoomTransitionLoadState` once per `sim.step()`, so a transaction that
//! opened and closed inside one advance would read as zero. The two signals above
//! are what make it a bypass rather than a sampling artifact.
//!
//! The deferral is deliberate (`session::lifecycle_commit` module docs: the load
//! machine is not rollback-registered, so it must not run on a speculative
//! frame). The DEFECT is that the deferred route inherited none of the
//! transaction's obligations — cover included. Fixing that is a composition
//! question and deliberately not in this commit.

#![cfg(feature = "rl_sim")]

use ambition_app::rl_sim::{
    AgentAction, AmbitionSim, Platformer2dSimHarness, Platformer2dSimHarnessOptions, TimestepMode,
};

use crate::common::base;

fn interact() -> AgentAction {
    AgentAction {
        interact: true,
        interact_held: true,
        ..base()
    }
}

fn active_room(sim: &mut Platformer2dSimHarness) -> String {
    sim.observation().active_room.clone()
}

fn transaction_open(sim: &Platformer2dSimHarness) -> bool {
    sim.world()
        .get_resource::<ambition_platformer2d::runtime::room_transition::RoomTransitionLoadState>()
        .is_some_and(|state| state.active.is_some())
}

fn boundary_present(sim: &Platformer2dSimHarness) -> bool {
    sim.world()
        .get_resource::<ambition_platformer2d::engine_core::ConfirmedFrameBoundary>()
        .is_some()
}

/// The OTHER route's signal, and the reason this census is evidence rather than
/// a sampling artifact: the deferred intent `detect_room_transition_system`
/// records instead of writing `RoomTransitionRequested` when a rollback host is
/// present. A host that opens no transaction and records no intent has simply
/// not changed rooms; a host that records intents while opening no transaction
/// is changing rooms by the bypass.
fn deferred_intent_pending(sim: &Platformer2dSimHarness) -> bool {
    sim.world()
        .get_resource::<ambition_platformer2d::actors::session::lifecycle_commit::PendingLifecycleCommit>()
        .is_some_and(|state| state.pending.is_some())
}

struct Census {
    room_changes: usize,
    transactions: usize,
    deferred_intents: usize,
    boundary: bool,
}

fn census(rollback: bool, frames: usize) -> Census {
    let mut options =
        Platformer2dSimHarnessOptions::default().with_timestep(TimestepMode::fixed_60hz());
    if rollback {
        options = options.with_sync_test_rollback_settings(4, 10);
    }
    let mut sim = Platformer2dSimHarness::new_with_options(options).expect("the harness builds");

    for _ in 0..20 {
        sim.step(base());
    }
    let boundary = boundary_present(&sim);

    let door = {
        let world = sim.world_mut();
        let mut query = world.query::<&ambition_platformer2d::actors::rooms::RoomSet>();
        let room_set = query
            .iter(world)
            .next()
            .expect("the active room has a RoomSet");
        room_set
            .active_loading_zones()
            .iter()
            .find(|zone| {
                zone.activation == ambition_platformer2d::world::rooms::LoadingZoneActivation::Door
            })
            .cloned()
            .expect("the start room authors a Door zone")
    };
    let centre = {
        use ambition_platformer2d::engine_core::AabbExt as _;
        door.aabb.center()
    };
    sim.teleport_player((centre.x, centre.y));

    let mut room = active_room(&mut sim);
    let mut room_changes = 0usize;
    let mut transactions = 0usize;
    let mut deferred_intents = 0usize;
    let mut was_open = transaction_open(&sim);
    let mut was_pending = deferred_intent_pending(&sim);
    for _ in 0..frames {
        sim.step(interact());
        let open = transaction_open(&sim);
        if open && !was_open {
            transactions += 1;
        }
        was_open = open;
        let pending = deferred_intent_pending(&sim);
        if pending && !was_pending {
            deferred_intents += 1;
        }
        was_pending = pending;
        let now = active_room(&mut sim);
        if now != room {
            room_changes += 1;
            room = now;
        }
    }
    Census {
        room_changes,
        transactions,
        deferred_intents,
        boundary,
    }
}

/// The other half: is the SHIPPED app's host the rollback one? The harness above
/// asks for a sync-test session explicitly; this asks the real composition.
#[test]
#[ignore = "PROBE, print-only: panics to report its census. Preserved mid-investigation \
           (D71) when the run was paused; run explicitly with --ignored."]
fn d71_probe_shipped_app_host() {
    use ambition_app::app::{build_visible_app, shell_host, VisibleRenderMode};
    use ambition_platformer2d::game_shell::ShellCommand;

    let mut app = build_visible_app(VisibleRenderMode::NoWindow, true);
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_millis(16),
    ));
    for _ in 0..ambition_app::app::shared_host_startup_ticks() * 2 {
        app.update();
    }
    app.world_mut().write_message(ShellCommand::GoTo(
        shell_host::AMBITION_GAMEPLAY_ROUTE.into(),
    ));
    let mut boundary = false;
    let mut ownership = String::from("<none>");
    for _ in 0..900 {
        app.update();
        std::thread::sleep(std::time::Duration::from_millis(2));
        boundary = app
            .world()
            .get_resource::<ambition_platformer2d::engine_core::ConfirmedFrameBoundary>()
            .is_some();
        if boundary {
            ownership = format!(
                "{:?}",
                app.world()
                    .get_resource::<ambition_platformer2d::runtime::rollback::RollbackSessionOwnership>()
            );
            break;
        }
    }
    panic!("D71 SHIPPED HOST: ConfirmedFrameBoundary present={boundary} ownership={ownership}");
}

#[test]
#[ignore = "PROBE, print-only: panics to report its census. Preserved mid-investigation \
           (D71) when the run was paused; run explicitly with --ignored."]
fn d71_probe_counts_room_changes_against_transactions() {
    let eager = census(false, 120);
    let roll = census(true, 360);
    panic!(
        "D71 CENSUS\n  \
         fixed-tick host (ConfirmedFrameBoundary={}): \
         room changes={} transactions={} deferred intents={}\n  \
         ROLLBACK host  (ConfirmedFrameBoundary={}): \
         room changes={} transactions={} deferred intents={}",
        eager.boundary,
        eager.room_changes,
        eager.transactions,
        eager.deferred_intents,
        roll.boundary,
        roll.room_changes,
        roll.transactions,
        roll.deferred_intents,
    );
}
