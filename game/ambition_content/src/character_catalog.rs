//! Ambition's character-catalog DATA + the curated playable cast —
//! CONTENT, evicted from the engine core (R3.2, violations #3 and #10).
//!
//! The catalog schema, parser, and App-local fragment registry live in
//! `ambition_characters::actor::character_catalog`. Runtime systems consume the
//! assembled `CharacterCatalog` resource. The RON stays a loose file here so the Python tools
//! (`ambition_ldtk_tools.codegen_character_catalog`, the hall generator)
//! keep reading it off disk.

/// The authored roster RON (compile-time include; single source of truth
/// shared with the off-disk tooling).
pub const CHARACTER_CATALOG_RON: &str = include_str!("../assets/data/character_catalog.ron");

/// Parse Ambition's checked-in catalog into an explicit immutable value.
///
/// Tests and pure validators use this without constructing a Bevy `App`.
/// Runtime composition should use [`register`].
pub fn load_catalog() -> ambition_characters::actor::character_catalog::CharacterCatalog {
    ambition_characters::actor::character_catalog::CharacterCatalog::from_data(
        ambition_characters::actor::character_catalog::parse_catalog(CHARACTER_CATALOG_RON),
    )
}

/// Register Ambition's immutable character fragment in one Bevy `App` and
/// rebuild the deterministic assembled catalog resource.
pub fn register(app: &mut bevy::prelude::App) {
    use ambition_characters::actor::character_catalog::{
        CharacterCatalogAppExt, CharacterCatalogFragment,
    };

    app.register_character_catalog_fragment(
        CharacterCatalogFragment::from_ron(
            crate::AMBITION_CONTENT_PROVIDER,
            Some(PLAYABLE_ROSTER[0]),
            CHARACTER_CATALOG_RON,
        )
        .expect("Ambition character catalog should be valid"),
    );
}

/// A curated cast of characters the player can start as. The character-select
/// surface cycles through these; every id is a `character_catalog.ron` row with
/// a renderable sheet. Deliberately hand-picked and small (not "every NPC") so
/// it reads as an intentional playable roster — narrow + specific over wide +
/// generic. Extend by adding a catalog id here.
pub const PLAYABLE_ROSTER: &[&str] = &[
    "player",                     // player robot (protagonist)
    "goblin",                     // melee striker
    "npc_pirate_admiral",         // pistol + cutlass
    "perfect_cellular_automaton", // the PCA (Fable extension target)
    "stochastic_parrot",          // the parrot
    "sandbag",                    // the training dummy, playable for laughs
];

/// The next id in [`PLAYABLE_ROSTER`] after `current`, wrapping. Unknown ids
/// (not in the roster) resolve to the first entry, so a stale selection always
/// re-enters the cast cleanly.
pub fn next_playable(current: &str) -> &'static str {
    let idx = PLAYABLE_ROSTER.iter().position(|id| *id == current);
    match idx {
        Some(i) => PLAYABLE_ROSTER[(i + 1) % PLAYABLE_ROSTER.len()],
        None => PLAYABLE_ROSTER[0],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_actors::avatar::StartingCharacter;

    #[test]
    fn every_playable_roster_id_is_a_real_catalog_character() {
        // The curated cast is a hand-maintained list; without this pin it rots
        // silently when a catalog id is renamed/removed, and the launch flag
        // would spawn a colored rectangle. Every id must resolve a catalog row.
        let catalog = load_catalog();
        for id in PLAYABLE_ROSTER {
            assert!(
                catalog.display_name(id).is_some(),
                "PLAYABLE_ROSTER id '{id}' has no character_catalog.ron row — the \
                 curated cast rotted; fix the roster or the catalog",
            );
        }
    }

    #[test]
    fn playable_roster_starts_with_protagonist_and_has_no_dupes() {
        // The protagonist ("player") is the roster's head. Content owns this
        // provider-relative default through the App-local registry.
        assert_eq!(PLAYABLE_ROSTER[0], "player");
        let mut app = bevy::prelude::App::new();
        register(&mut app);
        assert_eq!(
            app.world()
                .resource::<ambition_characters::actor::character_catalog::CharacterCatalogDefaults>()
                .for_provider(crate::AMBITION_CONTENT_PROVIDER),
            Some(PLAYABLE_ROSTER[0]),
            "the App-local fragment publishes the provider default"
        );
        assert_eq!(
            StartingCharacter::default().effective_id(PLAYABLE_ROSTER[0]),
            PLAYABLE_ROSTER[0]
        );
        for (i, a) in PLAYABLE_ROSTER.iter().enumerate() {
            for b in &PLAYABLE_ROSTER[i + 1..] {
                assert_ne!(a, b, "duplicate id in PLAYABLE_ROSTER: {a}");
            }
        }
    }

    #[test]
    fn next_playable_wraps_and_recovers_unknown() {
        assert_eq!(next_playable("player"), PLAYABLE_ROSTER[1]);
        assert_eq!(
            next_playable(PLAYABLE_ROSTER[PLAYABLE_ROSTER.len() - 1]),
            "player"
        );
        // Unknown / stale ids re-enter at the top of the cast.
        assert_eq!(next_playable("not_a_real_id"), PLAYABLE_ROSTER[0]);
    }
}
