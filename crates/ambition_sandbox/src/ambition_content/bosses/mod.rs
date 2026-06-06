//! Named Ambition boss content registration.
//!
//! Owns the install of the default [`BossEncounterRegistry`] so the named
//! boss roster is constructed in one content-owned place. The boss
//! definitions / profiles still live in `crate::boss_encounter`; this
//! module only owns registering the default roster as a sandbox resource.

use bevy::prelude::*;

/// Installs the default Ambition boss encounter registry resource.
pub struct AmbitionBossContentPlugin;

impl Plugin for AmbitionBossContentPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(crate::boss_encounter::BossEncounterRegistry::default());
    }
}
