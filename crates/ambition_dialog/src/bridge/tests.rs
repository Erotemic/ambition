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
            "title: final\n---\nGuide: Final line.\n===\ntitle: choice\n---\nGuide: Pick one.\n-> A\n    Guide: One.\n-> B\n    Guide: Two.\n===\n",
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
fn automatic_dialogue_waits_then_closes_its_final_line_without_input() {
    let mut app = final_line();
    app.add_systems(
        Update,
        crate::dialog_reveal_tick.in_set(crate::DialogFrameSet::Input),
    );
    app.world_mut()
        .resource_mut::<DialogState>()
        .start_with_settings(
            "final",
            "Guide",
            crate::DialogueContext::scripted(),
            ambition_input::InputOwner::Primary,
            crate::DialogSettings {
                text_reveal: crate::TextReveal::Instant,
                line_advance: crate::LineAdvance::Automatic { delay_seconds: 0.1 },
                ..Default::default()
            },
        );
    for _ in 0..3 {
        app.update();
    }
    assert!(app.world().resource::<DialogState>().active());
    for _ in 0..10 {
        app.update();
    }
    assert!(!app.world().resource::<DialogState>().active());
    app.world_mut()
        .resource_mut::<DialogState>()
        .start_with_settings(
            "choice",
            "Guide",
            crate::DialogueContext::scripted(),
            ambition_input::InputOwner::Primary,
            crate::DialogSettings {
                text_reveal: crate::TextReveal::Instant,
                line_advance: crate::LineAdvance::Automatic { delay_seconds: 0.0 },
                ..Default::default()
            },
        );
    for _ in 0..20 {
        app.update();
    }
    let state = app.world().resource::<DialogState>();
    assert!(state.active());
    assert_eq!(state.current_options.len(), 2);
    assert!(state.pending_select.is_none());
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

/// What `<<mark>>` heard: its argument and the runner that ran it.
#[derive(Resource, Default)]
struct Marks(Vec<(String, Option<Entity>)>);

fn install_mark(commands: &mut Commands, runner: &mut DialogueRunner, _: &crate::YarnStateMirror) {
    let mark = commands.register_system(
        |In(tag): In<String>, speaking: Res<SpeakingRunner>, mut marks: ResMut<Marks>| marks.0.push((tag, speaking.0)),
    );
    runner.commands_mut().add_command("mark", mark);
}

/// SIMULATION-ISLANDS N2: two conversations run at once, each on its own
/// runner. A command names the runner that ran it, each runner reads its own
/// participants, and a variable one sets the other reads: the store is
/// shared, and a runner made later does not reset it to the declared values.
#[test]
fn two_conversations_run_at_once_each_on_its_own_runner() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(AssetPlugin::default());
    app.add_message::<ambition_sfx::OwnedSfxMessage>();
    app.add_plugins((crate::DialogSimStatePlugin, crate::YarnBindingsPlugin, crate::YarnBridgePlugin));
    app.init_resource::<Marks>();
    app.world_mut().resource_mut::<crate::YarnContentBindings>().installers.push(install_mark);
    app.add_plugins(YarnSpinnerPlugin::with_yarn_source(YarnFileSource::InMemory(YarnFile::new(
        "two_runners.yarn",
        "title: shop\n---\n<<declare $listener_id = \"\">>\n<<declare $met = false>>\n\
         <<set $met to true>>\n<<mark shop>>\nMerchant: Hello {$listener_id}.\n===\n\
         title: forge\n---\n<<mark forge>>\nSmith: Met {$met}, {$listener_id}.\n===\n",
    ))));
    for _ in 0..1000 {
        app.update();
        if app.world().contains_resource::<DialogueRunnerEntity>() {
            break;
        }
        std::thread::yield_now();
    }
    let main = app.world().resource::<DialogueRunnerEntity>().0;
    let owner = |seat| ambition_input::InputOwner::Participant(ambition_input::ParticipantId(seat));
    app.world_mut().resource_mut::<DialogState>().start("shop", "Merchant", crate::DialogueContext::between("robot", "alice"), owner(0));
    for _ in 0..5 {
        app.update();
    }
    assert_eq!(app.world().resource::<DialogState>().current_line, "Hello alice.", "the main runner reads its own listener");

    let mut forge = DialogSlot::default();
    forge.0.start("forge", "Smith", crate::DialogueContext::between("robot", "bob"), owner(1));
    let slot = app.world_mut().spawn(forge).id();
    for _ in 0..5 {
        app.update();
    }
    assert!(app.world().get::<DialogueRunner>(slot).is_some(), "the slot has a runner of its own");
    let slot_state = &app.world().get::<DialogSlot>(slot).unwrap().0;
    assert_eq!(slot_state.current_line, "Met true, bob.", "the slot reads the shop's $met and its own listener");
    assert_eq!(app.world().resource::<DialogState>().current_line, "Hello alice.", "the main conversation is untouched");
    assert_eq!(
        app.world().resource::<Marks>().0,
        vec![("shop".to_owned(), Some(main)), ("forge".to_owned(), Some(slot))],
        "each command named the runner that ran it",
    );
    assert_eq!(app.world().resource::<SpeakingRunner>().0, None, "no runner speaks outside a call");
}
