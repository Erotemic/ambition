//! Named Ambition quest content registration.
//!
//! Owns the install of the default [`QuestRegistry`] so the named quest
//! roster is constructed in one content-owned place instead of inline in
//! `app/sim_resources.rs`. The registry's *contents* (quest definitions)
//! still live in `crate::content::quest`; this module only owns the
//! decision to register the default roster as a sandbox resource.

use bevy::prelude::*;

/// Installs the default Ambition quest registry resource.
pub struct AmbitionQuestContentPlugin;

impl Plugin for AmbitionQuestContentPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(crate::content::quest::QuestRegistry::default());
    }
}
