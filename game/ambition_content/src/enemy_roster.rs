//! THE Ambition hostile-archetype roster — named, authored game data.
//!
//! The machinery crate (`ambition_platformer2d_actor_monolith`) owns the generic schema and spawn
//! pipeline. This module contributes Ambition's immutable fragment to the
//! current Bevy [`App`](bevy::prelude::App); no process-global install order is
//! involved.

use ambition_platformer2d_actor_monolith::features::{
    CharacterRosterAppExt, CharacterRosterFragment,
};
use bevy::prelude::App;

/// Provider identity used by every Ambition-authored catalog fragment.
pub const PROVIDER_ID: &str = "ambition";

/// The authored hostile roster, embedded at compile time. Top-level keys are
/// the spawn brain keys a `LoadingZone` / encounter authors as
/// `Brain::Custom("…")`; `"combatant"` is Ambition's fallback row.
pub const CHARACTER_ROSTER_RON: &str = include_str!("../assets/data/character_archetypes.ron");

/// Register Ambition's hostile archetypes into this Bevy App.
///
/// ⛔ **THROUGH THE COMPILER, not beside it.** This used to call
/// `CharacterRosterFragment::from_ron(…, CHARACTER_ROSTER_RON)`, re-parsing bytes
/// the pack had already read and judged — the two-readers split the content pack
/// exists to close.
///
/// The `character_archetypes` schema additionally refuses what `from_ron`
/// accepted: an `inherits` naming a row this roster does not define (which used
/// to fall back to the baseline silently, reading as "my inheritance did
/// nothing"), a self-inheriting row, and a `patrol_effort` / `chase_effort`
/// outside `0.0..=1.0` — effort is a FRACTION of `run_speed` (§4.7) and the seam
/// clamps it, so an out-of-range value reads as tuned and behaves identically.
pub fn register(app: &mut App) {
    let by_brain =
        ambition_combat::content_schema::lowered_character_archetypes(crate::pack::prepared())
            .expect("the archetype schema lowers its roster for every pack that compiles")
            .clone();
    app.register_character_roster_fragment(
        CharacterRosterFragment::from_prepared_specs(
            PROVIDER_ID,
            Some("combatant"),
            by_brain,
            CHARACTER_ROSTER_RON,
        )
        .expect("Ambition character_archetypes.ron should be a valid roster fragment"),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_roster_parses_and_registers() {
        let mut app = App::new();
        register(&mut app);
        assert!(app
            .world()
            .contains_resource::<ambition_platformer2d_actor_monolith::features::CharacterRoster>(
            ));
    }

    #[test]
    fn sandbag_archetypes_are_passive() {
        let mut app = App::new();
        register(&mut app);
        assert!(
            app.world()
                .resource::<ambition_platformer2d_actor_monolith::features::CharacterRoster>()
                .sandbags_are_passive(),
            "a sandbag/training-dummy archetype carries a melee attack — passive targets must have `melee: None`"
        );
    }
}
