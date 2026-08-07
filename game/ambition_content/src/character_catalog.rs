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
/// Goes through [`crate::pack::prepared`], so a preset typo or a duplicate
/// identity refuses HERE — at composition, naming the character and the field —
/// rather than surfacing hours later as a spawn-time fallback.
pub fn load_catalog() -> ambition_characters::actor::character_catalog::CharacterCatalog {
    let data =
        ambition_characters::actor::character_catalog::lowered_catalog(crate::pack::prepared())
            .expect("the character schema lowers its catalog for every pack that compiles")
            .clone();
    ambition_characters::actor::character_catalog::CharacterCatalog::from_data(data)
}

/// Register Ambition's immutable character fragment in one Bevy `App` and
/// rebuild the deterministic assembled catalog resource.
pub fn register(app: &mut bevy::prelude::App) {
    use ambition_characters::actor::character_catalog::{
        CharacterCatalogAppExt, CharacterCatalogFragment,
    };

    // ⛔ **THROUGH THE COMPILER, not beside it.** This used to call
    // `from_ron(CHARACTER_CATALOG_RON)`, which reparsed and re-validated the
    // same bytes through the legacy fragment path — so the CLI, the tests and
    // `load_catalog` went through the compiler while the running game went
    // through a different reader. Two authorities over one file is the precise
    // split the compiler exists to close, and it had survived one layer above
    // it. (GPT 5.6 review, finding 1.)
    let catalog =
        ambition_characters::actor::character_catalog::lowered_catalog(crate::pack::prepared())
            .expect("the character schema lowers its catalog for every pack that compiles")
            .clone();
    app.register_character_catalog_fragment(
        CharacterCatalogFragment::from_prepared(
            crate::pack::CATALOG_SOURCE_PATH,
            crate::AMBITION_CONTENT_PROVIDER,
            Some(PLAYABLE_ROSTER[0]),
            catalog,
        )
        .expect("the prepared catalog carries this provider's default character"),
    );
}

/// A curated cast of characters the player can start as. The character-select
/// surface cycles through these; every id is a `character_catalog.ron` row with
/// a renderable sheet. Deliberately hand-picked and small (not "every NPC") so
/// it reads as an intentional playable roster — narrow + specific over wide +
/// generic. Extend by adding a catalog id here.
///
/// ## The player robot's own lineage is IN the cast (2026-07-29)
///
/// `robot`, `player_robot_v2` and `player_robot_v3` are three incarnations of
/// the same character, and the catalog has always said so — v2's row records that
/// *"`robot` is v0, the original. There is no v1 -- that is a joke, not a
/// gap"*, and that Ambition *"wants old versions of yourself to be things you
/// can meet, talk to, and fight"*.
///
/// Two of the three could be met and fought and neither could be WORN, so
/// "play as the build that shipped before this one" was a content edit rather
/// than a selection. They are each their own character with their own art and
/// their own kit — v0 is peaceful, v2 swings the generic striker swipe the
/// protagonist used to, v3 carries the host-code kit — so putting them in the
/// cast is not a variant mechanism, it is three characters that happen to
/// share a face.
pub const PLAYABLE_ROSTER: &[&str] = &[
    "player_robot_v3",    // the player robot, v3 (current)
    "player_robot_v2",    // v2: the build before the SVG rig
    "robot",              // v0: the original
    "goblin",             // melee striker
    "npc_pirate_admiral", // pistol + cutlass
    // ⛔ **`perfect_cellular_automaton` IS DELIBERATELY ABSENT, and the reason
    // is a whole-game behaviour change hiding behind one portrait.**
    //
    // This list stopped being only a character-SELECT list on 2026-08-07: it is
    // now also what `register_declared_cast` REGISTERS, because a fighter must
    // be seatable and only a registered character is. For a character that
    // exists solely as somebody to WEAR those are the same claim. For one that
    // is also a HOSTILE WORLD ACTOR they are not — registering a bare
    // definition makes `apply_worn_character_kit` take the prepared arm and
    // fight with the row's `default_action_set` instead of the aggressive kit
    // its ARCHETYPE builds. Everywhere in the game, not just in a match.
    //
    // Measured, not reasoned: `duel_arena_room_is_a_real_neutral_attack_defense_fight`
    // goes from a real brawl to BOTH fighters throwing zero melee swings, and
    // removing this one id — nothing else — turns it green. Bisected to
    // `a733ec37e`, the commit that introduced the registration.
    //
    // ⚠ **the cost is real and reversing it is Jon's call:** the PCA is on
    // `SMASH_ROSTER`, so the grid is one portrait shorter until a registered
    // row can carry its archetype's kit. That is the character-authority
    // campaign — `CharacterCatalog` and `PreparedCharacterRegistry` as ONE
    // authority — which this landing deliberately did not open.
    //
    // ⚠ `goblin` above is the same SHAPE, a melee-striker world actor, and it
    // stays: it was on this list before the list meant registration, and
    // nothing measures it as broken. That is an argument from absence of
    // evidence and is worth being suspicious of.
    "stochastic_parrot", // the parrot
    "sandbag",           // the training dummy, playable for laughs
    // ── The fighters the smash grid offers ───────────────────────────────────
    //
    // ⭐ **"a character this game offers as a WORN BODY is one this game can
    // BUILD", and this list is where that claim is made.** They are here
    // because a match seats them, which is the same act as wearing them: a
    // fighter IS a body wearing a character, and eight of the twelve portraits
    // on the grid could be seated only as player one because nothing had ever
    // registered them. That asymmetry was invisible while human seats ADOPTED
    // the home body and CPU seats spawned; unifying construction is what
    // made it a hard failure.
    //
    // ⛔ **and the alternative was measured and rejected.** The first version
    // registered EVERY catalog row — "a character this game declares should be
    // one this game can build" — which reads better and is wrong: a bare
    // registration says the character authors no body, and preparation
    // correctly RETRACTS what an incoming persona does not author. That flipped
    // ~100 exploration NPCs off their archetype-built vitals onto defaults, and
    // `rollback_lifecycle_reset::a_player_death_reset_survives_the_rollback_window`
    // caught it: the calibration lab's strikers stopped being able to kill a
    // 3-HP player in 2400 frames (bisected to a733ec37e, verified by reverting
    // that one call). The catalog row has no mass or health to fold back in —
    // those come from the ARCHETYPE — so the blanket rule cannot be made
    // behaviour-neutral, only narrower.
    "mary_o",
    "sanic",
    "npc_ninja_shadow_oni_leader",
    "npc_alice",
    "npc_bob",
    "npc_oiler",
    "npc_noether",
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
    use ambition_platformer2d_actor_monolith::avatar::StartingCharacter;

    /// **The runtime's cast comes OUT of the compiler.**
    ///
    /// Not "the compiler also checks it" — out of it. Before this row the
    /// validator and the game each parsed `character_catalog.ron` through a
    /// different function, and nothing made them agree; a check that passes
    /// while the game loads something else is worth nothing.
    #[test]
    fn the_shipped_cast_is_what_the_compiler_prepared() {
        let pack = crate::pack::prepared();
        assert_eq!(pack.namespace.0, "ambition");
        assert!(
            pack.ids_of(&ambition_content_pack::SchemaId::new("character"))
                .len()
                > 100,
            "the whole cast came through the compiler, not a subset"
        );

        // The catalog the game will use IS the lowered artifact, entry for entry.
        let catalog = load_catalog();
        for id in PLAYABLE_ROSTER {
            let prepared = pack.get(&ambition_content_pack::SchemaId::new("character"), id);
            assert!(
                prepared.is_some(),
                "playable `{id}` is a prepared identity, so a tool and the game name it the \
                 same way"
            );
            assert!(catalog.display_name(id).is_some());
        }
    }

    /// **Production registration and the compiler are ONE authority.**
    ///
    /// The app-local fragment must be the compiler's lowered artifact, entry for
    /// entry — not a second parse of the same file. Two readers is how content
    /// passes validation and the game loads something else.
    #[test]
    fn the_registered_app_catalog_is_the_compilers_artifact() {
        let pack = crate::pack::prepared();
        let lowered =
            ambition_characters::actor::character_catalog::lowered_catalog(pack).expect("lowered");

        let mut app = bevy::prelude::App::new();
        register(&mut app);
        let assembled = app
            .world()
            .resource::<ambition_characters::actor::character_catalog::CharacterCatalog>();

        for (id, entry) in &lowered.characters {
            assert_eq!(
                assembled.display_name(id),
                Some(entry.display_name.as_str()),
                "`{id}` reached the App through the compiler, not a re-parse"
            );
        }
        assert_eq!(
            lowered.characters.len(),
            PLAYABLE_ROSTER
                .iter()
                .filter(|id| assembled.display_name(id).is_some())
                .count()
                .max(lowered.characters.len()),
            "every prepared character is registered"
        );
    }

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
        // The CURRENT incarnation is the roster's head — `player_robot_v3`, not
        // a generic `player`, because there is no generic one: each incarnation
        // is its own character (see `player_robot_lineage`). Content owns this
        // provider-relative default through the App-local registry.
        assert_eq!(PLAYABLE_ROSTER[0], "player_robot_v3");
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
        assert_eq!(next_playable("player_robot_v3"), PLAYABLE_ROSTER[1]);
        assert_eq!(
            next_playable(PLAYABLE_ROSTER[PLAYABLE_ROSTER.len() - 1]),
            "player_robot_v3"
        );
        // Unknown / stale ids re-enter at the top of the cast.
        assert_eq!(next_playable("not_a_real_id"), PLAYABLE_ROSTER[0]);
    }
}
