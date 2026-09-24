//! Morph Ball belongs to the Ambition protagonist in the Ambition game. Mary-O
//! is played through the same generic player bundle, and the bundle grants
//! nothing: her kit is her own authored set, and her game adds no Morph Ball.

use ambition_demo_mary_o_app::build_demo_app;
use ambition_platformer2d::actor::{AbilityBase, BodyAbilities};
use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::input::ControlFrame;
use ambition_platformer2d::platformer::markers::PrimaryPlayer;
use bevy::prelude::*;

fn boot() -> App {
    let mut app = build_demo_app();
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_secs_f32(1.0 / 60.0),
    ));
    for _ in 0..600 {
        app.update();
        let mut q = app
            .world_mut()
            .query_filtered::<&ae::BodyKinematics, With<PrimaryPlayer>>();
        if q.iter(app.world()).next().is_some() {
            ambition_platformer2d::scripted_input::drive_the_local_participant(&mut app);
            return app;
        }
    }
    panic!("the demo never activated a playable body");
}

fn step(app: &mut App, frame: ControlFrame) {
    app.world_mut()
        .resource_mut::<ambition_platformer2d::scripted_input::ScriptedControls>()
        .0 = frame;
    app.update();
}

fn mode(app: &mut App) -> ae::player_state::BodyMode {
    let world = app.world_mut();
    let mut q = world
        .query_filtered::<&ae::body_clusters::BodyModeState, With<PrimaryPlayer>>();
    q.iter(world).next().expect("a player").body_mode
}

fn can_morph(app: &mut App) -> bool {
    let world = app.world_mut();
    let mut q = world.query_filtered::<&BodyAbilities, With<PrimaryPlayer>>();
    q.iter(world).next().expect("a player").abilities.morph
}

/// Settle on the ground, then double-tap DOWN. Returns every mode she passed
/// through.
fn double_tap_down(app: &mut App) -> Vec<ae::player_state::BodyMode> {
    for _ in 0..60 {
        step(app, ControlFrame::default());
    }
    let mut press = ControlFrame::default();
    press.axis_y = 1.0;
    press.down_pressed = true;
    let mut seen = Vec::new();
    for _ in 0..2 {
        step(app, press.clone());
        seen.push(mode(app));
        for _ in 0..3 {
            step(app, ControlFrame::default());
            seen.push(mode(app));
        }
    }
    for _ in 0..10 {
        step(app, ControlFrame::default());
        seen.push(mode(app));
    }
    seen
}

/// **MARY-O IN MARY-O CANNOT ENTER MORPH BALL.**
#[test]
fn mary_o_cannot_morph() {
    let mut app = boot();
    assert!(
        !can_morph(&mut app),
        "Mary-O's effective kit carries Morph Ball; only the Ambition game grants it"
    );
    let seen = double_tap_down(&mut app);
    assert!(
        !seen.contains(&ae::player_state::BodyMode::MorphBall),
        "her double-tap DOWN rolled her into a ball: {seen:?}"
    );

    // ANTI-VACUITY: the same gesture on the same body, with Morph Ball in its
    // authored base, does morph. Without this the refusal above could be an
    // airborne body or a swallowed tap.
    let mut app = boot();
    {
        let world = app.world_mut();
        let mut q = world.query_filtered::<&mut AbilityBase, With<PrimaryPlayer>>();
        q.iter_mut(world).next().expect("a player").abilities.morph = true;
    }
    let seen = double_tap_down(&mut app);
    assert!(
        seen.contains(&ae::player_state::BodyMode::MorphBall),
        "the double-tap never morphed even an entitled body, so the refusal is vacuous: {seen:?}"
    );
}
