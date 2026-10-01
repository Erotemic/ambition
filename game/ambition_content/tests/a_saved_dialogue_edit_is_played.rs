//! ⭐ THE RUNNING GAME PLAYS A SAVED DIALOGUE EDIT (`content_watch::yarn`): a
//! Yarn file saved while the project runs is compiled with the rest and taken
//! by the running project; an edit that does not compile changes nothing.
#![cfg(all(feature = "ui", not(feature = "static_content")))]

use ambition_content::content_watch::{watch_yarn_sources, YarnSourceWatch};
use bevy::prelude::*;
use bevy_yarnspinner::events::PresentLine;
use bevy_yarnspinner::prelude::*;

const NAME: &str = "sandbox.yarn";

fn source(line: &str) -> String {
    format!("title: Start\n---\n{line}\n===\n")
}

#[derive(Resource, Default)]
struct PresentedLines(Vec<String>);

fn record_line(event: On<PresentLine>, mut lines: ResMut<PresentedLines>) {
    lines.0.push(event.line.text.clone());
}

/// Run `Start` to its end and return the lines it presented.
fn play(app: &mut App) -> Vec<String> {
    app.world_mut().resource_mut::<PresentedLines>().0.clear();
    let mut state: bevy::ecs::system::SystemState<(Commands, Res<YarnProject>)> =
        bevy::ecs::system::SystemState::new(app.world_mut());
    let (mut commands, project) = state.get_mut(app.world_mut()).expect("yarn params");
    let runner = project.create_dialogue_runner(&mut commands);
    state.apply(app.world_mut());
    let entity = app.world_mut().spawn(runner).id();
    app.world_mut().get_mut::<DialogueRunner>(entity).unwrap().start_node("Start");
    app.update();
    for _ in 0..4 {
        let mut runner = app.world_mut().get_mut::<DialogueRunner>(entity).unwrap();
        if !runner.is_running() {
            break;
        }
        runner.continue_in_next_update();
        app.update();
    }
    app.world_mut().despawn(entity);
    app.world().resource::<PresentedLines>().0.clone()
}

/// The text the running project holds for the file.
fn project_text(app: &App) -> String {
    let project = app.world().resource::<YarnProject>();
    let assets = app.world().resource::<Assets<YarnFile>>();
    let handle = project.yarn_files().next().expect("the project has its file");
    assets.get(handle).expect("loaded").content().to_string()
}

/// Save `text`, let the watch look, and let the project recompile.
fn save(app: &mut App, path: &std::path::Path, text: &str) {
    std::thread::sleep(std::time::Duration::from_millis(20));
    std::fs::write(path, text).unwrap();
    app.world_mut().resource_mut::<YarnSourceWatch>().poll_now();
    for _ in 0..4 {
        app.update();
    }
}

#[test]
fn a_saved_dialogue_edit_is_played_and_a_broken_one_is_refused() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join(NAME);
    std::fs::write(&path, source("Hello.")).unwrap();

    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(AssetPlugin {
        watch_for_changes_override: Some(false),
        ..default()
    });
    app.add_plugins(YarnSpinnerPlugin::with_yarn_source(YarnFileSource::InMemory(YarnFile::new(
        NAME,
        source("Hello."),
    ))));
    app.init_resource::<PresentedLines>();
    app.add_observer(record_line);
    app.insert_resource(YarnSourceWatch::new(root.path().to_path_buf(), [NAME.to_string()]));
    app.add_systems(Update, watch_yarn_sources.run_if(resource_exists::<YarnProject>));
    while !app.world().contains_resource::<YarnProject>() {
        app.update();
    }
    assert_eq!(play(&mut app), ["Hello."], "the premise");

    save(&mut app, &path, &source("Goodbye."));
    assert_eq!(play(&mut app), ["Goodbye."], "the saved line plays");
    assert_eq!(app.world().resource::<YarnSourceWatch>().reloaded, 1);

    // Two ways to be broken: the strings pass refuses an unclosed block; only
    // the whole-project compile refuses a type error.
    const UNCLOSED: &str = "title: Start\n---\n<<if true>>\nNever closed.\n===\n";
    const MISTYPED: &str = "title: Start\n---\n<<declare $x = \"a\">>\n<<set $x to 1>>\nNo.\n===\n";
    for broken in [UNCLOSED, MISTYPED] {
        assert!(
            ambition_content::content_watch::replace_yarn_sources(app.world_mut(), vec![(NAME.into(), broken.into())])
                .is_err(),
            "the premise: {broken:?} does not compile"
        );
        // ⛔ THE TEXT TOO, not only the program: bevy_yarnspinner's own
        // recompile refuses a broken file and keeps the old program, so the
        // played line alone cannot tell a refused edit from a half-applied
        // one, whose asset holds text the program was not compiled from.
        assert_eq!(project_text(&app), source("Goodbye."), "a refused edit leaves the file's text");
        save(&mut app, &path, broken);
        assert_eq!(project_text(&app), source("Goodbye."), "a refused save leaves the file's text");
        assert_eq!(play(&mut app), ["Goodbye."], "a broken save changes nothing");
    }
    assert_eq!(app.world().resource::<YarnSourceWatch>().reloaded, 1, "and is not counted as taken");
}
