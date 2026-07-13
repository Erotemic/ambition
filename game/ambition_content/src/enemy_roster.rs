//! THE Ambition hostile-archetype roster — named, authored game data.
//!
//! The machinery crate (`ambition_actors`) owns the generic schema and spawn
//! pipeline. This module contributes Ambition's immutable fragment to the
//! current Bevy [`App`](bevy::prelude::App); no process-global install order is
//! involved.

use ambition_actors::features::{CharacterRoster, CharacterRosterAppExt, CharacterRosterFragment};
use bevy::prelude::App;

/// Provider identity used by every Ambition-authored catalog fragment.
pub const PROVIDER_ID: &str = "ambition";

/// The authored hostile roster, embedded at compile time. Top-level keys are
/// the spawn brain keys a `LoadingZone` / encounter authors as
/// `Brain::Custom("…")`; `"combatant"` is Ambition's fallback row.
pub const CHARACTER_ROSTER_RON: &str = include_str!("../assets/data/character_archetypes.ron");

/// Register Ambition's hostile archetypes into this Bevy App.
pub fn register(app: &mut App) {
    app.register_character_roster_fragment(
        CharacterRosterFragment::from_ron(PROVIDER_ID, Some("combatant"), CHARACTER_ROSTER_RON)
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
        assert!(app.world().contains_resource::<CharacterRoster>());
    }

    #[test]
    fn sandbag_archetypes_are_passive() {
        let mut app = App::new();
        register(&mut app);
        assert!(
            app.world().resource::<CharacterRoster>().sandbags_are_passive(),
            "a sandbag/training-dummy archetype carries a melee attack — passive targets must have `melee: None`"
        );
    }
}
