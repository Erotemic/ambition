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
/// ## The player robot's own lineage is IN the cast
///
/// `robot`, `player_robot_v2` and `player_robot_v3` are three incarnations of
/// the same character, and the catalog has always said so — v2's row records that
/// *"`robot` is v0, the original. There is no v1 -- that is a joke, not a
/// gap"*, and that Ambition *"wants old versions of yourself to be things you
/// can meet, talk to, and fight"*.
///
/// Two of the three could be met and fought and neither could be WORN, so "play as the build that
/// shipped before this one" was a content edit rather than a selection.
pub const PLAYABLE_ROSTER: &[&str] = &[
    "player_robot_v3",            // the player robot, v3 (current)
    "player_robot_v2",            // v2: the build before the SVG rig
    "robot",                      // v0: the original
    "goblin",                     // melee striker
    "npc_pirate_admiral",         // pistol + cutlass
    "perfect_cellular_automaton", // the PCA — see the note below (D74)
    // All five were refuted, leaving ONE standing, the vaguest and the oldest: *"one more
    // registered character is one more sheet demanded at load"*, with a step-4 `vel.x`
    // divergence as its symptom.
    //
    // `CharacterLoadStates` reports `staged=3, ready=0` at every one of the first twelve steps
    // in BOTH builds, and the possession trail is identical to the last decimal. There is no
    // extra sheet, so there is no timing to differ.
    //
    // verified before landing, not inferred: `ambition_app` 337 + 179 + 1,
    // `ambition_content` 192 + 32, `ambition_demo_smash` 67, and the workspace
    // gate — all green with this line in.
    "stochastic_parrot", // the parrot
    "sandbag",           // the training dummy, playable for laughs
    // ── The fighters the smash grid offers ───────────────────────────────────
    //
    // "a character this game offers as a WORN BODY is one this game can
    // BUILD", and this list is where that claim is made. They are here
    // because a match seats them, which is the same act as wearing them: a
    // fighter IS a body wearing a character, and eight of the twelve portraits
    // on the grid could be seated only as player one because nothing had ever
    // registered them. That asymmetry was invisible while human seats ADOPTED
    // the home body and CPU seats spawned; unifying construction is what
    // made it a hard failure.
    //
    // The catalog row has no mass or health to fold back in — those come from the ARCHETYPE —
    // so the blanket rule cannot be made behaviour-neutral, only narrower.
    //
    // AMBITION'S OWN, and `mary_o`/`sanic` were here and should not have been. They are on
    // the smash grid and they are other providers' characters — no row for either exists in
    // this game's catalog, so `register_declared_cast` skipped them silently (`catalog.get(id)`
    // → `None` → `continue`) and they registered nothing. Their own demos declare them, which
    // is why the grid carries them either way. What the two entries DID do was break this
    // crate's own `every_playable_roster_id_is_a_real_ catalog_character` and
    // `the_shipped_cast_is_what_the_compiler_prepared`, both of which say a curated id must
    // resolve a row here — correctly.
    "npc_ninja_shadow_oni_leader",
    "npc_alice",
    "npc_bob",
    "npc_oiler",
    "npc_emmy_noether",
];

/// Characters this game can build without offering them as player selections.
///
/// Buildability comes from authored character registration and is distinct from
/// the playable roster. The build-only cast is derived from authored definitions
/// rather than maintained as a second list.
pub fn buildable_only_cast() -> impl Iterator<Item = &'static str> {
    crate::authored::authored_ids()
        .chain(REGISTERED_WITHOUT_A_BODY.iter().copied())
        // Five characters author a body AND appear on the select grid (the parrot, the goblin, the
        // admiral, the oni leader, the sandbag), so deriving the cast from the authoring surfaced
        // all five at once. Excluding here makes the rule structural;
        // `the_build_only_cast_resolves_rows_and_does_not_overlap_the_selection_cast` still fails
        // on an overlap reintroduced by hand below.
        .filter(|id| !PLAYABLE_ROSTER.contains(id))
}

/// EMPTY, and that is the point of it. (AC4)
///
/// Do not retain fallback health or incomplete body definitions because we are waiting for balance
/// decisions."* Carl Stargan sat here for the sibling reason, and the same handoff settled him.
/// Both now author bodies in `authored/`.
///
/// keep it empty. An entry here is a character whose body is somebody
/// else's to state, and the empty list is what makes "authoring a character makes
/// it buildable" true without an exception clause. If a future character genuinely
/// cannot state its body yet, the honest move is to say so on the
/// maintainer-decision surface rather than to register it bare.
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
pub fn authored_intrinsics(
    id: &str,
    definition: ambition_platformer2d_actor_monolith::character_runtime::CharacterDefinition,
) -> ambition_platformer2d_actor_monolith::character_runtime::CharacterDefinition {
    // this is a RULE rather than nine arms, because the thing it replaces
    // was a rule too — a worse one. `hostile_brain_id_for_actor` asks whether
    // an id, a display name or a dialogue node contains `"pirate"`, or one of
    // `"broadside bess"` / `"iron mary"` / `"salt annet"`, and hands the body a
    // whole archetype. Nine characters answer that matcher, and every one of them
    // has to state its own answer before the two rows it points at can die.
    //
    // the heavy/light split is the matcher's own: it tests `pirate_heavy`
    // FIRST, so the three named heavies take the brute policy and the rest take
    // the boarder. Reproducing that split here rather than re-deciding it keeps
    // the migration a migration.
    let definition = if id.starts_with("npc_pirate_") {
        definition.with_provoked_profile_named(if id.contains("pirate_heavy") {
            "pirate_boarder_heavy"
        } else {
            "pirate_boarder"
        })
    } else {
        definition
    };
    // AND THE CREATURE'S OWN FILE STATES THE REST.
    //
    //  `authored/` — one file per creature, beside its moveset, and ONE table
    // ([`crate::authored::AUTHORED_CAST`]) that the module list already forces
    // anybody to keep true. See its module doc.
    match crate::authored::author_for(id) {
        Some(author) => author(id, definition),
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

    /// ⭐⭐ THE KERNEL GUIDE HAS ITS OWN `CharacterDefinition`, AND NO KIT (D56).
    ///
    /// Jon, W8 playtest, closing the decision: *"Kernel Guide gets its own
    /// `CharacterDefinition`. Character identity is not sprite identity... Do
    /// not invent a combat kit or capabilities merely to fill the definition."*
    ///
    /// ⛔⛔ AND THE SECOND HALF IS THE ONE THIS FILE'S OWN HISTORY DEMANDS.
    /// `register_declared_cast` excludes exploration NPCs for a stated reason —
    /// *"a bare registration for an exploration NPC would incorrectly replace
    /// its archetype-authored body"* — so a registration that arrived carrying a
    /// body or an ability set would take facts away from the guide rather than
    /// give it any. It authors identity (its walk, its four health) and states
    /// NOTHING about what it is made of or what it can do, which is what leaves
    /// the archetype road in charge of both.
    ///
    /// ⭐ MEASURED AGAINST A PEER AND A CONTROL. Alice is a hub NPC that made
    /// this same migration, so the guide matching her is the claim; the vault
    /// keeper is a hub NPC that has NOT, so its absence is what proves the
    /// registration is one character's rather than a rule that swept the hall.
    #[test]
    fn the_kernel_guide_authors_an_identity_and_no_combat_kit() {
        let mut app = bevy::prelude::App::new();
        crate::character_catalog::register(&mut app);
        crate::player_robot_lineage::register_declared_cast(&mut app);
        ambition_platformer2d_shared_tangle::app_finalization::finalize(&mut app);
        let prepared = app
            .world()
            .resource::<ambition_platformer2d_actor_monolith::character_runtime::PreparedCharacterRegistry>();

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

        // ⛔ THE ABSENCES ARE THE CONTENT.
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

        // The peer that already made this migration, and the one that has not.
        let alice = prepared.get("npc_alice").expect("Alice is prepared");
        assert_eq!(
            (guide.abilities.is_some(), guide.body.is_some()),
            (alice.abilities.is_some(), alice.body.is_some()),
            "the guide prepared differently from the hub NPC it was modelled on"
        );
        assert!(
            prepared.get("npc_vault_keeper").is_none(),
            "another hub NPC gained a definition too, so this was a rule that \
             swept the hall rather than one character taking its own identity"
        );
    }

    /// Practice-target characters may own attack actions, but their autonomous
    /// policy must neither notice nor reach opponents. The count assertion keeps
    /// the invariant non-vacuous if the cast changes.
    #[test]
    fn practice_target_characters_do_not_strike_back() {
        let mut app = bevy::prelude::App::new();
        crate::character_catalog::register(&mut app);
        crate::player_robot_lineage::register_declared_cast(&mut app);
        ambition_platformer2d_shared_tangle::app_finalization::finalize(&mut app);
        let prepared = app
            .world()
            .resource::<ambition_platformer2d_actor_monolith::character_runtime::PreparedCharacterRegistry>();

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

    /// THE PUPPY SLUG'S PINS, beside the definition that states them.
    ///
    /// Moving the pins rather than deleting them is what keeps the migration honest — the facts
    /// did not stop mattering, they changed owner.
    ///
    /// and leaving them where they were would have been worse than losing
    /// them: `test_spec` answers an unknown key with the `combatant` fallback,
    /// so six assertions about a deleted row would have gone on passing about
    /// the wrong creature.
    #[test]
    fn the_puppy_slug_authors_the_body_its_archetype_row_used_to() {
        use ambition_characters::brain::{CharacterBrainTemplate, MoveStyleSpec};

        let definition = authored_intrinsics(
            "npc_puppy_slug",
            ambition_platformer2d_actor_monolith::character_runtime::CharacterDefinition::new(
                "npc_puppy_slug",
                "Puppy Slug",
                crate::AMBITION_CONTENT_PROVIDER,
            ),
        );
        assert_eq!(definition.vitals.max_health, Some(2));

        let locomotion = definition
            .locomotion
            .expect("the slug states how it moves, or it cannot be built as a character");
        assert_eq!(locomotion.run_speed, 80.0);
        assert!(matches!(locomotion.move_style, MoveStyleSpec::Slither));
        assert!(locomotion.surface_walker, "a crawlid that walks off walls");
        assert!(locomotion.cling_breaks_on_hit);

        let contact = definition
            .contact_damage
            .expect("its body hurts on touch — the only way it damages anything");
        assert_eq!(contact.amount, 1);

        let profile = definition
            .autonomous_profile
            .expect("ambient wildlife still has a policy: it wanders");
        assert_eq!(profile.template, CharacterBrainTemplate::Wanderer);
        assert_eq!(profile.aggro_radius, 0.0, "it notices nobody");

        assert_eq!(
            definition.dream_seed,
            Some(0.271828),
            "the slug-only psychedelic pass, which only an archetype row could \
             grant until this field existed"
        );
    }

    /// THE PARROT'S PINS, beside the definition that states them.
    ///
    /// Its `sky_parrot` row is deleted, and the two facts that did NOT come
    /// across are the interesting ones: `is_aerial` stays a CATALOG answer
    /// (`body_kind: Floating`, which the row was duplicating) and `mass` was
    /// inert on a creature that is neither a mount nor a rider.
    #[test]
    fn the_parrot_authors_the_body_its_archetype_row_used_to() {
        use ambition_characters::brain::{CharacterBrainTemplate, MoveStyleSpec};

        let definition = authored_intrinsics(
            "stochastic_parrot",
            ambition_platformer2d_actor_monolith::character_runtime::CharacterDefinition::new(
                "stochastic_parrot",
                "Stochastic Parrot",
                crate::AMBITION_CONTENT_PROVIDER,
            ),
        );
        assert_eq!(definition.vitals.max_health, Some(3));
        let locomotion = definition.locomotion.expect("it states how it flies");
        assert_eq!(locomotion.run_speed, 240.0);
        assert!(matches!(locomotion.move_style, MoveStyleSpec::Float));
        let profile = definition
            .autonomous_profile
            .expect("the dive-bomber policy");
        assert_eq!(profile.template, CharacterBrainTemplate::Aerial);
        assert_eq!(profile.aggro_radius, 620.0);
        assert!(
            definition
                .action_set
                .as_ref()
                .is_some_and(|set| set.melee.is_some()),
            "the peck is what makes a dive a threat"
        );

        // the control: the catalog still owns gravity-freedom, and this test
        // would be describing a different creature if that moved.
        assert!(
            matches!(
                load_catalog().body_kind("stochastic_parrot"),
                Some(ambition_characters::actor::character_catalog::CharacterBodyKind::Floating)
            ),
            "the parrot stopped being Floating in the catalog, which is where its \
             gravity-freedom lives now that the archetype row is gone"
        );
    }

    /// A migrated character has no archetype row left.
    ///
    /// Production readiness is measured through `body_blueprint()`, the same
    /// definition path used by spawning. The census may only increase so losing
    /// authored locomotion cannot masquerade as migration progress.
    #[test]
    fn the_body_complete_cast_only_grows() {
        let complete: Vec<&str> = crate::character_catalog::buildable_cast()
            .filter(|id| {
                let definition = authored_intrinsics(
                    id,
                    ambition_platformer2d_actor_monolith::character_runtime::CharacterDefinition::new(
                        *id,
                        *id,
                        crate::AMBITION_CONTENT_PROVIDER,
                    ),
                );
                // This census uses authored locomotion as the completeness signal.
                // If preparation gains additional requirements, update the census to match.
                definition.locomotion.is_some()
            })
            .collect();

        // a FLOOR, not a pin: every migration adds one, and a test that had to
        // be edited on the way past would be edited without being read.
        assert!(
            complete.len() >= 19,
            "only {} of Ambition's characters can build a body without an \
             archetype, and it was NINETEEN on 2026-08-12 — a migration does not \
             REMOVE completeness. Complete: {complete:?}",
            complete.len()
        );

        // and the control: the count must not be everybody, or `is_ok()` is
        // answering something other than "this character authored a body".
        let total = crate::character_catalog::buildable_cast().count();
        assert!(
            complete.len() < total,
            "every one of the {total} buildable characters reports body-complete, \
             which would mean `body_blueprint` has stopped distinguishing — the \
             migration is not finished, so this cannot be true yet"
        );
    }

    /// AND HOW MANY STATE THEIR OWN VERBS — P3.25's number, measured the same
    /// way and for the same reason.
    ///
    /// `effective_abilities` reads `(authored ∪ granted) ∩ permitted`, and its
    /// default is the bridge: a character that authors nothing is treated as
    /// having whatever the mode PERMITS. That default is the scaffold P3.25
    /// deletes, and it disappears when this count reaches the cast.
    ///
    /// a FLOOR again, and the control is the same: it must not yet be
    /// everybody, because the day it is, the bridge is dead and this test should
    /// be replaced by the refusal rather than kept as a ratchet.
    #[test]
    fn the_cast_that_states_its_own_verbs_only_grows() {
        let authored: Vec<&str> = crate::character_catalog::buildable_cast()
            .filter(|id| {
                authored_intrinsics(
                    id,
                    ambition_platformer2d_actor_monolith::character_runtime::CharacterDefinition::new(
                        *id,
                        *id,
                        crate::AMBITION_CONTENT_PROVIDER,
                    ),
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
        crate::character_catalog::register(&mut app);
        // The lineage and declared cast are separate registration paths; include both.
        crate::player_robot_lineage::register(&mut app);
        crate::player_robot_lineage::register_declared_cast(&mut app);
        ambition_platformer2d_shared_tangle::app_finalization::finalize(&mut app);
        let prepared = app
            .world()
            .resource::<ambition_platformer2d_actor_monolith::character_runtime::PreparedCharacterRegistry>();

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
                ambition_platformer2d_actor_monolith::character_runtime::CharacterDefinition::new(
                    id,
                    id,
                    crate::AMBITION_CONTENT_PROVIDER,
                ),
            ))
            // BOTH shapes count — this read only `autonomous_profile` at
            // first, and the exemption list's own rot-check caught it: the goblin
            // and the lab raider state their policy by NAME
            // (`autonomous_profile_ref` → the shared `medium_striker` entry),
            // which is just as much an authority as an inlined one.
            .map(|definition| {
                definition.autonomous_profile.is_some()
                    || definition.autonomous_profile_ref.is_some()
            })
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

        // and the exemption list cannot rot: one that got FIXED must LEAVE it,
        // or the count stops meaning anything and the list becomes decoration.
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

    /// The giant carries its own facts now — every one its archetype row stated, authored
    /// on the definition, and that row is DELETED ( closed once three layers learned to ask the
    /// character before the archetype: the limbed-host predicate, the activation path's
    /// construction context, and `mount_capabilities_of`).
    #[test]
    fn the_giant_gnu_authors_the_mount_its_archetype_row_used_to() {
        use ambition_characters::brain::{CharacterBrainTemplate, MoveStyleSpec};

        let definition = authored_intrinsics(
            "npc_giant_gnu",
            ambition_platformer2d_actor_monolith::character_runtime::CharacterDefinition::new(
                "npc_giant_gnu",
                "Giant GNU",
                crate::AMBITION_CONTENT_PROVIDER,
            ),
        );
        assert_eq!(definition.vitals.max_health, Some(42));
        assert_eq!(
            definition.vitals.mass,
            Some(8.0),
            "the mount pair's centre of gravity sits on the giant"
        );
        let locomotion = definition.locomotion.expect("it states its gait");
        assert_eq!(locomotion.run_speed, 0.0, "stationary, and it SAYS so");
        assert!(matches!(locomotion.move_style, MoveStyleSpec::WalkHeavy));
        assert!(
            definition.contact_damage.is_none(),
            "standing next to a prop does not hurt"
        );
        let mount = definition.mount.expect("it is a mount");
        assert_eq!(mount.class.as_deref(), Some("giant"));
        assert!(
            mount.pilotable_classes.is_empty(),
            "the giant rides nothing"
        );
        let profile = definition.autonomous_profile.expect("its policy");
        assert_eq!(profile.template, CharacterBrainTemplate::StandStill);
        assert_eq!(
            profile.aggro_radius, 0.0,
            "the scholar on its shoulders is the threat, and a driver that \
             notices nobody is the whole of what the deleted `attacks_player` \
             said as POLICY — the rest of it was a relationship, and the \
             sandbox placement says `Peaceful`"
        );
        assert_eq!(profile.attack_range, 0.0);
    }

    /// The two shark riders differ from each other, which is what the pair of
    /// nearly-identical archetype rows existed to express. Health, weight,
    /// pace, gait, bolt damage and which gun-sword — six numbers and a row each.
    ///
    /// neither authors `contact_damage`, and that is the migration doing its
    /// job: both rows carried a `contact_strength` and a `damage_amount` beside
    /// `body_contact_damage: false`, which turned them off. Numbers that
    /// described nothing.
    #[test]
    fn the_shark_riders_author_the_bodies_their_archetype_rows_used_to() {
        use ambition_characters::brain::{CharacterBrainTemplate, MoveStyleSpec};

        let rider = |id: &str| {
            authored_intrinsics(
                id,
                ambition_platformer2d_actor_monolith::character_runtime::CharacterDefinition::new(
                    id,
                    "Rider",
                    crate::AMBITION_CONTENT_PROVIDER,
                ),
            )
        };
        let light = rider("npc_pirate_raider");
        let heavy = rider("npc_pirate_heavy_iron_mary");

        assert_eq!(light.vitals.max_health, Some(4));
        assert_eq!(heavy.vitals.max_health, Some(6), "Iron Mary is the heavy");
        assert_eq!(light.held_item.as_deref(), Some("gun_sword"));
        assert_eq!(heavy.held_item.as_deref(), Some("gun_sword_heavy"));
        assert!(
            light.contact_damage.is_none() && heavy.contact_damage.is_none(),
            "touching a raider does not hurt; its gun-sword does"
        );

        let light_locomotion = light.locomotion.expect("it states its pace");
        let heavy_locomotion = heavy.locomotion.expect("so does she");
        assert_eq!(light_locomotion.run_speed, 230.0);
        assert_eq!(heavy_locomotion.run_speed, 215.0);
        assert!(matches!(light_locomotion.move_style, MoveStyleSpec::Walk));
        assert!(matches!(
            heavy_locomotion.move_style,
            MoveStyleSpec::WalkHeavy
        ));

        for (definition, effort) in [(&light, 0.4783), (&heavy, 0.5116)] {
            let profile = definition.autonomous_profile.expect("the standoff policy");
            assert_eq!(profile.template, CharacterBrainTemplate::Skirmisher);
            assert_eq!(profile.aggro_radius, 1200.0);
            assert_eq!(
                profile.patrol_effort, effort,
                "a TUNED amble — the number the constructor's literal 0.5 would \
                 have silently replaced"
            );
            let mount = definition.mount.as_ref().expect("it boards a shark");
            assert_eq!(mount.pilotable_classes, vec!["shark".to_string()]);
            assert!(mount.class.is_none(), "a raider is not itself rideable");
            assert!(
                definition
                    .action_set
                    .as_ref()
                    .is_some_and(|set| set.ranged.is_some()),
                "the bolt is the whole standoff"
            );
        }
    }

    /// The giant's left and right hands reuse the same character definition.
    #[test]
    fn the_giants_hands_author_the_limb_their_archetype_row_used_to() {
        use ambition_characters::brain::CharacterBrainTemplate;

        let definition = authored_intrinsics(
            "npc_giant_gnu_hands",
            ambition_platformer2d_actor_monolith::character_runtime::CharacterDefinition::new(
                "npc_giant_gnu_hands",
                "Giant GNU Hand",
                crate::AMBITION_CONTENT_PROVIDER,
            ),
        );
        assert_eq!(definition.vitals.max_health, Some(42));
        assert_eq!(definition.vitals.mass, Some(2.0));
        assert!(
            definition.contact_damage.is_none(),
            "a limb is not a hazard"
        );
        assert!(
            definition.mount.is_none(),
            "a hand is neither ridden nor rides"
        );
        let profile = definition.autonomous_profile.expect("its policy");
        assert_eq!(profile.template, CharacterBrainTemplate::StandStill);
        assert_eq!(
            profile.aggro_radius, 0.0,
            "the rider's routed strikes hurt; the hand is their vehicle, and a \
             vehicle notices nobody"
        );
        assert_eq!(profile.attack_range, 0.0);
    }

    /// The practice target says it is one. `practice_target` is the fact
    /// with four consumers — the save sync, the path assignment and two sprite
    /// reads — and the one that kept the sandbags on the archetype file.
    ///
    /// it authors NO contact damage, and the old row's comment claimed
    /// otherwise directly above the `body_contact_damage: false` that turned it
    /// off. Believing the prose would have given the dummy a hitbox it never
    /// had.
    #[test]
    fn the_sandbag_authors_the_dummy_its_archetype_row_used_to() {
        use ambition_characters::brain::CharacterBrainTemplate;

        let definition = authored_intrinsics(
            "sandbag",
            ambition_platformer2d_actor_monolith::character_runtime::CharacterDefinition::new(
                "sandbag",
                "Sandbag",
                crate::AMBITION_CONTENT_PROVIDER,
            ),
        );
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

    /// THE FIRST CHARACTER THAT NAMES ITS POLICY INSTEAD OF CARRYING ONE.
    ///
    /// The goblin's five sandbox placements wore the `medium_striker` ARCHETYPE
    /// — a whole body borrowed for its fighting style. Its controller half is a
    /// shared `autonomous_profiles` entry now, and the goblin points at it while
    /// keeping its own health, reach and pace.
    ///
    /// the reference is PROVIDER-NAMESPACED, because assembly namespaces every
    /// preset map: a bare "medium_striker" resolves to nothing.
    #[test]
    fn the_goblin_names_the_shared_striker_policy() {
        use ambition_characters::brain::MoveStyleSpec;

        let definition = authored_intrinsics(
            "goblin",
            ambition_platformer2d_actor_monolith::character_runtime::CharacterDefinition::new(
                "goblin",
                "Goblin",
                crate::AMBITION_CONTENT_PROVIDER,
            ),
        );
        assert_eq!(definition.vitals.max_health, Some(5));
        let locomotion = definition.locomotion.expect("its own body");
        assert_eq!(locomotion.run_speed, 170.0);
        assert!(matches!(locomotion.move_style, MoveStyleSpec::Walk));
        assert_eq!(
            definition
                .autonomous_profile_ref
                .as_ref()
                .map(ambition_characters::brain::BrainProfileRef::as_str),
            Some("medium_striker"),
            "it NAMES the shared policy, provider-relative; carrying one inline \
             would make it unshareable, which is the whole point"
        );
        assert!(
            definition.autonomous_profile.is_none(),
            "and does not also carry one — two authorities for one decision"
        );
    }

    /// The shared policy exists in the shipped catalog, and says only
    /// controller things. A body fact in here would be the archetype's
    /// three-authorities muddle arriving by another door.
    #[test]
    fn the_shipped_catalog_authors_a_shared_striker_policy() {
        // the SHIPPED bytes, parsed the way the game parses them — and the
        // key is namespaced by ASSEMBLY, which `load_catalog` does not perform,
        // so this reads the local name the file authors.
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
    /// this is the guard that stops the NEXT one, and it matters most while
    /// the preset vocabulary is being retired: a key whose last adopter migrates
    /// to a `BrainProfile` should fail here on the same change that moved it,
    /// rather than sitting in the file as a row somebody later has to migrate.
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

    /// THE ADMIRAL SAYS IT CAN RIDE A SHARK, AND SAYS IT HERE.
    ///
    /// ⛔⛔ THIS IS THE FACT THE SMASH UP-B SHIPPED WITHOUT. The capability was
    /// manufactured by the match instead — granted per seat by `smash_roster`,
    /// and NOT by `SmashSelect::roster_seeded`, which is the road a player
    /// travels from the character-select grid. So the admiral reached a real
    /// match unable to board the shark its own up-B summons, and the shark just
    /// stood there. Jon found it by playing.
    ///
    /// ⭐ A CHARACTER FACT IS INHERITED BY EVERY ROAD. `prepared_match` unions
    /// `pilotable_classes` into `CanPilot` wherever a body is realized, so there
    /// is no second list for a future roster builder to forget. Jon settled the
    /// premise this rests on: *"Yes the admiral could fly on a shark in
    /// ambition"* — the up-B is Smash-only, the PILOTING is not.
    #[test]
    fn the_pirate_admiral_can_pilot_a_shark_because_it_is_a_pirate_admiral() {
        let definition = authored_intrinsics(
            "npc_pirate_admiral",
            ambition_platformer2d_actor_monolith::character_runtime::CharacterDefinition::new(
                "npc_pirate_admiral",
                "Pirate Admiral",
                crate::AMBITION_CONTENT_PROVIDER,
            ),
        );
        let mount = definition
            .mount
            .as_ref()
            .expect("an admiral states what it can board");
        assert_eq!(
            mount.pilotable_classes,
            vec!["shark".to_string()],
            "the admiral cannot pilot a shark, so its up-B summons a mount it \
             may not board"
        );
        // ⭐ AND IT IS NOT ITSELF RIDEABLE, which is the other half of the same
        // sentence — `npc_pirate_raider` states the identical pair.
        assert!(
            mount.class.is_none(),
            "an admiral became something you can ride"
        );
    }

    /// Every character the provocation name-matcher answers states its own
    /// provoked policy.
    ///
    /// A single character that did not would fall through to the matcher, find no row, and
    /// become a generic `combatant` with nothing to read.
    #[test]
    fn every_pirate_answers_the_provocation_question_for_itself() {
        let light = [
            "npc_pirate_admiral",
            "npc_pirate_raider",
            "npc_pirate_quartermaster",
            "npc_pirate_lookout",
            "npc_pirate_navigator",
            "npc_pirate_cutlass_viper",
        ];
        let heavy = [
            "npc_pirate_heavy_broadside_bess",
            "npc_pirate_heavy_iron_mary",
            "npc_pirate_heavy_salt_annet",
        ];
        for (ids, expected) in [
            (&light[..], "pirate_boarder"),
            (&heavy[..], "pirate_boarder_heavy"),
        ] {
            for id in ids {
                let definition = authored_intrinsics(
                    id,
                    ambition_platformer2d_actor_monolith::character_runtime::CharacterDefinition::new(
                        *id,
                        *id,
                        crate::AMBITION_CONTENT_PROVIDER,
                    ),
                );
                assert_eq!(
                    definition
                        .provoked_profile_ref
                        .as_ref()
                        .map(ambition_characters::brain::BrainProfileRef::as_str),
                    Some(expected),
                    "`{id}` still needs the display-name matcher to know what it \
                     becomes when struck"
                );
            }
        }
        // and a NON-pirate must not pick one up, or the rule is a blanket
        // rather than a migration.
        let goblin = authored_intrinsics(
            "goblin",
            ambition_platformer2d_actor_monolith::character_runtime::CharacterDefinition::new(
                "goblin",
                "goblin",
                crate::AMBITION_CONTENT_PROVIDER,
            ),
        );
        assert!(goblin.provoked_profile_ref.is_none());
    }

    // It swept `character_archetypes.ron` for rows belonging to creatures that had become
    // characters — nine of them, each one a place where "two authorities describe one creature"
    // would have been true. Its control asserted the file still held `combatant`, so it could
    // not pass on an empty file.
    //
    //  the file is deleted, so no creature can have two authorities: a body is
    // built from its character or construction refuses it.

    /// The runtime's cast comes OUT of the compiler.
    ///
    /// Not "the compiler also checks it" — out of it.
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

    /// Production registration and the compiler are ONE authority.
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

    /// The two lists answer two questions, and the build-only one has to obey the same rules
    /// as the selection one.
    ///
    /// poison: empty an arm of [`authored_intrinsics`] and this reds. That
    /// matters more than it looks — a registered character that authors nothing
    /// does not fall back to its archetype, it simply has no death behaviour,
    /// and an exploding mite that stops exploding is invisible until someone
    /// stands next to one.
    #[test]
    fn the_migrated_mites_author_their_own_death_and_health() {
        for (id, explodes, divides_into, health) in [
            ("npc_exploding_mite", true, None, 2),
            ("npc_dividing_mite", false, Some("npc_puppy_slug"), 4),
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
        for id in buildable_only_cast() {
            let bare =
                ambition_platformer2d_actor_monolith::character_runtime::CharacterDefinition::new(
                    id,
                    "unused",
                    crate::AMBITION_CONTENT_PROVIDER,
                );
            let authored = authored_intrinsics(id, bare.clone());
            let authors_a_body =
                authored.death_traits.is_some() || authored.vitals.max_health.is_some();
            // A POLICY-ONLY REGISTRATION RETRACTS NOTHING, and this guard
            // could not previously say so.
            //
            // The rule above is right about BODIES: a definition that states no
            // vitals says *"this character authors none"*, preparation correctly
            // retracts, and the recorded cost is ~100 exploration NPCs losing
            // their archetype-built ones. It was applied as a blanket, and it
            // therefore also refused a character that states only a CONTROLLER
            // policy — which has no body to retract, and whose statement is true
            // whether or not anyone ever authors its vitals.
            //
            // the distinction is REAL and it is checked elsewhere, not asserted
            // here: `an_incomplete_character_uses_peaceful_npc_defaults` pins that a
            // registered-but-incomplete definition does not partially leak body
            // facts into the peaceful-NPC path; only a complete blueprint supplies
            // character-owned vitals/locomotion there.
            let authors_only_policy = !authors_a_body && authored != bare;
            // AND A THIRD SAFE CASE: a character that has no archetype body
            // to lose. The rule protects ARCHETYPE-built vitals; a character
            // placed only as a peaceful Hall `NpcSpawn` never had any, so a bare
            // registration costs it nothing and buys it a seat. Each entry
            // carries the placement evidence, because that is the whole argument.
            const KNOWN_BARE_REGISTRATIONS: &[(&str, &str)] = &[(
                "npc_carl_stargan",
                "one placement: hall_of_characters NpcSpawn, brain_override \
                 stand_still. Never an EnemySpawn, so no archetype vitals exist \
                 to retract. Registered because Jon put him on the Smash grid \
                 (2026-08-11) and the grid drops what it cannot seat.",
            )];
            let exempt = KNOWN_BARE_REGISTRATIONS
                .iter()
                .any(|(known, _)| *known == id);
            assert!(
                authors_a_body || authors_only_policy || exempt,
                "`{id}` is registered as buildable and authors NOTHING — not a \
                 body, not a policy, not a moveset. A bare registration means it \
                 has no body, not that its archetype keeps it. If it has no \
                 archetype body to lose, say so in `KNOWN_BARE_REGISTRATIONS` \
                 with the placement evidence."
            );
        }
    }

    /// AND THE OTHER DIRECTION, which is the one that loses work silently.
    ///
    /// The dangerous direction is the reverse: a character somebody wrote an
    /// `authored_intrinsics` arm for and never added to either list. It is never registered,
    /// so the arm runs for nobody, and nothing anywhere fails — the body simply does not exist
    /// and the author's work sits in the file looking done.
    ///
    /// the question is answerable without parsing the match: hand
    /// `authored_intrinsics` a bare definition for EVERY character in the
    /// assembled catalog and ask whether it came back changed. An id it changes
    /// is an id it has an arm for.
    #[test]
    fn every_character_with_an_authored_body_is_registered_as_buildable() {
        // Six pirates could not deliver the `provoked_profile_ref` the prefix rule gives them,
        // and the Patent Clerk's eleven-move repertoire reached no body. Both were silent: a
        // body that is never built cannot break.
        const KNOWN_UNREGISTERED: &[(&str, &str)] = &[];

        let catalog = load_catalog();
        let registered: std::collections::BTreeSet<&str> = buildable_cast().collect();
        let mut unregistered = Vec::new();
        for id in catalog.data().characters.keys() {
            let bare =
                ambition_platformer2d_actor_monolith::character_runtime::CharacterDefinition::new(
                    id.as_str(),
                    "unused",
                    crate::AMBITION_CONTENT_PROVIDER,
                );
            if authored_intrinsics(id.as_str(), bare.clone()) != bare
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

        // and the exemption list cannot rot: one that got FIXED must LEAVE it,
        // or the seven stop being a count and become decoration.
        let stale: Vec<_> = KNOWN_UNREGISTERED
            .iter()
            .filter(|(id, _)| !unregistered.iter().any(|found| found == id))
            .collect();
        assert!(
            stale.is_empty(),
            "these are exempted as unregistered and are no longer unregistered — \
             remove them from `KNOWN_UNREGISTERED`: {stale:?}"
        );

        // the control. If `authored_intrinsics` ever became the identity for
        // every id — a refactor that dropped the match, say — the loop above
        // would find nothing and pass while checking nothing at all.
        let authors_someone = catalog.data().characters.keys().any(|id| {
            let bare =
                ambition_platformer2d_actor_monolith::character_runtime::CharacterDefinition::new(
                    id.as_str(),
                    "unused",
                    crate::AMBITION_CONTENT_PROVIDER,
                );
            authored_intrinsics(id.as_str(), bare.clone()) != bare
        });
        assert!(
            authors_someone,
            "no character in the catalog authors any intrinsics — the check above \
             is passing over an empty set"
        );
    }

    /// ALL NINE PIRATES DELIVER THE POLICY THE PREFIX RULE GIVES THEM — the thing
    /// registration actually buys, asserted at the seam a provoked body reads.
    ///
    /// the rule (`id.starts_with("npc_pirate_")` → one of two published profiles) has always
    /// applied to all nine rows.
    ///
    /// this asserts the END of that chain rather than the rule: every pirate
    /// in the shipped catalog resolves a provoked profile through PREPARATION,
    /// which is the only form the runtime can use.
    #[test]
    fn every_pirate_delivers_the_provoked_policy_its_rule_states() {
        let catalog = load_catalog();
        let pirates: Vec<String> = catalog
            .data()
            .characters
            .keys()
            .filter(|id| id.starts_with("npc_pirate_"))
            .cloned()
            .collect();
        assert!(
            pirates.len() >= 9,
            "the prefix rule is written for nine pirate rows; found {}",
            pirates.len()
        );

        let registered: std::collections::BTreeSet<&str> = buildable_cast().collect();
        let mut broken = Vec::new();
        for id in &pirates {
            let bare =
                ambition_platformer2d_actor_monolith::character_runtime::CharacterDefinition::new(
                    id.as_str(),
                    "unused",
                    crate::AMBITION_CONTENT_PROVIDER,
                );
            // BOTH halves, because either alone is silent. The rule must state a
            // policy, AND the id must be one registration actually visits — an
            // arm that runs for nobody is what this whole row was about.
            let states = authored_intrinsics(id.as_str(), bare)
                .provoked_profile_ref
                .is_some();
            if !states || !registered.contains(id.as_str()) {
                broken.push((id.clone(), states, registered.contains(id.as_str())));
            }
        }
        assert!(
            broken.is_empty(),
            "these pirates do not deliver a provoked policy — `(id, states_one, \
             registered)`: {broken:?}. A policy stated by a rule that registration \
             never visits reaches no body, and provoking one falls to the generic \
             archetype instead."
        );

        // the poison: a character the rule does NOT name must not acquire one
        // by accident. Without it this would also pass on a build where every
        // character got a provoked policy from somewhere else.
        let bare =
            ambition_platformer2d_actor_monolith::character_runtime::CharacterDefinition::new(
                "npc_alice",
                "unused",
                crate::AMBITION_CONTENT_PROVIDER,
            );
        assert!(
            authored_intrinsics("npc_alice", bare)
                .provoked_profile_ref
                .is_none(),
            "a character outside the pirate rule must state no provoked policy"
        );
    }

    /// it is empty today, so this asserts the CONTRACT rather than any
    /// current content: an id here must resolve a catalog row, and must not
    /// duplicate the selection cast — registering a character twice is how a
    /// definition silently loses to whichever registration ran last.
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
        // And the union is what registration actually walks, so a reader can
        // trust the two constants without reading `register_declared_cast`.
        let union: Vec<&str> = buildable_cast().collect();
        assert_eq!(
            union.len(),
            PLAYABLE_ROSTER.len() + buildable_only_cast().count()
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

#[cfg(test)]
mod assembled_provider_tests {
    /// four attempts to let a character name no brain preset ended with the Hall's
    /// `brain_override` resolving BARE, which implies the catalog reaching the NPC road holds
    /// unassembled entries. So: ask the assembled catalog directly.
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

    /// EVERY CHARACTER HAS EXACTLY ONE AUTONOMOUS-POLICY AUTHORITY, and this
    /// asks whether it is REACHABLE.
    ///
    /// So a green test here is: the resolver either answers, or refuses in the one way that has
    /// an answer waiting.
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
                // Any OTHER error is a real content defect: a named preset that
                // does not exist, which no road can rescue.
                Err(other) => panic!("`{id}`: {other}"),
            }
        }

        // the redirect must have somewhere to go. Every character the
        // resolver refuses for has to author the profile the NPC road will ask
        // it for; one that authors neither is unauthored, and its body silently
        // becomes a stand-still.
        let authors_a_profile = |id: &str| {
            let definition = super::authored_intrinsics(
                id,
                ambition_platformer2d_actor_monolith::character_runtime::CharacterDefinition::new(
                    id,
                    id,
                    crate::AMBITION_CONTENT_PROVIDER,
                ),
            );
            definition.autonomous_profile.is_some() || definition.autonomous_profile_ref.is_some()
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

        // and both halves must be non-empty, or this test is measuring a world that does not
        // exist: some characters still resolve a preset, and some have migrated to a profile.
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
