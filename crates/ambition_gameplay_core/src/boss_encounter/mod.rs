//! Sandbox-side coordinator for boss fights (distinct from the generic
//! `crate::encounter` enemy-wave system).
//!
//! Boss HP/phase state is ENTITY-LOCAL (`BossEncounter.health` +
//! `BossEncounter.encounter: BossPhaseState`); this module bridges it to the
//! in-arena boss ECS clusters (`features::BossClusterQueryData` / `BossRef`),
//! the optional first-class encounter entity (`EncounterDef` + `EncounterScript`),
//! and the adaptive music + cutscene + save-state systems. The registry is a
//! read-only `BossProfile` data catalog.
//!
//! This `mod.rs` is intentionally a facade: type ownership, registration,
//! update systems, rewards, and event publication live in child modules so
//! future boss work doesn't pile into the entry point. Children:
//! `behavior`/`profile`/`specs`/`roster` (data schemas + installed registries),
//! `registry` (`BossEncounterRegistry` resource), `systems` (per-frame tick +
//! HP mirror), `encounter_entity`/`encounter_script` (the optional encounter
//! entity + its scripted beats), `events` (event publication), `rewards`
//! (reward chests), `ids` (id slugging), `attack_geometry` (hitbox math),
//! `sprites` (boss spritesheets). (Player→boss damage routing lives in
//! `features::ecs::damage`.)
//!
//! Each `BossSpawn` LDtk entity in the active room maps to one encounter id
//! (defaulting to the boss `name`). When the player enters the room the
//! encounter goes Dormant -> Intro and the cutscene queue is asked to play
//! `boss_intro_<id>`. From that point the phase machine drives transitions;
//! this module mirrors them onto the boss cluster, the audio request, and
//! the save resource.

pub mod attack_geometry;
pub mod behavior;
mod encounter_entity;
mod encounter_script;
mod events;
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
    BossEncounterEvent, BossEncounterPhase, BossEncounterSpec, BossPhaseEvent, BossPhaseState,
    PhaseTrigger, PhaseTriggerCondition,
};
pub use behavior::{
    install_boss_profiles, install_boss_special_anim_keys, BossProfileRegistry, LimbMotion,
    LimbRoute,
};
pub use encounter_entity::{
    release_payloads_on_death, sync_boss_encounter_entities, update_encounter_progress,
    EncounterDef, EncounterProgress, EncounterWin, MemberProgress, PayloadReleased, ReleaseOnDeath,
};
pub use encounter_script::{
    tick_commanded_moves, tick_encounter_scripts, tick_falling_hazards, CommandedMove,
    EncounterBeat, EncounterEffect, EncounterGate, EncounterScript, EncounterTrigger,
    FallingHazard,
};
pub use profile::{default_boss_profiles, BossProfile, BossRewardProfile};
pub use registry::BossEncounterRegistry;
pub use roster::BossSpecRoster;
pub use specs::{boss_content_installed, default_boss_specs, install_boss_encounter_specs};
pub use systems::{
    boss_phase_transition_feedback, notify_bosses_on_mount_death, populate_boss_encounter_registry,
    update_boss_encounters,
};

// ── Progression-phase content slots (E-track de-weave) ──────────────────────
//
// The `SandboxSet::Progression` chain is ENGINE-generic (boss-encounter tick,
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
