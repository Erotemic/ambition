//! Named Ambition boss content registration.
//!
//! Owns the install of the default [`BossEncounterRegistry`] so the named
//! boss roster is constructed in one content-owned place. The general boss
//! machinery (profiles, specs, encounter registry/system, patterns) still
//! lives in `ambition_gameplay_core::boss_encounter`; this module owns the bespoke per-boss
//! *behavior* and *bark content* that names individual bosses:
//!
//! - [`gnu_ton`] — GNU-ton's bespoke arena gating (retreat-ladder reveal +
//!   floor-gate) and head-hurtbox regression coverage.
//! - [`banter`] — boss combat-banter lines + the idle-bark ticker
//!   ([`banter::install_boss_banter`] / [`banter::tick_boss_idle_barks`]),
//!   installed next to its dialogue registration.

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
    CutRopeBossArenaState, CutRopeHeavyObjectCycle, CutRopeRoomReplayRequested,
    PendingCutRopeRoomReplay, SmirkingBehemothVictoryNpc, CUT_ROPE_BOSS_ID,
    CUT_ROPE_VICTORY_NPC_DIALOGUE_ID, CUT_ROPE_VICTORY_NPC_ID,
};
pub use gnu_ton::gate_gnu_ton_arena_ladder;

/// Install the named boss-behavior roster (`boss_profiles.ron`) into the
/// machinery lib's holder. Called by [`AmbitionBossContentPlugin`] at build
/// time, and by content tests that resolve boss profiles without assembling the
/// full app. First install wins (idempotent across the test binary).
pub fn install_boss_roster() {
    // Per-boss behavior (movement / attacks / rewards).
    ambition_gameplay_core::boss_encounter::install_boss_profiles(
        ambition_gameplay_core::boss_encounter::BossProfileRegistry::from_ron(include_str!(
            "../../assets/data/boss_profiles.ron"
        )),
    );

    // Per-boss SPRITESHEET layouts (C6 — content out of core). Byte-identical to
    // the engine's built-in demo-boss defaults (pinned by
    // `boss_sheets_ron_matches_builtin_defaults`), so shipped bosses render
    // unchanged; content re-authors a boss's sheet by editing its row in
    // `boss_sheets.ron` with no Rust change.
    ambition_gameplay_core::boss_encounter::sprites::install_boss_sheets(
        ambition_gameplay_core::boss_encounter::sprites::BossSheetRegistry::from_ron(include_str!(
            "../../assets/data/boss_sheets.ron"
        )),
    );

    // Per-boss encounter specs (HP / phase thresholds / timings / music), one
    // embedded RON per boss. Embedded (not fs-read) so shipped binaries carry
    // the data; the lib holds only the generic `BossEncounterSpec` schema.
    const BOSS_ENCOUNTER_RONS: &[&str] = &[
        include_str!("../../assets/data/boss_encounters/clockwork_warden.ron"),
        include_str!("../../assets/data/boss_encounters/mockingbird.ron"),
        include_str!("../../assets/data/boss_encounters/gnu_ton.ron"),
        include_str!("../../assets/data/boss_encounters/smirking_behemoth_boss.ron"),
        include_str!("../../assets/data/boss_encounters/flying_spaghetti_monster_boss.ron"),
        include_str!("../../assets/data/boss_encounters/trex_boss.ron"),
        include_str!("../../assets/data/boss_encounters/mode_collapse_boss.ron"),
        include_str!("../../assets/data/boss_encounters/exploding_gradient_boss.ron"),
        include_str!("../../assets/data/boss_encounters/overflow_boss.ron"),
    ];
    let specs = BOSS_ENCOUNTER_RONS
        .iter()
        .map(|text| {
            ron::from_str::<ambition_gameplay_core::boss_encounter::BossEncounterSpec>(text)
                .expect("boss_encounters/*.ron should parse as BossEncounterSpec")
        })
        .collect();
    ambition_gameplay_core::boss_encounter::install_boss_encounter_specs(specs);

    // Telegraph anim rows for each content boss-special key. The engine ships
    // no anim row for content specials (it names none); this is where the
    // key→sprite-row mapping lives. `apple_rain` damages via projectile bodies
    // and has no body-mounted telegraph row, so it's simply absent (→ no row).
    ambition_gameplay_core::boss_encounter::install_boss_special_anim_keys(
        std::collections::HashMap::from([
            (
                "overfit_volley".to_string(),
                &["spike_halo", "eye_beam"] as &'static [&'static str],
            ),
            ("eye_beam".to_string(), &["eye_beam", "spike_halo"]),
            ("minima_trap".to_string(), &["spike_halo"]),
            ("saddle_point".to_string(), &["spike_halo"]),
            ("gradient_cascade".to_string(), &["spike_halo"]),
            ("mode_collapse_converge".to_string(), &["spike_halo"]),
            ("gradient_nova".to_string(), &["spike_halo"]),
            ("overflow_flood".to_string(), &["spike_halo"]),
            ("seismic_stomp".to_string(), &["floor_slam", "spike_halo"]),
            ("echo_fan".to_string(), &["spike_halo", "eye_beam"]),
        ]),
    );
}

/// Installs the default Ambition boss encounter registry resource and
/// the cut-rope Yarn vocabulary + mirror feed.
pub struct AmbitionBossContentPlugin;

impl Plugin for AmbitionBossContentPlugin {
    fn build(&self, app: &mut App) {
        // Install the named boss-behavior roster into the machinery lib's
        // holder at plugin-build time (before any boss spawn / profile clone),
        // so `BossBehaviorProfile::from_data` resolves against content data —
        // the lib embeds no boss data in production. Mirrors the enemy roster.
        install_boss_roster();

        app.insert_resource(
            ambition_gameplay_core::boss_encounter::BossEncounterRegistry::default(),
        );

        // The named per-boss special-attack Techniques (state attachment +
        // schedule into the engine's `CombatSet::ContentSpecials` slot) are
        // a self-contained content domain unit.
        app.add_plugins(specials::BossSpecialContentPlugin);

        // Cut-rope post-damage flavor (rope-cut detection → gate, hazard→
        // visual mirror + impact flavor, prop visuals). Runs in the engine's
        // `CombatSet::ContentFlavor` slot (after feature-hit resolution so it
        // observes this frame's alive-flag transitions); the slot's position
        // in the combat chain is configured by the app's `CombatSchedulePlugin`.
        // Grouped into a `.chain()` to preserve the former inline ordering.
        app.add_systems(
            Update,
            (
                detect_cut_rope_rope_cut
                    .run_if(ambition_gameplay_core::session::game_mode::gameplay_allowed),
                tick_cut_rope_flavor
                    .run_if(ambition_gameplay_core::session::game_mode::gameplay_allowed),
                sync_cut_rope_boss_arena_prop_visuals,
            )
                .chain()
                .in_set(ambition_gameplay_core::schedule::CombatSet::ContentFlavor),
        );

        // Generic "lured movement" steering: any boss carrying a `CommandedMove`
        // (e.g. the cut-rope behemoth lured under the anvil by the encounter
        // script's `CommandMoveTo`) is steered toward its target, overriding the
        // brain. Runs in the machinery-defined `BossSteerSlot` (between
        // `tick_boss_brains_system` and `update_ecs_bosses` in the WorldPrep boss
        // chain) — exactly where the old cut-rope-specific steering ran.
        app.add_systems(
            Update,
            ambition_gameplay_core::boss_encounter::tick_commanded_moves
                .in_set(ambition_gameplay_core::schedule::BossSteerSlot),
        );

        // Cut-rope Yarn vocabulary: installed on the DialogueRunner via the
        // dialog runtime's content-bindings seam, plus the per-frame extras
        // feed (after the generic mirror refresh so the snapshot Yarn reads
        // is consistent within the tick).
        #[cfg(feature = "ui")]
        {
            app.init_resource::<ambition_gameplay_core::dialog::yarn_bindings::YarnContentBindings>();
            app.world_mut()
                .resource_mut::<ambition_gameplay_core::dialog::yarn_bindings::YarnContentBindings>(
                )
                .installers
                .push(yarn::install_cut_rope_yarn_bindings);
            app.add_systems(
                Update,
                yarn::mirror_cut_rope_heavy_object.after(
                    ambition_gameplay_core::dialog::yarn_bindings::refresh_yarn_state_mirror,
                ),
            );
        }
    }
}
