//! A2 (adaptive split in one room, `docs/planning/game/multiplayer.md`): the
//! screen splits when the second player leaves what the shared view frames,
//! and merges again, with hysteresis, when the players are back together.

#![cfg(feature = "rl_sim")]

use ambition_app::rl_sim::{AgentAction, AmbitionSim as _, Platformer2dSimHarness};
use ambition_platformer2d::engine_core::{BodyKinematics, ControlFrame, Vec2};

use crate::a_second_seat_joins_the_session::{bodies_of_the_seat, join, the_primary};
use crate::two_players_two_live_rooms::the_views;

const HUB: &str = "central_hub_complex";

fn pos(sim: &Platformer2dSimHarness, body: bevy::prelude::Entity) -> Vec2 {
    sim.world().get::<BodyKinematics>(body).expect("the body has kinematics").pos
}

/// The world size the first view shows.
fn visible(sim: &mut Platformer2dSimHarness) -> Vec2 {
    let world = sim.world_mut();
    world
        .query::<(
            &ambition_platformer2d::sim_view::LocalViewId,
            &ambition_platformer2d::sim_view::camera_snapshot::ResolvedCameraSnapshot,
        )>()
        .iter(world)
        .find(|(id, _)| **id == ambition_platformer2d::sim_view::LocalViewId::FIRST)
        .and_then(|(_, resolved)| resolved.frame().map(|frame| frame.snapshot.visible_view))
        .expect("the first view is framed")
}

/// Seat 1 joins beside Alice in the hub and runs right, away from her, until
/// it is farther from her than the shared view shows (or `frames` pass).
/// Returns the sim, Alice's body and seat 1's body.
fn bob_runs_out_of_the_shared_frame() -> (Platformer2dSimHarness, bevy::prelude::Entity, bevy::prelude::Entity) {
    use crate::common::{a_save_that_has_seen_the_hub_intro, fixed_60hz_room_options};
    let mut sim = Platformer2dSimHarness::new_with_options(
        fixed_60hz_room_options(HUB).with_save(a_save_that_has_seen_the_hub_intro()),
    )
    .unwrap_or_else(|error| panic!("{HUB} boots: {error:?}"));
    sim.step_n(AgentAction::default(), 10);
    join(&mut sim, 4).expect("seat 1 pressed Jump and no body was built");
    sim.drive_seat(1, ControlFrame::default());
    sim.step_n(AgentAction::default(), 10);
    assert_eq!(the_views(&mut sim), vec![(0, None, false)], "control: two players together, one view");
    let alice = the_primary(&mut sim);
    let bob = bodies_of_the_seat(&mut sim)[0];
    let half = visible(&mut sim) * 0.5;
    for _ in 0..900 {
        sim.drive_seat(1, ControlFrame { axis_x: 1.0, ..ControlFrame::default() });
        sim.step(AgentAction::default());
        if (pos(&sim, bob) - pos(&sim, alice)).x.abs() > half.x * 1.2 {
            break;
        }
    }
    sim.drive_seat(1, ControlFrame::default());
    let apart = (pos(&sim, bob) - pos(&sim, alice)).x.abs();
    assert!(
        apart > half.x * 1.2,
        "precondition: seat 1 got only {apart} from Alice, and the shared view shows {half:?} each way"
    );
    (sim, alice, bob)
}

/// Bob runs out of what the shared view shows: a view opens for his seat.
/// He comes back beside Alice: the view stays for a moment (hysteresis), then
/// closes. Control: together at the start, one view.
#[test]
fn a_second_view_opens_when_the_players_drift_apart_in_one_room_and_closes_when_they_regroup() {
    let (mut sim, alice, bob) = bob_runs_out_of_the_shared_frame();
    sim.step_n(AgentAction::default(), 5);
    assert_eq!(
        the_views(&mut sim),
        vec![(0, None, false), (1, Some(1), true)],
        "the views while seat 1 is out of the shared frame"
    );
    let beside = pos(&sim, alice) + Vec2::new(30.0, 0.0);
    crate::a_second_seat_joins_the_session::put_body_at(&mut sim, bob, beside);
    sim.step_n(AgentAction::default(), 10);
    assert_eq!(the_views(&mut sim).len(), 2, "the split held for a moment after they met (hysteresis)");
    sim.step_n(AgentAction::default(), 120);
    assert_eq!(the_views(&mut sim), vec![(0, None, false)], "the views after they stood together a while");
}
