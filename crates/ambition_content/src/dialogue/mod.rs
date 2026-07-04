//! Named Ambition dialogue / cutscene content registration.
//!
//! Owns the install of the named cutscene library, the room → cutscene
//! bindings, and the combat-banter registry (boss + pirate barks). The named
//! cutscene *content* lives in [`cutscene_defaults`]; the reusable runtime
//! types live in `ambition_cutscene` and the playback systems in
//! `ambition_gameplay_core::cutscene`. The banter *content* lives in
//! `crate::banter` / `crate::bosses`; this module only owns assembling those
//! named rosters into sandbox resources.
//!
//! Intro raider barks and intro cutscene scripts are layered on top by
//! `crate::intro::IntroPlugin` (installed via the content plugin), which
//! extends these registries at startup.

use bevy::prelude::*;

pub mod cutscene_defaults;
/// The authored Yarn dialogue set (sources, the Yarn Spinner plugin
/// constructor, and the validator's known-id surface).
pub mod yarn;

#[cfg(feature = "ui")]
pub use yarn::yarn_spinner_plugin;
pub use yarn::{known_dialogue_ids, YARN_SOURCES};

/// Installs the named Ambition cutscene + combat-banter content resources.
pub struct AmbitionDialogueContentPlugin;

impl Plugin for AmbitionDialogueContentPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(cutscene_defaults::default_cutscene_library())
            .insert_resource(ambition_cutscene::ActiveCutscene::default())
            .insert_resource(
                ambition_gameplay_core::cutscene_trigger::CutsceneTriggerQueue::default(),
            )
            .insert_resource(ambition_cutscene::CutsceneAdvanceRequest::default())
            .insert_resource(cutscene_defaults::default_room_cutscene_bindings())
            // Combat-banter registry — story-content lines for the
            // `apply_feature_hit_events` hit handler. Boss barks are
            // installed inline; IntroPlugin adds the intro raiders' lines
            // via a startup system.
            .insert_resource({
                let mut reg = crate::banter::CombatBanterRegistry::default();
                crate::bosses::install_boss_banter(&mut reg);
                crate::banter::install_pirate_banter(&mut reg);
                reg
            });
    }
}
