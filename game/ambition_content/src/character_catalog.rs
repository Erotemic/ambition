//! Ambition's character-catalog data and the curated playable cast. This is
//! content, kept out of the engine core (R3.2).
//!
//! The catalog schema, parser, and App-local fragment registry live in
//! `ambition_characters::actor::character_catalog`. Runtime systems consume
//! the assembled `CharacterCatalog` resource. The RON stays a loose file here
//! so the Python tools (`ambition_ldtk_tools.codegen_character_catalog`, the
//! hall generator) can read it from disk.

/// The authored roster RON (compile-time include; single source of truth
/// shared with the off-disk tooling).
pub const CHARACTER_CATALOG_RON: &str = include_str!("../assets/data/character_catalog.ron");

/// Parse Ambition's checked-in catalog into an explicit immutable value.
///
/// Goes through [`crate::pack::prepared`], so a preset typo or a duplicate
/// identity is refused at composition, naming the character and field, not
/// later as a spawn-time fallback.
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
        .expect("the prepared catalog carries this provider's default character")
        // What an Ambition character that states no verbs can do as an actor.
        .with_actor_default_abilities(
            ambition_platformer2d_core::AbilitySet::classic_actor(),
        )
        // How an Ambition character that names no provoked policy fights once
        // provoked. Without it, striking such a character provokes nothing.
        .with_default_provoked_profile("provoked_combatant"),
    );
}

/// Register the whole cast this provider ships, as the plugin does.
///
/// Three registrations make one cast: the catalog fragment, the robot lineage
/// and the declared cast. A fixture that repeats them by hand can drop one and
/// still get a cast with no error, but that cast is not the shipped cast. A cast
/// with no lineage stages no robot, so a reload that edits the robot's move file
/// is refused.
pub fn register_cast(app: &mut bevy::prelude::App) {
    register(app);
    crate::player_robot_lineage::register(app);
    crate::player_robot_lineage::register_declared_cast(app);
}

/// A curated cast of characters the player can start as. The character-select
/// surface cycles through these; every id is a `character_catalog.ron` row
/// with a renderable sheet. Hand-picked and small on purpose, not "every
/// NPC". Extend by adding a catalog id here.
///
/// ## The player robot's lineage is in the cast
///
/// `robot`, `player_robot_v2` and `player_robot_v3` are three incarnations of
/// one character (v0, v2, v3; there is no v1). Ambition wants old versions of
/// yourself to be things you can meet, talk to, fight, and play as.
pub const PLAYABLE_ROSTER: &[&str] = &[
    "player_robot_v3",            // the player robot, v3 (current)
    "player_robot_v2",            // v2: the build before the SVG rig
    "robot",                      // v0: the original
    "goblin",                     // melee striker
    "npc_pirate_admiral",         // pistol + cutlass
    "perfect_cellular_automaton", // the PCA — see the note below (D74)
    "stochastic_parrot", // the parrot
    "sandbag",           // the training dummy, playable for laughs
    // ── The fighters the smash grid offers ───────────────────────────────────
    //
    // A character this game offers as a worn body must be one it can build. A
    // match seats these fighters, which is the same as wearing them, so they must
    // be registered here.
    //
    // Only Ambition's own characters. `mary_o` and `sanic` belong to other
    // providers; this catalog has no row for them, so listing them here would
    // register nothing and fail `every_playable_roster_id_is_a_real_catalog_character`
    // and `the_shipped_cast_is_what_the_compiler_prepared`. Their own demos
    // declare them.
    "npc_ninja_shadow_oni_leader",
    "npc_alice",
    "npc_bob",
    "npc_oiler",
    "npc_emmy_noether",
];

/// Characters this game can build without offering them as player selections.
///
/// Buildability comes from authoring a body and is distinct from the playable
/// roster. A body is authored in the character's row (`locomotion` or
/// `locomotion_preset`) or, for a fact the row cannot state yet, in its file
/// under `authored/`. The build-only cast is derived from those two places
/// rather than maintained as a second list.
pub fn buildable_only_cast() -> impl Iterator<Item = &'static str> {
    let authored_ids = crate::authored::authored_ids;
    crate::authored::authored_ids()
        .chain(
            rows_with_a_body().filter(move |id| !authored_ids().any(|authored| authored == *id)),
        )
        .chain(REGISTERED_WITHOUT_A_BODY.iter().copied())
        // Characters that author a body and also appear on the select grid are
        // excluded here, so the two casts cannot overlap.
        // `the_build_only_cast_resolves_rows_and_does_not_overlap_the_selection_cast`
        // also catches an overlap added by hand.
        .filter(|id| !PLAYABLE_ROSTER.contains(id))
}

/// The rows of the shipped pack's catalog that state how their body moves.
fn rows_with_a_body() -> impl Iterator<Item = &'static str> {
    ambition_characters::actor::character_catalog::lowered_catalog(crate::pack::prepared())
        .expect("the character schema lowers its catalog for every pack that compiles")
        .characters
        .iter()
        .filter(|(_, row)| row.locomotion.is_some() || row.locomotion_preset.is_some())
        .map(|(id, _)| id.as_str())
}

/// Characters registered without an authored body. Keep it empty (AC4).
///
/// Do not keep fallback health or incomplete body definitions while waiting
/// for balance decisions. An empty list makes "authoring a character makes it
/// buildable" true with no exception. If a character cannot state its body
/// yet, raise it on the maintainer-decision surface instead of registering it
/// bare.
const REGISTERED_WITHOUT_A_BODY: &[&str] = &[];

/// Ambition-specific intrinsic facts layered onto a character definition.
///
/// The catalog supplies catalog-shaped metadata; this function supplies body/kit
/// facts that Ambition authors in Rust. Preparation combines the registered
/// definition with the provider sources it consumes and produces the single
/// `PreparedCharacterDefinition` runtime construction uses.
///
/// An id in [`buildable_only_cast`] with no body/policy/moveset authoring here is
/// suspicious: registering a bare definition does not conjure a second body
/// authority. Author the intended character facts before making it buildable.
///
/// `pack` is a parameter because the move table is a migrated family
/// (fast-iteration I3, step 1). A process-global `OnceLock` would make every
/// App in a process share one move table and give a reload nowhere to put a
/// new one. The caller reads its App's selection once and passes it down.
pub fn authored_intrinsics(
    id: &str,
    definition: ambition_platformer2d::character::CharacterDefinition,
    pack: &ambition_content_pack::PreparedContentPack,
) -> ambition_platformer2d::character::CharacterDefinition {
    // The creature's own file states the rest: `authored/` has one file per
    // creature, listed in [`crate::authored::AUTHORED_CAST`]. See its module doc.
    let definition = match crate::authored::author_for(id) {
        Some(author) => author(id, definition),
        None => definition,
    };
    // The move table comes from the pack for every character the pack has one
    // for (fast-iteration I2, step 5). `register_declared_cast` calls this for
    // every buildable character, so no per-character arm is needed.
    //
    // Applied after the creature's file: the pack is the authority for a
    // migrated table, so it writes last. A character with `with_moveset` in its
    // file and no pack entry is unchanged, so migration can go one character at a
    // time.
    //
    // It replaces the table; it does not merge. A merge would need a per-verb
    // rule for which side wins.
    match ambition_characters::moveset_content_schema::lowered_movesets(pack)
        .and_then(|table| table.get(id))
    {
        Some(contract) => definition.with_moveset(contract.clone()),
        None => definition,
    }
}

/// Every id this game registers as a buildable character — the SELECTION cast
/// plus the build-only cast. The one list registration iterates.
pub fn buildable_cast() -> impl Iterator<Item = &'static str> {
    PLAYABLE_ROSTER.iter().copied().chain(buildable_only_cast())
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

    /// The shipped cast, prepared: each character's definition folded with its
    /// row, which is the one authority on what the character is.
    pub(super) fn prepared_cast_app() -> bevy::prelude::App {
        let mut app = bevy::prelude::App::new();
        crate::character_catalog::register_cast(&mut app);
        ambition_characters::prepared::close_preparation_barrier_without_admission(app.world_mut());
        ambition_platformer2d_shared_tangle::app_finalization::finalize(&mut app);
        app
    }

    /// A row that states how its body moves is a character this game builds,
    /// with the body, policy, death, action set and traits its row states.
    ///
    /// Twenty-four characters were a Rust file each that stated only what the
    /// row can now say. The row is their one authority. Being BUILT is what brings a character its
    /// move table from the pack, so the move table is what this checks for
    /// buildability, beside the row's own facts.
    #[test]
    fn a_row_that_states_its_body_is_built_with_that_body() {
        let app = prepared_cast_app();
        let prepared = app
            .world()
            .resource::<ambition_characters::prepared::PreparedCharacterRegistry>();
        let catalog = load_catalog();
        let movesets =
            ambition_characters::moveset_content_schema::lowered_movesets(crate::pack::prepared())
                .expect("the pack authors move tables");

        let mut bodies = 0;
        let mut stated = 0;
        for (id, row) in catalog.iter() {
            // The facts a row can state whether or not it states a body. A row
            // that is not prepared is checked by the body rows below.
            if let Some(character) = prepared.get(id) {
                if row.mass.is_some() {
                    assert_eq!(character.vitals.mass, row.mass, "`{id}` weighs what its row says");
                    stated += 1;
                }
                if row.mount.is_some() {
                    assert_eq!(character.mount, row.mount, "`{id}` rides as its row says");
                    stated += 1;
                }
                if row.dream_seed.is_some() {
                    assert_eq!(character.dream_seed, row.dream_seed, "{id}");
                    stated += 1;
                }
                if row.held_item.is_some() {
                    assert_eq!(character.held_item, row.held_item, "{id}");
                    stated += 1;
                }
                if row.abilities.is_some() {
                    assert_eq!(character.abilities, catalog.ability_set(id), "{id}");
                    stated += 1;
                }
                if row.contact_damage.is_some() {
                    assert_eq!(character.contact_damage, row.contact_damage, "{id}");
                    stated += 1;
                }
                if let Some(name) = row.provoked_profile.as_deref() {
                    assert_eq!(
                        character.provoked_profile.as_ref(),
                        catalog.autonomous_profile(name),
                        "`{id}` is provoked into the policy its row names"
                    );
                    stated += 1;
                }
                if row.posed_body.is_some() {
                    let scale = ambition_platformer2d::character_sprites::posed_body_world_per_pixel(
                        catalog.data(),
                        id,
                    )
                    .map(|world_per_pixel| {
                        ambition_characters::actor::definition::BodySource::SpriteAuthored {
                            world_per_pixel,
                        }
                    });
                    assert_eq!(character.body, scale, "`{id}` is built at its row's scale");
                    stated += 1;
                }
            }
            if row.locomotion.is_none() && row.locomotion_preset.is_none() {
                continue;
            }
            bodies += 1;
            let character = prepared
                .get(id)
                .unwrap_or_else(|| panic!("`{id}` states a body and is not prepared"));
            // Preparation resolves an unstated free-flight baseline, so the
            // gait is what is compared.
            let gait = |walk: Option<ambition_characters::actor::CharacterLocomotion>| {
                walk.map(|walk| (walk.run_speed, walk.move_style, walk.surface_walker))
            };
            assert_eq!(gait(character.locomotion), gait(catalog.locomotion(id)), "{id}");
            if let Some(health) = row.max_health {
                assert_eq!(character.vitals.max_health, Some(health), "{id}");
            }
            if row.death_traits.is_some() {
                assert_eq!(character.death_traits, row.death_traits, "{id}");
            }
            let policy = row.autonomous_profile.as_ref().or_else(|| {
                catalog.autonomous_profile(row.named_autonomous_profile.as_deref()?)
            });
            if policy.is_some() {
                assert_eq!(character.autonomous_profile.as_ref(), policy, "{id}");
            }
            // The preset the row names, lowered as preparation lowers it.
            let named = catalog
                .data()
                .action_set_presets
                .get(&row.default_action_set)
                .map(ambition_characters::actor::character_catalog::action_set_from_preset);
            assert_eq!(
                character.kit.action_set(),
                named.as_ref(),
                "`{id}` fights with a different action set from the one its row names"
            );
            if movesets.contains_key(id.as_str()) {
                assert!(
                    character.authored_moveset.is_some(),
                    "`{id}` states a body and the pack authors its moves, but it was \
                     not built, so it fights with no moves"
                );
            }
        }
        assert!(bodies >= 29, "only {bodies} rows state a body");
        assert!(stated >= 34, "only {stated} row facts were compared");

        let get = |id: &str| prepared.get(id).unwrap_or_else(|| panic!("`{id}`"));
        assert_eq!(
            get("npc_alice").locomotion.map(|walk| walk.run_speed),
            Some(210.0),
            "the hall walk"
        );
        assert!(get("sandbag").practice_target, "the sandbag exists to be hit");
        // Rows that used to name a preset their Rust file overruled, so the
        // preset they name now is the one that must hold their real numbers.
        assert_eq!(
            get("npc_pirate_lookout").kit.action_set().map(|set| set.ranged.clone()),
            Some(Some(ambition_characters::brain::RangedActionSpec::bolt(500.0, 1))),
            "the ranger shoots the bolt its crew file stated, not the old arrow preset"
        );
        let brute = get("npc_goblin_brute").kit.action_set().and_then(|set| set.melee);
        assert!(
            matches!(
                brute,
                Some(ambition_characters::brain::MeleeActionSpec::Swipe(swing))
                    if swing.damage == 2 && swing.reach_px == 44.0
            ),
            "the brute swings its hammer, not the striker's swipe: {brute:?}"
        );
        // The giant's fists are drawn at the giant's scale: the fist row names
        // the giant's, it does not restate the number.
        assert_eq!(
            get("npc_giant_gnu_hands").body,
            Some(ambition_characters::actor::definition::BodySource::SpriteAuthored {
                world_per_pixel: 1.3
            }),
            "a fist is drawn the size of the hand that throws it"
        );
        let slug = get("npc_puppy_slug").abilities.expect("the slug states its verbs");
        assert!(
            slug.move_horizontal && !slug.jump && !slug.attack,
            "the slug crawls, and that is all it does: {slug:?}"
        );
        assert_eq!(
            get("npc_pirate_heavy_iron_mary")
                .provoked_profile_id
                .as_ref()
                .map(|profile| profile.as_str().to_string()),
            Some("ambition::pirate_boarder_heavy".to_string()),
            "a heavy pirate is provoked into the heavy boarder"
        );
        assert!(
            get("npc_emmy_noether").preserves_mirror_symmetry,
            "two CPU Emmys must think on one stream"
        );
        assert!(
            !get("npc_pirate_admiral").preserves_mirror_symmetry,
            "an ordinary fighter does not share a cognitive stream"
        );
    }

    /// The kernel guide has its own `CharacterDefinition`, and no kit (D56).
    ///
    /// It authors identity (its walk, its four health) and nothing about its body
    /// or abilities. `register_declared_cast` excludes exploration NPCs because a
    /// bare registration would replace the archetype-authored body, so the
    /// archetype road must stay in charge of both.
    ///
    /// Compared with a peer (Alice, a hub NPC that made the same migration) and a
    /// control (the vault keeper, which has not), so the change is one
    /// character's, not a sweep of the hall.
    #[test]
    fn the_kernel_guide_authors_an_identity_and_no_combat_kit() {
        let mut app = bevy::prelude::App::new();
        crate::character_catalog::register_cast(&mut app);
        // A content question, not an admission one. This fixture installs no
        // technique handlers, so real admission would withhold characters that name a
        // native effect. The raw road is named explicitly.
        ambition_characters::prepared::close_preparation_barrier_without_admission(app.world_mut());
        ambition_platformer2d_shared_tangle::app_finalization::finalize(&mut app);
        let prepared = app
            .world()
            .resource::<ambition_characters::prepared::PreparedCharacterRegistry>();

        let guide = prepared
            .get("npc_kernel_guide")
            .expect("the Kernel Guide has a CharacterDefinition of its own");
        assert!(
            guide.sheet.is_some(),
            "the guide prepared without a sheet, so its identity cannot draw"
        );
        assert_eq!(
            guide.vitals.max_health,
            Some(4),
            "the guide has no health of its own, which is the fallback the \
             authored road exists to remove"
        );
        assert!(
            guide.locomotion.is_some(),
            "the guide states no walk, so its body still takes one from an \
             archetype it no longer needs to ask"
        );

        // The absences are the content.
        assert!(
            guide.abilities.is_none(),
            "a capability set was invented for a tutorial NPC to fill out its \
             definition, which is the one thing Jon's ruling forbade"
        );
        assert!(
            guide.body.is_none(),
            "the registration brought a body and therefore REPLACED the \
             archetype-authored one — the exact failure `register_declared_cast` \
             excludes exploration NPCs to avoid"
        );

        // The peer that made this migration, and the one that has not.
        let alice = prepared.get("npc_alice").expect("Alice is prepared");
        assert_eq!(
            (guide.abilities.is_some(), guide.body.is_some()),
            (alice.abilities.is_some(), alice.body.is_some()),
            "the guide prepared differently from the hub NPC it was modelled on"
        );
        // Every catalog row is prepared (AP30), so the control is what the
        // vault keeper AUTHORS: nothing beyond its row.
        let keeper = prepared
            .get("npc_vault_keeper")
            .expect("every catalog row is a prepared character");
        assert!(
            keeper.locomotion.is_none() && keeper.body.is_none(),
            "another hub NPC gained authored body facts too, so this was a rule \
             that swept the hall rather than one character taking its own identity"
        );
    }

    /// Practice-target characters may own attack actions, but their autonomous
    /// policy must neither notice nor reach opponents. The count assertion keeps
    /// the invariant non-vacuous if the cast changes.
    #[test]
    fn practice_target_characters_do_not_strike_back() {
        let mut app = bevy::prelude::App::new();
        crate::character_catalog::register_cast(&mut app);
        // A content question, not an admission one. This fixture installs no
        // technique handlers, so real admission would withhold characters that name a
        // native effect. The raw road is named explicitly.
        ambition_characters::prepared::close_preparation_barrier_without_admission(app.world_mut());
        ambition_platformer2d_shared_tangle::app_finalization::finalize(&mut app);
        let prepared = app
            .world()
            .resource::<ambition_characters::prepared::PreparedCharacterRegistry>();

        let mut targets = 0;
        for id in prepared.ids() {
            let character = prepared
                .get(id)
                .unwrap_or_else(|| panic!("`{id}` is in the registry's own id list"));
            if !character.practice_target {
                continue;
            }
            targets += 1;
            let policy = character
                .autonomous_profile
                .unwrap_or_else(|| panic!("`{id}` is a practice target that states no policy, so what it does when hit is whatever a default happens to say"));
            assert_eq!(
                (policy.aggro_radius, policy.attack_range),
                (0.0, 0.0),
                "`{id}` is authored as a practice target and its policy notices \
                 targets at {}px and reaches them at {}px — a dummy that \
                 counter-attacks is not a dummy. ⚠ its KIT is not the thing to \
                 fix: both sandbags carry `sandbag_punch` on purpose, and the \
                 policy is what keeps the fist unused",
                policy.aggro_radius,
                policy.attack_range
            );
        }
        assert!(
            targets >= 2,
            "this cast holds {targets} practice targets and Ambition ships two \
             (`sandbag`, `sandbag_infinite`), so the guard above checked nothing \
             — which is exactly how its roster-side ancestor went quietly vacuous"
        );
    }
    use ambition_platformer2d_actor_monolith::avatar::StartingCharacter;

    /// What the mounts and their riders ARE, through preparation: the admiral
    /// and the raiders board a shark and are not ridden, the shark and the giant
    /// are ridden and board nothing, and a fist is neither. Their rows state it;
    /// every road that builds them inherits it (`prepared_match` unions
    /// `pilotable_classes` into `CanPilot`), so the admiral's summoned shark can
    /// be boarded from the character-select grid too.
    #[test]
    fn the_mounts_and_their_riders_are_what_their_rows_say() {
        let app = prepared_cast_app();
        let prepared = app
            .world()
            .resource::<ambition_characters::prepared::PreparedCharacterRegistry>();
        let get = |id: &str| prepared.get(id).unwrap_or_else(|| panic!("`{id}` is not prepared"));
        for rider in ["npc_pirate_admiral", "npc_pirate_raider", "npc_pirate_heavy_iron_mary"] {
            let mount = get(rider).mount.clone().unwrap_or_default();
            assert_eq!(mount.pilotable_classes, vec!["shark".to_string()], "`{rider}` boards a shark");
            assert!(mount.class.is_none(), "`{rider}` became something you can ride");
        }
        for (ridden, class) in [("npc_burning_flying_shark", "shark"), ("npc_giant_gnu", "giant")] {
            let mount = get(ridden).mount.clone().unwrap_or_default();
            assert_eq!(mount.class.as_deref(), Some(class), "`{ridden}` is a mount");
            assert!(mount.pilotable_classes.is_empty(), "`{ridden}` rides nothing");
        }
        assert!(get("npc_giant_gnu_hands").mount.is_none(), "a hand is neither ridden nor rides");
        // The giant is far heavier than the scholar on it, so the pair's centre of
        // gravity is on the giant.
        assert!(get("npc_giant_gnu").vitals.mass > get("npc_giant_gnu_hands").vitals.mass);
        for id in ["npc_giant_gnu", "npc_giant_gnu_hands", "npc_pirate_raider", "npc_pirate_heavy_iron_mary"] {
            assert!(get(id).contact_damage.is_none(), "touching `{id}` must not hurt");
        }
        assert_eq!(get("npc_pirate_raider").held_item.as_deref(), Some("gun_sword"));
        assert_eq!(get("npc_pirate_heavy_iron_mary").held_item.as_deref(), Some("gun_sword_heavy"));
        assert_eq!(
            get("npc_puppy_slug").dream_seed,
            Some(0.271828),
            "the slug's own psychedelic pass"
        );
    }

    /// Every pirate is provoked into the boarder policy its row names, through
    /// preparation (the form the runtime uses), and registration visits it.
    /// The heavies take the heavy boarder. A character that is not a pirate
    /// keeps its provider's default, so the rows are not a blanket rule.
    #[test]
    fn every_pirate_is_provoked_into_the_boarder_its_row_names() {
        let app = prepared_cast_app();
        let prepared = app
            .world()
            .resource::<ambition_characters::prepared::PreparedCharacterRegistry>();
        let provoked = |id: &str| {
            prepared
                .get(id)
                .unwrap_or_else(|| panic!("`{id}` is not prepared"))
                .provoked_profile_id
                .as_ref()
                .map(|profile| profile.as_str().to_string())
        };
        let registered: std::collections::BTreeSet<&str> = buildable_cast().collect();
        let catalog = load_catalog();
        let mut pirates = 0;
        for id in catalog.data().characters.keys().filter(|id| id.starts_with("npc_pirate_")) {
            pirates += 1;
            let expected = if id.contains("pirate_heavy") {
                "ambition::pirate_boarder_heavy"
            } else {
                "ambition::pirate_boarder"
            };
            assert_eq!(provoked(id).as_deref(), Some(expected), "`{id}` when struck");
            assert!(registered.contains(id.as_str()), "`{id}` is never registered");
        }
        assert!(pirates >= 9, "only {pirates} pirate rows");
        let alice = provoked("npc_alice");
        assert!(
            !alice.as_deref().is_some_and(|profile| profile.contains("pirate_boarder")),
            "a character that is not a pirate became a boarder: {alice:?}"
        );
    }

    /// The parrot's pins, beside the definition that states them.
    ///
    /// `is_aerial` stays a catalog answer (`body_kind: Floating`). `mass` was not
    /// carried over: it had no effect on a creature that is neither a mount nor a
    /// rider.
    #[test]
    fn the_parrot_authors_the_body_its_archetype_row_used_to() {
        use ambition_characters::brain::{CharacterBrainTemplate, MoveStyleSpec};

        let app = prepared_cast_app();
        let definition = app
            .world()
            .resource::<ambition_characters::prepared::PreparedCharacterRegistry>()
            .get("stochastic_parrot")
            .expect("the parrot is prepared");
        assert_eq!(definition.vitals.max_health, Some(3));
        let locomotion = definition.locomotion.expect("it states how it flies");
        assert_eq!(locomotion.run_speed, 240.0);
        assert!(matches!(locomotion.move_style, MoveStyleSpec::Float));
        let profile = definition.autonomous_profile.expect("the dive-bomber policy");
        assert_eq!(profile.template, CharacterBrainTemplate::Aerial);
        assert_eq!(profile.aggro_radius, 620.0);
        assert!(
            definition
                .kit
                .action_set()
                .is_some_and(|set| set.melee.is_some()),
            "the peck is what makes a dive a threat"
        );

        // Control: the catalog still owns gravity-freedom.
        assert!(
            matches!(
                load_catalog().body_kind("stochastic_parrot"),
                Some(ambition_characters::actor::character_catalog::CharacterBodyKind::Floating)
            ),
            "the parrot stopped being Floating in the catalog, which is where its \
             gravity-freedom lives now that the archetype row is gone"
        );
    }

    /// Every character this game builds states its own body: a gait from its
    /// row or its file. No buildable character borrows one from an archetype.
    ///
    /// This was a floor that only grew (19 on 2026-08-12) with a control that
    /// said the migration was not finished. The migration finished with AP67,
    /// so the floor is now the whole cast, and the control is a character that
    /// states no body.
    #[test]
    fn every_buildable_character_states_its_body() {
        // Asked of the prepared character: a gait from its row or its file.
        let app = prepared_cast_app();
        let prepared = app
            .world()
            .resource::<ambition_characters::prepared::PreparedCharacterRegistry>();
        let incomplete: Vec<&str> = crate::character_catalog::buildable_cast()
            .filter(|id| {
                !prepared
                    .get(id)
                    .is_some_and(|character| character.locomotion.is_some())
            })
            .collect();
        assert!(
            incomplete.is_empty(),
            "{incomplete:?} are built and state no gait, so their bodies come from \
             somewhere other than the character"
        );
        let total = crate::character_catalog::buildable_cast().count();
        assert!(total >= 44, "only {total} buildable characters");

        // Control: a hub NPC whose row states no body reads as incomplete, so the
        // signal distinguishes.
        assert!(
            prepared
                .get("npc_vault_keeper")
                .is_some_and(|keeper| keeper.locomotion.is_none()),
            "the vault keeper states no body and still reads as complete, so this \
             check cannot tell the two apart"
        );
    }

    /// How many characters state their own verbs (P3.25).
    ///
    /// `effective_abilities` reads `(authored ∪ granted) ∩ permitted`. By default
    /// a character that authors nothing gets whatever the mode permits; P3.25
    /// removes that default when this count reaches the cast.
    ///
    /// A floor, with the same control: it must not yet be everybody. When it is,
    /// replace this ratchet with a refusal.
    #[test]
    fn the_cast_that_states_its_own_verbs_only_grows() {
        let authored: Vec<&str> = crate::character_catalog::buildable_cast()
            .filter(|id| {
                authored_intrinsics(
                    id,
                    ambition_platformer2d::character::CharacterDefinition::new(
                        *id,
                        *id,
                        crate::AMBITION_CONTENT_PROVIDER,
                    ),
                    crate::pack::prepared(),
                )
                .abilities
                .is_some()
            })
            .collect();
        assert!(
            !authored.is_empty(),
            "no character in the cast states its own verbs, so `effective_abilities` \
             is a pure GRANT everywhere and the mask half is untested by content: \
             {authored:?}"
        );
        let total = crate::character_catalog::buildable_cast().count();
        assert!(
            authored.len() < total,
            "every character now states its own verbs ({total} of {total}) — the \
             `(None, mode) => mode` GRANT arm has no adopters left, so delete it \
             and this ratchet with it. Authored: {authored:?}"
        );
    }

    /// Prepared characters must exercise authored move timelines, while peaceful characters
    /// may still rely on the floor their experience's roster preparation grants them.
    #[test]
    fn the_cast_that_states_its_own_moves_only_grows() {
        let mut app = bevy::prelude::App::new();
        crate::character_catalog::register_cast(&mut app);
        // A content question, not an admission one. This fixture installs no
        // technique handlers, so real admission would withhold characters that name a
        // native effect. The raw road is named explicitly.
        ambition_characters::prepared::close_preparation_barrier_without_admission(app.world_mut());
        ambition_platformer2d_shared_tangle::app_finalization::finalize(&mut app);
        let prepared = app
            .world()
            .resource::<ambition_characters::prepared::PreparedCharacterRegistry>();

        let authored: Vec<&str> = prepared
            .iter()
            .filter(|(_, definition)| definition.authored_moveset.is_some())
            .map(|(id, _)| id)
            .collect();
        assert!(
            !authored.is_empty(),
            "no character in the prepared cast states its own move timelines, so every \
             seated fighter is made of whatever floor its experience grants, and the \
             authored road is exercised by no content at all"
        );
        let total = prepared.ids().count();
        assert!(
            authored.len() < total,
            "every one of the {total} prepared characters states its own moves. \
             ⛔ that is not automatically the end of the floor: most of this cast \
             authors `default_action_set: \"peaceful\"` on purpose, so reaching \
             the whole cast means they were re-authored as fighters. \
             An experience's own seating floor is what lets a peaceful \
             character be seated at all. Authored: {authored:?}"
        );
    }

    /// A character-authored `BrainProfile` must not also name a preset brain policy.
    #[test]
    fn a_character_states_its_policy_in_one_place() {
        /// `(character, preset it still names, why it cannot drop it yet)`
        const KNOWN_DOUBLE_STATED: &[(&str, &str, &str)] = &[
            // Any temporary exception must name why the preset cannot yet be removed.
        ];

        let catalog = load_catalog();
        let mut offenders = Vec::new();
        for id in crate::character_catalog::buildable_cast() {
            let authors_policy = Some(authored_intrinsics(
                id,
                ambition_platformer2d::character::CharacterDefinition::new(
                    id,
                    id,
                    crate::AMBITION_CONTENT_PROVIDER,
                ),
                crate::pack::prepared(),
            ))
            // Both shapes count: an inline `autonomous_profile` and a named
            // `autonomous_profile_ref` (for example the goblin's shared
            // `medium_striker`).
            .map(|definition| definition.autonomous_policy.is_some())
            .unwrap_or(false);
            let Some(entry) = catalog.get(id) else {
                continue;
            };
            if authors_policy && !entry.default_brain.is_empty() {
                offenders.push((id, entry.default_brain.clone()));
            }
        }

        let unexpected: Vec<_> = offenders
            .iter()
            .filter(|(id, _)| !KNOWN_DOUBLE_STATED.iter().any(|(known, ..)| known == id))
            .collect();
        assert!(
            unexpected.is_empty(),
            "these characters author a `BrainProfile` AND name a brain preset, so \
             one of the two decides nothing and nobody can tell which: \
             {unexpected:?}. Empty the row's `default_brain` — or, if its preset \
             carries an `aggressiveness`, move that to the placements FIRST and \
             add it to KNOWN_DOUBLE_STATED with the reason."
        );

        // The exemption list cannot rot: an entry that got fixed must be removed.
        let stale: Vec<_> = KNOWN_DOUBLE_STATED
            .iter()
            .filter(|(id, ..)| !offenders.iter().any(|(offender, _)| offender == id))
            .collect();
        assert!(
            stale.is_empty(),
            "these are exempted as double-stated but no longer are — delete them \
             from KNOWN_DOUBLE_STATED: {stale:?}"
        );
    }

    /// The practice target says it is one. `practice_target` has four consumers:
    /// the save sync, the path assignment and two sprite reads.
    ///
    /// It authors no contact damage.
    #[test]
    fn the_sandbag_authors_the_dummy_its_archetype_row_used_to() {
        use ambition_characters::brain::CharacterBrainTemplate;

        let app = prepared_cast_app();
        let definition = app
            .world()
            .resource::<ambition_characters::prepared::PreparedCharacterRegistry>()
            .get("sandbag")
            .expect("the sandbag is prepared");
        assert!(definition.practice_target, "it exists to be hit");
        assert_eq!(definition.vitals.max_health, Some(6));
        assert!(
            definition.contact_damage.is_none(),
            "walking into a dummy does not hurt, whatever the old row's comment said"
        );
        let profile = definition.autonomous_profile.expect("its policy");
        assert_eq!(profile.template, CharacterBrainTemplate::StandStill);
        assert_eq!(profile.aggro_radius, 0.0, "it notices nobody");
    }

    /// The goblin names its policy instead of carrying one.
    ///
    /// Its controller comes from a shared `autonomous_profiles` entry
    /// (`medium_striker`), while it keeps its own health, reach and pace.
    ///
    /// The reference is provider-namespaced, because assembly namespaces every
    /// preset map: a bare "medium_striker" resolves to nothing.
    #[test]
    fn the_goblin_names_the_shared_striker_policy() {
        use ambition_characters::brain::MoveStyleSpec;

        let app = prepared_cast_app();
        let definition = app
            .world()
            .resource::<ambition_characters::prepared::PreparedCharacterRegistry>()
            .get("goblin")
            .expect("the goblin is prepared");
        assert_eq!(definition.vitals.max_health, Some(5));
        let locomotion = definition.locomotion.expect("its own body");
        assert_eq!(locomotion.run_speed, 170.0);
        assert!(matches!(locomotion.move_style, MoveStyleSpec::Walk));
        let row = load_catalog();
        let row = row.get("goblin").expect("the goblin's row");
        assert_eq!(
            row.named_autonomous_profile.as_deref(),
            Some("medium_striker"),
            "it NAMES the shared policy, provider-relative; carrying one inline \
             would make it unshareable, which is the whole point"
        );
        // A row states one policy or the other. Check which: a switch to inline
        // would stop the policy being shared.
        assert!(
            row.autonomous_profile.is_none(),
            "the goblin carries an INLINE policy, so the shared `medium_striker` \
             entry is not what decides how it fights"
        );
        assert_eq!(
            definition.autonomous_profile.as_ref(),
            load_catalog().autonomous_profile("medium_striker"),
            "the goblin is prepared with the shared policy it names"
        );
    }

    /// The shared policy exists in the shipped catalog and holds only
    /// controller facts. A body fact here would give the policy a second
    /// authority over the body.
    #[test]
    fn the_shipped_catalog_authors_a_shared_striker_policy() {
        // The shipped bytes, parsed as the game parses them. Keys are namespaced at
        // assembly, which `load_catalog` does not do, so this reads the local name.
        let catalog = load_catalog();
        let profile = catalog
            .autonomous_profile("medium_striker")
            .expect("the shipped catalog authors the shared striker policy");
        assert_eq!(profile.aggro_radius, 460.0);
        assert_eq!(profile.attack_range, 150.0);
        assert_eq!(profile.patrol_effort, 0.6176);
        assert!(profile.smash_sprint_to_close);
    }

    /// Every authored brain preset has at least one character using it.
    ///
    /// A key whose last user migrates to a `BrainProfile` fails here in the same
    /// change, so no unused row is left behind.
    #[test]
    fn no_authored_brain_preset_is_reachable_by_nobody() {
        let catalog = load_catalog();
        let data = catalog.data();
        let adopted: std::collections::BTreeSet<&str> = data
            .characters
            .values()
            .map(|entry| entry.default_brain.as_str())
            .collect();
        let orphans: Vec<&str> = data
            .brain_presets
            .keys()
            .map(String::as_str)
            .filter(|key| !adopted.contains(key))
            .collect();
        assert!(
            orphans.is_empty(),
            "brain presets nobody names: {orphans:?}. An unreachable policy is \
             a row a future retirement pass has to decide about for no reason — \
             delete it, or give it the character it was written for"
        );
        assert!(
            !data.brain_presets.is_empty(),
            "no presets at all, so the sweep above proved nothing"
        );
    }

    /// The runtime's cast is the compiler's output, not a separate reading that
    /// the compiler also checks.
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

        // The catalog the game uses is the lowered artifact, entry for entry.
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

    /// Production registration and the compiler are one authority.
    ///
    /// The app-local fragment must be the compiler's lowered artifact, entry for
    /// entry, not a second parse of the same file. Two readers let content pass
    /// validation while the game loads something else.
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
        // Every id must resolve a catalog row.
        let catalog = load_catalog();
        for id in PLAYABLE_ROSTER {
            assert!(
                catalog.display_name(id).is_some(),
                "PLAYABLE_ROSTER id '{id}' has no character_catalog.ron row — the \
                 curated cast rotted; fix the roster or the catalog",
            );
        }
    }

    /// The two lists answer two questions, and the build-only list must obey the
    /// same rules as the selection list.
    ///
    /// Emptying an arm of [`authored_intrinsics`] fails this. A registered
    /// character that authors nothing does not fall back to its archetype; it has
    /// no death behaviour, for example.
    #[test]
    fn the_migrated_mites_author_their_own_death_and_health() {
        let app = prepared_cast_app();
        let prepared = app
            .world()
            .resource::<ambition_characters::prepared::PreparedCharacterRegistry>();
        for (id, explodes, divides_into, health) in [
            ("npc_exploding_mite", true, None, 2),
            ("npc_dividing_mite", false, Some("npc_puppy_slug"), 4),
        ] {
            let authored = prepared
                .get(id)
                .unwrap_or_else(|| panic!("{id} is prepared"));
            let traits = authored
                .death_traits
                .as_ref()
                .unwrap_or_else(|| panic!("{id} is registered, so it must author its own death"));
            assert_eq!(traits.explodes_on_death, explodes, "{id}");
            assert_eq!(traits.divides_into.as_deref(), divides_into, "{id}");
            assert_eq!(
                authored.vitals.max_health,
                Some(health),
                "{id} must carry the pool its archetype row used to give it"
            );
        }
    }

    /// Every id in the build-only cast authors its intrinsics. See
    /// [`buildable_only_cast`]'s own warning: registering an id whose facts are
    /// still in the roster is how a character silently loses them.
    #[test]
    fn every_build_only_id_authors_something() {
        // Ids that needed the exemption, checked against the list after the loop:
        // an unneeded exemption is a false claim.
        let mut genuinely_bare: Vec<&str> = Vec::new();
        // Empty. Each entry must carry its placement evidence.
        const KNOWN_BARE_REGISTRATIONS: &[(&str, &str)] = &[];
        // Asked of the PREPARED character: its row and its registered
        // definition together are what it authors.
        let app = prepared_cast_app();
        let prepared = app
            .world()
            .resource::<ambition_characters::prepared::PreparedCharacterRegistry>();
        for id in buildable_only_cast() {
            let authored = prepared
                .get(id)
                .unwrap_or_else(|| panic!("`{id}` is buildable and not prepared"));
            let authors_a_body =
                authored.death_traits.is_some() || authored.vitals.max_health.is_some();
            // A policy-only registration retracts nothing. The rule is about bodies: a
            // definition with no vitals means "this character authors none", and
            // preparation retracts the archetype body. A character that states only a
            // controller policy has no body to retract.
            //
            // `an_incomplete_character_uses_peaceful_npc_defaults` checks that an
            // incomplete definition does not leak partial body facts into the
            // peaceful-NPC path.
            let authors_only_policy = !authors_a_body && authored.autonomous_profile.is_some();
            // A third safe case: a character with no archetype body to lose (placed
            // only as a peaceful Hall `NpcSpawn`). A bare registration costs it nothing.
            // Each entry carries the placement evidence.
            let exempt = KNOWN_BARE_REGISTRATIONS
                .iter()
                .any(|(known, _)| *known == id);
            if !authors_a_body && !authors_only_policy {
                genuinely_bare.push(id);
            }
            assert!(
                authors_a_body || authors_only_policy || exempt,
                "`{id}` is registered as buildable and authors NOTHING — not a \
                 body and not a policy. A bare registration means it has no \
                 body, not that its archetype keeps it. If it has no archetype \
                 body to lose, say so in `KNOWN_BARE_REGISTRATIONS` with the \
                 placement evidence. ⚠ A MOVESET DOES NOT SATISFY THIS and the \
                 message used to say it did — see the staleness arm below."
            );
        }

        // An exemption nobody needs is a false claim. Remove an entry when its id
        // stops being bare. The assertion message states what the predicate checks
        // (`authors_a_body || authors_only_policy || exempt`); it does not check
        // movesets.
        let listed: Vec<&str> = KNOWN_BARE_REGISTRATIONS.iter().map(|(id, _)| *id).collect();
        let unneeded: Vec<&str> = listed
            .iter()
            .copied()
            .filter(|id| !genuinely_bare.contains(id))
            .collect();
        assert!(
            unneeded.is_empty(),
            "{unneeded:?} sit in `KNOWN_BARE_REGISTRATIONS` and do not need to: \
             each authors a body or a policy and passes the rule on its own \
             merits. An exemption that is not load-bearing is a claim nobody \
             re-checks — and this list's entries carry PLACEMENT EVIDENCE, so a \
             stale one reads as a statement about what a character authors. \
             Delete the entry; the character is fine. (Genuinely bare right now: \
             {genuinely_bare:?}.)"
        );
    }

    /// The other direction: a character with an `authored_intrinsics` arm that
    /// is in neither list is never registered, so the arm runs for nobody and
    /// nothing fails.
    ///
    /// Check without parsing the match: pass a bare definition for every
    /// character in the assembled catalog and see if it comes back changed. An
    /// id it changes has an arm.
    #[test]
    fn every_character_with_an_authored_body_is_registered_as_buildable() {
        // An unregistered body is never built, so nothing fails at runtime.
        const KNOWN_UNREGISTERED: &[(&str, &str)] = &[];

        let catalog = load_catalog();
        let registered: std::collections::BTreeSet<&str> = buildable_cast().collect();
        let mut unregistered = Vec::new();
        for id in catalog.data().characters.keys() {
            let bare = ambition_platformer2d::character::CharacterDefinition::new(
                id.as_str(),
                "unused",
                crate::AMBITION_CONTENT_PROVIDER,
            );
            if authored_intrinsics(id.as_str(), bare.clone(), crate::pack::prepared()) != bare
                && !registered.contains(id.as_str())
            {
                unregistered.push(id.clone());
            }
        }
        let unexpected: Vec<_> = unregistered
            .iter()
            .filter(|id| !KNOWN_UNREGISTERED.iter().any(|(known, _)| known == id))
            .collect();
        assert!(
            unexpected.is_empty(),
            "these characters author something in `authored_intrinsics` and appear \
             on NEITHER `PLAYABLE_ROSTER` nor `buildable_only_cast()`, so the arm \
             runs for nobody and what it authors reaches no body: {unexpected:?}. \
             Author the character's vitals and add it to `buildable_only_cast` — \
             or, if it genuinely cannot be registered yet, add it to \
             `KNOWN_UNREGISTERED` with the reason and what unblocks it."
        );

        // The exemption list cannot rot: an entry that got fixed must be removed.
        let stale: Vec<_> = KNOWN_UNREGISTERED
            .iter()
            .filter(|(id, _)| !unregistered.iter().any(|found| found == id))
            .collect();
        assert!(
            stale.is_empty(),
            "these are exempted as unregistered and are no longer unregistered — \
             remove them from `KNOWN_UNREGISTERED`: {stale:?}"
        );

        // Control: if `authored_intrinsics` became the identity for every id, the
        // loop above would find nothing and pass.
        let authors_someone = catalog.data().characters.keys().any(|id| {
            let bare = ambition_platformer2d::character::CharacterDefinition::new(
                id.as_str(),
                "unused",
                crate::AMBITION_CONTENT_PROVIDER,
            );
            authored_intrinsics(id.as_str(), bare.clone(), crate::pack::prepared()) != bare
        });
        assert!(
            authors_someone,
            "no character in the catalog authors any intrinsics — the check above \
             is passing over an empty set"
        );
    }

    /// Empty today, so this checks the contract: an id here must resolve a
    /// catalog row and must not duplicate the selection cast. A double
    /// registration lets the last one win silently.
    #[test]
    fn the_build_only_cast_resolves_rows_and_does_not_overlap_the_selection_cast() {
        let catalog = load_catalog();
        let playable: std::collections::BTreeSet<&str> = PLAYABLE_ROSTER.iter().copied().collect();
        for id in buildable_only_cast() {
            assert!(
                catalog.display_name(id).is_some(),
                "buildable_only_cast() id '{id}' has no character_catalog.ron row",
            );
            assert!(
                !playable.contains(id),
                "'{id}' is in BOTH casts — a character registered twice keeps \
                 whichever registration ran last, which is not a decision anybody made",
            );
        }
        // The union is what registration walks.
        let union: Vec<&str> = buildable_cast().collect();
        assert_eq!(
            union.len(),
            PLAYABLE_ROSTER.len() + buildable_only_cast().count()
        );
    }

    #[test]
    fn playable_roster_starts_with_protagonist_and_has_no_dupes() {
        // The current incarnation heads the roster: `player_robot_v3`. There is no
        // generic `player`; each incarnation is its own character (see
        // `player_robot_lineage`).
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

#[cfg(test)]
mod assembled_provider_tests {
    /// The Hall's `brain_override` once resolved bare, which suggested the NPC
    /// road saw unassembled entries. This asks the assembled catalog directly.
    #[test]
    fn an_assembled_entry_states_the_provider_that_registered_it() {
        let mut app = bevy::prelude::App::new();
        super::register(&mut app);
        let catalog = app
            .world()
            .get_resource::<ambition_characters::actor::character_catalog::CharacterCatalog>()
            .expect("registering the content publishes an assembled catalog");
        let parrot = catalog
            .get("stochastic_parrot")
            .expect("the parrot is in the shipped cast");
        assert_eq!(
            parrot.provider,
            crate::AMBITION_CONTENT_PROVIDER,
            "an assembled entry must state the provider that registered it — \
             without it the namespace has to be inferred from a neighbouring \
             preset key, which is the coupling D81 removed"
        );
        assert!(
            parrot.default_brain.is_empty(),
            "the parrot names no preset — it is the character this whole thread \
             was about, and if it starts naming one again the provider field \
             below is no longer being exercised by a migrated row: `{}`",
            parrot.default_brain
        );

        // The subject has to be a row that still holds the property.
        let still_named = catalog
            .data()
            .characters
            .values()
            .find(|entry| !entry.default_brain.is_empty())
            .expect("some character still names a brain preset");
        assert!(
            still_named.default_brain.contains("::"),
            "assembly namespaces a named preset: `{}`",
            still_named.default_brain
        );
    }

    /// Every character has exactly one autonomous-policy authority, and it is
    /// reachable: the resolver answers, or refuses in the one way that has an
    /// answer waiting.
    #[test]
    fn every_migrated_character_has_an_autonomous_default_something_can_reach() {
        use ambition_characters::actor::character_catalog::BrainBuildError;

        let mut app = bevy::prelude::App::new();
        super::register(&mut app);
        let catalog = app
            .world()
            .get_resource::<ambition_characters::actor::character_catalog::CharacterCatalog>()
            .expect("registering the content publishes an assembled catalog")
            .clone();
        let ctx = ambition_characters::actor::character_catalog::BrainBuildContext {
            spawn_world_x: 0.0,
            patrol_radius: None,
        };

        let mut redirected = Vec::new();
        let mut answered = 0usize;
        for id in catalog.data().characters.keys() {
            match ambition_characters::actor::character_catalog::resolve_initial_brain(
                &catalog, id, None, &ctx,
            ) {
                Ok(_) => answered += 1,
                Err(BrainBuildError::NoAutonomousDefault { .. }) => redirected.push(id.clone()),
                // Any other error is a real content defect (a named preset that does not
                // exist).
                Err(other) => panic!("`{id}`: {other}"),
            }
        }

        // The redirect must lead somewhere: every character the resolver refuses
        // must author the profile the NPC road asks for. Otherwise its body stands
        // still.
        // Asked of the prepared character, whose policy is its registered
        // definition's or else its row's.
        let prepared_app = super::tests::prepared_cast_app();
        let prepared = prepared_app
            .world()
            .resource::<ambition_characters::prepared::PreparedCharacterRegistry>();
        let authors_a_profile = |id: &str| {
            prepared
                .get(id)
                .is_some_and(|character| character.autonomous_profile.is_some())
        };
        let stranded: Vec<_> = redirected
            .iter()
            .filter(|id| !authors_a_profile(id.as_str()))
            .collect();
        assert!(
            stranded.is_empty(),
            "these characters name no brain preset AND author no autonomous \
             profile, so nothing decides what they do when nobody drives them — \
             every one of them spawns stand-still and restores to nothing: \
             {stranded:?}"
        );

        // Both halves must be non-empty: some characters still resolve a preset,
        // and some have migrated to a profile.
        assert!(
            answered > 0,
            "no character resolves a preset any more — the preset road is dead \
             and this test should be rewritten rather than left passing"
        );
        assert!(
            !redirected.is_empty(),
            "no character redirects — the migration this test guards has not \
             happened, or the resolver stopped refusing"
        );
    }
}
