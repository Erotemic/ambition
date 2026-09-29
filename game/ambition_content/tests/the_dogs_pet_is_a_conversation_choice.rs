//! The companion dog's conversation offers the pet as a choice, and choosing
//! it runs `<<pet>>`.
//!
//! The authored node runs through the real Yarn interpreter, and the choice is
//! selected by its text, as a player selects it. `<<pet>>` is a stub that only
//! counts, because the command's own effect is witnessed in the shipped app
//! (`companion_dog.rs`), which has no dialog box to select a choice in. The two
//! together cover the road: the choice runs the command, and the command pets.
#![cfg(feature = "ui")]

use bevy::prelude::*;
use bevy_yarnspinner::events::PresentOptions;
use bevy_yarnspinner::prelude::*;

const NODE: &str = "hall_npc_companion_dog";

/// The dog's node, cut verbatim from the shipped file.
fn authored_node() -> String {
    let file = include_str!("../assets/dialogue/sandbox/hall.yarn");
    let start = file
        .find(&format!("title: {NODE}\n"))
        .expect("hall.yarn authors the dog's node");
    let end = start + file[start..].find("\n===").expect("the node ends") + "\n===\n".len();
    file[start..end].to_string()
}

#[derive(Resource, Default)]
struct Pets(usize);

#[derive(Resource, Default)]
struct Offered(Vec<(OptionId, String)>);

fn stub_pet(mut pets: ResMut<Pets>) {
    pets.0 += 1;
}

fn install_stub_pet(
    commands: &mut Commands,
    runner: &mut DialogueRunner,
    _mirror: &ambition_dialog::YarnStateMirror,
) {
    let id = commands.register_system(stub_pet);
    runner.commands_mut().add_command("pet", id);
}

fn record_options(event: On<PresentOptions>, mut offered: ResMut<Offered>) {
    offered.0 = event
        .options
        .iter()
        .map(|option| (option.id, option.line.text.clone()))
        .collect();
}

#[test]
fn choosing_pet_the_dog_runs_pet_and_the_other_choices_do_not() {
    for (choice, pets) in [("Pet the dog.", 1), ("Say hello.", 0)] {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(AssetPlugin {
            watch_for_changes_override: Some(false),
            ..default()
        });
        app.add_plugins(YarnSpinnerPlugin::with_yarn_source(YarnFileSource::InMemory(
            YarnFile::new("the_dog.yarn", authored_node()),
        )));
        app.init_resource::<Pets>();
        app.init_resource::<Offered>();
        app.add_observer(record_options);
        while !app.world().contains_resource::<YarnProject>() {
            app.update();
        }
        let mut state: bevy::ecs::system::SystemState<(Commands, Res<YarnProject>)> =
            bevy::ecs::system::SystemState::new(app.world_mut());
        let (mut commands, project) = state.get_mut(app.world_mut()).expect("yarn params");
        let mut runner = project.create_dialogue_runner(&mut commands);
        install_stub_pet(
            &mut commands,
            &mut runner,
            &ambition_dialog::YarnStateMirror::default(),
        );
        state.apply(app.world_mut());
        let runner = app.world_mut().spawn(runner).id();
        app.world_mut()
            .get_mut::<DialogueRunner>(runner)
            .expect("runner")
            .start_node(NODE);

        // Read the greeting until the choices are offered.
        for _ in 0..8 {
            app.update();
            if !app.world().resource::<Offered>().0.is_empty() {
                break;
            }
            app.world_mut()
                .get_mut::<DialogueRunner>(runner)
                .expect("runner")
                .continue_in_next_update();
        }
        let offered = app.world().resource::<Offered>().0.clone();
        let Some((id, _)) = offered.iter().find(|(_, text)| text == choice) else {
            panic!("the dog does not offer {choice:?}; it offers {offered:?}");
        };
        app.world_mut()
            .get_mut::<DialogueRunner>(runner)
            .expect("runner")
            .select_option(*id)
            .expect("the choice is selectable");
        for _ in 0..8 {
            app.update();
            let mut running = app.world_mut().get_mut::<DialogueRunner>(runner).expect("runner");
            if !running.is_running() {
                break;
            }
            running.continue_in_next_update();
        }
        assert_eq!(
            app.world().resource::<Pets>().0,
            pets,
            "choosing {choice:?} ran `<<pet>>` a different number of times"
        );
    }
}
