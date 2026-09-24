//! Boss encounter domain.
//!
//! Boss health and phase state are entity-local. This crate owns boss catalogs,
//! cluster views, encounter scripts, rewards, events, attack geometry, sprites, and
//! the systems that synchronize those pieces. Generic encounter timeline vocabulary
//! remains in `ambition_encounter`.
//!
//! The runtime owns the outer progression schedule; this crate exposes named content
//! sets so game-specific systems can interleave without the runtime depending on
//! boss content.
//!
//! What this crate does not own:
//!
//! - Generic body geometry. A boss was the first user of AABB overlap, swept
//!   extents and the shared `CombatGeometry` vocabulary, not their owner. That
//!   code lives in `ambition_combat::body_geometry`, and sprite metrics in
//!   `ambition_sprite_sheet`. Do not add a generic concept here because a
//!   boss needed it first.
//! - Anything whose second user would not be a boss.

pub mod anim;
pub mod attack_geometry;
pub mod attack_moveset;
pub mod behavior;
pub mod conditions;
mod catalog;
mod clusters;
pub mod ecs;
mod encounter_entity;
mod encounter_script;
mod events;
pub mod pattern;
pub use events::BossPhaseChanged;
mod ids;
mod profile;
mod registry;
mod rewards;
mod specs;
pub mod sprites;
mod systems;

mod roster;
#[cfg(test)]
mod tests;

pub use ids::{encounter_id_from_name, renamed_encounter_id};
// The engine hard-codes no boss id: `ids` ships only the slugging helper.
pub use ambition_characters::boss_encounter::{
    ActorPhaseState, BossEncounterPhase, BossEncounterSpec, BossPhaseEvent, PhaseTrigger,
    PhaseTriggerCondition,
};
pub use behavior::{BossBehaviorProfileExt, BossProfileRegistry, LimbMotion, LimbRoute};
// The boss data model — the authoritative components + the borrow views the per-tick systems
// mutate/read.
#[cfg(any(test, feature = "test-support"))]
pub use catalog::test_boss_catalog;
pub use catalog::{
    BossCatalog, BossCatalogAppExt, BossCatalogAssemblyError, BossCatalogFragment,
    BossCatalogRegistry,
};
#[cfg(any(test, feature = "test-support"))]
pub use clusters::test_support;
pub use clusters::{
    boss_is_cleared, placement_is_cleared, BossClusterQueryData, ClearedBossPlacements, BossClusterRef, BossClusterScratch, BossConfig,
    BossEncounter, BossMut, BossOverrides, BossRef,
};
pub use encounter_entity::{
    release_payloads_on_death, sync_boss_encounter_entities, update_encounter_progress,
    EncounterDef, EncounterProgress, MemberProgress, PayloadReleased, ReleaseOnDeath,
};
pub use encounter_script::{
    drop_hazard, tick_commanded_moves, tick_encounter_scripts, tick_falling_hazards, CommandedMove,
    FallingHazard,
};
// The generic timeline vocabulary lives in `ambition_encounter` (the one
// timeline authority); re-exported so boss content and the schedule import it
// through `boss_encounter`.
pub use ambition_encounter::{
    EncounterBeat, EncounterEffect, EncounterGate, EncounterScript, EncounterTrigger,
};
pub use profile::{default_boss_profiles, BossProfile, BossRewardProfile};
pub use registry::BossEncounterRegistry;
pub use rewards::sync_boss_reward_chests_ecs;
pub use roster::BossSpecRoster;
pub use specs::default_boss_specs;
pub use systems::{
    boss_phase_transition_feedback, notify_bosses_on_mount_death, populate_boss_encounter_registry,
    update_boss_encounters,
};

/// Installs the boss-encounter capability: its sim systems, its messages and its
/// resources.
///
/// A consumer can omit this plugin (`.disable::<_>()`), for example to compose
/// generic encounters without boss encounters
/// (`docs/planning/engine/decomposition.md`).
///
/// It names only published set vocabulary: `ProgressionSet::BossAdvance` and
/// `BossHazards` live in `ambition_platformer2d_shared_tangle::schedule`. A
/// capability whose ordering edges name another capability's systems cannot
/// be installed as a plugin like this.
///
/// The host owns the sets. This plugin does not `configure_sets`; the runtime
/// anchors `ProgressionSet` into the engine chain, and this only places
/// systems in two of its slots. A capability that configured its own ordering
/// would be a second authority over the schedule.
pub struct BossEncounterSimulationPlugin;

impl bevy::prelude::Plugin for BossEncounterSimulationPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        use ambition_platformer2d_shared_tangle::schedule::{ProgressionSet, SimScheduleExt};
        use bevy::prelude::IntoScheduleConfigs;

        let sim = app.sim_schedule();

        app.init_resource::<BossCatalog>();
        app.init_resource::<BossEncounterRegistry>();
        app.add_message::<EncounterGate>();
        app.add_message::<PayloadReleased>();
        app.add_message::<BossPhaseChanged>();

        app.add_systems(
            sim,
            (
                // Mount-death → `mount_died` external phase trigger, ahead of
                // the phase driver so the swap is same-frame (Q19).
                notify_bosses_on_mount_death,
                update_boss_encounters,
                sync_boss_encounter_entities,
                update_encounter_progress,
            )
                .chain()
                .in_set(ProgressionSet::BossAdvance),
        );
        app.add_systems(
            sim,
            (
                tick_falling_hazards,
                tick_encounter_scripts,
                release_payloads_on_death,
                boss_phase_transition_feedback,
            )
                .chain()
                .in_set(ProgressionSet::BossHazards),
        );
    }
}

// ── Progression-phase content slots ──────────────────────────────────────────
//
// The `Platformer2dSimulationPhaseMonolith::Progression` chain is
// engine-generic (boss-encounter tick, save mirrors, room metadata/music,
// portal phase, map visits). Named-game content that must interleave with it
// uses these labeled slots: the host anchors each slot into the engine chain
// via `configure_sets`, and content plugins register systems
// `.in_set(the slot)`, so the engine chain never names a content system. This
// is the same shape as the combat-schedule (`CombatSet::ContentSpecials` /
// `ContentFlavor`) and reset (`ContentRoomResetSet`) slots. They live here
// because Progression is mostly the boss-encounter phase.

/// Progression slot for content that sets up an encounter's scripted state
/// mid boss-tick: after the engine advances encounter progress, before the
/// scripted hazards/beats tick (e.g. the cut-rope arena's per-attempt setup).
#[derive(bevy::ecs::schedule::SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct ContentEncounterScriptSet;

/// Progression slot for content that reacts to an encounter's resolution:
/// after the boss chain finishes (payloads released, phase feedback), before
/// the save mirrors run (e.g. spawning a victory NPC once the payload is free).
#[derive(bevy::ecs::schedule::SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct ContentEncounterVictorySet;

/// Progression slot for content quest-completion effects: after the engine's
/// quest advance pump, before room metadata/music sync (e.g. granting authored
/// completion rewards).
#[derive(bevy::ecs::schedule::SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct ContentQuestRewardSet;

// Domain-owned rollback declaration; the host supplies the backend registrar.
mod rollback_registration;
pub use rollback_registration::register_rollback_state;
