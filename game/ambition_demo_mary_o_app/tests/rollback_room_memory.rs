//! Ending a level under rollback must end it the same way on resimulation.
//!
//! ```text
//! MaryOLevelState         rollback state — rewinds (registered + probed)
//! flag::FlagSequence      rollback state — rewinds (registered + probed)
//! PendingLifecycleCommit  rollback state — rewinds
//! the dwell timer gating all three  ← was a `Local`, and did NOT rewind
//! ```
//!
//! ⛔⛔ A `Local` ON A SIM SYSTEM IS NOT ROLLBACK STATE.
//! `cycle_level_on_flag_tally` accumulated `dwell` on the SIM clock and compared
//! it against `LEVEL_CYCLE_DWELL`. First simulation: the threshold crosses at
//! tick N, the level re-arms and records a transition intent. GGRS rewinds the
//! world — but not the local — and resimulates from a dwell that is already
//! past the threshold, so the crossing lands on a different tick. One timeline
//! re-armed the level and asked for its successor; the other did not.
//!
//! ⭐ MEASURED, NOT REASONED. Before the fix this desynced at frames 92-93 on
//! exactly two checksummed types, `MaryOLevelState` and
//! `PendingLifecycleCommit` — which are precisely what the threshold gates. The
//! localizer that named them is `which_component_diverges_on_the_pole_slide`
//! below.
//!
//! ⚠ WHAT THIS FILE DOES NOT PIN. `follow_the_active_room`'s room memory moved
//! onto the same component, and putting a `Local` back there leaves this test
//! GREEN — verified by poisoning that leg alone. A confirmed room transition
//! REBASES GGRS onto a new frame-zero baseline, so no rewind ever crosses the
//! commit frame and that memory's divergence is unreachable today. The move is
//! still right — a correctness that holds only because some other layer rebases
//! is a correctness that moves when the rebase does — but it is unpinned, and
//! saying so is better than implying a guard that does not exist.
//!
//! ⭐ THE ENGINE FIXED THE ROOM-MEMORY SHAPE ONCE ALREADY, for cutscene
//! triggers: see the `narrative trigger seam` note in
//! `game/ambition_app/tests/rollback_coverage.rs`, whose memory became
//! `ambition_cutscene::LastCutsceneRoom` for exactly this reason.

use ambition_demo_mary_o::level_1_2::LEVEL_1_2_ROOM_ID;
use ambition_demo_mary_o::LEVEL_1_1_ROOM_ID;
use ambition_demo_mary_o::{MaryOExperiencePlugin, MaryOLevelState, MARY_O_GAMEPLAY_ROUTE};
use ambition_platformer2d::game_shell::{
    ShellHostConfiguration, ShellHostSpec, ShellLaunchCatalog, ShellRouteCatalog, ShellRouteSpec,
};
use ambition_platformer2d::platformer::markers::PrimaryPlayer;
use ambition_platformer2d::world::rooms::RoomSet;
use bevy::prelude::*;

/// The demo shell on the GGRS host. Same composition as
/// `rollback_restore.rs`'s, and inline for the same reason: the fixed-tick
/// composer is private to the demo app crate.
fn build_rollback_demo_app() -> App {
    let mut app = App::new();
    ambition_platformer2d::engine::add_headless_foundation(&mut app);
    app.add_plugins(ambition_platformer2d::rollback::RollbackEnginePlugin);
    app.add_plugins(ambition_platformer2d::windowed_host::PlatformerHostPlugins);
    app.add_plugins(ambition_platformer2d::game_shell::MinimalShellPlugins);
    app.insert_resource(
        ambition_platformer2d::audio::selection::FrontendAudioRegistry::direct(
            ambition_platformer2d::audio::selection::FrontendAudioProfile::new(
                ambition_demo_mary_o::MARY_O_EXPERIENCE,
            ),
        ),
    );
    app.add_plugins(ambition_platformer2d::load_presentation::MinimalShellLoadPresentationPlugins);
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

fn sync_test(app: &mut App) {
    ambition_platformer2d::rollback::start_sync_test_session(
        app.world_mut(),
        ambition_platformer2d::rollback::SyncTestSettings {
            check_distance: 4,
            max_prediction_window: 10,
            ..ambition_platformer2d::rollback::SyncTestSettings::for_players(1)
        },
    )
    .expect("the demo composition starts a GGRS sync-test session");
}

fn level_state(app: &mut App) -> Option<MaryOLevelState> {
    let mut query = app.world_mut().query::<&MaryOLevelState>();
    query.iter(app.world()).next().copied()
}

fn active_room(app: &mut App) -> Option<String> {
    let mut query = app.world_mut().query::<&RoomSet>();
    let world = app.world();
    query
        .iter(world)
        .next()
        .map(|set| set.rooms[set.active].id.clone())
}

fn place_player(app: &mut App, pos: Vec2) {
    let mut query = app.world_mut().query_filtered::<(
        ambition_platformer2d::engine_core::BodyClusterQueryData,
        &mut ambition_platformer2d::actor::MotionModel,
    ), With<PrimaryPlayer>>();
    let world = app.world_mut();
    let Some((mut cluster_item, mut motion_model)) = query.iter_mut(world).next() else {
        panic!("gameplay has no primary player to place");
    };
    let mut clusters = cluster_item.as_clusters_mut();
    ambition_platformer2d::engine_core::movement::transit_body(
        &mut motion_model,
        &mut clusters,
        pos,
        ambition_platformer2d::engine_core::movement::TransitVelocity::Zero,
    );
}

/// ⛔ THE CONTROL. Everything below drives an unusual path — an out-of-band
/// placement, a rebase, a scripted pole slide — and a desync anywhere in it
/// means nothing until this says the fixture's own boot is checksum-clean over
/// the same number of frames.
#[test]
fn the_fixture_boot_is_checksum_clean_on_its_own() {
    let mut app = boot_to_gameplay();
    for frame in 0..400 {
        app.update();
        ambition_platformer2d::rollback::session_health(app.world())
            .unwrap_or_else(|error| panic!("control frame {frame} desynced: {error}"));
    }
    assert_eq!(
        active_room(&mut app).as_deref(),
        Some(LEVEL_1_1_ROOM_ID),
        "the control arm must not change rooms — it is the arm without the slide"
    );
}

/// Boot the rollback demo to a live gameplay session with GGRS driving the sim.
fn boot_to_gameplay() -> App {
    let mut app = build_rollback_demo_app();

    // Under the GGRS host the sim advances only once a session exists, so the
    // sim-side world follows the shell's activation.
    let mut activated = false;
    for _ in 0..600 {
        app.update();
        if app
            .world()
            .get_resource::<ambition_platformer2d::game_shell::ActiveGameplaySession>()
            .is_some_and(|session| session.0.is_some())
        {
            activated = true;
            break;
        }
    }
    assert!(
        activated,
        "the Mary-O shell never activated a gameplay session under the rollback host"
    );

    sync_test(&mut app);
    let mut owner = false;
    for _ in 0..300 {
        app.update();
        if level_state(&mut app).is_some() && active_room(&mut app).is_some() {
            owner = true;
            break;
        }
    }
    assert!(
        owner,
        "the mode owner never spawned once GGRS started driving the sim"
    );
    assert_eq!(
        active_room(&mut app).as_deref(),
        Some(LEVEL_1_1_ROOM_ID),
        "the demo should start on the surface"
    );
    app
}

/// WHICH COMPONENT the pole slide's divergence lives in.
///
/// ⚠ It is STRICTER than the GGRS checksum: it censuses every registered
/// component, including ones outside the checksum, so `Transform`,
/// `GlobalTransform` and `SessionScopedEntity` appear from frame 9 on a boot the
/// checksum calls clean. Read the frames the checksum actually flagged.
///
/// `#[ignore]` for cost — a per-component census on every save and load. Run it
/// when this module is red:
///
/// ```text
/// cargo test -p ambition_demo_mary_o_app --test mary_o_it -- \
///     rollback_room_memory::which_component --ignored --nocapture
/// ```
#[test]
#[ignore = "diagnostic: per-component restore census on every save/load; run when this module is red"]
fn which_component_diverges_on_the_pole_slide() {
    let mut app = boot_to_gameplay();
    app.world_mut()
        .insert_resource(ambition_platformer2d::rollback::RollbackRestoreAudit::enabled());

    let pole = ambition_demo_mary_o::pole_for_room(LEVEL_1_1_ROOM_ID);
    place_player(&mut app, Vec2::new(pole.x, pole.base_y - 24.0));
    sync_test(&mut app);
    for _ in 0..200 {
        app.update();
    }

    let audit = app
        .world()
        .resource::<ambition_platformer2d::rollback::RollbackRestoreAudit>();
    // Vacuity guard FIRST: a localizer that compared nothing must not read as
    // "nothing diverged".
    assert!(
        audit.comparisons > 0 && audit.resimulations > 0,
        "the audit compared nothing, so its verdict is meaningless: saves={} loads={} \
         resimulations={} comparisons={}",
        audit.saves,
        audit.loads,
        audit.resimulations,
        audit.comparisons
    );
    assert!(
        audit.divergences.is_empty(),
        "divergences: {:#?}",
        audit.divergences
    );
}

#[test]
fn ending_a_level_resimulates_identically() {
    let mut app = boot_to_gameplay();

    // ⭐ PLACE HER ON THE POLE, THEN REBASE. A body moved outside GGRS input is
    // a change resimulation cannot reproduce, so it may not sit behind the
    // rollback cursor: restarting the sync-test session makes the placement the
    // new frame-zero baseline. Without the rebase this fixture would desync on
    // its own setup and prove nothing about the room memory.
    let pole = ambition_demo_mary_o::pole_for_room(LEVEL_1_1_ROOM_ID);
    place_player(&mut app, Vec2::new(pole.x, pole.base_y - 24.0));
    sync_test(&mut app);

    // Ride the flag sequence into 1-2. No input: catching the pole is what
    // starts the slide, and the tally is what asks for the level change.
    let mut arrived = None;
    for frame in 0..900 {
        app.update();
        ambition_platformer2d::rollback::session_health(app.world()).unwrap_or_else(|error| {
            panic!("frame {frame} before/around the room change desynced: {error}")
        });
        if active_room(&mut app).as_deref() == Some(LEVEL_1_2_ROOM_ID) {
            arrived = Some(frame);
            break;
        }
    }
    let arrived = arrived.unwrap_or_else(|| {
        panic!(
            "the pole slide never changed the active room, so this test never \
             reached the transition it is about"
        )
    });

    // ⛔ AND PAST IT. The divergence is between the first simulation of the
    // committing frame and its resimulation, so it only becomes visible once
    // GGRS has rolled back over that frame — which is `check_distance` updates
    // later, not on the frame itself.
    for frame in 0..120 {
        app.update();
        ambition_platformer2d::rollback::session_health(app.world()).unwrap_or_else(|error| {
            panic!(
                "frame {frame} after the room change (which landed at {arrived}) \
                 desynced: {error}"
            )
        });
    }
}
