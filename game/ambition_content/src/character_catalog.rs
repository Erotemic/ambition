//! Ambition's character-catalog DATA + the curated playable cast —
//! CONTENT, evicted from the engine core (R3.2, violations #3 and #10).
//!
//! The catalog schema, parser, and App-local fragment registry live in
//! `ambition_characters::actor::character_catalog`. Runtime systems consume the
//! assembled `CharacterCatalog` resource. The legacy
//! `ambition_actors::character_roster` cache remains temporarily for pure helper
//! call sites that have not yet received explicit catalog access. The RON stays a loose file here so the Python tools
//! (`ambition_ldtk_tools.codegen_character_catalog`, the hall generator)
//! keep reading it off disk.

/// The authored roster RON (compile-time include; single source of truth
/// shared with the off-disk tooling).
pub const CHARACTER_CATALOG_RON: &str = include_str!("../assets/data/character_catalog.ron");

/// Feed the temporary process-global compatibility seam used by remaining pure
/// lookup call sites. New provider and player-session paths use [`register`].
pub fn install() {
    ambition_actors::character_roster::install_character_catalog(CHARACTER_CATALOG_RON);
    // Content owns which row is the default the home box wears with no override
    // (C2): the engine names no character, so inject `PLAYABLE_ROSTER[0]` here.
    ambition_actors::character_roster::install_default_character_id(PLAYABLE_ROSTER[0]);
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
    install();
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
        install();
        for id in PLAYABLE_ROSTER {
            assert!(
                ambition_actors::character_roster::display_name_for_character_id(id).is_some(),
                "PLAYABLE_ROSTER id '{id}' has no character_catalog.ron row — the \
                 curated cast rotted; fix the roster or the catalog",
            );
        }
    }

    #[test]
    fn playable_roster_starts_with_protagonist_and_has_no_dupes() {
        // The protagonist ("player") is the roster's head. Content OWNS this
        // (C2): installing the catalog injects `PLAYABLE_ROSTER[0]` as the
        // engine's default character, and an unset `StartingCharacter` then
        // resolves to it.
        assert_eq!(PLAYABLE_ROSTER[0], "player");
        install();
        assert_eq!(
            ambition_actors::character_roster::default_character_id(),
            PLAYABLE_ROSTER[0],
            "content install injects the default character id"
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
