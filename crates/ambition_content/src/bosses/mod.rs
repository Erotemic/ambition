//! Named Ambition boss content registration.
//!
//! Owns the install of the default [`BossEncounterRegistry`] so the named
//! boss roster is constructed in one content-owned place. The general boss
//! machinery (profiles, specs, encounter registry/system, patterns) still
//! lives in `ambition_sandbox::boss_encounter`; this module owns the bespoke per-boss
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
    emit_cut_rope_room_replay_after_dialogue_closes, is_cut_rope_boss,
    reset_cut_rope_boss_arena_on_room_reset, reset_cut_rope_boss_attempt,
    spawn_cut_rope_victory_npc, steer_cut_rope_boss_under_anvil,
    sync_cut_rope_boss_arena_prop_visuals, tick_cut_rope_boss_arena, CutRopeBossArenaState,
    CutRopeHeavyObjectCycle, CutRopeRoomReplayRequested, PendingCutRopeRoomReplay,
    SmirkingBehemothVictoryNpc, CUT_ROPE_BOSS_ID, CUT_ROPE_VICTORY_NPC_DIALOGUE_ID,
    CUT_ROPE_VICTORY_NPC_ID,
};
pub use gnu_ton::gate_gnu_ton_arena_ladder;

/// Install the named boss-behavior roster (`boss_profiles.ron`) into the
/// machinery lib's holder. Called by [`AmbitionBossContentPlugin`] at build
/// time, and by content tests that resolve boss profiles without assembling the
/// full app. First install wins (idempotent across the test binary).
pub fn install_boss_roster() {
    // Per-boss behavior (movement / attacks / rewards).
    ambition_sandbox::boss_encounter::install_boss_profiles(
        ambition_sandbox::boss_encounter::BossProfileRegistry::from_ron(include_str!(
            "../../assets/data/boss_profiles.ron"
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
            ron::from_str::<ambition_sandbox::boss_encounter::BossEncounterSpec>(text)
                .expect("boss_encounters/*.ron should parse as BossEncounterSpec")
        })
        .collect();
    ambition_sandbox::boss_encounter::install_boss_encounter_specs(specs);

    // Telegraph anim rows for each content boss-special key. The engine ships
    // no anim row for content specials (it names none); this is where the
    // key→sprite-row mapping lives. `apple_rain` damages via projectile bodies
    // and has no body-mounted telegraph row, so it's simply absent (→ no row).
    ambition_sandbox::boss_encounter::install_boss_special_anim_keys(
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

        app.insert_resource(ambition_sandbox::boss_encounter::BossEncounterRegistry::default());

        // Boss special Techniques own their per-boss temporal state. Attach each
        // to every boss (the `BossConfig` marker) via required components, so the
        // machinery lib's spawn no longer names a boss technique. Registered
        // before any boss spawns (plugin build time). First: the eye beam.
        app.register_required_components::<
            ambition_sandbox::features::BossConfig,
            specials::EyeBeamState,
        >();
        app.register_required_components::<
            ambition_sandbox::features::BossConfig,
            specials::AppleRainSpawnState,
        >();
        app.register_required_components::<
            ambition_sandbox::features::BossConfig,
            specials::OverfitVolleyState,
        >();
        app.register_required_components::<
            ambition_sandbox::features::BossConfig,
            specials::MinimaTrapState,
        >();
        app.register_required_components::<
            ambition_sandbox::features::BossConfig,
            specials::SaddlePointState,
        >();
        app.register_required_components::<
            ambition_sandbox::features::BossConfig,
            specials::GradientCascadeState,
        >();
        app.register_required_components::<
            ambition_sandbox::features::BossConfig,
            specials::ModeCollapseState,
        >();
        app.register_required_components::<
            ambition_sandbox::features::BossConfig,
            specials::ExplodingGradientState,
        >();

        // Cut-rope boss steering: tracks the hanging anvil during the
        // encounter. Runs in the machinery-defined `BossSteerSlot`
        // (between `tick_boss_brains_system` and `update_ecs_bosses`
        // inside the WorldPrep boss chain).
        app.add_systems(
            Update,
            cut_rope::steer_cut_rope_boss_under_anvil.in_set(ambition_sandbox::app::BossSteerSlot),
        );

        // Cut-rope Yarn vocabulary: installed on the DialogueRunner via the
        // dialog runtime's content-bindings seam, plus the per-frame extras
        // feed (after the generic mirror refresh so the snapshot Yarn reads
        // is consistent within the tick).
        #[cfg(feature = "ui")]
        {
            app.init_resource::<ambition_sandbox::dialog::yarn_bindings::YarnContentBindings>();
            app.world_mut()
                .resource_mut::<ambition_sandbox::dialog::yarn_bindings::YarnContentBindings>()
                .installers
                .push(yarn::install_cut_rope_yarn_bindings);
            app.add_systems(
                Update,
                yarn::mirror_cut_rope_heavy_object
                    .after(ambition_sandbox::dialog::yarn_bindings::refresh_yarn_state_mirror),
            );
        }
    }
}
