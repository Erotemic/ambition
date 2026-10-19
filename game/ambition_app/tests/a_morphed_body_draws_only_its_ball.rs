//! A morphed body draws only its ball, in the shipped composition (reported
//! 2026-10-19: the standing sprite drew behind the ball).
//!
//! Two systems wrote the body sprite's `Visibility`: the morph-ball sync hid
//! it, and the submerged sync, which ran after it, set every hidden body that
//! was not submerged back to `Inherited`. So the sprite and the rig that
//! follows it drew behind the ball on each frame. One system now decides
//! whether a body's own presentation draws.

use bevy::prelude::*;
use leafwing_input_manager::prelude::Buttonlike;

use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::game_shell::ShellCommand;
use ambition_platformer2d::platformer::body::PrimaryBody;
use ambition_platformer2d::platformer::lifecycle::{PlayerVisual, PresentationOf};
use ambition_platformer2d::render::rendering::actors::rigged::RiggedPresentedBy;
use ambition_platformer2d::render::rendering::morph_ball::MorphBallVisual;

use crate::shell_host_rendered::{rendered_app, settle};

fn ambition_session() -> App {
    let mut app = rendered_app();
    settle(&mut app);
    crate::common::the_hub_intro_has_already_played(app.world_mut());
    app.world_mut().write_message(ShellCommand::GoTo(
        ambition_content::provider::AMBITION_GAMEPLAY_ROUTE.into(),
    ));
    settle(&mut app);
    for _ in 0..60 {
        app.update();
    }
    app
}

fn the_body(app: &mut App) -> Entity {
    let world = app.world_mut();
    world
        .query_filtered::<Entity, (With<PrimaryBody>, With<PlayerVisual>)>()
        .single(world)
        .expect("one primary body that is drawn")
}

fn mode(app: &App, body: Entity) -> ae::BodyMode {
    app.world()
        .get::<ae::body_clusters::BodyModeState>(body)
        .expect("a body mode")
        .body_mode
}

/// What draws: (the body's own sprite, its rig owner, its ball). `None` for a
/// rig or a ball that does not exist.
type Drawn = (Visibility, Option<Visibility>, Option<Visibility>);

fn drawn(app: &mut App, body: Entity) -> Drawn {
    let world = app.world_mut();
    let own = *world.get::<Visibility>(body).expect("the body sprite has a visibility");
    let rig = world
        .get::<RiggedPresentedBy>(body)
        .and_then(|owners| owners.iter().next())
        .and_then(|owner| world.get::<Visibility>(owner).copied());
    let ball = world
        .query_filtered::<(&PresentationOf, &Visibility), With<MorphBallVisual>>()
        .iter(world)
        .find(|(owner, _)| owner.0 == body)
        .map(|(_, visibility)| *visibility);
    (own, rig, ball)
}

fn tap(app: &mut App, key: KeyCode) {
    Buttonlike::press(&key, app.world_mut());
    app.update();
    Buttonlike::release(&key, app.world_mut());
    for _ in 0..3 {
        app.update();
    }
}

/// ENTRY, A SUSTAINED MORPH, EXIT. Control: standing, the body sprite draws
/// and no ball does. Morphed, for each of 30 frames, the sprite and its rig
/// are hidden and the ball draws. Standing again, the sprite draws and the
/// ball is hidden.
#[test]
fn a_morphed_body_hides_its_sprite_and_rig_and_draws_its_ball() {
    let mut app = ambition_session();
    let body = the_body(&mut app);
    assert_eq!(mode(&app, body), ae::BodyMode::Standing, "premise: the body stands");
    let standing = drawn(&mut app, body);
    assert!(
        standing.0 != Visibility::Hidden && standing.1 != Some(Visibility::Hidden) && standing.2 != Some(Visibility::Visible),
        "control: a standing body draws its sprite and no ball: {standing:?}"
    );

    for _ in 0..2 {
        tap(&mut app, KeyCode::ArrowDown);
    }
    assert_eq!(mode(&app, body), ae::BodyMode::MorphBall, "premise: a grounded double tap of Down morphs the body");
    for frame in 0..30 {
        app.update();
        let morphed = drawn(&mut app, body);
        assert_eq!(
            (morphed.0, morphed.1.unwrap_or(Visibility::Hidden), morphed.2),
            (Visibility::Hidden, Visibility::Hidden, Some(Visibility::Visible)),
            "frame {frame} of the morph: (body sprite, rig, ball) is {morphed:?}"
        );
    }

    tap(&mut app, KeyCode::ArrowUp);
    for _ in 0..3 {
        app.update();
    }
    assert_eq!(mode(&app, body), ae::BodyMode::Standing, "premise: Up stands the body back up");
    let stood = drawn(&mut app, body);
    assert!(
        stood.0 == Visibility::Inherited && stood.1 != Some(Visibility::Hidden) && stood.2 == Some(Visibility::Hidden),
        "after the morph: (body sprite, rig, ball) is {stood:?}"
    );
}
