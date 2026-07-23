//! **Behavioral restore proof for Mary-O's demo sim state (Phase 5b).**
//!
//! The pin test (`rollback_registration.rs`) proves the NAMES reach the
//! registry; this proves the machinery: the demo boots under the ROLLBACK
//! engine host, a real GGRS sync-test session runs it, and a dirty
//! out-of-band mutation of [`MaryOLevelState`] is OVERWRITTEN by the next
//! rollback's restore. An unregistered type would keep the dirty value —
//! nothing in resimulation rewrites it — so this discriminates real
//! save→mutate→restore coverage from registration theater.

use ambition::game_shell::{
    ShellHostConfiguration, ShellHostSpec, ShellLaunchCatalog, ShellRouteCatalog, ShellRouteSpec,
};
use ambition_demo_mary_o::{MaryOExperiencePlugin, MaryOLevelState, MARY_O_GAMEPLAY_ROUTE};
use bevy::prelude::*;

/// The demo shell composed on the GGRS host instead of the fixed tick — the
/// same provider, the same shell wiring as `build_demo_app`, only the engine
/// group's host choice differs. Inline because the fixed-tick composer is
/// (rightly) private to the demo app crate.
fn build_rollback_demo_app() -> App {
    let mut app = App::new();
    ambition::engine::add_headless_foundation(&mut app);
    app.add_plugins(ambition::engine::PlatformerEnginePlugins::rollback());
    app.add_plugins(ambition::windowed_host::PlatformerHostPlugins);
    app.add_plugins(ambition::game_shell::MinimalShellPlugins);
    app.insert_resource(ambition::audio::selection::FrontendAudioProfile::new(
        ambition_demo_mary_o::MARY_O_EXPERIENCE,
    ));
    app.add_plugins(ambition::load::AmbitionLoadPlugin);
    app.add_plugins(ambition::load_presentation::MinimalShellLoadPresentationPlugins);
    app.add_plugins(MaryOExperiencePlugin);
    app.world_mut()
        .resource_mut::<ShellRouteCatalog>()
        .register(ShellRouteSpec::new(
            ambition_demo_mary_o::MARY_O_LAUNCHER_ROUTE,
            ShellLaunchCatalog::basic_experience_id(),
        ));
    app.world_mut()
        .resource_mut::<ShellHostConfiguration>()
        .spec = Some(ShellHostSpec::new(
        MARY_O_GAMEPLAY_ROUTE,
        ambition_demo_mary_o::MARY_O_LAUNCHER_ROUTE,
    ));
    let timestep = std::time::Duration::from_secs_f32(1.0 / 60.0);
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(timestep));
    app
}

fn level_state(app: &mut App) -> Option<MaryOLevelState> {
    let mut query = app.world_mut().query::<&MaryOLevelState>();
    query.iter(app.world()).next().copied()
}

#[test]
fn a_dirty_level_state_mutation_is_rolled_back_by_restore() {
    let mut app = build_rollback_demo_app();

    // Boot until the SHELL activates the gameplay session. This is the
    // Update-side fact; the sim-side world (rooms staged, the mode owner
    // spawned) cannot follow yet, because under the GGRS host the sim
    // advances only through session requests and no session exists.
    let mut activated = false;
    for _ in 0..600 {
        app.update();
        let session_active = app
            .world()
            .get_resource::<ambition::game_shell::ActiveGameplaySession>()
            .is_some_and(|session| session.0.is_some());
        if session_active {
            activated = true;
            break;
        }
    }
    assert!(
        activated,
        "the Mary-O shell never activated a gameplay session under the rollback host"
    );

    // A real sync-test session over the activated world: every update saves,
    // rolls back `check_distance` frames, and resimulates. GGRS now drives
    // the sim, which finishes staging the room and spawns the mode owner.
    ambition::runtime::rollback::start_sync_test_session(
        app.world_mut(),
        ambition::runtime::rollback::SyncTestSettings {
            check_distance: 4,
            max_prediction_window: 10,
        },
    )
    .expect("the demo composition starts a GGRS sync-test session");

    let mut owner_exists = false;
    for _ in 0..300 {
        app.update();
        if level_state(&mut app).is_some() {
            owner_exists = true;
            break;
        }
    }
    assert!(
        owner_exists,
        "the mode owner never spawned once GGRS started driving the sim"
    );

    // The sim advances under GGRS: the level clock ticks.
    let before = level_state(&mut app).expect("mode owner survives session start");
    for _ in 0..12 {
        app.update();
    }
    let running = level_state(&mut app).expect("mode owner survives GGRS frames");
    assert!(
        running.time_remaining < before.time_remaining,
        "the level clock must tick under the GGRS host ({} -> {})",
        before.time_remaining,
        running.time_remaining
    );
    ambition::runtime::rollback::session_health(app.world())
        .expect("the demo's registered state resimulates checksum-identical");

    // Save → MUTATE → restore: poke a score no gameplay produced, outside any
    // GGRS input. The next update rolls back behind the poke and resimulates;
    // a restore-registered component comes back with history's value, while an
    // unregistered one would keep 7777 forever.
    {
        let world = app.world_mut();
        let mut query = world.query::<&mut MaryOLevelState>();
        let mut state = query
            .single_mut(world)
            .expect("exactly one mode owner in gameplay");
        state.score = 7777;
    }
    for _ in 0..6 {
        app.update();
    }
    let restored = level_state(&mut app).expect("mode owner survives the restore");
    assert_ne!(
        restored.score, 7777,
        "a dirty out-of-band score must be OVERWRITTEN by the rollback restore \
         — surviving it means MaryOLevelState is not actually snapshot/restored"
    );
    ambition::runtime::rollback::session_health(app.world())
        .expect("the run stays checksum-identical after the dirty write is rolled back");
}
