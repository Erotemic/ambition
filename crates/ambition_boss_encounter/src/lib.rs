//! Ambition's boss-fight coordinator — the BOSS DOMAIN, carved out of the actor
//! monolith 2026-08-17 (D33). Distinct from the generic enemy-wave system in
//! `ambition_encounter`.
//!
//! Boss HP/phase state is ENTITY-LOCAL (`BossEncounter.health` +
//! `BossEncounter.encounter: ActorPhaseState`); this crate bridges it to the
//! in-arena boss ECS clusters (`BossClusterQueryData` / `BossRef`, `clusters`),
//! the optional first-class encounter entity (`EncounterDef` + `EncounterScript`),
//! and the adaptive music + cutscene + save-state systems. The registry is a
//! read-only `BossProfile` data catalog.
//!
//! `lib.rs` is intentionally a facade: type ownership, registration, update
//! systems, rewards, and event publication live in child modules so future boss
//! work doesn't pile into the entry point. Children:
//! `behavior`/`profile`/`specs`/`roster` (data schemas + App-local catalog views),
//! `clusters` (the authoritative boss components + borrow views),
//! `registry` (`BossEncounterRegistry` resource), `systems` (per-frame tick +
//! HP mirror), `encounter_entity`/`encounter_script` (the optional encounter
//! entity + its scripted beats), `events` (event publication), `rewards`
//! (reward chests), `ids` (id slugging), `attack_geometry` (hitbox math),
//! `sprites` (boss spritesheets).
//!
//! Each `BossSpawn` LDtk entity in the active room maps to one encounter id
//! (defaulting to the boss `name`). When the player enters the room the
//! encounter goes Dormant -> Intro and the cutscene queue is asked to play
//! `boss_intro_<id>`. From that point the phase machine drives transitions;
//! this crate mirrors them onto the boss cluster, the audio request, and the
//! save resource.
//!
//! # What made this a carve
//!
//! ⭐ **the two relocations came FIRST, and without them this was a facade
//! move.** The boss DATA MODEL used to live in the monolith's `features` hub
//! (`features::ecs::boss_clusters`, `BossOverrides` in the spawner,
//! `sync_boss_reward_chests_ecs` in the reward table); the earlier D33 slice
//! moved all ten symbols here, which dropped this domain's outward edges from
//! 49 sites to 21 — none of them boss vocabulary.
//!
//! ⭐ **and the honest instrument found exactly two real blockers left.**
//! Counting `crate::` on NON-COMMENT lines (a `use`-grep undercounts this
//! repository badly; a raw `crate::` grep measures its prose), every sibling
//! name this code reached was a re-export of a crate BELOW the monolith —
//! `BodyKinematics`, `CenteredAabb`, `FeatureId`, `FeatureSimEntity`,
//! `GameplayBanner`, `ChestFeature`, `Opened`, `FallingChest`,
//! `BossRewardChest` — except:
//!
//! * `CutsceneTriggerQueue`, which moved DOWN to `ambition_cutscene` beside the
//!   script format it triggers, and
//! * `MountDied`, which moved DOWN to
//!   `ambition_platformer2d_shared_tangle::body` because two domains share it:
//!   the monolith's mount coupling writes it and this crate reads it.
//!
//! ⚠ **the monolith still names this crate 200+ times and that is fine** — the
//! arrow points down. What would have blocked the carve is an edge pointing the
//! other way, and after the relocations there was none.
//!
//! # Ordering
//!
//! The `Progression` phase chain that drives the per-frame boss tick is
//! registered by `ambition_platformer2d_runtime`, not here; this crate owns the
//! CONTENT SLOTS in that chain ([`ContentEncounterScriptSet`],
//! [`ContentEncounterVictorySet`], [`ContentQuestRewardSet`]) so a named game
//! can interleave without the engine chain ever naming a content system.

pub mod attack_geometry;
pub mod behavior;
mod catalog;
mod clusters;
mod encounter_entity;
mod encounter_script;
mod events;
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
// The boss DATA MODEL — the authoritative components + the borrow views the
// per-tick systems mutate/read. Relocated from `features::ecs::boss_clusters`
// (D33): boss vocabulary belongs to the boss domain, and the hub it sat in was
// re-exporting it back to this module's own children.
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
