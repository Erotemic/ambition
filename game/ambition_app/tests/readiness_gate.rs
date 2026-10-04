//! The readiness gate of a room transition, on each host: how many updates
//! and ticks pass from the opening of the transaction to its authorization and
//! to the room change.
//!
//! The rule: a transaction opened in a pass of the readiness set is not
//! authorized in that pass, so a request and its apply are never in one pass.
//! These are the controls of that rule. `two_peers.rs` holds the sync-test
//! control and the peer arm.

use ambition_app::{AmbitionSim as _, Platformer2dSimHarness};
use ambition_platformer2d::runtime::room_transition::{RoomTransitionLoadPhase, RoomTransitionLoadState};

use crate::common::{a_save_that_has_seen_the_hub_intro, base, door_to, fixed_60hz_room_options};
use crate::two_players_two_live_rooms::ROOM;

const HUB: &str = "central_hub_complex";

fn sim_tick(sim: &Platformer2dSimHarness) -> u64 {
    sim.world().resource::<ambition_platformer2d::time::SimTick>().0
}

fn phase(sim: &Platformer2dSimHarness) -> Option<RoomTransitionLoadPhase> {
    sim.world()
        .resource::<RoomTransitionLoadState>()
        .active
        .as_ref()
        .map(|transaction| transaction.phase)
}

/// (updates, ticks) from the update that opened the transaction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Since {
    updates: u32,
    ticks: u64,
}

#[derive(Debug, PartialEq, Eq)]
struct Crossing {
    /// The phase at the end of the update that opened the transaction.
    /// `AwaitingReadiness`: the opening pass authorized nothing.
    after_the_opening_pass: RoomTransitionLoadPhase,
    /// To the first update that showed the transaction authorized (or past it).
    authorized: Since,
    /// To the first update that showed the hero in the hub.
    committed: Since,
    /// The updates of the walk that ran no simulation step.
    updates_with_no_step: u32,
}

/// The hero stands on the door to the hub and holds interact. With
/// `halve_the_frame`, one update advances half of the host's step period, so
/// each second update runs no simulation step.
fn cross(fixed_tick: bool, halve_the_frame: bool) -> Crossing {
    use ambition_platformer2d::engine_core::AabbExt as _;
    let options = fixed_60hz_room_options(ROOM)
        .with_save(a_save_that_has_seen_the_hub_intro())
        .with_fixed_tick(fixed_tick);
    let mut sim = Platformer2dSimHarness::new_with_options(options).expect("the room boots");
    for _ in 0..20 {
        sim.step(base());
    }
    if halve_the_frame {
        let period = ambition_platformer2d::sim::manual_step_period(sim.app_mut())
            .expect("a fixed-tick host has a step period");
        sim.app_mut()
            .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(period / 2));
    }
    let center = door_to(&mut sim, HUB).aabb.center();
    sim.teleport_player((center.x, center.y));

    let interact = ambition_app::rl_sim::AgentAction {
        interact: true,
        interact_held: true,
        ..base()
    };
    let mut opened: Option<(u32, u64, RoomTransitionLoadPhase)> = None;
    let mut authorized: Option<Since> = None;
    let mut updates_with_no_step = 0;
    for update in 1..=400u32 {
        let before = sim_tick(&sim);
        sim.step(interact);
        let tick = sim_tick(&sim);
        if tick == before {
            updates_with_no_step += 1;
        }
        let now = phase(&sim);
        let in_the_hub = sim.observation().active_room == HUB;
        if let (None, Some(phase)) = (opened, now) {
            opened = Some((update, tick, phase));
        }
        let Some((opened_on, opened_at, after_the_opening_pass)) = opened else {
            assert!(
                !in_the_hub,
                "the room changed in update {update} and no update before it showed \
                 an open transaction: the request and its apply were in one pass"
            );
            continue;
        };
        let since = Since {
            updates: update - opened_on,
            ticks: tick - opened_at,
        };
        if authorized.is_none()
            && (in_the_hub || !matches!(now, Some(RoomTransitionLoadPhase::AwaitingReadiness)))
        {
            authorized = Some(since);
        }
        if in_the_hub {
            return Crossing {
                after_the_opening_pass,
                authorized: authorized.expect("set above"),
                committed: since,
                updates_with_no_step,
            };
        }
    }
    panic!("the hero did not reach the hub in 400 updates; the transaction opened at {opened:?}");
}

fn since(updates: u32, ticks: u64) -> Since {
    Since { updates, ticks }
}

/// The simulation is in `Update`, with the readiness set between the
/// detection and the apply. The opening pass authorizes nothing, and the next
/// update authorizes and applies: one update and one tick.
#[test]
fn a_render_frame_host_commits_on_the_update_after_the_opening() {
    assert_eq!(
        cross(false, false),
        Crossing {
            after_the_opening_pass: RoomTransitionLoadPhase::AwaitingReadiness,
            authorized: since(1, 1),
            committed: since(1, 1),
            updates_with_no_step: 0,
        }
    );
}

/// The simulation is in `FixedUpdate`, before `Update` in the frame. The
/// update after the opening authorizes, and the step of the update after
/// that applies: two updates and two ticks.
#[test]
fn a_fixed_tick_host_commits_two_steps_after_the_opening() {
    assert_eq!(
        cross(true, false),
        Crossing {
            after_the_opening_pass: RoomTransitionLoadPhase::AwaitingReadiness,
            authorized: since(1, 1),
            committed: since(2, 2),
            updates_with_no_step: 0,
        }
    );
}

/// ⚠ THE ONE HOST WHERE THE PASS GATE AND THE OLD TICK GATE DIFFER, measured
/// 2026-10-04. Each second update of a fixed-tick host runs no step here.
///
/// The gate counts passes of the readiness set, so the update after the
/// opening authorizes, also when it ran no step: authorized after (1 update,
/// 0 ticks), committed after (2, 1). The tick gate waited for a step:
/// authorized after (2, 1), committed after (4, 2). So on this host the room
/// changes one tick earlier than it did. The request and the apply are still
/// in different passes: the apply is in a later simulation step.
#[test]
fn a_fixed_tick_host_authorizes_in_an_update_that_runs_no_step() {
    assert_eq!(
        cross(true, true),
        Crossing {
            after_the_opening_pass: RoomTransitionLoadPhase::AwaitingReadiness,
            authorized: since(1, 0),
            committed: since(2, 1),
            updates_with_no_step: 3,
        }
    );
}
