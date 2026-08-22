//! Resolved attack-art facts for presentation.
//!
//! The component distinguishes an unpublished fact (component absent) from a
//! resolved character with no attack sheet (`sheet: None`). Presentation must
//! not read the character catalog or collapse those states, because a missing
//! catalog resource does not imply unauthored art.

use bevy::prelude::{Commands, Component, Entity, Query, Res};

/// The attack-VFX sheet the character this body wears authors, resolved
/// sim-side from the character catalog. See the module docs for why the ABSENT
/// case is not the same as `sheet: None`.
#[derive(Component, Clone, Debug, PartialEq, Eq)]
pub struct AttackVfxView {
    /// `Some` when this body's worn character names its own attack sheet.
    pub sheet: Option<String>,
}

impl AttackVfxView {
    /// True when the character's own art draws its attacks, so the unauthored
    /// stand-in must stay out of the way.
    pub fn authored(&self) -> bool {
        self.sheet.is_some()
    }
}

/// Publish resolved attack-art state for every body.
///
/// The catalog is required rather than optional so compositions without one
/// leave the fact unpublished. Writes occur only when the resolved sheet changes.
pub fn rebuild_attack_vfx_views(
    mut commands: Commands,
    catalog: Res<ambition_characters::actor::character_catalog::CharacterCatalog>,
    bodies: Query<(
        Entity,
        Option<&ambition_characters::actor::WornCharacter>,
        Option<&AttackVfxView>,
    )>,
) {
    for (entity, worn, current) in &bodies {
        // A body with no worn character is a resolved no-sheet case.
        let sheet = worn
            .and_then(|worn| catalog.attack_vfx(worn.id()))
            .map(str::to_owned);
        if current.map(|view| &view.sheet) != Some(&sheet) {
            commands.entity(entity).insert(AttackVfxView { sheet });
        }
    }
}
