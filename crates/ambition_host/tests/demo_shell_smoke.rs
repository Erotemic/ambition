//! E5 step 6 — THE DEMO GATE, executable: a demo-shaped app assembles from
//! the engine group + the host group + a tiny fixture content plugin and
//! ticks without panicking. This is the permanent regression guard for the
//! demo gate: if an engine-group system grows a bare `Res<T>` on state only
//! Ambition's assembly provides, THIS test panics — move the default into
//! `SimCoreResourcesPlugin` (engine state) or document it as world/content
//! state the fixture must provide (like `RoomSet`/`RoomGeometry` below).
//!
//! The fixture provides exactly what the ENGINE deliberately does not own:
//! the installed WORLD (which rooms exist is the game's choice).

use bevy::prelude::*;

use ambition_actors::rooms::{RoomSet, RoomSpec};
use ambition_engine_core as ae;
use ambition_engine_core::RoomGeometry;

/// The demo content plugin: one empty room + the engine's own sim-world
/// setup (spawns the player box). This is the shape every demo app copies.
struct FixtureContentPlugin;

/// A one-character catalog: the demo's content choice. Every demo installs
/// its own roster; the engine ships none (ADR 0017).
const FIXTURE_CATALOG_RON: &str = r#"(
    brain_presets: { "stand_still": StandStill },
    action_set_presets: {},
    characters: {
        "player": (
            display_name: "Fixture Player",
            spritesheet: "sprites/fixture.png",
            manifest: "sprites/fixture.ron",
            tier: MainHall,
            body_kind: Standard,
            composition: None,
            default_brain: "stand_still",
            default_action_set: "peaceful",
            tags: ["player"],
        ),
    },
)"#;

impl Plugin for FixtureContentPlugin {
    fn build(&self, app: &mut App) {
        ambition_actors::character_roster::install_character_catalog(FIXTURE_CATALOG_RON);
        let world = ae::World::new(
            "fixture_room",
            ae::Vec2::new(640.0, 480.0),
            ae::Vec2::new(96.0, 96.0),
            Vec::new(),
        );
        let room = RoomSpec::new("fixture_room", world.clone());
        app.insert_resource(RoomGeometry(world));
        app.insert_resource(ambition_actors::rooms::ActiveRoomMetadata::default());
        app.insert_resource(RoomSet::from_parts("fixture_room", vec![room], Vec::new()));
        app.add_systems(
            Startup,
            fixture_setup.in_set(ambition_actors::schedule::SimulationSetupSet),
        );
    }
}

/// The demo's world construction: the engine's `simulation_world` with the
/// fixture room. Labeled `SimulationSetupSet` so the host's input attach
/// (and any other "after the world exists" startup work) orders correctly.
fn fixture_setup(
    mut commands: Commands,
    world: Res<RoomGeometry>,
    room_set: Res<RoomSet>,
    ldtk_index: Res<ambition_actors::ldtk_world::LdtkRuntimeIndex>,
    editable_abilities: Res<ambition_actors::dev::dev_tools::EditableAbilitySet>,
    editable_tuning: Res<ambition_actors::dev::dev_tools::EditableMovementTuning>,
    starting_character: Res<ambition_actors::player::StartingCharacter>,
    asset_server: Res<AssetServer>,
) {
    ambition_actors::session::setup::simulation_world(
        &mut commands,
        ambition_actors::session::setup::SimulationSetup {
            world: &world,
            room_set: &room_set,
            ldtk_index: &ldtk_index,
            editable_abilities: &editable_abilities,
            editable_tuning: &editable_tuning,
            starting_character: &starting_character,
            sandbox_data_asset: None,
            sandbox_asset_collection: None,
            asset_server: &asset_server,
        },
    );
}

#[test]
fn demo_shell_boots_and_ticks() {
    let mut app = App::new();
    ambition_runtime::add_headless_foundation(&mut app);
    app.add_plugins(ambition_runtime::PlatformerEnginePlugins);
    app.add_plugins(ambition_host::PlatformerHostPlugins);
    app.add_plugins(FixtureContentPlugin);

    // First update runs Startup; a couple more prove the sim loop holds.
    app.update();
    app.update();
    app.update();
}
