//! Consumer-matrix row 5: Smash.
//!
//! Smash proves "participants, character selection, atomic match lifecycle,
//! scoped rules, rollback".
//!
//! This file was a PARTIAL proof for one day and the matrix recorded it as such — four fifths, with
//! rollback named as the single missing property because ADR 0031 deferred it to its own slice.
//!
//! Keeping the partial honest is what made this cheap.

use ambition_platformer2d::app::prelude::*;

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

impl ambition_platformer2d::bevy::prelude::Plugin for VersusCapability {
    fn build(&self, app: &mut ambition_platformer2d::bevy::prelude::App) {
        ambition_app::app::versus::compose_versus_experience(app);
    }
}

/// A two-participant stage composes through the SDK and seats its cast.
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
        .resource::<ambition_platformer2d::character::CharacterCatalog>();
    let ids: Vec<&String> = catalog.iter().map(|(id, _)| id).collect();
    // Spelled out rather than importing `versus::FIGHTERS`, which is private.
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
        .resource::<ambition_platformer2d::game_shell::ShellRouteCatalog>()
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

/// The fifth property: rollback, with TWO participants.
///
/// Two is the number that matters, and one would have been the easy version of this test. A
/// single-participant Smash proof would reproduce exactly that blind spot in the test that exists
/// to rule it out.
///
/// Participants are declared at COMPOSITION, so the count the session seats and
/// the count the stage was built for are the same fact rather than two facts
/// that usually agree.
#[test]
fn the_versus_stage_rolls_back_with_two_participants() {
    use ambition_platformer2d::rollback::{RollbackPlan, RollbackRegistry};

    let mut app = PlatformerApp::headless()
        .rollback(2)
        .mount(VersusModule)
        .try_build()
        .expect("the versus stage must compose for rollback through the public API");

    let session = ambition_platformer2d::rollback::start(&mut app, RollbackPlan::new())
        .expect("the versus stage must reach a running rollback session");

    assert_eq!(
        session.participants(),
        2,
        "the session seated {} participant(s) for a two-fighter stage — the \
         exact shape of the bug this test exists to rule out",
        session.participants()
    );

    // `session.participants()` is the DECLARATION, not the seating, and
    // blind run 7 proved the difference matters: it declared 1, 2 and 4 and got
    // one body every time. This test was written to avoid the one-input-stream
    // blind spot and reproduced a subtler version of it, so the seating is now
    // asserted separately against the composed cast — the two fighters checked
    // by name in the sibling test above.
    //
    // What is still NOT proven here: that two SEPARATE input streams reach
    // two separate bodies. `ambition_platformer2d::actor::MatchSeat` now answers the QUERY
    // half (see the sibling test), but there is no public seam for driving
    // input to a NAMED seat — `drive_control_frame` writes one frame for the
    // composition. Slice-G finding (g), open, and named rather than papered
    // over: the alternative is a Smash row that reads as proven while the
    // participants half is a number.

    // ATOMIC MATCH LIFECYCLE, under rollback: the match survives resimulation.
    // A session that started and then stopped being live would pass everything
    // above, because starting is the part that is easy — and `is_running()`
    // alone does NOT see that: a frozen sim still reports `Running`.
    let before = ambition_platformer2d::rollback::health(&app)
        .frame()
        .expect("a started session has a frame");
    for _ in 0..120 {
        app.update();
    }
    let health = ambition_platformer2d::rollback::health(&app);
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

/// The match has two distinct seated bodies, and it simulates with both.
///
/// The gamepads were theatre.
///
/// Deleting the theatre leaves a sharper statement of blind run 7's finding (g)
/// than the run made. It reported "N participants get one body". The truth is
/// worse in a more useful way: `RollbackSession::participants()` and the
/// seating are independent facts and nothing reconciles them. The count
/// reaches GGRS (how many input streams are checksum-compared); the seating
/// comes from the stage and its devices. A composition can declare four and
/// seat two, and no error says so.
///
/// So what is asserted here is only what is true: the match has two distinct
/// seats, and the session advances undesynced with both in it.
#[test]
fn the_match_has_two_distinct_seats_and_simulates_with_both() {
    use ambition_platformer2d::rollback::RollbackPlan;

    let mut app = PlatformerApp::headless()
        .rollback(2)
        .mount(VersusModule)
        .build();
    let session = ambition_platformer2d::rollback::start(&mut app, RollbackPlan::new())
        .expect("the versus stage must reach a running rollback session");
    assert_eq!(session.participants(), 2);

    let mut seats: Vec<usize> = Vec::new();
    for _ in 0..600 {
        app.update();
        let world = app.world_mut();
        let mut query = world.query::<&ambition_platformer2d::actor::MatchSeat>();
        seats = query.iter(world).map(|seat| seat.0).collect();
        seats.sort_unstable();
        if seats.len() >= 2 {
            break;
        }
    }

    assert!(
        seats.len() >= 2,
        "the versus match seated {} body/bodies ({seats:?})",
        seats.len()
    );
    let distinct = {
        let mut d = seats.clone();
        d.dedup();
        d.len()
    };
    assert!(
        distinct >= 2,
        "the seated bodies share a seat index ({seats:?}), so they are not \
         distinguishable participants"
    );

    // And the match still simulates with both of them in it. `is_running()`
    // would not see a frozen sim; the frame advancing is the fact.
    let before = ambition_platformer2d::rollback::health(&app)
        .frame()
        .expect("a frame");
    for _ in 0..120 {
        app.update();
    }
    let health = ambition_platformer2d::rollback::health(&app);
    assert!(
        health.frame().expect("a frame") > before && health.is_healthy(),
        "the two-seat match did not advance cleanly: {health:?}"
    );
}

/// The versus stage's CPU roster is seatable by the composition the SDK
/// builds. (API 1.0 row (g))
///
/// it has to run against THIS composition, not the full app. Written first against
/// `versus_app()` — the whole game, which composes `ambition_content` and therefore does have
/// `medium_striker` — and it stayed green with the original bug put back.
#[test]
fn the_versus_cpu_roster_is_satisfiable_by_the_sdk_composition() {
    let app = PlatformerApp::headless()
        .start_at_launcher()
        .mount(VersusModule)
        .try_build()
        .expect("the versus stage must compose through the public API");

    // It called `unsatisfiable_seats(&archetypes, None)` — reading a `CharacterRoster` out of
    // the composed app and passing NO policies — so the guard could only ever see one of the
    // two places a seat's answer could live. The archetype half is gone from the check entirely
    // now.
    //
    // the CLAIM is unchanged and still the one that shipped a statue: this
    // composition, with no content crate under it, must be able to seat its own
    // CPU.
    let profiles = app
        .world()
        .get_resource::<ambition_platformer2d::characters::actor::character_catalog::BrainProfileRegistry>()
        .cloned()
        .unwrap_or_default();

    // One local player: seat 1 is the CPU, which is the default versus
    // experience and the one anybody with a single controller plays.
    let roster = ambition_app::app::versus::versus_roster(1);
    let problems = roster.unsatisfiable_seats(Some(&profiles));
    assert!(
        problems.is_empty(),
        "the versus stage declares a CPU seat the SDK composition cannot seat, \
         so that fighter is a stand-still body: {problems:?}"
    );
}
