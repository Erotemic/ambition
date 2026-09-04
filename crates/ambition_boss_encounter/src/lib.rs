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
//! ⛔⛔ WHAT THIS CRATE REFUSES, because a destination that says nothing accepts
//! everything — and this one accepted for months.
//!
//! - **Generic body geometry.** A boss was the FIRST customer of AABB overlap,
//!   swept extents and the shared `CombatGeometry` vocabulary, not their owner,
//!   and holding them made every consumer read that vocabulary as boss-specific.
//!   531 lines left for `ambition_combat::body_geometry` and the sprite metrics
//!   for `ambition_sprite_sheet`; ⛔ do not let the next generic concept land here
//!   because a boss needed it first.
//! - **Anything whose second consumer would not be a boss.** That is the test,
//!   and it is answerable before the code moves.

pub mod anim;
pub mod attack_geometry;
pub mod attack_moveset;
pub mod behavior;
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

pub use ids::encounter_id_from_name;
// The engine hard-codes no boss id: `ids` ships only the slugging helper.
pub use ambition_characters::boss_encounter::{
    ActorPhaseState, BossEncounterPhase, BossEncounterSpec, BossPhaseEvent, PhaseTrigger,
    PhaseTriggerCondition,
};
pub use behavior::{BossBehaviorProfileExt, BossProfileRegistry, LimbMotion, LimbRoute};
// The boss DATA MODEL — the authoritative components + the borrow views the per-tick systems
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
    boss_is_cleared, BossClusterQueryData, BossClusterRef, BossClusterScratch, BossConfig,
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
// timeline authority); re-exported here so boss content + the schedule keep
// importing it through `boss_encounter`.
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
/// ⭐⭐ THE POINT IS THAT A CONSUMER CAN NOW OMIT IT. Before this, these eight
/// systems were scheduled by `ambition_platformer2d_runtime`'s
/// `progression_schedule`, its three messages were registered there, and two of
/// its resources were initialised in `sim_core_resources` — so "generic
/// encounters without boss encounters", one of the compositions named in
/// `docs/planning/engine/decomposition.md`, could not be written at all: there
/// was no seam to omit the capability through. It is a `.disable::<_>()` now.
///
/// ⭐ IT NAMES ONLY PUBLISHED SET VOCABULARY. `ProgressionSet::BossAdvance` and
/// `BossHazards` live in `ambition_platformer2d_shared_tangle::schedule`, which
/// this crate already depended on, so nothing had to move to make this possible
/// and no ordering was renegotiated. That is what made it a plugin rather than a
/// carve — and it is the check worth running before proposing the next one:
/// a capability whose ordering edges name another capability's SYSTEMS cannot be
/// installed this way, however coherent its authority is.
///
/// ⚠ THE HOST STILL OWNS THE SETS. This plugin does not `configure_sets`; the
/// runtime anchors `ProgressionSet` into the engine chain, and this only says
/// which systems belong in two of its slots. A capability that configured the
/// ordering it runs in would be a second authority over the schedule.
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

// ── Progression-phase content slots (E-track de-weave) ──────────────────────
//
// The `Platformer2dSimulationPhaseMonolith::Progression` chain is ENGINE-generic (boss-encounter tick,
// save mirrors, room metadata/music, portal phase, map visits). Named-game
// CONTENT that must interleave with it hangs on these labeled slots; the host
// anchors each slot into the engine chain via `configure_sets`, and content
// plugins register their systems `.in_set(the slot)` — the engine chain never
// names a content system (anti-god rule 3), same shape as the combat-schedule
// (`CombatSet::ContentSpecials`/`ContentFlavor`) and reset (`ContentRoomResetSet`)
// slots. Co-located here because Progression is the boss-encounter-dominated
// phase (mirrors `session::reset` owning both of ITS content slots).

/// Progression slot for content that sets up an encounter's scripted state
/// MID boss-tick — after the engine advances encounter progress, before the
/// scripted hazards/beats tick (e.g. the cut-rope arena's per-attempt setup).
#[derive(bevy::ecs::schedule::SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct ContentEncounterScriptSet;

/// Progression slot for content that reacts to an encounter's RESOLUTION —
/// after the boss chain finishes (payloads released, phase feedback), before
/// the save mirrors run (e.g. spawning a victory NPC once the payload is free).
#[derive(bevy::ecs::schedule::SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct ContentEncounterVictorySet;

/// Progression slot for content quest-completion effects — after the engine's
/// quest advance pump, before room metadata/music sync (e.g. granting authored
/// completion rewards).
#[derive(bevy::ecs::schedule::SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct ContentQuestRewardSet;

// Domain-owned rollback declaration; the host supplies the backend registrar.
mod rollback_registration;
pub use rollback_registration::register_rollback_state;
