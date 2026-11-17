//! The Creator's displayed line responds to each confirm from the keyboard.

use ambition_app::app::{
    build_visible_app_with, StartRoomMustResolve, StartRoomOverride, VisibleRenderMode,
};
use ambition_platformer2d::cutscene::ActiveCutscene;
use ambition_platformer2d::sim_view::DialogView;
use bevy::prelude::*;
use leafwing_input_manager::prelude::Buttonlike;

fn opening(frame_hz: f64) -> App {
    let mut app = build_visible_app_with(VisibleRenderMode::NoWindow, false, |app| {
        app.insert_resource(StartRoomOverride("intro_wake_room".to_owned()));
        app.insert_resource(StartRoomMustResolve);
    });
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_secs_f64(1.0 / frame_hz),
    ));
    for _ in 0..600 {
        app.update();
        let view = app.world().resource::<DialogView>();
        if view.active && view.body == "Hey you, you're finally awake." {
            return app;
        }
    }
    panic!("the opening never showed the Creator's first line");
}

fn wait(app: &mut App, frames: usize) {
    for _ in 0..frames {
        app.update();
    }
}

fn confirm(app: &mut App) {
    Buttonlike::press(&KeyCode::KeyF, app.world_mut());
    app.update();
    Buttonlike::release(&KeyCode::KeyF, app.world_mut());
    app.update();
}

fn assert_creator_portrait(app: &mut App) {
    assert_eq!(
        app.world().resource::<DialogView>().speaker_character_id,
        "npc_creator"
    );
    let world = app.world_mut();
    let image = world
        .query::<(&Name, &ImageNode)>()
        .iter(world)
        .find(|(name, _)| name.as_str() == "Dialogue Portrait Image")
        .map(|(_, image)| image.image.clone())
        .expect("the Creator's box shows a portrait image");
    let path = world
        .resource::<AssetServer>()
        .get_path(image.id())
        .expect("the portrait has an asset path");
    assert_eq!(
        path.path().file_stem().unwrap().to_str(),
        Some("creator_portraits")
    );
}

#[test]
fn the_creators_cutscene_uses_one_press_per_shown_line() {
    for frame_hz in [30.0, 60.0, 144.0] {
        let mut app = opening(frame_hz);
        assert_creator_portrait(&mut app);
        wait(&mut app, frame_hz as usize);
        confirm(&mut app);
        wait(&mut app, 8);
        let view = app.world().resource::<DialogView>();
        assert!(
            view.body.starts_with("Don't sit up fast."),
            "{frame_hz} Hz: one F left {view:?}"
        );
        wait(&mut app, frame_hz as usize);
        confirm(&mut app);
        wait(&mut app, 12);
        assert!(
            !app.world().resource::<ActiveCutscene>().is_playing(),
            "{frame_hz} Hz: the second F did not end the opening"
        );
    }
}

#[test]
fn the_creators_conversation_reveals_and_advances_the_shown_text() {
    use crate::alices_reading_reaches_bob::{npc, put_body_at};
    use crate::neighbor_prefetch_prepares_rooms::alice;
    let mut app = opening(60.0);
    confirm(&mut app);
    wait(&mut app, 10);
    confirm(&mut app);
    wait(&mut app, 30);
    let creator = npc(&mut app, "npc_creator");
    let at = app
        .world()
        .get::<ambition_platformer2d::engine_core::BodyKinematics>(creator)
        .unwrap()
        .pos;
    let player = alice(&mut app);
    put_body_at(&mut app, player, at);
    wait(&mut app, 10);
    confirm(&mut app);
    let mut bodies = Vec::new();
    for _ in 0..120 {
        let view = app.world().resource::<DialogView>();
        if view.active {
            bodies.push(view.body.clone());
        }
        app.update();
    }
    assert!(
        bodies.iter().any(|body| !body.is_empty()
            && "Can you hear me all right? Good.".starts_with(body)
            && body != "Can you hear me all right? Good."),
        "no partial text reached the view: {bodies:?}"
    );
    let before = app.world().resource::<DialogView>().body.clone();
    assert_creator_portrait(&mut app);
    confirm(&mut app);
    wait(&mut app, 120);
    let view = app.world().resource::<DialogView>();
    assert_ne!(
        view.body, before,
        "F on a complete line did not change the view: {view:?}"
    );
    assert!(
        !view.option_labels.is_empty(),
        "the Creator's choices did not appear: {view:?}"
    );
}
