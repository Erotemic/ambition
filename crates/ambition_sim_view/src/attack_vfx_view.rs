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
///
/// ⛔⛔ THE FILTER IS LOAD-BEARING AND IT WAS MISSING. Both of the other terms are
/// `Option`, so an unfiltered query here matches EVERY ENTITY IN THE WORLD — and
/// the write below fires for any entity that does not already carry the
/// component, because an absent `AttackVfxView` never equals a resolved
/// `Some(&None)`. Measured 2026-08-29 in a two-fighter Smash match: **1297 of
/// 2048 entities carried `AttackVfxView`**, the largest population in the world,
/// stamped onto falling-sand chunks and UI nodes alike for two bodies that could
/// use it.
///
/// ⚠ THE COST IS NOT THE COMPONENT, IT IS THE ARCHETYPES. A component added to
/// an unrelated entity moves that entity into a new archetype, and archetype
/// count is what every query in the app pays to match against.
///
/// ⭐ THE FILTER IS A UNION ON PURPOSE. `BodyKinematics` is what makes something
/// a simulation body, and `WornCharacter` is what resolves a sheet — the module
/// docs above turn on absent-vs-`None` being meaningful, so anything either
/// consumer could reasonably ask about must still be published rather than
/// silently reading as unresolved.
pub fn rebuild_attack_vfx_views(
    mut commands: Commands,
    catalog: Res<ambition_characters::actor::character_catalog::CharacterCatalog>,
    bodies: Query<
        (
            Entity,
            Option<&ambition_characters::actor::WornCharacter>,
            Option<&AttackVfxView>,
        ),
        bevy::prelude::Or<(
            bevy::prelude::With<ambition_platformer2d_core::BodyKinematics>,
            bevy::prelude::With<ambition_characters::actor::WornCharacter>,
        )>,
    >,
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
