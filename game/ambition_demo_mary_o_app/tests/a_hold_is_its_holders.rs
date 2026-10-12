//! A hold on a body is its holder's, in the shipped Mary-O (review A3, A4).
//!
//! * The Snake Plant holds its own pose (`PoseHold::Path`). It lets go of that
//!   hold, and only of that hold, when it dies. It is shy of every player of
//!   its live room: it read one global primary player, so with two players
//!   nobody kept it down.
//! * The pipe ride holds her control and her pose with its own holds
//!   (`ControlHold::Transit`, `PoseHold::Transit`). It borrowed the combat
//!   recoil lock and set it to zero at the end of the ride, and it removed
//!   the whole pose marker, so a recoil and another holder ended with it.

use bevy::prelude::*;

use ambition_demo_mary_o::ldtk_vocabulary::MaryOPipeMouth;
use ambition_demo_mary_o::pipe::PipeTransit;
use ambition_platformer2d::characters::actor::{BodyCombat, BodyHealth, Invulnerability};
use ambition_platformer2d::characters::control::{ControlHold, ControlHolds};
use ambition_platformer2d::combat::actor_tuning::{ActorConfig, ContactThreatWithdrawn};
use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::engine_core::AabbExt;
use ambition_platformer2d::input::ControlFrame;
use ambition_platformer2d::platformer::lifecycle::{InRoomInstance, LiveRoomInstance, RoomInstanceRoot};
use ambition_platformer2d::platformer::markers::{PlayerEntity, PrimaryPlayer};

const PLANTS: &str = "mary_o_1_3";

fn her(app: &mut App) -> Entity {
    let world = app.world_mut();
    world.query_filtered::<Entity, With<PrimaryPlayer>>().iter(world).next().expect("her body")
}

/// The room 1-3, with her seated, and its Snake Plant furthest to the left.
fn the_plants_room() -> (App, Entity) {
    let mut app = ambition_demo_mary_o_app::build_demo_app();
    app.insert_resource(ambition_demo_mary_o::provider::MaryOEntryRoom(PLANTS.to_string()));
    let plant = (0..600)
        .find_map(|_| {
            app.update();
            let world = app.world_mut();
            let seated = world.query_filtered::<(), With<PrimaryPlayer>>().iter(world).next().is_some();
            let mut q = world.query::<(Entity, &ActorConfig, &ae::BodyKinematics)>();
            let plant = q
                .iter(world)
                .filter(|(_, config, _)| ambition_demo_mary_o::plant::is_snake_plant_brain(&config.brain))
                .min_by(|a, b| a.2.pos.x.total_cmp(&b.2.pos.x))
                .map(|(entity, ..)| entity);
            plant.filter(|_| seated)
        })
        .expect("1-3 builds its Snake Plants and seats Mary-O");
    (app, plant)
}

/// Put `body` at `at` with no velocity.
fn pin(app: &mut App, body: Entity, at: Vec2) {
    let mut kin = app.world_mut().get_mut::<ae::BodyKinematics>(body).expect("a body");
    kin.pos = at;
    kin.vel = Vec2::ZERO;
}

/// The ticks of `ticks` on which the plant is out far enough to hurt, with
/// `on_the_pipe` held on its pipe and Mary-O held far from it. The first
/// cycle lets a plant that is out go down; the second is counted.
fn ticks_out(app: &mut App, plant: Entity, on_the_pipe: Option<Entity>) -> usize {
    let at = app.world().get::<ae::BodyKinematics>(plant).unwrap().pos;
    let size = app.world().get::<ae::BodyKinematics>(plant).unwrap().size;
    // On the pipe: over the plant, its feet above the mouth.
    let on_pipe = Vec2::new(at.x, at.y - size.y - 48.0);
    let mary_o = her(app);
    let mut out = 0;
    for tick in 0..660 {
        let far = Vec2::new(at.x + 1200.0, at.y - 400.0);
        if on_the_pipe != Some(mary_o) {
            pin(app, mary_o, far);
        }
        if let Some(body) = on_the_pipe {
            pin(app, body, on_pipe);
        }
        if let Some(mut health) = app.world_mut().get_mut::<BodyHealth>(mary_o) {
            health.health.invulnerable.set(Invulnerability::SCRIPTED, true);
        }
        app.update();
        let withdrawn = app.world().get::<ContactThreatWithdrawn>(plant).expect("the plant").0;
        if tick >= 330 && !withdrawn {
            out += 1;
        }
    }
    out
}

/// A second player body in `room`: what the plant's shyness reads.
fn another_player(app: &mut App, room: LiveRoomInstance) -> Entity {
    app.world_mut()
        .spawn((
            ae::BodyKinematics {
                pos: Vec2::ZERO,
                vel: Vec2::ZERO,
                size: Vec2::new(24.0, 32.0),
                facing: 1.0,
            },
            PlayerEntity,
            InRoomInstance(room),
        ))
        .id()
}

fn plant_room(app: &App, plant: Entity) -> LiveRoomInstance {
    app.world().get::<InRoomInstance>(plant).expect("the plant is stamped with its live room").0
}

/// ⭐ A3: every player of the plant's live room keeps it down, and a player
/// of another live room does not.
#[test]
fn the_snake_plant_is_shy_of_each_player_of_its_room_and_of_no_other() {
    // The control: Mary-O on the pipe.
    let (mut app, plant) = the_plants_room();
    let mary_o = her(&mut app);
    assert_eq!(ticks_out(&mut app, plant, Some(mary_o)), 0, "the premise: she keeps it down");
    // And nobody on it: it comes up.
    let (mut app, plant) = the_plants_room();
    let free = ticks_out(&mut app, plant, None);
    assert!(free > 30, "the premise: a plant with nobody near comes up ({free} ticks out)");

    // A second player of its room keeps it down. The single global read gave
    // `None` with two players: the plant came up under him.
    let (mut app, plant) = the_plants_room();
    let room = plant_room(&app, plant);
    let second = another_player(&mut app, room);
    assert_eq!(ticks_out(&mut app, plant, Some(second)), 0, "a second player on its pipe keeps it down");

    // A player of another live room at the same place does not.
    let (mut app, plant) = the_plants_room();
    let other = LiveRoomInstance::ACTIVATION.next().next().next();
    app.world_mut().spawn((RoomInstanceRoot, other));
    let elsewhere = another_player(&mut app, other);
    let out = ticks_out(&mut app, plant, Some(elsewhere));
    assert!(out > 30, "a player of another live room does not keep it down ({out} ticks out)");
}

/// ⭐ A3: a plant that dies lets go of its hold, and only of its own.
#[test]
fn a_dead_snake_plant_lets_go_of_its_own_hold_only() {
    let (mut app, plant) = the_plants_room();
    for _ in 0..5 {
        app.update();
    }
    let holds = |app: &App| app.world().get::<ae::PoseOwnedExternally>(plant).copied();
    assert!(
        holds(&app).is_some_and(|holds| holds.holds(ae::PoseHold::Path)),
        "the premise: a live plant in its pot holds its pose: {:?}",
        holds(&app)
    );
    // Another holder has it too.
    ae::claim_pose_hold_on(&mut app.world_mut().entity_mut(plant), ae::PoseHold::Ride);
    app.world_mut().get_mut::<BodyHealth>(plant).unwrap().health.current = 0;
    for _ in 0..3 {
        app.update();
    }
    let after = holds(&app);
    assert!(
        after.is_some_and(|holds| !holds.holds(ae::PoseHold::Path) && holds.holds(ae::PoseHold::Ride)),
        "dead, it lets go of its path and the other holder keeps its hold: {after:?}"
    );
}

fn step(app: &mut App, frame: ControlFrame) {
    app.world_mut().resource_mut::<ambition_platformer2d::scripted_input::ScriptedControls>().0 = frame;
    app.update();
}

fn in_a_pipe(app: &mut App) -> bool {
    let world = app.world_mut();
    world.query_filtered::<(), (With<PrimaryPlayer>, With<PipeTransit>)>().iter(world).next().is_some()
}

/// ⭐ A4: a ride through a pipe takes its own holds and leaves the rest. A
/// recoil from a hit in the pipe, and another holder of her pose, are hers
/// after the ride.
#[test]
fn a_pipe_ride_leaves_a_recoil_and_another_holders_hold() {
    let mut app = ambition_demo_mary_o_app::build_demo_app();
    ambition_platformer2d::scripted_input::drive_the_local_participant(&mut app);
    for _ in 0..90 {
        app.update();
    }
    let tubes = ambition_demo_mary_o::tubes_for_room(ambition_demo_mary_o::LEVEL_1_1_ROOM_ID);
    let down = tubes.iter().find(|tube| tube.entrance.mouth == MaryOPipeMouth::Up).expect("1-1 has a tube down");
    let mary_o = her(&mut app);
    let size = app.world().get::<ae::BodyKinematics>(mary_o).unwrap().size;
    let band = down.entrance.mouth_band();
    {
        let world = app.world_mut();
        let mut q = world
            .query_filtered::<(ae::BodyClusterQueryData, &mut ambition_platformer2d::actor::MotionModel), With<PrimaryPlayer>>();
        let (mut item, mut model) = q.iter_mut(world).next().expect("her body");
        let mut clusters = item.as_clusters_mut();
        let at = Vec2::new(band.center().x, band.min.y - size.y * 0.5);
        ae::movement::transit_body(&mut model, &mut clusters, at, ae::movement::TransitVelocity::Zero);
    }
    step(&mut app, ControlFrame::default());
    let mut press = ControlFrame::default();
    press.axis_y = 1.0;
    let entered = (0..60).any(|_| {
        step(&mut app, press);
        in_a_pipe(&mut app)
    });
    assert!(entered, "the premise: she goes down the pipe");
    assert!(
        app.world().get::<ControlHolds>(mary_o).is_some_and(|holds| holds.holds(ControlHold::Transit)),
        "the ride holds her control"
    );
    for _ in 0..3 {
        step(&mut app, ControlFrame::default());
    }
    // A hit in the pipe: its recoil. And another holder of her pose.
    app.world_mut().get_mut::<BodyCombat>(mary_o).unwrap().recoil_lock_timer = 3.0;
    ae::claim_pose_hold_on(&mut app.world_mut().entity_mut(mary_o), ae::PoseHold::Ride);
    let mut ticks = 0;
    while in_a_pipe(&mut app) {
        assert!(ticks < 600, "the premise: the ride ends");
        step(&mut app, ControlFrame::default());
        ticks += 1;
    }
    let recoil = app.world().get::<BodyCombat>(mary_o).unwrap().recoil_lock_timer;
    assert!(recoil > 0.5, "the recoil runs its own course after the ride: {recoil}");
    let pose = app.world().get::<ae::PoseOwnedExternally>(mary_o).copied();
    assert!(
        pose.is_some_and(|holds| holds.holds(ae::PoseHold::Ride) && !holds.holds(ae::PoseHold::Transit)),
        "the ride let go of its pose hold and the other holder keeps its own: {pose:?}"
    );
    assert!(
        !app.world().get::<ControlHolds>(mary_o).is_some_and(|holds| holds.holds(ControlHold::Transit)),
        "the ride let go of her control"
    );
}
