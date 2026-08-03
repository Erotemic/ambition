//! **A test is not a player, and must not write a player's files.**
//!
//! `PersistenceSchedulePlugin` loads user settings AND the sandbox save at
//! `Startup` and autosaves both. Its root used to be resolved from the
//! environment at every call, which made it PER-USER rather than per-App: every
//! `app_it` test shared one settings file, one save and one developer file — with
//! every other test in the binary, with every other checkout on the machine, and
//! with any concurrent session. A headless acceptance run could overwrite a real
//! save.
//!
//! `PersistenceRoot` made that path App state instead of an ambient process fact,
//! and windowless hosts insert `PersistenceRoot::isolated()` beside
//! `AudioOutputMode::Recording` — the same rule for the other side effect a
//! non-session App should not have.
//!
//! ⚠ **this file exists because that is exactly the kind of fix that rots.** The
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

/// ⭐ **Two Apps in one process must not share a root either.**
///
/// This is the half that the "is it the platform dir" check above cannot see: a
/// single fixed temp path would pass that test and still put every App in one
/// binary back on shared mutable files — the original defect with a different
/// directory name.
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
