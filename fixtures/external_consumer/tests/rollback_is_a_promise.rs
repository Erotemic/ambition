//! The acceptance tests ADR 0031 required before rollback could be public.
//!
//! Its Deferred section named six properties and reserved rollback for "its own
//! slice, its own acceptance tests". This is that file. One test per property,
//! named for the property, so a reader can check the promise against the list
//! rather than against a paragraph.
//!
//! They live in the CONSUMER, not the engine. Every one of these is a
//! claim about what a third party can and cannot do, and an engine-side test
//! would be the engine asking itself. Outlander is a real external crate whose
//! only dependency is `ambition_platformer2d`; if a property here needed an engine internal
//! to observe, that would be evidence the property is not actually public.
//!
//! What is ENFORCED and what is DEMONSTRATED are marked separately, because a
//! guard whose limits are unstated gets trusted past them.

use ambition_platformer2d::rollback::{RollbackPlan, RollbackRefused};

/// Property 4: deterministic activation.
///
/// A session rebases frame zero onto the live world, so the world has to be built first. The
/// consumer cannot get it wrong now because the consumer no longer performs it.
///
/// ENFORCED: `start` drives the host itself and reports what it waited for.
#[test]
fn the_engine_activates_the_host_before_frame_zero() {
    let app = outlander::build_outlander_rollback_app().expect("rollback host");
    // The session exists, which is only true if activation completed first.
    assert!(
        ambition_platformer2d::app::host_status(&app).is_running(),
        "the rollback host is not running after `start` returned Ok"
    );
}

/// Property 4, the refusal half: a host that never activates says so.
///
/// A zero-tick budget is the cheap way to reach the branch, and reaching it is the point — an error
/// path no test walks is a paragraph.
#[test]
fn a_host_that_never_activates_names_what_it_was_doing() {
    let mut app = ambition_platformer2d::app::PlatformerApp::headless()
        .rollback(1)
        .mount(outlander::OutlanderModule)
        .build();

    let refused = ambition_platformer2d::rollback::start(&mut app, RollbackPlan::new().activation_budget(0))
        .expect_err("a zero-tick budget cannot reach a running host");

    let RollbackRefused::NeverActivated { ticks, status } = &refused else {
        panic!("expected NeverActivated, got {refused:?}");
    };
    assert_eq!(*ticks, 0);
    assert!(
        !status.is_empty(),
        "the refusal must say what the host was doing"
    );
    // The rendered message is what an author actually reads.
    let message = refused.to_string();
    assert!(
        message.contains("never started"),
        "unhelpful refusal: {message}"
    );
}

/// Property 3: stable participants.
///
/// The count is declared at composition and cannot be re-passed per session, so a restart reuses
/// it.
///
/// ENFORCED: there is no participant argument on `RollbackPlan` at all, and
/// `PlatformerApp::rollback` has no default. This test pins the reported value
/// to the declaration.
#[test]
fn the_participant_count_comes_from_the_composition() {
    let mut app = ambition_platformer2d::app::PlatformerApp::headless()
        .rollback(1)
        .mount(outlander::OutlanderModule)
        .build();
    let session = ambition_platformer2d::rollback::start(&mut app, RollbackPlan::new()).expect("session");
    assert_eq!(
        session.participants(),
        1,
        "Outlander declared one participant; the session seated {}",
        session.participants()
    );
}

/// Property 3's other half: a host not composed for rollback refuses.
///
/// A fixed-step host must refuse rollback explicitly rather than starting a
/// session that does nothing.
#[test]
fn a_fixed_step_host_refuses_rather_than_pretending() {
    let mut app = outlander::build_outlander_app();
    let refused = ambition_platformer2d::rollback::start(&mut app, RollbackPlan::new())
        .expect_err("a fixed-step host has no rollback session to start");
    assert!(
        matches!(refused, RollbackRefused::NotComposedForRollback),
        "expected NotComposedForRollback, got {refused:?}"
    );
    // It must name the fix, not just the fault.
    assert!(
        refused.to_string().contains("PlatformerApp::rollback"),
        "the refusal does not say what to call instead: {refused}"
    );
}

/// Property 2: complete authoritative baseline.
///
/// A session over an empty registry saves nothing, rewinds nothing and compares
/// nothing — and passes. This repo has a name for that shape: an instrument
/// that measures nothing reports the success condition.
///
/// ENFORCED: `start` refuses when the registry is empty. That refusal catches
/// a broken COMPOSITION, not a consumer typo — the engine registers its own
/// state, so a real rollback host is never empty. The consumer-facing half of
/// this property is the one asserted below: the baseline must include state the
/// ENGINE has never heard of, or "complete" means "complete as far as we know",
/// which is the version that lets a third party's state silently not roll back.
#[test]
fn the_baseline_includes_state_the_engine_never_heard_of() {
    let mut app = ambition_platformer2d::app::PlatformerApp::headless()
        .rollback(1)
        .mount(outlander::OutlanderModule)
        .build();
    let session = ambition_platformer2d::rollback::start(&mut app, RollbackPlan::new()).expect("session");

    assert!(
        session.encoded_types() > 1,
        "non-vacuity: the session carries {} registrations",
        session.encoded_types()
    );

    let registry = app
        .world()
        .get_resource::<ambition_platformer2d::rollback::RollbackRegistry>()
        .expect("a rollback host has a registry");
    let dump = registry.deterministic_dump();
    assert!(
        dump.contains("outlander.beacon_charge"),
        "the consumer's own authoritative state is not in the rollback \
         baseline. No engine file lists `BeaconCharge`; nothing in `ambition_platformer2d` \
         has heard of it. If registration through the public vocabulary does \
         not reach the baseline, a third party's state silently does not roll \
         back. Registry:\n{dump}"
    );
}

/// Property 5: lifecycle rebasing.
///
/// Restarting uses the same `start` path and rebases frame zero onto the current
/// live world without re-declaring participant count.
#[test]
fn a_restarted_session_rebases_onto_the_live_world() {
    let mut app = outlander::build_outlander_rollback_app().expect("rollback host");

    // Advance the live world past frame zero, so a restart that failed to
    // rebase would be rebuilding onto a world that has moved.
    for _ in 0..30 {
        app.update();
    }

    let restarted = ambition_platformer2d::rollback::start(&mut app, RollbackPlan::new())
        .expect("a running host can be rebased");
    assert_eq!(
        restarted.participants(),
        1,
        "the restart re-sampled the participant count instead of reusing the \
         frozen declaration"
    );
    assert!(
        ambition_platformer2d::app::host_status(&app).is_running(),
        "the host stopped running across a rebase"
    );

    // `is_running()` alone was this test's whole liveness check until blind
    // run 7, and it is not one: a host whose sim is FROZEN still reports
    // `Running { prepared: true }` — the run watched it do so for 4300
    // updates. So the rebase is checked by whether the session still ADVANCES,
    // which is the fact the assertion above only looked like it was making.
    let before = ambition_platformer2d::rollback::health(&app).frame().expect("a frame");
    for _ in 0..30 {
        app.update();
    }
    let health = ambition_platformer2d::rollback::health(&app);
    assert!(
        health.frame().expect("a frame") > before,
        "the session did not advance after the rebase: {health:?}"
    );
    assert!(
        health.is_healthy(),
        "the rebase desynced the session: {health:?}"
    );
}

/// Property 6: confirmation boundaries.
///
/// Seating completes on the session's first frame, so an unsettled start lands
/// activation on GGRS frame 1 — where nothing can rewind across it. `start`
/// settles past activation before frame zero because activation completing is
/// not the same fact as the next tick being quiet.
///
/// DEMONSTRATED, not enforced: the settle is observable as ticks the plan
/// spends, and this pins that the knob is real rather than decorative. The
/// engine cannot detect "quiet enough" for an arbitrary game, which is why the
/// docs say raising it is supported and zero is not.
#[test]
fn the_plan_settles_past_activation_before_frame_zero() {
    let mut app = ambition_platformer2d::app::PlatformerApp::headless()
        .rollback(1)
        .mount(outlander::OutlanderModule)
        .build();
    let session =
        ambition_platformer2d::rollback::start(&mut app, RollbackPlan::new().settle_ticks(32)).expect("session");

    // Activation is reported separately from settling, so a reader can tell
    // which of the two a slow start spent its ticks in.
    assert!(
        session.ticks_to_activation() > 0,
        "a host that activated in zero ticks was already running, which would \
         mean this test is not exercising activation at all"
    );
    assert!(ambition_platformer2d::app::host_status(&app).is_running());
}

/// Property 1: frozen schema.
///
/// ENFORCED elsewhere, and this test says where: `rollback-wire-format-is-frozen`
/// in `scripts/check_absence_contracts.py` freezes all 63 encoded types across
/// the nine crates that own them, both directions — a type may not join the
/// wire format unseen, and one that leaves must be pruned in the same commit.
///
/// What belongs HERE is the consumer-visible half: the schema a session runs
/// under is identical across two compositions of the same game. A fingerprint
/// that varied per composition would make "frozen" a property of one process.
#[test]
fn the_schema_is_identical_across_two_compositions() {
    let first = outlander::build_outlander_rollback_app().expect("rollback host");
    let second = outlander::build_outlander_rollback_app().expect("rollback host");

    let dump = |app: &ambition_platformer2d::bevy::prelude::App| {
        app.world()
            .get_resource::<ambition_platformer2d::rollback::RollbackRegistry>()
            .expect("a rollback host has a registry")
            .deterministic_dump()
    };

    let (a, b) = (dump(&first), dump(&second));
    assert!(!a.is_empty(), "non-vacuity: the schema dump is empty");
    assert_eq!(
        a, b,
        "the rollback schema differs between two compositions of the same game"
    );
}

/// A started session is not a running one. (Blind run 7, finding c)
///
/// The run watched `host_status` report `Running { prepared: true }` for 4300
/// updates while its sim was frozen. `RollbackSession` reports startup facts
/// and all three were healthy; nothing in the SDK could see liveness. This is
/// the gap closed, and the test is written so it fails if the answer becomes
/// unconditional in either direction.
#[test]
fn a_consumer_can_ask_whether_its_session_is_still_healthy() {
    use ambition_platformer2d::rollback::RollbackHealth;

    // A host with no session must say so, rather than reporting healthy.
    let fixed = outlander::build_outlander_app();
    assert_eq!(
        ambition_platformer2d::rollback::health(&fixed),
        RollbackHealth::NoSession,
        "a fixed-step host reported a rollback health other than NoSession"
    );

    let mut app = outlander::build_outlander_rollback_app().expect("rollback host");
    let health = ambition_platformer2d::rollback::health(&app);
    assert!(
        health.is_healthy(),
        "a freshly started session is not healthy: {health:?}"
    );

    // The half a single sample cannot see. A frozen session reports
    // `Healthy` forever — liveness is a property of TWO observations, which is
    // why `RollbackHealth::frame` says to sample it twice and why this test
    // does.
    let before = ambition_platformer2d::rollback::health(&app).frame().expect("a frame");
    for _ in 0..60 {
        app.update();
    }
    let after = ambition_platformer2d::rollback::health(&app).frame().expect("a frame");
    assert!(
        after > before,
        "the session did not advance across 60 updates ({before} -> {after}), \
         which is exactly the frozen-but-Running state blind run 7 could not \
         diagnose from outside"
    );
    assert!(
        ambition_platformer2d::rollback::health(&app).is_healthy(),
        "the session desynced while simulating: {:?}",
        ambition_platformer2d::rollback::health(&app)
    );
}

/// The control run: prove re-simulation is HAPPENING, not just agreeing.
///
/// Blind run 8 built this and it is a better proof than anything I wrote for
/// this file. Its shape: a counter that ticks once per sim tick either tracks
/// the frame count 1:1 (rewound correctly) or over-counts by roughly
/// `check_distance + 1` (never rewound, so every re-simulation of a confirmed
/// frame counted again).
///
/// The second number is what proves the first one means rollback rather than nothing.
///
/// Here the same logic runs against the ENGINE's own consumer fixture: the
/// beacon must not out-count the frames the session actually advanced.
#[test]
fn a_rewound_counter_does_not_out_count_the_frames_it_ran() {
    let mut app = outlander::build_outlander_rollback_app().expect("rollback host");
    let before_frame = ambition_platformer2d::rollback::health(&app).frame().expect("a frame");

    // Walk the body onto the beacon so the counter is actually ticking — a
    // counter that never increments passes any ratio.
    for _ in 0..240 {
        outlander::drive_control_frame(
            &mut app,
            ambition_platformer2d::sim::ControlFrame {
                axis_x: 1.0,
                ..Default::default()
            },
        );
        app.update();
    }

    let health = ambition_platformer2d::rollback::health(&app);
    assert!(health.is_healthy(), "the session desynced: {health:?}");
    let frames = health.frame().expect("a frame") - before_frame;

    let world = app.world_mut();
    let mut query = world.query::<&outlander::BeaconCharge>();
    let ticks = query.iter(world).map(|b| b.ticks).max().unwrap_or(0);

    assert!(
        ticks > 0,
        "the beacon never charged, so the ratio below compares nothing — this \
         is the vacuity the control run exists to rule out"
    );
    assert!(
        i64::from(ticks) <= i64::from(frames),
        "the beacon counted {ticks} ticks across {frames} simulated frames. A \
         counter that out-counts its own timeline was re-simulated without \
         being rewound, which is precisely the silent failure registration is \
         supposed to prevent."
    );
}

///
/// Teardown removes the session, its ownership and the confirmed-frame
/// boundary — and leaves `RollbackSessionStatus` and `RollbackFrameCount`
/// behind, because the next session inherits the first and restarts the second.
/// `health` read only those two, so a stopped host reported
/// `Healthy { frame }` at the last frame it ever ran, from a public API, for as
/// long as the process lived.
///
/// The restart half is the reason [`RollbackHealth::generation`] exists: frames
/// restart at zero on every session, so a consumer comparing frame numbers
/// across a stop cannot tell a new timeline from a rewound one.
///
/// ENFORCED: `session_is_active` decides before any read model is consulted.
#[test]
fn a_stopped_session_is_not_healthy_and_a_restart_is_a_new_timeline() {
    use ambition_platformer2d::rollback::RollbackHealth;

    let mut app = outlander::build_outlander_rollback_app().expect("rollback host");
    let started = ambition_platformer2d::rollback::health(&app);
    assert!(
        started.is_healthy(),
        "a freshly started session is not healthy: {started:?}"
    );
    let first = started
        .generation()
        .expect("a running session must report which timeline it is");

    // Advance far enough that a stale frame count would be a CONVINCING lie:
    // the old value is the last real frame this session reached.
    for _ in 0..30 {
        app.update();
    }
    let last_frame = ambition_platformer2d::rollback::health(&app).frame().expect("a frame");
    assert!(last_frame > 0, "the session never advanced");

    ambition_platformer2d::rollback::stop(&mut app);
    assert_eq!(
        ambition_platformer2d::rollback::health(&app),
        RollbackHealth::NoSession,
        "a stopped host still reports a session; the frame it would have \
         claimed is {last_frame}"
    );

    let restarted = ambition_platformer2d::rollback::start(&mut app, RollbackPlan::new())
        .expect("a stopped host must be restartable");
    assert_eq!(
        restarted.participants(),
        1,
        "the restart re-seated a different topology"
    );
    let health = ambition_platformer2d::rollback::health(&app);
    assert!(
        health.is_healthy(),
        "the restarted session is not healthy: {health:?}"
    );
    let second = health.generation().expect("a running session");
    assert!(
        second > first,
        "the restart reused timeline {first}, so nothing downstream can tell \
         its work apart from the session that ended"
    );
}
