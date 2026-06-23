//! Sandbox-side coordinator for ONE scripted boss fight (distinct from the
//! generic `crate::encounter` enemy-wave system).
//!
//! Bridges `BossEncounterState` (the actor-crate phase machine, re-exported
//! here) with the in-arena boss ECS clusters (`features::BossClusterQueryData`
//! / `BossRef`) and the adaptive music + cutscene + save-state systems.
//!
//! This `mod.rs` is intentionally a facade: type ownership, registration,
//! update systems, rewards, event publication, and damage routing live in
//! child modules so future boss work doesn't pile into the entry point.
//! Children: `behavior`/`profile`/`specs`/`roster` (data schemas + installed
//! registries), `registry` (`BossEncounterRegistry` resource), `systems`
//! (per-frame tick + HP mirror), `damage`/`events` (damage routing +
//! publication), `ids` (id slugging), `attack_geometry` (hitbox math),
//! `sprites` (boss spritesheets).
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
// `MOCKINGBIRD_ENCOUNTER_ID` is no longer re-exported — the dialog
// redirect that read it moved to the data-driven
// `assets/data/dialogue/registry.ron` `BossCleared("mockingbird")`
// rule. Internal tests reference it via `super::ids::MOCKINGBIRD_ENCOUNTER_ID`.
pub use ambition_characters::boss_encounter::{
    BossEncounterEvent, BossEncounterPhase, BossEncounterSpec, BossEncounterState, BossPhaseEvent,
    BossPhaseState, PhaseTrigger, PhaseTriggerCondition,
};
pub use behavior::{install_boss_profiles, install_boss_special_anim_keys, BossProfileRegistry};
pub use encounter_entity::{
    sync_boss_encounter_entities, update_encounter_progress, EncounterDef, EncounterProgress,
    EncounterWin, MemberProgress,
};
pub use profile::{default_boss_profiles, BossProfile, BossRewardProfile};
pub use registry::BossEncounterRegistry;
pub use roster::BossSpecRoster;
pub use specs::{default_boss_specs, install_boss_encounter_specs};
pub use systems::{
    boss_phase_transition_feedback, populate_boss_encounter_registry, update_boss_encounters,
};
