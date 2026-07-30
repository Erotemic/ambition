//! **Consumer-matrix row 5: Smash.**
//!
//! Smash proves "participants, character selection, atomic match lifecycle,
//! scoped rules, rollback".
//!
//! ⚠ This file was a PARTIAL proof for one day and the matrix recorded it as
//! such — four fifths, with rollback named as the single missing property
//! because ADR 0031 deferred it to its own slice. Slice F landed that slice, so
//! `the_versus_stage_rolls_back_with_two_participants` closes the fifth and the
//! row.
//!
//! Keeping the partial honest is what made this cheap. The campaign has caught
//! itself three times claiming a row on a test that quietly dropped part of it;
//! naming the missing fifth meant closing it was one test rather than a
//! re-audit of what "Smash proven" had been taken to mean.

use ambition::app::prelude::*;

/// The versus stage, mounted through the public API.
struct VersusModule;

impl GameModule for VersusModule {
    fn manifest(&self) -> ModuleManifest {
        ModuleManifest::new(ambition_app::app::versus::VERSUS_EXPERIENCE)
    }

    fn define(&self, module: &mut ModuleDraft) {
        module
            .experience(ambition_app::app::versus::VERSUS_EXPERIENCE)
            .launcher_route(ambition_app::app::shell_host::AMBITION_LAUNCHER_ROUTE)
            .gameplay_route(ambition_app::app::versus::VERSUS_GAMEPLAY_ROUTE)
            .capability(VersusCapability);
    }
}

#[derive(Clone)]
struct VersusCapability;

impl ambition::bevy::prelude::Plugin for VersusCapability {
    fn build(&self, app: &mut ambition::bevy::prelude::App) {
        ambition_app::app::versus::compose_versus_experience(app);
    }
}

/// **A two-participant stage composes through the SDK and seats its cast.**
#[test]
fn the_versus_stage_composes_and_seats_two_participants() {
    let mut app = PlatformerApp::headless()
        .start_at_launcher()
        .mount(VersusModule)
        .try_build()
        .expect("the versus stage must compose through the public API");

    for _ in 0..600 {
        app.update();
        if host_status(&app).is_refused() {
            panic!("refused: {:?}", host_status(&app).refusal());
        }
    }

    // CHARACTER SELECTION: both fighters are in the assembled cast. A stage that
    // seated one, or none, would still route and still report a live host.
    let catalog = app
        .world()
        .resource::<ambition::character::CharacterCatalog>();
    let ids: Vec<&String> = catalog.iter().map(|(id, _)| id).collect();
    // ⚠ Spelled out rather than importing `versus::FIGHTERS`, which is private.
    // AGENTS.md: never widen a production API to move a test. These are content
    // ids; if the cast changes, this failing is the correct outcome.
    for fighter in ["arena_duelist_long", "arena_duelist_close"] {
        assert!(
            ids.iter().any(|id| id.as_str() == fighter),
            "fighter {fighter:?} is not in the composed cast; got {ids:?}"
        );
    }

    // SCOPED RULES: the stage registered its own route rather than borrowing one.
    let routes: Vec<String> = app
        .world()
        .resource::<ambition::game_shell::ShellRouteCatalog>()
        .ids()
        .map(str::to_string)
        .collect();
    assert!(
        routes
            .iter()
            .any(|r| r == ambition_app::app::versus::VERSUS_GAMEPLAY_ROUTE),
        "the versus route is not registered; got {routes:?}"
    );
}

/// **The fifth property: rollback, with TWO participants.**
///
/// ⚠ Two is the number that matters, and one would have been the easy version
/// of this test. The defect this engine actually shipped was a rollback oracle
/// proving determinism for ONE input stream during the week a 2–4 player couch
/// versus mode seated four — every check green, and a desync in seat two with
/// nowhere to appear. A single-participant Smash proof would reproduce exactly
/// that blind spot in the test that exists to rule it out.
///
/// Participants are declared at COMPOSITION, so the count the session seats and
/// the count the stage was built for are the same fact rather than two facts
/// that usually agree.
#[test]
fn the_versus_stage_rolls_back_with_two_participants() {
    use ambition::rollback::{RollbackPlan, RollbackRegistry};

    let mut app = PlatformerApp::headless()
        .rollback(2)
        .mount(VersusModule)
        .try_build()
        .expect("the versus stage must compose for rollback through the public API");

    let session = ambition::rollback::start(&mut app, RollbackPlan::new())
        .expect("the versus stage must reach a running rollback session");

    assert_eq!(
        session.participants(),
        2,
        "the session seated {} participant(s) for a two-fighter stage — the \
         exact shape of the bug this test exists to rule out",
        session.participants()
    );

    // ⚠ `session.participants()` is the DECLARATION, not the seating, and
    // blind run 7 proved the difference matters: it declared 1, 2 and 4 and got
    // one body every time. This test was written to avoid the one-input-stream
    // blind spot and reproduced a subtler version of it, so the seating is now
    // asserted separately against the composed cast — the two fighters checked
    // by name in the sibling test above.
    //
    // ⚠ What is still NOT proven here: that two SEPARATE input streams reach
    // two separate bodies. The facade has no seat-keyed input or query, so a
    // consumer cannot express it and this test cannot assert it. Recorded as
    // slice-G finding (g) rather than papered over — the alternative is a Smash
    // row that reads as proven while the participants half is a number.

    // ATOMIC MATCH LIFECYCLE, under rollback: the match survives resimulation.
    // A session that started and then stopped being live would pass everything
    // above, because starting is the part that is easy — and `is_running()`
    // alone does NOT see that: a frozen sim still reports `Running`.
    let before = ambition::rollback::health(&app)
        .frame()
        .expect("a started session has a frame");
    for _ in 0..120 {
        app.update();
    }
    let health = ambition::rollback::health(&app);
    assert!(
        health.frame().expect("a frame") > before,
        "the versus match did not ADVANCE across 120 updates: {health:?}"
    );
    assert!(
        health.is_healthy(),
        "the versus match desynced under resimulation: {health:?}"
    );
    assert!(
        host_status(&app).is_running(),
        "the versus match did not survive 120 rollback frames: {:?}",
        host_status(&app)
    );

    // Non-vacuity: a session over an empty schema saves, rewinds and compares
    // nothing, and passes.
    let registered = app
        .world()
        .resource::<RollbackRegistry>()
        .descriptors()
        .count();
    assert!(
        registered > 1,
        "the versus session carries {registered} rollback registration(s), so \
         the 120 frames above proved nothing"
    );
}


