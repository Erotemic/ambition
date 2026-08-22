//! Whether a body's character authors its own attack art — resolved
//! sim-side, so presentation never asks the catalog.
//!
//! `engine.character-authority-is-app-local` forbids exactly that shape, and the reason is not
//! tidiness:
//!
//! an ABSENT catalog read as an EMPTY one. `Option<Res<_>>` is `None` in
//! every headless and test composition that does not install the catalog — the
//! compositions the `Option` was added for — and `None.and_then(…)` is
//! indistinguishable from *"this character authors no attack VFX"*. So
//! `attack_vfx` returned `None`, `authored` was false, and a stand-in volume was
//! drawn over every attack, including the ones whose characters author their
//! own art. Silent, and backwards.
//!
//! It is that presentation should not consult the catalog at all: *does this character
//! author its own attack VFX* is a static per-character fact, and a static per-body fact is
//! what this read-model is for. It already carries the sprite quad for the same reason.
//!
//! # Absent means UNKNOWN, and that is the whole point
//!
//! | state | meaning | what presentation does |
//! |---|---|---|
//! | component absent | the publisher did not run — no catalog in this composition | nothing: it does not know, so it neither draws art nor a stand-in |
//! | [`AttackVfxView::sheet`] `None` | resolved: this body authors no attack art (no worn character, or a character that names no sheet) | draw the stand-in volume |
//! | [`AttackVfxView::sheet`] `Some` | resolved: the character names this sheet | draw its own art |

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

/// Publish [`AttackVfxView`] for every body that can swing.
///
/// the catalog is `Res`, not `Option<Res>`, and that is deliberate: a
/// composition without a catalog must leave the component ABSENT rather than
/// write `None` into it, because `None` is a positive claim that the character
/// authors nothing. Bevy skips a system whose required resource is missing,
/// which is exactly the behaviour wanted here — the *absence* of the resource
/// becomes the *absence* of the fact, instead of being laundered into a value.
///
/// Runs in `FeatureViewSync` beside the other read-model rebuilds. The fact is
/// static per character, so this writes only when the answer actually changes.
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
        // A body with no worn character (a bare fixture, a prop that swings)
        // authors nothing BY DEFINITION — that is a resolved `None`, not an
        // unknown, so it still gets the component and still draws its stand-in.
        let sheet = worn
            .and_then(|worn| catalog.attack_vfx(worn.id()))
            .map(str::to_owned);
        if current.map(|view| &view.sheet) != Some(&sheet) {
            commands.entity(entity).insert(AttackVfxView { sheet });
        }
    }
}
