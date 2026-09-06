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
//! This helper keeps every manually driven App on the same lifecycle contract as
//! a runner without scattering hand-written `finish()` calls across tests/tools.

use bevy::prelude::App;

/// Bring a hand-driven `App` to the state a runner would leave it in, then run
/// one update.
///
/// NOT idempotent on Bevy's behalf. `App::finish` walks the ENTIRE plugin
/// registry every time it is called and re-runs each plugin's `finish` — it does
/// not remember which ones already ran. A plugin whose `finish` CONSUMES
/// something must guard itself; character preparation republished an empty
/// registry on the second call before it did (see the test below).
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

    /// `App::update` does not finalize plugins, and this repository drives
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

    /// `App::finish` re-runs every plugin's `finish`, every time.
    ///
    /// The dangerous half of this helper, pinned as a test because the obvious
    /// assumption is the opposite one and it cost real debugging: character
    /// preparation consumes its staged overrides at the barrier, so a second
    /// `finish` published an empty cast over a good one. A plugin that consumes
    /// anything in `finish` must guard itself — this helper cannot do it for it,
    /// and neither does Bevy.
    #[test]
    fn finishing_twice_runs_every_plugin_finish_twice() {
        #[derive(Resource, Default)]
        struct FinishCount(usize);

        struct CountsFinishes;

        impl Plugin for CountsFinishes {
            fn build(&self, app: &mut App) {
                app.init_resource::<FinishCount>();
            }

            fn finish(&self, app: &mut App) {
                app.world_mut().resource_mut::<FinishCount>().0 += 1;
            }
        }

        let mut app = App::new();
        app.add_plugins(CountsFinishes);
        finalize_and_update(&mut app);
        finalize_and_update(&mut app);
        assert_eq!(
            app.world().resource::<FinishCount>().0,
            2,
            "Bevy started tracking which plugins have been finished — if so, the \
             self-guards written against this behaviour (character preparation's \
             `finalized` flag) are now dead weight and should be removed"
        );
    }
}
