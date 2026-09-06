//! A test is not a player, and must not write a player's files.
//!
//! `PersistenceRoot` made that path App state instead of an ambient process fact,
//! and windowless hosts insert `PersistenceRoot::isolated()` beside
//! `AudioOutputMode::Recording` — the same rule for the other side effect a
//! non-session App should not have.
//!
//! this file exists because that is exactly the kind of fix that rots. The
//! isolation is one `insert_resource` inside a `matches!` on the render mode; the
//! `Default` impl is still the real platform directory, so anything that stops
//! taking that branch silently goes back to writing `~/.local/share/ambition/`
//! and every test still passes. Nothing else notices — which is how it got there
//! the first time.

use ambition_app::app::{build_visible_app, VisibleRenderMode};
use ambition_platformer2d::persistence::PersistenceRoot;

/// The shipped windowless host writes somewhere private.
///
/// Compared against `PersistenceRoot::default()` rather than against a hardcoded
/// `~/.local/share/ambition` — the default IS the platform directory, so asking it
/// keeps this true on every platform and cannot drift from the thing it guards.
#[test]
fn a_windowless_host_does_not_write_the_players_directory() {
    let app = build_visible_app(VisibleRenderMode::NoWindow, true);
    let root = app
        .world()
        .get_resource::<PersistenceRoot>()
        .expect("the persistence root is App state, so a composed App always has one");
    let players_dir = PersistenceRoot::default();

    assert_ne!(
        root.0, players_dir.0,
        "a windowless host must not resolve to the player's own data directory \
         ({:?}) — that is the shared per-USER path three mutable files live in, \
         and a test, a capture or an acceptance run is not a player",
        players_dir.0
    );
    assert!(
        root.0.starts_with(std::env::temp_dir()),
        "and it should be under the temp dir, which is what makes it disposable: \
         got {:?}",
        root.0
    );
}

/// Two Apps in one process must not share a root either.
#[test]
fn two_isolated_roots_in_one_process_are_different_directories() {
    let first = PersistenceRoot::isolated();
    let second = PersistenceRoot::isolated();
    assert_ne!(
        first.0, second.0,
        "isolated() is unique per call (pid + counter); two Apps in one test \
         binary sharing a root is the same bug as sharing the player's"
    );
}

/// A windowless host arrives with its clock PINNED.
///
/// Stating a lesson is what a rule does instead of enforcing it.
///
/// the same rot risk as the persistence guard above, and the same answer:
/// the pin is one `insert_resource` inside a `matches!` on the render mode, the
/// Bevy default is still `Automatic`, and anything that stops taking that branch
/// silently goes back to a wall-clock frame with every test still passing.
///
/// it asserts the DEFAULT is not what we get, rather than a specific duration, so a host
/// that pins a different dt on purpose stays green while a host that pins nothing does not.
#[test]
fn a_windowless_host_does_not_advance_its_clock_by_wall_clock() {
    let app = build_visible_app(VisibleRenderMode::NoWindow, true);
    let strategy = app
        .world()
        .get_resource::<bevy::time::TimeUpdateStrategy>()
        .expect(
            "a windowless host inserts a TimeUpdateStrategy — without one Bevy \
             uses Automatic, and `app.update()` becomes a unit of wall clock",
        );
    assert!(
        !matches!(strategy, bevy::time::TimeUpdateStrategy::Automatic),
        "a windowless host has no display to pace against, so advancing its \
         clock by real elapsed time makes every count-like assertion in every \
         test depend on how busy the machine is"
    );
}

/// A HEADLESS COMPOSITION CAN PERSIST — and it writes to a root of its own.
///
/// The durable horizon is SIM state — its own row says *"the on-disk form IS the checkpoint's own
/// description, serialized"* — so a composition that simulates should be able to persist.
///
/// and the root matters as much as the plugin. `PersistenceRoot::default()`
/// is the PLAYER's platform data dir; installing the writer without an isolated
/// root would point every headless run at the user's real save. Both halves are
/// asserted here, because installing one without the other is worse than
/// neither.
#[test]
fn a_headless_sim_persists_into_a_root_of_its_own() {
    use ambition_app::{AmbitionSim as _, Platformer2dSimHarness};

    let mut sim = Platformer2dSimHarness::new().expect("the headless sim composes");
    sim.step_n(crate::common::base(), 8);

    let root = sim
        .world()
        .get_resource::<PersistenceRoot>()
        .expect("a headless composition installs persistence at all")
        .0
        .clone();
    assert_ne!(
        root,
        PersistenceRoot::default().0,
        "a headless run is pointed at the PLAYER's save directory"
    );
    assert!(
        sim.world()
            .get_resource::<ambition_platformer2d::persistence::save::SaveFileWritable>()
            .is_some(),
        "the save writer's own resources are absent, so the plugin is not really \
         installed however the root reads"
    );
}
