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

use ambition_engine_core as ae;
use ambition_engine_core::RoomGeometry;
use ambition_runtime::demo_fixture::{RoomSet, RoomSpec};

/// The demo content plugin: one empty room + the engine's own sim-world
/// setup (spawns the player box). This is the shape every demo app copies.
struct FixtureContentPlugin;

/// A one-character catalog: the demo's content choice. Every demo installs
/// its own roster; the engine ships none (ADR 0017).
const FIXTURE_CATALOG_RON: &str = r#"(
    brain_presets: { "stand_still": StandStill },
    action_set_presets: {
        "peaceful": (
            move_style: Walk,
            melee: None,
            ranged: None,
            special: None,
        ),
    },
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
            // This fixture intentionally exercises the host-built protagonist
            // kit. Declare that ownership explicitly; malformed Authored rows
            // must never gain host capabilities by falling through.
            playable_kit: HostCode,
            tags: ["player"],
        ),
    },
)"#;

impl Plugin for FixtureContentPlugin {
    fn build(&self, app: &mut App) {
        use ambition_characters::actor::character_catalog::{
            CharacterCatalogAppExt, CharacterCatalogFragment,
        };
        app.register_character_catalog_fragment(
            CharacterCatalogFragment::from_ron("fixture", Some("player"), FIXTURE_CATALOG_RON)
                .expect("fixture character catalog should be valid"),
        );
        ambition_runtime::demo_fixture::install_character_catalog(FIXTURE_CATALOG_RON);
        let world = ae::World::new(
            "fixture_room",
            ae::Vec2::new(640.0, 480.0),
            ae::Vec2::new(96.0, 96.0),
            Vec::new(),
        );
        let room = RoomSpec::new("fixture_room", world.clone());
        app.insert_resource(RoomGeometry(world));
        app.insert_resource(ambition_runtime::demo_fixture::ActiveRoomMetadata::default());
        app.insert_resource(RoomSet::from_parts("fixture_room", vec![room], Vec::new()));
        app.add_systems(
            Startup,
            fixture_setup.in_set(ambition_runtime::demo_fixture::SimulationSetupSet),
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
    ldtk_index: Res<ambition_runtime::demo_fixture::LdtkRuntimeIndex>,
    editable_abilities: Res<ambition_runtime::demo_fixture::EditableAbilitySet>,
    editable_tuning: Res<ambition_runtime::demo_fixture::EditableMovementTuning>,
    starting_character: Res<ambition_runtime::demo_fixture::StartingCharacter>,
    character_catalog: Res<ambition_characters::actor::character_catalog::CharacterCatalog>,
    asset_server: Res<AssetServer>,
) {
    ambition_runtime::demo_fixture::simulation_world(
        &mut commands,
        ambition_platformer_primitives::lifecycle::SessionSpawnScope::UNSCOPED,
        ambition_runtime::demo_fixture::SimulationSetup {
            world: &world,
            room_set: &room_set,
            ldtk_index: &ldtk_index,
            editable_abilities: &editable_abilities,
            editable_tuning: &editable_tuning,
            starting_character: &starting_character,
            character_catalog: &character_catalog,
            default_character_id: "player",
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
    app.add_plugins(ambition_runtime::PlatformerEnginePlugins::default());
    app.add_plugins(ambition_host::PlatformerHostPlugins);
    app.add_plugins(FixtureContentPlugin);

    // First update runs Startup; a couple more prove the sim loop holds.
    app.update();
    app.update();
    app.update();
}

// ─────────────────────────────────────────────────────────────────────────────
// Netcode N0.1 — the same shell, hosted on the fixed-tick clock.
//
// The exit check has two halves. Here: the demo assembly boots in `FixedUpdate`
// and the sim graph does not SPLIT across two schedules. In
// `game/ambition_app/tests/{player,actor}_phase_split.rs`: the rl_sim
// schedule-shape suites pass with the label threaded BOTH ways.
// ─────────────────────────────────────────────────────────────────────────────

use ambition_platformer_primitives::schedule::SandboxSet;
use ambition_runtime::SimTick;
use bevy::ecs::schedule::Schedules;
use bevy::time::{Fixed, Time, TimeUpdateStrategy};

/// Every sim phase. `PresentationVisualSync` is deliberately absent: it is the
/// one presentation-side label in `SandboxSet`, and render joins it in `Update`.
const SIM_PHASES: &[SandboxSet] = &[
    SandboxSet::CoreSimulation,
    SandboxSet::WorldPrep,
    SandboxSet::PlayerInput,
    SandboxSet::PlayerSimulation,
    SandboxSet::RoomTransition,
    SandboxSet::Combat,
    SandboxSet::PresentationSync,
    SandboxSet::FeatureCollection,
    SandboxSet::FeatureInteraction,
    SandboxSet::LdtkRuntimeSpine,
    SandboxSet::EncounterSimulation,
    SandboxSet::Cutscene,
    SandboxSet::GameplayEffects,
    SandboxSet::Progression,
    SandboxSet::ResetProcessing,
    SandboxSet::FeatureViewSync,
    SandboxSet::Trace,
];

fn systems_in(
    app: &App,
    schedule: impl bevy::ecs::schedule::ScheduleLabel,
    set: SandboxSet,
) -> usize {
    let schedules = app.world().resource::<Schedules>();
    let Some(graph) = schedules.get(schedule).map(|s| s.graph()) else {
        return 0;
    };
    // `SetNotFound` means the set has no node in this schedule at all — which is
    // exactly "no systems", not a failure. A node with zero members reads the
    // same. (The host's `.before(CoreSimulation)` edges DO create empty nodes in
    // `Update`, which is why MEMBERSHIP, not existence, is what this asserts.)
    graph.systems_in_set(set.intern()).map_or(0, |s| s.len())
}

/// Build the shell and run Bevy's Startup frame.
///
/// Bevy's very first frame has `dt == 0`, so the fixed accumulator expends
/// nothing: `Startup` runs, the sim does not. Every frame after it advances
/// exactly one tick, because the frame dt is pinned to the tick dt (identical
/// `Duration`s, hence integer nanoseconds, hence no accumulator drift ever).
fn fixed_tick_shell() -> App {
    let mut app = App::new();
    ambition_runtime::add_headless_foundation(&mut app);
    app.add_plugins(ambition_runtime::PlatformerEnginePlugins::fixed_tick());
    app.add_plugins(ambition_host::PlatformerHostPlugins);
    app.add_plugins(FixtureContentPlugin);
    let timestep = app.world().resource::<Time<Fixed>>().timestep();
    app.insert_resource(TimeUpdateStrategy::ManualDuration(timestep));
    app.update(); // Startup; zero ticks.
    app
}

#[test]
fn fixed_tick_demo_shell_boots_and_ticks() {
    let mut app = fixed_tick_shell();
    assert_eq!(
        app.world().resource::<SimTick>().get(),
        0,
        "Startup alone must not advance the timeline"
    );

    for expected in 0..=5 {
        app.update();
        assert_eq!(
            app.world().resource::<SimTick>().get(),
            expected,
            "one frame at exactly the tick dt must expend exactly one tick"
        );
    }
}

/// The graph must not split. Under fixed tick, `Update` may still hold
/// presentation and device systems — but not one single system belonging to a
/// SIM phase. A content or engine plugin that hardcoded `Update` instead of
/// asking `app.sim_schedule()` would land its systems here, where they would
/// silently stop ordering against the rest of the sim.
#[test]
fn fixed_tick_leaves_no_sim_system_in_update() {
    let mut app = fixed_tick_shell();
    app.update(); // one real tick, so BOTH schedule graphs are initialized

    let mut stranded = Vec::new();
    for &phase in SIM_PHASES {
        let n = systems_in(&app, Update, phase);
        if n > 0 {
            stranded.push(format!("{phase:?} ({n} system(s))"));
        }
    }
    assert!(
        stranded.is_empty(),
        "sim systems stranded in `Update` under fixed tick: {}. \
         They were registered with a literal `Update` instead of \
         `app.sim_schedule()`, so they no longer order against the sim.",
        stranded.join(", "),
    );
}

/// ...and the phases really are populated on the other side.
#[test]
fn fixed_tick_puts_the_sim_phases_in_fixed_update() {
    let mut app = fixed_tick_shell();
    app.update(); // one real tick, so the FixedUpdate graph is initialized

    for phase in [
        SandboxSet::PlayerInput,
        SandboxSet::WorldPrep,
        SandboxSet::Combat,
        SandboxSet::FeatureViewSync,
    ] {
        assert!(
            systems_in(&app, FixedUpdate, phase) > 0,
            "{phase:?} must carry systems in FixedUpdate under fixed tick"
        );
    }
}

/// Frame-stepped is the default and is unchanged: the sim lives in `Update`,
/// and `FixedUpdate` carries nothing of ours.
#[test]
fn frame_stepped_shell_keeps_the_sim_in_update() {
    let mut app = App::new();
    ambition_runtime::add_headless_foundation(&mut app);
    app.add_plugins(ambition_runtime::PlatformerEnginePlugins::default());
    app.add_plugins(ambition_host::PlatformerHostPlugins);
    app.add_plugins(FixtureContentPlugin);
    app.update();

    assert!(systems_in(&app, Update, SandboxSet::WorldPrep) > 0);
    assert_eq!(systems_in(&app, FixedUpdate, SandboxSet::WorldPrep), 0);
    // The timeline advances in both modes.
    assert_eq!(app.world().resource::<SimTick>().get(), 0);
    app.update();
    assert_eq!(app.world().resource::<SimTick>().get(), 1);
}

/// Choosing the mode after a sim plugin has already committed systems is the
/// one way to get a split graph. It must be loud, not silent.
#[test]
#[should_panic(expected = "sim schedule already sealed")]
fn changing_the_sim_schedule_after_a_sim_plugin_panics() {
    use ambition_platformer_primitives::schedule::SimScheduleExt as _;
    let mut app = App::new();
    ambition_runtime::add_headless_foundation(&mut app);
    app.add_plugins(ambition_runtime::PlatformerEnginePlugins::default());
    app.set_sim_schedule(FixedUpdate);
}
