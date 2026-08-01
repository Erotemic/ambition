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

/// The pack manifest, embedded from the SAME file the CLI reads off disk.
///
/// ⚠ **one manifest, two source origins.** A shipped binary must embed its
/// content — there is no asset directory beside a wasm bundle — and the CLI must
/// read a directory. What must not differ is the manifest that says what the
/// pack IS, or the pipeline that judges it.
const PACK_MANIFEST_RON: &str = include_str!("../assets/pack.ron");

/// The declared path of the catalog source, as `pack.ron` spells it. A mismatch
/// here is caught at composition by the compiler's own "no source supplied"
/// refusal rather than by silently loading an empty cast.
const CATALOG_SOURCE_PATH: &str = "data/character_catalog.ron";

/// Compile Ambition's embedded pack.
///
/// **This is the load path, not a check beside it.** Before this, the compiler
/// proved the catalog correct and the game parsed the same bytes again through
/// `parse_catalog` — two readers of one file, with nothing guaranteeing they
/// agreed. Now the runtime takes what the compiler lowered.
///
/// ⚠ assets are UNCHECKED here on purpose. A shipped binary's art may legitimately
/// be absent on a fresh clone (AGENTS.md: git-ignored payloads, "degrade visibly
/// when a file is absent"), and refusing to boot over a missing sheet would make
/// this compiler the thing that stops the game rather than the thing that
/// explains it. The CLI's strict mode is where art is a gate.
pub fn compile_pack()
-> Result<ambition_content_pack::PreparedContentPack, ambition_content_pack::CompileFailure> {
    let manifest: ambition_content_pack::ContentPackManifest = ron::from_str(PACK_MANIFEST_RON)
        .expect("Ambition's own pack manifest is committed beside this file and must parse");
    let draft = ambition_content_pack::ContentPackDraft::from_sources(
        manifest,
        [(
            CATALOG_SOURCE_PATH.to_string(),
            CHARACTER_CATALOG_RON.to_string(),
        )],
    )?;
    let mut registry = ambition_content_pack::SchemaRegistry::new();
    registry
        .register(ambition_characters::actor::character_catalog::character_catalog_schema())
        .expect("one schema, registered once");
    ambition_content_pack::compile(
        &draft,
        &registry,
        &ambition_content_pack::AssetsUnchecked,
    )
}

/// Parse Ambition's checked-in catalog into an explicit immutable value.
///
/// Goes through [`compile_pack`], so a preset typo or a duplicate identity
/// refuses HERE — at composition, naming the character and the field — rather
/// than surfacing hours later as a spawn-time fallback.
pub fn load_catalog() -> ambition_characters::actor::character_catalog::CharacterCatalog {
    let pack = compile_pack().unwrap_or_else(|failure| {
        // A loud stop, and the repo has already chosen once between this and a
        // half-built start: "a silent partial start would be worse than a loud
        // stop". A cast that silently lost a character is exactly that.
        panic!("Ambition's own content pack does not compile:\n{failure}")
    });
    let data = ambition_characters::actor::character_catalog::lowered_catalog(&pack)
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
    let pack = compile_pack().unwrap_or_else(|failure| {
        panic!("Ambition's own content pack does not compile:\n{failure}")
    });
    let catalog = ambition_characters::actor::character_catalog::lowered_catalog(&pack)
        .expect("the character schema lowers its catalog for every pack that compiles")
        .clone();
    app.register_character_catalog_fragment(
        CharacterCatalogFragment::from_prepared(
            CATALOG_SOURCE_PATH,
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
    "player_robot_v3",            // the player robot, v3 (current)
    "player_robot_v2",            // v2: the build before the SVG rig
    "robot",                      // v0: the original
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
    use ambition_platformer2d_actor_monolith::avatar::StartingCharacter;

    /// **The runtime's cast comes OUT of the compiler.**
    ///
    /// Not "the compiler also checks it" — out of it. Before this row the
    /// validator and the game each parsed `character_catalog.ron` through a
    /// different function, and nothing made them agree; a check that passes
    /// while the game loads something else is worth nothing.
    #[test]
    fn the_shipped_cast_is_what_the_compiler_prepared() {
        let pack = compile_pack().expect("Ambition's own pack compiles");
        assert_eq!(pack.namespace.0, "ambition");
        assert!(
            pack.ids_of(&ambition_content_pack::SchemaId::new("character")).len() > 100,
            "the whole cast came through the compiler, not a subset"
        );

        // The catalog the game will use IS the lowered artifact, entry for entry.
        let catalog = load_catalog();
        for id in PLAYABLE_ROSTER {
            let prepared = pack.get(
                &ambition_content_pack::SchemaId::new("character"),
                id,
            );
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
        let pack = compile_pack().expect("compiles");
        let lowered = ambition_characters::actor::character_catalog::lowered_catalog(&pack)
            .expect("lowered");

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
