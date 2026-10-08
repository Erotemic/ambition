//! LEDGE-OCCUPANCY (Q43): a rewind across a ledge trump gives the same holder.
//!
//! The trump (`ambition_combat::ledge_trump::resolve_ledge_trumps`) keeps no
//! state of its own. Each tick it derives the holders from the hangs, and the
//! hang is in `actor.motion_model`, which the rollback restores. The arbitration
//! is proved on a hand-built app in `ledge_trump/tests.rs`, where nothing
//! rewinds. This arm drives the same world with a GGRS sync-test session and
//! with none, and compares which body holds the edge on each tick.
//!
//! Two player bodies fall past the right face of one floating block: Alice
//! and the body seat 1 joins with. Seat 1's body starts just above the lip
//! and catches it first. Alice starts higher, catches it later, and trumps it.
//! Both are player bodies so that neither harms the other on contact (a
//! hostile rival's touch knocked Alice off the edge). They are one character,
//! so one size: bodies of different sizes on one corner are one edge in
//! `two_fighters_of_different_sizes_on_one_corner_are_one_edge`.

#![cfg(feature = "rl_sim")]

use ambition_app::rl_sim::{
    AmbitionSim as _, Platformer2dSimHarness, Platformer2dSimHarnessOptions, TimestepMode,
};
use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::platformer::markers::PrimaryPlayer;
use bevy::prelude::{Entity, With};

use crate::common::base;

/// The room the other rollback arms use.
const ROOM: &str = "combat_calibration_lab";
/// Ticks after the setup. The trump happens near the middle.
const TICKS: usize = 60;
/// The seat that joins with the second body.
const SEAT: u8 = 1;

fn rewinding_sim() -> Platformer2dSimHarness {
    Platformer2dSimHarness::new_with_options(
        Platformer2dSimHarnessOptions::default()
            .with_timestep(TimestepMode::fixed_60hz())
            .with_required_start_room(ROOM)
            .with_sync_test_rollback_settings(4, 10)
            .with_rollback_players(2),
    )
    .expect("the two-seat GGRS sync-test harness builds in the calibration lab")
}

/// The control: the same world with no rollback session.
fn fixed_tick_sim() -> Platformer2dSimHarness {
    Platformer2dSimHarness::new_with_options(
        Platformer2dSimHarnessOptions::default()
            .with_timestep(TimestepMode::fixed_60hz())
            .with_required_start_room(ROOM),
    )
    .expect("the same world builds without a rollback session")
}

fn alice(sim: &mut Platformer2dSimHarness) -> Entity {
    let world = sim.world_mut();
    world
        .query_filtered::<Entity, With<PrimaryPlayer>>()
        .single(world)
        .expect("Alice's body is in the world")
}

/// Is `body` hanging on a ledge (not climbing)?
fn hangs(sim: &mut Platformer2dSimHarness, body: Entity) -> bool {
    match sim.world().get::<ae::MotionModel>(body) {
        Some(ae::MotionModel::AxisSwept(axis)) => {
            axis.state.ledge_grab.as_ref().is_some_and(|hang| !hang.climbing)
        }
        _ => false,
    }
}

/// Give `body` the ledge grab, in its base and in its effective set (the
/// effective set is projected from the base each tick).
fn can_grab_ledges(sim: &mut Platformer2dSimHarness, body: Entity) {
    let mut entity = sim.world_mut().entity_mut(body);
    entity
        .get_mut::<ae::body_clusters::AbilityBase>()
        .expect("the body has an ability base")
        .abilities
        .ledge_grab = true;
    entity
        .get_mut::<ae::body_clusters::BodyAbilities>()
        .expect("the body has abilities")
        .abilities
        .ledge_grab = true;
}

/// Put `body` at rest beside the block's right face, with its head `above`
/// px above the lip.
fn place(sim: &mut Platformer2dSimHarness, body: Entity, face: f32, lip: f32, above: f32) {
    let mut kin = sim
        .world_mut()
        .get_mut::<ae::BodyKinematics>(body)
        .expect("the body has kinematics");
    let half = kin.size * 0.5;
    kin.pos = ae::Vec2::new(face + half.x + 1.0, lip - above + half.y);
    kin.vel = ae::Vec2::ZERO;
}

/// Build the scene in `sim` and return, for each tick after it, whether Alice
/// and seat 1's body hang, or the rollback refusal that stopped the run.
fn drive(mut sim: Platformer2dSimHarness) -> (Vec<(bool, bool)>, Option<String>) {
    for _ in 0..10 {
        sim.drive_seat(SEAT, ae::ControlFrame::default());
        sim.step(base());
    }
    let (rival, _) = crate::a_second_seat_joins_the_session::join(&mut sim, 4)
        .expect("seat 1 pressed Jump and no body was built");
    let floor = {
        let alice = alice(&mut sim);
        sim.world().get::<ae::BodyKinematics>(alice).expect("Alice has kinematics").pos
    };
    // A floating block, well clear of the floor.
    let (face, lip) = (floor.x - 40.0, floor.y - 260.0);
    sim.add_block(ae::Block::solid(
        "trump_ledge",
        ae::Vec2::new(face - 120.0, lip),
        ae::Vec2::new(120.0, 24.0),
    ));
    let alice = alice(&mut sim);
    can_grab_ledges(&mut sim, alice);
    can_grab_ledges(&mut sim, rival);
    place(&mut sim, rival, face, lip, 20.0);
    place(&mut sim, alice, face, lip, 90.0);
    sim.rebase_rollback_history().expect("the rollback history rebases over the scene");

    let mut timeline = Vec::new();
    for _ in 0..TICKS {
        sim.drive_seat(SEAT, ae::ControlFrame::default());
        if let Err(refusal) = sim.try_step(base()) {
            return (timeline, Some(refusal));
        }
        timeline.push((hangs(&mut sim, alice), hangs(&mut sim, rival)));
    }
    (timeline, None)
}

/// ⭐ A REWIND ACROSS A TRUMP GIVES THE SAME HOLDER. The fixed-tick world is
/// the authority; its floor is a real trump (seat 1's body held the edge, then
/// Alice did, and never both). The world under a sync-test session (which
/// rewinds 4 frames each tick and checks each resimulation) must stay healthy
/// and give the same holder on every tick.
#[test]
fn a_rewind_across_a_ledge_trump_gives_the_same_holder() {
    let (fixed, fixed_refusal) = drive(fixed_tick_sim());
    assert_eq!(fixed_refusal, None, "a world with no rollback session refuses nothing");
    let rival_held = fixed.iter().position(|&(alice, rival)| rival && !alice);
    let alice_took = fixed.iter().position(|&(alice, rival)| alice && !rival);
    assert!(
        matches!((rival_held, alice_took), (Some(first), Some(then)) if first < then)
            && fixed.iter().all(|&(alice, rival)| !(alice && rival)),
        "precondition: in the fixed-tick world, seat 1's body did not hold the edge and then lose \
         it to Alice, so there is no trump to rewind across: {fixed:?}"
    );

    let (rewound, refusal) = drive(rewinding_sim());
    assert_eq!(
        (rewound, refusal),
        (fixed, None),
        "(holders per tick, refusal): a rewind across the trump gave another holder"
    );
}
