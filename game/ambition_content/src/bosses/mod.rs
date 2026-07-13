//! Named Ambition boss content registration.
//!
//! Owns Ambition's immutable App-local boss fragment and the live
//! [`BossEncounterRegistry`] resource used by an active session. The general
//! boss machinery (profiles, specs, encounter registry/system, patterns) still
//! lives in `ambition_actors::boss_encounter`; this module owns the bespoke per-boss
//! *behavior* and *bark content* that names individual bosses:
//!
//! - [`gnu_ton`] — GNU-ton's bespoke arena gating (retreat-ladder reveal +
//!   floor-gate) and head-hurtbox regression coverage.
//! - [`banter`] — boss combat-banter lines + the idle-bark ticker
//!   ([`banter::install_boss_banter`] / [`banter::tick_boss_idle_barks`]),
//!   installed next to its dialogue registration.

use ambition_platformer_primitives::schedule::gameplay_allowed;
use ambition_platformer_primitives::schedule::SimScheduleExt;
use bevy::prelude::*;

pub mod banter;
pub mod cut_rope;
pub mod gnu_ton;
pub mod specials;
#[cfg(feature = "ui")]
pub mod yarn;

pub use banter::{install_boss_banter, tick_boss_idle_barks};
pub use cut_rope::{
    detect_cut_rope_rope_cut, emit_cut_rope_room_replay_after_dialogue_closes, is_cut_rope_boss,
    reset_cut_rope_boss_arena_on_room_reset, reset_cut_rope_boss_attempt, setup_cut_rope_encounter,
    spawn_cut_rope_victory_npc, sync_cut_rope_boss_arena_prop_visuals, tick_cut_rope_flavor,
    CutRopeBossArenaState, CutRopeHeavyObjectCycle, PendingCutRopeRoomReplay,
    SmirkingBehemothVictoryNpc, CUT_ROPE_BOSS_ID, CUT_ROPE_VICTORY_NPC_DIALOGUE_ID,
    CUT_ROPE_VICTORY_NPC_ID,
};
pub use gnu_ton::gate_gnu_ton_arena_ladder;

/// The authored boss-behavior roster, verbatim. Exposed so the seed-library test
/// can re-derive each seed's duration bands from the same bytes the game loads —
/// a band measured against a copy is not a measurement.
pub const BOSS_PROFILES_RON: &str = include_str!("../../assets/data/boss_profiles.ron");

/// The boss SEED LIBRARY (`boss-design.md` §2, slice BD4): nine attack archetypes
/// extracted from the roster above, each with its design intent, fair-counter set,
/// and measured telegraph/active bands.
pub const BOSS_SEEDS_RON: &str = include_str!("../../assets/data/boss_seeds.ron");

/// BD5's per-game fairness calibration (`boss-design.md` §3). One RON per game, so
/// re-calibrating a fight's fairness is an edit, not a recompile.
pub const BOSS_VALIDATOR_BANDS_RON: &str =
    include_str!("../../assets/data/boss_validator_bands.ron");

/// The parsed validator bands. Panics at first use if the RON is malformed, which
/// a content test catches long before a fight does.
pub fn validator_bands(
) -> &'static ambition_characters::brain::boss_pattern::validator::ValidatorBands {
    use ambition_characters::brain::boss_pattern::validator::ValidatorBands;
    static BANDS: std::sync::LazyLock<ValidatorBands> = std::sync::LazyLock::new(|| {
        ValidatorBands::from_ron(BOSS_VALIDATOR_BANDS_RON)
            .unwrap_or_else(|err| panic!("boss_validator_bands.ron failed to deserialize: {err}"))
    });
    &BANDS
}

/// The parsed seed library. Parsed once; panics at first use if the RON is
/// malformed, which a content test catches long before a player does.
pub fn seed_library() -> &'static ambition_characters::brain::boss_pattern::seeds::SeedLibrary {
    use ambition_characters::brain::boss_pattern::seeds::SeedLibrary;
    static LIB: std::sync::LazyLock<SeedLibrary> = std::sync::LazyLock::new(|| {
        SeedLibrary::from_ron(BOSS_SEEDS_RON)
            .unwrap_or_else(|err| panic!("boss_seeds.ron failed to deserialize: {err}"))
    });
    &LIB
}

/// Embedded encounter rows contributed by the Ambition provider.
pub const BOSS_ENCOUNTER_RONS: &[&str] = &[
    include_str!("../../assets/data/boss_encounters/clockwork_warden.ron"),
    include_str!("../../assets/data/boss_encounters/mockingbird.ron"),
    include_str!("../../assets/data/boss_encounters/gnu_ton_rider.ron"),
    include_str!("../../assets/data/boss_encounters/smirking_behemoth_boss.ron"),
    include_str!("../../assets/data/boss_encounters/flying_spaghetti_monster_boss.ron"),
    include_str!("../../assets/data/boss_encounters/trex_boss.ron"),
    include_str!("../../assets/data/boss_encounters/mode_collapse_boss.ron"),
    include_str!("../../assets/data/boss_encounters/exploding_gradient_boss.ron"),
    include_str!("../../assets/data/boss_encounters/overflow_boss.ron"),
];

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
        (
            "mode_collapse_converge".into(),
            vec!["spike_halo".into()],
        ),
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
pub fn boss_catalog_fragment() -> ambition_actors::boss_encounter::BossCatalogFragment {
    ambition_actors::boss_encounter::BossCatalogFragment::from_ron(
        crate::AMBITION_CONTENT_PROVIDER,
        Some("clockwork_warden"),
        Some("gradient_sentinel"),
        BOSS_PROFILES_RON,
        BOSS_ENCOUNTER_RONS,
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
pub fn authored_boss_catalog() -> ambition_actors::boss_encounter::BossCatalog {
    let mut registry = ambition_actors::boss_encounter::BossCatalogRegistry::default();
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
    use ambition_actors::boss_encounter::BossCatalogAppExt as _;
    app.register_boss_catalog_fragment(boss_catalog_fragment());
}

/// Registers Ambition's boss fragment, initializes the live encounter
/// registry resource, and installs the cut-rope Yarn vocabulary + mirror feed.
pub struct AmbitionBossContentPlugin;

impl Plugin for AmbitionBossContentPlugin {
    fn build(&self, app: &mut App) {
        let sim = app.sim_schedule();
        // Compose all authored boss behavior, encounter, sheet, and special-row
        // data into this App. The same provider fragment serves standalone and
        // multi-game hosts without process-global install order.
        register(app);

        app.insert_resource(ambition_actors::boss_encounter::BossEncounterRegistry::default());

        // Cut-rope arena state is CONTENT state — owned and initialized here,
        // never by the host's sim plugin (anti-god rule 5).
        app.init_resource::<CutRopeBossArenaState>();
        app.init_resource::<CutRopeHeavyObjectCycle>();
        app.init_resource::<PendingCutRopeRoomReplay>();

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
                    .in_set(ambition_actors::session::reset::ContentRoomResetSet),
                emit_cut_rope_room_replay_after_dialogue_closes
                    .in_set(ambition_actors::session::reset::ContentDialogueFollowupSet),
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
                detect_cut_rope_rope_cut.run_if(gameplay_allowed),
                tick_cut_rope_flavor.run_if(gameplay_allowed),
                sync_cut_rope_boss_arena_prop_visuals,
            )
                .chain()
                .in_set(ambition_platformer_primitives::schedule::CombatSet::ContentFlavor),
        );

        // Generic "lured movement" steering: any boss carrying a `CommandedMove`
        // (e.g. the cut-rope behemoth lured under the anvil by the encounter
        // script's `CommandMoveTo`) is steered toward its target, overriding the
        // brain. Runs in the machinery-defined `BossSteerSlot` (between
        // `tick_boss_brains_system` and `update_ecs_bosses` in the WorldPrep boss
        // chain) — exactly where the old cut-rope-specific steering ran.
        app.add_systems(
            sim,
            ambition_actors::boss_encounter::tick_commanded_moves
                .in_set(ambition_platformer_primitives::schedule::BossSteerSlot),
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
                setup_cut_rope_encounter
                    .in_set(ambition_actors::boss_encounter::ContentEncounterScriptSet),
                // Victory NPC spawn — after the boss chain frees the payload,
                // before the save mirrors run.
                spawn_cut_rope_victory_npc
                    .in_set(ambition_actors::boss_encounter::ContentEncounterVictorySet),
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
                .after(ambition_actors::features::rebuild_feature_ecs_world_overlay)
                .before(ambition_actors::features::update_ecs_hazards)
                .in_set(ambition_platformer_primitives::schedule::SandboxSet::WorldPrep),
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
            app.add_systems(
                Update,
                yarn::mirror_cut_rope_heavy_object
                    .after(ambition_actors::dialog::yarn_bindings::refresh_yarn_state_mirror),
            );
        }
    }
}
