//! A badnik is squashed by each player of its own live room, by a stomp, a
//! roll, or a touch that harms, and by no player of another room. The squash
//! is a module on the `body_contact` road (`ambition_demo_sanic_modules`), so
//! these ask it of the built game: the module admitted, the game answering its
//! observations, and the engine lowering its requests.

use std::collections::BTreeMap;

use ambition_demo_sanic_app::build_demo_app;
use ambition_platformer2d::actor::ActorFaction;
use ambition_platformer2d::characters::actor::{BodyHealth, Invulnerability};
use ambition_platformer2d::characters::control::{DrivingParticipant, PlayerSlot};
use ambition_platformer2d::combat::actor_tuning::ActorConfig;
use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::engine_core::AabbExt;
use ambition_platformer2d::entity_catalog::placements::CharacterBrain;
use ambition_platformer2d::platformer::lifecycle::{InRoomInstance, LiveRoomInstance, RoomInstanceRoot, SessionScopedEntity};
use ambition_platformer2d::platformer::markers::{PlayerEntity, PrimaryPlayer};
use ambition_platformer2d::platformer::{frame_env::ResolvedMotionFrame, sim_id::SimId};
use bevy::prelude::*;

fn sanic(app: &mut App) -> Entity {
    let world = app.world_mut();
    world.query_filtered::<Entity, With<PrimaryPlayer>>().iter(world).next().expect("his body")
}

/// Each live enemy by the brain key of its placement.
fn badniks(app: &mut App) -> BTreeMap<String, Entity> {
    let world = app.world_mut();
    world
        .query::<(Entity, &ActorConfig, &ActorFaction, &BodyHealth)>()
        .iter(world)
        .filter(|(_, _, faction, health)| matches!(faction, ActorFaction::Enemy) && health.alive())
        .filter_map(|(entity, config, ..)| match &config.brain {
            CharacterBrain::Custom(key) => Some((key.clone(), entity)),
            _ => None,
        })
        .collect()
}

/// The speedway with him seated and its badniks landed.
fn the_speedway() -> App {
    let mut app = build_demo_app();
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(std::time::Duration::from_secs_f32(1.0 / 60.0)));
    let ready = (0..600).any(|_| {
        app.update();
        let world = app.world_mut();
        let seated = world.query_filtered::<(), With<PrimaryPlayer>>().iter(world).next().is_some();
        seated && badniks(&mut app).len() >= 4
    });
    assert!(ready, "the speedway seats Sanic and places its badniks");
    for _ in 0..90 {
        app.update();
    }
    app
}

/// A second player body in `room`, driven by a seat.
fn a_player_body(app: &mut App, room: LiveRoomInstance) -> Entity {
    let first = sanic(app);
    let session = *app.world().get::<SessionScopedEntity>(first).unwrap();
    app.world_mut()
        .spawn((
            ae::BodyKinematics {
                material: Default::default(),
                pos: Vec2::ZERO,
                vel: Vec2::ZERO,
                canonical_size: Vec2::new(24.0, 32.0),
                facing: 1.0,
            },
            PlayerEntity,
            DrivingParticipant(PlayerSlot(1)),
            SimId::placement(&format!("fixture_second_player_{}", room.ordinal())),
            session,
            ResolvedMotionFrame::default(),
            InRoomInstance(room),
        ))
        .id()
}

/// How a toucher meets the badnik each tick.
#[derive(Clone, Copy, Debug)]
enum Touch {
    /// Falling, its feet just into the badnik's head.
    Stomp,
    /// Running into its side.
    Side,
}

/// Put `toucher` on `badnik` each tick for some ticks, with Sanic far away
/// unless he is the toucher and harmless to it. `dress` is applied to the
/// toucher each tick. The toucher's upward speed when the badnik was squashed,
/// or `None` when it was not.
fn squash(app: &mut App, badnik: Entity, toucher: Entity, touch: Touch, dress: fn(&mut EntityWorldMut)) -> Option<f32> {
    let him = sanic(app);
    for _ in 0..20 {
        let Some(it) = app.world().get::<ae::BodyKinematics>(badnik).map(|kin| kin.aabb()) else {
            return app.world().get::<ae::BodyKinematics>(toucher).map(|kin| -kin.vel.y);
        };
        if let Some(mut health) = app.world_mut().get_mut::<BodyHealth>(him) {
            health.health.invulnerable.set(Invulnerability::SCRIPTED, true);
        }
        let size = app.world().get::<ae::BodyKinematics>(toucher).unwrap().size();
        let (at, vel) = match touch {
            Touch::Stomp => (Vec2::new(it.center().x, it.min.y + 1.0 - size.y * 0.5), Vec2::new(0.0, 200.0)),
            Touch::Side => (Vec2::new(it.min.x - size.x * 0.5 + 4.0, it.center().y), Vec2::new(300.0, 0.0)),
        };
        if toucher != him {
            put(app, him, at + Vec2::new(1500.0, -400.0), Vec2::ZERO);
        }
        dress(&mut app.world_mut().entity_mut(toucher));
        put(app, toucher, at, vel);
        app.update();
        if app.world().get::<BodyHealth>(badnik).is_none_or(|health| !health.alive()) {
            return app.world().get::<ae::BodyKinematics>(toucher).map(|kin| -kin.vel.y);
        }
    }
    None
}

/// Move `body` to `at` at `vel`: through the transit authority for a body
/// with a motion model (his), whose clusters a bare position write leaves
/// behind, and by its position for the fixture body.
fn put(app: &mut App, body: Entity, at: Vec2, vel: Vec2) {
    let world = app.world_mut();
    let mut q = world.query::<(ae::BodyClusterQueryData, &mut ambition_platformer2d::actor::MotionModel)>();
    if let Ok((mut clusters, mut model)) = q.get_mut(world, body) {
        ae::movement::transit_body(&mut model, &mut clusters.as_clusters_mut(), at, ae::movement::TransitVelocity::Set(vel));
        return;
    }
    let mut kin = world.get_mut::<ae::BodyKinematics>(body).unwrap();
    kin.pos = at;
    kin.vel = vel;
}

fn plain(_: &mut EntityWorldMut) {}

fn rolling(body: &mut EntityWorldMut) {
    body.insert(ambition_demo_sanic::ball_dash::Rolling::default());
}

fn harming(body: &mut EntityWorldMut) {
    body.insert(ambition_platformer2d::actors::body::empowerment::Empowered::held(
        ambition_platformer2d::character::Empowerment::HARMS_ON_CONTACT,
    ));
}

/// ⭐ A stomp squashes each kind of badnik the speedway places, and he leaves
/// it at the bounce his pack states (`badnik.bounce_speed`), read by the
/// module through `sanic.rules.badnik`.
#[test]
fn a_stomp_squashes_each_kind_and_bounces_him_at_his_packs_speed() {
    let mut app = the_speedway();
    let him = sanic(&mut app);
    let authority = ambition_demo_sanic::rules::sanic_rules().badnik.bounce_speed;
    let placed = badniks(&mut app);
    assert!(placed.len() >= 4, "the premise: the speedway places its kinds, {:?}", placed.keys().collect::<Vec<_>>());
    for (kind, badnik) in placed {
        let bounced = squash(&mut app, badnik, him, Touch::Stomp, plain)
            .unwrap_or_else(|| panic!("his stomp does not squash `{kind}`"));
        // The bounce is set in the contact phase; the rest of the tick's
        // gravity takes off at most a tick or two of fall.
        assert!(
            bounced <= authority + 0.5 && bounced > authority - 80.0,
            "he left `{kind}` at {bounced} up, and his pack states {authority}"
        );
    }
}

/// A roll or a touch that harms squashes a badnik from the side, and does not
/// bounce; a plain side touch does not squash it (the shared contact damage
/// has it).
#[test]
fn a_roll_or_a_harming_touch_squashes_from_the_side_and_a_plain_one_does_not() {
    let mut app = the_speedway();
    let him = sanic(&mut app);
    let placed: Vec<Entity> = badniks(&mut app).into_values().collect();
    assert!(squash(&mut app, placed[0], him, Touch::Side, plain).is_none(), "a plain side touch squashes it");
    let bounced = squash(&mut app, placed[0], him, Touch::Side, rolling).expect("a roll into its side squashes it");
    assert!(bounced < 100.0, "a roll is not bounced: he left it at {bounced} up");
    let bounced = squash(&mut app, placed[1], him, Touch::Side, harming).expect("a harming touch squashes it");
    assert!(bounced < 100.0, "a side touch is not bounced: he left it at {bounced} up");
}

/// ⭐ Each player of the badnik's live room squashes it; a player of another
/// live room at the same place does not.
#[test]
fn a_badnik_is_squashed_by_each_player_of_its_room_and_by_no_other() {
    let mut app = the_speedway();
    let badnik = badniks(&mut app)[ambition_demo_sanic::badnik::BADNIK_BRAIN_KEY];
    let room = app.world().get::<InRoomInstance>(badnik).expect("the badnik has its live room").0;
    let second = a_player_body(&mut app, room);
    assert!(squash(&mut app, badnik, second, Touch::Stomp, plain).is_some(), "a second player of its room squashes it");

    let mut app = the_speedway();
    let badnik = badniks(&mut app)[ambition_demo_sanic::badnik::BADNIK_BRAIN_KEY];
    let other = LiveRoomInstance::ACTIVATION.next().next().next();
    app.world_mut().spawn((RoomInstanceRoot, other));
    let elsewhere = a_player_body(&mut app, other);
    assert!(squash(&mut app, badnik, elsewhere, Touch::Stomp, plain).is_none(), "a player of another live room does");
}
