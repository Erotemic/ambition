//! Cutscene playback: the half of a cutscene that decides what the player can do.
//!
//! ⛔ **`rollback_coverage` waives the whole `ambition_cutscene::` namespace as
//! "scripted presentation sequence state", and playback is not that.**
//! `ActiveCutscene::is_playing()` drives a CAPTURING input-context claim
//! (`CUTSCENE_CONTEXT`), so while a cutscene plays the participant's gameplay
//! input is suppressed. Whether the player can ACT is gameplay truth: a rewind
//! into a playing frame that did not restore this would let the resimulation act
//! through beats the original could not, and GGRS would report that only as a
//! checksum difference.
//!
//! ⚠ the snapshot carries the SEMANTIC half alone — script, beat index, elapsed,
//! finished. The dialogue line, banner, camera target and fade alpha are derived
//! and re-published by the tick, so encoding them would put four copies of one
//! fact in every snapshot and invite them to disagree with the beat index that
//! produced them. See `ambition_cutscene`'s `snapshot` module.

use bevy::prelude::App;

use super::super::AmbitionRollbackApp;

const OWNER: &str = "ambition_cutscene";

/// Register the playback state a rewind has to reproduce.
pub(in crate::rollback) fn register(app: &mut App) {
    // ⚠ **OPTIONAL-canonical, not canonical.** `CutscenePlugin` is installed by
    // compositions that HAVE cutscenes; a bare oracle harness or a demo without
    // scripted beats carries no `ActiveCutscene`, and the plain canonical form
    // installs a checksum system taking `Res<T>` that panics on every frame the
    // resource is absent. The lifecycle domain next door learned that by turning
    // eight rollback-oracle tests red.
    app.rollback_resource_optional_canonical::<ambition_cutscene::ActiveCutscene>(
        OWNER,
        "cutscene.playback",
    );
}
