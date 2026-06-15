//! Named Ambition dialogue / cutscene content registration.
//!
//! Owns the install of the named cutscene library, the room → cutscene
//! bindings, and the combat-banter registry (boss + pirate barks). The
//! cutscene/banter *content* still lives in `ambition_render::cutscene`
//! and `crate::banter` / `crate::bosses`; this module only
//! owns assembling those named rosters into sandbox resources.
//!
//! Intro raider barks and intro cutscene scripts are layered on top by
//! `crate::intro::IntroPlugin` (installed via the content plugin), which
//! extends these registries at startup.

use bevy::prelude::*;

/// Installs the named Ambition cutscene + combat-banter content resources.
pub struct AmbitionDialogueContentPlugin;

impl Plugin for AmbitionDialogueContentPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ambition_render::cutscene::default_cutscene_library())
            .insert_resource(ambition_render::cutscene::ActiveCutscene::default())
            .insert_resource(
                ambition_render::cutscene::CutsceneTriggerQueue::default(),
            )
            .insert_resource(
                ambition_render::cutscene::CutsceneAdvanceRequest::default(),
            )
            .insert_resource(
                ambition_render::cutscene::RoomCutsceneBindings::defaults(),
            )
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
