//! Named Ambition boss content registration.
//!
//! Owns Ambition's immutable App-local boss fragment and the live
//! [`BossEncounterRegistry`] resource used by an active session. The general
//! boss machinery (profiles, specs, encounter registry/system, patterns) still
//! lives in `ambition_boss_encounter`; this module owns the bespoke per-boss
//! *behavior* and *bark content* that names individual bosses:

use ambition_platformer2d_shared_tangle::schedule::GameplayGated;
use ambition_platformer2d_shared_tangle::schedule::SimScheduleExt;
use bevy::prelude::*;

pub mod banter;
pub mod cut_rope;
pub mod gnu_ton;
pub mod specials;
#[cfg(feature = "ui")]
pub mod yarn;

pub use banter::{install_boss_banter, tick_boss_idle_barks};
pub use cut_rope::{
    detect_cut_rope_rope_cut, emit_cut_rope_room_replay_after_the_conversation_ends,
    is_cut_rope_boss, reset_cut_rope_attempt_on_replay, reset_cut_rope_boss_arena_on_room_reset,
    reset_cut_rope_boss_attempt, setup_cut_rope_encounter, spawn_cut_rope_victory_npc,
    sync_cut_rope_boss_arena_prop_visuals, tick_cut_rope_flavor, CutRopeBossArenaState,
    CutRopeHeavyObjectCycle, CutRopeRoomReplayRequested, PendingCutRopeRoomReplay,
    SmirkingBehemothVictoryNpc, CUT_ROPE_BOSS_ID, CUT_ROPE_VICTORY_NPC_DIALOGUE_ID,
    CUT_ROPE_VICTORY_NPC_ID,
};
pub use gnu_ton::gate_gnu_ton_arena_ladder;

pub const BOSS_PROFILES_RON: &str = include_str!("../../assets/data/boss_profiles.ron");

pub const BOSS_SEEDS_RON: &str = include_str!("../../assets/data/boss_seeds.ron");

/// BD5's per-game fairness calibration (`boss-design.md` §3). One RON per game, so
/// re-calibrating a fight's fairness is an edit, not a recompile.
pub const BOSS_VALIDATOR_BANDS_RON: &str =
    include_str!("../../assets/data/boss_validator_bands.ron");

/// The validator bands the fight validator judges against.
pub fn validator_bands() -> &'static ambition_boss_encounter::pattern::validator::ValidatorBands {
    ambition_boss_encounter::pattern::content_schema::lowered_validator_bands(
        crate::pack::prepared(),
    )
    .expect("the bands schema lowers its calibration for every pack that compiles")
}

/// The boss seed library.
///
/// Same move as the bands: the compiler's lowered artifact, not a re-parse.
/// The schema refuses an inverted duration band (which matches nothing, so every
/// instance silently falls outside it) and an attack claimed by two seeds.
pub fn seed_library() -> &'static ambition_boss_encounter::pattern::seeds::SeedLibrary {
    ambition_boss_encounter::pattern::content_schema::lowered_seed_library(crate::pack::prepared())
        .expect("the seed schema lowers its library for every pack that compiles")
}

/// Embedded encounter rows contributed by the Ambition provider: each source's
/// path and its bytes, together, once.
///
/// Reordering either list attached a file's contents to the wrong diagnostic path and fingerprint
/// while the runtime went on resolving rows by their internal ids — so nothing would fail, and a
/// compile error would point at the wrong file forever.
///
/// positional coupling across two files is the shape, not the count. It
/// was maintainable at nine and would have been silently wrong at ten.
pub const BOSS_ENCOUNTERS: &[(&str, &str)] = &[
    (
        "data/boss_encounters/clockwork_warden.ron",
        include_str!("../../assets/data/boss_encounters/clockwork_warden.ron"),
    ),
    (
        "data/boss_encounters/mockingbird.ron",
        include_str!("../../assets/data/boss_encounters/mockingbird.ron"),
    ),
    (
        "data/boss_encounters/gnu_ton_rider.ron",
        include_str!("../../assets/data/boss_encounters/gnu_ton_rider.ron"),
    ),
    (
        "data/boss_encounters/smirking_behemoth_boss.ron",
        include_str!("../../assets/data/boss_encounters/smirking_behemoth_boss.ron"),
    ),
    (
        "data/boss_encounters/flying_spaghetti_monster_boss.ron",
        include_str!("../../assets/data/boss_encounters/flying_spaghetti_monster_boss.ron"),
    ),
    (
        "data/boss_encounters/trex_boss.ron",
        include_str!("../../assets/data/boss_encounters/trex_boss.ron"),
    ),
    (
        "data/boss_encounters/mode_collapse_boss.ron",
        include_str!("../../assets/data/boss_encounters/mode_collapse_boss.ron"),
    ),
    (
        "data/boss_encounters/exploding_gradient_boss.ron",
        include_str!("../../assets/data/boss_encounters/exploding_gradient_boss.ron"),
    ),
    (
        "data/boss_encounters/overflow_boss.ron",
        include_str!("../../assets/data/boss_encounters/overflow_boss.ron"),
    ),
];

// `boss_encounter_rons()` — "just the bytes, for the catalog builder" — is
// GONE. It existed to hand nine raw files to a builder that parsed
// them itself; the builder now takes the compiler's merged book, so the only
// consumer of those bytes is the pack. [`BOSS_ENCOUNTERS`] still carries them,
// because the pack is where they belong.

fn boss_sprite_filenames() -> std::collections::BTreeMap<String, String> {
    std::collections::BTreeMap::from([
        ("gradient_sentinel".into(), "boss_spritesheet.png".into()),
        (
            "mockingbird".into(),
            "mockingbird_boss/mockingbird_boss_spritesheet.png".into(),
        ),
        (
            "smirking_behemoth_boss".into(),
            "smirking_behemoth_boss_spritesheet.png".into(),
        ),
        (
            "giant_gnu".into(),
            "gnu_ton_boss/giant_gnu_spritesheet.png".into(),
        ),
        (
            "gnu_ton_rider".into(),
            "gnu_ton_boss/gnu_ton_rider_spritesheet.png".into(),
        ),
        (
            "flying_spaghetti_monster_boss".into(),
            "flying_spaghetti_monster_boss_spritesheet.png".into(),
        ),
        ("trex_boss".into(), "trex_enemy_spritesheet.png".into()),
    ])
}

fn special_animation_keys() -> std::collections::BTreeMap<String, Vec<String>> {
    std::collections::BTreeMap::from([
        (
            "overfit_volley".into(),
            vec!["spike_halo".into(), "eye_beam".into()],
        ),
        (
            "eye_beam".into(),
            vec!["eye_beam".into(), "spike_halo".into()],
        ),
        ("minima_trap".into(), vec!["spike_halo".into()]),
        ("saddle_point".into(), vec!["spike_halo".into()]),
        ("gradient_cascade".into(), vec!["spike_halo".into()]),
        ("mode_collapse_converge".into(), vec!["spike_halo".into()]),
        ("gradient_nova".into(), vec!["spike_halo".into()]),
        ("overflow_flood".into(), vec!["spike_halo".into()]),
        (
            "seismic_stomp".into(),
            vec!["floor_slam".into(), "spike_halo".into()],
        ),
        (
            "echo_fan".into(),
            vec!["spike_halo".into(), "eye_beam".into()],
        ),
    ])
}

/// Ambition's immutable App-local boss contribution.
///
/// NOT all of this is compiled content, and the pack fingerprint does not
/// cover the rest. In the pack: the roster (`boss_profiles.ron`), the nine
/// encounter files, the seed library and the calibration. NOT in the pack, and
/// therefore changeable without moving the fingerprint:
///
/// * `boss_sheets.ron` — `BossSheetSpec` lives in `ambition_sprite_sheet`, which
///   pulls `bevy_render`; migrating it needs the same placement analysis the
///   profile vocabulary got, not just a handler;
/// * [`boss_sprite_filenames`] and [`special_animation_keys`] — content authored
///   as Rust `BTreeMap`s. They have to become authored DATA before a schema can
///   own them.
///
/// Stated here rather than left implied, because "the boss content goes through
/// the compiler" is the kind of half-true claim this whole effort exists to stop
/// making.
pub fn boss_catalog_fragment() -> ambition_boss_encounter::BossCatalogFragment {
    // The schema additionally refuses what `from_ron` accepted: a row whose `id`
    // disagrees with its map KEY. Every runtime path looks a boss up by key and
    // then reads `profile.id` for its sheet, music and barks — so a mismatch
    // means the boss is found under one name and draws as another, with each
    // half individually valid and nothing reporting it.
    let behaviors = ambition_boss_encounter::pattern::content_schema::lowered_boss_profiles(
        crate::pack::prepared(),
    )
    .expect("the boss-profile schema lowers its roster for every pack that compiles")
    .clone();
    let encounters = ambition_boss_encounter::pattern::content_schema::lowered_boss_encounters(
        crate::pack::prepared(),
    )
    .expect("the encounter schema merges its nine files for every pack that compiles")
    .clone();
    ambition_boss_encounter::BossCatalogFragment::from_prepared(
        crate::AMBITION_CONTENT_PROVIDER,
        Some("clockwork_warden"),
        Some("gradient_sentinel"),
        behaviors,
        encounters,
        include_str!("../../assets/data/boss_sheets.ron"),
        boss_sprite_filenames(),
        special_animation_keys(),
    )
    .expect("Ambition boss content should form one valid catalog fragment")
}

/// Assemble Ambition's boss catalog without constructing a Bevy App.
///
/// Pure content tests use this helper so they exercise the same provider
/// fragment as production composition rather than installing process state.
pub fn authored_boss_catalog() -> ambition_boss_encounter::BossCatalog {
    let mut registry = ambition_boss_encounter::BossCatalogRegistry::default();
    registry
        .register(boss_catalog_fragment())
        .expect("Ambition boss fragment should register");
    registry
        .assemble()
        .expect("Ambition boss fragment should assemble")
}

/// Contribute Ambition's immutable boss fragment to one Bevy App.
///
/// Registration is idempotent for the same provider payload, so a host may
/// call this before building its asset catalog and later add
/// [`AmbitionBossContentPlugin`] without coordinating install order.
pub fn register(app: &mut App) {
    use ambition_boss_encounter::BossCatalogAppExt as _;
    app.register_boss_catalog_fragment(boss_catalog_fragment());
}

/// Registers Ambition's boss fragment, initializes the live encounter
/// registry resource, and installs the cut-rope Yarn vocabulary + mirror feed.
pub struct AmbitionBossContentPlugin;

pub fn register_rollback_state(
    registrar: &mut impl ambition_platformer2d_core::snapshot::RollbackRegistrar,
) {
    registrar
        // ⭐ CHECKSUMMED 2026-08-29: a presence-only probe cannot see WHICH prop
        // the cycle chose, and `reset_cut_rope_boss_arena_on_room_reset`
        // advances it on the sim schedule.
        .rollback_resource_clone_checksum::<CutRopeHeavyObjectCycle>(
            "ambition_content::bosses",
            "content.cut_rope_heavy_object_cycle",
            "heavy-object cycle index",
            CutRopeHeavyObjectCycle::checksum,
        )
        .rollback_resource_clone_checksum_with_schema_detail::<PendingCutRopeRoomReplay>(
            "ambition_content::bosses",
            "content.pending_cut_rope_room_replay",
            "the dialogue-authored room replay, latched until the conversation ends",
            |pending| u64::from(pending.requested),
        )
        .clear_message_on_rollback::<CutRopeRoomReplayRequested>(
            "ambition_content::bosses",
            "message.cut_rope_room_replay_requested",
        );
    specials::register_rollback_state(registrar);
}

impl Plugin for AmbitionBossContentPlugin {
    fn build(&self, app: &mut App) {
        let sim = app.sim_schedule();
        // Compose all authored boss behavior, encounter, sheet, and special-row
        // data into this App. The same provider fragment serves standalone and
        // multi-game hosts without process-global install order.
        register(app);

        app.insert_resource(ambition_boss_encounter::BossEncounterRegistry::default());

        // Cut-rope arena state is CONTENT state — owned and initialized here,
        // never by the host's sim plugin (anti-god rule 5).
        app.init_resource::<CutRopeBossArenaState>();
        app.init_resource::<CutRopeHeavyObjectCycle>();
        app.init_resource::<PendingCutRopeRoomReplay>();
        // Content's own narrative vocabulary, registered the same way the
        // engine registers its own — which is the whole reason the ledger is
        // generic. `<<reset_cut_rope_room>>` is a gameplay-bearing choice
        // (a room replay despawns the world), so it crosses the boundary with a
        // stamped record rather than by setting a resource from `Update`.
        app.add_plugins(ambition_conversation::NarrativeInputPlugin::<
            CutRopeRoomReplayRequested,
        >::default());
        // The cycle/replay state is content-owned rollback state. Record its
        // host-independent schema here; the selected rollback host installs the
        // same declarations later from the application composition layer.
        {
            let mut registrar =
                ambition_platformer2d_runtime::rollback::SchemaRollbackRegistrar::new(app);
            register_rollback_state(&mut registrar);
        }

        // The named per-boss special-attack Techniques (state attachment +
        // schedule into the engine's `CombatSet::ContentSpecials` slot) are
        // a self-contained content domain unit.
        app.add_plugins(specials::BossSpecialContentPlugin);

        // Content hangs its room-reset + dialogue-followup work on the
        // engine's labeled slots — the host anchors the SLOTS into its
        // chains; it never names these systems (E5-finish de-weave):
        // - room re-entry/reset: restore the cut-rope arena's persisted state.
        // - dialogue followup: the "try again" beat emits the engine's
        //   generic `RoomReplayRequested` once the dialog closes.
        app.add_systems(
            sim,
            (
                reset_cut_rope_boss_arena_on_room_reset
                    .in_set(ambition_platformer2d_actor_monolith::session::reset::ContentRoomResetSet),
                emit_cut_rope_room_replay_after_the_conversation_ends
                    .in_set(ambition_platformer2d_actor_monolith::session::reset::ContentDialogueFollowupSet),
                // The content half of an in-place room replay: clear the cut-rope
                // boss's per-attempt state. The host anchors this slot before its
                // generic replay consumer (which owns the player/world reset).
                reset_cut_rope_attempt_on_replay
                    .in_set(ambition_platformer2d_actor_monolith::session::reset::ContentRoomReplayResetSet),
            ),
        );

        // Cut-rope post-damage flavor (rope-cut detection → gate, hazard→
        // visual mirror + impact flavor, prop visuals). Runs in the engine's
        // `CombatSet::ContentFlavor` slot (after feature-hit resolution so it
        // observes this frame's alive-flag transitions); the slot's position
        // in the combat chain is configured by the app's `CombatSchedulePlugin`.
        // Grouped into a `.chain()` to preserve the former inline ordering.
        app.add_systems(
            sim,
            (
                detect_cut_rope_rope_cut.in_set(GameplayGated),
                tick_cut_rope_flavor.in_set(GameplayGated),
                sync_cut_rope_boss_arena_prop_visuals,
            )
                .chain()
                .in_set(ambition_platformer2d_shared_tangle::schedule::CombatSet::ContentFlavor),
        );

        // Generic "lured movement" steering: any boss carrying a `CommandedMove`
        // (e.g. the cut-rope behemoth lured under the anvil by the encounter
        // script's `CommandMoveTo`) is steered toward its target, overriding the
        // brain. Runs in the machinery-defined `BossSteerSlot` (between
        // `tick_boss_brains_system` and `update_ecs_bosses` in the WorldPrep boss
        // chain) — exactly where the old cut-rope-specific steering ran.
        app.add_systems(
            sim,
            ambition_boss_encounter::tick_commanded_moves
                .in_set(ambition_platformer2d_shared_tangle::schedule::BossSteerSlot),
        );

        // Content progression systems hang on the engine's labeled Progression
        // slots — the host anchors each slot into the Progression chain at the
        // exact former position; this plugin never depends on that position, only
        // on the slot (E-track progression de-weave, same shape as the room-reset
        // and combat-flavor slots above).
        app.add_systems(
            sim,
            (
                // Cut-rope arena per-attempt setup — MID boss-tick (after the
                // engine advances encounter progress, before scripted hazards).
                setup_cut_rope_encounter.in_set(ambition_boss_encounter::ContentEncounterScriptSet),
                // Victory NPC spawn — after the boss chain frees the payload,
                // before the save mirrors run.
                spawn_cut_rope_victory_npc
                    .in_set(ambition_boss_encounter::ContentEncounterVictorySet),
            ),
        );

        // GNU-ton arena gate: a derived collision-overlay contributor (hides the
        // retreat ladder while the boss is alive; opens the floor-gate on defeat).
        // Runs in WorldPrep after the overlay rebuild clears the per-frame
        // contributions and before the WorldPrep collision consumers — exactly
        // like the encounter / intro lock-wall gates — so this frame's player /
        // actor / projectile collision sees the derived geometry.
        app.add_systems(
            sim,
            gate_gnu_ton_arena_ladder
                .after(ambition_platformer2d_shared_tangle::schedule::FeatureWorldOverlaySet)
                .before(ambition_platformer2d_actor_monolith::features::HazardTickSet)
                .in_set(ambition_platformer2d_shared_tangle::schedule::Platformer2dSimulationPhaseMonolith::WorldPrep),
        );

        // Cut-rope Yarn vocabulary: installed on the DialogueRunner via the
        // dialog runtime's content-bindings seam, plus the per-frame extras
        // feed (after the generic mirror refresh so the snapshot Yarn reads
        // is consistent within the tick).
        #[cfg(feature = "ui")]
        {
            app.init_resource::<ambition_dialog::YarnContentBindings>();
            app.world_mut()
                .resource_mut::<ambition_dialog::YarnContentBindings>()
                .installers
                .push(yarn::install_cut_rope_yarn_bindings);
            // ⭐ Same gate as its sibling, and for the same reason: this took a
            // WRITE guard on the Yarn mirror's `RwLock` and allocated two
            // `String`s every frame of every room, feeding a value only a live
            // `<<if>>` ever reads. Its `save.is_none()` bail is the rare path,
            // not the common one.
            app.add_systems(
                Update,
                yarn::mirror_cut_rope_heavy_object
                    .after(ambition_dialog::YarnStateMirrorRefreshed)
                    .run_if(crate::yarn_vocabulary::a_conversation_is_live),
            );
        }
    }
}

#[cfg(test)]
mod apple_rain_animation_key_tests {
    use ambition_characters::brain::BossAttackProfile;

    /// `apple_rain`'s hurtbox row is reachable ONLY through the profile
    /// identity, and that is what blocks the boss-animator fold.
    ///
    /// The fold's first slice wants to replace four
    /// `sample.profile == Some(profile)` checks with a key comparison, so
    /// `BossAnimationFrameSample` can drop `BossAttackProfile` and become
    /// character-generic. For every profile the ENGINE names, the sample writer's
    /// key still lands inside the profile's own key list, so the swap would be
    /// safe (`every_hardcoded_sample_key_names_a_row_its_profile_claims`).
    ///
    /// `apple_rain` is the exception, and it is CONTENT, which is why the
    /// engine could not answer it. It is a `Special`, so its key list comes
    /// from this crate's `special_animation_keys()` — and it is not in that map.
    /// The profile therefore claims NOTHING, while
    /// `boss_animation_key_for_sample` emits `"head_down"` for it.
    ///
    /// So today the hurtbox row is found only because the profile matched:
    /// `runtime_animation_keys` pushes the sample's own key into the list when
    /// `sample.profile == Some(profile)`. A key-based rule would look for
    /// `"head_down"` in an EMPTY list, miss, and fall back to elapsed-time
    /// sampling — a different hurtbox on a live boss.
    ///
    /// Two ways out, and this test fails on either so nobody does it silently:
    /// give `apple_rain` its rows in `special_animation_keys()`, or keep the
    /// profile identity. It is deliberately not asserting which.
    #[test]
    fn apple_rain_claims_no_animation_rows_which_is_why_the_fold_is_blocked() {
        let catalog = super::authored_boss_catalog();
        let profile = BossAttackProfile::Special("apple_rain".to_string());
        let claimed =
            ambition_boss_encounter::behavior::boss_animation_keys_for_profile(&catalog, &profile);
        assert!(
            claimed.is_empty(),
            "`apple_rain` now claims {claimed:?}. If that list contains \
             \"head_down\" — the key `boss_animation_key_for_sample` emits for it \
             — then the last blocker on the boss-animator fold's first slice is \
             gone and the profile identity can be replaced by a key comparison. \
             Update the fold's row in the 72h queue rather than just this test"
        );
    }
}

#[cfg(test)]
mod encounter_book_tests {
    /// The nine encounter files reach the runtime through the COMPILER.
    ///
    /// This is the probe that the merged book is what the catalog actually holds.
    ///
    /// it asserts the ARTIFACT and the CATALOG agree, not just that both
    /// exist. "The book has nine entries" would pass over a builder that
    /// ignored it and reparsed, which is the exact defect being closed.
    #[test]
    fn the_encounter_book_the_runtime_loads_is_the_one_the_compiler_merged() {
        let book = ambition_boss_encounter::pattern::content_schema::lowered_boss_encounters(
            crate::pack::prepared(),
        )
        .expect("the encounter schema merges its files for every pack that compiles");
        assert_eq!(
            book.len(),
            super::BOSS_ENCOUNTERS.len(),
            "one merged entry per authored file"
        );
        let catalog = super::authored_boss_catalog();
        for (id, spec) in book {
            assert_eq!(
                catalog.encounter(id),
                Some(spec),
                "encounter `{id}` in the catalog must BE the compiler's merged value, not a \
                 second parse that happens to agree today"
            );
        }
    }
}
