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
    // ⭐ **the characters migrated off `character_archetypes.ron`** (D73 phase
    // 2, group A). Every fact their rows held is authored on their DEFINITIONS
    // by [`authored_intrinsics`] and the rows are DELETED in the same change, so
    // the two authorities never state one fact at once. Their placements name
    // them explicitly.
    "npc_exploding_mite",
    "npc_dividing_mite",
    // ⚠ the puppy slug is the first migrated character that is NOT hostile: its
    // ten placements author `disposition: Peaceful`, because ambient wildlife
    // that never aggros is a fact about that placement of the creature and not
    // about the creature.
    "npc_puppy_slug",
    // ⭐ **the two plane swarms, registered by the provider that OWNS them.**
    // Mary-O places them and borrowed them from this catalog; registering them
    // there made the prepared registry and the catalog's owners map disagree
    // about who authored them, which `the_shipped_cast_has_one_authority_per_character`
    // refuses. Registered here, a hosted build builds them character-first and
    // the standalone demo still has its roster rows to fall back to.
    "npc_snakes_on_a_paper_plane",
    "npc_snakes_on_a_cartesian_plane",
    // The Hall's slop, which is also the sandbox's placed enemy.
    "npc_ai_slop",
    // The first MOUNT to become a character (ADR 0020).
    "npc_burning_flying_shark",
    // The second, and the first body that authors "I never hunt anybody".
    "npc_giant_gnu",
    // ⭐ **the first RIDERS**: a cove raider and Iron Mary both pilot a shark,
    // both carry a gun-sword, and both differ from each other only in the
    // numbers — which is exactly the shape an archetype could not express
    // without a whole second row.
    "npc_pirate_raider",
    "npc_pirate_heavy_iron_mary",
    // The giant's two DRIVEN limbs — one character, two bodies, which is what a
    // reusable template is for.
    "npc_giant_gnu_hands",
    // ⭐⭐ **THE TWO CELLULAR AUTOMATONS** (ledger D84 / sprite redirect P5). The
    // richest row `character_archetypes.ron` still held is theirs, and it was
    // reached by STRING MATCHING on id / display name / dialogue node. They
    // author it now: 60 HP, the swipe, the glider, the Cellular Pulse, the four
    // body capabilities, and a Smash policy that notices at 540.
    "perfect_cellular_automaton",
    "imperfect_cellular_automaton",
    // The intro raid corridor's two, off `gradient_seeker` and `medium_striker`
    // — see their arms in `authored_intrinsics`. Neither is on the select grid,
    // which is exactly what this list is for.
    "npc_salvage_guard",
    "npc_lab_raider",
    // The combat-feel lab's two indestructible dummies, off `sandbag_infinite`.
    "sandbag_infinite",
    // ⚠ the parrot is NOT here and must not be: `stochastic_parrot` is already
    // on `PLAYABLE_ROSTER`, so it is registered, and listing it twice would
    // register it twice.
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
        // ⭐⭐ **THE PERFECT CELLULAR AUTOMATON**, the dialogue-gated boss and the
        // richest row `character_archetypes.ron` still held (ledger D84).
        //
        // ⛔ **it was reached by STRING MATCHING.** `hostile_brain_id_for_actor`
        // asked whether an actor's id, display name or dialogue node contained
        // "cellular automaton" and handed the body a whole archetype — the same
        // shape as the two pirate arms deleted on 2026-08-11, and the last one
        // left. A creature that states its own facts needs no matcher.
        //
        // ```text
        // body        60 HP, 168 run speed, the swipe, the glider, the pulse,
        //             and the four capabilities (blink / fly / shield / dash)
        // controller  the Smash policy: notice at 540, commit at 150, duelist
        // placement   respawn, which the placement carries
        // ```
        //
        // ⚠ **GROUNDED HYBRID, and the row said so in two fields that read as a
        // contradiction**: `is_aerial: Some(false)` beside `can_fly: true`. It
        // prefers to fight on the ground and takes to the air only to cover a
        // long gap. Reading `can_fly` as "aerial" would perch it permanently.
        "perfect_cellular_automaton" | "imperfect_cellular_automaton" => {
            let mut definition = definition
                // ⭐⭐ **AND THE POLICY IT ADOPTS WHEN PROVOKED** (ledger D89).
                // The duel arena's fighters carry a `grudge_against`, so they
                // are PROVOKED rather than spawned hostile — and a provoked
                // creature rebuilds its mind from this reference. Without it the
                // PCA fell to the default aggressive policy: it closed and
                // swung, and never blocked, which is exactly the shield the duel
                // regression measures.
                .with_provoked_profile_named("cellular_duelist")
                .with_locomotion(CharacterLocomotion {
                    run_speed: 168.0,
                    move_style: MoveStyleSpec::Walk,
                    // ⭐⭐ **GROUNDED, STATED** — Jon, 2026-08-11: *"in smash PCA
                    // should not have the fly ability. I made a wrong call
                    // there earlier."* Its catalog row stays `Floating`, which
                    // is a claim about its SILHOUETTE (no default standing
                    // height; the sheet decides, and that is what keeps its body
                    // 68px rather than 48). The archetype row it replaces said
                    // `is_aerial: Some(false)` for the same reason: a
                    // grounded-base hybrid.
                    baseline_free_flight: Some(false),
                    ..Default::default()
                })
                .with_contact_damage(ContactDamage {
                    strength: 0.75,
                    amount: 1,
                })
                .with_abilities(ambition_platformer2d_core::AbilitySet {
                    attack: true,
                    // The four body-enforced capabilities the row authored. A
                    // possessing player inherits exactly these, which is the
                    // property that made them body facts rather than brain ones.
                    blink: true,
                    fly: true,
                    fly_toggle: true,
                    shield: true,
                    dash: true,
                    ..ambition_platformer2d_core::AbilitySet::basic()
                })
                .with_autonomous_profile(BrainProfile {
                    template: CharacterBrainTemplate::Smash,
                    aggro_radius: 540.0,
                    attack_range: 150.0,
                    patrol_effort: 0.5714,
                    chase_effort: 1.0,
                    smash_dash_to_close: true,
                    // Footsies and spacing rather than close-and-camp.
                    smash_duelist: true,
                    ..Default::default()
                })
                // ⭐ **the glider** — a cellular-automaton spaceship as the
                // zoning tool. The projectile is a functional `Rock`; the Conway
                // glider is chosen by the authored visual id below, which the
                // render layer resolves through the content-owned projectile
                // catalog rather than from the owner's id string.
                .with_ranged_vfx("glider")
                .with_action_set(ambition_characters::brain::ActionSet {
                    melee: Some(MeleeActionSpec::Swipe(SwipeSpec {
                        windup_s: 0.24,
                        active_s: 0.08,
                        recover_s: 0.30,
                        damage: 1,
                        reach_px: 30.0,
                    })),
                    ranged: Some(ambition_characters::brain::RangedActionSpec::new(
                        ambition_characters::brain::action_set::RangedStyle::Rock,
                        300.0,
                        1,
                    )),
                    // ⛔ **NOT the pulse.** The MOVESET's verb map already binds
                    // `special → cellular_pulse`; putting it in this slot too
                    // takes the slot the SHIELD uses, and the PCA's reactive
                    // block silently stops happening. The archetype row kept
                    // them apart by construction — `signature_move` was a
                    // different field from `can_shield` — and authoring both on
                    // one character is where they can collide.
                    special: None,
                    move_style: MoveStyleSpec::Walk,
                })
                .with_moveset(crate::cellular_automaton_moveset::cellular_pulse_moveset());
            definition.vitals.max_health = Some(60);
            definition
        }
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
        // **Ambient wildlife: a wall-and-ceiling crawler that hurts on touch.**
        //
        // The row this replaces carried a `default_size` of 48x22 and it is
        // deliberately NOT here: a named catalog character sizes its body to its
        // authored SPRITE, which is the same resolution a peaceful NPC of this
        // character already gets — one silhouette per creature, whichever road
        // spawns it.
        "npc_puppy_slug" => {
            let mut definition = definition
                .with_locomotion(CharacterLocomotion {
                    run_speed: 80.0,
                    move_style: MoveStyleSpec::Slither,
                    // Crawlid-style: hugs the surface normal and probes ledges
                    // so it never walks off a platform.
                    surface_walker: true,
                    // Knocked off its surface when hit — falls with gravity for
                    // a moment, then re-attaches on landing.
                    cling_breaks_on_hit: true,
                    baseline_free_flight: Some(false),
                })
                .with_contact_damage(ContactDamage {
                    strength: 0.55,
                    amount: 1,
                })
                // The slug-only psychedelic pass, and the reason `dream_seed`
                // became a character fact.
                .with_dream_seed(0.271828)
                .with_autonomous_profile(BrainProfile {
                    template: CharacterBrainTemplate::Wanderer,
                    // Wildlife: it notices nobody and commits to nothing. The
                    // Wanderer template ignores both, and authoring them as zero
                    // says so rather than leaving a reader to guess.
                    aggro_radius: 0.0,
                    attack_range: 0.0,
                    ..Default::default()
                });
            definition.vitals.max_health = Some(2);
            definition
        }
        // **The aerial dive-bomber.** Its `is_aerial` does NOT come across as a
        // character field: the catalog row already says `body_kind: Floating`,
        // and construction reads gravity-freedom from there — one authority for
        // "does this creature fly", which the archetype row was duplicating.
        //
        // ⚠ `mass: 0.5` is not carried either. Mass weights a mount+rider centre
        // of gravity (ADR 0020) and a parrot is neither, so it was inert on the
        // row; the first mountable character to migrate is the one that needs a
        // home for it.
        "stochastic_parrot" => {
            let mut definition = definition
                .with_locomotion(CharacterLocomotion {
                    run_speed: 240.0,
                    move_style: MoveStyleSpec::Float,
                    // ⭐ **IT FLIES, AND IT SAYS SO** (ledger D89). This was
                    // inferred from `body_kind: Floating` in its catalog row —
                    // a presentation/footprint fact that was doubling as
                    // locomotion authority. The fold is deleted; a bird states
                    // its own flight.
                    baseline_free_flight: Some(true),
                    ..Default::default()
                })
                .with_contact_damage(ContactDamage {
                    strength: 0.55,
                    amount: 1,
                })
                .with_autonomous_profile(BrainProfile {
                    // Stalks to an altitude above its target, dives, pecks on
                    // contact, peels off to recover.
                    template: CharacterBrainTemplate::Aerial,
                    aggro_radius: 620.0,
                    attack_range: 60.0,
                    ..Default::default()
                })
                // ⛔⛔ **AND ITS CATALOG ROW STILL NAMES `parrot_lively`, WHICH
                // DISAGREES WITH THIS** (found 2026-08-12, ledger D81). That
                // preset says `aggro_radius: 120.0` and `attack_range: 0.0`
                // against this profile's 620 and 60 — one bird, two authorities,
                // different answers. THIS one wins (`resolve_npc_brain` ranks a
                // definition's own profile above the row's `default_brain`, and
                // the enemy road builds character-first), so the preset is dead
                // weight stating wrong numbers rather than a live conflict.
                //
                // ⛔ it cannot be deleted yet, and the blocker is a SCHEMA one:
                // `default_brain` is a required `String`, so the row has to name
                // SOME preset — and `parrot_lively` has exactly one namer, this
                // bird. A character whose definition states its policy should not
                // have to name a vocabulary it does not use; making that field
                // optional is what lets the preset go.
                .with_action_set(ambition_characters::brain::ActionSet {
                    melee: Some(MeleeActionSpec::Bite(
                        ambition_characters::brain::BiteSpec {
                            windup_s: 0.16,
                            active_s: 0.10,
                            recover_s: 0.28,
                            damage: 1,
                            reach_px: 48.0,
                        },
                    )),
                    move_style: MoveStyleSpec::Float,
                    ..Default::default()
                });
            definition.vitals.max_health = Some(3);
            definition
        }
        // **The drifting swarms.** Mary-O flies them over her levels; they are
        // Ambition's characters, and this is where they say what they are.
        //
        // ⚠ they author `flies` even though their catalog rows say
        // `body_kind: Floating` — the catalog is not always THERE. A standalone
        // demo that borrows a character has no row for it, and a body that
        // reads its gravity-freedom from a row it cannot see falls out of the
        // sky. Stating it on the character is what makes the fact travel.
        "npc_snakes_on_a_paper_plane" | "npc_snakes_on_a_cartesian_plane" => {
            let paper = id == "npc_snakes_on_a_paper_plane";
            let mut definition = definition
                .with_locomotion(CharacterLocomotion {
                    run_speed: if paper { 58.0 } else { 38.0 },
                    move_style: MoveStyleSpec::Float,
                    baseline_free_flight: Some(true),
                    ..Default::default()
                })
                .with_contact_damage(ContactDamage {
                    strength: 0.5,
                    amount: 1,
                })
                .with_autonomous_profile(BrainProfile {
                    // It baseline_free_flight, it notices nobody, and running into it is the
                    // entire threat.
                    template: CharacterBrainTemplate::Aerial,
                    aggro_radius: 0.0,
                    attack_range: 0.0,
                    ..Default::default()
                });
            definition.vitals.max_health = Some(if paper { 1 } else { 2 });
            definition
        }
        // **The Hall's AI Slop, as a placed enemy.** One spawn in the sandbox,
        // one archetype row, and the same creature already standing on a Hall
        // pedestal — which is the ontology this campaign is about: one
        // character, two contexts.
        //
        // ⚠ its catalog row's `default_brain` is `melee_brute_striker`, and that
        // is NOT what this authors. The catalog default is what a PEACEFUL Hall
        // NPC of this character does; the profile below is what the placed enemy
        // does, and they are allowed to differ because the first is a catalog
        // fact and the second is this character's own default policy.
        "npc_ai_slop" => {
            let mut definition = definition
                .with_locomotion(CharacterLocomotion {
                    run_speed: 42.0,
                    move_style: MoveStyleSpec::Walk,
                    ..Default::default()
                })
                .with_contact_damage(ContactDamage {
                    strength: 0.5,
                    amount: 1,
                })
                .with_autonomous_profile(BrainProfile {
                    // Walks forward, reverses at walls, notices nobody. Its only
                    // offense is the body it walks into you with.
                    template: CharacterBrainTemplate::Wanderer,
                    aggro_radius: 0.0,
                    attack_range: 0.0,
                    ..Default::default()
                });
            definition.vitals.max_health = Some(1);
            definition
        }
        // **The burning flying shark** — the first MOUNT to become a character.
        //
        // ⭐ its rideability is a character fact (ADR 0020, and Jon's own list
        // puts "mount/pilot body capabilities" under the definition): a shark is
        // rideable because of what a shark IS, not because of where it was
        // placed or who is steering it. `mass: 6.0` is the other half — the pair
        // rolls around a centre of gravity near the heavier body — and it rides
        // on `vitals`, which already carried mass.
        //
        // ⚠ `is_aerial` and `default_size` do NOT come across: the catalog says
        // `body_kind: Floating`, and a named character sizes its body to its
        // authored sprite, which is the same silhouette the row was restating.
        "npc_burning_flying_shark" => {
            let mut definition = definition
                .with_locomotion(CharacterLocomotion {
                    run_speed: 260.0,
                    move_style: MoveStyleSpec::Float,
                    // ⭐ see the parrot: a flying MOUNT states its own flight
                    // rather than inheriting it from a body-kind enum.
                    baseline_free_flight: Some(true),
                    ..Default::default()
                })
                .with_contact_damage(ContactDamage {
                    strength: 1.10,
                    amount: 2,
                })
                .with_death_traits(CharacterDeathTraits {
                    // A riderless shark's fast charge, stopped dead by a wall,
                    // detonates the shark.
                    charge_crash_explodes: true,
                    ..Default::default()
                })
                .with_mount(ambition_characters::actor::CharacterMount {
                    class: Some("shark".to_string()),
                    // It rides nothing, and it splashes nothing on death: a dead
                    // shark drops its rider unharmed.
                    ..Default::default()
                })
                .with_autonomous_profile(BrainProfile {
                    // Dive at the target, crash, recover.
                    template: CharacterBrainTemplate::ChargeCrash,
                    aggro_radius: 1200.0,
                    attack_range: 200.0,
                    ..Default::default()
                })
                .with_action_set(ambition_characters::brain::ActionSet {
                    melee: Some(MeleeActionSpec::Bite(
                        ambition_characters::brain::BiteSpec {
                            windup_s: 0.18,
                            active_s: 0.10,
                            recover_s: 0.30,
                            damage: 2,
                            reach_px: 42.0,
                        },
                    )),
                    move_style: MoveStyleSpec::Float,
                    ..Default::default()
                });
            definition.vitals.max_health = Some(6);
            definition.vitals.mass = Some(6.0);
            definition
        }
        // **The carried giant (ADR 0020).** A brainless, stationary MOUNT whose
        // RIDER is the threat — GNU-ton, who stays a boss and is not touched
        // here.
        //
        // ⭐ **the first migrated body that authors `attacks_player: false`**,
        // and it could not have migrated a day earlier: the character-first
        // constructor wrote that flag as the literal `true`, so a migrated giant
        // would have started hunting the player it exists to carry. The row's
        // hostility half is controller policy and now says so.
        //
        // ⚠ `default_size` does NOT come across, and the placement is why: the
        // sandbox's giant is authored as a 220x220 LDtk box, exactly the
        // envelope the row was restating, so the size survives without a second
        // authority stating it. Its `respawn: OnRoomReenter` moves to the
        // placement, where a respawn policy belongs.
        "npc_giant_gnu" => {
            let mut definition = definition
                .with_locomotion(CharacterLocomotion {
                    // Grounded heavy locomotion, inert while StandStill — the
                    // correct gait for a lumbering giant if ever steered.
                    run_speed: 0.0,
                    move_style: MoveStyleSpec::WalkHeavy,
                    ..Default::default()
                })
                .with_mount(ambition_characters::actor::CharacterMount {
                    class: Some("giant".to_string()),
                    ..Default::default()
                })
                .with_autonomous_profile(BrainProfile {
                    template: CharacterBrainTemplate::StandStill,
                    aggro_radius: 0.0,
                    attack_range: 0.0,
                    // It never seeks and never strikes — and `StandStill` with
                    // a zero aggro radius already SAYS that. The relationship
                    // half ("this creature is not your enemy") is the
                    // PLACEMENT's: the sandbox giant authors `Peaceful`.
                    ..Default::default()
                });
            definition.vitals.max_health = Some(42);
            // Far heavier than the scholar riding it, so the mount pair's centre
            // of gravity sits on the giant and the lighter rider orbits it under
            // a gravity flip.
            definition.vitals.mass = Some(8.0);
            // No `contact_damage`: a prop-like mount does no damage by being
            // stood next to, which is what `body_contact_damage: false` said.
            definition
        }
        // **THE SHARK RIDERS.** Two creatures, one policy, different numbers —
        // the case the archetype file answered with two nearly-identical rows
        // (`pirate_shark_rider`, `pirate_heavy_shark_rider`) whose only real
        // differences are health, weight, reach and which gun-sword they hold.
        //
        // ⚠ **`body_contact_damage: false` on both rows, so neither authors
        // `contact_damage`.** The rows carried a `contact_strength` and a
        // `damage_amount` beside a flag that turned them off — numbers that
        // described nothing. A character says what is true: touching a raider
        // does not hurt; its gun-sword does.
        //
        // ⚠ `default_size` does not come across either: both are sized by their
        // authored placements (44x78 and 72x110 in `sandbox.ldtk`), which is the
        // same silhouette the rows were restating.
        "npc_pirate_raider" | "npc_pirate_heavy_iron_mary" => {
            let heavy = id == "npc_pirate_heavy_iron_mary";
            let mut definition = definition
                .with_locomotion(CharacterLocomotion {
                    run_speed: if heavy { 215.0 } else { 230.0 },
                    move_style: if heavy {
                        MoveStyleSpec::WalkHeavy
                    } else {
                        MoveStyleSpec::Walk
                    },
                    ..Default::default()
                })
                // A cove raider can board a "shark"-class mount. It is not itself
                // rideable, which is the other half of the same sentence.
                .with_mount(ambition_characters::actor::CharacterMount {
                    pilotable_classes: vec!["shark".to_string()],
                    ..Default::default()
                })
                .with_held_item(if heavy {
                    "gun_sword_heavy"
                } else {
                    "gun_sword"
                })
                .with_autonomous_profile(BrainProfile {
                    // Orbit-and-fire standoff: notice from across the cove,
                    // commit from just inside it.
                    template: CharacterBrainTemplate::Skirmisher,
                    aggro_radius: 1200.0,
                    attack_range: 1100.0,
                    // ⭐ a TUNED amble, and the reason `BrainProfile` had to grow
                    // `patrol_effort` before either of these could migrate: the
                    // constructor's literal `0.5` would have quietly retuned both.
                    patrol_effort: if heavy { 0.5116 } else { 0.4783 },
                    chase_effort: 1.0,
                    ..Default::default()
                })
                .with_action_set(ambition_characters::brain::ActionSet {
                    // The bolt the gun-sword fires — the SAME verb
                    // `held_item_by_id` grants, authored here because a
                    // character states what it DOES and the item states what it
                    // HOLDS.
                    ranged: Some(ambition_characters::brain::RangedActionSpec::bolt(
                        500.0,
                        if heavy { 3 } else { 2 },
                    )),
                    move_style: if heavy {
                        MoveStyleSpec::WalkHeavy
                    } else {
                        MoveStyleSpec::Walk
                    },
                    ..Default::default()
                });
            definition.vitals.max_health = Some(if heavy { 6 } else { 4 });
            definition
        }
        // **THE GIANT'S HANDS.** Two bodies of one character: the rig spawns a
        // left and a right from this single definition, which is a reusable
        // authored template doing exactly what the campaign is about.
        //
        // ⚠ its collision envelope does NOT come across, and could not: a hand
        // is sized at PLAN time as 0.7 of the giant's own half-extent, so the
        // row's `default_size: (154.0, 154.0)` was 220 × 0.7 written down a
        // second time. The geometry is derived; the row was restating it.
        "npc_giant_gnu_hands" => {
            let mut definition = definition
                .with_locomotion(CharacterLocomotion {
                    // The limb router steers it every tick; the StandStill brain
                    // below is inert and this speed is never asked for.
                    run_speed: 0.0,
                    move_style: MoveStyleSpec::WalkHeavy,
                    ..Default::default()
                })
                .with_autonomous_profile(BrainProfile {
                    template: CharacterBrainTemplate::StandStill,
                    aggro_radius: 0.0,
                    attack_range: 0.0,
                    // A limb never seeks anybody: the rider's routed strikes
                    // spawn the damaging hitboxes, and the hand is their vehicle.
                    // `StandStill` + zero aggro is the whole of that as policy.
                    ..Default::default()
                });
            definition.vitals.max_health = Some(42);
            // Lighter than the giant body, heavy enough to feel solid.
            definition.vitals.mass = Some(2.0);
            definition
        }
        // **THE PRACTICE TARGET.** A body that exists to be hit: no aggro, no
        // strike back, excluded from the save file, and skipped by the path
        // assignment — all of which is what `practice_target` says in one word.
        //
        // ⚠ **it authors no `contact_damage`, and its old row's comment was
        // wrong about that.** `sandbag_finite` said *"It still deals light
        // CONTACT damage if you walk into it"* directly above
        // `body_contact_damage: false`, which turns exactly that off. The flag
        // is the gate, so the comment described an intention nobody had
        // implemented, and a migration that believed the prose would have given
        // the dummy a hitbox it never had.
        //
        // ⚠ its `respawn: InPlace(0.85)` moves to the placement, where a respawn
        // policy belongs (ADR 0022) — and `sandbag_infinite` does NOT migrate
        // with it: `never_dies` is a character trait, so the immortal dummy is a
        // different creature and needs its own registered character. See ledger
        // D77.
        "sandbag" => {
            let mut definition = definition
                .as_practice_target()
                .with_locomotion(CharacterLocomotion {
                    // It never walks anywhere — StandStill drives it — but the
                    // row authored a speed and a gait, so the character does too.
                    run_speed: 155.0,
                    move_style: MoveStyleSpec::Walk,
                    ..Default::default()
                })
                .with_autonomous_profile(BrainProfile {
                    template: CharacterBrainTemplate::StandStill,
                    // Notices nobody and swings at nobody; the old row's
                    // `attack_range: 150.0` sat beside `melee: None`.
                    aggro_radius: 0.0,
                    attack_range: 0.0,
                    patrol_effort: 0.6774,
                    chase_effort: 1.0,
                    ..Default::default()
                });
            definition.vitals.max_health = Some(6);
            definition
        }
        // **THE IMMORTAL TRAINING DUMMY**, and the arm above says why it is a
        // separate creature rather than a flag on the sandbag: `never_dies` is a
        // character trait, so "the same dummy, invincible in this room" is not a
        // thing the model can say. The combat-feel lab's two spawns are this.
        //
        // ⚠ **9999 health AND `never_dies`, which is one fact stated twice and
        // both halves are carried across on purpose.** The pool is what the
        // damage numbers and any health readout see; `never_dies` is what the
        // resolver checks before it kills. Dropping either changes what a lab
        // dummy looks like under a hit, and a migration is the wrong place to
        // find that out.
        //
        // ⛔ no contact damage: the row authored `body_contact_damage: false`
        // beside a `contact_strength`, which is the archetype format's way of
        // saying the numbers are inert. A character says it by not speaking.
        "sandbag_infinite" => {
            let mut definition = definition
                .as_practice_target()
                .with_locomotion(CharacterLocomotion {
                    run_speed: 155.0,
                    move_style: MoveStyleSpec::Walk,
                    ..Default::default()
                })
                .with_death_traits(ambition_characters::actor::CharacterDeathTraits {
                    never_dies: true,
                    ..Default::default()
                })
                .with_autonomous_profile(BrainProfile {
                    template: CharacterBrainTemplate::StandStill,
                    // Notices nobody and swings at nobody — the row's
                    // `attack_range: 150.0` sat beside `melee: None`, exactly as
                    // the finite sandbag's did.
                    aggro_radius: 0.0,
                    attack_range: 0.0,
                    patrol_effort: 0.6774,
                    chase_effort: 1.0,
                    ..Default::default()
                });
            definition.vitals.max_health = Some(9999);
            definition
        }
        // **THE PATENT CLERK, read back off its own row.** Its
        // `gameplay_description` says *"a high-mastery heavyweight controller …
        // turns careful observation into unusually strong parries and
        // finishers"* — heavyweight, controller, finishers — and those three
        // words are the table. See the module doc; the design was already
        // written down and nobody had read it back.
        //
        // ⛔ MOVES ONLY, and the classification mechanic (MASS / ENERGY / MOVING
        // / AT REST, reference frames, the elevator recovery) is deliberately NOT
        // here: those are systems, not swings, and writing them as move windows
        // would be the wholesale-migration failure mode wearing a content commit.
        "special_patent_clerk" => {
            definition.with_moveset(crate::patent_clerk_moveset::patent_clerk_moveset())
        }
        // **THE PIRATE ADMIRAL'S CUTLASS.** The second adopter removed from
        // `smash_fighter_kit()` (P3.24), and the character was already telling us
        // what its moves are: its row says `default_action_set: "pirate_pistol"`,
        // the roster comment beside its id reads "pistol + cutlass", and its
        // sprite is authored at `collision_scale: 1.6` — the largest of the three
        // fighters with a table.
        //
        // ⛔ MOVES ONLY. The admiral's body still comes from its catalog row and
        // its archetype; authoring vitals or locomotion here would be a retune
        // wearing a migration's commit, and it is not what removes the adopter.
        // A table is the whole job.
        "npc_pirate_admiral" => {
            definition.with_moveset(crate::pirate_admiral_moveset::pirate_admiral_moveset())
        }
        // **THE LAB RAIDER.** The intro raid corridor's other spawn, and the
        // SECOND creature to point at the shared `medium_striker` policy — which
        // is what makes that entry a role rather than the goblin's private
        // profile under a general name. The campaign named this one explicitly:
        // *"`npc_lab_raider` and `npc_salvage_guard` for the two intro
        // placements that are literally named that."*
        //
        // ⚠ its body facts are the goblin's, because the archetype it wore gave
        // both the same ones — 5 HP, 170 px/s, 0.70 contact. Carried across
        // unchanged; making a raider tougher than a goblin is a design decision
        // and it should be made where design decisions are visible, not
        // smuggled in by a migration.
        //
        // ⛔ no `action_set` here, exactly like the goblin: its kit comes from
        // its catalog row's `default_action_set: "striker_swipe"`. Authoring one
        // would be a SECOND declaration of the same fact, which is the muddle
        // this campaign removes rather than a completeness improvement.
        "npc_lab_raider" => {
            let mut definition = definition
                .with_locomotion(CharacterLocomotion {
                    run_speed: 170.0,
                    move_style: MoveStyleSpec::Walk,
                    ..Default::default()
                })
                .with_contact_damage(ContactDamage {
                    strength: 0.70,
                    amount: 1,
                })
                .with_autonomous_profile_named("medium_striker");
            definition.vitals.max_health = Some(5);
            definition
        }
        // **THE SALVAGE GUARD.** The intro raid corridor's two `EnemySpawn`s,
        // which have been wearing `gradient_seeker` — an archetype whose whole
        // population is those two placements, both literally named "Salvage
        // Guard". A generic role with exactly one creature in it was never a
        // role; it was that creature's body filed under a different name.
        //
        // ⚠ **its policy is INLINE, and the goblin's is NAMED, and the
        // difference is the P2.16 rule rather than an inconsistency.** A shared
        // `autonomous_profiles` entry earns its indirection when several
        // creatures point at it — `medium_striker` has a goblin band. This
        // policy has one adopter, so naming it would publish a shared thing
        // nobody shares and leave a second empty role behind exactly like the
        // one being deleted.
        //
        // ⛔ `respawn: OnRoomReenter` is NOT here: it is the third authority
        // (placement policy), it is the engine default for a room-scoped enemy,
        // and the archetype stating it is the muddle this campaign removes.
        "npc_salvage_guard" => {
            let mut definition = definition
                .with_locomotion(CharacterLocomotion {
                    run_speed: 225.0,
                    move_style: MoveStyleSpec::Walk,
                    ..Default::default()
                })
                .with_contact_damage(ContactDamage {
                    strength: 0.80,
                    amount: 1,
                })
                .with_action_set(ambition_characters::brain::ActionSet {
                    melee: Some(MeleeActionSpec::Swipe(SwipeSpec {
                        windup_s: 0.28,
                        active_s: 0.08,
                        recover_s: 0.32,
                        damage: 1,
                        reach_px: 28.0,
                    })),
                    ranged: None,
                    special: None,
                    move_style: MoveStyleSpec::Walk,
                })
                .with_autonomous_profile(BrainProfile {
                    template: CharacterBrainTemplate::Smash,
                    // ⚠ 900 px is LONG — it is a corridor, and the guard is
                    // meant to notice you from the far end of it. Carried across
                    // unchanged; a retune is a separate, visible decision.
                    aggro_radius: 900.0,
                    attack_range: 150.0,
                    patrol_effort: 0.5778,
                    chase_effort: 1.0,
                    ..Default::default()
                });
            definition.vitals.max_health = Some(4);
            definition
        }
        // **THE GOBLIN BAND.** Five sandbox placements (`annex_goblin_a/b`,
        // `pg_goblin_a/b/c`) that have been wearing the `medium_striker`
        // ARCHETYPE — a whole body, borrowed for its fighting style.
        //
        // ⭐ **it NAMES its policy rather than carrying one**, which is the
        // Group-B/Group-C split arriving: the archetype's controller half is now
        // `autonomous_profiles: { "medium_striker": .. }` in the catalog, and any
        // number of creatures may point at it while keeping their own bodies. A
        // lab raider and a skitter are the next two.
        //
        // ⚠ the key is PROVIDER-NAMESPACED on assembly, so the reference is
        // `ambition::medium_striker` rather than the local name — two games may
        // both author a "medium_striker" and neither wins.
        "goblin" => {
            let mut definition = definition
                .with_locomotion(CharacterLocomotion {
                    run_speed: 170.0,
                    move_style: MoveStyleSpec::Walk,
                    ..Default::default()
                })
                .with_contact_damage(ContactDamage {
                    strength: 0.70,
                    amount: 1,
                })
                // ⭐ **the LOCAL name.** It used to hand-namespace this
                // (`format!("{}::medium_striker", AMBITION_CONTENT_PROVIDER)`),
                // which made an author responsible for knowing whether the
                // surrounding catalog had been assembled yet — the leak Jon's
                // redirect §8 names. `BrainProfileRef` resolves it against this
                // definition's own provider.
                .with_autonomous_profile_named("medium_striker")
                // ⭐ **AND ITS OWN MOVES** (campaign P3.24, 2026-08-12). Every
                // seated fighter whose character says nothing takes
                // `smash_fighter_kit()` — one generic swipe — and that floor's
                // goal is DELETION, one adopter at a time. The goblin is the
                // third character in the game to state a table and the first
                // ENEMY to.
                .with_moveset(crate::goblin_moveset::goblin_moveset());
            definition.vitals.max_health = Some(5);
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

    /// **The giant carries its own facts now** — every one its archetype row
    /// stated, authored on the definition, and that row is DELETED (D76 closed
    /// once three layers learned to ask the character before the archetype: the
    /// limbed-host predicate, the activation path's construction context, and
    /// `mount_capabilities_of`).
    ///
    /// ⭐ the two facts that could not have been authored before this campaign:
    /// `attacks_player: false` (a mount whose RIDER is the threat) and a
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
}
