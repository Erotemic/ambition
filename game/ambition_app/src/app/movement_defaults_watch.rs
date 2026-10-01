//! A running development build plays a saved movement-defaults edit.
//!
//! `platformer_defaults.ron` is read once at boot
//! (`Platformer2dGameplayDefaults::load`). In a build that reads it off disk
//! (no `static_content`), this watch looks at the file while the game runs. When
//! it changes, the new tuning is written to `EditableMovementTuning`, the same
//! resource the F3 inspector writes. The developer-edit road then proposes it,
//! the timeline owner admits or refuses it, and it is published to
//! `ActiveMovementTuning` in `PreUpdate`, before the advance
//! (`propose_editable_movement_tuning`). The file's `feel:` values take the
//! same road through `EditableFeelTuning`. A file that does not parse is
//! refused and the running tuning stays. Only a value that changed is written,
//! so a save that changes the feel proposes no movement edit.
//!
//! The starting abilities in the same file are NOT applied while the game runs:
//! they are what a body starts with, and a body that already started keeps its
//! abilities. A change to them is reported, and a restart takes it.

use ambition_platformer2d::actors::assets::gameplay_defaults::{
    Platformer2dGameplayDefaults, PLATFORMER_DEFAULTS_FILE,
};
use ambition_platformer2d::combat::feel::EditableFeelTuning;
use ambition_platformer2d::dev_tools::dev_tools::EditableMovementTuning;
use bevy::prelude::*;

/// The defaults file and the last state of it this watch took.
#[derive(Resource)]
pub struct MovementDefaultsWatch {
    file: std::path::PathBuf,
    seen: Option<std::time::SystemTime>,
    frames_until_poll: u32,
    /// The file as it was last read, to write only what changed.
    last: Option<Platformer2dGameplayDefaults>,
    /// Movement tunings this watch wrote. For tests and the inspector.
    pub applied: u32,
    /// Feel tunings this watch wrote.
    pub applied_feel: u32,
}

impl MovementDefaultsWatch {
    /// Watch `file`, as it is now.
    pub fn new(file: std::path::PathBuf) -> Self {
        let seen = modified(&file);
        let last = std::fs::read_to_string(&file)
            .ok()
            .and_then(|text| Platformer2dGameplayDefaults::parse(&text).ok());
        Self {
            file,
            seen,
            frames_until_poll: POLL_FRAMES,
            last,
            applied: 0,
            applied_feel: 0,
        }
    }
}

/// Frames between two looks; the same rate as the content watch.
const POLL_FRAMES: u32 = 20;

fn modified(path: &std::path::Path) -> Option<std::time::SystemTime> {
    std::fs::metadata(path).and_then(|m| m.modified()).ok()
}

/// Install the watch on the shipped defaults file.
pub(crate) fn register(app: &mut App) {
    app.insert_resource(MovementDefaultsWatch::new(PLATFORMER_DEFAULTS_FILE.into()))
        .add_systems(Update, watch_movement_defaults);
}

/// Look at the file; on a change, write its tuning to the developer mirror.
pub fn watch_movement_defaults(
    mut watch: ResMut<MovementDefaultsWatch>,
    editable: Option<ResMut<EditableMovementTuning>>,
    editable_feel: Option<ResMut<EditableFeelTuning>>,
) {
    if watch.frames_until_poll > 0 {
        watch.frames_until_poll -= 1;
        return;
    }
    watch.frames_until_poll = POLL_FRAMES;
    let now = modified(&watch.file);
    if now == watch.seen {
        return;
    }
    watch.seen = now;
    let text = match std::fs::read_to_string(&watch.file) {
        Ok(text) => text,
        Err(error) => {
            error!("{} changed and is not readable ({error}); the running tuning stays", watch.file.display());
            return;
        }
    };
    let defaults = match Platformer2dGameplayDefaults::parse(&text) {
        Ok(defaults) => defaults,
        Err(error) => {
            error!("{} changed and does not parse; the running tuning stays: {error}", watch.file.display());
            return;
        }
    };
    let last = watch.last.replace(defaults.clone());
    if last.as_ref().is_none_or(|last| last.abilities != defaults.abilities) {
        warn!(
            "{} changed its starting abilities; a running body keeps its own, restart to take them",
            watch.file.display()
        );
    }
    // ⛔ No mirror means no developer-edit road in this composition, and no
    // other road may write the simulation's tuning around the timeline owner.
    if last.as_ref().is_none_or(|last| last.tuning != defaults.tuning) {
        match editable {
            Some(mut editable) => {
                *editable = EditableMovementTuning::from(defaults.tuning);
                watch.applied += 1;
                info!("movement tuning reloaded from {}", watch.file.display());
            }
            None => warn!("{} changed; this composition has no developer tuning road, restart to take it", watch.file.display()),
        }
    }
    if last.as_ref().is_none_or(|last| last.feel != defaults.feel) {
        match editable_feel {
            Some(mut editable) => {
                editable.0 = defaults.feel;
                watch.applied_feel += 1;
                info!("feel tuning reloaded from {}", watch.file.display());
            }
            None => warn!("{} changed its feel; this composition has no developer feel road, restart to take it", watch.file.display()),
        }
    }
}
