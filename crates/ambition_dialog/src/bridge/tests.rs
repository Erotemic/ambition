//! Final text closes on its confirm and remains visible until that confirm.

use super::*;

fn final_line() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_secs_f64(1.0 / 60.0),
    ));
    app.add_plugins(AssetPlugin::default());
    app.add_message::<ambition_sfx::OwnedSfxMessage>();
    app.add_plugins((
        crate::DialogSimStatePlugin,
        crate::YarnBindingsPlugin,
        crate::YarnBridgePlugin,
    ));
    app.add_plugins(YarnSpinnerPlugin::with_yarn_source(
        YarnFileSource::InMemory(YarnFile::new(
            "final_line.yarn",
            "title: final\n---\nGuide: Final line.\n===\n",
        )),
    ));
    for _ in 0..1000 {
        app.update();
        if app.world().contains_resource::<DialogueRunnerEntity>() {
            break;
        }
        std::thread::yield_now();
    }
    assert!(app.world().contains_resource::<DialogueRunnerEntity>());
    app.world_mut().resource_mut::<DialogState>().start(
        "final",
        "Guide",
        crate::DialogueContext::scripted(),
        ambition_input::InputOwner::Primary,
    );
    for _ in 0..10 {
        app.update();
        if !app
            .world()
            .resource::<DialogState>()
            .current_line
            .is_empty()
        {
            break;
        }
    }
    assert_eq!(
        app.world().resource::<DialogState>().current_line,
        "Final line."
    );
    app.world_mut()
        .resource_mut::<DialogState>()
        .reveal_full_line();
    app
}

#[test]
fn confirming_the_final_line_closes_without_an_extra_press() {
    let mut app = final_line();
    app.world_mut()
        .resource_mut::<DialogState>()
        .confirm_or_advance();
    app.update();
    assert!(!app.world().resource::<DialogState>().active());
}

#[test]
fn automatic_completion_keeps_an_unconfirmed_final_line() {
    let mut app = final_line();
    app.world_mut()
        .resource_mut::<DialogState>()
        .pending_advance = true;
    app.update();
    let mut state = app.world_mut().resource_mut::<DialogState>();
    assert!(state.active());
    assert_eq!(state.body(), "Final line.");
    assert!(state.runner_done_pending_close);
    state.confirm_or_advance();
    assert!(!state.active());
}
