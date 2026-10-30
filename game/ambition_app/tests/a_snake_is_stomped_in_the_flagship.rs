//! Landing on a Solid Snake in Mary-O, hosted by the flagship, is a stomp:
//! the snake goes into its shell and she is not hurt (Jon, 2026-10-30:
//! "Stomping on solid snake now hurts her instead of its koopa-like
//! behavior"). The demo's own app has this test
//! (`level_1_acceptance::landing_on_a_snake_stomps_it_instead_of_hurting_her`);
//! this is the same drop in the host Jon plays her in.

use bevy::prelude::*;

use ambition_demo_mary_o::snake::{is_snake_brain, SnakeShell};
use ambition_platformer2d::characters::actor::BodyHealth;
use ambition_platformer2d::combat::actor_tuning::ActorConfig;
use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::engine_core::AabbExt;
use ambition_platformer2d::input::ControlFrame;
use ambition_platformer2d::platformer::markers::PrimaryPlayer;

fn step(app: &mut App) {
    app.world_mut().resource_mut::<ambition_platformer2d::scripted_input::ScriptedControls>().0 = ControlFrame::default();
    app.update();
}

fn her_health(app: &mut App) -> i32 {
    let mut q = app.world_mut().query_filtered::<&BodyHealth, With<PrimaryPlayer>>();
    q.iter(app.world()).next().expect("her body").current()
}

fn walking_snake(app: &mut App) -> Option<(Entity, ae::Aabb)> {
    let world = app.world_mut();
    let mut q = world.query::<(Entity, &ActorConfig, &SnakeShell, &ae::BodyKinematics)>();
    q.iter(world)
        .filter(|(_, config, shell, _)| is_snake_brain(&config.brain) && matches!(shell, SnakeShell::Walking))
        .min_by_key(|(entity, ..)| *entity)
        .map(|(entity, _, _, kin)| (entity, kin.aabb()))
}

#[test]
fn landing_on_a_snake_stomps_it_in_the_flagship() {
    let mut app = crate::mary_o_hud_surround::mary_o_hud_app(ae::Vec2::new(1600.0, 900.0));
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(std::time::Duration::from_secs_f64(1.0 / 60.0)));
    ambition_platformer2d::scripted_input::drive_the_local_participant(&mut app);
    crate::mary_o_hud_surround::enter_mary_o(&mut app);
    let mut snake = None;
    for _ in 0..600 {
        step(&mut app);
        snake = walking_snake(&mut app);
        if snake.is_some() {
            break;
        }
    }
    let (snake, aabb) = snake.expect("Mary-O's 1-1 builds a walking snake");
    for _ in 0..30 {
        step(&mut app);
    }
    let aabb = app.world().get::<ae::BodyKinematics>(snake).map(|kin| kin.aabb()).unwrap_or(aabb);
    let hp = her_health(&mut app);
    {
        let mut q = app.world_mut().query_filtered::<&mut ae::BodyKinematics, With<PrimaryPlayer>>();
        let mut kin = q.single_mut(app.world_mut()).expect("her body");
        kin.pos = ae::Vec2::new(aabb.center().x, aabb.min.y - 160.0);
        kin.vel = ae::Vec2::ZERO;
    }
    let mut stomped = false;
    for tick in 0..240 {
        step(&mut app);
        assert_eq!(her_health(&mut app), hp, "landing on a snake hurt her (tick {tick})");
        if !matches!(app.world().get::<SnakeShell>(snake), Some(SnakeShell::Walking)) {
            stomped = true;
            break;
        }
    }
    assert!(stomped, "falling onto a walking snake must put it into its shell");
}
