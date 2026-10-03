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
    // This body is born only once GGRS runs the simulation, so the save is
    // applied over the live timeline. That is owed (`BODY-BORN-ON-THE-TIMELINE`
    // in the queue). With this declaration the session counts each application
    // instead of refusing it, and the fixture asserts the count.
    app.init_resource::<ambition_platformer2d::rollback::TheBodyIsBornOnTheTimeline>();
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
    assert_eq!(
        app.world()
            .resource::<ambition_platformer2d::rollback::TheBodyIsBornOnTheTimeline>()
            .restores_over_a_live_timeline,
        1,
        "the declared road: the save is applied once, over the live timeline"
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

/// How many times `Update` applies the save over 60 frames of a sync-test
/// session that rolls back 4 frames on every frame. `lower_at` lowers the latch
/// once, in `Update` after the GGRS step, as a reload would. A write before the
/// GGRS step is no test: the step loads a snapshot and puts the latch back.
/// Counts each frame on which `SaveRestored` was down before the restore chain
/// and up after it, whatever the frame began with.
fn applications_of_the_save(lower_at: Option<u32>) -> u32 {
    use ambition_platformer2d::actors::session::durable_horizon::{
        DurableRestoreSet, SaveRestored,
    };
    #[derive(Resource, Default)]
    struct Seen {
        frame: u32,
        lower_at: Option<u32>,
        before: bool,
        rises: u32,
    }
    let mut app = build_rollback_demo_app();
    app.insert_resource(Seen::default());
    app.add_systems(
        Update,
        (|mut restored: ResMut<SaveRestored>, mut seen: ResMut<Seen>| {
            seen.frame += 1;
            if seen.lower_at == Some(seen.frame) {
                restored.0 = false;
            }
            seen.before = restored.0;
        })
        .before(DurableRestoreSet::Lifecycle),
    );
    app.add_systems(
        Update,
        (|restored: Res<SaveRestored>, mut seen: ResMut<Seen>| {
            if !seen.before && restored.0 {
                seen.rises += 1;
            }
        })
        .after(DurableRestoreSet::Complete),
    );
    start_gameplay_under_sync_test(&mut app);
    let started = app.world().resource::<Seen>().frame;
    app.world_mut().resource_mut::<Seen>().lower_at = lower_at.map(|at| started + at);
    for _ in 0..60 {
        app.update();
    }
    app.world().resource::<Seen>().rises
}

/// The save is applied once in this composition, though `SaveRestored` is
/// rollback state and the restore chain runs in `Update`. A rewind to a
/// snapshot from before the rise would put the latch down and let the chain run
/// again. Traced 2026-10-03: the rise is on the first live frame, and no later
/// GGRS step puts the latch back down. The probable reason is that only the
/// frame-0 snapshot holds it down and this sync test does not load frame 0;
/// that is inferred, not read in GGRS. A body born later on the timeline is not
/// covered by this result. The declared count cannot see a second rise, because such a frame
/// begins with the latch up.
#[test]
fn the_save_is_applied_once_on_the_first_live_frame() {
    assert_eq!(applications_of_the_save(None), 1);
}

/// The control: the instrument sees a second application when the latch is
/// lowered once after the GGRS step.
#[test]
fn a_lowered_latch_is_seen_as_a_second_application() {
    assert_eq!(applications_of_the_save(Some(30)), 2);
}
