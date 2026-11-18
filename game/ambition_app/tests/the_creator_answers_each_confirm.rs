//! The Creator's displayed line responds to each confirm from the keyboard.

use ambition_app::app::{
    build_visible_app_with, StartRoomMustResolve, StartRoomOverride, VisibleRenderMode,
};
use ambition_platformer2d::cutscene::ActiveCutscene;
use ambition_platformer2d::sim_view::DialogView;
use bevy::prelude::*;
use leafwing_input_manager::prelude::Buttonlike;

fn opening(frame_hz: f64) -> App {
    opening_at(frame_hz, true)
}

fn opening_at(frame_hz: f64, complete: bool) -> App {
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
        if view.active && view.speaker_character_id == "npc_creator"
            && (!complete || view.body == "Hey you, you're finally awake.") {
            return app;
        }
    }
    panic!("the opening never showed the Creator's first line");
}

fn wait_for_complete_line(app: &mut App) {
    for _ in 0..600 {
        if app.world().resource::<DialogView>().confirm_action == ambition_platformer2d::dialog::DialogConfirmAction::Advance { return; }
        app.update();
    }
    panic!("the line did not finish revealing");
}

#[test]
fn the_creators_triggered_line_types_and_f_reveals_before_advancing() {
    let mut app = opening_at(60.0, false);
    wait(&mut app, 2);
    let first = "Hey you, you're finally awake.";
    let shown = &app.world().resource::<DialogView>().body;
    assert!(!shown.is_empty() && shown != first && first.starts_with(shown));
    assert_eq!(app.world().resource::<DialogView>().confirm_action, ambition_platformer2d::dialog::DialogConfirmAction::Reveal);
    confirm(&mut app);
    assert_eq!(app.world().resource::<DialogView>().body, first);
    confirm(&mut app);
    wait(&mut app, 2);
    assert!("Don't sit up fast.".starts_with(&app.world().resource::<DialogView>().body));
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
        wait_for_complete_line(&mut app);
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
    wait_for_complete_line(&mut app);
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

fn talk_to_creator(app: &mut App) {
    use crate::alices_reading_reaches_bob::{npc, put_body_at};
    let creator = npc(app, "npc_creator");
    let at = app
        .world()
        .get::<ambition_platformer2d::engine_core::BodyKinematics>(creator)
        .unwrap()
        .pos;
    let player = crate::neighbor_prefetch_prepares_rooms::alice(app);
    put_body_at(app, player, at);
    wait(app, 10);
    confirm(app);
    wait(app, 120);
}

fn choose(app: &mut App, label: &str) {
    let choices = &app.world().resource::<DialogView>().option_labels;
    let index = choices
        .iter()
        .position(|text| text == label)
        .unwrap_or_else(|| panic!("no {label:?} in {choices:?}"));
    for _ in 0..index {
        Buttonlike::press(&KeyCode::ArrowDown, app.world_mut());
        app.update();
        Buttonlike::release(&KeyCode::ArrowDown, app.world_mut());
        app.update();
    }
    confirm(app);
    wait(app, 120);
}

fn finish_conversation(app: &mut App) {
    for _ in 0..30 {
        if !app
            .world()
            .resource::<ambition_platformer2d::dialog::DialogState>()
            .active()
        {
            return;
        }
        confirm(app);
        wait(app, 120);
    }
    panic!("the conversation did not end");
}

#[test]
fn the_creator_offers_optional_practice_before_the_raid() {
    for accept in [true, false] {
        let mut app = opening(60.0);
        confirm(&mut app);
        wait_for_complete_line(&mut app);
        confirm(&mut app);
        wait(&mut app, 30);
        talk_to_creator(&mut app);
        confirm(&mut app);
        wait(&mut app, 120);
        choose(
            &mut app,
            if accept {
                "Yes. Show me the basics."
            } else {
                "I remember. Ask your questions."
            },
        );
        finish_conversation(&mut app);
        wait(&mut app, 240);
        let save = app
            .world()
            .resource::<ambition_platformer2d::persistence::save::AmbitionGameSave>();
        assert_eq!(save.data().flag("intro_training_active"), accept);
        assert!(
            !save.data().flag("intro_raiders_coming"),
            "neither branch starts a fight during the conversation"
        );
        if !accept {
            assert!(save.data().flag("intro_questions_done"));
            let creator = crate::alices_reading_reaches_bob::npc(&mut app, "npc_creator");
            for _ in 0..1200 {
                let visit = app
                    .world()
                    .get::<ambition_platformer2d::actors::body::visit::Visit>(creator);
                if visit.is_some_and(|visit| {
                    visit.outcome
                        == ambition_platformer2d::actors::body::visit::VisitOutcome::Arrived
                }) {
                    break;
                }
                app.update();
            }
            talk_to_creator(&mut app);
            choose(&mut app, "I'm ready to finish.");
            finish_conversation(&mut app);
            for _ in 0..600 {
                app.update();
                if app.world().resource::<ActiveCutscene>().script_id()
                    == Some("intro_raid_at_the_door")
                {
                    break;
                }
            }
            assert_eq!(
                app.world().resource::<ActiveCutscene>().script_id(),
                Some("intro_raid_at_the_door")
            );
        }
    }
}
