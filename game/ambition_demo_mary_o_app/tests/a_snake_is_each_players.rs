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
/// (`effects.shell_stomp`).
#[test]
fn a_stomp_plays_the_burst_its_rules_state() {
    let (mut app, snake) = a_walking_snake();
    crate::the_switch_ends_the_castle::record_effects(&mut app);
    let mary_o = her(&mut app);
    assert!(!still_walking_after_a_stomp(&mut app, snake, mary_o), "the premise: her stomp makes a shell");
    let stated = crate::the_switch_ends_the_castle::shape_of(&ambition_demo_mary_o::rules::mary_o_rules().effects.shell_stomp);
    let played = &app.world().resource::<crate::the_switch_ends_the_castle::Played>().bursts;
    assert!(played.contains(&stated), "the stomp did not play the file's burst {stated:?}: played {played:?}");
}
