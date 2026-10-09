//! Saved game, quest, and settings shapes for Ambition.
//!
//! This crate owns the data that can be stored, loaded, and mirrored into
//! Bevy resources. Menu/UI policy stays above this crate and reads these typed
//! settings instead of owning their serialized shape.

pub mod host;
pub mod quest;
mod rollback_registration;
pub mod save;
pub mod store;

pub use rollback_registration::register_rollback_state;

/// The directory this App reads and writes its settings, save and developer
/// files in. Defaults to the platform data dir, so production is unchanged.
///
/// it exists so that "where my files are" is an APP fact. As a global it
/// was shared by every test in a binary and by every process on the machine.
#[derive(bevy::prelude::Resource, Clone, Debug)]
pub struct PersistenceRoot(
    pub std::path::PathBuf,
    // Held for its `Drop` only, which removes an isolated directory: nothing
    // reads it.
    #[allow(dead_code)] Option<std::sync::Arc<IsolatedRoot>>,
);

/// The directory of an isolated root, removed when the last holder of the
/// root is dropped (the App's resource, and each clone of it).
#[derive(Debug)]
pub struct IsolatedRoot(std::path::PathBuf);

impl Drop for IsolatedRoot {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

impl Default for PersistenceRoot {
    fn default() -> Self {
        Self::at(settings::platform_paths::data_dir_root())
    }
}

impl PersistenceRoot {
    /// A root at a path that somebody else owns: the player's data directory,
    /// a directory a test made. It is never removed here.
    pub fn at(path: impl Into<std::path::PathBuf>) -> Self {
        Self(path.into(), None)
    }

    /// A private directory nobody else writes — for an App that is not a
    /// player's session.
    ///
    /// the symmetry to keep in mind: a windowless host already redirects
    /// AUDIO away from the user's speakers (`AudioOutputMode::Recording`). This
    /// is the same rule for the other side effect a non-session App should not
    /// have — writing the user's settings and save.
    ///
    /// Unique per call: process id plus a counter, so two Apps in one test
    /// binary do not share a root either.
    ///
    /// ⛔ IT IS REMOVED WHEN ITS LAST HOLDER IS DROPPED ([`IsolatedRoot`]). To
    /// read what an App wrote after the App is gone, keep a clone of its
    /// `PersistenceRoot`. This comment said "nothing cleans these up,
    /// deliberately ... cheaper than a harness that deletes paths", and that
    /// was a rule nobody gave: the roots grew with no bound (54,893 on one
    /// machine and 133,576 on another in two weeks, 0.9 and 2.3 GB). Jon,
    /// 2026-10-09: an agent may always delete in its own temp directory.
    ///
    /// ⛔ AND NEVER THE LEFTOVER OF A DEAD PROCESS. A process that is killed
    /// leaves its root, and a process id is used again, so `<pid>-<counter>`
    /// alone named the directory of an earlier process with this id: its
    /// `sandbox_save.ron` was read at `Startup` (`load_save_at_startup`) over
    /// the save the harness was given, and a test started in another world.
    /// Measured 2026-10-09 (712 leftovers in the next 100,000 process ids):
    /// about 1 test process in 500 went red, on a different premise each
    /// time. So a native root is CLAIMED: the directory is made here, and a
    /// name that is taken is passed over.
    pub fn isolated() -> Self {
        use std::sync::atomic::{AtomicU64, Ordering};
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let base = std::env::temp_dir().join("ambition-app-state");
        loop {
            let unique = NEXT.fetch_add(1, Ordering::Relaxed);
            let root = base.join(format!("{}-{unique}", std::process::id()));
            if Self::claim(&root) {
                let guard = cfg!(not(target_arch = "wasm32")).then(|| std::sync::Arc::new(IsolatedRoot(root.clone())));
                return Self(root, guard);
            }
        }
    }

    /// Make `root` as a new directory. `false`: it is there already, so it is
    /// not this call's. A temp dir that cannot be written gives the name
    /// unclaimed, as before: there is no leftover to read where nothing can be
    /// written.
    #[cfg(not(target_arch = "wasm32"))]
    fn claim(root: &std::path::Path) -> bool {
        if let Some(parent) = root.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        match std::fs::create_dir(root) {
            Err(error) => error.kind() != std::io::ErrorKind::AlreadyExists,
            Ok(()) => true,
        }
    }

    /// The web has no directories: a root is a key prefix there.
    #[cfg(target_arch = "wasm32")]
    fn claim(_root: &std::path::Path) -> bool {
        true
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod isolated_root_tests {
    use super::PersistenceRoot;

    /// A root from `isolated()` is empty, when an earlier process with this
    /// process id left its files under the names the next calls would have
    /// had. (Other tests of this binary take roots at the same time; they
    /// pass the leftovers over too.)
    #[test]
    fn an_isolated_root_is_never_the_leftover_of_an_earlier_process() {
        let first_root = PersistenceRoot::isolated();
        let first = first_root.0.clone();
        let name = first.file_name().unwrap().to_str().unwrap().to_owned();
        let (pid, counter) = name.rsplit_once('-').expect("a root is <pid>-<counter>");
        let counter: u64 = counter.parse().expect("a counter");
        let base = first.parent().unwrap().to_owned();
        // The leftovers: the next 64 names, each with a save in it.
        let leftovers: Vec<_> = (counter + 1..=counter + 64).map(|n| base.join(format!("{pid}-{n}"))).collect();
        let mut planted = Vec::new();
        for leftover in &leftovers {
            // A name another test took in this instant is not a leftover.
            if std::fs::create_dir(leftover).is_ok() {
                std::fs::create_dir_all(leftover.join("ambition")).unwrap();
                std::fs::write(leftover.join("ambition").join("sandbox_save.ron"), "(stale)").unwrap();
                planted.push(leftover.clone());
            }
        }
        assert!(planted.len() >= 32, "premise: only {} leftovers were planted", planted.len());

        for _ in 0..8 {
            // Held: a root is removed with its last holder.
            let held = PersistenceRoot::isolated();
            let root = &held.0;
            assert!(!planted.contains(root), "`isolated()` gave the directory of an earlier process: {root:?}");
            let entries = std::fs::read_dir(root).expect("the root is a directory this call made").count();
            assert_eq!(entries, 0, "a new root has {entries} entries: {root:?}");
        }
        // Take away what this test planted, and no other path.
        for leftover in planted {
            let _ = std::fs::remove_dir_all(leftover);
        }
    }

    /// An isolated root is a directory while a holder of it lives, and it is
    /// gone when the last holder is dropped. A clone is a holder: it is how a
    /// test reads what an App wrote after the App is gone. A root at a path
    /// somebody else owns (`at`) is not removed.
    #[test]
    fn an_isolated_root_is_removed_with_its_last_holder_and_no_other_root_is() {
        let root = PersistenceRoot::isolated();
        let path = root.0.clone();
        std::fs::write(path.join("a_file"), "written").expect("the root is a directory");
        let kept = root.clone();
        drop(root);
        assert!(path.join("a_file").exists(), "a clone holds the root, and the root is gone");
        drop(kept);
        assert!(!path.exists(), "the root is there after its last holder was dropped");

        let owned = std::env::temp_dir().join(format!("ambition-root-somebody-owns-{}", std::process::id()));
        std::fs::create_dir_all(&owned).unwrap();
        drop(PersistenceRoot::at(&owned));
        assert!(owned.exists(), "a root at a path somebody else owns was removed");
        let _ = std::fs::remove_dir_all(owned);
    }
}
pub mod save_data;
pub mod settings;

/// Schedules user-settings and sandbox-save persistence.
///
/// Headless / RL drivers omit this plugin so they never read or write user files."* The second
/// sentence names a real hazard and the first drew the wrong line from it: the hazard is writing
/// the PLAYER's files, not persisting at all, and the two were conflated because whoever
/// installed it first (the presentation group) decided who paid for it.
///
///  who installs it: any composition that SIMULATES. The durable horizon is sim state — the
/// on-disk form is the checkpoint's own description, serialized — so a headless run that reaches a
/// checkpoint and cannot write one is a capability that exists only when somebody is watching.
///
///  what a non-player App owes: its own [`PersistenceRoot`]. `PersistenceRoot::default()`
/// is the player's platform data dir, so the isolation is the caller's obligation and
/// `PersistenceRoot::isolated()` is the answer — the same redirection a windowless host already
/// makes for audio with `AudioOutputMode::Recording`.
pub struct PersistenceSchedulePlugin;

impl bevy::prelude::Plugin for PersistenceSchedulePlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        use bevy::prelude::{IntoScheduleConfigs as _, Startup, Update};

        // Nothing declared that, and a test cannot opt out of a global.
        //
        // `init_resource` keeps production behaviour exactly (the default IS the
        // platform dir) while letting any App — a test, a tool, a second
        // instance — declare its own root before adding this plugin.
        app.init_resource::<PersistenceRoot>();
        app.init_resource::<save::SaveFileWritable>()
            .init_resource::<save::LastPersistedSave>()
            .init_resource::<save::SaveOwner>()
            .init_resource::<settings::persistence::LastPersistedSettings>()
            .add_systems(
                Startup,
                (
                    settings::persistence::load_settings_at_startup,
                    save::load_save_at_startup,
                ),
            )
            .add_systems(
                Update,
                (
                    settings::persistence::save_settings_on_change,
                    // The sandbox save IS rollback state, so a speculating host
                    // must not commit it to disk while anything is predicted.
                    // On every non-rollback host this condition is always true
                    // and the behaviour is unchanged. See `autosave_sandbox_save`.
                    save::autosave_sandbox_save
                        .run_if(ambition_platformer2d_core::world_state_is_confirmed),
                ),
            );
    }
}

/// Serializes every test that repoints `AMBITION_DATA_DIR`.
///
/// That variable is process-global, and both the save and the settings suites
/// resolve real on-disk paths through it. One lock per module is not mutual
/// exclusion — the suites raced, and each other's scratch directory looked like
/// a missing or unexpected file. A poisoned lock is deliberately tolerated:
/// otherwise the first genuine assertion failure cascades into every later test
/// reporting `PoisonError` instead of its own result.
#[cfg(test)]
pub(crate) fn lock_data_dir() -> std::sync::MutexGuard<'static, ()> {
    static DATA_DIR: std::sync::Mutex<()> = std::sync::Mutex::new(());
    DATA_DIR
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}
