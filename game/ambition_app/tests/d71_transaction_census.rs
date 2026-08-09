//! **PROBE (D71): does a room change on the PLAYER's route open a transition
//! transaction?**
//!
//! Counts, over one authored `Door` crossing driven by a held interact press:
//! * how many times the ACTIVE ROOM changed, and
//! * how many times a `RoomTransitionLoadState` transaction was open.
//!
//! Run under both hosts. The rollback host is the one the shipped desktop binary
//! composes (`cli.rs` sets `SimulationHost::Ggrs` unconditionally and
//! `runtime::rollback::local_session` autostarts a `LocalSyncTest` session), so
//! its numbers are the ones Jon's play produced.

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

/// (room changes, transaction openings, rollback boundary present)
fn census(rollback: bool, frames: usize) -> (usize, usize, bool) {
    let mut options = Platformer2dSimHarnessOptions::default().with_timestep(TimestepMode::fixed_60hz());
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
                zone.activation
                    == ambition_platformer2d::world::rooms::LoadingZoneActivation::Door
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
    let mut was_open = transaction_open(&sim);
    for _ in 0..frames {
        sim.step(interact());
        let open = transaction_open(&sim);
        if open && !was_open {
            transactions += 1;
        }
        was_open = open;
        let now = active_room(&mut sim);
        if now != room {
            room_changes += 1;
            room = now;
        }
    }
    (room_changes, transactions, boundary)
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
    let (eager_rooms, eager_tx, eager_boundary) = census(false, 120);
    let (roll_rooms, roll_tx, roll_boundary) = census(true, 360);
    panic!(
        "D71 CENSUS\n  \
         fixed-tick host (no ConfirmedFrameBoundary={eager_boundary}): \
         room changes={eager_rooms} transactions={eager_tx}\n  \
         ROLLBACK host  (ConfirmedFrameBoundary={roll_boundary}): \
         room changes={roll_rooms} transactions={roll_tx}"
    );
}
