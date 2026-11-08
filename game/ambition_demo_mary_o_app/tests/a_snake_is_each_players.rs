//! A snake of Mary-O is stomped by each player of its own live room, and by
//! no player of another. It read one global `PrimaryPlayer` with `single()`.

use bevy::prelude::*;

use ambition_demo_mary_o::snake::{is_snake_brain, SnakeShell};
use ambition_platformer2d::characters::actor::{BodyHealth, Invulnerability};
use ambition_platformer2d::combat::actor_tuning::ActorConfig;
use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::engine_core::AabbExt;
use ambition_platformer2d::platformer::lifecycle::{InRoomInstance, LiveRoomInstance, RoomInstanceRoot};
use ambition_platformer2d::platformer::markers::{PlayerEntity, PrimaryPlayer};

fn her(app: &mut App) -> Entity {
    let world = app.world_mut();
    world.query_filtered::<Entity, With<PrimaryPlayer>>().iter(world).next().expect("her body")
}

/// 1-1 with her seated, and a snake that walks.
fn a_walking_snake() -> (App, Entity) {
    let mut app = ambition_demo_mary_o_app::build_demo_app();
    let snake = (0..600)
        .find_map(|_| {
            app.update();
            let world = app.world_mut();
            let seated = world.query_filtered::<(), With<PrimaryPlayer>>().iter(world).next().is_some();
            let mut q = world.query::<(Entity, &ActorConfig, &SnakeShell)>();
            let snake = q
                .iter(world)
                .filter(|(_, config, shell)| is_snake_brain(&config.brain) && matches!(shell, SnakeShell::Walking))
                .map(|(entity, ..)| entity)
                .min();
            snake.filter(|_| seated)
        })
        .expect("1-1 builds a snake and seats Mary-O");
    (app, snake)
}

fn a_player_body(app: &mut App, room: LiveRoomInstance) -> Entity {
    app.world_mut()
        .spawn((
            ae::BodyKinematics { material: Default::default(),
                pos: Vec2::ZERO,
                vel: Vec2::ZERO,
                canonical_size: Vec2::new(24.0, 32.0),
                facing: 1.0,
            },
            PlayerEntity,
            InRoomInstance(room),
        ))
        .id()
}

/// Stand `stomper` on the snake's head for some ticks, with Mary-O far away
/// unless she is the stomper. Is the snake still walking after?
fn still_walking_after_a_stomp(app: &mut App, snake: Entity, stomper: Entity) -> bool {
    let mary_o = her(app);
    for _ in 0..20 {
        let snake_box = app.world().get::<ae::BodyKinematics>(snake).expect("the snake").aabb();
        let size = app.world().get::<ae::BodyKinematics>(stomper).unwrap().size;
        let on_head = Vec2::new(snake_box.center().x, snake_box.min.y + 1.0 - size.y * 0.5);
        if stomper != mary_o {
            let mut kin = app.world_mut().get_mut::<ae::BodyKinematics>(mary_o).unwrap();
            kin.pos = on_head + Vec2::new(1500.0, -400.0);
            kin.vel = Vec2::ZERO;
        }
        if let Some(mut health) = app.world_mut().get_mut::<BodyHealth>(mary_o) {
            health.health.invulnerable.set(Invulnerability::SCRIPTED, true);
        }
        let mut kin = app.world_mut().get_mut::<ae::BodyKinematics>(stomper).unwrap();
        kin.pos = on_head;
        kin.vel = Vec2::ZERO;
        app.update();
        if !matches!(app.world().get::<SnakeShell>(snake), Some(SnakeShell::Walking)) {
            return false;
        }
    }
    true
}

/// ⭐ Each player of the snake's live room stomps it; a player of another
/// live room at the same place does not.
#[test]
fn a_snake_is_stomped_by_each_player_of_its_room_and_by_no_other() {
    // The control: Mary-O stomps it.
    let (mut app, snake) = a_walking_snake();
    let mary_o = her(&mut app);
    assert!(!still_walking_after_a_stomp(&mut app, snake, mary_o), "the premise: her stomp makes a shell");

    // A second player of its room.
    let (mut app, snake) = a_walking_snake();
    let room = app.world().get::<InRoomInstance>(snake).expect("the snake has its live room").0;
    let second = a_player_body(&mut app, room);
    assert!(!still_walking_after_a_stomp(&mut app, snake, second), "a second player of its room stomps it");

    // A player of another live room.
    let (mut app, snake) = a_walking_snake();
    let other = LiveRoomInstance::ACTIVATION.next().next().next();
    app.world_mut().spawn((RoomInstanceRoot, other));
    let elsewhere = a_player_body(&mut app, other);
    assert!(still_walking_after_a_stomp(&mut app, snake, elsewhere), "a player of another live room does not");
}

/// A stomp plays the burst Mary-O's rules file states
/// (`stomp.dust`).
#[test]
fn a_stomp_plays_the_burst_its_rules_state() {
    let (mut app, snake) = a_walking_snake();
    crate::the_switch_ends_the_castle::record_effects(&mut app);
    let mary_o = her(&mut app);
    assert!(!still_walking_after_a_stomp(&mut app, snake, mary_o), "the premise: her stomp makes a shell");
    let stated = crate::the_switch_ends_the_castle::shape_of(&ambition_demo_mary_o::rules::mary_o_rules().stomp.dust);
    let played = &app.world().resource::<crate::the_switch_ends_the_castle::Played>().bursts;
    assert!(played.contains(&stated), "the stomp did not play the file's burst {stated:?}: played {played:?}");
}

fn joined_shell_party() -> (App, Entity, [Entity; 2]) {
    use ambition_platformer2d::characters::control::{DrivingParticipant, PlayerSlot};
    use ambition_platformer2d::actor::Health;
    let mut app = ambition_demo_mary_o_app::build_demo_app();
    app.insert_resource(ambition_demo_mary_o::provider::MaryOEntryRoom(
        ambition_demo_mary_o::test_course::TEST_COURSE_ROOM_ID.into()));
    for _ in 0..120 { app.update(); }
    let mut snakes = app.world_mut().query_filtered::<Entity, With<SnakeShell>>();
    let snake = snakes.single(app.world()).expect("the fixture course has one snake");
    let alice = her(&mut app);
    let mut bob = None;
    for _ in 0..60 {
        ambition_platformer2d::rollback::drive_slot_frame(app.world_mut(), PlayerSlot(1),
            ae::ControlFrame { start_pressed: true, ..Default::default() });
        app.update();
        let mut query = app.world_mut().query_filtered::<(Entity, &DrivingParticipant), With<PlayerEntity>>();
        bob = query.iter(app.world()).find(|(_, seat)| seat.0 == PlayerSlot(1)).map(|(body, _)| body);
        if bob.is_some() { break; }
    }
    let bob = bob.expect("the second participant joins through committed Start input");
    ambition_platformer2d::rollback::drive_slot_frame(app.world_mut(), PlayerSlot(1), ae::ControlFrame::default());
    for body in [alice, bob, snake] {
        app.world_mut().entity_mut(body).insert(BodyHealth::new(Health::new(100)));
        let mut kin = app.world_mut().get_mut::<ae::BodyKinematics>(body).unwrap();
        kin.pos = Vec2::new(350.0, 416.0 - kin.size().y * 0.5);
        kin.vel = Vec2::ZERO;
    }
    app.world_mut().entity_mut(snake).insert(
        ambition_platformer2d::characters::control::ControlHolds::only(
            ambition_platformer2d::characters::control::ControlHold::Sequence));
    (app, snake, [alice, bob])
}

fn shell_contacts(app: &mut App, snake: Entity, bodies: [Entity; 2]) -> Vec<ambition_platformer2d::combat::events::HitEvent> {
    use ambition_platformer2d::combat::events::{HitEvent, HitSource};
    let mut cursor = app.world().resource::<Messages<HitEvent>>().get_cursor_current();
    let mut hits = Vec::new();
    for _ in 0..3 {
        app.update();
        hits.extend(cursor.read(app.world().resource::<Messages<HitEvent>>())
            .filter(|hit| hit.source == HitSource::Contact && hit.attacker
                .is_some_and(|owner| owner == snake || bodies.contains(&owner))).cloned());
    }
    hits
}

#[test]
fn a_sliding_shell_uses_body_contacts_for_both_local_participants() {
    use ambition_platformer2d::combat::events::HitTarget;
    let (mut app, snake, bodies) = joined_shell_party();
    app.world_mut().entity_mut(snake).insert(SnakeShell::Sliding { dir: 1.0, grace: 0.0 });
    let hits = shell_contacts(&mut app, snake, bodies);
    for body in bodies {
        assert_eq!(app.world().get::<BodyHealth>(body).unwrap().current(), 99,
            "the shell must deal one impact to participant body {body:?}");
        assert!(hits.iter().any(|hit| hit.target == HitTarget::Body(body)),
            "the shell did not resolve participant body {body:?}");
    }
    assert!(hits.iter().all(|hit| hit.room.is_some()), "the shell lost its emission room");
    assert_eq!(app.world().get::<BodyHealth>(snake).unwrap().current(), 100,
        "the shell struck its own body");
}

#[test]
fn a_fresh_shell_kick_protects_only_the_body_that_kicked_it() {
    use ambition_demo_mary_o::snake::ShellKickCredit;
    use ambition_platformer2d::combat::events::HitTarget;
    let (mut app, snake, bodies) = joined_shell_party();
    app.world_mut().entity_mut(snake).insert(SnakeShell::Boxed(100.0));
    let hits = shell_contacts(&mut app, snake, bodies);
    let kicker = app.world().get::<ShellKickCredit>(snake).unwrap().0.expect("the kick names its body");
    assert!(bodies.contains(&kicker));
    for body in bodies {
        let expected = if body == kicker { 100 } else { 99 };
        assert_eq!(app.world().get::<BodyHealth>(body).unwrap().current(), expected,
            "kick grace must protect only {kicker:?}, body={body:?}");
        assert_eq!(hits.iter().any(|hit| hit.target == HitTarget::Body(body)), body != kicker);
    }
    assert!(hits.iter().all(|hit| hit.attacker == Some(kicker)), "the kick lost its credit");
}

#[test]
fn a_shell_stays_in_its_emission_room_when_its_kicker_is_in_another_room() {
    use ambition_demo_mary_o::snake::ShellKickCredit;
    use ambition_platformer2d::combat::events::HitTarget;
    use ambition_platformer2d::world::rooms::LiveRoomDefinition;
    let (mut app, snake, bodies @ [alice, bob]) = joined_shell_party();
    let emission = app.world().get::<InRoomInstance>(snake).unwrap().0;
    let other = emission.next();
    let mut roots = app.world_mut().query_filtered::<(&LiveRoomInstance, &ae::RoomGeometry, &LiveRoomDefinition), With<RoomInstanceRoot>>();
    let (_, geometry, definition) = roots.iter(app.world()).find(|(room, ..)| **room == emission).unwrap();
    let (geometry, definition) = (geometry.clone(), *definition);
    app.world_mut().spawn((RoomInstanceRoot, other, geometry, definition));
    app.world_mut().entity_mut(alice).insert(InRoomInstance(other));
    app.world_mut().entity_mut(snake).insert((
        SnakeShell::Sliding { dir: 1.0, grace: 0.0 }, ShellKickCredit(Some(alice))));
    let hits = shell_contacts(&mut app, snake, bodies);
    assert_eq!(app.world().get::<BodyHealth>(alice).unwrap().current(), 100);
    assert_eq!(app.world().get::<BodyHealth>(bob).unwrap().current(), 99);
    assert!(!hits.iter().any(|hit| hit.target == HitTarget::Body(alice)));
    assert!(hits.iter().any(|hit| hit.target == HitTarget::Body(bob)));
    assert!(hits.iter().all(|hit| hit.room == Some(emission) && hit.attacker == Some(alice)),
        "the shell must keep its emission room and remote kick credit");
}
