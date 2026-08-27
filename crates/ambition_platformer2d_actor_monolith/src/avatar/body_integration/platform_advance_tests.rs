//! A world with no home avatar still moves.

use super::advance_moving_platforms;
use ambition_platformer2d_core as ae;
use ambition_platformer2d_world::collision::MovingPlatformSet;
use ambition_platformer2d_world::platforms::MovingPlatformState;
use bevy::prelude::*;

fn app_with_one_platform(scaled_dt: f32) -> App {
    let mut app = App::new();
    app.insert_resource(ambition_time::WorldTime {
        raw_dt: 1.0 / 60.0,
        scaled_dt,
    });
    app.insert_resource(MovingPlatformSet(vec![MovingPlatformState::from_authored(
        ae::Vec2::new(400.0, 800.0),
        ae::Vec2::new(155.0, 18.0),
        240.0,
        130.0,
    )]));
    app.add_systems(Update, advance_moving_platforms);
    app
}

fn platform_x(app: &App) -> f32 {
    app.world().resource::<MovingPlatformSet>().0[0].pos.x
}

/// A session with no `PrimaryPlayer` still advances its platforms.
///
/// No body is spawned here at all, which is the condition under test.
#[test]
fn platforms_advance_in_a_world_with_no_player_body() {
    let mut app = app_with_one_platform(1.0 / 60.0);
    let before = platform_x(&app);
    app.update();
    let after = platform_x(&app);

    assert!(
        (after - before - 130.0 / 60.0).abs() < 1e-3,
        "a platform in a world with no home avatar must travel its own speed \
         ({before} -> {after}); anything that consults a player body to decide \
         whether the WORLD may move is frozen in every match"
    );
    let world = app.world_mut();
    let mut primaries = world.query_filtered::<Entity, With<ambition_platformer2d_shared_tangle::markers::PrimaryPlayer>>();
    assert_eq!(
        primaries.iter(world).count(),
        0,
        "the fixture must contain no primary player, or it is not testing the \
         condition that broke"
    );
}

/// The world's own clock is what stops them — the poison half.
///
/// A zero `sim_dt` is how hitstop, pause and bullet-time all reach this system,
/// because the primary body's hitstop already drives the global clock to zero.
/// If a later edit reintroduces a per-body read to decide the freeze, this stays
/// green while the test above goes red — so the pair pins the authority, not
/// just the outcome.
#[test]
fn a_stopped_clock_stops_them() {
    let mut app = app_with_one_platform(0.0);
    let before = platform_x(&app);
    app.update();
    assert_eq!(
        platform_x(&app),
        before,
        "a zero sim dt must hold the platform still — that is how hitstop, pause \
         and bullet-time reach world geometry"
    );
}
