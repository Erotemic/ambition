//! Behavioral restore proof for Sanic's demo sim state (Phase 5b).

use ambition_demo_sanic::{SanicActState, SanicExperiencePlugin, SANIC_GAMEPLAY_ROUTE};
use ambition_platformer2d::game_shell::{
    ShellHostConfiguration, ShellHostSpec, ShellExperienceId, ShellRouteCatalog, ShellRouteSpec,
};
use bevy::prelude::*;

/// The demo shell composed on the GGRS host instead of the fixed tick — the
/// same provider and shell wiring as `build_demo_app`, only the engine
/// group's host choice differs.
fn build_rollback_demo_app() -> App {
    let mut app = App::new();
    ambition_platformer2d::engine::add_headless_foundation(&mut app);
    app.add_plugins(ambition_platformer2d::rollback::RollbackEnginePlugin);
    app.add_plugins(ambition_platformer2d::windowed_host::PlatformerHostPlugins);
    app.add_plugins(ambition_platformer2d::game_shell::MinimalShellPlugins);
    app.insert_resource(
        ambition_platformer2d::audio::selection::FrontendAudioRegistry::direct(
            ambition_platformer2d::audio::selection::FrontendAudioProfile::new(
                ambition_demo_sanic::SANIC_EXPERIENCE,
            ),
        ),
    );
    // The engine group supplies `AmbitionLoadPlugin` (the room-transition
    // transaction is a load plan); a duplicate is a hard panic.
    app.add_plugins(ambition_platformer2d::load_presentation::MinimalShellLoadPresentationPlugins);
    app.add_plugins(SanicExperiencePlugin);
    app.world_mut()
        .resource_mut::<ShellRouteCatalog>()
        .register(ShellRouteSpec::new(
            ambition_demo_sanic::SANIC_LAUNCHER_ROUTE,
            ShellExperienceId::basic_launcher(),
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

/// Boot until the shell activates gameplay, start a GGRS sync-test session,
/// and step until the act owner exists: every frame after this is resimulated
/// and checksum-compared.
fn start_gameplay_under_sync_test(app: &mut App) {
    // Boot until the SHELL activates the gameplay session (the Update-side
    // fact; the sim is frozen until a GGRS session drives it).
    let mut activated = false;
    for _ in 0..600 {
        app.update();
        let session_active = app
            .world()
            .get_resource::<ambition_platformer2d::game_shell::ActiveGameplaySession>()
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

    ambition_platformer2d::rollback::start_sync_test_session(
        app.world_mut(),
        ambition_platformer2d::rollback::SyncTestSettings {
            check_distance: 4,
            max_prediction_window: 10,
            ..ambition_platformer2d::rollback::SyncTestSettings::for_players(1)
        },
    )
    .expect("the demo composition starts a GGRS sync-test session");

    // GGRS now drives the sim: staging finishes and the mode owner spawns.
    let mut owner_exists = false;
    for _ in 0..300 {
        app.update();
        if act_state(app).is_some() {
            owner_exists = true;
            break;
        }
    }
    assert!(
        owner_exists,
        "the act-state owner never spawned once GGRS started driving the sim"
    );
}

#[test]
fn a_dirty_act_state_mutation_is_rolled_back_by_restore() {
    let mut app = build_rollback_demo_app();
    start_gameplay_under_sync_test(&mut app);

    // Every carrier has a canonical identity, so frame zero's rebase orders it
    // by name rather than by this App's construction order. The act owner was
    // the one that did not (`spawn_mode_owner` now names it).
    {
        let world = app.world_mut();
        let carriers = world
            .query_filtered::<Entity, With<ambition_platformer2d::rollback::Rollback>>()
            .iter(world)
            .count();
        let unnamed = world
            .query_filtered::<Entity, (
                With<ambition_platformer2d::rollback::Rollback>,
                Without<ambition_platformer2d::platformer::sim_id::SimId>,
            )>()
            .iter(world)
            .count();
        assert!(carriers > 0, "the census saw no rollback carriers at all");
        assert_eq!(unnamed, 0, "{unnamed} of {carriers} rollback carriers have no SimId");
    }

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
    ambition_platformer2d::rollback::session_health(app.world())
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
    ambition_platformer2d::rollback::session_health(app.world())
        .expect("the run stays checksum-identical after the dirty write is rolled back");
}

/// A spin-dash launched and rolled under the sync-test session resimulates
/// checksum-identical: the roll and what it carries are restored with the
/// frame they belong to.
#[test]
fn a_spin_dash_rolls_checksum_identical_under_resimulation() {
    use ambition_demo_sanic::ball_dash::Rolling;
    use ambition_platformer2d::sim::ControlFrame;

    let mut app = build_rollback_demo_app();
    start_gameplay_under_sync_test(&mut app);

    let rolling = |app: &mut App| {
        let mut q = app
            .world_mut()
            .query_filtered::<(), (With<Rolling>, With<ambition_platformer2d::platformer::markers::PrimaryPlayer>)>();
        q.iter(app.world()).next().is_some()
    };
    // Rev: Down held, the attack verb pressed every fourth frame, through the
    // seat input GGRS publishes. Then release Down to launch.
    let drive = |app: &mut App, frame: ControlFrame| {
        ambition_platformer2d::sim::drive_control_frame(app.world_mut(), frame);
        app.update();
    };
    for frame in 0..90 {
        drive(
            &mut app,
            ControlFrame {
                axis_y: 1.0,
                attack_pressed: frame % 4 == 0,
                ..Default::default()
            },
        );
    }
    let mut rolled = false;
    for _ in 0..60 {
        drive(&mut app, ControlFrame::default());
        rolled |= rolling(&mut app);
    }
    assert!(rolled, "the premise: Down+X then releasing Down launches a roll under GGRS");
    ambition_platformer2d::rollback::session_health(app.world())
        .expect("a roll resimulates checksum-identical");
}

/// Where the save is applied over 60 frames of a sync-test session that rolls
/// back 4 frames on every frame: `(in the GGRS step, in Update)`.
///
/// `SaveRestored` is read at `First`, at the head of `Update` and at `Last`.
/// GGRS steps the simulation in `PreUpdate`, so a rise between `First` and
/// `Update` is the simulation's, and a rise between `Update` and `Last` is
/// `Update`'s. The `Update` reader runs before `DurableRestoreSet`, so a chain
/// moved back into `Update` is seen there.
fn where_the_save_is_applied() -> (u32, u32) {
    use ambition_platformer2d::actors::session::durable_horizon::{
        DurableRestoreSet, SaveRestored,
    };
    #[derive(Resource, Default)]
    struct Seen {
        at_first: bool,
        at_update: bool,
        in_the_step: u32,
        in_update: u32,
    }
    let mut app = build_rollback_demo_app();
    app.init_resource::<Seen>();
    app.add_systems(
        First,
        |restored: Res<SaveRestored>, mut seen: ResMut<Seen>| seen.at_first = restored.0,
    );
    app.add_systems(
        Update,
        (|restored: Res<SaveRestored>, mut seen: ResMut<Seen>| {
            seen.at_update = restored.0;
            if !seen.at_first && restored.0 {
                seen.in_the_step += 1;
            }
        })
        .before(DurableRestoreSet::Lifecycle),
    );
    app.add_systems(
        Last,
        |restored: Res<SaveRestored>, mut seen: ResMut<Seen>| {
            if !seen.at_update && restored.0 {
                seen.in_update += 1;
            }
        },
    );
    start_gameplay_under_sync_test(&mut app);
    for _ in 0..60 {
        app.update();
    }
    ambition_platformer2d::rollback::session_health(app.world())
        .expect("the run stays checksum-identical across the save's application");
    let seen = app.world().resource::<Seen>();
    (seen.in_the_step, seen.in_update)
}

/// The save is applied by the simulation, once, and never by `Update`. This
/// body is born only once GGRS runs the simulation, so the save is applied on
/// the timeline; the restore chain runs in the simulation schedule, from
/// rollback state, so a rewind past it applies it again on the same tick and the
/// sync test stays healthy (BODY-BORN-ON-THE-TIMELINE).
#[test]
fn the_save_is_applied_by_the_simulation() {
    assert_eq!(where_the_save_is_applied(), (1, 0));
}
