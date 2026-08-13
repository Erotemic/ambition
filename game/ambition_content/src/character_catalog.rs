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
    "player_robot_v3",            // the player robot, v3 (current)
    "player_robot_v2",            // v2: the build before the SVG rig
    "robot",                      // v0: the original
    "goblin",                     // melee striker
    "npc_pirate_admiral",         // pistol + cutlass
    "perfect_cellular_automaton", // the PCA — see the note below (D74)
    // ⭐⭐ **`perfect_cellular_automaton` IS ON THE ROSTER (2026-08-13, ledger
    // D74), and what unblocked it was a coupling this campaign DELETED rather
    // than a bug anybody fixed.**
    //
    // It was held out for six days as an explicit WORKAROUND — "the grid is one
    // portrait shorter" — behind a chain of hypotheses that D74 records in full:
    // a lost kit, a missed sample, a fragile instrument, the provocation rebuild,
    // `is_aerial`. All five were refuted, leaving ONE standing, the vaguest and
    // the oldest: *"one more registered character is one more sheet demanded at
    // load"*, with a step-4 `vel.x` divergence as its symptom.
    //
    // ⇒ **the probe D74 asked for was about the WORLD, not the actor** — count
    // in-flight loads per step in both builds — and it answers the row twice
    // over. `CharacterLoadStates` reports `staged=3, ready=0` at every one of the
    // first twelve steps in BOTH builds, and the possession trail is identical to
    // the last decimal. There is no extra sheet, so there is no timing to differ.
    //
    // ⭐ **because registration stopped demanding art.** D73 made it declarative
    // (`try_register_character` ends without calling `CharacterLoadDemand::
    // request`; loading is driven by what a session STAGES). The last hypothesis
    // standing described a coupling that no longer exists, which is why the
    // symptom went with it and no fix was needed.
    //
    // ⚠ verified before landing, not inferred: `ambition_app` 337 + 179 + 1,
    // `ambition_content` 192 + 32, `ambition_demo_smash` 67, and the workspace
    // gate — all green with this line in.
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
/// **THE BUILD-ONLY CAST IS DERIVED, not listed.**
///
/// ⛔⛔ this was a 24-entry hand list beside a 19-arm `authored_intrinsics`
/// match, and the two disagreed in BOTH directions: five characters authored a
/// body without being listed (they reached registration through
/// [`PLAYABLE_ROSTER`] instead) and seven were listed without authoring one.
/// Keeping two hand-maintained lists agreeing is the failure that produced D98
/// (seven characters authoring facts nothing read) and D99 (Stargan missing
/// from the grid he had been added to) in a single run.
///
/// ⇒ authoring a character MAKES it buildable. `authored/` is one file per
/// creature and [`crate::authored::AUTHORED_CAST`] is the one table; twenty-two
/// of the twenty-four ids come from there now and cannot fall out of sync with
/// their own authoring, because they ARE their own authoring.
///
/// ⚠ what remains hand-listed is the genuinely different case below: characters
/// registered as buildable that author NO body. That is dangerous by default —
/// a bare registration loses whatever `character_archetypes.ron` used to give
/// the character (the measured ~100-NPC regression recorded on
/// [`PLAYABLE_ROSTER`]) — so each entry states why it is safe.
pub fn buildable_only_cast() -> impl Iterator<Item = &'static str> {
    crate::authored::authored_ids()
        .chain(REGISTERED_WITHOUT_A_BODY.iter().copied())
        // ⭐⭐ **BUILD-ONLY MEANS "AND NOT ON THE SELECTION CAST", and that used
        // to be a comment.** The old hand list carried *"⚠ the parrot is NOT
        // here and must not be: `stochastic_parrot` is already on
        // `PLAYABLE_ROSTER`, so listing it twice would register it twice"* — a
        // rule enforced by a reader noticing a note. Five characters author a
        // body AND appear on the select grid (the parrot, the goblin, the
        // admiral, the oni leader, the sandbag), so deriving the cast from the
        // authoring surfaced all five at once. Excluding here makes the rule
        // structural; `the_build_only_cast_resolves_rows_and_does_not_overlap_the_selection_cast`
        // still fails on an overlap reintroduced by hand below.
        .filter(|id| !PLAYABLE_ROSTER.contains(id))
}

/// See [`buildable_only_cast`]. Registered, buildable, and authoring nothing —
/// which is safe ONLY where the character never had archetype-built facts to
/// lose. Every entry says which.
///
/// ⚠ this list should SHRINK. An entry here is a character whose body is still
/// somebody else's to state.
const REGISTERED_WITHOUT_A_BODY: &[&str] = &[
    // ⭐ **the six remaining pirates.** They author no body of their own yet,
    // but they are not bare in the way that matters: every id starting with
    // `npc_pirate_` takes its provoked policy from the RULE at the head of
    // [`authored_intrinsics`], which is what replaced the substring matcher on
    // display names and dialogue nodes (ledger D84). Their VITALS are D96
    // item 8 — how tough a pirate quartermaster is is a content decision, and
    // authoring a number to empty this list would be inventing one.
    //
    // ⚠ **not derived from the rule, because the rule cannot enumerate.**
    // `starts_with("npc_pirate_")` covers a pirate added tomorrow automatically,
    // which is the property it exists for; this list answers the different
    // question of which ids to REGISTER, and that needs names.
    "npc_pirate_cutlass_viper",
    "npc_pirate_heavy_broadside_bess",
    "npc_pirate_heavy_salt_annet",
    "npc_pirate_lookout",
    "npc_pirate_navigator",
    "npc_pirate_quartermaster",
    // ⭐⭐ **JON ASKED FOR HIM ON THE SMASH GRID, 2026-08-11 — and he was not on
    // it.** The grid filters `SMASH_ROSTER` against the prepared REGISTRY
    // (`SmashRoster::assemble`), precisely so an unbuildable portrait is dropped
    // rather than offered — so Stargan was silently absent from the grid he had
    // been added to, and dropping is the safe behaviour that hid it.
    //
    // ⚠ **provably safe here**: he has exactly one placement in the game, a Hall
    // `NpcSpawn` with `brain_override: stand_still`, so he has never had
    // archetype-built vitals to lose. Whether he FIGHTS is still Jon's (D96
    // item 5).
    "npc_carl_stargan",
];

/// **What a migrated character authors about its own body.**
///
/// The registration loop builds a bare definition from the catalog row — id,
/// display name, sheet — which is all an unmigrated character can say. This is
/// where a character that has taken its facts back from
/// `character_archetypes.ron` states them.
///
/// ⛔ **an id in [`buildable_only_cast`] with no arm here is the bug that list's
/// doc warns about**: a bare registration means "this character authors no
/// body", and anything its archetype used to give it is simply lost. Author
/// first, register second.
pub fn authored_intrinsics(
    id: &str,
    definition: ambition_platformer2d_actor_monolith::character_runtime::CharacterDefinition,
) -> ambition_platformer2d_actor_monolith::character_runtime::CharacterDefinition {
    // ⭐⭐ **EVERY PIRATE STATES WHAT IT BECOMES WHEN STRUCK** (ledger D84).
    //
    // ⛔ **this is a RULE rather than nine arms, because the thing it replaces
    // was a rule too — a worse one.** `hostile_brain_id_for_actor` asks whether
    // an id, a display name or a dialogue node contains `"pirate"`, or one of
    // `"broadside bess"` / `"iron mary"` / `"salt annet"`, and hands the body a
    // whole archetype. Nine characters answer that matcher, and every one of them
    // has to state its own answer before the two rows it points at can die.
    //
    // ⚠ the heavy/light split is the matcher's own: it tests `pirate_heavy`
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
    // ⭐⭐ **AND THE CREATURE'S OWN FILE STATES THE REST.**
    //
    // ⛔ what stood here was a 850-line `match id`, nineteen arms deep: every
    // migrated creature's vitals, locomotion, abilities and autonomous policy in
    // one function, each arm carrying the note on which archetype row it
    // replaced. `character_archetypes.ron` is nearly deleted, but a table that
    // long is the same authority wearing Rust — and adding a character had
    // become *edit the catalog data, remember `buildable_only_cast`, add an arm
    // here, maybe touch a roster*.
    //
    // ⇒ `authored/` — one file per creature, beside its moveset, and ONE table
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
    use ambition_platformer2d_actor_monolith::avatar::StartingCharacter;

    /// **THE PUPPY SLUG'S PINS, beside the definition that states them.**
    ///
    /// ⭐ these six assertions used to live in the actor crate, reading a
    /// `character_archetypes.ron` row. That row is deleted: the slug states its
    /// own health, gait, top speed, surface cling, contact damage and wandering
    /// policy, and its placements state the disposition that made it ambient
    /// wildlife. Moving the pins rather than deleting them is what keeps the
    /// migration honest — the facts did not stop mattering, they changed owner.
    ///
    /// ⛔ and leaving them where they were would have been worse than losing
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

    /// **THE PARROT'S PINS**, beside the definition that states them.
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

        // ⚠ the control: the catalog still owns gravity-freedom, and this test
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

    /// **A migrated character has no archetype row left.**
    ///
    /// ⛔ the acceptance signal for this campaign is a DELETION, and a test that
    /// only checked the new authority would pass just as well with both
    /// standing. This is the other half: the file must not still describe a
    /// creature its character now describes.
    /// **WHICH OF AMBITION'S CHARACTERS CAN BUILD A BODY WITHOUT AN ARCHETYPE** —
    /// the census, as a test rather than as a number in a commit message.
    ///
    /// ⛔ **I measured this with a regex first and it was WRONG** (2026-08-12). A
    /// pattern over `authored_intrinsics`'s match arms cannot see nested braces,
    /// so it reported migrated characters — both shark riders among them — as
    /// incomplete, and would have put a false count in the ledger. The sound
    /// instrument is the one production uses: build the definition and ask
    /// `body_blueprint()`, which is the same call the spawn roads make.
    ///
    /// ⭐ **this number is the campaign's remaining distance.** A placement naming
    /// a body-complete character is built character-first and never touches the
    /// archetype road; every other one is why `combatant` still has to exist. The
    /// test asserts the count only moves UP, so a migration that quietly stops
    /// authoring locomotion cannot pass.
    ///
    /// ⚠ **NINETEEN as of 2026-08-12**, and the regex said thirteen — it missed
    /// both shark riders, the giant's hands, the salvage guard and the lab
    /// raider, every one of them migrated. Six characters is the size of the
    /// error a plausible-looking one-off measurement made, which is the argument
    /// for the census living here instead of in a shell pipeline.
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
                // ⚠ **"authors its own locomotion" is the CAMPAIGN's criterion**,
                // not a copy of `body_blueprint`'s. The brief says it in those
                // words — *"a placement naming a COMPLETE character (one that
                // states its locomotion) is built by `new_character_in`"* — and
                // `body_blueprint` happens to check the same single fact today.
                //
                // ⛔ if preparation ever requires a SECOND fact, this census
                // becomes optimistic rather than wrong, and the place to look is
                // `PreparedCharacterDefinition::body_blueprint`'s missing list.
                // Asking that function directly is not possible from here:
                // preparation's test entry point is `#[cfg(test)]` inside the
                // monolith, so it does not exist for another crate.
                definition.locomotion.is_some()
            })
            .collect();

        // ⚠ a FLOOR, not a pin: every migration adds one, and a test that had to
        // be edited on the way past would be edited without being read.
        assert!(
            complete.len() >= 19,
            "only {} of Ambition's characters can build a body without an \
             archetype, and it was NINETEEN on 2026-08-12 — a migration does not \
             REMOVE completeness. Complete: {complete:?}",
            complete.len()
        );

        // ⛔ and the control: the count must not be everybody, or `is_ok()` is
        // answering something other than "this character authored a body".
        let total = crate::character_catalog::buildable_cast().count();
        assert!(
            complete.len() < total,
            "every one of the {total} buildable characters reports body-complete, \
             which would mean `body_blueprint` has stopped distinguishing — the \
             migration is not finished, so this cannot be true yet"
        );
    }

    /// **AND HOW MANY STATE THEIR OWN VERBS** — P3.25's number, measured the same
    /// way and for the same reason.
    ///
    /// ⭐ `effective_abilities` is an INTERSECTION when a character authors an
    /// `AbilitySet` — the mode may forbid and may never grant — but its third arm
    /// is `(None, mode) => mode`: a character that authors nothing takes the
    /// mode's whole set as a GRANT. That arm is the scaffold P3.25 deletes, and
    /// it disappears when this count reaches the cast.
    ///
    /// ⚠ **a FLOOR again, and the control is the same**: it must not yet be
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

    /// **WHO STILL NEEDS THE PROBE SEAM** — the population that keeps
    /// `adopt_character_intrinsics` alive, and therefore the size of checklist
    /// item 9 (*"delete it once the constructor replaces the precedence it
    /// performs"*).
    ///
    /// The enemy spawn road builds a placement's body one of two ways: a
    /// character whose prepared definition yields a `body_blueprint` is built
    /// FROM it, and a character that is registered but cannot yet build a body
    /// still gets to CORRECT one — that second road is
    /// `adopt_character_intrinsics`, described at its call site as serving *"the
    /// shrinking population of half-migrated characters"*. Nothing said how
    /// large that population was.
    ///
    /// ⭐ **14 of 36, and two thirds of them are already filed as content
    /// decisions**: six pirates (D96 item 8, *"a pirate quartermaster's
    /// vitals"* — the entry says six and it is exactly these six),
    /// `npc_carl_stargan` (D96 item 5), four Hall NPCs, and three characters
    /// that author their own MOVE timelines while still not authoring a body.
    ///
    /// ⚠ **body-incomplete is not unseatable**, and reading it that way would be
    /// wrong: `npc_pirate_admiral` and `npc_ninja_shadow_oni_leader` are on the
    /// Smash grid and fight. It means "cannot build a body from the character
    /// ALONE" — the assist is what item 9 deletes, not the character.
    ///
    /// ⛔⛔ **AND THIS COUNT IS THE POPULATION, NOT THE CALLER COUNT — a
    /// distinction I got wrong when I first wrote this doc.** The enemy road
    /// reaches `adopt_character_intrinsics` only on the FALL-THROUGH, when a
    /// placement names a character whose `body_blueprint()` is `Err`. Zero
    /// shipped placements do: `worlds::tests::what_still_needs_an_archetype_row`
    /// reports one placement that resolves to no character at all and none that
    /// resolves to an incomplete one. ⇒ **the seam has no shipped caller today.**
    /// These 14 are what would reach it if one of them were placed as an enemy,
    /// which is why the number is still worth ratcheting — but item 9's deletion
    /// is not waiting on them.
    ///
    /// ⚠ a CEILING rather than a floor, because this one is supposed to shrink.
    #[test]
    fn the_cast_that_still_needs_a_body_assist_only_shrinks() {
        let mut app = bevy::prelude::App::new();
        crate::character_catalog::register(&mut app);
        crate::player_robot_lineage::register(&mut app);
        crate::player_robot_lineage::register_declared_cast(&mut app);
        ambition_platformer2d_shared_tangle::app_finalization::finalize(&mut app);
        let prepared = app
            .world()
            .resource::<ambition_platformer2d_actor_monolith::character_runtime::PreparedCharacterRegistry>();

        let mut incomplete: Vec<&str> = prepared
            .iter()
            .filter(|(_, definition)| definition.body_blueprint().is_err())
            .map(|(id, _)| id)
            .collect();
        incomplete.sort();

        assert!(
            incomplete.len() <= 14,
            "{} characters cannot build a body from their own definition, and it \
             was FOURTEEN on 2026-08-13. This number is supposed to fall: every \
             one of them is a body `adopt_character_intrinsics` has to correct, \
             and that seam is checklist item 9's deletion. Incomplete: \
             {incomplete:?}",
            incomplete.len()
        );
        assert!(
            !incomplete.is_empty(),
            "every registered character can now build its own body — \
             `adopt_character_intrinsics` has no population left to serve, so \
             delete it (checklist item 9) and this ratchet with it"
        );
    }

    /// **AND HOW MANY STATE THEIR OWN MOVES** — P3.24's number, which had no
    /// ratchet while its twin above did.
    ///
    /// ⭐ **the named subject of P3.24 is already deleted**: `smash_fighter_kit()`
    /// is gone, and its numbers moved verbatim into
    /// `DeclaredCombatRules::unarmed_melee`, where a ruleset fact belongs — a
    /// STAGE states what an unarmed fighter swings. But the concept survives the
    /// rename, and so does the thing that ends it: a character that authors its
    /// own timelines never reaches that floor.
    ///
    /// ⚠ **a floor and a control, exactly like the verbs ratchet.** It must not be
    /// empty (or the unarmed declaration is what every fight is made of, and no
    /// content exercises the authored road) and it must not yet be everybody.
    ///
    /// ⛔⛔ **but the control does NOT instruct a deletion, and that is a
    /// correction.** Most of this cast authors `default_action_set: "peaceful"`
    /// — `melee: None, ranged: None, special: None` — deliberately, and Mary-O's
    /// row says so outright: *"Mary-O Classic is deliberately only the run/jump
    /// floor."* `unarmed_melee` is what lets such a character be seated at all,
    /// so whether it is scaffolding or permanent architecture is a PRODUCT
    /// question rather than a migration step.
    ///
    /// ⛔ **this asks the PREPARED registry, not the authoring functions.** A
    /// moveset reaches a fighter as `PreparedCharacterDefinition::authored_moveset`
    /// — which is the field `ambition_demo_smash` actually filters on when it
    /// decides which seats need the floor — so asking anything else would measure
    /// a different set than the one the game uses.
    #[test]
    fn the_cast_that_states_its_own_moves_only_grows() {
        let mut app = bevy::prelude::App::new();
        crate::character_catalog::register(&mut app);
        // ⛔ BOTH, and the pair is not interchangeable: `register_declared_cast`
        // deliberately SKIPS the lineage ("the lineage registers itself above"),
        // so a fixture with only that call measures the NPC cast and silently
        // leaves out the player robots — the characters most likely to author a
        // table. Measured before this line existed: 33 characters, no robot
        // among them.
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
            "no character in the prepared cast states its own move timelines, so              every seated fighter is made of `DeclaredCombatRules::unarmed_melee`              and the authored road is exercised by no content at all"
        );
        let total = prepared.ids().count();
        assert!(
            authored.len() < total,
            "every one of the {total} prepared characters states its own moves. \
             ⛔ that is not automatically the end of the floor: most of this cast \
             authors `default_action_set: \"peaceful\"` on purpose, so reaching \
             the whole cast means they were re-authored as fighters. \
             `DeclaredCombatRules::unarmed_melee` is what lets a peaceful \
             character be seated at all. Authored: {authored:?}"
        );
    }

    /// **A CHARACTER THAT AUTHORS ITS POLICY MUST NOT ALSO NAME A PRESET.**
    ///
    /// ⭐ the campaign's own rule — every migrated fact in exactly ONE authority —
    /// applied to the one place it was being broken sixteen times. A definition's
    /// `BrainProfile` outranks the row's `default_brain` everywhere it is read, so
    /// a character holding both states its policy twice and the loser is invisible
    /// until somebody reads three files. `npc_burning_flying_shark` was pointing
    /// at a SLUG'S WANDER while authoring `ChargeCrash`; nothing was wrong on
    /// screen, and the row was still absurd.
    ///
    /// ⛔ **the exemptions are the ones that cannot go YET, each with the reason**,
    /// and they are not a to-do list somebody may extend casually: a preset that
    /// carries `aggressiveness` also carries a RELATIONSHIP, which belongs to the
    /// placement's disposition, and dropping it first is what took the cove
    /// parrot's peacefulness away for four attempts.
    #[test]
    fn a_character_states_its_policy_in_one_place() {
        /// `(character, preset it still names, why it cannot drop it yet)`
        const KNOWN_DOUBLE_STATED: &[(&str, &str, &str)] = &[
            // ⭐ EMPTY as of 2026-08-12. Every character that authors a policy
            // now states it in exactly one place. ⛔ an entry added here must
            // carry the reason its character cannot drop the preset yet, and it
            // must LEAVE the moment that stops being true — the rot-check below
            // enforces that, and it has already caught one wrong entry.
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
            // ⚠ **BOTH shapes count** — this read only `autonomous_profile` at
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

        // ⛔ and the exemption list cannot rot: one that got FIXED must LEAVE it,
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

    /// **The giant carries its own facts now** — every one its archetype row
    /// stated, authored on the definition, and that row is DELETED (D76 closed
    /// once three layers learned to ask the character before the archetype: the
    /// limbed-host predicate, the activation path's construction context, and
    /// `mount_capabilities_of`).
    ///
    /// ⭐ the two facts that could not have been authored before this campaign:
    /// `is_hostile: false` (a mount whose RIDER is the threat) and a
    /// `run_speed` of exactly zero (a body that stands still, which the
    /// constructor used to read as "said nothing" and answer with a sprinter's
    /// top speed).
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

    /// **The two shark riders differ from each other, which is what the pair of
    /// nearly-identical archetype rows existed to express.** Health, weight,
    /// pace, gait, bolt damage and which gun-sword — six numbers and a row each.
    ///
    /// ⭐ neither authors `contact_damage`, and that is the migration doing its
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

    /// **One character, two bodies** — the giant's left and right hands are the
    /// same definition spawned twice by the rig, which is a reusable authored
    /// template doing the thing the campaign exists to make possible.
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

    /// **The practice target says it is one.** `practice_target` is the fact
    /// with four consumers — the save sync, the path assignment and two sprite
    /// reads — and the one that kept the sandbags on the archetype file.
    ///
    /// ⚠ it authors NO contact damage, and the old row's comment claimed
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

    /// **THE FIRST CHARACTER THAT NAMES ITS POLICY INSTEAD OF CARRYING ONE.**
    ///
    /// The goblin's five sandbox placements wore the `medium_striker` ARCHETYPE
    /// — a whole body borrowed for its fighting style. Its controller half is a
    /// shared `autonomous_profiles` entry now, and the goblin points at it while
    /// keeping its own health, reach and pace.
    ///
    /// ⚠ the reference is PROVIDER-NAMESPACED, because assembly namespaces every
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

    /// **The shared policy exists in the shipped catalog, and says only
    /// controller things.** A body fact in here would be the archetype's
    /// three-authorities muddle arriving by another door.
    #[test]
    fn the_shipped_catalog_authors_a_shared_striker_policy() {
        // ⚠ the SHIPPED bytes, parsed the way the game parses them — and the
        // key is namespaced by ASSEMBLY, which `load_catalog` does not perform,
        // so this reads the local name the file authors.
        let catalog = load_catalog();
        let profile = catalog
            .autonomous_profile("medium_striker")
            .expect("the shipped catalog authors the shared striker policy");
        assert_eq!(profile.aggro_radius, 460.0);
        assert_eq!(profile.attack_range, 150.0);
        assert_eq!(profile.patrol_effort, 0.6176);
        assert!(profile.smash_dash_to_close);
    }

    /// **Every authored brain preset has at least one character using it.**
    ///
    /// ⛔ **`sniper_default` was authored, validated and reachable by nobody**
    /// (found 2026-08-11, ledger D81). It cost nothing to run and everything to
    /// reason about: a retirement census has to decide what to do with a policy
    /// no body has, and the honest answer — delete it — was invisible until
    /// somebody counted `default_brain:` by hand.
    ///
    /// ⭐ this is the guard that stops the NEXT one, and it matters most while
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

    /// **Every character the provocation name-matcher answers states its own
    /// provoked policy.**
    ///
    /// ⛔ **the measurement D84 said had to happen before the rows go.** Nine
    /// characters match `hostile_brain_id_for_actor`'s substring test — six on
    /// `"pirate"`, three on the named heavies — and the archetype rows those two
    /// arms point at (`pirate_raider`, `pirate_heavy`, 103 lines) can only be
    /// deleted once EVERY one of them answers for itself. A single character
    /// that did not would fall through to the matcher, find no row, and become a
    /// generic `combatant` with nothing to read.
    ///
    /// ⚠ the heavy/light split is asserted, not just the presence: the matcher
    /// tests `pirate_heavy` first, and a migration that quietly gave Iron Mary
    /// the duelist policy would be a retune wearing a migration's commit.
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
        // ⚠ and a NON-pirate must not pick one up, or the rule is a blanket
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

    #[test]
    fn the_migrated_characters_rows_are_gone_from_the_archetype_file() {
        let rows = include_str!("../assets/data/character_archetypes.ron");
        for key in [
            "exploding_mite",
            "dividing_mite",
            "puppy_slug",
            "sky_parrot",
            "giant_gnu",
            "pirate_shark_rider",
            "pirate_heavy_shark_rider",
            "giant_gnu_hands",
            "sandbag_finite",
        ] {
            assert!(
                !rows.contains(&format!("\"{key}\": (")),
                "`{key}` still has a row in character_archetypes.ron, so two \
                 authorities describe one creature"
            );
        }
        // ⚠ the control: a creature that has NOT migrated must still be there,
        // or this test would pass on an empty file.
        assert!(rows.contains("\"combatant\": ("));
    }

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
            // ⭐⭐ **A POLICY-ONLY REGISTRATION RETRACTS NOTHING, and this guard
            // could not previously say so** (2026-08-12, ledger D98).
            //
            // The rule above is right about BODIES: a definition that states no
            // vitals says *"this character authors none"*, preparation correctly
            // retracts, and the recorded cost is ~100 exploration NPCs losing
            // their archetype-built ones. It was applied as a blanket, and it
            // therefore also refused a character that states only a CONTROLLER
            // policy — which has no body to retract, and whose statement is true
            // whether or not anyone ever authors its vitals.
            //
            // ⚠ that refusal was not free: it is what left six of the nine
            // pirates unable to deliver the `provoked_profile_ref` the prefix rule
            // gives them, after the string-matcher arms that used to do it were
            // deleted. A guard that blocks a fact from reaching the game is doing
            // damage, not preventing it.
            //
            // ⛔ the distinction is REAL and it is checked elsewhere, not asserted
            // here: `an_unmigrated_character_still_gets_the_roads_defaults` pins
            // that a registered-but-incomplete character keeps the road's
            // `max_health: 1` and `MAX_RUN_SPEED`, because the peaceful road reads
            // body facts only from a body-complete blueprint.
            let authors_only_policy = !authors_a_body && authored != bare;
            // ⭐ **AND A THIRD SAFE CASE: a character that has no archetype body
            // to lose.** The rule protects ARCHETYPE-built vitals; a character
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

    /// **AND THE OTHER DIRECTION, which is the one that loses work silently.**
    ///
    /// ⛔ `every_build_only_id_authors_something` asks *"is everything on the
    /// list authored?"* — the direction where the symptom is loud, because an
    /// unauthored registration strips a body and something falls over. The
    /// dangerous direction is the reverse: **a character somebody wrote an
    /// `authored_intrinsics` arm for and never added to either list.** It is
    /// never registered, so the arm runs for nobody, and nothing anywhere fails
    /// — the body simply does not exist and the author's work sits in the file
    /// looking done.
    ///
    /// ⭐ the question is answerable without parsing the match: hand
    /// `authored_intrinsics` a bare definition for EVERY character in the
    /// assembled catalog and ask whether it came back changed. An id it changes
    /// is an id it has an arm for.
    #[test]
    fn every_character_with_an_authored_body_is_registered_as_buildable() {
        // ⛔ **THE SEVEN THIS FOUND ON ITS FIRST RUN ARE REGISTERED NOW**, so
        // the exemption list they lived on is empty — see D98. Six pirates could
        // not deliver the `provoked_profile_ref` the prefix rule gives them, and
        // the Patent Clerk's eleven-move repertoire reached no body. Both were
        // silent: a body that is never built cannot break.
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

        // ⛔ and the exemption list cannot rot: one that got FIXED must LEAVE it,
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

        // ⛔ the control. If `authored_intrinsics` ever became the identity for
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

    /// **ALL NINE PIRATES DELIVER THE POLICY THE PREFIX RULE GIVES THEM** — the
    /// thing D98's registration actually buys, asserted at the seam a provoked
    /// body reads.
    ///
    /// ⛔ the rule (`id.starts_with("npc_pirate_")` → one of two published
    /// profiles) has always applied to all nine rows. Only three of them were
    /// registered, so only three had a PREPARED definition for `record_provoked`
    /// to read — and the string-matcher arms that used to hand the other six the
    /// pirate policy were deleted in the same change that added the rule. Six
    /// pirates were provoked into `pirate_boarder` before the migration and into
    /// generic `combatant` after it, and nothing said so.
    ///
    /// ⭐ this asserts the END of that chain rather than the rule: every pirate
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

        // ⛔ the poison: a character the rule does NOT name must not acquire one
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

    /// ⚠ it is empty today, so this asserts the CONTRACT rather than any
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
    /// **Does an ASSEMBLED entry carry its provider?** — the falsifier for the
    /// fourth blocker (ledger D81).
    ///
    /// ⛔ four attempts to let a character name no brain preset ended with the
    /// Hall's `brain_override` resolving BARE, which implies the catalog reaching
    /// the NPC road holds unassembled entries. That is an inference from a
    /// symptom, and this campaign has already been wrong twice about symptoms in
    /// this exact area. So: ask the assembled catalog directly.
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

        // ⚠ **and a character that DOES name one is still namespaced by
        // assembly**, which is what the old inference relied on and what the
        // ordered fallbacks still use for rows the provider has not reached.
        // ⛔ this assertion used to be made about the PARROT, and it went red the
        // moment the parrot stopped naming a preset — a test whose subject
        // migrated out from under it. The subject has to be a row that still
        // holds the property.
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

    /// **EVERY CHARACTER HAS EXACTLY ONE AUTONOMOUS-POLICY AUTHORITY, and this
    /// asks whether it is REACHABLE** (GPT 5.6's review, 2026-08-12).
    ///
    /// ⛔ the review's finding, reproduced before it was fixed: a migrated
    /// character states its policy as a `BrainProfile` and its catalog
    /// `default_brain` was emptied so one authority decides — but the NPC road
    /// spoke only the PRESET vocabulary, and built `BrainPresetId::new("")` for
    /// the absence. Measured against the shipped worlds: two sandbox placements
    /// (`pirate_cove`'s parrot, `gravity_lab`'s puppy slug) author no
    /// `brain_override` at all and PANICKED at spawn with *"unknown preset ``"*;
    /// twenty-one Hall placements spawned holding `Some("")` and had every
    /// `RestoreDefault` rejected for the rest of the session.
    ///
    /// ⭐ **the assertion is about the SEAM, not about one function.** After the
    /// fix `resolve_initial_brain` deliberately REFUSES for these characters —
    /// `NoAutonomousDefault`, because lowering a profile needs the body and this
    /// crate has none — and the NPC road answers the redirect. So a green test
    /// here is: the resolver either answers, or refuses in the one way that has
    /// an answer waiting. A refusal with nothing behind it is the failure.
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

        // ⛔ **the redirect must have somewhere to go.** Every character the
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

        // ⚠ and both halves must be non-empty, or this test is measuring a world
        // that does not exist: some characters still resolve a preset, and some
        // have migrated to a profile. If either count hits zero the assertion
        // above has stopped being about anything.
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
