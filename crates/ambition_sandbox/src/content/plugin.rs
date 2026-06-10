//! [`AmbitionContentPlugin`] — the single composer that owns registration
//! of named Ambition game content.
//!
//! Stage 11 / Task J groups the named-content registrations that used to be
//! inlined in `app/sim_resources.rs` behind one explicit plugin so the app
//! assembly installs content through a single seam:
//!
//! ```ignore
//! app.add_plugins(crate::ambition_content::AmbitionContentPlugin);
//! ```
//!
//! This stage is about *registration ownership*, not relocating every
//! implementation file. The named rosters / registries are constructed here
//! (or in the per-content submodules this composes), while the heavy content
//! data + systems still live in their current modules
//! (`crate::content::quest`, `crate::boss_encounter`, `crate::ambition_content::intro`,
//! `crate::presentation::cutscene`, …) and are merely *called from* here.
//!
//! What this composer owns today:
//!
//! - quest registry default roster ([`super::quests`])
//! - boss encounter default roster ([`super::bosses`])
//! - named cutscene library + room bindings + combat-banter registry
//!   ([`super::dialogue`])
//! - the intro / cut-rope story content hooks ([`crate::ambition_content::intro::IntroPlugin`])
//! - the Ambition portal input/inventory adapters
//!   ([`super::portal::AmbitionPortalAdaptersPlugin`], `portal` feature)
//!
//! What it intentionally does NOT own yet (see module docs for why):
//!
//! - the starter item roster ([`super::items::AmbitionItemRosterPlugin`]) —
//!   installed from the presentation assembly to keep its original
//!   insertion point, so headless builds keep their pre-Stage-11 behavior.
//! - named world manifest / asset-ID bindings — those are constructed
//!   eagerly inside `init_sandbox_resources` (the LDtk project + asset
//!   catalog must exist before room-set validation), which is too entangled
//!   with app-assembly ordering to move this pass.
//! - runtime music request channels (`EncounterMusicRequest`,
//!   `RoomMusicRequest`, …) and the empty `EncounterRegistry` default —
//!   these are mechanic-runtime state populated from LDtk, not named
//!   content rosters.

use bevy::prelude::*;

/// Installs all named Ambition game-content registration.
///
/// Mounted by `app::plugins::add_simulation_plugins` (after the runtime /
/// mechanic plugins) so both visible and headless builds register the same
/// named content in the same place.
pub struct AmbitionContentPlugin;

impl Plugin for AmbitionContentPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(super::quests::AmbitionQuestContentPlugin);
        app.add_plugins(super::bosses::AmbitionBossContentPlugin);
        app.add_plugins(super::dialogue::AmbitionDialogueContentPlugin);

        // Intro story content plugin. Extends CutsceneLibrary +
        // RoomCutsceneBindings (always) and GameAssets.characters.npcs
        // (visible builds only — the sprite installer is a no-op in
        // headless where GameAssets is absent). Keeps story content out
        // of sandbox-owned files in preparation for a future
        // sandbox / game crate split.
        app.add_plugins(crate::ambition_content::intro::IntroPlugin);

        // Ambition-specific portal adapters (ControlFrame → portal intents,
        // inventory drop glue). Lives under the content composer so all
        // Ambition content registration flows through one place; the
        // reusable portal core (`crate::portal::PortalPlugin`) is still
        // installed separately in `add_simulation_plugins`.
        #[cfg(feature = "portal")]
        app.add_plugins(super::portal::AmbitionPortalAdaptersPlugin);
    }
}
