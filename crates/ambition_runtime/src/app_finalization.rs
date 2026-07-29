//! Finish a manually driven `App` the way a runner would.
//!
//! # The trap this exists to close
//!
//! Bevy runs `Plugin::finish` and `Plugin::cleanup` from the RUNNER, not from
//! `App::update`. Read in `bevy_app` 0.18.1 rather than assumed:
//!
//! * `run_once` — the default runner — does `app.finish(); app.cleanup();
//!   app.update();`
//! * `ScheduleRunnerPlugin` calls `app.finish()` before its loop
//! * `App::update()` does neither. It checks that no plugin is mid-build and
//!   then runs the sub-apps.
//!
//! This repository drives `App::update` by hand almost everywhere: every
//! rendered test, the external-consumer fixture, the rollback harnesses, the
//! headless acceptance runners. Every one of those apps has plugins whose
//! `finish` has never run.
//!
//! Today that costs nothing, because nothing in the workspace implements
//! `finish` — verified: zero occurrences. It stops costing nothing the moment
//! character preparation seals its registry there
//! ([`docs/planning/character-preparation-finalization-plan.md`]), because then
//! production would receive a sealed complete registry while every test and
//! tool silently kept only the preparation fragments. Green tests, wrong game —
//! and the failure would read as a preparation bug rather than a lifecycle one.
//!
//! So the helper lands BEFORE the thing that needs it, and the audit has one
//! place to point at instead of a scattering of hand-written `finish()` calls.

use bevy::prelude::App;

/// Bring a hand-driven `App` to the state a runner would leave it in, then run
/// one update.
///
/// Idempotent by Bevy's own accounting: `finish`/`cleanup` only act on plugins
/// that have not had them run, so calling this on an app a runner already
/// finalized is a plain update.
pub fn finalize_and_update(app: &mut App) {
    finalize(app);
    app.update();
}

/// Bring a hand-driven `App` to the state a runner would leave it in, WITHOUT
/// updating.
///
/// For a caller that wants to inspect the finalized world before any system has
/// run — which is exactly what a test of a `finish`-time barrier wants.
pub fn finalize(app: &mut App) {
    app.finish();
    app.cleanup();
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::prelude::*;

    #[derive(Resource, Default)]
    struct FinishRan(bool);

    struct SealsAtFinish;

    impl Plugin for SealsAtFinish {
        fn build(&self, app: &mut App) {
            app.init_resource::<FinishRan>();
        }

        fn finish(&self, app: &mut App) {
            app.world_mut().resource_mut::<FinishRan>().0 = true;
        }
    }

    /// **`App::update` does not finalize plugins**, and this repository drives
    /// `update` by hand nearly everywhere.
    ///
    /// Pinned as a TEST rather than trusted as a comment because the whole
    /// character-preparation plan rests on it: if this ever became false the
    /// helper would be dead weight, and if it stays true a hand-driven app that
    /// forgets the helper gets an unsealed registry while production gets a
    /// sealed one.
    #[test]
    fn a_hand_driven_update_leaves_plugins_unfinished() {
        let mut app = App::new();
        app.add_plugins(SealsAtFinish);
        app.update();
        assert!(
            !app.world().resource::<FinishRan>().0,
            "`App::update` finalized plugins — if Bevy changed this, the \
             finalization helper and the audit it exists for are unnecessary"
        );
    }

    /// And the helper does what the runner would.
    #[test]
    fn the_helper_finalizes_what_a_runner_would() {
        let mut app = App::new();
        app.add_plugins(SealsAtFinish);
        finalize_and_update(&mut app);
        assert!(
            app.world().resource::<FinishRan>().0,
            "the helper must leave a hand-driven app in the state a runner does"
        );
    }

    /// Finalizing twice is harmless: Bevy tracks which plugins have been
    /// finished, so a helper called on an already-finalized app is an update.
    #[test]
    fn finalizing_an_already_finalized_app_is_harmless() {
        let mut app = App::new();
        app.add_plugins(SealsAtFinish);
        finalize_and_update(&mut app);
        finalize_and_update(&mut app);
        assert!(app.world().resource::<FinishRan>().0);
    }
}
