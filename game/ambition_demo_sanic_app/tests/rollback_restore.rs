//! **Behavioral restore proof for Sanic's demo sim state (Phase 5b).**
//!
//! The pin test proves the NAMES reach the registry; this proves the
//! machinery: the demo boots under the ROLLBACK engine host, a real GGRS
//! sync-test session drives it, and a dirty out-of-band mutation of
//! [`SanicActState`] is OVERWRITTEN by the next rollback's restore. The
//! Mary-O twin of this test caught exactly the failure it guards against:
//! state registered for restore but carried by an entity no rollback anchor
//! reached, so the registration was theater until `entity:sanic_mode_owner`
//! anchored the owner.

use ambition::game_shell::{
    ShellHostConfiguration, ShellHostSpec, ShellLaunchCatalog, ShellRouteCatalog, ShellRouteSpec,
};
use ambition_demo_sanic::{SanicActState, SanicExperiencePlugin, SANIC_GAMEPLAY_ROUTE};
use bevy::prelude::*;

/// The demo shell composed on the GGRS host instead of the fixed tick — the
/// same provider and shell wiring as `build_demo_app`, only the engine
/// group's host choice differs.
fn build_rollback_demo_app() -> App {
    let mut app = App::new();
    ambition::engine::add_headless_foundation(&mut app);
    app.add_plugins(ambition::engine::PlatformerEnginePlugins::rollback());
    app.add_plugins(ambition::windowed_host::PlatformerHostPlugins);
    app.add_plugins(ambition::game_shell::MinimalShellPlugins);
    app.insert_resource(ambition::audio::selection::FrontendAudioProfile::new(
        ambition_demo_sanic::SANIC_EXPERIENCE,
    ));
    app.add_plugins(ambition::load::AmbitionLoadPlugin);
    app.add_plugins(ambition::load_presentation::MinimalShellLoadPresentationPlugins);
    app.add_plugins(SanicExperiencePlugin);
    app.world_mut()
        .resource_mut::<ShellRouteCatalog>()
        .register(ShellRouteSpec::new(
            ambition_demo_sanic::SANIC_LAUNCHER_ROUTE,
            ShellLaunchCatalog::basic_experience_id(),
        ));
    app.world_mut()
        .resource_mut::<ShellHostConfiguration>()
        .spec = Some(ShellHostSpec::new(
        SANIC_GAMEPLAY_ROUTE,
        ambition_demo_sanic::SANIC_LAUNCHER_ROUTE,
    ));
    let timestep = std::time::Duration::from_secs_f32(1.0 / 60.0);
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(timestep));
    app
}

fn act_state(app: &mut App) -> Option<SanicActState> {
    let mut query = app.world_mut().query::<&SanicActState>();
    query.iter(app.world()).next().copied()
}

#[test]
fn a_dirty_act_state_mutation_is_rolled_back_by_restore() {
    let mut app = build_rollback_demo_app();

    // Boot until the SHELL activates the gameplay session (the Update-side
    // fact; the sim is frozen until a GGRS session drives it).
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
        "the Sanic shell never activated a gameplay session under the rollback host"
    );

    ambition::runtime::rollback::start_sync_test_session(
        app.world_mut(),
        ambition::runtime::rollback::SyncTestSettings {
            check_distance: 4,
            max_prediction_window: 10,
        },
    )
    .expect("the demo composition starts a GGRS sync-test session");

    // GGRS now drives the sim: staging finishes and the mode owner spawns.
    let mut owner_exists = false;
    for _ in 0..300 {
        app.update();
        if act_state(&mut app).is_some() {
            owner_exists = true;
            break;
        }
    }
    assert!(
        owner_exists,
        "the act-state owner never spawned once GGRS started driving the sim"
    );

    // The act clock ticks under GGRS.
    let before = act_state(&mut app).expect("act owner survives session start");
    for _ in 0..12 {
        app.update();
    }
    let running = act_state(&mut app).expect("act owner survives GGRS frames");
    assert!(
        running.elapsed > before.elapsed,
        "the act clock must tick under the GGRS host ({} -> {})",
        before.elapsed,
        running.elapsed
    );
    ambition::runtime::rollback::session_health(app.world())
        .expect("the demo's registered state resimulates checksum-identical");

    // Save → MUTATE → restore: an out-of-band milestone index no gameplay
    // produced must be overwritten by the rollback's restore.
    {
        let world = app.world_mut();
        let mut query = world.query::<&mut SanicActState>();
        let mut state = query
            .single_mut(world)
            .expect("exactly one act owner in gameplay");
        state.next_milestone = 777;
    }
    for _ in 0..6 {
        app.update();
    }
    let restored = act_state(&mut app).expect("act owner survives the restore");
    assert_ne!(
        restored.next_milestone, 777,
        "a dirty out-of-band milestone must be OVERWRITTEN by the rollback \
         restore — surviving it means SanicActState is not actually snapshot/restored"
    );
    ambition::runtime::rollback::session_health(app.world())
        .expect("the run stays checksum-identical after the dirty write is rolled back");
}
