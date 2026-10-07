//! Named Ambition dialogue / cutscene content registration.
//!
//! Owns the install of the named cutscene library, the room → cutscene
//! bindings, the dialogue voiceprint catalog, and the combat-banter registry
//! (boss + pirate barks). The named cutscene *content* lives in
//! [`cutscene_defaults`]; the reusable runtime
//! types live in `ambition_cutscene` and the playback systems in
//! `ambition_platformer2d_actor_monolith::cutscene`. The banter *content* lives in
//! `crate::banter` / `crate::bosses`; this module only owns assembling those
//! named rosters into sandbox resources.
//!
//! Intro raider barks and intro cutscene scripts are layered on top by
//! `crate::intro::IntroPlugin` (installed via the content plugin), which
//! extends these registries at startup.

use bevy::prelude::*;

pub mod cutscene_defaults;
mod voiceprints;
/// The authored Yarn dialogue set (sources, the Yarn Spinner plugin
/// constructor, and the validator's known-id surface).
pub mod yarn;

#[cfg(feature = "ui")]
pub use yarn::yarn_spinner_plugin;
pub use yarn::{known_dialogue_ids, yarn_sources};

/// Installs Ambition dialogue voices, cutscenes, and combat-banter content.
pub struct AmbitionDialogueContentPlugin;

impl Plugin for AmbitionDialogueContentPlugin {
    fn build(&self, app: &mut App) {
        voiceprints::register(app);
        app.insert_resource(ambition_cutscene::ActiveCutscene::default())
            .insert_resource(ambition_cutscene::CutsceneTriggerQueue::default());
        // The registries are shared: other plugins (the intro) add their rows
        // too. So this plugin adds its rows and does not replace the registry,
        // and the order in which the plugins are added does not matter.
        let world = app.world_mut();
        let pack = crate::pack::select(world);
        world
            .get_resource_or_init::<ambition_cutscene::CutsceneLibrary>()
            .scripts
            .extend(cutscene_defaults::cutscene_library_of(&pack).scripts);
    }
}
