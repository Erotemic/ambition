//! Sandbox-side coordinator for boss fights (distinct from the generic
//! `crate::encounter` enemy-wave system).
//!
//! Boss HP/phase state is ENTITY-LOCAL (`BossStatus.health` +
//! `BossStatus.encounter: BossPhaseState`); this module bridges it to the
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
// `MOCKINGBIRD_ENCOUNTER_ID` is not re-exported — it's the one hard-coded
// archetype id, referenced only by internal tests via
// `super::ids::MOCKINGBIRD_ENCOUNTER_ID`.
pub use ambition_characters::boss_encounter::{
    BossEncounterEvent, BossEncounterPhase, BossEncounterSpec, BossPhaseEvent, BossPhaseState,
    PhaseTrigger, PhaseTriggerCondition,
};
pub use behavior::{install_boss_profiles, install_boss_special_anim_keys, BossProfileRegistry};
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
pub use specs::{default_boss_specs, install_boss_encounter_specs};
pub use systems::{
    boss_phase_transition_feedback, populate_boss_encounter_registry, update_boss_encounters,
};
