//! A real Yarn interpreter, driven from a test, with a chosen vocabulary.
//!
//! shared by both authored-logic verbs, and extracted the moment the
//! second one wanted it. Calling `install_*_binding` directly from a test would
//! prove the function works and prove nothing about whether an authored line can
//! reach it — the interpreter's dispatch, its arity rules and its value handling
//! are exactly the parts that were believed impossible and were not.
//!
//! the vocabulary arrives through [`ambition_dialog::YarnContentBindings`],
//! the same installer seam the production plugin pushes into, so a change that
//! broke the real installation breaks these tests too.

use bevy::prelude::*;
use bevy_yarnspinner::events::PresentLine;
use bevy_yarnspinner::prelude::*;

/// Every line the interpreter presented, in order.
#[derive(Resource, Default)]
pub struct PresentedLines(pub Vec<String>);

fn record_line(event: On<PresentLine>, mut lines: ResMut<PresentedLines>) {
    lines.0.push(event.line.text.clone());
}

/// Build an app running `source`, whose dialogue vocabulary is exactly
/// `installers`.
pub fn app_running(source: &str, installers: &[ambition_dialog::YarnBindingInstaller]) -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(AssetPlugin {
        watch_for_changes_override: Some(false),
        ..default()
    });
    app.add_plugins(YarnSpinnerPlugin::with_yarn_source(
        YarnFileSource::InMemory(YarnFile::new("authored_logic_test.yarn", source)),
    ));
    app.init_resource::<PresentedLines>();
    app.init_resource::<ambition_dialog::YarnStateMirror>();
    app.init_resource::<ambition_dialog::YarnContentBindings>();
    app.world_mut()
        .resource_mut::<ambition_dialog::YarnContentBindings>()
        .installers
        .extend_from_slice(installers);
    app.add_observer(record_line);
    app
}

fn runner_entity(app: &mut App) -> Entity {
    if let Some(entity) = app
        .world_mut()
        .query_filtered::<Entity, With<DialogueRunner>>()
        .iter(app.world())
        .next()
    {
        return entity;
    }
    while !app.world().contains_resource::<YarnProject>() {
        app.update();
    }
    let mirror = app
        .world()
        .resource::<ambition_dialog::YarnStateMirror>()
        .clone();
    let installers = app
        .world()
        .resource::<ambition_dialog::YarnContentBindings>()
        .installers
        .clone();
    let mut system_state: bevy::ecs::system::SystemState<(Commands, Res<YarnProject>)> =
        bevy::ecs::system::SystemState::new(app.world_mut());
    let (mut commands, project) = system_state
        .get_mut(app.world_mut())
        .expect("yarn harness params");
    let mut runner = project.create_dialogue_runner(&mut commands);
    for install in &installers {
        install(&mut commands, &mut runner, &mirror);
    }
    system_state.apply(app.world_mut());
    app.world_mut().spawn(runner).id()
}

/// Spawn the runner if needed, install the vocabulary, and run the node to its
/// first line.
pub fn start(app: &mut App, node: &str) {
    let entity = runner_entity(app);
    let mut runner = app
        .world_mut()
        .get_mut::<DialogueRunner>(entity)
        .expect("runner");
    if runner.is_running() {
        runner.stop();
    }
    runner.start_node(node);
    app.update();
}

/// Advance one beat.
pub fn advance(app: &mut App) {
    let entity = runner_entity(app);
    app.world_mut()
        .get_mut::<DialogueRunner>(entity)
        .expect("runner")
        .continue_in_next_update();
    app.update();
}

pub fn lines(app: &App) -> Vec<String> {
    app.world().resource::<PresentedLines>().0.clone()
}
