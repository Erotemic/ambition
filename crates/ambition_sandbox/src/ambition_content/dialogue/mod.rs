//! Named Ambition dialogue / cutscene content registration.
//!
//! Owns the install of the named cutscene library, the room → cutscene
//! bindings, and the combat-banter registry (boss + pirate barks). The
//! cutscene/banter *content* still lives in `crate::presentation::cutscene`
//! and `crate::content::banter` / `crate::boss_encounter`; this module only
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
        app.insert_resource(crate::presentation::cutscene::default_cutscene_library())
            .insert_resource(crate::presentation::cutscene::ActiveCutscene::default())
            .insert_resource(crate::presentation::cutscene::CutsceneTriggerQueue::default())
            .insert_resource(crate::presentation::cutscene::CutsceneAdvanceRequest::default())
            .insert_resource(crate::presentation::cutscene::RoomCutsceneBindings::defaults())
            // Combat-banter registry — story-content lines for the
            // `apply_feature_hit_events` hit handler. Boss barks are
            // installed inline; IntroPlugin adds the intro raiders' lines
            // via a startup system.
            .insert_resource({
                let mut reg = crate::content::banter::CombatBanterRegistry::default();
                crate::boss_encounter::install_boss_banter(&mut reg);
                crate::content::banter::install_pirate_banter(&mut reg);
                reg
            });
    }
}
