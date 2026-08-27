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

pub mod anim;
pub mod attack_geometry;
pub mod behavior;
mod catalog;
mod clusters;
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
    tick_commanded_moves, tick_encounter_scripts, tick_falling_hazards, CommandedMove,
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
