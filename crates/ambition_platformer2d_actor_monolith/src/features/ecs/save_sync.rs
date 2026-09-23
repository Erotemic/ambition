//! Project persisted switch state onto ECS switch components.
//!
//! ⚠ No actor is mirrored: a persisted death, a cleared boss and a provoked
//! person are BUILT in their recorded state (`construction::PersistedFates`,
//! read when the commit is requested) — census row DUP-PERSISTED-FATE.

use super::*;
use ambition_combat::components::FeatureId;
use ambition_encounter::switches::{SwitchFeature, SwitchOn};

/// Mirror persisted save switch state onto ECS switch components.
///
/// Encounter arming now reads `EncounterSwitchIndex`, which is rebuilt from
/// these ECS components.
pub fn sync_ecs_switches_from_save(
    save: Res<ambition_persistence::save::AmbitionGameSave>,
    mut switches: Query<(&FeatureId, &mut SwitchOn), With<SwitchFeature>>,
) {
    for (id, mut switch_on) in &mut switches {
        switch_on.0 = save.data().switch(id.as_str());
    }
}

#[cfg(test)]
mod switch_save_tests;
