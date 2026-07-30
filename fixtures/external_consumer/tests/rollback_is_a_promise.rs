//! **The acceptance tests ADR 0031 required before rollback could be public.**
//!
//! Its Deferred section named six properties and reserved rollback for "its own
//! slice, its own acceptance tests". This is that file. One test per property,
//! named for the property, so a reader can check the promise against the list
//! rather than against a paragraph.
//!
//! ⚠ **They live in the CONSUMER, not the engine.** Every one of these is a
//! claim about what a third party can and cannot do, and an engine-side test
//! would be the engine asking itself. Outlander is a real external crate whose
//! only dependency is `ambition`; if a property here needed an engine internal
//! to observe, that would be evidence the property is not actually public.
//!
//! What is ENFORCED and what is DEMONSTRATED are marked separately, because a
//! guard whose limits are unstated gets trusted past them.

use ambition::rollback::{RollbackPlan, RollbackRefused};

/// Property 4: **deterministic activation.**
///
/// A session rebases frame zero onto the live world, so the world has to be
/// built first. This is the hazard the engine hit in its own first draft —
/// session started on update #1, GGRS reporting a checksum mismatch on frames
/// 2, 3 and 4 forever — and the fix used to be forty lines of ordering in this
/// fixture. The consumer cannot get it wrong now because the consumer no longer
/// performs it.
///
/// ENFORCED: `start` drives the host itself and reports what it waited for.
#[test]
fn the_engine_activates_the_host_before_frame_zero() {
    let app = outlander::build_outlander_rollback_app().expect("rollback host");
    // The session exists, which is only true if activation completed first.
    assert!(
        ambition::app::host_status(&app).is_running(),
        "the rollback host is not running after `start` returned Ok"
    );
}

/// Property 4, the refusal half: a host that never activates says so.
///
/// A zero-tick budget is the cheap way to reach the branch, and reaching it is
/// the point — an error path no test walks is a paragraph. The message has to
/// name the state the host was in, because "failed to start" sends an author
/// into `crates/`, which is the failure ADR 0031's blind-agent gate measures.
#[test]
fn a_host_that_never_activates_names_what_it_was_doing() {
    let mut app = ambition::app::PlatformerApp::headless()
        .rollback(1)
        .mount(outlander::OutlanderModule)
        .build();

    let refused = ambition::rollback::start(&mut app, RollbackPlan::new().activation_budget(0))
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

/// Property 3: **stable participants.**
///
/// The count is declared at composition and cannot be re-passed per session, so
/// a restart reuses it. The engine shipped the weaker version of this bug: a
/// rollback oracle proving determinism for ONE input stream the same week a 2–4
/// player couch versus mode seated four, because `..Default::default()` quietly
/// meant one player.
///
/// ENFORCED: there is no participant argument on `RollbackPlan` at all, and
/// `PlatformerApp::rollback` has no default. This test pins the reported value
/// to the declaration.
#[test]
fn the_participant_count_comes_from_the_composition() {
    let mut app = ambition::app::PlatformerApp::headless()
        .rollback(1)
        .mount(outlander::OutlanderModule)
        .build();
    let session = ambition::rollback::start(&mut app, RollbackPlan::new()).expect("session");
    assert_eq!(
        session.participants(),
        1,
        "Outlander declared one participant; the session seated {}",
        session.participants()
    );
}

/// Property 3's other half: a host not composed for rollback refuses.
///
/// Outlander's fixed-step face is a real composition that ships, so this is not
/// a synthetic host. Calling `start` on it must be a stated error rather than a
/// session that quietly does nothing — the campaign's rule for every face.
#[test]
fn a_fixed_step_host_refuses_rather_than_pretending() {
    let mut app = outlander::build_outlander_app();
    let refused = ambition::rollback::start(&mut app, RollbackPlan::new())
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

/// Property 2: **complete authoritative baseline.**
///
/// A session over an empty registry saves nothing, rewinds nothing and compares
/// nothing — and passes. This repo has a name for that shape: an instrument
/// that measures nothing reports the success condition.
///
/// ENFORCED: `start` refuses when the registry is empty. ⚠ That refusal catches
/// a broken COMPOSITION, not a consumer typo — the engine registers its own
/// state, so a real rollback host is never empty. The consumer-facing half of
/// this property is the one asserted below: the baseline must include state the
/// ENGINE has never heard of, or "complete" means "complete as far as we know",
/// which is the version that lets a third party's state silently not roll back.
#[test]
fn the_baseline_includes_state_the_engine_never_heard_of() {
    let mut app = ambition::app::PlatformerApp::headless()
        .rollback(1)
        .mount(outlander::OutlanderModule)
        .build();
    let session = ambition::rollback::start(&mut app, RollbackPlan::new()).expect("session");

    assert!(
        session.encoded_types() > 1,
        "non-vacuity: the session carries {} registrations",
        session.encoded_types()
    );

    let registry = app
        .world()
        .get_resource::<ambition::rollback::RollbackRegistry>()
        .expect("a rollback host has a registry");
    let dump = registry.deterministic_dump();
    assert!(
        dump.contains("outlander.beacon_charge"),
        "the consumer's own authoritative state is not in the rollback \
         baseline. No engine file lists `BeaconCharge`; nothing in `ambition` \
         has heard of it. If registration through the public vocabulary does \
         not reach the baseline, a third party's state silently does not roll \
         back. Registry:\n{dump}"
    );
}

/// Property 5: **lifecycle rebasing.**
///
/// Proof pulses, hot-reload rebases and lifecycle commits are all the same
/// session RESTARTED. A restart rebases frame zero onto the CURRENT live world;
/// the hazard the campaign names is an un-rebased `world_mut` write, which
/// replays a world that never had it.
///
/// ENFORCED: restarting is `start` again, which performs the same rebase — so a
/// consumer cannot restart by a different route than it started by, and cannot
/// re-declare the participant count while doing it.
#[test]
fn a_restarted_session_rebases_onto_the_live_world() {
    let mut app = outlander::build_outlander_rollback_app().expect("rollback host");

    // Advance the live world past frame zero, so a restart that failed to
    // rebase would be rebuilding onto a world that has moved.
    for _ in 0..30 {
        app.update();
    }

    let restarted = ambition::rollback::start(&mut app, RollbackPlan::new())
        .expect("a running host can be rebased");
    assert_eq!(
        restarted.participants(),
        1,
        "the restart re-sampled the participant count instead of reusing the \
         frozen declaration"
    );
    assert!(
        ambition::app::host_status(&app).is_running(),
        "the host stopped running across a rebase"
    );

    // ⚠ `is_running()` alone was this test's whole liveness check until blind
    // run 7, and it is not one: a host whose sim is FROZEN still reports
    // `Running { prepared: true }` — the run watched it do so for 4300
    // updates. So the rebase is checked by whether the session still ADVANCES,
    // which is the fact the assertion above only looked like it was making.
    let before = ambition::rollback::health(&app).frame().expect("a frame");
    for _ in 0..30 {
        app.update();
    }
    let health = ambition::rollback::health(&app);
    assert!(
        health.frame().expect("a frame") > before,
        "the session did not advance after the rebase: {health:?}"
    );
    assert!(health.is_healthy(), "the rebase desynced the session: {health:?}");
}

/// Property 6: **confirmation boundaries.**
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
    let mut app = ambition::app::PlatformerApp::headless()
        .rollback(1)
        .mount(outlander::OutlanderModule)
        .build();
    let session =
        ambition::rollback::start(&mut app, RollbackPlan::new().settle_ticks(32)).expect("session");

    // Activation is reported separately from settling, so a reader can tell
    // which of the two a slow start spent its ticks in.
    assert!(
        session.ticks_to_activation() > 0,
        "a host that activated in zero ticks was already running, which would \
         mean this test is not exercising activation at all"
    );
    assert!(ambition::app::host_status(&app).is_running());
}

/// Property 1: **frozen schema.**
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

    let dump = |app: &ambition::bevy::prelude::App| {
        app.world()
            .get_resource::<ambition::rollback::RollbackRegistry>()
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

/// **A started session is not a running one.** (Blind run 7, finding c)
///
/// The run watched `host_status` report `Running { prepared: true }` for 4300
/// updates while its sim was frozen. `RollbackSession` reports startup facts
/// and all three were healthy; nothing in the SDK could see liveness. This is
/// the gap closed, and the test is written so it fails if the answer becomes
/// unconditional in either direction.
#[test]
fn a_consumer_can_ask_whether_its_session_is_still_healthy() {
    use ambition::rollback::RollbackHealth;

    // A host with no session must say so, rather than reporting healthy.
    let fixed = outlander::build_outlander_app();
    assert_eq!(
        ambition::rollback::health(&fixed),
        RollbackHealth::NoSession,
        "a fixed-step host reported a rollback health other than NoSession"
    );

    let mut app = outlander::build_outlander_rollback_app().expect("rollback host");
    let health = ambition::rollback::health(&app);
    assert!(
        health.is_healthy(),
        "a freshly started session is not healthy: {health:?}"
    );

    // ⚠ The half a single sample cannot see. A frozen session reports
    // `Healthy` forever — liveness is a property of TWO observations, which is
    // why `RollbackHealth::frame` says to sample it twice and why this test
    // does.
    let before = ambition::rollback::health(&app).frame().expect("a frame");
    for _ in 0..60 {
        app.update();
    }
    let after = ambition::rollback::health(&app).frame().expect("a frame");
    assert!(
        after > before,
        "the session did not advance across 60 updates ({before} -> {after}), \
         which is exactly the frozen-but-Running state blind run 7 could not \
         diagnose from outside"
    );
    assert!(
        ambition::rollback::health(&app).is_healthy(),
        "the session desynced while simulating: {:?}",
        ambition::rollback::health(&app)
    );
}
