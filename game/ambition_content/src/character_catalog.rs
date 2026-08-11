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
    // ⛔ **`perfect_cellular_automaton` IS DELIBERATELY ABSENT — and the reason
    // is NOT what an earlier version of this comment claimed.**
    //
    // This list stopped being only a character-SELECT list on 2026-08-07: it is
    // now also what `register_declared_cast` REGISTERS, because a fighter must
    // be seatable and only a registered character is.
    //
    // ⚠ **the first explanation here was that registering a hostile world actor
    // replaces its archetype kit with the row's peaceful one. MEASURED AND
    // FALSE.** The duel arena's two fighters carry byte-identical components
    // either way — `size`, `hp`, `melee=true`, `attack_range`, `aggro_radius`,
    // `sprite_character_id`, no `WornCharacter`, no `BrainBinding`. Registering
    // the PCA changes nothing about the PCA.
    //
    // What it changes is TIMING: one more registered character is one more sheet
    // demanded at load, and `duel_arena_room_is_a_real_neutral_attack_defense_fight`
    // starts measuring three frames after room load. With the extra sheet in
    // flight both fighters throw ZERO melee for a sixty-second bout; settling
    // 180 frames first turns that into a real fight (melee 4). ⭐ **that a fight
    // which starts before its sheets land never recovers is the real finding
    // here, and it is unexamined** — a combat geometry resolved from a missing
    // sheet appears to stick for the life of the body.
    //
    // So this line is a WORKAROUND holding a fragile instrument green, not a
    // statement about the PCA, and the cost is real: the PCA is on
    // `SMASH_ROSTER`, so the grid is one portrait shorter.
    //
    // ⇥ **MEASURED 2026-08-10 (queue D74). Registering it reds
    // `possession_end_to_end::attack_while_possessing_…`, and the difference is
    // not any of the four things guessed.** Same probe, both builds, at the end
    // of the attack window:
    //
    //     registered      gravity 1.0   on_ground FALSE   size 38.1x96.3   x=663
    //     not registered  gravity 1.0   on_ground true    size 38.1x96.3   x=1246
    //
    // ⇒ gravity is normal in BOTH, the collision size is IDENTICAL, and the body
    // is **580 px away** and airborne. So this is not aerial-ness, not a resize
    // under its own feet, and not a lost kit (it carries all seven attack
    // verbs): the possession sequence simply plays out somewhere else, and the
    // grounded swing never happens because the body is not grounded.
    //
    // ⛔ what moves it is still UNKNOWN, and four wrong mechanisms have been
    // written down for this already — do not add a fifth without output.
    //
    // ⇥ **LOCATED**: the per-step trail is identical through step 3 and parts at
    // step 4 on `vel.x` — baseline zeroes it on a 4-step cadence, the registered
    // body accumulates at −10.83/step, both falling. Same `hp = (60, 60)`, same
    // `brain = Player(PlayerSlot(0))`, same size, same gravity. ⇒ the fault is
    // UPSTREAM of combat: a movement or contact decision on a falling body.
    // Deterministic at step 4, so bisect the movement kernel — see queue D74.
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
    //
    // ⛔ **AMBITION'S OWN, and `mary_o`/`sanic` were here and should not have
    // been.** They are on the smash grid and they are other providers'
    // characters — no row for either exists in this game's catalog, so
    // `register_declared_cast` skipped them silently (`catalog.get(id)` →
    // `None` → `continue`) and they registered nothing. Their own demos declare
    // them, which is why the grid carries them either way. What the two entries
    // DID do was break this crate's own `every_playable_roster_id_is_a_real_
    // catalog_character` and `the_shipped_cast_is_what_the_compiler_prepared`,
    // both of which say a curated id must resolve a row here — correctly.
    // ⚠ measured, not assumed: removing them changes no registration.
    "npc_ninja_shadow_oni_leader",
    "npc_alice",
    "npc_bob",
    "npc_oiler",
    "npc_noether",
];

/// **Characters this game can BUILD but does not OFFER as a selection.**
///
/// ⭐ **the split the character-template campaign requires** (D73 phase 2, Jon
/// 2026-08-10): *"`PLAYABLE_ROSTER` may remain a UI/content decision about which
/// characters appear in a selection screen. It must NOT define which characters
/// the engine is capable of constructing."* Until this existed the two questions
/// were one list, so making a character buildable also put a portrait on the
/// select grid — and a mite does not belong there.
///
/// Registration is `PLAYABLE_ROSTER ∪ this`. Empty today, so nothing changes
/// yet; it is the door phase 2's migration walks through, one character at a
/// time.
///
/// ⛔ **an id belongs here only once its intrinsic facts are on its DEFINITION.**
/// A bare registration says the character authors no body, preparation correctly
/// retracts what a persona does not author, and a character whose health, mass
/// and kit still live in `character_archetypes.ron` loses them — the measured
/// ~100-NPC regression recorded on [`PLAYABLE_ROSTER`]. Author first, register
/// second; that ordering is the whole reason this list is empty rather than
/// pre-filled with the obvious candidates.
pub const BUILDABLE_ONLY_CAST: &[&str] = &[
    // ⭐ **the first two characters migrated off `character_archetypes.ron`**
    // (D73 phase 2, group A). Their death traits and health pools are authored
    // on their DEFINITIONS by [`authored_intrinsics`] and deleted from their
    // archetype rows in the same change, so the two authorities never state the
    // same fact at once. Their sandbox placements name them explicitly.
    "npc_exploding_mite",
    "npc_dividing_mite",
];

/// **What a migrated character authors about its own body.**
///
/// The registration loop builds a bare definition from the catalog row — id,
/// display name, sheet — which is all an unmigrated character can say. This is
/// where a character that has taken its facts back from
/// `character_archetypes.ron` states them.
///
/// ⛔ **an id in [`BUILDABLE_ONLY_CAST`] with no arm here is the bug that list's
/// doc warns about**: a bare registration means "this character authors no
/// body", and anything its archetype used to give it is simply lost. Author
/// first, register second.
pub fn authored_intrinsics(
    id: &str,
    definition: ambition_platformer2d_actor_monolith::character_runtime::CharacterDefinition,
) -> ambition_platformer2d_actor_monolith::character_runtime::CharacterDefinition {
    use ambition_characters::actor::CharacterDeathTraits;

    use ambition_characters::actor::{CharacterLocomotion, ContactDamage};
    use ambition_characters::brain::{
        BrainProfile, CharacterBrainTemplate, MeleeActionSpec, MoveStyleSpec, SwipeSpec,
    };

    match id {
        // ⭐ **THE FIRST TWO CHARACTERS TO OWN THEIR WHOLE BODY.** Their
        // `character_archetypes.ron` rows are DELETED in the same change: what
        // used to be twenty lines of `exploding_mite` is these facts, split
        // across the three authorities that own them.
        //
        // ```text
        // body        health, run speed, gait, contact damage, the swipe
        // controller  the Smash policy: aggro 460, commit at 60, hit band 30
        // placement   respawn, which the LDtk spawn already carries
        // ```
        //
        // The sandbag kamikaze mite: two hit points and a corpse that detonates.
        "npc_exploding_mite" => {
            let mut definition = definition
                .with_death_traits(CharacterDeathTraits {
                    explodes_on_death: true,
                    ..Default::default()
                })
                .with_locomotion(CharacterLocomotion {
                    run_speed: 245.0,
                    move_style: MoveStyleSpec::Walk,
                    ..Default::default()
                })
                .with_contact_damage(ContactDamage {
                    strength: 0.60,
                    amount: 1,
                })
                .with_autonomous_profile(BrainProfile {
                    template: CharacterBrainTemplate::Smash,
                    aggro_radius: 460.0,
                    attack_range: 60.0,
                    smash_hit_band: 30.0,
                    ..Default::default()
                })
                .with_action_set(ambition_characters::brain::ActionSet {
                    melee: Some(MeleeActionSpec::Swipe(SwipeSpec {
                        windup_s: 0.22,
                        active_s: 0.08,
                        recover_s: 0.30,
                        damage: 1,
                        reach_px: 26.0,
                    })),
                    move_style: MoveStyleSpec::Walk,
                    ..Default::default()
                });
            definition.vitals.max_health = Some(2);
            definition
        }
        // The splitter: four hit points, slower and tankier, and it becomes two
        // on death.
        "npc_dividing_mite" => {
            let mut definition = definition
                .with_death_traits(CharacterDeathTraits {
                    divides_on_death: true,
                    ..Default::default()
                })
                .with_locomotion(CharacterLocomotion {
                    run_speed: 130.0,
                    move_style: MoveStyleSpec::Walk,
                    ..Default::default()
                })
                .with_contact_damage(ContactDamage {
                    strength: 0.70,
                    amount: 1,
                })
                .with_autonomous_profile(BrainProfile {
                    template: CharacterBrainTemplate::Smash,
                    aggro_radius: 380.0,
                    attack_range: 55.0,
                    smash_hit_band: 34.0,
                    ..Default::default()
                })
                .with_action_set(ambition_characters::brain::ActionSet {
                    melee: Some(MeleeActionSpec::Swipe(SwipeSpec {
                        windup_s: 0.30,
                        active_s: 0.10,
                        recover_s: 0.34,
                        damage: 1,
                        reach_px: 30.0,
                    })),
                    move_style: MoveStyleSpec::Walk,
                    ..Default::default()
                });
            definition.vitals.max_health = Some(4);
            definition
        }
        _ => definition,
    }
}

/// Every id this game registers as a buildable character — the SELECTION cast
/// plus the build-only cast. The one list registration iterates.
pub fn buildable_cast() -> impl Iterator<Item = &'static str> {
    PLAYABLE_ROSTER
        .iter()
        .chain(BUILDABLE_ONLY_CAST.iter())
        .copied()
}

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

    /// **The two lists answer two questions, and the build-only one has to obey
    /// the same rules as the selection one** (D73 phase 2).
    ///
    /// **The two characters that have taken their body back from the archetype
    /// roster author it here** — D73 phase 2, group A, the first migration.
    ///
    /// ⭐ this is where the coverage that used to live in the monolith's
    /// `archetype_capabilities_match_the_legacy_identity_checks` went. That test
    /// asserted `explodes_on_death` on `character_archetypes.ron`'s
    /// `exploding_mite` row; the row no longer says it, because the CHARACTER
    /// does. The fact did not lose its guard, it moved with the fact.
    ///
    /// ⛔ poison: empty an arm of [`authored_intrinsics`] and this reds. That
    /// matters more than it looks — a registered character that authors nothing
    /// does not fall back to its archetype, it simply has no death behaviour,
    /// and an exploding mite that stops exploding is invisible until someone
    /// stands next to one.
    #[test]
    fn the_migrated_mites_author_their_own_death_and_health() {
        for (id, explodes, divides, health) in [
            ("npc_exploding_mite", true, false, 2),
            ("npc_dividing_mite", false, true, 4),
        ] {
            let bare =
                ambition_platformer2d_actor_monolith::character_runtime::CharacterDefinition::new(
                    id,
                    "unused",
                    crate::AMBITION_CONTENT_PROVIDER,
                );
            let authored = authored_intrinsics(id, bare);
            let traits = authored
                .death_traits
                .as_ref()
                .unwrap_or_else(|| panic!("{id} is registered, so it must author its own death"));
            assert_eq!(traits.explodes_on_death, explodes, "{id}");
            assert_eq!(traits.divides_on_death, divides, "{id}");
            assert_eq!(
                authored.vitals.max_health,
                Some(health),
                "{id} must carry the pool its archetype row used to give it"
            );
        }
    }

    /// Every id in the build-only cast authors its intrinsics. See
    /// [`BUILDABLE_ONLY_CAST`]'s own warning: registering an id whose facts are
    /// still in the roster is how a character silently loses them.
    #[test]
    fn every_build_only_id_authors_something() {
        for id in BUILDABLE_ONLY_CAST {
            let bare =
                ambition_platformer2d_actor_monolith::character_runtime::CharacterDefinition::new(
                    *id,
                    "unused",
                    crate::AMBITION_CONTENT_PROVIDER,
                );
            let authored = authored_intrinsics(id, bare);
            assert!(
                authored.death_traits.is_some() || authored.vitals.max_health.is_some(),
                "`{id}` is registered as buildable but authors no body — a bare \
                 registration means it has none, not that its archetype keeps it"
            );
        }
    }

    /// ⚠ it is empty today, so this asserts the CONTRACT rather than any
    /// current content: an id here must resolve a catalog row, and must not
    /// duplicate the selection cast — registering a character twice is how a
    /// definition silently loses to whichever registration ran last.
    #[test]
    fn the_build_only_cast_resolves_rows_and_does_not_overlap_the_selection_cast() {
        let catalog = load_catalog();
        let playable: std::collections::BTreeSet<&str> = PLAYABLE_ROSTER.iter().copied().collect();
        for id in BUILDABLE_ONLY_CAST {
            assert!(
                catalog.display_name(id).is_some(),
                "BUILDABLE_ONLY_CAST id '{id}' has no character_catalog.ron row",
            );
            assert!(
                !playable.contains(id),
                "'{id}' is in BOTH casts — a character registered twice keeps \
                 whichever registration ran last, which is not a decision anybody made",
            );
        }
        // And the union is what registration actually walks, so a reader can
        // trust the two constants without reading `register_declared_cast`.
        let union: Vec<&str> = buildable_cast().collect();
        assert_eq!(
            union.len(),
            PLAYABLE_ROSTER.len() + BUILDABLE_ONLY_CAST.len()
        );
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
